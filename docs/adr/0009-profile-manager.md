# ADR-0009：Profile 管理器——文件系统层越界补位的实现边界

- **日期**：2026-08-27（2026-08-28 评审修订）
- **状态**：已接受（2026-08-28 评审通过；pnpm 口径 / 补齐链 / 防御检测经维护者拍板）
- **提出人**：guan
- **相关方**：profile 管理（4.3）、插件管理（4.4）、`settings.rs`（defaultProfile 持久化）、WSL executor（GUEST_BOOT）、AGENTS.md §6
- **关联**：`docs/roadmap.md`（4.3/4.4）、`docs/spikes/0001-pnpm-forward-chain.md`、`docs/spikes/0002-profile-reference-surface.md`、上游 ADR-0005（pnpm global-bin-dir + npm 链，2026-08-28 起兼载 pnpm 补齐）、ADR-0006（镜像链）

---

## 1. 背景与问题

dsh 没有 profile 全生命周期的官方命令：列出/创建/复制/重命名/删除 profile 均不存在；唯一的隐式物化路径是「内置模板名（web/headless）首启动」与「首次 `plugin add`」。（源码锚定：`dsh-app-boot/lib/index.js` v0.1.1-rc.2——`PROFILE_TEMPLATES` 仅 web/headless `@ 323`；`resolveProfileDir` 只校验与定位 `@ 318`；dsh 无 profile 管理 CLI 命令。**2026-08-28 评审勘误**：初版行号 `@ 11826` / `@ 13418` 有误——该文件仅 1216 行，系混入早期 bundle 行号；`initProfile` 实为 `@ 353`、`healProfilesModuleFallback @ 409`、`runPlugin` 在 `lib/plugin-9h8shc4d.js @ 101`。）壳要提供全局、跨 profile、可离线的 profile 管理能力。

矛盾点：
- 这个能力是「在文件系统层模拟 dsh 未提供的语义」——属于路线图的**越界补位**，须立 ADR 说明（roadmap 4.3 明确此要求）。
- 创建路径有两条候选：① spawn `dsh plugin --profile <新名> add <bundle>`（半官方路径）；② 壳侧复刻 `initProfile` 写入三件套。前者是 dsh 官方支持的模式（文档注释明言），但 GUI 子进程无 shell rc，pnpm 是否可用需 spike 验证——ADR-0005 曾踩过「GUI 子进程不加载 shell rc，pnpm 环境不对」的同源坑。
- 复制/重命名/删除的引用面此前只有路线图级清单（`package.json` 内 `dsh-profile-<名>` 字段、`profiles/node_modules` 符号链接农场...），未逐处锚定——若删错会留悬空引用。

两个前置 spike 已完成：
- **Spike A**（`docs/spikes/0001-pnpm-forward-chain.md`）：转发链在 dsh-dock 注入的 PATH 下**可用**，dsh 自带 pnpm-not-found 文案；无需 global-bin-dir 式注入，无需壳侧复刻。
- **Spike B**（`docs/spikes/0002-profile-reference-surface.md`）：真实引用面逐处锚定；关键修正是「农场链接集与 profile 无关」。

---

## 2. 约束与硬指标

红线（依据 2026-08-27 边界重定义，见 AGENTS.md §0）：

1. **不修改 dsh 源码**：不 fork / 不上游 patch；允许读 dsh 源码、调用 dsh CLI、在文件系统层复现其行为（复现须锚定源码参考位置并带日期注释）。
2. **安装包不内置依赖 + pnpm 为环境检查硬依赖**：Node / dsh / pnpm 均不内置；经宿主解析链（system → bundle → download）实时补齐（ADR-0004/0005 定位）。**pnpm 与 node/dsh 同列 boot 硬依赖**（2026-08-28 裁定，口径 = 环境检查保证「dsh 全部子命令可用」）：依赖 pnpm 的是 dsh 自身的 `dsh plugin` 子命令（`runPlugin` 硬编码 `spawnSync("pnpm", …)`，`plugin-9h8shc4d.js:108`，无任何回退参数），壳无法代其选择工具——而壳侧装 dsh 有 npm 回退，「dsh 可用」不蕴含「pnpm 在」，故环境检查必须补上这一硬前提。缺失时经 **`npm i -g pnpm`** 补齐（node 自带 npm；复用 ADR-0005 的 npm 全局安装链，非新下载机制）；补齐失败 → 阻断 boot + 可行动文案（检查网络 / 镜像配置，ADR-0006 链）。**WSL 边界**：客体内与本地同口径——补齐链 node → pnpm → dsh 全自动（2026-08-28 维护者修订原 4.9 / ADR-0004 用户主权裁定：node 缺失自动安装，与本地档同源、tarball 解压壳管理目录，详见 ADR-0004 §7 补录）；二进制下载走 ADR-0006 镜像链，与「npm 镜像参数不注入」不冲突。
3. **不破坏 dsh 文件系统不变量**：壳**不得生成/复刻**三件套内容（初始化模板语义归 dsh，方案 B/C 否决的核心）；既有三件套的**整目录复制**与 `package.json` `name` 字段**一致化改写**（`dsh-profile-<新名>`，Spike B §2.2）属生命周期管理，允许，落地时登记复现台账；`.credentials.yaml` 保持 0600 权限、顶层仅 version/refs/records 三键、原子写；会话目录只读不删；`profiles/node_modules` 符号链接农场不得直接写入。（2026-08-28 评审修订：原表述「三件套只经 dsh CLI / dsh 自身写」与复制/重命名的必做动作字面冲突，已精确化；AGENTS §6 同步修订。）

工程设计准则（非红线，但必须遵守）：

- **按优秀软件工程设计**：职责清晰、可测试、可维护；管理功能按需要引入数据库 / 持久化（2026-08-27 边界重定义放宽「无状态库」仅约束运行时）。
- **技术正确性**：新增 IPC 命令三处同步（build.rs + capabilities + lib.rs）；Tauri 2.11 语义。
- **命名校验严格**：profile 命名校验与 `resolveProfileDir` 逐字一致（空名 / `/` `\` / `.` / `..` / `node_modules`）。
- **不级联删除**：删除 profile 不删除会话等全局数据（dsh 明示）。
- **运行中防护**：删除/重命名前比对壳当前 `launch.profile`（`shell.rs` 持有），命中则拒绝并提示先停止；确认文案须含「确保无其他 dsh 实例（含终端自启）正在使用该 profile」与「内置模板名（web/headless）删除后首次使用将重新物化」。（2026-08-28 评审新增：POSIX 下删运行中目录致 dsh 半瘫、Windows 下目录占用删除失败。）

> **2026-08-28 评审裁定记录**：红线 2 的 pnpm 硬依赖按「**口径 2：环境检查保证 dsh 全部子命令可用**」执行；备选的「口径 1：仅保证可启动（pnpm 软依赖）」「折中：boot 后台预补齐」已评估并否决（口径 1 遗留创建时同步等待与两处检测；折中多一条异步路径且补齐失败静默至创建时才暴露）。补齐机制定为 `npm i -g pnpm`；创建/操作时**保留防御性检测**（毫秒级，防 boot 后环境变化：卸载 pnpm、fnm 切 node 版本），缺失则复用同一补齐函数同步补齐，再失败才降级为 dsh 自带文案 + 平台化安装建议。

---

## 3. 备选方案及评估

### 方案 A：主路径 spawn `dsh plugin` 转发链，无壳侧复刻 —— ✅ 最终采纳

- 思路：创建 profile 用 `dsh plugin --profile <新名> add <bundle>`（任一新名首用即 initProfile 写入三件套 + pnpm 安装 + reconcile 回写 bundles）。壳侧只做封装：校验重名/非法名 → spawn → 解析 dsh 输出 → 展示状态。
- 优点：
  - 半官方路径：dsh 明确支持（`runPlugin` 注释「thin pnpm forwarder: initialize the profile on first use」）。
  - 不复制 dsh 内部格式：三件套内容由 dsh 自己写，格式漂移风险归 dsh 自己。
  - 失败语义已有：dsh 自带 `pnpm not found on PATH` 文案（exit 127）；网络失败模式类同 npm（镜像）。
  - 与 4.4 插件管理同一条转发链：一次验证覆盖两个功能。
- 代价/风险：
  - 依赖 pnpm 在 PATH（Spike A 已证：dsh-dock 注入的 PATH 含 pnpm 常见安装位）。2026-08-28 起 pnpm 缺失由环境检查 boot 硬保证 + 操作时防御补齐兜底（口径 2），此风险收窄为「补齐失败」——阻断 boot / 阻断该操作，均有可行动文案。
- 对照约束：
  - 不修改 dsh 源码 ✅（只 spawn dsh CLI）
  - 不破坏 dsh 文件系统不变量 ✅（三件套由 dsh 自身写，壳只读——此对照仅覆盖创建路径；复制/重命名的写入边界见红线 3 修订）
  - 工程准则（命名校验 / 不级联）✅（复用 dsh 模块，见 §2 命名校验）
  - 技术正确性（IPC 三处同步）✅（spawn 封装，不新增文件格式写）

### 方案 B：主路径 spawn + 壳侧复刻 initProfile 写入三件套（fallback） —— ❌ 否决（本版）

- 思路：pnpm 缺失/失败时，壳直接写 `package.json` + `cordis.patch.yml` + `pnpm-workspace.yaml`（内容 = dsh 的 `PROFILE_PATCH_TEMPLATE` / `PROFILE_PNPM_WORKSPACE`）兜底，让「profile 目录存在但依赖未装」仍算创建成功。
- 否决理由：
  - **违背最小面 + 引入格式漂移维护面**：三件套格式（尤其 `pnpm-workspace.yaml` 的 `nodeLinker: hoisted`）是 dsh 内部格式，随版本变动；dsh 自己的注释明言「Existing files are never touched, so re-running is a no-op」——即 dsh 不期望外部写这些文件，壳写 = 复制 dsh 内部合同，风险自担。
  - **Spike A 显示转发链可用**，无需 fallback；且 dsh 有「init 先执行、pnpm 失败不回滚」语义——创建成功但没装插件的中间态天然存在，壳只需容忍并展示。
  - 未来 dsh 若改三件套内容（如 `nodeLinker` 从 hoisted 变其他），壳侧复刻会悄悄失配，产生难排查 bug。
- 触发：若 spike 显示转发链不可用，则此方案重新评估（留作复审条件）。

### 方案 C：壳直接写三件套作为主路径（不经 dsh CLI） —— ❌ 否决

- 思路：创建 profile 完全由壳做：`mkdir profiles/<新名>` + 写三件套 + 可选 spawn pnpm。
- 否决理由：最严重漂移风险——dsh 若改初始化三件套格式，壳直接静默不兼容；且无法利用 dsh 对依赖闭包的 reconcile 与 pnpm store 复用；与「不修改 dsh 源码」精神相悖（虽未改源码，但在复刻其内部行为）。
- 此方案与方案 B 的区别：方案 B 是补位兜底（只在该 fallback 情景），方案 C 是主路径全部自建。

### 方案 D：复用 dsh 内置模板（web/headless）—— 仅对模板名 —— ⚠️ 部分采纳（不作为通用路径）

- 思路：创建时若用户选了 `web`/`headless`，直接启动即触发 dsh 模板物化，不经 `dsh plugin add`。
- 采纳部分：PROFILE_TEMPLATES 只对这两个名字；`dsh plugin --profile web add` 在 web 目录未物化时也会触发模板初始化（initProfile 用 `PROFILE_TEMPLATES[profile]`）——**所以 `dsh plugin add` 对模板名也适用**，无需单独路径。
- 注意：内置模板名与用户自定义名语义有差异——`web`/`headless` 有官方 bundle 列表（web = base+web-app；headless = base+headless），而新自定义名默认 base。4.3 创建 UI 应提示这个区别。

### 方案 E：纯壳侧文件系统模拟（列举/详情/删除不经 dsh） —— ✅ 接受（与 A 互补）

- 思路：列出 = 扫描 `profiles/*/package.json`；详情 = 读 `package.json` + `cordis.patch.yml` + `dsh --dump-config`；删除 = 目录删除（确认 + 不级联）。这些不需要 dsh CLI 参与（dsh 没有对应命令）。
- 采纳：与方案 A 不冲突（A 只负责创建用的 `dsh plugin add`），其余生命周期操作本来就是文件系统操作。
- 注意（2026-08-28 评审修订）：现有 `list_web_ui_profiles`（`resolve.rs:763`）是 webUi 选择器原型，**不可直接复用**——它无条件注入 `"web"` 并跳过同名目录；管理器须独立扫描器，合并未物化模板名，区分「已物化 profile / 模板名可首启」两种状态（roadmap 4.3①）。
- 注意：`--dump-config` 需要 dsh 运行一次（`dsh --profile <名> --dump-config`）——若 dsh 未安装/未启动，详情只能显示文件层信息（package.json + patch），不能展示组合配置树。详情页对 `--dump-config` **按需触发**（用户显式请求），不随详情页自动加载：spawn dsh 有启动期副作用（heal 会写符号链接农场），落地前先实机确认 dump 是否早退、是否纯读。

---

## 4. 最终决策

**采用方案 A（创建走 `dsh plugin --profile <新名> add <bundle>` 主路径）+ 方案 E（生命周期操作走纯文件系统），壳侧复刻三件套（方案 B）本版不做**。

> **2026-08-28 执行细则修订（方案 A 内部，非换方案）**：创建命令由
> `add @deepseek-ai/dsh-base` 改为 **`install`（不带 bundle 名）**。原因：
> ① 创建语义应为「原始版 profile」——initProfile 以 `PROFILE_TEMPLATES[名] ??
> DEFAULT_PROFILE_BUNDLES` 写三件套（bundles 列表即含 dsh-base），`pnpm install`
> 只装依赖里声明的外挂插件（初始为空 → `Already up to date`，零网络毫秒级）；
> 内置 bundle 本体随 dsh 安装，boot 时 `resolveBundleDir` 双锚点即可解析，
> 无需下载进 profile（Spike B：闭包内插件农场已覆盖）。② `add` 裸包名有
> **dist-tag 版本语义坑**：pnpm 按 `latest` 解析——dsh-base 的 `latest` 停在
> 已弃用 0.0.1-rc.1（依赖 37+ 个已从 registry 删除的旧包名），当前版本走
> `next` tag（0.1.1-rc.2），裸名必装旧版 → 404 + pnpm 递增重试 → 数分钟失败/
> 超时（2026-08-28 本机实测 `test` profile 日志逐字复现）。`install` 不解析
> 任何 dist-tag，版本语义免疫。③ `reconcilePlugins`（plugin-9h8shc4d.js：
> In-box bundles are not dependencies and are never touched）对模板内置
> bundle 零动作，install 后 bundles 保持初始化原始列表，后续用户加外挂插件
> 走同一条 `dsh plugin add` 链（4.4 插件管理）。影响面：仅 `dsh plugin` 的
> 命令参数；pnpm 缺失 → 补齐 → 失败降级（ADR §4 口径 2）与「已创建未装插件」
> 中间态重试语义不变（`install` 对空依赖同幂等）。

> **2026-08-28 第二次执行细则修订（仍为方案 A 内部）：创建补 Web 工作台声明**。
> 非模板名 install 成功后，壳对已初始化三件套的 `dsh.profile.bundles`
> **追加 `@deepseek-ai/dsh-web-app` 单键声明**（幂等；模板名跳过——dsh 拥有
> 模板元组，headless 语义即无 webUi）。动机：defaultProfile 消费只认 webUi
> 候选（bundles 含 web-app，`list_web_ui_profiles`）——纯 dsh-base 原始版
> 创建出来即无法设为默认启动（无 URL 可导航，boot 静默回退 web），与用户
> 预期脱节（2026-08-28 用户实机踩坑）。**红线边界（三件套写入例外 #2，同类
> 先例 = name 一致化改写）**：① 这是既有 dsh 初始化产物上的单键追加，非
> 生成/复刻三件套；② dsh 自认此状态 user-owned——`normalizeShippedProfile`
> （app-boot index.js @ 472，2026-08-28 读）：「Normalize an exact
> installation-owned bundle tuple…**Any other list is user-owned**」，模板
> 精确元组之外的 bundles 列表本就归用户/工具所有；③ 目标状态与出厂 web 模板
> 同构（web 模板 bundles = `[dsh-base, dsh-web-app]` @ 323，web-app 不在
> dependencies、由 `resolveBundleDir` 双锚点从 dsh 安装目录解析、零下载），
> 该形态即用户日常运行的 web profile 本身，行为已被长期验证。**否决替代**：
> `dsh plugin add @deepseek-ai/dsh-web-app@<版本>`（dsh 原生但把 web-app 变成
> 真实依赖 + 网络安装 + 版本锚定难题）偏离「初始 web 标准」；JSON 直改范围
> 严格限 `dsh.profile.bundles` 追加一元素，序列化与 initProfile 逐字同构
> （2 空格缩进 + 尾换行）。写入失败 → `installed=false` 走既有「已创建未装
> 插件」pending 态（重试幂等，重试同时补写）；旧版创建的纯 dsh-base profile
> **不追溯**（既有清单是用户财产，重试创建才按新标准补齐）。

> **2026-08-29 第三次执行细则修订（范围扩展：新增「切换 profile」生命周期操作）**。
> 管理器行内「启动」入口：**停当前会话 → 以目标 profile 重启**（重启语义，非热
> 切换——bundles 在进程启动时挂载，dsh CLI 旗标面只有 `--profile`/`--patch`
> （dsh-cmdline @ 4-9），热切换需改 dsh 源码踩红线 1，排除）。**语义裁定**：
> ① 仅 webUi 候选可切换（bundles 含 web-app；headless 无工作台 URL 可导航），
> 列表新增 `web_ui` 字段做入口可见性（本模块内判定，不复活 `list_web_ui_profiles`
> 复用禁令）；② 切换**不写** defaultProfile——星标是唯一写入口，杜绝「临时切
> 一下，重启后默认被改」的双写口意外；③ 失败落既有错误卡 + 重试同目标
> （`forced_profile` 注入 probe 延续），**不自动回滚**（回滚自身可失败成级联）；
> ④ 强制目标仅用户 home 世界消费（bundle 快照档忽略，同 defaultProfile 档位
> 守卫）。**WSL 同轮覆盖**：guest 启动脚本由写死 `--profile web` 参数化
> （`guest_boot_script(profile)`），profile 名经 `sh_quote` 单引号字面量进脚本
> （validate 拒绝集外仍可含空格/引号/`;` 等元字符——防脚本断裂与注入面；
> 反例测试 + bash 实跑回读覆盖）；teardown 的 stop 标志文件机制不变。
> **多开（多 profile 并行多窗口）不在本修订**：机制面可行（`--port 0` 官方旗标
> = OS 选空闲端口，dsh-web-app help 明文），但双实例并发写全局 `~/.dsh/storages`
> JSON 的竞态未验证，且突破「壳与 dsh 1:1 生命周期」约束需先 spike + 修订本 ADR
> ——已登记 roadmap 待办，届时另立修订或新 ADR。

- 创建：spawn `dsh plugin --profile <新名> install`（2026-08-28 执行细则修订：原
  `add <bundle>` 改 `install`，见本页 §4 修订注——原始版语义，零网络毫秒级），
  任一新名首用即 dsh 自动 initProfile + `pnpm install` + reconcile。bundle 声明
  由 initProfile 以 `PROFILE_TEMPLATES[名] ?? DEFAULT_PROFILE_BUNDLES`
  （`@deepseek-ai/dsh-base`，app-boot `@ 334`）写入三件套的 `dsh.profile.bundles`，
  不参与下载；**非模板名 install 成功后壳追加 web-app 声明（第二次修订，见 §4——
  创建即 webUi 候选，可设为默认启动）**；`web`/`headless` 由 dsh 模板命中
  （`PROFILE_TEMPLATES` `@ 323`，Spike A 遗留事项第 3 条在此收编）；后续加外挂
  插件 = `dsh plugin add <包>`。
  「已创建未装插件」中间态的**重试 = 重跑同名命令**（init-if-needed 幂等，
  `runPlugin` `@ 101`；第二次修订后重试同时补写 web-app 声明，同幂等），
  UI 状态机据此简化。
- 列出/详情/删除/复制/重命名：纯文件系统 + dsh CLI 无相应命令的部分用文件读；删除/重命名前执行运行中防护（比对 `launch.profile`，见 §2 工程准则）。
- 切换（4.3⑥，第三次修订）：管理器行内「启动」→ `switch_profile` IPC（校验：合法名 + webUi 候选）→ teardown → 主窗口回壳 boot 屏（**先回屏再启动**：事件总线模块加载期装配，反序吞首发遥测）→ 强制目标注入 probe → 重启 → 就绪自动导航。当前运行 profile 查询走 `get_active_profile`（会话槽真相，与删除/重命名防护同源）；运行中徽标/确认文案共用。
- 复制/重命名的引用面按 Spike B 的结论执行（尤其：rename 需改写 `name: dsh-profile-<新名>`；`profiles/node_modules` 农场不动的修正；node_modules 处理以「删 + dsh 下次启动自愈」为第一方案）。

> **2026-08-29 第四次执行细则修订（范围扩展：插件禁用/启用，4.4③）——patch 写入例外 #3**。
> 禁用/启用 = 修改 profile 的 `cordis.patch.yml`：写入 `{id: <行id>, disabled: true}`
> 单键条目（禁用）或移除该条目/disabled 键（启用恢复原状）。**dsh 侧依据**：patch
> 语义「纯 disabled 键不碰原行 config」（roadmap §1 已核 2026-08-27；config 键
> 整体替换不深合并，故任何写回只增删 disabled 键、不动既有行）。**行 id 不可从
> 包名推导**（2026-08-29 实测：`@mars-sea/dsh-commandcode-provider` 行 id =
> `llm-commandcode`；`dsh-better-sidebar` = `better-sidebar`——id 由各插件包导出
> 的 `dsh.bundle.patch` 声明）⇒ id 来源定死 `dsh --profile <名> --dump-config`
> 的行表（`- id:`/`name:` 配对；一次 spawn 全量拿到，勿自解析包内 patch 结构）。
> **写入策略**：serde_yaml 0.9 读改写 patch 顶层数组——找到 `id` 匹配条目则仅
> 增删其 `disabled` 键，无条目则追加 `{id, disabled: true}` 双键条目；启用时若
> 条目只剩 id 键则整条移除。**注释保真**：模板头注释为用户可见文档，序列化会
> 丢——实现必须抽取文件头部连续 `#` 注释块、写回时原样前置（其余位置注释不保，
> 记为已知代价）。**选型代价**：serde_yaml 上游已停维护（roadmap 已登记），
> 接受用于本最小写面，读写各收敛在一个函数内便于后继替换。**运行语义**：patch
> 变更对运行中会话不热生效（hmr 默认停用），重启后生效（4.4③ 重启按钮承接）；
> 生效后的运行态变化经回环快照可见。**生效状态真相**：壳写入的 toggle 条目是
> 禁用意图的真相（读自家 patch 文件），dump-config 的 `disabled:`（可能是 `!!js`
> 表达式）只作展示佐证不作解析目标。
> **2026-08-30 第五次执行细则修订（范围扩展：跨 profile 插件聚合 + 从其他
> profile 安装，4.4「跨 profile 复制」收口）——patch 写入例外 #4**。维护者
> 裁定把路线图原「跨 profile 复制（整条目搬移）」重定义为两个更贴使用场景的
> 能力（经 grilling 逐条确认）：①**插件总览**——管理器页内切换视图，聚合展示
> 全部已物化 profile 的第三方依赖插件（内置 bundle 每个 profile 一套、无聚合
> 信息量，不进聚合）；纯文件扫描 manifest（`scan_profiles` 同源，复用
> `list_profile_plugins`），零 dsh 子进程、零网络、只读；视图不承载写操作
> （启停/卸载仍在各 profile 详情）。②**从其他 profile 安装**——详情对话框
> 入口，多选批量（一行 = 插件 × 来源 profile）、失败继续、末尾汇总成败与
> 失败原因；版本默认取来源已装版本（`pkg@<ver>` 固定——规避裸名 dist-tag 坑
> 复现点 7；声明未安装的条目不进候选）；执行走既有 `install_plugin` 转发链
> 前端串行逐项 await，无新安装 IPC。**可选「连配置」（勾选框，默认不勾）**：
> 把来源 profile `cordis.patch.yml` 中该插件行 id 的**全部条目原样复制**到目标
> patch 顶层数组——**写入例外 #4**（同类先例 = #2 web-app 声明、#3 disabled
> 单键）：ⅰ 用户显式勾选触发，非壳自作主张；ⅱ 原样搬移 = 用户既有 patch
> 数据的整体迁移，非壳生成/复刻 dsh 格式内容；ⅲ **只追加不覆盖**——目标已有
> 同 id 条目时零写入并报 skipped（patch 行按 id 定位、config 键整体替换，
> 覆盖会毁目标既有配置）。**行 id 映射**：包名 → 行 id 不可推导（第四次修订
> 实测），复制前经 dump-config 行表定位；`PluginRowState` 扩展 `patch_entries`
> （来源自身 patch 中该 id 的条目数）供勾选框置灰预检，复制时后端权威复核。
> 配置复制独立 IPC `copy_plugin_config`（文件层 + 一次 dump-config spawn），
> 聚合查询新 IPC `list_all_plugins`；两者均 spawn_blocking。变更生效同 #3：
> 不热生效，重启承接。**npm 搜索（roadmap 4.4⑤）维持挂账**，本修订不涉及。
- 默认启动 profile（4.3④）持久化到 `settings.json` 新字段 `defaultProfile`（第二最小面例外），落地时同步登记 AGENTS §6；失效回退值**定死为 `web`**（模板名恒可首启，Spike B §3.3 的「或清除」就此关闭）。
- 失败模式（2026-08-28 口径 2 统一）：创建/插件操作前防御性检测 pnpm（基准 = `effective_path` 注入后的 PATH——壳注入什么 dsh 就能看见什么，Spike A §3.4 同链）→ 缺失则同步补齐（`npm i -g pnpm`，复用 boot 同一函数）→ 补齐失败才降级为 dsh 自带文案（exit 127）+ 壳侧平台化安装建议；网络失败 → 提示检查 npm registry 镜像可达性（ADR-0006）。boot 期同一补齐失败 = 阻断启动 + 可行动文案。

---

## 5. 后果与后续行动项

### 正面后果

- 创建路径半官方、低漂移、与 4.4 插件管理同链。
- 不需要引「复刻 dsh 内部格式」的维护债务。
- 失败文案由 dsh 提供，壳侧只需平台化补充。
- （2026-08-28 执行细则修订）创建走 `install` 后**零网络零下载**：创建 profile
  从「视网络数十秒到数分钟 + 版本解析漂移风险」收敛为「本地秒级且版本语义
  免疫」；外挂插件（4.4）才是 pnpm 网络操作发生的地方——职责更清晰。

### 负面后果 / 新增债务

- **pnpm 为 boot 硬依赖（口径 2）新增一个 boot 失败模式**：补齐失败（网络 / 镜像不可达）阻断启动——含从不用插件管理的用户；这是 2026-08-28 权衡（依赖可预期性 > boot 零阻断面）后接受的代价。操作时的防御补齐失败则只阻断该操作。
- 复制/重命名要处理 `node_modules/`（删除 + 自愈 vs 搬移），增加实现复杂度。
- 添加 `defaultProfile` 到 `settings.json`（第二例外），删除 profile 时需引用检查。
- `--dump-config` 详情页依赖 dsh 可运行（未安装时详情降级为文件层）。

### 行动项（负责人：guan；目标：4.3 开工前完成文档项，实现项随 4.3）

- [ ] 文档同步（2026-08-28 已随本修订完成）：ADR-0005 补录 npm 链角色变化 · AGENTS §6 三件套表述精确化 · AGENTS §7 网络面登记（boot 期 pnpm 补齐）· AGENTS §9 索引行修正 · roadmap 4.3/4.9 更新 · 频道广播 + 落档 `docs/broadcasts.md`
- [ ] 实现 4.3 Profile 管理器（按 roadmap 关键行动①②③④⑤；只读先行：列出 → 详情 → 创建 → 默认持久化 → 复制/重命名/删除）——**只读/创建/生命周期三刀均已落地（2026-08-28）**：`profiles.rs`（命名校验 + 扫描器两态合并 + 详情 + 创建转发链 + 复制/重命名/删除 + 运行中防护）与 IPC `list_profiles` / `get_profile_detail` / `create_profile` / `copy_profile` / `rename_profile` / `delete_profile` / `set_default_profile` / `get_default_profile`；消费 defaultProfile 的 boot 接线（含 WSL 放开多 profile 评估）归后续刀
- [x] 新增 IPC 命令：`list_profiles` / `create_profile` / `copy_profile` / `delete_profile` / `rename_profile` / `get_profile_detail` / `set_default_profile` 三处同步 + AGENTS §7 登记——**全部落地（2026-08-28，另加只读 `get_default_profile`）**；三处同步流程已收敛为 ipc.rs COMMANDS → lib.rs + capabilities，机器闸门 gate_tests
- [ ] YAML 依赖选型：`serde_yaml` 上游已归档停止维护（2024），评估后继（serde_norway 一类）后引入，用于 `cordis.patch.yml` 读写
- [x] pnpm 检测/补齐实现：检测基准 = `effective_path` 注入后的 PATH；补齐 = `npm i -g pnpm`（updates.rs，boot 期 + 操作时复用同一函数）；补齐后验证新 pnpm 在同 PATH 上可见——**全部落地（2026-08-28）**：`updates::ensure_pnpm`（可见即返回 → 缺失经 npm 镜像链同步补齐 → 装完必须同 PATH 可见，否则按失败给可行动文案）。boot 期接线 = LocalExecutor::probe（缺失补齐、失败阻断 boot 出错误卡；WSL 执行器不走此路径，客体内链归 4.9）；创建时接线 = profiles.rs 防御检测升级为「缺失 → 补齐 → 再失败才报错」
- [x] `settings.rs` 增加 `defaultProfile` 字段（原子写 + 损坏回退；失效回退 `web`），并同步登记 AGENTS §6（第二例外）——已落地（2026-08-28）；`switch_mode`/`choose_mode` 同步改为 load-modify-save 防抹掉该字段
- [x] 复现台账入册：ledger 复现点 6/7/8 已落地（2026-08-28），name 一致化改写补录复现点 9
- [x] 重命名实现时：自动扫描 `cordis.patch.yml` 的 `../` 相对路径引用并出警告（替用户做 Spike B 要求的人工检查）——已落地（2026-08-28，纯文本逐行扫描跳过注释行；复制同样带此警告）
- [x] WSL GUEST_BOOT 放开多 profile 评估（与 4.3④ 默认 profile 合并评估，见 Spike B §2.5；客体内 profile 不存在时的行为须一并定义）——**评估结论（2026-08-28）：本版不放开，GUEST_BOOT 维持 `--profile web`，归 4.9 WSL v2**。理由：① WSL 客体 boot 的是客体自身 dsh home 的 profile，管理器只管壳侧 home——壳侧存储的 defaultProfile 在客体内（自定义名）大概率不存在，dsh 报「profile does not exist」；② 非 webUi profile 无 URL 可导航，WSL 的就绪模型（哨兵文件解析 URL）与本地同样不成立；③ 客体内 profile 物化/管理本身是 4.9 的范围。本地消费已落地：`resolve::consume_default_profile` + LocalExecutor::probe 接线——存储默认值命中 webUi 候选即直接启动并跳过选择器（仅 system/download 档，bundle 快照世界不适用；非 webUi/失效值回退常规流程并记日志）
- [ ] executor：客体内 node 自动安装落地（node-map 同源 tarball → `~/.dsh-dock/node`，`guest_prep` 纳入 PATH，`NODE_MISSING` 分支改为自动补齐；归 4.9，见 ADR-0004 §7）
- [x] profile 命名校验复用 dsh 规则（空名 / `/` `\` / `.` / `..` / `node_modules`；锚定 `resolveProfileDir` `@ 318`）——`profiles::validate_profile_name`（2026-08-28，逐字一致含正反例测试，ledger 复现点 8）
- [ ] 验证：`cargo test` 全绿 + `dsh plugin --profile <新名> install` 转发链实机验证（**macOS 已验 2026-08-28**：init 先行 / pnpm 经注入 PATH 可定位 / 空依赖 `Already up to date` 199ms 零网络；2026-08-28 修订后成功路径 = `install`，`--dump-config` 组合启动正常）+ 复制/重命名后 `dsh --profile <新名> --dump-config` 可正常输出 + Windows 转发链（`shell: true` 分支，Spike A 遗留）+ WSL 客体内 node → pnpm → dsh 全补齐链实机（含 node 自动安装）

## 6. 复审条件

- **boot 期 pnpm 补齐失败率/耗时显著**（网络差环境 boot 被阻断）→ 复评口径（降级口径 1「仅保证可启动」或改后台预补齐）。
- **dsh 上游给 `runPlugin` 增加包管理器回退/配置项** → pnpm 硬依赖失去存在理由，红线 2 降级，本决策重开。
- **dsh 改变初始化三件套格式**（如 `nodeLinker: hoisted` 改名）→ 复核方案 A 是否仍成立；若 dsh 改了 `initProfile` 语义，重评引用面。
- **dsh 官方新增 profile 管理 CLI 或全局 profile UI**（roadmap 重排触发器）→ 重新评估壳的增量价值。
- **Spike A 的 Windows 实机测试出现可用性风险**（`shell: win32` 分支语义不同）→ 重开本 ADR（Windows 转发链未实机验证是本版遗留）。
- **WSL 客体内 profile 操纵需求浮现**（4.9 WSL v2）→ 本 ADR 的删除/重命名范围可能需扩到客体内。
