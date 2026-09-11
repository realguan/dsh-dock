//! commands/console.rs —— console 域 IPC 命令处理层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 本层只做「参数校验 + 数据目录定位 + 阻塞动作下沉 spawn_blocking」，
//! 业务实现全在对应域模块（profiles/plugins/sessions/settings/…）。
//! 命令清单的唯一事实源仍是 `src/ipc.rs::COMMANDS`，三处同步由 cargo test 闸门拦。
//!
//! **世界择源（ADR-0016 §4 P3）**：凭据 / DSH 设置 / MCP / 系统诊断
//! 均根据当前世界择源：本地模式操作宿主 home，WSL 模式下沉至客体读写与探测。
//! 例外：`get_shell_settings`/`set_shell_settings`（壳自身设置，存 app_data）与
//! `get_app_logs`（源主要是壳自己的日志）与运行世界无关，不设闸。

use tauri::Manager;

/// 偏好与设置：读取壳设置（locale、auto_restart、default_mode 等）
#[tauri::command]
pub fn get_shell_settings(app: tauri::AppHandle) -> Result<crate::settings::ShellSettings, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(crate::settings::load(&data_dir))
}
/// 偏好与设置：保存壳设置并跨窗口广播更新
#[tauri::command]
pub fn set_shell_settings(
    app: tauri::AppHandle,
    settings: crate::settings::ShellSettings,
) -> Result<(), String> {
    use tauri::Emitter;
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    crate::settings::save(&data_dir, &settings)?;
    let _ = app.emit("app:settings-changed", &settings);
    Ok(())
}
/// 环境健康体检：全量诊断大盘数据采集（4.11）
#[tauri::command]
pub async fn get_system_diagnostics(
    app: tauri::AppHandle,
) -> Result<crate::diagnostics::SystemDiagnosticsReport, String> {
    let world = crate::mgmt::current_world(&app)?;
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            Ok(crate::diagnostics::collect_diagnostics(&home, &data_dir))
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::diagnostics::collect_diagnostics_in_guest(&distro)
        }
    })
    .await
    .map_err(|e| format!("诊断报告收集异常终止：{e}"))?
}
/// 运行日志：安全读取指定源的尾部日志（4.11）
#[tauri::command]
pub async fn get_app_logs(
    app: tauri::AppHandle,
    source: String,
    tail_lines: Option<usize>,
) -> Result<crate::diagnostics::LogQueryResult, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let home = crate::resolve::user_dsh_home();
        crate::diagnostics::read_app_logs(&source, &data_dir, &home, tail_lines.unwrap_or(500))
    })
    .await
    .map_err(|e| format!("读取日志任务异常终止：{e}"))?
}
/// 凭据管理：读取 .credentials.yaml 原文（4.5）
#[tauri::command]
pub async fn get_credentials_raw(app: tauri::AppHandle) -> Result<String, String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::credentials::read_credentials(&home)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::credentials::get_credentials_raw_in_guest(&distro)
        }
    })
    .await
    .map_err(|e| format!("读取凭据任务异常终止：{e}"))?
}
/// 凭据管理：保存 .credentials.yaml 原文（4.5，严格 0600 权限与原子写）
#[tauri::command]
pub async fn save_credentials_raw(app: tauri::AppHandle, content: String) -> Result<(), String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::credentials::overwrite_credentials(&home, &content)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::credentials::save_credentials_raw_in_guest(&distro, &content)
        }
    })
    .await
    .map_err(|e| format!("保存凭据任务异常终止：{e}"))?
}
/// 凭据管理：获取脱敏后的凭据摘要列表（4.5）
#[tauri::command]
pub async fn get_credentials_summary(
    app: tauri::AppHandle,
) -> Result<Vec<crate::credentials::CredentialSummaryItem>, String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::credentials::get_credentials_summary(&home)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::credentials::get_credentials_summary_in_guest(&distro)
        }
    })
    .await
    .map_err(|e| format!("读取凭据摘要任务异常终止：{e}"))?
}
/// 凭据管理：针对指定 Provider 设置 API Key（4.5）
#[tauri::command]
pub async fn set_credential_key(
    app: tauri::AppHandle,
    provider: String,
    key: String,
) -> Result<(), String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::credentials::set_provider_key(&home, &provider, &key)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::credentials::set_provider_key_in_guest(&distro, &provider, &key)
        }
    })
    .await
    .map_err(|e| format!("保存 Provider 凭据任务异常终止：{e}"))?
}
/// DSH 引擎设置：读取 settings.yaml 原文（4.5）
#[tauri::command]
pub async fn get_dsh_settings_raw(app: tauri::AppHandle) -> Result<String, String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::dsh_settings::read_dsh_settings(&home)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::dsh_settings::read_dsh_settings_in_guest(&distro)
        }
    })
    .await
    .map_err(|e| format!("读取 DSH 设置任务异常终止：{e}"))?
}
/// DSH 引擎设置：保存 settings.yaml 原文（4.5）
#[tauri::command]
pub async fn save_dsh_settings_raw(app: tauri::AppHandle, content: String) -> Result<(), String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::dsh_settings::overwrite_dsh_settings(&home, &content)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::dsh_settings::overwrite_dsh_settings_in_guest(&distro, &content)
        }
    })
    .await
    .map_err(|e| format!("保存 DSH 设置任务异常终止：{e}"))?
}
/// MCP 管理：获取指定 profile 的 MCP 服务列表（4.7）
#[tauri::command]
pub async fn list_mcp_servers(
    app: tauri::AppHandle,
    profile: String,
) -> Result<Vec<crate::mcp::McpServerConfig>, String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::mcp::list_mcp_servers(&home, &profile)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::mcp::list_mcp_servers_in_guest(&distro, &profile)
        }
    })
    .await
    .map_err(|e| format!("读取 MCP 服务列表任务异常终止：{e}"))?
}
/// MCP 管理：保存或更新单个 MCP 服务（4.7）
#[tauri::command]
pub async fn save_mcp_server(
    app: tauri::AppHandle,
    profile: String,
    server: crate::mcp::McpServerConfig,
) -> Result<(), String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::mcp::save_mcp_server(&home, &profile, server)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::mcp::save_mcp_server_in_guest(&distro, &profile, server)
        }
    })
    .await
    .map_err(|e| format!("保存 MCP 服务任务异常终止：{e}"))?
}
/// MCP 管理：删除指定 MCP 服务（4.7）
#[tauri::command]
pub async fn delete_mcp_server(
    app: tauri::AppHandle,
    profile: String,
    server_name: String,
) -> Result<(), String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::mcp::delete_mcp_server(&home, &profile, &server_name)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::mcp::delete_mcp_server_in_guest(&distro, &profile, &server_name)
        }
    })
    .await
    .map_err(|e| format!("删除 MCP 服务任务异常终止：{e}"))?
}
