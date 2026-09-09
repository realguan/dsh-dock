// lib/installFlight.ts —— 「点击安装 → 下载管理」飞行动画的纯几何 + 锚点登记
// （2026-09-09，问题记录-2026-09-09 §1.1）。
//
// 复现的缺陷：市场点「安装」→ 确认 → 弹窗消失，只在底部弹一条 toast、右上角
// 角标悄悄 +1，新用户完全不知道东西去哪了。现在从点击处飞一颗胶囊到「下载管理」
// 按钮，落点角标弹跳——本模块只算「飞哪、飞多久、弧线多高」，DOM 读写（锚点
// 元素登记）与动画渲染分别归 stores/installFlightStore 与 components/InstallFlight。
//
// 几何是纯函数（Vitest 直测）；锚点读写是 DOM 薄封装，只在浏览器里被调用。
export interface FlightPoint {
  x: number
  y: number
}

export interface FlightTrack {
  /** x 轴关键帧（起点 0 → 终点 dx） */
  x: number[]
  /** y 轴关键帧（中段抬升成弧线，避免直线贴地飞过整个列表） */
  y: number[]
  /** 关键帧时间轴，与 x/y 等长 */
  times: number[]
  /** 动画时长（秒） */
  duration: number
}

/** 飞行时长下限/上限（秒）：太短看不见，太长拖沓。 */
const MIN_DURATION = 0.38
const MAX_DURATION = 0.9
/** 基础时长 + 每像素增量：近距离 ~0.45s，跨屏 ~0.8s。 */
const BASE_DURATION = 0.34
const DURATION_PER_PX = 0.0004
/** 弧线抬升量 = 距离 × 比例，钳制在 [MIN_LIFT, MAX_LIFT]。 */
const LIFT_RATIO = 0.12
const MIN_LIFT = 18
const MAX_LIFT = 96
/** 中段关键帧的时间位置：略早于中点，视觉上「先冲出去再落下来」。 */
const MID_TIME = 0.55

function clamp(v: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, v))
}

/**
 * 飞行轨道：起点 → 终点，中段向上抬升成弧线。
 * 距离为 0（点击处就是角标）时退化为原地一次轻微弹跳，不做无意义的横向位移。
 */
export function flightTrack(from: FlightPoint, to: FlightPoint): FlightTrack {
  const dx = to.x - from.x
  const dy = to.y - from.y
  const distance = Math.hypot(dx, dy)
  const duration = clamp(BASE_DURATION + distance * DURATION_PER_PX, MIN_DURATION, MAX_DURATION)
  if (distance === 0) {
    return { x: [0, 0, 0], y: [0, 0, 0], times: [0, MID_TIME, 1], duration: MIN_DURATION }
  }
  const lift = clamp(distance * LIFT_RATIO, MIN_LIFT, MAX_LIFT)
  return {
    x: [0, dx * 0.5, dx],
    y: [0, dy * 0.5 - lift, dy],
    times: [0, MID_TIME, 1],
    duration,
  }
}

// ---------- 锚点登记（「下载管理」触发按钮） ----------

let queueAnchor: HTMLElement | null = null

/** 由 QueuePanel 挂载时登记触发按钮；卸载时传 null（HMR / 切 Tab 都会走）。 */
export function setQueueAnchor(el: HTMLElement | null): void {
  queueAnchor = el
}

/** 触发按钮中心的视口坐标；未登记或已卸载返回 null（调用方退化为不飞）。 */
export function queueAnchorCenter(): FlightPoint | null {
  const el = queueAnchor
  if (!el || !el.isConnected) return null
  const rect = el.getBoundingClientRect()
  return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 }
}
