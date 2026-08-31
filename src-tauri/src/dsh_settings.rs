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
}
