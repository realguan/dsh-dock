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
///
/// **2026-09-11 修复（F1，复现先行）**：口径为**字符数**而非字节数。
/// 旧实现用字节下标切片（`&trimmed[..4]` / `&trimmed[len-4..]`），值含非 ASCII
/// （CJK / 全角 / emoji 等任何多字节 UTF-8）时，字节 4 或 `len-4` 落在字符中间 →
/// `panic: byte index is not a char boundary`；而 `Cargo.toml:52` release 剖面
/// `panic = "abort"` ⇒ **不是返回错误而是进程被杀**，且调用点全在
/// `get_credentials_summary` 路径上（凭据面板加载即中招）。
/// 长度判据同改字符数：旧 `len() <= 8` 让「3 个 CJK 字符 = 9 字节」误入切片分支。
/// 掩码语义 = 前 4 字符 + `••••••••` + 后 4 字符；字符数 ≤ 8 时整串掩掉。
pub fn mask_api_key(key: &str) -> String {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= 8 {
        return "••••••••".to_string();
    }
    let prefix: String = chars[..4].iter().collect();
    let suffix: String = chars[chars.len() - 4..].iter().collect();
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

/// 解析凭据文件并生成脱敏摘要列表（纯函数，宿主/客体共用）
pub fn parse_credentials_summary(raw: &str) -> Vec<CredentialSummaryItem> {
    let parsed: BTreeMap<String, serde_json::Value> = if raw.trim().is_empty() {
        BTreeMap::new()
    } else {
        serde_yaml::from_str(raw).unwrap_or_default()
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

    result
}

/// 解析凭据文件并生成脱敏摘要列表
pub fn get_credentials_summary(home: &Path) -> Result<Vec<CredentialSummaryItem>, String> {
    let raw = read_credentials(home)?;
    Ok(parse_credentials_summary(&raw))
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

/// 原文覆写入口（4.5）：**先备份**再原子写回（2026-09-08，U9——确认框已向用户
/// 承诺「原文件会先备份」，故备份失败即中止，不降级）。
pub fn overwrite_credentials(home: &Path, content: &str) -> Result<(), String> {
    crate::fs_backup::backup_before_overwrite(&home.join(".credentials.yaml"))?;
    write_credentials(home, content)
}

/// 纯变换（宿主/客体共用）：对凭据 YAML 文本修改单个 Provider 的 API Key 并序列化为新 YAML
pub fn apply_set_provider_key(raw: &str, provider: &str, key: &str) -> Result<String, String> {
    let mut parsed: BTreeMap<String, serde_json::Value> = if raw.trim().is_empty() {
        BTreeMap::new()
    } else {
        serde_yaml::from_str(raw).map_err(|e| format!("解析 .credentials.yaml 失败：{e}"))?
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

    serde_yaml::to_string(&parsed).map_err(|e| format!("序列化凭据失败：{e}"))
}

/// 针对单个 Provider 安全设置 API Key（原子写回 + 保持 0600 权限）
pub fn set_provider_key(home: &Path, provider: &str, key: &str) -> Result<(), String> {
    let raw = read_credentials(home)?;
    let serialized = apply_set_provider_key(&raw, provider, key)?;
    write_credentials(home, &serialized)
}

/// 读取客体 `.credentials.yaml` 原文（不存在返回空串）
pub fn get_credentials_raw_in_guest(distro: &str) -> Result<String, String> {
    let files = crate::guest::read_files(distro, &[".credentials.yaml".to_string()])?;
    Ok(files
        .into_iter()
        .find_map(|(p, c)| {
            if p.ends_with(".credentials.yaml") {
                c
            } else {
                None
            }
        })
        .unwrap_or_default())
}

/// 解析客体凭据文件并生成脱敏摘要列表
pub fn get_credentials_summary_in_guest(
    distro: &str,
) -> Result<Vec<CredentialSummaryItem>, String> {
    let raw = get_credentials_raw_in_guest(distro)?;
    Ok(parse_credentials_summary(&raw))
}

/// 覆写客体 `.credentials.yaml`（先留备份再原子写回，保持 0600 权限）
pub fn save_credentials_raw_in_guest(distro: &str, content: &str) -> Result<(), String> {
    crate::guest::backup_file(distro, ".credentials.yaml")?;
    crate::guest::write_home_files(
        distro,
        &[(".credentials.yaml".to_string(), content.to_string())],
    )
}

/// 针对客体单个 Provider 安全设置 API Key（原子写回 + 保持 0600 权限）
pub fn set_provider_key_in_guest(distro: &str, provider: &str, key: &str) -> Result<(), String> {
    let raw = get_credentials_raw_in_guest(distro)?;
    let serialized = apply_set_provider_key(&raw, provider, key)?;
    crate::guest::write_home_files(distro, &[(".credentials.yaml".to_string(), serialized)])
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

    /// F1 回归（2026-09-11 复现先行）：`mask_api_key` 曾用**字节**下标切片
    /// （`&trimmed[..4]` / `&trimmed[trimmed.len()-4..]`）——非 ASCII 值的字节 4
    /// 或 `len-4` 落在字符中间 → `panic: byte index is not a char boundary`。
    /// release 剖面 `panic = "abort"`（Cargo.toml:52）下不是返回错误而是**进程被杀**，
    /// 而调用点在 `get_credentials_summary` 路径上 ⇒ 凭据面板加载即中招。
    ///
    /// 掩码语义裁定（本任务）：**按字符数**——前 4 字符 + `••••••••` + 后 4 字符；
    /// 字符数 ≤ 8 时整串掩掉（维持既有 `"••••••••"` 回退语义）。
    #[test]
    fn mask_api_key_handles_non_ascii_without_panic() {
        // ① 3 个 CJK 字符 = 9 **字节**（旧判据 `len() <= 8` 判否 → 走进切片分支），
        //    但字符数 3 ≤ 8 → 全掩码。
        assert_eq!(mask_api_key("凭据测"), "••••••••");
        // ② 前缀与后缀各自跨多字节边界：9 字符 = 5 CJK + 4 ASCII
        //    （旧实现 `&trimmed[..4]` 直接 panic：字节 4 落在「据」中间）。
        assert_eq!(mask_api_key("凭据测试值abcd"), "凭据测试••••••••abcd");
        // 前后缀都是多字节、总字符数 7 ≤ 8 → 全掩码（不得 panic）。
        assert_eq!(mask_api_key("密钥abc密钥"), "••••••••");
        // ③ 单字符多字节。
        assert_eq!(mask_api_key("密"), "••••••••");
        // 多字节超长：12 个 CJK 字符 → 前 4 + 掩码 + 后 4。
        assert_eq!(
            mask_api_key("凭据测试密钥对甲乙丙丁"),
            "凭据测试••••••••甲乙丙丁"
        );
        // 4 字节码位（emoji）：11 字符，前后缀都含 4 字节字符。
        assert_eq!(
            mask_api_key("🔑🔑a🔑🔑🔑🔑🔑🔑🔑🔑"),
            "🔑🔑a🔑••••••••🔑🔑🔑🔑"
        );
    }

    /// 边界自查（修复后口径写死，防回归）：
    /// - 空串 / 全空白 → `""`（trim 后为空，无值不显示掩码）；
    /// - 1–8 字符（含多字节）→ `"••••••••"`（不足 4+4 可披露，整串掩掉）；
    /// - 9 字符 → 前 4 + 掩码 + 后 4，ASCII 与多字节同口径；
    /// - ASCII 超长 → 与修复前逐字相同（见上方既有用例，不得改坏）。
    #[test]
    fn mask_api_key_boundaries_are_char_counted() {
        assert_eq!(mask_api_key(""), "");
        assert_eq!(mask_api_key("   "), "");
        assert_eq!(mask_api_key("12345678"), "••••••••");
        assert_eq!(mask_api_key("123456789"), "1234••••••••6789");
        assert_eq!(mask_api_key("密密密密密密密密"), "••••••••");
        assert_eq!(
            mask_api_key("密密密密密密密密密"),
            "密密密密••••••••密密密密"
        );
    }

    /// 端到端复现：真实调用路径（`get_credentials_summary` 的 refs 分支与顶层
    /// 自定义 provider 分支）拿到非 ASCII 秘密值时必须不 panic——这是本缺陷
    /// 从"函数边界"升级为"进程被杀"的那一跳。
    #[test]
    fn summary_with_non_ascii_secret_does_not_panic() {
        let tmp =
            std::env::temp_dir().join(format!("dsh-cred-test-nonascii-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(
            tmp.join(".credentials.yaml"),
            "refs:\n  DEEPSEEK_API_KEY: 凭据测试值abcd\ncustom-provider: 密钥abc密钥\n",
        )
        .unwrap();

        let summary = get_credentials_summary(&tmp).unwrap();
        let ds = summary.iter().find(|s| s.provider == "deepseek").unwrap();
        assert!(ds.configured);
        assert_eq!(ds.masked_key, "凭据测试••••••••abcd");
        let custom = summary
            .iter()
            .find(|s| s.provider == "custom-provider")
            .unwrap();
        assert!(custom.configured);
        assert_eq!(custom.masked_key, "••••••••");

        let _ = std::fs::remove_dir_all(&tmp);
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
    fn overwrite_credentials_backs_up_previous_content() {
        // 2026-09-08（U9）：原文覆写前留一份备份；凭据文件是用户唯一的密钥真相源，
        // 写残无法回退的代价最高。
        let tmp = std::env::temp_dir().join(format!("dsh-cred-bak-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        write_credentials(&tmp, "deepseek:\n  apiKey: sk-old\n").unwrap();
        overwrite_credentials(&tmp, "deepseek:\n  apiKey: sk-new\n").unwrap();

        assert_eq!(
            read_credentials(&tmp).unwrap(),
            "deepseek:\n  apiKey: sk-new\n"
        );
        let backups: Vec<String> = std::fs::read_dir(&tmp)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.starts_with(".credentials.yaml.bak-"))
            .collect();
        assert_eq!(backups.len(), 1, "应留一份备份：{backups:?}");
        assert_eq!(
            std::fs::read_to_string(tmp.join(&backups[0])).unwrap(),
            "deepseek:\n  apiKey: sk-old\n",
            "备份内容应为覆写前原文"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn overwrite_credentials_without_existing_file_skips_backup() {
        let tmp = std::env::temp_dir().join(format!("dsh-cred-nobak-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        overwrite_credentials(&tmp, "deepseek:\n  apiKey: sk-first\n").unwrap();

        let backups = std::fs::read_dir(&tmp)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains(".bak-"))
            .count();
        assert_eq!(backups, 0, "首次写入无原文件可备份");

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
