//! product.manifest.json —— 壳与「产品配置/宿主解析」之间的**唯一运行时契约**。
//!
//! v3（2026-09-04，ADR-0010 引擎倒置落地，见 docs/contract.md「运行时策略 v3」）：
//! 引擎档**缺省**——manifest 不声明 snapshot 三件套即引擎启动（壳自管
//! node/pnpm/dsh，pnpm12 引导器随壳内置）；声明 snapshot 三件套则快照档
//! （内置只读快照，离线可用，语义沿 v1 fallback）。v2 的 resolution/fallback
//! 语义废止（兼容读取迁移：fallback → 快照档；极简在线档 → 引擎档）。
//!
//! 加载后规范化为统一形态：`tiers ∈ {[Engine], [Bundle]}` + 可选 fallback——
//! 解析器（resolve_launch）只消费规范化形态，不感知来源版本。
//!
//! 产品名称 / 图标 / 标识符是**构建期身份**（render-product.sh 注入 tauri.conf.json），
//! 不在本契约里。路径解析：开发态 = `src-tauri/resources/`，发布态 = bundle 资源目录。

use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 当前契约版本（v3：引擎档缺省）。
pub const MANIFEST_FORMAT: u32 = 3;
/// 兼容读取的下限：v1 / v2 文件将被迁移加载（文档：docs/contract.md 契约改动流程）。
pub const MANIFEST_MIN_COMPAT: u32 = 1;

/// 解析档位：宿主解析链的一级（docs/contract.md「运行时策略」）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TierKind {
    /// 引擎档（v3 缺省）：壳自管 node/pnpm/dsh，boot 期幂等引导（ADR-0010）。
    Engine,
    /// 内置兜底（bundle 内 offline 副本；快照档语义）。
    Bundle,
}

/// 单个件的解析档（v3 规范化产物）：`tiers` 不从文件反序列化——加载时由
/// normalize 统一写定 [Engine] / [Bundle]，v1/v2 旧档序（system/download）
/// 在解析期即被忽略（skip）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TierSpec {
    #[serde(skip_deserializing, default)]
    pub tiers: Vec<TierKind>,
}

/// terminal 区块：终端行为（webUi profile 选择器，见 docs/contract.md manifest v2）。
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

/// 解析策略集合：与 dsh 成对判定（借执行器成对，见 docs/contract.md「运行时策略」）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionSpec {
    #[serde(default)]
    pub node: TierSpec,
    #[serde(default)]
    pub dsh: TierSpec,
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

/// v3 runtime 区块：引擎档缺省。目前仅定义 mode = "engine"（省略同义）——
/// 预留前向字段，未知值拒绝（不静默吞）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSpec {
    #[serde(default)]
    pub mode: Option<String>,
}

/// product.manifest.json 根结构。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductManifest {
    /// 契约版本（1 = v1 兼容，2 = v2 兼容，3 = 当前引擎档缺省）。
    pub format: u32,
    /// 人类可读产品名（展示用）。
    pub product_name: String,
    /// 终端行为与宿主解析策略（v2；v3 仅消费 default_profile）。
    #[serde(default)]
    pub terminal: TerminalSpec,
    /// v3 运行时区块（引擎档缺省）。
    #[serde(default)]
    pub runtime: Option<RuntimeSpec>,
    /// 离线兜底副本（内置档）。v3 由 snapshot 三件套规范化而来。
    #[serde(default)]
    pub fallback: Option<FallbackSpec>,
    /// v1 遗留 / v3 快照档：snapshot 三件套。加载时规范化为 fallback。
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
    /// 从 JSON 加载并规范化到统一形态（tiers ∈ {[Engine], [Bundle]} + fallback），
    /// 解析器不感知来源版本。
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)?;
        let mut manifest: Self = serde_json::from_str(&text)?;
        match manifest.format {
            MANIFEST_FORMAT => {
                manifest.normalize_v3()?;
                Ok(manifest)
            }
            2 => {
                // v2 兼容迁移：fallback（内置档）→ 快照档；极简在线档 → 引擎档
                //（resolution 档序语义废止——在线补齐由引擎引导承接，ADR-0010）。
                if manifest.fallback.is_some() {
                    manifest.set_bundle_tiers();
                } else {
                    manifest.set_engine_tiers();
                }
                tracing::warn!("product.manifest v2 已迁移加载（v3 引擎档语义）");
                Ok(manifest)
            }
            1 => {
                // v1 迁移：snapshot 三件套 → fallback + 快照档（v1 语义=固定内置）。
                let snap = manifest.snapshot.take().ok_or_else(|| {
                    anyhow::anyhow!("v1 契约缺失 snapshot 字段，无法迁移")
                })?;
                manifest.fallback = Some(FallbackSpec {
                    node_bin: snap.node_bin,
                    dsh_bin_js: snap.dsh_bin_js,
                    dsh_home: snap.dsh_home,
                    profile: snap.profile,
                });
                manifest.set_bundle_tiers();
                tracing::warn!("product.manifest v1 已迁移加载（快照档语义）");
                Ok(manifest)
            }
            other => anyhow::bail!(
                "product.manifest.json format 不兼容：文件为 {other}，壳支持 {MANIFEST_MIN_COMPAT}–{MANIFEST_FORMAT}。\
                 该桌面版由旧版本启动器打包，请重新打包后安装。"
            ),
        }
    }

    /// v3 规范化：快照档（snapshot 三件套）→ fallback + Bundle 档；否则引擎档。
    fn normalize_v3(&mut self) -> Result<()> {
        if let Some(rt) = &self.runtime {
            if let Some(mode) = &rt.mode {
                if mode != "engine" {
                    anyhow::bail!("runtime.mode 不支持：{mode}（当前仅 engine，引擎档为缺省）");
                }
            }
        }
        if let Some(snap) = self.snapshot.take() {
            self.fallback = Some(FallbackSpec {
                node_bin: snap.node_bin,
                dsh_bin_js: snap.dsh_bin_js,
                dsh_home: snap.dsh_home,
                profile: snap.profile,
            });
            self.set_bundle_tiers();
        } else {
            self.set_engine_tiers();
        }
        self.validate_fallback_consistency()
    }

    /// 快照档统一形态：node/dsh 档序 = [Bundle]（fallback 必在，由迁移保证）。
    fn set_bundle_tiers(&mut self) {
        for spec in [
            &mut self.terminal.resolution.node,
            &mut self.terminal.resolution.dsh,
        ] {
            spec.tiers = vec![TierKind::Bundle];
        }
    }

    /// 引擎档统一形态：node/dsh 档序 = [Engine]（v3 缺省）。
    fn set_engine_tiers(&mut self) {
        for spec in [
            &mut self.terminal.resolution.node,
            &mut self.terminal.resolution.dsh,
        ] {
            spec.tiers = vec![TierKind::Engine];
        }
    }

    /// 自洽性校验：解析档含 bundle 必须有 fallback 副本。
    fn validate_fallback_consistency(&self) -> Result<()> {
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
        let dir = std::env::temp_dir().join(format!("dsh-shell-man-{}-{seq}", std::process::id()));
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
    fn loads_v3_engine_minimal() {
        // 引擎档缺省：不声明 snapshot 三件套 → [Engine]，无 fallback。
        let (path, _g) = write_temp(r#"{"format": 3, "productName": "T"}"#);
        let m = ProductManifest::load(&path).unwrap();
        assert_eq!(m.format, 3);
        assert_eq!(m.terminal.default_profile, "web");
        assert_eq!(m.terminal.resolution.dsh.tiers, vec![TierKind::Engine]);
        assert_eq!(m.terminal.resolution.node.tiers, vec![TierKind::Engine]);
        assert!(m.fallback.is_none());
        // runtime.mode 显式声明 engine 与省略同义
        let (path, _g) =
            write_temp(r#"{"format": 3, "productName": "T", "runtime": {"mode": "engine"}}"#);
        let m = ProductManifest::load(&path).unwrap();
        assert_eq!(m.terminal.resolution.dsh.tiers, vec![TierKind::Engine]);
    }

    #[test]
    fn loads_v3_snapshot_as_bundle_tier() {
        // 快照档：snapshot 三件套 → 规范化为 fallback + [Bundle]（离线快照语义）。
        let json = r#"{
          "format": 3,
          "productName": "Bundled",
          "snapshot": {
            "nodeBin": "dsh-snapshot/node/bin/dsh-node",
            "dshBinJs": "dsh-snapshot/dsh/@deepseek-ai/dsh/lib/bin.js",
            "dshHome": "dsh-snapshot/home",
            "profile": "desktop-demo"
          }
        }"#;
        let (path, _g) = write_temp(json);
        let m = ProductManifest::load(&path).unwrap();
        assert_eq!(m.terminal.resolution.dsh.tiers, vec![TierKind::Bundle]);
        assert_eq!(m.terminal.resolution.node.tiers, vec![TierKind::Bundle]);
        let fb = m.fallback.expect("snapshot 应规范化为 fallback");
        assert_eq!(fb.profile, "desktop-demo");
        let expected = Path::new("/res").join("dsh-snapshot/dsh/@deepseek-ai/dsh/lib/bin.js");
        assert_eq!(fb.resolve_path(Path::new("/res"), &fb.dsh_bin_js), expected);
    }

    #[test]
    fn rejects_unknown_runtime_mode() {
        // 未知 runtime.mode 拒绝（不静默吞前向字段）
        let (path, _g) =
            write_temp(r#"{"format": 3, "productName": "T", "runtime": {"mode": "ghost"}}"#);
        let err = ProductManifest::load(&path).unwrap_err().to_string();
        assert!(err.contains("runtime.mode"), "{err}");
    }

    #[test]
    fn legacy_v2_fallback_migrates_to_snapshot_tier() {
        // v2 内置档（bundle tier + fallback）→ 快照档；声明 resolution 被忽略
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
        assert_eq!(m.terminal.default_profile, "web");
        assert_eq!(
            m.terminal.resolution.dsh.tiers,
            vec![TierKind::Bundle],
            "v2 内置档迁移为快照档（resolution 档序语义废止）"
        );
        assert_eq!(m.fallback.unwrap().profile, "desktop-demo");
    }

    #[test]
    fn legacy_v2_minimal_migrates_to_engine_tier() {
        // v2 极简在线档（system→download）→ 引擎档（在线补齐由引擎引导承接）
        let (path, _g) = write_temp(r#"{"format": 2, "productName": "T"}"#);
        let m = ProductManifest::load(&path).unwrap();
        assert_eq!(m.terminal.default_profile, "web");
        assert_eq!(m.terminal.resolution.dsh.tiers, vec![TierKind::Engine]);
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
}
