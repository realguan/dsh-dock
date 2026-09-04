//! resolve.rs —— 终端宿主解析链（docs/contract.md「运行时策略」）。
//!
//! 职责：按 manifest 的 resolution 档序（system → bundle → download）解析出
//! 本次启动要用的 `LaunchSpec`（node / dsh 入口 / DSH_HOME / profile）。
//!
//!   - **system**：探测用户官方安装（PATH → realpath → 包树），过三重校验闸
//!     （版本下限 / engines.node / 平台——system 树是就地安装的，平台天然一致）。
//!   - **bundle**：manifest.fallback（内置档兜底副本）。
//!   - **download**：updates 模块在线补齐 Node 与官方 dsh（pnpm 优先、npm 回退）。
//!
//! 借执行器、不借配置（Q2b）：system 命中时 DSH_HOME 指向**用户自身 home**
//! （$DSH_HOME 或 ~/.dsh），boot 的是用户 dsh 世界里的官方/自定义 profile。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::manifest::{FallbackSpec, ProductManifest, TierKind};

/// dsh 执行形态（ADR-0010 引擎档引入）：解析器产出、spawn_dsh / no-open 探测消费。
#[derive(Debug, Clone)]
pub enum DshEntry {
    /// node 前缀执行包入口 lib/bin.js（system / download / 快照档）。
    NodeScript { bin_js: PathBuf },
    /// 引擎 dsh 启动器直接执行（pnpm 全局 shim：Unix shebang 脚本 / Windows
    /// .cmd，child_cmd 吸收差异；node/pnpm 经 PATH 解析）。
    Launcher { bin: PathBuf },
}

/// 解析后的启动规格：一次具体 spawn 的全部决定。
#[derive(Debug, Clone)]
pub struct LaunchSpec {
    pub node_bin: PathBuf,
    pub dsh_entry: DshEntry,
    pub dsh_home: PathBuf,
    pub profile: String,
    pub tier: TierKind,
    /// dsh 是否支持 `--no-open`（system 档按版本探测；旧版不支持时不得传，
    /// 否则 dsh 秒退——rc.5 实测）。False 时 dsh 会自开浏览器（妥协但能启动）。
    pub no_open: bool,
    /// 本次 boot 引导是否补装了 dsh（引擎档首启）：驱动版本状态即时刷新
    ///（否则关于页/菜单停留在安装前的「未检出」）。
    pub first_bootstrap: bool,
}

// ---------- 用户 home ----------

/// GUI 启动时统一取用户 home：Windows 常见的是 USERPROFILE，Unix 使用 HOME。
fn user_home_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
    } else {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
    }
}

/// 终端在 system 档 boot 用户世界：$DSH_HOME 或 ~/.dsh。
pub fn user_dsh_home() -> PathBuf {
    std::env::var_os("DSH_HOME")
        .map(PathBuf::from)
        .or_else(|| user_home_dir().map(|home| home.join(".dsh")))
        .unwrap_or_else(|| PathBuf::from(".dsh"))
}

// ---------- 版本比较（含 rc 语义的排序） ----------

type Seg = (bool, u64, String);

fn version_key(v: &str) -> Vec<Seg> {
    v.split(['.', '-'])
        .map(|s| match s.parse::<u64>() {
            Ok(n) => (true, n, String::new()),
            Err(_) => (false, 0, s.to_string()),
        })
        .collect()
}

/// 升序比较（0.1.0-rc.6 < 0.1.0-rc.7 < 0.1.0）。
pub fn compare_versions_asc(a: &str, b: &str) -> std::cmp::Ordering {
    let (ka, kb) = (version_key(a), version_key(b));
    for (x, y) in ka.iter().zip(kb.iter()) {
        let ord = match (x.0, y.0) {
            (true, true) => x.1.cmp(&y.1),
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            (false, false) => x.2.cmp(&y.2),
        };
        if ord != std::cmp::Ordering::Equal {
            return ord;
        }
    }
    if ka.len() != kb.len() {
        let long_seg = if ka.len() > kb.len() {
            &ka[kb.len()]
        } else {
            &kb[ka.len()]
        };
        // 多出来的段是纯文本（rc/beta 等预发布标记）→ 长列表是预发布，更小
        if !long_seg.0 {
            // 长列表 = 带预发布标记的版本 → 它更小
            return if ka.len() > kb.len() {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        return ka.len().cmp(&kb.len());
    }
    std::cmp::Ordering::Equal
}

// ---------- 环境感知（GUI 启动的 PATH 是系统最小集，必须先补全） ----------

/// 解码 wsl.exe / 子进程的原始输出字节：探测 UTF-16LE（BOM 或 NUL 间隔）则按
/// UTF-16LE 解码，否则按 UTF-8 lossy 解码。跨平台纯函数（任何平台可测）。
/// wsl.exe 重定向输出非 UTF-8（老版本/非 tty 为 UTF-16LE）——executor 的
/// run_wsl_capture 与 shell 的日志轮询共用（2026-08-26 实机 bug：UTF-16LE
/// 日志里 URL 是 `\x00h\x00t\x00t\x00p\x00` 间隔，`starts_with("http://")`
/// 永远失败 → 判「等待超时」）。
pub fn decode_output_bytes(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.contains(&0) {
        if let Some(s) = decode_utf16le(bytes) {
            return s;
        }
    }
    String::from_utf8_lossy(bytes).to_string()
}

/// 按 UTF-16LE（含小端 BOM 或不含）解码字节。纯函数，全平台可测。
pub fn decode_utf16le(bytes: &[u8]) -> Option<String> {
    let mut bytes = bytes;
    if bytes.starts_with(&[0xFF, 0xFE]) {
        bytes = &bytes[2..];
    }
    let units: Vec<u16> = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    let mut s = String::from_utf16(&units).ok()?;
    while s.ends_with('\u{0}') {
        s.pop();
    }
    Some(s)
}

/// 读取可能为 UTF-16LE（wsl.exe 重定向）的日志文件为 UTF-8 文本。
/// `read_to_string` 严格 UTF-8 会把 UTF-16LE 日志读成含 NUL 的"假有效"
/// 文本（NUL 是合法 UTF-8 字节），导致 URL 匹配失败——统一先读原始字节
/// 再自动解码。读不到（文件不存在/无权限）→ 空串。
pub fn read_log_auto(path: &std::path::Path) -> String {
    match std::fs::read(path) {
        Ok(bytes) => decode_output_bytes(&bytes),
        Err(_) => String::new(),
    }
}

/// 带超时执行并取原始 stdout 字节（login shell 拉 PATH / executor 的 wsl.exe
/// 捕获共用——统一走 `quiet_cmd` 纪律与超时上限；返回原始字节以便按需解码，
/// 如 wsl.exe 可能输出 UTF-16LE 而非 UTF-8）。
// WSL 执行器（Windows-only）消费；非 Windows 构建下合法 dead（lib 级告警抑制）。
#[cfg_attr(not(windows), allow(dead_code))]
pub fn run_with_timeout_raw(cmd: &mut Command, timeout: std::time::Duration) -> Option<Vec<u8>> {
    crate::quiet_cmd(cmd);
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().ok().flatten() {
            if !status.success() {
                return None;
            }
            use std::io::Read;
            let mut out = Vec::new();
            child.stdout.take()?.read_to_end(&mut out).ok()?;
            return if out.is_empty() { None } else { Some(out) };
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

fn path_separator() -> char {
    if cfg!(windows) {
        ';'
    } else {
        ':'
    }
}

/// 合并多路 PATH 源（用户环境优先 → 固定目录 → 当前 PATH，去重保序）。
pub fn merge_paths(sources: &[String]) -> String {
    merge_paths_with_separator(sources, path_separator())
}

/// 可指定分隔符的 PATH 合并实现，供 Windows 路径回归测试复用。
fn merge_paths_with_separator(sources: &[String], separator: char) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut dirs: Vec<String> = Vec::new();
    for source in sources {
        for d in source.split(separator) {
            if d.is_empty() || !seen.insert(d.to_string()) {
                continue;
            }
            dirs.push(d.to_string());
        }
    }
    dirs.join(&separator.to_string())
}

/// 壳进程可见 PATH（原样）。探测层退役（ADR-0010）后壳不再补全用户环境：
/// 引擎工具走 engines/bin 前置（dsh_child_path），系统实用程序（tar 等）
/// 在 GUI 最小 PATH 内恒可达。
pub fn effective_path() -> String {
    static CACHE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    CACHE
        .get_or_init(|| std::env::var("PATH").unwrap_or_default())
        .clone()
}

/// 把选中的 Node 可执行目录放到 PATH 首位。
///
/// download 档的 Node 存在应用数据目录，不会出现在用户登录 shell 的 PATH；
/// dsh 启动后若再拉起 helper/子命令，仍需要通过 PATH 找到同一份 Node。
pub fn path_with_bin(bin: &Path, path_env: &str) -> String {
    let Some(parent) = bin.parent() else {
        return path_env.to_string();
    };
    let separator = path_separator();
    if path_env.is_empty() {
        parent.display().to_string()
    } else {
        format!("{}{}{}", parent.display(), separator, path_env)
    }
}

// ---------- 引擎目录（ADR-0010 P2，2026-09-03） ----------

/// 引擎目录：壳自管工具（pnpm 等）的私有落点，在数据目录下，不动用户世界。
/// 单目录布局实测见 docs/spikes/0003 §2.3（PNPM_HOME 兼作 runtime 项目为 P3 形态）。
pub fn engines_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("engines")
}

/// dsh 子进程环境自构（ADR-0010）：引擎 bin（捆绑 pnpm / node shim / dsh
/// 启动器恒可达）→ 选定 node 所在目录 → 用户 PATH。spawn_dsh、插件/创建
/// 工具链、no-open 探测必须同源本函数。
pub fn dsh_child_path(node_bin: &Path, data_dir: &Path) -> String {
    merge_paths(&[
        crate::engines::engine_bin_dir(data_dir)
            .display()
            .to_string(),
        path_with_bin(node_bin, &effective_path()),
    ])
}

/// 按档序解析本次启动的 LaunchSpec（规范化后仅 Engine / Bundle 两档）。
/// 引擎档引导的网络进度经 `progress` 上抛。
pub fn resolve_launch(
    manifest: &ProductManifest,
    resources_dir: &Path,
    path_env: &str,
    data_dir: &Path,
    progress: crate::updates::DownloadProgress,
) -> Result<LaunchSpec> {
    let spec = &manifest.terminal.resolution.dsh;

    let [tier] = spec.tiers.as_slice() else {
        anyhow::bail!("resolution 档序为空，无法解析宿主");
    };
    match tier {
        TierKind::Engine => {
            // 引擎档（ADR-0010，v3 缺省）：壳引擎三件幂等引导——pnpm 随壳
            // 重铺、node=node-map（缺失必补/不符切换/离线降级）、dsh=最新
            // 稳定版（缺失才装，dist-tags 惰性解析保离线零网络）。
            let outcome = crate::updates::ensure_engine_bootstrapped(
                data_dir,
                resources_dir,
                path_env,
                progress,
            )
            .map_err(|e| anyhow::anyhow!("引擎引导失败：{e}"))?;
            tracing::info!(
                "引擎引导完成：pnpm={:?} node={:?} dsh={:?}（node 切换={}，dsh 补装={}）",
                outcome.status.pnpm,
                outcome.status.node,
                outcome.status.dsh,
                outcome.node_switched,
                outcome.dsh_installed,
            );
            engine_launch_spec(
                data_dir,
                outcome.status.dsh.as_deref(),
                outcome.dsh_installed,
                manifest.terminal.default_profile.clone(),
            )
        }
        TierKind::Bundle => {
            let fb = manifest.fallback.clone().ok_or_else(|| {
                anyhow::anyhow!("契约声明 bundle 档但缺少 fallback（自洽性校验应拦截，此属异常）")
            })?;
            // 快照 home 内是装配时固化的 profile：boot 它而不是 default_profile。
            let mut spec = launch_from_fallback(&fb, resources_dir, fb.profile.clone());
            spec.no_open = true;
            // 快照 home 在 bundle 内只读：首启同步到
            // 可写数据目录，dsh 的会话/设置才落得下；仅覆盖不删除，运行数据保留。
            match sync_fallback_home(&spec.dsh_home, data_dir) {
                Ok(home) => spec.dsh_home = home,
                Err(e) => {
                    anyhow::bail!("同步兜底 home 到数据目录失败：{e}");
                }
            }
            Ok(spec)
        }
    }
}

/// 把兜底 home 同步到可写数据目录（`<data_dir>/runtimes/fallback-home`）。
/// 语义：覆盖同路径文件、**不删除**目标多余内容（dsh 写过的会话/设置保留）；
/// 每次启动重同步（home 只有装配的 profile/settings，体积小）。
pub fn sync_fallback_home(src: &Path, data_dir: &Path) -> Result<PathBuf> {
    let dest = data_dir.join("runtimes").join("fallback-home");
    fs::create_dir_all(&dest).with_context(|| format!("创建数据目录 {}", dest.display()))?;
    copy_tree(src, &dest)?;
    Ok(dest)
}

/// 递归复制：覆盖文件、保留目标多余项。
fn copy_tree(src: &Path, dst: &Path) -> Result<()> {
    for entry in fs::read_dir(src).with_context(|| format!("读取 {}", src.display()))? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            fs::create_dir_all(&to).with_context(|| format!("创建 {}", to.display()))?;
            copy_tree(&from, &to)?;
        } else {
            fs::copy(&from, &to)
                .with_context(|| format!("复制 {} → {}", from.display(), to.display()))?;
        }
    }
    Ok(())
}

/// 引擎档 LaunchSpec 构造（引导完成后）：引擎目录定位 node + dsh 启动器 +
/// no-open 探测。独立成函数供离线单测（引导本身的网络动作归
/// ensure_engine_bootstrapped，见其测试）。
fn engine_launch_spec(
    data_dir: &Path,
    dsh_version: Option<&str>,
    first_bootstrap: bool,
    default_profile: String,
) -> Result<LaunchSpec> {
    let node_bin = crate::engines::engine_node_bin(data_dir).ok_or_else(|| {
        anyhow::anyhow!(
            "引擎引导完成但未定位到 node（引擎目录异常）——删除 engines 目录后重启应用可重建"
        )
    })?;
    let launcher = crate::engines::engine_dsh_bin(data_dir).ok_or_else(|| {
        anyhow::anyhow!(
            "引擎引导完成但未定位到 dsh 启动器（引擎目录异常）——删除 engines 目录后重启应用可重建"
        )
    })?;
    let no_open = engine_no_open_supported(dsh_version.unwrap_or(""), &launcher, data_dir);
    Ok(LaunchSpec {
        node_bin,
        dsh_entry: DshEntry::Launcher { bin: launcher },
        dsh_home: user_dsh_home(),
        profile: default_profile,
        tier: TierKind::Engine,
        no_open,
        first_bootstrap,
    })
}

/// bundle 档：fallback 三件套（相对 resources 根）。
fn launch_from_fallback(fb: &FallbackSpec, resources_dir: &Path, profile: String) -> LaunchSpec {
    LaunchSpec {
        node_bin: fb.resolve_path(resources_dir, &fb.node_bin),
        dsh_entry: DshEntry::NodeScript {
            bin_js: fb.resolve_path(resources_dir, &fb.dsh_bin_js),
        },
        dsh_home: fb.resolve_path(resources_dir, &fb.dsh_home),
        profile,
        tier: TierKind::Bundle,
        no_open: true,
        first_bootstrap: false,
    }
}

// ---------- --no-open 探测缓存（2026-09-01，AGENTS §6 例外册登记） ----------
//
// 语义：**可丢失、可重建的运行时缓存**——dsh 版本 → 该版本是否支持
// `--no-open`。支持性是版本特性（旧版 dsh 收到未知参数会秒退），版本不变
// 结果稳定；损坏 / 缺失 / 解析失败一律回退探测，绝不阻断启动。

fn probe_cache_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("probe-cache.json")
}

/// 读探测缓存：缺失 / 损坏 / 非对象 → 空表（回退探测路径）。
fn load_probe_cache(data_dir: &Path) -> std::collections::HashMap<String, bool> {
    std::fs::read_to_string(probe_cache_path(data_dir))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// 原子写探测缓存（tmp + rename，复用 settings 的模式）；失败静默——
/// 缓存语义即可丢失，失败只是下次启动重新探测，不阻断 boot。
fn save_probe_cache(
    data_dir: &Path,
    cache: &std::collections::HashMap<String, bool>,
) -> Result<()> {
    std::fs::create_dir_all(data_dir)?;
    let path = probe_cache_path(data_dir);
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(cache)?;
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

/// 引擎档 no-open 支持性：dsh 启动器直接探测（node 经 PATH 解析），
/// 版本 = 引导终态版本（缓存键与 system 档同机制）。
fn engine_no_open_supported(version: &str, launcher: &Path, data_dir: &Path) -> bool {
    let key = launcher.to_path_buf();
    no_open_supported(
        version,
        (key.clone(), key),
        || probe_no_open_launcher(launcher, PROBE_NO_OPEN_TIMEOUT),
        data_dir,
    )
}

/// no-open 支持性判定（system / 引擎档共用）：磁盘缓存按版本（版本特性，
/// 结果稳定）→ 进程内缓存按探测键（同一次运行多次解析）→ 实测兜底并回写。
fn no_open_supported(
    version: &str,
    probe_key: (PathBuf, PathBuf),
    probe: impl FnOnce() -> bool,
    data_dir: &Path,
) -> bool {
    let cache = load_probe_cache(data_dir);
    if let Some(&v) = cache.get(version) {
        return v;
    }
    // 进程内缓存兜底（同一次运行多次解析命中）
    static CACHE: std::sync::OnceLock<std::sync::Mutex<Vec<(PathBuf, PathBuf, bool)>>> =
        std::sync::OnceLock::new();
    let cache2 = CACHE.get_or_init(|| std::sync::Mutex::new(Vec::new()));
    let in_proc = {
        let guard = cache2.lock().unwrap();
        guard
            .iter()
            .find(|(n, b, _)| n == &probe_key.0 && b == &probe_key.1)
            .map(|(_, _, v)| *v)
    };
    if let Some(v) = in_proc {
        return v;
    }
    let v = probe();
    cache2.lock().unwrap().push((probe_key.0, probe_key.1, v));
    let mut updated = cache;
    updated.insert(version.to_string(), v);
    let _ = save_probe_cache(data_dir, &updated);
    v
}

/// 探测总超时：宽松于本机实测 8~11s（Windows 冷加载更慢），远小于
/// BOOT_TIMEOUT 90s；超时按「不支持 --no-open」处理（宁可不传），
/// 探测进程卡死不再永久阻塞 boot。
const PROBE_NO_OPEN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

/// 引擎档探测：dsh 启动器直接执行（shebang 脚本 / .cmd shim 经 child_cmd
/// 分发，node 经 PATH 解析），参数与 system 档探测同款。
fn probe_no_open_launcher(launcher: &Path, timeout: std::time::Duration) -> bool {
    let mut cmd = crate::child_cmd(launcher);
    cmd.args(["--profile", "web", "--help"]);
    probe_no_open_cmd(cmd, timeout)
}

/// no-open 探测公共体：构造好的命令 + 总超时；输出含 `--no-open` 判支持。
fn probe_no_open_cmd(mut cmd: std::process::Command, timeout: std::time::Duration) -> bool {
    use std::io::{BufRead, BufReader};
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => return false,
    };
    let deadline = std::time::Instant::now() + timeout;
    // stdout / stderr 各有独立线程读到共享 buffer：主循环只轮询 buffer，
    // 避免 `read_line` 在子进程无输出时无限阻塞绕过 deadline（超时兜底
    // 必须能真正打断）。rc 版本可能把 usage 打到 stderr，两路都查。
    let shared: std::sync::Arc<std::sync::Mutex<String>> =
        std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let mut readers = Vec::new();
    let streams: [Option<Box<dyn std::io::Read + Send>>; 2] = [
        child
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
        child
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
    ];
    for stream in streams {
        let Some(stream) = stream else {
            continue;
        };
        let shared2 = shared.clone();
        readers.push(std::thread::spawn(move || {
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let mut g = shared2.lock().unwrap();
                        g.push_str(&line);
                    }
                }
            }
        }));
    }
    loop {
        if std::time::Instant::now() >= deadline {
            break;
        }
        if shared.lock().unwrap().contains("--no-open") {
            let _ = child.kill();
            let _ = child.wait();
            return true;
        }
        if child.try_wait().ok().flatten().is_some() {
            // 子进程已自然退出：等待 reader 线程把管道残留数据完全读尽（防并发早退漏读）
            for r in readers {
                let _ = r.join();
            }
            let hit = shared.lock().unwrap().contains("--no-open");
            return hit;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    // 超时分支（子进程卡死 hang 住）：强杀后立即返回 false，绝不死等 reader
    let _ = child.kill();
    let _ = child.wait();
    false
}

/// defaultProfile 消费决策（4.3④ 接线，2026-08-28；纯函数）。
/// 存储的默认 profile 直接作为本次启动 profile（并跳过选择器）当且仅当：
/// 它在 webUi 候选内。壳的启动链路以「日志解析 URL → WebView 导航」为前提，
/// 非 webUi profile（如 headless）boot 后无 URL 可导航——自动启动必然超时，
/// 故不在候选的存储值（headless / 自定义无 webUi / 已被手工删除）一律回退
/// 常规流程（多 webUi 仍出选择器），不猜用户意图。未存储 = None 不消费。
pub fn consume_default_profile(
    stored: Option<&str>,
    webui_candidates: &[String],
) -> Option<String> {
    stored
        .filter(|n| webui_candidates.iter().any(|c| c == n))
        .map(String::from)
}

/// 用户 home 下「webUi=true」的 profile 列表：scan profiles/*/package.json 的
/// dsh.profile.bundles 是否含 `@deepseek-ai/dsh-web-app`；官方 web 恒为首选。
/// （F-b：boot 选择器数据源；v1 默认 profile 仍是 manifest.default_profile。）
pub fn list_web_ui_profiles(home: &Path) -> Vec<String> {
    let mut out = vec!["web".to_string()];
    let Ok(entries) = fs::read_dir(home.join("profiles")) else {
        return out;
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let Some(name) = dir.file_name().map(|s| s.to_string_lossy().into_owned()) else {
            continue;
        };
        if name == "web" {
            continue;
        }
        let manifest = dir.join("package.json");
        let Ok(text) = fs::read_to_string(&manifest) else {
            continue;
        };
        let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let has_web = pkg
            .get("dsh")
            .and_then(|d| d.get("profile"))
            .and_then(|p| p.get("bundles"))
            .and_then(|b| b.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str())
                    .any(|s| s == "@deepseek-ai/dsh-web-app")
            })
            .unwrap_or(false);
        if has_web {
            out.push(name);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn tmp() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "dsh-shell-res-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn version_ordering_handles_rc() {
        use std::cmp::Ordering;
        assert_eq!(
            compare_versions_asc("0.1.0-rc.9", "0.1.0-rc.6"),
            Ordering::Greater
        );
        assert_eq!(
            compare_versions_asc("0.1.0-rc.5", "0.1.0-rc.6"),
            Ordering::Less
        );
        assert_eq!(
            compare_versions_asc("0.1.0", "0.1.0-rc.9"),
            Ordering::Greater
        );
        assert_eq!(
            compare_versions_asc("0.1.0-rc.6", "0.1.0-rc.6"),
            Ordering::Equal
        );
    }

    #[test]
    fn merge_paths_deduplicates_keeping_first() {
        let separator = path_separator();
        let sep = separator.to_string();
        let s = merge_paths_with_separator(
            &[
                format!("/usr/local/bin{sep}/usr/bin"),
                format!("/usr/bin{sep}/bin"),
                format!("/opt/homebrew/bin{sep}/usr/local/bin"),
            ],
            separator,
        );
        assert_eq!(
            s,
            format!("/usr/local/bin{sep}/usr/bin{sep}/bin{sep}/opt/homebrew/bin")
        );
    }

    #[test]
    fn windows_path_separator_preserves_drive_letters() {
        let s = merge_paths_with_separator(
            &[
                "C:/Users/demo/AppData/Roaming/npm;C:/Windows/System32".to_string(),
                "C:/Windows/System32;C:/Program Files/nodejs".to_string(),
            ],
            ';',
        );
        assert_eq!(
            s,
            "C:/Users/demo/AppData/Roaming/npm;C:/Windows/System32;C:/Program Files/nodejs"
        );
    }

    #[test]
    fn selected_node_is_prepended_to_runtime_path() {
        let node = Path::new("/private/dsh-dock/node/bin/node");
        let path = path_with_bin(node, "/usr/local/bin:/usr/bin");
        let separator = if cfg!(windows) { ';' } else { ':' };
        assert_eq!(
            path.split(separator).next(),
            Some("/private/dsh-dock/node/bin")
        );
    }

    #[test]
    fn dsh_child_path_orders_engines_node_then_user() {
        // 引擎 bin（pnpm 恒可达）→ node bin → 用户 PATH 的次序是
        // spawnSync("pnpm") 与 helper 执行器两边的硬前提（ADR-0010 P2）。
        let root = tmp();
        let node = Path::new("/opt/node/bin/node");
        let merged = dsh_child_path(node, &root);
        let parts: Vec<&str> = merged.split(path_separator()).collect();
        let engines = crate::engines::engine_bin_dir(&root).display().to_string();
        let engine_pos = parts.iter().position(|p| *p == engines).unwrap();
        let node_pos = parts.iter().position(|p| *p == "/opt/node/bin").unwrap();
        assert!(engine_pos < node_pos, "引擎 bin 先于 node bin：{merged}");
        assert!(node_pos < parts.len() - 1, "用户 PATH 在最后：{merged}");
    }

    #[cfg(unix)]
    #[test]
    fn engine_launch_spec_builds_launcher_entry_and_probes_no_open() {
        // 引擎档 LaunchSpec 构造（离线）：启动器形态 + no-open 实测（假 dsh
        // 的 --help 输出含 --no-open）。引导网络动作不在此函数（见 engines.rs）。
        use std::os::unix::fs::PermissionsExt;
        let root = tmp();
        let data_dir = root.join("data");
        let bin = data_dir.join("engines/bin");
        std::fs::create_dir_all(&bin).unwrap();
        let make = |name: &str, body: &str| {
            let p = bin.join(name);
            std::fs::write(&p, body).unwrap();
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        };
        make("node", "#!/bin/sh\necho v24.18.0\n");
        make(
            "dsh",
            "#!/bin/sh\necho \"  --no-open             do not open browser\"\n",
        );

        let spec = engine_launch_spec(&data_dir, Some("0.9.0"), false, "web".to_string()).unwrap();
        assert_eq!(spec.tier, TierKind::Engine);
        assert_eq!(spec.node_bin.parent(), Some(bin.as_path()));
        assert!(spec.no_open, "启动器 --help 含 --no-open 应判支持");
        match &spec.dsh_entry {
            DshEntry::Launcher { bin: launcher } => {
                assert_eq!(launcher.parent(), Some(bin.as_path()))
            }
            DshEntry::NodeScript { .. } => panic!("引擎档应为启动器执行形态"),
        }
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn consume_default_profile_only_for_webui_candidates() {
        let cands = vec!["web".to_string(), "custom-a".to_string()];
        // 候选内：直接消费（web 与自定义 webUi 均可）
        assert_eq!(
            consume_default_profile(Some("custom-a"), &cands).as_deref(),
            Some("custom-a")
        );
        assert_eq!(
            consume_default_profile(Some("web"), &cands).as_deref(),
            Some("web")
        );
        // 非 webUi（headless）/已删除名：不消费（自动启动无 URL 可导航或必然失败）
        assert_eq!(consume_default_profile(Some("headless"), &cands), None);
        assert_eq!(consume_default_profile(Some("ghost"), &cands), None);
        // 未设置：不消费
        assert_eq!(consume_default_profile(None, &cands), None);
    }

    #[test]
    fn web_ui_profiles_scan() {
        let home = tmp();
        std::fs::create_dir_all(home.join("profiles/custom-a")).unwrap();
        std::fs::write(
            home.join("profiles/custom-a/package.json"),
            r#"{"dsh": {"profile": {"bundles": ["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-web-app", "x"]}}}"#,
        )
        .unwrap();
        std::fs::create_dir_all(home.join("profiles/custom-b")).unwrap();
        std::fs::write(
            home.join("profiles/custom-b/package.json"),
            r#"{"dsh": {"profile": {"bundles": ["@deepseek-ai/dsh-base"]}}}"#,
        )
        .unwrap();
        let list = list_web_ui_profiles(&home);
        // web 恒在首，custom-a 含 web-app，custom-b 不含
        assert_eq!(list, vec!["web".to_string(), "custom-a".to_string()]);
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn resolve_bundle_tier_from_fallback() {
        let root = tmp();
        let res = root.join("resources");
        std::fs::create_dir_all(&res).unwrap();
        let json = r#"{
          "format": 2,
          "productName": "T",
          "terminal": {
            "resolution": {
              "dsh": { "tiers": ["bundle"] }
            }
          },
          "fallback": {
            "nodeBin": "dsh-snapshot/node/bin/dsh-node",
            "dshBinJs": "dsh-snapshot/dsh/@deepseek-ai/dsh/lib/bin.js",
            "dshHome": "dsh-snapshot/home",
            "profile": "desktop-demo"
          }
        }"#;
        // fallback home 必须真实存在（sync 语义）
        std::fs::create_dir_all(res.join("dsh-snapshot/home/profiles/desktop-demo")).unwrap();
        std::fs::write(res.join("dsh-snapshot/home/settings.yaml"), "k: v\n").unwrap();
        // tiers 是规范化产物（skip 反序列化）→ 走 load 迁移：v2+fallback → [Bundle]
        let manifest_path = root.join("product.manifest.json");
        std::fs::write(&manifest_path, json).unwrap();
        let m = ProductManifest::load(&manifest_path).unwrap();
        let spec = resolve_launch(&m, &res, "", &root, &mut |_, _| {}).unwrap();
        assert_eq!(spec.tier, TierKind::Bundle);
        assert_eq!(spec.profile, "desktop-demo");
        assert!(spec.node_bin.ends_with("dsh-snapshot/node/bin/dsh-node"));
        // bundle 档 home 已被同步到数据目录（可写）
        assert!(spec
            .dsh_home
            .starts_with(root.join("runtimes/fallback-home")));
        assert!(spec.dsh_home.join("settings.yaml").is_file());
    }

    #[test]
    fn sync_fallback_home_overwrites_and_keeps_extras() {
        let root = tmp();
        let src = root.join("src");
        std::fs::create_dir_all(src.join("profiles/a")).unwrap();
        std::fs::write(src.join("settings.yaml"), "v1\n").unwrap();
        std::fs::write(src.join("profiles/a/package.json"), "{}").unwrap();
        let dest = sync_fallback_home(&src, &root).unwrap();
        assert!(dest.join("settings.yaml").is_file());
        // 运行数据：目标多出的文件不被删
        std::fs::create_dir_all(dest.join("sessions")).unwrap();
        std::fs::write(dest.join("sessions/run.log"), "run\n").unwrap();
        // 源变更 → 再同步覆盖，多余保留
        std::fs::write(src.join("settings.yaml"), "v2\n").unwrap();
        sync_fallback_home(&src, &root).unwrap();
        assert_eq!(
            std::fs::read_to_string(dest.join("settings.yaml")).unwrap(),
            "v2\n"
        );
        assert!(dest.join("sessions/run.log").is_file());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn resolve_empty_tiers_fails_with_message() {
        let json = r#"{"format": 2, "productName": "T",
          "terminal": {"resolution": {"dsh": {"tiers": []}}}}"#;
        let m: ProductManifest = serde_json::from_str(json).unwrap();
        let err = resolve_launch(
            &m,
            Path::new("/res"),
            "",
            Path::new("/tmp/none"),
            &mut |_, _| {},
        )
        .unwrap_err();
        assert!(err.to_string().contains("档序为空"));
    }

    // ---------- probe_no_open 探测（2026-09-01：流式早退 + 超时兜底） ----------

    /// 造一个「假 node 执行器」：把 dsh 参数拼进一个可配置 shell 脚本。
    /// `script` 内可用 `$1`（dsh bin 路径）。unix 下chmod +x。
    #[cfg(unix)]
    fn fake_node_executor(dir: &Path, script: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let bin = dir.join("fake-node");
        std::fs::write(&bin, script).unwrap();
        std::fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
        bin
    }

    /// 测试用 dsh 假体路径（不存在也合法：脚本不真的读它）。
    #[cfg(unix)]
    fn fake_dsh(dir: &Path) -> PathBuf {
        dir.join("fake-dsh.js")
    }

    /// 命中 --no-open 立即早退：dsh 假体先打印 usage（含 --no-open）再 sleep 30。
    /// 旧实现（output 等自然退出）要 30s+；流式读 stdout 应 <2s 返回 true。
    #[cfg(unix)]
    #[test]
    fn probe_no_open_returns_early_on_hit() {
        let dir = tmp();
        let node = fake_node_executor(
            &dir,
            r#"#!/bin/sh
echo "Usage: dsh --profile web [options]"
echo "  --no-open    do not open the Web UI"
sleep 30
"#,
        );
        let t0 = std::time::Instant::now();
        let mut cmd = std::process::Command::new(&node);
        cmd.arg(fake_dsh(&dir));
        let hit = probe_no_open_cmd(cmd, std::time::Duration::from_secs(60));
        assert!(hit, "usage 含 --no-open 应判 true");
        assert!(
            t0.elapsed() < std::time::Duration::from_secs(10),
            "命中后应 kill 早退（并行负载容差），实测 {:?}",
            t0.elapsed()
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 无 --no-open 的 usage → false（宁可不传；旧防护语义回归）。
    #[cfg(unix)]
    #[test]
    fn probe_no_open_missing_flag_is_false() {
        let dir = tmp();
        let node = fake_node_executor(
            &dir,
            r#"#!/bin/sh
echo "Usage: dsh --profile web [options]"
echo "  --port <port>  listen port"
"#,
        );
        let mut cmd = std::process::Command::new(&node);
        cmd.arg(fake_dsh(&dir));
        assert!(!probe_no_open_cmd(cmd, std::time::Duration::from_secs(10)));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 卡死（无输出不退出）→ 超时兜底 false，不无限阻塞（按 PROBE 语义兜底）。
    #[cfg(unix)]
    #[test]
    fn probe_no_open_times_out_when_hung() {
        let dir = tmp();
        let node = fake_node_executor(&dir, "#!/bin/sh\nsleep 60\n");
        let t0 = std::time::Instant::now();
        let mut cmd = std::process::Command::new(&node);
        cmd.arg(fake_dsh(&dir));
        let hit = probe_no_open_cmd(cmd, std::time::Duration::from_millis(300));
        assert!(!hit, "卡死进程超时应判 false");
        assert!(
            t0.elapsed() < std::time::Duration::from_secs(3),
            "超时应及时返回"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// stderr 上打印 usage 的版本也能命中（rc 常见形态）。
    #[cfg(unix)]
    #[test]
    fn probe_no_open_finds_flag_on_stderr() {
        let dir = tmp();
        let node = fake_node_executor(
            &dir,
            r#"#!/bin/sh
echo "Usage: dsh ... [options]" >&2
echo "  --no-open    do not open" >&2
sleep 30
"#,
        );
        let t0 = std::time::Instant::now();
        let mut cmd = std::process::Command::new(&node);
        cmd.arg(fake_dsh(&dir));
        let hit = probe_no_open_cmd(cmd, std::time::Duration::from_secs(60));
        assert!(hit, "stderr 上的 --no-open 同样应命中");
        assert!(t0.elapsed() < std::time::Duration::from_secs(5));
        std::fs::remove_dir_all(&dir).ok();
    }

    // ---------- 探测缓存（2026-09-01：跨启动持久化） ----------

    #[test]
    fn probe_cache_roundtrips() {
        let dir = tmp();
        let mut cache = std::collections::HashMap::new();
        cache.insert("0.1.1-rc.2".to_string(), true);
        save_probe_cache(&dir, &cache).unwrap();
        let loaded = load_probe_cache(&dir);
        assert_eq!(loaded.get("0.1.1-rc.2"), Some(&true));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn probe_cache_corrupted_falls_back_empty() {
        let dir = tmp();
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(probe_cache_path(&dir), "{ not json").unwrap();
        let loaded = load_probe_cache(&dir);
        assert!(loaded.is_empty(), "损坏缓存应回退空表，不 panic");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 缓存判定泛化后回归：命中零探测、miss 探测写回、版本变化失效
    ///（no_open_supported 纯缓存逻辑 + 注入探测闭包，零 spawn 可测）。
    #[test]
    fn no_open_supported_cache_hit_miss_and_version_invalidation() {
        let dir = tmp();
        // miss → 探测（写标记文件证明闭包执行）→ 写回
        let marker = dir.join("probed");
        let probe = || {
            std::fs::write(&marker, b"1").unwrap();
            true
        };
        let v1 = no_open_supported(
            "0.1.0",
            (PathBuf::from("/p"), PathBuf::from("/p")),
            probe,
            &dir,
        );
        assert!(v1 && marker.is_file(), "miss 应实测并写回");
        // 命中 → 零探测（标记文件删掉后不再重建）
        std::fs::remove_file(&marker).unwrap();
        let v2 = no_open_supported(
            "0.1.0",
            (PathBuf::from("/q"), PathBuf::from("/q")),
            probe,
            &dir,
        );
        assert!(v2 && !marker.exists(), "命中应零探测");
        // 版本变化 → 重新探测
        let v3 = no_open_supported(
            "0.2.0",
            (PathBuf::from("/r"), PathBuf::from("/r")),
            || false,
            &dir,
        );
        assert!(!v3, "新版本重新实测");
        let loaded = load_probe_cache(&dir);
        assert_eq!(loaded.get("0.1.0"), Some(&true));
        assert_eq!(loaded.get("0.2.0"), Some(&false));
        std::fs::remove_dir_all(&dir).ok();
    }
}
