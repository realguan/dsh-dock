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

use crate::boot::ShellState;

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

/// `ClientUpdate` 的**相位**（判别式，丢掉负载）。
///
/// v1.2.0 D6（2026-09-11）：状态推进此前**没有任何机器可判的秩序**——
/// `run_download_and_install` 先发 `Installing` 再发 `Downloading`（`:156` 在下载调用
/// 之前），与本文档头（`:37-38`）写的 `… → downloading → installing →` **正好相反**。
/// 用户看到「安装中」先闪、「下载中」才到。相位枚举 + [`phase_rank`] 把"顺序"
/// 变成可判据、可测的东西（源码顺序闸门见 `tests::apply_emits_download_before_install`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UpdatePhase {
    Idle,
    Checking,
    Available,
    UpToDate,
    Downloading,
    Installing,
    Done,
    Relaunching,
    Failed,
}

impl ClientUpdate {
    /// 取相位（纯函数，供顺序闸门与 `set_state` 的漂移告警用）。
    pub(crate) fn phase(&self) -> UpdatePhase {
        match self {
            ClientUpdate::Idle => UpdatePhase::Idle,
            ClientUpdate::Checking => UpdatePhase::Checking,
            ClientUpdate::Available { .. } => UpdatePhase::Available,
            ClientUpdate::UpToDate { .. } => UpdatePhase::UpToDate,
            ClientUpdate::Downloading { .. } => UpdatePhase::Downloading,
            ClientUpdate::Installing => UpdatePhase::Installing,
            ClientUpdate::Done { .. } => UpdatePhase::Done,
            ClientUpdate::Relaunching => UpdatePhase::Relaunching,
            ClientUpdate::Failed { .. } => UpdatePhase::Failed,
        }
    }
}

/// 相位序（**唯一权威顺序**），与模块文档头 `:37-38` 一致。
///
/// `Done` 在 `Relaunching` **之前**——这是既有实际发射序（`Done{version}` →
/// `Relaunching` → `app.restart()`，见 `:239-251`），前端 `TRANSITIONS` 亦按此
/// （`done → relaunching` 合法）。**不要"顺手"对调**：那会改变用户可见的最后一帧
/// 语义并与前端表失配。
fn phase_rank(p: UpdatePhase) -> u8 {
    match p {
        UpdatePhase::Idle => 0,
        UpdatePhase::Checking => 1,
        // 检查的两个终局同档（互斥，无先后）
        UpdatePhase::Available | UpdatePhase::UpToDate => 2,
        UpdatePhase::Downloading => 3,
        UpdatePhase::Installing => 4,
        UpdatePhase::Done => 5,
        UpdatePhase::Relaunching => 6,
        // Failed 是跨相位终态，不参与排序
        UpdatePhase::Failed => u8::MAX,
    }
}

/// 相位推进是否合法（**纯函数**）。
///
/// 三条规则：
/// 1. **同相位 = 幂等允许**：进度事件天然是**重复**的（100ms 节流），把重复判为非法
///    会让进度冻结（D6 的用户可见症状之一）。
/// 2. **`Failed` 可从任意相位进入**：终态，不参与序。
/// 3. **允许回到 `Checking`**：用户再点检查 / 「应用更新」在下载前必须重新解析出
///    `Update` 句柄（见 `run_download_and_install` 的说明），故
///    `available|upToDate|done|failed → checking` 是正当回流，不是倒流。
///
/// 其余一律要求**单调前进**（rank 递增）；倒退即非法——`installing → downloading`
/// 正是本次要消灭的那条倒退。
///
/// **另有一条"跳跃"限制**：`Downloading`/`Installing` 只能由**已有更新可装**的相位
/// 进入（`checking`/`available`/`downloading`/`installing`）。只按 rank 递增的话
/// `idle → downloading` 会被放行——那意味着"没查过就开始下载"，语义上不可能，
/// 且会让前端的 loading 语义失去前置状态（自测发现：`reverse_transition_is_rejected`
/// 首跑正是在这条上红）。
pub(crate) fn transition_allowed(from: UpdatePhase, to: UpdatePhase) -> bool {
    use UpdatePhase::*;
    if to == Failed {
        return true;
    }
    if from == to {
        return true; // 幂等重复（进度）
    }
    if to == Checking && matches!(from, Available | UpToDate | Done | Failed) {
        return true; // 重新解析/重新检查
    }
    if matches!(to, Downloading | Installing)
        && !matches!(from, Checking | Available | Downloading | Installing)
    {
        return false; // 未解析出更新就不能开始下载/安装
    }
    phase_rank(to) > phase_rank(from)
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
        // spawn-gate: exempt(reqwest::Error::status() 是 HTTP 状态码读取，不拉起子进程)
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
        // **进入动作即发可见态**（v1.2.0 D6，2026-09-11）：`blocked_check` 要连
        // 10s 连接超时 + 包体解析，此前这段时间**零状态变更** ⇒ 用户点了「下载」
        // 完全没反应（截图症状）。先把相位推到 `Checking`（前端即有 loading），
        // 再去做解析；解析完成从 `checking` 前进到 `downloading`/`installing`
        // 均满足单调序（`checking < downloading < installing`）。
        //
        // 语义正确性：`ClientUpdate::Checking` 在这条路径上表示「正在准备更新」
        // （重新解析 Release 句柄），与检查路径同义——前端已有该相位的 loading 文案，
        // **不新增相位**（避免契约形状变更）。
        set_state(&state, &app, ClientUpdate::Checking);
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
        // 首个可见的下载态：在真正下载**之前**发（进度回调要到第一个 100ms 节流点
        // 才发，若等到那时才首次出现 `downloading`，中间仍有一段空白）。
        set_state(
            &state,
            &app,
            ClientUpdate::Downloading {
                current: Some(0),
                total: None,
            },
        );
        let mut bytes_done: u64 = 0;
        let mut content_length: Option<u64> = None;
        let mut last_emit = std::time::Instant::now();
        // **`Installing` 必须在下载之后发**（本次修的核心）：此前它在下载调用之前
        // （原 `:156`），于是发出序是 `Installing → Downloading…`——与文档头 `:37-38`
        // 的状态机相反，也与前端 `TRANSITIONS`（`installing → downloading` 曾非法）相悖。
        // 现改为下载完成后、进入安装阶段前发。
        let mut progress = |chunk: usize, total: Option<u64>| {
            bytes_done += chunk as u64;
            if let Some(t) = total {
                content_length = Some(t);
            }
            let now = std::time::Instant::now();
            if now.duration_since(last_emit) >= std::time::Duration::from_millis(100) {
                last_emit = now;
                // 相位仍为 `downloading` ⇒ 重复事件幂等允许（见 `transition_allowed`）。
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
            // 下载完成 → 进入安装（此刻才发 `Installing`）。
            set_state(&state, &app, ClientUpdate::Installing);
            if let Some(mut ex) = state.session.lock().unwrap().take() {
                let _ = ex.teardown();
            }
            update.install(&bytes)
        };

        #[cfg(not(target_os = "windows"))]
        let result = {
            // 直连失败 → 改写 URL 经镜像重试一次（download_and_install 失败时尚未
            // 安装，重试安全）。
            //
            // 非 Windows 走 `download_and_install` 一体化 API，**没有**「下载完成」
            // 这个可拦截的时刻 ⇒ 不能在中间插 `Installing`（那会造成
            // `installing` 与 `downloading` 交错的双相位）。故这里**不预发**
            // `Installing`：真正的安装态由下面的 `Done → Relaunching` 序列承接。
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
    // **相位倒退告警**（v1.2.0 D6，2026-09-11）：广播前的顺序自检。
    // 不阻断（生产路径不得因日志而失败），但把"倒退"从静默变成可观测——
    // 这正是本次缺陷（先 Installing 后 Downloading）此前无人发现的原因。
    let prev = state.client_update.lock().ok().and_then(|g| g.clone());
    if let Some(prev) = prev {
        let (from, to) = (prev.phase(), value.phase());
        if !transition_allowed(from, to) {
            tracing::warn!(
                "客户端更新相位倒退：{from:?} → {to:?}（前端 TRANSITIONS 可能丢弃该事件）"
            );
        }
    }
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

    /// **复现锚（v1.2.0 D6，2026-09-11）**：`run_download_and_install` 里
    /// **首个 `Installing` 必须晚于首个 `Downloading`**。
    ///
    /// 缺陷：原实现在下载调用**之前**发 `Installing`（`:156`）⇒ 发出序
    /// `Installing → Downloading…`，与文档头 `:37-38` 的状态机相反，也与前端
    /// `TRANSITIONS`（`installing → downloading` 曾非法）相悖 ⇒ 进度事件被丢弃。
    ///
    /// **为什么用源码文本闸门**：本函数收 `&AppHandle`、内部跑真实网络下载，
    /// 单测环境既起不出 Tauri 运行时也无法伪造下载。而缺陷本质是**发出的先后顺序**
    /// ——判据就是"这两个 `set_state` 谁在前"。序列语义另由
    /// `emitted_sequence_advances_monotonically` 覆盖（纯逻辑，与 qa-verify 的
    /// 序列级判据同构）。
    #[test]
    fn apply_emits_download_before_install() {
        // CRLF 归一（v1.1.1 教训）。
        let src = include_str!("updater.rs").replace("\r\n", "\n");
        let start = src
            .find("pub fn run_download_and_install(")
            .expect("run_download_and_install 应存在");
        let rest = &src[start..];
        let end = rest
            .find("\n/// 同步执行一次版本检查")
            .unwrap_or(rest.len());
        let body = &rest[..end];
        let first_downloading = body
            .find("ClientUpdate::Downloading")
            .expect("函数内应有 Downloading 状态");
        let first_installing = body
            .find("ClientUpdate::Installing")
            .expect("函数内应有 Installing 状态");
        assert!(
            first_downloading < first_installing,
            "首个 Downloading 必须早于首个 Installing（否则发出序倒退：先『安装中』\
             后『下载中』，前端会丢弃进度事件、进度条冻结）。\n\
             downloading@{first_downloading} installing@{first_installing}"
        );
    }

    /// **进入动作即发可见态**：函数最早的一条 `set_state` 必须在 `blocked_check`
    /// 之前——否则 10s 连接超时期间零状态变更 = 用户点了「下载」毫无反应（截图症状）。
    #[test]
    fn apply_emits_visible_state_before_blocked_check() {
        let src = include_str!("updater.rs").replace("\r\n", "\n");
        let start = src.find("pub fn run_download_and_install(").expect("存在");
        let rest = &src[start..];
        let body = &rest[..rest
            .find("\n/// 同步执行一次版本检查")
            .unwrap_or(rest.len())];
        let first_set_state = body.find("set_state(").expect("应至少发一次状态");
        let check = body.find("blocked_check(").expect("应调用 blocked_check");
        assert!(
            first_set_state < check,
            "首条 set_state 必须早于 blocked_check（否则解析期间 UI 无任何反馈）"
        );
    }

    /// **相位序纯逻辑**（与 qa-verify 的序列级判据同构）：把「事件序列 → 末态」
    /// 做成可断言的东西。覆盖 D6 要求的全序
    /// `available → checking(起始可见态) → downloading×N → installing → done`。
    #[test]
    fn emitted_sequence_advances_monotonically() {
        // 完整序列（含多个重复的进度事件）
        let seq = [
            ClientUpdate::Available {
                latest: Some("1.3.0".into()),
                notes: None,
            },
            ClientUpdate::Checking, // 进入动作即发的可见态
            ClientUpdate::Downloading {
                current: Some(0),
                total: None,
            },
            ClientUpdate::Downloading {
                current: Some(10),
                total: Some(100),
            },
            ClientUpdate::Downloading {
                current: Some(50),
                total: Some(100),
            },
            ClientUpdate::Downloading {
                current: Some(90),
                total: Some(100),
            },
            ClientUpdate::Installing,
            ClientUpdate::Done {
                version: "1.3.0".into(),
            },
            ClientUpdate::Relaunching,
        ];
        // 逐对断言合法（这就是"序"的机器判据）
        for w in seq.windows(2) {
            assert!(
                transition_allowed(w[0].phase(), w[1].phase()),
                "非法相位迁移：{:?} → {:?}",
                w[0].phase(),
                w[1].phase()
            );
        }
        // 重复事件幂等允许（进度不得被丢）——三条 downloading 全部合法
        let dl = seq
            .iter()
            .filter(|s| s.phase() == UpdatePhase::Downloading)
            .count();
        assert_eq!(dl, 4, "四条进度事件都应合法（含首条 0 值）");
        // 末态
        assert_eq!(seq.last().unwrap().phase(), UpdatePhase::Relaunching);
    }

    /// 倒退必须被拒（**复现缺陷序**）：`installing → downloading` 正是修前那条。
    #[test]
    fn reverse_transition_is_rejected() {
        assert!(
            !transition_allowed(UpdatePhase::Installing, UpdatePhase::Downloading),
            "安装后再回到下载是相位倒退——必须拒（这正是 D6 修掉的序）"
        );
        assert!(!transition_allowed(
            UpdatePhase::Done,
            UpdatePhase::Downloading
        ));
        assert!(!transition_allowed(
            UpdatePhase::Idle,
            UpdatePhase::Downloading
        ));
    }

    /// 允许集：重复 / 失败终态 / 重新检查（三条规则各自钉住）。
    #[test]
    fn allowed_special_transitions() {
        // 规则 1：同相位幂等（进度）
        assert!(transition_allowed(
            UpdatePhase::Downloading,
            UpdatePhase::Downloading
        ));
        // 规则 2：Failed 可从任意相位进入
        for from in [
            UpdatePhase::Idle,
            UpdatePhase::Checking,
            UpdatePhase::Available,
            UpdatePhase::Downloading,
            UpdatePhase::Installing,
            UpdatePhase::Done,
            UpdatePhase::Relaunching,
        ] {
            assert!(
                transition_allowed(from, UpdatePhase::Failed),
                "{from:?} → Failed"
            );
        }
        // 规则 3：重新检查（用户在 available/done/failed 后再点检查）
        for from in [
            UpdatePhase::Available,
            UpdatePhase::UpToDate,
            UpdatePhase::Done,
            UpdatePhase::Failed,
        ] {
            assert!(
                transition_allowed(from, UpdatePhase::Checking),
                "{from:?} → Checking"
            );
        }
        // 单调前进仍在
        assert!(transition_allowed(
            UpdatePhase::Checking,
            UpdatePhase::Available
        ));
        assert!(transition_allowed(
            UpdatePhase::Downloading,
            UpdatePhase::Installing
        ));
        assert!(transition_allowed(
            UpdatePhase::Installing,
            UpdatePhase::Done
        ));
        assert!(transition_allowed(
            UpdatePhase::Done,
            UpdatePhase::Relaunching
        ));
    }

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

    // ---------- 跨层闸门：Rust 发射序 ↔ 前端 TRANSITIONS（T-D6c，2026-09-11） ----------

    /// 前端状态机表所在文件。**与 `app_update_event_name_matches_frontend_constant`
    /// 同一范式**：从 Rust 侧读前端源码 ⇒ 任何一侧单方面改动都可被机器判定。
    const FRONTEND_UPDATE_STORE_TS: &str =
        include_str!("../../frontend/src/stores/clientUpdateStore.ts");

    /// 抹掉 Rust 行注释与块注释（**保留换行** ⇒ 位置/行号不变）。
    ///
    /// 必须有：函数体里有多处**注释**提到 `ClientUpdate::Checking` 之类的字样
    /// （如「语义正确性：`ClientUpdate::Checking`…」），不抹就会把它们当成发射点。
    fn strip_rust_comments(text: &str) -> String {
        let b = text.as_bytes();
        let mut out = text.as_bytes().to_vec();
        let mut i = 0usize;
        while i < b.len() {
            if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'/' {
                while i < b.len() && b[i] != b'\n' {
                    out[i] = b' ';
                    i += 1;
                }
            } else if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
                out[i] = b' ';
                out[i + 1] = b' ';
                i += 2;
                while i < b.len() && !(b[i] == b'*' && i + 1 < b.len() && b[i + 1] == b'/') {
                    if b[i] != b'\n' {
                        out[i] = b' ';
                    }
                    i += 1;
                }
                if i < b.len() {
                    out[i] = b' ';
                    out[i + 1] = b' ';
                    i += 2;
                }
            } else {
                i += 1;
            }
        }
        String::from_utf8(out).expect("抹注释只替换为空格，不破坏 UTF-8")
    }

    /// 花括号配对：返回 `open` 处 `{` 对应的 `}` 下标（越界则返回末尾）。
    fn brace_close(text: &str, open: usize) -> usize {
        let b = text.as_bytes();
        let mut d = 0i32;
        let mut i = open;
        while i < b.len() {
            match b[i] {
                b'{' => d += 1,
                b'}' => {
                    d -= 1;
                    if d == 0 {
                        return i;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        b.len()
    }

    /// 圆括号配对：返回 `open` 处 `(` 对应的 `)` 下标。
    fn paren_close(text: &str, open: usize) -> usize {
        let b = text.as_bytes();
        let mut d = 0i32;
        let mut i = open;
        while i < b.len() {
            match b[i] {
                b'(' => d += 1,
                b')' => {
                    d -= 1;
                    if d == 0 {
                        return i;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        b.len()
    }

    /// `run_download_and_install` 的函数体源码（已抹注释）。
    fn apply_fn_body() -> String {
        let src = include_str!("updater.rs").replace("\r\n", "\n");
        let start = src
            .find("pub fn run_download_and_install(")
            .expect("run_download_and_install 应存在");
        let rest = &src[start..];
        let end = rest.find("/// 同步执行一次版本检查").unwrap_or(rest.len());
        strip_rust_comments(&rest[..end])
    }

    /// `#[cfg(target_os = "windows")]` / `#[cfg(not(target_os = "windows"))]` 块范围：
    /// `(起点, 终点, 是否 windows 支)`。
    fn cfg_blocks(code: &str) -> Vec<(usize, usize, bool)> {
        let mut out = Vec::new();
        let mut from = 0usize;
        while let Some(rel) = code[from..].find("#[cfg(") {
            let at = from + rel;
            let close = match code[at..].find(")]") {
                Some(c) => at + c + 1,
                None => break,
            };
            let attr = &code[at..close];
            if attr.contains("target_os = \"windows\"") {
                if let Some(brace_rel) = code[close..].find('{') {
                    let open = close + brace_rel;
                    out.push((at, brace_close(code, open), !attr.contains("not(")));
                }
            }
            from = close + 1;
        }
        out
    }

    /// 按目标平台分支遮蔽源码：保留指定分支，另一分支整段抹为空格（**保留换行**）。
    fn branch_view(code: &str, keep_windows: bool) -> String {
        let mut b = code.as_bytes().to_vec();
        for (s, e, is_win) in cfg_blocks(code) {
            if is_win != keep_windows {
                for k in s..=e.min(b.len().saturating_sub(1)) {
                    if b[k] != b'\n' {
                        b[k] = b' ';
                    }
                }
            }
        }
        String::from_utf8(b).expect("遮蔽只替换为空格")
    }

    /// `=> {` 打开的**匹配分支块**：`(起点, 终点, 分支 id)`（分支 id 全局唯一）。
    ///
    /// 用途：识别「同一 `match` 的两个兄弟分支」——它们的发射**不是**前后相继关系
    /// （互斥），故不得结成相邻对（否则 `failed → failed` 这类假对会误报）。
    fn arm_blocks(code: &str) -> Vec<(usize, usize, u32)> {
        let b = code.as_bytes();
        let mut out = Vec::new();
        let mut pending = false;
        let mut next_id = 0u32;
        let mut i = 0usize;
        while i < b.len() {
            if b[i] == b'=' && i + 1 < b.len() && b[i + 1] == b'>' {
                pending = true;
                i += 2;
                continue;
            }
            if b[i] == b'{' {
                if pending {
                    let close = brace_close(code, i);
                    out.push((i, close, next_id));
                    next_id += 1;
                }
                pending = false;
            }
            i += 1;
        }
        out
    }

    /// 某位置的**分支栈**（外层→内层，仅含匹配分支块）。
    fn arm_key(arms: &[(usize, usize, u32)], pos: usize) -> Vec<u32> {
        let mut key: Vec<u32> = arms
            .iter()
            .filter(|(s, e, _)| *s <= pos && pos <= *e)
            .map(|(s, _, id)| (*s, *id))
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(_, id)| id)
            .collect();
        key.sort_unstable(); // 分支 id 递增 == 出现顺序；嵌套时外层 id 更小
        key
    }

    /// 两个分支栈是否为**同一 `match` 的兄弟分支**（互斥，非相继）。
    fn sibling_arms(a: &[u32], b: &[u32]) -> bool {
        !a.is_empty()
            && a.len() == b.len()
            && a[..a.len() - 1] == b[..b.len() - 1]
            && a[a.len() - 1] != b[b.len() - 1]
    }

    /// 一次 `set_state` 发射：`(变体名, 调用起点, 调用终点)`。
    fn set_state_emissions(code: &str) -> Vec<(String, usize, usize)> {
        let mut out = Vec::new();
        let mut from = 0usize;
        while let Some(rel) = code[from..].find("set_state") {
            let at = from + rel;
            let open = match code[at..].find('(') {
                Some(o) => at + o,
                None => break,
            };
            let close = paren_close(code, open);
            let args = &code[open..=close.min(code.len() - 1)];
            if let Some(v) = args.strip_prefix('(').and_then(|a| {
                a.find("ClientUpdate::").map(|k| {
                    let tail = &a[k + "ClientUpdate::".len()..];
                    tail.chars()
                        .take_while(|c| c.is_ascii_alphanumeric())
                        .collect::<String>()
                })
            }) {
                out.push((v, at, close));
            }
            from = close.max(at + 1);
        }
        out
    }

    /// **从一次分支视图提取「运行时相邻」的相位对**（前驱 → 后继）。
    ///
    /// 模型（三条，缺一会假阳/假阴）：
    /// 1. **`return;` 终结分支**：某次发射之后、下一次发射之前出现 `return;`
    ///    ⇒ 该发射是**终止性备选**（如 `Ok(None) => UpToDate`），它**不更新**
    ///    "落空路径"的当前状态，故不解锁后续对；但它与当前状态**结成一对**（备选也是迁移）。
    ///    这样 `Checking → UpToDate` 与**跨过 return 的** `Checking → Downloading`
    ///    都能被覆盖（后者正是 D6b 缺的那条边）。
    /// 2. **兄弟分支互斥**：同一 `match` 的两个分支之间的发射不构成相继（见 `sibling_arms`）。
    /// 3. 其余相邻即相继。
    fn emitted_pairs(code: &str) -> Vec<(String, String)> {
        let ems = set_state_emissions(code);
        let arms = arm_blocks(code);
        let mut pairs = Vec::new();
        let mut fallthrough: Option<(String, Vec<u32>)> = None;
        for (i, (v, _at, end)) in ems.iter().enumerate() {
            let next_at = ems.get(i + 1).map(|e| e.1).unwrap_or(code.len());
            let terminal = code[*end..next_at].contains("return;");
            let key = arm_key(&arms, at_pos(&ems, i));
            if let Some((pv, pk)) = &fallthrough {
                if !sibling_arms(pk, &key) {
                    pairs.push((pv.clone(), v.clone()));
                }
            }
            if !terminal {
                fallthrough = Some((v.clone(), key));
            }
        }
        pairs
    }

    /// 取第 `i` 次发射的起点（供 `arm_key` 用）。
    fn at_pos(ems: &[(String, usize, usize)], i: usize) -> usize {
        ems[i].1
    }

    /// 解析前端 `TRANSITIONS` 表 → `(from, [to…])`（顺序保留）。
    ///
    /// 抗 CRLF（v1.1.1 教训）；只认 `export const TRANSITIONS … = { … }` 顶层块；
    /// 每行形如 `  idle: ["checking"],`（行内 `//` 注释先剥）。
    fn frontend_transitions() -> Vec<(String, Vec<String>)> {
        let ts = FRONTEND_UPDATE_STORE_TS.replace("\r\n", "\n");
        let start = ts
            .find("export const TRANSITIONS")
            .expect("前端应导出 TRANSITIONS（表名变更需同步本闸门）");
        let open = start + ts[start..].find('{').expect("TRANSITIONS 应有对象字面量");
        let close = brace_close(&ts, open);
        let mut out = Vec::new();
        for line in ts[open + 1..close].lines() {
            let line = line.split("//").next().unwrap_or("").trim();
            let Some((left, right)) = line.split_once(':') else {
                continue;
            };
            let from = left.trim();
            if from.is_empty() || !from.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                continue;
            }
            let Some(list) = right.trim().strip_prefix('[') else {
                continue;
            };
            let Some(list) = list.split(']').next() else {
                continue;
            };
            let tos: Vec<String> = list
                .split(',')
                .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                .filter(|s| !s.is_empty())
                .collect();
            out.push((from.to_string(), tos));
        }
        out
    }

    /// Rust 侧全部相位：`(变体名, serde 相位名)`——相位名经 serde 取**生产真值**，
    /// 不手抄（否则改了 `rename_all` 闸门会失去意义）。
    fn rust_phases() -> Vec<(&'static str, String)> {
        [
            ("Idle", ClientUpdate::Idle),
            ("Checking", ClientUpdate::Checking),
            (
                "Available",
                ClientUpdate::Available {
                    latest: None,
                    notes: None,
                },
            ),
            ("UpToDate", ClientUpdate::UpToDate { latest: None }),
            (
                "Downloading",
                ClientUpdate::Downloading {
                    current: None,
                    total: None,
                },
            ),
            ("Installing", ClientUpdate::Installing),
            (
                "Done",
                ClientUpdate::Done {
                    version: "0.0.0".into(),
                },
            ),
            ("Relaunching", ClientUpdate::Relaunching),
            (
                "Failed",
                ClientUpdate::Failed {
                    message: String::new(),
                },
            ),
        ]
        .into_iter()
        .map(|(v, c)| {
            let name = serde_json::to_value(&c).unwrap()["phase"]
                .as_str()
                .expect("相位应可序列化为字符串")
                .to_string();
            (v, name)
        })
        .collect()
    }

    /// 变体名 → serde 相位名（提取器拿到的是**变体名**，前端表用的是**相位名**）。
    fn phase_name_of(variant: &str) -> String {
        rust_phases()
            .into_iter()
            .find(|(v, _)| *v == variant)
            .unwrap_or_else(|| panic!("未知相位变体 `{variant}`——本闸门的分支表需同步"))
            .1
    }

    /// Rust 侧全部相位名。
    fn rust_phase_names() -> Vec<String> {
        rust_phases().into_iter().map(|(_, n)| n).collect()
    }

    /// **跨层闸门（T-D6c，2026-09-11）**：`run_download_and_install` 的**每一个运行时
    /// 相邻发射对**都必须被前端 `TRANSITIONS` 表接受。
    ///
    /// 为什么需要它（qa-verify task-50 §6.1 的**实测反例**）：在真实函数里插入一条
    /// 前端表不含的迁移，**Rust 全部测试仍绿、前端 seam 测试也发现不了**（它不读 Rust
    /// 源码）⇒ 新增一条相位发射对，**两侧都不会红**。而 D6b 正是这个形态的复发
    /// （Rust 加了 `Checking`、前端没跟上）——同一类"两侧各自绿、跨界即破"本会话已两次。
    ///
    /// 双向性：Rust 改序（本函数体）**或**前端改表，都会让本断言红。
    /// **两侧须同步修改**；若确需新增边，先改前端表再改此处（或反之），并跑本测试。
    #[test]
    fn frontend_transitions_accept_every_emitted_pair_per_branch() {
        let fe = frontend_transitions();
        let accepts = |from: &str, to: &str| -> bool {
            fe.iter()
                .find(|(f, _)| f == from)
                .is_some_and(|(_, tos)| tos.iter().any(|t| t == to))
        };
        let body = apply_fn_body();
        let mut violations = Vec::new();
        for (label, keep_windows) in [("windows", true), ("非 Windows", false)] {
            let view = branch_view(&body, keep_windows);
            // 提取器给的是**变体名**，前端表用**相位名** ⇒ 经 serde 归一。
            let pairs: Vec<(String, String)> = emitted_pairs(&view)
                .into_iter()
                .map(|(a, b)| (phase_name_of(&a), phase_name_of(&b)))
                .collect();
            assert!(
                !pairs.is_empty(),
                "{label} 分支未提取到任何发射对——提取器失效（只绿不测最危险）"
            );
            for (from, to) in &pairs {
                if !accepts(from, to) {
                    violations.push(format!("[{label}] {from} → {to}"));
                }
            }
        }
        assert!(
            violations.is_empty(),
            "以下「Rust 真实发射序」的相邻对不被前端 TRANSITIONS 接受——\
             前端会丢弃这些事件（进度/状态卡住）：\n{}\n\
             ⇒ 两侧须同步：改前端 `clientUpdateStore.ts` 的 TRANSITIONS，或改 Rust 发射序。\
             （本闸门从 Rust 侧读前端源码，任一侧单方面改动都会红。）",
            violations.join("\n")
        );
    }

    /// **相位名集合一致**（防前端拼写错 → 事件被静默丢）。
    #[test]
    fn frontend_phase_names_match_rust() {
        let fe = frontend_transitions();
        let mut fe_names: Vec<String> = fe.iter().map(|(f, _)| f.clone()).collect();
        fe_names.sort();
        let mut rs_names = rust_phase_names();
        rs_names.sort();
        assert_eq!(
            fe_names, rs_names,
            "前端 TRANSITIONS 的键集合与 Rust 相位名不一致（拼写错 = 事件被静默丢弃）"
        );
        // 表**值**里的相位名也必须是合法相位（防 to 侧拼写错）
        for (from, tos) in &fe {
            for to in tos {
                assert!(
                    rs_names.contains(to),
                    "TRANSITIONS[{from}] 指向未知相位 `{to}`（拼写错 = 该边永不生效）"
                );
            }
        }
    }

    /// 提取器自检：**分支视图必须真的不同**（非 Windows 分支不预发 `Installing`）。
    ///
    /// 若遮蔽逻辑失效（两支一样），上面那条闸门就等于只测了一条路径——
    /// 「绿得没有信息量」（qa-verify §6.2 的同类教训）。
    #[test]
    fn cross_layer_extractor_separates_the_two_branches() {
        let body = apply_fn_body();
        let win = emitted_pairs(&branch_view(&body, true));
        let non = emitted_pairs(&branch_view(&body, false));
        assert_ne!(
            win, non,
            "两分支提取结果相同 ⇒ 遮蔽/提取失效（闸门只覆盖一条路径）"
        );
        // Windows 有 installing、非 Windows 无（这正是两分支的语义差异）
        let has = |pairs: &[(String, String)], to: &str| {
            pairs.iter().any(|(_, t)| phase_name_of(t) == to)
        };
        assert!(
            has(&win, "installing"),
            "Windows 分支应出现 installing 目标：{win:?}"
        );
        assert!(
            !has(&non, "installing"),
            "非 Windows 分支不应出现 installing（该分支走一体化 API）：{non:?}"
        );
    }
}
