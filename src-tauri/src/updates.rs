//! updates.rs —— 宿主 dsh 版本管理 + download 档实装（docs/contract.md「运行时策略」）。
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
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Output;

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
/// 下载进度回调：`(已传输字节, 总字节)`；服务器未报长度时总字节为 None。
/// updates 模块保持零 tauri 依赖——进度经回调上抛，由 lib.rs 桥接为事件。
pub type DownloadProgress<'a> = &'a mut dyn FnMut(u64, Option<u64>);
/// 单次读块大小：64KB 平滑驱动进度回调。
const DOWNLOAD_CHUNK: usize = 64 * 1024;
/// 发行包体积上限（防异常响应无限写盘）。
const NODE_ARCHIVE_MAX: u64 = 300 * 1024 * 1024;
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
fn node_download_urls(dist: &str, version: &str) -> [String; 2] {
    let extension = node_archive_extension(dist);
    [
        format!(
            "https://cdn.npmmirror.com/binaries/node/{version}/node-{version}-{dist}.{extension}"
        ),
        format!("https://nodejs.org/dist/{version}/node-{version}-{dist}.{extension}"),
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

// ---------- Node 版本映射（远程签名映射 → 本地缓存 → 内置基线） ----------

/// 映射包名（scoped 包发布到 npm；发布与密钥流程见 node-map/README.md）。
const NODE_MAP_PACKAGE: &str = "@dsh-dock/node-map";
/// 映射包体积上限（正常 <10KB）。
const NODE_MAP_MAX_BYTES: u64 = 1024 * 1024;
/// 钉在壳内的 ed25519 公钥（hex，32 字节裸钥）。私钥只在 CI secret / 本地 gitignore 文件。
/// 轮换流程：node scripts/gen-key.mjs → 换此常量发新壳。
const NODE_MAP_PUBKEY_HEX: &str =
    "f16247b0471d0695e9db849515aa2ff04e85b751be84d5ececcfa6b6d2eb8670";

/// Node 下载计划：版本 + 各平台 SHA-256。
#[derive(Debug, Clone)]
pub struct NodePlan {
    pub version: String,
    /// dist（如 darwin-arm64）→ 官方 SHA-256。
    pub checksums: std::collections::HashMap<String, String>,
    /// 采纳来源（诊断用）：remote / cache / builtin。
    pub source: &'static str,
}

impl NodePlan {
    fn sha256_for(&self, dist: &str) -> Option<&str> {
        self.checksums.get(dist).map(String::as_str)
    }
}

/// 内置基线（fail-closed 的兜底；与 node-map/map.json 初始内容一致）。
fn builtin_node_plan() -> NodePlan {
    let mut checksums = std::collections::HashMap::new();
    for dist in [
        "darwin-arm64",
        "darwin-x64",
        "linux-arm64",
        "linux-x64",
        "win-arm64",
        "win-x64",
    ] {
        if let Some(sha) = node_sha256(dist) {
            checksums.insert(dist.to_string(), sha.to_string());
        }
    }
    NodePlan {
        version: NODE_VERSION.to_string(),
        checksums,
        source: "builtin",
    }
}

/// hex → bytes（小写/大写均可；长度奇数或非法字符 → None）。
fn decode_hex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.is_empty() || s.len() % 2 != 0 {
        return None;
    }
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok())
        .collect()
}

/// ed25519 验签（对 map.json 的原始字节；sig 为 hex 文本）。
fn verify_map_signature(map_bytes: &[u8], sig_hex: &str) -> bool {
    verify_signature_with(NODE_MAP_PUBKEY_HEX, map_bytes, sig_hex)
}

/// `verify_map_signature` 的参数化内层（公钥可注入，供测试）。
fn verify_signature_with(pubkey_hex: &str, msg: &[u8], sig_hex: &str) -> bool {
    let Some(raw) = decode_hex(pubkey_hex) else {
        return false;
    };
    let Ok(publishing_key) = <[u8; 32]>::try_from(raw) else {
        return false;
    };
    let Ok(publishing_key) = ed25519_dalek::VerifyingKey::from_bytes(&publishing_key) else {
        return false;
    };
    let Some(sig) = decode_hex(sig_hex) else {
        return false;
    };
    let Ok(sig) = ed25519_dalek::Signature::from_slice(&sig) else {
        return false;
    };
    use ed25519_dalek::Verifier;
    publishing_key.verify(msg, &sig).is_ok()
}

/// 解析并校验映射内容（format / 版本形态 / minShellVersion / 六平台全覆盖）。
/// 任何一项不合法 → None（宁可回退内置，不采不完整映射）。
fn parse_node_plan(map: &[u8]) -> Option<NodePlan> {
    let v: serde_json::Value = serde_json::from_slice(map).ok()?;
    if v.get("format")?.as_u64()? != 1 {
        return None;
    }
    let version = v.get("nodeVersion")?.as_str()?.to_string();
    if !version.starts_with('v') || version.len() < 3 {
        return None;
    }
    if let Some(min) = v.get("minShellVersion").and_then(|m| m.as_str()) {
        if resolve::compare_versions_asc(env!("CARGO_PKG_VERSION"), min) == std::cmp::Ordering::Less
        {
            return None;
        }
    }
    let artifacts = v.get("artifacts")?.as_object()?;
    let mut checksums = std::collections::HashMap::new();
    for (dist, info) in artifacts {
        let sha = info.get("sha256")?.as_str()?;
        if sha.len() != 64 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        checksums.insert(dist.clone(), sha.to_ascii_lowercase());
    }
    for dist in [
        "darwin-arm64",
        "darwin-x64",
        "linux-arm64",
        "linux-x64",
        "win-arm64",
        "win-x64",
    ] {
        if !checksums.contains_key(dist) {
            return None;
        }
    }
    Some(NodePlan {
        version,
        checksums,
        source: "remote",
    })
}

/// 拉映射包：packument（dist-tags.latest → tarball URL）→ tarball → 内存解包。
/// 走既有 registry 镜像链，不引入新 CDN 语义。
fn fetch_node_map() -> Option<(Vec<u8>, String)> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(NET_TIMEOUT_SECS))
        .build();
    for base in package_registry_bases() {
        // scoped 包在 registry URL 里必须把 `/` 编码为 %2F（与 npm CLI 行为一致）。
        let packument_url = format!("{base}/{}", NODE_MAP_PACKAGE.replace('/', "%2F"));
        let Ok(resp) = agent.get(&packument_url).call() else {
            continue;
        };
        let Ok(text) = read_body_capped(resp.into_reader(), PACKUMENT_MAX_BYTES) else {
            continue;
        };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let latest = v
            .get("dist-tags")
            .and_then(|t| t.get("latest"))
            .and_then(|l| l.as_str())?;
        let tarball = v
            .get("versions")
            .and_then(|vs| vs.get(latest))
            .and_then(|ver| ver.get("dist"))
            .and_then(|d| d.get("tarball"))
            .and_then(|t| t.as_str())?;
        let Ok(resp) = agent.get(tarball).call() else {
            continue;
        };
        let mut tgz = vec![];
        resp.into_reader()
            .take(NODE_MAP_MAX_BYTES)
            .read_to_end(&mut tgz)
            .ok()?;
        if let Some(found) = extract_node_map_files(&tgz) {
            return Some(found);
        }
    }
    None
}

/// 从 npm tarball（gzip tar）中取 `package/map.json` 与 `package/map.json.sig`。
fn extract_node_map_files(tgz: &[u8]) -> Option<(Vec<u8>, String)> {
    let gz = flate2::read::GzDecoder::new(tgz);
    let mut archive = tar::Archive::new(gz);
    let mut map: Option<Vec<u8>> = None;
    let mut sig: Option<String> = None;
    for entry in archive.entries().ok()? {
        let mut entry = entry.ok()?;
        let path = entry
            .path()
            .ok()?
            .to_string_lossy()
            .trim_start_matches("./")
            .to_string();
        match path.as_str() {
            "package/map.json" => {
                let mut bytes = vec![];
                entry.read_to_end(&mut bytes).ok()?;
                map = Some(bytes);
            }
            "package/map.json.sig" => {
                let mut text = String::new();
                entry.read_to_string(&mut text).ok()?;
                sig = Some(text);
            }
            _ => {}
        }
    }
    Some((map?, sig?))
}

/// 解析本次进程的 Node 下载计划（OnceLock 保证全程只解析一次）。
/// 链路：远程签名映射 → 本地缓存（重验签）→ 内置基线。任何失败 fail-closed。
pub fn node_plan(data_dir: &Path) -> NodePlan {
    static PLAN: std::sync::OnceLock<NodePlan> = std::sync::OnceLock::new();
    PLAN.get_or_init(|| resolve_node_plan(data_dir)).clone()
}

fn node_map_cache_paths(data_dir: &Path) -> (PathBuf, PathBuf) {
    (
        data_dir.join("node-map.json"),
        data_dir.join("node-map.json.sig"),
    )
}

fn resolve_node_plan(data_dir: &Path) -> NodePlan {
    // ① 远程：拉包 → 验签 → 校验内容；通过即采纳并写缓存（写失败不影响采纳）。
    if let Some((map_bytes, sig_text)) = fetch_node_map() {
        if verify_map_signature(&map_bytes, &sig_text) {
            if let Some(plan) = parse_node_plan(&map_bytes) {
                let (cache, cache_sig) = node_map_cache_paths(data_dir);
                let _ = fs::write(&cache, &map_bytes);
                let _ = fs::write(&cache_sig, &sig_text);
                tracing::info!(
                    "Node 映射：{}（来源 {}，dist {} 条）",
                    plan.version,
                    plan.source,
                    plan.checksums.len()
                );
                return plan;
            }
            tracing::warn!("Node 映射内容不合法，回退内置基线");
        } else {
            tracing::warn!("Node 映射验签失败，回退内置基线");
        }
    }
    // ② 本地缓存（上次验签通过的副本；本地文件可能被动过，重验签再信）。
    let (cache, cache_sig) = node_map_cache_paths(data_dir);
    if let (Ok(map_bytes), Ok(sig_text)) = (fs::read(&cache), fs::read_to_string(&cache_sig)) {
        if verify_map_signature(&map_bytes, &sig_text) {
            if let Some(plan) = parse_node_plan(&map_bytes) {
                let mut plan = plan;
                plan.source = "cache";
                tracing::info!("Node 映射：{}（来源 cache）", plan.version);
                return plan;
            }
        }
    }
    // ③ 内置基线。
    tracing::info!("Node 映射：内置基线 {}", NODE_VERSION);
    builtin_node_plan()
}

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
                // read_body_capped 已返回 anyhow::Error，无需再转换
                Err(e) => last_err = Some(e),
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

/// 单个可升级组件的版本维度（dsh 本体 / 桌面客户端）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ComponentUpdate {
    pub current: Option<String>,
    pub latest: Option<String>,
    pub newer: bool,
    pub error: Option<String>,
}

/// Node 运行时维度（只读信息，无升级动作——版本由下载计划决定）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct NodeRuntimeInfo {
    pub version: String,
    /// system = 复用用户已装的 node；managed = 应用私有缓存（下载档自备）。
    pub origin: &'static str,
}

/// 更新检测聚合：dsh 本体 + 桌面客户端 + Node 运行时 三维度（boot:update 载荷）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct UpdateStatus {
    pub dsh: ComponentUpdate,
    pub client: ComponentUpdate,
    pub node: Option<NodeRuntimeInfo>,
}

/// 客户端自身的更新源（GitHub Releases 的 latest API）。
/// None = 客户端维度只显示当前版本、不出检查入口。
const APP_RELEASE_FEED: Option<&str> =
    Some("https://api.github.com/repos/realguan/dsh-dock/releases/latest");

/// 有新版判定（纯函数，供测试）。
pub fn is_newer(current: &str, latest: &str) -> bool {
    crate::resolve::compare_versions_asc(current, latest) == std::cmp::Ordering::Less
}

/// 从 GitHub Releases API 响应提取最新版本号（`v` 前缀剥掉；纯函数，供测试）。
fn parse_release_tag(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    v.get("tag_name")?
        .as_str()?
        .trim()
        .trim_start_matches('v')
        .to_string()
        .into()
}

/// 客户端最新版（feed 未配置 → None，不触网）。
fn fetch_client_latest() -> Option<String> {
    let url = APP_RELEASE_FEED?;
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(NET_TIMEOUT_SECS))
        .build();
    let resp = agent
        .get(url)
        .set("User-Agent", "dsh-dock-updater")
        .call()
        .ok()?;
    let text = read_body_capped(resp.into_reader(), 1024 * 1024).ok()?;
    parse_release_tag(&text)
}

/// Node 运行时维度：与 ensure_node 同一优先级（系统 node 优先，其次托管计划）。
fn node_runtime_info(data_dir: &Path) -> Option<NodeRuntimeInfo> {
    let path_env = resolve::effective_path();
    if let Some(sys) = resolve::detect_system_node(&path_env) {
        return Some(NodeRuntimeInfo {
            version: sys.version,
            origin: "system",
        });
    }
    Some(NodeRuntimeInfo {
        version: node_plan(data_dir).version,
        origin: "managed",
    })
}

/// 当前宿主 dsh 版本：system 档探测（跟随启动链语义）。
pub fn detect_current_version() -> Option<String> {
    let path = crate::resolve::effective_path();
    crate::resolve::detect_system_dsh(&path).map(|d| d.version)
}

fn component_update(
    current: Option<String>,
    latest: Option<String>,
    error: Option<String>,
) -> ComponentUpdate {
    let newer = match (&current, &latest) {
        (Some(c), Some(l)) => is_newer(c, l),
        _ => false,
    };
    ComponentUpdate {
        current,
        latest,
        newer,
        error,
    }
}

/// 一次完整检测（三维度）。网络失败不视为致命：对应维度 error 展示。
pub fn check_now(data_dir: &Path) -> UpdateStatus {
    let dsh = match fetch_latest_version() {
        Some(latest) => component_update(detect_current_version(), Some(latest), None),
        None => component_update(
            detect_current_version(),
            None,
            Some("registry 不可达或返回异常".to_string()),
        ),
    };
    let client_current = env!("CARGO_PKG_VERSION").to_string();
    let client = match APP_RELEASE_FEED {
        Some(_) => match fetch_client_latest() {
            Some(latest) => component_update(Some(client_current), Some(latest), None),
            None => component_update(Some(client_current), None, Some("更新源不可达".to_string())),
        },
        // feed 未配置：客户端维度只报当前版本，不算错误、不触网。
        None => component_update(Some(client_current), None, None),
    };
    UpdateStatus {
        dsh,
        client,
        node: node_runtime_info(data_dir),
    }
}

// ---------- node 私有缓存 ----------

/// node 缓存根：<data_dir>/tools/node/<version>/。
fn cached_node_dir(data_dir: &Path, version: &str) -> PathBuf {
    data_dir.join("tools").join("node").join(version)
}

/// 断点续传 .part 文件根（与版本缓存同层；跨进程存活，中断后可续）。
fn parts_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("tools").join("node")
}

/// 当前发行包的 .part 路径（文件名含版本与平台：版本升级后旧 .part 自然孤儿化）。
fn part_path(data_dir: &Path, version: &str, dist: &str) -> PathBuf {
    let extension = node_archive_extension(dist);
    parts_dir(data_dir).join(format!("node-{version}-{dist}.{extension}.part"))
}

/// 清理不属于当前版本/平台的孤儿 .part（上次升级遗留）。
fn clean_stale_parts(data_dir: &Path, keep: &Path) {
    let dir = parts_dir(data_dir);
    let Ok(entries) = fs::read_dir(&dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("part") && path != keep {
            let _ = fs::remove_file(&path);
        }
    }
}

/// 私有 Node prefix 下已安装的 dsh 包树（Unix/Windows npm root 布局均覆盖）。
fn cached_dsh_tree(data_dir: &Path, version: &str) -> Option<PathBuf> {
    let prefix = cached_node_dir(data_dir, version);
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
    let mut cmd = crate::child_cmd(node);
    cmd.arg(tree.join("lib/bin.js"))
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// 缓存 node 是否可用（存在 + 能报版本）。
pub fn cached_node_usable(data_dir: &Path, version: &str) -> Option<PathBuf> {
    let dir = cached_node_dir(data_dir, version);
    let bin = node_bin_in(&dir, version)?;
    if !bin.is_file() {
        return None;
    }
    let mut cmd = crate::child_cmd(&bin);
    let out = cmd.arg("--version").output().ok()?;
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
fn node_bin_in(dir: &Path, version: &str) -> Option<PathBuf> {
    node_bin_in_for(dir, node_dist_dir(), version)
}

/// 按指定发行版查找入口；拆出参数后可以在 Unix 单测 Windows zip 的目录布局。
fn node_bin_in_for(dir: &Path, dist: &str, version: &str) -> Option<PathBuf> {
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
    let prefix = format!("node-{version}-");
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

/// 解压 Node 官方发行包（从 .part 文件按路径读，下载全程不驻内存）。
/// Windows zip 条目必须经过路径约束，避免归档路径穿越。
fn extract_node_archive(archive_path: &Path, target: &Path, dist: &str) -> Result<()> {
    if dist.starts_with("win-") {
        let file = fs::File::open(archive_path).context("读取 node zip")?;
        let mut archive = zip::ZipArchive::new(file).context("读取 node zip")?;
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

    let gz =
        flate2::read::GzDecoder::new(fs::File::open(archive_path).context("读取 node tar.gz")?);
    let mut archive = tar::Archive::new(gz);
    archive.unpack(target).context("解压 node tar.gz")
}

// ---------- 断点续传（HTTP Range） ----------

/// 本地已有部分内容时的 Range 请求头；从零开始则不下头。
fn range_header(existing: u64) -> Option<String> {
    (existing > 0).then(|| format!("bytes={existing}-"))
}

/// 解析 `Content-Range: bytes <start>-<end>/<total>` → (start, total)。
fn parse_content_range(header: &str) -> Option<(u64, u64)> {
    let lowered = header.trim().to_ascii_lowercase();
    let rest = lowered.strip_prefix("bytes")?.trim();
    let (range, total) = rest.rsplit_once('/')?;
    let total = total.trim().parse::<u64>().ok()?;
    let (start, end) = range.split_once('-')?;
    let start = start.trim().parse::<u64>().ok()?;
    let end = end.trim().parse::<u64>().ok()?;
    (end >= start && total > start).then_some((start, total))
}

/// 断点续传分叉决策（纯函数，供测试）。
#[derive(Debug, PartialEq, Eq)]
enum ResumePlan {
    /// 服务器认可 Range（206）：本地已有 start 字节可续，全量总长 total。
    Append { start: u64, total: u64 },
    /// 服务器忽略 Range（200）：从零重写；总长可能未知。
    Restart { total: Option<u64> },
}

fn resume_plan(
    status: u16,
    content_range: Option<&str>,
    content_length: Option<&str>,
    existing: u64,
) -> Option<ResumePlan> {
    if status == 206 {
        let (start, total) = parse_content_range(content_range?)?;
        // 起点与本地长度不一致 = .part 与远端工件不匹配（版本漂移/损坏）→ 重写。
        if start == existing {
            Some(ResumePlan::Append { start, total })
        } else {
            Some(ResumePlan::Restart { total: Some(total) })
        }
    } else if status == 200 {
        Some(ResumePlan::Restart {
            total: content_length
                .map(str::trim)
                .and_then(|s| s.parse::<u64>().ok()),
        })
    } else {
        None
    }
}

/// 流式计算文件 SHA-256。续传的正确性不依赖镜像诚实性——最终整包哈希才是
/// 仲裁者，因此允许跨镜像续传（两个镜像分发的是同一官方工件）。
fn sha256_file(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).with_context(|| format!("打开 {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// 下载并解压官方 node 到私有缓存（macOS/Linux/Windows，在线兜底）。
/// .part 落盘 + HTTP Range 断点续传，中断后（跨进程、跨镜像）可续；进度逐块上抛。
pub fn download_node(data_dir: &Path, progress: DownloadProgress) -> Result<PathBuf> {
    if node_dist_dir() == "unsupported" {
        anyhow::bail!(
            "当前平台（{}-{}）暂无官方 node 下载支持",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
    }
    let dist = node_dist_dir();
    let plan = node_plan(data_dir);
    let expected_sha256 = plan
        .sha256_for(dist)
        .ok_or_else(|| anyhow::anyhow!("Node 映射缺 {dist} 校验和（来源 {}）", plan.source))?;
    let target = cached_node_dir(data_dir, &plan.version);
    let part = part_path(data_dir, &plan.version, dist);
    fs::create_dir_all(parts_dir(data_dir)).context("创建 node 缓存目录")?;
    clean_stale_parts(data_dir, &part);
    let urls = node_download_urls(dist, &plan.version);
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(NODE_CONNECT_TIMEOUT_SECS))
        .timeout_read(std::time::Duration::from_secs(NODE_READ_TIMEOUT_SECS))
        .build();
    let mut errors = Vec::new();
    for url in urls {
        // 解压目录每次尝试前清空（上一镜像可能解压了一半）；.part 保留供续传。
        fs::remove_dir_all(&target).ok();
        fs::create_dir_all(&target).context("创建 node 缓存目录")?;
        tracing::info!("下载 node {}（{url}）…", plan.version);
        let result = download_from_mirror(
            &agent,
            &url,
            &part,
            &target,
            dist,
            &plan.version,
            expected_sha256,
            progress,
        );
        match result {
            Ok(bin) => return Ok(bin),
            Err(e) => errors.push(format!("{url}: {e}")),
        }
    }
    fs::remove_dir_all(&target).ok();
    // 全部失败：清掉 .part——损坏的半成品（如 416 类不匹配）不能把后续重试永久卡死。
    let _ = fs::remove_file(&part);
    anyhow::bail!("Node 下载失败：{}", errors.join("；"))
}

/// 单镜像下载：Range 协商 → 追加/重写 .part → 整包 SHA-256 校验 → 解压 → 自检。
// 2026-08-27 裁定：8 个参数全为镜像链沿路透传的显式上下文（agent/url/路径/校验/进度），
// 聚合 struct 的收益低于引入新类型的噪声，允许超参（clippy::too_many_arguments）。
#[allow(clippy::too_many_arguments)]
fn download_from_mirror(
    agent: &ureq::Agent,
    url: &str,
    part: &Path,
    target: &Path,
    dist: &str,
    version: &str,
    expected_sha256: &str,
    progress: DownloadProgress,
) -> Result<PathBuf> {
    let existing = fs::metadata(part).map(|m| m.len()).unwrap_or(0);
    let mut request = agent.get(url);
    if let Some(range) = range_header(existing) {
        request = request.set("Range", &range);
    }
    let resp = request
        .call()
        .with_context(|| format!("下载 node 失败：{url}"))?;
    let plan = resume_plan(
        resp.status(),
        resp.header("Content-Range"),
        resp.header("Content-Length"),
        existing,
    )
    .ok_or_else(|| anyhow::anyhow!("镜像返回意外状态 {}", resp.status()))?;
    let (mut file, mut transferred, total) = match plan {
        ResumePlan::Append { start, total } => {
            let file = fs::OpenOptions::new()
                .append(true)
                .open(part)
                .context("续传打开 .part")?;
            tracing::info!("续传 node 包：本地 {start}/{total} 字节");
            (file, start, Some(total))
        }
        ResumePlan::Restart { total } => {
            let file = fs::File::create(part).context("重写 .part")?;
            (file, 0, total)
        }
    };
    let mut reader = resp.into_reader();
    let mut buf = vec![0u8; DOWNLOAD_CHUNK];
    loop {
        let n = reader.read(&mut buf).context("读取 node 包失败")?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).context("写入 .part 失败")?;
        transferred += n as u64;
        if transferred > NODE_ARCHIVE_MAX {
            anyhow::bail!("node 包超过 {NODE_ARCHIVE_MAX} 字节上限，已中止");
        }
        progress(transferred, total);
    }
    drop(file);
    // 整包校验：失败即弃 .part，交由外层换镜像从零重下（半截哈希对不上的是毒药）。
    let actual = sha256_file(part).context("校验 node 包失败")?;
    if actual != expected_sha256 {
        let _ = fs::remove_file(part);
        anyhow::bail!("Node 包 SHA-256 校验失败（期望 {expected_sha256}，实际 {actual}）");
    }
    extract_node_archive(part, target, dist).context("解压 node 包失败")?;
    let bin = node_bin_in(target, version)
        .ok_or_else(|| anyhow::anyhow!("解压后找不到 node 可执行文件"))?;
    let mut cmd = crate::child_cmd(&bin);
    let out = cmd
        .arg("--version")
        .output()
        .context("验证缓存 node")?;
    if !out.status.success() {
        anyhow::bail!("缓存 node 验证失败");
    }
    let _ = fs::remove_file(part);
    Ok(bin)
}

/// 取得可用执行器：系统 node 优先，否则下载缓存 node。
pub fn ensure_node(data_dir: &Path, progress: DownloadProgress) -> Result<PathBuf> {
    let path_env = resolve::effective_path();
    if let Some(sys) = resolve::detect_system_node(&path_env) {
        return Ok(sys.bin);
    }
    let version = node_plan(data_dir).version;
    if let Some(bin) = cached_node_usable(data_dir, &version) {
        return Ok(bin);
    }
    download_node(data_dir, progress)
}

/// 只有携带 npm-cli 的 Node 才能作为下载档执行器。
fn usable_node_with_npm(node_bin: PathBuf) -> Option<PathBuf> {
    find_npm_cli(&node_bin).map(|_| node_bin)
}

/// 当 pnpm 不可用时，准备一个可执行 npm-cli 的 Node。
fn ensure_node_with_npm(data_dir: &Path, progress: DownloadProgress) -> Result<PathBuf> {
    let path_env = resolve::effective_path();
    if let Some(sys) = resolve::detect_system_node(&path_env) {
        if let Some(bin) = usable_node_with_npm(sys.bin) {
            return Ok(bin);
        }
    }
    let version = node_plan(data_dir).version;
    if let Some(bin) = cached_node_usable(data_dir, &version) {
        if let Some(bin) = usable_node_with_npm(bin) {
            return Ok(bin);
        }
    }
    let bin = download_node(data_dir, progress)?;
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
    // GUI 子进程不加载 shell rc：`PNPM_HOME` 环境变量对 pnpm 10 无效（实测
    // 2026-08-25），global-bin-dir 只能是 undefined → `pnpm add -g` 报
    // ERR_PNPM_NO_GLOBAL_BIN_DIR 失败回退 npm。这里显式注入
    // `--config.global-bin-dir=<pnpm 目录>`（该目录天然在 PATH 里，满足 pnpm
    // 的校验），让 pnpm 路径直接可用。
    let bin_dir_args = pnpm_global_bin_dirs(pnpm_bin);
    for registry in package_registry_bases() {
        tracing::info!("pnpm add -g {spec}（registry={registry}）…");
        // pnpm 在 Windows 是 pnpm.cmd：child_cmd 负责 cmd /C 包装 + 无窗口启动。
        let mut command = crate::child_cmd(pnpm_bin);
        command.args(&bin_dir_args);
        command.args(pnpm_install_args(registry, &spec));
        let out = command
            .env("PATH", path_env)
            .output()
            .with_context(|| format!("执行 pnpm add -g {spec}"))?;
        if !out.status.success() {
            errors.push(format!("{registry}: {}", output_detail(&out)));
            continue;
        }
        let mut root_cmd = crate::child_cmd(pnpm_bin);
        let root_out = root_cmd
            .args(&bin_dir_args)
            .args(["root", "-g"])
            .env("PATH", path_env)
            .output()
            .context("解析 pnpm 全局根")?;
        if !root_out.status.success() {
            errors.push(format!(
                "pnpm root -g 失败：{}",
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

/// pnpm 的 global-bin-dir 注入参数。
/// 返回 `["--config.global-bin-dir=<父目录>"]`；父目录不可得时为空（回退 pnpm 默认）。
fn pnpm_global_bin_dirs(pnpm_bin: &Path) -> Vec<String> {
    match pnpm_bin.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => {
            vec![format!("--config.global-bin-dir={}", dir.display())]
        }
        _ => Vec::new(),
    }
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
        let mut command = crate::child_cmd(node_bin);
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
    let mut cmd = crate::child_cmd(node_bin);
    let Ok(output) = cmd.arg(npm_cli).arg("--version").output() else {
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
    let mut command = crate::child_cmd(node_bin);
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
    // 缓存形态：<node_dir>/bin/node → lib/node_modules/npm（父级结构差异）。
    // 系统 node（homebrew/fnm）不探测 `which npm` 兜底——执行器纪律：系统 npm
    // 可能引向别的 node，这里直接失败，由安装命令提示手动安装。
    [
        dir.join("../lib/node_modules/npm/bin/npm-cli.js"),
        dir.join("node_modules/npm/bin/npm-cli.js"),
        dir.join("npm-cli.js"),
    ]
    .into_iter()
    .find(|cand| cand.is_file())
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
pub fn install_latest_global(
    data_dir: &Path,
    progress: DownloadProgress,
) -> Result<(PathBuf, PathBuf)> {
    let node = ensure_node(data_dir, progress).context("准备 node 执行器失败")?;
    let plan_version = node_plan(data_dir).version;
    let private_prefix = cached_node_dir(data_dir, &plan_version);
    let node_is_private = node.starts_with(&private_prefix);
    let npm_prefix = node_is_private.then_some(private_prefix.as_path());
    let latest = fetch_latest_version()
        .ok_or_else(|| anyhow::anyhow!("无法获取官方版本列表（registry 不可达或返回异常）"))?;
    tracing::info!("下载档将全局安装 dsh {latest}");
    if node_is_private {
        if let Some(tree) = cached_dsh_tree(data_dir, &plan_version) {
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
                ensure_node_with_npm(data_dir, progress).context("准备 npm 执行器失败")?
            } else {
                download_node(data_dir, progress).context("准备私有 npm 执行器失败")?
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
    use std::io::Cursor;

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

        let node_urls = node_download_urls("darwin-arm64", NODE_VERSION);
        assert!(node_urls[0].starts_with("https://cdn.npmmirror.com/binaries/node/"));
        assert!(node_urls[1].starts_with("https://nodejs.org/dist/"));
        assert!(node_urls[0].ends_with(".tar.gz"));

        let windows_urls = node_download_urls("win-x64", NODE_VERSION);
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
    fn range_header_only_sent_for_partial_files() {
        assert_eq!(range_header(0), None);
        assert_eq!(range_header(1024).as_deref(), Some("bytes=1024-"));
    }

    #[test]
    fn content_range_parses_official_form() {
        assert_eq!(parse_content_range("bytes 100-199/1234"), Some((100, 1234)));
        // 大小写与空白容忍；畸形输入一律 None。
        assert_eq!(parse_content_range("Bytes 0-49/50"), Some((0, 50)));
        assert_eq!(parse_content_range("bytes 199-100/1234"), None);
        assert_eq!(parse_content_range("bytes 100-199/*"), None);
        assert_eq!(parse_content_range("garbage"), None);
    }

    #[test]
    fn resume_plan_forks_on_206_and_200() {
        // 206 + 起点匹配 → 追加续传。
        assert_eq!(
            resume_plan(206, Some("bytes 100-199/1234"), Some("100"), 100),
            Some(ResumePlan::Append {
                start: 100,
                total: 1234
            })
        );
        // 206 但起点不匹配（.part 与远端工件不一致）→ 重写。
        assert_eq!(
            resume_plan(206, Some("bytes 50-199/1234"), None, 100),
            Some(ResumePlan::Restart { total: Some(1234) })
        );
        // 200（服务器忽略 Range）→ 从零重写，总长尽力取 Content-Length。
        assert_eq!(
            resume_plan(200, None, Some("1234"), 100),
            Some(ResumePlan::Restart { total: Some(1234) })
        );
        assert_eq!(
            resume_plan(200, None, None, 0),
            Some(ResumePlan::Restart { total: None })
        );
        // 416 等其他状态 → 该镜像失败（外层换镜像/清 .part）。
        assert_eq!(resume_plan(416, None, None, 100), None);
    }

    #[test]
    fn decode_hex_accepts_pairs_only() {
        assert_eq!(decode_hex("0f10"), Some(vec![15, 16]));
        assert_eq!(decode_hex("0F10"), Some(vec![15, 16]));
        assert_eq!(decode_hex("0f1"), None);
        assert_eq!(decode_hex("zz"), None);
        assert_eq!(decode_hex(""), None);
    }

    #[test]
    fn builtin_plan_covers_all_six_dists() {
        let plan = builtin_node_plan();
        assert_eq!(plan.source, "builtin");
        assert_eq!(plan.version, NODE_VERSION);
        for dist in [
            "darwin-arm64",
            "darwin-x64",
            "linux-arm64",
            "linux-x64",
            "win-arm64",
            "win-x64",
        ] {
            assert!(plan.sha256_for(dist).is_some(), "builtin 缺 {dist}");
        }
    }

    fn six_dist_map(node_version: &str, min_shell: &str, sha_len: usize) -> String {
        let sha = "a".repeat(sha_len);
        let artifacts: std::collections::BTreeMap<String, serde_json::Value> = [
            "darwin-arm64",
            "darwin-x64",
            "linux-arm64",
            "linux-x64",
            "win-arm64",
            "win-x64",
        ]
        .iter()
        .map(|dist| (dist.to_string(), serde_json::json!({ "sha256": sha })))
        .collect();
        serde_json::json!({
            "format": 1,
            "nodeVersion": node_version,
            "minShellVersion": min_shell,
            "artifacts": artifacts,
        })
        .to_string()
    }

    #[test]
    fn node_map_parses_when_complete() {
        let map = six_dist_map("v25.0.0", "0.1.0", 64);
        let plan = parse_node_plan(map.as_bytes()).unwrap();
        assert_eq!(plan.version, "v25.0.0");
        assert_eq!(plan.source, "remote");
        assert!(plan.sha256_for("win-x64").is_some());
    }

    #[test]
    fn node_map_rejects_incomplete_or_malformed() {
        // sha 长度不对
        assert!(parse_node_plan(six_dist_map("v25.0.0", "0.1.0", 32).as_bytes()).is_none());
        // format 不认识
        let bad_format =
            six_dist_map("v25.0.0", "0.1.0", 64).replace("\"format\":1", "\"format\":2");
        assert!(parse_node_plan(bad_format.as_bytes()).is_none());
        // minShellVersion 高于当前壳 → 拒用
        assert!(parse_node_plan(six_dist_map("v25.0.0", "99.0.0", 64).as_bytes()).is_none());
        // 版本形态不对
        assert!(parse_node_plan(six_dist_map("25.0.0", "0.1.0", 64).as_bytes()).is_none());
        // 缺一个平台 → 宁可回退内置（重命名一个 dist 使六平台不齐）
        let missing = six_dist_map("v25.0.0", "0.1.0", 64).replace("win-arm64", "not-a-dist");
        assert!(parse_node_plan(missing.as_bytes()).is_none());
    }

    #[test]
    fn signature_verify_happy_forged_and_wrong_key() {
        use ed25519_dalek::{Signer, SigningKey};
        fn hex(bytes: &[u8]) -> String {
            bytes.iter().map(|b| format!("{b:02x}")).collect()
        }
        let key = SigningKey::from_bytes(&[9u8; 32]);
        let pubkey_hex = hex(&key.verifying_key().to_bytes());
        let msg = b"hello map";
        let sig_hex = hex(&key.sign(msg).to_bytes());
        assert!(verify_signature_with(&pubkey_hex, msg, &sig_hex));
        // 篡改消息一字节 → 验签失败
        assert!(!verify_signature_with(&pubkey_hex, b"hello nap", &sig_hex));
        // 换公钥（他人签的）→ 验签失败
        let other = SigningKey::from_bytes(&[8u8; 32]);
        assert!(!verify_signature_with(
            &hex(&other.verifying_key().to_bytes()),
            msg,
            &sig_hex
        ));
        // 畸形签名文本
        assert!(!verify_signature_with(&pubkey_hex, msg, "not-hex"));
    }

    #[test]
    fn node_map_tarball_extraction_finds_files() {
        let mut tgz = Vec::new();
        {
            let enc = flate2::write::GzEncoder::new(&mut tgz, flate2::Compression::default());
            let mut builder = tar::Builder::new(enc);
            let mut put = |path: &str, data: &[u8]| {
                let mut header = tar::Header::new_gnu();
                header.set_size(data.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                builder
                    .append_data(&mut header, path, Cursor::new(data.to_vec()))
                    .unwrap();
            };
            put("package/map.json", b"{\"format\":1}");
            put("package/map.json.sig", b"deadbeef\n");
            put("package/package.json", b"{}");
            builder.into_inner().unwrap().finish().unwrap();
        }
        let (map, sig) = extract_node_map_files(&tgz).unwrap();
        assert_eq!(map, b"{\"format\":1}".to_vec());
        assert_eq!(sig.trim(), "deadbeef");
        // 缺 sig 文件 → None（宁缺毋滥）
        let mut lone = Vec::new();
        {
            let enc = flate2::write::GzEncoder::new(&mut lone, flate2::Compression::default());
            let mut builder = tar::Builder::new(enc);
            let mut header = tar::Header::new_gnu();
            header.set_size(2);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, "package/map.json", Cursor::new(b"{}".to_vec()))
                .unwrap();
            builder.into_inner().unwrap().finish().unwrap();
        }
        assert!(extract_node_map_files(&lone).is_none());
    }

    #[test]
    fn stale_parts_are_cleaned_but_current_kept() {
        let data = tmp("parts-clean");
        std::fs::create_dir_all(parts_dir(&data)).unwrap();
        let keep = part_path(&data, NODE_VERSION, "darwin-arm64");
        let stale_version = part_path(&data, "v99.0.0", "darwin-arm64");
        let stale_dist = part_path(&data, NODE_VERSION, "win-x64");
        let unrelated = parts_dir(&data).join("node.tar.gz");
        for path in [&keep, &stale_version, &stale_dist, &unrelated] {
            std::fs::write(path, b"x").unwrap();
        }
        clean_stale_parts(&data, &keep);
        assert!(keep.is_file(), "当前 .part 必须保留");
        assert!(!stale_version.exists(), "旧版本 .part 应清理");
        assert!(!stale_dist.exists(), "其他平台 .part 应清理");
        assert!(unrelated.is_file(), "非 .part 文件不动");
        std::fs::remove_dir_all(&data).ok();
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
        assert_eq!(node_bin_in(&dir, NODE_VERSION), Some(bin));
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
        assert_eq!(cached_dsh_tree(&data, NODE_VERSION), Some(tree.clone()));
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
        assert_eq!(node_bin_in_for(&dir, "win-x64", NODE_VERSION), Some(bin));
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
        // .part 落盘形态：解压入口按路径读文件。
        let part = target.join("node.part");
        std::fs::write(&part, bytes.get_ref()).unwrap();

        extract_node_archive(&part, &target, "win-x64").unwrap();
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
    fn pnpm_global_bin_dirs_injects_directory_on_path() {
        // 有父目录 → 注入 --config.global-bin-dir=<父目录>（unix 风格路径）
        let args = pnpm_global_bin_dirs(Path::new("/usr/local/bin/pnpm"));
        assert_eq!(
            args,
            vec!["--config.global-bin-dir=/usr/local/bin".to_string()]
        );
        // 裸文件名（无父目录）→ 不注入，回退 pnpm 默认
        assert_eq!(pnpm_global_bin_dirs(Path::new("pnpm")), Vec::<String>::new());
    }

    #[cfg(windows)]
    #[test]
    fn pnpm_global_bin_dirs_injects_windows_dir() {
        // Windows 反斜杠路径同样生效（pnpm.cmd 场景）
        let win = pnpm_global_bin_dirs(Path::new(r"C:\Users\me\AppData\Roaming\npm\pnpm.cmd"));
        assert_eq!(
            win,
            vec![
                "--config.global-bin-dir=C:\\Users\\me\\AppData\\Roaming\\npm".to_string()
            ]
        );
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
        // 脚本需跳过注入的 --config.global-bin-dir=... 前置参数（首个参数是它）。
        let script = r#"#!/bin/sh
set -eu
root="$(dirname "$0")/global/node_modules"
# 前置参数列表：--config.global-bin-dir=<dir> 后才是子命令（root / add）
if [ "${1#--config.global-bin-dir=*}" != "$1" ]; then
  shift  # 吃掉注入的 global-bin-dir
fi
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
    fn release_tag_parses_github_latest() {
        assert_eq!(
            parse_release_tag(r#"{"tag_name": "v0.2.0", "name": "rel"}"#).as_deref(),
            Some("0.2.0")
        );
        assert_eq!(parse_release_tag(r#"{"message": "Not Found"}"#), None);
        assert_eq!(parse_release_tag("not json"), None);
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
