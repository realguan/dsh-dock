//! sessions.rs —— 会话列表扫描与日志自愈修复（2026-08-31）。
//!
//! 职责：
//! 1. 扫描 `$DSH_HOME/sessions/` 下各项目目录与会话文件；
//! 2. 统计会话元数据（ID、所属项目、更新时间、大小、备份状态）；
//! 3. 执行会话修复（调用 Node 运行自愈脚本进行 Turn 归流与 Contiguous Seq 重排）；
//! 4. 支持单会话修复与全量自愈。

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// 会话状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Healthy,
    NeedsRepair,
    Unknown,
}

/// 会话简要信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionItem {
    pub id: String,
    pub project_name: String,
    pub project_dir_raw: String,
    pub file_path: String,
    pub updated_at: u64,
    pub size_bytes: u64,
    pub is_compressed: bool,
    pub has_backup: bool,
    pub status: SessionStatus,
}

/// 修复操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairOutcome {
    pub session_id: String,
    pub success: bool,
    pub message: String,
}

/// 全量修复统计
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRepairSummary {
    pub total: usize,
    pub repaired: usize,
    pub skipped: usize,
    pub failures: Vec<String>,
}

/// 将项目目录名称（例如 `--Users-guan-git-dsh-dock--`）转为可读的项目标识
pub fn decode_project_dir_name(raw: &str) -> String {
    let stripped = raw.trim_matches('-');
    if stripped.is_empty() {
        return "root".to_string();
    }
    stripped.to_string()
}

/// 扫描指定 DSH HOME 下的所有会话
pub fn scan_sessions(home: &Path) -> Result<Vec<SessionItem>, String> {
    let sessions_dir = home.join("sessions");
    if !sessions_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    let entries =
        fs::read_dir(&sessions_dir).map_err(|e| format!("读取 sessions 目录失败：{e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let project_dir_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let project_name = decode_project_dir_name(&project_dir_name);

        // 遍历项目目录下的各会话文件夹
        if let Ok(sess_entries) = fs::read_dir(&path) {
            for s_entry in sess_entries.flatten() {
                let s_path = s_entry.path();
                if !s_path.is_dir() {
                    continue;
                }

                let session_id = s_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                let zstd_file = s_path.join("session.jsonl.zstd");
                let jsonl_file = s_path.join("session.jsonl");

                let (target_file, is_compressed) = if zstd_file.is_file() {
                    (zstd_file, true)
                } else if jsonl_file.is_file() {
                    (jsonl_file, false)
                } else {
                    continue;
                };

                let meta = fs::metadata(&target_file).ok();
                let size_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                let updated_at = meta
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);

                let bak_file = if is_compressed {
                    s_path.join("session.jsonl.zstd.bak")
                } else {
                    s_path.join("session.jsonl.bak")
                };
                let has_backup = bak_file.is_file();

                items.push(SessionItem {
                    id: session_id,
                    project_name: project_name.clone(),
                    project_dir_raw: project_dir_name.clone(),
                    file_path: target_file.to_string_lossy().to_string(),
                    updated_at,
                    size_bytes,
                    is_compressed,
                    has_backup,
                    status: SessionStatus::Healthy, // 默认标记
                });
            }
        }
    }

    // 按最后修改时间倒序排列（最新活跃在前）
    items.sort_by_key(|a| std::cmp::Reverse(a.updated_at));

    Ok(items)
}

/// 执行单会话或全量会话修复（通过内置修复脚本）
pub fn run_repair(target: Option<&str>, home: &Path) -> Result<RepairOutcome, String> {
    let script_content = include_str!("../../scripts/repair-session.mjs");
    let temp_dir = std::env::temp_dir();
    let script_path = temp_dir.join("dsh-dock-repair-session.mjs");

    fs::write(&script_path, script_content).map_err(|e| format!("写入临时修复脚本失败：{e}"))?;

    let path_env = std::env::var("PATH").unwrap_or_default();
    let node_bin = crate::resolve::detect_system_node(&path_env)
        .map(|n| n.bin)
        .unwrap_or_else(|| PathBuf::from("node"));

    let mut cmd = crate::child_cmd(&node_bin);
    cmd.arg(&script_path);
    cmd.env("DSH_HOME", home);

    if let Some(t) = target {
        cmd.arg(t);
    } else {
        cmd.arg("--all");
    }

    let output = cmd
        .output()
        .map_err(|e| format!("执行修复脚本失败（无法拉起 Node）：{e}"))?;

    let _ = fs::remove_file(&script_path);

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!("修复失败：\n{stderr}\n{stdout}"));
    }

    Ok(RepairOutcome {
        session_id: target.unwrap_or("all").to_string(),
        success: true,
        message: stdout,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_project_dir_extracts_basename() {
        assert_eq!(
            decode_project_dir_name("--Users-guan-git-realguan-dsh-dock--"),
            "Users-guan-git-realguan-dsh-dock"
        );
        assert_eq!(decode_project_dir_name("----"), "root");
        assert_eq!(decode_project_dir_name("--my-project--"), "my-project");
    }

    #[test]
    fn scan_sessions_empty_when_no_sessions_dir() {
        let temp = std::env::temp_dir().join(format!("dsh-test-sess-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();

        let list = scan_sessions(&temp).unwrap();
        assert!(list.is_empty());

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn scan_sessions_finds_nested_session_files() {
        let temp = std::env::temp_dir().join(format!("dsh-test-sess-scan-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let sess_dir = temp
            .join("sessions")
            .join("--my-app--")
            .join("session-12345");
        fs::create_dir_all(&sess_dir).unwrap();
        fs::write(sess_dir.join("session.jsonl"), "{\"type\":\"session\"}\n").unwrap();

        let list = scan_sessions(&temp).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "session-12345");
        assert_eq!(list[0].project_name, "my-app");
        assert!(!list[0].is_compressed);

        let _ = fs::remove_dir_all(&temp);
    }
}
