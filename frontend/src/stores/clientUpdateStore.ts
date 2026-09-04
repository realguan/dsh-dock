// 客户端自更新状态机（About 页消费）。Rust（updater.rs）是唯一写者，
// 前端经 app:update 事件 + get_client_update 播种只读推进。
// applyUpdateEvent / TRANSITIONS 为纯函数：非法迁移丢弃并经 logger 告警。
import { create } from "zustand"
import type { ClientUpdate } from "@/types/ipc"
import type { AppUpdateEvent } from "@/types/events"
import { logger } from "@/lib/logger"

export type UpdatePhase = ClientUpdate["phase"]

/// 合法迁移表（锚定 updater.rs set_state 的全部调用位点——
/// run_check / run_download_and_install / Failed 各分支）。done → idle 由
/// 用户「知道了」类交互驱动；failed/upToDate 复查走 checking。
export const TRANSITIONS: Record<UpdatePhase, UpdatePhase[]> = {
  idle: ["checking"],
  checking: ["available", "upToDate", "failed"],
  available: ["downloading", "checking", "failed"],
  upToDate: ["checking", "idle"],
  downloading: ["installing", "failed"],
  installing: ["relaunching", "done", "failed"],
  relaunching: ["done", "failed"],
  done: ["idle", "checking"],
  failed: ["checking", "idle"],
}

export function phaseOf(e: AppUpdateEvent): UpdatePhase {
  return e.phase
}

/// 纯函数迁移动作：s.phase → e.phase 不在合法表内则返回 null（调用方忽略）。
export function applyUpdateEvent(
  s: { phase: UpdatePhase },
  e: AppUpdateEvent,
): AppUpdateEvent | null {
  const allowed = TRANSITIONS[s.phase]
  if (!allowed.includes(e.phase)) return null
  return e
}

export interface ClientUpdateState {
  /** 当前快照；null = 尚未播种（整页重载后先占位，hydrate 后有值） */
  snapshot: ClientUpdate | null

  hydrate: (snapshot: ClientUpdate) => void
  dispatch: (e: AppUpdateEvent) => void
  reset: () => void
}

export const useClientUpdateStore = create<ClientUpdateState>((set, get) => ({
  snapshot: null,

  hydrate: (snapshot) => set({ snapshot }),

  dispatch: (e) => {
    const cur = get().snapshot ?? { phase: "idle" as const }
    const next = applyUpdateEvent(cur, e)
    if (next === null) {
      // 非法迁移：丢事件不崩 UI（对 Rust 先行升级新增迁移路径保持可见性）
      logger.warn("client-update", "非法状态迁移已忽略", {
        from: cur.phase,
        to: e.phase,
      })
      return
    }
    set({ snapshot: next })
  },

  reset: () => set({ snapshot: null }),
}))
