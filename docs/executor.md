# 执行环境抽象（executor）——local / wsl（ssh 预留）

> 状态：本地（Local）重构完成；WSL 迭代 v1 已实现（Windows 实机验证清单见
> [wsl-verification.md](wsl-verification.md)）；SSH 仅预留配置形状，未实现。

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
- **就绪**：`wsl.exe -e bash -lc '<模板>'` 的 stdout 转发到本地日志（wsl.exe 透传
  guest stdout），复用壳的通用日志轮询；日志里的 `127.0.0.1:<port>` 经 WSL2 转发从
  Windows 侧也通，capabilities `http://127.0.0.1:*` 零改动。
- **生命周期**：客体内 wrapper（后台 dsh + watcher 轮询 stop 标志）→ teardown 只
  `touch /tmp/dsh-dock-stop`，确定性停掉本会话 dsh，不误伤发行版内其它进程。
  （早期方案 `pkill -f 'dsh --profile web'` 有缺陷：dsh shim 最终 exec 成
  `<node> <bin.js> --profile web...`，命令行无连续子串匹配。）
- **兼容 wsl.exe 非 UTF-8 输出**：`run_wsl_capture` 取原始字节，探测 NUL 则按
  UTF-16LE 解码（vscode/tailscale 同款踩坑）。
- **边界（迭代 v2）**：WSL 内缺失 node/dsh 时**不自动安装**，给
  `npm i -g @deepseek-ai/dsh` 可行动提示（自动补齐会触网，按 AGENTS 网络面登记后再做）。

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
  退出：不在 90s 后误报错误卡、不误导航/监护新会话。

## 验证状态

- macOS 主机编译 + `x86_64-pc-windows-gnu` 交叉编译 + `cargo test` 全绿（74 tests）。
- **Windows 实机未验**：WSL 运行时行为（`wsl -l -v` 实机输出、localhost 转发、
  stop 标志 teardown、rc source）需按 [wsl-verification.md](wsl-verification.md) 验证。
  shell.log / dsh-wsl.log 位于 `%APPDATA%\io.github.realguan.dsh-dock\`。
