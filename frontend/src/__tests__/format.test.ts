// 格式化纯函数测试（frontend-migration §8.1）。
import { describe, expect, it } from "vitest"
import {
  fmtBytes,
  fmtEta,
  fmtPercent,
  fmtSpeed,
  getProfileColorClass,
  localizeLogTimestamp,
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

describe("localizeLogTimestamp", () => {
  it("UTC 时间戳换算为本地时区（保留毫秒/ANSI 转义）", () => {
    // 机器无关断言：结果应等于「UTC 时刻对应的本地 ISO」——用 Date 重构目标值，
    // 避免依赖运行机器时区（CI 可能是 UTC 也可能是 +8）。
    const input = "\u001b[2m2026-09-05T05:15:33.883479Z\u001b[0m INFO 会话修复开始"
    const out = localizeLogTimestamp(input)
    const d = new Date("2026-09-05T05:15:33.883Z")
    const pad = (n: number) => String(n).padStart(2, "0")
    const expected = `\u001b[2m${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(
      d.getHours(),
    )}:${pad(d.getMinutes())}:${pad(d.getSeconds())}.883\u001b[0m INFO 会话修复开始`
    expect(out).toBe(expected)
    // 时间值确实变了（UTC 05:15 → 本地非 05:15，除非本机在 UTC）
    expect(out).not.toContain("05:15:33")
  })

  it("无时间戳行原样返回", () => {
    expect(localizeLogTimestamp("INFO 普通日志")).toBe("INFO 普通日志")
    expect(localizeLogTimestamp("")).toBe("")
  })

  it("跨日边界正确（UTC 23:xx → 本地次日 07:xx，UTC+8）", () => {
    const out = localizeLogTimestamp("2026-09-04T23:59:59.999999Z 跨日")
    const d = new Date("2026-09-04T23:59:59.999Z")
    const pad = (n: number) => String(n).padStart(2, "0")
    expect(out).toContain(
      `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(
        d.getHours(),
      )}:${pad(d.getMinutes())}:${pad(d.getSeconds())}.999`,
    )
  })

  it("无 ANSI 的纯文本日志行也可转换", () => {
    const out = localizeLogTimestamp("2026-09-05T05:30:00.123456Z INFO plain")
    // 目标 = UTC 时刻的本地表示（机器无关），断言时间戳已非原 UTC 值或同为本地值
    const d = new Date("2026-09-05T05:30:00.123Z")
    const pad = (n: number) => String(n).padStart(2, "0")
    const localHm = `${pad(d.getHours())}:${pad(d.getMinutes())}`
    expect(out).toContain(`T${localHm}:${pad(d.getSeconds())}.123`)
    expect(out).toContain(" INFO plain")
  })
})
