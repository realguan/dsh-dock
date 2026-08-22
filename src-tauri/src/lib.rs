//! dsh-desktop-shell —— ADR-0005 的「桌面终端」：一个极小的 Tauri 桌面壳。
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
}

/// spawn dsh 并起监护线程（setup 默认路径与 choose_profile 共用）。
/// 进度经 `boot:step` 推给前端；失败经 `boot:error`（前端渲染错误卡）。
fn boot(state: Arc<ShellState>, app: tauri::AppHandle, launch: crate::resolve::LaunchSpec, data_dir: PathBuf) -> Result<(), String> {
    emit_step(&app, 2, "running", &format!("spawn dsh（{} · tier={:?}）", launch.profile, launch.tier));
    let log_path = data_dir.join("dsh-shell.log");
    let dsh = shell::spawn_dsh(&launch, &data_dir).map_err(|e| e.to_string())?;
    let child_log = dsh.log_path.clone();
    *state.dsh.lock().unwrap() = Some(dsh);

    let _ = std::thread::spawn(move || {
        match shell::detect_url(&child_log, BOOT_TIMEOUT) {
            None => {
                let detail = read_error_detail(&child_log);
                let tail = read_log_tail(&child_log);
                emit_step(&app, 3, "error", "等待超时");
                emit_boot_error(&app, &format!("dsh 未在预期时间内就绪{detail}"), &tail);
            }
            Some(raw) => {
                match tauri::Url::parse(&raw) {
                    Ok(url) => {
                        tracing::info!("dsh 已就绪，进入 {url}");
                        emit_step(&app, 3, "done", &format!("dsh 已就绪：{url}"));
                        emit_step(&app, 4, "running", "导航到工作台界面");
                        let _ = state.window.navigate(url);
                        emit_step(&app, 4, "done", "已进入工作台");
                    }
                    Err(e) => {
                        emit_step(&app, 3, "error", "无效地址");
                        emit_boot_error(&app, &format!("dsh 报告了无效地址（{raw}）：{e}"), "");
                        return;
                    }
                }
                // 监护：终端与 dsh 同生命周期——dsh 崩溃即错误卡。
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    let exited = {
                        let mut guard = state.dsh.lock().unwrap();
                        guard.as_mut().and_then(|p| p.child.try_wait().ok()).flatten()
                    };
                    if let Some(status) = exited {
                        let code = status.code().unwrap_or(-1);
                        let detail = read_error_detail(&child_log);
                        let tail = read_log_tail(&child_log);
                        tracing::error!("dsh 异常退出 code={code}{detail}");
                        emit_boot_error(&app, &format!("dsh 进程已退出（code={code}）{detail}"), &tail);
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
    let state = app
        .state::<Arc<ShellState>>()
        .inner()
        .clone();
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
        let _ = boot(state, handle.clone(), launch, data_dir);
    });
    Ok(())
}

/// 错误卡动作（retry / upgrade）：重新解析并启动；upgrade 先升级全局 dsh。
#[tauri::command]
fn terminal_action(app: tauri::AppHandle, action: String) -> Result<(), String> {
    let state = app
        .state::<Arc<ShellState>>()
        .inner()
        .clone();
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let handle = app.clone();
    std::thread::spawn(move || {
        // upgrade：动用户全局 dsh（按钮确认即授权，动作本身在后台线程）
        if action == "upgrade" {
            emit_step(&handle, 2, "running", "升级官方 dsh（npm 全局，最新版）…");
            match crate::updates::install_latest_global(&data_dir) {
                Ok(_) => emit_step(&handle, 2, "done", "dsh 升级完成"),
                Err(e) => {
                    emit_boot_error(&handle, &format!("升级失败：{e}"), "");
                    return;
                }
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
    let manifest = match manifest::ProductManifest::load(&resources_dir.join("product.manifest.json")) {
        Ok(m) => m,
        Err(e) => {
            emit_boot_error(&app, &format!("产品清单读取失败：{e}"), "");
            return;
        }
    };
    let path_env = crate::resolve::effective_path();
    emit_step(&app, 0, "running", "重新扫描用户环境");
    match crate::resolve::resolve_launch(&manifest, &resources_dir, &path_env, &data_dir) {
        Ok(launch) => {
            emit_step(&app, 0, "done", "环境扫描完成");
            emit_step(&app, 1, "done", &format!("命中档位：{:?}", launch.tier));
            if launch.tier == crate::manifest::TierKind::System {
                let profiles = crate::resolve::list_web_ui_profiles(&crate::resolve::user_dsh_home());
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
    let exe_res = app.path().executable_dir().ok().unwrap_or_default().join("resources");
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
            let manifest = match manifest::ProductManifest::load(&resources_dir.join("product.manifest.json"))
            {
                Ok(m) => m,
                Err(e) => {
                    tracing::error!("product.manifest 读取失败: {e}");
                    emit_boot_error(&app_handle, &e.to_string(), "");
                    return Ok(());
                }
            };

            // 进度事件：环境检测 / 宿主解析
            emit_step(&app_handle, 0, "running", "扫描用户环境（PATH · 版本闸）");
            emit_step(&app_handle, 1, "running", "解析宿主档位");

            // 宿主解析链（ADR-0005）：system → bundle → download。
            // GUI 启动 PATH 是系统最小集：用合并后的用户环境 PATH 探测（环境感知修复）。
            let path_env = crate::resolve::effective_path();
            let launch = match resolve::resolve_launch(&manifest, &resources_dir, &path_env, &data_dir) {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!("宿主解析失败: {e}");
                    emit_step(&app_handle, 1, "error", &e.to_string());
                    emit_boot_error(&app_handle, &e.to_string(), "");
                    return Ok(());
                }
            };
            emit_step(&app_handle, 0, "done", "环境扫描完成");
            emit_step(&app_handle, 1, "done", &format!("命中档位：{:?}", launch.tier));

            let state = Arc::new(ShellState {
                dsh: Mutex::new(None),
                window: window.clone(),
                pending: Mutex::new(None),
            });
            app.manage(state.clone());

            // F-b：system 档且用户世界有多个 webUi profile → 先出选择器，选定再 spawn。
            if launch.tier == crate::manifest::TierKind::System {
                let profiles = crate::resolve::list_web_ui_profiles(&crate::resolve::user_dsh_home());
                if profiles.len() > 1 {
                    *state.pending.lock().unwrap() = Some(launch);
                    emit_step(&app_handle, 2, "running", "选择器：多个 webUi 工作台");
                    let _ = window.eval(&format!(
                        "location.assign('selector.html?profiles={}')",
                        profiles.join(",")
                    ));
                    return Ok(());
                }
            }

            // 默认路径：直接启动。
            if let Err(e) = boot(state, app_handle, launch, data_dir) {
                tracing::error!("启动 dsh 失败: {e}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![choose_profile, terminal_action])
        .build(tauri::generate_context!())
        .expect("构建 Tauri app 失败")
        .run(|app_handle, event| {
            // 应用退出 → 优雅停止 dsh（壳退 = dsh 停，同生命周期）。
            if let RunEvent::Exit = event {
                if let Some(state) = app_handle.try_state::<Arc<ShellState>>() {
                    if let Some(mut dsh) = state.dsh.lock().unwrap().take() {
                        let code = shell::stop_dsh(&mut dsh.child, std::time::Duration::from_secs(3));
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
            "宿主 dsh 与您的凭据格式不匹配",
            "通常是 dsh 版本过旧：升级到官方最新版可解决（升级只动 npm 全局，不碰您的数据）。",
            vec!["upgrade", "retry"],
        )
    } else if d.contains("unknown option") || d.contains("incompatible") {
        (
            "宿主 dsh 参数不兼容",
            "请升级您的 dsh 到支持当前终端行为的版本。",
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
            "dsh 世界启动失败",
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
        let suffix = if line.chars().count() > 200 { "…" } else { "" };
        format!("\n错误摘要：{}{}", cut, suffix)
    }
}

#[cfg(test)]
mod tests {
    use super::classify_boot_error;

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
