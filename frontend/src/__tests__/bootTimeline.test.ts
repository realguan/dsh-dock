// 启动屏时间线可见性/收起判据（2026-09-10，v1.1.0 Windows 实测 1.1b 回归锚）。
import { describe, expect, it } from "vitest"
import {
  shouldOfferTimelineCollapse,
  shouldShowTimeline,
  type TimelineVisibility,
} from "@/lib/bootTimeline"

/** 日常秒启：什么都没发生。 */
const idle: TimelineVisibility = {
  hasError: false,
  hasEverDownloaded: false,
  inDownload: false,
  forceShowTimeline: false,
}

describe("shouldShowTimeline", () => {
  it("日常秒启不展开向导（走极简启动屏）", () => {
    expect(shouldShowTimeline(idle)).toBe(false)
  })

  it("下载中 / 下载过 / 出错 / 用户主动要求 → 展开", () => {
    expect(shouldShowTimeline({ ...idle, inDownload: true })).toBe(true)
    expect(shouldShowTimeline({ ...idle, hasEverDownloaded: true })).toBe(true)
    expect(shouldShowTimeline({ ...idle, hasError: true })).toBe(true)
    expect(shouldShowTimeline({ ...idle, forceShowTimeline: true })).toBe(true)
  })
})

describe("shouldOfferTimelineCollapse", () => {
  /**
   * **本组的核心回归项**：WSL 客体引导只发 boot:step、从不发 boot:progress，
   * 故 hasEverDownloaded 恒假。若收起判据只看它，用户点开详情后就再也回不去。
   */
  it("用户主动点开详情时必须能收起（即使从未收到下载进度）", () => {
    expect(
      shouldOfferTimelineCollapse({
        ...idle,
        hasEverDownloaded: false, // ← WSL 路径的真实取值
        forceShowTimeline: true,
      }),
    ).toBe(true)
  })

  it("下载过而展开的，可收起", () => {
    expect(
      shouldOfferTimelineCollapse({ ...idle, hasEverDownloaded: true }),
    ).toBe(true)
  })

  it("出错时不给收起出口（错误必须可见）", () => {
    expect(
      shouldOfferTimelineCollapse({
        ...idle,
        hasError: true,
        forceShowTimeline: true,
      }),
    ).toBe(false)
  })

  it("未展开时没有收起出口", () => {
    expect(shouldOfferTimelineCollapse(idle)).toBe(false)
  })

  it("不变量：给出收起出口 ⇒ 当前确实在展开态", () => {
    const cases: TimelineVisibility[] = [
      idle,
      { ...idle, forceShowTimeline: true },
      { ...idle, hasEverDownloaded: true },
      { ...idle, inDownload: true },
      { ...idle, hasError: true },
      { ...idle, hasError: true, forceShowTimeline: true },
      { ...idle, hasEverDownloaded: true, inDownload: true },
    ]
    for (const c of cases) {
      if (shouldOfferTimelineCollapse(c)) {
        expect(shouldShowTimeline(c)).toBe(true)
      }
    }
  })
})
