//! ui.rs —— 窗口 / 菜单 / 托盘 / 注入脚本装配层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 职责：主窗口创建与 WebView 初始化脚本装配、应用菜单与托盘菜单构建、
//! 关于窗口与 Profile 管理器窗口的打开、更新态在菜单/托盘上的刷新。
//! 业务与启动管线不在此层——命令在 `commands/`，boot 流程在 `lib.rs::run`（后续再拆）。

use std::path::PathBuf;
use std::sync::Arc;

use tauri::Manager;

use crate::boot::ShellState;
use crate::is_allowed_external_url;

/// 主窗口原生底色 = `frontend/src/index.css` 的 `--color-bg`。
///
/// 「冷启动无闪色」要求首帧 HTML 底色、CSS `--color-bg`、原生窗口
/// `background_color` 三处**逐值一致**。2026-09-10 批次 E 发现这三处长期不一致
/// （HTML/CSS 是 #f7f8fb，本文件是 #f9fafb），注释宣称同调而事实不符。
/// 现由 `background_color_matches_theme_token` 测试逐值锁定——改 index.css 的
/// `--color-bg` 而漏改此处即测试红。
pub(crate) const WINDOW_BACKGROUND: tauri::utils::config::Color =
    tauri::utils::config::Color(241, 244, 249, 255);

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

    // 交接幕布（2026-09-10，ADR-0014）：重启/切换时主窗口要经历两次跨 origin 的
    // 整文档替换（旧工作台 → 壳启动屏 → 新工作台）。壳启动屏那屏由 React 渲染，
    // 两端的工作台页面属于 dsh（源码不可改，红线 1），于是本脚本在 document-start
    // 先画一层与启动屏同构图的幕布：
    //   - 自发现路径：新文档若确为工作台 origin 且壳侧交接仍在途，首帧即亮幕；
    //   - 注入路径：Rust 在 teardown 之前 eval `show(intent,{sticky:true})`，
    //     盖住"进程已死、页面还在"的那几秒。
    // 与 switcher.js 的分工：幕布管"进/出工作台的过渡"，胶囊管"常驻互跳入口"。
    let handoff_curtain_script = include_str!("../../frontend/src/injected/handoff-curtain.js");

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
        // 2026-09-10 批次 E：底色见 WINDOW_BACKGROUND 常量（三处一致性有测试锁定）。
        .background_color(WINDOW_BACKGROUND)
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
        .initialization_script(handoff_curtain_script)
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

/// 当前生效运行模式。优先会话内 active_mode；回落已存默认；再回落 local。
///
/// 消费方：非 macOS 的托盘菜单勾选态（macOS 菜单无「打开方式」）与
/// **全平台**的管理面世界择源（`mgmt::current_world`，ADR-0016 §5-a）——
/// 后者是 macOS 也编译本函数的原因（不再按平台 cfg 掉）。
pub(crate) fn current_active_mode(app: &tauri::AppHandle) -> crate::settings::Mode {
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
    crate::settings::Mode::Local
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
                    if mode == crate::settings::Mode::Local {
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
                    if mode == crate::settings::Mode::Wsl {
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
/// 窗口创建必须在主线程执行（WebView2 在非主线程建窗口会白板），且**必须经
/// [`post_to_event_loop`] 从事件循环投递**——不可就地执行，理由见该函数文档
/// （v1.1.0 Windows 实测 1.3：就地执行会在 WebView2 回调里跑嵌套消息泵而挂死）。
pub(crate) fn open_about_window(app: &tauri::AppHandle) {
    // 闭包要求 Send + 'static，需持有 owned AppHandle；方法调用本身只借 app。
    let handle = app.clone();
    post_to_event_loop(app, move || {
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
        .background_color(WINDOW_BACKGROUND)
        .initialization_script(&platform_script);
        match builder.build() {
            Ok(_) => tracing::info!("关于窗口已创建"),
            Err(e) => tracing::error!("创建关于窗口失败：{e}"),
        }
    });
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

/// 壳页面（启动屏 / 选择器 / 控制中心）的根地址。
///
/// **2026-09-10 修复（v1.1.0 Windows 实测 1.6）**：原实现在 **release 包里也返回
/// dev 服务器地址**——`devUrl` 会被 tauri-utils 的 `BuildConfig::ToTokens` **原样
/// 编译进二进制**（只把 `before_*_command` 置 None），所以 `dev_url.is_some()`
/// 在正式包里同样为真。后果：切 profile / 切模式 / 崩溃自恢复这三条"回壳启动屏"
/// 的路径，全把主窗口导航到**已死的 Vite 端口**（`http://localhost:1420`），
/// 用户在整个启动/重启期间看到的是 `ERR_CONNECTION_REFUSED` 页，直到 dsh 就绪
/// 才被导航回工作台——即"重启时短暂出现 localhost 拒绝连接"。
///
/// 判据改用 `cfg!(dev)`：它由 tauri-build 依据 `DEP_TAURI_DEV`
/// （= `!custom-protocol`）注入，与 Tauri **自己**解析 `WebviewUrl::App` 时用的
/// `#[cfg(dev)]`（`tauri/src/manager/mod.rs::get_app_url`）**同一口径**。
/// 换句话说：原来只有我们这一处与 Tauri 的判断不一致，现在一致了。
pub(crate) fn shell_app_url(app: &tauri::AppHandle) -> tauri::Url {
    shell_app_url_for(
        cfg!(dev),
        app.config().build.dev_url.as_ref().map(|u| u.as_str()),
    )
}

/// `shell_app_url` 的纯函数内核（供测试）：dev 门 + dev URL 解析 + 平台回退。
///
/// `dev=false`（正式包）时**必须忽略** `dev_url`——这正是上面那条事故的修复点。
fn shell_app_url_for(dev: bool, dev_url: Option<&str>) -> tauri::Url {
    if dev {
        if let Some(raw) = dev_url {
            if let Ok(url) = tauri::Url::parse(raw) {
                return url;
            }
        }
    }
    tauri::Url::parse(shell_url_str()).expect("valid shell url")
}

/// 平台相关的壳页面地址字面量（Windows 无自定义 scheme，走虚拟 host 映射）。
fn shell_url_str() -> &'static str {
    if cfg!(windows) {
        "http://tauri.localhost/"
    } else {
        "tauri://localhost/"
    }
}

/// 把「必须由主线程执行」的动作**从事件循环投递**过去——**永不就地执行**。
///
/// ## 为什么不能直接 `app.run_on_main_thread(f)`
///
/// `tauri-runtime-wry` 的 `send_user_message`（`src/lib.rs:235-255`）在当前线程
/// **就是**主线程时会**立即就地执行**闭包（**不是**"排队到下一帧"——本文件原先
/// 关于"从主线程调用只是排队，无害"的注释正是这么误写的）。于是
/// `WebviewWindowBuilder::build()` 可能跑在 WebView2 的**事件回调里**，而 wry 建
/// 控制器时用的是 `webview2_com::wait_with_pump`（`webview2-com/src/lib.rs:60`，
/// **嵌套消息泵**）。WebView2 不支持在自己的回调内重入并跑嵌套泵 → 挂死：窗口
/// 出得来但**没有内容**、关闭请求无人处理、任务管理器标「无响应」。
///
/// ## 症状与此修复的对应（v1.1.0 Windows 实测 1.3）
///
/// 从工作台悬浮胶囊点「控制中心」→ 白窗 + 卡死 + 关不掉；**从托盘点同一入口
/// 正常**。差别正是调用上下文：
/// - 托盘 / 菜单 → `on_menu_event`（tao 事件循环）→ 就地执行也安全；
/// - 悬浮胶囊 → 注入在 **dsh 工作台页**（remote origin）里的 `invoke`。remote
///   页面走 `window.ipc.postMessage` 回退通道（跨源 fetch 到 `ipc.localhost`
///   不可用），该通道的事件处理在 **UI 线程**触发 → 就地执行 → 挂死。
///
/// 因此这里**先跳到一条独立线程**，保证 `run_on_main_thread` 一定走「投递」分支：
/// 窗口创建永远发生在事件循环里，与托盘路径**同上下文**，两条入口行为一致。
///
/// 代价 = 窗口晚一个事件循环轮次出现（用户不可感）；收益 = 不再依赖"调用方恰好
/// 不在主线程"这一脆弱前提。
pub(crate) fn post_to_event_loop<F>(app: &tauri::AppHandle, f: F)
where
    F: FnOnce() + Send + 'static,
{
    let handle = app.clone();
    if let Err(e) = std::thread::Builder::new()
        .name("dsh-dock-ui-post".to_string())
        .spawn(move || {
            if let Err(e) = handle.run_on_main_thread(f) {
                tracing::error!("调度动作到主线程失败：{e}");
            }
        })
    {
        tracing::error!("启动主线程调度线程失败：{e}");
    }
}

#[cfg(test)]
mod window_background_tests {
    use super::WINDOW_BACKGROUND;

    /// 冷启动底色三处一致性闸门（2026-09-10 批次 E）。
    ///
    /// 「冷启动无闪色」= 首帧 HTML 底色 ≡ CSS `--color-bg` ≡ 原生窗口
    /// `background_color`。批次 E 发现三处长期不一致（HTML/CSS #f7f8fb、
    /// 本文件 #f9fafb），注释宣称同调而事实不符。此处从**源码**解析
    /// `--color-bg` 与 index.html 的首帧底色逐值比对——改一处漏改另两处即红。
    fn parse_theme_bg(css: &str) -> (u8, u8, u8) {
        let start = css.find("@theme {").expect("index.css 缺少 @theme 块");
        let end = css[start..].find("\n}").expect("index.css @theme 块未闭合") + start;
        let block = &css[start..end];
        let key = "--color-bg:";
        let at = block.find(key).expect("index.css 缺少 --color-bg");
        let hex = block[at + key.len()..]
            .trim_start()
            .split(|c: char| !c.is_ascii_hexdigit() && c != '#')
            .next()
            .expect("--color-bg 取值解析失败");
        let hex = hex.trim_start_matches('#');
        assert_eq!(hex.len(), 6, "--color-bg 应为 6 位 hex，实际 {hex}");
        let b = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex 解析失败");
        (b(0), b(2), b(4))
    }

    #[test]
    fn background_color_matches_theme_token() {
        let css = include_str!("../../frontend/src/index.css");
        let (r, g, b) = parse_theme_bg(css);
        assert_eq!(
            (
                WINDOW_BACKGROUND.0,
                WINDOW_BACKGROUND.1,
                WINDOW_BACKGROUND.2
            ),
            (r, g, b),
            "ui.rs 的 WINDOW_BACKGROUND 与 index.css 的 --color-bg 不一致——\
             冷启动会闪色。请同步（并检查 frontend/index.html 的首帧底色）"
        );
        assert_eq!(WINDOW_BACKGROUND.3, 255, "窗口底色必须不透明");
    }

    #[test]
    fn index_html_first_paint_matches_theme_token() {
        let css = include_str!("../../frontend/src/index.css");
        let html = include_str!("../../frontend/index.html");
        let (r, g, b) = parse_theme_bg(css);
        let expect = format!("#{r:02x}{g:02x}{b:02x}");
        let at = html.find("html {").expect("index.html 缺少首帧底色样式块");
        let block = &html[at..];
        assert!(
            block.contains(&expect),
            "frontend/index.html 的首帧底色应为 {expect}（与 --color-bg 一致），\
             否则 WebView 首帧会闪色"
        );
    }

    #[test]
    fn theme_bg_parser_rejects_missing_token() {
        // 闸门自检：解析器必须在 token 缺失时 panic，而不是静默返回假值。
        let result =
            std::panic::catch_unwind(|| parse_theme_bg("@theme {\n  --color-ink: #191d27;\n}"));
        assert!(result.is_err(), "缺少 --color-bg 时解析器应报错");
    }

    /// **v1.1.0 Windows 实测 1.6 的回归闸门**：正式包（`dev=false`）**绝不允许**
    /// 返回 dev 服务器地址——那会把主窗口导航到已死的 Vite 端口
    /// （症状：重启/切换期间 `localhost 拒绝连接`）。
    ///
    /// 反例就是线上：`devUrl` 被编译进 release 二进制，`is_some()` 恒真。
    #[test]
    fn release_never_returns_dev_url() {
        let url = super::shell_app_url_for(false, Some("http://localhost:1420"));
        assert_eq!(
            url.as_str(),
            super::shell_url_str(),
            "正式包必须走壳页面地址，而不是 dev 服务器"
        );
        assert!(
            !url.as_str().contains("1420"),
            "正式包泄漏了 dev 端口：{url}"
        );
    }

    /// dev 构建且 devUrl 可用 → 用 dev 服务器（前后端分离开发）。
    #[test]
    fn dev_uses_dev_url_when_available() {
        let url = super::shell_app_url_for(true, Some("http://localhost:1420/"));
        assert!(url.as_str().starts_with("http://localhost:1420"));
    }

    /// dev 构建但 devUrl 缺失/畸形 → 回退壳页面（不 panic、不返回空 URL）。
    #[test]
    fn dev_falls_back_to_shell_url_without_usable_dev_url() {
        for bad in [None, Some("not a url"), Some("")] {
            let url = super::shell_app_url_for(true, bad);
            assert_eq!(
                url.as_str(),
                super::shell_url_str(),
                "devUrl={bad:?} 时应回退壳页面地址"
            );
        }
    }

    /// 平台分叉：Windows 走虚拟 host（WebView2 不支持自定义 scheme），
    /// 其余平台走 `tauri://`。两处必须与 `is_dev()` 的判断方式保持同源。
    #[test]
    fn shell_url_matches_platform_scheme() {
        let s = super::shell_url_str();
        if cfg!(windows) {
            assert_eq!(s, "http://tauri.localhost/");
        } else {
            assert_eq!(s, "tauri://localhost/");
        }
    }

    /// **第四处真相源闸门**（2026-09-10，v1.1.0 实测附带项）：除 `ui.rs` 常量、
    /// `index.css` 的 `--color-bg`、`index.html` 首帧底色之外，**控制中心窗口**
    /// （`commands/window.rs`）也设了自己的 `background_color`——它此前是硬编码的
    /// 旧底色 `(249,250,251)`，而批次 E 已把主题改成 `#f1f4f9`：开窗会闪一下旧色。
    ///
    /// 这条闸门要求该窗口**引用常量**而非自己写一份数值，从根上消灭第四处副本。
    #[test]
    fn profiles_window_reuses_the_single_background_constant() {
        let src = include_str!("commands/window.rs");
        assert!(
            src.contains("crate::ui::WINDOW_BACKGROUND"),
            "控制中心窗口必须引用 WINDOW_BACKGROUND 常量，不得自己硬编码底色"
        );
        // 反例守卫：不得再出现「裸 Color(r, g, b, a)」字面量
        let has_literal = src.lines().any(|l| {
            l.contains("Color(")
                && l.chars().any(|c| c.is_ascii_digit())
                && !l.trim_start().starts_with("//")
        });
        assert!(
            !has_literal,
            "检测到硬编码 Color(...) 字面量——请改用 crate::ui::WINDOW_BACKGROUND"
        );
    }
}
