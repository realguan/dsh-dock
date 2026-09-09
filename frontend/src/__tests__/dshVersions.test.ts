// dshVersions.test.ts —— 版本选择器纯逻辑（2026-09-09）。
// 复现语境：registry 高位只有 alpha 时，「有新版」与「升级」口径分叉造成
// 升级空转；版本选择器把通道选择权交给用户。本文件钉住筛选/行动作/确认
// 三组纯判定——版本比较在后端，前端不做第二套。
import { describe, expect, it } from "vitest"
import {
  filterVersions,
  formatPublishedAt,
  needsConfirm,
  rowAction,
  type VersionFilter,
} from "@/lib/dshVersions"
import type { DshVersionEntry } from "@/types/ipc"

function entry(partial: Partial<DshVersionEntry>): DshVersionEntry {
  return {
    version: "0.1.2-rc.1",
    channel: "rc",
    relation: "newer",
    published_at: null,
    ...partial,
  }
}

describe("matchFilter / filterVersions", () => {
  const list = [
    entry({ version: "0.1.5-alpha.1", channel: "alpha" }),
    entry({ version: "0.1.2-rc.1", channel: "rc" }),
    entry({ version: "0.1.2", channel: "stable" }),
    entry({ version: "0.1.2-beta.1", channel: "other" }),
  ]

  const cases: [VersionFilter, string[]][] = [
    ["all", ["0.1.5-alpha.1", "0.1.2-rc.1", "0.1.2", "0.1.2-beta.1"]],
    ["upgradable", ["0.1.2-rc.1", "0.1.2"]],
    ["preview", ["0.1.5-alpha.1", "0.1.2-beta.1"]],
  ]

  it.each(cases)("筛选档 %s", (filter, expected) => {
    expect(filterVersions(list, filter).map((e) => e.version)).toEqual(expected)
  })
})

describe("rowAction", () => {
  it("newer → install；current → current；older → rollback", () => {
    expect(rowAction(entry({ relation: "newer" }))).toBe("install")
    expect(rowAction(entry({ relation: "current" }))).toBe("current")
    expect(rowAction(entry({ relation: "older" }))).toBe("rollback")
  })
})

describe("needsConfirm（知情同意判定）", () => {
  it("高于当前的稳定/rc 直接安装，不确认", () => {
    expect(needsConfirm(entry({ channel: "stable", relation: "newer" }))).toBe(false)
    expect(needsConfirm(entry({ channel: "rc", relation: "newer" }))).toBe(false)
  })

  it("预览通道（alpha/other）一律确认——含高于当前的 alpha", () => {
    expect(needsConfirm(entry({ channel: "alpha", relation: "newer" }))).toBe(true)
    expect(needsConfirm(entry({ channel: "other", relation: "newer" }))).toBe(true)
  })

  it("低于当前（回退）一律确认——含回退到稳定/rc", () => {
    expect(needsConfirm(entry({ channel: "rc", relation: "older" }))).toBe(true)
    expect(needsConfirm(entry({ channel: "stable", relation: "older" }))).toBe(true)
  })
})

describe("formatPublishedAt", () => {
  it("RFC3339 截取日期段；无时间为 null；异常形态原样拒绝", () => {
    expect(formatPublishedAt("2026-09-07T00:00:00.000Z")).toBe("2026-09-07")
    expect(formatPublishedAt("2026-09-07T12:34:56Z")).toBe("2026-09-07")
    expect(formatPublishedAt(null)).toBe(null)
    expect(formatPublishedAt("not-a-date")).toBe(null)
  })
})
