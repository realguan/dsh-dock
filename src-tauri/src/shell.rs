//! 运行 dsh 子进程：spawn（`--port 0`）→ 从日志轮询实际 URL → 优雅停止。
//!
//! 逻辑移植自 dsh-launcher 的 `process_guard`（那里已被实测打磨过）：
//!   - `--port 0` 由 OS 分配端口，URL 从 dsh 打在 stdout 的地址行解析；
//!   - 只认 `http://` / `https://` 开头的词，拒绝 `file://`（Node 栈帧）与 `data:`；
//!   - 优雅停止 = SIGTERM → 等待 grace → SIGKILL 兜底（unix；Windows 用 kill）。
//!
//! 与启动器/持久服务的差异：**产品壳与 dsh 严格同生命周期**（壳退 = dsh 停），
//! 不需要独立进程组、也不需要 stdout/stderr 与壳解耦——日志文件每个启动周期
//! 截断重建，URL 探测从文件头开始即可，省掉跨启动偏移逻辑。

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::manifest::ProductManifest;

/// 一个被产品壳托管的 dsh 子进程。
pub struct DshProcess {
    pub child: Child,
    pub log_path: PathBuf,
}

/// 启动 dsh：`<node> <dsh-bin.js> --profile <p> --port 0`，`DSH_HOME` 指向
/// 快照内的虚拟 home。stdout/stderr 进数据目录日志文件（可排查故障）。
pub fn spawn_dsh(
    manifest: &ProductManifest,
    resources_dir: &Path,
    data_dir: &Path,
) -> Result<DshProcess> {
    let node_bin = manifest.snapshot_path(resources_dir, &manifest.snapshot.node_bin);
    let dsh_bin = manifest.snapshot_path(resources_dir, &manifest.snapshot.dsh_bin_js);
    let dsh_home = manifest.snapshot_path(resources_dir, &manifest.snapshot.dsh_home);

    // 快照零部件缺一不可：慢一点把错误讲清楚，别让 node 裸奔报「command not found」。
    if !node_bin.is_file() {
        anyhow::bail!("快照缺少 Node 可执行文件: {}", node_bin.display());
    }
    if !dsh_bin.is_file() {
        anyhow::bail!("快照缺少 dsh 入口: {}", dsh_bin.display());
    }
    if !dsh_home.is_dir() {
        anyhow::bail!("快照缺少虚拟 DSH_HOME 目录: {}", dsh_home.display());
    }

    std::fs::create_dir_all(data_dir).context("创建数据目录")?;
    let log_path = data_dir.join("dsh-shell.log");
    // 每次启动重建：本进程一进程一日志，不跨启动累积（无历史偏移问题）。
    let log = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&log_path)
        .with_context(|| format!("打开日志 {}", log_path.display()))?;

    let mut cmd = Command::new(&node_bin);
    cmd.arg(&dsh_bin)
        .arg("--profile")
        .arg(&manifest.snapshot.profile)
        .arg("--port")
        .arg("0")
        // 桌面壳接管呈现：禁止 dsh 自开系统浏览器（冒烟实测：不加会在
        // 每次启动时弹外部浏览器，把用户从壳里拽出去）。
        .arg("--no-open")
        .env("DSH_HOME", &dsh_home)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone().context("克隆日志句柄")?))
        .stderr(Stdio::from(log));

    let child = cmd
        .spawn()
        .with_context(|| format!("spawn {}", node_bin.display()))?;
    tracing::info!(
        "dsh 已启动：pid={} profile={}",
        child.id(),
        manifest.snapshot.profile
    );
    Ok(DshProcess { child, log_path })
}

/// 从日志文件轮询 dsh 报告的访问地址（最长 `timeout`）。
/// 增量扫描：scanned 只前进，写入中途的截断/非法 UTF-8 下轮再读（追加写）。
pub fn detect_url(log_path: &Path, timeout: Duration) -> Option<String> {
    let deadline = Instant::now() + timeout;
    let mut scanned = 0usize;
    loop {
        if let Ok(text) = std::fs::read_to_string(log_path) {
            let start = text.floor_char_boundary(scanned);
            if scanned < start {
                scanned = start;
            }
            if text.len() > scanned && text.is_char_boundary(scanned) {
                if let Some(url) = parse_detected_url(&text[scanned..]) {
                    return Some(url);
                }
                scanned = text.len();
            }
        }
        if Instant::now() >= deadline {
            return None;
        }
        thread::sleep(Duration::from_millis(50));
    }
}

/// 只接受 `http://` / `https://` 开头的词，去掉尾部 `/`/`,`/`;`。
/// 拒绝 `file://`（Node 加载 bundle 的栈帧）与 `data:` 等，防止把报错路径
/// 当访问地址（移植自启动器 process_guard，含回归测试）。
fn parse_detected_url(text: &str) -> Option<String> {
    text.split_whitespace()
        .find(|w| w.starts_with("http://") || w.starts_with("https://"))
        .map(|url| url.trim_end_matches(['/', ',', ';']).to_string())
}

/// 优雅停止 dsh：unix 发 SIGTERM 等待 grace，超时 SIGKILL；Windows 直接 kill。
/// dsh 对 SIGTERM 以 exit 0 收尾，正常路径秒退。
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

    #[test]
    fn detects_url_from_line() {
        assert_eq!(
            parse_detected_url("DSH web listening on http://127.0.0.1:34567/\n")
                .as_deref(),
            Some("http://127.0.0.1:34567")
        );
    }

    #[test]
    fn detects_none_when_absent() {
        assert_eq!(parse_detected_url("no url here"), None);
        assert_eq!(parse_detected_url("booting...\n"), None);
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

    #[test]
    fn detects_url_from_log_file_without_waiting() {
        let dir = std::env::temp_dir().join(format!("dsh-shell-ulog-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("inst.log");
        std::fs::write(
            &path,
            "booting...\nDSH web listening on http://127.0.0.1:34567/\n",
        )
        .unwrap();
        assert_eq!(
            detect_url(&path, Duration::from_millis(500)).as_deref(),
            Some("http://127.0.0.1:34567")
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn detect_url_waits_for_later_lines() {
        let dir = std::env::temp_dir().join(format!("dsh-shell-ulog2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("inst.log");
        std::fs::write(&path, "booting...\n").unwrap();
        let path2 = path.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(150));
            std::fs::write(&path2, "booting...\nDSH web listening on http://127.0.0.1:34567/\n")
                .unwrap();
        });
        assert_eq!(
            detect_url(&path, Duration::from_millis(2000)).as_deref(),
            Some("http://127.0.0.1:34567")
        );
        std::fs::remove_dir_all(&dir).ok();
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
        assert!(matches!(child.try_wait().unwrap(), Some(_)), "子进程应已回收");
    }
}
