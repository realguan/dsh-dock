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

mod build_approvals;
mod commands;
mod credentials;
mod diagnostics;
mod dsh_settings;
mod engines;
mod executor;
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

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
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

/// 壳运行时状态：当前执行环境会话（executor）+ 主窗口句柄 + 待选 profile 的会话。
pub(crate) struct ShellState {
    /// 当前会话的执行器（local / wsl；ssh 预留）。等待/监护线程对它做短锁轮询，
    /// 不独占锁——退出处理器随时能拿到会话做 teardown（同生命周期纪律）。
    session: crate::executor::Session,
    /// 会话代际：每次 teardown/切换递增。等待/监护线程启动时记录自己的代际，
    /// 会话被外部切换（如 boot_in_wsl 停掉旧会话）后旧线程立即静默退出，
    /// 不会在 90s 后误报错误卡、也不会误监护新会话。
    session_epoch: AtomicU64,
    /// 当前会话的运行环境（菜单勾选 / retry 重建用；None=尚未启动会话）。
    active_mode: Mutex<Option<settings::Mode>>,
    window: tauri::WebviewWindow,
    /// 选择器场景：probe 完成但尚未 spawn 的会话（用户选定 profile 后落地）。
    pending: Mutex<Option<Box<dyn crate::executor::Executor>>>,
    /// 最近一次更新检测结果（前端 chip / 托盘菜单共用）。
    update_status: Mutex<Option<crate::updates::UpdateStatus>>,
    /// 当前工作台地址（dsh 就绪导航时记录；「在浏览器中打开」入口用）。
    workbench_url: Mutex<Option<tauri::Url>>,
    /// 强制启动目标（4.3⑥ 管理器切换写入；错误卡重试经注入延续同一目标；
    /// 模式切换清空重走常规解析）。probe 内按档位消费——bundle 快照档忽略。
    /// 与 `active_session_profile`（会话槽真相：删除/重命名防护、
    /// get_active_profile 数据源）分工——本字段是「目标记录」，非「运行真相」。
    forced_profile: Mutex<Option<String>>,
    /// 桌面客户端自更新状态机（updater.rs；Rust 侧唯一写者，前端只读）。
    client_update: Mutex<Option<crate::updater::ClientUpdate>>,
    /// 崩溃历史时间戳（4.12 崩溃守护与熔断，记录最近 60s 内异常退出次数）。
    crash_timestamps: Mutex<Vec<std::time::Instant>>,
    /// 最近发射的 boot:error（用于前端挂载后经 get_boot_status 补水，防早期事件竞态丢失）。
    boot_error: Mutex<Option<serde_json::Value>>,
    /// 最近发射的 boot:step 列表（用于前端挂载后经 get_boot_status 补水）。
    boot_steps: Mutex<Vec<serde_json::Value>>,
}

impl ShellState {
    fn clear_boot_cache(&self) {
        if let Ok(mut err) = self.boot_error.lock() {
            *err = None;
        }
        if let Ok(mut steps) = self.boot_steps.lock() {
            steps.clear();
        }
    }
}

/// 启动当前会话（probe 已完成）：start → 就绪等待 → 导航 → 监护。
/// 统一入口，与执行环境（local / wsl）无关——具体动作经 BootSink 上抛、
/// 就绪经 executor::log_path + check_exited 轮询。
pub(crate) fn run_executor_session(
    state: Arc<ShellState>,
    app: tauri::AppHandle,
    mut executor: Box<dyn crate::executor::Executor>,
) -> Result<(), String> {
    tracing::info!("启动 {} 执行环境会话", executor.kind().as_str());
    {
        let mut sink = boot_sink(&app);
        if let Err(e) = executor.start(&mut sink) {
            emit_step(&app, 2, "error", &e);
            emit_boot_error(&app, &e, "");
            return Err(e);
        }
    } // sink 借 app 结束，之后 app 可安全 move 进监护线程
    let log = executor.log_path();
    // 记录本线程的会话代际：若等待期间会话被外部切换（teardown_session），
    // 旧线程据此静默退出，不误报错误卡、不误导航/监护新会话。
    let epoch = state.session_epoch.fetch_add(1, Ordering::SeqCst) + 1;
    *state.session.lock().unwrap() = Some(executor);

    std::thread::spawn(move || {
        // 短锁轮询：每轮锁一次会话槽检查是否已退出（不独占锁 90s，
        // 退出处理器随时能拿到会话做 teardown——壳与 dsh 同生命周期）。
        let mut exited = || {
            state
                .session
                .lock()
                .unwrap()
                .as_mut()
                .and_then(|e| e.check_exited())
        };
        // 就绪标记读取器（WSL 用，绕开 wsl.exe 输出缓冲——见 executor.rs
        // guest_boot_script 注释）：本地执行器 read_ready_marker 默认返回 None，
        // wait_for_ready 立即走 log 路径。
        let mut marker = || {
            state
                .session
                .lock()
                .unwrap()
                .as_mut()
                .and_then(|e| e.read_ready_marker())
        };
        // 等待期运转指示：step2 已在 spawn 成功后收口，step3 从未发过 running
        // ——时间线上「启动工作台」永挂 loading 而「等待就绪」凭空 done（倒挂）。
        // 发 running 让卡头在等待期正确显示「等待就绪」。
        emit_step(&app, 3, "running", "等待 DSH 服务就绪…");
        match crate::shell::wait_for_ready(&log, &mut exited, &mut marker, BOOT_STALL, BOOT_TIMEOUT)
        {
            shell::ReadyOutcome::Exited(code) => {
                if !session_is_current(&state, epoch) {
                    return; // 已被外部切换（模式切换/退出）：静默，不出错误卡
                }
                // 会话先退出了：真失败，立即报错（不等满上限）。
                let detail = read_error_detail(&log);
                let tail = read_log_tail(&log);
                emit_step(&app, 3, "error", &format!("DSH 意外退出（代码 {code}）"));
                let _ = teardown_session(&state);
                emit_boot_error(
                    &app,
                    &format!("DSH 进程已退出（代码 {code}）{detail}"),
                    &tail,
                );
            }
            shell::ReadyOutcome::Stalled => {
                if !session_is_current(&state, epoch) {
                    return; // 已被外部切换（模式切换/退出）：静默，不出错误卡
                }
                // 进程活着但长时间未就绪：停掉旧会话再报错，重试不残留孤儿。
                let detail = read_error_detail(&log);
                let tail = read_log_tail(&log);
                emit_step(&app, 3, "error", "等待服务响应超时");
                let _ = teardown_session(&state);
                emit_boot_error(&app, &format!("DSH 未在预期时间内就绪{detail}"), &tail);
            }
            shell::ReadyOutcome::Ready(raw) => {
                if !session_is_current(&state, epoch) {
                    return; // 就绪前被切走：不再导航/监护新会话
                }
                match tauri::Url::parse(&raw) {
                    Ok(url) => {
                        tracing::info!("DSH 已就绪，进入 {url}");
                        *state.workbench_url.lock().unwrap() = Some(url.clone());
                        emit_step(&app, 3, "done", &format!("DSH 已就绪，地址：{url}"));
                        emit_step(&app, 4, "running", "正在打开工作台界面");
                        let navigate_url = match authenticate_workbench_session(&state.window, &url)
                        {
                            Ok(Some(clean_url)) => clean_url,
                            Ok(None) => url.clone(),
                            Err(e) => {
                                tracing::warn!(
                                    "预植入工作台认证 Cookie 失败（回退原地址导航）：{e}"
                                );
                                url.clone()
                            }
                        };
                        let _ = state.window.navigate(navigate_url);
                        emit_step(&app, 4, "done", "已进入工作台");
                        guard_session(&app, &state, &log, epoch);
                    }
                    Err(e) => {
                        emit_step(&app, 3, "error", "服务地址无效");
                        emit_boot_error(&app, &format!("DSH 报告了无效地址（{raw}）：{e}"), "");
                        let _ = teardown_session(&state);
                    }
                }
            }
        }
    });
    Ok(())
}

/// 为 dsh web（0.1.2+）在壳侧先兑换 token 并将 SameSite=Lax 的会话 Cookie 注入 WebView，
/// 从而规避 WebKit 跨域导航（tauri://localhost → 127.0.0.1）在 303 重定向时
/// 丢弃 SameSite=Strict Cookie 导致的 401（dsh web authentication required）。
fn authenticate_workbench_session(
    window: &tauri::WebviewWindow,
    url: &tauri::Url,
) -> anyhow::Result<Option<tauri::Url>> {
    if !url.query().unwrap_or("").contains("token=") {
        return Ok(None);
    }
    tracing::info!("检测到工作台携带 launch token，在壳内预兑换会话 Cookie…");
    let resp = ureq::builder()
        .redirects(0)
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .get(url.as_str())
        .call();

    let set_cookie_str = match &resp {
        Ok(r) => r.header("set-cookie").map(|s| s.to_string()),
        Err(ureq::Error::Status(_, r)) => r.header("set-cookie").map(|s| s.to_string()),
        Err(e) => anyhow::bail!("向工作台请求认证 Token 失败：{e}"),
    };

    let Some(raw_cookie) = set_cookie_str else {
        tracing::warn!("工作台响应未携带 Set-Cookie，回退原地址导航");
        return Ok(None);
    };

    let mut parsed = tauri::webview::cookie::Cookie::parse(raw_cookie.as_str())
        .map_err(|e| anyhow::anyhow!("解析工作台 Cookie 失败: {e}"))?
        .into_owned();

    let domain = url.host_str().unwrap_or("127.0.0.1");
    parsed.set_domain(domain);
    parsed.set_path("/");
    parsed.set_same_site(tauri::webview::cookie::SameSite::Lax);

    window
        .set_cookie(parsed)
        .map_err(|e| anyhow::anyhow!("注入 WebView Cookie 失败: {e}"))?;

    tracing::info!("成功向 WebView 注入工作台会话 Cookie（Lax），准备直达根路径");

    let mut clean_url = url.clone();
    clean_url.set_query(None);
    Ok(Some(clean_url))
}

/// 会话代际是否仍为当前：等待/监护线程每轮/关键节点调用；被外部切换则 false。
fn session_is_current(state: &ShellState, epoch: u64) -> bool {
    state.session_epoch.load(Ordering::SeqCst) == epoch
}

/// dsh 引擎进程存活（2026-09-07）：壳持有的会话执行器 try_wait 判定——
/// 会话槽在且进程未退出 = 存活；无会话（未启动/已 teardown）= 不存活。
/// 「运行中」复合判据前半（会话维护）：dsh 未运行时任何会话文件都不可能
/// 再被写入，mtime 新鲜不再构成活跃；WSL 客体形态下 wsl.exe 子进程即
/// 会话存活代理，同一判定覆盖。
pub(crate) fn engine_session_alive(state: &ShellState) -> bool {
    let mut session = state.session.lock().unwrap();
    // 2026-09-08 修复：is_none_or 会在无会话时误报存活（19ec36e 机械改写引入，
    // 问题记录095 #5——死会话标「进行中」+ 运行态回环查询必败），恢复 None=false。
    session.as_mut().is_some_and(|e| e.check_exited().is_none())
}

/// 取出并清理当前会话（幂等）：错误卡 / 模式切换共用。
/// 同时推进代际——旧等待/监护线程据此静默退出（见 run_executor_session）。
pub(crate) fn teardown_session(state: &Arc<ShellState>) -> Result<(), String> {
    state.session_epoch.fetch_add(1, Ordering::SeqCst);
    if let Some(mut ex) = state.session.lock().unwrap().take() {
        ex.teardown()?;
    }
    Ok(())
}

/// dsh 就绪后的监护：会话与壳同生命周期——会话退出即错误卡。
/// 在就绪导航成功后的同一监护线程内持续运行（不新开线程，避免竞态）。
/// 只监护自己启动的代际：会话被外部切换后立即静默退出（不误监护新会话）。
pub(crate) fn guard_session(
    app: &tauri::AppHandle,
    state: &Arc<ShellState>,
    log: &std::path::Path,
    epoch: u64,
) {
    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if !session_is_current(state, epoch) {
            return;
        }
        let exited = state
            .session
            .lock()
            .unwrap()
            .as_mut()
            .and_then(|e| e.check_exited());
        if let Some(code) = exited {
            if !session_is_current(state, epoch) {
                return;
            }
            let detail = read_error_detail(log);
            let tail = read_log_tail(log);
            tracing::error!("会话异常退出 code={code}{detail}");

            let data_dir = match app.path().app_data_dir() {
                Ok(d) => d,
                Err(_) => {
                    let _ = teardown_session(state);
                    emit_boot_error(
                        app,
                        &format!("DSH 进程已退出（code={code}）{detail}"),
                        &tail,
                    );
                    return;
                }
            };

            let shell_settings = crate::settings::load(&data_dir);
            let should_auto_restart = shell_settings.auto_restart.unwrap_or(false);

            if should_auto_restart {
                let now = std::time::Instant::now();
                let mut crashes = state.crash_timestamps.lock().unwrap();
                // 保留 60 秒内的崩溃记录
                crashes.retain(|t| now.duration_since(*t).as_secs() < 60);

                if crashes.len() >= 3 {
                    tracing::warn!("60 秒内连续崩溃已达 {} 次，触发熔断保护", crashes.len());
                    crashes.clear();
                    let _ = teardown_session(state);
                    emit_boot_error(
                        app,
                        &format!(
                            "DSH 连续崩溃（60 秒内 3 次，已暂停自动重启）：代码 {code}{detail}"
                        ),
                        &tail,
                    );
                    return;
                }

                crashes.push(now);
                drop(crashes);

                tracing::info!("崩溃守护触发：正在自动拉起 DSH 会话...");
                emit_step(app, 2, "running", "检测到 DSH 意外退出，正在自动重启…");

                let handle = app.clone();
                let state_clone = Arc::clone(state);
                let mode = state
                    .active_mode
                    .lock()
                    .unwrap()
                    .unwrap_or(crate::settings::Mode::Local);

                let _ = teardown_session(state);

                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(600));
                    let _ = state_clone.window.eval("location.assign('/')");
                    match executor_for_mode(mode, &handle, data_dir) {
                        Ok(executor) => launch_executor_after_probe(state_clone, handle, executor),
                        Err(e) => emit_boot_error(&handle, &format!("自动恢复启动失败: {e}"), ""),
                    }
                });
                return;
            }

            let _ = teardown_session(state);
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
    crate::resolve::read_log_auto(log_path)
        .lines()
        .rev()
        .take(10)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
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
pub(crate) fn cached_update_status<R: tauri::Runtime>(
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

/// 读当前会话占用中的 profile（运行中防护比对源；无会话/未启动 = None）。
pub(crate) fn active_session_profile(app: &tauri::AppHandle) -> Option<String> {
    let state = app.try_state::<Arc<ShellState>>()?;
    let guard = state.session.lock().ok()?;
    guard
        .as_ref()
        .and_then(|e| e.active_profile().map(String::from))
}

/// 切换目标的可启动性校验（纯函数）：webUi 候选内才可切换——非 webUi
/// （headless / 无 web-app 的自定义档）无 URL 可导航；不存在的名字会被 dsh
/// 拒绝或意外物化。名字合法性已由调用方 `validate_profile_name` 先行把关。
pub(crate) fn ensure_switchable_profile(
    profile: &str,
    webui_candidates: &[String],
) -> Result<(), String> {
    if webui_candidates.iter().any(|c| c == profile) {
        Ok(())
    } else {
        Err(format!(
            "profile「{profile}」不是可启动的 webUi 工作台（无工作台界面或不存在，无法在窗口中切换）。"
        ))
    }
}

/// probe 完成后的统一分派：NeedsProfile → 出选择器（沿用 F-b）；Ready → 启动会话。
/// setup 启动线程 / retry 重试 / boot_in_wsl 切换共用——执行环境不感知。
pub(crate) fn launch_executor_after_probe(
    state: Arc<ShellState>,
    app: tauri::AppHandle,
    mut executor: Box<dyn crate::executor::Executor>,
) {
    // 记录 probe 开始时的会话代际：probe 期间（可能长达分钟级——WSL 自动安装
    // dsh）用户若经菜单/托盘切换了环境，`switch_mode` 会 teardown + epoch++；
    // 旧 probe 线程完成后必须静默丢弃，否则会覆盖新会话（自动安装让窗口变长，
    // 0.4.2 修复前该竞态一直存在，只是窗口小）。
    let probe_epoch = state.session_epoch.load(Ordering::SeqCst);
    // 强制目标注入（4.3⑥ 管理器切换 / 错误卡重试延续）：probe 内按档位消费。
    // 首启与模式切换此处为 None（switch_mode 清空后重走常规解析）。
    executor.set_forced_profile(state.forced_profile.lock().unwrap().clone());
    // probe 借 app 构造两个 sink（boot:step + 下载进度 bridge）；作用域结束即释放，
    // 之后 app 才能 move 进后续动作/监护线程。
    let probe_result = {
        let mut sink = boot_sink(&app);
        let mut progress = download_progress_bridge(&app);
        executor.probe(&mut sink, &mut progress)
    };
    if probe_epoch != state.session_epoch.load(Ordering::SeqCst) {
        tracing::info!("probe 期间会话被切换（epoch 变更），丢弃本次探测结果");
        return;
    }
    match probe_result {
        Err(e) => {
            tracing::error!("环境解析失败: {e}");
            emit_boot_error(&app, &e, "");
        }
        Ok(crate::executor::ProbeOutcome::NeedsProfile(profiles)) => {
            emit_step(&app, 2, "running", "选择器：多个 webUi 工作台");
            // 2026-08-27 前端迁移：selector 由 SPA pathname 路由承载（§3.1）
            let _ = state.window.eval(format!(
                "location.assign('/selector?profiles={}')",
                profiles.join(",")
            ));
            *state.pending.lock().unwrap() = Some(executor);
        }
        Ok(crate::executor::ProbeOutcome::Ready) => {
            // 下载档刚补齐 dsh：立即刷新版本状态（否则关于页/菜单停留在
            // 安装前的「未检出」，要等用户手动检查才正确）。
            if executor.just_installed() {
                let st = state.clone();
                let hd = app.clone();
                std::thread::spawn(move || refresh_update_ui(&hd, &st));
            }
            if let Err(e) = run_executor_session(state, app.clone(), executor) {
                tracing::error!("启动 DSH 失败: {e}");
            }
        }
    }
}

/// 按运行环境构建执行器（local/wsl 同等地位的统一入口：首启 / 重试 / 菜单切换共用）。
pub(crate) fn executor_for_mode(
    mode: settings::Mode,
    app: &tauri::AppHandle,
    data_dir: PathBuf,
) -> Result<Box<dyn crate::executor::Executor>, String> {
    match mode {
        settings::Mode::Local => {
            let resources_dir = ui::resolve_resources_dir(app);
            let manifest =
                manifest::ProductManifest::load(&resources_dir.join("product.manifest.json"))
                    .map_err(|e| format!("产品清单读取失败：{e}"))?;
            Ok(Box::new(crate::executor::LocalExecutor::new(
                manifest,
                resources_dir,
                data_dir,
            )))
        }
        settings::Mode::Wsl => {
            #[cfg(windows)]
            {
                Ok(Box::new(crate::executor::WslExecutor::new(
                    crate::executor::WslConfig { distro: None },
                    data_dir,
                    ui::resolve_resources_dir(app),
                )))
            }
            #[cfg(not(windows))]
            {
                Err("WSL 模式仅支持 Windows 平台。".to_string())
            }
        }
    }
}

/// retry/upgrade 共用：按**当前会话的运行环境**重建执行器并重新走 probe →
/// 分派（不再是永远 local——WSL 会话挂掉后重试仍留在 WSL）。
pub(crate) fn lib_boot_again(state: Arc<ShellState>, app: tauri::AppHandle, data_dir: PathBuf) {
    state.clear_boot_cache();
    let mode = state
        .active_mode
        .lock()
        .unwrap()
        .unwrap_or(settings::Mode::Local);
    *state.active_mode.lock().unwrap() = Some(mode);
    match executor_for_mode(mode, &app, data_dir) {
        Ok(executor) => launch_executor_after_probe(state, app, executor),
        Err(e) => emit_boot_error(&app, &e, ""),
    }
}

/// 模式切换（菜单/托盘共用）：停掉当前会话（幂等）→ 写默认 → 按新模式启动。
/// 切换失败或 probe 失败会把主窗口拉回 SPA 根路径 /（壳错误卡在那里渲染）。
pub(crate) fn switch_mode(
    app: tauri::AppHandle,
    state: Arc<ShellState>,
    mode: settings::Mode,
    data_dir: PathBuf,
) {
    tracing::info!("切换运行环境 → {}", mode.as_str());
    let _ = teardown_session(&state);
    state.clear_boot_cache();
    // 清空强制目标：模式切换重走常规解析（defaultProfile → 选择器），
    // 不继承上一次的 profile 切换目标（4.3⑥）。
    *state.forced_profile.lock().unwrap() = None;
    // load-modify-save：不得整体覆盖 ShellSettings——会把 defaultProfile 等
    // 其他已存字段抹掉（4.3 生命周期刀引入 defaultProfile 后的必然要求）。
    let mut shell_settings = crate::settings::load(&data_dir);
    shell_settings.default_mode = Some(mode);
    if let Err(e) = crate::settings::save(&data_dir, &shell_settings) {
        emit_boot_error(&app, &e, "");
        let _ = state.window.eval("location.assign('/')");
        return;
    }
    *state.active_mode.lock().unwrap() = Some(mode);
    // 立即刷新菜单勾选（✓ 跟随当前模式）。
    ui::refresh_app_menu(&app, &state);
    std::thread::spawn(move || {
        // 菜单切换时页面可能已在工作台（remote，不渲染壳错误卡）：先回启动页，
        // 新会话就绪后 run_executor_session 会把主窗口导航过去。
        let _ = state.window.eval("location.assign('/')");
        match executor_for_mode(mode, &app, data_dir) {
            Ok(executor) => launch_executor_after_probe(state, app, executor),
            Err(e) => emit_boot_error(&app, &e, ""),
        }
    });
}

/// dsh 启动等待的硬上限（2026-08-24 放宽）：Windows 冷启动被 Defender 首扫 /
/// Node 冷加载吃掉的实测远超过 20s（点 2 次重试才起）。放宽容许慢机，
/// 同时靠 `wait_for_ready`（进程退出即判败/停滞判卡死）避免"真失败干等"。
const BOOT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(90);

/// 进程存活且日志无进展的上限：超过即视为疑似卡死（防死等）。
const BOOT_STALL: std::time::Duration = std::time::Duration::from_secs(20);

/// dev 双写 MakeWriter：日志同落文件与 stdout（`cargo tauri dev` 终端实时可见）。
/// 文件写入失败不阻断（追加语义尽力而为），stdout 失败忽略（GUI 无控制台）。
struct TeeWriter {
    file: std::sync::Arc<std::fs::File>,
}

struct TeeWriterGuard {
    file: std::sync::Arc<std::fs::File>,
}

impl std::io::Write for TeeWriterGuard {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _ = self.file.write_all(buf);
        let _ = std::io::stdout().write_all(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        let _ = self.file.flush();
        let _ = std::io::stdout().flush();
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for TeeWriter {
    type Writer = TeeWriterGuard;
    fn make_writer(&'a self) -> Self::Writer {
        TeeWriterGuard {
            file: self.file.clone(),
        }
    }
}

/// 日志初始化（幂等）：release 纯文件；debug 构建双写 stdout——dev 终端是
/// 第一现场（boot 步骤 / 引擎引导 / 网络各阶段全量可见），release 行为不变。
fn init_tracing(file: std::fs::File) {
    let file = std::sync::Arc::new(file);
    #[cfg(debug_assertions)]
    {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_writer(TeeWriter { file })
            .with_target(false)
            .try_init();
    }
    #[cfg(not(debug_assertions))]
    {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_writer(file)
            .with_target(false)
            .try_init();
    }
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
    install_signal_exit_handler();
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
                    init_tracing(file);
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
            let state = Arc::new(ShellState {
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
                match executor_for_mode(mode, &boot_app, boot_data) {
                    Ok(executor) => launch_executor_after_probe(boot_state, boot_app, executor),
                    Err(e) => {
                        tracing::error!("启动失败（{e}）");
                        emit_boot_error(&boot_app, &e, "");
                    }
                }
            });
            // 信号监护（unix，install_signal_exit_handler 置位）：命中后走
            // app.exit(0)，让 RunEvent::Exit 的统一 teardown 收干净会话子进程。
            #[cfg(unix)]
            {
                let sig_app = app.handle().clone();
                std::thread::spawn(move || loop {
                    if SIGNAL_EXIT.load(std::sync::atomic::Ordering::SeqCst) {
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
            let state = app.state::<Arc<ShellState>>().inner().clone();
            let handle = app.clone();
            match event.id().as_ref() {
                "about" => ui::open_about_window(&handle),
                "profiles_manager" => commands::window::open_profiles_window(handle),
                "open_in_browser" => {
                    let state = handle.state::<Arc<ShellState>>().inner().clone();
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
                        switch_mode(handle, state, mode, data_dir);
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
                if let Some(state) = app_handle.try_state::<Arc<ShellState>>() {
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
static SIGNAL_EXIT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(unix)]
extern "C" fn signal_exit_handler(_: i32) {
    SIGNAL_EXIT.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// 安装 SIGTERM/SIGINT 处理器（unix）：仅置位，复杂动作全部留给监护线程。
#[cfg(unix)]
fn install_signal_exit_handler() {
    use nix::sys::signal::{sigaction, SaFlags, SigAction, SigHandler, SigSet, Signal};
    let action = SigAction::new(
        SigHandler::Handler(signal_exit_handler),
        SaFlags::empty(),
        SigSet::empty(),
    );
    for sig in [Signal::SIGTERM, Signal::SIGINT] {
        // 覆盖默认终止行为是本函数的意图：命中后由监护线程走优雅退出。
        unsafe {
            let _ = sigaction(sig, &action);
        }
    }
}

/// 发射 boot:step 事件（state: pending|running|done|error）。
pub(crate) fn emit_step(app: &tauri::AppHandle, step: usize, state: &str, detail: &str) {
    use tauri::Emitter;
    tracing::info!("boot:step step={step} state={state} {detail}");
    let payload = serde_json::json!({
        "step": step,
        "state": state,
        "detail": detail,
    });
    if let Some(shell_state) = app.try_state::<Arc<ShellState>>() {
        if let Ok(mut steps) = shell_state.boot_steps.lock() {
            steps.push(payload.clone());
        }
    }
    let _ = app.emit("boot:step", payload);
}

/// executor 进度回调（BootSink）→ boot:step 事件的适配（executor 保持零
/// tauri 依赖，同 updates.rs 的 DownloadProgress 约定）。
fn boot_sink(app: &tauri::AppHandle) -> impl FnMut(usize, &str, &str) + use<'_> {
    move |step, state, detail| emit_step(app, step, state, detail)
}

/// 下载进度 → `boot:progress` 事件的桥接（updates 模块保持零 tauri 依赖）。
/// 节流：≥100ms 一次；完成（current ≥ total）必发。量的单位随阶段：
/// Node = 字节，Dsh = 包计数（前端按 kind 分形态展示）。
fn download_progress_bridge(
    app: &tauri::AppHandle,
) -> impl FnMut(crate::updates::ProgressStage, u64, Option<u64>) + use<'_> {
    let mut last: Option<std::time::Instant> = None;
    move |stage, current, total| {
        let now = std::time::Instant::now();
        let done = total.is_some_and(|t| current >= t);
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
                "kind": stage.as_str(),
                "current": current,
                "total": total,
            }),
        );
    }
}

/// 发射 `dsh:upgrade` 事件（4.4⑤：DSH 升级链路 running/done/failed，detail =
/// 失败时安装器错误链含 pnpm 输出尾部；广播全窗口，关于页升级按钮消费）。
pub(crate) fn emit_upgrade(app: &tauri::AppHandle, phase: &str, detail: &str) {
    use tauri::Emitter;
    let _ = app.emit(
        "dsh:upgrade",
        serde_json::json!({ "phase": phase, "detail": detail }),
    );
}

/// 发射 boot:error 事件（错误卡数据：标题/详情/建议/可用动作）。
pub(crate) fn emit_boot_error(app: &tauri::AppHandle, detail: &str, log_tail: &str) {
    use tauri::Emitter;
    let (title, suggestion, actions) = classify_boot_error(detail);
    let payload = serde_json::json!({
        "title": title,
        "detail": detail,
        "suggestion": suggestion,
        "actions": actions,
        "log": log_tail,
    });
    if let Some(shell_state) = app.try_state::<Arc<ShellState>>() {
        if let Ok(mut err) = shell_state.boot_error.lock() {
            *err = Some(payload.clone());
        }
    }
    let _ = app.emit("boot:error", payload);
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
    let text = crate::resolve::read_log_auto(log_path);
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
    use super::{cached_status_or_default, classify_boot_error, ensure_switchable_profile};

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

/// 发射 boot:update 事件（前端版本行芯片消费）。
fn emit_update(app: &tauri::AppHandle, status: &crate::updates::UpdateStatus) {
    use tauri::Emitter;
    let _ = app.emit("boot:update", status);
}

/// 后台检测一次并同步应用菜单 + 事件（首启/手动/升级后共用）。
pub(crate) fn refresh_update_ui(app: &tauri::AppHandle, state: &Arc<ShellState>) {
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
    ui::refresh_app_menu(app, state);
    emit_update(app, &status);
}
