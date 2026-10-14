//! commands/window.rs —— window 域 IPC 命令处理层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 本层只做「参数校验 + 数据目录定位 + 阻塞动作下沉 spawn_blocking」，
//! 业务实现全在对应域模块（profiles/plugins/sessions/settings/…）。
//! 命令清单的唯一事实源仍是 `src/ipc.rs::COMMANDS`，三处同步由 cargo test 闸门拦。

use tauri::Manager;

/// deminiaturize 是**异步动画**（见 [`bring_to_front`]）：补一次复焦要跨过它。
const REFOCUS_DELAY_MS: u64 = 320;

/// 把指定窗口可靠地唤到前台（2026-09-21 立，修「点返回工作台，工作台不出现」）。
///
/// **为什么不能沿用 `show() → unminimize() → set_focus()`**（旧写法，主窗口三处入口同款）：
/// tao 0.35.3 的 macOS `set_focus()` 自带守卫——`is_minimized || !is_visible` 时
/// **整个调用直接返回（静默 no-op）**（`tao-0.35.3/src/platform_impl/macos/window.rs:677-685`）；
/// 而 `unminimize()` 走 `NSWindow::deminiaturize`（**异步动画**），`show()` 只做
/// `makeKeyAndOrderFront`（**不激活 app**）。旧顺序里 `set_focus()` 往往跑在
/// deminiaturize 落地之前 ⇒ 守卫判否 ⇒ 既没把窗口提到前台、也没执行
/// `NSApp.activateIgnoringOtherApps`（它**只在守卫内**：
/// `tao-0.35.3/src/platform_impl/macos/util/async.rs:231-238`）⇒ 窗口停在 Dock /
/// 别的 Space，用户看到的就是「点了没反应」。
///
/// 现顺序：先 `unminimize()`（**仅当确实被最小化**）→ `show()` → `set_focus()`，
/// 再在 [`REFOCUS_DELAY_MS`] 后补一次——那一刻 deminiaturize 已落地，守卫必为真。
/// 窗口不存在时**响亮记日志**（旧写法 `if let Some` 把这条路径静默吞掉）。
pub(crate) fn bring_to_front(app: &tauri::AppHandle, label: &str) {
    let handle = app.clone();
    let label = label.to_owned();
    crate::ui::post_to_event_loop(app, move || {
        if !raise_once(&handle, &label) {
            return;
        }
        let retry_app = handle.clone();
        let retry_label = label.clone();
        if let Err(e) = std::thread::Builder::new()
            .name("dsh-dock-refocus".to_string())
            .spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(REFOCUS_DELAY_MS));
                let inner_app = retry_app.clone();
                crate::ui::post_to_event_loop(&retry_app, move || {
                    raise_once(&inner_app, &retry_label);
                });
            })
        {
            tracing::warn!("补焦线程启动失败：{e}");
        }
    });
}

/// 单次唤起：**顺序即正确性**（`unminimize → show → set_focus`，见 [`bring_to_front`]）。
///
/// 返回 `false` = 窗口不存在（已记日志），调用方不必再补焦。
///
/// 句柄解析走 [`crate::ui::window_by_label`]——**禁直呼 `get_webview_window`**：
/// 聊天内嵌的 guest 子视窗挂上后 `is_webview_window` 旗标翻 false，那条查找
/// 路永久 None（2026-10-14 真机事故，详见 ui.rs 该函数的说明）。
fn raise_once(app: &tauri::AppHandle, label: &str) -> bool {
    let Some(win) = crate::ui::window_by_label(app, label) else {
        tracing::warn!("唤起窗口失败：label={label} 不存在（窗口可能已被关闭）");
        return false;
    };
    if win.is_minimized().unwrap_or(false) {
        let _ = win.unminimize();
    }
    let _ = win.show();
    let _ = win.set_focus();
    true
}

/// 壳能力查询（2026-09-21，§3.6）：前端据 `residentEntryAvailable` 决定是否在窗口内补
/// 「关于 / 更新」入口。**只读进程内状态**，不碰文件、不触网。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellCapabilities {
    /// 常驻入口（macOS 菜单栏 / 其余平台托盘）是否可达。false ⇒ 用户没有别的路进「关于 / 更新」。
    pub resident_entry_available: bool,
}

#[tauri::command]
pub fn get_shell_capabilities(app: tauri::AppHandle) -> Result<ShellCapabilities, String> {
    let state = app
        .try_state::<std::sync::Arc<crate::boot::ShellState>>()
        .ok_or_else(|| "壳状态尚未就绪（setup 未完成？）".to_string())?;
    Ok(ShellCapabilities {
        resident_entry_available: state
            .resident_entry_available
            .load(std::sync::atomic::Ordering::SeqCst),
    })
}

/// 打开「关于」窗口（**2026-09-21 恢复**，§3.6 裁定）。
///
/// 历史：原 `open_about` IPC 于 **2026-08-27 删除**，理由是「与原生常驻入口重复」——
/// 该前提在**无 StatusNotifier 宿主的 Linux 桌面**上不成立（托盘建不起来 ⇒ 既看不到更新、
/// 也打不开关于）。故恢复它，并由前端**只在 `residentEntryAvailable === false` 时**渲染入口：
/// 既不重现当年的重复按钮，也不让用户困在没有入口的状态里。
#[tauri::command]
pub fn open_about(app: tauri::AppHandle) {
    crate::ui::open_about_window(&app);
}

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
///
/// 返回 `Result` 而非 `()`（2026-09-21）：前端 `ProfileManager` 的「返回工作台」
/// 已挂 `.catch(showToast)`。句柄解析走 [`crate::ui::main_window`]（2026-10-14
/// 真机：guest 子视窗在场时 `is_webview_window` 旗标翻 false，直呼
/// `get_webview_window` 会让按钮时灵时不灵——详见 ui.rs 事故记录）；三种查找
/// 都落空（窗口真被销毁）才走重建——dsh 侧由 boot 重新导航恢复。
#[tauri::command]
pub fn focus_main_window(app: tauri::AppHandle) -> Result<(), String> {
    if crate::ui::main_window(&app).is_none() {
        let handle = app.clone();
        crate::ui::post_to_event_loop(&app, move || match crate::ui::create_main_window(&handle) {
            Ok(win) => {
                tracing::info!("主窗口缺失，「返回工作台」按重建路径恢复");
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
            Err(e) => tracing::error!("重建主窗口失败：{e}"),
        });
        return Ok(());
    }
    bring_to_front(&app, "main");
    Ok(())
}

#[cfg(test)]
mod tests {
    //! 唤起序闸门（2026-09-21）：顺序即正确性，注释与实现分叉过一次就必须红。

    /// `raise_once` 内三连的顺序必须是 unminimize → show → set_focus：
    /// tao 的 macOS `set_focus()` 在 `is_minimized || !is_visible` 时静默 no-op，
    /// 先 focus 再 unminimize 等于永远 focus 不上（真机现象：点返回工作台不出现）。
    #[test]
    fn raise_once_orders_unminimize_show_focus() {
        let src = include_str!("window.rs");
        let body = src
            .split("fn raise_once")
            .nth(1)
            .expect("raise_once 必须存在");
        let i_min = body.find("unminimize()").expect("缺 unminimize");
        let i_show = body.find(".show()").expect("缺 show");
        let i_focus = body.find("set_focus()").expect("缺 set_focus");
        assert!(i_min < i_show, "unminimize 必须在 show 之前");
        assert!(i_show < i_focus, "show 必须在 set_focus 之前");
    }

    /// 主窗口的三处入口（IPC 命令 / 单实例回调 / 托盘左键）必须都走
    /// [`bring_to_front`]——任何一处回退成裸三连即丢补焦与缺失日志。
    #[test]
    fn main_window_entry_points_share_helper() {
        assert!(include_str!("window.rs").contains("bring_to_front(&app, \"main\")"));
        assert!(include_str!("../lib.rs").contains("bring_to_front(app, \"main\")"));
        assert!(include_str!("../ui.rs").contains("bring_to_front(tray.app_handle(), \"main\")"));
    }

    /// 2026-10-12 真机实测：registry 丢失（`get_webview_window("main")` 为 None）
    /// 而 AX 仍可见窗口时，「返回工作台」若返回 Err，按钮**永远点不动**。故缺失
    /// 分支必须走重建路径，而不是把错误抛给前端 toast。
    #[test]
    fn missing_main_window_rebuilds_instead_of_erroring() {
        let body = include_str!("window.rs");
        let fns = body
            .split("pub fn focus_main_window")
            .nth(1)
            .expect("focus_main_window 必须存在");
        let missing = fns
            .split("bring_to_front(&app, \"main\")")
            .next()
            .expect("函数体必须走到 bring_to_front");
        assert!(
            missing.contains("create_main_window"),
            "主窗口缺失时必须重建（create_main_window），不得返回 Err 让按钮失灵"
        );
        assert!(
            !missing.contains("return Err("),
            "缺失分支不得再返回 Err——2026-10-12 实测这正是「点了没反应」的成因"
        );
    }

    /// 2026-10-14 真机事故（v1.4.0 验收现场）的闸门：Tauri 的
    /// `is_webview_window` 旗标 = 「窗口全部 webview 与窗口同名」，聊天内嵌
    /// `add_child` 挂上 guest 后翻 false ⇒ 直呼 `get_webview_window("main")`
    /// **永久 None**（webview 仍在地图里，重建还撞 already exists）⇒ 三条
    /// 唤起路全灭，「时灵时不灵」的"不灵"就是 guest 在场。故一切找主窗口的
    /// 调用点必须走 `ui::main_window` / `ui::window_by_label`；注释里允许
    /// 引用这个坑（事故记录要留），**代码里直呼即红**。
    #[test]
    fn main_window_lookup_goes_through_the_resolver() {
        for (name, src) in [
            ("window.rs", include_str!("window.rs")),
            ("chat_embed.rs", include_str!("chat_embed.rs")),
            ("lib.rs", include_str!("../lib.rs")),
        ] {
            for line in src.lines() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") {
                    continue; // 注释允许引用（事故记录）
                }
                assert!(
                    !trimmed.contains("get_webview_window(\"main\")"),
                    "{name} 直呼了 get_webview_window(\"main\")——add_child 后旗标翻转必失效，\
                     改走 ui::main_window() / ui::window_by_label()"
                );
            }
        }
        // 解析助手自身必须存在于 ui.rs 且两条查找路都在（get_window 主线 +
        // webviews 枚举退路），防止将来被"简化"回单条查找。
        let ui = include_str!("../ui.rs");
        assert!(
            ui.contains("pub(crate) fn main_window("),
            "ui.rs 缺 main_window 解析助手"
        );
        assert!(
            ui.contains("app.get_window(label)"),
            "window_by_label 必须走 get_window（不受旗标影响的稳定 API）"
        );
        assert!(
            ui.contains("app.webviews().get(label)"),
            "window_by_label 必须保留 webviews 枚举退路"
        );
        // 更新事件的两条目标（main/about）同样必须走解析助手：main 中招时
        // app:update 到不了工作台窗口（v1.4.0 同宗事故的另一个受害面）。
        let updater = include_str!("../updater.rs");
        assert!(
            updater.contains("crate::ui::window_by_label(app, label)"),
            "updater 的 UPDATE_EVENT_TARGETS 发射必须走 ui::window_by_label"
        );
    }
}
