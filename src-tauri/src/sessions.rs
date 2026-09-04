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
    pub decoded_project_path: String,
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

/// 将编码后的项目目录名反解为真实工作区操作系统路径（支持文件系统智能贪婪探测与带连字符目录还原）
pub fn decode_project_dir_to_path(raw: &str) -> String {
    let stripped = raw.trim_matches('-');
    if stripped.is_empty() {
        return "/".to_string();
    }

    // Windows 盘符判定，如 C:-Users-guan-project-
    let is_windows = stripped.len() >= 2 && stripped.chars().nth(1) == Some(':');
    let (sep, mut current_base, remaining_raw) = if is_windows {
        let drive = &stripped[..2];
        let rest = stripped[2..].trim_start_matches('-');
        ("\\", format!("{drive}\\"), rest)
    } else {
        ("/", String::from("/"), stripped)
    };

    let segments: Vec<&str> = remaining_raw.split('-').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return current_base;
    }

    let mut i = 0;
    while i < segments.len() {
        let mut matched_len = 1;
        let max_lookahead = (segments.len() - i).min(6);

        let mut found_dir = false;
        let base_path = Path::new(&current_base);
        if base_path.is_dir() {
            for len in (1..=max_lookahead).rev() {
                let joined_name = segments[i..i + len].join("-");
                let candidate_path = base_path.join(&joined_name);
                if candidate_path.exists() {
                    current_base = candidate_path.to_string_lossy().to_string();
                    matched_len = len;
                    found_dir = true;
                    break;
                }
            }
        }

        if !found_dir {
            if current_base.ends_with(sep) {
                current_base.push_str(segments[i]);
            } else {
                current_base.push_str(sep);
                current_base.push_str(segments[i]);
            }
        }

        i += matched_len;
    }

    current_base
}

/// 将项目目录名称（例如 `--Users-guan-git-dsh-dock--`）转为简洁可读的项目名（如 `dsh-dock`）
pub fn decode_project_dir_name(raw: &str) -> String {
    let stripped = raw.trim_matches('-');
    if stripped.is_empty() {
        return "root".to_string();
    }

    let path_str = decode_project_dir_to_path(raw);
    let p = Path::new(&path_str);
    if p.exists() {
        if let Some(file_name) = p.file_name().and_then(|n| n.to_str()) {
            if !file_name.is_empty() {
                return file_name.to_string();
            }
        }
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

                let decoded_project_path = decode_project_dir_to_path(&project_dir_name);

                items.push(SessionItem {
                    id: session_id,
                    project_name: project_name.clone(),
                    project_dir_raw: project_dir_name.clone(),
                    decoded_project_path,
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

/// 执行单会话或全量会话修复（通过内置修复脚本）。node 来源引擎档优先
///（ADR-0010：不依赖用户环境），引擎未就绪回退系统探测，双缺给可行动错误
///（不再裸调 PATH 上的 `node`——存在性未知必败且不可诊断）。
pub fn run_repair(
    target: Option<&str>,
    home: &Path,
    data_dir: &Path,
) -> Result<RepairOutcome, String> {
    let script_content = include_str!("../../scripts/repair-session.mjs");
    let temp_dir = std::env::temp_dir();
    let script_path = temp_dir.join("dsh-dock-repair-session.mjs");

    fs::write(&script_path, script_content).map_err(|e| format!("写入临时修复脚本失败：{e}"))?;

    let node_bin = crate::engines::engine_node_bin(data_dir)
        .ok_or_else(|| "引擎未就绪（node 缺失）——请先启动应用完成引擎引导后重试".to_string())?;
    tracing::info!(
        target = ?target,
        node = %node_bin.display(),
        "会话修复开始"
    );

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

/// 删除指定会话文件或目录（4.6 会话维护）
pub fn remove_session(home: &Path, session_path_str: &str) -> Result<(), String> {
    let session_path = PathBuf::from(session_path_str);
    let sessions_root = home.join("sessions");

    // 安全检查：目标必须落在 sessions_root 内部
    if !session_path.starts_with(&sessions_root) {
        return Err("非法会话路径：超出 sessions 根目录范围".to_string());
    }

    if !session_path.exists() {
        return Ok(());
    }

    if session_path.is_file() {
        // 如果是单文件（如 session.jsonl / session.jsonl.zst），且父目录即会话目录，删除父目录或文件
        if let Some(parent) = session_path.parent() {
            if parent != sessions_root && parent.parent() != Some(&sessions_root) {
                // 是 session-xxx 文件夹
                let _ = fs::remove_dir_all(parent);
                return Ok(());
            }
        }
        fs::remove_file(&session_path).map_err(|e| format!("删除会话文件失败：{e}"))?;
    } else if session_path.is_dir() {
        fs::remove_dir_all(&session_path).map_err(|e| format!("删除会话目录失败：{e}"))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_project_dir_extracts_basename() {
        assert_eq!(decode_project_dir_name("----"), "root");
        assert_eq!(decode_project_dir_name("--my-project--"), "my-project");
        assert_eq!(
            decode_project_dir_to_path("-C:-Users-guan-project-"),
            "C:\\Users\\guan\\project"
        );

        // 如果在真实文件系统上有匹配的目录，能够贪婪还原复合名称
        let temp = std::env::temp_dir().join(format!("dsh-dock-greedy-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let nested = temp.join("sub-project-a").join("deep-hub");
        fs::create_dir_all(&nested).unwrap();

        let raw_str = nested.to_string_lossy().replace(['\\', '/'], "-");
        let encoded = format!("--{}--", raw_str.trim_matches('-'));
        let decoded = decode_project_dir_to_path(&encoded);
        assert_eq!(decoded, nested.to_string_lossy().to_string());
        assert_eq!(decode_project_dir_name(&encoded), "deep-hub");

        let _ = fs::remove_dir_all(&temp);
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

        // 测试删除
        remove_session(&temp, &list[0].file_path).unwrap();
        let after = scan_sessions(&temp).unwrap();
        assert!(after.is_empty());

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn decode_project_dir_to_path_edge_cases() {
        assert_eq!(decode_project_dir_to_path(""), "/");
        assert_eq!(decode_project_dir_to_path("---"), "/");
        assert_eq!(
            decode_project_dir_to_path("-D:-workspace-app-"),
            "D:\\workspace\\app"
        );
        assert_eq!(
            decode_project_dir_to_path("--var-log-dsh--"),
            "/var/log/dsh"
        );
    }

    #[test]
    fn repair_session_cleans_corrupt_jsonl_and_creates_backup() {
        // CI 单元测试阶段若尚未安装 Node.js 则优雅跳过外部进程执行
        let node_available = crate::child_cmd(Path::new("node"))
            .arg("-v")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !node_available {
            return;
        }

        let temp = std::env::temp_dir().join(format!("dsh-sess-repair-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let sess_dir = temp.join("sessions").join("--demo--").join("sess-fail");
        fs::create_dir_all(&sess_dir).unwrap();
        let target_file = sess_dir.join("session.jsonl");

        // 写入含有 header 和乱序/倒退事件的数据
        let corrupt_data = r#"{"id":"sess-fail","type":"session_header"}
{"seq":5,"turn":2,"msg":"later"}
{"seq":1,"turn":1,"msg":"earlier"}
"#;
        fs::write(&target_file, corrupt_data).unwrap();

        // 修复链 node 来源 = 引擎档唯一：预置假体 shim（转发 PATH 上的真 node）
        let engine_bin = temp.join("engines/bin");
        std::fs::create_dir_all(&engine_bin).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let shim = engine_bin.join("node");
            std::fs::write(&shim, "#!/bin/sh\nexec node \"$@\"\n").unwrap();
            std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        #[cfg(not(unix))]
        std::fs::write(engine_bin.join("node.exe"), b"").unwrap();

        let outcome = run_repair(Some(target_file.to_str().unwrap()), &temp, &temp).unwrap();
        assert!(outcome.success);

        // 验证备份文件已创建
        let backup_file = sess_dir.join("session.jsonl.bak");
        assert!(backup_file.is_file());

        // 验证修复后的文件内容按 turn/seq 重排
        let repaired_content = fs::read_to_string(&target_file).unwrap();
        let valid_lines: Vec<&str> = repaired_content.lines().collect();
        assert_eq!(valid_lines.len(), 3);
        assert!(valid_lines[0].contains("\"id\":\"sess-fail\""));
        assert!(valid_lines[1].contains("\"turn\":1"));
        assert!(valid_lines[2].contains("\"turn\":2"));

        let _ = fs::remove_dir_all(&temp);
    }
}
