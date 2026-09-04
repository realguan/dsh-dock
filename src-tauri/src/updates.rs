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
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::resolve;

/// 下载档使用的官方 node 版本（与兜底副本对齐；LTS）。
const NODE_VERSION: &str = "v24.18.0";
/// 元数据请求整体超时（秒）：registry 拉包清单等小响应，整体限时合理。
const NET_TIMEOUT_SECS: u64 = 60;
/// 下载进度回调：`(已传输字节, 总字节)`；服务器未报长度时总字节为 None。
/// updates 模块保持零 tauri 依赖——进度经回调上抛，由 lib.rs 桥接为事件。
pub type DownloadProgress<'a> = &'a mut dyn FnMut(u64, Option<u64>);
/// pnpm v10 默认会阻止依赖的 install/postinstall；dsh 的 native/helper 依赖必须放行。
const PNPM_BUILD_PACKAGES: [&str; 5] = [
    "@deepseek-ai/dsh-subprocess-local",
    "@google/genai",
    "koffi",
    "node-pty",
    "protobufjs",
];

/// 包管理器使用的 registry 顺序：国内镜像优先，官方源兜底。
pub(crate) fn package_registry_bases() -> [&'static str; 2] {
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
    if s.is_empty() || !s.len().is_multiple_of(2) {
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

/// 引擎三件探测（版本维度共用一次 spawn 开销）。
fn engine_status(data_dir: &Path) -> crate::engines::EngineStatus {
    crate::engines::probe_engine(data_dir, &resolve::effective_path())
}

/// Node 运行时维度：引擎优先（ADR-0010）→ 系统探测（退役前过渡）→ 托管计划。
fn node_runtime_info(data_dir: &Path) -> Option<NodeRuntimeInfo> {
    if let Some(v) = engine_status(data_dir).node {
        return Some(NodeRuntimeInfo {
            version: v,
            origin: "engine",
        });
    }
    Some(NodeRuntimeInfo {
        version: node_plan(data_dir).version,
        origin: "managed",
    })
}

/// 当前宿主 dsh 版本：引擎优先（ADR-0010），引擎未就绪回退系统探测
///（探测层退役前的过渡口径；两者都缺 = None，前端展示「未检出」）。
pub fn detect_current_version(data_dir: &Path) -> Option<String> {
    engine_status(data_dir).dsh
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
    let engine_dsh = engine_status(data_dir).dsh;
    let dsh = match fetch_latest_version() {
        Some(latest) => component_update(Some(engine_dsh).flatten().or(None), Some(latest), None),
        None => component_update(
            Some(engine_dsh).flatten().or(None),
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

/// 升级引擎内 dsh 到最新稳定版（ADR-0010：升级全显式 + 引擎私有——不碰
/// 用户全局安装）。返回实际写入的 dsh 版本。引擎 pnpm 缺位（boot 未跑成/
/// 目录被清）先从捆绑包重铺；dsh 版本比对排除预发布（latest_stable）。
pub fn upgrade_engine_dsh(data_dir: &Path, resources_dir: &Path, path_env: &str) -> Result<String> {
    let version = latest_stable_dsh_version()?;
    tracing::info!(
        data_dir = %data_dir.display(),
        target = %version,
        "dsh 升级开始（引擎内 add -g，不触用户全局）"
    );
    if !crate::engines::engine_pnpm_bin(data_dir).exists() {
        tracing::info!("引擎 pnpm 缺位，先从捆绑包重铺");
        crate::engines::stage_pnpm_from_bundle(&engine_pnpm_bundle(resources_dir), data_dir)
            .context("引擎 pnpm 重铺失败")?;
    }
    crate::engines::install_dsh_global(data_dir, &version, path_env)?;
    tracing::info!(version = %version, "dsh 升级完成");
    Ok(version)
}

// ---------- dsh 全局安装（pnpm 引擎内通道） ----------

/// pnpm 10 只识别 `--allow-build=<package>` 形式，必须显式拼在同一个参数中。
pub(crate) fn pnpm_install_args(registry: &str, spec: &str) -> Vec<String> {
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
// ---------- 引擎引导入口（AGENTS §7「引擎引导」用途：updates 编排 engines 子进程网络） ----------

/// 打包期随壳内置的 pnpm 压缩包位置（@pnpm/exe.<platform> tgz，边界 A 裁定：
/// 安装包内压缩存储）。命名契约 = resources/pnpm/<平台>.tgz，装配方落位（P3-f）。
#[allow(dead_code)] // P3-b boot 接线启用
pub fn engine_pnpm_bundle(resources_dir: &Path) -> PathBuf {
    let platform = if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "darwin-arm64"
        } else {
            "darwin-x64"
        }
    } else if cfg!(target_os = "windows") {
        if cfg!(target_arch = "aarch64") {
            "win32-arm64"
        } else {
            "win32-x64"
        }
    } else if cfg!(target_arch = "aarch64") {
        "linux-arm64"
    } else {
        "linux-x64"
    };
    resources_dir.join("pnpm").join(format!("{platform}.tgz"))
}

/// dsh 引导目标版本：排序最高**稳定版**（排除预发布——ADR-0010 台账
/// 0.1.2-alpha.2 事故预防；与更新检测「rc 也追」的 H-1 口径分离）。
pub fn latest_stable_dsh_version() -> Result<String> {
    let packument =
        fetch_packument().context("无法获取官方版本列表（registry 不可达或返回异常）")?;
    parse_versions(&packument)
        .into_iter()
        .find(|v| !v.contains('-'))
        .ok_or_else(|| anyhow::anyhow!("官方版本列表无稳定版"))
}

/// 引擎引导唯一入口（AGENTS §7「引擎引导」，boot 接线 = resolve_launch 引擎档）。
/// node 期望版本取 node-map（fail-closed 有内置基线与本地缓存）；dsh 目标版本
/// 惰性解析（dist-tags 查询）——仅 dsh 真缺件时才触网，就绪引擎离线 boot 零
/// 网络（contract v3 在线语义）。
pub fn ensure_engine_bootstrapped(
    data_dir: &Path,
    resources_dir: &Path,
    path_env: &str,
    progress: DownloadProgress<'_>,
) -> Result<crate::engines::BootstrapOutcome> {
    crate::engines::bootstrap(
        data_dir,
        path_env,
        &engine_pnpm_bundle(resources_dir),
        &mut || Ok(node_plan(data_dir).version),
        &mut latest_stable_dsh_version,
        progress,
    )
}
// ---------- 插件更新检查（4.4④）：registry packument 通用查询 ----------
//
// §7 登记的外网用途（2026-08-29）：与 dsh 版本检查同链（镜像链 npmmirror →
// npmjs、同超时、同 packument 体积上限），网络代码只住本模块——plugins.rs
// 经本函数取版本数据，自身不触网。

/// 解析 packument → (dist-tags.latest, 全版本升序)。纯函数，Vitest/Cargo 直测。
fn parse_packument_versions(text: &str) -> Option<(String, Vec<String>)> {
    let v = serde_json::from_str::<serde_json::Value>(text).ok()?;
    let latest = v.get("dist-tags")?.get("latest")?.as_str()?.to_string();
    let mut versions: Vec<String> = v.get("versions")?.as_object()?.keys().cloned().collect();
    versions.sort_by(|a, b| crate::resolve::compare_versions_asc(a, b));
    Some((latest, versions))
}

/// 查询 npm 包的 (latest, 全版本升序)：镜像链顺序尝试，全部失败才 Err。
/// 包名须先过 `plugins::validate_plugin_spec`（调用方把关），此处只做
/// URL 安全拼装（scoped `/` → `%2F`，与 npm CLI 一致）。
pub fn npm_packument_versions(package: &str) -> Result<(String, Vec<String>), String> {
    if package.is_empty() || package.starts_with('-') {
        return Err("包名非法".to_string());
    }
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(NET_TIMEOUT_SECS))
        .build();
    let mut last_err = String::from("镜像链均不可达");
    for base in package_registry_bases() {
        let url = format!("{base}/{}", package.replace('/', "%2F"));
        let resp = match agent.get(&url).call() {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("{base}：{e}");
                continue;
            }
        };
        let text = match read_body_capped(resp.into_reader(), PACKUMENT_MAX_BYTES) {
            Ok(t) => t,
            Err(e) => {
                last_err = format!("{base}：读取失败 {e}");
                continue;
            }
        };
        match parse_packument_versions(&text) {
            Some(pair) => return Ok(pair),
            None => {
                last_err = format!("{base}：packument 形状不符");
            }
        }
    }
    Err(last_err)
}

// ---------- 社区插件市场 Registry 拉取 (dsh-market / awesome-dsh-plugin) ----------

/// 市场 Registry CDN 列表（镜像链，与 packument 同模式）。
const MARKET_REGISTRY_URLS: &[&str] = &[
    "https://awesome-dsh-plugin.com/plugins.json",
    "https://raw.githubusercontent.com/awesome-dsh-plugin/awesome-dsh-plugin/main/plugins.json",
];

/// Registry 最大体积上限（当前 ~650KB，预留到 3MB）。
const MARKET_REGISTRY_MAX_BYTES: u64 = 3 * 1024 * 1024;

/// 拉取社区插件市场目录 JSON（原样透传给前端解析）。
pub fn fetch_market_registry() -> Result<String, String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(NET_TIMEOUT_SECS))
        .build();
    let mut last_err = String::from("市场 Registry 均不可达");
    for url in MARKET_REGISTRY_URLS {
        tracing::info!("读取插件市场 Registry: {url}");
        let resp = match agent.get(url).call() {
            Ok(r) => r,
            Err(e) => {
                last_err = format!("{url}: {e}");
                continue;
            }
        };
        match read_body_capped(resp.into_reader(), MARKET_REGISTRY_MAX_BYTES) {
            Ok(text) => return Ok(text),
            Err(e) => {
                last_err = format!("{url}: {e}");
                continue;
            }
        }
    }
    Err(last_err)
}

#[cfg(test)]
mod packument_tests {
    use super::*;

    #[test]
    fn parses_packument_latest_and_sorted_versions() {
        let text = r#"{
            "dist-tags": {"latest": "0.16.1", "next": "0.17.0-rc.1"},
            "versions": {
                "0.16.1": {"dist": {}},
                "0.9.0": {"dist": {}},
                "0.17.0-rc.1": {"dist": {}},
                "0.10.0": {"dist": {}}
            }
        }"#;
        let (latest, versions) = parse_packument_versions(text).unwrap();
        assert_eq!(latest, "0.16.1");
        // 升序按 semver（0.9 < 0.10 < 0.16 < 0.17-rc），非字典序
        assert_eq!(versions, vec!["0.9.0", "0.10.0", "0.16.1", "0.17.0-rc.1"]);
    }

    #[test]
    fn malformed_packument_is_none() {
        assert!(parse_packument_versions("not json").is_none());
        assert!(parse_packument_versions(r#"{"versions":{}}"#).is_none());
    }
}
