# ADR-0009：Profile 管理器——文件系统层越界补位的实现边界

- **日期**：2026-08-27
- **状态**：草案（待维护者确认）
- **提出人**：guan
- **相关方**：profile 管理（4.3）、插件管理（4.4）、`settings.rs`（defaultProfile 持久化）、WSL executor（GUEST_BOOT）、AGENTS.md §6
- **关联**：`docs/roadmap.md`（4.3/4.4）、`docs/spikes/0001-pnpm-forward-chain.md`、`docs/spikes/0002-profile-reference-surface.md`、上游 ADR-0005（pnpm global-bin-dir，同源环境风险对照）

---

## 1. 背景与问题

dsh 没有 profile 全生命周期的官方命令：列出/创建/复制/重命名/删除 profile 均不存在；唯一的隐式物化路径是「内置模板名（web/headless）首启动」与「首次 `plugin add`」。（源码锚定：`dsh-app-boot/lib/index.js`——`PROFILE_TEMPLATES` 仅 web/headless；`resolveProfileDir` 只校验与定位，无列举；Docker 无 profile 管理 CLI 命令。）壳要提供全局、跨 profile、可离线的 profile 管理能力。

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
2. **安装包不内置依赖**：Node / dsh / pnpm 均不内置；优先检测宿主机依赖，不存在时经宿主解析链（system → bundle → download）实时下载补齐（ADR-0004/0005 定位）。管理功能创建 profile 若宿主无 pnpm，**面板自动下载 pnpm 补齐**（与 node/dsh 下载同链），不做「提示用户安装」的降级。
3. **不破坏 dsh 文件系统不变量**：profile 三件套只经 dsh CLI / dsh 自身写；`.credentials.yaml` 保持 0600 权限、顶层仅 version/refs/records 三键、原子写；会话目录只读不删；`profiles/node_modules` 符号链接农场不得直接写入。

工程设计准则（非红线，但必须遵守）：

- **按优秀软件工程设计**：职责清晰、可测试、可维护；管理功能按需要引入数据库 / 持久化（2026-08-27 边界重定义放宽「无状态库」仅约束运行时）。
- **技术正确性**：新增 IPC 命令三处同步（build.rs + capabilities + lib.rs）；Tauri 2.11 语义。
- **命名校验严格**：profile 命名校验与 `resolveProfileDir` 逐字一致（空名 / `/` `\` / `.` / `..` / `node_modules`）。
- **不级联删除**：删除 profile 不删除会话等全局数据（dsh 明示）。

本 ADR 审查时请特别留意：**第二条红线的「pnpm 自动下载」是本次边界重定义新增的扩展**——原 ADR-0004/0005 只覆盖 Node/dsh，pnpm 补齐是本 ADR 决定纳入的（与「不内置依赖、缺失时下载」同一哲学）。若你不同意，可在评审时提出删除该扩展，仅保留「提示用户安装」降级。

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
  - 依赖 pnpm 在 PATH（Spike A 已证：dsh-dock 注入的 PATH 含 pnpm 常见安装位）。
  - pnpm 不在 PATH 时无法创建（除非用内置模板名 web/headless？——仍走 `dsh plugin add`，不解决）。
  - 用户没有 pnpm（macOS 常见）时绕不开——未来可加「引导安装 pnpm」步骤。
- 对照约束：
  - 不修改 dsh 源码 ✅（只 spawn dsh CLI）
  - 不破坏 dsh 文件系统不变量 ✅（三件套由 dsh 自身写，壳只读）
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

- 思路：列出 = 扫描 `profiles/*/package.json`（现有 `list_web_ui_profiles` 就是原型）；详情 = 读 `package.json` + `cordis.patch.yml` + `dsh --dump-config`；删除 = 目录删除（确认 + 不级联）。这些不需要 dsh CLI 参与（dsh 没有对应命令）。
- 采纳：与方案 A 不冲突（A 只负责创建用的 `dsh plugin add`），其余生命周期操作本来就是文件系统操作。
- 注意：`--dump-config` 需要 dsh 运行一次（`dsh --profile <名> --dump-config`）——若 dsh 未安装/未启动，详情只能显示文件层信息（package.json + patch），不能展示组合配置树。

---

## 4. 最终决策

**采用方案 A（创建走 `dsh plugin --profile <新名> add <bundle>` 主路径）+ 方案 E（生命周期操作走纯文件系统），壳侧复刻三件套（方案 B）本版不做**。

- 创建：spawn `dsh plugin --profile <新名> add <bundle>`，任一新名首用即 dsh 自动 initProfile + pnpm 安装 + reconcile。
- 列出/详情/删除/复制/重命名：纯文件系统 + dsh CLI 无相应命令的部分用文件读。
- 复制/重命名/删除的引用面按 Spike B 的结论执行（尤其：rename 需改写 `name: dsh-profile-<新名>`；`profiles/node_modules` 农场不动的修正）。
- 默认启动 profile（4.3④）持久化到 `settings.json` 新字段 `defaultProfile`（第二最小面例外），落地时同步登记 AGENTS §6。
- 失败模式：pnpm 不在 PATH → 展示 dsh 自带文案 + 壳侧平台化安装建议；网络失败 → 提示检查 npm registry 镜像可达性。

---

## 5. 后果与后续行动项

### 正面后果

- 创建路径半官方、低漂移、与 4.4 插件管理同链。
- 不需要引「复刻 dsh 内部格式」的维护债务。
- 失败文案由 dsh 提供，壳侧只需平台化补充。

### 负面后果 / 新增债务

- 创建依赖 pnpm 在 PATH（Spike A 已证 dsh-dock PATH 注入可覆盖常见安装位，但仍有边缘用户无 pnpm）。
- 复制/重命名要处理 `node_modules/`（删除 + 自愈 vs 搬移），增加实现复杂度。
- 添加 `defaultProfile` 到 `settings.json`（第二例外），删除 profile 时需引用检查。
- `--dump-config` 详情页依赖 dsh 可运行（未安装时详情降级为文件层）。

### 行动项

- [ ] 实现 4.3 Profile 管理器（按 roadmap 关键行动①②③④⑤；只读先行：列出 → 详情 → 创建 → 默认持久化 → 复制/重命名/删除）
- [ ] 新增 IPC 命令（如 `list_profiles` / `create_profile` / `delete_profile` / `rename_profile` / `get_profile_detail` / `set_default_profile`）三处同步（build.rs + capabilities + lib.rs）+ AGENTS §7 登记
- [ ] 引入 `serde_yaml` 依赖（用于 `cordis.patch.yml` 读写）
- [ ] `settings.rs` 增加 `defaultProfile` 字段（原子写 + 损坏回退），并同步登记 AGENTS §6（第二例外）
- [ ] WSL GUEST_BOOT 放开多 profile 评估（与 4.3④ 默认 profile 合并评估，见 Spike B §2.5）
- [ ] profile 命名校验复用 dsh 规则（空名 / `/` `\` / `.` / `..` / `node_modules`）
- [ ] 验证：`cargo test` 全绿 + `dsh plugin --profile <新名> add` 转发链实机验证 + 复制/重命名后 `dsh --profile <新名> --dump-config` 可正常输出
- [ ] 公共频道广播宪法变更（AGENTS §6 第二例外登记）

## 6. 复审条件

- **pnpm 在 PATH 不可用的用户占比升高**（如 macOS 出厂 pnpm 已移除、越来越多用户没装 pnpm）→ 复评是否引入「引导安装 pnpm」或重新考虑方案 B 的壳侧复刻 fallback。
- **dsh 改变初始化三件套格式**（如 `nodeLinker: hoisted` 改名）→ 复核方案 A 是否仍成立；若 dsh 改了 `initProfile` 语义，重评引用面。
- **dsh 官方新增 profile 管理 CLI 或全局 profile UI**（roadmap 重排触发器）→ 重新评估壳的增量价值。
- **Spike A 的 Windows 实机测试出现可用性风险**（`shell: win32` 分支语义不同）→ 重开本 ADR（Windows 转发链未实机验证是本版遗留）。
- **WSL 客体内 profile 操纵需求浮现**（4.9 WSL v2）→ 本 ADR 的删除/重命名范围可能需扩到客体内。
