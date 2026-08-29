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

    fn tmp() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "dsh-dock-plugins-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
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
