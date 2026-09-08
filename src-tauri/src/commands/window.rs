//! commands/window.rs —— window 域 IPC 命令处理层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 本层只做「参数校验 + 数据目录定位 + 阻塞动作下沉 spawn_blocking」，
//! 业务实现全在对应域模块（profiles/plugins/sessions/settings/…）。
//! 命令清单的唯一事实源仍是 `src/ipc.rs::COMMANDS`，三处同步由 cargo test 闸门拦。

use tauri::Manager;

/// Profile 管理器窗口（控制中心）：独立窗口（label=profiles，React 渲染
/// pages/ProfileManager.tsx）。独立窗口与 about 同理——主窗口 boot 后会
/// 导航进 dsh 工作台（remote），壳页不可达；管理器要随时可达。
/// 主线程创建约束同 open_about_window（WebView2 白板坑）。
#[tauri::command]
pub fn open_profiles_window(app: tauri::AppHandle) {
    let handle = app.clone();
    if let Err(e) = app.run_on_main_thread(move || {
        if let Some(win) = handle.get_webview_window("profiles") {
            let _ = win.show();
            let _ = win.unminimize();
            let _ = win.set_focus();
            return;
        }
        let platform_script = format!(
            "window.__DSH_PLATFORM__ = {{ os: '{}', wsl: {} }};",
            std::env::consts::OS,
            if cfg!(windows) { "true" } else { "false" }
        );
        let builder = tauri::WebviewWindowBuilder::new(
            &handle,
            "profiles",
            tauri::WebviewUrl::App("/".into()),
        )
        .title("控制中心")
        // Master-Detail 双栏工作台：宽 1180 + 高 780，开箱即得宽屏完整双栏布局
        .inner_size(1180.0, 780.0)
        .min_inner_size(860.0, 600.0)
        .resizable(true)
        .center()
        .background_color(tauri::utils::config::Color(249, 250, 251, 255))
        .initialization_script(&platform_script);
        match builder.build() {
            Ok(_) => tracing::info!("控制中心窗口已创建"),
            Err(e) => tracing::error!("创建控制中心窗口失败：{e}"),
        }
    }) {
        tracing::error!("调度控制中心窗口创建到主线程失败：{e}");
    }
}
/// 聚焦主工作台窗口（label=main）。
#[tauri::command]
pub fn focus_main_window(app: tauri::AppHandle) {
    let handle = app.clone();
    if let Err(e) = app.run_on_main_thread(move || {
        if let Some(win) = handle.get_webview_window("main") {
            let _ = win.show();
            let _ = win.unminimize();
            let _ = win.set_focus();
        }
    }) {
        tracing::error!("调度主窗口聚焦到主线程失败：{e}");
    }
}
