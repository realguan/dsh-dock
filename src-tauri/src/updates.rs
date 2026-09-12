//! updates.rs —— 宿主 dsh 版本管理 + download 档实装（docs/contract.md「运行时策略」）。
//!
//! 壳是终端的**唯一网络面**（纪律：本模块之外不得触网）：
//!   - 版本获取：npm registry packument（镜像链 npmmirror → npmjs）。升级口径 =
//!     排序最高**可接受**版本（稳定/rc；2026-09-09 裁定，检测与升级同口径——
//!     此前检测追排序最高含 alpha，出现「有新版但升级装不上」的假角标）；
//!     alpha 等预览版只进版本列表（`list_dsh_versions`），经用户显式选择安装。
//!   - node 兜底：用户无 node 时优先从 npmmirror、再从 nodejs.org 下载到**私有缓存**
//!     （不替用户全局装 node，Q2b 推论 5），充当执行器。
//!   - dsh 安装（ADR-0017，2026-09-11 修订）：引擎内 **project 内安装**（非全局，
//!     Windows 普通账户免符号链接特权），经引擎内置 pnpm 按 npmmirror → npmjs 顺序
//!     尝试。（原「全局安装」与「npm-cli 兜底」两条描述均已退役——后者随探测层退役，
//!     见 AGENTS §6。）
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
/// 下载进度阶段（ui `boot:progress` 的 kind 字段）：Node = `runtime set`
/// 的下载字节；Dsh = dsh 安装（ADR-0017：project 内 `add`）的包计数（downloaded / resolved）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressStage {
    Node,
    Dsh,
}

impl ProgressStage {
    pub fn as_str(self) -> &'static str {
        match self {
            ProgressStage::Node => "node",
            ProgressStage::Dsh => "dsh",
        }
    }
}

/// 下载进度回调：`(阶段, 已完成量, 总量)`；总量未知时为 None。
/// 阶段决定量的单位——Node 为字节，Dsh 为包数。updates 模块保持零 tauri
/// 依赖——进度经回调上抛，由 lib.rs 桥接为事件。
pub type DownloadProgress<'a> = &'a mut dyn FnMut(ProgressStage, u64, Option<u64>);
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

/// registry 链消费口（2026-09-07）：`DSH_DOCK_NPM_REGISTRIES`（逗号分隔）覆盖
/// 顺序——默认链（npmmirror → 官方）面向最终用户网络；境外 CI 冒烟置官方源
/// 优先。仅调整 §7 已登记用途内的源顺序，不新增网络面。
pub(crate) fn registry_chain() -> Vec<String> {
    parse_chain(
        &std::env::var("DSH_DOCK_NPM_REGISTRIES").unwrap_or_default(),
        &package_registry_bases(),
    )
}

/// 纯函数：逗号分隔解析（去空白、滤空项）；空输入回退 defaults。
pub(crate) fn parse_chain(spec: &str, defaults: &[&str]) -> Vec<String> {
    let parsed: Vec<String> = spec
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    if parsed.is_empty() {
        defaults.iter().map(|s| s.to_string()).collect()
    } else {
        parsed
    }
}

/// 随壳捆绑的 pnpm 版本（ADR-0010：resources/pnpm/<platform>.tgz 经
/// scripts/fetch-pnpm-bundle.sh 打包期取得，该脚本从本常量推导版本，防两处
/// 漂移）。升级必过 ADR-0010 升级清单（runtime set / dsh project 内安装 / spawnSync
/// 可达 / 三平台 boot 冒烟）。
/// Rust 侧无运行时消费（bundle 落地只解包不关版本）——唯一消费者是打包期
/// 脚本，dead_code 豁免即此契约。
#[allow(dead_code)]
pub const PINNED_PNPM_VERSION: &str = "12.3.1";

fn npm_registry_urls() -> Vec<String> {
    registry_chain()
        .iter()
        .map(|base| format!("{base}/@deepseek-ai%2Fdsh"))
        .collect()
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

/// 展示用版本号：剥掉可选的 `v` 前缀（**不要**用于传给 pnpm 的实参）。
///
/// **2026-09-10 修复（v1.1.0 Windows 实测 1.1）**：node-map 与内置基线里的版本是
/// **带 `v` 的**（`v24.18.0`，与 nodejs.org 发布标签一致），而展示处又拼了一个
/// `v` → 用户看到 `node vv24.18.0`（WSL 引导提示）。
///
/// 归一逻辑与 `engines::readiness_gaps` 同一口径（它一直会剥 `v` 再比较，说明
/// 只有展示层忘了归一）。**喂 pnpm 的实参保持原样**——实测 `runtime set node
/// v24.18.0` 是接受的，无需为显示问题去改调用契约。
pub fn display_version(v: &str) -> &str {
    v.strip_prefix('v').unwrap_or(v)
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
    fetch_node_map_with(&registry_chain(), &UreqGet::new())
}

/// `fetch_node_map` 的可注入实现（离线覆盖镜像链回退 / 坏响应跳过）。
/// `bases` = registry 基址链（生产传 `registry_chain()`，测试传固定链）。
fn fetch_node_map_with(bases: &[String], http: &dyn HttpGet) -> Option<(Vec<u8>, String)> {
    for base in bases {
        // scoped 包在 registry URL 里必须把 `/` 编码为 %2F（与 npm CLI 行为一致）。
        let packument_url = format!("{base}/{}", NODE_MAP_PACKAGE.replace('/', "%2F"));
        let Ok(text) = http.get_text(&packument_url, PACKUMENT_MAX_BYTES, None) else {
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
        let Ok(tgz) = http.get_bytes(tarball, NODE_MAP_MAX_BYTES) else {
            continue;
        };
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

// ---------- HTTP seam（2026-09-08 架构评审批次 5，P6） ----------
//
// 本模块是壳的唯一网络面，但「镜像链回退 / 坏响应跳过 / 全失败收敛」这些**编排逻辑**
// 过去只能靠真网络或人工验证。这里把「取一个 URL」抽成 trait：生产实现走 ureq，
// 测试注入假体 → 离线覆盖编排分支（不触网、不依赖 mock server）。

/// 单次 HTTP GET 的最小面。实现负责超时与体积上限。
pub(crate) trait HttpGet {
    /// 取响应体文本（超过 `cap` 视为失败）。
    fn get_text(&self, url: &str, cap: u64, user_agent: Option<&str>) -> Result<String>;
    /// 取响应体字节（tarball 等二进制）。
    fn get_bytes(&self, url: &str, cap: u64) -> Result<Vec<u8>>;
}

/// 生产实现：ureq 阻塞客户端（唯一网络面的唯一出口）。
struct UreqGet {
    agent: ureq::Agent,
}

impl UreqGet {
    fn new() -> Self {
        Self {
            agent: ureq::AgentBuilder::new()
                .timeout(std::time::Duration::from_secs(NET_TIMEOUT_SECS))
                .build(),
        }
    }
}

impl HttpGet for UreqGet {
    fn get_text(&self, url: &str, cap: u64, user_agent: Option<&str>) -> Result<String> {
        let mut req = self.agent.get(url);
        if let Some(ua) = user_agent {
            req = req.set("User-Agent", ua);
        }
        let resp = req.call().with_context(|| format!("请求 {url} 失败"))?;
        read_body_capped(resp.into_reader(), cap).with_context(|| format!("{url} 响应体读取失败"))
    }

    fn get_bytes(&self, url: &str, cap: u64) -> Result<Vec<u8>> {
        let resp = self
            .agent
            .get(url)
            .call()
            .with_context(|| format!("请求 {url} 失败"))?;
        let mut buf = Vec::new();
        resp.into_reader()
            .take(cap + 1)
            .read_to_end(&mut buf)
            .with_context(|| format!("{url} 响应体读取失败"))?;
        if buf.len() as u64 > cap {
            anyhow::bail!("{url} 响应体超过 {cap} 字节上限");
        }
        Ok(buf)
    }
}

/// 拉取 packument（镜像链逐个尝试，首个成功即返回）。
fn fetch_packument() -> Result<serde_json::Value> {
    fetch_packument_with(&npm_registry_urls(), &UreqGet::new())
}

/// `fetch_packument` 的可注入实现（离线覆盖镜像链回退与坏响应跳过）。
/// `urls` = 完整 packument URL 链（生产传 `npm_registry_urls()`，测试传固定链）。
fn fetch_packument_with(urls: &[String], http: &dyn HttpGet) -> Result<serde_json::Value> {
    let mut last_err: Option<anyhow::Error> = None;
    for url in urls {
        tracing::info!("读取 dsh 版本列表：{url}");
        match http.get_text(url, PACKUMENT_MAX_BYTES, None) {
            Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
                Ok(v) => return Ok(v),
                Err(e) => last_err = Some(e.into()),
            },
            Err(e) => last_err = Some(e),
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

/// 单次 packument 的 dsh 版本口径拆解（纯函数，供测试）：返回
/// `(可升级口径最高版, 预览口径最高版)`。可升级 = 排序最高**可接受**版本
/// （稳定/rc，`is_acceptable_dsh_version`）；预览 = 排序最高版本若不可接受
/// （alpha 等）则为其，与可升级口径同版时为 None。
pub fn split_dsh_versions(packument: &serde_json::Value) -> (Option<String>, Option<String>) {
    let vs = parse_versions(packument);
    let preview = vs.first().cloned();
    let upgradable = vs.into_iter().find(|v| is_acceptable_dsh_version(v));
    let preview_latest = match (&preview, &upgradable) {
        (Some(p), Some(u)) if p == u => None,
        (Some(_), _) => preview,
        _ => None,
    };
    (upgradable, preview_latest)
}

// ---------- dsh 版本列表（版本选择器，2026-09-09） ----------

/// 版本列表条目（`list_dsh_versions`；`relation` 由后端按 semver 比较——
/// 前端不做版本比较，避免两套比较器漂移）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct DshVersionEntry {
    pub version: String,
    /// stable / rc / alpha / other（`DshVersionChannel::as_str`）。
    pub channel: &'static str,
    /// 与当前已装版本的相对关系；当前版本未检出时一律 newer（无法判定新旧，
    /// 重复安装同版无害——pnpm 幂等重装）。
    pub relation: &'static str,
    /// 发布时间（packument `time` 原文 RFC3339；registry 未记录为 None）。
    pub published_at: Option<String>,
}

/// 版本列表响应：当前已装版本 + 降序条目。
#[derive(Debug, Clone, serde::Serialize)]
pub struct DshVersionsResult {
    pub current: Option<String>,
    pub versions: Vec<DshVersionEntry>,
}

/// packument → 版本列表条目（纯函数，供测试）。只读 `time` 中与版本键同名的
/// 条目（`created` / `modified` 等元键不进列表）。
pub fn parse_dsh_version_entries(
    packument: &serde_json::Value,
    current: Option<&str>,
) -> Vec<DshVersionEntry> {
    parse_versions(packument)
        .into_iter()
        .map(|version| {
            let relation = match current {
                Some(c) => match resolve::compare_versions_asc(&version, c) {
                    std::cmp::Ordering::Greater => "newer",
                    std::cmp::Ordering::Equal => "current",
                    std::cmp::Ordering::Less => "older",
                },
                None => "newer",
            };
            let published_at = packument
                .get("time")
                .and_then(|t| t.get(&version))
                .and_then(|t| t.as_str())
                .map(String::from);
            DshVersionEntry {
                channel: dsh_version_channel(&version).as_str(),
                version,
                relation,
                published_at,
            }
        })
        .collect()
}

/// dsh 版本列表（版本选择器数据源）：一次 packument 拉取，降序 + 通道 +
/// 发布时间 + 与已装版本的相对关系。网络失败返回 Err（前端列表内展示重试）。
pub fn list_dsh_versions(current: Option<&str>) -> Result<DshVersionsResult, String> {
    let packument = fetch_packument().map_err(|e| format!("无法获取官方版本列表：{e:#}"))?;
    Ok(DshVersionsResult {
        current: current.map(String::from),
        versions: parse_dsh_version_entries(&packument, current),
    })
}

// ---------- 版本状态（更新检测） ----------

/// 单个可升级组件的版本维度（dsh 本体 / 桌面客户端）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ComponentUpdate {
    pub current: Option<String>,
    pub latest: Option<String>,
    pub newer: bool,
    pub error: Option<String>,
    /// 排序最高但**不在升级口径内**的预览版（alpha 等）；无或与 `latest` 同版为
    /// None。2026-09-09：检测与升级口径对齐的配套字段——「有新版/升级」只按
    /// 可接受口径（稳定/rc）判定，预览版经版本列表显式选择，不再伪装成新版。
    /// 仅 dsh 维度使用；client 维度恒为 None。
    pub preview_latest: Option<String>,
}

/// Node 运行时维度（只读信息，无升级动作——版本由下载计划决定）。
///
/// **2026-09-10 修复（v1.1.0 Windows 实测 1.2 的"说谎"项）**：原实现把**下载计划
/// 版本**当成已装版本返回（`version` 恒为 `String`），于是引擎 node 其实**没装**
/// 时，关于页照样显示「v24.18.0 · 应用托管 · 随启动自动准备」——与健康大盘（真探测
/// `engines/bin`：未检出）自相矛盾。用户看到的那个版本号从来没被安装过。
/// 现拆成两个字段：`version` = **实测**（未装 = None），`plannedVersion` = 计划
/// （仅在未装时用于提示"将要装哪个"）。**判据是"装没装"，不是"打算装什么"。**
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeRuntimeInfo {
    /// 实测版本（引擎 `engines/bin/node --version`）。**未安装 = None**。
    pub version: Option<String>,
    /// engine = 壳引擎资产；managed = 应用托管（计划下载，尚未落位）。
    pub origin: &'static str,
    /// 引擎缺失时**计划**安装的版本——仅供"未安装"态展示，不得冒充已装。
    pub planned_version: Option<String>,
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
    fetch_client_latest_with(&UreqGet::new())
}

/// `fetch_client_latest` 的可注入实现（离线可测）。
fn fetch_client_latest_with(http: &dyn HttpGet) -> Option<String> {
    let url = APP_RELEASE_FEED?;
    let text = http
        .get_text(url, 1024 * 1024, Some("dsh-dock-updater"))
        .ok()?;
    parse_release_tag(&text)
}

/// 引擎三件探测（版本维度共用一次 spawn 开销）。
fn engine_status(data_dir: &Path) -> crate::engines::EngineStatus {
    crate::engines::probe_engine(data_dir, &resolve::effective_path())
}

/// Node 运行时维度：引擎实测优先（ADR-0010）；未装则报「未安装 + 计划版本」。
///
/// 探测层退役后只有引擎一个来源，故 `origin` 只可能是 `engine`（已装）或
/// `managed`（未装，走引导补齐）。**不再回退系统探测**——系统 node 不再是任何
/// 环节的来源（ADR-0010），报它只会误导。
fn node_runtime_info(data_dir: &Path) -> Option<NodeRuntimeInfo> {
    match engine_status(data_dir).node {
        Some(v) => Some(NodeRuntimeInfo {
            version: Some(v),
            origin: "engine",
            planned_version: None,
        }),
        None => Some(NodeRuntimeInfo {
            version: None,
            origin: "managed",
            planned_version: Some(node_plan(data_dir).version),
        }),
    }
}

/// 当前宿主 dsh 版本：引擎优先（ADR-0010），引擎未就绪回退系统探测
///（探测层退役前的过渡口径；两者都缺 = None，前端展示「未检出」）。
pub fn detect_current_version(data_dir: &Path) -> Option<String> {
    engine_status(data_dir).dsh
}

// ---------- 世界感知的引擎探测（2026-09-11，task-45 / D5） ----------
//
// 缺陷（v1.2.0 Windows 实测 2.2）：`get_update_status` 的版本/引擎维度**只探宿主**
// `engines/`，而健康大盘走 `mgmt::current_world` → 客体诊断（截图④：客体 dsh
// v0.1.5-rc.1 就绪）。WSL 模式下 dsh 装在客体，宿主必然为 None → 关于页「未检出」
// （截图③）——同一个 dsh，两块界面两个答案。ADR-0016 §2.6：呈现对象必须与**当前
// 会话实际运行的世界**一致。
//
// 复用既有客体通道（`guest.rs::diagnostics_script` → `diagnostics::
// collect_diagnostics_in_guest`），**不新造第二套客体探测**——同名第二实现是
// 本仓已记录的返工项。

/// 探测源：世界择源的结果（纯值，便于单测）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProbeSource {
    /// 宿主：壳引擎目录（现状路径，行为零变化）。
    Host,
    /// WSL 客体：`distro` 的世界；经既有客体诊断通道采集版本。
    Guest { distro: String },
}

/// 世界 → 探测源（纯函数，供单测）。
pub(crate) fn probe_source_for(world: &crate::mgmt::World) -> ProbeSource {
    match world {
        crate::mgmt::World::Local => ProbeSource::Host,
        crate::mgmt::World::Wsl { distro } => ProbeSource::Guest {
            distro: distro.clone(),
        },
    }
}

/// 客体版本探测的最小结果（只取关于页需要的两个维度）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct GuestVersions {
    /// 客体 dsh 实测版本；None = 客体**确实没有** dsh（不是探测失败）。
    pub dsh: Option<String>,
    /// 客体 node 实测版本；None = 客体确实没有 node。
    pub node: Option<String>,
}

/// 把宿主基底状态改写为「本世界」的实测结果（纯逻辑；探测经 `probe` 注入 → 离线可测）。
///
/// 契约兼容：只改 `dsh.current` / `dsh.error` / `dsh.newer` 与 `node` 四个既有字段的
/// **取值**，不新增字段（`ipc-shapes.json` 的形状闸门不动，前端零改动即可正确）。
pub(crate) fn status_for_source(
    base: UpdateStatus,
    source: &ProbeSource,
    probe: &dyn Fn(&str) -> Result<GuestVersions, String>,
) -> UpdateStatus {
    let ProbeSource::Guest { distro } = source else {
        // 宿主世界：现状路径，行为零变化（也不触碰客体探测）。
        return base;
    };
    let mut out = base;
    match probe(distro) {
        Ok(v) => {
            out.dsh.current = v.dsh;
            // 网络检查的既有失败标记与本维度无关，保留；但本次探测成功不得**新增**失败。
            out.dsh.newer = component_newer(&out.dsh.current, &out.dsh.latest);
            out.node = Some(NodeRuntimeInfo {
                version: v.node,
                // 客体 node 同属壳管理资产（ADR-0010：WSL 客体同源）——TS 契约的
                // origin 是闭集 engine|system|managed，此处取 engine，不加新值。
                origin: "engine",
                // 宿主 node 下载计划（node-map，宿主平台）不描述客体：不冒充客体计划。
                planned_version: None,
            });
        }
        Err(e) => {
            // **探测失败 ≠ 未检出**（DiagnosticsPane.tsx:69/99 的既有纪律）：
            // 如实报失败原因，绝不静默渲染成「未检出」。
            out.dsh.current = None;
            out.dsh.error = Some(match out.dsh.error.take() {
                Some(prev) => format!("{distro} 客体版本探测失败：{e}（另有检查失败：{prev}）"),
                None => format!("{distro} 客体版本探测失败：{e}"),
            });
            out.dsh.newer = false;
            out.node = None;
        }
    }
    out
}

/// `newer` 重算（与 `component_update` 同口径：两侧都有值才比对）。
fn component_newer(current: &Option<String>, latest: &Option<String>) -> bool {
    match (current, latest) {
        (Some(c), Some(l)) => is_newer(c, l),
        _ => false,
    }
}

/// 归一客体版本：与宿主 `engines::probe_engine` 的 `norm` 同口径
/// （trim + 剥一个 `v`）——两侧形态不一致会让 `is_newer` 拿 `v0` 去比 `0`。
fn norm_guest_version(v: &str) -> String {
    v.trim().trim_start_matches('v').to_string()
}

/// 客体版本的**真探测**：复用既有客体诊断通道（`guest.rs::diagnostics_script` →
/// `diagnostics::collect_diagnostics_in_guest`，30s 超时）——不新造第二套客体探测。
fn probe_guest_versions(distro: &str) -> Result<GuestVersions, String> {
    let report = crate::diagnostics::collect_diagnostics_in_guest(distro)?;
    Ok(guest_versions_from_report(&report))
}

/// 诊断报告 → 版本维度（**纯函数**，离线可测：哨兵值/`v` 前缀/未就绪三态）。
///
/// `is_ready` 是唯一可信判据：诊断结构在未就绪时把 version 填成哨兵「未检出」
/// （宿主 `collect_diagnostics` 与客体脚本两条路径同口径，2026-09-10 已核对）——
/// 直接读 version 会把哨兵当成版本号显示给用户。
fn guest_versions_from_report(
    report: &crate::diagnostics::SystemDiagnosticsReport,
) -> GuestVersions {
    GuestVersions {
        dsh: report
            .dsh
            .is_ready
            .then(|| report.dsh.version.clone())
            .flatten()
            .map(|v| norm_guest_version(&v)),
        node: report
            .node
            .is_ready
            .then(|| norm_guest_version(&report.node.version))
            .filter(|v| !v.is_empty() && v != "未检出"),
    }
}

/// 客体探测缓存 TTL。量级依据：诊断脚本含 `du -sb` 全 home 统计（客体侧秒级），
/// 且每次都要起一个 `wsl.exe` 子进程——关于页 + 托盘 + 手动刷新会重复问，必须缓存。
/// 60s：版本变化只可能由本壳的升级动作引起（显式、低频），一分钟的陈旧无风险。
const GUEST_PROBE_TTL: std::time::Duration = std::time::Duration::from_secs(60);

type GuestProbeCache =
    std::collections::HashMap<String, (std::time::Instant, Result<GuestVersions, String>)>;

fn guest_probe_cache() -> &'static std::sync::Mutex<GuestProbeCache> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<GuestProbeCache>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(GuestProbeCache::new()))
}

/// 缓存命中判定 + 探测（纯逻辑，时钟/探测均注入 → 离线可测）。
/// **失败也缓存**：客体不可达时反复重试只会反复付 30s 超时，同样在 TTL 内复用结论。
fn cache_get_or_probe(
    cache: &mut GuestProbeCache,
    distro: &str,
    now: std::time::Instant,
    ttl: std::time::Duration,
    probe: &dyn Fn(&str) -> Result<GuestVersions, String>,
) -> Result<GuestVersions, String> {
    if let Some((at, cached)) = cache.get(distro) {
        if now.duration_since(*at) < ttl {
            return cached.clone();
        }
    }
    let fresh = probe(distro);
    cache.insert(distro.to_string(), (now, fresh.clone()));
    fresh
}

/// 带缓存的客体版本探测（真实路径）。
fn guest_versions_cached(distro: &str) -> Result<GuestVersions, String> {
    let mut cache = guest_probe_cache()
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    cache_get_or_probe(
        &mut cache,
        distro,
        std::time::Instant::now(),
        GUEST_PROBE_TTL,
        &probe_guest_versions,
    )
}

/// 关于页/托盘读取路径的世界择源入口：按 `world` 取本世界的实测版本。
///
/// **不得阻塞启动**：调用方把本函数放在 `spawn_blocking` 里（客体探测会起
/// `wsl.exe` 子进程，最坏 30s 超时）。
///
/// 调用点（2026-09-11 订正）：**两条路径都走本函数**——
/// ① `commands/update.rs` 的关于页/托盘读取；② `boot::refresh_update_ui`
/// （首启 / 手动「检查更新」/ 升级后的后台刷新，经 `boot::status_for_world` 委托）。
/// 原注释写「两者互不重叠」已与事实不符：D5b 修复（v1.2.0 实测 2.2 的续修）
/// 的成因正是 `refresh_update_ui` 曾直连**只探宿主**的 `check_now` 并 emit，
/// 于是 WSL 模式下点一次「检查更新」就把关于页刷回「未检出」。
///
/// 由此增加的成本**已核并在可接受范围**（D5b 报告 §未决）：
/// ① 该刷新跑在**后台线程**，不阻塞 UI 与启动；
/// ② 客体探测有 **60s TTL 缓存**，关于页随后读取会命中缓存；
/// ③ 最坏情形（WSL 挂死）仅 30s 超时，且仍发生在后台。
pub(crate) fn world_aware_status(base: UpdateStatus, world: &crate::mgmt::World) -> UpdateStatus {
    status_for_source(base, &probe_source_for(world), &guest_versions_cached)
}

/// 世界无法确定（WSL 模式但发行版未记录）时的诚实降级：不回落宿主数据，
/// 而是把原因写进既有 `error` 字段（ADR-0016「绝不回落 Local」的同口径）。
pub(crate) fn status_world_unresolved(base: UpdateStatus, reason: &str) -> UpdateStatus {
    let mut out = base;
    out.dsh.current = None;
    out.dsh.error = Some(reason.to_string());
    out.dsh.newer = false;
    out.node = None;
    out
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
        preview_latest: None,
    }
}

/// dsh 维度检测（`check_now` 与测试共用）：可升级口径（稳定/rc 最高版）+
/// 预览口径（`preview_latest`）一次 packument 拆解。
fn dsh_component_update(current: Option<String>, packument: &serde_json::Value) -> ComponentUpdate {
    let (upgradable, preview) = split_dsh_versions(packument);
    let mut d = component_update(current, upgradable, None);
    d.preview_latest = preview;
    d
}

/// 一次完整检测（三维度）。网络失败不视为致命：对应维度 error 展示。
/// dsh 维度 = 可升级口径（稳定/rc 最高版），预览版（排序最高但不可接受）走
/// `preview_latest`——一次 packument 同时拆出两个口径（2026-09-09 假角标修复）。
pub fn check_now(data_dir: &Path) -> UpdateStatus {
    let engine_dsh = engine_status(data_dir).dsh;
    let dsh = match fetch_packument() {
        Ok(packument) => dsh_component_update(engine_dsh, &packument),
        Err(_) => component_update(
            engine_dsh,
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

/// 升级执行计划（纯函数 `plan_explicit` / `plan_default` 产出）：Skip = 目标
/// 与已装一致，短路跳过安装；Install = 实际执行 dsh 的 project 内安装（ADR-0017）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpgradePlan {
    Skip(String),
    Install(String),
}

/// 显式指定的安装目标版本形状校验（纯函数，供测试）。只做形状闸（semver-ish
/// 字符集 + 长度上限），**不按通道过滤**——alpha 经版本列表显式选择是合法目标。
/// fail-closed 防注入：版本号会拼进 `pnpm add pkg@<version>` 的同一 argv，
/// 虽有 `pkg@` 前缀兜底，仍拒绝 `-` 开头与越界字符（flag 注入反例见测试）。
pub fn is_valid_dsh_version_spec(v: &str) -> bool {
    !v.is_empty()
        && v.len() <= 64
        && !v.starts_with('-')
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+' | '_'))
}

/// 显式目标的升级计划（纯函数，供测试）：形状闸 + 同版本短路。不触网——
/// 离线回退 / 同版本短路均可判（2026-09-09：同版本短路，此前每次点击都
/// 空跑一遍 pnpm 解析+网络，用户视角 = 「升级了但没变化」）。
pub fn plan_explicit(target: &str, current: Option<&str>) -> Result<UpgradePlan> {
    if !is_valid_dsh_version_spec(target) {
        anyhow::bail!("非法的 DSH 版本号：{target}");
    }
    if current == Some(target) {
        Ok(UpgradePlan::Skip(target.to_string()))
    } else {
        Ok(UpgradePlan::Install(target.to_string()))
    }
}

/// 缺省目标的升级计划（纯函数，供测试）：目标 = 最新可接受版（稳定/rc），
/// 与已装一致则短路。
pub fn plan_default(current: Option<&str>, latest_stable: &str) -> UpgradePlan {
    if current == Some(latest_stable) {
        UpgradePlan::Skip(latest_stable.to_string())
    } else {
        UpgradePlan::Install(latest_stable.to_string())
    }
}

/// 升级引擎内 dsh（ADR-0010：升级全显式 + 引擎私有——不碰用户全局安装）。
/// `target` 缺省 = 最新可接受版（稳定/rc，触网解析）；显式版本（含 alpha，
/// 版本列表选择进入）过形状闸后安装，计划解析零网络（离线回退可判）。
/// 返回实际落位版本与是否真执行了安装。引擎 pnpm 缺位（boot 未跑成/目录被清）
/// 先从捆绑包重铺。
pub fn upgrade_engine_dsh(
    data_dir: &Path,
    resources_dir: &Path,
    path_env: &str,
    target: Option<&str>,
) -> Result<UpgradePlan> {
    let current = engine_status(data_dir).dsh;
    let plan = match target {
        Some(v) => plan_explicit(v, current.as_deref())?,
        None => plan_default(current.as_deref(), &latest_stable_dsh_version()?),
    };
    let version = match &plan {
        UpgradePlan::Skip(v) => {
            tracing::info!(version = %v, "dsh 目标版本与已装一致，短路跳过安装");
            return Ok(plan);
        }
        UpgradePlan::Install(v) => v.clone(),
    };
    tracing::info!(
        data_dir = %data_dir.display(),
        target = %version,
        "dsh 升级开始（引擎内 project 内安装，不触用户全局；ADR-0017）"
    );
    if !crate::engines::engine_pnpm_bin(data_dir).exists() {
        tracing::info!("引擎 pnpm 缺位，先从捆绑包重铺");
        crate::engines::stage_pnpm_from_bundle(&engine_pnpm_bundle(resources_dir), data_dir)
            .context("引擎 pnpm 重铺失败")?;
    }
    // 升级入口有自己的 busy 呈现（更新按钮），不消费 boot 进度卡——空回调。
    crate::engines::install_dsh_project_local(data_dir, &version, path_env, &mut |_, _, _| {})?;
    tracing::info!(version = %version, "dsh 升级完成");
    Ok(plan)
}

// ---------- dsh 的 project 内安装（pnpm 引擎内通道，ADR-0017） ----------

/// dsh 的 **project 内**安装实参（ADR-0017）：`pnpm add`（**无 `--global`**）。
///
/// **为什么不再 `-g`**：pnpm 12 的全局安装建 `global/v11/<hash>` 符号链接，Windows
/// 普通账户无 `SeCreateSymbolicLinkPrivilege` ⇒ `os error 5`，且无配置可绕（ADR-0017 §1）。
/// 安装目标目录由调用方经 `run_engine_pnpm_streaming` 的 `cwd` 指定
/// （`<engines>/dsh-runtime/`），故实参里不需要路径。
///
/// pnpm 10 只识别 `--allow-build=<package>` 形式，必须显式拼在同一个参数中。
pub(crate) fn pnpm_project_install_args(registry: &str, spec: &str) -> Vec<String> {
    let mut args = vec![
        "add".to_string(),
        "--registry".to_string(),
        registry.to_string(),
    ];
    args.extend(pnpm_allow_build_flags());
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

/// WSL 客体投递份（ADR-0010 台账「客体投递」）：Windows 包内置 linux-x64
///（glibc）tgz，客体架构固定 x64；随 fetch-pnpm-bundle.sh 于 CI 取得。
#[cfg_attr(not(windows), allow(dead_code))] // 消费点在 Windows 客体投递链
pub fn guest_pnpm_bundle(resources_dir: &Path) -> PathBuf {
    resources_dir.join("pnpm").join("linux-x64.tgz")
}

/// dsh 引导的 allow-build 放行旗标（pnpm v10 起构建脚本需显式放行；
/// host `pnpm_project_install_args` 与 WSL 客体脚本单源共用）。
pub(crate) fn pnpm_allow_build_flags() -> Vec<String> {
    PNPM_BUILD_PACKAGES
        .iter()
        .map(|package| format!("--allow-build={package}"))
        .collect()
}

/// dsh 版本通道（2026-09-09）：升级可接受口径 = Stable | Rc（即
/// `is_acceptable_dsh_version`，单一事实源）；Alpha / Other（beta 等其余预发布
/// 标签）只进版本列表，经用户显式选择安装，永不进升级判定与「有新版」。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DshVersionChannel {
    Stable,
    Rc,
    Alpha,
    Other,
}

impl DshVersionChannel {
    /// 序列化口径（`DshVersionEntry.channel`，前端按此映射徽章与文案）。
    pub fn as_str(self) -> &'static str {
        match self {
            DshVersionChannel::Stable => "stable",
            DshVersionChannel::Rc => "rc",
            DshVersionChannel::Alpha => "alpha",
            DshVersionChannel::Other => "other",
        }
    }
}

/// 版本通道归类（纯函数，供测试）。
pub fn dsh_version_channel(v: &str) -> DshVersionChannel {
    let lower = v.to_ascii_lowercase();
    if lower.contains("alpha") {
        DshVersionChannel::Alpha
    } else if lower.contains("-rc") || lower.contains(".rc") {
        DshVersionChannel::Rc
    } else if !v.contains('-') {
        DshVersionChannel::Stable
    } else {
        DshVersionChannel::Other
    }
}

/// 判断 dsh 版本是否满足引导要求：
/// 稳定版（无连字符）与 rc 候选版（包含 -rc）均可接受；
/// alpha 等未稳定的前置预览版本明确拒绝（防依赖未完整发布等启动事故）。
pub fn is_acceptable_dsh_version(v: &str) -> bool {
    matches!(
        dsh_version_channel(v),
        DshVersionChannel::Stable | DshVersionChannel::Rc
    )
}

/// dsh 引导目标版本：排序最高可接受版本（稳定版优先；无稳定版时接受 rc，明确排除 alpha 等未稳定版本）。
pub fn latest_stable_dsh_version() -> Result<String> {
    let packument =
        fetch_packument().context("无法获取官方版本列表（registry 不可达或返回异常）")?;
    parse_versions(&packument)
        .into_iter()
        .find(|v| is_acceptable_dsh_version(v))
        .ok_or_else(|| {
            anyhow::anyhow!("官方版本列表无可引导版本（需稳定版或 rc 候选版，排除 alpha）")
        })
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
    npm_packument_versions_with(&registry_chain(), &UreqGet::new(), package)
}

/// `npm_packument_versions` 的可注入实现（离线覆盖镜像链回退）。
/// `bases` = registry 基址链（生产传 `registry_chain()`，测试传固定链）。
fn npm_packument_versions_with(
    bases: &[String],
    http: &dyn HttpGet,
    package: &str,
) -> Result<(String, Vec<String>), String> {
    if package.is_empty() || package.starts_with('-') {
        return Err("包名非法".to_string());
    }
    let mut last_err = String::from("镜像链均不可达");
    for base in bases {
        let url = format!("{base}/{}", package.replace('/', "%2F"));
        let text = match http.get_text(&url, PACKUMENT_MAX_BYTES, None) {
            Ok(t) => t,
            Err(e) => {
                last_err = format!("{base}：{e}");
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
    fetch_market_registry_with(&UreqGet::new())
}

/// `fetch_market_registry` 的可注入实现（离线可测镜像链回退）。
fn fetch_market_registry_with(http: &dyn HttpGet) -> Result<String, String> {
    let mut last_err = String::from("市场 Registry 均不可达");
    for url in MARKET_REGISTRY_URLS {
        tracing::info!("读取插件市场 Registry: {url}");
        match http.get_text(url, MARKET_REGISTRY_MAX_BYTES, None) {
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
mod world_probe_tests {

    /// **ADR-0017**：dsh 安装实参**不得含全局标志**——这是"project 内安装"的
    /// 实参级判据（与 `engines.rs` 的源码闸门互补：那条管生产段不出现旧构造，
    /// 这条管新构造本身确实非全局）。
    #[test]
    fn project_install_args_are_not_global() {
        let args =
            pnpm_project_install_args("https://registry.npmjs.org/", "@deepseek-ai/dsh@0.1.5-rc.2");
        assert_eq!(args[0], "add", "首个实参必须是 add");
        for forbidden in ["--global", "-g", "--location=global"] {
            assert!(
                !args.iter().any(|a| a == forbidden),
                "project 内安装不得出现全局标志 `{forbidden}`：{args:?}"
            );
        }
        // registry 与 spec 必须在（复用既有镜像链语义）
        assert!(args
            .windows(2)
            .any(|w| w == ["--registry", "https://registry.npmjs.org/"]));
        assert_eq!(args.last().unwrap(), "@deepseek-ai/dsh@0.1.5-rc.2");
        // allow-build 放行口径不变（ADR-0013）
        assert!(
            args.iter().any(|a| a.starts_with("--allow-build=")),
            "allow-build 放行口径必须保留：{args:?}"
        );
    }
    use super::*;

    fn base_host_status() -> UpdateStatus {
        UpdateStatus {
            // 宿主演现状：WSL 模式下 dsh 装在客体 → 宿主探测恒 None（截图③「未检出」）
            dsh: component_update(None, Some("0.1.5-rc.2".to_string()), None),
            client: component_update(Some("1.2.0".to_string()), None, None),
            node: Some(NodeRuntimeInfo {
                version: Some("v24.18.0".to_string()),
                origin: "engine",
                planned_version: None,
            }),
        }
    }

    fn never_called(_: &str) -> Result<GuestVersions, String> {
        panic!("宿主世界不得触碰客体探测（会白起 WSL 子进程）");
    }

    /// 世界择源（ADR-0016 §2.6）：Local → 宿主；Wsl{distro} → 该发行版客体。
    #[test]
    fn probe_source_follows_the_running_world() {
        assert_eq!(
            probe_source_for(&crate::mgmt::World::Local),
            ProbeSource::Host
        );
        assert_eq!(
            probe_source_for(&crate::mgmt::World::Wsl {
                distro: "Ubuntu-20.04".to_string()
            }),
            ProbeSource::Guest {
                distro: "Ubuntu-20.04".to_string()
            },
            "WSL 世界必须择客体源，否则关于页与健康大盘继续互相矛盾（D5 根因）"
        );
    }

    /// **D5 主断言**：WSL 世界的 dsh/node 实测版本必须来自客体——
    /// 客体有 v0.1.5-rc.1 时，关于页不得再显示「未检出」。
    #[test]
    fn guest_world_reports_the_guest_versions() {
        let probe = |d: &str| {
            assert_eq!(d, "Ubuntu-20.04");
            Ok(GuestVersions {
                dsh: Some("0.1.5-rc.1".to_string()),
                node: Some("v24.18.0".to_string()),
            })
        };
        let out = status_for_source(
            base_host_status(),
            &ProbeSource::Guest {
                distro: "Ubuntu-20.04".to_string(),
            },
            &probe,
        );
        assert_eq!(
            out.dsh.current.as_deref(),
            Some("0.1.5-rc.1"),
            "客体 dsh 版本必须呈现在关于页（对照截图④的就绪版本）"
        );
        assert_eq!(out.dsh.error, None, "探测成功不得带失败标记");
        let node = out.node.expect("客体 node 维度不得为 None");
        assert_eq!(node.version.as_deref(), Some("v24.18.0"));
        assert_eq!(node.origin, "engine", "客体引擎同属壳管理资产（ADR-0010）");
        assert_eq!(
            node.planned_version, None,
            "宿主 node 下载计划不描述客体，不得冒充客体计划"
        );
    }

    /// 客体版本必须参与 `newer` **重算**：基底状态的 `newer` 是用宿主
    /// `current = None` 算出来的（恒 false），直接用会让「客体确实有新版」漏报。
    /// 语义：`newer` = `latest > current`（有更新可用），故客体低于 latest → true。
    #[test]
    fn guest_world_recomputes_newer_from_guest_version() {
        let lower = |_: &str| {
            Ok(GuestVersions {
                dsh: Some("0.1.5-rc.1".to_string()),
                node: None,
            })
        };
        let out = status_for_source(
            base_host_status(),
            &ProbeSource::Guest {
                distro: "Ubuntu".to_string(),
            },
            &lower,
        );
        assert!(
            out.dsh.newer,
            "客体 0.1.5-rc.1 < latest 0.1.5-rc.2 → 应判有新版（基底恒 false，未重算即漏报）"
        );

        // 反向：客体版本高于 latest（如手装 alpha/预览）→ 不得误报「有新版」
        let higher = |_: &str| {
            Ok(GuestVersions {
                dsh: Some("0.1.5-rc.3".to_string()),
                node: None,
            })
        };
        let out = status_for_source(
            base_host_status(),
            &ProbeSource::Guest {
                distro: "Ubuntu".to_string(),
            },
            &higher,
        );
        assert!(
            !out.dsh.newer,
            "客体 0.1.5-rc.3 > latest 0.1.5-rc.2 → 不应报「有新版」"
        );
    }

    /// **探测失败 ≠ 未检出**（DiagnosticsPane.tsx:69/99 的既有纪律）：
    /// 客体不可达时必须带失败原因，不得静默渲染成「未检出」。
    #[test]
    fn guest_probe_failure_is_reported_as_failure_not_absence() {
        let probe = |_: &str| Err("wsl.exe 调用失败或无输出（客体不可达？）".to_string());
        let out = status_for_source(
            base_host_status(),
            &ProbeSource::Guest {
                distro: "Ubuntu-20.04".to_string(),
            },
            &probe,
        );
        assert_eq!(out.dsh.current, None);
        let err = out.dsh.error.expect("探测失败必须带原因");
        assert!(err.contains("Ubuntu-20.04"), "失败原因须点名发行版：{err}");
        assert!(err.contains("客体不可达"), "失败原因须保留底层原因：{err}");
    }

    /// 客体**确实没装** dsh（脚本跑通、版本为空）→ 这才是「未检出」，且不得带失败标记。
    #[test]
    fn guest_absence_is_not_a_probe_failure() {
        let probe = |_: &str| Ok(GuestVersions::default());
        let out = status_for_source(
            base_host_status(),
            &ProbeSource::Guest {
                distro: "Ubuntu".to_string(),
            },
            &probe,
        );
        assert_eq!(out.dsh.current, None);
        assert_eq!(out.dsh.error, None, "装没装与「探测失败」是两件事");
    }

    /// 宿主世界**零行为变化**：不得触碰客体探测（否则每次读关于页都白起 WSL 子进程）。
    #[test]
    fn local_world_never_probes_the_guest() {
        let before = base_host_status();
        let out = status_for_source(before.clone(), &ProbeSource::Host, &never_called);
        assert_eq!(out.dsh.current, before.dsh.current);
        assert_eq!(out.dsh.latest, before.dsh.latest);
        assert_eq!(
            out.node.map(|n| n.version),
            Some(before.node.map(|n| n.version)).flatten()
        );
    }

    /// 诊断报告 → 版本维度：哨兵「未检出」不得当版本号；`v` 前缀按宿主同口径剥掉。
    #[test]
    fn guest_report_maps_to_versions_without_sentinel_or_v_prefix() {
        use crate::diagnostics::{
            DshDiagnosticInfo, NodeDiagnosticInfo, PlatformDiagnosticInfo, PnpmDiagnosticInfo,
            StorageDiagnosticInfo, SystemDiagnosticsReport,
        };
        let mk = |node_v: &str, node_ready: bool, dsh_v: Option<&str>, dsh_ready: bool| {
            SystemDiagnosticsReport {
                node: NodeDiagnosticInfo {
                    path: "/home/guan/.dsh-dock/engines/bin/node".into(),
                    version: node_v.into(),
                    source: "engine".into(),
                    is_ready: node_ready,
                },
                pnpm: PnpmDiagnosticInfo {
                    path: String::new(),
                    version: None,
                    is_ready: false,
                },
                dsh: DshDiagnosticInfo {
                    path: "/home/guan/.dsh-dock/engines/bin/dsh".into(),
                    version: dsh_v.map(String::from),
                    source: "engine".into(),
                    is_ready: dsh_ready,
                },
                storage: StorageDiagnosticInfo {
                    dsh_home: String::new(),
                    total_bytes: 0,
                    profiles_bytes: 0,
                    sessions_bytes: 0,
                    profiles_count: 0,
                    sessions_count: 0,
                },
                platform: PlatformDiagnosticInfo {
                    os: "linux".into(),
                    arch: "x86_64".into(),
                },
            }
        };

        // ④ 图的实际形态：客体两者就绪（node 带 v，dsh 为 rc 版本）
        let both = guest_versions_from_report(&mk("v24.18.0", true, Some("v0.1.5-rc.1"), true));
        assert_eq!(
            both.node.as_deref(),
            Some("24.18.0"),
            "v 前缀须按宿主 norm 同口径剥掉"
        );
        assert_eq!(both.dsh.as_deref(), Some("0.1.5-rc.1"));

        // 未就绪：node 版本是哨兵「未检出」，绝不可当版本号透出
        let none_ready = guest_versions_from_report(&mk("未检出", false, None, false));
        assert_eq!(none_ready.node, None, "哨兵不得冒充客体 node 版本");
        assert_eq!(none_ready.dsh, None, "未就绪 + version 缺失 = 客体确实没有");

        // 边界：is_ready 为真但版本是哨兵（异常组合）→ 仍不得透出哨兵
        let odd = guest_versions_from_report(&mk("未检出", true, Some("未检出"), true));
        assert_eq!(odd.node, None, "is_ready 为真但版本是哨兵：宁可报无");
        assert_eq!(
            odd.dsh.as_deref(),
            Some("未检出"),
            "dsh 走 Option 通道，交由上层判据"
        );
    }

    /// **缓存（性能纪律）**：TTL 内重复调用只探测一次；过期后重新探测；
    /// 失败同样进缓存（避免反复付 30s 超时）。
    #[test]
    fn guest_probe_cache_hits_within_ttl_and_refreshes_after() {
        use std::cell::Cell;
        use std::time::{Duration, Instant};

        let calls = Cell::new(0);
        let probe = |_: &str| {
            calls.set(calls.get() + 1);
            Ok(GuestVersions {
                dsh: Some(format!("0.1.5-rc.{}", calls.get())),
                node: None,
            })
        };
        let ttl = Duration::from_secs(60);
        let t0 = Instant::now();
        let mut cache = GuestProbeCache::new();

        // 首次：探测
        let a = cache_get_or_probe(&mut cache, "Ubuntu", t0, ttl, &probe).unwrap();
        assert_eq!(calls.get(), 1);
        // TTL 内多次（关于页 + 托盘 + 刷新）：命中缓存，不再起 WSL
        for _ in 0..5 {
            let _ = cache_get_or_probe(
                &mut cache,
                "Ubuntu",
                t0 + Duration::from_secs(30),
                ttl,
                &probe,
            );
        }
        assert_eq!(
            calls.get(),
            1,
            "TTL 内 6 次调用应只探测 1 次（缓存命中 5 次）"
        );
        assert_eq!(a.dsh.as_deref(), Some("0.1.5-rc.1"));
        // 过期：重新探测
        let b = cache_get_or_probe(&mut cache, "Ubuntu", t0 + ttl, ttl, &probe).unwrap();
        assert_eq!(calls.get(), 2, "TTL 到期后应重新探测");
        assert_eq!(b.dsh.as_deref(), Some("0.1.5-rc.2"));
        // 不同发行版各自成键（不串味）
        let _ = cache_get_or_probe(&mut cache, "Debian", t0, ttl, &probe);
        assert_eq!(calls.get(), 3, "换发行版必须重新探测");

        // 失败也缓存：客体不可达时不反复重试
        let fails = Cell::new(0);
        let bad = |_: &str| {
            fails.set(fails.get() + 1);
            Err("wsl.exe 调用失败".to_string())
        };
        let mut cache2 = GuestProbeCache::new();
        for _ in 0..4 {
            let _ = cache_get_or_probe(&mut cache2, "Ubuntu", t0, ttl, &bad);
        }
        assert_eq!(
            fails.get(),
            1,
            "失败结论同样在 TTL 内复用（否则每次白付 30s 超时）"
        );
    }

    /// 世界未定（WSL 模式但发行版未记录）：**绝不回落宿主数据**——
    /// 否则关于页又会拿宿主「未检出」充当客体结论（ADR-0016「绝不回落 Local」）。
    #[test]
    fn unresolved_world_reports_reason_instead_of_host_data() {
        let out = status_world_unresolved(base_host_status(), "无法确定当前管理世界：…");
        assert_eq!(out.dsh.current, None);
        assert!(
            out.dsh
                .error
                .as_deref()
                .unwrap_or("")
                .contains("无法确定当前管理世界"),
            "必须如实给出未定原因：{:?}",
            out.dsh.error
        );
        assert!(!out.dsh.newer);
        assert!(out.node.is_none(), "世界未定时不得拿宿主 node 冒充");
    }

    /// **客体脚本契约**（真客体行为不可测的替代验证）：在宿主 bash 里真跑
    /// `guest.rs::diagnostics_script`，把它的真实输出经 `guest_versions_from_report`
    /// 解析——证明「脚本输出 ⇄ 我读的字段」这条接口一致（哨兵/`isReady`/版本形态）。
    /// 脚本内容本身读 `which node/dsh --version`，故用假 shim 目录模拟客体 PATH。
    ///
    /// **shim 必须落在 `$HOME/.dsh-dock/engines/bin`，不能只靠 PATH**（2026-09-11 CI 实测）：
    /// `guest_prep!()` 开头会 `. /etc/profile`，而 macOS 的 `/etc/profile` 执行
    /// `path_helper` —— 它把 `/etc/paths` 各项（首项 `/usr/local/bin`）**前置**到 PATH。
    /// GitHub macOS runner 的 node 恰在 `/usr/local/bin`（v24.20.0）⇒ 只把 shim 放进 PATH
    /// 会被 runner 的真 node 顶掉（实测 `left: Some("24.20.0")`）。本机却**恰好绿**：
    /// `/usr/local/bin/node` 不存在，而本机真 node（引擎 bin）**也是 v24.18.0，与夹具同值**
    /// —— 双重巧合掩盖了它，属 M10/M11「夹具可移植性」同类。
    /// 引擎 bin 是脚本 **source 三个 rc 之后**才前置的（`PATH="$DSH_ENGINES/bin:$PATH"`），
    /// 故放这里确定胜出。
    #[cfg(unix)]
    #[test]
    fn guest_script_output_parses_into_versions() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("dsh-dock-d45-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let home = root.join("home");
        // 引擎 bin：脚本在 source 完 rc 之后前置它，故这里是唯一能压过宿主 PATH 的位置。
        let engine_bin = home.join(".dsh-dock").join("engines").join("bin");
        std::fs::create_dir_all(&engine_bin).unwrap();
        // 假 shim：node → v24.18.0（带 v，验 v 前缀归一），dsh → 0.1.5-rc.1（不带 v）
        for (name, out) in [("node", "v24.18.0"), ("dsh", "0.1.5-rc.1")] {
            let p = engine_bin.join(name);
            std::fs::write(&p, format!("#!/bin/sh\necho {out}\n")).unwrap();
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let script = crate::guest::diagnostics_script("Ubuntu-20.04");
        let out = std::process::Command::new("bash")
            .arg("-c")
            .arg(&script)
            .env("HOME", &home)
            .env("PATH", format!("{}:/usr/bin:/bin", engine_bin.display()))
            .env_remove("DSH_HOME")
            // spawn-gate: exempt(测试专用——在宿主 bash 里真跑客体脚本，做「脚本输出 ⇄ 读取字段」契约断言；本模块名非 `mod tests`，spawn 闸门的 cfg(test) 截断标记覆盖不到它，2026-09-11)
            .output()
            .expect("bash 应可用");
        let stdout = String::from_utf8_lossy(&out.stdout);
        let line = stdout
            .lines()
            .find(|l| l.starts_with(crate::guest::DIAG_SENTINEL))
            .unwrap_or_else(|| panic!("脚本未输出哨兵行：{stdout}"));
        let report: crate::diagnostics::SystemDiagnosticsReport =
            serde_json::from_str(&line[crate::guest::DIAG_SENTINEL.len()..])
                .expect("脚本 JSON 应可解析");

        let v = guest_versions_from_report(&report);
        // **夹具自证**（防上面那个坑复发）：脚本必须读到**我们的** shim，而不是
        // 宿主 PATH 上的真 node/pnpm/dsh。若哪天 shim 位置又被改回"只进 PATH"，
        // 这条会比版本断言更早、更明确地报出原因（而不是在 CI 上莫名其妙地版本不符）。
        assert!(
            std::path::Path::new(&report.node.path).starts_with(&home),
            "脚本读到的 node 不在假 HOME 内（{}）——夹具 shim 未生效，说明宿主 PATH \
             抢在了前面；shim 须落在 $HOME/.dsh-dock/engines/bin（脚本 source 完 rc 后才前置）",
            report.node.path
        );
        assert_eq!(v.node.as_deref(), Some("24.18.0"), "脚本的 v 前缀须被归一");
        assert_eq!(v.dsh.as_deref(), Some("0.1.5-rc.1"));

        let _ = std::fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod packument_tests {
    use super::*;

    #[test]
    fn parse_chain_splits_trims_and_falls_back_to_defaults() {
        assert_eq!(
            parse_chain(" https://a , ,https://b ", &["https://x"]),
            vec!["https://a".to_string(), "https://b".to_string()]
        );
        assert_eq!(
            parse_chain("", &["https://d1", "https://d2"]),
            vec!["https://d1".to_string(), "https://d2".to_string()],
            "空输入回退默认链"
        );
        assert_eq!(
            parse_chain("  , , ", &["https://d"]),
            vec!["https://d".to_string()],
            "全空白同样回退默认链"
        );
        assert_eq!(
            parse_chain("https://only", &[]),
            vec!["https://only".to_string()],
            "单项链合法（CI 常用：官方源优先去重）"
        );
    }

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

    // ---------- 响应体上限（离线，2026-09-08 架构评审批次 5）----------
    // `read_body_capped` 是网络面里唯一可离线全覆盖的一层：ureq 的 `into_string()`
    // 自带内部上限且阈值随版本漂移，本函数把它换成显式、可测的实现——此前零测试。

    #[test]
    fn body_cap_accepts_payload_under_limit() {
        let body = "a".repeat(100);
        let out = read_body_capped(std::io::Cursor::new(body.clone()), 1024).unwrap();
        assert_eq!(out, body, "未超限应原样返回");
    }

    #[test]
    fn body_cap_accepts_payload_exactly_at_limit() {
        let body = "a".repeat(64);
        let out = read_body_capped(std::io::Cursor::new(body.clone()), 64).unwrap();
        assert_eq!(out.len(), 64, "恰好等于上限应放行（边界含等号）");
    }

    #[test]
    fn body_cap_rejects_payload_over_limit() {
        let body = "a".repeat(65);
        let err = read_body_capped(std::io::Cursor::new(body), 64).unwrap_err();
        assert!(
            err.to_string().contains("超过 64 字节上限"),
            "超限应报明确上限文案，实测：{err}"
        );
    }

    #[test]
    fn body_cap_handles_empty_body() {
        let out = read_body_capped(std::io::Cursor::new(Vec::<u8>::new()), 16).unwrap();
        assert_eq!(out, "", "空响应体合法（调用方按空串走解析失败路径）");
    }

    #[test]
    fn body_cap_rejects_invalid_utf8() {
        let bytes = vec![0xff, 0xfe, 0xfd];
        let err = read_body_capped(std::io::Cursor::new(bytes), 16).unwrap_err();
        assert!(
            err.to_string().contains("读取响应体失败"),
            "非 UTF-8 应带上下文报错，实测：{err}"
        );
    }

    #[test]
    fn acceptable_dsh_version_accepts_stable_and_rc_rejects_alpha() {
        assert!(is_acceptable_dsh_version("0.1.2"));
        assert!(is_acceptable_dsh_version("0.1.2-rc.1"));
        assert!(is_acceptable_dsh_version("0.1.0-rc.8"));
        assert!(!is_acceptable_dsh_version("0.1.2-alpha.2"));
        assert!(!is_acceptable_dsh_version("0.1.2-alpha.5"));
    }

    // ---------- dsh 版本口径与版本列表（2026-09-09 假角标修复 + 版本选择器）----------
    //
    // 复现：registry 高位版本只有 alpha（如实况 0.1.5-alpha.1 > 0.1.2-rc.1）时，
    // 检测按「排序最高」报有新版、升级按「排除 alpha」装回同版——用户视角 =
    // 「点了升级不成功也不报错」。修复后检测与升级同口径：可升级判定只认
    // 稳定/rc，alpha 进 preview_latest 走版本列表显式选择。

    /// 实况镜像 fixture：dist-tags.latest = 0.1.2-rc.1，高位只有 alpha。
    fn dsh_packument() -> serde_json::Value {
        serde_json::json!({
            "dist-tags": {"latest": "0.1.2-rc.1", "alpha": "0.1.5-alpha.1"},
            "versions": {
                "0.1.2-rc.1": {},
                "0.1.3-alpha.2": {},
                "0.1.5-alpha.1": {}
            },
            "time": {
                "created": "2026-08-01T00:00:00Z",
                "modified": "2026-09-07T00:00:00Z",
                "0.1.2-rc.1": "2026-09-04T09:00:00.000Z",
                "0.1.3-alpha.2": "2026-08-30T00:00:00.000Z",
                "0.1.5-alpha.1": "2026-09-07T00:00:00.000Z"
            }
        })
    }

    #[test]
    fn version_channel_classifies_stable_rc_alpha_other() {
        assert_eq!(dsh_version_channel("0.1.2"), DshVersionChannel::Stable);
        assert_eq!(dsh_version_channel("0.1.2-rc.1"), DshVersionChannel::Rc);
        assert_eq!(dsh_version_channel("0.1.0-RC8"), DshVersionChannel::Rc);
        assert_eq!(dsh_version_channel("0.1.2.rc1"), DshVersionChannel::Rc);
        assert_eq!(
            dsh_version_channel("0.1.2-alpha.5"),
            DshVersionChannel::Alpha
        );
        assert_eq!(
            dsh_version_channel("0.1.2-beta.1"),
            DshVersionChannel::Other
        );
        // 通道归类与可接受口径单源一致
        assert!(is_acceptable_dsh_version("0.1.2.rc1"));
        assert!(!is_acceptable_dsh_version("0.1.2-beta.1"));
    }

    #[test]
    fn split_versions_alpha_above_rc_keeps_upgradable_and_preview() {
        let (upgradable, preview) = split_dsh_versions(&dsh_packument());
        assert_eq!(upgradable.as_deref(), Some("0.1.2-rc.1"));
        assert_eq!(preview.as_deref(), Some("0.1.5-alpha.1"));
    }

    #[test]
    fn split_versions_highest_acceptable_means_no_preview() {
        let mut p = dsh_packument();
        // 0.1.6-rc.1 排序高于 0.1.5-alpha.1 且可接受 → 不再报预览
        p["versions"]["0.1.6-rc.1"] = serde_json::json!({});
        let (upgradable, preview) = split_dsh_versions(&p);
        assert_eq!(upgradable.as_deref(), Some("0.1.6-rc.1"));
        assert_eq!(preview, None, "排序最高是可接受版时不再报预览");
    }

    #[test]
    fn split_versions_alpha_only_registry_has_no_upgradable() {
        let p = serde_json::json!({"versions": {"0.1.5-alpha.1": {}}});
        let (upgradable, preview) = split_dsh_versions(&p);
        assert_eq!(upgradable, None);
        assert_eq!(preview.as_deref(), Some("0.1.5-alpha.1"));
    }

    /// 假角标回归：current 已是可升级口径最高版 → newer=false（修复前
    /// fetch_latest_version 取 0.1.5-alpha.1 会误报 newer=true）。
    #[test]
    fn dsh_component_update_current_at_upgradable_is_not_newer() {
        let d = dsh_component_update(Some("0.1.2-rc.1".to_string()), &dsh_packument());
        assert!(
            !d.newer,
            "0.1.2-rc.1 已是可升级口径最高版，不得报「有新版」"
        );
        assert_eq!(d.latest.as_deref(), Some("0.1.2-rc.1"));
        assert_eq!(d.preview_latest.as_deref(), Some("0.1.5-alpha.1"));
    }

    #[test]
    fn version_entries_relation_channel_and_time() {
        let entries = parse_dsh_version_entries(&dsh_packument(), Some("0.1.3-alpha.2"));
        let vs: Vec<&str> = entries.iter().map(|e| e.version.as_str()).collect();
        assert_eq!(vs, ["0.1.5-alpha.1", "0.1.3-alpha.2", "0.1.2-rc.1"], "降序");
        let by_v = |v: &str| entries.iter().find(|e| e.version == v).unwrap();
        let alpha = by_v("0.1.5-alpha.1");
        assert_eq!(alpha.channel, "alpha");
        assert_eq!(alpha.relation, "newer");
        assert_eq!(
            alpha.published_at.as_deref(),
            Some("2026-09-07T00:00:00.000Z")
        );
        assert_eq!(by_v("0.1.3-alpha.2").relation, "current");
        let rc = by_v("0.1.2-rc.1");
        assert_eq!(rc.relation, "older");
        assert_eq!(rc.channel, "rc");
        // time 元键（created/modified）不进版本列表
        assert!(entries.iter().all(|e| e.version != "created"));
    }

    #[test]
    fn version_entries_without_current_are_all_newer() {
        let entries = parse_dsh_version_entries(&dsh_packument(), None);
        assert!(entries.iter().all(|e| e.relation == "newer"));
    }

    #[test]
    fn version_spec_shape_gate_rejects_injection_and_empty() {
        // 正例：稳定 / rc / alpha / 带构建元数据
        assert!(is_valid_dsh_version_spec("0.1.2"));
        assert!(is_valid_dsh_version_spec("0.1.2-rc.1"));
        assert!(is_valid_dsh_version_spec("0.1.5-alpha.1"));
        assert!(is_valid_dsh_version_spec("0.1.5-alpha.1+build.7"));
        // 反例：空 / flag 注入 / 越界字符 / 过长
        assert!(!is_valid_dsh_version_spec(""));
        assert!(!is_valid_dsh_version_spec(
            "0.1.2 --registry=http://evil.example"
        ));
        assert!(!is_valid_dsh_version_spec("-flag"));
        assert!(!is_valid_dsh_version_spec("0.1.2/../../etc"));
        assert!(!is_valid_dsh_version_spec(&"a".repeat(65)));
    }

    #[test]
    fn plan_upgrade_defaults_latest_short_circuits_same_and_gates_shape() {
        use UpgradePlan::{Install, Skip};
        // 缺省目标 = 最新可接受版；与已装一致 → 短路（同版本空跑修复）
        assert_eq!(
            plan_default(Some("0.1.2-rc.1"), "0.1.2-rc.1"),
            Skip("0.1.2-rc.1".to_string())
        );
        assert_eq!(
            plan_default(Some("0.1.1-rc.2"), "0.1.2-rc.1"),
            Install("0.1.2-rc.1".to_string())
        );
        // 显式 alpha 是合法目标（版本列表选择进入）；与已装一致 → 短路
        assert_eq!(
            plan_explicit("0.1.5-alpha.1", None).unwrap(),
            Install("0.1.5-alpha.1".to_string())
        );
        assert_eq!(
            plan_explicit("0.1.5-alpha.1", Some("0.1.5-alpha.1")).unwrap(),
            Skip("0.1.5-alpha.1".to_string())
        );
        // 当前版本未检出 → 不短路（pnpm 幂等重装无害）
        assert_eq!(
            plan_explicit("0.1.2-rc.1", None).unwrap(),
            Install("0.1.2-rc.1".to_string())
        );
        // 非法形状（flag 注入反例）拒绝
        assert!(plan_explicit("x --registry=http://evil", None).is_err());
    }

    // ---------- HTTP seam（离线，2026-09-08 架构评审批次 5 · P6）----------
    // 镜像链回退 / 坏响应跳过 / 全失败收敛属于**编排**，过去只能靠真网络或人工验证。
    // 这里用「URL 子串 → 预置响应」的假体换掉 ureq：不触网、不起 mock server，
    // 逐条覆盖分支（真网络编排仍走 docs/executor.md 验证清单）。

    /// 离线 HTTP 假体：首个命中的 URL 子串生效；未命中即失败并记录调用顺序。
    #[derive(Default)]
    struct FakeHttp {
        text: Vec<(
            &'static str,
            std::result::Result<&'static str, &'static str>,
        )>,
        bytes: Vec<(&'static str, Vec<u8>)>,
        calls: std::cell::RefCell<Vec<String>>,
    }

    impl FakeHttp {
        fn calls(&self) -> Vec<String> {
            self.calls.borrow().clone()
        }
    }

    impl HttpGet for FakeHttp {
        fn get_text(&self, url: &str, _cap: u64, _ua: Option<&str>) -> Result<String> {
            self.calls.borrow_mut().push(url.to_string());
            match self.text.iter().find(|(k, _)| url.contains(*k)) {
                Some((_, Ok(body))) => Ok((*body).to_string()),
                Some((_, Err(msg))) => Err(anyhow::anyhow!(*msg)),
                None => Err(anyhow::anyhow!("假体未预置该 URL：{url}")),
            }
        }

        fn get_bytes(&self, url: &str, _cap: u64) -> Result<Vec<u8>> {
            self.calls.borrow_mut().push(url.to_string());
            match self.bytes.iter().find(|(k, _)| url.contains(*k)) {
                Some((_, body)) => Ok(body.clone()),
                None => Err(anyhow::anyhow!("假体未预置该 URL：{url}")),
            }
        }
    }

    const PK1: &str = "https://m1.invalid/@deepseek-ai%2Fdsh";
    const PK2: &str = "https://m2.invalid/@deepseek-ai%2Fdsh";
    const NM_PK1: &str = "https://m1.invalid/@dsh-dock%2Fnode-map";
    const GOOD_PACKUMENT: &str =
        r#"{"dist-tags":{"latest":"0.16.1"},"versions":{"0.16.1":{"dist":{}}}}"#;

    fn bases2() -> Vec<String> {
        vec![
            "https://m1.invalid".to_string(),
            "https://m2.invalid".to_string(),
        ]
    }

    fn packument_urls2() -> Vec<String> {
        vec![PK1.to_string(), PK2.to_string()]
    }

    #[test]
    fn packument_chain_skips_bad_json_and_uses_next_mirror() {
        let http = FakeHttp {
            text: vec![
                ("m1.invalid", Ok("<html>502 Bad Gateway</html>")),
                ("m2.invalid", Ok(GOOD_PACKUMENT)),
            ],
            ..Default::default()
        };
        let v = fetch_packument_with(&packument_urls2(), &http).unwrap();
        assert_eq!(v["dist-tags"]["latest"], "0.16.1");
        assert_eq!(
            http.calls(),
            packument_urls2(),
            "按链序请求：坏 JSON 不终止链，继续下一镜像"
        );
    }

    #[test]
    fn packument_chain_skips_transport_error_and_uses_next_mirror() {
        let http = FakeHttp {
            text: vec![
                ("m1.invalid", Err("连接超时")),
                ("m2.invalid", Ok(GOOD_PACKUMENT)),
            ],
            ..Default::default()
        };
        assert!(fetch_packument_with(&packument_urls2(), &http).is_ok());
        assert_eq!(http.calls().len(), 2, "传输失败同样不终止链");
    }

    #[test]
    fn packument_chain_all_mirrors_fail_reports_last_error() {
        let http = FakeHttp {
            text: vec![
                ("m1.invalid", Err("第一镜像挂了")),
                ("m2.invalid", Err("第二镜像挂了")),
            ],
            ..Default::default()
        };
        let err = fetch_packument_with(&packument_urls2(), &http).unwrap_err();
        assert!(
            err.to_string().contains("第二镜像挂了"),
            "全失败应报最后一个错误（用户看到的应是最终原因），实测：{err}"
        );
    }

    #[test]
    fn npm_versions_falls_through_shape_mismatch_to_next_mirror() {
        let http = FakeHttp {
            text: vec![
                ("m1.invalid", Ok(r#"{"versions":{}}"#)),
                ("m2.invalid", Ok(GOOD_PACKUMENT)),
            ],
            ..Default::default()
        };
        let (latest, versions) =
            npm_packument_versions_with(&bases2(), &http, "@deepseek-ai/dsh").unwrap();
        assert_eq!(latest, "0.16.1");
        assert_eq!(versions, vec!["0.16.1"]);
        assert_eq!(
            http.calls()[0],
            "https://m1.invalid/@deepseek-ai%2Fdsh",
            "scoped 包的 `/` 必须编码为 %2F（与 npm CLI 一致）"
        );
    }

    #[test]
    fn npm_versions_all_mirrors_shape_mismatch_reports_shape_error() {
        let http = FakeHttp {
            text: vec![
                ("m1.invalid", Ok(r#"{"versions":{}}"#)),
                ("m2.invalid", Ok("not json")),
            ],
            ..Default::default()
        };
        let err = npm_packument_versions_with(&bases2(), &http, "dsh-x").unwrap_err();
        assert!(err.contains("packument 形状不符"), "实测：{err}");
        assert!(err.contains("m2.invalid"), "实测：{err}");
    }

    #[test]
    fn npm_versions_rejects_illegal_package_without_touching_network() {
        let http = FakeHttp::default();
        assert_eq!(
            npm_packument_versions_with(&bases2(), &http, "-x").unwrap_err(),
            "包名非法"
        );
        assert_eq!(
            npm_packument_versions_with(&bases2(), &http, "").unwrap_err(),
            "包名非法"
        );
        assert!(http.calls().is_empty(), "非法包名不得发起任何请求");
    }

    #[test]
    fn market_registry_falls_back_to_github_raw() {
        let http = FakeHttp {
            text: vec![
                ("awesome-dsh-plugin.com", Err("CDN 502")),
                ("raw.githubusercontent.com", Ok(r#"{"plugins":[]}"#)),
            ],
            ..Default::default()
        };
        assert_eq!(
            fetch_market_registry_with(&http).unwrap(),
            r#"{"plugins":[]}"#
        );
        assert_eq!(http.calls().len(), 2);
    }

    #[test]
    fn market_registry_all_mirrors_fail_reports_last_error() {
        let http = FakeHttp {
            text: vec![
                ("awesome-dsh-plugin.com", Err("CDN 502")),
                ("raw.githubusercontent.com", Err("raw 404")),
            ],
            ..Default::default()
        };
        let err = fetch_market_registry_with(&http).unwrap_err();
        assert!(err.contains("raw 404"), "实测：{err}");
    }

    /// 造一个只含 `package/map.json` + `package/map.json.sig` 的 npm tarball。
    fn node_map_tgz(map: &[u8], sig: &str) -> Vec<u8> {
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut builder = tar::Builder::new(&mut gz);
        for (path, data) in [
            ("package/map.json", map),
            ("package/map.json.sig", sig.as_bytes()),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(data.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append_data(&mut header, path, data).unwrap();
        }
        drop(builder);
        gz.finish().unwrap()
    }

    #[test]
    fn node_map_follows_packument_then_tarball() {
        let tgz = node_map_tgz(b"{\"node\":{}}", "sig-hex");
        let packument = r#"{"dist-tags":{"latest":"1.2.3"},
            "versions":{"1.2.3":{"dist":{"tarball":"https://cdn.invalid/node-map-1.2.3.tgz"}}}}"#;
        let http = FakeHttp {
            text: vec![("m1.invalid", Ok(packument))],
            bytes: vec![("node-map-1.2.3.tgz", tgz)],
            ..Default::default()
        };
        let (map, sig) = fetch_node_map_with(&bases2(), &http).unwrap();
        assert_eq!(map, b"{\"node\":{}}");
        assert_eq!(sig, "sig-hex");
        assert_eq!(
            http.calls(),
            vec![
                NM_PK1.to_string(),
                "https://cdn.invalid/node-map-1.2.3.tgz".to_string()
            ],
            "首个镜像成功即停；packument → tarball 两步走，且包名按 %2F 编码"
        );
    }

    #[test]
    fn node_map_falls_back_to_next_registry_base() {
        let tgz = node_map_tgz(b"{}", "sig");
        let packument = r#"{"dist-tags":{"latest":"1.0.0"},
            "versions":{"1.0.0":{"dist":{"tarball":"https://cdn.invalid/x.tgz"}}}}"#;
        let http = FakeHttp {
            text: vec![("m1.invalid", Err("502")), ("m2.invalid", Ok(packument))],
            bytes: vec![("x.tgz", tgz)],
            ..Default::default()
        };
        assert!(fetch_node_map_with(&bases2(), &http).is_some());
        assert_eq!(http.calls()[0], NM_PK1, "先试首镜像");
        assert!(
            http.calls()[1].contains("m2.invalid"),
            "首镜像失败后回落次镜像，实测：{:?}",
            http.calls()
        );
    }

    #[test]
    fn node_map_all_mirrors_fail_is_none() {
        let http = FakeHttp {
            text: vec![("m1.invalid", Err("502")), ("m2.invalid", Err("503"))],
            ..Default::default()
        };
        assert!(fetch_node_map_with(&bases2(), &http).is_none());
    }

    /// **v1.1.0 Windows 实测 1.1 的回归闸门**：node-map 的版本是带 `v` 的，
    /// 展示层必须先归一，否则用户看到 `node vv24.18.0`。
    #[test]
    fn display_version_strips_single_v_prefix() {
        assert_eq!(display_version("v24.18.0"), "24.18.0");
        assert_eq!(display_version("24.18.0"), "24.18.0", "无前缀时原样");
        // 只剥一个：`vv` 是脏数据，不该被"修好"成版本号而掩盖问题
        assert_eq!(display_version("vv24.18.0"), "v24.18.0");
        assert_eq!(display_version(""), "");
        // 与 readiness_gaps 的口径一致（那里也剥 v 再比较）：真实 node-map 版本
        // 必须被归一成不带 v 的形态。
        let plan = node_plan(&std::env::temp_dir());
        assert_eq!(
            display_version(&plan.version),
            plan.version.trim_start_matches('v')
        );
        assert!(!display_version(&plan.version).starts_with('v'));
    }
}
