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
2. **安装包不内置依赖**（Node / dsh）：首次使用联网由引擎自补齐；**唯一例外 = pnpm
   引导器随壳内置**（各平台一份，Windows 包另带 musl 份供 WSL 客体；ADR-0010）。

**工程准则**：职责清晰、可测试、可维护，不破坏 dsh 文件系统不变量（§6；详见 ADR-0009）。

## 1. 技术栈锚点（只记真坑，版本见 Cargo.toml / package.json）

| 锚点 | 裁定 |
|:---|:---|
| Tauri v2 | **tauri-cli 必须与 crate 同代 2.11.x**（不同代 bundler 产物补丁失败）；single-instance 须在 Builder 链最先注册 |
| Rust | edition 2021，**无 MSRV 下限**（2026-08-27 裁定：工具链跟随最新 stable） |
| zip | pinned `=4.2.0`，升级需专项验证 |
| 前端 | React 19 + TS strict + Tailwind v4 + shadcn/ui + React Router v7 + Zustand + Framer Motion（ADR-0008；node ≥20） |
| 数据库 | 按需、仅限管理功能；壳运行时（启动/版本/宿主解析）保持无状态 |

- 命令：Rust `cd src-tauri && cargo test`；前端 `cd frontend && pnpm install --frozen-lockfile && pnpm run
  typecheck/lint/test`；CI 闸门 `cargo fmt --check` + `clippy -D warnings`（三平台）。
- **`cargo check` ≠ `clippy`：clippy 需逐目标各跑一次**（2026-09-11 教训，v1.1.1 发版
  构建因此在 CI 才红）。`cfg(windows)` 等分叉代码在宿主上根本不编译，而 `check` 也不跑
  lint——只跑"宿主 clippy + 交叉 check"必然漏掉目标平台独有的 lint。照 CI 同款：
  `cargo clippy --all-targets -- -D warnings` 对宿主与各目标分别执行。
- `src-tauri/rustfmt.toml` 仅锁 edition，改动 = 全仓 diff，改前须频道知会。

## 2. 目录（`ls` 即得，只留陷阱）

- `docs/`：contract 契约 · CONTRIBUTING 协作 · broadcasts 知会档案（append-only）·
  prompts · contracts（含 dsh 复现台账）· adr 决策记录（§9）· executor ·
  macos-signing · roadmap（陷阱清单）· frontend-migration · spikes
- `src-tauri/`：`src/` 模块按职责命名（updates = 唯一网络面；settings = 唯一持久化）；
  **main.rs 6 行有效代码勿动**（7 物理行含 1 空行）；build.rs 自动生成 allow-* 权限；capabilities/ 授权 remote 页面
  （§7 三处同步）；resources/ 的 `dsh-snapshot/` 与 `pnpm/`（打包期经
  `scripts/fetch-pnpm-bundle.sh` 拉取）**永不入库**
- `frontend/`：React SPA，单入口按窗口 label 路由；`node-map/` 的
  `node-map-private.key` **永不入库**；`scripts/` regen-icons / render-product（仅打包期）

## 3. 品牌

- 图标采用 **dsh dock 专属 3D 鲸鱼娘（透明背景主形象 + macOS HIG 浅色陶瓷圆角方框，100% 透明角与微投影，胸前 DeepSeek 官方鲸标，默认比耶 Wink 款）**；主图 `assets/whale-chan-cutout.png` & `assets/whale-chan-light-icon.png` →
  `scripts/regen-icons.sh` 重生成，自带四周透明度强门禁，`src-tauri/icons/` 是产物**勿手改**。
- 页内徽章统一经 `Emblem` 组件渲染：
  - `framed={false}`（默认）：纯透明背景 3D 立绘，消除矩形边界，自带环境光晕，用于启动欢迎、Hero 页面与全屏等候；
  - `framed={true}`：Apple HIG 风格浅色纯色/微渐变圆角方框（`public/icon.png`），用于小尺寸顶栏导航、配置中心与 About 弹窗。禁止散落内联硬编码图标。

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
- 日志统一 `lib/logger.ts`（格式 `[模块] 描述 {ctx}`，2026-09-04）：禁直呼
  `console.*`；绝不打密钥/PII；禁紧密循环/高频事件内打点（Rust 侧同口径：
  禁 `println!`，结构化字段，`error/warn/info/debug` 分级）。

### 4.4 前端三红线

1. **依赖白名单**：React / Radix / Zustand / Framer Motion / Lucide / 数据获取层
   （取数·缓存·同步）/ CodeMirror 6（YAML 编辑器，2026-09-18 裁定入白，仅限配置
   编辑面）；白名单外包需先回写本清单（唯一权威）再广播。
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
  （2026-08-28 落地；None 由消费方兜底 `web`，失效值不消费＝出选择器；删除/重命名同步引用）·
  `locale` 界面语言偏好（2026-08-31）· `autoRestart` 崩溃自动拉起守护（2026-08-31）·
  `showFloatingSwitcher` 悬浮胶囊开关（2026-09-01）· `switcherShortcut` 切换快捷键偏好（2026-09-01）·
  `probe-cache.json` `--no-open` 探测缓存（2026-09-01；可丢失可重建的运行时缓存，
  损坏/缺失回退探测不阻断 boot）· `engines/` 引擎目录（2026-09-03，ADR-0010：
  `PNPM_HOME` 指向的壳管理运行时资产 node/pnpm/dsh；可丢失可重建，缺失走引导）·
  `pluginRegistry` 插件安装源偏好与 `pluginRegistryLastGood` 上次可用源
  （2026-09-16，ADR-0006 §6：auto 先官方、失败换源、成功才记），
  `dismissedUpdate` 升级提示条已忽略版本键（2026-09-04，ADR-0010 升级呈现；
  形如 `dsh@<ver>`，同键不再弹非阻断提示条）· `~/.dsh-dock-dev`（dev 构建的 dsh
  home，2026-09-10，ADR-0015：与正式 `~/.dsh` 隔离，消除"开发期泄漏锁死正式包"）·
  `~/.dsh-dock-test`（`cargo test` 的 dsh home，2026-09-10 审核：测试**一律无视**
  环境里的 `DSH_HOME` 并锁进此目录——否则真机用例会打开用户真实 profile 与正式包
  抢同一 profile；可丢失可重建）。
- 已登记落盘资产（2026-09-11 补登记，判据 `docs/team/文档一致性巡检-2026-09-11.md`）：
  `node-map.json` / `.sig`（`updates.rs:339`，签名校验过的 node 版本映射缓存，
  1 MiB 上限、校验失败回退内置基线）· `<app_data>/safe-mode/<profile>.json` 安全模式记账
  （ADR-0026，2026-09-16：记"安全模式写进配置的停用行 id + 当时的 dsh home + 横幅是否已被
  关闭（`dismissed_at`：关闭只对本轮生效，下次进入重新提示）"，用于把
  "安全模式停的"与"用户自己停的"分开；**不承载任何恢复动作**——一键恢复已按维护者裁定移除，
  覆写前的 `.bak-<unix秒>` 备份仍在，需要时手工取用；可丢失，丢了只是少一条横幅说明；
  旧的 `.yml` overlay 已退役并在进入安全模式时清理）
  · `procs/`（`lifecycle.rs:75`，ADR-0015 孤儿清扫的
  PID 锁登记表）· `<文件名>.bak-<unix秒>` 覆写前备份族（`fs_backup.rs`，2026-09-08 U9）·
  MCP / 插件配置对 `cordis.patch.yml` 的写入：**统一走 `plugins.rs::PatchFile`**
  （宿主/客体同一内核：未改条目原文保真含行间注释 + 覆写前备份 + 原子替换；2026-09-11 统一）。
  **写入目标 = 两个用户层**（profile 层 / `$DSH_HOME` 全局层；2026-09-18，ADR-0027）。
- **dsh 文件系统不变量**：三件套**不得生成/复刻内容**（初始化归 dsh）；既有三件套的
  整目录复制、`name` 一致化改写、非模板名创建成功后的 web-app 声明单键追加
  （写入例外 #2，2026-08-28）属 profile 生命周期管理（ADR-0009）；profile 的
  `pnpm-workspace.yaml` 顶层键 `dangerouslyAllowAllBuilds: true` 单键受控写入
  （写入例外 #5 重立，2026-09-09，ADR-0013：pnpm 构建脚本**默认批准**，非三件套
  成员；原 allowBuilds 逐包裁决链已退役）；`.credentials.yaml` 保持 0600、顶层仅三键、原子写；
  会话目录只读不删；`profiles/node_modules` 符号链接农场不得直写（陷阱清单见 roadmap §1）。
- 壳与 dsh 严格 1:1 生命周期：退出 / 崩溃 / **硬杀（`SIGKILL`、强制退出）**都收干净
  子进程，不留孤儿（2026-09-10 扩展，ADR-0015：原口径只覆盖"父进程临死前能跑代码"的
  路径，硬杀会逃逸成持着会话写锁的孤儿——**新增 spawn 一律经 `lifecycle::spawn`/`run`**，
  有机器闸门拦裸 `Command::spawn()`）。
- **pnpm 为环境检查硬依赖**（ADR-0009 口径 2；补齐方式见 ADR-0010）：随壳内置恒在，node/pnpm/dsh
  缺失走引擎引导补齐（宿主 dsh 安装形态见 ADR-0017/0018），WSL 客体同口径（ADR-0004 §7）。

## 7. IPC 与网络面（例外册，登记制）

- **登记册已迁出**：[`docs/contracts/ipc-and-network-register.md`](docs/contracts/ipc-and-network-register.md)
  ——IPC 命令清单（含各自落地日期与边界）＋ 网络面用途登记 ＋ 文件系统读取域登记。
  迁出理由同 §9：
  登记册随功能**线性增长**，与 §11.4 的固定行数预算结构性冲突（2026-09-15 迁出时为
  249/250，**再加一条登记即越界**）。**规则留在本节，登记册在册**；两处禁双源。
- **新增 IPC 命令**：先登记（本册 + `ipc.rs::COMMANDS`）再实现；三处同步（`ipc.rs`
  COMMANDS → `lib.rs` handler + `capabilities/default.json`）全由机器闸门兜底
  （build.rs 生成 + `ipc.rs` gate_tests，漏处测试红），不必靠人记。
- **唯一网络面 = `updates.rs`**（ADR-0006）；其余模块禁触网，**新网络需求先登记**（登记册 §二）；
  外链域名在 `EXTERNAL_URL_HOSTS` 登记；机器投影必须同步到 `network_gate.rs` 的 `EXEMPTIONS`。
  **回环调用必须附 `/api` 会话 Cookie**：dsh 0.1.6+ 在 Host 栅栏之外还有 `browserAuth`
  （缺失恒 401）；Cookie 由 boot 期 launch token 兑换后留在 `ShellState.workbench_cookie`
  （**仅内存、不打日志**；2026-09-15 实测更正，台账复现点 11）。
- 前端经 `window.__TAURI__.core.invoke` / `event.listen` 消费（remote 页面不享默认授权）；
  事件 = `boot:step` / `boot:error` / `boot:update` / `boot:progress` / `app:update` / `app:settings-changed`
  （仅 main/about/profiles，capability 授权）。

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
8. **发版日志**：发版严禁裸打 tag；必须先按 `docs/prompts/release-notes.md` 生成规范日志并落盘 `docs/RELEASE_NOTES.md` 顶部，保持 `## [vX.Y.Z] - YYYY-MM-DD` 强契约供 CI 脚本精准提取（2026-09-09 裁定）。

## 9. 关键决策索引

影响契约 / 架构 / 安全边界的决策必须**先立 ADR 再动代码**（`docs/adr/`，模板
TEMPLATE.md；立项依据见姊妹仓库 dsh-launcher ADR-0004/0005）。

**ADR 索引（全部编号与一行结论）见 [`docs/adr/README.md`](docs/adr/README.md)。**
（2026-09-15 由本节迁出：索引随 ADR 数量线性增长，与 §11.4 全文行数预算结构性冲突——
当时恰为 250 行，补一行索引即越界；按 §11.4「回收」机制迁出，规则仍留在本节。）

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
