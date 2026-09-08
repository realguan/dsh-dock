//! commands/market.rs —— market 域 IPC 命令处理层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 本层只做「参数校验 + 数据目录定位 + 阻塞动作下沉 spawn_blocking」，
//! 业务实现全在对应域模块（profiles/plugins/sessions/settings/…）。
//! 命令清单的唯一事实源仍是 `src/ipc.rs::COMMANDS`，三处同步由 cargo test 闸门拦。

/// 插件市场：拉取社区 Registry 静态 JSON 目录
#[tauri::command]
pub async fn fetch_market_registry() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(crate::updates::fetch_market_registry)
        .await
        .map_err(|e| format!("拉取插件市场任务异常终止：{e}"))?
}
