//! commands/plugin.rs —— plugin 域 IPC 命令处理层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 本层只做「参数校验 + 数据目录定位 + 阻塞动作下沉 spawn_blocking」，
//! 业务实现全在对应域模块（profiles/plugins/sessions/settings/…）。
//! 命令清单的唯一事实源仍是 `src/ipc.rs::COMMANDS`，三处同步由 cargo test 闸门拦。

use crate::boot::{active_session_profile, ShellState};
use std::sync::Arc;

use tauri::Manager;

/// 插件清单（4.4①，Spike B 方案）：静态清单 = bundles（官方内置）+
/// dependencies（第三方，含已装版本/描述）。阻塞文件操作走 spawn_blocking。
#[tauri::command]
pub async fn list_profile_plugins(
    profile: String,
) -> Result<Vec<crate::plugins::PluginEntry>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let home = crate::resolve::user_dsh_home();
        crate::plugins::list_profile_plugins(&home, &profile)
    })
    .await
    .map_err(|e| format!("清单任务异常终止：{e}"))?
}
/// 插件运行态快照（一次性，不订阅）：仅活跃会话的 profile 有数据——
/// 无会话返回 `{profile: None, entries: []}`。回环只读查询见 AGENTS §7
/// 登记与复现点 11；阻塞 HTTP 走 spawn_blocking（2s 超时兜底）。
#[tauri::command]
pub async fn get_plugin_runtime(
    app: tauri::AppHandle,
) -> Result<crate::plugins::PluginRuntimeSnapshot, String> {
    let target = {
        let state = app.state::<Arc<ShellState>>().inner().clone();
        let profile = active_session_profile(&app);
        let origin = state.workbench_url.lock().unwrap().as_ref().map(|u| {
            format!(
                "{}://{}{}",
                u.scheme(),
                u.host_str().unwrap_or("127.0.0.1"),
                u.port().map(|p| format!(":{p}")).unwrap_or_default()
            )
        });
        (profile, origin)
    };
    match target {
        (Some(profile), Some(origin)) => {
            let entries = tauri::async_runtime::spawn_blocking(move || {
                crate::plugins::fetch_runtime_snapshot(&origin)
            })
            .await
            .map_err(|e| format!("运行态任务异常终止：{e}"))??;
            Ok(crate::plugins::PluginRuntimeSnapshot {
                profile: Some(profile),
                entries,
            })
        }
        _ => Ok(crate::plugins::PluginRuntimeSnapshot {
            profile: None,
            entries: Vec::new(),
        }),
    }
}
/// 安装/卸载/更新插件（4.4②）：`dsh plugin --profile <名> add/remove/update`
/// 转发链（复用创建刀基建，pnpm 防御补齐同源）；阻塞转发走 spawn_blocking，
/// 超时同创建 600s。ok=false 时 detail 带输出尾部，前端按警示态展示。
#[tauri::command]
pub async fn install_plugin(
    app: tauri::AppHandle,
    profile: String,
    package: String,
) -> Result<crate::plugins::PluginOpOutcome, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("定位数据目录失败：{e}"))?;
    tauri::async_runtime::spawn_blocking(move || {
        crate::plugins::mutate_plugin_blocking(
            crate::plugins::PluginOp::Install,
            &profile,
            &package,
            &data_dir,
        )
    })
    .await
    .map_err(|e| format!("安装任务异常终止：{e}"))?
}
#[tauri::command]
pub async fn remove_plugin(
    app: tauri::AppHandle,
    profile: String,
    package: String,
) -> Result<crate::plugins::PluginOpOutcome, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("定位数据目录失败：{e}"))?;
    tauri::async_runtime::spawn_blocking(move || {
        crate::plugins::mutate_plugin_blocking(
            crate::plugins::PluginOp::Remove,
            &profile,
            &package,
            &data_dir,
        )
    })
    .await
    .map_err(|e| format!("卸载任务异常终止：{e}"))?
}
#[tauri::command]
pub async fn update_plugin(
    app: tauri::AppHandle,
    profile: String,
    package: String,
) -> Result<crate::plugins::PluginOpOutcome, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("定位数据目录失败：{e}"))?;
    tauri::async_runtime::spawn_blocking(move || {
        crate::plugins::mutate_plugin_blocking(
            crate::plugins::PluginOp::Update,
            &profile,
            &package,
            &data_dir,
        )
    })
    .await
    .map_err(|e| format!("更新任务异常终止：{e}"))?
}
/// 插件行表（4.4③）：`dsh --profile <名> --dump-config` 行 id↔包名配对 +
/// 壳 patch toggle 态——行 id 不可从包名推导（ADR-0009 第四次修订），一次
/// spawn 全量拿到。阻塞 spawn 走 spawn_blocking。
#[tauri::command]
pub async fn get_plugin_rows(
    app: tauri::AppHandle,
    profile: String,
) -> Result<Vec<crate::plugins::PluginRowState>, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("定位数据目录失败：{e}"))?;
    tauri::async_runtime::spawn_blocking(move || {
        crate::plugins::plugin_rows_blocking(&profile, &data_dir)
    })
    .await
    .map_err(|e| format!("行表任务异常终止：{e}"))?
}
/// 禁用/启用切换（4.4③）：patch 写入例外 #3（`{id, disabled}` 单键，
/// ADR-0009 第四次修订）；运行中会话不热生效，重启承接。
#[tauri::command]
pub async fn set_plugin_disabled(
    profile: String,
    row_id: String,
    disabled: bool,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let home = crate::resolve::user_dsh_home();
        crate::plugins::set_plugin_disabled(&home, &profile, &row_id, disabled)
    })
    .await
    .map_err(|e| format!("切换任务异常终止：{e}"))?
}
/// 更新检查（4.4④）：逐外挂插件查 registry dist-tags.latest（外网经
/// `updates.rs` 镜像链，§7 已登记）；串行阻塞走 spawn_blocking，按钮触发。
#[tauri::command]
pub async fn check_plugin_updates(
    profile: String,
) -> Result<crate::plugins::PluginUpdateReport, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let home = crate::resolve::user_dsh_home();
        crate::plugins::check_updates_blocking(&home, &profile)
    })
    .await
    .map_err(|e| format!("更新检查任务异常终止：{e}"))?
}
/// 版本列表（选版本更新，4.4④）：降序最新在前；外网同镜像链。
#[tauri::command]
pub async fn list_plugin_versions(package: String) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || crate::plugins::plugin_versions_blocking(&package))
        .await
        .map_err(|e| format!("版本查询任务异常终止：{e}"))?
}
/// 插件总览聚合（4.4④ 收口，ADR-0009 第五次修订）：全部已物化 profile 的第
/// 三方插件按包名归组。只读纯文件扫描（零 dsh 子进程、零网络），spawn_blocking。
#[tauri::command]
pub async fn list_all_plugins() -> Result<Vec<crate::plugins::AggregatePlugin>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        Ok(crate::plugins::aggregate_plugins_blocking(
            &crate::resolve::user_dsh_home(),
        ))
    })
    .await
    .map_err(|e| format!("聚合任务异常终止：{e}"))?
}
/// 配置行原样复制（4.4④ 收口，patch 写入例外 #4，ADR-0009 第五次修订）：
/// 来源 patch 中该插件行 id 的全部条目 → 追加到目标 patch（只追加不覆盖，
/// 目标已有同 id 条目则零写入 skipped）。dump-config spawn + 文件操作走
/// spawn_blocking。
#[tauri::command]
pub async fn copy_plugin_config(
    app: tauri::AppHandle,
    source: String,
    target: String,
    package: String,
) -> Result<crate::plugins::CopyConfigOutcome, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("定位数据目录失败：{e}"))?;
    tauri::async_runtime::spawn_blocking(move || {
        crate::plugins::copy_plugin_config_blocking(
            &crate::resolve::user_dsh_home(),
            &source,
            &target,
            &package,
            &data_dir,
        )
    })
    .await
    .map_err(|e| format!("配置复制任务异常终止：{e}"))?
}
