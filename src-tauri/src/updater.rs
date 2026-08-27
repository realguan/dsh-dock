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

/// GitHub Releases 下载加速镜像（中国大陆直连 github.com 受阻时回退）。
/// 镜像仅做传输代理；更新产物仍经 minisign 验签，镜像无法篡改安装包。
/// 检查清单的镜像端点在 tauri.conf.json `plugins.updater.endpoints`（直连优先、镜像兜底）；
/// 二进制下载 URL 由 latest.json 内联给出（绝对 github.com URL），需在下载失败时改写。
const GITHUB_MIRROR_PREFIX: &str = "https://gh-proxy.com/";

/// `app:update` 事件的合法目标窗口（roadmap 4.2 测试锚定此契约）：
/// 壳自带窗口 only——dsh Web UI 是 remote origin，壳事件不流经第三方内容
/// （宪法 §7 最小面纪律）。与 capabilities/default.json 的 windows 列表
/// 交叉验证见 tests::update_event_targets_are_capability_windows。
const UPDATE_EVENT_TARGETS: [&str; 2] = ["main", "about"];

/// 自动更新状态机（前端只读；Rust 侧唯一写者）。
/// 状态推进：idle → checking → available(latest/) | upToDate(latest/) | failed(msg)
///          → downloading(progress) → installing → relaunching → done(version)
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum ClientUpdate {
    #[default]
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

/// 读取当前状态（IPC `get_client_update` / 前端初始渲染）。
pub fn current(state: &Arc<ShellState>) -> ClientUpdate {
    state
        .client_update
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_default()
}

/// 把 github.com 的 Release 下载 URL 改写为经加速镜像的 URL（直连失败时回退）。
/// 仅改写本仓库 Release 资产 URL；其他 URL 返回 None（不动）。
fn mirror_download_url(url: &tauri::Url) -> Option<tauri::Url> {
    if url.host_str()? != "github.com" {
        return None;
    }
    if !url.path().starts_with("/realguan/dsh-dock/releases/") {
        return None;
    }
    tauri::Url::parse(&format!("{GITHUB_MIRROR_PREFIX}{}", url.as_str())).ok()
}

/// 把 updater 错误映射为用户可读文案：裸 reqwest 错误（如 "error sending request
/// for url"）对用户无可行动性；网络层失败统一提示检查网络/代理，HTTP 错误保留原文。
fn friendly_error(e: &tauri_plugin_updater::Error) -> String {
    if let tauri_plugin_updater::Error::Reqwest(re) = e {
        // status() = None 表示非 HTTP 响应错误（连接被拒 / 超时 / TLS / DNS），
        // 即 GitHub 直连受阻的典型形态。
        if re.status().is_none() {
            return "无法连接更新服务器（GitHub 直连失败），请检查网络或代理后重试。".to_string();
        }
    }
    format!("更新失败：{e}")
}

/// 「检查更新」动作（IPC `client_update_check` 入口）。
/// 后台执行：完成时经 `app:update` 回推 Available/UpToDate/Failed。
pub fn run_check(app: tauri::AppHandle, state: Arc<ShellState>) {
    std::thread::spawn(move || {
        tracing::info!("客户端更新检查：IPC 触发");
        set_state(&state, &app, ClientUpdate::Checking);
        match blocked_check(&app) {
            Ok(Some(update)) => {
                let latest = Some(update.version.clone());
                let notes = update.body.clone();
                set_state(&state, &app, ClientUpdate::Available { latest, notes });
            }
            Ok(None) => set_state(&state, &app, ClientUpdate::UpToDate { latest: None }),
            Err(e) => {
                tracing::warn!("客户端更新检查失败：{e}");
                set_state(
                    &state,
                    &app,
                    ClientUpdate::Failed {
                        message: friendly_error(&e),
                    },
                )
            }
        }
    });
}

/// 「确认更新」动作（IPC `client_update_apply` 入口）：
/// 下载 → 安装 → 重启。
/// - Windows：下载完成后先停 dsh，再交给安装器（插件随后 exit(0)）。
/// - macOS/Linux：安装完成后经 `app.restart()` 进入新版本。
pub fn run_download_and_install(app: tauri::AppHandle, state: Arc<ShellState>) {
    std::thread::spawn(move || {
        let mut update = match blocked_check(&app) {
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
                        message: friendly_error(&e),
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
            // 所以 dsh 必须在 install 前显式停掉（壳与 dsh 同生命周期，会话式 teardown）。
            let download_result =
                tauri::async_runtime::block_on(update.download(&mut progress, || {}));
            let bytes = match download_result {
                Ok(b) => b,
                Err(first_err) => {
                    let msg = friendly_error(&first_err);
                    let Some(mirrored) = mirror_download_url(&update.download_url) else {
                        set_state(&state, &app, ClientUpdate::Failed { message: msg });
                        return;
                    };
                    tracing::warn!("客户端更新直连下载失败（{msg}），经镜像重试");
                    update.download_url = mirrored;
                    match tauri::async_runtime::block_on(update.download(&mut progress, || {})) {
                        Ok(b) => b,
                        Err(e) => {
                            tracing::error!("镜像下载也失败：{e}");
                            set_state(&state, &app, ClientUpdate::Failed { message: msg });
                            return;
                        }
                    }
                }
            };
            if let Some(mut ex) = state.session.lock().unwrap().take() {
                let _ = ex.teardown();
            }
            update.install(&bytes)
        };

        #[cfg(not(target_os = "windows"))]
        let result = {
            // 直连失败 → 改写 URL 经镜像重试一次（download_and_install 失败时尚未
            // 安装，重试安全）。
            match tauri::async_runtime::block_on(update.download_and_install(&mut progress, || {}))
            {
                Ok(()) => Ok(()),
                Err(first_err) => {
                    let msg = friendly_error(&first_err);
                    match mirror_download_url(&update.download_url) {
                        Some(mirrored) => {
                            tracing::warn!("客户端更新直连下载失败（{msg}），经镜像重试");
                            update.download_url = mirrored;
                            tauri::async_runtime::block_on(
                                update.download_and_install(&mut progress, || {}),
                            )
                        }
                        None => Err(first_err),
                    }
                }
            }
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
                    // 重启前先停掉 dsh 会话（壳退 = dsh 停，同生命周期）。
                    if let Some(mut ex) = state.session.lock().unwrap().take() {
                        let _ = ex.teardown();
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
                        message: friendly_error(&e),
                    },
                );
            }
        }
    });
}

/// 同步执行一次版本检查（updater 的 check 是 async；本壳在后台线程跑，
/// block_on 即可；不进入 Tauri 事件循环）。
///
/// connect_timeout（2026-08-26，issue #3）：端点数组首项为 GitHub 直连、次项为
/// 镜像。受限网络下直连可能黑洞（TCP 握手无响应），10s 连接超时让插件快速跳过
/// 直连、尝试镜像。该配置经 UpdaterBuilder 传入，对 check 与后续 download 均生效
/// （仅连接阶段，不影响大包传输总时长）。
fn blocked_check(
    app: &tauri::AppHandle,
) -> tauri_plugin_updater::Result<Option<tauri_plugin_updater::Update>> {
    let updater = app
        .updater_builder()
        .configure_client(|builder| builder.connect_timeout(std::time::Duration::from_secs(10)))
        .build()?;
    tauri::async_runtime::block_on(updater.check())
}

/// 写入状态并广播 `app:update`（前端只读；事件负载 = ClientUpdate）。
/// 只发给壳自带窗口（main / about）——dsh Web UI 是 remote origin，不消费
/// 本事件（最小面纪律：壳事件不流经第三方内容）。
fn set_state(state: &Arc<ShellState>, app: &tauri::AppHandle, value: ClientUpdate) {
    use tauri::Emitter;
    tracing::info!("客户端更新状态 → {:?}", value);
    *state.client_update.lock().unwrap() = Some(value.clone());
    for label in UPDATE_EVENT_TARGETS {
        if let Some(win) = app.get_webview_window(label) {
            let _ = win.emit("app:update", value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirror_download_url_rewrites_own_release_assets() {
        let u = tauri::Url::parse(
            "https://github.com/realguan/dsh-dock/releases/download/v0.4.6/DSH.Dock_0.4.6_x64-setup.exe",
        )
        .unwrap();
        assert_eq!(
            mirror_download_url(&u).unwrap().as_str(),
            "https://gh-proxy.com/https://github.com/realguan/dsh-dock/releases/download/v0.4.6/DSH.Dock_0.4.6_x64-setup.exe"
        );
    }

    #[test]
    fn mirror_download_url_leaves_non_github_and_other_repos_untouched() {
        // 非 github.com → None
        let other_host = tauri::Url::parse("https://example.com/file.exe").unwrap();
        assert!(mirror_download_url(&other_host).is_none());
        // github.com 但非本仓库 release → None（不代理任意 GitHub 流量）
        let other_repo =
            tauri::Url::parse("https://github.com/someone/else/releases/download/v1/f").unwrap();
        assert!(mirror_download_url(&other_repo).is_none());
        // github.com 本仓库但非 release 路径 → None
        let repo_root = tauri::Url::parse("https://github.com/realguan/dsh-dock").unwrap();
        assert!(mirror_download_url(&repo_root).is_none());
    }

    // ---- roadmap 4.2：ClientUpdate 状态机与事件目标窗口 ----

    /// 全变体 serde 往返 + `phase` 词形锚定（tag="phase" + camelCase 是
    /// 前后端契约：frontend/src/lib/events.ts 的 KNOWN_PHASES 与 lib.rs 事件
    /// 均以此匹配，词形漂移 = 静默断链，必须在此钉死）。
    #[test]
    fn client_update_serde_round_trip_all_variants() {
        let cases: Vec<(ClientUpdate, &str)> = vec![
            (ClientUpdate::Idle, "idle"),
            (ClientUpdate::Checking, "checking"),
            (
                ClientUpdate::Available {
                    latest: Some("0.6.0".into()),
                    notes: Some("release notes".into()),
                },
                "available",
            ),
            (ClientUpdate::UpToDate { latest: None }, "upToDate"),
            (
                ClientUpdate::Downloading {
                    current: Some(1024),
                    total: None,
                },
                "downloading",
            ),
            (ClientUpdate::Installing, "installing"),
            (ClientUpdate::Relaunching, "relaunching"),
            (
                ClientUpdate::Done {
                    version: "0.6.0".into(),
                },
                "done",
            ),
            (
                ClientUpdate::Failed {
                    message: "x".into(),
                },
                "failed",
            ),
        ];
        for (value, phase) in cases {
            let json = serde_json::to_value(&value).unwrap();
            assert_eq!(
                json.get("phase").and_then(|p| p.as_str()),
                Some(phase),
                "phase 词形漂移：{json}"
            );
            assert_eq!(
                serde_json::from_value::<ClientUpdate>(json).unwrap(),
                value,
                "{phase} 往返失真"
            );
        }
    }

    /// `skip_serializing_if` 契约：None 字段不出现——payload 保持最小形态，
    /// 前端 normalize 的「缺字段补默认」依赖这一点。
    #[test]
    fn none_fields_are_omitted_from_payload() {
        let json = serde_json::to_value(ClientUpdate::Available {
            latest: None,
            notes: None,
        })
        .unwrap();
        let obj = json.as_object().unwrap();
        assert!(!obj.contains_key("latest"));
        assert!(!obj.contains_key("notes"));
        // 对照组：Downloading 未跳过字段带值时正常出现
        let json = serde_json::to_value(ClientUpdate::Downloading {
            current: Some(7),
            total: Some(100),
        })
        .unwrap();
        assert_eq!(json["current"], 7);
        assert_eq!(json["total"], 100);
    }

    /// 前向兼容：未知字段忽略（壳先升级新增字段时旧前端侧不受影响，
    /// 反向同理——serde 默认行为在此显式钉死，防止未来加 deny_unknown_fields）。
    #[test]
    fn deserialize_tolerates_unknown_fields() {
        let v: ClientUpdate =
            serde_json::from_str(r#"{"phase":"done","version":"1.2.3","futureField":true}"#)
                .unwrap();
        assert_eq!(
            v,
            ClientUpdate::Done {
                version: "1.2.3".into()
            }
        );
    }

    /// Default 派生锚定第一变体 Idle：进程冷启动即「从未检查过」，前端据
    /// 此决定是否自动首查（About 页语义），默认态改变会静默破坏该链路。
    #[test]
    fn default_is_idle() {
        assert_eq!(ClientUpdate::default(), ClientUpdate::Idle);
    }

    /// 事件目标窗口 ↔ capability 双向契约：
    /// 发送列表必须是常量 ["main", "about"]（remote dsh 页永不收壳事件），
    /// 且两窗口都在 capabilities/default.json 的 windows 白名单里（漏登记 =
    /// 该窗口收不到事件，宪法 §7「三处同步」的事件面版本）。
    #[test]
    fn update_event_targets_are_capability_windows() {
        assert_eq!(UPDATE_EVENT_TARGETS, ["main", "about"]);
        let caps: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/default.json")).unwrap();
        let windows = caps["windows"].as_array().expect("windows 列表");
        for label in UPDATE_EVENT_TARGETS {
            assert!(
                windows.iter().any(|w| w.as_str() == Some(label)),
                "窗口 {label} 未在 capabilities windows 登记"
            );
        }
    }

    /// `app:update` 事件名与前端 EV 常量的跨语言契约：events.ts 里必须存在
    /// 同名字符串（词形漂移 = 监听落空，v0.5.0 断链事故的防回归位）。
    #[test]
    fn app_update_event_name_matches_frontend_constant() {
        // 字面量在 types/events.ts 的 EV 常量表（lib/events.ts 只 import 消费）
        const FRONTEND_EVENTS_TS: &str = include_str!("../../frontend/src/types/events.ts");
        for ev in [
            "app:update",
            "boot:step",
            "boot:update",
            "boot:error",
            "boot:progress",
        ] {
            assert!(
                FRONTEND_EVENTS_TS.contains(&format!("\"{ev}\"")),
                "前端事件常量缺失：{ev}"
            );
        }
    }
}
