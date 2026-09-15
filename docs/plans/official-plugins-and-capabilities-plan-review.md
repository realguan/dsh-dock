# 审核报告：`official-plugins-and-capabilities-plan.md`

> **被审文档**：`docs/plans/official-plugins-and-capabilities-plan.md`（2026-09-15，271 行）
> **审核对象版本**：文件未入库（`git status` → `?? docs/plans/`，HEAD `d89780a`）
> **审核日期**：2026-09-15
> **审核方式**：DSH 上游源码逐点取证（`/Users/guan/git/deepseek-harness`，`0.1.6-alpha.1`）
> ＋ 本机实机复现实验（抛离式 `DSH_HOME`，不触碰真实 `~/.dsh`）
> ＋ npm registry 实查 ＋ 本仓库源码与台账逐点比对
> **结论**：**修订后重审（不通过）**。方向正确，但含 7 项阻断级缺陷，其中 3 项会导致**静默失效**。

---

## 0. 审核结论摘要

| 维度 | 判定 |
|:---|:---|
| 方向与定位（壳做管理面、守两条红线） | ✅ 正确，与 ADR 体系一致 |
| 功能选型（官方实验包 / 归档 / MCP / SSH / headless） | ✅ 均为真实能力，包名与命令旗标基本属实 |
| **配置写入方案** | ❌ **阻断**：`cordis.patch.yml` 写法错误，**静默失效**（B1） |
| **激活语义** | ❌ **阻断**：bundle 类包被重复挂载、plain 类包不会挂载（B2） |
| **版本策略** | ❌ **阻断**：裸包名解析到 `0.1.5-alpha.2`，与 `0.1.6-alpha.1` 运行时错配（B3） |
| **需求新鲜度** | ❌ **阻断**：P0-2 的归档筛选**在壳内已实现**，且**上游已自带取消归档 UI**，方案在描述已完成的工作（B4） |
| **IPC 契约** | ❌ **阻断**：签名缺 `AppHandle`；同步面数与方案不符；未择定实现路线（B5） |
| **MCP 探测合规路径** | ❌ **阻断**：`updates.rs` 定性错误，漏机器闸门登记；且无 MCP RPC 可枚举，价值主张待论证（B6） |
| **SSH 配置与前置条件** | ❌ **阻断**：缺必填字段，前置条件严重低估，且"远程开发模式"承诺与上游范围冲突（B7） |
| 目录设计 | ⚠️ 需新增 provider 互斥与配套关系建模（B2c/B2d） |
| 文档准确性 | ⚠️ 6 处路径/符号不存在或定性错误 |
| 治理登记面 | ⚠️ 5 个 ADR、台账、闸门豁免、roadmap 收口全缺 |
| 流程合规（§8.1/§8.2） | ⚠️ 5 特性一锅端，违反"增量生成 / 禁跨模块批量改动" |

---

## 1. 证据基线（全部可复现）

本次审核不依赖"我记得"，全部结论锚定源码行号或实机实验。复现命令附于附录 A。

**上游源码锚点**（`/Users/guan/git/deepseek-harness`）：

| 事实 | 锚点 |
|:---|:---|
| `dsh plugin` 是 pnpm 转发器 **＋ bundles 协调器** | `apps/cli/src/plugin.ts:1-11`、`:59-91`、`:115-163` |
| profile 层栈 = `package.json` 的 `dsh.profile.bundles` | `packages/boot/app-boot/src/profile.ts:53-58`、`:141-157` |
| patch 挂载须 `insert:` 形式 | `packages/bundle/base/README.md:55-61`、`docs/user/develop/basic/config.md:37-45` |
| patch 覆盖语义 = 整体替换不深合并 | `packages/bundle/base/README.md`（"Each patch entry replaces the target's whole configuration"） |
| `unarchiveSession` 真实存在 | `packages/workspace/workspace/src/index.ts:266-275` |
| 归档集合字段 `archivedSessionIds` | `packages/workspace/workspace/src/index.ts:232`、`spec.ts:46-55` |
| 存储落盘格式 = `{unit,global,tables}` 版本戳文档 | `packages/storage/storage-json/src/format.ts:20-45`、`:47-80` |
| headless 旗标 `--json` / `--session-id` / `-` | `packages/bundle/headless/src/startup.ts:43`；实机 `dsh --profile headless --help` |
| NDJSON 事件名与截断上限 | `packages/bundle/headless/src/json-stream.ts:18-19`、`:257-332` |
| `dsh-ssh` 必填配置字段 | `packages/ssh/ssh/README.md`（Use this package 字段表） |
| ssh 家族须四包组合 | `packages/ssh/README.md:23-28`、`packages/ssh/ssh/README.md` |
| MCP transport 仅 `stdio` \| `streamable-http` | `packages/mcp/mcp-client/src/index.ts:49-95` |
| MCP 方法名 `resources/list`、`resources/templates/list` | `packages/mcp/mcp-client/src/connection.ts:372,376`；`packages/mcp/mcp-resources/src/index.ts:22` |
| 实验包 `dsh.bundle` 分类 | `packages/experimental/*/package.json`（实查） |
| 实验包"无支持承诺" | `packages/experimental/README.md`（"contracts can change and carry no support promise"；"Released products outside this group must not depend on experimental packages"） |
| 上游**已自带**取消归档设置页 | `packages/bundle/web-app/cordis.patch.yml:251-252` 挂载 `@deepseek-ai/dsh-client-ui-settings-unarchive-sessions`；`packages/client/ui-settings-unarchive-sessions/src/client/ArchivedSessionsSection.tsx`（逐行 Unarchive）；`locales.ts:11,33`（`取消归档` / `Unarchive`） |
| `unarchiveSession` 另有 **Remote** 出口 | `packages/api/workspace-controller/src/index.ts:35,43,118-121`（`@Remote('unarchiveSession')`，namespace `workspace`） |
| patch 行真实 schema（无 `before`/`after`） | `vendor/include/src/index.ts:129-141`（`PatchOptions`）、`vendor/loader/src/config/entry.ts:9-22`（`EntryOptions`）；排序 = 数组位置 |
| 挂载行的 `id` 是身份键（非 `name`） | `vendor/loader/src/config/tree.ts:51-59`（按 `id` 建 store；缺 `id` 自动生成则**再也无法被 patch**） |
| 存储格式与写者唯一性 | `packages/storage/storage-json/src/format.ts:5,44-58`；`single-unit.ts:1-5`；`atomic.ts:1-10`（"exactly one writer per process and **last-write-wins is correct**"） |
| 同族 provider 互斥 | `computer-use-cua-driver-mcp/README.md:53`；`docs/subsystems/browser-use.md:17` |
| SSH 适用范围限定 | `docs/subsystems/ssh.md`（Composition scope）；`packages/ssh/ssh/README.md:93`（"The initial composition scope is **POSIX headless and custom profiles**"） |
| headless 完整事件集（含 `error`） | `packages/bundle/headless/README.md:58`；`startup.ts:84` |
| `--session-id` 拒绝集 | `packages/bundle/headless/README.md:54,145` |

---

## 2. 阻断级缺陷（必须修订，否则不得进入实施）

### B1 —— `cordis.patch.yml` 的插件挂载写法错误，且**静默失效** ⛔

**方案原文**：§3.4.2（`:180-188`）给出"自动化 Profile 生成"YAML：

```yaml
- name: '@deepseek-ai/dsh-ssh'
  config:
    host: my-remote-server
    node: /usr/bin/node
    workspace: /home/developer/workspace
```

§3.1.1 时序图（`:106`）同样写"根据插件模板写入 `cordis.patch.yml`"。

**实机复现结论**（抛离式 `DSH_HOME`，详见附录 A 的 TEST A/B）：

| 写法 | `--dump-config` 退出码 | stderr | 插件是否挂载 |
|:---|:---:|:---|:---:|
| `- name: '@deepseek-ai/dsh-browser-use'` | **0** | ⚠️ `patch: id is required for non-insert patches` | ❌ **否**（0 命中） |
| `- insert:` + `id` + `name` | 0 | 无告警 | ✅ **是**（命中于第 574 行） |

**性质判定**：这不是"写法瑕疵"，而是**静默失败**。`dsh` 以退出码 0 结束、只在 stderr 打一行告警、patch 条目被丢弃。一个"安装成功 / 配置写入成功"的 GUI 会向用户报告成功，而插件永远不会加载。方案 §3.4.2 的 SSH 向导与 §3.1.1 的安装链**全部依赖这条写入路径**，因此缺陷沿 P0 → P1 传播。

**正确写法**（上游两个权威出处一致：`packages/bundle/base/README.md:55-61` 与 `docs/user/develop/basic/config.md:37-45`）：

```yaml
- insert:
    - id: remote-dev-box-ssh
      name: '@deepseek-ai/dsh-ssh'
      config:
        # …
```

**语义边界（必须先分清，再谈实现）**：

- `- insert:` 行 = **挂载一个原本不在树里的插件**；
- `- name:` / `- id:` 行 = **覆盖/禁用树里已存在的行**（仓库既有 `set_plugin_disabled` 用的就是这一形态，见台账 #13）。

方案把两者混为一谈，既没写对挂载，也没说清覆盖。

**附带必改项**：`config` 键是**整体替换而非深合并**（`packages/bundle/base/README.md`；本仓库 `docs/roadmap.md:31` 早已登记同一陷阱："一旦写 `config` 键，该行 config **整体替换不深合并**，多层按序应用后者胜"）。因此任何写 `config` 的实现**必须先把该行现有完整 config 读出来再回写**，方案 §3.1.2 只提到"保留原有注释与结构"，漏了这条更危险的合并语义。

**另一处必须显式建模的细节**：行的身份键是 **`id`**，不是 `name`——`vendor/loader/src/config/tree.ts:51-59` 按 `id` 建立 store；缺 `id` 时**自动生成**，该行此后**再也无法被 patch 命中**（后续 `- id:` 无从上手）。故壳生成的每一个 `insert` 行**必须自带稳定、可复算的 `id`**（例如由包名规范化派生），否则将产生"装了却永远改不了配置"的不可维护状态。方案全程未提 `id`。真实 schema 为 `PatchOptions`（`id`/`insert`/`name`/`config`/`group`/`disabled`/`inject`/`intercept`/`isolate`，`vendor/include/src/index.ts:129-141`）＋ `EntryOptions`（`id`/`name`/`config`/`group`/`disabled`/`inject`，`vendor/loader/src/config/entry.ts:9-22`）；**不存在 `before`/`after` 排序键**，顺序即数组位置。

---

### B2 —— bundle 类包由 `dsh plugin add` **自动激活**；统一写 patch 会造成重复挂载或全量漏挂 ⛔

**方案原文**：§3.1.1（`:104-107`）时序图为"先 `dsh plugin add`，**再**由壳写 patch 激活"，且对 §3.1.1 收录的 7 个包采用同一流程。

**实机复现 + 源码结论**：`dsh plugin add <pkg>` **不是**单纯 pnpm 转发。`apps/cli/src/plugin.ts:1-11` 定义其为"initialize the profile on first use, run `pnpm <args...>`, **then reconcile the `dsh.profile.bundles` layer list against the installed state**"；`:65-76` 把"解析到声明 `dsh.bundle` 的包"**追加进 `bundles`**。

实测（附录 A TEST C）：`dsh plugin --profile probe add @deepseek-ai/dsh-experimental-agent-team-profile` 后，profile 清单由

```json
"bundles": ["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-web-app"]
```

变为

```json
"bundles": ["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-web-app", "@deepseek-ai/dsh-experimental-agent-team-profile"]
```

**本仓库台账 #7 早已登记该机制**（2026-08-29 复核，原文）："**reconcile 会把声明 `dsh.bundle` 的新装依赖追加进 bundles**……同一包名 bundles/dependencies 双现属 dsh 数据模型本然"。方案未与之对齐。

**实查分类表**（`packages/experimental/*/package.json`）：

| 包 | `dsh.bundle` | `add` 后是否自动激活 | 壳应做的事 |
|:---|:---:|:---:|:---|
| `dsh-experimental-auto-review` | ✅ `./cordis.patch.yml` | ✅ 是 | **只覆盖配置**（写 `- id:`），**禁止** `insert:` |
| `dsh-experimental-agent-team-profile` | ✅ | ✅ 是 | 同上（且**方案完全未收录此包**） |
| `dsh-experimental-agent-team-web-profile` | ✅ | ✅ 是 | 同上（未收录） |
| `dsh-experimental-browser-use-playwright-mcp` | ❌ 无 | ❌ 否（仅告警"declares no dsh.bundle"） | **须 `insert:` 挂载** |
| `dsh-experimental-browser-use-chrome-devtools-mcp` | ❌ | ❌ 否 | 须 `insert:` |
| `dsh-experimental-browser-use-stagehand-native` | ❌ | ❌ 否 | 须 `insert:` |
| `dsh-experimental-computer-use-cua-driver-mcp` | ❌ | ❌ 否 | 须 `insert:` |
| `dsh-experimental-computer-use-cua-driver-native` | ❌ | ❌ 否 | 须 `insert:` |
| `dsh-experimental-agent-team` | ❌ | ❌ 否 | 须 `insert:` |
| `dsh-experimental-tool-agent-team` | ❌ | ❌ 否 | 须 `insert:`（未收录） |

**由此得出两条相反方向的错误，方案两者都会踩**：

1. 若统一用 `- insert:`（即采纳 B1 的修正而不分类）→ `auto-review` 被挂载**两次**（bundle 层一次 + 用户 patch 层一次）；而用户 patch 层在 bundle 层**之后**应用，重复实例的真实后果需实测确认（可能双份工具注册或加载报错）。
2. 若统一用 `- name:`（方案现状）→ 6 个 plain 包**全部静默漏挂**（B1 已证）。

**结论**：实现**必须按目标包是否声明 `dsh.bundle` 分支**——"挂载"与"配置"是两种操作，方案缺这一层抽象。

**另需修正的目录完整性**：方案 §3.1.1 声称收录"首批官方插件清单"7 个，但真实启用 Agent Teams 所需的组合是 `agent-team` ＋ `tool-agent-team` ＋ `agent-team-profile`（＋ Web 侧 `agent-team-web-profile` / `client-ui-agent-team`）。`tool-agent-team` 的职责就是"Nine tools that let the model create, message, and coordinate teammates"（`packages/experimental/README.md`），不可省——`agent-team` 自述"It provides **no tools of its own** — mount the sibling `dsh-experimental-tool-agent-team`"（`agent-team/README.md:12`），且需 `@deepseek-ai/dsh-session-persistence-jsonl` 提供持久化（`agent-team/src/index.ts:60` 的 inject 含 `sessionPersistence`/`sessionProjections`）。方案 §3.1.2 只对 Browser Use 提了配套，且把配套猜成 `@deepseek-ai/dsh-browser-use`——该判断在 browser-use 上**正确**（`packages/experimental/browser-use-playwright-mcp/README.md` 的官方示例正是先挂 `@deepseek-ai/dsh-browser-use` 再挂 provider），但漏了 `browser-use-runtime`，且对 Agent Teams 完全没有配套分析。

**配套关系的三处关键更正（直接决定 P0-1 的目录设计）**：

| 事实 | 含义 |
|:---|:---|
| `browser-use-runtime` 是**库、无 `dsh.bundle`、不可挂载**（`browser-use-runtime/README.md:28`："This public experimental library is a dependency of the browser providers. **It has no plugin entry or mount configuration.**"） | 它**不能**作为目录条目出现，只能作为依赖被带上；可挂载的配套是 release 服务 `@deepseek-ai/dsh-browser-use` ＋**恰好一个** provider |
| **不存在 `computer-use-runtime`**（`packages/experimental/*runtime*` 只有 browser-use / code-runtime-python / ptc-runtime-python / webworker-runtime） | computer-use 两个 provider 的配套是 release 服务 `@deepseek-ai/dsh-computer-use`（peer）＋ `@deepseek-ai/dsh-mcp-client` |
| **同族 provider 互斥**：浏览器三选一、桌面控制二选一。`computer-use-cua-driver-mcp/README.md:53`："A second computer-use provider **fails activation**, including another instance of this package."；浏览器同口径见 `docs/subsystems/browser-use.md:17` | ⛔ **目录 UI 必须强制互斥**（同族只能装一个，并给出"替换"语义而非"叠加"），否则用户装第二个即**激活失败**。方案把 3 个 browser-use ＋ 2 个 computer-use 平铺为独立条目，未做任何互斥建模 |

**"首批"这一表述本身需要修正**：上游无"首批"概念，政策是**全量发布**——`packages/experimental/README.md:12`（"**All current packages** publish under their `@deepseek-ai/dsh-experimental-*` names…"）与 `scripts/experimental-package-policy.ts:2`（`PRIVATE_EXPERIMENTAL_PACKAGE_DIRECTORIES = []`），实际共 **16** 个包；方案未收录 9 个（`agent-team-profile`、`agent-team-web-profile`、`browser-use-runtime`、`client-ui-agent-team`、`inspector`、`ptc-runtime-python`、`tool-agent-team`、`webworker-packer`、`webworker-runtime`）。dsh-dock 完全有权**自行策展**一个精选集，但必须写明"这是 dsh-dock 的策展范围"，而非"官方首批"，否则是归属失实。

**另需注意的上游事实**：7 个包中唯一 README 给出 `dsh plugin` 命令的 `auto-review`，其命令是**源码仓相对路径**（`auto-review/README.md:33`：`dsh plugin --profile web add ./packages/experimental/auto-review`），**在 npm 安装场景下不适用**。5 个 browser/computer provider 的 README **完全没有 `dsh plugin` 命令**，只给手工 YAML 行——即方案要为它们首次定义安装流程，无上游范本。

---

### B3 —— 版本必须钉死：裸包名解析到 `0.1.5-alpha.2`，与 `0.1.6-alpha.1` 运行时错配 ⛔

**方案原文**：§3.1.1（`:103-105`）为 `install_plugin(profile, official_pkg, version)`，时序图为 `dsh plugin --profile <name> add <pkg>`（裸包名）。

**实查 npm dist-tags**（2026-09-15）：

| 包 | `latest` | `alpha` |
|:---|:---|:---|
| `dsh-experimental-agent-team-profile` | **0.1.5-alpha.2** ⚠️ | 0.1.6-alpha.1 |
| `dsh-experimental-agent-team` | **0.1.5-alpha.2** ⚠️ | 0.1.6-alpha.1 |
| `dsh-experimental-tool-agent-team` | **0.1.5-alpha.2** ⚠️ | 0.1.6-alpha.1 |
| `dsh-experimental-auto-review` | 0.1.6-alpha.1 ✅ | 0.1.6-alpha.1 |
| `dsh-experimental-browser-use-*`（3 个） | 0.1.6-alpha.1 ✅ | 同 |
| `dsh-experimental-computer-use-*`（2 个） | 0.1.6-alpha.1 ✅ | 同 |
| `dsh-ssh` / `dsh-fs-ssh` / `dsh-subprocess-ssh` / `dsh-sandbox-ssh` | 0.1.6-alpha.1 ✅ | 同 |

`dsh plugin add <裸包名>` 等价于 `pnpm add <裸包名>`，按 **`latest`** 解析。实测已复现错配：本机运行时 `dsh --version` = `0.1.6-alpha.1`，而 `add @deepseek-ai/dsh-experimental-agent-team-profile` 实装 **`0.1.5-alpha.2`**。

**这正是台账 #7 记载过的同源事故**（原文节选）："**add 裸包名会按 dist-tag `latest` 解析**（dsh-base 的 latest 停在已弃用 0.0.1-rc.1……）→ 404 + pnpm 递增重试 → 数分钟失败/超时"。

**影响面**：Agent Teams 恰好是方案的重点卖点之一，且其 3 个包**全部**处于 `latest` 落后状态——用方案的流程安装，用户必然拿到与运行时错配的版本。

**修正要求**：
1. 安装一律显式带版本或 tag：`dsh plugin add <pkg>@alpha` 或 `<pkg>@<运行时版本>`；
2. "期望版本"应从已安装 dsh 运行时推导（本仓库已有 `resolve.rs` 的版本闸与 `list_dsh_versions` 通道归类能力可复用）；
3. GUI 在安装确认弹窗中**必须显示将要安装的确切版本**，而非仅显示包名。

> **修正方案已实测有效**（附录 A TEST F）：`dsh plugin --profile pin add "@deepseek-ai/dsh-experimental-agent-team-profile@0.1.6-alpha.1"` 实装 `0.1.6-alpha.1`（而非 `latest` 的 `0.1.5-alpha.2`），且 `dsh.profile.bundles` 协调逻辑照常生效。即"钉版本"不破坏 B2 的自动激活，两者可叠加。

---

### B4 —— P0-2 的"新增已归档筛选"**已经实现** ⛔

**方案原文**：§2 矩阵（`:65`）与 §3.2.2（`:141-144`）把"会话管理器增加『活跃 / 已归档 / 全部』状态过滤"列为 **P0 待开发**。

**实查结论：读路径、离线字段、UI 筛选、徽章、测试全部已存在**（2026-09-07 落地）：

| 能力 | 既有实现 |
|:---|:---|
| `SessionItem.archived` 字段 | `src-tauri/src/sessions.rs:47-50`（含设计注释与日期） |
| 读 `storages/workspace.json` 的 `global.archivedSessionIds` | `sessions.rs:502-516`（`read_archived_session_ids`）、`:482-500`（解析）、`:303-308`（应用） |
| 前端筛选档 | `frontend/src/components/profiles/SessionManager.tsx:90` `useState<"all" \| "needs_repair" \| "archived">` |
| 归档可见性口径 | `SessionManager.tsx:237-241`（默认隐藏归档、搜索跟随可见性） |
| Tab 按钮 / 归档徽章 | `SessionManager.tsx:651-663` / `:340-346` |
| IPC 形状契约 | `frontend/src/types/ipc-shapes.json` 已含 `archived`；`frontend/src/types/ipc.ts:234-235` |
| i18n | `content/zh-CN.ts:548`（`filterArchived`）、`:572-573`（`archivedTag`） |
| 既有测试 | `sessions.rs:941`、`:860`、`:902`、`:2495-2517` |

**因此 P0-2 的真实增量只有一件事：`unarchive_session` 写路径。** 方案把一个 ~80% 完成的需求描述成从零开始，且其提出的分类法（"全部 / 活跃 / 已归档"）与既有实现（`all / needs_repair / archived`）不一致——照方案做会**改动已稳定的既有交互**，属于 §8.2 禁止的超范围改动。

**进一步：该能力上游已自带 UI。** `@deepseek-ai/dsh-client-ui-settings-unarchive-sessions`（`packages/client/ui-settings-unarchive-sessions/`）由 `dsh-web-app` bundle **无条件挂载**（`packages/bundle/web-app/cordis.patch.yml:251-252`，无 `disabled`/`config`），其 `src/client/ArchivedSessionsSection.tsx` 就是"已归档会话"列表＋**逐行「取消归档」按钮**，中英双语（`src/client/locales.ts:11,33`：`取消归档` / `Unarchive`）。dsh-dock 的 WebView 载入的正是这套 Web 工作台，**故用户今天就能取消归档，无需 dsh-dock 做任何事**。

**修正要求**：
1. P0-2 收敛为"仅新增 unarchive 写路径"，并显式声明**不动既有筛选/徽章/i18n/字段**；
2. 方案必须**正面论证**为什么在 dsh-dock 自己的 `SessionManager` 窗口里再做一个（唯一站得住的理由是：该窗口是独立 Tauri 窗口，够不到 Web 设置页）——否则应直接判定为**冗余需求并删除**；
3. 若保留，实现路线见 B5 的修正（优先走 Host RPC，而非文件改写）。

---

### B5 —— `unarchive_session` 的签名与同步面不完整；且这是一处**新的 dsh 存储域写面** ⛔

方案 §3.2.2（`:130-139`）给出的实现：

```rust
#[tauri::command]
pub async fn unarchive_session(session_id: String) -> Result<bool, String> { … }
```

**三个问题**：

1. **签名缺 `app: tauri::AppHandle`**。本仓库全部会话命令都带它，因为 ADR-0016 的宿主/客体分派依赖它：`src-tauri/src/commands/session.rs:14-16,38-40,67-69,92`，分派形态为 `let world = crate::mgmt::current_world(&app)?;` ＋ `match world { World::Local => …, World::Wsl { distro } => … }`。无 `AppHandle` 则**无法解析 WSL 客体世界**，与"控制中心跨环境一致"（ADR-0016）直接冲突。
2. **同步面不是"三处"**。AGENTS §7 的"三处同步"指 COMMANDS → handler → capabilities；但机器闸门实际覆盖 **4 处名字面 ＋ 1 处形状面**：
   - `tauri.ts` 名字面：`src-tauri/src/ipc.rs:292 tauri_ts_matches_ipc_commands`（失败文案："前端第 5 消费面漏了"）；
   - 形状面：`ipc.rs:381 ipc_struct_shapes_match_fixture` 对 `frontend/src/types/ipc-shapes.json`；
   - 另有 `ipc.rs:311 no_direct_invoke_outside_tauri_ts`。
   方案 §3.2.2 与 §5（`:270`）均只列三处，且路径写作 `src/ipc.rs`（仓库根视角应为 `src-tauri/src/ipc.rs`）。
3. **写 `storages/workspace.json` 是全新的 dsh 存储域写面，方案未做任何治理登记**。今天该文件在仓库内**严格只读**（生产代码唯一触碰点是 `sessions.rs:509`）。新写面须同时满足：
   - AGENTS §6「写入例外册」登记（该册是"新增字段须先在此登记"的登记制）；
   - `docs/contracts/dsh-behavior-ledger.md` 新增复现点行（格式见该文档 §一 的 5 列表头；参照行 11 的写法）；
   - 立项 ADR（见 §4）。

**另有一项方案完全未考虑的并发风险（重要）**：`storages/workspace.json` 是 dsh **内存权威状态的投影**——`packages/storage/storage-json/src/format.ts:5` 明说"the file is always the current net state"，`single-unit.ts:1-5` 明说"The in-memory state is authoritative"，`atomic.ts:1-10` 明说"a unit file has **exactly one writer per process and last-write-wins is correct**"。**若 dsh 正在运行，其下一次 `set()` 会整体覆盖壳刚写入的改动**。方案 §3.2.2 的 TIP（`:146-147`）只论证了"幂等"，未论证"并发安全"。另外该文件带版本戳头（`format.ts:63,68`：`unit.name` 不符报 `missing or foreign unit header`，`version` 不匹配报 `version-mismatch`），手改必须逐字保留 `unit` 与 `tables`——错一处即**整个工作区功能报错**。

**修正路线（二选一；强烈推荐 A）**：

- **路线 A（推荐）：复用 Host RPC，完全不碰文件。** `unarchiveSession` 已作为 **Remote 方法**暴露：`packages/api/workspace-controller/src/index.ts:118-121` `@Remote('unarchiveSession')`，服务以 `{ namespace: 'workspace' }` 注册（`:35,43`），且上游 Web 设置页正是走这条路。dsh-dock **已经在用同一条 typert 回环通道**（`plugins.rs::fetch_runtime_snapshot` 的 `POST 127.0.0.1:<port>/api/…`，信封 `{type:"client-request",rpcId,method,payload:{args:{}}}`，台账 #11）。走这条路的收益是决定性的：**不引入新写面、不耦合存储格式与版本戳、不存在 last-write-wins 竞争、且无需新增 AGENTS §6 登记与台账行**（仅需按台账 #11 口径登记这条 RPC 用途＋网络闸门豁免行的复用）。代价：必须有活跃 Host（与既有回环查询的前置一致；无 Host 时如实提示用户"请先启动"而不是绕路改文件）。
- **路线 B（不推荐，仅在前者不可行时考虑）：文件改写。** 若确需落地，必须：仅在无活跃 Host 时执行；写入前校验 `unit.name === 'workspace' && unit.version === 2`；只在 `global.archivedSessionIds` 内做增删并逐字保留其余字段；沿用本仓库 `fs_backup.rs` 的覆写前备份 ＋ 原子替换；并补 §4 的全部治理登记。

**修正要求**：方案必须择一写明，并把另一个明确标注为不采用及其理由。当前方案两处都没写，只给了一个既缺 `AppHandle`、又假定"直接改状态"的签名。

---

### B6 —— MCP 探测的合规路径定性错误，且漏机器闸门登记 ⛔

方案 §3.3.2（`:158`）："Tauri 后端通过 `updates.rs`（或专用 stdio 临时握手子进程）向 MCP 服务器发起标准的 JSON-RPC … 请求"；§5（`:270-271`）进一步收紧为"探测网络请求严格通过 `src-tauri/src/updates.rs` 发起"。

**四点问题**：

1. **`updates.rs` 定性错误**。它是**壳自身的** registry / 镜像链 / 发布源 / 引擎引导网络面（AGENTS §7:178-181 的登记用途清单）。MCP 服务器是用户在 `cordis.patch.yml` 里自填的任意端点，属**管理域**。把管理域探测塞进 `updates.rs` 会同时违反 AGENTS §2"`src/` 模块按职责命名"，并在形状上绕过 `network_gate.rs` 的设计意图。
2. **stdio 探测不是网络，是子进程**——两者的合规regime完全不同。`network_gate.rs:80-107` 的原语表只收 in-process 网络客户端（`ureq`/`reqwest`/`hyper::`/`tonic::`/`TcpStream`/`UdpSocket`/`"curl"`/`"wget"`），并明文把其余子进程网络能力"由 `lifecycle` 的 spawn 闸门与 AGENTS §7 登记管"。所以 §3.3.2 的括号备选与 §5 的"严格通过 updates.rs"**互相矛盾**，两者不可同时成立。
3. **HTTP/SSE 探测若不登记，`cargo test` 会直接红**。`src-tauri/src/network_gate.rs:848 production_network_primitives_are_registered`（文档头 `:1-9`）会在生产代码出现网络原语而未列入 `EXEMPTIONS` 时报错；失败文案给出的两条正规出路正是：① 能搬进 `updates.rs` 就搬；② 确需就地触网 → **先登记 AGENTS §7，再在 `network_gate.rs` 的 `EXEMPTIONS` 表加一行（含理由 + 日期）**。方案的治理清单里这两处**都没有**。
4. **子进程路径另有 spawn 闸门**。AGENTS §6:141-144："**新增 spawn 一律经 `lifecycle::spawn`/`run`**，有机器闸门拦裸 `Command::spawn()`"；seam 在 `lifecycle.rs:417` / `:465`，闸门 `lifecycle.rs:1876`；Windows 侧另须经 `crate::child_cmd`（`lib.rs:70`）。

**方案中经核实正确的部分**：MCP 方法名 `resources/list` 与 `resources/templates/list` **完全正确**（`packages/mcp/mcp-client/src/connection.ts:372,376`）。transport 实为 `stdio | streamable-http`（SSE 即 streamable-http 的别名，`packages/mcp/mcp-client/src/index.ts:49-95`），方案未提 transport 维度——而**这正是决定走"网络登记"还是"子进程登记"的分叉点**，必须在方案里显式建模。

**参考先例（方案应引用而未引）**：`plugins.rs:258-275 fetch_runtime_snapshot` 是仓库内最贴近的探测范式——回环、只读、2s 超时、一次性快照不订阅、仅活跃会话；且其豁免在 2026-09-11 被**从整文件收窄为条目级**（`network_gate.rs:170-180`），理由正是"整文件豁免会让将来任何人往本文件加第二处触网都不被拦下"。MCP 探测应照抄这一"条目级 + 登记制 + 有超时"的形态。

**第五点——⛔ 方案的功能前提本身需要重新论证：dsh **没有**可供 GUI 枚举 MCP 的 RPC。** `packages/api/`（9 个 API 包）中 **`mcp` 命中数为 0**；`mcp-resources` 注册的是**面向模型**的三个工具（`list_mcp_resources` / `list_mcp_resource_templates` / `read_mcp_resource`，`packages/mcp/mcp-resources/src/tools.ts:35,43,57`），且 `grep '@Remote' packages/mcp/*/src/*.ts` 只命中 `mcpResources` 服务注册。DSH 自己通过 MCP SDK 完成 `initialize`/`tools/list`/`resources/*`（`connection.ts:308,372-381`；`tools.ts:121-123`）。**结论**：GUI 要枚举 MCP，只有两条路——① 读/写 profile patch 文件（即"静态配置视图"，正是 `McpManager` 今天已做的事）；② 自己说 MCP 协议（即本节的探测实现）。方案 §3.3.1 的动机表述（"用户需要知道配置的 MCP 到底暴露了哪些资源"）在**模型侧**其实已有答案（三个内置工具），因此方案应先论证**为什么桌面 GUI 的探测比"直接问模型"更有价值**（例如：配置体检、连接失败归因、无需消耗 token），否则 P1-1 的价值主张不成立。这一点与 roadmap §4.7 的"工具列表与连接状态查看"未关闭项是同一件事，方案应承接该条目而非另起。

---

### B7 —— SSH 配置模式缺必填字段，前置条件被严重低估 ⛔

方案 §3.4.2（`:180-188`）给出的自动化 Profile 生成 YAML 为 `{host, node, workspace}`。**上游权威字段表**（`packages/ssh/ssh/README.md`）：

| 字段 | 默认 | 含义 |
|:---|:---|:---|
| `host` | **required** | 已存在的 OpenSSH **host alias** ✅ 方案写对 |
| `node`、`helper`、`workspace` | **required** | 远端 node 绝对路径、**bundled helper 入口**、默认 workspace ⚠️ 方案缺 `helper` |
| `helperHash` | **required** | 已安装 helper 入口的小写 SHA-256 ⚠️ **方案完全未提** |
| `bootstrapPath`、`bootstrapHash` | 省略 | PTC 配套；"Basic filesystem and Bash use may omit the pair" |
| `requestTimeoutMs` / `maxFrameBytes` / `maxPending` / `leaseMs` | 30000 / 64 MiB / 128 / 30000 | 连接与租约调参 |

**方案缺失的前置条件（远比"探测 node 版本 ≥ 20"重）**：

1. **必须在远端预装 helper 及其运行时依赖**，且 helper 与依赖须**留在 workspace 与可写临时根之外**（还须避开 bwrap 私有 `/tmp` 这类后端替换树），并核对 SHA-256。方案的向导第 2 步只写"轻量 SSH 连接探测远程主机是否存在 `node`（版本 ≥ 20）以及预备工作区目录"，把最重的一步整个略去了。
2. **两端都必须是 Linux 或 macOS**（"Both endpoints require Linux or macOS"）。即：Windows 宿主**不能**用作 SSH 远程工作区。方案面向三平台 Tauri 应用却未标注此平台限制，与 AGENTS §1 的三平台 CI 口径需要对齐。
3. **须组合四个包**：`dsh-ssh`（连接，`ctx.ssh`）＋ `dsh-fs-ssh`（`ctx.fs`）＋ `dsh-subprocess-ssh`（`ctx.subprocess`）＋ `dsh-sandbox-ssh`（`ctx.sandbox`）。`packages/ssh/ssh/README.md`："**Compose this service with** `fs-ssh`, `subprocess-ssh` and `sandbox-ssh`"。方案 §1/§3.4 虽列出了这四个包名（这点正确），但其 YAML 示例只注册了 `@deepseek-ai/dsh-ssh` 一个——**只给连接服务、不给文件与进程服务，等于挂了空壳**。
4. **交互式密码无路可走**：服务启用 `BatchMode`、强制严格 host-key 校验、**禁用 agent forwarding**、不提供交互式认证流程。因此方案 §3.4.2 的 WARNING（`:191-192`）方向正确（"必须前置连通性校验"），但应把结论写死为"**必须在向导首步就以非交互方式验证 `ssh -o BatchMode=yes` 可达**，否则直接劝阻"。
5. **若未采纳 B1 的修正，上述 YAML 因缺 `id` 会被整体丢弃**（见 B1），进一步放大。
6. **⛔ 方案 §3.4.2 第 4 步"用户直接选择该 Profile 启动，透明进入远程开发模式"与上游范围声明冲突。** 上游明文限定该家族的适用范围：`docs/subsystems/ssh.md`（Composition scope）—"**Web workspace views that assume host filesystem access need separate integration; replacing providers alone does not make those views remote-aware**"；同一口径见 `packages/ssh/ssh/README.md:93` 与 `.agents/notes/implemented/architecture/2026-09-11-posix-ssh-runtime.md:45`（"The initial composition scope is **POSIX headless and custom profiles**"）。即：**把 SSH 提供方塞进 web profile，Web 工作台的文件/编辑器视图仍按宿主本地路径工作，不会自动变成远端视图**。因此"透明进入远程开发模式"这个卖点**当前上游不成立**——真实可用形态是 **headless / 自建 profile**（例如 `dsh --profile remote-dev-box "<task>"`）。方案必须改写这一承诺，或明确降级为"面向 headless 场景"。
7. **上游不存在任何 ssh 预设/示例可参考**：六个 bundle patch（`packages/bundle/{base,web-app,headless,sdk-app,sdk-minimal,acp-app}/cordis.patch.yml`）、全部 agent preset、`apps/cli/config/examples/`（仅 `cordis`/`github-review`/`mcp-memory`/`schedule`）均无 ssh 行。故方案是本仓库首个落地者，**没有上游范本可抄**——这本身是应写入方案的风险声明。

> [!NOTE]
> **一处方向相反的更正（对方案有利）**：本仓库**已有预留 seam**，I4 是"填空"而非"新造"。`src-tauri/src/executor.rs:42,50` 已有 `ExecutorKind::Ssh` 预留变体；`:161,178` 已有 `pub struct SshConfig` 预留形状；模块头 `:17` 明记"SSH 为预留：`SshConfig` 形状先定型（本期不实现），执行逻辑留后续版本"；`:170` 明记"当前未接到会话路径上"；`:1264` 已有 `assert_eq!(ExecutorKind::Ssh.as_str(), "ssh")`。这与 roadmap §4.8 `:206` 的"`SshConfig` 形状已预留"一致。**但** `grep -rn "\.ssh" src-tauri/src/` 为 **0 匹配**——即确实**不存在任何 `~/.ssh/config` 读取**，该读取仍是首个 `$DSH_HOME` / `app_data_dir()` 之外的路径读取。故"新文件系统范围须登记"的结论**成立**，而"无任何先例可循"的说法**不成立**，方案应改为援引既有预留 seam。

---

## 3. 文档准确性问题（中等，须逐条订正）

| # | 方案位置 | 原文 | 实况 |
|:--:|:---|:---|:---|
| 1 | `:64` | `frontend/PluginHub` | 组件存在但路径错：`frontend/src/components/market/PluginHub.tsx:15` |
| 2 | `:68` | `frontend/QuickRunnerWindow` | **不存在**。全仓 grep（含 `QuickRunner`/`quick_runner`）唯一命中就是本方案自身 |
| 3 | `:66` | `src-tauri/commands/mcp.rs` | **不存在**。MCP 命令在 `src-tauri/src/commands/console.rs:170/189/209`；`commands/mod.rs:6-15` 无此模块 |
| 4 | `:68` | `src-tauri/commands/runner.rs` | **不存在**（同上） |
| 5 | `:64-67` | `frontend/MarketplaceView`、`frontend/SessionManager`、`frontend/McpManager`、`frontend/ProfileCreateDialog` | 均缺 `src/components/{market,profiles}/` 前缀 |
| 6 | `:113` | "dsh-dock 的 `InstallFlight`… 自动注入配套服务" | **定性错误**。`frontend/src/components/market/InstallFlight.tsx:20` 是 Framer Motion 飞行动画层（"从点击处飞一颗胶囊到下载管理触发按钮"），**零安装参数逻辑**。真实链路是 `MarketInstallDialog.tsx:102` → `queueStore.ts:60` → `tauri.ts:99` |
| 7 | `:112` | 在 `types/market.ts` 扩展 `isOfficial` 字段与 `official` 分类 | **方向正确但方案未说明迁移面**。`market.ts` 现无此字段（`MarketPlugin` 共 12 键）；"官方"概念**已存在为局部启发式**：`MarketPluginCard.tsx:50` `owner.toLowerCase().includes("deepseek") \|\| name.startsWith("@deepseek-ai/")`，并已渲染官方徽章（`:61-62`、`:87`）＋ i18n `t.market.officialCoreTitle`。用权威目录取代启发式是**改进**，但必须同步迁移该卡片与徽章渲染，否则双源。另：`MarketPluginCard` 消费的是社区 registry 的 `MarketPlugin` 形状（含 `npm`/`install`/`stars`），官方目录为非 registry 来源，须决定复用该形状还是另立类型 |
| 8 | `:127`、`:270` | "IPC 命令三处同步（… `capabilities/`）" | 名称面实为 **4 处**（另含 `frontend/src/lib/tauri.ts`，闸门 `ipc.rs:292`）＋形状面第 5 处（`ipc.rs:381`）；路径 `src/ipc.rs` 应为 `src-tauri/src/ipc.rs` |
| 9 | `:222` | Gantt 标"方案评审与 ADR 确立 **:done**" | **不实**。`docs/adr/` 止于 0019，无任何 ADR 覆盖本方案 5 个特性 |
| 10 | `:67` vs `:250` | §2 把 `~/.ssh/config` 读取归于 `frontend/ProfileCreateDialog` ＋ `src-tauri/profiles.rs`；§4 又说"后端读取" | **自相矛盾**；按 §4.4 红线 2 必须在后端（前端发起即违规） |
| 11 | `:158` vs `:271` | §3.3.2"updates.rs 或 stdio 子进程"；§5"严格通过 updates.rs" | **自相矛盾**（见 B6） |

**另需注意的方案外事实**：方案自称参考"官方公开 API"，但 ADR-0008 **不含**前端依赖白名单条文（白名单唯一权威在 AGENTS §4.4 rule 1，ADR-0008 只是来源）；若方案后续按其行文引用 ADR-0008，需改为引 AGENTS §4.4。

---

## 4. 治理缺口（Go/No-Go 前必须补齐）

### 4.1 立项 ADR（AGENTS §9："影响契约 / 架构 / 安全边界的决策必须**先立 ADR 再动代码**"）

| # | 需立 ADR 的决策 | 触发理由 |
|:--:|:---|:---|
| 1 | **官方插件目录 ＋ 配套包自动注入** | 改变安装语义，受 ADR-0011「安装来源三形态白名单」约束 |
| 2 | **`storages/workspace.json` 写面 ＋ 取消归档语义** | 首次写入 dsh 存储域（非 profile 文件）；并发/覆盖语义未定 |
| 3 | **MCP 探测的网络/子进程面** | 新网络需求，受 ADR-0006 约束；须定 transport 分支与超时 |
| 4 | **SSH 远程工作区**（含会话级 capability 收敛） | 新文件系统域（`~/.ssh/config` 在 `$DSH_HOME`/`app_data` 之外）＋ 子进程 ＋ 隧道 |
| 5 | **OS 全局热键 ＋ 新悬浮窗** | 新 Cargo 依赖（`tauri-plugin-global-shortcut`，当前 **Cargo.toml 中没有**）＋ 新窗口（`tauri.conf.json` **无 `app.windows` 数组**）＋ `capabilities/default.json:5-9` 的 `windows` 白名单须扩 |

> 附带：`docs/adr/0019-dsh-client-module-proxies-on-windows.md` 已存在于磁盘却**缺席 AGENTS §9 索引表**（表末行是 0018）。本方案须新增 ADR，正好一并修这处索引漂移。

### 4.2 登记制条目

| 登记处 | 需新增/修改 | 依据 |
|:---|:---|:---|
| `AGENTS.md` §7 | MCP 探测的网络/子进程用途；`~/.ssh/config` 读取范围 | §7"其余模块禁触网，新网络需求先在此登记" |
| `AGENTS.md` §6 | `storages/workspace.json` 新写面（写入例外册） | §6"新增字段须先在此登记" |
| `src-tauri/src/network_gate.rs` `EXEMPTIONS` | MCP 探测行：stdio-only 用 `Kind::Registered`；in-process 用条目级 `Kind::Exempt`；reason 须含 `AGENTS §7` 或 `ADR-`，date 为 `YYYY-MM-DD` | `network_gate.rs:147-199`、`:798-823`。注意 `:931` 会强制 Registered 文件**不得**出现 in-process 原语 |
| `docs/contracts/dsh-behavior-ledger.md` | 新行：`storages/workspace.json` 的**写**语义（读结构已有旁证）；扩行 7/13：官方 `dsh-experimental-*` 包集与配套关系 | 台账 5 列表头 `:14-15`；参照行 11 写法；§三 append-only 复核记录 |

**明确无需变更（应写进方案，避免后人重复论证）**：`docs/contract.md` 与 `MANIFEST_FORMAT` **不需要改**。该契约治理的是"装配方 ↔ 产品壳"接口（manifest schema、快照布局、`format: 3`），而本方案 5 个特性全部落在 `$DSH_HOME` 内的管理面。

### 4.3 roadmap 收口（方案与既有陷阱清单直接相关）

| roadmap 位置 | 内容 | 与方案关系 |
|:---|:---|:---|
| §4.6 `:188-194` | "「工作区增删管理」子项未见对应 IPC，**未随本次回收关闭**" | 正是 P0-2 的另一半 |
| §4.7 `:196-202` | "「MCP 工具列表与连接状态查看」子项未见对应实现，**未随本次回收关闭**" | 正是 P1-1；方案将其当新想法 |
| §4.8 `:204-209` | SSH executor 变体；`:208`"**需要设计会话级 capability 收敛**（远端会话拒绝 upgrade 类动作）" | P1-2，方案**完全遗漏** capability 收敛要求 |
| §1 `:31` | patch 逐字段赋值、写 `config` 即整体替换 | 方案 §3.1.1/§3.4.2 的写入直接受约束 |
| §1 `:41-43` | `profiles/node_modules` 符号链接农场**壳不得直写** | P1-2 生成 profile 时须避开 |
| §5 `:331` | **不做清单**："dsh 自身功能扩展（会话管理、插件市场、模型配置等）\| 是 dsh 本体的事，壳只负责呈现和文件层面的管理" | ⚠️ 该行同时点名"会话管理"与"插件市场"，故需逐特性判定边界：P0-1（官方插件目录）与 P0-2（取消归档写路径）落在"**文件层面的管理**"内、且有既有先例（`plugins.rs::PatchFile`、`sessions.rs` 只读扫描）可援引；但 **P2「桌面任务快跑器」越过该线**——它要"通过 `dsh --profile headless` 执行任务并流式渲染思维链"，属**任务执行前端**而非"呈现 + 文件层面管理"。须按 AGENTS §8.6/§8.7 提请**裁定**，不得静默纳入 |

### 4.4 流程合规（AGENTS §8）

- **违反 §8.1/§8.2**：方案把 **5 个特性、12 个工作项、跨 6 个模块**（前端 `market/`＋`profiles/`＋新窗口；Rust `plugins.rs`/`sessions.rs`/`mcp.rs`/`profiles.rs`/`updates.rs`/新 `commands/runner.rs`）打包成一条 2026-09-15 → 2026-10-10 的时间线。§8.1 要求"一次会话只做一个明确意图"，§8.2 要求"禁止跨模块批量改动"并**要求 AI 拒绝超范围改动、记入计划另行开工**——本方案应拆为 5 个独立增量。
- **违反 §8.3**：`：245` 的测试义务只有一个复选框"自动化测试覆盖（Rust 单元测试 ＋ 前端纯逻辑测试）"。§5 要求逐项义务："**必带测试**：URL/导航解析（含恶意反例）· 契约字段（正反例各一）· bug 修复（复现先行）· 跨平台分叉"。
- **违反 §8.5**：任务清单无 `docs/broadcasts.md` 落档、无台账登记、无 roadmap 更新、无 ADR 索引更新。
- **§11.2 归属**：方案自身承载的陷阱（patch 语义、版本漂移、bundle 分类）按 §11.2 应落 `docs/roadmap.md`，且 §11.3 禁双源。当前 `docs/plans/` 为**新建未入库目录**，需先确认其在本仓库的定位与约定。

---

## 5. 经核实**正确**的部分（修订时应保留）

审核价值同样在于确认哪些能站住。以下均经一手取证：

| 方案主张 | 核验结论 |
|:---|:---|
| 实验包名 `@deepseek-ai/dsh-experimental-*` 全部真实 | ✅ 目录与 `package.json` 实查一一对应 |
| 这些包可从 npm 安装 | ✅ registry 实查：全部已发布（`publishConfig.access: public`），**无一为 `private`** |
| `dsh --profile headless [--json] [--session-id <id>] [task... \| -]` | ✅ **逐字正确**（实机 `--help`；`startup.ts:43`）。`-` 读 stdin、`--json` 输出 NDJSON、`--session-id` 采纳已持久化 Session——**方案 P2 的调用式可用** |
| headless 是真实出厂 profile | ✅ `dsh --profile headless --help` 直接可用；`profile.ts:149` 的 `PROFILE_TEMPLATES` 含 `['@deepseek-ai/dsh-base','@deepseek-ai/dsh-headless']` |
| NDJSON 事件名 `thinking` / `tool_call` / `tool_result` / `text` / `final` | ✅ 全部正确（`json-stream.ts:274,275,294,306,325`） |
| `WorkspaceRegistry.unarchiveSession(sessionId)` 真实存在 | ✅ `packages/workspace/workspace/src/index.ts:266-275` |
| `archivedSessionIds` 是真实字段名 | ✅ `index.ts:232`、`spec.ts:55` |
| MCP 方法名 `resources/list`、`resources/templates/list` | ✅ `connection.ts:372,376` |
| Browser Use 需 `@deepseek-ai/dsh-browser-use` 作前置 | ✅ 官方示例正是两行组合（`browser-use-playwright-mcp/README.md`） |
| `cordis.patch.yml` 写入统一走 `plugins.rs::PatchFile` | ✅ 该原则正确且已被 AGENTS §6:131 登记；`PatchFile` 定义在 `plugins.rs:1284` |
| Web 侧边栏终端随内核自动可用 | ✅ `@deepseek-ai/dsh-client-ui-sidebar-terminal` ＋ `api-terminal-controller` 已在 `dsh-web-app` bundle 依赖内。**但**：`dsh-base` bundle 不含 terminal 依赖，故该能力**绑定 web profile**——方案宜补此限定 |
| 两条红线的遵守意识 | ✅ §5 风险章节表述正确 |

**P2 需补的三处技术精度（否则"流式打字机"做不出来）**：

1. **事件类型漏三种**。方案列的 5 种外还需处理 **`session`**（开篇，含 `sessionId`/`cwd`）、**`status`**（`turn_start`/`step_start`/`step_end`/`turn_end`，`step_end` 可能带 `usage`）、**`error`**（`startup.ts:84`；"A failure the runner raises outside a turn writes an `error` event and ends the stream without `final`"）。完整集合共 **8 种**，契约见 `packages/bundle/headless/README.md:58`。失败信号应取"退出码 1 ＋ `turn_end` 的 `reason`"。
2. **"流式打字机"不成立**。`json-stream.ts:1-6` 文档头明说："Every projected event is a commit point: text and reasoning come from **committed** `assistant/message` content, **never from a live attempt**"。即 `text`/`thinking` 是**按已提交消息块**产出，**不是 token 级增量**。方案的"流式打字机输出"是做不到的；只能做"逐块追加"，或改为消费 live attempt（但那会引入被重试/丢弃的内容）。
3. **截断与丢事件必须处理**。`MAX_STRING_BYTES = 8 KiB`（单串）、`MAX_EVENT_BYTES = 32 KiB`（单事件，终止事件 `final` 豁免）——超限会被截断并置 `truncated` 标记；且 `tool_result` 在 `event.surfaceOp !== 'append'`（被压缩替换）时**直接丢弃**（`json-stream.ts:300-303`），因此 UI 必须容忍"有 `tool_call` 无对应 `tool_result`"。
4. **`--session-id` 有明确的拒绝集，方案的"持久会话"设想需按此建模**。未知 id 是错误；此外**拒绝** subagent/forked 会话、无记录 `cwd` 的会话、preset 不匹配的会话、以及**已在本进程内活跃**的 id（`packages/bundle/headless/README.md:54,145`）。故"快跑器复用一个持久会话 id"必须处理：会话当前是否正被 Web 工作台占用（冲突时的降级路径要写清）。
5. **`-` 必须是唯一的任务参数**：`startup.ts:97-99` 规定 `` `-` `` 只能单独出现，否则报错。方案 §3.5.2 的 `dsh --profile headless --json --session-id <id> -` 形式**合法**，但实现时不可再追加位置参数。

---

## 6. 修订清单（Checklist，逐项勾选后方可重审）

### 6.1 阻断项（必改）

- [ ] **B1** 把所有插件挂载改写为 `- insert: [{id, name, config}]`；覆盖既有行才用 `- id:`/`- name:`。并在方案里写明二者语义边界。
- [ ] **B1b** 写 `config` 前先读该行现有完整 config 再回写（整体替换语义）。
- [ ] **B1c** 每个生成的 `insert` 行**自带稳定可复算的 `id`**（缺 `id` 会被自动生成，该行此后永不可被 patch 命中）；并记录真实 schema 无 `before`/`after`、顺序即数组位置。
- [ ] **B2** 引入"目标包是否声明 `dsh.bundle`"的分支：bundle 类只覆盖配置、plain 类才 `insert:` 挂载；并在方案中给出分类表。
- [ ] **B2b** 补全 Agent Teams 组合（`agent-team` ＋ `tool-agent-team` ＋ 会话持久化，Web 侧走 `agent-team-profile` / `agent-team-web-profile` ＋ `client-ui-agent-team`）与 `browser-use-runtime`（库、不可挂载）。
- [ ] **B2c** 目录 UI **强制 provider 互斥**：浏览器三选一、桌面控制二选一（装第二个会激活失败），提供"替换"语义而非"叠加"。
- [ ] **B2d** 把"首批官方插件"改写为"dsh-dock 策展集"（上游为全量发布 16 包，无"首批"概念）；并注明 5 个 provider 无上游 `dsh plugin` 范本。
- [ ] **B3** 安装一律钉版本/tag；版本由已装运行时推导；安装弹窗显示确切版本。
- [ ] **B4** P0-2 收敛为"仅新增 unarchive 写路径"，显式声明不动既有筛选/徽章/i18n/字段；并**正面论证**为何上游 Web 设置页已有的"取消归档"还需在壳内重做（否则判定为冗余需求删除）。
- [ ] **B5** 择一并写明实现路线：**A（推荐）复用 Host RPC** `workspace.unarchiveSession`（不碰文件、无需新登记）；**B（次选）文件改写**（须无活跃 Host ＋ 版本戳校验 ＋ 备份原子写 ＋ 全套治理登记）。无论哪条都要补 `app: tauri::AppHandle`、`tauri.ts` 与 `ipc-shapes.json` 同步面。
- [ ] **B6** MCP 探测按 transport 分支（`stdio`→`lifecycle` seam ＋ `Kind::Registered`；`streamable-http`→条目级 `Kind::Exempt`），并写明 AGENTS §7 ＋ `network_gate.rs` 两处登记；删除"严格通过 updates.rs"的表述。
- [ ] **B6b** 重新论证 P1-1 的价值主张（模型侧已有三个 MCP 资源工具；GUI 探测的增量价值须写明），并承接 roadmap §4.7 未关闭项。
- [ ] **B7** SSH 配置补 `helper`/`helperHash`；补远端 helper 预装与哈希核验前置；标注"两端须 Linux/macOS，Windows 宿主不可用"；YAML 注册四包而非一包。
- [ ] **B7b** 补 roadmap §4.8 的会话级 capability 收敛要求。
- [ ] **B7c** 改写"透明进入远程开发模式"的承诺：上游明文限定 SSH 家族适用于 **headless / 自建 profile**，**Web 工作台视图不会因替换 provider 而变为远端感知**。

### 6.2 治理项（必补）

- [ ] 5 个 ADR 立项（§4.1），并修 AGENTS §9 缺失的 ADR-0019 索引行。
- [ ] `AGENTS.md` §6/§7 登记；`network_gate.rs::EXEMPTIONS` 行；`dsh-behavior-ledger.md` 新行 ＋ 复核记录。
- [ ] 显式记录 `contract.md` / `MANIFEST_FORMAT` **无需变更**。
- [ ] roadmap §4.6/§4.7/§4.8 收口；**P2 快跑器与 §5 不做清单的冲突提请裁定**。
- [ ] 拆为 5 个独立增量（§8.1/§8.2）；每特性补测试义务（§8.3）；补 `docs/broadcasts.md` 落档（§8.5）。

### 6.3 文档订正项

- [ ] 删除/改正 §3 表格中不存在的 `QuickRunnerWindow`、`commands/mcp.rs`、`commands/runner.rs`。
- [ ] `InstallFlight` 由"安装参数注入点"改为正确链路（`MarketInstallDialog` → `queueStore` → `tauri.ts`）。
- [ ] 全部路径补全为仓库根视角；`isOfficial` 与 `MarketPluginCard.tsx:50` 既有启发式做去重决策。
- [ ] 删除 Gantt 中不实的"ADR 确立 :done"。
- [ ] 消除 §2↔§4（SSH 读取位置）与 §3.3.2↔§5（MCP 路径）两处自相矛盾。
- [ ] 补 P2 的三处技术精度（事件类型补 `session`/`status`；"流式"改为"逐块追加"；截断与丢事件处理）。

---

## 附录 A：复现实验记录

全部实验在抛离式 `DSH_HOME` 中进行，**未触碰真实 `~/.dsh`**。运行时为引擎内 `dsh 0.1.6-alpha.1`。

```sh
ENG="/Users/guan/Library/Application Support/io.github.realguan.dsh-dock/engines"
export PATH="$ENG/bin:$PATH"
export DSH_HOME="/tmp/dsh-review-home2"
rm -rf "$DSH_HOME"; mkdir -p "$DSH_HOME"
dsh --profile probe --from-default-profile web --dump-config >/dev/null   # 初始化 profile
```

**TEST A —— 裸 `- name:` 行（方案写法）**

```sh
cat > "$DSH_HOME/profiles/probe/cordis.patch.yml" <<'YAML'
- name: '@deepseek-ai/dsh-browser-use'
YAML
dsh --profile probe --dump-config > /tmp/dumpA.txt 2>/tmp/dumpA.err
```
→ 退出码 `0`；stdout 571 行；**stderr：`dsh: [<patch>] patch: id is required for non-insert patches`**；`browser-use` 命中数 **0**（未挂载）。

**TEST B —— `- insert:` 形式（正确写法）**

```sh
cat > "$DSH_HOME/profiles/probe/cordis.patch.yml" <<'YAML'
- insert:
    - id: bu-probe
      name: '@deepseek-ai/dsh-browser-use'
YAML
dsh --profile probe --dump-config > /tmp/dumpB.txt 2>/tmp/dumpB.err
```
→ 退出码 `0`；**`browser-use` 命中于第 574 行**（已挂载）；stderr 无告警。

**TEST C —— `dsh plugin add` 是否改写 `dsh.profile.bundles`**

```sh
dsh plugin --profile probe add "@deepseek-ai/dsh-experimental-agent-team-profile"
cat "$DSH_HOME/profiles/probe/package.json"
```
→ pnpm 安装 `+ @deepseek-ai/dsh-experimental-agent-team-profile 0.1.5-alpha.2`（**与运行时 0.1.6-alpha.1 错配，见 B3**）；`package.json` 的 `dsh.profile.bundles` 由 `[base, web-app]` 变为 `[base, web-app, …-agent-team-profile]`（**自动激活，见 B2**）。

**TEST D —— npm dist-tags 实查**

```sh
curl -sS "https://registry.npmjs.org/@deepseek-ai%2Fdsh-experimental-agent-team-profile" \
  | node -e "…console.log(JSON.stringify(j['dist-tags']))"
```
→ `{"alpha":"0.1.6-alpha.1","latest":"0.1.5-alpha.2","next":"0.1.5-rc.2"}`

**TEST E —— `dsh.bundle` 分类实查**

```sh
cd /Users/guan/git/deepseek-harness/packages/experimental
for d in auto-review agent-team agent-team-profile tool-agent-team browser-use-playwright-mcp \
         computer-use-cua-driver-mcp; do
  node -e "const p=require('./$d/package.json');console.log('$d', JSON.stringify(p.dsh&&p.dsh.bundle||'NONE'))"
done
```
→ `auto-review` / `agent-team-profile` = `{"patch":"./cordis.patch.yml"}`；其余 = `NONE`。

**TEST F —— 钉版本安装（B3 修正方案的自验证）**

```sh
dsh plugin --profile pin add "@deepseek-ai/dsh-experimental-agent-team-profile@0.1.6-alpha.1"
node -e "const p=require('…/profiles/pin/package.json');console.log(p.dependencies, p.dsh.profile.bundles)"
```
→ 实装 `0.1.6-alpha.1`（对照 TEST C 的 `0.1.5-alpha.2`）；`bundles` 照常追加。**结论：钉版本与 bundle 自动激活可同时成立**，B2 与 B3 的修正互不冲突。

---

## 附录 B：审核签署

- **判定**：**修订后重审**。方案的问题不是"想法不对"，而是**实现路径与上游 0.1.6 的真实契约存在系统性偏差**——尤其是 `cordis.patch.yml` 的挂载语义、bundle 自动激活、以及 dist-tag 版本漂移这三条，**全部以"退出码 0 的成功"形式静默失败**，是最危险的一类缺陷：照方案实现，测试可能全绿、UI 可能全绿，而用户装完插件发现什么都没生效。
- **建议**：先按 §6.1 修正阻断项（预计方案文档改动量小于实现改动量），同时并行启动 §6.2 的 ADR 立项；**不要**在 ADR 落地前开始写 `storages/workspace.json` 的写路径与 MCP 探测代码。
