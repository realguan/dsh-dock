# ADR-0024：桌面任务快跑器（headless 消费）的范围裁定与技术形态

- **日期**：2026-09-15
- **状态**：草案 —— **暂缓实施**（2026-09-15 维护者裁定「**暂时不做**」）。本 ADR **未被否决**，
  但**当前批次不实施**（方案 A 与方案 B 均不启动）；`docs/roadmap.md:331` 的边界问题
  **留白未裁定**（维护者选择不在此刻裁决，故**不改动** §5 不做清单）。§1–§3 的证据与 §5 的
  口径**全部保留**，供重开时直接复用。
- **提出人**：guan（AI 协作起草）
- **相关方**：`src-tauri/src/shell.rs`（dsh 子进程）、`src-tauri/src/lifecycle.rs`（spawn seam）、
  `src-tauri/src/settings.rs`（既有 `switcherShortcut` / `showFloatingSwitcher`）、
  `src-tauri/src/ui.rs`（`switcher.js` 注入）、`src-tauri/tauri.conf.json`、
  `src-tauri/capabilities/default.json`、`src-tauri/Cargo.toml`、
  新增 `src-tauri/src/commands/runner.rs`（拟）、新增前端快跑器窗口、
  `frontend/src/lib/events.ts`（跨窗广播）、`frontend/src/components/layout/QuickDshSwitcher.tsx`
- **关联**：**ADR-0006**（网络面；本功能不新增网络面）、
  **ADR-0014**（重启/切换交接带与启动代际闸门）、**ADR-0015**（子进程生命周期归属，spawn seam）；
  AGENTS §1（三平台）/ §4.4 三红线 / §6（持久化例外册）/ §7（IPC 与事件）/ §8.1 / §8.3 / §8.5；
  `docs/roadmap.md:323-331`（§5 不做清单，**范围冲突所在**）；
  `docs/plans/official-plugins-and-capabilities-plan-review.md` §5（P2 精度）

---

## 1. 背景与问题

### 1.1 需求

提供一个可从任意应用呼出的轻量输入窗，把临时任务交给 dsh 并就地呈现结果，以避开打开完整 Web
工作台的重载开销。典型场景：临时分析日志、快速问答、审查单文件 diff。

### 1.2 技术路径确实存在（本 ADR 不需要论证可行性）

- `headless` 是**真实的出厂 profile 模板**（`packages/boot/app-boot/src/profile.ts:148-151`：
  `bundles: ['@deepseek-ai/dsh-base', '@deepseek-ai/dsh-headless']`），`apps/cli/README.md:20`
  列明它与 `web` / `sdk` / `sdk-minimal` / `acp` 一样**首次使用时自动初始化**。
- 调用式经实机 `--help` 与源码双重确认（`packages/bundle/headless/src/startup.ts:38-43`）：
  `dsh --profile headless [--json] [--session-id <id>] [task... | -]`，`-` 读 stdin
  （`startup.ts:97-99`：`` `-` `` 必须是唯一的任务参数）。

**因此本 ADR 的性质不是"能不能做"，而是"该不该由壳做、做到哪一步"。**

### 1.3 范围冲突（必须先裁定，不得静默假设）

`docs/roadmap.md:331`（§5 不做清单，节标题在 `:323`）原文：

> | dsh 自身功能扩展（会话管理、插件市场、模型配置等） | 是 dsh 本体的事，壳只负责呈现和文件层面的管理 |

一个**执行任务并渲染思维链**的快跑器属于**任务执行前端**，落在"呈现 + 文件层面的管理"之外。
这不意味着它必然被否——但意味着它**必须由维护者显式破例**，而不能由本 ADR 自行放行。
AGENTS §8.6（不确定就问，不猜）与 §8.7（驳回不合理的规则须举证并提出修订建议）正为此而设。

### 1.4 既有先例（不得把本功能描述为从零开始）

"快速呼出"在壳内**已有实现**，只是形态不同：持久化偏好 `showFloatingSwitcher` /
`switcherShortcut`（`src-tauri/src/settings.rs:52,54`，已登记进 AGENTS §6，2026-09-01）；
注入脚本经 `src-tauri/src/ui.rs:56`（`include_str!`）与 `:136`（`initialization_script`）
把 `frontend/src/injected/switcher.js` 注入 **WebView**；React 侧
`frontend/src/components/layout/QuickDshSwitcher.tsx` 是 **WebView 内的 keydown 监听**，
动作是 `api.focusMainWindow()`。

关键结论：既有快捷键是**页面级 keydown**，**只在壳的窗口拥有焦点时有效**，不是 OS 级全局热键，
因此无法满足"在任意应用里呼出"。这是新增 OS 热键的**唯一正当理由**，本 ADR 必须把它讲清楚，
而不是把新窗口当作无根之木。

### 1.5 headless `--json` 的真实契约（与实施计划的描述有实质差异）

实施计划 §3.5.2 声称"流式打字机输出最终结果"。实测与源码不支持这一描述：

- **事件共 8 种，不是 5 种**（`packages/bundle/headless/src/json-stream.ts:261-325`；
  `startup.ts:84`；契约文本在 `packages/bundle/headless/README.md:58`）：
  `session`（开篇，携带 `sessionId` 与 `cwd`）、`status`（`phase` ∈ `turn_start` /
  `step_start` / `step_end` / `turn_end`，`step_end` 可能带 `usage`）、`thinking`、`text`、
  `tool_call`（`callId` ＋ `tool`/`input`）、`tool_result`（`callId` ＋ `status`/`result`）、
  `final`（终止，携带无损答案，**不截断**）、`error`（轮次之外抛出的失败；出现即**没有** `final`）。

- **不存在 token 级流式**：`json-stream.ts:1-6` 明写每个投影事件都是一个 **commit point**——
  `text` / `thinking` 只来自**已提交**的 `assistant/message` 内容，"never from a live attempt
  that may still be retried or discarded"；`README.md:58` 进一步明确它们"arrive when the step
  commits, **not per token**"。故"流式打字机"从 `--json` 拿不到，只能做**逐块追加**。
- **截断与丢弃语义**（`json-stream.ts:18-19`、`:300-303`）：除 `final` 外每个字符串/键上限
  **8 KiB** 并置 `truncated` 标志，单行（含换行）上限 **32 KiB**；`tool_result` 在
  `event.surfaceOp !== 'append'`（被压缩替换）时**直接丢弃**，故 UI 必须容忍"有 `tool_call`
  而无对应 `tool_result`"。**失败信号** = 退出码 1 **＋** `turn_end` 的 `reason`。
- **`--session-id` 有明确拒绝集**（`README.md:54`、`:145`）：未知 id 是错误；此外**拒绝**
  subagent / forked 会话、未记录工作目录的会话、本 profile 未组合的 agent preset、
  preset 记录畸形者，以及**已在本进程内活跃**的 id（"another owner may still drive it"）。

### 1.6 新增平台面

- 需要 **OS 全局热键**：`src-tauri/Cargo.toml` 现有插件仅 `tauri-plugin-single-instance`（`:33`）
  与 `tauri-plugin-updater`（`:34`），**没有** `tauri-plugin-global-shortcut`——这是**新增 Rust
  依赖**，按 AGENTS §4.2/§9 须先立 ADR。
- 需要**第四个窗口**：`tauri.conf.json` 的 `app` 段只有 `security` 与 `withGlobalTauri` 两个键，
  **`app.windows` 为 `undefined`**（该文件另有两处 `windows` 键，分别是 `plugins.updater.windows`
  与 `bundle.windows`，与窗口声明无关，排障时勿误改）；且 `capabilities/default.json:5-9` 把窗口
  白名单硬编码为 `["main", "about", "profiles"]`，新增窗口须扩该数组并单独授权事件（AGENTS §7:168-170）。
- **跨窗口真相源**（AGENTS §4.4 红线 3、§4.3）：各窗口是独立 JS runtime，Zustand **不跨窗**；
  结果与状态必须经 `lib/events.ts` 的**模块加载期装配**广播传递——晚挂监听会吞掉首发遥测。

---

## 2. 约束与硬指标

1. **范围须有维护者裁定**：落在 `roadmap.md:331` 不做清单边界之外的功能范围必须取得显式
   裁定，本 ADR 不得自行放行。
2. **增量生成**（AGENTS §8.1/§8.2）：只做一个明确意图，不得与其它特性同批交付；超范围想法
   记入计划另行开工。
3. **前端三红线**（AGENTS §4.4）：依赖白名单内（既有栈 ＋ 既有 12 个 `components/ui` 原语，
   折叠/标签循 `PluginHub.tsx:24-58` 的手写 `role="tablist"` 惯例，不引组件库）；`--json` 流
   由 Rust 捕获后经 IPC / 事件推送，前端不发网络请求；跨窗只经广播，不得假设 Zustand 跨窗可见。
4. **spawn 一律经 `lifecycle` seam**（AGENTS §6）：`lifecycle.rs:417` / `:465`，闸门
   `lifecycle.rs:1876`；Windows 侧经 `crate::child_cmd`（`lib.rs:70`）。
5. **生命周期 1:1**：快跑器起的 dsh 进程必须随壳退出被收干净，不得产生孤儿
   （ADR-0015 硬杀口径同样适用）。
6. **事件解析必须完整**：8 种事件全部处理（§1.5），正确处理 8 KiB / 32 KiB 截断、
   `tool_result` 丢弃、`error` 无 `final`；**不得宣称 token 级流式**。
7. **会话复用语义必须定义**：`--session-id` 拒绝集（§1.5）意味着"复用一个持久快跑会话"必须
   写明冲突检测与降级路径（如该 id 已被工作台占用时如何提示与处置）。
8. **新依赖与新窗口须显式批准**：`tauri-plugin-global-shortcut` 与第四窗口的白名单扩展均须
   在本 ADR 记录后方可引入。
9. **可测且须落档**（AGENTS §5/§8.3/§8.5）：NDJSON 解析器须为**纯函数**并带 fixture 单测
   （覆盖 8 种事件、截断、丢弃的 `tool_result`、拒绝集分支），不引测试依赖；完成后落
   `docs/broadcasts.md`，并同步 AGENTS §6（若新增持久化字段）与 §7（新命令与事件）。

---

## 3. 备选方案及评估

### 方案 A：OS 全局热键 + 独立轻量窗口，但**只做"任务转交"**（不执行、不渲染运行）—— ✅ 采纳（可立即实施部分）

- 思路：注册全局热键呼出一个轻量窗口；窗口内只做三件事——输入任务、选择目标
  （转交 Web 工作台 / 交给 `dsh --profile headless` 以**分离终端**方式执行）、复制结果。
  **壳本身不解析 `--json`、不渲染思维链**，因此不构成"任务执行前端"，留在
  `roadmap.md:331` 的"呈现 + 文件层面管理"之内。
- 优点：不越不做清单，**无需维护者破例即可实施**；复用既有 `switcherShortcut` 的偏好模型与
  注入脚本经验；把"呼出"这一真正的用户痛点（§1.4 的页面级 keydown 限制）解决掉；
  技术上不引入 NDJSON 解析与流式渲染的复杂度。
- 代价/风险：体验弱于完整版（用户要切到工作台或终端看结果）；仍**新增 Cargo 依赖与第四窗口**，
  故 §2.8 仍需本 ADR 背书。
- 对照约束：§2.1 不越界 ✅；§2.2 单一意图 ✅；§2.3 前端不变 ✅；§2.4 转交 headless 则走 seam ✅；
  §2.5 1:1 生命周期 ✅；§2.6 不涉及解析（无此风险）✅；§2.7 会话复用可不引入 ✅；
  §2.8 已记录 ✅；§2.9 逻辑面小、易测且落档 ✅。

### 方案 B：完整版——执行并流式渲染思维链 —— ⏸ **条件挂起，待维护者裁定**

- 思路：壳起 `dsh --profile headless --json --session-id <id> -`，解析 8 种 NDJSON 事件，
  在快跑窗内渲染思维链折叠条、工具执行徽章与最终答案。
- 挂起理由：**它直接撞上 §2.1 的范围边界**（`roadmap.md:331`）。这不是技术缺陷——
  §1.2/§1.5 已证明技术与契约都清楚——而是**职责归属未经裁定**。
  本 ADR 的义务是把这个冲突摆到台面上，而不是替维护者做决定。
- 若裁定**允许**：本方案转为采纳，并须完整承担 §2.6/§2.7/§2.9 的全部义务（8 事件解析、截断、
  丢弃容忍、拒绝集与降级路径、纯函数 fixture 测试），且实施计划中"流式打字机"须改为"逐块追加"。
- 若裁定**不允许**：本方案作废，方案 A 成为最终形态，结论记入 `roadmap.md` §5 不做清单，
  避免同一提案再次排队。

### 方案 C：不做 —— ❌ 否决为唯一方案（但其约束被方案 A 内化）

- 思路：维持现状，用户需要跑临时任务就在工作台里做。
- 否决理由：**它是对范围冲突最保守的答案，但忽略了一个已存在的真实缺陷**——既有快捷键是
  WebView 内 keydown（§1.4），用户**必须先切到壳窗口**才能呼出，与"快速呼出"根本冲突。
  方案 A 正是为在**不越范围**的前提下修掉该缺陷；本方案的精神（不扩张壳的职责）被方案 A 完整
  保留。作为一等备选**显式留下**：若维护者连方案 A 的热键扩展也不接受，则回落到本方案。

### 方案 D：沿用既有 WebView 内快捷键，不新增 OS 热键 —— ❌ 否决

- 思路：扩展 `QuickDshSwitcher` / `switcher.js`，不引入 `tauri-plugin-global-shortcut`、不新增窗口。
- 否决理由：既有实现是 `window.addEventListener("keydown", …, true)`（`QuickDshSwitcher.tsx`）
  ＋ `api.focusMainWindow()`，只在壳窗口有焦点时触发；不引入 OS 级热键，"任意应用呼出"这一核心
  诉求**在机理上不可能满足**，方案会退化成"又一个工作台内快捷键"，与既有功能重复而无增量。
  被否决的是"不新增依赖"这一取巧路径，**不是**既有实现本身——既有实现应保留为工作台内入口
  （两者共存，快捷键不得冲突）。

### 方案 E：消费 live attempt 以实现 token 级流式渲染 —— ❌ 否决

- 思路：不走 `--json` 投影，改为消费进行中的 assistant attempt 以获得逐 token 输出。
- 否决理由：**违反 §2.6 与上游契约**。`json-stream.ts:1-6` 说明投影**刻意**只取已提交内容，
  因为 live attempt "may still be retried or discarded"——照此实现会把**最终不存在的文本**渲染给
  用户（重试后内容可能被替换或丢弃），UI 与事实不一致。`README.md:58` 更指明 default 模式的
  stderr 推理是"the only live text channel"，即**上游自己也没有把 live 文本作为契约提供**。

### 方案 F：壳自建解析缓存 / 会话镜像 —— ❌ 否决

- 思路：壳把 `--json` 事件流落盘成自己的会话记录，供快跑窗回看与检索。
- 否决理由：**与 dsh 的会话日志构成双源**（AGENTS §6：会话目录只读不删，壳只做元数据层面管理）。
  双源会在压缩、修复、归档等既有机制介入时静默分叉，并把"谁的记录是真的"引入壳层。事件流是
  **投影**而非日志（`README.md:145`：`--json` "is not a lossless copy of the Session log"），
  以投影为源必然失真。

---

## 4. 最终决策

**维护者裁定（2026-09-15）：「暂时不做」。当前批次不实施——方案 A 与方案 B 均不启动。**

具体含义，三条边界必须写清，否则后人会误读：

1. **本 ADR 未被否决**。裁定是**时序上的暂缓**（"暂时"），技术评估与形态设计仍然成立；
   维护者**没有**对 `docs/roadmap.md:331` 的边界作出实体裁决，也**没有**要求改动 §5 不做清单。
   故**不得**把本 ADR 记入不做清单，也**不得**以"已被否决"为由删除本文件。
2. **范围问题留白**。即"壳内渲染任务运行过程是否越出'呈现 + 文件层面的管理'"**仍未裁定**。
   重开时该问题需一并裁决；裁决结果决定是采纳方案 A（转交版，留在边界内）还是方案 B
   （完整版，须显式修订 `roadmap` §5 并落档，**不得**静默例外）。
3. **两项平台扩展未获批准**。方案 A 所需的 `tauri-plugin-global-shortcut` **新依赖**与
   **第四个窗口**（扩 `capabilities/default.json:5-9` 窗口白名单 + 单独事件授权）一律**未批准**，
   不得因本 ADR 存在而先行引入。

**重开时的强制口径（无论采纳 A 还是 B，均须遵守）**：事件 **8 种**必须全处理（`session` /
`status` / `thinking` / `text` / `tool_call` / `tool_result` / `final` / `error`）；
`text`/`thinking` 只能**逐块追加**（**不得**宣称 token 级流式——`--json` 只在 step 提交时产出）；
8 KiB 单串 / 32 KiB 单行截断与 `tool_result` 被压缩丢弃必须容忍；`--session-id` 拒绝集必须定义
降级路径；失败信号取退出码 1 ＋ `turn_end` 的 `reason`。**明确不做**：不沿用仅有页面级 keydown
的既有快捷键充当全局热键（方案 D）、不以 live attempt 伪造 token 级流式（方案 E）、
不落盘壳自建的会话镜像（方案 F）。

---

## 5. 后果与后续行动项

### 正面后果

- 修掉一个**已存在的真实缺陷**：既有"快速呼出"只是 WebView 内快捷键，用户必须先切窗口（§1.4）；
  全局热键使"任意应用呼出"第一次成立。
- 把一处**潜在的范围越界**从"实施计划里的一句话"提升为**显式的维护者裁定项**，避免壳在无人注意的
  情况下长出任务执行前端。
- 修正了实施计划对 headless 契约的三处失实描述（5 事件 → 8 事件；"流式打字机" → 逐块追加；
  未提截断与拒绝集），这些结论对后续任何 headless 消费方同样适用。

### 负面后果 / 新增债务

- **新增 Rust 依赖 `tauri-plugin-global-shortcut`**：与 AGENTS §1 的 Tauri 同代约束绑定，
  升级 Tauri 时须一并验证；全局热键的多平台注册失败模式不同（被其它应用占用、macOS 辅助功能
  权限），须有失败回退（回落到工作台内入口）。
- **新增第四个窗口**：`capabilities/default.json` 窗口白名单从 3 变 4，事件授权面扩大
  （AGENTS §7 须同步登记）；窗口生命周期与 ADR-0014 交接带、ADR-0015 的 1:1 收口需额外实机验证。
- **方案 A 的体验天花板是刻意设低的结果**：用户拿到任务转交而非就地结果，UI 必须诚实表达，
  不得暗示"任务会在这里跑完"。
- **方案 B 的裁定悬空本身是债务**：裁定作出前，实施计划中关于本功能的描述须标注"待裁定"，
  不得被当作已批准范围；若方案 A 落地而方案 B 被否，会产生"能力半成品"的观感，需靠文案与
  不做清单登记化解。

### 行动项

- **范围声明（非行动项）**：本 ADR **不需要**修改 `docs/contract.md` 或升 `MANIFEST_FORMAT`——快跑器是壳内窗口与 headless 消费，不触及「装配方 ↔ 产品壳」契约。
- [x] **已提请维护者裁定**（2026-09-15）→ 裁定为「**暂时不做**」，范围边界问题**留白未裁决**，
      当前批次不实施；本项**不再阻塞**，本 ADR 转入暂缓。
- [ ] **重开前置**（仅当维护者重新提出该需求时）：先就 `roadmap.md:331` 的边界作实体裁定，
      再据裁定选方案 A 或 B，并回写 §4；若走方案 B，须显式修订 `docs/roadmap.md` §5 并落档。
- [ ] **以下各项均为重开后**方可执行，暂缓期间不得启动：
- [ ] AGENTS §7 登记新 IPC 命令（如 `run_quick_task` / `open_runner_window`）与新窗口的事件授权；
      新命令走 4 处名字面同步（`ipc.rs::COMMANDS` → `lib.rs` handler →
      `capabilities/default.json` → `frontend/src/lib/tauri.ts`，闸门 `ipc.rs:292`）；AGENTS §6 登记
      新增持久化字段（如"上次快跑会话 id"/"快跑窗尺寸"，参照 `showFloatingSwitcher` /
      `switcherShortcut` 的 2026-09-01 条目）
- [ ] `src-tauri/Cargo.toml` 引入 `tauri-plugin-global-shortcut`（同代约束）+ 注册失败回退；
      新增第四个窗口（`tauri.conf.json` 的 `app.windows` ＋ `capabilities/default.json:5-9` 白名单
      扩展 ＋ 事件授权）
- [ ] 纯函数 NDJSON 解析器 + fixture 单测（**AGENTS §8.3 强制**）：覆盖全部 **8** 种事件、
      8 KiB 字符串截断与 `truncated` 标志、32 KiB 行截断、`surfaceOp !== 'append'` 导致的
      `tool_result` 丢失、`error` 无 `final`、退出码 1 ＋ `turn_end.reason` 的失败判定，以及
      `--session-id` 拒绝集分支（未知 id / subagent / 无 cwd / preset 不匹配 / 已活跃）
- [ ] `--session-id` 复用策略与冲突降级路径的设计与文档化（§2.7）；spawn 经 `lifecycle` seam
      （`lifecycle.rs:417/465`），Windows 经 `crate::child_cmd`，实机验证硬杀无孤儿（ADR-0015 口径）；
      跨窗通信经 `lib/events.ts` 模块加载期装配（AGENTS §4.3 的坑），不得依赖 Zustand 跨窗
- [ ] `docs/executor.md` 实机验证清单：热键在 macOS / Windows 注册成功、被占用时的回退、快跑窗与
      主窗的交接、`-` 作为唯一任务参数的调用形态
- [ ] `docs/broadcasts.md` 落档（**AGENTS §8.5**）；`docs/roadmap.md` §5 与 §4 相关条目同步；
      AGENTS §9 索引行

---

## 6. 复审条件

- **维护者重新提出桌面快跑器需求** → 本 ADR 重开（2026-09-15 裁定为「暂时不做」，故这是唯一的
  解冻触发）。重开时**第一件事**是就 `roadmap.md:331` 的边界作实体裁定——该问题本次留白未决。
- **维护者就 `roadmap.md:331` 边界作出裁定** → 依裁定结果落定方案 A 或方案 B，并回写 §4 与
  （若走方案 B）不做清单；**不得**在该裁定缺席时以 A 或 B 的名义动工。
- **上游为插件/会话提供官方任务执行 RPC 或就地渲染契约** → 方案 B 的"壳自己解析 NDJSON"前提消失，
  应改为薄转发，§3 与 §4 重开。
- **headless 事件集或截断语义变化**（新增事件类型、调整 8 KiB / 32 KiB、改变 `tool_result` 丢弃
  规则、`--session-id` 拒绝集调整）→ 复评 §2.6/§2.7 与 §4 的强制口径。
- **`tauri-plugin-global-shortcut` 与 crate 代际失配或该插件停止维护** → 复评热键实现路径
  （可能要回到方案 D 并接受"仅工作台内呼出"）。
- **第四个窗口在真实用户环境出现生命周期问题**（孤儿、交接带冲突、事件授权失效）→ 复评窗口形态，
  考虑改为在主窗内浮层实现。
- **方案 A 上线后"转交"路径使用率极低而"就地结果"呼声高** → 以数据重提方案 B 的裁定。
