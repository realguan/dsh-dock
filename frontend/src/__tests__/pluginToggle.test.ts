// 插件开关目标纯逻辑（2026-09-08，ADR-0009 第七次修订：补丁包开关）。
// fixture 内联，不引 DOM。
import { describe, expect, it } from "vitest"
import { pluginToggleTargets } from "@/lib/pluginToggle"
import type { PluginRowState } from "@/types/ipc"

const row = (over: Partial<PluginRowState>): PluginRowState => ({
  id: "row-id",
  pkg_name: "pkg",
  shell_disabled: false,
  patch_entries: 0,
  contributed_ids: [],
  ...over,
})

describe("pluginToggleTargets", () => {
  it("普通插件：开关目标 = 自身行 id", () => {
    expect(pluginToggleTargets(row({ id: "better-sidebar" }))).toEqual([
      "better-sidebar",
    ])
  })

  it("补丁包：开关目标 = 全部贡献行（合成条目共用第一行作 id）", () => {
    expect(
      pluginToggleTargets(
        row({ id: "openviking-memory", contributed_ids: ["openviking-memory"] }),
      ),
    ).toEqual(["openviking-memory"])
    expect(
      pluginToggleTargets(
        row({ id: "row-a", contributed_ids: ["row-a", "row-b"] }),
      ),
    ).toEqual(["row-a", "row-b"])
  })

  it("无目标（空贡献行 + 空 id 的退化态）：返回单元素 [id] 不抛错", () => {
    expect(pluginToggleTargets(row({ id: "" }))).toEqual([""])
  })
})
