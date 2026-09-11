//! commands/session.rs —— session 域 IPC 命令处理层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 本层只做「参数校验 + 数据目录定位 + 阻塞动作下沉 spawn_blocking」，
//! 业务实现全在对应域模块（profiles/plugins/sessions/settings/…）。
//! 命令清单的唯一事实源仍是 `src/ipc.rs::COMMANDS`，三处同步由 cargo test 闸门拦。

use crate::boot::{engine_session_alive, ShellState};
use std::sync::Arc;

use tauri::Manager;

/// 会话管理与自愈：扫描会话列表（只读文件扫描 + 健康检查/标题提取）
#[tauri::command]
pub async fn list_sessions(
    app: tauri::AppHandle,
) -> Result<Vec<crate::sessions::SessionItem>, String> {
    let world = crate::mgmt::current_world(&app)?;
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let engine_alive = {
        let state = app.state::<Arc<ShellState>>();
        engine_session_alive(&state)
    };
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => crate::sessions::scan_sessions(
            &crate::resolve::user_dsh_home(),
            &data_dir,
            engine_alive,
        ),
        crate::mgmt::World::Wsl { distro } => {
            crate::sessions::scan_sessions_in_guest(&distro, &data_dir, engine_alive)
        }
    })
    .await
    .map_err(|e| format!("会话列表扫描任务异常终止：{e}"))?
}
/// 会话管理与自愈：修复单个指定会话
#[tauri::command]
pub async fn repair_session(
    app: tauri::AppHandle,
    session_path: String,
) -> Result<crate::sessions::RepairOutcome, String> {
    let world = crate::mgmt::current_world(&app)?;
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let engine_alive = {
        let state = app.state::<Arc<ShellState>>();
        engine_session_alive(&state)
    };
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => crate::sessions::run_repair(
            Some(&session_path),
            &crate::resolve::user_dsh_home(),
            &data_dir,
            engine_alive,
        ),
        crate::mgmt::World::Wsl { distro } => crate::sessions::run_repair_in_guest(
            Some(&session_path),
            &distro,
            &data_dir,
            engine_alive,
        ),
    })
    .await
    .map_err(|e| format!("单会话修复任务异常终止：{e}"))?
}
/// 会话管理与自愈：全量体检与自愈修复
#[tauri::command]
pub async fn repair_all_sessions(
    app: tauri::AppHandle,
) -> Result<crate::sessions::RepairOutcome, String> {
    let world = crate::mgmt::current_world(&app)?;
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let engine_alive = {
        let state = app.state::<Arc<ShellState>>();
        engine_session_alive(&state)
    };
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => crate::sessions::run_repair(
            None,
            &crate::resolve::user_dsh_home(),
            &data_dir,
            engine_alive,
        ),
        crate::mgmt::World::Wsl { distro } => {
            crate::sessions::run_repair_in_guest(None, &distro, &data_dir, engine_alive)
        }
    })
    .await
    .map_err(|e| format!("全量会话自愈任务异常终止：{e}"))?
}
/// 会话管理：删除指定会话（4.6）
#[tauri::command]
pub async fn delete_session(app: tauri::AppHandle, session_path: String) -> Result<(), String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::sessions::remove_session(&home, &session_path)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::sessions::remove_session_in_guest(&distro, &session_path)
        }
    })
    .await
    .map_err(|e| format!("删除会话任务异常终止：{e}"))?
}
