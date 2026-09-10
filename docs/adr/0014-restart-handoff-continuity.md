# ADR-0014：重启/切换的交接带（handoff）、启动代际闸门与会话槽纪律

- **日期**：2026-09-10
- **状态**：已接受并实施（2026-09-10）
- **提出人**：guan（AI 起草；触发 = 「点击 profile 重启 → dsh 重载出来」全程无 loading 贯穿 + 子进程逃逸排查）
- **相关方**：`src-tauri/src/boot.rs`（交接状态 / 代际闸门 / 会话槽）、`src-tauri/src/commands/profile.rs`（切换编排）、
  `src-tauri/src/shell.rs`（Windows 停止语义）、`frontend/src/lib/handoff.ts`（新增纯模型）、
  `frontend/src/stores/bootStore.ts`、`frontend/src/pages/{BootIndex,ProfileManager}.tsx`、
  `frontend/src/injected/handoff-curtain.js`（新增）
- **关联**：ADR-0009（profile 生命周期，切换 = 停旧起新）、ADR-0012（boot 失败类型化，错误卡出口）；
  AGENTS §6（壳与 dsh 严格 1:1 生命周期）、§7（IPC 例外册）

---

## 1. 背景与问题

「重启」= 停当前 dsh → 以目标 profile 重新 spawn → 就绪后导航进工作台（ADR-0009 §4）。
今天这条链路上有 **三处状态断裂** 和 **两处子进程收口漏洞**，都是实测可复现的：

**断裂（用户体验）**

1. **控制中心先"说谎"再沉默**：`switch_profile` 一返回，管理器就把行徽标切到「重新加载中」，
   并且 `activeProfile === switchingTarget` 一到就解除过渡态——而 `get_active_profile`
   在 **进程 spawn 成功时就返回 Some**，早于"等待就绪 + 注入 Cookie + 导航 + 前端 bundle 加载"
   的全过程。用户看到「运行中」时，主窗口可能还停在启动屏或白屏；真失败时更糟：
   15s 兜底静默解除，控制中心不留任何失败痕迹（错误卡只在主窗口）。
2. **主窗口没有上下文**：`switch_profile` 先把 dsh 杀掉，再 `navigate` 回壳启动屏。
   旧工作台页面在死掉与导航之间无人接管；回到壳后又只渲染通用「正在启动工作台…」
   （`isSetupMode` 默认为假 → 连 5 步时间线都不显示），用户看到的是一个与刚才操作无关的空白等待。
3. **跨文档无连续性**：主窗口在 `dsh 页面 → 壳页面 → dsh 页面` 之间有两次整文档替换，
   每次都是「旧内容消失 → 新内容首帧」。没有任何元素（文案/计时/进度）跨过这两次替换。

**漏洞（子进程逃逸，审计发现）**

4. **会话槽静默覆盖**：`boot.rs::run_executor_session` 用 `*state.session.lock() = Some(executor)`
   落新会话。并发启动（双击重启、重启与模式切换/重试撞车、崩溃守护自动拉起）时，
   后到的 boot 会把先到的会话 **直接覆盖**——旧 dsh 的 `Child` 被 drop，
   既不 kill 也不 wait：进程继续跑（占端口、继续写 DSH_HOME），壳再也拿不到它，
   连 `RunEvent::Exit` 都收不走。`launch_executor_after_probe` 的 `probe_epoch` 是
   **进入函数时才读**的，两次快速切换读到的是同一个（最新的）epoch，两道都放行。
5. **Windows 停止只杀壳层**：`child_cmd` 对 `.cmd/.bat` 包一层 `cmd.exe /C`，而
   `stop_dsh` 的 Windows 分支就是 `child.kill()` = TerminateProcess(cmd.exe)。
   cmd.exe 的子进程 node **不会随之终止**（Windows 无进程组连坐）——
   「重启」在 Windows 上实际是「再起一个 dsh，旧的继续跑」。进程树必须显式收口。

## 2. 约束与硬指标

1. **不修改 dsh 源码**（AGENTS 红线 1）：所有连续性只能由壳侧注入 / 壳页面 / 壳状态实现。
2. **不新增 IPC 命令**：既有 `get_boot_status` 已承担「早期事件补水」职责，交接意图搭它的车
   （只扩 payload，不动 `ipc.rs::COMMANDS` 三处同步面）。
3. **不新增依赖**（AGENTS §1/§5）：Windows 进程树收口只用系统自带 `taskkill`。
4. **零网络**：前端三红线（AGENTS §4.4）——幕布与导轨只用本地资源，不引外部字体/图片。
5. **壳与 dsh 严格 1:1**（AGENTS §6）：任何路径都不得留下活着的 dsh 子进程，包括
   并发启动、启动中被取代、应用退出竞态。
6. **可测**：阶段模型、代际闸门、进程树参数都必须是纯函数，离线单测覆盖（AGENTS §5）。
7. **失败可见**：交接失败必须在用户所在的那个窗口可见，不得只落在另一个窗口。

## 3. 备选方案及评估

### 方案 A：单一"交接意图"贯穿 + 启动代际闸门 + 会话槽先收后落 —— ✅ 采纳

- 思路：
  - Rust 侧 `ShellState` 持有 `handoff: Mutex<Option<Handoff>>`（`target/kind/phase/startedAt/generation`）
    与 `boot_generation: AtomicU64`、`shutting_down: AtomicBool`；`get_boot_status` 暴露 intent。
  - 两个窗口用**同一个纯函数模型**（`lib/handoff.ts::deriveHandoff`）把 intent + boot:step 事件流
    折算成同一条四段导轨（停止旧会话 / 启动新会话 / 等待就绪 / 进入工作台）+ 连续计时
    （`elapsed = now - startedAt`，`startedAt` 来自 Rust，**跨文档不变**）。
  - 主窗口三段各有承接物：旧工作台页面上盖「壳注入幕布」（`switch_profile` 在 teardown **之前**
    eval）；壳启动屏由 document-start 注入脚本先画同款幕布、React 挂载即接管；进入工作台后
    注入脚本在 dsh 页面上再画同款幕布，待首帧内容出现自行淡出。
    **主窗口不新增任何界面**：重启复用它本来就有的启动屏（`BootIndex`），只换文案
    （「正在重启「X」」）与副题（当前阶段），再加一个连续计时（见 §7 修订）。
  - 启动代际：每次「开始一次启动」（首启/切换/模式切换/重试/崩溃自动拉起）`fetch_add(1)` 取令牌，
    所有分叉点（probe 后 / spawn 前 / spawn 后落槽前 / 导航前）校验令牌，被取代者**在 spawn 前静默退出**，
    已 spawn 者就地收口。
  - 会话槽纪律：落新会话前先 `take()` 旧会话并 teardown（防御性第二道）。
- 优点：一处真相源（intent）+ 一处折算（纯函数）→ 两窗天然一致；失败可见（控制中心也有 rail 终态）；
  闸门同时消灭"并发双 spawn"与"退出竞态留孤儿"两类漏洞；幕布与启动屏同构图，跨文档视觉连续。
- 代价/风险：注入脚本多一份（约 6KB，含内联鲸标 SVG）；`get_boot_status` payload 扩字段
  （向后兼容，前端按可选读）。

### 方案 B：仅做前端过渡动画（spinner/骨架屏） —— ❌ 否决

- 否决理由：治不了根因——进程真死、真重启的耗时是秒级且不可预测，纯动画只是把"干等"包装得好看；
  且控制中心依旧不知道真实阶段（`get_active_profile` 的语义就是错的时机）。

### 方案 C：把工作台内嵌为壳页面（iframe / 同 origin 反代） —— ❌ 否决

- 否决理由：越过红线与契约（dsh Web UI 的鉴权 Cookie、跨域导航策略、ADR-0002 内存策略均按
  「整文档导航」设计）；反代等于壳替 dsh 提供网络面（AGENTS §7 唯一网络面 = updates.rs）。

### 方案 D：进程组 / Job Object 收口（unix `setsid`+`killpg`、Windows Job） —— ❌ 本次否决

- 思路：把 dsh 放进独立进程组/作业对象，teardown 时连坐整组。
- 否决理由：unix 侧会连带杀死用户经 dsh 起的**长驻后台任务**（dsh 的合法用法），
  属语义变更，须独立 ADR 与实测（Windows 侧 Job Object 需新依赖 `windows-sys`）。
  本次只修「壳自己造成的逃逸」（覆盖/竞态/Windows 壳层 kill），dsh 自身孙进程留给后续专项。

## 4. 最终决策

采纳方案 A。重构「重启/切换」为**一次带代际令牌的交接**：Rust 持有交接意图并按阶段推进，
两个窗口用同一纯函数模型渲染同一条贯穿导轨与连续计时，主窗口三段各由同款幕布承接；
所有启动路径统一经代际闸门，会话槽落新之前必先收旧；Windows 停止改为进程树收口。

## 5. 后果与后续行动项

### 正面后果

- 用户点下重启后，控制中心立刻出现四段导轨 + 连续计时，主窗口横跨三次文档替换仍是同一块视觉；
- 「运行中」徽标不再早于真实可用状态，失败在用户所在窗口可见；
- 并发启动不再可能双 spawn / 覆盖丢进程；应用退出与启动竞态不再留孤儿（Windows 侧同时修掉
  `.cmd` 壳层 kill 的长期漏洞）。

### 负面后果 / 新增债务

- 幕布判定用「intent 阶段 + `active`（Rust 裁决，TTL 90s = 启动等待硬上限）」近似
  "页面已接管"，理论上在交接后 90s 内手动刷新工作台会闪一帧幕布（首帧内容出现即撤，
  用户体感为"刷新有过渡"）；
- unix 侧 dsh 孙进程仍不连坐（方案 D 留待专项）；
- 交接意图只活在内存，应用重启即丢（与"壳运行时无状态"一致，不做持久化）。

### 行动项

- [x] `boot.rs`：`Handoff` / `boot_generation` / `shutting_down` / 会话槽先收后落 / 幕布 eval
- [x] `commands/profile.rs`：切换编排改为「建意图 → 落幕布 → teardown → 带令牌启动」，返回意图
- [x] `commands/boot.rs`：`get_boot_status` 暴露 `intent`（`switch_profile` 返回值同形状）
- [x] `shell.rs`：Windows `taskkill /PID /T /F` 进程树收口（参数构造纯函数 + 单测）
- [x] 前端：`lib/handoff.ts` 纯模型 + `bootStore.intent` + 两窗 UI + `injected/handoff-curtain.js`
- [x] 契约闸门：`ipc-shapes.json` 增 `HandoffSnapshot`（Rust ↔ TS 两侧同闸）
- [x] 测试：Rust 236 → 241（交接阶段/闸门/进程树/形状）；前端 176 → 190（handoff 模型 13 例）
- [ ] 实机验证清单（`docs/executor.md`）：macOS 重启连续性、Windows 进程树收口与 `.cmd` 档位、
      WSL 客体重启（负责人待排期）
- [ ] 广播落档 `docs/broadcasts.md`

## 6. 复审条件

- dsh 提供运行时切换 profile 能力（本 ADR 的"重启语义"前提消失）；
- 主窗口改为多 WebView / 同 origin 承载工作台（幕布层失去存在必要）；
- 出现「交接后刷新误显幕布」的用户反馈（当前 TTL 90s 的代价），或任何一次实机孤儿进程
  复现（该项一旦复现，立即升级为方案 D 专项）。

## 7. 实施修订（2026-09-10，维护者当场否决首版的一处设计冗余）

首版把「四段导轨」也画进了主窗口启动屏，并为此在交接期间强制展开 5 步向导态。
维护者指出：**重启应当复用打开 App 的那个屏，而不是长得像另一个页面。** 该意见成立，
首版属"信息重复 + 形态分叉"：

- **重复**：导轨四段（停旧/起新/等就绪/进工作台）与 5 步时间线（环境检测/准备引擎/
  启动工作台/等待就绪/进入工作台）讲的是同一件事，同屏两张卡两套阶段名；
- **分叉**：`isSetupMode` 默认关（秒启走极简启动屏），强制打开后重启与开机**长得不一样**，
  正是"像新造页面"的观感来源。

修订后的分工（职责按"谁缺什么补什么"）：

| 界面 | 承接物 | 理由 |
|:---|:---|:---|
| 控制中心 | **四段导轨**（`HandoffRail`）：标题 + 四段 + 计时 + 查看进度/进入工作台 | 它没有任何启动 UI，全屏都属于"另一个窗口在忙"，必须自建一条贯穿带 |
| 主窗口启动屏 | **本来的极简启动屏**：只换标题/副题 + 加连续计时 | 它自带 loading 与时间线；重启复用它，形态与开机逐像素一致 |
| dsh 工作台页面 | 幕布（`handoff-curtain.js`） | 壳管不到它的文档 |

连带清理：`HandoffView.inflight` 接回导轨（壳判过期却未落定的意图不再转圈/呼吸，
改静态灰点——"永远在等待"比不显示更伤人）；`isSetupMode` 恢复既有规则，不再为交接开特例。

## 8. 复审条件触发（2026-09-10，append-only 补记）

§6 末条「任何一次实机孤儿进程复现（该项一旦复现，立即升级为方案 D 专项）」**已触发**：
2026-09-10 实测发现 **11 个由 `.dev` 构建逃逸的 dsh 进程**，其中 `pid 85156` 持有
`session-6521e973…` 的内核写租约，导致正式包（1.1.0）该会话永久打不开，用户只能
"删会话 + 重启应用"（证据见 `docs/known-issues/问题记录-2026-09-10-会话写锁被孤儿dsh占死.md`）。

已开专项 **ADR-0015**（草案）。但 §3 方案 D 的**否决理由本身在专项中依然有效**——
方案 D 原样照搬会连坐用户经 dsh 起的长驻后台任务，故 ADR-0015 把守护作用域收窄为
「壳生的那个 pid」（而非进程组），以此绕开本条否决理由。本节仅补记触发事实，
不改写 §3/§6 原文。
