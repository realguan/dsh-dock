# ADR-0004：WSL 客体内 dsh 自动安装（壳不触网）

- **日期**：2026-08-26
- **状态**：已接受（2026-08-26 登记，本 ADR 追溯补录）
- **提出人**：guan
- **相关方**：executor.rs（`GUEST_INSTALL_DSH` / `install_dsh_in_distro` / `NODE_MISSING` / `GuestProbeState`）
- **关联**：docs/executor.md（WSL 执行器）

---

## 1. 背景与问题

WSL 执行器探测到「客体内有 node 缺 dsh」时，需要补齐 dsh。但壳的运行时网络面白名单只有 `updates.rs`（§7 边界）。若让 Windows 侧壳跨过 WSL 边界装 dsh，既越网络边界、也把镜像配置权收进壳（不该如此）。需把安装动作放进 WSL 发行版内，壳只透传脚本。

## 2. 约束与硬指标

- 壳运行时网络面只在 `updates.rs`——Windows 侧壳不得因 WSL 安装触网。
- 镜像配置由用户客体内 npm 决定，壳不注入镜像参数（用户主权）。
- 缺 node（`NODE_MISSING`）不自动装 Node——发行版安装方式属用户主权，只给可行动提示。
- 固定脚本模板透传，不注入用户输入（防注入）。
- 安装可能慢，需超时 + 诊断回传，但全量 npm 输出不回传（噪声大）。

## 3. 备选方案及评估

### 方案 A：固定脚本经 wsl.exe 透传 + 日志落客体 /tmp + 回传尾部 —— ✅ 最终采纳

- 思路：`GUEST_INSTALL_DSH` 固定脚本（`concat!`，无用户输入插值）经 `wsl.exe -e bash -lic` 执行；npm 输出落 `/tmp/dsh-dock-npm.log`，只回传尾部 2KB 诊断；缺 node 走 `NODE_MISSING` 只给提示。
- 优点：网络在 WSL 发行版内（壳不触网）；镜像由客体 npm 决定；脚本固定防注入；诊断够用又不刷屏。
- 代价/风险：依赖客体内有 node/npm；2KB 诊断极端情况可能不够。
- 对照约束：逐条满足。

### 方案 B：Windows 侧壳直接 npm 装 dsh —— ❌ 否决

- 思路：壳在 Windows 侧联网装 dsh 到 WSL 文件系统。
- 否决理由：违反「网络只在 updates.rs」与「镜像配置属用户主权」两条硬指标。

### 方案 C：缺 node 也自动装 Node —— ❌ 否决

- 思路：探测到 `NODE_MISSING` 时自动在客体内装 Node。
- 否决理由：发行版 Node 安装方式（nvm / apt / 源码）属用户主权，壳替用户决定会踩各发行版的坑；只给可行动提示让用户自选。

## 4. 最终决策

WSL 探测到「有 node 缺 dsh」→ 在客体内执行固定脚本模板 `GUEST_INSTALL_DSH`（经 `wsl.exe` 透传），`npm i -g @deepseek-ai/dsh` 输出落 `/tmp/dsh-dock-npm.log`，只回传尾部 2KB 诊断。网络动作发生在 WSL 发行版内，镜像配置由客体内 npm 决定。缺 node（`NODE_MISSING`）不自动装 Node，只给可行动提示。HOW 见 `executor.rs`。

## 5. 后果与后续行动项

### 正面后果
- 壳网络边界不被打破；镜像配置权留在用户客体；零配置体验（有 node 即用）。

### 负面后果 / 新增债务
- 依赖客体有 node/npm——缺 node 场景只能给提示，不能一键补齐。
- `/tmp/dsh-dock-npm.log` 路径写死，不同发行版 `/tmp` 行为差异（极少）。

### 行动项
- [x] `GUEST_INSTALL_DSH` / `install_dsh_in_distro` / `NODE_MISSING` 分类与提示实现（executor.rs）。
- [x] `GuestProbeState` 三态分类测试（executor.rs）。

## 6. 复审条件

- WSL 探测方式变化（如改用 `wsl --status` / 发行版 API）→ 复评脚本透传路径。
- 客体内镜像注入策略调整（如壳需兜底默认镜像）→ 本决策重开。
- Node 安装可经 corepack / 官方脚本一键化且跨发行版稳定 → 复评 `NODE_MISSING` 是否仍「只提示」。
