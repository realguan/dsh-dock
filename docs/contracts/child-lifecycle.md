# 子进程生命周期（child-lifecycle）契约

> 本文件是 **ADR-0015** 的配套方案文档：定义「壳生的子进程」的生命周期承诺、
> 平台差异、接口 seam 与验证闸门。ADR 记决策，本册记**不许悄悄变的那一面**。

## 状态

版本：**v1**｜状态：**已实施（v1.1.1 起生效）**｜
消费方：`lib.rs`（`init` / 启动期清扫接线）· `shell.rs`（dsh 服务）· `executor.rs`（wsl 投递与会话）·
`engines.rs`（引擎引导）· `profiles.rs`（`run_dsh_forward`）· `sessions.rs`（session-scan、repair）·
`resolve.rs`（版本探测）——即**全部 spawn 面**（6 个模块 / 13 个生产调用点，2026-09-11 实测）。

> **状态回填（2026-09-11）**｜判据：`docs/team/child-lifecycle契约漂移-2026-09-11.md`
> 原状态头为「版本：**v1（草案，随 ADR-0015 评审；实施未开始）**｜状态：**演进中**」，
> 与事实相反——ADR-0015 八刀已全部交付、v1.1.1 起生效（见 §8 回收补注）。
> 消费方原列 `boot.rs`：实测 `boot.rs` 对 `lifecycle::` **零引用**——它是 §3.1 单向行为的
> **宿主**（熔断器 / 错误卡），不是 §1 接口消费方；真正的接线消费方 `lib.rs`
> （`lifecycle::init` / `lifecycle::sweep_orphans`）原先漏列，现补入。

---

## 0. 为什么需要这份契约

事故（2026-09-10）：11 个逃逸的 dsh 进程持着 `~/.dsh` 会话目录的内核写锁，
使正式包（1.1.0）的会话永久打不开，用户只能"删会话 + 重启应用"。
根因不是锁（锁是 dsh 的、且设计上无过期），而是**壳没守住自己承诺的 1:1 生命周期**
（AGENTS §6）——所有清理都挂在父进程临死前跑代码的路径上，`SIGKILL` 面前全失效。

本契约要锁死的是：**「谁 spawn 谁收尸」在*任何*死亡模式下都成立。**

---

## 1. 接口 seam（按实现回填，2026-09-11）

> **本节及全文的行号口径（重要）**：文中 `:NNN` 是 **2026-09-11 快照行号**（基线 HEAD
> `6956a9a`）。**契约的权威是标识符名，不是行号**——`lifecycle.rs` 正被并行任务修改
> （`task-30` 已使 `reap_pid` / `taskkill` / 闸门解析器三处行号发生漂移）。
> 定位时请 `grep` 标识符名；行号仅供快速跳转，**不作为契约面**（§4 已声明内部布局不得被依赖）。

> **本节的效力**：本节原先写的是一份**实施前设计草图**，识别名为
> `guard(child, data_dir, role, ctx) -> Result<ChildGuard>`、`spawn_guarded(...)`、
> `ChildGuard`、`Role::Tool`。该草图**从未被 ADR-0015 批准**——ADR-0015 全文对这四个名字
> **零命中**（实测 grep），且其失败语义自相矛盾（「守卫装配失败 → `Err`」同时要求
> 「调用方不得因此放弃子进程」）。`mod lifecycle;`（`lib.rs`）是 **crate 私有**，
> 故这些名字也从未是对外 API。
> **草图已退役，以下为按实现回填的现状记录**；原草图全文见
> `docs/team/child-lifecycle契约漂移-2026-09-11.md` §1.2 与 git 历史。
> 退役标识名（可 grep 确认零使用）：`guard` · `spawn_guarded` · `ChildGuard` · `Role::Tool`。

```rust
// src-tauri/src/lifecycle.rs —— 模块私有（lib.rs: `mod lifecycle;`），非对外 API

/// 子进程角色：决定**是否配生命线 watcher**与清扫留痕字段。
/// 登记（`procs/` 登记锁）对**全部**角色生效——漏登记一处，该处的孤儿就不受清扫管辖。
pub enum Role { DshServer, DshWsl, DshCli, Pnpm, Probe }  // :36

/// 初始化（幂等，`OnceLock`）：装配生命线管道 + 绑定 data_dir。
/// 管道创建失败 → warn 降级为「只有登记表、没有生命线」（守卫是兜底，非可用性单点）。
pub fn init(data_dir: &Path)  // :107

/// 单点 spawn seam：登记 +（长命角色）挂生命线后 spawn。
/// 失败语义：`Err` **仅表示进程启动失败**；守卫装配失败一律**内部 warn 降级**，
/// 不阻断子进程、不冒泡（= §3.1「守卫自身故障 → 壳照常运行」的接口侧兑现）。
/// 未初始化 / 无 data_dir（早期路径）→ 退化为普通 `cmd.spawn()`，不阻断。
pub fn spawn(cmd: &mut Command, role: Role, ctx: GuardCtx) -> std::io::Result<Child>  // :417

/// 同 `spawn`，但收全 `Output`（`Command::output()` 语义）。
/// 与 `spawn` 共用同一登记/生命线路径；**原先漏登记**，2026-09-11 补入。
/// 实测调用点：`spawn` 7 处 / `run` 6 处（合计 13，见 §3.4）。
pub fn run(cmd: &mut Command, role: Role, ctx: GuardCtx) -> std::io::Result<Output>  // :465

/// 启动期清扫：收口**上一代壳**遗留的、仍持有登记锁的子进程。
/// 幂等；只作用于本 data_dir 的登记表；**绝不触碰**非登记进程。
pub fn sweep_orphans(data_dir: &Path) -> SweepReport  // :737

pub struct SweepReport {              // :680
    pub reaped: Vec<u32>,             // 实际收口的 pid
    pub stale: usize,                 // 清理的陈旧登记文件数（持有者已死）
    pub failed: Vec<(u32, String)>,   // 收口失败（权限/竞态/内容缺失），必须 warn 留痕
}

/// 锁判定三态（§2.3 的纯函数形态，供单测；消费方只需知道存在三态）。
pub enum RegistrationVerdict { Stale, LiveOrphan, Indeterminate }  // :697
pub fn classify_lock_result(result: &Result<(), std::fs::TryLockError>) -> RegistrationVerdict  // :719
```

`GuardCtx`（`:387`）：`{ profile: Option<String>, argv0: String, started_at_ms: u64 }`，
由 `GuardCtx::of(argv0, profile)` 构造——**仅供留痕与排查**，不参与任何判定。

> **为什么签名里没有 `data_dir`**：`data_dir` 由模块内的 `active_data_dir()`
> （全局 / 线程覆盖）给出，不作为每个调用点的参数——否则 13 个调用点都要穿一层
> 与自身逻辑无关的路径参数。这是对草图 `spawn_guarded(cmd, data_dir, …)` 的**有意偏离**。

**生命线分配**：仅 `DshServer` / `DshWsl` / `DshCli` / `Pnpm` 配 watcher；
`Probe`（短命探测，毫秒级退出）不配——给它配 watcher 只会白留一个 `sh` 常驻到壳退出。

**漏登记 = 失管**：新增 spawn 面必须同时走 `spawn`/`run` 并在 §3.4 登记。
（现有强制手段 = §5 行 12 的源码文本闸门 `production_spawns_go_through_lifecycle_seam`，
覆盖 **14** 个文件的 `SOURCES` 白名单——2026-09-11 由 13 扩至 14，补入 seam 属主文件
`lifecycle.rs` 自身，此前它正是盲区。其**唯一**盲区 = 「**新增** `.rs` 文件不在白名单内则不被扫」，
见 `docs/team/child-lifecycle契约漂移-2026-09-11.md` §5 B3。）

> **闸门机制分辨（勿混写，2026-09-11 核对）**：spawn 闸门走 **`include_str!` 白名单**
> （故有上述「新文件不被扫」盲区）；**`network_gate.rs` 走运行时遍历 `read_dir`**
> （显式注释「刻意不用 `include_str!` 白名单」，见其 `source_files()`）——**无此盲区**。
> 二者机制不同，引用时不要互相套用结论。

---

## 2. 数据模型

### 2.1 proc 登记锁（清扫的唯一判据）

| 项 | 约定 |
|:---|:---|
| 路径 | `<app_data>/procs/<token>.lock`；`token = <父 pid>-<序号>-<纳秒>`（`new_token`，`:361`） |
| 为何不用子 pid | 登记文件必须在 `spawn` **之前**创建并持锁（子进程要继承该 fd），**此刻子进程 pid 尚不存在**；子 pid 写进文件**内容**，清扫时读取（判据始终是"能否加锁"，不是 pid） |
| 锁 | 壳 spawn 时对该文件持**排他锁**，并让**子进程继承该 fd**（`pre_exec` 里 `dup2` 到 `INHERITED_LOCK_FD = 198`，`:545` / `:584`）；子进程死亡由内核释放 |
| 内容 | JSON：`{pid, role, profile, argv0, startedAtMs}`（camelCase，`:374-383`）——**仅排查用**；判据是"能否加锁"，不是 pid |
| PID 复用 | **天然免疫**：判据是内核锁，复用的 pid 不会持锁 |
| 陈旧文件 | 能加锁成功 = 持有者已死 → 删除该文件（不算孤儿） |

> [!IMPORTANT]
> **实施前置已通过（2026-09-10 实测，见 ADR-0015 §7）**：node **全程保留**继承来的 fd，
> 且该 fd **不流入孙进程**。因此判据按**原设计**实现（内核锁 → 与"壳的直接子进程"1:1 对应），
> **不启用** pid + 启动时间降级路径。三条实测同时确认：父 SIGKILL 后锁仍被持有、
> 子进程死亡后锁自动释放、孙进程存活不影响判定。

> **登记锁 fd 的纪律与失败方向（2026-09-11 补）**：登记锁走**固定 fd 号 198**
> （`dup2` 注入），这与 §2.2「**禁止固定 fd 号**」看似同一技法，但**失败方向相反**，
> 故纪律不同：
> - **生命线**误判 EOF（fd 异常）→ **误杀**（不可挽回）→ 一律走 `Stdio`，**禁用**固定 fd 号；
> - **登记锁** fd 被意外关闭 → 锁提前释放 → 清扫判"陈旧" → **漏收**（还有下次启动兜底），
>   且 198 远离 stdio 占用区。
> 二者共用一条底线：**不确定时宁可不收口**（§2.3）。
> **未决（B2）**：子进程（node）是否可能在运行中关闭 fd 198，**尚无实测**——
> 缺的证据 = 一次「壳 `SIGKILL` 后 `lsof -p <orphan>` 确认 fd 198 仍持锁」的实机复现。

### 2.2 生命线管道

| 项 | 约定 |
|:---|:---|
| 形态 | 壳持**写端**（永不写入），watcher 持**读端**（经 `Stdio` 交给其 **stdin**） |
| 语义 | 壳以任何方式死亡 → 内核关闭写端 → watcher 读到 EOF |
| 作用域 | watcher 只对**登记时给的 pid 白名单**发信号——**不是进程组**（ADR-0015 §2.2） |
| 载体 | unix：`/bin/sh` 短脚本；Windows：不适用（走 Job Object） |
| **通道纪律** | **禁止固定 fd 号**（`dup2` 到魔数 + `read <&N`）。曾因此误杀正常子进程（ADR-0015 §7.1）；一律走 `Stdio` |
| **失败方向** | 读不到生命线（fd 异常）→ **放弃收口**，绝不误杀；收口另有启动期清扫兜底 |

### 2.3 清扫判定的三态（**安全方向**，2026-09-10 审核修正）

`File::try_lock` 返回 `TryLockError`，它有**两档**，必须分开判定：

| 结果 | 含义 | 处置 |
|:---|:---|:---|
| `Ok(())` | 加锁成功 → 持有者已死 | 陈旧文件，删除 |
| `Err(WouldBlock)` | **明确**被别的句柄/进程持有 | 真孤儿 → 收口 |
| `Err(Error(io))` | **锁机制本身失败**（文件系统不支持 flock / I/O 故障 / 权限） | **无法判定 → 跳过，绝不收口** |

> [!IMPORTANT]
> 首版把 `Err(_)` 一律当成"仍被持有"。在不支持 `flock` 的文件系统上，这会让
> **每一条登记**都被判成活跃孤儿，并按文件内容里的 pid 去 `SIGTERM`——而那个 pid
> 完全可能已被复用给无关进程，构成**误杀**。纪律：**不确定时宁可不收口**
> （还有下次启动兜底；误杀无法挽回）。闸门 = `lock_verdict_separates_would_block_from_io_error`
> 与 `only_would_block_may_authorize_reaping`。

---

## 3. 行为承诺

### 3.1 单向语义（**最重要的一条**）

```
壳死            → 子必须死        ✅ 本契约的目标
子死            → 壳不动          ✅ 用户需要（出错卡 + 控制中心可操作）
子死            → 自动重启        ⚠️ 只由既有熔断器管（60s 内 3 次），守卫不得自维重启
守卫自身故障    → 壳照常运行      ✅ 守卫是兜底，不得成为可用性的单点
```

> 违反任一条即违约。特别是：**"supervisor/watchdog"不得实现为"崩了就拉起"**——
> 那会与用户的排障动作（禁用可疑插件后重载）打架。

### 3.2 平台差异（显式声明，禁止"看起来一致"）

| 平台 | 正常停止（今天，不变） | 硬杀（**本契约后**） | 机制 |
|:---|:---|:---|:---|
| unix | `SIGTERM` → grace 3s → `SIGKILL`，**只杀直接子 pid** | 同左（watcher 补位） | 生命线 watcher |
| Windows | `taskkill /PID /T /F`（**整树**，ADR-0014 已落地） | 同左（内核补位） | Job Object `KILL_ON_JOB_CLOSE` |
| WSL | 客体 wrapper 看门狗（`executor.rs::guest_boot_script`，既有） | 同口径 + 宿主侧 job/生命线双保险 | 既有 + 本契约 |

**两侧都只是让"硬杀"与"各自的正常停止"对齐，未引入新语义。**
Windows 的整树连坐是 ADR-0014 已接受的行为；unix 不连坐进程组是 ADR-0015 §2.2 的硬约束。

### 3.3 时序保证

- 收口序列：`SIGTERM` → grace（默认 **3s**，与 `stop_dsh` 同口径）→ `SIGKILL` → `wait()` 回收；
  - **`wait()` 仅适用于正常停止路径**（`stop_dsh`，`shell.rs:238`——它是子进程的父进程）。
    **启动期清扫路径无法 `wait()`**（`reap_pid`，`lifecycle.rs` 的 `#[cfg(unix)]` 分支）：清扫进程不是孤儿的
    父进程，只能按**存活探测**判定收口结果。本行原写「收口序列…→ `wait()` 回收」未作此区分
    （2026-09-11 限定）。
- **幂等**：目标 pid 已消失时全部动作 no-op，不报错；
- **不阻塞壳退出**：watcher 与清扫都在独立线程/进程，不得拖慢 `RunEvent::Exit`；
- 清扫必须在 **probe 之前**完成（否则可能与新一代探测竞争同一份 profile 目录）。

### 3.4 覆盖范围（必须全部登记，无例外）

`spawn_dsh`（服务本体）、`executor` wsl 启动、`run_dsh_forward`（插件装卸，**用户恢复流程主路径**）、
`dsh --dump-config`、`repair-session.mjs`、`engines` 的 **`pnpm add -g`（分钟级）**、
`tar` 解包、各版本探测、`taskkill`。

> **补录（2026-09-11）**：上列九项**全部已登记**（逐项实测），但清单**漏列**两处实测存在的面，
> 现补入：**`session-scan.mjs`**（`sessions.rs` 的会话扫描）、
> **`wsl.exe base64 投递`**（`executor.rs` 的客体引擎资产投递）。
> 当前**实际生产调用点 = 13 处**（`spawn` 7 + `run` 6，覆盖 6 个模块；2026-09-11 实测）。
> 清单仍是**枚举**——新增面漏登记不会自动变红（机器闸门存在「新文件不被扫」盲区，
> 见 §1 尾注）。
>
> **统计口径（复算必读，2026-09-11）**：计数时截断测试模块的标记**必须**用
> `"\n#[cfg(test)]\nmod tests"`（闸门自身口径）。**不可**用「首个 `#[cfg(test)]` 即截断」——
> `resolve.rs` 在 `:87`/`:100` 有**单条目级** `#[cfg(test)]`（非 `mod tests`），
> 用前者会让该文件在 `:100` 提前截断，**漏掉 `:238`/`:583` 两处生产调用点**，
> 结果误得 11（或 12）。团队教训 M4（`docs/team/README.md` §8）。
> 另注意：**「13 处调用点」与「闸门白名单文件数 14」是两个不同的量**，勿相互推断。

> 漏登记一处 = 该处的孤儿不受清扫管辖。

---

## 4. 禁止外部访问的内部实现

- `procs/<token>.lock` 的**布局与内容**是内部实现：消费方（含未来的 UI）不得依赖其字段，
  只可依赖 `sweep_orphans` 的返回值；
- 生命线 watcher 的命令行形态、登记锁的 fd 号（`INHERITED_LOCK_FD`）：内部实现；
- Job Object 的 **job 句柄与限制参数**（`CreateJobObjectW(null, null)` = **未命名** job，`:299`）：内部实现；
- **严禁**任何模块直接读写、删除、加解锁 `~/.dsh/sessions/**/session.lock`——
  那是 dsh 的协议面，壳碰它就是违约（ADR-0015 方案 D）。

> **本节的回填（2026-09-11）**：原列「生命线 **fd 编号**…内部实现」——实现走 `Stdio`，
> **根本没有 fd 编号**（§2.2 明文禁止固定 fd 号），属残留描述，已删；
> 原列「**Job Object 名称**：内部实现」——实现是**未命名** job，暗示具名会误导，已改为句柄与参数。

---

## 5. 测试闸门

| # | 测试 | 类型 | 现状 |
|:--|:---|:---|:---|
| 1 | **父被 `SIGKILL` → 子进程在 grace 内消失** | 集成（复现先行） | ✅ `guarded_child_dies_when_parent_is_sigkilled`（含 1.5s **观察窗**：壳存活期间子进程不得消失，否则"误杀"会冒充"收口"而假绿，ADR-0015 §7.1） |
| 1b | 对照组：**未经守卫**的裸 spawn 在父横死后子进程仍存活 | 集成 | ✅ `unguarded_child_survives_parent_sigkill_control`——钉住"问题确实存在"，防误以为父死子必死是内核行为 |
| 2 | 清扫：陈旧锁文件（持有者已死）→ 删除且不误杀 | 单测（fixture） | ✅ `sweep_deletes_stale_registration_without_killing_anything` |
| 3 | 清扫：活锁（持有者存活）→ 收口并留痕 | 单测（fixture） | ✅ `sweep_reaps_live_orphan_holding_registration_lock` |
| 4 | 清扫：**不触碰**非登记进程（负例） | 单测 | ✅ `sweep_never_touches_unregistered_processes` |
| 5 | 守卫失败（管道/赋值失败）→ 子进程照常运行，仅 warn | 单测 | ✅ `guard_failure_does_not_block_child` |
| 6 | 单向性：子进程自行退出 → 壳不退出、不重启（除既有熔断器） | 集成 | ❌ **未兑现（未测）**——见下方「行 6/行 7 缺口」 |
| 7 | Windows 停止参数构造（`taskkill` 回退路径） | 纯函数单测 | ⚠️ **部分**：纯函数 `shell::windows_kill_args` 与其测试存在，但**回退路径**（`lifecycle.rs::reap_pid` 的 `#[cfg(not(unix))]` 分支）**内联**构造 `/PID /T /F`、**不复用**该纯函数——覆盖的不是本行点名的路径。见下方「行 6/行 7 缺口」 |
| 8 | 平台分叉：三平台各自编译通过 + 目标语义测试 | CI 三平台 | ✅ 已有闸门（`.github/workflows/build.yml` matrix 各自跑 clippy `-D warnings`） |
| 9 | **真 dsh** 被硬杀后收口（走生产 `spawn_dsh`） | 真机锚（`#[ignore]`） | ✅ `real_dsh_is_reaped_when_shell_is_sigkilled`；已用**变异测试**证明会红 |
| 10 | 锁判定三态：只有 `WouldBlock` 授权收口 | 单测 | ✅ `lock_verdict_separates_would_block_from_io_error` + `only_would_block_may_authorize_reaping` |
| 11 | **引导真的调用了清扫**（且早于派发执行器） | 源码文本闸门 | ✅ `boot_sweeps_orphans_before_dispatching_executor`；删掉那行即红（变异验证） |
| 12 | 生产 spawn 全覆盖（`spawn/output/status/wait_with_output`） | 源码文本闸门 | ✅ `production_spawns_go_through_lifecycle_seam`（已扩面；`SOURCES` 白名单 **14** 文件——2026-09-11 补入 seam 属主 `lifecycle.rs`；唯一盲区「新增文件不被扫」见 §1 尾注） |

> **行 6 / 行 7 缺口（2026-09-11 标注，判据 `docs/team/child-lifecycle契约漂移-2026-09-11.md` §2 E / §3.2）**：
> 这两行是**规范承诺未兑现**（非文档漂移），处置方向 = **改实现**（已另派单，不在契约回填范围）。
> 本契约在此**如实标注为未兑现**，不改为 ✅——避免"承诺悬空"被读成"已交付"。
> 其中行 6 的「集成」类型标注本身即**不可兑现**（本仓库无 Tauri `AppHandle` 集成测试基建，
> 且 AGENTS §5 规定不引测试依赖）；可兑现的替代形态见诊断报告 §4.4（源码闸门 + 进程级 + 纯函数单测），
> 落地后**由实现方同步回填本行**。
>
> **并行进展（2026-09-11 观测，以最终 commit 为准）**：`task-30` 已把 `reap_pid` 的裸
> `Command::new("taskkill")` 改为经由 `crate::child_cmd`（修掉诊断的附带发现 B1 / 违 AGENTS §4.1）；
> **行 7 的缺口未因此关闭**——参数构造仍内联、仍不复用 `shell::windows_kill_args`，
> 纯函数单测覆盖的仍不是回退路径本身。**行 6 / 行 7 的最终回填由实现方完成后写入。**

---

## 6. 实机验证清单（2026-09-11 拆分：本地保留 / 远端改指针）

> **拆分判据**：本契约只保留**本机（macOS）可执行**的项——它们是可兑现的承诺；
> **本机无法验证的项改指针**指向 `docs/executor.md` 的实机清单，**避免双源**
> （写入判据同 AGENTS §11.3；本机跑不了的项写在契约里只会腐烂）。

**A. macOS / 引导期（本机可跑 = 可执行承诺）**

- [ ] macOS：`kill -9` 壳 → `ps` 无残留 dsh；被它锁住的会话可正常打开（**本次事故的直接回归项**）
- [ ] macOS：`cargo tauri dev` 连续重编重启 5 次 → 孤儿数为 0
- [ ] macOS：工作台内起一个长驻 dev server → 壳退出后**它仍然活着**（ADR-0015 §2.2 的负向验证）
- [ ] 引导期：`pnpm add -g` 进行中强杀壳 → 下次引导不被 store 锁阻塞

**B. Windows / WSL（本机无法验证 → 指针，不在此重复承诺）**

| 原清单项 | 落点（唯一事实源） |
|:---|:---|
| Windows：任务管理器强杀壳 → dsh 进程树消失 | `docs/executor.md` §C |
| Windows：`taskkill` 回退路径（Job 赋值失败时）仍能收口 | `docs/executor.md` **§C2**（2026-09-11 补写；含「第 0 步先判定路径是否可达」） |
| WSL：客体 wrapper + 宿主守卫双路径 | `docs/executor.md` **§D**（2026-09-11 补写为双路径表 + 分步验证；原 §D 只是一句指针） |

**C. 本机可跑（不依赖 Windows 实机）**

| 项 | 落点 |
|:---|:---|
| Windows 目标的编译 + lint（覆盖 `#[cfg(not(unix))]` 回退分支） | `docs/executor.md` **§E1** |
| 机器闸门建议：`taskkill` 回退参数纯函数化（本机可跑） | `docs/executor.md` **§E2** |
| 机器闸门建议：客体停止位常量 ↔ wrapper 字面量（本机可跑） | `docs/executor.md` **§E3** |

> **指针缺口已闭合（2026-09-11 晚）**：原注记「`executor.md` 未把这两条列为验证项」已由
> `task-32` 补写 §C2 / §D 解决——**指针现在指向真实存在的内容**。
> 补写只让「跑的时候知道跑什么」，**不等于要求现在跑**：Windows 真机验证按维护者指示**搁置**，
> 相关项在 `executor.md` 内保持「待跑 / 未决」。
> 两处**未决**已就地标注、且**不属契约面**（一处待判「回退路径是否可达」，
> 一处待判「是否加 `wsl --terminate` 兜底」）。
> 判决依据：`docs/team/executor清单补写-2026-09-11.md`。

---

## 7. 演进规则

- **验证纪律（2026-09-10 审核补）**：本模块的关键断言必须能**变异测试**证伪——
把被守护的机制临时关掉，对应用例必须变红。已对三条做过：关生命线 → 真 dsh 用例红
（并真的留下孤儿）；删清扫接线 → 接线闸门红；去掉 dev 门 → release 用例红。
新增断言时请照此自检，否则"绿灯"可能只是没测到。

**收窄容易、放宽难**：任何把作用域从"壳生的 pid"放宽到"进程组/整树"的改动，
  都是对 ADR-0015 §2.2 的破坏，须重开 ADR；
- AGENTS §4.1 口径同步：从"spawn 一律经 `child_cmd`"扩展为
  "**spawn 一律经 `child_cmd` + `lifecycle::spawn`/`run`**"（AGENTS §6 现行原文，
  2026-09-11 核对；原写 `lifecycle::spawn_guarded` 是退役草图名，已校正）；
- 新增 spawn 面时**必须**同步登记（§3.4），并在本契约的消费方列表中加入该模块；
- 若 dsh 上游自带父死自退（ADR-0015 §8 复审条件），本契约降级为冗余层，可整体撤回。

> **台账（2026-09-11 补）**：**本契约已登记进 `docs/contracts/README.md` §5 台账（2026-09-11）**。
> 补登前它**不在台账里**——而 §5 的 WARNING 原文正是「台账里没有、但两个模块正在共享的东西
> = **未登记的事实契约**」：**契约自己成了未登记的事实契约**。这正是本契约能漂移至此
> 而无人定期复核的**根因**（推断；佐证 = 台账是唯一的契约复核入口清单）。
> 登记后即受 README §4「独立分支 + 消费方同步 + 尽快合并」流程管辖。

---

## 8. 实施记录

> **本节已回收（2026-09-11）**：原为「建议实施顺序（分刀，AGENTS §8.1 一次一个意图）」的
> 八刀计划表（刀 0–7）。八刀**已全部交付**，v1.1.1 起生效——计划表属**实施期工件**，
> 按 AGENTS §11.4「回收触发 = 已升格 ADR / 已有测试 CI 兜底 / 已失效」整节回收，
> 留此带日期补注不留正文。
>
> 实施记录（唯一事实源）：`docs/broadcasts.md` 2026-09-10 条目（ADR-0015 交付落档）
> + `docs/adr/0015-child-process-lifecycle-ownership.md` §5 行动项。
> 原八刀表全文见 git 历史与 `docs/team/child-lifecycle契约漂移-2026-09-11.md` §2 F 第 39 条。
>
> 其中原「刀 6 dev/prod `DSH_HOME` 隔离」的成果（`~/.dsh-dock-dev`）已升格为
> AGENTS §6 持久化例外册登记项，不再由本契约承载。
