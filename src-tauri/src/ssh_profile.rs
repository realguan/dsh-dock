//! ssh_profile.rs —— SSH 远程工作区 profile 的**生成**（2026-09-15，ADR-0023 §2.2 / §2.4 / §2.8）。
//!
//! ## 产出形态
//!
//! 一个**自建 profile**（`dsh.profile.bundles` 声明 `@deepseek-ai/dsh-headless`，见
//! [`crate::profiles::SSH_APP_BUNDLE`] 的理由），其 `cordis.patch.yml` 以 `insert` 形式
//! 注册**四个** ssh 包，**每行带稳定 `id`**，且**只有 `dsh-ssh` 带 `config`**。
//!
//! ## 四条不能省的纪律
//!
//! 1. **四包齐注册**（ADR-0023 §2.2）：`dsh-ssh` 只提供 `ctx.ssh`；缺
//!    `fs-ssh` / `subprocess-ssh` / `sandbox-ssh` 就没有文件、进程与沙箱服务。
//!    依赖边是 `fs-ssh → ['ssh','sandboxPolicy']`、`subprocess-ssh → ['ssh']`、
//!    `sandbox-ssh → ['ssh']`，故 **`dsh-ssh` 必须排在最前**。
//! 2. **每行必须有稳定 `id`**（§2.4）：loader 按 `id` 建 store
//!    （`vendor/loader/src/config/tree.ts:51-59`），缺 `id` 会被自动生成，该行**此后
//!    永远无法被 patch 命中**。
//! 3. **只有 `dsh-ssh` 带 `config`**：其余三个 README 明写无配置；给它们写 `config`
//!    会被 schema 拒绝。
//! 4. **写后自证**：`config` 键是**整体替换**语义（`docs/roadmap.md:31`），一次
//!    写错就没有"深合并"兜底。故写完必须回读确认，不能靠"写成功了"当结论。
//!
//! ## 不做什么
//!
//! - **不装包**：四个包的安装复用既有 `install_plugin`（经前端队列，用户能看到
//!   「下载管理」进度与失败重试）。本模块只写 patch 行——把 pnpm 拉进一个"生成"命令
//!   会让它变成数分钟级的黑盒 IPC。
//! - **不承诺 Web 视图远端感知**（§2.8）：正因如此 app bundle 取 headless 而非 web-app。

use std::path::Path;

use serde_yaml::Value;

use crate::plugins::PatchFile;
use crate::ssh_remote::SshTarget;

/// 一个待注册的 ssh 包（行 id / 包名 / 是否带 `config`）。
///
/// **顺序即语义**：`dsh-ssh` 必须第一（其余三个都 inject 它）。
pub const SSH_PACKAGES: &[(&str, &str)] = &[
    // 行 id 带 `dsh-dock-` 前缀，与策展目录的挂载行同一命名口径，便于人在
    // `cordis.patch.yml` 里一眼认出"这是壳写的行"。
    ("dsh-dock-ssh", "@deepseek-ai/dsh-ssh"),
    ("dsh-dock-fs-ssh", "@deepseek-ai/dsh-fs-ssh"),
    ("dsh-dock-subprocess-ssh", "@deepseek-ai/dsh-subprocess-ssh"),
    ("dsh-dock-sandbox-ssh", "@deepseek-ai/dsh-sandbox-ssh"),
];

/// 带 `config` 的那一行（ADR-0023 §2.2：只有 `dsh-ssh` 接受 `config`）。
pub const SSH_CONFIG_ROW: &str = "dsh-dock-ssh";

/// 五个键的 `config` 映射（`None` = 该包不带 config）。
pub fn config_for(row_id: &str, target: &SshTarget) -> Option<Value> {
    if row_id != SSH_CONFIG_ROW {
        return None;
    }
    let mut m = serde_yaml::Mapping::new();
    for (key, value) in [
        ("host", &target.host),
        ("node", &target.node),
        ("helper", &target.helper),
        ("helperHash", &target.helper_hash),
        ("workspace", &target.workspace),
    ] {
        m.insert(Value::String(key.to_string()), Value::String(value.clone()));
    }
    Some(Value::Mapping(m))
}

/// 在 patch 里注册四行（**纯变换**，返回是否改动）。
///
/// 幂等：任意 `insert` 行里已有同 `id` 即跳过该行（**逐行**判定，不整批短路——
/// 中途失败后重跑要能把缺的那几行补上）。
pub fn apply_ssh_rows(patch: &mut PatchFile, target: &SshTarget) -> bool {
    let insert_key = Value::String("insert".into());
    let id_key = Value::String("id".into());

    // 只读预扫：已存在的行 id + 第一个 `insert` 条目的下标。
    let mut existing: Vec<String> = Vec::new();
    let mut target_idx: Option<usize> = None;
    for (idx, entry) in patch.entries.iter().enumerate() {
        let Some(seq) = entry
            .as_mapping()
            .and_then(|m| m.get(&insert_key))
            .and_then(|v| v.as_sequence())
        else {
            continue;
        };
        if target_idx.is_none() {
            target_idx = Some(idx);
        }
        for row in seq {
            if let Some(id) = row
                .as_mapping()
                .and_then(|r| r.get(&id_key))
                .and_then(|v| v.as_str())
            {
                existing.push(id.to_string());
            }
        }
    }

    // 待写行（去掉已存在的）。
    let mut rows: Vec<Value> = Vec::new();
    for (row_id, package) in SSH_PACKAGES {
        if existing.iter().any(|e| e == row_id) {
            continue;
        }
        let mut row = serde_yaml::Mapping::new();
        row.insert(id_key.clone(), Value::String((*row_id).to_string()));
        row.insert(
            Value::String("name".into()),
            Value::String((*package).to_string()),
        );
        if let Some(config) = config_for(row_id, target) {
            row.insert(Value::String("config".into()), config);
        }
        rows.push(Value::Mapping(row));
    }
    if rows.is_empty() {
        return false;
    }

    match target_idx {
        Some(idx) => {
            // **必须**走 `for_each_entry_mut`：它负责把该条目的原文片段置 `None`。
            // 直接改 `entries` 会让 `render()` 原样回填**旧文本**——静默不生效
            // （同 `plugins::apply_catalog_insert_row` 的注释）。
            let mut handled = false;
            patch.for_each_entry_mut(|i, entry| {
                if i != idx {
                    return false;
                }
                let Some(seq) = entry
                    .as_mapping_mut()
                    .and_then(|m| m.get_mut(&insert_key))
                    .and_then(|v| v.as_sequence_mut())
                else {
                    return false;
                };
                seq.extend(rows.clone());
                handled = true;
                true
            });
            handled
        }
        None => {
            let mut m = serde_yaml::Mapping::new();
            m.insert(insert_key, Value::Sequence(rows));
            patch.push(Value::Mapping(m));
            true
        }
    }
}

/// 把四行写进 `profiles/<profile>/cordis.patch.yml`（经 [`PatchFile`]：覆写前备份 + 原子替换）。
///
/// 返回是否实际改动（幂等时 `false`，零写入——免 mtime 抖动）。
pub fn ensure_ssh_rows(home: &Path, profile: &str, target: &SshTarget) -> Result<bool, String> {
    crate::profiles::validate_profile_name(profile)?;
    let patch_path = home.join("profiles").join(profile).join("cordis.patch.yml");
    if !patch_path.is_file() {
        return Err(format!(
            "profile「{profile}」不存在或未物化（缺 {}）——请先生成 profile",
            patch_path.display()
        ));
    }
    let mut patch = PatchFile::read(&patch_path)?;
    let changed = apply_ssh_rows(&mut patch, target);
    if changed {
        patch.write(&patch_path)?;
    }
    Ok(changed)
}

/// **写后自证**：回读 `cordis.patch.yml`，确认四行都在、且 `dsh-ssh` 那行带着五个键。
///
/// 为什么必须自证：`config` 是整体替换语义，且 `PatchFile` 的原文保真路径曾出过
/// "改了 `entries` 却回填旧文本"的静默失败。只根据"写入调用返回 Ok"下结论，
/// 正是本仓库最忌讳的那类过度声称。
pub fn verify_ssh_rows(home: &Path, profile: &str, target: &SshTarget) -> Result<(), String> {
    let patch_path = home.join("profiles").join(profile).join("cordis.patch.yml");
    let text = std::fs::read_to_string(&patch_path)
        .map_err(|e| format!("回读 {} 失败：{e}", patch_path.display()))?;
    let doc: Value = serde_yaml::from_str(&text)
        .map_err(|e| format!("回读的 {} 不是合法 YAML：{e}", patch_path.display()))?;

    let mut rows: Vec<&serde_yaml::Mapping> = Vec::new();
    if let Some(seq) = doc.as_sequence() {
        for entry in seq {
            if let Some(list) = entry
                .as_mapping()
                .and_then(|m| m.get(Value::String("insert".into())))
                .and_then(|v| v.as_sequence())
            {
                rows.extend(list.iter().filter_map(|r| r.as_mapping()));
            }
        }
    }
    let find = |want: &str| -> Option<&serde_yaml::Mapping> {
        rows.iter()
            .copied()
            .find(|r| r.get(Value::String("id".into())).and_then(|v| v.as_str()) == Some(want))
    };

    for (row_id, package) in SSH_PACKAGES {
        let row = find(row_id)
            .ok_or_else(|| format!("写后自证失败：patch 里没有行 id「{row_id}」（未生效）"))?;
        let name = row
            .get(Value::String("name".into()))
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if name != *package {
            return Err(format!(
                "写后自证失败：行「{row_id}」的 name 是「{name}」，应为「{package}」"
            ));
        }
    }

    // 五键逐项回读比对：只查"存在 config"不够，键值写错同样致命。
    let cfg_row =
        find(SSH_CONFIG_ROW).ok_or_else(|| format!("写后自证失败：缺少 {SSH_CONFIG_ROW} 行"))?;
    let cfg = cfg_row
        .get(Value::String("config".into()))
        .and_then(|v| v.as_mapping())
        .ok_or_else(|| format!("写后自证失败：{SSH_CONFIG_ROW} 行没有 config"))?;
    for (key, want) in [
        ("host", &target.host),
        ("node", &target.node),
        ("helper", &target.helper),
        ("helperHash", &target.helper_hash),
        ("workspace", &target.workspace),
    ] {
        let got = cfg
            .get(Value::String(key.into()))
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if got != want {
            return Err(format!(
                "写后自证失败：config.{key} 回读为「{got}」，应为「{want}」"
            ));
        }
    }

    // 反过来：其余三行**不得**带 config（带了会被 schema 拒绝）。
    for (row_id, _) in SSH_PACKAGES {
        if *row_id == SSH_CONFIG_ROW {
            continue;
        }
        if let Some(row) = find(row_id) {
            if row.get(Value::String("config".into())).is_some() {
                return Err(format!(
                    "写后自证失败：行「{row_id}」不该带 config（该包 README 明写无配置）"
                ));
            }
        }
    }
    Ok(())
}

/// 生成结果（回给向导，用于区分"新建 vs 复用"与"是否真的写了"）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshProfileOutcome {
    /// 本次是否**新建**了 profile（`false` = 复用已存在的同名 profile）。
    pub created: bool,
    /// 是否**改动了** `cordis.patch.yml`（`false` = 四行本就齐全，零写入）。
    pub changed: bool,
}

/// 生成入口：建 profile（如缺）→ **装四包（钉版本）** → 写四行 → **写后自证**。
///
/// ## 为什么四包安装放在这里，而不是复用前端队列
///
/// ① **顺序是硬约束**：挂载行必须**在四包装完之后**才写。反过来（先写行再装包）
///    一旦安装失败就留下指向未安装包的**幽灵挂载行**——dsh 启动时该行会加载失败，
///    比"包装了但没挂上"（可重试、无害）糟糕得多。放进一个命令里就没有中间态。
/// ② **版本必须钉**：`dsh plugin add <裸包名>` 按 `latest` dist-tag 解析，而
///    `@deepseek-ai/*` 的 `latest` 已实测会落后于运行时（台账行 7、ADR-0020 §2.4）。
///    钉版本要用运行期版本，只有后端拿得到——故复用
///    [`crate::official_catalog::pinned_spec`]，不在前端复刻一份。
///
/// 代价（诚实记档）：这是一次**可能数分钟**的调用，且没有逐包进度（队列那套进度
/// 与重试用不上）。向导必须显示明确的进行中文案，不能只转圈。
pub fn generate(
    profile: &str,
    data_dir: &Path,
    target: &SshTarget,
    world: &crate::mgmt::World,
) -> Result<SshProfileOutcome, String> {
    let problems = crate::ssh_remote::validate_target(target);
    if !problems.is_empty() {
        return Err(format!("配置校验未通过：{}", problems.join("；")));
    }
    crate::profiles::validate_profile_name(profile)?;

    let home = crate::resolve::user_dsh_home();
    let manifest = home.join("profiles").join(profile).join("package.json");
    let created = if manifest.is_file() {
        false
    } else {
        // 复用既有创建链（pnpm/toolchain/物化判定全在 `profiles.rs`），只换 app bundle：
        // SSH 工作区取 **headless** 而非 web-app（ADR-0023 §1.3/§2.8）。
        let outcome = crate::profiles::create_profile_with_app_bundle_blocking(
            profile,
            data_dir,
            crate::profiles::ssh_app_bundle(),
        )?;
        if !manifest.is_file() {
            return Err(format!(
                "创建 profile「{profile}」未物化（{} 不存在）：{}",
                manifest.display(),
                outcome.detail
            ));
        }
        true
    };

    // 运行期版本 = 钉版本的依据（与 `list_official_plugins` 同一来源）。
    let version = crate::updates::detect_current_version(data_dir).unwrap_or_default();
    if version.is_empty() {
        return Err(
            "读不到运行时 dsh 版本，无法为 ssh 家族钉版本——裸包名会按 `latest` 装，\
             而 @deepseek-ai/* 的 latest 已实测可能落后于运行时（ADR-0020 §2.4）。\
             请先让壳完成一次版本探测再试。"
                .to_string(),
        );
    }

    for (_, package) in SSH_PACKAGES {
        let spec = crate::official_catalog::pinned_spec(package, &version);
        let outcome = crate::plugins::mutate_plugin_blocking(
            crate::plugins::PluginOp::Install,
            profile,
            &spec,
            data_dir,
            world,
        )?;
        if !outcome.ok {
            return Err(format!(
                "安装「{}」失败：{}——**未写入任何挂载行**（避免留下指向未安装包的幽灵行）；\
                 修好后可重跑本向导（创建与安装均幂等）",
                spec, outcome.detail
            ));
        }
    }

    let changed = ensure_ssh_rows(&home, profile, target)?;
    verify_ssh_rows(&home, profile, target)?;
    Ok(SshProfileOutcome { created, changed })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> SshTarget {
        SshTarget {
            host: "myserver".to_string(),
            node: "/usr/local/bin/node".to_string(),
            helper: "/opt/dsh/helper.js".to_string(),
            helper_hash: "a".repeat(64),
            workspace: "/home/deploy/work".to_string(),
        }
    }

    fn patch(text: &str) -> PatchFile {
        PatchFile::from_text(text).expect("fixture 应是合法 patch")
    }

    #[test]
    fn four_rows_are_written_with_ssh_first() {
        let mut p = patch("");
        assert!(apply_ssh_rows(&mut p, &target()));
        let yaml = p.render().unwrap();
        let idx = |needle: &str| {
            yaml.find(needle)
                .unwrap_or_else(|| panic!("缺 {needle}：{yaml}"))
        };
        // 顺序即语义：其余三个都 inject `ssh`，故 dsh-ssh 必须排在前面。
        assert!(idx("dsh-dock-ssh") < idx("dsh-dock-fs-ssh"), "{yaml}");
        assert!(
            idx("dsh-dock-fs-ssh") < idx("dsh-dock-subprocess-ssh"),
            "{yaml}"
        );
        assert!(
            idx("dsh-dock-subprocess-ssh") < idx("dsh-dock-sandbox-ssh"),
            "{yaml}"
        );
    }

    #[test]
    fn every_row_carries_a_stable_id_and_package_name() {
        let mut p = patch("");
        apply_ssh_rows(&mut p, &target());
        let yaml = p.render().unwrap();
        for (row_id, package) in SSH_PACKAGES {
            assert!(yaml.contains(row_id), "缺行 id {row_id}：{yaml}");
            assert!(yaml.contains(package), "缺包名 {package}：{yaml}");
        }
    }

    /// §2.4：缺 `id` 的行会被 loader 自动生成 id，此后**永远无法被 patch 命中**。
    #[test]
    fn only_the_ssh_row_carries_config() {
        let mut p = patch("");
        apply_ssh_rows(&mut p, &target());
        let doc: Value = serde_yaml::from_str(&p.render().unwrap()).unwrap();
        let rows: Vec<&serde_yaml::Mapping> = doc
            .as_sequence()
            .unwrap()
            .iter()
            .filter_map(|e| {
                e.as_mapping()?
                    .get(Value::String("insert".into()))?
                    .as_sequence()
            })
            .flat_map(|s| s.iter().filter_map(|r| r.as_mapping()))
            .collect();
        for row in &rows {
            let id = row
                .get(Value::String("id".into()))
                .and_then(|v| v.as_str())
                .unwrap();
            let has_cfg = row.get(Value::String("config".into())).is_some();
            assert_eq!(
                has_cfg,
                id == SSH_CONFIG_ROW,
                "只有 {SSH_CONFIG_ROW} 该带 config，而 {id} 的 config={has_cfg}"
            );
        }
    }

    #[test]
    fn config_carries_exactly_the_five_required_keys() {
        let mut p = patch("");
        apply_ssh_rows(&mut p, &target());
        let doc: Value = serde_yaml::from_str(&p.render().unwrap()).unwrap();
        let cfg = doc
            .as_sequence()
            .unwrap()
            .iter()
            .filter_map(|e| {
                e.as_mapping()?
                    .get(Value::String("insert".into()))?
                    .as_sequence()
            })
            .flat_map(|s| s.iter().filter_map(|r| r.as_mapping()))
            .find(|r| {
                r.get(Value::String("id".into())).and_then(|v| v.as_str()) == Some(SSH_CONFIG_ROW)
            })
            .and_then(|r| r.get(Value::String("config".into())))
            .and_then(|c| c.as_mapping())
            .expect("dsh-ssh 行必须带 config");
        let mut keys: Vec<&str> = cfg.keys().filter_map(|k| k.as_str()).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["helper", "helperHash", "host", "node", "workspace"]);
    }

    #[test]
    fn reapply_is_idempotent_and_reports_no_change() {
        let mut p = patch("");
        assert!(apply_ssh_rows(&mut p, &target()));
        assert!(!apply_ssh_rows(&mut p, &target()), "第二次应零改动");
        let yaml = p.render().unwrap();
        assert_eq!(
            yaml.matches("dsh-dock-ssh").count(),
            1,
            "不得产生第二份：{yaml}"
        );
    }

    /// 中途失败后重跑要能**补齐缺的那几行**，而不是整批短路。
    #[test]
    fn partial_state_is_completed_not_skipped() {
        let mut p = patch(
            "- insert:\n    - id: dsh-dock-ssh\n      name: '@deepseek-ai/dsh-ssh'\n      config:\n        host: old\n        node: /n\n        helper: /h\n        helperHash: x\n        workspace: /w\n",
        );
        assert!(apply_ssh_rows(&mut p, &target()), "缺三行应被补上");
        let yaml = p.render().unwrap();
        for (row_id, _) in SSH_PACKAGES {
            assert!(yaml.contains(row_id), "缺 {row_id}：{yaml}");
        }
        // 已存在的行**不覆盖**（幂等口径：不擅自改用户/既有取值）。
        assert!(yaml.contains("host: old"), "{yaml}");
    }

    /// 既有 `insert` 数组要被**并入**，而不是新增第二个 `- insert:` 条目。
    #[test]
    fn rows_merge_into_the_existing_insert_entry() {
        let mut p = patch("- insert:\n    - id: other\n      name: something\n");
        apply_ssh_rows(&mut p, &target());
        let yaml = p.render().unwrap();
        assert_eq!(yaml.matches("- insert:").count(), 1, "{yaml}");
        assert!(yaml.contains("id: other"), "既有的行必须保真：{yaml}");
    }

    /// 非 `insert` 条目必须原文保真（行间注释不得被吞）。
    #[test]
    fn unrelated_entries_keep_their_original_text() {
        let src = "# 用户写的注释\n- id: keep-me\n  config:\n    a: 1\n";
        let mut p = patch(src);
        apply_ssh_rows(&mut p, &target());
        let yaml = p.render().unwrap();
        assert!(yaml.contains("# 用户写的注释"), "{yaml}");
        assert!(yaml.contains("id: keep-me"), "{yaml}");
        assert!(yaml.contains("a: 1"), "{yaml}");
    }

    #[test]
    fn yaml_special_characters_in_paths_are_quoted_by_the_serializer() {
        // 路径里带 `:` 或 `#` 时，手写字符串会产出坏 YAML——交给序列化器保证。
        let t = SshTarget {
            workspace: "/home/a: b#c".to_string(),
            ..target()
        };
        let mut p = patch("");
        apply_ssh_rows(&mut p, &t);
        let doc: Value = serde_yaml::from_str(&p.render().unwrap()).unwrap();
        assert!(doc.is_sequence(), "产出的必须是合法 YAML");
    }
}
