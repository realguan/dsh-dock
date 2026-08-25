//! executor.rs —— 执行环境抽象（壳只认识这个接口）。
//!
//! 定位：local（本机子进程）/ wsl（WSL2 发行版）/ ssh（预留）三者在壳眼里
//! 都是"一次把工作台端点交给壳的会话"。壳的主流程
//! （probe → start → 就绪 → 监护 → teardown）只依赖本 trait，不感知具体
//! 环境；环境差异封装在各 Executor 实现里。对应 docs/ 待补的设计：执行环境
//! 也是数据（壳是通用机制，产物是关于执行器的配置）。
//!
//! 纪律：
//!   - 跨平台语义用 `#[cfg]` 显式（AGENTS.md）：WSL 专属动作在非 Windows 上
//!     编译为明确报错（"仅支持 Windows 平台"），纯解析逻辑（wsl -l -v 行解析）
//!     在所有平台编译并可单测；
//!   - 零 tauri 依赖：进度经 BootSink 回调上抛（同 updates.rs 的
//!     DownloadProgress 约定），executor 可脱离 GUI 单测；
//!   - 新增 spawn 一律经 `crate::child_cmd`（CREATE_NO_WINDOW + cmd /C 纪律）。
//!
//! SSH 为预留：SshConfig 形状先定型（本期不实现），执行逻辑留后续版本。

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use crate::manifest::ProductManifest;
use crate::resolve::LaunchSpec;

#[cfg(windows)]
use std::process::Command;

// ---------- 共享类型 ----------

/// 环境种类：壳侧展示 / 遥测用。
/// `Wsl` 在 Windows 构建中被 WslExecutor 构造；`Ssh` 为预留变体（全平台未构造）。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutorKind {
    Local,
    Wsl,
    Ssh,
}

impl ExecutorKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutorKind::Local => "local",
            ExecutorKind::Wsl => "wsl",
            ExecutorKind::Ssh => "ssh",
        }
    }
}

/// 启动/探测阶段的进度回传（step, state, detail）——与 ui `boot:step` 协议同构。
pub type BootSink<'a> = &'a mut dyn FnMut(usize, &str, &str);

/// 下载字节进度回调（与 updates.rs 的 DownloadProgress 同构；保持零 tauri 依赖）。
/// 本机 download 档补齐 Node/dsh 时经它上抛字节进度（ui `boot:progress`）。
pub type DownloadProgress<'a> = &'a mut dyn FnMut(u64, Option<u64>);

/// `probe` 之后的状态：可直接 `start`，或需要用户先选 profile（F-b 选择器）。
pub enum ProbeOutcome {
    Ready,
    NeedsProfile(Vec<String>),
}

/// 一次"把工作台端点交给壳"的环境会话。
///
/// 生命周期契约：`probe` → （必要时 `select_profile`）→ `start` →
/// 壳轮询 `log_path` + `check_exited` 判就绪 → 监护 `check_exited` →
/// `teardown`（壳退 / 错误卡重试均走这里）。
pub trait Executor: Send {
    fn kind(&self) -> ExecutorKind;

    /// 探测环境可用性（步 0-1）；失败返回带域名的可行动错误（壳转错误卡）。
    /// `progress`：本机 download 档补齐运行时的字节进度（其余环境忽略）。
    fn probe(
        &mut self,
        sink: BootSink<'_>,
        progress: DownloadProgress<'_>,
    ) -> Result<ProbeOutcome, String>;

    /// F-b：用户从选择器选定的 profile（仅 `NeedsProfile` 之后调用）。
    fn select_profile(&mut self, profile: String);

    /// 补齐运行时并启动 dsh（步 2）；返回后可轮询就绪。`log_path()` 下的日志
    /// 由壳统一等待/监护（同生命周期）。
    fn start(&mut self, sink: BootSink<'_>) -> Result<(), String>;

    /// 就绪判定要轮询的**本地**日志路径（URL 从该文件解析）。WSL 经 wsl.exe
    /// 把用户态 stdout/stderr 转发到本地文件；SSH 无此来源，见 `endpoint`。
    fn log_path(&self) -> PathBuf;

    /// 预留：无日志 URL 来源的环境直接给出已知地址（如 SSH 隧道的本地映射口）。
    /// SSH 未实现前无人消费，标记 dead_code（保留为 trait 契约面）。
    #[allow(dead_code)]
    fn endpoint(&self) -> Option<String> {
        None
    }

    /// 会话进程是否已退出：`Some(code)` = 已退出（就绪等待与监护共用）。
    fn check_exited(&mut self) -> Option<i32>;

    /// 本次 probe 是否刚补齐过运行时（本地 download 档）→ 壳刷新版本状态。
    fn just_installed(&self) -> bool {
        false
    }

    /// 清理：停子进程 / 断隧道。壳退与错误卡重试均走这里，必须幂等。
    fn teardown(&mut self) -> Result<(), String>;
}

/// 会话槽：ShellState 里持有一个执行器（等待/监护线程内被短锁轮询）。
pub type Session = Mutex<Option<Box<dyn Executor>>>;

// ---------- 配置形状（SSH 预留：先定型形状，不实现执行） ----------

/// WSL 会话配置。v1 零配置：`None` = 使用 WSL 默认发行版。
/// `distro` 仅在 Windows 运行时读取（WSL 执行器仅 Windows 编译），
/// macOS/Linux 上仅作配置形状存在，故标记 dead_code。
#[derive(Debug, Clone)]
pub struct WslConfig {
    #[allow(dead_code)]
    pub distro: Option<String>,
}

/// 预留：SSH 连接配置。执行逻辑留后续版本。
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SshConfig {
    pub host: String,
    pub user: String,
    pub port: u16,
    pub local_port: u16,
    pub remote_port: u16,
}

/// 本次会话的执行环境。v1：启动默认 `Local`，经 `boot_in_wsl` IPC 切换为 `Wsl`。
/// SSH 为预留形状（`Ssh(SshConfig)`）；当前未接到会话路径上（boot 路径按各
/// 命令硬编码构建具体执行器），故整枚举暂标记 dead_code——它是"执行环境也是
/// 数据"的落点，后续会话配置子系统接入后再真正驱动。
#[allow(dead_code)]
pub enum ExecutionMode {
    Local,
    Wsl(WslConfig),
    #[allow(dead_code)]
    Ssh(SshConfig),
}

// ---------- LocalExecutor：本机子进程（原 resolve + shell 路径的原样封装） ----------

/// 本机执行器：system → bundle → download 宿主解析链 + 本地子进程。
/// 这是现有本地行为的**原样搬移**（纯重构）：probe=解析链；start=spawn；
/// 就绪=轮询本地日志；清理=优雅停止。
pub struct LocalExecutor {
    manifest: ProductManifest,
    resources_dir: PathBuf,
    data_dir: PathBuf,
    path_env: String,
    launch: Option<LaunchSpec>,
    proc: Option<crate::shell::DshProcess>,
}

impl LocalExecutor {
    pub fn new(manifest: ProductManifest, resources_dir: PathBuf, data_dir: PathBuf) -> Self {
        Self {
            manifest,
            resources_dir,
            data_dir,
            path_env: crate::resolve::effective_path(),
            launch: None,
            proc: None,
        }
    }
}

impl Executor for LocalExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::Local
    }

    fn probe(
        &mut self,
        sink: BootSink<'_>,
        progress: DownloadProgress<'_>,
    ) -> Result<ProbeOutcome, String> {
        sink(0, "running", "扫描用户环境（PATH · 版本闸）");
        sink(1, "running", "解析宿主档位");
        let launch = crate::resolve::resolve_launch(
            &self.manifest,
            &self.resources_dir,
            &self.path_env,
            &self.data_dir,
            progress,
        )
        .map_err(|e| {
            sink(1, "error", &e.to_string());
            e.to_string()
        })?;
        sink(0, "done", "环境扫描完成");
        sink(1, "done", &format!("命中档位：{:?}", launch.tier));
        // F-b：system 档且用户世界有多个 webUi profile → 由壳出选择器。
        let home = crate::resolve::user_dsh_home();
        let profiles = crate::resolve::list_web_ui_profiles(&home);
        let needs_selector =
            launch.tier == crate::manifest::TierKind::System && profiles.len() > 1;
        self.launch = Some(launch);
        Ok(if needs_selector {
            ProbeOutcome::NeedsProfile(profiles)
        } else {
            ProbeOutcome::Ready
        })
    }

    fn select_profile(&mut self, profile: String) {
        if let Some(l) = self.launch.as_mut() {
            l.profile = profile;
        }
    }

    fn start(&mut self, sink: BootSink<'_>) -> Result<(), String> {
        let launch = self
            .launch
            .clone()
            .ok_or_else(|| "无启动规格（请先探测环境）".to_string())?;
        sink(
            2,
            "running",
            &format!("spawn DSH（{} · tier={:?}）", launch.profile, launch.tier),
        );
        let dsh = crate::shell::spawn_dsh(&launch, &self.data_dir).map_err(|e| e.to_string())?;
        self.proc = Some(dsh);
        Ok(())
    }

    fn log_path(&self) -> PathBuf {
        // 单一来源：spawn_dsh 落盘的日志路径（DshProcess.log_path），未 spawn 前兜底。
        self.proc
            .as_ref()
            .map(|p| p.log_path.clone())
            .unwrap_or_else(|| self.data_dir.join("dsh-shell.log"))
    }

    fn check_exited(&mut self) -> Option<i32> {
        self.proc
            .as_mut()
            .and_then(|p| p.child.try_wait().ok())
            .flatten()
            .map(|s| s.code().unwrap_or(-1))
    }

    fn just_installed(&self) -> bool {
        self.launch
            .as_ref()
            .map(|l| l.tier == crate::manifest::TierKind::Download)
            .unwrap_or(false)
    }

    fn teardown(&mut self) -> Result<(), String> {
        if let Some(mut p) = self.proc.take() {
            crate::shell::stop_dsh(&mut p.child, Duration::from_secs(3));
        }
        Ok(())
    }
}

// ---------- WSL 支持（迭代 v1：WSL2 发行版内跑 dsh，零配置） ----------
//
// 语义要点：
//   - **只认 WSL2**：localhost 自动转发（localhostForwarding）是 Windows 侧
//     WebView 经 127.0.0.1 访问 WSL 内 dsh 的前提；WSL1 无此能力，探测即拒绝
//     并给可行动提示。
//   - 客体内命令 = **固定脚本模板**，不拼接用户输入（防注入/转义地狱）；
//     模板先 source 常见 rc（登录 shell 不读 ~/.bashrc，nvm/fnm 的 PATH 补不上
//     会 command not found），再启动/探测。
//   - 就绪判定复用壳的通用日志轮询：wsl.exe 会把用户态 stdout/stderr 转发到
//     Windows 侧，重定向到本地日志即可（`--port 0` 由 WSL 内分配，WSL2 的
//     localhostForwarding 让日志里的 127.0.0.1:<port> 从 Windows 侧也通）。
//   - 生命周期：客体内用一个 **stop 标志文件**控制 wrapper——dsh 崩溃 wrapper
//     随 `wait` 退出（wsl.exe 子进程=会话存活代理）；teardown 只 touch 标志文件，
//     确定性地停掉本会话的 dsh，不误伤发行版内其它进程（比 pkill 模式可靠，
//     因为 dsh shim 最终 exec 成 `<node> <bin.js> --profile web ...`，命令行里
//     并不含 "dsh --profile web" 连续子串）。

/// `wsl -l -v` 输出的一个发行版行。
/// Windows 运行时 + 全平台测试编译（跨平台可测的纯解析）；非 Windows 的
/// 非测试构建裁剪（见 `parse_wsl_list_v`）。
#[cfg(any(windows, test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WslDistro {
    pub name: String,
    pub is_default: bool,
    pub version: Option<u32>, // WSL1=1 / WSL2=2；解析失败=None
}

/// 解析 `wsl -l -v` 输出：`*` 标记默认发行版，末列是版本号。
/// 纯函数，任何平台可单测（wsl.exe 老版本可能输出 UTF-16 或本地化表头，
/// 这里只认结构化部分，不碰表头文案）。
#[cfg(any(windows, test))]
pub fn parse_wsl_list_v(raw: &str) -> Vec<WslDistro> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // 跳表头：首行含 NAME 且含 STATE/VERSION（只对首行容错，数据行不动）。
        let lower = trimmed.to_ascii_lowercase();
        if out.is_empty() && lower.contains("name") {
            continue;
        }
        let mut rest = trimmed;
        let is_default = rest.starts_with('*');
        if is_default {
            rest = rest[1..].trim_start();
        }
        let mut parts = rest.split_whitespace();
        if let (Some(name), Some(_state)) = (parts.next(), parts.next()) {
            let version = parts.next().and_then(|v| v.parse::<u32>().ok());
            // 只要版本列可解析的行——真实发行版必有 VERSION（1/2）；
            // 横幅等杂物（无版本号）直接丢弃，避免混进发行版列表。
            if let Some(version) = version {
                out.push(WslDistro {
                    name: name.to_string(),
                    is_default,
                    version: Some(version),
                });
            }
        }
    }
    out
}

/// 目标发行版选择 + WSL2 门禁：显式 distro 必须存在且为 WSL2；未指定时选
/// 默认发行版（须 WSL2），否则退到第一个 WSL2。
#[cfg(any(windows, test))]
fn select_wsl2_distro(cfg_distro: &Option<String>, distros: &[WslDistro]) -> Option<String> {
    if let Some(name) = cfg_distro {
        return distros
            .iter()
            .find(|d| &d.name == name)
            .filter(|d| d.version == Some(2))
            .map(|d| d.name.clone());
    }
    distros
        .iter()
        .find(|d| d.is_default && d.version == Some(2))
        .or_else(|| distros.iter().find(|d| d.version == Some(2)))
        .map(|d| d.name.clone())
}

/// 枚举 WSL 发行版状态。Windows 专属（WSL 执行器整体仅 Windows 编译；
/// 跨平台可测的纯解析见 `parse_wsl_list_v` / `select_wsl2_distro`）。
#[cfg(windows)]
pub fn wsl_distros() -> Result<Vec<WslDistro>, String> {
    let raw = run_wsl_capture(None, &["-l", "-v"])
        .ok_or_else(|| "wsl.exe 不可用或调用失败（wsl.exe 应在 PATH / System32）".to_string())?;
    Ok(parse_wsl_list_v(&raw))
}

/// 构造 `wsl.exe` 命令（可选目标发行版）。Windows 专属。
#[cfg(windows)]
fn wsl_command(distro: Option<&str>) -> Command {
    let mut cmd = crate::child_cmd(std::path::Path::new("wsl.exe"));
    if let Some(d) = distro {
        cmd.arg("-d").arg(d);
    }
    cmd
}

/// 在 WSL 内执行并捕获 stdout（带超时）。
/// 兼容 wsl.exe 的 UTF-16 重定向输出（老版本/非英语区域实测）：取原始字节后
/// 探测 NUL（UTF-16LE 特征），命中就按 UTF-16LE 解码。
#[cfg(windows)]
fn run_wsl_capture(distro: Option<&str>, args: &[&str]) -> Option<String> {
    let mut cmd = wsl_command(distro);
    cmd.args(args);
    let raw = crate::resolve::run_with_timeout_raw(&mut cmd, Duration::from_secs(5))?;
    if raw.iter().any(|&b| b == 0) {
        return decode_utf16le(&raw);
    }
    let t = String::from_utf8_lossy(&raw).trim().to_string();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

/// 按 UTF-16LE（含小端 BOM 或不含）解码 wsl.exe 的 stdout 字节。
#[cfg(any(windows, test))]
fn decode_utf16le(bytes: &[u8]) -> Option<String> {
    let mut bytes = bytes;
    if bytes.starts_with(&[0xFF, 0xFE]) {
        bytes = &bytes[2..];
    }
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let mut s = String::from_utf16(&units).ok()?;
    while s.ends_with('\u{0}') {
        s.pop();
    }
    Some(s)
}

/// 客体内停止标志文件：teardown touch 它 → wrapper 收到后 kill dsh 并退出。
#[cfg(windows)]
const GUEST_STOP_FILE: &str = "/tmp/dsh-dock-stop";

/// 客体内启动 dsh 的固定脚本模板（不插值用户输入；迭代 v1 固定 boot web profile）。
/// 结构：source rc → 后台起 dsh → 起 watcher（轮询 stop 标志）→ wait dsh →
/// 退出码回传。dsh 崩溃或收到 stop 都让本 wrapper 退出，wsl.exe 子进程随之退出
/// （= 会话存活代理，check_exited 可用）。
#[cfg(windows)]
const GUEST_BOOT: &str = concat!(
    "rm -f /tmp/dsh-dock-stop; . /etc/profile 2>/dev/null;",
    ". \"$HOME/.profile\" 2>/dev/null; . \"$HOME/.bashrc\" 2>/dev/null;",
    "cd \"$HOME\"; dsh --profile web --port 0 --no-open & PID=$!;",
    "(while [ ! -f /tmp/dsh-dock-stop ]; do sleep 1; done; kill -TERM \"$PID\" 2>/dev/null) & WATCH=$!;",
    "wait \"$PID\"; RC=$?; kill \"$WATCH\" 2>/dev/null; rm -f /tmp/dsh-dock-stop; exit $RC",
);

/// 客体内探测 node + dsh 的固定脚本模板（先 source rc 补 PATH）。
#[cfg(windows)]
const GUEST_PROBE: &str =
    ". /etc/profile 2>/dev/null; . \"$HOME/.profile\" 2>/dev/null; . \"$HOME/.bashrc\" 2>/dev/null;\
     command -v node >/dev/null 2>&1 && command -v dsh >/dev/null 2>&1 && echo READY || echo MISSING";

/// WSL2 执行器（迭代 v1：WSL2 发行版内跑 dsh，零配置）。Windows 专属。
#[cfg(windows)]
pub struct WslExecutor {
    cfg: WslConfig,
    data_dir: PathBuf,
    selected: Option<String>, // probe 选定的发行版名
    child: Option<std::process::Child>,
    log_path: PathBuf,
}

#[cfg(windows)]
impl WslExecutor {
    pub fn new(cfg: WslConfig, data_dir: PathBuf) -> Self {
        Self {
            cfg,
            data_dir: data_dir.clone(),
            selected: None,
            child: None,
            log_path: data_dir.join("dsh-wsl.log"),
        }
    }
}

#[cfg(windows)]
impl Executor for WslExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::Wsl
    }

    fn probe(
        &mut self,
        sink: BootSink<'_>,
        progress: DownloadProgress<'_>,
    ) -> Result<ProbeOutcome, String> {
        let _ = progress; // WSL 迭代 v1 不在客体内下载，无字节进度
        sink(0, "running", "探测 WSL（wsl.exe · WSL2 发行版）");
        let distros = wsl_distros()?;
        if distros.is_empty() {
            sink(0, "error", "未检测到 WSL 发行版");
            return Err(
                "未检测到 WSL 发行版。请先在 Windows 安装 WSL2 并装一个发行版（`wsl --install`）。"
                    .to_string(),
            );
        }
        if let Some(name) = self.cfg.distro.as_deref() {
            if !distros.iter().any(|d| d.name == name) {
                sink(0, "error", "指定的发行版不存在");
                let avail = distros
                    .iter()
                    .map(|d| d.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(format!("发行版 {name} 不存在；本机可用：{avail}"));
            }
        }
        let target = select_wsl2_distro(&self.cfg.distro, &distros).ok_or_else(|| {
            sink(0, "error", "无可用 WSL2 发行版");
            "WSL 模式需要 WSL2 发行版（WSL1 不具备 localhost 端口转发，Windows 侧无法直达）。\
             请升级到 WSL2：`wsl --set-version <发行版> 2`。"
                .to_string()
        })?;
        sink(1, "running", &format!("探测 {target} 内的 node / dsh"));
        match probe_guest_in_distro(&target) {
            Ok(true) => {
                sink(0, "done", "WSL2 环境就绪");
                sink(1, "done", &format!("{target} 内发现 dsh 与 node"));
                self.selected = Some(target);
                Ok(ProbeOutcome::Ready)
            }
            Ok(false) => {
                sink(1, "error", "发行版内缺少 node 或 dsh");
                Err(format!(
                    "{target} 内缺少可用的 node 或 dsh。请在 WSL 内执行 `npm i -g @deepseek-ai/dsh` 后重试。"
                ))
            }
            Err(e) => {
                sink(1, "error", &e);
                Err(e)
            }
        }
    }

    /// 迭代 v1：WSL 内固定 boot web profile（选择器留后续版本）。
    fn select_profile(&mut self, _profile: String) {}

    fn start(&mut self, sink: BootSink<'_>) -> Result<(), String> {
        let target = self
            .selected
            .clone()
            .ok_or_else(|| "未探测 WSL 发行版（请先 probe）".to_string())?;
        sink(2, "running", &format!("在 WSL（{target}）中启动 DSH"));
        std::fs::create_dir_all(&self.data_dir).map_err(|e| e.to_string())?;
        let log = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&self.log_path)
            .map_err(|e| format!("打开日志 {} 失败：{e}", self.log_path.display()))?;
        let mut cmd = wsl_command(Some(&target));
        cmd.arg("-e")
            .arg("bash")
            .arg("-lc")
            .arg(GUEST_BOOT)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::from(
                log.try_clone().map_err(|e| e.to_string())?,
            ))
            .stderr(std::process::Stdio::from(log));
        let child = cmd.spawn().map_err(|e| format!("spawn wsl.exe 失败：{e}"))?;
        self.child = Some(child);
        Ok(())
    }

    fn log_path(&self) -> PathBuf {
        self.log_path.clone()
    }

    fn check_exited(&mut self) -> Option<i32> {
        self.child
            .as_mut()
            .and_then(|c| c.try_wait().ok())
            .flatten()
            .map(|s| s.code().unwrap_or(-1))
    }

    fn teardown(&mut self) -> Result<(), String> {
        // 1) 通知客体内 wrapper 停 dsh：touch stop 标志 → wrapper kill dsh → 退出。
        //    wsl.exe 子进程随之退出（会话存活代理）。只动标志文件，不影响发行版
        //    内其它进程。注意 /tmp/dsh-dock-stop 须与 GUEST_BOOT 内字面量一致。
        if let Some(target) = &self.selected {
            let script = format!("touch {GUEST_STOP_FILE}");
            let _ = run_wsl_capture(Some(target), &["-e", "sh", "-lc", &script]);
        }
        // 2) 等 wsl.exe 退出（grace），兜底 kill。若客体内 wrapper 未退出
        //    （异常），kill 掉 wsl.exe 后客体内进程可能残留——见 GUEST_BOOT 注释，
        //    TODO(Windows 实机) 验证后决定是否需要 `wsl --terminate` 兜底。
        if let Some(child) = self.child.as_mut() {
            let deadline = std::time::Instant::now() + Duration::from_secs(3);
            loop {
                if let Some(_status) = child.try_wait().ok().flatten() {
                    break;
                }
                if std::time::Instant::now() >= deadline {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        Ok(())
    }
}

/// 在指定发行版内探测 node+dsh（固定脚本模板；先 source rc 补 PATH）。Windows 专属。
#[cfg(windows)]
fn probe_guest_in_distro(target: &str) -> Result<bool, String> {
    let out = run_wsl_capture(Some(target), &["-e", "bash", "-lc", GUEST_PROBE])
        .ok_or_else(|| format!("{target} 内探测命令不可用（wsl.exe 调用失败）"))?;
    Ok(out.contains("READY"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wsl_list_v_with_default_marker() {
        let raw = "\
  NAME            STATE           VERSION
* Ubuntu-24.04    Running         2
  Debian          Stopped         1
  OracleLinux_8_9 Stopped         2
";
        let distros = parse_wsl_list_v(raw);
        assert_eq!(distros.len(), 3);
        assert_eq!(distros[0].name, "Ubuntu-24.04");
        assert!(distros[0].is_default);
        assert_eq!(distros[0].version, Some(2));
        assert_eq!(distros[1].name, "Debian");
        assert!(!distros[1].is_default);
        assert_eq!(distros[1].version, Some(1));
        assert_eq!(distros[2].version, Some(2));
    }

    #[test]
    fn parses_wsl_list_v_without_header_when_empty() {
        // 空输出 / 没有表头 → 空列表，不 panic
        assert!(parse_wsl_list_v("").is_empty());
        assert!(parse_wsl_list_v("   ").is_empty());
    }

    #[test]
    fn selects_wsl2_default_preferring_marked_default() {
        let distros = parse_wsl_list_v(
            "* WindowsLegacy Kernel   Running         1\n  Ubuntu-24.04  Running   2",
        );
        // 标记的默认是 WSL1 → 不应选它，落回第一个 WSL2
        let picked = select_wsl2_distro(&None, &distros);
        assert_eq!(picked.as_deref(), Some("Ubuntu-24.04"));
    }

    #[test]
    fn selects_explicit_distro_must_be_wsl2() {
        let distros = parse_wsl_list_v("* Ubuntu-24.04  Running   2\n  Debian  Stopped  1");
        assert_eq!(select_wsl2_distro(&Some("Debian".into()), &distros), None);
        assert_eq!(
            select_wsl2_distro(&Some("Ubuntu-24.04".into()), &distros),
            Some("Ubuntu-24.04".to_string())
        );
        assert_eq!(select_wsl2_distro(&Some("Nope".into()), &distros), None);
    }

    #[test]
    fn executor_kind_labels() {
        assert_eq!(ExecutorKind::Local.as_str(), "local");
        assert_eq!(ExecutorKind::Wsl.as_str(), "wsl");
        assert_eq!(ExecutorKind::Ssh.as_str(), "ssh");
    }

    #[test]
    fn decode_utf16le_handles_bom_and_trailing_nul() {
        // wsl.exe 重定向输出可能为 UTF-16LE（含 BOM + 结尾 NUL）。
        let text = "Ubuntu-24.04\r\n";
        let mut bytes = Vec::from(&[0xFF, 0xFEu8][..]);
        for u in text.encode_utf16() {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        bytes.push(0);
        bytes.push(0);
        assert_eq!(decode_utf16le(&bytes).as_deref(), Some("Ubuntu-24.04\r\n"));
    }

    #[test]
    fn parses_wsl_list_v_drops_banner_lines() {
        // 老版本 `-l` 会打横幅行（无版本列）——应被丢，不影响真实发行版。
        let raw = "\
Windows Subsystem for Linux Distributions:
  NAME            STATE           VERSION
* Ubuntu-24.04    Running         2
";
        let distros = parse_wsl_list_v(raw);
        assert_eq!(distros.len(), 1);
        assert_eq!(distros[0].name, "Ubuntu-24.04");
        assert_eq!(distros[0].version, Some(2));
    }
}
