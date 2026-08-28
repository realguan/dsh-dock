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

- 创建：spawn `dsh plugin --profile <新名> add <bundle>`，任一新名首用即 dsh 自动 initProfile + pnpm 安装 + reconcile。bundle 参数：非模板名 = `@deepseek-ai/dsh-base`（`DEFAULT_PROFILE_BUNDLES`，app-boot `@ 334`）；`web`/`headless` 由 dsh 模板命中（`PROFILE_TEMPLATES` `@ 323`，Spike A 遗留事项第 3 条在此收编）。「已创建未装插件」中间态的**重试 = 重跑同名 add**（init-if-needed 幂等，`runPlugin` `@ 101`），UI 状态机据此简化。
- 列出/详情/删除/复制/重命名：纯文件系统 + dsh CLI 无相应命令的部分用文件读；删除/重命名前执行运行中防护（比对 `launch.profile`，见 §2 工程准则）。
- 复制/重命名的引用面按 Spike B 的结论执行（尤其：rename 需改写 `name: dsh-profile-<新名>`；`profiles/node_modules` 农场不动的修正；node_modules 处理以「删 + dsh 下次启动自愈」为第一方案）。
- 默认启动 profile（4.3④）持久化到 `settings.json` 新字段 `defaultProfile`（第二最小面例外），落地时同步登记 AGENTS §6；失效回退值**定死为 `web`**（模板名恒可首启，Spike B §3.3 的「或清除」就此关闭）。
- 失败模式（2026-08-28 口径 2 统一）：创建/插件操作前防御性检测 pnpm（基准 = `effective_path` 注入后的 PATH——壳注入什么 dsh 就能看见什么，Spike A §3.4 同链）→ 缺失则同步补齐（`npm i -g pnpm`，复用 boot 同一函数）→ 补齐失败才降级为 dsh 自带文案（exit 127）+ 壳侧平台化安装建议；网络失败 → 提示检查 npm registry 镜像可达性（ADR-0006）。boot 期同一补齐失败 = 阻断启动 + 可行动文案。

---

## 5. 后果与后续行动项

### 正面后果

- 创建路径半官方、低漂移、与 4.4 插件管理同链。
- 不需要引「复刻 dsh 内部格式」的维护债务。
- 失败文案由 dsh 提供，壳侧只需平台化补充。

### 负面后果 / 新增债务

- **pnpm 为 boot 硬依赖（口径 2）新增一个 boot 失败模式**：补齐失败（网络 / 镜像不可达）阻断启动——含从不用插件管理的用户；这是 2026-08-28 权衡（依赖可预期性 > boot 零阻断面）后接受的代价。操作时的防御补齐失败则只阻断该操作。
- 复制/重命名要处理 `node_modules/`（删除 + 自愈 vs 搬移），增加实现复杂度。
- 添加 `defaultProfile` 到 `settings.json`（第二例外），删除 profile 时需引用检查。
- `--dump-config` 详情页依赖 dsh 可运行（未安装时详情降级为文件层）。

### 行动项（负责人：guan；目标：4.3 开工前完成文档项，实现项随 4.3）

- [ ] 文档同步（2026-08-28 已随本修订完成）：ADR-0005 补录 npm 链角色变化 · AGENTS §6 三件套表述精确化 · AGENTS §7 网络面登记（boot 期 pnpm 补齐）· AGENTS §9 索引行修正 · roadmap 4.3/4.9 更新 · 频道广播 + 落档 `docs/broadcasts.md`
- [ ] 实现 4.3 Profile 管理器（按 roadmap 关键行动①②③④⑤；只读先行：列出 → 详情 → 创建 → 默认持久化 → 复制/重命名/删除）——**只读刀已落地（2026-08-28）**：`profiles.rs`（命名校验 + 扫描器两态合并 + 详情）与 IPC `list_profiles` / `get_profile_detail`；创建/持久化/复制/重命名/删除归后续刀
- [ ] 新增 IPC 命令：`list_profiles` / `create_profile` / `copy_profile` / `delete_profile` / `rename_profile` / `get_profile_detail` / `set_default_profile` 三处同步（build.rs + capabilities + lib.rs）+ AGENTS §7 登记——`list_profiles` / `get_profile_detail` 已落地登记（2026-08-28，三处同步流程已收敛为 ipc.rs COMMANDS → lib.rs + capabilities，机器闸门 gate_tests）；其余随对应刀
- [ ] YAML 依赖选型：`serde_yaml` 上游已归档停止维护（2024），评估后继（serde_norway 一类）后引入，用于 `cordis.patch.yml` 读写
- [ ] pnpm 检测/补齐实现：检测基准 = `effective_path` 注入后的 PATH；补齐 = `npm i -g pnpm`（updates.rs，boot 期 + 操作时复用同一函数）；补齐后验证新 pnpm 在同 PATH 上可见
- [ ] `settings.rs` 增加 `defaultProfile` 字段（原子写 + 损坏回退；失效回退 `web`），并同步登记 AGENTS §6（第二例外）
- [ ] 复现台账入册：ledger §二 复现点 6/7/8 随实现落地，行号锚定按下述修正
- [ ] 重命名实现时：自动扫描 `cordis.patch.yml` 的 `../` 相对路径引用并出警告（替用户做 Spike B 要求的人工检查）
- [ ] WSL GUEST_BOOT 放开多 profile 评估（与 4.3④ 默认 profile 合并评估，见 Spike B §2.5；客体内 profile 不存在时的行为须一并定义）
- [ ] executor：客体内 node 自动安装落地（node-map 同源 tarball → `~/.dsh-dock/node`，`guest_prep` 纳入 PATH，`NODE_MISSING` 分支改为自动补齐；归 4.9，见 ADR-0004 §7）
- [x] profile 命名校验复用 dsh 规则（空名 / `/` `\` / `.` / `..` / `node_modules`；锚定 `resolveProfileDir` `@ 318`）——`profiles::validate_profile_name`（2026-08-28，逐字一致含正反例测试，ledger 复现点 8）
- [ ] 验证：`cargo test` 全绿 + `dsh plugin --profile <新名> add` 转发链实机验证 + 复制/重命名后 `dsh --profile <新名> --dump-config` 可正常输出 + Windows 转发链（`shell: true` 分支，Spike A 遗留）+ WSL 客体内 node → pnpm → dsh 全补齐链实机（含 node 自动安装）

## 6. 复审条件

- **boot 期 pnpm 补齐失败率/耗时显著**（网络差环境 boot 被阻断）→ 复评口径（降级口径 1「仅保证可启动」或改后台预补齐）。
- **dsh 上游给 `runPlugin` 增加包管理器回退/配置项** → pnpm 硬依赖失去存在理由，红线 2 降级，本决策重开。
- **dsh 改变初始化三件套格式**（如 `nodeLinker: hoisted` 改名）→ 复核方案 A 是否仍成立；若 dsh 改了 `initProfile` 语义，重评引用面。
- **dsh 官方新增 profile 管理 CLI 或全局 profile UI**（roadmap 重排触发器）→ 重新评估壳的增量价值。
- **Spike A 的 Windows 实机测试出现可用性风险**（`shell: win32` 分支语义不同）→ 重开本 ADR（Windows 转发链未实机验证是本版遗留）。
- **WSL 客体内 profile 操纵需求浮现**（4.9 WSL v2）→ 本 ADR 的删除/重命名范围可能需扩到客体内。
