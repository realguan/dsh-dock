// marketCopy.test.ts —— 市场文案数字一致性门禁（2026-09-11 fix）。
// 复现先行：修复前 zh-CN:669/670/697 与 en-US:644/645/672 共 6 处字面量
// 「2700+」与社区 Registry 实际 count 漂移，属用户可见文案失真。判据 =
// 界面上出现的数量必须来自真实数据：需要展示数量的文案走函数入参，其余
// 一律不内嵌数字（加载中态数量不可知，尤其不得谎报）。
// 纯逻辑测试：只读字典常量，不渲染 DOM、不引 RTL/jsdom（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"
import { t as zhCN } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"

/** 固定文案中禁止出现任何数字：数量由 registry 真实数据在渲染期产生。 */
const COUNT_FREE_COPY: Array<[string, string]> = [
  ["zh-CN market.subtitle", zhCN.market.subtitle],
  ["zh-CN market.searchPlaceholder", zhCN.market.searchPlaceholder],
  ["zh-CN market.loadingRegistry", zhCN.market.loadingRegistry],
  ["en-US market.subtitle", enUS.market.subtitle],
  ["en-US market.searchPlaceholder", enUS.market.searchPlaceholder],
  ["en-US market.loadingRegistry", enUS.market.loadingRegistry],
]

describe("市场文案不得写死插件数量", () => {
  it("subtitle / searchPlaceholder / loadingRegistry 均为无数字表述", () => {
    for (const [label, text] of COUNT_FREE_COPY) {
      expect(text.length, `${label} 不应为空`).toBeGreaterThan(0)
      expect(text, `${label} 内嵌了硬编码数字`).not.toMatch(/[0-9]/)
    }
  })

  it("需要展示数量的文案一律数据驱动（函数入参，不得字面量）", () => {
    expect(typeof zhCN.market.pageInfo).toBe("function")
    expect(typeof zhCN.market.totalPlugins).toBe("function")
    expect(typeof zhCN.market.categoryCount).toBe("function")
    expect(typeof enUS.market.pageInfo).toBe("function")
    expect(typeof enUS.market.totalPlugins).toBe("function")
    expect(typeof enUS.market.categoryCount).toBe("function")
  })

  it("数据驱动文案按传入的真实 count 求值", () => {
    expect(zhCN.market.pageInfo(1, 285, 3408)).toBe("第 1 / 285 页（共 3408 款）")
    expect(enUS.market.pageInfo(1, 285, 3408)).toBe("Page 1 of 285 (3408 items)")
    expect(zhCN.market.totalPlugins(3408)).toBe("3408 款插件")
    expect(enUS.market.totalPlugins(3408)).toBe("3408 Plugins")
    expect(zhCN.market.categoryCount(12)).toBe("12 个分类")
    expect(enUS.market.categoryCount(12)).toBe("12 Categories")
  })

  it("zh-CN 与 en-US 术语一致：品牌与 Registry 专名保留", () => {
    expect(zhCN.market.subtitle).toContain("awesome-dsh-plugin")
    expect(enUS.market.subtitle).toContain("awesome-dsh-plugin")
    expect(zhCN.market.loadingRegistry).toMatch(/registry/i)
    expect(enUS.market.loadingRegistry).toMatch(/registry/i)
  })
})
