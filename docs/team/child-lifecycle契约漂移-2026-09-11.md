# C2 契约漂移诊断：`child-lifecycle.md` ↔ `lifecycle.rs` 全量对照（2026-09-11）

> 执行人：`rust-core` ｜ 任务：`task-29` ｜ 来源：`docs/team/待裁定清册-2026-09-11.md` C2
> 性质：**只读诊断**。`docs/contracts/child-lifecycle.md` 与 `src-tauri/**` **一行未改**；
> 本报告是唯一产物。修复由 lead 裁定后另派。
>
> **证据强度三档**（`docs/team/README.md` §4）：**实测** = 贴命令/引原文 + 行号；
> **推断** = 说明推理链与缺口；**无法判定** = 写明缺什么证据。
>
> 基线：HEAD = `a34b7b1`；工作树另有他人改动（`frontend/**`、`mcp.rs`、`plugins.rs`），
> 与本诊断无关；`child-lifecycle.md` 与 `lifecycle.rs` 均 clean（`git status` 实测）。

---

## 0. 结论摘要

**这条漂移不是「文档写错了」，而是「一份写于实施之前的草案，从未随实施回填」。**
事实记录与规范承诺**混在同一份文件里**，须**逐节定性、分别处置**——整体二选一会做错其中一半。

| 判定 | 章节 | 性质 | 处置方向 |
| :-- | :-- | :-- | :-- |
| **已兑现的规范承诺** | §2.1–§2.3 / §3.1–§3.4 / §4 / §7 | 契约有效，实现守住 | **保留**，个别措辞回填 |
| **未生效的实施前草案** | §1（接口签名）/ §8（分刀计划） | 从未被 ADR 批准、无任何消费方引用 | **改契约**（§1 回填现状；§8 回收） |
| **真实现缺口** | §5 行 6（单向性缺测试）/ 行 7（回退路径未复用纯函数） | 规范承诺**未兑现** | **改实现**（范围小、风险低） |
| **文档侧缺口** | §6（7 项实机清单 → `executor.md` 只有 1 项）/ 台账未登记 | 承诺悬空 | **改契约 + 补台账** |

**三条最关键的发现**（详见 §2）：

1. **§1 的签名从未被 ADR 批准**——`spawn_guarded` / `guard` / `ChildGuard` / `Role::Tool`
   在 ADR-0015 全文**零出现**（实测 grep 空）。§1 是实施前的设计草图，且其失败语义**自相矛盾**
   （`→ Err` 同时要求「调用方不得因此放弃子进程」）→ **不构成「实现违约」**，应改契约。
2. **本契约根本不在 `docs/contracts/README.md §5` 契约台账里**（实测）。而 README §5 的
   WARNING 原文正是「台账里没有、但两个模块正在共享的东西 = 未登记的事实契约」——
   **契约自己成了未登记的事实契约**。这解释了 C2 为何会漂移这么久：没有台账指针，就没人复核。
3. **`lifecycle.rs` 是纯叶模块**（只 `use std::path / std::process / std::sync` + `nix`，
   零 `crate::` 依赖，实测）——这正是 §3.1「单向性」的结构性保证，也说明**补闸门 6 不需要
   Tauri 集成环境**（方案见 §4）。

---

## 1. 契约定性：证据（不猜）

任务是「改文档迁就实现」还是「改实现对齐契约」，取决于定性。**证据指向「两层定性」**。

### 1.1 证据 A：它是**契约**（规范性承诺）——文件自身与规范都这么说

| 证据 | 原文（实测引用） |
| :-- | :-- |
| `docs/contracts/README.md:3-4` | 「防止有人（或某个 AI）绕过公共接口改内部实现……契约 = 「**这一面不许悄悄变**」的书面承诺。」 |
| `docs/contracts/README.md:18` | 「判断口诀：**消费者不在你这次 commit 里 = 有契约。**」 |
| `docs/contracts/README.md:71` | 「新增/修改契约时，契约测试与契约文档必须在**同一个 PR**。」 |
| `docs/contracts/README.md:99-101` | 「**向后兼容默认义务**……破坏性变更 = 升 format + 迁移说明写入契约「演进规则」节」 |
| `child-lifecycle.md:3-4` | 「本文件是 **ADR-0015** 的配套方案文档……ADR 记决策，本册记**不许悄悄变的那一面**。」 |
| `child-lifecycle.md:6` | 按 README §2 标准格式写了「版本｜状态｜消费方」 |
| `docs/adr/0015…md:189` | 行动项：「契约文档：新增 `docs/contracts/child-lifecycle.md`（谁 spawn 谁收尸 + 平台差异 + 测试闸门）」 |
| `docs/broadcasts.md:427` | ADR-0015 落档条目把该契约与 `lifecycle.rs` 并列为交付物 |

**且 §1 判定条件命中**（README §1 第 3 行「被两个以上模块依赖的稳定内部接口」）——
实测 **6 个模块**直接调用 `lifecycle::spawn`/`run`：`shell.rs:101`、`executor.rs:926,1137`、
`engines.rs:260,344,462,501`、`resolve.rs:238,583`、`sessions.rs:521,596`、`profiles.rs:493`。
**定性结论（强证据）：§2–§5、§7 是有效契约，实现必须守它。**

### 1.2 证据 B：§1 是**实施前草案**（事实记录位）——四条独立证据

| 证据 | 原文 / 实测 |
| :-- | :-- |
| ① 状态头自述 | `child-lifecycle.md:8`：「版本：**v1（草案，随 ADR-0015 评审；实施未开始）**」 |
| ② §1 标注「新增」 | `child-lifecycle.md:29`：`// src-tauri/src/lifecycle.rs（新增）` ——写于文件存在**之前** |
| ③ **ADR 从未批准这些签名** | `grep -n "spawn_guarded\|ChildGuard\|Role::Tool\|guard(" docs/adr/0015…md` → **零命中**（实测）。ADR 只说「单点 seam」「裸 FFI 不引 windows-sys」，**没规定签名**。→ §1 的签名是契约作者自拟的，**未经 ADR 追认** |
| ④ 失败语义自相矛盾 | `child-lifecycle.md:35-36`：「守卫装配失败 → **Err**；**调用方不得因此放弃子进程**，应记 warn 后继续」——返回 Err 又要求调用方忽略它。**这条无法按字面实现**，实现另择了正确解法（见 §3 第 5 条） |

**定性结论（强证据）：§1 是设计草图，不是被违反的承诺**；§8 同理（见 §3 第 19 条）。
**处置方向：改契约。** 把 `spawn_guarded` 之类「加回实现」是**错误方向**——那会为了迁就一份
未被批准的旧草图，给 10 个调用点穿一个 `data_dir` 参数并引入一个无用的 `ChildGuard` 类型。

### 1.3 定性的最终口径（建议 lead 采纳）

> **同一份文件，两层效力**：
> - **§2–§5、§7 = 契约**（跨 6 模块的稳定内部接口 + 行为承诺）→ 实现守契约；**本次唯一需要改实现的只有 §5 行 6/行 7**；
> - **§1、§8 = 实施前计划**（无 ADR 追认、无消费方引用）→ 按 README「实施记录」位处理：**回填现状 / 回收**。

---

## 2. 不一致全清单（实测，逐条给行号）

> 「契约处」= `docs/contracts/child-lifecycle.md`（L 行号）；「实现处」= `src-tauri/`。
> 归属判定 = 我建议处置哪一边（**裁定权在 lead**）。

### A. §1 接口签名（草案位）

| # | 契约处 | 实现处 | 差异 | 归属判定 |
| :-- | :-- | :-- | :-- | :-- |
| 1 | L32 `Role { …, Probe, Tool }` | `lifecycle.rs:36-47`（5 变体，**无 `Tool`**） | 多一个变体 | **改契约**（`Tool` 零使用；`taskkill` 走 `Role::Probe`，`shell.rs:248`） |
| 2 | L37-38 `guard(child, data_dir, role, ctx) -> Result<ChildGuard>` | **不存在** | 函数与类型均无 | **改契约**（RAII 内化为 `Registration`，见 L485-580） |
| 3 | L42-43 `spawn_guarded(cmd, data_dir, role, ctx) -> Result<(Child, ChildGuard)>` | `spawn(cmd, role, ctx) -> io::Result<Child>`（`lifecycle.rs:417`） | 改名 + 去 `data_dir` + 去元组 | **改契约**（`data_dir` 由 `active_data_dir()` 全局/线程覆盖给出，理由见 `lifecycle.rs:166-179`） |
| 4 | **§1 未收录** `run()` | `lifecycle.rs:465` `pub fn run(cmd, role, ctx) -> io::Result<Output>` | 公开 seam 漏登记 | **改契约**（补文档；`run` 是实际使用最广的 seam，11 处调用） |
| 5 | L35-36 失败语义「→ Err；调用方不得因此放弃」 | `lifecycle.rs:417-457`：守卫装配失败**从不**冒泡（内部 warn 降级），`Err` 仅表示**进程启动失败** | 语义相反 | **改契约**（实现的解法更正确，且与 §3.1「守卫自身故障 → 壳照常运行」一致） |

> 实测：`mod lifecycle;`（`lib.rs:26`）是**crate 私有**——§1 全部签名都不是对外 API，
> 所以「契约面」其实是**内部 seam + 行为承诺**，而非 pub 签名清单。这进一步支持把 §1 降为现状记录。

### B. §2 数据模型

| # | 契约处 | 实现处 | 差异 | 归属判定 |
| :-- | :-- | :-- | :-- | :-- |
| 6 | L67 `procs/<pid>.lock`（§4 L153 重复） | `lifecycle.rs:79-81` + `:361-363`：`procs/<token>.lock`，token = **父 pid**-序号-纳秒 | 文件名不是子进程 pid；实机样例 `10122-9-1789105217877284000.lock` | **改契约**（实现有充分理由：加锁发生在 spawn **之前**，此刻子 pid 尚不存在——见 `lifecycle.rs:356-360`） |
| 7 | L69 内容 `{pid, role, profile, started_at_ms, argv0}` | `lifecycle.rs:374-383` → JSON key **`startedAtMs`**（camelCase） | 字段名不符 | **改契约**（内容已声明「仅排查用」，改文档零风险） |
| 8 | L68 锁「`pre_exec` 清 `FD_CLOEXEC`」 | `lifecycle.rs:538-549` 用 `dup2`（dup2 本身清除目标 fd 的 CLOEXEC） | 措辞 | ✅ **一致**（实现注释已说明机制） |
| 9 | L74-77 前置实测（ADR-0015 §7） | 与 ADR-0015:210-219 逐条一致 | 无 | ✅ 一致 |
| 10 | L83-88 §2.2 生命线五项 | `lifecycle.rs:618-673`（`WATCHER_SCRIPT` + `Lifeline`） | 无 | ✅ 一致（含「禁止固定 fd 号」——`watcher_script_contract` 测试钉住） |
| 11 | L96-98 §2.3 三态 | `lifecycle.rs:719-725` `classify_lock_result` | 无 | ✅ 一致 |
| 12 | L67 / §2.1 **未规定**登记锁的 fd 号 | `lifecycle.rs:584` `INHERITED_LOCK_FD = 198`（`dup2` 进 `pre_exec`） | 契约静默 | **建议补契约**（涉及 §7.1 同类技法，见 §5 附带发现 B2） |

### C. §3 行为承诺

| # | 契约处 | 实现处 | 差异 | 归属判定 |
| :-- | :-- | :-- | :-- | :-- |
| 13 | L114「壳死 → 子必须死」 | `lifecycle.rs:1048` 测试 | 无 | ✅ 一致（+ 对照组 `:1174`） |
| 14 | L115「子死 → 壳不动」 | `boot.rs:544-550`（`guard_session` 只 `emit_boot_error` + `return`，**不调 `app.exit()`**） | 行为正确，**无测试** | 见 §4（闸门 6 主项） |
| 15 | L116「子死 → 自动重启仅由熔断器管」 | `boot.rs:489-541`（`auto_restart` 默认 false + 60s 内 3 次熔断） | 行为正确，**无测试** | 见 §4（闸门 6 次项） |
| 16 | L117「守卫自身故障 → 壳照常运行」 | `lifecycle.rs:110-111`（装配失败降级 warn）+ `:417-421` | 无 | ✅ 一致（`guard_failure_does_not_block_child`） |
| 17 | L127-129 §3.2 平台差异三行 | unix `shell.rs:219-239` ✓；Windows `shell.rs:240-276` ✓；WSL `executor.rs:577-585`（客体 `while [ ! -f /tmp/dsh-dock-stop ]… kill -TERM "$PID"` 看门狗**确实存在**）✓ | 无 | ✅ 一致 |
| 18 | L136「收口序列…→ **`wait()` 回收**」 | `stop_dsh` 有 `wait()`（`shell.rs:238`）；清扫的 `reap_pid`（`lifecycle.rs:853-874`）**无法** `wait()`——它不是孤儿的父进程 | 措辞过宽 | **改契约**（措辞）或标注「清扫路径按存活探测而非 wait」 |
| 19 | L137-139 幂等 / 不阻塞退出 / 清扫早于 probe | `lifecycle.rs:860,871`（ESRCH=成功）；`:663-665`（收割线程）；`lib.rs:278` + 文本闸门 `lifecycle.rs:1718` | 无 | ✅ 一致 |
| 20 | L143-145 §3.4 九个 spawn 面「必须全部登记」 | 逐项实测：`spawn_dsh`→`shell.rs:101`；wsl→`executor.rs:926`；`run_dsh_forward`→`profiles.rs:493`；`dsh --dump-config`→**间接**（`plugins.rs:888` 调 `profiles::run_toolchain_forward`，spawn 落在 `profiles.rs:493`）；`repair-session.mjs`→`sessions.rs:596`；`pnpm add -g`→`engines.rs:462,501`；`tar`→`engines.rs:344`；版本探测→`resolve.rs:238,583`；`taskkill`→`shell.rs:246` | **九个面全部已登记 ✓**，但清单**漏列**两处实测存在的面：`session-scan.mjs`（`sessions.rs:521`）、`wsl.exe base64 投递`（`executor.rs:1137`） | **改契约**（补两项，或把「无例外」改为「列举见 X」避免穷尽承诺） |

### D. §4 禁止外部访问

| # | 契约处 | 实现处 | 差异 | 归属判定 |
| :-- | :-- | :-- | :-- | :-- |
| 21 | L153 `procs/<pid>.lock` | 同 #6 | 同 #6 | **改契约**（同步修） |
| 22 | L155「生命线 **fd 编号**…内部实现」 | 实现走 `Stdio`，**根本没有 fd 编号**（且 §2.2 明文禁止） | 残留描述 | **改契约**（删除「fd 编号」） |
| 23 | L155「**Job Object 名称**：内部实现」 | `lifecycle.rs:299` `CreateJobObjectW(null, null)` = **未命名** job | 暗示具名 | **改契约**（改为「job 句柄与限制参数」） |
| 24 | L156-157 严禁触碰 `session.lock` | 实测全仓**零**路径级引用（唯一提及是 `lifecycle.rs:23` 的禁止性注释） | 无 | ✅ 一致（**这条守得最好**） |

### E. §5 测试闸门（含本任务重点：行 6、行 7）

| # | 契约处 | 实现处 | 差异 | 归属判定 |
| :-- | :-- | :-- | :-- | :-- |
| 25 | L165 行 1 | `lifecycle.rs:1048` `guarded_child_dies_when_parent_is_sigkilled`（**含 1.5s 观察窗**） | 无 | ✅ 一致 |
| 26 | L166 行 1b | `lifecycle.rs:1174` `unguarded_child_survives_parent_sigkill_control` | 无 | ✅ 一致 |
| 27 | L167-170 行 2/3/4/5 现状栏写「**新增**」 | `:1216` `sweep_deletes_stale…` / `:1250` `sweep_reaps_live_orphan…` / `:1288` `sweep_never_touches_unregistered…` / `:1340` `guard_failure_does_not_block_child` **均已存在** | 状态栏过期 | **改契约**（「新增」→「✅」） |
| 28 | **L171 行 6** 单向性 | **无对应测试**（`lifecycle.rs` 25 个 `#[test]` 全量核对；`boot.rs` 仅 4 个 handoff 测试，实测 `grep -c`） | **规范承诺未兑现** | **改实现**（补测试，方案见 §4） |
| 29 | **L172 行 7**「Windows 停止参数构造（`taskkill` **回退路径**）｜纯函数单测｜已有（ADR-0014）」 | 纯函数 `shell::windows_kill_args`（`shell.rs:283-290`）+ 测试 `:384` **确实存在**；但**回退路径不用它**——`lifecycle.rs:887` **内联** `vec!["/PID", pid, "/T", "/F"]` | 测试覆盖的是 `stop_dsh` 路径，**不是**契约点名的回退路径 | **改实现**（见 §3 第 29 条；范围小） |
| 30 | L174 行 9 | `lifecycle.rs:1645` `real_dsh_is_reaped_when_shell_is_sigkilled`（`#[ignore]`） | 无 | ✅ 一致 |
| 31 | L175 行 10 | `:1578` `lock_verdict_separates_would_block_from_io_error` + `:1607` `only_would_block_may_authorize_reaping` | 无 | ✅ 一致 |
| 32 | L176 行 11 | `:1718` `boot_sweeps_orphans_before_dispatching_executor` | 无 | ✅ 一致 |
| 33 | L177 行 12 | `:1491` `production_spawns_go_through_lifecycle_seam`（扫描 4 个入口） | 无 | ✅ 一致 |
| 34 | L173 行 8「CI 三平台」 | `.github/workflows/build.yml:124-133` 在**三平台 matrix 各自**跑 `cargo clippy --all-targets -- -D warnings` | 无 | ✅ 一致 |

### F. §6 / §7 / §8 与文件头

| # | 契约处 | 实现处 / 现状 | 差异 | 归属判定 |
| :-- | :-- | :-- | :-- | :-- |
| 35 | L183-189 §6 **7 项**实机清单（标注「`docs/executor.md` 增补」） | `docs/executor.md` §C（:211-222）**只有 1 项**（Windows 强制结束 + tasklist）。实测 grep：`kill -9`=0、`dev server`=0、`长驻`=0、`pnpm add -g`=0 | 契约承诺的 7 项里 **6 项未落进被引用文档** | **改契约 + 补文档**（需 lead 裁定：这是「待跑的承诺」还是「计划」，见 §3） |
| 36 | L202-203「AGENTS §4.1 口径同步为 `child_cmd` + **`lifecycle::spawn_guarded`**」 | `AGENTS.md:143` 实际写的是「新增 spawn 一律经 **`lifecycle::spawn`/`run`**」 | 名称不符 | **改契约**（这条已生效，只是名字错了） |
| 37 | L195-198 §7 变异纪律（声称三条已做） | 关生命线→真 dsh 用例红 ✓（`docs/team/网络面闸门…` 同范式）；删清扫接线→闸门红 ✓（`lifecycle.rs:1708-1713` 注释有记录）；去 dev 门→release 用例红 ✓（`resolve.rs:1196-1211` `home_dir_names_are_distinct_and_stable`） | 无 | ✅ 一致（**且这条纪律正是本任务 §4 方案要照抄的范式**） |
| 38 | L204 新增 spawn 面须同步登记 | 无机器闸门强制（`production_spawns_go_through_lifecycle_seam` 只覆盖**已列入 SOURCES 的 13 个文件**，新文件不会被扫——实测 `lifecycle.rs:1492-1506`） | 承诺无强制 | **改实现**（可选，见 §5 附带发现 B3） |
| 39 | L211-220 §8 **八刀计划表** | 刀 0–7 **全部已交付**（`broadcasts.md:427` 落档 + 逐项实测） | 计划已完成，未回收 | **回收建议**（计划表属「实施期工件」，勿长期驻留契约正文） |
| 40 | L8 状态头「**实施未开始**」 | 已实现、已发布（v1.1.1）、已落档广播 | **事实相反** | **改契约**（改为「已实施，v1.1.1 起生效」+ 日期） |
| 41 | L9-11 消费方列 `boot.rs` | 实测 `boot.rs` **零** `lifecycle::` 引用；`lib.rs:178,278` 才是直接消费方（`init` / `sweep_orphans`）却**未列入** | 一个假阳性 + 一个假阴性 | **改契约**（改列 `lib.rs`；`boot.rs` 标注为「§3.1 行为宿主，非 §1 接口消费方」） |

### G. 文件外：台账缺口（**本诊断最意外的发现**）

| # | 事实 | 判定 |
| :-- | :-- | :-- |
| 42 | `docs/contracts/README.md:105-111` §5 契约台账列了 5 份契约（contract.md / node-map / IPC / executor.md / updater feed），**没有 `child-lifecycle.md`** | **补台账** |
| — | README §5 WARNING 原文（L113-115）：「台账里没有、但两个模块正在共享的东西 = **未登记的事实契约**。发现即登记」 | 本契约**自身**命中该警告 |

> **推断（非实测）**：C2 能漂移至今，与「台账无指针 → 无人定期复核」高度相关。

---

## 3. 逐条方案与影响面

> 原则：**能改文档就改文档**（实现是经过实机验证的、且 6 模块已依赖它）；**只有真缺口才改实现**。

### 3.1 改契约（零代码风险）——覆盖 #1–#12、#18–#24、#27、#36、#40–#42、#35（部分）

| 动作 | 具体 | 影响面 |
| :-- | :-- | :-- |
| §1 重写为**现状记录** | 删 `guard` / `spawn_guarded` / `ChildGuard` / `Role::Tool`；补 `init` / `run` / `spawn` 三签名的**真实形态**；失败语义改为「守卫装配失败内部降级（warn），`Err` 仅表进程启动失败」 | 无代码影响；**降低后人误信草图而改实现的风险**（这正是本任务存在的理由） |
| §2.1 / §4 修路径与字段 | `<token>.lock`（并说明 token = 父 pid-序号-纳秒、为何不用子 pid）；`startedAtMs` | 无（内容已声明「仅排查用」） |
| §4 删两处残留 | 删「生命线 fd 编号」；「Job Object 名称」→「job 句柄与限制参数（未命名）」 | 无 |
| §3.3 措辞 | 「→ `wait()` 回收」限定为「正常停止路径」；清扫路径改为「按存活探测（非父进程，无法 wait）」 | 无 |
| §3.4 补两项 | 增 `session-scan.mjs`（`sessions.rs:521`）、`wsl.exe base64 投递`（`executor.rs:1137`） | 无 |
| §5 状态栏 | 行 2/3/4/5「新增」→「✅」（附测试名） | 无 |
| §7 / 状态头 / 消费方 | `spawn_guarded` → `spawn`/`run`；「实施未开始」→「已实施（v1.1.1）」；消费方补 `lib.rs`、`boot.rs` 改注 | 无 |
| **§8 回收** | 八刀计划全部交付 → 按 AGENTS §11.4「回收触发 = 已升格 ADR / 已有测试 CI 兜底」**整节回收**（或折叠为一行「实施记录见 `broadcasts.md` 2026-09-10 条目」） | 无；**收益**：契约正文去掉 15 行已失效内容 |
| **补台账** | `docs/contracts/README.md §5` 增一行：`子进程生命周期 | docs/contracts/child-lifecycle.md | v1 | src-tauri 全部 spawn 面（6 模块）` | 无；**这是防复发的关键一步** |

### 3.2 改实现（真缺口，**本轮不动手**，仅列范围与风险）

**第 28 条 —— §5 行 6 单向性测试缺失**
- 范围：`src-tauri/src/lifecycle.rs`（新增测试 + 可能新增一个 env 门控 role 探针）、
  可选 `src-tauri/src/boot.rs`（抽纯函数）。
- 见 §4 完整方案。风险：**低**（纯加测试；唯一实际改动是可选的小重构）。

**第 29 条 —— `taskkill` 回退路径未复用纯函数，且绕过 `child_cmd`**
- 现状：`lifecycle.rs:887-888` 内联 `vec!["/PID", pid, "/T", "/F"]` + **裸 `Command::new("taskkill")`**。
- 双重问题：① 契约点名的「纯函数单测」覆盖不到这里；② **违 AGENTS §4.1**「Windows 子进程一律经
  `crate::child_cmd`」——而 `child_cmd` 的作用之一正是**防终端弹窗**（对照 `shell.rs:244` 的同一动作
  **是**走 `child_cmd` 的，两处不一致）。
- ⚠️ **一处必须避免的陷阱（本诊断的关键约束）**：**不要**让 `lifecycle.rs` 去调
  `shell::windows_kill_args` —— `shell.rs:101` 已依赖 `lifecycle`，反向依赖会造成**模块环**，
  且会让 `lifecycle.rs` 从纯叶模块变成依赖链内部节点，**破坏 §4 方案里 Tier-1 单向性闸门的前提**
  （见 §4.1）。→ 可选项：
  - **A（推荐，最小）**：`lifecycle.rs` 的 Windows 回退改走 `crate::child_cmd(Path::new("taskkill"))`
    （`child_cmd` 在 crate 根，子模块可见，无环）；args 重复保留 + 加注释交叉引用 `shell::windows_kill_args`。
  - **B**：把 args 构造下移到 `lifecycle.rs` 自己的纯函数（`windows_reap_args(pid)`）并加
    `#[cfg(windows)]` 单测 —— 仍有两份同形函数，但各自可测。
  - **C**：把 `windows_kill_args` 提升到中立位置（如 `lib.rs`）——改动面最大，需动 `shell.rs`。
- 风险：Windows-only 路径，**macOS 上无法运行时验证**（只能 `clippy --target x86_64-pc-windows-gnu`
  + 真机清单）；`child_cmd` 的引入是行为等价（仅加 `CREATE_NO_WINDOW`）。

**第 38 条 —— 新增 spawn 面无强制登记（可选）**
- 现状：spawn 闸门用 `include_str!` **白名单 13 个文件**，**新文件不会被扫**。
  （对照：`network_gate.rs` 用**运行时遍历** 35 个文件，无此盲区——但那是网络原语，语义不同。）
- 可选项：把 `source_files()` 式遍历引入 spawn 闸门。**风险**：`lifecycle.rs` 自身含
  `cmd.spawn()`（seam 本体，`lifecycle.rs:436`）与 `cmd.status()`（Windows 回退，`:893`），
  需条目级豁免——否则闸门自红。**建议列独立任务**，不夹带进 C2。

### 3.3 需 lead 裁定（我给两方证据，不预设）

**第 35 条 —— §6 那 7 项实机清单是什么性质？**

| 支持「承诺（须补跑）」 | 支持「计划（可回收）」 |
| :-- | :-- |
| 契约 L22 总纲「『谁 spawn 谁收尸』在*任何*死亡模式下都成立」＋ §6 明细是其**验收面** | L181 标题写「（`docs/executor.md` **增补**）」——是**待做的动作**，不是已达成的承诺 |
| 7 项含「本次事故的**直接回归项**」（L183 原文），属**事故回归**，不宜静默消失 | 全部未勾选 `[ ]`，与 §8 计划表同属实施期工件 |
| `docs/team/README.md` §6 明写「唯一未验证项 = Windows 真机人工验证，待维护者跑并回填」 | — |

**我的建议**：**拆开处理**——macOS 三项（L183-185）与本机可跑的「引导期强杀」项（L189）
**属可执行承诺，建议留在契约并列入待跑清单**；Windows/WSL 两项**已在 `executor.md` §C/§D 有落点**，
契约侧改为**指针**（避免双源）；同时把「3 项 macOS + 1 项引导期」补进 `docs/executor.md`
（补文档 = 独立小任务，涉及 `docs/executor.md`，**不在我域**）。

---

## 4. §5 闸门 6：缺失的「单向性」是什么 + 可执行测试方案

### 4.1 缺的到底是什么（分解）

契约 L171：**「单向性：子进程自行退出 → 壳不退出、不重启（除既有熔断器）｜集成｜新增」**。
它其实是**两条独立断言**，且**第二条的实现位置**决定了测试形态：

| 子断言 | 现状实现 | 实测证据 | 有无测试 |
| :-- | :-- | :-- | :-- |
| **(6a) 壳不退出** | `guard_session` 只 `emit_boot_error` + `return`，**从不调 `app.exit()`** | `boot.rs:544-550` | ❌ 无 |
| **(6b) 不重启（除熔断器）** | `auto_restart` 默认 false（`boot.rs:489` `unwrap_or(false)`）+ 60s 内 ≥3 次熔断（`:497`） | `boot.rs:489-541` | ❌ 无 |

**为什么至今没有测试**：契约把行 6 标为「**集成**」——而本仓库**没有 Tauri AppHandle 的集成测试
基建**（无 dev-dependencies，AGENTS §5；`boot.rs` 仅 4 个纯逻辑 handoff 测试）。
同时 (6b) 的熔断逻辑**内联在 `guard_session` 里**（`boot.rs:491-542`），依赖 `State`/`AppHandle`，
**无法直接单测**。

> ⚠️ 因此：**「集成」这个类型标注本身就是不可兑现的**。诚实做法是**降低标注到当前可验证的形态**，
> 或为它立 ADR 引入集成测试基建（后者违反「不引测试依赖」→ 必须走 ADR）。**建议前者**。

**「单向性」的准确含义**（引 ADR-0015:76 原文）：
> 「**看门狗语义严格单向**：`壳死 → 子死` ✅；`子死 → 壳不动` ✅；`子死 → 自动重启` ⚠️ 仅由既有熔断器（60s 内 3 次）管，**看门狗不得自维重启**。」

即：**依赖方向**不得反转——`lifecycle`（守卫）只被壳调用，**绝不能反向操作壳的存活/重启**。

### 4.2 关键结构性事实（方案的地基）

**实测**：`lifecycle.rs` 只 `use std::path / std::process / std::sync`（`:26-28`），
其余全是 `std::` / `nix` / `serde_json` 全路径——**零 `crate::` 依赖**，
且实测 grep `app.exit|AppHandle|begin_boot|launch_executor|RunEvent|restart` → **零命中**。

⇒ **「守卫不能反向影响壳」在本仓库是一条可静态判定的不变量**，
**不需要 Tauri 环境就能把它钉死**。这正是 Tier-1 的可行性依据。

### 4.3 三级测试方案（照抄 `network_gate.rs` / spawn 闸门的范式）

#### Tier 1 —— 方向哨兵：源码文本闸门（**成本最低、价值最高、可变异证伪**）

- **形态**：在 `lifecycle.rs` 的 `#[cfg(test)]` 内加纯函数
  `scan_shell_control_symbols(name, text) -> Vec<String>`，扫描 `lifecycle.rs` 自身源码
  （`include_str!("lifecycle.rs")`），命中**禁止符号**即失败。
- **禁止清单**（= 「反向影响壳」的可机读定义）：
  `AppHandle` · `app.exit(` · `begin_boot(` · `boot_superseded(` · `launch_executor_after_probe` ·
  `executor_for_mode(` · `RunEvent` · `crate::boot::` · `crate::ui::` · `crate::shell::`
- **必须照抄的两条纪律**（否则重蹈覆辙）：
  1. **CRLF 归一**（`text.replace("\r\n", "\n")`）——v1.1.1 教训：Windows `autocrlf` 会让
     源码文本闸门在 Windows 恒红/失效（`lifecycle.rs:1430-1437` 已有同款注释，`.gitattributes` 已加，但仍应自查）；
  2. **`#[cfg(test)]` 段剔除**——否则本测试自己的 fixture 字符串会命中（`network_gate.rs` 的两个文本视图范式可直接复用）。
- **变异证伪（必做，照 `ADR-0015 §7` 的纪律）**：往 `lifecycle.rs` 临时插一行
  `let _ = crate::boot::emit_step;` → 闸门**必须红**；删除 → 绿。**不跑变异不算完成**。
- **收益**：把「§3.1 单向语义」从**没人守的口头承诺**变成**改回去即红**的结构性保证；
  且它同时保护了 §3.2 的收窄作用域（`lifecycle` 一旦能碰壳状态，「只杀壳生 pid」的约束就会被绕开）。

#### Tier 2 —— 行为级：子进程自行退出，父侧存活（进程级复现先行范式）

- **形态**：照抄 `guarded_child_dies_when_parent_is_sigkilled`（`lifecycle.rs:1048`）的
  **「测试二进制自我唤起为父角色」**手法（现有基建：`scratch()` `:908`、`alive()` `:923`、
  `wait_until()` `:943`、`parent_role_probe` 的 env 门控 `:1121`）。
- **新增一个 env 门控 role（如 `one_way_role_probe`）**：`lifecycle::init(dir)` →
  `lifecycle::spawn(sleep 1, Role::DshServer)` → 落盘「父 pid + 子 pid」→ **原地阻塞 120s**（不清理）。
- **断言链**：
  1. 前置：子进程**确实起来了**（否则没复现到问题）；
  2. 等子进程**自行自然退出**（`sleep 1`，轮询 `alive(child_pid) == false`）；
  3. **核心断言**：子进程退出后再等 **≥ 3s**（= watcher grace 上限），父角色进程**必须仍存活**
     —— 若 watcher 把「子死」误当信号反馈给父侧，这里会红；
  4. 收尾：`SIGKILL` 父角色 + 清理 scratch 目录（不留孤儿，沿用现有用例的收尾写法）。
- **它证明什么**：子进程自行退出**不会**触发任何针对其父/壳的收口，
  生命线**严格单向**；同时间接覆盖「watcher 不因目标退出而误伤他人」。
- **注意**：父角色是**测试进程的替身**，不是真壳——**报告结论时必须写明这一层近似**，
  不得宣称「已证明壳不退出」（那是 Tier 3 的范围）。

#### Tier 3 —— 熔断决策纯函数（(6b) 的唯一可单测化路径，**需 lead 批准的小重构**）

- **形态**：把 `boot.rs:489-541` 的决策**抽出为纯函数**（**不改逻辑、不改行为**），例如：
  ```rust
  pub(crate) enum RestartDecision { Disabled, Restart, BreakerTripped }
  pub(crate) fn restart_decision(auto_restart: bool, crashes_in_window: usize) -> RestartDecision
  pub(crate) fn prune_crashes(now_ms: u64, stamps: &[u64], window_ms: u64) -> Vec<u64>
  ```
  `guard_session` 改为调用它们（副作用留在原地：`emit_*` / `thread::spawn` / `teardown`）。
- **用例**（正反例 + 边界，AGENTS §5）：
  `auto_restart=false` → `Disabled`（**默认值即「不重启」——(6b) 的第一道防线**）；
  `true` × 崩溃数 0/1/2 → `Restart`；`true` × 3 → `BreakerTripped`（**边界：`>=` 而非 `>`**）；
  窗口裁剪：59.9s 保留 / 60.1s 剔除 / 空数组 / 乱序输入。
- **变异证伪**：把 `>= 3` 改成 `> 3` → 边界用例必红。
- **范围与风险**：仅 `boot.rs`，约 15 行搬移 + 4–6 个测试；**逻辑等价**（同输入同分支），
  风险**低**；但属**行为代码重构**，按 AGENTS §8.1 应作为**独立意图**派单，不夹带。
- **诚实边界**：Tier 3 只证明**决策函数**正确，**不证明**「壳不会因它而退出」——
  壳存活仍由 Tier 1（结构性方向）+ 人工验证兜底。

### 4.4 方案小结（建议 lead 采纳的口径）

| 层 | 覆盖 | 成本 | 是否改生产行为 | 建议 |
| :-- | :-- | :-- | :-- | :-- |
| Tier 1 源码闸门 | §3.1 单向性（结构） | 低 | 否 | **做**（含变异证伪） |
| Tier 2 进程级 | §3.1 子死→父侧存活 | 中 | 否（纯加测试） | **做** |
| Tier 3 纯函数抽取 | §3.1 熔断（6b） | 中 | **是**（等价重构） | **建议做，独立派单** |
| 契约行 6 类型标注 | — | 极低 | 否 | **改契约**：「集成」→「源码闸门 + 进程级 + 纯函数单测」 |

> 若 lead 只想做一件事：**做 Tier 1**。它用一个上午的成本，把「最重要的一条」（契约 L111 原话）
> 从口头承诺变成可变异证伪的机器闸门，且**零生产行为风险**。

---

## 5. 附带发现（**非契约漂移**，但同源，供 lead 派单）

> 这三条不在 C2 定义内，诊断过程中实测发现；按 AGENTS §8.1 **只记不动**。

**B1（= 第 29 条）`lifecycle.rs:888` 裸 `Command::new("taskkill")` 绕过 `child_cmd`**
- 违 AGENTS §4.1；对照 `shell.rs:244` 同一动作**是**走 `child_cmd` 的。
- 加剧因素：spawn 闸门的 `SOURCES`（`lifecycle.rs:1492-1506`）**不含 `lifecycle.rs`**
  （豁免理由「它就是 seam 的实现」，见 `:1486`），故这处 `.status()` **机器闸门也拦不到**。
- 结论：AGENTS §4.1 是**人肉纪律**，在本处漏了；修复见 §3.2 选项 A/B/C。

**B2（推断，非实测）登记锁 fd 号 `198` 与 ADR-0015 §7.1 的教训同源**
- 实测：`lifecycle.rs:584` `INHERITED_LOCK_FD = 198`，经 `pre_exec` 里 `dup2` 注入子进程——
  **与 §7.1 被推翻的「固定 fd 号」是同一技法**（`docs/adr/0015…md:224-243` 记录该技法曾因
  「目标号在子进程里恰好空闲且不会被随后关闭」不成立而**误杀正常子进程**）。
- **为何当前大概率安全（推断）**：失败方向不同——生命线误判 EOF → **误杀**（不可挽回）；
  登记锁 fd 被关 → 锁提前释放 → 清扫判「陈旧」→ **漏收**（还有下次启动兜底），且 198 远离 stdio。
- **但我无法判定**：子进程（node）是否可能在运行中关闭 fd 198。**缺的证据** =
  一次「壳 `SIGKILL` 后，`lsof -p <orphan>` 确认 fd 198 仍持锁」的实机复现。
- **建议**：契约 §2.1 补一行说明该 fd 纪律与失败方向（**改文档**，无风险）；
  若要更强的保证，加一条真机锚用例（需引擎，`#[ignore]`）。**不建议本轮动**。

**B3（= 第 38 条）spawn 闸门用白名单，新文件不被扫**
- `lifecycle.rs:1492-1506` 是 13 项 `include_str!` 白名单；**新增 `.rs` 文件不在管辖内**——
  与 A1 那次「没人登记」是同一类缺口形态（`network_gate.rs` 已用运行时遍历解决同类问题）。
- 建议独立任务评估（注意 `lifecycle.rs` 自身需要条目级豁免，否则自红）。

---

## 6. 未决问题与建议下一步

1. **请裁定 §1/§8 的归属**（我的建议：§1 回填现状、§8 整节回收、补台账）——
   这是本诊断的核心交付，**无需改任何代码**。
2. **请裁定 §6 那 7 项的性质**（§3.3 给了两方证据与拆分建议）。
3. **请裁定闸门 6 走哪几级**（建议 Tier 1 + Tier 2 做、Tier 3 独立派单；契约行 6 类型标注同步改）。
4. **第 29 条（`taskkill` 回退路径）**建议作为**独立 fix 任务**派给我（我域内），
   并按 §3.2 的**选项 A** 实施——**关键是不得引入 `lifecycle → shell` 的反向依赖**。
5. **B2 / B3 只登记不动**，等 lead 排期。
6. **本轮未动任何文件**（除本报告）；`child-lifecycle.md` 与 `src-tauri/**` 均 clean。

---

## 7. 关联

- 来源：`docs/team/待裁定清册-2026-09-11.md` C2
- 被诊断对象：`docs/contracts/child-lifecycle.md`（223 行）↔ `src-tauri/src/lifecycle.rs`（1778 行）
- 规范依据：`docs/contracts/README.md`（§1 判定条件 / §2 标准格式 / §3 契约测试要求 / §5 台账 + WARNING）
- 决策依据：`docs/adr/0015-child-process-lifecycle-ownership.md`（§2.4 单向语义 L76 / §5 行动项 L184-192 / §7 刀 0 实测 / §7.1 被推翻的实现细节 L224-243）
- 宪法依据：`AGENTS.md:141-143`（§6 1:1 生命周期 + spawn 口径）、`:129`（`procs/` 登记）
- 落档：`docs/broadcasts.md:426-440`（ADR-0015 交付记录）
- 同范式参照：`src-tauri/src/network_gate.rs`（task-17，真变异单向性证明 + 两个文本视图 + CRLF 归一）
- 前序诊断：`docs/team/孤儿会话写锁-残留核对-2026-09-11.md`（该档 §三 R7 已列出部分同名漂移，本报告为其全量展开与归属判定）
