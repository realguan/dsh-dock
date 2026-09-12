//! commands/boot.rs —— boot 域 IPC 命令处理层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 本层只做「参数校验 + 数据目录定位 + 阻塞动作下沉 spawn_blocking」，
//! 业务实现全在对应域模块（profiles/plugins/sessions/settings/…）。
//! 命令清单的唯一事实源仍是 `src/ipc.rs::COMMANDS`，三处同步由 cargo test 闸门拦。

use crate::boot::{
    emit_boot_error, emit_step, emit_upgrade, executor_for_mode, launch_executor_after_probe,
    refresh_update_ui, run_executor_session, switch_mode, ShellState,
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
    // 选择器落地也是一次新启动（ADR-0014 代际令牌）：领新令牌，作废在途启动线程。
    let token = state.begin_boot();
    // 后台线程启动（npm/下载动作不阻塞）
    std::thread::spawn(move || {
        if let Err(e) = run_executor_session(state, handle.clone(), executor, token) {
            tracing::error!("启动 DSH 失败: {e}");
        }
    });
    Ok(())
}
pub(crate) fn evaluate_needs_mode_selection(
    is_windows: bool,
    active_mode_is_none: bool,
    default_mode_is_none: bool,
) -> bool {
    is_windows && active_mode_is_none && default_mode_is_none
}

/// 读取启动阶段缓存的状态与错误（前端挂载时补水，解决 early emit 竞态丢失事件的问题）。
/// 2026-09-10（ADR-0014）：新增 `intent`（交接意图）——主窗口在切换/重启后是**整文档
/// 重载**，新文档只剩这一条通道能拿到「我是被谁重启的、从什么时候开始」，
/// 于是启动屏与幕布才能续上控制中心那条导轨与计时器。
#[tauri::command]
pub fn get_boot_status(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    if let Some(shell_state) = app.try_state::<Arc<ShellState>>() {
        // 语义（2026-09-11 收紧）：缓存由轮次起点 `begin_boot()` 清空，故这里返回的
        // 永远是**当前轮次**的可见状态——新一轮已开始但尚无新错误时必为 `null`，
        // 而不是上一轮的错误。曾因清缓存散落在 3 个调用点（漏 2 处）导致旧错误
        // 经本命令补水进新文档（v1.2.0 实测 2.1）。
        let error = shell_state.boot.error();
        let steps = shell_state.boot.steps();
        let data_dir = app.path().app_data_dir().ok();
        let needs_mode_selection = if cfg!(windows) {
            let active_is_none = shell_state.active_mode.lock().unwrap().is_none();
            let default_is_none = data_dir
                .as_deref()
                .map(crate::settings::load)
                .map(|s| s.default_mode.is_none())
                .unwrap_or(true);
            evaluate_needs_mode_selection(true, active_is_none, default_is_none)
        } else {
            false
        };
        Ok(serde_json::json!({
            "steps": steps,
            "error": error,
            "intent": shell_state.handoff_json(),
            "needs_mode_selection": needs_mode_selection,
            "needsModeSelection": needs_mode_selection,
        }))
    } else {
        Ok(serde_json::json!({
            "steps": [],
            "error": null,
            "intent": serde_json::Value::Null,
            "needs_mode_selection": false,
            "needsModeSelection": false,
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
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    // 选择落地同样是一次全新启动（ADR-0014）：领令牌作废在途启动线程。
    // 可见缓存（上一轮的错误/步骤）随 begin_boot 一并撤销——2026-09-11 前此处
    // 另有一句 clear_boot_cache()，与 begin_boot 合并后由轮次起点统一负责。
    // 位置刻意留在 data_dir 解析之后：解析失败时本轮尚未开始，不应作废在途启动。
    let token = state.begin_boot();
    *state.handoff.lock().unwrap() = None;
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
        Ok(executor) => launch_executor_after_probe(state, handle, executor, token),
        Err(e) => emit_boot_error(&handle, &e, ""),
    });
    Ok(())
}
/// 错误卡动作（retry / upgrade）：重新解析并启动；upgrade 先升级全局 dsh。
/// upgrade_only：仅升级 + 刷新状态（不打断进行中的会话）。
/// `version`（2026-09-09 版本选择器）：upgrade / upgrade_only 的显式目标版本
/// （含 alpha 预览版，经版本列表选择进入）；None = 最新可接受版（稳定/rc）。
#[tauri::command]
pub fn terminal_action(
    app: tauri::AppHandle,
    action: String,
    version: Option<String>,
) -> Result<(), String> {
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
            emit_upgrade(&handle, "running", "", false);
            let step_detail = match &version {
                Some(v) => format!("正在安装 DSH {v}…"),
                None => "正在升级官方 DSH 到最新稳定版…".to_string(),
            };
            emit_step(&handle, 2, "running", &step_detail);
            // 升级 = 引擎私有动作（ADR-0010）：pnpm add -g 到引擎目录，
            // 不再动用户全局安装（「根本不碰」取代「不覆盖」）。
            let resources_dir = crate::ui::resolve_resources_dir(&handle);
            let path_env = crate::resolve::effective_path();
            match crate::updates::upgrade_engine_dsh(
                &data_dir,
                &resources_dir,
                &path_env,
                version.as_deref(),
            ) {
                Ok(plan) => {
                    let (installed_version, installed) = match &plan {
                        crate::updates::UpgradePlan::Install(v) => (v, true),
                        crate::updates::UpgradePlan::Skip(v) => (v, false),
                    };
                    let msg = if installed {
                        format!("DSH 已升级到 {installed_version}")
                    } else {
                        format!("DSH 已是最新（{installed_version}）")
                    };
                    emit_step(&handle, 2, "done", &msg);
                    // 刷新版本状态（托盘/前端 chip）
                    refresh_update_ui(&handle, &state);
                    emit_upgrade(&handle, "done", installed_version, installed);
                }
                Err(e) => {
                    tracing::error!(err = ?e, "dsh 升级失败");
                    emit_upgrade(&handle, "failed", &format!("{e:#}"), false);
                    emit_boot_error(&handle, &format!("升级失败：{e:#}"), "");
                    return;
                }
            }
            if only {
                let _ = handle;
                return;
            }
        }
        // 重新走解析链 + 启动（重试同样领新令牌：作废在途的旧启动线程）
        let token = state.begin_boot();
        crate::boot::lib_boot_again(state, handle.clone(), data_dir, token);
        let _ = handle;
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_windows_never_needs_mode_selection() {
        assert!(!evaluate_needs_mode_selection(false, true, true));
        assert!(!evaluate_needs_mode_selection(false, false, true));
        assert!(!evaluate_needs_mode_selection(false, true, false));
        assert!(!evaluate_needs_mode_selection(false, false, false));
    }

    #[test]
    fn windows_needs_mode_selection_only_when_unselected_and_no_default() {
        // 首次启动且未设默认：需要模式选择
        assert!(evaluate_needs_mode_selection(true, true, true));
        // 已有默认模式：不需要模式选择
        assert!(!evaluate_needs_mode_selection(true, true, false));
        // 运行期已通过 choose_mode 选定模式（当前会话已激活）：不需要再次选择
        assert!(!evaluate_needs_mode_selection(true, false, true));
        // 既有默认又已激活：不需要
        assert!(!evaluate_needs_mode_selection(true, false, false));
    }
}
