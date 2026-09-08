//! commands/ —— IPC 命令处理层（2026-09-08 架构评审批次 2，P1 拆 lib.rs 第一步）。
//!
//! 55 个 `#[tauri::command]` 按域分文件；每文件只做「参数校验 + 数据目录定位 +
//! 阻塞动作下沉 `spawn_blocking`」，业务实现留在各域模块。
//! 命令清单的唯一事实源 = `src/ipc.rs::COMMANDS`（handler / capabilities / tauri.ts
//! 四处一致由 `ipc::gate_tests` 拦，handler 条目可带模块路径）。

pub mod boot;
pub mod console;
pub mod link;
pub mod market;
pub mod plugin;
pub mod profile;
pub mod session;
pub mod update;
pub mod window;
