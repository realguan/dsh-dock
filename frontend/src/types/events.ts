// 事件协议类型 + 事件名常量（消灭魔法字符串）。
// 形状锚定 src-tauri/src/lib.rs 的 emit 调用与 updater.rs 的 set_state：
// - boot:step    lib.rs emit_step —— {step, state, detail}
// - boot:progress lib.rs download_progress_bridge —— {kind, current, total}
// - boot:error   lib.rs emit_boot_error / classify_boot_error
// - boot:update  lib.rs emit_update（updates::UpdateStatus 原样序列化）
// - app:update   updater.rs set_state（ClientUpdate，仅发给 main/about 窗口）
import type { ClientUpdate, UpdateStatus } from "./ipc"

export const EV = {
  bootStep: "boot:step",
  bootProgress: "boot:progress",
  bootError: "boot:error",
  bootUpdate: "boot:update",
  appUpdate: "app:update",
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
export interface BootErrorEvent {
  title?: string
  detail?: string
  suggestion?: string
  actions?: string[]
  log?: string
}

export type AppUpdateEvent = ClientUpdate
export type VersionsSnapshot = UpdateStatus
