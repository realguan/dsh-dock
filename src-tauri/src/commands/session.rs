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
/// 会话管理：取消归档（2026-09-15，ADR-0021 **路线 A**）——经 Host RPC
/// `workspace/unarchiveSession`，**不触碰** `$DSH_HOME/storages/workspace.json`
/// （理由见 `sessions::request_unarchive`：该文件是内存状态的投影，
/// 运行中 Host 会整体覆盖外部写入）。
///
/// **世界无关**：归档状态由**当前活跃 Host** 持有，其工作台地址在 Local 与 WSL
/// 两种模式下都记在 `ShellState.workbench_url`（`boot.rs:384` 就绪时写入），
/// 故本命令**不按 `World` 分支**——路线 A 没有文件路径参与，也就不存在宿主/客体
/// 分派问题（这是相对 ADR-0021 行动项"world 分派"的一处**有意简化**，已回报）。
/// 无活跃 Host（`workbench_url` 为空）时如实报错，**不降级改文件**（ADR-0021 §4）。
#[tauri::command]
pub async fn unarchive_session(
    app: tauri::AppHandle,
    session_id: String,
) -> Result<Vec<String>, String> {
    // 2026-09-15：回环调用必须带启动期兑换的 `/api` 会话 Cookie——dsh 0.1.6-alpha.1
    // 的 `/api` 在 Host 栅栏之后还有一道 `browserAuth`（失败 401），无 Cookie 恒 401。
    // Cookie 只在内存，不落盘不打日志（AGENTS §4.3）。
    let (origin, cookie) = {
        let state = app.state::<Arc<ShellState>>().inner().clone();
        let origin = {
            let guard = state.workbench_url.lock().unwrap();
            guard.as_ref().map(|u| {
                format!(
                    "{}://{}{}",
                    u.scheme(),
                    u.host_str().unwrap_or("127.0.0.1"),
                    u.port().map(|p| format!(":{p}")).unwrap_or_default()
                )
            })
        };
        let cookie = state.workbench_cookie.lock().unwrap().clone();
        (origin, cookie)
    };
    let Some(origin) = origin else {
        return Err(
            "工作台尚未就绪，无法取消归档：该操作需 DSH 运行时在线（壳不直接改磁盘状态）。\
             请先启动 DSH 后重试。"
                .to_string(),
        );
    };
    tauri::async_runtime::spawn_blocking(move || {
        crate::sessions::request_unarchive(&origin, cookie.as_deref(), &session_id)
    })
    .await
    .map_err(|e| format!("取消归档任务异常终止：{e}"))?
}
