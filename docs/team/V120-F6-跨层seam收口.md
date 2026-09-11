# V120-F6：跨层 seam 收口（`checking → downloading` 缺失）

> 日期：2026-09-11 ｜ 执行：`frontend` ｜ 任务：**task-53（T-F6）**，接续 task-46 / task-52
> 证据强度：**实测**（复现先行 + 变异证伪 + 全表差异枚举 + sha256 校验复原 + 闸门逐条 exit code）
> 基线：44 files / 359 passed → 交付后 **45 files / 365 passed**
> 范围：`frontend/src/**`（`clientUpdateStore.ts` + 两个测试文件）；**未动 Rust / 依赖**；**禁 git 写**（未执行）

## 0. 闸门（逐条独立执行）

| 命令 | exit | 输出 |
| :--- | :--- | :--- |
| `cd frontend && pnpm run typecheck` | **0** | `tsc -b --pretty false`（无输出） |
| `cd frontend && pnpm run lint` | **0** | `Found 0 warnings and 0 errors.`（144 files） |
| `cd frontend && pnpm run test` | **0** | `Test Files 45 passed (45)` / `Tests 365 passed (365)` |

## 1. 复现先行：seam 测试红（点名断言 + file:line）

新建 `__tests__/clientUpdateSeam.test.ts` 后，**改表之前**实测 **5 failed**：

```text
× Windows 路径：零丢弃 + 末态 relaunching + 进度采样 ≥2 点
    AssertionError: 新发射序不得被前端丢弃: expected [ 'checking->downloading', …(6) ] to deeply equal []
    ❯ src/__tests__/clientUpdateSeam.test.ts:77
× 非 Windows 路径（无 Installing）：零丢弃 + 末态 relaunching + 进度采样 ≥2 点
    AssertionError: 非 Windows 序不得被前端丢弃: expected [ 'checking->downloading', …(5) ] to deeply equal []
    ❯ src/__tests__/clientUpdateSeam.test.ts:85
× 逐对相位在真实序内合法
    AssertionError: windows: checking → downloading 被前端拒绝: expected null not to be null
    ❯ src/__tests__/clientUpdateSeam.test.ts:100
× 首条进度事件即被接受
    AssertionError: checking → downloading 必须放行: expected null not to be null
    ❯ src/__tests__/clientUpdateSeam.test.ts:109
× 守卫未被放宽成万能表
    AssertionError: installing → downloading 是倒退，Rust 侧明文拒绝: expected {…} to be null
    ❯ src/__tests__/clientUpdateSeam.test.ts:122
```

**与 lead 探针输出逐字吻合**（`expected [ 'checking->downloading', …(6) ]`）⇒ 独立复现成立。

## 2. 根因：两侧各自漂移，且**没有任何测试跨过 IPC 边界**

- **Rust 侧**（D6r）：为「点了下载立刻有 loading」，在 `blocked_check` **之前**新增
  `set_state(Checking)`（`updater.rs:243`），真实序首条由 `downloading` 变为 `checking`。
- **前端侧**（task-46 定表）：`checking: ["available","upToDate","failed"]` —— 锚定的是
  **task-46 当时的** Rust 序。
- ⇒ `checking → downloading` 非法 ⇒ **整条进度序列从第一条起全部丢弃**、界面卡在「检查中」。

**为什么两侧单测当时都绿**：Rust 测的是 Rust 自己的相位模型（`transition_allowed` 允许该边），
前端测的是前端表的**自洽性**（表里写什么就断言什么）——**两边都没比较「实际发射序」与
「实际消费表」**。这正是需要常驻 seam 测试的理由（见 §4）。

## 3. 真实发射序（我按**当前源码**逐行核对，非照抄任务描述）

| 平台 | 序 | 依据 |
| :--- | :--- | :--- |
| Windows | `available → Checking → Downloading{0,None} → Downloading×N → Installing → Done → Relaunching` | `updater.rs:243`（Checking）/ `:262`（首帧 Downloading）/ `:325`（`Installing` 在**下载完成后**、`#[cfg(target_os="windows")]` 块内）/ `:363+` Done / `:379+` Relaunching |
| 非 Windows | `… → Downloading×N → Done → Relaunching`（**无 `Installing`**） | `:332` `#[cfg(not(target_os="windows"))]` 分支注释：「走 `download_and_install` 一体化 API，**没有**『下载完成』这个可拦截的时刻 ⇒ 不能在中间插 `Installing`」 |

**两处与任务描述的差异（已核对，非臆测）**：
1. 任务说「`available → installing` 也不在表内」——那是 task-46 时的情形；**现序中
  `Installing` 已移到下载之后**，且**下载前**的可见态是 `Checking`。故「点下载立刻有 feedback」
  的真正机制 = `available → checking`（该边表内已有）。
2. lead 关于「非 Windows 不预发 Installing」的说明**与源码一致**，已按此写测试。

## 4. 修复（两处，一放一收）

### 4.1 放宽（最小修复，任务交付物 1）

```diff
- checking: ["available", "upToDate", "failed"],
+ checking: ["available", "upToDate", "downloading", "failed"],
```

`checking → installing` **未加**：按现序，`Installing` 只出现在 `downloading` 之后，
`checking → installing` 不发生在任何真实路径上；加上它属于无依据的放宽。

### 4.2 收紧（**请 lead 复核**：与 Rust 明文不变式对齐）

```diff
- installing: ["installing", "downloading", "done", "relaunching", "failed"],
+ installing: ["installing", "done", "relaunching", "failed"],
```

- **理由**：`installing → downloading` 是**相位倒退**，Rust 侧
  `updater.rs::reverse_transition_is_rejected` **明文断言必须拒绝**（"这正是 D6 修掉的序"）。
  task-46 我为「T-D6r 可能改序」双向放行，而 lead 已明确**序现已固定、不会再改**，
  该放行失去依据，且与本表「锚定真实序」的定位矛盾。
- **风险**：若 Rust 将来真的回到 `installing → downloading`，seam 测试会先红（§5 的固化价值）。
- **回退成本**：一行。若 lead 判应保留双向，改回即可（我的 seam 测试中对应断言需同步）。

## 5. 变异证伪（任务交付物 3，**实测**）

移除 `checking` 的 `downloading` ⇒ seam 测试 **4 failed**，点名如下：

```text
× Windows 路径：零丢弃…  AssertionError: 新发射序不得被前端丢弃: expected [ 'checking->downloading', …(6) ] (seam:77)
× 非 Windows 路径…       AssertionError: 非 Windows 序不得被前端丢弃: expected [ 'checking->downloading', …(5) ] (seam:85)
× 逐对相位在真实序内合法   AssertionError: windows: checking → downloading 被前端拒绝 (seam:100)
× 首条进度事件即被接受     AssertionError: checking → downloading 必须放行 (seam:109)
✓ 守卫未被放宽成万能表     ← 该条与本次变异无关，故仍绿（符合预期，说明变体精确）
```

`trap` 复原后 sha256 `4e40918b0da6929494acec2fde36df777a8f7b3c7729ec208d2db40cf18b0dfe` 前后一致 = **RESTORE_OK**。

## 6. 常驻 seam 契约测试（任务交付物 2，**本任务的核心价值**）

`__tests__/clientUpdateSeam.test.ts`（6 例），用**真实** `applyUpdateEvent` 回放**真实发射序**：

| 断言 | 内容 |
| :--- | :--- |
| Windows 序 | **零丢弃** + 末态 `relaunching` + **进度采样 `[0,10,50,90]`（≥2 点）** |
| 非 Windows 序 | 同上（**无 `Installing`** 的变体——下一个潜在破口） |
| 逐对相位合法 | 把「序」本身当断言对象，逐对检查表是否接受 |
| 首条进度即被接受 | 单钉 `checking → downloading`（本次破口） |
| 防万能化 | `idle→done` / `idle→downloading` / `available→relaunching` / `installing→downloading` 必须仍被拒 |
| 幂等重复 | 连续 `downloading` 三次零丢弃、采样 `[1,2,3]`（Rust 规则 1） |

**同源声明**（写入文件头注释）：发射序锚定
`updater.rs::emitted_sequence_advances_monotonically` 与 `run_download_and_install` 的真实
`set_state` 位点；**两侧任一改动本序/本表，都必须同步本文件**——本文件红了即代表两侧又漂移了。

## 7. 顺带修掉的**结构性隐患**：同一事实的两个副本

task-46 在 `updatePhase.test.ts` 内联了一份「真实序」，与 seam 文件**各存一份** ⇒
两份副本必然随 Rust 演进分叉（本次破口正是这种分叉的产物）。故：

- `updatePhase.test.ts`：**删除内联的真实序回放**（保留表的接线级/顺序无关断言），
  并在注释中指向 seam 文件；同时订正 2 条**锚定旧序**的断言：
  - `下载链 available→installing→downloading→…` → 改为 `checking→downloading→installing→…`；
  - `点下载后立即显示的 Installing…` → 改为断言真正的即时可见态 `available → checking`。
- **发射序自此只有一处事实源** = `clientUpdateSeam.test.ts`。

> 说明：这不是「测试被削弱」——删除的是**已过时的重复副本**，其覆盖被 seam 文件以更强形式
> （零丢弃 + 末态 + 进度推进）承接，且新增了非 Windows 分支与变异证伪。

## 8. 全表差异枚举（任务协调项：**列出、未擅改**）

逐字移植 Rust `transition_allowed` + `phase_rank` 到脚本，穷举 9×9=81 格，与**修复后的前端表**比对：

### 【A】Rust 允许 / 前端拒绝 —— 21 条（**未改**）

```
idle→idle, idle→available, idle→upToDate, idle→relaunching, idle→done, idle→failed,
checking→checking, checking→installing, checking→relaunching, checking→done,
available→available, available→relaunching, available→done,
upToDate→upToDate, upToDate→relaunching, upToDate→done, upToDate→failed,
downloading→relaunching, relaunching→relaunching, done→done, failed→failed
```

**评估**：绝大多数是 Rust 的 **rank 单调性**泛化许可 + **同相位幂等**，**不发生在任何真实发射序**上
（真实序只需 §3 那 7 条边，现已全绿）。**未扩大改动面**（lead 明确要求不放宽成万能表）。
其中**唯一值得后续考虑**的一条：

- **`idle → failed`**：Rust 规则 1「Failed 可从任意相位进入」；前端 `idle: ["checking"]` 拒绝。
  风险场景：整页重载后经 `get_client_update` 播种为 `idle`，随后立即收到 `Failed`（如安装器异常）
  ⇒ **失败被静默丢弃、用户什么都看不到**。**未改**（属放宽，按协调要求先报）；
  若要修，最小改动 = `idle: ["checking", "failed"]`。

### 【B】前端允许 / Rust 拒绝 —— 4 条（1 条已收紧，3 条保留）

| 边 | 状态 | 评估 |
| :--- | :--- | :--- |
| `installing→downloading` | **本次已收紧（拒绝）** | Rust 明文不变式，见 §4.2 |
| `upToDate→idle` / `done→idle` / `failed→idle` | 保留 | 属**前端 UI 驱动**的迁移（用户「知道了」类交互回到待机）；Rust **从不发射 `Idle`**，故发射方向上无冲突。前端保留是有意的产品行为，非漂移 |
| `relaunching→done` | 保留（既有） | Rust 拒绝（rank 倒退）；实际不发生。**未动**——收它需改 task-46 既有断言，且无用户可见收益，留待 lead 判定 |

## 9. 未决 / 风险

1. **§4.2 的收紧请复核**：这是本任务唯一的「行为收窄」，与 task-46 的「双向放行」相反。
   依据是 Rust 明文不变式 + lead「序已固定」的前提；回退成本一行。
2. **§8-A 的 `idle → failed`**：建议后续单列（真实但低频的静默丢失败路径）。本轮按协调要求只报。
3. **Windows 无实机**：结论 = **「机制级修复 + 单元/契约级验证，真机待复验」**。
   本机已验证：两条真实序（Windows 序列/非 Windows 序列）在真实表下的零丢弃、末态、进度推进；
   真机需核「点下载 → 进度条真的在动」与「非 Windows 平台实际发射序无 Installing」。
4. **seam 测试的有效边界**（如实标注）：它守卫的是「**前端表是否接受 Rust 的发射序**」，
   但发射序**本身**是**人抄进测试的常量**——若 rust-core 改了序而**没人同步这个常量**，
   本测试**不会自动发现**。真正端到端的自动比对需要跨语言（Rust 导出序 → 前端消费），
   超出本任务范围，建议作为「测试门禁质量」同类议题另行立项。
