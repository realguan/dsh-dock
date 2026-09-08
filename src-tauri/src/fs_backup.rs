//! fs_backup.rs —— 覆写前备份（2026-09-08，UI/UX 评审 U9）。
//!
//! 场景：壳的「原文编辑 + 保存」入口（`.credentials.yaml` / `settings.yaml`）是
//! **整文件覆写**，写坏或误删无从回退。确认对话框已向用户承诺「先备份」，因此备份
//! 失败必须中止写入（fail-closed），不得静默降级——静默降级等于毁约。
//!
//! 命名 `<文件名>.bak-<unix 秒>`：与 `settings.json` 的单份 `.bak` 不同，这里每次
//! 覆写各留一份（同一秒内重复覆写追加 `-N`，不覆盖既有备份），可当轻量撤销历史。

use std::path::Path;

/// 覆写前备份 `target`：目标不存在 → 无需备份（Ok）；备份失败 → Err（调用方须中止）。
pub fn backup_before_overwrite(target: &Path) -> Result<(), String> {
    if !target.exists() {
        return Ok(());
    }
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = target
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "backup".to_string());

    let mut backup = target.with_file_name(format!("{name}.bak-{stamp}"));
    let mut seq = 1;
    while backup.exists() {
        backup = target.with_file_name(format!("{name}.bak-{stamp}-{seq}"));
        seq += 1;
    }
    // copy 保留源文件权限位：凭据备份同样 0600，不因备份而放松。
    std::fs::copy(target, &backup)
        .map_err(|e| format!("备份 {} 失败（已中止写入）：{e}", backup.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_target_needs_no_backup() {
        let dir = std::env::temp_dir().join(format!("dsh-fsbak-none-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("settings.yaml");

        backup_before_overwrite(&target).unwrap();
        let entries: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(entries.is_empty(), "目标不存在时不应产生备份：{entries:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn repeated_backups_do_not_clobber_each_other() {
        let dir = std::env::temp_dir().join(format!("dsh-fsbak-rep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("settings.yaml");

        std::fs::write(&target, "v1\n").unwrap();
        backup_before_overwrite(&target).unwrap();
        std::fs::write(&target, "v2\n").unwrap();
        // 同一秒内的第二次备份：必须另起文件名，不能覆盖第一份
        backup_before_overwrite(&target).unwrap();

        let mut backups: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .filter(|n| n.starts_with("settings.yaml.bak-"))
            .collect();
        backups.sort();
        assert_eq!(backups.len(), 2, "两次覆写应留两份备份：{backups:?}");
        assert_eq!(
            std::fs::read_to_string(dir.join(&backups[0])).unwrap(),
            "v1\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join(&backups[1])).unwrap(),
            "v2\n"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn backup_keeps_source_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("dsh-fsbak-perm-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join(".credentials.yaml");
        std::fs::write(&target, "a: b\n").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600)).unwrap();

        backup_before_overwrite(&target).unwrap();

        let backup = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .find(|p| p.to_string_lossy().contains(".bak-"))
            .expect("应产生备份");
        let mode = std::fs::metadata(&backup).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "凭据备份不得放宽权限：{mode:o}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
