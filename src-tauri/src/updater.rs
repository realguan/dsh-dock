//! updater.rs —— 桌面客户端自更新（tauri-plugin-updater 的壳内封装）。
//!
//! 为什么经壳封装而不是让前端直接调插件命令：remote 页面（dsh Web UI）能拿到
//! `window.__TAURI__`，若把 `plugin:updater|*` 暴露给它等于把更新/安装权限交给了
//! 第三方内容（AGENTS 最小面例外册）。因此：更新动作全在 Rust 侧，
//! 前端只消费自有 IPC（`client_update_check` / `client_update_apply`）与
//! `app:update` 事件——事件与 IPC 都走 capability 显式授权。
//!
//! 更新源：GitHub Releases 的 `latest.json`（tauri.conf plugins.updater.endpoints）。
//! 产物签名：minisign（TAURI_SIGNING_PRIVATE_KEY），构建期签，运行期验。
//!
//! 平台差异（2026-08-25 按插件源码核实）：
//! - Windows：插件安装成功后 `process::exit(0)`——会跳过 `RunEvent::Exit` 的
//!   dsh 清理（孤儿风险）。因此本模块在下载完成后、安装前**显式停掉 dsh**。
//! - macOS/Linux：安装完成后需 `app.restart()`（同样先停 dsh，同生命周期）。

use std::sync::Arc;

use tauri::Manager;
use tauri_plugin_updater::UpdaterExt;

use crate::ShellState;

/// 自动更新状态机（前端只读；Rust 侧唯一写者）。
/// 状态推进：idle → checking → available(latest/) | upToDate(latest/) | failed(msg)
///          → downloading(progress) → installing → relaunching → done(version)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum ClientUpdate {
    Idle,
    Checking,
    Available {
        #[serde(skip_serializing_if = "Option::is_none")]
        latest: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        notes: Option<String>,
    },
    UpToDate {
        latest: Option<String>,
    },
    Downloading {
        #[serde(skip_serializing_if = "Option::is_none")]
        current: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        total: Option<u64>,
    },
    Installing,
    Relaunching,
    Done {
        version: String,
    },
    Failed {
        message: String,
    },
}

impl Default for ClientUpdate {
    fn default() -> Self {
        ClientUpdate::Idle
    }
}

/// 读取当前状态（IPC `get_client_update` / 前端初始渲染）。
pub fn current(state: &Arc<ShellState>) -> ClientUpdate {
    state
        .client_update
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_default()
}

/// 「检查更新」动作（IPC `client_update_check` 入口）。
/// 后台执行：完成时经 `app:update` 回推 Available/UpToDate/Failed。
pub fn run_check(app: tauri::AppHandle, state: Arc<ShellState>) {
    std::thread::spawn(move || {
        set_state(&state, &app, ClientUpdate::Checking);
        match blocked_check(&app) {
            Ok(Some(update)) => {
                let latest = Some(update.version.clone());
                let notes = update.body.clone();
                set_state(&state, &app, ClientUpdate::Available { latest, notes });
            }
            Ok(None) => set_state(&state, &app, ClientUpdate::UpToDate { latest: None }),
            Err(e) => set_state(
                &state,
                &app,
                ClientUpdate::Failed {
                    message: e.to_string(),
                },
            ),
        }
    });
}

/// 「确认更新」动作（IPC `client_update_apply` 入口）：
/// 下载 → 安装 → 重启。
/// - Windows：下载完成后先停 dsh，再交给安装器（插件随后 exit(0)）。
/// - macOS/Linux：安装完成后经 `app.restart()` 进入新版本。
pub fn run_download_and_install(app: tauri::AppHandle, state: Arc<ShellState>) {
    std::thread::spawn(move || {
        let update = match blocked_check(&app) {
            Ok(Some(u)) => u,
            Ok(None) => {
                set_state(&state, &app, ClientUpdate::UpToDate { latest: None });
                return;
            }
            Err(e) => {
                set_state(
                    &state,
                    &app,
                    ClientUpdate::Failed {
                        message: e.to_string(),
                    },
                );
                return;
            }
        };
        set_state(&state, &app, ClientUpdate::Installing);
        let mut bytes_done: u64 = 0;
        let mut content_length: Option<u64> = None;
        let mut last_emit = std::time::Instant::now();
        let mut progress = |chunk: usize, total: Option<u64>| {
            bytes_done += chunk as u64;
            if let Some(t) = total {
                content_length = Some(t);
            }
            let now = std::time::Instant::now();
            if now.duration_since(last_emit) >= std::time::Duration::from_millis(100) {
                last_emit = now;
                set_state(
                    &state,
                    &app,
                    ClientUpdate::Downloading {
                        current: Some(bytes_done),
                        total: content_length,
                    },
                );
            }
        };

        #[cfg(target_os = "windows")]
        let result = {
            // 下载与安装分离：插件 install 会 exit(0)（跳过 RunEvent::Exit 清理），
            // 所以 dsh 必须在 install 前显式停掉（壳与 dsh 同生命周期）。
            let bytes = match tauri::async_runtime::block_on(update.download(&mut progress, || {}))
            {
                Ok(b) => b,
                Err(e) => {
                    set_state(
                        &state,
                        &app,
                        ClientUpdate::Failed {
                            message: e.to_string(),
                        },
                    );
                    return;
                }
            };
            if let Some(mut dsh) = state.dsh.lock().unwrap().take() {
                let _ = crate::shell::stop_dsh(&mut dsh.child, std::time::Duration::from_secs(3));
            }
            update.install(&bytes)
        };

        #[cfg(not(target_os = "windows"))]
        let result = {
            tauri::async_runtime::block_on(update.download_and_install(&mut progress, || {}))
        };

        match result {
            Ok(()) => {
                // Windows：插件内部已在启动安装器后 exit(0)，不会走到这里。
                #[cfg(not(target_os = "windows"))]
                {
                    set_state(
                        &state,
                        &app,
                        ClientUpdate::Done {
                            version: update.version.clone(),
                        },
                    );
                    set_state(&state, &app, ClientUpdate::Relaunching);
                    // 重启前先停掉 dsh（壳退 = dsh 停，同生命周期）。
                    if let Some(mut dsh) = state.dsh.lock().unwrap().take() {
                        let _ = crate::shell::stop_dsh(
                            &mut dsh.child,
                            std::time::Duration::from_secs(3),
                        );
                    }
                    app.restart();
                }
                #[cfg(target_os = "windows")]
                {
                    // 正常情况不会到这里（install 已 exit(0)）；万一回来了说明
                    // 安装器启动异常，按失败报。
                    set_state(
                        &state,
                        &app,
                        ClientUpdate::Failed {
                            message: "安装器未接管（已退出前回退）".into(),
                        },
                    );
                }
            }
            Err(e) => {
                set_state(
                    &state,
                    &app,
                    ClientUpdate::Failed {
                        message: e.to_string(),
                    },
                );
            }
        }
    });
}

/// 同步执行一次版本检查（updater 的 check 是 async；本壳在后台线程跑，
/// block_on 即可；不进入 Tauri 事件循环）。
fn blocked_check(
    app: &tauri::AppHandle,
) -> tauri_plugin_updater::Result<Option<tauri_plugin_updater::Update>> {
    let updater = app.updater()?;
    tauri::async_runtime::block_on(updater.check())
}

/// 写入状态并广播 `app:update`（前端只读；事件负载 = ClientUpdate）。
/// 只发给壳自带窗口（main / about）——dsh Web UI 是 remote origin，不消费
/// 本事件（最小面纪律：壳事件不流经第三方内容）。
fn set_state(state: &Arc<ShellState>, app: &tauri::AppHandle, value: ClientUpdate) {
    use tauri::Emitter;
    *state.client_update.lock().unwrap() = Some(value.clone());
    for label in ["main", "about"] {
        if let Some(win) = app.get_webview_window(label) {
            let _ = win.emit("app:update", value.clone());
        }
    }
}
