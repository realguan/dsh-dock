//! lifecycle.rs —— 子进程生命周期归属（ADR-0015）。
//!
//! 唯一职责：让「壳 spawn 的子进程」在**任何死亡模式**下都被收口——包括
//! `SIGKILL` / 强制退出 / 崩溃 / panic 这些**父进程没有机会跑清理代码**的路径。
//! 既有清理（`RunEvent::Exit` / 信号处理器 / `stop_dsh`）在那些路径上必然失效，
//! 且内核不会替你连坐，故本模块提供两条旁路防线：
//!
//! 1. **生命线 watcher（unix，§3）**：壳持一根管道的写端且**永不写入**；每个
//!    长命子进程配一个旁挂 `sh` watcher 持读端。壳以任何方式死亡 → 内核关闭写端
//!    → watcher 读到 EOF → 对它**那一个** pid 发 `SIGTERM` → grace → `SIGKILL`。
//!    作用域刻意**只到「壳生的那个进程」而非进程组**：连坐进程组会杀死用户经
//!    dsh 起的长驻任务（ADR-0014 §3 方案 D 的否决理由 = ADR-0015 §2.2 硬约束）。
//! 2. **登记表 + 启动期清扫（§2）**：壳 spawn 时在 `<data_dir>/procs/<token>.lock`
//!    持排他锁并**让子进程继承该 fd**（内核在子进程死亡时释放）。下一次壳启动
//!    扫描该目录——**加得上锁 = 持有者已死（陈旧文件，删）**；**加不上锁 = 仍有
//!    活着的子进程 = 真孤儿（收口）**。判据是内核锁而非 pid，故 **PID 复用免疫**。
//!
//! 实测依据（ADR-0015 §7，2026-09-10）：node **全程保留**继承 fd，且该 fd
//! **不流入孙进程**——因此登记锁与「壳的直接子进程」严格 1:1，不会因孙进程存活
//! 而误判；父被 `SIGKILL` 后锁仍被持有、子进程死亡后锁自动释放。
//!
//! 边界纪律（AGENTS 红线 1）：本模块**绝不触碰** dsh 的会话写租约
//! （`<会话>/session.lock`）。那是 dsh 的协议面，删它 = 丧失互斥 = 日志撕裂
//! （ADR-0015 方案 D）。本模块收的是**进程**，不是锁。

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output};
use std::sync::OnceLock;

/// 子进程角色：决定**是否配生命线 watcher**与清扫留痕字段。
///
/// 登记（`procs/` 锁文件）对**全部**角色生效——漏登记一处，该处的孤儿就不受
/// 清扫管辖（契约 §3.4）。生命线只给**长命或持状态**的角色：短命探测进程
/// （毫秒级退出）配 watcher 只会白留一个 `sh` 常驻到壳退出，得不偿失。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// dsh 服务本体（会话写锁的持有者——本 ADR 的事故主角）。
    DshServer,
    /// WSL 客体会话（`wsl.exe` 即会话存活代理）。
    DshWsl,
    /// dsh CLI 转发（插件装卸 / 创建 profile，最长 10 分钟，持 profile 目录与 pnpm store）。
    DshCli,
    /// 引擎引导的 pnpm（`runtime set node` / `add -g`，分钟级，持 pnpm store 锁）。
    Pnpm,
    /// 短命探测（版本查询、`--dump-config`、`tar` 解包等）。
    Probe,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::DshServer => "dsh-server",
            Role::DshWsl => "dsh-wsl",
            Role::DshCli => "dsh-cli",
            Role::Pnpm => "pnpm",
            Role::Probe => "probe",
        }
    }

    /// 是否需要生命线 watcher（壳横死时的主动收口）。
    ///
    /// 仅 unix 的生命线实现消费它（Windows 用 Job Object，不需要 watcher）；
    /// 非 unix 构建下保留定义只为分类语义完整 + 测试可跨平台断言。
    #[cfg_attr(not(unix), allow(dead_code))]
    pub fn needs_lifeline(self) -> bool {
        matches!(
            self,
            Role::DshServer | Role::DshWsl | Role::DshCli | Role::Pnpm
        )
    }
}

/// 登记表目录：`<app_data>/procs`。
pub fn procs_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("procs")
}

/// 登记文件路径（纯函数，供测试）。`token` 见 [`new_token`]。
pub fn registration_path(data_dir: &Path, token: &str) -> PathBuf {
    procs_dir(data_dir).join(format!("{token}.lock"))
}

/// 全局数据目录 + 生命线管道（每次进程生命周期只初始化一次）。
struct Global {
    data_dir: PathBuf,
    /// 生命线**写端**：自持、**永不写入**，只在壳死亡时由内核关闭 → watcher 读到 EOF。
    ///
    /// 刻意"读了也没用"——它的价值全在**持有**（RAII）：一旦被 drop，内核立即
    /// 关闭写端，所有 watcher 会误判"壳已死"而收口正常运行的子进程。
    /// 故此处显式豁免 dead_code，并加 `lifeline_write_end_is_held_for_process_lifetime`
    /// 测试钉住"它确实活着"。
    #[cfg(unix)]
    #[allow(dead_code)]
    lifeline_write: std::os::fd::OwnedFd,
    /// 生命线**读端**：壳保留一份，仅用于给每个 watcher `dup2` 一个副本。
    /// 壳持有读端**不影响 EOF 判据**（EOF 只取决于写端是否全关）。
    #[cfg(unix)]
    lifeline_read: std::os::fd::OwnedFd,
}

static GLOBAL: OnceLock<Option<Global>> = OnceLock::new();

/// 装配生命周期子系统（`run()` 的 setup 早期调用一次）。
///
/// 幂等：重复调用只保留首次（`OnceLock`）。管道创建失败 → 记 warn 并降级为
/// 「只有登记表、没有生命线」——**守卫是兜底，不得成为可用性的单点**（契约 §3.1）。
pub fn init(data_dir: &Path) {
    let _ = GLOBAL.set(build_global(data_dir));
    if GLOBAL.get().is_some_and(Option::is_none) {
        tracing::warn!("子进程生命线装配失败，降级为仅登记表（硬杀收口只剩启动期清扫）");
    }
}

fn build_global(data_dir: &Path) -> Option<Global> {
    #[cfg(unix)]
    {
        match nix::unistd::pipe() {
            Ok((read_end, write_end)) => {
                // **必须显式设 CLOEXEC**：`nix::unistd::pipe()` 只在部分平台走
                // `pipe2(O_CLOEXEC)`（macOS 实测**不设**）。若不设，写端会随每次
                // spawn 泄漏进子进程，于是「壳死亡 → 写端全关 → EOF」的判据永远
                // 不成立，watcher 形同虚设（2026-09-10 实测：泄漏后 watcher 永久
                // 阻塞在 read，孤儿照旧存活）。
                use std::os::fd::AsRawFd;
                for fd in [read_end.as_raw_fd(), write_end.as_raw_fd()] {
                    if let Err(e) = set_cloexec(fd) {
                        tracing::warn!("生命线管道设置 CLOEXEC 失败（fd={fd}）：{e}");
                        return None;
                    }
                }
                Some(Global {
                    data_dir: data_dir.to_path_buf(),
                    lifeline_write: write_end,
                    lifeline_read: read_end,
                })
            }
            Err(e) => {
                tracing::warn!("创建生命线管道失败：{e}");
                None
            }
        }
    }
    #[cfg(not(unix))]
    {
        // Windows 走 Job Object（`KILL_ON_JOB_CLOSE`，内核保证，见 `job` 子模块），
        // 不需要管道式生命线。
        Some(Global {
            data_dir: data_dir.to_path_buf(),
        })
    }
}

fn global() -> Option<&'static Global> {
    GLOBAL.get().and_then(Option::as_ref)
}

/// 给 fd 设 `FD_CLOEXEC`（显式，不依赖 `pipe()` 的平台差异——见 `build_global`）。
#[cfg(unix)]
fn set_cloexec(fd: std::os::fd::RawFd) -> std::io::Result<()> {
    use nix::fcntl::{fcntl, FcntlArg, FdFlag};
    fcntl(fd, FcntlArg::F_SETFD(FdFlag::FD_CLOEXEC))
        .map(|_| ())
        .map_err(std::io::Error::from)
}

thread_local! {
    /// 单测用数据目录覆盖：`GLOBAL` 是进程级 `OnceLock`，而测试并行跑在同一进程里
    /// 且各有独立临时目录——故数据目录按线程覆盖，生命线管道（与目录无关）仍共享。
    static DATA_DIR_OVERRIDE: std::cell::RefCell<Option<PathBuf>> =
        const { std::cell::RefCell::new(None) };
}

/// 本次 spawn 实际使用的数据目录（线程覆盖优先）。
fn active_data_dir() -> Option<PathBuf> {
    if let Some(d) = DATA_DIR_OVERRIDE.with(|o| o.borrow().clone()) {
        return Some(d);
    }
    global().map(|g| g.data_dir.clone())
}

thread_local! {
    /// 最近一次 `Registration::prepare` 生成的 token（同线程内紧邻读取）。
    /// 供 `run()` 在子进程退出后就地清理登记文件。
    static LAST_TOKEN: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
}

/// 单测：在指定数据目录下执行（登记表隔离；生命线管道共享）。
#[cfg(test)]
fn with_data_dir<T>(dir: &Path, f: impl FnOnce() -> T) -> T {
    ensure_global_for_tests();
    DATA_DIR_OVERRIDE.with(|o| *o.borrow_mut() = Some(dir.to_path_buf()));
    let out = f();
    DATA_DIR_OVERRIDE.with(|o| *o.borrow_mut() = None);
    out
}

/// 单测：确保 `GLOBAL` 已装配（数据目录为占位值，实际由线程覆盖决定）。
#[cfg(test)]
fn ensure_global_for_tests() {
    let _ = GLOBAL.set(build_global(Path::new("/nonexistent-dsh-dock-test")));
}

// ---------- Windows：Job Object（内核连坐，裸 FFI） ----------

/// Windows 硬杀收口：**Job Object + `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`**。
///
/// 为什么 Windows 不用生命线管道：Windows 没有可移植的 fd 继承（Rust std 无
/// `pass_fds` 等价物），而**内核自带**更强的原语——把子进程塞进一个 job，
/// 壳进程一死（含 `TerminateProcess` / 崩溃 / 任务管理器强杀），内核就杀光
/// job 内全部进程，**零配合、零竞态**。
///
/// 与 ADR-0014 既有 `taskkill /T` 的关系：那是"正常停止"路径（且要求执行者
/// 还活着）；本机制补的是"执行者已经死了"的路径。两者并存、互不冲突。
///
/// 作用域说明（ADR-0015 §5「平台语义不对称」）：job 会连坐**整棵树**，比 unix
/// 侧（只杀壳生的那个 pid）更宽。这不是新语义——Windows 的既有停止路径
/// `taskkill /T` 本来就是整树；本机制只是让"硬杀"与之一致。
///
/// 依赖纪律（ADR-0015 §2.5）：**裸 `extern "system"` FFI，不引 `windows-sys`**
/// ——只用 3 个 kernel32 符号，不值当为此加一棵依赖树。
#[cfg(windows)]
mod job {
    use std::sync::OnceLock;

    /// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`：job 句柄全部关闭时杀光成员。
    const JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE: u32 = 0x0000_2000;
    /// `JobObjectExtendedLimitInformation`
    const JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS: u32 = 9;

    #[repr(C)]
    #[derive(Default)]
    struct JobObjectBasicLimitInformation {
        per_process_user_time_limit: i64,
        per_job_user_time_limit: i64,
        limit_flags: u32,
        minimum_working_set_size: usize,
        maximum_working_set_size: usize,
        active_process_limit: u32,
        affinity: usize,
        priority_class: u32,
        scheduling_class: u32,
    }

    #[repr(C)]
    #[derive(Default)]
    struct IoCounters {
        read_operation_count: u64,
        write_operation_count: u64,
        other_operation_count: u64,
        read_transfer_count: u64,
        write_transfer_count: u64,
        other_transfer_count: u64,
    }

    #[repr(C)]
    #[derive(Default)]
    struct JobObjectExtendedLimitInformation {
        basic_limit_information: JobObjectBasicLimitInformation,
        io_info: IoCounters,
        process_memory_limit: usize,
        job_memory_limit: usize,
        peak_process_memory_used: usize,
        peak_job_memory_used: usize,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn CreateJobObjectW(
            attrs: *mut core::ffi::c_void,
            name: *const u16,
        ) -> *mut core::ffi::c_void;
        fn SetInformationJobObject(
            job: *mut core::ffi::c_void,
            info_class: u32,
            info: *mut core::ffi::c_void,
            info_len: u32,
        ) -> i32;
        fn AssignProcessToJobObject(
            job: *mut core::ffi::c_void,
            process: *mut core::ffi::c_void,
        ) -> i32;
    }

    /// job 句柄的包装。刻意**不实现 Drop**：句柄必须活到进程消亡——届时由内核
    /// 关闭并触发 `KILL_ON_JOB_CLOSE`，这正是收口机制本身。
    struct JobHandle(*mut core::ffi::c_void);
    // SAFETY: 内核对象句柄，跨线程只读使用（仅在 spawn 时赋值一次）。
    unsafe impl Send for JobHandle {}
    unsafe impl Sync for JobHandle {}

    /// 进程级唯一 job（壳存活期间不关闭 → 不触发 kill）。
    static JOB: OnceLock<Option<JobHandle>> = OnceLock::new();

    fn job() -> Option<&'static JobHandle> {
        JOB.get_or_init(|| {
            // SAFETY: 只传本函数构造的合法句柄与结构体；任一失败即 None（降级）。
            unsafe {
                let handle = CreateJobObjectW(std::ptr::null_mut(), std::ptr::null());
                if handle.is_null() {
                    tracing::warn!("创建 Job Object 失败，Windows 硬杀收口降级为启动期清扫");
                    return None;
                }
                let mut info = JobObjectExtendedLimitInformation::default();
                info.basic_limit_information.limit_flags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let ok = SetInformationJobObject(
                    handle,
                    JOB_OBJECT_EXTENDED_LIMIT_INFORMATION_CLASS,
                    &mut info as *mut _ as *mut core::ffi::c_void,
                    std::mem::size_of::<JobObjectExtendedLimitInformation>() as u32,
                );
                if ok == 0 {
                    tracing::warn!("设置 Job Object 限制失败，硬杀收口降级为启动期清扫");
                    return None;
                }
                Some(JobHandle(handle))
            }
        })
        .as_ref()
    }

    /// 把刚 spawn 的子进程塞进 job。
    ///
    /// 失败语义：只 warn 不阻断——job 只是兜底的一层（另有启动期清扫）；
    /// 且**壳自身可能已被更外层 job 收编**（某些 CI / 终端 / 容器），此时
    /// `AssignProcessToJobObject` 在旧版 Windows 上会失败，属已知边界
    /// （ADR-0015 §5 负面后果）。
    pub fn assign(child: &std::process::Child) {
        let Some(j) = job() else { return };
        use std::os::windows::io::AsRawHandle;
        // `RawHandle` 在 Windows 上**就是** `*mut c_void`，无需再转（clippy
        // `unnecessary_cast`，2026-09-11 CI 在本目标抓到——本机 macOS 的 clippy
        // 看不到 cfg(windows) 分支，`cargo check --target windows` 也不跑 lint，
        // 故此类问题只有**跨目标 clippy** 能拦）。
        let proc_handle = child.as_raw_handle();
        // SAFETY: 两个句柄都是本进程持有的合法对象。
        let ok = unsafe { AssignProcessToJobObject(j.0, proc_handle) };
        if ok == 0 {
            let err = std::io::Error::last_os_error();
            tracing::warn!(
                "AssignProcessToJobObject 失败（pid={}）：{err}——硬杀收口降级（启动期清扫仍在）",
                child.id()
            );
        }
    }

    /// 供测试断言 job 是否装配成功。
    #[cfg(test)]
    pub fn is_ready() -> bool {
        job().is_some()
    }
}

// ---------- 登记 token（纯函数，供测试） ----------

/// 生成登记 token：`<父 pid>-<序号>-<纳秒>`。
///
/// 刻意**不用子进程 pid 命名**——登记文件必须在 `spawn` **之前**创建并持锁
/// （子进程要继承这个 fd），而此刻子进程 pid 尚不存在。pid 写进文件**内容**，
/// 清扫时读取（判据始终是"能否加锁"，不是 pid）。
pub fn new_token(seq: u64, nanos: u128) -> String {
    format!("{}-{}-{}", std::process::id(), seq, nanos)
}

/// 从登记文件内容解析子进程 pid（纯函数，供测试）。
/// 内容缺失/畸形 → None（调用方按"无法收口"留痕，不猜）。
pub fn parse_registration_pid(content: &str) -> Option<u32> {
    let v: serde_json::Value = serde_json::from_str(content).ok()?;
    let pid = v.get("pid")?.as_u64()?;
    u32::try_from(pid).ok().filter(|p| *p > 1)
}

/// 登记文件内容（仅排查用；判据是锁，不是内容）。
fn registration_body(pid: u32, role: Role, ctx: &GuardCtx) -> String {
    serde_json::json!({
        "pid": pid,
        "role": role.as_str(),
        "profile": ctx.profile,
        "argv0": ctx.argv0,
        "startedAtMs": ctx.started_at_ms,
    })
    .to_string()
}

/// 守卫上下文：**仅供留痕与排查**，不参与任何判定。
#[derive(Debug, Clone, Default)]
pub struct GuardCtx {
    pub profile: Option<String>,
    pub argv0: String,
    pub started_at_ms: u64,
}

impl GuardCtx {
    pub fn of(argv0: &str, profile: Option<&str>) -> Self {
        Self {
            profile: profile.map(str::to_string),
            argv0: argv0.to_string(),
            started_at_ms: now_ms(),
        }
    }
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------- spawn / run 唯一 seam（契约 §1） ----------

/// 单点 spawn seam：替代 `Command::spawn()`。
///
/// 生产路径**禁止**再直接调用 `Command::spawn()` / `Command::output()`
/// ——闸门见本文件 `tests::no_unguarded_spawn_in_production`。
/// 守卫装配失败**不阻断**子进程（记 warn 继续）：守卫是兜底，不是前置条件。
pub fn spawn(cmd: &mut Command, role: Role, ctx: GuardCtx) -> std::io::Result<Child> {
    let Some(_) = global() else {
        // 未初始化（早期路径）：退化为普通 spawn，不阻断。
        return cmd.spawn();
    };
    let Some(data_dir) = active_data_dir() else {
        return cmd.spawn();
    };
    let mut reg = Registration::prepare(&data_dir);
    if let Some(reg) = reg.as_mut() {
        reg.attach(cmd);
    }
    #[cfg(unix)]
    let lifeline = if role.needs_lifeline() {
        Lifeline::prepare()
    } else {
        None
    };

    let result = cmd.spawn();
    // 父进程立即交出登记锁的 fd 副本：此后**只有子进程**持有该锁，
    // 「加不上锁」才等价于「子进程仍活着」（父持有会让判据恒为真）。
    let token = reg.as_ref().map(|r| r.token().to_string());
    drop(reg);
    let child = result?;
    #[cfg(unix)]
    if let Some(l) = lifeline {
        l.launch(child.id(), role);
    }
    // Windows：塞进 job——壳以任何方式死亡时由**内核**收口整棵树（ADR-0015 §3 方案 A）。
    #[cfg(windows)]
    job::assign(&child);
    // spawn 之后才拿得到 pid：写入内容供清扫读取（判据仍是锁本身，内容仅排查用）。
    if let Some(token) = token {
        let path = registration_path(&data_dir, &token);
        if let Err(e) = std::fs::write(&path, registration_body(child.id(), role, &ctx)) {
            tracing::warn!("写登记文件失败（{}）：{e}", path.display());
        }
    }
    Ok(child)
}

/// `Command::output()` 的守卫版。
///
/// **必须复刻 `std::process::Command::output()` 的 stdio 约定**（2026-09-10 实测踩坑）：
/// std 的 `output()` 自己会把 stdout/stderr 设成 piped，而我们走的是"先 `spawn`
/// 再 `wait_with_output`"——不显式 piped 的话 `wait_with_output` 拿回的是**空 stdout**
/// （子进程的输出直接写进了父进程的 stdout），调用方会静默拿到空结果。
pub fn run(cmd: &mut Command, role: Role, ctx: GuardCtx) -> std::io::Result<Output> {
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let child = spawn(cmd, role, ctx)?;
    let out = child.wait_with_output();
    // 阻塞到退出 = 该子进程已不存在 → 登记文件立即失效，就地删除：免得探测类
    // 调用（短命、每次启动都会跑几轮）在 procs 目录里堆一地陈旧文件。
    // 判据仍是锁——此刻无人持有，删除安全（`spawn` 内已交出父进程的锁副本）。
    if out.is_ok() {
        if let Some(dir) = active_data_dir() {
            if let Some(token) = LAST_TOKEN.with(|t| t.borrow().clone()) {
                let _ = std::fs::remove_file(registration_path(&dir, &token));
            }
        }
    }
    out
}

/// 一个已持锁的登记文件。
struct Registration {
    token: String,
    /// 持有排他锁的句柄。**必须活到 `drop`**——它是锁的 RAII 载体，早丢即丢锁；
    /// 字段本身"读了没用"，豁免 dead_code。
    #[allow(dead_code)]
    file: std::fs::File,
    #[cfg(unix)]
    raw_fd: std::os::fd::RawFd,
}

impl Registration {
    /// 在 `spawn` **之前**创建并加锁（子进程要继承该 fd）。
    fn prepare(data_dir: &Path) -> Option<Registration> {
        let dir = procs_dir(data_dir);
        if let Err(e) = std::fs::create_dir_all(&dir) {
            tracing::warn!("创建子进程登记目录失败（{}）：{e}", dir.display());
            return None;
        }
        let token = new_token(next_seq(), nanos_now());
        let path = registration_path(data_dir, &token);
        let file = match std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(&path)
        {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!("创建登记文件失败（{}）：{e}", path.display());
                return None;
            }
        };
        if let Err(e) = file.try_lock() {
            tracing::warn!("登记文件加锁失败（{}）：{e:?}", path.display());
            return None;
        }
        #[cfg(unix)]
        let raw_fd = {
            use std::os::fd::AsRawFd;
            file.as_raw_fd()
        };
        LAST_TOKEN.with(|t| *t.borrow_mut() = Some(token.clone()));
        Some(Registration {
            token,
            file,
            #[cfg(unix)]
            raw_fd,
        })
    }

    /// 让子进程继承登记锁 fd（固定 fd 号，避开 stdio 与常见占用）。
    #[cfg(unix)]
    fn attach(&mut self, cmd: &mut Command) {
        use std::os::unix::process::CommandExt;
        let src = self.raw_fd;
        // SAFETY: pre_exec 闭包内只做 `dup2`（async-signal-safe，无分配、无锁）。
        unsafe {
            cmd.pre_exec(move || {
                // dup2 到固定号：目标 fd 的 FD_CLOEXEC 由 dup2 清除 → 子进程持有。
                nix::unistd::dup2(src, INHERITED_LOCK_FD).map_err(std::io::Error::from)?;
                Ok(())
            });
        }
    }

    #[cfg(not(unix))]
    fn attach(&mut self, _cmd: &mut Command) {
        // Windows：登记锁的 fd 继承不可移植（Rust std 无 pass_fds 等价物）。
        // 但登记**文件本身**仍然写（清扫据此知道有哪些壳生的进程、以及 pid），
        // 而硬杀收口由 Job Object 的 `KILL_ON_JOB_CLOSE` 承担（内核保证，
        // 见 `job` 子模块）——两层互补：job 管"壳死"，登记表管"壳没死但进程
        // 成了孤儿"（如 job 赋值失败的降级路径）。
    }

    fn token(&self) -> &str {
        &self.token
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        // **绝不调用 `unlock()`**（2026-09-10 实测踩坑）。
        //
        // `flock(2)` 的锁属于**打开文件描述**（open file description），而
        // `fork`/`dup` 出来的 fd 与父**共享同一个描述**。因此：
        //   - `close()` 父的 fd → 锁仍由子进程持有（描述还有引用）✅ 正是我们要的；
        //   - 显式 `LOCK_UN` → **直接释放整个描述的锁**，连子进程的持有一起撤销 ❌
        // 早先为"语义清晰"写了 `unlock()`，结果登记锁在 spawn 之后立刻失效，
        // 清扫把活着的孤儿误判成陈旧文件——`sweep_reaps_live_orphan_*` 因此红。
        // 此处只关 fd（`File` 析构），让内核按"是否还有引用"决定锁的去留。
        //
        // （父进程持锁副本期间 `try_lock` 会失败，故 `spawn` 在子进程启动后
        //   立即 drop 本结构——见 `spawn` 内注释。）
    }
}

/// 继承给子进程的登记锁 fd 号（避开 0/1/2 与常见运行时占用）。
#[cfg(unix)]
pub const INHERITED_LOCK_FD: std::os::fd::RawFd = 198;

fn next_seq() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    SEQ.fetch_add(1, Ordering::Relaxed)
}

fn nanos_now() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

// ---------- 生命线 watcher（unix） ----------

/// watcher 脚本（纯常量，供测试）：**阻塞在 stdin 上，EOF 即收口目标 pid**。
///
/// 生命线读端经 `Stdio` 交给本进程的 **stdin**——刻意不玩 fd 号（`dup2` 到固定
/// 号 + `pre_exec`）。理由（2026-09-10 实测踩坑）：`pre_exec` 里的 `dup2` 依赖
/// "目标 fd 号在子进程里恰好空闲且不会被随后关掉"，而这在 std/运行时的 fd 管理
/// 下不成立——曾出现 fd 3 被关闭，`read` 立即失败，脚本**误判壳已死**、把正常
/// 运行的子进程杀掉（症状：dsh 一启动就吃 SIGTERM）。`Stdio` 是 std 明确支持的
/// 通道，dup2/CLOEXEC 由它保证。
///
/// EOF 判据用 `cat` 而非 `read`：`cat` 在 EOF 时返回 0、在**读错误**时返回非 0。
/// 于是 `cat … || exit 0` 把"fd 坏了"与"壳死了"区分开——**失败方向安全**：
/// 读不到生命线时宁可不收口，也绝不误杀（收口还有启动期清扫兜底）。
/// EOF 只可能来自"壳进程消失"——写端由壳独占且永不写入。
///
/// 收口序列与 `shell::stop_dsh` 同口径（`SIGTERM` → grace 3s → `SIGKILL`），
/// 且**只对 `$1` 这一个 pid** 发信号（不是进程组，ADR-0015 §2.2）。
#[cfg(unix)]
pub const WATCHER_SCRIPT: &str = "\
[ -n \"$1\" ] || exit 0; \
cat >/dev/null 2>&1 || exit 0; \
kill -TERM \"$1\" 2>/dev/null; \
i=0; \
while kill -0 \"$1\" 2>/dev/null && [ \"$i\" -lt 30 ]; do sleep 0.1; i=$((i+1)); done; \
kill -KILL \"$1\" 2>/dev/null; \
exit 0";

/// 生命线 watcher：旁挂 `sh`，阻塞在 stdin（= 生命线读端）上，EOF 即收口目标 pid。
#[cfg(unix)]
struct Lifeline;

#[cfg(unix)]
impl Lifeline {
    /// 为目标子进程准备生命线（`spawn` 之前调用）：只做可用性判定。
    fn prepare() -> Option<Lifeline> {
        global()?;
        Some(Lifeline)
    }

    /// `spawn` 成功后拉起 watcher（此时才知道目标 pid）。
    fn launch(self, pid: u32, role: Role) {
        use std::os::fd::AsFd;
        let Some(g) = global() else { return };
        // 读端是共享资源（多个 watcher 共用一根生命线），故克隆一份交出去。
        let Ok(read_end) = g.lifeline_read.try_clone() else {
            tracing::warn!("生命线读端复制失败（pid={pid}）——该子进程只剩启动期清扫兜底");
            return;
        };
        let _ = read_end.as_fd();
        let mut watcher = Command::new("/bin/sh");
        watcher
            .arg("-c")
            .arg(WATCHER_SCRIPT)
            .arg("dsh-dock-lifeline") // $0
            .arg(pid.to_string()) // $1
            // 生命线 = stdin：EOF 即"壳已消失"。std 负责 dup2 与 CLOEXEC，
            // 不依赖任何固定 fd 号（见 WATCHER_SCRIPT 注释的踩坑记录）。
            .stdin(std::process::Stdio::from(read_end))
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        match watcher.spawn() {
            Ok(mut w) => {
                // 起一个极轻的收割线程：watcher 收口完就自己退出，避免留僵尸。
                std::thread::spawn(move || {
                    let _ = w.wait();
                });
                tracing::debug!("生命线 watcher 已挂：pid={pid} role={}", role.as_str());
            }
            Err(e) => {
                // 守卫是兜底：拉起失败只留痕，其余防线（登记表 + 启动期清扫）照常。
                tracing::warn!("生命线 watcher 拉起失败（pid={pid}）：{e}");
            }
        }
    }
}

// ---------- 启动期清扫（契约 §2） ----------

/// 清扫结果（`SweepReport`，契约 §1）。
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SweepReport {
    /// 实际收口的 pid。
    pub reaped: Vec<u32>,
    /// 清理的陈旧登记文件数（持有者已死）。
    pub stale: usize,
    /// 收口失败（权限/竞态/内容缺失），必须 warn 留痕。
    pub failed: Vec<(u32, String)>,
}

impl SweepReport {
    pub fn is_empty(&self) -> bool {
        self.reaped.is_empty() && self.stale == 0 && self.failed.is_empty()
    }
}

/// 一条登记记录的判定结果（纯函数，供测试）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationVerdict {
    /// 加锁成功 = 持有者已死 → 陈旧文件，删除（不算孤儿）
    Stale,
    /// 明确"被别人持有" = 真孤儿 → 收口
    LiveOrphan,
    /// **无法判定**（锁机制本身报错：文件系统不支持 flock、权限、I/O 故障）
    /// → **跳过，绝不收口**
    Indeterminate,
}

/// 由 `try_lock` 的结果判定登记记录的归属（纯函数，供测试）。
///
/// **2026-09-10 审核修正（安全方向）**：`File::try_lock` 返回 `Result<(),
/// TryLockError>`，而 `TryLockError` 有**两个**变体（std `fs.rs:144-152`）：
/// - `WouldBlock` = 确实被别的句柄/进程持有 → 真孤儿；
/// - `Error(io::Error)` = **锁机制本身失败**（文件系统不支持、I/O 故障、权限）。
///
/// 首版实现把 `Err(_)` 一律当成"仍被持有"，于是**在不支持 flock 的文件系统上，
/// 每一条登记都会被判成活跃孤儿**，随后按文件内容里的 pid 去 SIGTERM——那个 pid
/// 完全可能已被复用给无关进程。这正是 ADR-0015 反复强调要避免的**误杀**。
/// 现在把"无法判定"单列一档：**不确定时宁可不收口**（收口还有下次启动兜底，
/// 误杀无法挽回）。
pub fn classify_lock_result(result: &Result<(), std::fs::TryLockError>) -> RegistrationVerdict {
    match result {
        Ok(()) => RegistrationVerdict::Stale,
        Err(std::fs::TryLockError::WouldBlock) => RegistrationVerdict::LiveOrphan,
        Err(std::fs::TryLockError::Error(_)) => RegistrationVerdict::Indeterminate,
    }
}

/// 启动期清扫（契约 §1/§2）：收口**上一代壳**遗留的、仍持有登记锁的子进程。
///
/// 判据 = **能否对登记文件加锁**（内核锁，非 pid → PID 复用免疫）：
/// - 加得上 → 持有者已死 → 陈旧文件，删除（不算孤儿）；
/// - 加不上 → 子进程仍活着 → **真孤儿** → 收口（`SIGTERM` → grace → `SIGKILL`）。
///
/// 幂等；只作用于本 `data_dir` 的登记表；**绝不触碰**非登记进程——更不触碰
/// dsh 的会话写租约（红线 1）。
///
/// 必须在 probe **之前**调用（否则可能与新一代探测竞争同一份 profile 目录）。
pub fn sweep_orphans(data_dir: &Path) -> SweepReport {
    sweep_orphans_with(data_dir, &mut reap_pid)
}

/// `sweep_orphans` 的可注入实现（收口动作可替换，供离线测试）。
fn sweep_orphans_with(
    data_dir: &Path,
    reaper: &mut dyn FnMut(u32, Role) -> Result<(), String>,
) -> SweepReport {
    let mut report = SweepReport::default();
    let dir = procs_dir(data_dir);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return report; // 目录不存在 = 从未 spawn 过，正常
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("lock") {
            continue;
        }
        let Ok(file) = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
        else {
            continue;
        };
        let verdict = classify_lock_result(&file.try_lock());
        match verdict {
            // 加锁成功 = 持有者已死 → 陈旧文件
            RegistrationVerdict::Stale => {
                // 只 close，不 `unlock()`：见 `Registration::drop` 的 flock 语义说明
                // （本函数是独立 fd，unlock 无害但没必要；统一纪律避免后人照抄出错）。
                drop(file);
                if std::fs::remove_file(&path).is_ok() {
                    report.stale += 1;
                }
            }
            // 锁机制本身失败 → 无法判定，**跳过**（绝不按 pid 去杀）
            RegistrationVerdict::Indeterminate => {
                drop(file);
                tracing::warn!(
                    "登记文件无法判定归属（锁机制报错，可能文件系统不支持 flock）：{}——跳过收口",
                    path.display()
                );
            }
            // 明确被持有 = 真孤儿
            RegistrationVerdict::LiveOrphan => {
                drop(file);
                let pid = std::fs::read_to_string(&path)
                    .ok()
                    .as_deref()
                    .and_then(parse_registration_pid);
                let role = std::fs::read_to_string(&path)
                    .ok()
                    .and_then(|c| {
                        serde_json::from_str::<serde_json::Value>(&c)
                            .ok()?
                            .get("role")?
                            .as_str()
                            .map(str::to_string)
                    })
                    .map(|r| role_from_str(&r))
                    .unwrap_or(Role::Probe);
                match pid {
                    Some(pid) => match reaper(pid, role) {
                        Ok(()) => {
                            report.reaped.push(pid);
                            let _ = std::fs::remove_file(&path);
                        }
                        Err(e) => {
                            tracing::warn!(
                                "孤儿收口失败：pid={pid} role={} err={e}（登记文件保留待下次清扫）",
                                role.as_str()
                            );
                            report.failed.push((pid, e));
                        }
                    },
                    None => {
                        // 内容缺失/畸形：无法定位 pid。登记文件保留（锁仍被持有），
                        // 下次启动再试——不猜 pid 是纪律（猜错就是误杀别人的进程）。
                        tracing::warn!("登记文件无可解析 pid（{}）——跳过收口", path.display());
                        report
                            .failed
                            .push((0, format!("登记内容不可解析：{}", path.display())));
                    }
                }
            }
        }
    }
    if !report.is_empty() {
        tracing::warn!(
            "启动期清扫：收口 {} 个孤儿 / 清理 {} 个陈旧登记 / 失败 {} 个",
            report.reaped.len(),
            report.stale,
            report.failed.len()
        );
    }
    report
}

/// 角色名 → 角色（纯函数，供测试）。未知值按最保守的 `Probe` 处理（不影响收口）。
pub fn role_from_str(s: &str) -> Role {
    match s {
        "dsh-server" => Role::DshServer,
        "dsh-wsl" => Role::DshWsl,
        "dsh-cli" => Role::DshCli,
        "pnpm" => Role::Pnpm,
        _ => Role::Probe,
    }
}

/// 实际收口：`SIGTERM` → grace 3s（与 `stop_dsh` 同口径）→ `SIGKILL`。
///
/// **只对给定 pid 发信号，绝不发进程组**：连坐进程组会杀死用户经 dsh 起的
/// 长驻任务（ADR-0015 §2.2）。
#[cfg(unix)]
fn reap_pid(pid: u32, role: Role) -> Result<(), String> {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    let target = Pid::from_raw(pid as i32);
    tracing::warn!("收口孤儿子进程：pid={pid} role={}", role.as_str());
    // ESRCH = 目标已消失（竞态）→ 视为成功（幂等语义）。
    match kill(target, Signal::SIGTERM) {
        Ok(()) | Err(nix::errno::Errno::ESRCH) => {}
        Err(e) => return Err(format!("SIGTERM 失败：{e}")),
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while std::time::Instant::now() < deadline {
        if kill(target, None).is_err() {
            return Ok(()); // 已退出
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    match kill(target, Signal::SIGKILL) {
        Ok(()) | Err(nix::errno::Errno::ESRCH) => Ok(()),
        Err(e) => Err(format!("SIGKILL 失败：{e}")),
    }
}

/// `taskkill` 收口参数（**纯函数**，跨平台可测）：`/PID <pid> /T /F`。
///
/// **单一来源（2026-09-11，task-34 / E2）**：本函数是「taskkill 参数构造」的
/// **唯一实现**。此前 `shell::windows_kill_args` 与本模块 `reap_pid` 各写一份
/// 同形 `vec!`，且**只有 shell 侧那份被测**——两处实现、一处覆盖。
/// 现 `shell::windows_kill_args` **委托**本函数（`shell → lifecycle` 依赖早已存在，
/// 见 `shell.rs:101`，**未新增任何依赖边**），于是：
/// - 参数构造只有一个来源；
/// - `shell` 既有测试 `windows_kill_args_cover_whole_tree` 原样转绿 = 天然回归锚。
///
/// **依赖纪律（不得违反）**：本函数**必须**留在本模块内、**不得**改调
/// `crate::shell::*` —— 那会形成 `lifecycle ↔ shell` 模块环，并破坏「本模块不依赖
/// 持壳状态的兄弟模块」这条单向性前提（spawn 闸门把本文件纳入扫描面的地基）。
/// `crate::child_cmd` 在 crate 根，不属兄弟模块、不成环，**允许**（见 `reap_pid`）。
///
/// `/T` = 整棵进程树、`/F` = 强制（Windows 无 POSIX 信号；`/F` 即 `TerminateProcess`）。
/// 少 `/T` 会退回「只杀 `cmd.exe` 壳层、pnpm shim 的 node 继续跑」的老漏洞（ADR-0014）。
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn reap_args(pid: u32) -> Vec<String> {
    vec![
        "/PID".to_string(),
        pid.to_string(),
        "/T".to_string(),
        "/F".to_string(),
    ]
}

/// Windows：孤儿防护由 Job Object 的 `KILL_ON_JOB_CLOSE` 承担——
/// **内核保证**壳一死就杀光 job 内的进程，因此不存在可清扫的孤儿。
/// 保留函数只为签名一致（且 Job 装配失败时也不会静默：见 `job::assign` 的 warn）。
#[cfg(not(unix))]
fn reap_pid(pid: u32, role: Role) -> Result<(), String> {
    // Job Object 未覆盖到的极端情形（装配失败且壳横死）：仍尝试收口，
    // 复用既有的整树收口语义（`taskkill /T /F`，ADR-0014）。
    tracing::warn!(
        "收口孤儿子进程（taskkill 回退路径）：pid={pid} role={}",
        role.as_str()
    );
    let args = reap_args(pid);
    // 2026-09-11 修（task-30）：原为裸 `Command::new("taskkill")`，违 AGENTS §4.1——
    // GUI 进程裸起控制台程序**会闪黑窗**，且丢掉 `.cmd/.bat` 包装；对照 `shell.rs:244`
    // 的**同一动作**本来就走了 `crate::child_cmd`，两处不一致。`child_cmd` 定义在
    // crate 根（`lib.rs:68`），子模块调用**不成环**。
    // 纪律：此处**不得**改调 `crate::shell::*`——`shell.rs:101` 已依赖本模块，
    // 反向依赖会成模块环，并破坏「本模块不依赖持壳状态兄弟模块」的单向性前提
    // （该前提是 spawn 闸门把本文件纳入扫描面的地基）。
    let mut cmd = crate::child_cmd(Path::new("taskkill"));
    cmd.args(&args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    match cmd.status() {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("taskkill 失败：{e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    use std::io::Write as _;
    #[cfg(unix)]
    use std::time::{Duration, Instant};

    /// 每个测试一个独立数据目录（登记表隔离）。
    fn scratch(label: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQ: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "dsh-lifecycle-{label}-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 进程是否存活（仅 unix 测试用；Windows 侧硬杀收口由 Job Object 覆盖，
    /// 断言方式不同，故这里不提供假的跨平台实现以免误导）。
    #[cfg(unix)]
    fn alive(pid: u32) -> bool {
        use nix::sys::signal::kill;
        use nix::unistd::Pid;
        kill(Pid::from_raw(pid as i32), None).is_ok()
    }

    /// 测试用子进程一律**脱离测试进程的 stdio**（2026-09-10 踩坑）。
    ///
    /// 本模块的用例会故意制造"父死子活"的孤儿；子进程若继承测试进程的
    /// stdout/stderr，就会一直握着那条管道——于是 `cargo test | tail` 这类
    /// 管道读端永不 EOF，**测试早已结束但外层命令永久挂起**（实测把一次
    /// 5 分钟的测试拖成无限等待）。孤儿的数据口一律指向 /dev/null。
    #[cfg(unix)]
    fn quiet(cmd: &mut Command) {
        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
    }

    #[cfg(unix)]
    fn wait_until(deadline: Duration, mut f: impl FnMut() -> bool) -> bool {
        let end = Instant::now() + deadline;
        while Instant::now() < end {
            if f() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        f()
    }

    // ---------- 纯函数 ----------

    #[test]
    fn token_carries_parent_pid_and_is_unique_per_call() {
        let a = new_token(1, 100);
        let b = new_token(2, 100);
        assert_ne!(a, b, "同纳秒不同序号也必须唯一（否则登记文件互相覆盖）");
        assert!(a.starts_with(&std::process::id().to_string()));
        assert!(!a.contains('/'), "token 直接做文件名，不得含路径分隔符");
    }

    #[test]
    fn parse_registration_pid_reads_body_and_rejects_junk() {
        assert_eq!(
            parse_registration_pid(r#"{"pid":4321,"role":"dsh-server"}"#),
            Some(4321)
        );
        // 反例：畸形 / 缺字段 / 荒谬值（pid 0/1 是内核与 launchd，绝不收口）
        assert_eq!(parse_registration_pid("not json"), None);
        assert_eq!(parse_registration_pid(r#"{"role":"pnpm"}"#), None);
        assert_eq!(parse_registration_pid(r#"{"pid":0}"#), None);
        assert_eq!(parse_registration_pid(r#"{"pid":1}"#), None);
        assert_eq!(parse_registration_pid(r#"{"pid":-5}"#), None);
        assert_eq!(parse_registration_pid(""), None);
    }

    #[test]
    fn role_round_trips_through_str() {
        for r in [
            Role::DshServer,
            Role::DshWsl,
            Role::DshCli,
            Role::Pnpm,
            Role::Probe,
        ] {
            assert_eq!(role_from_str(r.as_str()), r);
        }
        assert_eq!(role_from_str("unknown-future-role"), Role::Probe);
    }

    #[test]
    fn lifeline_only_covers_state_holding_roles() {
        // 短命探测不配 watcher：否则每次版本探测都白留一个 sh 常驻到壳退出。
        assert!(!Role::Probe.needs_lifeline());
        for r in [Role::DshServer, Role::DshWsl, Role::DshCli, Role::Pnpm] {
            assert!(r.needs_lifeline(), "{r:?} 持状态，必须有生命线");
        }
    }

    #[cfg(unix)]
    #[test]
    fn watcher_script_contract() {
        // 硬约束：只对 $1 单个 pid 发信号（绝不发进程组 —— ADR-0015 §2.2）。
        assert!(
            !WATCHER_SCRIPT.contains('-') || !WATCHER_SCRIPT.contains("kill -TERM -"),
            "watcher 不得对进程组发信号"
        );
        assert!(WATCHER_SCRIPT.contains(r#"kill -TERM "$1""#));
        assert!(WATCHER_SCRIPT.contains(r#"kill -KILL "$1""#));
        assert!(
            WATCHER_SCRIPT.contains("cat >/dev/null"),
            "必须在 stdin（生命线读端）上阻塞等待 EOF"
        );
        assert!(
            WATCHER_SCRIPT.contains("cat >/dev/null 2>&1 || exit 0;"),
            "读错误（fd 异常）时必须**放弃收口**而不是误杀——失败方向安全"
        );
        assert!(
            !WATCHER_SCRIPT.contains("<&3"),
            "不得再玩固定 fd 号（曾因 fd 3 被关而误判壳已死、杀掉正常运行子进程）"
        );
        // grace 3s = stop_dsh 同口径（30 × 0.1s）
        assert!(WATCHER_SCRIPT.contains(r#"[ "$i" -lt 30 ]"#));
        assert!(
            WATCHER_SCRIPT.starts_with(r#"[ -n "$1" ] || exit 0;"#),
            "空 pid 必须直接退出（防御：绝不对空串执行 kill）"
        );
    }

    // ---------- 复现先行：父被 SIGKILL → 子进程的两种命运 ----------

    /// **本 ADR 的验收锚（AGENTS §5 复现先行）**：父被 `SIGKILL` 后，
    /// 经 `lifecycle::spawn` 守卫的子进程必须在 grace 内消失。
    ///
    /// 手法：把测试二进制以「父角色」再唤起一次（libtest 的 `--exact` 过滤），
    /// 该进程 spawn 一个长命 `sleep` 守卫子进程后**阻塞等待**（不跑任何清理代码）；
    /// 由**本测试**对它发 `SIGKILL`。这样就有一段确定的观察窗：杀之前子进程确实
    /// 活着（前置），杀之后必须自行消失（断言）。
    ///
    /// 为什么不由父角色自杀：那样"子进程是否还活着"就与 watcher 的收口速度
    /// 赛跑，前置断言会随机失败（2026-09-10 首版实测如此）。让测试掌控死亡时刻
    /// 才能稳定复现。
    #[cfg(unix)]
    #[test]
    fn guarded_child_dies_when_parent_is_sigkilled() {
        let dir = scratch("kill9");
        let pid_file = dir.join("child.pid");
        let exe = std::env::current_exe().expect("测试二进制路径");

        let mut parent = Command::new(&exe)
            .args([
                "--exact",
                "lifecycle::tests::parent_role_probe",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("DSH_DOCK_LIFECYCLE_PARENT", "1")
            .env("DSH_DOCK_LIFECYCLE_DIR", &dir)
            .env("DSH_DOCK_LIFECYCLE_PIDFILE", &pid_file)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("唤起父角色进程");

        assert!(
            wait_until(Duration::from_secs(10), || pid_file.is_file()),
            "父角色未在 10s 内落盘子进程 pid"
        );
        let child_pid: u32 = std::fs::read_to_string(&pid_file)
            .unwrap()
            .trim()
            .parse()
            .expect("子进程 pid");

        // 前置：父与子都活着（否则本用例没有复现到问题）。
        let parent_pid = parent.id();
        assert!(alive(parent_pid), "前置：父角色应仍在运行");
        assert!(
            alive(child_pid),
            "前置：守卫子进程应已启动（pid={child_pid}）"
        );
        // **关键**：壳（此处 = 父角色）**活着**时，watcher 必须一直阻塞——
        // 停够一个余量再复查，否则本用例会在"watcher 一开始就误杀子进程"的
        // 情况下**假绿**（2026-09-10 实测教训：首版正是如此）。
        std::thread::sleep(Duration::from_millis(1500));
        assert!(
            alive(child_pid),
            "壳还活着时子进程却已消失（pid={child_pid}）——watcher 误判 EOF 并误杀，\
             这不是收口而是故障"
        );
        assert!(alive(parent_pid), "前置：父角色在整个观察窗内应始终运行");

        // 事故时刻：**不给父进程任何跑清理代码的机会**。
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;
            kill(Pid::from_raw(parent_pid as i32), Signal::SIGKILL).expect("SIGKILL 父角色");
        }
        let status = parent.wait().expect("回收父角色进程");
        assert!(
            !status.success(),
            "父角色应以 SIGKILL 结束（无清理机会），实际：{status:?}"
        );

        // 守卫必须收口它：watcher 的 grace 3s + 启动余量。
        assert!(
            wait_until(Duration::from_secs(10), || !alive(child_pid)),
            "父被 SIGKILL 后守卫子进程仍存活（pid={child_pid}）——生命线未生效"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 父角色（仅在被 `DSH_DOCK_LIFECYCLE_PARENT` 唤起时有效）：
    /// 装配生命周期 → 守卫式 spawn 一个长命子进程 → 落盘 pid → **原地阻塞**，
    /// 静候测试方发来的 `SIGKILL`（绝不主动清理——那正是本 ADR 要否证的路径）。
    #[cfg(unix)]
    #[test]
    fn parent_role_probe() {
        let Ok(_on) = std::env::var("DSH_DOCK_LIFECYCLE_PARENT") else {
            return; // 正常测试运行：本用例是空操作
        };
        let dir = PathBuf::from(std::env::var("DSH_DOCK_LIFECYCLE_DIR").unwrap());
        let pid_file = PathBuf::from(std::env::var("DSH_DOCK_LIFECYCLE_PIDFILE").unwrap());
        init(&dir);
        // `DSH_DOCK_LIFECYCLE_REAL_DSH=1` → 用**真的 dsh**（生产同一条 spawn_dsh
        // 路径）当被守护对象，否则用 `sleep` 替身。前者是"用户会话被孤儿占死"
        // 那条链路的逐字复刻，故由专门的 `real_dsh_...` 用例驱动。
        let pid = if std::env::var("DSH_DOCK_LIFECYCLE_REAL_DSH").is_ok() {
            let data_dir = crate::resolve::launch_data_dir_for_test();
            let engine = crate::engines::engine_dsh_bin(&data_dir).expect("本机引擎未就绪");
            let node = crate::engines::engine_node_bin(&data_dir).expect("引擎 node");
            let launch = crate::resolve::LaunchSpec {
                node_bin: node,
                dsh_entry: crate::resolve::DshEntry::Launcher { bin: engine },
                dsh_home: crate::resolve::user_dsh_home(),
                profile: "web".to_string(),
                tier: crate::manifest::TierKind::Engine,
                no_open: true,
                first_bootstrap: false,
            };
            // 生产路径：内部就是 lifecycle::spawn(Role::DshServer)
            crate::shell::spawn_dsh(&launch, &data_dir)
                .expect("spawn_dsh")
                .child
                .id()
        } else {
            let mut cmd = Command::new("sleep");
            cmd.arg("300");
            quiet(&mut cmd);
            spawn(
                &mut cmd,
                Role::DshServer,
                GuardCtx::of("sleep", Some("web")),
            )
            .expect("守卫式 spawn")
            .id()
        };
        let mut f = std::fs::File::create(&pid_file).unwrap();
        write!(f, "{pid}").unwrap();
        f.sync_all().unwrap();
        // 阻塞等待被 SIGKILL。超时兜底（120s）防测试基础设施异常时永久占着。
        std::thread::sleep(Duration::from_secs(120));
        std::process::exit(98);
    }

    /// 对照组：**未经守卫**的裸 spawn，父横死后子进程照样活着——
    /// 这就是本次事故的机理（也是本 ADR 存在的理由）。它固化"问题确实存在"，
    /// 防止将来有人误以为"父死子必死"是内核行为。
    #[cfg(unix)]
    #[test]
    fn unguarded_child_survives_parent_sigkill_control() {
        let dir = scratch("control");
        let pid_file = dir.join("bg.pid");
        // 裸 spawn（不经 lifecycle）：父进程把后台 sleep 的 pid 落盘后自杀。
        // 子进程 stdio 全部脱离父（`>/dev/null 2>&1`），否则它会一直握着
        // 测试进程的 stdout 管道，令外层 `cargo test | ...` 永不 EOF。
        let mut parent = Command::new("/bin/sh")
            .arg("-c")
            .arg("sleep 300 >/dev/null 2>&1 & echo $! > \"$1\"; kill -9 $$")
            .arg("sh")
            .arg(&pid_file)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("起对照组父进程");
        assert!(
            wait_until(Duration::from_secs(10), || pid_file.is_file()),
            "对照组父进程未落盘后台 pid"
        );
        let _ = parent.wait();
        let pid: u32 = std::fs::read_to_string(&pid_file)
            .unwrap()
            .trim()
            .parse()
            .expect("对照组子进程 pid");
        // 竞态说明：后台 sleep 可能尚未 exec 完就被读取，给一点余量再断言。
        std::thread::sleep(Duration::from_millis(300));
        assert!(
            alive(pid),
            "对照组前提：裸 spawn 的子进程在父横死后**应当**存活（内核不连坐）"
        );
        // 收尾，不留垃圾
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;
        let _ = kill(Pid::from_raw(pid as i32), Signal::SIGKILL);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---------- 清扫（契约 §5-2/3/4） ----------

    #[test]
    fn sweep_deletes_stale_registration_without_killing_anything() {
        let dir = scratch("stale");
        std::fs::create_dir_all(procs_dir(&dir)).unwrap();
        with_data_dir(&dir, || {
            let path = registration_path(&dir, "stale-token");
            // 加锁后立刻释放 = 持有者已死
            {
                let f = std::fs::OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .read(true)
                    .write(true)
                    .open(&path)
                    .unwrap();
                f.try_lock().unwrap();
                f.unlock().unwrap();
            }
            let mut reaped = Vec::new();
            let report = sweep_orphans_with(&dir, &mut |pid, _| {
                reaped.push(pid);
                Ok(())
            });
            assert_eq!(report.stale, 1, "陈旧登记文件应被回收");
            assert!(report.reaped.is_empty());
            assert!(reaped.is_empty(), "陈旧文件绝不触发收口");
            assert!(!path.exists(), "陈旧登记文件应被删除");
        });
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 清扫必须**真的收口活着的孤儿**：守卫式 spawn 一个长命子进程后，
    /// 父进程丢弃自己的锁副本（子进程仍持有）→ 清扫应把子进程收掉。
    #[cfg(unix)]
    #[test]
    fn sweep_reaps_live_orphan_holding_registration_lock() {
        let dir = scratch("live");
        let mut child = with_data_dir(&dir, || {
            let mut tmp = Command::new("sleep");
            tmp.arg("300");
            quiet(&mut tmp);
            // 用 Probe：不配生命线，确保本用例检验的是**清扫**而非 watcher。
            spawn(&mut tmp, Role::Probe, GuardCtx::of("sleep", None)).unwrap()
        });
        let pid = child.id();
        assert!(alive(pid), "前置：守卫子进程应已启动");
        assert_eq!(
            std::fs::read_dir(procs_dir(&dir)).unwrap().count(),
            1,
            "应有且仅有 1 个登记文件"
        );

        let report = sweep_orphans(&dir);
        assert_eq!(report.reaped, vec![pid], "活着的登记子进程应被收口");
        // 关键：必须 `wait()` 回收，否则子进程是**僵尸**而 `kill(pid, 0)` 对僵尸
        // 仍返回成功——`alive()` 会误报存活（2026-09-10 实测踩坑：断言假红）。
        let status = child.wait().expect("回收被收口的子进程");
        assert!(!status.success(), "被收口的进程不应正常退出：{status:?}");
        assert!(
            wait_until(Duration::from_secs(3), || !alive(pid)),
            "收口后子进程必须消失"
        );
        assert_eq!(
            std::fs::read_dir(procs_dir(&dir)).unwrap().count(),
            0,
            "收口成功后登记文件应被清理"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 负例：清扫**绝不触碰**未登记进程。
    #[cfg(unix)]
    #[test]
    fn sweep_never_touches_unregistered_processes() {
        let dir = scratch("negative");
        // 一个与登记表无关的旁观进程
        let mut bcmd = Command::new("sleep");
        bcmd.arg("300");
        quiet(&mut bcmd);
        let mut bystander = bcmd.spawn().unwrap();
        with_data_dir(&dir, || {
            // 目录里放一个非 .lock 文件，且不放任何登记
            std::fs::create_dir_all(procs_dir(&dir)).unwrap();
            std::fs::write(procs_dir(&dir).join("readme.txt"), "not a registration").unwrap();
            let report = sweep_orphans(&dir);
            assert!(report.is_empty(), "无登记时清扫应为空操作：{report:?}");
        });
        assert!(alive(bystander.id()), "旁观进程不得被误杀");
        let _ = bystander.kill();
        let _ = bystander.wait();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 清扫对**不可解析内容**的登记：留痕但不猜 pid（不误杀）。
    #[cfg(unix)]
    #[test]
    fn sweep_reports_unparsable_registration_without_killing() {
        let dir = scratch("garbage");
        let path = registration_path(&dir, "garbage");
        std::fs::create_dir_all(procs_dir(&dir)).unwrap();
        {
            // 持锁不放（模拟"持有者仍活着"）但内容不可解析
            let f = std::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .read(true)
                .write(true)
                .open(&path)
                .unwrap();
            f.try_lock().unwrap();
            std::fs::write(&path, "{{{ not json").unwrap();
            let report = sweep_orphans(&dir);
            assert!(
                report.reaped.is_empty(),
                "不可解析时不得收口（猜 pid = 误杀风险）"
            );
            assert_eq!(report.failed.len(), 1, "必须留痕：{report:?}");
            f.unlock().unwrap();
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 守卫装配失败（数据目录不可写）**不得阻断子进程**（契约 §3.1：
    /// 守卫是兜底，不是前置条件）。
    #[test]
    fn guard_failure_does_not_block_child() {
        let dir = scratch("nowrite");
        // 把 procs 位置占成普通文件 → create_dir_all 必败
        std::fs::write(dir.join("procs"), "occupied").unwrap();
        let out = with_data_dir(&dir, || {
            let mut cmd = Command::new(if cfg!(windows) { "cmd" } else { "echo" });
            if cfg!(windows) {
                cmd.args(["/C", "echo", "ok"]);
            } else {
                cmd.arg("ok");
            }
            run(&mut cmd, Role::Probe, GuardCtx::of("echo", None))
        })
        .expect("守卫装配失败也必须让子进程跑完");
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "ok");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 运行期不留残余：`run()` 阻塞到子进程退出，登记文件**就地自清**
    /// （子进程已不存在，锁必然无人持有，删除安全）。
    ///
    /// 这条正是"探测类高频调用"（版本查询、dump-config）不把 procs 目录堆满的
    /// 保证；也让下一次启动的清扫不必处理它们。
    #[test]
    fn run_cleans_its_own_registration_on_exit() {
        let dir = scratch("selfclean");
        with_data_dir(&dir, || {
            let mut cmd = Command::new(if cfg!(windows) { "cmd" } else { "echo" });
            if cfg!(windows) {
                cmd.args(["/C", "echo", "done"]);
            } else {
                cmd.arg("done");
            }
            run(&mut cmd, Role::Probe, GuardCtx::of("echo", None)).unwrap();
        });
        assert_eq!(
            std::fs::read_dir(procs_dir(&dir))
                .map(|d| d.count())
                .unwrap_or(0),
            0,
            "run() 退出后不得留下登记文件"
        );
        let report = sweep_orphans(&dir);
        assert!(report.is_empty(), "无事可扫：{report:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 陈旧登记（持有者已死但文件残留 —— 如上一代壳被硬杀、子进程也已退出）
    /// 必被清扫回收为 `stale`，且**绝不触发收口**。
    #[test]
    fn stale_registration_is_collected_without_reaping() {
        let dir = scratch("collected");
        std::fs::create_dir_all(procs_dir(&dir)).unwrap();
        let path = registration_path(&dir, "left-behind");
        {
            // 造出"上一代壳留下的文件"：写内容 → 加锁 → 释放（= 持有者已死）。
            //
            // **顺序要紧**（2026-09-11，CI 在 windows-latest 抓到）：必须先写后加锁。
            // Windows 的 `LockFileEx` 是**强制锁**——持锁期间另一个句柄（`fs::write`
            // 会新开句柄）根本写不进去，报 os error 33 "another process has locked a
            // portion of the file"；而 macOS 的 `flock` 是劝告锁，持锁写入照样成功。
            // 原顺序在 macOS 绿、在 Windows 红。
            std::fs::write(&path, r#"{"pid":999999,"role":"dsh-server"}"#).unwrap();
            let f = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&path)
                .unwrap();
            f.try_lock().unwrap();
            f.unlock().unwrap();
        }
        let mut reaped = Vec::new();
        let report = sweep_orphans_with(&dir, &mut |pid, _| {
            reaped.push(pid);
            Ok(())
        });
        assert_eq!(report.stale, 1);
        assert!(
            reaped.is_empty(),
            "持有者已死的登记绝不能触发收口（会误杀复用该 pid 的新进程）"
        );
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---------- 闸门：生产路径不得绕过 seam（契约 §3.4 / AGENTS §4.1） ----------

    /// **spawn 闸门的条目级豁免（集中表）** —— 2026-09-11（task-30）。
    ///
    /// 为什么是「条目级」而不是「整文件排除」：本文件**自己曾经不在闸门扫描面里**
    /// （`SOURCES` 漏了 seam 的属主文件），于是 `reap_pid` 的裸 `Command::new("taskkill")`
    /// 绕过 AGENTS §4.1 而**机器完全看不见**（task-29 诊断发现、task-30 修复）。
    /// 整文件排除正是那个盲区的成因——把「这个文件可以随便来」换成
    /// 「**这几个函数是 seam 本体，该文件其余部分一律照扫**」。
    ///
    /// `hits` = 摘掉本条豁免后**应当恰好报出**的违规行数。它一次钉死两件事：
    /// - 非空 ⇒ 该条目**仍在使用**（不是腐化的空条目）；
    /// - 恰好 ⇒ 豁免区间**没有过宽**（扩大到邻接函数会让计数变大）。
    ///
    /// 由 `spawn_gate_exemptions_are_live` 断言，由 `reason` / `date` 保证可解释
    /// （`spawn_gate_exemption_entries_are_well_formed`）。
    #[derive(Clone)]
    struct SpawnExemption {
        file: &'static str,
        /// 函数名（`fn <item>`）。方法亦可（如 `Lifeline::launch` → `"launch"`）。
        item: &'static str,
        /// 摘掉本条后应报出的行数（精确值，见上）。
        hits: usize,
        /// 为什么它**必须**绕过 seam（每条都要能被「这个函数就是 seam 本体」论证）。
        reason: &'static str,
        date: &'static str,
    }

    const SPAWN_EXEMPTIONS: &[SpawnExemption] = &[
        SpawnExemption {
            file: "src/lifecycle.rs",
            item: "spawn",
            hits: 3,
            reason: "seam 本体：守卫式 spawn 的唯一实现（两处降级早退 + 主路径各一）",
            date: "2026-09-11",
        },
        SpawnExemption {
            file: "src/lifecycle.rs",
            item: "run",
            hits: 1,
            reason:
                "seam 本体：`Command::output()` 的守卫版（`wait_with_output()` 是它的等待动作）",
            date: "2026-09-11",
        },
        SpawnExemption {
            file: "src/lifecycle.rs",
            item: "launch",
            hits: 1,
            reason: "生命线 watcher（`Lifeline::launch`）**必须**不经 seam：它的职责正是\
                     壳死收口子进程，若被登记/守卫，清扫会把 watcher 自己当孤儿收掉",
            date: "2026-09-11",
        },
        SpawnExemption {
            file: "src/lifecycle.rs",
            item: "reap_pid",
            hits: 1,
            reason: "清扫器本体：`taskkill` 回退路径**不能**经 seam（清扫在启动早期运行，\
                     再登记一层只会制造自指）；已改走 `crate::child_cmd` 满足 AGENTS §4.1",
            date: "2026-09-11",
        },
    ];

    /// 原始字符串起始判定，返回 `(井号个数, 引导字节数)`（引导含开启引号本身）。
    ///
    /// **识符检查必须在推进下标之前**（2026-09-11 自测发现的一个真 bug）：首版先
    /// `p += 1`（跳过 `b`）再判 `!is_ident(b[p-1])`，于是 `br"…"` 的 `b[p-1]` 恰是
    /// 那个 `b` 本身 → 判据恒假 → **字节原始字符串从未被识别**，其内部花括号会被
    /// 当成结构字符，可能算错函数区间（偏大即静默漏报）。修法与 `network_gate.rs`
    /// 同构：先看 `b[i-1]` 再决定是否推进。
    fn raw_string_start(b: &[u8], i: usize) -> Option<(usize, usize)> {
        let n = b.len();
        if i > 0 && is_ident_byte(b[i - 1]) {
            return None;
        }
        let mut p = i;
        if b[p] == b'b' {
            p += 1;
            if p >= n || b[p] != b'r' {
                return None;
            }
        } else if b[p] != b'r' {
            return None;
        }
        p += 1;
        let mut hashes = 0usize;
        while p < n && b[p] == b'#' {
            hashes += 1;
            p += 1;
        }
        (p < n && b[p] == b'"').then_some((hashes, p - i + 1))
    }

    fn is_ident_byte(c: u8) -> bool {
        c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
    }

    /// 词法掩码：`true` = 该**字节**是参与花括号配对的代码（注释与字面量置 false）。
    ///
    /// 为什么需要：函数体边界靠花括号计数，而 `"}"`（字面量）与 `// }`（注释）里的
    /// 花括号**不是结构**。被骗到的后果不对称——区间偏小 → 误报（可见红，可接受）；
    /// 区间偏大 → **吞掉邻接函数的违规**（静默漏报，正是本缺陷的成因形态）。故按字节做词法。
    ///
    /// 手写状态机（不引依赖，AGENTS §5）：行注释 / **可嵌套**块注释 / 普通与字节字符串 /
    /// 原始字符串（`r"…"`、`r#"…"#`、`br"…"`）/ 字符字面量，并区分字符字面量与生命周期
    /// （`'a` 是代码，`'}'` 是字面量）。
    fn structural_bytes(text: &str) -> Vec<bool> {
        let b = text.as_bytes();
        let n = b.len();
        let mut m = vec![true; n];
        let mut i = 0usize;
        while i < n {
            let c = b[i];
            // 行注释
            if c == b'/' && i + 1 < n && b[i + 1] == b'/' {
                while i < n && b[i] != b'\n' {
                    m[i] = false;
                    i += 1;
                }
                continue;
            }
            // 块注释（可嵌套）
            if c == b'/' && i + 1 < n && b[i + 1] == b'*' {
                let mut depth = 1usize;
                m[i] = false;
                m[i + 1] = false;
                i += 2;
                while i < n && depth > 0 {
                    if b[i] == b'/' && i + 1 < n && b[i + 1] == b'*' {
                        m[i] = false;
                        m[i + 1] = false;
                        depth += 1;
                        i += 2;
                        continue;
                    }
                    if b[i] == b'*' && i + 1 < n && b[i + 1] == b'/' {
                        m[i] = false;
                        m[i + 1] = false;
                        depth -= 1;
                        i += 2;
                        continue;
                    }
                    m[i] = false;
                    i += 1;
                }
                continue;
            }
            // 原始字符串（`r"…"` / `r#…#"` / `br"…"`）
            if let Some((hashes, lead)) = raw_string_start(b, i) {
                for slot in m.iter_mut().take((i + lead).min(n)).skip(i) {
                    *slot = false;
                }
                i += lead;
                while i < n {
                    m[i] = false;
                    if b[i] == b'"' {
                        let mut ok = i + 1 + hashes <= n;
                        for k in 0..hashes {
                            if i + 1 + k >= n || b[i + 1 + k] != b'#' {
                                ok = false;
                                break;
                            }
                        }
                        if ok {
                            for k in 0..hashes {
                                m[i + 1 + k] = false;
                            }
                            i += 1 + hashes;
                            break;
                        }
                    }
                    i += 1;
                }
                continue;
            }
            // 普通字符串 / 字节字符串
            if c == b'"' {
                m[i] = false;
                i += 1;
                while i < n {
                    m[i] = false;
                    if b[i] == b'\\' && i + 1 < n {
                        m[i + 1] = false;
                        i += 2;
                        continue;
                    }
                    if b[i] == b'"' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                continue;
            }
            // 字符字面量 vs 生命周期
            if c == b'\'' {
                if let Some(len) = char_lit_len(b, i) {
                    for slot in m.iter_mut().take((i + len).min(n)).skip(i) {
                        *slot = false;
                    }
                    i += len;
                } else {
                    i += 1; // 生命周期标注：留在代码
                }
                continue;
            }
            i += 1;
        }
        m
    }

    /// 字符字面量长度（含引号）；`None` = 这是生命周期标注（`'a`），不是字面量。
    fn char_lit_len(b: &[u8], i: usize) -> Option<usize> {
        let n = b.len();
        if i + 1 >= n {
            return None;
        }
        if b[i + 1] == b'\\' {
            let end = (i + 13).min(n);
            return b[i + 2..end]
                .iter()
                .position(|&c| c == b'\'')
                .map(|rel| rel + 3);
        }
        let first = b[i + 1];
        if first < 0x80 {
            return (i + 2 < n && b[i + 2] == b'\'').then_some(3);
        }
        let extra = match first {
            0xF0..=0xF7 => 4,
            0xE0..=0xEF => 3,
            0xC0..=0xDF => 2,
            _ => return None,
        };
        (i + extra < n && b[i + extra] == b'\'').then_some(extra + 2)
    }

    /// 按「函数名」定位条目区间的**字节偏移**——返回**全部**匹配，
    /// 故 `reap_pid` 的两个 `#[cfg]` 分支都在内（`#[cfg(not(unix))]` 的那个才有命中）。
    ///
    /// 只认**代码**里的 `fn <ident>`（词边界，`fn spawn` 不得命中 `fn spawn_x`），
    /// 且要求先找到函数体的首个 `{`（跳过参数表/返回类型里的圆括号与方括号，
    /// 遇同层 `;` 视为无体声明而放弃）再配对到 `}`。
    fn item_spans(text: &str, ident: &str) -> Vec<(usize, usize)> {
        let b = text.as_bytes();
        let m = structural_bytes(text);
        let needle = format!("fn {ident}");
        let mut out = Vec::new();
        for (i, _) in text.match_indices(&needle) {
            if i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_') {
                continue;
            }
            let after = i + needle.len();
            if after < b.len() && (b[after].is_ascii_alphanumeric() || b[after] == b'_') {
                continue;
            }
            if !m[i] {
                continue; // 注释 / 字面量里的伪命中
            }
            let mut k = after;
            let mut pd = 0i32;
            let body = loop {
                if k >= b.len() {
                    break None;
                }
                if m[k] {
                    match b[k] {
                        b'(' | b'[' => pd += 1,
                        b')' | b']' => pd -= 1,
                        b'{' if pd == 0 => break Some(k),
                        b';' if pd == 0 => break None,
                        _ => {}
                    }
                }
                k += 1;
            };
            let Some(mut k) = body else { continue };
            let mut depth = 0usize;
            let mut end = None;
            while k < b.len() {
                if m[k] {
                    if b[k] == b'{' {
                        depth += 1;
                    } else if b[k] == b'}' {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(k + 1);
                            break;
                        }
                    }
                }
                k += 1;
            }
            if let Some(e) = end {
                out.push((i, e));
            }
        }
        out
    }

    /// 把给定字节区间抹成空白——**保留换行**，故行号恒等于原始行号。
    fn blank_spans(text: &str, spans: &[(usize, usize)]) -> String {
        let mut b = text.as_bytes().to_vec();
        for &(s, e) in spans {
            let end = e.min(b.len());
            for byte in b.iter_mut().take(end).skip(s) {
                if *byte != b'\n' && *byte != b'\r' {
                    *byte = b' ';
                }
            }
        }
        String::from_utf8(b).expect("区间边界均落在 ASCII 定界符上，不应破坏 UTF-8")
    }

    /// 读一份源码（豁免表自检用；与主闸门的 `include_str!` 同一文件）。
    fn read_source(name: &str) -> String {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("读取 {} 失败：{e}", p.display()))
    }

    /// 摘掉一条豁免后的表（用来证明该条目确实在挡东西）。
    fn table_without(entry: &SpawnExemption) -> Vec<SpawnExemption> {
        SPAWN_EXEMPTIONS
            .iter()
            .filter(|e| !(e.file == entry.file && e.item == entry.item))
            .cloned()
            .collect()
    }

    /// 无豁免表的扫描（合成片段用；语义与扩建前完全一致）。
    fn scan_unguarded_spawns(name: &str, text: &str) -> Vec<String> {
        scan_unguarded_spawns_with(name, text, &[])
    }

    /// 纯函数：扫描一份源码文本，返回绕过 seam 的行（`名:行号: 内容`）。
    ///
    /// **行尾归一（2026-09-11，CI 在 windows-latest 抓到）**：原先模式写死
    /// `"\n#[cfg(test)]\nmod tests"`，而 Git for Windows 的 `autocrlf` 会把源码
    /// checkout 成 CRLF，`include_str!` 于是拿到 `\r\n`，模式永不匹配 → 截断失效 →
    /// **测试模块里的裸 spawn 被误报成生产代码**，闸门在 Windows 上恒红。故先归一。
    ///
    /// 该失效模式值得记住：**任何以源码文本为判据的闸门都可能被行尾/编码打穿**，
    /// 且只在非 LF 检出环境暴露（本机 macOS 永远看不到）。`.gitattributes` 已是第二道保险，
    /// 但闸门自己必须扛住。
    ///
    /// `table` = 条目级豁免表：命中的函数区间先被抹掉（行号不变），其余一律照扫。
    fn scan_unguarded_spawns_with(name: &str, text: &str, table: &[SpawnExemption]) -> Vec<String> {
        let text = text.replace("\r\n", "\n");
        let spans: Vec<(usize, usize)> = table
            .iter()
            .filter(|e| e.file == name)
            .flat_map(|e| item_spans(&text, e.item))
            .collect();
        let text = if spans.is_empty() {
            text
        } else {
            blank_spans(&text, &spans)
        };
        // 测试模块之后不再扫描（测试要故意制造裸 spawn 做对照）。
        let prod = match text.find("\n#[cfg(test)]\nmod tests") {
            Some(i) => &text[..i],
            None => text.as_str(),
        };
        let mut violations = Vec::new();
        let mut prev_trimmed = "";
        for (lineno, line) in prod.lines().enumerate() {
            let t = line.trim();
            let prev = prev_trimmed;
            prev_trimmed = t;
            if t.starts_with("//") {
                continue;
            }
            // 覆盖**全部**会真正拉起子进程的 std 入口：`spawn()` / `output()` /
            // `status()` / `wait_with_output()`（后两者易被漏记——2026-09-10 审核补齐）。
            let bare = t.contains(".spawn()")
                || t.contains(".output()")
                || t.contains(".status()")
                || t.contains("wait_with_output()");
            if !bare {
                continue;
            }
            // 用 lifecycle 的写法不会出现 `.spawn()` 直接挂在 Command 上的形态。
            if t.contains("lifecycle::spawn(")
                || t.contains("lifecycle::run(")
                || t.contains("lifecycle::wait_with_output(")
            {
                continue;
            }
            // 豁免标注可写在本行（行尾）或**紧邻上一行**（后者更贴近 `#[allow(...)]` 惯例）。
            // （注：本文件自己的豁免走 `SPAWN_EXEMPTIONS` 集中表；行内标注保留给
            //  其它文件里那些「函数级粒度太粗」「极个别」的场合，如 `updater.rs` 的
            //  `reqwest::Error::status()`——那是读状态码，不拉起任何进程。）
            if t.contains("spawn-gate: exempt(") || prev.contains("spawn-gate: exempt(") {
                continue;
            }
            violations.push(format!("{}:{}: {}", name, lineno + 1, t));
        }
        violations
    }

    /// 机器闸门：生产源码里的 `Command` spawn 必须经 `lifecycle::spawn`/`run`。
    ///
    /// 为什么要有这条闸门：本模块的全部价值建立在"**没有漏网 spawn**"上——
    /// 漏一处，那一处的孤儿就不受清扫管辖（契约 §3.4 原话）。而"自觉遵守"
    /// 在 19 个调用点的规模上必然腐化。此测试从源码文本判定，改回去即红。
    ///
    /// **扫描面覆盖 seam 的属主文件（2026-09-11，task-30）**：`src/lifecycle.rs`
    /// 原先**不在 `SOURCES` 里**，于是往本文件新加裸 spawn 对机器完全不可见——
    /// `reap_pid` 的裸 `taskkill` 就是这么漏了两个月。现纳入扫描面，
    /// 本文件自己的合法 spawn 走 `SPAWN_EXEMPTIONS` **条目级**豁免（不是整文件排除，
    /// 那正是盲区成因）。
    ///
    /// 豁免（必须是**可解释**的枚举，不是"随手加豁免"）：
    /// - `#[cfg(test)]` 内的代码：测试要故意制造裸 spawn 做对照；
    /// - `SPAWN_EXEMPTIONS` 里逐条论证的 seam 本体函数；
    /// - 显式标注 `// spawn-gate: exempt(<理由>)` 的行（行内标注，留给其它文件的极个别场合）。
    #[test]
    fn production_spawns_go_through_lifecycle_seam() {
        const SOURCES: &[(&str, &str)] = &[
            ("src/boot.rs", include_str!("boot.rs")),
            ("src/commands/window.rs", include_str!("commands/window.rs")),
            ("src/engines.rs", include_str!("engines.rs")),
            ("src/executor.rs", include_str!("executor.rs")),
            ("src/ipc.rs", include_str!("ipc.rs")),
            // seam 的属主文件自身（2026-09-11 补入，此前是闸门盲区）。
            ("src/lifecycle.rs", include_str!("lifecycle.rs")),
            ("src/plugins.rs", include_str!("plugins.rs")),
            ("src/profiles.rs", include_str!("profiles.rs")),
            ("src/resolve.rs", include_str!("resolve.rs")),
            ("src/sessions.rs", include_str!("sessions.rs")),
            ("src/shell.rs", include_str!("shell.rs")),
            ("src/updater.rs", include_str!("updater.rs")),
            ("src/updates.rs", include_str!("updates.rs")),
            ("src/lib.rs", include_str!("lib.rs")),
        ];
        let violations: Vec<String> = SOURCES
            .iter()
            .flat_map(|(name, text)| scan_unguarded_spawns_with(name, text, SPAWN_EXEMPTIONS))
            .collect();
        assert!(
            violations.is_empty(),
            "以下生产代码绕过了 lifecycle 守卫 seam（子进程会成为不受清扫管辖的孤儿）：\n{}\n\
             请改用 `crate::lifecycle::spawn` / `crate::lifecycle::run`；确需豁免时\
             ① 在 `SPAWN_EXEMPTIONS` 加一条**条目级**豁免（函数名 + 理由 + 日期，\
             并写明摘掉它应报出几行），或 ② 在本行/紧邻上一行标注 \
             `// spawn-gate: exempt(<理由>)`。",
            violations.join("\n")
        );
    }

    // ---------- Windows：Job Object（ADR-0015 §3 方案 A） ----------

    /// Windows 上 job 必须装配成功，且 `Role` 语义与 unix 侧一致（平台无关部分）。
    ///
    /// 注意：本测试跑在 Windows 上（CI 或在 mac 上的交叉 `cargo check` 只验类型），
    /// 断言的是"job 创建 + 限制设置"这条链在真实内核上可用——它是硬杀收口的
    /// 唯一依赖。赋值（`assign`）在壳自身被外层 job 收编时可能失败，那属已知
    /// 边界（ADR-0015 §5），故此处只断言 job 自身就绪。
    #[cfg(windows)]
    #[test]
    fn windows_job_object_is_ready_for_kill_on_close() {
        assert!(
            job::is_ready(),
            "Job Object 未装配成功——Windows 硬杀收口会退化为仅启动期清扫"
        );
    }

    /// 角色集合在 Windows 上同样只给长命角色配生命线语义（`needs_lifeline`
    /// 在 Windows 不生效，但 `Role` 分类仍用于清扫留痕与优先级）。
    #[test]
    fn role_taxonomy_is_platform_independent() {
        // 这条与 `lifeline_only_covers_state_holding_roles` 互补：那条管"配不配
        // watcher"，这条钉住分类本身不随平台漂移（清扫留痕字段依赖它）。
        assert_eq!(Role::DshServer.as_str(), "dsh-server");
        assert_eq!(Role::DshWsl.as_str(), "dsh-wsl");
        assert_eq!(Role::DshCli.as_str(), "dsh-cli");
        assert_eq!(Role::Pnpm.as_str(), "pnpm");
        assert_eq!(Role::Probe.as_str(), "probe");
    }

    /// 生命线写端必须**全程持有**：它是 RAII 载体，被 drop 会让 watcher 误判
    /// 壳已死并收口正常运行的子进程（灾难性误杀）。
    #[cfg(unix)]
    #[test]
    fn lifeline_write_end_is_held_for_process_lifetime() {
        ensure_global_for_tests();
        let g = global().expect("测试应已装配 GLOBAL");
        use std::os::fd::AsRawFd;
        assert!(
            g.lifeline_write.as_raw_fd() >= 0,
            "写端必须活着（fd 有效）——它一旦被关闭，全部 watcher 会立刻收到 EOF"
        );
        // 再取一次仍应有效（证明没有在别处被 take/drop）。
        let again = global().expect("GLOBAL 应常驻");
        assert_eq!(
            again.lifeline_write.as_raw_fd(),
            g.lifeline_write.as_raw_fd(),
            "写端句柄应稳定常驻，不得被替换或释放"
        );
    }

    // ---------- 审核修正：锁判定的安全方向（2026-09-10） ----------

    /// `TryLockError` 有两档，**必须分开**：只有 `WouldBlock` 才是"真被持有"。
    /// `Error(_)`（文件系统不支持 flock / I/O 故障）属**无法判定**——首版把它
    /// 当成"仍被持有"，会在不支持锁的盘上把每条登记都判成活跃孤儿并按文件里的
    /// pid 去杀（pid 可能已复用），正是 ADR-0015 要避免的误杀。
    #[test]
    fn lock_verdict_separates_would_block_from_io_error() {
        use std::fs::TryLockError;

        assert_eq!(
            classify_lock_result(&Ok(())),
            RegistrationVerdict::Stale,
            "加锁成功 = 持有者已死"
        );
        assert_eq!(
            classify_lock_result(&Err(TryLockError::WouldBlock)),
            RegistrationVerdict::LiveOrphan,
            "明确被持有 = 真孤儿"
        );
        // 关键：I/O 错误**不得**升级为 LiveOrphan（那会导致误杀）
        assert_eq!(
            classify_lock_result(&Err(TryLockError::Error(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "flock unsupported"
            )))),
            RegistrationVerdict::Indeterminate
        );
        assert_eq!(
            classify_lock_result(&Err(TryLockError::Error(std::io::Error::other("io")))),
            RegistrationVerdict::Indeterminate
        );
    }

    /// 不变量：判定为 `LiveOrphan` 的**唯一**来源是 `WouldBlock`。
    #[test]
    fn only_would_block_may_authorize_reaping() {
        use std::fs::TryLockError;
        let cases: Vec<Result<(), TryLockError>> = vec![
            Ok(()),
            Err(TryLockError::WouldBlock),
            Err(TryLockError::Error(std::io::Error::other("x"))),
            Err(TryLockError::Error(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "denied",
            ))),
            Err(TryLockError::Error(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "gone",
            ))),
        ];
        for c in cases {
            let reaps = classify_lock_result(&c) == RegistrationVerdict::LiveOrphan;
            if reaps {
                assert!(
                    matches!(c, Err(TryLockError::WouldBlock)),
                    "只有 WouldBlock 才允许收口，实际：{c:?}"
                );
            }
        }
    }

    /// **端到端复刻用户事故（真 dsh 版，需已装引擎，故 `#[ignore]`）**：
    /// 走**生产同一条** `spawn_dsh` 起一个真的 dsh，然后对壳进程发 `SIGKILL`
    /// （不留任何跑清理代码的机会），断言真 dsh 被生命线收口。
    ///
    /// 与 `guarded_child_dies_when_parent_is_sigkilled` 的分工：那条用 `sleep`
    /// 替身验**机制**（快、无依赖、进默认套件）；本用例验**真身**——真的 dsh
    /// 进程、真的 LaunchSpec、真的 spawn_dsh 装配链，正是"会话被孤儿占死"现场。
    ///
    /// 跑法：`cargo test --lib real_dsh_is_reaped_when_shell_is_sigkilled -- --ignored --nocapture`
    #[cfg(unix)]
    #[test]
    #[ignore = "需要本机已装引擎（engines/bin）；CI 与无引擎环境跳过"]
    fn real_dsh_is_reaped_when_shell_is_sigkilled() {
        // 无引擎（CI / 干净机器）时优雅跳过——本用例是"真机锚"，不进默认套件。
        if crate::engines::engine_dsh_bin(&crate::resolve::launch_data_dir_for_test()).is_none() {
            eprintln!("跳过：本机引擎未就绪");
            return;
        }
        let dir = scratch("realdsh");
        let pid_file = dir.join("child.pid");
        let exe = std::env::current_exe().expect("测试二进制路径");

        let mut parent = Command::new(&exe)
            .args([
                "--exact",
                "lifecycle::tests::parent_role_probe",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("DSH_DOCK_LIFECYCLE_PARENT", "1")
            .env("DSH_DOCK_LIFECYCLE_REAL_DSH", "1")
            .env("DSH_DOCK_LIFECYCLE_DIR", &dir)
            .env("DSH_DOCK_LIFECYCLE_PIDFILE", &pid_file)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("唤起父角色进程");

        assert!(
            wait_until(Duration::from_secs(30), || pid_file.is_file()),
            "父角色未在 30s 内落盘真 dsh 的 pid"
        );
        let dsh_pid: u32 = std::fs::read_to_string(&pid_file)
            .unwrap()
            .trim()
            .parse()
            .expect("dsh pid");
        let parent_pid = parent.id();

        // 前置：真 dsh 起来了、父还活着。
        assert!(alive(dsh_pid), "真 dsh 应已启动（pid={dsh_pid}）");
        assert!(alive(parent_pid), "父角色应仍在运行");

        // 事故时刻：硬杀壳，且**不给它任何清理机会**。
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;
            kill(Pid::from_raw(parent_pid as i32), Signal::SIGKILL).expect("SIGKILL 父角色");
        }
        let _ = parent.wait();

        // 生命线必须在 grace 内收口真 dsh。
        assert!(
            wait_until(Duration::from_secs(15), || !alive(dsh_pid)),
            "壳被 SIGKILL 后真 dsh 仍存活（pid={dsh_pid}）——生命线未生效，\
             用户会话仍会被这个孤儿占死"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---------- 闸门：引导必须真的调用清扫（否则用户的事故会静默复发） ----------

    /// 机器闸门：`lib.rs` 的启动线程必须在**派发执行器之前**调用
    /// `lifecycle::sweep_orphans`。
    ///
    /// 为什么需要它：`sweep_orphans` 本身有完整单测，但"引导到底调没调它"没有任何
    /// 测试覆盖——2026-09-10 审核用变异测试证实：把这一行删掉，**全部测试照样绿**。
    /// 而这一行一旦丢失，用户报的「会话打不开、只能删会话」就会**静默复发**（存量
    /// 孤儿没人回收）。引导路径依赖完整 Tauri App，无法在单测里跑，故照仓库既有
    /// 惯例用**源码文本闸门**钉住（同 `production_spawns_go_through_lifecycle_seam`
    /// 与 `background_color_matches_theme_token`）。
    ///
    /// 同时钉住**顺序**：清扫必须在 `executor_for_mode`（→ probe → 建会话）之前，
    /// 否则可能与新一代探测竞争同一份 profile 目录（契约 §3.3）。
    #[test]
    fn boot_sweeps_orphans_before_dispatching_executor() {
        let src = include_str!("lib.rs");
        let sweep_at = src
            .find("lifecycle::sweep_orphans(")
            .expect("引导路径必须调用 lifecycle::sweep_orphans——缺了它，上一代壳遗留的孤儿不会被回收，用户会话会被永久占死");
        let dispatch_at = src
            .find("executor_for_mode(")
            .expect("引导路径应有 executor_for_mode 派发点");
        assert!(
            sweep_at < dispatch_at,
            "清扫必须在派发执行器（probe/建会话）之前——否则可能与新一代探测竞争同一份 profile 目录"
        );
    }

    // ---------- 闸门的行尾健壮性（windows-latest CI 实测踩坑） ----------

    /// **扫描器必须对行尾不敏感**：仓库无 `.gitattributes`，Git for Windows 的
    /// `autocrlf` 会把源码 checkout 成 CRLF，`include_str!` 于是含 `\r\n`。
    /// 首版模式写死 `"\n#[cfg(test)]\nmod tests"`，CRLF 下永不匹配 → 截断失效 →
    /// 测试模块里的裸 spawn 被误报 → **闸门在 Windows 上恒红**（CI 实测）：
    /// 它报出的"违规"全是 `src/shell.rs` 测试模块里的 `Command::new("sleep")`。
    #[test]
    fn spawn_gate_is_line_ending_agnostic() {
        let lf = "fn a() {\n    cmd.spawn();\n}\n#[cfg(test)]\nmod tests {\n    fn t() {\n        Command::new(\"sleep\").spawn().unwrap();\n    }\n}\n";
        let crlf = lf.replace('\n', "\r\n");

        // 生产段的裸 spawn 两种行尾都要报
        for (label, text) in [("LF", lf), ("CRLF", crlf.as_str())] {
            let v = scan_unguarded_spawns("x.rs", text);
            assert_eq!(v.len(), 1, "{label}: 生产段应报 1 条，实际 {v:?}");
            assert!(v[0].contains("cmd.spawn()"), "{label}: 应报生产段那条");
        }
        // 测试模块不得被报（这正是 CRLF 下失效的那一步）
        for (label, text) in [("LF", lf), ("CRLF", crlf.as_str())] {
            let v = scan_unguarded_spawns("x.rs", text);
            assert!(
                !v.iter().any(|x| x.contains("sleep")),
                "{label}: 测试模块的裸 spawn 不该被报（CRLF 若未归一即会误报）"
            );
        }
    }

    /// **豁免表自检（2026-09-11，task-30）**：每条豁免都必须**真的在挡东西**，
    /// 且**不多不少**。
    ///
    /// 为什么需要：豁免表的最大腐化方式不是"漏加"，而是**过宽**——一条吞掉邻接函数的
    /// 豁免，会把别处的裸 spawn 静默放行（正是 `lifecycle.rs` 曾长期不在扫描面里的
    /// 同一种失效）。故此处不仅断言「非空」，还断言**精确等于登记的行数**：
    /// 区间偏小 → 计数变大而红；区间偏大 → 计数变小而红。两侧都拦。
    #[test]
    fn spawn_gate_exemptions_are_live() {
        for e in SPAWN_EXEMPTIONS {
            let text = read_source(e.file);
            let hits = scan_unguarded_spawns_with(e.file, &text, &table_without(e));
            assert_eq!(
                hits.len(),
                e.hits,
                "豁免条目 {} `fn {}` 摘掉后应恰好报出 {} 处（非空 = 仍在使用；\
                 恰好 = 区间没有过宽），实际 {} 处：{hits:?}",
                e.file,
                e.item,
                e.hits,
                hits.len()
            );
            let with = scan_unguarded_spawns_with(e.file, &text, SPAWN_EXEMPTIONS);
            assert!(
                with.is_empty(),
                "豁免条目 {} `fn {}` 未生效，仍报出 {with:?}",
                e.file,
                e.item
            );
        }
    }

    /// 豁免条目必须**可解释**（理由 + 日期），且理由要说明「为什么必须绕过 seam」。
    #[test]
    fn spawn_gate_exemption_entries_are_well_formed() {
        for e in SPAWN_EXEMPTIONS {
            assert!(
                e.reason.len() >= 12,
                "{} `fn {}` 的理由太短（必须能论证「这个函数就是 seam 本体」）：{:?}",
                e.file,
                e.item,
                e.reason
            );
            assert!(
                e.reason.contains("seam")
                    || e.reason.contains("清理器")
                    || e.reason.contains("清扫"),
                "{} `fn {}` 的理由须点明它与 seam 的关系：{:?}",
                e.file,
                e.item,
                e.reason
            );
            assert_eq!(e.date.len(), 10, "{} 的日期应为 YYYY-MM-DD", e.file);
            assert_eq!(
                e.date.chars().filter(|c| *c == '-').count(),
                2,
                "{} 的日期格式应为 YYYY-MM-DD：{:?}",
                e.file,
                e.date
            );
            assert!(e.hits > 0, "{} `fn {}` 的 hits 必须为正", e.file, e.item);
        }
        // 同一个 (文件, 函数) 不得重复登记（重复会掩盖「哪条在起作用」）。
        for (i, a) in SPAWN_EXEMPTIONS.iter().enumerate() {
            for b in SPAWN_EXEMPTIONS.iter().skip(i + 1) {
                assert!(
                    !(a.file == b.file && a.item == b.item),
                    "豁免表有重复条目：{} `fn {}`",
                    a.file,
                    b.item
                );
            }
        }
    }

    /// **条目级豁免不得泄漏到同文件的其它函数**（这正是"整文件排除"被弃用的理由）。
    #[test]
    fn spawn_gate_item_exemption_does_not_leak_to_rest_of_file() {
        const SYNTHETIC: &str = "\
fn exempt_one() { let _ = std::process::Command::new(\"x\").spawn(); }\n\
fn not_exempt() { let _ = std::process::Command::new(\"y\").status(); }\n";
        let table = [SpawnExemption {
            file: "src/synthetic_exempt.rs",
            item: "exempt_one",
            hits: 1,
            reason: "测试用：seam 本体论证",
            date: "2026-09-11",
        }];
        let hits = scan_unguarded_spawns_with("src/synthetic_exempt.rs", SYNTHETIC, &table);
        assert_eq!(
            hits.len(),
            1,
            "条目级豁免泄漏到了整文件（会把邻接函数的违规一起放行）：{hits:?}"
        );
        assert!(hits[0].contains("not_exempt"), "实际：{hits:?}");
    }

    /// 区间定位器必须**不被字面量/注释里的花括号骗到**。
    ///
    /// 被骗到的后果不对称：区间偏小 → 误报（可见红）；区间偏大 → **吞掉邻接函数的违规**
    /// （静默漏报）。故两侧都测：字符串 / 字符字面量 / 原始字符串 / 行注释 / 块注释里的
    /// 花括号，都不得改变 `fn` 区间。
    #[test]
    fn spawn_gate_extent_finder_ignores_braces_in_literals_and_comments() {
        const SYNTHETIC: &str = r##"
fn exempt_one() {
    let _json = "{\"a\":1}";
    let _close = '}';
    let _raw = r#"{"nested":"}"}"#;
    // 行注释里的 } 不是结构
    /* 块注释里的 { 与 } 也不是结构 */
    let _ = std::process::Command::new("x").spawn();
}

fn not_exempt() { let _ = std::process::Command::new("y").status(); }
"##;
        let table = [SpawnExemption {
            file: "src/synthetic_braces.rs",
            item: "exempt_one",
            hits: 1,
            reason: "测试用：seam 本体论证",
            date: "2026-09-11",
        }];
        let hits = scan_unguarded_spawns_with("src/synthetic_braces.rs", SYNTHETIC, &table);
        assert_eq!(
            hits.len(),
            1,
            "字面量/注释里的花括号打穿了区间定位：{hits:?}"
        );
        assert!(hits[0].contains("not_exempt"), "实际：{hits:?}");

        // 反过来：摘掉豁免必须报出被豁免函数里的那一处（证明区间确实覆盖到它）。
        let bare = scan_unguarded_spawns_with("src/synthetic_braces.rs", SYNTHETIC, &[]);
        assert_eq!(bare.len(), 2, "无豁免时应报出两处：{bare:?}");
    }

    /// **单向性证明（本闸门的核心）**：给一份**形如本文件**的源码注入一处
    /// **未豁免函数**里的裸 spawn，闸门必须红且**精确报行**。
    ///
    /// 这是"闸门真的会拦"的合成证明；另有对**真实文件**的变异实测（见报告），
    /// 二者互补：本用例进默认套件（无副件、可回归），真实变异由复核者重演。
    #[test]
    fn spawn_gate_reports_bare_spawn_in_non_exempt_function() {
        // 摘掉本文件的全部豁免，再把 `SPAWN_EXEMPTIONS` 认识的那个函数名换成别的，
        // 等价于"往生命周期模块里加了一个不豁免的新函数"。
        const SYNTHETIC: &str = "\
fn role_from_str(s: &str) -> u8 {\n\
    let _ = std::process::Command::new(\"evil\").spawn();\n\
    0\n\
}\n";
        let hits = scan_unguarded_spawns_with("src/lifecycle.rs", SYNTHETIC, SPAWN_EXEMPTIONS);
        assert_eq!(hits.len(), 1, "未豁免函数里的裸 spawn 必须被拦：{hits:?}");
        assert!(
            hits[0].starts_with("src/lifecycle.rs:2:"),
            "必须精确报出行号：{hits:?}"
        );
    }

    /// 字节原始字符串（`br"…"`）必须被词法器识别——**这是实现期自测抓出的真 bug**：
    /// 首版把识符检查放在推进下标**之后**，`b[p-1]` 恰是那个 `b`，判据恒假 ⇒
    /// `br"…"` 从未被识别，其内部花括号会被当成结构字符（区间偏大 = 静默漏报）。
    #[test]
    fn spawn_gate_extent_finder_handles_byte_raw_strings() {
        const SYNTHETIC: &str = "fn exempt_one() {\n    let _ = br\"}\";\n    let _ = std::process::Command::new(\"x\").spawn();\n}\n\nfn not_exempt() { let _ = std::process::Command::new(\"y\").status(); }\n";
        let table = [SpawnExemption {
            file: "src/synthetic_br.rs",
            item: "exempt_one",
            hits: 1,
            reason: "测试用：seam 本体论证",
            date: "2026-09-11",
        }];
        // 若 `br"}"` 未被识别，其中的 `}` 会提前闭合 `exempt_one`，
        // 于是豁免区间偏小 → 报出 2 处而非 1 处。
        let hits = scan_unguarded_spawns_with("src/synthetic_br.rs", SYNTHETIC, &table);
        assert_eq!(
            hits.len(),
            1,
            "`br\"…\"` 未被词法器识别（花括号打穿了区间）：{hits:?}"
        );
        assert!(hits[0].contains("not_exempt"), "实际：{hits:?}");
    }

    /// 行尾：同一份源码 LF 与 CRLF 的判定必须**逐条相同**（Windows `autocrlf` 教训）。
    #[test]
    fn spawn_gate_crlf_and_lf_are_judged_identically() {
        const LF: &str = "\
fn a() { let _ = c.spawn(); }\n\
\n\
#[cfg(test)]\n\
mod tests {\n\
    fn t() { let _ = d.spawn(); }\n\
}\n";
        let crlf = LF.replace('\n', "\r\n");
        let a = scan_unguarded_spawns("src/x.rs", LF);
        let b = scan_unguarded_spawns("src/x.rs", &crlf);
        assert_eq!(a.len(), 1, "LF 下应只报生产段那一条：{a:?}");
        assert_eq!(
            a.iter()
                .map(|s| s.split(':').nth(1).map(str::to_string))
                .collect::<Vec<_>>(),
            b.iter()
                .map(|s| s.split(':').nth(1).map(str::to_string))
                .collect::<Vec<_>>(),
            "CRLF 与 LF 必须报出相同行号：LF={a:?} CRLF={b:?}"
        );
    }

    /// **`taskkill` 参数构造（E2，2026-09-11 / task-34）**：本模块自己的那份现已被测。
    ///
    /// 此前 `shell::windows_kill_args` 与 `reap_pid` 各写一份同形 `vec!`，
    /// **只有 shell 侧被测**——清扫回退路径的参数构造从未被断言。现已委托同源
    /// （`shell::windows_kill_args` → 本函数），本用例是该唯一实现的直接断言。
    ///
    /// **跨平台跑**（不 `#[cfg(windows)]`）：本函数是纯函数，在 macOS/Linux 同样可测，
    /// 且 Windows 专用代码在宿主上**根本不编译**（AGENTS §1 的 clippy 漏检教训），
    /// 故必须让它在宿主上也受测试管辖。
    #[test]
    fn reap_args_cover_whole_tree() {
        let args = reap_args(4321);
        assert_eq!(args, vec!["/PID", "4321", "/T", "/F"]);
        // 语义要点：`/T` 少不得——退回「只杀壳层、node 继续跑」的老漏洞（ADR-0014）。
        assert!(args.iter().any(|a| a == "/T"), "必须覆盖整棵进程树");
        assert!(args.iter().any(|a| a == "/F"), "必须强制结束");
        // pid 必须**逐字**出现在自己的位置上（防拼串错误）。
        assert_eq!(args[1], "4321");
        // 与参数个数绑定的负例：多一个/少一个都说明构造被改动了。
        assert_eq!(args.len(), 4, "taskkill 参数个数固定为 4");
    }

    /// `reap_args` 的**依赖纪律**闸门：本模块不得反向依赖 `shell`（成环）。
    ///
    /// 为什么用源码文本判据：这是**架构约束**而非行为约束——一旦有人把
    /// `crate::shell::windows_kill_args` 抄进来"消除重复"，编译**照样通过**、
    /// 测试**照样绿**，但会形成 `lifecycle ↔ shell` 环并破坏 spawn 闸门把本文件
    /// 纳入扫描面的单向性前提（task-30 的地基）。故只能从源码文本拦。
    ///
    /// 范围说明：本条只钉**依赖方向**（E2 的硬约束）。契约 §3.1「单向语义」里
    /// 「守卫不得反向影响壳存活/重启」的完整方向哨兵属 task-29 §4 Tier 1，
    /// **是独立意图，不在本任务内**——此处不越界实现，只保留本模块的依赖纪律。
    #[test]
    fn lifecycle_does_not_depend_on_state_holding_sibling_modules() {
        // 行尾归一：任何以源码文本为判据的闸门都必须自己扛住 CRLF（v1.1.1 教训）。
        let src = include_str!("lifecycle.rs").replace("\r\n", "\n");
        let prod = match src.find("\n#[cfg(test)]\nmod tests") {
            Some(i) => &src[..i],
            None => &src[..],
        };
        // **必须跳过注释行**（2026-09-11 自测发现）：本模块的纪律注释里**必然**写着
        // 「不得改调 `crate::shell::*`」——若按整段文本匹配，闸门会**被自己的文档触发**
        // （首版即如此，实测红）。这同时是"文本判据必须做词法/注释处理"的又一实例。
        let code_lines: Vec<&str> = prod
            .lines()
            .map(str::trim)
            .filter(|l| !l.starts_with("//"))
            .collect();
        for forbidden in ["crate::shell::", "crate::boot::", "crate::ui::"] {
            assert!(
                !code_lines.iter().any(|l| l.contains(forbidden)),
                "lifecycle.rs 生产代码出现 `{forbidden}`（注释除外）——本模块不得依赖\
                 持有壳状态的兄弟模块：会与 shell → lifecycle 形成模块环，并破坏 spawn\
                 闸门单向性证明的地基。确需共用逻辑时，把它下沉到本模块或 crate 根\
                 （如 `crate::child_cmd`）。"
            );
        }
        // 白名单：crate 根的 child_cmd 允许（不成环），正向钉住防被误删。
        assert!(
            code_lines.iter().any(|l| l.contains("crate::child_cmd(")),
            "reap_pid 的 Windows 回退路径应经 `crate::child_cmd`（AGENTS §4.1 防闪窗）"
        );
    }

    /// 扫描器的既有语义不能因归一而丢：豁免标注（本行 / 紧邻上一行）与
    /// lifecycle 写法仍须被识别。
    #[test]
    fn spawn_gate_still_honours_exemptions_and_seam() {
        let cases = [
            "fn a() {\n    lifecycle::spawn(&mut c, R, x)?;\n}\n",
            "fn a() {\n    lifecycle::run(&mut c, R, x)?;\n}\n",
            "fn a() {\n    // spawn-gate: exempt(理由)\n    cmd.output();\n}\n",
            "fn a() {\n    cmd.output(); // spawn-gate: exempt(理由)\n}\n",
            "fn a() {\n    // 注释里的 cmd.spawn() 不算\n}\n",
        ];
        for c in cases {
            assert!(
                scan_unguarded_spawns("x.rs", c).is_empty(),
                "不应报违规：{c:?}"
            );
        }
    }
}
