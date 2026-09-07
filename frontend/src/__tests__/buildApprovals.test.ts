import { describe, expect, it } from "vitest"
import { defaultApprovals, mergeApprovals } from "@/lib/buildApprovals"

describe("defaultApprovals", () => {
  it("全部默认跳过并保序", () => {
    expect(defaultApprovals(["ssh2", "node-pty"])).toEqual([
      { name: "ssh2", allowed: false },
      { name: "node-pty", allowed: false },
    ])
  })
  it("去重且容忍空白与空项", () => {
    expect(defaultApprovals(["ssh2", " ssh2 ", "", "ssh2"])).toEqual([
      { name: "ssh2", allowed: false },
    ])
  })
  it("空输入为空表", () => {
    expect(defaultApprovals([])).toEqual([])
  })
})

describe("mergeApprovals", () => {
  it("重试再撞门时保留既有选择", () => {
    const prior = [
      { name: "ssh2", allowed: true },
      { name: "cpu-features", allowed: false },
    ]
    expect(mergeApprovals(["ssh2", "cloudflared"], prior)).toEqual([
      { name: "ssh2", allowed: true },
      { name: "cloudflared", allowed: false },
    ])
  })
})
