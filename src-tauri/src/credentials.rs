//! credentials.rs —— `.credentials.yaml` 结构化安全凭据管理（4.5）。
//!
//! 约束与不变量（AGENTS §6 & roadmap §1）：
//! 1. 路径固定在 `$DSH_HOME/.credentials.yaml`；
//! 2. Unix 下严格维持 0o600 权限（仅当前用户可读写）；
//! 3. 顶层结构保持合规 YAML；
//! 4. 写入必须采用原子写（临时文件 + rename），防止意外断电写坏；
//! 5. 脱敏安全：读出时仅返回掩码（如 `sk-••••••••abcd`），前端禁止持有全量明文。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// 脱敏凭据摘要项（供前端安全展示）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CredentialSummaryItem {
    pub provider: String,
    pub label: String,
    pub configured: bool,
    pub masked_key: String,
}

/// 常用 Provider 元数据映射
const KNOWN_PROVIDERS: &[(&str, &str)] = &[
    ("deepseek", "DeepSeek"),
    ("openai", "OpenAI"),
    ("anthropic", "Anthropic (Claude)"),
    ("google", "Google Gemini"),
    ("moonshot", "Moonshot (Kimi)"),
    ("zhipu", "智谱 GLM"),
    ("groq", "Groq"),
    ("openrouter", "OpenRouter"),
];

/// 生成脱敏掩码（如 `sk-1234567890abcdef` → `sk-1•••••••cdef`）
pub fn mask_api_key(key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.len() <= 8 {
        return "••••••••".to_string();
    }
    let prefix = &trimmed[..4];
    let suffix = &trimmed[trimmed.len() - 4..];
    format!("{prefix}••••••••{suffix}")
}

/// 凭据文件保留元数据字段（非模型提供商）
const RESERVED_METADATA_KEYS: &[&str] = &[
    "version",
    "refs",
    "schema",
    "$schema",
    "_meta",
    "meta",
    "defaultProvider",
    "default_provider",
    "defaultModel",
    "default_model",
];

/// 获取 Provider 的别名/环境变量映射列表
fn provider_aliases(id: &str) -> Vec<String> {
    match id {
        "deepseek" => vec![
            "DEEPSEEK_API_KEY".into(),
            "deepseek_api_key".into(),
            "deepseek".into(),
            "DEEPSEEK".into(),
        ],
        "openai" => vec![
            "OPENAI_API_KEY".into(),
            "openai_api_key".into(),
            "openai".into(),
            "OPENAI".into(),
        ],
        "anthropic" => vec![
            "ANTHROPIC_API_KEY".into(),
            "CLAUDE_API_KEY".into(),
            "anthropic_api_key".into(),
            "anthropic".into(),
            "ANTHROPIC".into(),
        ],
        "google" => vec![
            "GEMINI_API_KEY".into(),
            "GOOGLE_API_KEY".into(),
            "google_api_key".into(),
            "gemini_api_key".into(),
            "google".into(),
            "gemini".into(),
        ],
        "moonshot" => vec![
            "MOONSHOT_API_KEY".into(),
            "KIMI_API_KEY".into(),
            "moonshot_api_key".into(),
            "kimi_api_key".into(),
            "moonshot".into(),
            "kimi".into(),
        ],
        "zhipu" => vec![
            "ZHIPU_API_KEY".into(),
            "ZHIPUAI_API_KEY".into(),
            "GLM_API_KEY".into(),
            "zhipu_api_key".into(),
            "glm_api_key".into(),
            "zhipu".into(),
            "glm".into(),
        ],
        "groq" => vec![
            "GROQ_API_KEY".into(),
            "groq_api_key".into(),
            "groq".into(),
            "GROQ".into(),
        ],
        "openrouter" => vec![
            "OPENROUTER_API_KEY".into(),
            "openrouter_api_key".into(),
            "openrouter".into(),
            "OPENROUTER".into(),
        ],
        other => vec![
            format!("{}_API_KEY", other.to_uppercase()),
            other.to_lowercase(),
            other.to_string(),
        ],
    }
}

/// 从 JSON/YAML Value 提取有效 API Key
fn extract_key_from_val(val: Option<&serde_json::Value>) -> Option<String> {
    match val {
        Some(serde_json::Value::String(s)) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        Some(serde_json::Value::Object(obj)) => {
            let k = obj
                .get("apiKey")
                .or_else(|| obj.get("api_key"))
                .or_else(|| obj.get("key"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if k.is_empty() {
                None
            } else {
                Some(k.to_string())
            }
        }
        _ => None,
    }
}

/// 读取凭据文件原文（不存在时返回空字符串）
pub fn read_credentials(home: &Path) -> Result<String, String> {
    let file = home.join(".credentials.yaml");
    if !file.is_file() {
        return Ok(String::new());
    }
    std::fs::read_to_string(&file).map_err(|e| format!("读取 .credentials.yaml 失败：{e}"))
}

/// 解析凭据文件并生成脱敏摘要列表
pub fn get_credentials_summary(home: &Path) -> Result<Vec<CredentialSummaryItem>, String> {
    let raw = read_credentials(home)?;
    let parsed: BTreeMap<String, serde_json::Value> = if raw.trim().is_empty() {
        BTreeMap::new()
    } else {
        serde_yaml::from_str(&raw).unwrap_or_default()
    };

    let mut result = Vec::new();
    let mut claimed_top_keys = std::collections::BTreeSet::new();
    let mut claimed_ref_keys = std::collections::BTreeSet::new();

    let refs_map = match parsed.get("refs") {
        Some(serde_json::Value::Object(map)) => Some(map),
        _ => None,
    };

    // 1. 先匹配知名 Provider
    for &(id, label) in KNOWN_PROVIDERS {
        let aliases = provider_aliases(id);
        let mut found_key = None;

        // 1.1 先查 refs
        if let Some(refs) = refs_map {
            for alias in &aliases {
                if let Some(val) = refs.get(alias) {
                    if let Some(k) = extract_key_from_val(Some(val)) {
                        found_key = Some(k);
                        claimed_ref_keys.insert(alias.clone());
                        break;
                    }
                }
            }
        }

        // 1.2 再查 top-level
        if found_key.is_none() {
            for alias in &aliases {
                if let Some(val) = parsed.get(alias) {
                    if let Some(k) = extract_key_from_val(Some(val)) {
                        found_key = Some(k);
                        claimed_top_keys.insert(alias.clone());
                        break;
                    }
                }
            }
        }

        let configured = found_key.is_some();
        let masked_key = found_key.as_deref().map(mask_api_key).unwrap_or_default();
        result.push(CredentialSummaryItem {
            provider: id.to_string(),
            label: label.to_string(),
            configured,
            masked_key,
        });
    }

    // 2. 补齐 refs 中用户自定义的额外 Provider
    if let Some(refs) = refs_map {
        for (k, v) in refs {
            if claimed_ref_keys.contains(k) || RESERVED_METADATA_KEYS.contains(&k.as_str()) {
                continue;
            }
            if let Some(key_str) = extract_key_from_val(Some(v)) {
                result.push(CredentialSummaryItem {
                    provider: k.clone(),
                    label: k.clone(),
                    configured: true,
                    masked_key: mask_api_key(&key_str),
                });
            }
        }
    }

    // 3. 补齐顶层用户自定义的额外 Provider（过滤保留元数据键与已认领键）
    for (k, v) in &parsed {
        if claimed_top_keys.contains(k) || RESERVED_METADATA_KEYS.contains(&k.as_str()) {
            continue;
        }
        if let Some(key_str) = extract_key_from_val(Some(v)) {
            result.push(CredentialSummaryItem {
                provider: k.clone(),
                label: k.clone(),
                configured: true,
                masked_key: mask_api_key(&key_str),
            });
        }
    }

    Ok(result)
}

/// 原子安全写入凭据文件（严格 0600 权限）
pub fn write_credentials(home: &Path, content: &str) -> Result<(), String> {
    if !home.is_dir() {
        std::fs::create_dir_all(home).map_err(|e| format!("创建 DSH_HOME 目录失败：{e}"))?;
    }

    let target = home.join(".credentials.yaml");
    let tmp = home.join(format!(".credentials.tmp.{}", std::process::id()));

    // 写入临时文件
    std::fs::write(&tmp, content).map_err(|e| format!("写入临时凭据文件失败：{e}"))?;

    // Unix 下设置 0600 权限
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        if let Err(e) = std::fs::set_permissions(&tmp, perms) {
            let _ = std::fs::remove_file(&tmp);
            return Err(format!("设置凭据文件 0600 权限失败：{e}"));
        }
    }

    // 原子覆盖目标文件
    if let Err(e) = std::fs::rename(&tmp, &target) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("重命名凭据文件失败：{e}"));
    }

    Ok(())
}

/// 针对单个 Provider 安全设置 API Key（原子写回 + 保持 0600 权限）
pub fn set_provider_key(home: &Path, provider: &str, key: &str) -> Result<(), String> {
    let raw = read_credentials(home)?;
    let mut parsed: BTreeMap<String, serde_json::Value> = if raw.trim().is_empty() {
        BTreeMap::new()
    } else {
        serde_yaml::from_str(&raw).map_err(|e| format!("解析 .credentials.yaml 失败：{e}"))?
    };

    let trimmed = key.trim();
    let aliases = provider_aliases(provider);

    // 如果包含 refs 对象：
    if let Some(serde_json::Value::Object(refs)) = parsed.get_mut("refs") {
        let existing_ref_key = aliases.iter().find(|a| refs.contains_key(*a)).cloned();
        let target_key =
            existing_ref_key.unwrap_or_else(|| format!("{}_API_KEY", provider.to_uppercase()));
        if trimmed.is_empty() {
            refs.remove(&target_key);
            for alias in &aliases {
                parsed.remove(alias);
            }
        } else {
            refs.insert(target_key, serde_json::Value::String(trimmed.to_string()));
        }
    } else {
        // 无 refs 对象时维护顶层结构
        if trimmed.is_empty() {
            for alias in &aliases {
                parsed.remove(alias);
            }
        } else if let Some(serde_json::Value::Object(map)) = parsed.get_mut(provider) {
            map.insert(
                "apiKey".to_string(),
                serde_json::Value::String(trimmed.to_string()),
            );
        } else {
            parsed.insert(
                provider.to_string(),
                serde_json::Value::String(trimmed.to_string()),
            );
        }
    }

    let serialized = serde_yaml::to_string(&parsed).map_err(|e| format!("序列化凭据失败：{e}"))?;
    write_credentials(home, &serialized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_api_key_handles_various_lengths() {
        assert_eq!(mask_api_key(""), "");
        assert_eq!(mask_api_key("123"), "••••••••");
        assert_eq!(mask_api_key("12345678"), "••••••••");
        assert_eq!(mask_api_key("123456789"), "1234••••••••6789");
        assert_eq!(mask_api_key("sk-1234567890abcdef"), "sk-1••••••••cdef");
        assert_eq!(
            mask_api_key("sk-ant-api03-abcdefghijklmn"),
            "sk-a••••••••klmn"
        );
    }

    #[test]
    fn read_credentials_returns_empty_when_missing() {
        let tmp = std::env::temp_dir().join(format!("dsh-cred-test-miss-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        let res = read_credentials(&tmp).unwrap();
        assert_eq!(res, "");
    }

    #[test]
    fn set_provider_key_and_summary_flow() {
        let tmp = std::env::temp_dir().join(format!("dsh-cred-test-flow-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        // 初始为空
        let summary = get_credentials_summary(&tmp).unwrap();
        let ds = summary.iter().find(|s| s.provider == "deepseek").unwrap();
        assert!(!ds.configured);

        // 设置 DeepSeek Key
        set_provider_key(&tmp, "deepseek", "sk-abcdef1234567890").unwrap();

        // 再次获取摘要
        let summary2 = get_credentials_summary(&tmp).unwrap();
        let ds2 = summary2.iter().find(|s| s.provider == "deepseek").unwrap();
        assert!(ds2.configured);
        assert_eq!(ds2.masked_key, "sk-a••••••••7890");

        // 设置 OpenAI Key
        set_provider_key(&tmp, "openai", "sk-proj-9876543210zyxwvu").unwrap();
        let summary3 = get_credentials_summary(&tmp).unwrap();
        let oai = summary3.iter().find(|s| s.provider == "openai").unwrap();
        assert!(oai.configured);
        assert_eq!(oai.masked_key, "sk-p••••••••xwvu");

        // 删除 / 清除 DeepSeek Key
        set_provider_key(&tmp, "deepseek", "").unwrap();
        let summary4 = get_credentials_summary(&tmp).unwrap();
        let ds4 = summary4.iter().find(|s| s.provider == "deepseek").unwrap();
        assert!(!ds4.configured);
        assert_eq!(ds4.masked_key, "");

        // OpenAI 依然保持
        let oai4 = summary4.iter().find(|s| s.provider == "openai").unwrap();
        assert!(oai4.configured);

        // 清理
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    #[cfg(unix)]
    fn write_credentials_enforces_0600_mode() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = std::env::temp_dir().join(format!("dsh-cred-test-perm-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        write_credentials(&tmp, "test: secret\n").unwrap();
        let cred_path = tmp.join(".credentials.yaml");
        let meta = std::fs::metadata(&cred_path).unwrap();
        let mode = meta.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "必须严格维持 0600 权限");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn metadata_keys_like_version_and_refs_are_excluded() {
        let tmp = std::env::temp_dir().join(format!("dsh-cred-test-meta-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let raw = r#"
version: 1
refs:
  foo: bar
defaultProvider: deepseek
deepseek:
  apiKey: sk-1234567890abcdef
custom_llm:
  apiKey: sk-custom987654321
"#;
        std::fs::write(tmp.join(".credentials.yaml"), raw).unwrap();

        let summary = get_credentials_summary(&tmp).unwrap();
        let providers: Vec<&str> = summary.iter().map(|s| s.provider.as_str()).collect();

        // 知名提供商与自定义提供商
        assert!(providers.contains(&"deepseek"));
        assert!(providers.contains(&"custom_llm"));

        // 元数据字段绝对不能被当作 Provider 展示
        assert!(!providers.contains(&"version"));
        assert!(!providers.contains(&"refs"));
        assert!(!providers.contains(&"defaultProvider"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn refs_env_key_parsing_and_setting_works() {
        let tmp = std::env::temp_dir().join(format!("dsh-cred-test-refs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let raw = r#"version: 1
refs:
  DEEPSEEK_API_KEY: sk-3refsenvtestokeya2a6
  COMMANDCODE_API_KEY: user_testtokenfaketokencdef
"#;
        std::fs::write(tmp.join(".credentials.yaml"), raw).unwrap();

        let summary = get_credentials_summary(&tmp).unwrap();
        let deepseek = summary.iter().find(|s| s.provider == "deepseek").unwrap();
        assert!(deepseek.configured);
        assert_eq!(deepseek.masked_key, "sk-3••••••••a2a6");

        let cmdcode = summary
            .iter()
            .find(|s| s.provider == "COMMANDCODE_API_KEY")
            .unwrap();
        assert!(cmdcode.configured);
        assert_eq!(cmdcode.masked_key, "user••••••••cdef");

        // 设置/更新 deepseek key
        set_provider_key(&tmp, "deepseek", "sk-newkey1234567890").unwrap();
        let summary2 = get_credentials_summary(&tmp).unwrap();
        let deepseek2 = summary2.iter().find(|s| s.provider == "deepseek").unwrap();
        assert!(deepseek2.configured);
        assert_eq!(deepseek2.masked_key, "sk-n••••••••7890");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
