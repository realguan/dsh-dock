# 项目级 AI 编码规范（dsh-dock / DSH Dock）

> 本文件是所有 AI 编码工具读取的核心上下文，与具体工具无关。
> 多人协作流程（分支 / review / 占用声明 / 发布）见 `docs/CONTRIBUTING.md`；
> 共享 Prompt 模板见 `docs/prompts/`；公共模块契约见 `docs/contracts/`。

## 0. 定位（必须理解再动手）

本仓库是**通用产品壳**（终端），不是打包装配工具。改动前先问：

- 这个逻辑属于「**壳**」（spawn / URL 解析 / 优雅停止 / WebView 导航 / 契约读取 /
  下载与进度 / 签名映射验证）→ 进本仓库；
- 还是属于「打包装配」（物化快照、版本 pin、插件集成、本地源扫描、任务台）→ 属于
  **装配方**（外部打包工具，经 `product.manifest.json` 契约与壳对接），本仓库不承接；
- 还是属于「产品数据」（某个具体工作台叫什么、装什么插件）→ 属于快照/构建期身份，不写死进壳。

**壳是通用机制，产品是数据**：壳不得感知任何具体产品身份；运行时身份只经
`product.manifest.json`（docs/contract.md）进入，构建期身份只经 `render-product.sh` 注入。

## 1. 项目概述与技术栈

dsh（@deepseek-ai/dsh）的桌面终端：极小 Tauri v2 壳，把 dsh 工作台以独立、可安装、
跨平台桌面应用呈现。在线极简档安装包不内置 Node/dsh，首启自动补齐；宿主解析链
system → bundle → download；Node 版本经签名映射包热升级，不发新壳。

| 层 | 选型 | 版本锚点 |
|:---|:---|:---|
| 桌面框架 | Tauri v2（Rust 后端 + 系统 WebView） | tauri crate `2`；**tauri-cli 必须同代 2.11.x**（CI env `TAURI_CLI_VERSION=2.11.4`，bundler 与 crate 不同代会产物补丁失败） |
| Rust | edition 2021 | `rust-version = "1.77.2"`（Cargo.toml）；CI 用 stable toolchain |
| 序列化 | serde / serde_json | `1` |
| HTTP 客户端 | ureq（阻塞式，仅后台线程） | `2` |
| 签名验证 | ed25519-dalek（仅 verify 路径） | `2` |
| 校验和 | sha2（Node 二进制 SHA-256） | `0.10` |
| 压缩/归档 | flate2 / tar / zip | zip **pinned `=4.2.0`**（升级需专项验证） |
| 信号 | nix（unix only：SIGTERM/SIGKILL） | `0.29` |
| 外链打开 | open crate | `5` |
| 单实例 | tauri-plugin-single-instance（Builder 链最先注册） | `2` |
| 自更新 | tauri-plugin-updater（endpoint = GitHub Releases `latest.json`） | `2` |
| 日志 | tracing + tracing-subscriber | `0.1` / `0.3` |
| 错误处理 | anyhow（壳无 services 分层，不用 AppError 枚举） | `1` |
| 前端 | **静态 HTML/CSS/JS（`ui/`），零构建器、零框架、零依赖** | — |
| 数据库 | **无**（禁止引入） | — |
| 构建/发布 | GitHub Actions 三平台矩阵 → tag `v*` 出 Release + updater 元数据 | `.github/workflows/build.yml` |

- 开发环境只需 Rust toolchain + 平台 Tauri 前置（macOS Xcode CLT / Windows WebView2 /
  Linux WebKitGTK）；**壳前端免构建，日常开发不需要 node/npm**。仅 node-map 发布流程
  （`node-map/scripts/sign.mjs`）需要 node。
- Lint/Format：当前**无** rustfmt.toml / clippy.toml / rust-toolchain 文件
  （`cargo fmt` 走默认配置）。`[待补充]` 建议提交一份锁定 edition 风格的
  `rustfmt.toml` + CI 加 `cargo fmt --check` / `cargo clippy -D warnings`，
  落地前以「不引入全仓格式化 diff」为先。

## 2. 目录结构约定

```
dsh-dock/
├── AGENTS.md               # 本文件：AI 编码宪法（改动规则见 §10）
├── docs/
│   ├── contract.md         # 壳 ↔ 装配方运行时契约（product.manifest.json，format=1/2）
│   ├── CONTRIBUTING.md     # 协作规范：分支/review/占用声明/发布协议
│   ├── prompts/            # 团队共享 Prompt 模板库
│   ├── contracts/          # 公共模块契约管理（哪些模块要契约、怎么改）
│   ├── adr/                # 架构决策记录（本仓库；TEMPLATE.md + ADR-0001~0007，见 §9）
│   ├── executor.md         # 执行环境抽象设计（local/wsl，ssh 预留）
│   ├── wsl-verification.md # WSL Windows 实机验证清单
│   └── macos-signing.md    # macOS 签名与公证手册
├── ui/                     # 壳自带页面（静态，无框架无构建器）
│   ├── index.html          # 启动序列（时间线 + 下载进度 + 错误卡）
│   ├── mode.html           # 首次运行环境选择（local/wsl，Windows-only 分支）
│   ├── selector.html       # 工作台选择器（system 档多 webUi profile）
│   ├── about.html          # 关于面板（版本 + 检查/升级）
│   └── assets/             # app.css + 官方标（mark.svg / dsh-logo.svg）
├── src-tauri/
│   ├── src/
│   │   ├── main.rs         # 入口（6 行，勿动）
│   │   ├── lib.rs          # run()：装配 Builder；child_cmd()；IPC 命令；菜单/托盘；注入脚本
│   │   ├── executor.rs     # 执行环境抽象：local / wsl（ssh 预留），壳只认识 Executor
│   │   ├── manifest.rs     # product.manifest.json 契约解析（format=1/2）
│   │   ├── resolve.rs      # 宿主解析链（system → bundle → download）
│   │   ├── updates.rs      # 唯一网络面：版本检测 / Node 下载 / dsh 安装 / 签名映射
│   │   ├── shell.rs        # spawn / URL 解析 / 优雅停止 / wait_for_ready
│   │   ├── settings.rs     # settings.json（仅 defaultMode 一字段，原子写）
│   │   └── updater.rs      # tauri-plugin-updater 桥接
│   ├── build.rs            # AppManifest commands → 自动生成 allow-* 权限
│   ├── capabilities/       # default.json：remote 页面调用命令的显式授权
│   ├── resources/          # product.manifest.json（可选 dsh-snapshot/ 离线档，gitignore）
│   └── icons/              # 生成产物（勿手改，见品牌规则）
├── node-map/               # @dsh-dock/node-map：签名的 Node 版本映射包（npm 发布物）
├── scripts/
│   ├── regen-icons.sh      # 图标重生成（rsvg-convert → cargo tauri icon）
│   └── render-product.sh   # 打包期身份注入（仅装配方/CI 使用，运行时不执行）
├── assets/icon-master.svg  # 图标形状源
├── sample/                 # 示例 product.manifest.json（装配方参考，非运行时）
└── .github/workflows/build.yml  # 三平台矩阵构建 + tag 发版
```

职责红线：`ui/` 不出现任何 `package.json`/构建器痕迹；`src-tauri/resources/dsh-snapshot/`
永不入库；`node-map-private.key` 永不入库（.gitignore 双防）。

## 3. 品牌规则

- 桌面图标 / 页内徽章一律用 dsh 官方标；**禁止手绘或自造占位 logo**。
- 改图标 = 改 `assets/icon-master.svg`（官方 path 合成 + 深色圆角底）→
  跑 `scripts/regen-icons.sh` 整体重生成（rsvg-convert → cargo tauri icon）；
  `src-tauri/icons/` 是生成产物，**禁止直接手改**。
- 页内徽章（index/mode/selector/about 四页）统一为 `.emblem` 组件：
  `ui/assets/mark.svg`（形状源）+ CSS mask 上白色——**不允许在页面里内联第二份
  鲸鱼 path 或第二种颜色**。
- 官方原始 SVG 溯源于 `ui/assets/dsh-logo.svg`；未获官方新版本前不得偏离该几何。

## 4. 代码规范

### 4.1 Rust ✅ 必须

- 跨平台语义显式：优雅停止按平台分叉（unix SIGTERM→SIGKILL，Windows kill），
  用 `#[cfg]` 不用运行时猜。
- **Windows 子进程一律经 `crate::child_cmd` 构造**（lib.rs）：防黑色终端窗口弹窗
  + 解决 `.cmd`/`.bat` 直接 spawn 必失败（CreateProcess 不认批处理）。新增 spawn
  点禁止裸 `Command::new`；壳自身 `windows_subsystem = "windows"` 无条件生效
  （debug/release 均无控制台）。
- 子进程 stdout/stderr 进数据目录日志文件（`Read + try_wait` 轮询），不阻塞 UI 线程。
- 快照零部件缺失 → **就地错误页 + 可行动文案**（ADR-0004 A6），绝不静默降级。
- `product.manifest.json` 契约改动：先改 `docs/contract.md` → 升 `MANIFEST_FORMAT`
  → 打包侧同步（缺一不可，详见 `docs/contracts/`）。
- URL 解析只认 `http://` / `https://`（拒绝 `file://`、`data:`），带回归测试。
- 新增函数优先给单元测试（`shell.rs` / `manifest.rs` / `resolve.rs` 已有先例）。
- 命名与结构沿用现状：模块级中文 doc 注释说明「为什么」；裁定性注释带日期
  （如 `// 2026-08-25 裁定：…`）；公开函数一行 `///` 说明用途与失败语义。

### 4.2 Rust ❌ 禁止

- `unwrap()` 于可能返回 `Err` 的逻辑结果（仅限 `expect` 于「构建期不可变不变量」，
  如 main 窗口必存在）；`std::sync::Mutex::lock()` 中毒属 panic-on-poison 语义，
  用 `expect` 标注不变量即可，不视为违规。
- 阻塞主线程的同步等待；dsh 就绪用后台线程轮询 + 超时上限。
- 把具体产品（名称/图标/插件/凭据）硬编码进壳。
- 引入任何前端构建链 / IPC 总线 / 数据库 / 领域服务——壳要保持薄。
- 直接依赖宿主 pnpm store 或触网取依赖——快照必须自包含（ADR-0004 硬指标）。
- `println!` / `dbg!` 入库——日志一律 tracing。
- 非 `updates.rs` 模块触网（网络面白名单见 §7）。

### 4.3 UI（壳自带前端）

- 纯静态 HTML/CSS/原生 JS；**禁止引入框架、构建器、外部 CDN 依赖、npm 包**。
- 页面间共享样式走 `ui/assets/app.css`；徽章走 `.emblem`（§3），不复制粘贴 SVG path。
- 调用壳命令：`window.__TAURI__.core.invoke(...)`（tauri.conf 已开 withGlobalTauri），
  返回 Promise **必须 `.catch`**（try/catch 捕不到 rejection）；消费事件用
  `window.__TAURI__.event.listen`。
- 新增 IPC 命令的前置授权链见 §7「三处同步」；漏任一处 remote 页面调用静默失败。

## 5. 测试要求

- **框架**：Rust 内置 `#[cfg(test)] mod tests` + `#[test]`（现状分布于
  updates / resolve / executor / shell / lib / manifest / settings 七模块，
  具体数见各模块 `#[test]`；不在此固定总数，以免漂移）。
  **不引入额外测试依赖**（Cargo.toml 无 dev-dependencies，保持）。
- **运行**：`cd src-tauri && cargo test`——任何 Rust 改动合入前必须全绿；
  CI 三平台矩阵会再跑一遍。
- **Mock 策略**：不引 mock 框架。测**纯函数**（URL 解析、契约字段校验、路径推导、
  镜像链排序、签名格式校验），输入用内联 fixture 字符串 / `tempdir` 式临时目录；
  真实网络与真实 WSL/实机行为不在单测覆盖，走对应验证清单
  （WSL 见 `docs/wsl-verification.md`）。
- **必须带测试的场景**：① URL/导航解析类改动 → 回归测试（含恶意输入反例）；
  ② `manifest.rs` 契约字段 → 正反例各一（合法 v1/v2 + 缺字段/错 format 拒绝）；
  ③ bug 修复 → 复现该 bug 的测试先行；④ 跨平台分叉逻辑 → 至少覆盖编译目标语义。
- `[待补充]` 覆盖率目标与工具（建议 `cargo-llvm-cov` 接入 CI，先出基线再定阈值）。
- `[待补充]` `updater.rs` 当前无测试（tauri-plugin-updater 桥接层，依赖运行时环境）；
  补测前至少保证改动经 `cargo tauri build` + 手动「检查更新」路径验证。

## 6. 存储与生命周期

- 无状态库：本仓库**不持久化任何核心态**。运行期只写：数据目录的 `dsh-shell.log`
  （排查用）。
- **最小持久化例外（2026-08-25 登记）**：`<app_data>/settings.json`、仅 `defaultMode`
  一个字段（settings.rs：首次打开可选运行环境 + 设置默认打开方式；菜单/托盘
  「打开方式」切换即写默认）。原子写（tmp+rename）、损坏回退默认。其余核心态一律不落盘。
- **同生命周期**：壳与 dsh 严格 1:1；退出/崩溃都要把子进程收干净，不留孤儿。

## 7. IPC 与网络面（最小面例外册）

- IPC 命令（已登记）：`choose_profile`（选择器）、`terminal_action`（错误卡
  动作 retry/upgrade/upgrade_only）、`get_update_status`（版本状态即读）、
  `check_updates`（手动后台检测）、`get_client_update`（客户端自更新状态即读）、
  `client_update_check`（检查客户端更新，结果经 `app:update` 回推）、
  `client_update_apply`（下载并安装客户端更新 → 重启）、`open_about`（开
  「关于与更新」小窗，前端顶栏按钮）、`open_external`（白名单外链 → 系统
  浏览器）、`open_workbench_in_browser`（当前工作台 → 系统浏览器）、
  `get_workbench_url`（读当前工作台地址）、`boot_in_wsl`（「在 WSL 中打开」：
  切换并写默认；零配置，Windows-only 渲染/调用，非 Windows 防御性拒绝）、
  `choose_mode`（首次运行环境选择落地：写默认（可选）→ 按所选模式启动；
  mode.html → index.html?mode=…；WSL 分支仅 Windows 接受）。前端经
  `window.__TAURI__.core.invoke` 调用、事件经 `window.__TAURI__.event.listen`
  消费——**注册例外**；新命令必须先在本节登记。
- **新增 IPC 命令三处同步（2026-08-25 裁定）**：Tauri 2.11 规定 remote origin
  （dsh 页面 http://127.0.0.1）调用自定义命令会被 ACL 拒绝，除非 capability 显式引用
  `allow-<command>` 权限。链路 = `build.rs` 的 `AppManifest::new().commands([...])`
  （构建期自动生成 allow-\*/deny-\*）+ `capabilities/default.json` permissions 逐个
  引用 + `lib.rs` 命令实现。**漏任何一处，remote 页面调用即静默失败。**
- **WSL 客体内安装 = 网络面例外**：WSL 探测到「有 node 缺 dsh」时，在客体内安装 dsh。
  网络动作发生在 **WSL 发行版内**（Windows 侧壳不触网）；镜像配置由用户客体内 npm
  决定（不注入镜像参数）；缺 node（`NODE_MISSING`）不自动装 Node（用户主权），
  只给可行动提示。脚本模板 / 日志回传细节见 [ADR-0004](docs/adr/0004-wsl-guest-dsh-install.md)。
- **外链策略**：主窗口由 setup 内 `create_main_window` 创建（tauri.conf.json 不静态
  定义 windows——只有代码创建才能挂处理器）。dsh 超链接 / 新窗口统一转系统默认浏览器；
  非白名单 http(s) 拦截，壳不成为任意跳板；新外链域 = 在 `EXTERNAL_URL_HOSTS` 登记。
  导航 / 新窗口 / 兜底实现见 [ADR-0003](docs/adr/0003-external-link-and-navigation.md)。
- **WebView 长会话内存**：dsh web 前端无虚拟化，WebKit 视口外资源回收弱，长会话
  WebContent 膨胀。壳经 initialization_script 注入 CSS 缓解（content-visibility + 动态豁免 +
  能力探测降级），不动用户本地状态。技法与禁手清单见 [ADR-0002](docs/adr/0002-webview-memory-policy.md)。
- 事件协议：`boot:step` / `boot:error`（启动遥测）+ `boot:update`
  （三维度版本状态 `{dsh, client, node:{version, origin}|null}`）+
  `boot:progress`（下载进度 `{kind:"node", current, total|null}`；Rust 侧节流
  ≥100ms，updates 经回调上抛、lib.rs 桥接为事件——updates 保持零 tauri 依赖）。
- 更新常驻入口：**macOS = 应用菜单**，**非 macOS = 系统托盘**（窗口菜单一律
  `#[cfg(target_os = "macos")]` 门控，事件同一 `on_menu_event` 分发）。另有前端顶栏
  「关于」按钮（`open_about`）。`upgrade_only` 不打断会话（下次启动生效）；about 窗口
  label 须在 capabilities windows 列表。平台分叉与托盘修订史见 [ADR-0007](docs/adr/0007-update-entry-menu-vs-tray.md)。
- **dsh 就绪等待**：`shell::wait_for_ready` 取代死等——进程存活感知（退出即败 /
  日志无进展判卡死 / 超时先 teardown 再报错，重试不残留孤儿）。阈值与实现见
  [ADR-0001](docs/adr/0001-ready-wait-process-liveness.md)。
- **唯一网络面 = `updates.rs`**：包元数据 / dsh 安装、Node 二进制、Node 版本映射包
  `@dsh-dock/node-map`（ed25519 验签后才采纳，失败回退内置基线 fail-closed）、客户端
  自更新源均在 updates.rs 内。其余模块不得触网（Windows 安装器 WebView2 在线引导属
  打包配置，不属壳运行时网络面）；非 updates.rs 的网络需求先在本节登记再写。网络动作
  一律后台线程。镜像链 / 下载双超时 / 自更新源细节见 [ADR-0006](docs/adr/0006-network-surface-and-mirror-chain.md)。
- **pnpm 全局安装需显式注入 global-bin-dir**：GUI 子进程不加载 shell rc，pnpm 10 缺
  `global-bin-dir` 会失败 → 回退 npm。注入与回退细节见 [ADR-0005](docs/adr/0005-pnpm-global-bin-dir.md)。

## 8. AI 交互约束

无论使用哪种 AI 编码工具，操作者对以下约束负责：

1. **增量生成**：一次会话只做一个明确意图（一个功能 / 一个修复 / 一次重构）；
   「顺便把 X 也改了」= 停下，拆成下一次。
2. **禁止一次性大改**：不做跨模块批量改动、不做全仓格式化/批量重命名/
   `clippy --fix` 扫荡（冲突协议见 docs/CONTRIBUTING.md §占用声明）。
   AI 提出超范围改动时一律拒绝，记入计划另行开工。
3. **必须附带测试**：行为改动必须有对应测试或明确验证记录
   （单测 / 实机清单条目 / 手动验证说明写入 PR）。「没测过」的代码不合入。
4. **先读后写**：动任何文件前，先读该文件现状与本文件相关章节；
   AI 声称「我记得这里应该……」时，一律以仓库现状为准，不猜、不静默扩权。
5. **收尾三件事**：`cargo test` 绿 → 人肉读 `git diff` 确认无越界 →
   按 CONTRIBUTING 规范提交并在公共频道广播完成通知。
6. **不确定就问**：逻辑归属（壳 vs 打包侧）、契约影响面、是否踩本文件红线——
   问人，不猜。

## 9. 关键决策记录索引

| 记录 | 位置 | 说明 |
|:---|:---|:---|
| ADR-0004 独立桌面打包 | 姊妹仓库 `dsh-launcher/docs/adr/0004-standalone-desktop-package.md` | 本仓库立项依据：内嵌 WebView + 冻结快照 + 壳/快照解耦 |
| ADR-0005 桌面终端定位 | 姊妹仓库 `dsh-launcher/docs/adr/0005-desktop-terminal.md` | 产物语义修正：dsh 的桌面终端（宿主解析链），非纯离线分发 |
| **本仓库 ADR**（`docs/adr/`，编号自 0001 起） | | |
| ADR-0001 dsh 就绪等待语义 | `docs/adr/0001-ready-wait-process-liveness.md` | 进程存活感知取代死等（退出即败 / 卡死判定 / teardown） |
| ADR-0002 WebView 内存策略 | `docs/adr/0002-webview-memory-policy.md` | content-visibility 注入缓解长会话膨胀 |
| ADR-0003 外链与导航策略 | `docs/adr/0003-external-link-and-navigation.md` | 系统浏览器兜底 + 白名单拦截 + 三层覆盖 |
| ADR-0004 WSL 客体内 dsh 安装 | `docs/adr/0004-wsl-guest-dsh-install.md` | 壳不触网，安装进 WSL 发行版 |
| ADR-0005 pnpm global-bin-dir 注入 | `docs/adr/0005-pnpm-global-bin-dir.md` | GUI 子进程无 rc 的 pnpm 10 兼容 + npm 回退 |
| ADR-0006 唯一网络面与镜像链 | `docs/adr/0006-network-surface-and-mirror-chain.md` | 四路镜像链 + 下载双超时 + 自更新源 |
| ADR-0007 更新常驻入口 | `docs/adr/0007-update-entry-menu-vs-tray.md` | macOS 菜单 vs 非 macOS 托盘（含托盘修订史） |
| 本仓库 ADR 规范 | `docs/adr/TEMPLATE.md` | 影响契约/架构/安全边界的决策必须立 ADR |
| 内联裁定 | 本文件各节 | 例行裁定（日期 + 结论 + 一句理由）可留本文件；升格为 ADR 的时机 = 决策影响多个模块、需要否决过的备选方案可追溯。§7 原有带日期裁定已按此判据批量升格为 ADR-0001~0007 |

ADR 流程：先立 ADR → 相关方确认 → 再动代码（上游 ADR-0004 即此先例）。

## 10. 试验协议（AI 协作）

- 修改运行时契约或快照布局前，先对照 `docs/contract.md` 确认两侧同步方案。
- 不确定某逻辑归属（壳 vs 打包侧）时，先问清楚再动手，不猜、不静默扩权。
- 多人协同（Git 工作流 / 文件占用声明 / 沟通与发布协议）见 `docs/CONTRIBUTING.md`——
  本文件是共享宪法，同一时间仅一人可改，改前须在公共频道知会所有活跃协作者，
  改后贴出 diff 摘要。
