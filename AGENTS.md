# 项目级 AI 编码规范（dsh-dock / DSH Dock）

> 本文件是所有 AI 编码工具读取的核心上下文，与具体工具无关。
> 多人协作流程（分支 / review / 占用声明 / 发布）见 `docs/CONTRIBUTING.md`；
> 共享 Prompt 模板见 `docs/prompts/`；公共模块契约见 `docs/contracts/`。
> 本文件是**最小必要集**，不是知识库——写入边界见 §11，事无巨细不准入。

## 0. 定位（必须理解再动手）

本仓库是 **dsh 的桌面管理面板**（Tauri v2 壳）：把 dsh 工作台以独立、可安装、跨平台
桌面应用呈现，并在不修改 dsh 源码的前提下提供 dsh 管理能力。**壳是通用机制，产品是
数据**：壳不得感知具体产品身份——运行时身份只经 `product.manifest.json`（docs/contract.md）
进入，构建期身份只经 `render-product.sh` 注入。改动前先判断归属：

- 壳运行时（spawn / 宿主解析 / 下载与进度 / 签名验证 / WebView 导航）→ 本仓库；
- dsh 管理（profile 生命周期 / 插件 / 设置与凭据 / 会话 / 诊断）→ 本仓库（独特价值）；
- 打包装配（物化快照 / 版本 pin / 本地源扫描 / 任务台）→ 装配方（经契约对接），不承接；
- 产品数据（某工作台叫什么 / 装什么插件）→ 快照/构建期身份，不写死进壳。

**红线两条（2026-08-27 边界重定义）**：

1. **不修改 dsh 源码**（不 fork / 不上游 patch）；允许读源码、调 CLI、文件系统层复现其行为
   （须锚定源码位置 + 日期，登记复现台账 `docs/contracts/dsh-behavior-ledger.md`，升级逐条复核）。
2. **安装包不内置依赖**（Node / dsh / pnpm）：优先检测宿主，缺失经解析链
   system → bundle → download 实时补齐。

**工程准则（非红线，详见 ADR-0009）**：职责清晰、可测试、可维护、不破坏 dsh 文件系统不变量（§6）。

## 1. 技术栈锚点（只记坑与裁定，版本明细见 Cargo.toml / package.json）

| 锚点 | 裁定 |
|:---|:---|
| Tauri v2 | **tauri-cli 必须与 crate 同代 2.11.x**（CI `TAURI_CLI_VERSION=2.11.4`；不同代 bundler 产物补丁失败） |
| Rust | edition 2021，**无 MSRV 下限**（2026-08-27 裁定：`rust-version` 已移除，工具链跟随最新 stable） |
| 单实例 | tauri-plugin-single-instance **须在 Builder 链最先注册** |
| zip | pinned `=4.2.0`，升级需专项验证 |
| 自更新 | tauri-plugin-updater，endpoint = GitHub Releases `latest.json` |
| 前端 | React 19 + TS strict + Tailwind v4 + shadcn/ui + React Router v7 + Zustand + Framer Motion + Vite 8（ADR-0008；node ≥20） |
| 数据库 | 按需（管理功能可引入 SQLite 等；壳运行时启动/版本/宿主解析仍无状态） |
| 网络 | ureq 阻塞式，仅限 `updates.rs` 后台线程（唯一网络面 §7） |
| 构建/发布 | GitHub Actions 三平台矩阵 → tag `v*` 出 Release + updater 元数据（build.yml） |

- 常用命令：Rust `cd src-tauri && cargo test`（合入前必须全绿）；前端 `cd frontend &&
  npm ci && npm run typecheck/lint/test`；CI 闸门 `cargo fmt --check` + `clippy -D warnings`。
- `rustfmt.toml`（在 `src-tauri/`）仅锁 edition，改动 = 全仓 diff，改前须频道知会；
  CI 另有 ubuntu coverage job（先出数，阈值另定，roadmap 4.1）。

## 2. 目录结构（`ls` 即得明细，此处只留职责与陷阱）

- `docs/`：contract.md 契约 · CONTRIBUTING 协作 · broadcasts 知会档案（append-only）·
  prompts 模板 · contracts 模块契约 · adr/ 决策记录（§9）· executor.md 执行环境 ·
  macos-signing 签名手册 · roadmap 路线与陷阱清单 · frontend-migration 迁移记录 ·
  spikes/ 专项验证
- `src-tauri/src/`：lib（装配 + IPC + 菜单托盘）/ shell（spawn · URL · 优雅停止）/
  resolve（宿主解析链）/ updates（唯一网络面）/ manifest（契约解析）/ executor（执行
  环境抽象 local/wsl）/ settings（唯一持久化）/ updater（桥接）；**main.rs 6 行勿动**
- `src-tauri/` 其余：build.rs 自动生成 allow-* 权限 · capabilities/ remote 页面授权
  （§7 三处同步）· resources/ 运行时契约 + 可选离线档（`dsh-snapshot/` **永不入库**）·
  icons/ **生成产物勿手改**
- `frontend/`：React 19 SPA，单入口按窗口 label 路由；`lib/tauri.ts` invoke 唯一入口 ·
  `lib/events.ts` 事件总线 · `content/zh-CN.ts` 全部文案 · `__tests__/` Vitest 单测
- `node-map/` 签名 Node 版本映射包（`node-map-private.key` **永不入库**）· `scripts/`
  regen-icons.sh 图标重生成 / render-product.sh 构建期身份注入（仅装配方 / CI，运行时不执行）

职责红线：前端禁入白名单外依赖、禁网络请求（§4.4）。

## 3. 品牌规则

- 桌面图标 / 页内徽章一律 dsh 官方标，**禁止手绘或自造占位 logo**。
- 改图标 = 改 `assets/icon-master.svg` → 跑 `scripts/regen-icons.sh` 整体重生成；
  `src-tauri/icons/` 是生成产物，**禁止直接手改**。
- 页内徽章统一 React 组件 `Emblem`（CSS mask + `frontend/public/mark.svg` 形状源），
  **不允许内联第二份鲸鱼 path 或第二种颜色**；几何溯源于 `assets/dsh-logo.svg`，
  未获官方新版不得偏离。

## 4. 代码规范

### 4.1 Rust ✅ 必须

- 跨平台语义显式：`#[cfg]` 分叉（unix SIGTERM→SIGKILL / Windows kill）；Windows 子进程
  一律经 `crate::child_cmd`（防终端弹窗 + `.cmd/.bat` spawn 必败），**禁裸 `Command::new`**。
- 子进程 stdout/stderr 进数据目录日志（`Read + try_wait` 轮询），不阻塞 UI 线程。
- 快照零部件缺失 → 就地错误页 + 可行动文案（ADR-0004 A6），绝不静默降级。
- 契约改动：先改 `docs/contract.md` → 升 `MANIFEST_FORMAT` → 打包侧同步，缺一不可。
- URL 解析只认 `http(s)://`（拒绝 `file://` / `data:`），带回归测试；新函数优先单元测试。
- 中文 doc 注释讲「为什么」，裁定性注释带日期。

### 4.2 Rust ❌ 禁止

- 禁 `unwrap()`（不变量用 `expect` / Mutex 中毒同此）、阻塞主线程、`println!`/`dbg!`、硬编码产品身份。
- 引入数据库 / IPC 总线 / 领域服务 / React 生态外的前端框架——**需 ADR**；
  数据库仅限管理功能侧，壳运行时保持无状态。
- 依赖宿主 pnpm store 或触网取依赖（快照必须自包含，ADR-0004）；非 `updates.rs`
  模块触网（§7 白名单）。

### 4.3 前端（React 壳页面，ADR-0008）

- 函数组件 + hooks，文件名 PascalCase；页面 `pages/`、组件 `components/`、基础件 `components/ui/`（shadcn 只读）。
- 样式一律 Tailwind + `@theme` token，禁硬编码 hex；明暗基调属契约区
  （frontend-migration.md §0），暗色远期经 data-theme 覆盖变量、组件零改动。
- 状态一个领域一个 store（Zustand），组件用精细选择器，禁整体解构订阅高频字段。
- IPC 统一走 `lib/tauri.ts` 的 `api` 对象，组件内不直接 `invoke`；Promise **必须 `.catch`**。
- 事件总线 `lib/events.ts` **模块加载期**装配（早于页面播种 invoke——React 子 effect
  先于父执行，晚挂监听会吞掉首发遥测）；组件只消费 store。
- 文案集中 `content/zh-CN.ts`，组件不硬编码中文；新 IPC 命令走 §7 三处同步。

### 4.4 前端三条红线（2026-08-27）

1. **依赖白名单**：React / Radix / Zustand / Framer Motion / Lucide / 数据获取层（取数·缓存·同步）；
   禁大全件库、CSS-in-JS 运行时、白名单外包。新增依赖 = 先回写本清单（唯一权威）再广播。
2. **前端运行时禁止发起新网络请求**（无 CDN / 字体 / 统计）；网络需求一律经 IPC 到 Rust。
3. **跨窗口真相源**：各窗口独立 JS runtime，Zustand 不跨窗共享；跨窗信息只经事件广播。

其余：TS strict 禁 `any`；平台语义走 `usePlatform().can.*`；Vitest 只测纯逻辑、
不引 RTL/jsdom（再评估触发：Profile 管理器 UI 落地时重评组件测试策略）。

## 5. 测试要求

- 运行与闸门命令见 §1；合入前 Rust / 前端测试必须全绿（CI 三平台复跑）。
  **不引入额外测试依赖**（Cargo.toml 无 dev-dependencies）；为写好测试确需专用依赖
  （异步 / 时间控制等）时，先 ADR 再引入。
- Mock 策略：不引 mock 框架，测纯函数（URL / 契约校验 / 路径推导 / 镜像链 / 签名格式），
  fixture 内联 + 临时目录；真实网络与 WSL 行为走验证清单（`docs/executor.md`）。
- **必须带测试**：① URL/导航解析 → 回归测试含恶意反例；② 契约字段 → 正反例各一
  （合法 v1/v2 + 缺字段 / 错 format 拒绝）；③ bug 修复 → 复现测试先行；
  ④ 跨平台分叉 → 至少覆盖编译目标语义。
- 前端 Vitest 只测纯逻辑（格式化 / 采样 / 状态机迁移），不测 UI 渲染与 Tauri 事件
  （手动验证见 frontend-migration.md §9）。
- 覆盖率：CI coverage job 已出 lcov（roadmap 4.1），阈值攒数后另定；updater 真实
  download / install 链路走实机「检查更新」验证（roadmap 4.2）。

## 6. 存储与生命周期

- 壳运行时无状态（2026-08-27 重定义）：数据目录只写 `dsh-shell.log`；**管理功能不在
  此限**——管理数据可按需持久化到 `app_data` 自有数据库 / 文件。
- 运行时持久化例外册（登记制，新增字段须先在此登记）：载体 `<app_data>/settings.json`，
  原子写（tmp+rename）、损坏回退默认。已登记：`defaultMode`（2026-08-25）·
  默认启动 profile（2026-08-27 批准，落地先在此登记字段名方可合入，roadmap 硬约束 4）。
  其余运行时核心态一律不落盘。
- **dsh 文件系统不变量**（管理功能必须维护）：profile 三件套只经 dsh CLI 写；
  `.credentials.yaml` 保持 0600、顶层仅三键、原子写；会话目录只读不删；
  `profiles/node_modules` 符号链接农场不得直接写入（陷阱清单见 docs/roadmap.md §1）。
- 壳与 dsh 严格 1:1 生命周期：退出 / 崩溃都要收干净子进程，不留孤儿。

## 7. IPC 与网络面（最小面例外册，登记制）

- **IPC 命令登记**（新命令必须先在此登记再实现）：`choose_profile` `terminal_action`
  `get_update_status` `check_updates` `get_client_update` `client_update_check`
  `client_update_apply` `open_external` `open_workbench_in_browser` `get_workbench_url`
  `boot_in_wsl` `choose_mode`。
- **前端调用例外**：经 `window.__TAURI__.core.invoke` / `event.listen` 消费（remote
  页面不享默认授权）；事件 = `boot:step` / `boot:error` / `boot:update` / `boot:progress`
  （Rust 侧节流 ≥100ms；updates 零 tauri 依赖，经 lib.rs 桥接上抛）+ `app:update`
  （updater 回推，仅 main/about 窗口，capability 显式授权）。
- **新增 IPC 三处同步（2026-08-25 裁定；漏一处 remote 调用即静默失败）**：
  `build.rs` AppManifest commands（自动生成 allow-*）+ `capabilities/default.json`
  permissions 逐个引用 + `lib.rs` 命令实现。
- **唯一网络面 = `updates.rs`**（元数据 / dsh 安装 / Node 下载 / node-map 验签 fail-closed /
  客户端自更新）；其余模块不得触网，新网络需求先在此登记再写；网络动作一律后台线程。
- 专项裁定（推理与细节一律见 §9 索引对应 ADR）：就绪等待 = 进程存活感知（0001）·
  WebView 内存 = 注入 CSS 缓解（0002）· 外链白名单 `EXTERNAL_URL_HOSTS`（0003）·
  WSL 客体内安装且 Windows 侧壳不触网（0004）· pnpm 需注入 global-bin-dir、失败回退
  npm（0005）· 镜像链与下载双超时（0006）· 更新常驻入口 macOS=菜单 / 非 macOS=托盘、
  顶栏「关于」已删、upgrade_only 下次启动生效（0007）。

## 8. AI 交互约束

无论使用哪种 AI 编码工具，操作者对以下约束负责：

1. **增量生成**：一次会话只做一个明确意图（一个功能 / 一个修复 / 一次重构）；
   「顺便把 X 也改了」= 停下，拆成下一次。
2. **禁止一次性大改**：不做跨模块批量改动、全仓格式化 / 批量重命名 / `clippy --fix`
   扫荡（冲突协议见 CONTRIBUTING 占用声明）；AI 提出超范围改动时一律拒绝，记入计划另行开工。
3. **必须附带测试**：行为改动必须有对应测试或明确验证记录（单测 / 实机清单 / PR 说明）；
   「没测过」的代码不合入。
4. **先读后写**：动文件前先读现状与本文件相关章节；AI 声称「我记得应该……」时一律以
   仓库现状为准，不猜、不静默扩权。
5. **收尾三件事**：`cargo test` 绿 → 人肉读 `git diff` 确认无越界 → 按 CONTRIBUTING
   提交并频道广播完成通知，落档 `docs/broadcasts.md`（频道留不住，落盘才存在）。
6. **不确定就问**：逻辑归属（壳 vs 打包侧）、契约影响面、是否踩红线——问人，不猜。
7. **驳回不合理的规则（2026-08-27 维护者授权）**：判定规范冲突 / 失效时，停手向维护者
   提出驳回与修订建议（举证冲突点、影响面、建议文本），经裁定后修规则；不得变形绕行——
   顺从 ≠ 忠诚，变形合规比违规更危险。

## 9. 关键决策记录索引

立项依据（姊妹仓库）：`dsh-launcher/docs/adr/0004`（独立桌面打包）、`0005`（桌面终端
定位）。本仓库 ADR 在 `docs/adr/`（模板 TEMPLATE.md）；**影响契约 / 架构 / 安全边界的
决策必须先立 ADR 再动代码**（上游 ADR-0004 即先例）。

| ADR | 一行结论 |
|:---|:---|
| [0001](docs/adr/0001-ready-wait-process-liveness.md) ready-wait | 进程存活感知取代死等（退出即败 / 卡死判定 / teardown） |
| [0002](docs/adr/0002-webview-memory-policy.md) webview-memory | content-visibility 注入缓解长会话膨胀 |
| [0003](docs/adr/0003-external-link-and-navigation.md) external-link | 系统浏览器兜底 + 白名单拦截 + 三层覆盖 |
| [0004](docs/adr/0004-wsl-guest-dsh-install.md) wsl-guest-install | 壳不触网，安装进 WSL 发行版 |
| [0005](docs/adr/0005-pnpm-global-bin-dir.md) pnpm-global-bin-dir | GUI 子进程无 rc 的 pnpm 10 兼容 + npm 回退 |
| [0006](docs/adr/0006-network-surface-and-mirror-chain.md) network-surface | 唯一网络面 + 四路镜像链 + 下载双超时 + 自更新源 |
| [0007](docs/adr/0007-update-entry-menu-vs-tray.md) update-entry | macOS 菜单 vs 非 macOS 托盘（含托盘修订史） |
| [0008](docs/adr/0008-frontend-framework.md) frontend-framework | React 19 生态白名单与前端三条红线 |
| [0009](docs/adr/0009-profile-manager.md) profile-manager | 管理功能定位与工程准则（3 红线） |

内联裁定规则见 §11.3（例行裁定可留各节；升格后必须回收）。

## 10. 试验协议（AI 协作）

- 修改运行时契约或快照布局前，先对照 `docs/contract.md` 确认两侧同步方案。
- 逻辑归属不确定时先问再动手；多人协同（Git 工作流 / 占用声明 / 发布）见 CONTRIBUTING。
- **本文件是共享宪法**：同一时间仅一人可改，改前频道知会所有活跃协作者，改后贴 diff 摘要。
- **知会落档（2026-08-27）**：宪法级改动 / 快车道直推 / PR 合并 / 发版 / 占用声明，
  频道发送后须在 `docs/broadcasts.md` 追加条目——频道消息不作存证，检索以档案为准。

## 11. 本文件写入边界（元规则，2026-08-28 裁定）

> 本文件每次 AI 会话全量读入，定位是**最小必要集**，不是知识库。
> 准入一句话判据：**没有这条，AI 会做错吗？** 会——才写入；不会——落专项文档。

### 11.1 准入判据（四条同时满足才写入）

1. **高频**：几乎任何改动都会遇到（红线 / 测试 / 提交 / 发布闸门），非特定模块偶发；
2. **违约即事故**：踩了导致构建失败、契约破坏、越界、安全隐患，而非「不够优雅」；
3. **不可推导**：代码 / docs / ADR / 类型签名推不出的裁定——写「为什么」，不写「是什么」；
4. **无家可归**：放不进任何既有文档（ADR / contract.md / CONTRIBUTING / roadmap / 模块注释）。

### 11.2 排除清单（不写入，落对应归宿）

| 内容 | 归宿 |
|:---|:---|
| 模块内部约定、实现技法 | 模块级注释 / `docs/contracts/` |
| 单次决策完整推理（背景 / 备选 / 后果） | ADR——本文件只留一行结论 + 指针 |
| 协作流程细则（分支 / review / 占用声明操作） | `docs/CONTRIBUTING.md` |
| 契约字段与格式细节 | `docs/contract.md` |
| 计划、进度、指标、陷阱明细 | `docs/roadmap.md` |
| 操作手册（签名 / 验证清单 / 实测记录） | `docs/` 专项文件（macos-signing / executor 等） |
| 事件通知与广播原文 | `docs/broadcasts.md` |

### 11.3 形态规则

- 单条 = **结论一句 + 日期 + 指针**；写「必须 / 禁止什么」，不展开「怎么实现」；
- §7 例外册类登记只允许一行一条，细节一律外链 ADR / 专项文档；
- 内联裁定升格 ADR 后，原条目**必须回收**（删或缩为一行指针），禁止双源并存；
- 不合 11.1 判据的新增条目，review 可依据本节直接驳回。

### 11.4 预算与回收

- 预算：全文 ≤ 250 行、单节 ≤ 40 行；超支先想下沉，再想扩写。
- 回收触发（任一）：① 已升格 ADR；② 行为已有测试 / CI 闸门兜底；③ 与后续裁定冲突失效。
- 首轮回收（2026-08-28）：394 → ≤250 行，明细见当日广播。
