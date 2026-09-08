//! commands/update.rs —— update 域 IPC 命令处理层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 本层只做「参数校验 + 数据目录定位 + 阻塞动作下沉 spawn_blocking」，
//! 业务实现全在对应域模块（profiles/plugins/sessions/settings/…）。
//! 命令清单的唯一事实源仍是 `src/ipc.rs::COMMANDS`，三处同步由 cargo test 闸门拦。

use crate::{cached_update_status, refresh_update_ui, ShellState};
use std::sync::Arc;

use tauri::Manager;

/// 前端/托盘读取最近一次检测结果（即读，不触网）。
#[tauri::command]
pub fn get_update_status(app: tauri::AppHandle) -> Result<crate::updates::UpdateStatus, String> {
    // 启动清单或宿主解析失败时，前端仍会请求版本状态。这里不能用
    // `state()`：它在状态尚未注册时会 panic，反而让本应展示错误卡的应用崩溃。
    Ok(cached_update_status(&app))
}
/// 手动触发后台检测（异步：立即返回，完成时 boot:update + 托盘刷新）。
#[tauri::command]
pub fn check_updates(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    let handle = app.clone();
    std::thread::spawn(move || refresh_update_ui(&handle, &state));
    Ok(())
}
/// 读取桌面客户端自更新状态（即读，不触网；前端初始渲染）。
#[tauri::command]
pub fn get_client_update(app: tauri::AppHandle) -> Result<crate::updater::ClientUpdate, String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    Ok(crate::updater::current(&state))
}
/// 「检查客户端更新」：后台查 GitHub Releases latest.json，结果经 app:update 回推。
#[tauri::command]
pub fn client_update_check(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    crate::updater::run_check(app, state);
    Ok(())
}
/// 「确认安装客户端更新」：下载 → 安装 → 重启（Windows 由安装器接手后退出）。
#[tauri::command]
pub fn client_update_apply(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    crate::updater::run_download_and_install(app, state);
    Ok(())
}
