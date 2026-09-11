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

use std::collections::BTreeMap;
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
    /// package.json dependencies 声明值原样：npm 来源 = 版本区间；git/tarball
    /// 来源 = 安装 spec（如 `github:o/r#path:/x`）。市场条目与已装依赖的
    /// 连接键——git 形态的真实包名（@scope/xxx）可能与市场展示名不同
    /// （ADR-0011）。
    pub spec: Option<String>,
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
    let dir = home.join("profiles").join(profile);
    let text = fs_err(&dir.join("package.json"))?;
    // 已装版本/描述：读齐各依赖的 `node_modules/<包>/package.json` 原文
    // （符号链接农场只读穿透，不直写）；解析交给共用的纯装配函数。
    let installed: BTreeMap<String, Option<String>> = dependency_names(&text)?
        .into_iter()
        .map(|name| {
            let manifest =
                std::fs::read_to_string(dir.join("node_modules").join(&name).join("package.json"))
                    .ok();
            (name, manifest)
        })
        .collect();
    assemble_plugin_entries(&text, &installed)
}

/// 客体档孪生（ADR-0016 §5-c）：清单与各依赖的已装 `package.json` 经客体读原语
/// **批量**取回（两次往返：先清单拿依赖名，再一次读齐各依赖），解析与派生走同一份
/// [`assemble_plugin_entries`]——解析层零改动，不产生第二套实现。
pub fn list_profile_plugins_in_guest(
    distro: &str,
    profile: &str,
) -> Result<Vec<PluginEntry>, String> {
    crate::profiles::validate_profile_name(profile)?;
    let rel = format!("profiles/{profile}/package.json");
    let manifest = crate::guest::read_files(distro, std::slice::from_ref(&rel))?
        .into_iter()
        .next()
        .and_then(|(_, text)| text)
        .ok_or_else(|| format!("profile「{profile}」尚未初始化（客体 {distro} 内无 {rel}）"))?;
    let names = dependency_names(&manifest)?;
    let installed: BTreeMap<String, Option<String>> = if names.is_empty() {
        BTreeMap::new()
    } else {
        let rels: Vec<String> = names
            .iter()
            .map(|n| format!("profiles/{profile}/node_modules/{n}/package.json"))
            .collect();
        let got = crate::guest::read_files(distro, &rels)?;
        names
            .into_iter()
            .zip(got.into_iter().map(|(_, text)| text))
            .collect()
    };
    assemble_plugin_entries(&manifest, &installed)
}

/// 纯装配（本地 / 客体共用）：manifest 原文 + 「依赖包名 → 已装 package.json 原文」
/// → 插件清单条目。**解析与派生逻辑只有这一份**——ADR-0016 §3-A 选择"读原文"
/// 正是为了这个收益。
fn assemble_plugin_entries(
    manifest_text: &str,
    installed: &BTreeMap<String, Option<String>>,
) -> Result<Vec<PluginEntry>, String> {
    let pkg: serde_json::Value = serde_json::from_str(manifest_text)
        .map_err(|e| format!("package.json 不是合法 JSON：{e}"))?;

    let mut out = Vec::new();
    // 官方内置 bundle：版本锚在 dsh 安装目录，不进 profile node_modules，不实读。
    for b in crate::profiles::manifest_bundles(&pkg) {
        out.push(PluginEntry {
            name: b,
            kind: PluginKind::Bundle,
            installed_version: None,
            description: None,
            spec: None,
        });
    }
    // 第三方依赖：manifest 声明序（BTreeMap 字典序）+ 安装实况。
    // 官方桌面运行时内嵌的 desktop-packages 视为内置 Bundle，不作为外挂插件。
    if let Some(deps) = pkg.get("dependencies").and_then(|v| v.as_object()) {
        for (name, declared) in deps {
            let spec_str = declared.as_str();
            if crate::profiles::is_desktop_internal_spec(spec_str) {
                out.push(PluginEntry {
                    name: name.clone(),
                    kind: PluginKind::Bundle,
                    installed_version: None,
                    description: None,
                    spec: spec_str.map(str::to_string),
                });
            } else {
                let (version, description) = installed
                    .get(name)
                    .and_then(|text| text.as_deref())
                    .map(installed_info)
                    .unwrap_or((None, None));
                out.push(PluginEntry {
                    name: name.clone(),
                    kind: PluginKind::Dependency,
                    installed_version: version,
                    description,
                    spec: spec_str.map(str::to_string),
                });
            }
        }
    }
    Ok(out)
}

/// 已装包 `package.json` 原文 → `(version, description)`；缺失/损坏 → (None, None)
/// （清单容忍半初始化，与列表页口径一致）。
fn installed_info(pkg_text: &str) -> (Option<String>, Option<String>) {
    let Ok(pkg) = serde_json::from_str::<serde_json::Value>(pkg_text) else {
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

/// manifest 原文 → 依赖包名（字典序；缺失 `dependencies` = 空表，非法 JSON = Err）。
fn dependency_names(manifest_text: &str) -> Result<Vec<String>, String> {
    let pkg: serde_json::Value = serde_json::from_str(manifest_text)
        .map_err(|e| format!("package.json 不是合法 JSON：{e}"))?;
    Ok(pkg
        .get("dependencies")
        .and_then(|v| v.as_object())
        .map(|d| d.keys().cloned().collect())
        .unwrap_or_default())
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
    fn classifies_desktop_internal_packages_as_bundle() {
        let home = tmp();
        const DESKTOP_PKG: &str = r#"{
  "name": "@deepseek-ai/dsh-desktop-runtime",
  "dependencies": {
    "@deepseek-ai/cordis": "file:./desktop-packages/deepseek-ai-cordis-4.0.2.tgz",
    "user-plugin": "^1.0.0"
  },
  "dsh": { "profile": { "bundles": ["@deepseek-ai/dsh-base"] } }
}"#;
        write(&home.join("profiles/desktop/package.json"), DESKTOP_PKG);
        write(
            &home.join("profiles/desktop/node_modules/user-plugin/package.json"),
            r#"{"name":"user-plugin","version":"1.0.0","description":"用户外挂插件"}"#,
        );
        let list = list_profile_plugins(&home, "desktop").unwrap();
        // 1 bundle from dsh + 1 bundle from desktop-packages + 1 external dependency
        assert_eq!(list.len(), 3);
        let internal_cordis = list
            .iter()
            .find(|p| p.name == "@deepseek-ai/cordis")
            .unwrap();
        assert_eq!(internal_cordis.kind, PluginKind::Bundle);
        assert_eq!(
            internal_cordis.spec.as_deref(),
            Some("file:./desktop-packages/deepseek-ai-cordis-4.0.2.tgz")
        );

        let ext = list.iter().find(|p| p.name == "user-plugin").unwrap();
        assert_eq!(ext.kind, PluginKind::Dependency);
        assert_eq!(ext.installed_version.as_deref(), Some("1.0.0"));
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
// 与创建刀同链（profiles.rs run_toolchain_forward）：`dsh plugin --profile <名>
// add/remove/update <spec>` 原样转发 pnpm；pnpm 防御补齐复用创建同一函数。
// add 裸包名 dist-tag 坑（ledger 复现点 7）由 UI 引导带版本段规避；reconcile
// 会把声明 dsh.bundle 的新装依赖回写进 bundles（同复现点 7），装完刷新即见。

/// 插件操作结果：ok = dsh 退出 0 且未超时；detail 为人读文案（失败附输出尾部）。
/// pnpm 12 构建审批门已改为**默认批准**（build_policy 模块，ADR-0013）：操作前
/// 幂等写入 profile 的 `dangerouslyAllowAllBuilds: true`，结果里不再有逐包裁决载荷。
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

/// npm 包名/规格判别（纯函数，严格形态）：只接受纯 npm spec。更新检查的
/// 依赖过滤（`check_updates_blocking`）与选版本（`plugin_versions_blocking`）
/// 用它避免拿 `github:` 等非 npm 形态去打 registry。安装入口的宽口径见
/// [`validate_install_spec`]（ADR-0011，两谓词勿混用）。
///
/// spec 作为单个 argv 传给 dsh→pnpm（无 shell 参与，无注入面），仍须防两类
/// 滥用——① pnpm 旗标注入（前导 `-` 会被 pnpm 当参数，如 `--frozen-lockfile`）；
/// ② 控制字符/空白进日志与清单。允许 scope 包名（`@scope/name`）、版本段
/// （`@tag|精确|^~区间`，不含 `><`——需要语义区间时走终端，v1 不开）。
/// 与前端 lib/profiles.ts 的预检镜像同规则。
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
    validate_npm_body(spec)
}

/// 安装 spec 校验（纯函数，宽口径，安装/卸载/更新转发链专用——ADR-0011）：
/// 注入安全不变（单 argv、拒前导 `-`、拒空白/控制字符、拒 `><` 语义区间），
/// 放行线上 registry 实测三形态——npm spec（现规则）、`github:用户名/仓库名`
/// （可带 `#path:/子目录`、`#分支`、`#提交` 片段）、`https://…` tarball 直链。
/// 片段/URL 的语义合法性（分支是否存在、路径遍历等）归 pnpm/dsh，壳只守
/// 注入面。更新检查/选版本仍用 [`validate_plugin_spec`]，勿混用。
pub fn validate_install_spec(spec: &str) -> Result<(), String> {
    if spec.is_empty() {
        return Err("包名不能为空".to_string());
    }
    if spec.starts_with('-') {
        return Err("包名不能以 - 开头（会被当作命令参数）".to_string());
    }
    if spec.len() > 512 {
        return Err("安装来源过长（上限 512 字符）".to_string());
    }
    if let Some(rest) = spec.strip_prefix("github:") {
        return validate_github_body(rest);
    }
    if let Some(rest) = spec.strip_prefix("https://") {
        return validate_tarball_body(rest);
    }
    // npm 形态保持 214 上限（github/tarball 链接走 512 总长上限）
    if spec.len() > 214 {
        return Err("包名过长（npm 上限 214 字符）".to_string());
    }
    validate_npm_body(spec)
}

/// npm 形态字符集（长度上限由调用方按口径先行把关）。
fn validate_npm_body(spec: &str) -> Result<(), String> {
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

/// `github:` 之后的部分：`用户名/仓库名` + 可选 `#片段`（`#path:/…`、`#分支`、
/// `#分支&path:/…`）。字符集不含空白/引号/尖括号/分号；`#semver:` 等语义
/// 片段 v1 不开（走终端，同 `><` 区间口径）。
fn validate_github_body(rest: &str) -> Result<(), String> {
    let bad =
        || "GitHub 来源格式：github:用户名/仓库名，可带 #path:/子目录 或 #分支/#提交".to_string();
    let (owner_repo, frag) = match rest.split_once('#') {
        Some((body, f)) => (body, Some(f)),
        None => (rest, None),
    };
    let mut parts = owner_repo.split('/');
    let (owner, repo) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    if parts.next().is_some() || !is_repo_segment(owner) || !is_repo_segment(repo) {
        return Err(bad());
    }
    if let Some(f) = frag {
        if f.is_empty()
            || !f
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "_./:=&-".contains(c))
            || f.strip_prefix("path:").is_some_and(str::is_empty)
        {
            return Err(bad());
        }
    }
    Ok(())
}

/// `https://` 之后的部分：host 非空、其余走字符集（覆盖 query 串）。
fn validate_tarball_body(rest: &str) -> Result<(), String> {
    let bad = "安装链接仅支持 https:// 的 tarball 直链".to_string();
    if rest.split('/').next().unwrap_or("").is_empty()
        || !rest
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ":/._~#?&=%+-".contains(c))
    {
        return Err(bad);
    }
    Ok(())
}

fn is_repo_segment(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

/// 安装/卸载/更新（阻塞转发，IPC 层走 spawn_blocking；超时同创建 600s）。
/// profile 必须已物化（模板名先创建/首启）；spec 先过校验。
///
/// **世界择源**（ADR-0016 §5-a/c）：`world` 由 IPC 层按会话实际运行环境解析。
/// 本地世界 = 现状路径（宿主 fs + 宿主引擎）；WSL 世界 = 客体原语
/// （客体读 profile 清单 + 客体 dsh CLI 转发 + 客体侧单键写入），**绝不回落本地**。
pub fn mutate_plugin_blocking(
    op: PluginOp,
    profile: &str,
    spec: &str,
    data_dir: &Path,
    world: &crate::mgmt::World,
) -> Result<PluginOpOutcome, String> {
    crate::profiles::validate_profile_name(profile)?;
    // 安装/卸载/更新走宽口径三形态（ADR-0011）；更新检查/选版本仍严格 npm 判别
    validate_install_spec(spec)?;
    let args = [
        "plugin".to_string(),
        "--profile".to_string(),
        profile.to_string(),
        op.verb().to_string(),
        spec.to_string(),
    ];
    let log_path = data_dir.join("plugin-op.log");
    let run = match world {
        crate::mgmt::World::Local => {
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
            // pnpm 12 构建脚本默认批准（ADR-0013）：操作前幂等补写 profile 的
            // `dangerouslyAllowAllBuilds: true`，pnpm 不再进入审批门（复现点 12）。
            // 写失败只告警不阻断——操作本身可能根本不含构建脚本。
            crate::build_policy::ensure_profile_build_policy_best_effort(profile);
            crate::profiles::run_toolchain_forward(
                &crate::engines::resolve_toolchain(data_dir)?,
                &args,
                &home,
                &log_path,
                data_dir,
            )?
        }
        crate::mgmt::World::Wsl { distro } => {
            // 客体 profile 存在性（读原语；宿主 home 在 WSL 模式下**不是**这个世界）
            let rel = format!("profiles/{profile}/package.json");
            let exists = crate::guest::read_files(distro, std::slice::from_ref(&rel))?
                .into_iter()
                .next()
                .and_then(|(_, text)| text)
                .is_some();
            if !exists {
                return Err(format!(
                    "profile「{profile}」尚未初始化（客体 {distro} 内无 {rel}）——\
                     先在该发行版里创建或首启一次再管理插件"
                ));
            }
            // 客体侧单键写入（ADR-0016 §5-d）：客体 profile 由客体 dsh 物化，
            // 宿主这份写入器从未碰过它——不补写则带构建脚本的插件必撞 pnpm 审批门。
            crate::build_policy::ensure_profile_build_policy_in_guest_best_effort(distro, profile);
            crate::profiles::run_dsh_cli_in_guest(distro, &args, &log_path)?
        }
    };
    Ok(classify_op_outcome(op, profile, spec, &run))
}

/// 转发运行结果 → 插件操作结果（**纯函数**，本地 / 客体共用同一份分类与文案）：
/// 退出码 0 = 成功；超时指向网络与 registry；其余取输出尾部做人读诊断。
fn classify_op_outcome(
    op: PluginOp,
    profile: &str,
    spec: &str,
    run: &crate::profiles::ForwardRun,
) -> PluginOpOutcome {
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
    PluginOpOutcome { ok, detail }
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
    fn install_spec_accepts_registry_three_forms() {
        // npm 现规则原样
        for ok_spec in [
            "dsh-better-sidebar",
            "@scope/pkg",
            "pkg@0.16.1",
            "pkg@next",
            "pkg@^1.0.0",
        ] {
            assert!(validate_install_spec(ok_spec).is_ok(), "{ok_spec}");
        }
        // github：线上 registry 实测形态（ADR-0011）
        for ok_spec in [
            "github:CAI-MH/dsh-quality-review",
            "github:zhu1090093659/dsh-web-ui#path:/packages/dsh-pet",
            "github:owner/repo#main",
            "github:owner/repo#dev&path:/packages/p",
            "github:O-R.e1/r_2.N-x",
        ] {
            assert!(validate_install_spec(ok_spec).is_ok(), "{ok_spec}");
        }
        // tarball：registry 实测（成对引号由提取层剥离后到达）
        assert!(
            validate_install_spec("https://github.com/o/r/releases/latest/download/p.tgz").is_ok()
        );
        // github 链接超 214 但在 512 上限内合法（npm 形态仍守 214）
        let long_link = format!("github:o/{}", "r".repeat(300));
        assert!(validate_install_spec(&long_link).is_ok());
        assert!(validate_install_spec(&format!("a{}", "b".repeat(215))).is_err());
    }

    #[test]
    fn install_spec_rejects_injection_and_malformed() {
        for bad in [
            "",
            "-flag",
            "--frozen-lockfile",
            "pkg; rm -rf ~",
            "a b",
            "pkg\tx",
            "pkg@>=2",
            "pkg`id`",
            "pkg$(id)",
            // 非 npm 协议/URL 形态 v1 不开（fail-closed）
            "npm:o/r",
            "git+https://github.com/o/r",
            "http://x/y.tgz",
            // github 形态残缺/越界
            "github:",
            "github:o",
            "github:/r",
            "github:o/",
            "github:o/r/r2",
            "github:o/r#",
            "github:o/r#path:",
            "github:o/r#x;rm",
            "github:o/r#x y",
            // tarball 残缺
            "https://",
            "https:///x",
            // 总长上限
            &format!("a{}", "b".repeat(513)),
        ] {
            assert!(validate_install_spec(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn mutate_rejects_unmaterialized_profile_and_bad_spec_before_spawn() {
        // 未物化：先于任何 spawn/网络拒绝
        let data_dir = std::env::temp_dir().join("dsh-dock-op-test");
        let ghost = format!("dsh-dock-ghost-{}", std::process::id());
        let local = crate::mgmt::World::Local;
        assert!(
            mutate_plugin_blocking(PluginOp::Install, &ghost, "pkg", &data_dir, &local)
                .unwrap_err()
                .contains("尚未初始化")
        );
        // 非法 spec：同样先拒（伪 profile 名保证不触发 spawn）
        assert!(
            mutate_plugin_blocking(PluginOp::Install, &ghost, "-flag", &data_dir, &local).is_err()
        );
        // 非法 profile 名（路径遍历）：任何世界都在触达客体之前被拒
        let wsl = crate::mgmt::World::Wsl {
            distro: "Ubuntu".to_string(),
        };
        assert!(
            mutate_plugin_blocking(PluginOp::Install, "../escape", "pkg", &data_dir, &wsl).is_err()
        );
    }

    /// 客体档在非 Windows 上给**诚实错误**（客体只存在于 Windows），绝不静默回落
    /// 宿主世界执行——ADR-0016 §2.6。
    #[cfg(not(windows))]
    #[test]
    fn wsl_world_never_silently_falls_back_to_host_on_non_windows() {
        let data_dir = std::env::temp_dir().join("dsh-dock-op-test");
        let wsl = crate::mgmt::World::Wsl {
            distro: "Ubuntu".to_string(),
        };
        let err =
            mutate_plugin_blocking(PluginOp::Install, "web", "pkg", &data_dir, &wsl).unwrap_err();
        assert!(err.contains("仅在 Windows 宿主可用"), "{err}");
        let err = plugin_rows_blocking("web", &data_dir, &wsl).unwrap_err();
        assert!(err.contains("仅在 Windows 宿主可用"), "{err}");
        let err = list_profile_plugins_in_guest("Ubuntu", "web").unwrap_err();
        assert!(err.contains("仅在 Windows 宿主可用"), "{err}");
    }

    /// 转发结果分类（纯函数，本地/客体共用）：成功 / 超时 / 退出码非零三态文案。
    #[test]
    fn op_outcome_classification_covers_three_terminal_states() {
        use crate::profiles::ForwardRun;
        let ok = classify_op_outcome(
            PluginOp::Install,
            "web",
            "pkg@1.2.3",
            &ForwardRun {
                code: Some(0),
                timed_out: false,
                output: String::new(),
            },
        );
        assert!(ok.ok);
        assert!(ok.detail.contains("已安装 pkg@1.2.3"), "{}", ok.detail);

        let timed_out = classify_op_outcome(
            PluginOp::Remove,
            "web",
            "pkg",
            &ForwardRun {
                code: None,
                timed_out: true,
                output: String::new(),
            },
        );
        assert!(!timed_out.ok);
        assert!(timed_out.detail.contains("超时"), "{}", timed_out.detail);

        let failed = classify_op_outcome(
            PluginOp::Update,
            "web",
            "pkg",
            &ForwardRun {
                code: Some(1),
                timed_out: false,
                output: "l1\nl2\nERR_PNPM_IGNORED_BUILDS\n".to_string(),
            },
        );
        assert!(!failed.ok);
        assert!(failed.detail.contains("dsh 退出码 1"), "{}", failed.detail);
        assert!(
            failed.detail.contains("ERR_PNPM_IGNORED_BUILDS"),
            "输出尾部必须带上：{}",
            failed.detail
        );
    }

    /// 纯装配：bundle / 官方内嵌（desktop-packages）/ 已装依赖 / 未装依赖四态。
    #[test]
    fn assemble_plugin_entries_reuses_parsers_across_worlds() {
        let manifest = serde_json::json!({
            "dsh": { "profile": { "bundles": ["@deepseek-ai/dsh-base"] } },
            "dependencies": {
                "@deepseek-ai/cordis": "file:./desktop-packages/deepseek-ai-cordis-4.0.2.tgz",
                "dsh-pet": "github:o/r#path:/p",
                "dsh-missing": "^1.0.0"
            }
        })
        .to_string();
        let mut installed = BTreeMap::new();
        installed.insert(
            "dsh-pet".to_string(),
            Some(r#"{"version":"0.3.17","description":"宠物"}"#.to_string()),
        );
        installed.insert("dsh-missing".to_string(), None);
        let got = assemble_plugin_entries(&manifest, &installed).unwrap();
        let by_name = |n: &str| got.iter().find(|e| e.name == n).cloned().unwrap();
        assert_eq!(by_name("@deepseek-ai/dsh-base").kind, PluginKind::Bundle);
        assert_eq!(
            by_name("@deepseek-ai/cordis").kind,
            PluginKind::Bundle,
            "官方内嵌包不作为外挂插件"
        );
        let pet = by_name("dsh-pet");
        assert_eq!(pet.kind, PluginKind::Dependency);
        assert_eq!(pet.installed_version.as_deref(), Some("0.3.17"));
        assert_eq!(pet.description.as_deref(), Some("宠物"));
        assert_eq!(pet.spec.as_deref(), Some("github:o/r#path:/p"));
        let ghost = by_name("dsh-missing");
        assert_eq!(ghost.installed_version, None);
        assert_eq!(ghost.description, None);
        // 损坏清单 → Err（与宿主实现同口径）
        assert!(assemble_plugin_entries("not json", &installed).is_err());
    }

    /// patch 读改写内核（宿主 / 客体共用）：禁用追加/置键、启用回收只剩 id 的条目、
    /// **未改条目逐字节保真（含行间注释）**。
    #[test]
    fn patch_toggle_kernel_is_shared_by_host_and_guest() {
        let original = "# 用户手写注释\n# 第二行\n- id: a\n  foo: 1\n";
        let mut patch = PatchFile::from_text(original).unwrap();
        assert_eq!(patch.entries.len(), 1);

        // 禁用既有 id：只加 disabled 键
        apply_disabled_toggle(&mut patch, "a", true);
        let text = patch.render().unwrap();
        assert!(text.starts_with("# 用户手写注释\n# 第二行\n"), "{text}");
        assert!(text.contains("foo: 1"), "载荷字段不得丢：{text}");
        assert_eq!(
            PatchFile::from_text(&text)
                .unwrap()
                .entries
                .first()
                .and_then(|e| e.get("disabled"))
                .and_then(|d| d.as_bool()),
            Some(true)
        );

        // 启用回到原状，且只剩 id 的条目整条移除
        let mut patch = PatchFile::from_text(&text).unwrap();
        apply_disabled_toggle(&mut patch, "a", false);
        assert_eq!(patch.render().unwrap(), original);

        let mut patch = PatchFile::from_text("- id: b\n").unwrap();
        apply_disabled_toggle(&mut patch, "b", true);
        apply_disabled_toggle(&mut patch, "b", false);
        assert_eq!(patch.render().unwrap(), "[]\n");

        // 未命中且禁用 → 追加双键条目
        let mut patch = PatchFile::from_text("[]\n").unwrap();
        apply_disabled_toggle(&mut patch, "new-id", true);
        assert_eq!(patch.render().unwrap(), "- id: new-id\n  disabled: true\n");

        // 顶层不是数组 → 拒绝（不代 dsh 改写非 patch 方言）
        assert!(PatchFile::from_text("foo: 1\n").is_err());
    }

    /// **未改条目的行间注释必须存活**（本移植的核心动机，2026-09-11 合并 PR #13）：
    /// 分支旧内核 `render_patch_entries` 走整数组 `serde_yaml::to_string` ⇒ 除头部
    /// 连续注释块外**全部注释丢失**，而 `cordis.patch.yml` 的注释是用户人工资产。
    /// 本用例以"禁用 A 条目"为操作，断言**未改动的 B 条目行间注释逐字节仍在**。
    #[test]
    fn disabling_one_row_preserves_other_rows_inline_comments() {
        let original = "- id: a\n  foo: 1\n- id: b\n  # 用户手写：这个键不能删\n  bar: 2\n";
        let mut patch = PatchFile::from_text(original).unwrap();
        apply_disabled_toggle(&mut patch, "a", true);
        let text = patch.render().unwrap();
        assert!(
            text.contains("# 用户手写：这个键不能删"),
            "未改动条目的行间注释被吃掉了：{text}"
        );
        assert!(text.contains("bar: 2"), "未改动条目的载荷不得丢：{text}");
    }

    /// 行 id 校验（宿主 / 客体共用）：沿用宿主既有口径——路径分隔符与换行一律拒绝
    /// （行 id 来自 dump-config，绝不来自用户输入；这里只守"不进路径/不断行"）。
    #[test]
    fn row_id_validation_rejects_paths_and_newlines() {
        assert!(validate_row_id("dsh-pet").is_ok());
        assert!(validate_row_id("include:plugin-inventory").is_ok());
        assert!(validate_row_id("").is_err());
        assert!(validate_row_id("a/b").is_err());
        assert!(validate_row_id("a\nb").is_err());
    }

    /// 客体档更新检查 / 聚合在非 Windows 上给诚实错误（不静默回落宿主世界）。
    #[cfg(not(windows))]
    #[test]
    fn guest_update_check_and_aggregate_are_unavailable_off_windows() {
        let err = check_updates_blocking_in_guest("Ubuntu", "web").unwrap_err();
        assert!(err.contains("仅在 Windows 宿主可用"), "{err}");
        let err = aggregate_plugins_blocking_in_guest("Ubuntu").unwrap_err();
        assert!(err.contains("仅在 Windows 宿主可用"), "{err}");
    }

    /// 客体档切换在非 Windows 上给诚实错误（不静默回落宿主世界）。
    #[cfg(not(windows))]
    #[test]
    fn guest_toggle_is_unavailable_off_windows() {
        let err = set_plugin_disabled_in_guest("Ubuntu", "web", "id", true).unwrap_err();
        assert!(err.contains("仅在 Windows 宿主可用"), "{err}");
    }

    /// patch 原文解析与宿主文件版同源（同一份 `patch_entry_map_text`）。
    #[test]
    fn patch_entry_map_text_is_shared_by_host_and_guest() {
        let map = patch_entry_map_text("- id: a\n  disabled: true\n- id: a\n- id: b\n");
        assert_eq!(map.get("a"), Some(&(true, 2)));
        assert_eq!(map.get("b"), Some(&(false, 1)));
        assert!(patch_entry_map_text("这不是序列").is_empty());
        assert!(patch_entry_map_text("").is_empty());
    }

    #[test]
    fn toolchain_prefers_engine_when_engines_ready() {
        // 引擎 bin 内 node/dsh 落位即选引擎档（写全平台命名变体使单测跨平台
        // 成立）；path_env 故意给死目录——引擎分支不触系统探测的证明
        let data_dir = std::env::temp_dir().join("dsh-dock-engine-toolchain-test");
        let _ = std::fs::remove_dir_all(&data_dir);
        let bin = data_dir.join("engines/bin");
        std::fs::create_dir_all(&bin).unwrap();
        for name in ["node", "node.exe", "dsh", "dsh.cmd", "dsh.exe"] {
            std::fs::write(bin.join(name), "").unwrap();
        }
        match crate::engines::resolve_toolchain(&data_dir).unwrap() {
            crate::engines::DshToolchain::Engine { node_bin, dsh_bin } => {
                assert_eq!(node_bin.parent(), Some(bin.as_path()));
                assert_eq!(dsh_bin.parent(), Some(bin.as_path()));
            }
        }
        std::fs::remove_dir_all(&data_dir).ok();
    }

    #[test]
    fn toolchain_errors_actionable_when_engine_not_ready() {
        // 引擎未就绪（node/dsh 缺件）：错误文案必须指向引擎引导
        //（探测层退役后引擎是 dsh 操作的唯一来源，不再回退系统探测）
        let data_dir = std::env::temp_dir().join("dsh-dock-noengine-toolchain-test");
        let _ = std::fs::remove_dir_all(&data_dir);
        let err = crate::engines::resolve_toolchain(&data_dir).unwrap_err();
        assert!(err.contains("引擎未就绪"), "{err}");
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
    /// 补丁包（无自身行的 bundle）贡献的行 id 列表；普通行为空。
    /// 2026-09-08 补丁包开关（ADR-0009 第七次执行细则修订）：开关目标 = 普通插件
    /// 取自身行 id、补丁包取全部贡献行；id 来源仍定死 dump-config 行表（段落注释
    /// `# == <bundle>` 归属，台账复现点 13），不解析包内 patch 结构（第四次修订）。
    pub contributed_ids: Vec<String>,
}

/// 行表解析（带 bundle 段落归属）：dump-config 为每个 bundle 输出顶格段落注释
/// `# == <bundle 包名>`（机器生成，2026-09-08 实测 dsh 0.1.2-rc.1，台账复现点
/// 13），段内 `- id:` 行即该 bundle 声明/插入的行——补丁包（`dsh.bundle.patch`
/// 形态，自身不成行）由此映射「包 → 贡献行」，无需解析包内 patch 结构
/// （ADR 第四次修订口径）。行级扫描，不用 YAML 解析器：dump-config 输出含
/// `!!js` 标签等 serde_yaml 不保证友好的形态；行表形态是机器生成的稳定两行组
/// （`- id: X` 顶格 + `  name: Y` 二行缩进）。带引号的 name 去引号。
/// 返回三元组 = (行 id, 行 name, 段 bundle 名)。
fn parse_dump_rows_with_section(text: &str) -> Vec<(String, String, Option<String>)> {
    let mut rows = Vec::new();
    let mut pending_id: Option<String> = None;
    let mut section: Option<String> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# == ") {
            let mut name = rest.trim().to_string();
            // 2026-09-08 实机实测（复现点 13）：段落有 profile patch 命中时，段头
            // 追加 `, patched by <路径>` 后缀（如 `# == @tt-a1i/archify-dsh, patched
            // by /…/cordis.patch.yml`）——归属名只取逗号前，否则包名匹配失效。
            if let Some(idx) = name.find(", patched by ") {
                name.truncate(idx);
            }
            section = Some(name);
        } else if let Some(rest) = line.strip_prefix("- id: ") {
            pending_id = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("  name: ") {
            if let Some(id) = pending_id.take() {
                let name = rest.trim().trim_matches('\'').trim_matches('"').to_string();
                rows.push((id, name, section.clone()));
            }
        } else if !line.starts_with(' ') && !line.is_empty() {
            pending_id = None; // 顶格非空行打断配对（进入其他段落）
        }
    }
    rows
}

/// 由行表 + 依赖清单 + 自家 patch 状态构建行状态表（纯函数，可单测）。
/// - 行表条目（含内置 bundle 行）原样输出（现状行为，UI 按包名匹配自身行）；
/// - 补丁包合成条目：依赖包无自身行、但作为 dump 段落贡献了行 → 追加合成条目
///   `id` = 第一贡献行（兼容单目标引用）、`shell_disabled` = 贡献行**全部**被
///   禁用（部分禁用显示为启用，切换一次全量禁用——注释见前端）、
///   `patch_entries` = 贡献行自家 patch 条目数合计。
fn build_row_states(
    rows: &[(String, String, Option<String>)],
    deps: &[String],
    patch: &std::collections::BTreeMap<String, (bool, usize)>,
) -> Vec<PluginRowState> {
    let mut out: Vec<PluginRowState> = rows
        .iter()
        .map(|(id, name, _)| PluginRowState {
            id: id.clone(),
            pkg_name: name.clone(),
            shell_disabled: patch.get(id).map(|(d, _)| *d).unwrap_or(false),
            patch_entries: patch.get(id).map(|(_, n)| *n).unwrap_or(0),
            contributed_ids: Vec::new(),
        })
        .collect();
    // 段落归属：bundle 包名 → 该段贡献的行 id（行表内恒有序）。
    let mut by_bundle: std::collections::BTreeMap<&str, Vec<&str>> = Default::default();
    for (id, _, bundle) in rows {
        if let Some(b) = bundle {
            by_bundle.entry(b.as_str()).or_default().push(id.as_str());
        }
    }
    // 已由自身行表示的依赖包：不再合成（开关目标 = 自身行）。
    let own: std::collections::HashSet<&str> = rows
        .iter()
        .filter(|(_, name, _)| deps.iter().any(|d| d == name))
        .map(|(_, name, _)| name.as_str())
        .collect();
    for dep in deps {
        if own.contains(dep.as_str()) {
            continue;
        }
        let ids = match by_bundle.get(dep.as_str()) {
            Some(ids) if !ids.is_empty() => ids,
            _ => continue,
        };
        let shell_disabled = ids
            .iter()
            .all(|id| patch.get(*id).map(|(d, _)| *d).unwrap_or(false));
        let patch_entries = ids
            .iter()
            .map(|id| patch.get(*id).map(|(_, n)| *n).unwrap_or(0))
            .sum();
        out.push(PluginRowState {
            id: ids[0].to_string(),
            pkg_name: dep.clone(),
            shell_disabled,
            patch_entries,
            contributed_ids: ids.iter().map(|s| s.to_string()).collect(),
        });
    }
    out
}

/// 读 profile 自家 patch：id -> (含 disabled:true, 条目数)。文件缺失/损坏 →
/// 空表（与清单容忍半初始化同口径）。
fn patch_entry_map(patch_path: &Path) -> std::collections::BTreeMap<String, (bool, usize)> {
    let Ok(text) = std::fs::read_to_string(patch_path) else {
        return Default::default();
    };
    patch_entry_map_text(&text)
}

/// patch **原文** → id -> (含 disabled:true, 条目数)（纯函数：本地读文件、客体读
/// 原语共用同一份解析——ADR-0016 §3-A）。损坏/非序列 → 空表。
fn patch_entry_map_text(text: &str) -> std::collections::BTreeMap<String, (bool, usize)> {
    let Ok(v) = serde_yaml::from_str::<serde_yaml::Value>(text) else {
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
/// 2026-09-08 补丁包开关（ADR 第七次修订）：行表之外按依赖清单与 dump 段落
/// 归属合成补丁包条目（见 [`build_row_states`]），一次 spawn 全量拿到。
pub fn plugin_rows_blocking(
    profile: &str,
    data_dir: &Path,
    world: &crate::mgmt::World,
) -> Result<Vec<PluginRowState>, String> {
    crate::profiles::validate_profile_name(profile)?;
    let args = [
        "--profile".to_string(),
        profile.to_string(),
        "--dump-config".to_string(),
    ];
    let log_path = data_dir.join("plugin-rows.log");
    let (run, manifest_text, patch) = match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            let dir = home.join("profiles").join(profile);
            let manifest_path = dir.join("package.json");
            if !manifest_path.is_file() {
                return Err(format!("profile「{profile}」尚未初始化"));
            }
            let run = crate::profiles::run_toolchain_forward(
                &crate::engines::resolve_toolchain(data_dir)?,
                &args,
                &home,
                &log_path,
                data_dir,
            )?;
            // 自家 patch（缺失/损坏 = 空表，同清单容忍口径）
            let patch = patch_entry_map(&dir.join("cordis.patch.yml"));
            (run, fs_err(&manifest_path)?, patch)
        }
        crate::mgmt::World::Wsl { distro } => {
            // 客体批量读（一次往返）：清单 + 自家 patch（patch 缺失 = 空表，同宿主口径）
            let rels = [
                format!("profiles/{profile}/package.json"),
                format!("profiles/{profile}/cordis.patch.yml"),
            ];
            let mut got = crate::guest::read_files(distro, &rels)?.into_iter();
            let manifest = got.next().and_then(|(_, text)| text).ok_or_else(|| {
                format!("profile「{profile}」尚未初始化（客体 {distro} 内无 package.json）")
            })?;
            let patch_text = got.next().and_then(|(_, text)| text);
            let run = crate::profiles::run_dsh_cli_in_guest(distro, &args, &log_path)?;
            (
                run,
                manifest,
                patch_entry_map_text(patch_text.as_deref().unwrap_or("")),
            )
        }
    };
    if run.timed_out || run.code != Some(0) {
        return Err(format!(
            "行表查询失败（dsh 退出码 {}）",
            run.code
                .map(|c| c.to_string())
                .unwrap_or_else(|| "未知".into())
        ));
    }
    let deps = dependency_names(&manifest_text)?;
    Ok(build_row_states(
        &parse_dump_rows_with_section(&run.output),
        &deps,
        &patch,
    ))
}

/// `cordis.patch.yml` 的**共享读写器**（2026-09-11，task-26）。
///
/// **两处必须同源（钉死理由）**：同一份用户数据 `profiles/<名>/cordis.patch.yml`
/// 有两条写入路径——插件中心（`set_plugin_disabled` / `copy_config_entries`）与
/// MCP 管理（`mcp.rs` 的 `save_mcp_server` / `delete_mcp_server`）。它们曾各写各的，
/// 于是**无声漂移成两套行为**：一边保留头部注释但**非原子写、无备份**，另一边
/// **原子写但注释全丢**（且因经 `serde_json::Value` 中转，键序被字典序打乱、
/// 缩进被重排）。两条路径都"能跑"，只在用户的注释被吃掉时才暴露——而
/// **YAML 注释是用户人工资产，丢了不可逆**。故：本结构是唯一实现，任何触碰该文件的
/// 路径都必须调用它，**不得再写第二份**。
///
/// 保真口径：
/// - **未改动的条目逐字节原样回填**——其行间注释、缩进、键序、引号风格全保；
/// - 只有**被改动的条目**重新序列化（其内部注释无法保留：文件层无 CST 解析器，
///   本仓库不引新依赖；这是已知且有意的代价，MCP 条目本身由壳机器生成）；
/// - 文件**首部连续注释块**在任何情况下都保住；
/// - 写入前**先备份**（`fs_backup::backup_before_overwrite`，AGENTS §6 已登记），
///   写入走**原子替换**（tmp + rename，与 settings / credentials 同口径）。
pub(crate) struct PatchFile {
    /// 首部连续 `#` / 空行块（原样前置）。
    header: String,
    /// 第一个顶层条目之前的其它内容（如 `---` 文档标记）；通常为空。
    preamble: String,
    /// 每条目的原文片段（含前导 `- `）；`None` = 本次新构造或已改写 → 写时需重新序列化。
    raw: Vec<Option<String>>,
    /// 顶层数组条目（`serde_yaml::Mapping` 基于 IndexMap，**插入序保真**）。
    pub(crate) entries: Vec<serde_yaml::Value>,
}

impl PatchFile {
    /// 读 patch 文件（缺失 → 调用方用 [`PatchFile::empty`]）。
    /// 顶层数组之外还有内容 → 拒绝（patch 方言即数组；不代 dsh 生成三件套）。
    pub(crate) fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("读取 {} 失败：{e}", path.display()))?;
        Self::from_text(&text)
    }

    /// 空载体（目标文件尚不存在时的首次写入）。
    pub(crate) fn empty() -> Self {
        Self {
            header: String::new(),
            preamble: String::new(),
            raw: Vec::new(),
            entries: Vec::new(),
        }
    }

    /// 从**文本**构造（宿主读文件 / 客体读原语共用）。
    ///
    /// 这是「宿主与客体共用同一份保真语义」的入口：客体内写 `cordis.patch.yml` 走不了
    /// [`PatchFile::read`]/[`PatchFile::write`]（那是宿主文件系统），但可以走
    /// `读客体原文 → from_text → 变更 → render → 客体备份 + 客体原子写`。
    /// **禁止再写第二份解析/渲染实现**——2026-09-11 合并 PR #13 时，客体档曾自带
    /// `parse_patch_entries`/`render_patch_entries`（整数组重序列化 ⇒ 除头部连续注释块
    /// 外**全部注释丢失**），与本结构的保真口径漂移成两套语义，与 task-26 修掉的
    /// 那个 bug 同源。
    pub(crate) fn from_text(text: &str) -> Result<Self, String> {
        // 头部 = 从首行起连续 `#` 行与其间空行（用户可见文档，序列化会丢）。
        let mut header = String::new();
        let mut body_start = 0usize;
        for line in text.split_inclusive('\n') {
            let t = line.trim_end_matches(['\n', '\r']);
            if t.starts_with('#') || t.trim().is_empty() {
                header.push_str(line);
                body_start += line.len();
            } else {
                break;
            }
        }
        let body = &text[body_start..];
        let entries = parse_patch_body(body)?;

        // 按顶格 `- ` 切出条目原文片段；与解析条目数不一致（块标量里出现顶格
        // `- ` 等罕见形态）→ 放弃逐条保真，退回整文件重序列化（收窄前行为，绝不写坏）。
        let starts = top_level_item_starts(body);
        let (preamble, raw) = if starts.len() == entries.len() {
            let mut raw = Vec::with_capacity(starts.len());
            for (i, &s) in starts.iter().enumerate() {
                let e = starts.get(i + 1).copied().unwrap_or(body.len());
                raw.push(Some(body[s..e].to_string()));
            }
            let pre = body[..starts.first().copied().unwrap_or(0)].to_string();
            (pre, raw)
        } else {
            (String::new(), vec![None; entries.len()])
        };

        Ok(Self {
            header,
            preamble,
            raw,
            entries,
        })
    }

    /// 逐条可变访问；闭包返回 `true` = 该条已被改写（原文保真失效，写时重新序列化）。
    pub(crate) fn for_each_entry_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(usize, &mut serde_yaml::Value) -> bool,
    {
        debug_assert_eq!(self.raw.len(), self.entries.len());
        for i in 0..self.entries.len() {
            let changed = f(i, &mut self.entries[i]);
            if changed {
                self.raw[i] = None;
            }
        }
    }

    /// 追加新条目（原文保真天然失效：新条目无原文）。
    pub(crate) fn push(&mut self, entry: serde_yaml::Value) {
        self.raw.push(None);
        self.entries.push(entry);
    }

    /// 按谓词保留条目（`raw` 同步增删，**始终与 `entries` 同长同序**）。
    pub(crate) fn retain<F>(&mut self, keep: F)
    where
        F: Fn(&serde_yaml::Value) -> bool,
    {
        let mut i = 0;
        while i < self.entries.len() {
            if keep(&self.entries[i]) {
                i += 1;
            } else {
                self.entries.remove(i);
                self.raw.remove(i);
            }
        }
    }

    /// 渲染为文件文本（**纯函数**，不触文件系统；客体侧据此投递）。
    pub(crate) fn render(&self) -> Result<String, String> {
        let mut out = String::with_capacity(self.header.len() + 1024);
        out.push_str(&self.header);
        if self.entries.is_empty() {
            // 空数组补 `[]\n`（保持单文档可解析；沿用既有口径）。
            if !out.ends_with("[]\n") {
                out.push_str("[]\n");
            }
        } else {
            out.push_str(&self.preamble);
            for (i, e) in self.entries.iter().enumerate() {
                match self.raw.get(i).and_then(|r| r.as_deref()) {
                    Some(raw) => out.push_str(raw),
                    None => out.push_str(&serialize_patch_item(e)?),
                }
            }
        }
        Ok(out)
    }

    /// 写回：渲染 → 覆写前**备份**（fail-closed）→ **原子替换**。
    pub(crate) fn write(&self, path: &Path) -> Result<(), String> {
        let out = self.render()?;
        crate::fs_backup::backup_before_overwrite(path)?;
        atomic_replace(path, &out)
    }
}

/// 解析顶层数组（patch 方言）。空 / 仅空白 / `null` → 空数组。
fn parse_patch_body(body: &str) -> Result<Vec<serde_yaml::Value>, String> {
    match serde_yaml::from_str::<serde_yaml::Value>(body) {
        Ok(v) if v.is_null() => Ok(Vec::new()),
        Ok(v) => v
            .as_sequence()
            .cloned()
            .ok_or_else(|| "cordis.patch.yml 顶层数组之外还有内容——拒绝写入".to_string()),
        Err(e) => Err(format!("cordis.patch.yml 解析失败：{e}")),
    }
}

/// 顶格（第 0 列）`- ` / `-` 行的字节起点 = 顶层条目边界。
/// 缩进的 `-` 属条目内部（映射值 / 块标量内容），不算边界。
fn top_level_item_starts(body: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut off = 0usize;
    for line in body.split_inclusive('\n') {
        let t = line.trim_end_matches(['\n', '\r']);
        if t == "-" || t.starts_with("- ") {
            starts.push(off);
        }
        off += line.len();
    }
    starts
}

/// 单个条目 → YAML 片段（序列化 `[v]` 得到带 `- ` 前缀的片段，去掉可能的文档标记）。
fn serialize_patch_item(v: &serde_yaml::Value) -> Result<String, String> {
    let text = serde_yaml::to_string(std::slice::from_ref(v))
        .map_err(|e| format!("序列化条目失败：{e}"))?;
    let text = text.strip_prefix("---\n").unwrap_or(&text);
    if text.ends_with('\n') {
        Ok(text.to_string())
    } else {
        Ok(format!("{text}\n"))
    }
}

/// 原子替换：同目录临时文件 + rename（与 settings / credentials 同口径）。
fn atomic_replace(path: &Path, content: &str) -> Result<(), String> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "patch.yml".to_string());
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = dir.join(format!(".{name}.tmp.{}.{nanos}", std::process::id()));
    if let Err(e) = std::fs::write(&tmp, content) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("写入临时 patch 失败：{e}"));
    }
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("覆盖 {} 失败：{e}", path.display()));
    }
    Ok(())
}

/// 禁用/启用切换（patch 写入例外 #3，读改写顶层数组；未改条目原文保真见
/// [`PatchFile`]）。
/// 禁用：id 条目存在则仅置 disabled 键，否则追加 `{id, disabled}` 双键条目；
/// 启用：移除 disabled 键，条目只剩 id 则整条移除。
pub fn set_plugin_disabled(
    home: &Path,
    profile: &str,
    row_id: &str,
    disabled: bool,
) -> Result<(), String> {
    crate::profiles::validate_profile_name(profile)?;
    validate_row_id(row_id)?;
    let patch_path = home.join("profiles").join(profile).join("cordis.patch.yml");
    let mut patch = PatchFile::read(&patch_path)?;
    apply_disabled_toggle(&mut patch, row_id, disabled);
    patch.write(&patch_path)
}

/// **客体档孪生**（ADR-0016：让已下沉的插件中心在 WSL 世界可用——装上了却关不掉
/// 是半截功能）：读客体 patch 原文（读原语）→ 同一份 [`apply_disabled_toggle`] →
/// 渲染 → 客体侧原子写。语义逐项对齐宿主实现（patch 写入例外 #3，ADR-0009）。
pub fn set_plugin_disabled_in_guest(
    distro: &str,
    profile: &str,
    row_id: &str,
    disabled: bool,
) -> Result<(), String> {
    crate::profiles::validate_profile_name(profile)?;
    validate_row_id(row_id)?;
    let rel = format!("profiles/{profile}/cordis.patch.yml");
    let text = crate::guest::read_files(distro, std::slice::from_ref(&rel))?
        .into_iter()
        .next()
        .and_then(|(_, text)| text)
        .ok_or_else(|| {
            format!("读取 {distro}:{rel} 失败：文件不存在（三件套不完整，不代 dsh 生成）")
        })?;
    let mut patch = PatchFile::from_text(&text)?;
    apply_disabled_toggle(&mut patch, row_id, disabled);
    let next = patch.render()?;
    if next == text {
        return Ok(()); // 无变化零写入（免 mtime 抖动，同 ADR-0013 纪律）
    }
    // 覆写前备份：与宿主 `PatchFile::write` 同口径（fail-closed）。
    crate::guest::backup_file(distro, &rel)?;
    crate::guest::write_home_files(distro, &[(rel, next)])
}

/// 行 id 合法性（宿主 / 客体共用；行 id 来自 dump-config，不可从包名推导）。
fn validate_row_id(row_id: &str) -> Result<(), String> {
    if row_id.is_empty() || row_id.contains(['/', '\n']) {
        return Err("行 id 非法".to_string());
    }
    Ok(())
}

/// 纯变换（宿主 / 客体共用）：禁用 → id 条目仅置 `disabled` 键（不存在则追加
/// `{id, disabled}` 双键条目）；启用 → 移除 `disabled` 键，条目只剩 id 则整条移除。
fn apply_disabled_toggle(patch: &mut PatchFile, row_id: &str, disabled: bool) {
    let id_key = serde_yaml::Value::String("id".into());
    let disabled_key = serde_yaml::Value::String("disabled".into());
    let mut found = false;
    // 走 `for_each_entry_mut`：**只有真正被改写的条目**才失去原文保真，其余条目
    // 连同其行间注释逐字节回填（旧内核整数组重序列化 ⇒ 全文件注释丢失）。
    patch.for_each_entry_mut(|_, entry| {
        let Some(m) = entry.as_mapping_mut() else {
            return false;
        };
        if m.get(&id_key).and_then(|v| v.as_str()) != Some(row_id) {
            return false;
        }
        found = true;
        if disabled {
            if m.get(&disabled_key) == Some(&serde_yaml::Value::Bool(true)) {
                return false; // 已是目标态：不改写，保住本条目原文
            }
            m.insert(disabled_key.clone(), serde_yaml::Value::Bool(true));
            true
        } else {
            m.remove(&disabled_key).is_some()
        }
    });
    if !found && disabled {
        let mut m = serde_yaml::Mapping::new();
        m.insert(
            id_key.clone(),
            serde_yaml::Value::String(row_id.to_string()),
        );
        m.insert(disabled_key, serde_yaml::Value::Bool(true));
        patch.push(serde_yaml::Value::Mapping(m));
    }
    // 启用后只剩 `id` 键的条目整条移除（恢复原状）——**分支既有语义，必须保留**：
    // 本函数曾因只在 `for_each_entry_mut` 里 remove 键而丢掉这一步，
    // 被 `patch_toggle_kernel_is_shared_by_host_and_guest` 与基线
    // `disable_appends_entry_enabling_removes_it` 双双抓住。
    if !disabled {
        patch.retain(|e| {
            e.as_mapping()
                .map(|m| m.len() > 1 || !m.contains_key(&id_key))
                .unwrap_or(true)
        });
    }
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
        let rows = parse_dump_rows_with_section(dump);
        assert_eq!(
            rows,
            vec![
                (
                    "llm-pi-ai".into(),
                    "@deepseek-ai/dsh-llm-pi-ai".into(),
                    None
                ),
                (
                    "llm-commandcode".into(),
                    "@mars-sea/dsh-commandcode-provider".into(),
                    None
                ),
            ]
        );
    }

    /// 补丁包开关（2026-09-08，ADR-0009 第七次修订）：段落注释 `# == <bundle>`
    /// 归属解析——段内行归该 bundle 所有（含其 insert 贡献的行）。
    #[test]
    fn parse_dump_rows_with_section_tracks_bundle_sections() {
        let dump = "# == @openviking/dsh-memory-plugin\n- id: openviking-memory\n  name: '@deepseek-ai/cordis-plugin-group'\n  group: true\n  config:\n    - id: openviking-memory-runtime\n      name: '@openviking/dsh-memory-plugin'\n# == dsh-better-sidebar\n- id: better-sidebar\n  name: dsh-better-sidebar\n";
        let rows = parse_dump_rows_with_section(dump);
        assert_eq!(
            rows,
            vec![
                (
                    "openviking-memory".into(),
                    "@deepseek-ai/cordis-plugin-group".into(),
                    Some("@openviking/dsh-memory-plugin".into())
                ),
                (
                    "better-sidebar".into(),
                    "dsh-better-sidebar".into(),
                    Some("dsh-better-sidebar".into())
                ),
            ]
        );
    }

    /// 段落头带 `, patched by <路径>` 后缀（profile patch 命中该段时，2026-09-08
    /// 实机实测）：归属名取逗号前，否则补丁包匹配失效。
    #[test]
    fn parse_dump_rows_with_section_strips_patched_by_suffix() {
        let dump = "# == @tt-a1i/archify-dsh, patched by /Users/x/profiles/web/cordis.patch.yml\n- id: archify-skill-filesystem\n  name: '@deepseek-ai/dsh-skill-filesystem'\n  disabled: true\n";
        let rows = parse_dump_rows_with_section(dump);
        assert_eq!(
            rows,
            vec![(
                "archify-skill-filesystem".into(),
                "@deepseek-ai/dsh-skill-filesystem".into(),
                Some("@tt-a1i/archify-dsh".into())
            )]
        );
    }

    /// 补丁包合成：无自身行的依赖包 → 合成条目（id = 第一贡献行、全禁用才
    /// shell_disabled、patch_entries 合计）；有自身行的依赖与无归属依赖不变。
    #[test]
    fn build_row_states_synthesizes_patch_bundles() {
        let rows = vec![
            (
                "agy-link".into(),
                "dsh-agy-link".into(),
                Some("dsh-agy-link".into()),
            ),
            (
                "archify-skill-filesystem".into(),
                "@deepseek-ai/dsh-skill-filesystem".into(),
                Some("@tt-a1i/archify-dsh".into()),
            ),
            (
                "openviking-memory".into(),
                "@deepseek-ai/cordis-plugin-group".into(),
                Some("@openviking/dsh-memory-plugin".into()),
            ),
            (
                "other-insert".into(),
                "@deepseek-ai/dsh-other".into(),
                Some("@mars-sea/dsh-commandcode-provider".into()),
            ),
        ];
        let deps = vec![
            "dsh-agy-link".to_string(),
            "@tt-a1i/archify-dsh".to_string(),
            "@openviking/dsh-memory-plugin".to_string(),
            "@mars-sea/dsh-commandcode-provider".to_string(),
            "not-installed".to_string(),
        ];
        let mut patch = std::collections::BTreeMap::new();
        patch.insert("archify-skill-filesystem".to_string(), (true, 0usize));
        patch.insert("openviking-memory".to_string(), (true, 0usize));
        let states = build_row_states(&rows, &deps, &patch);

        // 普通行（自身行 name == 依赖包）：原有行为不变
        let agy = states
            .iter()
            .find(|s| s.pkg_name == "dsh-agy-link")
            .unwrap();
        assert_eq!(agy.id, "agy-link");
        assert!(agy.contributed_ids.is_empty());

        // 补丁包 A：单贡献行，禁用 → 合成条目禁用
        let arch = states
            .iter()
            .find(|s| s.pkg_name == "@tt-a1i/archify-dsh")
            .unwrap();
        assert_eq!(arch.id, "archify-skill-filesystem");
        assert_eq!(arch.contributed_ids, vec!["archify-skill-filesystem"]);
        assert!(arch.shell_disabled);

        // 补丁包 B：贡献行被禁用 → 合成条目禁用（组 id 即开关目标）
        let m = states
            .iter()
            .find(|s| s.pkg_name == "@openviking/dsh-memory-plugin")
            .unwrap();
        assert_eq!(m.id, "openviking-memory");
        assert_eq!(m.contributed_ids, vec!["openviking-memory"]);
        assert!(m.shell_disabled);

        // 贡献行依赖（自身无行）：同样合成（@mars-sea 段内 other-insert 是它的 insert）
        let mars = states
            .iter()
            .find(|s| s.pkg_name == "@mars-sea/dsh-commandcode-provider")
            .unwrap();
        assert_eq!(mars.id, "other-insert");
        assert_eq!(mars.contributed_ids, vec!["other-insert"]);
        assert!(!mars.shell_disabled);

        // 未安装依赖：无段落归属 → 无条目
        assert!(states.iter().all(|s| s.pkg_name != "not-installed"));
    }

    /// 补丁包部分禁用（手改 patch 中间态）：all 语义 = 显示启用，切换一次全量禁用。
    #[test]
    fn build_row_states_partial_disabled_counts_as_enabled() {
        let rows = vec![
            ("row-a".into(), "pkg-a".into(), Some("@dep/patched".into())),
            ("row-b".into(), "pkg-b".into(), Some("@dep/patched".into())),
        ];
        let deps = vec!["@dep/patched".to_string()];
        let mut patch = std::collections::BTreeMap::new();
        patch.insert("row-a".to_string(), (true, 0usize));
        patch.insert("row-b".to_string(), (false, 0usize));
        let states = build_row_states(&rows, &deps, &patch);
        let patched = states
            .iter()
            .find(|s| s.pkg_name == "@dep/patched")
            .unwrap();
        assert!(!patched.shell_disabled);
        assert_eq!(patched.contributed_ids, vec!["row-a", "row-b"]);
        assert_eq!(patched.id, "row-a");
        // patch_entries 为合计
        assert_eq!(patched.patch_entries, 0);
    }

    /// 有自身行的依赖包即使贡献了其他行，也不产生合成条目（开关目标 = 自身行）。
    #[test]
    fn build_row_states_own_row_wins_over_section() {
        let rows = vec![
            (
                "llm-commandcode".into(),
                "@mars-sea/dsh-commandcode-provider".into(),
                Some("@mars-sea/dsh-commandcode-provider".into()),
            ),
            (
                "other-insert".into(),
                "@deepseek-ai/dsh-other".into(),
                Some("@mars-sea/dsh-commandcode-provider".into()),
            ),
        ];
        let deps = vec!["@mars-sea/dsh-commandcode-provider".to_string()];
        let patch = Default::default();
        let states = build_row_states(&rows, &deps, &patch);
        let mars = states
            .iter()
            .find(|s| s.pkg_name == "@mars-sea/dsh-commandcode-provider")
            .unwrap();
        assert_eq!(mars.id, "llm-commandcode");
        assert!(mars.contributed_ids.is_empty());
        assert_eq!(
            states
                .iter()
                .filter(|s| s.pkg_name == "@mars-sea/dsh-commandcode-provider")
                .count(),
            1
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
    check_updates_from(updatable_deps(list_profile_plugins(home, profile)?))
}

/// **客体档孪生**（ADR-0016 §5 读侧下沉）：已装版本来自**客体**清单（客体读原语），
/// registry 查询仍是 `updates.rs` 唯一网络面——ADR-0016 §1 明示"市场 registry 拉取
/// 与模式无关"，故这里不新增网络用途，只是比对基准换成客体世界。
pub fn check_updates_blocking_in_guest(
    distro: &str,
    profile: &str,
) -> Result<PluginUpdateReport, String> {
    check_updates_from(updatable_deps(list_profile_plugins_in_guest(
        distro, profile,
    )?))
}

/// 可查更新的依赖（宿主 / 客体共用）：第三方且已装出实际版本。
fn updatable_deps(entries: Vec<PluginEntry>) -> Vec<PluginEntry> {
    entries
        .into_iter()
        .filter(|p| p.kind == PluginKind::Dependency && p.installed_version.is_some())
        .collect()
}

/// 更新检查内核（宿主 / 客体共用）：逐依赖查 registry 最新版，与当前版本比较。
fn check_updates_from(deps: Vec<PluginEntry>) -> Result<PluginUpdateReport, String> {
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
    /// package.json 依赖声明值原样（git/tarball 来源 = 安装 spec）——前端
    /// 以 spec 对齐市场条目（市场名 ≠ 真实包名的 git 形态，ADR-0011）。
    pub spec: Option<String>,
}

/// 插件总览聚合（只读纯文件扫描，零 dsh 子进程、零网络）：全部已物化 profile
/// 的第三方依赖按包名归组。单 profile 清单损坏 → 跳过该 profile（聚合不让
/// 单点损坏全页失败，与列表页容忍口径一致）。
pub fn aggregate_plugins_blocking(home: &Path) -> Vec<AggregatePlugin> {
    let profiles = crate::profiles::scan_profiles(home);
    aggregate_from(&profiles, |name| list_profile_plugins(home, name))
}

/// **客体档孪生**（ADR-0016 §5 读侧下沉）：profile 清单与各 profile 的插件清单都
/// 取自客体（纯读，零 dsh 子进程、零网络），归组逻辑走同一份 [`aggregate_from`]。
pub fn aggregate_plugins_blocking_in_guest(distro: &str) -> Result<Vec<AggregatePlugin>, String> {
    let profiles = crate::profiles::scan_profiles_in_guest(distro)?;
    Ok(aggregate_from(&profiles, |name| {
        list_profile_plugins_in_guest(distro, name)
    }))
}

/// 聚合内核（宿主 / 客体共用）：`lister` 给出某 profile 的插件清单（世界由调用方定）。
fn aggregate_from(
    profiles: &[crate::profiles::ProfileSummary],
    lister: impl Fn(&str) -> Result<Vec<PluginEntry>, String>,
) -> Vec<AggregatePlugin> {
    let mut by_name: std::collections::BTreeMap<String, AggregatePlugin> = Default::default();
    for p in profiles {
        if !p.materialized {
            continue;
        }
        let entries = match lister(&p.name) {
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
                spec: e.spec,
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
    // 行 id 定位：dump-config 来源 profile（一次 spawn 全量行表，秒级）。
    // 本函数为本地世界分支（WSL 客体分支见 `copy_plugin_config_in_guest`）。
    let row_id = plugin_rows_blocking(source, data_dir, &crate::mgmt::World::Local)?
        .into_iter()
        .find(|r| r.pkg_name == package)
        .map(|r| r.id)
        .ok_or_else(|| {
            format!("来源 profile「{source}」的行表中没有插件「{package}」——无可搬移的配置行")
        })?;
    copy_config_entries(home, source, target, package, &row_id)
}

/// 纯逻辑：从来源 patch 原文提取 row_id 条目并追加到目标 patch 原文（宿主/客体共用）。
/// 返回 (CopyConfigOutcome, Option<新目标原文>)。若已存在则新目标原文为 None。
fn apply_copy_config_entries(
    source_text: &str,
    target_text: &str,
    source: &str,
    target: &str,
    package: &str,
    row_id: &str,
) -> Result<(CopyConfigOutcome, Option<String>), String> {
    let source_patch = PatchFile::from_text(source_text)?;
    let source_entries = entries_with_id(&source_patch.entries, row_id);
    if source_entries.is_empty() {
        return Err(format!(
            "来源 profile「{source}」的 cordis.patch.yml 没有「{package}」（行 id {row_id}）的配置条目"
        ));
    }
    let mut target_patch = PatchFile::from_text(target_text)?;
    if !entries_with_id(&target_patch.entries, row_id).is_empty() {
        return Ok((
            CopyConfigOutcome {
                copied: 0,
                skipped_existing: true,
                detail: format!(
                    "目标 profile「{target}」已有「{package}」的配置行——为不覆盖既有配置，本次未复制"
                ),
            },
            None,
        ));
    }
    let copied = source_entries.len();
    for e in source_entries {
        target_patch.push(e);
    }
    let next = target_patch.render()?;
    Ok((
        CopyConfigOutcome {
            copied,
            skipped_existing: false,
            detail: format!(
                "已把「{package}」的 {copied} 条配置行从「{source}」原样复制到「{target}」——重启「{target}」后生效。"
            ),
        },
        Some(next),
    ))
}

/// 复制的文件层核心（行 id 已定位；与 spawn 边界分离便于单测）。
fn copy_config_entries(
    home: &Path,
    source: &str,
    target: &str,
    package: &str,
    row_id: &str,
) -> Result<CopyConfigOutcome, String> {
    let src_path = home.join("profiles").join(source).join("cordis.patch.yml");
    let tgt_path = home.join("profiles").join(target).join("cordis.patch.yml");
    let src_text = std::fs::read_to_string(&src_path)
        .map_err(|e| format!("读取 {} 失败：{e}", src_path.display()))?;
    let tgt_text = std::fs::read_to_string(&tgt_path)
        .map_err(|e| format!("读取 {} 失败：{e}", tgt_path.display()))?;
    let (outcome, updated) =
        apply_copy_config_entries(&src_text, &tgt_text, source, target, package, row_id)?;
    if let Some(next) = updated {
        // 覆写前备份（fail-closed）+ 原子替换，与 `PatchFile::write` 同口径。
        crate::fs_backup::backup_before_overwrite(&tgt_path)?;
        atomic_replace(&tgt_path, &next)?;
    }
    Ok(outcome)
}

/// 客体档插件配置复制（P2 下沉）：在 WSL 模式下读取来源与目标 patch 原文，
/// 通过 pure 函数追加条目并原子写回目标客体 patch 文件。
pub fn copy_plugin_config_in_guest(
    distro: &str,
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
    let src_pkg_rel = format!("profiles/{source}/package.json");
    let tgt_pkg_rel = format!("profiles/{target}/package.json");
    let src_patch_rel = format!("profiles/{source}/cordis.patch.yml");
    let tgt_patch_rel = format!("profiles/{target}/cordis.patch.yml");
    let files = crate::guest::read_files(
        distro,
        &[
            src_pkg_rel.clone(),
            tgt_pkg_rel.clone(),
            src_patch_rel.clone(),
            tgt_patch_rel.clone(),
        ],
    )?;
    let mut map: std::collections::HashMap<String, Option<String>> = files.into_iter().collect();
    if map.get(&src_pkg_rel).and_then(|o| o.as_ref()).is_none() {
        return Err(format!("profile「{source}」尚未初始化"));
    }
    if map.get(&tgt_pkg_rel).and_then(|o| o.as_ref()).is_none() {
        return Err(format!("profile「{target}」尚未初始化"));
    }
    let src_patch_text = map
        .remove(&src_patch_rel)
        .flatten()
        .ok_or_else(|| "来源 profile 尚无 cordis.patch.yml——无可搬移的配置层".to_string())?;
    let tgt_patch_text = map
        .remove(&tgt_patch_rel)
        .flatten()
        .ok_or_else(|| "目标 profile 尚无 cordis.patch.yml——无可搬移的配置层".to_string())?;

    let row_id = plugin_rows_blocking(
        source,
        data_dir,
        &crate::mgmt::World::Wsl {
            distro: distro.to_string(),
        },
    )?
    .into_iter()
    .find(|r| r.pkg_name == package)
    .map(|r| r.id)
    .ok_or_else(|| {
        format!("来源 profile「{source}」的行表中没有插件「{package}」——无可搬移的配置行")
    })?;

    let (outcome, updated) = apply_copy_config_entries(
        &src_patch_text,
        &tgt_patch_text,
        source,
        target,
        package,
        &row_id,
    )?;
    if let Some(next) = updated {
        // 覆写前备份（fail-closed）：与 `PatchFile::write` 及另外三条客体 patch 写入
        // 路径同口径。**否则这是唯一一条不留备份的客体 patch 覆写**（2026-09-11
        // 合并 PR #13 时逐路径审计发现）。
        crate::guest::backup_file(distro, &tgt_patch_rel)?;
        crate::guest::write_home_files(distro, &[(tgt_patch_rel, next)])?;
    }
    Ok(outcome)
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
    fn aggregate_carries_declared_spec_for_git_form_deps() {
        // git 来源：真实包名（@scope/pet）≠ 市场展示名（pet）——声明值即安装
        // spec，是前端对齐市场条目的连接键（ADR-0011）。
        let home = tmp();
        write(
            &home.join("profiles/web/package.json"),
            r#"{"dependencies":{"@scope/pet":"github:o/r#path:/packages/pet"}}"#,
        );
        write(
            &home.join("profiles/web/node_modules/@scope/pet/package.json"),
            r#"{"name":"@scope/pet","version":"0.3.18"}"#,
        );
        let agg = aggregate_plugins_blocking(&home);
        assert_eq!(agg.len(), 1, "{agg:?}");
        assert_eq!(agg[0].name, "@scope/pet");
        assert_eq!(agg[0].sources.len(), 1);
        assert_eq!(agg[0].sources[0].profile, "web");
        assert_eq!(agg[0].sources[0].version.as_deref(), Some("0.3.18"));
        assert_eq!(
            agg[0].sources[0].spec.as_deref(),
            Some("github:o/r#path:/packages/pet")
        );
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
