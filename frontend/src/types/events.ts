// 事件协议类型 + 事件名常量（消灭魔法字符串）。
// 形状锚定 src-tauri/src/lib.rs 的 emit 调用与 updater.rs 的 set_state：
// - boot:step    lib.rs emit_step —— {step, state, detail}
// - boot:progress lib.rs download_progress_bridge —— {kind, current, total}
// - boot:error   boot.rs emit_boot_error —— BootErrorPayload（含 failure.kind）
// - boot:update  lib.rs emit_update（updates::UpdateStatus 原样序列化）
// - app:update   updater.rs set_state（ClientUpdate，仅发给 main/about 窗口）
// - app:settings-changed  commands/console.rs::set_shell_settings —— ShellSettings
//   全量（含 locale / switcherShortcut 等）。2026-09-11（task-27）：该名字原分别
//   硬编码在 App.tsx 与 QuickDshSwitcher.tsx 两处，收敛到此处。
import type { BootFailure, ClientUpdate, ShellSettings, UpdateStatus } from "./ipc"

export const EV = {
  bootStep: "boot:step",
  bootProgress: "boot:progress",
  bootError: "boot:error",
  bootUpdate: "boot:update",
  appUpdate: "app:update",
  settingsChanged: "app:settings-changed",
} as const

export type BootStepState = "pending" | "running" | "done" | "error"

export interface BootStepEvent {
  step: number
  state: BootStepState
  detail: string
}

export interface BootProgressEvent {
  kind: string
  current: number
  total: number | null
}

/// boot:error 的真实载荷：actions[] 由后端下发（可行动动作 id 集合），
/// 前端只做 id→文案映射（content/zh-CN.ts error.actions），不自行决定动作集合。
///
/// 2026-09-08（ADR-0012）：新增 `failure`（结构化分类）。旧缓存/旧后端载荷没有该
/// 字段 → 保持可选，ErrorCard 回退后端 `title`/`suggestion`。
export interface BootErrorEvent {
  failure?: BootFailure
  title?: string
  detail?: string
  suggestion?: string
  actions?: string[]
  log?: string
}

export type AppUpdateEvent = ClientUpdate
export type VersionsSnapshot = UpdateStatus
/// `app:settings-changed` 载荷：壳设置全量（locale / switcherShortcut / …）。
/// 与 `ShellSettings` 同形——emit 侧直接序列化该结构（commands/console.rs）。
export type SettingsChangedEvent = ShellSettings
