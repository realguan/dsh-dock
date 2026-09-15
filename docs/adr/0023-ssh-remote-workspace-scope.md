# ADR-0023：SSH 远程工作区的适用范围与边界（Profile 向导）

- **日期**：2026-09-15
- **状态**：**已接受**（2026-09-15 维护者裁定「评审通过」，含确认 §3 方案 C 的否决——
  即接受「Web 视图不随 provider 变远端感知」这一范围裁剪）——I4 增量据此解冻
- **提出人**：guan（AI 协作起草）
- **相关方**：`src-tauri/src/profiles.rs`（向导生成 profile 的落点）、`src-tauri/src/executor.rs`
  （既有 `ExecutorKind::Ssh` 预留位）、`src-tauri/src/ipc.rs`（新命令登记）、
  `src-tauri/capabilities/default.json`、`frontend/src/components/profiles/ProfileCreateDialog.tsx`、
  `frontend/src/lib/tauri.ts`、`docs/executor.md`（实机验证清单）
- **关联**：AGENTS §0 红线 1（不修改 dsh 源码）/ §4.3 / §4.4 红线 2 / §7 / §9；
  **ADR-0009**（profile 生命周期：创建走 `dsh plugin` 转发链，其余文件层）；
  **ADR-0016**（宿主 / 客体管理面同构，`World::Local | World::Wsl`）；
  **ADR-0020**（patch 行的 `insert` 挂载 / 浅覆盖 / 稳定 `id` 语义，本 ADR 直接沿用）；
  `docs/roadmap.md:204-209`（§4.8 SSH 远程执行器，含会话级 capability 收敛要求）；
  `docs/contracts/dsh-behavior-ledger.md`（新增复现点登记）；
  `docs/plans/official-plugins-and-capabilities-plan-review.md` §B7

---

## 1. 背景与问题

### 1.1 需求

在 Profile 创建流程中提供一个「Remote SSH」模板：读取本地 `~/.ssh/config` 供用户挑选 host，
校验远端环境，并生成一个可直接启动的远程工作区 profile。
### 1.2 现状

- 上游提供 POSIX SSH 提供方**家族**，共四个包，分别对应四个 context 服务
  （`packages/ssh/README.md:23-28`）：

  | 包 | 服务 | 职责 |
  |:---|:---|:---|
  | `@deepseek-ai/dsh-ssh` | `ctx.ssh` | 连接、helper 身份与传输生命周期 |
  | `@deepseek-ai/dsh-fs-ssh` | `ctx.fs` | 远端文件标识、读取与受保护的原子修改 |
  | `@deepseek-ai/dsh-subprocess-ssh` | `ctx.subprocess` | 可执行查找、进程、控制流与终端 |
  | `@deepseek-ai/dsh-sandbox-ssh` | `ctx.sandbox` | 远端文件效果隔离与执行事实 |

- 壳侧**已有一个预留位但未启用**：`src-tauri/src/executor.rs:44-52` 的
  `ExecutorKind` 已含 `Ssh` 变体（`as_str()` 返回 `"ssh"`），模块文档
  （`executor.rs:3`）写明"local（本机子进程）/ wsl（WSL2 发行版）/ **ssh（预留）**"；
  `boot.rs:127` 同样注释"local / wsl；**ssh 预留**"。但
  `settings.rs:162` 的测试断言 `Mode::parse("ssh") == None`——即**尚无任何路径能真正选中它**。
- 壳侧**没有任何 `~/.ssh/config` 读取**：`grep -rn "\.ssh" src-tauri/src/` 零命中
  （`grep -rn "ssh" src-tauri/src/` 有 10 处命中，但全部是上述预留位注释、`Ssh` 变体、
  以及插件测试夹具里 pnpm 包名 `ssh2`——**没有一处读取用户的 SSH 配置**）。

### 1.3 上游范围限定（本 ADR 的核心矛盾）

上游明确把该家族的适用范围限定为 **POSIX headless 与自建（custom）profile**：

- `docs/subsystems/ssh.md`（Composition scope）：
  > Web workspace views that assume host filesystem access need separate integration;
  > replacing providers alone does **not** make those views remote-aware.
- `packages/ssh/ssh/README.md:93` 与
  `.agents/notes/implemented/architecture/2026-09-11-posix-ssh-runtime.md:45`
  同口径："The initial composition scope is **POSIX headless and custom profiles**."

**含义**：把 SSH 提供方装进 `web` profile **不会**让 Web 工作台的文件树、编辑器、
终端自动指向远端——那些视图假定宿主本地文件系统。因此实施计划里"用户在启动器中直接选择
该 Profile 启动，**透明进入远程开发模式**"这一承诺**当前上游不成立**。

### 1.4 部署前置条件（比"探测 node 是否 ≥ 20"重得多）

- **远端须预装 helper 及其匹配的运行时依赖**，且 helper 与依赖必须留在
  **workspace 与可写临时根之外**（还须避开 bwrap 私有 `/tmp` 这类后端替换树），
  并用 SHA-256 核验——摘要不符即**拒绝连接**。
- **仅非交互**：服务启用 `BatchMode`、强制严格 host-key 校验、
  **禁用 agent forwarding**、不提供任何交互式认证流程。
- **平台范围**：`packages/ssh/ssh/README.md` 明写 "Both endpoints require Linux or macOS"。

### 1.5 无上游范本

上游**不存在任何 ssh 预设或示例**：六个 bundle patch
（`packages/bundle/{base,web-app,headless,sdk-app,sdk-minimal,acp-app}/cordis.patch.yml`）、
全部 agent preset、以及 `apps/cli/config/examples/`（仅 `cordis` / `github-review` /
`mcp-memory` / `schedule`）均无 ssh 行；唯一字面量出现在测试夹具与生成的 `docs/config-catalog.md`
中。**dsh-dock 将是首个落地者**，没有可抄的模板。

---

## 2. 约束与硬指标

1. **不修改 dsh 源码**（AGENTS §0 红线 1）：只能通过文件层与 CLI 使用上游能力。
2. **必须四包齐注册**：`dsh-ssh` 只提供 `ctx.ssh`；缺少其余三个则没有文件、进程与沙箱服务。
   依赖边为：`fs-ssh/src/index.ts:21`（`inject = ['ssh', 'sandboxPolicy']`）、
   `subprocess-ssh/src/index.ts:230`（`inject = ['ssh']`）、`sandbox-ssh/src/index.ts:11`
   （`inject = ['ssh']`）。`packages/ssh/ssh/README.md:28` 原文：
   "**Compose this service with** `fs-ssh`, `subprocess-ssh` and `sandbox-ssh`"。
   注意"provider family"只是**文档词汇**（`packages/ssh/README.md:2` 的
   `kind: package-group`），**不存在**伞包或 meta 包——四个包各自独立注册，
   且**只有 `dsh-ssh` 接受 `config`**（其余三个 README 明写无配置）。
3. **配置必须是完整的五个必填键**（`packages/ssh/ssh/src/index.ts:17-40,48-54`）：
   `host`（**已存在的 OpenSSH alias**）、`node`、`helper`、`workspace`、`helperHash`；
   可选配对 `bootstrapPath` / `bootstrapHash`（仅 PTC，须成对出现，
   `index.ts:76-84` 的 `refine` 强制），以及调参
   `requestTimeoutMs`=30000、`maxFrameBytes`=64 MiB、`maxPending`=128、`leaseMs`=30000。
   运行时校验**严于** schema：`node`/`helper`/`workspace` 必须是绝对 POSIX 路径
   （`startsWith('/')`）、`helperHash` 必须是小写 64 位十六进制 SHA-256
   （`/^[0-9a-f]{64}$/`）、`host` 须匹配 `/^[a-zA-Z0-9][a-zA-Z0-9_.@-]*$/`。
   **不存在** `target` 或 `remoteNodePath` 选项。
4. **每个生成的插件行必须带稳定 `id`**：loader 按 `id` 建立 store
   （`vendor/loader/src/config/tree.ts:51-59`），缺 `id` 会被自动生成，该行**此后永远无法被
   patch 命中**（patch 挂载与浅覆盖语义见 **ADR-0020**，本 ADR 不重复论证）。
5. **必须非交互预检**：向导第一步须以 `ssh -o BatchMode=yes` 证明可达；不可达即**拒绝生成**
   profile 并给出明确指引，不得生成一个注定启动失败的 profile。
6. **读取 `~/.ssh/config` 必须在 Rust 后端**：这是本仓库**首次**读取
   `$DSH_HOME` / `app_data_dir()` 之外的路径。前端发起即违反 AGENTS §4.4 红线 2
   （前端运行时禁止发起新网络请求）与 §4.3（组件内不直接 `invoke`，须经 `lib/tauri.ts`）。
7. **新文件系统域须登记**：`~/.ssh/config` 的读取范围须在 AGENTS §7（或等价的显式授权）
   登记，并由本 ADR 背书。
8. **必须尊重上游范围限定**：不得对 `web` profile 承诺"远端感知"的 Web 视图。
9. **继承 roadmap §4.8 的安全要求**：`docs/roadmap.md:208` 明写"需要设计**会话级
   capability 收敛**（远端会话拒绝 upgrade 类动作，`docs/executor.md` 已标注安全边界）"。
   同时 `:209` 指出"SSH 配置管理（保存 host/user/port）属于设置扩展，需要配置面板"。
10. **无上游范本**（§1.5）→ 必须以风险声明 + 实机验证清单替代"照抄模板"的确定性。
11. **可测**（AGENTS §5）：`~/.ssh/config` 解析须为纯函数并有单测（含畸形输入反例），不引测试依赖。
12. **平台差异必须显式**（AGENTS §1）：与三平台 CI 口径对齐，`#[cfg]` 分叉须覆盖编译目标语义。

---

## 3. 备选方案及评估

### 方案 A：面向 headless / 自建 profile 的 SSH 向导 —— ✅ 最终采纳

- 思路：向导产出一个**自建 profile**（例如 `remote-dev-box`），其 `cordis.patch.yml`
  用 `insert` 形式注册**四个 ssh 包**（只有 `dsh-ssh` 带完整五键 `config`，四行各带稳定 `id`）。
  `~/.ssh/config` 的解析在 Rust 侧完成并经 IPC 供前端下拉；向导首步做
  `ssh -o BatchMode=yes` 可达性预检 + 远端 `node`/helper/workspace 与 `helperHash` 校验，
  任一项不通过即拒绝生成并给出可执行的修复指引。UI 文案明确标注"**远程开发经 headless /
  自建 profile 使用**"，且**不承诺** Web 工作台视图会变为远端感知。
- 优点：与上游声明的适用范围一致，因此是**唯一真正可用**的形态；不引入与上游语义对抗的
  期望；四包齐注册使"连接 + 文件 + 进程 + 沙箱"一次到位，不会产出空壳 profile。
- 代价/风险：只覆盖 headless 场景，用户面较窄（`roadmap.md:209` 已预判"用户覆盖面较窄"）；
  远端 helper 部署与哈希核验是本功能的真正门槛，向导必须扛住这层复杂度；
  **无上游范本**可抄（§1.5），需以实机清单兜底。
- 对照约束：§2.1 只读上游 ✅；§2.2 四包齐注册 ✅；§2.3 五键完整 ✅；§2.4 稳定 `id` ✅；
  §2.5 非交互预检 ✅；§2.6 读取在 Rust ✅；§2.7 登记 ✅；§2.8 不越范围 ✅；
  §2.9 收敛与设置面板列为行动项 ✅；§2.10 风险声明 + 实机清单 ✅；§2.11 纯函数可测 ✅；§2.12 显式平台差异 ✅。

### 方案 B：只注册 `@deepseek-ai/dsh-ssh` —— ❌ 否决

- 思路：只挂连接包，用户需要文件/进程能力时自行再加。
- 否决理由：**违反 §2.2**。`dsh-ssh` 只提供 `ctx.ssh`；没有 `fs-ssh` 就没有 `ctx.fs`，
  没有 `subprocess-ssh` 就没有 `ctx.subprocess`，没有 `sandbox-ssh` 就没有 `ctx.sandbox`——
  产出的是一个**连文件都读不了的远程 profile**，用户会在启动后才撞上缺失服务的错误。
  这与实施计划 §3.4.2 的 YAML 示例（只写了一个 `@deepseek-ai/dsh-ssh` 行）是同一个错误。

### 方案 C：支持 `web` profile 并承诺"透明进入远程开发模式" —— ⛔ 否决（**本 ADR 最重要的一条**）

- 思路：向导产出 web profile 的 SSH 变体，用户选它启动即得到"远程版工作台"。
- 否决理由：**违反 §2.8，且与上游明文声明的范围直接冲突**。上游三处（`docs/subsystems/ssh.md`
  的 Composition scope、`packages/ssh/ssh/README.md:93`、
  `.agents/notes/implemented/architecture/2026-09-11-posix-ssh-runtime.md:45`）一致说明：
  替换 provider **不会**让假定宿主文件系统的 Web 视图变为远端感知，那些视图需要**单独的集成工作**。
  照此实施的结果是"用户按承诺启动、然后发现文件树指向本地"，比不提供该功能更糟。
  故本 ADR **显式拒绝**这一承诺，并要求把它从实施计划中删除或降级为"面向 headless"。

### 方案 D：由前端解析 `~/.ssh/config` —— ❌ 否决

- 思路：WebView 侧直接读文件并渲染下拉。
- 否决理由：**违反 §2.6**。前端运行时不得发起新网络/文件请求，且组件内直接 `invoke`
  会被闸门拦下（`ipc.rs:311 no_direct_invoke_outside_tauri_ts`）。此外该文件承载用户的
  主机名、用户名、端口、密钥路径等敏感信息，让其穿过 WebView 边界属无谓的暴露面扩大。
  正确落点是 Rust 侧解析 + IPC + `lib/tauri.ts` 封装，且解析结果只回传 UI 需要的字段。

### 方案 E：从上游复制一个 ssh 预设/示例作为起点 —— ❌ 否决（不可能）

- 思路：找一个上游现成的 ssh profile 模板改一改。
- 否决理由：**违反 §2.10 的前提不成立**——上游**不存在**这样的模板（§1.5 已逐处核查：
  六个 bundle patch、全部 agent preset、`apps/cli/config/examples/` 全部无 ssh 行）。
  本方案不是"不选"，而是"不存在"，故必须把"无范本"作为**已登记的风险**写入 §5，
  并以实机验证清单承担原本由模板承担的确定性。

### 方案 F：只做"SSH 配置存储"（保存 host/user/port），不做向导与校验 —— ❌ 否决

- 思路：按 `roadmap.md:209` 的字面表述，只把 SSH 配置作为设置扩展存起来。
- 否决理由：**违反 §2.5 与 §2.3**。仅存配置而不校验，用户得到的是一个**必然失败**的
  profile（缺 `helper` / `helperHash` 会被 schema 直接拒绝，或哈希不符被拒绝连接），
  且失败发生在 dsh 启动期而非向导期，排障成本被推给用户。存储可以保留为向导的副产物，
  但不能作为独立交付物。

---

## 4. 最终决策

采纳**方案 A**：dsh-dock 提供面向 **headless / 自建 profile** 的 SSH 远程工作区向导——
`~/.ssh/config` 的**解析在 Rust 后端**（首次读取 `$DSH_HOME` / `app_data_dir()` 之外的路径，
经 IPC + `lib/tauri.ts` 供前端使用），向导以 `ssh -o BatchMode=yes` 做**非交互可达性预检**，
校验远端 `node` / helper / workspace 与 `helperHash`，通过后生成一个**自建 profile**，
其 `cordis.patch.yml` 以 `insert` 形式注册**四个 ssh 包**（仅 `dsh-ssh` 带完整五键 `config`，
每行带稳定 `id`）。**明确不做**：不为 `web` profile 承诺"透明进入远程开发模式"（上游三处
明文排除该形态）、不只注册 `dsh-ssh`、不在前端解析 `~/.ssh/config`、不假设存在上游模板、
不交付未经校验的"仅存配置"形态。本决策同时继承 `roadmap.md:208` 的会话级 capability 收敛
要求（远端会话须拒绝 upgrade 类动作）与 `:209` 的配置面板要求，二者作为行动项而非本 ADR 的
范围外事项。

---

## 5. 后果与后续行动项

### 正面后果

- 壳首次接通一个"把执行面移到远端"的能力，且**与上游语义一致**，不会制造无法兑现的期望。
- 填补 `executor.rs` 中 `ExecutorKind::Ssh` 这一**既有预留位**：`settings.rs:162` 当前断言
  `Mode::parse("ssh") == None`，本决策是让该断言有条件翻转的第一步（模式解析的放开本身
  仍须单独评审，见 §6）。
- roadmap §4.8 从"未开工"进入"有明确范围与边界的可实施状态"，且范围比原文更窄、更诚实。

### 负面后果 / 新增债务

- **用户面狭窄**（`roadmap.md:209` 已预判）：只服务 headless 场景，Web 用户拿不到该能力，
  需在 UI 上明确说明，否则会被当作"功能没做完"。
- **无上游范本（§1.5）**：本功能没有可对照的正确实现，正确性完全依赖自建实机清单。
  这是本仓库首次在"上游无先例"的情况下落地一个运行时能力，属真实风险。
- **Windows 宿主不可用**（§1.4）：两端须 Linux/macOS，而本仓库的 CI 是三平台
  （AGENTS §1）。这意味着**同一条功能在不同平台上的可用性不同**，必须在 UI 与文档中显式，
  且 CI 无法覆盖 Windows 上的"不可用"路径（只能覆盖其分支编译）。
- **新增文件系统读取域**：`~/.ssh/config` 是首个 `$DSH_HOME` 之外的读取面，扩大了壳的读取
  边界；须在 AGENTS §7 登记，并在实现中只取必要字段、不回传密钥内容。
- **远端 helper 的部署复杂度转嫁给用户**：向导只能校验与指引，无法替用户完成远端预装。
- 会话级 capability 收敛（§2.9）尚未设计，本 ADR 只继承该要求，**不解决它**。

### 行动项

- **范围声明（非行动项）**：本 ADR **不需要**修改 `docs/contract.md` 或升 `MANIFEST_FORMAT`——SSH 工作区是 `$DSH_HOME` 内的 profile/管理面能力，不触及「装配方 ↔ 产品壳」契约。
- [ ] 维护者确认 §3 方案 C 的否决（即接受"Web 视图非远端感知"这一范围裁剪）
- [ ] AGENTS §7 登记"**读取 `~/.ssh/config`**（向导 host 选择；Rust 后端、只取必要字段）"
- [ ] `ipc.rs::COMMANDS` 登记新命令（如 `list_ssh_hosts`、`generate_ssh_profile`、
      `probe_ssh_target`）→ AGENTS §7 登记 → `lib.rs` handler →
      `capabilities/default.json` 加对应 `allow-*` → `frontend/src/lib/tauri.ts` 封装
- [ ] `src-tauri/src/` 新增 `~/.ssh/config` 纯函数解析器（含畸形输入反例单测，AGENTS §5）
- [ ] 向导生成 profile：四包 `insert` 行 + 稳定 `id` + 五键 `config`（复用 ADR-0009 的
      profile 生命周期路径与 `plugins.rs` 的写入内核，做好覆写前备份）
- [ ] `frontend/src/components/profiles/ProfileCreateDialog.tsx`：Remote SSH 模板 +
      平台不可用提示（Windows 宿主）+ "仅 headless / 自建 profile" 的范围说明
- [ ] `docs/contracts/dsh-behavior-ledger.md` 新增复现点行：四包服务映射、五键必填与
      运行时校验规则、非交互约束（参照行 11 的写法）
- [ ] 设计并落地 `roadmap.md:208` 的会话级 capability 收敛（远端会话拒绝 upgrade 类动作）
- [ ] `docs/executor.md` 实机验证清单：Linux↔macOS 双向、helper 哈希不符被拒、
      `BatchMode` 不可达时不生成 profile、远端 workspace 路径生效
- [ ] 广播落档 `docs/broadcasts.md`；AGENTS §9 索引行

### 明确不在本 ADR 范围（另开工，AGENTS §8.1）

- 放开 `Mode::parse("ssh")` / 把 `ExecutorKind::Ssh` 接入启动路径——本次只产出 profile，
  不改变宿主/客体择源逻辑（ADR-0016 的 `World` 枚举语义）；
- SSH 配置的持久化设置面板（`roadmap.md:209`）；
- 远端会话的 capability 收敛**设计**（本 ADR 只继承要求，设计另立）。

---

## 5.1 实施补注（2026-09-15，append-only；正文保持原样）

本 ADR 的决策**已完整落地**（R4a–R4d）。本节只记正文没写、实施时必须自己定的三件事，
以及行动项状态。凡与代码冲突之处，以本节为准。

**补注 1：app bundle 取 `@deepseek-ai/dsh-headless`，而非 `@deepseek-ai/dsh-web-app`。**
正文 §4 只说"生成一个**自建 profile**"，未指定它声明哪个 app bundle。实现时发现这不是
可留空的细节：既有创建链（`profiles.rs::create_profile_blocking`，写入例外 #2）会**自动**
追加 `@deepseek-ai/dsh-web-app` 声明——而那恰恰是 §3 方案 C 否决的形态（给 SSH profile
贴上"Web 工作台"标签 ⇒ 用户会以为文件树指向远端）。故实现改为把 bundle 作为参数传入，
SSH 路径取 `@deepseek-ai/dsh-headless`（上游自述"无 Host / HTTP / 浏览器层"的组合，
正对 §1.3 的 "POSIX headless and custom profiles" 范围）。该写入已在 `AGENTS.md` §6
按例外 #2 的同一机制登记。

**补注 2：四包安装放在后端一次性完成，**不复用**前端安装队列。**
两条硬约束：① **挂载行必须在四包装完之后才写**——反过来一旦安装失败就留下指向未安装
包的**幽灵挂载行**，dsh 启动即加载失败，比"装了没挂上"（可重试、无害）糟得多；
② **版本必须钉**（`dsh plugin add <裸包名>` 按 `latest` 解析，而 `@deepseek-ai/*` 的
`latest` 已实测会落后于运行时，见台账行 7 / ADR-0020 §2.4），钉版本要用运行期版本，
只有后端拿得到。代价：这是一次**可能数分钟且无逐包进度**的调用——向导必须给出明确的
进行中文案（已做），不能只转圈。

**补注 3：IPC 是 3 条而不是 2 条**（`list_ssh_hosts` / `probe_ssh_target` /
`generate_ssh_profile`）。正文 §5 行动项本就以"如 …"列了这三个；把预检单独成命令是为
让"不可达"有自己的错误面——并进生成命令会让"生成"按钮暗中先做一次网络往返。

**行动项状态**（截至 2026-09-15）：

- [x] 维护者确认 §3 方案 C 的否决（2026-09-15 评审通过）
- [x] `~/.ssh/config` 读取面登记：登记册**新增 §三「壳的文件系统读取域」**（本仓库首个
      `$DSH_HOME` 之外的读取面），边界写死"只读该文件 / 不跟随 `Include` / 1 MiB 上限 /
      只取非机密字段"
- [x] 3 条 IPC 登记 → handler → capability → `tauri.ts`（四处由 `ipc::gate_tests` 兜底）
- [x] `~/.ssh/config` 纯函数解析器 + 畸形输入反例（**23 项测试**，含 7 项语义与真 `ssh -G`
      实测对齐——见台账**复现点 18**）
- [x] 四包 `insert` 行 + 稳定 `id` + 五键 `config`（复用 `PatchFile`；**写后自证**回读确认）
- [x] 向导：`SshWorkspaceWizard.tsx`（独立对话框，入口挂在 ProfileManager 次级按钮）；
      Windows 不可用提示 + "仅 headless / 自建 profile"范围说明 + 预检逐项红/绿；
      结构闸门 `sshWizardGate.test.ts`（11 项）
- [x] 台账**复现点 19**（四包服务映射 / 五键与运行时校验 / 非交互约束 / 平台硬错 /
      范围限定 / 无上游范本）
- [ ] **会话级 capability 收敛**（正文 §2.9 继承的要求）——**仍未设计**，本 ADR 不解决它，
      已在 `docs/roadmap.md` §4.8 的落地记录里显式留白
- [ ] **实机验证清单**（`docs/executor.md` G1–G12）——**待人工执行**（真实 SSH 两端无法在
      单测里覆盖）
- [x] 广播落档 `docs/broadcasts.md`

---

## 6. 复审条件

- **上游解除 Web 视图限制**（出现"Web workspace views 支持远端路径"的集成）→ 方案 C 的否决
  理由消失，本 ADR 的范围应放宽到 web profile，并重写 §4。
- **`dsh-ssh` 配置 schema 变化**（必填键增减、新增别名替代 `host`、`helperHash` 机制变更）→
  复评 §2.3 与向导的校验集。
- **上游出现 shipped ssh 预设/示例** → §2.10 的"无范本"风险解除，应以该预设为对照基准
  重审向导产出。
- **ssh 家族包数量或服务映射变化**（例如新增 `dsh-terminal-ssh`）→ 复评 §2.2 的注册清单。
- **`ExecutorKind` / `Mode` 语义变化**（ADR-0016 的分派模型调整）→ 复评本 ADR 与启动路径的边界。
- **Windows 侧出现可用的等价路径**（例如 WSL 客体充当 SSH 跳板）→ 复评 §1.4 的平台限制。
