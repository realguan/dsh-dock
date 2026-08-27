// 事件总线：5 类事件 → store，payload 边界规整（frontend-migration §3.3）。
// 规整原则：缺字段补默认、未知字段忽略、无法识别的整体丢弃——对
// 「dsh/壳先行升级新增字段」前向兼容。normalize* 为纯函数，Vitest 可直接测。
import { listen } from "@tauri-apps/api/event"
import type { ClientUpdate } from "@/types/ipc"
import {
  EV,
  type AppUpdateEvent,
  type BootErrorEvent,
  type BootProgressEvent,
  type BootStepEvent,
  type VersionsSnapshot,
} from "@/types/events"
import { useBootStore } from "@/stores/bootStore"
import { useClientUpdateStore } from "@/stores/clientUpdateStore"

// ---------- normalize（边界规整纯函数） ----------

export function normalizeStep(payload: unknown): BootStepEvent | null {
  if (typeof payload !== "object" || payload === null) return null
  const p = payload as Record<string, unknown>
  if (typeof p.step !== "number") return null
  const state = p.state
  if (state !== "pending" && state !== "running" && state !== "done" && state !== "error")
    return null
  return { step: p.step, state, detail: typeof p.detail === "string" ? p.detail : "" }
}

export function normalizeProgress(payload: unknown): BootProgressEvent | null {
  if (typeof payload !== "object" || payload === null) return null
  const p = payload as Record<string, unknown>
  if (typeof p.current !== "number" || !Number.isFinite(p.current)) return null
  const total =
    typeof p.total === "number" && Number.isFinite(p.total) ? p.total : null
  return { kind: typeof p.kind === "string" ? p.kind : "node", current: p.current, total }
}

const KNOWN_PHASES = new Set([
  "idle",
  "checking",
  "available",
  "upToDate",
  "downloading",
  "installing",
  "relaunching",
  "done",
  "failed",
])

/// 宽进严出：phase 已知即放行，字段残缺留给组件层兜底——Rust 是状态机唯一
/// 写者，前端只丢「无法识别」的形态（如未来新增 phase），不替 Rust 裁决迁移。
export function normalizeAppUpdate(payload: unknown): AppUpdateEvent | null {
  if (typeof payload !== "object" || payload === null) return null
  const phase = (payload as Record<string, unknown>).phase
  if (typeof phase !== "string" || !KNOWN_PHASES.has(phase)) return null
  return payload as ClientUpdate
}

function isVersionTriplet(v: unknown): boolean {
  if (typeof v !== "object" || v === null) return false
  const p = v as Record<string, unknown>
  return (
    typeof p.current === "string" ||
    p.current === null ||
    p.current === undefined
  )
}

export function normalizeVersions(payload: unknown): VersionsSnapshot | null {
  if (typeof payload !== "object" || payload === null) return null
  const p = payload as Record<string, unknown>
  // dsh / client 为 ComponentUpdate 必有；node 可为 null。容忍未知附加字段。
  if (!isVersionTriplet(p.dsh) || !isVersionTriplet(p.client)) return null
  if (p.node !== null && p.node !== undefined && typeof p.node !== "object") return null
  return p as unknown as VersionsSnapshot
}

export function normalizeError(payload: unknown): BootErrorEvent | null {
  if (typeof payload !== "object" || payload === null) return null
  const p = payload as Record<string, unknown>
  const actions = Array.isArray(p.actions)
    ? p.actions.filter((a): a is string => typeof a === "string")
    : undefined
  return {
    title: typeof p.title === "string" ? p.title : undefined,
    detail: typeof p.detail === "string" ? p.detail : undefined,
    suggestion: typeof p.suggestion === "string" ? p.suggestion : undefined,
    actions,
    log: typeof p.log === "string" ? p.log : undefined,
  }
}

// ---------- 总线初始化（仅 App 顶层 useEffect 调用一次；返回 cleanup） ----------

export function initEventBus(): () => void {
  const unlisteners: Promise<() => void>[] = [
    listen<unknown>(EV.bootStep, ({ payload }) => {
      const e = normalizeStep(payload)
      if (e) useBootStore.getState().setStep(e)
    }),
    listen<unknown>(EV.bootProgress, ({ payload }) => {
      const p = normalizeProgress(payload)
      if (p) useBootStore.getState().setProgress(p)
    }),
    listen<unknown>(EV.bootError, ({ payload }) => {
      const err = normalizeError(payload)
      if (err) useBootStore.getState().setError(err)
    }),
    listen<unknown>(EV.bootUpdate, ({ payload }) => {
      const v = normalizeVersions(payload)
      if (v) useBootStore.getState().setVersions(v)
    }),
    listen<unknown>(EV.appUpdate, ({ payload }) => {
      const ev = normalizeAppUpdate(payload)
      if (ev) useClientUpdateStore.getState().dispatch(ev)
    }),
  ]
  return () => unlisteners.forEach((u) => u.then((fn) => fn()))
}
