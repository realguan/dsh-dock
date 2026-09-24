//! diagnostics.rs —— 环境诊断大盘与应用日志查看器（4.11）。
//!
//! 职责：
//! 1. 采集 Node.js、pnpm、DSH 核心版本、路径及来源元数据；
//! 2. 统计 `$DSH_HOME` 及各子目录（profiles / sessions / cache 等）磁盘占用；
//! 3. 安全读取应用日志文件（shell.log、dsh 运行日志）的尾部与分页。

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

/// 收集全量诊断信息（ADR-0010 引擎四件套：pnpm/node/dsh 版本与落点 + 存储）。
/// 数据源 = 壳引擎目录（engines/bin），不再探测用户环境（探测层已退役）。
pub fn collect_diagnostics(home: &Path, data_dir: &Path) -> SystemDiagnosticsReport {
    let engine_bin = crate::engines::engine_bin_dir(data_dir);
    let status = crate::engines::probe_engine(data_dir, &crate::resolve::effective_path());

    let node = match (&status.node, crate::engines::engine_node_bin(data_dir)) {
        (Some(version), Some(bin)) => NodeDiagnosticInfo {
            path: bin.to_string_lossy().to_string(),
            version: version.clone(),
            source: "engine".to_string(),
            is_ready: true,
        },
        _ => NodeDiagnosticInfo {
            path: engine_bin.to_string_lossy().to_string(),
            version: "未检出".to_string(),
            source: "none".to_string(),
            is_ready: false,
        },
    };

    let pnpm = match (
        &status.pnpm,
        crate::engines::engine_pnpm_bin(data_dir).exists(),
    ) {
        (Some(version), true) => PnpmDiagnosticInfo {
            path: crate::engines::engine_pnpm_bin(data_dir)
                .to_string_lossy()
                .to_string(),
            version: Some(version.clone()),
            is_ready: true,
        },
        _ => PnpmDiagnosticInfo {
            path: crate::engines::engine_pnpm_bin(data_dir)
                .to_string_lossy()
                .to_string(),
            version: None,
            is_ready: false,
        },
    };

    let dsh = match (&status.dsh, crate::engines::engine_dsh_bin(data_dir)) {
        (Some(version), Some(bin)) => DshDiagnosticInfo {
            path: bin.to_string_lossy().to_string(),
            version: Some(version.clone()),
            source: "engine".to_string(),
            is_ready: true,
        },
        _ => DshDiagnosticInfo {
            path: engine_bin.to_string_lossy().to_string(),
            version: None,
            source: "none".to_string(),
            is_ready: false,
        },
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

/// 客体环境系统诊断数据采集（ADR-0016 P3）。
pub fn collect_diagnostics_in_guest(distro: &str) -> Result<SystemDiagnosticsReport, String> {
    crate::guest::collect_diagnostics_in_guest(distro)
}

/// 本次日志查询所属的「运行世界」（ADR-0016 §2.6：管理面必须与运行中的会话同源）。
///
/// 2026-09-21 平台审计 A5 修复：日志查询此前是 `commands/console.rs` 里**唯一**没有
/// `mgmt::current_world` 分发的命令 —— WSL 客体档下会静默读**宿主**世界的日志
/// （"给错结果"比报错更坏）。同时 `dsh` 源指向一个全仓无人写入的 `app_data/dsh.log`；
/// 真实来源是宿主档 `dsh-shell.log`（`shell.rs:72`）与客体档 `dsh-wsl.log`
/// （`executor.rs:807`，壳捕获 `wsl.exe` 输出）。两处一并修。
#[derive(Debug, Clone, Copy)]
pub enum LogWorld<'a> {
    /// 宿主世界（macOS / Linux / Windows 本地档）
    Local,
    /// WSL 客体档：profile 级日志在**客体** dsh home（`${DSH_HOME:-$HOME/.dsh}`）内
    Guest { distro: &'a str },
}

/// 日志源解析结果（**纯数据、零 IO** ⇒ 三平台可单测）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogTarget {
    /// 宿主侧文件（壳自产日志恒在宿主 `app_data`）
    HostFile { path: PathBuf, label: String },
    /// 客体侧文件（路径相对客体 dsh home；`shown_path` 供 UI 如实行显示）
    GuestFile {
        distro: String,
        rel: String,
        label: String,
        shown_path: String,
    },
}

fn host_file(path: PathBuf, label: &str) -> LogTarget {
    LogTarget::HostFile {
        path,
        label: label.to_string(),
    }
}

/// 日志源 → 目标（`world` × `source` 的**完整真值表**，纯函数）。
///
/// 与旧版的行为差异（有意）：未知源不再"文件不存在就悄悄回退到壳日志"，而是如实
/// 报"该 profile 日志不存在" —— 静默替换成别的来源属于红线 3 禁的静默降级。
pub fn resolve_log_target(
    source: &str,
    world: LogWorld<'_>,
    app_data: &Path,
    home: &Path,
) -> LogTarget {
    match (world, source) {
        // 壳自身的应用日志：壳是宿主进程，客体档下同样读宿主这一份。
        (_, "shell") => host_file(app_data.join("shell.log"), "DSH Dock 壳运行日志"),
        // dsh 进程输出：宿主档与客体档落在不同文件（两者都是宿主侧文件）。
        (LogWorld::Local, "dsh") => host_file(app_data.join("dsh-shell.log"), "DSH 服务运行时日志"),
        (LogWorld::Guest { .. }, "dsh") => host_file(
            app_data.join("dsh-wsl.log"),
            "DSH 服务运行时日志（WSL 客体）",
        ),
        // profile 级日志：宿主档读宿主 home；客体档**必须去客体 home 读**
        // （读宿主 = 给错结果，正是本次修的那条）。
        (LogWorld::Local, name) => host_file(
            home.join("profiles").join(name).join("profile.log"),
            &format!("Profile [{name}] 运行日志"),
        ),
        (LogWorld::Guest { distro }, name) => LogTarget::GuestFile {
            distro: distro.to_string(),
            rel: format!("profiles/{name}/profile.log"),
            label: format!("Profile [{name}] 运行日志"),
            shown_path: format!("{distro}:~/.dsh/profiles/{name}/profile.log"),
        },
    }
}

/// 读取指定日志源内容（支持 tail 截取，防止超大日志卡死前端）。
pub fn read_app_logs(
    source: &str,
    world: LogWorld<'_>,
    app_data: &Path,
    home: &Path,
    tail_lines: usize,
) -> Result<LogQueryResult, String> {
    let (display_name, shown_path, raw) = match resolve_log_target(source, world, app_data, home) {
        LogTarget::HostFile { path, label } => {
            let shown = path.to_string_lossy().to_string();
            if !path.is_file() {
                (label, shown, None)
            } else {
                // 宿主侧日志可能是 UTF-16LE（`wsl.exe` 重定向输出；2026-08-26 实机 bug
                // 同源）⇒ 一律经 `decode_output_bytes` 探测解码，不用严格 UTF-8。
                match std::fs::read(&path) {
                    Ok(bytes) => (
                        label,
                        shown,
                        Some(crate::resolve::decode_output_bytes(&bytes)),
                    ),
                    Err(e) => return Err(format!("读取日志文件失败：{e}")),
                }
            }
        }
        LogTarget::GuestFile {
            distro,
            rel,
            label,
            shown_path,
        } => {
            let files = crate::guest::read_files(&distro, std::slice::from_ref(&rel))?;
            let content = files.into_iter().next().and_then(|(_, content)| content);
            (label, shown_path, content)
        }
    };

    let Some(raw) = raw else {
        return Ok(LogQueryResult {
            source: source.to_string(),
            path: shown_path.clone(),
            lines: vec![format!("（日志文件暂未生成或不存在: {shown_path}）")],
            total_lines: 0,
            truncated: false,
        });
    };

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
        path: shown_path,
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

    // ---------- 世界分发真值表（2026-09-21，平台审计 A5） ----------

    /// `dsh` 源在两个世界必须指向**两个不同的真实文件**；`shell` 源恒在宿主。
    #[test]
    fn log_target_truth_table_matches_where_logs_are_actually_written() {
        let app = Path::new("/appdata");
        let home = Path::new("/home/u/.dsh");

        let local_dsh = resolve_log_target("dsh", LogWorld::Local, app, home);
        assert_eq!(
            local_dsh,
            LogTarget::HostFile {
                path: app.join("dsh-shell.log"),
                label: "DSH 服务运行时日志".to_string()
            },
            "宿主档 dsh 输出落在 dsh-shell.log（shell.rs:72）"
        );

        let guest_dsh = resolve_log_target(
            "dsh",
            LogWorld::Guest {
                distro: "Ubuntu-24.04",
            },
            app,
            home,
        );
        assert_eq!(
            guest_dsh,
            LogTarget::HostFile {
                path: app.join("dsh-wsl.log"),
                label: "DSH 服务运行时日志（WSL 客体）".to_string()
            },
            "客体档 dsh 输出是壳捕获的 dsh-wsl.log（executor.rs:807）"
        );

        for world in [LogWorld::Local, LogWorld::Guest { distro: "d" }] {
            assert_eq!(
                resolve_log_target("shell", world, app, home),
                LogTarget::HostFile {
                    path: app.join("shell.log"),
                    label: "DSH Dock 壳运行日志".to_string()
                },
                "壳自身日志恒在宿主 app_data（壳是宿主进程）"
            );
        }
    }

    /// profile 级日志：客体档**必须**改读客体 home —— 读宿主就是"给错结果"。
    #[test]
    fn profile_log_reads_the_guest_home_in_guest_world() {
        let app = Path::new("/appdata");
        let home = Path::new("/home/u/.dsh");

        assert_eq!(
            resolve_log_target("web", LogWorld::Local, app, home),
            LogTarget::HostFile {
                path: home.join("profiles/web/profile.log"),
                label: "Profile [web] 运行日志".to_string()
            },
        );

        let guest = resolve_log_target(
            "web",
            LogWorld::Guest {
                distro: "Ubuntu-24.04",
            },
            app,
            home,
        );
        match guest {
            LogTarget::GuestFile {
                distro,
                rel,
                shown_path,
                ..
            } => {
                assert_eq!(distro, "Ubuntu-24.04");
                assert_eq!(
                    rel, "profiles/web/profile.log",
                    "rel 相对客体 dsh home（guest.rs HOME_EXPR）"
                );
                assert!(
                    shown_path.contains("Ubuntu-24.04"),
                    "UI 要能看出读的是哪个发行版：{shown_path}"
                );
            }
            other => panic!("客体档 profile 日志必须走客体读，得到 {other:?}"),
        }
    }

    /// 反例守卫：不存在的 profile 源**不得**静默回退成壳日志（红线 3 禁静默降级）。
    #[test]
    fn unknown_profile_source_never_falls_back_to_another_log() {
        let app = Path::new("/appdata");
        let home = Path::new("/home/u/.dsh");
        let target = resolve_log_target("nope", LogWorld::Local, app, home);
        assert_eq!(
            target,
            LogTarget::HostFile {
                path: home.join("profiles/nope/profile.log"),
                label: "Profile [nope] 运行日志".to_string()
            },
            "要如实报该 profile 没有日志，而不是拿壳日志顶替"
        );
    }

    /// 命令层契约：`get_app_logs` 必须做世界分发（它曾是 console.rs 里唯一漏掉的那条）。
    #[test]
    fn get_app_logs_dispatches_the_running_world() {
        let src = include_str!("commands/console.rs").replace("\r\n", "\n");
        let cmd = src
            .find("pub async fn get_app_logs")
            .expect("找不到 get_app_logs");
        let dispatch = src[cmd..]
            .find("current_world")
            .expect("get_app_logs 必须经 mgmt::current_world 分发世界（审计 A5）");
        assert!(
            dispatch < 1200,
            "世界分发应贴在命令开头（偏移 {dispatch} 字符）"
        );
    }

    #[test]
    fn read_app_logs_returns_placeholder_when_missing() {
        let tmp = std::env::temp_dir().join(format!("dsh-diag-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let res = read_app_logs("shell", LogWorld::Local, &tmp, &tmp, 100).unwrap();
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

        let res = read_app_logs("shell", LogWorld::Local, &tmp, &tmp, 5).unwrap();
        assert_eq!(res.total_lines, 20);
        assert_eq!(res.lines.len(), 5);
        assert_eq!(res.lines[0], "Line 15");
        assert_eq!(res.lines[4], "Line 19");
        assert!(res.truncated);

        // 当请求行数大于总行数时，不发生截断
        let res_all = read_app_logs("shell", LogWorld::Local, &tmp, &tmp, 50).unwrap();
        assert_eq!(res_all.lines.len(), 20);
        assert!(!res_all.truncated);

        // 测试 dsh 源：宿主档的真实落点是 dsh-shell.log（shell.rs:72），
        // 不是曾经的 app_data/dsh.log（全仓无人写入，审计 A5）。
        let dsh_log = tmp.join("dsh-shell.log");
        std::fs::write(&dsh_log, "dsh log line 1\ndsh log line 2\n").unwrap();
        let res_dsh = read_app_logs("dsh", LogWorld::Local, &tmp, &tmp, 10).unwrap();
        assert_eq!(res_dsh.total_lines, 2);
        assert_eq!(res_dsh.lines[0], "dsh log line 1");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
