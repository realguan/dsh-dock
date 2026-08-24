//! dsh-dock —— DSH Dock：一个极小的 Tauri 桌面壳。
//!
//! 职责（docs/contract.md「运行时策略」）：
//!   1. 读取 product.manifest.json（运行时契约，v2 终端 + 宿主解析策略）；
//!   2. 宿主解析链：system（用户官方 dsh）→ bundle（内置档）→ download（实时下载）；
//!   3. system 档多 webUi profile 时先出选择器（F-b），选定后 spawn dsh（`--port 0`）
//!      并从日志解析实际地址，主窗口 WebView 导航进 `http://127.0.0.1:<port>/`；
//!   4. 应用退出时优雅停止 dsh（SIGTERM → SIGKILL 兜底）。
//!
//! IPC 面最小化（AGENTS 例外册）：`choose_profile`（选择器）与 `terminal_action`
//! （错误卡动作：retry / upgrade）。前端经 `window.__TAURI__.core.invoke` +
//! `window.__TAURI__.event.listen` 接收 `boot:step` / `boot:error` 事件流，
//! 启动过程全链路可视化（见 ui/index.html）。

mod manifest;
mod resolve;
mod shell;
mod updates;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{Manager, RunEvent};

/// 壳运行时状态：dsh 子进程 + 主窗口句柄 + 待选 profile 的启动规格。
struct ShellState {
    dsh: Mutex<Option<shell::DshProcess>>,
    window: tauri::WebviewWindow,
    /// 选择器场景：解析完成但尚未 spawn 的 LaunchSpec（用户选择后落地）。
    pending: Mutex<Option<crate::resolve::LaunchSpec>>,
    /// 最近一次更新检测结果（前端 chip / 托盘菜单共用）。
    update_status: Mutex<Option<crate::updates::UpdateStatus>>,
}

/// spawn dsh 并起监护线程（setup 默认路径与 choose_profile 共用）。
/// 进度经 `boot:step` 推给前端；失败经 `boot:error`（前端渲染错误卡）。
fn boot(
    state: Arc<ShellState>,
    app: tauri::AppHandle,
    launch: crate::resolve::LaunchSpec,
    data_dir: PathBuf,
) -> Result<(), String> {
    emit_step(
        &app,
        2,
        "running",
        &format!("spawn DSH（{} · tier={:?}）", launch.profile, launch.tier),
    );
    let log_path = data_dir.join("dsh-shell.log");
    let dsh = match shell::spawn_dsh(&launch, &data_dir) {
        Ok(dsh) => dsh,
        Err(e) => {
            let detail = e.to_string();
            report_boot_failure(&app, &detail);
            return Err(detail);
        }
    };
    let child_log = dsh.log_path.clone();
    *state.dsh.lock().unwrap() = Some(dsh);

    let _ = std::thread::spawn(move || {
        match shell::detect_url(&child_log, BOOT_TIMEOUT) {
            None => {
                let detail = read_error_detail(&child_log);
                let tail = read_log_tail(&child_log);
                emit_step(&app, 3, "error", "等待超时");
                emit_boot_error(&app, &format!("DSH 未在预期时间内就绪{detail}"), &tail);
            }
            Some(raw) => {
                match tauri::Url::parse(&raw) {
                    Ok(url) => {
                        tracing::info!("dsh 已就绪，进入 {url}");
                        emit_step(&app, 3, "done", &format!("DSH 已就绪：{url}"));
                        emit_step(&app, 4, "running", "导航到工作台界面");
                        let _ = state.window.navigate(url);
                        emit_step(&app, 4, "done", "已进入工作台");
                    }
                    Err(e) => {
                        emit_step(&app, 3, "error", "无效地址");
                        emit_boot_error(&app, &format!("DSH 报告了无效地址（{raw}）：{e}"), "");
                        return;
                    }
                }
                // 监护：终端与 dsh 同生命周期——dsh 崩溃即错误卡。
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    let exited = {
                        let mut guard = state.dsh.lock().unwrap();
                        guard
                            .as_mut()
                            .and_then(|p| p.child.try_wait().ok())
                            .flatten()
                    };
                    if let Some(status) = exited {
                        let code = status.code().unwrap_or(-1);
                        let detail = read_error_detail(&child_log);
                        let tail = read_log_tail(&child_log);
                        tracing::error!("dsh 异常退出 code={code}{detail}");
                        emit_boot_error(
                            &app,
                            &format!("DSH 进程已退出（code={code}）{detail}"),
                            &tail,
                        );
                        return;
                    }
                }
            }
        }
    });
    // 保持 log_path 变量绑定（避免未读警告）：boot 日志路径即 dsh-shell.log
    let _ = log_path;
    Ok(())
}

/// 日志尾部（错误卡「查看原始日志」区）。
fn read_log_tail(log_path: &std::path::Path) -> String {
    std::fs::read_to_string(log_path)
        .unwrap_or_default()
        .lines()
        .rev()
        .take(10)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}

/// 唯一 IPC 命令（②b profile 选择器）：选定 profile → 用 pending 的 LaunchSpec 启动。
#[tauri::command]
fn choose_profile(app: tauri::AppHandle, profile: String) -> Result<(), String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    let launch = state
        .pending
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| "无待启动任务（请重新打开终端）".to_string())?;
    let mut launch = launch;
    launch.profile = profile;
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let handle = app.clone();
    // 后台线程启动（npm/下载动作不阻塞）
    std::thread::spawn(move || {
        if let Err(e) = boot(state, handle.clone(), launch, data_dir) {
            tracing::error!("启动 dsh 失败: {e}");
        }
    });
    Ok(())
}

/// 前端/托盘读取最近一次检测结果（即读，不触网）。
#[tauri::command]
fn get_update_status(app: tauri::AppHandle) -> Result<crate::updates::UpdateStatus, String> {
    // 启动清单或宿主解析失败时，前端仍会请求版本状态。这里不能用
    // `state()`：它在状态尚未注册时会 panic，反而让本应展示错误卡的应用崩溃。
    Ok(cached_update_status(&app))
}

/// 无可读更新状态时交给前端的安全初始值。
fn empty_update_status() -> crate::updates::UpdateStatus {
    let none_component = crate::updates::ComponentUpdate {
        current: None,
        latest: None,
        newer: false,
        error: None,
    };
    crate::updates::UpdateStatus {
        dsh: none_component.clone(),
        client: none_component,
        node: None,
    }
}

/// 读取缓存状态；启动早期尚未注册 ShellState 时必须安全返回默认值。
fn cached_update_status<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
) -> crate::updates::UpdateStatus {
    let cached = app
        .try_state::<Arc<ShellState>>()
        .and_then(|state| state.update_status.lock().ok().and_then(|s| s.clone()));
    cached_status_or_default(cached)
}

/// 把尚未产生的缓存映射为前端可消费的初始状态。
fn cached_status_or_default(
    cached: Option<crate::updates::UpdateStatus>,
) -> crate::updates::UpdateStatus {
    cached.unwrap_or_else(empty_update_status)
}

/// 手动触发后台检测（异步：立即返回，完成时 boot:update + 托盘刷新）。
#[tauri::command]
fn check_updates(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    let handle = app.clone();
    std::thread::spawn(move || refresh_update_ui(&handle, &state));
    Ok(())
}

/// 错误卡动作（retry / upgrade）：重新解析并启动；upgrade 先升级全局 dsh。
/// upgrade_only：仅升级 + 刷新状态（托盘场景，不打断进行中的会话）。
#[tauri::command]
fn terminal_action(app: tauri::AppHandle, action: String) -> Result<(), String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let handle = app.clone();
    std::thread::spawn(move || {
        // upgrade / upgrade_only：动用户全局 dsh（按钮确认即授权，动作在后台线程）
        if action == "upgrade" || action == "upgrade_only" {
            emit_step(
                &handle,
                2,
                "running",
                "升级官方 DSH（pnpm 优先，npm 回退，最新版）…",
            );
            match crate::updates::install_latest_global(
                &data_dir,
                &mut download_progress_bridge(&handle),
            ) {
                Ok(_) => {
                    emit_step(&handle, 2, "done", "DSH 升级完成");
                    // 刷新版本状态（托盘/前端 chip）
                    refresh_update_ui(&handle, &state);
                }
                Err(e) => {
                    emit_boot_error(&handle, &format!("升级失败：{e}"), "");
                    return;
                }
            }
            if action == "upgrade_only" {
                return;
            }
        }
        // 重新走解析链 + 启动
        crate::lib_boot_again(state, handle.clone(), data_dir);
        let _ = handle;
    });
    Ok(())
}

/// retry/upgrade 共用：从 manifest 重新解析并启动。
fn lib_boot_again(state: Arc<ShellState>, app: tauri::AppHandle, data_dir: PathBuf) {
    let resources_dir = resolve_resources_dir(&app);
    let manifest =
        match manifest::ProductManifest::load(&resources_dir.join("product.manifest.json")) {
            Ok(m) => m,
            Err(e) => {
                emit_boot_error(&app, &format!("产品清单读取失败：{e}"), "");
                return;
            }
        };
    let path_env = crate::resolve::effective_path();
    emit_step(&app, 0, "running", "重新扫描用户环境");
    match crate::resolve::resolve_launch(
        &manifest,
        &resources_dir,
        &path_env,
        &data_dir,
        &mut download_progress_bridge(&app),
    ) {
        Ok(launch) => {
            emit_step(&app, 0, "done", "环境扫描完成");
            emit_step(&app, 1, "done", &format!("命中档位：{:?}", launch.tier));
            // 下载档刚补齐 dsh：立即刷新版本状态——否则关于页/菜单停留在
            // 安装前的「未检出」，要等用户手动检查才正确。
            if launch.tier == crate::manifest::TierKind::Download {
                let st = state.clone();
                let hd = app.clone();
                std::thread::spawn(move || refresh_update_ui(&hd, &st));
            }
            if launch.tier == crate::manifest::TierKind::System {
                let profiles =
                    crate::resolve::list_web_ui_profiles(&crate::resolve::user_dsh_home());
                if profiles.len() > 1 {
                    *state.pending.lock().unwrap() = Some(launch);
                    let _ = state.window.eval(&format!(
                        "location.assign('selector.html?profiles={}')",
                        profiles.join(",")
                    ));
                    return;
                }
            }
            if let Err(e) = boot(state, app.clone(), launch, data_dir) {
                tracing::error!("重启 dsh 失败: {e}");
            }
        }
        Err(e) => {
            tracing::error!("重新解析失败: {e}");
            emit_boot_error(&app, &format!("重新解析失败：{e}"), "");
        }
    }
}

/// dsh 启动等待上限：超过即认为装配有问题，进错误页。
const BOOT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

/// 定位含 product.manifest.json 的资源根（dev/prod 布局差异见 setup 注释）。
fn resolve_resources_dir<M: tauri::Manager<tauri::Wry>>(app: &M) -> PathBuf {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 日志初始化移入 setup（落 shell.log；GUI 下 stdout 不可见）。
    // 测试/外部如需独立日志可自行 try_init（幂等）。
    tauri::Builder::default()
        // 单实例锁：必须最先注册。OS 级原语随进程消亡自动释放，无残留锁文件；
        // 二次启动的回调发生在主实例里——唤起主窗口即可（防多开导致的
        // 双份下载 / 双 dsh 子进程 / 同一私有 prefix 并发 npm 安装）。
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.show();
                let _ = win.unminimize();
                let _ = win.set_focus();
            }
        }))
        .setup(|app| {
            // 壳侧诊断日志落 `<数据目录>/shell.log`（GUI 下 stderr 不可见）。
            // 子进程输出在 dsh-shell.log（shell.rs）；两者分离。
            if let Ok(data_dir) = app.path().app_data_dir() {
                if let Ok(file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(data_dir.join("shell.log"))
                {
                    let _ = tracing_subscriber::fmt()
                        .with_max_level(tracing::Level::INFO)
                        .with_writer(std::sync::Arc::new(file))
                        .try_init();
                }
            }
            // 资源根解析（dev/prod 差异）：
            //   - 生产（bundle）：Tauri v2 保留相对 src-tauri 的路径前缀，
            //     `resources/**` 落在 `.app/Contents/Resources/resources/`；
            //   - dev（cargo run，macOS）：resource_dir() 指向不存在的 target/Resources，
            //     回退链：exe_dir/resources（tauri-build 的副本，Windows 语义）→
            //     CARGO_MANIFEST_DIR/resources（源码树，本仓库开发常态）。
            let resources_dir = resolve_resources_dir(app);
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;

            // 契约缺失/不兼容 → 直接在窗口里给出可行动错误（A6：就地呈现）。
            let window = app
                .get_webview_window("main")
                .expect("main 窗口应由 tauri.conf.json 创建");
            let app_handle = app.handle().clone();
            // 必须在任何可失败的检查之前注册：即使 manifest 或 dsh 解析失败，
            // 启动页的 IPC（版本状态、重试）也有可用状态，不能因 `state()` panic
            // 把原本可展示的错误卡变成整个应用退出。
            let state = Arc::new(ShellState {
                dsh: Mutex::new(None),
                window: window.clone(),
                pending: Mutex::new(None),
                update_status: Mutex::new(None),
            });
            app.manage(state.clone());
            // 启动页防陈旧缓存：WKWebView 曾把旧版启动页缓存下来（2026-08-23 实测）。
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.clear_all_browsing_data();
            }
            // 更新检测（首启后台异步 + 应用菜单常驻）：先给"检测中"初始状态，
            // 背景线程完成后经 boot:update 推送并刷新应用菜单。
            {
                let status = crate::updates::UpdateStatus {
                    dsh: crate::updates::ComponentUpdate {
                        current: crate::updates::detect_current_version(),
                        latest: None,
                        newer: false,
                        error: None,
                    },
                    client: crate::updates::ComponentUpdate {
                        current: Some(env!("CARGO_PKG_VERSION").to_string()),
                        latest: None,
                        newer: false,
                        error: None,
                    },
                    node: None,
                };
                *state.update_status.lock().unwrap() = Some(status.clone());
                refresh_app_menu(&app_handle, &state);
                emit_update(&app_handle, &status);
            }
            let s2 = state.clone();
            let h2 = app.handle().clone();
            std::thread::spawn(move || refresh_update_ui(&h2, &s2));

            // 宿主解析可能触发网络下载和 npm 安装，必须在后台执行：setup 运行在
            // 主线程，阻塞它会让 macOS 把窗口判定为无响应，也会让早期错误事件丢失。
            let boot_state = state.clone();
            let boot_app = app_handle.clone();
            let boot_resources = resources_dir.clone();
            let boot_data = data_dir.clone();
            std::thread::spawn(move || {
                let manifest = match manifest::ProductManifest::load(
                    &boot_resources.join("product.manifest.json"),
                ) {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::error!("product.manifest 读取失败: {e}");
                        emit_boot_error(&boot_app, &e.to_string(), "");
                        return;
                    }
                };

                // 进度事件：环境检测 / 宿主解析
                emit_step(&boot_app, 0, "running", "扫描用户环境（PATH · 版本闸）");
                emit_step(&boot_app, 1, "running", "解析宿主档位");

                // 宿主解析链（docs/contract.md）：system → bundle → download。
                // GUI 启动 PATH 是系统最小集：用合并后的用户环境 PATH 探测（环境感知修复）。
                let path_env = crate::resolve::effective_path();
                let launch = match resolve::resolve_launch(
                    &manifest,
                    &boot_resources,
                    &path_env,
                    &boot_data,
                    &mut download_progress_bridge(&boot_app),
                ) {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::error!("宿主解析失败: {e}");
                        emit_step(&boot_app, 1, "error", &e.to_string());
                        emit_boot_error(&boot_app, &e.to_string(), "");
                        return;
                    }
                };
                emit_step(&boot_app, 0, "done", "环境扫描完成");
                emit_step(
                    &boot_app,
                    1,
                    "done",
                    &format!("命中档位：{:?}", launch.tier),
                );
                // 下载档刚补齐 dsh：立即刷新版本状态（同 lib_boot_again 处的说明）。
                if launch.tier == crate::manifest::TierKind::Download {
                    let st = boot_state.clone();
                    let hd = boot_app.clone();
                    std::thread::spawn(move || refresh_update_ui(&hd, &st));
                }

                // F-b：system 档且用户世界有多个 webUi profile → 先出选择器，选定再 spawn。
                if launch.tier == crate::manifest::TierKind::System {
                    let profiles =
                        crate::resolve::list_web_ui_profiles(&crate::resolve::user_dsh_home());
                    if profiles.len() > 1 {
                        *boot_state.pending.lock().unwrap() = Some(launch);
                        emit_step(&boot_app, 2, "running", "选择器：多个 webUi 工作台");
                        let _ = boot_state.window.eval(&format!(
                            "location.assign('selector.html?profiles={}')",
                            profiles.join(",")
                        ));
                        return;
                    }
                }

                // 默认路径：直接启动。
                if let Err(e) = boot(boot_state, boot_app.clone(), launch, boot_data) {
                    tracing::error!("启动 dsh 失败: {e}");
                }
            });
            Ok(())
        })
        .on_menu_event(|app, event| {
            let state = app.state::<Arc<ShellState>>().inner().clone();
            let handle = app.clone();
            match event.id().as_ref() {
                "check" => {
                    std::thread::spawn(move || refresh_update_ui(&handle, &state));
                }
                "upgrade" => {
                    std::thread::spawn(move || {
                        if let Ok(data_dir) = handle.path().app_data_dir() {
                            let _ = crate::updates::install_latest_global(
                                &data_dir,
                                &mut download_progress_bridge(&handle),
                            );
                            refresh_update_ui(&handle, &state);
                        }
                    });
                }
                "about" => open_about(&handle),
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            choose_profile,
            terminal_action,
            get_update_status,
            check_updates
        ])
        .build(tauri::generate_context!())
        .expect("构建 Tauri app 失败")
        .run(|app_handle, event| {
            // 应用退出 → 优雅停止 dsh（壳退 = dsh 停，同生命周期）。
            if let RunEvent::Exit = event {
                if let Some(state) = app_handle.try_state::<Arc<ShellState>>() {
                    if let Some(mut dsh) = state.dsh.lock().unwrap().take() {
                        let code =
                            shell::stop_dsh(&mut dsh.child, std::time::Duration::from_secs(3));
                        tracing::info!("dsh 已停止（exit {code}）");
                    }
                    // 选择器场景可能尚未 spawn：清理 pending，不留状态。
                    state.pending.lock().unwrap().take();
                }
            }
        });
}

/// 发射 boot:step 事件（state: pending|running|done|error）。
fn emit_step(app: &tauri::AppHandle, step: usize, state: &str, detail: &str) {
    use tauri::Emitter;
    let _ = app.emit(
        "boot:step",
        serde_json::json!({
            "step": step,
            "state": state,
            "detail": detail,
        }),
    );
}

/// 下载进度 → `boot:progress` 事件的桥接（updates 模块保持零 tauri 依赖）。
/// 节流：≥100ms 一次；完成（current ≥ total）必发。
fn download_progress_bridge(app: &tauri::AppHandle) -> impl FnMut(u64, Option<u64>) + use<'_> {
    let mut last: Option<std::time::Instant> = None;
    move |current, total| {
        let now = std::time::Instant::now();
        let done = total.map(|t| current >= t).unwrap_or(false);
        let throttled = last
            .map(|t| now.duration_since(t) < std::time::Duration::from_millis(100))
            .unwrap_or(false);
        if throttled && !done {
            return;
        }
        last = Some(now);
        use tauri::Emitter;
        let _ = app.emit(
            "boot:progress",
            serde_json::json!({
                "kind": "node",
                "current": current,
                "total": total,
            }),
        );
    }
}

/// 把 spawn/初始化阶段的错误统一回传给启动页。
///
/// 这些错误发生在监护线程建立之前；若只写日志，前端会永远停在“启动 dsh”。
fn report_boot_failure(app: &tauri::AppHandle, detail: &str) {
    tracing::error!("启动 dsh 失败: {detail}");
    emit_step(app, 2, "error", detail);
    emit_boot_error(app, detail, "");
}

/// 发射 boot:error 事件（错误卡数据：标题/详情/建议/可用动作）。
fn emit_boot_error(app: &tauri::AppHandle, detail: &str, log_tail: &str) {
    use tauri::Emitter;
    let (title, suggestion, actions) = classify_boot_error(detail);
    let _ = app.emit(
        "boot:error",
        serde_json::json!({
            "title": title,
            "detail": detail,
            "suggestion": suggestion,
            "actions": actions,
            "log": log_tail,
        }),
    );
}

/// 错误分类：把 dsh 世界的问题归到可行动动作（upgrade / retry）。
fn classify_boot_error(detail: &str) -> (&'static str, &'static str, Vec<&'static str>) {
    let d = detail.to_lowercase();
    if d.contains("credentials") || d.contains("must be a string") {
        (
            "宿主 DSH 与您的凭据格式不匹配",
            "通常是 DSH 版本过旧：升级到官方最新版可解决（升级只动 pnpm/npm 全局，不碰您的数据）。",
            vec!["upgrade", "retry"],
        )
    } else if d.contains("unknown option") || d.contains("incompatible") {
        (
            "宿主 DSH 参数不兼容",
            "请升级您的 DSH 到支持当前终端行为的版本。",
            vec!["upgrade", "retry"],
        )
    } else if d.contains("network") || d.contains("registry") || d.contains("timeout") {
        (
            "网络不可用",
            "实时下载需要网络连接；检查网络后重试。",
            vec!["retry"],
        )
    } else {
        (
            "DSH 工作台启动失败",
            "详情见日志；可重试，若持续请反馈。",
            vec!["retry"],
        )
    }
}

/// 从日志提取崩溃原因摘要（首条顶层 Error 行，截断 200 字符），带 `<br/>` 前缀。
fn read_error_detail(log_path: &std::path::Path) -> String {
    let text = std::fs::read_to_string(log_path).unwrap_or_default();
    let line = text
        .lines()
        .find(|l| l.starts_with("Error:") || l.starts_with("error:"))
        .map(|l| {
            l.trim_start_matches("Error:")
                .trim_start_matches("error:")
                .trim()
                .to_string()
        })
        .unwrap_or_default();
    if line.is_empty() {
        String::new()
    } else {
        let cut: String = line.chars().take(200).collect();
        let suffix = if line.chars().count() > 200 {
            "…"
        } else {
            ""
        };
        format!("\n错误摘要：{}{}", cut, suffix)
    }
}

#[cfg(test)]
mod tests {
    use super::{cached_status_or_default, classify_boot_error};

    #[test]
    fn update_status_is_safe_before_shell_state_is_managed() {
        // 解析失败时 setup 可能尚未注册状态；release 的 panic=abort 不能让
        // 前端这次普通的版本查询把整个桌面进程带崩。
        let status = cached_status_or_default(None);
        assert!(status.dsh.current.is_none());
        assert!(status.dsh.latest.is_none());
        assert!(!status.dsh.newer);
        assert!(status.dsh.error.is_none());
        assert!(status.client.current.is_none());
        assert!(status.node.is_none());
    }

    #[test]
    fn credential_mismatch_classifies_upgrade() {
        let (title, _, actions) = classify_boot_error(
            "credentials-local: the value for \"version\" in ~/.dsh/.credentials.yaml must be a string",
        );
        assert!(title.contains("凭据"));
        assert!(actions.contains(&"upgrade"));
        assert!(actions.contains(&"retry"));
    }

    #[test]
    fn network_classifies_retry_only() {
        let (_, _, actions) = classify_boot_error("registry 不可达：network timeout");
        assert_eq!(actions, vec!["retry"]);
    }

    #[test]
    fn unknown_option_classifies_upgrade() {
        let (_, _, actions) = classify_boot_error("error: unknown option '--no-open'");
        assert!(actions.contains(&"upgrade"));
    }

    #[test]
    fn generic_failure_classifies_retry() {
        let (title, _, actions) = classify_boot_error("some weird crash");
        assert!(title.contains("启动失败"));
        assert_eq!(actions, vec!["retry"]);
    }
}

// ---------- 更新应用菜单（macOS 菜单栏；托盘已砍，裁定 2026-08-23） ----------

/// 组装应用菜单：macOS 菜单栏结构 = 根菜单内放「App 子菜单」+「编辑」子菜单。
/// 第一个子菜单被 macOS 自动视为 App 菜单（标题取 app 名、图标取 bundle 图标）——
/// 平铺 MenuItem 会导致菜单栏出现齿轮占位图标（2026-08-23 实测）。
/// App 菜单内容：状态行 → 检查更新…（⌘U）→ 升级到 X → 关于 → 标准项。
fn build_app_menu(
    app: &tauri::AppHandle,
    status: &crate::updates::UpdateStatus,
) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{MenuBuilder, MenuItem, PredefinedMenuItem, SubmenuBuilder};

    let dsh = &status.dsh;
    let state_line = if dsh.error.is_some() {
        "检测失败（网络不可达）".to_string()
    } else if dsh.newer {
        format!(
            "DSH {} · 发现新版 {}",
            dsh.current.clone().unwrap_or_else(|| "?".into()),
            dsh.latest.clone().unwrap_or_default()
        )
    } else if dsh.current.is_some() && dsh.latest.is_some() {
        format!(
            "DSH {} · 已是最新",
            dsh.current.clone().unwrap_or_else(|| "?".into())
        )
    } else if dsh.latest.is_some() {
        // 未检出本地 dsh（多半是下载档正在补装）：不能误称「已是最新」。
        format!(
            "DSH 未检出 · 官方最新 {}",
            dsh.latest.clone().unwrap_or_default()
        )
    } else {
        format!(
            "DSH {} · 检测中…",
            dsh.current.clone().unwrap_or_else(|| "?".into())
        )
    };

    let st = MenuItem::with_id(app, "st", state_line, false, None::<&str>)?;
    let check = MenuItem::with_id(app, "check", "检查更新…", true, Some("CmdOrCtrl+U"))?;
    let upgrade = MenuItem::with_id(
        app,
        "upgrade",
        format!("升级到 {}", dsh.latest.clone().unwrap_or_default()),
        dsh.newer,
        None::<&str>,
    )?;
    let about = MenuItem::with_id(app, "about", "关于", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;

    // App 子菜单（macOS 忽略其 text，标题自动为 app 名）
    let app_menu = SubmenuBuilder::new(app, "dsh-dock")
        .item(&st)
        .item(&check)
        .item(&upgrade)
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

/// 发射 boot:update 事件（前端版本行芯片消费）。
fn emit_update(app: &tauri::AppHandle, status: &crate::updates::UpdateStatus) {
    use tauri::Emitter;
    let _ = app.emit("boot:update", status);
}

/// 后台检测一次并同步应用菜单 + 事件（首启/手动/升级后共用）。
fn refresh_update_ui(app: &tauri::AppHandle, state: &Arc<ShellState>) {
    let Ok(data_dir) = app.path().app_data_dir() else {
        return;
    };
    let status = crate::updates::check_now(&data_dir);
    tracing::info!(
        "更新检测：dsh={:?}/{:?}(newer={}) client={:?}/{:?} node={:?}",
        status.dsh.current,
        status.dsh.latest,
        status.dsh.newer,
        status.client.current,
        status.client.latest,
        status
            .node
            .clone()
            .map(|n| format!("{}({})", n.version, n.origin))
    );
    *state.update_status.lock().unwrap() = Some(status.clone());
    refresh_app_menu(app, state);
    emit_update(app, &status);
}

/// 用最近状态重建并设置应用菜单（检测完成/升级后调用）。
fn refresh_app_menu(app: &tauri::AppHandle, state: &Arc<ShellState>) {
    let status = state
        .update_status
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(empty_update_status);
    if let Ok(menu) = build_app_menu(app, &status) {
        let _ = app.set_menu(menu);
    }
}

/// 关于面板：独立小窗（壳版本 + 宿主 dsh 版本 + 检查/升级），复用 ui/about.html。
fn open_about(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("about") {
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let builder =
        tauri::WebviewWindowBuilder::new(app, "about", tauri::WebviewUrl::App("about.html".into()))
            .title("关于")
            .inner_size(460.0, 360.0)
            .resizable(false)
            .center();
    let _ = builder.build();
}
