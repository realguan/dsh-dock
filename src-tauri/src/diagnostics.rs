//! diagnostics.rs —— 环境诊断大盘与应用日志查看器（4.11）。
//!
//! 职责：
//! 1. 采集 Node.js、pnpm、DSH 核心版本、路径及来源元数据；
//! 2. 统计 `$DSH_HOME` 及各子目录（profiles / sessions / cache 等）磁盘占用；
//! 3. 安全读取应用日志文件（shell.log、dsh 运行日志、会话自愈日志）的尾部与分页。

use serde::{Deserialize, Serialize};
use std::path::Path;

/// 运行环境诊断报告
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemDiagnosticsReport {
    pub node: NodeDiagnosticInfo,
    pub pnpm: PnpmDiagnosticInfo,
    pub dsh: DshDiagnosticInfo,
    pub storage: StorageDiagnosticInfo,
    pub platform: PlatformDiagnosticInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeDiagnosticInfo {
    pub path: String,
    pub version: String,
    pub source: String,
    pub is_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PnpmDiagnosticInfo {
    pub path: String,
    pub version: Option<String>,
    pub is_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DshDiagnosticInfo {
    pub path: String,
    pub version: Option<String>,
    pub source: String,
    pub is_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageDiagnosticInfo {
    pub dsh_home: String,
    pub total_bytes: u64,
    pub profiles_bytes: u64,
    pub sessions_bytes: u64,
    pub profiles_count: usize,
    pub sessions_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDiagnosticInfo {
    pub os: String,
    pub arch: String,
}

/// 日志查询返回
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogQueryResult {
    pub source: String,
    pub path: String,
    pub lines: Vec<String>,
    pub total_lines: usize,
    pub truncated: bool,
}

/// 计算指定目录的总大小（字节数）与一级子项数量
pub fn dir_size_and_count(dir: &Path) -> (u64, usize) {
    if !dir.is_dir() {
        return (0, 0);
    }
    let mut total_size = 0u64;
    let mut count = 0usize;

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            count += 1;
            let path = entry.path();
            if path.is_file() {
                total_size += entry.metadata().map(|m| m.len()).unwrap_or(0);
            } else if path.is_dir() {
                let (sub_size, _) = dir_size_and_count(&path);
                total_size += sub_size;
            }
        }
    }
    (total_size, count)
}

/// 收集全量诊断信息
pub fn collect_diagnostics(home: &Path) -> SystemDiagnosticsReport {
    let path_env = std::env::var("PATH").unwrap_or_default();

    // Node 探测
    let node_detected = crate::resolve::detect_system_node(&path_env);
    let node = if let Some(n) = node_detected {
        NodeDiagnosticInfo {
            path: n.bin.to_string_lossy().to_string(),
            version: n.version.as_str().to_string(),
            source: "system".to_string(),
            is_ready: true,
        }
    } else {
        NodeDiagnosticInfo {
            path: "未检出".to_string(),
            version: "未知".to_string(),
            source: "none".to_string(),
            is_ready: false,
        }
    };

    // pnpm 探测
    let pnpm_detected = crate::updates::find_pnpm(&path_env);
    let pnpm = if let Some(p) = pnpm_detected {
        PnpmDiagnosticInfo {
            path: p.to_string_lossy().to_string(),
            version: None,
            is_ready: true,
        }
    } else {
        PnpmDiagnosticInfo {
            path: "未检出".to_string(),
            version: None,
            is_ready: false,
        }
    };

    // DSH 探测
    let dsh_detected = crate::resolve::detect_system_dsh(&path_env);
    let dsh = if let Some(d) = dsh_detected {
        DshDiagnosticInfo {
            path: d.bin_js.to_string_lossy().to_string(),
            version: Some(d.version),
            source: "system".to_string(),
            is_ready: true,
        }
    } else {
        DshDiagnosticInfo {
            path: "未检出".to_string(),
            version: None,
            source: "none".to_string(),
            is_ready: false,
        }
    };

    // 存储占用统计
    let profiles_dir = home.join("profiles");
    let sessions_dir = home.join("sessions");

    let (profiles_bytes, profiles_count) = dir_size_and_count(&profiles_dir);
    let (sessions_bytes, sessions_count) = dir_size_and_count(&sessions_dir);
    let (total_bytes, _) = dir_size_and_count(home);

    let storage = StorageDiagnosticInfo {
        dsh_home: home.to_string_lossy().to_string(),
        total_bytes,
        profiles_bytes,
        sessions_bytes,
        profiles_count,
        sessions_count,
    };

    let platform = PlatformDiagnosticInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
    };

    SystemDiagnosticsReport {
        node,
        pnpm,
        dsh,
        storage,
        platform,
    }
}

/// 读取指定日志源内容（支持 tail 截取，防止超大日志卡死前端）
pub fn read_app_logs(
    source: &str,
    app_data: &Path,
    home: &Path,
    tail_lines: usize,
) -> Result<LogQueryResult, String> {
    let (file_path, display_name) = match source {
        "shell" => (app_data.join("shell.log"), "DSH Dock 壳运行日志".to_string()),
        "dsh" => (app_data.join("dsh.log"), "DSH 服务运行时日志".to_string()),
        "session_repair" => (
            std::env::temp_dir().join("dsh-repair.log"),
            "会话自愈修复日志".to_string(),
        ),
        _ => {
            // 尝试读取指定 profile 的日志或回退
            let candidate = home.join("profiles").join(source).join("profile.log");
            if candidate.is_file() {
                (candidate, format!("Profile [{source}] 运行日志"))
            } else {
                (app_data.join("shell.log"), "DSH Dock 壳运行日志".to_string())
            }
        }
    };

    if !file_path.is_file() {
        return Ok(LogQueryResult {
            source: source.to_string(),
            path: file_path.to_string_lossy().to_string(),
            lines: vec![format!("（日志文件暂未生成或不存在: {}）", file_path.display())],
            total_lines: 0,
            truncated: false,
        });
    }

    let raw = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("读取日志文件失败：{e}"))?;

    let all_lines: Vec<&str> = raw.lines().collect();
    let total_lines = all_lines.len();

    let (sliced, truncated) = if all_lines.len() > tail_lines && tail_lines > 0 {
        let start = all_lines.len() - tail_lines;
        (&all_lines[start..], true)
    } else {
        (&all_lines[..], false)
    };

    Ok(LogQueryResult {
        source: display_name,
        path: file_path.to_string_lossy().to_string(),
        lines: sliced.iter().map(|s| s.to_string()).collect(),
        total_lines,
        truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir_size_and_count_computes_correctly() {
        let tmp = std::env::temp_dir().join(format!("dsh-diag-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("sub")).unwrap();

        std::fs::write(tmp.join("a.txt"), "hello").unwrap();
        std::fs::write(tmp.join("sub").join("b.txt"), "world123").unwrap();

        let (size, count) = dir_size_and_count(&tmp);
        assert_eq!(size, 13); // 5 + 8
        assert_eq!(count, 2); // a.txt + sub

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn read_app_logs_returns_placeholder_when_missing() {
        let tmp = std::env::temp_dir().join(format!("dsh-diag-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let res = read_app_logs("shell", &tmp, &tmp, 100).unwrap();
        assert_eq!(res.total_lines, 0);
        assert!(res.lines[0].contains("日志文件暂未生成"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn read_app_logs_tails_lines() {
        let tmp = std::env::temp_dir().join(format!("dsh-diag-log-tail-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let log_file = tmp.join("shell.log");
        let content = (0..20)
            .map(|i| format!("Line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&log_file, content).unwrap();

        let res = read_app_logs("shell", &tmp, &tmp, 5).unwrap();
        assert_eq!(res.total_lines, 20);
        assert_eq!(res.lines.len(), 5);
        assert_eq!(res.lines[0], "Line 15");
        assert_eq!(res.lines[4], "Line 19");
        assert!(res.truncated);

        // 当请求行数大于总行数时，不发生截断
        let res_all = read_app_logs("shell", &tmp, &tmp, 50).unwrap();
        assert_eq!(res_all.lines.len(), 20);
        assert!(!res_all.truncated);

        // 测试 dsh 源
        let dsh_log = tmp.join("dsh.log");
        std::fs::write(&dsh_log, "dsh log line 1\ndsh log line 2\n").unwrap();
        let res_dsh = read_app_logs("dsh", &tmp, &tmp, 10).unwrap();
        assert_eq!(res_dsh.total_lines, 2);
        assert_eq!(res_dsh.lines[0], "dsh log line 1");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
