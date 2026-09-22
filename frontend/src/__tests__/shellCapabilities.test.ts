import { describe, expect, it } from "vitest"
import { CAPABILITIES_FALLBACK, shouldShowAboutEntry } from "@/lib/shellCapabilities"

describe("shouldShowAboutEntry（§3.6 纯判据）", () => {
  it("常驻入口不可达 ⇒ 补窗口内入口（这正是 2026-08-27 删除 open_about 的失效场景）", () => {
    expect(shouldShowAboutEntry({ residentEntryAvailable: false })).toBe(true)
  })

  it("常驻入口可达 ⇒ 不补（避免重现当年被判重复的那个按钮）", () => {
    expect(shouldShowAboutEntry({ residentEntryAvailable: true })).toBe(false)
  })

  it("未知（尚未取到）⇒ 补：漏补是功能缺失，多补只是观感瑕疵", () => {
    expect(shouldShowAboutEntry(null)).toBe(true)
    expect(shouldShowAboutEntry(CAPABILITIES_FALLBACK)).toBe(true)
  })

  it("反例守卫：CAPABILITIES_FALLBACK 必须偏向「补入口」——改成 true 会让拉取失败的桌面失去唯一入口", () => {
    expect(CAPABILITIES_FALLBACK.residentEntryAvailable).toBe(false)
  })
})
