# 项目级 AI 编码规范（dsh-dock / DSH Dock）

> 多人协作流程见 `docs/CONTRIBUTING.md`；共享 Prompt 见 `docs/prompts/`；
> 模块契约见 `docs/contracts/`。本文件是**最小必要集**——只写边界与真坑，
> 写入判据见 §11，其余靠你的工程常识与仓库现状。

## 0. 定位（必须理解再动手）

本仓库是 **dsh 的桌面管理面板**（Tauri v2 壳）：把 dsh 工作台以独立、可安装、跨平台
桌面应用呈现，并在不修改 dsh 源码的前提下提供 dsh 管理能力。**壳是通用机制，产品是
数据**：壳不感知具体产品身份，运行时身份只经 `product.manifest.json`（docs/contract.md）
进入，构建期身份只经 `render-product.sh` 注入。逻辑归属：

- 壳运行时（spawn / 宿主解析 / 下载 / 签名验证 / WebView 导航）→ 本仓库；
- dsh 管理（profile / 插件 / 设置凭据 / 会话 / 诊断）→ 本仓库（独特价值）；
- 打包装配 → 装配方（经契约对接），产品数据 → 快照/构建期身份——都不写死进壳。

**红线两条**：

1. **不修改 dsh 源码**（不 fork / 不上游 patch）；允许读源码、调 CLI、文件系统层复现
   其行为（须锚定源码位置 + 日期，登记复现台账 `docs/contracts/dsh-behavior-ledger.md`，
   dsh 升级逐条复核）。
2. **安装包不内置依赖**（Node / dsh / pnpm）：检测宿主，缺失经 system → bundle →
   download 实时补齐。

**工程准则**：职责清晰、可测试、可维护，不破坏 dsh 文件系统不变量（§6；详见 ADR-0009）。

## 1. 技术栈锚点（只记真坑，版本见 Cargo.toml / package.json）

| 锚点 | 裁定 |
|:---|:---|
| Tauri v2 | **tauri-cli 必须与 crate 同代 2.11.x**（不同代 bundler 产物补丁失败）；single-instance 须在 Builder 链最先注册 |
| Rust | edition 2021，**无 MSRV 下限**（2026-08-27 裁定：工具链跟随最新 stable） |
| zip | pinned `=4.2.0`，升级需专项验证 |
| 前端 | React 19 + TS strict + Tailwind v4 + shadcn/ui + React Router v7 + Zustand + Framer Motion（ADR-0008；node ≥20） |
| 数据库 | 按需、仅限管理功能；壳运行时（启动/版本/宿主解析）保持无状态 |

- 命令：Rust `cd src-tauri && cargo test`；前端 `cd frontend && npm ci && npm run
  typecheck/lint/test`；CI 闸门 `cargo fmt --check` + `clippy -D warnings`（三平台）。
- `src-tauri/rustfmt.toml` 仅锁 edition，改动 = 全仓 diff，改前须频道知会。

## 2. 目录（`ls` 即得，只留陷阱）

- `docs/`：contract 契约 · CONTRIBUTING 协作 · broadcasts 知会档案（append-only）·
  prompts · contracts（含 dsh 复现台账）· adr 决策记录（§9）· executor ·
  macos-signing · roadmap（陷阱清单）· frontend-migration · spikes
- `src-tauri/`：`src/` 模块按职责命名（updates = 唯一网络面；settings = 唯一持久化）；
  **main.rs 6 行勿动**；build.rs 自动生成 allow-* 权限；capabilities/ 授权 remote 页面
  （§7 三处同步）；resources/ 的 `dsh-snapshot/` **永不入库**
- `frontend/`：React SPA，单入口按窗口 label 路由；`node-map/` 的
  `node-map-private.key` **永不入库**；`scripts/` regen-icons / render-product（仅打包期）

## 3. 品牌

- 图标 / 徽章一律 **dsh 官方标，禁止手绘或自造 logo**；改 `assets/icon-master.svg` →
  `scripts/regen-icons.sh` 重生成，`src-tauri/icons/` 是产物**勿手改**。
- 页内徽章统一 `Emblem` 组件（CSS mask + `frontend/public/mark.svg`），禁止内联
  第二份鲸鱼 path；几何溯源于 `assets/dsh-logo.svg`。

## 4. 代码规范（项目特有约定，通用工程常识不赘述）

### 4.1 Rust

- Windows 子进程一律经 `crate::child_cmd`（防终端弹窗 + `.cmd/.bat` spawn 必败），
  **禁裸 `Command::new`**。
- 契约改动：先改 `docs/contract.md` → 升 `MANIFEST_FORMAT` → 打包侧同步，缺一不可。
- URL 解析只认 `http(s)://`（拒 `file://` / `data:`），带回归测试；新函数优先单测。
- 裁定性代码注释带日期（`// 2026-08-25 裁定：…`）。

### 4.2 Rust 禁止

- 硬编码产品身份进壳；引入数据库 / IPC 总线 / 领域服务 / React 生态外前端框架——**需 ADR**。
- 非 `updates.rs` 模块触网（§7）；依赖宿主 pnpm store（快照必须自包含，ADR-0004）。

### 4.3 前端

- IPC 统一走 `lib/tauri.ts` 的 `api` 对象，组件内不直接 `invoke`；Promise **必须 `.catch`**。
- 事件总线 `lib/events.ts` **模块加载期**装配——React 子 effect 先于父执行，晚挂监听
  会吞掉首发遥测（boot:step 等），这是踩过的坑；组件只消费 store。
- 样式走 `@theme` token 禁硬编码 hex；文案集中 `content/zh-CN.ts`。

### 4.4 前端三红线

1. **依赖白名单**：React / Radix / Zustand / Framer Motion / Lucide / 数据获取层
   （取数·缓存·同步）；白名单外包需先回写本清单（唯一权威）再广播。
2. **前端运行时禁止发起新网络请求**；网络需求一律经 IPC 到 Rust。
3. **跨窗口真相源**：各窗口独立 JS runtime，Zustand 不跨窗；跨窗信息只经事件广播。

其余：平台语义走 `usePlatform().can.*`；Vitest 只测纯逻辑、不引 RTL/jsdom
（2026-08-28 重评：管理器 UI 落地后仍维持纯逻辑测试——取数/交互逻辑已抽纯函数，
引入 DOM 测试栈无对应诉求；再评触发 = 出现需 DOM 断言的复杂交互或回归）。

## 5. 测试

- 合入前 Rust / 前端测试全绿；**不引测试依赖**（无 dev-dependencies），确需专用依赖
  （异步 / 时间控制等）先 ADR。测纯函数，fixture 内联；真实网络与 WSL 走验证清单
  （`docs/executor.md`）。
- **必带测试**：URL/导航解析（含恶意反例）· 契约字段（正反例各一）· bug 修复
  （复现先行）· 跨平台分叉（覆盖编译目标语义）。

## 6. 存储与生命周期

- 壳运行时无状态；**管理功能不在此限**——管理数据可按需持久化到 `app_data` 自有库/文件。
- 运行时持久化例外册（新增字段须先在此登记）：`settings.json` 原子写、损坏回退默认；
  已登记 `defaultMode`（2026-08-25）· `defaultProfile` 默认启动 profile
  （2026-08-28 落地；None/失效值读取侧兜底 `web`；删除时引用清除、重命名时引用同步）·
  `locale` 界面语言偏好（2026-08-31）· `autoRestart` 崩溃自动拉起守护（2026-08-31）·
  `probe-cache.json` `--no-open` 探测缓存（2026-09-01；可丢失可重建的运行时缓存，
  损坏/缺失回退探测不阻断 boot）。
- **dsh 文件系统不变量**：三件套**不得生成/复刻内容**（初始化归 dsh）；既有三件套的
  整目录复制、`name` 一致化改写、非模板名创建成功后的 web-app 声明单键追加
  （写入例外 #2，2026-08-28）属 profile 生命周期管理（ADR-0009）；`.credentials.yaml`
  保持 0600、顶层仅三键、原子写；会话目录只读不删；`profiles/node_modules` 符号链接
  农场不得直写（陷阱清单见 roadmap §1）。
- 壳与 dsh 严格 1:1 生命周期：退出 / 崩溃都收干净子进程，不留孤儿。
- **pnpm 为环境检查硬依赖**（2026-08-28，ADR-0009 口径 2）：缺失经 `npm i -g pnpm`
  补齐（updates.rs）；WSL 客体内同口径——node/pnpm/dsh 缺失均自动补齐（同日修订原
  4.9 用户主权裁定；node 与本地档同源、tarball 落壳管理目录，ADR-0004 §7）。

## 7. IPC 与网络面（例外册，登记制）

- **IPC 命令登记**（新命令先登记再实现）：`choose_profile` `terminal_action`
  `get_update_status` `check_updates` `get_client_update` `client_update_check`
  `client_update_apply` `open_external` `open_workbench_in_browser` `get_workbench_url`
  `boot_in_wsl` `choose_mode` `list_profiles` `get_profile_detail` `create_profile`
  `copy_profile` `rename_profile` `delete_profile` `set_default_profile`
  `get_default_profile` `switch_profile` `get_active_profile`
  `list_profile_plugins` `get_plugin_runtime`。
  `install_plugin` `remove_plugin` `update_plugin`。
  `get_plugin_rows` `set_plugin_disabled`。
  `check_plugin_updates` `list_plugin_versions`。
  `list_all_plugins`（插件总览聚合，只读文件扫描）`copy_plugin_config`（patch
  配置行原样复制，写入例外 #4，ADR-0009 第五次修订 2026-08-30）。
  `list_sessions` `repair_session` `repair_all_sessions`（会话维护与自愈，2026-08-31）。
  `get_shell_settings` `set_shell_settings` `get_system_diagnostics` `get_app_logs`（系统控制台与诊断，2026-08-31）。
  `get_credentials_raw` `save_credentials_raw` `get_credentials_summary` `set_credential_key`（凭据安全管理与脱敏，2026-08-31）。
  `get_dsh_settings_raw` `save_dsh_settings_raw`（DSH 全局引擎设置，2026-08-31）。
  `list_mcp_servers` `save_mcp_server` `delete_mcp_server`（MCP 服务器结构化管理，2026-08-31）。
  `delete_session`（会话删除，2026-08-31）。
  `fetch_market_registry`（社区插件市场 Registry 拉取，2026-08-31）。
- 前端经 `window.__TAURI__.core.invoke` / `event.listen` 消费（remote 页面不享默认授权）；
  事件 = `boot:step` / `boot:error` / `boot:update` / `boot:progress` / `app:update`
  （仅 main/about，capability 授权）。
- **新增 IPC 三处同步（漏一处 remote 调用即静默失败）**：`src/ipc.rs` COMMANDS 登记 →
  `lib.rs` handler + `capabilities/default.json` 授权。build.rs 由常量生成；一致性有
  cargo test 机器闸门（`ipc.rs` gate_tests，2026-08-28），漏处测试红。
- **唯一网络面 = `updates.rs`**；其余模块禁触网，新网络需求先在此登记；外链域名在
  `EXTERNAL_URL_HOSTS` 登记。已登记用途：boot 期 pnpm 补齐（`npm i -g pnpm`，
  2026-08-28，ADR-0009 口径 2）；**插件运行态回环只读查询**（`plugins.rs`，
  `POST http://127.0.0.1:<port>/api/pluginInventory/list`，2s 超时、仅活跃会话、
  一次性快照不订阅——2026-08-29，Spike B / 复现点 11）；**插件更新检查（外网
  registry）**：`updates.rs` `npm_packument_versions`，与 dsh 版本检查同镜像链 /
  同超时 / 同 packument 体积上限（2026-08-29，4.4④）；**社区插件市场 Registry 拉取**：
  `updates.rs` `fetch_market_registry`，镜像链 `awesome-dsh-plugin.com` 与 GitHub raw（2026-08-31）。专项裁定见 §9 索引对应 ADR。

## 8. AI 交互约束

操作者对以下约束负责：

1. **增量生成**：一次会话只做一个明确意图；「顺便把 X 也改了」= 停下，拆成下一次。
2. **禁止一次性大改**：跨模块批量改动、全仓格式化 / 批量重命名 / `clippy --fix` 扫荡
   一律不做；AI 提出超范围改动时拒绝，记入计划另行开工。
3. **必须附带测试**：行为改动必须有对应测试或验证记录；「没测过」不合入。
4. **先读后写**：动文件前先读现状；AI 声称「我记得应该……」时一律以仓库现状为准。
5. **收尾三件事**：相关测试绿 → 人肉读 `git diff` 确认无越界 → 按 CONTRIBUTING 提交
   并频道广播，落档 `docs/broadcasts.md`（频道留不住，落盘才存在）。
6. **不确定就问**：逻辑归属、契约影响面、是否踩红线——问人，不猜。
7. **驳回不合理的规则**：判定规范冲突 / 失效时，停手向维护者提出修订建议（举证冲突点、
   影响面、建议文本），经裁定后修规则；不得变形绕行——变形合规比违规更危险。

## 9. 关键决策索引

影响契约 / 架构 / 安全边界的决策必须**先立 ADR 再动代码**（`docs/adr/`，模板
TEMPLATE.md；立项依据见姊妹仓库 dsh-launcher ADR-0004/0005）。

| ADR | 一行结论 |
|:---|:---|
| [0001](docs/adr/0001-ready-wait-process-liveness.md) | 就绪等待 = 进程存活感知，非死等 |
| [0002](docs/adr/0002-webview-memory-policy.md) | WebView 长会话内存 = 注入 CSS 缓解 |
| [0003](docs/adr/0003-external-link-and-navigation.md) | 外链 = 系统浏览器兜底 + 白名单拦截 |
| [0004](docs/adr/0004-wsl-guest-dsh-install.md) | WSL 客体内安装，Windows 侧壳不触网 |
| [0005](docs/adr/0005-pnpm-global-bin-dir.md) | pnpm 需注入 global-bin-dir，失败回退 npm |
| [0006](docs/adr/0006-network-surface-and-mirror-chain.md) | 唯一网络面 + 镜像链 + 下载双超时 |
| [0007](docs/adr/0007-update-entry-menu-vs-tray.md) | 更新入口 macOS=菜单 / 非 macOS=托盘 |
| [0008](docs/adr/0008-frontend-framework.md) | React 生态白名单与前端三红线 |
| [0009](docs/adr/0009-profile-manager.md) | Profile 生命周期：创建走 dsh plugin 转发链，其余文件层；pnpm boot 硬依赖 |

## 10. 试验协议

- 修改运行时契约或快照布局前，先对照 `docs/contract.md` 确认两侧同步方案；归属
  不确定先问再动手。
- **本文件是共享宪法**：同一时间仅一人可改，改前频道知会，改后贴 diff 摘要；
  宪法级改动 / 快车道 / PR 合并 / 发版 / 占用声明须落档 `docs/broadcasts.md`。

## 11. 写入边界（元规则，2026-08-28 裁定）

> 准入一句话判据：**没有这条，AI 会做错吗？** 会——才写入；不会——落专项文档。
> 内容多了会限制发挥：本文件只写「边界 + 真坑 + 为什么」，不写「怎么做」。

1. **高频**（几乎任何改动都遇到）+ **违约即事故**（构建失败 / 契约破坏 / 越界 /
   安全隐患）+ **不可推导**（代码 / docs / ADR 推不出）+ **无家可归**（放不进既有文档）
   ——四条同时满足才写入。
2. 排除：可推导明细、决策推理（→ADR）、协作细则（→CONTRIBUTING）、契约字段
   （→contract.md）、计划陷阱（→roadmap）、手册实测（→docs 专项）、通知（→broadcasts）、
   模块技法（→模块注释 / contracts）。
3. 形态：结论一句 + 日期 + 指针；升格 ADR 后原条目必须回收，禁双源；不合判据的
   新增 review 可驳回。
4. 预算：全文 ≤ 250 行、单节 ≤ 40 行；回收触发 = 已升格 ADR / 已有测试 CI 兜底 /
   已失效。2026-08-28 减法一轮：248 → 178 行，删除通用工程常识与可推导明细
   （明细见当日广播）。
