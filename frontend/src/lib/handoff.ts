// 交接（handoff）纯模型（2026-09-10，ADR-0014）。
//
// 背景：一次「重启 / 切换 profile」= 停旧 dsh → 起新 dsh → 等就绪 → 导航进工作台，
// 主窗口在这中间会经历两次**整文档替换**（工作台 → 壳启动屏 → 工作台），
// 控制中心窗口则一直活着。要让用户感到"一条 loading 贯穿到底"，必须满足：
//
//   1. 两个窗口对"现在到哪一步了"给出**同一个答案**（否则状态接不上）；
//   2. 计时器跨文档**不归零**（归零 = 又开了一段新的等待，割裂感即来自此）；
//   3. 阶段只前进不后退（Rust 显式推进 + boot:step 事件流兜底，取二者更靠前者）。
//
// 本模块是「1 + 3」的唯一折算处：输入 = Rust 的交接意图（`get_boot_status.intent`）
// 与 boot:step 事件流折算出的五步视图，输出 = 一条四段导轨的视图模型。
// 纯函数、无 React / 无 IO —— 供 `__tests__/handoff.test.ts` 穷尽覆盖。
import type { HandoffIntent, HandoffSnapshot, HandoffKind, HandoffPhase } from "@/types/ipc"

/** 五步启动时间线的步骤数（与 bootStore.STEP_COUNT 对齐；此处不 import
    以免纯模型层反向依赖 store）。 */
const STEP_COUNT = 5

/** 四段导轨（用户可读的阶段序；与 Rust HandoffPhase 的映射见 phaseStage）。 */
export const HANDOFF_STAGES = ["stopping", "booting", "waiting", "entering"] as const
export type HandoffStage = (typeof HANDOFF_STAGES)[number]

const STAGE_INDEX: Record<HandoffStage, number> = {
  stopping: 0,
  booting: 1,
  waiting: 2,
  entering: 3,
}

/** 阶段序（只用于「取更靠前者」，不得跳级判定）。 */
const PHASE_RANK: Record<HandoffPhase, number> = {
  stopping: 0,
  booting: 1,
  waiting: 2,
  entering: 3,
  ready: 4,
  failed: 4,
}

export interface HandoffView {
  /** 交接仍在途（Rust 判定：未终结且未超 TTL）；ready/failed 后由 UI 决定停留 */
  inflight: boolean
  kind: HandoffKind
  target: string
  phase: HandoffPhase
  /** 四段导轨的当前节点下标（0..3）；ready/failed 时等于最后完成的那段 */
  stageIndex: number
  /** 已完成段数（0..4）：导轨填充用，只增不减 */
  completedStages: number
  /** 交接起始时刻（Unix ms）：计时叶子组件据此自走（本视图不掺时钟，
      免得 5Hz 重渲染整个页面） */
  startedAt: number
  failed: boolean
  done: boolean
  /** 五步时间线已完成的步数（0..5；启动屏分段进度用） */
  stepsDone: number
}

/**
 * 阶段折算：Rust 显式推进的阶段 vs boot:step 事件流推出的阶段，取更靠前者。
 * 为什么取 max：事件是异步广播，重启瞬间控制中心可能先收到事件、Rust 阶段
 * 还停在上一格；取 max 保证导轨不会"倒退一格再前进"（视觉抖动）。
 */
export function phaseFromSteps(steps: { status: string }[]): HandoffPhase | null {
  if (steps.some((s) => s.status === "error")) return null
  const st = (i: number) => steps[i]?.status
  if (st(4) === "done") return "ready"
  if (st(4) === "running") return "entering"
  if (st(3) === "running" || st(3) === "done") return "waiting"
  if (st(2) === "running" || st(2) === "done") return "booting"
  if (st(0) === "running" || st(1) === "running" || st(0) === "done" || st(1) === "done") {
    return "booting"
  }
  return null
}

/** 折算交接视图；无意图返回 null。时钟不在此掺入（见 startedAt 注释）。 */
export function deriveHandoff(
  intent: HandoffIntent | HandoffSnapshot | null,
  steps: { status: string }[],
): HandoffView | null {
  if (!intent) return null
  const fromSteps = phaseFromSteps(steps)
  const phase =
    fromSteps && PHASE_RANK[fromSteps] > PHASE_RANK[intent.phase] ? fromSteps : intent.phase
  const failed = phase === "failed"
  const done = phase === "ready"
  const stageIndex = done ? HANDOFF_STAGES.length - 1 : STAGE_INDEX[stageOf(phase)]
  const stepsDone = steps.filter((s) => s.status === "done").length
  return {
    // ready/failed 之后导轨不立刻收起（由 UI 停留一拍再淡出）：那一刻用户正
    // 盯着主窗口看工作台出来，突然消失反而像"没做完"。
    inflight: "active" in intent ? intent.active : true,
    kind: intent.kind,
    target: intent.target,
    phase,
    stageIndex: Math.max(0, Math.min(HANDOFF_STAGES.length - 1, stageIndex)),
    completedStages: failed ? Math.max(0, stageIndex) : done ? HANDOFF_STAGES.length : stageIndex,
    startedAt: intent.startedAt,
    failed,
    done,
    stepsDone: Math.min(STEP_COUNT, stepsDone),
  }
}

/** ready/failed 归到最后一段（导轨不新增节点）。 */
function stageOf(phase: HandoffPhase): HandoffStage {
  if (phase === "ready" || phase === "failed") return "entering"
  return phase
}

/** 计时文案（纯函数）：<10s 保留一位小数，≥10s 用 m:ss，跨文档连续。 */
export function formatElapsed(ms: number): string {
  const safe = Math.max(0, ms)
  if (safe < 10_000) return `${(safe / 1000).toFixed(1)}s`
  const total = Math.floor(safe / 1000)
  const m = Math.floor(total / 60)
  const s = total % 60
  return `${m}:${String(s).padStart(2, "0")}`
}
