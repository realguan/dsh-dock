// 客户端更新状态机纯函数测试（frontend-migration §8.1 updatePhase）。
// 迁移表锚定 updater.rs set_state 的真实调用位点（TRANSITIONS 见 store 注释）。
import { beforeEach, describe, expect, it } from "vitest"
import {
  TRANSITIONS,
  applyUpdateEvent,
  useClientUpdateStore,
} from "@/stores/clientUpdateStore"
import type { AppUpdateEvent } from "@/types/events"

describe("TRANSITIONS 合法迁移表", () => {
  it("检查链 idle→checking→available/upToDate/failed 合法", () => {
    expect(TRANSITIONS.idle).toContain("checking")
    expect(TRANSITIONS.checking).toContain("available")
    expect(TRANSITIONS.checking).toContain("upToDate")
    expect(TRANSITIONS.checking).toContain("failed")
  })
  it("下载链 available→downloading→installing→relaunching→done", () => {
    expect(TRANSITIONS.available).toContain("downloading")
    expect(TRANSITIONS.downloading).toContain("installing")
    expect(TRANSITIONS.installing).toContain("relaunching")
    expect(TRANSITIONS.installing).toContain("done")
    expect(TRANSITIONS.relaunching).toContain("done")
  })
  it("非法迁移不在任何行（如 idle→done 一步到位）", () => {
    expect(TRANSITIONS.idle).not.toContain("done")
    expect(TRANSITIONS.checking).not.toContain("done")
  })
})

describe("applyUpdateEvent 纯函数", () => {
  it("合法迁移放行并返回事件", () => {
    const e: AppUpdateEvent = { phase: "available", latest: "0.5.0" }
    expect(applyUpdateEvent({ phase: "checking" }, e)).toEqual(e)
  })
  it("非法迁移返回 null", () => {
    expect(applyUpdateEvent({ phase: "idle" }, { phase: "done", version: "1" })).toBeNull()
  })
  it("下载进度事件（含 current/total）通过", () => {
    const e: AppUpdateEvent = { phase: "downloading", current: 10, total: 100 }
    expect(applyUpdateEvent({ phase: "available" }, e)).toEqual(e)
  })
})

describe("useClientUpdateStore dispatch/hydrate", () => {
  beforeEach(() => useClientUpdateStore.getState().reset())

  it("hydrate 播种快照（整页重载恢复）", () => {
    useClientUpdateStore.getState().hydrate({ phase: "upToDate", latest: "0.4.7" })
    expect(useClientUpdateStore.getState().snapshot).toEqual({
      phase: "upToDate",
      latest: "0.4.7",
    })
  })

  it("dispatch 非法迁移忽略（snapshot 不变）", () => {
    useClientUpdateStore.getState().hydrate({ phase: "idle" })
    useClientUpdateStore.getState().dispatch({ phase: "available", latest: "1" })
    expect(useClientUpdateStore.getState().snapshot?.phase).toBe("idle")
  })

  it("dispatch 合法迁移推进", () => {
    useClientUpdateStore.getState().hydrate({ phase: "idle" })
    useClientUpdateStore.getState().dispatch({ phase: "checking" })
    useClientUpdateStore.getState().dispatch({ phase: "upToDate", latest: "0.4.7" })
    expect(useClientUpdateStore.getState().snapshot?.phase).toBe("upToDate")
  })

  it("reset 幂等：两次 reset 后 snapshot 恒 null", () => {
    useClientUpdateStore.getState().hydrate({ phase: "done", version: "1" })
    useClientUpdateStore.getState().reset()
    useClientUpdateStore.getState().reset()
    expect(useClientUpdateStore.getState().snapshot).toBeNull()
  })
})
