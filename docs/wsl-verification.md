# WSL 支持 · Windows 实机验证清单（迭代 v1）

> 面向 Windows 实机的验收：dsh-dock 的「在 WSL 中打开」全链路。
> 清单按「预期行为 + 对应代码假设」组织；任何一项不符，把日志和现象按文末模板回传即可定位。

## 0. 前置条件（朋友机器需要满足）

| 项 | 要求 |
|:--|:--|
| 系统 | Windows 10/11（较新更新，带 WSL2） |
| WSL | `wsl --status` 有输出；至少一个发行版是 **WSL2**（`wsl -l -v` 看 VERSION=2） |
| 发行版内 | 已装 `node` 与 `dsh`（`npm i -g @deepseek-ai/dsh`，WSL 内执行） |
| 网络 | 首次运行灰本机（非 WSL）路径可能在线补齐 Node/dsh，需能联网 |

> 说明：本文件稿时是"在线极简档"——「在 WSL 中打开」只要求 **WSL 内**已有 node+dsh；
> 本机（Windows 侧）路径仍走 system→download 在线补齐。若只想验 WSL，Windows 侧
> 第一次启动会先下载 Node/dsh（在线档特性，非 WSL 逻辑）。

## 1. 怎么跑

> 首选 **CI 产物**（推荐），次选 **装 Rust 跑仓库**。
> ~~本机交叉产出 exe~~ 不可用：macOS 的 mingw-w64 工具链在 Tauri 依赖（新版
> `windows` crate）下报 `export ordinal too large`，属于交叉链接工具链限制（ld 序数上限）。

### 方式 A（推荐）：CI 产出的 Windows 安装包

1. 把本分支推到 GitHub（`git push -u origin <分支名>`）→ 触发
   `.github/workflows/build.yml` 的 `windows-latest` 矩阵。
2. CI 跑完，进 Actions 页面该次运行的 **Artifacts** 下载 Windows 安装器
   （NSIS `.exe` 或 `.msi`，即 `*.nsis.zip` / `*.msi.zip` 或打包 zip）。
3. 在朋友机器安装并启动 → 出现启动页（时间线 0-4 步）。

### 方式 B：仓库 + Rust

1. 装 Rust：`winget install Rustup` 或用 rustup-init.exe，装默认 toolchain。
2. `git clone <仓库> && cd dsh-dock/src-tauri && cargo run`（dev 从源码树取 resources）。

> 两种方式都以"在线极简档"运行：首次启动会在 Windows 侧在线补齐 Node/dsh（**非 WSL 逻辑**，
> 是本地档的既有特性）；「在 WSL 中打开」只要求 **WSL 内**已有 node+dsh。

## 2. 验证矩阵

启动后按顺序跑，每步记下「预期 → 实际」。

| # | 动作 | 预期 | 对应代码假设 |
|:-:|:--|:--|:--|
| 0 | 首次启动（无 settings.json） | 进入**运行环境选择页**（mode.html：本机 / WSL2 + 设为默认勾选） | `settings::load` 无 defaultMode → 导航 mode.html |
| 1 | 选「WSL2」并勾选「设为默认」→ 开始 | 出现"探测 WSL"时间线 → 步 0 就绪 → 步 1 发行版内发现 dsh → 自动进工作台 | `choose_mode` → `executor_for_mode(Wsl)` → WSL probe |
| 2 | 托盘右键（Windows）→「打开方式：本机」 | 会话切回本机（时间线重走，随后进工作台）；托盘 ✓ 移到本机 | `switch_mode(Local)`（teardown→写默认→launch） |
| 3 | 再切回 WSL（托盘 →「打开方式：WSL2」） | 同上反向切换；结束时仍无残留 | `switch_mode(Wsl)` + stop 标志 teardown |
| 4 | 重启应用 | **不再弹选择页**，直接按上次默认（WSL）启动并进入工作台 | settings.json defaultMode 生效 |
| 5 | 在工作台随便聊一句 | 能正常收发（证明 **127.0.0.1:端口 确实穿透到 WSL 内的 dsh**） | capabilities `http://127.0.0.1:*` 不变即够 |
| 6 | 关掉应用 | 无残留：WSL 内 dsh 应停止 | teardown touch `GUEST_STOP_FILE` → wrapper kill dsh |
| 7 | （可选守环境）WSL 内 `ps aux \| grep -i dsh` | 无 dsh 残留进程 | 同上（孤儿检查） |

**探测若报"需 WSL2"**：这是**预期行为**（WSL1 不支持端口转发）。用 `wsl --set-version <发行版> 2` 升后再试。

**友情机发行版内的 PATH**：若 dsh 是经 nvm/fnm 装的，probe/boot 模板已先 source `~/.bashrc`
补 PATH——此项也顺带验证。

## 3. 日志位置（出问题先看这里）

| 文件 | 内容 |
|:--|:--|
| `%APPDATA%\io.github.realguan.dsh-dock\shell.log` | 壳诊断日志（Rust tracing） |
| `%APPDATA%\io.github.realguan.dsh-dock\dsh-wsl.log` | WSL 模式：客体内 dsh 的 stdout/stderr（URL 解析来源） |
| `%APPDATA%\io.github.realguan.dsh-dock\dsh-shell.log` | 本机模式日志 |
| 启动页「启动详情」/ 错误卡「原始日志」 | 前端看到的错误信息与动作 |

## 4. 已知风险点（验证时应特别留意）

- **teardown 的停止可靠性**：依赖客体内 wrapper 读 stop 标志。若关应用后 WSL 内仍有
  `dsh` 残留（第 6/7 步失败），把 `shell.log` 尾部回传（需要时再评估 `wsl --terminate` 兜底）。
- **`wsl -l -v` 输出解析**：中文/其它区域 Windows 的表头与横幅不应影响解析（只认 `*` + 版本列）；
  若探测把发行版名解析乱（乱码），说明遭遇了 wsl.exe 非 UTF-8 输出，把现象回传。
- **doc 内未覆盖**：WSL 模式下客体内「缺失 dsh」应出现"请在 WSL 内 `npm i -g ...` 后重试"
  的可行动错误卡（第 1 步前置条件不满足时）。

## 5. 回传模板（任一现象不符预期时）

```
环境：Windows 版本 / wsl --version / wsl -l -v 输出
步骤：第几项、做了什么
现象：错误卡文案 or 时间线卡在/报错哪一步
日志：shell.log 与 dsh-wsl.log 的尾部（或截图）
```

---

校验的代码假设集中处：`src-tauri/src/executor.rs`（WSL 执行器）/ `lib.rs`
（会话切换 + 代际）/ `settings.rs`（默认运行环境）/ `ui/mode.html`（首次选择）。
设计说明见 [executor.md](executor.md)。
