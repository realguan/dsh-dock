//! resolve.rs —— 终端宿主解析链（ADR-0005 / docs/contract.md「运行时策略」）。
//!
//! 职责：按 manifest 的 resolution 档序（system → bundle → download）解析出
//! 本次启动要用的 `LaunchSpec`（node / dsh 入口 / DSH_HOME / profile）。
//!
//!   - **system**：探测用户官方安装（PATH → realpath → 包树），过三重校验闸
//!     （版本下限 / engines.node / 平台——system 树是就地安装的，平台天然一致）。
//!   - **bundle**：manifest.fallback（内置档兜底副本）。
//!   - **download**：v2 占位——由 updates 模块（③）实装，当前返回可行动文案。
//!
//! 借执行器、不借配置（Q2b）：system 命中时 DSH_HOME 指向**用户自身 home**
//! （$DSH_HOME 或 ~/.dsh），boot 的是用户 dsh 世界里的官方/自定义 profile。

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;

use crate::manifest::{FallbackSpec, ProductManifest, TierKind, TierSpec};

/// 解析后的启动规格：一次具体 spawn 的全部决定。
#[derive(Debug, Clone)]
pub struct LaunchSpec {
    pub node_bin: PathBuf,
    pub dsh_bin_js: PathBuf,
    pub dsh_home: PathBuf,
    pub profile: String,
    pub tier: TierKind,
}

// ---------- 用户 home ----------

/// 终端在 system 档 boot 用户世界：$DSH_HOME 或 ~/.dsh。
pub fn user_dsh_home() -> PathBuf {
    std::env::var_os("DSH_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".dsh")))
        .unwrap_or_else(|| PathBuf::from(".dsh"))
}

// ---------- 版本比较（移植自 dsh-launcher runtime.rs，含 rc 语义） ----------

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
        let long_seg = if ka.len() > kb.len() { &ka[kb.len()] } else { &kb[ka.len()] };
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

// ---------- system 探测 ----------

pub struct SystemDsh {
    /// dsh 入口：tree/lib/bin.js。
    pub bin_js: PathBuf,
    pub version: String,
    pub engines_node: Option<String>,
}

/// 在 PATH 上找官方安装的 dsh（npm/pnpm 全局）：`which dsh` → 解符号链 →
/// 逐级上溯找包根 → 读 package.json。找不到返回 None。
pub fn detect_system_dsh(path_env: &str) -> Option<SystemDsh> {
    let bin = path_dirs(path_env).into_iter().find_map(|dir| {
        let cand = dir.join("dsh");
        if cand.is_file() && is_executable(&cand) {
            Some(cand)
        } else {
            None
        }
    })?;
    let real = fs::canonicalize(&bin).ok()?;
    let tree = find_package_root(&real, "@deepseek-ai/dsh")?;
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

/// 从可执行文件路径逐级上溯，找到 name 匹配的 package.json 所在的包根。
fn find_package_root(start: &Path, name: &str) -> Option<PathBuf> {
    let mut dir = start.parent()?;
    for _ in 0..8 {
        let manifest = dir.join("package.json");
        if manifest.is_file() {
            if let Ok(text) = fs::read_to_string(&manifest) {
                if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&text) {
                    if pkg.get("name").and_then(|n| n.as_str()) == Some(name) {
                        return Some(dir.to_path_buf());
                    }
                }
            }
        }
        dir = dir.parent()?;
    }
    None
}

/// PATH 上的系统 node 及其版本（"--version"）。
pub struct SystemNode {
    pub bin: PathBuf,
    pub version: String,
}

pub fn detect_system_node(path_env: &str) -> Option<SystemNode> {
    let bin = path_dirs(path_env).into_iter().find_map(|dir| {
        let cand = dir.join("node");
        if cand.is_file() && is_executable(&cand) {
            Some(cand)
        } else {
            None
        }
    })?;
    let version = Command::new(&bin).arg("--version").output().ok()?;
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
    let Some(required) = req
        .split(['>', '=', '<', '~', '^', ' '])
        .find_map(|t| t.chars().next().filter(|c| c.is_ascii_digit()).and_then(|_| major_of(t)))
    else {
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
    TooOld { found: String, min: String },
}

/// 按档序解析本次启动的 LaunchSpec。
pub fn resolve_launch(
    manifest: &ProductManifest,
    resources_dir: &Path,
    path_env: &str,
    data_dir: &Path,
) -> Result<LaunchSpec> {
    let spec = &manifest.terminal.resolution.dsh;

    for tier in &spec.tiers {
        match tier {
            TierKind::System => {
                match probe_system(spec, path_env) {
                    SystemOutcome::Hit(hit) => {
                        return Ok(LaunchSpec {
                            node_bin: hit.node.bin,
                            dsh_bin_js: hit.dsh.bin_js,
                            dsh_home: user_dsh_home(),
                            profile: manifest.terminal.default_profile.clone(),
                            tier: TierKind::System,
                        });
                    }
                    SystemOutcome::TooOld { found, min } => {
                        anyhow::bail!(
                            "您机器上的 dsh 版本过低（{found} < 终端要求 {min}）。                             终端不会自动覆盖您的全局安装；请确认后执行 `npm i -g @deepseek-ai/dsh`                              升级，或安装内置档桌面版。"
                        );
                    }
                    SystemOutcome::Miss => {
                        tracing::info!("system 档未命中（用户环境无可用官方 dsh）");
                    }
                }
            }
            TierKind::Bundle => {
                let fb = manifest.fallback.clone().ok_or_else(|| {
                    anyhow::anyhow!("契约声明 bundle 档但缺少 fallback（自洽性校验应拦截，此属异常）")
                })?;
                // 快照 home 内是装配时固化的 profile：boot 它而不是 default_profile
                return Ok(launch_from_fallback(&fb, resources_dir, fb.profile.clone()));
            }
            TierKind::Download => {
                // 实时下载：node 执行器（系统优先，无则缓存下载）→ npm 全局装官方最新 dsh
                let (node, tree) = crate::updates::install_latest_global(data_dir)
                    .map_err(|e| anyhow::anyhow!("实时下载档失败：{e}"))?;
                return Ok(LaunchSpec {
                    node_bin: node,
                    dsh_bin_js: tree.join("lib").join("bin.js"),
                    dsh_home: user_dsh_home(),
                    profile: manifest.terminal.default_profile.clone(),
                    tier: TierKind::Download,
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
            return SystemOutcome::TooOld { found: dsh.version, min: min.clone() };
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

/// bundle 档：fallback 三件套（相对 resources 根）。
fn launch_from_fallback(fb: &FallbackSpec, resources_dir: &Path, profile: String) -> LaunchSpec {
    LaunchSpec {
        node_bin: fb.resolve_path(resources_dir, &fb.node_bin),
        dsh_bin_js: fb.resolve_path(resources_dir, &fb.dsh_bin_js),
        dsh_home: fb.resolve_path(resources_dir, &fb.dsh_home),
        profile,
        tier: TierKind::Bundle,
    }
}

// ---------- 工具 ----------

fn path_dirs(path_env: &str) -> Vec<PathBuf> {
    path_env
        .split(':')
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
        let Ok(text) = fs::read_to_string(&manifest) else { continue };
        let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let has_web =
            pkg.get("dsh")
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
    fn engines_gate_is_lenient() {
        assert!(engines_satisfied("v24.18.0", Some(">=22")));
        assert!(engines_satisfied("v20.0.0", Some(">=22")) == false);
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

    #[cfg(unix)]
    #[test]
    fn detects_system_dsh_via_fake_path() {
        use std::os::unix::fs::PermissionsExt;
        let root = tmp();
        let bindir = root.join("bin");
        let pkg = root.join("lib/node_modules/@deepseek-ai/dsh");
        std::fs::create_dir_all(&bindir).unwrap();
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::create_dir_all(&pkg.join("lib")).unwrap();
        std::fs::write(
            pkg.join("package.json"),
            r#"{"name": "@deepseek-ai/dsh", "version": "0.1.0-rc.7",
                "engines": {"node": ">=22"}}"#,
        )
        .unwrap();
        std::fs::write(&pkg.join("lib/bin.js"), "#!/usr/bin/env node\n// bin\n").unwrap();
        std::fs::set_permissions(&pkg.join("lib/bin.js"), fs::Permissions::from_mode(0o755)).unwrap();
        // npm 全局形态：bin 目录里的 dsh 是指向包内入口的符号链接
        std::os::unix::fs::symlink(&pkg.join("lib/bin.js"), bindir.join("dsh")).unwrap();

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
        let m: ProductManifest = serde_json::from_str(json).unwrap();
        let spec = resolve_launch(&m, &res, "", &root).unwrap();
        assert_eq!(spec.tier, TierKind::Bundle);
        assert_eq!(spec.profile, "desktop-demo");
        assert!(spec.node_bin.ends_with("dsh-snapshot/node/bin/dsh-node"));
    }

    #[test]
    fn resolve_empty_tiers_fails_with_message() {
        let json = r#"{"format": 2, "productName": "T",
          "terminal": {"resolution": {"dsh": {"tiers": []}}}}"#;
        let m: ProductManifest = serde_json::from_str(json).unwrap();
        let err = resolve_launch(&m, Path::new("/res"), "", Path::new("/tmp/none")).unwrap_err();
        assert!(err.to_string().contains("档序为空"));
    }
}