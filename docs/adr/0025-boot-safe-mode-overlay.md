# ADR-0025：启动失败的安全模式（临时 overlay 禁用用户层行）

- **状态**：**已采纳**（2026-09-16 维护者裁定；**口径已两次修订，以正文为准**：
  ① 停用范围初取 A+（用户层行 ＋ 第三方 bundle 层行）→ **实测推翻**，回退"**只停用户层行**"
  （§4 有实测记录：整层摘掉 ⇒ `exit 1`）；② 首屏动作由三个收敛为**一个**（§7）；
  「patch 语法坏」的兜底分支**本轮一并做**，但必须二次确认 + 备份）
- **日期**：2026-09-16
- **相关**：ADR-0012（启动失败类型化）、ADR-0020（实验能力开关）、ADR-0015（生命线与孤儿清扫）
- **上游锚点**：`/Users/guan/git/deepseek-harness` @ `0d1f50007f9bca3f52b06e1c3074fa14d5fb0720`（2026-09-15 只读调研）

## 1. 背景

一条坏的插件挂载行会让**整棵 plugin tree 拒绝加载**，dsh 在 34–44 秒后退出码 1，
用户**进不去应用界面**，只能手工编辑 `cordis.patch.yml` 才能恢复（2026-09-16 真机两次：
`failed to apply loader entry`（MCP 连接/配置缺字段）与 `failed to import loader entry`
（包被卸载，`ERR_MODULE_NOT_FOUND`））。

现有的「点名出错行 + 移除该行并重启」只在**报错确实点名了某一行**时可用；若一次失败涉及
多行、YAML 语法坏、或用户就是想"先能用起来再慢慢查"，需要一条**不依赖定位到具体行**的退路。

维护者要求：**参考官方 app，提供"安全模式启动"（关闭所有插件再启动）**。

## 2. 上游调研结论（含锚点）

| # | 结论 | 锚点 |
|:--|:--|:--|
| 1 | **CLI 没有 safe/no-plugins 类开关**；`dsh --help` 里的 `rescue` 只是示例 profile 名（`--from-default-profile web` 的示范），非硬编码 | `apps/cli/src/args.ts:78-79,145-149`、` :187` |
| 2 | **官方桌面 app 有等价能力**：启动失败页提供「Disable all third-party plugins and retry」 | `apps/desktop/src/startup-document.ts:12-26`、`locale.ts:8-13` |
| 3 | 官方那招的机制是**把 `dsh.profile.bundles` 重置为 `[dsh-base, dsh-web-app]`**（保留文件）——**只摘 bundle 层插件** | `apps/desktop/src/project-manager.ts:83,335-344` |
| 4 | **官方机制治不了本仓库这次踩的病**：坏行在 `cordis.patch.yml` 的 `- insert:` 里，不在 `bundles` 里，重置 bundles **清不掉它** | 同上 + 本次事故现场 |
| 5 | `--patch <path>` 是**最高优先级** overlay：层栈 `bundle → profile 层 → home 层 → --patch`，且各层在挂载前**拍平成一个 patch 列表** → 可跨层按 `id` 定位 | `apps/cli/src/profile-boot.ts:212-219,246-249`、`packages/boot/app-boot/src/index.ts:393-395` |
| 6 | overlay 语义：`- id: x` + 任意键（含 `disabled: true`）覆盖该行；**id 匹配不到只 warning 不报错** → "能禁就禁"是安全的 | `vendor/include/src/index.ts:57-127`（尤其 `:110-112,120-123`） |
| 7 | **home 层是第二个用户层**：`$DSH_HOME/cordis.patch.yml` 对每个 profile 生效且**优先级高于** profile 层 → 救援 profile 方案躲不开它 | `apps/cli/src/profile-boot.ts:77-79,243` |
| 8 | `--dump-*` 只打印**永不 boot**，且 `--dump-default-config` 与 `--patch` **互斥** | `apps/cli/src/args.ts:100-105,113-115`、`bin.ts:48-57` |
| 9 | 失败时上游**无重试/降级**：`boot()` 只有一个 catch，dispose 后原样抛出 | `packages/boot/app-boot/src/index.ts:867-916`（`:888,903,915`） |
| 10 | 上游有"可选条目"概念但**只在启动审计层**（required 名单 7 个 id），**没有行级 `optional`/`if`** | `packages/boot/app-boot/src/index.ts:711-719,818-835`、`vendor/loader/src/config/entry.ts:9-22` |
| 11 | **源码与构建产物分歧**：`vendor/loader/src`（@0d1f500）对坏行是**宽容**的（import 失败 → 记日志、条目 inactive、审计降级为 warning）；`vendor/loader/lib`（陈旧构建）是**严格**的（`failed to … loader entry` 抛出 + 整组回滚），且与**已安装运行时逐字节相同** | `vendor/loader/src/config/entry.ts:175-189`、`group.ts:56-64` vs `vendor/loader/lib/index.js:91-125,307-309,516-529`；`diff` 0 行 |

## 3. 我方实测（2026-09-16，克隆 dev home，未动现场）

| 实验 | 结果 |
|:--|:--|
| `--dump-default-config`（bundle 层）id 数 | **161**（集 B） |
| `--dump-config`（合成）id 数 | **166**（集 C） |
| **差分 D = C \ B** | **恰为用户层 5 行**（含那条坏行）→ 枚举规则成立 |
| 坏 profile + `--patch` overlay（D 全部 `disabled: true`） | **正常就绪**（`dsh web: http://…53045/?token=…`）✅ |
| 坏 profile 原样启动（对照组） | 退出码 1、5671 字节错误栈 ✅（复现事故） |
| patch **语法坏**时 `--dump-config` | **exit 1**（`failed to parse … cordis.patch.yml`）→ 无法枚举，需分层兜底 |

## 4. 决策

> **实现方式与 §3 实验方法的区别**（2026-09-16 独立复核 #5 指出文档/代码漂移）：§3 的
> **两趟 dump 差分**（`--dump-config` − `--dump-default-config`）只是当时的**验证判据**；
> **落地的代码不调 `--dump-default-config`**（全仓无该串）。`commands/boot.rs` 走的是
> `plugins::row_attributions_blocking`——按 dump 的**段落归属**判定用户层
> （bundle 层标包名、用户 patch 层标文件绝对路径，`safe_mode::should_disable`），
> 同一 profile 上得到的正是同样的那 5 行。差异记此，勿按 §3 去核对实现。

**方案 A（采纳）：安全模式 = 临时 `--patch` overlay，禁用**用户层**全部行。**

> **裁定记录（含当日回退）**：维护者先裁定 A+（连第三方 bundle 层行一起停，只留
> `dsh-base` / `dsh-web-app`）。**实测推翻**：真机 profile 上停 33 行 → dsh **exit 1**，
> stderr `6 entries did not activate` + `pending (waiting for service: tools)` ——
> 第三方层的行不是孤立插件，与被保留层有服务依赖，整层摘掉会让依赖悬空；
> 同一 profile **只停用户层 5 行 → 正常就绪**。故按证据回退为 **方案 A（只停用户层行）**，
> 并把"A+ 需要更聪明的规则（按服务依赖求闭包）"记为后续可选项。

1. **枚举**（两次只读子进程，均不占端口、不 boot）：
   `--dump-default-config` → B；`--dump-config` → C；**D = C \ B**。
   D 天然覆盖 profile 层 + home 层（含用户手写行与壳写的挂载行），**无需区分坏行在哪一层**。
2. **overlay 落点 = dsh-dock 自己的数据目录**（`<app_data>/safe-mode/<profile>.yml`），
   **不写进 profile 目录、不改任何 dsh 文件**；每行 `- id: <d>` + `disabled: true`。
3. **启动**：在现有 spawn 路径上追加 `--patch <overlay>`（启动器 flag 必须在 app 参数边界之前）。
4. **退出安全模式 = 不带 `--patch` 重启**（原子回退：删文件或不传参，用户 profile 全程零改动）。
5. **UI**：错误卡新增动作「安全模式启动」；进入后顶部常驻横幅「安全模式：已临时停用 N 个插件行」
   + 「退出安全模式并重启」；横幅里点名被停用的行，引导回「实验能力」逐条处理。
6. **分层兜底**（实测驱动）：`--dump-config` 非 0（YAML 语法坏）→ 无法枚举 → 提示走
   **方案 B：把 `cordis.patch.yml` 改名备份后放空**（`fs_backup.rs` 既有 `.bak-<unix秒>` 惯例；
   这是唯一会碰用户文件的分支，须显式确认 + 明确告知"文件已备份，随时可还原"）。
7. **WSL 客体档**：本轮显式不支持（与实验能力目录/写行同口径：客体侧需补原语，宁可报错不回落宿主）。

**不采纳**：
- **重置 `dsh.profile.bundles`（官方 app 的做法）**：治不了 `cordis.patch.yml` 的 insert 行（结论 4）；
  且会写用户的 `package.json`。
- **救援 profile（`--from-default-profile`）**：home 层照样生效（结论 7），救不了 home 层坏行的情况；
  且用户看到的是另一个应用，不是自己的。
- **行级容错**：上游不提供（结论 10），无法在不改 dsh 源码的前提下获得。

## 5. 后果

- **正面**：任何"用户层行导致起不来"的故障都有**一键、零文件改动、可原子回退**的出路；
  它同时是"多点坏行/语法坏"这类**定位不到单行**场景的唯一出口；与既有「移除该行并重启」互补
  （后者治单点、前者治多点与未知）。
- **负面 / 新增债务**：
  - 新增 IPC 命令与一套启动分支（须按 AGENTS §7 登记 + 四处同步闸门）；
  - 安全模式下的工作台"少了一批插件"，**行为与正常启动不同**——UI 必须持续可见地说明，
    否则用户会把"插件没生效"当成 bug；
  - 枚举依赖 `--dump-*` 的**文本格式**（现按行解析 `- id:`）：上游换格式会静默少禁（症状是
    安全模式仍起不来）→ 需以"安全模式启动后仍失败"为信号保留回退到方案 B。
- **未解的更优解（选项 E，留档不实施）**：结论 11 表明**由 `0d1f500` 重新构建的 dsh** 可能把
  非 required 坏行降级为 warning——若成立，"升级引擎"本身就消除致命性，安全模式退化为兜底。
  需单独做一次"构建 + 单坏行实验"确认（**未做**，不写进任何承诺）。

## 6. 行动项（**已全部落地**，2026-09-16）

> 落地过程踩到并修掉三处硬错（都在广播里留了档）：
> ① 错误卡动作拿不到 profile（启动失败先 teardown 会话 → 会话槽恒空；改为 spawn 时记账
>    `ShellState::last_boot_profile`）；② A+ 口径实测不成立（第三方层行与被保留层有服务依赖，
>    整层摘掉即悬空）→ 回退"只停用户层行"；③ `--patch` 参数位置写错（必须在 app 参数之前，
>    否则由 web app 报 `unknown option '--patch'`）。

### 原行动项（已完成）

1. `safe_mode` 模块：枚举（两次 dump）＋差分＋overlay 读写＋失败回退到方案 B；
2. `LaunchSpec`/`spawn_dsh` 支持 `--patch`（版本适配同 `no_open` 先例：旧版 dsh 不认则不得传）；
3. 新 IPC（安全模式启动 / 退出）＋四处同步＋登记册；
4. 错误卡动作 + 安全模式横幅 + i18n（zh/en）；
5. 复现台账新行（结论 5–8、11 的上游行为锚点）；ADR 索引补一行；
6. 测试：overlay 生成/差分/幂等/清理、spawn 参数注入（**含顺序契约**）、坏 YAML 回退分支、
   状态快照、安全模式可见性门禁；真机项进 `docs/executor.md`。

### 可见性补完（同日第二次裁定后落实）

- **控制中心横幅**（所有 Tab 可见）：说明"本轮已临时停用 N 行、**配置文件未改动**"，
  并给「退出安全模式并重启」入口（走 `terminal_action("safe_mode_exit")`）。
- **实验能力面板**顶部说明：下表的「已启用」指**配置层**，本轮实际未生效。
- 新增只读 IPC `get_safe_mode_state`（四处同步 + 登记册 63 → 64）。

> 为什么必须有横幅：安全模式**刻意不写配置文件**（零改动、原子回退），于是
> "配置层说启用"与"运行态说停用"**必然并存**。不解释这一句，用户看到的就是
> 自相矛盾的界面（维护者 2026-09-16 实测报告）。

## 7. 首屏单按钮（同日第三次裁定：维护者反馈"做复杂了"）

**维护者原话**：「我感觉做复杂了，遇到是插件相关的报错，提供一个按钮即可，点击按钮直接把
插件开关都暂时关闭，然后启动，这样不好吗？」

**采纳**（2026-09-16 落地）：

- 插件行失败的首屏**恰好一个**动作 = [`safe_mode`]（文案「**停用全部插件并启动**」，
  影响一行说明"临时关掉全部插件开关再启动；不改任何文件，进应用后可一键恢复"）；
- 另两条出路**不删而是下沉**：新载荷字段 `advancedActions`（`boot_failure.rs::
  advanced_actions`）收进错误卡"展开详情 → 其它出路"——
  「只移除出错的那一行并重启」（`quarantine_plugin_row`，仅壳自有行）与
  「备份并放空插件配置后启动」（`safe_mode_reset`，**YAML 写坏时唯一出路**）。
  下沉理由：删除 = 让用户在"patch 语法坏"时无路可走；平铺 = 用户要先读懂三条机制的
  差别才能点（这正是本次反馈）。
- `safe_mode` 增加**诚实门**：若枚举出的可停行数为 0（坏行来自随包 / 第三方**插件包**，
  不在用户 patch 层），**不写 overlay、不启动**，而是如实报错并指向"其它出路"
  —— 起了也是同一张错误卡，只会把"点过按钮却回到原点"变成新的困惑。

**边界（写清给后续维护者，勿当成 bug）**：「关闭**所有**插件」在本机制下 = 关闭**用户层
插件行**（即「实验能力」开关写进 `cordis.patch.yml` 的那些行）。第三方**插件包**（`dsh.bundle.patch`
形态，如 Agent Teams 三层）不在其中——§4 已实测：整层摘掉会让服务依赖悬空（`exit 1`）。
这类坏行的出路是"卸掉那个插件包"，本轮**未**实现；触发条件 = 用户报「安全模式也起不来」。

两个由此衍生的已知粗糙面：

1. 诚实门只在"可停行数 = 0"时触发。若 profile 同时有用户层行（本机就是：5 条），
   overlay 仍会写入并启动，而坏行的 bundle 行仍在 ⇒ **启动仍失败**。这是 overlay 机制的
   天花板，不是缺陷；要根治得做"把该包从 `dsh.profile.bundles` 摘掉"（官方 app 的等价动作，
   `project-manager.ts:335-344`），属独立意图。
2. 门触发时的提示走的是既有兜底分类（`Unknown`）⇒ 卡片会带一个「重试」按钮。它不会白点
   （重试只是回到上一张失败卡；新文案已写明手工出路：控制中心 → 插件页卸载该插件，或改该
   profile 的 `package.json` 的 `dsh.profile.bundles`），但与"不做必然失败的按钮"的口径
   不完全一致。**为什么没顺手改**：`terminal_action` 里枚举行是一次阻塞的 dsh 调用，拿到结果时
   命令已返回，只能经 `boot:error` 反馈；要做成"就地红字"就得给这类失败立专用分类
   （新 kind + 前端两字典 + 契约），属独立意图，不夹带。

### 独立复核（同日，第三方子 agent 反方核查）抓到并已修的三个缺陷

1. **「其它出路」整块永不渲染（阻断级）**：`frontend/src/lib/eventPayloads.ts::normalizeError`
   是按字段手写的白名单入口，新字段 `advancedActions` 被漏掉 ⇒ `ErrorCard` 的
   `payload.advancedActions ?? []` 恒为空 ⇒ 两条次级出路在 UI 上**彻底不可达**
   （`safe_mode_reset` 在前端没有第二个调用点，等于 YAML 写坏的救生门失效）。
   而 Rust 契约测试 / ipc-shapes 形状闸门 / ErrorCard 源码门禁**全绿**——它们只证明"线上有这字段"
   与"组件会渲染它"，不证明**边界放行了它**。
   修法：① `normalizeError` 放行 `advancedActions`；② 新增**防漂移闸门**（用共享 fixture
   `ipc-shapes.json` 的字段表逐键验"放行"），并先证明它在缺放行时**红**；③ 顺带修同源旧缺陷：
   kind 白名单原本手抄一份且漏了 `symlink_privilege_required`（D1 的分类被静默丢弃、回退中文文案）
   → 改为由 `types/ipc.ts::BOOT_FAILURE_KINDS` **派生**（类型层唯一事实源）。
2. **替换卡上的死指针**：`emit_boot_error` 是替换语义，旧卡的按钮随之消失；枚举失败时提示却让用户
   "去点放空"。修法：新增 `BootErrorPayload::with_actions` + `boot::emit_boot_error_payload`，
   让替换卡**自己带上**唯一还能走的那一步（枚举失败 → 首屏一个「备份并放空插件配置后启动」）。
3. **首屏"恰好一个"在重选页不成立**：`/selector` 路由下 `ErrorCard` 的 `onReselect` 与 `actions`
   同处首屏 ⇒ 插件行失败会显示两个按钮（「停用全部插件并启动」+「返回重选」）。
   裁定：**保留**——「返回重选」是导航而非修复动作，且在重选页上它是唯一出口；文档口径改为
   "首屏只有一个**修复类**动作"，验收项按此描述（勿按"只有 1 个按钮"去验重选页）。

**复核发现但本轮未修（已记档，触发即做）**：

- `plugin_row_failed` 在**两本字典里都没有文案**（本轮只把 kind 放进类型层白名单，没补 copy）：
  这类卡走的是**后端中文** `title`/`suggestion`，en-US 用户看到中文。
  **为什么没顺手补**：这一类的文案必须**点名出错的行 id**，而 id 是后端拼进文案的（前端拿不到
  结构化字段）；要正确本地化得先把 `failure.row_id/package/cause` 收进前端类型、并让 `error.kinds`
  支持函数型文案（本仓已有函数型条目的先例：`safeModeBody(n)` 等），属独立意图，不夹带。
  前端防漂移测试已把 `plugin_row_failed` 记为**显式豁免**（白名单 + 注释），不是遗忘。
- 诚实门"可停行数为 0"的判据沿用 `should_disable` 的启发式（dump 段落标签格式）；
  若用户 patch 只**改写**随包行而不新增行，计划同样为空 ⇒ 门会触发、文案会断言一个它并不真正
  知道的原因。已随 #7 把文案改成只陈述已知事实，根因判据待上游 dump 变格式时一并复评（ADR §8）。

## 8. 复审条件

- 上游提供官方"安全模式/跳过用户层启动"开关 → 本方案整体退役，改用上游能力；
- 结论 11 的"升级引擎即消除致命性"被实测证实 → 安全模式降级为兜底（保留但不前置）；
- `--dump-*` 文本格式变更 → 枚举解析复评（并检查是否需要结构化输出）。
