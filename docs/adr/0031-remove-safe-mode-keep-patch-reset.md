# ADR-0031：删除安全模式（只保留「备份并放空 patch」兜底）

- **状态**：**已采纳**（2026-09-24 维护者裁定，**取代 ADR-0026 的机制**：一键停用与横幅记账整体删除）
- **日期**：2026-09-24
- **提出人**：维护者（亲测裁定）
- **相关方**：boot 失败面（`boot_failure.rs` / `commands/boot.rs` / `ErrorCard`）、
  控制中心（`ProfileManager` 横幅）、`plugins.rs`（patch 写入）、IPC 登记册、AGENTS §6
- **关联**：ADR-0025 / ADR-0026（旧机制，保留决策史）、ADR-0012（启动失败类型化）、
  ADR-0009（patch 写入例外与 `PatchFile`）；上游 `deepseek-ai/deepseek-harness` master
  （`apps/desktop/src/fatal-recovery.ts`、`packages/boot/app-boot`）
- **上游锚点**：本机引擎 `@deepseek-ai/dsh-app-boot@0.1.7-rc.1`（`lib/index.js`：
  `requiredStartupEntryIds` @ 3819、`prepareProfileEntries` 兼容性预检 @ 2028-2080、
  `loadProfileDirectory` bundle skip @ 925）；复现实录见行为台账复现点 24

---

## 1. 背景与问题

ADR-0026（2026-09-16）的安全模式 = 启动失败时一键把 profile 配置里全部三方行写成
`disabled: true` + 横幅记账。其立项前提是**插件问题会让 dsh 起不来**——依据是当日
真机事故：一条挂载行加载失败 → 整棵插件树拒绝加载 → 34s 后 exit 1。

2026-09-24 维护者亲测并提出：**插件不兼容不会让 dsh 起不来**，安全模式因此失去前提。
同日壳侧在克隆体 `~/.dsh-dock-dev/profiles/111` 上对**现钉版本 dsh 0.1.7-rc.1**
做了三组复现实测（未动现场，跑后还原）：

| 实验（改 profile `111` 的 `cordis.patch.yml`） | 结果 |
|:--|:--|
| 插入悬空行 `dsh-dock--test-dangling` → `@deepseek-ai/dsh-no-such-package-xyz`（包装了但不存在） | **不砖**：75s 观察窗内 web 正常就绪（`http://127.0.0.1:60908`），上游仅打 warning `1 entry did not activate … failed to import` |
| YAML 语法坏（`this is: [not, a, valid, array` + `bad yaml {{{`） | **砖**：exit 1，`failed to parse overlay … YAMLException`；`--dump-config` 同样失败（行枚举不出来） |
| 核心条目 `webserver` 配置写坏（`port: "not-a-number"`） | **砖**：exit 1，`2 required plugins did not activate / webserver (required) ValidationError` |

源码侧解释（0.1.7-rc.1）：

- **不兼容**（peer 不满足）：`loadProfileDirectory`（app-boot `lib/index.js:925`）把不兼容
  bundle 整层 skip（stderr 警告）；行级由 `prepareProfileEntries` 预检（`:2028-2080`）
  自动置 `disabled`——**两类都不砖**，且上游另有 `dsh plugin allow-version` 豁免通道。
- **悬空行**： optional 条目激活失败 → warning 后继续（`auditStartupEntries`：
  `requiredStartupEntryIds` 是固定 7 项核心集 `agent-loop` / `webserver` / `modules` /
  `connection` / `headless-runner` / `acp` / `sdk-jsonrpc-server` + bootstrap include 根，
  `:3819`）。2026-09-16 事故时的 `exit 1` 是旧版 dsh 行为，0.1.6→0.1.7 之间已变更。
- **仍砖的只有用户层自写坏**：YAML 语法坏（`parsePatchList` 抛错，任何 dump 都失败）
  与 required 核心条目配置写坏（schema 校验失败 → `StartupError`）。

另有一层背景：上游官方桌面端（Electron）2026-09-15 起有原生致命恢复对话框
（`apps/desktop/src/fatal-recovery.ts`：退出 / 重启 /「停用三方插件并备份 profile patch
后重启」），底层原语 `sanitizeProfile` 由 `dsh-app-boot` 导出（npm 0.1.6-alpha.2 起，
本机引擎已带）——但该交互**只存在于官方 desktop app**，dsh CLI 本身依旧只有
「打印 + 落盘 `$DSH_HOME/logs/startup-*.log` + exit 1」，壳启动的正是 CLI。

## 2. 约束与硬指标

1. **壳不得因删功能而失去任何 in-app 恢复出路**：仍砖的场景（YAML 坏 / 核心条目配置坏）
   必须保留至少一条「点一下能回到应用」的按钮。
2. **删除 = 彻底**：不留死代码、不留孤儿 IPC、不留指向消失机制的文案（死指针比没有更糟）。
3. **配置即状态**（ADR-0026 第二版口径仍有效）：一切启停走 profile 配置层，不造第二真相源。
4. **写入纪律不变**：任何 patch 写入仍走 `plugins.rs::PatchFile` / 覆写前备份 / 原子替换
   （ADR-0009 写入例外）。
5. **三平台 + 客体档语义不收窄**：删功能不得把 WSL 客体档原本能用的面变得更差；原本就
   显式拒绝的档位（快照 home）保持显式拒绝。

## 3. 备选方案及评估

### 方案 A：删「一键停用 + 横幅记账」，保留「备份并放空 patch」兜底 —— ✅ 最终采纳

- 思路：`safe_mode` 按钮、`get_safe_mode_state` / `dismiss_safe_mode_notice` 两条 IPC、
  横幅、记账文件（`<app_data>/safe-mode/<profile>.json`）、`safe_mode.rs` 模块整体删除；
  仅保留 `quarantine_patch`（备份 + 放空 `cordis.patch.yml`）作为错误卡动作
  `safe_mode_reset`。`PluginRowFailed` 首屏动作改为行归壳所有时的
  `quarantine_plugin_row`（只移除出错行，2026-09-16 已接线、本次提上首屏）。
- 优点：两个仍砖场景都被 `quarantine_patch` 覆盖（放空 = 去掉用户全部行 → 回退 bundle
  默认值）；删除面占原模块绝大部分（846 行 → 约 40 行迁入 `plugins.rs`）；上游 CLI 侧
  本就没有对应交互，壳这层不留重复建设。
- 代价/风险：`PluginRowFailed` 在行不归壳时首屏无按钮（只有展开详情里的放空兜底 +
  文案指路）——可接受，因为该类失败在 0.1.7-rc.1 上已基本不可达（降级 warning）。
- 对照约束：① 两条出路（quarantine / 放空）都在；②③④ 见 §4 删除清单与写入路径不变；
  ⑤ 放空兜底本就仅宿主档（客体档经 home 守卫显式拒绝，与删除前同口径）。

### 方案 B：连「备份并放空」一起全删 —— ❌ 否决

- 思路：patch 语法坏时用户只能手工出去编辑 YAML。
- 否决理由：违反约束 1。今天实测 YAML 语法坏仍 exit 1 且 dump 枚举不出行——删了它，
  这类用户在本壳内**零**恢复出路；而它正是唯一能同时覆盖两类砖面的机制。

### 方案 C：对齐上游 desktop 的 bundle 层重置（调 `sanitizeProfile` 把 bundles 恢复成出厂 web 模板）—— ❌ 否决

- 思路：壳 spawn node 调 `dsh-app-boot` 的 `sanitizeProfile`，与官方桌面端同机制。
- 否决理由：① 需新增 node 脚本调用面与 profile 锁语义移植，代价与收益倒挂（方案 A 的
  40 行已覆盖全部仍砖场景）；② 上游该交互属 desktop app 而非 CLI，壳对齐它等于把
  desktop 的产品决策搬进壳；③ bundle 重置会连用户 patch 一起放倒，比行级/整层放空更重。

## 4. 最终决策

删安全模式：`safe_mode` 一键停用按钮、两条只读 IPC、横幅与记账、`safe_mode.rs` 模块
（含其客体档孪生接线）整体移除；`quarantine_patch`（备份 + 放空 patch）迁入
`plugins.rs` 保留为错误卡动作 `safe_mode_reset`；`PluginRowFailed` 首屏动作改为行归壳
所有时的 `quarantine_plugin_row`。理由一句话：上游 0.1.7-rc.1 实测插件侧问题已不砖
启动（ADR-0026 前提消失），只剩用户层自写坏两类砖面，一个 40 行的放空兜底即可全覆盖。

**删除清单**（与本 ADR 同 PR）：

- Rust：`safe_mode.rs` 整文件；`commands/boot.rs` 的 `safe_mode` 分支与两个命令；
  `boot_failure.rs` 动作表/文案/`with_actions`；`executor.rs` 横幅发射；
  `guest.rs` 随动死代码（`dsh_home_abs` / `parse_dsh_home_abs` 及其测试）；
  `plugins.rs` 的 `RowAttribution` / `row_attributions_blocking` / `disabled_row_ids` 一族；
  `ipc.rs::COMMANDS` 两条 + `lib.rs` handler + `capabilities/default.json` 两权限（三处同步）。
- 前端：`ErrorCard` 的 `safe_mode` 分派与首屏判定；`ProfileManager` 横幅（状态/取数/渲染/
  `X` 图标）；`SafeModeState` 类型、两个 api 方法、双字典横幅与 `safe_mode` 动作文案、
  `ipc-shapes.json` 字段表；`safeModeBanner.test.ts` 删除，`errorCardActions` /
  `bootFailure` / `ipcShapes` 三测试更新。
- 文档：AGENTS §6 登记回收；IPC 登记册两条命令与计数重校；行为台账新增复现点 24；
  本 ADR；ADR-0026 状态改「已被 ADR-0031 取代」；broadcasts 落档。

## 5. 后果与后续行动项

### 正面后果

- 壳少一个 846 行模块、两条 IPC、一整套横幅/记账/世界接线——「插件崩了怎么办」的答案
  收敛为两个按钮（移除出错行 / 备份放空），都写在用户最终会读的那份配置上。
- 与上游行为对齐：壳不再维护一份上游已经改掉的失败假设（悬空行 = warning）。

### 负面后果 / 新增债务

- 存量用户机器上 `<app_data>/safe-mode/*.json` 成为孤儿文件（无人读、无人写）；本 PR
  **不主动删**（壳不清用户数据目录），自然积累。
- `PluginRowFailed` 在行不归壳所有时首屏为空——依赖文案指路；若未来 required 集扩大
  导致该类失败重新高发，需重开「一键停用」评估（见复审条件）。
- Windows 交叉 clippy 本次未跑（本机缺 MSVC/clang 与 mingw32-gcc，`ring` 无法交叉编译）；
  windows 专属代码增量为注释 + 一对函数删除，CI 三平台闸门兜底。

### 行动项

- [x] 代码删除与测试更新（Rust 525 passed / 前端 673 passed / 宿主 clippy -D warnings 绿）
- [x] AGENTS.md §6 登记回收、IPC 登记册同步、ADR-0026 状态、台账复现点 24、broadcasts
- [ ] CI 三平台 clippy（含 Windows 目标）复核通过
- [ ] 实机清单（`docs/executor.md`）：宿主 + WSL 客体各走一次「备份并放空」错误卡动作

## 6. 复审条件

- 上游 dsh 若把 `requiredStartupEntryIds` 扩大到三方插件、或恢复「行加载失败即整树
  拒绝加载」语义 → 重开「一键停用」评估；
- 上游 CLI 自身出现交互式恢复（prompt / TUI 选择）→ 壳应改为转发/引导而非自建；
- 用户反馈「patch 写坏后找不到放空入口」→ 把 `safe_mode_reset` 从展开详情提回首屏；
- dsh 升级复核触发（台账复现点 22 口径）：上游提供官方安全模式开关的语义变化。
