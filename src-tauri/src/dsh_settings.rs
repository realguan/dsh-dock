//! dsh_settings.rs —— DSH 引擎全局配置管理（`$DSH_HOME/settings.yaml`，4.5）。
//!
//! 职责：读取与安全写入 DSH 引擎全局配置文件。

use std::path::Path;

/// 读取 `$DSH_HOME/settings.yaml` 原文（不存在时返回空字符串）
pub fn read_dsh_settings(home: &Path) -> Result<String, String> {
    let file = home.join("settings.yaml");
    if !file.is_file() {
        return Ok(String::new());
    }
    std::fs::read_to_string(&file).map_err(|e| format!("读取 settings.yaml 失败：{e}"))
}

/// 原子写入 `$DSH_HOME/settings.yaml`
pub fn write_dsh_settings(home: &Path, content: &str) -> Result<(), String> {
    if !home.is_dir() {
        std::fs::create_dir_all(home).map_err(|e| format!("创建 DSH_HOME 目录失败：{e}"))?;
    }

    let target = home.join("settings.yaml");
    let tmp = home.join(format!("settings.tmp.{}", std::process::id()));

    std::fs::write(&tmp, content).map_err(|e| format!("写入临时 settings 文件失败：{e}"))?;

    if let Err(e) = std::fs::rename(&tmp, &target) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("覆盖 settings.yaml 失败：{e}"));
    }

    Ok(())
}

/// 原文覆写入口（4.5）：**先备份**再原子写回（2026-09-08，U9——确认框已向用户
/// 承诺「原文件会先备份」，故备份失败即中止，不降级）。
pub fn overwrite_dsh_settings(home: &Path, content: &str) -> Result<(), String> {
    crate::fs_backup::backup_before_overwrite(&home.join("settings.yaml"))?;
    write_dsh_settings(home, content)
}

/// 读取客体 `$DSH_HOME/settings.yaml` 原文（不存在返回空串）
pub fn read_dsh_settings_in_guest(distro: &str) -> Result<String, String> {
    let files = crate::guest::read_files(distro, &["settings.yaml".to_string()])?;
    Ok(files
        .into_iter()
        .find_map(|(p, c)| {
            if p.ends_with("settings.yaml") {
                c
            } else {
                None
            }
        })
        .unwrap_or_default())
}

/// 覆写客体 `$DSH_HOME/settings.yaml`（先留备份再原子写回）
pub fn overwrite_dsh_settings_in_guest(distro: &str, content: &str) -> Result<(), String> {
    crate::guest::backup_file(distro, "settings.yaml")?;
    crate::guest::write_home_files(
        distro,
        &[("settings.yaml".to_string(), content.to_string())],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_write_dsh_settings_flow() {
        let tmp = std::env::temp_dir().join(format!("dsh-settings-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);

        // 初始为空
        assert_eq!(read_dsh_settings(&tmp).unwrap(), "");

        // 写入并读回
        write_dsh_settings(&tmp, "model: deepseek-chat\n").unwrap();
        assert_eq!(read_dsh_settings(&tmp).unwrap(), "model: deepseek-chat\n");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn overwrite_backs_up_previous_settings() {
        // 2026-09-08（U9）：覆写前必须留一份原文件——写坏 settings.yaml 会直接
        // 影响 dsh 启动，用户凭确认框的「先备份」承诺需要落地。
        let tmp = std::env::temp_dir().join(format!("dsh-settings-bak-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        write_dsh_settings(&tmp, "model: old\n").unwrap();
        overwrite_dsh_settings(&tmp, "model: new\n").unwrap();

        assert_eq!(read_dsh_settings(&tmp).unwrap(), "model: new\n");
        let backups: Vec<String> = std::fs::read_dir(&tmp)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.starts_with("settings.yaml.bak-"))
            .collect();
        assert_eq!(backups.len(), 1, "应留一份备份：{backups:?}");
        assert_eq!(
            std::fs::read_to_string(tmp.join(&backups[0])).unwrap(),
            "model: old\n",
            "备份内容应为覆写前原文"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn overwrite_without_existing_file_skips_backup() {
        let tmp = std::env::temp_dir().join(format!("dsh-settings-nobak-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        overwrite_dsh_settings(&tmp, "model: first\n").unwrap();

        let backups = std::fs::read_dir(&tmp)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains(".bak-"))
            .count();
        assert_eq!(backups, 0, "首次写入无原文件可备份");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
