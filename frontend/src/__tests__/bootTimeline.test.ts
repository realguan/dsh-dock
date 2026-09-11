// 启动屏时间线可见性/收起判据（2026-09-10 建立；v1.2.0 实测 1.2 修订 2026-09-11）。
//
// 本文件的两次回归锚：
//   · v1.1.0 实测 1.1b：WSL 客体引导只发 boot:step、从不发 boot:progress，
//     `hasEverDownloaded` 恒假 → 用户点开详情后再无收起入口；
//   · v1.2.0 实测 1.2：`hasError` 一票否决收起，而 ErrorCard 在时间线之外独立
//     渲染（BootIndex.tsx:279）→ 否决无正当性，错误出现后就收不回。
import { describe, expect, it } from "vitest"
import {
  shouldOfferTimelineCollapse,
  shouldShowTimeline,
  type TimelineIntent,
  type TimelineVisibility,
} from "@/lib/bootTimeline"

/** 日常秒启：什么都没发生、用户没表态。 */
const idle: TimelineVisibility = {
  hasError: false,
  hasEverDownloaded: false,
  inDownload: false,
  userIntent: "auto",
}

describe("shouldShowTimeline", () => {
  it("日常秒启不展开向导（走极简启动屏）", () => {
    expect(shouldShowTimeline(idle)).toBe(false)
  })

  it("下载中 / 下载过 / 出错 / 用户主动要求 → 展开", () => {
    expect(shouldShowTimeline({ ...idle, inDownload: true })).toBe(true)
    expect(shouldShowTimeline({ ...idle, hasEverDownloaded: true })).toBe(true)
    expect(shouldShowTimeline({ ...idle, hasError: true })).toBe(true)
    expect(shouldShowTimeline({ ...idle, userIntent: "expanded" })).toBe(true)
  })

  it("显式收起压过一切自动信号（否则收起按钮是死按钮）", () => {
    for (const auto of [
      { hasError: true },
      { hasEverDownloaded: true },
      { inDownload: true },
      { hasError: true, hasEverDownloaded: true, inDownload: true },
    ]) {
      expect(
        shouldShowTimeline({ ...idle, ...auto, userIntent: "collapsed" }),
        `collapsed 应隐藏向导：${JSON.stringify(auto)}`,
      ).toBe(false)
    }
  })

  it("显式展开压过缺失的自动信号（v1.1.0 1.1b 回归锚）", () => {
    expect(shouldShowTimeline({ ...idle, userIntent: "expanded" })).toBe(true)
  })
})

describe("shouldOfferTimelineCollapse", () => {
  /** v1.1.0 1.1b 回归锚：WSL 客体引导只发 boot:step，hasEverDownloaded 恒假。 */
  it("用户主动点开详情时必须能收起（即使从未收到下载进度）", () => {
    expect(
      shouldOfferTimelineCollapse({ ...idle, userIntent: "expanded" }),
    ).toBe(true)
  })

  /**
   * **v1.2.0 实测 1.2 回归锚（本组的核心）**：
   * 先点「查看启动详情」，随后才发生错误 —— 向导可见且是用户自己开的，
   * 旧实现被 `hasError` 一票否决，页面上没有任何收起出口。
   */
  it("有错误 + 用户主动展开过 → 仍必须能收起（ErrorCard 在时间线之外，不会藏掉错误）", () => {
    expect(
      shouldOfferTimelineCollapse({
        ...idle,
        hasError: true,
        userIntent: "expanded",
      }),
    ).toBe(true)
  })

  it("有错误 + 自动展开（用户没表态）→ 也给出收起出口", () => {
    expect(
      shouldOfferTimelineCollapse({ ...idle, hasError: true }),
    ).toBe(true)
  })

  it("下载过而展开的，可收起", () => {
    expect(
      shouldOfferTimelineCollapse({ ...idle, hasEverDownloaded: true }),
    ).toBe(true)
  })

  it("已收起 / 未展开 → 没有收起出口", () => {
    expect(shouldOfferTimelineCollapse(idle)).toBe(false)
    expect(
      shouldOfferTimelineCollapse({ ...idle, userIntent: "collapsed" }),
    ).toBe(false)
  })

  /**
   * 真值表断言（**非 vacuous**）：逐格写死期望值，而不是拿两个函数互相推导。
   * 说明：在新判据下 `offer ⇒ show` 属于构造性成立，若只写
   * `if (offer(c)) expect(show(c)).toBe(true)` 就是在断言恒真命题（装饰性断言）。
   * 故此处改为**穷举 4×3 = 12 格的显式期望表**，任何一格算错都会红。
   */
  it("真值表：4 种自动信号 × 3 种用户意图 的 (show, offer)", () => {
    const autoCases: Array<Omit<TimelineVisibility, "userIntent">> = [
      { hasError: false, hasEverDownloaded: false, inDownload: false },
      { hasError: true, hasEverDownloaded: false, inDownload: false },
      { hasError: false, hasEverDownloaded: true, inDownload: false },
      { hasError: false, hasEverDownloaded: false, inDownload: true },
    ]
    /** [show, offer]；collapsed 恒为 [false,false]，auto 由自动信号决定，expanded 恒 [true,true]。 */
    const expectedAuto: Array<[boolean, boolean]> = [
      [false, false], // 无信号
      [true, true], // 仅错误
      [true, true], // 仅下载过
      [true, true], // 仅下载中
    ]
    autoCases.forEach((auto, i) => {
      for (const userIntent of ["auto", "expanded", "collapsed"] as TimelineIntent[]) {
        const v: TimelineVisibility = { ...auto, userIntent }
        const want: [boolean, boolean] =
          userIntent === "collapsed"
            ? [false, false]
            : userIntent === "expanded"
              ? [true, true]
              : expectedAuto[i]
        expect(
          [shouldShowTimeline(v), shouldOfferTimelineCollapse(v)],
          `auto=${JSON.stringify(auto)} intent=${userIntent}`,
        ).toEqual(want)
      }
    })
  })

  it("不变量：给出收起出口 ⇒ 当前确实在展开态（构造性成立，防未来回归打破）", () => {
    for (const userIntent of ["auto", "expanded", "collapsed"] as TimelineIntent[]) {
      for (const hasError of [false, true]) {
        for (const hasEverDownloaded of [false, true]) {
          const v: TimelineVisibility = {
            hasError,
            hasEverDownloaded,
            inDownload: false,
            userIntent,
          }
          if (shouldOfferTimelineCollapse(v)) {
            expect(shouldShowTimeline(v)).toBe(true)
          }
        }
      }
    }
  })
})
