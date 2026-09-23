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

    // DSH 工作台快捷切换悬浮胶囊与应用内快捷键（2026-09-01 引入，零遮挡重构）：
    // 1. 页内 keydown 监听（默认 Cmd/Ctrl+, 或配置的快捷键）呼出控制中心——
    //    **非 OS 级全局热键**（`tao` 未启 global_shortcut、无 global-shortcut 插件；
    //    ADR-0024 已裁"暂缓"）⇒ 窗口失焦时无效，文案不得写成"全局"（审计 §3.5）；
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

    // 沉浸式标题栏（2026-09-21，ADR-0029；拖拽机制 2026-09-23 改版，见该档 §7）：官方客户端的
    // 沉浸式 chrome 真相源在 dsh 自己的 web 前端（`packages/client` 里按
    // `html[data-platform='darwin']` 生效的一整批桌面 CSS：透明底、侧栏 tint、顶栏与红绿灯
    // 共行；源锚见 ADR-0029 §1 表与 §7.3）。壳里它们休眠，本脚本补两件事：① 补打 dsh 官方
    // Electron preload 同款标记；② **按几何语义自驱窗口拖拽**（读 dsh 自己发布的
    // `data-shell-leading-band` 钩子 + 它的交互元素排除表 ⇒ `startDragging()`）。
    // 注意：不要再回到「翻译 `-webkit-app-region`」那条路 —— WKWebView 不认该属性，
    // 2026-09-23 已实测证伪（ADR-0029 §7.1）。仅 macOS 生效（与下方窗口配置的 cfg 同门），
    // 只认工作台 origin，拿不到拖拽带即静默降级（用顶部 52px 兜底几何带）。
    let immersive_script = include_str!("../../frontend/src/injected/immersive-chrome.js");

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
    // 2026-09-21 ADR-0029：`mut` 仅供下方 macOS 段的 overlay/vibrancy 重新赋值——
    // 非 macOS 目标该段被 cfg 掉，`mut` 看似多余；按目标显式 allow，避免三平台
    // clippy（-D warnings）在 Win/Linux 上被 unused_mut 炸掉。
    #[cfg_attr(not(target_os = "macos"), allow(unused_mut))]
    let mut builder =
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
            .initialization_script(switcher_script)
            .initialization_script(handoff_curtain_script);

    // 沉浸式标题栏（ADR-0029）——仅 macOS：overlay 标题栏（红绿灯浮在内容左上角、
    // 原生标题文字隐去）+ Sidebar 材质 vibrancy（对标官方 Electron 的
    // `vibrancy:'sidebar'` + `visualEffectState:'active'`，main.ts:125-132）。
    // 窗口底色**保持不透明**（WINDOW_BACKGROUND 三处一致性不动）：毛玻璃靠 dsh
    // 工作台页面自身的 `html[data-platform='darwin']{background:transparent}` 透出，
    // 壳页面（启动屏等）仍画不透明底，首帧不闪色口径不变。
    // Windows/Linux 无红绿灯且 Tauri 稳定版无 titleBarOverlay 等价物 → 保持原生装饰。
    //
    // 2026-09-21：上面这句"原生标题文字隐去"原先只是**注释里的承诺**——代码只设了
    // `title_bar_style(Overlay)`，而 Overlay 仅让红绿灯浮起，**不会**隐去 `.title()`
    // 的文字，于是红绿灯旁边长期挂着一行「DSH Dock」（维护者截图圈出）。补上真正
    // 隐去它的那一项：`hidden_title(true)`（macOS-only）。注意**窗口标题本身不动**
    // ——它仍供窗口切换器/任务栏/关于弹窗使用，这里只关掉标题栏里的那份渲染。
    #[cfg(target_os = "macos")]
    {
        builder = builder
            .title_bar_style(tauri::TitleBarStyle::Overlay)
            .hidden_title(true)
            .effects(
                tauri::window::EffectsBuilder::new()
                    .effects([tauri::window::Effect::Sidebar])
                    .state(tauri::window::EffectState::Active)
                    .build(),
            );
    }

    // Windows 沉浸式标题栏：2026-09-22 进场（`73c6af1`），**2026-09-23 维护者裁定撤回**
    // （ADR-0030）。此处保留撤回理由，避免后人「照官方方案」再加一次：
    //   ① **等价映射不成立**：官方 Electron 的 `titleBarStyle:'hidden'` 保留原生 frame
    //      （可缩放 + 可贴靠），而 Tauri 的 `decorations(false)` 在 tao 里会一并摘掉
    //      `WS_CAPTION | WS_THICKFRAME`（`tao-0.35.3/src/platform_impl/windows/window_state.rs:307`）
    //      ⇒ 鼠标缩放边框与 Aero Snap 同时消失（该提交自己写的验收判据「缩放/贴靠退化即回退」
    //      当场命中）；
    //   ② **拖拽条挂不上属性**：dsh 的 40px 标题栏带是伪元素
    //      （`AppFrame.module.css:40-46` 的 `.frame::before { -webkit-app-region: drag }`），
    //      伪元素无法承载 `data-tauri-drag-region`——这正是 ADR-0029 §6 要求「另立评审」的那一项，
    //      进场时没走 ⇒ 窗口完全拖不动；
    //   ③ **自绘三控件静默失效**：`minimize` / `toggle_maximize` / `close` 不在
    //      `core:window:default` 里，而 capabilities 只授了 `core:default` ⇒ ACL 拒绝 + 调用点
    //      `.catch(function () {})` 吞掉 = 用户侧「点了没反应」（issue #16：移动 / 最大 / 最小 /
    //      关闭四症状）。
    // 现状 = Windows 与 Linux 同口径：**维持原生装饰**。闸门
    // `windows_native_decorations_tests` 钉住（含反例方向）；要重开沉浸式档 = 先立 ADR，
    // 并逐条解决上述三点（真机验收不可省）。
    //
    // 2026-09-23 裁定：`decorations(false)` 不得再按平台加回本文件。

    let window = builder.initialization_script(immersive_script).build()?;

    // 红绿灯定位（ADR-0029）：目标 = 与侧边栏收起按钮（dsh darwin topStrip 的 toggle，
    // 52px 条带垂直居中）同一水平线，即官方 Electron 的 trafficLightPosition x16/y18。
    // 建窗后先落一次（主线程、未 show，用户看不到跳变）。
    // 调用点三平台同构：函数内部按 target_os 分叉（非 macOS = no-op）。
    crate::traffic_lights::align(&window);

    // 落位不是一次性的：探针实测 AppKit 在 resize 时把标准按钮弹回默认位（show 不会）。
    // 且 reset 可能发生在**不发 Tauri 窗口事件**的时机（tao 建窗后的内部 setFrame、
    // vibrancy 视图插入等）——所以三管齐下：
    //   ① 任何窗口事件都重放（align 内部幂等 + 变化检测，已就位即静默，不刷屏）；
    //   ② setup 结束后经主线程事件循环补一枪（覆盖"建窗后内部 reset 无事件"）；
    //   ③ 首次落位打 info 并读回帧坐标——实机核对"到底生效没有"以日志为凭据。
    #[cfg(target_os = "macos")]
    {
        let lights = window.clone();
        window.on_window_event(move |event| {
            if !matches!(event, tauri::WindowEvent::Destroyed) {
                crate::traffic_lights::align(&lights);
            }
        });
        let lights = window.clone();
        let _ = app.run_on_main_thread(move || crate::traffic_lights::align(&lights));
    }

    // 「关窗 = 隐藏」而非销毁（2026-09-21，修维护者报的「关掉工作台窗口后，点控制台的
    // 『返回工作台』没反应」）：
    //
    // 根因：窗口被 CloseRequested 默认**销毁**后，`get_webview_window("main")` 返回
    // `None` ⇒ 三处唤回入口（控制台「返回工作台」IPC / 托盘左键 / 二次启动）全部落进
    // "窗口不存在"分支 —— 用户侧表现就是**点了没反应**（此前连日志都没有）。
    // 隐藏则窗口与工作台都保留（dsh 子进程不受影响），与 README「窗口可关，任务台仍在」
    // 一致；退出仍走托盘 / 菜单栏的「退出」，语义不变。
    //
    // 与既有 macOS 红绿灯对齐监听**并存**：Tauri 的 `on_window_event` 是逐条
    // `AddEventListener`（tauri-runtime-wry `lib.rs:1986`），不是覆盖式单槽。
    {
        let win = window.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = win.hide();
                tracing::info!("主窗口 CloseRequested → 隐藏（窗口与工作台进程均保留）");
            }
        });
    }

    Ok(window)
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
    // 「显示工作台」（2026-09-21）：主窗口改成「关窗 = 隐藏」后，菜单栏必须有一个显式
    // 唤回入口 —— macOS 无托盘，不能只靠"再点一次 Dock"（Dock 走 `RunEvent::Reopen`）。
    let show_main = MenuItem::with_id(app, "show_main", "显示工作台", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;

    // 非 macOS 之外无「打开方式」子菜单（2026-08-26 裁定）：本函数仅 macOS
    // 编译，WSL 只存在于 Windows——本地是唯一环境，无可切换，菜单不出现 WSL 字样。

    // App 子菜单（macOS 忽略其 text，标题自动为 app 名）
    let app_menu = SubmenuBuilder::new(app, "dsh-dock")
        .item(&show_main)
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
    let show_main = MenuItem::with_id(app, "show_main", "显示工作台", true, None::<&str>)?;
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
    // 「显示工作台」（2026-09-21）：主窗口改成「关窗 = 隐藏」后，托盘右键必须能唤回
    // （左键唤起是快路径，但菜单项才可发现）。事件走同一个 on_menu_event 分派。
    entries.push(&show_main);
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

/// 托盘运行期依赖的候选 soname（**顺序与 `libappindicator-sys` 0.9.0 一致**，
/// 见该 crate `src/lib.rs:14,19,31,36`）。
///
/// **契约方向**：本表必须 ⊇ 该 crate 会尝试的名单——它加载成功而我们漏探 ⇒ 白崩；
/// 我们多探到它不认的名字 ⇒ 只是跳过托盘（安全方向）。升级该 crate 时须复核本表
/// （`ui.rs` 的 `tray_runtime_libs_cover_libappindicator_candidates` 钉住条目）。
///
/// 名单在 macOS/Windows 上无运行期消费者（探测只对 Linux 有意义），但**契约测试**
/// 要用它，故保留并显式豁免 dead_code。
#[allow(dead_code)]
const TRAY_RUNTIME_LIBS: [&str; 4] = [
    "libayatana-appindicator3.so.1",
    "libappindicator3.so.1",
    "libayatana-appindicator3.so",
    "libappindicator3.so",
];

/// 建托盘**之前**的运行期依赖探测（2026-09-21，平台审计 A1 / AGENTS §0 红线 3）。
///
/// 为什么必须自己做：`tray-icon` 在 Linux 经 `libappindicator-sys` 用 `libloading`
/// **dlopen** 系统库，四个候选名全失败即 `panic!`；本仓 release 档 `panic = "abort"`
/// ⇒ **启动即崩**（用户什么都看不到），而 `lib.rs` 的 `if let Err` 与 `catch_unwind`
/// 都拦不住 abort —— 只能在建托盘前自己先加载一次。
///
/// 探测失败 ⇒ 返回**可行动**错误（含包名与两条发行版命令）⇒ 跳过托盘、应用照常启动。
/// 不静默：原因由 `lib.rs` 落 `warn` 日志（ADR-0007「应用不可用不可接受」边界不变）。
#[cfg(target_os = "linux")]
fn check_tray_runtime_libs() -> Result<(), String> {
    for name in TRAY_RUNTIME_LIBS {
        // SAFETY: 仅 dlopen/dlclose 探测命中与否，不解析、不调用任何符号。
        if unsafe { libloading::Library::new(name) }.is_ok() {
            return Ok(());
        }
    }
    Err(format!(
        "托盘不可用：系统缺少 ayatana/appindicator 动态库（已尝试 {names}）。\
         影响：托盘图标与其上的「关于 / 检查更新」入口缺失，其余功能不受影响。\
         补齐：Debian/Ubuntu 执行 `sudo apt install libayatana-appindicator3-1`；\
         Fedora 执行 `sudo dnf install libayatana-appindicator-gtk3`。",
        names = TRAY_RUNTIME_LIBS.join(" / ")
    ))
}

/// Windows：托盘走 Win32 原生路径，无 dlopen 依赖 ⇒ 恒可用。
#[cfg(all(not(target_os = "macos"), not(target_os = "linux")))]
fn check_tray_runtime_libs() -> Result<(), String> {
    Ok(())
}

/// setup 阶段创建托盘（非 macOS）：左键唤起主窗口，右键出菜单。
///
/// 返回 `Err` 时调用方只记日志、不阻断启动（ADR-0007 边界：常驻入口缺失可接受，
/// 应用不可用不可接受）。错误串本身承载**可行动原因**，故此处不做二次包装。
#[cfg(not(target_os = "macos"))]
pub(crate) fn setup_update_tray(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    // 必须**先探测后建**：顺序颠倒即回到"缺库 panic ⇒ 启动即崩"的老路
    // （`tray_dependency_is_probed_before_building_the_tray` 钉住本行位置）。
    check_tray_runtime_libs()?;

    let menu = build_tray_menu(app).map_err(|e| format!("构建托盘菜单失败：{e}"))?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| "缺少默认窗口图标：托盘无法建立".to_string())?;

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
                // 唤起序走 bring_to_front（同「返回工作台」修复：tao 的 macOS
                // set_focus 在 minimized||!visible 时静默 no-op，2026-09-21）。
                crate::commands::window::bring_to_front(tray.app_handle(), "main");
            }
        })
        .build(app)
        .map_err(|e| format!("建立托盘图标失败：{e}"))?;
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

    /// 沉浸式标题栏（ADR-0029）的**实现**闸门。
    ///
    /// 事故形态：注释一直写着「原生标题文字隐去」，代码却只设了
    /// `title_bar_style(Overlay)`——Overlay 仅让红绿灯浮到内容上，**不会**隐去
    /// `.title()` 的文字，于是红绿灯旁边长期挂着一行「DSH Dock」（维护者截图圈出）。
    /// 这是"注释承诺 ≠ 代码事实"的典型，靠人读注释永远发现不了。
    ///
    /// 断言从**生产段源码**取（截到测试模块之前），避免测试自身的字面量把闸门
    /// 变成永远为真——同 `lifecycle::tests::production_code_view_truncates_at_the_real_test_module`。
    #[test]
    fn macos_titlebar_hides_native_title_text() {
        let src = include_str!("ui.rs");
        let production = src
            .split("mod window_background_tests")
            .next()
            .expect("ui.rs 应含 window_background_tests 模块");
        assert!(
            production.contains("TitleBarStyle::Overlay"),
            "macOS 应使用 overlay 标题栏（红绿灯浮在内容左上角）"
        );
        assert!(
            production.contains("hidden_title(true)"),
            "macOS 段必须 `hidden_title(true)`：Overlay 只让红绿灯浮起，\
             **不会**隐去 `.title()` 的文字——漏掉它，红绿灯旁会一直挂着一行窗口名"
        );
        assert!(
            production.contains(r#".title("DSH Dock")"#),
            "窗口标题本身要保留（窗口切换器/任务栏/关于弹窗仍用它），\
             被隐藏的只是标题栏里的那份渲染"
        );
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

/// 沉浸式标题栏（ADR-0029）注入脚本的**内容契约闸门**。
///
/// 脚本跑在 dsh 工作台文档里、无打包器、无单测环境，Rust 侧只能按内容钉契约。
/// 钉的是四件「被误删也不会立刻炸、但沉浸式会静默死掉」的事：
///   ① 双重注入 guard（同文档重入防护，与既有脚本同款）；
///   ② macOS 平台门（v1 仅 macOS；Win/Linux 保持原生装饰，与窗口配置的 cfg 同门）；
///   ③ 标记值 `darwin`（必须与 dsh 官方 preload 打的同款，见 ADR-0029 §1）；
///   ④ 拖拽机制要件（2026-09-23 改版后：几何语义 —— 读 dsh 的拖拽带钩子 + 命中测试；
///      旧「app-region → `data-tauri-drag-region`」键名已作废，见 ADR-0029 §7）。
#[cfg(test)]
mod immersive_chrome_tests {
    #[test]
    fn immersive_script_carries_its_contract() {
        let src = include_str!("../../frontend/src/injected/immersive-chrome.js");
        // 只有**生效代码**才算数：注释里为说明历史而引用旧键名会干扰纯 contains 断言
        // （本闸门 2026-09-23 就栽过一次 —— 键名只剩在注释里，断言照样绿）。
        let code = src
            .lines()
            .filter(|line| {
                let trimmed = line.trim_start();
                !(trimmed.starts_with("//")
                    || trimmed.starts_with("/*")
                    || trimmed.starts_with('*'))
            })
            .collect::<Vec<_>>()
            .join("\n");
        for needle in [
            "window.__dshDockImmersiveInjected", // ① 重入 guard
            "platform.os !== 'macos'",           // ② v1 平台门
            "dataset.platform",                  // ③ 补打 dsh 官方标记的落点
            "'darwin'",                          // ③ 标记值（= DSH_PLATFORM_MARKER）
            "data-shell-leading-band",           // ④ 拖拽带钩子（dsh 发布）
            "elementFromPoint",                  // ④ 命中测试：决定拖动还是点击
        ] {
            assert!(
                code.contains(needle),
                "immersive-chrome.js 的生效代码缺少契约要素 `{needle}`——ADR-0029 的机制\
                 会因此静默失效（工作台回退原生标题栏 / 窗口拖不动）。若确有重构，\
                 请同步本闸门与 ADR。"
            );
        }
    }

    /// 主窗口注入链必须挂上沉浸式脚本（接线闸门：只进主窗口，控制中心窗口
    /// label=profiles 是壳页面，不得挂——它由 `commands/window.rs` 单独创建）。
    #[test]
    fn main_window_wires_immersive_script() {
        let src = include_str!("ui.rs");
        assert!(
            src.contains(".initialization_script(immersive_script)"),
            "主窗口注入链缺少 immersive_script——ADR-0029 的唤醒步（①）没接上"
        );
        // 反例守卫：控制中心窗口不得挂沉浸式脚本（壳页面无 dsh 桌面 CSS，挂了也是空转）。
        let window_rs = include_str!("commands/window.rs");
        assert!(
            !window_rs.contains("immersive"),
            "控制中心窗口（壳页面）不应引用沉浸式脚本"
        );
    }
}

/// 托盘运行期依赖（平台审计 A1，2026-09-21）：Linux 缺库 ⇒ crate 内 `panic!`
/// ⇒ `panic = "abort"` ⇒ 启动即崩。本组用例钉住"先探测后建"与"包声明"两条防线。
#[cfg(test)]
mod tray_runtime_dependency_tests {
    use super::TRAY_RUNTIME_LIBS;

    /// 名单覆盖 `libappindicator-sys` 0.9.0 的四个候选（该 crate `src/lib.rs:14,19,31,36`）。
    ///
    /// 升级该 crate 若改了名单，本用例会红 —— 提醒按"⊇ crate 名单"契约复核。
    #[test]
    fn tray_runtime_libs_cover_libappindicator_candidates() {
        assert_eq!(
            TRAY_RUNTIME_LIBS,
            [
                "libayatana-appindicator3.so.1",
                "libappindicator3.so.1",
                "libayatana-appindicator3.so",
                "libappindicator3.so",
            ],
            "探测名单与 libappindicator-sys 的候选不一致：少了它会崩、多了只是跳托盘"
        );
    }

    /// **顺序契约**：探测必须在建托盘之前（颠倒即回到"缺库 panic ⇒ 启动即崩"）。
    #[test]
    fn tray_dependency_is_probed_before_building_the_tray() {
        let src = include_str!("ui.rs").replace("\r\n", "\n");
        let probe = src
            .find("check_tray_runtime_libs()?;")
            .expect("setup_update_tray 里找不到前置探测调用——缺库平台会退回 panic 崩溃路径");
        let build = src
            .find("TrayIconBuilder::with_id(\"main\")")
            .expect("找不到托盘创建点");
        assert!(
            probe < build,
            "探测必须**先于**建托盘执行（当前 probe@{probe} build@{build}）"
        );
    }

    /// 包声明契约：deb / rpm 都必须把该库声明为 `Recommends`。
    ///
    /// 为什么是 Recommends 而不是 Depends：`libayatana-appindicator3-1` 在 Ubuntu
    /// 属 **universe**（2026-09-21 实查 packages.ubuntu.com）——硬依赖会让关掉
    /// universe 的机器**根本装不上包**，比丢托盘严重；Recommends 在 apt/dnf 下
    /// 默认安装、仓库里没有也不阻断安装，与"应用照常启动"的降级配合。
    #[test]
    fn linux_packages_recommend_the_tray_library() {
        let conf: serde_json::Value =
            serde_json::from_str(include_str!("../tauri.conf.json")).expect("tauri.conf.json 非法");
        let linux = conf
            .get("bundle")
            .and_then(|b| b.get("linux"))
            .expect("bundle.linux 缺失：deb/rpm 将不声明任何托盘依赖");
        for (format, pkg) in [
            ("deb", "libayatana-appindicator3-1"),
            ("rpm", "libayatana-appindicator-gtk3"),
        ] {
            let list = linux
                .get(format)
                .and_then(|f| f.get("recommends"))
                .and_then(|r| r.as_array())
                .unwrap_or_else(|| panic!("bundle.linux.{format}.recommends 缺失"));
            assert!(
                list.iter().any(|v| v.as_str() == Some(pkg)),
                "bundle.linux.{format}.recommends 必须含 {pkg}（当前 {list:?}）"
            );
        }
    }
}

/// 主窗口生命周期闸门（2026-09-21，修维护者报的「关掉工作台窗口后，点『返回工作台』没反应」）。
///
/// GUI 行为本机没法点，但**正确性全在这几处结构与顺序上**，故用源码契约钉住：
/// 窗口被销毁 ⇒ 所有唤回入口静默落空；入口缺失 ⇒ 关窗后用户被困在隐藏态。
#[cfg(test)]
mod main_window_lifecycle_tests {
    /// 关窗必须是「隐藏」而不是销毁，且**不得**只在 macOS 生效。
    #[test]
    fn main_window_close_hides_instead_of_destroying() {
        let src = include_str!("ui.rs").replace("\r\n", "\n");
        let create = src
            .find("pub(crate) fn create_main_window")
            .expect("找不到 create_main_window");
        let end = src[create..]
            .find("\n}\n")
            .map(|i| create + i)
            .unwrap_or(src.len());
        let body = &src[create..end];

        assert!(
            body.contains("tauri::WindowEvent::CloseRequested"),
            "主窗口必须处理 CloseRequested：默认销毁会让 get_webview_window(\"main\") 变 None，\
             三处唤回入口全部静默落空（用户表现 = 点了没反应）"
        );
        assert!(
            body.contains("api.prevent_close()"),
            "必须 prevent_close，否则窗口照样被销毁"
        );
        assert!(
            body.contains("win.hide()"),
            "必须 hide：保留窗口与工作台进程"
        );
        assert!(
            !body.contains("#[cfg(target_os = \"macos\")]\n    {\n        let win"),
            "隐藏语义必须三平台一致（macOS 已有红绿灯监听，不得把关窗处理塞进它的 cfg 块）"
        );
    }

    /// 「显示工作台」必须在两条常驻入口上都存在，且接到同一个分派：
    /// macOS 菜单栏（无托盘）+ Windows/Linux 托盘右键。
    #[test]
    fn show_main_is_reachable_from_every_resident_entry() {
        let src = include_str!("ui.rs").replace("\r\n", "\n");
        let app_menu = src
            .find("pub(crate) fn build_app_menu")
            .expect("macOS 菜单构建");
        let tray_menu = src
            .find("pub(crate) fn build_tray_menu")
            .expect("托盘菜单构建");
        assert!(
            src[app_menu..tray_menu].contains("\"show_main\""),
            "macOS 菜单栏缺「显示工作台」：关窗=隐藏后 macOS 无托盘，用户没别的入口"
        );
        assert!(
            src[tray_menu..].contains("\"show_main\""),
            "托盘菜单缺「显示工作台」：关窗=隐藏后必须能从托盘右键唤回"
        );
        let lib = include_str!("lib.rs").replace("\r\n", "\n");
        assert!(
            lib.contains("\"show_main\" => commands::window::bring_to_front"),
            "菜单事件分派必须处理 show_main（否则菜单项点了没反应 —— 正是本次修的那类缺陷）"
        );
    }

    /// macOS 点 Dock 图标（无可视窗口）必须唤回主窗口 —— macOS 无托盘，这是最后的退路。
    #[test]
    fn dock_reopen_raises_the_main_window() {
        let lib = include_str!("lib.rs").replace("\r\n", "\n");
        let reopen = lib
            .find("RunEvent::Reopen")
            .expect("未处理 RunEvent::Reopen：关窗=隐藏后点 Dock 图标将毫无反应");
        // 按**字符**取窗口：字节切片会撞上中文注释的 UTF-8 边界（本用例第一版就死在这）。
        let tail: String = lib[reopen..].chars().take(200).collect();
        assert!(
            tail.contains("bring_to_front"),
            "Reopen 分支必须唤回主窗口（当前分支体：{}）",
            tail.lines().take(4).collect::<Vec<_>>().join(" / ")
        );
    }

    /// IPC 命令缺失窗口时**必须报错**，不得静默 no-op（前端已挂 `.catch(showToast)`）。
    #[test]
    fn focus_main_window_reports_a_missing_window() {
        let src = include_str!("commands/window.rs").replace("\r\n", "\n");
        assert!(
            src.contains("pub fn focus_main_window(app: tauri::AppHandle) -> Result<(), String>"),
            "focus_main_window 必须返回 Result：返回 () 会让前端的 catch 永远不触发，\
             用户侧表现就是『点了没反应』"
        );
        assert!(
            src.contains("get_webview_window(\"main\").is_none()"),
            "必须前置判存在并给出可行动错误"
        );
    }
}

/// Windows 维持原生装饰的「防复辟」闸门（2026-09-23 维护者裁定，ADR-0030）。
///
/// 撤回的是一次真机事故（issue #16：**窗口不能移动 / 缩放 / 关闭**）：`decorations(false)`
/// 在 Windows 上摘掉 `WS_CAPTION | WS_THICKFRAME`（缩放 + 贴靠随之消失），而替代它的自绘控件
/// 缺 ACL 授权、拖拽条是伪元素挂不上属性 —— 三件事各自都足够让窗口变砖。本模块保证这半成品
/// 不会再被悄悄加回来。
#[cfg(test)]
mod windows_native_decorations_tests {
    fn ui_source() -> &'static str {
        include_str!("ui.rs")
            .split("mod windows_native_decorations_tests")
            .next()
            .expect("split 至少返回一段")
    }

    /// 正例：窗口链上的 `decorations(false)` 一处都不许有（只允许出现在解释性注释里）。
    #[test]
    fn windows_window_stays_decorated() {
        for (idx, line) in ui_source().lines().enumerate() {
            if line.contains("decorations(false)") {
                assert!(
                    line.trim_start().starts_with("//"),
                    "ui.rs:{} 出现了实际生效的 decorations(false)：Windows 必须维持原生装饰\
                     （2026-09-23 裁定，ADR-0030）——它会摘掉 WS_CAPTION|WS_THICKFRAME，\
                     鼠标缩放与 Aero Snap 一并消失，而拖拽条/自绘控件在 Windows 上都无可用接线",
                    idx + 1
                );
            }
        }
    }

    /// 反例方向：注入脚本里的 Windows 沉浸式分支（标记 / 自绘控件 / Tauri window API 调用）
    /// 必须已整体移除——断言的是**代码形态**（注释里引用官方标记名不在此列）。
    #[test]
    fn injected_script_has_no_windows_branch() {
        let injected = include_str!("../../frontend/src/injected/immersive-chrome.js");
        for needle in [
            "installWindowsTitlebar",
            "dsh-dock-window-controls",
            "setAttribute('data-windows-titlebar'",
        ] {
            assert!(
                !injected.contains(needle),
                "注入脚本仍带 Windows 沉浸式分支（{needle}）：2026-09-23 裁定 Windows 维持原生装饰\
                 （ADR-0030），该分支的控件调用没有 ACL 授权、拖拽条是伪元素，属半成品"
            );
        }
    }

    /// 撤回不得误伤 macOS 档：Overlay + 隐标题那套仍在原位。
    #[test]
    fn macos_immersive_path_is_intact() {
        let src = ui_source();
        assert!(
            src.contains("title_bar_style(tauri::TitleBarStyle::Overlay)"),
            "macOS 的 Overlay 标题栏（ADR-0029）不得随本次回退丢失"
        );
        assert!(
            src.contains(".hidden_title(true)"),
            "macOS 隐藏标题文字（ADR-0029 §3）不得随本次回退丢失"
        );
    }
}

/// 沉浸式拖拽的接线闸门（2026-09-23 第二次改版后重建，沿用 issue #16 与 WKWebView 实测的教训）。
///
/// 两道链必须同时成立，缺一即「窗口拖不动 / 页面被吃掉」：
///
/// ① **机制（2026-09-23 改版）**：注入脚本按**几何语义**自驱拖拽 —— 命中点落在 dsh 自己的
///    拖拽带矩形内（`data-shell-leading-band`，取不到则顶部 52px 兜底）∧ 在 `#root` 内
///    ∧ 不在 dsh 的交互元素排除表上 ⇒ `startDragging()`；双击（按下不算、抬起未移动才算）
///    ⇒ `toggleMaximize()`。旧机制「扫 `-webkit-app-region` → 写 `data-tauri-drag-region`」
///    已由本机 WKWebView 探针**证伪**（`CSS.supports` 为 false、CSSOM 读回为空；且拖拽带是
///    `pointer-events:none`，命中测试型机制永远看不到它），**不得回流**。
///
/// ② **权限**：`start_dragging` 不在 `core:window:default`（tauri 2.11.5 = 28 条只读 getter +
///    `internal_toggle_maximize`）里，必须显式授权，否则 ACL 拒绝且被 `.catch` 吞掉 ——
///    用户侧同样只剩「拖不动」。
///
/// 另钉住「TS 纯模型 ↔ 注入脚本」的常量同步：两处靠人肉同步（无打包器），是最易漂的一环。
#[cfg(test)]
mod immersive_drag_acl_tests {
    const INJECTED: &str = include_str!("../../frontend/src/injected/immersive-chrome.js");
    const PURE_MODEL: &str = include_str!("../../frontend/src/lib/immersiveChrome.ts");

    /// 取 `marker` 之后、`stop` 之前的片段里所有双引号字面量并拼接（这两处的字面量内只有
    /// 单引号，故按 `"` 切分取奇数段即可）。
    fn joined_literals(src: &str, marker: &str, stop: &str) -> String {
        let start = src
            .find(marker)
            .unwrap_or_else(|| panic!("源码里找不到常量锚点 {marker}"))
            + marker.len();
        let rest = &src[start..];
        let end = rest
            .find(stop)
            .unwrap_or_else(|| panic!("常量 {marker} 之后找不到结束锚点 {stop}"));
        rest[..end].split('"').skip(1).step_by(2).collect()
    }

    fn capability_permissions() -> Vec<String> {
        let json: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/default.json"))
                .expect("capabilities/default.json 非法 JSON");
        json["permissions"]
            .as_array()
            .expect("capabilities/default.json 缺 permissions 数组")
            .iter()
            .filter_map(|v| v.as_str().map(str::to_owned))
            .collect()
    }

    #[test]
    fn drag_region_permissions_are_granted() {
        let perms = capability_permissions();
        assert!(
            perms.iter().any(|p| p == "core:default"),
            "capabilities 丢了 core:default：双击拖拽区最大化（internal_toggle_maximize）会失效"
        );
        assert!(
            perms
                .iter()
                .any(|p| p == "core:window:allow-start-dragging"),
            "缺 core:window:allow-start-dragging：`data-tauri-drag-region` 的单击拖拽会被 ACL \
             拒绝、窗口拖不动（tauri 2.11.5 的 core:window:default 不含它 —— v1.3.0~v1.3.2 实况）"
        );
    }

    /// 注入脚本的**生效代码**（剥掉整行注释：`//`、`/*`、`*`、`*/` 开头）——闸门断言的是
    /// 代码形态，注释里为说明历史而引用被证伪的属性名不算违规。
    fn injected_code() -> String {
        INJECTED
            .lines()
            .filter(|line| {
                let trimmed = line.trim_start();
                !(trimmed.starts_with("//")
                    || trimmed.starts_with("/*")
                    || trimmed.starts_with('*'))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 机制闸门：几何语义的四个要件齐全，且两条**已证伪**的老机制不得回流。
    #[test]
    fn injected_script_drives_drag_by_geometry() {
        let code = injected_code();
        for needle in [
            "data-shell-leading-band", // dsh 发布的拖拽带钩子
            "elementFromPoint",        // 命中测试：决定这一下是拖动还是点击
            "startDragging()",         // 窗口拖拽
            "toggleMaximize()",        // 双击最大化
            "#root",                   // 拖拽面锚点（body 下全是 no-drag 浮层）
        ] {
            assert!(
                code.contains(needle),
                "注入脚本缺几何语义要件 {needle}：窗口会拖不动或按钮点不动"
            );
        }
        for dead in ["-webkit-app-region", "data-tauri-drag-region"] {
            assert!(
                !code.contains(dead),
                "注入脚本的生效代码里出现了已被证伪的老机制 {dead}：WKWebView 不认\
                 `-webkit-app-region`（CSS.supports=false、CSSOM 读回为空），且拖拽带是\
                 pointer-events:none，命中测试型机制看不到它 —— 该路径 2026-09-23 起已废弃，\
                 不得回流"
            );
        }
    }

    /// 同步闸门：TS 纯模型与注入脚本的常量必须逐字一致（无打包器，两处同步靠人肉）。
    #[test]
    fn injected_script_constants_match_the_pure_model() {
        let js_exclusion = joined_literals(INJECTED, "var EXCLUSION_SELECTOR =", ";\n");
        let ts_exclusion =
            joined_literals(PURE_MODEL, "export const DRAG_EXCLUSION_SELECTOR =", "\n\n");
        assert_eq!(
            js_exclusion, ts_exclusion,
            "交互元素排除表两处不一致：与 dsh `web/src/base.css:72-78` 的 no-drag 列表必须\
             逐字同步（改一处漏一处 ⇒ 按钮被拖拽吃掉或可拖区莫名挖洞）"
        );
        assert!(
            js_exclusion.contains("button") && js_exclusion.contains("[role='tab']"),
            "排除表内容可疑：至少应覆盖 button 与 [role='tab']"
        );
        assert!(
            INJECTED.contains("'[data-shell-leading-band]'")
                && PURE_MODEL.contains(r#""[data-shell-leading-band]""#),
            "拖拽带钩子两处不一致（须同为 [data-shell-leading-band]）"
        );
        assert!(
            INJECTED.contains("FALLBACK_BAND_HEIGHT = 52")
                && PURE_MODEL.contains("DRAG_BAND_FALLBACK_HEIGHT = 52"),
            "兜底带高两处不一致（须与 dsh .leadingBand 的 52px 同值）"
        );
    }
}
