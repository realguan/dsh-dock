//! product.manifest.json —— 壳与「产品配置/宿主解析」之间的**唯一运行时契约**。
//!
//! v2（2026-08-21，grill 定稿，见 docs/contract.md「运行时策略」章）：
//! 本产物是 **dsh 的桌面终端**（ADR-0005）。契约从「快照三件套」扩展为
//! **终端 + 宿主解析策略**：
//!   - `terminal.resolution`：node / dsh 各自的解析档序（system → bundle → download）
//!     与版本下限；极简档无 bundle tier（随包不内置），内置档 bundle 优先（launcher 装配产物）。
//!   - `fallback`：离线兜底副本（内置档才有），相对 resources 根。
//!   - v1（format=1）兼容读取：snapshot 三件套迁移为 bundle-only 解析 + fallback。
//!
//! 产品名称 / 图标 / 标识符是**构建期身份**（render-product.sh 注入 tauri.conf.json），
//! 不在本契约里。路径解析：开发态 = `src-tauri/resources/`，发布态 = bundle 资源目录。

use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 当前契约版本（v2）。
pub const MANIFEST_FORMAT: u32 = 2;
/// 兼容读取的下限：v1 文件将被迁移加载（文档：docs/contract.md 契约改动流程）。
pub const MANIFEST_MIN_COMPAT: u32 = 1;

/// 解析档位：宿主解析链的一级（docs/contract.md「运行时策略」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TierKind {
    /// 用户环境复用（官方安装，过三重校验闸才成立）。
    System,
    /// 内置兜底（bundle 内 offline 副本；存在即优先——内置档语义）。
    Bundle,
    /// 实时下载（npm/registry 官方通道；网络动作）。
    Download,
}

/// 单个件的解析策略。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TierSpec {
    /// 解析次序；system 缺失/不达标即进下一档。
    #[serde(default = "default_tiers")]
    pub tiers: Vec<TierKind>,
    /// 版本下限（语义化区间左端，如 "0.1.0-rc.6"）；低于下限的 system 复用不成立。
    /// 主要用于 dsh；node 用 engines 校验。
    #[serde(default)]
    pub min_version: Option<String>,
    /// 复用 system 时任选 engines.node 校验（默认开）。
    #[serde(default = "default_true")]
    pub require_engines: bool,
}

fn default_tiers() -> Vec<TierKind> {
    vec![TierKind::System, TierKind::Download] // 极简档语义：无内置
}
fn default_true() -> bool {
    true
}

impl Default for TierSpec {
    fn default() -> Self {
        TierSpec {
            tiers: default_tiers(),
            min_version: None,
            require_engines: true,
        }
    }
}

/// terminal 区块：终端行为（ADR-0005 Q4：webUi profile 选择器）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSpec {
    /// 默认 boot 的 profile（官方 web profile 名）。
    #[serde(default = "default_web_profile")]
    pub default_profile: String,
    /// node / dsh 的宿主解析策略。
    #[serde(default)]
    pub resolution: ResolutionSpec,
}

fn default_web_profile() -> String {
    "web".to_string()
}

impl Default for TerminalSpec {
    fn default() -> Self {
        TerminalSpec {
            default_profile: default_web_profile(),
            resolution: ResolutionSpec::default(),
        }
    }
}

/// 解析策略集合：与 dsh 成对判定（借执行器成对，见 ADR-0005）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionSpec {
    #[serde(default)]
    pub node: TierSpec,
    #[serde(default)]
    pub dsh: TierSpec,
}

impl Default for ResolutionSpec {
    fn default() -> Self {
        ResolutionSpec {
            node: TierSpec::default(),
            dsh: TierSpec::default(),
        }
    }
}

/// 离线兜底副本（内置档才有）：v1 快照三件套的归宿，只读种子。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FallbackSpec {
    /// 相对 resources 根的 Node 可执行文件。
    pub node_bin: String,
    /// 相对 resources 根的 dsh 入口（`dsh/@deepseek-ai/dsh/lib/bin.js`）。
    pub dsh_bin_js: String,
    /// 相对 resources 根的虚拟 $DSH_HOME（内含 profiles/<profile>）。
    pub dsh_home: String,
    /// 兜底要 boot 的 profile 名。
    pub profile: String,
}

impl FallbackSpec {
    /// 把相对路径解析到 resources 根下。
    pub fn resolve_path(&self, resources_dir: &Path, rel: &str) -> std::path::PathBuf {
        resources_dir.join(rel)
    }
}

/// product.manifest.json 根结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductManifest {
    /// 契约版本（1 = v1 兼容，2 = 当前）。
    pub format: u32,
    /// 人类可读产品名（展示用）。
    pub product_name: String,
    /// 终端行为与宿主解析策略（v2）。
    #[serde(default)]
    pub terminal: TerminalSpec,
    /// 离线兜底副本（内置档）。
    #[serde(default)]
    pub fallback: Option<FallbackSpec>,
    /// v1 遗留字段：snapshot 三件套（format=1 文件）；v2 下应为空。
    #[serde(default)]
    pub snapshot: Option<SnapshotSpec>,
}

/// v1 遗留：快照三件套。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSpec {
    pub node_bin: String,
    pub dsh_bin_js: String,
    pub dsh_home: String,
    pub profile: String,
}

impl ProductManifest {
    /// 从 JSON 加载并规范化（v1 迁移 / v2 校验）。
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)?;
        let mut manifest: Self = serde_json::from_str(&text)?;
        match manifest.format {
            MANIFEST_FORMAT => {
                manifest.validate_v2()?;
                Ok(manifest)
            }
            1 => {
                // v1 迁移：snapshot 三件套 → fallback + bundle-only 解析（v1 语义=固定内置）。
                let snap = manifest.snapshot.take().ok_or_else(|| {
                    anyhow::anyhow!("v1 契约缺失 snapshot 字段，无法迁移")
                })?;
                manifest.fallback = Some(FallbackSpec {
                    node_bin: snap.node_bin,
                    dsh_bin_js: snap.dsh_bin_js,
                    dsh_home: snap.dsh_home,
                    profile: snap.profile,
                });
                for spec in [&mut manifest.terminal.resolution.node, &mut manifest.terminal.resolution.dsh] {
                    spec.tiers = vec![TierKind::Bundle];
                }
                tracing::warn!("product.manifest v1 已迁移加载（bundle-only 语义）");
                Ok(manifest)
            }
            other => anyhow::bail!(
                "product.manifest.json format 不兼容：文件为 {other}，壳支持 {MANIFEST_MIN_COMPAT}–{MANIFEST_FORMAT}。\
                 该桌面版由旧版本启动器打包，请重新打包后安装。"
            ),
        }
    }

    /// v2 自洽性校验：声明 bundle 档必须有 fallback。
    fn validate_v2(&self) -> Result<()> {
        let dsh_tiers = &self.terminal.resolution.dsh.tiers;
        if dsh_tiers.contains(&TierKind::Bundle) && self.fallback.is_none() {
            anyhow::bail!(
                "契约自洽性校验失败：dsh 解析档含 bundle，但缺少 fallback 副本（内置档必须在 pack 时顺势放入）"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_temp(json: &str) -> (std::path::PathBuf, TempGuard) {
        // 每个测试独立目录：cargo test 并行跑，共享路径会互相覆盖。
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "dsh-shell-man-{}-{seq}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("product.manifest.json");
        std::fs::write(&path, json).unwrap();
        (path, TempGuard { dir })
    }
    struct TempGuard {
        dir: std::path::PathBuf,
    }
    impl Drop for TempGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn loads_v2_terminal_manifest() {
        let json = r#"{
          "format": 2,
          "productName": "DSH Dock",
          "terminal": {
            "defaultProfile": "web",
            "resolution": {
              "node": { "tiers": ["bundle", "system", "download"], "requireEngines": true },
              "dsh": { "tiers": ["bundle", "system", "download"], "minVersion": "0.1.0-rc.6" }
            }
          },
          "fallback": {
            "nodeBin": "dsh-snapshot/node/bin/dsh-node",
            "dshBinJs": "dsh-snapshot/dsh/@deepseek-ai/dsh/lib/bin.js",
            "dshHome": "dsh-snapshot/home",
            "profile": "desktop-demo"
          }
        }"#;
        let (path, _g) = write_temp(json);
        let m = ProductManifest::load(&path).unwrap();
        assert_eq!(m.format, 2);
        assert_eq!(m.terminal.default_profile, "web");
        assert_eq!(
            m.terminal.resolution.node.tiers,
            vec![TierKind::Bundle, TierKind::System, TierKind::Download]
        );
        assert_eq!(
            m.terminal.resolution.dsh.min_version.as_deref(),
            Some("0.1.0-rc.6")
        );
        let fb = m.fallback.unwrap();
        assert_eq!(fb.profile, "desktop-demo");
        // 平台无关路径断言：期望值同样经 Path::join 构造（Windows 分隔符为 \）
        let expected = Path::new("/res").join("dsh-snapshot/dsh/@deepseek-ai/dsh/lib/bin.js");
        assert_eq!(fb.resolve_path(Path::new("/res"), &fb.dsh_bin_js), expected);
    }

    #[test]
    fn minimal_v2_defaults_to_minimal_tier() {
        // 极简档：不写 terminal/fallback 也能加载，默认 system→download、defaultProfile=web。
        let (path, _g) = write_temp(r#"{"format": 2, "productName": "T"}"#);
        let m = ProductManifest::load(&path).unwrap();
        assert_eq!(m.terminal.default_profile, "web");
        assert_eq!(m.terminal.resolution.dsh.tiers, vec![TierKind::System, TierKind::Download]);
        assert!(m.fallback.is_none());
    }

    #[test]
    fn legacy_v1_migrates_to_bundle_only() {
        let json = r#"{
          "format": 1,
          "productName": "Legacy",
          "snapshot": {
            "nodeBin": "dsh-snapshot/node/bin/dsh-node",
            "dshBinJs": "dsh-snapshot/dsh/@deepseek-ai/dsh/lib/bin.js",
            "dshHome": "dsh-snapshot/home",
            "profile": "default"
          }
        }"#;
        let (path, _g) = write_temp(json);
        let m = ProductManifest::load(&path).unwrap();
        assert_eq!(m.terminal.resolution.dsh.tiers, vec![TierKind::Bundle]);
        assert_eq!(m.terminal.resolution.node.tiers, vec![TierKind::Bundle]);
        assert_eq!(m.fallback.as_ref().unwrap().profile, "default");
    }

    #[test]
    fn rejects_unknown_format() {
        let (path, _g) = write_temp(r#"{"format": 99, "productName": "x"}"#);
        assert!(ProductManifest::load(&path).is_err());
    }

    #[test]
    fn rejects_bundle_tier_without_fallback() {
        let json = r#"{
          "format": 2,
          "productName": "x",
          "terminal": { "resolution": { "dsh": { "tiers": ["bundle"] } } }
        }"#;
        let (path, _g) = write_temp(json);
        assert!(ProductManifest::load(&path).is_err());
    }
}