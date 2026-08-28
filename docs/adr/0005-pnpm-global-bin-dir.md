# ADR-0005：pnpm 全局安装显式注入 global-bin-dir

- **日期**：2026-08-26
- **状态**：已接受（2026-08-25 裁定，本 ADR 追溯补录）
- **提出人**：guan
- **相关方**：updates.rs（`install_global_dsh_pnpm` / `pnpm_global_bin_dirs` / `install_global_dsh_npm`）
- **关联**：AGENTS.md §7（网络面 = updates.rs）
- **注（2026-08-27 边界重定义）**：本 ADR 的「pnpm 全局安装」是运行时宿主链（dsh 安装
  到全局）路径；4.3 管理功能的 `dsh plugin --profile` 转发链是 **profile 目录内安装**，
  不涉及 global-bin-dir（见 Spike A）。二者同源环境风险但机制不同，本 ADR 不覆盖
  4.3 转发链。

---

## 1. 背景与问题

壳经 pnpm 优先、npm 回退把 dsh 装进全局。但 GUI 子进程不加载 shell rc，`PNPM_HOME` 环境变量对 pnpm 10 无效，`global-bin-dir` 缺省为 undefined → `pnpm add -g` 报 `ERR_PNPM_NO_GLOBAL_BIN_DIR` 失败，回退 npm（慢）。需要一个不依赖 shell rc 的显式注入方式。

## 2. 约束与硬指标

- 不依赖 shell rc（GUI 子进程不加载）。
- pnpm 失败必须能回退 npm（不阻断安装）。
- 不改用户全局 pnpm 配置文件（侵入用户环境）。
- `root -g` 等后续命令也要带上同一注入（一致）。

## 3. 备选方案及评估

### 方案 A：经 `--config.global-bin-dir=<pnpm 父目录>` 显式注入 —— ✅ 最终采纳

- 思路：`pnpm_global_bin_dirs` 取 pnpm 可执行文件父目录，生成 `--config.global-bin-dir=<dir>` 参数；`install_global_dsh_pnpm` 与 `root -g` 都带上；pnpm 失败回退 npm。
- 优点：不依赖 rc、不改用户配置文件、命令级注入即生效；回退 npm 兜底。
- 代价/风险：父目录需可写（一般 pnpm 安装位可写）；注入是命令行参数，多一处需同步。
- 对照约束：逐条满足。

### 方案 B：依赖 `PNPM_HOME` 环境变量 —— ❌ 否决

- 思路：给子进程设 `PNPM_HOME`。
- 否决理由：对 pnpm 10 在无 rc 环境实测无效（正是问题根因）。

### 方案 C：写用户全局 pnpm config —— ❌ 否决

- 思路：`pnpm config set global-bin-dir` 持久化。
- 否决理由：侵入用户全局环境，违反「不改用户配置」。

## 4. 最终决策

`install_global_dsh_pnpm` 一律经 `pnpm_global_bin_dirs` 注入 `--config.global-bin-dir=<pnpm 可执行文件父目录>`；`root -g` 同步注入；pnpm 失败回退 `install_global_dsh_npm`。HOW 见 `updates.rs`。

## 5. 后果与后续行动项

### 正面后果
- pnpm 全局安装不再因 `global-bin-dir` 缺省失败；npm 回退兜底保底。

### 负面后果 / 新增债务
- 注入依赖「pnpm 可执行文件父目录可写」，非标准安装位（如只读系统目录）仍会回退 npm。
- pnpm 跨版本行为变化时需复测。

### 行动项
- [x] `pnpm_global_bin_dirs` / `install_global_dsh_pnpm` 实现 + 测试（updates.rs）。

## 6. 复审条件

- pnpm 修复 `PNPM_HOME` 在无 rc 环境的生效 → 复评是否仍需显式注入。
- 改走 corepack 管理 pnpm → 本决策重开。
- pnpm 跨版本 `global-bin-dir` 语义变化 → 复测注入。
