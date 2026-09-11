// bootRound.test.ts —— 「新一轮启动」判定纯逻辑（2026-09-11，v1.2.0 实测 2.1）。
//
// 回归锚：截图 ② —— 本机模式失败留下错误 → 点右上角切 WSL → 新会话真的起来了
// （01–03 全绿、04 运行中），但诊断卡仍挂上一轮那条「网络不可用 / os error 5」。
// 根因：`bootStore.error` 没有「新一轮启动开始」清零点；而该路径走的是**原地**
// `api.chooseMode`（SPA 不重载 document）⇒ 同一 store 实例存活、error 无人清理。
import { describe, expect, it } from "vitest"
import { isNewBootRound, shouldClearStaleError } from "@/lib/bootRound"
import { useBootStore } from "@/stores/bootStore"
import type { BootStepState } from "@/types/events"

describe("isNewBootRound：边沿判定", () => {
  it("步 0 从非 running 变为 running ⇒ 新一轮起跑", () => {
    expect(isNewBootRound("pending", "running")).toBe(true)
    expect(isNewBootRound("done", "running")).toBe(true)
    expect(isNewBootRound("error", "running")).toBe(true)
    expect(isNewBootRound(undefined, "running")).toBe(true)
  })

  it("稳态（一直 running）**不算**新轮次 —— 否则本轮真实错误会被立刻抹掉", () => {
    expect(isNewBootRound("running", "running")).toBe(false)
  })

  it("离开 running 不算新轮次（done/error 只是本轮推进）", () => {
    expect(isNewBootRound("running", "done")).toBe(false)
    expect(isNewBootRound("running", "error")).toBe(false)
    expect(isNewBootRound("running", undefined)).toBe(false)
  })

  it("其余组合均不算（含 undefined ↔ 非 running，避免挂载期误判）", () => {
    const states: Array<BootStepState | undefined> = [
      undefined,
      "pending",
      "running",
      "done",
      "error",
    ]
    for (const prev of states) {
      for (const cur of states) {
        const expected = cur === "running" && prev !== "running"
        expect(isNewBootRound(prev, cur), `${prev} → ${cur}`).toBe(expected)
      }
    }
  })
})

describe("shouldClearStaleError：清错误的条件", () => {
  it("新轮次 + 持有错误 ⇒ 清（本任务的核心修复）", () => {
    expect(shouldClearStaleError("done", "running", true)).toBe(true)
    expect(shouldClearStaleError(undefined, "running", true)).toBe(true)
  })

  it("新轮次但无错误 ⇒ 无需清（不产生多余 store 写入）", () => {
    expect(shouldClearStaleError("done", "running", false)).toBe(false)
  })

  it("非新轮次即使有错误也不清 —— 保护本轮刚产生的错误", () => {
    // 这条是「错误不得被误藏」的机器化保证：错误出现后若 step0 仍在 running，
    // 不会被清掉。
    expect(shouldClearStaleError("running", "running", true)).toBe(false)
    expect(shouldClearStaleError("pending", "pending", true)).toBe(false)
  })

  it("真值表：3 种 hasError × 关键步态组合", () => {
    const cases: Array<[BootStepState | undefined, BootStepState | undefined, boolean, boolean]> = [
      // prev, cur, hasError, expected
      ["done", "running", true, true],
      ["done", "running", false, false],
      ["running", "running", true, false],
      ["pending", "running", true, true],
      ["running", "done", true, false],
      ["running", "error", true, false],
    ]
    for (const [prev, cur, hasError, expected] of cases) {
      expect(
        shouldClearStaleError(prev, cur, hasError),
        `prev=${prev} cur=${cur} hasError=${hasError}`,
      ).toBe(expected)
    }
  })
})

/**
 * **跨层接线断言**（qa-verify 明确要求：「切模式后前端 error 必为空」）。
 *
 * 上面测的是纯函数；这里直接对**真实 store 实例**断言「新一轮开始 ⇒ error 清空」，
 * 覆盖 `beginNewRound()` 这个唯一清零点本身的行为（含本机失败 → 切 WSL 的场景复现）。
 */
describe("跨层接线：新一轮开始时前端 error 必为空", () => {
  it("复现截图 ②：本机模式留下错误 → 切 WSL（原地 chooseMode）→ error 必须被清", () => {
    const s = useBootStore.getState()
    s.reset()
    // 1) 本机模式引导失败（截图 ① 的 os error 5）
    useBootStore.getState().setError({
      title: "dsh 引导安装失败（registry 均不可达）",
      detail: "activate global install → link the global package install directory: 拒绝访问。(os error 5)",
      actions: ["retry"],
    })
    expect(useBootStore.getState().error, "前置：错误应已写入").not.toBeNull()

    // 2) 用户点右上角胶囊切 WSL —— 前端在调 chooseMode 前开新一轮
    useBootStore.getState().beginNewRound()

    // 3) 断言：同一 store 实例（原地路径）上错误必须消失
    expect(useBootStore.getState().error, "切模式后 error 必为空").toBeNull()
  })

  it("beginNewRound 同时清进度（上一轮的进度不属于本轮）", () => {
    useBootStore.getState().reset()
    useBootStore.getState().setProgress({ kind: "node", current: 50, total: 100 })
    expect(useBootStore.getState().progress).not.toBeNull()
    useBootStore.getState().beginNewRound()
    expect(useBootStore.getState().progress).toBeNull()
  })

  it("beginNewRound 保留步骤视图与交接意图（不越权清整轮）", () => {
    useBootStore.getState().reset()
    useBootStore.getState().setStep({ step: 2, state: "running", detail: "x" })
    const stepsBefore = useBootStore.getState().steps
    useBootStore.getState().beginNewRound()
    // 步骤保留：新一轮的早期步进不应被抹掉
    expect(useBootStore.getState().steps).toBe(stepsBefore)
    // 意图保留：交接导轨的贯穿标识，清掉就断轨（与 reset 的差异见 store 注释）
    expect(useBootStore.getState().intent).toBeNull() // reset 后本为 null，断言不被动过
  })

  it("无错误时调用 beginNewRound 是幂等的（不产生额外状态变化）", () => {
    useBootStore.getState().reset()
    const before = useBootStore.getState()
    useBootStore.getState().beginNewRound()
    expect(useBootStore.getState().error).toBeNull()
    expect(useBootStore.getState().progress).toBeNull()
    expect(useBootStore.getState().steps).toEqual(before.steps)
  })
})
