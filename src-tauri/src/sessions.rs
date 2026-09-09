//! sessions.rs —— 会话列表扫描与日志自愈修复（2026-08-31）。
//!
//! 职责：
//! 1. 扫描 `$DSH_HOME/sessions/` 下各项目目录与会话文件；
//! 2. 统计会话元数据（ID、所属项目、更新时间、大小、备份状态）；
//! 3. 执行会话修复（调用 Node 运行自愈脚本：存储层 seq 修复 + 恢复层
//!    dsh 本尊 restore 校验与 surface 最小变异，2026-09-07）；
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
    /// 会话标题（取自 `session/title` 事件的 data.title；无标题时回退为空串
    /// 或截断的会话 ID 前缀，前端负责最终展示）。
    pub title: String,
    pub project_name: String,
    pub project_dir_raw: String,
    pub decoded_project_path: String,
    pub file_path: String,
    pub updated_at: u64,
    pub size_bytes: u64,
    pub is_compressed: bool,
    pub has_backup: bool,
    pub status: SessionStatus,
    /// 健康检查附加信息（异常原因/未修复原因），无异常时为空。
    pub health_detail: Option<String>,
    /// 可能仍在被 dsh 写入（复合判据，2026-09-07 立法；2026-09-08 降噪增订：
    /// dsh 引擎进程存活 **且** mtime < 5 分钟 **且** 脚本侧未见正常收尾
    /// endState === 'open'，引擎未运行时恒 false）——仅 UI 提示「运行中」，
    /// 不参与健康判定；活跃会话不应在运行时修复。
    pub active: bool,
    /// 已归档（dsh `workspace.json` 的 `archivedSessionIds`，2026-09-07）：
    /// dsh 侧栏对所有分组与搜索隐藏已归档会话，壳同口径默认隐藏，仅
    /// 「已归档」筛选档展示；归档会话仍可加载可修复。
    pub archived: bool,
    /// 会话创建时间（header.createdAt，毫秒 epoch；脚本不可用时 0）。
    pub created_at: u64,
    /// 展开后的事件总数（信息性统计；不可解析时 0）。
    pub event_count: u64,
    /// 结束状态：最后一条 `turn/end` 的 reason.kind（stop/interrupted…）；
    /// 无 turn/end 记 `open`（未正常收尾，常见于崩溃尾）；不可解析为 None。
    pub end_state: Option<String>,
    /// 子代理会话（header.origin = 'subagent'）。dsh 侧栏同样隐藏此类。
    pub subagent: bool,
    /// 会话代理预设（header.agentPreset，如 standard）。
    pub agent_preset: Option<String>,
    /// 健康判定所用校验器（`dsh-session@版本` / fallback），供 UI 透明展示。
    pub validator: Option<String>,
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

/// 扫描指定 DSH HOME 下的所有会话。
///
/// 健康检查与标题提取经引擎 node 运行内置脚本 `--scan`（只读，不解压进 Rust
/// ——Rust 无 zstd 依赖；脚本与 dsh 加载器语义对齐）。node 不可用时降级：
/// 健康状态标记为 Unknown、标题为空（列表仍可用，修复入口保留）。
/// `engine_alive`（2026-09-07）：dsh 引擎进程存活标志（壳持有的子进程
/// try_wait），注入脚本作为「运行中」复合判据前半——dsh 未运行时 mtime
/// 新鲜不再构成活跃（归档/重命名会刷新 mtime，裸 mtime 判据误标 5 分钟）。
pub fn scan_sessions(
    home: &Path,
    data_dir: &Path,
    engine_alive: bool,
) -> Result<Vec<SessionItem>, String> {
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

                // 会话日志文件发现（2026-09-07 世代感知）：dsh ≥0.1.3 采用不可变
                // 世代模型——v0 源（session.jsonl[.zstd]）与迁移世代
                //（session.vN.jsonl[.zstd]）可能并存，dsh 经 findLog 读取**最高
                // 世代**。同目录多文件时只保留最高世代，保证列表展示与修复入口
                // 对准 dsh 实际读取的文件。
                let mut selected: Option<(PathBuf, bool)> = None;
                if let Ok(log_entries) = fs::read_dir(&s_path) {
                    for log_entry in log_entries.flatten() {
                        let name = log_entry.file_name();
                        let name = name.to_string_lossy();
                        if !is_session_log_filename(&name) {
                            continue;
                        }
                        let candidate = s_path.join(name.as_ref());
                        let better = match &selected {
                            None => true,
                            Some((current, _)) => {
                                session_log_generation(&candidate.to_string_lossy())
                                    > session_log_generation(&current.to_string_lossy())
                            }
                        };
                        if better {
                            let compressed = name.ends_with(".zstd");
                            selected = Some((candidate, compressed));
                        }
                    }
                }
                let Some((target_file, is_compressed)) = selected else {
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
                    title: String::new(), // 由 --scan 结果填充
                    project_name: project_name.clone(),
                    project_dir_raw: project_dir_name.clone(),
                    decoded_project_path,
                    file_path: target_file.to_string_lossy().to_string(),
                    updated_at,
                    size_bytes,
                    is_compressed,
                    has_backup,
                    status: SessionStatus::Unknown, // 由 --scan 结果填充
                    health_detail: None,
                    active: false,
                    archived: false, // 由 workspace.json 归档集合填充
                    created_at: 0,
                    event_count: 0,
                    end_state: None,
                    subagent: false,
                    agent_preset: None,
                    validator: None,
                });
            }
        }
    }

    // 按最后修改时间倒序排列（最新活跃在前）
    items.sort_by_key(|a| std::cmp::Reverse(a.updated_at));

    // 归档标志（2026-09-07）：读 dsh 工作区存储域的归档会话集合。与脚本
    // 健康检查相互独立——node 缺失降级时归档标记仍然生效。
    let archived = read_archived_session_ids(home);
    for item in &mut items {
        item.archived = archived.contains(&item.id);
    }

    // 引擎 node 健康扫描：填充 title 与 status。失败时保持 Unknown 降级。
    if let Ok(health_map) = scan_health_via_script(home, data_dir, engine_alive) {
        for item in &mut items {
            if let Some(h) = health_map.get(&item.file_path) {
                item.title = h.title.clone().unwrap_or_default();
                item.status = match h.status.as_str() {
                    "healthy" => SessionStatus::Healthy,
                    "needs_repair" => SessionStatus::NeedsRepair,
                    _ => SessionStatus::Unknown,
                };
                item.health_detail = h.detail.clone();
                item.active = h.active;
                item.created_at = h.created_at.unwrap_or(0);
                item.event_count = h.event_count.unwrap_or(0);
                item.end_state = h.end_state.clone();
                item.subagent = h.subagent;
                item.agent_preset = h.agent_preset.clone();
                item.validator = h.validator.clone();
            }
        }
    }

    Ok(items)
}

/// 读取 dsh 工作区存储域的归档会话集合：`~/.dsh/storages/workspace.json` 的
/// `global.archivedSessionIds`（裸会话 ID 数组；2026-09-07 实证结构：
/// `{unit:{name,version}, global:{initialized,workspaceIds,archivedSessionIds},
/// tables:{workspaces:{…}}}`，归档动作只原子重写这一个文件、不触碰会话日志）。
/// 容错口径：文件缺失 / JSON 损坏 / 字段缺失一律视为「无归档」——归档信息
/// 缺失只影响默认隐藏，不得阻断列表本身。
fn read_archived_session_ids(home: &Path) -> std::collections::HashSet<String> {
    let path = home.join("storages").join("workspace.json");
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        // 文件不存在 = 归档功能未使用/未生成：正常路径，不告警。
        Err(_) => return Default::default(),
    };
    let parsed: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("workspace.json 解析失败（按无归档处理）：{e}");
            return Default::default();
        }
    };
    parsed
        .get("global")
        .and_then(|g| g.get("archivedSessionIds"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// 会话日志文件名判定（与脚本 `SESSION_LOG_FILENAME` 同口径）：
/// `session.jsonl(.zstd)` 与 `session.vN.jsonl(.zstd)`。
fn is_session_log_filename(name: &str) -> bool {
    let base = name.strip_suffix(".zstd").unwrap_or(name);
    if base == "session.jsonl" {
        return true;
    }
    base.strip_prefix("session.v")
        .and_then(|rest| rest.strip_suffix(".jsonl"))
        .is_some_and(|v| !v.is_empty() && v.parse::<u32>().is_ok())
}

/// 解析会话日志文件名的格式世代：`session.v2.jsonl(.zstd)` → 2；
/// `session.jsonl(.zstd)`（v0 源世代）→ 0。命名方案锚 dsh session-format
/// filename.ts（CANONICAL_LOG_FILENAME，2026-09-07 对照 v0.1.3-alpha.1）。
fn session_log_generation(file_path: &str) -> u32 {
    let name = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let base = name.strip_suffix(".zstd").unwrap_or(name);
    base.strip_prefix("session.v")
        .and_then(|rest| rest.strip_suffix(".jsonl"))
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(0)
}

/// `--scan` 健康检查的 JSON 行（脚本 `scanSessionHealth` 的结构）。
/// title/detail 在脚本侧可为 null（无标题/无异常），必须用 Option 接收——
/// 否则任一 null 字段会让整批反序列化失败（2026-09-05 实测：
/// 一个 title:null 即致全列表降级 Unknown）。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScriptHealthEntry {
    path: String,
    status: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    active: bool,
    #[serde(default)]
    detail: Option<String>,
    // 元数据透出（2026-09-07 扩展项1）：全部 Option/默认值容忍 null 与缺字段
    //（同一批次里混有早退路径的缺省形态，单个 null 不得致整批反序列化失败）。
    #[serde(default)]
    created_at: Option<u64>,
    #[serde(default)]
    event_count: Option<u64>,
    #[serde(default)]
    end_state: Option<String>,
    #[serde(default)]
    subagent: bool,
    #[serde(default)]
    agent_preset: Option<String>,
    #[serde(default)]
    validator: Option<String>,
}

/// 进程内临时脚本命名计数器：并行线程的时间戳可能同纳秒（cargo test 并行
/// 实测过同名互踩），用原子自增保证唯一。
static SCRIPT_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn unique_script_path(prefix: &str) -> PathBuf {
    let seq = SCRIPT_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "{prefix}-{}-{}-{seq}.mjs",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ))
}

/// 运行内置脚本 `--scan`（只读）并解析结果。失败返回 Err（调用方降级）。
fn scan_health_via_script(
    home: &Path,
    data_dir: &Path,
    engine_alive: bool,
) -> Result<std::collections::HashMap<String, ScriptHealthEntry>, String> {
    let script_content = include_str!("../../scripts/repair-session.mjs");
    let script_path = unique_script_path("dsh-dock-scan-session");
    fs::write(&script_path, script_content).map_err(|e| format!("写入临时扫描脚本失败：{e}"))?;

    let node_bin = crate::engines::engine_node_bin(data_dir)
        .ok_or_else(|| "引擎未就绪（node 缺失）".to_string())?;

    let mut cmd = crate::child_cmd(&node_bin);
    cmd.arg(&script_path);
    cmd.arg("--scan");
    cmd.env("DSH_HOME", home);
    // 恢复层校验用引擎档 @deepseek-ai/dsh-session 本尊（与加载该会话的 dsh
    // 同版本，校验结论零漂移）；缺包时脚本内部降级 fallback fold。
    cmd.env("DSH_DOCK_ENGINES", crate::engines::pnpm_home(data_dir));
    // 「运行中」复合判据前半（2026-09-07）：显式 1/0，避免「未设 = 不明」歧义。
    cmd.env("DSH_ENGINE_ALIVE", if engine_alive { "1" } else { "0" });

    let output = cmd
        .output()
        .map_err(|e| format!("执行扫描脚本失败（无法拉起 Node）：{e}"))?;
    let _ = fs::remove_file(&script_path);

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let entries: Vec<ScriptHealthEntry> =
        serde_json::from_str(stdout.trim()).map_err(|e| format!("解析扫描结果失败：{e}"))?;
    Ok(entries.into_iter().map(|e| (e.path.clone(), e)).collect())
}

/// 执行单会话或全量会话修复（通过内置修复脚本）。node 来源引擎档优先
///（ADR-0010：不依赖用户环境），引擎未就绪回退系统探测，双缺给可行动错误
///（不再裸调 PATH 上的 `node`——存在性未知必败且不可诊断）。
///
/// 脚本契约（2026-09-04 重写，与上游 dsh 加载器语义对齐）：
/// - 健康的文件不写回、不备份，退出码 0（幂等 no-op）；
/// - 可安全修复（重放重叠去重 / 连续前缀截断）→ 备份 + 原子写回 + 写后
///   加载器语义校验，成功退出码 0；
/// - 任何失败（无法解析、无法安全修复、校验不过）→ 文件保持原样，退出码非 0，
///   错误信息在 stdout（stderr 仅留给进程级异常）。
pub fn run_repair(
    target: Option<&str>,
    home: &Path,
    data_dir: &Path,
    engine_alive: bool,
) -> Result<RepairOutcome, String> {
    let script_content = include_str!("../../scripts/repair-session.mjs");
    // 脚本路径含 PID + 时间戳 + 进程内自增序号：并发修复（多窗口/并行单测）
    // 互不踩踏，且与「脚本执行中删除自身」的竞态彻底隔离。
    let script_path = unique_script_path("dsh-dock-repair-session");

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
    // 与 --scan 同源：恢复层校验定位引擎档 dsh-session（ADR-0010 资产）。
    cmd.env("DSH_DOCK_ENGINES", crate::engines::pnpm_home(data_dir));
    // 活跃跳过判据与 --scan 同源（复合判据，2026-09-07）。
    cmd.env("DSH_ENGINE_ALIVE", if engine_alive { "1" } else { "0" });

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

    // 全量模式：脚本对每个失败文件打印错误并继续，最终以聚合退出码汇报；
    // 单文件模式：失败即非 0 退出码。两种情况都应以退出码为准。
    if !output.status.success() {
        return Err(format!("修复失败：\n{stderr}\n{stdout}"));
    }

    Ok(RepairOutcome {
        session_id: target.unwrap_or("all").to_string(),
        success: true,
        message: stdout.trim_end().to_string(),
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
    fn scan_health_parses_null_title_and_detail() {
        // 回归（2026-09-05）：脚本对无标题/无异常会话输出 title/detail 为 null，
        // 反序列化必须接受 Option，否则任一会话致整批失败并让列表降级 Unknown。
        let fixture = r#"[
          {"path":"/a/session.jsonl.zstd","status":"healthy","title":null,"detail":null},
          {"path":"/b/session.jsonl.zstd","status":"needs_repair","title":"有标题","detail":"重放重叠"}
        ]"#;
        let entries: Vec<ScriptHealthEntry> = serde_json::from_str(fixture).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].title.is_none());
        assert_eq!(entries[1].title.as_deref(), Some("有标题"));
        assert!(entries[0].detail.is_none());
        assert_eq!(entries[1].detail.as_deref(), Some("重放重叠"));
    }

    #[test]
    fn script_health_entry_parses_extended_metadata() {
        // 扩展项1（2026-09-07）：脚本 --scan 新增元数据字段的反序列化契约——
        // 完整形态、null 形态与缺字段形态混批，单个缺失/ null 不得致整批失败。
        let fixture = r#"[
          {"path":"/a/session.jsonl.zstd","status":"healthy","title":null,"detail":null,
           "createdAt":1788507561510,"eventCount":42,"endState":"stop","subagent":false,
           "agentPreset":"standard","validator":"dsh-session@0.1.2-rc.1"},
          {"path":"/b/session.jsonl.zstd","status":"unknown","title":null,"detail":"header 非法",
           "createdAt":null,"eventCount":0,"endState":null,"subagent":true,"agentPreset":null},
          {"path":"/c/session.jsonl.zstd","status":"needs_repair","title":"t","detail":null}
        ]"#;
        let entries: Vec<ScriptHealthEntry> = serde_json::from_str(fixture).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].created_at, Some(1_788_507_561_510));
        assert_eq!(entries[0].event_count, Some(42));
        assert_eq!(entries[0].end_state.as_deref(), Some("stop"));
        assert!(!entries[0].subagent);
        assert_eq!(entries[0].agent_preset.as_deref(), Some("standard"));
        assert_eq!(
            entries[0].validator.as_deref(),
            Some("dsh-session@0.1.2-rc.1")
        );
        assert!(entries[1].created_at.is_none());
        assert_eq!(entries[1].end_state, None);
        assert!(entries[1].subagent);
        assert!(entries[1].agent_preset.is_none());
        // 缺字段形态：默认值生效
        assert_eq!(entries[2].created_at, None);
        assert!(!entries[2].subagent);
        assert!(entries[2].validator.is_none());
    }

    #[test]
    fn read_archived_session_ids_tolerates_missing_and_malformed() {
        // 容错口径（2026-09-07）：缺失/损坏/字段缺失一律「无归档」，不阻断列表。
        let temp = std::env::temp_dir().join(format!("dsh-sess-arch-parse-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let home = temp.join("home");
        fs::create_dir_all(&home).unwrap();

        // ① 文件缺失（归档功能未使用）
        assert!(read_archived_session_ids(&home).is_empty());

        // ② JSON 损坏
        let storages = home.join("storages");
        fs::create_dir_all(&storages).unwrap();
        fs::write(storages.join("workspace.json"), "{ not json").unwrap();
        assert!(read_archived_session_ids(&home).is_empty());

        // ③ 字段缺失（旧版 schema / zod default 未落盘）
        fs::write(
            storages.join("workspace.json"),
            r#"{"unit":{"name":"workspace","version":2},"global":{"initialized":true},"tables":{}}"#,
        )
        .unwrap();
        assert!(read_archived_session_ids(&home).is_empty());

        // ④ 正常形态（锚 2026-09-07 实证结构）
        fs::write(
            storages.join("workspace.json"),
            r#"{"unit":{"name":"workspace","version":2},"global":{"initialized":true,"workspaceIds":["w0"],"archivedSessionIds":["session-a","session-b"]},"tables":{}}"#,
        )
        .unwrap();
        let ids = read_archived_session_ids(&home);
        assert_eq!(ids.len(), 2);
        assert!(ids.contains("session-a"));
        assert!(ids.contains("session-b"));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn scan_sessions_marks_archived_from_workspace_json() {
        // 归档标记（2026-09-07）：workspace.json 中的会话在列表上带 archived=true；
        // 标记与脚本健康检查相互独立（此处 node 缺失降级 Unknown，归档仍生效）。
        let temp = std::env::temp_dir().join(format!("dsh-sess-arch-scan-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let home = temp.join("home");
        let sess_dir = home
            .join("sessions")
            .join("--demo--")
            .join("session-archived-1");
        fs::create_dir_all(&sess_dir).unwrap();
        fs::write(sess_dir.join("session.jsonl"), "{\"type\":\"session\"}\n").unwrap();
        let live_dir = home
            .join("sessions")
            .join("--demo--")
            .join("session-live-2");
        fs::create_dir_all(&live_dir).unwrap();
        fs::write(live_dir.join("session.jsonl"), "{\"type\":\"session\"}\n").unwrap();

        let storages = home.join("storages");
        fs::create_dir_all(&storages).unwrap();
        fs::write(
            storages.join("workspace.json"),
            r#"{"unit":{"name":"workspace","version":2},"global":{"initialized":true,"workspaceIds":[],"archivedSessionIds":["session-archived-1"]},"tables":{}}"#,
        )
        .unwrap();

        let list = scan_sessions(&home, &temp, false).unwrap();
        assert_eq!(list.len(), 2);
        let archived = list.iter().find(|i| i.id == "session-archived-1").unwrap();
        let live = list.iter().find(|i| i.id == "session-live-2").unwrap();
        assert!(archived.archived, "归档集合中的会话应标记 archived");
        assert!(!live.archived, "未归档会话不应误标");

        let _ = fs::remove_dir_all(&temp);
    }

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

        let list = scan_sessions(&temp, &temp, false).unwrap();
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

        // node 缺失时降级：状态 Unknown、标题空（该目录布局样例无引擎档）。
        let list = scan_sessions(&temp, &temp, false).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "session-12345");
        assert_eq!(list[0].project_name, "my-app");
        assert!(!list[0].is_compressed);

        // 测试删除
        remove_session(&temp, &list[0].file_path).unwrap();
        let after = scan_sessions(&temp, &temp, false).unwrap();
        assert!(after.is_empty());

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn scan_sessions_active_requires_unfinished_end_state() {
        // 「运行中」降噪（2026-09-08，问题记录095 #5）：引擎存活 + mtime 新鲜
        // 时，正常收尾（turn/end）的会话不再标 active；未见收尾才标。回归锚：
        // lib.rs engine_session_alive 曾被 is_none_or 反转（无会话=存活），
        // 与本判据叠加产生「死会话标运行中」——修复以本测试 + is_some_and 锁定。
        let temp = std::env::temp_dir().join(format!("dsh-sess-active-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        // 健康扫描的 node 来源 = engine_node_bin(data_dir)（引擎档）。测试在
        // 临时 data_dir 下铺 engines/bin/node shim 指向系统 node；找不到系统
        // node 则跳过（降级路径已有其他测试覆盖）。
        let data_dir = temp.join("data");
        let bin_dir = data_dir.join("engines").join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        let system_node = find_system_node();
        if let Some(real) = &system_node {
            #[cfg(unix)]
            std::os::unix::fs::symlink(real, bin_dir.join("node")).unwrap();
            #[cfg(windows)]
            fs::write(
                bin_dir.join("node.cmd"),
                format!("@\"{}\" %*\r\n", real.display()),
            )
            .unwrap();
        }

        let header = r#"{"type":"session","version":1,"id":"s-1","createdAt":1700000000000,"delegationDepth":0}"#;
        let mk = |id: &str, body: &str| {
            let d = data_dir
                .join("home")
                .join("sessions")
                .join("--demo--")
                .join(id);
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join("session.jsonl"), body).unwrap();
        };
        mk("session-closed", &format!("{header}\n{{\"type\":\"turn/end\",\"data\":{{\"reason\":{{\"kind\":\"stop\"}}}}}}\n"));
        mk("session-open", &format!("{header}\n"));

        let home = data_dir.join("home");
        let list = scan_sessions(&home, &data_dir, true).unwrap();
        if system_node.is_none() {
            assert!(
                list.iter().all(|i| !i.active),
                "node 缺失 = 健康降级，active 恒 false"
            );
            eprintln!("跳过降噪断言：系统未找到 node");
            let _ = fs::remove_dir_all(&temp);
            return;
        }
        assert_eq!(list.len(), 2);
        let closed = list.iter().find(|i| i.id == "session-closed").unwrap();
        let open = list.iter().find(|i| i.id == "session-open").unwrap();
        assert!(!closed.active, "正常收尾（stop）的会话不得标「运行中」");
        assert_eq!(closed.end_state.as_deref(), Some("stop"));
        assert!(
            open.active,
            "未见收尾（open）的会话 mtime 新鲜 + 引擎存活 = 运行中"
        );

        // 引擎未运行时恒 false（反转修复的另一半：None ≠ 存活）
        let list = scan_sessions(&home, &data_dir, false).unwrap();
        assert!(
            list.iter().all(|i| !i.active),
            "引擎未运行时任何会话都不得标「运行中」"
        );

        let _ = fs::remove_dir_all(&temp);
    }

    /// 测试前置：这些用例真跑要 PATH 上的 `node`（引擎 shim 转发）。
    ///
    /// 2026-09-08（架构评审批次 5）：原来缺 node 直接 `return` —— 静默变「0 断言通过」，
    /// CI 上 node 缺失时红灯不亮、绿灯是假的。现在本地允许跳过（打印提示），
    /// CI 通过 `DSH_TEST_REQUIRE_NODE=1` 强制：缺 node 即 panic。
    fn require_node_or_skip(test_name: &str) -> bool {
        let available = crate::child_cmd(Path::new("node"))
            .arg("-v")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if available {
            return true;
        }
        assert!(
            std::env::var("DSH_TEST_REQUIRE_NODE").is_err(),
            "{test_name}：PATH 上找不到 node，但 CI 要求真跑（DSH_TEST_REQUIRE_NODE=1）"
        );
        eprintln!(
            "跳过 {test_name}：系统未找到 node（本地允许跳过；CI 由 DSH_TEST_REQUIRE_NODE 强制）"
        );
        false
    }

    /// 测试辅助：沿 PATH 探测系统 node（健康扫描 shim 的落点）。
    #[cfg(unix)]
    fn find_system_node() -> Option<std::path::PathBuf> {
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path)
            .map(|d| d.join("node"))
            .find(|p| p.is_file())
    }

    #[cfg(windows)]
    fn find_system_node() -> Option<std::path::PathBuf> {
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path)
            .map(|d| d.join("node.exe"))
            .find(|p| p.is_file())
    }

    // ---------- 损坏类别 fixture（2026-09-08 架构评审批次 5 · P6）----------
    //
    // `scripts/repair-session.mjs` 1,521 行、是唯一改用户数据的脚本。此前每加一个
    // 损坏类别都要手抄一遍脚手架（临时目录 + 引擎 shim + mtime 回拨）；这里把脚手架
    // 收成 fixture 原语，判定改由下面这张表驱动——新增类别 = 表里加一行。

    /// 独立 fixture 根目录（tag + PID + 进程内自增，避免并行用例互踩）。
    fn fixture_home(tag: &str) -> PathBuf {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let home =
            std::env::temp_dir().join(format!("dsh-sess-{tag}-{}-{seq}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        home
    }

    /// 写入会话 fixture：`<home>/sessions/--demo--/<sess>/<file>`，mtime 回拨 1h
    ///（活跃判据 = 引擎存活 + mtime<5min；测试恒传 engine_alive=false）。
    fn write_session_fixture(home: &Path, sess: &str, file: &str, content: &str) -> PathBuf {
        let dir = home.join("sessions").join("--demo--").join(sess);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(file);
        fs::write(&path, content).unwrap();
        // Windows 上 File::open 是只读句柄，set_modified 需要写属性权限
        //（os error 5 PermissionDenied）——write 句柄两平台通用。
        let f = fs::OpenOptions::new().write(true).open(&path).unwrap();
        f.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(3600))
            .unwrap();
        // 立即释放句柄：Windows 上 rename 不能替换仍被打开的文件（EPERM），
        // 而修复链会把改好的 tmp rename 回原名（unix 无此限制）。
        drop(f);
        path
    }

    /// 引擎档 node shim（转发 PATH 上的真 node）——修复链的 node 唯一来源。
    fn install_engine_node_shim(home: &Path) {
        let engine_bin = home.join("engines/bin");
        fs::create_dir_all(&engine_bin).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let shim = engine_bin.join("node");
            fs::write(&shim, "#!/bin/sh\nexec node \"$@\"\n").unwrap();
            fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).unwrap();
        }
        // 与 unix shim 同语义：转发 PATH 上的真 node（空桩在 Windows 上无法执行，
        // 健康检查走不到「版本不受支持」分支——2026-09-07 CI 实证）。
        #[cfg(not(unix))]
        fs::write(engine_bin.join("node.cmd"), b"@node %*\r\n").unwrap();
    }

    /// 扫描 fixture 目录，返回唯一会话条目的判定（status, detail）。
    fn scan_verdict(home: &Path) -> (SessionStatus, String) {
        let list = scan_sessions(home, home, false).unwrap();
        assert_eq!(list.len(), 1, "fixture 应恰好产出一个会话条目");
        (
            list[0].status.clone(),
            list[0].health_detail.clone().unwrap_or_default(),
        )
    }

    /// 拼会话日志：header + 逐行 body + 结尾换行。
    fn session_log(header: &str, body: &[&str]) -> String {
        let mut out = String::from(header);
        for line in body {
            out.push('\n');
            out.push_str(line);
        }
        out.push('\n');
        out
    }

    /// 修复期望（三态，锚脚本契约）：健康 no-op / 可修复备份写回 / 不可修复拒绝。
    enum RepairExpect {
        /// 健康：不写回、不备份、字节不变。
        Untouched,
        /// 可修复：备份 + 写回 + 修复后 `--scan` 健康；`dropped` 必须从内容中消失。
        Repairable { dropped: &'static str },
        /// 不可修复：退出码非 0、不备份、字节不变。
        Refused,
    }

    /// 损坏类别 fixture 行（`scan` / `repair` 两个用例共用同一张表）。
    struct DamageFixture {
        /// 用例名（断言失败时指认是哪一行）。
        name: &'static str,
        /// 会话文件名（世代路由用例需 `session.vN.jsonl`）。
        file: &'static str,
        content: String,
        /// `--scan` 期望判定。
        scan: SessionStatus,
        /// `--scan` 期望 detail 关键子串（空串 = 期望无 detail）。
        scan_detail: &'static str,
        repair: RepairExpect,
    }

    /// 期望值锚 `repair-session.mjs` 顶部「损坏类别」文档 + 实测输出
    ///（2026-09-08 逐条跑 `--scan` / `--all` 取得，detail 为原文）。
    fn damage_fixtures() -> Vec<DamageFixture> {
        let hdr = r#"{"type":"session","version":0,"id":"sess-fx","createdAt":1,"cwd":"/tmp","delegationDepth":0}"#;
        let hdr_v2 = r#"{"type":"session","version":2,"id":"sess-fx","createdAt":1,"cwd":"/tmp","delegationDepth":0,"isSeeded":false}"#;
        let call0 = r#"{"type":"tool/call","seq":0,"time":1,"data":{"turn":1,"step":1,"callId":"c1","name":"bash","arguments":"{}"}}"#;
        let call1 = r#"{"type":"tool/call","seq":1,"time":1,"data":{"turn":1,"step":1,"callId":"c1","name":"bash","arguments":"{}"}}"#;
        let result1 = r#"{"type":"tool/result","seq":1,"time":1,"data":{"turn":1,"step":1,"message":{"role":"user","id":"m0","source":{"kind":"tool","callId":"c1"},"content":[{"type":"tool-result","toolCallId":"c1","content":[{"type":"text","text":"ok"}]}]}},"surfaceOp":"append"}"#;
        let result2 = r#"{"type":"tool/result","seq":2,"time":1,"data":{"turn":1,"step":1,"message":{"role":"user","id":"m0","source":{"kind":"tool","callId":"c1"},"content":[{"type":"tool-result","toolCallId":"c1","content":[{"type":"text","text":"ok"}]}]}},"surfaceOp":"append"}"#;
        let end0 =
            r#"{"type":"turn/end","seq":0,"time":1,"data":{"turn":1,"reason":{"kind":"stop"}}}"#;
        let end1 =
            r#"{"type":"turn/end","seq":1,"time":2,"data":{"turn":1,"reason":{"kind":"stop"}}}"#;
        let end2 =
            r#"{"type":"turn/end","seq":2,"time":2,"data":{"turn":1,"reason":{"kind":"stop"}}}"#;
        let end4 =
            r#"{"type":"turn/end","seq":4,"time":3,"data":{"turn":1,"reason":{"kind":"stop"}}}"#;
        // 重放重叠（类别 1）：连续前缀 + 相同 seq 的重放块，旧占位被遮蔽。
        let placeholder = r#"{"type":"tool/result","seq":1,"time":1,"data":{"turn":1,"step":1,"message":{"role":"user","id":"m0","source":{"kind":"tool","callId":"c1"},"content":[{"type":"tool-result","toolCallId":"c1","content":[{"type":"text","text":"placeholder"}]}]}},"surfaceOp":"append"}"#;
        let step_end2 = r#"{"type":"step/end","seq":2,"time":1,"data":{"turn":1,"step":1}}"#;
        let step_end2b = r#"{"type":"step/end","seq":2,"time":3,"data":{"turn":1,"step":1}}"#;
        let step_start3 = r#"{"type":"step/start","seq":3,"time":4,"data":{"turn":1,"step":2}}"#;
        let interrupted3 = r#"{"type":"turn/end","seq":3,"time":1,"data":{"turn":1,"reason":{"kind":"interrupted"}}}"#;
        let end_seed4 = r#"{"type":"session/end-seed","seq":4,"time":2,"data":{}}"#;
        let chunk4 = r#"{"type":"assistant/chunk","seq":4,"time":5,"data":{"turn":1,"step":2,"chunk":{"type":"text-delta","index":0,"text":"hi"}}}"#;
        let end5 =
            r#"{"type":"turn/end","seq":5,"time":6,"data":{"turn":1,"reason":{"kind":"stop"}}}"#;
        // 悬空 surface replace（类别 4）：end 指向当前 surface 不存在的节点。
        let user0 = r#"{"type":"user/message","seq":0,"time":1,"data":{"role":"user","id":"u0","source":{"kind":"user"},"content":[{"type":"text","text":"hello"}]},"surfaceOp":"append"}"#;
        let summary3 = r#"{"type":"user/message","seq":3,"time":2,"data":{"role":"user","id":"u1","source":{"kind":"user"},"content":[{"type":"text","text":"summary"}]},"surfaceOp":{"op":"replace","start":0,"end":9999}}"#;
        vec![
            DamageFixture {
                name: "健康（seq 连续 + envelope 完整）",
                file: "session.jsonl",
                content: session_log(hdr, &[call0, end1]),
                scan: SessionStatus::Healthy,
                scan_detail: "",
                repair: RepairExpect::Untouched,
            },
            DamageFixture {
                name: "序列缺失（类别 2：seq 0 → 2 跳变）",
                file: "session.jsonl",
                content: session_log(hdr, &[call0, end2]),
                scan: SessionStatus::NeedsRepair,
                scan_detail: "第 3 行 seq=2 跳变（期望 1），已按加载器语义截断到连续前缀",
                repair: RepairExpect::Repairable {
                    dropped: r#""seq":2"#,
                },
            },
            DamageFixture {
                name: "重放重叠（类别 1：旧占位被重放块遮蔽）",
                file: "session.jsonl",
                content: session_log(
                    hdr,
                    &[
                        call0,
                        placeholder,
                        step_end2,
                        interrupted3,
                        end_seed4,
                        result1,
                        step_end2b,
                        step_start3,
                        chunk4,
                        end5,
                    ],
                ),
                scan: SessionStatus::NeedsRepair,
                scan_detail: "重放重叠已修复：丢弃被遮蔽的旧事件",
                repair: RepairExpect::Repairable {
                    dropped: "placeholder",
                },
            },
            DamageFixture {
                name: "悬空 surface replace（类别 4）",
                file: "session.jsonl",
                content: session_log(hdr, &[user0, call1, result2, summary3, end4]),
                scan: SessionStatus::NeedsRepair,
                scan_detail: "surface replace: end seq 9999 not found in surface",
                repair: RepairExpect::Repairable {
                    dropped: r#""replace""#,
                },
            },
            DamageFixture {
                name: "JSON 不可解析（类别 3）",
                file: "session.jsonl",
                content: session_log(hdr, &["not json at all"]),
                scan: SessionStatus::Unknown,
                scan_detail: "第 2 行 JSON 解析失败",
                repair: RepairExpect::Refused,
            },
            DamageFixture {
                name: "末行截断（类别 3：无换行 + 半截 JSON）",
                file: "session.jsonl",
                content: format!(
                    "{hdr}\n{}",
                    r#"{"type":"turn/end","seq":0,"time":1,"data":{"turn":1,"reason":{"kind":"sto"#
                ),
                scan: SessionStatus::Unknown,
                scan_detail: "第 2 行 JSON 解析失败",
                repair: RepairExpect::Refused,
            },
            DamageFixture {
                name: "空文件",
                file: "session.jsonl",
                content: String::new(),
                scan: SessionStatus::Unknown,
                scan_detail: "文件为空",
                repair: RepairExpect::Refused,
            },
            DamageFixture {
                name: "存储版本高于本构建（v2 / fallback 校验）",
                file: "session.v2.jsonl",
                content: session_log(hdr_v2, &[end0]),
                scan: SessionStatus::Unknown,
                scan_detail: "存储格式版本不受支持",
                repair: RepairExpect::Refused,
            },
        ]
    }

    #[test]
    fn scan_classifies_damage_fixtures() {
        // 每个损坏类别一条 fixture：`--scan` 判定 + detail 锚点。此前只有「重放
        // 重叠 / 悬空 replace / 健康」三类有覆盖，类别 2/3 与世代路由的判定
        //（needs_repair vs unknown）无人守住——分类错会让用户点修复被拒，或
        // 本该可修的会话被标成不可修。
        if !require_node_or_skip("scan_classifies_damage_fixtures") {
            return;
        }
        for (i, fx) in damage_fixtures().into_iter().enumerate() {
            let home = fixture_home(&format!("fx-scan-{i}"));
            write_session_fixture(&home, "sess-fx", fx.file, &fx.content);
            install_engine_node_shim(&home);

            let (status, detail) = scan_verdict(&home);
            assert_eq!(status, fx.scan, "[{}] 判定不符，detail：{detail}", fx.name);
            if fx.scan_detail.is_empty() {
                assert!(
                    detail.is_empty(),
                    "[{}] 健康不应带 detail：{detail}",
                    fx.name
                );
            } else {
                assert!(
                    detail.contains(fx.scan_detail),
                    "[{}] detail 缺锚点「{}」，实测：{detail}",
                    fx.name,
                    fx.scan_detail
                );
            }
            let _ = fs::remove_dir_all(&home);
        }
    }

    #[test]
    fn repair_verdict_matches_damage_fixture() {
        // 同一张表的修复侧：健康幂等 no-op / 可修复备份写回且修复后健康 /
        // 不可修复必须非 0 退出并原样保留（脚本契约三条，逐类守住）。
        if !require_node_or_skip("repair_verdict_matches_damage_fixture") {
            return;
        }
        for (i, fx) in damage_fixtures().into_iter().enumerate() {
            let home = fixture_home(&format!("fx-repair-{i}"));
            let path = write_session_fixture(&home, "sess-fx", fx.file, &fx.content);
            install_engine_node_shim(&home);
            let backup = path.with_file_name(format!("{}.bak", fx.file));

            let result = run_repair(Some(path.to_str().unwrap()), &home, &home, false);
            let after = fs::read_to_string(&path).unwrap();

            match fx.repair {
                RepairExpect::Untouched => {
                    result.unwrap_or_else(|e| panic!("[{}] 健康文件修复应成功：{e}", fx.name));
                    assert!(!backup.exists(), "[{}] 健康文件不应备份", fx.name);
                    assert_eq!(after, fx.content, "[{}] 健康文件不应写回", fx.name);
                }
                RepairExpect::Repairable { dropped } => {
                    let outcome = result.unwrap_or_else(|e| panic!("[{}] 应可修复：{e}", fx.name));
                    assert!(outcome.success, "[{}] {}", fx.name, outcome.message);
                    assert!(backup.is_file(), "[{}] 可修复必须留备份", fx.name);
                    assert!(
                        !after.contains(dropped),
                        "[{}] 修复后不应残留 {dropped}",
                        fx.name
                    );
                    assert_eq!(
                        scan_verdict(&home).0,
                        SessionStatus::Healthy,
                        "[{}] 修复后应健康",
                        fx.name
                    );
                }
                RepairExpect::Refused => {
                    assert!(result.is_err(), "[{}] 不可修复应非 0 退出", fx.name);
                    assert!(!backup.exists(), "[{}] 不可修复不得备份", fx.name);
                    assert_eq!(after, fx.content, "[{}] 不可修复必须原样保留", fx.name);
                }
            }
            let _ = fs::remove_dir_all(&home);
        }
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
    fn repair_session_cleans_replay_overlap_and_creates_backup() {
        if !require_node_or_skip("repair_session_cleans_replay_overlap_and_creates_backup") {
            return;
        }

        let temp = fixture_home("repair-replay");

        // 复现真实损坏类别（2026-09-04，锚 dsh v0.1.2-rc.1 加载器语义）：
        // 会话中断恢复后，dsh 以相同 seq 重放被中断轮次的真实事件，
        // 磁盘上残留旧占位事件，形成「连续前缀 + 重放块」重叠。
        // 加载器在重叠处报 seq gap 并丢弃重叠点之后全部恢复事件。
        // 消息形状按 dsh 真实事件补全（id/role/source + surfaceOp）——
        // 恢复层校验（2026-09-07）会逐事件验 envelope，残缺形状会被判损坏。
        let corrupt_data = r#"{"type":"session","version":0,"id":"sess-fail","createdAt":1,"cwd":"/tmp","delegationDepth":0}
{"type":"tool/call","seq":0,"time":1,"data":{"turn":1,"step":1,"callId":"c1","name":"bash","arguments":"{}"}}
{"type":"tool/result","seq":1,"time":1,"data":{"turn":1,"step":1,"message":{"role":"user","id":"m0","source":{"kind":"tool","callId":"c1"},"content":[{"type":"tool-result","toolCallId":"c1","content":[{"type":"text","text":"placeholder"}]}]}},"surfaceOp":"append"}
{"type":"step/end","seq":2,"time":1,"data":{"turn":1,"step":1}}
{"type":"turn/end","seq":3,"time":1,"data":{"turn":1,"reason":{"kind":"interrupted"}}}
{"type":"session/end-seed","seq":4,"time":2,"data":{}}
{"type":"tool/result","seq":1,"time":3,"data":{"turn":1,"step":1,"message":{"role":"user","id":"m1","source":{"kind":"tool","callId":"c1"},"content":[{"type":"tool-result","toolCallId":"c1","content":[{"type":"text","text":"real result"}]}]}},"surfaceOp":"append"}
{"type":"step/end","seq":2,"time":3,"data":{"turn":1,"step":1}}
{"type":"step/start","seq":3,"time":4,"data":{"turn":1,"step":2}}
{"type":"assistant/chunk","seq":4,"time":5,"data":{"turn":1,"step":2,"chunk":{"type":"text-delta","index":0,"text":"hi"}}}
{"type":"turn/end","seq":5,"time":6,"data":{"turn":1,"reason":{"kind":"stop"}}}
"#;
        let target_file = write_session_fixture(&temp, "sess-fail", "session.jsonl", corrupt_data);
        let sess_dir = target_file.parent().unwrap().to_path_buf();
        install_engine_node_shim(&temp);

        let outcome = run_repair(Some(target_file.to_str().unwrap()), &temp, &temp, false).unwrap();
        assert!(outcome.success, "修复应成功：{}", outcome.message);

        // 验证备份文件已创建
        let backup_file = sess_dir.join("session.jsonl.bak");
        assert!(backup_file.is_file());

        // 验证修复后的内容：旧占位事件（tool/result 占位 + step/end + turn/end +
        // session/end-seed，seq 1–4 的旧尾部）被丢弃，重放块保留。
        let repaired_content = fs::read_to_string(&target_file).unwrap();
        let valid_lines: Vec<&str> = repaired_content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect();
        // header + 6 条保留记录（未被遮蔽的 tool/call + 重放块 5 条）
        assert_eq!(valid_lines.len(), 7, "预期保留 header + 6 条记录");
        // 保留块内容：真实 result（非 placeholder）
        assert!(
            repaired_content.contains("\"text\":\"real result\""),
            "应保留重放块的真实结果"
        );
        assert!(
            !repaired_content.contains("placeholder"),
            "不应保留被遮蔽的旧占位内容"
        );
        assert!(
            repaired_content.contains("\"type\":\"turn/end\""),
            "应保留重放块尾部的 turn/end"
        );
        // 顺序与 seq 原样（不重编号、不重排）：seq 0..5 严格递增
        let seqs: Vec<i64> = valid_lines
            .iter()
            .skip(1)
            .map(|l| {
                let v: serde_json::Value = serde_json::from_str(l).unwrap();
                v["seq"].as_i64().or(v["seq0"].as_i64()).unwrap()
            })
            .collect();
        assert_eq!(
            seqs,
            vec![0, 1, 2, 3, 4, 5],
            "重放块 seq 应原样保留（不重编号）"
        );

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn session_log_generation_parses_generation_filenames() {
        // 锚 dsh session-format filename.ts：v0 保持 session.jsonl，v1+ 带 .vN。
        assert_eq!(session_log_generation("/a/session.jsonl"), 0);
        assert_eq!(session_log_generation("/a/session.jsonl.zstd"), 0);
        assert_eq!(session_log_generation("/a/session.v2.jsonl"), 2);
        assert_eq!(session_log_generation("/a/session.v2.jsonl.zstd"), 2);
        assert_eq!(session_log_generation("/a/session.v10.jsonl.zstd"), 10);
        // 非规范名按 v0 处理（去重保守侧：不误删任何条目）。
        assert_eq!(session_log_generation("/a/session.v0.jsonl"), 0);
        assert_eq!(session_log_generation("/a/other.jsonl"), 0);
    }

    #[test]
    fn scan_sessions_prefers_highest_generation_per_session_dir() {
        // dsh ≥0.1.3 世代并存：v0 源与 v2 迁移世代同目录时，dsh 读最高世代，
        // 列表必须只保留该条目（否则同一会话出现两行、修复打在 dsh 不读的文件上）。
        let temp = std::env::temp_dir().join(format!("dsh-test-sess-gen-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let sess_dir = temp.join("sessions").join("--demo--").join("sess-gen");
        fs::create_dir_all(&sess_dir).unwrap();
        fs::write(sess_dir.join("session.jsonl"), "{\"type\":\"session\"}\n").unwrap();
        fs::write(
            sess_dir.join("session.v2.jsonl"),
            "{\"type\":\"session\"}\n",
        )
        .unwrap();

        let list = scan_sessions(&temp, &temp, false).unwrap();
        assert_eq!(
            list.len(),
            1,
            "同目录多世代应去重：{:?}",
            list.iter().map(|i| i.file_path.clone()).collect::<Vec<_>>()
        );
        assert_eq!(
            session_log_generation(&list[0].file_path),
            2,
            "应保留 dsh 实际读取的最高世代"
        );

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn scan_sessions_classifies_newer_generation_as_unknown_not_repair() {
        // 版本路由回归（2026-09-07）：dsh ≥0.1.3 世代文件（version:2）在旧引擎
        // （无 catalog，fallback 校验）下必须归 unknown + 升级提示——不是
        // needs_repair（不可修复，不允许用户点了修复被拒）。
        if !require_node_or_skip("scan_sessions_classifies_newer_generation_as_unknown_not_repair")
        {
            return;
        }

        let temp = std::env::temp_dir().join(format!("dsh-test-sess-v2-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let sess_dir = temp.join("sessions").join("--demo--").join("sess-v2");
        fs::create_dir_all(&sess_dir).unwrap();
        fs::write(
            sess_dir.join("session.v2.jsonl"),
            concat!(
                r#"{"type":"session","version":2,"id":"sess-v2","createdAt":1,"cwd":"/tmp","delegationDepth":0,"isSeeded":false}"#,
                "\n",
                r#"{"type":"turn/end","seq":0,"time":1,"data":{"turn":1,"reason":{"kind":"stop"}}}"#,
                "\n",
            ),
        )
        .unwrap();

        let engine_bin = temp.join("engines/bin");
        std::fs::create_dir_all(&engine_bin).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let shim = engine_bin.join("node");
            std::fs::write(&shim, "#!/bin/sh\nexec node \"$@\"\n").unwrap();
            std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        // 与 unix shim 同语义：转发 PATH 上的真 node（空桩在 Windows 上无法
        // 执行，健康检查走不到「版本不受支持」分支——2026-09-07 CI 实证）。
        #[cfg(not(unix))]
        std::fs::write(engine_bin.join("node.cmd"), b"@node %*\r\n").unwrap();

        let list = scan_sessions(&temp, &temp, false).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(
            list[0].status,
            SessionStatus::Unknown,
            "v2 文件应归 unknown：{:?}",
            list[0].health_detail
        );
        let detail = list[0].health_detail.clone().unwrap_or_default();
        assert!(
            detail.contains("版本不受支持"),
            "detail 应提示版本不受支持：{detail}"
        );

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn repair_session_heals_dangling_surface_replace() {
        // 回归（2026-09-07，session-8650d6f2）：存储层 seq 连续，但
        // surfaceOp.replace 的 end 指向 surface 中不存在的节点（历史重编号
        // 未同步引用）。旧健康检查只看 seq 连续性 → 误判健康；dsh 打开即
        // `invalid seed event at index N: surface replace: end seq X not
        // found in surface`。修复 = 悬空 replace 转 append（内容与 seq 保留）。
        if !require_node_or_skip("repair_session_heals_dangling_surface_replace") {
            return;
        }

        let temp = fixture_home("repair-surface");

        let corrupt_data = r#"{"type":"session","version":0,"id":"sess-surface","createdAt":1,"cwd":"/tmp","delegationDepth":0}
{"type":"user/message","seq":0,"time":1,"data":{"role":"user","id":"u0","source":{"kind":"user"},"content":[{"type":"text","text":"hello"}]},"surfaceOp":"append"}
{"type":"tool/call","seq":1,"time":1,"data":{"turn":1,"step":1,"callId":"c1","name":"bash","arguments":"{}"}}
{"type":"tool/result","seq":2,"time":1,"data":{"turn":1,"step":1,"message":{"role":"user","id":"m0","source":{"kind":"tool","callId":"c1"},"content":[{"type":"tool-result","toolCallId":"c1","content":[{"type":"text","text":"ok"}]}]}},"surfaceOp":"append"}
{"type":"user/message","seq":3,"time":2,"data":{"role":"user","id":"u1","source":{"kind":"user"},"content":[{"type":"text","text":"summary"}]},"surfaceOp":{"op":"replace","start":0,"end":9999}}
{"type":"turn/end","seq":4,"time":3,"data":{"turn":1,"reason":{"kind":"stop"}}}
"#;
        let target_file =
            write_session_fixture(&temp, "sess-surface", "session.jsonl", corrupt_data);
        let sess_dir = target_file.parent().unwrap().to_path_buf();
        install_engine_node_shim(&temp);
        #[cfg(not(unix))]
        std::fs::write(engine_bin.join("node.cmd"), b"@node %*\r\n").unwrap();

        // 健康扫描必须发现该损坏（旧版判定 healthy 的盲区）。
        let list = scan_sessions(&temp, &temp, false).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(
            list[0].status,
            SessionStatus::NeedsRepair,
            "扫描应发现 surface 损坏：{:?}",
            list[0].health_detail
        );
        let detail = list[0].health_detail.clone().unwrap_or_default();
        assert!(
            detail.contains("surface replace: end seq 9999 not found in surface"),
            "detail 应携带 dsh 原始校验错误：{detail}"
        );

        // 修复：悬空 replace → append，内容与 seq 原样。
        let outcome = run_repair(Some(target_file.to_str().unwrap()), &temp, &temp, false).unwrap();
        assert!(outcome.success, "修复应成功：{}", outcome.message);
        assert!(sess_dir.join("session.jsonl.bak").is_file(), "应创建备份");

        let repaired = fs::read_to_string(&target_file).unwrap();
        let lines: Vec<&str> = repaired.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(lines.len(), 6, "修复不得增删记录");
        let summary: serde_json::Value = serde_json::from_str(lines[4]).unwrap();
        assert_eq!(summary["seq"], 3);
        assert_eq!(summary["surfaceOp"], "append", "悬空 replace 应转为 append");
        assert_eq!(
            summary["data"]["content"][0]["text"], "summary",
            "消息内容必须原样保留"
        );

        // 修复后再扫描 = 健康（同一套恢复层校验闭环）。
        let after = scan_sessions(&temp, &temp, false).unwrap();
        assert_eq!(
            after[0].status,
            SessionStatus::Healthy,
            "{:?}",
            after[0].health_detail
        );

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn repair_session_healthy_file_stays_untouched() {
        // 健康文件 = 修复是幂等 no-op：不写回、不创建备份、退出码 0。
        if !require_node_or_skip("repair_session_healthy_file_stays_untouched") {
            return;
        }

        let temp = fixture_home("repair-healthy");

        // 健康样例 = dsh 真实事件形状（消息带 id/role/source、surface 事件带
        // surfaceOp）：恢复层校验（2026-09-07）逐事件验 envelope，缺标记会被
        // 判需修复。
        let healthy_data = r#"{"type":"session","version":0,"id":"sess-ok","createdAt":1,"cwd":"/tmp","delegationDepth":0}
{"type":"tool/call","seq":0,"time":1,"data":{"turn":1,"step":1,"callId":"c1","name":"bash","arguments":"{}"}}
{"type":"tool/result","seq":1,"time":1,"data":{"turn":1,"step":1,"message":{"role":"user","id":"m0","source":{"kind":"tool","callId":"c1"},"content":[{"type":"tool-result","toolCallId":"c1","content":[{"type":"text","text":"ok"}]}]}},"surfaceOp":"append"}
{"type":"turn/end","seq":2,"time":2,"data":{"turn":1,"reason":{"kind":"stop"}}}
"#;
        let target_file = write_session_fixture(&temp, "sess-ok", "session.jsonl", healthy_data);
        let sess_dir = target_file.parent().unwrap().to_path_buf();
        install_engine_node_shim(&temp);

        let outcome = run_repair(Some(target_file.to_str().unwrap()), &temp, &temp, false).unwrap();
        assert!(outcome.success);

        // 文件未被改写（无备份、内容字节一致）
        assert!(
            !sess_dir.join("session.jsonl.bak").is_file(),
            "健康文件不应创建备份"
        );
        assert_eq!(
            fs::read_to_string(&target_file).unwrap(),
            healthy_data,
            "健康文件不应被写回"
        );

        let _ = fs::remove_dir_all(&temp);
    }

    // ---------- 世代分叉（类别 5，2026-09-09）fixture 与测试 ----------
    //
    // 真实引擎（0.1.5-alpha.1）用 format-catalog 流式 API 还原；测试环境以
    // **stub 引擎包**（v0/v3 恒等编解码）驱动脚本的真实分支：检测（世代分叉
    // 标记）、重建（从源重发当前世代）、拒绝（真分叉不动文件）。stub 与真实
    // catalog 的 API 面一致（readHeader/createRestore/encodeCurrentHeader/
    // encodeCurrentEvent），脚本侧 Zero 漂移。

    /// 装配引擎档 stub 包（引擎目录下 global/v11/<id>/node_modules/.pnpm 布局，
    /// 与 findEnginePackageSync 的扫描路径一致）：dsh-session（词汇表）+
    /// dsh-session-format-catalog（v0/v3 恒等编解码）。
    fn install_engine_catalog_stub(home: &Path) {
        fn write_pkg(root: &Path, pkg: &str, body: &str) {
            let dir = root
                .join("global")
                .join("v11")
                .join("stub")
                .join("node_modules")
                .join(".pnpm")
                .join(format!("@deepseek-ai+{pkg}@0.0.0-fixture_"))
                .join("node_modules")
                .join("@deepseek-ai")
                .join(pkg);
            fs::create_dir_all(dir.join("lib")).unwrap();
            fs::write(dir.join("lib").join("index.js"), body).unwrap();
            fs::write(
                dir.join("package.json"),
                format!(r#"{{"name":"{pkg}","type":"module"}}"#),
            )
            .unwrap();
        }
        let engines = home.join("engines");
        write_pkg(
            &engines,
            "dsh-session",
            r#"
export const SESSION_FORMAT_VERSION = 3
export const KNOWN_SESSION_EVENT_TYPES = new Set(['permission/preset','sandbox/mode','approval/policy','session/end-seed','command/run','model/selection','turn/start','step/start','user/message','assistant/message','step/end','turn/end','todo/write','session/title'])
export function adoptSessionEvent() {}
export function interruptedTurnClosers() { return [] }
export function decodeSeqRanges(value) { return value }
export function decodeStorageRecord(record) { return [record] }
export class Session {
  static fromRestore() {}
}
export default {}
"#,
        );
        write_pkg(
            &engines,
            "dsh-session-format-catalog",
            r#"
export const sessionFormatCatalog = {
  currentVersion: 3,
  readHeader(header) {
    if (header.version === 3) return { status: 'current', header }
    if (typeof header.version === 'number' && header.version < 3) return { status: 'migration-required', header }
    return { status: 'unsupported', reason: 'stored Session format is newer than this build' }
  },
  createRestore(header, options) {
    const rows = []
    let finished = false
    return {
      decodeRow(row) {
        if (finished) throw new Error('decodeRow after finish')
        rows.push(row)
      },
      finish() {
        finished = true
        const logical = {
          version: 3,
          id: header.id,
          createdAt: header.createdAt,
          ...(header.cwd !== undefined ? { cwd: header.cwd } : {}),
          isSeeded: header.isSeeded === true || header.seedLength !== undefined,
          ...(header.origin !== undefined ? { origin: header.origin } : {}),
          delegationDepth: header.delegationDepth,
          ...(header.agentPreset !== undefined ? { agentPreset: header.agentPreset } : {}),
        }
        return { header: logical, inheritedEventCount: 0, events: rows }
      },
    }
  },
  encodeCurrentHeader(header, inheritedEventCount) {
    return { type: 'session', ...header }
  },
  encodeCurrentEvent(event) {
    return event
  },
}
"#,
        );
    }

    /// 世代分叉 fixture 的 v0 源（真实会话头部语义：v1→v2 迁移需 system head，
    /// 前置 model/selection；事件形状对齐 dsh 真实记录）。
    fn gen_div_v0() -> &'static str {
        concat!(
            r#"{"type":"session","version":0,"id":"sess-gendiv","createdAt":1,"cwd":"/tmp/demo","delegationDepth":0,"agentPreset":"standard"}"#,
            "\n",
            r#"{"type":"permission/preset","seq":0,"time":1,"data":{"preset":"workspace-write"}}"#,
            "\n",
            r#"{"type":"sandbox/mode","seq":1,"time":1,"data":{"mode":"workspace-write"}}"#,
            "\n",
            r#"{"type":"approval/policy","seq":2,"time":1,"data":{"policy":"ask"}}"#,
            "\n",
            r#"{"type":"session/end-seed","seq":3,"time":2,"data":{}}"#,
            "\n",
            r#"{"type":"command/run","seq":4,"time":3,"data":{"commandId":"cmd-1","name":"permission","args":" danger-full-access","source":{"kind":"user"}}}"#,
            "\n",
            r#"{"type":"model/selection","seq":5,"time":4,"data":{"provider":"deepseek-official","model":"m1","reasoningEffort":"high"}}"#,
            "\n",
            r#"{"type":"turn/start","seq":6,"time":5,"data":{"turn":1}}"#,
            "\n",
            r#"{"type":"step/start","seq":7,"time":6,"data":{"turn":1,"step":1}}"#,
            "\n",
            r#"{"type":"user/message","seq":8,"time":7,"data":{"role":"user","id":"u0","source":{"kind":"user"},"content":[{"type":"text","text":"hello"}]},"surfaceOp":"append"}"#,
            "\n",
            r#"{"type":"assistant/message","seq":9,"time":8,"data":{"turn":1,"step":1,"message":{"role":"assistant","id":"a0","source":{"kind":"model","provider":"deepseek-official","model":"m1"},"content":[{"type":"text","text":"hi"}]}},"surfaceOp":"append"}"#,
            "\n",
            r#"{"type":"step/end","seq":10,"time":9,"data":{"turn":1,"step":1}}"#,
            "\n",
            r#"{"type":"turn/end","seq":11,"time":10,"data":{"turn":1,"reason":{"kind":"stop"}}}"#,
            "\n",
            r#"{"type":"todo/write","seq":12,"time":11,"data":{"todos":[{"content":"a","status":"in_progress"}]}}"#,
            "\n",
        )
    }

    /// v0 源的事件行（不含 header，供构造 v3 世代）。
    fn gen_div_v0_event_lines() -> &'static str {
        concat!(
            r#"{"type":"permission/preset","seq":0,"time":1,"data":{"preset":"workspace-write"}}"#,
            "\n",
            r#"{"type":"sandbox/mode","seq":1,"time":1,"data":{"mode":"workspace-write"}}"#,
            "\n",
            r#"{"type":"approval/policy","seq":2,"time":1,"data":{"policy":"ask"}}"#,
            "\n",
            r#"{"type":"session/end-seed","seq":3,"time":2,"data":{}}"#,
            "\n",
            r#"{"type":"command/run","seq":4,"time":3,"data":{"commandId":"cmd-1","name":"permission","args":" danger-full-access","source":{"kind":"user"}}}"#,
            "\n",
            r#"{"type":"model/selection","seq":5,"time":4,"data":{"provider":"deepseek-official","model":"m1","reasoningEffort":"high"}}"#,
            "\n",
            r#"{"type":"turn/start","seq":6,"time":5,"data":{"turn":1}}"#,
            "\n",
            r#"{"type":"step/start","seq":7,"time":6,"data":{"turn":1,"step":1}}"#,
            "\n",
            r#"{"type":"user/message","seq":8,"time":7,"data":{"role":"user","id":"u0","source":{"kind":"user"},"content":[{"type":"text","text":"hello"}]},"surfaceOp":"append"}"#,
            "\n",
            r#"{"type":"assistant/message","seq":9,"time":8,"data":{"turn":1,"step":1,"message":{"role":"assistant","id":"a0","source":{"kind":"model","provider":"deepseek-official","model":"m1"},"content":[{"type":"text","text":"hi"}]}},"surfaceOp":"append"}"#,
            "\n",
            r#"{"type":"step/end","seq":10,"time":9,"data":{"turn":1,"step":1}}"#,
            "\n",
            r#"{"type":"turn/end","seq":11,"time":10,"data":{"turn":1,"reason":{"kind":"stop"}}}"#,
            "\n",
            r#"{"type":"todo/write","seq":12,"time":11,"data":{"todos":[{"content":"a","status":"in_progress"}]}}"#,
            "\n",
        )
    }

    /// v3 世代 header（与 stub encodeCurrentHeader 输出键序一致）。
    fn gen_div_v3_header() -> &'static str {
        concat!(
            r#"{"type":"session","version":3,"id":"sess-gendiv","createdAt":1,"cwd":"/tmp/demo","isSeeded":false,"delegationDepth":0,"agentPreset":"standard"}"#,
            "\n",
        )
    }

    /// 铺「世代分叉」目录：v0 全量 + v3 仅 header（真实 4885a34d 场景）。
    /// 返回 v3 文件路径。
    fn write_divergence_fixture(home: &Path, tag: &str) -> PathBuf {
        write_session_fixture(home, tag, "session.jsonl", gen_div_v0());
        write_session_fixture(home, tag, "session.v3.jsonl", gen_div_v3_header())
    }

    /// 铺「真分叉」目录：v3 与 v0 互不包含（todo 行内容不同）。
    fn write_fork_fixture(home: &Path, tag: &str) -> PathBuf {
        write_session_fixture(home, tag, "session.jsonl", gen_div_v0());
        let fork_v3 = format!(
            "{}{}",
            gen_div_v3_header(),
            gen_div_v0_event_lines().replace(r#""content":"a""#, r#""content":"FORK-MARKER""#)
        );
        write_session_fixture(home, tag, "session.v3.jsonl", &fork_v3)
    }

    #[test]
    fn scan_flags_generation_divergence_for_rebuild() {
        // 类别 5（2026-09-09，真实 session-4885a34d）：v3 空快照 + v0 全量并存
        // ——引擎本尊还原比对后，最高世代条目必须标需要修复并给出可重建说明。
        if !require_node_or_skip("scan_flags_generation_divergence_for_rebuild") {
            return;
        }
        let home = fixture_home("gen-div-scan");
        let v3_path = write_divergence_fixture(&home, "sess-gendiv");
        install_engine_node_shim(&home);
        install_engine_catalog_stub(&home);

        let list = scan_sessions(&home, &home, false).unwrap();
        assert_eq!(list.len(), 1, "同目录多世代只保留最高世代条目");
        assert_eq!(
            list[0].file_path,
            v3_path.to_string_lossy().to_string(),
            "列表条目应为 dsh 实际读取的最高世代"
        );
        assert_eq!(
            list[0].status,
            SessionStatus::NeedsRepair,
            "世代分叉应标可修复：{:?}",
            list[0].health_detail
        );
        let detail = list[0].health_detail.clone().unwrap_or_default();
        assert!(
            detail.contains("世代分叉") && detail.contains("可无损重建"),
            "detail 应说明分叉与重建路径：{detail}"
        );
        assert_eq!(
            list[0].validator.as_deref(),
            Some("dsh-session@0.0.0-fixture+catalog")
        );

        // 未经 stub 引擎时不误报（fallback 无迁移管线 → 保持未知，不冒充判定）。
        let no_stub = fixture_home("gen-div-scan-no-stub");
        write_divergence_fixture(&no_stub, "sess-gendiv");
        install_engine_node_shim(&no_stub);
        let list2 = scan_sessions(&no_stub, &no_stub, false).unwrap();
        assert_eq!(
            list2[0].status,
            SessionStatus::Unknown,
            "fallback 不得标分叉"
        );

        let _ = fs::remove_dir_all(&home);
        let _ = fs::remove_dir_all(&no_stub);
    }

    #[test]
    fn scan_no_divergence_flag_when_generations_equal() {
        // 两代一致（v3 为源的全量迁移快照）：不得误报分叉。
        if !require_node_or_skip("scan_no_divergence_flag_when_generations_equal") {
            return;
        }
        let home = fixture_home("gen-div-equal");
        write_session_fixture(&home, "sess-gendiv", "session.jsonl", gen_div_v0());
        let v3 = format!("{}{}", gen_div_v3_header(), gen_div_v0_event_lines());
        write_session_fixture(&home, "sess-gendiv", "session.v3.jsonl", &v3);
        install_engine_node_shim(&home);
        install_engine_catalog_stub(&home);

        let list = scan_sessions(&home, &home, false).unwrap();
        assert_eq!(
            list[0].status,
            SessionStatus::Healthy,
            "{:?}",
            list[0].health_detail
        );
        assert!(
            list[0].health_detail.as_deref().unwrap_or("").is_empty(),
            "一致世代不应带 detail：{:?}",
            list[0].health_detail
        );

        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn repair_rebuilds_diverged_generation_from_source() {
        // 一键修复：v3 空快照被按 v0 源重建（备份 + 写后本尊校验），源文件保持
        // 原样；修复后再扫描 = 健康。
        if !require_node_or_skip("repair_rebuilds_diverged_generation_from_source") {
            return;
        }
        let home = fixture_home("gen-div-repair");
        let v3_path = write_divergence_fixture(&home, "sess-gendiv");
        let v0_path = v3_path.with_file_name("session.jsonl");
        let v0_before = fs::read_to_string(&v0_path).unwrap();
        install_engine_node_shim(&home);
        install_engine_catalog_stub(&home);

        let outcome = run_repair(Some(v3_path.to_str().unwrap()), &home, &home, false).unwrap();
        assert!(outcome.success, "{}", outcome.message);
        assert!(
            outcome.message.contains("重建当前世代"),
            "修复消息应说明世代重建：{}",
            outcome.message
        );

        // 备份 = 旧的空 v3；新 v3 = 源全量内容
        let bak = v3_path.with_file_name("session.v3.jsonl.bak");
        assert!(bak.is_file(), "重建必须备份旧世代");
        assert_eq!(
            fs::read_to_string(&bak).unwrap(),
            gen_div_v3_header(),
            "备份应是重建前的空 v3"
        );
        let repaired = fs::read_to_string(&v3_path).unwrap();
        let v3_lines: Vec<&str> = repaired.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(v3_lines.len(), 14, "重建后的 v3 应含 header + 13 条事件");
        assert!(
            v3_lines[1].contains("\"permission/preset\""),
            "重建内容应来自源"
        );

        // 源保持原样（世代只读模型）
        assert_eq!(
            fs::read_to_string(&v0_path).unwrap(),
            v0_before,
            "源文件不得改动"
        );

        // 修复后再扫描 = 健康（同一套比对闭环）
        let after = scan_sessions(&home, &home, false).unwrap();
        assert_eq!(
            after[0].status,
            SessionStatus::Healthy,
            "{:?}",
            after[0].health_detail
        );

        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    fn repair_refuses_true_generation_fork() {
        // 真分叉（两代互不包含）：不可无损合并 —— 修复必须拒绝、文件未动、无备份。
        if !require_node_or_skip("repair_refuses_true_generation_fork") {
            return;
        }
        let home = fixture_home("gen-div-fork");
        let v3_path = write_fork_fixture(&home, "sess-gendiv");
        let v3_before = fs::read_to_string(&v3_path).unwrap();
        install_engine_node_shim(&home);
        install_engine_catalog_stub(&home);

        let err = run_repair(Some(v3_path.to_str().unwrap()), &home, &home, false)
            .expect_err("真分叉必须拒绝修复");
        assert!(
            err.contains("无法无损合并"),
            "拒绝消息应说明合并不可行：{err}"
        );
        assert_eq!(
            fs::read_to_string(&v3_path).unwrap(),
            v3_before,
            "拒绝后文件必须原样保留"
        );
        assert!(
            !v3_path.with_file_name("session.v3.jsonl.bak").exists(),
            "拒绝不得创建备份"
        );

        let _ = fs::remove_dir_all(&home);
    }
}
