//! build_approvals.rs —— pnpm 12 构建脚本审批门（2026-09-07）。
//!
//! pnpm 12 起默认拦截依赖的安装脚本（postinstall 等）：`pnpm add` 在装完包后
//! 若依赖树存在未获批脚本，写 `allowBuilds: {包名: "set this to true or false"}`
//! 裁决模板进 profile 的 `pnpm-workspace.yaml` 并以 `ERR_PNPM_IGNORED_BUILDS`
//! 硬失败退出 1；dsh 转发链（`runPlugin`）原样透传退出码并跳过 bundle 调和，
//! 插件操作随之失败。dsh/pnpm 的既定出路都是人工编辑该文件的 allowBuilds 键
//! ——本模块把这一步产品化：从 dsh 输出解析被点名包 → 用户逐包裁决 →
//! 受控改写 allowBuilds 单键 → 前端重试原操作（红线不动 dsh 源码，复现点 12）。
//!
//! 写入边界（ADR-0009 第六次修订，写入例外 #5）：只允许改写 pnpm 自己生成的
//! allowBuilds 块（或文件无该键时追加），其余内容逐字节保留；YAML 结构超出
//! 平面 `key: 标量` 时拒绝写入——宁可不自动化也不写坏用户文件。

use serde::Deserialize;

/// 单包裁决：允许 = 运行其安装脚本；跳过 = 显式忽略（安装必成）。
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct BuildApproval {
    pub name: String,
    pub allowed: bool,
}

const GATE_MARKER: &str = "ERR_PNPM_IGNORED_BUILDS";
const LIST_HEAD: &str = "Ignored build scripts:";

/// 从 dsh 转发链输出中解析被审批门点名的包名（去重保序，去版本段）。
/// 输出无门槛标记（非该失败模式）时返回空——只在失败路径调用，但以
/// `ERR_PNPM_IGNORED_BUILDS` 为前置条件，避免把 pnpm 10 式的纯警告当失败。
/// pnpm 的列表可能因终端宽度折行（包名从连字符处断开，如 `node-` /
/// `pty@1.1.0`），故收集到 help 行/空行为止后去除全部空白再切分。
pub fn parse_ignored_builds(output: &str) -> Vec<String> {
    if !output.contains(GATE_MARKER) {
        return Vec::new();
    }
    let mut collected: Option<String> = None;
    for line in output.lines() {
        match collected.as_mut() {
            None => {
                if let Some(idx) = line.find(LIST_HEAD) {
                    collected = Some(line[idx + LIST_HEAD.len()..].to_string());
                }
            }
            Some(acc) => {
                let t = line.trim();
                // 列表终止：空行 / help 行 / 新的错误图元（×、╰─▶）
                if t.is_empty()
                    || t.starts_with("help:")
                    || t.starts_with('×')
                    || t.starts_with("╰")
                {
                    break;
                }
                acc.push_str(t);
            }
        }
    }
    let Some(acc) = collected else {
        return Vec::new();
    };
    let compact: String = acc.chars().filter(|c| !c.is_whitespace()).collect();
    let mut out: Vec<String> = Vec::new();
    for entry in compact.split(',') {
        // 句末可能带句号（取决于渲染），包名/版本不含 '.'
        let entry = entry.strip_suffix('.').unwrap_or(entry);
        if entry.is_empty() {
            continue;
        }
        // name@version → 取最后一段 '@' 之前；@scope/name（无版本）整体保留
        let name = match entry.rfind('@') {
            Some(p) if p > 0 => &entry[..p],
            _ => entry,
        };
        if !name.is_empty() && !out.iter().any(|n| n == name) {
            out.push(name.to_string());
        }
    }
    out
}

/// allowBuilds 键合法性：npm 裸名（可带 scope），禁止版本段与 YAML 结构字符。
/// 该名会作为 YAML 键写入，字符面收窄是注入防线（IPC 是信任边界）。
pub fn validate_build_pkg_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("依赖名不能为空".to_string());
    }
    if name.len() > 214 {
        return Err("依赖名过长（npm 上限 214 字符）".to_string());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "@/._-~".contains(c))
    {
        return Err("依赖名只允许字母数字与 @/._-~".to_string());
    }
    if !name.starts_with('@')
        && !name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
    {
        return Err("依赖名必须以字母数字或 @scope 开头".to_string());
    }
    let scoped = name.starts_with('@');
    let at_count = name.chars().filter(|c| *c == '@').count();
    if at_count > usize::from(scoped) {
        return Err("依赖名不能带版本段（写裸包名，如 ssh2）".to_string());
    }
    if scoped && !name[1..].contains('/') {
        return Err("scope 名缺少 /（如 @scope/name）".to_string());
    }
    Ok(())
}

/// YAML 纯标量输出：字符面安全的裸出，其余单引号包裹（YAML 保留起始字符，
/// 如 scope 名的 `@`，必须引号；包名不含单引号，无需转义）。
fn yaml_scalar(s: &str) -> String {
    let plain = !s.is_empty()
        && s.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c));
    if plain {
        s.to_string()
    } else {
        format!("'{s}'")
    }
}

/// 解析 allowBuilds 块的一行 `键: 值`。值为**原样文本**（含引号），未裁决
/// 条目按字节保留；键容单双引号。返回 None = 结构超出平面标量（嵌套/流式/
/// 空值），调用方拒绝改写。
fn split_entry(line: &str) -> Option<(String, String)> {
    let t = line.trim();
    if t.is_empty() || t.starts_with('#') {
        return None;
    }
    let (key, value) = match t.starts_with(['\'', '"']) {
        true => {
            let q = t.chars().next()?;
            let rest = &t[1..];
            let close = rest.find(q)?;
            let after = &rest[close + 1..];
            (
                rest[..close].to_string(),
                after.strip_prefix(':')?.trim().to_string(),
            )
        }
        false => {
            let (k, v) = t.split_once(':')?;
            (k.trim().to_string(), v.trim().to_string())
        }
    };
    if value.is_empty() || value.contains('{') || value.contains('[') {
        return None;
    }
    Some((key, value))
}

/// 受控改写：merge/追加 allowBuilds 条目，其余内容逐字节保留。返回与原文
/// 相同的 String 表示无变化。结构超出演知范围（嵌套/流式/引号残缺/Tab 缩进）
/// 时 Err，不做部分写入。
pub fn apply_build_approvals(yaml: &str, approvals: &[(&str, bool)]) -> Result<String, String> {
    if approvals.is_empty() {
        return Ok(yaml.to_string());
    }
    let lines: Vec<&str> = yaml.lines().map(|l| l.trim_end_matches('\r')).collect();
    // 顶层 allowBuilds 键定位：只认裸键且零缩进的块式写法；其余含该词的行
    // （流式/引号键/二级缩进）一律拒绝——避免改写出重复键或写错层级
    let key_idx = match lines.iter().position(|l| *l == "allowBuilds:") {
        Some(i) => Some(i),
        None => {
            let conflicts = lines.iter().any(|l| {
                let t = l.trim_start();
                let bare = t.strip_prefix(['\'', '"']).unwrap_or(t);
                bare.starts_with("allowBuilds")
            });
            if conflicts {
                return Err("allowBuilds 写法不受支持（流式/引号键），请手工编辑".to_string());
            }
            None
        }
    };

    // (键, 原样值)：非裁决条目的值逐字节保留（含 pnpm 占位模板——占位是
    // 「未决」，改写成显式 false 会静默放过门槛，语义不同）
    let mut entries: Vec<(String, String)> = Vec::new();
    let mut indent = "  ".to_string();
    let (head, tail): (Vec<&str>, Vec<&str>) = match key_idx {
        Some(i) => {
            let mut j = i + 1;
            let mut block: Vec<&str> = Vec::new();
            while j < lines.len() && (lines[j].starts_with(' ') || lines[j].starts_with('\t')) {
                if lines[j].starts_with('\t') {
                    return Err("allowBuilds 块含 Tab 缩进，拒绝自动改写".to_string());
                }
                block.push(lines[j]);
                j += 1;
            }
            if let Some(first) = block.first() {
                let n = first.len() - first.trim_start().len();
                indent = " ".repeat(n);
            }
            for line in &block {
                let (k, v) = split_entry(line)
                    .ok_or_else(|| "allowBuilds 含非平面标量条目，拒绝自动改写".to_string())?;
                entries.push((k, v));
            }
            (lines[..=i].to_vec(), lines[i + 1 + block.len()..].to_vec())
        }
        None => (lines.clone(), Vec::new()),
    };

    for (name, allowed) in approvals {
        validate_build_pkg_name(name)?;
        let rendered = if *allowed { "true" } else { "false" };
        match entries.iter_mut().find(|(k, _)| k == name) {
            Some(e) => e.1 = rendered.to_string(),
            None => entries.push((name.to_string(), rendered.to_string())),
        }
    }
    let mut out: Vec<String> = head.iter().map(|l| l.to_string()).collect();
    if key_idx.is_none() {
        // 追加新块：与既有内容留一空行，保持可读
        if !out.is_empty() && !out.last().is_some_and(|l| l.is_empty()) {
            out.push(String::new());
        }
        out.push("allowBuilds:".to_string());
    }
    for (k, v) in &entries {
        out.push(format!("{indent}{}: {}", yaml_scalar(k), v));
    }
    out.extend(tail.iter().map(|l| l.to_string()));
    let mut joined = out.join("\n");
    if yaml.ends_with('\n') && !joined.ends_with('\n') {
        joined.push('\n');
    }
    Ok(joined)
}

/// IPC 入口：校验 profile 与包名 → 读 profile 的 pnpm-workspace.yaml → 受控
/// 改写 → 原子替换（tmp + rename，同 settings.rs 口径）。内容无变化时不写，
/// 避免 mtime 抖动。
pub fn set_profile_build_approvals(
    profile: &str,
    approvals: &[BuildApproval],
) -> Result<(), String> {
    crate::profiles::validate_profile_name(profile)?;
    if approvals.is_empty() {
        return Err("审批清单为空".to_string());
    }
    let pairs: Vec<(&str, bool)> = approvals
        .iter()
        .map(|a| (a.name.as_str(), a.allowed))
        .collect();
    let home = crate::resolve::user_dsh_home();
    let path = home
        .join("profiles")
        .join(profile)
        .join("pnpm-workspace.yaml");
    if !path.is_file() {
        return Err(format!(
            "「{}」不存在——profile「{profile}」尚未初始化，先创建或首启一次",
            path.display()
        ));
    }
    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("读取 {} 失败：{e}", path.display()))?;
    let next = apply_build_approvals(&raw, &pairs)?;
    if next == raw {
        return Ok(());
    }
    let tmp = path.with_extension("dsh-approvals.tmp");
    std::fs::write(&tmp, &next).map_err(|e| format!("写临时文件失败：{e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("替换 {} 失败：{e}", path.display())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 真实失败输出的 fixture（取自 2026-09-07 本机 plugin-op.log，含折行）。
    const REAL_OUTPUT: &str = "Packages: +240\nProgress: resolved 240, reused 217, downloaded 23, added 240, done\n\ndependencies:\n+ @linxin666/dsh-web-all ^0.3.16\n\nError: ERR_PNPM_IGNORED_BUILDS\n\n  × adding a new package\n  ╰─▶ Ignored build scripts: cloudflared@0.7.3, cpu-features@0.0.10, node-\n      pty@1.1.0, ssh2@1.17.0\n  help: Run \"pnpm approve-builds\" to pick which dependencies should be allowed\n        to run scripts.\n\ndsh: pnpm failed in profile directory /Users/guan/.dsh/profiles/web\n";

    #[test]
    fn parse_extracts_names_from_real_wrapped_output() {
        let got = parse_ignored_builds(REAL_OUTPUT);
        assert_eq!(
            got,
            vec![
                "cloudflared".to_string(),
                "cpu-features".to_string(),
                "node-pty".to_string(),
                "ssh2".to_string()
            ]
        );
    }

    #[test]
    fn parse_handles_scoped_names_and_bare_names() {
        let out = "Error: ERR_PNPM_IGNORED_BUILDS\n  ╰─▶ Ignored build scripts: @scope/pkg-a@1.0.0, plain, @scope/b@2.0\n  help: x\n";
        assert_eq!(
            parse_ignored_builds(out),
            vec!["@scope/pkg-a", "plain", "@scope/b"]
        );
    }

    #[test]
    fn parse_ignores_warning_only_output_without_gate_marker() {
        // pnpm 10 式纯警告（无硬失败标记）不得误报
        let out = "warn: Ignored build scripts: ssh2@1.0.0\n";
        assert!(parse_ignored_builds(out).is_empty());
        assert!(parse_ignored_builds("").is_empty());
    }

    #[test]
    fn parse_stops_list_at_help_line() {
        let out = "Error: ERR_PNPM_IGNORED_BUILDS\n  ╰─▶ Ignored build scripts: ssh2@1.0\n  help: Run \"pnpm approve-builds\"\n  dsh: pnpm failed\n";
        assert_eq!(parse_ignored_builds(out), vec!["ssh2"]);
    }

    #[test]
    fn pkg_name_rejects_version_segments_and_yaml_meta() {
        assert!(validate_build_pkg_name("ssh2").is_ok());
        assert!(validate_build_pkg_name("@scope/pkg-name._~").is_ok());
        for bad in [
            "",
            "ssh2@1.0.0",
            "@scope",
            "a:b",
            "a b",
            "-lead",
            "a;touch",
            "a\\b",
        ] {
            assert!(validate_build_pkg_name(bad).is_err(), "应当拒绝：{bad}");
        }
    }

    #[test]
    fn apply_replaces_placeholder_template_in_place() {
        let yaml = "packages:\n  - .\n\nnodeLinker: hoisted\nallowBuilds:\n  cloudflared: set this to true or false\n  cpu-features: set this to true or false\nminimumReleaseAgeExclude:\n  - dsh-agy-link@0.4.26\n";
        let got =
            apply_build_approvals(yaml, &[("cloudflared", false), ("cpu-features", true)]).unwrap();
        assert_eq!(
            got,
            "packages:\n  - .\n\nnodeLinker: hoisted\nallowBuilds:\n  cloudflared: false\n  cpu-features: true\nminimumReleaseAgeExclude:\n  - dsh-agy-link@0.4.26\n"
        );
    }

    #[test]
    fn apply_appends_missing_keys_preserving_order_and_rest() {
        let yaml = "allowBuilds:\n  ssh2: true\nother: 1\n";
        let got = apply_build_approvals(yaml, &[("node-pty", false), ("ssh2", false)]).unwrap();
        assert_eq!(
            got,
            "allowBuilds:\n  ssh2: false\n  node-pty: false\nother: 1\n"
        );
    }

    /// 真实 web profile 的 pnpm-workspace.yaml 全文（2026-09-07 取自本机
    /// ~/.dsh/profiles/web，含 `||` 版本区间等特殊字符）——四包裁决后其余
    /// 内容必须逐字节保留。
    #[test]
    fn apply_rewrites_real_world_workspace_yaml_end_to_end() {
        let yaml = "packages:\n  - .\n\nnodeLinker: hoisted\nautoInstallPeers: false\nminimumReleaseAgeExclude:\n  - '@mars-sea/dsh-commandcode-provider@0.10.0-alpha.7 || 0.10.1'\n  - dsh-agy-link@0.4.26\nallowBuilds:\n  cloudflared: set this to true or false\n  cpu-features: set this to true or false\n  node-pty: set this to true or false\n  ssh2: set this to true or false\n";
        let got = apply_build_approvals(
            yaml,
            &[
                ("cloudflared", false),
                ("cpu-features", false),
                ("node-pty", true),
                ("ssh2", true),
            ],
        )
        .unwrap();
        assert_eq!(
            got,
            "packages:\n  - .\n\nnodeLinker: hoisted\nautoInstallPeers: false\nminimumReleaseAgeExclude:\n  - '@mars-sea/dsh-commandcode-provider@0.10.0-alpha.7 || 0.10.1'\n  - dsh-agy-link@0.4.26\nallowBuilds:\n  cloudflared: false\n  cpu-features: false\n  node-pty: true\n  ssh2: true\n"
        );
    }

    #[test]
    fn apply_appends_block_when_key_missing() {
        let yaml = "packages:\n  - .\n";
        let got = apply_build_approvals(yaml, &[("@scope/pkg", true)]).unwrap();
        assert_eq!(
            got,
            "packages:\n  - .\n\nallowBuilds:\n  '@scope/pkg': true\n"
        );
    }

    #[test]
    fn apply_rejects_structures_it_cannot_rewrite() {
        // 流式写法 → 拒绝（避免生成重复键）
        assert!(apply_build_approvals("allowBuilds: {ssh2: true}\n", &[("a", true)]).is_err());
        // 引号键 → 拒绝
        assert!(apply_build_approvals("'allowBuilds':\n  a: true\n", &[("b", true)]).is_err());
        // 嵌套值 → 拒绝
        assert!(
            apply_build_approvals("allowBuilds:\n  ssh2:\n    deep: true\n", &[("a", true)])
                .is_err()
        );
        // Tab 缩进 → 拒绝
        assert!(apply_build_approvals("allowBuilds:\n\tssh2: true\n", &[("a", true)]).is_err());
        // 非法包名 → 拒绝（不写入）
        assert!(apply_build_approvals("allowBuilds:\n  a: true\n", &[("a@1.0", true)]).is_err());
    }

    #[test]
    fn apply_preserves_undecided_entries_it_was_not_asked_about() {
        // 未裁决条目逐字节保留：占位「未决」≠ 显式 false（后者会静默放过门槛）
        let yaml = "allowBuilds:\n  a: set this to true or false\n  b: true\n";
        assert_eq!(apply_build_approvals(yaml, &[("b", true)]).unwrap(), yaml);
    }

    #[test]
    fn apply_is_noop_when_values_already_match() {
        let yaml = "allowBuilds:\n  ssh2: true\n";
        assert_eq!(
            apply_build_approvals(yaml, &[("ssh2", true)]).unwrap(),
            yaml
        );
    }

    #[test]
    fn apply_handles_crlf_and_trailing_newline() {
        let yaml = "allowBuilds:\r\n  ssh2: set this to true or false\r\n";
        let got = apply_build_approvals(yaml, &[("ssh2", false)]).unwrap();
        assert_eq!(got, "allowBuilds:\n  ssh2: false\n");
    }

    #[test]
    fn apply_empty_approvals_is_noop() {
        let yaml = "allowBuilds:\n  ssh2: true\n";
        assert_eq!(apply_build_approvals(yaml, &[]).unwrap(), yaml);
    }
}
