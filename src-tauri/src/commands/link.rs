//! commands/link.rs —— link 域 IPC 命令处理层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 本层只做「参数校验 + 数据目录定位 + 阻塞动作下沉 spawn_blocking」，
//! 业务实现全在对应域模块（profiles/plugins/sessions/settings/…）。
//! 命令清单的唯一事实源仍是 `src/ipc.rs::COMMANDS`，三处同步由 cargo test 闸门拦。

use crate::{is_allowed_external_url, ShellState};
use std::sync::Arc;

use tauri::Manager;

/// 用系统默认浏览器打开外链或用系统文件管理器打开本地路径。
/// - 若为 HTTP(S) URL：必须通过白名单校验后打开浏览器；
/// - 若为本地路径：调用系统文件管理器（Finder / Explorer）打开该目录或文件。
#[tauri::command]
pub fn open_external(url: String) -> Result<(), String> {
    let trimmed = url.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        if !is_allowed_external_url(trimmed) {
            return Err(format!("不允许的外链：{url}"));
        }
        open::that_detached(trimmed).map_err(|e| format!("打开浏览器失败：{e}"))
    } else {
        let p = std::path::Path::new(trimmed);
        if p.exists() {
            open::that_detached(trimmed).map_err(|e| format!("打开文件管理器失败：{e}"))
        } else if let Some(parent) = p.parent() {
            if parent.exists() {
                open::that_detached(parent).map_err(|e| format!("打开文件管理器失败：{e}"))
            } else {
                Err(format!("目标路径不存在：{url}"))
            }
        } else {
            Err(format!("目标路径不存在：{url}"))
        }
    }
}
/// 用系统默认浏览器打开当前工作台（壳内 WebView → 浏览器；dsh 就绪后可用）。
#[tauri::command]
pub fn open_workbench_in_browser(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    let url = state
        .workbench_url
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "工作台尚未就绪".to_string())?;
    open::that_detached(url.as_str()).map_err(|e| format!("打开浏览器失败：{e}"))
}
/// 读取当前工作台地址（关于页/菜单展示用；未就绪返回 null）。
#[tauri::command]
pub fn get_workbench_url(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    let url = state.workbench_url.lock().unwrap().clone();
    Ok(url.map(|u| u.to_string()))
}
