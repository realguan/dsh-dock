//! mcp_probe.rs —— MCP 服务器能力探测（2026-09-15，ADR-0022 **两条分支**）。
//!
//! ## 职责
//!
//! 主动连上一个 MCP 服务器，握手后枚举 Tools / Resources / Resource Templates，
//! 供「MCP 工作台」呈现**配置体检与失败归因**（ADR-0022 §1.4 的价值主张）。
//!
//! ## 为什么壳要自己说 MCP（ADR-0022 §1）
//!
//! DSH **没有**可供 GUI 枚举 MCP 的 RPC：`packages/api/` 零 `mcp` 命中，
//! `mcp-resources` 只注册三个**面向模型**的工具。壳要看得见能力，只有两条路——
//! 读 patch 文件（已有），或自己说协议（本模块）。
//!
//! ## 合规边界（ADR-0022 §3.3，**分支即纪律**）
//!
//! 本模块同时承载**两条互不相同**的合规通道（2026-09-15 R3 起）：
//!
//! - **`stdio` 分支**（[`probe_stdio`]）：子进程 + 逐行 JSON-RPC。
//!   - spawn **必须**经 [`crate::lifecycle`] seam（AGENTS §6 机器闸门拦裸 `Command::spawn`），
//!     角色用 `Role::Probe`（短命探测，不堆 `procs/` 目录）；
//!   - Windows 经 [`crate::child_cmd`]（`.cmd/.bat` 直 spawn 必败）；
//!   - 网络发生在**被 spawn 的 MCP 服务器进程内**，本文件不得出现进程内客户端。
//! - **`streamable-http` 分支**（[`probe_http`]）：进程内 HTTP。
//!   - 本模块内**唯一**的进程内网络原语在 [`post_rpc`] 里，按**条目**授权
//!     （`network_gate.rs` 的 `item: Some("post_rpc")` + AGENTS §7 登记），
//!     **不是整文件豁免**；
//!   - `mcp_probe.rs` 的整文件 `Kind::Registered` 行**仍然保留**——它继续守
//!     "除该条目外，本文件不得再有进程内原语"。将来有人加第三处触网（例如
//!     `resources/read` 预览），主闸门照样红。
//!   - 因此：`ureq` 只允许以**全限定路径**写在 `fn post_rpc` 体内。在模块顶部
//!     `use ureq::…` 会让原语落在豁免条目范围之外 → 闸门当场红（这是设计，不是坑）。
//!
//! ## 线协议锚点（2026-09-15 实查）
//!
//! - 安装的 SDK 为 `@modelcontextprotocol/sdk@1.30.0`，
//!   `LATEST_PROTOCOL_VERSION = "2025-11-25"`，supported = [该版本, `2025-06-18`,
//!   `2025-03-26`, `2024-11-05`, `2024-10-07`]。本模块发送最新版，并**以服务端
//!   协商回来的版本为准**（服务端可能降级）。
//! - 方法名：`initialize` / `tools/list` / `resources/list` / `resources/templates/list`
//!   （后两者 DSH 自身亦在用，见 `mcp-client/src/connection.ts:372,376`）。
//! - 能力可选：服务端未声明 `resources` 时，`resources/list` 可能回 `-32601`；
//!   此时**降级为「不支持」而不是让整个探测失败**（并在 `notes` 里如实记一笔）。

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::mcp::McpServerConfig;

/// 本模块发送的协议版本（与 DSH 所用 SDK 一致）。协商结果以服务端返回为准。
pub const LATEST_PROTOCOL_VERSION: &str = "2025-11-25";

/// 一条具名能力（tool / resource / template）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpNamed {
    /// 名称：tool 取 `name`；resource 取 `name`（缺省回退 `uri`）；template 取 `name`
    /// （缺省回退 `uriTemplate`）。
    pub name: String,
    /// 描述或标识：tool 取 `description`；resource 取 `uri`；template 取 `uriTemplate`。
    pub detail: String,
}

/// 一次探测的结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpProbe {
    /// **服务端协商后**的协议版本（非我方发送值）。
    pub protocol_version: String,
    /// 服务端自报名称（缺省回退配置里的 `serverName`）。
    pub server_name: String,
    pub tools: Vec<McpNamed>,
    pub resources: Vec<McpNamed>,
    pub templates: Vec<McpNamed>,
    /// 降级说明（如"该服务器不支持 resources"）；空 = 全部枚举成功。
    pub notes: Vec<String>,
}

// ---------- 纯函数：请求构造与信封解析（静默失败面，重点单测） ----------

/// `initialize` 请求。`capabilities` 留空对象：本探测器只读，不声明任何能力。
pub fn initialize_request(id: u64) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "protocolVersion": LATEST_PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": { "name": "dsh-dock", "version": env!("CARGO_PKG_VERSION") },
        },
    })
    .to_string()
}

/// `notifications/initialized`：握手第二步，**无 `id`**（通知，等服务端响应会卡死）。
pub fn initialized_notification() -> String {
    serde_json::json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }).to_string()
}

/// 列能力请求（`tools/list`、`resources/list`、`resources/templates/list`）。
pub fn list_request(id: u64, method: &str) -> String {
    serde_json::json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": {} }).to_string()
}

/// 解析一行 JSON-RPC 信封。
///
/// 返回 `None` 表示**这一行不该唤醒等待中的请求**（无 `id` 的通知/日志行/非 JSON）——
/// 调用方应**跳过并继续等**，而不是当失败。返回 `Some((id, Ok(result)))` 或
/// `Some((id, Err(错误描述)))`。
pub fn parse_rpc_envelope(line: &str) -> Option<(u64, Result<serde_json::Value, String>)> {
    let value: serde_json::Value = serde_json::from_str(line.trim()).ok()?;
    let id = value.get("id").and_then(|v| v.as_u64())?;
    if let Some(error) = value.get("error") {
        let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
        let message = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("(无 message)");
        return Some((id, Err(format!("{code}: {message}"))));
    }
    Some((
        id,
        Ok(value
            .get("result")
            .cloned()
            .unwrap_or(serde_json::Value::Null)),
    ))
}

/// 把 `{ "<key>": [ {name, description|uri|uriTemplate} ] }` 抽成具名列表。
///
/// `name_key` 缺失时回退 `fallback_key`（resource 的 `name` 可缺，`uri` 恒有）。
pub fn parse_named_list(
    result: &serde_json::Value,
    key: &str,
    name_key: &str,
    detail_key: &str,
    fallback_key: &str,
) -> Vec<McpNamed> {
    let Some(items) = result.get(key).and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    items
        .iter()
        .map(|item| {
            let detail = item
                .get(detail_key)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let name = item
                .get(name_key)
                .and_then(|v| v.as_str())
                .or_else(|| item.get(fallback_key).and_then(|v| v.as_str()))
                .unwrap_or_default()
                .to_string();
            McpNamed { name, detail }
        })
        .collect()
}

/// 是否属"方法不支持"（`-32601`）：据此降级而非整单失败。
pub fn is_method_not_found(error: &str) -> bool {
    error.starts_with("-32601")
}

// ---------- 传输：stdio 子进程 + 逐行 JSON-RPC ----------

/// 等待指定 `id` 的响应（跳过通知与其他 id；带总期限）。
fn await_id(
    rx: &mpsc::Receiver<String>,
    want: u64,
    deadline: Instant,
    label: &str,
) -> Result<serde_json::Value, String> {
    loop {
        let now = Instant::now();
        if now >= deadline {
            return Err(format!("{label} 超时（未在期限内收到 id={want} 的响应）"));
        }
        match rx.recv_timeout(deadline - now) {
            Ok(line) => {
                if line.trim().is_empty() {
                    continue;
                }
                match parse_rpc_envelope(&line) {
                    Some((id, Ok(result))) if id == want => return Ok(result),
                    Some((id, Err(message))) if id == want => {
                        return Err(format!("{label} 被服务端拒绝：{message}"))
                    }
                    // 通知 / 其他请求的响应：MCP 允许交错，跳过继续等。
                    _ => continue,
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                return Err(format!("{label} 超时（等待响应期间无数据）"))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(format!("{label} 失败：服务器已退出（stdout 关闭）"))
            }
        }
    }
}

/// 探测一个 **stdio** MCP 服务器（阻塞；调用方负责放进 `spawn_blocking`）。
///
/// `timeout` 是**整轮**探测的总期限（含启动、握手与三次枚举）——避免一个卡住的
/// 服务器把详情页挂死。
pub fn probe_stdio(server: &McpServerConfig, timeout: Duration) -> Result<McpProbe, String> {
    if server.command.trim().is_empty() {
        return Err(format!(
            "MCP 服务器「{}」的 transport 是 stdio，但配置里没有 command，无法探测\
             （若它其实是 streamable-http，请把 transport 改为 streamable-http 并填 url）",
            server.name
        ));
    }
    let mut cmd = crate::child_cmd(std::path::Path::new(&server.command));
    cmd.args(&server.args)
        .envs(&server.env)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    // 守卫式 spawn（ADR-0015/AGENTS §6）：短命探测用 Role::Probe
    // ——不把 procs/ 目录堆满（lifecycle 对该角色有就地删除口径）。
    let mut child = crate::lifecycle::spawn(
        &mut cmd,
        crate::lifecycle::Role::Probe,
        crate::lifecycle::GuardCtx::of(&server.command, None),
    )
    .map_err(|e| format!("启动 MCP 服务器失败（{}）：{e}", server.command))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "无法取得 MCP 服务器 stdin".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "无法取得 MCP 服务器 stdout".to_string())?;
    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(l) => {
                    if tx.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let deadline = Instant::now() + timeout;
    let outcome = run_probe(&mut stdin, &rx, deadline, server);

    // 无论成败都收干净子进程（守护是兜底，这里是正路）。
    let _ = child.kill();
    let _ = child.wait();
    outcome
}

/// 三次枚举的结果 → [`McpProbe`]（**两分支共用**，2026-09-15 R3 抽取）。
///
/// 为什么必须是共享的：`tools` 失败即整单失败、`resources` / `templates` 遇 `-32601`
/// 降级为"空 + 说明"——这套口径是 ADR-0022 §2.9「两条分支对前端呈现为**同一种卡片**」
/// 的落点。两分支各写一份 = 两套降级口径，将来必然分叉（且分叉只在某一种 transport
/// 上暴露，最难发现）。
fn assemble_probe(
    fallback_name: &str,
    init: &serde_json::Value,
    tools: Result<serde_json::Value, String>,
    resources: Result<serde_json::Value, String>,
    templates: Result<serde_json::Value, String>,
) -> Result<McpProbe, String> {
    // 服务端可能降级协议版本——**以它为准**（我方发送值只作缺省）。
    let protocol_version = init
        .get("protocolVersion")
        .and_then(|v| v.as_str())
        .unwrap_or(LATEST_PROTOCOL_VERSION)
        .to_string();
    let server_name = init
        .pointer("/serverInfo/name")
        .and_then(|v| v.as_str())
        .unwrap_or(fallback_name)
        .to_string();

    // tools 是 MCP 的必备能力：这一条失败就是**探测失败**，不是"没这能力"。
    let tools = parse_named_list(&tools?, "tools", "name", "description", "name");

    // resources / templates 属可选能力：服务端不支持时降级为"空 + 说明"，
    // **不得**让整个探测失败（否则用户看到的是"连不上"，而其实是"没这能力"）。
    let mut notes = Vec::new();
    let resources = match resources {
        Ok(result) => parse_named_list(&result, "resources", "name", "uri", "uri"),
        Err(e) if is_method_not_found(&e) => {
            notes.push("该服务器未声明 resources 能力（resources/list 返回 -32601）".to_string());
            Vec::new()
        }
        Err(e) => return Err(e),
    };
    let templates = match templates {
        Ok(result) => parse_named_list(
            &result,
            "resourceTemplates",
            "name",
            "uriTemplate",
            "uriTemplate",
        ),
        Err(e) if is_method_not_found(&e) => {
            notes.push(
                "该服务器未声明 resource templates 能力（resources/templates/list 返回 -32601）"
                    .to_string(),
            );
            Vec::new()
        }
        Err(e) => return Err(e),
    };

    Ok(McpProbe {
        protocol_version,
        server_name,
        tools,
        resources,
        templates,
        notes,
    })
}

// ---------- streamable-http 分支（2026-09-15 R3，ADR-0022 §3.3 另一条合规路径） ----------

/// 一次 streamable-http 往返的原始结果。
///
/// **非 2xx 也是 Ok**：MCP 服务端对未知方法常回 `HTTP 200 + -32601` 信封，也可能回
/// `HTTP 400 + -32601` 信封。若在这里按状态码短路成 `Err`，`resources/list` 的
/// `-32601` 降级就永远走不到——用户会把"没这能力"读成"连不上"。判定集中在
/// [`pick_result`] 一处。
struct HttpRpc {
    status: u16,
    content_type: String,
    body: String,
    /// 服务端在 `initialize` 响应里下发的 `Mcp-Session-Id`；后续请求必须回带。
    session_id: Option<String>,
}

/// HTTP 响应体 → JSON-RPC 负载列表（**纯函数**）。
///
/// streamable-http 允许两种响应形态，**两种都要认**：
/// - `application/json`：整体即一条消息；也可能是 JSON-RPC **批量**（数组）——逐元素
///   当一个负载。整段解析不了时再按行试一次（部分服务端用 NDJSON 回多条消息，
///   逐行解析不了的碎片由 [`parse_rpc_envelope`] 自然丢弃）；
/// - `text/event-stream`（SSE）：负载藏在 `data:` 字段里。SSE 规定同一事件的多行
///   `data:` 以 `\n` 拼接、空行结束事件——**不能按行切**，否则多行 JSON 会被截断成
///   两条解析不了的碎片。
///
/// 只认 JSON 形态会把**合规**的 SSE 服务端判成"连不上"，那是假失败——本功能的全部
/// 价值就是"失败可归因"，假失败比不报更糟。
pub fn rpc_payloads(content_type: &str, body: &str) -> Vec<String> {
    let media_type = content_type.split(';').next().unwrap_or("").trim();
    if !media_type.eq_ignore_ascii_case("text/event-stream") {
        if body.trim().is_empty() {
            return Vec::new();
        }
        return match serde_json::from_str::<serde_json::Value>(body) {
            Ok(serde_json::Value::Array(items)) => items.iter().map(|i| i.to_string()).collect(),
            Ok(_) => vec![body.to_string()],
            Err(_) => body
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect(),
        };
    }

    let mut out = Vec::new();
    let mut acc: Vec<&str> = Vec::new();
    for raw in body.lines() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.is_empty() {
            if !acc.is_empty() {
                out.push(acc.join("\n"));
                acc.clear();
            }
            continue;
        }
        if line.starts_with(':') {
            continue; // SSE 注释（心跳常用 `: keep-alive`）不是负载
        }
        if let Some(rest) = line.strip_prefix("data:") {
            acc.push(rest.strip_prefix(' ').unwrap_or(rest));
        }
        // event: / id: / retry: 与 JSON-RPC 负载无关，忽略。
    }
    if !acc.is_empty() {
        out.push(acc.join("\n"));
    }
    out
}

/// 从一次 HTTP 往返里取出**指定 `id`** 的信封结果。
///
/// 复用 [`parse_rpc_envelope`]（与 stdio 分支同一信封口径）：非 JSON 负载、无 `id`
/// 的通知、心跳行都会被跳过——**跳过不是失败**。
fn pick_result(rpc: &HttpRpc, id: u64, what: &str) -> Result<serde_json::Value, String> {
    let payloads = rpc_payloads(&rpc.content_type, &rpc.body);
    for p in &payloads {
        if let Some((got, outcome)) = parse_rpc_envelope(p) {
            if got == id {
                return outcome;
            }
        }
    }
    // 300 段：redirects=0 下跳转不会被跟随，如实报出来（跳转几乎都是端点写错，
    // 跟随它等于换了端点还报成功，且会把用户配置的鉴权头带到另一个 host）。
    let hint = if (300..400).contains(&rpc.status) {
        "——该地址返回跳转且本探测器不跟随跳转（探测目标是配置里的 URL 本身）"
    } else {
        ""
    };
    Err(format!(
        "HTTP {} 未返回 {what} 的 JSON-RPC 响应{hint}：收到 {} 条负载，均无 id={id}。响应片段：{}",
        rpc.status,
        payloads.len(),
        truncate(rpc.body.trim(), 300)
    ))
}

/// 截断错误详情：错误信息要能一眼读完，也不能把整页 HTML 灌进 UI。
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max).collect();
    format!("{head}…")
}

/// 一次 streamable-http JSON-RPC POST。
///
/// **本函数是本模块唯一的进程内网络原语落点**，也是 `network_gate.rs` 里
/// `item: Some("post_rpc")` 那条豁免指向的条目。往本函数**之外**添 `ureq` /
/// `reqwest` / `TcpStream` 会被主闸门拦下（整文件 `Kind::Registered` 行仍在守）；
/// 因此 `ureq` 只能以全限定路径出现在这里，模块顶部不得 `use ureq::…`。
///
/// 传输层失败（DNS / 拒绝连接 / 超时）返回 `Err`；**HTTP 状态码不在此处判定**
/// （见 [`HttpRpc`] 的注释）。
fn post_rpc(
    url: &str,
    headers: &BTreeMap<String, String>,
    session_id: Option<&str>,
    body: &str,
    budget: Duration,
) -> Result<HttpRpc, String> {
    let agent = ureq::AgentBuilder::new()
        // 不跟随跳转：见 `pick_result` 的 300 段说明。
        .redirects(0)
        .timeout(budget)
        .build();
    let mut req = agent
        .post(url)
        .set("content-type", "application/json")
        // MCP streamable-http 要求客户端同时声明接受两种响应形态。
        .set("accept", "application/json, text/event-stream");
    for (k, v) in headers {
        req = req.set(k, v);
    }
    if let Some(sid) = session_id {
        req = req.set("mcp-session-id", sid);
    }

    // 4xx/5xx 在 ureq 里是 `Err(Status)`，但**不是**传输失败——统一摊平成 HttpRpc。
    let resp = match req.send_string(body) {
        Ok(r) => r,
        Err(ureq::Error::Status(_, r)) => r,
        Err(e) => return Err(format!("HTTP 请求失败：{e}")),
    };
    let status = resp.status();
    let content_type = resp.content_type().to_string();
    let session_id = resp
        .header("mcp-session-id")
        .map(|s| s.to_string())
        .or_else(|| session_id.map(|s| s.to_string()));
    let body = resp
        .into_string()
        .map_err(|e| format!("读取响应失败：{e}"))?;
    Ok(HttpRpc {
        status,
        content_type,
        body,
        session_id,
    })
}

/// 探测一个 **streamable-http** MCP 服务器（阻塞；调用方负责放进 `spawn_blocking`）。
///
/// `timeout` 是**整轮**总期限：5 次往返共享一个 deadline，每次只用剩余时间。
/// 否则"每请求 15s × 5"会把最坏耗时变成 75s（stdio 分支的口径是整轮，这里必须一致）。
pub fn probe_http(server: &McpServerConfig, timeout: Duration) -> Result<McpProbe, String> {
    let url = server.url.trim();
    if url.is_empty() {
        return Err(format!(
            "MCP 服务器「{}」的 transport 是 streamable-http，但配置里没有 url，无法探测",
            server.name
        ));
    }
    // URL 解析只认 http(s)（AGENTS §4.1）：`file://` / `data:` 之类既非本意，
    // 交给 ureq 拒绝也是一句难懂的错，不如在这里给出可读的拒绝。
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(format!(
            "MCP 服务器「{}」的 url 不是 http(s) 地址：{url}",
            server.name
        ));
    }

    let deadline = Instant::now() + timeout;
    let budget = |what: &str| -> Result<Duration, String> {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(format!(
                "MCP 探测超时（整轮 {}s 用尽，未到 {what}）",
                timeout.as_secs()
            ));
        }
        Ok(left)
    };

    // ① initialize
    let init_rpc = post_rpc(
        url,
        &server.headers,
        None,
        &initialize_request(1),
        budget("initialize")?,
    )?;
    let session = init_rpc.session_id.clone();
    let init = pick_result(&init_rpc, 1, "initialize")?;

    // ② initialized 通知——无 `id`，服务端只应答状态码，**不解析响应体**
    //    （去匹配 id 会永远等不到响应）。
    let ack = post_rpc(
        url,
        &server.headers,
        session.as_deref(),
        &initialized_notification(),
        budget("notifications/initialized")?,
    )?;
    if !(200..300).contains(&ack.status) {
        return Err(format!(
            "MCP 服务端拒绝 `notifications/initialized`（HTTP {}）：{}",
            ack.status,
            truncate(ack.body.trim(), 300)
        ));
    }

    // ③ 三次枚举
    let tools = {
        let rpc = post_rpc(
            url,
            &server.headers,
            session.as_deref(),
            &list_request(2, "tools/list"),
            budget("tools/list")?,
        )?;
        pick_result(&rpc, 2, "tools/list")?
    };
    let resources = {
        let rpc = post_rpc(
            url,
            &server.headers,
            session.as_deref(),
            &list_request(3, "resources/list"),
            budget("resources/list")?,
        )?;
        pick_result(&rpc, 3, "resources/list")
    };
    let templates = {
        let rpc = post_rpc(
            url,
            &server.headers,
            session.as_deref(),
            &list_request(4, "resources/templates/list"),
            budget("resources/templates/list")?,
        )?;
        pick_result(&rpc, 4, "resources/templates/list")
    };

    assemble_probe(&server.name, &init, Ok(tools), resources, templates)
}

fn run_probe(
    stdin: &mut std::process::ChildStdin,
    rx: &mpsc::Receiver<String>,
    deadline: Instant,
    server: &McpServerConfig,
) -> Result<McpProbe, String> {
    let send = |stdin: &mut std::process::ChildStdin, json: &str| -> Result<(), String> {
        stdin
            .write_all(json.as_bytes())
            .and_then(|_| stdin.write_all(b"\n"))
            .and_then(|_| stdin.flush())
            .map_err(|e| format!("写入 MCP 服务器失败：{e}"))
    };

    send(stdin, &initialize_request(1))?;
    let init = await_id(rx, 1, deadline, "initialize")?;
    send(stdin, &initialized_notification())?;

    // tools 必备：失败即整单失败，不再往下发（与既有行为一致）。
    send(stdin, &list_request(2, "tools/list"))?;
    let tools = await_id(rx, 2, deadline, "tools/list")?;

    // 两个可选能力都发出请求，判定集中交给 `assemble_probe`（一处口径）。
    send(stdin, &list_request(3, "resources/list"))?;
    let resources = await_id(rx, 3, deadline, "resources/list");
    send(stdin, &list_request(4, "resources/templates/list"))?;
    let templates = await_id(rx, 4, deadline, "resources/templates/list");

    assemble_probe(&server.name, &init, Ok(tools), resources, templates)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 握手请求必须携带**真实**协议版本与身份；缺 `params` 会被服务端直接拒。
    #[test]
    fn initialize_request_carries_protocol_and_identity() {
        let v: serde_json::Value = serde_json::from_str(&initialize_request(1)).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 1);
        assert_eq!(v["method"], "initialize");
        assert_eq!(v["params"]["protocolVersion"], LATEST_PROTOCOL_VERSION);
        assert_eq!(v["params"]["clientInfo"]["name"], "dsh-dock");
        assert!(v["params"]["capabilities"].is_object());
    }

    /// `notifications/initialized` **必须无 id**——若误带 id，我们会去等一个
    /// 永不存在的响应，整轮探测卡到超时（静默假失败）。
    #[test]
    fn initialized_notification_has_no_id() {
        let v: serde_json::Value = serde_json::from_str(&initialized_notification()).unwrap();
        assert_eq!(v["method"], "notifications/initialized");
        assert!(v.get("id").is_none(), "通知不得带 id：{v}");
        assert!(v.get("params").is_none(), "本通知无参数：{v}");
    }

    /// 信封解析：通知/日志行/非 JSON 一律 `None`（跳过继续等），
    /// 错误分支要把 code 与 message 都带出来。
    #[test]
    fn envelope_parser_skips_noise_and_surfaces_errors() {
        assert!(parse_rpc_envelope("not json at all").is_none());
        assert!(parse_rpc_envelope(r#"{"jsonrpc":"2.0","method":"notifications/x"}"#).is_none());
        // 服务端日志行（合法 JSON 但无 id）同样跳过。
        assert!(parse_rpc_envelope(r#"{"level":"info","msg":"starting"}"#).is_none());

        let (id, ok) = parse_rpc_envelope(r#"{"jsonrpc":"2.0","id":7,"result":{"a":1}}"#).unwrap();
        assert_eq!(id, 7);
        assert_eq!(ok.unwrap()["a"], 1);

        let (id, err) = parse_rpc_envelope(
            r#"{"jsonrpc":"2.0","id":8,"error":{"code":-32601,"message":"nope"}}"#,
        )
        .unwrap();
        assert_eq!(id, 8);
        let message = err.unwrap_err();
        assert!(
            message.contains("-32601") && message.contains("nope"),
            "{message}"
        );
    }

    /// 具名列表抽取：resource 缺 `name` 时回退 `uri`；缺 key 时返回空表而非 panic。
    #[test]
    fn named_list_extraction_handles_fallback_and_missing_key() {
        let result = serde_json::json!({
            "resources": [
                { "uri": "file:///a", "name": "alpha", "mimeType": "text/plain" },
                { "uri": "file:///b" }
            ]
        });
        let got = parse_named_list(&result, "resources", "name", "uri", "uri");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "alpha");
        assert_eq!(got[0].detail, "file:///a");
        // 缺 name → 回退 uri（否则界面上会出现空行）。
        assert_eq!(got[1].name, "file:///b");

        assert!(parse_named_list(&result, "tools", "name", "description", "name").is_empty());
    }

    /// `-32601` 判定：只有它才降级；其他错误必须原样上抛（不能把真故障吞成"没能力"）。
    #[test]
    fn method_not_found_is_narrowly_recognized() {
        assert!(is_method_not_found("-32601: Method not found"));
        assert!(!is_method_not_found("-32000: server exploded"));
        assert!(!is_method_not_found(
            "initialize 超时（未在期限内收到 id=1 的响应）"
        ));
    }

    /// `streamable-http` 服务器没有 command：stdio 探测必须**明确报错并指出该走 http 分支**，
    /// 不能拿空 command 去 spawn（那会得到一句无用的系统错误）。
    #[test]
    fn stdio_probe_refuses_http_only_server_with_actionable_message() {
        let server = McpServerConfig {
            name: "remote".to_string(),
            command: String::new(),
            args: Vec::new(),
            env: std::collections::BTreeMap::new(),
            disabled: false,
            transport: crate::mcp::McpTransport::StreamableHttp,
            url: "https://example.test/mcp".to_string(),
            headers: std::collections::BTreeMap::new(),
        };
        let err = probe_stdio(&server, Duration::from_millis(50)).unwrap_err();
        assert!(err.contains("remote"), "须点名是哪个服务器：{err}");
        assert!(
            err.contains("streamable-http"),
            "须指出该走 http 分支：{err}"
        );
    }

    // ---------- streamable-http 分支（2026-09-15 R3） ----------
    //
    // 本模块**不引测试依赖**（AGENTS §5）：不起真实 HTTP 服务，全部经纯函数断言。
    // 真实端点的手工验证走 `docs/executor.md` 验证清单。

    fn http_json(status: u16, body: &str) -> HttpRpc {
        HttpRpc {
            status,
            content_type: "application/json".to_string(),
            body: body.to_string(),
            session_id: None,
        }
    }

    fn sse(body: &str) -> HttpRpc {
        HttpRpc {
            status: 200,
            content_type: "text/event-stream; charset=utf-8".to_string(),
            body: body.to_string(),
            session_id: None,
        }
    }

    /// 只认 JSON 形态会把**合规的 SSE 服务端**判成"连不上"——那是假失败。
    #[test]
    fn sse_multiline_data_is_joined_not_split() {
        // 同一事件的两段 `data:`（中间还夹一条心跳注释）：SSE 规定用 `\n` 拼接。
        let payloads = rpc_payloads(
            "text/event-stream",
            "event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\n: keep-alive\ndata: \"result\":{\"ok\":true}}\n\n",
        );
        assert_eq!(
            payloads.len(),
            1,
            "多行 data 属**同一个**负载：{payloads:?}"
        );
        let (id, outcome) = parse_rpc_envelope(&payloads[0]).expect("拼好后应是合法信封");
        assert_eq!(id, 1);
        assert_eq!(outcome.unwrap()["ok"], serde_json::json!(true));
    }

    #[test]
    fn sse_heartbeat_and_comments_produce_no_payload() {
        assert!(rpc_payloads("text/event-stream", ": keep-alive\n\n").is_empty());
        assert!(rpc_payloads("text/event-stream", "").is_empty());
    }

    /// 非 2xx 也要能把信封取出来：MCP 服务端常以 `HTTP 400 + -32601` 回未知方法，
    /// 在传输层按状态码短路会让 `-32601` 降级永远走不到。
    #[test]
    fn envelope_is_picked_up_even_on_non_2xx_status() {
        let rpc = HttpRpc {
            status: 400,
            content_type: "application/json".to_string(),
            body:
                r#"{"jsonrpc":"2.0","id":3,"error":{"code":-32601,"message":"Method not found"}}"#
                    .to_string(),
            session_id: None,
        };
        let err = pick_result(&rpc, 3, "resources/list").unwrap_err();
        assert!(
            is_method_not_found(&err),
            "-32601 必须能被降级识别（而不是当成连不上）：{err}"
        );
    }

    /// 无 `id` 的通知 / 心跳行必须**跳过而不是当失败**；批量数组与 NDJSON 都要认。
    #[test]
    fn pick_result_skips_notifications_and_handles_batches() {
        // 批量（JSON-RPC array）：两条消息，取 id=2 那条。
        let batch = r#"[
            {"jsonrpc":"2.0","method":"notifications/progress"},
            {"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"t1","description":"d"}]}}
        ]"#;
        let got = pick_result(&http_json(200, batch), 2, "tools/list").unwrap();
        assert_eq!(got["tools"][0]["name"], serde_json::json!("t1"));

        // NDJSON（整段不是合法 JSON）：逐行解析，无 id 的行被跳过。
        let ndjson = "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\"}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"ok\":1}}";
        let got = pick_result(&http_json(200, ndjson), 2, "tools/list").unwrap();
        assert_eq!(got["ok"], serde_json::json!(1));
    }

    /// 真·失败（没有任何匹配 id 的负载）必须报出**状态码与响应片段**，并点明跳转。
    #[test]
    fn pick_result_reports_status_and_body_when_nothing_matches() {
        let rpc = HttpRpc {
            status: 301,
            content_type: "text/html".to_string(),
            body: "<html>Moved Permanently</html>".to_string(),
            session_id: None,
        };
        let err = pick_result(&rpc, 1, "initialize").unwrap_err();
        assert!(err.contains("301"), "须报状态码：{err}");
        assert!(err.contains("Moved"), "须带响应片段：{err}");
        assert!(
            err.contains("不跟随跳转"),
            "300 段须点明跳转未被跟随：{err}"
        );
    }

    /// **两分支共用降级口径**的正面证据：同一份 `-32601` 输入,经 http 路径装配后，
    /// 结果形状与 stdio 路径一致（tools 保留、可选能力降级为"空 + 两条说明"）。
    #[test]
    fn http_assembly_degrades_optional_capabilities_like_stdio() {
        let init = serde_json::json!({
            "protocolVersion": "2025-06-18",
            "serverInfo": { "name": "srv-from-wire" }
        });
        let tools = pick_result(
            &http_json(
                200,
                r#"{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"t1","description":"d"}]}}"#,
            ),
            2,
            "tools/list",
        )
        .unwrap();
        let not_found =
            r#"{"jsonrpc":"2.0","id":3,"error":{"code":-32601,"message":"Method not found"}}"#;
        let resources = pick_result(&http_json(200, not_found), 3, "resources/list");
        let not_found_tpl =
            r#"{"jsonrpc":"2.0","id":4,"error":{"code":-32601,"message":"Method not found"}}"#;
        let templates = pick_result(
            &http_json(200, not_found_tpl),
            4,
            "resources/templates/list",
        );

        let probe = assemble_probe("fallback", &init, Ok(tools), resources, templates).unwrap();
        // 协商后的版本与服务端自报名以**服务端返回**为准，而不是我方发送值。
        assert_eq!(probe.protocol_version, "2025-06-18");
        assert_eq!(probe.server_name, "srv-from-wire");
        assert_eq!(probe.tools.len(), 1);
        assert!(probe.resources.is_empty());
        assert!(probe.templates.is_empty());
        assert_eq!(probe.notes.len(), 2, "两条降级说明：{:#?}", probe.notes);
    }

    /// SSE 端到端（纯函数段）：握手响应走 SSE 也要能装配出结果。
    #[test]
    fn http_assembly_accepts_sse_transport() {
        let init = pick_result(
            &sse("data: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"protocolVersion\":\"2025-11-25\"}}\n\n"),
            1,
            "initialize",
        )
        .unwrap();
        let tools = pick_result(
            &sse("data: {\"jsonrpc\":\"2.0\",\"id\":2,\"result\":{\"tools\":[]}}\n\n"),
            2,
            "tools/list",
        )
        .unwrap();
        let probe = assemble_probe(
            "fallback",
            &init,
            Ok(tools),
            Err("-32601: Method not found".to_string()),
            Err("-32601: Method not found".to_string()),
        )
        .unwrap();
        assert_eq!(probe.server_name, "fallback");
        assert!(probe.tools.is_empty());
        assert_eq!(probe.notes.len(), 2);
    }

    /// tools 是必备能力：它失败就是**探测失败**，不得被降级成"没这能力"。
    #[test]
    fn http_assembly_fails_when_tools_missing() {
        let init = serde_json::json!({ "protocolVersion": "2025-11-25" });
        let err = assemble_probe(
            "srv",
            &init,
            Err("tools/list 超时".to_string()),
            Ok(serde_json::json!({ "resources": [] })),
            Ok(serde_json::json!({ "resourceTemplates": [] })),
        )
        .unwrap_err();
        assert!(err.contains("tools/list 超时"), "{err}");
    }

    /// http 分支的前置校验：缺 url / 非 http(s) 一律**明确报错**，不去发请求。
    /// （AGENTS §4.1：URL 解析只认 http(s)。）
    #[test]
    fn probe_http_refuses_bad_url_without_sending_anything() {
        let base = McpServerConfig {
            name: "remote".to_string(),
            command: String::new(),
            args: Vec::new(),
            env: std::collections::BTreeMap::new(),
            disabled: false,
            transport: crate::mcp::McpTransport::StreamableHttp,
            url: String::new(),
            headers: std::collections::BTreeMap::new(),
        };
        let err = probe_http(&base, Duration::from_millis(50)).unwrap_err();
        assert!(err.contains("没有 url"), "{err}");

        let mut file_url = base.clone();
        file_url.url = "file:///etc/passwd".to_string();
        let err = probe_http(&file_url, Duration::from_millis(50)).unwrap_err();
        assert!(err.contains("不是 http(s) 地址"), "{err}");
    }
}
