// 客户端自更新状态机（About 页消费）。Rust（updater.rs）是唯一写者，
// 前端经 app:update 事件 + get_client_update 播种只读推进。
// applyUpdateEvent / TRANSITIONS 为纯函数：非法迁移丢弃并经 logger 告警。
import { create } from "zustand"
import type { ClientUpdate } from "@/types/ipc"
import type { AppUpdateEvent } from "@/types/events"
import { logger } from "@/lib/logger"

export type UpdatePhase = ClientUpdate["phase"]

/**
 * 合法迁移表（2026-09-11 v1.2.0 实测 3 修订；**同日 T-F6 二次修订**）。
 *
 * **锚定 `updater.rs` 的真实事件序**，而不是"状态机看起来该有的样子"。
 * 现行真实序（2026-09-11 逐行核对 `run_download_and_install`）：
 * ```
 * available
 *   → Checking                      :243（blocked_check 之前先发，让「点下载」立刻有反馈）
 *   → Downloading{0,None}           :262（首个可见下载态，仍在真正下载之前）
 *   → Downloading × N               progress 回调（100ms 节流 ⇒ **重复事件**）
 *   → Installing                    :325（**仅 Windows**：下载完成才发）
 *   → Done{version}                 :363+
 *   → Relaunching                   :379+（随即 app.restart()）
 * ```
 * **非 Windows**（`#[cfg(not(target_os = "windows"))]`，:332 注释）：走
 * `download_and_install` 一体化 API，**没有「下载完成」这个可拦截时刻**，故
 * **不发 `Installing`** ⇒ 序为 `… → Downloading×N → Done → Relaunching`。
 * 两条序都由 `__tests__/clientUpdateSeam.test.ts` 回放钉住。
 *
 * **本表的两轮修订史（都是"两侧各自漂移"的产物）**：
 *   ① T-F（task-46）修「进度被丢」：补 `downloading → downloading` 自环、
 *      `downloading → done`、`done → relaunching`；
 *   ② **T-F6（task-53）修二次漂移**：rust-core 随后在 `blocked_check` 前新增了
 *      `Checking`，真实序首条变成 `checking → downloading`，而本表 `checking`
 *      不含它 ⇒ **整条进度序列从第一条起全丢、界面卡在「检查中」**（问题 3 换形式复发）。
 *
 * ⚠️ **改本表前先读 `__tests__/clientUpdateSeam.test.ts`**——它是跨 IPC 边界的
 * 契约闸门：只测本表自洽（旧测法）**抓不住**两侧漂移，必须用真实发射序回放。
 *
 * **不放宽成万能表**：`idle → done` / `idle → downloading` / `available → relaunching`
 * 仍拒绝；`installing → downloading` 亦拒绝（Rust `reverse_transition_is_rejected`
 * 明文断言它是**倒退**）。
 */
export const TRANSITIONS: Record<UpdatePhase, UpdatePhase[]> = {
  idle: ["checking"],
  // `checking → downloading`：进入下载动作时先发 Checking，随后发首个 Downloading
  // （updater.rs:243 → :262）。缺这条会让整条进度序列从第一天起被丢弃（task-53）。
  checking: ["available", "upToDate", "downloading", "failed"],
  available: ["installing", "downloading", "checking", "failed"],
  upToDate: ["checking", "idle"],
  // 自环：进度是重复事件（progress 回调每 100ms 一发）
  downloading: ["downloading", "installing", "done", "failed"],
  // 注：**不含 `downloading`** —— 安装后再回到下载是相位倒退，Rust 侧
  // `reverse_transition_is_rejected` 明文拒绝；T-F 曾为"可能改序"双向放行，
  // 现序已固定（Windows 下载完成才发 Installing），故收紧回契约。
  installing: ["installing", "done", "relaunching", "failed"],
  relaunching: ["done", "failed"],
  // Done 之后立即 Relaunching
  done: ["relaunching", "idle", "checking", "failed"],
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
