# 方案 B 验证：Windows 只读探针（2026-09-11）

> 目的：回答「**非全局安装 dsh 在 Windows 普通账户下是否需要符号链接特权**」。
> 这是方案 B 唯一无法在 macOS 上判定的事实，其余环节 Lead 已在本机实测通过（见 §0）。
> **本探针只读**：不改动应用 `engines/`、不写注册表、不装全局包；只在 `%TEMP%` 下建临时目录。
> 请**不要在管理员 PowerShell** 里跑——我们要验的正是「普通账户」。

## 0. 已在 macOS 实测通过的部分（无需你在 Windows 重验）

| 判据 | 结果 |
|:---|:---|
| 非全局 `pnpm add`（`nodeLinker: hoisted`）是否创建 `global/` | **未创建** —— 完全不触碰 global hash link 机制 |
| 包树内符号链接数 | 仅 `.bin/*` 为链接；**包目录本身零链接**（真实目录） |
| 自建 shim（157 字节 `sh` 文本文件）指向 `dsh-runtime/node_modules/@deepseek-ai/dsh/lib/bin.js` | **`dsh --version` → `0.1.5-rc.2` 成功** |
| `dsh` 包的 bin 字段 | `{"dsh": "lib/bin.js"}` |

⇒ 即：**除 `.bin` 一条外，方案 B 全链路已证可行**。而 `.bin` 我们**根本不用**（自建 shim 替代）。

## 1. 唯一待你在 Windows 上回答的问题

pnpm 在 Windows 上创建 `node_modules/.bin/*` 时，用的是**符号链接**（需要特权 → 会失败）
还是**`.cmd` 垫片文件**（真实文件 → 不需要特权）？

- pnpm 有 `prefer-symlinked-executables` 设置（本机 `strings` 确认存在于内置 pnpm 12.3.1 的
  设置目录中），业界惯例是 Windows 下走垫片；**但这是 Windows 专属行为，我不能凭空断言**。
- **若为垫片 → 方案 B 立刻可实施**（无需管理员/开发者模式）。
- **若为符号链接 → `.bin` 是新的拦路虎**，需要用 `bin-links=false` 或改 `modules-dir` 规避，
  届时我据探针结果调整方案。

## 2. 探针（已落成脚本文件，不要在控制台粘贴）

**脚本位置**：`scripts/probe-windows-engine-install.ps1`（仓库内，随 master 推送）

在**普通（非管理员）** PowerShell 窗口运行：

```powershell
git pull                                  # 或直接使用仓库内现有副本
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\probe-windows-engine-install.ps1
```

为什么不让你粘贴：脚本里有跨行管道（`Get-ChildItem |` 换行接 `Where-Object`）与
`if/else` 块，交互式粘贴按行执行容易出错；而且 Windows PowerShell 5.1 读 `.ps1`
若无 BOM 会按 ANSI 解码。**故脚本输出已写成纯 ASCII**（实测 0 个非 ASCII 字节、
无 BOM），任何控制台代码页都不会让它乱码。

脚本是**只读**的：只在 `%TEMP%` 下建临时目录、跑完自删；不碰 `engines/`、
不写注册表、不装全局包。若 `pnpm.exe` 尚未落位（应用没启动过引擎），它会明确
提示并中止，不会报一堆看不懂的错。

## 3. 判读口径（看 §2 输出怎么读）

| 观察 | 结论 |
|:---|:---|
| 第 2 节退出码 **0** + 3a 未创建 `global\` | 方案 B **可行**，且不需要任何特权 ⇒ 直接实施 |
| 第 2 节退出码 **非 0**，报 `os error 5` / `拒绝访问` | 失败点是 `.bin` 链接 ⇒ 改用 `--config.bin-links=false` 或改 `modules-dir` 规避，Lead 据 3b 调整 |
| 3b 显示 `.cmd` 垫片 | 与业界惯例一致，印证上面的判断 |
| 3b 显示 `SYMLINK/JUNCTION` | 需要规避，但**方案 B 依然可行**（我们不用 `.bin`，只要能让安装不因它失败） |

**关键**：即便 `.bin` 是符号链接，方案 B 距离成功也只差一个"别让 pnpm 建 `.bin`"的开关——
而 `global hash link` 那条路是**结构性地无法规避**的（已穷举 7 个候选配置）。
这就是我推荐 B 的理由。

## 4. 探针若通过，后续实施范围（供你预估，尚未动码）

按 AGENTS §9，改安装形态属**架构决策 → 必须先立 ADR**（拟修订 ADR-0010/ADR-0005）：

1. `src-tauri/src/engines.rs`：`install_dsh_global` → 改为 project 内安装
   （`<engines>/dsh-runtime/`，沿用既有 `pnpm_allow_build_flags()`）；
2. 新增自建 shim 生成（POSIX `dsh` + Windows `dsh.cmd`），指向
   `dsh-runtime/node_modules/@deepseek-ai/dsh/lib/bin.js`；
3. 升级链路（版本切换/回滚）与 `readiness_gaps` 判定同步；
4. WSL 客体路径是否同构（Linux 无该问题，倾向保持一致以减少分叉）；
5. 回归测试：安装形态断言 + shim 内容断言 + 「不再调用 `add -g`」源码闸门；
6. **真机验证锚**：本探针第 2 节可作为"修前必红 / 修后必绿"的实机判据。
