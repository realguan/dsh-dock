//! updates.rs —— 宿主 dsh 版本管理 + download 档实装（ADR-0005 H / Q2b 推论 5）。
//!
//! 壳是终端的**唯一网络面**（纪律：本模块之外不得触网）：
//!   - 版本获取：npm registry packument（镜像链 npmjs → npmmirror），排序最高 = 目标
//!     （H-1：rc 也追，不认 dist-tag）。
//!   - node 兜底：用户无 node 时下载官方 node 到**私有缓存**（不替用户全局装 node，
//!     Q2b 推论 5），充当执行器。
//!   - dsh 全局安装：用执行器跑官方 npm-cli `install -g @deepseek-ai/dsh`（b 落点：
//!     dsh 进用户全局，命令行也可用）。
//!
//! 不做的事（v1 边界，写死）：用户 dsh 已存在但低于下限 → 不自动覆盖，返回可行动
//! 文案由用户确认（H：「提示+经确认」的确认环节尚无 UI，宁可不动）。

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

use crate::resolve;

/// 下载档使用的官方 node 版本（与兜底副本对齐；LTS）。
const NODE_VERSION: &str = "v24.18.0";
/// 下载超时（秒）：registry 拉包清单 / node 二进制。
const NET_TIMEOUT_SECS: u64 = 60;

fn npm_registry_urls() -> [&'static str; 2] {
    [
        "https://registry.npmjs.org/@deepseek-ai%2Fdsh",
        "https://registry.npmmirror.com/@deepseek-ai/dsh",
    ]
}

/// 拉取 packument（镜像链逐个尝试，首个成功即返回）。
fn fetch_packument() -> Result<serde_json::Value> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(NET_TIMEOUT_SECS))
        .build();
    let mut last_err: Option<anyhow::Error> = None;
    for url in npm_registry_urls() {
        match agent.get(url).call() {
            Ok(resp) => match resp.into_string() {
                Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(v) => return Ok(v),
                    Err(e) => last_err = Some(e.into()),
                },
                Err(e) => last_err = Some(e.into()),
            },
            Err(e) => last_err = Some(anyhow::anyhow!("{url}: {e}")),
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("registry 不可达")))
}

/// 从 packument 提取版本列表（降序；复用 resolve 的 rc 语义比较器）。
pub fn parse_versions(packument: &serde_json::Value) -> Vec<String> {
    let mut vs: Vec<String> = packument
        .get("versions")
        .and_then(|v| v.as_object())
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default();
    vs.sort_by(|a, b| resolve::compare_versions_asc(a, b).reverse());
    vs
}

/// 官方最新（排序最高，rc 也追）。网络失败返回 None（调用方走人工提示路径）。
pub fn fetch_latest_version() -> Option<String> {
    let packument = fetch_packument().ok()?;
    parse_versions(&packument).into_iter().next()
}

// ---------- node 私有缓存 ----------

/// node 缓存根：<data_dir>/tools/node/<version>/。
fn cached_node_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("tools").join("node").join(NODE_VERSION)
}

/// 缓存 node 是否可用（存在 + 能报版本）。
pub fn cached_node_usable(data_dir: &Path) -> Option<PathBuf> {
    let dir = cached_node_dir(data_dir);
    let bin = node_bin_in(&dir)?;
    if !bin.is_file() {
        return None;
    }
    let out = Command::new(&bin).arg("--version").output().ok()?;
    if out.status.success() {
        Some(bin)
    } else {
        None
    }
}

/// 平台/架构 → node 发行版子目录名。
fn node_dist_dir() -> &'static str {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "darwin-arm64"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "darwin-x64"
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "linux-x64"
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "linux-arm64"
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "win-x64"
    }
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64")
    )))]
    {
        // 编译期兜底：未覆盖平台由运行期报错（G-a：macOS 首发）
        "unsupported"
    }
}

/// node dist 解压后的 bin 入口（mac/linux：bin/node；win：node.exe）。
fn node_bin_in(dir: &Path) -> Option<PathBuf> {
    let rel = if cfg!(windows) { "node.exe" } else { "bin/node" };
    // dist 目录结构：node-vX-dir/... → 里面一层
    let v1 = dir.join(rel);
    if v1.is_file() {
        return Some(v1);
    }
    // 或：顶层就是展开内容
    let v2 = dir.join("node").join(rel);
    if v2.is_file() {
        return Some(v2);
    }
    None
}

/// 下载并解压官方 node 到私有缓存（G-a：macOS 首发；Windows v1 占位）。
pub fn download_node(data_dir: &Path) -> Result<PathBuf> {
    if cfg!(windows) {
        anyhow::bail!("Windows 下载档将在后续版本提供");
    }
    if node_dist_dir() == "unsupported" {
        anyhow::bail!("当前平台（{}-{}）暂无官方 node 下载支持", std::env::consts::OS, std::env::consts::ARCH);
    }
    let dist = node_dist_dir();
    let url = format!(
        "https://nodejs.org/dist/{NODE_VERSION}/node-{NODE_VERSION}-{dist}.tar.gz"
    );
    let target = cached_node_dir(data_dir);
    fs::create_dir_all(&target).context("创建 node 缓存目录")?;

    tracing::info!("下载 node {NODE_VERSION}（{url}）…");
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(NET_TIMEOUT_SECS))
        .build();
    let resp = agent
        .get(&url)
        .call()
        .with_context(|| format!("下载 node 失败：{url}"))?;
    let mut body = vec![];
    resp.into_reader()
        .take(300 * 1024 * 1024)
        .read_to_end(&mut body)
        .context("读取 node 包失败")?;

    // 解压：tar.gz → 写入缓存目录（dist 包内一层 node-vX-dir/）
    let gz = flate2::read::GzDecoder::new(&body[..]);
    let mut ar = tar::Archive::new(gz);
    ar.unpack(&target).context("解压 node 包失败")?;

    let bin = node_bin_in(&target)
        .ok_or_else(|| anyhow::anyhow!("解压后找不到 node 可执行文件"))?;
    let out = Command::new(&bin).arg("--version").output().context("验证缓存 node")?;
    if !out.status.success() {
        fs::remove_dir_all(&target).ok();
        anyhow::bail!("缓存 node 验证失败，已清理（可重试）");
    }
    Ok(bin)
}

/// 取得可用执行器：系统 node 优先，否则下载缓存 node。
pub fn ensure_node(data_dir: &Path) -> Result<PathBuf> {
    let path_env = resolve::effective_path();
    if let Some(sys) = resolve::detect_system_node(&path_env) {
        return Ok(sys.bin);
    }
    match cached_node_usable(data_dir) {
        Some(bin) => Ok(bin),
        None => download_node(data_dir),
    }
}

// ---------- dsh 全局安装 ----------

/// 用执行器跑官方 npm-cli：`node <npm-cli> install -g @deepseek-ai/dsh[@version]`。
/// 成功返回全局包树目录（npm root -g 解析）。
pub fn install_global_dsh(node_bin: &Path, version: Option<&str>) -> Result<PathBuf> {
    let npm_cli = find_npm_cli(node_bin).ok_or_else(|| {
        anyhow::anyhow!("执行器 node 未携带 npm（发行包异常）")
    })?;
    let spec = match version {
        Some(v) => format!("@deepseek-ai/dsh@{v}"),
        None => "@deepseek-ai/dsh".to_string(),
    };
    tracing::info!("npm install -g {spec} …");
    let out = Command::new(node_bin)
        .arg(&npm_cli)
        .args(["install", "-g", &spec])
        .output()
        .with_context(|| format!("执行 npm install -g {spec}"))?;
    if !out.status.success() {
        let tail = String::from_utf8_lossy(&out.stderr)
            .lines()
            .rev()
            .take(5)
            .collect::<Vec<_>>()
            .join("\n");
        anyhow::bail!("npm install -g {spec} 失败：{tail}");
    }
    // npm root -g → <prefix>/lib/node_modules → 树目录
    let root_out = Command::new(node_bin)
        .arg(&npm_cli)
        .args(["root", "-g"])
        .output()
        .context("解析 npm 全局根")?;
    let root = String::from_utf8_lossy(&root_out.stdout).trim().to_string();
    let tree = PathBuf::from(root).join("@deepseek-ai").join("dsh");
    if !tree.join("lib/bin.js").is_file() {
        anyhow::bail!("全局安装后找不到 dsh 入口（{}）", tree.display());
    }
    Ok(tree)
}

/// 从 node 发行目录定位 npm-cli.js：bin/npm 旁路 lib/node_modules/npm/bin/npm-cli.js。
fn find_npm_cli(node_bin: &Path) -> Option<PathBuf> {
    let dir = node_bin.parent()?;
    // 缓存形态：<node_dir>/bin/node → lib/node_modules/npm（父级结构差异）
    for cand in [
        dir.join("../lib/node_modules/npm/bin/npm-cli.js"),
        dir.join("node_modules/npm/bin/npm-cli.js"),
        dir.join("npm-cli.js"),
    ] {
        if cand.is_file() {
            return Some(cand);
        }
    }
    // 系统 node（homebrew/fnm）：路径较深，走 `which npm` 兜底？——不用：执行器纪律，
    // 系统 npm 可能引向别的 node。这里直接失败，由安装命令提示手动安装。
    None
}

/// download 档完整动作：node 执行器 → 全局装 dsh（排序最高版本，rc 也追）
/// → 返回（node 执行器, 包树）。H-1：显式取列表最高版，不依赖 npm dist-tag。
pub fn install_latest_global(data_dir: &Path) -> Result<(PathBuf, PathBuf)> {
    let node = ensure_node(data_dir).context("准备 node 执行器失败")?;
    let latest = fetch_latest_version()
        .ok_or_else(|| anyhow::anyhow!("无法获取官方版本列表（registry 不可达或返回异常）"))?;
    tracing::info!("下载档将全局安装 dsh {latest}");
    let tree = install_global_dsh(&node, Some(&latest)).context("全局安装 dsh 失败")?;
    Ok((node, tree))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versions_sorted_desc_with_rc() {
        let packument: serde_json::Value = serde_json::from_str(
            r#"{"versions": {"0.1.0-rc.5": {}, "0.1.0": {}, "0.1.0-rc.9": {}, "0.1.0-rc.7": {}}}"#,
        )
        .unwrap();
        let vs = parse_versions(&packument);
        assert_eq!(vs, vec!["0.1.0", "0.1.0-rc.9", "0.1.0-rc.7", "0.1.0-rc.5"]);
    }

    #[test]
    fn dist_dir_supported_on_current() {
        assert_ne!(node_dist_dir(), "unsupported");
    }

    #[test]
    fn find_npm_cli_looks_preferred_locations() {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "dsh-shell-npm-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(dir.join("lib/node_modules/npm/bin")).unwrap();
        std::fs::write(
            dir.join("lib/node_modules/npm/bin/npm-cli.js"),
            "// npm",
        )
        .unwrap();
        let node_bin = dir.join("bin/node");
        std::fs::create_dir_all(node_bin.parent().unwrap()).unwrap();
        std::fs::write(&node_bin, "#!/bin/sh\n").unwrap();
        let cli = find_npm_cli(&node_bin).unwrap();
        assert!(cli.ends_with("lib/node_modules/npm/bin/npm-cli.js"));
        std::fs::remove_dir_all(&dir).ok();
    }
}