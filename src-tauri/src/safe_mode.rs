//! safe_mode.rs —— 启动失败的**安全模式**（ADR-0025，2026-09-16 维护者裁定）。
//!
//! ## 解决什么
//!
//! 一条坏的插件行会让**整棵 plugin tree 拒绝加载**：真机两次实测 dsh 在 34s / 44s 后
//! 退出码 1，用户**进不去应用界面**，只能手工改 `cordis.patch.yml` 才能恢复。
//! 「点名出错行 + 移除该行并重启」只在报错确实点名了某一行时可用；多点坏行、YAML
//! 语法坏、或用户只想"先能用起来"时，需要一条**不依赖定位到具体行**的退路。
//!
//! ## 机制（上游既有能力，零 dsh 文件改动）
//!
//! `dsh --patch <overlay>` 是**最高优先级**的 patch overlay（层栈
//! `bundle → profile 层 → home 层 → --patch`，各层挂载前拍平成单列表），因此可用
//! `- id: <行>` + `disabled: true` 停用**任何更早层**的行；overlay 里匹配不到的 id
//! 上游只 warning 不报错（`vendor/include/src/index.ts:110-112`），故"能禁就禁"是安全的。
//!
//! overlay 落在**壳自有数据目录**（`<app_data>/safe-mode/<profile>.yml`），**不写进
//! profile 目录、不改任何 dsh 文件**；「退出安全模式」= 删掉该文件（原子回退）。
//!
//! ## 停用范围（**只停用户层行**——A+ 口径已被实测推翻）
//!
//! 只停"用户自己加的东西"：profile 层 / home 层 patch 行（`contributed_by` 是文件路径
//! 或无段落归属）。随包层与**第三方 bundle 层**一律保留，详见 [`should_disable`] 的
//! 判据说明与 ADR-0025 §4 的实测记录（整层摘掉 ⇒ `exit 1`）。官方桌面 app 的等价按钮
//! 只重置 `dsh.profile.bundles`（`apps/desktop/src/project-manager.ts:335-344`）——
//! **治不了 `cordis.patch.yml` 的 insert 行**，即本仓库这次踩的病，故不能照搬。
//!
//! ## 分层兜底（方案 B）
//!
//! YAML 语法坏时 `--dump-config` 直接 exit 1（实测），**枚举不出行** → 只能把
//! `cordis.patch.yml` **备份后放空**（[`quarantine_patch`]）。这是本模块唯一会碰用户
//! 文件的分支，且必须由用户在确认框里显式同意（调用方职责）。
use std::path::{Path, PathBuf};

/// overlay 存放目录（壳自有数据目录下，绝不写进 profile）。
pub fn overlay_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("safe-mode")
}

/// 某 profile 的 overlay 路径。`profile` 必须已经过调用方校验（本函数只拼路径）。
pub fn overlay_path(data_dir: &Path, profile: &str) -> PathBuf {
    overlay_dir(data_dir).join(format!("{profile}.yml"))
}

/// 该行是否应在安全模式下停用（**纯函数，判据单源**）。
///
/// 口径 = **只停用户层行**（profile 层 / home 层）；bundle 层行（随包与第三方）一律保留。
///
/// **为什么不是"连第三方 bundle 层行一起停"**（2026-09-16 实测推翻更猛的 A+ 口径）：
/// 真机 profile 上停 33 行（含第三方层行）→ dsh **exit 1**，stderr 报
/// `6 entries did not activate` + 若干 `pending (waiting for service: tools)` ——
/// 第三方层的行并非孤立插件，与被保留层之间存在服务依赖，整层摘掉会让依赖悬空。
/// 同一 profile 只停用户层 5 行 → **正常就绪**。故安全模式只停"用户自己加的东西"，
/// 不碰随包与第三方**层**结构；单点坏行仍由「只移除出错的那一行并重启」处理
/// （2026-09-16 维护者反馈后：首屏一个按钮，这一条在错误卡"展开详情 → 其它出路"里）。
///
/// 判据：`contributed_by` 为 `None`（无段落归属）或指向**文件**（用户 patch 行的段落头
/// 是文件路径，如 `/Users/…/profiles/web/cordis.patch.yml`）。
pub fn should_disable(contributed_by: Option<&str>) -> bool {
    match contributed_by {
        None => true,
        Some(section) => looks_like_file_section(section),
    }
}

/// 段落头是不是**文件路径**（用户层的标志），而不是包名。
fn looks_like_file_section(section: &str) -> bool {
    section.starts_with('/') || section.contains(".yml") || section.contains('\\')
}

/// 安全模式的**停放计划**（纯函数，判据单源）：给定全部挂载行的归属，算出要停用的 id 表。
///
/// **空表 ≠ "没什么可停"这么简单**：它意味着**本机制管辖不到这次失败**——坏行来自
/// 随包 / 第三方**插件包**（不在用户 patch 层）。调用方（`commands/boot.rs` 的
/// `safe_mode` 分支）必须据此**拒绝空转启动**并如实报错（2026-09-16 诚实门）；
/// 起了也是同一张错误卡，只会把"点过按钮却回到原点"变成新的困惑。
///
/// 抽成纯函数是为了能机测这条判据：命令分支本身要 `AppHandle` 且在线程里跑，单测够不着。
pub fn disable_plan(rows: &[crate::plugins::RowAttribution]) -> Vec<String> {
    rows.iter()
        .filter(|r| should_disable(r.contributed_by.as_deref()))
        .map(|r| r.id.clone())
        .collect()
}

/// 生成 overlay 文本（纯函数；上游要求顶层 YAML 数组，每项是 mapping）。
pub fn overlay_text(ids: &[String]) -> String {
    let mut out = String::from(
        "# dsh-dock 安全模式（ADR-0025，临时文件，删除即退出安全模式）\n\
         # 由「实验能力」错误卡生成：停用全部非随包层行，只保留 dsh-base / dsh-web-app。\n",
    );
    for id in ids {
        out.push_str(&format!("- id: {id}\n  disabled: true\n"));
    }
    out
}

/// 写入 overlay（原子：同目录 tmp + rename；先建目录）。返回落盘路径。
///
/// 空 `ids` 也会写（一个只含注释的文件 = 合法 YAML 数组的"空"形态？不是——
/// 上游要求顶层数组，故空表写成 `[]`，避免"注释文件解析失败"这种自伤）。
pub fn write_overlay(data_dir: &Path, profile: &str, ids: &[String]) -> Result<PathBuf, String> {
    let dir = overlay_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建 {} 失败：{e}", dir.display()))?;
    let path = overlay_path(data_dir, profile);
    let body = if ids.is_empty() {
        format!(
            "{}\n[]\n",
            overlay_text(&[])
                .lines()
                .take(2)
                .collect::<Vec<_>>()
                .join("\n")
        )
    } else {
        overlay_text(ids)
    };
    let tmp = dir.join(format!("{profile}.yml.tmp"));
    std::fs::write(&tmp, body).map_err(|e| format!("写 {} 失败：{e}", tmp.display()))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("替换 {} 失败：{e}", path.display()))?;
    Ok(path)
}

/// 读回 overlay 里已停用的行 id（无文件 → 空表）。解析失败按空表处理并**不报错**：
/// 它是壳自己写的文件，损坏时最合理的动作是"当作没开安全模式"。
pub fn disabled_rows(data_dir: &Path, profile: &str) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(overlay_path(data_dir, profile)) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim_start();
        if let Some(rest) = line.strip_prefix("- id:") {
            let id = rest.trim();
            if !id.is_empty() {
                out.push(id.to_string());
            }
        }
    }
    out
}

/// 安全模式状态（给前端的**只读**快照：横幅/文案用）。
///
/// 边界：只报"壳自己的 overlay 文件在不在、停了哪些行"——**不报运行态**。
/// 运行态由回环快照（`plugins::fetch_runtime_snapshot`）负责，两者禁止混一条数据。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SafeModeState {
    /// 本轮是否以安全模式启动。
    pub active: bool,
    /// 被临时停用的行 id（空 = 未启用安全模式）。
    pub disabled_rows: Vec<String>,
}

/// 读当前安全模式状态（无文件 = 未启用）。
pub fn state(data_dir: &Path, profile: &str) -> SafeModeState {
    SafeModeState {
        active: is_active(data_dir, profile),
        disabled_rows: disabled_rows(data_dir, profile),
    }
}

/// 安全模式当前是否生效。
pub fn is_active(data_dir: &Path, profile: &str) -> bool {
    overlay_path(data_dir, profile).is_file()
}

/// 生效时返回 overlay 路径（供 `resolve_launch` 决定是否给 dsh 传 `--patch`）。
pub fn active_overlay(data_dir: &Path, profile: &str) -> Option<PathBuf> {
    is_active(data_dir, profile).then(|| overlay_path(data_dir, profile))
}

/// 退出安全模式：删除 overlay（幂等；返回是否真的删掉了文件）。
pub fn clear(data_dir: &Path, profile: &str) -> Result<bool, String> {
    let path = overlay_path(data_dir, profile);
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("删除 {} 失败：{e}", path.display())),
    }
}

/// **方案 B**：把 `cordis.patch.yml` 备份后放空（YAML 语法坏、行枚举不出来时的唯一出路）。
///
/// 备份走既有的 [`crate::fs_backup::backup_before_overwrite`]（`.bak-<unix秒>`，AGENTS §6
/// 已登记资产），放空写 `[]`（dsh 模板的"空 patch 层"形态）。**调用方必须先经用户确认**。
pub fn quarantine_patch(home: &Path, profile: &str) -> Result<PathBuf, String> {
    crate::profiles::validate_profile_name(profile)?;
    let path = home.join("profiles").join(profile).join("cordis.patch.yml");
    if !path.is_file() {
        return Err(format!(
            "{} 不存在（该 profile 尚未初始化？）",
            path.display()
        ));
    }
    crate::fs_backup::backup_before_overwrite(&path)?;
    std::fs::write(&path, "[]\n").map_err(|e| format!("写入 {} 失败：{e}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dsh-safe-mode-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 停用判据（**只停用户层行**）：任何 bundle 层行（含第三方层）都保留。
    #[test]
    fn only_user_layer_rows_are_disabled() {
        assert!(!should_disable(Some("@deepseek-ai/dsh-base")));
        assert!(!should_disable(Some("@deepseek-ai/dsh-web-app")));
        assert!(
            !should_disable(Some("@deepseek-ai/dsh-experimental-agent-team-profile")),
            "第三方 bundle 层行不得停：整层摘掉会让服务依赖悬空（2026-09-16 实测 exit 1）"
        );
        assert!(should_disable(Some(
            "/Users/x/.dsh-dock-dev/profiles/web/cordis.patch.yml"
        )));
        assert!(should_disable(Some("/Users/x/.dsh/cordis.patch.yml")));
        assert!(should_disable(None), "无归属 = 用户 patch 行，必须停");
    }

    /// **停放计划 + 诚实门**（2026-09-16）：只停用户层行；**全是 bundle 层行时计划为空**
    /// ——命令层据此拒绝空转启动（否则用户点完按钮会回到同一张错误卡）。
    #[test]
    fn disable_plan_covers_user_rows_and_flags_bundle_only_profiles() {
        let row = |id: &str, by: Option<&str>| crate::plugins::RowAttribution {
            id: id.to_string(),
            contributed_by: by.map(str::to_string),
        };
        // 混合（真机形态）：只挑用户层两行，bundle 层一行保留。
        let mixed = vec![
            row(
                "dsh-dock--a",
                Some("/Users/x/.dsh-dock-dev/profiles/web/cordis.patch.yml"),
            ),
            row("dsh-dock--b", None),
            row(
                "agent-team-profile",
                Some("@deepseek-ai/dsh-experimental-agent-team-profile"),
            ),
        ];
        assert_eq!(disable_plan(&mixed), vec!["dsh-dock--a", "dsh-dock--b"]);

        // 全 bundle 层（坏行来自第三方插件包）→ **空计划 = 本机制管辖不到**。
        let bundle_only = vec![
            row(
                "agent-team-profile",
                Some("@deepseek-ai/dsh-experimental-agent-team-profile"),
            ),
            row(
                "auto-review",
                Some("@deepseek-ai/dsh-experimental-auto-review"),
            ),
        ];
        assert!(
            disable_plan(&bundle_only).is_empty(),
            "全 bundle 层时必须给空计划，好让命令层拒绝空转启动"
        );
        // 反向：一条用户行就足以让计划非空（别把可用场景误判成"管辖不到"）。
        let one_user = vec![row("dsh-dock--x", None)];
        assert_eq!(disable_plan(&one_user), vec!["dsh-dock--x"]);
    }

    /// overlay 文本形态：顶层数组 + 每行 `- id:` / `disabled: true`（上游要求）。
    #[test]
    fn overlay_text_is_a_top_level_patch_array() {
        let text = overlay_text(&["a".to_string(), "b".to_string()]);
        assert!(text.starts_with('#'), "带注释头：{text}");
        assert!(text.contains("- id: a\n  disabled: true\n"), "{text}");
        assert!(text.contains("- id: b\n  disabled: true\n"), "{text}");
        assert!(!text.contains("insert"), "安全模式不插行：{text}");
    }

    /// 写 → 读 → 生效 → 退出 的往返；空表写成合法 YAML 数组而不是纯注释。
    #[test]
    fn write_read_clear_round_trip() {
        let dir = tmp();
        assert!(!is_active(&dir, "web"));
        assert!(disabled_rows(&dir, "web").is_empty());

        let ids = vec!["dsh-dock-a".to_string(), "tool-agent-team".to_string()];
        let path = write_overlay(&dir, "web", &ids).unwrap();
        assert!(path.is_file());
        assert!(is_active(&dir, "web"));
        assert_eq!(disabled_rows(&dir, "web"), ids);
        assert_eq!(
            active_overlay(&dir, "web").as_deref(),
            Some(path.as_path()),
            "生效时要把路径交给 spawn 侧"
        );

        assert!(clear(&dir, "web").unwrap());
        assert!(!is_active(&dir, "web"));
        assert!(!clear(&dir, "web").unwrap(), "退出是幂等的");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 空 id 表：必须写成 `[]`（顶层数组），否则上游解析报错、安全模式反而起不来。
    #[test]
    fn empty_overlay_is_still_a_valid_array() {
        let dir = tmp();
        let path = write_overlay(&dir, "web", &[]).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.trim_end().ends_with("[]"), "{text}");
        assert!(disabled_rows(&dir, "web").is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 状态快照：未启用 = `active:false` + 空表；启用后行数与停用集合一致。
    #[test]
    fn state_reports_active_and_rows() {
        let dir = tmp();
        assert_eq!(
            state(&dir, "web"),
            SafeModeState {
                active: false,
                disabled_rows: Vec::new()
            }
        );
        let ids = vec!["dsh-dock-a".to_string()];
        write_overlay(&dir, "web", &ids).unwrap();
        let s = state(&dir, "web");
        assert!(s.active);
        assert_eq!(s.disabled_rows, ids);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// 方案 B：备份 + 放空；备份文件必须存在（用户可随时还原）。
    #[test]
    fn quarantine_backs_up_then_empties() {
        let home = tmp();
        let profile = "web";
        let dir = home.join("profiles").join(profile);
        std::fs::create_dir_all(&dir).unwrap();
        let patch = dir.join("cordis.patch.yml");
        std::fs::write(&patch, "- insert:\n    - id: bad\n      name: 'x'\n").unwrap();

        quarantine_patch(&home, profile).unwrap();
        assert_eq!(std::fs::read_to_string(&patch).unwrap(), "[]\n");
        let backups: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".bak-"))
            .collect();
        assert_eq!(backups.len(), 1, "必须留下恰好一份备份");

        // 不存在的 profile 目录：明确报错，不静默建目录。
        assert!(quarantine_patch(&home, "nope").is_err());
        std::fs::remove_dir_all(&home).ok();
    }
}
