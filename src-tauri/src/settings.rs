//! settings.rs —— 壳的最小持久化（AGENTS.md「无状态库」的登记例外）。
//!
//! 仅 `<app_data>/settings.json`、仅 `defaultMode` 一个字段（首次打开可选运行环境 +
//! 设置默认打开方式）；其余核心态一律不落盘。字段全可选：缺文件/损坏 → 默认值
//! （首次语义），绝不因配置文件问题阻塞启动。

use std::path::Path;

use serde::{Deserialize, Serialize};

/// 运行环境默认（壳侧的用户意图；与 executor::ExecutorKind / ExecutionMode 同构）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Local,
    Wsl,
}

impl Mode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Mode::Local => "local",
            Mode::Wsl => "wsl",
        }
    }

    pub fn parse(s: &str) -> Option<Mode> {
        match s {
            "local" => Some(Mode::Local),
            "wsl" => Some(Mode::Wsl),
            _ => None,
        }
    }
}

/// `<app_data>/settings.json` 内容。全部字段可选（缺省 = 从未设置，首次引导）。
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ShellSettings {
    /// 未设置（None）= 首次运行：启动页先出运行环境选择。
    pub default_mode: Option<Mode>,
    /// 默认启动 profile 名（4.3④；第二最小面例外，2026-08-28 落地，AGENTS §6
    /// 已登记）。None = 未设置——消费方按 `web` 兜底（存储值失效同兜底，
    /// ADR-0009 §4 定死回退值 `web`：模板名恒可首启）。
    pub default_profile: Option<String>,
    /// 壳界面语言偏好（4.13；None = 跟随操作系统语言；可设 "zh-CN", "en-US" 等）。
    pub locale: Option<String>,
    /// 崩溃自动拉起守护（4.12；None / false = 关闭；true = 启用，短时间多次崩溃自动熔断）。
    pub auto_restart: Option<bool>,
}

fn settings_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("settings.json")
}

/// 读设置：缺文件/解析失败 → 默认（首次语义），绝不因配置损坏阻塞启动。
pub fn load(data_dir: &Path) -> ShellSettings {
    std::fs::read_to_string(settings_path(data_dir))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// 写设置：先写临时文件再 rename（原子替换，避免半写状态）。
pub fn save(data_dir: &Path, settings: &ShellSettings) -> Result<(), String> {
    std::fs::create_dir_all(data_dir).map_err(|e| e.to_string())?;
    let path = settings_path(data_dir);
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> std::path::PathBuf {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let d = std::env::temp_dir().join(format!(
            "dsh-dock-settings-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn missing_file_loads_default() {
        let dir = tmp();
        assert_eq!(load(&dir), ShellSettings::default());
        assert_eq!(load(&dir).default_mode, None);
        assert_eq!(load(&dir).locale, None);
        assert_eq!(load(&dir).auto_restart, None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn corrupted_file_loads_default() {
        let dir = tmp();
        std::fs::write(dir.join("settings.json"), "{ not json").unwrap();
        assert_eq!(load(&dir), ShellSettings::default());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tmp();
        let s = ShellSettings {
            default_mode: Some(Mode::Wsl),
            default_profile: Some("custom-profile".to_string()),
            locale: Some("en-US".to_string()),
            auto_restart: Some(true),
        };
        save(&dir, &s).unwrap();
        assert_eq!(load(&dir), s);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn default_profile_roundtrips_and_old_format_still_parses() {
        let dir = tmp();
        let s = ShellSettings {
            default_mode: Some(Mode::Local),
            default_profile: Some("my-profile".to_string()),
            locale: None,
            auto_restart: None,
        };
        save(&dir, &s).unwrap();
        assert_eq!(load(&dir), s);
        // 旧格式（仅 defaultMode，字段加入前的存量文件）兼容：其余字段缺省 None
        let old: ShellSettings = serde_json::from_str(r#"{"defaultMode":"wsl"}"#).unwrap();
        assert_eq!(old.default_mode, Some(Mode::Wsl));
        assert_eq!(old.default_profile, None);
        assert_eq!(old.locale, None);
        assert_eq!(old.auto_restart, None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn mode_parse_and_label() {
        assert_eq!(Mode::parse("local"), Some(Mode::Local));
        assert_eq!(Mode::parse("wsl"), Some(Mode::Wsl));
        assert_eq!(Mode::parse("ssh"), None);
        assert_eq!(Mode::Local.as_str(), "local");
        assert_eq!(Mode::Wsl.as_str(), "wsl");
        // 非 default 的已存值也回读正确
        let text = r#"{"defaultMode":"wsl"}"#;
        let s: ShellSettings = serde_json::from_str(text).unwrap();
        assert_eq!(s.default_mode, Some(Mode::Wsl));
    }
}
