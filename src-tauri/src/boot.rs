//! boot.rs —— 启动管线与会话状态（2026-09-08 架构评审批次 2 最后一步从 lib.rs 拆出）。
//!
//! 职责：`ShellState`（壳进程内会话真相源）、executor 会话拉起与守卫、boot 遥测
//! （`emit_step`/`emit_upgrade`/`emit_boot_error`）、启动失败分类、tracing 落盘。
//! `run()` 作为组合根留在 `lib.rs`——它只做装配，不承载领域逻辑。
//!
//! 依赖方向：`boot::` → `crate::{executor, resolve, settings, updater, updates, engines, ui}`；
//! `commands/*` 与 `lib.rs::run` → `boot::`。不反向依赖 `commands::`。

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tauri::Manager;

use crate::{manifest, settings, shell, ui};

/// 壳运行时状态：当前执行环境会话（executor）+ 主窗口句柄 + 待选 profile 的会话。
pub(crate) struct ShellState {
    /// 当前会话的执行器（local / wsl；ssh 预留）。等待/监护线程对它做短锁轮询，
    /// 不独占锁——退出处理器随时能拿到会话做 teardown（同生命周期纪律）。
    pub(crate) session: crate::executor::Session,
    /// 会话代际：每次 teardown/切换递增。等待/监护线程启动时记录自己的代际，
    /// 会话被外部切换（如 boot_in_wsl 停掉旧会话）后旧线程立即静默退出，
    /// 不会在 90s 后误报错误卡、也不会误监护新会话。
    pub(crate) session_epoch: AtomicU64,
    /// 当前会话的运行环境（菜单勾选 / retry 重建用；None=尚未启动会话）。
    pub(crate) active_mode: Mutex<Option<settings::Mode>>,
    pub(crate) window: tauri::WebviewWindow,
    /// 选择器场景：probe 完成但尚未 spawn 的会话（用户选定 profile 后落地）。
    pub(crate) pending: Mutex<Option<Box<dyn crate::executor::Executor>>>,
    /// 最近一次更新检测结果（前端 chip / 托盘菜单共用）。
    pub(crate) update_status: Mutex<Option<crate::updates::UpdateStatus>>,
    /// 当前工作台地址（dsh 就绪导航时记录；「在浏览器中打开」入口用）。
    pub(crate) workbench_url: Mutex<Option<tauri::Url>>,
    /// 强制启动目标（4.3⑥ 管理器切换写入；错误卡重试经注入延续同一目标；
    /// 模式切换清空重走常规解析）。probe 内按档位消费——bundle 快照档忽略。
    /// 与 `active_session_profile`（会话槽真相：删除/重命名防护、
    /// get_active_profile 数据源）分工——本字段是「目标记录」，非「运行真相」。
    pub(crate) forced_profile: Mutex<Option<String>>,
    /// 桌面客户端自更新状态机（updater.rs；Rust 侧唯一写者，前端只读）。
    pub(crate) client_update: Mutex<Option<crate::updater::ClientUpdate>>,
    /// 崩溃历史时间戳（4.12 崩溃守护与熔断，记录最近 60s 内异常退出次数）。
    pub(crate) crash_timestamps: Mutex<Vec<std::time::Instant>>,
    /// 最近发射的 boot:error（用于前端挂载后经 get_boot_status 补水，防早期事件竞态丢失）。
    pub(crate) boot_error: Mutex<Option<serde_json::Value>>,
    /// 最近发射的 boot:step 列表（用于前端挂载后经 get_boot_status 补水）。
    pub(crate) boot_steps: Mutex<Vec<serde_json::Value>>,
}

impl ShellState {
    pub(crate) fn clear_boot_cache(&self) {
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
pub(crate) fn authenticate_workbench_session(
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
pub(crate) fn session_is_current(state: &ShellState, epoch: u64) -> bool {
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
                    let shell_url = ui::shell_app_url(&handle);
                    let _ = state_clone.window.navigate(shell_url);
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
pub(crate) fn read_log_tail(log_path: &std::path::Path) -> String {
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
pub(crate) fn empty_update_status() -> crate::updates::UpdateStatus {
    let none_component = crate::updates::ComponentUpdate {
        current: None,
        latest: None,
        newer: false,
        error: None,
        preview_latest: None,
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
pub(crate) fn cached_status_or_default(
    cached: Option<crate::updates::UpdateStatus>,
) -> crate::updates::UpdateStatus {
    cached.unwrap_or_else(empty_update_status)
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
            // 优先调用 React SPA 路由桥接（零闪烁平滑直达），未就绪时回退 location.assign
            let target = format!("/selector?profiles={}", profiles.join(","));
            let _ = state.window.eval(format!(
                "if (typeof window.__DSH_NAVIGATE__ === 'function') {{ window.__DSH_NAVIGATE__('{target}'); }} else {{ location.assign('{target}'); }}"
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
        let shell_url = ui::shell_app_url(&app);
        let _ = state.window.navigate(shell_url);
        return;
    }
    *state.active_mode.lock().unwrap() = Some(mode);
    // 立即刷新菜单勾选（✓ 跟随当前模式）。
    ui::refresh_app_menu(&app, &state);
    let app_handle = app.clone();
    std::thread::spawn(move || {
        // 菜单切换时页面可能已在工作台（remote，不渲染壳错误卡）：先回启动页，
        // 新会话就绪后 run_executor_session 会把主窗口导航过去。
        let shell_url = ui::shell_app_url(&app_handle);
        let _ = state.window.navigate(shell_url);
        match executor_for_mode(mode, &app, data_dir) {
            Ok(executor) => launch_executor_after_probe(state, app, executor),
            Err(e) => emit_boot_error(&app, &e, ""),
        }
    });
}

/// dsh 启动等待的硬上限（2026-08-24 放宽）：Windows 冷启动被 Defender 首扫 /
/// Node 冷加载吃掉的实测远超过 20s（点 2 次重试才起）。放宽容许慢机，
/// 同时靠 `wait_for_ready`（进程退出即判败/停滞判卡死）避免"真失败干等"。
pub(crate) const BOOT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(90);

/// 进程存活且日志无进展的上限：超过即视为疑似卡死（防死等）。
pub(crate) const BOOT_STALL: std::time::Duration = std::time::Duration::from_secs(20);

/// dev 双写 MakeWriter：日志同落文件与 stdout（`cargo tauri dev` 终端实时可见）。
/// 文件写入失败不阻断（追加语义尽力而为），stdout 失败忽略（GUI 无控制台）。
pub(crate) struct TeeWriter {
    file: std::sync::Arc<std::fs::File>,
}

pub(crate) struct TeeWriterGuard {
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

/// 日志初始化（幂等）：release 纯文件；debug 构建双写 stdout——dev 终端是
/// 第一现场（boot 步骤 / 引擎引导 / 网络各阶段全量可见），release 行为不变。
pub(crate) fn init_tracing(file: std::fs::File) {
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

/// 安装 SIGTERM/SIGINT 处理器（unix）：仅置位，复杂动作全部留给监护线程。
#[cfg(unix)]
pub(crate) fn install_signal_exit_handler() {
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
pub(crate) fn boot_sink(app: &tauri::AppHandle) -> impl FnMut(usize, &str, &str) + use<'_> {
    move |step, state, detail| emit_step(app, step, state, detail)
}

/// 下载进度 → `boot:progress` 事件的桥接（updates 模块保持零 tauri 依赖）。
/// 节流：≥100ms 一次；完成（current ≥ total）必发。量的单位随阶段：
/// Node = 字节，Dsh = 包计数（前端按 kind 分形态展示）。
pub(crate) fn download_progress_bridge(
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
/// `installed` 仅 done 阶段有意义：false = 目标版本与已装一致短路跳过
/// （前端提示「已是最新」而非「升级完成」）。
pub(crate) fn emit_upgrade(app: &tauri::AppHandle, phase: &str, detail: &str, installed: bool) {
    use tauri::Emitter;
    let _ = app.emit(
        "dsh:upgrade",
        serde_json::json!({ "phase": phase, "detail": detail, "installed": installed }),
    );
}

/// 发射 boot:error 事件（错误卡数据：分类 + 标题/详情/建议/可用动作）。
///
/// 2026-09-08（ADR-0012）：分类改为 `BootFailure`（tagged enum，`kind` 判别式），
/// 外部文本经兜底表转换；前端按 `kind` 取本地化文案，`title`/`suggestion` 保留为
/// 兼容分支。
pub(crate) fn emit_boot_error(app: &tauri::AppHandle, detail: &str, log_tail: &str) {
    use tauri::Emitter;
    let payload = crate::boot_failure::BootErrorPayload::classify(detail, log_tail);
    let value = serde_json::to_value(&payload).unwrap_or_else(|e| {
        tracing::error!("boot:error 载荷序列化失败: {e}");
        serde_json::json!({ "detail": detail, "log": log_tail })
    });
    if let Some(shell_state) = app.try_state::<Arc<ShellState>>() {
        if let Ok(mut err) = shell_state.boot_error.lock() {
            *err = Some(value.clone());
        }
    }
    let _ = app.emit("boot:error", value);
}

/// 从日志提取崩溃原因摘要（首条顶层 Error 行，截断 200 字符），带 `<br/>` 前缀。
pub(crate) fn read_error_detail(log_path: &std::path::Path) -> String {
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

/// 发射 boot:update 事件（前端版本行芯片消费）。
pub(crate) fn emit_update(app: &tauri::AppHandle, status: &crate::updates::UpdateStatus) {
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

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for TeeWriter {
    type Writer = TeeWriterGuard;
    fn make_writer(&'a self) -> Self::Writer {
        TeeWriterGuard {
            file: self.file.clone(),
        }
    }
}

#[cfg(unix)]
extern "C" fn signal_exit_handler(_: i32) {
    SIGNAL_EXIT.store(true, std::sync::atomic::Ordering::SeqCst);
}

pub(crate) static SIGNAL_EXIT: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
