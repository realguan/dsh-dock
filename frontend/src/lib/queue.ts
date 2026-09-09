// lib/queue.ts —— 插件安装队列纯逻辑（095 #4 / ADR-0011 队列形态）。
// 只承载状态迁移与选择，网络/IPC 编排见 stores/queueStore；迁移函数全部
// 纯函数可单测（AGENTS §4.4：Vitest 只测纯逻辑）。

export type QueueStatus = "queued" | "installing" | "blocked_gate" | "done" | "failed"

export interface QueueItem {
  id: string
  /** install = 市场安装；distribute = 总览分发（可连带配置迁移） */
  kind: "install" | "distribute"
  /** 展示名（市场条目名或依赖包名） */
  pkg: string
  /** 安装 spec（git/tarball 来源即来源地址，npm 来源为名@区间） */
  spec: string
  /** 目标 profile——入队瞬间快照绑定，审批/重试不可改写（2026-09-09） */
  profile: string
  status: QueueStatus
  detail?: string
  /** blocked_gate 时被点名的 allowBuilds 键 */
  gatePkgs?: string[]
  /** distribute 勾选连带配置迁移时的来源 profile（装完复制 patch 行） */
  sourceProfile?: string
  withConfig?: boolean
  createdAt: number
}

/** install_plugin 返回的结构形（避免与 types/ipc 循环依赖的本地形） */
export interface QueueOutcomeLike {
  ok: boolean
  detail?: string | null
  ignored_builds?: string[] | null
}

/** 串行执行：队列中最早入队的待处理项。 */
export function nextQueued(items: QueueItem[]): QueueItem | undefined {
  return items.find((i) => i.status === "queued")
}

/** 一次安装调用的结果落到队列项：成功 / 审批门阻塞 / 失败三态。 */
export function applyOutcome(item: QueueItem, outcome: QueueOutcomeLike): QueueItem {
  if (outcome.ok) {
    return { ...item, status: "done", detail: undefined, gatePkgs: undefined }
  }
  if (outcome.ignored_builds && outcome.ignored_builds.length > 0) {
    return { ...item, status: "blocked_gate", gatePkgs: outcome.ignored_builds }
  }
  return { ...item, status: "failed", detail: outcome.detail ?? undefined }
}

/** 面板角标：未终结（排队/安装中/待审批）的项数。 */
export function activeCount(items: QueueItem[]): number {
  return items.filter(
    (i) => i.status === "queued" || i.status === "installing" || i.status === "blocked_gate",
  ).length
}
