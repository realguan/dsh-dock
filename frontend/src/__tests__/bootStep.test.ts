// 步骤状态推演测试（frontend-migration §8.1 bootStep）。
// 直接驱动 zustand store：setStep 的事件语义与 setStep 细节见 bootStore.ts。
import { beforeEach, describe, expect, it } from "vitest"
import { useBootStore, STEP_COUNT } from "@/stores/bootStore"

function stateOf(i: number) {
  return useBootStore.getState().steps[i].status
}

describe("bootStore.setStep 推演", () => {
  beforeEach(() => useBootStore.getState().reset())

  it("初始五步全 pending，activeStep -1", () => {
    const s = useBootStore.getState()
    expect(s.steps).toHaveLength(STEP_COUNT)
    expect(s.steps.every((x) => x.status === "pending")).toBe(true)
    expect(s.activeStep).toBe(-1)
  })

  it("running 触发前置 pending 自动置 done", () => {
    useBootStore.getState().setStep({ step: 3, state: "running", detail: "等待" })
    expect(stateOf(0)).toBe("done")
    expect(stateOf(1)).toBe("done")
    expect(stateOf(2)).toBe("done")
    expect(stateOf(3)).toBe("running")
    expect(stateOf(4)).toBe("pending")
    expect(useBootStore.getState().activeStep).toBe(3)
  })

  it("已有状态的前置步骤不被误覆盖为 done", () => {
    useBootStore.getState().setStep({ step: 1, state: "error", detail: "环境解析失败" })
    useBootStore.getState().setStep({ step: 2, state: "running", detail: "" })
    expect(stateOf(1)).toBe("error") // error 保持：k < i 且非 pending 才不动；error 不被覆盖
  })

  it("done 后 activeStep 回 -1；detail 仅在 running 步落视", () => {
    useBootStore.getState().setStep({ step: 0, state: "running", detail: "扫描 PATH" })
    expect(useBootStore.getState().activeStep).toBe(0)
    useBootStore.getState().setStep({ step: 0, state: "done", detail: "" })
    expect(useBootStore.getState().activeStep).toBe(-1)
    expect(useBootStore.getState().steps[0].detail).toBeUndefined()
  })

  it("越界 step 忽略（不崩、不改状态）", () => {
    useBootStore.getState().setStep({ step: 9, state: "running", detail: "" })
    useBootStore.getState().setStep({ step: -1, state: "running", detail: "" })
    expect(useBootStore.getState().steps.every((x) => x.status === "pending")).toBe(true)
  })

  it("reset 清空进度/错误/版本", () => {
    const st = useBootStore.getState()
    st.setProgress({ kind: "node", current: 5, total: 10 })
    st.setError({ title: "x", detail: "y" })
    st.setVersions({
      dsh: { current: "1", latest: null, newer: false, error: null },
      client: { current: "1", latest: null, newer: false, error: null },
      node: { version: "v1", origin: "system" },
    })
    st.reset()
    const after = useBootStore.getState()
    expect(after.progress).toBeNull()
    expect(after.error).toBeNull()
    expect(after.versions).toBeNull()
  })
})
