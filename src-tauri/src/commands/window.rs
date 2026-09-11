//! commands/window.rs —— window 域 IPC 命令处理层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 本层只做「参数校验 + 数据目录定位 + 阻塞动作下沉 spawn_blocking」，
//! 业务实现全在对应域模块（profiles/plugins/sessions/settings/…）。
//! 命令清单的唯一事实源仍是 `src/ipc.rs::COMMANDS`，三处同步由 cargo test 闸门拦。

use tauri::Manager;

/// Profile 管理器窗口（控制中心）：独立窗口（label=profiles，React 渲染
/// pages/ProfileManager.tsx）。独立窗口与 about 同理——主窗口 boot 后会
/// 导航进 dsh 工作台（remote），壳页不可达；管理器要随时可达。
///
/// **2026-09-10 修复（v1.1.0 Windows 实测 1.3）**：窗口创建经
/// [`crate::ui::post_to_event_loop`] **从事件循环投递**，不再依赖"调用方恰好不在
/// 主线程"。本命令有两条入口，上下文截然不同：
/// - 托盘 / 菜单（`on_menu_event`）→ tao 事件循环 → 旧写法也正常；
/// - 工作台悬浮胶囊（注入在 dsh 页面里的 `invoke`，remote origin 走
///   `window.ipc.postMessage` 回退通道）→ 事件处理在 **UI 线程** → 旧写法让
///   `build()` 就地跑在 WebView2 回调里 → 嵌套消息泵挂死：**白窗 + 卡死 +
///   关不掉**（正是实测现象）。
#[tauri::command]
pub fn open_profiles_window(app: tauri::AppHandle) {
    let handle = app.clone();
    crate::ui::post_to_event_loop(&app, move || {
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
        // 底色 = `index.css` 的 `--color-bg`（与主窗口同源；2026-09-10 批次 E 的
        // 「三处一致性」闸门原先漏了本窗口这**第四处**真相源，现统一引用常量）。
        .background_color(crate::ui::WINDOW_BACKGROUND)
        .initialization_script(&platform_script);
        match builder.build() {
            Ok(_) => tracing::info!("控制中心窗口已创建"),
            Err(e) => tracing::error!("创建控制中心窗口失败：{e}"),
        }
    });
}
/// 聚焦主工作台窗口（label=main）。
#[tauri::command]
pub fn focus_main_window(app: tauri::AppHandle) {
    let handle = app.clone();
    crate::ui::post_to_event_loop(&app, move || {
        if let Some(win) = handle.get_webview_window("main") {
            let _ = win.show();
            let _ = win.unminimize();
            let _ = win.set_focus();
        }
    });
}
