# 📢 广播记录（公共频道知会档案）

> 协作知会的**仓库内正式载体**。依据 [`CONTRIBUTING.md`](./CONTRIBUTING.md) §0——
> 「所有必须共享的信息，唯一合法载体是仓库落盘内容」：聊天频道用于即时协调
> （占用抢先后到者得），但消息沉底即失忆、AI 冷启动读不到；知会类事件必须在此
> 落档。检索、审计、纠纷回溯一律以本档为准。
> 本文件是普通区 append-only 文档：只追加不改写历史条目（同 ADR 惯例），
> 不走宪法级修改流程。

## 一、登记范围（触发即记，当天落档）

| 类型 | 触发点 | 依据 |
|:---|:---|:---|
| 宪法级文件改动 | `AGENTS.md` / `docs/contract.md` / `CONTRIBUTING.md` / `node-map/` 的改动预告与合入归档 | AGENTS §10；CONTRIBUTING §3 |
| 快车道直推 master | 直合完成即记（单人小改通道同样适用） | CONTRIBUTING §2 流程图 I→M |
| PR 合并完成 | 合并人 squash 后记录 reviewer 与结论 | CONTRIBUTING §2 流程图 M |
| 发版事项 | 打 tag、冻结期起止、Release notes 征集与三平台验收 | CONTRIBUTING §8 |
| 占用声明/释放 | 频道声明后补一行即可（时效判定仍以频道时间戳为准） | CONTRIBUTING §3 |

## 二、条目格式

倒序追加（最新在上），一条一个三级标题：

```
### YYYY-MM-DD <类型> · <一句话主题> —— <发起人>
- 变更：<commit hash / 分支 / 文件清单>，两三行摘要
- 影响：<需要他人做什么动作；无需动作写「仅周知」>
- 凭据：测试结果 / diff 规模 / 频道消息时间点
```

漏记不补改旧条目——另发一条「补记」并注明原委。

## 三、记录

### 2026-09-23 发版 · v1.3.3（Windows 窗口回退 + macOS 拖拽区权限修复）—— guan（AI 协作）

- **内容**：两笔修复 —— `724fa01`（Windows 撤回沉浸式标题栏，issue #16）+ `c972c17`
  （macOS 补 `core:window:allow-start-dragging`）；本笔 `release:` 落版本号与发版日志。
- **版本一致性**（CI 的 tag 闸门同款口径，四处齐为 1.3.3）：`src-tauri/Cargo.toml` ·
  `src-tauri/tauri.conf.json` · `frontend/package.json` · `src-tauri/Cargo.lock`。
- **发版日志**：`docs/RELEASE_NOTES.md` 顶部 `## [v1.3.3] - 2026-09-23`；
  `scripts/extract-release-notes.py v1.3.3` 提取通过（1176 字符）。
- **凭据**：`cargo test` 578 passed + `fmt --check` / `clippy -D warnings` 干净；
  前端 `typecheck` ✓ · `oxlint` 0 warning · `vitest` 678 passed。
- **待办 / 未做（明确边界）**：
  1. **未打 tag、未推送** —— 按本轮裁定留给维护者执行（CI 三平台构建与产物验收随之启动）。
  2. **macOS 拖拽区实机验证未做** —— 发版说明已如实披露该边界；验收不通过则按 ADR-0030 §5
     回退该授权并另出说明。
  3. v1.3.2 用户需升级到本版才恢复 Windows 窗口的移动 / 缩放 / 关闭能力。

### 2026-09-23 缺陷修复 · macOS 拖拽区补 ACL 授权（ADR-0029 的翻译此前一直是死的）—— guan（AI 协作）

- **发现**（复核 issue #16 时顺带，与 Windows 档无关的独立缺陷）：ADR-0029 的
  app-region → `data-tauri-drag-region` 翻译，最终由 Tauri 自己的 `drag.js` 在单击时
  invoke `plugin:window|start_dragging`；而该命令**不在** `core:window:default`
  （tauri 2.11.5，该默认集 = 28 条只读 getter + `internal_toggle_maximize`），
  capabilities 又只授了 `core:default` ⇒ **自 v1.3.0 起该翻译在真机上从未生效**：
  能拖的只有系统原生标题栏那一条带，dsh 自绘的 topStrip / titleRow 拖拽区是死的
  （双击最大化走 `internal_toggle_maximize`，在默认集内，故只有单击拖拽失能 ——
  这也是它没被日常使用暴露出来的原因）。
- **处置**：`capabilities/default.json` 补 `core:window:allow-start-dragging`；新增配对闸门
  `immersive_drag_acl_tests`，把「机制 → 权限」这条链钉死（正 = 权限在册 + `core:default`
  在册；反 = 脚本仍须设置拖拽属性且 `drag → deep`）——直接针对 issue #16 的教训：
  脚本调命令 + capabilities 没授 + `.catch` 吞掉 = 用户侧「没反应」。代码笔 `c972c17`。
- **影响 / 需要他人做什么**：
  1. macOS 用户的行为变化 = 顶栏 / 侧栏条带的单击拖拽开始生效；
     **但「拖得动」本机无法证明** —— 需 macOS 实机验证（顶栏/侧栏条带拖动 + 双击最大化），
     **验证前不得对外宣称已修**（ADR-0030 §5 行动项）。
  2. 权限面变化：多一条 `core:window:allow-start-dragging`（remote 授权仍限
     `http://127.0.0.1:*`）；无新增自研 IPC、无新增网络面。
- **凭据**：`cargo fmt --check` ✓ · `clippy --all-targets -D warnings` ✓ ·
  `cargo test` 578 passed（+2 新闸门）· **负例实测**：临时摘掉该授权 →
  `drag_region_permissions_are_granted` FAILED（报错点名权限与 tauri 默认集），
  还原后恢复绿。

### 2026-09-23 缺陷修复 · issue #16 —— Windows 撤回沉浸式标题栏（窗口不可移动 / 缩放 / 关闭）—— guan（AI 协作）

- **触发**：Windows 用户 issue #16「Windows平台 web界面的 窗口没法移动 最大化 最小化
  关闭按钮均无效果」。该特性随 v1.3.2（2026-09-22）发布，次日即被真机证伪。
- **根因（三条独立断点，全部可在本机复核）**：
  1. **窗口拖不动**：dsh 的 40px 拖拽带是**伪元素**
     （`packages/client/ui-layout/src/client/AppFrame.module.css:40-46` 的
     `.frame::before{ -webkit-app-region: drag }`），承载不了 `data-tauri-drag-region`；
     注入脚本的 Windows 分支又完全没做 app-region 翻译。ADR-0029 §6 早已写明此处
     「**另立评审**」，进场时未走。
  2. **三控件点了没反应**：自绘控件调 `plugin:window|minimize / toggle_maximize / close`，
     三者均**不在** `core:window:default`（tauri 2.11.5，本仓构建产物
     `src-tauri/gen/schemas/acl-manifests.json` 可直接核对），而 capabilities 只授了
     `core:default` ⇒ ACL 拒绝 + 调用点 `.catch(function () {})` 吞错。
  3. **缩放 / 贴靠退化**（issue 未提，同源）：`decorations(false)` 在 tao 里摘掉
     `WS_CAPTION | WS_THICKFRAME`（`tao-0.35.3/src/platform_impl/windows/window_state.rs:307`），
     而官方 Electron 的 `titleBarStyle:'hidden'` 是**保留原生 frame** 的 ⇒「等价映射」不成立；
     该提交自己写的验收判据「缩放/贴靠退化即回退」当场命中。
  **闸门为何没拦**：原闸门只断言注入脚本里**出现** `minimize()` / `toggleMaximize()` /
  `close()` 这些**字符串**，不断言 ACL 有对应授权 —— 结构性通过、行为性失效（平台审计
  点名的「验证空洞」一类）；而「待 Windows 真机验收」自陈未完成即随 v1.3.2 发版。
- **处置**（维护者 2026-09-23 裁定，[ADR-0030](../adr/0030-windows-keeps-native-decorations.md)）：
  **Windows 与 Linux 同口径维持原生装饰**。`ui.rs` 删 `decorations(false)` cfg 块并在原位留下
  撤回理由（避免后人「照官方方案」再加一次）；注入脚本收窄回 macOS 单平台；纯模型删
  `WindowsTitlebarPlan` / `windowsTitlebarPlanFor`；新增「防复辟」闸门
  `windows_native_decorations_tests` 三例（正例 + 反例 + macOS 不误伤）。代码笔 `724fa01`。
- **影响 / 需要他人做什么**：
  1. Windows 用户升到**下一 patch 版**即恢复移动 / 缩放 / 贴靠 / 三控件 ——
     注意 **v1.3.2 用户手上仍是缺陷版**，需发 patch 覆盖（发版流程未启动，待维护者定）。
  2. **macOS 侧遗留一条独立待办**（本次刻意未夹带）：ADR-0029 的 app-region →
     `data-tauri-drag-region` 翻译最终要 invoke `plugin:window|start_dragging`，该命令同样
     未授权 ⇒ 按 ACL 判据，该翻译**自 v1.3.0 起在真机上从未生效**（现在能拖的只是系统原生
     标题栏那一条带，dsh 自绘的 topStrip / titleRow 拖拽区是死的）。修法一行（补
     `core:window:allow-start-dragging`），**须 macOS 实机验证拖拽后再合**。
  3. 平台审计 §3.1/§3.2 与平台对齐计划 B-4/C-4 状态已回写（Windows 沉浸式档**关闭、不排期**；
     重开条件 = 恢复 Windows 真机验证 + 解决拖拽带伪元素与缩放边框两个结构问题）。
  4. README 经核从未宣传过该特性 ⇒ 无需改写；v1.3.2 的发行日志是已发行正文的来源，不动。
- **凭据**：`cargo fmt --check` ✓ · `cargo clippy --all-targets -- -D warnings` ✓ ·
  `cargo test` 576 passed · 前端 `typecheck` ✓ · `oxlint` 0 warning · `vitest` 678 passed
  （-2 = 移除的 Windows 计划两例）· **闸门负例实测**：临时注入
  `builder = builder.decorations(false)` 与脚本 Windows 分支后，正例/反例**双双 FAILED**
  （报错精确到 `ui.rs:196` 与 needle 字符串），还原后三例恢复绿。

### 2026-09-22 发版修正 · 改用 v1.3.2（原 v1.4.0 已撤回）—— guan（AI 协作）

- **原委**：本轮内容我先按 semver minor 发成 **v1.4.0**（内容：Windows 沉浸式标题栏 +
  WSL 客体档四项能力补齐 + 无托盘桌面的窗口内「关于 / 更新」）。维护者裁定**用 patch 号
  v1.3.2**，故同内容改号重发。
- **处置**：
  1. `c902f61`：版本号四处（`Cargo.toml` / `tauri.conf.json` / `frontend/package.json` /
     `Cargo.lock`）1.4.0 → **1.3.2**；发布日志标题改为 `## [v1.3.2] - 2026-09-22`
     （顺带订正上一版把标题日期写成工作开始日 09-21 的记述瑕疵）；
  2. tag **`v1.3.2`**（annotated，指向 `c902f61`）→ CI 构建 **success**、Release **published**；
  3. **v1.4.0 的 Release 由维护者在 GitHub UI 删除**（本机无 token）；其 tag 随后由本机删除
     （本地 + 远端 `refs/tags/v1.4.0`）⇒ 仓库与 GitHub 两侧不再残留 v1.4.0。
- **发行核验（查内容）**：v1.3.2 正文 **21 行**、首行 `## [v1.3.2] - 2026-09-22`；
  **17 个资产、0 字节：无**；`latest.json` 的 `notes` 与本地提取**逐字一致**、`version: v1.3.2`、
  `platforms` 七键齐备；资产名带 `1.3.2`（如 `DSH.Dock_1.3.2_aarch64.dmg`）。
- **现状**：Releases 列表 = v1.3.2 / v1.3.1 / v1.3.0 / v1.2.7（**无 v1.4.0**）；
  `/releases/latest` = v1.3.2 ⇒ 更新弹窗首屏即本次日志。冻结期继续（三平台真机验收通过前
  master 只收修复）。
- **教训登记**：版本号属**维护者裁定项**，AI 不应仅凭 semver 自行从 minor/patch 里挑一个 ——
  本次流程返工（删除 Release + 清 tag）本可避免。以后发版前先确认版本号。

### 2026-09-21/22 发版 · v1.4.0（Windows 沉浸式窗口 + WSL 客体档能力补齐 + 窗口内「关于 / 更新」）—— guan（AI 协作）

- **变更**：`9fa46e5`（版本号四处 + 发布日志）→ 附注 tag **`v1.4.0`**。minor 的依据：自 v1.3.1 起
  落地三项**用户可见新能力**，非纯修复。三个主题：
  ① **Windows 沉浸式标题栏**（维护者裁定「参照官方 dsh 客户端方案」：`decorations(false)` ≡ 官方
  `titleBarStyle:'hidden'`，注入官方同款 `data-windows-titlebar` + 40px 高度标记，三控件壳自绘）；
  ② **WSL 客体档四项核心能力补齐**（安全模式 / MCP 能力探测 / 官方挂载行增删 / 实验能力目录，
  四处客体拒绝清零）；③ **无托盘宿主桌面的窗口内「关于 / 更新」**入口。
- **发版前检查拦下一处会毁掉本次发版的问题**（`68b4294`）：CI 的 Windows leg `Unit tests` 红
  （同 leg clippy 已过 ⇒ 非编译问题）—— 我新加的两处客体脚本**实跑**闸门漏了 `#[cfg(unix)]`，
  在 Git Bash 下 `$DSH_HOME` 为 `C:\...` 导致断言失真。修补后 CI 全绿（`run 35679052716`）。
  **这正是 AGENTS §1「本机绿灯 ≠ CI 绿」的又一次实例**；也说明 tag 前跑一次远端检查是必须的。
- **发行核验（逐项查内容，不只看状态）**：Release **published** · 正文 **21 行**、首行
  `## [v1.4.0] - 2026-09-21` · **17 个资产、0 字节资产：无** · `latest.json` 的 `notes` 与本地提取
  **逐字一致**（更新弹窗首屏即此）· `platforms` 七键齐备 · tag 构建 **success**（run `35679641227`，
  四 leg 全过、含修复后的 Windows leg）。
- **冻结期开始**（CONTRIBUTING §8）：三平台产物验收通过前 master 只收修复。
- **需维护者真机验收（按可操作性排序）**：
  1. **Windows**（本次最大变更面）：沉浸式标题栏观感是否与官方一致；`decorations(false)` 后
     **缩放 / 贴靠 / 投影**是否退化（退化则回退 `ui.rs` 那个 cfg 块，成本一行）；
     标题栏带内的自绘控件位置/尺寸是否顺手。
  2. **Windows + WSL2**：安全模式、MCP 探测、挂载行增删、实验能力目录四项在客体档是否如期可用
     （此前只记「已实现（待 Windows 真机验收）」，本轮未变）。
  3. **Linux**（含无托盘宿主桌面）：安装包能否正常运行；i3/sway 类桌面上控制中心是否出现
     「关于 / 更新」入口且可用。
- **已知小瑕疵（如实登记）**：日志标题日期写作 `2026-09-21`（该项工作开始的日期），而实际发布时刻
  是本地 `2026-09-22` 上午 —— 属记述口径，不影响提取契约（CI 正则只认 `## [vX.Y.Z] - `）。
  已发布正文按前例**不回溯修改**（要改只能走 GitHub UI 编辑正文，或随下版覆盖）。

### 2026-09-21 平台待裁定项 · 维护者裁定与两项实现（§3.6 窗口内入口 / Windows 沉浸式）—— guan（AI 协作）

- **裁定（维护者 2026-09-21，落档备查）**：
  | 项 | 裁定 | 处置 |
  |:--|:--|:--|
  | §3.1/§3.2 Windows 沉浸式 | **参照官方 dsh 桌面客户端方案做** | ✅ 已实现 `73c6af1`（见下） |
  | §3.3 ARM64 覆盖 | **维持现状（x64），只在 README 说明** | 无需改动（README 已显式说明非静默） |
  | §2.8 macOS Intel 执行验证 | **维持现状（只编译）** | 无需改动（注释已写明理由） |
  | §3.6 Linux 无托盘宿主入口 | **加窗口内「关于 / 更新」入口** | ✅ 已实现 `995f61e`（见下） |
  | §3.4 关窗即退 | （无需裁定）| 已随 v1.3.1 修复（`18c0cf3`：关窗=隐藏 + 四路唤回） |
- **§3.6 实现** `995f61e`：`ShellState.resident_entry_available`（据托盘成败置位、macOS 恒 true、
  默认取"不可达"这一安全方向）+ 两个 IPC（`get_shell_capabilities` / `open_about`，四处登记同步 +
  登记册）+ 前端 `shouldShowAboutEntry` 纯判据，**只在无可达常驻入口时**于控制中心渲染入口。
  历史留痕：原 `open_about` 于 2026-08-27 因「与常驻入口重复」被删 —— 那个前提在无 StatusNotifier
  宿主的 Linux 桌面上不成立；新入口**只在该场景**出现，故不重现当年的重复按钮。
- **Windows 沉浸式实现** `73c6af1`：先读官方源码定位做法（`apps/desktop/src/main.ts:111-133`
  `titleBarStyle:'hidden'` + `titleBarOverlay{height:40}`；`preload-windows.ts:12` 打
  `data-windows-titlebar` + `--dsh-windows-titlebar-height:40px`；页面 `AppFrame.module.css:20-40`
  预留 40px 带并自绘拖拽条），再等价映射 —— Tauri **无** `titleBarOverlay` 等价 API
  （`TitleBarStyle` 文档原文 "on macOS"），故：`decorations(false)` ≡ 官方 hidden 标题栏；
  **同一套标记**保证页面行为与官方逐字一致；三控件由壳**自绘**（唯一偏差）。
  自绘件**零裸色值**（`color:inherit` + `color-mix` 派生）—— 官方那套是系统原生画，
  自绘就必须守本仓 token 纪律（`paletteTokens` 闸门当场抓过我一次，已改）。
- **闸门**：Windows 沉浸式 3 例（标记与官方常量一致 + 仅 Windows 生效的反例 / `decorations(false)`
  只在 Windows / 自绘件零裸 hex）、前端 2 例、§3.6 前端 4 例（含"回退常量必须偏向补入口"的反例）；
  IPC 闸门 7 例（COMMANDS ↔ handler ↔ capabilities ↔ tauri.ts）。
- **诚实边界**：两项均**只在真机才可验收** ⇒ 记「已实现（待真机验收）」：Windows 观感 +
  `decorations(false)` 后缩放/贴靠/投影是否退化（退化则回退该 cfg 块，成本一行）；无托盘宿主的
  Linux 桌面上入口是否如期出现并可用。
- **凭据**：rust **576 passed / 0 failed**；前端 **680 passed**；python 32 passed；fmt ✓；
  clippy `-D warnings` ✓（pipefail 判退出码）。

### 2026-09-21 WSL 客体档能力对齐 · **完成**（P0 / P0-c / P1 / P2 + 收尾）—— guan（AI 协作）

- **结论**：四处 `World::Wsl => Err(...)`（挂载行写入 / 删除 / 实验能力目录 / MCP 探测）**全部清零**；
  安全模式（读 / 关横幅 / 进入，P0-c）一并下沉。唯一保留的拒绝是 **streamable-http 型 MCP 在客体档**
  —— 其 url 指**客体内部**地址，从宿主发请求会打到别处或打不通（错结论比报错更坏）；这是**语义边界**，
  不是平台收窄（stdio 分支已下沉；http 需要客体侧 HTTP 客户端，而壳的唯一网络面纪律不允许为此新增）。
- **贯穿全程的一条纪律**：**不造第二个内核**。原方案（客体内复刻 JS 补丁内核）与"客体版 MCP 探测"
  都被否掉，改用：宿主同一内核（`PatchFile::from_text` / `apply_catalog_insert_row` /
  `manifest_declares_bundle` / `disabled_row_ids_from_text` / `displace…`）+ 客体侧只提供**端点原语**
  （读 / 写 / 备份 / 原子替换 / 通用 stdio 搬运）。客体脚本一律**零业务知识**。
- **本轮自查抓出的三个真问题（都在落地前修掉）**：
  ① 客体内核方案会造第二个内核（违 §6）⇒ 改为 `PatchFile::from_text` 同一内核；
  ② MCP 批次投递**违反握手顺序**（initialize ⇒ 等响应 ⇒ 才能 list）⇒ 搬运器改为**脚本化对话**
     （宿主给 steps，脚本只认 write / awaitId），并加**反例闸门**：批次模式在守顺序服务器上只拿到 id 1；
  ③ "每次只回一条哨兵"的备份脚本让安全模式一键还原在客体档**断链** ⇒ 备份改为**回传相对路径**。
- **验证**（全部本机可跑，**不依赖 WSL**）：rust **573 passed / 0 failed**；python **32 passed**；
  fmt ✓；clippy `-D warnings` ✓（一律 `set -o pipefail` 判退出码 —— 该纪律本轮拦下 3 次
  "管道吃掉退出码导致 dead_code 漏过门禁"）；闸门含**本机 bash 真跑**脚本、**本机 node 真跑**子进程。
- **诚实边界（不许打折）**：真机验收需要 **Windows + WSL2**（`docs/executor.md` 现为搁置）⇒
  以上一律记「**已实现（待 Windows 真机验收）**」，**不写**"客体档已适配"。
- **凭据**：commits `3bb7fc3` `e9cbde0` `e7c7449` `c849e2e` `6c53c83` `e972c21` `15d7574` `b3d9bc8`
  `121cc6b` `89ae6b2`；方案档 `docs/plans/wsl-parity-plan-2026-09-21.md` 各阶段状态已同步。

### 2026-09-21 WSL 客体档能力对齐 · 阶段记录（P0 / P0-c / P1 完成，P2 待做）—— guan（AI 协作）

- **背景（维护者裁定）**：安全模式、MCP 能力探测、官方插件挂载行增删、实验能力目录是**核心能力**，
  不得缺失对 WSL 的支持 —— WSL 客体模式是 `AGENTS.md` §0 红线 3 明写的适配目标，原报错文案
  「需补客体侧 X 原语后方可启用」是**欠债自陈**，不是限制。方案见 `docs/plans/wsl-parity-plan-2026-09-21.md`。
- **关键判断：没有新写内核**。原方案要在客体内复刻一份 JS 内核 —— 那会造第二个内核、违 §6
  「宿主/客体**同一内核**」且两侧必然漂移。核查发现三条不变量与写前自证**全部由既有原语提供**：
  保真 = `PatchFile::render()`（文本级拼接，未改条目原样保留）、自证 = `render_checked()`（写前回读解析）、
  备份 = `guest::backup_file*`、原子 = `guest::write_home_files`（tmp + mv -f）。
- **已完成**：
  | 阶段 | commit | 内容 |
  |:--|:--|:--|
  | P0 | `3bb7fc3`→`e9cbde0` | 挂载行**写入/删除**两侧接线客体孪生；客体侧前置门 `guest::missing_commands`（宿主 PATH 判不了客体的事）；`manifest_declares_bundle` / `manifest_description` 判定式抽核共用 |
  | P1 | `e7c7449` | **实验能力目录**下沉：插件事实按世界取（客体**一次批量** read_files），行态仍走 world-aware 的 dump-config |
  | P0-c | `c849e2e`→`e972c21` | **安全模式**全量下沉：客体备份可寻址（回传相对路径，原脚本只回 BACKUP_OK ⇒ 一键还原断在这里）、记账世界身份 `wsl:<distro>`（不改 schema，向后兼容）、state/dismiss/enter 三个孪生、`guest::dsh_home_abs`（`split_by_layer` 必须用**客体** patch 路径比对层序，否则可停行全判成"够不到"） |
- **闸门**（**均在本机可跑，不依赖 WSL**）：内核 4 例（原文保真/幂等零写入/所有权边界/删净自证）、
  命令探测 4 例（含**本机 bash 真跑**脚本）、备份脚本**真跑** 1 例（回传名字必须是真实同目录兄弟）、
  客体 home 解析 1 例、记账世界身份 3 例、接线不许回退 5 例（写入/删除/能力目录/安全模式两处）。
  **568 passed / 0 failed**；fmt ✓；clippy `-D warnings` ✓（一律 `set -o pipefail` 判退出码 —— 本轮
  因此当场拦下两次"管道吃掉退出码导致 dead_code 漏过门禁"）。
- **剩余**：**P2 MCP 能力探测**（`commands/console.rs:277` 是最后一处客体档拒绝）——协议对话整体放进
  客体（跨 `wsl.exe` 拉长连接 stdio 语义不等价，`executor.rs:607` 实测 90s 不 flush），只回结果 JSON。
- **诚实边界**：真机验收需要 **Windows + WSL2**（`docs/executor.md` 现为搁置）⇒ 以上一律记
  「已实现（待 Windows 真机验收）」，**不写**"客体档已适配"。
- **凭据**：各 commit 内附闸门清单；负例实测见各提交说明；方案档状态已同步。

### 2026-09-21 快车道修复 · 发布日志「已知限制」违宪更正 + WSL 安全模式读错世界（审计 A9）—— guan（AI 协作）

- **维护者指出（成立）**：v1.3.1 发布日志的「已知限制」把**技术允许但尚未实现**的项写成了限制 ——
  沉浸式窗口 Windows/Linux 档、WSL 客体模式四项核心能力、Windows ARM64 —— 等于**擅自把平台收窄并公告**，
  违 `AGENTS.md` §0 红线 3（「不得擅自做主不适配哪一个平台」；收窄须**维护者裁定 + ADR**）。
  **尤其 WSL**：`Windows（x64，含 WSL2 客体模式）`是红线明写的适配目标，把这四项核心能力
  （安全模式 / MCP 能力探测 / 官方插件挂载行增删 / 实验能力目录）写成"限制"，是把**欠债自陈**
  （原文「需补客体侧原语后方可启用」）冒充成了不支持的正当理由。
- **修（两部分）**：
  1. **日志措辞**（`docs/RELEASE_NOTES.md` v1.3.1）：标题改「环境要求与仍在补齐的适配项」，
     只保留**外部环境依赖**（Linux 托盘库）与 i18n 缺口；**移除 WSL 四项的列举** ——
     平台能力不写进"限制"，遇到时由**运行期显式报错**说明并给替代路径（红线 3 规定的通道）。
  2. **顺手修掉一处与审计相反的谎**：我原写"这些入口会明确报错，不会悄悄按本地处理"，
     而审计 A9 早已查出**安全模式的读侧会静默回落宿主 home** —— WSL 执行器 `dsh_home()` 返回
     `None`（`executor.rs:108-110`），旧写法 `.unwrap_or_else(user_dsh_home)` 于是拿**宿主世界**的
     home 去查记账，界面显示出另一个世界的状态（"读错世界"比报错更坏）。现抽 `safe_mode_home()`
     **纯函数**：客体档**显式拒绝**（与写侧同口径、同一句文案，单源），读侧两命令一并接入。
- **闸门**：`safe_mode_home` 2 例（本地档用本次启动的 home、缺记录退用户 home；**客体档必须拒绝
  而不是回落宿主**）。负例实测：去掉客体档分支 → 「客体档不得返回宿主 home: /home/u/.dsh」精确红灯。
  551 passed（+2）/ 0 failed；fmt ✓；clippy ✓；python 26 ✓；日志严格提取 26 行 ✓。
- **影响与后续**：① v1.3.1 的**已发布正文**仍带旧措辞（GitHub 上只能人工编辑，或随下版覆盖 ——
  维护者裁定）；② **A3（WSL 五项下沉）从"道 C 待排期"升为最高优先级**：它是红线 3 的目标面，
  不再是"可选功能"，施工方案见 `docs/plans/wsl-parity-plan-2026-09-21.md`。

### 2026-09-21 发版 · v1.3.1（Linux 启动崩溃 + 窗口唤起修复）—— guan（AI 协作）

- **变更**：`2d5964e`（版本号四处 + 发布日志）→ 附注 tag **`v1.3.1`**。本轮修复集：
  ① **Linux 缺 `libayatana-appindicator3` 时启动即崩**（改为建托盘前探测 → 跳过托盘 + 可行动日志；`.deb`/`.rpm` 声明 Recommends）；
  ② **关窗改为隐藏**（工作台继续跑）+ 四路唤回（控制台「返回工作台」/ 托盘左键与右键菜单 / macOS 菜单栏「显示工作台」/ macOS Dock `RunEvent::Reopen`）+ 唤起序 unminimize→show→set_focus + 延迟补焦 + 命令改返 `Result` 不再静默；
  ③ 运行日志取错来源与 UTF-16LE 读不出；④ 凭据 0600 说明限定 Unix；⑤ 快捷键措辞与 Windows SmartScreen 披露。
- **发布日志写法（沿用维护者口径）**：`v1.3.0 → v1.3.1` 的**用户可感对比**——核心亮点 2 · 缺陷修复 5 · 交互文案 2 · 已知限制 6，**不含**审计编号 / commit / 闸门名 / 装配脚本等工程内部。严格提取 **26 行**。
- **顺序纪律（v1.3.0 教训的落地）**：**日志先落盘并提交，再打 tag** —— 正文取自 tag 指向的树（`2d5964e`），不再出现"正文取自重写前那棵树"。
- **发行核验（逐项查内容，不只看状态）**：Release **published**（09:27:02Z）· 正文 **26 行** 且首行为 `## [v1.3.1] - 2026-09-21` · **17 个资产、0 字节资产：无** · `latest.json` 的 `notes` 与本地提取**逐字一致**（更新弹窗首屏就是这份）· 无旧版迭代史关键词。CI：tag 两个 run（`35582403030`/`35582403286`）均 **success**。
- **冻结期开始**（CONTRIBUTING §8）：三平台产物验收通过前 master 只收修复。
- **需维护者真机验收（三平台，按可操作性排序）**：
  1. **macOS（本机即可，不依赖 Windows）**：关掉工作台窗口 → 控制台「返回工作台」应唤回；菜单栏「显示工作台」；点 Dock 图标；托盘**不适用**（macOS 走菜单栏）。
  2. **Linux（首次真机运行，价值最高）**：装 `.deb` / 跑 `.AppImage` 看是否正常启动（v1.3.0 在缺库机器上启动即崩）；缺库时应"能启动、无托盘、日志给命令"。
  3. **Windows**：托盘右键应有「显示工作台」；关窗后从托盘唤回；SmartScreen 提示是否符合预期。
- **诚实边界**：①②两条的 GUI 行为本机无法点击验证，闸门钉的是**结构与顺序**（6 例 + 3 个负例）；Linux 运行期行为仍是首次真机检验。

### 2026-09-21 快车道修复 · 关掉工作台窗口后「返回工作台」没反应（维护者真机报告）—— guan（AI 协作）

- **变更**：`18c0cf3` —— 两半根因一并修，缺一不可：
  ① **窗口被销毁**：`CloseRequested` 默认销毁窗口 ⇒ `get_webview_window("main")` 返回 `None`
  ⇒ 三处唤回入口（控制台 IPC / 托盘左键 / 二次启动）全部落进"窗口不存在"分支，且旧写法
  `if let Some` 把它**静默吞掉**（连日志都没有）——用户侧就是"点了没反应"。
  ② **即便窗口还在，裸三连也会落空**：tao 0.35.3 的 macOS `set_focus()` 在
  `is_minimized || !is_visible` 时**整个调用直接返回**（静默 no-op），而 `unminimize()` 是
  **异步**动画 ⇒ 旧顺序常把 focus 跑在 deminiaturize 落地之前，既没提到前台也没走到
  `NSApp.activateIgnoringOtherApps`。
- **修法**：**关窗 = 隐藏**（三平台一致，`prevent_close()` + `hide()`，窗口与工作台进程都保留，
  对齐 README「窗口可关，任务台仍在」；退出仍走托盘/菜单栏「退出」）＋ `bring_to_front`
  唤起序（unminimize → show → set_focus ＋ 320ms 补焦，窗口缺失时响亮记日志）＋ **新增显式
  唤回入口**（macOS 菜单栏「显示工作台」、托盘右键「显示工作台」、macOS 点 Dock 图标经
  `RunEvent::Reopen`——macOS 无托盘，缺这些用户就困在"应用在跑、界面看不见"）＋
  `focus_main_window` 改返回 `Result`（前端 `ProfileManager` 早有 `.catch(showToast)`，
  返回 `()` 让那条路径**永远亮不起来**，这正是"没反应"的可见性缺口）。
- **署名与协作（需并行工作流知悉）**：本修复的**基座**（`bring_to_front` 及其 2 例测试、单实例与
  托盘左键改走它）来自**你方留在工作区的未提交改动**。维护者指示"你来继续把那个 bug 修了"
  ⇒ 我接手补全（关窗=隐藏、菜单入口、Reopen、Result 返回、4 例闸门）并**连同基座一并提交**，
  否则我的提交会引用未提交的 `bring_to_front` 而**编译不过**。若你方对基座有后续改动，
  请直接在其上迭代（`18c0cf3` 之后工作区已干净）。
- **闸门 6 例**（4 新 + 基座 2）：关窗必须 hide 且**不得只在 macOS** / 「显示工作台」在 macOS 菜单
  与托盘两处都存在且接同一分派 / Dock Reopen 必须唤回 / 命令必须返回 Result 并前置判存在 /
  raise 顺序 unminimize→show→set_focus / 三入口共用 helper。**负例实测**：去掉 `prevent_close`、
  去掉 `show_main` 分派、命令不再判存在 ⇒ 各自精确红灯。
- **影响**：① 关窗不再等于退出（行为变更，README 已写明唤回与退出方式）；② 控制台「返回工作台」、
  托盘左键、二次启动、macOS Dock 四条路径行为一致。**无需他人动作，仅周知**。
- **诚实边界**：GUI 行为本机点不了，闸门钉的是**结构与顺序**；关窗=隐藏与 Dock 唤回仍需维护者
  在 macOS 真机点一次（本机即可，不依赖 Windows）。
- **凭据**：`cargo test` **549 passed / 0 failed / 7 ignored**；`cargo fmt --check` ✓；宿主
  `clippy --all-targets -D warnings` ✓；`python3 -m unittest discover -s scripts/tests` 26 ✓。

### 2026-09-21 平台对齐修复第二刀 · 快车道一轮（道 A 五项 + 开发计划）—— guan（AI 协作）

- **计划**：`docs/plans/platform-parity-plan-2026-09-21.md` —— 审计剩余项按**能否在本机验证**分三道（道 A 本机可验 5 项 / 道 B 依赖 Windows 真机 5 项 / 道 C 待裁定 6 项），
  分道的理由写死在档里：**不许把没验的东西说成验过了**。
- **道 A 五项全部落地（快车道直推 master，各自独立提交 + 闸门/可复跑验证 + 负例）**：

  | 项 | commit | 修了什么 | 证据 |
  |:--|:--|:--|:--|
  | **A-1（审计 A6）** | `462d6fb` | `render-product.sh` 把 Windows 的 `node.exe` 落成无扩展名 `dsh-node` ⇒ CreateProcess 自动追加 `.exe` 找不到 ⇒ Windows 快照档启动必败；该脚本 CI 零引用，无人会发现。改为按**传入文件扩展名**定落位名，manifest 引同一变量单源 | 新闸门 3 例**真跑脚本**（临时 `--out`，绝不碰仓库）：Windows 保留 `.exe` 且不留残影 / 非 Windows 不加 `.exe` / **`--out` 生效**（若被忽略仓库 manifest 会被写脏即红）；负例：退回忽略 `.exe` → 精确红灯 |
  | **A-2（§3.9）** | `29026e2` | 闸门只查"leg 声明的份 ⊆ 白名单"，不查"取全" ⇒ Windows leg 只取一份仍全绿，用户切 WSL 才炸 | 新用例从 build.yml 解析 fetch 参数、从 `guest_pnpm_bundle()` 解析客体份名（单源）断言两份都取；负例：只取 `win32-x64` → 精确红灯 |
  | **A-3（§2.10）** | `43027bc`+`0e4c7ff` | `require_local` / `unsupported_in_wsl` 零调用点死码（唯一引用是前端单测里逐字复制的文案 = 测试守着一句用户看不到的话）；文档死链一并改事实 | 删码 + fixture 换活文案；**宿主 clippy 立刻抓到单元素循环退化**（`-D warnings`），已修 —— 又一例"fmt 过 ≠ clippy 过" |
  | **A-4（§3.5）** | `fcdd4eb` | 文案写"全局快捷键"，实为**页内 keydown**（失焦即无效，ADR-0024 已裁暂缓） | zh/en 改为"应用内生效，窗口需在前台"；`ui.rs` 注释写明非 OS 级；RELEASE_NOTES 的 v1.3.0 措辞**不动**（已发行正文来源，随下版覆盖） |
  | **A-5（§3.7）** | `fcdd4eb` | macOS 有 Gatekeeper 提示、Windows 无对应披露 | README 增 SmartScreen 提示 + 说明自动更新用的是本项目签名校验、不受影响 |

- **顺带修一处**：`render-product.sh` 的 `--out` 此前"用法第 21 行写了、参数解析里没实现"（传了就报未知参数）。
- **并发写者的一次真实教训（已自查修复，未推前）**：A-4 那次提交我用了 `git add src-tauri/src/ui.rs`，
  而该文件**同时含并行写者未提交的托盘 hunk**（`bring_to_front` 调用）—— 其定义在同样未提交的
  `commands/window.rs` 里 ⇒ **那条提交树会编译失败**。**未推送前**已重写该提交（只留我方注释改动，
  他们的 hunk 原样退回工作区），并做**隔离验证**：把他们的三个文件 stash 后工作区 == HEAD 树，
  `clippy --all-targets -D warnings` ✓、`cargo test` 543 passed ✓，再 pop 还原。
  教训：**动到共享文件时必须按 hunk 而不是按文件 add**（`git add -p` 或先重建目标态）；此前
  "按文件精确 add"的做法只在"文件不重叠"时成立。
- **诚实边界（道 B/C 未动的理由，不是没时间）**：道 B（A2 Windows 孤儿清扫第二层 / A4 尾 DACL / A6 尾实机 / §3.1 Windows 毛玻璃 / §3.4 关窗语义）**依赖 Windows 真机验证**——现在改完无法证明，只会把"已知失效"变成"未知是否失效"；道 C 里 C-1（A3 WSL 5 项下沉）是独立一轮的活、C-2（macOS Intel 执行验证）三条路各有代价需先选路、C-3/C-6 需裁定。
- **影响**：① Linux/Windows 用户不再踩 A6/§3.9 这两条"只在用户机器上炸"的坑；② 三项过时/虚假文案（快捷键、Windows 披露、死码 fixture）已校正；③ 剩余项的道与前置已写死，接续不需重新调研。**无需他人动作，仅周知**。
- **凭据**：`cargo test` **545 passed / 0 failed / 7 ignored**；`cargo fmt --check` ✓；宿主 `clippy --all-targets -D warnings` ✓；`python3 -m unittest discover -s scripts/tests` **26 passed**（+4）；前端 typecheck ✓ / oxlint 0-0 ✓ / vitest **674 passed** ✓；负例 7 连（A1×2 · A7×2 · A5×1 · A-1×1 · A-2×1）各自精确红灯。

### 2026-09-21 平台对齐修复第一刀 · A1 / A7 / A4 / A5（四项，含闸门与负例）—— guan（AI 协作）

- **变更（四处独立提交，各自带闸门与负例实测）**：
  | 项 | commit | 修了什么 | 闸门 |
  |:--|:--|:--|:--|
  | **A1** | `5137af1` | Linux 缺 `libayatana-appindicator3` ⇒ crate 内 `panic!` + release `panic="abort"` ⇒ **启动即崩**。改为建托盘**前**按同一组四个候选名 dlopen 探测，失败即跳过托盘并给可行动错误（应用照常启动）；`ui.rs`；`.deb`/`.rpm` 声明 **Recommends**；README 披露 | 3 例（名单覆盖 / **探测先于建托盘** / 包声明）；负例：挪到建托盘后、删 `bundle.linux` 各自红灯 |
  | **A7** | `0852000` | 「运行期取哪份 pnpm」与「CI 取哪份」两个真相源此前**无交叉校验**（互换 `darwin-arm64`/`darwin-x64` 仍全绿）。新闸门调用运行期同一函数求名并断言该文件真在 `resources/pnpm/`；Windows 另要求客体投递份 | 1 例；CI 打包态强制（`DSH_TEST_REQUIRE_PNPM_BUNDLE=1`，同 `DSH_TEST_REQUIRE_NODE` 套路），并把「多背两份」判红；负例：本目标那份不见 / 多带别的份各自红灯 |
  | **A4** | `a531d55` | 凭据 `0600` 承诺在 Windows 上是**假声明**（代码整段 `#[cfg(unix)]`、无替代）。UI 文案 zh/en 12 处 + README ×2 + AGENTS §6 一律限定 Unix，并写明 Windows 依赖 profile 默认 ACL | 前端三闸门（typecheck / oxlint / vitest 674） |
  | **A5** | `9c99f81` | 日志台：`get_app_logs` 是 console.rs 里**唯一**没有世界分发的命令（WSL 档静默读宿主日志）；`dsh` 源指向全仓无人写入的 `dsh.log`；解码用严格 UTF-8 不认 UTF-16LE。新增 `LogWorld` + 纯函数真值表，客体档 profile 日志改读客体 home | 4 例（真值表 / 客体读 / **反例**不得回退 / 命令层必须世界分发）；负例：改成恒本地世界 → 精确红灯 |
- **为什么先修这四条**：审计把 A 级分成"功能缺失"与"静默降级 / 假实现"两类，后一类**不报错**、用户无从察觉，而 A1 更是唯一"应用起不来"的那条。四条都是小改动，且都能本地验证。
- **诚实边界**：① **A1 的 Linux 运行期行为仍需 Linux 真机**（本机无 Linux，交叉 clippy 也跑不了——仓库既有约束，由 CI ubuntu leg 编译校验）；② A4 只做了**披露止损**，Windows 显式 DACL 待做且需 Windows 真机；③ A5 客体档日志内容需 Windows+WSL 真机复核。
- **重要协作观察（需维护者知悉）**：本轮工作期间本树**另有并行写者**在改 `src-tauri/src/lib.rs`、`commands/window.rs`、`ui.rs`（`bring_to_front` 唤起序修复），其改动**未被我的提交卷入**（我按文件精确 `git add`，四次提交只含我方文件；其改动仍留在工作区）。共享工作区并发写同一 crate 有相互覆盖风险，建议约定"同一时间一人改 Rust 面"。
- **影响**：① 修 A1 后 Linux 用户不再可能"启动即崩"，缺库时降级为无托盘 + 可行动日志；② A7 让"平台取份错配"在 CI 上编译期级别的早红；③ A4/A5 消除两处"对用户说了不成立的话 / 给错结果"。**无需他人动作，仅周知**。
- **凭据**：`cargo test` **546 passed / 0 failed / 7 ignored**（v1.3.0 基线 536，本轮 +10）；`cargo fmt --check` ✓；宿主 `clippy --all-targets -D warnings` ✓；前端 typecheck ✓ / oxlint 0-0 ✓ / vitest 674 ✓；`python3 -m unittest discover -s scripts/tests` 22 ✓；负例五连（A1×2、A7×2、A5×1）各自精确红灯。审计档 §2.1/2.4/2.5/2.7 已回写「已修」状态。

### 2026-09-21 平台对齐审计 · 深挖补充（三路并行复核）+ 宪法级失真校正 —— guan（AI 协作）

- **原委**：上条广播（`663a77a`）是首轮扫描结论。随后三路并行深挖（Rust 分叉归类 / 构建发布链 /
  系统集成面）返回，**发现首轮漏判与误判**，逐条回源核实后已重写审计档。
- **新增 A 级（都比"功能缺失"更危险——它们不报错）**：
  ① **Linux 托盘缺 `libayatana-appindicator3` 即 panic**（`libappindicator-sys-0.9.0/src/lib.rs:41,50` dlopen 失败
  即 `panic!`；`Cargo.toml:65` release `panic="abort"` ⇒ **启动即崩**；`lib.rs:253` 的 `if let Err` 拦不住 panic）——
  而 `tauri.conf.json` **无 `bundle.linux`** 块 ⇒ deb/rpm 不声明依赖、AppImage 不含（dlopen 无 DT_NEEDED）；
  CI 装了库（`build.yml:157,537`）所以永不触发。**唯一"用户什么都看不到、应用起不来"的缺口**。
  ② **Windows 登记表孤儿清扫第二层结构性失效**：`lifecycle.rs:551-558` 非 unix `attach` 空体 ⇒ 锁在
  Windows 上恒无持有者 ⇒ `:721 Ok(()) => Stale` 恒成立 ⇒ 只删登记不杀进程；三条"活孤儿"测试全
  `#[cfg(unix)]`（`:1249/1282/1320/1343`）；Job 三层失败仅 `warn`（`:301/313/340`）⇒ 此时两层全失，
  而 `:554-557` 注释宣称第二层有效（**注释不实**）。【静态推断】
  ③ **`get_app_logs` 无 World 分发**（`commands/console.rs:53`，console 里唯一一条）⇒ WSL 档读**宿主**日志，
  与 ADR-0016"控制台 100% 支持客体模式"冲突；叠加 `diagnostics.rs` 的 `dsh` 源指向**无人写入**的
  `app_data/dsh.log`、且用严格 UTF-8（未走现成的 `resolve::read_log_auto`）⇒ 日志台整块失效。
  ④ **`render-product.sh:56`** 把 `node.exe` 落成无扩展名 `dsh-node`（manifest `:78` 同名字面量）⇒ Windows
  快照档 spawn 必败；该脚本 **CI 零引用**。【静态推断】
  ⑤ **取份交叉缺口**：`updates.rs:1055-1074`（运行期求名）与 `build.yml:60/67/183`（CI 取份）无任何交叉闸门——
  把 `darwin-arm64`/`darwin-x64` 两字面量互换，现有闸门**仍全绿**，Intel 包引导必炸。
  ⑥ **macOS Intel leg 一个断言都不执行**（`native_arch:false` ⇒ 只 `cargo test --no-run`），且 `boot-smoke.yml`
  **无 macOS 作业**。⑦ WSL 档安全模式读宿主 home（`executor.rs:107-110/361-363` + `boot.rs:348`）。
- **首轮两处误判已改**：① 前端 **typecheck 三平台都跑**（经 `beforeBuildCommand` 的 `tsc -b`），只有
  oxlint/vitest 是 Linux-only；② **Linux 托盘不是"已就位"**（原判 B，现判 A，见上）。
- **宪法级失真校正（§10 流程，依据 ADR-0010 §223 + `build.yml:181-183`）**：
  `AGENTS.md` §0 红线 2 与 `docs/contract.md:217` 原称「Windows 包另带 **musl** 份供 WSL 客体」——
  而 ADR-0010 §223 明写「2026-09-04 裁定统一不考虑 Alpine 后，**原 musl 静态份取消**」，CI 实取
  `win32-x64 linux-x64`（后者为 **glibc**）⇒ 两处改为「**linux-x64（glibc）** 份 + 注明 musl 已取消、
  客体仅支持 glibc 发行版」。**这是失真校正，不改规则**；如需保留原表述请指示。
- **影响**：① 修复排序见审计档 §8（A1 Linux 托盘 → A4 凭据披露 → A5 日志台 → A3 WSL 5 项 → A7 闸门…）；
  ② **依赖 Windows 运行时的 A 级结论（A2/A6）全部只能静态推断**，因 Windows 真机验证现为搁置（§7）；
  ③ 上条广播中"A/B/C"结论**以本档为准**。
- **凭据**：`docs/team/平台对齐审计-2026-09-21.md`（重写版，10 节，每条标【已核实】/【静态推断】）；
  关键裁决点附 crate 行号（`libappindicator-sys-0.9.0/src/lib.rs:41,50`、`tauri-2.11.5/src/window/mod.rs:766,787,798-802`、
  `tauri-utils-2.9.3/src/lib.rs:122-152,161`、`tauri-2.11.5/src/window/plugin.rs:243`）。

### 2026-09-21 平台对齐审计 · 依据 AGENTS §0 红线 3 的首轮全仓核查 —— guan（AI 协作）

- **变更**：`d9f9523` — 新档 `docs/team/平台对齐审计-2026-09-21.md`（9 节：方法/边界 · 结论速览 ·
  A 真缺口 · C 待裁定 · B 正当差异清单 · 系统集成面 · 构建签名更新器 · 验证覆盖 · 建议）。
  扫描口径 = `src-tauri/src` 全部 **248 处**平台分叉 + 前端/injected 全部平台语义使用点；
  判定回到 **crate 源码**核对（tauri 2.11.5 / tauri-utils 2.9.3 本地 registry 实读），不引二手结论。
- **结论（回答维护者"哪些功能没有多平台对齐"）**：真正对不齐的**用户可感**功能只有两类 ——
  ① **WSL 客体档 5 项未下沉**（`list_experimental_capabilities` / `apply_`+`remove_official_patch_row` /
  `probe_mcp_server` / 安全模式）：错误文案自称「需补客体侧原语」，而 `guest.rs` 的
  `read_files` / `write_home_files` / `backup_file` / dsh CLI 转发**已在**，`mcp.rs:1334` 甚至在客体档
  写 patch ⇒ **实现债，不是技术不可能**；
  ② **沉浸式窗口仅 macOS**：`TitleBarStyle` 确为 macOS 专属（crate 原话 "on macOS"），但
  `WindowEffect::Mica/Acrylic/Tabbed` 是**同一个 `EffectsBuilder` API 上的 Windows 等价物**
  ⇒ 「无等价物的那半」没做是事实，「有等价物的那半」也没做。
- **另发现三项**：③ **凭据 `0600` 加固整段 `#[cfg(unix)]`，Windows 静默不生效**，而 README×2 +
  AGENTS §6 **无条件下承诺 0600** —— 红线 3 明禁的「静默降级」，也是本次唯一"安全承诺与实际不符"项；
  ④ `mgmt.rs:76/102` 的 P0 兜底（`unsupported_in_wsl` / `require_local`）**零调用点**，唯一引用是
  前端单测里逐字复制的文案（测试守着一句用户永不看到的话）；⑤ **Linux 包从未有人真机运行过**
  （Windows 验到 v1.2.0 后按指示搁置），CI 只覆盖逻辑层。
- **正面确认（同样重要）**：所有已知平台差异**都有可行动文案**（WSL 5 项给了替代路径、绝不回落宿主）；
  前端平台语义**只有 6 个文件触碰、能力门仅 `can.*` 两处**，无散写判断；托盘/菜单三平台齐备
  （`ui.rs:329/416` 的 `#[cfg(not(target_os = "macos"))]` + `Cargo.toml:32` 无平台条件）；
  `clippy`+`test` 在三个原生平台都跑；平台矩阵已有机器闸门。
- **影响**：① 后续平台相关改动以本档为现状底本；② **需维护者裁定**：§8 七条建议的取舍与排期
  （其中「凭据权限披露止损」代价最小、「WSL 5 项下沉」收益最大）；③ 功能级收窄（沉浸式 macOS 专属、
  Windows ARM64 走仿真、x64-only）目前**只有文档、没有 ADR** ⇒ 红线 3 要求裁定，建议补齐或明确放弃时收口。
- **凭据**：`docs/team/平台对齐审计-2026-09-21.md` 全篇 `file:line` 可抽查；关键裁决点附 crate 源码行号
  （`tauri-utils-2.9.3/src/lib.rs:122-152,161,156`、`tauri-2.11.5/src/window/plugin.rs:243`）。

### 2026-09-21 补记 · v1.3.0 已发行正文为旧版发布日志（维护者裁定不修，下版自然覆盖）—— guan（AI 协作）

- **原委**：发布日志已按维护者口径重写为「**1.2.7 → 1.3.0 用户可感对比**」（42 行，`455a243`，
  只写升级后看得见的差别、剔除开发期才存在的 bug 与工程内部叙述）。但重写**落在打 tag 之后**：
  原 tag 构建（run `35568612865`，tag 指向重写前的 `51b3233`）先跑到终点，**06:42 已将 v1.3.0
  发布为公开态**。故已发行正文 = 重写前的 67 行旧版，且 `latest.json` 的 `notes` 字段携带
  同一份旧文本 —— **更新弹窗显示的就是它**（内含"见上…一条"这类自指句）。
- **裁定**：维护者 2026-09-21 选择 **C：先不动，下个版本自然覆盖**。故：
  ① 不删 Release、不移 tag —— tag 现指向的 `51b3233` 恰是**实际构建并发布的那棵树**，语义自洽；
  ② `docs/RELEASE_NOTES.md` 在 master 上已是新版（`extract-release-notes.py v1.3.0 --strict`
  本地跑通 → 42 行），**下个版本的日志与更新弹窗正文从源头就是干净的**；
  ③ 诚实的用户可见后果：下个版本发布前，Releases 页与更新弹窗显示的是**过程叙述版旧日志**。
- **教训（后续发版照此办）**：**发布日志的改动必须落在打 tag 之前**——正文取自 tag 指向的那棵树，
  `docs/RELEASE_NOTES.md` 是**发布输入**而非发布后能补的文档；重写正文后若旧构建仍在飞，
  必须先取消再移 tag，否则竞态由 CI 完成时间决定。
- **影响**：仅周知。产物与签名不受影响（资产已正常发行，17 个资产 + `latest.json`）。
- **凭据**：`curl .../releases/tags/v1.3.0` → `draft: false` / `published_at 2026-09-21T06:42:18Z` /
  正文 67 行；线上 `latest.json` 的 `notes` 行数 67（新版 42）。

### 2026-09-21 宪法级改动 · AGENTS §0 新增红线 3「平台覆盖不得擅自收窄」—— guan（AI 协作）

- **变更**：`c582fb4` — `AGENTS.md` §0「红线两条」→「红线三条」，新增第 3 条平台覆盖铁律：
  **macOS（arm64 / x86_64）· Windows（x64，含 WSL2 客体模式）· Linux（x64），只要技术
  允许就必须都适配**——任何人不许自行判定「这个平台先不做 / 先不适配」。确有技术不可能 →
  该面**显式报错并说明原因**（禁静默降级、禁假实现；先例 = WSL 客体档缺客体侧原语的命令，
  ADR-0016）；**删 leg / 删上传 / 删发布下载属宪法级决策：维护者裁定 + ADR**。用户可见矩阵
  以 README「下载与安装」表为唯一权威（禁双源）。
- **机器投影（同批落地，避免铁律只停在散文层）**：`scripts/tests/test_release_platform_matrix.py`
  新增 `PlatformCoverageTests` 5 例 —— ① 三平台四 leg 一个不许少 ② 每个 leg 必须上传
  安装器 + updater ③ 发布 job 必须下载每个世界的产物 ④ **构建工件集合 == 发布侧下载集合**
  （漏发布 / 凭空下载双向都红）⑤ 构建 job 内不得出现 `continue-on-error`（非阻断 = 永远绿的
  红灯平台）。另修同文件 docstring「三个被比对的集合」→「四个」（原文列了 4 条）与一处
  未使用的 `import importlib.util`。
- **影响**：① 后续任何平台相关改动一律以 AGENTS §0 红线 3 为准，**收窄平台不能靠改文档
  悄悄完成**——闸门会红，且红了的正确处理方式不是把测试改绿；② 不需要他人动作，仅周知。
  存量失真提示：`docs/handoff-4.3-readonly.md` 与 `docs/plans/positioning-pivot-roadmap-2026-09-18.md`
  中「红线两条」的表述是各自日期下的快照，**本次未改**（以 AGENTS §0 为准）。
- **凭据**：`python3 -m unittest discover -s scripts/tests` → **22 passed / 0 failed**（原 17，+5）；
  反例三连实测（删 leg / 删发布下载 / 加非阻断）各自精确红灯；`AGENTS.md` 230 行（预算 ≤250）、
  §0 29 行（预算 ≤40）。依据 = **维护者 2026-09-21 会话直接裁定**（§10 的「改前频道知会」
  由此裁定本身满足）。

### 2026-09-21 发版 · v1.3.0（插件模块整轮重做 + 沉浸式标题栏）—— guan（AI 协作）

- **变更**：`release: v1.3.0` 提交 + 附注 tag `v1.3.0`。本版覆盖 **v1.2.7 之后的全部 49+ 提交**
  （v1.3.0 此前只在源码里备好、从未打过 tag，故内容以打 tag 时的树为准）。两个主题：
  ① **插件模块整轮重做**（插件列表 / 实验能力 / 插件中心三处收敛到同一套信息层级与命名规则：
  「实验能力」是实验性插件的唯一操作入口、名字取插件原本的名字 + 中文为辅、同类标记不复述、
  描述性文字收进 ⓘ）；② **ADR-0029 沉浸式标题栏**（macOS：隐藏标题栏 + Sidebar 材质 +
  红绿灯按官方几何落位 + 平台标记注入与 `app-region` 映射；Windows/Linux 维持原生装饰）。
- **影响**：发版 tag 已推 `origin`（GitHub）→ CI 触发三平台**签名**构建并生成 Release；
  自此刻进入 **冻结期**（Release notes 发出至三平台产物验收通过期间 master 只收 fix 不收 feat，
  见 `docs/contracts/README.md` §冻结期）。`cnb` 远端**未推**，需要时另行同步。
- **凭据**：`cargo fmt --check` 净 · 宿主 `clippy --all-targets -D warnings` 净 ·
  `cargo test` **536 passed** · `pnpm install --frozen-lockfile` ✓ · `typecheck` / `lint` 0 错 0 警 ·
  `pnpm test` **674 passed（66 files）** · `vite build` ✓ · 发布脚本 `python3 -m unittest` **17 passed** ·
  tag 双闸门自测通过（版本号三处一致 = 1.3.0；`docs/RELEASE_NOTES.md` 含 `## [v1.3.0]`）。
- **留白（诚实标注）**：① 红绿灯落位与毛玻璃的**真机观感**尚待维护者复核（ADR-0029 追加二的
  广播已记 "待验证（维护者侧）"）；② 交叉目标 clippy（Windows / Linux）本机跑不了
  （`ring` 的 build script 需交叉 C 工具链），按 AGENTS §1 归 CI；③ `docs/acceptance-2026-09-16.md`
  里仍有 9 项 🖐 待人工验收，与本版并发、不阻塞本版打 tag。
### 2026-09-21 ADR-0029（追加四）· 红绿灯容器扩充：对标 Electron 同款 52px 容器垂直居中，彻底消除 19px 贴顶错位 —— guan（AI 协作）

- **真因定位**：AppKit 的 `NSTitlebarView` 默认高度仅为 32px 且坐标原点在底部（`isFlipped = false`）。此前直接对按钮设 `y = 18`，在 32px 容器内 `18 + 14 = 32px`，导致红绿灯直接**贴死在窗口最顶端天花板**（中心距顶仅 7px），与 DSH 侧边栏 `.topStrip`（52px 高度居中，中心距顶 26px）形成近 19px 的巨大高低差（截图里的红框）。
- **对齐官方 Electron 实现**（`shell/browser/ui/cocoa/window_buttons_proxy.mm`）：
  ① 将 `NSTitlebarContainerView` 与 `NSTitlebarView` 同步扩充到 52px（与 DSH `.topStrip` 高度一致）；
  ② 在 52px 容器内将按钮垂直居中放置于 `y = (52 - 14) / 2 = 19px`（窗口全局坐标中心距顶恰为 `52 / 2 = 26px`），与侧边栏收起按钮 `◫` 完美处于**同一水平线**；
  ③ 水平方向以 `TRAFFIC_LIGHT_X = 16.0` 起始，自适应保留原生按钮间距（~23px）；
  ④ 全量窗口事件幂等重放，已就位（误差 < 0.5pt）静默，彻底解决 resize 弹回问题。
- **凭据**：Cocoa 原生探针实测通过（全局坐标中线距顶精确 26.0px）· `cargo fmt --check` 净 · `cargo test` 536 过 · clippy `-D warnings` 净 · 非 macOS 模拟编译净。

### 2026-09-21 ADR-0029（追加三）· 红绿灯重放加固：全量窗口事件 + 变化检测 + setup 后补刀 —— guan（AI 协作）

- **触发**：维护者点明目标——红绿灯须与**侧边栏收起按钮水平对齐**（此前表述绕了）。
  目标值不变（(16,18) → 灯光中心 24 ≈ toggle 中心 25）；问题是落位未生效。
- **变更**（`traffic_lights.rs` + `ui.rs`）：① 事件重放从三类扩到**任何窗口事件**
  （Destroyed 除外）；② align 改**幂等 + 变化检测**——已就位不写不打日志，挂全量
  事件零刷屏；③ 新增 setup 结束后 `run_on_main_thread` 补一枪，覆盖"tao 建窗后
  内部 setFrame / vibrancy 插入导致 reset 但不发 Tauri 事件"的时机；④ 首次落位
  info 日志带读回帧坐标。
- **凭据**：fmt/test 536 过 / 宿主 clippy 净 / 非 macOS 三点 cfg 模拟编译净。
- **待验证（维护者侧）**：**重启 dev**（Rust 不走 HMR）→ 终端应出现
  `红绿灯已定位 #0=(16.0,18.0) #1=(39.0,18.0) #2=(62.0,18.0)`；无此行 = 跑的旧
  二进制。然后再截图与官方比对。

### 2026-09-21 ADR-0029（追加二）· 红绿灯重放：实测 resize 会把按钮弹回默认位，落位改事件驱动 —— guan（AI 协作）

- **触发**：维护者截图——红绿灯仍停在系统默认位（距顶 ≈0，toggle 行中心低约 18px），
  「你调整成这样？」。
- **实证**（探针续测）：`makeKeyAndOrderFront`（show）**不**重置按钮；`setFrame:`
  （resize）把三个按钮**全部弹回 (9,9)**——即建窗时的一次性落位会被任何 resize 冲掉。
- **变更**（`traffic_lights.rs` + `ui.rs`）：落位后挂 `on_window_event`
  （Resized / Focused(true) / ScaleFactorChanged）重放；首次落位改 info 且**读回帧
  坐标**写日志（`红绿灯已定位 #0=(16.0,18.0) …`），实机核对生效与否以该行为凭据；
  重放打 debug（resize 高频）。
- **待办**：Rust 改动不走 Vite 热更新——须**重启 dev / 重新构建**后再截图比对；
  若日志无 `红绿灯已定位` 行 = 二进制未含本模块（排查方向：跑的是旧进程）。
- **凭据**：fmt/test 536 过 / 宿主 clippy 净 / 非 macOS 三点 cfg 模拟编译净。

### 2026-09-21 ADR-0029（追加）· 红绿灯定位补立：AppKit FFI 逐按钮 setFrameOrigin 对标官方 x16/y18 —— guan（AI 协作）

- **触发**：维护者截图比对——壳侧红绿灯比官方高约 10px，与 dsh 顶栏条带垂直对不齐。
  官方显式 `trafficLightPosition {x:16,y:18}`（`main.ts:127`），Tauri Overlay 无此 API。
- **变更**（src-tauri 2 文件）：新增 `traffic_lights.rs`（objc2 裸 msg_send，仅 macOS；
  `Cargo.toml` target-gated `objc2 = "0.6"`——传递依赖提升，lock 在册无新 crate）；
  `ui.rs` 建窗后无条件调用（函数内分平台）。ADR-0029 补 §3.1 小节。
- **实证**（原生探针 /tmp/tltest，2026-09-21）：① 按钮 frame 在 theme frame（flipped）
  里，y 从窗口顶部下量，与 Electron 语义一致，(16,18) 无需换算；② `setFrameOrigin:`
  绝对定位，三按钮同点会叠成一坨（首版踩中）——按各按钮默认 x 相对 close 的偏移平移
  （实测系统间距 23px），结果 close(16,18)/mini(39,18)/zoom(62,18)。
- **凭据**：`cargo fmt --check` 净 · `cargo test` 536 过（含常量闸门）· 宿主 clippy
  `-D warnings` 净 · 非 macOS 分支经合法 cfg 名模拟编译净（早前"报错"系自造 cfg 名触发
  unexpected_cfgs 的假象，已排除）。前端三闸门仍待补跑（本机无 node）。
- **影响**：仅周知。失败只记日志、停系统默认位，功能零影响；实机红绿灯位置待验。

### 2026-09-21 ADR-0029 · 沉浸式标题栏：唤醒 dsh 自带桌面 CSS（标记补打 + app-region→拖拽区映射） —— guan（AI 协作）

- **变更**（src-tauri 2 文件 + frontend 4 文件 + docs 2 文件）：
  - 新增 `docs/adr/0029-immersive-titlebar-marker-injection.md`（含 README 索引一行）；
  - 新增 `frontend/src/lib/immersiveChrome.ts`（纯逻辑）+ `__tests__/immersiveChrome.test.ts`；
  - 新增 `frontend/src/injected/immersive-chrome.js`（document-start，仅主窗口、仅工作台
    origin、v1 仅 macOS）：补打 dsh 官方 Electron preload 同款 `data-platform="darwin"`
    标记 → dsh 自己的桌面 CSS（透明底/侧栏 tint/topStrip）全量生效；再把 dsh CSS 的
    `-webkit-app-region` 计算样式翻译成 Tauri `data-tauri-drag-region`
    （drag→deep、no-drag→false），MutationObserver 跟随 SPA 重渲染；
  - `src-tauri/src/ui.rs`：主窗口 `#[cfg(target_os = "macos")]` 加
    `TitleBarStyle::Overlay` + `Effect::Sidebar/Active`（vibrancy）；窗口底色保持
    不透明（三处一致性不动）；新增两条内容契约闸门测试；
  - `src-tauri/Cargo.toml`：target-gated `tauri = { features = ["macos-private-api"] }`。
- **影响**：仅周知。不改 dsh 源码（红线 1）；Win/Linux 行为不变（原生装饰）；
  扫描/标记落空即静默降级回原生标题栏。dsh 升级须按 ADR-0029 §5 表复核渲染层依赖。
- **凭据**：dsh 源码实证（`/Users/guan/git/deepseek-harness`：`apps/desktop/src/main.ts:111-133`、
  两个 preload、`packages/client` 六处桌面 CSS，2026-09-21 读）；`cargo test` 534 过 /
  `cargo fmt --check` 净 / 宿主 clippy `-D warnings` 净 / 非 macOS 分支经 cfg 翻转模拟
  编译净。**前端 typecheck/lint/test 本机无 node 工具链未跑——合入前须补跑**
  （`cd frontend && pnpm run typecheck && pnpm run lint && pnpm run test`）。
  实机验证（拖拽/红绿灯避让/vibrancy/未聚焦态）待排。

### 2026-09-20 ADR-0028 · 清单打标合并：「底座组合」tab 退役，三类插件一行各带标记 —— guan（AI 协作）

- **变更**（frontend/src 8 文件 + docs 3 文件 + 新增 3 文件；本批为 ADR-0028 §5 行动项
  第 5 条「下一次意图」的落地，与 ADR-0028 第一批——面板迁入 Profile 详情页、子页下线、
  在途换档三道闸门——同一变更集提交）：
  - 新增 `lib/pluginCatalog.ts`（纯函数：三类打标 + 层序合并 + 筛选判据）与
    `__tests__/pluginCatalog.test.ts`（9 例）；
  - `ProfileDetailPane.tsx`：「插件列表」三类合一（内置 / 第三方 / 实验性），
    「底座组合」tab 退役；层栈成员在前、按**去重后**组合序，行内「层 N」标；目录 caps
    改父级取数（带档位护栏，禁双源）并下发给能力面板；旧档回读作废护栏补齐到
    reload/refreshRuntime；
  - **判据修订（真机截图回退第一版后锚定 dsh 源码，ADR-0028 §6.1）**：
    `dsh.profile.bundles` 是**激活的层列表**（reconcile 保留栈内重复、dsh 插件页自己也
    按包名去重）→ 壳侧按包名折叠为一行（不再一个包 N 行）；「内置」= **dsh 安装提供**
    （bundle 条目 ∧ 无依赖条目，镜像 dsh `removable = installed && !installation.dependencies`）
    → 用户经 npm 装进 profile 的层归「第三方」（可更新可卸载）；安装提供的层版本显
    「版本随 dsh」（bundle 行不再谎报「未安装」）。依据：`deepseek-harness`
    `packages/boot/app-boot/src/profile.ts` / `profile-plugins.ts` /
    `packages/boot/plugin-manager/src/index.ts::listBundles` 与设计笔记
    `2026-09-15-shipped-optional-bundles.md`；
  - `ExperimentalCapabilities.tsx`：目录数据受控 props 化，内部 load 退役，
    动作侧三道闸门（落账复核 / 进度按发起档归属 / 迟到 onChanged 作废）保留；
    新增 `focus` prop 承接「插件列表」的「去开关」跳转；
  - 字典两份：删 tabBundles/bundleIntro/bundleBaseTag/hiddenLayersHint，新增筛选 chips、
    三个行内标记、层序标、去开关、目录失败提示等键；删 `capTab`/`capTargetProfile`/
    `capPickProfile`（随子页下线）；
  - 闸门：`experimentalCapabilitiesGate.test.ts` 三条负载判据改指父级（判据跟架构走），
    新增 `pluginListMergeGate.test.ts`（15 例：单一入口 / 层序折叠 / 打标 / 禁双源），
    关键闸门均做先红后绿核验（拆守卫/改判据逐一复红后复原，含两条真机回归）。
  - **第三批（2026-09-20 维护者裁定，ADR-0028 §6.2）**：dsh 随安装自带的 optional
    bundle（Agent Teams 两层）**从能力面板移除**——官方插件页托管，dock 不摆第二套入口。
    过滤规则单一（`lib/pluginCatalog.ts::dockCuratedCaps`，父级过滤一次、面板与列表共用）；
    ShippedPane / 自带徽标 / 开关豁免 / 「清理旧副本」整条链删除，字典 6 键删除，
    契约字段 `shippedByDsh` / `legacyCopy` 后端仍下发、本壳不再消费；启用后其层在插件
    列表与模板层同列（内置 + 层 N + 版本随 dsh，无控制面）。
  - **第四批（2026-09-20 真机截图，ADR-0028 §6.3）**：能力的**基座与后端不再并列**。
    源码事实：`@deepseek-ai/dsh-browser-use` 官方描述 "Exclusive named browser-use
    provider registration"、README "adds no model-visible tools"——它是登记插槽、
    在每个变体里都有；provider 才互斥生效。新增 `capabilityRole`（出现在 1 个变体
    = primary 标识包 / ≥2 = shared 基座）：标识包行挂完整「实验性 · <能力>」+「去开关」，
    基座行挂弱化「<能力> · 基座」且不给跳转（该能力只装基座时才由基座承担入口）。
  - **第五批（2026-09-20 维护者要求"布局和交互着着重构"，ADR-0028 §6.4）**：
    「实验能力」模块**控制面/后果面拆分**。左栏 = 唯一控制面（一能力一行：图标 · 名称 ·
    状态 · 生效后端包名 · 开关；多变体行尾「切换后端」溢出菜单，点选即发起）；右栏 = 后果面
    （只读：后端对照 / 会发生什么 / 排障细节折叠 / 移除沉底）。**picked 中间态取消**
    （行里显示的就是事实，misaligned 隐藏态死亡）。方案经 ui-ux-pro-max 规则核验后修订：
    初稿行内 pills 被 compact-label-overflow / truncation-strategy / web-target-size
    叠加枪毙（50+ 字符包名在 25rem 栏宽里放不下、不许截断、可点目标 ≥24px），改走
    overflow-menu。闸门新增 ⑫ 五条，先红后绿核验（复活 picked / 放开生效档可点 → 复红）。
  - **第六批（2026-09-20 维护者两条裁定，ADR-0028 §6.5）**：① 「层 N」号标撤除
    （字典 tagLayer/tagLayerHint 删除，闸门翻面为反向：行内与字典都不得出现；组合序
    仍保留在排列里）；② 基座/provider **从属分组**——同能力包相邻成组，基座在上
    （anchor：完整能力标 + 去开关 + 「基座」标），provider 缩进其下（「后端」标 + 左侧
    从属线），**整组 hover 联动**；输入序不翻转从属关系（provider 字典序在前也基座居首）。
    先红后绿：组不拉相邻 / anchor 不优先基座 → 行为用例与闸门复红，复原全绿。
- **影响**：仅周知。「插件列表」里实验性包不再有行内开关/卸载（单一入口：开关与移除
  只在「实验能力」面板，行内「去开关」跳转）；用户装进 profile 的层与普通第三方控制面
  一致（可更新可卸载）；dsh 安装提供的层无控制面；Agent Teams 等安装自带能力不在
  「实验能力」面板出现（开关在 dsh 自己的插件页，启用后进插件列表）；切档时多一次
  `list_experimental_capabilities` 回读（内部一次 `dsh --dump-config`，与行表回读同族），
  开销记账在 ADR-0028 §6。
- **凭据**：前端 typecheck 0 错误 / oxlint 0 warning / vitest **597 passed**（较批前 +38）
  / 9 处闸门先红后绿核验；本批不触 Rust（`official_catalog.rs` 的自命名行修复属同变更集
  第一批，其 cargo 闸门随第一批核验）。ADR-0028 §6 已落档三项开放问题的裁定
  （层序保留 / dsh 自带层内置 / 实验性标到具体能力）与落地决策，§6.1 记录源码锚定后的
  判据修订与已知边界，§6.2 记录 optional bundle 退出面板的裁定与遗留副本出路变化。

### 2026-09-20 快车道 · 修「弹窗一开就自动弹出解释悬浮」：挂载焦点 + 焦点开悬浮两条闸门 —— guan（AI 协作）

- **触发**：维护者拿截图指出「这个 bug 也得修，每次打开都自动打开了这个 tip 黑窗」
  （「配置 MCP 服务」弹窗一开，`服务标识` 的 `Tip` 悬浮就自己弹出来）。
- **两条根因叠成**（缺一不成立，故两处都改）：
  1. `DialogContent` 的关闭键渲染在 `children` **之后** ⇒ Radix 的默认挂载焦点
     （= 容器里第一个可聚焦元素）落到「服务标识」那个图标按钮上；
  2. `@radix-ui/react-tooltip@1.2.16` 的 `TooltipTrigger.onFocus` **无条件**
     `context.onOpen()`（源码 `if (!isPointerDownRef.current) context.onOpen()`，
     没有 `:focus-visible` 闸门）⇒ "被聚焦"就等于"弹悬浮"。
- **修法**：① `ui/dialog.tsx` 接管 `onOpenAutoFocus`（调用方给了就让位）——容器里第一个
  `input/textarea/select`（排除 hidden 与 disabled）拿焦点；**没有可填字段的弹窗不接管**，
  仍走 Radix 默认（确认框因此照旧落在按钮上）。② `ui/info-tip.tsx` 的 `onFocus` 里
  对非 `:focus-visible` 的焦点 `preventDefault()`，借 `composeEventHandlers` 的
  defaultPrevented 检查跳过 Radix 后续 handler（focus 事件本不可取消 ⇒ 无副作用）。
- **两条不是重复保险**（实测）：键盘模态下的程序化 focus 同样命中 `:focus-visible`
  （本机 `activeFocusVisible: true`），故只改 ② 拦不住这个 repro；只改 ① 则留下
  「任何程序化焦点都弹悬浮」的隐患（Popover / 菜单同理）。分工：**①管焦点落在哪，②管这种焦点该不该开**。
- **影响**：仅周知，但**所有表单弹窗的开弹窗落点变了**（现在落在第一个输入框，与用户
  要填的东西对齐；确认框不变）。若上游修了 tooltip 的焦点闸门，②会变冗余但无害，届时复核。
- **凭据**：`tsc -b` 0 错 / `oxlint` 0 warning（165 文件）/ `vitest` **550 passed（58 文件）**，
  含新结构闸门 `__tests__/tooltipFocusGate.test.ts` 3 条（钉"弹窗自己决定挂载焦点 + Tip 只对
  `:focus-visible` 开"两条不变量，回归形态 = 有人把 autofocus 交回默认）。
  浏览器真机验证（devMock，profiles 窗）：打开「配置 MCP 服务」⇒ `activeElement =
  INPUT#mcp-form-name`、`role=tooltip` **0 个**（关闭再开同样 0）；hover 仍开
  （`data-state=delayed-open`，computed `bg rgb(13,18,31)` / `color rgb(232,236,246)`、
  `13.5px/21.94px`，非纯黑块）；键盘焦点仍开（`instant-open`，读到「生效范围」那条说明）；
  无字段的「移除 MCP 服务」确认框 ⇒ 不接管，Radix 默认聚焦「取消」。
  ⚠️ **两条未实测**：(a) 鼠标模态下"程序化 focus 且 `:focus-visible=false`"这一分支在本机
  造不出（in-app Browser 无可见表面，隐藏页里所有程序化 focus 都判为键盘模态），该分支只有
  源码与闸门证据；(b) 像素级截图未取到（`take_screenshot` 报
  `NATIVE_BROWSER_VIEWPORT_UNAVAILABLE`，viewport 584×598 / `visibilityState=hidden`），
  且窗口撑不到 profiles 真实 `1180×780`——视觉复核请维护者开一次 Browser 面板看这个弹窗。

### 2026-09-19 UI 优化 · 审美升级三批：字号阶梯 / 动作强调 / 导航层级 —— guan（AI 协作）

- **触发**：维护者「全面审查 UIUX 布局等做的不好的地方，然后开始优化」+「可以做 UIUX
  审美升级，不仅仅局限于当前」（授权重排层级/节奏/配色，不限 token 收口）。密度取舍经
  裁定：**可读性优先，接受一屏少约 15% 行**。字体仍限系统栈（前端禁发网络请求 ⇒ 不引 webfont）。
- **批次 1（阶梯与字重）**：`index.css` 的 `--text-*` 重排为**成对声明行高**的阶梯
  （micro 9→10.5 / meta 10→12 / label 11→12 / note 13→13.5 / body·lead 15→17…），
  策略是**改 Tailwind 原生档名的值**而非新增档名 ⇒ 数百调用点零改动即整体抬字号。
  字重契约收口：`bold` 只留展示型大字，`semibold` 为结构上限，徽标/按钮/选中文字回落
  `medium`（判据：**先字号、再颜色、最后才字重**）。等宽只用于标识符，中文从 `font-mono`
  摘出（macOS 裸 `monospace` 落到 Monaco，含汉字 ⇒ 混排打字机感），计数改 `tabular-nums`。
- **批次 2（动作强调与选中态）**：`ui/button.tsx` 新增 `destructive-ghost`——行内删除/
  卸载键静态 `text-dim` + 透明底，hover 才出 danger（实测 `rgb(175,5,50)` /
  `rgba(175,5,50,.1)`）；**实心 destructive 只留给 `ConfirmDialog` 的确认键**（那才是
  真正下单承诺的一步）。市场卡片不再挂实心 primary（一屏 20 张同款蓝按钮 = 没有主行动）
  改 `outline`。选中只说一遍：`ProfileRow` 去掉蓝色左竖条（只留 `border-brand/40`），
  左竖条收回给运行态；会话行的复制键与删除键成对同权重（不再一框一无框）。
- **批次 3（导航层级与版面节奏）**：`ui/segmented.tsx` 升两级——`md`=换页面（顶栏），
  `sm`=换面内内容（实测 37/31/13.5px vs 30/24/12px），此前两级同款同尺寸，插件中心顶栏
  与页面内子 Tab 读起来像兄弟；新增 `stretch` 让详情面板四段 Tab 靠**整宽等分**而非
  "更大更亮"分层级（顺手收掉第 6 份手搓分段器与 2 个裸 `<button>` 图标键）。系统控制台
  左栏 `md:sticky md:top-16`（实测滚动后 top=64px），消除右列 900px / 左列 274px 的
  626px 底部空洞。去噪：停用行不再划删除线（标识符划掉读不动，且已由徽标+开关+底色三重
  表达）、boot 卡片目录名徽标只在 `title≠name` 时出现、`runtimeSummary` 改纯计数图例、
  关于页 `lines.idle` 与「查看预览版」后缀的复读收掉（`previewView` 键已删）。
- **影响**：`index.css` 与 `cn()` 同为全站面，他人若有未合并的 UI 分支，**重新看一眼
  行高与字重**即可，无需改代码（档名未增删）。7 条破坏性入口的 `ConfirmDialog` 闸门
  （`destructiveConfirmGate.test.ts`）与 `paletteTokens` 禁硬编码色闸门均未放宽。
- **凭据**：`tsc -b` 0 错 / `oxlint` 0 warning（164 文件）/ `vitest` **547 passed
  （57 文件）**，未新增测试依赖。视觉证据按**各窗口真实 `inner_size`** 取图（main
  1280×820 / profiles 1180×780 / about 480×580，见 `src-tauri/src/ui.rs`、
  `commands/window.rs`），并用 `getComputedStyle` 成对读色：每屏恰好 1 个实心 primary
  （列表=新建工作台、会话=一键体检，插件/控制台/关于=0）；DOM 内 `line-through` 节点
  归零；状态行实测渲染为「运行 7 · 失败 1 · 加载中 1 · 停用 3」。
  ⚠️ 唯一**未经像素级证明**的一条：`BootSelector` 在 dev 主窗口走不到（渲染的是启动
  等待屏），该处只有代码级验证。

### 2026-09-19 补记 + 修复 · InfoTip 悬浮整块乌漆麻黑：`cn()` 把颜色类合并掉了 —— guan（AI 协作）

- **补记原委**：同日「UI 收口」条目的凭据栏只核对了 **DOM 里有没有这段文本**（`innerText`
  读得到），没核对视觉，并写下"本机截图不可行"。该判定**已失效**——本轮 in-app Browser
  能出图，像素级复核做成了。维护者截图打脸：悬浮层是纯黑一块，字完全看不见。
  按本档规矩旧条目不改写，另立此条。
- **根因三层**（第一层是主因，且**源码读不出来**）：
  1. `cn()` = `twMerge(clsx())`。tailwind-merge 只按 Tailwind 原生刻度认 `text-xs/sm/base`
     为字号，本仓库的 `text-label` 落进它的 **text-color 兜底组** ⇒ 判定与 `text-background`
     同族冲突 ⇒ **颜色类被静默删除** ⇒ ink 底 + ink 字。typecheck / lint / paletteTokens /
     contrast 四道闸门全绿：类名合法，只是不在最终 class 串里。
  2. 旧口径 `bg-foreground text-background` 跨两个命名族，读的人无从判断这是一对。
  3. 悬浮面零阴影 + `text-xs` 当正文；且 `data-open:` / `data-closed:` 变体永不命中
     （Radix 只写 `data-state`）——等于既没有层次也没有入场。
- **变更**：`lib/utils.ts` 用 `extendTailwindMerge` 登记五档字号（根因修在这里，全站受益）；
  `ui/tooltip.tsx` 悬浮面重做为**成对 term token**（`bg-term` + `text-term-ink` ≈ 15.8:1，
  非纯白字）、正文档 `text-note` 13px / `leading-relaxed`、`px-3.5 py-2.5`、`rounded-lg`、
  `shadow-lg`（浮层档）、`max-w-80`、`collisionPadding=8`（窄窗下不再贴住窗口边缘）、
  `animate-in fade-in-0 zoom-in-95 duration-100`（hover 与 focus 两路径同享）、箭头
  `fill-term`，kbd 死类退役；`info-tip.tsx` 不再自己传字号。
- **顺带修好的两个同源静默 bug**（同一 merge 误判）：`MarketPluginCard.tsx:107` 与
  `ExperimentalCapabilities.tsx:1562` 的 Badge 同时写 `text-micro` + `text-<颜色>`，
  旧 merge 把 `text-micro` 当颜色挤掉 ⇒ **9px 档从未生效**；`PluginOverview.tsx:267`
  SelectTrigger 的 `text-label` 与 base `text-sm` 同组互斥，现按调用方生效。
- **闸门**：新增 `__tests__/cnFontScale.test.ts`——① 行为：字号档与颜色档必须共存
  （含"乌漆麻黑那一次"的回归锚点）；② 同步：`index.css` 每新增一个 `--text-*` 档位，
  `lib/utils.ts` 的 font-size 组必须登记，否则该档名又落进颜色兜底组。**"合并即消失"的
  缺陷只有 merge 结果断言拦得住**，读源码与看类名字符串都拦不住。
- **影响**：`cn()` 是全站原语，merge 语义变化的作用面 = 同一元素同时出现「字号档 + 颜色档」
  的调用点；已全仓扫过，仅上述三处，且都是修复而非回归。**合入前请开一次 Browser 面板**
  复核 MCP 弹窗与列表两屏（本轮已自查三屏截图，见凭据）。
- **凭据**：`tsc -b` 0 错 / `oxlint` 0 warning / `vitest` **545 passed（57 文件）**。
  截图复核三屏：弹窗字段 Tip、列表行状态 Tip、实验能力页头 Tip——均为深色浮层 + 可读
  13px 正文 + 指向触发图标的箭头；computed 实测 `color rgb(232,236,246)` /
  `background rgb(13,18,31)`、对比度 15.81:1、字号 13px/行高 21px、圆角 10px、阴影生效、
  入场动画 60ms 时 opacity 0.575 → 落定 1。
- **本轮刻意未动**：MCP 列表行 `conflicted` 整卡描红（`McpManager.tsx:835`
  `border-danger/40`）在重名行多时一片红，噪声大于信号（结论已由行内徽标承担）。
  属另一处视觉裁定，已向维护者提出，另行开工。

### 2026-09-19 UI 收口 · 解释性文案不平铺：改 InfoTip 悬浮（MCP 面板 + 实验能力）—— guan（AI 协作）

- **触发**：维护者「文案是不是有点多啊，mcp 功能和实验性功能那里都直接把文案平铺了，
  显得内容很多杂乱，解释性文本可以用一个 icon 悬浮可见就行了吧」。上一轮把六个真 bug
  摆到界面上，代价正是这些解释——每条重名/表达式/装配状态各带一整句散文，一屏最多 4 个
  堆叠告警盒 + 弹窗 7 段说明，"要填什么"被"为什么"淹没。
- **裁定口径（写进 `info-tip.tsx` 头注）**：**结论留在表面，原因挂悬浮**。状态/重名/失败
  仍以短徽标可见，只有解释性长句进 Tip；刻意**不做**"整块徽标当触发器"的变体——悬浮是
  扫不到的，把结论放悬浮等于把信息藏起来。
- **变更**：新增原语 `frontend/src/components/ui/info-tip.tsx`（`Tip`）。Radix Tooltip
  原语（`ui/tooltip.tsx`）此前**零使用**，本轮启用；自带 `TooltipProvider` 一层（各窗口
  独立 JS runtime，AGENTS §4.4 红线 3，全局挂载点不止一处）；触发器是**可聚焦 `button` +
  `aria-label`**（Radix 在 focus 时同样打开 ⇒ 键盘可达）。未沿用仓库既有的原生 `title=`
  （~1s 延迟、不可样式、键盘不可达）。字典新增顶层 `tip.aria/ariaFor` 与五个短徽标键
  （`mcpScopeConflictTag` / `mcpDupSameScopeTag` / `mcpExprShort` / 四个 `mcpRuntime*Tag`），
  长句原样保留作悬浮正文；`mcpExprNote` 并入 `mcpExprHint`（消双源）。
- **MCP 面**：弹窗 7 段 `<p>` 说明全改为标签行尾的 Tip（stdio 6 枚 / http 4 枚，
  transport 提示随分支切换）；列表行最多 4 个告警盒压成**一行三枚徽标**（本层重名 /
  跨层重名 / 装配状态），每枚各挂自己的解释；`ScopeBadge` 的原生 `title` 一并移除。
- **实验能力面**：页头"这不是社区插件"段与"dsh 已内置"指路段落改挂徽标（它回答的正是
  鼠标停在徽标上时想问的）；**保持平铺**的：前置条件缺项、重启提示、失败块、能力解锁清单
  ——这些是可执行结论，不进悬浮。
- **影响**：仅周知。IPC / 落盘零改动；文案键有增有删（`mcpExprNote` 删），zh 字典为权威、
  en 必须同键（`AppCopy` 编译期兜底，漏键即 `tsc -b` 红）。
- **凭据**：`tsc -b` 0 错 / `oxlint` 0 warning（163 文件）/ `vitest` **541 passed（56 文件）**。
  `mcpScope.test.ts` 因呈现层改动漂移的窗口锚点已修，并新增闸门「**结论留在表面，长句挂在
  Tip 上（两者都不得少）**」：行级三态各配一枚 Tip（断言 `<Tip` 计数 = 3），短标签与长句
  必须同时在场——防止收拢退化成"只剩悬浮"。浏览器 devMock 逐态核对：hover 与 focus **两条
  路径都实测**渲染出 `[data-slot="tooltip-content"]` 且句子完整（列表 16 枚 Tip 标签全中、
  弹窗只剩 2 段散文、实验能力 2 枚、en-US 同 16 枚无中文漏字）。
  ⚠️ **未做像素级核对**（本机 in-app Browser viewport 0×0，无可见表面，截图与尺寸测量
  拿不到）；视觉复核请维护者开一次 Browser 面板看 MCP 一屏。

### 2026-09-19 修复 + 优化 · MCP 面板三修：行身份贯穿、`!!js` 保真、表单补齐 http（ADR-0027 §7）—— guan（AI 协作）

- **触发**：维护者「深度检测一下待提交代码中对 mcp 功能的设计和理解……全部优化，还有
  UIUX 等视觉用户体验方面也需要优化」。前一轮（ADR-0027 主体）把**两层生效范围**建模出来
  了，本轮是它暴露出的后果：**同层重名第一次被摆到界面上，而写操作只按 `(name, scope)`
  定位**——于是"点第二条动第一条"从假设变成可达路径。
- **修掉的六个真 bug**（按危害排序）：
  1. **删错/改错行**：`delete_mcp_server` / `probe_mcp_server` / `save_mcp_server` 现各带
     `row_id`。**空 ⇒ 按名字首条命中**（兼容无 `id` 的手写行）；**给了却没命中 ⇒ 报错**
     （"请刷新列表后重试"），不再回退按名字操作另一行、也不再"找不到就 append 一行"。
  2. **`!!js` 表达式行被展平**：serde_yaml 解析到 `Value` 时静默丢标签 ⇒ 保存一次就把
     `!!js process.env.X` 写成字面量，**密钥引用从此失效且界面看不出来**。现：`expr` 从
     原文判定（粒度 = **顶层条目**，因 `PatchFile` 重序列化粒度就是顶层条目）、结构化保存
     一律拒绝、纯启停走**逐字节文本改写**（`toggle_disabled_in_text`）。
  3. **streamable-http 行渲染成"没配命令"**（前端恒显示 `command args`）⇒ 表单补 transport
     分段 + URL + 请求头，列表按传输分派（http 显示端点，stdio 显示命令 + cwd）。
  4. **编辑全局层条目 = 凭空多一条 profile 条**（表单不带 scope）⇒ `openEditDialog` 带层，
     改名/改层**先存后删**（反序会在保存失败时丢条目）。
  5. **装配状态徽标恒空**（扫 `mcp__` 前缀，而清单给的是插件行）⇒ 按 `moduleName` +
     `entryId`（剥 `include:`）匹配，`rowId` 精确、名字兜底；四种结论各有文案。
  6. **探测假通过/假失败**：引擎未就绪时回落"当前进程 PATH"探到的命令 dsh 子进程根本看不
     见 ⇒ 改为显式失败；预算 15s → 30s（`pnpm dlx` 首跑在装依赖不算超时）；`expr` 行不探。
- **UI/UX（同一屏）**：两层分区头带**文件名**（"写在哪个文件"是排查第一问）、行内启停
  Switch（不再"看徽标 + 进弹窗"两段式）、ENV/请求头默认打码点击显形（这块正是截图求助的
  场合）、重名两种情形分别陈述并给出**加载序第几条**、表达式行徽标 + 可改范围说明、
  参数按**引号语义**切分（`"/Program Files/…"` 算一个参数，`splitArgs`/`joinArgs` 互逆）、
  名称非法即时禁用保存、确认框在同层重名时点出**行 id**。
- **落档**：ADR-0027 **新增 §7 修订补录**（三条：行身份收紧 / 标签值保真只能走文本 /
  探测口径同步），并登记 `PatchFile::for_each_entry_mut` 的返回值语义坑（返回 `false` 时
  已做的改动被静默丢弃——本轮踩过一次）；IPC 登记册 §一 补"三修"块（`row_id` 入参、
  `rowId`/`expr` 字段、expr 保存拒绝、探测不回落）、§二 把 http 探测预算改 30s；
  台账复现点 23 未变（层序 / serverName 预留 / 无 npx 三条已在前一轮登记）。
- **影响**：IPC **命令条数不变（62）**，`delete`/`probe` 的 `row_id` 为**可选**入参（旧载荷
  仍可工作），但 `save` 对 `row_id` 失效改判错 ⇒ **前后端须同批上线**。`devMock` 的 MCP
  一屏现在覆盖全部异常形状（同层重名 / 跨层重名 / 表达式行 / http / 停用 / 四种装配状态），
  浏览器直开即可逐态核对。
- **凭据**：Rust `cargo fmt --check` 通过 / 宿主 `clippy --all-targets -D warnings` 0 警告 /
  `cargo test` **531 passed / 7 ignored**（其中 mcp 相关 47 passed，本轮新增 12 条：显式
  row_id 优先于名字兜底、`row_id` 失效判错、同层重名逐条删、启停保真等）；
  `cargo clippy --target x86_64-apple-darwin` 0 警告。
  ⚠️ **windows / linux 交叉 leg 本机跑不了**：`ring` 缺 Windows SDK 头、缺
  `x86_64-linux-gnu-gcc`（第三方 C 构建脚本先失败，进不到本 crate）——**不是**代码问题，
  但也意味着本机没有该两平台的 lint 面。本轮以机器核对补位：`git diff` 新增行**不含任何
  `cfg(windows)` / `cfg(unix)` 分叉**（只有一处 `#[cfg(test)]`），故缺口不落在本轮代码上；
  CI 三平台 leg 仍须全绿才算合入。
  前端 `tsc -b` 0 错 / `oxlint` 0 warning / `vitest` **540 passed（56 文件）**，含新增
  `mcpForm.test.ts` 14 条纯逻辑（名称校验正反例、参数引号往返、`dupInfo` 的 rank 与两种
  重名）与重写后的 `mcpScope.test.ts` 22 条结构闸门。
  UI 验证方式：浏览器直开 devMock，逐态读无障碍树 + 文案（探测成功卡 / `!!js` 拒绝卡 /
  密钥显形 / 同层重名确认框点出行 id / 编辑全局 http 条目带 scope+url+headers / 新建默认
  profile+stdio+`pnpm`+保存禁用 / en-US 无中文漏字）。**未做像素级核对**——本机 in-app
  Browser 无可见表面（viewport 0×0），截图与尺寸测量都拿不到；视觉复核请维护者开一次
  Browser 面板看 MCP 一屏。

### 2026-09-18 宪法级改动 · 依赖白名单入 CodeMirror 6（YAML 配置编辑面）—— guan（AI 协作）

- **触发**：维护者反馈「配置的编辑体验很差，没有 VSCode 那种编辑器体验」（DSH 引擎配置
  页，原为生 `textarea`）。经裁定选 CodeMirror 6 路线。
- **变更**：AGENTS §4.4.1 白名单回写「CodeMirror 6（仅限配置编辑面）」；新增共享原语
  `frontend/src/components/ui/yaml-editor.tsx`（行号 / 折叠 / YAML 高亮 / ⌘F 查找 /
  活动行，主题全走 term-* token，零硬编码 hex；`readOnly` + `maxRows` 支持只读视窗），
  替换三处：`DshSettingsPane` 与 `CredentialsPane` 原文模式（编辑）+ Profile 详情
  「Patch YAML」页（只读，原为生 `<pre>`，沿用 420px≈21 行封顶口径）；
  **Tab 失焦根因**：CM6 `basicSetup` 按无障碍惯例不把 Tab 绑给缩进（留给浏览器移焦），
  显式补 `indentWithTab`（故依赖含 @codemirror/commands）；只读模式刻意用
  `EditorView.editable(false)` 而非 `EditorState.readOnly`——后者连程序化 dispatch
  一起拦，换 profile 时新内容同步不进去；`contrast.test.ts` 的 dark: 闸门收紧为
  `dark:[a-zA-Z]`（原式误伤 CM 主题规格 `{ dark: true }` 对象字面量，非 tailwind 变体）。
  前端依赖 +7：codemirror / @codemirror/{lang-yaml,state,view,language,commands} /
  @lezer/highlight（构建期打包进 bundle，不触前端运行时网络红线）。
- **影响**：仅周知（用户可见：三处 YAML 面统一为专业编辑器形态；Tab/Shift-Tab 缩进）。
- **凭据**：typecheck / oxlint 0 警告 / vitest 519 全绿；浏览器真机验证（devMock）：
  编辑器挂载、行号与折叠槽渲染、键高亮 = term-brand、编辑触发 docChanged 回传状态；
  **Tab 实测** `defaultPrevented` + 焦点保持 + 多行选区缩进生效；**只读实测**
  `contenteditable=false`、模拟输入不改文档、滚动封顶正常。
  实测记录：裸数字/布尔 lang-yaml 6.1 不产 token，未虚构高亮规则（组件注释在案）。

### 2026-09-18 宪法级改动 + 快车道 · MCP 生效范围建模（ADR-0027）与两层 patch 写入登记 —— guan（AI 协作）

- **触发**：维护者「对当前 dsh-dock 的 mcp 相关功能进行优化，还需要考虑到 mcp 的生效范围」。
  上一起因（前一轮已修）：面板说"已启用"、模型侧却永远没有 `mcp__<名>__*` 工具。
- **先读上游、再实测**（依据都可复核）：
  · **层顺序**：`dsh-app-boot/lib/index.js:1005 readProfilePatches` —— bundle 层 →
    **profile 层**（`profiles/<名>/cordis.patch.yml`）→ **home 级全局层**
    （`$DSH_HOME/cordis.patch.yml`）→ `--patch` overlays；
  · **实测坐实**：`dsh --profile web --dump-config` 的输出里，`web` 层的 `mcp-dayu`
    之后紧跟 `# == /Users/guan/.dsh/cordis.patch.yml` 段的 `mcp-tempad-dev`
    —— 全局层确实进**每一个** profile 的组合树；
  · **PATH 实测**：Harness 主进程 `PATH=/usr/bin:/bin:/usr/sbin:/sbin`（`ps eww`），
    且壳的 `engines/bin` 里**没有 npx**（只有 dsh / node / pnpm）⇒ 壳与 dsh 两个宿主下
    `command: npx` 都不可达（`spawnSync` 实测 ENOENT）。这正是"MCP 配了却起不来"的头号真因；
  · **空层会砖**：`parsePatchList` 对"解析结果不是顶层数组"**直接抛错**；纯注释文件
    js-yaml 得 `null` ⇒ 该 profile **起不来**（本机 js-yaml 实测）。本文件中招过一次，
    已修（空层写 `[]`）。
- **ADR-0027 采纳的决策**：dsh 的两层用户 patch 是**两种生效范围**，壳必须同时建模——
  `McpScope::{Profile, Global}` 进 IPC 契约；`list` 读两层并逐条标注（顺序 = patch 应用顺序）；
  `save` 按 `scope` 选文件；`delete`/`probe` 带 `scope`（行身份 = `(scope, name)`）。
  **跨层同名不是覆盖**（上游 `serverName` 是加载期预留，后加载的那条实例化失败），
  故前端**显式告警且不去重**。同时补齐 `cwd` 字段、保存改**就地合并**（壳不认识的
  `failOnStartupError`/`toolCallTimeoutMs`/`maxInstructionBytes`/`reconnect` 不得被静默删除）、
  `serverName` 校验对齐上游 `^[A-Za-z0-9_-]{1,32}$`（否则"保存成功但永不生效"）、
  `probe_stdio` 复现 dsh 的 PATH 与 `cwd`（否则"探测通过但 dsh 起不来"的假通过）。
- **宪法改动（AGENTS §6）**：MCP/插件配置的写入目标从"profile `cordis.patch.yml`"
  更正为"**两个用户层**"，指针挂 ADR-0027（§11.3 回收：明细不在正文双源）。
  ⚠️ **提请维护者注意**：§6 当前 **47 行**，超出 §11.4 的"单节 ≤ 40 行"预算——
  这是**既有**超支（本轮仅 +1 行指针），未擅自做大范围删除，留给维护者裁定回收范围。
- **影响**：用户可见——MCP 面板多出「生效范围」徽标 / 作用域选择器 / 跨层冲突告警 /
  `cwd` 输入 / 装配状态徽标；`全局` 条目自此可见可改可删。**需重启 Harness** 才对
  desktop profile 生效（本机两层配置已修好：全局层保留可用那条，profile 层重复行已移除）。
  IPC **命令条数不变（62）**，但 `delete_mcp_server` / `probe_mcp_server` 增 `scope` 入参、
  `McpServerConfig` 增 `cwd`/`scope` 字段——登记册已同步，**前后端须同批上线**。
- **凭据**：Rust `cargo fmt --check` 通过 / `clippy --all-targets -D warnings` 0 警告 /
  `cargo test --lib` **519 passed / 7 ignored**；前端 `tsc -b` 0 错 / `oxlint` 0 warning
  （159 文件）/ `vitest` **519 passed（55 文件）**；新增 Rust 用例 9 条
  （两层读写删、跨层同名不去重、`cwd` 往返、未建模键存活、既有行 id 不被改写、
  切传输不留残键、名称校验正反例）+ 前端结构闸门 16 条（`mcpScope.test.ts`）。
  实机复核：`dsh --profile web --dump-config` 确认全局层条目进 `web` 的组合树；
  `js-yaml` 实测纯注释 patch = `null` ⇒ `parsePatchList` 会抛（空层必须 `[]`）。

### 2026-09-17 裁决 · dsh 0.1.6-alpha.2 起自带 Agent Teams：实验能力页「dsh 自带的、dock 不代管」 —— guan（AI 协作）

- **触发**（维护者给了两张截图）：「dsh 新版本已经把智能体团队插件内置了，我们的实验性功能可以把
  那儿撤了吧，我看他会显示在系统基底上了」，并要求"看看 dsh 最新的代码，看这个模块该怎么调整才
  最合理，也要考虑 dsh 后续发展（可能逐渐都把实验性的功能内置）"。
- **先读上游，再定方案**（依据都可复核）：
  · `packages/boot/app-boot/src/profile.ts` 的 `OPTIONAL_BUNDLES` ＝ 两个 Agent Teams bundle；
  · 设计笔记 `.agents/notes/implemented/process/2026-09-15-shipped-optional-bundles.md`：
    随安装包下发、默认关、dsh 自己插件页开关、**永不卸载**；并**否决**了"面板按名字从 registry
    装官方 bundle"这条路（＝本模块原来的做法）；
  · 真机实测（本机正式档引擎）：`dsh 0.1.6-alpha.2` 的 `node_modules/@deepseek-ai/` 下确有
    `dsh-experimental-agent-team-profile` / `-web-profile`，且用户 `~/.dsh/profiles/web` 的
    `dsh.profile.bundles` 里已有它们（**不在** profile 依赖里 → 由 dsh 提供）；dev 档引擎是
    `0.1.6-alpha.1`，没有这两个包——**这正是"随 dsh 版本而变"的实证**。
- **落地**（ADR-0020 §7.2 第三次修订）：
  · **判据 = 这次安装实测**（不写死旗标）：某能力的每个包都在
    `<engines>/dsh-runtime/node_modules/` 里 ⇒ 这个安装自带它（`installation_shipped`，纯函数
    + 单测）。理由是同日实测到的分叉：dev 档引擎 `0.1.6-alpha.1` **不带**、正式档 `0.1.6-alpha.2`
    **带**——旗标必然在其中一边说谎；实测判据还会自动跟上 dsh 后续内置的节奏；
    `auto-review` / `browser-use` / `computer-use` 三边都没自带，继续由 dock 策展；
  · 视图加 `shippedByDsh` / `legacyCopy`；自带能力走**独立详情面**（不套"互斥变体"模型——
    dsh 侧是两个各自可开关的官方插件，宿主层与 Web 层并不互斥），界面**无开关、无安装、无移除**，
    只说明"归 dsh 管、开关在 dsh 插件页" + 列出随 dsh 自带的包名；
  · **遗留副本**：本 Profile 还持有那些包时（dock 早期按 profile 装的），给**唯一**动作
    「清理旧副本」（走既有破坏性确认链）——不清理会持续遮蔽 dsh 自带的那一份；
  · `handleSwitch` 加防御式早退（界面藏了按钮、计划还在跑就是半吊子）。
- **顺手修掉的相邻 bug**：Profile 详情「底座组合」把**每一层**都描述成"Web 界面与交互控制台
  渲染器"（截图里 agent-team 两层就是这么显示的）——原来是个二元判断；现已改成"只有确实认识的
  两层才具体描述"，其余给中性说明，文案进字典。
- **影响**：用户可见——实验能力页少一项可操作项，多一块"dsh 已内置"的说明面；动作语义其余不变。
  **dsh 升级复核点**已写进 ADR：安装包 `@deepseek-ai/dsh` 的 `dependencies` 里一旦出现新的
  `@deepseek-ai/dsh-experimental-*`，就把对应能力标为自带（这是我们这套策展集**必然逐步收缩**
  的机制）。
- **凭据**：Rust `fmt` / `clippy -D warnings` / `cargo test` **504 passed / 7 ignored**；
  前端 `tsc` 0 错 / `oxlint` 0 warning（153 文件）/ `vitest` **498 passed** / `pnpm build` 通过；
  浏览器侧 CDP 实测：清单行第一项**没有开关**且显示「dsh 已内置」、自带面列出两个包名、
  「清理旧副本」走既有破坏性确认框（确认框内容已核对）、整页仍 820px=视口。

### 2026-09-17 补记 · 实验能力页再重做：**清单/详情分栏**（维护者验收打回）—— guan（AI 协作）

- **原委**：上一条（同日）交付的 v2 卡片版被验收打回——「进去占用的空间也太多了，一屏只能
  看到一个工具，点击详情还又加长卡片内容」。所以这是**同日第二次重做**，不是新功能，
  按 append-only 惯例另发补记，不改写上一条。
- **先量后改**（不靠感觉）：`pnpm dev` + 隔离 user-data-dir 的 headless Chrome（CDP）
  在**真机默认窗口 1280×820** 上量出来——整页 **1897px**、四张卡 341/530/360/278px、
  一屏内完整可见 **1** 张。病根不是"卡里字多"，而是**把清单与详情塞进同一个纵向流**：
  详情于是只有"内联展开（越点越长）"或"藏起来"两条路，两条都错。
- **v3 形态**（ADR-0020 §7.2 第 5 条同日第二次修订）：
  · 左栏**清单行**恒紧凑（图标 · 名称 · 状态徽标 · 当前插件名 · 开关），一屏四项全见；
  · 右栏**详情面常驻**（后端切换 · 前置 · 排障细节 · 移除），**没有"展开"这回事**——
    点左栏只换右栏内容、**左栏恒不动**（1280×820 四行 66/66/66/52px）；右栏不裁剪内容，
  故三后端那一项整页仍会长出约 70px（820 → 889）——口径已在 ADR/发布日志/验收清单里改准；
  窄窗口（<1024，应用最小 960）降级为下钻 + 返回；
  · 详情面内的「实现细节（排障用）」默认收起（出问题时才看的东西），展开只增长右栏；
  · 共同前置与**各档共用基座包**提到面板级只讲一次（`dsh-browser-use` 原本在三行里
    重复三遍）；新纯函数 `commonSharedPackages` + 单测。
- **改完再量**（同一窗口）：整页 **820px = 视口高**（**不再滚动**），四行 66/66/78/52px
  全部完整可见；切到三后端的能力也只涨到 889px，左栏纹丝不动。960×640 下四行各 52px，
  点行下钻、`返回清单` 回列表。
- **顺带抓到并修掉一个真 bug**：面板用 `variant.prerequisiteMissing !== null` 判"宿主前置
  缺失"，而 dev mock 少写了该字段 → `undefined !== null` 为真 → **四行全亮"前置缺失"红灯、
  开关全灰**。改 `Boolean(...)`，并把 mock 补齐成与 serde 同形，加闸门 §⑨（变体数与
  `prerequisiteMissing:` 次数必须相等，且至少一条真值，否则硬门分支在 dev 里走不到）。
- **门禁**（`experimentalCapabilitiesGate.test.ts` §⑧ 新增，随裁定走）：源码不得再出现
  `openDetail` / 「详情」展开键 / `AnimatePresence` 高度动画；必须两栏布局、行内不得有
  块级段落、长描述与实现细节不得进清单行；选中态不得只靠颜色（`aria-current` +
  `aria-controls` 指向详情面）。
- **独立复核（同日，子代理对抗式评审）抓到并已修的 8 条**（记在这条补记里，不另开条）：
  · **高**｜三处"层类能力"文案裸读 `toggleOffSupported`（移除确认框的 note 与要点、详情面移除说明）：
    一次**装到一半**的失败就会让面板谎称"该能力由 profile 层提供"，而那句话是假的
    （browser-use 是纯 `insert_row`）。四处统一到新纯函数 `offMeansRemove`（带"真就位"守卫）+ 单测
    + 闸门⑥扩到四处；
  · **高**｜`run` 是单槽、`busy` 却按能力算 → A 在跑时点 B 会顶掉 A 的进度，A 还能被再次启动，
    两条编排行交错（行写不入队、`plugins.rs` 无互斥）→ 可能丢配置写入。改为**全局 busy**
    （行与详情面都收 `busy={run !== null}`）+ 在途时禁止换档 + 闸门⑩；
  · 在途动作的回读改用**请求令牌**：否则旧档的 `finally` 会用它发起时的 profile 覆盖面，
    下拉显示 B、清单是 A 的事实（接着按 A 的 `installed` 算计划装进 B）；
  · 换档时补清 `dirty`：原来会留着"配置已变更"，点「立即重启」重启的是**没改过**的那个档；
  · 组件里 5 处**全角标点**进字典（en 界面会出现 `…capabilities：…`），闸门④ 的 CJK 口径
    对齐 `enUsNoLeak`（原来只查汉字，全角标点在盲区）；
  · 「需要处理」不再把**用户自己关掉的**「已停用」算进去（面板给自己造成的正常状态报警），
    单列一档中性色说明；
  · 移除确认框的"卸载 N 个包"改按**真会卸的**数（partial 时报总步数是谎报）；
    无"当前生效档"但别的档已有包就位（冲突态）时，新增如实告知"会先移除哪些包"（用后端算好的 `displaced`）；
  · 无障碍：radiogroup 补齐键盘语义（roving tabindex + 方向键/Home/End，组名改「后端插件」，
    组内子项**全部**是 radio）；被前置门挡住的档不再可选（选它只会把行开关切到一个必灰的档）；
    灰开关的原因与"选中≠生效"进 `aria-describedby`（原来只在 `aria-hidden` 图标与悬停里）；
    实现细节的折叠控件把 `<h4>` 换成 `<span>`、`aria-controls` 指向常驻元素；
    窄窗口下钻时焦点跟着搬（清单隐藏不再把焦点留在 `display:none` 里）。
  · 闸门强度：两栏改为钉"固定轨 + 自适应轨"（原正则挡不住退回纵向堆叠）、插件名必须完整折行、
    `offMeansRemove` 四处共用、全局 busy、mock 载荷按契约 `satisfies Capability[]`
    （少字段编译期就红，比数出现次数强）。
- **影响**：用户可见——实验能力页从"卡片流"变成"清单 + 详情面"；功能动作语义未变
  （开 / 关 / 换后端 / 移除四种动作与确认链原样）。发版日志与验收清单 S4 已同步。
- **凭据**：`tsc` 0 错 / `oxlint` 0 warning（153 文件）/ `vitest` **495 passed**（新增 18 条）/
  `pnpm build` 通过；Rust `fmt` / `clippy -D warnings` / `cargo test` 全绿（Rust 本轮未改）。
  浏览器侧另跑了四组 CDP 实测：版面量测（1280/960）、键盘 Tab 顺序与七个控件的 focus ring、
  radiogroup 方向键（选中态与焦点同步）、窄窗下钻的焦点搬运（清单隐藏后焦点=详情面，
  「返回清单」后焦点=原行）。
  截图与量测脚本在 `/tmp/hotcheck/`（临时，不入库）：`before-1280.png`（改前）/
  `probe-1280c.png`（改后）/ `probe-960.png`（窄窗下钻）。

### 2026-09-17 重构 · 实验能力页按「插件名 + 版面规范」重做（顺带修掉三个真 bug）—— guan（AI 协作）

- **触发**（维护者看了官方口径那一版截图后的两条裁定）：
  ①「实际上就是插件，名字使用插件名就行了吧」；
  ②「这一页的 UI/UX 需要优化规范一下」（并按 `frontend-design` 走一遍）。
- **① 命名改用插件名**（ADR-0020 §7.2 第 5 条同日修订）：一个"后端"本就是**一串插件**
  ——browser-use 三档都是 `@deepseek-ai/dsh-browser-use` 基座 + 各自 provider；
  Agent Teams 的 Web 档 = 基座 + `-web-profile`，而"自建档"**就是基座本身**。
  于是卡片**一档一行、行首即插件名**，不再出现「Web 档 / 复用已装的 cua-driver」这类
  我们发明的名字（用户要拿名字去对 `node_modules`、`cordis.patch.yml`、上游 README，
  自造名反而对不上）。配套引入纯函数 `variantPackageRoles`（标识包 = 该档独有；
  基座包另标「另装共用包」；无独有包时标识退回自身全部包——"自建档"那档）+ 单测；
  确认框 / 替换提示 / 失败点名也统一改用插件名（不再出现卡片上已消失的档名）。
- **② 版面规范**（不新造风格，落实仓库既有 token 纪律）：
  · 刻度只走 token：标题 lead(15) / 正文 note(13) / 次要 label(11) / 节标 meta(10) / 角标 micro(9)；
  · 圆角按角色：面 xl(14)、可选行与控制件 md(10)、状态徽标胶囊；
  · **品牌色只承担"选中 / 主操作"，状态色只承担状态**（旧版把两者画成同一个点）；
  · 各档**共同前置去重**成卡片级一条（Chromium 那行原样重复三遍，把真正的差异淹了）；
  · 「详情」从标题行挪到「插件」节标右侧（离它展开的内容更近），改用有边框小控件；
  · 动效克制：卡片入场上浮 4px（≤6 张错峰）、详情高度展开、`prefers-reduced-motion` 全关。
- **顺带抓到并修掉三个真 bug**（都是"渲染出来看一眼"才发现的）：
  · **行内状态徽标照抄「已启用」**：后端契约里 `activeVariant` 含"已就位但停用"，
    于是卡片头说「已停用」而行内说「已启用」——自相矛盾，已按 `v.state` 分档；
  · **用户可见文案里的 markdown 标记原样直出**：面板来源说明写着 ``@deepseek-ai/*``、
    Rust 侧 `unlocks_zh` 写着 `**权限最高**`（D6 当年只挡住了反引号）。已清文案，
    并加两道门禁：Rust `user_facing_copy_has_no_markdown_markup`（反引号 + `**`）、
    前端 `dictCopyHygiene.test.ts`（扫两本字典的**求值叶子**，避免源码正则被模板字面量误报）；
  · **单档能力在行内重复卡片头已说过的状态**（噪音），已改为仅多档时显示。
- **验证方式**（这次是真看了）：`pnpm dev` + 隔离 user-data-dir 的 headless Chrome（CDP）
  把页面拍下来逐版复盘，改了三轮；临时把 `PluginHub` 默认子页改到「实验能力」用于截图，
  **已回滚**；截图用的 Chrome 与 dev server 均已收掉。
- **影响**：用户可见——实验能力页的命名、层级与状态用词都变了；功能语义未变。
- **凭据**：前端 `tsc` 0 错 / `oxlint` 0 warning（153 文件）/ `vitest` **477 passed** / build 通过；
  Rust `fmt` / `clippy -D warnings` / `cargo test` 全绿（文案清理后重跑）。

### 2026-09-17 移除 · SSH 远程工作区（I4）整体退役 + 实验能力卡改以官方口径呈现 —— guan（AI 协作）

- **触发**（维护者三条裁定，发版前）：
  ①「SSH 远程工作区向导这个功能相关的代码逻辑都干掉」；
  ②「实验功能模块要突出是 dsh 官方实验功能，插件的名字和描述也要以官方为主」；
  ③「没做的功能不需要在 release doc 里面说，我们只说做了什么」。
- **① SSH 整体退役**（**该功能从未随任何版本发布**，故对用户不可见）：
  - 代码：`ssh_config.rs`(771) / `ssh_profile.rs`(525) / `ssh_remote.rs`(675) / `commands/ssh.rs`、
    前端 `SshWorkspaceWizard.tsx` 与 `sshWizardGate.test.ts`；`list_ssh_hosts` / `probe_ssh_target` /
    `generate_ssh_profile` 三条 IPC（登记册 **65 → 62**）＋ capabilities 三条授权；
  - 连带回收：`network_gate.rs` 的 ssh_remote 整文件登记、登记册 §二「SSH 非交互预检」行与
    §三「用户 SSH 配置」读取域、`AGENTS §6` 的 app-bundle 写入例外（§11.4 回收触发=已失效）、
    `profiles.rs::ssh_app_bundle` 与 `create_profile_with_app_blocking` 的"指定 bundle"参数化
    （唯一另一调用方就是向导，遂收回单一路径）、`resolve::user_home` 公开访问器（唯一消费者是解析器）、
    `guest::sh_quote` 恢复 `#[cfg(any(windows, test))]` 门控（当年为 ssh 解除）。
  - 文档：ADR-0023 状态改「**已撤回**」（正文留决策史）、ADR 索引同步、计划 §7 R4 行改 `❌ 已撤回`、
    roadmap §4.8 回到「未实施」、台账复现点 **18/19 标记 ❌已退役**＋§三退役记录、
    `docs/executor.md` G1–G12 清单整节退役、验收清单 D 组整体撤销（D1/D2 记录保留）。
  - **保留**：`ExecutorKind::Ssh` 是 v0.4.0 就存在的**预留位**（与本次向导无关），不动。
- **② 实验能力卡（`ExperimentalCapabilities.tsx`）**：改以**官方口径**呈现——面板与每张卡标注
  「DSH 官方」，明写这些是 DeepSeek 发布的 dsh 实验包（`@deepseek-ai/*`）、dsh-dock 只做策展与开关；
  每个包的**官方包名**（= 官方名）与**该包自己的 description**（装好后从 `package.json` 读，
  不转述不翻译）进「详情」逐行呈现；未安装时如实显示"装好后显示官方简介"，**不拿我们的转述冒充官方描述**。
- **③ 发布日志**：按「只说做了什么」重写——删除 SSH 条目、删除"暂缓/未实施"类说明
  （ADR-0024 桌面任务快跑器），「已知限制」只留已交付功能的边界。
- **影响**：① 仅周知（功能未发布，用户侧无感）；② 用户可见（实验能力页文案与包信息）。
- **凭据**：Rust `fmt --check` / `clippy --all-targets -D warnings` 干净、`cargo test` 全绿；
  前端 `tsc` / `oxlint` / `vitest` / `build` 全绿（数字见同批下方条目）。

### 2026-09-17 修复 · 插件行「已禁用 / 已停用」两词并存 + 运行态不刷新（实机报障）—— guan（AI 协作）

- **触发**（维护者实机，附截图）：外挂插件列表里同一屏既有「已禁用」又有「已停用」，
  问两者差别；随后报「我刚打开了插件却还是显示已禁用，我切换到别的标签页又切回来
  现在又是运行中——这交互逻辑有bug？」。
- **诊断**（克隆实机 + 上游源码双锚，未动现场；详见台账复现点 21）：
  ① 两个词是两个真相源、都不是错：**配置侧**「已禁用」= 壳写进该 Profile
  `cordis.patch.yml` 的意图；**运行侧**徽标 = `pluginInventory/list` 的观测
  （`enabled = !entry.disabled`，`fiberPhase` 为 null 时旧版一律报「已停用」）。
  问题在**我们把两个近义词并排摆着，且没有一句解释**——这是文案/信息架构的缺陷。
  ② 「切页才自愈」是**壳的真 bug**：开关写成功后只重取行表（配置），运行态快照停在
  进页面那一刻，所以同一行会出现"开关是开的 / 徽标说停用"的自相矛盾。
  ③ 顺带推翻一条旧结论：文档里「patch 变更不热生效、重启后生效」（ADR-0009 §4 4.4③，
  基线 v0.1.1-rc.2）对 0.1.6-alpha.1 的 **live profile 不成立**——出厂 `web` 模板
  `patchReload: live`，dsh 用 chokidar 盯 `cordis.patch.yml` 并热应用；克隆实测追加
  `disabled: true` 后 **0.43s** fiber 注销、删除后 **0.44s** 重建。所以开关旁那句
  「（重启后生效）」是与事实相反的承诺。
- **修**：
  · 运行侧徽标按真相源拆成 `运行中 / 加载中 / 失败 / 未加载（配置启用但会话内无实例）
    / 未生效（会话里这行仍是禁用）`，各带自解释 title；「没到位」不再刷成 ok 绿；
  · 开关写完后立刻重取运行态并**短轮询到落定**（400ms×8），期间该行显「生效中」；
    会话没在跑则不空转轮询，直接如实报「重启后生效」；
  · 开关标签只讲动作（`禁用/启用`），toast 按**观测结果**回报「已生效 / 重启后生效」；
    行表重取失败不再静默（原来 `catch(()=>{})` 会留下假状态）；
  · 文案全部进字典（en 侧同时补齐，修掉"运行态徽标中文硬编码漏给 en 用户"的旧漏）。
- **影响**：仅周知。用户可见行为变化 = 开关后不再需要切页；两个徽标各带 tooltip 解释。
- **凭据**：前端 `tsc` 0 错 / `oxlint` 0 warning / `vitest` **467 passed**（新增
  `pluginToggleState.test.ts` 结构门禁，钉住"写完必须追平运行态"与"文案不得写死重启"）；
  实机证据见台账复现点 21（克隆 `~/.dsh-dock-dev`，自起 dsh + 固定端口回环查询）。
- **验收**：`docs/acceptance-2026-09-16.md` 新增 **S3**（🖐 需手点）。
- **2026-09-17 补记（独立复核补正，`bb55b04`）**：本条首个提交 `5979e93` 自己带了一个
  **P0**——`set_plugin_disabled(..., disabled)` 写的是 **disabled** 口径，而文案与落定判据
  吃 **enabled**，我把 `!shell_disabled` 同时喂给了两处 ⇒ toast 动词与"是否已生效"全反
  （点"关掉"报「已启用（已生效）」；点"打开"报「已禁用（重启后生效）」，即维护者原报障
  从行内徽标挪到了 toast）。复核还抓到 P1：落定判定读了渲染闭包里的旧 `runtime`
  （先选已停止的 profile → 启动 → 再拨开关时会跳过轮询、谎报"重启后生效"）。
  已修：`toggleIntent()` 把两种口径在类型上分开且恒反相（新增组合级单测，
  变异验证过红）；`refreshRuntime` 返回本次结论，并把"查询失败"与"会话没在跑"分开
  （前者只报「配置已写入」）；待定集合化（连点两行不互相挤掉结论）。
  凭据：`vitest` **476 passed**（+9），变异重放（把口径换回旧写法）2 条门禁转红。
  **教训**：源码文本门禁抓不到实参极性——口径必须各自具名 + 组合级断言。

### 2026-09-16 补完 · 安全模式可见性与出口（横幅 + 退出按钮 + 两源说明 + 新 IPC）—— guan（AI 协作）

- **触发**：维护者实测报「已装插件列表显示『已停用』，但开关全是开」——我查后确认：
  两个真相源（**运行时**回环快照 vs **配置层** `cordis.patch.yml`）各自都对，
  而安全模式**刻意不写配置**（零文件改动、原子回退），所以两者必然并存；
  缺的是**一句解释 + 一个出口**。这是我的漏项（上一轮已自标"进安全模式后没有可见出口"）。
- **补完**：
  · 控制中心**安全模式横幅**（四个 Tab 都可见）：`本轮已临时停用 N 个插件的挂载行——
    配置文件未改动（所以列表里的开关仍是开）` + 「**退出安全模式并重启**」按钮；
  · 「实验能力」面板顶部说明：下表的「已启用」指**配置层**，本轮实际未生效；
  · 新增只读 IPC **`get_safe_mode_state`**（`ipc.rs` / `lib.rs` / `capabilities` / `tauri.ts`
    四处同步 + `ipc-shapes.json` + TS 类型；登记册 63 → **64**）。
- **凭据**：Rust `fmt` / `clippy -D warnings` 干净、`cargo test` **540 passed / 8 ignored**；
  前端 `tsc` / `oxlint` 干净、`vitest` **446 passed**（新增安全模式可见性门禁）。
- **验收**：`docs/acceptance-2026-09-16.md` 新增 S1/S2 两条（我验的部分 + 需手点的部分）。

### 2026-09-16 修复 · 安全模式的 `error: unknown option '--patch'`（参数顺序是契约）—— guan（AI 协作）

- **现象**（维护者真机）：点「安全模式启动」→ 错误卡报 `DSH 进程已退出（代码 1）错误摘要：
  unknown option '--patch'`，被分类成「宿主 DSH 参数不兼容」并建议升级 DSH（误导）。
- **根因**：`dsh_launcher_args` 把 `--patch` 写在了 **app 参数之后**
  （`--profile web --port 0 --patch X --no-open`）。启动器只认自己那几个 flag，**遇到 app 的
  参数（`--port`）就把其后全部交给 app** ⇒ `--patch` 落到 web app 手里 → 未知选项。
  位置写错时上游不会说"顺序错了"，看起来像"这个版本不支持 --patch"（连引擎版本核查都白费）。
- **A/B 实证**（克隆 dev home，同一 overlay）：旧顺序 → `error: unknown option '--patch'`；
  新顺序（`--patch` 紧跟 `--profile`）→ `dsh web: http://127.0.0.1:60058/?token=…` **就绪** ✅
  （同轮也再次确认：只停用户层 5 行的安全模式能起来）。
- **修**：`dsh_launcher_args` 顺序改为 `--profile <p> [--patch <overlay>] --port 0 [--no-open]`，
  单测期望同步（原来那条测试把**错的顺序**钉住了，所以闸门全绿也没拦住——教训：顺序类契约的
  测试要写"为什么是这个顺序"）。
- **凭据**：Rust `fmt` / `clippy -D warnings` 干净、`cargo test` **539 passed / 8 ignored**。

### 2026-09-16 修复 · 安全模式两个真机缺陷（错误卡动作拿不到 profile；A+ 口径不成立）—— guan（AI 协作）

- **现象**（维护者真机）：点「安全模式启动」只出现「重试」，点重试也起不来；
  且同轮截图里「移除该行并重启」也不见了。
- **缺陷 1（根因，已修）**：`emit_boot_error` 与安全模式分支都用
  `active_session_profile` —— 而**启动失败路径会先 `teardown_session`**，此后它恒为
  `None`：于是「移除该行并重启」下不发隔离计划（按钮消失）、「安全模式启动」报
  "查不到启动目标"并 return（点了像没反应）。修：新增 `boot::boot_target_profile`
  （会话槽 → 退回 `forced_profile` 启动目标），错误卡与安全模式分支统一走它。
  证据：现场 `safe-mode/` 目录不存在、`plugin-rows.log` 无 17:13 写入（说明分支没跑到 dump）。
- **缺陷 2（口径，已按证据回退）**：维护者裁定的 **A+**（连第三方 bundle 层行一起停）
  在真机 profile 上**起不来**：停 33 行 → dsh `exit 1` + `6 entries did not activate`
  + `pending (waiting for service: tools)`（第三方层行与被保留层有服务依赖，整层摘掉即悬空）；
  同一 profile**只停用户层 5 行 → 正常就绪**。故 `should_disable` 改为**只停用户层行**
  （段落头是文件路径 / 无归属），bundle 层行一律保留；「A+ 按服务依赖求闭包」记为后续可选项。
- **附带**：安全模式生效时把「已停用 N 个插件行（删除 <overlay> 即退出）」写进启动时间线
  （退出入口的 UI 仍在下一增量）。
- **凭据**：Rust `fmt` / `clippy -D warnings` 干净、`cargo test` **539 passed / 8 ignored**
  （safe_mode 6 项）；前端不变（441 passed）。

### 2026-09-16 新增 · 启动失败的「安全模式」（ADR-0025）＋ 错误卡交互改版（首屏给动作与影响）—— guan（AI 协作）

- **触发**：维护者要求参考官方 app 给启动失败加兜底（关掉插件再启动），并裁定范围
  **A+**（用户层行 ＋ 第三方 bundle 层行）＋「patch 语法坏」兜底本轮一并做（须二次确认 + 备份）。
- **机制**（零 dsh 文件改动）：`--patch` 临时 overlay 停用非随包层行，overlay 落在
  `<app_data>/safe-mode/<profile>.yml`；退出安全模式 = 删文件（原子回退）。
  上游机制盘点与锚点见 ADR-0025 §2（含官方 app 的 `plugins-disable-all` **只重置
  `dsh.profile.bundles`、治不了 `cordis.patch.yml` insert 行** 这一关键差异）。
- **链路**：`safe_mode` 模块 + `plugins::row_attributions_blocking`（与可见行表共用同一次
  `--dump-config` 与解析器）＋ `LaunchSpec.patch_overlay` / `shell::dsh_launcher_args`
  ＋ `terminal_action` 三个动作（safe_mode / safe_mode_exit / safe_mode_reset）。
- **交互改版**（同日维护者第二次裁定）：诊断卡**默认收起**——首屏主角是"可点的动作 +
  每个动作会造成什么"（按钮旁一句影响文案），错误事实（标题 + 一行摘要）与全部动作
  在收起态依旧可见；详情/建议/原始日志收进展开区。`safe_mode_reset` 走 ConfirmDialog。
- **登记**：ADR-0025（已采纳）＋ ADR 索引一行；复现台账**复现点 22**；AGENTS §6
  已登记落盘资产 += `safe-mode/`。
- **凭据**：Rust `fmt` / `clippy -D warnings` 干净、`cargo test` **539 passed / 8 ignored**；
  前端 `tsc` / `oxlint` 干净、`vitest` **441 passed**、生产构建通过。

### 2026-09-16 调研 · 启动失败的"安全模式"：上游机制盘点 + 我方可行性实测 + ADR-0025 草案 —— guan（AI 协作）

- **触发**：维护者要求参考官方 app 给"启动报错"加一条安全模式兜底（关掉所有插件再启动）。
- **上游调研**（只读，锚 `0d1f500`）：
  · CLI **没有** safe/no-plugins 类开关（`args.ts:145-149`）；`dsh --help` 里的 `rescue` 只是示例名。
  · **官方桌面 app 有**：失败页提供「Disable all third-party plugins and retry」
  （`apps/desktop/src/startup-document.ts:12-26`），机制是**把 `dsh.profile.bundles` 重置为
  `[dsh-base, dsh-web-app]`**（`project-manager.ts:83,335-344`）——但它**清不掉
  `cordis.patch.yml` 的 `- insert:` 行**，治不了本次事故，不能照搬。
  · `--patch` 是**最高优先级 overlay**，各层挂载前拍平成单列表 → 可按 id 跨层定位
  （`profile-boot.ts:212-219,246-249`）；id 匹配不到**只 warning**（`include/src/index.ts:110-112`）。
  · `$DSH_HOME/cordis.patch.yml` 是**第二个用户层**且优先级更高（`profile-boot.ts:243`）——
    救援 profile 方案躲不开它。
  · 失败路径**无重试/降级**（`app-boot/src/index.ts:867-916`）；行级无 `optional`（`entry.ts:9-22`）。
  · **重要分歧**：`vendor/loader/src`（@0d1f500）对坏行**宽容**（降级 warning），而
    `vendor/loader/lib`（陈旧构建，与已安装运行时**逐字节相同**）是**严格**的（抛出+整组回滚）。
    ⇒ 本机跑的是严格版（解释症状）；"重新构建 dsh 能否消除非 required 行的致命性"**未实测**，
    列为选项 E/留档不实施。
- **我方实测**（克隆 dev home，未动现场）：`--dump-config` − `--dump-default-config` 的 id 差集
  **恰为用户层 5 行**（含坏行）；用该差集生成 `--patch` overlay → **坏 profile 正常就绪**；
  对照组原样启动 exit 1。patch **语法坏**时 `--dump-config` **exit 1** ⇒ 需分层兜底。
- **产出**：`docs/adr/0025-boot-safe-mode-overlay.md`（**提议**，含方案 A/B、不采纳项、复审条件）
  + `docs/adr/README.md` 索引一行。**未动代码**——按 §9 先立 ADR 待裁。

### 2026-09-16 修复 · 验收第二轮抓到的两个缺陷（import 措辞分类 + 两处 UI 假象）—— guan（AI 协作）

- **触发**：维护者按 `docs/acceptance-2026-09-16.md` 做 A 组受控复现（手工写一条
  `chrome-devtools-mcp` 挂载行，但该包已卸载），诊断台**退回兜底**：标题「DSH 工作台启动失败」、
  建议「详情见日志」、并摆了一个**必然再失败**的「重试」。
- **缺陷 1（分类器措辞缺口）**：模块解析失败用的是 `failed to **import** loader entry`，
  而 09-16 早先实现只认 `failed to **apply** loader entry` → 解析返回 `None` → 落 `unknown`。
  修：两种措辞都认（`LOADER_ENTRY_MARKS`），单测以**真机原文**（`ERR_MODULE_NOT_FOUND`）钉住
  行 id / 包名 / 根因，并断言该类失败**不给 `retry`**、壳自有行必须下发一键隔离。
- **缺陷 2（宽限余量过薄）**：该路径实测 **44s** 才退出，而首版宽限是 20+25=45s（只多 1 秒）。
  修：`BOOT_STALL_GRACE` 25s → **40s**（合计 60s，硬上限仍 90s），并把两次真机实测
  （`apply` 路径 34s、`import` 路径 44s）写进常量文档。
- **缺陷 3（两处 UI 假象，来自同轮截图）**：
  · 卡片徽标按**当前选中的后端**渲染 → 能力其实在生效（native 档在跑），只因点了被前置门
  挡住的档，整张卡报「需要修复」；改为徽标一律说**能力**的实话，选中档状态由 chip 表达，
  被挡的档在 chip 上加 ⚠（悬停即原因）。
  · 被前置门挡住的档仍给「修复」按钮（修复=装包+写行，后端两道都拒）→ 加 `!blocked` 门，
  出路改写进红字（先装 `cua-driver` 或换自包含档）。
- **链路核对**：坏 profile 下 `dsh --profile web --dump-config` 仍 exit 0（隔离动作的"删后自证"
  能过），故「移除该行并重启」在**当前坏态下也能一次修好**。
- **凭据**：Rust `fmt` / `clippy -D warnings` 干净、`cargo test` **533 passed / 8 ignored**；
  前端 `tsc` / `oxlint` 干净、`vitest` **440 passed**、生产构建通过。

### 2026-09-16 合并 · I1–I4 ＋ ADR-0020~0024 直合 master（`c3be5a1`，本地未推送）—— guan（AI 协作）

- **触发**：维护者指示「从昨天到今天做了很多功能，先合并到主分支，再出一份验收测试文档」。
- **合入内容**（来源分支 `refactor/experimental-capability-switches`，9 提交 / 80 文件 / +14796 −262；
  master 此前无分叉，共同祖先 `d89780a`）：
  · 治理：ADR-0020~0024 立项 + `docs/adr/README.md` 索引；IPC/网络登记册迁出（AGENTS §7）；
  复现点 13–21；AGENTS §6 追加持久化键（宪法级改动，见下"流程偏离"）。
  · I1 实验能力开关（ADR-0020，含本日两轮修订：写行前当场重判、行载荷单源 + 前置硬门 + 启动可见性）；
  · I2 会话取消归档（ADR-0021，复用 typert 回环 RPC，不碰 `workspace.json`）；
  · I3 MCP 能力探测（ADR-0022；stdio 走子进程、streamable-http 为**条目级**网络豁免）；
  · I4 SSH 远程工作区向导（ADR-0023；新增 `ssh_config.rs` / `ssh_profile.rs` / `ssh_remote.rs`，
    新登记 `~/.ssh/config` 读取域）；
  · 插件安装源策略（ADR-0006 §6）；WebView `content-visibility` 策略移除（ADR-0002 修订）。
- **合并方式**：`git merge --no-ff`，合并提交 `c3be5a1`；**未推送**（origin / cnb 的 master 仍为 `d89780a`）。
- **流程偏离（如实登记）**：CONTRIBUTING §2 对**宪法级改动（`AGENTS.md`）**要求 PR + reviewer，
  本次按维护者直接指示**本地直合 master**，未走 PR。若需回到 PR 流程，`c3be5a1` 可整体 revert
  （分支仍在，无外部依赖）。
- **合入前验证**（master 上重跑）：`cargo fmt --check` / `clippy --all-targets -D warnings` 干净、
  `cargo test` **532 passed**；前端 `tsc -b` / `oxlint` 干净、`vitest` **438 passed**、`pnpm run build` 通过。
- **配套产出**：验收清单 `docs/acceptance-2026-09-16.md`（A–H 八组、逐条可打勾；机器已验项与
  需手点项分开标注；含一次受控复现步骤用于验证"插件行导致启动失败"的诊断与一键隔离）。

### 2026-09-16 修复 · 实验插件装上后 dsh 起不来：挂载行载荷契约 + 宿主前置硬门 + 启动可见性 —— guan（AI 协作）

- **触发**：真机装完 3 个实验包后重启，工作台**再也起不来**（step3「等待服务响应超时」），
  且 `dsh-shell.log` **0 字节**——诊断台显示「详情见日志」，而日志是空的。
- **根因（隔离复现，未动现场）**：克隆 dsh home（`cp -Rc`）后拿到真实死因，退出码 1、**34 s**：
  ① 浏览器族 MCP provider 的 `Config` 里 `mode` 是**必填**，壳原先只写 `{id, name}` →
  `TypeError: Cannot read properties of undefined (reading 'mode')`；
  ② `cua-driver-mcp` 需要外部 `cua-driver` 可执行文件（本机没有）→ `spawn cua-driver ENOENT`；
  两者都让**整棵 plugin tree 拒绝加载**。③ 壳的停滞判定只有 20 s，**在 dsh 开口前就 SIGKILL**
  ——所以日志是空的（这是"没有真实日志"的真正原因）。
- **修复**（ADR-0020 §8，含 D8/D9/D10 三条缺陷）：
  · **行载荷单源** `official_catalog::required_row_config`（3 条：两个浏览器档
  `{mode: launch, headless: true}`、cua-driver MCP `{command: cua-driver, args: [mcp]}`）；
  既有坏行**就地补齐**（幂等、保留用户手写键、不重建第二行）→「修复」能真正修好；
  · **宿主前置硬门**（前后端各一道）：缺 `cua-driver` 时前端禁用开关并露原因，
  `apply_official_patch_row` **同样拒绝写行**（前端是呈现，不是闸门）；
  · **静默 ≠ 已死**：`wait_for_ready` 加 `stall_grace`（20 + 25 s），进程自退即判 `Exited`
  并取回真实错误栈；
  · **错误卡点名 + 就地修**：`BootFailure::PluginRowFailed{row_id,package,cause}` 解析上游
  `failed to apply loader entry …`（逐处扫，`include` 那条不算行 id），行归壳所有时下发
  `quarantine_plugin_row` → 一键「移除该行并重启」；该类失败**不给 `retry`**（必然再失败）。
- **契约面**：无新增 IPC 命令（复用 `remove_official_patch_row`）、无新增网络面；
  `BootErrorPayload` 加 `quarantine` 字段 → **形状闸门要求同步 `ipc-shapes.json` + TS 接口**
  （已同步，闸门 `ipc_struct_shapes_match_fixture` / `ipcShapes.test.ts` 全绿）。
- **登记**：ADR-0020 §8（第三次修订）；`docs/contracts/dsh-behavior-ledger.md` 复现点 21。
- **凭据**：Rust `fmt` / `clippy -D warnings` 干净、`cargo test` **532 passed**；
  前端 `tsc` / `oxlint` 干净、`vitest` **438 passed**、生产构建通过；
  真机等价验证：克隆体写入**带 config** 的 chrome-devtools 行 → 就绪 URL 正常（去掉 config
  即复现退出码 1）。
- **现场处理**：用户 dev home 的 `cordis.patch.yml` 已摘掉两条坏行（备份
  `cordis.patch.yml.bak-lead-repair-*`），验证就绪。

### 2026-09-16 新增 · 插件安装源选择策略（官方优先 / 失败换源 / 成功记忆 / 偏好可固定）—— guan（AI 协作）

- **触发**：真机装实验性插件大面积失败——`@deepseek-ai/dsh-experimental-*` 的部分 provider
  **在 `registry.npmmirror.com` 上未同步（404）**，官方源 200；而 `~/.npmrc` 把源指向镜像。
  维护者裁定：「对于有条件的环境，直接走官方源，没有条件的环境走 npmmirror。」
- **机制（不新增网络面）**：`dsh plugin` 是 **pnpm 薄转发器**（`apps/cli/src/plugin.ts:98`
  原文 "registry names, and every other pnpm argument pass through"），故按次传
  `--registry <url>`；取包仍发生在 **pnpm 子进程内**（登记册 §二已覆盖）。**壳不开 in-process
  网络客户端、不做可达性探测、也不改写用户的 npm 配置**（实测 `dsh plugin … add --help`
  直通 pnpm，且 pnpm 的 `--registry` 是"每条命令都接受的 rc-option"）。
- **策略**（ADR-0006 §6）：偏好三态 `auto|official|configured`（`None` = auto）。
  `auto` 先按 `pluginRegistryLastGood` 试、**无记忆时先官方**，失败后**换另一个源重试一次**；
  **双向兜底**（官方不可达 ↔ 镜像缺包，既有镜像链只覆盖了其中一个方向）。**只在换源有意义时换**
  （网络类 / 包不存在；构建审批门与 spec 非法不换，否则白等一轮还埋真因）。**成功才写记忆**。
  两个源都失败时**两侧错误都报**。Preferences 新增「插件安装源」三选项，可固定单源。
- **分类只有一处**：`plugin_registry::classify_failure` → `PluginOpOutcome.failureKind`；
  前端**删掉**了同日早先那份正则分类，改为消费后端给的 kind（AGENTS §11.3 禁双源）。
- **登记**：`AGENTS.md` §6 两个持久化键（`pluginRegistry` / `pluginRegistryLastGood`）；
  登记册 §二补注"可按次指定 registry"；**machine projection 不变**（`network_gate.rs` 无需改）。
- **凭据**：`cargo fmt --check` / `clippy --all-targets -D warnings` 干净、`cargo test`
  **523 passed**；前端 `tsc` / `oxlint` 干净、`vitest` **432 passed**、生产构建通过。

### 2026-09-16 重构 · 「官方实验室」（按包）→「实验能力」开关（按能力）：开/关/换后端/移除 —— guan（AI 协作）

- **触发**：维护者实测反馈「实验性功能开关的实现效果与交互逻辑体验极差」。逐条复现为
  ADR-0020 §7.1 的缺陷表 D1–D7（不是风格问题）。
- **变更**（分支 `refactor/experimental-capability-switches`，叠在未合并的
  `feat/capabilities-and-ssh-workspace` 之上）：
  - 呈现单位由**包**改为**能力**：四项能力（多智能体协同 / 浏览器操作 / 桌面控制 /
    自动安全审查），同族后端降级为**能力内的变体**——互斥在同一张卡内表达为"当前后端"，
    不再要求用户理解"三选一"；
  - **开关是真开关**：关闭 = 行级 `disabled`（不卸载、秒级可逆）；含 profile 层的变体
    不能纯行级关闭（层 patch 带副作用），此时关闭即移除并在卡面与确认框里**明说原因**；
    「移除」是独立的破坏性动作（清停用桩 → 删壳写的行 → 逆序卸包），走 ConfirmDialog；
  - 状态由后端一次算全（`off/on/disabled/partial/conflict` = 「包 × 行 × disabled」的函数）；
    实现细节（包名 / 钉版本 / 激活方式 / 行 id）收进「详情」折叠区，第一阅读层只留
    价值 / 前置 / 状态 / 后果；
  - **修掉 v1 的两个真 bug**：① D7 重复挂载——`activation` 是**安装前**算的，那时包还没进
    `node_modules`，任何包都被判成"需写行"，于是 profile 层包（如 auto-review）被多写一条
    `insert` = 同插件挂两份；现改为 `apply_official_patch_row` **写前当场重判**；
    ② D5 悬空挂载行——只有写行原语、没有反向原语，包被卸载后行还在；现补
    `remove_official_patch_row`（只删 `dsh-dock-` 前缀的行，删除后回读组合树自证）。
  - 上游锚点入台账复现点 20（无运行时 feature flag；「关而不卸」= 行级 `disabled`；
    层类能力须走 `remove` 才干净；两个基座包无 `dsh` 字段 ⇒ 浏览器/桌面两族可整族秒开秒关）。
- **影响**：IPC **净增 1 条**（`list_official_plugins` → `list_experimental_capabilities` 属改名，
  新增 `remove_official_patch_row`，62 → 63，四处同步 + 登记册已更）；`AGENTS.md` **未改**
  （不改宪法文件，故本分支不触发 CONTRIBUTING §2 的 PR 强制，但仍叠在未合并分支上）。
- **凭据**：`cargo fmt --check` / `clippy --all-targets -D warnings` 干净；
  `cargo test` **515 passed / 0 failed / 5 ignored**；前端 `tsc -b` 0 错误、`oxlint` 0 警告、
  `vitest` **425 passed / 51 文件**。视觉与交互经浏览器直开实测（dev mock：四态一屏 +
  启用确认框 + 进度导轨 + 失败续跑 + 重启提示），非仅靠单测。
- **独立对抗性评审核出并已修 3 条阻断项**（不是自评）：① **版本错配提示全链失效**——
  `latest` 需联网而该命令禁网，`latest_by_package` 恒空 ⇒ 提示永不可达，且前端分支还会
  把降级告知吞掉；**已删这条死路径**（§2.4 的"显示确切版本"由确认框直接列 spec 满足），
  确认框改为"降级告知优先"。② **`planEnable` 自拆同族旧后端只 remove 不删行** ⇒ 换后端
  会留下**悬空挂载行**（D5 换了个入口），且测试把该缺陷固化成了期望值；已把让位职责
  收归 `planReplace`，并加回归护栏。③ **失败归因错**：`failures` 只按能力记，"继续剩余步骤"
  会用**当前选中**的变体 ⇒ 换过选择器后等于"什么都没做还报成功"；已按「能力+变体」记，
  续跑绑定发起失败的那个变体，并在失败块里点名变体。
  另修 4 条重要项：选择器默认落在**当前生效变体**（否则刷新即退回首个，D1 的修复目标丢失）、
  dev mock 的 rowId 缺 `dsh-dock-` 前缀（dev 直开走不完一次移除）、破坏性门禁注释数字漂移、
  确认框替换文案未说明"旧后端会被卸载且不会自动装回"。
- **真机事故与修复**（2026-09-16，维护者报告 `cargo tauri dev` 起不来）：dev home
  （`~/.dsh-dock-dev`）的 patch 里被 v1 写入了两条 `insert` 行，指向 **Agent Teams 的两个
  profile 层**（实测 `dsh.bundle.patch: True`，内容全在 patch 文档、`src/index.ts` 是空模块）。
  Cordis 把层当插件挂 → `invalid plugin, expect function or object with an "apply" method,
  received object` → `plugin tree failed to load` → dsh 永不打印就绪地址 → 壳在「等待就绪」
  超时（截图如实）。**即 D7 的真机形态**。手工修复：删掉那两条错误的挂载行（保留 browser-use
  两条正当行与其 disabled 桩），备份见该目录 `cordis.patch.yml.bak-*`；修复后手工启动
  `DSH_HOME=~/.dsh-dock-dev dsh --profile web --port 0 --no-open` 复现"已就绪且 stderr 为空"。
  另记：正式档（`~/.dsh`）的 agent-team 层钉在 `0.1.5-rc.2` 而运行时为 `0.1.6-alpha.1`（错配，
  本次未动，已告知维护者）；该档 patch 另有 9 条指向已消失行的 `{id, disabled}` 桩，dsh 每次
  启动打 "entry not found" 警告，**不致命**，属旧启停路径的残留。
- **真机暴露的第二个缺陷（D8）**：Agent Teams 两档是**子集关系**而非互斥，装 Web 档时"自建档"
  必然也齐 → 误报「后端冲突」。已修：变体新增 `subsumed_by`，被包含的档显式标「已包含」、
  禁用其独立开关与移除（关它会拆坏超集档），且不再参与冲突判定；真互斥族仍报冲突（正反各有
  回归测试）。UI 经浏览器直开复核（真机数据形态）。
- **真机第 3 轮反馈（2026-09-16）**：
  - **"安装失败"是网络/镜像层，不是壳的 bug**：失败的两个 provider 里，
    `…browser-use-chrome-devtools-mcp` 在 `registry.npmmirror.com` 上 **404**（未同步），
    官方源 `registry.npmjs.org` **200 · 0.1.6-alpha.1**；另叠加一次 `tls handshake eof` 抖动。
    两个**基座包**（`dsh-browser-use` / `dsh-computer-use`）已装上并写行，所以卡片停在
    「需要修复」= 真话（能力缺后端，装不上就跑不起来）。绕开办法：profile 目录放一条
    `@deepseek-ai:registry=https://registry.npmjs.org/` 的 `.npmrc`（属用户侧配置，壳不代改）。
  - **UI 三处缺陷（本次已修）**：① 失败块里的「继续剩余步骤」与卡片底部「修复」是**同一个动作**，
    并列出现 ⇒ 有失败块时不再渲染「修复」；② 「该能力由 profile 层提供：关闭即移除」
    被 `!toggleOffSupported` 单条件触发，而**装了一半**的档同样为 false（分类要读包自己的
    manifest，没装就不知道）⇒ 一次失败的安装让卡片谎称自己是层类能力；已收紧为
    **仅 on/disabled**；③ 原始输出（registry URL + pnpm 调用链 + TLS 细节，上百字符）直接铺在
    卡面上，把"我该怎么办"淹没 ⇒ 新增纯函数 `classifyFailure`（network / notFound /
    buildApproval / unknown，抠出 registry 主机名）渲染成**一句话**，原始输出折叠在后面。
- **真机第 4 轮反馈（2026-09-16）：错误卡首屏收敛为单按钮**（维护者原话："我感觉做复杂了，
  遇到是插件相关的报错，提供一个按钮即可，点击按钮直接把插件开关都暂时关闭，然后启动"）：
  - **采纳**：`PluginRowFailed` 首屏**恰好一个**动作 = `safe_mode`（文案「停用全部插件并启动」）。
    另两条出路**不删而下沉**——新载荷字段 `advancedActions`（`boot_failure.rs::advanced_actions`）
    收进"展开详情 → 其它出路"：`quarantine_plugin_row`（只移除出错那一行，仅壳自有行）与
    `safe_mode_reset`（备份并放空插件配置，**YAML 写坏时唯一出路**）。下沉理由：删除 = 用户在
    "patch 语法坏"时无路可走；平铺 = 用户要先读懂三条机制的差别才能点（正是本次反馈）。
  - **新增诚实门**（`commands/boot.rs` 的 `safe_mode` 分支）：枚举出的可停行数为 0 时
    （坏行来自随包 / 第三方**插件包**，不在用户 patch 层）**不写 overlay、不启动**，如实报错并指向
    "其它出路"——起了也是同一张错误卡，只会把"点过按钮却回到原点"变成新的困惑。
  - **口径边界（勿当 bug）**：「关闭所有插件」= 关闭**用户层插件行**（实验能力开关写进
    `cordis.patch.yml` 的那些行）；第三方插件包（`dsh.bundle.patch`，如 Agent Teams 三层）不在其中
    ——ADR-0025 §4 已实测整层摘掉会让服务依赖悬空（`exit 1`）。这类坏行的出路是"卸掉那个插件包"，
    本轮**未实现**（触发条件 = 用户报"安全模式也起不来"）。
  - **独立复核（第三方子 agent 反方核查）抓到 1 个阻断级 + 2 个中等缺陷，已全修**：
    **①「其它出路」整块永不渲染**——`lib/eventPayloads.ts::normalizeError` 是按字段手写的边界
    白名单，新字段 `advancedActions` 被漏掉 ⇒ 两条次级出路在 UI 上**彻底不可达**（`safe_mode_reset`
    在前端没有第二个调用点＝YAML 写坏的救生门失效），而 Rust 契约测试 / ipc-shapes 形状闸门 /
    ErrorCard 源码门禁**全绿**：它们只证明"线上有这字段""组件会渲染它"，不证明**边界放行了它**。
    修法：放行字段 + **防漂移闸门**（用 `ipc-shapes.json` 的字段表逐键验"放行"，并以"临时移除放行
    → 2 条测试红 → 还原为绿"证明门禁有效）；**② 替换卡死指针**——`emit_boot_error` 是替换语义，
    枚举失败后的新卡没有"其它出路"，提示却让用户去点它 ⇒ 新增 `BootErrorPayload::with_actions`
    + `boot::emit_boot_error_payload`，让新卡**自带**唯一还能走的那一步（首屏一个「备份并放空
    插件配置后启动」）；**③ 同源旧缺陷**：kind 白名单手抄一份且漏了 `symlink_privilege_required`
    （D1 的分类被静默丢弃、回退中文文案）⇒ 改为由 `types/ipc.ts::BOOT_FAILURE_KINDS` 派生。
    另修：`QuarantineRow` 补三层形状闸门（此前无闸门，而它承载破坏性删行动作）、空计划文案
    只陈述已知事实、`impacts` 门禁要求 zh/en 双侧齐备。
    **复核发现但本轮未修（已记档 ADR-0025 §7）**：`plugin_row_failed` 两本字典都还没文案 ⇒
    en-US 用户看到中文标题/建议（正确本地化需先把行 id 收进前端结构化字段 + 字典函数型文案）。
  - **凭据**：Rust `cargo fmt --check` 干净 · `clippy --all-targets -D warnings` 干净 ·
    `cargo test` **543 passed** / 0 failed / 8 ignored；前端 `tsc -b` 0 错误 · `oxlint` 0 warning
    （153 文件）· `vitest` **453 passed / 52 文件** · `pnpm run build` 通过；
    契约面新增 `advancedActions`（Rust struct ↔ `ipc-shapes.json` ↔ TS 两侧接口，四处同步）
    与 `QuarantineRow` 形状条目。
- **真机第 5 轮反馈（2026-09-16）：安全模式机制换代 —— `--patch` overlay → 写配置（ADR-0026）**。
  维护者原话：「所谓安全模式应该是在配置文件里面把所有三方插件都设置为 disable 就行了，这样就能
  正常启动，后面可以有用户自己选择启动哪个插件，我们做好用户提醒这些用户体验就好了」；
  「可以一键恢复，就是把备份好的配置文件覆盖回去」。
  - **推翻了 ADR-0025 §4 的一条错误结论**：旧记「三方 bundle 层行停不掉，整层摘掉必 `exit 1`」——
    **不成立**。克隆体（`~/.dsh-dock-dev` 副本，未动现场）复测：9 条三方行（5 用户行 + 4 三方
    bundle 行）写 `disabled: true` → **8.2s 正常就绪**；同一组走 `--patch` → 7.9s 就绪（等价）；
    再多停一条**随包行** `tools` → `exit 1`（`1 entry did not activate` +
    `agent-loop: pending (waiting for service: tools)`）——**这就是当年那次失败的签名**，真因是
    判据过宽：dump 段落标签形如 `@deepseek-ai/dsh-base, patched by …/cordis.patch.yml`，按"含 `.yml`
    即用户行"会把**被用户 patch 过的随包行**也算进停用集合。现在判据取**段落主段**（`, patched by`
    之前），随包行永不停（`safe_mode::primary_section`）。
  - **新机制**：进入 = 在 profile 的 `cordis.patch.yml` 里给全部三方行写 `disabled: true`
    （复用 `PatchFile` + `apply_disabled_toggle`：未改条目原文保真、幂等零写入；覆写前备份
    `fs_backup::backup_before_overwrite_path`）→ 正常启动；退出 = 按**记账**里记下的那份备份
    **逐字节覆盖回去**（`atomic_replace`，不重新解析；覆盖前再备份当前态）。
    记账 = `<app_data>/safe-mode/<profile>.json`（`{disabled_rows, patch_backup, applied_at}`），
    它是"恢复回哪一份"的唯一依据（不按文件名猜"最新一份"）；`restorable` = 那份备份还在不在，
    不在则**不渲染**恢复按钮（不给点了必然报错的按钮）。
  - **删除**：`--patch` 注入、overlay 读写、`LaunchSpec.patch_overlay`、以及那条"解释两个真相源"的
    横幅/面板说明（配置层现在是唯一真相源：开关、徽标、下次启动同源）。旧的
    `<app_data>/safe-mode/<profile>.yml` 在进入/恢复时清理。回归锚：`shell::
    launcher_args_never_pass_patch…`（壳**永不**再传 `--patch`）。
  - **凭据**：Rust `cargo fmt --check` / `clippy --all-targets -D warnings` 干净、`cargo test`
    **551 passed** / 0 failed / 8 ignored（safe_mode 12 项：进入-恢复**逐字节**往返、幂等零写入、
    备份缺失如实报错、备份路径不可信拒绝覆盖、旧 overlay 清理）；前端 `tsc -b` 0 错误、
    `oxlint` 0 warning（153 文件）、`vitest` **454 passed / 52 文件**、`pnpm run build` 通过。
  - **独立复核（第三方子 agent 反方核查）确认机制成立，并抓到四处需加固（已全修）**：
    ① **档位意识丢失**（真缺陷）：原实现固定写 `user_dsh_home()`，而**快照档**的 home 是
    `<data_dir>/runtimes/fallback-home`（每次启动被重同步覆写）⇒ 会**改错文件**（动用户 `~/.dsh`
    的同名 profile）且不可能生效；旧机制本有档位意识（`--patch` 只在引擎档传），换代时丢了。
    修：`Executor::dsh_home()` 在 spawn 时记账，安全模式用**本次实际 home**，与用户 home 不同即
    显式拒绝（口径同 WSL）。② **记账按 home 键**：dev/正式档共用同一份壳数据目录，只按 profile
    名记账会让另一侧显示"安全模式中"+必然失败的恢复按钮 ⇒ `Journal.dsh_home` + 归属校验，且进入时
    把**别的 home** 的记账归档而不是抹掉。③ **内核既有缺陷落在救援路径**：`PatchFile::render`
    逐字拼接原文片段，**文件末行无换行**时追加条目会拼成 `name: '@x'- id: X`（非法 YAML）⇒
    片段补行尾换行 + 新增 `render_checked()`（覆写前读回解析自证，fail-closed）；复现先行单测
    已验证"撤掉修复即红"。④ **判据口径两处修正**：随包 bundle 取**全表 6 个**（含 headless /
    acp / sdk，避免同时含 web-app+headless 的清单被误停）；**无归属行改为"不停"**（格式漂移时
    "当成用户行"会连随包行一起停 ⇒ 主动制造 exit 1；反方向只是没救到，由空计划诚实门报错）。
    另：`profile` 层停不到 **home 层** insert 行（层序更晚）——现已如实归入"够不到"并在提示里点名，
    根治（写两个文件/两份备份）**记档未做**，见 ADR-0026 §5。
  - **同日第三次裁定：移除"一键恢复"**。维护者原话：「把『一键恢复插件配置并重启』这个按钮及功能
    移除吧，我觉得不需要了，多此一举。**恢复了还是起不来报错**」——语义上他是对的：恢复 = 把那条
    坏行原样搬回来，撤销的正是让用户回到应用的那个动作。故 `safe_mode::exit()` / `ExitOutcome` /
    备份路径校验 / `SafeModeState.restorable` / 前端按钮与四个字典键**全部删除**；出路 = 在
    「实验能力」里逐个打开要用的插件。横幅改为报"**此刻**还有几个插件行是停用的"（记账集合 ∩ 当前
    配置的停用桩：用户逐个打开后数字随之下降，全开即自动消失）。覆写前的 `.bak-<unix秒>` 备份
    **仍在**（既有写入纪律，失败即中止），需要时手工取用；界面不再提供整份覆盖回去的动作。
  - **凭据（第三次裁定后）**：Rust `fmt`/`clippy -D warnings` 干净、`cargo test` **548 passed**
    / 0 failed / 8 ignored（safe_mode 9 项：进入写入＋备份逐字节保真＋状态实时联动、层序切分、
    随包全表、幂等零写入等）；前端 `tsc` 0 错误、`oxlint` 0 warning（153 文件）、
    `vitest` **453 passed / 52 文件**、`pnpm run build` 通过。
  - **同日第四次裁定：重写安全模式横幅的文案与交互**（维护者按 PM 口径提出三点）：
    ① 旧文案「已在配置文件里停用 9 个三方插件（进入前已备份）。想用哪个，就到「实验能力」里打开
    哪个开关。」有两处**指错方向**——"已备份"诱导用户去找一个我们已移除的功能（整份恢复），
    「实验能力」只有策展能力（Agent Teams / Browser Use / Computer Use）而安全模式停的是**全部**
    三方插件；② 横幅只在**确实以安全模式进入过**时出现；③ 必须有**关闭 icon**，关掉后不再打扰。
    新文案：「为让 DSH 能正常启动，全部三方插件已停用（当前 N 个）。把你需要的插件到「插件」页
    重新打开即可。」（数字是**实时**状态，随用户开回而下降；全部打开 → 横幅自动消失）。关闭 =
    新 IPC `dismiss_safe_mode_notice`（**只写壳自有记账 `dismissed_at`，不碰 dsh 配置**），
    同轮不再出现、**下一次进入安全模式重新提示**（新事件）；关闭失败如实提示并重取状态。
    同源文案（启动时间线、错误卡 `impacts.safe_mode`）一并改掉。
  - **凭据（第四次裁定后）**：Rust `fmt`/`clippy -D warnings` 干净、`cargo test` **549 passed**
    / 0 failed / 8 ignored（safe_mode 10 项，含关闭的持久/幂等/新进入重置/跨 home 不动）；
    前端 `tsc` 0 错误、`oxlint` 0 warning（153 文件）、`vitest` **454 passed / 52 文件**、
    `pnpm run build` 通过；登记册 64 → **65** 条命令。
  - **仍未做（诚实留白）**：WSL 客体档的安全模式（缺客体侧写原语，仍显式报错不回落宿主）；
    `plugin_row_failed` 的中英字典文案（en 用户仍看到中文标题/建议，见 ADR-0025 §7 末）。
- **诚实留白**：WSL 客体档三个命令仍**显式报错**（不回落宿主）；`inspector` /
  `ptc-runtime-python` / 三个库包**有意不在策展集内**（理由见 ADR-0020 §7.5 末）；
  「改完重启该 Profile 才生效」依赖用户点「立即重启」（复用既有重启确认链，不自动重启）。

### 2026-09-15 优化/撤销 · 彻底移除壳侧 WebView content-visibility 内存策略（解决页面左右两侧显示不全与触控板滑动卡顿）—— guan（AI 协作）

- 变更：`src-tauri/src/ui.rs`（移除 `create_main_window` 中的 `webview_memory_policy` 脚本注入及 `WEBVIEW_MEMORY_POLICY_SCRIPT` 常量）、`src-tauri/src/lib.rs`（移除针对该注入脚本的单测）、`frontend/src/injected/memory-policy.js`（彻底删除）、`frontend/src/injected/handoff-curtain.js`（清理相关注释）、`docs/adr/0002-webview-memory-policy.md` 与 `docs/adr/README.md`（状态标记为已废弃/撤销并补齐排查与决策记录）。
- 影响：仅周知。解决由于 `content-visibility: auto` 隐式 Paint Containment 强行截断卡片左右两侧内容（Action 按钮、阴影、外延元素），以及 `contain-intrinsic-size` 高度估算落差在触控板滚动时引发 WebKit 主线程 Layout Thrashing、掉帧与视口跳动的问题。移除后恢复 macOS 原生平滑滚动体验且视觉不再被裁切。
- 凭据：`cargo test` 全绿；前端测试 51 个文件 425 测试全绿。

### 2026-09-15 chore(release-prep) · R5 收尾：分组提交完成（**未推送、未合 master**）—— guan（AI 协作）

- **提交**（3 条，分支 `feat/capabilities-and-ssh-workspace`，基线 `d89780a`）：
  1. `3220b1d docs(adr,plan,register)` —— ADR-0020~0024 ＋ 登记册迁出 ＋ §三 文件系统读取域 ＋ 复现点 17/18/19 ＋ AGENTS §6 写入例外。14 文件。
  2. `2f3ce7d feat(backend)` —— I1 策展目录 / I2 取消归档 ＋ 回环鉴权修复 / I3 MCP 两分支 / R4 SSH 三模块。21 文件 4595+。
  3. `0237b2b feat(frontend)` —— I1 官方实验室 / I3 探测卡片 / R2 队列可 await / R4d SSH 向导。23 文件 2002+。
- **分组口径与计划 §7 R5 的差异（诚实记档）**：R5 原计划按"① ADR ② I1 ③ I2 ④ I3 ⑤ R2 ⑥ R4"六组提交。
  实际改为**按层分三组**，原因是结构性的：IPC 契约要求 4 处同步（`ipc.rs::COMMANDS` →
  `lib.rs` handler → `capabilities/default.json` → `lib/tauri.ts`），且 `network_gate.rs`
  的登记表**同时**承载四个功能的条目——按功能切会让**每一个** commit 都处于"契约半同步"
  状态（例如"只提交 I3"会让 `tauri.ts` 多一个未登记命令，闸门当场红）。用 `git add -p`
  硬凑六组属于**形式合规**：它不增加可复核性，只增加出错面。三个 commit 的文件集**两两不相交**，
  且各自自洽。
- **未推送、未合 master**，理由有二：① `AGENTS.md` 属**宪法级文件**，CONTRIBUTING §2 的流程图
  要求"碰宪法级文件 → PR → 至少一名其他开发者 review → squash merge"，**不得走快车道直推**；
  ② 本会话无频道可见性，无法按 §10 完成"改前频道知会"。**请维护者拉分支复核后开 PR**。
- **收尾三件事状态**：① 相关测试绿 ✅（见下）；② 人肉读 `git diff` 确认无越界 ✅——逐文件核对
  过 36 个改动文件，全部落在本会话的四个功能面内；顺带修掉三处**我自己引入的**问题：
  登记册 §二 两行排错标题、§三 排到 §二 之前、`commands/mod.rs` 与 AGENTS §7 的**会漂移的条数**。
  ③ 提交 ＋ 广播：提交完成，**广播落档于本条**，但"频道知会"这一步因无频道而不适用。
- **仍未落地（诚实留白，勿当已完成）**：
  - `docs/executor.md` **MCP F1–F9** 与 **SSH G1–G12** 实机清单 **全部待跑**——真实网络/SSH 两端
    无法在单测里覆盖（AGENTS §5 口径）；
  - ADR-0023 §5 明列三项不在范围：放开 `Mode::parse("ssh")`、**会话级 capability 收敛（仍未设计）**、
    SSH 配置持久化面板；
  - 已知缺口未动：WSL 客体档的策展目录读写、WSL 客体档的 MCP 探测与 SSH 预览（均需客体侧原语，
    **有意不回落宿主**）；
  - ADR-0024（桌面快跑器）按维护者裁定**暂缓**，边界问题留白未裁；
  - 官方实验室的 `list_official_plugins` 与 WSL 档仍显式报错，未做客体读原语。
- **凭据（提交后于干净树上复跑）**：Rust `cargo fmt --check` 干净 ·
  `clippy --all-targets -D warnings` 干净 · `cargo test` **505 passed** / 0 failed / 5 ignored；
  前端 `tsc -b` 0 错误 · `oxlint` 0 warning 0 error（152 文件）· `vitest` **407 passed / 50 文件**。
  `git status` 干净。


### 2026-09-15 feat(ssh) · R4 收口：SSH 远程工作区向导（R4b/R4c/R4d 三刀，含一处关键判据修正与一处 AGENTS 写入例外登记）—— guan（AI 协作）

- **R4b — 五键校验 ＋ `BatchMode` 非交互预检**（`ssh_remote.rs` 新增，17 项测试）：
  - 校验与上游**运行时** zod **同口径**（比 cordis/schemastery 那层更严）：
    `host` 正则 `/^[a-zA-Z0-9][a-zA-Z0-9_.@-]*$/`、`node`/`helper`/`workspace`
    `startsWith('/')`、`helperHash` `/^[0-9a-f]{64}$/`（**只认小写**）。一次报**全部**问题
    ——向导要一次标完表单，逐个报错会让用户来回五轮。
  - ssh 参数锁死四项：`BatchMode=yes`（壳没有终端，弹提示即挂死）、`ForwardAgent=no`
    （**不依赖**用户配置恰好没开）、`StrictHostKeyChecking=yes`、`ConnectTimeout=10`。
    **刻意不传 `-F`**：别名从用户默认配置读出，连接也必须读同一份（否则两个真相源）。
  - 一次往返的远端体检脚本 + **纯函数输出解析**（忽略 banner/motd 噪声行）；
    `sha256sum` 与 `shasum` 都试——Linux 有前者、macOS 只有后者，而两端都可能是 macOS。
  - 路径经 `guest::sh_quote` 单引号进参：`ssh` 把远端 argv **按空格拼接**下发，
    不自己 quote 的话带空格的路径会在远端被重新切分。**为此解除 `sh_quote` 的
    `cfg(any(windows, test))` 门控以复用，而不是复刻第二份**——那个函数的全部价值
    就是注入面收口，它已有两条反例/实证测试，复制一份等于把保护范围砍一半。
- **R4c — 生成 SSH profile**（`ssh_profile.rs` 新增，9 项测试）：
  - 四包齐注册且 **`dsh-ssh` 必须最前**（其余三个都 `inject ['ssh']`；顺序即语义，
    有单测钉住）；只有 `dsh-ssh` 带 `config`；每行**稳定 `id`**（缺 id 的行会被 loader
    自动生成 id，**此后永远无法被 patch 命中**）。
  - **写后自证**：回读 patch 逐行确认 id/name/五键取值，并含**反向断言**"其余三行
    不得带 `config`"。`config` 是整体替换语义，没有深合并兜底——只凭"写入返回 Ok"
    下结论就是过度声称（`PatchFile` 出过"改了 entries 却回填旧文本"的静默失败）。
  - 四包安装**放在写行之前**且**钉运行期版本**：反过来一旦安装失败就留下指向未安装包
    的**幽灵挂载行**（dsh 启动即加载失败），比"装了没挂上"（可重试、无害）糟得多；
    而裸包名按 `latest` 解析，`@deepseek-ai/*` 的 latest 已实测会落后于运行时。
    代价诚实记档：这是一次**可能数分钟且无逐包进度**的调用，向导给明确进行中文案。
- **R4d — 向导 UI**（`SshWorkspaceWizard.tsx` 新增；入口挂在 ProfileManager **次级按钮**，
  刻意不与「新建」争主位）；结构闸门 `sshWizardGate.test.ts` 11 项，钉住三条会实质伤人的
  决策：① 非 POSIX 宿主**拦住**（提示 + 真的禁用按钮）；② **范围说明常显**且中英都明写
  "Web 视图**不会**因此远端感知"；③ 生成只允许在**预检通过之后**发起（反之会造出一份
  注定连不上的 profile，用户会以为是 dsh 的问题）。
- **关键决定 1：app bundle 取 `headless` 而非 web-app**。ADR-0023 §4 只说"生成一个自建
  profile"，未指定 bundle；实施时发现既有的 `create_profile_blocking` 会**自动**追加
  `@deepseek-ai/dsh-web-app`——而那**恰是** §3 方案 C 否决的形态（贴上"Web 工作台"标签
  ⇒ 用户以为文件树指向远端）。改为把 bundle 作为参数传入，SSH 路径取
  `@deepseek-ai/dsh-headless`（上游自述"无 Host / HTTP / 浏览器层"，正对 §1.3 的
  "POSIX headless and custom profiles"范围）。**该写入已在 `AGENTS.md` §6 按例外 #2 的
  同一机制/同一代码路径登记**（唯一差别是 bundle 取值）。
- **关键决定 2：IPC 是 3 条不是 2 条**（`list_ssh_hosts` / `probe_ssh_target` /
  `generate_ssh_profile`，登记册 60 → **62**）。ADR-0023 行动项本就以"如…"列了三个；
  预检单独成命令是为了让"不可达"有自己的错误面——并进生成命令会让"生成"按钮暗中先做
  一次网络往返。计划原判据写"2 条"，已更正。
- 台账新增**复现点 19**（四包服务映射 / 五键与运行时校验 / 非交互约束 / 平台硬错 /
  范围限定 / 无上游范本）；`docs/executor.md` 新增 **G1–G12** SSH 实机清单（**待跑**）；
  roadmap §4.8 追加落地记录并**显式留白三项未落地**（放开 `Mode::parse("ssh")`、
  **会话级 capability 收敛仍未设计**、SSH 配置持久化面板）。
- 影响：**仅周知**（新增 3 条 IPC、1 处 AGENTS 写入例外登记；无契约改动）。**§7 R4 收口**，
  计划内只剩 **R5 收尾提交**。
- 凭据：Rust `cargo fmt --check` 干净 · `clippy --all-targets -D warnings` 干净 ·
  `cargo test` **505 passed** / 0 failed / 5 ignored；前端 `tsc` 0 错误 · `oxlint`
  0 warning（152 文件）· `vitest` **407 passed / 50 文件**。


### 2026-09-15 feat(ssh) · R4a：`~/.ssh/config` 解析器 ＋ `list_ssh_hosts`（新读取域登记）—— guan（AI 协作）

- **变更**（依 §7 的 R4，**拆出的第一刀**：ADR-0023 §3 方案 D 否决了"前端解析"，
  §2.6 要求解析固定在 Rust 后端，故这一刀先落地"数据源"）：
  - 新增 `src-tauri/src/ssh_config.rs`：`parse_ssh_config` 纯函数 + `load_ssh_hosts`
    （**唯一 IO 点**）。语义与真 ssh 对齐：`#` 只在行首是注释、单值关键字只取首参、
    **首个取值优先且全局段优先**、通配段填充未设字段、`!` 取反排除、`Match` 段不参与、
    `Include` **不跟随**、双引号/续行/`Keyword=Value` 全部支持。
  - 新增 `src-tauri/src/commands/ssh.rs::list_ssh_hosts`；IPC 四处同步
    （`COMMANDS` 60 条 → handler → `allow-list-ssh-hosts` → `tauri.ts`，由 `ipc::gate_tests` 兜底）。
  - `resolve.rs` 补 `pub fn user_home()`：读 `~/.ssh/config` 要解析 `~`，而
    Windows/USERPROFILE 的分支**只能有一处**（两处各写一遍＝两套口径，差异只在平台上暴露）。
  - 前端只加类型与封装（`SshHost` / `SshHosts` / `listSshHosts`），**向导 UI 属 R4d**。
- **新读取域已登记**：登记册新增 **§三「壳的文件系统读取域登记」**（此前只有 IPC 与网络
  两册）。边界写死：只读这一个文件、不跟随 `Include`、1 MiB 上限、只取非机密字段
  （alias / HostName / User / Port / ProxyJump / IdentityFile **路径**）——**结构体里没有
  能承载私钥的字段**。本节同时说明"读取面没有机器闸门（`network_gate.rs` 只拦网络原语），
  纪律只能靠人守"。
- **台账新增复现点 18**（锚 **OpenSSH** 行为，同复现点 15 锚 OS 行为的先例）：七项全部以
  `ssh -F <file> -G <alias>` **实测回读**锚定。**开发期实测当场抓住一个真 bug**：
  初版把 `Match` 段的关键字落进"全局段"，于是 `Match` 里的 `HostName` **泄漏给所有主机**
  ——由反例单测 `match_block_does_not_leak_into_previous_host` 拦下。
  另一处是**我自己的测试写错了**：我以为行中 `#` 会被当数据留进值里，实测 `ssh -G` 显示
  单值关键字只取首参；已按实测改正断言，并把"行中 `#` 会变成额外主机名"这一反直觉结论
  改成**如实保留 + 另出 `notes` 解释**（否则用户看到下拉里有个 `#` 会以为壳解析错了）。
- **测试**（Rust **456 → 479**，+23，全部为 `ssh_config`）：正例（全字段/大小写/`=` 形式/
  多别名逗号分隔/首个取值优先/全局段优先/通配填充/取反/引号内空白/续行/CRLF）＋
  **畸形反例**（`Host` 无参、非法与越界端口、未闭合引号、纯注释/垃圾输入、通配与取反
  不得成为可选 alias）。
- **顺带修一处我上轮引入的结构错误**：登记册 §二 里 `引擎引导` / `WSL 客体投递` 两行
  在 R3 时被排到了「进程内触网」标题之下（应属「子进程内触网」）；本轮重排，并核对
  每行都落在正确标题下。§三 首次落位时也曾排在 §二 之前，已调为 一 → 二 → 三。
- 影响：**仅周知**（新增 1 条 IPC；无契约改动）。§7 R4 **未收口**——R4b（`BatchMode`
  预检 + 五键校验）、R4c（生成 profile：四包 `insert` + 稳定 `id` + 五键 `config`）、
  R4d（向导 UI + Windows 宿主排除）待做。
- 凭据：Rust `cargo fmt --check` 干净 · `clippy --all-targets -D warnings` 干净 ·
  `cargo test` **479 passed** / 0 failed / 5 ignored；前端 `tsc` 0 错误 · `oxlint`
  0 warning（150 文件）· `vitest` **396 passed / 49 文件**。


### 2026-09-15 feat(mcp) · R3：`streamable-http` 探测分支（含两条登记并存的关键裁定）—— guan（AI 协作）

- **变更**（依 §7 的 R3）：
  - `mcp_probe.rs` 新增 [`probe_http`] 与 [`post_rpc`]：进程内 HTTP 走完
    `initialize` → `notifications/initialized` → 三次枚举。**托底口径与 stdio 一致**：
    整轮 15s（5 次往返**共享一个 deadline**，不是每请求 15s）、只读、一次性快照。
  - `rpc_payloads`（纯函数）认**四种**响应形态：单条 JSON、JSON-RPC **批量**数组、
    NDJSON、**SSE**。SSE 按规范把同一事件的多行 `data:` 用 `\n` 拼接——**按行切会把
    多行 JSON 截成解析不了的碎片**，把合规服务端判成"连不上"（假失败比不报更糟）。
  - 抽出 `assemble_probe`：**两分支共用同一降级口径**（tools 失败=整单失败；
    `resources`/`templates` 遇 `-32601` 降级为"空 + notes"）。两分支各写一份
    必然分叉，且分叉只在某一种 transport 上暴露，最难发现。
  - `post_rpc` **非 2xx 不短路**：MCP 服务端常以 `HTTP 400 + -32601` 回未知方法，
    在传输层按状态码短路会让降级永远走不到。判定集中在 `pick_result` 一处。
  - `redirects=0`：① 探测语义是"这个 URL 通不通"，跟随跳转等于换了端点还报成功；
    ② 用户配置的鉴权头会随跳转泄漏到另一个 host。跳转如实报错。
  - `commands/console.rs` 按 `transport` 分派；`McpManager` 的 `ProbeCard` 零改动复用
    （R1b 的展示层本就与 IO 无关）。
  - **UI 补上 ADR 明确要求的一项**：`ProbeCard` 表头恒显快照时间（ADR-0022 §5
    「UI **必须**标注快照时间」）——R1b 首版漏了，本轮补上并把不变量写进结构闸门。
- **关键裁定：两条网络登记并存，而非"改为"**。计划 §7 原写"`Registered` 行**改为**
  条目级 `Exempt`"，实施判为**不可依此执行**：`network_gate.rs` 该行自带注释与登记册
  §二原文均写"**不得**只改那一行豁免"。正确动作 = **保留**整文件 `Registered`
  （含义收窄为"除豁免条目外本文件不得再有进程内原语"）**＋ 新增** `item: Some("post_rpc")`
  的 `Exempt`（授权"恰好一处"）。两条并存严格强于二者取一。计划判据已同步更正。
  `mcp_probe.rs` 的模块注释同时写死"`ureq` 只能以全限定路径出现在 `fn post_rpc` 内，
  模块顶部 `use ureq::…` 会让闸门当场红"。
- **补登台账复现点 17**：ADR-0022 §5 早已要求登记"transport 取值集合 / `*/list` 方法名 /
  'DSH 无 MCP RPC'"，但 stdio 分支落地时**只登记了网络面、漏回写 `dsh-behavior-ledger`**
  （红线 1 的"锚定源码位置 + 日期"因此有缺口）。本轮**在本机逐条重验**后补上，
  并新增 ④ SDK 客户端行为（`accept: application/json, text/event-stream`、
  SSE 媒体类型分支、`mcp-session-id` 头名）——自实现 HTTP 客户端必须与这三条逐字对齐。
- **ADR-0022 新增 §5.1 实施补注**（append-only，不改正文），记录三处偏差：
  ① 文件名 `mcp.rs` → 实为配置模型、探测在新模块 `mcp_probe.rs`；② 两条登记并存；
  ③ 超时 2s → 15s（2s 会把"慢"误报成"连不上"，而误报正是本功能要消灭的东西）。
- **`docs/executor.md` 新增 F1–F9 实机清单（待跑）**：stdio/http 可探测、不可达不卡 UI、
  跳转不跟随、`-32601` 降级、缺能力不撒谎、配置变更即失效、WSL 明确报错、与回环面无关性。
  **未跑项保持"待跑"**。
- **测试**（Rust **447 → 456**，+9）：SSE 多行拼接不切碎、心跳/注释零负载、
  非 2xx 取信封、批量/NDJSON 跳过通知行、301 报状态+片段+不跟随、两分支降级同形、
  SSE 端到端装配、tools 缺失即失败、缺 `url`/非 http(s) 拒绝（不发请求）。
- 影响：**仅周知**（无新 IPC、无契约改动；网络面登记已同步两处）。§7 R3 收口；
  下一步序位 = **R4**（SSH 向导，含 2 条新 IPC 与文件系统范围登记）。
- 凭据：Rust `cargo fmt --check` 干净 · `clippy --all-targets -D warnings` 干净 ·
  `cargo test` **456 passed** / 0 failed / 5 ignored；前端 `tsc` 0 错误 · `oxlint`
  0 warning（150 文件）· `vitest` **396 passed / 49 文件**。


### 2026-09-15 feat(mcp) · R1b 收口：探测结果落成行内折叠卡（R1 完结）—— guan（AI 协作）

- **变更**（依 §7 的 R1b，即 R1 的收口刀）：
  - `McpManager.tsx` 新增模块级 `ProbeCard` / `ProbeList` 两个展示组件：
    **成功 → 可折叠**（表头给 `Tools n · Resources n · Templates n` 计数，收起时也
    看得见；展开才是清单，含 `McpNamed.detail`＝tool 描述 / resource uri / template
    uriTemplate）＋ `notes` 降级说明；**失败 → 恒展开**，`mcpProbeFailed` 原样显示。
  - **探测结果不再只经通知**：R1a 的 `onNotice` 路径移除（通知瞬时、一次一条，承载
    不了"对照着看哪个 server 有哪些 tool"）。这是 R1b 的收口判据，有测试钉住。
  - **旧结果失效**（新增 `forgetProbe`）：保存配置、删除 server 各丢弃该项；换
    `profileName` 整批清空。理由不是"整洁"——继续展示等于**拿旧清单描述新配置**，
    那是给出错误事实（同名 server 在不同 profile 配置不同是常态）。
  - i18n 五键中英对称：`mcpProbeToggle` / `mcpProbeProtocol` / `mcpProbeTools` /
    `mcpProbeResources` / `mcpProbeTemplates`。
- **新增结构闸门** `__tests__/mcpProbeCard.test.ts`（6 项）：本仓库不引 RTL/jsdom
  （AGENTS §4.4），组件渲染没有断言通道，故用**窗口切片式源码断言**钉住两条**会被
  无意破坏**的落档决策——① 错误不得被折叠（`\bopen\b` 不得出现在失败分支）；
  ② 配置变更即失效（`forgetProbe` 恰两处调用 + 换 profile 清空）。
  **闸门已做反证**：临时把 `open` 注入失败分支 → ① 立即转红，随后回滚（`grep -c
  FALSIFY-TEMP` = 0）。不能证伪的闸门不如没有。
- **顺带修一处门禁判红**：`shapeTokens.test.ts` 抓到我在计数徽章上写了裸 `rounded`
  （Tailwind 默认 4px＝@theme 之外的第四档），已改 `rounded-md`——机器闸门按设计
  拦住了作者本人。
- 影响：**仅周知**（纯前端增量；无 IPC/Rust/契约改动）。**§7 R1 至此收口**；
  下一步序位 = R3（`streamable-http` 分支）。
- 凭据：前端 `tsc -b` 0 错误 · `oxlint` 0 warning 0 error（150 文件）·
  `vitest` **393 passed / 49 文件**。Rust 本轮未改动（沿用 447 passed）。

### 2026-09-15 feat(frontend) · R2：安装队列可 await ＋ 官方目录安装/卸载接入「下载管理」—— guan（AI 协作）

- **变更**（依 `docs/plans/official-plugins-and-capabilities-plan.md` §7 的 R2）：
  - `lib/queue.ts`：`kind` 增 `"remove"`；新增纯函数 `outcomeOf(item) → QueueOutcome`
    ——**可 await 契约的判据落在这个纯函数上**（`done` → `ok:true`；**未终结项一律
    `ok:false`**，防止调用方在终结前把"还没跑完"当成"成功"而继续下发）。
  - `stores/queueStore.ts`：新增 **`enqueueAndWait(input) → Promise<QueueOutcome>`**。
    实现是模块私有的 id→resolve 发令枪（`waiters`，不进 state）；pump 在终态时
    **先摘除再 resolve**，保证重试不会二次解析同一个 Promise。pump 按 `kind` 分派
    （`remove` → `remove_plugin`，其余 → `install_plugin`）。
  - `lib/officialCatalog.ts`：**删掉原「已知偏离」记档并真正收敛**——`install`/`remove`
    都改接队列（此前是直接调 `install_plugin`/`remove_plugin`）。IO 面 `install`
    签名补 `pkg`（队列面板的展示名；只用 spec 会显示 `host-layer@1.0.0` 这类来源串）。
  - `QueuePanel.tsx`：卸载项复用 done 相位（成功=绿）但文案改「**已卸载**」。
  - i18n 四键中英对称：`queueRemoveQueued` / `queueRemoveDone` /
    `queueStatusRemoved` / `queueRemoveFailedNotice`——**卸载不借安装文案**（对用户
    说"已安装"/"安装失败"是错的陈述，不是措辞偏好）。
- **两条有意保留的边界**（写进 `lib/officialCatalog.ts` 生产绑定注释）：
  ① `writeRow`（写 patch 挂载行）**不入队**——它是一次配置文件原子写，不是包操作；
  把配置写塞进队列会让面板「重试」按钮的含义分叉。② 队列「重试」只重跑**该项**，
  **不续跑** `runPlan` 的后续步骤（编排进度在调用方栈上，队列不持有）；自动续跑
  需要队列持有调用方上下文，那会让「下载管理」变成「工作流引擎」。该边界有测试钉住。
- **测试**（+11：**376 → 387**，48 文件）：新增 `__tests__/queueStore.test.ts`（6 项，
  mock `@/lib/tauri` 边界、跑真实 store 与真实文案）——成功/软失败/IPC 抛错三态都不
  reject；`remove` 走 `remove_plugin`；**串行证伪**（用 deferred Promise 钉住"前一项
  终结前后一项不得发起 IPC"——这正是"先移除同族 provider 再装新的"的语义）；重试边界。
  `queue.test.ts` +4（`outcomeOf` 四态，含未终结项不算成功）；`officialCatalog.test.ts` +1
  （包名单列传给 IO）。同步修正该文件假 IO 的 `install` 签名。
- 影响：**仅周知**（纯前端增量；无 IPC/Rust/契约改动。§7 R2 收口，R1b/R3/R4/R5 待做）。
- **UX 观察（未改，留给维护者）**：目录条目逐步走队列后，一次 3 步安装会多出
  6 条队列 toast（入队+完成 ×3）＋ 原有 1 条计划级 toast。逐条都真实且对应面板角标，
  故**没有**加抑制机制——抑制需要自己的设计（否则角标弹了却无解释）。若嫌吵，
  这是一次独立的、可用 ADR 裁定的取舍。
- 凭据：前端 `tsc -b` 0 错误 · `oxlint` 0 warning 0 error（149 文件）·
  `vitest` **387 passed / 48 文件**。Rust 本轮未改动（沿用 447 passed）。
  工作区未提交（R5 分组提交负责）。

### 2026-09-15 feat(mcp) · R1 第一刀：MCP 能力探测接入 UI —— guan（AI 协作）

- **变更**（依 §7 执行计划的 R1，**最小可验证一刀**）：
  - `McpManager.tsx`：MCP 服务器行新增「**探测**」按钮（`Wrench` 图标、探测中转圈）＋
    `handleProbe` 处理函数——调既有 `api.probeMcpServer`（ADR-0022 stdio 分支），
    结果经**既有通知通道**呈现（`Tools n · Resources n · Templates n`），
    `probe.notes` 的降级说明一并拼出。
  - **失败原样透出**（这是本轮的重点）：`streamable-http` 分支未实现 / WSL 客体档 /
    该 profile 未配置此服务器——三类错误后端各有明确文案，**吞掉会让用户误以为
    "这个服务器没能力"**，而其实是"没探测成"，两者处置完全不同。
  - i18n：`mcpProbeBtn` / `mcpProbeResult` / `mcpProbeFailed`，中英对称。
- **本刀刻意不碰 JSX 结构**（不动行内折叠卡片）：那需要重排 620 行组件的行容器，
  在没有完整验证余量的情况下做属于"改到一半"。**故 R1 尚未收口**——`§7 R1` 的
  「行内折叠卡片（Tools/Resources/Templates 列表）」仍是待办；当前能力数据已可获取，
  只是经通知呈现而非列表展开。
- **影响**：**周知**。用户首次能在「MCP 工作台」对已配置服务器**主动探测能力**并看到
  结果或**明确原因**；后端探测能力由此变为用户可用。
- **凭据**：前端 `tsc` 0 错误 · `oxlint` **0 警告 0 错误**（148 文件）· `vitest`
  **376 passed**（47 文件）。Rust 未改动（沿用 447 passed 绿态）。

### 2026-09-15 docs(agents) · §7 两张登记册迁出至 `docs/contracts/`（解除预算阻塞）—— guan（AI 协作）

- **变更**（宪法级文件改动，依 AGENTS §10 预告与归档；工作区未提交）：
  - 新建 `docs/contracts/ipc-and-network-register.md`：承接 **IPC 命令登记（59 条，
    逐字迁移，含各自落地日期与边界）** ＋ **网络面用途登记**（回环三条 / 子进程内触网三条 /
    直接触网三条，改为表格式，边界信息一字未丢）。
  - `AGENTS.md` §7 只保留**规则**与指针：新增 IPC 的登记义务 + 三处同步由闸门兜底、
    唯一网络面 + "新网络需求先登记"、**回环必须附 `/api` 会话 Cookie**（含
    `ShellState.workbench_cookie` 仅内存不打日志）、前端消费与事件清单。
  - **`AGENTS.md` 249 → 213 行**（腾出 36 行）；**§7 由 54 → 18 行**（回到单节 ≤40 内）。
- **为什么现在做**：迁出前 `AGENTS.md` 已 **249/250**——**再加一条 IPC 或网络用途登记
  就必然越界**。这与 §9 当时"补一行 ADR 索引即越界"是**同一个结构性病**：登记册随功能
  线性增长，而 §11.4 给的是固定预算。这次把**两张都**迁走，不是只挪一张——
  否则下一次撞墙只是时间问题（§6 的持久化例外册同理，**下次撞墙时的候选**）。
- **一致性校验（可复查）**：`ipc.rs::COMMANDS` 的 **59 个命令名全部**在新册中逐条命中；
  `AGENTS.md` 中命令名残留 **0 处**（禁双源，§11.3）；§7 仍完整保留全部**规则**语义。
- **影响**：**周知**。查命令清单改看 `docs/contracts/ipc-and-network-register.md`；
  写新命令/新网络用途时**改那本册**，§7 只读规则。**预算阻塞已解除**，I3 剩余
  （`streamable-http` 分支登记、McpManager UI）与 I4（SSH 向导）的新 IPC 可继续登记。
- **凭据**：行数核算 249 → 213（`wc -l`）；§7 54 → 18；命令名覆盖 59/59；
  AGENTS 内命令名残留 0；本批为**纯文档**改动，未跑测试（代码零改动）。

### 2026-09-15 feat(mcp) · I3 stdio 分支：MCP 能力探测落地 —— guan（AI 协作）

- **变更**：
  - 新模块 `src-tauri/src/mcp_probe.rs`：握手（`initialize` → `notifications/initialized`）
    后枚举 `tools/list` / `resources/list` / `resources/templates/list`。协议版本发
    `2025-11-25`（与 DSH 所用 `@modelcontextprotocol/sdk@1.30.0` 的 `LATEST_PROTOCOL_VERSION`
    一致），并**以服务端协商回来的版本为准**。
  - 合规按 ADR-0022 §3.3：spawn 经 `lifecycle` seam（`Role::Probe`，短命探测不堆 `procs/`）、
    Windows 经 `crate::child_cmd`；**本模块无任何 in-process 网络客户端**，
    并在 `network_gate.rs` 加 `Kind::Registered` 行**反向绑定**该点——将来实现
    `streamable-http`（进程内 HTTP）分支时，闸门会当场红，必须改走条目级 `Kind::Exempt`
    + AGENTS §7 登记，**不得**只改那一行豁免。
  - IPC `probe_mcp_server`（`COMMANDS` **58 → 59**），四处名字面全同步 + 新 TS 类型。
  - `AGENTS.md` §7 登记命令与"探测（stdio 分支）"网络用途。
- **两处降级/拒绝，都是有意**：
  ① 服务端未声明 `resources` 时 `resources/list` 回 `-32601` → **降级为"空 + 说明"**，
  不让整轮探测失败（否则用户看到"连不上"，实际只是"没这能力"）；其他错误原样上抛，
  **不**吞成"没能力"。
  ② `streamable-http` 服务器（无 `command`）与 **WSL 客体档**均**显式拒绝**并说明原因
  （前者指出该走 http 分支；后者因探测需在客体内部 spawn，宿主执行语义不等价）。
- **⚠️⚠️ `AGENTS.md` 已到 249/250 行（预算上限）**：本轮加登记时**只差一行**。
  这与 §9 当时的情形同源——**§6/§7 的登记册同样随功能线性增长，却挂在固定预算下**。
  下一条登记就会越界。**建议尽快按 §11.4「回收」机制再减一轮**（候选：把 §7 的命令
  清单迁到 `docs/contracts/` 下的专项册，§7 只留规则 + 指针——与 §9 迁出同一手法）。
  在该裁定之前，**新增 IPC 命令/网络用途都会先撞预算**，请知悉。
- **影响**：**周知**。I3 的**后端探测能力就位**（UI 卡片未做，用户侧暂不可见）；
  `mcp.rs` 此前完全没有探测能力，现在有了 stdio 分支。
- **凭据**：`cargo test` **447 passed**（441 → 447，+6：握手请求形状 / 通知**无 id** /
  信封解析跳过噪声并透出错误 / 具名列表回退 / `-32601` 窄判定 / http 服务器被拒且文案可操作）·
  `clippy --all-targets -D warnings` 0 问题 · `fmt --check` 干净 · **`lifecycle` spawn 闸门
  与 `network_gate` 三闸门全绿** · IPC 59 三处一致 · 前端 `tsc` 0 错误 · `oxlint` 0 警告 ·
  `vitest` 376 passed。
  **注意**：`tauri.ts` 漏包一次由机器闸门 `tauri_ts_matches_ipc_commands` 当场拦下
  （注册了 IPC 却没前端封装）——闸门按设计工作，非事故。

### 2026-09-15 fix(mcp) · I3 前置：MCP 配置模型补 `transport`（修一个静默错配）—— guan（AI 协作）

- **先发现的问题**：壳的 `McpServerConfig` 只有 `name`/`command`/`args`/`env`/`disabled`
  ——**模型是 stdio-only**，完全不认识上游支持的 `streamable-http`。而 `parse_mcp_servers`
  对缺失 `command` 的条目兜底成 `"npx"`：一个 `streamable-http` 服务器（本就是
  `url` + `headers`、**没有 command**）会被**静默误读成 npx stdio**。这是 ADR-0022
  按 transport 分支的前提缺口——分支之前，模型先得认识 transport。
- **变更**：
  - `McpServerConfig` 增 `transport: McpTransport`（枚举，kebab-case，**缺省 `stdio`**）、
    `url`、`headers`，全部 `#[serde(default)]`（向后兼容）。
  - `parse_mcp_servers`：**先判 transport**；`streamable-http` 分支**不再塞 npx 兜底**
    （`command` 保持空），并读 `url`/`headers`；未知 transport 值保守按 `stdio`（不猜新传输）。
  - `server_to_yaml`：`streamable-http` 写 `transport`/`url`/`headers` 三键、**不写**
    `command`/`args`（上游该分支没有这两字段）；**`stdio` 分支的既有写法一字未动**（零回归）。
  - 前端 `types/ipc.ts` 的 `McpServerConfig` 同步（`transport`/`url`/`headers` 可选，
    兼容既有构造点）。
- **影响**：**周知**。① 修复了 streamable-http 服务器被误读成 npx 的静默错配；
  ② 为 I3 的「按 transport 分支探测」扫清了前提（`stdio` → `lifecycle` 子进程 +
  `Kind::Registered`；`streamable-http` → 条目级 `Kind::Exempt` + AGENTS §7 登记）。
  **I3 的探测本体（握手 + 能力枚举）尚未实现**；`mcp.rs` 仍无任何探测/连接能力。
- **凭据**：`cargo test` **441 passed**（440 → 441，+1：`streamable-http` 不得被误读成
  npx stdio ＋ 写回三键且不含 `command`/`args`）· 既有 6 项 MCP 测试**全绿**（stdio 路径
  零回归，含注释保真/键序/备份）· `clippy --all-targets -D warnings` 0 问题 ·
  `fmt --check` 干净 · 前端 `tsc` 0 错误 · `oxlint` 0 警告 · `vitest` 376 passed。

### 2026-09-15 feat(market) · I1 收口：官方实验室 UI ＋ `isOfficial` 单源收敛 —— guan（AI 协作）

- **变更**：
  - 新增 `components/market/OfficialLab.tsx` ＋ `PluginHub` 第三子 Tab「官方实验室」。
    卡片呈现：条目名、前置说明、**互斥"将先移除谁"**、每步 `ordinal`/包名/已装标记/
    **激活方式（dsh 自动激活 vs 需写挂载行——v1 方案两向皆错之处，不让用户猜）**/
    钉版本/`versionNotice`；按钮驱动 `runPlan`，按 `onProgress` 显示「进行中 n/N」，
    失败时按钮变「继续剩余步骤」并从 `completedOps` 续装。中英 i18n 对称。
  - **`isOfficial` 启发式删除**：`MarketPluginCard` 原以 `owner` 含 deepseek 或 name 以
    `@deepseek-ai/` 开头猜"官方"，改为**必填 prop** `official: boolean`；社区 registry
    无 officialness 字段，故 `MarketplaceView` 明确传 `false` 并留注释。**官方性自此由
    策展目录独家拥有**（ADR-0020 §4 单源），启发式无回流缝。
  - **运行时版本改由后端本地检出**（`updates::detect_current_version`，离线读 `engines/`），
    `list_official_plugins` 不再要求前端传 `runtime_version`——前端没有廉价且权威的来源，
    传参只会引入漂移。**检出不到时不拼坏 spec**：退回裸包名并在该步 `versionNotice`
    **明示未钉版本**（裸包名按 latest 解析，正是 Agent Teams 三包错配的成因）。
- **⚠️ 一处有意偏离（已回报）**：目录安装**直接调 `install_plugin`、不经安装队列**——
  `queueStore.enqueue` 签名是 `(input) => void`（**不可 await**），而有序编排需要"逐步
  串行 + 每步知成败"。后果：**经官方实验室安装的条目不出现在「下载管理」面板**，进度改由
  `onProgress` 呈现（执行仍严格串行，不并发）。收敛方向已写进代码注释：给队列加可 await
  句柄（`enqueueAndWait`）后再改接；**不得**用轮询 store 状态糊上去。
- **影响**：**周知**。功能面至此**可见可点**：I1 的「决策 → 写入 → 自证 → 编排 → UI」五环齐备。
  一处用户可见的行为变化：社区市场卡片的「OFFICIAL」徽章**不再出现**（卡片无法验证官方性）。
- **凭据**：Rust `cargo test` **440 passed** · `clippy --all-targets -D warnings` 0 问题 ·
  `fmt --check` 干净 · IPC 58 三处一致 · 前端 `tsc` 0 错误 · `oxlint` 0 警告 0 错误（148 文件）·
  `vitest` **376 passed**（47 文件）· `AGENTS.md` 243/250。

### 2026-09-15 feat(frontend) · I1 安装编排纯逻辑（`lib/officialCatalog.ts` ＋ 7 项测试）—— guan（AI 协作）

- **变更**：
  - 新增 `frontend/src/lib/officialCatalog.ts`：把"点一个目录条目"展开成**有序操作序列**并
    串行执行——`planEntryRun`（同族冲突先 `remove` 走"替换"而非叠加；逐包 `install`；
    `insert_row` 步**紧跟**一步 `writeRow`；乱序输入按后端 `ordinal` **纠正**，因为顺序即语义：
    Agent Teams 的 Web 层装反了会在 Team 服务就位前挂载而失败）＋ `runPlan`（**严格串行**、
    失败即停、回报 `completedOps` 供续装、`onProgress` 供 UI 渲染）。
  - 新增 `frontend/src/__tests__/officialCatalog.test.ts`：**7 项**纯逻辑测试（IO 注入模式沿用
    `lib/shellSettings.ts` 先例；仓测只测纯逻辑，不引 RTL/jsdom）。
- **一处关键正确性判断**：安装结果 `PluginOpOutcome` 是 `{ok, detail}`——IPC 可以**成功解析
  但 `ok:false`**（pnpm 审批门、包不存在等）。编排把这种**软失败也当失败**并立即停止，
  否则会"界面报成功、实际没装上"，且会接着写一条指向**未安装包**的挂载行。已用专门用例钉住。
- **失败语义（有意）**：失败**不回滚**——已完成的步本处于一致状态（`insert` 行幂等、安装可重入），
  回滚反而会动到用户别的插件；改为回报 `completedOps` 让用户**续装**。
- **影响**：**周知**。至此 `apply_official_patch_row` 有了真正的调用方（此前是惰性代码）；
  I1 只剩**视觉层**（条目卡片/进度/续装按钮），逻辑与后端均已就位。
- **凭据**：前端 `vitest` **376 passed**（369 → 376，+7）· `tsc` 0 错误 · `oxlint`
  **0 警告 0 错误**（147 文件）· Rust 侧本轮未改动（沿用上一轮 439 passed 绿态）。

### 2026-09-15 feat(plugin) · I1 写后自证闸门：`--dump-config` 回读组合树 —— guan（AI 协作）

- **变更**（工作区未提交）：
  - `plugins.rs::verify_catalog_row`（纯函数）：判据**两条同时**成立——① dump 行表里存在
    `id == row_id`；② 该行 `pkg_name` 命中目标包名（防"行在、但指向别的包"的半对状态）。
  - **折进 `apply_official_patch_row`**：写行后立刻回读组合树，**命令返回成功即意味着
    "行已在组合树中"，而非仅"文件已写"**。dump-config 自身失败时**不谎报成功**，
    明确回"已写入但未能复核，请人工确认"。
  - **复用而非新建**：走既有 `plugin_rows_blocking`（`dsh --profile <名> --dump-config`，
    经 `profiles::run_toolchain_forward` → `lifecycle` seam；已支持 Local/WSL 读）。
    **未新增任何 spawn 路径**，spawn 闸门与既有超时/日志口径原样适用。
- **为什么必须有这一步**（本轮的核心动机）：v1 方案最危险的失败形态是**静默失效**——
  patch 写法不对时 DSH **退出码 0** 地把条目丢掉（本机实测：`- name:` 行只往 stderr 打
  一行 `patch: id is required for non-insert patches`）。只校验"文件写成功"**抓不到**它；
  只有**回读组合树**才算数。至此 I1 的写入路径具备"写 → 自证"闭环。
- **影响**：**周知**。`apply_official_patch_row` 的成功语义收紧（更严），前端若已按
  "成功即生效"理解则无需改动；若有代码把它的失败当"没写"处理，需按新文案调整
  （失败可能发生在**写之后**，文案已明确点出文件状态）。
- **凭据**：`cargo test` **439 passed**（438 → 439，+1）· `cargo clippy --all-targets
  -- -D warnings` 0 问题 · `cargo fmt --check` 干净 · IPC 58 三处一致 · 前端 `typecheck`
  0 错误 · `oxlint` 0 警告 · `vitest` 369 passed。

### 2026-09-15 feat(plugin) · I1 挂载行写入路径（`ensure_catalog_insert_row` ＋ IPC）—— guan（AI 协作）

- **变更**（宪法级文件改动含 `AGENTS.md`，依 §10 预告与归档；工作区未提交）：
  - `plugins.rs::ensure_catalog_insert_row` ＋ 纯变换 `apply_catalog_insert_row`：把一条
    `- insert: [{id, name}]` 并入 profile 的 `cordis.patch.yml`，一切写入经既有 `PatchFile`
    （覆写前备份 ＋ 原子替换 ＋ 未改条目原文保真）。**幂等**（同 `id` 已存在则零写入）；
    **只写 `{id, name}`、不写 `config`**（`config` 是整体替换语义，`docs/roadmap.md:31`，
    留给"先读后写"的配置流）；已有 `insert` 数组则**并入**该数组而非新建第二个 `insert` 条目。
  - IPC **`apply_official_patch_row`**（`COMMANDS` **57 → 58**）：四处名字面全同步；
    **WSL 客体档显式报错、不回落本地写**（写错 profile 比报错严重）。`AGENTS.md` §7 登记。
  - **职责边界（有意）**：本命令**只写行、不安装**——安装仍走既有 `install_plugin`
    （`dsh plugin add` 转发链）与前端**串行安装队列**。前端按目录行的 `steps` 有序执行
    「装 → 若该步 `insert_row` 则写行」，任一步失败停在一致态可续装。好处是复用既有队列的
    pnpm 审批门与错误分类，**不在壳内复制第二条安装链**。
- **并发/正确性要点（值得记一笔）**：`PatchFile` 的 `render()` 对未改动条目**回填原文片段**，
  故改动既有条目**必须**走 `for_each_entry_mut`（它负责把该条目原文置 `None`）；直接改
  `entries` 会让 `render()` 原样回填**旧文本**＝**静默不生效**。已用一项单测专门钉住这一点
  （若绕过 `for_each_entry_mut`，该测会因"找不到新行"而红）。
- **影响**：**周知**。I1 的**写入半**完成；前端目录 UI 未做，故用户侧仍看不到新界面。
- **凭据**：`cargo test` **438 passed**（436 → 438，+2）· `cargo clippy --all-targets
  -- -D warnings` 0 问题 · `cargo fmt --check` 干净 · IPC **58** 三处一致（`COMMANDS` /
  `capabilities` / AGENTS 声明）· 前端 `typecheck` 0 错误 · `oxlint` 0 警告 · `vitest` 369 passed ·
  `AGENTS.md` 243/250 行。

### 2026-09-15 feat(plugin) · 增量 I1 策展目录（决策层 ＋ IPC）—— guan（AI 协作）

- **变更**（宪法级文件改动含 `AGENTS.md`，依 §10 预告与归档；工作区未提交，无 commit hash）：
  - **新模块 `src-tauri/src/official_catalog.rs`**（**纯决策层**，不触网/不起子进程/不写文件）：
    `activation_for`（按 `dsh.bundle` 声明分支——bundle 类自动激活、壳**不写** patch；
    plain 类壳**必须**写 `insert` 行）；`row_id_for`（稳定可复算行 id，缺 id 的行永不可再
    patch）；`pinned_spec` / `version_skew_notice`（钉版本 ＋ latest 落后提示）；
    `exclusive_conflict` / `family_of_package`（浏览器三选一、桌面控制二选一，UI 走"替换"）；
    `install_plan` / `resolve_rows`（有序组合 ＋ 整行解析）。附 **7 项单测**。
  - **策展集数据** 8 条：auto-review / browser-use×3 / computer-use×2 / Agent Teams 两档
    （headless 与 Web，后者为**两步有序**）。
  - **IPC `list_official_plugins`**（`COMMANDS` **56 → 57**）：命令层采集已装态（读
    `profiles/<名>/package.json` ＋ 逐个依赖的 `package.json` 判 `dsh.bundle.patch`），
    返回可下发的目录行。四处名字面全同步（`ipc.rs` → `lib.rs` → `capabilities`
    `allow-list-official-plugins` → `tauri.ts` ＋ `types/ipc.ts` 新类型）。
  - `AGENTS.md` §7 登记该命令（57 条）。
- **影响**：**周知**。I1 的**语义核心**（v1 方案两向皆错的激活契约）现在是纯函数 ＋ 单测，
  "退出码 0 但插件没生效"这类静默失败**先在壳内红**。**尚未接线**：安装/写入路径与前端
  目录 UI 未做，故用户侧暂时看不到新界面——下一步是"把 `insert` 行经 `PatchFile` 落盘 ＋
  串行下发 ＋ 前端卡片"。
- **已知缺口（有意，非阻塞）**：`list_official_plugins` 在 **WSL 客体档显式报错**——
  客体侧需补 profile 读原语；**有意不回落本地读**，否则会把客体插件全误报成"未安装"
  （静默错数据比明确报错更糟）。已写入命令注释与实施计划 I1 残留项。
- **凭据**：`cargo test --lib` **436 passed**（428 → 436，+8）· `cargo clippy --all-targets
  -- -D warnings` 0 警告 · `cargo fmt --check` 干净 · IPC 四闸门与 `network_gate` 全绿 ·
  前端 `typecheck` 0 错误 · `oxlint` 0 警告 · `vitest` 369 passed。

### 2026-09-15 adr(0020,0022,0023) · 三份 ADR 评审通过 → I1/I3/I4 解冻 —— guan（AI 协作）

- **变更**（工作区未提交）：
  - **ADR-0020**（官方插件目录与安装激活契约）→ **已接受**。含 §1.4 无额外条件；I1 解冻。
  - **ADR-0022**（MCP 探测的网络/子进程面）→ **已接受**。维护者同时确认 §1.4 的**增量价值
    主张**（配置体检 / 失败归因 / 不耗 token），故不退回方案 E；I3 解冻。
  - **ADR-0023**（SSH 远程工作区范围与边界）→ **已接受**。维护者确认 §3 **方案 C 的否决**，
    即接受「Web 视图不随 provider 替换而变为远端感知」这一范围裁剪；I4 解冻。
  - 同步：`docs/adr/README.md` 行（状态以各 ADR 头部为准，**不复制**）；实施计划 §0 v2.2、
    §6.1 状态列与 §4.1 清单。
- **影响**：**周知**。批次状态由「1 在办（I2）＋ 3 待评审 ＋ 1 暂缓」变为
  **「4 在办（I1/I2/I3/I4）＋ 1 暂缓（I5）」**。I4 依赖 I1 的 patch 写入内核（`PatchFile` 复用），
  故实施顺序为 **I1 → I3 → I4**（I3 与 I1 无依赖，可并行）。
- **顺带更正**：实施计划 §6.3 原写「I2 承接并收口 roadmap §4.6」属**过度声明**——I2 只落地
  归档侧，`工作区增删管理` 子项**仍开放**（壳内无 workspace CRUD IPC，上游却有
  `create`/`rename`/`delete`/`insertBefore`/`insertSessionBefore`）。已在 `docs/roadmap.md`
  §4.6 **追加**2026-09-15 落地记录（不删 09-11 记录）并改正计划。
- **凭据**：ADR 头部状态行；`docs/roadmap.md` diff = `6 insertions, 0 deletions`（纯追加）；
  本批为纯文档改动，未跑测试。

### 2026-09-15 fix(boot,sessions,plugins) · **回环鉴权失效**：`/api` 需签名 Cookie ＋ 顺带修复既有功能静默失效 —— guan（AI 协作）

- **发现路径**：I2 落地后按 §5/§8.3 做**实机对账**（不只跑单测）——对真实运行中的工作台
  （`127.0.0.1:53805`）打回环端点，得 **`401 unauthorized`**。查上游源码定位：
  `rpc-host.ts:97-99` 的 `/api` 是**两道**栅栏——`isTrustedApiRequest`（Host 回环/可信，
  失败 **403**）之后还有 `browserAuth.isAuthenticated`（失败 **401**），后者要启动期由
  launch token 兑换出的**签名 Cookie**（`browser-auth.ts:285-300`）。
- **实机复现（抛离式 `DSH_HOME`，未触碰用户数据）**：`GET /?token=…` → `303` +
  `set-cookie: dsh-auth-…`（`Max-Age=2592000`、`HttpOnly`、`SameSite=Strict`）→ 同请求带
  Cookie 后 `/api/pluginInventory/list` → **`200`**；`/api/workspace/unarchiveSession`
  （不存在的 id，上游保证**无写早退**）→ **`200` + `result.ok=true`**。
- **变更**：
  - `ShellState` 新增 **`workbench_cookie`**（仅内存；boot 期 `authenticate_workbench_session`
    兑换到 Cookie 时留存 `name=value`，**不落盘、不打日志**——AGENTS §4.3）。
  - `sessions.rs::request_unarchive` 与 `plugins.rs::fetch_runtime_snapshot` 均改为**附
    `Cookie` 头**（`cookie: Option<&str>`）；两个命令调用点同步传值。
  - `authenticate_workbench_session` 签名由 `(&WebviewWindow, &Url)` 改为 `(&ShellState, &Url)`
    以便留存 Cookie（唯一调用点已同步；**既有流程与错误回退路径一律未变**）。
  - `AGENTS.md` §7 补"两条回环用途必须附 `/api` 会话 Cookie"；台账**更正复现点 11**
    的失效断言并新增 §三 更正记录。
- **影响**：⚠️ **周知 + 需关注**。① I2（取消归档）由此**真正可用**；② **顺带修复既有功能**：
  `get_plugin_runtime`（插件运行态快照，2026-08-29 落地）此前对带栅栏的 dsh **一直恒 401
  静默失效**——属"记在册的上游姿态随版本漂移"的教科书案例；③ 壳内存里多了一份 `/api`
  会话 Cookie，**仅内存、不外发、不进日志**。
- **残余风险（须真机复核）**：WSL 客体模式下 `workbench_url` 是否同样携带 `token=` 未验证
  ——若不带，客体档回环仍 401（走 `docs/executor.md` 验证清单）。
- **凭据**：`cargo test` **428 passed**（含新增信封/解析 6 项）· `cargo clippy --all-targets
  -- -D warnings` 0 警告 · `cargo fmt --check` 干净 · IPC 四闸门 + `network_gate` 15 项全绿 ·
  前端 typecheck/lint/vitest（369）全绿。

### 2026-09-15 feat(sessions) · 增量 I2「取消归档」落地（ADR-0021 路线 A）—— guan（AI 协作）

- **变更**（宪法级文件改动含 `AGENTS.md`，依 §10 预告与归档；工作区未提交，无 commit hash）：
  - **后端**：`src-tauri/src/sessions.rs` 新增取消归档三件套——`unarchive_request_body`
    （纯函数，构造 typert 回环信封）、`parse_unarchive_response`（纯函数，解析
    `result.ok` 两层 + `value.archivedSessionIds`）、`request_unarchive`（唯一网络原语，
    `POST /api/workspace/unarchiveSession`，2s 超时）；`src-tauri/src/commands/session.rs`
    新增 `unarchive_session` 命令。**刻意不写** `$DSH_HOME/storages/workspace.json`
    （内存状态投影，运行中 Host 会整体覆盖）。
  - **IPC 四处名字面 + 形状面**：`ipc.rs::COMMANDS` 登记（**55 → 56 条**）→ `lib.rs`
    `generate_handler!` → `capabilities/default.json` `allow-unarchive-session` →
    `frontend/src/lib/tauri.ts` 类型化封装（无新结构过 IPC，故 `ipc-shapes.json` 不动）。
  - **网络闸门**：`network_gate.rs` 新增**条目级** `Kind::Exempt` 行（`sessions.rs::request_unarchive`）
    ——刻意不整文件豁免，防会话域后续触网偷跑（沿用 plugins.rs 2026-09-11 收窄口径）。
  - **共享信封**：`plugins.rs` 抽出 `loopback_request_body(method, args)`，`runtime_request_body`
    改为其无参封装——使回环信封壳内**只有一处**构造。
  - **复现台账**：`dsh-behavior-ledger.md` 新增复现点 16（取消归档远程契约，含上游
    `file:line` 锚点与升级复核项）＋ §三 复核记录。
  - **前端**：`SessionManager` 归档档位新增「取消归档」按钮（仅归档行可见）＋
    `zh-CN`/`en-US` 对称文案；无活跃 Host 时原样透出 Rust 侧可读错误（提示先启动 DSH）。
- **影响**：**仅周知**，无需他人动作。用户在壳内会话管理器即可取消归档，不必切到 Web
  设置页；该动作经 Host RPC 完成（幂等），**不会**绕过运行时直接改磁盘状态。
- **凭据**：Rust `cargo test` **428 passed**（+6 新单测：信封键集恰为 `["request"]`、
  响应正例/业务错误/形状漂移三反例、端点常量、信封抽取回归）· `cargo clippy --all-targets
  -- -D warnings` 0 警告 · `cargo fmt --check` 干净 · IPC 四个闸门 + `network_gate` 15 项
  全绿（含 `production_network_primitives_are_registered`、`exemptions_are_live`、
  `item_scoped_exemption_does_not_leak_to_the_rest_of_the_file`）· 前端 `typecheck` 0
  错误 · `oxlint` 0 警告 0 错误 · `vitest` 46 文件 **369 passed**。`AGENTS.md` 237 行
  （预算 250）。
- **实现取舍留痕（对 ADR-0021 的一处有意简化）**：ADR 行动项原写 `unarchive_session` 需
  "world 分派"，实施时**未加分派**——路线 A 不碰文件路径，归档状态由**当前活跃 Host**
  持有，其地址在 Local/WSL 两模式下都记在 `ShellState.workbench_url`（`boot.rs:384`），
  故无宿主/客体可分。已写入代码注释与台账 §三。

### 2026-09-15 adr(0021,0024) · **补记**：维护者对 ADR-0021 / 0024 的裁定 —— guan（AI 协作）

- **补记原委**：上方同日条目载明「5 份 ADR 均为草案」，该表述在数小时后**部分失效**——维护者
  即时就两项作出裁定。按 §二「漏记不补改旧条目——另发一条『补记』并注明原委」，另立本条。
- **变更**（工作区未提交）：
  - **ADR-0021 → 已接受**。维护者裁定「认可」，即认可「`SessionManager` 是独立 Tauri 窗口、
    够不到 Web 设置页、故壳内应自建入口」这一理由，**据此采纳路线 A**（复用 Host RPC
    `workspace.unarchiveSession`，不触碰 `$DSH_HOME/storages/workspace.json`）；原设的翻转
    条件（退回方案 C「不新增，指向 Web 设置页」）**未触发**。**I2 增量由此解锁。**
  - **ADR-0024 → 草案／暂缓实施**。维护者裁定「**暂时不做**」：方案 A（转交版）与方案 B
    （完整版）**均不启动**，I5 移出当前批次；两项平台扩展（`tauri-plugin-global-shortcut`
    新依赖、第四个窗口）**未获批准**。**本 ADR 未被否决**；`docs/roadmap.md:331` 的边界
    问题**留白未裁决**，故**不改动** §5 不做清单（既不承认越线、也不授权动工）。
  - **两处 ADR 头部状态、§4 结论、§6 复审条件、`docs/adr/README.md` 索引行、以及
    `docs/plans/official-plugins-and-capabilities-plan.md`（→ v2.1）**均已同步。
- **影响**：**仅周知**，无需他人动作。交付批次由 **5 增量收敛为 4 增量 ＋ 1 项暂缓**：
  I1（策展目录）、**I2（取消归档，已解锁）**、I3（MCP 探测）、I4（SSH 向导）继续；
  **I5 冻结**。任何人不得在 I5 暂缓期间引入 `tauri-plugin-global-shortcut` 或第四个窗口。
  I5 的解冻触发只有一个：**维护者重新提出该需求**（ADR-0024 §6）。
- **凭据**：`docs/adr/0021-archived-session-unarchive-route.md`（状态行「已接受」＋裁定引文）；
  `docs/adr/0024-desktop-quick-task-runner.md`（状态行、§4 三条边界、§5 行动项、§6 复审条件）；
  `docs/adr/README.md` 索引行；实施计划 §0 v2.1 行。本仓库未跑测试（纯文档改动）。

### 2026-09-15 docs(agents) · AGENTS §9 ADR 索引迁出至 `docs/adr/README.md` ＋ 新立 ADR-0020~0024 —— guan（AI 协作）

- **变更**（宪法级文件改动，依 AGENTS §10 预告与归档；工作区未提交，无 commit hash）：
  - **`AGENTS.md`**：§9 的 ADR 索引表（20 行，0001–0018）整体迁出，正文只保留「必须先立
    ADR 再动代码」规则 ＋ 一行指针。**250 → 233 行**（§9 由 26 行降至 9 行）。
  - **`docs/adr/README.md`（新建）**：承接索引，既有 18 行**逐字迁移**，补登漏失的
    ADR-0019 行，并新增 0020–0024 五行。含「新增 ADR 须同提交内补索引行」的维护约定。
  - **新立 5 份 ADR（草案，待维护者评审）**：`0020` 官方插件目录与安装激活契约 ·
    `0021` 已归档会话取消归档路线 · `0022` MCP 探测的网络/子进程面归属 ·
    `0023` SSH 远程工作区范围与边界 · `0024` 桌面任务快跑器范围裁定。
  - **配套文档**：`docs/plans/official-plugins-and-capabilities-plan.md`（修订版 v2，修正
    7 项阻断缺陷并拆为 5 个独立增量）＋ `docs/plans/official-plugins-and-capabilities-plan-review.md`（审核报告）。
- **冲突与依据**：迁出原因是**规范冲突**而非偏好——索引表随 ADR 数量线性增长，而 §11.4 对
  本文件设「全文 ≤ 250 行」固定预算；改前恰为 250 行，**连补一行 ADR-0019 索引都会越界**，
  该冲突即 ADR-0019 当日提交（`d2050b8`）却从未进索引的成因。按 AGENTS §8.7「停手提出修订
  建议、不得变形绕行」上报，经维护者裁定采纳「索引迁出」方案，并以 §11.4 的「回收」机制落地。
- **影响**：**仅周知**，无需他人动作。索引查法变化：查 ADR 编号与一行结论改看
  `docs/adr/README.md`；§9 仍承载 ADR 规则。5 份 ADR 均为**草案**，其 §5 行动项里的
  AGENTS §6/§7 登记、`network_gate.rs` EXEMPTIONS 行、台账行与 roadmap 收口**一律待评审通过后**
  于实现期执行，本次未提前登记。
- **凭据**：行数核算 250 → 233（`wc -l`）；索引完整性校验：`docs/adr/` 下 24 份 ADR ↔
  README 24 行，链接逐一可解析、无断链、无漏行；`AGENTS.md:131`（ADR-0020 引用的 `PatchFile` 条）
  位于 §6、在本次改动之前，行号未漂移；本仓库未跑测试（纯文档改动）。

### 2026-09-12 feat(boot): 首次启动工作台与 Windows 运行环境前置选择交互优化 —— guan（AI 协作）

- **变更**：
  - 后端：`src-tauri/src/lib.rs` 修复 Windows 首启未设默认模式时静默走本地启动的违约缺陷，改为挂起启动并导航至 `/mode`；`src-tauri/src/commands/boot.rs` 在 `get_boot_status` 注入 `needs_mode_selection` 标志与纯函数校验测试（420 passed）；
  - 前端：`frontend/src/pages/BootMode.tsx` 重构为直接 IPC 调用 `api.chooseMode` 并包含加载与错误态，移除过时的 query 中转；`frontend/src/pages/BootSelector.tsx` 交互重塑（卡片快捷「设为默认」、进入默认工作台区分、现代 Switch 偏好设置栏）；
  - 国际化与门禁：`zh-CN.ts` 与 `en-US.ts` 对称增补多语言文案，新增 `bootModeCopy.test.ts` 门禁守卫。
- **影响**：Windows 首次启动明确先选本地/WSL模式并可选设为默认，非 Windows 维持零 WSL 感知极速启动；工作台选择中心设默认与启动体验更加直观。
- **凭据**：`cargo test` 420 passed · `cargo clippy --all-targets -- -D warnings` 0 警告 · `cargo fmt --check` 干净 · 前端 vitest 46 文件 369 passed · `tsc` 0 错误 · `oxlint` 0 警告 0 错误。

### 2026-09-12 fix(engines)：为 Windows 模块代理模式补齐浏览器 client bundles 与 client 声明 (ADR-0019) —— guan（AI 协作）

- **变更**：新增 `src-tauri/src/dsh-client-proxies.mjs`；更新 `src-tauri/src/engines.rs`（落位、bootstrap 预热与单测断言）；新增架构决策文档 `docs/adr/0019-dsh-client-module-proxies-on-windows.md`。
- **原因**：ADR-0018 启用模块代理（proxy mode）规避 Windows 符号链接特权后，dsh 后端可启动，但浏览器打开 Web 工作台时报错 `Failed to load plugins: client-modules: HTML did not preload @deepseek-ai/dsh-client-modules/client.js` 导致白屏。根因为 dsh 的 `ensureModuleProxy` 丢弃了包原本的 `dsh.client` 与 `exports["./client"]`，纯 Node 模式下前端扫描不到客户端模块声明。
- **方案**：
  1. 编写独立 Node 模块 `dsh-client-proxies.mjs`，在 bootstrap 执行 `runCli()` 前调用 `augmentClientProxies`，将各包导出的浏览器 bundle（`dsh-client-bundle.js` 及 `.map`）复制到 proxy 目录，并增量写回 `exports["./client"]` 与 `dsh.client`；
  2. 严格维护 dsh 契约不变量（逐字节保留 `targets` 与 `version`，防止触发上游强制重建）；
  3. `write_dsh_client_proxies` 保证无变化零写入，并在全新安装与就绪启动路径（`refresh_dsh_layout_if_project_local`）中自动刷新。
- **影响**：仅周知，POSIX 侧维持符号链接原样，Windows 侧 Web 工作台恢复完整加载 52+ 前端插件。
- **凭据**：浏览器 CDP 实测复现红/绿闭环 · 真实 dsh 启动实测（`real_dsh_boots_in_proxy_mode`：0 软链 / 1184 entry 代理 / 55 client-bundle，进程存活） · `cargo test` **418 passed**（416 → +2） · `cargo fmt --check` 干净 · `clippy --all-targets -D warnings` 干净 · `node --check` 干净。

### 2026-09-12 fix(market)：对齐 dsh-market NPM catalog 镜像链与 CDN 扩容 —— guan（AI 起草）

- **变更**：`src-tauri/src/updates.rs`，`fetch_market_registry` 升级分发链路。
- **原因**：上游 `awesome-dsh-plugin.com/plugins.json` 插件增至 3400+、体积达 3.11MB（突破此前硬编码的 3MB 上限），回退的 `raw.githubusercontent.com` 系 Pages 构建产物恒为 404，导致控制中心插件市场目录加载失败。
- **对齐 dsh-market 方案 B**：
  1. 优先从 npm 镜像链（`registry_chain()`：npmmirror → npmjs）拉取官方分发包 `dsh-plugin-catalog` 的 tarball，解压 `package/plugins.json`（gzip 压缩包仅 ~800KB，国内秒级直达，抗封锁且省流）；
  2. NPM 镜像全失效时回落官方 CDN `awesome-dsh-plugin.com`，体积上限从 3MB 提高至 16MB；
  3. 彻底移除无效的 `raw.githubusercontent.com` 回退路径。
- **影响**：仅周知，对外 IPC 契约 `fetch_market_registry` 接口与返回值完全兼容。
- **凭据**：`cargo test` **416 passed**（414 → +2）· `real_fetch_market_registry` 联网实测 0.70s 通过 · `cargo fmt --check` 干净 · `clippy --all-targets -D warnings` 干净 · 前端 `typecheck` 0 错。

### 2026-09-12 发版 v1.2.4 · 让 Windows 修复作用于已安装用户（同形缺口的第二层）—— guan（AI 协作）

- **性质**：用户真机复验 v1.2.3 **仍报同一 EPERM** 后的修复与发版结果回填。
- **决定性证据（定位本次缺口）**：用户栈为 `…/@deepseek-ai/dsh/lib/bin.js:168:23` ——
  那是普通 node 直跑的顶层 `if (import.meta.main) await runCli()`；**经 bootstrap 时该行不执行**
  且栈中会出现 `dsh-boot.mjs` ⇒ **现场跑的不是新 shim**。
- **两层同形缺口**（都是「**修复没落在真正会走的那条路径上**」）：
  1. `ensure_dsh_runtime_layout`（写 bootstrap + 重写 shim）**只**挂在
     `install_dsh_project_local` ⇒ dsh 已就绪（= **所有升级用户**）从不刷新 ⇒
     **ADR-0018 的修复只对全新安装生效、对存量安装完全无效**（Lead 定位）；
  2. 本地模式有**就绪快速路径根本不进 `bootstrap()`**
     （`executor.rs:248 probe_engine_if_ready` → `:256 resolve_launch_engine_ready`）⇒
     在 bootstrap 内加刷新**对用户机器依然无效**（**rust-core 发现并主动报为未完成**，
     而非交付一个"测试全绿但到不了用户路径"的改动）。
- **修法（单点收口，Lead 裁定方案 B）**：`resolve.rs::engine_launch_spec` 开头调用新增的
  `engines::refresh_dsh_layout_if_project_local`。**两条启动路径**
  （`resolve_launch:377` 与 `resolve_launch_engine_ready:479`）**都收敛于此** ⇒ 一处覆盖两条；
  bootstrap 内的重复实现（④′）已删除，**无第二份实现**。
  - **顺序不可调换**：校正位于 `engine_dsh_bin()` 与 `engine_no_open_supported()` 之前 ——
    后者会**真的执行一次启动器**探测 `--no-open`，晚于刷新就会用旧 shim 探测。
  - **护栏**：仅当 `dsh_entry_js(...).is_file()` 才刷新；旧全局布局用户（dsh 在
    `global/v11/…`、`bin/dsh` 是 pnpm shim）的启动器**逐字不变**，不会被指向不存在的文件。
- **回归测试（全部走 resolve 真 seam，不走 bootstrap；+4 用例、零删除）**：
  T4 `ready_launch_path_refreshes_stale_shim`（走 `resolve_launch_engine_ready`，
  **前置断言 `probe_engine_if_ready(...).is_some()`**）；T-order 把「**顺序**」钉成独立一维；
  T2 护栏；T3 幂等。
- **独立验证**（`docs/team/ADR0018-独立验证-升级路径.md`，qa-verify）：判定**通过**。
  ① 单点确为单点（1 定义 + 1 调用点，④′ 无残留）；② **顺序独立**——把校正移到探测之后 ⇒
  **只有 T-order 红而 T4 仍绿**；③ T4 确走用户真实就绪路径且前置断言就绪态；
  ④ 护栏**反向变异后必红**（有牙齿）。另：其裁定的「T1 与 T4 关系」比作者自述更强 ——
  **T1 的 Windows 专属意图并未被取代而是保留**，T4 承接的是平台无关部分
  （建议措辞改为「承接 + 保留」，已采纳）。
- **闸门**：版本号四处一致 ✅ · `extract --strict` ✅ · cargo fmt `0` · clippy 宿主 `0` ·
  **clippy win-gnu `0`** · `cargo test` **414 passed / 0 failed / 4 ignored** ·
  前端 typecheck `0` / lint `0` / test **45 files · 365 passed** · scripts **17 OK**。
- **CI 结果（tag `v1.2.4`，全绿）**：`build` run **`34677405713`** **6/6**（4 build leg +
  `release`）；`boot-smoke` run **`34677405698`** **4/4**（含 `engine-bootstrap (windows-latest)`）；
  **17 个资产**。
- **结构性结论（本轮两次翻车的公共形状，建议列为通用检查项）**：
  **「修复必须落在真正会走的那条路径上」**；并附一条实现纪律 ——
  **校正类逻辑必须既挂在创建点、也挂在每次使用点**（`ensure_profile_build_policy_*` 的
  **双点**是正确范式；只挂创建点 ⇒ 修复对存量无效）。它与已登记的
  「判据必须与所判之物解耦」（测试纪律）并列，构成"本轮两次翻车"的两条同源教训。
- **⚠️ 边界**：**Windows 真机仍待复验**；macOS 只能证明「校正发生在就绪路径上 + 平台形态
  函数正确」，**不能证明「Windows 不再 EPERM」这一因果**；真实升级流程未端到端跑（用夹具）；
  「中间态」（`dsh-runtime` 存在但入口损坏）未构造验证。
- **凭据**：tag `v1.2.4` → commit **`bca9817`**；修复 `09e7a59`；CI `34677405713` / `34677405698`。

### 2026-09-12 发版 v1.2.3 · Windows 本地模式启动修复（ADR-0018）· 结果回填 —— guan（AI 协作）

- **性质**：ADR-0018 的实施完成 + 发版结果回填（承接同档「架构变更 ADR-0017」与
  「v1.2.2 结果」两条）。
- **真机驱动的第二段**：用户实测证明 **ADR-0017 成功**（报错路径已到
  `engines\dsh-runtime\node_modules\@deepseek-ai\dsh` ⇒ 引擎已装上），但紧接着暴露
  **同一根因的第二处显形**：dsh **启动时**在 `$DSH_HOME/profiles/node_modules`
  建约 481 个符号链接 ⇒ 普通账户 `EPERM: symlink` 退出。
- **定位（Lead + rust-core 各自实测）**：dsh 用 `isPackagedExecutable()`（判据 =
  `process.pkg !== undefined`，全文件**仅 1 处调用**）选 fallback 链接方式 ——
  普通 node ⇒ `ensureSymlink`（软链）；打包体 ⇒ `ensureModuleProxy`（**目录代理，零软链**）。
  **无任何 env/config 开关；上游两源最新仍为 0.1.5-rc.2**。
- **决策（ADR-0018）**：Windows 由壳自写 bootstrap `<engines>/bin/dsh-boot.mjs`
  （置 `process.pkg` → import 入口 → **显式 `await mod.runCli()`**）选中 dsh 自带的
  代理分支；**POSIX shim 逐字不动**（那里无特权问题）。否决「预建 junction」（需复刻
  dsh 依赖闭包 = 重复实现）。**仍拒绝自动提权**（理由同 ADR-0017）。
- **核心判据（实现无关，本机实测两次）**：同一真实 dsh、各自独立临时 `DSH_HOME`、
  唯一变量 = `process.pkg` ⇒ 普通 node **481 软链 / 0 代理**；代理模式 **0 软链 /
  1184 代理**；两者均可 boot。已落成 `#[ignore]` 测试（用**生产函数**落位）防回归。
- **独立验证**（`docs/team/ADR0018-独立验证.md`，qa-verify）：判定**通过**；核心判据由其
  **独立双路径重做**（跑作者的 ignored 测试 + 自建手工 spike，并核过不是「跳过式通过」）；
  **绊线经 3 回合构造场景实测**（含两个对照：`EACCES` 不吞错、`EPERM+open` 不误触发）；
  POSIX 分叉面 sha256 **逐字未变**；并**证实「注释满足断言」那个坑已堵住**。
  **隔离达标**：用户真实 `~/.dsh/profiles/node_modules` mtime 与条目数验证前后未变。
- **qa-verify 自创变异 M4 → 收口一轮（task-61）**：`DSH_BOOTSTRAP_NAME` 与 Windows shim 里
  硬编码字面量**无任何绑定** ⇒ 只改常量名时 **408 条测试全绿**，而 Windows 会指向不存在的
  文件且**静默**。修法取**构造上单一来源**（shim 由常量生成，非退化为断言绑定），并新增
  两条**端到端**断言（从 shim 文本解析目标名 → 核对文件确实落位；失败打印实际落盘列表）。
  **Lead 独立复现**确认抓得住（红于 `engines.rs:2542`）。
- **闸门**：版本号四处一致 ✅ · `extract --strict` ✅ · cargo fmt `0` · clippy 宿主 `0` ·
  **clippy win-gnu `0`** · `cargo test` **410 passed / 0 failed / 4 ignored**（基线 405）·
  前端 typecheck `0` / lint `0` / test **45 files · 365 passed** · scripts **17 OK**。
- **CI 结果（tag `v1.2.3`，全绿）**：`build` run **`34673689775`** **6/6**（4 build leg +
  `release`）；`boot-smoke` run **`34673689810`** **4/4**（含 `engine-bootstrap (windows-latest)`）；
  **17 个资产**；`latest.json` **7 平台**；**双 macOS 公证 Accepted ×2**。
- **⚠️ 必须同时知悉的边界**：
  1. **Windows 真机仍待复验**（本轮证据为机制级修复 + 单元/契约级验证）；
  2. **平台机制不同**：macOS 的「0 软链」只证明「代理模式确实零软链」这一**结果**，
     **不能替代** Windows「不再 EPERM」这一**因果**（qa-verify 明确写入报告）；
  3. **依赖 dsh 未公开内部判据 `process.pkg`** —— 已登记技术债，配「失效即响亮报错」的
     绊线（仅覆盖本启动路径），并已备上游缺陷报告 `docs/team/ADR0018-上游缺陷报告草稿.md`。
- **凭据**：tag `v1.2.3` → commit **`a0d7001`**；实施 `c38ab2c`；ADR `a70e5c4`；
  前段 `0e2be2c`（ADR-0017）/ `9d81393`（v1.2.2）；CI `34673689775` / `34673689810`。

### 2026-09-12 发版 v1.2.2 · Windows 本地模式根因修复（ADR-0017 实施完成）· 结果回填 —— guan（AI 协作）

- **性质**：承接同档「架构变更 ADR-0017」条目的**实施完成追加**（按既定「分两次写，不合并」）。
  问题 1.1 的状态由「根因已定案并进入实施」推进为「**实施完成 + 验证通过 + 已发版**」，
  **真机复验仍待用户执行**。
- **版本**：`v1.2.1` → **`v1.2.2`**（patch）。发版日志 `docs/RELEASE_NOTES.md` 顶部
  `## [v1.2.2] - 2026-09-12`；`extract-release-notes.py v1.2.2 --strict` 通过并作为 Release 正文。
- **实施**（`0e2be2c`）：`install_dsh_global` → `install_dsh_project_local`（`<engines>/dsh-runtime/`
  内非全局 `pnpm add`）；`ensure_dsh_runtime_layout` 幂等三件（package.json / `nodeLinker: hoisted`
  / 壳自写 shim）；自写 `bin/dsh`（POSIX 0o755）与 `bin/dsh.cmd`（Windows，`%~dp0` 相对、CRLF、纯 ASCII）；
  `run_engine_pnpm_streaming` 的 cwd 参数化（node 仍传 `pnpm_home`，与改前逐字等价 ⇒ 只改 dsh）；
  WSL 客体 `executor.rs` 哈希未变（分叉面未扩大）；复用 `build_policy::ensure_engine_linker`（零新键处理）。
- **旧布局兼容**（本改动最可能伤存量用户处）：`readiness_gaps` 签名**不含路径/布局信息**
  ⇒ 「按布局区分」在类型层面无从表达；另有行为级守门测试（加 legacy `global/` 符号链接树前后
  `EngineStatus` 逐字段相等）；无任何针对 `global/` 的删除逻辑。
- **独立验证**（`docs/team/ADR0017-独立验证.md`，qa-verify）：判定**通过**；4 次变异证伪均**点名红断言**
  （M1a `engines.rs:1793` / M1b `updates.rs:1259` / M2 `engines.rs:2095` / M3 `engines.rs:1831`）；
  反向证明 node 路径未动；shim 经**实跑**验证（找得到 / 0o755 / 非链接 / 幂等）。
- **闸门**（tag 前提交态实测）：版本号四处一致 ✅ · `extract --strict` ✅ · fmt `0` ·
  clippy 宿主 `0` · **clippy win-gnu `0`** · `cargo test` **405 passed / 0 failed / 3 ignored**
  （基线 394，+11）· 前端 typecheck `0` / lint `0` / test **45 files · 365 passed** · scripts **17 OK**。
- **CI 结果（tag `v1.2.2`，全绿）**：`build` run **`34666441862`** **6/6**（4 build leg + `release`，
  coverage 按设计 tag 上跳过）；`boot-smoke` run **`34666441860`** **4/4**（含
  `engine-bootstrap (windows-latest)`）；**17 个资产**；`latest.json` **7 平台**；
  **双 macOS 公证均 Accepted**（arm64 `21800a4d…` / Intel `0e63fcfc…`，`source=Notarized Developer ID`）。
- **⚠️ 必须与发版同时知悉的边界**：
  1. **Windows 真机行为待复验**（shim 的 `.cmd` 实际执行、真机 dsh 首启安装）——
     本轮证据链为「机制级修复 + 单元/契约级验证」，**不得**把 CI 全绿当作该类验证证据。
  2. **真机风险点已从「全局链接」转移到「原生构建脚本」**：dsh 依赖 koffi / node-pty / protobufjs /
     @google/genai / dsh-subprocess-local 的**原生构建**；旧路径在 Windows 上**从未越过全局链接步**，
     故这些步骤**从未在任何 Windows 机器上跑过**。**本轮修复不构成该风险的证据。**
     定点排查脚本：`scripts/probe-windows-dsh-install-full.ps1`（零安装、只写 `%TEMP%`）。
- **凭据**：tag `v1.2.2` → commit **`9d81393`**；实施 `0e2be2c`；落档 `b6085b1`；ADR `52b876e`；
  CI `34666441862` / `34666441860`。

### 2026-09-11 架构变更 ADR-0017 · dsh 改为 project 内安装（含**宪法级改动**）—— guan（AI 协作）

- **性质**：**宪法级改动告知**（AGENTS §10「宪法级改动须落档 broadcasts」）+ 架构决策落档。
  本条目为 `AGENTS.md`（提交 **`52b876e`**）与 **ADR-0017** 的正式知会载体。

#### 一、宪法级改动（`AGENTS.md`，全文 **250** 行 = 上限，全部在预算内）

| # | 位置 | 改动 | 判据 |
|:--|:---|:---|:---|
| 1 | **§9 索引表 `:226`** | 新增 [ADR-0017](docs/adr/0017-dsh-project-local-install.md) 行：「dsh 改为 project 内安装 + 自建 shim：结构性绕开 pnpm 全局 hash 符号链接，Windows 普通账户免提权」 | §9 是 ADR 唯一索引（§9 要求先立后改） |
| 2 | **§7 `:172`** | 「新增 IPC 三处同步」块 **3 行 → 1 行** | 按 **§11.4「已有测试 CI 兜底」回收**：三处同步已由 build.rs 生成 + `ipc.rs` gate_tests 全机器兜底，**不必再靠人记**（细节从宪法移出，非删除约束） |
| 3 | **§7 `:183`** | 引擎引导条：原「壳内置 pnpm12 经 `runtime set node` / `pnpm add -g` 下载 node 与 dsh」→ 校正为「经 `runtime set node` 下载 node、经 `pnpm add`（**project 内安装，非 `-g`**：Windows 免符号链接特权，ADR-0017）下载 dsh，**WSL 客体仍同源 `add -g`**」 | **这是"宪法里写着的安装方式变了"**——原文已随 ADR-0017 失真，不改即违 §11.3（禁双源） |

**行数证据**（两种口径并列，遵循团队教训 M5「报预算类数字须同时给命令」）：

```bash
wc -l AGENTS.md                                                   # 250（= 上限 250）✓
awk '/^## 7\./{f=1;next} /^## 8\./{f=0} f' AGENTS.md | wc -l      # 37 ← 规范口径（真实节内行数）
awk '/^## 9\./{f=1;next} /^## 10\./{f=0} f' AGENTS.md | wc -l     # 24 ← 规范口径
awk '/^## 7\./,/^## 8\./' AGENTS.md | wc -l                       # 39（含两端标题）；去首行 = 38
awk '/^## 9\./,/^## 10\./' AGENTS.md | wc -l                      # 26（含两端标题）；去首行 = 25
```

⇒ 规范口径 **§7 = 37 / §9 = 24**；「含下节标题」口径得 38 / 25（差 1 行，正是 M5 记录过的口径差）。
**两种口径均在 §11.4 上限（单节 ≤40）之内。**

#### 二、ADR-0017 决策要点

- **采纳方案 B**：dsh 由「pnpm 全局包」改为「**壳自管 project 依赖 + 壳自写 shim**」——
  在 `<engines>/dsh-runtime/` 内 `pnpm add`（沿用 `nodeLinker: hoisted`），
  壳自写 `bin/dsh`（POSIX）与 `bin/dsh.cmd`（纯文本，**不是链接**）；
  `find_engine_tool` 既有查找序（`dsh.exe` → `dsh.cmd` → `dsh`）**无需改动**。
- **为什么否决自动提权（方案 C）**：壳的 WebView **承载 dsh 工作台并加载用户第三方插件 JS**；
  壳一旦提权，WebView、dsh 及其 spawn 的一切都成管理员权限 ⇒ **信任面从「装一个包」放大到
  「整个壳 + 所有插件」**；而引擎安装**只在首启一次**需要该能力——严重不成比例。
- **亦不采纳 A（npm 全局回退）**：ADR-0005（2026-08-28 补录）已明写装 dsh 本体**不再回退 npm**，
  回退会引入**第二条安装路径**（双实现），且该函数已随之删除需重建。
- **关联**：修订 ADR-0010 的**安装形态**一节（「引擎=壳资产」「首启自补齐」结论不变）；
  不冲突 ADR-0005（对 pnpm 自身仍适用）与 ADR-0009 口径 2（仍是 pnpm）；**WSL 客体不改**。
- **迁移（不破坏存量）**：旧 `global/v11/…` 布局**继续可用**（`readiness_gaps` 只判「在位且版本匹配」，
  不判布局）⇒ 老用户不被强制重装；旧 `global/` **不主动删除**。

#### 三、Windows 真机探针结论（方案 B 两平台均成立）

**探针**：`scripts/probe-windows-engine-install.ps1`（**只读**：只在 `%TEMP%` 建临时目录、跑完自删；
不碰 `engines/`、不写注册表、不装全局包）。**普通账户 + 开发者模式关闭**下实测：

| # | 判据 | 结果 |
|:--|:---|:---|
| 1 | 非全局 `pnpm add` | **exit 0** |
| 2 | 是否创建 `global\` | **未创建** —— 完全不触碰 global hash link |
| 3 | `node_modules\.bin\{semver,semver.cmd,semver.ps1}` | **全是普通文件（cmd 垫片）**，非符号链接 |
| 4 | `.bin` 之外 | **零链接** |

配合 macOS 端到端实测（非全局 `pnpm add` 不创建 `global/`、包树零链接；自建 157 字节 shim →
`dsh --version` → `0.1.5-rc.2` 成功）⇒ **方案 B 两平台均成立**。
（原本唯一无法在 macOS 判定的未知 = Windows 的 `.bin` 形态，已由真机回答。）

#### 四、问题 1.1 的状态变化（**如实标注推进阶段，不写"已修好"**）

**「1.1 已从『只修诊断与出路』推进到『根因已定案并进入实施』，不是『已修好』。」**

| 阶段 | 状态 |
|:---|:---|
| 根因定位 | ✅ 已证实：pnpm 12 全局安装的 global hash link 需符号链接特权；配置级穷举 7 候选均未消除（阴性结论已自证） |
| **根因的解法** | ✅ **已定案**：ADR-0017 采纳方案 B，并经 Windows 真机探针 + macOS 端到端证实可行 |
| 诊断与出路（v1.2.1 已交付） | ✅ 已修：错误分类「特权 vs 网络」拆开、**权限类判据优先**，错误卡给 `boot_in_wsl` / `retry` |
| **安装能力（根因修复本体）** | 🚧 **实施中**（`task-56`）→ 独立验证（`task-57`）→ **Windows 真机待复验** |

⇒ 本批为**机制级修复 + 单元/契约级验证**；**Windows 真机复验尚未完成**——
**不得**据此写成「已修好」。实施与验证产物出来后另发一条**追加条目**（分两次写，不合并）。

#### 五、随本变更校正的失真文档

| 文件:行 | 原状 | 校正 |
|:---|:---|:---|
| `docs/contract.md:161` | 「pnpm 的全局目录或安装动作失败时**回退 npm**；因此 pnpm 是优先路径，**不是桌面应用的硬依赖**」 | 该句**在本变更之前即与既有裁定冲突**（ADR-0005 补录 2026-08-28「装 dsh 本体不再回退 npm」、AGENTS §6「pnpm 为环境检查硬依赖」）。已按对齐口径改写：pnpm = **环境检查硬依赖且随壳内置恒在**；node 经 `runtime set node` 补齐；**npm 链已随探测层退役（ADR-0010），不再用于安装 dsh 本体**；**dsh 本体 = project 内安装（ADR-0017）**，并明写该形态**正是为兑现「不要求管理员权限」** |
| `docs/contracts/child-lifecycle.md:208`（§3.4 spawn 面清单） | `engines` 的「**`pnpm add -g`（分钟级）**」 | 改为「dsh **project 内安装**（`<engines>/dsh-runtime/`；ADR-0017）」，并注明 **WSL 客体仍 `add -g`**、**仍经 `lifecycle::run`**（**闸门语义不变**，仅描述失真） |
| `docs/contracts/child-lifecycle.md:294`（§6 macOS 清单项） | 「引导期：**`pnpm add -g`** 进行中强杀壳」 | 同步改为 project 内 `pnpm add` |

**未升 `MANIFEST_FORMAT`**：以上为**散文失真校正**，不涉 `product.manifest.json` 字段
（`format` 仍为 3；`terminal.resolution.*.tiers` 语义未动）。§4.1「先改 contract → 升格式」
针对**契约字段**变更，本批无字段变更，故不升版。

- **影响**：仅周知。① 后续改引擎安装/升级链须以 **ADR-0017** 为准（`add -g` 仅 **WSL 客体**保留）；
  ② Windows 真机项仍待复验（清册 **D1** 搁置中），**不得**把 CI 全绿当该类验证证据
  （已登记 roadmap **§4.16**，该条已按本 ADR 更新为「消除一个具体成因 ≠ 消除结构性事实」）；
  ③ 待裁定项：清册 **H1/H3/H4/H6**。
- **凭据**：`52b876e`（AGENTS.md + ADR-0017）；`docs/adr/0017-dsh-project-local-install.md`；
  `docs/team/V120B-Windows探针-方案B验证.md`；`scripts/probe-windows-engine-install.ps1`；
  文档校正 diff = `docs/contract.md` + `docs/contracts/child-lifecycle.md`。

### 2026-09-11 发版 v1.2.1（Windows 实测问题修复）· 发版结果回填 —— guan（AI 协作）

- **版本**：`v1.2.0` → **`v1.2.1`**（patch：全部来自实机缺陷报告，无新契约/新 IPC）。
  四处版本号同步（`Cargo.toml` / `tauri.conf.json` / `package.json` / `Cargo.lock`）。
- **发版日志（AGENTS §8.8 先落盘再打 tag）**：`docs/RELEASE_NOTES.md` 顶部
  `## [v1.2.1] - 2026-09-11`；`extract-release-notes.py v1.2.1 --strict` 通过，
  并已作为 Release 正文发布。
- **打 tag 前闸门（提交态实测）**：版本号四处一致 ✅ · cargo fmt `0` · clippy 宿主 `0` ·
  clippy win-gnu `0`（touch 后确认 `Checking dsh-dock` 真编译，防缓存假绿）·
  `cargo test` **394 passed / 0 failed / 3 ignored** · 前端 typecheck `0` / lint `0` /
  test **45 files · 365 passed** · scripts **17 OK**。
- **CI 结果（tag `v1.2.1`，全绿）**：
  - `build` run **`34622009396`**：**6/6 job 全绿**——4 个 build leg（`macos-latest` /
    `macos-latest-x86_64` / `windows-latest` / `ubuntu-latest`）+ **`release`**；
    `coverage` 按设计在 tag 上跳过。**注意 `macos-latest` 正是前一 run 红掉的那条**。
  - `boot-smoke` run **`34622009478`**：4/4 全绿（含 `engine-bootstrap (windows-latest)`）。
  - **Windows 单测 345 passed / 0 failed**（v1.2.0 为 316）。
  - **双 macOS 架构公证均 Accepted**：arm64 id `32186889-0891-48a3-9022-abbd899e7c88`、
    Intel id `9e2279d6-0bbc-4b12-8eaf-552375b674f9`；两者均 `accepted` +
    `source=Notarized Developer ID`。
  - **Release 资产 17 个**；`latest.json` = **7 平台**，两个 macOS updater 目标
    （`darwin-aarch64` → `DSH.Dock_aarch64.app.tar.gz`、`darwin-x86_64` →
    `DSH.Dock_x86_64.app.tar.gz`）**均带架构后缀、无重名**——v1.2.0 引入的架构打标
    在本次继续生效。
- **发布内容**：见 `docs/RELEASE_NOTES.md` v1.2.1；逐条处置与证据见
  `docs/known-issues/v120-测试问题定位.md` §10 与本档同日「v1.2.0 实测问题处置」条目。
- **⚠️ 必须与发版同时知悉的边界**：
  1. **Windows 普通账户下的「本地模式」仍装不上引擎**——本次修的是**诊断与出路**，
     安装能力未变（pnpm 12 上游缺陷，架构级替代需 ADR-0010 修订，**未决策**）。
     发版日志已在「已知限制」中向用户明示（推荐改用 WSL 模式）。
  2. 涉 Windows / WSL / 真下载的路径为**机制级修复 + 单元与契约级验证**，
     **Windows 真机复验待排期**（维护者已搁置）；**不得**把 CI 全绿当作该类验证证据
     （roadmap §4.16）。
- **凭据**：tag `v1.2.1` → commit **`59eba77`**；CI `34622009396` / `34622009478`。

### 2026-09-11 推 CI 后补修：guest 脚本夹具密封化（第二次同根因）—— guan（AI 协作）

- **触发**：推 `2e864a4` 后 CI run `34618124850` **`build (macos-latest)` 单测红**（其余 3 leg 含 Windows 全绿）：
  `updates::world_probe_tests::guest_script_output_parses_into_versions` →
  `panicked at src/updates.rs:1587: assertion `left == right` failed: 脚本的 v 前缀须被归一`
  · `left: Some("24.20.0")` / `right: Some("24.18.0")`。
- **根因（机制已复现）**：`guest_prep!()` 开头 `. /etc/profile`；macOS 的 `/etc/profile` 执行
  `path_helper`，把 `/etc/paths` 各项（**首项 `/usr/local/bin`**）**前置**到 PATH。
  GitHub macOS runner 的 node 恰在 `/usr/local/bin`（v24.20.0）⇒ 夹具只把 shim 放进 PATH 时
  被 runner 的真 node 顶掉。
- **本机为何绿 = 双重巧合**（非可靠）：① `/usr/local/bin/node` 不存在；② 本机真 node
  （引擎 bin）**也是 v24.18.0，与夹具期望值同值** ⇒ 夹具即使失效，版本断言也**瞎**。
- **修复**（`053e156`）：shim 落 `$HOME/.dsh-dock/engines/bin`——该目录由脚本在
  source 完三个 rc **之后**才前置，故确定胜出；并加**夹具自证断言**（脚本读到的 node 路径
  必须在假 HOME 内），比版本断言更早、更明确地报因。
- **⚠️ 这是第二次同根因，不是新问题**：`executor.rs::run_guest` **早已密封**，且其注释
  **独立写明同一根因**（macOS `/etc/profile` → path_helper → `/usr/local/bin/node`，
  GitHub Actions macOS 实测），修于 `2e097b8` / `4cf0b2d`（2026-08-26）⇒ 本次在
  `updates.rs` 复发。**纪律（新）**：跑生产 shell 脚本的测试，其**工具夹具一律落
  `$HOME/.dsh-dock/engines/bin`**，不得只依赖继承的 PATH。已登记 roadmap §4.17。
- **端到端判据（决定性）**：**本机绿不算证据**（M10/M11 同类）——以 CI 为准：
  绿 run **`34619208743` 4/4 leg 全绿**，其中 `build (macos-latest)` 正是前一个 run 红掉的那条。
- **独立验证**：qa-verify 用**真实生产脚本**做 4 态矩阵（旧放法复现 `v24.20.0`，与 CI 报错
  **逐字一致**；新放法绿）+ **最大敌意序**（竞争者同时在继承 PATH / `.profile` / `.bashrc`）
  仍绿 ⇒ **结构性免疫**；并实测「版本断言在同变异下 0 命中」（证实本机确实瞎）、
  「该自证断言在修前旧放法下即为红」（本可本机提前抓出）。见
  `docs/team/V120-CI补修-夹具密封化.md`。
- **同类残留**：全仓矩阵扫描**未发现同失效类**（`guest.rs` 6 条经敌意 PATH 实测确认断言
  不敏感；`sessions.rs` 13 条属另一子类——有意用真 node，无 node 时优雅跳过：33 绿）。
- **边界**：本机无法把真 node 放进 `/usr/local/bin`（非 root + 不可写）⇒ 本机证据是
  **机制模型**而非 CI 机制本身；CI runner 的 PATH 快照未取。
- **凭据**：`053e156`（补修）· `2e864a4`（首推）· CI `34618124850`（红）/ `34619208743`（绿）。

### 2026-09-11 v1.2.0 实测问题处置（8 条）· 跨层契约元结论 —— guan（AI 协作）

- **输入**：`docs/known-issues/v120-测试问题.md`（v1.2.0 Windows 实机，4 张截图）
  → 定位 `docs/known-issues/v120-测试问题定位.md`（`task-47`）
  → 实施 **`2a87bfd`**（`fix(v1.2.0): 修复 Windows 实测 6 条缺陷 + 2 条实施中新发现（D6b/T-F5）`）
  → 本条目为**处置结果收口**（`task-49`）。
- **收口范围 = 8 条**：原 6 条 + **实施中新发现 2 条**（`D6b` 跨层 seam / `T-F5` 假按钮）。

**逐条结论**

| # | 现象 | 处置 | 状态 |
|:--|:---|:---|:---|
| 1.1 | Windows 本地引导 `registry 均不可达` | 根因 = pnpm 12 全局安装的 global hash link 需符号链接特权（普通账户 `os error 5`）⇒ **本轮修的是「诊断 + 出路」，不是「安装能力」**：错误分类拆开（特权 vs 网络，特权优先）+ 错误卡给可行动出路（`boot_in_wsl`/`retry`）。**本地模式在普通账户下仍装不上**；配置级解法本机穷举 7 条候选未消除该链接，架构级替代需 **ADR-0010 修订**（提案见 `V120-D1…md` §6，**未实施**） | ⚠️ **诊断/出路已修；安装能力未修（属上游缺陷）**；真机待复验 |
| 1.2 | 「查看启动详情」收不回 | 删 `hasError` 一票否决（`ErrorCard` 在时间线之外，收起不会藏错误） | ✅ 已修 |
| 1.3 | 「下次默认打开方式」窗口内无入口 | 偏好面板补入口，经 `patchShellSettings` 写 `defaultMode`，**不新增 IPC** | ✅ 已修 |
| 2.1 | 切 WSL 后残留本地旧错误 | 后端「开新一轮 + 撤销上一轮可见状态」合成一个动作 `BootRound::begin_round()`；前端补权威清零点 `beginNewRound` | ✅ 已修 |
| 2.2 | 关于页「未检出」vs 大盘「就绪」 | 版本探测**按世界择源**（复用既有客体通道，零新脚本） | ✅ 机制级已修；**客体真跑未验** |
| 3 | 点下载无 loading / 无进度条 | Rust 进入动作即发可见态（`Checking` 早于 `blocked_check`）；前端 `TRANSITIONS` 补自环与缺边 | ✅ 已修 |
| **D6b** | **实施中新发现**：D6 修完后换形式复发（卡「检查中」、进度 `[]`） | 前端补 `checking → downloading` + 新增**常驻跨层 seam 契约测试**；`task-55` 把手工同步换成**双向机器闸门** | ✅ 已修 + 闸门闭合 |
| **T-F5** | **实施中新发现**：D1 错误卡的「改用 WSL 打开」是**假按钮** | 动作→IPC 从隐式 `terminalAction` 改为**显式分派表** `ACTION_IPC` | ✅ 已修 |

> **D1 的三方案必须让读者一眼看到 B 尚未决策**（避免"以为修过"的翻版）：
> **A 保持现状 + 出路引导**（本轮已落地）、**B project 内安装 + 壳自建 shim**
> （触碰 ADR-0010「引擎=壳资产」的安装形态与 shim 语义，且 `dsh plugin` 转发链
> 依赖全局 shim ⇒ **影响面大，须专项，未决策**）、**C 等上游修**（本机无法判定）。
> 即：**引擎能否在普通账户 Windows 上装成，本轮未改变。**

**闸门（提交前冻结树实测）**：Rust `fmt=0` · clippy 宿主 `=0` · clippy `win-gnu=0`
（强制 touch 5 文件重编，确认真跑非缓存）· `cargo test` **394 passed / 0 failed / 3 ignored**；
前端 typecheck `=0` · lint `=0`（144 files/96 rules）· test **45 files / 365 passed**；
scripts **17 OK**。修前基线 = Rust 364 / 前端 41 files·308 passed。

**验证边界（不得粉饰）**：**Windows 无实机**（维护者已裁定搁置）⇒
1.1 / 2.2 / D6b 等一切涉 Windows·WSL·真下载路径一律为
「**机制级修复 + 单元/契约级验证，真机待复验**」——**不写"已修好"**。
未覆盖项（G1–G6）逐条登记于定位文档 §10.4。

---

#### 跨层契约元结论（本轮最有复用价值的产出）

**「事件生产者与消费者的测试各自绿」不构成契约已被覆盖。**

D6 修完之后**换形式复发**：rust-core 为「点了下载立刻有 loading」在 `blocked_check` 前
新增 `ClientUpdate::Checking`（意图正确），而前端 `TRANSITIONS.checking` 不含 `downloading`
⇒ **整条进度序列自首条起被判非法丢弃**。而**两侧单测当时都是绿的**——
Rust 测自己的相位模型、前端测自己表的自洽性，**没有任何一条测试跨过 IPC 边界**。

由 lead 在**提交前**端到端检查发现，实测两态：

| 状态 | 被丢弃 | 进度采样 | 末态 |
|:---|:---|:---|:---|
| 破损态 | **6 条**（`checking→downloading`×3 / `→installing` / `→done` / `→relaunching`） | `[]` | 卡 `checking` |
| 修复态 | `[]` | `[10,50,90]` | `relaunching` |

**这与 D5b（`updates.rs` 改路径 + 前端 `setVersions` 覆盖）同源 ⇒ 是本仓库的系统性风险类别，不是偶发。**

**判据口径**：跨层契约的判据必须落在「**发射序 × 消费表**」的**联合断言**上，
而非任一侧的自我一致性；且必须**同时消费两侧的真实产物**——
不得在测试内重实现逻辑，**也不得用手抄副本充当"真实序"**（副本必然过期，且过期后仍绿）。

已落地为团队纪律 **M12**（`docs/team/README.md` §8；编号刻意不回填 M10——
那两个是从未落盘的候选编号，而 M12 已三处交叉引用），并由 `task-55` 落成机器闸门
`updater.rs::frontend_transitions_accept_every_emitted_pair_per_branch`
（qa-verify 构造的反例已被捕获并精确点名非法边）。
**残余边界**（如实登记，见定位文档 §10.4）：只覆盖一处发射点 / 文本级推断的固有边界 /
刻意不做全表等价 / 前端那份序列仍是手抄副本。

**待裁定**：发射序是否升级为**共享 fixture**（`ipc-shapes.json` 先例，两侧断言同一文件）
以消除"副本过期"这一复发根因——列下一轮候选（清册 **H4**）。

- **影响**：仅周知。① 后续凡改 IPC 事件 / 序列化形状，须按 **M12** 补跨层联合断言；
  ② Windows 真机项仍待维护者复验（清册 **D1** 搁置中），**不得**把 CI 全绿当该类验证证据
  （已登记 roadmap §4.16）；③ 待裁定项：清册 **H1/H3/H4/H6** 与 ADR-0010 修订可能性（**H2 已订正**）。
- **凭据**：`2a87bfd`；定位 `docs/known-issues/v120-测试问题定位.md`（含 §10 处置表与 §10.3 逐条核实命令）；
  各域证据 `docs/team/V120-{D1,D4,D5,D5b,D6,D6c,F,F5,F6}*.md`；
  独立验证 `docs/team/V120-独立验证.md`（含其自建反例与 §6.2 自省）。

### 2026-09-11 发版 v1.2.0 · WSL 客体管理面 + macOS Intel 原生包（含 G3 隐患修复）—— guan（AI 协作）

- **发版依据**：AGENTS §8.8「发版严禁裸打 tag」——已按 `docs/prompts/release-notes.md`
  生成规范日志并落盘 `docs/RELEASE_NOTES.md` 顶部（`## [v1.2.0] - 2026-09-11` 强契约，
  `extract-release-notes.py --strict` 实测可提取）。
- **版本号语义**：`1.1.1 → 1.2.0`（minor）。本批含两个 feat 级变更
  ——WSL 客体管理面（ADR-0016）与 macOS Intel 原生包——按 semver 应升 minor。
  四处版本号（`Cargo.toml` / `tauri.conf.json` / `frontend/package.json` /
  `Cargo.lock`）同步改为 `1.2.0`，与 tag 一致（CI 有 tag 版本一致性闸门）。
- **本批内容（含此前未发版的全部改动）**：PR #13 合并（WSL 客体管理面 ADR-0016
  P1/P2/P3 + 契约/ADR 落档）、macOS Intel 构建通道、`cordis.patch.yml` 保真写入统一、
  两处 `panic=abort` 崩溃修复、会话临时脚本泄漏修复、i18n 收口、
  「唯一网络面」机器闸门、子进程生命周期闸门加固。
- **G3 隐患修复（本次一并解决）**：`engine_pnpm_bundle` 会映射 `win32-arm64`，
  而 `fetch-pnpm-bundle.sh` 白名单拒绝它（上游 `@pnpm/exe.win32-arm64` 实测存在，
  registry 六平台全 200）⇒ 本仓脚本缺口。修法不止补名字：
  1. 白名单补 `win32-arm64`；
  2. `uname` 自动探测补 `MINGW*-aarch64|MSYS*-aarch64`（否则 Windows ARM64 上
     无参调用会掉进 `*)` 直接退出——**只补白名单不够**）；
  3. 新增**不变式闸门** `scripts/tests/test_release_platform_matrix.py`：
     断言 engine 可产出集合 / uname 探测集合 / CI matrix 声明集合**三者皆 ⊆ 白名单**
     ——把「映射与打包支持必须一致」变成机器判据，同类缺口不再复发。
  **实证**：`fetch-pnpm-bundle.sh win32-arm64` 真跑成功（16.7 MB，
  归档含 `package/pnpm.exe`，正是 `stage_pnpm_from_bundle` 期望成员名）；
  **变异证伪**：回退白名单 ⇒ 3 条守卫用例红并点名该平台；恢复后 sha256 回一致、残留 0。
- **验证（提交态，本机四闸门 + CI 同款脚本）**：`fmt=0` ·
  `cargo test 364 passed / 0 failed / 3 ignored` · clippy 宿主 `0` · clippy win-gnu `0`；
  `scripts/tests` **17 OK**（新增 4 条平台矩阵守卫）；tag 版本一致性四处比对通过；
  发版日志 `--strict` 提取通过；workflow YAML 解析通过。
- **待 CI 确认（tag 触发）**：三平台签名构建与公证、`latest.json` 出现 **7 个**平台条目
  （含 `darwin-x86_64`）、两个 macOS `.app.tar.gz` 名字各带架构后缀不重名。

- **发版结果（tag `v1.2.0`，run `34602173101` `conclusion = success`）**：
  - **6 个 job 全绿**：build（4 leg：macos-latest / **macos-latest-x86_64** /
    windows-latest / ubuntu-latest）+ release；`coverage` 按设计在 tag 时 skip。
  - **`latest.json` = 7 个平台条目**（与预告一致）：`darwin-aarch64`、
    **`darwin-x86_64`**、`linux-x86_64-appimage`、`linux-x86_64-deb`、
    `linux-x86_64-rpm`、`windows-x86_64-msi`、`windows-x86_64-nsis`。
  - **Release 资产 17 个**：arm64 dmg 21.7MB / **Intel dmg 24.5MB**、
    两个 macOS `.app.tar.gz` **各带架构后缀**（`DSH.Dock_aarch64.app.tar.gz` 20.3MB /
    `DSH.Dock_x86_64.app.tar.gz` 23MB）——**首次出现两个 macOS updater 目标**。
  - **签名 + 公证（两个 macOS leg 均通过）**：
    arm64 `Notarizing … Accepted`（id `2bda1b50…`）、
    **Intel `Notarizing … Accepted`**（id `054b435a…`），
    两 leg 均 `source=Notarized Developer ID`、`spctl: accepted`。
    **Intel 签名是本批全新路径，此前从未走过 ⇒ 现已实测打通。**
  - **`PNPM_TGZ` 参数化在真 CI 生效**：Intel leg 重签的是
    `resources/pnpm/darwin-x64.tgz`（不再是硬编码 arm64），
    日志可见 `elif [ -n "darwin-x64" ] → fetch @pnpm/exe.darwin-x64`。
  - **Windows 单测 316 passed / 0 failed**（上次红的用例已消除）。
  - 打标步骤在 tag 构建同样生效：日志先后出现 Tauri 默认名 `DSH Dock.app.tar.gz`
    与打标后名，两 leg 不重名。

### 2026-09-11 macOS Intel (x86_64) 支持 · 发布矩阵加第四个 leg —— guan（AI 协作）

- **背景与目标**：此前发布产物只覆盖 Apple Silicon（`README.md` 已声明「Intel Mac 需自行源码构建」）。
  维护者裁定补上 Intel 原生产物。
- **方案选择（记录取舍，便于日后复核）**：
  - **否决 universal 通用包**：Tauri 的 `darwin-universal` 会让已装 arm64 客户端在
    `latest.json` 里找不到自己运行时请求的 `darwin-aarch64` 键 ⇒ **存量用户收不到更新**
    （迁移风险）；且体积翻倍（与「安装包保持在几十 MB」的既有取向相悖）。
  - **采用独立 x86_64 leg**：**纯增**——既有的 `darwin-aarch64` 条目与下载路径一字未动，
    存量用户升级链零影响；两个 arch 各自只内置对应 pnpm 引擎，体积与原生性能都不打折。
  - 不用 `macos-13`（Intel runner，GitHub 已在退役流程中），改在 arm64 runner 上
    **交叉构建**（`--target x86_64-apple-darwin`）。
- **变更清单**：
  1. `.github/workflows/build.yml`：build job 矩阵改为 `include:` **4 leg**；新增
     `macos-latest-x86_64`（`rust_target=x86_64-apple-darwin`、`pnpm_platform=darwin-x64`）。
     **既有三个 leg 的 job 名逐字未变**（不动分支保护的必需检查）。
  2. 每个 leg 显式 fetch **自己那一份** pnpm tgz——实测 Tauri 会把 `resources/` **整目录**
     塞进 `.app`，多取会让包里多背另两份（本机实测 59MB → 应约 24MB）。
  3. 新增「按架构打标」步骤：Tauri 对 `.app.tar.gz` 的默认名**不含架构**
     （`DSH Dock.app.tar.gz`），两个 macOS leg 会产出**同名文件**（Release 资产重名 +
     artifact 相互覆盖）⇒ 打标为 `DSH Dock_aarch64.app.tar.gz` / `_x86_64.app.tar.gz`
     （重命名不影响签名：签名对象是 tarball **内容**）。
  4. `scripts/generate-latest-json.py`：`REQUIRED_TARGETS` 加 `darwin-x86_64`；
     macOS 分支改为按资产名架构标记映射，且**刻意不设默认架构**——若漏打标，
     回落成 `darwin-aarch64` 会把 **Intel 包当 Apple Silicon 发出去**（用户装完起不来），
     故判为不可映射、由完整性检查**响亮失败**。
  5. `scripts/tests/test_generate_latest_json.py`：夹具带架构标记 + 新增
     「架构标记→目标键」「未打标不得静默映射」「同名撞车必须拒绝」3 个用例（共 12 用例）。
  6. `README.md`：平台表加 Intel 行（`.dmg` 24 MB）+ Intel/ARM 选包说明；原
     「Intel Mac 可自行源码构建」改为「Windows ARM64 等其余架构」。
- **影响**：`latest.json` 由六平台条目变为 **七平台**（新增 `darwin-x86_64`）；
  Intel Mac 用户获得原生包与自更新能力；存量 Apple Silicon 用户升级路径不受影响。
- **验证（实测，非推演）**：
  - 本机交叉构建 x86_64 成功，产物 `lipo -archs` = **x86_64**，DMG **24 MB**；
    只留 arm64 份重建，`DSH Dock_<ver>_aarch64.dmg` 仍是 **arm64**，DMG 23 MB；
    两个 leg 的 app 内 pnpm 资源各只有对应那一份（证明「每 leg 只取自己那份」这条约束）。
  - **端到端真跑 feed 生成**：伪造 CI 会产出的资产名 ⇒ `platforms` = 7 条，
    `darwin-aarch64` 与 `darwin-x86_64` URL 各自独立；
    **两个反例均响亮失败**：① macOS tarball 未打标 ⇒ `缺少目标：darwin-aarch64, darwin-x86_64`；
    ② 只缺 Intel 那份 ⇒ `缺少目标：darwin-x86_64`。
  - 打标脚本以本机产物真跑，覆盖「无 `.sig`」（`--no-sign` 的 PR 构建）分支。
  - 工作流 YAML 解析通过：4 leg、全 leg 键集合一致、job 名唯一、既有三个名未变。
  - 发布侧 `scripts/tests` 12 用例全绿。
- **独立验证（qa-verify，task-41）推翻了本批的一处阻断缺陷与一处测试恒真式**：
  1. **🔴 阻断级：新 leg 在 CI 必然红**（本批 `0631edf` 态）。证据链五环：
     `macos-latest` = **arm64** runner（runner-images README）→ 镜像脚本 `--profile=minimal`
     （只装**宿主** std）→ 全镜像链无 `rustup target add` → 本 workflow 也无
     → 实测 rustup **不自动补装**（wasm32 探针 + 精确复现首个失败点
     `error[E0463]: can't find crate for 'std'`, EXIT=101）⇒ 新 leg 会在第一个消费
     target 的步骤（clippy）就红，**Intel 产物根本不会产出**。
     **arm64 leg 不受影响**（`--target aarch64-apple-darwin` = 宿主本身）——正因如此极易漏掉。
     **已修（`c2cbde1`）**：加一步 `if: matrix.rust_target != ''` →
     `rustup target add ${{ matrix.rust_target }}`（不传 action 的 `targets:`，
     避免依赖 action 对空串的解释）。qa-verify 已独立复验。
     **教训：「本机可交叉构建」≠「CI 可交叉构建」**——本机绿灯是因为**早装过该 target**，
     CI runner 是干净的。这与 R1 同族但更隐蔽：**环境前置条件**不写在代码里，
     只看代码或只看本机绿灯都发现不了。
  2. **🟡 测试恒真式**：原「撞名」用例实为**因「缺少目标」而通过**，未触发目标重复检查
     ⇒ 该检查无覆盖（qa-verify 用「删掉该检查后用例仍 ok」证伪）。
     已改为「先用齐全 7 目标满足完整性，再塞同义标记别名（`_arm64`/`_aarch64`）」
     并**断言错误消息**；另增真实撞名场景用例（两 leg 各传未打标同名 tarball + 不同签名
     ⇒ 读签名阶段必须红）。**我方按同一证伪法自证**：删掉该检查 ⇒ 新用例报
     `ValueError not raised`（证明它是**唯一**红因）；恢复后 sha256 逐字节回一致、残留 0。
  3. **口径修正（A5，采纳）**：本批 commit message 写的「Windows/Linux 的命令与路径
     **逐字未变**」**不准确**——实测为**功能等价**（3 处 `if/elif` 分支求值后回落旧命令，
     另有 2 处纯空白差异：clippy 双空格、test 尾随空格）。
     **但不宜为消除空格改用 shell `if`**：本 workflow **无顶层 `defaults.run.shell`**，
     且 `Unit tests` 步骤未指定 shell ⇒ Windows 上默认走 **pwsh**，`[ -n … ]` 会直接报错。
     现用的 **GitHub 内联表达式**是跨 shell 中性且正确的最小选择，故保留并**只更正措辞**。
     （CI 日志原始行：`cargo clippy  --all-targets -- -D warnings`）
  4. 独立验证另确认：C1–C6 全部证实；A1 非静默跳过（有 `cargo test --no-run` 补偿步）；
     A2 全部 macOS 下游路径已带 triple；A4 空串正确；A6 真实资产名无歧义；
     **体积机制坐实**：三份 tgz 齐备时 x64 dmg = 58.87 MiB，与单份构建差
     **34.38 MiB ≈ 两份 tgz 之和（误差 0.004）**——「整目录塞进 .app」由此闭环。
- **CI 实跑结果（2026-09-11 推送后，run `34600694255`，`conclusion = success`）**：
  5 个 job 全绿——`build (macos-latest)` / `build (macos-latest-x86_64)` /
  `build (windows-latest)` / `build (ubuntu-latest)` / `coverage (baseline)`。
  - **Intel leg 从零到产物走通**：日志逐字显示
    `rustup target add x86_64-apple-darwin` → `cargo clippy --target x86_64-apple-darwin`
    → `cargo test --no-run --target …` → `cargo tauri build --no-sign --target …`
    → `DSH Dock_1.1.1_x64.dmg`（**产物 `DSH Dock-macOS-x86_64` = 24.4 MB**）。
  - **撞名消除在真 CI 被证实**（C3）：日志先后出现未打标的
    `DSH Dock.app.tar.gz`（Tauri 默认名）与打标后的 `DSH Dock_x86_64.app.tar.gz`
    ——**这就是「若不打标两 leg 必同名」的直接实据**。arm64 leg 同步为
    `DSH Dock_aarch64.app.tar.gz`，两者互不冲突。
  - **README 体积数字被 CI 产物证实**：arm64 `22.5 MB`（声明 22）、
    Intel `24.4 MB`（声明 24）、Windows `78.8 MB` = NSIS 39 + MSI 40（声明 39/40）、
    Linux `140 MB` = deb 23 + rpm 23 + AppImage 94（声明相符）。
  - **Windows 上次的红已消除**：`Unit tests` step = success（即本批 `0ab796c`
    的测试夹具修复在真 Windows 生效）。
- **未验证 / 边界（不假装闭合）**：**签名与公证路径仍未实测**（普通 push 走
  `--no-sign`；签名只在 `v*` tag 构建触发），故 Intel 包的
  codesign/notarize 链路要到下次 tag 才验；**self-update feed 的端到端**（真实客户端
  从 `darwin-aarch64` / `darwin-x86_64` 各自取到正确包）需下次 tag 发布后验；
  Intel leg **只编译不运行**测试（arm64 runner 上跑 x86_64 二进制依赖 Rosetta，
  不保证预装）——执行面由原生 leg 覆盖。
- **后续动作项**：下次 `v*` tag 发布时重点核对 ① macOS 两个 leg 的签名/公证均通过
  ② `latest.json` 出现 **7 个**平台条目（含 `darwin-x86_64`）
  ③ Release 资产里两个 `.app.tar.gz` 名字各带架构后缀、不重名。

### 2026-09-11 合并 PR #13（WSL 客体管理面 ADR-0016）并统一 patch 写入内核 —— guan（AI 协作）

- **背景与目标**：`feat/wsl-guest-management-plane`（PR #13，15 笔）与本地 `master`
  未推送的 28 笔**同基点 `6956a9a`**，构成标准双向分歧。维护者裁定方案 A：
  本地合并 + 解冲突 + 修 S1 + 验证，然后一次 push。
- **变更清单**：
  1. **合并**（`4d5c1d7`，`--no-ff`）：解 6 文件 15 块冲突。
     `AGENTS.md` 取我方 §7 压缩（55 条命令零缺失）＋分支实质新增（WSL 客体管理面
     网络登记、ADR-0016 索引）；§11 回收后**全文 250/250 · §6=39 · §7=39**；
     `docs/broadcasts.md` 倒序档案**两侧条目并存**（本轮 3 + 分支 8）；`lib.rs`
     两侧 `mod` 均保留；`commands/profile.rs` 取分支「世界择源」结构 ＋ 我方 C3 措辞。
  2. **S1 修复（核心）**：分支客体 patch 写入走 `render_patch_entries`
     （整数组重序列化 ⇒ **除头部连续注释块外注释全丢**）且**无备份**，与 AGENTS §6
     已登记的「统一走 `plugins.rs::PatchFile`」冲突，裸合并会让登记失真。
     修法 = **宿主与客体共用同一份保真语义**：`PatchFile::from_text` 提为
     `pub(crate)`、抽出纯函数 `render()`、`write()` = render + 覆写前备份（fail-closed）
     + 原子替换；**删除分支第二套内核**；四条客体 patch 写入路径
     （`set_plugin_disabled_in_guest`、`copy_plugin_config` 客体分支、
     `save_mcp_server_in_guest`、`delete_mcp_server_in_guest`）全部改为
     「同一内核 + 客体侧 `guest::backup_file` + 原子写」；`mcp.rs` upsert/remove
     提取为共用内核，未命中即**逐字节原样返回**。
  3. **回归保护**（判据 = 回退真实现即红）：新增
     `mcp::tests::guest_apply_paths_preserve_comments_and_key_order`（钉客体文本进出
     路径保真）与 `plugins::op_tests::disabling_one_row_preserves_other_rows_inline_comments`
     （钉未改动条目的行间注释）。**已注入自证**：换回旧内核 ⇒ sha256 变化、测试红在
     `src/mcp.rs:650` 的**目标断言本体**（非副判据）、恢复后 sha256 逐字节回一致、
     残留扫描 0。
  4. **`AGENTS.md` §6**：该条措辞补「宿主/客体同一内核」。
- **影响**：合并后测试 **332 → 364 passed**；WSL 客体管理面（ADR-0016 P1/P2/P3）
  与本地本轮全部改动同时可用，且 `cordis.patch.yml` 的保真+备份语义在**宿主与客体
  两侧一致**。分支 tip 已成为 HEAD 祖先 ⇒ push 后 PR #13 自动标记 merged。
- **验证**：提交态四闸门 `fmt=0` / `cargo test 364 passed 0 failed 3 ignored` /
  宿主 clippy `0` / win-gnu clippy `0`；前端 typecheck 干净、lint 0-0、**308 passed**；
  **全新克隆独立复核**（不依赖本仓 `target/`）同为 364 全绿。
  过程教训：我用 `&&` 串联闸门时 `fmt` 失败导致 **test 静默未执行**，已改为逐闸门
  取独立 exit code（与 M7「提交态验证」同族）。
- **未做（边界，不假装闭合）**：**尚未 push**；分支 CI 的绿是对 `origin/master` 的绿，
  **不能推出合并后绿**；本机无原生 MSVC，合并新增的 `#[cfg(windows)]` 面宿主 clippy
  覆盖不到；Linux clippy/打包本机不可执行；Windows 真机验证按指示搁置。
- **完整记录**：`docs/team/PR13整合实施记录-2026-09-11.md`（含逐文件决策与零损失核对）。

### 2026-09-11 第二轮 · 网络面机器闸门 + i18n 六类缺陷 + §6 措辞校正（5 笔）—— guan（AI 协作）

- **范围**：本轮由同一 AI 团队（5 角色 + lead）完成，共 9 个共享任务；提交 5 笔，**未推送**。
- **变更（按序）**：
  - `47ba645 test(net)`：**新增网络面机器闸门**（`src-tauri/src/network_gate.rs`，
    仅 `#[cfg(test)]`，生产二进制不含）。把 §7「唯一网络面 = `updates.rs`」从人肉纪律
    升级为机器可拦：运行时遍历 `src/**/*.rs`（**非白名单**——白名单要人记得加文件名，
    等于把漏登的坑换个地方再犯），豁免**集中表**且唯一事实源 = AGENTS §7，区分
    **「在册」与「授予豁免」**（`engines.rs`/`executor.rs` 网络在子进程内、当前零
    in-process 原语 ⇒ 在册不授权：给零命中文件开豁口等于预授权未来任意触网），豁免可
    收窄到**条目级**（`boot.rs` 仅放行 `authenticate_workbench_session`，`plugins.rs`
    仅放行 `fetch_runtime_snapshot`）。15 条单测含单向性证明（注入→红、删除→绿）、
    误报边界（注释 / `#[cfg(test)]` / 字符串与标识符）、CRLF 归一、cfg 谓词按语义求值。
    **闸门受 CI 保护**（`build.yml:142` 的 `cargo test` 无 `if` 限制、三平台都跑）。
  - `2d34ab8 fix(i18n)`：**跨窗口语言初始化缺失**（影响面最广）——`initFromSettings()`
    全仓仅 `ProfileManager` 一处调用，主窗口（启动序列）与 About **从不初始化语言**，
    恒用初始 `t: zhCN`：选了 en-US 的用户，启动页与 About 仍显中文。修 = 初始化收敛到
    App 单点 + 各窗独立持 store、运行期经 `app:settings-changed` 事件同步（守 §4.4 红线 3；
    emit 点广播的本就是 `ShellSettings` 全量，**无需新 IPC**）；**首帧**必然错（初始
    硬编码 `zhCN` + 异步 IPC）已用阻塞首帧 + 1500ms 兜底消除（无死锁路径）。
    另修**语言选项谎报默认**（中文卡「简体中文 (默认)」而产品默认是 `system`）、
    **控制流读字典值**（`BootSelector:106` `meta.tag === "DEFAULT"`——翻译该值即静默
    失效判定；抽纯函数解耦，**附带修好「custom-a 时双卡同显默认」的真 bug**）、
    **设置页 11 处硬编码文案**（含 3 处反向漏译）。
  - `b735c7c fix(i18n)`：**市场 15 处**（14 真缺陷 + 1 死代码；任务初列 7 处，补扫出
    8 处 JSX 文本节点/模板串）与 **en-US 字典系统性收口**——遍历**叶子值**结论：
    651 个叶子中**仅 1 处含 CJK**；语言自名（endonym）审定为真例外并白名单化 + 防腐化
    断言。`BootSelector` 4 处（含 1 处不可达但属潜在陷阱者，选择**消除**而非翻译）。
  - `2238807 docs(agents)`：**宪法级** —— §6 校正 `defaultProfile` 语义措辞。
    原写「None/**失效值**读取侧兜底 `web`」与实现不符（`get_default_profile` 原样
    返回、无归一化；兜底只覆盖 None，失效值靠上游清除）。**裁定 = 改措辞，不补归一**：
    行为本身是对的（非候选值不消费 → 出选择器，「不消费 + 让用户选」比「猜一个 web」
    更诚实），补归一会用写操作掩盖用户数据问题。改动后 **249/250 行、§6=40、§7=40**。
  - `d5ac541 docs(team)`：本轮 9 份报告与独立验证结论落盘 `docs/team/`。
- **凭据**：Rust `fmt` 0 · `clippy --all-targets -D warnings` 0（**host + `x86_64-pc-windows-gnu`
  交叉双目标**）· `cargo test` **312 passed / 0 failed / 3 ignored**（跨轮 290→292→297→312，
  逐轮增量均与新增测试数吻合）；前端 `typecheck` 0 · `oxlint` 0 warning（138 files）·
  `test` **40 files / 299 passed**（跨轮 229→250→275→299）。
  **提交前独立验证**（`docs/team/第二轮收尾验证-2026-09-11.md`，总判定「可提交」）：
  6 个新测试**逐一实测红**（回退→跑→trap 复原→sha256 全过）+ 网络闸门 6 项对抗
  （注入新文件精确报、5 原语 5/5、豁免表假条目双测试红、**条目级一放一拦对照**）+
  工作树未被污染三重证明 + 越界 14/14 在 scope、红线五项全 0。
- **如实登记的偏离**：① 「格式夹带」并非零差异——`--ignore-all-space` 341/71 vs 普通
  343/73，差异 2 行全部在 `i18nStore.ts`，形态为函数抽取导致的**缩进位移**，
  **非无意义全表重排**；② 任务描述中「zh 用户看到英文徽章 DEFAULT」的前提**不成立**
  （`displayProfiles[].tag` 是从未渲染的死字段，验证者已主动更正并标注错误源头）；
  ③ 「双默认徽章」为**函数级健壮性修复**，UI 层因选择器不显示而不可达，**不夸大为
  用户可见缺陷**。
- **影响（需维护者动作）**：5 笔**未推送**。`docs/team/待裁定清册-2026-09-11.md` 更新：
  A1/A3/A4/B1 标记已登记（含本轮 §6 校正在内）；**新增 C3（已裁定，含证据链）** 与
  **C4（两窗「默认」呈现口径不一致，留待产品裁定）**；D1 Windows 真机按指示「暂时搁置」；
  新设 F 类「本轮遗留待办」（`ErrorCard:66` 同串第三处、`EV.settingsChanged` 常量收敛、
  task-23 残留死字段 `tag` 的测试同步、`types/ipc.ts` 措辞统一等）。

### 2026-09-11 宪法级改动 · 补登 §7 网络面两处 + §6 落盘资产四处（预算内回收） —— guan（AI 协作）

- **占用声明**：`AGENTS.md` 为共享宪法、同一时间仅一人可改。本次改动由维护者在会话内
  明确指示「该登记的登记」后执行，改后按 §10 在此落档并附 diff 摘要。
- **依据**：团队巡检（`docs/team/文档一致性巡检-2026-09-11.md`）发现登记册与代码实际
  不一致四处，逐条复核后补登记：
  1. **§7 网络面漏登「工作台 Token 环回兑换」**（P0）：`boot.rs:361-405`
     `authenticate_workbench_session` 用 `ureq` 对工作台地址发 HTTP GET（5s、
     `redirects=0`）——2026-09-04 落地时**只登记了 `get_boot_status`，这条网络动作漏登**，
     与 §7「唯一网络面 = `updates.rs`」及 ADR-0006 §2 冲突。**本次补登记 = 承认现状 +
     可审计**；「是否下沉到 `updates.rs` 的 `HttpGet` seam」属改码，仍留作待裁定项
     （`docs/team/待裁定清册-2026-09-11.md` A1 遗留 / A2）。
  2. **§7 漏登「客户端自更新」两路**：`updates.rs::APP_RELEASE_FEED` +
     `updater.rs`（清单端点在 `tauri.conf` 的 `plugins.updater.endpoints`）——
     此前仅 ADR-0006 §4 有记载。
  3. **§6 漏登四处落盘资产**：`node-map.json`/`.sig`（签名校验过的 node 版本映射缓存，
     1 MiB 上限、失败回退内置基线）· `procs/`（ADR-0015 孤儿清扫 PID 锁表）·
     `<文件名>.bak-<unix秒>` 备份族（2026-09-08 U9）· MCP 对 `cordis.patch.yml` 的
     重序列化写入（并就地标注「待与插件中心头部保真写入器统一」）。运行日志族经核实为
     **只读读取、非落盘资产**，故不登记。
  4. **§2「main.rs 6 行勿动」措辞精度**：实测 7 物理行（含 1 空行）、有效代码 6 行，
     改「6 行有效代码勿动（7 物理行含 1 空行）」——防未来机器判据按行数误判。
- **预算（§11：全文 ≤ 250 行、单节 ≤ 40 行）**：登记前实测 **251/250、§7=42**（已超）；
  按「登记面不改行为、可密排」原则回收 §7 命令枚举的冗余排版（**55 条命令名与日期
  逐条保留，脚本比对 `ipc.rs::COMMANDS` 零缺失**），登记后 **249 行、§6=40、§7=40**。
- **未改动项（刻意留白）**：任何技术栈锚点、代码规范、测试要求、AI 交互约束与 ADR 索引
  一字未动；本次 diff 仅登记面 + 措辞精度（`AGENTS.md` 37+/38−）。
- **影响**：**仅周知**。对 AI 与人类协作者的实操口径无变化（登记的是既有事实，不是新规则）；
  后续新增网络动作 / 落盘资产须照 §7、§6 登记，**登记册现已与代码一致**。
- **凭据**：`docs/team/文档一致性巡检-2026-09-11.md`（证据）· `docs/team/待裁定清册-2026-09-11.md`
  （A1/A3/A4/B1 标记已登记，A2/B2/B3/C1/C2 列待裁定）· 行数校验 `wc -l AGENTS.md` = 249 ·
  命令名完整性由脚本比对（55/55，缺失 0）。

### 2026-09-11 团队建制 · AI 研发团队组建与本轮五笔合入（3 个真缺陷 + 文案/文档）—— guan（AI 协作）

- **建制**：按仓库真实接缝设 5 执行角色 + lead，写入
  [`docs/team/README.md`](./team/README.md)：`rust-core`（壳运行时）/ `rust-mgmt`
  （dsh 管理域）/ `frontend`（React SPA）/ `docs-contract`（契约与文档）/
  `qa-verify`（**独立验证者，不写生产代码**），各域独占写入范围 + 共享区（IPC
  三件套）单写纪律，任务板 write scope 声明制，越界先申请。lead 保留宪法级文件、
  本档与团队章程的写入权，并统一组装提交。
- **变更（5 笔，按序；均为 fix/docs，无 feat）**：
  - `f4cd394 fix(credentials)`：`mask_api_key` 字节切片 → 按字符数判长与切前后 4
    字符。**真缺陷**：非 ASCII 凭据值（CJK/全角/emoji）必 panic，而 release 剖面
    `panic="abort"`（`Cargo.toml:52`）⇒ 进程被杀；调用点全在
    `get_credentials_summary` 路径 ⇒ **凭据面板加载即崩**。
  - `d6edc65 fix(sessions)`：① `decode_project_dir_name` 同源量纲不一致（判据按
    字符、切片按字节）⇒ 会话目录名含 CJK 即 panic（目录名用户可手工创建，非理论
    风险）；改字节判据并用「`0x3A` 不可能是 UTF-8 续字节」的不变式证明切片恒安全。
    ② 临时脚本泄漏：两函数各有**两个** `?` 早退在清理之前（node 缺失 / `lifecycle::run`
    失败——**node 存在也会泄漏**），每次加载会话面板泄漏 ~90 KB；改 RAII guard 单点
    收口。实测修前单轮 `cargo test` +5 份 / 463,140 B、本机存量 68 项 / 6256 KB，
    修后同轮 +0，存量清理归零。
  - `a265c5e fix(i18n)`：市场文案写死「2700+」（Registry 实为 3408）——**如实界定
    影响面：6 处中仅 `searchPlaceholder` 用户可见**，另 2 键 × 2 locale 为死键，一并清
    错数字；`ProfileDetailPane`「240+ 预置服务」经核实**不成立**（实测 239 包，差值
    为平台专属包，且随 dsh 版本漂移、壳侧无事实源）→ 改不依赖数字的表述并迁入字典。
  - `1251bef docs(roadmap)`：时效刷新——头部日期、Next 阶段状态、Later 六项（随
    v0.9.0 `b9973fd` 已落地）补回收注（**未闭合子项据实标注**）、IPC 口径改指针。
  - `4879b3d docs(team)`：团队章程 + 本轮全部报告与待裁定清册。
- **凭据**：`cargo fmt --check` 0 · `clippy --all-targets -D warnings` 0（host +
  `x86_64-pc-windows-gnu` 交叉，21.5s 真 check）· `cargo test` **297 passed /
  0 failed / 3 ignored**（基线 292 → +5，与新增 `#[test]` 精确吻合）· 前端
  `typecheck` 0 / `oxlint` 0 warning（132 files）/ `test` **34 files · 229 passed**。
  diff 规模 7 文件 481+/30−。提交前独立验证（`docs/team/本轮收尾验证-2026-09-11.md`
  + `复验-task13-2026-09-11.md`）：**新增测试 5/5 实测红**（回退→跑→trap 复原→
  sha256 校验）、**修前 +5 / 修后 +0 双向独立复现**、9 文件哈希与快照逐一相同、
  清理后 `dsh-dock-scan-session-*` 归零。
- **影响（需维护者动作）**：本轮**未推送**（5 笔留在本地 master 之上，未 push）。
  另有 **[`docs/team/待裁定清册-2026-09-11.md`](./team/待裁定清册-2026-09-11.md)**
  登记 AI 团队无权自行决定的事项，其中两项请优先：
  1. **§7 网络面登记缺口**：`boot.rs:361-405 authenticate_workbench_session` 用
     `ureq` 发本地 HTTP GET，未见于 §7 任何登记处（同形态的 `plugins.rs` 环回已登记），
     与 ADR-0006 §2 冲突。修复须先取舍「补登记 vs 下沉 `updates.rs` seam」；
     且 `AGENTS.md` 实测 **250/250 行**、§11 预算用满，**加一行须先决定回收哪一行**
     （建议 §7 那份 55 条枚举改指针，唯一事实源 = `ipc.rs::COMMANDS`，可回收约 20 行）。
  2. **Windows 真机验证**（v1.1.1 唯一未验证项，清单见 `executor.md`）。
- **两条团队教训（已写入 `docs/team/README.md` 防复发）**：
  1. `docs/known-issues/` 被 `.gitignore:20` 忽略——写在那里的报告不进提交，
     「不落盘 = 不存在」直接失效；团队产物改落 `docs/team/`。
  2. 本档是**倒序**档案（最新在顶部）——本轮曾因只读尾部而误判「冻结期仍在」，
     把已于 2026-09-11T03:30Z 解除的临时约束当成现行约束；涉「当前状态」判断一律
     `head` 读顶部。
- **另核（本轮复核既有结论，无需动作）**：v1.1.1 tag 经三方独立解引用一致指向
  `43f65fd2c`（首打 `2b4272a` 当日返工后移动，仅留一轮 cancelled 构建、未产出
  Release）；`AGENTS §7` IPC 清单 ↔ `ipc.rs::COMMANDS` ↔ `capabilities` 三处集合
  比对**无偏差**（闸门有效）；桌面端 `-` 与 `wsl` 相关未动。
### 2026-09-11 单元测试修复 · 补齐客体 bash 实跑单测的 #[cfg(unix)] 门禁并静音测试 ping 杂讯 —— guan（AI 协作）

- **背景与目标**：Windows CI runner 在执行 `cargo test` 时，因直接在 Windows 宿主环境（Git Bash）执行面向 WSL Linux 客体的 bash 脚本，导致 `profile_lifecycle`、`backup_file`、`diagnostics`、`session_lifecycle` 4 个 bash 实跑测试失败；同时 `shell.rs` 的测试模拟进程泄露 ping 终端输出。
- **变更清单**：
  1. `src-tauri/src/guest.rs`：对 4 个直接调用 `Command::new("bash")` 的客体脚本实跑单测补齐 `#[cfg(unix)]` 条件编译守卫，严格对齐既有 `read_files` / `list_dir` / `write_home_files` 的单测门禁规范（客体脚本实跑仅在 macOS/Linux unix 环境校验，Windows 宿主不具备真实 WSL 环境）；
  2. `src-tauri/src/shell.rs`：对测试中创建的 4 处 `cmd.exe /C ping` 模拟进程重定向 `stdout(Stdio::null()).stderr(Stdio::null())`，消除 Windows 测试控制台的 ping 回显杂讯。
- **影响**：仅周知，消除 Windows 平台 CI 单测失败与控制台杂讯。
- **验证**：Windows 目标 MinGW clippy 0 错误 0 警告，cargo fmt 通过，前端测试全绿。

### 2026-09-11 单元测试修复 · 修复 WSL 会话扫描脚本 BSD stat 与 Windows 路径反斜杠兼容性 —— guan（AI 协作）

- **背景与目标**：GitHub Actions CI 在 macOS 与 Windows runner 运行 `cargo test` 时，`guest::tests::session_lifecycle_scripts_run_correctly_in_bash` 出现跨平台兼容失败。macOS 环境因缺少 BSD stat 支持导致扫描输出空，Windows 环境因宿主 tempdir 路径含反斜杠导致 bash glob 匹配异常。
- **变更清单**：
  1. `src-tauri/src/guest.rs`：`list_sessions_script` 补充 `stat -f '%N|%z|%m'` 兜底分支，实现 GNU find / Busybox / BSD stat 三平台全兼容；对 `$dir` 增加 `${dir//\\//}` 规范化；
  2. `src-tauri/src/guest.rs`：`delete_session_script` 增加 `${target//\\//}` 与 `${root//\\//}` 路径规范化，支持 `sessions/*` 相对路径解析，消除反斜杠导致的 bash 转义匹配失败；
  3. `src-tauri/src/guest.rs`：单测补充绝对路径与相对路径双重校验。
- **影响**：仅周知，修复 CI 三平台单测闸门。
- **验证**：Windows MinGW 静态检查 0 错误 0 警告，单测覆盖 bash 下的绝对路径与相对路径会话删除。

### 2026-09-11 契约与规范同步 · ADR-0016 落地配套契约与 AGENTS 登记 —— guan（AI 协作）

- **背景与目标**：随 ADR-0016 P1/P2/P3 全量下沉至 WSL 客体，完成配套模块子契约落地、AGENTS.md 登记与 ADR 状态同步。
- **变更清单**：
  1. `docs/contracts/wsl-guest-management.md`：建立 WSL 客体管理面契约（v1），固化跨环境原语签名、世界判定 Seam、文件系统不变量（0600 凭据权限、原子覆盖、写前备份、排除 node_modules、会话目录防逃逸保护、stdin 管道投递规避 Windows 32K 命令行溢出）；
  2. `docs/contracts/README.md`：台账追加 `child-lifecycle` 与 `wsl-guest-management` 契约索引；
  3. `AGENTS.md`：§7 登记 WSL 客体管理面网络与进程用途，§9 索引 ADR-0016，精简已退役条目维持全文 ≤ 250 行预算；
  4. `docs/adr/0016-wsl-guest-management-plane.md`：标记文档同步项完成。
- **影响**：仅周知。客体管理面原语与不变量已作为稳定公共契约锁定。
- **验证**：`cargo clippy` 0 警告、`cargo fmt` 通过、前端类型与逻辑测试全绿。

### 2026-09-11 分支推送 · ADR-0016 P2 与 P3 全量下沉：WSL 客体模式管理面全部打通 —— guan（AI 协作）

- **背景与目标**：在 P1（插件链）与读侧下沉基础上，完成 ADR-0016 规划的 P2（profile 生命周期写动作与配置复制）和 P3（控制台面板与会话维护），彻底解除全部 `require_local` 阻断，让 WSL 客体模式具备与 Local 模式对等的完整管理能力。
- **变更清单**：
  1. **P2 Profile 生命周期写动作**（commit `ef5da8f`）：
     - `guest.rs`：新增 `copy_profile_script`、`rename_profile_script`、`delete_profile_script` 及其执行原语，排除 `node_modules` 保持原子高效迁移；
     - `profiles.rs`：实现 `create_profile_in_guest`、`copy_profile_in_guest`、`rename_profile_in_guest`、`delete_profile_in_guest`；纯函数抽离 `rewrite_manifest_name_text` 与 `scan_patch_relative_path_warnings`；
     - `plugins.rs`：实现 `copy_plugin_config_in_guest`，纯函数抽离 `apply_copy_config_entries`；
     - `commands/profile.rs` 与 `plugin.rs`：移除写路径上的 `require_local` 阻断，按 `World` 分发。
  2. **P3a 控制台管理下沉**：
     - `guest.rs`：`write_home_files_script` 补充 `chmod 600` 凭据权限安全保障；新增 `backup_file_script` / `backup_file` 实现写前带时间戳备份；新增 `diagnostics_script` 收集客体系统报告；
     - `credentials.rs`：纯函数 `parse_credentials_summary` 与 `apply_set_provider_key`；实现 `get_credentials_raw_in_guest`、`get_credentials_summary_in_guest`、`save_credentials_raw_in_guest`、`set_provider_key_in_guest`；
     - `dsh_settings.rs`：实现 `read_dsh_settings_in_guest` 与写前自动备份的 `overwrite_dsh_settings_in_guest`；
     - `mcp.rs`：纯函数内核 `parse_mcp_servers`、`apply_save_mcp_server`、`apply_delete_mcp_server`；实现 `list_mcp_servers_in_guest`、`save_mcp_server_in_guest`、`delete_mcp_server_in_guest`；
     - `diagnostics.rs`：实现客体诊断结果收集与报告生成；
     - `commands/console.rs`：全部 10 个控制台命令移除 `require_local` 阻断，接入 `World` 分发。
  3. **P3b 会话维护下沉**：
     - `guest.rs`：新增 `list_sessions_script` / `scan_sessions_raw_in_guest` 原语扫描会话文件；新增 `delete_session_script` / `delete_session_in_guest`（带根路径与逃逸校验安全防线）；新增 `run_repair_in_guest` 经 stdin 管道传递 94 KiB `repair-session.mjs` 规避 Windows 命令行 32K 长度截断风险；
     - `sessions.rs`：纯函数内核 `assemble_session_items`；实现 `read_archived_session_ids_in_guest`、`scan_sessions_in_guest`、`remove_session_in_guest`、`run_repair_in_guest`；
     - `commands/session.rs`：全部 4 个会话维护命令移除 `require_local` 阻断，接入 `World` 分发。
- **影响**：WSL 客体模式现已支持全部控制中心功能：Profile 管理（增/删/改/查/复制/切换）、插件管理（装/卸/更/配置/配置复制/开关）、控制台（凭据/DSH 设置/MCP/系统诊断）、会话维护（扫描/单会话自愈/全量自愈/删除）。宿主与客体逻辑严格保持单一解析与对等文件不变量。
- **待他人动作**：仅周知。
- **验证**：`cargo fmt --check` ✓、宿主 target clippy 0 警告 ✓、Windows target clippy（MinGW 桩工具链）0 警告 ✓、前端 32 组 223 个测试全绿 ✓、单元测试覆盖客体会话装配与 bash 脚本端到端执行。

### 2026-09-11 分支推送 · ADR-0016 第三批（读侧下沉）：WSL 模式下控制中心恢复可用 —— guan（AI 协作）

- **原委（实机反馈）**：第二批的 P0 守卫把 `list_profiles` 一并挡住，实机会话里控制中心着陆页
  直接报「profile 列表在 WSL 客体模式下暂不支持」——而**市场安装的目标 profile 选择器**也吃这条
  列表，等于把第二批刚打通的插件链**挡在门外**（分期设计疏漏：把"读"与"写"一起归进了 P2/P3）。
- **变更**：`guest.rs` 新增 `list_dir`（客体目录列举：base64 条目帧 + 目录/文件标志 + 目录不存在
  哨兵；显式覆盖点文件，与宿主 `read_dir` 同口径）；`profiles.rs` 抽出纯装配内核
  （`assemble_profile_summaries` / `assemble_profile_detail` / `ensure_default_candidate_from`）
  并加客体孪生（`scan_profiles_in_guest` / `read_profile_detail_in_guest` /
  `web_ui_profiles_in_guest` / `ensure_default_candidate_in_guest`）；
  `commands/profile.rs` 的列表、详情、切换候选、默认档校验全部按世界择源（**消掉第二批登记的
  `switch_profile` 边界**）；`plugins.rs` 补齐同类读侧断点——禁用/启用切换
  （patch 读改写抽成纯内核 + 客体原子写）、更新检查（已装版本取自客体清单）、总览聚合
  （客体扫描 + 客体清单）。
- **影响**：WSL 模式下控制中心**读侧全通**（profile 列表/详情/切换/默认档 + 插件清单/行表/开关/
  更新检查/聚合总览）；写侧（profile 创建/复制/重命名/删除、插件配置复制）与会话、控制台面板
  仍在 P0 诚实守卫下报「暂不支持 + 替代路径」。本地模式行为零变化。
- **待他人动作**：仅周知；仍未签名构建（覆盖安装，identifier 同）。
- **验证**：宿主 `cargo fmt --check` ✓ / `cargo clippy --all-targets -D warnings` ✓ /
  **windows 目标 clippy ✓（用上次广播的桩 C 工具链法，本轮把新 `#[cfg(windows)]` 代码也验了）**；
  离线 harness 实跑：profiles 37 · plugins+build_policy 84（仅 2 个需真引擎桩件的用例跑不了）·
  guest 9 · mgmt 6；`cargo test` 与链接仍只能在 CI（本机缺 webkit2gtk-4.1 dev）。

### 2026-09-11 补记 · Windows 目标 clippy 本地可跑（桩 C 工具链法）+ 首次 CI 红灯复盘 —— guan（AI 协作）

- **红灯**：上一批（`a870d2d`）推 CI 后 `build (windows-latest)` 在 **Rust clippy gate** 红 ——
  `src/guest.rs` 的 `write_home_files` 上限检查里 `content` 未参与诊断（只报路径），
  Windows 目标 `-D warnings` 判「unused variable」；macOS/Ubuntu 全绿（该函数体在非 Windows 被
  cfg 掉，宿主 clippy 看不到）。修复：`de74cd1`（改 `_` 绑定）。
- **教训（AGENTS §1 的又一处实证）**：`#[cfg(windows)]` 函数体的 lint 只有「Windows 目标 clippy」
  能看见；宿主 clippy 全绿**不蕴含** Windows 全绿——本批第一批的原语地基就是在 CI 上才补上这条。
- **可复用做法（本机无 mingw，但把 windows 目标 clippy 跑起来了）**：
  依赖里 `ring` 等 C 构建脚本要 `x86_64-w64-mingw32-gcc`，`tauri-winres` 还要 `windres`，
  缺工具链时 cargo 在依赖阶段就失败。用**假工具链**顶掉 C 编译即可让 Rust 侧 lint 全量生效
  （clippy 是 check-only，不链接，假 `.o`/`.a` 不影响判据）：
  ```bash
  # /tmp/<dir>/x86_64-w64-mingw32-{gcc,ar,windres} 皆为「解析 -o 后 touch 该文件再 exit 0」的脚本
  PATH=/tmp/<dir>:$PATH \
  CC_x86_64_pc_windows_gnu=/tmp/<dir>/x86_64-w64-mingw32-gcc \
  AR_x86_64_pc_windows_gnu=/tmp/<dir>/x86_64-w64-mingw32-ar \
  cargo clippy --target x86_64-pc-windows-gnu --all-targets -- -D warnings   # → Finished（零告警）
  ```
  边界（如实）：只验 Rust 侧编译与 lint（含 `#[cfg(windows)]` 函数体与测试目标），
  **二进制链接与运行仍只在 CI/真机**；桩件放 `/tmp`，不入库（AGENTS §8 不建无用 scripts/）。
- **影响**：仅周知；后续涉 Windows 分叉的改动，推 CI 前应本地跑一次上述命令，可省一轮红灯往返。

### 2026-09-11 分支推送 · ADR-0016 P1 第二批（a–e 接线）：控制中心在 WSL 模式下管到真正的世界 —— guan（AI 协作）

- **变更**（分支 `feat/wsl-guest-management-plane`）：新增 `src-tauri/src/mgmt.rs`（管理面世界择源 +
  P0 诚实兜底守卫）；`guest.rs`（读原语改「相对客体 dsh home」，新增 base64 原子写原语）；
  `build_policy.rs`（客体侧单键写入孪生）；`plugins.rs`（三处择源，解析/装配/分类抽成两侧共用的纯函数）；
  `commands/{plugin,profile,session,console}.rs`（入口择源 / `require_local` 守卫）；
  `ui.rs`（`current_active_mode` 去 macOS cfg）；`frontend/src/{lib/profiles.ts,stores/profilesStore.ts}`
  （列表失败透出后端详情，不再被固定话术盖掉）；`docs/adr/0016-*.md`（§5 进度回填）。
- **影响**：WSL 模式下「市场插件安装/卸载/更新 + 插件清单 + 插件行表」现在打在**客体**
  （客体 dsh CLI + 客体读原语），并补写客体 profile 的 `dangerouslyAllowAllBuilds: true`
  （复用 ADR-0013 单键口径；客体侧 base64 载荷 → 同目录 tmp → `mv` 原子替换，父目录不存在即失败，
  不代 dsh 生成 profile 目录）。其余未下沉动作（profile 列表/详情/CRUD/默认档、会话四命令、
  控制台凭据·DSH 设置·MCP·诊断、插件开关、配置复制、更新检查、聚合总览）在 WSL 世界一律返回
  「暂不支持 + 替代路径」——**不再出现「请先启动应用完成引擎引导后重试」这类与 ADR-0004 矛盾的
  死路提示**（禁语有单测闸门）。本地（含非 Windows）路径行为零变化。
- **待他人动作**：仅周知；本批仍是**未签名**构建，装上会覆盖现有安装（identifier 相同）。
- **验证（如实登记边界）**：`cargo fmt --check` ✓；`cargo clippy --all-targets -- -D warnings`（宿主 Linux）✓；
  前端 `typecheck` + `lint` + `test`（223 tests）✓。本机 WSL **缺 webkit2gtk-4.1 与 mingw 工具链**，
  `cargo test` 与 windows 目标 clippy 在本机跑不了——用不入库的离线 harness（`rustc --test` + 最小桩件）
  实跑了 guest 7 / mgmt 6 / build_policy 18 / plugins 31 个用例（含两处**在 bash 里真跑脚本**的用例，
  抓到并修掉了两处测试自身的路径基址错误）；`#[cfg(windows)]` 函数体与三平台全量用例仍以 CI 为准。
- **未做（登记动作项）**：AGENTS §7 登记本次客体管理面网络用途；`docs/contracts/` 增客体管理面子契约
  （原语与文件不变量对账）；`docs/executor.md` 补 Windows+WSL 实机验证清单（插件装/卸/更 + 行表 +
  错误面六态：`wsl.exe` 不可用 / 无发行版 / musl / 断网 / 客体 home 不存在 / 发行版未选定）。
- **已知边界**：`switch_profile` 的 webUi 候选校验仍读宿主 home——控制中心在 WSL 模式已无 profile
  列表（P0 守卫），该路径在 WSL 世界不可达；profile 列表下沉时一并改按世界择源（已登记 ADR-0016 §5）。

### 2026-09-11 补记 · `gh` token 已补 `workflow` scope（上条遗留动作项闭环）—— guan

- **原委**：上一条（依赖升级处置）登记了本机 `gh` token 缺 `workflow` scope、导致
  `gh pr merge` 无法合并任何改动 `.github/workflows/` 的 PR，并留下动作项
  `gh auth refresh -s workflow`。维护者已执行。
- **核实**：`gh auth status` 现为
  `'admin:public_key', 'gist', 'read:org', 'repo', 'workflow'`；账号 realguan、
  git 协议仍为 ssh（`gh auth refresh` 仅重签 OAuth token，不影响 SSH 推送）。
- **核实边界（如实登记）**：本次只在 **scope 层**核实（即 GitHub 检查所依据的字符串，
  也正是原报错点名的那个 scope），**未端到端实证一次真实合并**——当前无开启中的
  workflow 类 PR，且不为此制造测试 PR。下一个 Dependabot workflow PR 合并时即为实证点。
- **另核**：三个已关闭 PR 未被 Dependabot 重新提起，说明它读得懂行内 `# v6.1.0` 注释、
  视该依赖为最新，故不会重复提案。
- **影响**：仅周知。后续 workflow 类依赖 PR 可正常走 `gh pr merge`（正常标记为 merged，
  不再需要"关闭 + 说明"的绕行）。

### 2026-09-11 依赖升级 · 三个 Dependabot PR 处置（合 2 · 改 1 后关闭）—— guan（AI 协作）

- **背景**：GitHub 积压 3 个 Dependabot PR（#10 pnpm/action-setup、#11 actions/checkout、
  #12 taiki-e/install-action），全部 CI 绿且 MERGEABLE。逐个核验后**没有照单全收**。
- **落地**：`c879d3c`（pnpm/action-setup v4 → v6.1.0）· `542275e`（taiki-e/install-action
  2.87.2 → 2.87.6）· `2f8db5c`（spike 文件 checkout 改固定 SHA）。三个 PR 均已带说明关闭，
  Dependabot 分支已清理。
- **#10 采纳理由（不是"版本新"，是支持范围错配）**：上游大版本依次为 v5.0.0 = 换 node24
  运行时、v6.0.0 = 支持 pnpm 11、v6.1.0 = 支持 pnpm 12（v6.1.0 唯一改动）。而四个作业
  pin 的都是 `version: "12.3.1"`、`frontend/package.json` 的 `packageManager` 亦为
  `pnpm@12.3.1` —— 等于长期用"早于 pnpm 11 就冻结"的 action 去装 pnpm 12。该线有前科：
  `30fb277` 记录该 action 的 SHA pin 上游失效，三平台 CI 自 09-05 起 3~5 秒死于 Set up job。
  新 SHA 经 `gh api` 复核可解析（HTTP 200）。
- **#12 采纳但属 no-op（已取证）**：两个 pin 的 `manifests/cargo-llvm-cov.json` **逐字节
  相同**（`diff` 为空）；2.87.3~2.87.6 的更新只落在未使用工具（zizmor / uv / typos / biome），
  升 `cargo-llvm-cov` 的是 2.87.7。价值仅在于不让更新 PR 堆积、避免下次跨度过大。
- **#11 不按原样合**：它标题写 "4 → 7"，实际只动 `spike-0003-verify.yml` 一行（其余 8 处
  早已是 v7.0.1），但它给的是**浮动 tag** `@v7` —— 与 `.github/dependabot.yml` 写明的
  「工作流 action 固定完整 SHA」纪律相悖。改为手工 pin 到与其余一致的
  `3d3c42e5… # v7.0.1`。成因是遗漏而非决策：checkout 7.0.1 于 09-01 合并（`d2cf73f`），
  而该文件 09-04 才新增（`eb50f7f`）。改后全仓 `uses:` 已无浮动 tag。
- **凭据（实测，非仅绿灯）**：`build` run 34559781660 —— 三平台 build + coverage 全 success
  （release skipped，非 tag 推送，未触发行）；`spike-0003-verify` run 34559781694 ——
  engine-bootstrap ×3 + wsl-delivery-channel 全 success，且 step 日志实证
  `Run actions/checkout@3d3c42e5…` 在三平台均执行成功（**该行此前从未在本工作流跑过**）。
- **流程教训（新，AI 犯后自纠）**：**"PR 绿" ≠ "覆盖了本 PR 的改动"**，必须按 run 的
  `head_sha` + `path`（workflow 文件）归属核对——否则会把 `boot-smoke` 的作业名
  误认成别的 workflow，得出"该 PR 的改动没被验证"的错误结论，或反之。本次仍应对
  Dependabot 的浮动 tag 保持警惕：**Dependabot 会沿用该行原有的 ref 风格**——行内本是
  浮动 tag 时它就给浮动 tag，不会自动升级为 SHA 固定。
- **工具限制（新发现，影响后续每周 Dependabot PR）**：本机 `gh` token scope 为
  `admin:public_key / gist / read:org / repo`，**缺 `workflow`** → `gh pr merge` 对任何
  改动 `.github/workflows/` 的 PR 会被 GraphQL 拒绝
  （`refusing to allow an OAuth App to create or update workflow`）。本次改经 **SSH 推送**
  落地（远端为 `git@github.com:`，推送不走 OAuth token），内容与 squash 合并等价，PR 以
  说明关闭。若希望 PR 正常标记为 merged，需 `gh auth refresh -s workflow`。
- **残留未覆盖（如实登记）**：`boot-smoke.yml` 的 3 处 pnpm pin **从未运行过**——该工作流
  仅 `workflow_dispatch` 触发，且 `windows-*` 两个作业是 `continue-on-error` 的实验档
  （近期 4 失败 1 成功）。同一 action SHA 与同一 `version: "12.3.1"` 已由 build.yml 三平台
  实证，故未额外手动触发；如需 boot 路径端到端覆盖，手动 `gh workflow run boot-smoke.yml`。
- **影响**：仅周知。workflow 改动不影响已发布的 v1.1.1 产物（产物由 tag 构建）。

### 2026-09-11 发版返工 · v1.1.1 首次构建红（Windows clippy）→ 修复后重打 tag —— guan（AI 协作）

- **事实**：v1.1.1 tag 推后，`build (windows-latest)` 的 **`Rust clippy gate`** 失败：
  ```
  error: casting raw pointers to the same type and constness is unnecessary
         (`*mut c_void` -> `*mut c_void`)
    --> src/lifecycle.rs:331  child.as_raw_handle() as *mut core::ffi::c_void
  ```
  （Windows Job Object FFI；`RawHandle` 本身就是 `*mut c_void`）。ubuntu 作业 success、
  macOS 作业被取消（fail-fast）。**Release 从未发布**（当时最新仍是 v1.1.0）。
- **漏检根因（流程教训，已写进 AGENTS §1）**：我在 Windows 侧只跑了
  `cargo check --target x86_64-pc-windows-gnu`——**`check` 不跑 clippy lint**，而 CI
  三平台跑的是 `cargo clippy --all-targets -- -D warnings`。`cfg(windows)` 分支在
  macOS 上根本不编译，于是该 lint 对本地完全不可见，直到 CI 才暴露。
  宪法新增一行：**clippy 需逐目标各跑一次，`cargo check` 不能替代**。
- **修复**：`84a6ac3`（去掉多余 cast）。已按 CI 同款命令复验：
  clippy host(macos) 0 issues · clippy `--target x86_64-pc-windows-gnu` 0 issues ·
  `cargo test` 285 绿 · 前端 typecheck 0 err / oxlint 0 warning / test 221 绿。
  （Linux 目标无法在 macOS 上跑 clippy：glib-sys/gdk-sys 缺 WebKitGTK pkg-config，
  属环境限制；本次改动无 Linux 专属分支，且 CI ubuntu 作业已 success。）
- **重发策略（维护者裁定）**：**强制移动 `v1.1.1` 到修复提交**——理由：内容确实就是
  1.1.1、Release 从未发布过（不会留幽灵版本号）、tag 存在仅约 30 分钟且实际只有 CI
  与本地 fetch 过。**这是本仓库首次移动已推 tag，故显式登记**；此后若再遇同类情形，
  默认仍应优先考虑"改版本号重发"而非移 tag。
- **另注**：`gh` 可用（`/opt/homebrew/bin/gh`，账号 realguan 已登录）——首次排查时
  我因 PATH 未含 homebrew 而误判为"gh 不可用"，实际本机 PATH 一直缺 `/opt/homebrew/bin`
  （同一天 Windows 交叉编译的 windres 也是这个原因）。教训：**PATH 类结论必须先
  显式补全常见 bin 目录再下判断**。
- **第二次返工（同日）**：clippy 修好后 CI 继续跑，windows-latest 的 `Unit tests`
  又红，两条**都是真 bug**（非笔误），且都只在 Windows 暴露：
  1. **源码闸门被 CRLF 打穿**：`production_spawns_go_through_lifecycle_seam` 的模式
     写死 LF，而仓库**没有 `.gitattributes`**——Git for Windows 的 `autocrlf` 把源码
     checkout 成 CRLF → 测试模块截断失效 → 测试里的裸 spawn 被误报成生产违规，
     **闸门在 Windows 上恒红**（维护者本地跑 cargo test 必撞）。修：抽
     `scan_unguarded_spawns` 纯函数并先把 `\r\n` 归一到 `\n`，补 CRLF/LF 双跑回归
     （变异验证：还原归一即红，且**精确复现 CI 的那条误报**）。
  2. **持锁期间写文件**：Windows 的 `LockFileEx` 是**强制锁**，持锁时另一句柄写不进去
     （os error 33）；macOS 的 `flock` 是劝告锁故绿。修：先写内容再加锁。
  修复提交 `413b708`。
- **防复发性加固**：新增 `.gitattributes`（`* text=auto eol=lf` + 二进制声明，
  `43f65fd`）——从根上消除"本地 checkout 换行 ≠ CI"这类只在一端暴露的偏差。
  **零 churn 验证**：`git add --renormalize .` 后暂存区仅该文件本身（索引原本即 LF），
  无大 diff、无历史改写。
- **收尾方式（维护者裁定）**：删除指向坏提交的 tag（Release 从未发布，可安全删）→
  **先把修复推 master 让 CI 三平台验一遍 → 验绿再打 tag**（不再"先打后赌"）。
  master 三平台全绿（含 windows-latest）后重打 `v1.1.1` 并推送。
- **✅ 发版成功（2026-09-11T03:30:00Z）**：三平台构建 + `release` 作业**全绿**；
  Release 已发布（非 draft / 非 prerelease）：
  <https://github.com/realguan/dsh-dock/releases/tag/v1.1.1>
  - 产物 14 件：macOS `.dmg`(aarch64) / `.app.tar.gz`、Windows `.msi` + `-setup.exe`、
    Linux `.deb` / `.rpm` / `.AppImage`，各平台 updater `.sig`，以及 `latest.json`。
  - `latest.json` 已核验：version=`v1.1.1`，六平台条目（darwin-aarch64 /
    windows-x86_64-msi / windows-x86_64-nsis / linux-x86_64-appimage / -deb / -rpm）
    url 与 signature 齐全——客户端「检查更新」数据源正确。
  - Release 正文已核验：1332 字符，首行为强契约标题 `## [v1.1.1] - 2026-09-11`。
- **冻结期解除（三平台产物验收通过）**：master 恢复收 feat。
  仍挂着的**唯一未验证项 = Windows 真机人工验证**（安装包已出，但"普通账户首启
  三卡就绪 / 胶囊开控制中心 / 重启无 ERR_CONNECTION_REFUSED / 强杀后不卡落位"
  这四项只有真机能确认）——清单见 `docs/executor.md`，待维护者用 Windows 笔记本跑。

### 2026-09-11 发版 · v1.1.1 会话占用根治与 Windows 首启修复 —— guan（AI 协作）

- **范围**：`v1.1.0..HEAD` 共 6 笔提交，全部为**修复**（无用户可感知新能力，
  故定 patch）。三条主线：
  ① **会话被占用／打不开的根治**（ADR-0015 子进程生命周期：硬杀收口 + 启动期
     孤儿清扫 + spawn 单点 seam）；
  ② **Windows 普通账户首启修复**（引擎引导改免符号链接布局，连带修掉「DSH 未检出／
     健康大盘全空／插件装不上」同一根因的一批现象），另含壳页地址 dev 门、
     控制中心建窗线程、pnpm 落位加固；
  ③ **测试与生产隔离**（审核期发现真机用例会打开用户真实 profile 并顶掉正式包会话，
     已修并加闸门）。
- **tag**：`v1.1.1`（待推）。版本号四处同步（Cargo.toml / tauri.conf.json /
  frontend/package.json / Cargo.lock）；`docs/RELEASE_NOTES.md` 顶部已落
  `## [v1.1.1] - 2026-09-11`（AGENTS §8.8 强契约）。
- **本地已复现 CI 两道 tag 闸门**：三处版本一致性 ✅；发布日志存在性 ✅；
  `scripts/extract-release-notes.py v1.1.1 --strict` 提取成功（1331 字符，
  未误伤 v1.1.0）。
- **凭据**：`cargo test` 285 绿（3 个 `#[ignore]` 真机锚不进默认套件）；`fmt --check`
  绿；`clippy --all-targets -D warnings` macOS 与 `x86_64-pc-windows-gnu` **双平台
  0 warning**；前端 `typecheck` 0 err · `oxlint` 0 warning · `test` 221 绿。
- **⚠️ 冻结期起：Release notes 已落盘 → 至三平台产物验收通过为止，master 只收 fix。**
- **⚠️ 未验证项（发版不免除，必须显式登记）**：**Windows 真机未验**。本机仅有
  macOS，本批 Windows 相关修复（免符号链接布局 / 建窗线程 / 死端口 / 落位加固）
  只在 macOS 端到端 + `x86_64-pc-windows-gnu` 类型检查层面验证过。验收流程已落
  `docs/executor.md` §「Windows 实机验证清单」（A 引擎引导 / B 真机 7 项 / C 孤儿
  收口），**待维护者用 Windows 笔记本跑**；跑完请回填该表。
  另：ADR-0015 的 Windows Job Object 路径同理未在真机验证（已在 ADR §5 登记为已知边界）。

### 2026-09-10 审核 · 上两笔修复的自查（含 3 处安全/覆盖缺口修补） —— guan（AI 协作）

- **触发**：维护者问「你需要再审核一下本次 bug 修复情况吗」——对 `8baffb7` /
  `0f1a186` 做对抗式自查，**发现并修掉 3 处真实缺口**（都不是编译期能发现的）：
  1. **清扫判定缺一档 → 存在误杀风险**（安全方向，最严重）。`File::try_lock` 返回的
     `TryLockError` 有 `WouldBlock`（明确被持有）与 `Error(io)`（锁机制本身失败：文件
     系统不支持 flock / I/O 故障）**两档**，首版一律按"仍被持有"处理 → 在不支持 flock
     的盘上**每条登记都会被判成活跃孤儿**，再按文件里的 pid 去 `SIGTERM`（pid 可能已
     复用给无关进程）。现拆出 `classify_lock_result` 三态纯函数，`Error(io)` →
     **跳过敏收**（不确定时宁可不收口；误杀无法挽回），并加 2 例闸门。
  2. **引导接线无覆盖 → 事故会静默复发**。变异测试证实：把 `lib.rs` 里那行
     `sweep_orphans` 删掉，**全部测试照样绿**。现加源码文本闸门
     `boot_sweeps_orphans_before_dispatching_executor`，同时钉住**顺序**（清扫必须早于
     派发执行器，否则与新一代探测竞争同一 profile 目录）。已验证删掉那行即红。
  3. **spawn 闸门漏两个入口**：原只查 `.spawn()`/`.output()`，漏 `status()` /
     `wait_with_output()`。补齐后发现 1 处命中（`updater.rs` 的
     `reqwest::Error::status()`，非子进程）→ 用显式豁免标注留痕，并把豁免语法扩为
     「本行或紧邻上一行」（后者更贴近 `#[allow(...)]` 惯例）。
- **另修**：关于页未安装态曾同时渲染「未安装（计划 X）」与「尚未确定」，自相矛盾
  → 去掉后者并回收死键 `nodeUnknown`（双语字典对称，`i18nStore` 闸门绿）。
- **新增验证资产**：
  - **真 dsh 硬杀端到端锚**（`real_dsh_is_reaped_when_shell_is_sigkilled`，`#[ignore]`）
    ——走**生产同一条 `spawn_dsh`** 起真 dsh，对壳发 `SIGKILL`，断言真 dsh 被生命线收口。
    这是用户事故（会话说锁被孤儿占死）的逐字复刻，此前只有 `sleep` 替身。
  - **变异测试自检**（新增验证纪律，已写入契约 §7）：关生命线 → 真 dsh 用例红
    （并真的留下孤儿，已回收）；删清扫接线 → 接线闸门红；去掉 dev 门 → release 用例红。
    **三条都确认能证伪**，故此前的绿灯不是"没测到"。
- **影响**：仅周知。契约新增 §2.3（三态判定）与测试闸门 9–12、§7 验证纪律；
  无契约字段 / IPC / 载荷形状变更。
- **⚠️ 事故（本次审核自身造成的生产影响，必须留痕）**：审核过程中新增的"真 dsh"
  用例**干扰了维护者正在运行的正式包**——02:05:30 正式包日志出现
  `等待服务响应超时` → `优雅停止超时（3s），SIGKILL`，会话被打断并重启（02:06:22
  自愈，现正常）。**根因**：本机环境导出了 `DSH_HOME=~/.dsh`（dsh 自身运行环境就会
  导出它），而 `user_dsh_home()` 把 `DSH_HOME` 当"用户主权"最高优先 → 测试继承了它
  → 真机用例**直接打开用户的真实 `web` profile**，与正式包抢同一 profile。
  **这与本次事故（孤儿占锁）是同一类错误的另一面：隔离没做在"测试不得碰生产"上。**
  已修：测试构建**一律无视 `DSH_HOME`**，锁进 `~/.dsh-dock-test`（`resolve.rs` 的
  `cfg(test)` 分支），并加闸门 `test_build_never_targets_the_real_user_home`
  （变异验证：改回尊重 `DSH_HOME` 即红）。用户数据未受影响（profiles/sessions 完好，
  中断的会话由正式包自动重启恢复）。
- **凭据**：`cargo test --lib` 285 绿（本轮 +5）；`fmt --check` 绿；
  `clippy --all-targets -D warnings` 在 macOS 与 `x86_64-pc-windows-gnu` **双平台
  0 warning**；前端 `typecheck`/`lint`/`test` 221 绿；两条真机锚（真 dsh 硬杀收口、
  真 dsh 经守卫 spawn→就绪→收口）在**隔离 home** 下复测绿；实测确认被 spawn 的 dsh
  其 `DSH_HOME` 已是 `~/.dsh-dock-test`。

### 2026-09-10 fix(v110)：Windows 实测 7 项问题修复——引擎引导免符号链接 + 壳页地址 dev 门 + 控制中心建窗线程 + pnpm 落位加固 —— guan（AI 协作）

- **背景**：`docs/known-issues/v110-测试问题记录.md`（v1.1.0 Windows 实机 9 张截图）。
  定位全文见 `docs/known-issues/v110-测试问题定位.md`。7 项里 **5 项同源**——
  引擎引导在普通权限 Windows 上必然失败，下游表现为「DSH 未检出 / 健康大盘全缺 /
  插件安装报引擎未就绪 / 关于页版本号对不上」。
- **变更**（按根因）：
  1. **引擎引导免符号链接布局**（实测 1.2/1.4/1.7 的根因）：`build_policy.rs` 新增
     `ensure_engine_linker`，引导开头向引擎目录幂等写 `nodeLinker: hoisted`
     （复用 ADR-0013 同一套 YAML 解析/拒写纪律，只动这一个顶层键）。pnpm 默认的
     `isolated` 布局要建**目录符号链接**，而 Windows 普通账户无
     `SeCreateSymbolicLinkPrivilege` → `os error 5`；hoisted 是扁平真实目录，
     不需要任何链接。**平台无关地写**（幂等、三平台一致）。
  2. **`find_runtime_node_bin` 认 Windows 布局**：候选路径补「包根 `node.exe`」
     （官方 Windows 份是 zip 变体，`node.exe` 在包根无 `bin/`，spike 0003 §2.7
     已记录该分叉）。旧实现只认 `bin/`，Windows 上必然找不到 → `engines/bin/node`
     退化成会让 postinstall 报 `ERR_PNPM_SHIM_NO_TARGET` 的 shim。
  3. **`shell_app_url` 加 dev 门**（实测 1.6）：`devUrl` 会被编译进 **release**
     二进制，旧判据 `dev_url.is_some()` 恒真 → 切 profile/切模式/崩溃自恢复全把主
     窗口导航到已死的 Vite 端口（`localhost 拒绝连接`）。改用 `cfg!(dev)`
     ——与 Tauri 自己解析 `WebviewUrl::App` 的口径**同源**。抽纯函数 + 4 例测试。
  4. **窗口创建移出 WebView2 回调**（实测 1.3）：新增 `ui::post_to_event_loop`，
     先跳独立线程再 `run_on_main_thread`，保证**永不就地执行**
     （`send_user_message` 在主线程时是就地执行，不是"排队到下一帧"——旧注释写错）。
     就地执行会让 `build()` 在 WebView2 回调里跑嵌套消息泵 → 白窗 + 卡死 + 关不掉；
     托盘入口在 tao 事件循环故正常，正是实测的分叉现象。
  5. **pnpm 落位加固**（实测 2.0）：暂存目录带唯一后缀（旧固定名会被并发轮次互删）
     → 先删后放 + 短重试 → 仍失败回落**版本化文件名**（从根上绕开"目标被残留进程
     占用不可覆盖"）→ `find_engine_tool` 认该回退形态 → 错误链 `{e:#}` 展开
     （旧实现只打印最外层上下文，`os error` 全丢）。
  6. **关于页不再说谎**（实测 1.2 的追问）：`NodeRuntimeInfo` 拆
     `version`（实测，未装 = null）与 `plannedVersion`（计划）——旧实现拿**下载计划
     版本**当已装版本渲染成「v24.18.0 · 应用托管」，与健康大盘的「未检出」自相矛盾。
  7. **展示层**：`updates::display_version` 剥 `v` 前缀（修 `node vv24.18.0`）；
     BootIndex「查看启动详情」的收起判据抽 `lib/bootTimeline.ts`——旧条件只看
     `hasEverDownloaded`，而 WSL 客体引导**只发 boot:step 不发 boot:progress**，
     该标志恒假 → 点开详情后**没有任何收起出口**；控制中心窗口底色改引
     `WINDOW_BACKGROUND` 常量并纳入批次 E 闸门（消灭第四处真相源）。
- **影响**：仅周知；无契约破坏、无 IPC 变更。`NodeRuntimeInfo` 增 `plannedVersion`
  字段（Rust + 前端类型 + mock 同步；`version` 由 `String` 变 `Option<String>`，
  属**载荷形状变更**——同包内两端同步，无外部消费方）。
- **凭据**：`cargo test --lib` 280 绿（本轮新增 20：nodeLinker 6、node 布局 2、
  落位回退/发现 4、shell_app_url 4、display_version 1、窗口底色闸门 1、其余为
  ADR-0015 配套）；`cargo fmt --check` 绿；`clippy --all-targets -D warnings`
  绿（macOS + `x86_64-pc-windows-gnu` **双平台 0 warning** —— Windows 侧曾因
  unix-only 测试助手报 dead_code，已按平台正确 gate）；前端 `typecheck`/`lint`/
  `test` 221 绿（新增 bootTimeline 7 例）；**引擎引导端到端**（`--ignored`：空数据
  目录跑完整引导）绿——实测确认 `nodeLinker: hoisted` 下 `node_modules/node` 是
  **真实目录而非符号链接**，node/dsh 均就绪可执行。
- **未做（诚实登记）**：① Windows 实机复验（本机只有 macOS；上述 1/2/5 的修复
  机制已在 macOS 端到端验证 + `x86_64-pc-windows-gnu` 类型检查，但**真机行为仍需
  维护者跑一次**——建议用 `cargo test --lib engine_bootstrap_uses_symlink_free_layout
  -- --ignored`，在**未开开发者模式**的 Windows 上跑，旧实现必红、新实现应绿）；
  ② 1.5 进程数量分析（结论：WebView2 多进程 + 会话 node + 隐藏 conhost，量级正常，
  唯一异常项是 1.3 那条「无响应」，已随本批修复）。

### 2026-09-10 宪法级（AGENTS §6/§9）· ADR-0015 子进程生命周期归属——硬杀收口 + 孤儿清扫 —— guan（AI 协作）

- **变更**：新增 `docs/adr/0015-child-process-lifecycle-ownership.md`（已接受）+ 配套
  `docs/contracts/child-lifecycle.md` + `src-tauri/src/lifecycle.rs`（新模块）；
  `child_cmd` 之外新增**唯一 spawn seam**（`lifecycle::spawn`/`run`），全部 19 处生产
  spawn 面收敛到它；`boot.rs` probe 前挂启动期清扫；`resolve.rs::user_dsh_home` 加
  **dev/prod 隔离**（`cargo build`/`test` → `~/.dsh-dock-dev`，release → `~/.dsh`）；
  `nix` 增 `fs` feature（既有依赖加 feature，未新增 crate）；Windows 走裸
  `extern "system"` Job Object（`KILL_ON_JOB_CLOSE`，未引 `windows-sys`）。
- **触发**：用户报「会话点不开，弹 `SessionAlreadyOwnedError … (gateway/internal)`，
  删会话 + 重启才恢复，且最近经常出现」。根因 = 11 个 `PPID=1` 的逃逸 dsh 持着
  `session.lock` 内核写锁（锁属 dsh 协议、刻意无过期；孤儿属壳侧违约）。诊断见
  `docs/known-issues/问题记录-2026-09-10-会话写锁被孤儿dsh占死.md`，dsh 侧行为入册
  复现点 14（锁语义）+ 15（守卫依赖的 OS 行为）。
- **影响**（需他人知悉）：
  1. **AGENTS §6 不变量扩展**（宪法级）：1:1 生命周期从"退出/崩溃"扩到"**含硬杀**"；
     §9 索引新增 0015 行；
  2. **新增 spawn 必须经 `lifecycle::spawn`/`run`**，裸 `Command::spawn()`/`.output()`
     被机器闸门（`production_spawns_go_through_lifecycle_seam`）拦下；豁免需在该行写
     `// spawn-gate: exempt(<理由>)`；
  3. **dev 与 release 的 `DSH_HOME` 从此不同**——开发期数据落在 `~/.dsh-dock-dev`，
     不再污染用户的正式会话（这正是本次事故的放大器）；
  4. `run()` 语义 = `Command::output()` 的守卫版（自己设 piped stdio + 退出后自清登记）。
- **凭据**：`cargo test --lib` 262 绿（lifecycle 模块新增 18 例：清扫正/负例、复现锚 + 对照组、
  fd/flock 语义、脚本契约、spawn 闸门、RAII 存活性）；`cargo fmt --check` + `clippy -D warnings`
  绿；`cargo check --target x86_64-pc-windows-gnu` 绿（该目标 `cargo build` 的
  `export ordinal too large` 为 mingw cdylib **既有**限制，已在基线复核确认非本次引入）；
  前端 `typecheck`/`lint`/`test`（214）绿；真机引擎端到端（`--ignored` 用例：真 dsh
  经守卫 spawn → 就绪 → 收口）绿；18 例 lifecycle 全绿 3.0s；**端到端硬杀验证**：
  壳被 `SIGKILL` 后守卫子进程 ~0s 内被 watcher 收口。
- **实施期推翻的两条设计细节**（已写入 ADR-0015 §7.1 / 契约 §2.2，防后人重蹈）：
  ① 显式 `flock(LOCK_UN)` 会**连同子进程的持有一起撤销**（锁属打开文件描述）→
  守卫改为只 close 绝不 unlock；② `pre_exec` + 固定 fd 号的 lifeline 通道在实机上让
  `read` 立即失败、watcher **误杀正常运行的真 dsh**（症状：一启动就吃 SIGTERM）→
  改用 `Stdio` 通道，并把失败方向改为"读不到就放弃收口、绝不误杀"；对应验收测试已加
  **观察窗**，否则"误杀"会冒充"收口"而假绿。
- **未做（登记待议）**：P0′ 会话维护显示持锁者 + 一键结束占用；安全模式（插件致崩后
  引导禁用）；`scripts/repair-session.mjs` 不参与锁协议的边界观察。

### 2026-09-10 发版 · v1.1.0 重启交接带与设计 token 收口 —— guan（AI 协作）

- **范围**：`v1.0.0..v1.1.0` 共 10 笔提交，两条主线（ADR-0014 重启/切换交接带 +
  设计 token 收口），另含排查中发现的子进程逃逸修复。版本定 minor：含用户可感知的
  新能力（交接带）。
- **tag**：`v1.1.0`（注解 tag，content sha `ca8dc81`，指向 `afbed7c`）已推 origin；
  master 同步至 `afbed7c`。
- **发布日志**：`docs/RELEASE_NOTES.md` `## [v1.1.0] - 2026-09-10`（AGENTS §8.8 强契约），
  经 `scripts/extract-release-notes.py v1.1.0 --strict` 提取通过（1614 字符，
  精确匹配、未误伤 v1.0.0）。
- **CI 验收（已通过）**：
  - `build` run [34470150340](https://github.com/realguan/dsh-dock/actions/runs/34470150340)
    三平台 **全绿**（macos-latest / ubuntu-latest / windows-latest），
    其中 `Verify tag version consistency` 与 `Extract Release Notes` 两道 tag 闸门通过；
  - `spike-0003-verify` run [34470150386](https://github.com/realguan/dsh-dock/actions/runs/34470150386) 通过；
  - master 的 build run [34470104166](https://github.com/realguan/dsh-dock/actions/runs/34470104166) 通过。
- **Release 产物（14 个，GitHub Release v1.1.0 已发布）**：dmg 21.8 MB · app.tar.gz 20.3 MB ·
  exe 39.0 MB · msi 39.7 MB · deb/rpm 22.8 MB · AppImage 94.4 MB（含各 `.sig` 与
  `latest.json` 自更新清单，6 个平台条目齐全）。
- **冻结期**：**自本条目起至三平台产物验收通过止**——master 只收 fix，不收 feat
  （CONTRIBUTING §8 / `docs/contracts/README.md` §冻结期）。
- **待人工验收**（发版前未在真机跑过，浏览器预览已逐屏核对）：
  ① 重启/切换全程的 loading 连续性与计时不归零（macOS 真窗口）；② Windows 重启后
  `tasklist | findstr node` 无遗留 dsh；③ 双击重启 / 重启与模式切换竞态只起一个 dsh；
  ④ WSL 客体 `ps aux | grep dsh` 干净；⑤ 新 elevation/圆角观感与深色日志面板整体感。
  清单见 ADR-0014 §5 与 `docs/executor.md`。
- 凭据：本地闸门 Rust **244** 测试 + fmt/clippy 绿；前端 typecheck 0 err ·
  oxlint 0 warning · **214** 测试绿；四处版本号一致（CI 闸门本地已预演）。


### 2026-09-10 fix(uiux)：批次 E 对抗式审查修复（bde32ae）—— 失败态误用警告色 + 闸门五处可绕过 —— guan（AI 起草）

- **方法**：批次 E 完成后请独立子 agent 做对抗式复核（目标是**证伪**而非确认），
  产出 22 条，逐条核实后修复 21 条、登记 1 条。核心发现：**批次 E 修好了「成功/
  进行中」的同义多色，却让「失败」继承了旧 warn**——token 层把 warn/danger 拆开了，
  但既有 warn 调用点从未按新语义重判；旧 `--color-warn` 是红橙 #c2410c（语义兼容
  失败），新 warn 是琥珀 #8d4e10（纯「需注意」），于是真正的失败渲染成琥珀、别处
  错误是绛红，「一个概念两个色」只是从「成功」搬到了「失败」。
- 修复（`bde32ae`，31 文件）：① 失败态全线改 danger（ErrorCard 启动失败卡、
  BootTimeline error、ClientUpdateCard failed、DshVersionListDialog 加载失败、
  PluginImportPickerDialog 导入失败、confirm-dialog 的 error），并拆开
  ProfileCreateDialog 里 pending 与 failed **逐字节相同**的死三元；
  ② npm 来源徽标**第三处**漏改（MarketCustomInstallDialog 仍成功绿，与同框
  「已安装」撞色）；③ 状态 token 被当分类色（诊断页存储条用 `bg-ok` 表示
  「会话数据」分类）→ 新增组成图色阶 `chart-1/2/3`（同色相梯度 + 中性收尾——
  状态四族已占满色环，新色相必然撞车，实测最小 ΔOKLab 低至 0.007）；
  ④ `switcher.js`（注入 dsh 页面的**用户可见**悬浮胶囊）整个在收口之外；
  ⑤ 新增 `term-brand`（`brand-deep` 在 term 底上仅 3.40，文字不可读）。
- **闸门五处可绕过（均为实证）**：shape/contrast 只扫 `.tsx` 而类名工厂在 `.ts`；
  任意值与裸色值全放行（`bg-[#047857]`、`.css` 里的 hex、`style={{color:"tomato"}}`、
  方向性边框 `border-s-rose-500`、`ring-offset-amber-500`）；裸 `rounded`（隐式
  4px 第四档）完全不被识别；对比度的「闸门」实为硬编码清单（新 token 可零触发），
  且解析正则 `[a-z-]+` **不含数字**（`--color-chart-1` 被静默跳过）；`components/ui/*`
  整目录豁免过宽。全部补齐，`themeTokens` 正则修正后 `chart-*` 才真正纳入受检。
- 登记未做（`docs/roadmap.md` §4.15）：CVD（色觉障碍）模拟、闸门解析器的块切分脆弱性。
- 凭据：`cargo test` **244 绿** · fmt+clippy 绿 · 前端 typecheck 0 err ·
  oxlint 0 warning · test **214 绿** · 诊断页与错误态经浏览器复核。

### 2026-09-10 refactor(uiux)：设计 token 收口——语义四族、真三级灰阶、elevation 与圆角梯度（评审批次 E）—— guan（AI 起草）

- **起因（维护者）**：「主题 token 设计的不好看」。实测诊断后确认根因**不是配色不好，
  而是 token 体系名存实亡**：`index.css` 定义了语义 token，但全仓 **200 处**
  直接使用 Tailwind 原生调色板绕过它——
  `ok`(28 处) 与 `emerald`(46 处) 并存 = 同一件「成功/运行中」两个绿；
  `warn`(41) 与 `amber`(59) 并存 = 同一个「警告」两个橙；
  `rose`(45) 全部无 token 对应，且被当成「NPM 官方包」的分类色（红色在暗示错误）；
  `purple`(9)+`violet`(6)+`indigo`(3) 三者混编同一类「分类」语义；
  `slate`(20) 六个档位散用于深色终端面。全仓共 36 个原生 shade。
- 变更（拆两个提交）：
  1. **token 层**（`index.css`）：① 中性层重建——`dim` #626a7a→**#3c4250**、
     `faint` #6b7280→**#626978**，与 `ink` 构成相邻 ΔOKLab 0.148/0.141 的**均匀三级阶梯**
     （旧 dim↔faint 仅 0.028 = 号称两级实为一级）；`bg` #f7f8fb→**#f1f4f9**
     （vs panel 1.062→1.102，卡片浮得起来）。**更正批次 C 的结论**：「浅底上三级灰阶不存在」
     只对「提亮三级」成立，正解是把二级压深。② 状态族新建 `ok`/`info`/`warn`/`danger`
     + 分类档 `alt`（各带 `-soft`），四族在 OKLCH **等明度 L≈0.49**、两两 ΔOKLab ≥0.12、
     且在 bg/panel/line-soft/自身 soft **四种底色**上全部 ≥4.5:1。③ 深色终端面新建
     `term` / `term-panel` / `term-line` / `term-ink` / `term-dim` / `term-faint`
     + 日志级别 `term-ok/-info/-warn/-danger`。④ 圆角 4 档→**按元素角色**两档
     （控制件 10px / 面 14px）。⑤ elevation 覆写 Tailwind 原生档位名（`--shadow-2xs…2xl`
     共 8 档），投影带冷色偏（rgb 23,37,84）双层——**131 处调用点零改动即获得新语义**。
     ⑥ shadcn 语义层 `:root` 改为 `var(--color-*)` 引用，消除第三份 hex 真相源。
     ⑦ 退役死 token `badge-a`（零引用）/`badge-b`（与 term 重复）。
  2. **调用点迁移**（20 文件）：原生档位→语义 token。**amber 逐处判语义**（唯一需人判的族）：
     `isDefault`/星标选中→`brand`（选中态）、`isSwitching`/启动中/交接中→`info`（进行中是
     信息不是警告）、`needs_repair`/熔断/覆盖安装→`warn`（真警告）、评分星标→`brand-deep`。
- **顺带修两处语义错位（真 bug）**：
  ① 「NPM 官方包」徽标在 `MarketPluginCard` 用危险红、在 `MarketInstallDialog` 用成功绿——
     同一概念两个色；改为中性（由图标表意），安装对话框内 npm/github 两徽标也统一中性。
  ② 「运行中」在 profile 用 `ok`、在会话维护用 `sky`——同一状态两个色；统一为 `info`。
- **顺带修一处机械映射引入的回归**：`ErrorCard` 的深色诊断日志面板里，绿色文字被按
  「浅底文字」映射成 `text-ok`（深绿）落在近黑底上不可读 → 改 `text-term-ok`。
- 机器闸门（**新增 `paletteTokens.test.ts`**，4 条）：禁止原生调色板档位回流（含对照表）、
  shadcn `:root` 不得手抄 hex、幕布脚本色值必须与 `index.css` 逐值同步、闸门自检。
  `contrast.test.ts` 由 6 条扩为 **13 条**：三底色（原漏 line-soft）、深色面专项、
  族色压自家 soft 底、`*-soft` RGB 必须与基色同源（防手抄失联）、族间 ΔOKLab、
  品牌色与族色不互冒、灰阶阶梯、底色三档可辨。
- 同步：`ui.rs` 原生窗口 `background_color` 由 `(249,250,251)` 改 `(241,244,249)`——
  **原值与本仓库 CSS 从来就不一致**（旧 `--color-bg` 是 #f7f8fb），「冷启动无闪色」的注释
  与事实不符，本批一并校正。
- 反向取舍记录：曾考虑新增 `star`（金色）token 表示评分，实测金色要可见就必须够深，
  够深则与 `warn` 分离度仅 0.101（低于可辨阈值）→ **不加该 token**，星标改用 `brand-deep`。
  `format.ts::getProfileColorClass` 的 7 色彩虹同样无解（等明度约束下最小 ΔOKLab <0.10），
  改为单一 `alt` 档——颜色本不承载信息（芯片里就有 profile 名字），颜色只负责「这是个标签」。
- 影响：**需人工目检**（Tauri 真窗口未实机验证）——① 卡片投影观感（elevation 全量换值）；
  ② 圆角收敛后大卡片/对话框的观感；③ 深色日志面板在新 `term` 底色下的整体感。
  维护者已确认「顺便微调状态色相」（四族等明度）与「全量收口」两项范围。
- **补强（同日，d53d382）**：对批次 E 做**绕过验证**（写探针文件实测闸门能否被规避）
  时发现三个盲区，其中一个是真 bug——
  ① **冷启动底色有第三处真相源**：`frontend/index.html` 的首帧 `<style>` 也硬编码了
  `#f7f8fb`，批次 E 改了 index.css 与 ui.rs 却漏了它，而它恰是「冷启动闪色」的
  直接来源（CSS 模块加载前的底色）。现三处逐值对齐，并由前端（index.html ≡
  `--color-bg`）与 Rust（从同一份 index.css 解析 `--color-bg` 比对
  `WINDOW_BACKGROUND` 常量）**双向锁定**，另含解析器自检。均经「故意改坏→报红→
  还原」实证。
  ② 内联 `style={{ color: "#047857" }}` 是类名闸门的盲区（探针实测），补闸门
  （现状零使用，属预防性收口）。
  ③ 闸门自检补 `hover:` / `md:` / `data-[state=open]:` 前缀与任意值形态的断言
  （原正则已能拦下，但缺断言则日后改正则无人知晓）。
  另把 `ui.rs` 的窗口底色提为具名常量 `WINDOW_BACKGROUND`（原内联字面量无锚点可搜）。
  凭据更新：`cargo test` **244 绿**（241 → +3）· 前端 `pnpm test` **209 绿**（207 → +2）。
- 凭据：`pnpm typecheck` 0 err · `oxlint` 0 warning/0 error · `pnpm test` **201 passed**
  （190 → +11，含新增 paletteTokens 4 条与 contrast 扩至 13 条）· `cargo fmt --check` 通过 ·
  `clippy --all-targets -D warnings` 通过 · 浏览器逐页复核（Profile 列表 / 会话维护 /
  系统控制台 / 运行日志）无回归 · `pnpm build` 后确认新工具类均已生成且 6 个旧色值
  从产物消失。

### 2026-09-10 宪法级（AGENTS §9 索引）· ADR-0014 重启/切换交接带 + 启动代际闸门 + Windows 进程树收口 —— guan（AI 协作）

- 变更（ADR-0014，`docs/adr/0014-restart-handoff-continuity.md`）：
  1. **交接意图（handoff）**：`boot.rs::Handoff`（target/kind/phase/startedAt/generation）为内存态贯穿状态，`switch_profile` 返回、`get_boot_status.intent` 补水；两个窗口用同一纯模型 `lib/handoff.ts::deriveHandoff` 折算同一条四段导轨（停旧/起新/等就绪/进工作台）+ 同一计时器（startedAt 跨文档不归零）。
  2. **主窗口三段承接**：新增 `frontend/src/injected/handoff-curtain.js`（document-start 幕布，自发现 + Rust `eval` sticky 注入），覆盖「旧工作台 → 壳启动屏 → 新工作台」两次整文档替换的空档；React 一挂载即接管。
  3. **控制中心不再说谎**：删掉 1.2s 轮询 + 15s 兜底的临时过渡态，改由导轨讲述真相；「运行中」徽标在交接结束前不点亮（旧实现早于真实可用状态）。
  4. **子进程逃逸修复（审计发现）**：① 会话槽 `Option<Box<dyn Executor>>` 直接赋值会把旧 dsh 丢在地上——落新会话前 `reap_previous_session` 先收口；② `probe_epoch` 晚读导致两次快速切换双双放行 → 改为**启动代际令牌**（`begin_boot` / `boot_superseded`），spawn 前/后逐个分叉点校验；③ `RunEvent::Exit` 先置 `shutting_down` 再收会话，退出竞态不再留孤儿；④ Windows `stop_dsh` 由 `child.kill()`（只杀 `cmd.exe` 壳层，pnpm shim 的 node 继续跑）改为 `taskkill /PID /T /F` 整树收口。
  5. 契约闸门：`ipc-shapes.json` 增 `HandoffSnapshot`（Rust `handoff_json_shape` ↔ TS `ipcShapes.test`）。
- 影响：AGENTS §9 索引新增 0014 行（宪法级改动，已落档）；`switch_profile` 返回值由 `()` 变交接意图快照（前端同步）；`get_boot_status` payload 增 `intent` 字段（向后兼容）。行为可见变化：重启/切换全程有连续 loading 贯穿，失败在控制中心可见。
- 修订（同日，维护者当场否决首版冗余）：首版把「四段导轨」也画进主窗口启动屏并强制展开
  5 步向导态 → **重启长得像另一个页面**（与开机不一致）。改为**复用**启动屏本身：主窗口只换
  标题（「正在重启「X」」）/副题（当前阶段）+ 加连续计时，构图与开机逐像素一致；导轨只留
  在控制中心（那里本来没有启动 UI）。连带把 `HandoffView.inflight` 接回导轨（过期未落定
  的意图转静态灰点，不再假装在转），`isSetupMode` 恢复既有规则。见 ADR-0014 §7。
- 凭据：`cargo test` 241 绿（新增 5：交接阶段/意图起始/代际闸门/快照形状/Windows 收树参数）；`cargo fmt --check` + `clippy -D warnings` 绿；前端 `typecheck`/`lint`/`test` 绿（190，新增 handoff 模型 13 例）；观感经浏览器预览截图核对（开机屏 vs 重启屏构图一致、控制中心导轨、幕布明暗两态）；实机验证清单（macOS/Windows/WSL）待排期，见 ADR-0014 §5。

### 2026-09-09 快车道直推 · 迁移器拒绝会话归类修正——不再误标「需自愈」（实测 8650d6f2） —— guan（AI 协作）

- 变更：
  1. **根因**：`session-8650d6f2` 是 0.1.2 时代会话（turn 起始前写 5 条 user/message 输入组），0.1.5 发布链 v2→v3 迁移边以 `SessionFormatUnsupportedMigrationError` 拒绝（format v2 surface before first step cannot acquire a system head without changing chronology）——**引擎本尊也打不开**；脚本 stream 路径漏判该错误类 → 归 needs_repair，用户点修复必然失败。
  2. `restoreCurrentArtifactStream` 错误分类：`SessionFormatUnsupportedMigrationError`（类名匹配，与 legacy 路径同法）→ `unsupportedVersionError`，scan 归 unknown + 「存储格式版本不受支持（不可修复，需升级适配）」+ 原始原因；修复入口如实拒绝（文件不动、无备份）。
  3. `sessions.rs`：stub catalog 增加迁移器拒绝模拟（cwd=/tmp/refuse 首 surface 抛名匹配错误）+ 2 项用例（scan 归 unknown 非 needs_repair / repair 拒绝且不动文件）。
- 影响：8650 类会话（旧代形态、官方迁移链拒绝）不再显示「一键修复」误导；数据完好（6172 事件、正常收尾、有 .bak），等 dsh 官方适配即可打开；如需内容可先取明文导出。
- 凭据：`cargo test` sessions 23 绿（新增 2 例）；真实 8650 只读复扫 = unknown + 升级提示（validator dsh-session@0.1.5-alpha.1+catalog）。

### 2026-09-09 快车道直推 · 会话修复后失效 dsh 投影缓存——stale blank 投影致侧栏隐藏（实测 4885） —— guan（AI 协作）

- 变更：
  1. **根因**：dsh 侧栏可见性（blank/title）由 `session_projcache` 投影片段决定，`recordFor` 只以日志身份（formatVersion/createdAt/cwd/isSeeded/inheritedEventCount）判定缓存有效；4885 的缓存是 16:48 对空 v3 写的 `blank:true` 投影，文件修复**不改变身份** → 缓存永不失效，写回又只发生在创建/turn/end/释放三处 → 修复 + 重启后侧栏仍隐藏（`sessionVisible` 过滤非当前 blank 行，client-ui-workspace 实锤）。
  2. `repair-session.mjs` 新增 `clearSessionProjectionCache`：任何写回成功（世代重建 + 常规修复）都删除该会话的投影缓存条目（derived data，缺失 = dsh 冷读重建，语义安全），消息附注清理结果。
  3. `sessions.rs` 重建用例预置 stale blank 缓存并断言修复后被清除。
- 影响：一键修复后重开 dsh 即可见（此前的 4885 缓存条目已由本次人工清除，下次修复自动覆盖）；投影缓存删除属 derived data 操作，dsh 冷读自动重建。
- 凭据：`cargo test` sessions 21 绿（新增断言），脚本真会话副本演练：修复 → 缓存条目移除；`fmt --check` 干净。

### 2026-09-09 快车道直推 · 会话自愈新增「世代分叉」检测与一键修复（类别 5）+ 校验器适配 dsh 0.1.5-alpha.1 —— guan（AI 协作）

- 变更：
  1. **根因**（实测 session-4885a34d）：dsh 0.1.5-alpha.1（SESSION_FORMAT_VERSION=3）对「仅种子段」的旧会话迁移发布出**空 v3 世代**；随后旧版 dsh 0.1.2-rc.1 不识世代文件名，把真实对话持续追加进 v0——新引擎只认最高世代（空），会话打开即空白、会话标题回退项目名。同目录多世代并存即「世代分叉」（类别 5）。
  2. `scripts/repair-session.mjs`：校验器能力分派适配 0.1.5+（stream catalog API `createRestore/encodeCurrentHeader/encodeCurrentEvent`；0.1.3 代 legacy API 兼容；无 catalog 才要求 0.1.2 代存储层导出）；`--scan` 对最高世代条目叠加引擎本尊还原比对（行比较忽略 time 元数据——30cbe3e5 实测仅末条 end-seed 时间不同不得误报），分类：两代一致=正常；当前世代为源早前前缀 / 当前世代不可读而源完好=可无损重建（needs_repair）；真分叉（互不包含）=unknown 保留现场；一键修复=按源经 catalog 编码重建当前世代（备份旧世代、写后本尊校验、源保持原样）。
  3. `src-tauri/src/sessions.rs`：新增 stub 引擎包（global/v11 布局的 dsh-session + format-catalog 恒等 stub）驱动 4 项用例：分叉标记 / 双代一致不误报 / 重建闭环 / 真分叉拒绝；既有损坏类别 fixture 全量回绿。
  4. 文案（zh-CN / en-US）：`statusNeedsRepairDesc` 覆盖世代分叉。
- 影响：会话维护新增可修复类别，需引擎档 dsh ≥0.1.3 + format-catalog 才启用检测/修复（fallback 不误判、维持原判定）；真分叉不自动合并（保留现场，建议向 dsh 官方报障）；受影响的 4885 类会话在下次会话维护刷新时自动标「需自愈」，一键修复即恢复。存量数据无需迁移。
- 凭据：`cargo test` 233 passed（sessions 21 项，含 4 项新用例），`cargo fmt --check` 与 `cargo clippy -- -D warnings` 干净；前端 typecheck + 170 测试绿；真实会话 /tmp 副本演练：扫描标记「世代分叉」→ 一键修复 → 引擎本尊还原 2913 逻辑事件与 v0 全等（29 用户 / 528 助手消息），对照会话 30cbe3e5 无分叉误报。

### 2026-09-09 快车道直推 · 启动体验双模重塑（日常秒启 Splash + 全新工作台启动台 Launchpad 与偏好持久化） —— guan（AI 协作）

- 变更：
  1. **日常秒启与环境准备向导解耦（BootIndex 双模）**：日常启动环境就绪且直达默认工作台时，屏蔽繁重的 5 步向导时间线，切换为极简典雅的 Direct Launch Splash（居中 Emblem 呼吸微光 + 状态提示 + PulseBar，提供可折叠的启动详情）；仅在环境下载（first-run）或异常报错时完整展开 BootTimeline。
  2. **全新工作台启动台（Workbench Launchpad / BootSelector）**：全面升级为产品级 Launchpad，富元数据卡片矩阵（标题、描述、插件统计、模板标识、默认标签）、键盘 `1~9` 快速盲打直达、一键“记住我的选择（下次启动直接进入此工作台）”、直达工作台管理中心与新建工作台；启动中实时 Connecting 原地反馈。
  3. **SPA 平滑导航桥接（消除白屏刷新闪烁）**：在 `App.tsx` 挂载 `window.__DSH_NAVIGATE__` 桥接，Rust 端 `launch_executor_after_probe` 派发时优先通过 SPA 客户端路由平滑转场，避免 `location.assign` 带来的全页白屏重载。
- 影响：仅周知；用户首次体验与日常高频启动彻底分离，默认工作台偏好闭环。
- 凭据：`cargo test` 229 passed，`pnpm test` 170 passed，`oxlint` 0 warning，`cargo clippy` 0 warning，`cargo fmt` check 通过。


### 2026-09-09 快车道直推 · 插件安装飞行动画 + 下载管理 Popover 点外收起 + 引擎就绪秒级直达（5a5d279） —— guan（AI 协作）

- 变更：
  1. **插件安装飞行动画（问题记录-2026-09-09 §1.1）**：新增 `stores/installFlightStore.ts`、`lib/installFlight.ts`（抛物线几何计算）、`components/market/InstallFlight.tsx`（悬浮飞行层），以及 `QueuePanel.tsx` 锚点登记与落点角标 spring 弹跳及脉冲光环；市场安装与总览分发弹窗均已接入 launch 坐标；配套单测 `installFlight.test.ts`。
  2. **下载管理 Popover 收起（问题记录-2026-09-09 §1.2）**：新增 `components/ui/popover.tsx`（Radix UI Popover 原语封装），替换原手写 absolute 展开，原生支持点击外部 dismiss 与 ESC 退出；配套闸门测试 `popoverDismissGate.test.ts`。
  3. **启动引擎就绪短路（问题记录-2026-09-09 §2）**：`engines.rs` 引入 `probe_engine_if_ready` 与 `is_engine_ready`，`resolve.rs` 引入 `resolve_launch_engine_ready`，在 `LocalExecutor::probe` 中若引擎三件套已就绪则跳过步 0 和步 1 耗时等待，直达步 2「启动工作台」；配套单测全绿。
- 影响：仅周知。
- 凭据：`cargo test` 229 passed，`pnpm test` 170 passed，`oxlint` 0 warning，`cargo fmt` check 通过。

### 2026-09-09 快车道直推 · 发版日志强契约门禁与提取脚本严格模式（1c82d36） —— guan（AI 协作）

- 变更：CI 工作流中增加 `docs/RELEASE_NOTES.md` 版本匹配校验门禁；`scripts/extract-release-notes.py` 增加 `--strict` 模式与单元测试；`AGENTS.md` §8.8 补充发版日志强契约规范。
- 影响：发版时必须先编写规范日志再打 tag。
- 凭据：`python3 -m unittest scripts/tests/test_extract_release_notes.py` 6 passed。

### 2026-09-07 建档 · Spike 0004：DeepSeek Harness SDK 能力源码调研 —— guan（AI 会话协作）

- 变更：新建 `docs/spikes/0004-dsh-sdk-capabilities-research.md`（+297 行，
  只读调研不触运行时）。核心结论：dsh 外部 SDK 是「本机进程集成边界」
  （stdio JSON-RPC 2.0 驱动完整 `dsh --profile` 子进程），不是托管 HTTP
  API；agent/工具/凭据/持久化/安全策略由 profile 决定，SDK 本身无会话
  存储层访问能力——会话修复等管理功能不能经 SDK 实现，维持文件层校验器
  路线（与 2026-09-07 会话维护两笔裁定互证）。
- 影响：仅周知；后续「经 SDK 集成」类诉求先读此档再立项。
- 凭据：纯文档；事实主张均锚 dsh 源码位置（d347e70，0.1.3-alpha.1）。

### 2026-09-07 快车道直推 · boot 启动页单主角重构 + 遥测文案去术语化 + get_boot_status 竞态补水 —— guan（AI 协作）

- 变更：
  1. **启动页单主角重构**：hero 区块撤编（与时间线重复讲述同一状态）——
     BootTimeline 卡成为唯一主角（卡头 = 徽标 + 当前状态标题/副题 + 分段
     进度条，danger 态转警示）；步骤行重做（done/pending 收单行 + running
     行保留完整遥测详情、等宽可选中 + 一键复制；竖向导轨容器层统一绘制，
     行高变化不再撕裂连接线）；下载进度经 banner 槽位入卡。
  2. **遥测文案去术语化**：boot.steps 五步 hint、executor/lib 的 sink/
     emit_step 遥测、崩溃守护与升级提示全部改为面向用户的可行动文案
     （PATH/spawn/tier/WebView/code= 等内部词汇不再外露；tier 经
     `tier_label` 映射「内置引擎/内置离线副本」）。
  3. **get_boot_status IPC 三处同步**（COMMANDS 登记 + handler +
     capability，AGENTS §7 登记）：BootIndex 挂载时播种缓存中的启动状态
     与错误（normalizeStep/normalizeError 入 bootStore）——修 WebView
     挂载前事件丢失的竞态；ErrorCard 动作触发前 clearError 防旧错残留。
- 影响：仅周知。`t.boot` 删除 consoleTitle/stError/stReady 三键（消费方
  已随重构移除）；新增 copyDetail/copied/progressAria。
- 凭据：cargo test 159 全绿 + fmt + clippy；前端 typecheck + oxlint +
  104 测试（全树在途状态下验证）；IPC 三处同步经 ipc.rs gate_tests 闸门。

### 2026-09-07 快车道直推 · 引擎引导加固：固化真实 node 二进制 + 引导版本口径放宽（rc 可用、alpha 仍拒） —— guan（AI 协作）

- 变更（ADR-0010 引导链修补，附在途复现工具一并入库）：
  1. **固化真实 node 到 `PNPM_HOME/bin/node`**：pnpm v12 `shim add node` 生成
     带上下文检查的 shim dispatcher，`pnpm add -g` 执行 postinstall（protobufjs/
     koffi 等）因子目录无 devEngines 声明报 `ERR_PNPM_SHIM_NO_TARGET` 退出 1。
     新增 `find_runtime_node_bin` / `link_real_node_binary`（symlink→硬链→复制
     三级兜底），`shim_add_node` / `install_dsh_global` / `bootstrap` 各环节接线；
     WSL 客体侧 guest 脚本同构修补（`ln -sf` 真实二进制）。
  2. **引导版本口径放宽**：`latest_stable_dsh_version` 由「仅稳定版」改为
     「稳定版或 rc 候选（`is_acceptable_dsh_version`），明确拒绝 alpha 等未
     稳定前置版」+ 测试——修 rc-only 发布期无引擎可引导的死路；alpha 事故
     预防口径不变。
  3. **打包资源补 `resources/pnpm/**/*`**：捆绑 pnpm 压缩包随壳安装包落地
     （ADR-0010 边界 A），此前缺失会致打包产物首启引导失败。
  4. 新增 `scripts/repro-boot-scenarios.sh`：引擎目录多场景裁剪复现工具
     （fresh/no-pnpm 等，只动壳自有资产，红线见脚本头注）。
- 影响：仅周知。引导行为变化点 = rc 版本可被自动引导；`bin/node` 由 shim
  变为真实二进制（生命周期脚本执行路径变化）。
- 凭据：cargo test 159 全绿（含新增 acceptable_dsh_version 用例）+ fmt +
  clippy（全树在途状态下验证）；真实引擎目录实证引导产物齐全。

### 2026-09-07 会话维护 · 归档感知 + 运行中复合判据 + 会话元数据透出 —— guan（AI 协作）

- 变更（grilling 共识后 Batch 1，意图 = 会话维护列表准确性）：
  1. **运行中误判修复（bug）**：active 判据由裸 `mtime<5min` 升级为复合式
     ——dsh 引擎进程存活（壳持有的会话执行器 `try_wait`，WSL 客体以
     wsl.exe 存活代理）**且** mtime<5min。Rust 经 `DSH_ENGINE_ALIVE=1/0`
     注入脚本（scan/repair 同源）。dsh 未运行时不再误判——修复了用户实测
     「归档/重命名会话被误标进行中 5 分钟、修不了」。残余竞态已评估并锚定：
     dsh 追加为逐批 `open("a")→write→fsync→close`（persistence-jsonl@0.1.2-rc.1
     appendLines），不持长驻 fd，失败批次回滚+游标重试，修复与写入竞争
     最坏情形 = dsh 稍后重放批次，无永久丢数据。
  2. **归档感知（bug/扩展）**：dsh 归档 = 仅原子重写 `~/.dsh/storages/
     workspace.json` 的 `global.archivedSessionIds`（2026-09-07 实证结构，
     会话日志零改动、仍可加载可修复；dsh 侧栏客户端过滤、无取消归档 API）。
     壳同口径：`SessionItem.archived` + 容错解析（缺失/损坏按无归档不阻断），
     默认隐藏 + 新增「已归档」筛选档；归档会话照常体检可修；统计卡/搜索
     跟随可见性（2026-09-07 口径 6 条）。
  3. **元数据透出（扩展项1）**：`--scan` 新增 createdAt（毫秒，实证真实日志）、
     eventCount（填实原恒 0 死字段）、endState（最后一条 turn/end 的
     reason.kind，真实语料全集 completed/aborted/error/open；无 turn/end 记
     open（未收尾））、subagent（dsh 侧栏亦隐藏子代理，壳标注展示不隐藏）、
     agentPreset；validator（校验器版本）透出到状态徽标 tooltip（0.1.3
     过渡期有用）。前端会话行新增元数据副行与三类徽标。
- 影响：仅周知。`list_sessions` 载荷扩展（无新 IPC 命令、不触 §7）；
  dsh 升级日「已归档集合经 workspace.json 读取」不受世代迁移影响。
- 凭据：cargo test 159 全绿 + fmt + clippy；前端 typecheck/oxlint/104 测试；
  真实语料 9 会话双态扫描（alive=1/0 复合判据行为正确）；fake-engines
  catalog 分支回归（原始损坏快照 needs_repair 检出）；同批工作区含另一在途
  boot 文案流改动，本次提交经 hunk 级摘取只含本意图 7 文件。

### 2026-09-07 会话维护补强 · dsh 0.1.3 格式世代前瞻适配（catalog 迁移管线 + 版本路由） —— guan（AI 协作）

- 变更（`scripts/repair-session.mjs` + `src-tauri/src/sessions.rs`；本条与该改动同
  commit 落盘）：
  1. **背景**：dsh 0.1.3（当前 alpha.1）将 `SESSION_FORMAT_VERSION` 0→2，存储改
     「不可变世代」模型——v0 源与 `session.vN.jsonl[.zstd]` 迁移世代并存，读取经
     `session-format-catalog` 迁移管线（decode → migrate → encode → 发布 v2 世代）。
     逐字核查其 jsonl 后端 / catalog / chain / filename 源码后提前适配。
  2. **校验器版本路由**：恢复层发现引擎档 `dsh-session-format-catalog`（与
     dsh-session 同代安装，`currentVersion` 须一致）时复刻真实读路径：readHeader
     分类 → decodeRecoverableArtifact → migrate → encodeCurrent → **迁移后表示**
     走同一条扫描+prepareCore 链；无 catalog（0.1.2 代）时维持「存储版本==已装
     版本」直通链。修复谓词跑在迁移后表示上，修复动作仍落在源世代表示。
  3. **语义修正**：存储版本比已装 dsh 新 / 迁移器拒绝 / fallback 遇 vN → 一律
     `unknown` + 升级提示（unsupportedVersion 标记），**不再误归 needs_repair**
     （与 dsh `refuseForeignFormatVersion` 同语义；修复入口对该类直接如实拒绝）。
  4. **世代文件族发现**：脚本 `findSessionFiles` 与 Rust `scan_sessions` 均识别
     `session.vN.jsonl[.zstd]`；同目录多世代只保留 dsh 实际读取的最高世代
    （列表单条目，修复入口不打在 dsh 不读的文件上）。
- 影响：dsh 升级 0.1.3 后存量 v0 会话不再被误判（迁移管线校验，闸门语义不变
  「通过 = dsh 一定能加载」）；本脚本无需随 dsh 升级改代码。**遗留一项升级日
  实测**：修复 v0 源后已发布 v2 世代的失效/重发布机制（generation.ts 源身份
  守卫）需装上 0.1.3 后实证；当前口径 = 各世代文件独立校验/修复。仅周知。
- 凭据：cargo test 156 全绿（+3：世代文件名解析、同目录最高世代选择、v2→
  unknown 分类回归）+ fmt + clippy；catalog 分支经高保真 stub 管道端到端验证
  （API 契约逐字锚 0.1.3-alpha.1 源码）——真实语料 9 会话双引擎路径全绿、
  原始 8650d6f2 损坏快照经 catalog 分支检出并修复、手造 v2 文件三模式
  （真引擎/有 catalog/fallback）一致归 unknown。

### 2026-09-07 会话维护 · 健康检测对齐 dsh 本尊恢复校验——surface 悬空 replace 可检可修 —— guan（AI 协作）

- 变更（`scripts/repair-session.mjs` + `src-tauri/src/sessions.rs`；本条与该改动同 commit 落盘）：
  1. **根因**：`session-8650d6f2` 为历代修复脚本的「重编号嵌合体」——压缩摘要
    （`user/message` + `surfaceOp.replace`）的 start/end 仍指向重编号前的旧 seq，
    dsh 打开即 `invalid seed event at index 37: surface replace: end seq 1686 not
    found in surface`；旧健康检查只验存储层 seq 连续性，对此类完全失明（第 4 类损坏）。
  2. **健康判定升级为两层**：存储层（原有 seq/出处链形检查）+ 恢复层（新增）。
    恢复层优先动态 import 引擎档 `@deepseek-ai/dsh-session` 本尊（与实际加载该
    会话的 dsh 同版本），按 `prepareCore` 全链复刻：词汇表闸门 → `adoptSessionEvent`
    → `interruptedTurnClosers` 补尾 → `Session.fromRestore`（含 surface fold 全量
    重放）——本层通过 = dsh 一定能加载。引擎缺包降级内置 fold 移植（锚已安装
    v0.1.2-rc.1 的 `surface.ts`/`index.ts`；注意 0.1.2 允许 assistant/message 携带
    出处链，与仓库 HEAD 0.1.3 规则不同，勿以后者为锚）。
  3. **修复策略（第 4 类）**：二分定位首个坏事件 → 最小变异（悬空 replace 转
    append、剥离失效 `sourceEventSeqs`；绝不重编号/不删事件/不改内容）→ 写盘前
    对产物字节再过「存储层 + 恢复层」双闸门，任一失败放弃写入。Rust 侧向脚本
    传 `DSH_DOCK_ENGINES`（引擎档根，ADR-0010 资产定位）。
- 影响：真实会话 8650d6f2 已修复（6171 事件零丢失，dsh 本尊 `fromRestore` 由
  失败转通过，重新打开即可用；损坏现场快照另存 /tmp，既有 `.bak` 未覆盖）；
  「一键检测/修复」此后对 surface 类损坏有检出能力。dsh 升级无需改脚本
  （校验器运行时解析引擎档最高版本包）。仅周知，无需动作。
- 凭据：cargo test 153 全绿（含新增回归 `repair_session_heals_dangling_surface_replace`，
  CI 走 fallback 路径确定性覆盖）+ fmt + clippy；真实语料 9 会话 `--scan` 零误报
  （dsh/fallback 双模式结论一致）；8650d6f2 修复前后 `Session.fromRestore` 失败→通过。

### 2026-09-05 会话维护 UI/UX 重构 · 健康检查 + 会话名称 + 日志时区修复 —— guan（AI 协作）

- 变更（commit 9935cdd）：
  1. **健康检查**：`repair-session.mjs` 新增 `--scan` 只读模式（JSON 输出 healthy/needs_repair/unknown + 标题），Rust `scan_sessions` 经引擎 node 调用填充 status/title/healthDetail（node 缺失降级 Unknown）；`SessionItem` 新增 `title`/`healthDetail` 字段。
  2. **会话名称**：标题取自 dsh `session/title` 事件，列表以会话名称为主视觉，ID 为等宽辅助（可复制）。
  3. **SessionManager UI/UX 重构（会话探针室风格）**：健康状态色点徽标；修复按钮仅非健康会话显示；全局「一键全量体检与自愈」仅异常时可用（脚本对健康 no-op）；状态筛选（全部/仅看异常带计数）；非健康行琥珀色脉冲边条 + 异常原因。
  4. **日志时区修复**：`localizeLogTimestamp`（lib/format.ts）ISO8601 UTC → 本地时区（兼容 ANSI 转义/跨日），LogViewerPane 接入；+4 单测。
- 验证：cargo test 151 全绿 + fmt + clippy；前端 typecheck + oxlint + 104 单测；真实 `--scan` 9 会话（8 健康 1 需修复）。
- 注：同批 `lib.rs` 含先前在途 boot 改动一并提交（编译依赖）。

### 2026-09-04 会话自愈重写 · 与 dsh 加载器语义对齐的重放重叠去重修复 —— guan（AI 协作）

- 触发：会话 `session-1214c12f`（用户会话）损坏，一键全量体检与自愈、单会话一键修复均无效（假成功）。
- 根因：dsh 0.1.2-rc.1 中断恢复后以相同 seq 重放被中断轮次真实事件，磁盘残留旧占位（`turn/end` / `session/end-seed` 等），形成「连续前缀 + 重放块」重叠；加载器 `dsh-session-persistence-jsonl` 在重叠处报 `seq gap in committed region` 并丢弃重叠点之后全部恢复事件。旧版自愈按 turn 重排 + 全量重编号 → 破坏 append-only 模型与 `sourceEventSeqs` 出处链，且失败一律 exit 0（假成功）。
- 变更（commit 2676ec4）：
  1. `scripts/repair-session.mjs` 重写：重放重叠检测与去重（丢弃被遮蔽旧事件，保留重放块，顺序与 seq 原样）；序列缺失按加载器语义截断；不可安全修复明确报错；写前备份 + 临时文件加载器语义校验 + 原子替换 + 写后竞态复查；健康文件幂等 no-op；失败退出码非 0。
  2. `src-tauri/src/sessions.rs`：临时脚本路径含 PID + 时间戳（并发不踩踏）；以脚本退出码为准；单测重写复现真实重放重叠（复现先行）+ 健康幂等测试。
- 实测：修复 session-1214c12f（11965 事件）与 session-8650d6f2（6171 事件），dsh v0.1.2-rc.1 真实 loader 语义校验通过，全量 8 会话扫描全部 OK。
- 凭据：`cargo test` 151 单测全绿 + `cargo fmt --check` + `clippy -D warnings` 零告警。

### 2026-09-04 宪法级改动 · 引擎引导链路与工作台 Token 跨源认证闭环 —— guan（AI 协作）

- 变更：
  1. **工作台 Token 认证与 Cookie 跨源直通**：针对 dsh 0.1.2-rc.1 引入的 URL `?token=...` 与 303 重定向设置 `SameSite=Strict` Cookie，在 WebKit/WKWebView 从 `tauri://localhost` 跨源导航时丢弃 Cookie 导致 401（`dsh web authentication required`）的问题，在 `src-tauri/src/lib.rs` 落地 `authenticate_workbench_session`：壳侧先经本地 HTTP 兑换 token 并将 `SameSite=Lax` Cookie 直接注入 WebView 原生 CookieStore，直达无参工作台根路径。
  2. **pnpm shim 调度器脱落修复**：pnpm v12 `shim add node` 生成的调度器硬链接在无 `devEngines` 依赖子目录（如 protobufjs/koffi postinstall）报错 `ERR_PNPM_SHIM_NO_TARGET`；在 `src-tauri/src/engines.rs` 与 `src-tauri/src/executor.rs` 增加真实 Node 二进制链接，确保生命周期脚本与客体引导平滑执行。
  3. **DSH 目标版本过滤放宽**：在 `src-tauri/src/updates.rs` 实现 `is_acceptable_dsh_version`，严格放行稳定版与 `-rc` 候选版本（排除 `alpha`），解决官方 registry 仅含 rc 版本时报无可用稳定版的阻塞。
  4. **启动初期事件竞态兜底（触 AGENTS §7）**：新增 `get_boot_status` IPC 命令并在 `ShellState` 缓存 step/error；前端 `BootIndex.tsx` 挂载时水合播种，根治早期错误卡无法呈现的竞态。
  5. **资源打包配置补齐**：`src-tauri/tauri.conf.json` 补齐 `resources/pnpm/**/*`，`lib.rs` 的 dev 模式解析优先回退源码树。
- 影响：**触宪法级**——`AGENTS.md` §7 增加 `get_boot_status` IPC 登记；全平台本地与 WSL 工作台启动认证与安装全链路畅通。
- 凭据：Rust 侧 `cargo test` 150 单测全绿（+2：`acceptable_dsh_version_accepts_stable_and_rc_rejects_alpha` / `cookie_parsing_adjusts_samesite_and_domain`）+ `cargo fmt --check` + `clippy -D warnings` 零告警；前端 `tsc` + `oxlint` + 15 文件 100 题全绿。

### 2026-09-04 完成通知 · 任务 G 启动页重构（对齐引擎倒置叙事 + 仪表盘级控制台视觉） —— guan（AI 协作）

- 占用声明：前端启动页组件与文案改动经维护者会话内指示（沿 P3-b 先例）。
- 变更：
  1. **文案全面对齐 ADR-0010 引擎倒置契约**：`zh-CN.ts` 与 `en-US.ts` 双语同步。启动步骤锚定后端实际序号（步骤 0 环境检测 / 步骤 1 准备引擎 / 步骤 2 启动工作台 / 步骤 3 等待就绪 / 步骤 4 进入工作台），剔除旧系统探测与 TooOld 叙事，模式说明全面更新为「基于应用内置引擎，首启自动引导 Node 与 DSH；就绪后完全离线运行」。
  2. **启动控制台流水线视觉（BootTimeline & BootStep）**：引入竖向一体化流水线导轨，5 步骤状态高保真渲染（Check/Spinner/Alert/Mono 数字），后端遥测 detail 升格为等宽代码徽标，进度芯片与状态点实时动态感知。
  3. **引擎引导专属卡片（DownloadProgress）**：下载主角位升级为自包含引擎引导面板，呈现实时传输速率与动态 ETA，结合两段式渐变进度条、SHA-256 完整性校验与离线直通提示。
  4. **顶栏与环境光晕质感精修（BootIndex & BootSelector & VersionChip）**：微光环境渐变光晕配合浮动 Emblem，顶栏统一轻量级品牌标识与精细化版本芯片，保留拖拽区与非阻断更新条能力。
- 影响：普通区前端组件与文案，后端契约与 IPC 零改动。
- 凭据：前端全量闸门通过：`pnpm run typecheck`（TS 7.0.2 绿）+ `pnpm run lint`（Oxlint 20ms 绿）+ `pnpm run test`（15 文件 100 单测全绿）+ Rust 侧 `cargo test` 148 单测全绿。

### 2026-09-04 宪法级改动 · 统一日志封装 lib/logger 与 console.* 全量收口（触 AGENTS §4.3） —— guan（AI 协作）

- 占用声明：经维护者会话内指示「统一日志封装标准」视同声明。
- 变更：
  1. **前端统一日志封装**：落地 `frontend/src/lib/logger.ts`，规范格式为 `[模块名] 行为描述 { 上下文参数 }`；debug 仅在 dev 构建输出，warn/error 全环境保留；明确保密合规（绝不打印密钥/Token/密码/PII）与防循环轰炸纪律。
  2. **console.* 全量收口**：存量 `console.*` 调用（clientUpdateStore 状态迁移告警、QuickDshSwitcher 聚焦失败）全部收拢至 logger。
  3. **单测覆盖**：`logger.test.ts` 落地（级别路由、空上下文省略、debug 门控三例）。
  4. **宪法 AGENTS.md §4.3 同步**：登记前端统一日志规范与口径。
  5. **Rust 侧审计**：`src-tauri/src/` 零残留 `println!`/`eprintln!`/`dbg!`，已全面落地 tracing 分级与结构化字段。
- 影响：**触宪法级 AGENTS.md §4.3**（日志统一 lib/logger，禁止直接调用 console.*）。
- 凭据：前端 typecheck + lint + 100 测试通过，Rust cargo clippy + cargo test 148 通过。

### 2026-09-04 宪法级改动 · 升级呈现与忽略版本记忆（触 AGENTS §6 持久化例外册） —— guan（AI 协作）

- 占用声明：经维护者会话内指示完成 ADR-0010 台账「升级呈现」增量。
- 变更：
  1. **窗口内非阻断升级提示条**：新增 `UpdateBanner` 组件（挂载于启动页 header 下方浮层），当 dsh 或桌面客户端有新版本时提示版本信息与升级后果文案，引导进入更新中心。
  2. **忽略版本偏好持久化**：新增 `settings.dismissedUpdate`（形如 `dsh@1.6.0`），点击「忽略此版本」后原子写回 `settings.json`，同一版本不再弹窗，新版本发布时自动恢复提示。
  3. **判定逻辑纯函数测试**：`updateBanner.ts` 抽离纯函数，Vitest 5 个测试覆盖全部状态分支。
- 影响：**触宪法级 AGENTS.md §6**（持久化例外册新增 `dismissedUpdate` 字段）。
- 凭据：Rust 148 测试 + 前端 97 测试通过，双侧闸门全绿。

### 2026-09-04 完成通知 · WSL 客体 glibc pnpm 投递与客体引擎链（ADR-0010 台账） —— guan（AI 协作）

- 占用声明：经维护者会话内指示完成客体投递增量。
- 变更：
  1. **客体引擎链五态探测**：GUEST_PROBE 升级为 `GUEST_MUSL / PNPM_MISSING / NODE_MISSING / DSH_MISSING / READY`，musl 系（Alpine）明确报出可行动错误。
  2. **glibc pnpm 投递**：Windows 宿主内置打包 `pnpm-linux-x64.tgz`，通过 `\\wsl$` 拷贝为主通道（体积复核），base64 stdin 为兜底通道，客体内 tar 解包落位至 `~/.dsh-dock/engines/bin`。
  3. **客体网络与执行**：在客体进程内通过 pnpm 执行 `runtime set node` 与 `pnpm add -g dsh`，网络发生在客体环境内，镜像链保持注入。旧 curl-tarball node 链路退役。
  4. **构建流水线支持**：Windows runner 构建前同时拉取 linux-x64 离线捆绑包。
- 影响：Windows 平台 WSL2 模式引擎自动化闭环，客体与宿主同享自包含引擎策略。
- 凭据：模板全量 bash 实跑测试（五态链序/prep前置/stage落位/RFC向量/UNC路径），Rust 148 测试全绿。

### 2026-09-04 宪法级改动 · 探测层退役与引擎档唯一来源（触 AGENTS §2、§7） —— guan（AI 协作）

- 占用声明：经维护者会话内指示完成 ADR-0010 核心重构（探测层退役）。
- 变更：
  1. **系统探测全量删除（-2799 行）**：删除 `login_shell_path`、固定目录扫描、fnm/nvm 查找、pnpm global 扫描等历史探测代码；`effective_path` 收缩为纯环境变量 PATH；`resolve_launch` 仅认 Engine/Bundle 档。
  2. **工具链收敛**：`engines.rs` 与 `plugins.rs` 仅保留引擎档，双缺直接出可行动错误，不再混搭系统环境。
  3. **诊断与更新收拢**：系统诊断返回引擎四件套状态，更新升级链路完全限定在引擎目录内。
  4. **宪法条款修订**：AGENTS.md §2 明确 `resources/pnpm/` 永不入库；§7 boot 期 `npm i -g pnpm` 条目标注退役。
- 影响：**触宪法级 AGENTS.md §2 与 §7**。用户机器安装的 Node/dsh 版本彻底与壳运行时解耦。
- 凭据：`cargo test` 147 绿（已清理退役机制测试），fmt 与 clippy -D warnings 零告警。

### 2026-09-04 宪法级改动 · 前端工具链全面 Native 化（pnpm 12 + TypeScript 7 + Oxlint + Lucide 1.40） —— guan（AI 协作）

- 变更：
  1. **包管理器全面倒置对齐 pnpm 12**：前端包管理由 `npm` 切换至 `pnpm 12`（锁定 `pnpm@12.3.1`，与壳内置引擎引导器版本单一真相源严格对齐）；生成 `frontend/pnpm-lock.yaml`；配置 `shared-workspace-lockfile=false` 隔离父仓库 monorepo 漂移；移除旧 `package-lock.json`。
  2. **TypeScript 7 + Oxlint 极速静态检查**：升级 TypeScript 至 `~7.0.2`（Go 原生编译器引擎）；废弃传统 ESLint 全家桶（移除 `eslint.config.js`），切换至 Rust 内核的 `oxlint`（`oxlint src` 耗时由 ~1.5s 骤降至 21ms，0 告警 0 错误）；`lucide-react` 平滑升级至 `^1.40.0`。
  3. **Tauri 与 CI 流水线同步升级**：`src-tauri/tauri.conf.json` 的 `beforeDevCommand` 与 `beforeBuildCommand` 统一切换为 `pnpm`；`.github/workflows/build.yml` 引入 `pnpm/action-setup@v4`（三平台统一预装 pnpm 12.3.1，自动接入 pnpm store 依赖缓存，Linux/macOS/Windows 构建闸门全闭环）。
  4. **宪法 AGENTS.md §1 对齐**：前端开发与质量闸门命令同步修订为 `cd frontend && pnpm install --frozen-lockfile && pnpm run typecheck/lint/test`。
- 影响：**宪法级**——前端构建链、锁文件与 CI 闸门切换至 pnpm 12。协作者需使用 pnpm 12 执行前端依赖安装。
- 凭据：全量质量闸门实测验证通过：`pnpm install --frozen-lockfile`（92ms）+ `pnpm run typecheck`（TS 7.0.2 绿）+ `pnpm run lint`（Oxlint 22ms 绿）+ `pnpm run test`（15 文件 100 单测全绿）+ `pnpm run build`（356ms 成功打包）+ Rust 侧 `cargo test` 148 单测全绿。

### 2026-09-04 完成通知 · P3-b boot 接线 + contract v3（MANIFEST_FORMAT=3，引擎档缺省） —— guan（AI 协作）

- 占用声明：lib.rs（共享区）改动经维护者会话内指示「继续」视同声明（2026-09-04，本条目即落档）。
- 变更：
  1. **contract v3 落地**（docs/contract.md「运行时策略 v3」同日实现）：壳
     `MANIFEST_FORMAT=3`，`TierKind::Engine` 新档位；manifest 加载统一规范化为
     `tiers ∈ {[Engine], [Bundle]}` + fallback——v3 快照档（snapshot 三件套）与
     v1/v2 兼容迁移（fallback→快照档、极简在线档→引擎档；resolution 档序语义废止）。
  2. **boot 接线**：resolve_launch 引擎档臂 = ensure_engine_bootstrapped → 引擎
     LaunchSpec；executor 引擎档跳过 pnpm 补齐（捆绑 pnpm 随 boot 重铺恒在）；
     lib.rs 去 engines 模块 allow(dead_code)。
  3. **离线语义收口**：engines::bootstrap 版本解析改**惰性闭包**——node 已装但
     解析失败（离线且无缓存）→ 警告后用已装版本继续；dsh 已装永不查 dist-tags
     → 就绪引擎离线 boot 零网络（契约「之后 registry 不可达 → 已装引擎直接启动」）。
  4. **执行形态**：LaunchSpec.dsh_entry = DshEntry（NodeScript | Launcher）——
     引擎档 dsh 启动器直接执行（spawn_dsh / no-open 探测共用，探测缓存机制不变）。
  5. **打包内置 pnpm**（边界 A）：新增 scripts/fetch-pnpm-bundle.sh（版本从
     updates.rs PINNED_PNPM_VERSION 推导防漂移；npmmirror→npmjs 镜像链 +
     packument dist.shasum 完整性校验；实测 darwin-arm64 16.8MB 落位）+
     build.yml 三平台构建前取件步骤 + resources/pnpm/ 入 .gitignore（永不入库）+
     render-product.sh 升 v3（snapshot 三件套，打包侧同步完成）+ 本仓 manifest
     升 v3 引擎档缺省。
- 影响：**引擎档自此为产品缺省形态**——下一发版起 boot 走壳引擎引导（首启需
  联网），用户全局 dsh/node 与启动解耦（TooOld 死局消失）。本仓 dev/CI 即时生效
  （resources manifest 已 v3）；本机冒烟 = `tauri dev` 走引擎档全链路。
  探测层退役、创建链切引擎档、WSL 客体投递、升级呈现为后续刀。
- 凭据：`cargo test` 197 全绿（+4：manifest v3 迁移×2/未知 mode 拒绝/引擎档
  LaunchSpec 离线构造/bootstrap 离线降级）+ fmt + clippy -D warnings 零告警；
  fetch 脚本本机实测通过（npmmirror 命中 + sha1 比对）。

### 2026-09-04 完成通知 · P3 插件操作改引擎档（工具链解析引擎优先 + 转发链统一内核） —— guan（AI 协作）

- 变更：
  1. `plugins.rs` 工具链解析 `resolve_toolchain`：引擎档优先——engines/bin 内
     node shim 与 dsh 全局启动器**双全**才选引擎（半就绪不混搭，整体回退）；
     引擎未就绪回退系统探测。移除「未检出系统 Node/系统 dsh」硬失败——引擎
     就绪后系统安装不再是插件操作的前置条件（P3-b 接线前引擎恒空，行为等价旧系统档）。
  2. 引擎档执行：dsh 启动器（pnpm 全局 shim）直接执行——Unix shebang 脚本 /
     Windows .cmd（child_cmd 吸收），node/pnpm 经 PATH 解析，不再深挖 pnpm
     全局树取 lib/bin.js；`engines.rs` 增 `engine_node_bin` / `engine_dsh_bin` 定位入口。
  3. `profiles.rs` 转发链统一内核 `run_dsh_forward`（program + prepend + child_path）：
     `run_dsh_plugin` 保持原行为成为系统档薄封装（创建链零改动，随 P3-b 一并切）。
  4. 顺带闭合系统档隐患：插件操作的 spawn PATH 改 `dsh_child_path`（引擎 bin →
     node bin → 用户 PATH），与 ensure_pnpm 可见性基准严格同源——原实现补齐到
     引擎目录的 pnpm 在 spawn 时不可见，dsh 内部 spawnSync("pnpm") 会 ENOENT。
- 影响：普通区三文件（engines / profiles / plugins），lib.rs / IPC / 网络面零改动；
  引擎档暂未激活（boot 接线前 engines/bin 为空），现有用户行为不变。
- 凭据：`cargo test` 193 全绿（+3：引擎优先选择 / 系统回退 fixture / 双缺可行动
  错误）+ `cargo fmt --check` + `clippy -D warnings` 零告警。

### 2026-09-04 完成通知 · Apple 凭据全量轮换与分发链路验收（CI 四处修复 + rc.4 试发回退） —— guan（AI 协作）

- 变更：
  1. **凭据轮换**：Developer ID Application 证书重签（openssl CSR 路线——旧证书私钥卡在
     数据保护钥匙串无法导出 p12，新路线私钥文件化、p12 由 key+cer 直接合成）；
     App Store Connect API Key 换新（`925T697654`）；GitHub 六个 `APPLE_*` secrets 同步
     更新（gh 客户端加密直传，密钥不落会话）。
  2. **CI 四处修复**（build.yml）：证书 CN 推导截断逗号尾巴（原实现连带 OU/O/C，与
     Tauri 的 p12 裸 CN 严格比对必挂）；tag 版本校验放行 `-rc.N` 预发布后缀；release
     守卫 `rg`→`grep`（runner 镜像无 ripgrep）；草稿识别改走 releases 列表过滤
     （`/releases/tags/{tag}` 对 draft 恒 404）。
  3. **rc.4 试发与回退**：v0.9.4-rc.4 全绿发布并完成验收（公证 Accepted id 994e99ee，
     CI runner Gatekeeper 实测 `source=Notarized Developer ID`），验收后按裁定删除
     release 与 tag，`/releases/latest` 回落 v0.9.3，更新链路恢复原状。
- 影响：分发链路（签名+公证）恢复健康；Spike ③ 挂账的「公证链路权威验证」正式闭环。
  本机 Gatekeeper 处于关闭态（`spctl` accepted 不算数），验收以 CI runner 为准。
  遗留优化：dmg/app staple 票据缺失（v0.9.3 起既有行为，在线校验不受影响，仅离线
  首启需要）。v0.9.4 正式版待发。
- 凭据：run 33835515772 全绿（三平台 build + release）；notarytool history 两条
  Accepted 可查；下载产物 codesign 有效、公证记录在案。

### 2026-09-04 完成通知 · P3-a 引擎编排模块落地（engines.rs + updates 引导入口） —— guan（AI 协作）

- 变更：
  1. 新增 `src-tauri/src/engines.rs`（引擎编排，ADR-0010 主体第一件）：单目录布局
     （PNPM_HOME = engines/）· pnpm 子进程 env（PNPM_HOME + 引擎 bin 前置 PATH）·
     node 镜像 env 注入（键=release）· 非 TTY 进度行解析（映射 boot:progress）·
     就绪判定（三件齐验版本，v 前缀归一）· 幂等引导四步（捆绑 pnpm 重铺 →
     `runtime set node` 镜像链重试 → `shim add node` → `add -g dsh` registry 链
     重试）；失败语义 = 离线可启动（缺件补不齐才 Err，首启必须联网）。
  2. `updates.rs` 引擎引导唯一入口（AGENTS §7「引擎引导」）：`ensure_engine_bootstrapped`
     （node 版本取 node-map、dsh 取最新**稳定版**并排除预发布）+ 内置 pnpm tgz
     命名契约（resources/pnpm/<平台>.tgz，边界 A 压缩存储，系统 tar 解包零新增
     依赖）；boot 接线随 P3-b，新入口暂标注 allow(dead_code)。
- 影响：纯新增，不改变现有 boot 行为（P3-b 接线前 engine 路径不激活）。
- 凭据：`cargo test` 190 全绿（+8：进度行 spike 实测格式 / 镜像 env JSON 形状 /
  env 次序 / 就绪判定 v 归一 / tar 解包落位 / 假体探测 / 幂等零网络路径）+
  `cargo fmt --check` + `clippy -D warnings` 零告警。

### 2026-09-03 完成通知 · Spike 0003 实机闭环 + P2 引擎私有 pnpm 与子进程 PATH 自构 —— guan（AI 协作）

- 变更：
  1. **Spike 0003**（`docs/spikes/0003-pnpm12-engine-bootstrap.md`）：ADR-0010
     Spike①② 的 macOS 侧实机闭环——镜像注入通道实锤（`PNPM_CONFIG_NODE_DOWNLOAD_MIRRORS`，
     JSON 键 = 发布通道 `release` 等，缺键静默回退默认源；SHASUMS256 强制且与镜像同源）、
     `runtime set node` 非 TTY 全绿（字节进度行可解析、npm/npx/corepack 缺位、
     单目录引擎布局成立）、引擎链 e2e（镜像装 node → 引擎 pnpm add -g dsh →
     引擎 node 执行 dsh）。
  2. **P2**：pnpm 补齐落 `<数据目录>/engines/npm`（替代用户 npm 全局；bundle 档
     只读 resources 亦因此可补齐）+ 显式 pin `pnpm@12.3.1`（`latest` dist-tag 实测
     仍指 v11）；子进程 PATH 自构第一步 `dsh_child_path`（引擎 bin → node bin →
     用户 PATH），ensure_pnpm 可见性检查与 dsh spawn 同源。
- 影响：boot 行为变化——pnpm 不再写用户 npm 全局（用户已装 pnpm 仍被复用，引擎 bin
  恒优先）；dsh 子进程 PATH 前置引擎 bin。**两处待维护者裁定**（spike 0003 §4）：
  pnpm 二进制解包 32MB 触发 ADR-0010 §6 体积复审线；musl node 下载源硬编码
  unofficial-builds.nodejs.org 不可镜像注入（实测可达，暂接受）。
- 凭据：`cargo test` 182 全绿（+4：引擎 bin 平台布局 / 前置去重 / dsh_child_path
  次序 / pin 全 triplet）+ `cargo fmt --check` + `clippy -D warnings` 零告警；
  Spike 决定性证据（本地 404 服务器路由实锤）见 spike 0003 §2.4。

### 2026-09-03 完成通知 · P1 TooOld 死局过渡修复（system dsh 过低不再拒绝启动） —— guan（AI 协作）

- 变更：
  1. `resolve_launch`（ADR-0010 P1，独立先行）：system dsh 版本低于 `minVersion`
     时不再直接 bail（旧实现整个应用拒绝启动 = 死局）——记 warn 后跳过 system 档、
     按档序落 bundle/download 继续 boot，用户全局 dsh 仍不被触碰；仅当档序耗尽仍无
     宿主时，报错保留可行动文案（含实测版本与升级命令）。
- 影响：过渡期行为修复，不依赖引擎倒置落地；manifest v2 语义不变。LocalExecutor
  与 WSL 客体探测的 `resolve_launch` 调用面随此自动受益，无接口变化。
- 凭据：复现先行测试 `resolve_too_old_system_falls_to_next_tier`（过低 → 落 bundle
  档）/ `resolve_too_old_exhausted_reports_actionable_error`（档序耗尽报错不丢版本
  信息）；`cargo test` 178 全绿 + `cargo fmt --check` + `clippy -D warnings` 零告警。

### 2026-09-03 宪法级改动 · ADR-0010 引擎倒置：环境准备阶段重造（pnpm12 引导 / 探测层退役 / 升级全显式） —— guan（AI 协作）

- 变更：
  1. `docs/adr/0010-engine-inversion.md`（新建，已接受，含全节点裁定台账 §7）：壳内置
     pnpm12 为唯一引导器（node/dsh/pnpm 下载、布局、激活全委托 pnpm），探测层与自有
     下载器退役；dsh_home 保持 `~/.dsh`；node/dsh 升级统一「提示 → 用户决定 → 下次
     启动生效」；首启联网、之后离线可启动；WSL 客体投递 musl pnpm（用户零安装）。
  2. `AGENTS.md`：红线 2 修订（内置依赖唯一例外 = pnpm 引导器）· §6 例外册登记
     `engines/`、pnpm 硬依赖补齐方式改写 · §7 网络面登记「引擎引导」用途 ·
     §9 索引补 ADR-0010。
  3. `docs/contract.md`：追加「运行时策略 v3」章节（resolution/fallback 废止，
     `runtime.mode: engine` 缺省 / 声明 snapshot 三件套即快照档；`format: 3` 随 P3
     实现升版）。
  4. 新建根 `CONTEXT.md` 术语表（引擎 / 引擎档 / 快照档 / 用户世界 / 引导 / 就绪判定 /
     引擎目录）。
- 影响：**宪法级**——红线 2 生效文本变化；装配方与壳按 contract v3 对接（P3 落地前
  v2 语义仍有效）。实现排期：Spike 实机验证 → P1（TooOld 死局过渡修复，可独立先行）
  → P2（子进程环境自构）→ P3（倒置落地）。
- 凭据：纯文档改动不触运行时；pnpm12 事实核查锚定 v12.3.1 源码（镜像注入通道 /
  CANNOT_MANAGE_NODE 判定 / 两段式安装 / 进度输出）；环境准备阶段全节点经 grilling
  三轮逐项裁定（台账见 ADR §7）。

### 2026-09-01 发版通知 · v0.9.4 Windows/WSL 启动体验优化与 WSL Node 自动补齐体系 —— guan（AI 协作）

- 变更：
  1. **WSL 客体内 Node.js 全自动补齐**：落地 ADR-0004 §7 裁定，通过 npmmirror 镜像链在客体内自动拉取并解压官方 tarball 至 `~/.dsh-dock/node`，`guest_prep!` 宏优先将其注入 PATH，全自动串联 `node -> dsh` 安装链。
  2. **启动准备阶段顶栏精简（方案 b）**：环境准备与宿主解析阶段精简 Header，隐藏未就绪的控制中心入口与品牌文字，保留透明可拖拽区域，就绪后恢复控制中心入口。
  3. **首次环境准备等待体验升级**：下载达到 100% 后平滑淡出，细化宿主解析、依赖准备与包管理器补齐阶段的状态提示，消除静默等待卡顿感。
  4. **模式切换双向感知**：启动页模式切换按钮动态感知当前环境，WSL 模式显示「在本机中打开」并调用 `chooseMode("local")`，Local 模式显示「在 WSL 中打开」，中英文字典同步更新。
  5. **控制中心窗口 WebView2 背景色防护**：注入统一背景色配置，消除 Win32 HWND 创建时 WebView2 异步加载白屏闪烁。
- 影响：仅周知。v0.9.4 发版完成并打 tag。
- 凭据：`cargo test` 176 个单测全绿（含客体 Node 探测新单测）+ `cargo fmt --check` + `cargo clippy -- -D warnings` 零告警通过。

### 2026-09-01 发版通知 · v0.9.3 主工作台与控制中心双向快捷切换胶囊体系 —— guan（AI 协作）

- 变更：
  1. **双向快速切换入口（零遮挡架构）**：
     - DSH 主工作台注入胶囊：`top: 8px; left: 50%` 居于顶部开阔空白区，支持鼠标自由拖拽；
     - 控制中心顶栏胶囊：`QuickDshSwitcher` 内嵌于 Header 工具区，彻底移除底部悬浮层；
     - 两端统一交互：平时仅展示极简文本，鼠标 Hover 时平滑单行展开（强制 nowrap）快捷键徽章。
  2. **一键双向 Toggle 快捷键与跨窗口事件广播**：
     - 共用一组快捷键 `⌘,`（macOS）/ `Ctrl+,`（Windows/Linux）在主工作台与控制中心之间来回 Toggle 切换；
     - 新增 `app:settings-changed` 事件广播，工作台胶囊开关（即刻挂载/卸载）与快捷键风格切换实时响应；
     - `AGENTS.md` §6、§7 规范登记 `open_profiles_window`、`focus_main_window`、`showFloatingSwitcher`、`switcherShortcut` 与 `app:settings-changed`。
  3. **控制中心窗口与偏好设置升级**：
     - 控制中心默认打开尺寸提升至 `1180x780`（开箱即为宽屏双栏工作台）；
     - 「系统控制台 -> 偏好设置」增加工作台悬浮胶囊开关与快捷键风格偏好配置。
  4. **跨平台原生适配加固**：
     - `profiles` 独立窗口注入 `platform_script`，`host.ts` 增强平台 fallback 识别，确保快捷键准确匹配操作系统按键。
- 影响：仅周知。v0.9.2 发版完成并打 tag。
- 凭据：`cargo test` 175 个单测全绿 + `cargo fmt --check`；前端 `npm run typecheck` + Vitest 92 个单测全绿。

- 变更：
  1. **插件中心统一视图**：Profile 管理器市场/总览两个 Tab 合并为「插件中心」单入口（`PluginHub.tsx` 子 Tab 承载），分类下拉改平铺标签矩阵（可展开收起），排序项矢量图标化。
  2. **跨 Profile 安装去重折叠**：导入弹窗同名插件按包名聚合 + 来源 Profile 标签切换器（`groupPickerCandidates`），独立绑定配置复制。
  3. **模型凭据别名识别**：`credentials.rs` 支持 `refs.DEEPSEEK_API_KEY` 等大写环境变量与别名映射，修复「已配置仍提示未配置」。
  4. **Release Notes 规范化体系**：新增模板与生成规范，构建期脚本优先读 `docs/RELEASE_NOTES.md`。
  5. **下线 `reset_profile_dependencies`**：IPC 登记/capabilities/前端入口三处同步清理，依赖自愈回归 pnpm 自动补齐。
  6. **i18n 与设计规范对齐**：组件全面切换 `useI18n()`；清理非标准 emoji 与冗余文案；悬浮提示补齐；Profile 彩色确定性标签。
- 影响：仅周知。v0.9.2 已打 tag，进入冻结期（master 只收 fix）。
- 凭据：`cargo test` 175 个单测全绿 + `clippy -D warnings` 零告警 + `cargo fmt --check`；前端 `typecheck`/`lint`/92 单测全绿。

### 2026-09-01 完成通知 · 启动环境检查提速（--no-open 探测缓存 + 流式早退 + 超时兜底） —— guan（AI 协作）

- 变更：`src-tauri/src/resolve.rs` 环境检查链路优化 + `AGENTS.md` §6 例外册登记：
  1. **跨启动持久化缓存**：`--no-open` 能力探测结果（dsh 版本 → bool）落 `<data_dir>/probe-cache.json`
     （原子写、损坏/缺失回退探测、不阻断 boot），稳态启动零 spawn（实测原 8~11s → 缓存命中 ≈0）。
  2. **流式读数早退**：`probe_no_open` 从 `output()` 等自然退出改为双线程读 stdout/stderr
     （usage 可能打到任一流），命中 `--no-open` 即 kill 早退——缓存 miss 时 1.4s 返回（原 8~11s）。
  3. **硬超时兜底**：20s 探测总超时（原无超时，探测进程卡死会永久阻塞 boot）；超时按
     「不支持」处理（不传 `--no-open`，保持旧版 dsh 秒退防护默认值）。
  4. 新增 8 个单测（缓存命中零 spawn / miss 写回 / 流式早退 / stderr 命中 / 卡死超时 /
     版本变化失效 / 缓存损坏回退 / 往返），全仓 `cargo test` 175 passed，`clippy -D warnings` 通过。
  5. `AGENTS.md` §6 运行时持久化例外册登记 `probe-cache.json`（可丢失可重建缓存）。
- 影响：仅周知。日常启动环境检查从 ~8~11s 降至 ~0.3s（仅剩 PATH/node 探测子进程开销）；
  探测进程卡死不再阻塞启动。
- 凭据：本机实测（macOS + fnm node v24.18.0 / dsh 0.1.1-rc.2）：旧实现 `--help` 探测 7~11s、
  新流式早退 1.3s；`cargo test` 175 全绿 + `cargo clippy --all-targets -- -D warnings` 零告警。

### 2026-08-31 发版通知 · v0.9.1 社区插件市场全量上线（awesome-dsh-plugin Registry 2700+ 插件发现与一键安装分发） —— guan（AI 协作）

- 变更：
  1. **社区插件市场（Registry 集成）**：`src-tauri/src/updates.rs` 接入 `awesome-dsh-plugin.com/plugins.json` 官方 Registry（单一网络面，带 3MB 上限与 GitHub raw 镜像兜底），新增 `fetch_market_registry` IPC 命令与外链域名白名单登记。
  2. **高质感插件市场工作台（`/frontend-design`）**：`ProfileManager.tsx` 新增第 5 个视图 Tab（`market`），引入 `MarketplaceView.tsx`、`MarketPluginCard.tsx` 与 `MarketInstallDialog.tsx`。
  3. **丰富发现与筛选能力**：支持全局搜索（名称/NPM/作者/描述）、22 个垂直分类快速筛选 Chips、四向排序（Stars ⭐ / Downloads ⬇️ / 最新 🆕 / 名称 🔤）与已安装插件过滤。
  4. **全域状态联动与一键分发**：插件卡片与全域已装 Profile 芯片实时联动（带绿色脉冲圆点），支持一键安装到指定 Profile 或分发至其他 Profile。
  5. **国际化与单测补齐**：中英多语言字典全量支持；新增 `market.test.ts` 14 个单测，全仓 166 Rust 单测 + 85 前端单测全绿。
- 影响：仅周知。用户可在 DSH Dock 内直接浏览、检索与安装 2700+ 社区插件。
- 凭据：`cargo test` 166 个单测全绿；前端 `npm run typecheck && npm run lint && npm run test && npm run build` 全量通过。

### 2026-08-31 完成通知 · 问题 9 & 10 深度优化（本地路径打开修复 + 系统控制台极简侧栏导航重构） —— guan（AI 协作）

- 变更：
  1. **问题 9（本地路径打开）**：`src-tauri/src/lib.rs` 升级 `open_external` 命令，判断参数为本地文件系统路径时调用系统文件管理器（Finder / File Explorer）直接打开该路径或其父目录，彻底解决之前被 Web URL 白名单拦截报错「不允许的外链」的问题。
  2. **问题 10（两层导航极简重构）**：严格遵循 `/frontend-design` 规范对 [`SystemConsole.tsx`](file:///Users/guan/git/realguan/dsh-plugin-hub/dsh-dock/frontend/src/components/system/SystemConsole.tsx) 进行极简与去噪重构：彻底移除左侧顶部冗余啰嗦的长句描述，将导航项升级为极简高级的轻量侧栏（单行微底色 Icon + 纯粹标题 + Active 边框态），消除多余边框包裹与视觉杂讯，与整体界面实现极致协调统一。
- 影响：仅周知。会话维护与系统控制台操作体验与视觉质感显著提升。
- 凭据：`cargo test` 166 个单测全绿；前端 `npm run typecheck && npm run test` 12 个测试套件 71 个单测全绿。

### 2026-08-31 完成通知 · 自动化发布日志流水线（GitHub Release & 客户端「关于」更新说明无缝打通） —— guan（AI 协作）

- 变更：全链路打通 GitHub Release 与桌面客户端自更新日志：
  1. **自动化发布日志提取引擎**：新增 [`scripts/extract-release-notes.py`](file:///Users/guan/git/realguan/dsh-plugin-hub/dsh-dock/scripts/extract-release-notes.py)，在打 tag 发布时自动从 `docs/broadcasts.md` 提取匹配当前版本的权威变更条目（包含变更要点、架构决议与测试凭据），并支持回退到 `git log`。
  2. **GitHub Actions 构建流无缝注入**：重构 [`.github/workflows/build.yml`](file:///Users/guan/git/realguan/dsh-plugin-hub/dsh-dock/.github/workflows/build.yml) 中的 `release` 任务，自动将提取出的 Markdown 升级日志写入 GitHub Release `body_path`，并注入到桌面自更新清单 `latest.json` 的 `notes` 字段中（彻底解决之前固定硬编码 `"DSH Dock v0.x"` 导致客户端无更新日志的缺陷）。
  3. **客户端「关于」更新卡片体验增强**：[`ClientUpdateCard.tsx`](file:///Users/guan/git/realguan/dsh-plugin-hub/dsh-dock/frontend/src/components/about/ClientUpdateCard.tsx) 升级 Release Notes 视窗，支持滚动阅读与一键「展开全部 / 收起」切换，让用户在应用内检查更新时能够清晰浏览完整的更新内容。
- 影响：仅周知。后续所有通过 git tag 触发的 CI 发布将全自动生成详尽的 GitHub Release 页面，且客户端关于窗口在检查到新版本时能完整呈现版本升级日志。
- 凭据：本地执行 `python3 scripts/extract-release-notes.py` 验证通过；`cargo test` 166 个单测全绿；前端 `npm run typecheck && npm run test` 12 个测试套件 71 个单测全绿。

### 2026-08-31 完成通知 · 8 项深度用户体验与逻辑缺陷全量优化（Bento 宫格插件市场、智能贪婪路径反解、凭据元数据过滤、诊断大盘缓存、命名升级控制中心） —— guan（AI 协作）

- 变更：全面落实 `问题记录.md` 中的 8 大反馈与优化建议：
  1. **新建 Profile 按钮布局**：侧边栏顶部独立设计高亮醒目的 `+ 新建工作台` 主操作按钮，与下方搜索框形成清晰主次操作流。
  2. **重置依赖感知增强**：明确弹窗文案（彻底清理 `node_modules` 并基于 `package.json` 通过 pnpm 纯净重装），执行态展示 Spinner 与防重点击，Toast 明确返回重置结果。
  3. **插件分发 Select 截断修复**：消除 Radix `SelectTrigger` 嵌套 span 截断问题，设置 `min-w-[240px]` 完整展示 Profile 名称。
  4. **插件总览 Bento 宫格卡片 + 分页 + Profile 筛选**：`PluginOverview.tsx` 重构为现代自适应 Bento Grid 宫格卡片，新增 Profile 专属下拉筛选器与分页控制器（支持 6/9/12/18 条切换与页码导航）。
  5. **会话项目分组、路径贪婪反解与 Icon 纠正**：`sessions.rs` 引入文件系统智能探测贪婪匹配算法，精准还原带连字符真实物理路径（`/Users/guan/git/realguan/dsh-plugin-hub/dsh-dock`）并提取简洁项目名（`dsh-dock`）；`SessionManager.tsx` 规整卡片上下双层结构，纠正复制会话路径 Icon 为 `Copy`。
  6. **凭据元数据过滤**：`credentials.rs` 引入 `RESERVED_METADATA_KEYS` 严格过滤 `version`、`refs`、`schema` 等非 Provider 元数据键，新增专属单测。
  7. **健康大盘缓存与 Icon 纠正**：`DiagnosticsPane.tsx` 引入 60 秒内存缓存机制实现秒开无感切换，保留手动「刷新体检」按钮；复制诊断报告按钮纠正为 `Copy` 图标。
  8. **产品命名统一升级**：程序菜单栏与独立窗口标题由「Profile 管理器」统一升级为更名副其实的「控制中心 (Control Center)」。
- 影响：仅周知。全栈用户体验与交互质感大幅提升。
- 凭据：`cargo test` 166 个单测全绿；前端 `npm run typecheck && npm run test` 12 个测试套件 71 个单测全绿，0 报错。

### 2026-08-31 完成通知 · v0.9.0 深度对齐审核报告与全量单元测试补齐（165 Rust 单测 + 71 前端单测全绿） —— guan（AI 协作）

- 变更：全量落实《DSH Dock v0.9.0 规划与可行性深度审核报告》的核心设计建议与落地红线，并补齐全部功能的边界单测：
  1. **4.5 凭据脱敏与 DSH 引擎全局设置**：Rust 新增 `dsh_settings.rs`（`settings.yaml` 安全原子读写）与 `credentials.rs`（脱敏掩码算法 `mask_api_key`、`get_credentials_summary`、`set_credential_key` 0600 权限）；前端 `CredentialsPane.tsx` 升级为结构化卡片与独立修改弹窗，新增 `DshSettingsPane.tsx` 管理 DSH 核心引擎配置。补齐掩码边界、清除/删除、多 Provider 并存单测。
  2. **4.6 会话与工作区真实路径联动**：Rust `sessions.rs` 实现 `decode_project_dir_to_path`，精准反解真实工作区绝对路径（含 Windows 盘符与 Unix 路径）；前端 `SessionManager.tsx` 支持按项目聚合分组折叠与一键在访达/资源管理器中打开项目。补齐各种异常目录名反解与序列倒退自愈重排备份单测。
  3. **4.7 MCP 服务器可视化结构化 CRUD 与运行态联动**：Rust `mcp.rs` 实现针对 `@deepseek-ai/dsh-mcp-client` 的结构化 CRUD，通过内存合并避免 Cordis Patch 覆盖非 MCP 插件条目；前端 `McpManager.tsx` 支持 GitHub/Filesystem/Postgres/Brave Search 预设一键应用、完整表单编辑与运行态 `mcp__<server>__*` 工具提取联动。补齐保留非 MCP 插件条目、更新已有 server、删除幂等单测。
  4. **4.11 / 4.12 诊断体检与多源日志流**：Rust `diagnostics.rs` 探测 Node/pnpm/dsh 运行状态并收集存储水位，提供多源日志安全尾部截断 `get_app_logs`；补齐各 source 路由与未截断/截断单测。
  5. **4.13 多语言深度对齐**：前端 `i18nStore.test.ts` 采用全量递归深度遍历断言，确保 `zh-CN.ts` 与 `en-US.ts` 所有层级 key 100% 深度对称无漏项。
- 影响：仅周知。全栈代码质量与测试覆盖率达到最高标准，保证所有新增功能与边缘场景均有机器单测严格守护。
- 凭据：`cargo test` 165 个测试全绿（165 passed, 0 failed）；前端 `npm run typecheck && npm run test` 12 个测试套件 71 个单测全绿（71 passed, 0 failed, 0 type errors）。

### 2026-08-31 完成通知 · v0.9.0 规划全量落地：稳定性守护、系统控制台与运维大盘、多语言基线、MCP 扩展与依赖重置 —— guan（AI 协作）

- 变更：全量三阶段（刀 1/2/3）落地收口：
  1. **稳定性、运维与多语言（4.11 + 4.12 + 4.13）**：Rust 接入崩溃守护熔断器 `guard_session`（60s 内 3 次崩溃触发熔断与诊断提示）；实现了 `diagnostics.rs`（系统环境健康体检、Node/pnpm/dsh 探测、存储分布递归统计、安全分页日志查看器 `get_app_logs`）；新增持久化字段 `locale` 与 `autoRestart`；前端落地完整中英双语国际化 `i18nStore` 与响应式翻译。
  2. **会话与设置大盘（4.6 + 4.5）**：实现 `credentials.rs`（`.credentials.yaml` 0600 安全权限原子写与脱敏查看）；实现 `remove_session` 物理删除会话与路径快捷复制；前端构建全新控制中心 `SystemConsole.tsx`（偏好设置、凭据安全编辑器、系统健康大盘、暗色终端日志视窗）。
  3. **MCP 生态与维护收口（4.7 + 4.11）**：实现 Profile 依赖一键重置 `reset_profile_dependencies`；前端实现 `McpManager.tsx` 可视化 MCP 服务器管理（支持 GitHub / Postgres / Brave Search / Filesystem 等预设与工具前缀一键复制）。
  - 新增 8 个 IPC 命令严格完成三处同步与 `AGENTS.md` 登记；全部通过 `gate_tests` 机器闸门。
- 影响：仅周知。v0.9.0 规划的全部目标均已高质量闭环交付，所有前端界面均严格遵循 `/frontend-design` 规范打造，兼具高质感设计与可靠健壮性。
- 凭据：Rust 侧 158 单元测试全部通过（158 passed, 0 failed）；前端 58 单元测试全部通过（58 passed, 0 failed, 0 type errors）。

### 2026-08-31 修复 · WebView 内存策略改用直接子代选择器（解决工具调用嵌套导致的大面积滚动空白与输入框悬空）—— guan（AI 协作）

- 变更：本 commit——`src-tauri/src/lib.rs`（`WEBVIEW_MEMORY_POLICY_SCRIPT` 中 CSS 规则由后代选择器 `FLOW + ' ' + ROW` 改为直接子代选择器 `FLOW + ' > ' + ROW`；`syncStreaming` 仅扫描顶层行；更新单测 `webview_memory_policy_script_contains_list_padding_defense` 断言直接子代连接符 `FLOW + ' > ' + ROW`）、`docs/adr/0002-webview-memory-policy.md`（补录直接子代修订说明）。
- 影响：仅周知。解决由于工具调用内部卡片（`ToolCallTree` 的 `callRow` / `subCalls`）带有相同的 `data-chat-anchor-key` 属性而触发多层嵌套 containment，导致 WebKit 严重虚高估算滚动高度、在内容到底后仍可下滑并露出大片空白、输入框脱离底部悬空的偶现 bug。
- 凭据：`cargo test` 152 绿 / `cargo fmt --check` / `cargo clippy -- -D warnings` 全绿；前端 test 55 绿。

### 2026-08-31 功能 · 会话自愈与健康维护面板（ProfileManager 会话工作台）—— guan（AI 协作）

- 变更：本 commit——新增 `src-tauri/src/sessions.rs`（扫描 `$DSH_HOME/sessions/`、单会话/全量自愈）、`scripts/repair-session.mjs`（会话日志 Turn 归流、seq 连续化重排与 Zstd 校验重打包）；`ipc.rs` / `lib.rs` / `capabilities/default.json`（登记 `list_sessions` / `repair_session` / `repair_all_sessions`）；`AGENTS.md` §7；`frontend/src/components/profiles/SessionManager.tsx`；`frontend/src/pages/ProfileManager.tsx`（三视图切换新增「会话维护」）；`frontend/src/content/zh-CN.ts` 与 `frontend/src/types/ipc.ts`。
- 影响：**触宪法级**——`AGENTS.md` §7 登记 3 项新 IPC 命令（IPC 三处同步严格保持一致）。不破坏 dsh 文件系统不变量，修复时自动备份原始 `.bak`，解决上游并发推流或断线重连导致的 "seq gap in committed region" / "history unavailable"。
- 凭据：`cargo test` 152 绿（新增 3 单测）/ `cargo fmt --check` / `cargo clippy -- -D warnings` 全绿；前端 typecheck 0 报错 / test 55 绿 / build 产物成功生成。

### 2026-08-31 修复 · WebView 内存策略 CSS 补齐列表内边距（解决 Paint Containment 裁切列表序号）—— guan（AI 协作）

- 变更：本 commit——`src-tauri/src/lib.rs`（`WEBVIEW_MEMORY_POLICY_SCRIPT` 常量化，注入 CSS 追加 `FLOW ol, FLOW ul { padding-left: 1.5em !important; }` 防止 `content-visibility: auto` 隐式 Paint Containment 把挂在行左侧外沿的有序/无序列表 markers 裁切截断；增加单测 `webview_memory_policy_script_contains_list_padding_defense`）、`docs/adr/0002-webview-memory-policy.md`（补录 2026-08-31 修订说明）。
- 影响：仅周知。保持长会话 WebKit 内存优化不变，彻底解决桌面端会话中有序列表序号（如 `1. 2. 3.`）只展示一半的问题。
- 凭据：`cargo test` 149 绿（+1 单测）/ `cargo fmt --check` / `cargo clippy -- -D warnings` 全绿；前端 test 55 绿。

### 2026-08-31 修复 + 发版事项 · 插件总览 UX 修正（包名展示全 / 描述降级）+ v0.8.0 tag（4.4 收口）—— guan（AI 协作）

- 变更：本 commit——PluginOverview 行布局重排（维护者实机截图反馈：包名被
  描述挤压截断、描述喧宾夺主）：包名独占整行 `break-all` 尽量展示齐全（超长
  折行不截断）、分布 chips 次行、描述降级为末行单行截断辅助信息；随后
  `chore: 版本 0.8.0` + 注解 tag `v0.8.0`，master 与 tag 推送。
- 影响：**冻结期重启**——v0.8.0 为新冻结点：Release notes 至三平台产物验收
  期间 master 只收 fix。Release notes 草稿见附录（覆盖 v0.7.0 → v0.8.0 全量：
  4.4 收口 ade90bd、升级链路事件化 83fa74b、可安装性预检 704f7ff、本 UX 修正）。
- 凭据：前端 typecheck / lint / test 55 绿（Rust 侧本批零改动，0.8.0 前全量
  147 绿见 ade90bd 凭据）。

**附录：Release notes 草稿（v0.7.0 → v0.8.0）**

```markdown
## 新增
- 插件总览：Profile 管理器页内新视图，聚合展示全部 profile 的第三方插件
  分布——包名独占整行尽量展示齐全，各 profile 实装版本一目了然（只读，
  纯本地文件扫描）
- 从其他 profile 安装插件：详情对话框「从其他导入」多选批量，默认安装来源
  同版本（装验证过能用的）；串行队列单项失败不中断，末尾汇总成败与明细
- 「连配置」可选搬移：勾选后把来源 profile 里该插件的配置行原样复制到目标
  （默认不勾；只追加不覆盖目标已有配置，ADR-0009 写入例外 #4）

## 修复
- DSH 升级「点了没反应」：升级链路全程事件反馈——真实进度与失败详情可见
- 升级前可安装性预检：目标版本依赖未完整发布时立即失败并指名缺失包，
  不再等数分钟后才失败
- 插件总览包名被描述挤压截断（包名独占整行、描述降级末行）

## 变更
- 插件管理器五个核心能力收口：清单 / 安装卸载更新 / 禁用启用 / 更新标识与
  选版本 / 跨 profile 安装
- AGENTS §7 IPC 名册 +2（list_all_plugins / copy_plugin_config）

## 已知问题
- 插件安装进度为单行 busy（pnpm 流式输出未回传）
- npm 搜索未实现（挂账）
```

### 2026-08-30 完成通知 · 4.4 收口：插件总览聚合 + 从其他 profile 安装（ADR-0009 第五次修订，patch 写入例外 #4）—— guan（AI 协作）

- 变更：本 commit——维护者经 grilling 逐条确认后把原「跨 profile 复制」重定义为
  ①**插件总览**（管理器页内切换视图，`list_all_plugins` 只读文件扫描聚合全部
  已物化 profile 的第三方插件，内置 bundle 不进聚合）与②**从其他 profile 安装**
  （详情对话框多选批量选择器，串行队列失败继续末尾汇总，版本默认来源同版本
  `pkg@ver`；「连配置」逐行可选勾选=来源 cordis.patch.yml 该插件行 id 全部条目
  原样复制，`copy_plugin_config` 只追加不覆盖——**写入例外 #4**，`PluginRowState`
  扩展 `patch_entries` 供置灰预检）。新 IPC 两枚三处同步 + AGENTS §7 登记；
  ADR-0009 第五次修订先行落档；roadmap 4.4 落地记录回写（五核心能力全齐）。
- 影响：仅周知。npm 搜索（4.4⑤）维持挂账；注意 `updates.rs` 尚有另一条工作线的
  H-1 预检未提交改动，本 commit 未包含、未触碰。
- 凭据：cargo test 147 绿（新增聚合归组 / 配置行原样复制与不覆盖 / patch 条目
  计数等 4 测试）+ fmt/clippy 干净；前端 55 测试绿（新增候选过滤与批量汇总
  纯逻辑）+ typecheck/lint 干净；IPC 一致性 gate_tests 兜底。

### 2026-08-30 排障 + 修复 · DSH 升级「点了没反应」——根因 = 不可安装的 alpha 版本 + 失败不可见；升级链路事件化 —— guan（AI 协作）

- 变更：本 commit——`lib.rs`（`terminal_action` 升级链路新增 `dsh:upgrade`
  事件 running/done/failed，failed 携带安装器完整错误链含 pnpm 输出尾部），
  `DshVersionCard`（升级 busy 改事件驱动真实时长，失败显示错误详情——原 2s
  固定假 busy，之后数分钟全程无反馈）、`ClientUpdateCard`（检查更新按钮
  busy 时禁用 + 内联转圈——原实现整组消失，无动效）。
- 影响：仅周知 + **一项裁定待议**：根因是 H-1 检查口径（排序最高，rc/预发布
  也追）会把 `0.1.2-alpha.2` 这种**依赖未发布的不可安装版本**提示为「有新版」
  （ledger 已记边界）。候选：a) 检查侧做可安装性预校验（每依赖一查，成本高）；
  b) 口径改 dist-tag 优先（推翻 H-1，需裁定）。现维持 H-1 + 失败可见。Windows
  侧注意：`install_global_dsh_with_prefix` 的回退链未变。
- 凭据：根因实机复现——shell.log 14:41-14:49 多次 `pnpm add -g
  @deepseek-ai/dsh@0.1.2-alpha.2` 双 registry 全败；手工复现同错
  （ERR_PNPM_META_FETCH_FAIL，依赖走死域名 r.cnpmjs.org）。cargo test 142 绿 /
  fmt / clippy；前端 typecheck / lint / test 49 绿。

### 2026-08-29 发版事项 · v0.7.0 tag（插件管理器全量 + Profile 重启），冻结期随验收重启 —— guan（AI 协作）

- 变更：本 commit——详情对话框行内操作防溢出修复（hover 操作组与运行徽标
  display 换位，长包名截断兜底）+ roadmap 4.4 落地记录回写；随后
  `chore: 版本 0.7.0` + 注解 tag `v0.7.0`，master 与 tag 推送。
- 影响：**冻结期重启**——v0.6.0 冻结期经维护者裁定提前开工 feat（见当日
  4.4① 批次条目），本 tag 即新的冻结点：Release notes 至三平台产物验收
  期间 master 只收 fix。Release notes 草稿见附录。
- 凭据：前端 typecheck / lint / test 49 绿；Rust 142 / fmt / clippy 全过。

**附录：Release notes 草稿（v0.6.0 → v0.7.0）**

```markdown
## 新增
- Profile 重启按钮（运行中行内，确认后同 profile 重启——插件变更借此生效）
- 插件安装 / 卸载 / 更新：详情对话框行内操作（dsh plugin 转发链，规格校验
  防参数注入，安装/卸载/更新均带「重启后生效」提示）
- 插件禁用 / 启用：cordis.patch.yml `{id, disabled}` 单键切换（ADR-0009
  第四次修订，写入例外 #3），行 id 经 dump-config 权威解析
- 插件更新标识 + 选版本更新：registry dist-tags 口径（镜像链 npmmirror →
  npmjs），版本选择弹窗标 最新/当前
- Profile 详情对话框：插件清单卡（内置/外挂、实装版本、运行态徽标、
  会话运行汇总）、多插件防溢出（对话框限高 + 分区滚动）

## 修复
- 行内操作组被卡片右缘裁切（hover 与徽标 display 换位 + 长包名截断）
- 多插件时详情对话框超出屏幕（基件无高度上限）

## 变更
- 外挂插件在徽章区与卡片区去重显示（dsh reconcile 数据模型本然双写）
- AGENTS §7 IPC 名册 +9、网络面 +2（回环运行态查询、registry 外网检查）

## 已知问题
- 安装进度为单行 busy（pnpm 流式输出未回传）
- 跨 profile 插件复制未实现（4.4 收尾项）
```

### 2026-08-29 完成通知 · 4.4④ 插件更新标识 + 选版本更新落地（registry 外网镜像链）—— guan（AI 协作）

- 变更：本 commit——`updates.rs`（`npm_packument_versions`：任意 npm 包
  packument 查询，与 dsh 版本检查同镜像链 npmmirror → npmjs / 同超时 / 同
  体积上限 + `parse_packument_versions` 纯函数 semver 升序排序 + 测试）、
  `plugins.rs`（`check_updates_blocking`：逐外挂插件查 dist-tags.latest——
  口径与 pnpm 默认安装一致，复现点 7 教训；current ≥ latest 不报；奇异名
  跳过不打 registry；`plugin_versions_blocking` 全版本降序）、`lib.rs`/
  `ipc.rs`/`capabilities`（`check_plugin_updates` / `list_plugin_versions`
  三处同步）、前端（外挂插件区「检查更新」按钮 + 行内 `0.16.1 ↑0.17.0`
  更新标识，点开版本选择弹窗——标 最新/当前，选定走既有安装链
  `pkg@version`）。
- 影响：**触宪法级**——`AGENTS.md` §7 两处：IPC 名册 +2；网络面登记新
  外网用途「插件更新检查」（registry packument，同镜像链）。本条即知会。
  语义：检查为按钮触发不自动跑（N 包串行查询，避免开窗即外网风暴）；
  dist-tag 口径意味着 dsh-base 那种「latest 停在坏版本」的包不会被误标
  升级目标之外——选版本弹窗可见全部版本自行决定。
- 凭据：cargo test 142 绿（+2 packument 解析含 semver 排序反例）/ fmt /
  clippy 全过；前端 typecheck / lint / test 49 绿。

### 2026-08-29 完成通知 · 4.4③ 禁用/启用插件落地（patch 单键切换，ADR 第四次修订实施）—— guan（AI 协作）

- 变更：本 commit——`Cargo.toml`（新增 `serde_yaml 0.9`，ADR 已裁定接受停维护
  风险，读写收敛在 `set_plugin_disabled` 单函数便于后继替换）、`plugins.rs`
  （`plugin_rows_blocking`：`dsh --profile <名> --dump-config` 行 id↔包名配对 +
  壳 toggle 态，行级扫描避开 `!!js` 标签；`set_plugin_disabled`：patch 顶层数组
  读改写——禁用置/追加 disabled 键、启用移键或整条移除，头部注释块保真，
  非顶层数组拒绝写入）、`lib.rs`/`ipc.rs`/`capabilities`（`get_plugin_rows` /
  `set_plugin_disabled` 三处同步）、前端（行内电源开关：禁用态常驻灰徽 +
  包名划线，运行徽标只对启用中插件显示；操作带「重启后生效」提示）。
- 影响：仅周知。行 id 权威来源 = dump-config 行表（一次 spawn 秒级，对话框
  打开时异步取）；重启按钮承接生效。
- 凭据：cargo test 140 绿（+3：toggle 幂等与注释保真、config 键保全、
  dump 行配对解析）/ fmt / clippy 全过；前端 typecheck / lint / test 49 绿。
  实机数据锚定见 ADR 第四次修订注。

### 2026-08-29 完成通知 · Profile 重启按钮 + 详情去重 + 禁用/启用 ADR（维护者四项指令批次 1/2）—— guan（AI 协作）

- 变更：本 commit——① 重启按钮（运行中行 RotateCw，同 profile 走切换链
  `switch_profile`，恒弹确认，弹窗文案按重启语义分叉）；④ 详情对话框去重
  （reconcile 把外挂同时写进 bundles 与 dependencies——徽章区只留层叠内置
  层，隐藏数 >0 给指引行，台账复现点 7）；③ 决策先行：**ADR-0009 第四次
  修订**——patch 写入例外 #3（`cordis.patch.yml` 的 `{id, disabled}` 单键
  切换）：行 id 不可从包名推导（实测 commandcode→`llm-commandcode`），来源
  定死 dump-config 行表；serde_yaml 0.9 读改写（注释头部保真策略）；运行中
  不热生效、重启承接；生效真相 = 壳自家 patch 条目。
- 影响：仅周知；③ 的**实现**（serde_yaml 依赖 + IPC + 行内开关 UI）随后续
  commit 落地，② 更新标识（npm registry 外网查询，§7 需新登记）排最后。
- 凭据：前端 typecheck / lint / test 49 绿；行 id 映射实测锚定（本机 web 档
  dump-config）。

### 2026-08-29 完成通知 · 4.4② 插件安装/卸载/更新落地：详情对话框行内操作 —— guan（AI 协作）

- 变更：本 commit——`plugins.rs`（`validate_plugin_spec` 纯校验：防 pnpm 旗标
  注入（前导 `-` 当参数）、控制字符/空白；scope 包名与版本段（tag/精确/^~
  区间）放行，`><` 语义区间 v1 不开 + `mutate_plugin_blocking`：`dsh plugin
  --profile <名> add/remove/update <spec>` 转发链复用创建刀基建，pnpm 防御
  补齐同源，未物化/非法名先拒不 spawn，超时同创建 600s，失败附 dsh 输出
  尾部）、`lib.rs`/`ipc.rs`/`capabilities`（`install_plugin` / `remove_plugin`
  / `update_plugin` 三条 IPC 三处同步）、前端（详情对话框区头「安装插件」
  输入行 + 行内更新/卸载 hover 操作 + busy 态 + 结果分箱展示；spec 预检
  `validatePluginSpec` 镜像后端校验 + Vitest）。
- 影响：**触宪法级**——`AGENTS.md` §7 IPC 名册 +3。本条即知会。语义边界：
  装到**运行中**的 profile 时 dsh 不热重载，壳侧成功文案带「重启后生效」；
  add 裸包名 dist-tag 坑由输入占位引导带版本段规避（复现点 7）；安装进度
  v1 为单行 busy（不订阅 pnpm 流式输出），后续独立插件管理视图再升级。
- 凭据：cargo test 137 绿（+2：spec 恶意反例集、未物化/非法名先拒不
  spawn）/ fmt / clippy 全过；前端 typecheck / lint / test 49 绿（+2 镜像
  校验）。

### 2026-08-29 完成通知 · 4.4① 插件清单落地：详情对话框插件卡 + 运行态回环快照 —— guan（AI 协作）

- 变更：本 commit——`plugins.rs`（新模块：静态清单 = bundles + dependencies +
  node_modules 已装版本/描述；运行态 = `POST /api/pluginInventory/list` 回环
  只读快照，2s 超时，Spike B 方案 + 复现点 11）、`lib.rs`/`ipc.rs`/
  `capabilities`（`list_profile_plugins` / `get_plugin_runtime` 两条 IPC 三处
  同步）、前端（详情对话框依赖区升级为插件卡：官方/第三方、已装版本、运行态
  徽标，快照按 profile 匹配合并防张冠李戴；纯函数 `runtimeChipFor` /
  `runtimeSummary` + Vitest）。
- 影响：**触宪法级**——`AGENTS.md` §7 两处：IPC 名册 +2；网络面登记新例外
  「插件运行态回环只读查询」（127.0.0.1、只读、仅活跃会话、一次性快照）。
  本条即对该宪法修订的知会。**冻结期说明**：v0.6.0 验收仍待三平台产物，
  本 feat 经维护者当日裁定提前开工（「直接开工吧」），验收并行不受影响。
- 凭据：cargo test 135 绿（+4：静态清单/非法名/信封形状/响应解析）/ fmt /
  clippy 全过；前端 typecheck / lint / test 47 绿（+7 运行态合并纯逻辑）；
  运行态获取路径已在本机运行中的 dsh 实例实机打通（Spike B 实测记录）。

### 2026-08-29 发版事项 · v0.6.0 tag 已推送，冻结期开始 —— guan（AI 协作）

- 变更：commit `5ef27ab`（`chore: 版本 0.6.0（Profile 管理器全量 + Profile 切换）`，
  bump Cargo.toml / tauri.conf.json / package.json + 双 lock）+ 注解 tag `v0.6.0`
  （指向 `5ef27ab`）；master 与 tag 已推 origin（`5cf0593..5ef27ab`）。tag `v*`
  触发 CI Release（CONTRIBUTING §8）。
- 影响：**冻结期开始**——Release notes 发出至三平台产物验收通过期间，master
  只收 fix 不收 feat。Release notes 草稿见本条附录，频道确认后随 Release 发布。
- 凭据：bump 后 cargo test 131 绿 / 前端 typecheck + test 40 绿；推送回执
  `master -> master` + `* [new tag] v0.6.0`。

**附录：Release notes 草稿（v0.5.1 → v0.6.0）**

```markdown
## 新增
- Profile 管理器（4.3 全量）：列表/详情（已物化 + 可首启模板两态合并）、
  创建（dsh plugin 转发链——零网络毫秒级，创建即 webUi 候选可设默认）、
  复制/重命名/删除（运行中防护、node_modules 删除 + dsh 自愈）、
  默认启动 profile 持久化与 boot 消费。
- Profile 切换（4.3⑥）：管理器行内「启动」= 停当前 dsh 以目标 profile
  重启；仅 webUi 候选；切换不写默认；运行中徽标实时（boot:step 广播订阅）；
  WSL guest 启动脚本参数化（profile 名 shell 引号安全）。
- pnpm 为 boot 硬依赖：缺失自动 `npm i -g pnpm` 补齐，失败阻断给可行动文案。
- IPC 三处同步机器闸门：ipc.rs 单一事实源，漏登记 cargo test 即红。

## 修复
- 创建路径 dist-tag 版本坑：`add @deepseek-ai/dsh-base` 裸名按 latest 解析到
  已弃用旧版 → 404 + 重试卡死；改 `install` 原始版语义。
- Profile 详情对话框：file: 依赖行溢出丢包名、cordis.patch.yml 折行失真、
  超长行 grid 撑破布局三连修。
- 工具窗口内容超高被裁顶：垂直居中改顶部锚定。

## 变更
- 对话框 footer 去脚手架灰底，收编为有意设计（静默化）。
- AGENTS.md 减法一轮 248→178 行；README/CONTRIBUTING/本地运行指引更新。

## 已知问题
- WSL 模式 Profile 切换待 Windows 真机人工验证（按 docs/executor.md 清单）。
- 多开（多 profile 并行多窗口）未排期：前置 = 双实例 storages 竞态 spike +
  1:1 生命周期 ADR 修订。
```

### 2026-09-08 完成通知 · 补丁包启用/禁用开关（4.4③ 补丁包形态补全）—— guan（AI 协作）

- 变更：本 commit——`plugins.rs`（`PluginRowState` 扩展 `contributed_ids`；
  行表解析 `parse_dump_rows_with_section` 带 bundle 段落归属 `# == <bundle>`
  （含 `, patched by <路径>` 后缀剥离——2026-09-08 实机实测）；`build_row_states`
  合成补丁包条目：id = 第一贡献行、shell_disabled = 贡献行**全禁用**（all 语义，
  部分禁用显示启用、切换一次收敛）、patch_entries 合计；`plugin_rows_blocking`
  读 manifest dependencies 走合成）、前端（`types/ipc.ts` 新字段；
  `lib/pluginToggle.ts` 开关目标纯函数；`ProfileDetailPane.toggleDisabled`
  补丁包多目标**串行**写——同一 patch 文件读改写，并发 invoke 相互覆盖）、
  文档（ADR-0009 第七次执行细则修订；ledger 复现点 13 入册——dsh 0.1.2-rc.1
  段落归属 + include 浅覆盖 + loader 组禁用继承；roadmap 4.4 落地记录）。
- 影响：无自身行的补丁包（实测 `@openviking/dsh-memory-plugin`、
  `@tt-a1i/archify-dsh`，`dsh.bundle.patch` 纯 insert 形态）此前无开关、用户
  误判「未安装/解析失败」——现在开关真实可切换：写 profile
  `cordis.patch.yml` 贡献行 `{id, disabled}`，重启生效（不热生效，同 4.4③
  口径）；bundle 级禁用第一类语义**不做**（dsh 无该命名空间）；id 来源仍定死
  dump-config 行表（第四次修订「勿自解析包内 patch 结构」口径不变）。
  **升级复核项**：dump 段落注释格式（含 patched by 后缀）随 dsh 升级须按
  ledger 复现点 13 逐条复核。
- 凭据：cargo test 182 绿（+3）/ clippy -D warnings 清 / fmt 过；前端
  typecheck / lint / vitest 112 绿（+3）；实机验证 = 备份 → 注入两条禁用补丁
  → dump 确认（archify-skill-filesystem 与 openviking-memory 行均得
  `disabled: true`、name/config 不丢）→ 还原 patch（diff 一致）。

### 2026-08-29 完成通知 · 4.3⑥ Profile 切换落地（重启语义）+ WSL guest 脚本参数化；多开登记待办 —— guan（AI 协作）

- 变更：本 commit——`executor.rs`（Executor trait 新增 `set_forced_profile`；
  Local/WSL 双执行器 probe 内按档位消费，bundle 快照档忽略；`GUEST_BOOT` 常量
  改 `guest_boot_script(profile)` + `sh_quote` 单引号进参——profile 名可含
  空格/引号等元字符，反例测试 + bash 实跑回读；WSL `select_profile`/`active_profile`
  同步参数化，v1「写死 web」收口）、`lib.rs`（`switch_profile` / `get_active_profile`
  两条新 IPC；`forced_profile` 目标记录 + `launch_executor_after_probe` 注入，
  错误卡重试延续同目标；模式切换清空重走常规解析）、`profiles.rs`
  （`ProfileSummary.web_ui` 字段 = 启动入口可见性，本模块内判定）、
  `ipc.rs`/`capabilities`/前端（types/api/store/行内启动按钮/运行中徽标/切换
  确认弹窗/文案）。维护者实测反馈两处收口：① 运行中徽标**实时化**——管理器
  订阅 `bootStore.activeStep`（boot:step 经事件总线每窗口广播），切换开始
  徽标即灭、boot 完成即亮，不再等聚焦/手动刷新；② 新增 UI 按 frontend-design
  口径与页面既有语言对齐——启动升级为带字按钮（与页头「新建 Profile」同配方）、
  运行中徽标加脉动心跳点、「无界面」由徽标降级进 meta 行、切换弹窗描述中性化
  （切换非常规破坏性操作，不走删除那套警示红）。
- 影响：**语义裁定（ADR-0009 §4 第三次修订，已先行经维护者逐题确认）**——
  ① 切换 = 停当前 dsh 以目标 profile 重启（dsh 无运行时切换能力，重启是唯一
  语义）；② 仅 webUi 候选可切换，headless/无界面档不给入口；③ 切换**不写**
  defaultProfile（星标是唯一写入口）；④ 失败错误卡 + 重试同目标，不自动回滚；
  ⑤ **WSL 同轮覆盖**：guest 脚本已参数化，但 WSL 真机验证无法在 macOS 执行，
  **待 guan 在 Windows 侧按 `docs/executor.md` 清单人工过一遍**（含切换到
  非 web 档 + 引号/空格 profile 名）。**多开（多 profile 并行多窗口）登记
  roadmap 待办未排期**：机制面可行（`--port 0` 官方旗标），前置 = 双实例
  并发写 `~/.dsh/storages` 竞态 spike + 1:1 生命周期约束的 ADR 修订。
- 凭据：cargo test 131 绿（+5：sh_quote 反例/回读、guest 脚本参数化、
  强制目标档位守卫、切换目标校验）/ fmt / clippy 全过 + `cargo check
  --target x86_64-pc-windows-gnu` 过（WSL 块编译目标语义）；前端
  typecheck / lint / test 40 绿。行为变更纯增，既有命令语义未动。

### 2026-08-28 完成通知 · Profile 创建二次修订：install 后壳补写 Web 工作台声明（创建即 webUi 候选）—— guan（AI 协作）

- 变更：本 commit——`profiles.rs`（`declare_webui_bundle` + 纯函数
  `append_bundle_declaration`：非模板名 install 成功后向
  `dsh.profile.bundles` 幂等追加 `@deepseek-ai/dsh-web-app`；声明补写失败
  降级 pending 态可重试；classify 增 webui_error 分支；模板名跳过——dsh
  拥有模板元组）；`lib.rs` doc；前端创建文案（基础 + Web 工作台/可设为默认）；
  `ADR-0009` §4 第二次修订注；ledger 复现点 7 + 追记；roadmap 4.3② 与事实
  边界行；**AGENTS §6 不变量行扩展（宪法改动，本条即知会 + diff 摘要）**：
  三件套写入例外 #2 = 「非模板名创建成功后的 web-app 声明单键追加」。
- 动机：defaultProfile 消费只认 webUi 候选（bundles 含 web-app，`resolve.rs`
  `list_web_ui_profiles`）——纯 dsh-base 原始版创建出来即无法设为默认启动
  （无 URL 可导航，boot 静默回退 web），用户实机踩坑（设「11」为默认、重启
  仍进 web）。
- 红线边界：三件套写入例外 #2，同类先例 = name 一致化改写。依据 dsh 源码
  `normalizeShippedProfile`（app-boot index.js @ 472，2026-08-28 读）：
  「Any other list is user-owned」——模板精确元组之外的 bundles 列表本就归
  用户/工具所有；目标状态与出厂 web 模板同构（web-app 不进 dependencies、
  `resolveBundleDir` 双锚点零下载），即 web profile 日常运行态。否决替代：
  `dsh plugin add @…@版本`（真实依赖 + 网络安装 + 版本锚定难题，偏离「初始
  web 标准」）。旧版创建的纯 dsh-base profile **不追溯**（重试创建才按新
  标准补齐）。
- 凭据：cargo test 126 绿（install_outcome 产物语义测试改写为
  create_declares_webui_bundle_like_web_template，classify 增⑥声明失败分支）/
  fmt / clippy 全过；前端 typecheck / lint / test 40 绿；实机目检见当日验证记录。

### 2026-08-28 完成通知 · 对话框 footer 静默化：去库存灰底，收编为有意设计 —— guan（AI 协作）

- 变更：本 commit——`ui/dialog.tsx` `DialogFooter` 去掉脚手架模板自带的
  `bg-muted/50` 灰底 + `border-t` 出血分隔带（2026-08-27 `a9c1656` 库存样式，
  未经审视），改为无底无线的静默 footer：按钮右缘与正文对齐，留白交给
  `DialogContent` 自身节奏。四个 profile 对话框（详情/创建/重命名/删除）
  共用组件，一致生效，无逐个覆盖。
- 依据：对话框正文为白底 + 发丝线卡片语言，灰带是全对话框唯一的填充色块，
  且详情对话框 footer 分隔线与 patch 卡片下边框相距 16px 贴出双线噪音；
  按「意图优先于强度」（refined minimalism = restraint）收编为有意设计。
- 凭据：前端 typecheck / lint / test 40 绿；tauri dev 实机目检（详情对话框
  用户协同截图确认：灰带与分隔线移除、按钮对齐、内容区无回归）。

### 2026-08-28 补记 · 详情对话框上一刀引入 grid 撑破回归，已修 —— guan（AI 协作）

- 变更：本 commit——`ProfileDetailDialog.tsx` 内容包装 div 补 `min-w-0`。
- 原委：上一刀把 patch 原文改 `whitespace-pre` 后，`<pre>` 最小内容宽度 =
  最长一行；`ui/dialog.tsx` 的 `DialogContent` 是 **grid** 布局，内容包装 div
  作为 grid 项未设 `min-w-0`，自动最小尺寸被 pre 撑破 → 整条轨道比对话框宽，
  依赖卡片与 footer 一起越界（用户实机截图复现）。`min-w-0` 归零该项对轨道
  尺寸的贡献后，pre 收敛回对话框宽度、由自身 `overflow-auto` 横向滚动。
- 凭据：前端 typecheck / lint / test 40 绿；**tauri dev 实机目检通过**
  （vite HMR 后 AX 点开「web」详情截图核对：依赖三行完整、file: spec 省略号
  截断、原文不折行、footer 归位）。

### 2026-08-28 完成通知 · Profile 详情对话框修复：file: 依赖行溢出 + patch 原文折行失真 —— guan（AI 协作）

- 变更：本 commit——`frontend/src/components/profiles/ProfileDetailDialog.tsx`
  两处：① 依赖行 `file:` 超长 spec 溢出卡片边框、包名被挤压至零宽不可见
  （spec `shrink-0` + 包名 `truncate` 的 flex 收缩方向写反）——改为包名
  `shrink-0` 恒可见、spec `truncate` + `title` 悬停全文兜底；② patch 原文
  `whitespace-pre-wrap` 折行续行顶格、与真实行混淆破坏「原文」语义——改
  `whitespace-pre` + 既有 `overflow-auto` 横向滚动，逐字保真。
- 影响：仅详情对话框展示层，无 IPC / 契约 / 数据改动；超长 spec 悬停可见
  全文。遗留（评审发现、未在本刀范围）：空插件组合复用「无额外依赖」文案
  的措辞错位、依赖版本号 `text-faint` 对比度偏低、对话框无高度约束的矮窗口
  健壮性——待后续小刀。
- 凭据：前端 typecheck / lint / test 40 绿（纯样式改动，Vitest 纯逻辑测试
  不涉及）；diff 已人肉复核。

### 2026-08-28 完成通知 · Profile 创建路径修订：add @deepseek-ai/dsh-base → install（原始版语义）—— guan（AI 协作）

- 变更：本 commit——`src-tauri/src/profiles.rs`（`create_command_args` 改
  `["plugin","--profile",<名>,"install"]`，删除 `CREATE_ADD_BUNDLE`；结果分类
  文案与注释同步「原始版语义」；测试：install 参数 + 原产物语义 + 成功路径
  `Already up to date`）；`lib.rs`（create_profile 文档注释）；`frontend`
  （`content/zh-CN.ts` 创建文案：busy 秒级 / done 内置声明就绪 / hint 原始版；
  `ProfileCreateDialog.tsx` 注释；`types/ipc.ts` 注释）；`docs/`（ADR-0009 §4
  执行细则修订注 + §5 正面后果 + 验证项；ledger 复现点 7 与复核记录；
  roadmap 4.3 ② 与事实边界行）。
- 影响：**ADR-0009 方案 A 执行细则修订（非换方案）**——创建命令由
  `add @deepseek-ai/dsh-base` 改为 `install`：创建语义 = 原始版 profile
  （initProfile 写三件套，bundles 含内置插件随 dsh 安装目录解析；空依赖
  `pnpm install` → `Already up to date`，零网络毫秒级）。触发：2026-08-28
  本机创建 `test` profile 慢至 2 分钟失败/超时，查因 = `add` 裸包名按
  dist-tag `latest` 解析到 dsh-base 0.0.1-rc.1（已弃用旧版，依赖 37+ 个已从
  registry 删除的旧包名：dsh-bash-env / dsh-tasks-local / dsh-skill-local…，
  npmmirror/npmjs 均 404）→ pnpm 递增重试（10s/60s × 37 包）卡死；npmmirror
  对缺失 scoped 包回退到死域名 `r.cnpmjs.org`（本机解析到保留段 198.18.0.192）
  放大表象。`install` 不解析 dist-tag，版本语义免疫。仅周知：pnpm 缺失 →
  补齐 → 失败降级（ADR 口径 2）与「已创建未装插件」中间态重试语义不变；
  后续加外挂插件走同一条 `dsh plugin add` 链（4.4）。
- 凭据：`cargo test` 126 绿；`gate_tests` 三处同步一致
  性绿；`cargo fmt --check` / `clippy -D warnings` 全过；前端 typecheck /
  lint / test 40 绿；实机（macOS，DSH_HOME=临时目录零污染）
  `dsh plugin --profile <新名> install` → init 先行 / `Already up to date`
  186ms 零网络 / 产物 `dependencies:{}` + bundles 仅 dsh-base（含
  pnpm-lock.yaml）/ 后 `--dump-config` 组合启动正常（退出码 0）；diff 已
  逐行人肉复核。

### 2026-08-28 完成通知 · 工具窗口布局修正：内容顶部锚定替代垂直居中 —— guan（AI 协作）

- 变更：commit `4bed1c9`（本 commit 落档本条）——`PageShell` 新增 `align`
  两态：常驻工具窗口（关于 / Profile 管理器）`top` 锚定 + `py-10` 上下
  呼吸位；`BootMode` 等主窗口启动抉择屏维持默认 `center`。
- 影响：仅周知。根因：旧静态壳窗口尺寸贴内容，`items-center` 居中不可见；
  前端迁移后窗口可调大小（profiles 680×700），内容悬浮正中、且内容超高时
  顶部被裁切（flex 居中 + overflow 的经典缺陷），顶部锚定一并消除。
  纯布局改动，IPC / 契约 / Rust 零变化。
- 凭据：`npm run typecheck` / `lint` / `test`（40）全绿；diff −4/+13 已
  人肉复核；布局观感按 4.4 口径归人工目验（Vitest 纯逻辑无 DOM 断言）。

### 2026-08-28 排障记录 · debug 构建白屏——dev 工作流文档修正 —— guan（AI 协作）

- 变更：本 commit——`README.md` 开发段与 `docs/CONTRIBUTING.md` ④ 的运行
  指引改为 `cargo tauri dev`（或 vite + cargo run 两终端），并明示「直接
  cargo run 而 vite 未起 = 壳窗口白屏」。
- 影响：仅周知。**现象与根因**：用户以 `cd src-tauri && cargo run` 启动后
  Profile 管理器 / 关于窗口全白。根因是 debug 构建的前端从 `devUrl`
  （localhost:1420）加载（tauri.conf.json，前端迁移时引入），vite 未运行
  → 资源加载失败；主窗口"正常"是假象——boot 完就导航进 dsh 工作台，把
  白屏的壳 SPA 盖掉了（shell.log 佐证：dsh 就绪 + 导航均正常，1420 端口
  无监听）。此为前端迁移后的既定工作流（`beforeDevCommand` 已配好），
  非管理器代码缺陷；上一条文档修正 commit 写反了 npm run dev 的用途，
  本次一并纠正。
- 凭据：`lsof -i :1420` 无监听 + `~/Library/Application Support/
  io.github.realguan.dsh-dock/shell.log` 显示 boot 全链正常；复现路径
  与修复命令均实机核对（tauri-cli 2.11.4 在机）。

### 2026-08-28 文档修正 · README/CONTRIBUTING 面向前端迁移与 Profile 管理器现状更新 —— guan（AI 协作）

- 变更：本 commit——`README.md`（「它做什么」补 Profile 管理器与 pnpm 环境
  保障两条、WSL 条目 settings.json 字段表述更正、开发命令全面修正：前端
  React SPA 的 npm 闸门 + **`cargo run` 须先 `cd src-tauri`**、品牌资源路径
  ui/assets → frontend/public 与 Emblem、结构树重画含 profiles.rs/ipc.rs 等
  新模块）；`docs/CONTRIBUTING.md`（「壳前端免构建（ui/ 静态页）」过时表述
  更正为 React SPA + npm 三闸门）。
- 影响：仅周知。根因是 2026-08-27 前端迁移与 4.3 各刀落地后文档未跟上——
  本机实测 `cargo run` 在仓库根直接失败（Cargo.toml 在 src-tauri/ 下），
  README 旧命令会误导所有新上手者。
- 凭据：文档改动；所有引用路径已 `ls` 核实存在（frontend/public/mark.svg、
  assets/dsh-logo.svg、scripts/*、node-map/README.md 等）。

### 2026-08-28 完成通知 · pnpm 补齐落地（boot 硬依赖 + 创建时复用，ADR 红线 2 收口）—— guan（AI 协作）

- 变更：本 commit——`updates.rs` 新增 `ensure_pnpm`（PATH 可见即返回；
  缺失经 `npm install -g pnpm` 同步补齐，复用 ADR-0005 npm 全局链 +
  ADR-0006 镜像序；补齐后必须在同一 PATH 重新可见，否则按失败处理）+
  `install_pnpm_via_npm`（双镜像逐试、聚合报错；pnpm 纯 JS 不传
  allow-scripts）+ 3 条测试（命中不装 / 装后可见性强制 / 失败聚合文案，
  假 node/npm-cli fixture）；`executor.rs` LocalExecutor::probe 接线
  （boot 期检查 → 失败阻断 boot 出可行动错误卡；基准 = 注入 dsh 的
  PATH）；`profiles.rs` 创建时防御检测升级为「缺失 → 补齐 → 再失败才报
  错」（复用同一函数）；`find_pnpm` 回归私有（外部消费点已被
  ensure_pnpm 收编）。
- 影响：仅周知。ADR-0009 红线 2 口径 2 至此完整：boot 环境检查保证
  「dsh 全部子命令可用」，补齐失败 = 新增 boot 失败模式（可行动文案：
  检查网络 / npm 镜像 / 手动 `npm install -g pnpm`）。WSL 客体内
  node → pnpm → dsh 链仍归 4.9（ADR-0004 §7）。AGENTS §7 网络面登记
  无变化（boot 期 pnpm 补齐已于 2026-08-28 登记）。4.3 遗留仅剩
  Windows 转发链实机验证（Spike A 遗留）。
- 凭据：`cargo test` 125 绿（+4，含恢复一处被测试插入截断的既有测试
  node_download_urls_mirror_first）/ `fmt --check` / `clippy -D warnings`
  全过；diff 已逐行人肉复核。

### 2026-08-28 完成通知 · 4.3④ defaultProfile 消费接线 + WSL 放开评估收口 —— guan（AI 协作）

- 变更：本 commit——`resolve.rs` 新增纯函数 `consume_default_profile`
  （存储默认值 ∈ webUi 候选才消费，含正反例测试）；`executor.rs`
  LocalExecutor::probe 接线——命中即以该 profile 启动并跳过选择器，
  仅覆盖 dsh_home = 用户 home 的档位（system/download），bundle 快照
  世界不适用；未命中（headless 类无 webUi / 已被手工删除）回退常规
  流程并记日志。ADR-0009 §5 WSL GUEST_BOOT 放开评估收口：**本版不放开，
  维持 `--profile web`，归 4.9**（客体 home 与壳侧 home 不同世界 /
  非 webUi 无 URL 可导航 / 客体内 profile 管理属 4.9 范围）。
- 影响：仅周知。「设为默认启动」语义自此完整：设置 → 持久化 → 下次
  启动自动使用（多 webUi 不再出选择器）；选择器仍在（未设默认时），
  其「选择只影响本次会话」语义不变。验收路径：管理器设默认 → 重启
  应用 → 直接进入该工作台（日志可见 defaultProfile 命中行）。
- 凭据：`cargo test` 121 绿（+1）/ `fmt --check` / `clippy -D warnings`
  全过；diff 已逐行人肉复核。

### 2026-08-28 完成通知 · 4.3 Profile 管理器第五刀（前端管理页）+ AGENTS §4.4 重评 —— guan（AI 协作）

- 变更：本 commit——前端 `pages/ProfileManager.tsx` + `components/profiles/*`
  （列表行两态 / 详情 / 创建 / 复制 / 重命名 / 删除确认五组组件）+
  `stores/profilesStore.ts` + `lib/profiles.ts`（校验镜像等纯逻辑，6 条 Vitest）
  + `lib/tauri.ts`（8 个 profile api，共 20 命令全类型化）+ `types/ipc.ts`
  （五个响应类型锚定 Rust serde 形状）+ `content/zh-CN.ts`（profiles 文案段）
  + `App.tsx`（label=profiles 路由）；后端 `lib.rs`（`open_profiles_window`
  镜像 about 主线程约束 + macOS 菜单 / 非 macOS 托盘入口
  `profiles_manager`）+ `capabilities/default.json`（windows 数组加
  `profiles`——ACL 按窗授权，漏加即整页 IPC 静默拒绝）。
- 影响：**触宪法级**两处，仅周知——① AGENTS §4.4 Vitest 重评条件已触发并
  落盘结论：维持纯逻辑测试，RTL/jsdom 不引入（再评触发 = 需 DOM 断言的
  复杂交互）；② TanStack Query 未接入：管理页是「读一次 + 变更后手动刷新」
  形态，frontend-migration §11 的触发条件裁定延后（rationale 见
  profilesStore.ts 头注释，出现跨窗口订阅诉求再立 micro-ADR）。
  范围声明：4.3 全部六项能力的 UI 至此可用（菜单/托盘 → Profile 管理器）；
  defaultProfile 的 boot 消费接线、pnpm 补齐仍归后续刀。前端文案含
  删除确认三要素（不级联全局数据 / 其他 dsh 实例 / 模板名重新物化，ADR
  §2 要求）与创建 pending 中间态（ADR §3 方案 A 契约）。
- 凭据：`npm run typecheck` / `lint` / `test`（40，+6）/ `build` 全绿；
  `cargo test` 120 绿 / `fmt --check` / `clippy -D warnings` 全过；
  diff 已逐行人肉复核。

### 2026-08-28 完成通知 · 4.3 Profile 管理器第四刀（生命周期 + 默认持久化）—— guan（AI 协作）

- 变更：本 commit——`profiles.rs`（复制/重命名/删除文件层 + 前置校验 + 运行中
  防护文案 + patch `../` 引用扫描警告 + 8 条测试）；`settings.rs`
  （`defaultProfile` 字段，第二最小面例外）；`executor.rs`（`active_profile`
  trait 方法：运行中防护比对源，本地取 launch、WSL 固定 web）；`lib.rs`
  （5 个新 IPC 命令；**连带修复** `switch_mode`/`choose_mode` 改
  load-modify-save——原整体覆盖写法会抹掉 defaultProfile）；三处同步
  （ipc.rs/capabilities）。引用面全按 Spike B §3 执行：复制排除 node_modules
  + name 改写；重命名删 node_modules 让 dsh 自愈；删除不级联 sessions；
  defaultProfile 删除时清除、重命名时同步，失效读取侧兜底 web。
- 影响：**触宪法级**——AGENTS §6 例外册登记 `defaultProfile` 落地、§7 IPC
  登记表新增 5 命令（`copy_profile`/`rename_profile`/`delete_profile`/
  `set_default_profile`/`get_default_profile`），仅周知。范围声明：管理器
  后端能力至此齐备（列出/详情/创建/复制/重命名/删除/切换默认）；前端管理页、
  defaultProfile 的 boot 消费接线（含 WSL GUEST_BOOT 放开多 profile 评估）、
  pnpm 补齐均归后续刀。
- 凭据：`cargo test` 120 绿（112 + 新增 8：复制排除与改写 / 重命名自愈 /
  删除不级联 / 运行中防护文案要素 / 默认候选校验 / settings 旧格式兼容等）；
  `gate_tests` 三处同步一致性绿；`cargo fmt --check` / `clippy -D warnings`
  全过；diff 已逐行人肉复核（经维护者裁定 ①+② 范围：生命周期 + 默认持久化
  一刀完成，pnpm 补齐与前端拆分后续刀）。

### 2026-08-28 完成通知 · 4.3 Profile 管理器第三刀（创建能力）—— guan（AI 协作）

- 变更：本 commit——`src-tauri/src/profiles.rs`（创建段：转发链 spawn 封装 +
  前置校验 + 结果分类 + 4 条纯函数测试）；`updates.rs`（`find_pnpm` 转
  pub(crate) 共用）；`ipc.rs` / `lib.rs` / `capabilities/default.json`（IPC 三处
  同步，`create_profile` 为**首个异步命令**：探测 + 转发链全在 spawn_blocking，
  避免同步命令冻结主线程）；`AGENTS.md` §7；ADR-0009 §5；ledger 复现点 7。
  能力：spawn `dsh plugin --profile <名> add @deepseek-ai/dsh-base` 半官方路径
  创建 profile（三件套由 dsh initProfile 写出，壳零写入）；重名拒绝 + 半初始化
  放行重试（重跑 add 幂等，ADR §4）；pnpm 防御检测缺失即拒 spawn（可行动文案，
  补齐归后续刀）。
- 影响：**触宪法级**——AGENTS §7 IPC 命令登记表新增 `create_profile`，仅周知。
  范围声明：定位仅系统探测（离线档/未装系统 dsh 用户暂不可创建，报可行动错误）；
  复制/重命名/删除/默认持久化/WSL 客体内 profile/pnpm 补齐均归后续刀。
  实机验证（macOS，DSH_HOME=临时目录零污染）：init 先行 / pnpm 经注入 PATH 可
  定位 / 顺带实测 pnpm 网络失败模式（镜像 ECONNRESET -> 已创建未装中间态
  exit 1，分类文案与单测 fixture 逐字吻合）；成功路径 reconcile 沿用 Spike A
  §3.2 同机同版本结论；Windows 转发链（shell: win32 分支）仍为遗留。
- 凭据：`cargo test` 112 绿（108 + 新增 4）；`gate_tests` 三处同步一致性绿；
  `cargo fmt --check` / `clippy -D warnings` 全过；diff 已逐行人肉复核。

### 2026-08-28 完成通知 · 4.3 Profile 管理器第二刀（只读能力）—— guan（AI 协作）

- 变更：本 commit——新增 `src-tauri/src/profiles.rs`；`ipc.rs` / `lib.rs` /
  `capabilities/default.json`（IPC 三处同步）；`AGENTS.md` §7；`docs/adr/0009`
  §5 勾选；`docs/contracts/dsh-behavior-ledger.md`。能力：profile 非法名校验
  （与 dsh `resolveProfileDir` @ 318 逐字一致）、profiles 扫描器（已物化 +
  未物化内置模板名两态合并；排除 `profiles/node_modules` 符号链接农场）、单
  profile 详情（package.json 关键字段 + `cordis.patch.yml` 原文，YAML 不解析——
  serde_yaml 已弃维，依赖选型推迟到启停插件刀）；新 IPC 命令
  `list_profiles` / `get_profile_detail`。纯读：零写入、零 dsh 子进程、零网络。
- 影响：**触宪法级**——AGENTS §7 IPC 命令登记表新增两条（新命令流程规定动作），
  仅周知。范围声明：管理器仅覆盖壳侧本地 home（`user_dsh_home()`）；WSL 客体内
  profile、创建/复制/重命名/删除、`--dump-config` 详情、pnpm 补齐均归后续刀。
  ledger 复现点 6/8 已按行号勘误口径（318/323/353，弃 11826/13418）入册。
- 凭据：`cargo test` 108 绿（基线 98 + 新增 10：校验正反例 / 扫描两态与农场
  排除 / 详情路径遍历拒绝等）；`gate_tests` 验证三处同步一致性；
  `cargo fmt --check` / `clippy -D warnings` 全过；diff 已逐行人肉复核。

### 2026-08-28 宪法级改动 · AGENTS.md 减法：248 → 178 行，删微观管理留边界 —— guan

- 变更：`AGENTS.md` 全文重写（commit `876dbcb` 之后）——删除三类内容：① 通用工程
  常识（跨平台 #[cfg] 分叉、阻塞主线程禁令、组件文件命名、Zustand 选择器细则、
  mock 策略细节等——AI 工程常识默认做对，写清单反而暗示「除此以外随便」）；
  ② 可推导明细（模块职责逐个注释、依赖版本行、构建发布行、§7 专项裁定长文——
  §9 索引已有等价一行结论，消双源）；③ 过度规定（错误页样式、stdout 日志技法——
  归还代码注释与 ADR）。保留全部边界与真坑：两红线（含台账指针）、dsh 文件系统
  不变量、唯一网络面、IPC 三处同步、事件总线模块加载期竞态坑、child_cmd、
  tauri-cli 同代、zip pinned、必测四场景、AI 交互七条、持久化例外册、§11 元规则。
- 影响：**宪法级**——规范哲学转向「只写边界 + 真坑 + 为什么，不写怎么做」，给
  AI 留发挥空间；被删内容均有归宿（代码 / ADR / roadmap / 全局 AI 配置）或属
  可推导。此前批准的全部提交于 `876dbcb`，本笔减法独立可回溯。
- 凭据：纯文档改动，不触运行时；全文 178 行 ≤ 250 预算、单节最大 31 行（§4）；
  真坑与边界关键词 grep 抽查全数在册；修改未提交，待确认后提交。

### 2026-08-28 宪法级改动 · 规则方向性审核落地：复现台账 + 白名单闭环 + 规则触发器 —— guan

- 变更：① 新建 `docs/contracts/dsh-behavior-ledger.md`——dsh 行为复现台账（已落地
  5 项 / 计划 3 项，基线 v0.1.1-rc.2），堵「复现点随 dsh 升级静默漂移、CI 无法发现」
  缺口，dsh 升级 = 逐条复核触发器；`AGENTS.md` §0 红线 1 挂台账指针。② §4.4 白名单
  治理闭环——「未来数据层」改「数据获取层（取数·缓存·同步）」判据，新增依赖 =
  先回写清单（唯一权威）再广播。③ 两条规则补再评估触发器——RTL/jsdom 禁令挂
  「Profile 管理器 UI 落地时重评」、无 dev-dependencies 加「确需专用依赖先 ADR」口子。
  ④ roadmap Next 加工程前置——IPC 三处同步自动化自检 spike（4.3 开工前把人肉纪律
  升级为机器闸门）。
- 影响：**宪法级**（AGENTS.md）+ 新增台账文档。dsh 升级流程多一步「复核台账」；
  前端新增依赖流程变「回写清单 + 广播」；4.3 开工前多一个 spike 闸门。纯文档，
  不触运行时。
- 凭据：复现点逐条 grep 锚定代码实际位置（shell.rs / resolve.rs / updates.rs /
  executor.rs）；AGENTS 全文 248 行 ≤ 250 预算、§4 恰 40 行；修改未提交。

### 2026-08-28 宪法级改动 · 跨文档定位统一 + AGENTS 一致性修正 —— guan

- 变更：三份文档定位统一为「dsh 的桌面管理面板」（2026-08-27 重定义的收尾）——
  roadmap §1 开头「桌面终端 / 极小的壳」改写、README 开头定位段改写（补管理能力句）、
  contract.md 宿主解析节加历史定位注（ADR-0005 语义不变）；AGENTS.md 五处一致性
  修正——§7 例外册补登记 `app:update` 事件（updater 回推，仅 main/about）、§2 docs
  清单补 frontend-migration 与 spikes/、§5 测试命令收敛为 §1 指针（消双源）、§0
  工程准则补 §6 指针、§6 持久化例外改「例外册登记制」（与 roadmap 硬约束 4 对齐，
  第二例外落地前先登记字段名）。
- 影响：**宪法级**（AGENTS.md + contract.md）。定位口径此后以 AGENTS §0 为唯一
  权威；settings.json 第二字段落地前须先在 §6 登记。纯文档，不触运行时。
- 凭据：grep 全仓「桌面终端」仅剩 ADR 历史引用（刻意保留）；本笔与上一条（制度
  建设）同批未提交，待确认后按 CONTRIBUTING 提交。

### 2026-08-28 宪法级改动 · AGENTS.md 首轮回收：394 → 244 行，回归 §11 预算 —— guan

- 变更：`AGENTS.md` 全文按 §11 回收——§2 目录树（58→17 行）：`ls` 可推导的文件级
  明细删除，只留职责与陷阱（勿动 / 勿手改 / 永不入库）；§7 例外册（50→19）：IPC 命令
  收敛为登记行，专项裁定收敛为「一行一裁定 + ADR 编号」，推理细节以 §9 索引 + ADR
  为唯一源（消除双源）；§1 技术栈（35→18）：依赖版本明细还 Cargo.toml / package.json，
  只留坑与裁定锚点；§4 代码规范（69→40）：正反例去重合并；§5 测试明细（25→13）还
  roadmap；§0 / §8 / §9 / §10 措辞收紧。只删可推导明细与已有归宿的长文，裁定结论全保留。
- 影响：**宪法级**——纯回收、无新增裁定；所有删除项均有既有归宿（ADR-0001~0009 /
  roadmap 4.1/4.2 / 代码现状）。协作者若发现某被删细节在 ADR / roadmap 也找不到，
  频道提出即可，git 历史可回溯旧版全文。本笔与上一条（§11 元规则）同批未提交，
  建议合为一笔提交（hunk 交叉无法拆分）。
- 凭据：纯文档改动，不触运行时；全文 244 行 ≤ 250 预算、单节最大 40 行（§4），
  diff 394→244（−150）。另发现 README.md「结构」节仍是 React 迁移前旧版（ui/ 静态页），
  属已知失真，另行开工修正，本笔不动。

### 2026-08-28 宪法级改动 · AGENTS.md 新增 §11 写入边界（元规则）—— guan

- 变更：`AGENTS.md`——① 文件头挂一行指针（「最小必要集，不是知识库」）；② 新增
  §11 元规则：准入判据四条（高频 / 违约即事故 / 不可推导 / 无家可归，核心判据 =
  「没有这条 AI 会做错吗」）、排除清单七类（模块技法→注释、决策推理→ADR、流程
  细则→CONTRIBUTING、契约细节→contract.md、计划指标→roadmap、操作手册→docs
  专项、通知→broadcasts）、形态规则（结论一句 + 日期 + 指针；升格 ADR 后原条目
  必须回收，禁双源）、预算与回收（全文 ≤ 250 行 / 单节 ≤ 40 行）。
- 影响：**宪法级**——此后向 AGENTS.md 写入任何条目须先过 §11.1 判据，不合者
  review 可依据本节驳回；知会落档、broadcasts 登记范围不变。存量超支（~354 行，
  §2 目录树 / §7 例外册为主）按新规则另行开工一笔回收。
- 凭据：纯文档改动，不触运行时；本笔仅新增元规则、不回收存量（一次一意图）；
  修改未提交，待确认后按 CONTRIBUTING 提交。

### 2026-08-27 宪法级改动 · 项目边界重定义：dsh 桌面管理面板 + 两红线 —— guan

- 变更：`AGENTS.md`（§0 定位重写为「dsh 的桌面管理面板」/ §1 技术栈表数据库行 /
  §4.2 禁库改「需 ADR 评估」/ §6 存储与生命周期重写）、`docs/roadmap.md`（硬约束
  2/4 重写）、`docs/adr/0008`（壳保持薄加注「仅约束运行时」）、`docs/adr/0005`
  （转发链 vs 全局安装区分注）、`docs/adr/0006`（管理功能网络面不属唯一网络面注）、
  `docs/executor.md`（defaultMode 例外表述同步）、`docs/adr/0009`（§2 重写为
  3 红线 + 工程准则）。
- 影响：**宪法级**——① 项目定位从「通用产品壳」改为「dsh 的桌面管理面板」；
  ② 红线定为两条：不修改 dsh 源码 + 安装包不内置依赖（优先宿主检测、缺失时
  实时下载，含 pnpm 自动补齐扩展）；③ 「壳保持薄 / 无状态库」降为**仅约束运行时**，
  管理功能按优秀软件工程设计（可引入数据库 / 持久化）；④ 历史 ADR 保持原状仅加注。
  协作者注意：管理功能（4.3+）不再受「无状态库」限制，但 dsh 文件系统不
  变量（0600 凭据 / 三键 / 农场只读）继续必须维护。
- 凭据：纯文档改动，不触运行时；diff 已逐文件核对（AGENTS §0/§1/§4.2/§6 +
  roadmap 硬约束 + 4 个 ADR 加注 + executor.md 同步）。修改未提交，待确认后按
  CONTRIBUTING 提交。

### 2026-08-27 快车道直推 · 完成通知：Now 阶段收口（4.1 工程化基线 + 4.2 updater 测试）—— guan

- 变更：六连提交——`bd94596` fix(clippy) 全部 17 处警告清零（关键：
  shell.rs `floor_char_boundary` 击穿 rust-version=1.77.2 的 MSRV 承诺，
  CI 全用最新 stable 故未暴露）；`dd521d1` style 全仓 fmt 归一
  （9 文件 168+/147- 纯机械，与 clippy 修复分仓提交）+ 落地仅锁 edition 的
  `rustfmt.toml`；`9029ebe` chore(rust)；`10e0957` ci 三平台
  fmt --check / clippy -D warnings 闸门 + ubuntu coverage job
  （cargo-llvm-cov 出 lcov，先出数不定阈值）；`5417987` test(updater)
  六条纯函数测试；docs 提交（见下）。
- **宪法级改动（本次知会）**：① AGENTS §1 Rust 行——移除 `rust-version`
  基线，**Rust 工具链跟随最新 stable**（2026-08-27 维护者裁定：不设 MSRV；
  上限纪律不变：CI @stable 自动跟新）；② AGENTS §1 Lint/Format 段改写为
  已建基线状态；③ AGENTS §5 updater 待补条目改为已覆盖表述；
  ④ AGENTS **新增 §8 第 7 条「驳回不合理的规则」**——AI 判定规范与现实
  冲突/自相矛盾/失效时应停手提请驳回与修订（举证义务在提请方），
  不得以变形实现绕行；顺从≠忠诚，变形合规比违规更危险。
- 影响：CI 首跑新闸门有红的风险已用本地预演对冲（本机 1.98 三道全绿）；
  Next 阶段（4.3 Profile 管理器）进入条件满足，开工前先做两个前置 spike。
- 凭据：本地 rustc 1.98.0 下 fmt --check / clippy --all-targets -D warnings /
  cargo test 95 绿；fmt 与 clippy 两类 diff 分仓提交均经人肉复核。

### 2026-08-27 发版事项 · v0.5.1 三平台验收通过，冻结期解除 —— guan

- 确认：tag run `33049383090` 三平台 job 与 release job 全部 success；Release
  资产 14 个齐全（dmg/exe/msi/AppImage/deb/rpm + 签名 + latest.json）；
  下载 macOS `.app.tar.gz` 实拆——`Info.plist` 版本 0.5.1，主二进制内嵌前端
  bundle 五个事件名（app:update / boot:step / boot:progress / boot:update /
  boot:error）grep 全中（本次缺陷的产物级判据）。上游 dsh 会话工作正常。
- 影响：**v0.5.1 发布完成，冻结期自此解除**——master 恢复正常合流
  （改动仍按 CONTRIBUTING 占用声明纪律）；下一步按路线图 Now 阶段推进。
  已装 0.5.0 的环境因自更新面板同受缺陷影响，需手动换装 0.5.1。
- 凭据：`gh api .../runs/33049383090/jobs` 全 success；本条即对上两条
  「处置见下一条广播」预告的闭环。

### 2026-08-27 发版事项 · v0.5.0 缺陷确认 → 重切 v0.5.1 热修 —— guan

- 变更：fix `f3cef30`（事件总线 import 锚点，见下条补记）+ updater 观测日志
  （run_check 入口与 set_state 每次推进记 tracing）+ 全仓版本升 `0.5.1`
  （tauri.conf.json / Cargo.toml / Cargo.lock / frontend package.json+lock），
  tag `v0.5.1` 当日推送。
- 裁定（为何不沿用上次 force 迁移 tag 的做法）：v0.5.0 三平台产物**已发布且含
  缺陷**——事件监听缺失不止影响关于页：启动时间线 / 错误卡 / 下载进度同链路
  全部不刷新，冷启动表现为启动页冻结后硬跳工作台、失败时错误卡不渲染。
  上次 force 迁移的前提是「原 run 无任何产物」；本次 Release 已存在、可能已有
  下载，force 迁移会留下同名异物的资产，违反可追溯原则。按语义化版本重切
  v0.5.1。注意：**0.5.0 客户端的自更新面板恰好也受此缺陷影响**（自动检查在
  Rust 侧正常执行，但 UI 不回显），已装用户需手动换装 0.5.1。
- 影响：冻结期继续（master 只收 fix）；CI 三平台验收通过前不宣布发布完成。
- 凭据：cargo test 89 绿 + 前端四道门禁绿；实机 AX 观察到修复后徽章
  「检测中 → 最新」翻转（自动首查全链路），手动点击路径同链路 +
  新增日志可事后定位；版本 diff 六文件已人肉复核。

### 2026-08-27 补记 · v0.5.0 实机缺陷：事件总线未进 bundle（关于页检查更新无反应） —— guan

- 变更：fix 待提交——`frontend/src/main.tsx` 增加 `import "./lib/events"` 副作用
  锚点；`docs/frontend-migration.md` §9 新增「事件总线」产物级回归清单条目。
- 根因：`lib/events.ts` 靠模块加载期 `initEventBus()` 自装配（宪法 §4.3 裁定），
  但全仓没有任何运行时 import 它——Vite 树摇将其整体排除出 bundle（v0.5.0
  dist 中 `app:update` 出现 0 次，实锤）。所有窗口的 boot:*/app:update 监听均未
  注册；关于页显示的「已是最新」全部来自进入时的播种 invoke，恰好掩盖断链。
  纯逻辑单测（34 例全绿）测不出「装配丢失」这类集成缺失。
- 影响：**v0.5.0 三平台产物若已出包则携带此 bug**——关于页更新状态机不再实时
  推进、启动时间线/错误卡/下载进度不刷新。处置与是否重打 tag 见下一条广播。
- 凭据：前端四道门禁绿 + 新产物 grep 五事件名各 ≥1；修后复验结论随附。

### 2026-08-27 补记 · v0.5.0 发布中断修复（Ubuntu CI 失败 → 重发） —— guan

- 变更：`915657f`（冻结期 fix）—— beforeBuildCommand 钩子 cwd 显式化
  （tauri.conf.json ScriptWithOptions `cwd="../frontend"`），build.yml/AGENTS
  注释同步；`v0.5.0` tag 已 force 迁移指向该修复（原 tag run 全失败、无任何
  产物/Release，无污染可追溯亏损）。
- 根因（CI 实证两连修）：① tauri-cli「自动发现含 package.json 目录」深度遍历
  在 Linux ext4 目录序下可能先命中 `node-map/` → npm ci 找不到 lockfile（本地
  APFS 碰巧命中 frontend/，阶段 A 的验证结论被事实击穿）；② 显式 cwd 相对基准
  实为 **src-tauri**（build.rs `set_current_dir(dirs.tauri)`），首修用的
  `frontend/` 本地复现 No such file 后改为 `../frontend` 复测通过。
- 影响：仅发布链路，不触运行时行为——macOS/Windows 两 job 原 run 继续走完
  但其构建内容同构（hook 修复对三平台同效），三平台产物仍以重发 run 为准。
  经验已落档：**tauri 钩子 cwd 永远相对 src-tauri 且必须显式**，勿复信自动发现。

### 2026-08-27 发版事项 · v0.5.0 发布开始 —— guan

- 变更：`chore: 版本 0.5.0（…整批提交）`——tauri.conf.json / Cargo.toml /
  Cargo.lock / frontend package.json+(lock) 同步升版；roadmap 适用版本标 v0.5.0。
  tag `v0.5.0` 由本档案登记当日推送，CI 三平台矩阵 + Release 聚合
  （notes 由 GitHub 自动生成）。
- 影响：**冻结期开始**（CONTRIBUTING §8）——Release notes 发出至三平台产物
  验收通过期间，master 只收 fix 不收 feat。本版内容：前端自静态 HTML 全量迁移
  Vite+React+TS+Tailwind v4+shadcn/ui（ADR-0008 全流程，span commit
  aab68c1→本次），壳行为与 12 IPC 命令零变更；Move/关于/启动/选择器四页
  组件化；宪法同步修订（AGENTS §1/§2/§4.2/§4.3/§4.4/§5/§7）。
- 凭据：frontend gate 34/34；cargo test 89 passed；本机 release 构建三产物齐；
  已知遗留——Windows/Mode 页实机走查未做（广播 2026-08-27 阶段 C 条目）；
  fmt/clippy 基线待专项（阶段 E 条目）。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 E 落地（d003905 / 1868411） —— guan

- 变更：**宪法级（1868411）**——AGENTS §1/§2/§3/§4.2/§4.3/新增 §4.4/§5/§7、
  docs/roadmap.md（硬约束 2 与不做清单）、docs/CONTRIBUTING.md（路径行）、
  .github/workflows/build.yml（Frontend gates 步骤）；**非宪法（d003905）**——
  Vitest 34 用例（format/bootProgress/bootStep/updatePhase）、`ui/` 目录删除
  （dsh-logo.svg 迁至仓库根 `assets/`）、全仓悬空引用清理。
- 影响：① **宪法已生效**——「禁止引入任何前端构建链」修订为「前端框架仅限
  React 生态（§1/§4.2/§4.4 白名单）」；② 开发者须知——本地构建/调试请从
  **仓库根**调用 `cargo tauri dev/build`（钩子 cwd 发现逻辑），前端开发需
  node ≥20（`cd frontend && npm ci`）；③ **fmt/clippy 基线评估结论**：存量
  35 文件未归一 + clippy 9 警告，需专项 chore 落地（遵守「不引入全仓格式化
  diff」红线），本轮 CI 只接前端四道闸门，roadmap §4.1 [待补充] 保持；④ 迁移
  完成发布契——master 自此无 `ui/`，release 产物壳页面全 React。
- 凭据：frontend typecheck/lint/vitest 全绿（34/34）；`cargo test` 89 passed；
  `cargo tauri build --no-sign` 出齐三产物；diff 逐行人肉复核（lib.rs 仅两处
  注释；fmt 越界改动已回退——本次 session 自身纪律记录）。

### 2026-08-27 占用声明 · 前端迁移阶段 E 开工（宪法级变更预告） —— guan

- 变更：占用 `AGENTS.md`（§1/§2/§3/§4.2/§4.3+新增§4.4/§5/§7）、
  `docs/roadmap.md`（硬约束 2 与不做清单）、`.github/workflows/build.yml`
  （node 质量闸门 + fmt/clippy 评估）、`ui/`（删除）、`frontend/`（Vitest 测试）。
  依据：ADR-0008 行动项 + docs/frontend-migration.md §6/§7/§10 阶段 E 清单。
- 影响：**宪法级预告**——AGENTS §4.2「禁止引入任何前端构建链」将修订为
  「前端框架仅限 React 生态（Vite+React+TS+Tailwind+shadcn/ui）」，§4.3 全面
  重写为 React 组件规范，新增 §4.4 三条红线（依赖白名单 / 前端禁止网络请求 /
  跨窗口真相源）。`ui/` 目录删除、`dsh-logo.svg` 迁至仓库根 `assets/`。
  执行顺序：测试 → 删 ui → 宪法/CI（单独 commit）→ 全量验证 → 完成通知。
- 凭据：阶段 A-D 均已通过闸门与实机验证（见前四条约）；本预告为宪法修改
  前置知会，修改范围与方案 §7 清单一一对应。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 D 落地（1008cd6） —— guan

- 变更：`pages/BootIndex.tsx` 整页、`components/boot/{BootStep,BootTimeline}.tsx`
  新增、ErrorCard diag 形态、`lib/events.ts` 总线装配时机、`pages/BootMode.tsx`
  握手时序、方案文档 §3.3（总线裁定同步）。**四页至此全部迁入 React**。
- 影响：一处时序裁定周知——事件总线要求在**页面任何播种 invoke 之前**注册；
  实现为模块加载期装配（详情见方案 §3.3 与 lib/events.ts 注释）。stage B 中
  BootMode「先 invoke 再导航」的写法本轮已更正为旧握手（携参回启动页由
  BootIndex 落地）。
- 凭据：typecheck/lint/build 全绿；release 冷启动事件链全通（日志钉板）；
  BootIndex 静态帧经 dev 预览核对；下载条/错误卡的实机触发依赖特定失败路径，
  逐行对照旧码迁移（已复核）。阶段 E 前 master 中间态照旧：壳页面功能已全，
  待删 `ui/` 与宪法修订。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 C 落地（d780855） —— guan

- 变更：`frontend/src/pages/{BootMode,BootSelector}.tsx` 整页、
  `components/boot/`（DownloadProgress/ErrorCard/VersionChip/PulseBar 落地，
  阶段 D 复用）、`hooks/usePlatform.ts`、文案层 mode/selector 扩展。
- 影响：仅周知 + 一项待办迁移——**Mode 页（运行环境选择）是 Windows-only
  表面**，React 版实机目视验证待 Windows 环境（非 Windows 访问按裁定防御性
  回启动页，本机已验证该兜底路径编译正确）。其余同前：master 中间态渐次回填。
- 凭据：typecheck/lint/build 全绿；release 实机 fixture 双工作台触发选择器
  （双卡片 + DEFAULT/CUSTOM 徽标 + 版本芯片渲染正确，截图存档）；点击官方卡
  打通 `choose_profile` IPC 全链路（shell.log 钉板：dsh 启动 profile=web →
  1.3s 就绪 → 导航工作台）。测试进程与 fixture 均已清理。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 B 落地（5ce1296） —— guan

- 变更：`frontend/src/components/about/`（新建四组件）、`pages/About.tsx` 整页、
  `content/zh-CN.ts` 文案扩展、`index.css` token 改名、`App.tsx` 预览钩子、
  方案文档 §3.5 同步。关于窗口自旧 `ui/about.html` 完整迁入 React。
- 影响：仅周知 + 一处 token 命名裁定——`--color-muted/--color-accent` 与
  shadcn 语义层重名导致工具类被覆盖（实机截图发现文字近白），域 token 更名
  **dim / brand**；后续页面（阶段 C/D）直接用新名。master 中间态照旧：壳骨架页
  渐次回填，阶段 E 收口删 `ui/`。
- 凭据：typecheck/lint/build 全绿（gzip 137KB，<500KB 复审线）；release 实机
  截图验证整链路（自动首查→upToDate、三维度真实数据、工作台地址注入）；
  配色修复以构建产物 CSS 钉板（`.text-dim{color:var(--color-dim)}` 解析唯一，
  内联映射机制反向解释原 bug）；本地 release 产物已重建包含修复。

### 2026-08-27 补记 · 阶段 A 目视验证完成（步骤 20/21 清账） —— guan

- 变更：`docs/frontend-migration.md` §3.1 一处标注（钉板句从「待复核」改为
  「已复核通过」）。无代码改动。
- 影响：仅周知——release 产物内以临时第二 webUi profile 触发
  `/selector` 直达，**SPA fallback 实机命中**（主窗口渲染 BootSelector 页，
  步骤 21 钉板完成）；about 窗口经菜单打开，label 路由渲染 About 页（步骤 20）。
  临时 fixture（仅含 package.json 的 `~/.dsh/profiles/probe-dual-webui`）
  已删除，真实环境零残留。
- 凭据：实机截图两帧（主窗口 /selector 内容 + 关于窗口内容）；进程已退出。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 A 落地（a9c1656） —— guan

- 变更：`frontend/`（新建 41 文件）、`src-tauri/tauri.conf.json`、
  `src-tauri/src/lib.rs`、`.github/workflows/build.yml`。Vite+React+TS+Tailwind v4
  + shadcn/ui 七组件脚手架；窗口 label 路由；四页占位骨架；Rust 主/about 窗口
  改载 SPA 根、selector/index 跳转改 pathname；platform_script 扩 `{os,wsl}`。
- 影响：**master 中间态**——自本 commit 起 release 产物壳页面为 React 骨架
  （功能回填顺序 B About → C Mode/Selector → D Index，阶段 E 收口删 `ui/`
  并改宪法）。与占用声明的偏差仅一处：CI node 步骤自阶段 E 提前（frontendDist
  切换后 `cargo tauri build` 硬依赖 npm 构建，不提前则打 tag 即挂）；
  Build installers 工作目录随之移到仓库根。开发者注意：本地构建/调试请从
  **仓库根**调用 `cargo tauri dev/build`（钩子 cwd 发现逻辑要求，见 build.yml 注释）。
- 凭据：typecheck/lint/vite build 全绿（JS gzip 79.9KB）；`cargo test` 89 passed；
  本机 `cargo tauri build --no-sign` 出 dmg/.app/updater tar 三产物；release 实机
  启动 Rust 全链路通过入工作台。步骤 20/21 的目视项（骨架观感、release 内
  /selector 直达 SPA fallback 实测）待人工复核。

### 2026-08-27 占用声明 · 前端迁移阶段 A 开工（ADR-0008 实施开始） —— guan

- 变更：占用 `frontend/`（新建）、`tauri.conf.json`（frontendDist 切换）、
  `.github/workflows/build.yml`（node 步骤合批）、`.gitignore`；阶段 E 收口时改动
  `AGENTS.md`（§1/§2/§3/§4/§5/§7）与 `docs/roadmap.md`（硬约束 2 与不做清单）。
  实施依据：ADR-0008 + `docs/frontend-migration.md`（commit aab68c1、f21df95）。
- 影响：**宪法级变更预告**——AGENTS §4.2「禁止引入前端构建链」将在阶段 E 按既定
  方案修订为「Vite + React 定向许可」；执行顺序 A 脚手架 → B About → C Mode/Selector
  → D Index → E 测试与治理收口，各阶段完成即在频道知会。阶段 A–D 仅新增文件，
  不动现有 `ui/` 与 Rust 行为；master 保持随时可构建。他人如需动上述文件请先在
  频道协调。
- 凭据：Tailwind v4 + shadcn/ui 兼容性 spike 已通过（2026-08-27 临时目录验证：
  shadcn init 显式识别 v4、七组件生成、strict TS 下 vite build 成功 JS gzip 73KB；
  结论与三注意点已回写方案 §1 并勾销 ADR-0008 行动项）。本条与 spike 回写同 commit。

### 2026-08-27 快车道直推 · 完成通知：roadmap 对照 dsh 源码核查修订 —— guan

- 变更：`docs/roadmap.md`（+43/−27；本档案与该改动同 commit 落盘）。对照
  deepseek-harness v0.1.1-rc.2 源码逐条核查路线图事实主张后修订：事实表重做
  （子包实测 227、`dsh plugin` = pnpm 原样转发、patch 按 id 逐字段赋值且 `config`
  整体替换不深合并、出厂 profile 模板仅 web/headless 且无任何 profile 管理官方命令、
  dsh 无插件安装/卸载 UI），新增「事实边界与陷阱」清单（非法名校验、profiles/node_modules
  符号链接农场、`.credentials.yaml` 三条硬约束等）；行动项修正：4.3 创建路径弃
  agent-presets 误用改为半官方 plugin-add 引导、4.4 pnpm 失败模式入错误处理并补全
  运行时状态枚举、4.2 updater 测试表述更新、编号引用与版本头（v0.4.7 起）修正。
- 影响：两项裁定仅周知——① 4.3④ 默认启动 profile 持久化到 `settings.json`，经
  维护者批准作为第二最小面例外（落地实现时同步登记 AGENTS §6 后方可合入）；
  ② Next 阶段（Profile 管理器）开工前须先做两个 spike：GUI 环境（无 shell rc）
  pnpm 经 `dsh plugin` 转发链的可用性验证、复制/重命名 profile 的引用清点。
- 凭据：纯文档改动不触运行时；事实主张均锚定 dsh 源码位置（app-boot/src/profile.ts、
  vendor/include/src/index.ts、apps/cli/src/plugin.ts 等）；残留检查干净
  （旧引用 197 包数/（3.4）/零测试 表述已全部清零）。

### 2026-08-27 建档 —— guan

- 变更：新建本档案；`AGENTS.md`（§2 目录树 / §8.5 收尾三件事 / §10 知会落档条目）
  与 `CONTRIBUTING.md` §0 各挂一处指针。
- 影响：协作者此后按上表登记知会；本次指针挂接触宪法级文件，本条即为对其的知会。
- 凭据：纯文档改动，不触运行时。

### 2026-08-27 补记 · 完成通知：删除前端顶栏「关于」入口 —— guan

- 变更：commit `8075eea`——`ui/index.html` / `ui/mode.html` / `ui/selector.html` /
  `src-tauri/src/lib.rs` / `build.rs` / `capabilities/default.json`。删除三页壳顶栏
  「关于」按钮及 `open_about` IPC 整链，与原生常驻入口重复（Windows 启动页问题报告）。
- 影响：**触宪法级**——`AGENTS.md` §7 IPC 注册表移除 `open_about`、「更新常驻入口」
  条目改写为裁定后状态；关于面板此后只能经菜单（macOS）/ 托盘（非 macOS）打开。
  仅周知，无需动作。
- 凭据：`cargo test` 全绿（89 passed）；diff −36/+6 已逐行人肉复核；本条视为对
  该次宪法修订的知会。

### 2026-09-07 完成通知：boot 链路修复批 + 跨平台测试面建立 —— guan（AI 起草）

- 变更：8 个 commit（`1daf6ad`..`6894bf7`）——① 引导进度回调阶段化（node 字节/
  dsh 包计数）+ add -g 流式进度解析 + 完成关单补发（修 99% 卡死）；② boot 步骤
  链生命周期（step0 即刻收口 / step2 done / step3 running，修双「运行中」与
  前后步倒挂）；③ Windows 潜伏编译错误修复（ensure_guest_engine 归位 + Path
  导入——master Windows CI 自 0dae87c 起红，本地 windows-gnu target check 抓出）；
  ④ SIGTERM/SIGINT 优雅退出收会话子进程（修孤儿进程）；⑤ 下载进度卡 dsh 形态 +
  ETA 亚秒隐藏 + 启动页撤通栏顶栏（双下巴）；⑥ 控制中心冗余检查更新入口；
  ⑦ repro-boot-scenarios.sh 补 no-runtime/corrupt-dsh/readonly 场景 + auto 全
  矩阵自动化（macOS 10 场景 PASS）；⑧ 新增 boot-smoke.yml 三平台 boot 端到端
  冒烟（仅 workflow_dispatch，重大变更后手动跑；日常 push/tag/release 不跑）。
- 影响：①②⑤ 为用户可感 boot 体验修复；③ 修复后 Windows CI 恢复可绿；
  ④ 补齐 §6「壳与 dsh 严格 1:1」在信号终止路径的缺口；⑦⑧ 为测试面扩充，
  非宪法级（AGENTS 未动）；Linux 无托盘宿主时 setup_update_tray 的 `?` 仍会
  中断启动（CI 侧 dbus-run-session 已绕过）——产品级降级策略待裁定另行开工。
- 凭据：cargo test 161 绿 / clippy -D warnings 干净 / fmt 过；windows-gnu
  target check 绿；前端 typecheck/lint/vitest（105）绿；macOS 场景矩阵 10/10
  PASS；pkill 孤儿检查实证；boot-smoke 三作业首跑待远端验证。

### 2026-09-07 补记 · 三项裁定落档：boot-smoke 实验性 + 托盘降级 + 修复链 Windows 韧性 —— guan（AI 起草）

- 变更：boot-smoke 六轮实证 GitHub hosted Windows runner 环境层启动不稳定
  （stderr/WER 干净、同代码同 env 结果随机），Windows 双作业标
  continue-on-error 实验性不挡闸门（`d4c3adf`）；Linux 作业四连 PASS 维持
  有效闸门。连带裁定三项：① `repair-session.mjs` 主修复路径 rename 加
  EPERM 退避重试（杀软/索引器瞬态锁，重试+退避，穷尽才报错）；② Linux
  托盘初始化失败降级 warn 不阻断启动（i3/sway/精简桌面可正常用壳，
  ADR-0007 边界澄清：托盘是首选常驻更新入口，但入口缺失 ≠ 应用不可用）；
  ③ Windows/WSL 确定性 boot 验证暂缓——走 docs/executor.md 手动清单，
  逻辑层由三平台 build 覆盖，待实机或 self-hosted runner 再收口。
- 影响：② 为 ADR-0007 的边界澄清（非推翻：托盘仍是首选入口），仅周知；
  ③ boot-smoke 的 Windows 作业结果仅作诊断参考，不作为任何合入闸门。
- 凭据：cargo test 162 绿 / clippy 干净 / fmt 过；node --check 过；macOS
  场景矩阵 10/10；Linux smoke 五连 PASS；build 三平台全绿。

### 2026-09-07 发版：v0.9.5 —— guan

- 变更：自 v0.9.4 起的累积发布——boot 体验批（引擎进度阶段化 + dsh 流式
  进度 + 99% 卡死修复、步骤链生命周期、启动页撤通栏顶栏）、Windows 潜伏
  编译错误修复（ensure_guest_engine 归位）、SIGTERM/SIGINT 优雅退出收孤儿、
  sessions 四项 Windows 单测兼容、托盘缺失降级 warn、修复链 rename EPERM
  退避重试、CI 闸门修复（失效 pin / fetch pnpm 顺序）与 boot-smoke 三平台
  冒烟建立（Windows 侧实验性）、repro-boot-scenarios 场景矩阵。详见同日
  两则落档与 commit 列表（1daf6ad..HEAD）。
- 影响：发版；tag 构建走签名安装器 + 更新 feed。
- 凭据：build 三平台全绿（34106505184 起持续）；smoke Linux 五连 PASS；
  Windows 冒烟实验性（环境层不稳定，见同日补记）。

### 2026-09-07 宪法外登记：pnpm 12 构建审批门产品化（ADR-0009 第六次修订）—— guan

- 背景：v0.9.5 引擎档 pnpm 12.3.1 上线当日，插件安装撞
  `ERR_PNPM_IGNORED_BUILDS`——pnpm 12 装完包后对未获批安装脚本的依赖硬失败
  退出 1，并把 allowBuilds 裁决模板写进 profile 的 pnpm-workspace.yaml；
  dsh 转发链透传退出码并跳过 bundle 调和（复现点 12 已入册）。
- 裁定（维护者选项 A）：壳把 dsh/pnpm 文档化的人工出路产品化——失败解析被
  点名包 → 前端逐包裁决框（默认跳过）→ 新 IPC `set_profile_build_approvals`
  受控改写 allowBuilds 单键（pnpm-workspace.yaml **写入例外 #5**，非三件套
  成员；非裁决条目逐字节保留，流式/嵌套/引号键拒绝写入）→ 自动重试原操作。
  AGENTS §6 不变量行 + §7 IPC 清单已同步登记。
- 影响：插件安装/更新/卸载失败面收敛为可自愈流程；allowBuilds 成为壳的第
  五个 profile 受控写点；真实 true/false 取值是用户供应链决定，壳不预填。
- 凭据：cargo test 176 绿（含真实 web profile yaml 全文 fixture）+ clippy
  干净 + fmt 过 + Windows 交叉 check 过；前端 typecheck/lint/test 全绿
  （buildApprovals 纯逻辑 4 例）。

### 2026-09-08 问题记录095 批 1：会话误判回归修复 + 控制中心反馈面治理—— guan

- 修复 ①：engine_session_alive 被 19ec36e（SIGTERM 批）机械改写为
  is_none_or 致语义反转（无会话=存活），叠加 mtime 窗口把刚收尾会话误标
  「进行中」；恢复 is_some_and + 脚本侧 active 增订 endState==='open'
  降噪子句（漏标两轮间隙的代价经裁定接受）。
- 修复 ②：MCP 面板「无限刷新 + 报错轰炸」——onNotice 内联引用不稳 +
  effect 依赖 loadData + catch 弹 toast 构成失败循环；改面板内错误态 +
  重试，运行态快照降级为辅助信息，删除无价值的刷新按钮。
- 治理 ③：全站刷新按钮清单裁定——删 2（Profile 顶栏/MCP）、补语义标签
  2（日志拉取/凭据重读，去硬编码走 i18n）、换图标 2（检查更新/更新插件
  停用 RefreshCw 防混淆），其余 9 处保留（错误重试/强刷绕缓存/编辑器
  重载均有真实语义）。
- 凭据：cargo test 177 绿 + fmt/clippy/Windows 交叉 check 过；前端
  typecheck/lint/109 测试全绿。问题记录095 剩余两项（插件边界梳理、
  下载队列）设计中，另批开工。

### 2026-09-08 发版：v0.9.6 —— guan

- 变更：自 v0.9.5 起累积——pnpm 12 构建审批门产品化（安装撞
  ERR_PNPM_IGNORED_BUILDS → 逐包裁决对话框 → allowBuilds 受控写入
  （写入例外 #5）→ 自动重试，ADR-0009 第六次修订 + 复现点 12）；
  会话「进行中」误判回归修复（engine_session_alive 反转 +
  endState 降噪子句）；MCP 面板失败死循环根治（面板内错误态）与
  全站刷新按钮语义化治理（删 2/补 2/换 2/留 9）。
- 影响：发版；存量用户经更新 feed 收提示。挂账：插件中心职责边界
  （问题记录095 #3/#4）留下个小版本。
- 凭据：cargo test 177 绿 + fmt/clippy/Windows 交叉 check 过；前端
  typecheck/lint/109 测试全绿；tag 构建验证随 CI。

### 2026-09-08 fix(boot)：selector 页双下巴清尾 + 悬浮胶囊误挂壳页面根治 —— guan

- 双下巴残留：af0e3ee 只撤了启动页通栏顶栏，selector 页漏网——原生标题栏
  之下仍叠一条导航（wordmark/SELECTOR 徽章/控制中心/版本芯片）。同口径
  撤除；控制中心入口经菜单/托盘/全局快捷键可达，boot 期页面不再重复承载。
- 根因补刀：截图里 selector 顶上多出的第二颗「控制中心」是 Rust 注入的
  悬浮胶囊——回环 hostname 判断在 macOS 被壳页面 origin（tauri://localhost，
  hostname 恰为 localhost）误命中，selector/启动页/控制中心/关于页全部
  误挂。挂载条件收紧为「location.origin 与 get_workbench_url 精确比对」，
  origin 未就绪一律不挂；工作台页（http://127.0.0.1:*）行为不变。
- 凭据：前端 typecheck/lint/112 测试绿；cargo test 182 绿 + fmt/clippy
  干净（probe_no_open 首跑偶发失败，单跑与全量复跑均过——存量偶发，与
  本次改动无关，留观）；vite 预览目检 selector 页顶栏已消失。

### 2026-09-08 fix(plugins)：市场非 npm 来源安装必败修复 + 弹窗溢出治理 + 插件职责边界立宪（ADR-0011）—— guan

- 缺陷（用户实测复现 dsh-pet）：线上 registry 3363 条中 51.6%（1736 条）
  install spec 为 `github:owner/repo`（含 `#path:/子目录` 片段）或带成对引号的
  tarball 直链，`validate_plugin_spec` 白名单不含 `:` `#` 必拒——市场半数插件
  点安装即报「包名只允许字母数字与 @/._^~*-」。pnpm 原生支持三形态，
  唯一挡路点是壳的校验白名单。
- 修复：校验谓词拆二——安装/卸载/更新入口换 `validate_install_spec`
  （npm / github[:#frag] / https tarball 三形态；总长 512、npm 形态仍守 214；
  `#path:` 空值 fail-closed；仍拒前导 `-`、空白、`><` 区间）；更新检查与
  选版本保持严格 npm 判别（防拿 github 形态打 packument）。前端预检镜像
  同步；registry 提取层剥离成对引号；市场弹窗提交前预检。恶意反例入测。
- UI：`DialogContent` 基类无 max-h 无滚动，fixed 居中定位下视口不足即上下
  两端溢出且不可滚动救援——补 `max-h-[calc(100dvh-2rem)] overflow-y-auto`，
  全站弹窗受益。
- 职责边界立宪（ADR-0011）：跨 profile 归插件中心、单 profile 归详情；
  安装来源三形态两面对齐。CONTEXT.md 新增插件领域词汇四条；AGENTS §9
  登记索引；roadmap 4.4 落地记录补 09-08 行。
- 小版本挂账：分发 github 包改从来源 package.json dependencies 取 spec；
  跨 profile 更新检查 + 批量更新 + 下载队列（问题记录095 #4）；动词统一
  「安装到…」/Tab 名「插件」/删 ProfileDetailDialog.tsx 死代码。
- 挂账实测：git dep 的安装/更新/卸载需 dev 环境各实测一轮（update 对
  git dep 是否重解析默认分支存疑）。
- 凭据：cargo test 184 绿 + fmt/clippy 干净；前端 typecheck/lint/117 测试绿。

### 2026-09-08 fix(uiux)：静默失败与引用不稳止血（UI/UX 评审批次 A：U1/U3/U4/U5）—— guan（AI 起草）

- 依据：`docs/uiux-review-2026-09-08.md` 批次 A（「用户看到的与事实相反」类）。
  ADR-0011 合入 `413570d` 后其文件面占用释放，本批为释放后的首个前端改动。
- U1 BLOCKER（5 个面板无限重试 + 无限弹 toast）：`ProfileManager` 曾以**内联箭头**
  传 `onNotice` → 每次渲染新引用 → 子面板 `useCallback([onNotice])` 失效 →
  `useEffect` 重跑 → 失败又通知 → `setToast` → 父重渲染 → 死循环。三处改传稳定引用
  `showToast`，并加裁定注释说明为何禁内联（McpManager 已踩过同坑）。
- U3 HIGH（诊断页伪造结论）：采集失败时 `report ?? {全缺失}` 渲染出「Node/pnpm/dsh
  全部未检出」的**伪造环境损坏报告**。改为失败且无缓存 → 报错块 + 重试，伪造分支删除。
- U4 HIGH（settings.json 键丢失）：`set_shell_settings` 是**整体覆盖写**
  （`settings.rs::save` 序列化全字段、无 merge），而 `PreferencesPane` 与
  `i18nStore.setLocale` 都在读取失败时以 `{}` 为基线回写 → 清空 `defaultProfile` /
  `locale` / `dismissedUpdate` 等未参与本次修改的键。抽 `lib/shellSettings.ts`
  安全基线：基线必须来自一次**成功**读取，读失败即中止（不写）；两处调用点收口；
  设置页读取失败改为渲染错误块 + 重试并停用保存。
- U5 HIGH（迁移假成功）：分发时勾了「迁移配置」但 `copyPluginConfig` 失败/未覆盖，
  仍弹「分发完成」。改为按实际结果通知（失败/未覆盖 → warn 文案）。
- 影响：仅周知。**遗留两条**已登记问题记录：① `setLocale` 存储失败仍 resolve →
  设置页会弹「设置已保存」（假成功 toast，批次 B）；② Rust 侧 `set_shell_settings`
  无 merge 语义，前端安全基线只是止血，任何新调用点仍可能踩坑——建议后续在
  `settings.rs` 补 merge 或收窄命令面（收窄属契约改动 → 先 ADR）。
- 凭据：`pnpm typecheck` 0 err · `oxlint` 0 warning · `pnpm test` **123 passed**
  （117 → +4 `shellSettings` 纯单测 +2 `onNoticeStability` 源码闸门）。
  复现先行：安全基线改回 `read().catch(() => ({}))` → 首条即红（已验证）；
  `onNoticeStability` 对修复前 HEAD 复算命中 `ProfileManager.tsx:258,263,268`，
  修复后 0 命中（已验证）。U1 缺陷对 tsc/oxlint 不可见（类型一致），故用与
  `ipc.rs` gate_tests 同口径的源码文本闸门兜底（`?raw` 读入，不引 DOM 测试栈）。

### 2026-09-08 fix(a11y)：键盘与读屏可达性（UI/UX 评审批次 B1）—— guan（AI 起草）

- 依据：`docs/uiux-review-2026-09-08.md` 批次 B。本次只做「能操作、能听见」两类，
  表单 label 关联 / 标题层级 / tab 语义 / 动效降级留批次 B2。
- U2 BLOCKER（主操作键盘不可达）：`ProfileRow` 整卡 `div onClick`。**卡片不能整体
  改 `<button>`**——内部还有启动/重启/更多菜单按钮，嵌套交互元素既非法又让读屏
  语义错乱。改为「名字即主控件」：名字变 `<button>` + `aria-current`，卡片用
  `focus-within:ring-2` 呈现整卡焦点环，指针点击行为不变。
- toast 播报：`role="status" aria-live="polite" aria-atomic` 必须挂在**常驻**容器上
  ——挂在随 toast 挂载/卸载的节点上，部分读屏会整条漏播；容器恒在 DOM，动画只作用
  于内层 motion.div。顺带补 `title`（消息 truncate 到 420px，长错误原本看不全）。
- 8 个 `Switch` 全部补 `aria-label`（评审记 7 处，实际含 `BootMode` 共 8 处；后者靠
  `<label>` 包裹关联，仍补显式名称）。
- 4 个图标按钮补名称：MCP 删除（原**无任何名称**）、MCP 删 ENV 行（原只有「×」）、
  市场清空搜索、ProfileRow 重启（原仅 `title`）。
- **机器闸门（新增）**：`ui/switch.tsx` 的 `SwitchProps` 改为类型层强制——开关必须带
  `aria-label` 或 `aria-labelledby`，漏写即 `pnpm typecheck` 红；`__tests__/switchA11y.test.tsx`
  用 `@ts-expect-error` 钉住闸门存在性（把类型退回全可选立即报 `TS2578`，已验证）。
  类型闸门优于源码文本闸门：不误判、不依赖路径、覆盖未来所有新调用点。
- 影响：仅周知。**实机验证清单待跑**（Tauri 壳需真实窗口，DOM 测试栈按 AGENTS §5 禁用）：
  Tab 走 Profile 列表 → Enter 切换详情；失败 toast 读屏播报；键盘走查 MCP 删除 /
  市场清空搜索 / 日志自动滚底开关。清单见评审档 §13。
- 凭据：`pnpm typecheck` 0 err · `oxlint` 0 warning · `pnpm test` **125 passed**
  （123 → +2 `switchA11y` 类型闸门测试）。

### 2026-09-08 test(ipc)：契约「形状」跨语言闸门（架构评审批次 1）—— guan（AI 起草）

- 依据：`docs/architecture-review-2026-09-08.md` 批次 1（评审结论：若只做一件就做它）。
  P4「名字有闸门、形状没有」——Rust 改字段名 → TS 静默 `undefined`，编译绿、测试绿、
  运行时错，是「编译绿、运行错」的高危面。
- 新增三个闸门（`ipc.rs::gate_tests`）：
  1. `tauri_ts_matches_ipc_commands`——`lib/tauri.ts` 的 invoke 名集 ↔ `COMMANDS` 双向。
     前端是第 5 个消费面，此前完全无闸（今日实测 55/55 手工一致）。
  2. `no_direct_invoke_outside_tauri_ts`——递归扫 `frontend/src`，`invoke(` / `invoke<`
     只允许出现在 `lib/tauri.ts`（AGENTS §4.3「组件内不直接 invoke」）。当前 0 命中。
  3. `ipc_struct_shapes_match_fixture`——14 个 IPC 结构体的**真实 serde 序列化 key 集**
     ↔ 共享 fixture。
- **共享 fixture = 唯一事实源**：新增 `frontend/src/types/ipc-shapes.json`。Rust 侧
  `include_str!` 读入比对真实序列化结果；前端侧 `__tests__/ipcShapes.test.ts` 用
  `AllKeys<T>`（`Required` 展开 + 多余属性检查）断言 TS 接口 key 集与同一份 fixture
  一致。**任一侧改名/换 casing 都会红**。
- 覆盖（14）：`ShellSettings`·`ProfileSummary`·`SessionItem`·`PluginRowState`·
  `AggregatePlugin`·`AggregateSource`·`CopyConfigOutcome`·`RepairOutcome`·
  `SystemDiagnosticsReport` + 5 个诊断子结构。
- 命名风格**保持现状并闸住**：profiles/plugins 域 snake_case（`web_ui`/`pkg_name`/
  `skipped_existing`），sessions/settings/diagnostics 域 camelCase。统一 `rename_all`
  属契约变更未做（须先裁定）；但两侧已钉死，再改名必须同时改 fixture。
- 未纳入：`emit_step` 的 `json!` 手拼 payload 换结构体（随批次 2 拆 `lib.rs` 一并做）。
- 实测抓漏（复现先行）：改 `tauri.ts` 的 `get_shell_settings` 名 → 闸门 1 红；
  改 fixture 的 `web_ui`→`webUi` → Rust 闸门 3 红 **且** 前端 `ipcShapes` 红。
- 凭据：`cargo test` **187 passed**（184 → +3）· `cargo fmt --check` 干净 ·
  `clippy --all-targets -D warnings` 干净 · `pnpm typecheck` 0 err · `oxlint` 0 warning ·
  `pnpm test` **131 passed**（125 → +6）。

### 2026-09-08 chore：清理惰性守卫、死代码与同义反复测试（架构评审批次 0a）—— guan（AI 起草）

- `build.rs` 删 5 条 `../ui/*` rerun 监视：`ui/` 早已不存在，是 2026-08-23「复用旧前端」
  事故留下的惰性守卫；**上游已覆盖**——`tauri-build` 2.6.3 自己就为 `frontendDist`
  （`src/codegen/context.rs:87-95`）与 `capabilities/`（`src/acl.rs:427`）发 rerun 指令。
  留着只会让人以为守卫还在。
- `scripts/regen-icons.sh` 三条 `ui/assets/dsh-logo.svg` 死路径 → `assets/dsh-logo.svg`
  （`frontend-migration.md:66` 早已记录搬移，脚本是漏网）。
- 删 `ProfileDetailDialog.tsx`（671 行、全仓 0 引用；ADR-0011 §5 行动项「随小版本批」）。
- 删 4 个同义反复测试（`credentials`/`diagnostics`/`mcp`/`sessions`）：仅 `import type`，
  在测试体重写逻辑后断言字面量，**从不触达生产代码**——假覆盖率比没有覆盖率更危险。
  被覆盖的纯逻辑仍在组件内，替代路径是「下沉 `lib/` 再写真测试」（归 P7 后续批次）。
- 文档漂移：`tauri.ts:1`「20 个命令」（实为 55）改为 rot-proof 表述（指向
  `ipc.rs::COMMANDS` + 双向闸门）；`frontend-migration.md` 加计数口径补注
  （该文是迁移期记录，「12 个命令」是当时数量）。
- 影响：仅周知。**未纳入**：10 处 clipboard promise 假成功（批次 0b）。
- 凭据：`cargo build` 通过 · `pnpm typecheck` 0 err · `oxlint` 0 warning ·
  `pnpm test` **120 passed**（131 − 11 条假测试）。





### 2026-09-08 fix(frontend)：剪贴板写入唯一入口（架构评审批次 0b）—— guan（AI 起草）

- 缺陷：11 处各自直呼 `navigator.clipboard.writeText(...)`，处理方式分四种——
  完全不接 promise（`ErrorCard` 写失败仍显示「已复制」）、`.catch(() => {})` 后照样
  置位（`BootStep`）、只挂 `.then` 成功分支（8 处，无失败反馈 + unhandled rejection）。
  **更正评审原判**：原文写「`BootStep.tsx:34` 是唯一正确写法」不成立——它吞掉拒绝后
  仍立即置位，与其余各处同病；另外 `SessionManager` 还有一处复制路径（`:219-225`）
  原表漏记，实为 11 处。
- 修法：新增 `lib/clipboard.ts::writeClipboard`（`write` 可注入 → 纯逻辑可测）+
  `hooks/useCopy`（成功才置位、自动复位、不收回调参数以免引用不稳，同 onNotice 裁定）。
  11 处改为 `await copy(...)` 后按结果分流：有 toast 渠道 → `onNotice(t.error.copyFailed)`，
  boot 页无 toast → `logger.warn`。
- 机器闸门：`__tests__/clipboardGate.test.ts`（`import.meta.glob(?raw)` 扫全量源）
  断言 `clipboard.writeText` 只允许出现在 `lib/clipboard.ts`；探针文件实测被抓出。
- 影响：仅周知。
- 凭据：`pnpm typecheck` 0 err · `oxlint` 0 warning · `pnpm test` **126 passed**
  （120 → +4 `clipboard` 单测 +2 闸门）。

### 2026-09-08 fix(a11y)：表单标签与标题层级（UI/UX 评审批次 B2）—— guan（AI 起草）

- 表单标签 12 处：8 处搜索框/凭据框只有 placeholder 当标签（placeholder 不是无障碍
  名称，输入后即消失）→ 补 `aria-label`（搜索框复用同一 i18n 键，凭据框新增
  `console.keyInputLabel`）；`McpManager` 服务器名/命令/参数 3 处 `<label>` 无
  `htmlFor` → 补 `id` + `htmlFor`；ENV 组标题与市场安装源只读框的 `<label>` 指向
  非控件 → 改 `<span>`（ENV 容器加 `role="group"` + `aria-label`）；两处 Radix Select
  的 `<label>` 无法关联 → `SelectTrigger` 加 `aria-label`。
- 标题层级：页面 h1 → 面板/节 h2 → 卡片/子块 h3。面板标题 h3→h2（凭据/引擎设置/
  诊断/偏好×4/会话/MCP/详情空态/客户端更新卡）；`DiagnosticsPane` 5 个指标卡 h4→h3
  （否则 h2 后跳 h4）；市场与已装总览的卡片 h3 缺父级 h2 → 补 `sr-only` h2。
  **更正评审原判**：`ProfileDetailPane.tsx:312 h3 先于 :327 h2` 实为空态与选中态两个
  互斥分支，不存在渲染顺序问题，仅把空态标题对齐到 h2。
- 机器闸门：`__tests__/formLabels.test.tsx`（`import.meta.glob(?raw)` 扫全量 .tsx）
  断言每个 `<input>` 至少有 aria-label/aria-labelledby/id/`<label>` 包裹/type=hidden；
  探针实测被抓出。正则把 `=>` 当整体跳过，避免 onChange 箭头把标签截断。
- 影响：仅周知。**B2 残留**：tab roving tabindex/aria-controls、prefers-reduced-motion、
  命中区 <24px、「仅靠 title 命名」15 处。
- 凭据：`pnpm typecheck` 0 err · `oxlint` 0 warning · `pnpm test` **128 passed**（126 → +2）。

### 2026-09-08 refactor(uiux)：字号刻度、断点对齐、页头吸顶、会话行去重（评审批次 D）—— guan（AI 起草）

- U7 字号 token 化：`@theme` 新增 5 档（`--text-micro/meta/label/note/lead` =
  9/10/11/13/15px，**逐像素等同改造前**，避免视觉回归）；161 处 `text-[Npx]` 全量
  替换（38 个文件）；`ui/button.tsx` 的 `text-[0.8rem]`→`text-xs`（12.8→12px，sm 按钮，
  视觉无感）。闸门 `__tests__/fontTokens.test.ts` 禁止再出现任意字号。
- U8 主从布局断点 `lg`(1024)→`md`(768)：`ProfileManager` 与 `SystemConsole`。窗口
  最小宽度 860/960 均 ≥ 768 ⇒ **允许的任何窗口尺寸下都不再塌成单列**。只改断点、
  不动窗口尺寸（评审 §10 明令「不在同一 PR 里同时改断点与窗口尺寸」）。
- U10 页头吸顶：`ProfileManager` 页头 `sticky top-0 z-20` + `bg-bg/90 backdrop-blur-sm`，
  负外边距抵消 `PageShell` 的 px-4/6/8；长列表滚动后视图切换入口常驻。
- U14 会话行去重：右侧操作区**只留动作**——删掉与左徽标重复的「运行中」「健康/未知」
  胶囊（原来三选一渲染），仅在「需修复且非运行中」时给修复按钮，修复/复制/删除不再被挤。
- 影响：**需人工目检两点**（Tauri 需真实窗口，本次未实机验证）：① 860px 宽时左栏
  Profile 卡片不挤（4/12 栏 ≈ 280px）；② 吸顶页头滚动时不遮首行内容。
- 凭据：`pnpm typecheck` 0 err · `oxlint` 0 warning · `pnpm test` **130 passed**（128 → +2）·
  `pnpm build` 通过（确认 5 档 token 生成 `.text-meta` 等工具类）。

### 2026-09-08 fix(uiux)：对比度达标与 dark 变体清理（评审批次 C）—— guan（AI 起草）

- U6 对比度：实测「既 ≥4.5:1 又能与 `dim` 明显区分」的浅灰不存在（浅底上需 ≈#656d7c，
  与 dim(#626a7a) 已无差别），故**三级灰阶压缩为两级**：`--color-faint` #a0a7b6→**#6b7280**
  （bg 4.55 / panel 4.83 / line-soft 4.27，最后一项为记录在案的边际值）；
  `--color-ok` #2f9e44→**#27793a**（3.45→5.41）；`--color-warn` #d9480f→**#c2410c**（4.30→5.18）。
- 品牌蓝**降级为图形象**（边框/描边/色块/点，3:1 适用）：`text-brand`→`text-brand-deep`
  （98 处）、`bg-brand text-white`→`bg-brand-deep text-white`（15 处）、shadcn
  `--primary`/`--destructive` 同步。
- 原始调色板同步修正：浅底文字色统一 -700 档（amber-500 2.15→5.02、emerald-500
  2.54→5.48、sky-500 3.0→5.93、rose-500 3.4→6.29、violet/indigo-500→-700；含 /70 变体
  改实色）；`bg-amber-500 text-white`（修复按钮，白字 2.15）→`bg-amber-700 text-white`。
  深底文字（toast/日志终端）保持 -300/-400 档不动。
- U12 `dark:` 去留 → **删 40 处**：全仓无 `.dark` 应用点（不可达），且与 index.css
  已记录的暗色方案（「追加 `.dark` 覆盖语义变量，组件零改动」）冲突——留着是地雷，
  一旦启用会以原始调色板破坏 token 体系。
- 机器闸门：`__tests__/contrast.test.ts`（5 条）——文字色 token ≥4.5、白字在填充 token
  上 ≥4.5、faint 的 line-soft 边际 ≥4.2、源码禁 `text-brand`、源码禁 `dark:`。
  为此 `vite.config.ts` 增 `test.css: true`（Vitest 默认把 CSS 桩成空串，`?raw` 读不到）；
  配置改用 `vitest/config` 的 `defineConfig` 以获得 `test` 键类型。
- 影响：**需人工目检**配色观感（尤其 faint 变深后的层级、修复按钮由黄转深橙）。
- 凭据：`pnpm typecheck` 0 err · `oxlint` 0 warning · `pnpm test` **135 passed**（130 → +5）·
  `pnpm build` 通过。闸门实测抓漏：把 faint 改回 #a0a7b6 → 两条断言即红。

### 2026-09-08 fix(uiux)：图标与动词统一、Select 截断、aria-label 收口（评审批次 E）—— guan（AI 起草）

- U13 图标错配 4 处：`DownloadCloud`→`RefreshCw`（检查更新不是下载）、`Code2`→
  `ExternalLink`（打开 GitHub 是外链）、`Trash2`→`Eraser`（清屏非破坏性）、
  `Clipboard`→`Copy`（同行复制路径用 Copy，复制 ID 却用 Clipboard）。
- U13 动词统一：所有刷新类动作以「刷新」起头——刷新扫描→刷新会话、拉取最新日志→刷新日志、
  重新读取凭据→刷新凭据、市场失败态重新加载→重试；复制标签去修饰词（复制完整诊断报告→
  复制诊断报告、复制全部日志→复制日志）；引擎设置面板「复制代码」→「复制 YAML」（内容是 YAML）。
- U15 Select 截断：`ui/select.tsx` 的 `SelectItem` 对纯文本子节点自动挂 `title`；
  `PluginOverview` 筛选 Select `w-48`→`w-56` + trigger 补 `title`（原长 profile 名被截成
  「只显示 3 个字符」且无法悬停读全）。
- 中文 `aria-label` 13 处 → i18n（读屏在英文环境不再读中文）。
- 影响：仅周知。**i18n 正文抽取拆为批次 E2**（实测 2,876 字 / 45 文件，其中 6 个大文件
  60 条可枚举文案 + 模板串 + 17 个较小文件；纯机械但横跨 25+ 文件，按 §8.1 单独立项）。
- 凭据：`pnpm typecheck` 0 err · `oxlint` 0 warning · `pnpm test` **135 passed**。

### 2026-09-08 refactor(tauri)：注入脚本迁出 Rust 原始字符串（架构评审批次 3）—— guan（AI 起草）

- 缺陷（P3）：330 行 JS 活在 Rust `r#"…"#` 里——tsc/oxlint/语法检查全部看不见，改错只能
  等运行时。
- 落地：三段全部迁到 `frontend/src/injected/*.js`，Rust 侧改 `include_str!`（同
  `updater.rs:461` 跨语言引用范式）：`memory-policy.js`（68 行）、`link-hook.js`（24 行）、
  `switcher.js`（238 行）。`lib.rs` 3,039 → 2,713 行（−326）。
- **迁出即见真章**：工具链首次扫到这 330 行立刻报 6 条——`memory-policy.js` 的
  `scan(root)` 是死代码（MutationObserver 已内联同逻辑、从未调用）、`switcher.js` 的
  `var isMac` 赋值未用、4 处 `catch (e)` 未用参数。已一并清理（改 `catch {}`）。
- 放 `frontend/src/injected/` 的理由：`pnpm run lint` 覆盖该目录 → 即被 oxlint 纳管；
  Vite 只打包被 import 的模块 → 不进产物；tsconfig 未开 `allowJs` → tsc 不误编译。
- 影响：仅周知。运行期行为不变（脚本内容逐行迁移，仅删死代码与未用参数）。
- 凭据：`cargo test` 187 绿（含内存策略三条子串断言）· `cargo fmt --check` 干净 ·
  `node --check` ×3 通过 · `pnpm lint` **0 warning**（迁出前这 330 行是 lint 盲区）。

### 2026-09-08 test(rust)：去 flaky + 强制 node 用例 + 网络面离线覆盖（架构评审批次 5）—— guan（AI 起草）

- 去 flaky（P8）：`resolve.rs` 三处挂钟断言过紧（`finds_flag_on_stderr` 的 `< 5s` 并行全量
  跑实测挂过一次）。口径改为「区分命中早退与等满自然退出」——假体 sleep 拉到 60s，
  断言留 3–6× 负载余量（10s/20s），回归形态必超。
- 静默跳过 → CI 硬失败（P6）：`sessions.rs` 4 个修复链用例缺 `node` 时 `return`（0 断言
  通过 = 假绿）。新增 `require_node_or_skip()`：本地允许跳过并打印提示，CI 由
  `DSH_TEST_REQUIRE_NODE=1` 强制 panic；`.github/workflows/build.yml` Unit tests 步骤已挂。
  **实测**：剥掉 PATH 里的 node + 打开开关 → 立即 panic。
- 网络面离线覆盖（P6）：`updates.rs::read_body_capped`（ureq 上限漂移的显式替代实现）
  此前零测试 → 补 5 条（未超限/恰好等于上限/超限文案/空体/非 UTF-8 上下文）。4 → 9 条。
- **欠账**：HTTP seam 注入（离线覆盖镜像链回退/超时）、`repair-session.mjs` fixture 驱动。
- 凭据：`cargo test` **192 passed**（187 → +5）· `cargo fmt --check` 干净 ·
  `clippy --all-targets -D warnings` 干净 · `DSH_TEST_REQUIRE_NODE=1` 下全量绿。

### 2026-09-08 ADR-0012 立项：启动失败错误类型化（架构评审批次 4 前置）—— guan（AI 起草）

- 依据架构评审 P5：`classify_boot_error`（lib.rs）把错误文本小写后做子串匹配来决定
  错误卡标题/建议/按钮——**措辞即契约**（上游改文案即静默落兜底）、**同词不同因**
  （任何含 timeout 的错误都判「网络不可用」）、**不可穷尽**（新增失败模式不触发编译错误）。
- ADR-0012 决策：只为 **boot 失败路径**引入 `BootFailure` 枚举（含 `Unknown { detail }`
  兜底），`classify_boot_error` 改造为 `from_legacy_detail` 映射表（与今天子串规则逐条等价
  并纳入离线单测）；其余 110 处 `Result<_, String>` 本次不动（§8.2 增量）。
  备选：B 强化分类器（措辞仍是契约，治标）／C 全仓类型化（一次性大改）／
  D 引 thiserror/specta（新依赖 + 与既有形状闸门重复）——均已记录否决理由。
- 状态：**草案，待维护者评审**；行动项 4 条在 ADR §5（枚举 + 等价映射表单测 → boot 路径
  改造 → 前端结构化分支 + `ipc-shapes.json` 登记 → AGENTS/台账回收）。
- 影响：AGENTS §9 索引已加 ADR-0012 一行（宪法级文件，改动即本广播）。**代码未动**——
  §9 要求先立 ADR 再动代码，实施待评审通过后另起一批。
- 凭据：纯文档（ADR 91 行）。

### 2026-09-08 refactor(tauri)：拆出 commands 模块（架构评审批次 2 第一步）—— guan（AI 起草）

- **`lib.rs` 2,713 → 1,778 行（−935）**：55 个 `#[tauri::command]` 按域迁到
  `src-tauri/src/commands/`——boot(5) / profile(10) / plugin(12) / console(13) /
  session(4) / update(5) / link(3) / window(2) / market(1)，共 9 文件 947 行。
- **先修闸门再搬**：`ipc.rs` 的 handler 解析器原把条目当裸标识符，搬完会变成
  `commands::profile::list_profiles` 而误判「未登记」→ 先改为取最后一段路径，并补
  `handler_parser_strips_module_path`（合成源码正反例）。搬完 55 条 handler 全绿。
- 可见性：`ShellState` 与 17 个被命令调用的辅助函数、`EXTERNAL_URL_HOSTS` 改 `pub(crate)`；
  命令层只依赖 `crate::…`，不反向依赖 `commands::`（lib.rs 仅菜单回调一处改为全路径）。
- **未纳入**：`ui/`（create_main_window 352 行 + 菜单/托盘/窗口）与 `boot/`
  （run() 400+ 行 + 会话守卫 + emit_* 族）。本次只搬叶子层（命令 → 域模块），
  不动启动管线，回归面可控。
- 影响：仅周知，无行为变更（纯搬迁；IPC 名集/形状/契约均未动）。
- 凭据：`cargo test` **193 passed**（192 → +1 解析器用例）· `cargo fmt --check` 干净 ·
  `clippy --all-targets -D warnings` 干净 · 前端 `typecheck`/`lint` 0 warning/`test` 135 全绿。

### 2026-09-08 refactor(tauri)：拆出 ui 模块（架构评审批次 2 第二步）—— guan（AI 起草）

- **`lib.rs` 1,778 → 1,372 行**（相对原始 3,039 已 −55%）：窗口/菜单/托盘层迁到
  `src-tauri/src/ui.rs`（423 行）——`create_main_window`（含三个 initialization_script
  装配与导航拦截）、`resolve_resources_dir`、`build_app_menu`、`build_tray_menu`、
  `setup_update_tray`、`refresh_app_menu`（`#[cfg]` 双实现）、`current_active_mode`、
  `open_about_window`；注入脚本常量 `WEBVIEW_MEMORY_POLICY_SCRIPT` 随之下沉（只被
  `create_main_window` 用），`lib.rs` 测试引用改 `crate::ui::…`。
- 依赖方向单向：`ui::` → `crate::{ShellState, is_allowed_external_url, settings, emit_*}`；
  `lib.rs::run` 与 `commands::window` 调用 `ui::create_main_window`。
- **未纳入（最后一步）**：`boot/`——`run()`（400+ 行）、`ShellState` 与会话守卫、
  `emit_*` 族、错误分类、`init_tracing`/`TeeWriter`。这一步动启动管线本身，需单独一轮
  并逐段验证。
- 影响：仅周知，无行为变更。
- 凭据：`cargo test` **193 passed** · `cargo fmt --check` 干净 ·
  `clippy --all-targets -D warnings` 干净 · 前端 `typecheck`/`lint` 0 warning/`test` 135 全绿。

### 2026-09-08 refactor(tauri)：拆出 boot 模块（架构评审批次 2 第三步 · 批次 2 收口）—— guan（AI 起草）

- **`lib.rs` 1,372 → 574 行**（相对原始 3,039 **−81%**）：启动管线迁到
  `src-tauri/src/boot.rs`（818 行）——`ShellState`（字段 pub(crate)）+ executor 会话
  拉起与 1:1 守卫、启动/切换/重启编排、boot 遥测（`emit_step`/`emit_upgrade`/
  `emit_boot_error`/`emit_update`/`refresh_update_ui`）、失败分类与日志刮取、
  `TeeWriter`+`MakeWriter`+`init_tracing`、`install_signal_exit_handler`+`SIGNAL_EXIT`、
  `BOOT_TIMEOUT`/`BOOT_STALL` 与状态缓存。
- **`run()` 有意留在 lib.rs**：它是组合根（装配 Builder / 注册 handler / 挂菜单托盘），
  只接线不承载领域逻辑。
- 最终模块地图：`lib.rs`(574) · `boot.rs`(818) · `ui.rs`(424) · `commands/`(9 文件 947) ·
  `injected/`(3 文件 330 JS) + 18 个既有域模块。`lib.rs` 从「全仓第一热点」变为薄入口层。
- 影响：仅周知，无行为变更（纯搬迁 + 可见性调整）。
- 凭据：`cargo test` **193 passed** · `cargo fmt --check` 干净 ·
  `clippy --all-targets -D warnings` 干净 · 前端 `typecheck`/`lint` 0 warning/`test` 135 全绿。

### 2026-09-08 test(rust)：网络面 HTTP seam 注入 + 镜像链离线覆盖（架构评审批次 5 欠账收口）—— guan（AI 起草）

- **P6 第一条缺口（唯一网络面几乎无测试）关闭**：`updates.rs` 的 `ureq::Agent` 调用收成
  `trait HttpGet`（`get_text`/`get_bytes`），生产实现 `UreqGet`，测试注入 `FakeHttp`
  （URL 子串 → 预置响应 + 调用顺序记录）——不触网、不起 mock server。
- seam 点 5 处：`fetch_packument_with` / `fetch_client_latest_with` /
  `npm_packument_versions_with` / `fetch_market_registry_with` / `fetch_node_map_with`。
  镜像链列表一并注入（生产传 `npm_registry_urls()`/`registry_chain()`，测试传固定链），
  避免用例跟着 `DSH_DOCK_NPM_REGISTRIES` 漂移。
- 新增 11 条离线用例：镜像链回退（坏 JSON / 传输错误 / 形状不符）、全失败报末错、
  非法包名零请求、市场 CDN → GitHub raw 回落、node 映射 packument→tarball 两步
  与 `%2F` 编码、`flate2`+`tar` 现造 tarball 覆盖双文件解包。
- **行为差异（有意，更严）**：`fetch_node_map` 旧 `.take(cap)` 静默截断 → 现显式超限报错，
  读失败回落次镜像（与 `read_body_capped` 同口径）。`dist-tags.latest` 缺失仍是既有
  `?` 提前返回语义，本次不动（已记账）。
- 影响：仅周知，无行为变更（对外 IPC / 契约未动）。
- 凭据：`cargo test` **204 passed**（193 → +11）· `cargo fmt --check` 干净 ·
  `clippy --all-targets -D warnings` 干净 · 前端 `typecheck`/`oxlint` 0 warning/
  `test` 135 全绿/`build` 通过（未动前端，仅回归确认）。

### 2026-09-08 test(sessions)：会话修复脚本损坏类别 fixture 驱动（架构评审批次 5 收口）—— guan（AI 起草）

- **P6 第二条缺口（唯一改用户数据的脚本靠可跳过测试）补齐**：`sessions.rs` 测试里的
  脚手架（临时目录 + 引擎 node shim + mtime 回拨）收成 fixture 原语
  （`fixture_home` / `write_session_fixture` / `install_engine_node_shim` / `scan_verdict`），
  判定改由**一张 8 行损坏类别表**驱动：新增类别 = 表里加一行。
- 表覆盖：健康 / 序列缺失（类别 2）/ 重放重叠（类别 1）/ 悬空 surface replace（类别 4）/
  JSON 不可解析 / 末行截断 / 空文件 / 存储版本高于本构建。两个用例共用该表——
  `scan_classifies_damage_fixtures` 断言 `--scan` 判定 + detail 锚点，
  `repair_verdict_matches_damage_fixture` 断言脚本契约三条（健康不写回不备份 /
  可修复备份 + 修复后健康 + 被丢弃内容消失 / 不可修复非 0 退出 + 字节原样）。
  期望值取自 2026-09-08 逐条实跑原文（非推测）。
- **顺带**：三个重用例脚手架改用 fixture 原语（删 98 行重复，断言一字未改）。
- **暴露的新缺陷已登记（未修）**：`--scan` 对损坏文件判 `unknown`，前端却呈现为
  「无法判定健康状态（可能为活跃会话或引擎未就绪）」且**隐藏 detail**，而脚本已给出确切
  原因（如「第 2 行 JSON 解析失败」）——记 `docs/uiux-review-2026-09-08.md` §18 +
  问题记录 U16，属 UI/UX 面，按 §8.1 另立项修。
- **仍是批次 5 欠账**：WSL 路径仍仅手工验证（`boot-smoke.yml` 仍是 `workflow_dispatch`）。
- 影响：仅周知，无产品行为变更（纯测试 + 文档）。
- 凭据：`cargo test` **206 passed**（204 → +2 表驱动用例）· `cargo fmt --check` 干净 ·
  `clippy --all-targets -D warnings` 干净 · 前端 `typecheck`/`oxlint`/`test` 135/`build` 全绿。

### 2026-09-08 fix(uiux)：破坏性操作统一确认与覆写前备份（UI/UX 评审 U9）—— guan（AI 起草）

- **U9（HIGH）收口**：卸载插件 / 覆写 `.credentials.yaml` / 覆写 `settings.yaml` 三个
  无确认的不可撤销动作，以及三处原生 `window.confirm`（删除会话 / 移除 MCP / 清除 Key），
  全部统一到新组件 `ui/confirm-dialog.tsx`（纯展示外壳，不含文案不发 IPC）；
  `ProfileDeleteDialog` 改用同一外壳（行为不变，取消按钮文案「关闭」→「取消」）。
- **覆写前备份（Rust 新增 `fs_backup.rs`）**：`<文件名>.bak-<unix 秒>`，同秒重复覆写追加
  `-N` 不覆盖既有备份；`fs::copy` 保留权限位（凭据备份同为 0600）；**备份失败即中止写入**
  （确认框已承诺「先备份」，静默降级等于毁约）。新增 `credentials::overwrite_credentials`
  与 `dsh_settings::overwrite_dsh_settings` 覆写入口，命令层改调；`set_provider_key`
  增量写路径不产生备份噪声。
- **闸门** `destructiveConfirmGate.test.ts`：全仓不得出现 `window.confirm`；
  6 个破坏性入口必须引用 `ConfirmDialog`；含闸门自检与清单路径存在性校验。
- 文案：新增 `confirm.cancel` 与 6 组站点键，zh-CN / en-US 同步（`AppCopy` 类型兜底）。
- **未做（登记）**：覆写 `settings.yaml` 仍无 diff 预览（确认与备份已补）。
- 影响：仅周知，无 IPC / 契约变更；`save_credentials_raw` / `save_dsh_settings_raw`
  的磁盘副作用多一个 `*.bak-*` 文件（新增，不删除任何既有文件）。
- 凭据：Rust `cargo test` **213 passed**（206 → +7）· `fmt --check` 干净 ·
  `clippy -D warnings` 干净 · 前端 `typecheck` 0 错 / `oxlint` 0 warning /
  `test` **139 passed**（135 → +4）/ `build` 通过。

### 2026-09-08 fix(uiux)：截断元素补齐 title（UI/UX 评审 U15 剩余项）—— guan（AI 起草）

- **U15 收口**：评审列的是抽样清单，本次按「凡 `truncate` 必有 `title`」复扫全仓，
  补 **22 处**——`McpManager`(2) · `SessionManager`(2) · `MarketPluginCard`(3，插件名
  `title` 原用未加工的 `plugin.name`，改为展示用 `displayName`) · `LogViewerPane`(1) ·
  `ProfileManager`(2) · `VersionChip`(2) · `BootTimeline`(2) · `QuickDshSwitcher`(1) ·
  `SystemConsole`(1) · `ProfileRow`(1) · `ProfileDetailPane`(1) · `BootSelector`(1)。
- **有意不加的 4 处**：`ui/select.tsx` trigger（显示值来自子节点，组件无法自推；`SelectItem`
  已自动挂 `title`）；`DiagnosticsPane` 三处分布图例为短数字串，非用户数据。
- **未加机器闸门**：JSX 元素与 `className`/`title` 的对应关系无法用正则可靠判定（多行
  属性 / 动态类名），误报闸门比没有更糟——改在评审 §20 留复扫口径。
- 影响：仅周知，无行为变更（纯属性补齐）；`title` 同时是浏览器原生 tooltip。
- 凭据：前端 `typecheck` 0 错 · `oxlint` 0 warning · `test` 139 passed · `build` 通过
  （本批未动 Rust）。

### 2026-09-08 refactor(boot)：启动失败类型化 BootFailure（架构评审批次 4 · ADR-0012 实施）—— guan（AI 起草）

- **P5 收口**：boot 失败分类从「错误文本子串匹配」改为 tagged enum（`{"kind":"…"}`），
  新增失败模式触发编译错误。新增 `src-tauri/src/boot_failure.rs`：`BootFailure`
  （`credentials_mismatch` / `incompatible_options` / `network_unavailable` /
  `unknown{detail}`）+ `from_legacy_detail` 兜底表 + `BootErrorPayload`。
- **形状入闸门**：`BootErrorPayload` 登记 `frontend/src/types/ipc-shapes.json`，
  Rust 真实 serde 序列化 ↔ TS 接口 key 集双闸门生效（15 → 16 条）。
- **前端**：`ErrorCard` 按 `failure.kind` 取 `content/{zh-CN,en-US}.ts` 本地化文案，
  取不到回退后端文案（旧缓存载荷兼容）；`normalizeError` 对 kind 做白名单校验。
  附带收益：**en-US 用户不再看到中文错误卡标题/建议**（原后端文案为中文硬编码）；
  `ErrorCard` 三处硬编码中文（DIAG 头 / 启动中断 / 修复建议）随本次入 i18n。
- **纯函数下沉**：事件载荷规整从 `lib/events.ts` 拆到 `lib/eventPayloads.ts`
  （原模块加载期即注册 Tauri 监听，测试 import 会失败），`normalizeError` 得以单测。
- **两处对 ADR 的偏离（证据见 ADR §7）**：① 裸 `timeout` 不再判为「网络不可用」
  （ADR §1「同词不同因」正是要修的缺陷；`network`/`registry` 仍命中）；
  ② 删除 ADR §3A 列出的 `EngineNotReady`——核对 14 处错误来源后确认 boot 路径不经
  `engines::resolve_toolchain`，该变体无生产者。
- 影响：`boot:error` 载荷**新增** `failure` 字段（向后兼容：前端对缺失字段有回退分支）；
  IPC 命令名集/`docs/contract.md` 契约未动。
- 凭据：Rust `cargo test` **218 passed**（213 → +5）· `fmt --check` 干净 ·
  `clippy -D warnings` 干净 · 前端 `typecheck` 0 错 / `oxlint` 0 warning /
  `test` **144 passed**（139 → +5）/ `build` 通过。

### 2026-09-08 fix(uiux)：会话 unknown 状态呈现（UI/UX 评审 §18 / 问题记录 U16）—— guan（AI 起草）

- **缺陷**：`--scan` 对「JSON 不可解析 / 末行截断 / 空文件 / 存储版本高于本构建」判
  `unknown` 并给出确切原因，但前端把 `unknown` 一律渲染成「无法判定健康状态（可能为
  活跃会话或引擎未就绪）」，且 `healthDetail` 只在 `needs_repair` 时展示——原因被吞。
- **修法**：`healthDetail` 改为凡有即展示（需修复 amber / 未知 faint 区分）；
  `unknown` 描述按「是否携带原因」分叉（新增 `statusUnknownDescWithReason`）；
  状态映射下沉纯函数 `lib/sessionStatus.ts::statusMeta(status, t, hasReason)`，
  补 `sessionStatus.test.ts` 3 条（含「有原因时不得再说无法判定」）。
- 影响：仅周知，无 IPC / 契约变更；`needs_repair` 呈现不变。
- 凭据：前端 `typecheck` 0 错 · `oxlint` 0 warning · `test` **147 passed**（144 → +3）·
  `build` 通过（本批未动 Rust）。

### 2026-09-08 fix(ui)：弹窗横向截断根治（grid 轨道被 nowrap 长串顶开）—— guan

- 上批 max-h 修复的后续：overflow-y-auto 把 overflow-x 从 visible 转为 auto，
  使存量横向溢出从「画出卡片外」显形为「裁切」——市场安装弹窗右缘被截、
  安装按钮不可达（用户实测复现）。
- 根因（浏览器实测复刻定位）：grid 隐式列 auto 轨道取子项固有宽度，安装源
  spec 展示串（whitespace-nowrap，389px）经 flex 容器把轨道顶到 414px，
  超出卡片 383px——所有整行元素随之越界。
- 修复：DialogContent 基类补 `[&>*]:min-w-0`，轨道以容器宽度为准，深层由
  各自 truncate 收口。实测复刻树验证：scrollWidth=clientWidth=383、零越界
  元素、spec 正确省略、footer 双按钮入卡。
- 凭据：前端 typecheck/lint/147 测试绿；布局指标经浏览器实测（上）。

### 2026-09-09 fix(plugins)：git 来源构建审批门解析修复——exact key + 第二错误码 —— guan

- 用户实测 dsh-pet（github 来源）坐实 ADR-0011 复审条件预判的形态分叉：
  git 托管包走 `ERR_PNPM_GIT_DEP_PREPARE_NOT_ALLOWED`（非
  `ERR_PNPM_IGNORED_BUILDS`），且 pnpm 不写 allowBuilds 模板文件、改在
  help 示例给出 exact key（`名@完整tarball URL`，含 commit hash，被终端
  折成三行）——旧解析按「最后一个 @ 剥版本段」会把 URL 剥没，写入键对
  不上 pnpm 要求，审批对话框整体不触发，用户直面裸错误。
- 修复（build_approvals）：错误码表驱动双形态路由；git gate 提取 help 示例
  exact key（去空白重组折行，按最后一个冒号切键值，多条目无法可靠切分时
  fail-closed 交人工）；IGNORED_BUILDS 列表遇 git 形态条目整条保留不剥；
  `validate_build_pkg_name` 增 URL 分支（字符集 `@/:._#?&=%~-`、512 上限，
  YAML 写入侧单引号包裹）。fixture 全部取自本机真实日志，锚定引擎自管的
  pnpm 版本（ADR-0010 红利：外部依赖输出形态从开放集变引擎更新时的复核事件）。
- 规程收编（ADR-0011 行动项）：任何新增安装来源/错误形态，收口前必须端到
  端跑通一轮（安装→门槛→审批→重试）——本次两个逻辑 bug 都窝在从未跑通的
  github 安装路径上，单测绿不等于路径通。
- 凭据：cargo test 222 绿 + fmt/clippy 干净（解析器自测还抓出我首版漏
  trim 值的错——fixture 即真实日志的价值）。

### 2026-09-09 fix(diag)：plugin-op.log 改追加式 + 轮转——失败历史不再被覆盖 —— guan

- 缘起：dsh-pet 连续两次安装失败排查时，plugin-op.log 的 truncate 式写入
  让中间失败的输出被最后一次运行覆盖，无据可查（且该文件是后续安装进度
  可视化的增量 tail 底座，截断式会弄乱读取偏移）。
- 修复（run_dsh_forward 日志策略，插件操作与创建链共用）：追加式写入 +
  单代轮转（超 512 KiB 让位 `.1`）+ 每次运行写分隔头
  `==== <UTC 时间戳> | <完整命令参数> ====`；UTC 格式化为纯 std civil
  历法换算，不引时间依赖。消费方核查：仅插件操作调用点一处，无前端/
  控制台读取，路径不变。
- 顺带结论（同批排查）：dsh-pet 实际已安装成功——allowBuilds exact key
  经审批流正确写入、依赖与 node_modules 完整、末次运行退出 0；用户看到
  的「两次失败」含重建前旧错误与中间网络失败（过程日志被旧策略覆盖，
  本修复正是堵这个洞）。
- 凭据：cargo test 224 绿 + fmt/clippy 干净（新增 UTC 锚点与追加/轮转
  行为测试）。

### 2026-09-09 fix(market)：git 来源插件的已装状态判定——聚合补声明 spec 连接键 —— guan

- 用户实测：test 装上的 dsh-pet，市场卡片仍显示「未安装」。根因：git 来源
  的真实包名（@linxin666/dsh-pet）与市场展示名（dsh-pet）不同，installedMap
  按「npm 名 / 市场名」双键匹配必然落空；唯一可靠连接键是安装 spec——
  package.json 依赖声明值与市场条目 install 串尾段天然一致。
- 修复：AggregateSource/PluginEntry 增 `spec` 字段（依赖声明值原样；IPC
  形状闸门三处同步：ipc.rs 断言 / ipc-shapes.json fixture / 前端类型）；
  前端 `installedProfilesFor` 纯函数——名字命中优先、安装 spec 兜底，
  市场卡片按其对齐。npm 来源行为不变。
- 同批日志结论（新追加式日志首次立功）：10:22 向 web profile 的安装停在
  构建审批门（allowBuilds 按 profile 隔离，web 未批）——审批对话框不显示
  错误文案属设计行为；web 侧完成审批即可装上。
- 凭据：cargo test 225 绿 + fmt/clippy 干净；前端 typecheck/lint/150 绿
  （双侧形状闸门同步更新）。

### 2026-09-09 fix(market)：审批门批准后安装串位 profile（选 web 装进 test）—— guan

- 用户实测：安装选 web → 撞审批门 → 批准 → 装进了 test。追加式日志实证
  两次（10:22 web 失败 → 10:23 test 成功；10:42 → 10:45 同型），且 web 的
  allowBuilds 始终没写上——审批写入与重试双双串位。
- 根因（两层叠加）：①安装弹窗的「默认选中 profile」effect 依赖
  profiles/installedProfiles 的**属性引用**，而父组件在窗口 focus 刷新/
  装后回填时会重建这两者——批准对话框开着的时候一次刷新就把用户手动
  改选的目标静默重置回默认值；②旧已装匹配 bug 下默认值恰为字典序第一个
  （test）。审批写入与重试读的都是**实时** selectedProfile → 双双串位。
- 修复：门槛失败瞬间**快照绑定** {profile, pkgs}——审批写入与自动重试
  只认快照，选择期间任何变化不可改写（市场安装弹窗 + 总览分发两处同治，
  含 onClick 直传事件对象的隐患）；默认选中 effect 改为仅随弹窗目标插件
  重算，并注明缘由。ProfileDetailPane 的重试锚定 pane 固定 profile，无此
  风险，未动。
- 凭据：前端 typecheck/lint/150 测试绿（纯前端改动，Rust 无涉）。

### 2026-09-09 docs(adr)：分发机制调研结论落档 ADR-0011（cp 否决 + 队列内联审批裁定）—— guan

- 「分发能不能 cp 过去」调研：否决文件级复制（hoisted 拍平无复制边界 /
  pnpm 记账面失配即被对齐删除 / 绕过 reconcile），钉进 §3 方案 D 防再排队；
  实证同机分发内容零下载（全局 store 命中），真正摩擦 = git 解析触网 +
  per-profile 审批门。
- 队列形态裁定（095 #4 设计输入，维护者确认）：审批门以**队列项内联审核**
  呈现而非模态——待审批状态内联展示被点名包的 允许/跳过 开关，批准即写
  allowBuilds 并自动重试该项，多 profile 分发的审批顺序清账。
- 复审条件扩充：`--offline/--prefer-offline` 与 reporter 透传合并为 dsh
  上游诉求（git 来源分发可完全离线）。

### 2026-09-09 feat(market)：安装队列与下载管理面板——095 #4 第一切片（ADR-0011 队列形态落地）—— guan

- 行为变化：市场安装与总览分发确认后**入队即关窗**，后台串行执行
  （install_plugin 逐项跑，避免并发 pnpm）；「下载管理」按钮（插件中心
  头部，未终结项角标）打开队列面板：排队/安装中/待审批/完成/失败五态，
  完成与失败经既有 toast 冒泡（store notifier 接 ProfileManager showToast）。
- 审批门内联：撞 ignored_builds 的项转「待审批」，面板内直接展示被点名
  allowBuilds 键的 允许/跳过 开关（默认跳过），批准即写该 profile 的
  allowBuilds 并自动重试该项；跳过语义附带不可用告警（ADR 裁定）。模态
  BuildApprovalDialog 自市场/分发两条路径退役（ProfileDetailPane 就地安装
  保留同步流 + 原对话框——单 profile 固定目标，队列化无收益）。
- 实现：lib/queue.ts 纯状态机（可单测）+ stores/queueStore.ts 编排（串行
  pump、审批重试、失败重试、通知接线）+ QueuePanel；列表回填订阅
  lastFinishedAt。profile 快照绑定原则贯穿入队项。Rust 进度流（Channel）
  与 offline 透传为后续切片（ADR-0011 复审条件已挂）。
- 设计闸门回归：重写 MarketInstallDialog 时把 UI/UX 批次已清理的任意字号/
  dark: 变体/text-brand 文字色又带了回来（fork 前旧读数），contrast/
  fontTokens 闸门当场拦下——已按刻度（micro/meta/label）与 brand-deep
  修正；三个设计闸门重新全绿。
- 凭据：前端 typecheck/lint/156 测试绿（新增 queue 纯逻辑 6 测）。

### 2026-09-09 宪法级 + 快车道 · 构建脚本默认批准：pnpm 审批门逻辑退役（ADR-0013） —— guan（AI 协作）

- 变更（commit `0bafb5d`）：27 文件，+532 / −1014（净 −482 行）。
- 缘起（用户实测 bug）：装 `dsh-ssh` 却弹出 `dsh-pet` 的旧 exact key，批准对真
  门槛（`cpu-features`/`ssh2`）无效 → 重试死循环。根因 = 追加式日志（9d341f8）
  让 `ForwardRun.output` 变成历史全量，审批门解析器先撞上旧
  `ERR_PNPM_GIT_DEP_PREPARE_NOT_ALLOWED` 标记。维护者裁定：不再逐包裁决，
  改**默认批准**。
- 决策（ADR-0013，新增）：向 profile 的 `pnpm-workspace.yaml` 幂等写入顶层键
  `dangerouslyAllowAllBuilds: true`（新建 profile 物化后写一次 + 每次插件操作前
  补齐；写失败只告警不阻断）。引擎档 pnpm 12.3.1 实测：两种门槛形态（npm 依赖 /
  git `prepare`）都被压过，含既有 `allowBuilds` 显式 false 与 pnpm 占位模板。
- 退役：`build_approvals.rs`（558 行）、`set_profile_build_approvals` IPC（ipc.rs /
  lib.rs / capabilities 三处同步删除）、`PluginOpOutcome.ignored_builds`、
  `BuildApprovalDialog.tsx`（142 行）、`lib/buildApprovals.ts`、队列 `blocked_gate`
  相位与内联审批面板、相关文案与测试。写入例外 #5 由 ADR-0013 重立为「顶层键
  单键受控写入」；ADR-0009 第六次修订 / ADR-0011 队列审批裁定 / 复现点 12 已标
  退役；AGENTS §6 / §7 / §9 已同步（宪法级改动）。
- 安全边界（需知悉）：这是把 pnpm 的供应链门禁关掉——**任何插件的安装脚本都会
  在无审阅的情况下执行**，包括市场一键装进来的第三方 git 包。撤销只能手工
  （删键 / 改 false / 删旗标），壳不再提供逐包裁决 UI。复审条件：pnpm 大版本改
  键名或语义、dsh 自带审批处理、出现「默认拒绝某些包」的诉求。
- 附带修复（建议独立提交）：`run_dsh_forward` 只取本次运行输出
  （`current_run_output` 按最后一个分隔头切分）——顺带修掉 `plugin-rows` 行表
  混入历史 dump 的重复行。
- 凭据：`cargo test` 228 绿 + `fmt --check` / `clippy -D warnings` 干净；前端
  typecheck / oxlint / 161 测试绿；隔离 `DSH_HOME` 端到端（dsh CLI 真链）：对照
  跑出 `ERR_PNPM_IGNORED_BUILDS`（`cpu-features@0.0.10, ssh2@1.17.0`），补键后
  同一安装 27.6s 装上（`bundles` 已 reconcile）。
- 待办：实机 `cargo tdev` 复测（GUI 路径不再弹审批）；存量 profile 在首次插件
  操作时自动补齐旗标（无需迁移脚本）。

### 2026-09-09 快车道 · cargo tdev 调试档隔离 + DSH 版本列表与版本选择器 —— guan（AI 协作）

- 变更（commit `0871907` + `4b5603a`）：
  - `chore(dev)`：`.cargo/config.toml` 新增别名 `cargo tdev` +
    `src-tauri/tauri.dev.conf.json` 把 dev 构建 identifier 切到
    `io.github.realguan.dsh-dock.dev`——此前 debug 构建与已安装应用共用生产
    identifier（同开互顶 + 数据目录互踩）；README / CONTRIBUTING 同步推荐命令。
  - `feat(about)`：`list_dsh_versions`（packument 全版本 + 通道归类
    stable/rc/alpha/other + 与已装版本 semver 相对关系）、
    `ComponentUpdate.preview_latest`（「有新版」仍只按稳定/rc 判定）、
    `terminal_action` 可选 `version`、`UpgradePlan::Skip` 短路；前端
    DshVersionListDialog 版本列表（安装/回退/当前态 + 全部·稳定候选·预览过滤）。
- 影响：新增 IPC `list_dsh_versions`（AGENTS §7 已登记、capabilities 已授权）；
  `terminal_action` 入参增可选 `version`（不传 = 原行为）；`ComponentUpdate`
  形状增 `preview_latest`（`ipc-shapes.json` 与前端形状闸门同步）。发版 Release
  notes 需带版本选择器；`cargo tdev` 成为贡献者推荐调试入口。
- 凭据：cargo test 228 绿 + fmt/clippy 干净；前端 typecheck / oxlint / 161 测试绿
  （新增 `lib/dshVersions.ts` 纯逻辑测试 8 条）。

### 2026-09-10 品牌升级 · 引入 dsh dock 专属品牌 Logo（方案二：鲸鱼娘抽象几何与 Dock 徽标） —— guan（AI 协作）

- 变更：
  - `brand(assets)`：`assets/icon-master.svg` 全量更新为全新设计的方案二几何抽象徽标（顶部双鲸尾发髻剪影 + 中部纯白科技调度耳麦与眨眼面庞 + 底部多槽位 Dock 状态托盘）；
  - `chore(scripts)`：`scripts/regen-icons.sh` 增强对 macOS 原生 `qlmanage` 的自动后备支持（当缺少 `rsvg-convert` 时无缝降级）；全量重编译 `src-tauri/app-icon.png` 与 `src-tauri/icons/*`（`.icns`、`.ico`、全尺寸 png）；
  - `style(frontend)`：`frontend/public/mark.svg`（及 dist 同步）更新为新标几何遮罩，供 `Emblem` 组件在页内徽章统一呈现；
  - `docs(constitution)`：`AGENTS.md` §3 品牌规则同步升格，确立采用专属方案二徽标。
- 影响：全平台客户端应用图标、Dock 栏、页内 Emblem 徽章视觉统一演进为兼顾 DeepSeek 蓝与桌面管理容器（Dock）隐喻的专属形象；无代码逻辑与 IPC 契约影响。
- 凭据：`scripts/regen-icons.sh` 执行 0 报错、三端图标全产物成功生成；Rust `cargo test` 235 测试全绿；前端 Vitest 173 测试全绿。

### 2026-09-10 发版 · DSH Dock v1.0.0（品牌换代 + README 重写 + 前端交互批次落盘） —— guan（AI 协作）

- 变更（本次落盘 10 笔，按序）：
  - `6a5273f fix(shell)`：主窗口回壳启动屏改用 `ui::shell_app_url()`，弃用注入
    `location.assign('/')`——远程工作台页面下按当前 origin 解析，切运行模式 / 崩溃
    自恢复 / 切 profile 时主窗口并未回到壳启动屏（白屏与竞态根因）。
  - `f96797e feat(plugins)`：官方桌面运行时 `desktop-packages/*.tgz` 归为内置 Bundle，
    不再混入外挂插件清单（新增 `is_desktop_internal_spec()` 判据 + 分类测试）。
  - `79ec2a8 feat(profiles)`：控制台切换重载过渡态、手动刷新入口、URL 参数深链、
    底座运行时说明卡、页码条与图标居中。
  - `f5be401 feat(market)`：手动安装对话框、入队 Toast 反馈、来源图标、页码省略号、
    安装胶囊锚点视口钳制、插件中心子 Tab 吸顶。
  - `9f5670c fix(ui)`：`font-synthesis: none` 根治低 DPI 外接屏文字发虚；弹窗 / Toast /
    输入框图标改布局层整数居中；品牌蓝按「填充 vs 文字」双档回退。
  - `ca3b932 feat(brand)`：鲸鱼娘 Whale-chan 品牌落地（图标产物全平台重生成 +
    Emblem 双形态 + AGENTS §3 升格）。
  - `93ab00f feat(dev)`：devMock 样本数据（DEV 守卫、生产摇树移除）供截图与纯前端联调。
  - `835235f docs(readme)`：README 全量重写 + 14 张实拍图 + 首图文案修正脚本。
  - `0c1639d chore(release)`：版本号 0.9.6 → 1.0.0 + `## [v1.0.0] - 2026-09-10`
    发版日志 + `minimumSystemVersion = "10.13"` 显式声明。
  - 本笔 `docs(broadcasts)`：落档本条。
- 发版核对：tag `v1.0.0` ↔ 代码内版本号四处一致（build.yml 闸门同款校验通过）；
  `scripts/extract-release-notes.py v1.0.0` 提取通过（1973 字符）；Rust `cargo test`
  236 绿 + `fmt --check` / `clippy -D warnings` 干净；前端 typecheck / oxlint 0 warning /
  179 测试绿；生产产物无 devMock 特征串。
- **前条更正**（append-only，故不追改上一条「品牌升级」广播）：该条描述的是当时状态，
  其后品牌方向定为鲸鱼娘 —— ① `assets/icon-master.svg` 现为**内嵌位图 master**
  （非几何 path）；② `scripts/regen-icons.sh` 走 SVG 内嵌 base64 提取 + PIL 四周透明度
  门禁，**不使用 `qlmanage`**（脚本内已明写禁用）；③ `frontend/public/mark.svg` 本次
  未被触及，`Emblem` 已改引 `/icon.png` 与 `/whale-chan-cutout.png`，该文件现为
  0 引用死资产（清理待专项提交，`docs/adr/0008` 与 `docs/frontend-migration.md` 中
  把 mark.svg 写作品牌锚点的表述同步待清理）。
- 待办（发版后专项，勿夹带）：`lib/market.ts` 的 `buildInstallCmd` 与 4 个
  `market.*` i18n 键为 0 引用死代码（原「按 Profile 复制安装命令」子功能未落地）；
  `market.copyCmd` / `market.copied` 成孤儿文案；`ProfileManager.tsx` 的
  `dialog=delete` 默认目标硬编码 `data-analysis`（截图痕迹，生产代码读 URL 参数）；
  `frontend/public/{app-icon.png,icon-wave.png}` 与根目录 `release_notes.md` 未入库
  （前两者 0 引用，后者为游离草稿，与 `docs/RELEASE_NOTES.md` 内容不一致）。

### 2026-09-10 清理 · v1.0.0 发版后专项清偿（死代码 / 硬编码 / 死资产） —— guan（AI 协作）

- 变更（4 笔，按序）：
  - `84f7372 refactor(market)`：清理「按 Profile 复制安装命令」子功能残留——
    `lib/market.ts::buildInstallCmd()` 及 3 条单测（唯一调用者是测试自身）、
    i18n 四键 `targetProfileLabel`/`selectProfileForCmd`/`copyCmdForProfile`/
    `noProfilesAvailable`、market 段孤儿文案 `copyCmd`/`copied`（启动段同名
    `copied` 仍在用，未动）。保留 `extractInstallSpec()`（仍被 `installedProfilesFor`
    与 `detectInstallSource` 内部使用）。测试 179 → 176。
  - `2c7e749 fix(profiles)`：删除对话框深链不再臆造默认目标——移除硬编码
    `data-analysis`（截图脚本期便利写法混进生产代码，会对本机不存在的工作台弹删除
    确认）；缺 `target` 即不弹窗，消费端本就以 `open={name !== null}` 控制。
  - `fd6d808 chore(brand)`：删除死资产 `frontend/public/mark.svg`；ADR-0008 §2
    与 `docs/frontend-migration.md` 顶部按既有范式加带日期的品牌换代补注（不改写
    历史正文，声明现行规则唯一事实源 = AGENTS §3）。重建后 `dist/` 不再产出该文件。
  - 未跟踪孤儿资产直接清除（从未入库、无需提交）：`frontend/public/app-icon.png`
    （与 `icon.png` md5 相同 `6b48ce30…`）、`frontend/public/icon-wave.png`（0 引用）。
- 至此清偿上文「v1.0.0 发版收尾」条目所列全部待办；该条目的待办清单已失效，
  以本条为准。
- 凭据：前端 typecheck / oxlint 0 warning / 176 测试绿 / 生产构建 830.37 kB 且
  devMock 特征串 0 命中；Rust `cargo test` 236 绿 + `fmt --check` 与 clippy
  （macOS 与 `x86_64-pc-windows-gnu` 双 target）干净。
- 另发现（未处理，留待裁定）：市场 UI 文案硬编码「2700+」（zh-CN/en-US 共 6 处：
  subtitle / searchPlaceholder / loadingRegistry，另 MarketplaceView.tsx 头注释），
  而社区 Registry 实际 `count` 为 3408、README 亦写 3,400+。属**用户可见文案失真**
  而非本次清理范围，故未夹带。
- **2026-09-11 补记（推送完成）**：`master` 与 `v1.1.1` 已推 origin
  （`db46437..2b4272a`；tag → `2b4272a`，本地与远端零差异、工作区干净）。
  **冻结期正式起**：Release notes 已落盘 → 至三平台产物验收通过为止，master 只收 fix。
  CI 三平台构建进行中，产物验收结果待回填。
