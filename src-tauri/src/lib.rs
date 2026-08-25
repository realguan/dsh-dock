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
mod updater;
mod updates;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use tauri::{Manager, RunEvent};

/// Windows：子进程一律「无控制台窗口」启动（CREATE_NO_WINDOW）——否则每次拉起
/// node/npm/pnpm 都会弹一个黑色终端窗口（2026-08-24 Windows 实测：环境检查阶段
/// 一连弹好几个，dsh 本体那个还会常驻整个会话）。
/// 非 Windows 平台无此问题，保持默认。
#[cfg(windows)]
fn quiet_cmd(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    // CREATE_NO_WINDOW = 0x08000000：子进程不创建控制台窗口（stdout/stderr 仍可经管道读取）。
    cmd.creation_flags(0x0800_0000);
}

#[cfg(not(windows))]
fn quiet_cmd(_cmd: &mut Command) {}

/// 构造子进程命令的统一入口：
/// 1. Windows 上 `.cmd`/`.bat` 批处理不能直接 spawn（CreateProcess 不认批处理），
///    必须包一层 `cmd /C`；pnpm 在 Windows 是 pnpm.cmd，不走这里必然失败。
/// 2. 一律 `quiet_cmd`（黑色终端窗口问题）。
fn child_cmd(bin: &Path) -> Command {
    let mut cmd = if cfg!(windows) && is_batch_script(bin) {
        let mut c = Command::new("cmd.exe");
        c.arg("/C").arg(bin);
        c
    } else {
        Command::new(bin)
    };
    quiet_cmd(&mut cmd);
    cmd
}

/// Windows 的 `.cmd`/`.bat` 批处理脚本（需 cmd.exe /C 包装）。
fn is_batch_script(p: &Path) -> bool {
    matches!(
        p.extension().and_then(|e| e.to_str()).map(str::to_lowercase).as_deref(),
        Some("cmd") | Some("bat")
    )
}

#[cfg(test)]
mod proc_tests {
    use super::*;

    #[test]
    fn external_url_allowlist_blocks_non_http_and_unknown_hosts() {
        // 白名单内（精确域 + 子域 + http/https）
        assert!(is_allowed_external_url("https://commandcode.ai/console"));
        assert!(is_allowed_external_url("https://api.commandcode.ai/v1"));
        assert!(is_allowed_external_url("http://docs.deepseek.com/intro"));
        assert!(is_allowed_external_url("https://github.com/x/y"));
        // 非 http(s) 一律拒绝
        assert!(!is_allowed_external_url("file:///etc/passwd"));
        assert!(!is_allowed_external_url("data:text/html,x"));
        assert!(!is_allowed_external_url("javascript:alert(1)"));
        // 未知域拒绝（含伪装后缀）
        assert!(!is_allowed_external_url("https://evil-commandcode.ai"));
        assert!(!is_allowed_external_url("https://deepseek.com.evil.io"));
        assert!(!is_allowed_external_url("https://example.com"));
        // 畸形 URL
        assert!(!is_allowed_external_url("not a url"));
    }

    #[test]
    fn batch_scripts_are_detected_case_insensitively() {
        assert!(is_batch_script(Path::new(r"C:\Users\me\AppData\Roaming\npm\pnpm.cmd")));
        assert!(is_batch_script(Path::new(r"D:\tools\setup.bat")));
        assert!(is_batch_script(Path::new("pnpm.CMD")));
        assert!(!is_batch_script(Path::new(r"C:\Program Files\nodejs\node.exe")));
        assert!(!is_batch_script(Path::new("/usr/local/bin/node")));
        assert!(!is_batch_script(Path::new("npm-cli.js")));
        assert!(!is_batch_script(Path::new("no_extension")));
    }

    #[test]
    fn child_cmd_wraps_batch_on_windows_only() {
        let win = cfg!(windows);
        let cmd = child_cmd(Path::new(r"C:\x\pnpm.cmd"));
        let exe = cmd.get_program().to_string_lossy().to_lowercase();
        assert_eq!(win, exe.ends_with("cmd.exe"), "批处理应被 cmd.exe 包装");
        let cmd2 = child_cmd(Path::new(r"C:\Program Files\nodejs\node.exe"));
        let exe2 = cmd2.get_program().to_string_lossy();
        assert!(
            exe2.ends_with("node.exe"),
            "exe 应直接 spawn，不走 cmd：{exe2}"
        );
    }
}

/// 壳运行时状态：dsh 子进程 + 主窗口句柄 + 待选 profile 的启动规格。
struct ShellState {
    dsh: Mutex<Option<shell::DshProcess>>,
    window: tauri::WebviewWindow,
    /// 选择器场景：解析完成但尚未 spawn 的 LaunchSpec（用户选择后落地）。
    pending: Mutex<Option<crate::resolve::LaunchSpec>>,
    /// 最近一次更新检测结果（前端 chip / 托盘菜单共用）。
    update_status: Mutex<Option<crate::updates::UpdateStatus>>,
    /// 当前工作台地址（dsh 就绪导航时记录；「在浏览器中打开」入口用）。
    workbench_url: Mutex<Option<tauri::Url>>,
    /// 桌面客户端自更新状态机（updater.rs；Rust 侧唯一写者，前端只读）。
    client_update: Mutex<Option<crate::updater::ClientUpdate>>,
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
        match shell::wait_for_ready(&child_log, &state.dsh, BOOT_STALL, BOOT_TIMEOUT) {
            shell::ReadyOutcome::Exited(code) => {
                // dsh 先退出了：真失败，立即报错（不等满上限）。
                let detail = read_error_detail(&child_log);
                let tail = read_log_tail(&child_log);
                emit_step(&app, 3, "error", &format!("DSH 进程退出（code={code}）"));
                let _ = state.dsh.lock().unwrap().take();
                emit_boot_error(&app, &format!("DSH 进程已退出（code={code}）{detail}"), &tail);
            }
            shell::ReadyOutcome::Stalled => {
                // 进程活着但长时间未就绪（冷启动慢/卡死）：停掉旧进程再报错，
                // 重试不会残留前一次 spawn 的 dsh（壳与 dsh 严格同生命周期）。
                let detail = read_error_detail(&child_log);
                let tail = read_log_tail(&child_log);
                emit_step(&app, 3, "error", "等待超时");
                if let Some(mut dsh) = state.dsh.lock().unwrap().take() {
                    let _ = shell::stop_dsh(&mut dsh.child, std::time::Duration::from_secs(3));
                }
                emit_boot_error(&app, &format!("DSH 未在预期时间内就绪{detail}"), &tail);
            }
            shell::ReadyOutcome::Ready(raw) => match tauri::Url::parse(&raw) {
                Ok(url) => {
                    tracing::info!("dsh 已就绪，进入 {url}");
                    *state.workbench_url.lock().unwrap() = Some(url.clone());
                    emit_step(&app, 3, "done", &format!("DSH 已就绪：{url}"));
                    emit_step(&app, 4, "running", "导航到工作台界面");
                    let _ = state.window.navigate(url);
                    emit_step(&app, 4, "done", "已进入工作台");
                    guard_dsh(&app, &state, &child_log);
                }
                Err(e) => {
                    emit_step(&app, 3, "error", "无效地址");
                    emit_boot_error(&app, &format!("DSH 报告了无效地址（{raw}）：{e}"), "");
                    let _ = state.dsh.lock().unwrap().take();
                }
            },
        }
    });
    // 保持 log_path 变量绑定（避免未读警告）：boot 日志路径即 dsh-shell.log
    let _ = log_path;
    Ok(())
}

/// dsh 就绪后的监护：终端与 dsh 同生命周期——dsh 崩溃即错误卡。
/// 在 boot 成功导航后的同一监护线程内持续运行（不新开线程，避免竞态）。
fn guard_dsh(app: &tauri::AppHandle, state: &Arc<ShellState>, child_log: &std::path::Path) {
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
            let detail = read_error_detail(child_log);
            let tail = read_log_tail(child_log);
            tracing::error!("dsh 异常退出 code={code}{detail}");
            emit_boot_error(
                app,
                &format!("DSH 进程已退出（code={code}）{detail}"),
                &tail,
            );
            return;
        }
    }
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

/// 读取桌面客户端自更新状态（即读，不触网；前端初始渲染）。
#[tauri::command]
fn get_client_update(app: tauri::AppHandle) -> Result<crate::updater::ClientUpdate, String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    Ok(crate::updater::current(&state))
}

/// 「检查客户端更新」：后台查 GitHub Releases latest.json，结果经 app:update 回推。
#[tauri::command]
fn client_update_check(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    crate::updater::run_check(app, state);
    Ok(())
}

/// 「确认安装客户端更新」：下载 → 安装 → 重启（Windows 由安装器接手后退出）。
#[tauri::command]
fn client_update_apply(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    crate::updater::run_download_and_install(app, state);
    Ok(())
}

/// 打开「关于与更新」面板（前端顶栏按钮；非 macOS 无应用菜单，这是唯一的
/// 关于/检查更新入口，macOS 菜单里的「关于」也走同一实现）。
#[tauri::command]
fn open_about(app: tauri::AppHandle) -> Result<(), String> {
    open_about_window(&app);
    Ok(())
}

/// 外链白名单：只放行 http/https 且主机在白名单内的 URL（壳的 IPC 不应成为
/// 任意 URL 的跳板）。dsh Web UI 的外链（文档/官网/控制台）都应落在这里；
/// 未收录的域会被拒绝——需要新域时在此登记。
const EXTERNAL_URL_HOSTS: &[&str] = &[
    "commandcode.ai",
    "www.commandcode.ai",
    "api.commandcode.ai",
    "deepseek.com",
    "www.deepseek.com",
    "platform.deepseek.com",
    "github.com",
];

/// 校验外链：仅 http/https，且主机等于或以白名单域结尾（`.example.com` 子域）。
fn is_allowed_external_url(raw: &str) -> bool {
    let Ok(url) = tauri::Url::parse(raw) else {
        return false;
    };
    if url.scheme() != "http" && url.scheme() != "https" {
        return false;
    }
    let Some(host) = url.host_str() else {
        return false;
    };
    let host = host.to_lowercase();
    EXTERNAL_URL_HOSTS
        .iter()
        .any(|allowed| host == *allowed || host.ends_with(&format!(".{allowed}")))
}

/// 用系统默认浏览器打开外链（dsh Web UI 里的超链接在 WebView 里点不动，
/// 2026-08-25 实测；统一转系统浏览器）。URL 必须过白名单。
#[tauri::command]
fn open_external(url: String) -> Result<(), String> {
    if !is_allowed_external_url(&url) {
        return Err(format!("不允许的外链：{url}"));
    }
    open::that_detached(&url).map_err(|e| format!("打开浏览器失败：{e}"))
}

/// 用系统默认浏览器打开当前工作台（壳内 WebView → 浏览器；dsh 就绪后可用）。
#[tauri::command]
fn open_workbench_in_browser(app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    let url = state
        .workbench_url
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "工作台尚未就绪".to_string())?;
    open::that_detached(url.as_str()).map_err(|e| format!("打开浏览器失败：{e}"))
}

/// 读取当前工作台地址（关于页/菜单展示用；未就绪返回 null）。
#[tauri::command]
fn get_workbench_url(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    let url = state.workbench_url.lock().unwrap().clone();
    Ok(url.map(|u| u.to_string()))
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
fn create_main_window(app: &tauri::AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    let hook_script = r#"(function () {
      if (window.__dshDockLinkHooked) return;
      window.__dshDockLinkHooked = true;
      document.addEventListener('click', function (ev) {
        var a = ev.target && ev.target.closest ? ev.target.closest('a[href]') : null;
        if (!a) return;
        var href = a.getAttribute('href') || '';
        if (!/^https?:\/\//i.test(href)) return; // 相对/锚点交给页面自己
        try {
          var u = new URL(href, location.href);
          if (u.origin === location.origin) return; // 同源（回环 dsh 自身）放行
        } catch (e) { return; }
        // 只有 __TAURI__ IPC 可用时才拦截（否则放行，让 WKWebView 原生
        // on_navigation/on_new_window 兜底——preventDefault 后 invoke 失败
        // 会变成"点了没反应"）。
        var tauri = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
        if (!tauri) return;
        ev.preventDefault();
        try {
          window.__TAURI__.core.invoke('open_external', { url: href })
            .catch(function (e) { console.error('[dsh-dock] 外链打开失败:', e); });
        } catch (e) {}
      }, true);
    })();"#;

    // WebView 渲染内存策略（2026-08-25）：dsh web 前端把整个会话一次性渲染进
    // DOM（无虚拟化）；WebKit（macOS WKWebView / Linux WebKitGTK）对「视口外
    // 大量已渲染内容」的回收远不如 Chromium——长会话下渲染资源持续累积，
    // WebContent 进程膨胀到数 GB（实测 4.3 GB / PID WebKit.WebContent）。
    //
    // 对策：`content-visibility: auto`（CSS 原生渲染级虚拟化）。视口外的行
    // 仍在 DOM、仍占布局（contain-intrinsic-size 占位），但 WebKit 跳过其
    // 布局/绘制并显式释放渲染资源；滚入视口时立即完整渲染。与 dsh 前端的
    // scroll anchoring（elementsFromPoint hit-test，ChatView.pagingAnchor）
    // 天然兼容——布局坐标不受影响。
    //
    // 豁免规则（只豁免「必须常驻渲染」的行，其余全部打上）：
    // - 流式中的行（[data-streaming]）：内容在增长，必须实时渲染；
    // - 空行：无内容可剪裁（同时也避免 pull 一个占位空盒）；
    // - 滚动容器本行（[data-conversation-scroll]）：是容器不是行。
    // 作用域：只在 dsh 会话流容器 [data-chat-flow] 内生效，壳页（index.html
    // 等）不受影响。`initialization_script` 会随导航注入 dsh 工作台页。
    let webview_memory_policy = r#"(function () {
      if (window.__dshDockMemoryPolicyApplied) return;
      window.__dshDockMemoryPolicyApplied = true;
      var ROW = '[data-chat-anchor-key]';
      var FLOW = '[data-chat-flow]';

      function shouldSkip(row) {
        if (row.querySelector('[data-streaming]')) return true; // 流式增长中
        if (row.children.length === 0) return true;              // 空行
        return false;
      }

      function applyTo(row) {
        if (row.dataset.dshCvBound === '1') return;
        if (shouldSkip(row)) return;
        row.dataset.dshCvBound = '1';
        row.style.setProperty('content-visibility', 'auto');
        // 占位尺寸：避免视口外行被跳过后滚动条抖动（1 行 ~26px，最坏常数）。
        row.style.setProperty('contain-intrinsic-size', 'auto 1px');
      }

      function scan(root) {
        if (!root || !root.querySelectorAll) return;
        var rows = root.querySelectorAll(ROW);
        for (var i = 0; i < rows.length; i++) {
          var row = rows[i];
          var flow = row.closest(FLOW);
          if (flow !== null) applyTo(row); // 只在会话流内生效
        }
      }

      // 初始 + 动态插入（会话长、节点持续进入）。
      scan(document);
      var mo = new MutationObserver(function (muts) {
        for (var m = 0; m < muts.length; m++) {
          var mut = muts[m];
          if (mut.type !== 'childList') continue;
          if (mut.target && mut.target.closest && mut.target.closest(FLOW) !== null) {
            scan(mut.target);
          }
        }
      });
      mo.observe(document.body, { childList: true, subtree: true });
    })();"#;

    tauri::WebviewWindowBuilder::new(
        app,
        "main",
        tauri::WebviewUrl::App("index.html".into()),
    )
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
        let loopback_dsh =
            matches!(url.host_str(), Some("127.0.0.1") | Some("localhost") | Some("[::1]"));
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
    .initialization_script(hook_script)
    .initialization_script(webview_memory_policy)
    .build()
}

/// dsh 启动等待的硬上限（2026-08-24 放宽）：Windows 冷启动被 Defender 首扫 /
/// Node 冷加载吃掉的实测远超过 20s（点 2 次重试才起）。放宽容许慢机，
/// 同时靠 `wait_for_ready`（进程退出即判败/停滞判卡死）避免"真失败干等"。
const BOOT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(90);

/// 进程存活且日志无进展的上限：超过即视为疑似卡死（防死等）。
const BOOT_STALL: std::time::Duration = std::time::Duration::from_secs(20);

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
        // 桌面客户端自更新：check → download → install。只经壳内 IPC 调用
        // （updater.rs 封装），插件命令不暴露给远程页面（最小面纪律）。
        .plugin(tauri_plugin_updater::Builder::new().build())
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

            // 主窗口在 setup 内创建（原 tauri.conf.json 静态定义移除）：只有
            // 代码创建才能挂 on_navigation / on_new_window 处理器——dsh Web UI
            // 里的外链与新窗口请求在 WebView 里默认点不动（2026-08-25 实测），
            // 这里统一转系统默认浏览器。
            let window = create_main_window(app.handle())?;

            let app_handle = app.handle().clone();
            // 必须在任何可失败的检查之前注册：即使 manifest 或 dsh 解析失败，
            // 启动页的 IPC（版本状态、重试）也有可用状态，不能因 `state()` panic
            // 把原本可展示的错误卡变成整个应用退出。
            let state = Arc::new(ShellState {
                dsh: Mutex::new(None),
                window: window.clone(),
                pending: Mutex::new(None),
                update_status: Mutex::new(None),
                workbench_url: Mutex::new(None),
                client_update: Mutex::new(None),
            });
            app.manage(state.clone());
            // 启动页防陈旧缓存：WKWebView 曾把旧版启动页缓存下来（2026-08-23 实测）。
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.clear_all_browsing_data();
            }
            // 更新检测（首启后台异步 + 常驻入口：macOS 应用菜单 / 其余平台托盘）。
            // 先给"检测中"初始状态，背景线程完成后经 boot:update 推送并刷新入口。
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
                // 非 macOS：托盘在 setup 早期创建（含初始菜单），之后统一走
                // refresh_app_menu 刷新；macOS 则在此时设置应用菜单。
                #[cfg(not(target_os = "macos"))]
                setup_update_tray(&app_handle, &status)?;
                refresh_app_menu(&app_handle, &state);
                emit_update(&app_handle, &status);
            }
            let s2 = state.clone();
            let h2 = app.handle().clone();
            std::thread::spawn(move || refresh_update_ui(&h2, &s2));

            // 客户端自更新：启动即后台查一次（顶栏芯片「客户端有新版」靠它点亮；
            // 不打断启动流程，失败静默——状态机进 failed 由 about 页展示）。
            {
                let cstate = state.clone();
                let chandle = app.handle().clone();
                crate::updater::run_check(chandle, cstate);
            }

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
                "about" => open_about_window(&handle),
                "open_in_browser" => {
                    let state = handle.state::<Arc<ShellState>>().inner().clone();
                    let url = state.workbench_url.lock().unwrap().clone();
                    if let Some(url) = url {
                        if let Err(e) = open::that_detached(url.as_str()) {
                            tracing::error!("浏览器打开工作台失败：{e}");
                        }
                    }
                }
                "quit" => handle.exit(0),
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            choose_profile,
            terminal_action,
            get_update_status,
            check_updates,
            get_client_update,
            client_update_check,
            client_update_apply,
            open_about,
            open_external,
            open_workbench_in_browser,
            get_workbench_url
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

// ---------- 更新应用菜单（仅 macOS 菜单栏；托盘已砍，裁定 2026-08-23） ----------
//
// 注意：muda 的原生菜单在 macOS 进系统菜单栏，但在 Windows/Linux 会渲染成
// 窗口内菜单条（2026-08-24 Windows 实测：标题栏下多出「dsh-dock · 编辑」一排
// 工具条，丑）。因此窗口菜单构建/设置一律 `#[cfg(target_os = "macos")]` 门控；
// 非 macOS 的更新/关于入口 = 系统托盘（2026-08-24 裁定）+
// 前端顶栏「关于」按钮（open_about IPC）。

/// dsh 状态行文案（macOS 菜单与托盘菜单共用）。
fn status_line_for(dsh: &crate::updates::ComponentUpdate) -> String {
    if dsh.error.is_some() {
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
    }
}

/// 组装应用菜单：macOS 菜单栏结构 = 根菜单内放「App 子菜单」+「编辑」子菜单。
/// 第一个子菜单被 macOS 自动视为 App 菜单（标题取 app 名、图标取 bundle 图标）——
/// 平铺 MenuItem 会导致菜单栏出现齿轮占位图标（2026-08-23 实测）。
/// App 菜单内容：状态行 → 检查更新…（⌘U）→ 升级到 X → 关于 → 标准项。
#[cfg(target_os = "macos")]
fn build_app_menu(
    app: &tauri::AppHandle,
    status: &crate::updates::UpdateStatus,
) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{MenuBuilder, MenuItem, PredefinedMenuItem, SubmenuBuilder};

    let dsh = &status.dsh;
    let st = MenuItem::with_id(app, "st", status_line_for(dsh), false, None::<&str>)?;
    let check = MenuItem::with_id(app, "check", "检查更新…", true, Some("CmdOrCtrl+U"))?;
    let upgrade = MenuItem::with_id(
        app,
        "upgrade",
        format!("升级到 {}", dsh.latest.clone().unwrap_or_default()),
        dsh.newer,
        None::<&str>,
    )?;
    let about = MenuItem::with_id(app, "about", "关于", true, None::<&str>)?;
    let in_browser = MenuItem::with_id(
        app,
        "open_in_browser",
        "在浏览器中打开",
        true,
        None::<&str>,
    )?;
    let sep = PredefinedMenuItem::separator(app)?;

    // App 子菜单（macOS 忽略其 text，标题自动为 app 名）
    let app_menu = SubmenuBuilder::new(app, "dsh-dock")
        .item(&st)
        .item(&check)
        .item(&upgrade)
        .item(&sep)
        .item(&in_browser)
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
/// 仅 macOS 有系统菜单栏；其余平台不设原生菜单（窗口内菜单条丑，2026-08-24 裁定）。
#[cfg(target_os = "macos")]
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

/// 非 macOS：常驻更新入口 = 系统托盘（2026-08-24 裁定：Windows/Linux 原生菜单
/// 会渲染成窗口内菜单条，丑；托盘菜单承载 状态行/检查更新/升级/关于/退出）。
#[cfg(not(target_os = "macos"))]
fn refresh_app_menu(app: &tauri::AppHandle, state: &Arc<ShellState>) {
    let status = state
        .update_status
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(empty_update_status);
    if let Some(tray) = app.tray_by_id("main") {
        if let Ok(menu) = build_tray_menu(app, &status) {
            let _ = tray.set_menu(Some(menu));
        }
    }
}

/// 托盘菜单：状态行(禁用) / 检查更新… / 升级到 X / 关于 / 退出。
/// 事件经 builder 级 on_menu_event（全局）送达，id 与 macOS 菜单一致：
/// check / upgrade / about / quit。
#[cfg(not(target_os = "macos"))]
fn build_tray_menu(
    app: &tauri::AppHandle,
    status: &crate::updates::UpdateStatus,
) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    use tauri::menu::{MenuBuilder, MenuItem, PredefinedMenuItem};

    let dsh = &status.dsh;
    let st = MenuItem::with_id(app, "st", status_line_for(dsh), false, None::<&str>)?;
    let check = MenuItem::with_id(app, "check", "检查更新…", true, None::<&str>)?;
    let upgrade = MenuItem::with_id(
        app,
        "upgrade",
        format!("升级到 {}", dsh.latest.clone().unwrap_or_default()),
        dsh.newer,
        None::<&str>,
    )?;
    let about = MenuItem::with_id(app, "about", "关于", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let in_browser = MenuItem::with_id(app, "open_in_browser", "在浏览器中打开", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;

    MenuBuilder::new(app)
        .item(&st)
        .item(&check)
        .item(&upgrade)
        .item(&sep)
        .item(&in_browser)
        .item(&about)
        .item(&sep)
        .item(&quit)
        .build()
}

/// setup 阶段创建托盘（非 macOS）：左键唤起主窗口，右键出菜单。
#[cfg(not(target_os = "macos"))]
fn setup_update_tray(
    app: &tauri::AppHandle,
    status: &crate::updates::UpdateStatus,
) -> tauri::Result<()> {
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let menu = build_tray_menu(app, status)?;
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

/// 关于面板：独立小窗（壳版本 + 宿主 dsh 版本 + 检查/升级），复用 ui/about.html。
fn open_about_window(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("about") {
        let _ = win.show();
        let _ = win.set_focus();
        return;
    }
    let builder =
        tauri::WebviewWindowBuilder::new(app, "about", tauri::WebviewUrl::App("about.html".into()))
            .title("关于")
            // 480x360 装不下三行维度 + 浏览器入口 + 脚注（2026-08-25 实测裁切）；
            // 更新中心接入后内容更高（客户端状态机卡片 + dimrows + 入口 + 脚注），
            // 加高并允许滚动兜底。
            .inner_size(480.0, 560.0)
            .min_inner_size(440.0, 460.0)
            .resizable(true)
            .center();
    let _ = builder.build();
}
