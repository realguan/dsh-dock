//! commands/boot.rs —— boot 域 IPC 命令处理层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 本层只做「参数校验 + 数据目录定位 + 阻塞动作下沉 spawn_blocking」，
//! 业务实现全在对应域模块（profiles/plugins/sessions/settings/…）。
//! 命令清单的唯一事实源仍是 `src/ipc.rs::COMMANDS`，三处同步由 cargo test 闸门拦。

use crate::{
    emit_boot_error, emit_step, emit_upgrade, executor_for_mode, launch_executor_after_probe,
    refresh_update_ui, resolve_resources_dir, run_executor_session, switch_mode, ShellState,
};
use std::sync::Arc;

use crate::settings;
use tauri::Manager;

/// 唯一 IPC 命令（②b profile 选择器）：选定 profile → 用 pending 的会话启动。
#[tauri::command]
pub fn choose_profile(app: tauri::AppHandle, profile: String) -> Result<(), String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    let mut executor = state
        .pending
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| "无待启动任务（请重新打开终端）".to_string())?;
    executor.select_profile(profile);
    let handle = app.clone();
    // 后台线程启动（npm/下载动作不阻塞）
    std::thread::spawn(move || {
        if let Err(e) = run_executor_session(state, handle.clone(), executor) {
            tracing::error!("启动 DSH 失败: {e}");
        }
    });
    Ok(())
}
/// 读取启动阶段缓存的状态与错误（前端挂载时补水，解决 early emit 竞态丢失事件的问题）。
#[tauri::command]
pub fn get_boot_status(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    if let Some(shell_state) = app.try_state::<Arc<ShellState>>() {
        let error = shell_state.boot_error.lock().ok().and_then(|e| e.clone());
        let steps = shell_state
            .boot_steps
            .lock()
            .ok()
            .map(|s| s.clone())
            .unwrap_or_default();
        Ok(serde_json::json!({
            "steps": steps,
            "error": error,
        }))
    } else {
        Ok(serde_json::json!({
            "steps": [],
            "error": null,
        }))
    }
}
/// 「在 WSL 中打开」（顶栏入口；现已记默认 = 与菜单切换同语义）。
/// 非 Windows：WSL 不存在——防御性拒绝（前端该按钮本就不渲染，这里兜底防
/// 手工调用 / 旧页面缓存把会话 teardown 后写进脏默认）。
#[tauri::command]
pub fn boot_in_wsl(app: tauri::AppHandle) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("WSL 仅支持 Windows 平台。".to_string());
    }
    let state = app.state::<Arc<ShellState>>().inner().clone();
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let handle = app.clone();
    std::thread::spawn(move || {
        switch_mode(handle, state, settings::Mode::Wsl, data_dir);
    });
    Ok(())
}
/// 首次运行选择落地（壳运行环境页 → 写默认（可选）→ 按所选模式启动；
/// 2026-08-27 前端迁移后页面为 SPA /mode 路由，回跳主窗口经 React Router）。
#[tauri::command]
pub fn choose_mode(app: tauri::AppHandle, mode: String, set_default: bool) -> Result<(), String> {
    let m = settings::Mode::parse(&mode).ok_or_else(|| format!("未知运行环境：{mode}"))?;
    // 非 Windows：WSL 不存在（mode.html 本就不渲染 WSL 卡，这里兜底防旧页面缓存）。
    if m == settings::Mode::Wsl && !cfg!(windows) {
        return Err("WSL 仅支持 Windows 平台。".to_string());
    }
    let state = app.state::<Arc<ShellState>>().inner().clone();
    state.clear_boot_cache();
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    if set_default {
        // load-modify-save：同 switch_mode，不得抹掉其他已存字段。
        let mut shell_settings = crate::settings::load(&data_dir);
        shell_settings.default_mode = Some(m);
        crate::settings::save(&data_dir, &shell_settings)
            .map_err(|e| format!("保存默认运行环境失败：{e}"))?;
    }
    *state.active_mode.lock().unwrap() = Some(m);
    let handle = app.clone();
    std::thread::spawn(move || match executor_for_mode(m, &handle, data_dir) {
        Ok(executor) => launch_executor_after_probe(state, handle, executor),
        Err(e) => emit_boot_error(&handle, &e, ""),
    });
    Ok(())
}
/// 错误卡动作（retry / upgrade）：重新解析并启动；upgrade 先升级全局 dsh。
/// upgrade_only：仅升级 + 刷新状态（不打断进行中的会话）。
#[tauri::command]
pub fn terminal_action(app: tauri::AppHandle, action: String) -> Result<(), String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let handle = app.clone();
    std::thread::spawn(move || {
        // upgrade / upgrade_only：动用户全局 dsh（按钮确认即授权，动作在后台线程）。
        // 4.4⑤：全链路发 `dsh:upgrade` 事件（关于页升级按钮的真实反馈通道）——
        // 此前失败只发 boot:error，主窗口不在 boot 屏时用户完全看不到：
        // 2026-08-30 实测 0.1.2-alpha.2 依赖未完整发布，pnpm 两个 registry 均
        // 失败，错误无任何可见出口，用户视角 = 「点了没反应，等了也没升级」。
        let only = action == "upgrade_only";
        if only || action == "upgrade" {
            emit_upgrade(&handle, "running", "");
            emit_step(&handle, 2, "running", "正在升级官方 DSH 到最新稳定版…");
            // 升级 = 引擎私有动作（ADR-0010）：pnpm add -g 到引擎目录，
            // 不再动用户全局安装（「根本不碰」取代「不覆盖」）。
            let resources_dir = resolve_resources_dir(&handle);
            let path_env = crate::resolve::effective_path();
            match crate::updates::upgrade_engine_dsh(&data_dir, &resources_dir, &path_env) {
                Ok(version) => {
                    emit_step(&handle, 2, "done", &format!("DSH 已升级到 {version}"));
                    // 刷新版本状态（托盘/前端 chip）
                    refresh_update_ui(&handle, &state);
                    emit_upgrade(&handle, "done", &version);
                }
                Err(e) => {
                    tracing::error!(err = ?e, "dsh 升级失败");
                    emit_upgrade(&handle, "failed", &format!("{e:#}"));
                    emit_boot_error(&handle, &format!("升级失败：{e:#}"), "");
                    return;
                }
            }
            if only {
                let _ = handle;
                return;
            }
        }
        // 重新走解析链 + 启动
        crate::lib_boot_again(state, handle.clone(), data_dir);
        let _ = handle;
    });
    Ok(())
}
