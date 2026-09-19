# ADR-0027：MCP 生效范围建模——两层用户 patch 是一等公民

- **日期**：2026-09-18
- **状态**：已接受
- **提出人**：guan（AI 协作）
- **相关方**：`src-tauri/src/mcp.rs`、`commands/console.rs`、`frontend/src/components/profiles/McpManager.tsx`、`docs/contracts/ipc-and-network-register.md`
- **关联**：ADR-0009（管理面写入口径）、ADR-0020（官方目录写 `- insert:` 挂载行）、ADR-0022（MCP 探测网络面）、AGENTS §6/§7

---

## 1. 背景与问题

真机事故：用户经 dsh-dock 的 MCP 面板添加 `tempad-dev`，面板显示"已启用"，但模型侧
始终没有 `mcp__tempad-dev__*` 工具。逐层核实后，这背后是**四个叠在一起的缺陷**，
其中三个是"壳说的"与"dsh 实际做的"不一致：

1. **写出的形状不可加载**。壳写 `- id: mcp / package: … / config.mcpServers: {…}`，
   但 dsh 组合树里**没有任何 bundle 声明 id 为 `mcp`**，`applyEntryPatches` 对
   找不到的 id 只 warn 再跳过（`vendor/include/src/index.ts:109-113`）；且上游
   `mcp-client` **一个实例只连一台服务器**，字段是扁平的 `serverName`/`transport`，
   没有 `mcpServers` 这个键。（已由前一轮改为 `- insert:` 修复。）
2. **看不见一半的生效范围**。dsh 的用户层其实是**两层**
   （`dsh-app-boot/lib/index.js:1005 readProfilePatches`）：
   bundle 层 → **profile 层**（`profiles/<名>/cordis.patch.yml`）→
   **home 级全局层**（`$DSH_HOME/cordis.patch.yml`）→ `--patch` overlays。
   而壳只读/只写 profile 层 ⇒ 全局层定义的 MCP **在面板上根本不可见**（用户以为没配），
   也**删不掉**。实测（`dsh --profile web --dump-config`） globale 层的
   `mcp-tempad-dev` 确实出现在 `web` 的组合树里——"对所有 profile 生效"是真的。
3. **两层同名不是覆盖**。上游 `serverName` 是**加载期预留**
   （`mcp-client/src/index.ts:160-176`）：两条都会插入，先加载的占住名字，
   后加载的那条**实例化失败**（`serverName "x" is already in use`）。
   本次事故里，profile 层一条 + 全局层一条同名，恰好把能用的那条顶掉。
4. **保存会静默删掉壳不认识的键**。`upsert` 原先整项替换 insert 条目 ⇒
   上游的 `cwd` / `failOnStartupError` / `toolCallTimeoutMs` / `maxInstructionBytes` /
   `reconnect` 全部丢失。本机唯一可用的 stdio 写法正依赖 `cwd`
   （壳的 `McpServerConfig` 当时甚至没有这个字段）——用户在面板里点一次"编辑→保存"，
   MCP 就再也起不来。

## 2. 约束与硬指标

- **不修改 dsh 源码**（AGENTS 红线 1）：两层语义只能"读源码 + 复现行为"，并在
  `docs/contracts/dsh-behavior-ledger.md` 锚定位置与日期。
- **写入面登记制**（AGENTS §6/§7）：新增持久化写入目标必须先登记。
- **判据必须与 dsh 同源**（ADR-0020 §7.5 先例）：壳对"命令能不能跑起来"的判断
  必须与 dsh 子进程实际拿到的 PATH 同源，否则出现"壳说行、dsh 不行"。
- **未建模字段不得被静默破坏**：壳只认识生命周期内的子集，不能替用户丢配置。
- 前端三红线（AGENTS §4.4）：文案进字典、样式走 token、IPC 走 `api`。

## 3. 备选方案及评估

### 方案 A：两层建模为 `McpScope`，读写删全部按层 —— ✅ 最终采纳

- 思路：`McpServerConfig` 增 `scope: profile | global`；`list` 读两层并逐条标注
  （顺序 = dsh 的 patch 应用顺序）；`save` 按 `scope` 选文件；`delete`/`probe` 带
  `scope`（行身份 = `(scope, name)`）；前端逐行显示范围徽标，**跨层同名显式告警**；
  并补 `cwd` 字段 + 更新时**就地合并**（保未建模键）。
- 优点：与 dsh 的真实组合顺序同构；"看得见"与"改得动"同时成立；撞名从静默事故
  变成一条可处置的告警。
- 代价/风险：IPC 参数与返回语义变更（命令条数不变）；前端行身份从 `name` 变
  `(scope, name)`，探测缓存键、复制态、删除待确认项都要跟着改。
- 对照约束：读/写都经既有 `PatchFile`（备原子写 + 覆写前备份 + 原文保真），
  新写入目标在 AGENTS §6 与 IPC 登记册登记；无 dsh 源码改动。

### 方案 B：只读两层、只写 profile 层 —— ❌ 否决

- 思路：面板显示全局条目（只读徽标 + "去全局层改"提示），写入仍限 profile 层。
- 否决理由：**半吊子**。用户看得见却改不动、删不掉全局条目；而"这个 MCP 想给所有
  profile 用"是最高频的真实诉求（本机 tempad-dev 就是），逼用户手写 YAML 等于
  把面板的存在意义抹掉。

### 方案 C：把全局层当"另一个 profile"平铺进选择器 —— ❌ 否决

- 思路：在 profile 列表里造一个伪 profile `$DSH_HOME`，复用既有增删改。
- 否决理由：**语义错位且危险**。全局层的写入口径与 profile 层不同（前者不要求
  profile 已物化、影响面是所有 profile），伪 profile 会让"删除 profile"这类既有
  语义有踩到全局层的可能；且凭空多出一个用户从未创建过的 profile 行。

### 方案 D：靠 `--dump-config` 的 `# == <路径>` 段落注释反推 scope —— ❌ 否决

- 思路：从组合树的段落注释读出每条来自哪个文件。
- 否决理由：`plugin_rows_blocking` 是为**行 id 定位**服务的，注释解析是它的内部
  细节；把"生效范围"这种一线事实挂在注释格式上，一旦 dsh 改注释风格就整片失效。
  两层就是两个文件，**直接读文件**才是单源。

## 4. 最终决策

**dsh 的两层用户 patch 是两种生效范围，壳必须同时建模**：`McpScope::{Profile, Global}`
进 IPC 契约，读两层、按层写、按层删/探；跨层同名**显式告警**（不是覆盖、不是去重）；
同时补齐 `cwd` 字段并让"保存"改为**就地合并**，使壳不认识的键在编辑后原样存活。

## 5. 后果与后续行动项

### 正面后果

- 全局层 MCP 可见、可改、可删；"这个服务给谁用"成为界面上的一等事实。
- 事故类（同名撞车、编辑即丢 `cwd`、`npx` 不可达）从静默失败变成可见状态或明确报错。
- `probe` 复现 dsh 的 PATH 与 `cwd`，不再给出"探测通过但 dsh 起不来"的假通过。

### 负面后果 / 新增债务

- IPC 参数与 `McpServerConfig` 字段变更**未随版本发布**，需与前端同批上线。
- 壳对"命令可解析性"只做**如实呈现**（探测 + 提示），**不自动改用户的 command**
  —— 自动把 `npx` 改写成绝对路径会猜错用户的 node 版本管理器选择。
- 占位：WSL 客体档的探测仍显式报错（客体内部 spawn 语义未解决，维持 ADR-0022 口径）。

### 行动项

- [x] `mcp.rs`：`McpScope` + 两层读写删 + `cwd` + `merge_insert_entry`（就地合并）
- [x] `serverName` 校验对齐上游 `SERVERNAME_PATTERN`（`^[A-Za-z0-9_-]{1,32}$`）
- [x] `mcp_probe.rs`：`probe_stdio` 增 `path_env`/`cwd`，复现 dsh 解析条件
- [x] 前端：范围徽标 / 跨层冲突告警 / 作用域选择器 / `cwd` 输入 / 装配状态徽标
- [x] 三修（§7）：`row_id` 贯穿探测/删除/保存 + 同层重名逐行可操作 + 两种重名分别文案
- [x] 三修（§7.2）：`expr` 文本判定 + 结构化保存拒绝 + 启停走文本改写（`toggle_disabled_in_text`）
- [x] 三修（§7.3）：预设与默认命令改 `pnpm dlx`、探测预算 30s、表单补齐 `streamable-http`（url/headers）
- [x] 登记：AGENTS §6（新写入目标）、`docs/contracts/ipc-and-network-register.md`
- [ ] 实机复核：重启 Harness 后确认 `mcp__tempad-dev__*` 工具出现（本机已修好两层配置）

## 6. 复审条件

- dsh 上游 `readProfilePatches` 的层顺序或层集合变化（新增第三层 / 去掉全局层）；
- `mcp-client` 的 `Config` 形状变化（如改为单实例多服务器、`serverName` 规则放宽）；
- `StdioConfig.cwd` 缺省值从空串变为 cwd 继承（届时 `cwd` 的提示口径要重写）；
- 壳开始支持 MCP 的**热更新**（当前"改完要重启/等 live 重载"的口径会过时）。

## 7. 修订补录（2026-09-18 三修：行身份贯穿 + 标签值保真）

本 ADR §3 方案 A 原写"行身份 = `(scope, name)`"。两层建模落地后，面板第一次把
**同层重名**摆到了用户面前 ⇒ `(scope, name)` 不再足以定位"用户点的那一行"。三条修订：

### 7.1 行身份收紧，写操作宁可报错也不回退

- **展示态**（探测缓存键、折叠态、待删/待停态）= `(scope, name, 层内序号)`；
  后端按"层 + 层内文件顺序"返回 ⇒ 数组下标就是 dsh 的**加载序**，UI 据此能说清
  "重名两条里活的是这条、失败的是那条"（`lib/mcpForm.ts::dupInfo`）。
- **写操作**（`save` / `delete` / `probe`）额外带 `row_id`。判定口径写死：
  `row_id` **为空** ⇒ 不参与判定（手写行可能没有 `id`，老调用方只有名字）；
  `row_id` **给了却没命中** ⇒ **报错**（"配置可能已被外部改动，请刷新列表后重试"），
  **绝不**回退成"按名字找第一条"。理由：回退 = 静默改写/删掉列表里的另一行，
  而同层重名时"另一行"是**可达路径**，不是理论。
- 连带撤掉一处旧取舍：同层重名一度被**禁止探测**（定位不到那一行）。rowId 贯穿后
  改回放行——禁用是比"探错行"更差的体验，而它已无必要性。

### 7.2 `!!js` 标签值：保真只能走文本，判定粒度是顶层条目

serde_yaml 0.9.34 把带标签的标量解析成 `Value` 时**静默展平**（标签不保留），
故任何"解析→改→重序列化"的值级合并都救不回来——上游用 `!!js process.env.X`
传 secret 引用，展平成字面量等于把凭据来源写坏，而且**界面上看不出来**。裁定：

1. `expr` 由**原文文本**判定（不看解析结果）；
2. 结构化保存在 `expr` 行上**一律拒绝**（错误文案指向"请在 dsh 会话里直接用它的工具"）；
3. 纯启用/停用走**逐字节文本改写**（`toggle_disabled_in_text`）：整行原文带回时放行，
   配置真的变了才拒——否则面板的行内开关会把用户的表达式配置展平掉；
4. 标签判定的粒度 = **顶层条目**（`entry_block_of`），不是"这一行"：`PatchFile`
   的重序列化粒度就是顶层条目，只看子键会漏掉兄弟键上的标签，然后把它展平。

配套坑（同一处代码，值得留字）：`PatchFile::for_each_entry_mut` 的闭包返回值语义是
"**true = 这个条目被改过 ⇒ 丢弃其原文保真并重序列化**"；改了内容却返回 `false`
＝ 改动被静默丢弃。本轮踩过一次（启停写不进去）。

### 7.3 探测口径同步

- 引擎未就绪时**不得**回落"当前进程 PATH"去探（回落会探到 dsh 子进程根本看不见的
  命令，给出可执行的假象）；宁可显式失败。
- 整轮预算 15s → **30s**：冷启动走 `pnpm dlx` / 首跑装包，15s 会把"其实在装依赖"
  误判成失败（登记册 §二 同步）。
- 默认命令与预设改 `pnpm dlx`（`engines/bin` 只有 node/pnpm/dsh，**无 npx**；
  台账复现点 23 ⑦）。
