// 交接模型测试（ADR-0014）：一次重启/切换的贯穿状态折算。
//
// 这套断言钉住三件容易在重构中悄悄坏掉的事：
//   1. **只前进不后退**——Rust 阶段与 boot:step 事件流取更靠前者（否则导轨抖动）；
//   2. **计时连续**——起点只认 intent.startedAt（跨文档替换不归零）；
//   3. **失败诚实**——error 步骤/失败阶段不得被算成"还在顺利推进"。
import { describe, expect, it } from "vitest"
import {
  HANDOFF_STAGES,
  deriveHandoff,
  formatElapsed,
  phaseFromSteps,
  type HandoffView,
} from "@/lib/handoff"
import type { HandoffPhase, HandoffSnapshot } from "@/types/ipc"

type StepStatus = "pending" | "running" | "done" | "error"

function steps(...statuses: StepStatus[]): { status: string }[] {
  const out: { status: string }[] = Array.from({ length: 5 }, () => ({ status: "pending" }))
  statuses.forEach((s, i) => {
    out[i] = { status: s }
  })
  return out
}

function snapshot(
  phase: HandoffPhase,
  over: Partial<HandoffSnapshot> = {},
): HandoffSnapshot {
  return {
    target: "web",
    kind: "restart",
    phase,
    startedAt: 1_000_000,
    generation: 3,
    active: true,
    ...over,
  }
}

function view(intent: HandoffSnapshot | null, st: { status: string }[]): HandoffView {
  const v = deriveHandoff(intent, st)
  if (!v) throw new Error("期望得到交接视图")
  return v
}

describe("handoff 阶段折算", () => {
  it("无意图 → 无视图（普通冷启动不渲染导轨）", () => {
    expect(deriveHandoff(null, steps())).toBeNull()
  })

  it("步骤事件流推出的阶段：boot → waiting → entering → ready", () => {
    expect(phaseFromSteps(steps("done", "done", "running"))).toBe("booting")
    expect(phaseFromSteps(steps("done", "done", "done", "running"))).toBe("waiting")
    expect(phaseFromSteps(steps("done", "done", "done", "done", "running"))).toBe("entering")
    expect(phaseFromSteps(steps("done", "done", "done", "done", "done"))).toBe("ready")
  })

  it("error 步骤不参与阶段推进（失败由错误卡讲述）", () => {
    expect(phaseFromSteps(steps("done", "done", "error"))).toBeNull()
  })

  it("Rust 阶段与事件流取更靠前者：事件先到不回退", () => {
    // 壳刚发 step4 running（进入工作台），而 Rust 阶段还在 waiting
    const v = view(snapshot("waiting"), steps("done", "done", "done", "done", "running"))
    expect(v.phase).toBe("entering")
    // Rust 已推进到 entering，事件流还停在 step2：不得被拉回 booting
    const v2 = view(snapshot("entering"), steps("done", "running"))
    expect(v2.phase).toBe("entering")
  })

  it("阶段 → 四段导轨下标；ready 时四段全完成", () => {
    expect(view(snapshot("stopping"), steps()).stageIndex).toBe(0)
    expect(view(snapshot("booting"), steps()).stageIndex).toBe(1)
    expect(view(snapshot("waiting"), steps()).stageIndex).toBe(2)
    expect(view(snapshot("entering"), steps()).stageIndex).toBe(3)
    const done = view(snapshot("ready"), steps("done", "done", "done", "done", "done"))
    expect(done.done).toBe(true)
    expect(done.completedStages).toBe(HANDOFF_STAGES.length)
  })

  it("失败：不算完成、保留已走过的段、携带失败标记", () => {
    const v = view(snapshot("failed", { active: false }), steps("done", "done", "running"))
    expect(v.failed).toBe(true)
    expect(v.done).toBe(false)
    expect(v.completedStages).toBeLessThan(HANDOFF_STAGES.length)
  })

  it("计时起点只认 startedAt（主窗口整文档重载后不归零）", () => {
    const v = view(snapshot("waiting", { startedAt: 500 }), steps("done", "done", "done", "running"))
    expect(v.startedAt).toBe(500)
  })

  it("五步完成数供启动屏分段进度使用", () => {
    const v = view(snapshot("booting"), steps("done", "done", "running"))
    expect(v.stepsDone).toBe(2)
  })

  it("active 透传（Rust 裁决 TTL；过期意图由 UI 收起）", () => {
    expect(view(snapshot("waiting", { active: true }), steps()).inflight).toBe(true)
    expect(view(snapshot("ready", { active: false }), steps()).inflight).toBe(false)
  })

  it("kind 透传（启动/重启/切换文案分叉的数据源）", () => {
    expect(view(snapshot("stopping", { kind: "switch" }), steps()).kind).toBe("switch")
    expect(view(snapshot("stopping", { kind: "start" }), steps()).kind).toBe("start")
  })
})

describe("handoff 计时文案", () => {
  it("10 秒内保留一位小数", () => {
    expect(formatElapsed(0)).toBe("0.0s")
    expect(formatElapsed(1234)).toBe("1.2s")
    expect(formatElapsed(9999)).toBe("10.0s")
  })

  it("10 秒以上转 m:ss（长等不抖动位数）", () => {
    expect(formatElapsed(10_000)).toBe("0:10")
    expect(formatElapsed(65_400)).toBe("1:05")
    expect(formatElapsed(600_000)).toBe("10:00")
  })

  it("负数（时钟回拨）钳到 0", () => {
    expect(formatElapsed(-500)).toBe("0.0s")
  })
})
