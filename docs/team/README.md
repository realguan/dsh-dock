# DSH Dock 研发团队编制（AI 协作）

> 状态：活文档 | 建立 2026-09-11 | 适用：本仓库全部 AI 协作会话
> 定位：**只回答「谁改哪里、谁验什么、谁有裁定权、什么必须落盘」**。
> 工程规范见 [`AGENTS.md`](../../AGENTS.md)（宪法）；人类协作流程见
> [`docs/CONTRIBUTING.md`](../CONTRIBUTING.md)；契约字段见 [`docs/contract.md`](../contract.md)。

## 0. 与既有文档的关系（禁双源）

本文件**不重复**宪法与协作指南的任何条文，只补「多角色同时开工」时的分工与仲裁。
三者冲突时以 `AGENTS.md` > `docs/CONTRIBUTING.md` > 本文件为序。
本文件不是宪法级文件，但改动须经 lead 并广播（团队编制变化会影响协作方式）。

## 1. 编制（5 名执行角色 + lead）

| 角色 | 域 | 独占写入范围 | 不碰 |
| :--- | :--- | :--- | :--- |
| `rust-core` | 壳运行时：启动链 / 生命周期收口 / 引擎引导 / 唯一网络面 | `src-tauri/src/{boot,boot_failure,lifecycle,shell,resolve,engines,executor,updates,updater,main}.rs`、`build.rs`、`Cargo.toml`、`rustfmt.toml` | 管理域模块、`frontend/**` |
| `rust-mgmt` | dsh 管理：profile / 插件 / 设置 / 凭据 / 会话 / MCP / 诊断 | `src-tauri/src/{profiles,plugins,settings,dsh_settings,credentials,sessions,mcp,diagnostics,manifest,fs_backup,build_policy}.rs`、`src-tauri/src/commands/` | 运行时模块、`frontend/**` |
| `frontend` | React SPA：页面 / 组件 / store / 事件消费 / 文案 | `frontend/**` | `src-tauri/**` |
| `docs-contract` | 契约与文档：ADR 体例 / contract / roadmap / 巡检落盘 | `docs/**`（除下列文件）、`README.md` | 生产代码 |
| `qa-verify` | **独立验证**：产物验收 / 闸门基线 / 对抗性复核 | 仅验证报告文件 | 一切生产代码 |
| `lead` | 仲裁 / 收敛 / 最终验收 | 宪法级文件、`docs/broadcasts.md`、`docs/team/` | — |

**独立性要求**：`qa-verify` 不写生产代码，也不复核自己写的文件；它对任何「声称已绿」只认自己跑出的原始输出。

## 2. 共享区与仲裁

| 共享资源 | 规则 |
| :--- | :--- |
| **IPC 三件套**（`ipc.rs` COMMANDS ↔ `lib.rs` handler ↔ `capabilities/default.json`） | 同一时间**仅一人**可改。`rust-core` 为托管人；其他域需要新增/变更 IPC 时，先报 lead，由 lead 派单或安排交接——**禁止两处同时改**（AGENTS §7 三处同步漏一处即静默失败） |
| 宪法级（`AGENTS.md` / `docs/contract.md` / `docs/CONTRIBUTING.md`） | 仅 lead，且改前知会、改后广播 + 落档 `docs/broadcasts.md` |
| `docs/broadcasts.md` | append-only，仅 lead 追加；历史条目**永不改写**（要更正就补新条目声明现行状态） |
| `Cargo.lock` / CI workflow / 前端注入脚本 | 改动前先向 lead 声明，尽快合完释放（CONTRIBUTING §3） |

**并发模型**：全体成员共享**同一个 git 工作树**。因此 **禁止一切 git 写操作**
（`commit` / `checkout` / `stash` / `reset` / `rebase` / `merge` / `push` / 切分支）——
一次切分支就会毁掉他人未提交的工作。只读 git（`status` / `diff` / `log` / `ls-files`）可用。
改动留在工作树，由 lead 审阅 `git diff` 后统一提交。
**热区文件的顺序纪律**：`ipc.rs`、`lib.rs`、`Cargo.toml`、`frontend/src/lib/tauri.ts` 等被多方需要的文件，先申请后动，动完立即报告释放。

## 2.5 产物落点（踩过的坑，2026-09-11）

`docs/known-issues/` **被 `.gitignore:20` 忽略**（该目录无任何 tracked 文件，内含本地截图与私有笔记）。
写进那里的报告**永远不会进提交——「不落盘 = 不存在」直接失效**（本轮三份报告已因此迁址到本目录）。

| 产物类型 | 落点 |
| :--- | :--- |
| 团队工作产出（巡检 / 核对 / 验证 / 方案报告） | `docs/team/`（tracked） |
| 需长期沉淀的结论 | 提升为既有专项文档的一节，或 `docs/broadcasts.md`（仅 lead 追加） |
| 采纳的架构决策 | `docs/adr/`（模板 + 索引） |
| 本地临时捕获（截图 / 原始笔记） | `docs/known-issues/`（可写，但**不得作为唯一交付**） |

> 写盘前先 `git check-ignore -v <路径>` 自检；被忽略即换落点，不要写完再说。

## 3. 任务板流转

`team_task_get`（取最新 revision）→ `claim` → 干活 → `complete`（用当前 revision）。
`blocked_by` 只表示**前置未完成前不得开工**，不会自动叫醒任何人——前置完成后由 lead 派单。
`write_scopes` 是**承诺**而非锁：越界前先申请，不申请即越界 = 团队事故（CONTRIBUTING §3）。

## 4. 验证与证据

- **写者自验**：行为改动必须带测试或验证记录（AGENTS §8.3），「没测过」不合入。
- **独立复核**：任何进入 master 的改动，由 `qa-verify` 复跑闸门并写对抗性复核（专找「声称绿实为红」「测试未覆盖新行为」「越界改了未声明文件」）。
- **证据强度三档**（汇报时必须标注）：`实测`（贴命令与原始输出）/ `推断`（说明推理链与缺口）/ `无法判定`（写明缺什么证据）。**禁止用「预计应该绿」冒充实测**。
- **闸门命令**（CI 同款口径，AGENTS §1）：

```bash
export PATH="$HOME/.cargo/bin:$PATH"          # 本机 cargo 不在默认 PATH
cd src-tauri && cargo fmt --check \
  && cargo clippy --all-targets -- -D warnings \
  && cargo test
cd frontend && pnpm run typecheck && pnpm run lint && pnpm run test
```

## 5. 必须停下问 lead 的六件事

1. 触碰 `docs/contract.md` 契约字段或 `MANIFEST_FORMAT`；
2. 新增/变更 IPC 命令（三处同步）；
3. 引入新依赖、新网络面、新持久化字段（AGENTS §4.2 / §6 / §7 例外册）；
4. 需要新 ADR 的决策（§9：影响契约 / 架构 / 安全边界）；
5. 跨出自己独占写入范围；
6. **冻结期内想做 feat**（冻结期规则见 CONTRIBUTING §8；当前状态见 §6）。

## 6. 当前阶段状态（会过期，以 `docs/broadcasts.md` 为准）

> **特别注意：`docs/broadcasts.md` 是倒序档案（最新条目在文件顶部）。**
> 读状态前先 `head -60`，不要用 `tail`——2026-09-11 本团队曾因只看尾部而误判
> 「冻结期仍在」，把一个已解除的临时约束当成了现行约束（教训登记于此）。

- **冻结期已解除**（2026-09-11T03:30Z，三平台产物验收通过——落档见 `docs/broadcasts.md`
  顶部条目）：master 恢复收 feat。唯一未验证项 = **Windows 真机人工验证**（清单见
  `docs/executor.md`），待维护者用 Windows 笔记本跑并回填。
- 发版指向（2026-09-11 校正，`docs/team/v1.1.1-验收取证-2026-09-11.md` §2.1 为判据）：
  **tag `v1.1.1` → `43f65fd2c`**（三方独立解引用一致：`rev-parse` / `ls-remote` /
  GitHub API）；首打指向 `2b4272a`，当日返工后**移动**，该旧提交只留下一轮
  `cancelled` 构建且未产出 Release。检索发版请用 `43f65fd2c`。
- 分支策略：`master` 唯一长期分支；`docs/roadmap.md` 的阶段状态以该文件头部与
  `docs/team/文档一致性巡检-2026-09-11.md` 为准。

## 7. 本机闸门 vs CI 的已知口径差距（QA 登记，2026-09-11）

> 判据 = `docs/team/gates-2026-09-11.md`。**本机全绿 ≠ CI 全绿**，以下差距是踩过的坑。

| # | 差距 | 后果与对策 |
| :--- | :--- | :--- |
| R1 | 本机 Windows 校验是**交叉** `--target x86_64-pc-windows-gnu`（MinGW ABI），CI `windows-latest` 跑**原生 MSVC** | 交叉 gnu ≠ 原生 msvc。新增 Windows 专属代码（`cfg(windows)` 分支、被打包器编译的路径）**不可只凭本机交叉绿断定 CI 会绿**——v1.1.1 首次构建红即此成因链。对策：推 master 让 CI 三平台实跑一遍再定论。**2026-09-11 补充（反向亦成立，已实证）**：**交叉目标能抓到宿主漏掉的真错误**——`#[cfg_attr(not(windows), allow(dead_code))]` 后一个空行，**宿主 clippy 绿、win-gnu clippy 红**（`empty line after outer attribute`）。机理：`cfg_attr` 的**假谓词分支**会把属性整体移除，代码在宿主上编译但**展开结果相反** ⇒ 这是 `cfg(windows)` 盲区的**隐形变体**。正确口径 = **宿主 / 交叉 / CI 原生三条链路互补，任一缺失都有盲区**；本地常规闸门**必须含 win-gnu clippy**（详见 `docs/team/msvc差异推演-2026-09-11.md`） |
| R2 | Linux 目标 clippy 在本机**不可执行**（缺 `webkit2gtk-4.1` / `javascriptcoregtk-4.1` / `gtk+-3.0` / `libsoup-3.0` / `ayatana-appindicator3-0.1`，`glib-sys` build script 失败 exit 101） | 环境缺失，非代码红。Linux 侧 lint 结论只能由 CI 给出 |

- 冷跑基线前留意磁盘：`target/` 已占 ~21 G（可用空间紧张时先 `cargo sweep`）。

## 8. 团队级教训（每条都付过代价，2026-09-11）

> 判据：**AI 下次会不会再犯？** 会——才写在这里。以下三条都不是"规范"，是三次真实事故。

| # | 教训 | 防复发动作 |
| :--- | :--- | :--- |
| M1 | **倒序档案要读顶部**：`docs/broadcasts.md` 最新条目在文件**顶部**。团队曾只看 `tail` 而误判「v1.1.1 冻结期未解除」，把一个 03:30Z 已解除的约束当成现行约束 | 涉「当前状态」一律 `head -60`；状态类判断标注取数位置 |
| M2 | **落点要满足「接收方读得到」**：lead 曾要求 `qa-verify` 交叉核对某 sha256，却把证据留在**只有 lead 能读到的会话消息**里——违反团队自己的「频道留不住，落盘才存在」。隐蔽点：不是"忘了记"，是"记在了对的人看不到的地方" | 凡是要求**他人**核对/复用的证据，落点判据 = **共享文件系统**（`docs/team/`），不是"发过消息"；接收方可能是无上下文的独立 agent |
| M3 | **测试要表达意图，不要钉住实现形态**：同一族问题本轮出现**四次**——① `toContain("tag: …")` 把死代码钉在原地（谁清理谁红）；② 变异靶子随目标删除变成 **vacuous assertion**（永远通过）；③ 断言钉住魔法字符串字面量，常量收敛后必红；④ 门禁用**子串/形态匹配**防「判定读字典」，抽常量 + 类型转换即可 **24/24 全绿逃逸** | 写完防复发断言后自问：**「这个失败条件真的能被制造吗？」** 靶子会随重构消失的、只匹配当前写法的，都不是门禁；已知未修案例见 `docs/team/前端遗留收口-2026-09-11.md` 附录 A |
| M4 | **源码扫描的截断口径**：统计「生产段」调用点时必须用 `"\n#[cfg(test)]\nmod tests"` 作为截断标记，**不能用「首个 `#[cfg(test)]`」**——`resolve.rs` 在 `:87`/`:100` 有**单条目级** `#[cfg(test)]`，错误口径会提前截断并漏掉 `:238`/`:583`。本轮 lead 与 task-29 均因此把 13 处误报为 11/12 处（`spawn` 7 + `run` 6，6 模块，lead 已于 2026-09-11 独立复算确认 13） | 任何按文件统计调用点/违规点的脚本，截断标记一律用闸门同款口径；结论给出前用「全文 grep 交叉一致」复核 |
| M5 | **章节行数统计要排除下节标题**：`awk '/^## 6\./,/^## 7\./'` 会把 `## 7.` 标题行也算进 §6 → 得 40；`awk '/^## 6\./{f=1} /^## 7\./{f=0} f'` 得真实 39。本轮 lead 报「§6=40」、rust-mgmt 复算 39，**差异纯属统计口径、非有人偷偷加行**（§11 预算判定以「真实节内行数 ≤40」为准，两种口径均不越界） | 报预算类数字时**同时给出命令**，让别人能复现同一口径；对不上先怀疑口径，别先怀疑人 |
| M6 | **`git status` 报 `M` 但 `git diff` 为空 ⇒ 是 mtime 引起的索引 stale，不是有人在写**。本轮 lead 见 `M src-tauri/src/lifecycle.rs` 一度以为有未声明写入，实测 `git hash-object` 与 `HEAD:<path>` **完全一致**、`git update-index --refresh` 后 status 即空 | 在共享工作树里判「谁在动」时，**不要只看 status**：配合 `git diff --stat`（空即无内容改动）或 `git hash-object <f>` vs `git rev-parse HEAD:<f>`。这是「三步判定」② 的必要补充，否则会被假 M 误导去追一个不存在的人 |
| M7 | **提交态验证 + 暂存前残留扫描（本轮付过真实代价）**：`8201cdf` 曾把验证探针残留（`zz_t1` + 一行真实 `crate::shell::` 依赖）提交入库 ⇒ **HEAD 的 `cargo test` 是红的**（1 failed / 328 passed，未推送故未外溢）。叠加成因：验证脚本长循环被 harness 60s SIGTERM 杀死、`trap` 兜底没跑；而 lead **用提交前跑出的旧结果背书，未在提交态上重跑** | ① **提交前必须验证的就是被提交的那份内容**——`git commit` 后立刻在**该提交态**上重跑闸门，不复用旧结果；② **暂存前扫描**：`grep -rn 'zz_\|MUTATION\|QA 注入' <改动目录>` 必须为 0；③ 验证类脚本**不用长循环**（逐案单跑或一次性多形态），备份落脚本外，不依赖脚本内 `trap`；④ **四闸门都要跑**——`fd50353` 曾只用「测试 + 宿主 clippy」验证，漏掉 win-gnu clippy 而留下真错误 |
| M8 | **「有回归用例」≠「回归用例守着真实现」**（2026-09-11 第三次重现）：T1 回归用例内**复刻了一份掩码逻辑**以便喂合成源码，于是它验的是**副本**——**回退真实现后全套 332 测试全绿、完全静默**（变异 H）。同族形态：spawn 闸门与 network_gate 各有一份词法器（`br"…"` bug 即其代价）。 | 判据 = **回退真实现时用例必须红**。用例内**禁止复刻被测逻辑**——抽成接受 `&str` 的纯函数（`production_code_lines_of`）+ 薄包装，让用例直调真实现；**算法只有一份**，重复实现必然产生"替身"。 |
| M9 | **「跑出红」≠「判据生效」**：必须核对 **红的是哪条断言**。本轮该陷阱出现两次：① qa-verify 首轮 T1 注入后 EXIT=101，实为**副判据**（截断把白名单锚点也截掉 ⇒ 正向 `crate::child_cmd` 断言报红），主判据从未报出；② 变异 G 同样 101 而副判据报红。**只看 EXIT 会把"偶然被别的断言拦住"误读为"闸门抓住了"。** | 验收判据写清「**必须报出 `:行号: <目标模式>`**」，要求贴 panic 全文；报告里区分「主判据触发」与「其它断言触发」 |

## 9. 汇报口径（对 lead）

≤15 行：状态（完成 / 部分 / 阻塞）· 产出与改动文件（精确路径）· 验证命令 + 关键原始输出 ·
证据强度 · 未决问题与建议下一步。**不复述任务描述、不写过程流水账**。
卡住即早报，不硬耗。
