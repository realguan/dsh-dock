# ADR-0016：管理面下沉 WSL 客体（控制中心跨环境一致）

- **日期**：2026-09-11
- **状态**：草案（待维护者评审）
- **提出人**：guan（AI 协作起草）
- **相关方**：`plugins` / `profiles` / `sessions` / `credentials` / `dsh_settings` / `diagnostics` / `mcp` / `executor` / `lifecycle` / `updates`（§7 网络面）/ 前端控制中心
- **关联**：ADR-0004（客体安装、Windows 壳不触网）· ADR-0006（网络面与镜像链）· ADR-0009（profile 生命周期与文件不变量）· ADR-0010（引擎倒置、客体同构）· ADR-0011（插件职责边界）· ADR-0015（子进程生命周期 seam）

---

## 1. 背景与问题

实机（Windows 宿主 + dsh 运行于 WSL 客体，2026-09-11）**控制中心安装市场插件全部失败**，队列逐条报
「引擎未就绪（node 已就绪、dsh 缺失）——请先启动应用完成引擎引导后重试」，点重试无效。

这不是"引导没跑完"，而是**结构性缺口**。证据链：

1. 文案出处 `engines.rs:61-70`：`resolve_toolchain(data_dir)` 只看 `<data_dir>/engines/bin` 下的 `node` / `dsh`。
2. 三个调用点**全在宿主侧、零模式分支**（`plugins.rs`/`profiles.rs` 内 `Mode::`、`wsl` 命中数为 0）：
   - `plugins.rs:533` 插件安装/卸载/更新；
   - `plugins.rs:889` 插件行（`dsh --dump-config`）；
   - `profiles.rs:638` 创建 profile。
3. **WSL 模式的引擎只落客体**：`executor.rs:872-878`（`WslExecutor::probe` → `ensure_guest_engine`）、
   `executor.rs:750-756`（把捆绑 pnpm 投递进客体）、`executor.rs:535-547`（客体脚本
   `DSH_ENGINES="$HOME/.dsh-dock/engines"`）。宿主 `<data_dir>/engines` 在 WSL 模式**全程不受管**——
   这正是 ADR-0004「Windows 侧壳不触网」+ ADR-0010 客体同构的**预期结果**。
4. 于是宿主引擎恒为「未就绪」；本例恰是 node 有、dsh 无的半就绪态。该半就绪是仓库已记录的
   Windows 失败形态（`engines.rs:802-806`：无符号链接特权时 `runtime set node` 报 os error 5，
   "进而 dsh 永远装不上"），多半来自更早某次**本机模式**（首启默认档）的引导。
5. **更深一层：管理面读写的 home 也是宿主的**。`resolve.rs:85-97` `user_dsh_home()` = 宿主
   `$DSH_HOME` / `~/.dsh`；`commands/profile.rs:19-21` `list_profiles` 扫的就是它。而客体 dsh 用自己的
   WSL `~/.dsh`（`executor.rs` 从不导出 `DSH_HOME`）。

**结论**：WSL 模式下，控制中心操作的**始终是宿主世界**，而运行中的 dsh 在**客体世界**。插件装卸与
profile 创建必需 dsh CLI，宿主引擎按设计在 WSL 模式永不就绪 → 结构性不可用；且错误文案所指的补救
（"再启动一次应用完成引导"）与 ADR-0004 的设计**直接矛盾**，重试是死路。

**影响面**（管理面全部按宿主 home 实现，需逐一核对归位）：profile 生命周期、插件中心、会话维护、
系统控制台（凭据 / dsh 设置 / MCP / 诊断）。市场 registry 拉取与模式无关（`updates.rs`）。

## 2. 约束与硬指标

1. **ADR-0004「Windows 侧壳不触网」**：凡涉及网络的动作必须发生在**客体进程内**。
2. **AGENTS §7 网络面登记制**：插件装卸的网络在客体 `dsh`/`pnpm` 子进程内发生，属"dsh CLI 工具链"用途，
   须在本 ADR 中登记并同步 §7 台账；壳自身不得新增网络客户端。
3. **AGENTS §6 + ADR-0015**：新增子进程一律经 `lifecycle::{spawn,run}`（`Role::DshCli` 已存在，
   `lifecycle.rs:41-42`）；裸 `Command::spawn()` / `output()` 有机器闸门拦。
4. **AGENTS §6 文件系统不变量**：`.credentials.yaml` 保持 0600、顶层仅三键、原子写；会话目录只读不删；
   `profiles/node_modules` 符号链接农场不得直写；三件套不得生成/复刻内容。**客体侧的写入必须维持同一套
   不变量**——这是本方案最大的正确性风险点。
5. **跨平台语义显式**：客体路径一律 `#[cfg(windows)]`；非 Windows 平台行为不得因此改变（测试须覆盖）。
6. **世界一致性（本 ADR 的立项根因）**：控制中心呈现与改写的对象，必须与**当前会话实际运行的
   世界**一致；不一致的"看起来在管、其实管错"比功能缺失更糟。
7. **红线 1**：不修改 dsh 源码；只能调 CLI 与文件系统。

## 3. 备选方案及评估

### 方案 A：客体原语（读 / 写 / CLI）+ 管理面按活动模式择源 —— ✅ 最终采纳

- **思路**：新增一组客体子过程原语，一次 `wsl.exe -e bash -lic` 调用内完成：
  - `guest::read_files(distro, paths)` → 「客体路径 → 文件原文」映射。**关键收益**：现有
    `profiles.rs` / `plugins.rs` / `sessions.rs` 的解析与派生逻辑（纯函数）**原样复用**，不必重写；
  - `guest::run_dsh_cli(distro, args, home)` → 复用 `guest_prep!`（`executor.rs:544`）与 `sh_quote`
    （`executor.rs:562`）拼装，经 `lifecycle::run(Role::DshCli)` 收口，返回形状对齐现有 `ForwardRun`，
    使 `classify_*` 分类逻辑不改；
  - `guest::write_file / remove / rename`（客体侧「临时文件 + `mv`」实现原子性）——仅 P2 需要。
  管理面入口加一层**「home 源」seam**：`Local` → 现状（宿主 fs + 宿主引擎）；`Wsl` → 客体原语。
  **客体发行版由会话启动时实际选中的那个落进 `ShellState`**（与 `active_mode` 对称），避免管理面
  另选一个发行版而与运行中的会话脱节（约束 6）。
- **优点**：与已跑通的客体链路同源（投递 pnpm、哨兵文件读取都走 `wsl.exe -e` + marker 判据，
  `executor.rs:510` / `shell.rs:324-339` 的 UTF-16 与缓冲经验直接继承）；解析层零改动；不依赖
  9p / UNC 读语义；不触 dsh 源码。
- **代价/风险**：每次操作一个 `wsl.exe` 进程（冷启动百 ms 级）；客体写脚本 = 第二处文件不变量实现，
  存在双源漂移风险（须与宿主实现同测同审）；实机验证只能在 Windows+WSL 做，CI 覆盖不到。
- **对照约束**：①网络全在客体进程内 ✔；②仅登记、壳不新增网络客户端 ✔；③经 lifecycle seam ✔；
  ④客体写脚本逐条复述不变量并在评审中逐条对账 ✔；⑤新增代码全部 `#[cfg(windows)]`，非 Windows 走原路径 ✔；
  ⑥择源由 `active_mode` + 实际选中发行版决定 ✔；⑦只调 CLI 与文件系统 ✔。

### 方案 B：宿主直连 `\\wsl$\<distro>\...` UNC 路径 —— ❌ 否决

- **思路**：把管理面 home 指到 UNC 路径，直接沿用现有宿主代码。
- **否决理由**：违反约束 4——0600 权限、原子写、跨进程锁在 UNC 语义下不可靠；且 UNC 可用性依赖
  9p 与访问上下文（服务/权限）。仓库现有 `wsl_unc_path`（`executor.rs:1061`）只用于**投递**一次性产物，
  不足以承载带不变量的读写。保留为"仅只读兜底"的备选，不采纳。

### 方案 C：壳在宿主也引导一份引擎，让现有宿主实现直接可用 —— ❌ 否决

- **思路**：WSL 模式下仍引导宿主 `engines`，于是现有代码不改即可通过 `resolve_toolchain`。
- **否决理由**：违反约束 1（WSL 模式壳不触网）与 ADR-0010 客体同构；**更致命的是违反约束 6**——
  即便装成，操作的也是宿主 `~/.dsh`，等于把用户的 WSL 世界管错。此法能把报错消掉，却让缺陷更隐蔽。

### 方案 D：只下沉 dsh CLI 能做的，文件级操作显式标"暂不支持" —— ✅ 作为 A 的 P1 收窄采纳

- **思路**：P1 只下沉 `plugins.rs` 的三条 CLI 路径（装卸 / 行 / 清单）；profile 复制、重命名、删除、
  会话自愈、凭据与设置编辑等**文件级写**操作，在 WSL 模式返回"暂不支持（原因 + 替代路径）"。
- **采纳理由**：AGENTS §8.2 禁止一次性大改。先把用户报障的那条链打通并**把话说诚实**，其余按批次推进。
  它不是与 A 并列的方案，而是 A 的落地顺序。

### 方案 E：走 dsh Web App 的 loopback API —— ❌ 否决

- **思路**：复用已登记的只读回环通道（`plugins.rs` 的 `POST http://127.0.0.1:<port>/api/pluginInventory/list`，
  §7 复现点 11）做管理。
- **否决理由**：dsh 未公开管理**写**接口（已登记的只有只读 inventory）；依赖未文档化接口违反稳健性；
  仅在会话运行中可用（无会话即不可用）；补接口需改 dsh 源码 = 红线 1。**若上游将来提供管理 API，
  本条应复活重评**（见 §6）。

## 4. 最终决策

**采纳方案 A，按其 P1→P3 分期落地**：

- **P1（修用户报障的链）**：客体原语（`read_files` / `run_dsh_cli`）+ 管理面「home 源」seam +
  插件中心三处调用点在 WSL 模式改走客体；错误面文案改为诚实可行动。
  **落地期范围修正（2026-09-11，实现时发现）**：P1 还必须含一个**客体侧单键写入**——
  `build_policy::ensure_profile_build_policy_best_effort` 目前只在宿主侧写 profile 的
  `pnpm-workspace.yaml`（ADR-0013 的 `dangerouslyAllowAllBuilds: true`）。WSL 模式下客体 profile
  由客体 dsh 自行物化，这个键**从未写过**；于是客体内的 `pnpm add` 会撞 pnpm 12 的构建审批门
  （`ERR_PNPM_IGNORED_BUILDS`），插件带构建脚本时必失败。故「客体侧单键写入」从 P2 提前到 P1，
  且必须复用 ADR-0013 的单键受控口径（不整体覆盖、不生成三件套内容）。
- **P2**：profile 生命周期——创建走客体 CLI（沿用 ADR-0009 的 `dsh plugin install` 转发链），
  复制/重命名/删除走客体写脚本（同套不变量）。
- **P3**：只读控制台——会话 / 凭据 / dsh 设置 / 诊断经客体读原语。

**分期未落地前，WSL 模式下对应动作必须显式、诚实地报"暂不支持（原因 + 替代路径）"**；
严禁再出现"请先启动应用完成引擎引导后重试"这类与 ADR-0004 相矛盾的死路提示与重试按钮
（这一条**不依赖任何分期**，应作为 P0 先落地）。

客体发行版选择落 `ShellState`（与 `active_mode` 对称），保证管理面与会话同源。

## 5. 后果与后续行动项

### 正面后果
- 控制中心在 WSL 模式下管的是**真正的世界**，与主工作台会话同源；
- 「宿主机 fs / 客体原语」成为显式 seam，为将来可能出现的新环境档（SSH、容器）留出落点；
- 现有全部解析/派生纯逻辑因"读原文"设计而零改动复用，减少双实现漂移面。

### 负面后果 / 新增债务
- 管理面每次操作多一次 `wsl.exe` 往返，冷启动延迟上升（可接受，但重活应合并为单次脚本）；
- **客体写脚本是第二套文件不变量的实现处**，与宿主实现构成双源——必须在评审与测试中逐条对账，
  否则会漂移出"宿主守住了 0600、客体没守住"这类安全缺口；
- 实机验证不可 CI 化，只能靠 `docs/executor.md` 的 Windows+WSL 验证清单。

### 行动项

- [ ] **P0**（不等待其余分期）：WSL 模式下管理动作的诚实错误面——文案 + 去掉死路重试；含单测。
      **落地（2026-09-11，第二批 e）**：`mgmt::require_local` 统一守卫；禁语回归闸门见 `mgmt.rs` 单测。
- [ ] 本 ADR 评审通过（维护者）——P1 开工前置。
- [ ] **P1**：`guest` 原语（读 / CLI）+ 「home 源」seam + `plugins.rs` 三处调用点择源；
      测试：原语拼装与解析为纯函数（宿主可测）、模式择源单测；`#[cfg(windows)]` 分叉；
      WSL 实机清单条目补 `docs/executor.md`。
      **落地进度（2026-09-11）**：
      ① **第一批（原语地基，commit `655e216`，PR CI 三平台绿）**：`src-tauri/src/guest.rs`
      （`read_files` 一次 `wsl.exe` 读多份文件、base64 单行帧；`run_script_to_log` 经
      `lifecycle::Role::DshCli`；`dsh_cli_script`；`base64_decode`；帧解析。`guest_prep!`/`sh_quote`
      自 `executor.rs` 迁入成唯一源）；`executor.rs` 的 `wsl_command`/`run_wsl_capture` 提
      `pub(crate)`、`Executor::target_distro()` 默认方法 + `WslExecutor` 实现；`profiles.rs` 的
      `run_dsh_cli_in_guest`（与本地 `run_dsh_forward` 契约逐项对齐、不注入 `DSH_HOME`）+ 非
      Windows 孪生；`boot.rs`/`lib.rs` 的 `ShellState.active_wsl_distro`（probe 成功后记录实际选中值）。
      ② **第二批（a–e 接线，2026-09-11）**：
      a) `src-tauri/src/mgmt.rs`：世界择源（`ui::current_active_mode` + `active_wsl_distro` →
      `Local`/`Wsl{distro}`，**绝不回落 Local**：模式为 WSL 而无发行版即报错）。纯内核
      `world_from(mode, distro, windows)` + 单测；`ui::current_active_mode` 去掉 macOS `cfg`
      （管理面全平台消费）。
      b) `commands/plugin.rs`：`install_plugin`/`remove_plugin`/`update_plugin`/`get_plugin_rows`/
      `list_profile_plugins` 在命令入口解析世界并下传（`List` 类命令新增 `app` 注入参数，前端无感）。
      c) `plugins.rs` 三处择源：`mutate_plugin_blocking`、`plugin_rows_blocking`、
      `list_profile_plugins`（客体档 = `list_profile_plugins_in_guest`：先读清单拿依赖名，再一次
      批量读各 `node_modules/<pkg>/package.json`）。解析/装配全部抽成**两侧共用的纯函数**
      （`assemble_plugin_entries`／`installed_info`／`dependency_names`／`patch_entry_map_text`／
      `classify_op_outcome`），**零第二实现**。客体读路径全部相对**客体 dsh home**
      （`guest::HOME_EXPR` = `${DSH_HOME:-$HOME/.dsh}`，与客体 dsh 自身解析同源）。
      d) **客体侧单键写入**（§4 范围修正）：`build_policy::ensure_profile_build_policy_in_guest(_best_effort)`
      复用同一纯函数 `ensure_allow_all_builds`；客体写原语 `guest::write_home_files`（base64 载荷内嵌
      → 客体侧 `base64 -d` 落同目录临时文件 → `mv` 原子替换；父目录不存在即失败，**不代 dsh 生成
      profile 目录**；单次写上限 8 KiB 超限拒绝而非截断）。
      e) **P0 诚实兜底**：`mgmt::require_local` 覆盖 profile 列表/详情/CRUD/默认档、会话四命令、
      控制台（凭据 · DSH 设置 · MCP · 系统诊断）、`set_plugin_disabled`、`copy_plugin_config`、
      `list_all_plugins`、`check_plugin_updates`；文案 = 原因 + 替代路径（指向客体终端里的 dsh 或
      切回本地模式），禁语（"请先启动应用完成引擎引导后重试"）有单测闸门。第一批的
      `#[expect(dead_code)]` 自清理闸门已按要求删除（接线后不再触发）。
      **接线后仍未下沉（属 P2/P3；WSL 世界经 P0 守卫诚实拒绝）**：profile 列表/详情/CRUD、
      会话维护、控制台只读面板、插件开关与配置复制、更新检查、聚合总览。
      **已知边界（登记待办）**：`switch_profile` 的 webUi 候选校验仍读宿主 home——控制中心在
      WSL 模式已无 profile 列表（P0 守卫），该路径在 WSL 世界不可达；profile 列表下沉时一并改按世界择源。
- [ ] **P2**：profile 生命周期下沉（逐条对齐 ADR-0009 的文件不变量与写入例外册）。
- [ ] **P3**：只读控制台下沉。
- [ ] 文档同步：AGENTS §7 登记本次客体管理面用途；`docs/contracts/` 增子契约（客体管理面原语与不变量）；
      `docs/broadcasts.md` 落档评审结论。
- [ ] 验证清单（Windows+WSL 实机）：插件装/卸/更 + 插件行 + profile 创建；以及错误面——
      `wsl.exe` 不可用、无发行版、发行版为非 glibc（musl）、断网、客体 home 不存在。

## 6. 复审条件

- 出现第三种环境档（SSH / 容器 / 远程）→「home 源」seam 需重新设计，本 ADR 必须重开；
- **dsh 上游提供管理 API（尤其插件装卸）** → 方案 E 复活重评（可能大幅简化客体原语）；
- ADR-0015 的 `Role` 集合变化（如 `DshCli` 按宿主/客体拆分）→ 本 ADR 的 spawn 口径复核；
- 客体写脚本与宿主实现出现第一次不变量漂移（无论是否造成事故）→ 立即收敛为单一实现或加机器闸门。
