// eventPayloads.ts —— 事件载荷的边界规整（纯函数，无副作用）。
//
// 2026-09-08（ADR-0012）：从 `lib/events.ts` 拆出。原因：`events.ts` 在**模块加载期**
// 装配总线（`export const eventBusStarted = initEventBus()`，见该文件底部注释），
// 因此任何 import 它的测试都会去注册 Tauri 监听并失败——规整逻辑是纯函数，不该
// 被副作用模块绑住（AGENTS §4.4「取数/交互逻辑抽纯函数」）。
//
// 规整原则：缺字段补默认、未知字段忽略、无法识别的整体丢弃——对「dsh/壳先行升级
// 新增字段」前向兼容。
import type { BootFailure, ClientUpdate } from "@/types/ipc"
import type {
  AppUpdateEvent,
  BootErrorEvent,
  BootProgressEvent,
  BootStepEvent,
  VersionsSnapshot,
} from "@/types/events"

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

/// 已知分类（ADR-0012）：只接受白名单内的 kind，未识别的值降级为 undefined——
/// 后端口径变了也不能让前端拿到一个「看起来有分类」的假值。
const BOOT_FAILURE_KINDS: readonly string[] = [
  "credentials_mismatch",
  "incompatible_options",
  "network_unavailable",
  "unknown",
]

function normalizeFailure(value: unknown): BootFailure | undefined {
  if (typeof value !== "object" || value === null) return undefined
  const v = value as Record<string, unknown>
  if (typeof v.kind !== "string" || !BOOT_FAILURE_KINDS.includes(v.kind)) return undefined
  return {
    kind: v.kind as BootFailure["kind"],
    detail: typeof v.detail === "string" ? v.detail : undefined,
  }
}

export function normalizeError(payload: unknown): BootErrorEvent | null {
  if (typeof payload !== "object" || payload === null) return null
  const p = payload as Record<string, unknown>
  const actions = Array.isArray(p.actions)
    ? p.actions.filter((a): a is string => typeof a === "string")
    : undefined
  return {
    failure: normalizeFailure(p.failure),
    title: typeof p.title === "string" ? p.title : undefined,
    detail: typeof p.detail === "string" ? p.detail : undefined,
    suggestion: typeof p.suggestion === "string" ? p.suggestion : undefined,
    actions,
    log: typeof p.log === "string" ? p.log : undefined,
  }
}
