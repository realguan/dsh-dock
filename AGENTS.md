# 项目级 AI 编码规范（dsh-desktop-shell）

## 定位（必须理解再动手）

本仓库是 ADR-0004 的**通用产品壳**，**不是启动器**的不完整副本。改动前先问：

- 这个逻辑属于「**壳**」（spawn / URL 解析 / 优雅停止 / WebView 导航 / 契约读取）→ 进本仓库；
- 还是属于「打包装配」（物化快照、版本 pin、插件集成、本地源扫描、任务台）→ 属于
  **启动器 packaging 服务**（dsh-launcher），本仓库不承接；
- 还是属于「产品数据」（某个具体工作台叫什么、装什么插件）→ 属于快照/构建期身份，不写死进壳。

**壳是通用机制，产品是数据**：壳不得感知任何具体产品身份；运行时身份只经
`product.manifest.json`（docs/contract.md）进入，构建期身份只经 `render-product.sh` 注入。

## 技术栈

| 层级 | 选型 |
|:---|:---|
| 框架 | Tauri v2（Rust 后端 + 系统 WebView） |
| 壳自带前端 | 静态 HTML/CSS/JS（`ui/`），**禁止引入构建器/框架/依赖** |
| 错误处理 | anyhow（壳无 IPC、无 services 分层，用不上 AppError 枚举） |
| Rust 日志 | tracing（禁止 println!） |
| 品牌 | dsh 官方标（来源：dsh-web-frontend `favicon.svg`；白标 = 官方深色模式渲染） |

## 品牌规则

- 桌面图标 / 加载页 logo 一律用 dsh 官方标；**禁止手绘或自造占位 logo**。
- 改图标 = 改 `assets/icon-master.svg`（官方 path 合成 + 深色圆角底）→
  跑 `scripts/regen-icons.sh` 整体重生成（rsvg-convert → cargo tauri icon）。
- 官方原始 SVG 溯源于 `ui/assets/dsh-logo.svg`；未获官方新版本前不得偏离该几何。

## Rust 规则

### ✅ 必须
- 跨平台语义显式：优雅停止按平台分叉（unix SIGTERM→SIGKILL，Windows kill），用 `#[cfg]` 不用运行时猜。
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
  `check_updates`（手动后台检测）。前端经 `window.__TAURI__.core.invoke` 调用
  （tauri.conf 已开 withGlobalTauri）、事件经 `window.__TAURI__.event.listen`
  消费——**注册例外**；新命令必须先在 AGENTS 登记。
- 事件协议：`boot:step` / `boot:error`（启动遥测，index/selector 两页）+
  `boot:update`（更新检测结果，版本行芯片）。macOS 菜单栏托盘（tauri tray-icon
  feature）为版本/升级的常驻落点：菜单项 check/upgrade/quit；`upgrade_only`
  不打断进行中的会话（升级下次启动生效）。
- 启动可视化协议：`boot:step {step, state, detail}`（0-4：环境检测/宿主解析/
  启动 dsh/等待就绪/进入工作台）、`boot:error {title, detail, suggestion, actions, log}`。
  前端状态推演：收到 N 步 running 时 N 之前未定步骤自动 done（防事件竞态）。
- 唯一网络面：`updates.rs`（npm registry 镜像链 / nodejs.org）。其余模块不得触网。
  网络动作一律后台线程 + 超时；非 updates.rs 的网络需求先登记再写。

## 试验协议（AI 协作）

- 修改运行时契约或快照布局前，先对照 `docs/contract.md` 确认两侧同步方案。
- 不确定某逻辑归属（壳 vs 打包侧）时，先问清楚再动手，不猜、不静默扩权。
