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
  it("下载链 checking→downloading→installing→done→relaunching（**当前**真实序；回放见 seam 文件）", () => {
    // 2026-09-11（task-53）订正：本条原先断言 `installing → downloading`
    // （即「Installing 在下载之前」）——那是 Rust 的**旧序**，已在 D6r 中改为
    // 「下载完成才发 Installing」，且 `reverse_transition_is_rejected` 明文拒绝该边。
    // 断言改为与现序一致。
    expect(TRANSITIONS.checking).toContain("downloading")
    expect(TRANSITIONS.downloading).toContain("installing")
    expect(TRANSITIONS.downloading).toContain("done")
    expect(TRANSITIONS.done).toContain("relaunching")
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

/**
 * 本表自身的**接线级**断言（顺序无关的那些边）。
 *
 * ⚠️ **真实发射序的回放已移到 `__tests__/clientUpdateSeam.test.ts`**（2026-09-11，task-53）：
 * 本组原先内联了一份「真实序」，但那份序**随 Rust 演进而过时**（`Installing` 从
 * 下载前移到了下载后、前置可见态改为 `Checking`），而它与 seam 文件各存一份 =
 * 同一事实两个副本 ⇒ **正是两侧漂移的温床**（task-53 的破口就是这么来的）。
 * 现约定：**发射序只在 `clientUpdateSeam.test.ts` 维护一处**；本文件只测
 * 「表的自洽性与守卫边界」，不重复声明任何序。
 */
describe("表接线（顺序无关的边；发射序回放见 clientUpdateSeam.test.ts）", () => {
  it("下载链的每条相邻边都在表内（按**当前** Rust 序：checking→downloading→installing→done→relaunching）", () => {
    expect(TRANSITIONS.checking).toContain("downloading")
    expect(TRANSITIONS.downloading).toContain("installing")
    expect(TRANSITIONS.installing).toContain("done")
    expect(TRANSITIONS.done).toContain("relaunching")
  })

  it("点下载后立即出现的可见态（Checking）不得被丢弃（无 loading 的直接成因）", () => {
    // 当前 Rust 在 blocked_check 之前发的是 `Checking`（非 Installing）：
    // `updater.rs:243`「进入动作即发的可见态」
    expect(
      applyUpdateEvent({ phase: "available" }, { phase: "checking" }),
      "available → checking 是「点下载立刻有反馈」的可见态，丢了就回到『点了没反应』",
    ).not.toBeNull()
  })

  it("下载完成 → Done 不得被丢弃", () => {
    expect(
      applyUpdateEvent({ phase: "downloading" }, { phase: "done", version: "1.2.1" }),
    ).not.toBeNull()
  })

  it("Done → Relaunching 不得被丢弃（原表缺此边）", () => {
    expect(
      applyUpdateEvent({ phase: "done" }, { phase: "relaunching" }),
    ).not.toBeNull()
  })

  it("下载先于 Installing 的两种进入方式都放行（available / checking 起手）", () => {
    expect(
      applyUpdateEvent({ phase: "available" }, { phase: "downloading", current: 1 }),
    ).not.toBeNull()
    expect(
      applyUpdateEvent({ phase: "checking" }, { phase: "downloading", current: 1 }),
    ).not.toBeNull()
  })

  it("installing → downloading 是**倒退**，必须拒绝（与 Rust reverse_transition_is_rejected 同判据）", () => {
    // T-F（task-46）曾为"T-D6r 可能改序"双向放行；现序已固定（Windows 下载完成才发
    // Installing），故按契约收紧。若 Rust 侧将来真的改序，seam 测试会先行报警。
    expect(
      applyUpdateEvent({ phase: "installing" }, { phase: "downloading", current: 1 }),
    ).toBeNull()
  })

  it("失败可从任一进行中态到达", () => {
    for (const phase of ["installing", "downloading", "done"] as const) {
      expect(
        applyUpdateEvent({ phase }, { phase: "failed", message: "x" }),
        `${phase} → failed`,
      ).not.toBeNull()
    }
  })

  it("仍然拒绝真正无意义的跳转（守卫未被放宽成万能）", () => {
    expect(applyUpdateEvent({ phase: "idle" }, { phase: "done", version: "1" })).toBeNull()
    expect(applyUpdateEvent({ phase: "idle" }, { phase: "downloading", current: 1 })).toBeNull()
    expect(
      applyUpdateEvent({ phase: "available" }, { phase: "relaunching" }),
    ).toBeNull()
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
