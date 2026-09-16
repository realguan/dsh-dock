# ADR-0022：MCP 连通性与能力探测的网络 / 子进程面归属

- **日期**：2026-09-15
- **状态**：**已接受**（2026-09-15 维护者裁定「评审通过」，含 §1.4 增量价值主张获确认）
  ——I3 增量据此解冻
- **提出人**：guan（AI 协作起草）
- **相关方**：`src-tauri/src/mcp.rs`（探测实现落点）、`src-tauri/src/commands/console.rs`
  （既有 MCP 命令所在，`list/save/delete_mcp_servers`）、`src-tauri/src/network_gate.rs`
  （EXEMPTIONS 登记表）、`src-tauri/src/lifecycle.rs`（spawn seam）、`src-tauri/src/ipc.rs`
  （新命令登记）、`src-tauri/capabilities/default.json`、`frontend/src/components/profiles/McpManager.tsx`、
  `frontend/src/lib/tauri.ts`
- **关联**：**ADR-0006**（唯一网络面 = `updates.rs`；注意其 2026-08-27 边界重定义注）；
  **ADR-0015**（子进程生命周期归属；spawn 收敛到 `lifecycle` 单点 seam）；
  **ADR-0016**（管理面下沉 WSL 客体，本探测须双侧同构）；
  **ADR-0020**（插件安装激活契约，本探测不改其安装语义）；
  **ADR-0021**（typert 回环通道的登记与消费先例，与本案的"新探测面"同类）；
  AGENTS §4.2 / §4.4 / §6 / §7；
  `docs/roadmap.md:196-202`（§4.7 未关闭子项）；`docs/contracts/dsh-behavior-ledger.md` 复现点 11
  （插件运行态回环只读查询 = 本 ADR 的范式先例）；`docs/plans/official-plugins-and-capabilities-plan-review.md` §B6

---

## 1. 背景与问题

### 1.1 需求

`McpManager` 目前只提供静态配置表单：用户能看到自己写了哪些 MCP 服务器，但看不到
**这些服务器实际暴露了什么**。需求是增加一个「连通性与能力探测」动作，对选定的
MCP 服务器枚举三类能力：

- Tools（`tools/list`）
- Resources（`resources/list`）
- Resource Templates（`resources/templates/list`）

### 1.2 现状

`src-tauri/src/mcp.rs` 是**纯静态配置 CRUD**：`parse_mcp_servers` / `list_mcp_servers` /
`upsert_into_patch` / `remove_from_patch` / `apply_save_mcp_server` / `apply_delete_mcp_server`
及其 WSL 客体孪生（`*_in_guest`）。该文件中 `probe` / `connect` / `handshake` / `initialize` /
`tools/list` / `resources/list` / `jsonrpc` **全部零命中**。前端 `McpManager.tsx` 只调用
`api.listMcpServers` / `api.saveMcpServer` / `api.deleteMcpServer`，无任何探测入口。

这不是新想法，而是一个**已知未关闭项**：`docs/roadmap.md:202` 明确记载
> 原文的「MCP 工具列表与连接状态查看」子项未见对应实现，**未随本次回收关闭**。

### 1.3 可行性硬事实：DSH 不提供任何可供壳枚举 MCP 的 RPC

这是本 ADR 最重要的事实，它决定了实现形态：

- `packages/api/`（9 个 API 包）中 **`mcp` 命中数为 0**；不存在 MCP Remote 命名空间。
- `mcp-resources` 注册的是**面向模型**的工具，不是面向宿主的 API：
  `list_mcp_resources` / `list_mcp_resource_templates` / `read_mcp_resource`
  （`packages/mcp/mcp-resources/src/tools.ts:35,43,57`）。
- `grep '@Remote' packages/mcp/*/src/*.ts` 只命中 `mcpResources` 服务注册，无 verb。
- DSH 自己通过官方 MCP SDK 在 `mcp-client` 内部完成握手与列举：
  `packages/mcp/mcp-client/src/connection.ts:308`（`connect` = SDK `initialize`）、
  `:372-381`（`listResources` / `listResourceTemplates` / `readResource`）、
  `packages/mcp/mcp-client/src/tools.ts:121-123`（`client.listTools`）。

**结论**：壳无法"转发"一次探测。要得到服务器的能力清单，**壳必须自己说 MCP 协议**。

### 1.4 增量价值论证（若此节不成立，本 ADR 应直接判为冗余）

模型侧已经能列举 MCP 资源（上述三个工具），因此"用户想知道 MCP 暴露了什么"这件事
**今天已有答案——直接问模型**。GUI 探测若要有存在价值，必须落在模型路径做不到或代价过高的地方：

| 增量 | 说明 |
|:---|:---|
| 配置体检 | 不消耗 token、不起会话即可判断"这条配置是否连得上"，用于装完/改完立刻自检 |
| 失败归因 | 区分"命令不存在" / "启动即退" / "握手超时" / "鉴权失败" / "传输不匹配"，模型路径只能拿到一个笼统的工具错误 |
| 零副作用 | 不写会话日志、不进入上下文、不污染用户与模型的对话 |
| 传输分支可见 | 明确告诉用户该 server 声明的是 `stdio` 还是 `streamable-http`，以及探测走了哪条合规通道 |

反之，若维护者判定以上价值不足以支撑一个新网络面，则本 ADR 的正当结论是**不做**，
并关闭 roadmap §4.7 该子项（见 §3 方案 E）。

### 1.5 传输面：恰好两种，且合规含义相反

MCP transport 的取值**恰好是** `stdio` | `streamable-http`
（`packages/mcp/mcp-client/src/transport.ts:23-38`；schema 联合见
`packages/mcp/mcp-client/src/index.ts:119-142`）。**不存在 `sse` 这个字面量**——
"SSE" 只是 Streamable HTTP 的实现细节，不是独立传输。

这个二分正是本 ADR 的核心：两种传输在**本仓库的合规框架下落点完全不同**。

- `transport: stdio` → 配置含 `command` / `args` / `env` / `cwd`，本质是**起一个子进程**。
- `transport: streamable-http` → 配置含 `url` / `headers`，本质是**发起 HTTP 请求**。

### 1.6 现状基线（本决策必须胜过它）

保持现状 = 静态 patch 文件视图。它的成本为零、无新网络面、无新依赖。方案 A 必须证明
其增量（§1.4）大于新增一个受登记约束的探测面所带来的审计与维护成本。

---

## 2. 约束与硬指标

1. **网络面登记制**（AGENTS §7 + ADR-0006）：`updates.rs` 是壳自身的 registry / 镜像链 /
   发布源 / 引擎引导网络面。ADR-0006 的 2026-08-27 边界重定义注明确：管理功能引入的网络
   **不属该 ADR 管辖**，但**"须单独评估网络面归属"**——本 ADR 即该评估。
2. **机器闸门不可绕过**：生产代码出现网络原语而未登记，`cargo test` 必红
   （`network_gate.rs:848 production_network_primitives_are_registered`）。判据是
   **"是否发起请求"而非"是否出现 URL"**；原语表见 `network_gate.rs:80-107`。
3. **子进程一律经 `lifecycle` seam**：AGENTS §6 —— "**新增 spawn 一律经
   `lifecycle::spawn`/`run`**，有机器闸门拦裸 `Command::spawn()`"；seam 在
   `lifecycle.rs:417` / `:465`，闸门 `lifecycle.rs:1876`。Windows 侧另须经
   `crate::child_cmd`（`lib.rs:70`），禁裸 `Command::new`。
4. **前端不发网络请求**（AGENTS §4.4 红线 2）：探测必须是 IPC 调用；前端经
   `lib/tauri.ts` 的 `api` 对象消费，组件内不直接 `invoke`（`ipc.rs:311` 有闸门）。
5. **探测面必须有界、只读、一次性**：单次超时（2s 量级）、只读（只调 `*/list`，绝不调
   `resources/read`、绝不写服务器状态）、一次调用一次快照，不做订阅、不缓存服务器数据。
6. **豁免必须条目级、集中登记**：豁免写在 `network_gate.rs` 的 `EXEMPTIONS` 表
   （`(文件, 条目, 种类, 理由, 日期)`，`:147-199`），不在别人文件里插内联标注；理由须含
   `AGENTS §7` 或 `ADR-`，日期 `YYYY-MM-DD`（`:798-823`）；且不得存在"登记后不再拦任何东西"
   的空豁免（`exemptions_are_live`，`:900`）。
7. **登记即约束**：`Kind::Registered` 的文件**不得**含 in-process 网络原语
   （`registered_entries_have_no_in_process_primitives`，`:931`）——这是"登记种类"反向绑定
   实现形态的机制，本 ADR 主动利用它（见 §3 方案 A）。
8. **可测**（AGENTS §5）：探测结果分类、超时、命令构造须为纯函数并有单测；不引测试依赖。
9. **双侧同构**（ADR-0016）：MCP 配置的读写已同时支持宿主与 WSL 客体，探测须给出同构口径
   （或显式声明某侧不支持及其理由）。

---

## 3. 备选方案及评估

### 方案 A：按 transport 分支——stdio 走子进程登记，streamable-http 走条目级豁免 —— ✅ 最终采纳

- 思路：探测入口先读该 MCP 行的 `config.transport`，据此走两条**互不相同**的合规通道：
  - **`stdio` 分支**：用 `crate::child_cmd` 构造命令，经 `lifecycle::spawn`/`run` 执行
    （满足 §2.3）；在 `network_gate.rs` 的 `EXEMPTIONS` 中为 `mcp.rs` 的这一**条目**加
    `Kind::Registered` 行（不授予任何权限，只登记）。利用 §2.7：该登记一旦存在，
    `registered_entries_have_no_in_process_primitives`（`:931`）就会在 `mcp.rs` 出现
    HTTP 客户端时**立刻变红**——等于把"这条探测只准走子进程"从口头约定升级为机器约束。
  - **`streamable-http` 分支**：进程内 HTTP 客户端访问用户配置的 `url`。这**会**触发
    §2.2 的闸门，因此需要 AGENTS §7 登记 **＋** `network_gate.rs` 中一条**条目级**
    `Kind::Exempt` 行（理由引用 AGENTS §7，带日期），并接受 `exemptions_are_live`（`:900`）的约束。
  - 两条分支共用同一超时策略（2s 量级）、同一只读口径、同一结果结构，对前端呈现为同一种卡片。
- 优点：合规含义与传输的真实语义一一对应，不需要为 stdio 伪造一条"网络"豁免；利用
  `Kind::Registered` 的反向绑定把实现形态钉死；与既有范式（复现点 11）同形。
- 代价/风险：`mcp.rs` 里同时存在两条合规通道，**必须**靠注释与测试把边界写死，否则
  后人加第三处会被误认为已豁免。缓解：`Kind::Exempt` 只给到**具体条目**，不给整个文件。
- 对照约束：§2.1 完成归属评估 ✅；§2.2 stdio 不触发、http 正确触发并登记 ✅；§2.3 走 seam ✅；
  §2.4 IPC ✅；§2.5 有界只读一次性 ✅；§2.6 条目级集中表 ✅；§2.7 主动利用 ✅；§2.8 纯函数可测 ✅；§2.9 双侧同构 ✅。

### 方案 B：统一走 `updates.rs` —— ❌ 否决

- 思路：（实施计划原文 §3.3.2/§5 的措辞）"探测网络请求严格通过 `src-tauri/src/updates.rs` 发起"。
- 否决理由：**违反 §2.1 与 AGENTS §2"`src/` 模块按职责命名"**。`updates.rs` 是**壳自身的**
  registry / 镜像链 / 发布源 / 引擎引导面（AGENTS §7:178-181 的登记用途清单），
  而 MCP 服务器是**用户在 `cordis.patch.yml` 里自填的任意端点**，属管理域。
  把管理域探测塞进壳运行时网络面，会让 `updates.rs` 从"一种职责"退化为"所有 HTTP 的垃圾桶"，
  并使 §2.5 的有界性无法在模块层面表达（该文件的既定用途都不是"用户自定义端点"）。
  附带：本方案的措辞本身自相矛盾——它同时把 stdio 子进程列为备选，而 stdio **根本不触网**。

### 方案 C：给 `mcp.rs` 整文件 `Kind::Exempt` —— ❌ 否决

- 思路：一次登记整个文件，省去逐条目维护。
- 否决理由：**违反 §2.6，且已有明确先例禁止**。`plugins.rs::fetch_runtime_snapshot` 的豁免
  在 2026-09-11 被**从整文件收窄为条目级**（`network_gate.rs:170-180`），理由原文即
  "整文件豁免会让**将来任何人往本文件加第二处触网都不被拦下**"。MCP 探测正是一个
  "将来很可能长出新触网点"的功能（例如新增传输、新增 `resources/read` 预览），
  采用整文件豁免等于预先放弃 §2.2 的防线。

### 方案 D：只支持 `stdio` 探测 —— ❌ 否决

- 思路：只实现子进程分支，`streamable-http` 一律返回"暂不支持"。
- 否决理由：**违反 §2.9 与 §1.4 的价值覆盖**。`streamable-http` 是 DSH 正式支持的传输
  （`transport.ts:23-38` 的 `switch` 只有两个分支，它占其一），且恰恰是**最容易配错**的一类
  （URL 拼错、headers 缺鉴权、内网不可达）——把最需要体检的对象排除在外，本功能的
  收益大半落空。若因工期需要分步，应作为**实现顺序**而非**范围裁剪**（见 §5 行动项）。

### 方案 E：不做——保持静态 patch 文件视图 —— ❌ 否决为唯一方案（保留为正当退出路径）

- 思路：不新增探测面，用户如需知道 MCP 暴露了什么，直接在会话里问模型（模型侧已有
  `list_mcp_resources` / `list_mcp_resource_templates` 等工具）。
- 否决理由：**它确实是一个严肃的备选，但不是最优**。其正当性依赖 §1.4——若维护者评审
  认为"零 token 体检 + 失败归因"不值一个新登记面，则本方案转为**采纳**，并应同时关闭
  `docs/roadmap.md:202` 的子项（明确记为"不做"而非"未实现"）。本 ADR 把它显式保留，
  以防半年后同一提案再排队（TEMPLATE §3 的要求）。
  **现状基线弱于方案 A 的根本原因**：模型路径无法在不消耗 token、不写会话日志的前提下
  给出"连不上是因为 command 不存在还是握手超时"这一层归因，而这正是配置排障的全部难点。

### 方案 F：把探测降级为"生成一段提示词让用户去问模型" —— ❌ 否决

- 思路：壳只负责拼一句"请列出 X 服务器的资源"，交给工作台执行。
- 否决理由：**违反 §2.5 的精神与 §1.4 的价值主张**。它消耗 token、结果非确定性、
  且失败时用户拿到的是模型对工具错误的转述而非结构化归因；更严重的是它把"配置体检"
  变成了"一次会话事件"，在用户只是改错一个 env 变量时成本过高。

---

## 4. 最终决策

采纳**方案 A**：MCP 探测的面归属**按 `transport` 分支**——`stdio` 一律作为**子进程**处理，
命令经 `crate::child_cmd` 构造、经 `lifecycle::spawn`/`run` 执行，并在 `network_gate.rs` 的
`EXEMPTIONS` 表中为 `mcp.rs` 的该**条目**登记 `Kind::Registered`（只登记、不授权，并借
`registered_entries_have_no_in_process_primitives` 反向绑定"本条目仅限子进程"）；
`streamable-http` 作为**进程内 HTTP 请求**处理，须先在 AGENTS §7 登记用途，再加一条**条目级**
`Kind::Exempt`（理由引用 AGENTS §7 ＋ 日期）。两条分支共用 2s 量级超时、只读、一次性快照口径，
探测结果经**新增 IPC 命令**回到前端（前端不发网络请求）。**明确不做**：不把探测放进
`updates.rs`、不给 `mcp.rs` 整文件豁免、不以"只支持 stdio"裁剪范围、不在探测中写服务器状态
或订阅变更。**前置依赖**：§1.4 的增量价值须在评审中获得维护者确认，否则本决策退回方案 E。

---

## 5. 后果与后续行动项

### 正面后果

- 关闭一个自 2026-08-27 起挂账的 roadmap 子项（`docs/roadmap.md:202`），且以机器可验证的
  合规形态落地，而非又一次"落地即漏登记"（`network_gate.rs:3-9` 记载的
  `boot.rs::authenticate_workbench_session` 漏登记事件正是本 ADR 要避免的重复）。
- 首次把"登记种类反向绑定实现形态"这一机制用于**新功能**（此前只用于事后收窄），
  为后续同类决策提供范式。
- 用户获得不消耗 token 的 MCP 配置体检能力，且失败可归因。

### 负面后果 / 新增债务

- `mcp.rs` 将成为**同时持有两条合规通道**的少数模块之一，可读性与审计复杂度上升；
  必须靠注释 + 闸门测试守住，属真实的新增债务。
- `streamable-http` 探测是**首次**由壳向用户自定义端点发起进程内 HTTP：这扩大了壳的
  信任边界（用户在配置里写的 `url` 会被壳真实访问）。**诚实登记**：本 ADR 不解决 SSRF 类
  风险，仅以"用户自己配的端点 + 只读 + 有界超时"限制影响面；如需策略（例如拒绝私网地址），
  另开工（AGENTS §8.1）。
- 探测结果**不是**能力的持续真相：一次快照不反映后续变更，UI 必须标注快照时间。
- WSL 客体侧的探测（§2.9）若第一步未覆盖，须在 UI 上明确标注"仅宿主可用"，不得静默降级。

### 行动项

- **范围声明（非行动项）**：本 ADR **不需要**修改 `docs/contract.md` 或升 `MANIFEST_FORMAT`——全部决策落在 `$DSH_HOME` 内的管理面，不触及「装配方 ↔ 产品壳」契约。
- [ ] 维护者评审 §1.4 增量价值；不通过则执行方案 E 并关闭 `roadmap.md:202` 子项
- [ ] AGENTS §7 登记"**MCP 连通性探测**（`mcp.rs`；stdio = `lifecycle` 子进程；
      streamable-http = 条目级豁免；2s 超时、只读、一次性快照）"＋ 日期
- [ ] `network_gate.rs` EXEMPTIONS 加两处：`mcp.rs` 探测条目的 `Kind::Registered` 与
      streamable-http 条目的 `Kind::Exempt`（理由引用 AGENTS §7，日期 YYYY-MM-DD）
- [ ] `ipc.rs::COMMANDS` 登记新命令（如 `probe_mcp_server`）→ AGENTS §7 登记 →
      `lib.rs` handler → `capabilities/default.json` 加 `allow-probe-mcp-server` →
      `frontend/src/lib/tauri.ts` 封装（4 处名字面由闸门 `ipc.rs:292` 兜底）
- [ ] `mcp.rs` 实现：transport 分支 + 纯函数化的命令构造与结果分类 + 超时
- [ ] `frontend/src/components/profiles/McpManager.tsx`：探测按钮 + 折叠能力卡片 + 快照时间标注
- [ ] `docs/contracts/dsh-behavior-ledger.md` 新增复现点行：MCP transport 取值集合、
      `*/list` 方法名、以及"DSH 无 MCP RPC"这一可行性前提（参照行 11 的写法）
- [ ] 单测：transport 分支选择、超时、错误分类、命令构造为纯函数（AGENTS §5）
- [ ] 实机验证清单（`docs/executor.md`）：stdio 服务器可探测、HTTP 服务器可探测、
      不可达时超时且不卡 UI、WSL 客体口径
- [ ] 广播落档 `docs/broadcasts.md`；AGENTS §9 索引行

---

## 5.1 实施补注（2026-09-15，append-only；不重写上文）

本节记录**落地时与上文两处偏差**与行动项状态。上文保持原样——ADR 是决策记录，
不是会随实现漂移的说明书；凡与代码冲突之处，以本节为准。

**偏差 1：文件名。** 上文写的 `mcp.rs` 只是**配置模型**（`McpServerConfig` /
`McpTransport`，2026-09-15 前置切片补入 `transport`/`url`/`headers`）。探测实现落在
**新模块 `src-tauri/src/mcp_probe.rs`**；`network_gate.rs` 的两条登记与登记册 §二
均指向 `mcp_probe.rs`。

**偏差 2：两条登记**并存**，而非"改为"**。§4 与行动项写"`Registered` → 条目级
`Exempt`"，实施时判为**不可依此执行**：正确动作是**保留**整文件 `Registered`
（其含义收窄为"除该豁免条目覆盖的范围外，本文件不得再有进程内原语"）**并新增**
`item: Some("post_rpc")` 的 `Exempt`（授权"恰好一处"）。依据是 `network_gate.rs`
该行自带注释与登记册 §二原文均写"**不得**只改那一行豁免"；两条并存严格强于二者取一
（只留 `Exempt` 会失去"第二处触网"的封堵）。计划文档 §7 的 R3 判据已同步更正。

**偏差 3：超时预算 2s → 15s。** §4 写"2s 量级"，实施取 **15s 整轮**（两分支同一
预算；http 分支的 5 次往返**共享一个 deadline**，不是每请求 15s）。理由是 MCP 服务器
冷启动 + 真实网络往返下 2s 实测过紧，会把"慢"误报成"连不上"——而误报正是本功能
要消灭的东西。「有界、不挂死 UI」这一实质要求未变；该数值已同步进登记册 §二与
`network_gate.rs` 的理由串。

**行动项状态**（截至 2026-09-15）：

- [x] §1.4 增量价值：随 ADR 评审通过（2026-09-15，维护者"评审都通过，接着做"）
- [x] 网络登记：两处（`Registered` 整文件 + `Exempt` 条目级）＋ 登记册 §二新增
      "进程内触网（条目级豁免）"一节
- [x] `ipc.rs::COMMANDS` 登记 `probe_mcp_server` → handler → capability → `tauri.ts`
      （4 处名字面由 `ipc.rs::gate_tests` 兜底）
- [x] transport 分支 + 纯函数化命令构造与结果分类（`assemble_probe` 由两分支**共用**，
      降级口径单源）
- [x] `McpManager.tsx` 探测按钮 + 折叠能力卡片；**快照时间标注**（§5 负面后果的硬要求）
      由 `ProbeCard` 表头 `fmtClock` 恒显
- [x] 复现点入册：台账**复现点 17**（实施 R3 时补登——ADR 落地 stdio 分支时漏写，
      已记入台账 §三）
- [x] 单测：两分支共用降级口径、`-32601` 降级、SSE/批量/NDJSON 响应形态、
      非 2xx 取信封、缺 `url` / 非 http(s) 拒绝
- [ ] **实机验证清单（`docs/executor.md`）**：stdio 服务器可探测 / HTTP 服务器可探测 /
      不可达时超时且不卡 UI / WSL 客体口径——**待人工执行**（Rust 单测不起真实 HTTP 服务，
      按仓库口径"真实网络走验证清单"）
- [x] 广播落档 `docs/broadcasts.md`

---

## 6. 复审条件

- **上游 MCP transport 集合变化**（新增第三种 transport，或移除某一种）→ 本 ADR 的
  二分前提消失，§3 与 §4 重开。
- **DSH 出现 MCP RPC / Remote 命名空间**（即壳可以"转发"探测而无需自己说协议）→
  方案 A 的子进程与豁免全部失去必要性，应退回"薄转发"并撤回两处登记。
- **`network_gate.rs` 的 PRIMITIVES 判据或 EXEMPTIONS 表结构变化**（例如新增 `npx`/`node`
  原语、或 `Kind::Registered` 语义调整）→ 复评 §2.2/§2.7 的落点。
- **`lifecycle` spawn seam 变更**（`lifecycle.rs:417/465` 签名或闸门口径变化）→ 复评 §2.3。
- **出现"探测被用于访问非用户预期端点"的安全反馈**（SSRF 类）→ 立即回到 §5 负面后果，
  评估引入端点策略。
- **探测功能上线后 3 个月内使用率极低**→ 按方案 E 撤回，撤回两处网络登记。
