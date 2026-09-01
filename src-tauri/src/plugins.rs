//! 插件清单（4.4①，Spike B 方案）：静态清单读 profile 文件层，运行态快照
//! 走回环 HTTP 单调用。
//!
//! - 静态：`dsh.profile.bundles`（官方内置，版本随 dsh 安装目录——不进
//!   profile node_modules）+ `dependencies`（第三方；已安装版本/描述从
//!   `profiles/<名>/node_modules/<pkg>/package.json` 读，符号链接农场只读
//!   穿透，不直写）。web 档实测：第三方插件纯靠 dependencies 加载，patch
//!   层可为空——清单以 manifest 为准，不做 patch 解析（Spike B §2.4）。
//! - 运行态：`POST http://127.0.0.1:<port>/api/pluginInventory/list`
//!   （复现点 11，`docs/spikes/0002-plugin-inventory.md`）：信封
//!   `{type:"client-request",rpcId,method,payload:{args:{}}}`，响应
//!   `{result:{ok,value:{entries:[{entryId,moduleName,enabled,fiberPhase}]}}}`。
//!   一次性快照、不订阅；仅活跃会话的 profile 消费（前端按 profile 匹配合并）。
//!   **id 空间**：entryId（`include:*` 树路径）≠ patch/配置行 id——4.4 后续
//!   禁用写入的 id 以 `--dump-config` 行 id 为准，本模块不提供写入。

use std::path::Path;

/// 清单条目：官方内置 bundle 或第三方依赖插件。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PluginEntry {
    pub name: String,
    /// `bundle` = dsh.profile.bundles（官方内置，随 dsh 安装目录）；
    /// `dependency` = package.json dependencies（第三方外挂）。
    pub kind: PluginKind,
    /// 已安装版本（node_modules 实读）；None = 未安装/内置随 dsh。
    pub installed_version: Option<String>,
    /// package.json `description`（仅第三方且已安装时非空）。
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginKind {
    Bundle,
    Dependency,
}

/// 运行态快照：活跃会话的一次性 pluginInventory。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PluginRuntimeSnapshot {
    /// 快照归属的 profile（活跃会话的）；None = 无活跃会话。
    pub profile: Option<String>,
    pub entries: Vec<RuntimeEntry>,
}

/// loader 树条目（形状锚定 dsh-host-plugin-inventory typert.host.js schema）。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RuntimeEntry {
    pub entry_id: String,
    pub module_name: String,
    pub enabled: bool,
    /// `null = disposed`；`failed|pending|active|loading|unloading` 原样透传。
    pub fiber_phase: Option<String>,
}

/// 列出 profile 的插件清单（阻塞文件操作，IPC 层走 spawn_blocking）。
/// 未物化 / 清单损坏 → Err（列表页两态由调用方把关，详情页已有同口径报错）。
pub fn list_profile_plugins(home: &Path, profile: &str) -> Result<Vec<PluginEntry>, String> {
    crate::profiles::validate_profile_name(profile)?;
    let manifest_path = home.join("profiles").join(profile).join("package.json");
    let text = fs_err(&manifest_path)?;
    let pkg: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("package.json 不是合法 JSON：{e}"))?;

    let mut out = Vec::new();
    // 官方内置 bundle：版本锚在 dsh 安装目录，不进 profile node_modules，不实读。
    for b in crate::profiles::manifest_bundles(&pkg) {
        out.push(PluginEntry {
            name: b,
            kind: PluginKind::Bundle,
            installed_version: None,
            description: None,
        });
    }
    // 第三方依赖：manifest 声明序（BTreeMap 字典序）+ 安装实况。
    if let Some(deps) = pkg.get("dependencies").and_then(|v| v.as_object()) {
        for name in deps.keys() {
            let (version, description) = read_installed(
                &home
                    .join("profiles")
                    .join(profile)
                    .join("node_modules")
                    .join(name),
            );
            out.push(PluginEntry {
                name: name.clone(),
                kind: PluginKind::Dependency,
                installed_version: version,
                description,
            });
        }
    }
    Ok(out)
}

/// 读已安装插件的 `(version, description)`；未安装/损坏 → (None, None)
/// （清单容忍半初始化，与列表页口径一致）。
fn read_installed(pkg_dir: &Path) -> (Option<String>, Option<String>) {
    let Ok(text) = std::fs::read_to_string(pkg_dir.join("package.json")) else {
        return (None, None);
    };
    let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&text) else {
        return (None, None);
    };
    (
        pkg.get("version")
            .and_then(|v| v.as_str())
            .map(String::from),
        pkg.get("description")
            .and_then(|v| v.as_str())
            .map(String::from),
    )
}

fn fs_err(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("读取 {} 失败：{e}", path.display()))
}

/// 构造 pluginInventory/list 请求体（纯函数，信封形状见模块头；rpcId 只需
/// 会话内唯一，纳秒时间戳足够）。
pub fn runtime_request_body(method: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    serde_json::json!({
        "type": "client-request",
        "rpcId": format!("dsh-dock-{nanos}"),
        "method": method,
        "payload": { "args": {} },
    })
    .to_string()
}

/// 解析 pluginInventory/list 响应体（纯函数）：信封 + result.ok 二层。
/// `ok:false`（含 payload 形状被拒的 internal 错）转可读 Err。
pub fn parse_runtime_response(text: &str) -> Result<Vec<RuntimeEntry>, String> {
    let v: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("响应不是合法 JSON：{e}"))?;
    let result = v
        .get("result")
        .ok_or_else(|| "响应缺少 result 字段".to_string())?;
    if !result.get("ok").and_then(|b| b.as_bool()).unwrap_or(false) {
        let msg = result
            .pointer("/error/message")
            .and_then(|m| m.as_str())
            .unwrap_or("未知错误");
        return Err(format!("pluginInventory/list 失败：{msg}"));
    }
    let entries = result
        .pointer("/value/entries")
        .and_then(|e| e.as_array())
        .ok_or_else(|| "响应缺少 value.entries 数组".to_string())?;
    Ok(entries
        .iter()
        .map(|e| RuntimeEntry {
            entry_id: e
                .get("entryId")
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string(),
            module_name: e
                .get("moduleName")
                .and_then(|x| x.as_str())
                .unwrap_or_default()
                .to_string(),
            enabled: e.get("enabled").and_then(|b| b.as_bool()).unwrap_or(true),
            fiber_phase: e
                .get("fiberPhase")
                .and_then(|x| x.as_str())
                .map(String::from),
        })
        .collect())
}

/// 拉取运行态快照（唯一新增网络用途：127.0.0.1 回环只读查询，AGENTS §7
/// 已登记 2026-08-29；复现点 11）。base_origin 形如 `http://127.0.0.1:PORT`；
/// 2s 超时——就绪但未响应的工作台按不可用处理，不拖详情页。
pub fn fetch_runtime_snapshot(base_origin: &str) -> Result<Vec<RuntimeEntry>, String> {
    let url = format!(
        "{}/api/pluginInventory/list",
        base_origin.trim_end_matches('/')
    );
    let resp = ureq::post(&url)
        .timeout(std::time::Duration::from_secs(2))
        .set("content-type", "application/json")
        .send_string(&runtime_request_body("pluginInventory/list"))
        .map_err(|e| format!("回环调用失败：{e}"))?;
    let text = resp
        .into_string()
        .map_err(|e| format!("读取响应失败：{e}"))?;
    parse_runtime_response(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp() -> std::path::PathBuf {
        let seq = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!(
            "dsh-dock-plugins-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            seq
        ));
        std::fs::create_dir_all(d.join("profiles")).unwrap();
        d
    }

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    const PKG: &str = r#"{
  "dependencies": { "dsh-better-sidebar": "^0.16.0", "zipped-pkg": "file:./x" },
  "dsh": { "profile": { "bundles": ["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-web-app"] } },
  "name": "dsh-profile-p",
  "private": true
}"#;

    #[test]
    fn lists_bundles_and_dependencies_with_installed_facts() {
        let home = tmp();
        write(&home.join("profiles/p/package.json"), PKG);
        write(
            &home.join("profiles/p/node_modules/dsh-better-sidebar/package.json"),
            r#"{"name":"dsh-better-sidebar","version":"0.16.1","description":"侧边栏增强"}"#,
        );
        // zipped-pkg 声明了但未安装：字段置空，不报错（半初始化容忍）

        let list = list_profile_plugins(&home, "p").unwrap();
        assert_eq!(list.len(), 4, "2 bundles + 2 dependencies");
        assert_eq!(list[0].name, "@deepseek-ai/dsh-base");
        assert_eq!(list[0].kind, PluginKind::Bundle);
        assert_eq!(list[0].installed_version, None, "内置版本随 dsh，不实读");
        let dep = &list[2];
        assert_eq!(dep.name, "dsh-better-sidebar");
        assert_eq!(dep.kind, PluginKind::Dependency);
        assert_eq!(dep.installed_version.as_deref(), Some("0.16.1"));
        assert_eq!(dep.description.as_deref(), Some("侧边栏增强"));
        // 声明了但未安装：字段置空，不报错（半初始化容忍）
        assert_eq!(list[3].name, "zipped-pkg");
        assert_eq!(list[3].installed_version, None);
        assert_eq!(list[3].description, None);
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn rejects_unmaterialized_and_illegal_names() {
        let home = tmp();
        assert!(list_profile_plugins(&home, "ghost").is_err());
        assert!(list_profile_plugins(&home, "../escape").is_err());
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn runtime_body_has_single_args_object() {
        let body = runtime_request_body("pluginInventory/list");
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["type"], "client-request");
        assert_eq!(v["method"], "pluginInventory/list");
        // Spike B：payload 必须恰一个 plain-object args 字段，否则 internal 错
        assert!(v["payload"]["args"].is_object());
        assert_eq!(v["payload"].as_object().unwrap().len(), 1);
    }

    #[test]
    fn parses_runtime_response_envelope() {
        let ok = r#"{"type":"server-response","rpcId":"x","result":{"ok":true,"value":{"entries":[
            {"entryId":"include:web-runtime","moduleName":"@deepseek-ai/dsh-web-app","enabled":true,"fiberPhase":"active"},
            {"entryId":"include:hmr","moduleName":"@deepseek-ai/cordis-plugin-hmr","enabled":false,"fiberPhase":null}
        ]}}}"#;
        let entries = parse_runtime_response(ok).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].module_name, "@deepseek-ai/dsh-web-app");
        assert_eq!(entries[0].fiber_phase.as_deref(), Some("active"));
        assert_eq!(entries[1].fiber_phase, None, "disposed 映射为 null");

        // ok:false → 可读 Err（internal 错透传 message）
        let err = r#"{"type":"server-response","rpcId":"x","result":{"ok":false,"error":{"code":"internal","message":"Remote payload must contain exactly one plain-object args field"}}}"#;
        let e = parse_runtime_response(err).unwrap_err();
        assert!(e.contains("plain-object"), "{e}");

        // 非 JSON → Err
        assert!(parse_runtime_response("not json").is_err());
    }
}

// ---------- 插件安装 / 卸载 / 更新（4.4②）：dsh plugin 转发链换动词 ----------
//
// 与创建刀同链（profiles.rs run_dsh_plugin）：`dsh plugin --profile <名>
// add/remove/update <spec>` 原样转发 pnpm；pnpm 防御补齐复用创建同一函数。
// add 裸包名 dist-tag 坑（ledger 复现点 7）由 UI 引导带版本段规避；reconcile
// 会把声明 dsh.bundle 的新装依赖回写进 bundles（同复现点 7），装完刷新即见。

/// 插件操作结果：ok = dsh 退出 0 且未超时；detail 为人读文案（失败附输出尾部）。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PluginOpOutcome {
    pub ok: bool,
    pub detail: String,
}

/// 插件操作种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginOp {
    Install,
    Remove,
    Update,
}

impl PluginOp {
    fn verb(self) -> &'static str {
        match self {
            PluginOp::Install => "add",
            PluginOp::Remove => "remove",
            PluginOp::Update => "update",
        }
    }
    fn label(self) -> &'static str {
        match self {
            PluginOp::Install => "安装",
            PluginOp::Remove => "卸载",
            PluginOp::Update => "更新",
        }
    }
}

/// 插件名/规格校验（纯函数）：spec 作为单个 argv 传给 dsh→pnpm（无 shell 参与，
/// 无注入面），但仍须防两类滥用——① pnpm 旗标注入（前导 `-` 会被 pnpm 当参数，
/// 如 `--frozen-lockfile`）；② 控制字符/空白进日志与清单。允许 scope 包名
/// （`@scope/name`）、版本段（`@tag|精确|^~区间`，不含 `><`——需要语义区间时
/// 走终端，v1 不开）。与前端 lib/profiles.ts 的预检镜像同规则。
pub fn validate_plugin_spec(spec: &str) -> Result<(), String> {
    if spec.is_empty() {
        return Err("包名不能为空".to_string());
    }
    if spec.len() > 214 {
        // npm 包名长度上限（scope 内每段 ≤214 总长），超长必非法
        return Err("包名过长（npm 上限 214 字符）".to_string());
    }
    if spec.starts_with('-') {
        return Err("包名不能以 - 开头（会被当作命令参数）".to_string());
    }
    if !spec
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "@/._^~*-".contains(c))
    {
        return Err(
            "包名只允许字母数字与 @/._^~*-（版本段支持 tag、精确版本、^~ 区间）".to_string(),
        );
    }
    Ok(())
}

/// 安装/卸载/更新（阻塞转发，IPC 层走 spawn_blocking；超时同创建 600s）。
/// profile 必须已物化（模板名先创建/首启）；spec 先过校验。
pub fn mutate_plugin_blocking(
    op: PluginOp,
    profile: &str,
    spec: &str,
    data_dir: &Path,
) -> Result<PluginOpOutcome, String> {
    crate::profiles::validate_profile_name(profile)?;
    validate_plugin_spec(spec)?;
    let home = crate::resolve::user_dsh_home();
    if !home
        .join("profiles")
        .join(profile)
        .join("package.json")
        .is_file()
    {
        return Err(format!(
            "profile「{profile}」尚未初始化——先创建或首启一次再管理插件"
        ));
    }
    let path_env = crate::resolve::effective_path();
    let node = crate::resolve::detect_system_node(&path_env)
        .ok_or("未检出系统 Node（PATH 上无 node）——插件操作需要系统 Node 与 dsh")?;
    let dsh = crate::resolve::detect_system_dsh(&path_env)
        .ok_or("未检出系统 dsh（PATH 上无官方安装）——插件操作经 dsh CLI 完成")?;
    crate::updates::ensure_pnpm(
        &node.bin,
        &crate::resolve::path_with_bin(&node.bin, &path_env),
    )?;
    let run = crate::profiles::run_dsh_plugin(
        &node.bin,
        &dsh.bin_js,
        &[
            "plugin".to_string(),
            "--profile".to_string(),
            profile.to_string(),
            op.verb().to_string(),
            spec.to_string(),
        ],
        &home,
        &data_dir.join("plugin-op.log"),
    )?;
    let ok = !run.timed_out && run.code == Some(0);
    let detail = if ok {
        format!(
            "已{label} {spec}（profile「{profile}」）——若该 profile 正在运行，重启后生效。",
            label = op.label(),
        )
    } else if run.timed_out {
        format!(
            "{label}超时（10 分钟）已终止：网络或 registry 不可达时常见，检查网络后重试。",
            label = op.label()
        )
    } else {
        let tail: String = run
            .output
            .lines()
            .rev()
            .take(8)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "{label}失败（dsh 退出码 {}）。输出尾部：\n{}",
            run.code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "未知".into()),
            tail,
            label = op.label()
        )
    };
    Ok(PluginOpOutcome { ok, detail })
}

#[cfg(test)]
mod op_tests {
    use super::*;

    #[test]
    fn plugin_spec_rejects_flag_injection_and_metacharacters() {
        // 合法：裸名 / scope / 版本段（tag、精确、^~ 区间）
        for ok_spec in [
            "dsh-better-sidebar",
            "@scope/pkg",
            "@mars-sea/dsh-commandcode-provider",
            "pkg@0.16.1",
            "pkg@next",
            "pkg@^1.0.0",
            "pkg@~2.3",
        ] {
            assert!(validate_plugin_spec(ok_spec).is_ok(), "{ok_spec}");
        }
        // 恶意/非法：pnpm 旗标注入、空白、元字符、超长、空串、>< 区间（v1 不开）
        for bad in [
            "",
            "-flag",
            "--frozen-lockfile",
            "pkg; rm -rf ~",
            "pkg && reboot",
            "a b",
            "pkg@>=2",
            "pkg`id`",
            "pkg$(id)",
            "pkg|x",
            &format!("a{}", "b".repeat(215)),
        ] {
            assert!(validate_plugin_spec(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn mutate_rejects_unmaterialized_profile_and_bad_spec_before_spawn() {
        // 未物化：先于任何 spawn/网络拒绝
        let data_dir = std::env::temp_dir().join("dsh-dock-op-test");
        let ghost = format!("dsh-dock-ghost-{}", std::process::id());
        assert!(
            mutate_plugin_blocking(PluginOp::Install, &ghost, "pkg", &data_dir)
                .unwrap_err()
                .contains("尚未初始化")
        );
        // 非法 spec：同样先拒（伪 profile 名保证不触发 spawn）
        assert!(mutate_plugin_blocking(PluginOp::Install, &ghost, "-flag", &data_dir).is_err());
    }
}

// ---------- 禁用/启用（4.4③，ADR-0009 第四次修订：patch 写入例外 #3） ----------

/// dump-config 行表条目：`- id: <行id>` / `name: <包名>` 配对 + 壳 toggle 态。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PluginRowState {
    pub id: String,
    /// dump-config 行内 `name:`（= 包名，与 dependencies 对并）。
    pub pkg_name: String,
    /// 壳写入的 patch toggle 是否为 disabled（生效意图真相，见 ADR 第四次修订）。
    pub shell_disabled: bool,
    /// 来源自身 cordis.patch.yml 中该 id 的条目数（4.4④ 收口：「连配置」勾选框
    /// 置灰预检——>0 才有可搬移的配置行；复制时后端权威复核，见第五次修订）。
    pub patch_entries: usize,
}

/// 行 id 配对解析（行级扫描，不用 YAML 解析器）：dump-config 输出含 `!!js`
/// 标签等 serde_yaml 不保证友好的形态；行表形态是机器生成的稳定两行组
/// （`- id: X` 顶格 + `  name: Y` 二行缩进）。带引号的 name 去引号。
fn parse_dump_rows(text: &str) -> Vec<(String, String)> {
    let mut rows = Vec::new();
    let mut pending_id: Option<String> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("- id: ") {
            pending_id = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("  name: ") {
            if let Some(id) = pending_id.take() {
                let name = rest.trim().trim_matches('\'').trim_matches('"').to_string();
                rows.push((id, name));
            }
        } else if !line.starts_with(' ') && !line.is_empty() {
            pending_id = None; // 顶格非空行打断配对（进入其他段落）
        }
    }
    rows
}

/// 读 profile 自家 patch：id -> (含 disabled:true, 条目数)。文件缺失/损坏 →
/// 空表（与清单容忍半初始化同口径）。
fn patch_entry_map(patch_path: &Path) -> std::collections::BTreeMap<String, (bool, usize)> {
    let Ok(text) = std::fs::read_to_string(patch_path) else {
        return Default::default();
    };
    let Ok(v) = serde_yaml::from_str::<serde_yaml::Value>(&text) else {
        return Default::default();
    };
    let Some(seq) = v.as_sequence() else {
        return Default::default();
    };
    let id_key = serde_yaml::Value::String("id".into());
    let disabled_key = serde_yaml::Value::String("disabled".into());
    let mut map = std::collections::BTreeMap::new();
    for e in seq.iter() {
        let Some(m) = e.as_mapping() else { continue };
        let Some(id) = m.get(&id_key).and_then(|v| v.as_str()) else {
            continue;
        };
        let entry = map.entry(id.to_string()).or_insert((false, 0usize));
        entry.1 += 1;
        if m.get(&disabled_key)
            .and_then(|d| d.as_bool())
            .unwrap_or(false)
        {
            entry.0 = true;
        }
    }
    map
}

/// 行表查询（阻塞 spawn `dsh --profile <名> --dump-config`，一次拿全量行 id
/// 与包名配对；行 id 不可从包名推导——ADR 第四次修订）。dump-config 只读，
/// 复用创建链的 spawn 基建（同 env 注入与超时）。
pub fn plugin_rows_blocking(profile: &str, data_dir: &Path) -> Result<Vec<PluginRowState>, String> {
    crate::profiles::validate_profile_name(profile)?;
    let home = crate::resolve::user_dsh_home();
    if !home
        .join("profiles")
        .join(profile)
        .join("package.json")
        .is_file()
    {
        return Err(format!("profile「{profile}」尚未初始化"));
    }
    let path_env = crate::resolve::effective_path();
    let node = crate::resolve::detect_system_node(&path_env)
        .ok_or("未检出系统 Node——行表查询需要系统 Node 与 dsh")?;
    let dsh = crate::resolve::detect_system_dsh(&path_env)
        .ok_or("未检出系统 dsh——行表查询经 dsh CLI 完成")?;
    let run = crate::profiles::run_dsh_plugin(
        &node.bin,
        &dsh.bin_js,
        &[
            "--profile".to_string(),
            profile.to_string(),
            "--dump-config".to_string(),
        ],
        &home,
        &data_dir.join("plugin-rows.log"),
    )?;
    if run.timed_out || run.code != Some(0) {
        return Err(format!(
            "行表查询失败（dsh 退出码 {}）",
            run.code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "未知".into())
        ));
    }
    let patch = patch_entry_map(&home.join("profiles").join(profile).join("cordis.patch.yml"));
    Ok(parse_dump_rows(&run.output)
        .into_iter()
        .map(|(id, pkg_name)| {
            let (shell_disabled, patch_entries) = patch.get(&id).copied().unwrap_or((false, 0));
            PluginRowState {
                shell_disabled,
                patch_entries,
                id,
                pkg_name,
            }
        })
        .collect())
}

/// 读 patch 文件为 (头部注释块, 顶层数组条目)。头部 = 从首行起连续 `#` 行与其
/// 间空行（用户可见文档，序列化会丢，写回原样前置；其余位置注释不保，已知
/// 代价——ADR 第四次修订）。顶层数组之外还有内容 → 拒绝（patch 方言即数组）。
/// 文件必须存在（缺失 = 三件套不完整，不代 dsh 生成）。
fn read_patch_entries(path: &Path) -> Result<(String, Vec<serde_yaml::Value>), String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("读取 {} 失败：{e}", path.display()))?;
    let mut header = String::new();
    let mut body_start = 0usize;
    for (i, line) in text.lines().enumerate() {
        if line.starts_with('#') || line.trim().is_empty() {
            header.push_str(line);
            header.push('\n');
            body_start = i + 1;
        } else {
            break;
        }
    }
    let body: String = text.lines().skip(body_start).collect::<Vec<_>>().join("\n");
    let seq: Vec<serde_yaml::Value> = match serde_yaml::from_str::<serde_yaml::Value>(&body) {
        Ok(v) if v.is_null() => Vec::new(),
        Ok(v) => v
            .as_sequence()
            .ok_or_else(|| "cordis.patch.yml 顶层数组之外还有内容——拒绝写入".to_string())?
            .clone(),
        Err(e) => return Err(format!("cordis.patch.yml 解析失败：{e}")),
    };
    Ok((header, seq))
}

/// 写 patch 文件：头部注释前置 + 条目序列化；空数组补 `[]\n`（保持单文档可解析）。
fn write_patch_entries(path: &Path, header: &str, seq: &[serde_yaml::Value]) -> Result<(), String> {
    let mut out = header.to_string();
    if !seq.is_empty() {
        out.push_str(&serde_yaml::to_string(seq).map_err(|e| format!("序列化失败：{e}"))?);
    } else if !out.ends_with("[]\n") {
        out.push_str("[]\n");
    }
    std::fs::write(path, out).map_err(|e| format!("写 {} 失败：{e}", path.display()))
}

/// 禁用/启用切换（patch 写入例外 #3，读改写顶层数组；文件头部连续注释块
/// 原样前置保真——注释为用户可见文档，序列化会丢其余位置注释，已知代价）。
/// 禁用：id 条目存在则仅置 disabled 键，否则追加 `{id, disabled}` 双键条目；
/// 启用：移除 disabled 键，条目只剩 id 则整条移除。
pub fn set_plugin_disabled(
    home: &Path,
    profile: &str,
    row_id: &str,
    disabled: bool,
) -> Result<(), String> {
    crate::profiles::validate_profile_name(profile)?;
    if row_id.is_empty() || row_id.contains(['/', '\n']) {
        return Err("行 id 非法".to_string());
    }
    let patch_path = home.join("profiles").join(profile).join("cordis.patch.yml");
    let (header, mut seq) = read_patch_entries(&patch_path)?;
    let id_key = serde_yaml::Value::String("id".into());
    let disabled_key = serde_yaml::Value::String("disabled".into());
    let mut found = false;
    for entry in seq.iter_mut() {
        let Some(m) = entry.as_mapping_mut() else {
            continue;
        };
        if m.get(&id_key).and_then(|v| v.as_str()) == Some(row_id) {
            found = true;
            if disabled {
                m.insert(disabled_key.clone(), serde_yaml::Value::Bool(true));
            } else {
                m.remove(&disabled_key);
            }
        }
    }
    if !found && disabled {
        let mut m = serde_yaml::Mapping::new();
        m.insert(
            id_key.clone(),
            serde_yaml::Value::String(row_id.to_string()),
        );
        m.insert(disabled_key, serde_yaml::Value::Bool(true));
        seq.push(serde_yaml::Value::Mapping(m));
    }
    // 启用后只剩 id 键的条目整条移除（恢复原状）
    if !disabled {
        seq.retain(|e| {
            e.as_mapping()
                .map(|m| m.len() > 1 || !m.contains_key(&id_key))
                .unwrap_or(true)
        });
    }
    write_patch_entries(&patch_path, &header, &seq)
}

#[cfg(test)]
mod patch_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp() -> std::path::PathBuf {
        let seq = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!(
            "dsh-dock-patch-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            seq
        ));
        std::fs::create_dir_all(d.join("profiles/p")).unwrap();
        d
    }

    const HEADER: &str =
        "# Your patch layer for this dsh profile\n# applied after every bundle layer\n";

    #[test]
    fn disable_appends_entry_enabling_removes_it() {
        let home = tmp();
        let patch = home.join("profiles/p/cordis.patch.yml");
        std::fs::write(&patch, format!("{HEADER}[]\n")).unwrap();
        // 禁用：追加双键条目 + 头部注释保真
        set_plugin_disabled(&home, "p", "better-sidebar", true).unwrap();
        let text = std::fs::read_to_string(&patch).unwrap();
        assert!(text.starts_with(HEADER), "注释头保真：{text}");
        assert!(text.contains("- id: better-sidebar"), "{text}");
        assert!(text.contains("disabled: true"), "{text}");
        // 重复禁用幂等（单条目）
        set_plugin_disabled(&home, "p", "better-sidebar", true).unwrap();
        let text = std::fs::read_to_string(&patch).unwrap();
        assert_eq!(text.matches("better-sidebar").count(), 1, "{text}");
        // 启用：条目只剩 id → 整条移除，恢复 `[]`
        set_plugin_disabled(&home, "p", "better-sidebar", false).unwrap();
        let text = std::fs::read_to_string(&patch).unwrap();
        assert!(text.trim_end().ends_with("[]"), "{text}");
        assert!(!text.contains("better-sidebar"), "{text}");
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn enable_keeps_entries_with_other_keys_and_toggles_in_place() {
        let home = tmp();
        let patch = home.join("profiles/p/cordis.patch.yml");
        std::fs::write(
            &patch,
            format!("{HEADER}- id: row-a\n  config:\n    k: v\n"),
        )
        .unwrap();
        // 已有带 config 的条目：禁用只加 disabled 键，不碰 config
        set_plugin_disabled(&home, "p", "row-a", true).unwrap();
        let text = std::fs::read_to_string(&patch).unwrap();
        assert!(text.contains("config:"), "{text}");
        assert!(text.contains("disabled: true"), "{text}");
        // 启用：移除 disabled 键但条目保留（还有 config 键）
        set_plugin_disabled(&home, "p", "row-a", false).unwrap();
        let text = std::fs::read_to_string(&patch).unwrap();
        assert!(text.contains("row-a") && text.contains("config:"), "{text}");
        assert!(!text.contains("disabled:"), "{text}");
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn parse_dump_rows_pairs_id_and_name_lines() {
        let dump = "meta: 1\n- id: llm-pi-ai\n  name: '@deepseek-ai/dsh-llm-pi-ai'\n- id: llm-commandcode\n  name: '@mars-sea/dsh-commandcode-provider'\n  config:\n    apiKeyEnv: X\nsomewhere-else:\n  - id: nested\n    name: not-top\n";
        let rows = parse_dump_rows(dump);
        assert_eq!(
            rows,
            vec![
                ("llm-pi-ai".into(), "@deepseek-ai/dsh-llm-pi-ai".into()),
                (
                    "llm-commandcode".into(),
                    "@mars-sea/dsh-commandcode-provider".into()
                ),
            ]
        );
    }
}

// ---------- 更新检查（4.4④）：registry 外网查询经 updates.rs 镜像链 ----------

/// 单插件可更新项。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PluginUpdateInfo {
    pub name: String,
    pub current: String,
    pub latest: String,
}

/// 更新检查报告：updates = 落后于 dist-tags.latest 的已装插件；
/// failed = 查询失败的个数（镜像链不可达/包名不存在），不计入 checked。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PluginUpdateReport {
    pub updates: Vec<PluginUpdateInfo>,
    pub checked: usize,
    pub failed: usize,
}

/// 逐个外挂插件查 registry（阻塞、串行；按钮触发不自动跑）。current ≥ latest
/// 的不进报告；latest 取 dist-tags（与 pnpm 默认安装语义一致，复现点 7 教训）。
pub fn check_updates_blocking(home: &Path, profile: &str) -> Result<PluginUpdateReport, String> {
    let deps: Vec<PluginEntry> = list_profile_plugins(home, profile)?
        .into_iter()
        .filter(|p| p.kind == PluginKind::Dependency && p.installed_version.is_some())
        .collect();
    let mut report = PluginUpdateReport {
        updates: Vec::new(),
        checked: 0,
        failed: 0,
    };
    for dep in deps {
        if validate_plugin_spec(&dep.name).is_err() {
            continue; // 奇异名（file: 镜像等 registry 查不到的形态）不打 registry
        }
        let current = dep.installed_version.clone().unwrap_or_default();
        report.checked += 1;
        match crate::updates::npm_packument_versions(&dep.name) {
            Ok((latest, _)) => {
                if crate::resolve::compare_versions_asc(&current, &latest)
                    == std::cmp::Ordering::Less
                {
                    report.updates.push(PluginUpdateInfo {
                        name: dep.name,
                        current,
                        latest,
                    });
                }
            }
            Err(_) => report.failed += 1,
        }
    }
    Ok(report)
}

/// 版本列表（选版本更新用）：降序，最新在前。
pub fn plugin_versions_blocking(package: &str) -> Result<Vec<String>, String> {
    validate_plugin_spec(package)?;
    let (_, mut versions) = crate::updates::npm_packument_versions(package)?;
    versions.reverse();
    Ok(versions)
}

// ---------- 跨 profile 聚合 + 从其他 profile 安装（4.4④ 收口，ADR-0009
// ---------- 第五次修订：聚合只读；配置行原样复制 = patch 写入例外 #4）。

/// 聚合条目：一个第三方插件在各 profile 的安装分布。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct AggregatePlugin {
    pub name: String,
    /// 首个非空 description（任一来源 profile 实读）。
    pub description: Option<String>,
    /// 安装分布（profile 字典序，来自 scan_profiles 排序）。
    pub sources: Vec<AggregateSource>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct AggregateSource {
    pub profile: String,
    /// 已装版本（node_modules 实读）；None = 声明未安装（聚合容忍半初始化）。
    pub version: Option<String>,
}

/// 插件总览聚合（只读纯文件扫描，零 dsh 子进程、零网络）：全部已物化 profile
/// 的第三方依赖按包名归组。单 profile 清单损坏 → 跳过该 profile（聚合不让
/// 单点损坏全页失败，与列表页容忍口径一致）。
pub fn aggregate_plugins_blocking(home: &Path) -> Vec<AggregatePlugin> {
    let mut by_name: std::collections::BTreeMap<String, AggregatePlugin> = Default::default();
    for p in crate::profiles::scan_profiles(home) {
        if !p.materialized {
            continue;
        }
        let entries = match list_profile_plugins(home, &p.name) {
            Ok(v) => v,
            Err(_) => continue,
        };
        for e in entries
            .into_iter()
            .filter(|e| e.kind == PluginKind::Dependency)
        {
            let agg = by_name.entry(e.name.clone()).or_insert_with(|| {
                let description = e.description.clone();
                AggregatePlugin {
                    name: e.name,
                    description,
                    sources: Vec::new(),
                }
            });
            if agg.description.is_none() {
                agg.description = e.description.clone();
            }
            agg.sources.push(AggregateSource {
                profile: p.name.clone(),
                version: e.installed_version,
            });
        }
    }
    by_name.into_values().collect()
}

/// 配置行复制结果：copied = 实际追加条目数；skipped_existing = 目标已有同 id
/// 条目零写入（不覆盖——patch 行按 id 定位、config 键整体替换，ADR 第五次修订）。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CopyConfigOutcome {
    pub copied: usize,
    pub skipped_existing: bool,
    /// 人读文案（成功含「重启后生效」，skipped 含不覆盖原因）。
    pub detail: String,
}

/// 顶层条目中 id 匹配的全部条目（原样克隆——例外 #4 的「原样搬移」）。
fn entries_with_id(seq: &[serde_yaml::Value], row_id: &str) -> Vec<serde_yaml::Value> {
    let id_key = serde_yaml::Value::String("id".into());
    seq.iter()
        .filter(|e| {
            e.as_mapping()
                .and_then(|m| m.get(&id_key))
                .and_then(|v| v.as_str())
                == Some(row_id)
        })
        .cloned()
        .collect()
}

/// 配置行原样复制（patch 写入例外 #4，ADR-0009 第五次修订）：把来源 profile
/// patch 中该插件行 id 的全部条目**原样追加**到目标 patch 顶层数组。只追加不
/// 覆盖；行 id 经 dump-config 行表定位（不可从包名推导，第四次修订）。阻塞
/// spawn + 文件操作，IPC 层走 spawn_blocking。
pub fn copy_plugin_config_blocking(
    home: &Path,
    source: &str,
    target: &str,
    package: &str,
    data_dir: &Path,
) -> Result<CopyConfigOutcome, String> {
    crate::profiles::validate_profile_name(source)?;
    crate::profiles::validate_profile_name(target)?;
    if source == target {
        return Err("来源与目标是同一个 profile".to_string());
    }
    for p in [source, target] {
        if !home.join("profiles").join(p).join("package.json").is_file() {
            return Err(format!("profile「{p}」尚未初始化"));
        }
    }
    let source_patch = home.join("profiles").join(source).join("cordis.patch.yml");
    let target_patch = home.join("profiles").join(target).join("cordis.patch.yml");
    for (role, path) in [("来源", &source_patch), ("目标", &target_patch)] {
        if !path.is_file() {
            return Err(format!(
                "{role} profile 尚无 cordis.patch.yml——无可搬移的配置层"
            ));
        }
    }
    // 行 id 定位：dump-config 来源 profile（一次 spawn 全量行表，秒级）
    let row_id = plugin_rows_blocking(source, data_dir)?
        .into_iter()
        .find(|r| r.pkg_name == package)
        .map(|r| r.id)
        .ok_or_else(|| {
            format!("来源 profile「{source}」的行表中没有插件「{package}」——无可搬移的配置行")
        })?;
    copy_config_entries(home, source, target, package, &row_id)
}

/// 复制的文件层核心（行 id 已定位；与 spawn 边界分离便于单测）。
fn copy_config_entries(
    home: &Path,
    source: &str,
    target: &str,
    package: &str,
    row_id: &str,
) -> Result<CopyConfigOutcome, String> {
    let source_entries = {
        let (_, seq) =
            read_patch_entries(&home.join("profiles").join(source).join("cordis.patch.yml"))?;
        entries_with_id(&seq, row_id)
    };
    if source_entries.is_empty() {
        return Err(format!(
            "来源 profile「{source}」的 cordis.patch.yml 没有「{package}」（行 id {row_id}）的配置条目"
        ));
    }
    let target_patch = home.join("profiles").join(target).join("cordis.patch.yml");
    let (header, mut seq) = read_patch_entries(&target_patch)?;
    if !entries_with_id(&seq, row_id).is_empty() {
        return Ok(CopyConfigOutcome {
            copied: 0,
            skipped_existing: true,
            detail: format!(
                "目标 profile「{target}」已有「{package}」的配置行——为不覆盖既有配置，本次未复制"
            ),
        });
    }
    let copied = source_entries.len();
    seq.extend(source_entries);
    write_patch_entries(&target_patch, &header, &seq)?;
    Ok(CopyConfigOutcome {
        copied,
        skipped_existing: false,
        detail: format!(
            "已把「{package}」的 {copied} 条配置行从「{source}」原样复制到「{target}」——重启「{target}」后生效。"
        ),
    })
}

#[cfg(test)]
mod aggregate_copy_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp() -> std::path::PathBuf {
        let seq = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let d = std::env::temp_dir().join(format!(
            "dsh-dock-agg-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            seq
        ));
        std::fs::create_dir_all(d.join("profiles")).unwrap();
        d
    }

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    const HEADER: &str =
        "# Your patch layer for this dsh profile\n# applied after every bundle layer\n";

    #[test]
    fn aggregate_groups_third_party_across_profiles() {
        let home = tmp();
        // 两个 profile：共享 dsh-better-sidebar（版本不同），各自独有插件
        write(
            &home.join("profiles/web/package.json"),
            r#"{"dependencies":{"dsh-better-sidebar":"^0.16.0","dsh-only-a":"^1.0.0"},"dsh":{"profile":{"bundles":["@deepseek-ai/dsh-base"]}}}"#,
        );
        write(
            &home.join("profiles/web/node_modules/dsh-better-sidebar/package.json"),
            r#"{"name":"dsh-better-sidebar","version":"0.16.1","description":"侧边栏增强"}"#,
        );
        write(
            &home.join("profiles/web/node_modules/dsh-only-a/package.json"),
            r#"{"name":"dsh-only-a","version":"1.2.0"}"#,
        );
        write(
            &home.join("profiles/dev/package.json"),
            r#"{"dependencies":{"dsh-better-sidebar":"^0.15.0","dsh-ghost":"^2.0.0"},"dsh":{"profile":{"bundles":[]}}}"#,
        );
        write(
            &home.join("profiles/dev/node_modules/dsh-better-sidebar/package.json"),
            r#"{"name":"dsh-better-sidebar","version":"0.15.3"}"#,
        );
        // dsh-ghost 声明未安装：聚合容忍（version=None）
        // 未物化模板名与损坏清单 profile 不进聚合
        std::fs::create_dir_all(home.join("profiles/broken")).unwrap();
        write(&home.join("profiles/broken/package.json"), "not json");

        let agg = aggregate_plugins_blocking(&home);
        assert_eq!(agg.len(), 3, "按包名归组：{agg:?}");
        assert_eq!(agg[0].name, "dsh-better-sidebar");
        assert_eq!(agg[0].description.as_deref(), Some("侧边栏增强"));
        assert_eq!(agg[0].sources.len(), 2);
        assert_eq!(agg[0].sources[0].profile, "dev");
        assert_eq!(agg[0].sources[0].version.as_deref(), Some("0.15.3"));
        assert_eq!(agg[0].sources[1].profile, "web");
        assert_eq!(agg[0].sources[1].version.as_deref(), Some("0.16.1"));
        // 内置 bundle（@deepseek-ai/dsh-base）不进聚合
        assert!(agg.iter().all(|a| !a.name.starts_with("@deepseek-ai")));
        let ghost = agg.iter().find(|a| a.name == "dsh-ghost").unwrap();
        assert_eq!(ghost.sources.len(), 1);
        assert_eq!(ghost.sources[0].version, None, "声明未安装 → None");
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn copy_config_appends_verbatim_and_refuses_overwrite() {
        let home = tmp();
        for p in ["src-p", "dst-p"] {
            write(&home.join(format!("profiles/{p}/package.json")), "{}");
        }
        // 来源：该 id 两条条目（config 行 + disabled toggle 行）——全部原样搬
        write(
            &home.join("profiles/src-p/cordis.patch.yml"),
            &format!(
                "{HEADER}- id: llm-commandcode\n  config:\n    apiKeyEnv: DSH_KEY\n    nested:\n      k: v\n- id: other-row\n  config:\n    x: 1\n- id: llm-commandcode\n  disabled: true\n"
            ),
        );
        // 目标：头部注释 + 空数组
        write(
            &home.join("profiles/dst-p/cordis.patch.yml"),
            &format!("{HEADER}[]\n"),
        );
        let out = copy_config_entries(
            &home,
            "src-p",
            "dst-p",
            "@mars-sea/dsh-commandcode-provider",
            "llm-commandcode",
        )
        .unwrap();
        assert_eq!(out.copied, 2);
        assert!(!out.skipped_existing);
        let text = std::fs::read_to_string(home.join("profiles/dst-p/cordis.patch.yml")).unwrap();
        assert!(text.starts_with(HEADER), "注释头保真：{text}");
        assert!(
            text.contains("apiKeyEnv: DSH_KEY"),
            "嵌套 config 原样：{text}"
        );
        assert!(text.contains("disabled: true"), "{text}");
        assert!(!text.contains("other-row"), "其他行不搬：{text}");
        assert_eq!(text.matches("llm-commandcode").count(), 2, "{text}");
        // 再次复制：目标已有同 id → 零写入 skipped
        let out = copy_config_entries(&home, "src-p", "dst-p", "pkg", "llm-commandcode").unwrap();
        assert!(out.skipped_existing);
        assert_eq!(out.copied, 0);
        let text2 = std::fs::read_to_string(home.join("profiles/dst-p/cordis.patch.yml")).unwrap();
        assert_eq!(text, text2, "skipped 时文件零变化");
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn copy_config_rejects_same_profile_and_missing_entries() {
        let home = tmp();
        for p in ["a-p", "b-p"] {
            write(&home.join(format!("profiles/{p}/package.json")), "{}");
        }
        write(&home.join("profiles/a-p/cordis.patch.yml"), "[]\n");
        write(&home.join("profiles/b-p/cordis.patch.yml"), "[]\n");
        // 同名拒绝（先于任何文件操作）
        assert!(copy_config_entries(&home, "a-p", "a-p", "pkg", "row").is_err());
        // 来源无该 id 条目 → 明确报错
        let e = copy_config_entries(&home, "a-p", "b-p", "pkg", "ghost-row").unwrap_err();
        assert!(e.contains("ghost-row"), "{e}");
        // 行表外层（spawn 路径）的同名 / 未初始化拒绝
        let data_dir = std::env::temp_dir().join("dsh-dock-copy-test");
        assert!(copy_plugin_config_blocking(&home, "a-p", "a-p", "pkg", &data_dir).is_err());
        let ghost = format!("ghost-{}", std::process::id());
        assert!(copy_plugin_config_blocking(&home, &ghost, "b-p", "pkg", &data_dir).is_err());
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn patch_entry_map_counts_entries_and_disabled() {
        let home = tmp();
        let patch = home.join("profiles/p/cordis.patch.yml");
        write(
            &patch,
            "- id: row-a\n  config:\n    k: v\n- id: row-a\n  disabled: true\n- id: row-b\n  disabled: true\n- no-id-entry\n",
        );
        let map = patch_entry_map(&patch);
        assert_eq!(
            map.get("row-a"),
            Some(&(true, 2)),
            "任一条目带 disabled 即记 toggle（同原 patch_disabled_ids 口径）"
        );
        assert_eq!(map.get("row-b"), Some(&(true, 1)));
        assert_eq!(map.len(), 2);
        // 文件缺失 → 空表（容忍）
        assert!(patch_entry_map(&home.join("profiles/p/none.yml")).is_empty());
        std::fs::remove_dir_all(&home).ok();
    }
}
