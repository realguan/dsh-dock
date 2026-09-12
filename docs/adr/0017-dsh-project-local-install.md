# ADR-0017：dsh 改为 project 内安装 + 自建 shim（Windows 免符号链接特权）

- **日期**：2026-09-11
- **状态**：已接受
- **提出人**：guan（AI 协作，Lead 起草）
- **相关方**：`engines.rs`（引导/探测）· `updates.rs`（升级链）· `shell.rs`（dsh 执行入口）· ADR-0010 / ADR-0005 / ADR-0009
- **关联**：`docs/known-issues/v120-测试问题.md` 1.1 · `docs/team/V120-D1-引擎引导全局安装.md` §6（本 ADR 即其方案 B）· `docs/team/V120B-Windows探针-方案B验证.md`（Windows 实机探针）· roadmap §4.16

---

## 1. 背景与问题

Windows 普通账户（未开开发者模式、非管理员）下，v1.1.0/v1.2.0/v1.2.1 的**本地模式引擎引导必然失败**，
用户看到「引擎引导失败」。v1.2.1 已把**诊断**修对（分类为特权不足并给出出路），但**安装能力未变**。

根因（已逐条证实）：

1. pnpm 12 的**全局安装**会创建 `…/engines/global/v11/<hash>` 到真实安装目录的**符号链接**
   （自带 pnpm 12.3.1 二进制内文案：`link the global package install directory at`）。
2. Windows 创建符号链接需要 `SeCreateSymbolicLinkPrivilege`（管理员）或用户手动开启「开发者模式」；
   普通账户 `CreateSymbolicLink` → `ERROR_ACCESS_DENIED (os error 5)`。
3. 该机制**无法用配置规避**：本机穷举 7 个候选
   （`enableGlobalVirtualStore` / `globalVirtualStoreDir` / `virtualStoreOnlyBinaryResolution`
   / env `PNPM_CONFIG_*` / `NPM_CONFIG_*` / CLI `--config.*` / workspace 键）**均未消除该链接**，
   且阴性结论已自证（写非法值 pnpm 硬报错 ⇒ 配置确被读；env 注入目录确被创建 ⇒ 注入确生效）。
4. 既有的 `nodeLinker: hoisted` 修复（v1.1.0）只解决了 **node_modules 侧**链接，
   管不到 global hash link —— 故 node 装得上（截图「环境检测通过」），dsh 装不上。

**这不是我们能修的 pnpm 配置问题，是 pnpm 12 的上游行为**（上游同类报告：pnpm#13694）。

## 2. 约束与硬指标

- **不得要求管理员**：桌面应用不能把「以管理员运行」当常规前提（ADR-0010 精神：引擎是壳资产，
  首启自补齐，用户零配置）。
- **不得引入提权路径**：壳的 WebView 承载 dsh 工作台并会加载**用户安装的第三方插件 JS**；
  壳一旦提权，WebView、dsh 及其 spawn 的一切都成管理员权限 —— 信任面从「装一个包」放大到
  「整个壳 + 所有插件」，且引擎安装**只在首启一次**需要该能力，严重不成比例。
- **不改用户全局 pnpm 配置**（ADR-0005 既有约束）。
- **不修改 dsh 源码**（AGENTS 红线 1）。
- **`engines/bin/dsh` 仍须是可被 `find_engine_tool` 找到的可执行入口**
  （Windows 查找序：`dsh.exe` → `dsh.cmd` → `dsh`），否则 `readiness_gaps` 与启动链全断。
- **唯一网络面**不动：安装仍经 `updates.rs` 的 registry 镜像链与 `pnpm_allow_build_flags()`。

## 3. 备选方案及评估

| 方案 | 做法 | 评估 |
|:---|:---|:---|
| **A. npm 全局回退** | 特权类失败时改走 `npm i -g`（npm 在 Windows 用 cmd 垫片，不需特权） | 改动小，但：① ADR-0005（2026-08-28 补录）明写「装 dsh 本体**不再回退 npm**」，pnpm 是硬依赖，回退会让引导链出现**第二条安装路径**（双实现）；② `install_global_dsh_npm` 已随该裁定**删除**，需重建；③ 全局安装仍会引入 npm 自己的全局目录语义，与 `PNPM_HOME` 单目录引擎布局打架 |
| **B. project 内安装 + 自建 shim** | 不再 `pnpm add -g`；在 `<engines>/dsh-runtime/` 内 `pnpm add @deepseek-ai/dsh@<ver>`（沿用 `nodeLinker: hoisted`），并在 `<engines>/bin/` 自写 `dsh` / `dsh.cmd` 指向该包入口 | **✅ 采纳**。结构性绕开 global hash link；**零特权**；不引入第二条安装器（仍是 pnpm）；可复用既有 registry 链与 allow-build 口径 |
| **C. 自动提权（UAC）** | `ShellExecuteW` + `runas`，或提升一次性子进程 | **否决**：违反 §2 两条硬指标（要求管理员 / 引入提权路径）。窄范围一次性子进程虽可降低面，但仍把「从 npm 装包」置于管理员上下文，且需新增 UAC 交互与失败分支 —— 为一次首启付出长期安全边界代价 |
| **D. 只做引导**（现状 + 深链开发者模式设置页） | 不改安装方式 | 已部分落地（v1.2.1 的诊断与出路）。**不足以单独成案**：用户若坚持本地模式仍无解；但可作为**补充**保留（见 §5） |

## 4. 决策

**采纳方案 B**：dsh 从「pnpm 全局包」改为「壳自管 project 依赖 + 壳自写 shim」。

### 4.1 形态

```
<engines>/
├── bin/
│   ├── node            # 既有（link_real_node_binary）
│   ├── pnpm            # 既有（内置引导器）
│   ├── dsh             # 【新】壳自写 shim（POSIX）
│   └── dsh.cmd         # 【新】壳自写 shim（Windows）
├── dsh-runtime/        # 【新】dsh 的 project 安装目录
│   ├── package.json
│   ├── pnpm-workspace.yaml   # nodeLinker: hoisted
│   └── node_modules/@deepseek-ai/dsh/lib/bin.js
├── node_modules/ store/ package.json   # 既有：node 运行时
└── global/             # 【退役】不再由 dsh 安装创建
```

shim 内容（纯文本，**不是链接**）：

- POSIX：`exec "<bin>/node" "<bin>/../dsh-runtime/node_modules/@deepseek-ai/dsh/lib/bin.js" "$@"`
- Windows：`@echo off` + `"%~dp0node.exe" "%~dp0..\dsh-runtime\node_modules\@deepseek-ai\dsh\lib\bin.js" %*`

`dsh` 包的 bin 字段已核实为 `{"dsh": "lib/bin.js"}`，入口稳定。

### 4.2 为什么 shim 是必要的（而不是直接用 `.bin`）

`node_modules/.bin/*` 在 POSIX 上是**符号链接**；虽然 Windows 上是 cmd 垫片（探针已证），
但**依赖平台差异会让方案分裂**。自写 shim 把入口固定在壳的 `bin/` 下，
`find_engine_tool` 的既有查找序（`dsh.exe` → `dsh.cmd` → `dsh`）**无需改动**。

## 5. 后果与影响

**收益**：
- Windows 普通账户**无需管理员/开发者模式**即可完成引擎引导（1.1 的根因消除，不再是"只修诊断"）。
- 不再触碰 global hash link ⇒ 该失败模式**结构性消失**（非"降低概率"）。
- 引导链仍只有 pnpm 一个安装器（不引入 ADR-0005 曾否决的 npm 双路）。

**代价 / 接受的取舍**：
- **`engines/global/` 退役**：存量已装用户需迁移（见 §5.1），不能假设目录为空。
- 壳需自行维护 shim（新增少量平台分叉代码）；shim 是**壳的资产**，版本随壳走。
- 与 ADR-0010「引擎=壳资产」一致，但其**安装形态**一节被本 ADR 修订；ADR-0005 的
  `global-bin-dir` 注入对 dsh 安装不再适用（对 pnpm 自身仍适用）。

### 5.1 迁移（不破坏存量）

- 已就绪的旧布局（`global/v11/…` + pnpm 生成的 `bin/dsh`）**继续可用**：`readiness_gaps`
  只判「三件是否在位且版本匹配」，不判布局 ⇒ **老用户不会因本改动被迫重装**。
- 仅当需要安装/升级 dsh 时，走新路径写入 `dsh-runtime/` 并**覆盖** `bin/dsh`（壳自写）。
- 旧 `global/` 不主动删除（避免误删用户可能正在使用的包），登记为可回收项。

### 5.2 WSL 客体

本 ADR **只改宿主**：Linux 无符号链接特权问题，客体沿用既有 `add -g`。
两侧布局分叉属**既有事实**（宿主/客体本就各有安装脚本），本条登记为观察项，
不因本次改动扩大分叉面。

## 6. 验证方式

- **单元/契约级**（本机可跑）：安装形态断言（不再调用 `add -g`、shim 内容与目标正确）；
  shim 生成的双平台形态各一例；`readiness_gaps` 对新旧布局均判就绪。
- **源码闸门**：`engines.rs` 生产段不得再出现 `add -g` / `install_dsh_global` 的全局安装实参。
- **真机判据（Windows，普通账户、开发者模式关闭）**：即
  `scripts/probe-windows-engine-install.ps1` 已验证的前提——非全局安装 exit 0、
  未创建 `global\`、`.bin` 为普通文件；修后应能观测到 `engines\bin\dsh.cmd` 存在且
  `dsh --version` 有输出、健康大盘三卡就绪、`engines\global\` **不再新增**。
- 三平台 CI 全绿 + `clippy` 逐目标（AGENTS §1）。

## 7. 关联决策

- **修订 ADR-0010**（引擎资产获取/安装形态一节）：dsh 的安装形态由「pnpm 全局包」改为
  「壳自管 project 依赖 + 壳自写 shim」；「引擎=壳资产」「首启自补齐」等结论**不变**。
- **不冲突 ADR-0005**：其核心（显式注入 `global-bin-dir`、不改用户全局配置）对 pnpm 自身仍适用；
  仅「dsh 经全局安装」这一句被本 ADR 取代。
- **ADR-0009 口径 2**（pnpm 为硬依赖）**不变**：本方案仍只用 pnpm。
