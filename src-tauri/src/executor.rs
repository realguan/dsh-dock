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
// WSL 客体链（cfg(windows) 专属）用 &Path 形参——macOS 侧 cfg 剥离后不可见，
// 缺导入只在 Windows 编译时暴露（windows-gnu target check 的存在意义）。
#[cfg(windows)]
use std::path::Path;
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

/// 下载进度回调（与 updates.rs 的 DownloadProgress 同构；保持零 tauri 依赖）。
/// 本机引擎引导补齐 node/dsh 时经它上抛（ui `boot:progress`）：node 上抛
/// 字节，dsh 上抛包计数——阶段枚举见 updates::ProgressStage。
pub type DownloadProgress<'a> = &'a mut dyn FnMut(crate::updates::ProgressStage, u64, Option<u64>);

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

    /// 注入「强制目标 profile」（管理器切换 / 错误卡重试的延续目标，ADR-0009
    /// §4 三次修订）。实现须在 `probe` 内按档位消费：仅 dsh_home = 用户 home
    /// 的世界生效——bundle 快照档无 profile 管理语义，忽略（同 defaultProfile
    /// 消费的档位守卫）。优先级高于 defaultProfile（用户此刻明确指定）。
    fn set_forced_profile(&mut self, profile: Option<String>);

    /// 当前会话已启动的 profile（运行中防护比对源，ADR-0009 §2：删除/重命名
    /// 前比对壳当前 launch.profile）。默认 None；本地档取 launch（select 后即
    /// 真值）；WSL 档取 guest 启动目标（4.3⑥ 起 guest 脚本 profile 参数化）。
    fn active_profile(&self) -> Option<&str> {
        None
    }

    /// WSL 档：本次启动**实际选中**的发行版（ADR-0016 §4 管理面世界择源）。
    /// 本地档 / 未探测完成 = None。probe 成功后由 `launch_executor_after_probe`
    /// 落进 `ShellState.active_wsl_distro`，控制中心据此与运行中会话同源。
    fn target_distro(&self) -> Option<&str> {
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
    /// 不可读。本地默认 None 走 log 路径；WSL 实现在 guest_boot_script 把 dsh 输出
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
/// 强制目标档位守卫（纯函数，4.3⑥）：仅 dsh_home = 用户 home 的世界消费——
/// bundle 快照档是快照世界，profile 管理语义（切换/重试延续）不适用，忽略之
/// （与 defaultProfile 消费的档位条件一致）。
fn effective_forced_profile(forced: Option<String>, dsh_home_is_user_home: bool) -> Option<String> {
    if dsh_home_is_user_home {
        forced
    } else {
        None
    }
}

/// 选择器决策（纯函数，F-b + ADR-0010）：用户世界档（system/引擎）且无直接
/// 目标且 webUi 候选多于一个 → 出选择器。引擎档 dsh_home = 用户 home，与
/// system 档同语义（快照档是独立世界，不消费选择器/默认值/强制目标）。
fn needs_profile_selector(
    tier: crate::manifest::TierKind,
    direct_hit: bool,
    profile_count: usize,
) -> bool {
    !direct_hit && tier == crate::manifest::TierKind::Engine && profile_count > 1
}

pub struct LocalExecutor {
    manifest: ProductManifest,
    resources_dir: PathBuf,
    data_dir: PathBuf,
    path_env: String,
    launch: Option<LaunchSpec>,
    proc: Option<crate::shell::DshProcess>,
    /// 强制目标 profile（管理器切换/重试注入）：probe 内按档位消费后即取走。
    forced_profile: Option<String>,
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
            forced_profile: None,
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
        let is_engine_tier = matches!(
            self.manifest.terminal.resolution.dsh.tiers.first(),
            Some(crate::manifest::TierKind::Engine)
        );
        let ready_status = if is_engine_tier {
            crate::engines::probe_engine_if_ready(&self.data_dir, &self.path_env)
        } else {
            None
        };

        let launch = if let Some(status) = ready_status {
            tracing::info!("引擎环境已完整就绪，跳过环境检测与准备引擎步骤，直接准备启动");
            sink(0, "done", "环境检测通过");
            sink(1, "done", "运行环境已就绪");
            crate::resolve::resolve_launch_engine_ready(
                &self.data_dir,
                self.manifest.terminal.default_profile.clone(),
                status.dsh.as_deref(),
            )
            .map_err(|e| {
                sink(1, "error", &e.to_string());
                e.to_string()
            })?
        } else {
            sink(0, "running", "正在检查系统环境");
            // 环境检测（清单/布局/三件探测）毫秒级，即刻收口——引擎引导全程归
            // 「准备引擎」独占，避免下载期间双步骤同挂「运行中」。
            sink(0, "done", "环境检测通过");
            sink(
                1,
                "running",
                "正在准备运行环境（首次使用需自动下载组件，请耐心等候）",
            );
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
            sink(1, "running", "环境准备完成，即将启动");
            sink(
                1,
                "done",
                &format!("环境已就绪（{}）", tier_label(launch.tier)),
            );
            launch
        };
        let home = crate::resolve::user_dsh_home();
        let profiles = crate::resolve::list_web_ui_profiles(&home);
        // 4.3④ defaultProfile 消费（2026-08-28）：用户设过默认且在 webUi 候选
        // 内 → 直接用它启动并跳过选择器。仅覆盖 dsh_home = 用户 home 的档位
        // （引擎档；bundle 快照档是独立世界，管理器与其默认值都不适用）。
        let stored = crate::settings::load(&self.data_dir).default_profile;
        // 强制目标（管理器切换/重试，4.3⑥）优先于 defaultProfile：用户此刻
        // 明确指定；档位守卫同上——bundle 快照档不消费（ADR-0009 §4 三次修订）。
        let forced = effective_forced_profile(self.forced_profile.take(), launch.dsh_home == home);
        let direct = forced.or_else(|| {
            if launch.dsh_home == home {
                crate::resolve::consume_default_profile(stored.as_deref(), &profiles)
            } else {
                None
            }
        });
        if let Some(p) = direct.as_ref() {
            tracing::info!("启动目标已定：profile={p}（跳过选择器）");
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
        // F-b：用户世界档（system/引擎）且 webUi profile 多于一个 → 由壳出
        // 选择器；defaultProfile 已命中时跳过——默认值语义即「下次自动使用」。
        let needs_selector = needs_profile_selector(tier, direct_hit, profiles.len());
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

    fn set_forced_profile(&mut self, profile: Option<String>) {
        self.forced_profile = profile;
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
            &format!("正在启动「{}」工作台", launch.profile),
        );
        let dsh = crate::shell::spawn_dsh(&launch, &self.data_dir).map_err(|e| e.to_string())?;
        self.proc = Some(dsh);
        // spawn 成功即收口（等待就绪归 step3）：否则 step2 永挂「运行中」，
        // 而后发的 step3/4 done 会让时间线出现「后步完成、前步 loading」的倒挂。
        sink(2, "done", &format!("「{}」工作台已启动", launch.profile));
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
            .map(|l| l.first_bootstrap)
            .unwrap_or(false)
    }

    fn teardown(&mut self) -> Result<(), String> {
        if let Some(mut p) = self.proc.take() {
            crate::shell::stop_dsh(&mut p.child, Duration::from_secs(3));
        }
        Ok(())
    }
}

/// 档位 → 用户可读标签（boot 遥测文案；内部术语 tier=Engine/Bundle 不外露）。
fn tier_label(tier: crate::manifest::TierKind) -> &'static str {
    match tier {
        crate::manifest::TierKind::Engine => "内置引擎",
        crate::manifest::TierKind::Bundle => "内置离线副本",
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
///
/// `pub(crate)`：客体管理面原语（`crate::guest`）与 boot 路径共用同一构造，
/// 避免第二处 wsl.exe 拼装（ADR-0016 §2 单源口径）。
#[cfg(windows)]
pub(crate) fn wsl_command(distro: Option<&str>) -> Command {
    let mut cmd = crate::child_cmd(std::path::Path::new("wsl.exe"));
    if let Some(d) = distro {
        cmd.arg("-d").arg(d);
    }
    cmd
}

/// 在 WSL 内执行并捕获 stdout（带超时）。
/// 兼容 wsl.exe 的 UTF-16 重定向输出（老版本/非英语区域实测）：取原始字节后
/// 探测 NUL（UTF-16LE 特征），命中就按 UTF-16LE 解码。
/// 超时参数化：探测/teardown 用 5s；客体内 pnpm 引导（node 下载 / dsh 安装）
/// 可能数分钟（见 `ensure_guest_engine` 各环的 120s–600s）。
#[cfg(windows)]
pub(crate) fn run_wsl_capture(
    distro: Option<&str>,
    args: &[&str],
    timeout: Duration,
) -> Option<String> {
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
///
/// **单一来源（2026-09-11，task-34）**：本常量既供宿主侧的 `touch`（`teardown`）用，
/// 也**直接插值进**客体内 wrapper 脚本（`guest_boot_script` 的 `while [ ! -f … ]`）。
/// 在本次改造前，脚本里写的是**字面量**，两处靠注释维系（旧注释原文：「注意
/// /tmp/dsh-dock-stop 须与 guest_boot_script 内字面量一致」）——**注释在防，机器没防**：
/// 只改常量不改字面量 ⇒ 宿主 touch 新路径、客体轮询旧路径 ⇒ **停止位永不生效
/// （teardown 静默失效）**，而当时的测试断言的正是那个字面量 ⇒ 仍绿。
/// 现已同源化，并由 `guest_boot_script_inlines_guest_paths_from_constants` 以
/// **出现次数断言**钉住（漏改任一处即红）。
///
/// `cfg` 必须与 `guest_boot_script` 的 `any(windows, test)` **一致**（不是更窄的
/// `windows`）：函数在测试构建下也被编译并调用，若常量只在 `windows` 下存在，
/// macOS/Linux 的 `cargo test` 会因 `E0425: cannot find value … found an item that
/// was configured out` **直接编译失败**（2026-09-11 实测复现）。
/// 放宽是安全的：非 windows 非 test 目标下 `guest_boot_script` 自身也被 cfg 掉，
/// 故常量不会成为无人引用的 dead_code。
#[cfg(any(windows, test))]
const GUEST_STOP_FILE: &str = "/tmp/dsh-dock-stop";

/// 客体内就绪哨兵文件：guest_boot_script 用 `tee` 把 dsh 输出镜像到这里（WSL 内
/// 行缓冲，**不**经 wsl.exe stdout 转发），壳经 `wsl.exe -e cat` 直读
/// 绕开 wsl.exe 输出缓冲导致的就绪误判。详见 docs/executor.md。
///
/// **单一来源**：同 `GUEST_STOP_FILE`——宿主 `read_ready_marker` 的 `cat` 与本常量、
/// 客体脚本内的 `tee` / `rm -f` 三处同源。此前它比 stop 更裸露：脚本里同样是硬编码
/// 字面量，而且**连测试断言都没有**（task-34 发现）。
#[cfg(any(windows, test))]
const GUEST_READY_FILE: &str = "/tmp/dsh-dock-ready";

// `guest_prep!`（客体 PATH/引擎目录准备）与 `sh_quote`（POSIX 单引号引用）
// 已迁至 `crate::guest`（ADR-0016：客体原语单一源，管理面原语与 boot 路径
// 共用同一份脚本片段）。此处反向引用，调用点文本不变。
#[cfg(any(windows, test))]
use crate::guest::{guest_prep, sh_quote};

/// 客体内启动 dsh 的脚本模板（4.3⑥ profile 参数化；v1 曾固定 web）。
/// 结构：guest_prep（PATH）→ 后台起 dsh（tee 镜像到就绪哨兵）→ 起 watcher（轮询 stop
/// 标志）→ wait dsh → 退出码回传。dsh 崩溃或收到 stop 都让本 wrapper 退出，wsl.exe
/// 子进程随之退出（= 会话存活代理，check_exited 可用）。
/// tee 行缓冲把 dsh 输出复制到 `GUEST_READY_FILE`（WSL 侧），独立于 wsl.exe 转发到
/// `dsh-wsl.log` 的路径——后者有缓冲（实测：90 s 不 flush，URL 不出现直到 wsl.exe 退
/// 出），tee 路径实时（行级），壳优先读这条。dsh 退出时 `rm -f` 清理哨兵。
///
/// **停止位/就绪位同源（2026-09-11，task-34）**：两处路径一律经 `{stop}` / `{ready}`
/// 具名参数插值（`GUEST_STOP_FILE` / `GUEST_READY_FILE`），**脚本内不得再出现硬编码
/// 字面量**——改造前它写死 `/tmp/dsh-dock-stop` 与 `/tmp/dsh-dock-ready` 各 3 处，
/// 与宿主侧常量构成「同一语义两份字面量 + 测试断言字面量」的**静默失效**结构：
/// 改常量 ⇒ 宿主与客体各说各的路径 ⇒ teardown 永不生效，而测试仍绿。
/// 现由 `guest_boot_script_inlines_guest_paths_from_constants` 的**出现次数断言**
/// （stop=3 / ready=3）+ 负向断言（脚本内不再出现字面量）双向钉住。
///
/// 改后脚本与改前**逐字节等价**（2026-09-11 实测：589 字节，sha256 `cb170d50…`；
/// 见 `docs/team/停止位同源与reap_args-2026-09-11.md`）——纯等价重构，零行为变更。
#[cfg(any(windows, test))]
// 非 Windows 非 test 目标下编译但无引用（引用点在 Windows 运行时路径与跨平台
// 测试里）——保留 cfg 以维持「模板可在 macOS/Linux 直接实跑测试」，豁免 dead。
#[allow(dead_code)]
fn guest_boot_script(profile: &str) -> String {
    format!(
        "rm -f {stop} {ready};{prep}\
         cd \"$HOME\"; ( dsh --profile {prof} --port 0 --no-open 2>&1 | tee {ready} ) & PID=$!;\
         (while [ ! -f {stop} ]; do sleep 1; done; kill -TERM \"$PID\" 2>/dev/null) & WATCH=$!;\
         wait \"$PID\"; RC=$?; kill \"$WATCH\" 2>/dev/null; rm -f {stop} {ready}; exit $RC",
        stop = GUEST_STOP_FILE,
        ready = GUEST_READY_FILE,
        prep = guest_prep!(),
        prof = sh_quote(profile),
    )
}

/// 客体内引擎链探测的固定脚本模板（ADR-0010 客体同构，2026-09-04）：
/// musl 识别 → pnpm → node → dsh 逐环报**第一缺口**。输出状态机：
/// `GUEST_MUSL`（musl libc，不支持）/ `PNPM_MISSING`（引擎 pnpm 缺，待
/// Windows 侧投递捆绑包）/ `NODE_MISSING`（待客体内 runtime set）/
/// `DSH_MISSING`（待客体内 add -g）/ `READY`。
/// 调用壳一律 `-e bash -lic`（rc 非交互守卫放行，同 guest_prep 约定）。
#[cfg(any(windows, test))]
const GUEST_PROBE: &str = concat!(
    guest_prep!(),
    "if ldd --version 2>/dev/null | head -1 | grep -qi musl; then echo GUEST_MUSL; exit 0; fi;",
    "if [ ! -x \"$DSH_ENGINES/bin/pnpm\" ]; then echo PNPM_MISSING; exit 0; fi;",
    "if [ ! -x \"$DSH_ENGINES/bin/node\" ]; then echo NODE_MISSING; exit 0; fi;",
    "if [ ! -x \"$DSH_ENGINES/bin/dsh\" ]; then echo DSH_MISSING; exit 0; fi;",
    "echo READY",
);

/// 客体内落位投递的 pnpm 压缩包（`/tmp/dsh-dock-pnpm.tgz`，\wsl$ 或 base64
/// stdin 投递）：解包 `package/pnpm` → 引擎 bin + 执行验证 + 清理暂存。
/// 总有输出且 `exit 0`（成败看 `STAGE_OK` / `STAGE_TAR_FAILED` /
/// `STAGE_VERIFY_FAILED` 行，规避 run_with_timeout_raw 的「非零退出=无输出」
/// 语义）；诊断尾部就地输出（全量日志 /tmp/dsh-dock-stage.log）。
#[cfg(any(windows, test))]
#[allow(dead_code)] // 同 guest_boot_script：非 Windows 非 test 目标无引用，保跨平台可测性
const GUEST_STAGE_PNPM: &str = concat!(
    "ENGINES=\"$HOME/.dsh-dock/engines\"; LOG=/tmp/dsh-dock-stage.log;",
    "mkdir -p \"$ENGINES/bin\" \"$ENGINES/stage-tmp\";",
    "if tar -xzf /tmp/dsh-dock-pnpm.tgz -C \"$ENGINES/stage-tmp\" package/pnpm >\"$LOG\" 2>&1 &&",
    "   mv -f \"$ENGINES/stage-tmp/package/pnpm\" \"$ENGINES/bin/pnpm\" &&",
    "   chmod 755 \"$ENGINES/bin/pnpm\"; then",
    " rm -rf \"$ENGINES/stage-tmp\"; rm -f /tmp/dsh-dock-pnpm.tgz;",
    " if \"$ENGINES/bin/pnpm\" --version >\"$LOG\" 2>&1; then echo STAGE_OK;",
    " else echo STAGE_VERIFY_FAILED; tail -c 2000 \"$LOG\"; fi",
    " else echo STAGE_TAR_FAILED; tail -c 2000 \"$LOG\"; fi; exit 0",
);

/// 客体内引擎引导 node 的脚本模板：`pnpm runtime set node`（cwd=引擎目录，
/// 项目作用域——单目录方案把 node 装进引擎）+ `pnpm shim add node`（激活硬链，
/// spike 0003 §2.2）。镜像链 shell 循环 = host `runtime_set_node` 的
/// npmmirror → 官方两次尝试同口径。下载全量输出落 /tmp/dsh-dock-node.log，
/// 只回传尾部诊断。输出 `NODE_OK` / `NODE_FAILED`。
#[cfg(any(windows, test))]
#[allow(dead_code)]
fn guest_bootstrap_node_script(version: &str) -> String {
    // 镜像链走 node_mirror_chain()（env 接缝）：境外 CI 置官方源优先，客体与
    // host 同口径（ADR-0010 台账「客体镜像主权」）。
    let mirrors = crate::engines::node_mirror_chain()
        .iter()
        .map(|m| format!("{{\"release\":\"{m}\"}}"))
        .collect::<Vec<_>>()
        .join("' '");
    format!(
        concat!(
            "ENGINES=\"$HOME/.dsh-dock/engines\"; LOG=/tmp/dsh-dock-node.log;",
            "cd \"$ENGINES\" || {{ echo NODE_FAILED; echo 引擎目录不存在; exit 0; }};",
            "export PNPM_HOME=\"$ENGINES\" PATH=\"$ENGINES/bin:$PATH\";",
            "for M in '{mirrors}'; do",
            " if PNPM_CONFIG_NODE_DOWNLOAD_MIRRORS=\"$M\" pnpm runtime set node {ver} >\"$LOG\" 2>&1 &&",
            "    pnpm shim add node >>\"$LOG\" 2>&1; then",
            "    REAL_NODE=$(find \"$ENGINES/node_modules\" -path \"*/node/bin/node\" -type f 2>/dev/null | head -n 1);",
            "    [ -n \"$REAL_NODE\" ] && ln -sf \"$REAL_NODE\" \"$ENGINES/bin/node\";",
            "    echo NODE_OK; exit 0;",
            " fi;",
            "done; echo NODE_FAILED; tail -c 2000 \"$LOG\"; exit 0",
        ),
        mirrors = mirrors,
        ver = sh_quote(version),
    )
}

/// 客体内引擎引导 dsh 的脚本模板：`pnpm add --global`，registry 镜像链循环
///（npmmirror → npmjs）+ allow-build 放行（host `install_dsh_global` 同口径，
/// flags 由 updates::pnpm_allow_build_flags 单源提供）。全量输出落
/// /tmp/dsh-dock-dsh.log，只回传尾部诊断。输出 `DSH_OK` / `DSH_FAILED`。
#[cfg(any(windows, test))]
#[allow(dead_code)]
fn guest_install_dsh_script(version: &str, registries: &[&str], allow_flags: &str) -> String {
    let regs = registries
        .iter()
        .map(|r| sh_quote(r))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        concat!(
            "ENGINES=\"$HOME/.dsh-dock/engines\"; LOG=/tmp/dsh-dock-dsh.log;",
            "export PNPM_HOME=\"$ENGINES\" PATH=\"$ENGINES/bin:$PATH\";",
            "REAL_NODE=$(find \"$ENGINES/node_modules\" -path \"*/node/bin/node\" -type f 2>/dev/null | head -n 1);",
            "if [ -n \"$REAL_NODE\" ]; then ln -sf \"$REAL_NODE\" \"$ENGINES/bin/node\"; fi;",
            "for R in {regs}; do",
            " if pnpm add --global --registry=\"$R\" {allow} {spec} >\"$LOG\" 2>&1;",
            "  then echo DSH_OK; exit 0; fi;",
            "done; echo DSH_FAILED; tail -c 2000 \"$LOG\"; exit 0",
        ),
        regs = regs,
        allow = allow_flags,
        spec = sh_quote(&format!("@deepseek-ai/dsh@{version}")),
    )
}

/// 客体内升级 DSH（复用 guest_install_dsh_script）。
/// 用于 WSL 模式下的关于页升级和引导恢复。
#[cfg(any(windows, test))]
pub(crate) fn upgrade_guest_dsh(distro: &str, version: Option<&str>) -> Result<String, String> {
    let target_version = match version {
        Some(v) => v.to_string(),
        None => crate::updates::latest_stable_dsh_version()
            .map_err(|e| format!("无法确定 dsh 引导目标版本：{e}"))?,
    };
    let registries = crate::updates::registry_chain();
    let allow = crate::updates::pnpm_allow_build_flags()
        .iter()
        .map(|f| sh_quote(f))
        .collect::<Vec<_>>()
        .join(" ");
    let script = guest_install_dsh_script(
        &target_version,
        &registries.iter().map(String::as_str).collect::<Vec<_>>(),
        &allow,
    );
    #[cfg(windows)]
    {
        let out = run_wsl_capture(
            Some(distro),
            &["-e", "bash", "-c", &script],
            Duration::from_secs(300),
        )
        .ok_or_else(|| format!("{distro} 内 dsh 升级失败（wsl.exe 调用失败或超时）"))?;
        if !out.contains("DSH_OK") {
            return Err(format!("{distro} 内 dsh 升级失败：{}", out.trim()));
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (distro, &script);
    }
    crate::updates::invalidate_guest_probe_cache(distro);
    Ok(target_version)
}

#[cfg(not(any(windows, test)))]
pub(crate) fn upgrade_guest_dsh(_distro: &str, _version: Option<&str>) -> Result<String, String> {
    Err("WSL 仅支持 Windows 平台".to_string())
}

/// WSL2 执行器（迭代 v1：WSL2 发行版内跑 dsh，零配置）。Windows 专属。
#[cfg(windows)]
pub struct WslExecutor {
    cfg: WslConfig,
    data_dir: PathBuf,
    /// resources 目录（投递捆绑 pnpm 的来源；ADR-0010 客体投递）。
    resources_dir: PathBuf,
    selected: Option<String>, // probe 选定的发行版名
    child: Option<std::process::Child>,
    log_path: PathBuf,
    installed_dsh: bool, // 本次 probe 是否补装过 dsh（→ 壳刷新版本状态）
    /// guest 启动目标 profile（4.3⑥ 参数化，默认 web；切换/重试经 forced 覆写）
    profile: String,
    /// 强制目标注入（probe 消费，同 LocalExecutor 语义；guest 世界恒为用户
    /// WSL home，无 bundle 快照档，故无档位守卫）
    forced_profile: Option<String>,
}

#[cfg(windows)]
impl WslExecutor {
    pub fn new(cfg: WslConfig, data_dir: PathBuf, resources_dir: PathBuf) -> Self {
        Self {
            cfg,
            data_dir: data_dir.clone(),
            resources_dir,
            selected: None,
            child: None,
            log_path: data_dir.join("dsh-wsl.log"),
            installed_dsh: false,
            profile: "web".to_string(),
            forced_profile: None,
        }
    }

    /// 引擎链补齐（客体同构 ADR-0010，2026-09-04）：按探测到的第一缺口逐环
    /// 修复并复查直到 READY——pnpm 缺 → Windows 侧投递捆绑包（\\wsl$ 主通道，
    /// stdin base64 兜底）+ 客体内落位；node 缺 → 客体内 runtime set（镜像链
    /// 同 host 口径）；dsh 缺 → 客体内 add -g。返回是否补装过 dsh（→ 壳刷新
    /// 版本状态）。musl 发行版不支持，出可行动错误（ADR-0010 台账裁定）。
    /// 固有 impl（非 trait 方法）——曾误置于 `impl Executor` 内，Windows 侧
    /// 编译即 E0407（2026-09-07 本地 windows-gnu target check 抓出并归位）。
    fn ensure_guest_engine(
        &mut self,
        target: &str,
        mut state: GuestProbeState,
        sink: BootSink<'_>,
    ) -> Result<bool, String> {
        let mut installed_dsh = false;
        loop {
            match state {
                GuestProbeState::Ready => {
                    sink(1, "done", &format!("{target} 内组件已就绪"));
                    return Ok(installed_dsh);
                }
                GuestProbeState::MuslUnsupported => {
                    let msg = format!(
                        "{target} 为 musl libc（Alpine 系）发行版——客体引擎仅支持 \
                         glibc 发行版（Ubuntu/Debian 等）。请改用 glibc 发行版或本地模式。"
                    );
                    sink(1, "error", &msg);
                    return Err(msg);
                }
                GuestProbeState::PnpmMissing => {
                    sink(1, "running", &format!("正在向 {target} 传送内置组件…"));
                    let bundle = crate::updates::guest_pnpm_bundle(&self.resources_dir);
                    deliver_pnpm_bundle(target, &bundle)?;
                    sink(1, "running", &format!("正在 {target} 内安装组件…"));
                    let out = run_wsl_capture(
                        Some(target),
                        &["-e", "bash", "-c", GUEST_STAGE_PNPM],
                        Duration::from_secs(120),
                    )
                    .ok_or_else(|| format!("{target} 内落位 pnpm 失败（wsl.exe 调用失败）"))?;
                    if !out.contains("STAGE_OK") {
                        return Err(format!("{target} 内落位 pnpm 失败：{}", out.trim()));
                    }
                }
                GuestProbeState::NodeMissing => {
                    let version = crate::updates::node_plan(&self.data_dir).version;
                    sink(
                        1,
                        "running",
                        &format!(
                            "{target} 内下载 node v{}（可能需要几分钟）…",
                            crate::updates::display_version(&version)
                        ),
                    );
                    let script = guest_bootstrap_node_script(&version);
                    let out = run_wsl_capture(
                        Some(target),
                        &["-e", "bash", "-c", &script],
                        Duration::from_secs(600),
                    )
                    .ok_or_else(|| {
                        format!("{target} 内 node 引导失败（wsl.exe 调用失败或超时）")
                    })?;
                    if !out.contains("NODE_OK") {
                        return Err(format!("{target} 内 node 引导失败：{}", out.trim()));
                    }
                }
                GuestProbeState::DshMissing => {
                    let version = crate::updates::latest_stable_dsh_version()
                        .map_err(|e| format!("无法确定 dsh 引导目标版本：{e}"))?;
                    sink(
                        1,
                        "running",
                        &format!(
                            "{target} 内安装 dsh v{}…",
                            crate::updates::display_version(&version)
                        ),
                    );
                    let registries = crate::updates::registry_chain();
                    let allow = crate::updates::pnpm_allow_build_flags()
                        .iter()
                        .map(|f| sh_quote(f))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let script = guest_install_dsh_script(
                        &version,
                        &registries.iter().map(String::as_str).collect::<Vec<_>>(),
                        &allow,
                    );
                    let out = run_wsl_capture(
                        Some(target),
                        &["-e", "bash", "-c", &script],
                        Duration::from_secs(300),
                    )
                    .ok_or_else(|| format!("{target} 内 dsh 安装失败（wsl.exe 调用失败或超时）"))?;
                    if !out.contains("DSH_OK") {
                        return Err(format!("{target} 内 dsh 安装失败：{}", out.trim()));
                    }
                    installed_dsh = true;
                }
            }
            // 复查 → 下一缺口 or READY（每次修复后重新探测，保持单一状态源）
            state = probe_guest_in_distro(target)?;
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
                          // 消费强制目标（管理器切换/重试，4.3⑥）：guest 世界恒为用户 WSL home。
        if let Some(p) = self.forced_profile.take() {
            self.profile = p;
        }
        sink(0, "running", "正在检查 WSL2 环境");
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
        // 与本机档同口径：环境检测到发行版选定即收口，客体引擎补齐全程归
        // 「准备引擎」独占（防下载期间双步骤同挂「运行中」）。
        sink(0, "done", "WSL2 环境检测通过");
        sink(1, "running", &format!("正在检查 {target} 内的运行组件"));
        match probe_guest_in_distro(&target) {
            Ok(first) => {
                // 引擎链补齐（客体同构 ADR-0010）：逐环修复并复查直到 READY。
                let installed = self.ensure_guest_engine(&target, first, sink)?;
                self.installed_dsh = installed;
                self.selected = Some(target);
                Ok(ProbeOutcome::Ready)
            }
            Err(e) => {
                sink(1, "error", &e);
                Err(e)
            }
        }
    }

    /// guest 启动目标（4.3⑥ 起参数化；选择器不会在 WSL 触发，此方法实际由
    /// 强制目标注入链路覆写 `profile` 字段，此处兜底可写）。
    fn select_profile(&mut self, profile: String) {
        self.profile = profile;
    }

    fn set_forced_profile(&mut self, profile: Option<String>) {
        self.forced_profile = profile;
    }

    fn active_profile(&self) -> Option<&str> {
        Some(&self.profile)
    }

    /// 本次实际选中的发行版（probe 内 `select_wsl2_distro` 定下）。
    fn target_distro(&self) -> Option<&str> {
        self.selected.as_deref()
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
            .arg(guest_boot_script(&self.profile))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::from(
                log.try_clone().map_err(|e| e.to_string())?,
            ))
            .stderr(std::process::Stdio::from(log));
        // 守卫式 spawn（ADR-0015）：wsl.exe 即会话存活代理，硬杀会留下
        // 客体里继续持锁的 dsh。
        let child = crate::lifecycle::spawn(
            &mut cmd,
            crate::lifecycle::Role::DshWsl,
            crate::lifecycle::GuardCtx::of("wsl.exe", Some(&self.profile)),
        )
        .map_err(|e| format!("spawn wsl.exe 失败：{e}"))?;
        self.child = Some(child);
        // 与本机档同口径：spawn 成功即收口（等待就绪归 step3）。
        sink(2, "done", &format!("WSL（{target}）内 DSH 已启动"));
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
        //    内其它进程。注意 /tmp/dsh-dock-stop 须与 guest_boot_script 内字面量一致。
        if let Some(target) = &self.selected {
            let script = format!("touch {GUEST_STOP_FILE}");
            let _ = run_wsl_capture(
                Some(target),
                &["-e", "sh", "-lc", &script],
                Duration::from_secs(5),
            );
        }
        // 2) 等 wsl.exe 退出（grace），兜底 kill。若客体内 wrapper 未退出
        //    （异常），kill 掉 wsl.exe 后客体内进程可能残留——见 guest_boot_script 注释，
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

/// 客体内引擎链探测结果（ADR-0010 客体同构；按链序 pnpm → node → dsh 报
/// 第一缺口，musl 系不支持）。纯枚举 + 纯分类函数 → 全平台可测。
#[cfg(any(windows, test))]
enum GuestProbeState {
    Ready,
    MuslUnsupported, // musl libc（Alpine 系）→ 可行动错误，不在支持范围
    PnpmMissing,     // 引擎 pnpm 缺 → Windows 侧投递捆绑包
    NodeMissing,     // pnpm 在 node 缺 → 客体内 runtime set
    DshMissing,      // node 在 dsh 缺 → 客体内 add -g
}

/// 探测输出分类（纯函数）：识别 GUEST_MUSL / PNPM_MISSING / NODE_MISSING /
/// DSH_MISSING / READY（按 GUEST_PROBE 链序优先匹配），其余 → None。
/// 全平台可测；Windows 运行时经 `probe_guest_in_distro` 调用。
#[cfg(any(windows, test))]
fn classify_guest_probe(out: &str) -> Option<GuestProbeState> {
    let t = out.trim();
    if t.contains("GUEST_MUSL") {
        Some(GuestProbeState::MuslUnsupported)
    } else if t.contains("PNPM_MISSING") {
        Some(GuestProbeState::PnpmMissing)
    } else if t.contains("NODE_MISSING") {
        Some(GuestProbeState::NodeMissing)
    } else if t.contains("DSH_MISSING") {
        Some(GuestProbeState::DshMissing)
    } else if t.contains("READY") {
        Some(GuestProbeState::Ready)
    } else {
        None
    }
}

/// 在指定发行版内探测壳引擎链（固定脚本模板；guest_prep 定位引擎目录）。
/// Windows 专属。探测前后各一条 info（2026-09-07 smoke 判读：第四轮客体探测
/// 后 20 分钟零日志——无打点则 hang 定位只能靠猜）。
#[cfg(windows)]
fn probe_guest_in_distro(target: &str) -> Result<GuestProbeState, String> {
    tracing::info!("WSL 探测 {target}（10s 超时）…");
    let out = run_wsl_capture(
        Some(target),
        &["-e", "bash", "-lic", GUEST_PROBE],
        Duration::from_secs(10),
    );
    tracing::info!("WSL 探测 {target} 返回：{}", out.is_some());
    let out = out.ok_or_else(|| format!("{target} 内探测命令不可用（wsl.exe 调用失败）"))?;
    classify_guest_probe(&out).ok_or_else(|| {
        // 输出无法识别（警报/横幅/翻译杂讯）→ 视为异常，给可行动信息。
        tracing::warn!("WSL 探测输出无法识别: {out}");
        format!("{target} 内探测输出无法识别（{}）", out.trim())
    })
}

/// 客体内投递暂存路径（/tmp；投递后由 GUEST_STAGE_PNPM 解包落引擎并清理）。
#[cfg(any(windows, test))]
const GUEST_PNPM_STAGING: &str = "/tmp/dsh-dock-pnpm.tgz";

/// guest 目标路径 → Windows UNC 路径（`\\wsl$` 主通道，spike 0003 §2.7 实测
/// 32MB 探针 0.3s 完整）。guest_path 为客体内绝对路径（/tmp/...）。
#[cfg(any(windows, test))]
fn wsl_unc_path(distro: &str, guest_path: &str) -> String {
    format!(r"\\wsl$\{distro}{guest_path}")
}

/// 投递捆绑 pnpm 到客体内暂存路径：`\\wsl$` 文件拷贝为主通道（带体积复核），
/// 失败转 base64 stdin 兜底（spike ④ 两通道均实测 32MB 完整）。
#[cfg(windows)]
fn deliver_pnpm_bundle(distro: &str, bundle: &Path) -> Result<(), String> {
    let expect = std::fs::metadata(bundle)
        .map_err(|e| format!("读取捆绑 pnpm 失败（{}）：{e}", bundle.display()))?
        .len();
    let dest = wsl_unc_path(distro, GUEST_PNPM_STAGING);
    match std::fs::copy(bundle, &dest) {
        Ok(n) if n == expect => {
            tracing::info!(distro = %distro, bytes = n, "pnpm 投递完成（\\\\wsl$ 主通道）");
            return Ok(());
        }
        Ok(n) => {
            tracing::warn!(
                copied = n,
                expected = expect,
                "\\wsl$ 拷贝不完整，转 stdin 兜底"
            )
        }
        Err(e) => tracing::warn!(err = %e, "\\wsl$ 拷贝失败，转 stdin 兜底"),
    }
    deliver_via_stdin(distro, bundle, expect)
}

/// base64 stdin 兜底投递：字节经管道进客体内 `base64 -d`（wsl.exe 转发 stdin，
/// spike ④ 实测 32MB 0.4s）。
#[cfg(windows)]
fn deliver_via_stdin(distro: &str, bundle: &Path, expect: u64) -> Result<(), String> {
    use std::io::Write;
    let bytes = std::fs::read(bundle).map_err(|e| format!("读取捆绑 pnpm 失败：{e}"))?;
    if bytes.len() as u64 != expect {
        return Err("捆绑 pnpm 体积异常（读取不完整）".to_string());
    }
    let mut cmd = wsl_command(Some(distro));
    cmd.args([
        "-e",
        "sh",
        "-c",
        &format!("base64 -d > {GUEST_PNPM_STAGING}"),
    ])
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null());
    let mut child = crate::lifecycle::spawn(
        &mut cmd,
        crate::lifecycle::Role::Probe,
        crate::lifecycle::GuardCtx::of("wsl.exe base64 投递", None),
    )
    .map_err(|e| format!("wsl.exe 启动失败：{e}"))?;
    {
        let mut sin = child.stdin.take().ok_or("wsl.exe stdin 不可用")?;
        sin.write_all(crate::guest::base64_encode(&bytes).as_bytes())
            .map_err(|e| format!("stdin 写入失败：{e}"))?;
    }
    let status = child.wait().map_err(|e| format!("wsl.exe 等待失败：{e}"))?;
    if !status.success() {
        return Err("base64 投递失败（wsl.exe 非零退出）".to_string());
    }
    tracing::info!(distro = %distro, bytes = bytes.len(), "pnpm 投递完成（base64 stdin 兜底通道）");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_shown_for_user_world_tiers_with_multiple_profiles() {
        use crate::manifest::TierKind;
        // 引擎档（用户世界档）：多候选且无直接目标 → 选择器（P3-b 修复：
        // 引擎档原被漏判，多 profile 用户永远进不了选择器）
        assert!(needs_profile_selector(TierKind::Engine, false, 2));
        // 直接目标（defaultProfile 命中/强制目标）→ 跳过
        assert!(!needs_profile_selector(TierKind::Engine, true, 2));
        // 单候选 / 快照档 → 不出选择器
        assert!(!needs_profile_selector(TierKind::Engine, false, 1));
        assert!(!needs_profile_selector(TierKind::Bundle, false, 2));
    }

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
    fn sh_quote_never_lets_metacharacters_escape() {
        // 反例集（恶意/刁钻名）：任何元字符都不得逃出单引号字面量
        for name in [
            "web",
            "my profile",
            "it's",
            "a'; b",
            "a'; rm -rf ~; b",
            "x$(rm -rf ~)y",
            "`rm -rf ~`",
            "a\"b",
            "a\\b",
        ] {
            let q = sh_quote(name);
            assert!(q.starts_with('\'') && q.ends_with('\''), "{name} -> {q}");
        }
        // 转义形状逐字：' → '\''
        assert_eq!(sh_quote("it's"), "'it'\\''s'");
    }

    #[cfg(unix)]
    #[test]
    fn sh_quote_round_trips_through_bash() {
        // 真实 bash 求值回读： quoted 字面量必须还原为原名（注入面收口的实证）
        for name in [
            "web",
            "my profile",
            "it's",
            "a'; b",
            "x$(echo no)y",
            "`echo no`",
        ] {
            let out = std::process::Command::new("/bin/bash")
                .arg("-c")
                .arg(format!("printf %s {}", sh_quote(name)))
                .output()
                .expect("spawn bash");
            assert_eq!(String::from_utf8_lossy(&out.stdout), name);
        }
    }

    #[test]
    fn guest_boot_script_parameterizes_profile_safely() {
        let plain = guest_boot_script("web");
        // 结构不变量：prep → 启动（--port 0 --no-open）→ watcher → wait，逐字保留
        assert!(plain.contains("dsh --profile 'web' --port 0 --no-open"));
        assert!(plain.contains("/tmp/dsh-dock-stop"));
        // 含空格/引号名：安全进参、脚本不断裂
        assert!(guest_boot_script("my profile").contains("dsh --profile 'my profile' --port 0"));
        assert!(guest_boot_script("it's").contains("dsh --profile 'it'\\''s' --port 0"));
    }

    /// **停止位 / 就绪位同源化的门禁（2026-09-11，task-34）**。
    ///
    /// 改造前：宿主侧常量与客体脚本内的**字面量**各写一份，靠注释维系
    /// （`teardown` 的旧注释：「须与 guest_boot_script 内字面量一致」）——**注释在防，
    /// 机器没防**。只改常量 ⇒ 宿主 `touch` 新路径、客体 `while [ ! -f 旧路径 ]` 永远
    /// 等不到 ⇒ **停止位静默失效**；而当时的测试断言的正是那个字面量 ⇒ 仍绿。
    ///
    /// 复现留证（实证）：让常量取三个不同值（原值 / `-x` / 完全不同的路径），
    /// `guest_boot_script("web")` 的输出 **sha256 三次完全一致** —— 当时的脚本与常量
    /// **零耦合**。详见 `docs/team/停止位同源与reap_args-2026-09-11.md`。
    ///
    /// 本用例分两层，**互补地覆盖两个方向**（缺一层就有盲区）：
    ///
    /// 1. **运行时计数**：脚本内每条路径各出现 **3** 次（`rm -f` 清理、`tee`/`while`
    ///    使用、退出前再清理）。它抓的是**漂移**——常量改了而脚本没跟上时，计数随即
    ///    下降 ⇒ 红。`contains` 式断言抓不到「漏改一处」。
    /// 2. **源码参数化**（负向）：断言格式串用的是 `{stop}` / `{ready}` 具名占位符。
    ///    这一层是必需的：**光靠计数有盲区**——若有人把脚本改回硬编码字面量、而该
    ///    字面量恰等于常量**当前**取值，计数仍是 3，第 1 层断言**照绿**（2026-09-11
    ///    自检发现：两条基于同一字面量的计数断言互为恒真式，等于没测）。
    ///    第 2 层直接钉住「来源单一」而非「取值一致」。
    #[test]
    fn guest_boot_script_inlines_guest_paths_from_constants() {
        let plain = guest_boot_script("web");
        assert_eq!(
            plain.matches(GUEST_STOP_FILE).count(),
            3,
            "停止位路径应出现 3 次（rm -f / while 轮询 / 退出前清理）——\
             若常量值变更而脚本未跟上，此处会下降：{plain}"
        );
        assert_eq!(
            plain.matches(GUEST_READY_FILE).count(),
            3,
            "就绪位路径应出现 3 次（rm -f / tee 镜像 / 退出前清理）：{plain}"
        );
        // 第 2 层：脚本必须由常量**构造**，不得写死（见本用例文档注释的盲区说明）。
        //
        // ⚠️ 必须**只扫生产段**：若对整文件做 `contains`，本断言自己的字面量就会命中
        // 自己 ⇒ **恒真式，等于没测**（2026-09-11 自测发现：把生产段"彻底回退成硬编码"
        // 后本断言仍绿，正是因为被搜索的串也出现在测试代码自身里）。
        // 这是"闸门文本判据必须排除自身"的又一实例，与 spawn 闸门排除 `#[cfg(test)]`
        // 段同源。
        let src = include_str!("executor.rs").replace("\r\n", "\n");
        let prod = match src.find("\n#[cfg(test)]\nmod tests") {
            Some(i) => &src[..i],
            None => &src[..],
        };
        assert!(
            prod.contains("\"rm -f {stop} {ready};{prep}\\"),
            "guest_boot_script 的格式串必须用 {{stop}} / {{ready}} 具名占位符插值；\
             若退回硬编码字面量，即使取值恰好正确，也会在常量下次变更时静默失效"
        );
    }

    #[test]
    fn effective_forced_profile_guarded_by_user_home() {
        // 用户 home 世界：强制目标生效且优先于 defaultProfile（or 链序保证）
        assert_eq!(
            effective_forced_profile(Some("33".into()), true).as_deref(),
            Some("33")
        );
        // bundle 快照档：忽略（快照世界无 profile 管理语义）
        assert_eq!(effective_forced_profile(Some("33".into()), false), None);
        assert_eq!(effective_forced_profile(None, true), None);
    }

    #[test]
    fn classifies_guest_probe_engine_chain_states() {
        // 引擎链五态：READY / GUEST_MUSL / PNPM_MISSING / NODE_MISSING / DSH_MISSING
        assert!(matches!(
            classify_guest_probe("READY\n"),
            Some(GuestProbeState::Ready)
        ));
        assert!(matches!(
            classify_guest_probe("  READY  "),
            Some(GuestProbeState::Ready)
        ));
        assert!(matches!(
            classify_guest_probe("GUEST_MUSL"),
            Some(GuestProbeState::MuslUnsupported)
        ));
        assert!(matches!(
            classify_guest_probe("PNPM_MISSING"),
            Some(GuestProbeState::PnpmMissing)
        ));
        assert!(matches!(
            classify_guest_probe("NODE_MISSING"),
            Some(GuestProbeState::NodeMissing)
        ));
        assert!(matches!(
            classify_guest_probe("DSH_MISSING"),
            Some(GuestProbeState::DshMissing)
        ));
        // 无法识别（横幅/警报/杂讯）→ None（运行时视作异常）
        assert!(classify_guest_probe("").is_none());
        assert!(classify_guest_probe("bash: warning: ...\n").is_none());
        assert!(classify_guest_probe("     ").is_none());
    }

    #[test]
    fn wsl_unc_path_formats_distro_and_guest_path() {
        assert_eq!(
            wsl_unc_path("Ubuntu-24.04", "/tmp/dsh-dock-pnpm.tgz"),
            r"\\wsl$\Ubuntu-24.04/tmp/dsh-dock-pnpm.tgz"
        );
    }

    #[test]
    fn guest_scripts_carry_version_mirrors_and_allow_build() {
        // node 引导：版本经 sh_quote、双镜像 JSON、shim 激活、失败总有输出
        let node = guest_bootstrap_node_script("22.23.2");
        assert!(node.contains("runtime set node '22.23.2'"), "{node}");
        assert!(node.contains("PNPM_CONFIG_NODE_DOWNLOAD_MIRRORS"), "{node}");
        assert!(node.contains("shim add node"), "{node}");
        assert!(node.contains("npmmirror.com/mirrors/node/"), "{node}");
        assert!(node.contains("nodejs.org/download/release/"), "{node}");
        assert!(
            node.contains("NODE_OK") && node.contains("NODE_FAILED"),
            "{node}"
        );
        // dsh 引导：registry 链 + allow-build + 版本 spec
        let dsh = guest_install_dsh_script(
            "1.2.3",
            &[
                "https://registry.npmmirror.com",
                "https://registry.npmjs.org",
            ],
            "--allow-build=koffi",
        );
        assert!(dsh.contains("'@deepseek-ai/dsh@1.2.3'"), "{dsh}");
        assert!(dsh.contains("--registry=\"$R\""), "{dsh}");
        assert!(dsh.contains("--allow-build=koffi"), "{dsh}");
        assert!(
            dsh.contains("registry.npmmirror.com") && dsh.contains("registry.npmjs.org"),
            "{dsh}"
        );
        assert!(
            dsh.contains("DSH_OK") && dsh.contains("DSH_FAILED"),
            "{dsh}"
        );
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

    /// 引擎目录造具：`~/.dsh-dock/engines/bin/{tools}`（ADR-0010 客体布局）
    fn engine_bin(home: &Path, tools: &[&str]) -> std::path::PathBuf {
        let bin = home.join(".dsh-dock").join("engines").join("bin");
        std::fs::create_dir_all(&bin).expect("mk engines bin");
        for t in tools {
            write_fake_bin(&bin, t);
        }
        bin
    }

    #[test]
    fn guest_probe_reports_pnpm_missing_in_clean_home() {
        // 引擎目录缺位 → 链首缺口 = PNPM_MISSING；系统/版本管理器 node 不再
        // 参与判定（探测层退役，ADR-0010 客体同构）
        let home = fake_home("pnpmmiss");
        std::fs::write(home.join(".bashrc"), "").expect("write bashrc");
        let out = run_guest(&home, GUEST_PROBE, true);
        assert!(
            out.as_deref().unwrap_or("").contains("PNPM_MISSING"),
            "干净 home 应报 PNPM_MISSING，得到：{out:?}"
        );
    }

    #[test]
    fn guest_probe_walks_engine_chain_in_order() {
        // 链序逐环：只投 pnpm → NODE_MISSING；+node → DSH_MISSING；+dsh → READY
        let home = fake_home("chain");
        std::fs::write(home.join(".bashrc"), "").expect("write bashrc");
        let bin = engine_bin(&home, &["pnpm"]);
        let out = run_guest(&home, GUEST_PROBE, true);
        assert!(
            out.as_deref().unwrap_or("").contains("NODE_MISSING"),
            "只有 pnpm 应报 NODE_MISSING，得到：{out:?}"
        );
        write_fake_bin(&bin, "node");
        let out = run_guest(&home, GUEST_PROBE, true);
        assert!(
            out.as_deref().unwrap_or("").contains("DSH_MISSING"),
            "pnpm+node 应报 DSH_MISSING，得到：{out:?}"
        );
        write_fake_bin(&bin, "dsh");
        let out = run_guest(&home, GUEST_PROBE, true);
        assert!(
            out.as_deref().unwrap_or("").contains("READY"),
            "引擎三件齐应报 READY，得到：{out:?}"
        );
    }

    #[test]
    fn guest_prep_exports_pnpm_home_and_engine_bin_beats_rc() {
        // rc（.bashrc）里把 decoy 目录放进 PATH，guest_prep 的引擎 bin 前置
        // 在 source rc **之后**执行 → 必须胜出；PNPM_HOME 指向引擎目录。
        let home = fake_home("prep");
        let engine = engine_bin(&home, &["node"]);
        let decoy = home.join("decoy");
        std::fs::create_dir_all(&decoy).expect("mk decoy");
        write_fake_bin(&decoy, "node");
        std::fs::write(
            home.join(".bashrc"),
            format!("export PATH=\"{}:$PATH\"\n", decoy.display()),
        )
        .expect("write bashrc");
        let script = format!("{} echo \"$PNPM_HOME\"; command -v node", guest_prep!());
        let out = run_guest(&home, &script, true);
        let text = out.as_deref().unwrap_or("");
        let engines_root = engine.parent().unwrap();
        assert!(
            text.contains(engines_root.to_str().unwrap()),
            "PNPM_HOME 应指向引擎目录，得到：{text:?}"
        );
        let resolved = text.lines().last().unwrap_or("");
        assert_eq!(
            std::path::Path::new(resolved).parent(),
            Some(engine.as_path()),
            "引擎 bin 应盖过 rc PATH（decoy），得到：{text:?}"
        );
    }

    #[test]
    fn guest_stage_pnpm_script_lands_bundle_in_engine_bin() {
        // 真实 tar 解包链：造含 package/pnpm 的 mini tgz → 模拟投递到暂存位 →
        // GUEST_STAGE_PNPM 落位引擎 bin 并验证（staging tgz 随即清理）。
        let home = fake_home("stage");
        std::fs::write(home.join(".bashrc"), "").expect("write bashrc");
        let pkg = home.join("pkg-src").join("package");
        std::fs::create_dir_all(&pkg).expect("mk pkg");
        let pnpm = pkg.join("pnpm");
        std::fs::write(&pnpm, "#!/bin/sh\necho 12.3.1\n").expect("write pnpm");
        std::fs::set_permissions(&pnpm, std::fs::Permissions::from_mode(0o755)).expect("chmod");
        let tgz = home.join("bundle.tgz");
        assert!(
            std::process::Command::new("tar")
                .arg("-czf")
                .arg(&tgz)
                .arg("-C")
                .arg(home.join("pkg-src"))
                .arg("package")
                .status()
                .expect("tar 可用")
                .success(),
            "tar 打包失败"
        );
        // 模拟 deliver_pnpm_bundle 的产物：模板读字面 /tmp 路径（unix 测试平台
        // 语义一致，test 前后都清理，避免残留影响并行/复跑）
        let staging = std::path::Path::new(GUEST_PNPM_STAGING);
        let _ = std::fs::remove_file(staging);
        std::fs::copy(&tgz, staging).expect("stage copy");
        // run_guest 的隔离 PATH（empty-bin）里没有 tar——模板在真实 WSL 里
        // 跑在正常 PATH 下，测试侧恢复 coreutils 路径后再执行。
        let script = format!("PATH=/usr/bin:/bin; export PATH; {GUEST_STAGE_PNPM}");
        let out = run_guest(&home, &script, true);
        let text = out.as_deref().unwrap_or("");
        assert!(text.contains("STAGE_OK"), "落位应成功，得到：{text:?}");
        let landed = home.join(".dsh-dock/engines/bin/pnpm");
        assert!(landed.is_file(), "pnpm 应落位引擎 bin");
        assert!(
            !staging.exists(),
            "暂存 tgz 应被模板清理（投递物不残留 /tmp）"
        );
        let _ = std::fs::remove_file(staging);
    }

    #[test]
    fn guest_upgrade_dsh_generates_correct_version_script() {
        let script = guest_install_dsh_script(
            "0.1.5-rc.2",
            &[
                "https://registry.npmmirror.com",
                "https://registry.npmjs.org",
            ],
            "--allow-build=@deepseek-ai/dsh",
        );
        assert!(script.contains("@deepseek-ai/dsh@0.1.5-rc.2"));
        assert!(script.contains("pnpm add --global"));
        assert!(script.contains("https://registry.npmmirror.com"));
        assert!(script.contains("DSH_OK"));
    }

    #[test]
    fn upgrade_guest_dsh_resolves_and_runs() {
        let res = upgrade_guest_dsh("Ubuntu", Some("0.1.5-rc.2"));
        assert_eq!(res.unwrap(), "0.1.5-rc.2");
    }
}
