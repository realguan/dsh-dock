# 执行环境抽象（executor）——local / wsl（ssh 预留）

## 状态

版本：v1（local 重构完成 + wsl 迭代 v1）｜状态：演进中（ssh 预留，未实现）｜消费方：shell.rs / lib.rs 启动链、`boot:step` / `boot:error` 遥测

> WSL 行为细节见 ADR-0004 与本文档；SSH 仅预留配置形状。

## 定位

壳的价值主张是"**壳是通用机制**"——产品是数据（工作台由 `product.manifest.json` 驱动）。
执行环境是这条线的自然延伸：**运行环境也是数据**。壳只与 `Executor` 会话打交道，
不感知 dsh 是跑在本机、WSL2 发行版内、还是（将来）远端 SSH。

```
probe（探测环境，步0-1）→  [NeedsProfile → 选择器/select_profile]
                    →  start（启动，步2）
                    →  壳统一 wait_for_ready（log_path + check_exited 轮询）
                    →  导航 → guard（监护） → teardown（壳退/错误卡/切模式共用）
```

## 接口（`src-tauri/src/executor.rs`）

```rust
pub trait Executor: Send {
    fn kind(&self) -> ExecutorKind;                       // local / wsl / ssh（遥测）
    fn probe(&mut self, sink: BootSink, progress: DownloadProgress)
        -> Result<ProbeOutcome, String>;                  // 失败 → 可行动错误卡
    fn select_profile(&mut self, profile: String);        // F-b 选择器回调
    fn start(&mut self, sink: BootSink) -> Result<(), String>;
    fn log_path(&self) -> PathBuf;                        // 就绪要轮询的本地日志
    fn endpoint(&self) -> Option<String>;                 // 预留：SSH 已知地址
    fn check_exited(&mut self) -> Option<i32>;            // 就绪等待/监护共用
    fn just_installed(&self) -> bool;                     // download 档 → 刷新版本状态
    fn teardown(&mut self) -> Result<(), String>;         // 幂等清理
}
```

纪律：
- **零 tauri 依赖**：进度经 `BootSink` / `DownloadProgress` 回调上抛（同 updates.rs 的
  `DownloadProgress` 约定），executor 可脱离 GUI 单测。
- **跨平台 `#[cfg]` 显式**：WSL 执行器整体仅 `#[cfg(windows)]` 编译；纯解析逻辑
  （`parse_wsl_list_v` / `select_wsl2_distro` / `decode_utf16le`）以
  `#[cfg(any(windows, test))]` 跨平台可测。
- **spawn 一律经 `crate::child_cmd`**（CREATE_NO_WINDOW + cmd /C 纪律）。

## 三种环境

### Local（已完成，纯重构）
现有 resolve+shell 逻辑原样搬移：probe=system→bundle→download 解析链；
start=spawn；就绪=轮询本地日志；teardown=优雅停止（SIGTERM→SIGKILL）。
验收线：行为零变化，`cargo test` 全绿。

### WSL 迭代 v1（已实现，待 Windows 实机验证）

- **只认 WSL2**：localhostForwarding 是 Windows 侧 WebView 经 127.0.0.1 访问
  WSL 内 dsh 的前提；WSL1 无此能力 → 探测即拒绝并给 `wsl --set-version ... 2` 提示。
- **零配置**：`WslConfig{distro: None}` = 默认发行版（须 WSL2，否则落回第一个 WSL2）；
  进入入口 = `boot_in_wsl` IPC（前端顶栏按钮）。
- **客体内命令 = 固定脚本模板**，不拼接用户输入；模板先 source
  `/etc/profile` / `~/.profile` / `~/.bashrc`（登录 shell 不读 .bashrc，nvm/fnm 的
  PATH 补不上会 command not found）。
- **PATH 兼容 nvm/fnm（2026-08-26 实机修复）**：探测/启动一律 **交互式登录壳
  `bash -lic`**——Ubuntu 默认 `.bashrc` 开头有非交互守卫 `case $- in *i*) ;; *)
  return;; esac`，旧方案 `-lc`（非交互）source 时直接 return，用户的 nvm/fnm 段
  根本不执行，**装了 node/dsh 也探测不到**（朋友实机复现）。`-lic` 守卫放行；
  另有兜底扫描 nvm/fnm（含 XDG）/n/volta 安装位前置 PATH，**不依赖任何 rc 被执行**。
  探测三态：`READY`（node+dsh）/ `DSH_MISSING`（有 node 缺 dsh）/ `NODE_MISSING`。
- **缺 dsh 自动安装（2026-08-26 登记网络面）**：`DSH_MISSING` → 客体内
  `npm i -g @deepseek-ai/dsh`（`GUEST_INSTALL_DSH` 模板，输出落
  `/tmp/dsh-dock-npm.log` 只回传尾部 2KB 诊断）→ 复查 → READY 才启动；
  `just_installed` 置位刷新版本状态。缺 node（`NODE_MISSING`）不自动装 Node
  （安装方式/版本策略属用户主权），给可行动提示。
- **就绪**：`wsl.exe -e bash -lic '<模板>'` 的 stdout 转发到本地日志（wsl.exe 透传
  guest stdout），复用壳的通用日志轮询；日志里的 `127.0.0.1:<port>` 经 WSL2 转发从
  Windows 侧也通，capabilities `http://127.0.0.1:*` 零改动。
- **生命周期**：客体内 wrapper（后台 dsh + watcher 轮询 stop 标志）→ teardown 只
  `touch /tmp/dsh-dock-stop`，确定性停掉本会话 dsh，不误伤发行版内其它进程。
  （早期方案 `pkill -f 'dsh --profile web'` 有缺陷：dsh shim 最终 exec 成
  `<node> <bin.js> --profile web...`，命令行无连续子串匹配。）
- **兼容 wsl.exe 非 UTF-8 输出**：`run_wsl_capture` 取原始字节，探测 NUL 则按
  UTF-16LE 解码（vscode/tailscale 同款踩坑）。
- **边界（迭代 v2）**：WSL 内缺失 **node** 时不自动安装（只有 node 缺失时仍给可行动
  提示）；客体内安装的镜像参数不注入（尊重用户客体内 npm 配置）。

### SSH（预留，未实现）
- 配置形状已定型（`SshConfig`：host / user / port / local_port / remote_port）。
- 落地方向（后续版本）：系统 `ssh` 子进程（复用用户 `~/.ssh` / agent / known_hosts，
  不引入 russh 原生栈）+ `-N -L <本地口>:127.0.0.1:<远端口>` 隧道；就绪判定改用
  `endpoint()` + **TCP 健康探测**（SSH 无本地日志 URL）；`--port 0` 不适用（需先探远端
  端口或约定固定口）；teardown=断隧道。
- 安全边界：隧道让远端页面获得与本地 dsh 相同的 127.0.0.1 权限（`allow-terminal-action`
  等）。落地时须做**会话级 capability 收敛**（远端会话拒绝 upgrade 类动作）。

## 会话槽与代际（`lib.rs`）

- `ShellState.session: executor::Session`（`Mutex<Option<Box<dyn Executor>>>`）——等待/
  监护线程短锁轮询，退出处理器随时可取用做 teardown（**壳与 dsh 同生命周期**）。
- `ShellState.session_epoch: AtomicU64`——每次 teardown/切换递增；等待/监护线程记录
  自己启动时的代际，会话被外部切换（如 `boot_in_wsl` 停掉本地会话）后旧线程**静默**
  退出：不在 90s 后误报错误卡、不误导航/监护新会话。**probe 阶段同样受代际保护**
  （0.4.2）：probe 开始记录 epoch，完成后不一致 → 丢弃探测结果（probe 可长达分钟
  级——WSL 自动安装 dsh——期间切换环境不得残留旧会话覆盖新会话）。
- `ShellState.boot_generation: AtomicU64` + `shutting_down: AtomicBool`（2026-09-10，
  ADR-0014）：**启动代际令牌**。每次「开始一次启动」（首启 / 切换 profile / 模式切换 /
  选择器落地 / 错误卡重试 / 崩溃自动拉起）经 `begin_boot()` 领令牌，启动线程在
  probe 后、spawn 前、spawn 后落槽前、导航前逐一校验；被取代者**在 spawn 之前**
  静默退出，已 spawn 才被取代的就地 teardown。旧写法在 `launch_executor_after_probe`
  **进入函数时**才读 epoch，两次快速切换会读到同一（最新）代际而双双放行 → 双 spawn，
  后到者覆盖会话槽、把前一个 dsh 丢在地上（进程继续跑、壳再也收不回）。
- **会话槽纪律**：落新会话前 `reap_previous_session` 先 `take()` 并 teardown 旧会话
  （槽位是 `Option<Box<dyn Executor>>` 赋值，直接覆盖 = 丢进程）；`RunEvent::Exit`
  先置 `shutting_down` 再收会话，退出竞态不再 spawn 出无人认领的 dsh。
- **交接（handoff）**：`ShellState.handoff`（target/kind/phase/startedAt/generation）
  为一次重启/切换的贯穿状态，经 `get_boot_status.intent` 与 `switch_profile` 返回值
  暴露给两个窗口（同一 `startedAt` → 计时跨文档连续）。生命周期与 UI 见 ADR-0014 与
  `frontend/src/lib/handoff.ts`。

## 运行环境的用户主权（首次选择 / 默认 / 菜单切换）

local 与 wsl 在 **Windows** 上**同等地位**（`settings.rs` + `executor_for_mode`
统一入口）；**WSL 只存在于 Windows**（2026-08-26 裁定），非 Windows 机器
零 WSL 感知：

- **Windows 首次打开**：`settings.json` 无 `defaultMode` → 导航壳 SPA `/mode`（frontend/src/pages/BootMode.tsx）
  选择页（本机 / WSL2 + 「设为默认」勾选）→ `index.html?mode=…&default=…` →
  `choose_mode` IPC 落地（写默认可选）并启动；设过默认则跳过直接按默认启动。
- **非 Windows 首次打开**：不出选择页——本机是唯一环境，直接按 local 启动；
  settings 残留 `wsl`（拷自 Windows 的数据目录）也强制 local。
- **默认持久化**：`<app_data>/settings.json` 仅 `defaultMode` 一个字段（AGENTS
  §6「运行时无状态」登记的**最小例外**；原子写 tmp+rename，损坏回退默认；
  2026-08-27 边界重定义后此例外仅约束运行时，管理功能持久化不在此限）。
- **菜单切换（即记默认，仅 Windows）**：托盘「打开方式」两项——当前模式带 ✓；
  选中 → `switch_mode`：teardown 旧会话 → 写默认 → 导航回启动页 → 按新模式启动
  （就绪后自动导航到工作台；probe 失败则错误卡在启动页可见）。macOS / Linux
  无此入口（本地是唯一环境）。
- **retry 修正**：错误卡重试按 `active_mode` 重建执行器——WSL 会话出错重试不回退 local。
- 顶栏「在 WSL 中打开」（`boot_in_wsl`，仅 Windows 渲染）保留，语义与菜单一致
  （切换+记默认）；非 Windows 调用该命令 / `choose_mode(wsl)` 均防御性拒绝
  （不 teardown、不写脏默认）。

## 验证状态

- macOS 主机编译 + `x86_64-pc-windows-gnu` 交叉编译 + `cargo test` 全绿（285 tests，
  2026-09-10；含 ADR-0015 子进程生命周期与 v1.1.0 Windows 修复两批）。
- **Windows 分叉的 lint 怎么在本地看见（2026-09-11 实证，无 mingw 也能跑）**：
  宿主 clippy 全绿**不蕴含** Windows 全绿——`#[cfg(windows)]` 函数体只有 windows 目标
  clippy 看得见（本批 `guest::write_home_files` 的未使用变量就是这样漏到 CI 才红，复盘见
  `docs/broadcasts.md` 2026-09-11 补记）。缺 mingw 时可用**桩 C 工具链**绕过依赖的 C 编译
  （clippy 是 check-only、不链接，假 `.o`/`.a` 不影响判据）：
  ```bash
  mkdir -p /tmp/fakebin   # 一个脚本：解析参数里的 -o 就 touch 该路径，然后 exit 0
  for n in gcc ar windres; do ln -sf /tmp/fakebin/touch-out /tmp/fakebin/x86_64-w64-mingw32-$n; done
  PATH=/tmp/fakebin:$PATH \
  CC_x86_64_pc_windows_gnu=/tmp/fakebin/x86_64-w64-mingw32-gcc \
  AR_x86_64_pc_windows_gnu=/tmp/fakebin/x86_64-w64-mingw32-ar \
  cargo clippy --target x86_64-pc-windows-gnu --all-targets -- -D warnings
  ```
  边界（如实）：只验 Rust 侧编译与 lint（含测试目标）；链接与运行仍只在 CI / 真机。
- **Windows 实机未验**：WSL 运行时行为（`wsl -l -v` 实机输出、localhost 转发、
  stop 标志 teardown、rc source）需按 ADR-0004 的执行要求验证。
  shell.log / dsh-wsl.log 位于 `%APPDATA%\io.github.realguan.dsh-dock\`。
- **交接/进程树实机清单待排期**（ADR-0014 §5）：① macOS 重启全程"一条 loading 贯穿"
  （控制中心导轨 ↔ 主窗口启动屏 ↔ 工作台，计时不归零）；② Windows `taskkill /T`
  收口——重启后 `tasklist | findstr node` 不得残留旧 dsh（旧实现只杀 `cmd.exe` 壳层）；
  ③ 双击重启 / 重启与模式切换撞车，只允许一个新 dsh 起来；④ WSL 客体重启后
  `ps aux | grep dsh` 无残留。

### Windows 实机验证清单 · v1.1.0 实测问题修复（2026-09-10，**待跑**）

> 背景与逐项根因见 `docs/known-issues/v110-测试问题定位.md`（本地台账，不入库）。
> 维护者只有 macOS 实机，本节是**照着粘贴即可**的 Windows 验收流程。
> **重要**：Windows CI runner 以管理员上下文运行，符号链接特权天然可用——
> 这条路径**只有普通权限实机能暴露**，CI 结构性覆盖不到（见 §"为什么 CI 没拦住"）。

**前置（一次性）**：Rust stable + Git for Windows（提供 bash/curl）+ node。
在仓库根用 **Git Bash** 执行：

```bash
scripts/fetch-pnpm-bundle.sh win32-x64 linux-x64   # 宿主份 + WSL 客体份
```

**A. 引擎引导免符号链接（对应实测 1.2 / 1.4 / 1.7，最关键的根因）**

务必在**未开启开发者模式、非管理员**的账户下跑（这是能复现旧 bug 的唯一条件）：

```bash
cd src-tauri
cargo test --lib engine_bootstrap_uses_symlink_free_layout -- --ignored --nocapture
```

- 期望：`ok`，且过程中 `%TEMP%` 下新建的引擎目录里 `node_modules\node` 是**真实目录**
  （不是 symlink），`pnpm-workspace.yaml` 含 `nodeLinker: hoisted`。
- 旧实现（未修）在此环境会以 `Failed to create symlink … 拒绝访问 (os error 5)` 失败。
- **注意**：该用例在缺少捆绑 pnpm 时会**硬失败**并提示 fetch 命令（刻意不静默跳过，
  防"绿了但什么都没测"）。

**B. 真机启动（覆盖 1.3 白窗 / 1.6 死端口 / 2.0 落位——这三条只有真机能验）**

```bash
cd frontend && pnpm install --frozen-lockfile && pnpm run build && cd ..
cd src-tauri && cargo tauri build        # ← 必须 release
```

> [!IMPORTANT]
> **B 必须用 release 构建，不能用 `cargo build`（debug）**：debug 构建的
> `cfg!(dev)` 为真，`shell_app_url()` 按设计返回 dev 服务器地址、`WebviewUrl::App`
> 也指向 `devUrl`——**1.6 的修复在 debug 下根本不走那条分支**，且不启 Vite 时页面
> 本身就是连不上的（那样"复现"到的是 dev 语义，不是 bug）。只有 release 才真正
> 检验"正式包不得落到 dev 端口"。
> （A 的引擎引导用例是直接函数调用，debug/release 皆可。）

装好后逐项：

| # | 操作 | 期望 | 覆盖 |
|:--|:---|:---|:---|
| B1 | 首启（干净数据目录） | 引导完成后健康大盘**三卡全"就绪"**；关于页 DSH 有版本号（非"未检出"） | 1.2 / 1.4 / 1.7 |
| B2 | 从**工作台悬浮胶囊**点「控制中心」 | 1s 内出界面、**可关闭**、不卡死（托盘点同一入口本就正常） | 1.3 |
| B3 | 切 profile / 切运行模式 / 重启 | 全程**不出现** `localhost 拒绝连接`（旧版会闪 ERR_CONNECTION_REFUSED） | 1.6 |
| B4 | 任务管理器**强制结束**应用 → 重新启动 | 不再卡「引擎引导失败：落位 …engines\bin\pnpm.exe」 | 2.0 |
| B5 | 关于页 Node 行 | 未装时显示「未安装（计划 vX）」，**不得**把计划版本显示成已装 | 1.2 追问 |
| B6 | WSL 模式引导提示 | 版本号形如 `node v24.18.0`，**不得**出现 `vv24.18.0` | 1.1a |
| B7 | 点「查看启动详情」后 | **有收起出口**（旧版在 WSL 路径下点了就回不去） | 1.1b |

**C. 孤儿收口（ADR-0015 硬杀路径，Windows 侧走 Job Object）**

```bash
# 应用运行中（dsh 已起），用任务管理器强制结束 DSH Dock
tasklist | findstr /i "node dsh"      # 期望：无残留 dsh/node 进程
```

- 期望：壳被强杀后其 dsh **随之消失**（Job Object `KILL_ON_JOB_CLOSE`，内核保证）。
- 若仍有残留：查 `%APPDATA%\io.github.realguan.dsh-dock\shell.log` 的
  `AssignProcessToJobObject 失败` 警告（属 ADR-0015 §5 已登记边界，会降级为
  启动期清扫兜底），并把日志附回。

**D. 回归面**：WSL 模式按 ADR-0004 既有清单（上文）。

> 跑完请把 B1–B7 的实际结果与任何 `shell.log` 异常回填到本节的表里（或广播），
> 未跑项保持"待跑"——**不许把没跑过的写成已验证**。
