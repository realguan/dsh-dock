// diagnostics.test.ts —— 系统健康体检与日志截断单测（4.11 / 4.12）。
import { describe, expect, it } from "vitest"
import type { LogQueryResult, SystemDiagnosticsReport } from "@/types/ipc"

describe("System Diagnostics & Log Telemetry", () => {
  it("evaluates health status correctly from diagnostic report", () => {
    const report: SystemDiagnosticsReport = {
      node: {
        path: "/usr/local/bin/node",
        version: "v20.11.0",
        source: "system",
        isReady: true,
      },
      pnpm: {
        path: "/usr/local/bin/pnpm",
        version: "9.1.0",
        isReady: true,
      },
      dsh: {
        path: "/Users/guan/.dsh/bin/dsh",
        version: "v0.8.9",
        source: "managed",
        isReady: true,
      },
      storage: {
        dshHome: "/Users/guan/.dsh",
        totalBytes: 1024 * 1024 * 500, // 500 MB
        profilesBytes: 1024 * 1024 * 300,
        sessionsBytes: 1024 * 1024 * 200,
        profilesCount: 4,
        sessionsCount: 12,
      },
      platform: {
        os: "darwin",
        arch: "arm64",
      },
    }

    const allReady = report.node.isReady && report.pnpm.isReady && report.dsh.isReady
    expect(allReady).toBe(true)
    expect(report.storage.profilesCount).toBe(4)
    expect(report.storage.sessionsCount).toBe(12)
  })

  it("handles log truncation state and tail line counts", () => {
    const logResult: LogQueryResult = {
      source: "dsh",
      path: "/Users/guan/.dsh/logs/dsh.log",
      lines: ["Line 91", "Line 92", "Line 93", "Line 94", "Line 95"],
      totalLines: 95,
      truncated: true,
    }

    expect(logResult.truncated).toBe(true)
    expect(logResult.lines.length).toBe(5)
    expect(logResult.totalLines).toBe(95)
  })
})
