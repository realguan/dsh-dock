// installFlight.test.ts —— 「点击安装 → 下载管理」飞行动画几何（2026-09-09，
// 问题记录-2026-09-09 §1.1）。动画本身靠目视，可测的是「飞哪、飞多久、弧线多高」。
import { describe, expect, it } from "vitest"
import { flightTrack } from "@/lib/installFlight"

describe("飞行动画轨道", () => {
  it("终点精确落在锚点中心，中段向上抬升成弧线", () => {
    const t = flightTrack({ x: 100, y: 600 }, { x: 900, y: 80 })
    expect(t.x).toEqual([0, 400, 800])
    expect(t.x[2]).toBe(900 - 100)
    expect(t.y[2]).toBe(80 - 600)
    // 中段比两点连线更靠上（y 更小 = 更靠屏幕上方）
    const straightMid = (0 + (80 - 600)) / 2
    expect(t.y[1]).toBeLessThan(straightMid)
  })

  it("抬升量钳制在 18~96 之间（近不贴地、远不飞出屏）", () => {
    const near = flightTrack({ x: 0, y: 0 }, { x: 10, y: 0 })
    const far = flightTrack({ x: 0, y: 0 }, { x: 4000, y: 0 })
    expect(near.y[1]).toBe(-18)
    expect(far.y[1]).toBe(-96)
  })

  it("时长随距离增长且钳制在 0.38~0.9 秒", () => {
    const near = flightTrack({ x: 0, y: 0 }, { x: 1, y: 0 })
    const mid = flightTrack({ x: 0, y: 0 }, { x: 600, y: 0 })
    const far = flightTrack({ x: 0, y: 0 }, { x: 9000, y: 0 })
    expect(near.duration).toBe(0.38)
    expect(far.duration).toBe(0.9)
    expect(mid.duration).toBeGreaterThan(near.duration)
    expect(mid.duration).toBeLessThan(far.duration)
  })

  it("距离为 0（点击处就是角标）退化为原地弹跳，不做无意义位移", () => {
    const t = flightTrack({ x: 42, y: 42 }, { x: 42, y: 42 })
    expect(t.x).toEqual([0, 0, 0])
    expect(t.y).toEqual([0, 0, 0])
    expect(t.duration).toBe(0.38)
  })

  it("关键帧时间轴与坐标等长（framer-motion 的 times 契约）", () => {
    const t = flightTrack({ x: 10, y: 20 }, { x: 300, y: 400 })
    expect(t.times).toEqual([0, 0.55, 1])
    expect(t.times).toHaveLength(t.x.length)
    expect(t.times).toHaveLength(t.y.length)
  })
})
