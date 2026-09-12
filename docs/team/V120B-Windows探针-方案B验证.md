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

## 2. 探针（复制整段到**普通** PowerShell 窗口运行）

```powershell
$ErrorActionPreference = 'Continue'
$App  = Join-Path $env:APPDATA 'io.github.realguan.dsh-dock'
$Pnpm = Join-Path $App 'engines\bin\pnpm.exe'
$Node = Join-Path $App 'engines\bin\node.exe'

Write-Host "===== 0. 环境事实 =====" -ForegroundColor Cyan
$devMode = (Get-ItemProperty -Path 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock' -Name AllowDevelopmentWithoutDevLicense -ErrorAction SilentlyContinue).AllowDevelopmentWithoutDevLicense
Write-Host "开发者模式 AllowDevelopmentWithoutDevLicense = $devMode   (1=已开, 0/空=未开)"
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
Write-Host "当前是否管理员 = $isAdmin   (应为 False；若是 True 请换普通窗口重跑)"
Write-Host "pnpm 存在 = $(Test-Path $Pnpm)   node 存在 = $(Test-Path $Node)"

Write-Host "`n===== 1. 建临时项目（仅 %TEMP%）=====" -ForegroundColor Cyan
$Tmp = Join-Path $env:TEMP ('dsh-b-probe-' + (Get-Random))
$Proj = Join-Path $Tmp 'proj'
$Home2 = Join-Path $Tmp 'home'
New-Item -ItemType Directory -Force -Path $Proj, $Home2 | Out-Null
Set-Content -Path (Join-Path $Proj 'pnpm-workspace.yaml') -Value 'nodeLinker: hoisted' -Encoding ascii
Write-Host "临时目录 = $Tmp"

Write-Host "`n===== 2. 非全局安装一个小包（决定性步骤）=====" -ForegroundColor Cyan
$env:PNPM_HOME = Join-Path $Tmp 'pnpmhome'
Push-Location $Proj
& $Pnpm add semver --registry=https://registry.npmjs.org 2>&1 | ForEach-Object { $_ }
$code = $LASTEXITCODE
Pop-Location
Write-Host "pnpm add 退出码 = $code   (0=普通账户下成功 → 方案 B 可行)"

Write-Host "`n===== 3. 关键判据 =====" -ForegroundColor Cyan
Write-Host "--- 3a. global\ 是否被创建（触碰 hash link 的标志）---"
if (Test-Path (Join-Path $env:PNPM_HOME 'global')) { Write-Host "  ⚠️ 创建了 global\ —— 仍触发了全局安装机制" -ForegroundColor Yellow }
else { Write-Host "  ✅ 未创建 global\ —— 未触碰 hash link" -ForegroundColor Green }

Write-Host "--- 3b. .bin 条目形态（垫片 vs 符号链接）---"
$binDir = Join-Path $Proj 'node_modules\.bin'
if (Test-Path $binDir) {
  Get-ChildItem $binDir -Force | ForEach-Object {
    $isLink = ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0
    $t = if ($isLink) { "SYMLINK/JUNCTION" } else { "普通文件(垫片)" }
    Write-Host ("  {0,-24} {1}" -f $_.Name, $t)
  }
} else { Write-Host "  （无 .bin 目录）" }

Write-Host "--- 3c. 包树内是否有符号链接/联接（排除 .bin）---"
$links = Get-ChildItem $Proj -Recurse -Force -ErrorAction SilentlyContinue |
         Where-Object { ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -and $_.FullName -notlike '*\.bin\*' }
if ($links) { $links | ForEach-Object { Write-Host ("  ⚠️ " + $_.FullName) } }
else { Write-Host "  ✅ 零链接（方案 B 需要的形态）" -ForegroundColor Green }

Write-Host "`n===== 4. 自建 shim 能否跑通（模拟 engines\bin\dsh.cmd）=====" -ForegroundColor Cyan
# 用一个有 bin 的包演示：semver 的入口
$entry = Join-Path $Proj 'node_modules\semver\bin\semver.js'
if (Test-Path $entry) {
  $shim = Join-Path $Tmp 'dsh-shim.cmd'
  # 生产里这一行会指向 dsh-runtime\node_modules\@deepseek-ai\dsh\lib\bin.js
  Set-Content -Path $shim -Value "@echo off`r`n`"$Node`" `"$entry`" %*" -Encoding ascii
  Write-Host "shim 文件已是普通文件 = $((Get-Item $shim).Attributes -notmatch 'ReparsePoint')"
  & $shim --version 2>&1 | ForEach-Object { Write-Host "  semver 版本 = $_" }
} else { Write-Host "  （未找到 semver 入口，跳过）" }

Write-Host "`n===== 5. 清理 =====" -ForegroundColor Cyan
Remove-Item -Recurse -Force $Tmp -ErrorAction SilentlyContinue
Write-Host "已删除 $Tmp"
Write-Host "`n请把上面全部输出（尤其第 2、3 节）贴回给 Lead。" -ForegroundColor Green
```

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
