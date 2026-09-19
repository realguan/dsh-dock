//! mcp.rs —— Profile 的 MCP 服务器结构化管理（4.7）。
//!
//! 契约与防坑准则（roadmap §1 & 4.7 审核意见）：
//! 1. Cordis Patch 对 `config` 键是整体替换（Replace，无深合并）；
//! 2. 增删改单个 MCP Server 时，必须先在内存中构建包含全部 servers 的完整 `config.mcpServers` 对象，
//!    再整行更新写回 `cordis.patch.yml`；
//! 3. 保持原子写入与原有其它插件 patch 顺序不变。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const MCP_CLIENT_PKG: &str = "@deepseek-ai/dsh-mcp-client";
const PROFILE_PATCH_FILENAME: &str = "cordis.patch.yml";

/// MCP 条目所在的 patch 层 —— 决定它的**生效范围**。
///
/// dsh 的 patch 应用顺序（`dsh-app-boot/lib/index.js:1005 readProfilePatches`，实查）：
/// 各 bundle 层 → **profile 层**（`profiles/<名>/cordis.patch.yml`）→
/// **home 级全局层**（`$DSH_HOME/cordis.patch.yml`）→ `--patch` overlays。
///
/// 两条用户层因此是**两种生效范围**，本模块必须同时建模，否则：
/// - 只读 profile 层 ⇒ 全局层定义的 MCP 在界面上**不可见**（用户以为没配），也删不掉；
/// - 只写 profile 层 ⇒ 无法表达"对所有 profile 生效"；
/// - 两层同名**不是覆盖**：两行都会插入，而上游 `serverName` 是**加载期预留**
///   （`dsh-mcp-client/src/index.ts:160-176`）——先加载的占住名字、后加载的那条
///   **实例化失败**（`serverName "x" is already in use`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpScope {
    /// `$DSH_HOME/profiles/<profile>/cordis.patch.yml`：**仅该 profile** 生效。
    #[default]
    Profile,
    /// `$DSH_HOME/cordis.patch.yml`：dsh 的 home 级用户层，**对所有 profile** 生效。
    Global,
}

impl McpScope {
    /// 该层在 `$DSH_HOME` 下的相对路径（客体侧同形）。
    pub fn patch_rel_path(&self, profile: &str) -> String {
        match self {
            McpScope::Global => PROFILE_PATCH_FILENAME.to_string(),
            McpScope::Profile => format!("profiles/{profile}/{PROFILE_PATCH_FILENAME}"),
        }
    }

    /// 宿主侧绝对路径。
    pub fn patch_path(&self, home: &Path, profile: &str) -> PathBuf {
        match self {
            McpScope::Global => home.join(PROFILE_PATCH_FILENAME),
            McpScope::Profile => home
                .join("profiles")
                .join(profile)
                .join(PROFILE_PATCH_FILENAME),
        }
    }
}

/// MCP 服务器配置项（前后端交互契约）
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub disabled: bool,
    /// 传输方式（2026-09-15，ADR-0022 前置）：上游 `@deepseek-ai/dsh-mcp-client`
    /// 只支持 `stdio` 与 `streamable-http` 两种（无 `sse` 字面量）。
    /// 缺省视为 `stdio`——兼容既有条目形状，**不改动 stdio 的既有写法**。
    #[serde(default)]
    pub transport: McpTransport,
    /// `streamable-http` 的 MCP 端点 URL（`stdio` 恒空）。
    #[serde(default)]
    pub url: String,
    /// `streamable-http` 的附加请求头（`stdio` 恒空）。
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// stdio 子进程工作目录（2026-09-18 补建模）。
    ///
    /// **为什么必须建模**：上游 `StdioConfig.cwd` 缺省是**空串**，而"绝对路径 node +
    /// 绝对路径入口脚本"这类**完全不依赖 PATH** 的写法（本机 tempad-dev 即如此，
    /// 见 `$DSH_HOME/cordis.patch.yml` 的实测注记）依赖显式 `cwd`。此前本模块不认识
    /// 这个键 ⇒ 在界面里编辑一次就把用户手写的 `cwd` **静默删掉**、MCP 从此起不来。
    #[serde(default)]
    pub cwd: String,
    /// 本条目所在的 patch 层（= 生效范围）。读取时由所在层填充，写入时据此选层。
    #[serde(default)]
    pub scope: McpScope,
    /// 条目在 patch 里的**行 id**（insert 项的 `id`；旧版字典形态为所在行 id）。
    ///
    /// 前端装配状态徽标按它精确匹配运行态快照的 `entryId`（`include:<行id>`）。
    /// 此前按 `mcp-<名>` 前缀剥名字 ⇒ 用户手写行 id 的行（本轮刻意保留不改写）
    /// 徽标**静默缺失**，而那种行恰恰最需要看装配状态。2026-09-18 补。
    #[serde(default)]
    pub row_id: String,
    /// `true` = 条目含 `!!js <表达式>` 之类的**标签值**（上游文档用它传 secret 引用，
    /// 如 `TOKEN: !!js process.env.GITHUB_TOKEN`）。表单展示的是它的**展平字符串**。
    /// **判据只能来自原文**：serde_yaml 0.9.34 在 `from_str::<Value>` 时会把未知标签
    /// **静默丢掉**（实测：`TOKEN: !!js process.env.X` → `String("process.env.X")`，
    /// `Value::Tagged` 根本不出现）⇒ 任何"从 Value 判定再合并回去"的路子都保不住它。
    /// 所以本字段的用处是**把那条静默破坏换成显式拒绝**：`expr` 为真的行，结构化保存
    /// 会被拒（错误信息指回所在层的 `cordis.patch.yml` 文件）；只有"仅启停"这一种改动经**文本级**改写放行
    /// （见 [`toggle_disabled_in_text`]）。
    #[serde(default)]
    pub expr: bool,
}

/// MCP 传输方式。字面量与上游 union 一致（kebab-case：`stdio` / `streamable-http`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpTransport {
    #[default]
    Stdio,
    StreamableHttp,
}

/// 取 YAML 标量的**展平字符串**（表单视图）。数字/布尔也给出字符串形态，保证
/// "读得出就显示得出"。非字符串容器（mapping/sequence）返回 `None`。
fn flat_scalar(v: &serde_yaml::Value) -> Option<String> {
    match v {
        serde_yaml::Value::String(s) => Some(s.clone()),
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        serde_yaml::Value::Tagged(t) => flat_scalar(&t.value),
        _ => None,
    }
}

/// 取 mapping 里某个键的字符串值（非 mapping / 键不存在 / 非字符串都 `None`）。
fn ystr_at<'a>(v: &'a serde_yaml::Value, key: &str) -> Option<&'a str> {
    v.as_mapping()?
        .get(serde_yaml::Value::String(key.into()))?
        .as_str()
}

/// 取 entry 的 `config` mapping。
fn ycfg_of(v: &serde_yaml::Value) -> Option<&serde_yaml::Mapping> {
    v.as_mapping()?
        .get(serde_yaml::Value::String("config".into()))?
        .as_mapping()
}

/// 一行 YAML 是否把值写成**带标签的标量**（`TOKEN: !!js process.env.X`、
/// `- !!js expr`）。注释行不算。
///
/// **只做"有 tag"的粗判，且刻意偏向多判**：行尾注释不剔除（`key: v # !!x` 会误判），
/// 因为多判的代价只是"这一行不让结构化编辑"，漏判的代价是"编辑即把 secret 引用
/// 展平成字面量"——两者不对称。
fn yline_has_tag(line: &str) -> bool {
    let t = line.trim_start();
    if t.starts_with('#') {
        return false;
    }
    match t.find(':') {
        Some(i) => t[i + 1..].trim_start().starts_with('!'),
        None => t.strip_prefix("- ").unwrap_or(t).starts_with('!'),
    }
}

/// 行的**键名**（可带 `- ` 前缀）；无 `:` 的行返回 `None`。
fn yline_key(line: &str) -> Option<&str> {
    let t = line
        .trim_start()
        .strip_prefix("- ")
        .unwrap_or(line.trim_start());
    let key = t.split_once(':')?.0.trim();
    (!key.is_empty() && !key.contains(' ')).then_some(key)
}

/// 行的键值（去引号；行尾 ` #` 注释剔除）。
fn yline_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    if yline_key(line) != Some(key) {
        return None;
    }
    let t = line
        .trim_start()
        .strip_prefix("- ")
        .unwrap_or(line.trim_start());
    let v = t.split_once(':')?.1.trim();
    let v = match v.find(" #") {
        Some(i) => v[..i].trim_end(),
        None => v,
    };
    let v = v.trim_matches(|c| c == '\'' || c == '"');
    (!v.is_empty()).then_some(v)
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// 定位某个 **patch 行**的原文块：`(起始行号, 结束行号, 同级键缩进)`（区间左闭右开）。
///
/// 为什么按原文定位而不是按解析后的 Value：见 [`McpServerConfig::expr`]——标签在
/// Value 里根本不存在，而"这一行能不能结构化重写""这一行的 `disabled` 在哪"都必须
/// 精确到行。块边界 = 下一条**同级或更浅**的非空行。
fn locate_row_block(lines: &[&str], row_id: &str) -> Option<(usize, usize, usize)> {
    if row_id.is_empty() {
        return None;
    }
    let start = lines
        .iter()
        .position(|l| yline_value(l, "id") == Some(row_id))?;
    let indent = indent_of(lines[start]);
    let dashed = lines[start].trim_start().starts_with("- ");
    // `- id: x`：同级键缩进 = dash 列 + 2，块在遇到同列或更浅的行时结束。
    // `  id: x`（dash 在上一行）：同级键缩进 = 本行缩进，块在更浅处结束。
    let (boundary, key_indent) = if dashed {
        (indent, indent + 2)
    } else {
        (indent.saturating_sub(2), indent)
    };
    let end = lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find(|(_, l)| !l.trim().is_empty() && indent_of(l) <= boundary)
        .map(|(i, _)| i)
        .unwrap_or(lines.len());
    Some((start, end, key_indent))
}

/// 该行所属**顶层条目**的原文块（`[start, end)`）。
///
/// 为什么范围要放大到条目而不是行：`PatchFile` 的重序列化粒度是**顶层条目**，
/// 改一条 `- insert:` 里的任意一行，会把该条目里**所有**兄弟行一起重新序列化 ⇒
/// 兄弟行的标签值同样被展平。判据放大到条目才与写入的实际风险面一致。
/// 边界口径与 `plugins::top_level_item_starts` 同形：第 0 列的 `- ` / `-` 行。
fn entry_block_of(lines: &[&str], row_start: usize, row_end: usize) -> (usize, usize) {
    let is_item_head = |l: &str| -> bool {
        let t = l.trim_end_matches(['\n', '\r']);
        t == "-" || t.starts_with("- ")
    };
    let start = (0..=row_start)
        .rev()
        .find(|&i| is_item_head(lines[i]))
        .unwrap_or(row_start);
    let end = (row_end..lines.len())
        .find(|&i| is_item_head(lines[i]))
        .unwrap_or(lines.len());
    (start, end)
}

/// 该行（按所属顶层条目）的原文里是否出现标签 = `expr`。
///
/// **定位不到行块时按整文件判**（保守方向）：宁可多标一条、让用户改用手工编辑原文，
/// 也不能漏标——漏标就是"点一次保存把 secret 引用静默展平"。
fn row_block_has_tag(lines: &[&str], row_id: &str) -> bool {
    match locate_row_block(lines, row_id) {
        Some((start, end, _)) => {
            let (es, ee) = entry_block_of(lines, start, end);
            lines[es..ee].iter().any(|l| yline_has_tag(l))
        }
        None => lines.iter().any(|l| yline_has_tag(l)),
    }
}

/// **文本级**启停：只在目标行块内增 / 改 / 删一行 `disabled:`，其余字节不动。
///
/// 这是 `expr` 行唯一放行的改动——重序列化整条必然展平 `!!js` 引用，而启停
/// 只需要动一个键，按行走反而比按节点走更安全。
/// 返回 `None` = 原文里定位不到这一行（宁可报错，也不猜它是哪一条）。
fn toggle_disabled_in_text(content: &str, row_id: &str, disabled: bool) -> Option<String> {
    let borrowed: Vec<&str> = content.split('\n').collect();
    let (start, end, key_indent) = locate_row_block(&borrowed, row_id)?;
    let pad = " ".repeat(key_indent);
    let hit = (start + 1..end).find(|i| {
        let l = borrowed[*i].trim_end_matches('\r');
        indent_of(l) == key_indent && yline_key(l) == Some("disabled")
    });
    let mut out: Vec<String> = borrowed.iter().map(|l| l.to_string()).collect();
    match (disabled, hit) {
        // 已是目标态：原样返回（调用方据此零写入）
        (true, Some(i))
            if yline_value(borrowed[i].trim_end_matches('\r'), "disabled") == Some("true") =>
        {
            return Some(content.to_string())
        }
        (false, None) => return Some(content.to_string()),
        (true, Some(i)) => {
            // 保留行尾注释（注释是用户资产，启停不该顺手把它套掉）
            let tail = match borrowed[i].trim_end_matches('\r').find(" #") {
                Some(c) => borrowed[i].trim_end_matches('\r')[c..].to_string(),
                None => String::new(),
            };
            out[i] = format!("{pad}disabled: true{tail}");
        }
        (true, None) => out.insert(start + 1, format!("{pad}disabled: true")),
        (false, Some(i)) => {
            out.remove(i);
        }
    }
    Some(out.join("\n"))
}

/// 除 `disabled`（和层/行身份）之外，**建模字段**是否完全一致。
///
/// `expr` 行的放行判据：只有"启停"这一种改动值得为它开文本级后门。
fn modeled_shape_equal(a: &McpServerConfig, b: &McpServerConfig) -> bool {
    a.name == b.name
        && a.command == b.command
        && a.args == b.args
        && a.env == b.env
        && a.transport == b.transport
        && a.url == b.url
        && a.headers == b.headers
        && a.cwd == b.cwd
}

/// 从单个 config mapping 和 disabled 标志中解析出 McpServerConfig。
///
/// `expr` 由调用方按**原文**判定（见 [`row_block_has_tag`]）——Value 里没有标签可言。
fn parse_single_mcp_config(
    name: String,
    row_id: String,
    cfg_val: &serde_yaml::Mapping,
    item_disabled: bool,
    scope: McpScope,
    expr: bool,
) -> McpServerConfig {
    let get = |key: &str| cfg_val.get(serde_yaml::Value::String(key.into()));
    let args = get("args")
        .and_then(|a| a.as_sequence())
        .map(|arr| arr.iter().filter_map(flat_scalar).collect::<Vec<_>>())
        .unwrap_or_default();
    let mut env_map = BTreeMap::new();
    if let Some(env_obj) = get("env").and_then(|e| e.as_mapping()) {
        for (k, v) in env_obj {
            if let (Some(k), Some(vs)) = (k.as_str(), flat_scalar(v)) {
                env_map.insert(k.to_string(), vs);
            }
        }
    }
    let disabled = item_disabled || get("disabled").and_then(|d| d.as_bool()).unwrap_or(false);

    let transport = match get("transport").and_then(|t| t.as_str()).unwrap_or("stdio") {
        "streamable-http" => McpTransport::StreamableHttp,
        _ => McpTransport::Stdio,
    };
    let cmd = if transport == McpTransport::StreamableHttp {
        String::new()
    } else {
        get("command")
            .and_then(flat_scalar)
            .unwrap_or_else(|| "pnpm".to_string())
    };
    let url = get("url").and_then(flat_scalar).unwrap_or_default();
    let mut headers = BTreeMap::new();
    if let Some(h) = get("headers").and_then(|h| h.as_mapping()) {
        for (k, v) in h {
            if let (Some(k), Some(vs)) = (k.as_str(), flat_scalar(v)) {
                headers.insert(k.to_string(), vs);
            }
        }
    }

    McpServerConfig {
        name,
        command: cmd,
        args,
        env: env_map,
        disabled,
        transport,
        url,
        headers,
        cwd: get("cwd").and_then(flat_scalar).unwrap_or_default(),
        scope,
        row_id,
        expr,
    }
}

/// 从 cordis.patch.yml 文本中解析 MCP 服务器配置列表，并标注**来自哪一层**。
///
/// **故意没有"默认层"重载**（2026-09-18）：层标注是"生效范围"的事实来源，
/// 留一个只有 `content` 的便捷入口，就等于让调用方可以悄悄把它当 profile 层
/// —— 本次改动的全部意义正是不让层被猜。纯函数，宿主/客体共用。
///
/// **同层重复的 serverName 不去重**（2026-09-18 二修）：上游 `serverName` 是
/// 加载期预留，同层重复与跨层重复的后果**完全相同**（后加载的实例化失败），
/// 去重会把这种坏状态藏起来，且删除会一次带走两条而界面只看得见一条。
///
/// **兼容双形态**：
/// 1. DSH 标准形态（2026-09-18 对齐）：`- insert: [ { id: "mcp-<name>", name: "@deepseek-ai/dsh-mcp-client", config: { serverName, ... } } ]`
/// 2. 遗留兼容形态：`- id: mcp, package: "@deepseek-ai/dsh-mcp-client", config: { mcpServers: { ... } }`
pub fn parse_mcp_servers_in_layer(
    content: &str,
    scope: McpScope,
) -> Result<Vec<McpServerConfig>, String> {
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let entries: Vec<serde_yaml::Value> = serde_yaml::from_str(content)
        .map_err(|e| format!("解析 cordis.patch.yml YAML 失败：{e}"))?;

    let mut result = Vec::new();
    // `expr` 只能按原文判（见 McpServerConfig::expr 的实测说明）
    let lines: Vec<&str> = content.split('\n').collect();

    let check_is_mcp = |val: &serde_yaml::Value| -> bool {
        ystr_at(val, "name") == Some(MCP_CLIENT_PKG)
            || ystr_at(val, "package") == Some(MCP_CLIENT_PKG)
    };
    // 定位锚 = **行 id**（`row_block_has_tag` 找不到时退回整文件，保守方向）。
    // 旧版 mcpServers 字典里的多台服务器共享一个行 id ⇒ 整段共享同一个 expr 结论，
    // 这方向是保守的（多标一条只是不让结构化编辑）。
    let expr_of = |row_id: &str| row_block_has_tag(&lines, row_id);

    for entry in &entries {
        // 分支 1：标准 insert 数组形态（- insert: [ ... ]）
        let inserts = entry
            .as_mapping()
            .and_then(|m| m.get(serde_yaml::Value::String("insert".into())))
            .and_then(|i| i.as_sequence());
        if let Some(inserts) = inserts {
            for item in inserts {
                if !check_is_mcp(item) {
                    continue;
                }
                let Some(cfg) = ycfg_of(item) else { continue };
                let item_disabled = item
                    .as_mapping()
                    .and_then(|m| m.get(serde_yaml::Value::String("disabled".into())))
                    .and_then(|d| d.as_bool())
                    .unwrap_or(false);
                let row_id = ystr_at(item, "id").unwrap_or_default().to_string();
                let server_name = cfg
                    .get(serde_yaml::Value::String("serverName".into()))
                    .and_then(flat_scalar)
                    .or_else(|| {
                        // 无 serverName 的行按 id 推导（`mcp-<名>` 是壳写入口径）
                        let id = row_id.strip_prefix("mcp-").unwrap_or(&row_id);
                        (!id.trim().is_empty()).then(|| id.to_string())
                    });
                if let Some(name) = server_name {
                    if !name.trim().is_empty() {
                        let expr = expr_of(&row_id);
                        result.push(parse_single_mcp_config(
                            name,
                            row_id,
                            cfg,
                            item_disabled,
                            scope,
                            expr,
                        ));
                    }
                }
            }
            continue;
        }

        // 分支 2：顶层单个 MCP 插件 entry 或旧版 mcpServers 字典
        if check_is_mcp(entry) {
            let Some(cfg) = ycfg_of(entry) else { continue };
            let item_disabled = entry
                .as_mapping()
                .and_then(|m| m.get(serde_yaml::Value::String("disabled".into())))
                .and_then(|d| d.as_bool())
                .unwrap_or(false);
            let row_id = ystr_at(entry, "id").unwrap_or_default().to_string();
            // 2a. 单 server 格式（带 serverName）
            if let Some(server_name) = cfg
                .get(serde_yaml::Value::String("serverName".into()))
                .and_then(flat_scalar)
            {
                if !server_name.trim().is_empty() {
                    let expr = expr_of(&row_id);
                    result.push(parse_single_mcp_config(
                        server_name,
                        row_id,
                        cfg,
                        item_disabled,
                        scope,
                        expr,
                    ));
                    continue;
                }
            }
            // 2b. 旧版 dock 的 mcpServers 字典（整行是**一条** patch 行；字典值不含
            //     tagged——上游本就拒收该形态，仅为"看得见、删得掉"保留读路径）
            if let Some(servers) = cfg
                .get(serde_yaml::Value::String("mcpServers".into()))
                .and_then(|s| s.as_mapping())
            {
                for (name, srv_val) in servers {
                    let Some(name) = name.as_str().map(str::to_string) else {
                        continue;
                    };
                    if name.trim().is_empty() {
                        continue;
                    }
                    if let Some(m) = srv_val.as_mapping() {
                        let expr = expr_of(&row_id);
                        result.push(parse_single_mcp_config(
                            name,
                            row_id.clone(),
                            m,
                            item_disabled,
                            scope,
                            expr,
                        ));
                    }
                }
            }
        }
    }

    Ok(result)
}

/// 读取指定 profile 可见的**全部** MCP 服务器配置（**两个用户层都读**）。
///
/// 顺序 = dsh 的 patch 应用顺序：**profile 层在前、home 级全局层在后**。
/// 同名条目两层都有时**不是覆盖**：两行都会插入，先加载的（profile 层）占住
/// `serverName`，后加载的（全局层）实例化失败——前端据此判"跨层撞名"并告警。
pub fn list_mcp_servers(home: &Path, profile: &str) -> Result<Vec<McpServerConfig>, String> {
    crate::profiles::validate_profile_name(profile)?;
    let mut out = Vec::new();
    for scope in [McpScope::Profile, McpScope::Global] {
        out.extend(read_layer(home, profile, scope)?);
    }
    Ok(out)
}

/// 读单层（文件不存在 = 空表）。
fn read_layer(home: &Path, profile: &str, scope: McpScope) -> Result<Vec<McpServerConfig>, String> {
    let patch_path = scope.patch_path(home, profile);
    if !patch_path.is_file() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&patch_path)
        .map_err(|e| format!("读取 {} 失败：{e}", patch_path.display()))?;
    parse_mcp_servers_in_layer(&content, scope)
}

/// streamable-http 的端点 URL 校验（宿主 / 客体共用，因为在同一条保存链路上）。
///
/// **为什么必须判**：上游把 `url` 直接交给 fetch——空串或 `file://` 这类地址写进
/// patch 后**行照样保存成功**，但插件在加载期才失败 ⇒ 界面显示"已启用"、模型侧
/// 永远拿不到工具（本项目最忌的静默失败面）。口径与全仓 URL 解析一致：只认
/// `http://` / `https://`。
fn validate_server_url(url: &str) -> Result<(), String> {
    let u = url.trim();
    if u.is_empty() {
        return Err("streamable-http 传输必须填 MCP 端点 URL".to_string());
    }
    if !(u.starts_with("http://") || u.starts_with("https://")) {
        return Err(format!(
            "MCP 端点 URL「{u}」必须以 http:// 或 https:// 开头（其它写法上游连不上）"
        ));
    }
    Ok(())
}

/// 校验 MCP 服务器名（宿主 / 客体共用）。
///
/// **判据必须与上游同源**：`dsh-mcp-client` 的 `SERVER_NAME_PATTERN` 是
/// `/^[A-Za-z0-9_-]{1,32}$/`（`mcp-client/src/index.ts:40` + `Config.serverName`）。
/// 名称不合规时**行照样能写进 patch**，但插件在加载期就被 schema 拒掉 ⇒
/// 界面显示"已启用"而模型侧永远拿不到 `mcp__<name>__*` 工具——
/// 正是"保存成功但永不生效"的静默失败面。故在这里按上游口径**当场拒**。
fn validate_server_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("MCP 服务器名称不能为空".to_string());
    }
    if name.chars().count() > 32 {
        return Err(format!("MCP 服务器名称「{name}」过长（上游上限 32 字符）"));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(format!(
            "MCP 服务器名称「{name}」含非法字符：上游只接受 ASCII 字母、数字、下划线、连字符\
             （名称会进工具名 mcp__<名称>__<工具>，中文/点号/空格会让插件加载被拒）"
        ));
    }
    Ok(())
}

/// 判断某个 entry（或 insert 数组中的 item）是否与目标 MCP 服务器匹配
fn is_matching_mcp_entry(item: &serde_yaml::Value, server_name: &str) -> bool {
    let Some(m) = item.as_mapping() else {
        return false;
    };
    let pkg_key = serde_yaml::Value::String("package".into());
    let name_key = serde_yaml::Value::String("name".into());
    let is_mcp = m.get(&pkg_key).and_then(|p| p.as_str()) == Some(MCP_CLIENT_PKG)
        || m.get(&name_key).and_then(|p| p.as_str()) == Some(MCP_CLIENT_PKG);
    if !is_mcp {
        return false;
    }
    let id_key = serde_yaml::Value::String("id".into());
    if let Some(id) = m.get(&id_key).and_then(|i| i.as_str()) {
        if id == format!("mcp-{}", server_name) || id == server_name {
            return true;
        }
    }
    let cfg_key = serde_yaml::Value::String("config".into());
    if let Some(cfg) = m.get(&cfg_key).and_then(|c| c.as_mapping()) {
        let srv_name_key = serde_yaml::Value::String("serverName".into());
        if cfg.get(&srv_name_key).and_then(|s| s.as_str()) == Some(server_name) {
            return true;
        }
    }
    false
}

/// 将 McpServerConfig 的配置部分序列化为 YAML Mapping（供 config 键使用）。
/// `serde_yaml::Mapping` 基于 IndexMap，插入序保真。
fn server_config_to_yaml(server: &McpServerConfig) -> serde_yaml::Value {
    let mut m = serde_yaml::Mapping::new();
    m.insert(
        serde_yaml::Value::String("serverName".into()),
        serde_yaml::Value::String(server.name.clone()),
    );
    if server.transport == McpTransport::StreamableHttp {
        // http 传输（2026-09-15，ADR-0022 前置）：写 transport/url/headers，不写 command/args
        m.insert(
            serde_yaml::Value::String("transport".into()),
            serde_yaml::Value::String("streamable-http".into()),
        );
        m.insert(
            serde_yaml::Value::String("url".into()),
            serde_yaml::Value::String(server.url.clone()),
        );
        if !server.headers.is_empty() {
            let mut headers = serde_yaml::Mapping::new();
            for (k, v) in &server.headers {
                headers.insert(
                    serde_yaml::Value::String(k.clone()),
                    serde_yaml::Value::String(v.clone()),
                );
            }
            m.insert(
                serde_yaml::Value::String("headers".into()),
                serde_yaml::Value::Mapping(headers),
            );
        }
        return serde_yaml::Value::Mapping(m);
    }
    m.insert(
        serde_yaml::Value::String("transport".into()),
        serde_yaml::Value::String("stdio".into()),
    );
    m.insert(
        serde_yaml::Value::String("command".into()),
        serde_yaml::Value::String(server.command.clone()),
    );
    m.insert(
        serde_yaml::Value::String("args".into()),
        serde_yaml::Value::Sequence(
            server
                .args
                .iter()
                .map(|a| serde_yaml::Value::String(a.clone()))
                .collect(),
        ),
    );
    if !server.env.is_empty() {
        let mut env = serde_yaml::Mapping::new();
        for (k, v) in &server.env {
            env.insert(
                serde_yaml::Value::String(k.clone()),
                serde_yaml::Value::String(v.clone()),
            );
        }
        m.insert(
            serde_yaml::Value::String("env".into()),
            serde_yaml::Value::Mapping(env),
        );
    }
    // `cwd`：**只在非空时写**。上游缺省是空串，显式写空串与不写等价；
    // 而"绝对路径 node + 绝对路径入口脚本"这类不依赖 PATH 的写法必须靠它。
    if !server.cwd.trim().is_empty() {
        m.insert(
            serde_yaml::Value::String("cwd".into()),
            serde_yaml::Value::String(server.cwd.clone()),
        );
    }
    serde_yaml::Value::Mapping(m)
}

/// 兼容测试与老 helper：返回 (server_name, config_value)
#[cfg(test)]
fn server_to_yaml(server: McpServerConfig) -> (String, serde_yaml::Value) {
    let name = server.name.clone();
    (name, server_config_to_yaml(&server))
}

/// 将单个 McpServerConfig 序列化为 DSH 标准的 insert 插件项（Mapping）。
/// 结构对齐上游 `@deepseek-ai/dsh-mcp-client` 规范：
/// ```yaml
/// id: mcp-<serverName>
/// name: '@deepseek-ai/dsh-mcp-client'
/// disabled: true # 若停用
/// config:
///   serverName: <serverName>
///   transport: stdio # 或 streamable-http
///   command: ...
///   args: [...]
///   env: { ... }
///   # 或 url, headers
/// ```
fn server_to_insert_entry(server: &McpServerConfig) -> serde_yaml::Value {
    let mut entry = serde_yaml::Mapping::new();
    entry.insert(
        serde_yaml::Value::String("id".into()),
        serde_yaml::Value::String(format!("mcp-{}", server.name)),
    );
    entry.insert(
        serde_yaml::Value::String("name".into()),
        serde_yaml::Value::String(MCP_CLIENT_PKG.into()),
    );
    if server.disabled {
        entry.insert(
            serde_yaml::Value::String("disabled".into()),
            serde_yaml::Value::Bool(true),
        );
    }
    entry.insert(
        serde_yaml::Value::String("config".into()),
        server_config_to_yaml(server),
    );
    serde_yaml::Value::Mapping(entry)
}

/// 合并保真：`fresh` 是重算出的值，`old` 是文件里的原节点。
///
/// **为什么必须有**（2026-09-18 二修）：表单值是字符串视图——`!!js
/// process.env.GITHUB_TOKEN` 展平成 `"process.env.GITHUB_TOKEN"` 显示。若合并时
/// 一律写回字符串，用户编辑**任何一个**字段都会把没碰过的 secret 引用**静默
/// 展平成字面量**（token 从此失效）——正是本轮要根除的事故类。规则：
/// 展平值与原节点一致 ⇒ 判定"用户没改它"，保留原节点（tag、数字、布尔原样存活）；
/// 不一致 ⇒ 用户显式改过，写新值（该项退化为字面量是其所愿，条目的 `expr`
/// 徽标已提示过风险）。
fn merge_preserved(
    old: Option<&serde_yaml::Value>,
    fresh: &serde_yaml::Value,
) -> serde_yaml::Value {
    let Some(old) = old else {
        return fresh.clone();
    };
    match (old, fresh) {
        (serde_yaml::Value::Mapping(os), serde_yaml::Value::Mapping(fs)) => {
            let mut out = serde_yaml::Mapping::new();
            for (k, v) in fs {
                out.insert(k.clone(), merge_preserved(os.get(k), v));
            }
            serde_yaml::Value::Mapping(out)
        }
        (serde_yaml::Value::Sequence(os), serde_yaml::Value::Sequence(fs))
            if os.len() == fs.len() =>
        {
            serde_yaml::Value::Sequence(
                os.iter()
                    .zip(fs.iter())
                    .map(|(o, f)| merge_preserved(Some(o), f))
                    .collect(),
            )
        }
        _ => match flat_scalar(old) {
            Some(s) if fresh.as_str() == Some(s.as_str()) => old.clone(),
            _ => fresh.clone(),
        },
    }
}

/// 把 `server` 的**已建模字段**合并进既有 insert 项（就地更新）。
///
/// **为什么不整项替换**（2026-09-18 修）：上游 `Config` 里还有本模块不建模的键
/// ——`failOnStartupError`、`toolCallTimeoutMs`、`maxInstructionBytes`、`reconnect`——
/// 而 `cwd` 这一条本机唯一的可用写法也依赖显式赋值。整项替换会把它们**静默删掉**：
/// 用户在界面上点一次"编辑→保存"，MCP 就再也起不来（且 patch 文件看不出被删了什么）。
///
/// 字段处理口径（与 `server_config_to_yaml` 的分支保持一致）：
/// - 恒写：`name` / `config`；`id` **只在缺失时补**（见下）
/// - 停用才写 `disabled: true`，启用则**删掉**该键（不留 `disabled: false` 噪音）
/// - `config` 内只动本传输方式该有的键，另一分支的键**删掉**（切传输不留残键）
/// - 本分支的键逐个经 [`merge_preserved`] 合并（tagged/数字/布尔不被展平覆盖）
///
/// **为什么保留既有 `id` 而不是改写**：行 id 是用户在 patch 里的行身份，**别的层可以
/// 按它定位**（后续补丁层按 id 覆盖/停用某行是常规手法，见 `applyEntryPatches`）。
/// 把用户手写的 `id: my-mcp` 擅自改写成 `mcp-<名>`，等于让那些补丁再也匹配不上——
/// 又是"壳静默改坏用户配置"。壳只在**新建**条目时自造 id。
fn merge_insert_entry(item: &mut serde_yaml::Value, server: &McpServerConfig) {
    let Some(m) = item.as_mapping_mut() else {
        *item = server_to_insert_entry(server);
        return;
    };
    let id_key = serde_yaml::Value::String("id".into());
    if !m.contains_key(&id_key) {
        m.insert(
            id_key,
            serde_yaml::Value::String(format!("mcp-{}", server.name)),
        );
    }
    m.insert(
        serde_yaml::Value::String("name".into()),
        serde_yaml::Value::String(MCP_CLIENT_PKG.into()),
    );
    let disabled_key = serde_yaml::Value::String("disabled".into());
    if server.disabled {
        m.insert(disabled_key, serde_yaml::Value::Bool(true));
    } else {
        m.remove(&disabled_key);
    }

    let cfg_key = serde_yaml::Value::String("config".into());
    if !m.get(&cfg_key).map(|c| c.is_mapping()).unwrap_or(false) {
        m.insert(
            cfg_key.clone(),
            serde_yaml::Value::Mapping(Default::default()),
        );
    }
    let Some(cfg) = m.get_mut(&cfg_key).and_then(|c| c.as_mapping_mut()) else {
        return;
    };
    // 先把本传输用不到的键清掉（切传输不留残键），再把本分支的键**经保真合并**写入。
    let owned: &[&str] = if server.transport == McpTransport::StreamableHttp {
        &["command", "args", "env", "cwd"]
    } else {
        &["url", "headers"]
    };
    for key in owned {
        cfg.remove(serde_yaml::Value::String((*key).into()));
    }
    let fresh = server_config_to_yaml(server);
    if let Some(fresh_map) = fresh.as_mapping() {
        let old_cfg = cfg.clone();
        for (k, v) in fresh_map {
            cfg.insert(k.clone(), merge_preserved(old_cfg.get(k), v));
        }
    }
}

/// upsert 内核（宿主写文件 / 客体文本变换共用）。
///
/// 契约（2026-09-18 对齐 DSH 上游与 Cordis patch 规范）：
/// 1. MCP 插件 `@deepseek-ai/dsh-mcp-client` 每个实例仅连一台 MCP 服务器；
/// 2. 在 cordis.patch.yml 中通过全局 `- insert: [...]` 挂载，每个服务拥有独立的 `id: mcp-<serverName>`；
/// 3. 若 patch 中存在旧版 dock 写入的 `mcpServers` 字典，先从中清除同名旧项，实现平滑升格迁移；
/// 4. 优先**就地合并**已有 insert 列表中的同名条目（保未建模键，见 [`merge_insert_entry`]）；
///    若无则追加到现存的无 id 顶层 insert 块，或新建顶层 insert 块；
/// 5. 走 `for_each_entry_mut`，仅改动的条目重新序列化，其余条目（含行间注释）逐字节回填。
fn upsert_into_patch(
    patch: &mut crate::plugins::PatchFile,
    server: McpServerConfig,
) -> Result<(), String> {
    let insert_key = serde_yaml::Value::String("insert".into());
    let pkg_key = serde_yaml::Value::String("package".into());
    let cfg_key = serde_yaml::Value::String("config".into());
    let servers_key = serde_yaml::Value::String("mcpServers".into());
    let srv_name_yaml = serde_yaml::Value::String(server.name.clone());
    let new_entry_val = server_to_insert_entry(&server);

    // 1. 先从旧格式 mcpServers 字典中清理可能存在的同名 server（防止残留双源配置）。
    //    迁空后**不留空壳**：空 `mcpServers: {}` 行会被上游 schema 拒（无 serverName），
    //    让用户每次启动都看到一行与本轮无关的加载 warning（2026-09-18 二修）。
    let mut migrated = false;
    patch.for_each_entry_mut(|_, entry| {
        let Some(m) = entry.as_mapping_mut() else {
            return false;
        };
        if m.get(&pkg_key).and_then(|p| p.as_str()) == Some(MCP_CLIENT_PKG) {
            if let Some(cfg) = m.get_mut(&cfg_key).and_then(|c| c.as_mapping_mut()) {
                let mut removed_any = false;
                let mut emptied = false;
                if let Some(servers) = cfg.get_mut(&servers_key).and_then(|s| s.as_mapping_mut()) {
                    if servers.remove(&srv_name_yaml).is_some() {
                        removed_any = true;
                        emptied = servers.is_empty();
                    }
                }
                if emptied {
                    cfg.remove(servers_key.clone());
                }
                if emptied && cfg.is_empty() {
                    m.remove(cfg_key.clone());
                }
                if m.get(&cfg_key).is_none() {
                    // 行已不含任何 config 内容 = 迁空的死壳
                    migrated = true;
                    return true;
                }
                // **改过就必须重序列化**：`for_each_entry_mut` 只在闭包返回 true 时
                // 放弃原文保真。返回"是否迁空"会让**部分**清除（字典里还有别的
                // server）被原文回填 ⇒ 同名双源（旧字典 + 新 insert 行）静默并存。
                return removed_any;
            }
        }
        false
    });
    if migrated {
        patch.retain(|entry| {
            let Some(m) = entry.as_mapping() else {
                return true;
            };
            let legacy_mcp = m.get(&pkg_key).and_then(|p| p.as_str()) == Some(MCP_CLIENT_PKG);
            let empty_cfg = match m.get(&cfg_key) {
                None => true,
                Some(v) => v.as_mapping().is_some_and(|c| c.is_empty()),
            };
            !(legacy_mcp && empty_cfg)
        });
    }

    // 2. 检查现存的 - insert: [...] 数组中是否有该 server（有则**就地合并**）
    //    同层重复 serverName 时"哪一条"由行身份决定（2026-09-18 三修，与删除同一口径）：
    //    `want_id` 给了就只认同 id 的那条；一条都没认上而同名行确实存在 ⇒ 列表已过期，
    //    **必须报错**——此时追加会凭空多出一条同名行，改写第一条又等于删错目标。
    let want_id = Some(server.row_id.as_str()).filter(|s| !s.is_empty());
    let mut found = false;
    let mut same_name_rows = 0usize;
    patch.for_each_entry_mut(|_, entry| {
        let Some(m) = entry.as_mapping_mut() else {
            return false;
        };
        let mut changed_here = false;
        if let Some(insert_seq) = m.get_mut(&insert_key).and_then(|i| i.as_sequence_mut()) {
            for item in insert_seq.iter_mut() {
                if !is_matching_mcp_entry(item, &server.name) {
                    continue;
                }
                same_name_rows += 1;
                if !found && row_id_matches(item, want_id) {
                    merge_insert_entry(item, &server);
                    found = true;
                    changed_here = true;
                }
            }
        }
        // 只在真改了的那一条上放弃原文保真（其余条目含注释照原样回填）。
        changed_here
    });
    if !found {
        if let Some(id) = want_id {
            if same_name_rows > 0 {
                return Err(stale_row_error(&server.name, id, "这一层里找不到"));
            }
        }
    }

    // 3. 若无匹配条目，追加到现有的未带 id 根 insert 块，或创建新根 insert 块
    if !found {
        let id_key = serde_yaml::Value::String("id".into());
        let mut appended = false;
        patch.for_each_entry_mut(|_, entry| {
            if appended {
                return false;
            }
            let Some(m) = entry.as_mapping_mut() else {
                return false;
            };
            if !m.contains_key(&id_key) && m.contains_key(&insert_key) {
                if let Some(insert_seq) = m.get_mut(&insert_key).and_then(|i| i.as_sequence_mut()) {
                    insert_seq.push(new_entry_val.clone());
                    appended = true;
                    return true;
                }
            }
            false
        });

        if !appended {
            let mut new_root_entry = serde_yaml::Mapping::new();
            new_root_entry.insert(insert_key, serde_yaml::Value::Sequence(vec![new_entry_val]));
            patch.push(serde_yaml::Value::Mapping(new_root_entry));
        }
    }
    Ok(())
}

/// 行身份过滤（2026-09-18 三修）。`row_id` 为 `None` / 空 ⇒ **不参与判定**：手写行可
/// 能没有 `id`，且老调用方只有名字可给。给了就必须逐条对齐——同层重复 `serverName`
/// 时只有行 id 能说明用户点的是哪一行，只按名字删会删掉**另一条**（列表里看得见的
/// 那条还在，被删的那条消失了）。
fn row_id_matches(item: &serde_yaml::Value, row_id: Option<&str>) -> bool {
    let Some(want) = row_id.filter(|s| !s.is_empty()) else {
        return true;
    };
    let id_key = serde_yaml::Value::String("id".into());
    item.as_mapping()
        .and_then(|m| m.get(&id_key))
        .and_then(|v| v.as_str())
        == Some(want)
}

/// remove 内核（宿主 / 客体共用）；返回 `true` = 确实删掉了键
/// （无改动即不写回，免 mtime 抖动与无谓备份，同 ADR-0013 纪律）。
fn remove_from_patch(
    patch: &mut crate::plugins::PatchFile,
    server_name: &str,
    row_id: Option<&str>,
) -> bool {
    let insert_key = serde_yaml::Value::String("insert".into());
    let pkg_key = serde_yaml::Value::String("package".into());
    let cfg_key = serde_yaml::Value::String("config".into());
    let servers_key = serde_yaml::Value::String("mcpServers".into());
    let srv_name_yaml = serde_yaml::Value::String(server_name.to_string());

    // 1. 从 insert 数组中移除——**一次只删一条**（2026-09-18 二修）：同层也可能有重复
    //    serverName 的行，`retain` 全删会让用户在界面上"删一条少两条"。逐条删、列表
    //    逐条可见，才对得上行身份。带 `row_id` 时按它选哪一条（三修）。
    let mut removed = false;
    patch.for_each_entry_mut(|_, entry| {
        if removed {
            return false;
        }
        let Some(m) = entry.as_mapping_mut() else {
            return false;
        };
        if let Some(insert_seq) = m.get_mut(&insert_key).and_then(|i| i.as_sequence_mut()) {
            let idx = insert_seq.iter().position(|item| {
                is_matching_mcp_entry(item, server_name) && row_id_matches(item, row_id)
            });
            if let Some(idx) = idx {
                insert_seq.remove(idx);
                removed = true;
                return true;
            }
        }
        false
    });

    // 2. 从旧版 mcpServers 映射中移除
    patch.for_each_entry_mut(|_, entry| {
        if !row_id_matches(entry, row_id) {
            return false;
        }
        let Some(m) = entry.as_mapping_mut() else {
            return false;
        };
        if m.get(&pkg_key).and_then(|p| p.as_str()) == Some(MCP_CLIENT_PKG) {
            if let Some(cfg) = m.get_mut(&cfg_key).and_then(|c| c.as_mapping_mut()) {
                if let Some(servers) = cfg.get_mut(&servers_key).and_then(|s| s.as_mapping_mut()) {
                    if servers.remove(&srv_name_yaml).is_some() {
                        removed = true;
                        return true;
                    }
                }
            }
        }
        false
    });

    // 3. 若移除了某项，清理变空的未带 id 根 insert: []
    if removed {
        patch.retain(|entry| {
            if let Some(m) = entry.as_mapping() {
                let id_key = serde_yaml::Value::String("id".into());
                if !m.contains_key(&id_key) {
                    if let Some(seq) = m.get(&insert_key).and_then(|s| s.as_sequence()) {
                        if seq.is_empty() {
                            return false;
                        }
                    }
                }
            }
            true
        });
    }

    removed
}

/// 纯变换（宿主写文件 / 客体写原语共用）：patch 文本进 → 文本出。
///
/// **保真语义与宿主完全同源**：走 `plugins::PatchFile` 的文本内核——未改动条目
/// 逐字节原样回填（含行间注释、键序、缩进），只有被本函数改写的 MCP 条目重新
/// 序列化。分支原实现经 `serde_json::Value` 中转 ⇒ **元数据全丢**（注释 + 键序 +
/// 缩进），2026-09-11 合并 PR #13 时统一到本内核。
///
/// **`expr` 行不进重序列化通道**（2026-09-18 二修）：条目原文含 `!!js` 标签时，
/// 重序列化会把它展平成字面量（`process.env.GITHUB_TOKEN` 从此变成一个字符串，
/// token 静默失效），而 serde_yaml 读不保留标签 ⇒ **没有任何"合并保真"能救**。
/// 口径改为显式拒绝 + 唯一后门：
/// - 改动只是 `disabled`（建模字段全等）⇒ 走 [`toggle_disabled_in_text`]，逐字节安全；
/// - 真改配置 ⇒ 报错，错误信息指回**文件本身**（手改所在层的 `cordis.patch.yml`
///   是唯一能同时保住 tag 的路径；Dock 的「Patch 原文」标签只读，不在此承诺可写）。
pub fn apply_save_mcp_server(content: &str, server: McpServerConfig) -> Result<String, String> {
    validate_server_name(&server.name)?;
    if server.transport == McpTransport::StreamableHttp {
        validate_server_url(&server.url)?;
    }
    if let Some(existing) = find_row(content, &server.name, Some(server.row_id.as_str())) {
        if existing.expr {
            if modeled_shape_equal(&existing, &server) {
                return toggle_disabled_in_text(content, &existing.row_id, server.disabled)
                    .ok_or_else(|| {
                        format!(
                            "MCP 服务器「{}」含表达式（`!!js`）且无法在原文里定位其行 id，\
                             壳拒绝改写这种条目以避免静默展平。请在文本编辑器里直接修改它所在的 \
                             cordis.patch.yml。",
                            server.name
                        )
                    });
            }
            return Err(format!(
                "MCP 服务器「{}」所在补丁行含 `!!js` 表达式（如 `!!js process.env.X`）。\
                 结构化保存会把它展平成字面量、secret 引用从此失效，故这里拒绝；\
                 仅「启用/停用」可以直接切换。要改这一行的配置，请在文本编辑器里修改它所在的 \
                 cordis.patch.yml（Profile 详情 →「Patch 原文」标签可查看本 Profile 层）。",
                server.name
            ));
        }
    }
    let mut patch = if content.trim().is_empty() {
        crate::plugins::PatchFile::empty()
    } else {
        crate::plugins::PatchFile::from_text(content)?
    };
    upsert_into_patch(&mut patch, server)?;
    patch.render()
}

/// 按 serverName（+ 可选行身份）找**这一份文本**里的目标条目（未命中 = `None`）。
///
/// 只看 `expr` / `row_id` 两个事实，层标注在此无意义（传进来的是单层文本），
/// 故固定按默认层解析、不回填给调用方。
/// `row_id` 为空 ⇒ 回退"同名第一条"：那条判 `expr` 是**有意的保守**（任一条含表达式都
/// 会让整条目重序列化，判第一条已覆盖得住这个风险面）。
fn find_row(content: &str, name: &str, row_id: Option<&str>) -> Option<McpServerConfig> {
    if content.trim().is_empty() {
        return None;
    }
    let want = row_id.filter(|s| !s.is_empty());
    parse_mcp_servers_in_layer(content, McpScope::default())
        .ok()?
        .into_iter()
        .find(|s| s.name == name && want.is_none_or(|id| s.row_id == id))
}

/// 删除也要避开重序列化通道（当原文里有标签时）：按**行块**逐字节移除。
fn remove_row_in_text(content: &str, row_id: &str) -> Option<String> {
    let lines: Vec<&str> = content.split('\n').collect();
    let (start, end, _) = locate_row_block(&lines, row_id)?;
    let (es, ee) = entry_block_of(&lines, start, end);
    let meaningful_left = (es..ee)
        .filter(|&i| i < start || i >= end)
        .filter(|&i| i != es)
        .any(|i| {
            let t = lines[i].trim();
            !t.is_empty() && !t.starts_with('#')
        });
    let (from, to) = if meaningful_left {
        (start, end)
    } else {
        (es, ee) // 条目只剩 `- insert:` 头 ⇒ 连头一起删，不留会被上游拒收的空壳
    };
    let mut out: Vec<&str> = Vec::with_capacity(lines.len() - (to - from));
    out.extend_from_slice(&lines[..from]);
    out.extend_from_slice(&lines[to..]);
    Some(out.join("\n"))
}

/// 原文里是否出现标签值（决定删除走哪条通道）。
fn text_has_tag(content: &str) -> bool {
    content.split('\n').any(yline_has_tag)
}

/// 保存或更新单个 MCP 服务器配置（**写入 `server.scope` 指出的那一层**）。
///
/// **变换内核与客体同一个** [`apply_save_mcp_server`]（含 `expr` 行的拒绝/放行口径），
/// 宿主侧只多了"读文件 + 写文件"。写入走 `PatchFile`：**覆写前备份（fail-closed）
/// + 原子替换 + 未改条目保真**；结果与原文一致时**零写入**（免 mtime 抖动与无谓备份）。
///
/// 作用域语义（2026-09-18）：`Profile` → `profiles/<名>/cordis.patch.yml`（仅本
/// profile）；`Global` → `$DSH_HOME/cordis.patch.yml`（对所有 profile 生效）。
/// 两层**各自独立成文件**，故保存一层不会碰到另一层的行（也就不会误删）。
pub fn save_mcp_server(home: &Path, profile: &str, server: McpServerConfig) -> Result<(), String> {
    crate::profiles::validate_profile_name(profile)?;
    let scope = server.scope;
    // profile 层要求 profile 已物化（写它的目录）；全局层与具体 profile 无关。
    if scope == McpScope::Profile {
        let profile_dir = home.join("profiles").join(profile);
        if !profile_dir.is_dir() {
            return Err(format!("profile「{profile}」不存在或尚未物化"));
        }
    }
    let patch_path = scope.patch_path(home, profile);
    let text = if patch_path.is_file() {
        std::fs::read_to_string(&patch_path)
            .map_err(|e| format!("读取 {} 失败：{e}", patch_path.display()))?
    } else {
        String::new()
    };
    let next = apply_save_mcp_server(&text, server)?;
    if next == text {
        return Ok(());
    }
    let patch = crate::plugins::PatchFile::from_text(&next)?;
    patch.write(&patch_path)
}

/// 纯变换（宿主写文件 / 客体写原语共用）：patch 文本进 → 文本出（未命中即原样返回）。
///
/// **原文含标签时走文本级删除**（2026-09-18 二修）：`PatchFile` 的重序列化粒度是顶层
/// 条目，删 `- insert:` 里的一行会把**同条目其它行**一起重写 ⇒ 兄弟行的 `!!js` 引用
/// 被展平。删除本该只碰被删那一行，按行块移除才名副其实。
///
/// `row_id`（2026-09-18 三修）= 用户点的那一行的身份（同层重复 `serverName` 时唯一
/// 可辨）。给了却没命中 ⇒ **显式报错**而不是回落按名字删：宁可让用户刷新重试，也不能
/// 删掉他没点的那条。
pub fn apply_delete_mcp_server(
    content: &str,
    server_name: &str,
    row_id: Option<&str>,
) -> Result<String, String> {
    if content.trim().is_empty() {
        return Ok(String::new());
    }
    let explicit = row_id.filter(|s| !s.is_empty());
    if text_has_tag(content) {
        let target = match explicit {
            Some(id) => Some(id.to_string()),
            None => find_row(content, server_name, None)
                .map(|r| r.row_id)
                .filter(|s| !s.is_empty()),
        };
        if let Some(id) = target {
            return remove_row_in_text(content, &id)
                .ok_or_else(|| stale_row_error(server_name, &id, "无法在原文里定位"));
        }
    }
    let mut patch = crate::plugins::PatchFile::from_text(content)?;
    if !remove_from_patch(&mut patch, server_name, row_id) {
        // 未命中：**逐字节原样返回**（不重序列化 ⇒ 不产生无谓 diff）。
        // 但显式给了行身份时不能装作"删好了"——那多半是列表已过期。
        if let Some(id) = explicit {
            return Err(stale_row_error(server_name, id, "这一层里找不到"));
        }
        return Ok(content.to_string());
    }
    patch.render()
}

fn stale_row_error(server_name: &str, row_id: &str, why: &str) -> String {
    format!(
        "未删除：{why} MCP 服务器「{server_name}」的行 id「{row_id}」。\
         该层的配置可能已被外部改动，请刷新列表后重试。"
    )
}

/// 删除指定层里的 MCP 服务器配置（无改动零写入）。
///
/// **必须按层删**：删除的语义是"从这条生效范围里拿掉"，所以调用方要带上条目所在层
/// （由 `list_mcp_servers` 的 `scope` 回传）。同名条目若两层都有，删一层后另一层仍在
/// ——这不是漏删，而是另一条独立的生效范围（前端会继续列出它）。
///
/// `row_id`：同一层内重复 `serverName` 的行身份（2026-09-18 三修；可 `None` ⇒ 回落按
/// 名字删第一条）。
///
/// **变换内核与客体同一个** [`apply_delete_mcp_server`]；无改动 ⇒ 零写入。
pub fn delete_mcp_server(
    home: &Path,
    profile: &str,
    server_name: &str,
    scope: McpScope,
    row_id: Option<&str>,
) -> Result<(), String> {
    crate::profiles::validate_profile_name(profile)?;
    let patch_path = scope.patch_path(home, profile);
    if !patch_path.is_file() {
        return Ok(());
    }
    let text = std::fs::read_to_string(&patch_path)
        .map_err(|e| format!("读取 {} 失败：{e}", patch_path.display()))?;
    let next = apply_delete_mcp_server(&text, server_name, row_id)?;
    if next == text {
        return Ok(());
    }
    let patch = crate::plugins::PatchFile::from_text(&next)?;
    patch.write(&patch_path)
}

/// 读取客体指定 profile 可见的**全部** MCP 服务器配置（两用户层都读，同宿主口径）。
///
/// 客体里 `$DSH_HOME` 就是客体的 `~/.dsh`，两层相对路径与宿主同形。
pub fn list_mcp_servers_in_guest(
    distro: &str,
    profile: &str,
) -> Result<Vec<McpServerConfig>, String> {
    crate::profiles::validate_profile_name(profile)?;
    let rels: Vec<String> = [McpScope::Profile, McpScope::Global]
        .iter()
        .map(|s| s.patch_rel_path(profile))
        .collect();
    let files = crate::guest::read_files(distro, &rels)?;
    let mut map: std::collections::HashMap<String, Option<String>> = files.into_iter().collect();
    let mut out = Vec::new();
    for scope in [McpScope::Profile, McpScope::Global] {
        let rel = scope.patch_rel_path(profile);
        let content = map.remove(&rel).flatten().unwrap_or_default();
        out.extend(parse_mcp_servers_in_layer(&content, scope)?);
    }
    Ok(out)
}

/// 在客体中保存或更新单个 MCP 服务器配置（**写入 `server.scope` 指出的那一层**）。
///
/// **与宿主同源**：同一份 `apply_save_mcp_server`（⇒ 同一份保真语义）+ 客体侧
/// 覆写前备份 + 客体侧原子写。分支原实现只有 `mv -f`（原子但**无备份**）。
pub fn save_mcp_server_in_guest(
    distro: &str,
    profile: &str,
    server: McpServerConfig,
) -> Result<(), String> {
    crate::profiles::validate_profile_name(profile)?;
    let pkg_rel = format!("profiles/{profile}/package.json");
    let patch_rel = server.scope.patch_rel_path(profile);
    // profile 层要确认 profile 已物化；全局层只碰 home 根的那份 patch。
    let mut rels = vec![patch_rel.clone()];
    if server.scope == McpScope::Profile {
        rels.push(pkg_rel.clone());
    }
    let files = crate::guest::read_files(distro, &rels)?;
    let mut map: std::collections::HashMap<String, Option<String>> = files.into_iter().collect();

    if server.scope == McpScope::Profile && map.get(&pkg_rel).and_then(|o| o.as_ref()).is_none() {
        return Err(format!("profile「{profile}」不存在或尚未物化"));
    }

    let content = map.remove(&patch_rel).flatten().unwrap_or_default();
    let serialized = apply_save_mcp_server(&content, server)?;
    if serialized == content {
        return Ok(()); // 无变化零写入
    }
    crate::guest::backup_file(distro, &patch_rel)?;
    crate::guest::write_home_files(distro, &[(patch_rel, serialized)])
}

/// 在客体指定层中删除指定 MCP 服务器配置（与宿主同源 + 客体备份）。
pub fn delete_mcp_server_in_guest(
    distro: &str,
    profile: &str,
    server_name: &str,
    scope: McpScope,
    row_id: Option<&str>,
) -> Result<(), String> {
    crate::profiles::validate_profile_name(profile)?;
    let patch_rel = scope.patch_rel_path(profile);
    let files = crate::guest::read_files(distro, std::slice::from_ref(&patch_rel))?;
    let content = files
        .into_iter()
        .find(|(p, _)| p == &patch_rel)
        .and_then(|(_, c)| c)
        .unwrap_or_default();
    if content.trim().is_empty() {
        return Ok(());
    }
    let serialized = apply_delete_mcp_server(&content, server_name, row_id)?;
    if serialized == content {
        return Ok(()); // 未命中 / 无变化：不写、不备份
    }
    crate::guest::backup_file(distro, &patch_rel)?;
    crate::guest::write_home_files(distro, &[(patch_rel, serialized)])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试便捷入口：按 **profile 层** 解析。层标注在多数用例里无关，但必须
    /// 显式传——生产代码已不留「默认层」的入口（见 `parse_mcp_servers_in_layer`）。
    fn parse_profile(content: &str) -> Result<Vec<McpServerConfig>, String> {
        parse_mcp_servers_in_layer(content, McpScope::Profile)
    }

    /// 一条带 `!!js` secret 引用的行（上游文档推荐的传 token 写法，本机真实存在）。
    const TAGGED_ROW: &str = "- insert:\n    - id: mcp-tagger\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: tagger\n        transport: stdio\n        command: npx\n        args:\n          - -y\n        env:\n          TOKEN: !!js process.env.X\n";

    #[test]
    fn mcp_crud_flow_on_patch_yaml() {
        let tmp = std::env::temp_dir().join(format!("dsh-mcp-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let prof_dir = tmp.join("profiles").join("testprof");
        std::fs::create_dir_all(&prof_dir).unwrap();

        // 初始为空
        let list = list_mcp_servers(&tmp, "testprof").unwrap();
        assert!(list.is_empty());

        // 保存一个 GitHub MCP 服务
        let mut env = BTreeMap::new();
        env.insert("TOKEN".to_string(), "abc".to_string());
        let srv = McpServerConfig {
            name: "github".to_string(),
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-github".to_string(),
            ],
            env,
            disabled: false,
            transport: McpTransport::Stdio,
            url: String::new(),
            headers: BTreeMap::new(),
            cwd: String::new(),
            scope: McpScope::Profile,
            row_id: String::new(),
            expr: false,
        };
        save_mcp_server(&tmp, "testprof", srv.clone()).unwrap();

        // 列表应包含 1 个
        let list2 = list_mcp_servers(&tmp, "testprof").unwrap();
        assert_eq!(list2.len(), 1);
        assert_eq!(list2[0].name, "github");
        assert_eq!(list2[0].command, "npx");
        assert_eq!(list2[0].env.get("TOKEN").map(String::as_str), Some("abc"));

        // 再添加一个 Postgres 服务
        let srv2 = McpServerConfig {
            name: "postgres".to_string(),
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-postgres".to_string(),
            ],
            env: BTreeMap::new(),
            disabled: true,
            transport: McpTransport::Stdio,
            url: String::new(),
            headers: BTreeMap::new(),
            cwd: String::new(),
            scope: McpScope::Profile,
            row_id: String::new(),
            expr: false,
        };
        save_mcp_server(&tmp, "testprof", srv2).unwrap();

        let list3 = list_mcp_servers(&tmp, "testprof").unwrap();
        assert_eq!(list3.len(), 2);

        // 删除 github 服务
        delete_mcp_server(&tmp, "testprof", "github", McpScope::Profile, None).unwrap();
        let list4 = list_mcp_servers(&tmp, "testprof").unwrap();
        assert_eq!(list4.len(), 1);
        assert_eq!(list4[0].name, "postgres");
        assert!(list4[0].disabled);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn mcp_preserves_existing_non_mcp_patch_entries() {
        let tmp = std::env::temp_dir().join(format!("dsh-mcp-preserve-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let prof_dir = tmp.join("profiles").join("testprof");
        std::fs::create_dir_all(&prof_dir).unwrap();

        // 预设一个包含其它插件的 cordis.patch.yml
        let initial_patch = r#"
- id: custom-plugin
  package: "@custom/plugin-demo"
  config:
    apiKey: "secret-123"
"#;
        std::fs::write(prof_dir.join("cordis.patch.yml"), initial_patch).unwrap();

        // 添加一个 MCP 服务
        let srv = McpServerConfig {
            name: "fs".to_string(),
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "server-fs".to_string()],
            env: BTreeMap::new(),
            disabled: false,
            transport: McpTransport::Stdio,
            url: String::new(),
            headers: BTreeMap::new(),
            cwd: String::new(),
            scope: McpScope::Profile,
            row_id: String::new(),
            expr: false,
        };
        save_mcp_server(&tmp, "testprof", srv).unwrap();

        // 读取 patch 内容，确保 custom-plugin 仍然存在
        let patch_content = std::fs::read_to_string(prof_dir.join("cordis.patch.yml")).unwrap();
        assert!(patch_content.contains("@custom/plugin-demo"));
        assert!(patch_content.contains("secret-123"));
        assert!(patch_content.contains(MCP_CLIENT_PKG));
        assert!(patch_content.contains("server-fs"));

        // 再更新这个 MCP 服务
        let srv_updated = McpServerConfig {
            name: "fs".to_string(),
            command: "uvx".to_string(),
            args: vec!["mcp-fs".to_string()],
            env: BTreeMap::new(),
            disabled: true,
            transport: McpTransport::Stdio,
            url: String::new(),
            headers: BTreeMap::new(),
            cwd: String::new(),
            scope: McpScope::Profile,
            row_id: String::new(),
            expr: false,
        };
        save_mcp_server(&tmp, "testprof", srv_updated).unwrap();

        let list = list_mcp_servers(&tmp, "testprof").unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].command, "uvx");
        assert!(list[0].disabled);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// 带注释的 patch fixture：头部注释块 + **段间注释** + 一个非 MCP 条目 +
    /// 一个 MCP 条目。注释是用户人工资产（YAML 注释无程序语义，丢了不可逆）。
    const PATCH_WITH_COMMENTS: &str = "\
# ======== 用户手写说明（不可丢）========
# 第二行头部注释
- id: custom-plugin
  package: \"@custom/plugin-demo\"
  config:
    apiKey: \"secret-123\"
# 段间注释：MCP 段从此开始（行间，非头部）
- id: mcp
  package: \"@deepseek-ai/dsh-mcp-client\"
  config:
    mcpServers:
      fs:
        command: npx
        args:
          - -y
          - server-fs
";

    fn mcp_fixture(tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let tmp = std::env::temp_dir().join(format!("{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let prof_dir = tmp.join("profiles").join("testprof");
        std::fs::create_dir_all(&prof_dir).unwrap();
        std::fs::write(prof_dir.join("cordis.patch.yml"), PATCH_WITH_COMMENTS).unwrap();
        (tmp, prof_dir)
    }

    /// ADR-0022 前置（2026-09-15）：`streamable-http` 服务器**没有 `command`**，
    /// 旧实现会把它兜底成 `npx` —— **静默错误配置**。这里钉住两件事：
    /// ① 解析后 `transport` 为 StreamableHttp 且 `command` **保持为空**；
    /// ② 写回是 `transport`/`url`/`headers` 三键，**不含** `command`/`args`。
    #[test]
    fn streamable_http_transport_is_not_misread_as_npx_stdio() {
        let content = r#"
- package: '@deepseek-ai/dsh-mcp-client'
  config:
    mcpServers:
      remote:
        transport: streamable-http
        url: https://example.test/mcp
        headers:
          authorization: Bearer abc
"#;
        let parsed = parse_profile(content).unwrap();
        assert_eq!(parsed.len(), 1);
        let srv = &parsed[0];
        assert_eq!(srv.transport, McpTransport::StreamableHttp);
        assert_eq!(srv.url, "https://example.test/mcp");
        assert_eq!(
            srv.headers.get("authorization").map(String::as_str),
            Some("Bearer abc")
        );
        assert!(
            srv.command.is_empty(),
            "http 传输不得兜底成 npx：得到 {:?}",
            srv.command
        );

        // 写回：三键，且不得出现 command/args（上游该分支没有这两个字段）。
        let (name, value) = server_to_yaml(srv.clone());
        assert_eq!(name, "remote");
        let m = value.as_mapping().unwrap();
        let has = |k: &str| m.contains_key(serde_yaml::Value::String(k.into()));
        assert!(has("transport") && has("url") && has("headers"), "缺键");
        assert!(
            !has("command") && !has("args"),
            "http 分支不得写 command/args"
        );
    }

    fn server(name: &str, command: &str) -> McpServerConfig {
        McpServerConfig {
            name: name.to_string(),
            command: command.to_string(),
            args: vec!["-y".to_string(), format!("server-{name}")],
            env: BTreeMap::new(),
            disabled: false,
            transport: McpTransport::Stdio,
            url: String::new(),
            headers: BTreeMap::new(),
            cwd: String::new(),
            scope: McpScope::Profile,
            row_id: String::new(),
            expr: false,
        }
    }

    /// **复现（2026-09-11，task-26）**：MCP 写入曾对 `cordis.patch.yml` 做整数组
    /// 重序列化 ⇒ **注释全丢**（头部与段间都丢，YAML 注释是用户人工资产，不可逆）；
    /// 且**无备份**。本用例坐实「注释必须保住」。
    #[test]
    fn mcp_write_preserves_user_comments() {
        let (tmp, prof_dir) = mcp_fixture("dsh-mcp-comments");

        save_mcp_server(&tmp, "testprof", server("github", "npx")).unwrap();

        let after = std::fs::read_to_string(prof_dir.join("cordis.patch.yml")).unwrap();
        assert!(
            after.contains("# ======== 用户手写说明（不可丢）========"),
            "头部注释块必须保住，实测内容：\n{after}"
        );
        assert!(
            after.contains("# 第二行头部注释"),
            "头部注释第二行必须保住，实测内容：\n{after}"
        );
        assert!(
            after.contains("# 段间注释：MCP 段从此开始（行间，非头部）"),
            "段间（行间）注释必须保住——只保头部不够，实测内容：\n{after}"
        );
        // 功能不得回归：MCP 增删改照旧生效
        let list = list_mcp_servers(&tmp, "testprof").unwrap();
        assert_eq!(list.len(), 2, "fs + github 都应在：{list:?}");
        assert!(list.iter().any(|s| s.name == "fs"));
        assert!(list.iter().any(|s| s.name == "github"));
        // 非 MCP 条目原样保留
        assert!(after.contains("@custom/plugin-demo"));
        assert!(after.contains("secret-123"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// 删除路径同源：`delete_mcp_server` 同样不得丢注释（同一修复必须覆盖两条路径）。
    #[test]
    fn mcp_delete_preserves_user_comments() {
        let (tmp, prof_dir) = mcp_fixture("dsh-mcp-comments-del");

        delete_mcp_server(&tmp, "testprof", "fs", McpScope::Profile, None).unwrap();

        let after = std::fs::read_to_string(prof_dir.join("cordis.patch.yml")).unwrap();
        assert!(
            after.contains("# ======== 用户手写说明（不可丢）========"),
            "删除路径同样必须保住头部注释：\n{after}"
        );
        assert!(
            after.contains("# 段间注释：MCP 段从此开始（行间，非头部）"),
            "删除路径同样必须保住段间注释：\n{after}"
        );
        assert!(!list_mcp_servers(&tmp, "testprof")
            .unwrap()
            .iter()
            .any(|s| s.name == "fs"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// **客体路径的保真不变量（2026-09-11，合并 PR #13 的 S1 缺口）**：
    /// 客体侧写 `cordis.patch.yml` 走 `apply_save_mcp_server` / `apply_delete_mcp_server`
    /// 这组**文本进出**纯函数（宿主侧走 `PatchFile::read`/`write`）。二者必须**同源**，
    /// 否则又回到"宿主保注释、客体丢注释"的两套语义——分支原实现正是后者
    /// （经 `serde_json::Value` 整数组中转 ⇒ 元数据全丢）。
    ///
    /// 本用例直接钉纯函数（不依赖 Windows/WSL）：**回退到重序列化实现即红**。
    #[test]
    fn guest_apply_paths_preserve_comments_and_key_order() {
        let src = PATCH_WITH_COMMENTS;

        // 保存路径：未改动的非 MCP 条目（含其行间注释 + 键序）必须逐字节存活
        let after = apply_save_mcp_server(src, server("github", "npx")).unwrap();
        assert!(
            after.contains("# ======== 用户手写说明（不可丢）========"),
            "客体保存路径丢了头部注释：\n{after}"
        );
        assert!(
            after.contains("# 段间注释：MCP 段从此开始（行间，非头部）"),
            "客体保存路径丢了段间注释：\n{after}"
        );
        assert!(
            after.contains("apiKey: \"secret-123\""),
            "非 MCP 条目键序/引号风格应原样保真：\n{after}"
        );

        // 删除路径同源
        let after_del = apply_delete_mcp_server(src, "fs", None).unwrap();
        assert!(
            after_del.contains("# ======== 用户手写说明（不可丢）========")
                && after_del.contains("# 段间注释：MCP 段从此开始（行间，非头部）"),
            "客体删除路径丢了注释：\n{after_del}"
        );
        // 未命中即逐字节原样返回（不重序列化 ⇒ 不产生无谓 diff）
        assert_eq!(
            apply_delete_mcp_server(src, "不存在的名字", None).unwrap(),
            src
        );
        // 显式给了行身份却没命中 ⇒ 报错而不是"当作已删"（列表已过期，删错方向未知）。
        assert!(
            apply_delete_mcp_server(src, "不存在的名字", Some("mcp-ghost"))
                .unwrap_err()
                .contains("mcp-ghost"),
            "未命中必须回带上没找到的行 id"
        );
    }

    /// **无备份**：覆写用户数据前必须留一份 `.bak-<unix秒>`（复用
    /// `fs_backup::backup_before_overwrite`，AGENTS §6 已登记），且备份内容 =
    /// 覆写前原文。
    #[test]
    fn mcp_write_backs_up_before_overwrite() {
        let (tmp, prof_dir) = mcp_fixture("dsh-mcp-backup");

        save_mcp_server(&tmp, "testprof", server("github", "npx")).unwrap();

        let backups: Vec<std::path::PathBuf> = std::fs::read_dir(&prof_dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().starts_with("cordis.patch.yml.bak-"))
                    .unwrap_or(false)
            })
            .collect();
        assert_eq!(backups.len(), 1, "覆写前应留一份备份：{backups:?}");
        assert_eq!(
            std::fs::read_to_string(&backups[0]).unwrap(),
            PATCH_WITH_COMMENTS,
            "备份内容必须是覆写前原文（含注释）"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// **DSH 标准 insert 形态解析（2026-09-18）**：
    /// 验证 DSH 官方 `@deepseek-ai/dsh-mcp-client` 的 `- insert:` 结构解析，
    /// 同时覆盖 stdio 与 streamable-http，以及 disabled 属性。
    #[test]
    fn dsh_standard_insert_format_parsing() {
        let yaml = r#"
# 示例 patch 配置
- insert:
    - id: mcp-tempad-dev
      name: '@deepseek-ai/dsh-mcp-client'
      config:
        serverName: tempad-dev
        transport: stdio
        command: npx
        args:
          - -y
          - '@tempad-dev/mcp@latest'
    - id: mcp-dayu
      name: '@deepseek-ai/dsh-mcp-client'
      disabled: true
      config:
        serverName: dayu-mcp
        transport: streamable-http
        url: https://example.test/mcp
        headers:
          Authorization: "Bearer token-123"
"#;
        let servers = parse_profile(yaml).unwrap();
        assert_eq!(servers.len(), 2);

        let tempad = servers.iter().find(|s| s.name == "tempad-dev").unwrap();
        assert_eq!(tempad.command, "npx");
        assert_eq!(tempad.args, vec!["-y", "@tempad-dev/mcp@latest"]);
        assert_eq!(tempad.transport, McpTransport::Stdio);
        assert!(!tempad.disabled);

        let dayu = servers.iter().find(|s| s.name == "dayu-mcp").unwrap();
        assert_eq!(dayu.transport, McpTransport::StreamableHttp);
        assert_eq!(dayu.url, "https://example.test/mcp");
        assert_eq!(
            dayu.headers.get("Authorization").map(String::as_str),
            Some("Bearer token-123")
        );
        assert!(dayu.disabled);
    }

    /// **DSH 标准 insert 形态写入验证（2026-09-18）**：
    /// 验证保存 MCP 时产出的 YAML 为 `- insert: [ { id: "mcp-...", name: "@deepseek-ai/dsh-mcp-client", config: ... } ]`，
    /// 从而能被 DSH loader 的 `applyEntryPatches` 正确加载，不再报 `entry "mcp" not found`。
    #[test]
    fn save_generates_standard_dsh_insert_format() {
        let tmp = std::env::temp_dir().join(format!("dsh-mcp-std-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let prof_dir = tmp.join("profiles").join("testprof");
        std::fs::create_dir_all(&prof_dir).unwrap();

        let srv = McpServerConfig {
            name: "tempad-dev".to_string(),
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "@tempad-dev/mcp@latest".to_string()],
            env: BTreeMap::new(),
            disabled: false,
            transport: McpTransport::Stdio,
            url: String::new(),
            headers: BTreeMap::new(),
            cwd: String::new(),
            scope: McpScope::Profile,
            row_id: String::new(),
            expr: false,
        };
        save_mcp_server(&tmp, "testprof", srv).unwrap();

        let patch_text = std::fs::read_to_string(prof_dir.join("cordis.patch.yml")).unwrap();
        assert!(patch_text.contains("- insert:"));
        assert!(patch_text.contains("id: mcp-tempad-dev"));
        assert!(patch_text.contains("name: '@deepseek-ai/dsh-mcp-client'"));
        assert!(patch_text.contains("serverName: tempad-dev"));
        assert!(!patch_text.contains("mcpServers:"));

        // 读取验证
        let list = list_mcp_servers(&tmp, "testprof").unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "tempad-dev");
        assert_eq!(list[0].command, "npx");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// **旧格式平滑迁移到新格式（2026-09-18）**：
    /// 如果原本有旧版 dock 写入的 `mcpServers` 结构，更新它之后，
    /// 该 server 会被迁出到标准的 `insert` 列表中，防止配置冲突。
    #[test]
    fn legacy_mcp_migrates_to_standard_insert_on_save() {
        let legacy_yaml = r#"
- id: mcp
  package: '@deepseek-ai/dsh-mcp-client'
  config:
    mcpServers:
      legacy-srv:
        command: npx
        args:
          - -y
          - legacy-mcp
"#;
        // 先确认能被读出来
        let parsed = parse_profile(legacy_yaml).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "legacy-srv");

        // 更新保存
        let updated = McpServerConfig {
            name: "legacy-srv".to_string(),
            command: "npx".to_string(),
            args: vec!["-y".to_string(), "updated-mcp".to_string()],
            env: BTreeMap::new(),
            disabled: true,
            transport: McpTransport::Stdio,
            url: String::new(),
            headers: BTreeMap::new(),
            cwd: String::new(),
            scope: McpScope::Profile,
            row_id: String::new(),
            expr: false,
        };
        let after = apply_save_mcp_server(legacy_yaml, updated).unwrap();
        assert!(after.contains("- insert:"));
        assert!(after.contains("id: mcp-legacy-srv"));
        assert!(after.contains("name: '@deepseek-ai/dsh-mcp-client'"));
        assert!(after.contains("disabled: true"));
        assert!(after.contains("updated-mcp"));

        // 验证读出
        let list = parse_profile(&after).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "legacy-srv");
        assert!(list[0].disabled);
    }

    // ---------- 生效范围（2026-09-18）：两层读写删 + 跨层撞名 + 不丢字段 ----------

    /// 建一个带 **两层** 的 home：profile 层 + home 级全局层。
    fn scoped_fixture(tag: &str, profile_yaml: &str, global_yaml: &str) -> std::path::PathBuf {
        let tmp = std::env::temp_dir().join(format!("{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let prof_dir = tmp.join("profiles").join("testprof");
        std::fs::create_dir_all(&prof_dir).unwrap();
        std::fs::write(prof_dir.join("cordis.patch.yml"), profile_yaml).unwrap();
        if !global_yaml.is_empty() {
            std::fs::write(tmp.join("cordis.patch.yml"), global_yaml).unwrap();
        }
        tmp
    }

    /// 全局层 = `$DSH_HOME/cordis.patch.yml`，**对所有 profile 生效**；profile 层只对
    /// 本 profile。UI 必须看得见两层的条目，否则全局配置"MCP 明明在跑，界面却显示没配"。
    #[test]
    fn list_reads_both_layers_and_stamps_scope() {
        let prof = "- insert:\n    - id: mcp-local\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: local\n        transport: stdio\n        command: /usr/bin/true\n";
        let glob = "- insert:\n    - id: mcp-everywhere\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: everywhere\n        transport: stdio\n        command: /usr/bin/true\n";
        let tmp = scoped_fixture("dsh-mcp-scope", prof, glob);

        let list = list_mcp_servers(&tmp, "testprof").unwrap();
        assert_eq!(list.len(), 2, "两层都要列出来：{list:?}");
        // 顺序 = dsh 的 patch 应用顺序：profile 层在前、全局层在后
        assert_eq!(list[0].name, "local");
        assert_eq!(list[0].scope, McpScope::Profile);
        assert_eq!(list[1].name, "everywhere");
        assert_eq!(list[1].scope, McpScope::Global);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// 两层**同名不是覆盖**：上游 `serverName` 是加载期预留，两条都在 ⇒ 后加载的那条
    /// 实例化失败。所以列表必须**两条都返回**（前端据此告警），不能去重掩盖。
    #[test]
    fn same_name_in_both_layers_is_reported_not_deduped() {
        let prof = "- insert:\n    - id: mcp-dup\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: dup\n        transport: stdio\n        command: /usr/bin/true\n";
        let tmp = scoped_fixture("dsh-mcp-dup", prof, prof);

        let list = list_mcp_servers(&tmp, "testprof").unwrap();
        assert_eq!(list.len(), 2, "跨层撞名必须两条都可见：{list:?}");
        assert_eq!(list.iter().filter(|s| s.name == "dup").count(), 2);
        assert_eq!(list[0].scope, McpScope::Profile);
        assert_eq!(list[1].scope, McpScope::Global);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// 保存按 `scope` 选层：`Global` 写 home 根的 patch，**不动** profile 层文件；
    /// `Profile` 反之。两层各自独立成文件，互不覆盖。
    #[test]
    fn save_routes_to_the_selected_layer() {
        let tmp = scoped_fixture("dsh-mcp-layer-save", "- id: custom\n  disabled: true\n", "");

        // ① 全局层：只动 home 根的 patch
        let mut global = server("everywhere", "/usr/bin/true");
        global.scope = McpScope::Global;
        save_mcp_server(&tmp, "testprof", global).unwrap();
        let global_text = std::fs::read_to_string(tmp.join("cordis.patch.yml")).unwrap();
        assert!(
            global_text.contains("serverName: everywhere"),
            "全局保存应落在 home 根 patch：{global_text}"
        );
        let prof_after_global =
            std::fs::read_to_string(tmp.join("profiles/testprof/cordis.patch.yml")).unwrap();
        assert!(
            !prof_after_global.contains("everywhere"),
            "全局保存不得污染 profile 层：{prof_after_global}"
        );
        assert!(
            prof_after_global.contains("- id: custom"),
            "全局保存不得改动 profile 层的其它条目"
        );

        // ② profile 层：只动 profile 文件
        save_mcp_server(&tmp, "testprof", server("local-only", "/usr/bin/true")).unwrap();
        let prof_text =
            std::fs::read_to_string(tmp.join("profiles/testprof/cordis.patch.yml")).unwrap();
        assert!(prof_text.contains("serverName: local-only"), "{prof_text}");
        assert!(
            prof_text.contains("- id: custom"),
            "非 MCP 条目要保住：{prof_text}"
        );
        let global_after = std::fs::read_to_string(tmp.join("cordis.patch.yml")).unwrap();
        assert!(
            !global_after.contains("local-only"),
            "profile 保存不得污染全局层：{global_after}"
        );

        // ③ 两层条目都在列表里，且各带自己的 scope
        let list = list_mcp_servers(&tmp, "testprof").unwrap();
        assert_eq!(list.len(), 2, "{list:?}");
        assert!(list
            .iter()
            .any(|s| s.name == "local-only" && s.scope == McpScope::Profile));
        assert!(list
            .iter()
            .any(|s| s.name == "everywhere" && s.scope == McpScope::Global));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// 删除按层：删全局层那一条，profile 层同名条目**必须留着**（它是另一条生效范围）。
    #[test]
    fn delete_targets_one_layer_only() {
        let prof = "- insert:\n    - id: mcp-dup\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: dup\n        transport: stdio\n        command: /usr/bin/true\n";
        let tmp = scoped_fixture("dsh-mcp-layer-del", prof, prof);

        delete_mcp_server(&tmp, "testprof", "dup", McpScope::Global, None).unwrap();
        let list = list_mcp_servers(&tmp, "testprof").unwrap();
        assert_eq!(list.len(), 1, "只该删掉全局层那条：{list:?}");
        assert_eq!(list[0].scope, McpScope::Profile);

        delete_mcp_server(&tmp, "testprof", "dup", McpScope::Profile, None).unwrap();
        assert!(list_mcp_servers(&tmp, "testprof").unwrap().is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// `cwd` 必须能读能写：本机唯一可用的写法（绝对路径 node + 入口脚本 + 显式 cwd）
    /// 全靠它；此前本模块不认识该键 ⇒ 界面里保存一次就把用户的 `cwd` 删掉。
    #[test]
    fn cwd_round_trips_and_survives_an_edit() {
        let src = "- insert:\n    - id: mcp-tempad-dev\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: tempad-dev\n        transport: stdio\n        command: /usr/bin/node\n        args:\n          - /tmp/entry.mjs\n        cwd: /Users/someone/.dsh/mcp-servers/tempad-dev\n";
        let mut parsed = parse_profile(src).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed[0].cwd, "/Users/someone/.dsh/mcp-servers/tempad-dev",
            "cwd 必须被解析出来（否则编辑即丢）"
        );

        // 走一次"编辑保存"：只改 args，cwd 必须原样留下
        parsed[0].args = vec!["/tmp/other.mjs".to_string()];
        let after = apply_save_mcp_server(src, parsed.remove(0)).unwrap();
        assert!(
            after.contains("cwd: /Users/someone/.dsh/mcp-servers/tempad-dev"),
            "{after}"
        );
        assert!(after.contains("/tmp/other.mjs"), "{after}");
    }

    /// **未建模键不得被编辑动作静默删除**：上游还有 `failOnStartupError` /
    /// `toolCallTimeoutMs` / `maxInstructionBytes` / `reconnect`，本模块不建模它们，
    /// 但"保存"是**就地合并**而非整项替换，故它们必须原样存活。
    #[test]
    fn update_preserves_keys_the_shell_does_not_model() {
        let src = "- insert:\n    - id: mcp-keep\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: keep\n        transport: stdio\n        command: /usr/bin/node\n        args:\n          - /tmp/a.mjs\n        failOnStartupError: true\n        toolCallTimeoutMs: 120000\n        maxInstructionBytes: 4096\n        reconnect:\n          enabled: false\n";
        let mut srv = parse_profile(src).unwrap().remove(0);
        srv.command = "/usr/bin/node".to_string();
        srv.args = vec!["/tmp/b.mjs".to_string()];

        let after = apply_save_mcp_server(src, srv).unwrap();
        for key in [
            "failOnStartupError: true",
            "toolCallTimeoutMs: 120000",
            "maxInstructionBytes: 4096",
            "enabled: false",
        ] {
            assert!(after.contains(key), "编辑后丢了未建模键 {key}：\n{after}");
        }
        assert!(after.contains("/tmp/b.mjs"), "{after}");
        // 停用态为 false ⇒ 不留 `disabled: false` 噪音
        assert!(!after.contains("disabled:"), "{after}");
    }

    /// 用户手写的**行 id 不得被改写**：别的补丁层可能按 id 定位这一行
    /// （`applyEntryPatches` 的 `id` 匹配）。改写 = 让那些补丁静默失效。
    /// 壳只在**新建**条目时自造 `mcp-<名>`。
    #[test]
    fn update_keeps_a_handwritten_row_id() {
        let src = "- insert:\n    - id: my-own-row\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: keepid\n        transport: stdio\n        command: /usr/bin/node\n        args:\n          - /tmp/a.mjs\n";
        let mut srv = parse_profile(src).unwrap().remove(0);
        assert_eq!(srv.name, "keepid");
        srv.command = "/usr/bin/node".to_string();
        srv.args = vec!["/tmp/b.mjs".to_string()];

        let after = apply_save_mcp_server(src, srv).unwrap();
        assert!(
            after.contains("id: my-own-row"),
            "既有行 id 必须原样保留：\n{after}"
        );
        assert!(
            !after.contains("id: mcp-keepid"),
            "不得改写成壳自造的 id：\n{after}"
        );
        assert!(after.contains("/tmp/b.mjs"), "功能不得回归：\n{after}");
    }

    /// 切换传输方式**不留另一分支的残键**（stdio 的 command/args/cwd 与 http 的
    /// url/headers 互斥；留下残键会让下游误判）。
    #[test]
    fn switching_transport_drops_the_other_branch_keys() {
        let src = "- insert:\n    - id: mcp-sw\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: sw\n        transport: stdio\n        command: /usr/bin/node\n        args:\n          - /tmp/a.mjs\n        cwd: /tmp\n";
        let mut srv = parse_profile(src).unwrap().remove(0);
        srv.transport = McpTransport::StreamableHttp;
        srv.url = "https://example.test/mcp".to_string();
        srv.command = String::new();
        srv.args.clear();
        srv.cwd = String::new();

        let after = apply_save_mcp_server(src, srv).unwrap();
        assert!(after.contains("streamable-http"), "{after}");
        assert!(after.contains("url: https://example.test/mcp"), "{after}");
        assert!(
            !after.contains("command:"),
            "切到 http 后不得留 command：\n{after}"
        );
        assert!(!after.contains("cwd:"), "切到 http 后不得留 cwd：\n{after}");
    }

    /// http 传输的 URL 必须在**保存时**就判（正反例各一）：空串与 `file://` 写进
    /// patch 后行照样保存成功，但插件加载期才失败 ⇒ "已启用却永无工具"。
    #[test]
    fn http_transport_requires_an_http_url() {
        let mk = |url: &str| McpServerConfig {
            name: "remote".to_string(),
            command: String::new(),
            args: Vec::new(),
            env: Default::default(),
            disabled: false,
            transport: McpTransport::StreamableHttp,
            url: url.to_string(),
            headers: Default::default(),
            cwd: String::new(),
            scope: McpScope::Profile,
            row_id: String::new(),
            expr: false,
        };
        assert!(apply_save_mcp_server("", mk("https://a.test/mcp")).is_ok());
        let empty = apply_save_mcp_server("", mk("")).unwrap_err();
        assert!(empty.contains("URL"), "要说清缺什么：{empty}");
        for bad in ["file:///etc/passwd", "data:text/plain,x", "//a.test/mcp"] {
            let err = apply_save_mcp_server("", mk(bad)).unwrap_err();
            assert!(
                err.contains("http://") || err.contains("https://"),
                "{bad} 必须被拒：{err}"
            );
        }
    }

    /// 名称校验必须与上游 `SERVER_NAME_PATTERN`（`^[A-Za-z0-9_-]{1,32}$`）同源：
    /// 不合规的名字**行能写进去**，但插件加载期被 schema 拒 ⇒ 界面显示"已启用"、
    /// 模型侧永远没有 `mcp__<名字>__*` 工具。这是"保存成功但永不生效"的静默失败面。
    #[test]
    fn server_name_validation_matches_upstream_pattern() {
        for ok in ["github", "tempad-dev", "dayu_mcp", "a", "A1_-b"] {
            assert!(validate_server_name(ok).is_ok(), "应接受：{ok}");
        }
        for bad in ["我的服务", "foo.bar", "has space", "a/b", "a\\b", ""] {
            assert!(validate_server_name(bad).is_err(), "应拒绝：{bad}");
        }
        let too_long = "a".repeat(33);
        assert!(validate_server_name(&too_long).is_err(), "上游上限 32 字符");
        assert!(validate_server_name(&"a".repeat(32)).is_ok());

        // 非 ASCII 名称必须给出可执行的原因（不是"非法字符"四个字的黑箱）
        let err = validate_server_name("我的服务").unwrap_err();
        assert!(err.contains("ASCII"), "错误信息要说清判据：{err}");
    }

    /// `!!js` 表达式行的**实测前提**（2026-09-18 二修的立论依据）：
    /// serde_yaml 0.9.34 在 `from_str::<Value>` 时把未知标签**丢掉**，所以
    /// `expr` 只能来自原文、"合并保真"救不了它。这条测试锁住该事实——
    /// 上游哪天修好，本测试变红就是在提示"可以改成真正的保真"。
    #[test]
    fn serde_yaml_drops_js_tags_so_expr_must_come_from_text() {
        let v: serde_yaml::Value =
            serde_yaml::from_str("env:\n  TOKEN: !!js process.env.X\n").unwrap();
        let token = v["env"].get("TOKEN").expect("结构应在");
        assert!(
            matches!(token, serde_yaml::Value::String(s) if s == "process.env.X"),
            "实测：标签被展平成字符串，Value 里读不出 tag：{token:?}"
        );

        // ⇒ 本模块的判据：原文有 tag 即 expr=true，且表单看到的是展平字符串
        let rows = parse_profile(TAGGED_ROW).unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].expr, "含 !!js 的行必须标出来");
        assert_eq!(
            rows[0].env.get("TOKEN").map(String::as_str),
            Some("process.env.X")
        );
    }

    /// `expr` 行**拒绝结构化保存**：宁可报错，也不能"保存成功 = secret 引用失效"。
    /// 错误信息必须给出下一步（手改所在层的 cordis.patch.yml），否则用户只看到一个死胡同。
    #[test]
    fn saving_a_tagged_row_is_refused_with_an_actionable_reason() {
        let mut srv = parse_profile(TAGGED_ROW).unwrap().remove(0);
        srv.command = "/usr/bin/node".to_string();
        let err = apply_save_mcp_server(TAGGED_ROW, srv).unwrap_err();
        assert!(err.contains("!!js"), "要说清是什么挡住了：{err}");
        assert!(
            err.contains("cordis.patch.yml"),
            "必须给出可执行的下一步（Dock 里没有 patch 原文的**可写**入口，只能指到文件）：{err}"
        );
    }

    /// `expr` 行的**唯一放行改动 = 启停**：文本级增删一行 `disabled:`，
    /// 其余字节逐字节不动 ⇒ tag 与注释都活着。
    #[test]
    fn toggling_a_tagged_row_keeps_the_tag_verbatim() {
        let mut srv = parse_profile(TAGGED_ROW).unwrap().remove(0);
        assert!(!srv.disabled);
        srv.disabled = true;
        let off = apply_save_mcp_server(TAGGED_ROW, srv.clone()).unwrap();
        assert!(
            off.contains("TOKEN: !!js process.env.X"),
            "启停不得动表达式：\n{off}"
        );
        assert!(off.contains("disabled: true"), "启停必须落盘：\n{off}");
        // 回读：停用态 + tag 仍在（读路径按展平字符串呈现）
        let after = parse_profile(&off).unwrap();
        assert_eq!(after.len(), 1);
        assert!(after[0].disabled, "停用没生效：\n{off}");
        assert!(after[0].expr, "读回后仍应标为表达式行");

        let back = apply_save_mcp_server(&off, {
            let mut s = after[0].clone();
            s.disabled = false;
            s
        })
        .unwrap();
        assert_eq!(back, TAGGED_ROW, "启用应回到原样（不留噪音）：\n{back}");
    }

    /// 幂等：已是目标态时**零改写**（免备份与 mtime 抖动，同 ADR-0013 纪律）。
    #[test]
    fn toggling_a_tagged_row_to_its_current_state_is_a_noop() {
        let mut srv = parse_profile(TAGGED_ROW).unwrap().remove(0);
        srv.disabled = false;
        let same = apply_save_mcp_server(TAGGED_ROW, srv).unwrap();
        assert_eq!(same, TAGGED_ROW);
    }

    /// 删除 `expr` 行同样不得伤及**兄弟行**的表达式（重序列化粒度是顶层条目）。
    #[test]
    fn deleting_one_tagged_row_leaves_its_tagged_sibling_intact() {
        let src = "- insert:\n    - id: mcp-a\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: a\n        transport: stdio\n        command: npx\n        env:\n          TOKEN: !!js process.env.A\n    - id: mcp-b\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: b\n        transport: stdio\n        command: npx\n        env:\n          TOKEN: !!js process.env.B\n";
        let after = apply_delete_mcp_server(src, "a", None).unwrap();
        assert!(
            after.contains("!!js process.env.B"),
            "兄弟行的表达式必须原样：\n{after}"
        );
        assert!(
            !after.contains("process.env.A"),
            "被删的行不该还在：\n{after}"
        );
        assert!(!after.contains("serverName: a"), "{after}");
        let rows = parse_profile(&after).unwrap();
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0].name, "b");
    }

    /// **带标签 + 同层同名**时，行身份必须压过名字回退：`find_row_by_name` 找的是第一条，
    /// 若显式 `row_id` 被忽略，用户点第二条会删掉第一条、且**兄弟行的 `!!js` 引用**还得
    /// 保住。两条口径在同一个用例里一起证。
    #[test]
    fn explicit_row_id_wins_over_the_name_fallback_in_tagged_text() {
        let src = "- insert:\n    - id: mcp-t1\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: dupe\n        transport: stdio\n        command: npx\n        env:\n          TOKEN: !!js process.env.FIRST\n    - id: mcp-t2\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: dupe\n        transport: stdio\n        command: /usr/bin/node\n        env:\n          TOKEN: !!js process.env.SECOND\n";
        let after = apply_delete_mcp_server(src, "dupe", Some("mcp-t2")).unwrap();
        assert!(
            after.contains("!!js process.env.FIRST"),
            "留下的那条表达式必须原样：\n{after}"
        );
        assert!(
            !after.contains("process.env.SECOND"),
            "点名的那条不该还在：\n{after}"
        );
        let rows = parse_profile(&after).unwrap();
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0].row_id, "mcp-t1");
        assert_eq!(rows[0].command, "npx");
    }

    /// 删掉条目里**最后一行**时连 `- insert:` 头一起删——空壳会被上游 schema 拒，
    /// 让用户每次启动都看到一条与本轮无关的 warning。
    #[test]
    fn deleting_the_last_row_of_an_entry_removes_the_entry_head_too() {
        let src = "- insert:\n    - id: mcp-solo\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: solo\n        transport: stdio\n        command: npx\n        env:\n          TOKEN: !!js process.env.S\n- id: other\n  name: '@deepseek-ai/other'\n";
        let after = apply_delete_mcp_server(src, "solo", None).unwrap();
        assert!(
            !after.contains("- insert:"),
            "空 insert 头必须一起清掉：\n{after}"
        );
        assert!(
            after.contains("@deepseek-ai/other"),
            "无关条目不得被碰：\n{after}"
        );
        assert!(parse_profile(&after).unwrap().is_empty());
        // 写出后仍可解析（自证：不能产出砖化的 YAML）
        assert!(crate::plugins::PatchFile::from_text(&after).is_ok());
    }

    /// **同层重复** serverName 两条都要可见（与跨层重复同后果：后加载的实例化失败），
    /// 且删除**一次只删一条**——"删一条少两条"是最坏的那种假象。
    #[test]
    fn same_layer_duplicates_are_both_visible_and_delete_one_at_a_time() {
        let src = "- insert:\n    - id: mcp-dupe\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: dupe\n        transport: stdio\n        command: npx\n    - id: mcp-dupe-2\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: dupe\n        transport: stdio\n        command: /usr/bin/node\n";
        let rows = parse_profile(src).unwrap();
        assert_eq!(rows.len(), 2, "两条都必须可见：{rows:?}");
        assert_eq!(rows[0].command, "npx");
        assert_eq!(rows[1].command, "/usr/bin/node");
        assert_eq!(rows[1].row_id, "mcp-dupe-2", "行身份必须各自独立");

        let after = apply_delete_mcp_server(src, "dupe", None).unwrap();
        let left = parse_profile(&after).unwrap();
        assert_eq!(left.len(), 1, "删一条应剩一条：{left:?}");
        assert_eq!(left[0].row_id, "mcp-dupe-2", "应删掉第一条：{left:?}");

        // **行身份优先**：用户点的是第二条 ⇒ 必须留第一条。只按名字删会删反。
        let by_id = apply_delete_mcp_server(src, "dupe", Some("mcp-dupe-2")).unwrap();
        let left = parse_profile(&by_id).unwrap();
        assert_eq!(left.len(), 1, "{left:?}");
        assert_eq!(left[0].row_id, "mcp-dupe", "点第二条该留下第一条：{left:?}");
    }

    /// **保存侧同样按行身份定位**（与删除同一口径）：改第二条只能动第二条；给了一个
    /// 这层里不存在的行 id ⇒ 报错，**既不追加**（凭空多一条同名行）**也不改第一条**。
    #[test]
    fn saving_targets_the_row_the_client_named() {
        let src = "- insert:\n    - id: mcp-dupe\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: dupe\n        transport: stdio\n        command: npx\n    - id: mcp-dupe-2\n      name: '@deepseek-ai/dsh-mcp-client'\n      config:\n        serverName: dupe\n        transport: stdio\n        command: /usr/bin/node\n";
        let mut srv = server("dupe", "/usr/bin/pnpm");
        srv.row_id = "mcp-dupe-2".to_string();
        let after = apply_save_mcp_server(src, srv).unwrap();
        let rows = parse_profile(&after).unwrap();
        assert_eq!(rows.len(), 2, "编辑不该多出同名行：{rows:?}");
        assert_eq!(rows[0].command, "npx", "第一条必须原样：{rows:?}");
        assert_eq!(rows[1].command, "/usr/bin/pnpm");

        let mut stale = server("dupe", "/usr/bin/pnpm");
        stale.row_id = "mcp-gone".to_string();
        let err = apply_save_mcp_server(src, stale).unwrap_err();
        assert!(err.contains("mcp-gone"), "错误要指出没认上的行 id：{err}");
        assert!(err.contains("刷新"), "错误要给出下一步（刷新列表）：{err}");
    }

    /// **legacy 空壳清理**：旧版 `mcpServers` 字典迁空后不留 `config: {}`，
    /// 否则上游按 schema 拒收这一行（每次启动一条无关 warning）。
    #[test]
    fn migrating_the_only_server_out_of_a_legacy_row_removes_the_dead_shell() {
        let src = "- id: mcp\n  package: '@deepseek-ai/dsh-mcp-client'\n  config:\n    mcpServers:\n      legacy:\n        command: npx\n        args:\n          - -y\n";
        let mut srv = server("legacy", "npx");
        srv.transport = McpTransport::Stdio;
        let after = apply_save_mcp_server(src, srv).unwrap();
        assert!(
            !after.contains("mcpServers"),
            "字典迁空后不该残留：\n{after}"
        );
        assert!(
            !after.contains("package: '@deepseek-ai/dsh-mcp-client'"),
            "迁空的死壳（旧 id: mcp 行）必须整条移除：\n{after}"
        );
        let rows = parse_profile(&after).unwrap();
        assert_eq!(rows.len(), 1, "迁移后只应有一条：{rows:?}");
        assert_eq!(rows[0].name, "legacy");
    }

    /// 字典里**还有别的** server 时，迁走一条只清同名键 —— 不能连坐删行。
    #[test]
    fn migrating_one_server_keeps_its_legacy_siblings() {
        let src = "- id: mcp\n  package: '@deepseek-ai/dsh-mcp-client'\n  config:\n    mcpServers:\n      go:\n        command: npx\n      stay:\n        command: npx\n";
        let after = apply_save_mcp_server(src, server("go", "pnpm")).unwrap();
        assert!(
            after.contains("stay:"),
            "同字典的其它 server 不该被删：\n{after}"
        );
        assert!(after.contains("serverName: go"), "{after}");
        let rows = parse_profile(&after).unwrap();
        assert!(rows.iter().any(|r| r.name == "stay"));
        assert_eq!(rows.iter().filter(|r| r.name == "go").count(), 1);
    }
}
