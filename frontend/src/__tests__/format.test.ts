// 格式化纯函数测试（frontend-migration §8.1）。
import { describe, expect, it } from "vitest"
import {
  fmtBytes,
  fmtEta,
  fmtPercent,
  fmtSpeed,
  getPaginationPages,
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
  it("不足 1 秒 null（即将完成，不展示 00:00）", () => {
    expect(fmtEta(0.4)).toBeNull()
    expect(fmtEta(0)).toBeNull()
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
  // 2026-09-10 批次 E：身份色不再与 profile 名相关（旧实现 = 7 色彩虹哈希 +
  // web 特判品牌蓝）。原因见 format.ts 注释：名字才是信息，彩虹在等明度约束下
  // 分不开，且借用了状态色域。现为单一分类档 token。
  it("所有 profile 派发同一分类档 token（不再哈希彩虹）", () => {
    const a = getProfileColorClass("web")
    const b = getProfileColorClass("frontend-dev")
    expect(a).toBe(b)
    expect(a).toContain("text-alt")
    expect(a).toContain("bg-alt-soft")
  })

  it("身份色不借用状态色域（ok/info/warn/danger）", () => {
    const cls = getProfileColorClass("anything")
    expect(cls).not.toMatch(/text-(ok|info|warn|danger)\b/)
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

describe("getPaginationPages", () => {
  it("<= 7 页时完整展示所有页码，无省略号", () => {
    expect(getPaginationPages(1, 1)).toEqual([1])
    expect(getPaginationPages(1, 5)).toEqual([1, 2, 3, 4, 5])
    expect(getPaginationPages(4, 7)).toEqual([1, 2, 3, 4, 5, 6, 7])
  })

  it("靠近开头（currentPage <= 4）展示前 5 页和末页", () => {
    expect(getPaginationPages(1, 29)).toEqual([1, 2, 3, 4, 5, "...", 29])
    expect(getPaginationPages(3, 29)).toEqual([1, 2, 3, 4, 5, "...", 29])
    expect(getPaginationPages(4, 29)).toEqual([1, 2, 3, 4, 5, "...", 29])
  })

  it("靠近末尾（currentPage >= totalPages - 3）展示首页和后 5 页", () => {
    expect(getPaginationPages(26, 29)).toEqual([1, "...", 25, 26, 27, 28, 29])
    expect(getPaginationPages(28, 29)).toEqual([1, "...", 25, 26, 27, 28, 29])
    expect(getPaginationPages(29, 29)).toEqual([1, "...", 25, 26, 27, 28, 29])
  })

  it("居中时首尾各保留一个省略号，中间围绕当前页", () => {
    expect(getPaginationPages(10, 29)).toEqual([1, "...", 9, 10, 11, "...", 29])
    expect(getPaginationPages(15, 29)).toEqual([1, "...", 14, 15, 16, "...", 29])
  })
})
