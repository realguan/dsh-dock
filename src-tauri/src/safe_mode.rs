//! safe_mode.rs —— 启动失败的**安全模式**（ADR-0026，2026-09-16 维护者裁定）。
//!
//! ## 一句话
//!
//! 安全模式 = **在 profile 的 `cordis.patch.yml` 里，把所有"非随包（三方）挂载行"写成
//! `disabled: true`**（覆写前备份），然后**正常启动**；「一键恢复」= 用那份备份**原样覆盖
//! 回去**。用户的插件开关（配置层）从此就是安全模式的真相源，不再有第二套状态。
//!
//! ## 为什么不用临时 overlay（ADR-0025 原方案，机制已退役）
//!
//! `--patch` overlay 落在壳自有目录、不动配置，代价是**两个真相源**：配置说"已启用"、
//! 运行态说"已停用"——维护者真机看到「开关全开、徽标却是已停用」的自相矛盾界面。
//! 维护者 2026-09-16 裁定：「安全模式就该是所有三方插件在配置文件里 disable，后面用户
//! 自己选择启动哪个插件」。改配置之后：开关、徽标、下次启动三者同源。
//!
//! ## 停用范围（判据单源；2026-09-16 更正了 ADR-0025 的一条错误结论）
//!
//! **非随包行** = 段落标签的主段**不是** `dsh-base` / `dsh-web-app` 的行：
//! 用户 patch 行的主段是**文件路径**，三方 bundle 层行的主段是**包名**——两者都停；
//! 随包行**一律保留**，即使它被用户 patch 过（标签形如
//! `@deepseek-ai/dsh-base, patched by …/cordis.patch.yml`）。
//!
//! > **更正**：ADR-0025 §4 曾记"三方 bundle 层行停不掉，整层摘掉 ⇒ dsh `exit 1`"。
//! > 2026-09-16 复测推翻：真因是**判据把"被 patch 过的随包行"也算成了三方行**。只停
//! > 9 条三方行（5 用户 + 4 bundle）→ **8.2s 就绪**；多停一条随包行 `tools` →
//! > `required startup failure: 1 entry did not activate … pending (waiting for service: tools)`
//! > （正是当初那条错误结论的来源）。
use std::path::{Path, PathBuf};

/// 随包发布的 bundle（上游 `PROFILE_TEMPLATES` + 安装期归一化 tuple 的全表，2026-09-16 补齐）：
/// 安全模式**永不**停它们的行——任一被停都可能让服务依赖悬空（实测：停 `tools` 即 exit 1）。
///
/// 为什么是 6 个而不是"本机在用的 2 个"：headless / acp / sdk 档虽不由本壳 boot，但用户的
/// 历史或手工清单里可能同时含 `dsh-web-app` 与 `dsh-headless`（上游安装期 owned tuple 就是
/// 这种组合）；把它们误判成三方行会主动制造 exit 1（2026-09-16 独立复核实测指出）。
pub const SHIPPED_BUNDLES: &[&str] = &[
    "@deepseek-ai/dsh-base",
    "@deepseek-ai/dsh-web-app",
    "@deepseek-ai/dsh-headless",
    "@deepseek-ai/dsh-acp-app",
    "@deepseek-ai/dsh-sdk-app",
    "@deepseek-ai/dsh-sdk-minimal",
];

/// 某 profile 的 patch 路径（安全模式唯一会改的 dsh 文件）。
pub fn profile_patch_path(home: &Path, profile: &str) -> PathBuf {
    home.join("profiles").join(profile).join("cordis.patch.yml")
}

/// 记账文件路径（壳自有数据目录；**不是** dsh 文件）。
pub fn journal_path(data_dir: &Path, profile: &str) -> PathBuf {
    data_dir.join("safe-mode").join(format!("{profile}.json"))
}

/// 段落标签 → **主段**（去掉 upstream 的 `, patched by <标签…>` 尾巴）。
///
/// 为什么必须去掉尾巴：随包行被用户 patch 之后标签会变成
/// `@deepseek-ai/dsh-base, patched by …/cordis.patch.yml`——按整串判"是不是用户行"会把
/// 随包行误判成用户行（这正是 2026-09-16 那次 `exit 1` 的成因）。
pub fn primary_section(section: &str) -> &str {
    section
        .split(", patched by")
        .next()
        .unwrap_or(section)
        .trim()
}

/// 该行是否应在安全模式下停用（**纯函数，判据单源**）。
pub fn should_disable(contributed_by: Option<&str>) -> bool {
    match contributed_by {
        // **无归属 = 不当成可停行**（失效方向的选择，2026-09-16 独立复核对口径的修正）：
        // 当前 dump 每条行都有段落标签，`None` 只在**格式漂移/解析退化**时出现；那时
        // "当成用户行"会把随包行一起停掉 ⇒ 主动制造 `exit 1`。反过来少停只是"安全模式没救到
        // 那几行"，交给空计划诚实门如实报错即可。**宁可不救，不可救坏**。
        None => false,
        Some(section) => !SHIPPED_BUNDLES.contains(&primary_section(section)),
    }
}

/// 按**层序**切分停放计划（纯函数）：profile 层的停用桩只能停"同层或更早层"的行。
///
/// 用户 patch 有两个文件：profile 层 `profiles/<p>/cordis.patch.yml`（我们写桩的地方）与
/// **home 层** `$DSH_HOME/cordis.patch.yml`（层序在 profile 之后）。因此：
/// - 归属为**包名**（bundle 层，更早）或**本 profile 的 patch 文件** → 可停；
/// - 归属为**别的文件路径**（home 层，更晚）或 `None` → **停不掉**（id 在写桩那一层还不存在）。
///
/// 返回值 = `(可停, 停不掉)`。调用方必须把后者**如实告诉用户**，不能报"已全部停用"
/// （2026-09-16 独立复核指出：home 层 insert 行会走成"报成功、再点一次改口早已停用"）。
pub fn split_by_layer(
    rows: &[crate::plugins::RowAttribution],
    profile_patch: &Path,
) -> (Vec<String>, Vec<String>) {
    let mut disableable = Vec::new();
    let mut elsewhere = Vec::new();
    for row in rows {
        match row.contributed_by.as_deref() {
            // 随包行：本就不该停（正常保留，不算"够不到"）
            Some(section) if !should_disable(Some(section)) => {}
            // 本 profile 的 patch（写入目标本身）或包名（更早的 bundle 层）→ 可停
            Some(section) if Path::new(section) == profile_patch => {
                disableable.push(row.id.clone())
            }
            Some(section) if !looks_like_path(section) => disableable.push(row.id.clone()),
            // home 层（层序更晚）或**无归属**（格式漂移）：够不到 → 如实上报，不静默跳过
            _ => elsewhere.push(row.id.clone()),
        }
    }
    (disableable, elsewhere)
}

/// 段落标签是不是**文件路径**（区分"用户 patch 行"与"bundle 包名"）。
fn looks_like_path(section: &str) -> bool {
    section.starts_with('/') || section.contains(".yml") || section.contains('\\')
}

/// 记账：进入安全模式时"我们写进去了什么、进入前那份配置在哪"。
///
/// 这是**一键恢复的唯一依据**：退出时按 `patch_backup` 覆盖回去，绝不按文件名猜最新一份
/// （用户进入安全模式后又改过插件时，"最新一份"是安全模式之后的状态）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Journal {
    pub disabled_rows: Vec<String>,
    pub patch_backup: String,
    /// 进入时的 dsh home（绝对路径）。**跨 home 不认账**：开发档（`~/.dsh-dock-dev`）与正式档
    /// （`~/.dsh`）用的是**同一份壳设置/数据目录**，只按 profile 名记账会让另一侧显示"安全模式中"
    /// 且给一个必然失败的恢复按钮（2026-09-16 独立复核）。
    #[serde(default)]
    pub dsh_home: String,
    #[serde(default)]
    pub applied_at: u64,
}

/// 进入安全模式的结果。
#[derive(Debug, Clone)]
pub struct EnterOutcome {
    /// 是否真的改了配置（false = 全部行已是停用态 → 零写入）。
    pub changed: bool,
    /// 本次覆写前留下的备份（`changed=false` 时为 `None`：没覆写就没备份）。
    pub backup: Option<PathBuf>,
}

/// 退出（一键恢复）的结果。
#[derive(Debug, Clone)]
pub struct ExitOutcome {
    /// 是否真的恢复了（false = 本就不在安全模式）。
    pub restored: bool,
    /// 被用来覆盖回去的那份备份（进入安全模式前的那份）。
    pub backup_used: Option<PathBuf>,
    /// 恢复前对"当前配置"补做的备份（安全模式期间的改动不会无迹可寻）。
    pub current_backup: Option<PathBuf>,
}

/// 安全模式状态（给前端的**只读**快照：横幅/文案用）。
///
/// 边界：只报"壳的记账文件在不在、记了哪些行、那份备份还在不在"——**不报运行态**。
/// 运行态由回环快照（`plugins::fetch_runtime_snapshot`）负责。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeModeState {
    pub active: bool,
    pub disabled_rows: Vec<String>,
    /// 一键恢复当前可用吗（记账里那份备份**还在**）。`false` = 备份被手动删了/挪了，
    /// 前端据此**不承诺**一键恢复（宁可不给按钮，也不给一个点了会报错的按钮）。
    pub restorable: bool,
}

/// 读记账（无文件 / 解析失败 → `None`：它是壳自己写的文件，损坏时按"未开安全模式"处理）。
pub fn read_journal(data_dir: &Path, profile: &str) -> Option<Journal> {
    let text = std::fs::read_to_string(journal_path(data_dir, profile)).ok()?;
    serde_json::from_str(&text).ok()
}

/// 记账是否属于这个 dsh home（旧格式没有 `dsh_home` → 视为不属于，宁可少认也不误认）。
fn journal_belongs_to(journal: &Journal, home: &Path) -> bool {
    !journal.dsh_home.is_empty() && Path::new(&journal.dsh_home) == home
}

/// 读当前安全模式状态（无记账 / 属于别的 home = 未启用）。
pub fn state(data_dir: &Path, profile: &str, home: &Path) -> SafeModeState {
    match read_journal(data_dir, profile).filter(|j| journal_belongs_to(j, home)) {
        Some(journal) => SafeModeState {
            active: true,
            restorable: Path::new(&journal.patch_backup).is_file(),
            disabled_rows: journal.disabled_rows,
        },
        None => SafeModeState {
            active: false,
            disabled_rows: Vec::new(),
            restorable: false,
        },
    }
}

/// 进入安全模式：把 `ids` 全部写成 `disabled: true`（**一次覆写、一次备份**）并记账。
///
/// 幂等：已经处于停用态的行**不改写**（保住既有条目原文，含行间注释）；
/// 若全部行都已是停用态（`changed=false`）→ **零写入、不写记账**：没有覆写就没有备份，
/// 也就没有"一键恢复"可言——此时如实告诉调用方"本就没有需要改的"。
pub fn enter(
    home: &Path,
    data_dir: &Path,
    profile: &str,
    ids: &[String],
) -> Result<EnterOutcome, String> {
    crate::profiles::validate_profile_name(profile)?;
    if ids.is_empty() {
        return Err(
            "没有可停用的三方挂载行（该 profile 的挂载行全部来自随包插件）——\
             这类失败不是安全模式能解决的。"
                .to_string(),
        );
    }
    let patch_path = profile_patch_path(home, profile);
    let text = std::fs::read_to_string(&patch_path).map_err(|e| {
        format!(
            "读取 {} 失败：{e}（profile 尚未初始化？壳不代 dsh 生成三件套）",
            patch_path.display()
        )
    })?;
    let mut patch = crate::plugins::PatchFile::from_text(&text)?;
    let mut changed = false;
    for id in ids {
        crate::plugins::validate_row_id(id)?;
        changed |= crate::plugins::apply_disabled_toggle(&mut patch, id, true);
    }
    if !changed {
        return Ok(EnterOutcome {
            changed: false,
            backup: None,
        });
    }
    let backup = patch
        .write_with_backup(&patch_path)?
        .ok_or_else(|| format!("{} 不存在，无法备份", patch_path.display()))?;
    // 已有记账且属于**别的 dsh home** → 先归档，别静默抹掉那一侧的恢复指针（它还能用）。
    if let Some(existing) = read_journal(data_dir, profile) {
        if !journal_belongs_to(&existing, home) {
            let path = journal_path(data_dir, profile);
            let archived = path.with_file_name(format!("{profile}.json.other-home-{}", now_unix()));
            let _ = std::fs::rename(&path, &archived);
        }
    }
    let journal = Journal {
        disabled_rows: ids.to_vec(),
        patch_backup: backup.display().to_string(),
        dsh_home: home.display().to_string(),
        applied_at: now_unix(),
    };
    write_journal(data_dir, profile, &journal)?;
    remove_legacy_overlay(data_dir, profile);
    Ok(EnterOutcome {
        changed: true,
        backup: Some(backup),
    })
}

/// **一键恢复**（维护者 2026-09-16 口径："就是把备份好的配置文件覆盖回去"）。
///
/// 逐字节覆盖，**不重新解析**：这样即使当前配置已经写坏也能恢复。
/// 覆盖前把**当前**配置再备份一份——用户在安全模式期间改动过的插件配置不会被无声抹掉。
pub fn exit(home: &Path, data_dir: &Path, profile: &str) -> Result<ExitOutcome, String> {
    crate::profiles::validate_profile_name(profile)?;
    let Some(journal) = read_journal(data_dir, profile).filter(|j| journal_belongs_to(j, home))
    else {
        return Ok(ExitOutcome {
            restored: false,
            backup_used: None,
            current_backup: None,
        });
    };
    let patch_path = profile_patch_path(home, profile);
    let backup = PathBuf::from(&journal.patch_backup);
    validate_backup_path(&patch_path, &backup)?;
    let text = std::fs::read_to_string(&backup).map_err(|e| {
        format!(
            "备份已不在或读不出来（{}）：{e}\n\
             ——安全模式仍生效；请手动把该 profile 的 cordis.patch.yml 改回原样\
             （这几行是我们加进去的停用桩：{}）。",
            backup.display(),
            journal.disabled_rows.join("、")
        )
    })?;
    let current_backup = crate::fs_backup::backup_before_overwrite_path(&patch_path)?;
    crate::plugins::atomic_replace(&patch_path, &text)?;
    remove_journal(data_dir, profile);
    remove_legacy_overlay(data_dir, profile);
    Ok(ExitOutcome {
        restored: true,
        backup_used: Some(backup),
        current_backup,
    })
}

/// 备份路径必须是**同一个 profile 目录下**的 `cordis.patch.yml.bak-*`。
///
/// 记账文件理论上可能被改坏；恢复是破坏性动作，故**先验证再覆盖**——宁可报错，
/// 也不拿一个指向别处的路径去覆盖配置。
fn validate_backup_path(patch_path: &Path, backup: &Path) -> Result<(), String> {
    let same_dir = patch_path.parent() == backup.parent();
    let name = backup
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if !same_dir || !name.starts_with("cordis.patch.yml.bak-") {
        return Err(format!(
            "记账里的备份路径不可信（{}）：只接受同目录的 cordis.patch.yml.bak-*，\
             拒绝用它覆盖配置。",
            backup.display()
        ));
    }
    Ok(())
}

fn write_journal(data_dir: &Path, profile: &str, journal: &Journal) -> Result<(), String> {
    let path = journal_path(data_dir, profile);
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| format!("创建 {} 失败：{e}", dir.display()))?;
    let text = serde_json::to_string_pretty(journal)
        .map_err(|e| format!("序列化安全模式记账失败：{e}"))?;
    crate::plugins::atomic_replace(&path, &format!("{text}\n"))
}

fn remove_journal(data_dir: &Path, profile: &str) {
    let _ = std::fs::remove_file(journal_path(data_dir, profile));
}

/// 清掉 ADR-0025 时代的临时 overlay（机制已退役）：留着只会误导排障。
fn remove_legacy_overlay(data_dir: &Path, profile: &str) {
    let _ = std::fs::remove_file(data_dir.join("safe-mode").join(format!("{profile}.yml")));
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// **兜底（配置已写坏）**：把 `cordis.patch.yml` 备份后放空。
///
/// YAML 语法坏时行枚举不出来（`--dump-config` exit 1），连停用桩都写不进去；此时唯一的
/// 出路是"备份 + 放空"。**调用方必须先经用户确认**（前端 ConfirmDialog）。
pub fn quarantine_patch(home: &Path, profile: &str) -> Result<PathBuf, String> {
    crate::profiles::validate_profile_name(profile)?;
    let path = profile_patch_path(home, profile);
    if !path.is_file() {
        return Err(format!(
            "{} 不存在（该 profile 尚未初始化？）",
            path.display()
        ));
    }
    crate::fs_backup::backup_before_overwrite(&path)?;
    crate::plugins::atomic_replace(&path, "[]\n")?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dsh-safe-mode-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 造一个最小 profile 目录（含 patch 文件原文：注释与既有条目都要能保真）。
    fn profile_with_patch(home: &Path, body: &str) -> PathBuf {
        let dir = home.join("profiles").join("web");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cordis.patch.yml");
        std::fs::write(&path, body).unwrap();
        path
    }

    const PATCH: &str = "# 用户注释（必须保真）\n- insert:\n  - id: dsh-dock--a\n    name: '@deepseek-ai/dsh-browser-use'\n";

    /// 停用判据：**非随包行一律停**（用户 patch 行 + 三方 bundle 层行），随包行永不停——
    /// 哪怕它被用户 patch 过（标签带 `, patched by <文件>`）也不能停。
    #[test]
    fn disables_non_shipped_rows_and_keeps_shipped_ones() {
        // 随包两层：保留（这是 2026-09-16 exit 1 的教训：随包行停一条都起不来）
        assert!(!should_disable(Some("@deepseek-ai/dsh-base")));
        assert!(!should_disable(Some("@deepseek-ai/dsh-web-app")));
        assert!(
            !should_disable(Some(
                "@deepseek-ai/dsh-base, patched by /Users/x/.dsh-dock-dev/profiles/web/cordis.patch.yml"
            )),
            "被用户 patch 过的随包行**不得**停：停掉 tools 这类行会让 agent-loop 悬空（实测 exit 1）"
        );
        // 三方 bundle 层：停（2026-09-16 复测：9 行全停 → 8.2s 正常就绪）
        assert!(should_disable(Some(
            "@deepseek-ai/dsh-experimental-agent-team-profile"
        )));
        assert!(should_disable(Some(
            "@deepseek-ai/dsh-experimental-auto-review"
        )));
        // 随包全表（上游 6 个模板 bundle）都不得停
        for shipped in [
            "@deepseek-ai/dsh-base",
            "@deepseek-ai/dsh-web-app",
            "@deepseek-ai/dsh-headless",
            "@deepseek-ai/dsh-acp-app",
            "@deepseek-ai/dsh-sdk-app",
            "@deepseek-ai/dsh-sdk-minimal",
        ] {
            assert!(!should_disable(Some(shipped)), "{shipped} 是随包层，不得停");
        }
        // 用户 patch 行（段落头是文件路径）：停
        assert!(should_disable(Some(
            "/Users/x/.dsh-dock-dev/profiles/web/cordis.patch.yml"
        )));
        // 无归属（格式漂移/解析退化）：**不停**——失效方向选"宁可不救，不可救坏"
        assert!(!should_disable(None));
    }

    /// 主段解析：`patched by` 尾巴必须去掉（否则随包行会被误判成用户行）。
    #[test]
    fn primary_section_drops_patched_by_tail() {
        assert_eq!(
            primary_section("@deepseek-ai/dsh-base, patched by /x/cordis.patch.yml"),
            "@deepseek-ai/dsh-base"
        );
        assert_eq!(
            primary_section("@deepseek-ai/dsh-base, patched by @deepseek-ai/dsh-web-app"),
            "@deepseek-ai/dsh-base"
        );
        assert_eq!(
            primary_section("@deepseek-ai/dsh-web-app"),
            "@deepseek-ai/dsh-web-app"
        );
    }

    /// 停放计划（纯函数）：真机形态 9 行（5 用户 + 4 三方 bundle）；随包行一条都不进。
    #[test]
    fn split_by_layer_covers_user_and_third_party_bundle_rows() {
        let row = |id: &str, by: Option<&str>| crate::plugins::RowAttribution {
            id: id.to_string(),
            contributed_by: by.map(str::to_string),
        };
        let rows = vec![
            row("timer", Some("@deepseek-ai/dsh-base")),
            row(
                "tools",
                Some("@deepseek-ai/dsh-base, patched by /x/profiles/web/cordis.patch.yml"),
            ),
            row(
                "agent-team",
                Some("@deepseek-ai/dsh-experimental-agent-team-profile"),
            ),
            row(
                "auto-review",
                Some("@deepseek-ai/dsh-experimental-auto-review"),
            ),
            row("dsh-dock--a", Some("/x/profiles/web/cordis.patch.yml")),
            row("dsh-dock--b", None),
        ];
        let profile_patch = Path::new("/x/profiles/web/cordis.patch.yml");
        let (ids, elsewhere) = split_by_layer(&rows, profile_patch);
        assert_eq!(ids, vec!["agent-team", "auto-review", "dsh-dock--a"]);
        assert_eq!(
            elsewhere,
            vec!["dsh-dock--b"],
            "无归属行（格式漂移/解析退化）不停，但要**如实上报**够不到，不能静默跳过"
        );

        // home 层（层序更晚）的行：profile 层的桩够不到 → 归入"停不掉"，如实上报
        let with_home = vec![
            row("dsh-dock--p", Some("/x/profiles/web/cordis.patch.yml")),
            row("dsh-dock--h", Some("/x/cordis.patch.yml")),
            row("orphan", None),
        ];
        let (ids, elsewhere) = split_by_layer(&with_home, profile_patch);
        assert_eq!(ids, vec!["dsh-dock--p"]);
        assert_eq!(elsewhere, vec!["dsh-dock--h", "orphan"]);

        // 只有随包行 → 空计划（命令层据此如实报错，不空转）
        let shipped_only = vec![row("timer", Some("@deepseek-ai/dsh-base"))];
        let (ids, elsewhere) = split_by_layer(&shipped_only, profile_patch);
        assert!(ids.is_empty() && elsewhere.is_empty());
    }

    /// 进入 → 记账 → 一键恢复 的完整往返：配置回到进入前**逐字节**相同，注释保真。
    #[test]
    fn enter_then_exit_restores_the_exact_previous_config() {
        let home = tmp("rt-home");
        let data = tmp("rt-data");
        let patch = profile_with_patch(&home, PATCH);

        let outcome = enter(
            &home,
            &data,
            "web",
            &["dsh-dock--a".to_string(), "agent-team".to_string()],
        )
        .unwrap();
        assert!(outcome.changed);
        let backup = outcome.backup.clone().expect("覆写必须留下备份");
        assert!(backup.is_file());

        let after = std::fs::read_to_string(&patch).unwrap();
        assert!(after.contains("dsh-dock--a"), "原有条目必须保留：{after}");
        assert!(after.contains("disabled: true"), "应写入停用桩：{after}");
        assert!(
            after.contains("# 用户注释（必须保真）"),
            "注释必须保真：{after}"
        );
        // 两条都停了：insert 行就地置 disabled；未出现的 id 追加双键条目
        assert_eq!(after.matches("disabled: true").count(), 2, "{after}");

        let snapshot = state(&data, "web", &home);
        assert!(snapshot.active && snapshot.restorable);
        assert_eq!(snapshot.disabled_rows.len(), 2);

        // 一键恢复 = 用备份覆盖回去
        let exit = exit(&home, &data, "web").unwrap();
        assert!(exit.restored);
        assert_eq!(exit.backup_used.as_deref(), Some(backup.as_path()));
        assert_eq!(
            std::fs::read_to_string(&patch).unwrap(),
            PATCH,
            "恢复后必须与进入前逐字节一致"
        );
        assert!(!state(&data, "web", &home).active, "恢复后记账必须清掉");
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&data);
    }

    /// 幂等：第二次进入**零写入**（不动 mtime、不留多余备份），也不覆盖已有记账——
    /// 否则"一键恢复"会指向"安全模式之后"的状态。
    #[test]
    fn second_enter_is_a_no_op_and_keeps_the_original_backup() {
        let home = tmp("idem-home");
        let data = tmp("idem-data");
        profile_with_patch(&home, PATCH);
        let ids = vec!["dsh-dock--a".to_string()];

        let first = enter(&home, &data, "web", &ids).unwrap();
        let first_backup = first.backup.clone().unwrap();
        let journal_first = read_journal(&data, "web").unwrap();

        let second = enter(&home, &data, "web", &ids).unwrap();
        assert!(!second.changed, "已是停用态 → 零写入");
        assert!(second.backup.is_none());
        assert_eq!(
            read_journal(&data, "web").unwrap(),
            journal_first,
            "记账必须仍指向第一份备份（进入安全模式前的状态）"
        );
        assert!(first_backup.is_file());
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&data);
    }

    /// 幂等反向断言：进入前若**没有**覆写（changed=false）就不该写记账——
    /// 没有备份就没有"一键恢复"，不能凭空承诺。
    #[test]
    fn no_write_means_no_journal() {
        let home = tmp("noop-home");
        let data = tmp("noop-data");
        profile_with_patch(
            &home,
            &format!("{PATCH}- id: dsh-dock--a\n  disabled: true\n"),
        );
        let outcome = enter(&home, &data, "web", &["dsh-dock--a".to_string()]).unwrap();
        assert!(!outcome.changed);
        assert!(read_journal(&data, "web").is_none());
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&data);
    }

    /// 未进入安全模式时点恢复：幂等，报"本就不在安全模式"，不动文件。
    #[test]
    fn exit_without_journal_is_a_no_op() {
        let home = tmp("noop2-home");
        let data = tmp("noop2-data");
        let patch = profile_with_patch(&home, PATCH);
        let outcome = exit(&home, &data, "web").unwrap();
        assert!(!outcome.restored);
        assert_eq!(std::fs::read_to_string(&patch).unwrap(), PATCH);
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&data);
    }

    /// 备份被用户删掉：状态里 `restorable=false`（前端不承诺一键恢复），
    /// 点恢复时明确报错并给出"我们加过哪几行"，不静默瞎恢复。
    #[test]
    fn missing_backup_is_reported_not_guessed() {
        let home = tmp("miss-home");
        let data = tmp("miss-data");
        profile_with_patch(&home, PATCH);
        enter(&home, &data, "web", &["dsh-dock--a".to_string()]).unwrap();
        let backup = PathBuf::from(&read_journal(&data, "web").unwrap().patch_backup);
        std::fs::remove_file(&backup).unwrap();

        let snapshot = state(&data, "web", &home);
        assert!(snapshot.active);
        assert!(!snapshot.restorable, "备份没了就不能承诺一键恢复");

        let err = exit(&home, &data, "web").unwrap_err();
        assert!(err.contains("备份已不在"), "{err}");
        assert!(
            err.contains("dsh-dock--a"),
            "要告诉用户我们加过哪几行：{err}"
        );
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&data);
    }

    /// 记账里的备份路径不可信（指向别处 / 不是 .bak-*）→ 拒绝覆盖（破坏性动作先验证）。
    #[test]
    fn untrusted_backup_path_is_refused() {
        let home = tmp("evil-home");
        let data = tmp("evil-data");
        let patch = profile_with_patch(&home, PATCH);
        let outside = home.join("elsewhere.yml");
        std::fs::write(&outside, "[]\n").unwrap();
        let journal = Journal {
            disabled_rows: vec!["dsh-dock--a".to_string()],
            patch_backup: outside.display().to_string(),
            dsh_home: home.display().to_string(),
            applied_at: 1,
        };
        write_journal(&data, "web", &journal).unwrap();
        let err = exit(&home, &data, "web").unwrap_err();
        assert!(err.contains("不可信"), "{err}");
        assert_eq!(std::fs::read_to_string(&patch).unwrap(), PATCH, "拒绝覆盖");
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&data);
    }

    /// 无三方行 → 明确报错（命令层不空转启动）。
    #[test]
    fn enter_without_rows_reports_instead_of_pretending() {
        let home = tmp("empty-home");
        let data = tmp("empty-data");
        profile_with_patch(&home, PATCH);
        let err = enter(&home, &data, "web", &[]).unwrap_err();
        assert!(err.contains("没有可停用"), "{err}");
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&data);
    }

    /// 兜底：备份 + 放空（配置已写坏时用；调用方必须先经用户确认）。
    #[test]
    fn quarantine_backs_up_then_empties() {
        let home = tmp("quar-home");
        let patch = profile_with_patch(&home, "- insert:\n  - id: bad\n");
        quarantine_patch(&home, "web").unwrap();
        assert_eq!(std::fs::read_to_string(&patch).unwrap(), "[]\n");
        let backups: Vec<_> = std::fs::read_dir(patch.parent().unwrap())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".bak-"))
            .collect();
        assert_eq!(backups.len(), 1, "必须留下恰好一份备份");
        assert!(quarantine_patch(&home, "nope").is_err());
        let _ = std::fs::remove_dir_all(&home);
    }

    /// 退役的临时 overlay 文件：进入/退出都顺手清掉（留着只会误导排障）。
    #[test]
    fn legacy_overlay_is_cleaned_up() {
        let home = tmp("legacy-home");
        let data = tmp("legacy-data");
        profile_with_patch(&home, PATCH);
        let legacy = data.join("safe-mode").join("web.yml");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(&legacy, "- id: whatever\n  disabled: true\n").unwrap();

        enter(&home, &data, "web", &["dsh-dock--a".to_string()]).unwrap();
        assert!(!legacy.exists(), "进入安全模式时应清掉旧 overlay");

        std::fs::write(&legacy, "- id: whatever\n  disabled: true\n").unwrap();
        exit(&home, &data, "web").unwrap();
        assert!(!legacy.exists(), "退出时同样清掉");
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&data);
    }
}
