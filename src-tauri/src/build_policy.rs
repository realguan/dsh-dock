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

use std::path::Path;

/// 顶层键（pnpm 12.3.1 实测；`pnpm config list` 的 settings 名同形）。
const KEY: &str = "dangerouslyAllowAllBuilds";
/// pnpm 的 kebab 别名（CLI 选项名口径）——同义键，识别到即视为已存在。
const KEY_KEBAB: &str = "dangerously-allow-all-builds";

/// 在 pnpm-workspace.yaml 文本上幂等补写 `dangerouslyAllowAllBuilds: true`。
/// 已为 `true` → 原样返回；已是其他标量（`false`/占位串/引号）→ 就地把值改成
/// `true`（保留行内注释）；无该键 → 末尾追加一块。返回与入参相同表示无变化。
pub fn ensure_allow_all_builds(yaml: &str) -> Result<String, String> {
    upsert_scalar_key(yaml, KEY, KEY_KEBAB, "true")
}

/// 通用「顶层标量键幂等补写」内核（2026-09-10 从 `ensure_allow_all_builds` 抽取，
/// 供 `nodeLinker` 复用同一套解析与拒写纪律——不复制第二份 YAML 处理逻辑）。
///
/// `want` = 目标标量字面量（布尔 `true`、裸值 `hoisted` 皆可）。
fn upsert_scalar_key(
    yaml: &str,
    key_name: &str,
    key_kebab: &str,
    want: &str,
) -> Result<String, String> {
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
            if trimmed.contains(key_name) || trimmed.contains(key_kebab) {
                return Err(format!(
                    "{key_name} 写法不受支持（引号键/流式结构），请手工编辑 pnpm-workspace.yaml"
                ));
            }
            continue;
        }
        let Some((key, rest)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim();
        if key != key_name && key != key_kebab {
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
                        "{key_name} 写法不受支持（引号未闭合），请手工编辑 pnpm-workspace.yaml"
                    ))
                }
            },
            _ => {
                let len = after_ws.find(char::is_whitespace).unwrap_or(after_ws.len());
                (&after_ws[..len], &after_ws[len..])
            }
        };
        if value == want {
            return Ok(yaml.to_string());
        }
        if value.is_empty() || value.starts_with(['{', '[']) {
            return Err(format!(
                "{key_name} 写法不受支持（空值/嵌套/流式），请手工编辑 pnpm-workspace.yaml"
            ));
        }
        // false / 占位串 / 引号 → 就地改值，保留缩进与行内注释
        let leading_ws: String = rest.chars().take_while(|c| c.is_whitespace()).collect();
        lines[i] = format!("{key}:{leading_ws}{want}{tail}");
        return Ok(join(&lines, trailing_newline));
    }

    // 无该键：末尾追加一块（与既有内容留一空行，保持可读）
    if !lines.is_empty() && !lines.last().is_some_and(|l| l.is_empty()) {
        lines.push(String::new());
    }
    lines.push(format!("{key_name}: {want}"));
    Ok(join(&lines, true))
}

fn join(lines: &[String], trailing_newline: bool) -> String {
    let mut out = lines.join("\n");
    if trailing_newline && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

// ---------- 引擎目录的 node-linker（2026-09-10，v1.1.0 Windows 实测 1.2/1.4/1.7） ----------

/// `nodeLinker` 顶层键（pnpm 12 的配置落点 = `pnpm-workspace.yaml`）。
const LINKER_KEY: &str = "nodeLinker";
/// pnpm 的 kebab 别名（CLI 选项名口径）。
const LINKER_KEY_KEBAB: &str = "node-linker";

/// **Windows 引擎引导的根因修复**：让 pnpm 在引擎目录里用**免符号链接**布局。
///
/// ## 为什么必须这样做（实测证据）
///
/// v1.1.0 Windows 实机首启直接失败，错误卡原文（路径分隔符已按文档惯例转写）：
/// ```text
/// pnpm runtime set node v24.18.0 失败：× adding a new package
///   ╰─▶ Failed to symlink "node" for importer ".":
///       Failed to create symlink at <engines>/node_modules/node
///       to <engines>/node_modules/.pnpm/node@runtime+24.18.0/node_modules/node：
///       拒绝访问。(os error 5)
/// ```
/// 即：pnpm 默认的 `isolated` 布局要给 node 建**目录符号链接**，而 Windows 上
/// 创建符号链接需要 `SeCreateSymbolicLinkPrivilege`（管理员）或用户手动开启
/// 「开发者模式」。普通账户下 `CreateSymbolicLink` 直接 `ERROR_ACCESS_DENIED`。
/// 失败链：node 装不上 → dsh 也永远装不上（`add -g` 用同一套链接机制）→
/// 「DSH 未检出」「健康大盘三件全缺」「插件安装报引擎未就绪」全是它的下游。
///
/// `hoisted` = 扁平 `node_modules`、真实目录，**不需要符号链接**，正是为这类
/// 环境设计的布局。
///
/// ## 为什么写在引擎目录而不是全局/用户配置
///
/// 只影响壳自管的引擎目录（ADR-0010：引擎是壳资产），不碰用户的 pnpm 全局配置，
/// 也不影响用户自己的项目——最小作用域。
///
/// ## 保守性
///
/// 与 ADR-0013 同一套纪律：只动这一个顶层键，文件其余内容逐字节保留；结构超出
/// 演知范围（流式/引号键/嵌套/空值）一律拒绝写入；无变化零写入。
pub fn ensure_engine_node_linker(yaml: &str) -> Result<String, String> {
    upsert_scalar_key(yaml, LINKER_KEY, LINKER_KEY_KEBAB, "hoisted")
}

/// 在引擎目录幂等写入 `nodeLinker: hoisted`（`<engines>/pnpm-workspace.yaml`）。
///
/// 返回 true = 本次确实写了。文件不存在 → 创建（引擎目录由壳自管，此文件本就
/// 可能不存在：pnpm 首次引导时才生成）。
pub fn ensure_engine_linker(engines_dir: &Path) -> Result<bool, String> {
    let path = engines_dir.join("pnpm-workspace.yaml");
    let raw = std::fs::read_to_string(&path).unwrap_or_default();
    let next = ensure_engine_node_linker(&raw)?;
    if next == raw {
        return Ok(false);
    }
    std::fs::create_dir_all(engines_dir)
        .map_err(|e| format!("创建引擎目录 {} 失败：{e}", engines_dir.display()))?;
    let tmp = path.with_extension("dsh-linker.tmp");
    std::fs::write(&tmp, &next).map_err(|e| format!("写临时文件失败：{e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("替换 {} 失败：{e}", path.display())
    })?;
    Ok(true)
}

/// 幂等前置（best effort）：写失败只告警不阻断——引导失败时 pnpm 的原始报错
/// 仍会带着完整错误链上抛（不会因为这里失败而变模糊）。
pub fn ensure_engine_linker_best_effort(engines_dir: &Path) {
    match ensure_engine_linker(engines_dir) {
        Ok(true) => tracing::info!(
            "引擎目录已写入 nodeLinker: hoisted（{}）——免符号链接布局",
            engines_dir.display()
        ),
        Ok(false) => {}
        Err(e) => tracing::warn!("引擎 nodeLinker 写入失败（{}）：{e}", engines_dir.display()),
    }
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

// ---------- 客体档孪生（ADR-0016 P1 范围修正，2026-09-11） ----------

/// **客体侧**单键写入：WSL 模式下 profile 由客体 dsh 自行物化，宿主这份写入器
/// 从未碰过客体文件——客体内 `pnpm add` 于是会撞 pnpm 12 的构建审批门
/// （`ERR_PNPM_IGNORED_BUILDS` / `ERR_PNPM_GIT_DEP_PREPARE_NOT_ALLOWED`），带构建
/// 脚本的插件必失败。故本孪生从 P2 提前到 P1（ADR-0016 §4 落地期范围修正）。
///
/// 与宿主实现**同一套纪律**（ADR-0013）：读原文（经客体读原语）→ 复用同一个纯函数
/// [`ensure_allow_all_builds`] → **仅在有变化时**写回（客体侧临时文件 + `mv` 原子
/// 替换）。不整体覆盖、不生成三件套内容、无变化零写入（免 mtime 抖动）。
///
/// 返回 true = 本次确实写了。客体不存在（非 Windows / 不可达）/ profile 未物化 → Err。
pub fn ensure_profile_build_policy_in_guest(distro: &str, profile: &str) -> Result<bool, String> {
    crate::profiles::validate_profile_name(profile)?;
    let rel = format!("profiles/{profile}/pnpm-workspace.yaml");
    let files = crate::guest::read_files(distro, std::slice::from_ref(&rel))?;
    let raw = match files.first() {
        Some((_, Some(text))) => text.clone(),
        _ => {
            return Err(format!(
                "客体 {distro} 内「{rel}」不存在——profile「{profile}」尚未初始化，先创建或首启一次"
            ))
        }
    };
    let next = ensure_allow_all_builds(&raw)?;
    if next == raw {
        return Ok(false);
    }
    crate::guest::write_home_files(distro, &[(rel, next)])?;
    Ok(true)
}

/// 客体档幂等前置（best effort，口径同宿主 [`ensure_profile_build_policy_best_effort`]）：
/// 写失败只告警不阻断，客体 pnpm 的原始报错仍随 detail 上抛。
pub fn ensure_profile_build_policy_in_guest_best_effort(distro: &str, profile: &str) {
    match ensure_profile_build_policy_in_guest(distro, profile) {
        Ok(true) => tracing::info!(
            "已写入 dangerouslyAllowAllBuilds: true（客体 {distro} · profile「{profile}」，ADR-0016 P1）"
        ),
        Ok(false) => {}
        Err(e) => tracing::warn!("客体构建脚本默认批准写入失败（profile「{profile}」）：{e}"),
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

    // ---------- 引擎 node-linker（Windows 免符号链接修复） ----------

    #[test]
    fn linker_written_into_empty_config() {
        assert_eq!(
            ensure_engine_node_linker("").unwrap(),
            "nodeLinker: hoisted\n"
        );
    }

    #[test]
    fn linker_is_idempotent_and_preserves_other_keys() {
        let once = ensure_engine_node_linker("packages:\n  - .\n").unwrap();
        assert_eq!(once, "packages:\n  - .\n\nnodeLinker: hoisted\n");
        // 再跑一次必须零变化（免 mtime 抖动）
        assert_eq!(ensure_engine_node_linker(&once).unwrap(), once);
    }

    #[test]
    fn linker_recognizes_kebab_alias_as_already_set() {
        // 用户手写过 kebab 别名 → 视为已设置，不写重复键
        let yaml = "node-linker: hoisted\n";
        assert_eq!(ensure_engine_node_linker(yaml).unwrap(), yaml);
    }

    #[test]
    fn linker_overwrites_wrong_value_in_place() {
        // 曾被写成 isolated（或占位）→ 就地改正为 hoisted，保留行内注释
        let got = ensure_engine_node_linker("nodeLinker: isolated # 注释\n").unwrap();
        assert_eq!(got, "nodeLinker: hoisted # 注释\n");
    }

    #[test]
    fn linker_rejects_structures_it_cannot_rewrite() {
        // 与 allowBuilds 同一套拒写纪律：宁可不自动化，也不写坏文件
        assert!(ensure_engine_node_linker("nodeLinker:\n  x: 1\n").is_err());
        assert!(ensure_engine_node_linker("nodeLinker: {a: 1}\n").is_err());
        assert!(ensure_engine_node_linker("'nodeLinker': isolated\n").is_err());
    }

    // ---------- 客体档孪生（ADR-0016 P1） ----------

    /// 非法 profile 名在**触达客体之前**即被拒（防路径遍历进客体脚本）。
    #[test]
    fn guest_policy_rejects_invalid_profile_name_before_touching_guest() {
        for bad in ["../escape", "a/b", "a\\b", "", ".", "node_modules"] {
            let err = ensure_profile_build_policy_in_guest("Ubuntu", bad).unwrap_err();
            assert!(err.contains("非法 profile 名"), "{bad}: {err}");
        }
    }

    /// 非 Windows：客体不存在 → 诚实错误，绝不静默按宿主路径写。
    #[cfg(not(windows))]
    #[test]
    fn guest_policy_is_unavailable_off_windows() {
        let err = ensure_profile_build_policy_in_guest("Ubuntu", "web").unwrap_err();
        assert!(err.contains("仅在 Windows 宿主可用"), "{err}");
    }

    /// **落盘层**：真实文件往返 + 幂等（这是引导启动时会跑的那条路径）。
    #[test]
    fn ensure_engine_linker_writes_and_is_idempotent_on_disk() {
        let dir = std::env::temp_dir().join(format!("dsh-linker-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // 引擎目录尚不存在（首启形态）→ 应自建并写入
        assert!(ensure_engine_linker(&dir).unwrap(), "首次应写入");
        let path = dir.join("pnpm-workspace.yaml");
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "nodeLinker: hoisted\n"
        );
        // 二次调用零写入
        assert!(!ensure_engine_linker(&dir).unwrap(), "二次应零变化");
        // 已存在的其他键必须保留
        std::fs::write(&path, "packages:\n  - .\n").unwrap();
        assert!(ensure_engine_linker(&dir).unwrap());
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("packages:"), "既有内容不得丢失：{text}");
        assert!(text.contains("nodeLinker: hoisted"), "应补写键：{text}");
        std::fs::remove_dir_all(&dir).ok();
    }
}
