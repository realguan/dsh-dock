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
//! 启动过程全链路可视化（壳页面：frontend/src/pages/BootIndex.tsx）。

mod boot;
mod build_approvals;
mod commands;
mod credentials;
mod diagnostics;
mod dsh_settings;
mod engines;
mod executor;
mod fs_backup;
pub mod ipc;
mod manifest;
mod mcp;
mod plugins;
mod profiles;
mod resolve;
mod sessions;
mod settings;
mod shell;
mod ui;
mod updater;
mod updates;

use std::path::Path;
use std::process::Command;
use std::sync::atomic::AtomicU64;
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
        p.extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .as_deref(),
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
        assert!(is_batch_script(Path::new(
            r"C:\Users\me\AppData\Roaming\npm\pnpm.cmd"
        )));
        assert!(is_batch_script(Path::new(r"D:\tools\setup.bat")));
        assert!(is_batch_script(Path::new("pnpm.CMD")));
        assert!(!is_batch_script(Path::new(
            r"C:\Program Files\nodejs\node.exe"
        )));
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

/// 外链白名单：只放行 http/https 且主机在白名单内的 URL（壳的 IPC 不应成为
/// 任意 URL 的跳板）。dsh Web UI 的外链（文档/官网/控制台）都应落在这里；
/// 未收录的域会被拒绝——需要新域时在此登记。
pub(crate) const EXTERNAL_URL_HOSTS: &[&str] = &[
    "commandcode.ai",
    "www.commandcode.ai",
    "api.commandcode.ai",
    "deepseek.com",
    "www.deepseek.com",
    "platform.deepseek.com",
    "github.com",
    "awesome-dsh-plugin.com",
    "npmjs.com",
    "www.npmjs.com",
];

/// 校验外链：仅 http/https，且主机等于或以白名单域结尾（`.example.com` 子域）。
pub(crate) fn is_allowed_external_url(raw: &str) -> bool {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // unix：SIGTERM/SIGINT 优雅退出（2026-09-07）。RunEvent::Exit 只覆盖优雅
    // 退出路径——SIGTERM（pkill、`cargo tauri dev` 重编重启）与 SIGINT（终端
    // Ctrl-C）会直接杀死进程，会话 dsh 子进程无人收（§6 1:1 生命周期被绕过，
    // 实测留孤儿）。处理器只置原子位（async-signal-safe），监护线程命中后走
    // app.exit(0) 复用 RunEvent::Exit 的统一 teardown。Windows 无对应信号面
    // （taskkill/TerminateProcess 本就无清理机会），不装。
    #[cfg(unix)]
    boot::install_signal_exit_handler();
    // 日志初始化移入 setup（落 shell.log；dev 另双写 stdout，见 init_tracing）。
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
        // 连接超时 / 镜像回退在 updater.rs 的 UpdaterBuilder 上配置（该 builder
        // 同时作用于 check 与 download，见 updater.rs blocked_check）。
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // 壳侧诊断日志落 `<数据目录>/shell.log`；dev（debug 构建）双写
            // stdout——`cargo tauri dev` 终端实时可见 boot/引擎引导/网络各阶段
            // （release 纯文件：GUI 下 stdout 无处可去，Windows 无控制台）。
            // 子进程输出在 dsh-shell.log（shell.rs）；两者分离。
            if let Ok(data_dir) = app.path().app_data_dir() {
                if let Ok(file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(data_dir.join("shell.log"))
                {
                    boot::init_tracing(file);
                }
            }
            // 资源根解析（dev/prod 差异）已由 executor_for_mode（Local 档）内部处理：
            //   - 生产（bundle）：Tauri v2 保留相对 src-tauri 的路径前缀，
            //     `resources/**` 落在 `.app/Contents/Resources/resources/`；
            //   - dev（cargo run，macOS）：resource_dir() 指向不存在的 target/Resources，
            //     回退链：exe_dir/resources（tauri-build 的副本，Windows 语义）→
            //     CARGO_MANIFEST_DIR/resources（源码树，本仓库开发常态）。
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;

            // 主窗口在 setup 内创建（原 tauri.conf.json 静态定义移除）：只有
            // 代码创建才能挂 on_navigation / on_new_window 处理器——dsh Web UI
            // 里的外链与新窗口请求在 WebView 里默认点不动（2026-08-25 实测），
            // 这里统一转系统默认浏览器。
            let window = ui::create_main_window(app.handle())?;

            let app_handle = app.handle().clone();
            // 必须在任何可失败的检查之前注册：即使 manifest 或 dsh 解析失败，
            // 启动页的 IPC（版本状态、重试）也有可用状态，不能因 `state()` panic
            // 把原本可展示的错误卡变成整个应用退出。
            let state = Arc::new(boot::ShellState {
                session: Mutex::new(None),
                session_epoch: AtomicU64::new(0),
                active_mode: Mutex::new(None),
                window: window.clone(),
                pending: Mutex::new(None),
                update_status: Mutex::new(None),
                workbench_url: Mutex::new(None),
                forced_profile: Mutex::new(None),
                client_update: Mutex::new(None),
                crash_timestamps: Mutex::new(Vec::new()),
                boot_error: Mutex::new(None),
                boot_steps: Mutex::new(Vec::new()),
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
                        current: crate::updates::detect_current_version(&data_dir),
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
                // 边界澄清（2026-09-07 维护者裁定）：托盘不可用（无 dbus 会话
                // 总线 / 无 StatusNotifier 宿主，如 i3/sway/精简桌面）降级 warn
                // 不阻断启动——常驻更新入口缺失可接受，应用不可用不可接受；
                // ADR-0007 边界澄清，落档 broadcasts。
                #[cfg(not(target_os = "macos"))]
                if let Err(e) = ui::setup_update_tray(&app_handle) {
                    tracing::warn!("托盘初始化失败（常驻更新入口缺失，应用继续）：{e}");
                }
                ui::refresh_app_menu(&app_handle, &state);
                boot::emit_update(&app_handle, &status);
            }
            let s2 = state.clone();
            let h2 = app.handle().clone();
            std::thread::spawn(move || boot::refresh_update_ui(&h2, &s2));

            // 客户端自更新：启动即后台查一次（顶栏芯片「客户端有新版」靠它点亮；
            // 不打断启动流程，失败静默——状态机进 failed 由 about 页展示）。
            {
                let cstate = state.clone();
                let chandle = app.handle().clone();
                crate::updater::run_check(chandle, cstate);
            }

            // 宿主解析可能触发网络下载和 npm 安装，必须在后台执行：setup 运行在
            // 主线程，阻塞它会让 macOS 把窗口判定为无响应，也会让早期错误事件丢失。
            // 启动路径（local/wsl 同等地位，见 settings.rs）：
            //   已设默认 → 按默认模式构建执行器并启动；
            //   非 Windows（含首次）→ 一律本机启动（WSL 只存在于 Windows，
            //   零 WSL 感知，2026-08-26 裁定）；Windows 首次未设默认 → 导航
            //   mode.html 让用户先选运行环境，经 choose_mode 落地。
            let boot_state = state.clone();
            let boot_app = app_handle.clone();
            let boot_data = data_dir.clone();
            std::thread::spawn(move || {
                let settings = crate::settings::load(&boot_data);
                let mode = settings.default_mode.unwrap_or(settings::Mode::Local);
                // 非 Windows：WSL 不存在（执行器编译为报错）——settings 里残留的
                // wsl（如从 Windows 拷来的数据目录）一律按 local 启动，绝不进
                // 环境选择页 / 不出 WSL 痕迹（2026-08-26 裁定）。
                let mode = if cfg!(windows) {
                    mode
                } else {
                    settings::Mode::Local
                };
                *boot_state.active_mode.lock().unwrap() = Some(mode);
                match boot::executor_for_mode(mode, &boot_app, boot_data) {
                    Ok(executor) => {
                        boot::launch_executor_after_probe(boot_state, boot_app, executor)
                    }
                    Err(e) => {
                        tracing::error!("启动失败（{e}）");
                        boot::emit_boot_error(&boot_app, &e, "");
                    }
                }
            });
            // 信号监护（unix，install_signal_exit_handler 置位）：命中后走
            // app.exit(0)，让 RunEvent::Exit 的统一 teardown 收干净会话子进程。
            #[cfg(unix)]
            {
                let sig_app = app.handle().clone();
                std::thread::spawn(move || loop {
                    if boot::SIGNAL_EXIT.load(std::sync::atomic::Ordering::SeqCst) {
                        tracing::info!("收到终止信号，优雅退出（收会话子进程）");
                        sig_app.exit(0);
                        return;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                });
            }
            Ok(())
        })
        .on_menu_event(|app, event| {
            let state = app.state::<Arc<boot::ShellState>>().inner().clone();
            let handle = app.clone();
            match event.id().as_ref() {
                "about" => ui::open_about_window(&handle),
                "profiles_manager" => commands::window::open_profiles_window(handle),
                "open_in_browser" => {
                    let state = handle.state::<Arc<boot::ShellState>>().inner().clone();
                    let url = state.workbench_url.lock().unwrap().clone();
                    if let Some(url) = url {
                        if let Err(e) = open::that_detached(url.as_str()) {
                            tracing::error!("浏览器打开工作台失败：{e}");
                        }
                    }
                }
                "mode_local" | "mode_wsl" => {
                    let mode = if event.id().as_ref() == "mode_local" {
                        settings::Mode::Local
                    } else {
                        settings::Mode::Wsl
                    };
                    // 切换会 teardown 当前会话（可能等 grace），放后台线程。
                    std::thread::spawn(move || {
                        let Ok(data_dir) = handle.path().app_data_dir() else {
                            return;
                        };
                        boot::switch_mode(handle, state, mode, data_dir);
                    });
                }
                "quit" => handle.exit(0),
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::boot::choose_profile,
            commands::boot::terminal_action,
            commands::update::get_update_status,
            commands::update::check_updates,
            commands::update::get_client_update,
            commands::update::client_update_check,
            commands::update::client_update_apply,
            commands::link::open_external,
            commands::link::open_workbench_in_browser,
            commands::link::get_workbench_url,
            commands::boot::boot_in_wsl,
            commands::boot::choose_mode,
            commands::profile::list_profiles,
            commands::profile::get_profile_detail,
            commands::profile::create_profile,
            commands::profile::copy_profile,
            commands::profile::rename_profile,
            commands::profile::delete_profile,
            commands::profile::set_default_profile,
            commands::profile::get_default_profile,
            commands::profile::switch_profile,
            commands::profile::get_active_profile,
            commands::plugin::list_profile_plugins,
            commands::plugin::install_plugin,
            commands::plugin::remove_plugin,
            commands::plugin::update_plugin,
            commands::plugin::get_plugin_rows,
            commands::plugin::set_plugin_disabled,
            commands::plugin::check_plugin_updates,
            commands::plugin::list_plugin_versions,
            commands::plugin::get_plugin_runtime,
            commands::plugin::list_all_plugins,
            commands::plugin::copy_plugin_config,
            commands::plugin::set_profile_build_approvals,
            commands::session::list_sessions,
            commands::session::repair_session,
            commands::session::repair_all_sessions,
            commands::console::get_shell_settings,
            commands::console::set_shell_settings,
            commands::console::get_system_diagnostics,
            commands::console::get_app_logs,
            commands::console::get_credentials_raw,
            commands::console::save_credentials_raw,
            commands::console::get_credentials_summary,
            commands::console::set_credential_key,
            commands::console::get_dsh_settings_raw,
            commands::console::save_dsh_settings_raw,
            commands::console::list_mcp_servers,
            commands::console::save_mcp_server,
            commands::console::delete_mcp_server,
            commands::session::delete_session,
            commands::market::fetch_market_registry,
            commands::window::open_profiles_window,
            commands::window::focus_main_window,
            commands::boot::get_boot_status,
        ])
        .build(tauri::generate_context!())
        .expect("构建 Tauri app 失败")
        .run(|app_handle, event| {
            // 应用退出 → 会话式 teardown（壳退 = 环境停：停子进程 / 断隧道，同生命周期）。
            if let RunEvent::Exit = event {
                if let Some(state) = app_handle.try_state::<Arc<boot::ShellState>>() {
                    if let Some(mut ex) = state.session.lock().unwrap().take() {
                        let _ = ex.teardown();
                    }
                    // 选择器场景可能尚未 spawn：清理 pending，不留状态。
                    state.pending.lock().unwrap().take();
                }
            }
        });
}

/// 收到 SIGTERM/SIGINT 的标志位（信号处理器只做原子写，async-signal-safe）。
#[cfg(unix)]
#[cfg(test)]
mod tests {
    use super::boot::{cached_status_or_default, classify_boot_error, ensure_switchable_profile};

    #[test]
    fn cookie_parsing_adjusts_samesite_and_domain() {
        let raw = "dsh-auth-enkR2grNa5kh2vWUO35uv_f-LSUholBNvVmCSRPr3-M=v1.eyJ2ZXJzaW9uIjoxLCJhdXRob3JpdHkiOiIxMjcuMC4wLjE6NTc3NDYiLCJpc3N1ZWRBdCI6MTc4ODUxNTg0NDEzNywiZXhwaXJlc0F0IjoxNzkxMTA3ODQ0MTM3fQ.6Ic6MHrPEbFn9jWvIGyIqOUfuz5L5tgvWkTWG2jRu44; Max-Age=2592000; Path=/; Expires=Sun, 04 Oct 2026 09:57:24 GMT; HttpOnly; SameSite=Strict";
        let mut cookie = tauri::webview::cookie::Cookie::parse(raw)
            .unwrap()
            .into_owned();
        assert_eq!(
            cookie.name(),
            "dsh-auth-enkR2grNa5kh2vWUO35uv_f-LSUholBNvVmCSRPr3-M"
        );
        assert_eq!(cookie.http_only(), Some(true));
        cookie.set_domain("127.0.0.1");
        cookie.set_path("/");
        cookie.set_same_site(tauri::webview::cookie::SameSite::Lax);
        assert_eq!(
            cookie.same_site(),
            Some(tauri::webview::cookie::SameSite::Lax)
        );
        assert_eq!(cookie.domain(), Some("127.0.0.1"));
        assert_eq!(cookie.path(), Some("/"));
    }

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

    #[test]
    fn switch_target_must_be_webui_candidate() {
        let cands = vec!["web".to_string(), "33".to_string()];
        // 候选内：web 与自定义 webUi 均可
        assert!(ensure_switchable_profile("web", &cands).is_ok());
        assert!(ensure_switchable_profile("33", &cands).is_ok());
        // 非 webUi（headless）/ 不存在 / 恶意名：拒绝且文案可行动
        for bad in ["headless", "ghost", "../escape", "node_modules"] {
            let err = ensure_switchable_profile(bad, &cands).unwrap_err();
            assert!(err.contains("webUi"), "{bad} -> {err}");
        }
    }

    /// 版本一致性契约（2026-09-01 维护者裁定）：关于页/更新中心的当前版本
    /// 显示与 updater 比较均锚定 Cargo.toml（CARGO_PKG_VERSION），而打包产物
    /// 版本来自 tauri.conf.json；两者漂移 = 「已升级但仍提示更新」事故。
    #[test]
    fn cargo_version_matches_tauri_conf() {
        let cargo = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
            .expect("读取 Cargo.toml");
        let conf = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tauri.conf.json"))
            .expect("读取 tauri.conf.json");
        let pkg_json = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../frontend/package.json"
        ))
        .expect("读取 frontend/package.json");
        let pkg_version = env!("CARGO_PKG_VERSION");
        assert!(
            cargo.contains(&format!("version = \"{pkg_version}\"")),
            "Cargo.toml 的 version 与 CARGO_PKG_VERSION 不一致：预期 {pkg_version}"
        );
        assert!(
            conf.contains(&format!("\"version\": \"{pkg_version}\"")),
            "tauri.conf.json 的 version 与 CARGO_PKG_VERSION 不一致：预期 {pkg_version}"
        );
        assert!(
            pkg_json.contains(&format!("\"version\": \"{pkg_version}\"")),
            "frontend/package.json 的 version 与 CARGO_PKG_VERSION 不一致：预期 {pkg_version}"
        );
    }

    #[test]
    fn webview_memory_policy_script_contains_list_padding_defense() {
        assert!(crate::ui::WEBVIEW_MEMORY_POLICY_SCRIPT.contains("content-visibility: auto"));
        assert!(crate::ui::WEBVIEW_MEMORY_POLICY_SCRIPT.contains("padding-left: 1.5em !important"));
        assert!(crate::ui::WEBVIEW_MEMORY_POLICY_SCRIPT.contains("FLOW + ' > ' + ROW"));
    }
}

// ---------- 更新应用菜单（仅 macOS 菜单栏；托盘已砍，裁定 2026-08-23） ----------
//
// 注意：muda 的原生菜单在 macOS 进系统菜单栏，但在 Windows/Linux 会渲染成
// 窗口内菜单条（2026-08-24 Windows 实测：标题栏下多出「dsh-dock · 编辑」一排
// 工具条，丑）。因此窗口菜单构建/设置一律 `#[cfg(target_os = "macos")]` 门控；
// 非 macOS 的更新/关于入口 = 系统托盘（2026-08-24 裁定）。前端顶栏「关于」
// 按钮与原生常驻入口重复，连同 open_about IPC 一并删除（2026-08-27 裁定）。
