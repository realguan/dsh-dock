//! 运行 dsh 子进程：spawn（`--port 0`）→ 从日志轮询实际 URL → 优雅停止。
//!
//! 进程监护语义：
//!   - `--port 0` 由 OS 分配端口，URL 从 dsh 打在 stdout 的地址行解析；
//!   - 只认 `http://` / `https://` 开头的词，拒绝 `file://`（Node 栈帧）与 `data:`；
//!   - 优雅停止 = SIGTERM → 等待 grace → SIGKILL 兜底（unix；Windows 用 kill）。
//!
//! 与启动器/持久服务的差异：**产品壳与 dsh 严格同生命周期**（壳退 = dsh 停），
//! 不需要独立进程组、也不需要 stdout/stderr 与壳解耦——日志文件每个启动周期
//! 截断重建，URL 探测从文件头开始即可，省掉跨启动偏移逻辑。

use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::resolve::LaunchSpec;

/// 一个被产品壳托管的 dsh 子进程。
pub struct DshProcess {
    pub child: Child,
    pub log_path: PathBuf,
}

/// 确保 dsh 的用户数据目录可用。
///
/// dsh 本身会在首次启动时写入 profiles/storage；桌面壳不能把“首次使用时
/// 目录尚未存在”误判成宿主缺失。若路径是普通文件，`create_dir_all` 会返回
/// 明确错误，交给上层错误卡展示。
fn ensure_dsh_home(dsh_home: &Path) -> Result<()> {
    if dsh_home.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(dsh_home)
        .with_context(|| format!("创建 DSH_HOME 目录 {}", dsh_home.display()))?;
    Ok(())
}

/// 启动 dsh：`<node> <dsh-bin.js> --profile <p> --port 0`，`DSH_HOME` 来自
/// LaunchSpec（system=用户 home；bundle=兜底副本 home）。stdout/stderr
/// 进数据目录日志文件（可排查故障）。
pub fn spawn_dsh(launch: &LaunchSpec, data_dir: &Path) -> Result<DshProcess> {
    let node_bin = &launch.node_bin;
    let dsh_bin = &launch.dsh_bin_js;
    let dsh_home = &launch.dsh_home;

    // 解析出的宿主零部件缺一不可：慢一点把错误讲清楚，别让 node 裸奔报「command not found」。
    if !node_bin.is_file() {
        anyhow::bail!("Node 可执行文件不存在: {}", node_bin.display());
    }
    if !dsh_bin.is_file() {
        anyhow::bail!("dsh 入口不存在: {}", dsh_bin.display());
    }
    ensure_dsh_home(dsh_home)?;

    std::fs::create_dir_all(data_dir).context("创建数据目录")?;
    let log_path = data_dir.join("dsh-shell.log");
    // 每次启动重建：本进程一进程一日志，不跨启动累积（无历史偏移问题）。
    let log = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&log_path)
        .with_context(|| format!("打开日志 {}", log_path.display()))?;

    let mut cmd = crate::child_cmd(node_bin);
    cmd.arg(dsh_bin)
        .arg("--profile")
        .arg(&launch.profile)
        .arg("--port")
        .arg("0");
    // 桌面壳接管呈现：禁止 dsh 自开系统浏览器——但仅当该版本支持
    // （system 档旧版 dsh 如 rc.5 收到未知参数会直接秒退，须按版本适配）。
    if launch.no_open {
        cmd.arg("--no-open");
    }
    cmd.env("DSH_HOME", dsh_home)
        // 用户环境感知：dsh 世界运行在用户 PATH 上；download 档还要把私有
        // Node 放在首位，确保 dsh 拉起 helper 时不会找错执行器。
        .env(
            "PATH",
            crate::resolve::path_with_bin(node_bin, &crate::resolve::effective_path()),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone().context("克隆日志句柄")?))
        .stderr(Stdio::from(log));

    let child = cmd
        .spawn()
        .with_context(|| format!("spawn {}", node_bin.display()))?;
    tracing::info!(
        "dsh 已启动：pid={} profile={} tier={:?}",
        child.id(),
        launch.profile,
        launch.tier
    );
    Ok(DshProcess { child, log_path })
}

/// 启动等待的三种结局：
/// - `Ready(url)`：日志出现访问地址，dsh 已就绪；
/// - `Exited(code)`：**dsh 进程先退出了**——不再干等，直接判失败；
/// - `Stalled`：进程还活着但日志长时间无进展（疑似卡死）或到硬上限。
#[derive(Debug)]
pub enum ReadyOutcome {
    Ready(String),
    Exited(i32),
    Stalled,
}

/// 进程存活感知的就绪等待（2026-08-24 裁定，Windows 冷启动慢场景）：
/// 旧 `detect_url` 的死等上限在 Windows 上会被 Defender 首扫/Node 冷加载吃掉，
/// 20s 常不够（实测点 2 次重试才起）。这里改成：
///   1. 硬上限 `limit`（宽松，默认 90s）——到点仍未就绪判 `Stalled`；
///   2. **dsh 进程中途退出 → 立即 `Exited`**（真失败秒报，不干等满上限）；
///   3. 进程活着但日志 `stall` 内无进展 → `Stalled`（防死等，提示卡死）。
///
/// 双路就绪源（2026-08-26 裁定，WSL 缓冲兜底）：
///   - **marker 优先**（`marker` 闭包）——WSL 用，GUEST_BOOT 把 dsh 输出
///     `tee` 到客体内哨兵文件，shell 经 `wsl.exe -e cat` 直读，绕开 wsl.exe
///     stdout 转发的内部缓冲（实测：URL 不出现直到 wsl.exe 退出）。命中即返
///     回 Ready，不等 log 文件。本地执行器传 `&mut (|| None)` 跳过此路。
///   - **log 兜底**——`executor::log_path()` 路径。本机或任何无 marker 的执行
///     环境走这条；UTF-16LE 自动探测解码。
///
/// 执行环境无关（executor.rs）：会话进程是否退出经 `exited` 回调查询（壳侧
/// 对 `Session` 槽做短锁轮询）。调用方只传回"已退出 → Some(exit_code)"，
/// 等待线程不独占会话锁 90s，退出处理器随时能拿到会话做 teardown。
pub fn wait_for_ready(
    log_path: &Path,
    exited: &mut dyn FnMut() -> Option<i32>,
    marker: &mut dyn FnMut() -> Option<String>,
    stall: Duration,
    limit: Duration,
) -> ReadyOutcome {
    let deadline = Instant::now() + limit;
    let mut scanned = 0usize;
    let mut last_grow = Instant::now();
    loop {
        // marker 优先：直读客体内哨兵文件，绕开 wsl.exe stdout 缓冲。命中即 Ready。
        if let Some(text) = marker() {
            if let Some(url) = parse_detected_url(&text) {
                return ReadyOutcome::Ready(url);
            }
        }
        // 主路径：log 文件轮询（本地执行器 / WSL 兜底）。wsl.exe 重定向日志
        // 可能是 UTF-16LE（NUL 间隔）——统一经 read_log_auto 解码（探测 NUL/BOM，
        // 2026-08-26 实机 bug：UTF-16LE 下 URL 词是 `\x00h\x00t\x00t\x00p\x00`
        // 间隔，starts_with("http://") 永远失败）。
        let text = crate::resolve::read_log_auto(log_path);
        if !text.is_empty() {
            let start = floor_char_boundary(&text, scanned);
            if scanned < start {
                scanned = start;
            }
            if text.len() > scanned && text.is_char_boundary(scanned) {
                if let Some(url) = parse_detected_url(&text[scanned..]) {
                    return ReadyOutcome::Ready(url);
                }
                scanned = text.len();
                last_grow = Instant::now();
            }
        }
        // 会话进程先退出：不干等，立即判失败（短锁回调，不阻塞退出处理器）。
        if let Some(code) = exited() {
            return ReadyOutcome::Exited(code);
        }
        if Instant::now() >= deadline {
            break;
        }
        // 进程活着但长时间没有任何新日志：疑似卡死，提示用户而不是继续死等。
        if last_grow.elapsed() >= stall {
            return ReadyOutcome::Stalled;
        }
        thread::sleep(Duration::from_millis(50));
    }
    ReadyOutcome::Stalled
}

/// 只接受 `http://` / `https://` 开头的词，去掉尾部 `/`/`,`/`;`。
/// 拒绝 `file://`（Node 加载 bundle 的栈帧）与 `data:` 等，防止把报错路径
/// 当访问地址（移植自启动器 process_guard，含回归测试）。
/// 把字节下标回退到最近的 UTF-8 字符边界（str::floor_char_boundary 的
/// MSRV 平替：该 API 1.91 才稳定，本壳 rust-version = 1.77.2，2026-08-27）。
fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    if index >= text.len() {
        return text.len();
    }
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

pub fn parse_detected_url(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|w| w.starts_with("http://") || w.starts_with("https://"))
        .map(|url| url.trim_end_matches(['/', ',', ';']).to_string())
}

/// 优雅停止 dsh：unix 发 SIGTERM 等待 grace，超时 SIGKILL；Windows 直接 kill。
/// dsh 对 SIGTERM 以 exit 0 收尾，正常路径秒退。
/// （Windows 分支不使用 grace 参数，故按平台允许未用变量。）
#[cfg_attr(not(unix), allow(unused_variables))]
pub fn stop_dsh(child: &mut Child, grace: Duration) -> i32 {
    #[cfg(unix)]
    {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;
        let pid = Pid::from_raw(child.id() as i32);
        let _ = kill(pid, Signal::SIGTERM);
        let deadline = Instant::now() + grace;
        loop {
            if let Some(status) = child.try_wait().ok().flatten() {
                return status.code().unwrap_or(-1);
            }
            if Instant::now() >= deadline {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        tracing::error!("优雅停止超时（{grace:?}），SIGKILL {}", child.id());
        let _ = kill(pid, Signal::SIGKILL);
        child.wait().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1)
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill();
        child.wait().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::sync::Mutex;

    #[test]
    fn floor_char_boundary_aligns_to_utf8_edges() {
        // "h日本語"：h=1 字节，日/本/語 各 3 字节
        let s = "h日本語";
        assert_eq!(floor_char_boundary(s, 0), 0);
        assert_eq!(floor_char_boundary(s, 1), 1);
        // 落在「日」中间（2、3 字节处）→ 回退到 1
        assert_eq!(floor_char_boundary(s, 2), 1);
        assert_eq!(floor_char_boundary(s, 3), 1);
        assert_eq!(floor_char_boundary(s, 4), 4); // 「本」起点
        assert_eq!(floor_char_boundary(s, s.len()), s.len());
        assert_eq!(floor_char_boundary(s, s.len() + 9), s.len()); // 越界钳到 len
    }

    #[test]
    fn creates_missing_dsh_home() {
        let root = std::env::temp_dir().join(format!("dsh-shell-home-{}", std::process::id()));
        let home = root.join("nested/.dsh");
        let _ = std::fs::remove_dir_all(&root);
        ensure_dsh_home(&home).unwrap();
        assert!(home.is_dir());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn detects_url_from_line() {
        assert_eq!(
            parse_detected_url("DSH web listening on http://127.0.0.1:34567/\n").as_deref(),
            Some("http://127.0.0.1:34567")
        );
    }

    #[test]
    fn detects_none_when_absent() {
        assert_eq!(parse_detected_url("no url here"), None);
        assert_eq!(parse_detected_url("booting...\n"), None);
    }

    #[test]
    fn detects_url_in_utf16le_log() {
        // wsl.exe 重定向日志 = UTF-16LE（NUL 间隔）——read_log_auto 解码后
        // URL 词才是连续的 `http://`；旧实现 read_to_string 读成 `\x00h\x00t...`
        // 导致永远判 Stalled（2026-08-26 实机 bug 回归）。
        let text = "dsh web: http://127.0.0.1:33375\n84) ExperimentalWarning\n";
        let mut bytes = vec![0xFF, 0xFE]; // BOM
        for u in text.encode_utf16() {
            bytes.extend_from_slice(&u.to_le_bytes());
        }
        let decoded = crate::resolve::decode_output_bytes(&bytes);
        assert!(
            parse_detected_url(&decoded)
                .as_deref()
                .is_some_and(|u| u == "http://127.0.0.1:33375"),
            "UTF-16LE 解码后应匹配 URL，得到：{decoded:?}"
        );
        // 无 BOM 的 UTF-16LE（老 wsl.exe 形态）同样命中
        let mut bytes2 = Vec::new();
        for u in text.encode_utf16() {
            bytes2.extend_from_slice(&u.to_le_bytes());
        }
        let decoded2 = crate::resolve::decode_output_bytes(&bytes2);
        assert!(
            parse_detected_url(&decoded2).as_deref() == Some("http://127.0.0.1:33375"),
            "无 BOM UTF-16LE 也应命中，得到：{decoded2:?}"
        );
    }

    #[test]
    fn rejects_node_stack_file_urls() {
        // Node 报错栈里的 file:// 源码路径绝不能被当成访问地址
        let text = "Error: cannot find module\n  at file:///x/dsh/lib/bin.js:1186\n";
        assert_eq!(parse_detected_url(text), None);
        assert_eq!(parse_detected_url("data:text/plain,hi"), None);
        assert_eq!(parse_detected_url("node:internal/modules/run_main"), None);
        // 真正的 http 地址仍正常识别
        assert_eq!(
            parse_detected_url("web: http://127.0.0.1:53599").as_deref(),
            Some("http://127.0.0.1:53599")
        );
    }

    /// 优雅停止路径实测：SIGTERM 后子进程应在 grace 内退出（而非等满超时被强杀）。
    /// dsh 对 SIGTERM 以 exit 0 收尾，sleep 在 macOS 上以信号终止（code=None），
    /// 断言只要求「提前退出」与「已回收」，不抠具体退出码。
    #[cfg(unix)]
    #[test]
    fn stop_sigterm_exits_within_grace() {
        let mut child = Command::new("sleep").arg("30").spawn().unwrap();
        let t0 = Instant::now();
        let _code = stop_dsh(&mut child, Duration::from_secs(3));
        assert!(
            t0.elapsed() < Duration::from_secs(2),
            "SIGTERM 路径应在 grace 内提前退出"
        );
        assert!(child.try_wait().unwrap().is_some(), "子进程应已回收");
    }

    /// 进程存活感知等待：
    /// 1) 日志出 URL → Ready；
    /// 2) 进程先退出（无 URL）→ Exited，且不等到硬上限；
    /// 3) 进程活着但日志停滞 → Stalled。
    #[test]
    fn wait_for_ready_reads_url_when_process_alive() {
        let dir = std::env::temp_dir().join(format!("dsh-shell-wfr-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("inst.log");
        std::fs::write(&path, "booting...\n").unwrap();

        // 用 sleep 进程模拟"活着但还没写完"：日志延迟补上 URL 应判 Ready。
        let child = if cfg!(unix) {
            Command::new("sleep").arg("5").spawn()
        } else {
            // Windows 测试：用真进程保底（cmd /c ping 慢返回），但不强依赖。
            Command::new("cmd.exe")
                .args(["/C", "ping", "-n", "3", "127.0.0.1"])
                .spawn()
        };
        let child = child.expect("spawn 模拟进程");
        let slot: Mutex<Option<DshProcess>> = Mutex::new(Some(DshProcess {
            child,
            log_path: path.clone(),
        }));

        // 延迟追写 URL
        let path2 = path.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(80));
            let mut f = std::fs::OpenOptions::new()
                .append(true)
                .open(&path2)
                .unwrap();
            use std::io::Write;
            writeln!(f, "DSH web listening on http://127.0.0.1:34567/").unwrap();
        });

        match wait_for_ready(
            &path,
            &mut (|| {
                slot.lock()
                    .unwrap()
                    .as_mut()
                    .and_then(|d| d.child.try_wait().ok())
                    .flatten()
                    .map(|s| s.code().unwrap_or(-1))
            }),
            &mut (|| None),
            Duration::from_secs(1),
            Duration::from_secs(5),
        ) {
            ReadyOutcome::Ready(url) => assert_eq!(url, "http://127.0.0.1:34567"),
            other => panic!("应判 Ready，得到 {other:?}"),
        }
        // 收尾：停掉模拟进程
        if let Some(mut d) = slot.lock().unwrap().take() {
            let _ = d.child.kill();
            let _ = d.child.wait();
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn wait_for_ready_returns_exited_immediately() {
        let dir = std::env::temp_dir().join(format!("dsh-shell-wfr4-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("inst.log");
        std::fs::write(&path, "booting...\n").unwrap();

        // 子进程立即退出（true 秒退）→ 应 Exited，不用等硬上限。
        let child = if cfg!(unix) {
            Command::new("true")
        } else {
            Command::new("cmd.exe")
        }
        .spawn()
        .expect("spawn 模拟进程");
        let slot: Mutex<Option<DshProcess>> = Mutex::new(Some(DshProcess {
            child,
            log_path: path.clone(),
        }));
        match wait_for_ready(
            &path,
            &mut (|| {
                slot.lock()
                    .unwrap()
                    .as_mut()
                    .and_then(|d| d.child.try_wait().ok())
                    .flatten()
                    .map(|s| s.code().unwrap_or(-1))
            }),
            &mut (|| None),
            Duration::from_secs(5),
            Duration::from_secs(3),
        ) {
            ReadyOutcome::Exited(_) => {}
            other => panic!("子进程秒退应判 Exited，得到 {other:?}"),
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn wait_for_ready_stalls_when_alive_but_silent() {
        let dir = std::env::temp_dir().join(format!("dsh-shell-wfr5-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("inst.log");
        std::fs::write(&path, "booting...\n").unwrap();

        // 进程活着（sleep 长）但日志不加内容 → 停滞阈值后判 Stalled。
        let child = if cfg!(unix) {
            Command::new("sleep").arg("30").spawn()
        } else {
            Command::new("cmd.exe")
                .args(["/C", "ping", "-n", "30", "127.0.0.1"])
                .spawn()
        };
        let child = child.expect("spawn 模拟进程");
        let slot: Mutex<Option<DshProcess>> = Mutex::new(Some(DshProcess {
            child,
            log_path: path.clone(),
        }));
        match wait_for_ready(
            &path,
            &mut (|| {
                slot.lock()
                    .unwrap()
                    .as_mut()
                    .and_then(|d| d.child.try_wait().ok())
                    .flatten()
                    .map(|s| s.code().unwrap_or(-1))
            }),
            &mut (|| None),
            Duration::from_millis(250),
            Duration::from_secs(5),
        ) {
            ReadyOutcome::Stalled => {}
            other => panic!("存活但停滞应判 Stalled，得到 {other:?}"),
        }
        if let Some(mut d) = slot.lock().unwrap().take() {
            let _ = d.child.kill();
            let _ = d.child.wait();
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    /// marker 路径优先于 log 路径（WSL 缓冲兜底）：log 永远为空模拟 wsl.exe
    /// 转发未 flush，marker 在 80 ms 后返回带 URL 文本。命中即 Ready，不等 stall
    /// / limit。marker 是 `FnMut`，共享 `Arc<Mutex<Option<String>>>`：闭包内
    // 读 clone 出 Option<String>；后台线程延迟写入。
    #[test]
    fn wait_for_ready_marker_takes_priority_over_log() {
        use std::sync::Arc;
        let dir = std::env::temp_dir().join(format!("dsh-shell-wfr-mrk-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("inst.log");
        // log 永远为空（模拟 wsl.exe 转发未 flush）。
        std::fs::write(&path, "").unwrap();

        // 模拟会话进程：sleep 长——永不退出，整个测试期间 alive。
        let child = if cfg!(unix) {
            Command::new("sleep").arg("30").spawn()
        } else {
            Command::new("cmd.exe")
                .args(["/C", "ping", "-n", "30", "127.0.0.1"])
                .spawn()
        };
        let child = child.expect("spawn 模拟进程");
        let slot: Mutex<Option<DshProcess>> = Mutex::new(Some(DshProcess {
            child,
            log_path: path.clone(),
        }));

        // 共享 marker 状态。闭包侧 clone 一份 Option<String>，线程侧延迟写 Some。
        let marker_state: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let writer = marker_state.clone();
        let mut marker = || marker_state.lock().unwrap().clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(80));
            *writer.lock().unwrap() = Some("dsh web: http://127.0.0.1:34777/\n".to_string());
        });

        match wait_for_ready(
            &path,
            &mut (|| {
                slot.lock()
                    .unwrap()
                    .as_mut()
                    .and_then(|d| d.child.try_wait().ok())
                    .flatten()
                    .map(|s| s.code().unwrap_or(-1))
            }),
            &mut marker,
            Duration::from_millis(250), // stall 极短——若 marker 不命中会先 Stalled
            Duration::from_secs(5),     // 总上限
        ) {
            ReadyOutcome::Ready(url) => assert_eq!(url, "http://127.0.0.1:34777"),
            other => panic!("marker 命中应判 Ready，得到 {other:?}"),
        }
        if let Some(mut d) = slot.lock().unwrap().take() {
            let _ = d.child.kill();
            let _ = d.child.wait();
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    /// marker 路径在无 URL 时不误报：marker 持续返回非空文本但不含 URL → 应
    /// 继续轮询 log / 走 stall 兜底，不被 marker 内容里的杂讯误判 Ready。
    #[test]
    fn wait_for_ready_marker_without_url_falls_through_to_log() {
        let dir = std::env::temp_dir().join(format!("dsh-shell-wfr-mrk2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("inst.log");
        std::fs::write(&path, "booting...\n").unwrap();
        let child = if cfg!(unix) {
            Command::new("sleep").arg("30").spawn()
        } else {
            Command::new("cmd.exe")
                .args(["/C", "ping", "-n", "30", "127.0.0.1"])
                .spawn()
        };
        let child = child.expect("spawn 模拟进程");
        let slot: Mutex<Option<DshProcess>> = Mutex::new(Some(DshProcess {
            child,
            log_path: path.clone(),
        }));
        let mut marker = || Some("loading...\nnode init...\n".to_string());

        // log 是空内容（booting 已被 consumed 一次），marker 不含 URL → Stalled
        match wait_for_ready(
            &path,
            &mut (|| {
                slot.lock()
                    .unwrap()
                    .as_mut()
                    .and_then(|d| d.child.try_wait().ok())
                    .flatten()
                    .map(|s| s.code().unwrap_or(-1))
            }),
            &mut marker,
            Duration::from_millis(150),
            Duration::from_secs(3),
        ) {
            ReadyOutcome::Stalled => {}
            other => panic!("marker 无 URL 应走 stall 兜底，得到 {other:?}"),
        }
        if let Some(mut d) = slot.lock().unwrap().take() {
            let _ = d.child.kill();
            let _ = d.child.wait();
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
