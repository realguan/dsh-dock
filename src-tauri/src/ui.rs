//! ui.rs —— 窗口 / 菜单 / 托盘 / 注入脚本装配层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 职责：主窗口创建与 WebView 初始化脚本装配、应用菜单与托盘菜单构建、
//! 关于窗口与 Profile 管理器窗口的打开、更新态在菜单/托盘上的刷新。
//! 业务与启动管线不在此层——命令在 `commands/`，boot 流程在 `lib.rs::run`（后续再拆）。

use std::path::PathBuf;
use std::sync::Arc;

use tauri::Manager;

use crate::{is_allowed_external_url, ShellState};

/// 创建主窗口（含外链拦截）。原静态配置（tauri.conf.json windows）等价迁移：
/// 1280x820、min 960x640、可缩放、居中、浅色底。
///
/// 外链策略（2026-08-25 裁定）：
/// - `on_navigation`：白名单外的 http/https 导航 → 系统浏览器打开并拦截（返回 false）；
///   回环 dsh（127.0.0.1）与壳内页面（tauri://）正常放行；
/// - `on_new_window`：`window.open`/target=_blank 一律 Deny 并转系统浏览器
///   （白名单校验同上；非白名单直接丢弃）；
/// - `initialization_script`：兜底捕获 `<a target=_blank>` 点击（部分 WKWebView
///   场景不触发 on_new_window），经 open_external IPC 走同一白名单。
pub(crate) fn create_main_window(app: &tauri::AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    let hook_script = include_str!("../../frontend/src/injected/link-hook.js");

    // WebView 内存与样式兜底策略脚本（ADR-0002，2026-08-26 引入，2026-08-31 修订）：
    // 1. `content-visibility: auto` + `contain-intrinsic-size` 缓解 WebKit 内存膨胀；
    // 2. 补齐列表内边距（`padding-left: 1.5em !important`），避免 `content-visibility: auto`
    //    触发的 Paint Containment 把挂在行左侧外沿（`list-style-position: outside`）
    //    的有序/无序列表序号与圆点裁切掉（2026-08-31 修复）。
    let webview_memory_policy = WEBVIEW_MEMORY_POLICY_SCRIPT;

    // DSH 工作台快捷切换悬浮胶囊与全局快捷键（2026-09-01 引入，零遮挡重构）：
    // 1. 全局监听快捷键（默认 Cmd/Ctrl+, 或配置的快捷键）呼出控制中心；
    // 2. 仅在真正的 DSH 工作台页挂载独立 Shadow DOM 磨砂胶囊——2026-09-08 裁定：
    //    挂载条件由「hostname 回环」收紧为「与 get_workbench_url 返回的 origin
    //    精确比对」。macOS 壳自身页面是 tauri://localhost（hostname 恰为
    //    localhost），旧判断把 selector / 启动页 / 控制中心全部误挂胶囊，
    //    与页面自带顶栏叠出双排「控制中心」；
    // 3. 默认位置：顶部水平正居中（Top-Center），处于 DSH 顶部天然空白留白区，完全避开 Logo 与按钮；
    // 4. 胶囊文案平时仅展示「控制中心」，鼠标 hover 时平滑展开快捷键徽章（强制 nowrap）；
    // 5. 动态监听 app:settings-changed 广播，实时响应开关（即刻消失/挂载）与快捷键切换；
    // 6. 支持鼠标拖拽（Draggable）：用户可随心拖动到任意无遮挡位置。
    let switcher_script = include_str!("../../frontend/src/injected/switcher.js");

    // 运行平台判定注入（2026-08-26 裁定）：WSL 仅存在于 Windows——非 Windows
    // 机器对 WSL 零感知：首次启动不出环境选择页、顶栏无「在 WSL 中打开」、
    // 菜单/托盘无 WSL 项。平台能力经 Rust `cfg!` 编译期判定注入
    // （AGENTS：跨平台语义显式，不用前端猜 UA），随窗口每次页面加载的
    // document-start 生效（主窗口全部壳页共用）。2026-08-27 前端迁移：扩为
    // { os, wsl } 对象（frontend-migration §5.2），os 取 std::env::consts::OS。
    let platform_script = format!(
        "window.__DSH_PLATFORM__ = {{ os: '{}', wsl: {} }};",
        std::env::consts::OS,
        if cfg!(windows) { "true" } else { "false" }
    );

    // 2026-08-27 前端迁移：所有窗口加载 SPA 根路径，React 按窗口 label 路由
    // （frontend-migration §3.1）；子页面经 pathname 可达（get_asset 兜底链）。
    tauri::WebviewWindowBuilder::new(app, "main", tauri::WebviewUrl::App("/".into()))
        .title("DSH Dock")
        .inner_size(1280.0, 820.0)
        .min_inner_size(960.0, 640.0)
        .resizable(true)
        .center()
        .background_color(tauri::utils::config::Color(249, 250, 251, 255))
        .on_navigation(move |url| {
            // 返回 true = 放行导航。壳页面与回环 dsh 放行；其余 http(s) 外链转浏览器。
            //
            // 壳页判定（2026-08-26 修正）：Tauri v2 的 App 内嵌资源在 macOS/Linux 用
            // `tauri://localhost`（scheme=tauri），Windows 用 `http://tauri.localhost`
            // （WebView2 不支持自定义 scheme，走虚拟 host 映射——tauri-utils 源码
            // config.rs 明示 access-control-allow-origin: http://tauri.localhost）。
            // 只按 scheme 判 shell_page 会在 Windows 上把启动页当外链拦掉 → 白屏
            // （实测：Windows 启动白屏直到 dsh 就绪 navigate 到 127.0.0.1 才显示）。
            let shell_page = matches!(url.scheme(), "tauri" | "about" | "data" | "blob")
                || matches!(url.host_str(), Some("tauri.localhost"));
            let loopback_dsh = matches!(
                url.host_str(),
                Some("127.0.0.1") | Some("localhost") | Some("[::1]")
            );
            if shell_page || loopback_dsh {
                return true;
            }
            if matches!(url.scheme(), "http" | "https") {
                let allowed = is_allowed_external_url(url.as_str());
                tracing::info!("外链导航拦截：url={url} allowed={allowed}");
                if allowed {
                    if let Err(e) = open::that_detached(url.as_str()) {
                        tracing::error!("外链打开失败：{e}");
                    }
                }
                // 非白名单：既不导航也不打开（壳不成为任意跳板）。
            } else {
                tracing::info!("未知协议导航拦截：{url}");
            }
            false
        })
        .on_new_window(move |url, _features| {
            // 新窗口请求（window.open / target=_blank）：一律拒绝，白名单内转浏览器。
            let allowed = is_allowed_external_url(url.as_str());
            tracing::info!("新窗口请求：url={url} allowed={allowed}");
            if allowed {
                if let Err(e) = open::that_detached(url.as_str()) {
                    tracing::error!("外链打开失败（新窗口路径）：{e}");
                }
            }
            tauri::webview::NewWindowResponse::Deny
        })
        .initialization_script(&platform_script)
        .initialization_script(hook_script)
        .initialization_script(webview_memory_policy)
        .initialization_script(switcher_script)
        .build()
}

/// 定位含 product.manifest.json 的资源根（dev/prod 布局差异见 setup 注释）。
pub(crate) fn resolve_resources_dir<M: tauri::Manager<tauri::Wry>>(app: &M) -> PathBuf {
    // dev 模式（debug_assertions）：源码树 resources 为最高优先级真理源，
    // 避免 target/debug 复制产物因构建时差或增量缺失导致资源不全。
    #[cfg(debug_assertions)]
    {
        let src_res = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
        if src_res.join("product.manifest.json").is_file() {
            return src_res;
        }
    }
    let runtime = app.path().resource_dir().ok().unwrap_or_default();
    // 生产（bundle）：Tauri v2 打包器保留相对 src-tauri 的路径前缀——
    // 配置 `"resources": ["resources/**"]` 时，文件实际落在 `<资源根>/resources/`
    // 下（打包 e2e 实测，2026-08-21）。优先探测嵌套布局。
    let bundled = runtime.join("resources");
    if bundled.join("product.manifest.json").is_file() {
        return bundled;
    }
    // 兼容平铺布局：不同 bundler 版本/配置可能把资源直接放在资源根。
    if runtime.join("product.manifest.json").is_file() {
        return runtime;
    }
    // dev 回退链（Windows 语义的 tauri-build 副本 → 源码树，本仓库开发常态）。
    let exe_res = app
        .path()
        .executable_dir()
        .ok()
        .unwrap_or_default()
        .join("resources");
    if exe_res.join("product.manifest.json").is_file() {
        return exe_res;
    }
    let src_res = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    if src_res.join("product.manifest.json").is_file() {
        return src_res;
    }
    // 全落空：返回运行时路径，让契约读取给出可行动错误（A6）。
    runtime
}

/// 当前生效运行模式（托盘菜单 ✓ 用；仅非 macOS——macOS 菜单无「打开方式」）。
/// 优先会话内 active_mode；回落已存默认；再回落 local。
#[cfg(not(target_os = "macos"))]
pub(crate) fn current_active_mode(app: &tauri::AppHandle) -> settings::Mode {
    if let Some(state) = app.try_state::<Arc<ShellState>>() {
        if let Some(m) = *state.active_mode.lock().unwrap() {
            return m;
        }
    }
    if let Ok(data_dir) = app.path().app_data_dir() {
        if let Some(m) = crate::settings::load(&data_dir).default_mode {
            return m;
        }
    }
    settings::Mode::Local
}

/// 组装应用菜单：macOS 菜单栏结构 = 根菜单内放「App 子菜单」+「编辑」子菜单。
/// 第一个子菜单被 macOS 自动视为 App 菜单（标题取 app 名、图标取 bundle 图标）——
/// 平铺 MenuItem 会导致菜单栏出现齿轮占位图标（2026-08-23 实测）。
/// App 菜单内容（2026-08-26 裁定：更新项统一收进「关于」更新中心，菜单只留
/// 「在浏览器中打开」「关于」+ 标准项）：在浏览器中打开 → 关于 → 标准项。
#[cfg(target_os = "macos")]
pub(crate) fn build_app_menu(
    app: &tauri::AppHandle,
) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{MenuBuilder, MenuItem, PredefinedMenuItem, SubmenuBuilder};

    let about = MenuItem::with_id(app, "about", "关于", true, None::<&str>)?;
    let in_browser =
        MenuItem::with_id(app, "open_in_browser", "在浏览器中打开", true, None::<&str>)?;
    let profiles_manager =
        MenuItem::with_id(app, "profiles_manager", "控制中心", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;

    // 非 macOS 之外无「打开方式」子菜单（2026-08-26 裁定）：本函数仅 macOS
    // 编译，WSL 只存在于 Windows——本地是唯一环境，无可切换，菜单不出现 WSL 字样。

    // App 子菜单（macOS 忽略其 text，标题自动为 app 名）
    let app_menu = SubmenuBuilder::new(app, "dsh-dock")
        .item(&in_browser)
        .item(&profiles_manager)
        .item(&sep)
        .item(&about)
        .item(&sep)
        .item(&PredefinedMenuItem::services(app, None)?)
        .item(&sep)
        .item(&PredefinedMenuItem::hide(app, None)?)
        .item(&PredefinedMenuItem::hide_others(app, None)?)
        .item(&PredefinedMenuItem::show_all(app, None)?)
        .item(&sep)
        .item(&PredefinedMenuItem::quit(app, None)?)
        .build()?;

    // 编辑子菜单（WebView 文本编辑可用）
    let edit_menu = SubmenuBuilder::new(app, "编辑")
        .item(&PredefinedMenuItem::undo(app, None)?)
        .item(&PredefinedMenuItem::redo(app, None)?)
        .item(&sep)
        .item(&PredefinedMenuItem::cut(app, None)?)
        .item(&PredefinedMenuItem::copy(app, None)?)
        .item(&PredefinedMenuItem::paste(app, None)?)
        .item(&PredefinedMenuItem::select_all(app, None)?)
        .build()?;

    MenuBuilder::new(app)
        .item(&app_menu)
        .item(&edit_menu)
        .build()
}

/// 用最近状态重建并设置应用菜单（检测完成/升级后调用）。
/// 仅 macOS 有系统菜单栏；其余平台不设原生菜单（窗口内菜单条丑，2026-08-24 裁定）。
/// 2026-08-26 起菜单不再含更新项（收进「关于」更新中心），菜单内容不随更新状态变化，
/// 但仍保留重建入口以维持调用方一致（幂等：重复设置同构菜单无害）。
#[cfg(target_os = "macos")]
pub(crate) fn refresh_app_menu(app: &tauri::AppHandle, _state: &Arc<ShellState>) {
    if let Ok(menu) = build_app_menu(app) {
        let _ = app.set_menu(menu);
    }
}

/// 非 macOS：常驻入口 = 系统托盘（2026-08-24 裁定：Windows/Linux 原生菜单
/// 会渲染成窗口内菜单条，丑；托盘菜单承载 在浏览器中打开/打开方式/关于/退出）。
/// 2026-08-26 起更新项收进「关于」更新中心，托盘菜单不再随更新状态变化；
/// 「打开方式」勾选态随当前模式变化，故切换模式后仍需重建。
#[cfg(not(target_os = "macos"))]
pub(crate) fn refresh_app_menu(app: &tauri::AppHandle, _state: &Arc<ShellState>) {
    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(menu) = build_tray_menu(app) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

/// 托盘菜单：在浏览器中打开 / 打开方式（local·wsl，仅 Windows）/ 关于 / 退出。
/// 事件经 builder 级 on_menu_event（全局）送达，id 与 macOS 菜单一致：
/// open_in_browser / mode_local / mode_wsl / about / quit。
#[cfg(not(target_os = "macos"))]
pub(crate) fn build_tray_menu(
    app: &tauri::AppHandle,
) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{MenuBuilder, MenuItem, PredefinedMenuItem};

    let about = MenuItem::with_id(app, "about", "关于", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let in_browser =
        MenuItem::with_id(app, "open_in_browser", "在浏览器中打开", true, None::<&str>)?;
    let profiles_manager =
        MenuItem::with_id(app, "profiles_manager", "控制中心", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;

    // 「打开方式」仅 Windows（WSL 只存在于 Windows，2026-08-26 裁定）：
    // Linux 上本地是唯一环境，无可切换——托盘菜单不出现 WSL 字样。
    // 条目在函数级持有（MenuBuilder.items 只借引用，跨 if 块引用会悬垂）。
    let mode = current_active_mode(app);
    let local_item = cfg!(windows)
        .then(|| {
            MenuItem::with_id(
                app,
                "mode_local",
                format!(
                    "打开方式：本地{}",
                    if mode == settings::Mode::Local {
                        " ✓"
                    } else {
                        ""
                    }
                ),
                true,
                None::<&str>,
            )
        })
        .transpose()?;
    let wsl_item = cfg!(windows)
        .then(|| {
            MenuItem::with_id(
                app,
                "mode_wsl",
                format!(
                    "打开方式：WSL2{}",
                    if mode == settings::Mode::Wsl {
                        " ✓"
                    } else {
                        ""
                    }
                ),
                true,
                None::<&str>,
            )
        })
        .transpose()?;

    let mut entries: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> = Vec::new();
    entries.push(&in_browser);
    entries.push(&profiles_manager);
    if let (Some(l), Some(w)) = (local_item.as_ref(), wsl_item.as_ref()) {
        entries.push(l);
        entries.push(w);
    }
    for it in [
        &sep as &dyn tauri::menu::IsMenuItem<tauri::Wry>,
        &about,
        &sep,
        &quit,
    ] {
        entries.push(it);
    }

    MenuBuilder::new(app).items(&entries).build()
}

/// setup 阶段创建托盘（非 macOS）：左键唤起主窗口，右键出菜单。
#[cfg(not(target_os = "macos"))]
pub(crate) fn setup_update_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let menu = build_tray_menu(app)?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".into()))?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("DSH Dock")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            // 左键/双击唤起主窗口（托盘常驻：窗口可关，任务台仍在）
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(win) = tray.app_handle().get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.unminimize();
                    let _ = win.set_focus();
                }
            }
        })
        .build(app)?;
    Ok(())
}

/// 关于面板：独立小窗（壳版本 + 宿主 dsh 版本 + 检查/升级），
/// React 首页面按窗口 label 渲染 frontend/src/pages/About.tsx。
///
/// 2026-08-26 修复（issue #2）：窗口创建必须在主线程执行。Tauri 的
/// `#[tauri::command]` handler 跑在 IPC 线程（async runtime），Windows/WebView2
/// 上在非主线程 `WebviewWindowBuilder::build()` 会导致 WebView2 环境初始化失败、
/// 窗口白板（顶栏「关于」空白；托盘路径在主事件循环故正常）。统一经
/// `run_on_main_thread` 序列化到主线程——从主线程调用时只是排队到下一帧，无害。
pub(crate) fn open_about_window(app: &tauri::AppHandle) {
    // run_on_main_thread 的闭包要求 Send + 'static，需持有一个 owned AppHandle；
    // 方法调用本身只借 app（参数引用），闭包 move 走 clone——二者不冲突。
    let handle = app.clone();
    if let Err(e) = app.run_on_main_thread(move || {
        if let Some(win) = handle.get_webview_window("about") {
            let _ = win.show();
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
            "about",
            // 2026-08-27 前端迁移：与主窗口同载 SPA 根，React 按 label=about 渲染
            tauri::WebviewUrl::App("/".into()),
        )
        .title("关于")
        // 480x360 装不下三行维度 + 浏览器入口 + 脚注（2026-08-25 实测裁切）；
        // 更新中心接入后内容更高（客户端状态机卡片 + dimrows + 入口 + 脚注），
        // 加高并允许滚动兜底。
        .inner_size(480.0, 580.0)
        .min_inner_size(440.0, 480.0)
        .resizable(true)
        .center()
        .initialization_script(&platform_script);
        match builder.build() {
            Ok(_) => tracing::info!("关于窗口已创建"),
            Err(e) => tracing::error!("创建关于窗口失败：{e}"),
        }
    }) {
        tracing::error!("调度关于窗口创建到主线程失败：{e}");
    }
}

/// WebView 渲染内存与样式兜底策略（ADR-0002，2026-08-25 提出，2026-08-26 CSS 注入，2026-08-31 列表裁切与直接子代嵌套修复）：
/// 1. `content-visibility: auto` + `contain-intrinsic-size` 缓解 WebKit 内存膨胀；
/// 2. 补齐列表内边距（`padding-left: 1.5em !important`），避免 `content-visibility: auto`
///    触发的 Paint Containment 把挂在行左侧外沿（`list-style-position: outside`）
///    的有序/无序列表序号与圆点裁切掉（2026-08-31 修复）；
/// 3. 使用直接子代选择器（`FLOW > ROW`）：仅对顶层会话行生效，防止规则穿透到嵌套
///    的工具调用节点（`ToolCallTree` 的 `callRow` / `subCalls` 也挂有 `data-chat-anchor-key`），
///    避免多层嵌套 containment 导致 WebKit 严重虚高估算滚动高度、在底部产生大片空白
///    滚动区及输入框悬空（2026-08-31 修复）。
pub const WEBVIEW_MEMORY_POLICY_SCRIPT: &str =
    include_str!("../../frontend/src/injected/memory-policy.js");
