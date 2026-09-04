// 统一日志封装测试：格式契约（[模块] 描述 {ctx}）、级别路由、debug 门控。
import { afterEach, describe, expect, it, vi } from "vitest"
import { logger } from "@/lib/logger"

afterEach(() => {
  vi.restoreAllMocks()
})

describe("lib/logger.ts", () => {
  it("routes levels to the matching console method", () => {
    const err = vi.spyOn(console, "error").mockImplementation(() => {})
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
    const log = vi.spyOn(console, "log").mockImplementation(() => {})

    logger.error("mod", "boom", { code: 1 })
    logger.warn("mod", "careful")
    logger.info("mod", "hello")

    expect(err).toHaveBeenCalledWith("[mod] boom", { code: 1 })
    expect(warn).toHaveBeenCalledWith("[mod] careful")
    expect(log).toHaveBeenCalledWith("[mod] hello")
  })

  it("omits empty context object", () => {
    const log = vi.spyOn(console, "log").mockImplementation(() => {})
    logger.info("mod", "plain", {})
    expect(log).toHaveBeenCalledWith("[mod] plain")
  })

  it("debug respects dev gating (test env = dev, outputs)", () => {
    const log = vi.spyOn(console, "log").mockImplementation(() => {})
    logger.debug("mod", "dbg", { k: "v" })
    expect(log).toHaveBeenCalledWith("[mod] dbg", { k: "v" })
  })
})
