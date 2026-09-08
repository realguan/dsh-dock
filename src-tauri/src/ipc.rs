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
    "list_all_plugins",
    "copy_plugin_config",
    "set_profile_build_approvals",
    "list_sessions",
    "repair_session",
    "repair_all_sessions",
    "get_shell_settings",
    "set_shell_settings",
    "get_system_diagnostics",
    "get_app_logs",
    "get_credentials_raw",
    "save_credentials_raw",
    "get_credentials_summary",
    "set_credential_key",
    "get_dsh_settings_raw",
    "save_dsh_settings_raw",
    "list_mcp_servers",
    "save_mcp_server",
    "delete_mcp_server",
    "delete_session",
    "fetch_market_registry",
    "open_profiles_window",
    "focus_main_window",
    "get_boot_status",
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

    // ---------- 契约「形状」闸门（2026-09-08，P4）----------
    //
    // 此前闸门只覆盖**名字**（handler ↔ COMMANDS ↔ capabilities），**形状**无人管：
    // 前端 `lib/tauri.ts` 是第 5 个消费面，字段改名/换 casing 两边都不会红，
    // 只会「编译绿、运行错」。以下三个闸门把形状钉死。

    /// 去注释行后逐行扫描 TS 源（`//` / `*` / `/*` 开头一律跳过）。
    /// 与 `extract_handler_commands` 同口径：格式漂移即 panic，提示更新解析器。
    fn ts_code_lines(ts: &str) -> impl Iterator<Item = &str> {
        ts.lines().filter(|l| {
            let t = l.trim_start();
            !(t.starts_with("//") || t.starts_with('*') || t.starts_with("/*"))
        })
    }

    /// 从 `frontend/src/lib/tauri.ts` 提取 `invoke(...)` / `invoke<T>(...)` 的命令名。
    ///
    /// 非调用位置（如 `import { invoke } from "@tauri-apps/api/core"`、注释里的
    /// `invoke()`）按标识符边界跳过；**调用位置**格式异常才 panic——解析漏项会退化成
    /// 名集比对失败，不会静默放过。
    fn extract_tauri_invoke_names(ts: &str) -> Vec<String> {
        const KW: &str = "invoke";
        let mut out = Vec::new();
        for line in ts_code_lines(ts) {
            let mut rest = line;
            while let Some(pos) = rest.find(KW) {
                let before = rest[..pos].chars().next_back();
                let after_kw = &rest[pos + KW.len()..];
                rest = after_kw;
                // 标识符边界：前面是字母/数字/下划线/点 → 不是本关键字
                if before.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '.') {
                    continue;
                }
                let head = after_kw.trim_start();
                if !(head.starts_with('(') || head.starts_with('<')) {
                    continue; // 非调用位置（import / 注释等）
                }
                // 可选泛型实参：按深度跳过成对尖括号
                let after_generic = if let Some(g) = head.strip_prefix('<') {
                    let mut depth = 1usize;
                    let mut cut = None;
                    for (i, ch) in g.char_indices() {
                        match ch {
                            '<' => depth += 1,
                            '>' => {
                                depth -= 1;
                                if depth == 0 {
                                    cut = Some(i + 1);
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    let cut = cut.unwrap_or_else(|| {
                        panic!("tauri.ts 的 invoke 泛型实参未闭合——格式漂移，请更新 ipc.rs 解析器")
                    });
                    &g[cut..]
                } else {
                    head
                };
                let args = after_generic
                    .trim_start()
                    .strip_prefix('(')
                    .unwrap_or_else(|| {
                        panic!("tauri.ts 的 invoke 调用格式漂移（缺左括号）——请更新 ipc.rs 解析器")
                    })
                    .trim_start();
                let quoted = args.strip_prefix('"').unwrap_or_else(|| {
                    panic!("tauri.ts 的 invoke 首参不是字符串字面量——请更新 ipc.rs 解析器")
                });
                let end = quoted
                    .find('"')
                    .unwrap_or_else(|| panic!("tauri.ts 的 invoke 命令名未闭合"));
                out.push(quoted[..end].to_string());
                rest = &quoted[end..];
            }
        }
        out
    }

    /// 递归收集 `frontend/src` 下全部 .ts/.tsx（不含 lib/tauri.ts）。
    fn frontend_source_files() -> Vec<std::path::PathBuf> {
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("读 {dir:?} 失败: {e}"));
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if matches!(
                    path.extension().and_then(|e| e.to_str()),
                    Some("ts") | Some("tsx")
                ) {
                    out.push(path);
                }
            }
        }
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/src");
        let mut out = Vec::new();
        walk(&root, &mut out);
        out
    }

    /// 闸门 1：`tauri.ts` 的 invoke 名集必须与 [`COMMANDS`] 逐字一致。
    #[test]
    fn tauri_ts_matches_ipc_commands() {
        let ts = repo_file("../frontend/src/lib/tauri.ts");
        let invoked: BTreeSet<String> = extract_tauri_invoke_names(&ts).into_iter().collect();
        let declared: BTreeSet<String> = COMMANDS.iter().map(|c| c.to_string()).collect();

        let missing: Vec<_> = declared.difference(&invoked).collect();
        assert!(
            missing.is_empty(),
            "命令 {missing:?} 已登记 ipc.rs 但 tauri.ts 未封装——前端第 5 消费面漏了（AGENTS §4.3：组件不直接 invoke）"
        );
        let extra: Vec<_> = invoked.difference(&declared).collect();
        assert!(
            extra.is_empty(),
            "tauri.ts 调用了未登记命令 {extra:?}——先登 ipc.rs 与 AGENTS §7"
        );
    }

    /// 闸门 2：`invoke` 只允许出现在 `lib/tauri.ts`（AGENTS §4.3 组件内不直接 invoke）。
    #[test]
    fn no_direct_invoke_outside_tauri_ts() {
        let mut offenders = Vec::new();
        for path in frontend_source_files() {
            if path.ends_with("lib/tauri.ts") {
                continue;
            }
            let src = std::fs::read_to_string(&path).expect("读前端源文件失败");
            for (i, line) in ts_code_lines(&src).enumerate() {
                if line.contains("invoke(") || line.contains("invoke<") {
                    offenders.push(format!("{}:{}", path.display(), i + 1));
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "以下位置直接调用 invoke——IPC 必须经 lib/tauri.ts 的 api 对象：{offenders:?}"
        );
    }

    // ---------- 契约「形状」：跨语言 key 集 fixture ----------

    /// 与前端共享的形状契约：`frontend/src/types/ipc-shapes.json`。
    /// Rust 侧断言「真实 serde 序列化的 key 集」== fixture；
    /// 前端侧断言「TS 接口 key 集」== fixture。任一侧改名/换 casing 都会红。
    const IPC_SHAPES_JSON: &str = include_str!("../../frontend/src/types/ipc-shapes.json");

    fn fixture_keys(name: &str) -> Vec<String> {
        let doc: serde_json::Value =
            serde_json::from_str(IPC_SHAPES_JSON).expect("ipc-shapes.json 非法 JSON");
        let mut keys: Vec<String> = doc
            .get(name)
            .unwrap_or_else(|| panic!("ipc-shapes.json 缺 {name} 条目"))
            .as_array()
            .unwrap_or_else(|| panic!("ipc-shapes.json 的 {name} 应为字符串数组"))
            .iter()
            .map(|v| {
                v.as_str()
                    .unwrap_or_else(|| panic!("{name} 的键名应为字符串"))
                    .to_string()
            })
            .collect();
        keys.sort();
        keys
    }

    fn json_keys<T: serde::Serialize>(sample: &T) -> Vec<String> {
        let mut keys: Vec<String> = serde_json::to_value(sample)
            .expect("样本序列化失败")
            .as_object()
            .expect("样本应序列化为 JSON 对象")
            .keys()
            .cloned()
            .collect();
        keys.sort();
        keys
    }

    macro_rules! assert_shape {
        ($name:expr, $sample:expr) => {{
            assert_eq!(
                json_keys(&$sample),
                fixture_keys($name),
                "{} 的 JSON key 集与 frontend/src/types/ipc-shapes.json 不一致——改结构体必须先改该 fixture（Rust 与 TS 两侧同闸）",
                $name
            );
        }};
    }

    /// 闸门 3：IPC 结构体 key 集（含 casing）与共享 fixture 一致。
    #[test]
    fn ipc_struct_shapes_match_fixture() {
        use crate::diagnostics::{
            DshDiagnosticInfo, NodeDiagnosticInfo, PlatformDiagnosticInfo, PnpmDiagnosticInfo,
            StorageDiagnosticInfo, SystemDiagnosticsReport,
        };
        use crate::plugins::{AggregatePlugin, AggregateSource, CopyConfigOutcome, PluginRowState};
        use crate::profiles::ProfileSummary;
        use crate::sessions::{RepairOutcome, SessionItem, SessionStatus};
        use crate::settings::ShellSettings;

        assert_shape!(
            "ShellSettings",
            ShellSettings {
                default_mode: None,
                default_profile: None,
                locale: None,
                auto_restart: None,
                show_floating_switcher: None,
                switcher_shortcut: None,
                dismissed_update: None,
            }
        );
        assert_shape!(
            "ProfileSummary",
            ProfileSummary {
                name: String::new(),
                materialized: false,
                bundles: Vec::new(),
                dependencies: Vec::new(),
                web_ui: false,
            }
        );
        assert_shape!(
            "SessionItem",
            SessionItem {
                id: String::new(),
                title: String::new(),
                project_name: String::new(),
                project_dir_raw: String::new(),
                decoded_project_path: String::new(),
                file_path: String::new(),
                updated_at: 0,
                size_bytes: 0,
                is_compressed: false,
                has_backup: false,
                status: SessionStatus::Healthy,
                health_detail: None,
                active: false,
                archived: false,
                created_at: 0,
                event_count: 0,
                end_state: None,
                subagent: false,
                agent_preset: None,
                validator: None,
            }
        );
        assert_shape!(
            "PluginRowState",
            PluginRowState {
                id: String::new(),
                pkg_name: String::new(),
                shell_disabled: false,
                patch_entries: 0,
                contributed_ids: Vec::new(),
            }
        );
        assert_shape!(
            "AggregatePlugin",
            AggregatePlugin {
                name: String::new(),
                description: None,
                sources: Vec::new(),
            }
        );
        assert_shape!(
            "AggregateSource",
            AggregateSource {
                profile: String::new(),
                version: None,
            }
        );
        assert_shape!(
            "CopyConfigOutcome",
            CopyConfigOutcome {
                copied: 0,
                skipped_existing: false,
                detail: String::new(),
            }
        );
        assert_shape!(
            "RepairOutcome",
            RepairOutcome {
                session_id: String::new(),
                success: false,
                message: String::new(),
            }
        );
        assert_shape!(
            "NodeDiagnosticInfo",
            NodeDiagnosticInfo {
                path: String::new(),
                version: String::new(),
                source: String::new(),
                is_ready: false,
            }
        );
        assert_shape!(
            "PnpmDiagnosticInfo",
            PnpmDiagnosticInfo {
                path: String::new(),
                version: None,
                is_ready: false,
            }
        );
        assert_shape!(
            "DshDiagnosticInfo",
            DshDiagnosticInfo {
                path: String::new(),
                version: None,
                source: String::new(),
                is_ready: false,
            }
        );
        assert_shape!(
            "StorageDiagnosticInfo",
            StorageDiagnosticInfo {
                dsh_home: String::new(),
                total_bytes: 0,
                profiles_bytes: 0,
                sessions_bytes: 0,
                profiles_count: 0,
                sessions_count: 0,
            }
        );
        assert_shape!(
            "PlatformDiagnosticInfo",
            PlatformDiagnosticInfo {
                os: String::new(),
                arch: String::new(),
            }
        );
        assert_shape!(
            "SystemDiagnosticsReport",
            SystemDiagnosticsReport {
                node: NodeDiagnosticInfo {
                    path: String::new(),
                    version: String::new(),
                    source: String::new(),
                    is_ready: false,
                },
                pnpm: PnpmDiagnosticInfo {
                    path: String::new(),
                    version: None,
                    is_ready: false,
                },
                dsh: DshDiagnosticInfo {
                    path: String::new(),
                    version: None,
                    source: String::new(),
                    is_ready: false,
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
                    os: String::new(),
                    arch: String::new(),
                },
            }
        );
    }
}
