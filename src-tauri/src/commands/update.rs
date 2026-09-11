//! commands/update.rs —— update 域 IPC 命令处理层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 本层只做「参数校验 + 数据目录定位 + 阻塞动作下沉 spawn_blocking」，
//! 业务实现全在对应域模块（profiles/plugins/sessions/settings/…）。
//! 命令清单的唯一事实源仍是 `src/ipc.rs::COMMANDS`，三处同步由 cargo test 闸门拦。

use crate::boot::{cached_update_status, refresh_update_ui, ShellState};
use std::sync::Arc;

use tauri::Manager;

/// 前端/托盘读取最近一次检测结果（即读，不触网）。
///
/// **2026-09-11（task-45 / D5）世界感知**：版本/引擎维度按**当前运行世界**择源——
/// 宿主世界读壳引擎（现状，行为零变化）；WSL 世界改为客体探测（复用
/// `guest.rs::diagnostics_script` → `diagnostics::collect_diagnostics_in_guest`）。
/// 修前关于页恒读宿主 `engines/`，而 WSL 模式下 dsh 装在客体 → 恒「未检出」，
/// 与健康大盘（客体就绪）自相矛盾。
///
/// 阻塞纪律：客体探测会起 `wsl.exe` 子进程（最坏 30s 超时），故整体下沉
/// `spawn_blocking`（不冻结 IPC/主线程），并带 60s TTL 缓存（关于页 + 托盘 +
/// 手动刷新会重复问）。本命令只在关于页/托盘读取路径上，**不在启动链**上——
/// 启动链的更新检测走 `boot::refresh_update_ui`（本任务未触碰）。
#[tauri::command]
pub async fn get_update_status(
    app: tauri::AppHandle,
) -> Result<crate::updates::UpdateStatus, String> {
    // 启动清单或宿主解析失败时，前端仍会请求版本状态。这里不能用
    // `state()`：它在状态尚未注册时会 panic，反而让本应展示错误卡的应用崩溃。
    let base = cached_update_status(&app);
    // 世界择源：失败（WSL 模式但发行版未记录）按诚实降级处理，绝不回落宿主数据。
    let world = crate::mgmt::current_world(&app);
    tauri::async_runtime::spawn_blocking(move || match world {
        Ok(world) => crate::updates::world_aware_status(base, &world),
        Err(reason) => {
            tracing::warn!("关于页世界未定，版本维度按「探测不可用」呈现：{reason}");
            crate::updates::status_world_unresolved(base, &reason)
        }
    })
    .await
    .map_err(|e| format!("版本状态任务异常终止：{e}"))
}
/// 手动触发后台检测（异步：立即返回，完成时 boot:update + 托盘刷新）。
#[tauri::command]
pub fn check_updates(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    let handle = app.clone();
    std::thread::spawn(move || refresh_update_ui(&handle, &state));
    Ok(())
}
/// DSH 版本列表（版本选择器数据源，2026-09-09）：packument 镜像链拉取 +
/// 通道归类 + 与已装版本的相对关系（比较在后端做，前端不实现第二套比较器）。
#[tauri::command]
pub async fn list_dsh_versions(
    app: tauri::AppHandle,
) -> Result<crate::updates::DshVersionsResult, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        crate::updates::list_dsh_versions(
            crate::updates::detect_current_version(&data_dir).as_deref(),
        )
    })
    .await
    .map_err(|e| format!("版本列表任务异常终止：{e}"))?
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
