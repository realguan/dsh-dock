import { describe, expect, it } from "vitest"

import { classifyLogLine } from "@/lib/logLevel"

describe("classifyLogLine", () => {
  it("识别 Rust/tracing 风格的行首级别", () => {
    expect(
      classifyLogLine("2026-09-18T10:00:00Z ERROR dsh::core: spawn failed"),
    ).toBe("error")
    expect(classifyLogLine("[2026-09-18 WARN dock::updates] slow")).toBe("warn")
    expect(classifyLogLine("INFO boot completed")).toBe("info")
    expect(classifyLogLine("FATAL crash")).toBe("error")
  })

  it("小写 error:/warn:/info: 前缀形式也可识别", () => {
    expect(classifyLogLine("error: cannot read settings")).toBe("error")
    expect(classifyLogLine("warn: retrying download")).toBe("warn")
    expect(classifyLogLine("info: listening on 127.0.0.1")).toBe("info")
  })

  it("路径/正文含 error 字样不再误判（回归闸门）", () => {
    expect(
      classifyLogLine("reading /Users/x/error-reports/error-handler.ts ok"),
    ).toBe(null)
    expect(classifyLogLine("recovered from error-free path scan")).toBe(null)
  })

  it("级别出现在扫描窗口之外视为正文", () => {
    const long = "a".repeat(120) + " ERROR tail"
    expect(classifyLogLine(long)).toBe(null)
  })

  it("普通行返回 null", () => {
    expect(classifyLogLine("server started, pid=4242")).toBe(null)
  })
})
