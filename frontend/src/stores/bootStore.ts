// 启动状态（BootIndex / BootSelector 消费）。事件驱动：boot:step /
// boot:progress / boot:error / boot:update 经 lib/events.ts 写入。
// 整页重载（Rust location.assign / navigate）后本 store 全新创建——
// 页面进入时自行经 resource.updateStatus() 等重新播种（§3.1 生命周期）。
import { create } from "zustand"
import type {
  BootErrorEvent,
  BootProgressEvent,
  BootStepEvent,
  BootStepState,
  VersionsSnapshot,
} from "@/types/events"

export const STEP_COUNT = 5

export interface StepView {
  status: BootStepState
  detail?: string
}

interface SpeedSample {
  t: number
  bytes: number
}

/// 滑动窗口速度采样窗宽：6s 前的样本丢弃；不足 2 个样本不出速度。
const SPEED_WINDOW_MS = 6000

function computeSpeed(samples: SpeedSample[]): number | null {
  if (samples.length < 2) return null
  const first = samples[0]
  const last = samples[samples.length - 1]
  const dt = last.t - first.t
  if (dt <= 0) return null
  return (last.bytes - first.bytes) / (dt / 1000)
}

export interface DownloadProgressState {
  kind: string
  current: number
  total: number | null
  speed: number | null
  eta: number | null
}

interface BootState {
  /** 五步视图（index 对齐 content 的 steps 三元组） */
  steps: StepView[]
  /** 当前正在跑的步骤号；无运行中步骤为 -1 */
  activeStep: number
  progress: DownloadProgressState | null
  error: BootErrorEvent | null
  versions: VersionsSnapshot | null

  setStep: (e: BootStepEvent) => void
  setProgress: (p: BootProgressEvent) => void
  setError: (e: BootErrorEvent) => void
  setVersions: (v: VersionsSnapshot) => void
  clearError: () => void
  reset: () => void
}

const EMPTY_STEPS: StepView[] = Array.from({ length: STEP_COUNT }, () => ({
  status: "pending" as const,
}))

/// 速度采样缓冲（store 外的模块级可变数组：非渲染态，不进 store 以免
/// 高频采样触发订阅者抖动；reset 清空）。
const prevSamples: SpeedSample[] = []

export const useBootStore = create<BootState>((set) => ({
  steps: EMPTY_STEPS.map((s) => ({ ...s })),
  activeStep: -1,
  progress: null,
  error: null,
  versions: null,

  setStep: (e) =>
    set((st) => {
      if (e.step < 0 || e.step >= STEP_COUNT) return st
      const steps = st.steps.map((s, i): StepView => {
        if (i < e.step && s.status === "pending") return { ...s, status: "done" }
        if (i === e.step) return { ...s, status: e.state, detail: e.detail || undefined }
        return s
      })
      const active =
        e.state === "running"
          ? e.step
          : st.activeStep === e.step
            ? -1
            : st.activeStep
      return { steps, activeStep: active }
    }),

  setProgress: (p) =>
    set(() => {
      // 进度可能针对不同 kind（目前只有 node）；窗口内直接覆盖，
      // 完成帧（current >= total）之后的新一轮下载靠 reset/重发自然续上。
      const now = Date.now()
      prevSamples.push({ t: now, bytes: p.current })
      while (
        prevSamples.length > 0 &&
        now - prevSamples[0].t > SPEED_WINDOW_MS
      ) {
        prevSamples.shift()
      }
      const speed = computeSpeed(prevSamples)
      const eta =
        p.total !== null && speed !== null && speed > 0
          ? Math.max(0, (p.total - p.current) / speed)
          : null
      return {
        progress: { kind: p.kind, current: p.current, total: p.total, speed, eta },
      }
    }),

  setError: (e) => set({ error: e }),
  setVersions: (v) => set({ versions: v }),
  clearError: () => set({ error: null }),
  reset: () =>
    set(() => {
      prevSamples.length = 0
      return {
        steps: EMPTY_STEPS.map((s) => ({ ...s })),
        activeStep: -1,
        progress: null,
        error: null,
        versions: null,
      }
    }),
}))
