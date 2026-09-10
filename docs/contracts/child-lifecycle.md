# 子进程生命周期（child-lifecycle）契约

> 本文件是 **ADR-0015** 的配套方案文档：定义「壳生的子进程」的生命周期承诺、
> 平台差异、接口 seam 与验证闸门。ADR 记决策，本册记**不许悄悄变的那一面**。

## 状态

版本：**v1（草案，随 ADR-0015 评审；实施未开始）**｜状态：**演进中**｜
消费方：`shell.rs`（dsh 服务）/ `boot.rs`（启动与崩溃守护）/ `executor.rs`（wsl）/
`engines.rs`（pnpm 引导）/ `profiles.rs`（`run_dsh_forward`）/ `sessions.rs`（dump-config、repair）/
`resolve.rs`（探测）——即**全部 spawn 面**。

---

## 0. 为什么需要这份契约

事故（2026-09-10）：11 个逃逸的 dsh 进程持着 `~/.dsh` 会话目录的内核写锁，
使正式包（1.1.0）的会话永久打不开，用户只能"删会话 + 重启应用"。
根因不是锁（锁是 dsh 的、且设计上无过期），而是**壳没守住自己承诺的 1:1 生命周期**
（AGENTS §6）——所有清理都挂在父进程临死前跑代码的路径上，`SIGKILL` 面前全失效。

本契约要锁死的是：**「谁 spawn 谁收尸」在*任何*死亡模式下都成立。**

---

## 1. 接口签名

```rust
// src-tauri/src/lifecycle.rs（新增）

/// 子进程角色：决定清扫时的处置优先级与留痕字段。
pub enum Role { DshServer, DshWsl, DshCli, Pnpm, Probe, Tool }

/// 登记并守护一个刚 spawn 的子进程。
/// 失败语义：守卫装配失败（管道/登记表/job 赋值）→ Err；**调用方不得因此放弃子进程**，
/// 应记 warn 后继续（守卫是兜底，不是前置条件）。
pub fn guard(child: &std::process::Child, data_dir: &Path, role: Role, ctx: GuardCtx)
    -> std::io::Result<ChildGuard>;

/// 单点 spawn seam：AGENTS §4.1 的 child_cmd 构造 Command，本函数负责 spawn + guard。
/// 生产路径禁止再直接调用 Command::spawn()。
pub fn spawn_guarded(cmd: &mut Command, data_dir: &Path, role: Role, ctx: GuardCtx)
    -> std::io::Result<(std::process::Child, ChildGuard)>;

/// 启动期清扫：收口**上一代壳**遗留的、仍持有 proc 锁的子进程。
/// 幂等；只作用于本 data_dir 的登记表；不触碰任何非登记进程。
pub fn sweep_orphans(data_dir: &Path) -> SweepReport;

pub struct SweepReport {
    pub reaped: Vec<u32>,              // 实际收口的 pid（已 SIGTERM/SIGKILL）
    pub stale: usize,                  // 清理的陈旧锁文件数（持有者已死）
    pub failed: Vec<(u32, String)>,    // 收口失败（权限/竞态），必须 warn 留痕
}
```

`GuardCtx`：`{ profile: Option<String>, argv0: String, started_at_ms: u64 }`——**仅供留痕与排查**，
不参与任何判定。

---

## 2. 数据模型

### 2.1 proc 登记锁（清扫的唯一判据）

| 项 | 约定 |
|:---|:---|
| 路径 | `<app_data>/procs/<pid>.lock` |
| 锁 | 壳 spawn 时对该文件持**排他锁**，并让**子进程继承该 fd**（`pre_exec` 清 `FD_CLOEXEC`）；子进程死亡由内核释放 |
| 内容 | JSON：`{pid, role, profile, started_at_ms, argv0}`——**仅排查用**；判据是"能否加锁"，不是 pid |
| PID 复用 | **天然免疫**：判据是内核锁，复用的 pid 不会持锁 |
| 陈旧文件 | 能加锁成功 = 持有者已死 → 删除该文件（不算孤儿） |

> [!IMPORTANT]
> **实施前置已通过（2026-09-10 实测，见 ADR-0015 §7）**：node **全程保留**继承来的 fd，
> 且该 fd **不流入孙进程**。因此判据按**原设计**实现（内核锁 → 与"壳的直接子进程"1:1 对应），
> **不启用** pid + 启动时间降级路径。三条实测同时确认：父 SIGKILL 后锁仍被持有、
> 子进程死亡后锁自动释放、孙进程存活不影响判定。

### 2.2 生命线管道

| 项 | 约定 |
|:---|:---|
| 形态 | 壳持**写端**（永不写入），watcher 持**读端**（经 `Stdio` 交给其 **stdin**） |
| 语义 | 壳以任何方式死亡 → 内核关闭写端 → watcher 读到 EOF |
| 作用域 | watcher 只对**登记时给的 pid 白名单**发信号——**不是进程组**（ADR-0015 §2.2） |
| 载体 | unix：`/bin/sh` 短脚本；Windows：不适用（走 Job Object） |
| **通道纪律** | **禁止固定 fd 号**（`dup2` 到魔数 + `read <&N`）。曾因此误杀正常子进程（ADR-0015 §7.1）；一律走 `Stdio` |
| **失败方向** | 读不到生命线（fd 异常）→ **放弃收口**，绝不误杀；收口另有启动期清扫兜底 |

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
- **幂等**：目标 pid 已消失时全部动作 no-op，不报错；
- **不阻塞壳退出**：watcher 与清扫都在独立线程/进程，不得拖慢 `RunEvent::Exit`；
- 清扫必须在 **probe 之前**完成（否则可能与新一代探测竞争同一份 profile 目录）。

### 3.4 覆盖范围（必须全部登记，无例外）

`spawn_dsh`（服务本体）、`executor` wsl 启动、`run_dsh_forward`（插件装卸，**用户恢复流程主路径**）、
`dsh --dump-config`、`repair-session.mjs`、`engines` 的 **`pnpm add -g`（分钟级）**、
`tar` 解包、各版本探测、`taskkill`。

> 漏登记一处 = 该处的孤儿不受清扫管辖。

---

## 4. 禁止外部访问的内部实现

- `procs/<pid>.lock` 的**布局与内容**是内部实现：消费方（含未来的 UI）不得依赖其字段，
  只可依赖 `sweep_orphans` 的返回值；
- 生命线 fd 编号、watcher 的命令行形态、Job Object 名称：内部实现；
- **严禁**任何模块直接读写、删除、加解锁 `~/.dsh/sessions/**/session.lock`——
  那是 dsh 的协议面，壳碰它就是违约（ADR-0015 方案 D）。

---

## 5. 测试闸门

| # | 测试 | 类型 | 现状预期 |
|:--|:---|:---|:---|
| 1 | **父被 `SIGKILL` → 子进程在 grace 内消失** | 集成（复现先行） | ✅ 且**必须带观察窗**：壳存活期间子进程不得消失（否则"误杀"会冒充"收口"而假绿，ADR-0015 §7.1） |
| 1b | 对照组：**未经守卫**的裸 spawn 在父横死后子进程仍存活 | 集成 | ✅ 钉住"问题确实存在"，防误以为父死子必死是内核行为 |
| 2 | 清扫：陈旧锁文件（持有者已死）→ 删除且不误杀 | 单测（fixture） | 新增 |
| 3 | 清扫：活锁（持有者存活）→ 收口并留痕 | 单测（fixture） | 新增 |
| 4 | 清扫：**不触碰**非登记进程（负例） | 单测 | 新增 |
| 5 | 守卫失败（管道/赋值失败）→ 子进程照常运行，仅 warn | 单测 | 新增 |
| 6 | 单向性：子进程自行退出 → 壳不退出、不重启（除既有熔断器） | 集成 | 新增 |
| 7 | Windows 停止参数构造（`taskkill` 回退路径） | 纯函数单测 | 已有（ADR-0014） |
| 8 | 平台分叉：三平台各自编译通过 + 目标语义测试 | CI 三平台 | 已有闸门 |

---

## 6. 实机验证清单（`docs/executor.md` 增补）

- [ ] macOS：`kill -9` 壳 → `ps` 无残留 dsh；被它锁住的会话可正常打开（**本次事故的直接回归项**）
- [ ] macOS：`cargo tauri dev` 连续重编重启 5 次 → 孤儿数为 0
- [ ] macOS：工作台内起一个长驻 dev server → 壳退出后**它仍然活着**（ADR-0015 §2.2 的负向验证）
- [ ] Windows：任务管理器强杀壳 → dsh 进程树消失
- [ ] Windows：`taskkill` 回退路径（Job 赋值失败时）仍能收口
- [ ] WSL：客体 wrapper + 宿主守卫双路径
- [ ] 引导期：`pnpm add -g` 进行中强杀壳 → 下次引导不被 store 锁阻塞

---

## 7. 演进规则

- **收窄容易、放宽难**：任何把作用域从"壳生的 pid"放宽到"进程组/整树"的改动，
  都是对 ADR-0015 §2.2 的破坏，须重开 ADR；
- AGENTS §4.1 口径同步：从"spawn 一律经 `child_cmd`"扩展为
  "**spawn 一律经 `child_cmd` + `lifecycle::spawn_guarded`**"（需宪法级改动流程 §10）；
- 新增 spawn 面时**必须**同步登记（§3.4），并在本契约的消费方列表中加入该模块；
- 若 dsh 上游自带父死自退（ADR-0015 §8 复审条件），本契约降级为冗余层，可整体撤回。

---

## 8. 建议实施顺序（分刀，AGENTS §8.1 一次一个意图）

| 刀 | 内容 | 依赖 |
|:--|:---|:---|
| 刀 0 | fd 继承存活性实测（§2.1 前置） | ✅ 2026-09-10 完成（ADR-0015 §7） |
| 刀 1 | 复现先行的红测试（§5-1） | 无 |
| 刀 2 | `lifecycle.rs`：清扫（§5-2/3/4）——**最小可用止血** | 刀 0/1 |
| 刀 3 | unix 生命线 watcher（§5-1 转绿） | 刀 2 |
| 刀 4 | `child_cmd` seam 接入全 spawn 面（§3.4） | 刀 3 |
| 刀 5 | Windows Job Object（裸 FFI）+ 回退 | 刀 4 |
| 刀 6 | dev/prod `DSH_HOME` 隔离 | 无（可并行，收益最大） |
| 刀 7 | 契约/AGENTS/广播落档 | 刀 5/6 |

> 刀 6 独立且成本最低，建议**最先做**——它是本次事故的放大器，隔离后 dev 的泄漏
> 再也伤不到正式包。
