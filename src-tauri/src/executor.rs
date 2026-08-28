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

    /// 当前会话已启动的 profile（运行中防护比对源，ADR-0009 §2：删除/重命名
    /// 前比对壳当前 launch.profile）。默认 None；本地档取 launch（select 后即
    /// 真值）；WSL 档 GUEST_BOOT 写死 web（4.9 放开多 profile 时同步本方法）。
    fn active_profile(&self) -> Option<&str> {
        None
    }

    /// 补齐运行时并启动 dsh（步 2）；返回后可轮询就绪。`log_path()` 下的日志
    /// 由壳统一等待/监护（同生命周期）。
    fn start(&mut self, sink: BootSink<'_>) -> Result<(), String>;

    /// 就绪判定要轮询的**本地**日志路径（URL 从该文件解析）。WSL 经 wsl.exe
    /// 把用户态 stdout/stderr 转发到本地文件；SSH 无此来源，见 `endpoint`。
    fn log_path(&self) -> PathBuf;

    /// 就绪标记读取器（**WSL 用**，绕开 wsl.exe 输出缓冲导致的就绪误判）：
    /// 返回 Some(text) = 当前 marker 内容（可能为空字符串）；None = 不支持 /
    /// 不可读。本地默认 None 走 log 路径；WSL 实现在 GUEST_BOOT 把 dsh 输出
    /// tee 到客体内哨兵文件，shell 经 wsl.exe -e cat 直读（绕开 wsl.exe 转发
    /// stdout 时的内部缓冲）。详见 docs/executor.md。
    fn read_ready_marker(&mut self) -> Option<String> {
        None
    }

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
        let home = crate::resolve::user_dsh_home();
        let profiles = crate::resolve::list_web_ui_profiles(&home);
        // 4.3④ defaultProfile 消费（2026-08-28）：用户设过默认且在 webUi 候选
        // 内 → 直接用它启动并跳过选择器。仅覆盖 dsh_home = 用户 home 的档位
        // （system/download；bundle 档是快照世界，管理器与其默认值都不适用）。
        let stored = crate::settings::load(&self.data_dir).default_profile;
        let direct = if launch.dsh_home == home {
            crate::resolve::consume_default_profile(stored.as_deref(), &profiles)
        } else {
            None
        };
        if let Some(p) = direct.as_ref() {
            tracing::info!("defaultProfile 命中：本次启动 profile={p}（跳过选择器）");
        } else if stored.is_some() {
            tracing::info!("defaultProfile={stored:?} 未命中 webUi 候选，按常规流程启动");
        }
        let direct_hit = direct.is_some();
        let tier = launch.tier;
        let launch = match direct {
            Some(p) => LaunchSpec {
                profile: p,
                ..launch
            },
            None => launch,
        };
        // F-b：system 档且用户世界有多个 webUi profile → 由壳出选择器；
        // defaultProfile 已命中时跳过——默认值语义即「下次自动使用」。
        let needs_selector =
            !direct_hit && tier == crate::manifest::TierKind::System && profiles.len() > 1;
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

    fn active_profile(&self) -> Option<&str> {
        self.launch.as_ref().map(|l| l.profile.as_str())
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
    let raw = run_wsl_capture(None, &["-l", "-v"], Duration::from_secs(5))
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
/// 超时参数化：探测/teardown 用 5s；`npm i -g` 慢镜像可能 30s+（见
/// `install_dsh_in_distro` 调用的 120s）。
#[cfg(windows)]
fn run_wsl_capture(distro: Option<&str>, args: &[&str], timeout: Duration) -> Option<String> {
    let mut cmd = wsl_command(distro);
    cmd.args(args);
    let raw = crate::resolve::run_with_timeout_raw(&mut cmd, timeout)?;
    let t = crate::resolve::decode_output_bytes(&raw).trim().to_string();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

/// 客体内停止标志文件：teardown touch 它 → wrapper 收到后 kill dsh 并退出。
#[cfg(windows)]
const GUEST_STOP_FILE: &str = "/tmp/dsh-dock-stop";

/// 客体内就绪哨兵文件：GUEST_BOOT 用 `tee` 把 dsh 输出镜像到这里（WSL 内
/// 行缓冲，**不**经 wsl.exe stdout 转发），壳经 `wsl.exe -e cat` 直读
/// 绕开 wsl.exe 输出缓冲导致的就绪误判。详见 docs/executor.md。
#[cfg(windows)]
const GUEST_READY_FILE: &str = "/tmp/dsh-dock-ready";

/// 客体内「准备好 PATH + 工具链」的公共前缀（固定脚本，不插值用户输入）。
///
/// 为什么这样补 PATH（2026-08-26 实机 bug 修复，nvm 用户探测失败）：
///   1. `.bashrc` 开头有非交互守卫 `case $- in *i*) ;; *) return;; esac`——
///      非交互登录壳（`bash -lc`）source 它时直接 return，nvm/fnm 段根本不执行。
///      因此调用方一律用 **交互式登录壳 `bash -lic`**（$- 含 i，守卫放行；
///      非 tty 下仅 stderr 打 job-control 警告，不影响 stdout/退出码）。
///   2. 兜底扫描常见版本管理器安装位，**不依赖任何 rc 被执行**（用户在
///      `.bashrc` `.profile` `/etc/profile` 里无论如何写 PATH 都生效）：
///      nvm / fnm（含 XDG 变体）/ n / volta。命中 node 的 bin 目录即前置进 PATH。
///   3. 仍显式 source 三个标准 rc（非 Ubuntu 发行版 .profile 可能不 source
///      .bashrc；交互模式 shopt 差异也统一掉）。source 一律 2>/dev/null 静默。
///
/// 模板为纯字符串 → 跨平台可测（macOS/Linux 测试直接以 bash 实跑验证）。
#[cfg(any(windows, test))]
macro_rules! guest_prep {
    () => {
        concat!(
            ". /etc/profile 2>/dev/null;",
            ". \"$HOME/.profile\" 2>/dev/null; . \"$HOME/.bashrc\" 2>/dev/null;",
            "for d in \"$HOME\"/.nvm/versions/node/*/bin \"$HOME\"/.local/share/fnm/node-versions/*/installation/bin",
            " \"$HOME\"/n/bin \"$HOME\"/.volta/bin",
            " \"${XDG_DATA_HOME:-$HOME/.local/share}\"/fnm/node-versions/*/installation/bin; do",
            " [ -x \"$d/node\" ] && PATH=\"$d:$PATH\";",
            "done;",
        )
    };
}

/// 客体内启动 dsh 的固定脚本模板（不插值用户输入；迭代 v1 固定 boot web profile）。
/// 结构：guest_prep（PATH）→ 后台起 dsh（tee 镜像到就绪哨兵）→ 起 watcher（轮询 stop
/// 标志）→ wait dsh → 退出码回传。dsh 崩溃或收到 stop 都让本 wrapper 退出，wsl.exe
/// 子进程随之退出（= 会话存活代理，check_exited 可用）。
/// tee 行缓冲把 dsh 输出复制到 `GUEST_READY_FILE`（WSL 侧），独立于 wsl.exe 转发到
/// `dsh-wsl.log` 的路径——后者有缓冲（实测：90 s 不 flush，URL 不出现直到 wsl.exe 退
/// 出），tee 路径实时（行级），壳优先读这条。dsh 退出时 `rm -f` 清理哨兵。
#[cfg(any(windows, test))]
// 非 Windows 非 test 目标下编译但无引用（引用点在 Windows 运行时路径与跨平台
// 测试里）——保留 cfg 以维持「模板可在 macOS/Linux 直接实跑测试」，豁免 dead。
#[allow(dead_code)]
const GUEST_BOOT: &str = concat!(
    "rm -f /tmp/dsh-dock-stop /tmp/dsh-dock-ready;",
    guest_prep!(),
    "cd \"$HOME\"; ( dsh --profile web --port 0 --no-open 2>&1 | tee /tmp/dsh-dock-ready ) & PID=$!;",
    "(while [ ! -f /tmp/dsh-dock-stop ]; do sleep 1; done; kill -TERM \"$PID\" 2>/dev/null) & WATCH=$!;",
    "wait \"$PID\"; RC=$?; kill \"$WATCH\" 2>/dev/null; rm -f /tmp/dsh-dock-stop /tmp/dsh-dock-ready; exit $RC",
);

/// 客体内探测工具链的固定脚本模板（先 guest_prep 补 PATH）。
/// 三态输出：`READY`（node+dsh）/ `DSH_MISSING`（有 node 缺 dsh，可自动装）/
/// `NODE_MISSING`（无 node，只能提示用户装）。
#[cfg(any(windows, test))]
const GUEST_PROBE: &str = concat!(
    guest_prep!(),
    "if command -v node >/dev/null 2>&1; then",
    " if command -v dsh >/dev/null 2>&1; then echo READY; else echo DSH_MISSING; fi;",
    "else echo NODE_MISSING; fi",
);

/// 客体内自动安装 dsh 的固定脚本模板（npm i -g；2026-08-26 登记网络面）。
/// 总有输出且 `exit 0`（成败看输出里的 `RC=` 行 / `ERR_NPM_MISSING`），
/// 规避 run_with_timeout_raw 的「非零退出=无输出」语义；npm 全量输出进
/// /tmp/dsh-dock-npm.log（管道无死锁风险），只回传尾部 2KB 作诊断。
#[cfg(any(windows, test))]
#[allow(dead_code)] // 同 GUEST_BOOT：非 Windows 非 test 目标无引用，保跨平台可测性
const GUEST_INSTALL_DSH: &str = concat!(
    "rm -f /tmp/dsh-dock-npm.log;",
    guest_prep!(),
    "if ! command -v npm >/dev/null 2>&1; then echo ERR_NPM_MISSING; exit 0; fi;",
    "npm i -g @deepseek-ai/dsh >/tmp/dsh-dock-npm.log 2>&1; RC=$?;",
    "echo '--- npm tail ---'; tail -c 2000 /tmp/dsh-dock-npm.log; echo; echo \"RC=$RC\"; exit 0",
);

/// WSL2 执行器（迭代 v1：WSL2 发行版内跑 dsh，零配置）。Windows 专属。
#[cfg(windows)]
pub struct WslExecutor {
    cfg: WslConfig,
    data_dir: PathBuf,
    selected: Option<String>, // probe 选定的发行版名
    child: Option<std::process::Child>,
    log_path: PathBuf,
    installed_dsh: bool, // 本次 probe 是否自动装过 dsh（→ 壳刷新版本状态）
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
            installed_dsh: false,
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
            Ok(GuestProbeState::Ready) => {
                sink(0, "done", "WSL2 环境就绪");
                sink(1, "done", &format!("{target} 内发现 dsh 与 node"));
                self.selected = Some(target);
                Ok(ProbeOutcome::Ready)
            }
            Ok(GuestProbeState::DshMissing) => {
                // 有 node 缺 dsh → 自动安装（2026-08-26 登记网络面；v1 曾只给提示）。
                // 步骤归属：0=环境检测，1=发行版内工具链（探测/安装/复查同属此步）。
                sink(1, "done", &format!("{target} 内 node 就绪，dsh 缺失"));
                sink(1, "running", "自动安装 dsh（npm i -g @deepseek-ai/dsh）");
                if let Err(e) = install_dsh_in_distro(&target) {
                    sink(1, "error", &e);
                    return Err(format!(
                        "{target} 内自动安装 dsh 失败：{e}\n可手动在 WSL 内执行 `npm i -g @deepseek-ai/dsh` 后重试。"
                    ));
                }
                // 安装后复查；失败一律报错（自动安装算探测定界：装不上就出错误卡）。
                sink(1, "running", "复查安装结果");
                match probe_guest_in_distro(&target) {
                    Ok(GuestProbeState::Ready) => {
                        sink(0, "done", "WSL2 环境就绪");
                        sink(1, "done", &format!("{target} 内发现 dsh 与 node"));
                        self.selected = Some(target);
                        self.installed_dsh = true;
                        Ok(ProbeOutcome::Ready)
                    }
                    Ok(_) => {
                        sink(1, "error", "安装后仍未检测到 dsh");
                        Err(format!(
                            "{target} 内安装 dsh 后仍不可用。请手动在 WSL 内执行 `npm i -g @deepseek-ai/dsh` 后重试。"
                        ))
                    }
                    Err(e) => {
                        sink(1, "error", &e);
                        Err(e)
                    }
                }
            }
            Ok(GuestProbeState::NodeMissing) => {
                sink(1, "error", "发行版内缺少 node");
                Err(format!(
                    "{target} 内缺少 node（探测不到 node 命令）。请在 WSL 内安装 Node.js（如 nvm 或 \
                     apt 的 nodejs 包）后重试；有 node 后 dsh 可由应用自动安装。"
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

    fn active_profile(&self) -> Option<&str> {
        // GUEST_BOOT 固定 `--profile web`（迭代 v1）：会话在槽中 = web 正在运行
        Some("web")
    }

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
            .arg("-lic")
            .arg(GUEST_BOOT)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::from(
                log.try_clone().map_err(|e| e.to_string())?,
            ))
            .stderr(std::process::Stdio::from(log));
        let child = cmd
            .spawn()
            .map_err(|e| format!("spawn wsl.exe 失败：{e}"))?;
        self.child = Some(child);
        Ok(())
    }

    fn just_installed(&self) -> bool {
        self.installed_dsh
    }

    /// 直读客体内哨兵文件（`tee` 写入 `/tmp/dsh-dock-ready`），绕开 wsl.exe
    /// stdout 转发的内部缓冲。500 ms 超时——远小于 wait_for_ready 的 50 ms 轮询
    /// 间隔；超时返回 None，主路径照旧走 log 轮询兜底。
    fn read_ready_marker(&mut self) -> Option<String> {
        let target = self.selected.as_deref()?;
        let script = format!("cat {GUEST_READY_FILE} 2>/dev/null");
        run_wsl_capture(
            Some(target),
            &["-e", "sh", "-lc", &script],
            Duration::from_millis(500),
        )
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
            let _ = run_wsl_capture(
                Some(target),
                &["-e", "sh", "-lc", &script],
                Duration::from_secs(5),
            );
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

/// 客体内探测结果三态（2026-08-26 从布尔改为三态：区分"缺 node"与"缺 dsh"，后者可自动装）。
/// 纯枚举 + 纯分类函数 → 全平台可测（Windows 运行时态见 `probe_guest_in_distro`）。
#[cfg(any(windows, test))]
enum GuestProbeState {
    Ready,
    DshMissing,  // 有 node 缺 dsh → 可自动安装
    NodeMissing, // 无 node → 只能提示用户先装 Node
}

/// 探测输出分类（纯函数）：识别 READY / DSH_MISSING / NODE_MISSING，其余 → None。
/// 全平台可测；Windows 运行时经 `probe_guest_in_distro` 调用。
#[cfg(any(windows, test))]
fn classify_guest_probe(out: &str) -> Option<GuestProbeState> {
    let t = out.trim();
    if t.contains("READY") {
        Some(GuestProbeState::Ready)
    } else if t.contains("DSH_MISSING") {
        Some(GuestProbeState::DshMissing)
    } else if t.contains("NODE_MISSING") {
        Some(GuestProbeState::NodeMissing)
    } else {
        None
    }
}

/// 在指定发行版内探测 node+dsh（固定脚本模板；guest_prep 补 PATH，兼容 nvm/fnm）。
/// Windows 专属。
#[cfg(windows)]
fn probe_guest_in_distro(target: &str) -> Result<GuestProbeState, String> {
    let out = run_wsl_capture(
        Some(target),
        &["-e", "bash", "-lic", GUEST_PROBE],
        Duration::from_secs(10),
    )
    .ok_or_else(|| format!("{target} 内探测命令不可用（wsl.exe 调用失败）"))?;
    classify_guest_probe(&out).ok_or_else(|| {
        // 输出无法识别（警报/横幅/翻译杂讯）→ 视为异常，给可行动信息。
        tracing::warn!("WSL 探测输出无法识别: {out}");
        format!("{target} 内探测输出无法识别（{}）", out.trim())
    })
}

/// 在指定发行版内自动安装 dsh（`npm i -g @deepseek-ai/dsh`；2026-08-26 登记网络面）。
/// 失败经 Err 带诊断尾部（run_wsl_capture 非零退出的输出不可用，模板里刻意 exit 0
/// 让输出总能回传；npm 全量输出落 /tmp/dsh-dock-npm.log 避免管道死锁）。
/// Windows 专属。
#[cfg(windows)]
fn install_dsh_in_distro(target: &str) -> Result<(), String> {
    let out = run_wsl_capture(
        Some(target),
        &["-e", "bash", "-lic", GUEST_INSTALL_DSH],
        Duration::from_secs(120),
    )
    .ok_or_else(|| format!("{target} 内执行 npm 安装失败（wsl.exe 调用失败）"))?;
    if out.contains("ERR_NPM_MISSING") {
        return Err(
            "发行版内没有 npm（装的是精简 Node 或未含 npm）。请安装 Node.js（含 npm）后重试。"
                .to_string(),
        );
    }
    if !out.contains("RC=0") {
        let tail = out
            .lines()
            .rev()
            .take(12)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();
        return Err(format!(
            "npm i -g @deepseek-ai/dsh 失败（尾部诊断）：\n{tail}"
        ));
    }
    Ok(())
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
        assert_eq!(
            crate::resolve::decode_utf16le(&bytes).as_deref(),
            Some("Ubuntu-24.04\r\n")
        );
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

    #[test]
    fn classifies_guest_probe_three_states() {
        // 三态：READY / DSH_MISSING（有 node 缺 dsh，可自动装）/ NODE_MISSING
        assert!(matches!(
            classify_guest_probe("READY\n"),
            Some(GuestProbeState::Ready)
        ));
        assert!(matches!(
            classify_guest_probe("  READY  "),
            Some(GuestProbeState::Ready)
        ));
        assert!(matches!(
            classify_guest_probe("DSH_MISSING"),
            Some(GuestProbeState::DshMissing)
        ));
        assert!(matches!(
            classify_guest_probe("NODE_MISSING"),
            Some(GuestProbeState::NodeMissing)
        ));
        // 无法识别（横幅/警报/杂讯）→ None（运行时视作异常）
        assert!(classify_guest_probe("").is_none());
        assert!(classify_guest_probe("bash: warning: ...\n").is_none());
        assert!(classify_guest_probe("     ").is_none());
    }
}

/// 真实 bash 行为回归（2026-08-26 nvm 探测失败修复）：模板是纯字符串，
/// 在 macOS/Linux/CI 直接以 bash 实跑验证（Windows 上跳过——路径语义不同，
/// Windows 运行时有 wsl.exe 包装，模板本身无平台差异）。
#[cfg(all(unix, test))]
mod guest_shell_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    fn fake_home(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("dsh-dock-guest-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mk home");
        dir
    }

    /// 在 HOME=home 下以 bash（-lic 或 -lc）执行模板，取 stdout 文本。
    /// 隔离要求（两层）：
    ///   1. PATH 指向**受控空目录** + bash 绝对路径——CI runner 的 /usr/bin、
    ///      /usr/local/bin 预装 node（GitHub Actions macOS 实测），收窄到
    ///      /usr/bin:/bin 仍会命中系统 node，污染「无 node」场景断言。
    ///   2. `$HOME/.profile` 统一重置 PATH 回受控目录——macOS 的 `/etc/profile`
    ///      会经 path_helper 把宿主默认 PATH（含 /usr/local/bin/node）塞回来；
    ///      模板 source 顺序是 /etc/profile → ~/.profile，后者正好盖掉前者。
    ///      WSL 发行版（Ubuntu 等）的 /etc/profile 无 path_helper 行为，故此
    ///      仅为测试环境隔离，不改变产品模板语义。
    fn run_guest(home: &Path, script: &str, interactive: bool) -> Option<String> {
        let empty_bin = home.join("empty-bin");
        std::fs::create_dir_all(&empty_bin).expect("mk empty bin");
        std::fs::write(
            home.join(".profile"),
            format!("export PATH=\"{}\"\n", empty_bin.display()),
        )
        .expect("write profile");
        let mut cmd = std::process::Command::new("/bin/bash");
        cmd.arg(if interactive { "-lic" } else { "-lc" })
            .arg(script)
            .env("HOME", home)
            .env("PATH", &empty_bin);
        crate::resolve::run_with_timeout_raw(&mut cmd, std::time::Duration::from_secs(10))
            .map(|b| String::from_utf8_lossy(&b).to_string())
    }

    fn write_fake_bin(dir: &Path, name: &str) {
        let p = dir.join(name);
        std::fs::write(&p, "#!/bin/sh\nexit 0\n").expect("write fake bin");
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }

    /// 模拟 nvm 安装位：`~/.nvm/versions/node/<ver>/bin/{node,dsh}`
    fn nvm_install_bin(home: &Path, ver: &str) -> std::path::PathBuf {
        let bin = home
            .join(".nvm")
            .join("versions")
            .join("node")
            .join(ver)
            .join("bin");
        std::fs::create_dir_all(&bin).expect("mk nvm bin");
        bin
    }

    /// 模拟 fnm 默认安装位：`~/.local/share/fnm/node-versions/<ver>/installation/bin`
    fn fnm_install_bin(home: &Path, ver: &str) -> std::path::PathBuf {
        let bin = home
            .join(".local")
            .join("share")
            .join("fnm")
            .join("node-versions")
            .join(ver)
            .join("installation")
            .join("bin");
        std::fs::create_dir_all(&bin).expect("mk fnm bin");
        bin
    }

    #[test]
    fn rc_guard_blocks_non_interactive_login_shell() {
        // Ubuntu 默认 .bashrc 守卫：非交互登录壳 source 时直接 return（本次实机 bug 根因）。
        let home = fake_home("guard");
        std::fs::write(
            home.join(".bashrc"),
            "case $- in *i*) ;; *) return;; esac\necho BASHRC_SOURCED\n",
        )
        .expect("write bashrc");
        // 交互式登录壳（-lic）：守卫放行 → .bashrc 完整执行
        let lic = run_guest(&home, ". \"$HOME/.bashrc\"", true);
        assert!(
            lic.as_deref().unwrap_or("").contains("BASHRC_SOURCED"),
            "-lic 应完整执行 .bashrc，得到：{lic:?}"
        );
        // 非交互登录壳（-lc，旧方案）：守卫早退 → .bashrc 未执行
        let lc = run_guest(&home, ". \"$HOME/.bashrc\"", false);
        assert!(
            lc.as_deref().is_none_or(|s| !s.contains("BASHRC_SOURCED")),
            "-lc 应被守卫拦截（旧 bug 形态），得到：{lc:?}"
        );
    }

    #[test]
    fn guest_probe_finds_nvm_node_via_rc() {
        // 用户经 nvm 安装（朋友实机形态）：.bashrc 有守卫 + nvm 段 → -lic 下 READY。
        let home = fake_home("nvmrc");
        let bin = nvm_install_bin(&home, "v22.23.2");
        write_fake_bin(&bin, "node");
        write_fake_bin(&bin, "dsh");
        std::fs::write(
            home.join(".bashrc"),
            concat!(
                "case $- in *i*) ;; *) return;; esac\n",
                "export PATH=\"$HOME/.nvm/versions/node/v22.23.2/bin:$PATH\"\n"
            ),
        )
        .expect("write bashrc");
        std::fs::write(home.join(".profile"), "").expect("write profile");
        let out = run_guest(&home, GUEST_PROBE, true);
        assert!(
            out.as_deref().unwrap_or("").contains("READY"),
            "nvm + 守卫版 .bashrc 在 -lic 下应探测 READY，得到：{out:?}"
        );
    }

    #[test]
    fn guest_probe_fallback_scan_finds_nvm_without_any_rc() {
        // 兜底扫描：rc 完全没配 PATH（.bashrc/.profile 全空）也要命中 nvm 安装位。
        let home = fake_home("nvmscan");
        let bin = nvm_install_bin(&home, "v22.23.2");
        write_fake_bin(&bin, "node");
        write_fake_bin(&bin, "dsh");
        std::fs::write(home.join(".bashrc"), "").expect("write bashrc");
        std::fs::write(home.join(".profile"), "").expect("write profile");
        let out = run_guest(&home, GUEST_PROBE, true);
        assert!(
            out.as_deref().unwrap_or("").contains("READY"),
            "空 rc 下兜底扫描应命中 nvm，得到：{out:?}"
        );
    }

    #[test]
    fn guest_probe_fallback_scan_finds_fnm_default_install() {
        // fnm 默认安装位（.local/share/fnm/...，非 XDG 变体）也要命中。
        let home = fake_home("fnmscan");
        let bin = fnm_install_bin(&home, "v22.23.2");
        write_fake_bin(&bin, "node");
        write_fake_bin(&bin, "dsh");
        std::fs::write(home.join(".bashrc"), "").expect("write bashrc");
        std::fs::write(home.join(".profile"), "").expect("write profile");
        let out = run_guest(&home, GUEST_PROBE, true);
        assert!(
            out.as_deref().unwrap_or("").contains("READY"),
            "空 rc 下兜底扫描应命中 fnm，得到：{out:?}"
        );
    }

    #[test]
    fn guest_probe_distinguishes_dsh_missing_from_node_missing() {
        // 只有 node（无 dsh）→ DSH_MISSING（可自动装）；啥都没有 → NODE_MISSING。
        let home = fake_home("states");
        let bin = nvm_install_bin(&home, "v22.23.2");
        write_fake_bin(&bin, "node");
        std::fs::write(home.join(".bashrc"), "").expect("write bashrc");
        std::fs::write(home.join(".profile"), "").expect("write profile");
        let out = run_guest(&home, GUEST_PROBE, true);
        assert!(
            out.as_deref().unwrap_or("").contains("DSH_MISSING"),
            "有 node 无 dsh 应报 DSH_MISSING，得到：{out:?}"
        );
        let empty = fake_home("states2");
        std::fs::write(empty.join(".bashrc"), "").expect("write bashrc");
        std::fs::write(empty.join(".profile"), "").expect("write profile");
        let out = run_guest(&empty, GUEST_PROBE, true);
        assert!(
            out.as_deref().unwrap_or("").contains("NODE_MISSING"),
            "无 node 应报 NODE_MISSING，得到：{out:?}"
        );
    }
}
