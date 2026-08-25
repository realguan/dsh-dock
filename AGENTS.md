# 项目级 AI 编码规范（dsh-dock / DSH Dock）

## 定位（必须理解再动手）

本仓库是**通用产品壳**（终端），不是打包装配工具。改动前先问：

- 这个逻辑属于「**壳**」（spawn / URL 解析 / 优雅停止 / WebView 导航 / 契约读取 /
  下载与进度 / 签名映射验证）→ 进本仓库；
- 还是属于「打包装配」（物化快照、版本 pin、插件集成、本地源扫描、任务台）→ 属于
  **装配方**（外部打包工具，经 `product.manifest.json` 契约与壳对接），本仓库不承接；
- 还是属于「产品数据」（某个具体工作台叫什么、装什么插件）→ 属于快照/构建期身份，不写死进壳。

**壳是通用机制，产品是数据**：壳不得感知任何具体产品身份；运行时身份只经
`product.manifest.json`（docs/contract.md）进入，构建期身份只经 `render-product.sh` 注入。

## 技术栈

| 层级 | 选型 |
|:---|:---|
| 框架 | Tauri v2（Rust 后端 + 系统 WebView） |
| 单实例 | `tauri-plugin-single-instance`（OS 级原语锁，Builder 链最先注册；二次启动 = 唤起主窗口） |
| 壳自带前端 | 静态 HTML/CSS/JS（`ui/`），**禁止引入构建器/框架/依赖** |
| 签名验证 | ed25519-dalek（node-map 验签，仅 verify 路径） |
| 错误处理 | anyhow（壳无 IPC、无 services 分层，用不上 AppError 枚举） |
| Rust 日志 | tracing（禁止 println!） |
| 品牌 | dsh 官方标（来源：dsh-web-frontend `favicon.svg`；白标 = 官方深色模式渲染） |

## 品牌规则

- 桌面图标 / 页内徽章一律用 dsh 官方标；**禁止手绘或自造占位 logo**。
- 改图标 = 改 `assets/icon-master.svg`（官方 path 合成 + 深色圆角底）→
  跑 `scripts/regen-icons.sh` 整体重生成（rsvg-convert → cargo tauri icon）。
- 页内徽章（index/selector/about 三页）统一为 `.emblem` 组件：`ui/assets/mark.svg`
  （形状源）+ CSS mask 上白色——**不允许在页面里内联第二份鲸鱼 path 或第二种颜色**。
- 官方原始 SVG 溯源于 `ui/assets/dsh-logo.svg`；未获官方新版本前不得偏离该几何。

## Rust 规则

### ✅ 必须
- 跨平台语义显式：优雅停止按平台分叉（unix SIGTERM→SIGKILL，Windows kill），用 `#[cfg]` 不用运行时猜。
- **Windows 子进程一律经 `crate::child_cmd` 构造**（2026-08-24 裁定）：
  内部 = `CREATE_NO_WINDOW`（黑色终端窗口弹窗，含 dsh 本体常驻窗口）
  + `.cmd`/`.bat` 自动 `cmd /C` 包装（CreateProcess 不认批处理，pnpm.cmd
  直接 spawn 必失败）。新增 spawn 点禁止裸 `Command::new`；壳自身
  `windows_subsystem = "windows"` 无条件生效（debug/release 均无控制台）。
- 子进程 stdout/stderr 进数据目录日志文件（`Read + try_wait` 轮询），不阻塞 UI 线程。
- 快照零部件缺失 → **就地错误页 + 可行动文案**（ADR-0004 A6），绝不静默降级。
- `product.manifest.json` 契约改动：先改 `docs/contract.md` → 升 `MANIFEST_FORMAT` → 打包侧同步（缺一不可）。
- URL 解析只认 `http://` / `https://`（拒绝 `file://` 栈帧、`data:`），带回归测试。
- 新增函数优先给单元测试（`shell.rs` / `manifest.rs` 已有先例）。

### ❌ 禁止
- `unwrap()` 在库代码路径上（仅限 `expect` 于「构建期不可变不变量」，如 main 窗口必存在）。
- 阻塞主线程的同步等待；dsh 就绪用后台线程轮询 + 超时上限。
- 把具体产品（名称/图标/插件/凭据）硬编码进壳。
- 引入任何前端构建链 / IPC / 数据库 / 领域服务——壳要保持薄。
- 直接依赖宿主 pnpm store 或触网取依赖——快照必须自包含（ADR-0004 硬指标）。

## 存储与生命周期

- 无状态库：本仓库**不持久化任何核心态**。运行期只写：数据目录的 `dsh-shell.log`（排查用）。
- **同生命周期**：壳与 dsh 严格 1:1；退出/崩溃都要把子进程收干净，不留孤儿。

## IPC 与网络面（最小面例外册）

- IPC 命令（已登记）：`choose_profile`（选择器）、`terminal_action`（错误卡
  动作 retry/upgrade/upgrade_only）、`get_update_status`（版本状态即读）、
  `check_updates`（手动后台检测）、`get_client_update`（客户端自更新状态即读）、
  `client_update_check`（检查客户端更新，结果经 `app:update` 回推）、
  `client_update_apply`（下载并安装客户端更新 → 重启）、`open_about`（开
  「关于与更新」小窗，前端顶栏按钮）、`open_external`（白名单外链 → 系统
  浏览器）、`open_workbench_in_browser`（当前工作台 → 系统浏览器）、
  `get_workbench_url`（读当前工作台地址）、`boot_in_wsl`（「在 WSL 中打开」：
  停掉现有会话、重建 WSL2 会话并启动；零配置，非 Windows 报错卡）。前端经
  `window.__TAURI__.core.invoke` 调用（tauri.conf 已开 withGlobalTauri）、
  事件经 `window.__TAURI__.event.listen` 消费——**注册例外**；新命令必须
  先在 AGENTS 登记。
- **外链策略（2026-08-25 裁定）**：主窗口由 setup 内 `create_main_window` 创建
  （tauri.conf.json 不再静态定义 windows——只有代码创建才能挂处理器）。
  dsh Web UI 的超链接/新窗口在 WebView 里默认点不动，统一转系统默认浏览器
  （`open` crate：mac=/usr/bin/open、linux=xdg-open、win=ShellExecuteW）：
  ①`on_navigation`：壳页（tauri/about/data/blob）与回环 dsh（127.0.0.1/
  localhost/[::1]）放行；其余 http(s) 过白名单（`EXTERNAL_URL_HOSTS`）后转
  浏览器并拦截导航，非白名单直接拦（壳不成为任意跳板）；②`on_new_window`：
  一律 Deny，白名单内转浏览器；③initialization_script 兜底捕获跨源 `<a>` 点击
  走 `open_external`。新外链域 = 在 `EXTERNAL_URL_HOSTS` 登记。
- **remote 页面调用自定义命令必须显式授权（2026-08-25 外链修复裁定）**：
  Tauri 2.11 规定 remote origin（dsh 页面 http://127.0.0.1）调用应用自定义命令
  会被 ACL 拒绝，除非 capability 显式引用 `allow-<command>` 权限。链路 =
  `build.rs` 的 `AppManifest::new().commands([...])`（构建期自动生成
  `allow-*`/`deny-*` 权限）+ `capabilities/default.json` permissions 里逐个
  引用。**新增 IPC 命令三处同步**：lib.rs 命令 + build.rs commands 列表 +
  capabilities permissions（漏任何一处，remote 页面调用即静默失败）。
  前端 hook 的 `invoke` 返回 Promise：`try/catch` 捕不到 rejection，必须
  `.catch`；`preventDefault` 必须在确认 `__TAURI__` 可用之后（否则点了没反应）。
- 事件协议：`boot:step` / `boot:error`（启动遥测）+ `boot:update`
  （三维度版本状态 `{dsh: ComponentUpdate, client: ComponentUpdate, node: {version,
  origin: system|managed}|null}`；dsh/client 含 current/latest/newer/error）+
  `boot:progress`（下载进度 `{kind:"node", current, total|null}`；Rust 侧节流
  ≥100ms，updates 模块经回调上抛、lib.rs 桥接为事件——updates 保持零 tauri 依赖）。
- 更新常驻落点 = **macOS 应用菜单**（托盘已砍，2026-08-23 裁定）：
  根菜单 id `st`（状态行，禁用）/ `check`（检查更新…，⌘U）/ `upgrade`
  （升级到 X，新版才可用）/ `about`（关于，开 ui/about.html 小窗）；
  on_menu_event 对应 check→后台检测 / upgrade→upgrade_only→检测 / about→开窗。
  `upgrade_only` 不打断会话（升级下次启动生效）；about 窗口 label 须在
  capabilities windows 列表。
- **非 macOS 常驻更新入口 = 系统托盘**（2026-08-24 裁定，修订 08-23「托盘已砍」）：
  muda 菜单在 Windows/Linux 若挂在窗口上会渲染成窗口内菜单条（标题栏下多出
  「dsh-dock · 编辑」一排，丑），故窗口菜单一律 `#[cfg(target_os = "macos")]`
  门控；非 macOS 改走 `TrayIconBuilder::with_id("main")`（左键唤主窗、右键菜单
  id st/check/upgrade/about/quit，事件经 builder 级 on_menu_event 同一处理）。
  「编辑」子菜单仅 macOS（供 WebView 复制粘贴快捷键），Windows/Linux 无。
  另有前端顶栏「关于」按钮（`open_about` → about 小窗，含检查/升级芯片）。
- 启动可视化协议：`boot:step {step, state, detail}`（0-4：环境检测/宿主解析/
  启动 dsh/等待就绪/进入工作台）、`boot:error {title, detail, suggestion, actions, log}`。
  前端状态推演：收到 N 步 running 时 N 之前未定步骤自动 done（防事件竞态）。
- **dsh 就绪等待 = 进程存活感知（2026-08-24 裁定）**：`shell::wait_for_ready`
  取代死等——①硬上限 90s（Windows 冷启动被 Defender/Node 冷加载吃掉，20s 常不够）；
  ②dsh 进程中途退出 → 立即判败报错（不等满上限，真失败秒报）；
  ③进程活着但日志 20s 无进展 → 判卡死 `Stalled`。等待期间会话留在
  `ShellState.session`（executor 会话槽，短锁轮询，不阻塞退出处理器）；超时/退出
  先 teardown 旧会话再报错，重试不残留孤儿 dsh（壳与 dsh 严格同生命周期）。
- 唯一网络面：`updates.rs`，四路镜像链——包元数据/dsh 安装（registry.npmmirror →
  registry.npmjs）、Node 二进制（cdn.npmmirror.com/binaries/node → nodejs.org/dist）、
  Node 版本映射包 `@dsh-dock/node-map`（registry 镜像链拉 packument+tarball，
  ed25519 验签后才采纳，失败回退内置基线；见 node-map/README.md）、
  客户端自身更新源 `APP_RELEASE_FEED`（GitHub Releases latest API；常量为 None 时
  不触网不出检查入口，开源仓库就位后填入）。
  其余模块不得触网（Windows 安装器的 WebView2 在线引导属打包配置，不属壳运行时网络面）。
  网络动作一律后台线程；元数据用整体超时，大文件下载用「连接 + 单次读」双超时、不设
  整体上限（慢网络下 40MB 合法地超过一分钟）；非 updates.rs 的网络需求先登记再写。
- **pnpm 全局安装需显式注入 global-bin-dir**（2026-08-25 裁定）：GUI 子进程不加载
  shell rc，`PNPM_HOME` 环境变量对 pnpm 10 无效，缺配置时 `pnpm add -g` 报
  `ERR_PNPM_NO_GLOBAL_BIN_DIR` 失败→回退 npm（慢）。`install_global_dsh_pnpm`
  一律经 `pnpm_global_bin_dirs` 注入 `--config.global-bin-dir=<pnpm 所在目录>`
  （该目录天然在 PATH 里，满足 pnpm 校验）；`root -g` 同步注入。

## 试验协议（AI 协作）

- 修改运行时契约或快照布局前，先对照 `docs/contract.md` 确认两侧同步方案。
- 不确定某逻辑归属（壳 vs 打包侧）时，先问清楚再动手，不猜、不静默扩权。
