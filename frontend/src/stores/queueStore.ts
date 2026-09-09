// stores/queueStore.ts —— 插件安装队列编排（095 #4 / ADR-0011 队列形态）。
// 前端编排的**串行**队列：市场安装与总览分发入队后逐项执行 install_plugin。
// 目标 profile 在入队瞬间快照绑定（2026-09-09 串位教训）。构建脚本审批门已改
// 默认批准（ADR-0013）——`blocked_gate` 相位与逐包裁决一并退役，撞门不再需要
// 用户介入。会话级运行态，不持久化；进度流（Rust Channel）为后续切片——本切片
// 队列项只呈现状态机相位。
import { create } from "zustand"
import { api } from "@/lib/tauri"
import { useI18nStore } from "@/stores/i18nStore"
import {
  activeCount,
  applyOutcome,
  nextQueued,
  type QueueItem,
  type QueueOutcomeLike,
} from "@/lib/queue"

export interface QueueEnqueueInput {
  pkg: string
  spec: string
  profile: string
  kind: "install" | "distribute"
  /** distribute 勾选连带配置迁移时的来源 profile（装完复制 patch 行） */
  sourceProfile?: string
  withConfig?: boolean
}

type NoticeFn = (text: string, kind?: "ok" | "warn") => void

// 通知器不进 state（无响应式诉求）：由宿主页面挂载时接线到 toast 链。
let notifier: NoticeFn | null = null
const notify = (text: string, kind?: "ok" | "warn") => notifier?.(text, kind)

interface QueueState {
  items: QueueItem[]
  running: boolean
  /** 最近一次终态（done/failed）时间戳——订阅方据此回填本地安装状态 */
  lastFinishedAt: number
  setNotifier: (n: NoticeFn | null) => void
  enqueue: (input: QueueEnqueueInput) => void
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
      const outcome: QueueOutcomeLike = await api
        .installPlugin(next.profile, next.spec)
        .catch((e: unknown) => ({ ok: false, detail: String(e) }))
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
          : t.market.installSuccess(finished.pkg, finished.profile),
        "ok",
      )
    } else if (finished.status === "failed") {
      notify(t.market.queueFailedNotice(finished.pkg, finished.detail ?? ""), "warn")
    }
    // 串行推进：当前项终结后立即取下一项
    void pump()
  }

  return {
    items: [],
    running: false,
    lastFinishedAt: 0,

    setNotifier: (n) => {
      notifier = n
    },

    enqueue: (input) => {
      set((s) => ({
        items: [
          ...s.items,
          {
            id: `q-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
            status: "queued" as const,
            createdAt: Date.now(),
            ...input,
          },
        ],
      }))
      notify(
        useI18nStore.getState().t.market.installQueued(input.pkg, input.profile),
        "ok",
      )
      void pump()
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
