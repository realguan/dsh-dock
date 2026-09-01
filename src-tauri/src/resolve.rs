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

use crate::manifest::{FallbackSpec, ProductManifest, TierKind, TierSpec};

/// 解析后的启动规格：一次具体 spawn 的全部决定。
#[derive(Debug, Clone)]
pub struct LaunchSpec {
    pub node_bin: PathBuf,
    pub dsh_bin_js: PathBuf,
    pub dsh_home: PathBuf,
    pub profile: String,
    pub tier: TierKind,
    /// dsh 是否支持 `--no-open`（system 档按版本探测；旧版不支持时不得传，
    /// 否则 dsh 秒退——rc.5 实测）。False 时 dsh 会自开浏览器（妥协但能启动）。
    pub no_open: bool,
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

pub fn version_at_least(current: &str, min: &str) -> bool {
    compare_versions_asc(current, min) != std::cmp::Ordering::Less
}

/// 从版本字符串取主版本号（"v24.18.0"/"24.18.0" → 24）。
fn major_of(v: &str) -> Option<u64> {
    let s = v.trim_start_matches('v');
    s.split('.').next()?.parse::<u64>().ok()
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

/// 带超时执行并取 stdout（UTF-8 lossy；wsl 相关请用 `run_with_timeout_raw` 再解码）。
pub fn run_with_timeout(cmd: &mut Command, timeout: std::time::Duration) -> Option<String> {
    let raw = run_with_timeout_raw(cmd, timeout)?;
    let t = String::from_utf8_lossy(&raw).trim().to_string();
    if t.is_empty() {
        None
    } else {
        Some(t)
    }
}

/// 登录 shell 的 PATH（zsh → bash，HOME 注入；GUI app 不含用户 PATH 的来源）。
fn login_shell_path() -> Option<String> {
    // macOS 官方登录 shell 是 zsh；Linux 一般为 bash
    for shell in ["/bin/zsh", "/bin/bash"] {
        if !Path::new(shell).is_file() {
            continue;
        }
        let mut cmd = crate::child_cmd(Path::new(shell));
        cmd.args(["-lc", "echo -n \"$PATH\""]);
        if let Some(home) = user_home_dir() {
            cmd.env("HOME", &home);
        }
        if let Some(p) = run_with_timeout(&mut cmd, std::time::Duration::from_secs(2)) {
            return Some(p);
        }
    }
    None
}

fn path_separator() -> char {
    if cfg!(windows) {
        ';'
    } else {
        ':'
    }
}

fn path_separator_string() -> &'static str {
    if cfg!(windows) {
        ";"
    } else {
        ":"
    }
}

fn join_path_dirs(dirs: impl IntoIterator<Item = PathBuf>) -> String {
    dirs.into_iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(path_separator_string())
}

/// 常见安装目录（GUI 下除登录 shell 之外的第二来源）。
fn fixed_path_dirs(home: &Path) -> Vec<PathBuf> {
    let dirs = if cfg!(windows) {
        vec![
            // npm / pnpm 的默认全局命令目录。新电脑首次下载后，下一次启动也能从这里复用。
            home.join("AppData/Roaming/npm"),
            home.join("AppData/Local/pnpm"),
            home.join(".volta/bin"),
            home.join("AppData/Local/Volta/bin"),
            home.join("scoop/shims"),
            std::env::var_os("ProgramFiles")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(r"C:\Program Files"))
                .join("nodejs"),
        ]
    } else {
        vec![
            home.join(".npm-global/bin"),
            home.join(".local/bin"),
            home.join("Library/pnpm"),
            home.join(".local/share/pnpm"),
            home.join(".volta/bin"),
            home.join(".nvm/versions/node"),
            home.join(".local/share/fnm/node-versions"),
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/opt/homebrew/sbin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin"),
        ]
    };
    dirs.into_iter().filter(|path| path.is_dir()).collect()
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

/// 合并后的探测 PATH 串（用户环境优先 → 固定目录 → 当前 PATH，去重保序）。
/// 本函数是 shell 侧环境感知的唯一入口；dsh 子进程启动时也应继承同一份。
pub fn effective_path() -> String {
    fn build() -> String {
        let home = user_home_dir().unwrap_or_default();
        merge_paths(&[
            login_shell_path().unwrap_or_default(),
            join_path_dirs(fixed_path_dirs(&home)),
            // fnm/nvm 的 Node 实际落在版本目录的 bin/；Finder 启动时 login
            // shell 不会读 .zshrc，因此不能只依赖 shell 注入的 PATH。
            join_path_dirs(fnm_nvm_bin_dirs()),
            std::env::var("PATH").unwrap_or_default(),
        ])
    }
    static CACHE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    CACHE.get_or_init(build).clone()
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

// ---------- system 探测 ----------

pub struct SystemDsh {
    /// dsh 入口：tree/lib/bin.js。
    pub bin_js: PathBuf,
    pub version: String,
    pub engines_node: Option<String>,
}

/// 在合并 PATH 上找官方安装的 dsh（npm/pnpm 全局）：`which dsh` → 解符号链 →
/// 逐级上溯找包根 → 读 package.json。找不到返回 None。
pub fn detect_system_dsh(path_env: &str) -> Option<SystemDsh> {
    // fnm/nvm 版本目录里也可能有全局 dsh（版本目录的 bin/）
    let mut dirs = path_dirs(path_env);
    for extra in fnm_nvm_bin_dirs() {
        dirs.push(extra);
    }
    let names: &[&str] = if cfg!(windows) {
        &["dsh.cmd", "dsh.exe", "dsh"]
    } else {
        &["dsh"]
    };
    let bin = dirs.into_iter().find_map(|dir| {
        names
            .iter()
            .map(|name| dir.join(name))
            .find(|candidate| candidate.is_file() && is_executable(candidate))
    });
    let Some(bin) = bin else {
        tracing::info!("system dsh 探测：PATH 中无可执行 dsh");
        return None;
    };
    // Unix shim 一般是指向 lib/bin.js 的符号链接。Windows 的 .cmd shim 不会被
    // canonicalize 到包树，因此额外从 npm/pnpm 全局 prefix 的 node_modules 查一次。
    let tree = fs::canonicalize(&bin)
        .ok()
        .and_then(|real| find_package_root(&real, "@deepseek-ai/dsh"))
        .or_else(|| find_global_package_root_from_command(&bin, "@deepseek-ai/dsh"));
    let Some(tree) = tree else {
        tracing::info!("system dsh 探测：找到 {} 但无法解析包树", bin.display());
        return None;
    };
    let manifest_path = tree.join("package.json");
    let text = fs::read_to_string(&manifest_path).ok()?;
    let pkg: serde_json::Value = serde_json::from_str(&text).ok()?;
    let version = pkg.get("version")?.as_str()?.to_string();
    let engines_node = pkg
        .get("engines")
        .and_then(|e| e.get("node"))
        .and_then(|n| n.as_str())
        .map(String::from);
    Some(SystemDsh {
        bin_js: tree.join("lib").join("bin.js"),
        version,
        engines_node,
    })
}

/// 从 npm 全局命令所在的 prefix 推导包树，覆盖 Windows 的 dsh.cmd shim。
fn find_global_package_root_from_command(command: &Path, name: &str) -> Option<PathBuf> {
    let mut dir = command.parent()?;
    for _ in 0..4 {
        let mut candidates = vec![dir.join("node_modules").join(name)];
        // pnpm setup 的 Windows 全局布局：<PNPM_HOME>/global/<major>/node_modules/<name>。
        candidates.extend((3..=10).map(|major| {
            dir.join("global")
                .join(major.to_string())
                .join("node_modules")
                .join(name)
        }));
        if let Some(candidate) = candidates
            .into_iter()
            .find(|candidate| package_root_matches(candidate, name))
        {
            return Some(candidate);
        }
        dir = dir.parent()?;
    }
    None
}

fn package_root_matches(dir: &Path, name: &str) -> bool {
    let manifest = dir.join("package.json");
    fs::read_to_string(&manifest)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|pkg| pkg.get("name").and_then(|n| n.as_str()).map(str::to_owned))
        .as_deref()
        == Some(name)
}

/// 从可执行文件路径逐级上溯，找到 name 匹配的 package.json 所在的包根。
fn find_package_root(start: &Path, name: &str) -> Option<PathBuf> {
    let mut dir = start.parent()?;
    for _ in 0..8 {
        if package_root_matches(dir, name) {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
    None
}

/// fnm/nvm 版本目录里的 bin/（node、可能的全局 dsh）；取最新版本目录。
fn fnm_nvm_bin_dirs() -> Vec<PathBuf> {
    let home = user_home_dir().unwrap_or_default();
    fnm_nvm_bin_dirs_in(&home)
}

/// 枚举指定 home 中最新 fnm/nvm Node 的可执行目录，供环境补全和测试共用。
fn fnm_nvm_bin_dirs_in(home: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let roots = if cfg!(windows) {
        vec![
            home.join("AppData/Roaming/fnm/node-versions"),
            home.join(".fnm/node-versions"),
        ]
    } else {
        vec![
            home.join(".local/share/fnm/node-versions"),
            home.join(".nvm/versions/node"),
        ]
    };
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        let mut versions: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        versions.sort_by(|a, b| {
            compare_versions_asc(
                &a.file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                &b.file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            )
        });
        if let Some(latest) = versions.last() {
            if cfg!(windows) {
                out.push(latest.join("installation"));
            }
            out.push(latest.join("installation/bin"));
            out.push(latest.join("bin"));
        }
    }
    out
}

/// PATH 上的系统 node 及其版本（"--version"）。
pub struct SystemNode {
    pub bin: PathBuf,
    pub version: String,
}

pub fn detect_system_node(path_env: &str) -> Option<SystemNode> {
    let mut dirs = path_dirs(path_env);
    for extra in fnm_nvm_bin_dirs() {
        dirs.push(extra);
    }
    let names: &[&str] = if cfg!(windows) {
        &["node.exe", "node"]
    } else {
        &["node"]
    };
    let bin = dirs.into_iter().find_map(|dir| {
        names
            .iter()
            .map(|name| dir.join(name))
            .find(|candidate| candidate.is_file() && is_executable(candidate))
    })?;
    let mut version_cmd = crate::child_cmd(&bin);
    version_cmd.arg("--version");
    let version = version_cmd.output().ok()?;
    if !version.status.success() {
        return None;
    }
    let version = String::from_utf8_lossy(&version.stdout).trim().to_string();
    if version.is_empty() {
        return None;
    }
    Some(SystemNode { bin, version })
}

/// 宽松 engines 校验：提取 engines 要求的主版本下限，与 node 主版本比较；
/// engines 缺失/解析失败视为通过（不挡复用，栅栏只拦明显不满足）。
pub fn engines_satisfied(node_version: &str, engines_node: Option<&str>) -> bool {
    let Some(req) = engines_node else { return true };
    let Some(required) = req.split(['>', '=', '<', '~', '^', ' ']).find_map(|t| {
        t.chars()
            .next()
            .filter(|c| c.is_ascii_digit())
            .and_then(|_| major_of(t))
    }) else {
        return true;
    };
    let Some(have) = major_of(node_version) else {
        return false;
    };
    // 仅当要求明确高于当前时才拒绝；"<23" 之类的上限忽略（宽松语义）。
    have >= required
}

// ---------- 解析链 ----------

/// system 档探测结果：命中 / 缺失（含 engines 不达标）/ 版本过低。
enum SystemOutcome {
    Hit(SystemHit),
    Miss,
    /// 用户已有 dsh 但低于下限：**不自动覆盖**（H：提示+经确认），携带版本信息出可行动文案。
    TooOld {
        found: String,
        min: String,
    },
}

/// 按档序解析本次启动的 LaunchSpec。download 档的下载进度经 `progress` 上抛。
pub fn resolve_launch(
    manifest: &ProductManifest,
    resources_dir: &Path,
    path_env: &str,
    data_dir: &Path,
    progress: crate::updates::DownloadProgress,
) -> Result<LaunchSpec> {
    let spec = &manifest.terminal.resolution.dsh;

    for tier in &spec.tiers {
        match tier {
            TierKind::System => match probe_system(spec, path_env) {
                SystemOutcome::Hit(hit) => {
                    let no_open = system_no_open_supported(&hit, data_dir);
                    return Ok(LaunchSpec {
                        node_bin: hit.node.bin,
                        dsh_bin_js: hit.dsh.bin_js,
                        dsh_home: user_dsh_home(),
                        profile: manifest.terminal.default_profile.clone(),
                        tier: TierKind::System,
                        no_open,
                    });
                }
                SystemOutcome::TooOld { found, min } => {
                    anyhow::bail!(
                        "您机器上的 DSH 版本过低（{found} < 终端要求 {min}）。\n\
                         终端不会自动覆盖您的全局安装；请确认后执行 \
                         `npm i -g @deepseek-ai/dsh` 升级，或安装内置档桌面版。"
                    );
                }
                SystemOutcome::Miss => {
                    tracing::info!("system 档未命中（用户环境无可用官方 dsh）");
                }
            },
            TierKind::Bundle => {
                let fb = manifest.fallback.clone().ok_or_else(|| {
                    anyhow::anyhow!(
                        "契约声明 bundle 档但缺少 fallback（自洽性校验应拦截，此属异常）"
                    )
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
                return Ok(spec);
            }
            TierKind::Download => {
                // 实时下载：node 执行器（系统优先，无则缓存下载）→ pnpm 优先、npm 回退，全局装官方最新 dsh
                let (node, tree) = crate::updates::install_latest_global(data_dir, progress)
                    .map_err(|e| anyhow::anyhow!("实时下载档失败：{e}"))?;
                return Ok(LaunchSpec {
                    node_bin: node,
                    dsh_bin_js: tree.join("lib").join("bin.js"),
                    dsh_home: user_dsh_home(),
                    profile: manifest.terminal.default_profile.clone(),
                    tier: TierKind::Download,
                    no_open: true,
                });
            }
        }
    }
    anyhow::bail!("resolution 档序为空，无法解析宿主")
}

/// system 档三重闸：dsh 树存在 + 版本 ≥ 下限 + node 可用且 engines 通过。
fn probe_system(spec: &TierSpec, path_env: &str) -> SystemOutcome {
    let Some(dsh) = detect_system_dsh(path_env) else {
        return SystemOutcome::Miss;
    };
    if let Some(min) = &spec.min_version {
        if !version_at_least(&dsh.version, min) {
            tracing::info!("system dsh 版本过低：{} < {}", dsh.version, min);
            return SystemOutcome::TooOld {
                found: dsh.version,
                min: min.clone(),
            };
        }
    }
    let Some(node) = detect_system_node(path_env) else {
        tracing::info!("system 档未命中（无系统 node，下载档会自备执行器）");
        return SystemOutcome::Miss;
    };
    if spec.require_engines && !engines_satisfied(&node.version, dsh.engines_node.as_deref()) {
        tracing::info!(
            "system node 不满足 dsh engines（node {} / {:?}）",
            node.version,
            dsh.engines_node
        );
        return SystemOutcome::Miss;
    }
    // 平台校验：system 树是就地安装的（npm 全局），架构天然一致；显式保留钩子。
    SystemOutcome::Hit(SystemHit { node, dsh })
}

struct SystemHit {
    node: SystemNode,
    dsh: SystemDsh,
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

/// bundle 档：fallback 三件套（相对 resources 根）。
fn launch_from_fallback(fb: &FallbackSpec, resources_dir: &Path, profile: String) -> LaunchSpec {
    LaunchSpec {
        node_bin: fb.resolve_path(resources_dir, &fb.node_bin),
        dsh_bin_js: fb.resolve_path(resources_dir, &fb.dsh_bin_js),
        dsh_home: fb.resolve_path(resources_dir, &fb.dsh_home),
        profile,
        tier: TierKind::Bundle,
        no_open: true,
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

/// system 档：探测该 dsh 版本是否支持 `--no-open`（`--profile web --help` 一次）。
/// 结果跨启动持久化到 `<data_dir>/probe-cache.json`（dsh 版本 → bool）：
/// `--no-open` 支持是版本特性，版本不变结果稳定——稳态启动零 spawn（2026-09-01
/// 实测：spawn 一次耗时 8s+，其中帮助文本 1.4s 即出现；缓存命中后该部分 ≈0）。
/// 探测失败 → 不支持（宁可不传，避免旧版 dsh 秒退）。
fn system_no_open_supported(hit: &SystemHit, data_dir: &Path) -> bool {
    let cache = load_probe_cache(data_dir);
    if let Some(&v) = cache.get(&hit.dsh.version) {
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
            .find(|(n, b, _)| n == &hit.node.bin && b == &hit.dsh.bin_js)
            .map(|(_, _, v)| *v)
    };
    if let Some(v) = in_proc {
        return v;
    }
    let v = probe_no_open_with(&hit.node.bin, &hit.dsh.bin_js, PROBE_NO_OPEN_TIMEOUT);
    cache2
        .lock()
        .unwrap()
        .push((hit.node.bin.clone(), hit.dsh.bin_js.clone(), v));
    let mut updated = cache;
    updated.insert(hit.dsh.version.clone(), v);
    let _ = save_probe_cache(data_dir, &updated);
    v
}

/// 探测总超时：宽松于本机实测 8~11s（Windows 冷加载更慢），远小于
/// BOOT_TIMEOUT 90s；超时按「不支持 --no-open」处理（宁可不传），
/// 探测进程卡死不再永久阻塞 boot。
const PROBE_NO_OPEN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

/// 参数化探测（超时注入，供测试）：`child_cmd` 只做 Windows 批处理包装
/// （cmd /C + CREATE_NO_WINDOW），跨平台可测。
fn probe_no_open_with(node: &Path, dsh_bin: &Path, timeout: std::time::Duration) -> bool {
    use std::io::{BufRead, BufReader};
    let mut cmd = crate::child_cmd(node);
    cmd.arg(dsh_bin)
        .args(["--profile", "web", "--help"])
        .stdout(std::process::Stdio::piped())
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
            // 进程先退出：结果是否命中看最后缓冲（已在上面的 contains 检查覆盖）。
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    // 超时：强杀兜底，绝不无限阻塞 boot。宽容窗口内可能已被/将被 read 线程
    // 释放的管道，kill 后管了就管了——查一次最终缓冲放弃精确命中（20s 已够）。
    let _ = child.kill();
    let _ = child.wait();
    false
}

// ---------- 工具 ----------

fn path_dirs(path_env: &str) -> Vec<PathBuf> {
    path_env
        .split(path_separator())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .collect()
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.metadata()
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}
#[cfg(not(unix))]
fn is_executable(_p: &Path) -> bool {
    true
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
        assert!(version_at_least("0.1.0-rc.9", "0.1.0-rc.6"));
        assert!(!version_at_least("0.1.0-rc.5", "0.1.0-rc.6"));
        assert!(version_at_least("0.1.0", "0.1.0-rc.9"));
        assert!(version_at_least("0.1.0-rc.6", "0.1.0-rc.6"));
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

    #[cfg(unix)]
    #[test]
    fn fnm_paths_include_latest_installation_bin() {
        // Windows 下 fnm 布局是 AppData/Roaming/fnm，此测试只覆盖 Unix 布局
        //（fnm_nvm_bin_dirs_in 的分支本身在 resolve 的 Windows 路径测试之外）。
        let root = tmp();
        let versions = root.join(".local/share/fnm/node-versions");
        std::fs::create_dir_all(versions.join("v20.0.0/installation/bin")).unwrap();
        std::fs::create_dir_all(versions.join("v24.18.0/installation/bin")).unwrap();

        let dirs = fnm_nvm_bin_dirs_in(&root);
        assert!(dirs.contains(&versions.join("v24.18.0/installation/bin")));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn engines_gate_is_lenient() {
        assert!(engines_satisfied("v24.18.0", Some(">=22")));
        assert!(!engines_satisfied("v20.0.0", Some(">=22")));
        assert!(engines_satisfied("v24.18.0", None));
        assert!(engines_satisfied("v24.18.0", Some("garbage"))); // 解析失败放行
    }

    #[test]
    fn find_package_root_walks_up() {
        let root = tmp();
        let pkg = root.join("lib/node_modules/@deepseek-ai/dsh");
        std::fs::create_dir_all(pkg.join("lib")).unwrap();
        std::fs::write(
            pkg.join("package.json"),
            r#"{"name": "@deepseek-ai/dsh", "version": "0.1.0-rc.8"}"#,
        )
        .unwrap();
        let bin = pkg.join("lib/bin.js");
        std::fs::write(&bin, "#!/usr/bin/env node\n").unwrap();
        assert_eq!(find_package_root(&bin, "@deepseek-ai/dsh"), Some(pkg));
        assert_eq!(find_package_root(&bin, "@deepseek-ai/nope"), None);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn windows_command_shim_resolves_npm_global_package_tree() {
        let root = tmp();
        let command = root.join("bin/dsh.cmd");
        let pkg = root.join("node_modules/@deepseek-ai/dsh");
        std::fs::create_dir_all(command.parent().unwrap()).unwrap();
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(&command, "@echo off\n").unwrap();
        std::fs::write(
            pkg.join("package.json"),
            r#"{"name":"@deepseek-ai/dsh","version":"0.1.0-rc.9"}"#,
        )
        .unwrap();
        assert_eq!(
            find_global_package_root_from_command(&command, "@deepseek-ai/dsh"),
            Some(pkg)
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn windows_pnpm_shim_resolves_global_package_tree() {
        let root = tmp();
        let command = root.join("pnpm-home/dsh.cmd");
        let pkg = root.join("pnpm-home/global/5/node_modules/@deepseek-ai/dsh");
        std::fs::create_dir_all(command.parent().unwrap()).unwrap();
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(&command, "@echo off\n").unwrap();
        std::fs::write(
            pkg.join("package.json"),
            r#"{"name":"@deepseek-ai/dsh","version":"0.1.0-rc.9"}"#,
        )
        .unwrap();
        assert_eq!(
            find_global_package_root_from_command(&command, "@deepseek-ai/dsh"),
            Some(pkg)
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn detects_system_dsh_via_fake_path() {
        use std::os::unix::fs::PermissionsExt;
        let root = tmp();
        let bindir = root.join("bin");
        let pkg = root.join("lib/node_modules/@deepseek-ai/dsh");
        std::fs::create_dir_all(&bindir).unwrap();
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::create_dir_all(pkg.join("lib")).unwrap();
        std::fs::write(
            pkg.join("package.json"),
            r#"{"name": "@deepseek-ai/dsh", "version": "0.1.0-rc.7",
                "engines": {"node": ">=22"}}"#,
        )
        .unwrap();
        std::fs::write(pkg.join("lib/bin.js"), "#!/usr/bin/env node\n// bin\n").unwrap();
        std::fs::set_permissions(pkg.join("lib/bin.js"), fs::Permissions::from_mode(0o755))
            .unwrap();
        // npm 全局形态：bin 目录里的 dsh 是指向包内入口的符号链接
        std::os::unix::fs::symlink(pkg.join("lib/bin.js"), bindir.join("dsh")).unwrap();

        let hit = detect_system_dsh(&bindir.display().to_string()).unwrap();
        assert_eq!(hit.version, "0.1.0-rc.7");
        assert_eq!(hit.engines_node.as_deref(), Some(">=22"));
        assert!(hit.bin_js.ends_with("lib/bin.js"));
        std::fs::remove_dir_all(&root).ok();

        // 无 dsh 的 PATH → None
        let empty = tmp();
        assert!(detect_system_dsh(&empty.display().to_string()).is_none());
        std::fs::remove_dir_all(&empty).ok();
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
        let m: ProductManifest = serde_json::from_str(json).unwrap();
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
        let hit = probe_no_open_with(&node, &fake_dsh(&dir), std::time::Duration::from_secs(60));
        assert!(hit, "usage 含 --no-open 应判 true");
        assert!(
            t0.elapsed() < std::time::Duration::from_secs(5),
            "命中后应 kill 早退，实测 {:?}",
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
        assert!(!probe_no_open_with(
            &node,
            &fake_dsh(&dir),
            std::time::Duration::from_secs(10)
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 卡死（无输出不退出）→ 超时兜底 false，不无限阻塞（按 PROBE 语义兜底）。
    #[cfg(unix)]
    #[test]
    fn probe_no_open_times_out_when_hung() {
        let dir = tmp();
        let node = fake_node_executor(&dir, "#!/bin/sh\nsleep 60\n");
        let t0 = std::time::Instant::now();
        let hit = probe_no_open_with(
            &node,
            &fake_dsh(&dir),
            std::time::Duration::from_millis(300),
        );
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
        let hit = probe_no_open_with(&node, &fake_dsh(&dir), std::time::Duration::from_secs(60));
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

    /// system_no_open_supported 缓存命中零 spawn：直接 pre 写缓存后调用，
    /// 断言命中且没有执行探测（用「会创建标记文件的 node」验证）。
    #[cfg(unix)]
    #[test]
    fn system_no_open_uses_cache_without_spawn() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tmp();
        let dsh_version = "0.1.1-rc.2";
        let mut cache = std::collections::HashMap::new();
        cache.insert(dsh_version.to_string(), true);
        save_probe_cache(&dir, &cache).unwrap();
        // node 假体：被 spawn 即写标记文件
        let node = dir.join("should-not-run");
        std::fs::write(&node, "#!/bin/sh\ntouch spawned-marker\n").unwrap();
        std::fs::set_permissions(&node, fs::Permissions::from_mode(0o755)).unwrap();
        let hit = SystemHit {
            node: SystemNode {
                bin: node.clone(),
                version: "v24.18.0".to_string(),
            },
            dsh: SystemDsh {
                bin_js: dir.join("nope.js"),
                version: dsh_version.to_string(),
                engines_node: None,
            },
        };
        let v = system_no_open_supported(&hit, &dir);
        assert!(v, "缓存命中应直接返回 true");
        assert!(
            !dir.join("spawned-marker").exists(),
            "缓存命中后不得 spawn 探测进程"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 缓存 miss → 探测 → 写回。
    #[cfg(unix)]
    #[test]
    fn system_no_open_miss_probes_and_writes_back() {
        let dir = tmp();
        let node = fake_node_executor(&dir, "#!/bin/sh\necho \"  --no-open    do not open\"\n");
        let hit = SystemHit {
            node: SystemNode {
                bin: node.clone(),
                version: "v24.18.0".to_string(),
            },
            dsh: SystemDsh {
                bin_js: dir.join("fake.js"),
                version: "0.1.1-rc.2".to_string(),
                engines_node: None,
            },
        };
        let v = system_no_open_supported(&hit, &dir);
        assert!(v);
        let cache = load_probe_cache(&dir);
        assert_eq!(cache.get("0.1.1-rc.2"), Some(&true), "探测后应写回缓存");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 版本变化失效：缓存 v1=true，新版本 → 重新探测（假体返回 false）。
    #[cfg(unix)]
    #[test]
    fn system_no_open_cache_invalidates_on_version_change() {
        let dir = tmp();
        let mut cache = std::collections::HashMap::new();
        cache.insert("0.1.0-rc.5".to_string(), true);
        save_probe_cache(&dir, &cache).unwrap();
        let node = fake_node_executor(&dir, "#!/bin/sh\necho \"Usage: plain\"\n");
        let hit = SystemHit {
            node: SystemNode {
                bin: node.clone(),
                version: "v24.18.0".to_string(),
            },
            dsh: SystemDsh {
                bin_js: dir.join("fake.js"),
                version: "0.1.1-rc.2".to_string(),
                engines_node: None,
            },
        };
        let v = system_no_open_supported(&hit, &dir);
        assert!(!v, "新版本未命中缓存应重新探测（假体无 --no-open → false）");
        let cache = load_probe_cache(&dir);
        assert_eq!(cache.get("0.1.0-rc.5"), Some(&true), "旧版本缓存保留");
        assert_eq!(cache.get("0.1.1-rc.2"), Some(&false), "新版本探测结果写回");
        std::fs::remove_dir_all(&dir).ok();
    }
}
