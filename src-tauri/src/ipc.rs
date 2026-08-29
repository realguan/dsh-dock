//! ipc.rs —— 应用自定义 IPC 命令的单一事实源（2026-08-28，4.3 工程前置闸门）。
//!
//! 三处消费与拦截关系：
//! - `build.rs` 经 `#[path]` 引入 [`COMMANDS`] 生成 AppManifest 的 allow-* 权限；
//!   capabilities 引用未知权限由 tauri-build 构建期拦截（tauri-utils ACL）。
//! - `capabilities/default.json` 的 `allow-*` 与 [`COMMANDS`] 一致性 → `gate_tests`。
//! - `lib.rs` 的 `generate_handler![…]` 是宏调用、需字面 token，无法由常量生成
//!   → 与 [`COMMANDS`] 一致性 → `gate_tests`（cargo test / CI 闸门）。
//!
//! 新增命令流程：① 本表登记 → ② AGENTS §7 登记 → ③ `lib.rs` handler +
//! capabilities allow-* 落地。漏任何一处会被构建或测试拦下，不再依赖人肉比对。

/// 全部应用自定义 IPC 命令（snake_case，与 `generate_handler!` 内标识符同形）。
pub const COMMANDS: &[&str] = &[
    "choose_profile",
    "terminal_action",
    "get_update_status",
    "check_updates",
    "get_client_update",
    "client_update_check",
    "client_update_apply",
    "open_external",
    "open_workbench_in_browser",
    "get_workbench_url",
    "boot_in_wsl",
    "choose_mode",
    "list_profiles",
    "get_profile_detail",
    "create_profile",
    "copy_profile",
    "rename_profile",
    "delete_profile",
    "set_default_profile",
    "get_default_profile",
    "switch_profile",
    "get_active_profile",
    "list_profile_plugins",
    "install_plugin",
    "remove_plugin",
    "update_plugin",
    "get_plugin_rows",
    "set_plugin_disabled",
    "check_plugin_updates",
    "list_plugin_versions",
    "get_plugin_runtime",
];

/// snake_case 命令名 → kebab-case 权限名（`choose_profile` → `choose-profile`）。
/// 与 tauri-build 由 AppManifest 命令生成 `allow-<name>` 的转换规则一致。
pub fn to_kebab(name: &str) -> String {
    name.replace('_', "-")
}

#[cfg(test)]
mod gate_tests {
    use super::*;
    use std::collections::BTreeSet;

    fn repo_file(rel: &str) -> String {
        std::fs::read_to_string(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel))
            .unwrap_or_else(|e| panic!("读取 {rel} 失败: {e}"))
    }

    /// 从 lib.rs 源文本提取 `generate_handler![ … ])` 块内的命令标识符。
    /// 格式漂移（找不到标记 / 块未闭合）直接 panic，提示更新本解析器。
    fn extract_handler_commands(lib_rs: &str) -> Vec<String> {
        const OPEN: &str = "generate_handler![";
        let start = lib_rs.find(OPEN).unwrap_or_else(|| {
            panic!("lib.rs 中找不到 generate_handler![——格式漂移，请更新 ipc.rs 解析器")
        });
        let body = &lib_rs[start + OPEN.len()..];
        let end = body
            .find(']')
            .unwrap_or_else(|| panic!("generate_handler![ 块未闭合——请更新 ipc.rs 解析器"));
        body[..end]
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    #[test]
    fn handler_matches_ipc_commands() {
        let lib_rs = repo_file("src/lib.rs");
        let extracted = extract_handler_commands(&lib_rs);
        let handler: BTreeSet<&str> = extracted.iter().map(String::as_str).collect();
        let declared: BTreeSet<&str> = COMMANDS.iter().copied().collect();

        let missing: Vec<_> = declared.difference(&handler).collect();
        assert!(
            missing.is_empty(),
            "命令 {missing:?} 已登记 ipc.rs 但 lib.rs generate_handler 缺失——漏 handler 即运行时静默失败"
        );
        let undeclared: Vec<_> = handler.difference(&declared).collect();
        assert!(
            undeclared.is_empty(),
            "命令 {undeclared:?} 在 lib.rs generate_handler 但未登记 ipc.rs——先登 ipc.rs 与 AGENTS §7 再落地"
        );
    }

    #[test]
    fn capabilities_match_ipc_commands() {
        let json: serde_json::Value = serde_json::from_str(&repo_file("capabilities/default.json"))
            .expect("capabilities/default.json 非法 JSON");
        let perms = json["permissions"]
            .as_array()
            .expect("capabilities/default.json 缺 permissions 数组");
        let granted: BTreeSet<String> = perms
            .iter()
            .filter_map(|p| p.as_str())
            .filter(|p| p.starts_with("allow-"))
            .map(|p| p.trim_start_matches("allow-").to_string())
            .collect();
        let declared: BTreeSet<String> = COMMANDS.iter().map(|c| to_kebab(c)).collect();

        let unauthorized: Vec<_> = declared.difference(&granted).collect();
        assert!(
            unauthorized.is_empty(),
            "命令 {unauthorized:?} 已登记但 capabilities/default.json 缺 allow-* 授权——remote 页面调用会被 ACL 静默拒绝"
        );
        let dangling: Vec<_> = granted.difference(&declared).collect();
        assert!(
            dangling.is_empty(),
            "capabilities/default.json 的 allow-* {dangling:?} 无对应命令——删除残留或补登记 ipc.rs"
        );
    }

    #[test]
    fn to_kebab_converts_snake() {
        assert_eq!(to_kebab("choose_profile"), "choose-profile");
        assert_eq!(
            to_kebab("open_workbench_in_browser"),
            "open-workbench-in-browser"
        );
        assert_eq!(to_kebab("boot_in_wsl"), "boot-in-wsl");
    }
}
