// 格式化纯函数测试（frontend-migration §8.1）。
import { describe, expect, it } from "vitest"
import {
  fmtBytes,
  fmtEta,
  fmtPercent,
  fmtSpeed,
  getProfileColorClass,
} from "@/lib/format"

describe("fmtBytes", () => {
  it("字节级按整数输出", () => {
    expect(fmtBytes(512)).toBe("512 B")
  })
  it("KB/MB 单位换算与一位小数", () => {
    expect(fmtBytes(1536)).toBe("1.5 KB")
    expect(fmtBytes(5 * 1048576)).toBe("5.0 MB")
  })
  it(">=100 截断小数", () => {
    expect(fmtBytes(250 * 1024)).toBe("250 KB")
  })
  it("非法输入返回 —", () => {
    expect(fmtBytes(-1)).toBe("—")
    expect(fmtBytes(Number.NaN)).toBe("—")
  })
})

describe("fmtSpeed", () => {
  it("null / 非正值无速度", () => {
    expect(fmtSpeed(null)).toBeNull()
    expect(fmtSpeed(0)).toBeNull()
  })
  it("输出带 /s 单位", () => {
    expect(fmtSpeed(1048576)).toBe("1.0 MB/s")
  })
})

describe("fmtEta", () => {
  it("分:秒格式", () => {
    expect(fmtEta(65)).toBe("01:05")
  })
  it("小时进位", () => {
    expect(fmtEta(3661)).toBe("1:01:01")
  })
  it("非法输入 null", () => {
    expect(fmtEta(null)).toBeNull()
    expect(fmtEta(-5)).toBeNull()
    expect(fmtEta(Number.POSITIVE_INFINITY)).toBeNull()
  })
})

describe("fmtPercent", () => {
  it("total 缺失为 null（不确定进度形态）", () => {
    expect(fmtPercent(10, null)).toBeNull()
    expect(fmtPercent(10, 0)).toBeNull()
  })
  it("百分比钳制 0-100 取整", () => {
    expect(fmtPercent(50, 200)).toBe(25)
    expect(fmtPercent(999, 100)).toBe(100)
    expect(fmtPercent(-1, 100)).toBe(0)
  })
})

describe("getProfileColorClass", () => {
  it("web profile 派发品牌色", () => {
    expect(getProfileColorClass("web")).toContain("border-brand")
  })

  it("非 web profile 确定性派发非空样式类", () => {
    const cls1 = getProfileColorClass("test")
    const cls2 = getProfileColorClass("test")
    expect(cls1).toBe(cls2)
    expect(cls1).toMatch(/border-.* bg-.* text-.*/)
  })
})
