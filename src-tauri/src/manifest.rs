//! product.manifest.json —— 壳与快照之间的**唯一运行时契约**。
//!
//! 设计原则（ADR-0004）：壳是通用机制、产品是数据。本结构是壳运行时感知的
//! 全部「产品身份」：要 spawn 哪一个 node、哪一份 dsh 入口、哪套虚拟 $DSH_HOME、
//! boot 哪个 profile。产品名称 / 图标 / 标识符属于**构建期身份**，由
//! `scripts/render-product.sh` 在打包时打进 tauri.conf.json，不在本契约里。
//!
//! 快照路径全部是**相对 resources 根**的：开发态 = `src-tauri/resources/`，
//! 发布态 = 应用 bundle 内的资源目录，用 `app.path().resource_dir()` 统一解析。

use std::fs;
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 契约版本。不匹配即拒绝启动（错误就地呈现，见 AGENTS / ADR-0004 A6）。
pub const MANIFEST_FORMAT: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductManifest {
    /// 契约版本，须等于 MANIFEST_FORMAT。
    pub format: u32,
    /// 人类可读产品名（展示用；窗口标题在 tauri.conf.json，构建期写入）。
    pub product_name: String,
    /// 快照布局：各部件相对 resources 根的路径 + 要 boot 的 profile。
    pub snapshot: SnapshotSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSpec {
    /// 相对 resources 根的 Node 可执行文件（打包侧按平台放置 node / node.exe / 单二进制）。
    pub node_bin: String,
    /// 相对 resources 根的 dsh 入口（如 `dsh/node_modules/@deepseek-ai/dsh/lib/bin.js`）。
    pub dsh_bin_js: String,
    /// 相对 resources 根的虚拟 $DSH_HOME（内含 `profiles/<profile>` 与配置/插件）。
    pub dsh_home: String,
    /// 要 boot 的 profile 名。
    pub profile: String,
}

impl ProductManifest {
    /// 从 JSON 文件加载并校验契约版本。
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)?;
        let manifest: Self = serde_json::from_str(&text)?;
        if manifest.format != MANIFEST_FORMAT {
            anyhow::bail!(
                "product.manifest.json format 不兼容：文件为 {}，壳要求 {MANIFEST_FORMAT}。\
                 该桌面版由旧版本启动器打包，请重新打包后安装。",
                manifest.format
            );
        }
        Ok(manifest)
    }

    /// 把快照相对路径解析到 resources 根下。
    pub fn snapshot_path(&self, resources_dir: &Path, rel: &str) -> std::path::PathBuf {
        resources_dir.join(rel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_valid_v1_manifest() {
        let json = r#"{
          "format": 1,
          "productName": "DeepSeek Harness Desktop",
          "snapshot": {
            "nodeBin": "dsh-snapshot/node/bin/dsh-node",
            "dshBinJs": "dsh-snapshot/dsh/node_modules/@deepseek-ai/dsh/lib/bin.js",
            "dshHome": "dsh-snapshot/home",
            "profile": "default"
          }
        }"#;
        let dir = std::env::temp_dir().join(format!("dsh-shell-man-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("product.manifest.json");
        std::fs::write(&path, json).unwrap();
        let m = ProductManifest::load(&path).unwrap();
        assert_eq!(m.product_name, "DeepSeek Harness Desktop");
        assert_eq!(m.snapshot.profile, "default");
        assert_eq!(
            m.snapshot_path(std::path::Path::new("/res"), &m.snapshot.node_bin)
                .display()
                .to_string(),
            "/res/dsh-snapshot/node/bin/dsh-node"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_unknown_format() {
        let json = r#"{"format": 99, "productName": "x", "snapshot": {}}"#;
        let dir = std::env::temp_dir().join(format!("dsh-shell-man2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("product.manifest.json");
        std::fs::write(&path, json).unwrap();
        assert!(ProductManifest::load(&path).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
