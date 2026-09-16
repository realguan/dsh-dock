// stores/queueStore.ts —— 插件安装队列编排（095 #4 / ADR-0011 队列形态）。
// 前端编排的**串行**队列：市场安装、总览分发与官方目录的安装/卸载入队后逐项执行
// （install → install_plugin；remove → remove_plugin，2026-09-15 R2）。目标 profile
// 在入队瞬间快照绑定（2026-09-09 串位教训）。构建脚本审批门已改默认批准（ADR-0013）
// ——`blocked_gate` 相位与逐包裁决一并退役，撞门不再需要用户介入。会话级运行态，
// 不持久化；进度流（Rust Channel）为后续切片——本切片队列项只呈现状态机相位。
// 2026-09-15（R2）：新增可 await 句柄 `enqueueAndWait`，让需要「逐步知道成败」的
// 调用方（官方目录编排）能用同一队列，而不是各起一套 —— 见该方法的注释与
// `lib/experimentalCapabilities.ts` 生产绑定处的两条边界记档。
import { create } from "zustand"
import { api } from "@/lib/tauri"
import { useI18nStore } from "@/stores/i18nStore"
import {
  activeCount,
  applyOutcome,
  nextQueued,
  outcomeOf,
  type QueueItem,
  type QueueOutcome,
  type QueueOutcomeLike,
} from "@/lib/queue"

export interface QueueEnqueueInput {
  pkg: string
  spec: string
  profile: string
  kind: "install" | "distribute" | "remove"
  /** distribute 勾选连带配置迁移时的来源 profile（装完复制 patch 行） */
  sourceProfile?: string
  withConfig?: boolean
}

type NoticeFn = (text: string, kind?: "ok" | "warn") => void

// 通知器不进 state（无响应式诉求）：由宿主页面挂载时接线到 toast 链。
let notifier: NoticeFn | null = null
const notify = (text: string, kind?: "ok" | "warn") => notifier?.(text, kind)

// 可 await 句柄的发令枪（2026-09-15 R2）：id → 一次性 resolve。
// 不进 state：它是编排内部的中转，不是可订阅的界面状态。`enqueueAndWait` 之外
// 的入队（`enqueue`）不登记，因此不会被误解析。
const waiters = new Map<string, (o: QueueOutcome) => void>()

interface QueueState {
  items: QueueItem[]
  running: boolean
  /** 最近一次终态（done/failed）时间戳——订阅方据此回填本地安装状态 */
  lastFinishedAt: number
  setNotifier: (n: NoticeFn | null) => void
  enqueue: (input: QueueEnqueueInput) => void
  /** 入队并等到该项终结（2026-09-15 R2）。**一次调用对应一次终结**：面板上的
   *  「重试」会重跑该项，但不会重新解析本 Promise，也不会续跑调用方的编排
   *  （编排进度留在调用方，队列不持有它）。 */
  enqueueAndWait: (input: QueueEnqueueInput) => Promise<QueueOutcome>
  /** failed → queued（手动重试） */
  retry: (id: string) => void
  dismiss: (id: string) => void
  clearFinished: () => void
}

export const useQueueStore = create<QueueState>((set, get) => {
  const pump = async () => {
    if (get().running) return
    const next = nextQueued(get().items)
    if (!next) return
    set((s) => ({
      running: true,
      items: s.items.map((i) =>
        i.id === next.id ? { ...i, status: "installing" as const } : i,
      ),
    }))
    let finished: QueueItem
    try {
      // remove 走卸载调用（无 spec）；install/distribute 都是安装，distribute 的
      // 差别只在成功文案与配置迁移，不在调用形态。
      const outcome: QueueOutcomeLike = await (
        next.kind === "remove"
          ? api.removePlugin(next.profile, next.pkg)
          : api.installPlugin(next.profile, next.spec)
      ).catch((e: unknown) => ({ ok: false, detail: String(e) }))
      finished = applyOutcome(next, outcome)
    } catch (e) {
      finished = { ...next, status: "failed", detail: String(e) }
    }
    const terminal = finished.status === "done" || finished.status === "failed"
    set((s) => ({
      running: false,
      lastFinishedAt: terminal ? Date.now() : s.lastFinishedAt,
      items: s.items.map((i) => (i.id === finished.id ? finished : i)),
    }))
    const { t } = useI18nStore.getState()
    if (finished.status === "done") {
      notify(
        finished.kind === "distribute"
          ? t.profiles.distributeDone(finished.pkg, finished.profile)
          : finished.kind === "remove"
            ? t.market.queueRemoveDone(finished.pkg, finished.profile)
            : t.market.installSuccess(finished.pkg, finished.profile),
        "ok",
      )
    } else if (finished.status === "failed") {
      notify(
        finished.kind === "remove"
          ? t.market.queueRemoveFailedNotice(finished.pkg, finished.detail ?? "")
          : t.market.queueFailedNotice(finished.pkg, finished.detail ?? ""),
        "warn",
      )
    }
    // 发令枪：先摘除再 resolve，保证重试不会二次解析同一个 Promise。
    if (terminal) {
      const resolve = waiters.get(finished.id)
      if (resolve) {
        waiters.delete(finished.id)
        resolve(outcomeOf(finished))
      }
    }
    // 串行推进：当前项终结后立即取下一项
    void pump()
  }

  const push = (input: QueueEnqueueInput): QueueItem => {
    const item: QueueItem = {
      id: `q-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
      status: "queued" as const,
      createdAt: Date.now(),
      ...input,
    }
    set((s) => ({ items: [...s.items, item] }))
    notify(
      input.kind === "remove"
        ? useI18nStore.getState().t.market.queueRemoveQueued(input.pkg, input.profile)
        : useI18nStore.getState().t.market.installQueued(input.pkg, input.profile),
      "ok",
    )
    return item
  }

  return {
    items: [],
    running: false,
    lastFinishedAt: 0,

    setNotifier: (n) => {
      notifier = n
    },

    enqueue: (input) => {
      push(input)
      void pump()
    },

    enqueueAndWait: (input) => {
      const item = push(input)
      const p = new Promise<QueueOutcome>((resolve) => {
        waiters.set(item.id, resolve)
      })
      void pump()
      return p
    },

    retry: (id) => {
      set((s) => ({
        items: s.items.map((i) =>
          i.id === id && i.status === "failed"
            ? { ...i, status: "queued" as const, detail: undefined }
            : i,
        ),
      }))
      void pump()
    },

    dismiss: (id) => {
      set((s) => ({ items: s.items.filter((i) => i.id !== id) }))
    },

    clearFinished: () => {
      set((s) => ({
        items: s.items.filter(
          (i) => i.status !== "done" && i.status !== "failed",
        ),
      }))
    },
  }
})

export { activeCount }
