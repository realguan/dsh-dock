//! commands/chat_embed.rs —— 内嵌 DeepSeek Chat 原生子视窗 IPC 命令
//!
//! 在 DSH Dock 主窗口内部直接挂载顶层原生 Webview（Tauri v2 add_child），
//! 绕过普通 iframe 遭受的 CSP `frame-ancestors 'none'` 拦截，
//! 实现与官方 Electron 桌面版完全等价的无缝窗口内嵌与会话互切。
//!
//! ## 2026-10-09 诊断批注
//! 上一轮出现「子视窗盖住 DSH 会话头部、顶部一大片空白」：实测 DeepSeek 自己的
//! 侧栏开关图标出现在窗口 y≈19，而 DSH 头部应在 y 0-76，说明子视窗实际被渲染在
//! (280, 0)。`embed_chat_open` 有日志，`embed_chat_resize` 没有，中间 5 分钟无
//! open 调用却有截图差异 ⇒ 怀疑某次 resize 把它挪上去。
//! 本批次：
//!   1. 去掉 `.auto_resize()` —— Tauri 的 auto_resize 把 bounds 存成相对窗口的
//!      **比例**（`y_rate = y / window_height`），窗口尺寸一变就按比例重算，
//!      与显式 `set_position/set_size` 互相打架；
//!   2. 所有命令回读实际 bounds 并打日志，下次一眼定位是谁挪错了。
//!
//! ## 2026-10-14 诊断批注（退化 rect 白卡）
//! 现场（官方 Electron 桌面版同源，Dock 侧同口径防线）：滚轮能把壳整页推上去，壳
//! 内容下方露出一张 538x124 的聊天白卡。根因在插件侧 —— 浮层的 `position: fixed`
//! 只写在可被 dispose 的 `<style>` 里，样式先走一步它就退化成 `<body>` 末尾的普通
//! 块，把壳的文档撑高（Electron 侧已修：行内不变量 + 应用期清扫 + 同寿清理）。
//! Dock 侧补的是第二条腿：**退化 rect 一律让位**（`rect_is_placeable`）。原生子视窗
//! 在图层面之上、不参与页面滚动、页面也盖不住，按「最小 100x100」把退化盒子摆出去，
//! 就是一张卡在窗口左上、谁也点不动的白卡 —— 宁可不显示，等真盒子到了再 `open`。

use serde::Serialize;
use tauri::{
    AppHandle, Emitter, EventTarget, LogicalPosition, LogicalSize, Manager, Webview,
    WebviewBuilder, WebviewUrl,
};

pub(crate) const CHAT_GUEST_LABEL: &str = "deepseek_chat_guest";
/// 主工作台 Webview 的 label（唯一的 `embed_chat_eval` 合法调用方，ADR-0033）。
const MAIN_WEBVIEW_LABEL: &str = "main";
const CHAT_URL: &str = "https://chat.deepseek.com/";
const NORMALIZED_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/152.0.0.0 Safari/537.36";
/// 客页 → 宿主的事件名（主 Webview 上的本地事件，不触网）。
const GUEST_MESSAGE_EVENT: &str = "dsce:guest-message";
const RESULT_TITLE_PREFIX: &str = "__DSCE_RESULT__:";
const ACTION_TITLE_PREFIX: &str = "__DSCE_EVENT__:";
const MAX_EVAL_SCRIPT_BYTES: usize = 16 * 1024;
/// 整条标题的字节上限：超过即视为普通页面标题，不当作桥消息。
const MAX_TITLE_MESSAGE_BYTES: usize = 8 * 1024;
/// 回传载荷（URI 编码后）的上限：标题本就短，超出的一律降级成 `null`。
const MAX_RESULT_PAYLOAD_BYTES: usize = 4 * 1024;

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct GuestTitleMessage {
    kind: &'static str,
    request_id: String,
    value: Option<String>,
}

fn valid_bridge_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn parse_guest_title(title: &str) -> Option<GuestTitleMessage> {
    if title.len() > MAX_TITLE_MESSAGE_BYTES {
        return None;
    }
    if let Some(message) = title.strip_prefix(RESULT_TITLE_PREFIX) {
        let (request_id, value) = message.split_once(':')?;
        if !valid_bridge_id(request_id) || value.len() > MAX_RESULT_PAYLOAD_BYTES {
            return None;
        }
        return Some(GuestTitleMessage {
            kind: "result",
            request_id: request_id.to_owned(),
            value: Some(value.to_owned()),
        });
    }
    if let Some(message) = title.strip_prefix(ACTION_TITLE_PREFIX) {
        let (action, request_id) = message.split_once(':')?;
        if action != "expand" || !valid_bridge_id(request_id) {
            return None;
        }
        return Some(GuestTitleMessage {
            kind: "expand",
            request_id: request_id.to_owned(),
            value: None,
        });
    }
    None
}

fn is_local_workbench_url(url: &tauri::Url) -> bool {
    url.scheme() == "http"
        && matches!(
            url.host_str(),
            Some("127.0.0.1") | Some("localhost") | Some("[::1]")
        )
}

/// 客页身份判定（纯函数，便于用恶意反例钉住）。
///
/// 只认 `https://chat.deepseek.com`：`chat.deepseek.com.evil.com` 之类后缀混淆、
/// 明文 http、以及任何子域都不算客页。
fn is_chat_guest_url(url: &tauri::Url) -> bool {
    url.scheme() == "https" && url.host_str() == Some("chat.deepseek.com")
}

fn is_chat_guest_webview(webview: &Webview) -> bool {
    webview.label() == CHAT_GUEST_LABEL && webview.url().is_ok_and(|url| is_chat_guest_url(&url))
}

fn eval_result_script(request_id: &str, script: &str) -> Result<String, String> {
    if !valid_bridge_id(request_id) {
        return Err("embed_chat_eval: 非法 request id".into());
    }
    if script.len() > MAX_EVAL_SCRIPT_BYTES {
        return Err("embed_chat_eval: 脚本超过大小上限".into());
    }
    let request_id = serde_json::to_string(request_id).map_err(|e| e.to_string())?;
    let script = serde_json::to_string(script).map_err(|e| e.to_string())?;
    let result_prefix = serde_json::to_string(RESULT_TITLE_PREFIX).expect("静态前缀可序列化");
    Ok(format!(
        r#"(()=>{{const id={request_id};const source={script};const previous=document.title;
const publish=value=>{{let encoded;try{{encoded=encodeURIComponent(JSON.stringify(value))}}catch(_error){{encoded="null"}}
if(encoded.length>{MAX_RESULT_PAYLOAD_BYTES})encoded="null";
const marker={result_prefix}+id+":"+encoded;document.title=marker;
setTimeout(()=>{{if(document.title===marker)document.title=previous}},80)}};
(async()=>{{try{{publish(await (0,eval)(source))}}catch(error){{publish({{error:String((error&&error.message)||error).slice(0,256)}})}}}})()}})()"#
    ))
}

const GUEST_SIGNAL_SCRIPT: &str = r#"(()=>{
  let serial=0;
  const signal=(action)=>{
    const previous=document.title;
    const marker='__DSCE_EVENT__:'+action+':'+Date.now()+'-'+(++serial);
    document.title=marker;
    setTimeout(()=>{if(document.title===marker)document.title=previous},80);
  };
  document.addEventListener('pointerdown',(event)=>{
    try{
      const target=event.target;
      const toggle=target instanceof Element?target.closest('[data-dsce-toggle]'):null;
      if(!toggle)return;
      const side=document.querySelector('[data-dsce-side]');
      if(!side||side.getBoundingClientRect().width<=120)signal('expand');
    }catch(_error){}
  },true);
})()"#;

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Debug, PartialEq)]
pub struct EmbedFrame {
    pub width: f64,
    pub height: f64,
    /// 页面的 devicePixelRatio。窗口尺寸是物理像素、页面尺寸是 CSS 像素，换算系数
    /// 由页面自己报：壳侧 `scale_factor()` 实测不稳定（同一台机器先后读到 1 和 2）。
    #[serde(default = "default_frame_scale")]
    pub scale: f64,
}

fn default_frame_scale() -> f64 {
    1.0
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Debug, PartialEq)]
pub struct EmbedRect {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

/// 页面视口原点相对子视窗坐标原点的偏移（逻辑像素）——**带平台门的入口**。
///
/// 只在 macOS 补，且只在调用方报了 `frame` 时补：Windows/Linux 的子视窗直接挂客户区，
/// 页面坐标即子视窗坐标（补了反而整体下移一个标题栏）；没报 frame 就没有可信差值，
/// 宁可不补也不猜。算术本体在 {@link inset_math}（无门纯函数）。
///
/// **平台门与算术分离**（2026-10-13 Linux CI 三红的修法）：旧版把两者焊死在一个
/// 函数里，测试只能跟着吃平台门——「窗口 − 页面 = 标题栏」这条算术于是只在 macOS
/// 宿主上被验证过，Linux 上一跑全红（断言 32、实得 0；coverage 任务同因挂掉）。
/// 分离后算术在任何宿主都可测，门本身由 cfg 成对测试钉住（AGENTS §5 跨平台分叉必带）。
fn page_origin_inset(window_physical: (f64, f64), frame: Option<EmbedFrame>) -> (f64, f64) {
    if !cfg!(target_os = "macos") {
        return (0.0, 0.0);
    }
    inset_math(window_physical, frame)
}

/// 偏移**算术**（纯函数，无平台门）：窗口物理尺寸 − 页面视口（CSS × dpr）再除回 dpr。
///
/// **2026-10-12 实测定论（「DSH Dock 右侧栏按钮全没了」的根因）**：macOS 上子视窗
/// 挂在窗口内容视图里，而页面视口比整窗矮一条标题栏（本机：窗口 1280x795、页面
/// 1280x763 ⇒ 32px）。不补这一下，请求 top=38 的子视窗实际落在页面 y≈6，整体上浮
/// 32px，正好把右侧栏顶部那条 shell chrome（标签条 + 分屏/全屏/收起）压在客页白底下；
/// 而 NSView frame 回读仍是 38，日志"看起来正确"，只有像素比对看得见。
///
/// ⚠️ 参数链路的教训（2026-10-12，本问题"修了三次没生效"的直接原因）：`frame`
/// 必须是命令的**顶层参数**。上一版把它放进 `EmbedRect` 内部，而插件发的是
/// `{ rect, frame }` 兄弟字段 —— `rect` 反序列化出的 `frame` 永远是 `None`，
/// 偏移恒为 0，日志"看起来正常"，改了等于没改。
///
/// 没报 `frame` / 非法 scale / 页面比窗口还大，都归零（不猜、不出负值）。
fn inset_math(window_physical: (f64, f64), frame: Option<EmbedFrame>) -> (f64, f64) {
    let Some(frame) = frame else {
        return (0.0, 0.0);
    };
    if frame.scale <= 0.0 || frame.width <= 0.0 || frame.height <= 0.0 {
        return (0.0, 0.0);
    }
    (
        ((window_physical.0 - frame.width * frame.scale) / frame.scale).max(0.0),
        ((window_physical.1 - frame.height * frame.scale) / frame.scale).max(0.0),
    )
}

/// 主窗口的物理尺寸。`outer_size()` 原样返回，**不除 `scale_factor()`** ——
/// 那个读数不稳定（见 `EmbedFrame::scale`），物理像素本身是可靠的。
///
/// 句柄走 `ui::main_window`（2026-10-14：guest 子视窗在场时直呼
/// `get_webview_window` 会因 `is_webview_window` 旗标翻 false 而永久 None，
/// 连这里的尺寸读数都会悄悄退化成 (0,0)——详见 ui.rs 事故记录）。
fn window_physical_size(handle: &AppHandle) -> (f64, f64) {
    crate::ui::main_window(handle)
        .and_then(|win| {
            let size = win.outer_size().ok()?;
            Some((size.width as f64, size.height as f64))
        })
        .unwrap_or((0.0, 0.0))
}

/// 页面→窗口偏移的「最后可信值」。
///
/// 实测（2026-10-12）：macOS 上 `outer_size()` **逐次调用返回不同的数**（同一窗口
/// 先后读到相当于 797 / 795 / 765 逻辑像素的高度）——open 那次算出 32、紧接着的
/// resize 算出 0，子视窗被拽回原位，按钮再次被盖。偏移只随窗口几何变化、不随
/// 调用时刻变化，因此：算出退化值（全 0）而调用方明明报了 `frame` 时，沿用最后
/// 可信值；算出可信值时刷新缓存。
static LAST_GOOD_INSET: std::sync::Mutex<Option<(f64, f64)>> = std::sync::Mutex::new(None);

/// 页面 rect 值得摆一个原生子视窗的最小边长（与插件 `syncToHost` 的 2px 同口径）。
const MIN_GUEST_SIDE: f64 = 2.0;

/// 页面 rect 是否值得摆一个原生子视窗（2026-10-14 现场故障的 Dock 侧防线）。
///
/// 插件在宿主被折叠 / 切走 / 还没量到 / 列正在动画推进时会送来退化 rect（0 宽、
/// 0 高、NaN）。这类盒子若按「最小 100x100」继续摆，就是一张卡在窗口左上角、
/// 页面盖不住、滚动也不带走的聊天白卡——原生子视窗在图层面之上，比页面里任何
/// 层级都难善后。所以退化即让位：隐藏，等真盒子到了再由 `open`/`resize` 接回。
fn rect_is_placeable(rect: EmbedRect) -> bool {
    rect.left.is_finite()
        && rect.top.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && rect.width >= MIN_GUEST_SIDE
        && rect.height >= MIN_GUEST_SIDE
}

/// 把插件的**页面坐标** rect 换算成 `add_child` 需要的窗口坐标。
pub fn child_position(
    handle: &AppHandle,
    rect: EmbedRect,
    frame: Option<EmbedFrame>,
) -> (LogicalPosition<f64>, LogicalSize<f64>, (f64, f64)) {
    child_position_with(rect, window_physical_size(handle), frame)
}

/// 纯函数部分：页面 rect + 页面视口 → 子视窗的窗口坐标位置、尺寸与所用偏移。
fn child_position_with(
    rect: EmbedRect,
    window_physical: (f64, f64),
    frame: Option<EmbedFrame>,
) -> (LogicalPosition<f64>, LogicalSize<f64>, (f64, f64)) {
    let computed = page_origin_inset(window_physical, frame);
    let mut guard = LAST_GOOD_INSET.lock().unwrap_or_else(|e| e.into_inner());
    let inset = if computed == (0.0, 0.0) && frame.is_some() {
        // 退化读数：几何 API 这次没给出可信值。沿用最后可信偏移。
        (*guard).unwrap_or((0.0, 0.0))
    } else {
        if computed != (0.0, 0.0) {
            *guard = Some(computed);
        }
        computed
    };
    drop(guard);
    (
        LogicalPosition::new(rect.left.max(0.0) + inset.0, rect.top.max(0.0) + inset.1),
        LogicalSize::new(rect.width.max(100.0), rect.height.max(100.0)),
        inset,
    )
}

/// 回读子视窗当前实际 bounds 并打日志（诊断用，永不失败）。
fn trace_bounds(
    handle: &AppHandle,
    where_: &str,
    want: EmbedRect,
    frame: Option<EmbedFrame>,
    inset: (f64, f64),
) {
    let Some(wv) = handle.get_webview(CHAT_GUEST_LABEL) else {
        tracing::warn!("embed[{where_}] 回读时子视窗不存在");
        return;
    };
    // 仅用于把子视窗 bounds 回读换算成逻辑像素打日志。
    // 注意 `Window::scale_factor()` 返回 `Result`（与 WebviewWindow 的同名方法
    // 签名不同），这里显式 `.ok()` 收成 f64。
    let sf = crate::ui::main_window(handle)
        .and_then(|win| win.scale_factor().ok())
        .unwrap_or(1.0);
    let (inset_x, inset_y) = inset;
    match wv.bounds() {
        Ok(b) => {
            let pos = b.position.to_logical::<f64>(sf);
            let size = b.size.to_logical::<f64>(sf);
            tracing::info!(
                "embed[{where_}] want=({},{},{},{}) frame={frame:?} inset=({inset_x:.0},{inset_y:.0}) actual=({:.1},{:.1},{:.1},{:.1}) sf={sf}",
                want.left,
                want.top,
                want.width,
                want.height,
                pos.x,
                pos.y,
                size.width,
                size.height
            );
        }
        Err(e) => tracing::warn!("embed[{where_}] bounds 回读失败: {e}"),
    }
}

/// 打开或显示内嵌聊天子 Webview
#[tauri::command]
pub fn embed_chat_open(
    app: AppHandle,
    rect: EmbedRect,
    frame: Option<EmbedFrame>,
) -> Result<(), String> {
    let handle = app.clone();
    crate::ui::post_to_event_loop(&app, move || {
        // 退化 rect 一律让位（2026-10-14 现场故障）：插件在宿主被折叠 / 还没量到 /
        // 列正在动画时会送来这种盒子，旧口径把它抬到 100x100 继续摆，结果是一张
        // 卡在窗口左上、谁也点不动的聊天白卡——而原生子视窗不参与页面滚动，页面
        // 也盖不住它。等插件量到真盒子会再 open 一次。
        if !rect_is_placeable(rect) {
            if let Some(wv) = handle.get_webview(CHAT_GUEST_LABEL) {
                let _ = wv.hide();
            }
            tracing::info!(
                "embed[open:degenerate] rect=({},{},{},{}) 退化，让位不摆",
                rect.left,
                rect.top,
                rect.width,
                rect.height
            );
            return;
        }

        // 页面坐标 → 窗口坐标：差值 = 窗口 chrome（标题栏），由 `frame` 揭示。
        let (pos, size, inset) = child_position(&handle, rect, frame);

        // 1. 若子视窗已存在：更新位置/尺寸并展示
        if let Some(wv) = handle.get_webview(CHAT_GUEST_LABEL) {
            let _ = wv.set_position(pos);
            let _ = wv.set_size(size);
            let _ = wv.show();
            trace_bounds(&handle, "open:existing", rect, frame, inset);
            return;
        }

        // 2. 首次创建：在主窗口中挂载顶层子 Webview
        //    主窗口句柄走 ui::main_window（2026-10-14：guest 二次挂载场景下
        //    直呼 get_webview_window 会踩 is_webview_window 旗标雷，见 ui.rs）。
        let Some(main_win) = crate::ui::main_window(&handle) else {
            tracing::warn!("embed_chat_open: 未找到主窗口");
            return;
        };

        let target_url: tauri::Url = match CHAT_URL.parse() {
            Ok(u) => u,
            Err(e) => {
                tracing::error!("embed_chat_open: 解析 URL 失败: {e}");
                return;
            }
        };

        let cleanup_script = r#"
            // 消除 DeepSeek 前端对 Tauri 环境的检测，彻底抑制「使用环境异常」弹窗
            try {
                delete window.__TAURI__;
                delete window.__TAURI_INTERNALS__;
            } catch (e) {}
            try {
                localStorage.setItem('closeUnsafeEnvWarn', 'true');
            } catch (e) {}
            // 强制将语言设定为简体中文（zh-CN），避免 macOS WKWebView 默认汇报 en-US 导致全英文
            try {
                Object.defineProperty(navigator, 'language', { get: () => 'zh-CN', configurable: true });
                Object.defineProperty(navigator, 'languages', { get: () => ['zh-CN', 'zh', 'en'], configurable: true });
                for (let i = 0; i < localStorage.length; i++) {
                    const k = localStorage.key(i);
                    if (k && k.includes('localePreference')) {
                        localStorage.setItem(k, '"zh_CN"');
                    }
                }
            } catch (e) {}
        "#;

        let title_app = handle.clone();
        let builder = WebviewBuilder::new(CHAT_GUEST_LABEL, WebviewUrl::External(target_url))
            .user_agent(NORMALIZED_UA)
            .initialization_script(cleanup_script)
            .initialization_script(GUEST_SIGNAL_SCRIPT)
            .on_document_title_changed(move |webview, title| {
                // 只有真客页的标题才算桥消息：普通标题、其它 Webview 一律忽略。
                if !is_chat_guest_webview(&webview) {
                    return;
                }
                let Some(message) = parse_guest_title(&title) else {
                    return;
                };
                if let Err(error) = title_app.emit_to(
                    EventTarget::webview(MAIN_WEBVIEW_LABEL),
                    GUEST_MESSAGE_EVENT,
                    message,
                ) {
                    tracing::warn!("embed[bridge] 事件回传失败: {error}");
                }
            });

        match main_win.add_child(builder, pos, size) {
            Ok(wv) => {
                let _ = wv.show();
                tracing::info!(
                    "DeepSeek Chat 原生子视窗挂载成功：label={CHAT_GUEST_LABEL} rect={rect:?}"
                );
                trace_bounds(&handle, "open:created", rect, frame, inset);
            }
            Err(e) => {
                tracing::error!("挂载 DeepSeek Chat 子视窗失败: {e}");
            }
        }
    });
    Ok(())
}

/// 在客页执行一段短脚本，结果经标题信道回传（ADR-0033）。
///
/// 这是唯一一处「宿主 → 客页」的脚本面，调用方校验是硬边界：
/// 调用者必须是 label 为 `main` 的 Webview，且其当前 URL 是 loopback 工作台。
/// 远端客页 `deepseek_chat_guest` 不在任何 capability 里，本来就调不到本命令；
/// 这里的校验是第二道闸（防将来 capability 被误扩）。
///
/// 命令本身不等待结果：客页把 `encodeURIComponent(JSON.stringify(result))` 写进
/// 固定前缀的 `document.title`，由 `on_document_title_changed` 解析后向 `main`
/// 发 `dsce:guest-message`（`kind = "result"`，带同一个 `requestId`），插件侧按
/// request id 配对。这样 Rust 侧不需要任何在途请求状态，也不存在超时/泄漏。
#[tauri::command]
pub async fn embed_chat_eval(
    app: AppHandle,
    webview: Webview,
    request_id: String,
    script: String,
) -> Result<(), String> {
    if webview.label() != MAIN_WEBVIEW_LABEL {
        return Err("embed_chat_eval: 仅允许主工作台调用".into());
    }
    if !webview.url().is_ok_and(|url| is_local_workbench_url(&url)) {
        return Err("embed_chat_eval: 调用方不是本地 loopback 工作台".into());
    }
    let js = eval_result_script(&request_id, &script)?;
    let Some(guest) = app.get_webview(CHAT_GUEST_LABEL) else {
        return Err("embed_chat_eval: 客页子视窗尚未挂载".into());
    };
    guest
        .eval(js)
        .map_err(|error| format!("embed_chat_eval: eval 失败: {error}"))
}

/// 隐藏内嵌聊天子 Webview（切回 DSH 会话时调用，保留进程与登录态）
#[tauri::command]
pub fn embed_chat_hide(app: AppHandle) -> Result<(), String> {
    let handle = app.clone();
    crate::ui::post_to_event_loop(&app, move || {
        if let Some(wv) = handle.get_webview(CHAT_GUEST_LABEL) {
            let _ = wv.hide();
            tracing::info!("embed[hide] 子视窗已隐藏");
        }
    });
    Ok(())
}

/// 窗口或中栏尺寸变动时同步子 Webview 几何坐标
#[tauri::command]
pub fn embed_chat_resize(
    app: AppHandle,
    rect: EmbedRect,
    frame: Option<EmbedFrame>,
) -> Result<(), String> {
    let handle = app.clone();
    crate::ui::post_to_event_loop(&app, move || {
        // 退化 rect = 宿主已经不在页面上（折叠 / 切走 / 未量到）：让位，别按最小
        // 尺寸摆一张白卡（见 embed_chat_open 的同口径判定）。
        if !rect_is_placeable(rect) {
            if let Some(wv) = handle.get_webview(CHAT_GUEST_LABEL) {
                let _ = wv.hide();
            }
            tracing::info!(
                "embed[resize:degenerate] rect=({},{},{},{}) 退化，隐藏子视窗",
                rect.left,
                rect.top,
                rect.width,
                rect.height
            );
            return;
        }
        if let Some(wv) = handle.get_webview(CHAT_GUEST_LABEL) {
            let (pos, size, inset) = child_position(&handle, rect, frame);
            let _ = wv.set_position(pos);
            let _ = wv.set_size(size);
            trace_bounds(&handle, "resize", rect, frame, inset);
        }
    });
    Ok(())
}

/// 彻底关闭并销毁内嵌聊天子视窗（插件销毁时调用）
#[tauri::command]
pub fn embed_chat_close(app: AppHandle) -> Result<(), String> {
    let handle = app.clone();
    crate::ui::post_to_event_loop(&app, move || {
        if let Some(wv) = handle.get_webview(CHAT_GUEST_LABEL) {
            tracing::info!("embed[close] 子视窗销毁");
            let _ = wv.close();
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(raw: &str) -> tauri::Url {
        raw.parse().expect("测试用 URL 必须合法")
    }

    #[test]
    fn title_result_round_trips() {
        let message = parse_guest_title("__DSCE_RESULT__:r12-1760000000000-3:collapsed")
            .expect("结果消息可解析");
        assert_eq!(message.kind, "result");
        assert_eq!(message.request_id, "r12-1760000000000-3");
        assert_eq!(message.value.as_deref(), Some("collapsed"));
    }

    #[test]
    fn title_expand_round_trips() {
        let message =
            parse_guest_title("__DSCE_EVENT__:expand:1760000000000-1").expect("事件可解析");
        assert_eq!(message.kind, "expand");
        assert_eq!(message.request_id, "1760000000000-1");
        assert_eq!(message.value, None);
    }

    #[test]
    fn ordinary_titles_are_ignored() {
        for title in [
            "",
            "DeepSeek - 探索未至之境",
            "__DSCE_RESULT__",
            "__DSCE_RESULT__:r1",
            "__DSCE_EVENT__:collapse:r1",
            "__DSCE_EVENT__:expand",
            "前缀 __DSCE_RESULT__:r1:ok",
        ] {
            assert!(
                parse_guest_title(title).is_none(),
                "普通标题被误判为桥消息: {title}"
            );
        }
    }

    #[test]
    fn bridge_ids_reject_injection_shapes() {
        for id in [
            "",
            "a b",
            "a:b",
            "a/b",
            "a.b",
            "../../etc",
            "r1\n",
            &"x".repeat(65),
        ] {
            assert!(!valid_bridge_id(id), "非法 request id 被放过: {id:?}");
        }
        assert!(valid_bridge_id("r12-1760000000000-3"));
        assert!(valid_bridge_id("a_b-9"));
    }

    #[test]
    fn oversized_titles_are_ignored() {
        let title = format!("__DSCE_RESULT__:r1:{}", "a".repeat(MAX_TITLE_MESSAGE_BYTES));
        assert!(parse_guest_title(&title).is_none());
    }

    #[test]
    fn oversized_result_payload_is_ignored() {
        let title = format!(
            "__DSCE_RESULT__:r1:{}",
            "a".repeat(MAX_RESULT_PAYLOAD_BYTES + 1)
        );
        assert!(
            parse_guest_title(&title).is_none(),
            "超长载荷必须被丢弃而不是截断"
        );
        let fitting = format!(
            "__DSCE_RESULT__:r1:{}",
            "a".repeat(MAX_RESULT_PAYLOAD_BYTES)
        );
        assert!(parse_guest_title(&fitting).is_some());
    }

    #[test]
    fn guest_url_requires_exact_https_host() {
        assert!(is_chat_guest_url(&url("https://chat.deepseek.com/")));
        assert!(is_chat_guest_url(&url("https://chat.deepseek.com/a/b?c=d")));
        for bad in [
            "http://chat.deepseek.com/",
            "https://chat.deepseek.com.evil.example/",
            "https://chat.deepseek.com.evil.example/x",
            "https://evil.example/?next=https://chat.deepseek.com/",
            "https://www.deepseek.com/",
            "https://api.deepseek.com/",
            "file:///chat.deepseek.com",
        ] {
            assert!(
                !is_chat_guest_url(&url(bad)),
                "非客页 URL 被认成客页: {bad}"
            );
        }
    }

    #[test]
    fn local_workbench_requires_loopback_http() {
        assert!(is_local_workbench_url(&url("http://127.0.0.1:61278/chat")));
        assert!(is_local_workbench_url(&url("http://localhost:1420/")));
        assert!(is_local_workbench_url(&url("http://[::1]:1420/")));
        for bad in [
            "https://127.0.0.1:61278/",
            "http://127.0.0.1.evil.example/",
            "http://evil.example/",
            "file:///tmp/workbench",
            "tauri://localhost/",
        ] {
            assert!(
                !is_local_workbench_url(&url(bad)),
                "非 loopback 被放行: {bad}"
            );
        }
    }

    #[test]
    fn eval_script_rejects_bad_input() {
        assert!(eval_result_script("bad id", "1").is_err());
        assert!(eval_result_script("r1", &"x".repeat(MAX_EVAL_SCRIPT_BYTES + 1)).is_err());
        assert!(eval_result_script("r1", "1").is_ok());
    }

    #[test]
    fn eval_script_carries_id_prefix_and_limit() {
        let js = eval_result_script("r12-3", "(() => 'expanded')()").expect("脚本可生成");
        assert!(js.contains("\"r12-3\""), "request id 必须进脚本: {js}");
        assert!(js.contains(RESULT_TITLE_PREFIX), "结果前缀必须进脚本");
        assert!(!js.contains(ACTION_TITLE_PREFIX), "结果脚本不得带事件前缀");
        assert!(
            js.contains(&MAX_RESULT_PAYLOAD_BYTES.to_string()),
            "载荷上限必须进脚本"
        );
        assert!(js.contains("(() => 'expanded')()"), "原脚本必须被原样嵌入");
    }

    /// 结果脚本把「脚本抛错」也回传成 `{error}`，而不是让调用方永远等下去。
    #[test]
    fn eval_script_reports_errors() {
        let js = eval_result_script("r1", "throw new Error('boom')").expect("脚本可生成");
        assert!(js.contains("error"), "错误分支必须存在");
        assert!(
            js.contains("document.title=marker"),
            "结果必须经标题信道回传"
        );
        assert!(js.contains("document.title=previous"), "标题必须恢复");
    }

    /// 本机 Retina 实测形状：窗口物理 2560x1590，页面视口 1280x763 @2x。
    fn retina() -> Option<EmbedFrame> {
        Some(EmbedFrame {
            width: 1280.0,
            height: 763.0,
            scale: 2.0,
        })
    }

    fn rect(top: f64) -> EmbedRect {
        EmbedRect {
            left: 820.0,
            top,
            width: 461.0,
            height: 757.0,
        }
    }

    /// 算术（无平台门，**任何宿主都必须成立**——2026-10-13 Linux CI 三红的修法：
    /// 旧测试直接测带 mac 门的 `page_origin_inset`，这条算术只在 macOS 宿主上
    /// 被验证过）：本机实测窗口 1280x795（物理 2560x1590）、页面视口 1280x763 @2x
    /// ⇒ 子视窗要下移 32px。偏移用物理像素减（页面 CSS × dpr）再除回 dpr，
    /// **不经过 `scale_factor()`**（那个读数实测不稳）。
    #[test]
    fn inset_math_is_the_window_minus_the_page() {
        assert_eq!(inset_math((2560.0, 1590.0), retina()), (0.0, 32.0));
    }

    /// 1x 显示器同样成立：窗口物理 1280x795、页面 1280x763 @1x ⇒ (0, 32)。
    #[test]
    fn inset_math_works_on_a_1x_display_too() {
        assert_eq!(
            inset_math(
                (1280.0, 795.0),
                Some(EmbedFrame {
                    width: 1280.0,
                    height: 763.0,
                    scale: 1.0
                })
            ),
            (0.0, 32.0)
        );
    }

    /// 调用方没报页面视口 ⇒ 没有可信差值 ⇒ 不补（宁可不补也不猜）。
    #[test]
    fn missing_frame_means_no_inset() {
        assert_eq!(inset_math((2560.0, 1590.0), None), (0.0, 0.0));
    }

    /// 非法 scale / 页面比窗口还大，都不得变成负偏移或除零。
    #[test]
    fn bad_scale_and_oversized_frame_clamp_to_zero() {
        assert_eq!(
            inset_math(
                (2560.0, 1590.0),
                Some(EmbedFrame {
                    width: 1280.0,
                    height: 763.0,
                    scale: 0.0
                })
            ),
            (0.0, 0.0)
        );
        assert_eq!(
            inset_math(
                (2560.0, 1590.0),
                Some(EmbedFrame {
                    width: 1400.0,
                    height: 1800.0,
                    scale: 2.0
                })
            ),
            (0.0, 0.0)
        );
    }

    /// 平台门语义（AGENTS §5 跨平台分叉必带，成对钉住）：
    /// macOS 上补标题栏那条偏移。
    #[cfg(target_os = "macos")]
    #[test]
    fn page_origin_inset_compensates_the_title_bar_on_macos() {
        assert_eq!(page_origin_inset((2560.0, 1590.0), retina()), (0.0, 32.0));
    }

    /// 非 macOS **一律不补**：Windows/Linux 子视窗直接挂客户区，页面坐标即
    /// 子视窗坐标——补了反而整体下移一个标题栏（2026-10-13 Linux CI 挂的正是
    /// 缺这条对照：旧测试只有 mac 一支，分叉另一侧无测试代言）。
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn page_origin_inset_never_compensates_off_macos() {
        assert_eq!(page_origin_inset((2560.0, 1590.0), retina()), (0.0, 0.0));
    }

    /// 端到端那一下（macOS）：页面 top=38 的 rect，在本机窗口（物理 2560x1590）/
    /// 页面 763 下必须落到窗口 y=70 —— 正好让出页面 0..38 那条 shell chrome。
    #[cfg(target_os = "macos")]
    #[test]
    fn child_position_applies_the_inset_to_the_page_box() {
        let (pos, size, inset) = child_position_with(rect(38.0), (2560.0, 1590.0), retina());
        assert_eq!(inset, (0.0, 32.0));
        assert_eq!((pos.x, pos.y), (820.0, 70.0));
        assert_eq!((size.width, size.height), (461.0, 757.0));
    }

    /// 端到端（非 macOS）：页面坐标即子视窗坐标，top=38 就落 38，不补偏移。
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn child_position_leaves_the_page_box_alone_off_macos() {
        let (pos, size, inset) = child_position_with(rect(38.0), (2560.0, 1590.0), retina());
        assert_eq!(inset, (0.0, 0.0));
        assert_eq!((pos.x, pos.y), (820.0, 38.0));
        assert_eq!((size.width, size.height), (461.0, 757.0));
    }

    #[test]
    fn negative_top_is_clamped_and_no_frame_leaves_the_box_alone() {
        let (pos, size, inset) = child_position_with(rect(-40.0), (2560.0, 1590.0), None);
        assert_eq!(inset, (0.0, 0.0));
        assert_eq!((pos.x, pos.y), (820.0, 0.0));
        assert_eq!((size.width, size.height), (461.0, 757.0));
    }

    #[test]
    fn tiny_boxes_are_raised_to_the_minimum() {
        let (_, size, inset) = child_position_with(rect(0.0), (0.0, 0.0), None);
        assert_eq!(inset, (0.0, 0.0));
        assert_eq!((size.width, size.height), (461.0, 757.0));
        let (_, small, _) = child_position_with(
            EmbedRect {
                left: 0.0,
                top: 0.0,
                width: 4.0,
                height: 4.0,
            },
            (0.0, 0.0),
            None,
        );
        assert_eq!((small.width, small.height), (100.0, 100.0));
    }

    /// 退化 rect（宿主折叠 / 未量到 / NaN）必须判为不可摆：宁可不显示，也不摆一张
    /// 100x100 的白卡（2026-10-14 现场故障在 Dock 侧的防线）。
    #[test]
    fn degenerate_boxes_are_not_placeable() {
        let zero_height = EmbedRect {
            left: 0.0,
            top: 446.0,
            width: 538.0,
            height: 0.0,
        };
        assert!(!rect_is_placeable(zero_height), "0 高 = 宿主已折叠");

        let zero_width = EmbedRect {
            left: 0.0,
            top: 446.0,
            width: 0.0,
            height: 124.0,
        };
        assert!(!rect_is_placeable(zero_width), "0 宽 = 宿主已折叠");

        let nan = EmbedRect {
            left: f64::NAN,
            top: 0.0,
            width: 538.0,
            height: 124.0,
        };
        assert!(!rect_is_placeable(nan), "NaN 坐标不可信");

        let real = EmbedRect {
            left: 0.0,
            top: 446.0,
            width: 538.0,
            height: 124.0,
        };
        assert!(
            rect_is_placeable(real),
            "真盒子照摆（现场那张白卡的几何本身合法）"
        );

        let small = EmbedRect {
            left: 0.0,
            top: 0.0,
            width: 4.0,
            height: 4.0,
        };
        assert!(
            rect_is_placeable(small),
            "4x4 仍算可摆，由最小尺寸抬到 100x100"
        );
    }
}
