# ADR-0021：已归档会话「取消归档」的实现路线

- **日期**：2026-09-15
- **状态**：**已接受**（2026-09-15 维护者裁定「认可」：认可 §1.5 的壳内自建入口理由——
  `SessionManager` 是独立 Tauri 窗口、够不到 Web 设置页，故据此采纳**路线 A**；
  §3 方案 C「不新增」的翻转条件**不予触发**，本 ADR 成立）
- **提出人**：guan（AI 协作起草）
- **相关方**：`src-tauri/src/sessions.rs` / `src-tauri/src/commands/session.rs` /
  `src-tauri/src/ipc.rs`（`COMMANDS`）/ `src-tauri/src/lib.rs`（`generate_handler!`）/
  `src-tauri/capabilities/default.json` / `frontend/src/lib/tauri.ts` /
  `frontend/src/types/{ipc.ts,ipc-shapes.json}` /
  `frontend/src/components/profiles/SessionManager.tsx`
- **关联**：ADR-0016（管理面跨环境一致，world 分派）· ADR-0006（网络面与镜像链）·
  ADR-0015（子进程生命周期 seam）· 复现台账行 11（typert 回环信封，
  `docs/contracts/dsh-behavior-ledger.md:26`）· `docs/roadmap.md` §4.6（工作区管理未关闭项）·
  `docs/plans/official-plugins-and-capabilities-plan-review.md`（审核报告 B4/B5）

---

## 1. 背景与问题

dsh `0.1.6-alpha.1` 引入了会话归档与恢复。dsh-dock 的 `SessionManager` 是桌面全局会话视图，
计划补齐"取消归档"能力。核查后发现：**上游能力、上游 UI、壳侧读路径三者都已存在，
真正缺的只是一个写动词**——而写动词有两条实现路线，其风险相差极大。

1. **上游动词真实存在且有溯源**：`WorkspaceRegistry.unarchiveSession(sessionId)`
   （`packages/workspace/workspace/src/index.ts:266`），由提交 `5f773a0ded`
   （"feat(workspace): restore archived sessions from a settings page"）引入，
   包含于 tag `dsh-v0.1.6-alpha.1`（注：该 tag 是本 checkout 中唯一的 0.1.6 tag，
   不存在 `dsh-v0.1.6` 正式发布 tag）。

2. **状态位置与形状**（已实测核对）：

   | 项 | 值 | 锚点 |
   |:---|:---|:---|
   | 路径 | `$DSH_HOME/storages/workspace.json` | 实测 `~/.dsh/storages/workspace.json` |
   | 键 | `global.archivedSessionIds` | `packages/workspace/workspace/src/spec.ts:55` |
   | 类型 | **数组**（非集合），zod `.default([])` | 同上 |
   | 域声明 | name `workspace`、version `2`、`single` 布局 | `spec.ts` 的 `workspaceDomainSpec` |
   | 文档形状 | `{ unit: { name, version }, global, tables }` | `packages/storage/storage-json/src/format.ts:29-35` |

   注：方案原稿写的 `$DSH_HOME/workspaces/` **不是真实路径**（该目录不存在）。

3. **壳侧读路径早已完成（2026-09-07 落地），并非本 ADR 的增量**：

   | 能力 | 既有实现 |
   |:---|:---|
   | 读 `global.archivedSessionIds` | `src-tauri/src/sessions.rs:502-516`（`read_archived_session_ids`，含设计注释与日期） |
   | `SessionItem.archived` 字段 | `sessions.rs:47-50` |
   | `list_sessions` 返回该字段 | `src-tauri/src/commands/session.rs:14-16`（序列化 camelCase） |
   | 前端筛选档 | `SessionManager.tsx:90` `useState<"all" \| "needs_repair" \| "archived">` |
   | 归档可见性口径 | `SessionManager.tsx:237-241`（默认隐藏、搜索跟随可见性） |
   | Tab / 徽章 / i18n | `SessionManager.tsx:651-663`、`:340-346`、`content/zh-CN.ts:548,572-573` |
   | 既有测试 | `sessions.rs:941`、`:860`、`:902` |
   | IPC 形状契约 | `frontend/src/types/ipc-shapes.json` 已含 `archived` |

   即：**方案的 P0-2 若表述为"新增归档筛选"，是在描述已完成的工作**；真实增量只有写动词。

4. **上游已自带取消归档 UI**（这一点改变了本 ADR 的备选结构）：
   `@deepseek-ai/dsh-client-ui-settings-unarchive-sessions` 由 `dsh-web-app` bundle
   **无条件挂载**（`packages/bundle/web-app/cordis.patch.yml:251-252`，无 `disabled`、无 `config`），
   其 `src/client/ArchivedSessionsSection.tsx` 即"已归档会话"列表＋**逐行取消归档按钮**，
   中英双语已就绪（`.../src/client/locales.ts:11,33`：`取消归档` / `Unarchive`）。
   而 dsh-dock 的 WebView 载入的正是这套 Web 工作台——**用户今天就能取消归档**。
   因此"壳不新增任何东西"是一个必须认真对待的真备选（见 §3 方案 C）。

5. **那为什么仍值得在壳内提供？** 唯一站得住的理由：dsh-dock 的 `SessionManager`
   是**独立的 Tauri 窗口**，不是 Web 工作台的一部分，**够不到 Web 设置页**。用户在壳内浏览
   会话列表（含"已归档"档）时，看到归档项却要切到工作台才能恢复，是交互断层。
   本 ADR 采纳此理由并据此决策；**若维护者不认可该理由，本 ADR 的结论翻转为方案 C**
   （不新增，UI 指向 Web 设置页）。这一翻转条件写在 §3 与 §6。

6. **方案原稿的实现有三处问题**（本 ADR 逐一修正）：
   - 签名 `unarchive_session(session_id: String) -> Result<bool, String>` **缺
     `app: tauri::AppHandle`**——本仓库全部会话命令都带它，因为 ADR-0016 的宿主/客体分派
     依赖它（`commands/session.rs:14-16` 起，`let world = crate::mgmt::current_world(&app)?;`）。
     无 `AppHandle` 即无法解析 WSL 客体世界。
   - 只提"三处同步"，实际机器闸门覆盖**四名字面 ＋ 一形状面**（见 §2.5）。
   - 把实现默认成"直接改状态"，未论证并发与格式风险（见 §1.7）。

7. **路线 B（直接改写文件）的风险是可证的具体风险，不是理论担忧**：

   | 风险 | 证据 |
   |:---|:---|
   | 文件是内存权威态的投影，运行中的 Host 会**整体覆盖**外部改动 | `packages/storage/storage-json/src/single-unit.ts:1-5`："**The in-memory state is authoritative**; every write primitive mutates it and republishes the whole file atomically"；`format.ts:5`："the file is always the current net state" |
   | 上游把"单写者 + last-write-wins"当作**正确语义**而非缺陷 | `packages/storage/storage-json/src/atomic.ts:1-10`："a unit file has **exactly one writer per process and last-write-wins is correct**" |
   | 文件带**受校验的版本头**，写坏即整个工作区功能报错 | `format.ts:63` `missing or foreign unit header`、`format.ts:68` `version-mismatch`（`unit.name` 不符或 `version` 不匹配即 `StorageError`） |

   折算成工程后果：文件改写会引入**一个新的 dsh 存储域写面**（今天该文件在仓库内严格只读，
   生产代码唯一触碰点是 `sessions.rs:509`），并且需要一个"当前无活跃 Host"的前置判定——
   即把"取消归档"从一次幂等的数组过滤，升级为一个带竞态与格式约束的写操作。

## 2. 约束与硬指标

1. **ADR-0016 跨环境一致**：命令必须带 `app: tauri::AppHandle`，按 `current_world` 分派
   `Local` / `Wsl`；不得只实现宿主侧。
2. **AGENTS §7 网络面登记制**：只有 `updates.rs` 是唯一网络面，其余模块触网须**先登记**。
   路线 A 的回环调用属已登记用途的同通道延伸（台账行 11 `:26` 的
   `POST http://127.0.0.1:<port>/api/<method>`，信封
   `{type:"client-request",rpcId,method,payload:{args:{}}}`），**须在 AGENTS §7 登记该新用途**，
   并按 `network_gate.rs` 的 `EXEMPTIONS` 表口径处理豁免；壳**不得新增网络客户端**。
3. **AGENTS §6 写入例外册**：**路线 A 不新增任何写面**（这是它的核心收益）；路线 B 则必须
   新增写入例外登记 ＋ 台账复现点行。
4. **dsh 文件系统不变量**（AGENTS §6）：不得破坏存储域文档格式；`unit` 头与 `tables` 必须逐字保留。
5. **IPC 闸门完整性**：四名字面 ＋ 一形状面缺一即 `cargo test` 红（详见 §4）。
6. **AGENTS §8.2 禁止超范围改动**：既有筛选分类法 `all | needs_repair | archived`
   （`SessionManager.tsx:90`）与徽章/i18n 已稳定，**本 ADR 明确不改**。
7. **红线 1**：不修改 dsh 源码；只调既有 RPC、CLI 与文件系统。
8. **有界且只读优先**：探测类调用的既有范式是有界超时（~2s 级）、一次性、不订阅
   （比照 `plugins.rs::fetch_runtime_snapshot` 的回环只读查询口径）。

## 3. 备选方案及评估

### 方案 A：复用 Host RPC（typert 回环） —— ✅ 最终采纳

- **思路**：`unarchiveSession` 除服务方法外**已是 Remote 方法**——
  `packages/api/workspace-controller/src/index.ts:35,43` 的
  `WorkspaceController extends TypertRemoteService` 以 `{ namespace: 'workspace' }` 注册，
  `:118-121` 标注 `@Remote('unarchiveSession')`；上游 Web 设置页走的正是这条路。
  dsh-dock 已经使用同一条回环通道（`plugins.rs::fetch_runtime_snapshot`，台账行 11 `:26`），
  故 `unarchive_session` 只需按该信封发一次 unary 调用：有界超时、一次性、
  无活跃 Host 时**如实提示"请先启动会话"**而不是绕路改文件。
- **优点**：①不引入新写面；②不耦合存储格式与版本头；③不存在 last-write-wins 竞态；
  ④**无需新增 AGENTS §6 写入例外登记**；⑤与上游 UI 行为完全一致（同一动词、同一语义）；
  ⑥天然继承 ADR-0016 的世界一致性（调用发生在实际运行的那个世界）。
- **代价/风险**：必须有活跃 Host（与既有回环查询前置一致）；回环无鉴权门
  （台账行 11 实测"伪造 Host 仍 200"），故该命令**只能作为用户显式动作**触发，
  不得被其它流程隐式调用；新增一处网络用途须登记。
- **对照约束**：①带 `AppHandle` 并按 world 选择回环端点 ✔；②登记后合规、壳无新客户端 ✔；
  ③无新写面 ✔；④不触碰存储文档 ✔；⑤补齐四名字面 ＋ 形状面 ✔；⑥不动既有筛选 ✔；
  ⑦只调 RPC ✔；⑧超时有界、一次性 ✔。

### 方案 B：直接改写 `$DSH_HOME/storages/workspace.json` —— ❌ 否决

- **思路**：壳在文件层把目标 id 从 `global.archivedSessionIds` 数组滤除后整体写回，
  与既有的 `sessions.rs` 只读解析共用同一路径认知。
- **否决理由**：违反约束 3 与约束 4。具体地：①运行中的 Host 会把外部改动**整体覆盖**
  （`single-unit.ts:1-5` 内存权威态；`atomic.ts:1-10` 明确 last-write-wins 为既定语义），
  用户会看到"我点了取消归档，过一会儿又回来了"，且无法归因；②文件带受校验的版本头
  （`format.ts:63,68`），格式写坏会让**整个工作区功能**报错，代价远大于收益；
  ③引入一个全新的 dsh 存储域写面，需要新增 AGENTS §6 登记与台账行。
  **若未来因某种原因必须走此路**，最小安全条件是全套装：仅在无活跃 Host 时执行；
  写前校验 `unit.name === 'workspace' && unit.version === 2`；只改
  `global.archivedSessionIds` 并逐字保留其余字段；沿用 `fs_backup.rs` 的覆写前备份
  ＋ 原子替换；并补齐上述两项登记。本 ADR 不采纳该路线，但保留此清单以防备未来重开。

### 方案 C：不新增，UI 指向 Web 设置页 —— ❌ 否决（附翻转条件）

- **思路**：上游已有 `ui-settings-unarchive-sessions`（`web-app/cordis.patch.yml:251-252`）
  与逐行取消归档按钮，壳只需在归档项上给一句"请到 Web 设置页恢复"。
- **否决理由（条件性）**：`SessionManager` 是独立 Tauri 窗口，**够不到** Web 设置页，
  指向它等于把用户推去另一个界面完成本可在原地完成的动作。
  **翻转条件**：若维护者不认可 §1.5 的这条理由，本 ADR 的结论改为采纳方案 C
  （不新增 IPC，仅补一句引导文案），届时 §4 的 IPC 契约与 §5 的相应行动项整体作废。
  记录于此以防半年后重复讨论。

### 方案 D：起一次性 `dsh` 子进程来触发 —— ❌ 否决

- **思路**：不依赖活跃 Host，另起一个 dsh 进程执行某个命令来落地取消归档。
- **否决理由**：违反约束 4 与约束 8。上游**没有**暴露该操作的 CLI 子命令
  （headless 子命令面只有 `--json` / `--session-id` / task，
  `packages/bundle/headless/src/startup.ts:38-43`，不含归档类动词）；为一次数组过滤
  起一个完整运行时属于重量级绕行，且新进程自己会持有该存储域，反而制造第二个写者
  ——与约束 4 直接冲突。

### 方案 E：读时顺带取消归档（隐式写） —— ❌ 否决

- **思路**：在 `list_sessions` 扫描到归档项时顺手把它们移出归档集合，省掉一次显式操作。
- **否决理由**：违反约束 8 与 §2.6 的精神。归档是**用户意图**而非脏数据，读操作必须有
  零副作用；隐式改写会让"我明明归档了"变成不可复现的状态，且把一次只读扫描变成写操作，
  在 WSL 客体路径上还会额外引入跨环境写。

## 4. 最终决策

**`unarchive_session` 采用方案 A：复用 Host RPC。** 命令签名带
`app: tauri::AppHandle` 并按 `current_world` 分派，经既有 typert 回环通道
（`POST http://127.0.0.1:<port>/api/<method>`，信封
`{type:"client-request",rpcId,method,payload:{args:{}}}`，台账行 11）向命名空间
`workspace` 的 `unarchiveSession` 发一次**有界超时、一次性、不订阅**的调用；
**不触碰 `$DSH_HOME/storages/workspace.json`**；无活跃 Host 时如实提示而非降级改文件。

IPC 契约（必须完整落地，缺一即 `cargo test` 红）：

| 面 | 落点 | 闸门 |
|:---|:---|:---|
| 名字面 1 | `src-tauri/src/ipc.rs` 的 `COMMANDS` 增加 `"unarchive_session"` | `handler_matches_ipc_commands` / `capabilities_match_ipc_commands` |
| 名字面 2 | `src-tauri/src/lib.rs` 的 `generate_handler!` 注册 | 同上 |
| 名字面 3 | `src-tauri/capabilities/default.json` 增 `"allow-unarchive-session"` | 同上（缺失则 remote 页被 ACL 静默拒绝） |
| 名字面 4 | `frontend/src/lib/tauri.ts` 的 `api` 封装 | `ipc.rs:292 tauri_ts_matches_ipc_commands` |
| 形状面 | 若新增跨 IPC 结构体，同步 `frontend/src/types/ipc-shapes.json` | `ipc.rs:381 ipc_struct_shapes_match_fixture` |
| 登记 | AGENTS §7 登记该回环用途 | 人工 + 评审 |

命令计数 **55 → 56**（当前 `COMMANDS` 实测 55 条）。既有的
`all | needs_repair | archived` 筛选分类法**不改**。

## 5. 后果与后续行动项

### 正面后果

- 壳成为"取消归档"的完整闭环（读路径 2026-09-07 已完成，本 ADR 补齐写动词）。
- **零新增写面**：`storages/workspace.json` 在壳内继续保持严格只读，AGENTS §6 无需新增条目。
- 与上游同一动词、同一语义，行为与 Web 设置页一致，不产生第二套归档观。
- 无竞态：不与运行中的 Host 争夺同一存储域。

### 负面后果 / 新增债务

- **依赖活跃 Host**：无 Host 时命令无法完成（用户须先启动会话）。这是与既有回环查询
  相同的既定前置，但本命令是**写**语义，用户预期更强，文案须明确。
- **回环无鉴权**（台账行 11 实测）：该命令不得被其它流程隐式调用，只能由用户在
  `SessionManager` 显式触发；这一点须在实现与评审中把住。
- **新增一处网络用途**，须登记；`network_gate.rs` 的豁免口径需按条目级（而非整文件）
  处理，理由沿用 2026-09-11 收窄 `plugins.rs` 豁免时的同一条：
  整文件豁免会让将来任何人往该文件加第二处触网都不被拦下。
- 若维护者翻转至方案 C，本 ADR 的 IPC 契约与前端行动项整体作废（见 §3 方案 C）。

### 行动项

- [ ] **Rust 命令**（`src-tauri/src/commands/session.rs`）：新增 `unarchive_session`，
  签名带 `app: tauri::AppHandle`，按 `current_world` 分派；WSL 分支走客体回环端点。
  参照 `list_sessions`（`:14-16`）的既有形态，不引入新的错误类型（维持 `Result<_, String>`）。
- [ ] **回环调用**（`src-tauri/src/plugins.rs` 或就近模块）：实现一次性 unary 调用，
  ~2s 级超时、不订阅、失败分类清晰（Host 未运行 / 会话不存在 / 其它）；
  参照 `fetch_runtime_snapshot` 的既有实现与台账行 11 的信封约定。
- [ ] **IPC 五面同步**：`ipc.rs::COMMANDS` → `lib.rs` handler →
  `capabilities/default.json` 的 `"allow-unarchive-session"` → `frontend/src/lib/tauri.ts`
  封装 →（如有新结构体）`frontend/src/types/ipc-shapes.json`；运行 `cargo test` 确认闸门全绿。
- [ ] **AGENTS §7 登记**：登记该回环用途（含超时与"仅用户显式动作"限制）；
  同步 `network_gate.rs` `EXEMPTIONS` 的条目级处理。
- [ ] **前端**（`SessionManager.tsx`）：仅在"已归档"档（`statusFilter === "archived"`）的
  行上提供"取消归档"操作；成功后**不走乐观更新**——以一次 `list_sessions` 重取为准
  （归档集合是服务端权威态）；Promise 必须 `.catch`（AGENTS §4.3）。
- [ ] **无 Host 文案**：新增用户可读提示（集中 `content/zh-CN.ts`，同步 `en-US.ts`），
  语义为"请先启动会话后再取消归档"，**不得**提供"直接改文件"的降级路径。
- [ ] **测试**：Rust 侧补"信封构造正确"与"失败分类"纯函数单测；
  前端补归档档可见性/操作可用性的纯逻辑测试（Vitest 只测纯逻辑，不引 RTL/jsdom）。
  真机验证项（活跃 Host 下定取消归档、无 Host 时提示）写入 `docs/executor.md` 验证清单。
- [ ] **回归护栏**：补一条测试钉死"壳内生产代码不得写 `storages/workspace.json`"
  （与既有 `network_gate` / spawn 闸门同风格的源码扫描），把本 ADR 的否决结论变成机器约束。
- [ ] **roadmap 收口**（文档）：`docs/roadmap.md` §4.6 的"工作区增删管理"未关闭项按本 ADR
  范围说明（本 ADR 只覆盖归档恢复，不含工作区增删）。
- [ ] **审核报告回写**（文档）：`docs/plans/...-plan-review.md` 的 B4/B5 以本 ADR 为结论收口；
  同时移除方案原稿中不实的"归档筛选待开发"表述。
- [ ] **范围声明**（文档）：记录本 ADR **不需要**修改 `docs/contract.md` 或升 `MANIFEST_FORMAT`。

## 6. 复审条件

- **维护者不认可 §1.5 的理由** → 立即翻转为方案 C（不新增，UI 指向 Web 设置页）。
- dsh 升级改变 `workspace` 域的 Remote 命名空间/动词名、或把该动词从 Remote 面移除。
- dsh 升级改变 `archivedSessionIds` 的类型或存储域 `version`（现为 2），
  或移除 `unit` 头校验——届时应连同 `sessions.rs:502-516` 的读解析一并复核
  （该文件是台账复现点，须随升级逐条对账）。
- 上游移除或在架构上替换 `ui-settings-unarchive-sessions`（`web-app/cordis.patch.yml:251-252`）
  ——若上游不再提供该 UI，方案 C 的前提消失，本 ADR 的自有价值上升。
- 出现"壳需要在无 Host 时完成归档恢复"的明确用户诉求 —— 届时重开路线 B 的评估，
  并按 §3 方案 B 的最小安全条件全套落地。
- `network_gate.rs` 的豁免口径变化（例如取消条目级豁免粒度），或 AGENTS §7 网络面
  登记制被修订。
