//! build_policy.rs —— pnpm 构建脚本策略：**默认批准**（2026-09-09，ADR-0013）。
//!
//! pnpm 12 默认拦截依赖安装脚本，两种门槛形态都会让插件操作硬失败退出 1
//! （复现点 12）：npm 来源 `ERR_PNPM_IGNORED_BUILDS`、git/tarball 来源
//! `ERR_PNPM_GIT_DEP_PREPARE_NOT_ALLOWED`。壳原先把门槛产品化为「解析被点名
//! 包 → 逐包裁决 → 改写 allowBuilds → 重试」（ADR-0009 第六次修订），实测维护
//! 成本与失效风险高于收益（解析器是对 pnpm 输出形态的单样本归纳；2026-09-09
//! 日志策略变更即让它弹出错包并陷入死循环）——ADR-0013 裁定改为**默认批准**：
//! 向 profile 的 `pnpm-workspace.yaml` 幂等写入 `dangerouslyAllowAllBuilds: true`，
//! pnpm 不再进入审批门，审批链整体退役。
//!
//! 写入边界（ADR-0013）：只动这一个顶层键（camelCase 与 pnpm 的 kebab 别名同
//! 认，避免用户手写过别名时写重），文件其余内容逐字节保留；结构超出演知范围
//! （流式/引号键/嵌套/空值）一律拒绝写入——宁可不自动化也不写坏用户文件；
//! 无变化零写入（免 mtime 抖动），tmp + rename 原子替换。

/// 顶层键（pnpm 12.3.1 实测；`pnpm config list` 的 settings 名同形）。
const KEY: &str = "dangerouslyAllowAllBuilds";
/// pnpm 的 kebab 别名（CLI 选项名口径）——同义键，识别到即视为已存在。
const KEY_KEBAB: &str = "dangerously-allow-all-builds";

/// 在 pnpm-workspace.yaml 文本上幂等补写 `dangerouslyAllowAllBuilds: true`。
/// 已为 `true` → 原样返回；已是其他标量（`false`/占位串/引号）→ 就地把值改成
/// `true`（保留行内注释）；无该键 → 末尾追加一块。返回与入参相同表示无变化。
pub fn ensure_allow_all_builds(yaml: &str) -> Result<String, String> {
    let trailing_newline = yaml.ends_with('\n');
    let mut lines: Vec<String> = yaml
        .lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();

    for i in 0..lines.len() {
        let line = lines[i].clone();
        let trimmed = line.trim_start();
        // 只看顶层（零缩进）行；嵌套层级的同名键不是本键，跳过
        if trimmed.len() != line.len() {
            continue;
        }
        if trimmed.starts_with(['\'', '"', '{', '[']) {
            // 引号键/流式结构：若含同义键名则拒绝（改写无法保证语义）
            if trimmed.contains(KEY) || trimmed.contains(KEY_KEBAB) {
                return Err(format!(
                    "{KEY} 写法不受支持（引号键/流式结构），请手工编辑 pnpm-workspace.yaml"
                ));
            }
            continue;
        }
        let Some((key, rest)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if key != KEY && key != KEY_KEBAB {
            continue;
        }
        let after_ws = rest.trim_start();
        // 值取到「标量末尾」：引号值取到配对引号（值内可含空格，如 pnpm 的
        // 占位串），裸值取到首个空白；tail 保留行内注释
        let (value, tail) = match after_ws.chars().next() {
            Some(q @ ('\'' | '"')) => match after_ws[1..].find(q) {
                Some(close) => {
                    let end = close + 2;
                    (&after_ws[..end], &after_ws[end..])
                }
                None => {
                    return Err(format!(
                        "{KEY} 写法不受支持（引号未闭合），请手工编辑 pnpm-workspace.yaml"
                    ))
                }
            },
            _ => {
                let len = after_ws.find(char::is_whitespace).unwrap_or(after_ws.len());
                (&after_ws[..len], &after_ws[len..])
            }
        };
        if value == "true" {
            return Ok(yaml.to_string());
        }
        if value.is_empty() || value.starts_with(['{', '[']) {
            return Err(format!(
                "{KEY} 写法不受支持（空值/嵌套/流式），请手工编辑 pnpm-workspace.yaml"
            ));
        }
        // false / 占位串 / 引号 → 就地改值，保留缩进与行内注释
        let leading_ws: String = rest.chars().take_while(|c| c.is_whitespace()).collect();
        lines[i] = format!("{key}:{leading_ws}true{tail}");
        return Ok(join(&lines, trailing_newline));
    }

    // 无该键：末尾追加一块（与既有内容留一空行，保持可读）
    if !lines.is_empty() && !lines.last().is_some_and(|l| l.is_empty()) {
        lines.push(String::new());
    }
    lines.push(format!("{KEY}: true"));
    Ok(join(&lines, true))
}

fn join(lines: &[String], trailing_newline: bool) -> String {
    let mut out = lines.join("\n");
    if trailing_newline && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// 读 profile 的 pnpm-workspace.yaml → 幂等补写 → 原子替换。返回 true 表示
/// 本次确实写了（无变化返回 false，不触碰文件）。
pub fn ensure_profile_build_policy(profile: &str) -> Result<bool, String> {
    crate::profiles::validate_profile_name(profile)?;
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
    let next = ensure_allow_all_builds(&raw)?;
    if next == raw {
        return Ok(false);
    }
    let tmp = path.with_extension("dsh-buildpolicy.tmp");
    std::fs::write(&tmp, &next).map_err(|e| format!("写临时文件失败：{e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("替换 {} 失败：{e}", path.display())
    })?;
    Ok(true)
}

/// 插件操作/创建链的幂等前置：写失败只告警不阻断——默认批准是「少一次门槛」
/// 的便利，不是操作前置条件；写入失败时 pnpm 的原始报错仍随 detail 返回。
pub fn ensure_profile_build_policy_best_effort(profile: &str) {
    match ensure_profile_build_policy(profile) {
        Ok(true) => {
            tracing::info!(
                "已写入 dangerouslyAllowAllBuilds: true（profile「{profile}」，ADR-0013）"
            )
        }
        Ok(false) => {}
        Err(e) => tracing::warn!("构建脚本默认批准写入失败（profile「{profile}」）：{e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 真实 profile 的 pnpm-workspace.yaml 全文（2026-09-09 取自本机
    /// ~/.dsh/profiles/test，含 pnpm 写的占位模板与 git exact key）。
    const REAL: &str = "packages:\n  - .\n\nnodeLinker: hoisted\nautoInstallPeers: false\nminimumReleaseAgeExclude:\n  - '@linxin666/dsh-client-ui-git-graph@0.3.17'\n\nallowBuilds:\n  '@linxin666/dsh-pet@https://codeload.github.com/x/y/tar.gz/abc#path:/packages/dsh-pet': true\n  cpu-features: set this to true or false\n  ssh2: set this to true or false\n";

    #[test]
    fn appends_key_when_missing_preserving_rest_byte_for_byte() {
        let got = ensure_allow_all_builds(REAL).unwrap();
        assert_eq!(got, format!("{REAL}\ndangerouslyAllowAllBuilds: true\n"));
        // 其余内容逐字节保留
        assert!(got.starts_with(REAL));
    }

    #[test]
    fn noop_when_already_true() {
        let yaml = "packages:\n  - .\n\ndangerouslyAllowAllBuilds: true\n";
        assert_eq!(ensure_allow_all_builds(yaml).unwrap(), yaml);
    }

    #[test]
    fn recognizes_kebab_alias_as_already_set() {
        let yaml = "dangerously-allow-all-builds: true\nother: 1\n";
        assert_eq!(ensure_allow_all_builds(yaml).unwrap(), yaml);
    }

    #[test]
    fn rewrites_false_in_place_keeping_comment_and_neighbours() {
        let yaml =
            "packages:\n  - .\ndangerouslyAllowAllBuilds: false  # 手工关闭\nnodeLinker: hoisted\n";
        let got = ensure_allow_all_builds(yaml).unwrap();
        assert_eq!(
            got,
            "packages:\n  - .\ndangerouslyAllowAllBuilds: true  # 手工关闭\nnodeLinker: hoisted\n"
        );
    }

    #[test]
    fn rewrites_quoted_placeholder_value() {
        let yaml = "dangerouslyAllowAllBuilds: 'set this to true or false'\n";
        assert_eq!(
            ensure_allow_all_builds(yaml).unwrap(),
            "dangerouslyAllowAllBuilds: true\n"
        );
    }

    #[test]
    fn ignores_nested_same_named_key() {
        // 二级缩进的同名键不是本键 → 视为缺失，追加顶层键
        let yaml = "something:\n  dangerouslyAllowAllBuilds: false\n";
        let got = ensure_allow_all_builds(yaml).unwrap();
        assert_eq!(
            got,
            "something:\n  dangerouslyAllowAllBuilds: false\n\ndangerouslyAllowAllBuilds: true\n"
        );
    }

    #[test]
    fn rejects_structures_it_cannot_rewrite() {
        // 空值（下挂嵌套块）/ 流式 / 引号键 → 拒绝写入
        assert!(ensure_allow_all_builds("dangerouslyAllowAllBuilds:\n  x: 1\n").is_err());
        assert!(ensure_allow_all_builds("dangerouslyAllowAllBuilds: {x: 1}\n").is_err());
        assert!(ensure_allow_all_builds("'dangerouslyAllowAllBuilds': false\n").is_err());
    }

    #[test]
    fn handles_crlf_and_missing_trailing_newline() {
        let got = ensure_allow_all_builds("packages:\r\n  - .\r\n").unwrap();
        assert_eq!(got, "packages:\n  - .\n\ndangerouslyAllowAllBuilds: true\n");
        let got = ensure_allow_all_builds("packages:\n  - .").unwrap();
        assert_eq!(got, "packages:\n  - .\n\ndangerouslyAllowAllBuilds: true\n");
    }

    #[test]
    fn appends_into_empty_file() {
        assert_eq!(
            ensure_allow_all_builds("").unwrap(),
            "dangerouslyAllowAllBuilds: true\n"
        );
    }
}
