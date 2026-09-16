# ADR-0025：启动失败的安全模式（临时 overlay 禁用用户层行）

- **状态**：**已采纳**（2026-09-16 维护者裁定：范围取 **A+**＝用户层行 ＋ 第三方 bundle 层行；
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

## 6. 行动项（实施时）

1. `safe_mode` 模块：枚举（两次 dump）＋差分＋overlay 读写＋失败回退到方案 B；
2. `LaunchSpec`/`spawn_dsh` 支持 `--patch`（版本适配同 `no_open` 先例：旧版 dsh 不认则不得传）；
3. 新 IPC（安全模式启动 / 退出）＋四处同步＋登记册；
4. 错误卡动作 + 安全模式横幅 + i18n（zh/en）；
5. 复现台账新行（结论 5–8、11 的上游行为锚点）；ADR 索引补一行；
6. 测试：overlay 生成/差分/幂等/清理、spawn 参数注入、坏 YAML 回退分支、
   「退出安全模式」不带 `--patch`；真机项进 `docs/executor.md`（坏 profile 上一键恢复）。

## 7. 复审条件

- 上游提供官方"安全模式/跳过用户层启动"开关 → 本方案整体退役，改用上游能力；
- 结论 11 的"升级引擎即消除致命性"被实测证实 → 安全模式降级为兜底（保留但不前置）；
- `--dump-*` 文本格式变更 → 枚举解析复评（并检查是否需要结构化输出）。
