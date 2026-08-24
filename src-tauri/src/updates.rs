//! updates.rs —— 宿主 dsh 版本管理 + download 档实装（ADR-0005 H / Q2b 推论 5）。
//!
//! 壳是终端的**唯一网络面**（纪律：本模块之外不得触网）：
//!   - 版本获取：npm registry packument（镜像链 npmmirror → npmjs），排序最高 = 目标
//!     （H-1：rc 也追，不认 dist-tag）。
//!   - node 兜底：用户无 node 时优先从 npmmirror、再从 nodejs.org 下载到**私有缓存**
//!     （不替用户全局装 node，Q2b 推论 5），充当执行器。
//!   - dsh 全局安装：优先使用用户已有 pnpm，失败后用执行器自带的 npm-cli；两者都按
//!     npmmirror → npmjs 顺序尝试（dsh 进用户全局，命令行也可用）。
//!
//! 不做的事（v1 边界，写死）：用户 dsh 已存在但低于下限 → 不自动覆盖，返回可行动
//! 文案由用户确认（H：「提示+经确认」的确认环节尚无 UI，宁可不动）。

use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::resolve;

/// 下载档使用的官方 node 版本（与兜底副本对齐；LTS）。
const NODE_VERSION: &str = "v24.18.0";
/// 元数据请求整体超时（秒）：registry 拉包清单等小响应，整体限时合理。
const NET_TIMEOUT_SECS: u64 = 60;
/// Node 大包下载的连接超时（秒）。
const NODE_CONNECT_TIMEOUT_SECS: u64 = 10;
/// Node 大包下载的单次读超时（秒）。
///
/// 大文件不能套整体超时：40MB 在慢网络下合法地超过一分钟。改为「连接 + 单次读」
/// 双超时——只要数据持续在推，下载可以慢慢跑完；彻底停滞的连接仍会在读超时被掐断。
const NODE_READ_TIMEOUT_SECS: u64 = 60;
/// pnpm v10 默认会阻止依赖的 install/postinstall；dsh 的 native/helper 依赖必须放行。
const PNPM_BUILD_PACKAGES: [&str; 5] = [
    "@deepseek-ai/dsh-subprocess-local",
    "@google/genai",
    "koffi",
    "node-pty",
    "protobufjs",
];

/// 包管理器使用的 registry 顺序：国内镜像优先，官方源兜底。
fn package_registry_bases() -> [&'static str; 2] {
    [
        "https://registry.npmmirror.com",
        "https://registry.npmjs.org",
    ]
}

fn npm_registry_urls() -> [String; 2] {
    let bases = package_registry_bases();
    [
        format!("{}/@deepseek-ai%2Fdsh", bases[0]),
        format!("{}/@deepseek-ai%2Fdsh", bases[1]),
    ]
}

/// Node 二进制下载顺序：npmmirror CDN 优先，nodejs.org 兜底。
fn node_download_urls(dist: &str) -> [String; 2] {
    let extension = node_archive_extension(dist);
    [
        format!(
            "https://cdn.npmmirror.com/binaries/node/{NODE_VERSION}/node-{NODE_VERSION}-{dist}.{extension}"
        ),
        format!(
            "https://nodejs.org/dist/{NODE_VERSION}/node-{NODE_VERSION}-{dist}.{extension}"
        ),
    ]
}

/// Windows 官方发行包是 zip，Unix 官方发行包是 tar.gz。
fn node_archive_extension(dist: &str) -> &'static str {
    if dist.starts_with("win-") {
        "zip"
    } else {
        "tar.gz"
    }
}

/// 固定 Node 发行包的官方 SHA-256；镜像只负责分发，二进制仍必须过校验。
/// 更新 NODE_VERSION 时必须同步更新本表，值来自 nodejs.org 的 SHASUMS256.txt。
fn node_sha256(dist: &str) -> Option<&'static str> {
    match dist {
        "darwin-arm64" => Some("e1a97e14c99c803e96c7339403282ea05a499c32f8d83defe9ef5ec66f979ed1"),
        "darwin-x64" => Some("dfd0dbd3e721503434df7b7205e719f61b3a3a31b2bcf9729b8b91fea240f080"),
        "linux-arm64" => Some("6b4484c2190274175df9aa8f28e2d758a819cb1c1fe6ab481e2f95b463ab8508"),
        "linux-x64" => Some("783130984963db7ba9cbd01089eaf2c2efb055c7c1693c943174b967b3050cb8"),
        "win-arm64" => Some("f274669adb93b1fd0fbf8f21fd078609e9dcc84333d4f2718d2dde3f9a161a01"),
        "win-x64" => Some("0ae68406b42d7725661da979b1403ec9926da205c6770827f33aac9d8f26e821"),
        _ => None,
    }
}

/// packument 读取上限：防异常响应撑爆内存；正常清单远小于此。
const PACKUMENT_MAX_BYTES: u64 = 32 * 1024 * 1024;

/// 读响应体为字符串，带显式字节上限。
/// ureq 的 `into_string()` 自带内部上限且阈值随版本漂移，这里改为显式、可测的实现。
fn read_body_capped(reader: impl Read, cap: u64) -> Result<String> {
    let mut text = String::new();
    reader
        .take(cap + 1)
        .read_to_string(&mut text)
        .context("读取响应体失败")?;
    if text.len() as u64 > cap {
        anyhow::bail!("响应体超过 {cap} 字节上限");
    }
    Ok(text)
}

/// 拉取 packument（镜像链逐个尝试，首个成功即返回）。
fn fetch_packument() -> Result<serde_json::Value> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(NET_TIMEOUT_SECS))
        .build();
    let mut last_err: Option<anyhow::Error> = None;
    for url in npm_registry_urls() {
        tracing::info!("读取 dsh 版本列表：{url}");
        match agent.get(&url).call() {
            Ok(resp) => match read_body_capped(resp.into_reader(), PACKUMENT_MAX_BYTES) {
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

// ---------- 版本状态（更新检测） ----------

/// 更新检测结果（前端 chip 与托盘菜单共用）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateStatus {
    /// 当前宿主 dsh 版本（system 探测；None = 内置/未知）。
    pub current: Option<String>,
    /// registry 排序最高版本（rc 也追）。
    pub latest: Option<String>,
    /// 有新版：current < latest 且两者都已知。
    pub newer: bool,
    /// 检测失败原因（仅展示）。
    pub error: Option<String>,
}

/// 有新版判定（纯函数，供测试）。
pub fn is_newer(current: &str, latest: &str) -> bool {
    crate::resolve::compare_versions_asc(current, latest) == std::cmp::Ordering::Less
}

/// 当前宿主 dsh 版本：system 档探测（跟随启动链语义）。
pub fn detect_current_version() -> Option<String> {
    let path = crate::resolve::effective_path();
    crate::resolve::detect_system_dsh(&path).map(|d| d.version)
}

/// 一次完整检测（当前版本 + 官方最新 + 比较）。网络失败不视为致命：error 展示。
pub fn check_now() -> UpdateStatus {
    let current = detect_current_version();
    let network = fetch_latest_version();
    let (latest, error) = match network {
        Some(v) => (Some(v), None),
        None => (None, Some("registry 不可达或返回异常".to_string())),
    };
    let newer = match (&current, &latest) {
        (Some(c), Some(l)) => is_newer(c, l),
        _ => false,
    };
    UpdateStatus {
        current,
        latest,
        newer,
        error,
    }
}

// ---------- node 私有缓存 ----------

/// node 缓存根：<data_dir>/tools/node/<version>/。
fn cached_node_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("tools").join("node").join(NODE_VERSION)
}

/// 私有 Node prefix 下已安装的 dsh 包树（Unix/Windows npm root 布局均覆盖）。
fn cached_dsh_tree(data_dir: &Path) -> Option<PathBuf> {
    let prefix = cached_node_dir(data_dir);
    [
        prefix.join("lib/node_modules/@deepseek-ai/dsh"),
        prefix.join("node_modules/@deepseek-ai/dsh"),
    ]
    .into_iter()
    .find(|tree| tree.join("lib/bin.js").is_file())
}

fn package_version(tree: &Path) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(&fs::read_to_string(tree.join("package.json")).ok()?)
        .ok()?
        .get("version")
        .and_then(|version| version.as_str())
        .map(str::to_owned)
}

fn cached_dsh_usable(node: &Path, tree: &Path) -> bool {
    Command::new(node)
        .arg(tree.join("lib/bin.js"))
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
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
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        "win-arm64"
    }
    #[cfg(not(any(
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "x86_64"),
        all(target_os = "linux", target_arch = "aarch64"),
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "windows", target_arch = "aarch64")
    )))]
    {
        // 编译期兜底：未覆盖平台由运行期报错。
        "unsupported"
    }
}

/// node dist 解压后的 bin 入口（mac/linux：bin/node；win：node.exe）。
fn node_bin_in(dir: &Path) -> Option<PathBuf> {
    node_bin_in_for(dir, node_dist_dir())
}

/// 按指定发行版查找入口；拆出参数后可以在 Unix 单测 Windows zip 的目录布局。
fn node_bin_in_for(dir: &Path, dist: &str) -> Option<PathBuf> {
    let rel = if dist.starts_with("win-") {
        "node.exe"
    } else {
        "bin/node"
    };
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
    // 官方 tarball 会包一层 `node-vX.Y.Z-<platform>/`，解压目录本身不带
    // 固定名称；只在目标缓存根下查这一层，避免误扫用户目录。
    let prefix = format!("node-{NODE_VERSION}-");
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .map(|name| name.to_string_lossy().starts_with(&prefix))
                    .unwrap_or(false)
        })
        .map(|path| path.join(rel))
        .find(|path| path.is_file())
}

/// 解压 Node 官方发行包。Windows zip 条目必须经过路径约束，避免归档路径穿越。
fn extract_node_archive(body: &[u8], target: &Path, dist: &str) -> Result<()> {
    if dist.starts_with("win-") {
        let mut archive = zip::ZipArchive::new(Cursor::new(body)).context("读取 node zip")?;
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .with_context(|| format!("读取 node zip 条目 {index}"))?;
            if entry.is_symlink() {
                anyhow::bail!("Node zip 含不支持的符号链接：{}", entry.name());
            }
            let relative = entry
                .enclosed_name()
                .ok_or_else(|| anyhow::anyhow!("Node zip 含非法路径：{}", entry.name()))?;
            let destination = target.join(relative);
            if entry.is_dir() {
                fs::create_dir_all(&destination)
                    .with_context(|| format!("创建 node 目录 {}", destination.display()))?;
                continue;
            }
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("创建 node 目录 {}", parent.display()))?;
            }
            let mut output = fs::File::create(&destination)
                .with_context(|| format!("创建 node 文件 {}", destination.display()))?;
            std::io::copy(&mut entry, &mut output)
                .with_context(|| format!("解压 node 文件 {}", destination.display()))?;
        }
        return Ok(());
    }

    let gz = flate2::read::GzDecoder::new(body);
    let mut archive = tar::Archive::new(gz);
    archive.unpack(target).context("解压 node tar.gz")
}

/// 下载并解压官方 node 到私有缓存（支持 macOS/Linux/Windows，在线兜底）。
pub fn download_node(data_dir: &Path) -> Result<PathBuf> {
    if node_dist_dir() == "unsupported" {
        anyhow::bail!(
            "当前平台（{}-{}）暂无官方 node 下载支持",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
    }
    let dist = node_dist_dir();
    let expected_sha256 =
        node_sha256(dist).ok_or_else(|| anyhow::anyhow!("当前平台没有内置 Node 校验和：{dist}"))?;
    let target = cached_node_dir(data_dir);
    let urls = node_download_urls(dist);
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(NODE_CONNECT_TIMEOUT_SECS))
        .timeout_read(std::time::Duration::from_secs(NODE_READ_TIMEOUT_SECS))
        .build();
    let mut errors = Vec::new();
    for url in urls {
        // 上一个镜像可能只下载/解压了一半，下一次尝试必须从干净缓存开始。
        fs::remove_dir_all(&target).ok();
        fs::create_dir_all(&target).context("创建 node 缓存目录")?;
        tracing::info!("下载 node {NODE_VERSION}（{url}）…");
        let result = (|| -> Result<PathBuf> {
            let resp = agent
                .get(&url)
                .call()
                .with_context(|| format!("下载 node 失败：{url}"))?;
            let mut body = vec![];
            resp.into_reader()
                .take(300 * 1024 * 1024)
                .read_to_end(&mut body)
                .context("读取 node 包失败")?;
            let actual_sha256 = format!("{:x}", Sha256::digest(&body));
            if actual_sha256 != expected_sha256 {
                anyhow::bail!(
                    "Node 包 SHA-256 校验失败（期望 {expected_sha256}，实际 {actual_sha256}）"
                );
            }

            // 官方发行包内含一层 node-vX-dir/；Windows 使用 zip，Unix 使用 tar.gz。
            extract_node_archive(&body, &target, dist).context("解压 node 包失败")?;

            let bin = node_bin_in(&target)
                .ok_or_else(|| anyhow::anyhow!("解压后找不到 node 可执行文件"))?;
            let out = Command::new(&bin)
                .arg("--version")
                .output()
                .context("验证缓存 node")?;
            if !out.status.success() {
                anyhow::bail!("缓存 node 验证失败");
            }
            Ok(bin)
        })();
        match result {
            Ok(bin) => return Ok(bin),
            Err(e) => errors.push(format!("{url}: {e}")),
        }
    }
    fs::remove_dir_all(&target).ok();
    anyhow::bail!("Node 下载失败：{}", errors.join("；"));
}

/// 取得可用执行器：系统 node 优先，否则下载缓存 node。
pub fn ensure_node(data_dir: &Path) -> Result<PathBuf> {
    let path_env = resolve::effective_path();
    if let Some(sys) = resolve::detect_system_node(&path_env) {
        return Ok(sys.bin);
    }
    if let Some(bin) = cached_node_usable(data_dir) {
        return Ok(bin);
    }
    download_node(data_dir)
}

/// 只有携带 npm-cli 的 Node 才能作为下载档执行器。
fn usable_node_with_npm(node_bin: PathBuf) -> Option<PathBuf> {
    find_npm_cli(&node_bin).map(|_| node_bin)
}

/// 当 pnpm 不可用时，准备一个可执行 npm-cli 的 Node。
fn ensure_node_with_npm(data_dir: &Path) -> Result<PathBuf> {
    let path_env = resolve::effective_path();
    if let Some(sys) = resolve::detect_system_node(&path_env) {
        if let Some(bin) = usable_node_with_npm(sys.bin) {
            return Ok(bin);
        }
    }
    if let Some(bin) = cached_node_usable(data_dir) {
        if let Some(bin) = usable_node_with_npm(bin) {
            return Ok(bin);
        }
    }
    let bin = download_node(data_dir)?;
    usable_node_with_npm(bin).ok_or_else(|| anyhow::anyhow!("下载的 Node 未携带 npm-cli"))
}

// ---------- dsh 全局安装 ----------

/// 优先用 pnpm，全局安装失败再回退到执行器自带的 npm-cli；download 档可传私有 prefix。
fn install_global_dsh_with_prefix(
    node_bin: &Path,
    version: Option<&str>,
    npm_prefix: Option<&Path>,
) -> Result<PathBuf> {
    let path_env = resolve::effective_path();
    let runtime_path = resolve::path_with_bin(node_bin, &path_env);
    if let Some(pnpm_bin) = find_pnpm(&runtime_path) {
        match install_global_dsh_pnpm(&pnpm_bin, version, &runtime_path) {
            Ok(tree) => return Ok(tree),
            Err(e) => tracing::warn!("pnpm 全局安装 dsh 失败，回退 npm：{e}"),
        }
    } else {
        tracing::info!("未找到 pnpm，使用 Node 自带 npm-cli");
    }
    install_global_dsh_npm(node_bin, version, &runtime_path, npm_prefix)
}

/// 用 pnpm 全局安装官方 dsh，并返回 pnpm 全局包树。
fn install_global_dsh_pnpm(
    pnpm_bin: &Path,
    version: Option<&str>,
    path_env: &str,
) -> Result<PathBuf> {
    let spec = package_spec(version);
    let mut errors = Vec::new();
    for registry in package_registry_bases() {
        tracing::info!("pnpm add -g {spec}（registry={registry}）…");
        let mut command = Command::new(pnpm_bin);
        command.args(pnpm_install_args(registry, &spec));
        let out = command
            .env("PATH", path_env)
            .output()
            .with_context(|| format!("执行 pnpm add -g {spec}"))?;
        if !out.status.success() {
            errors.push(format!("{registry}: {}", output_detail(&out)));
            continue;
        }
        let root_out = Command::new(pnpm_bin)
            .args(["root", "--global"])
            .env("PATH", path_env)
            .output()
            .context("解析 pnpm 全局根")?;
        if !root_out.status.success() {
            errors.push(format!(
                "pnpm root --global 失败：{}",
                output_detail(&root_out)
            ));
            continue;
        }
        let root = String::from_utf8_lossy(&root_out.stdout).trim().to_string();
        let tree = PathBuf::from(root).join("@deepseek-ai").join("dsh");
        if tree.join("lib/bin.js").is_file() {
            return Ok(tree);
        }
        errors.push(format!(
            "pnpm 全局根中找不到 dsh 入口（{}）",
            tree.display()
        ));
    }
    anyhow::bail!("pnpm 安装 dsh 失败：{}", errors.join("；"));
}

/// pnpm 10 只识别 `--allow-build=<package>` 形式，必须显式拼在同一个参数中。
fn pnpm_install_args(registry: &str, spec: &str) -> Vec<String> {
    let mut args = vec![
        "add".to_string(),
        "--global".to_string(),
        "--registry".to_string(),
        registry.to_string(),
    ];
    args.extend(
        PNPM_BUILD_PACKAGES
            .iter()
            .map(|package| format!("--allow-build={package}")),
    );
    args.push(spec.to_string());
    args
}

/// 用执行器跑官方 npm-cli：`node <npm-cli> install -g @deepseek-ai/dsh[@version]`。
/// 成功返回全局包树目录（npm root -g 解析）。
fn install_global_dsh_npm(
    node_bin: &Path,
    version: Option<&str>,
    path_env: &str,
    npm_prefix: Option<&Path>,
) -> Result<PathBuf> {
    let npm_cli = find_npm_cli(node_bin)
        .ok_or_else(|| anyhow::anyhow!("执行器 node 未携带 npm（发行包异常）"))?;
    let spec = package_spec(version);
    let allow_scripts = npm_supports_allow_scripts(node_bin, &npm_cli);
    // Finder 启动的 GUI 没有终端的完整 PATH；npm 的依赖安装脚本会通过
    // `node ...` 启动子命令，必须显式继承壳侧补全后的用户 PATH，否则会以
    // `node: command not found`（exit 127）失败。
    let mut errors = Vec::new();
    for registry in package_registry_bases() {
        tracing::info!("npm install -g {spec}（registry={registry}）…");
        let mut command = Command::new(node_bin);
        command.arg(&npm_cli);
        if let Some(prefix) = npm_prefix {
            // CLI 参数优先级高于项目/用户 .npmrc，避免旧 prefix 覆盖私有目录。
            command.arg("--prefix").arg(prefix);
        }
        command
            .args(npm_install_args(registry, &spec, allow_scripts))
            .env("PATH", path_env)
            // dsh 需要 native/helper postinstall；不能被用户旧 .npmrc 的
            // ignore-scripts 设定静默跳过，否则“安装成功”后仍会在启动时失败。
            .env("NPM_CONFIG_IGNORE_SCRIPTS", "false");
        if let Some(prefix) = npm_prefix {
            command.env("NPM_CONFIG_PREFIX", prefix);
        }
        let out = command
            .output()
            .with_context(|| format!("执行 npm install -g {spec}"))?;
        if !out.status.success() {
            errors.push(format!("{registry}: {}", output_detail(&out)));
            continue;
        }
        if let Some(tree) = global_dsh_tree_from_npm(node_bin, &npm_cli, path_env, npm_prefix) {
            return Ok(tree);
        }
        errors.push("安装成功但找不到 dsh 入口".to_string());
    }
    anyhow::bail!("npm 安装 dsh 失败：{}", errors.join("；"));
}

/// npm 11 引入 install script allowlist；旧 npm 没有该参数，不能盲目传入。
fn npm_supports_allow_scripts(node_bin: &Path, npm_cli: &Path) -> bool {
    let Ok(output) = Command::new(node_bin)
        .arg(npm_cli)
        .arg("--version")
        .output()
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .split('.')
        .next()
        .and_then(|major| major.parse::<u64>().ok())
        .map(|major| major >= 11)
        .unwrap_or(false)
}

/// 构造 npm 全局安装参数；npm 11 放行 dsh 的 native/helper 安装脚本。
fn npm_install_args(registry: &str, spec: &str, allow_scripts: bool) -> Vec<String> {
    let mut args = vec![
        "install".to_string(),
        "--global".to_string(),
        "--registry".to_string(),
        registry.to_string(),
    ];
    if allow_scripts {
        args.extend(
            PNPM_BUILD_PACKAGES
                .iter()
                .map(|package| format!("--allow-scripts={package}")),
        );
    }
    args.push(spec.to_string());
    args
}

fn package_spec(version: Option<&str>) -> String {
    match version {
        Some(v) => format!("@deepseek-ai/dsh@{v}"),
        None => "@deepseek-ai/dsh".to_string(),
    }
}

/// npm root -g → <prefix>/lib/node_modules → 树目录。
fn global_dsh_tree_from_npm(
    node_bin: &Path,
    npm_cli: &Path,
    path_env: &str,
    npm_prefix: Option<&Path>,
) -> Option<PathBuf> {
    let mut command = Command::new(node_bin);
    command.arg(npm_cli);
    if let Some(prefix) = npm_prefix {
        command.arg("--prefix").arg(prefix);
    }
    command.args(["root", "-g"]).env("PATH", path_env);
    if let Some(prefix) = npm_prefix {
        command.env("NPM_CONFIG_PREFIX", prefix);
    }
    let root_out = command.output().ok()?;
    if !root_out.status.success() {
        return None;
    }
    let root = String::from_utf8_lossy(&root_out.stdout).trim().to_string();
    let tree = PathBuf::from(root).join("@deepseek-ai").join("dsh");
    if !tree.join("lib/bin.js").is_file() {
        return None;
    }
    Some(tree)
}

fn output_detail(out: &Output) -> String {
    let stderr = String::from_utf8_lossy(&out.stderr)
        .lines()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    let stdout = String::from_utf8_lossy(&out.stdout)
        .lines()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    let detail = [stderr, stdout]
        .into_iter()
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if detail.is_empty() {
        format!("退出码 {:?}", out.status.code())
    } else {
        detail
    }
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

/// 在 GUI 补全后的 PATH 中定位 pnpm；不调用 shell，避免 Finder 环境下丢失用户 PATH。
fn find_pnpm(path_env: &str) -> Option<PathBuf> {
    let separator = if cfg!(windows) { ';' } else { ':' };
    let names: &[&str] = if cfg!(windows) {
        &["pnpm.cmd", "pnpm.exe", "pnpm"]
    } else {
        &["pnpm"]
    };
    path_env
        .split(separator)
        .filter(|dir| !dir.is_empty())
        .flat_map(|dir| names.iter().map(move |name| PathBuf::from(dir).join(name)))
        .find(|path| {
            if !path.is_file() {
                return false;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                path.metadata()
                    .map(|m| m.permissions().mode() & 0o111 != 0)
                    .unwrap_or(false)
            }
            #[cfg(not(unix))]
            {
                true
            }
        })
}

/// download 档完整动作：node 执行器 → 全局装 dsh（排序最高版本，rc 也追）
/// → 返回（node 执行器, 包树）。H-1：显式取列表最高版，不依赖 npm dist-tag。
pub fn install_latest_global(data_dir: &Path) -> Result<(PathBuf, PathBuf)> {
    let node = ensure_node(data_dir).context("准备 node 执行器失败")?;
    let private_prefix = cached_node_dir(data_dir);
    let node_is_private = node.starts_with(&private_prefix);
    let npm_prefix = node_is_private.then_some(private_prefix.as_path());
    let latest = fetch_latest_version()
        .ok_or_else(|| anyhow::anyhow!("无法获取官方版本列表（registry 不可达或返回异常）"))?;
    tracing::info!("下载档将全局安装 dsh {latest}");
    if node_is_private {
        if let Some(tree) = cached_dsh_tree(data_dir) {
            if package_version(&tree).as_deref() == Some(latest.as_str())
                && cached_dsh_usable(&node, &tree)
            {
                tracing::info!("复用私有 prefix 中已安装的 dsh {latest}");
                return Ok((node, tree));
            }
        }
    }
    match install_global_dsh_with_prefix(&node, Some(&latest), npm_prefix) {
        Ok(tree) => Ok((node, tree)),
        Err(first_error) if !node_is_private => {
            // 系统 Node/npm 可能存在权限、旧 npm 或用户配置问题；切换到私有 Node，
            // 并把 npm prefix 固定在应用数据目录，保证新电脑无需管理员权限。
            tracing::warn!("系统执行器安装 dsh 失败，准备私有 npm 执行器：{first_error}");
            let npm_node = if find_npm_cli(&node).is_none() {
                ensure_node_with_npm(data_dir).context("准备 npm 执行器失败")?
            } else {
                download_node(data_dir).context("准备私有 npm 执行器失败")?
            };
            let tree =
                install_global_dsh_with_prefix(&npm_node, Some(&latest), Some(&private_prefix))
                    .context("全局安装 dsh 失败")?;
            Ok((npm_node, tree))
        }
        Err(e) => Err(e).context("全局安装 dsh 失败"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(label: &str) -> PathBuf {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "dsh-dock-{label}-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn domestic_mirrors_are_first_with_official_fallbacks() {
        assert_eq!(
            package_registry_bases()[0],
            "https://registry.npmmirror.com"
        );
        assert_eq!(package_registry_bases()[1], "https://registry.npmjs.org");
        assert!(npm_registry_urls()[0].starts_with("https://registry.npmmirror.com/"));
        assert!(npm_registry_urls()[1].starts_with("https://registry.npmjs.org/"));

        let node_urls = node_download_urls("darwin-arm64");
        assert!(node_urls[0].starts_with("https://cdn.npmmirror.com/binaries/node/"));
        assert!(node_urls[1].starts_with("https://nodejs.org/dist/"));
        assert!(node_urls[0].ends_with(".tar.gz"));

        let windows_urls = node_download_urls("win-x64");
        assert!(windows_urls[0].starts_with("https://cdn.npmmirror.com/binaries/node/"));
        assert!(windows_urls[1].starts_with("https://nodejs.org/dist/"));
        assert!(windows_urls.iter().all(|url| url.ends_with(".zip")));
    }

    #[test]
    fn body_reader_enforces_explicit_cap() {
        assert_eq!(
            read_body_capped(Cursor::new(b"hello".to_vec()), 8).unwrap(),
            "hello"
        );
        assert!(read_body_capped(Cursor::new(vec![b'a'; 10]), 8).is_err());
    }

    #[test]
    fn node_mirror_downloads_have_pinned_checksums() {
        assert_eq!(node_sha256("darwin-arm64").unwrap().len(), 64);
        assert_eq!(node_sha256("darwin-x64").unwrap().len(), 64);
        assert_eq!(node_sha256("win-x64").unwrap().len(), 64);
        assert_eq!(node_sha256("win-arm64").unwrap().len(), 64);
        assert!(node_sha256("unknown").is_none());
    }

    #[test]
    fn finds_node_inside_official_versioned_archive_directory() {
        let dir = tmp("node-archive");
        let archive = dir.join(format!("node-{NODE_VERSION}-{}", node_dist_dir()));
        let bin = archive.join(if cfg!(windows) {
            "node.exe"
        } else {
            "bin/node"
        });
        std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
        std::fs::write(&bin, "node").unwrap();
        assert_eq!(node_bin_in(&dir), Some(bin));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn finds_cached_dsh_in_private_npm_prefix() {
        let data = tmp("cached-dsh");
        let tree = data
            .join("tools/node")
            .join(NODE_VERSION)
            .join("lib/node_modules/@deepseek-ai/dsh");
        std::fs::create_dir_all(tree.join("lib")).unwrap();
        std::fs::write(tree.join("lib/bin.js"), "// dsh").unwrap();
        std::fs::write(
            tree.join("package.json"),
            r#"{"name":"@deepseek-ai/dsh","version":"0.1.1-rc.2"}"#,
        )
        .unwrap();
        assert_eq!(cached_dsh_tree(&data), Some(tree.clone()));
        assert_eq!(package_version(&tree).as_deref(), Some("0.1.1-rc.2"));
        std::fs::remove_dir_all(&data).ok();
    }

    #[test]
    fn finds_node_inside_windows_zip_archive_directory() {
        let dir = tmp("node-windows-archive");
        let archive = dir.join(format!("node-{NODE_VERSION}-win-x64"));
        let bin = archive.join("node.exe");
        std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
        std::fs::write(&bin, "node").unwrap();
        assert_eq!(node_bin_in_for(&dir, "win-x64"), Some(bin));
        assert_eq!(node_archive_extension("win-x64"), "zip");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn extracts_windows_zip_into_private_cache() {
        use std::io::Write as _;

        let target = tmp("node-windows-zip");
        let mut bytes = Cursor::new(Vec::new());
        {
            let mut archive = zip::ZipWriter::new(&mut bytes);
            archive
                .start_file(
                    format!("node-{NODE_VERSION}-win-x64/node.exe"),
                    zip::write::SimpleFileOptions::default(),
                )
                .unwrap();
            archive.write_all(b"node").unwrap();
            archive.finish().unwrap();
        }

        extract_node_archive(bytes.get_ref(), &target, "win-x64").unwrap();
        assert!(target
            .join(format!("node-{NODE_VERSION}-win-x64/node.exe"))
            .is_file());
        std::fs::remove_dir_all(&target).ok();
    }

    #[test]
    fn pnpm_allowlist_covers_dsh_native_and_helper_dependencies() {
        assert!(PNPM_BUILD_PACKAGES.contains(&"node-pty"));
        assert!(PNPM_BUILD_PACKAGES.contains(&"koffi"));
        assert!(PNPM_BUILD_PACKAGES.contains(&"@deepseek-ai/dsh-subprocess-local"));
    }

    #[test]
    fn pnpm_install_args_use_allow_build_equals_form() {
        let args = pnpm_install_args(
            "https://registry.npmmirror.com",
            "@deepseek-ai/dsh@0.1.1-rc.2",
        );
        assert_eq!(
            &args[..4],
            [
                "add",
                "--global",
                "--registry",
                "https://registry.npmmirror.com"
            ]
        );
        assert!(args.iter().any(|arg| arg == "--allow-build=node-pty"));
        assert!(args.iter().any(|arg| arg == "--allow-build=koffi"));
        assert_eq!(
            args.last().map(String::as_str),
            Some("@deepseek-ai/dsh@0.1.1-rc.2")
        );
        assert!(!args.iter().any(|arg| arg == "--allow-build"));
    }

    #[test]
    fn npm_install_args_allow_native_scripts_only_for_new_npm() {
        let enabled = npm_install_args(
            "https://registry.npmmirror.com",
            "@deepseek-ai/dsh@0.1.1-rc.2",
            true,
        );
        assert!(enabled.iter().any(|arg| arg == "--allow-scripts=node-pty"));
        assert_eq!(
            enabled.last().map(String::as_str),
            Some("@deepseek-ai/dsh@0.1.1-rc.2")
        );

        let legacy = npm_install_args(
            "https://registry.npmjs.org",
            "@deepseek-ai/dsh@0.1.1-rc.2",
            false,
        );
        assert!(!legacy.iter().any(|arg| arg.starts_with("--allow-scripts")));
    }

    #[cfg(unix)]
    #[test]
    fn pnpm_install_runs_first_mirror_and_resolves_global_tree() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tmp("pnpm-install");
        let pnpm = dir.join("pnpm");
        let script = r#"#!/bin/sh
set -eu
root="$(dirname "$0")/global/node_modules"
if [ "$1" = "root" ]; then
  printf '%s\n' "$root"
  exit 0
fi
printf '%s\n' "$@" > "$(dirname "$0")/args.log"
mkdir -p "$root/@deepseek-ai/dsh/lib"
printf '%s\n' '// dsh entry' > "$root/@deepseek-ai/dsh/lib/bin.js"
"#;
        std::fs::write(&pnpm, script).unwrap();
        std::fs::set_permissions(&pnpm, fs::Permissions::from_mode(0o755)).unwrap();

        let path_env = format!("{}:/usr/bin:/bin", dir.display());
        let tree = install_global_dsh_pnpm(&pnpm, Some("0.1.1-rc.2"), &path_env).unwrap();
        assert_eq!(tree, dir.join("global/node_modules/@deepseek-ai/dsh"));
        let args = std::fs::read_to_string(dir.join("args.log")).unwrap();
        assert!(args.contains("https://registry.npmmirror.com"));
        assert!(args.contains("--allow-build=node-pty"));
        assert!(args.contains("@deepseek-ai/dsh@0.1.1-rc.2"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn private_npm_prefix_precedes_install_subcommand() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tmp("npm-private-prefix");
        let node = dir.join("bin/node");
        let npm_cli = dir.join("lib/node_modules/npm/bin/npm-cli.js");
        let prefix = dir.join("private-prefix");
        std::fs::create_dir_all(node.parent().unwrap()).unwrap();
        std::fs::create_dir_all(npm_cli.parent().unwrap()).unwrap();
        std::fs::write(&npm_cli, "// npm cli").unwrap();

        // 以 shell 脚本模拟 Node：首参为 npm-cli，随后检查 npm 的参数顺序。
        // 私有 prefix 放在 install 之后时，npm 11 的部分运行场景会把它当成包参数。
        let script = r#"#!/bin/sh
set -eu
root="$(dirname "$0")/.."
shift
if [ "$1" = "--version" ]; then
  printf '11.16.0\n'
  exit 0
fi
printf '%s\n' "$@" > "$root/npm-args.log"
if [ "$1" != "--prefix" ]; then
  exit 9
fi
prefix="$2"
shift 2
case "$1" in
  install)
    mkdir -p "$prefix/lib/node_modules/@deepseek-ai/dsh/lib"
    printf '%s\n' '// dsh entry' > "$prefix/lib/node_modules/@deepseek-ai/dsh/lib/bin.js"
    ;;
  root)
    printf '%s\n' "$prefix/lib/node_modules"
    ;;
  *)
    exit 8
    ;;
esac
"#;
        std::fs::write(&node, script).unwrap();
        std::fs::set_permissions(&node, fs::Permissions::from_mode(0o755)).unwrap();

        let tree =
            install_global_dsh_npm(&node, Some("0.1.1-rc.2"), "/usr/bin:/bin", Some(&prefix))
                .unwrap();
        assert_eq!(tree, prefix.join("lib/node_modules/@deepseek-ai/dsh"));
        let args = std::fs::read_to_string(dir.join("npm-args.log")).unwrap();
        let lines = args.lines().collect::<Vec<_>>();
        assert_eq!(lines[..3], ["--prefix", prefix.to_str().unwrap(), "root"]);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn node_without_npm_is_not_usable_for_download_tier() {
        let dir = tmp("node-without-npm");
        let node = dir.join("bin/node");
        std::fs::create_dir_all(node.parent().unwrap()).unwrap();
        std::fs::write(&node, "#!/bin/sh\n").unwrap();
        assert!(usable_node_with_npm(node.clone()).is_none());

        std::fs::create_dir_all(dir.join("lib/node_modules/npm/bin")).unwrap();
        std::fs::write(dir.join("lib/node_modules/npm/bin/npm-cli.js"), "// npm").unwrap();
        assert_eq!(usable_node_with_npm(node), Some(dir.join("bin/node")));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn selected_node_is_prepended_to_package_manager_path() {
        let node = Path::new("/private/dsh-dock/node/bin/node");
        let path = resolve::path_with_bin(node, "/usr/local/bin:/usr/bin");
        let separator = if cfg!(windows) { ';' } else { ':' };
        assert_eq!(
            path.split(separator).next(),
            Some("/private/dsh-dock/node/bin")
        );
    }

    #[cfg(unix)]
    #[test]
    fn finds_pnpm_in_gui_path() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tmp("pnpm-path");
        let pnpm = dir.join("pnpm");
        std::fs::write(&pnpm, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&pnpm, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(find_pnpm(&dir.display().to_string()), Some(pnpm));
        std::fs::remove_dir_all(&dir).ok();
    }

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
    fn newer_detection_uses_rc_ordering() {
        assert!(is_newer("0.1.0-rc.6", "0.1.1-rc.2"));
        assert!(is_newer("0.1.1-rc.2", "0.1.1"));
        assert!(!is_newer("0.1.1-rc.2", "0.1.1-rc.2"));
        assert!(!is_newer("0.1.1", "0.1.1-rc.9"));
    }

    #[test]
    fn dist_dir_supported_on_current() {
        assert_ne!(node_dist_dir(), "unsupported");
    }

    #[test]
    fn find_npm_cli_looks_preferred_locations() {
        let dir = tmp("npm-cli");
        std::fs::create_dir_all(dir.join("lib/node_modules/npm/bin")).unwrap();
        std::fs::write(dir.join("lib/node_modules/npm/bin/npm-cli.js"), "// npm").unwrap();
        let node_bin = dir.join("bin/node");
        std::fs::create_dir_all(node_bin.parent().unwrap()).unwrap();
        std::fs::write(&node_bin, "#!/bin/sh\n").unwrap();
        let cli = find_npm_cli(&node_bin).unwrap();
        assert!(cli.ends_with("lib/node_modules/npm/bin/npm-cli.js"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn find_npm_cli_supports_windows_node_zip_layout() {
        let dir = tmp("npm-cli-windows");
        std::fs::create_dir_all(dir.join("node_modules/npm/bin")).unwrap();
        std::fs::write(dir.join("node_modules/npm/bin/npm-cli.js"), "// npm").unwrap();
        let node_bin = dir.join("node.exe");
        std::fs::write(&node_bin, "node").unwrap();
        assert_eq!(
            find_npm_cli(&node_bin),
            Some(dir.join("node_modules/npm/bin/npm-cli.js"))
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
