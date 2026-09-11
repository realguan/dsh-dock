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
import type { HandoffIntent, HandoffSnapshot } from "@/types/ipc"

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

/** 追加采样并按 6s 窗裁剪（纯函数，供 bootProgress.test.ts 直接测）。 */
export function pushSample(
  samples: SpeedSample[],
  nowMs: number,
  bytes: number,
  windowMs: number = SPEED_WINDOW_MS,
): SpeedSample[] {
  samples.push({ t: nowMs, bytes })
  while (samples.length > 0 && nowMs - samples[0].t > windowMs) samples.shift()
  return samples
}

/** 窗口内平均速度 bytes/s；样本不足或时间零增量返回 null。 */
export function computeSpeed(samples: SpeedSample[]): number | null {
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
  /**
   * 交接意图（ADR-0014）：一次重启/切换的贯穿状态。两个窗口各自持有，
   * 但**内容同源**——控制中心由 `switchProfile` 的返回值播种，主窗口在
   * 整文档重载后经 `get_boot_status` 补水；`startedAt` 相同故计时连续。
   */
  intent: (HandoffIntent & { active?: boolean }) | null

  setStep: (e: BootStepEvent) => void
  setProgress: (p: BootProgressEvent) => void
  setError: (e: BootErrorEvent) => void
  setVersions: (v: VersionsSnapshot) => void
  /** 播种/更新交接意图（null = 清除，导轨随之收起） */
  setIntent: (intent: HandoffSnapshot | HandoffIntent | null) => void
  clearError: () => void
  /**
   * 开新一轮启动：清掉上一轮遗留的错误与进度（**权威清零点**，2026-09-11 v1.2.0 实测 2.1）。
   *
   * 存在的理由：`api.chooseMode` 是**原地**调用（Rust `commands/boot.rs::choose_mode`
   * 只 spawn 启动线程，不 navigate）⇒ SPA document 不重载、store 实例存活，
   * 于是上一轮的错误会挂在新会话上（截图 ②：切 WSL 成功后诊断卡仍显示本机模式的
   * `os error 5`）。托盘 `switch_mode` / 重启交接路径会 `window.navigate`，
   * store 随之重建而**自愈**——这正是该缺陷只在原地路径可见的原因。
   *
   * 与 `reset()` 的区别：`reset` 连**步骤视图**一起清（用于整轮重来）；
   * 本方法只清「属于上一轮、且在新一轮必然失效」的跨轮状态（error / progress），
   * 保留交接意图（intent）与步骤推进，避免新轮的早期步进被抹掉。
   */
  beginNewRound: () => void
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
  intent: null,

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
      pushSample(prevSamples, Date.now(), p.current)
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
  setIntent: (intent) => set({ intent }),
  clearError: () => set({ error: null }),

  beginNewRound: () =>
    set(() => {
      // 速度采样缓冲同属上一轮：清了才不会把两轮字节数算进同一速度窗口。
      prevSamples.length = 0
      return { error: null, progress: null }
    }),
  reset: () =>
    set(() => {
      prevSamples.length = 0
      return {
        steps: EMPTY_STEPS.map((s) => ({ ...s })),
        activeStep: -1,
        progress: null,
        error: null,
        versions: null,
        // 意图不随 reset 清空：reset 是「开始新一轮启动」的语义（旧步骤作废），
        // 而意图正是这一轮的贯穿标识——清掉它导轨就断了。
      }
    }),
}))
