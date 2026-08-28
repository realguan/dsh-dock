# dsh 行为复现台账（Behavior Ledger）

> 红线 1 允许「文件系统层复现 dsh 行为」——复现 = 与 dsh 源码的影子同步。
> dsh 升级后复现点可能**静默漂移**（CI 测的是壳自己的逻辑，发现不了）。
> 本台账是全部复现点的唯一登记处；**宿主解析命中新版 dsh（或 dsh 大版本升级）时，
> 须逐条复核第一节并更新「最后复核」列**，复核结论随广播知会。
> 锚点注释书写规范见 AGENTS.md §0 红线 1（源码参考位置 + 日期注释）；
> 决策推理见对应 ADR，本册只登记「是什么、在哪、锚什么」。

**基线**：dsh v0.1.1-rc.2（2026-08-27 架构核查口径，见 roadmap §1）

## 一、已落地复现点（随每次 dsh 升级复核）

| # | 复现点 | 壳侧位置 | 依赖的 dsh / 工具链行为 | 最后复核 |
|:--|:--|:--|:--|:--|
| 1 | dsh 就绪判定 | `shell.rs`（`--port 0` → 日志轮询 URL；无进展判 `Stalled`） | dsh 启动日志格式（打印访问地址） | 基线 |
| 2 | 宿主 dsh 版本闸 | `resolve.rs`（`version_at_least` / `engines.node` / 平台三重闸） | dsh `engines.node` 声明语义 | 基线 |
| 3 | pnpm global-bin-dir 注入 + npm 回退 | `updates.rs`（`pnpm_global_bin_dirs`，ADR-0005） | pnpm 10 全局目录解析（GUI 无 rc） | 基线 |
| 4 | WSL PATH 兼容探测 | `executor.rs`（`bash -lic` + nvm/fnm/n/volta 兜底扫描） | nvm/fnm 非交互 rc 守卫行为 | 2026-08-26 |
| 5 | WSL 客体内 dsh 自动安装 | `executor.rs`（ADR-0004：壳不触网，装进发行版） | dsh npm 包名与 `npm i -g` 语义 | 基线 |

## 二、计划复现点（4.3 Profile 管理器落地时入册）

| # | 复现点 | 依据 | 锚定的 dsh 行为 |
|:--|:--|:--|:--|
| 6 | profile 列举 / 详情（文件系统模拟） | ADR-0009 方案 E | `profiles/<名>/` 目录布局与三件套格式 |
| 7 | 创建 profile 半官方引导 | ADR-0009 方案 A | `dsh plugin add` 首用初始化（initProfile 三件套） |
| 8 | profile 非法名校验 | ADR-0009 硬指标 | `resolveProfileDir` 校验规则（空名 / `/` `\` / `.` / `..` / `node_modules`） |

## 三、复核记录（append-only）

- 2026-08-28 建册：基线 v0.1.1-rc.2，已落地 5 项、计划 3 项，全量登记。
