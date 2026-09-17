// 插件运行态合并纯逻辑（4.4①）：徽标种类、按 moduleName 的归并、开关是否落定、
// 会话级汇总。fixture 内联，不引 DOM。
//
// 2026-09-17 维护者实机提问（「为什么既有已禁用又有已停用」）后的口径：
//   配置侧（壳写的 cordis.patch.yml）→「已禁用」；运行侧（pluginInventory/list）→
//   运行中 / 加载中 / 失败 / 未加载（配置启用但会话里无实例）/ 未生效（会话里这行仍是禁用）。
//   两个「没到位」的状态必须能区分，否则用户无法判断"是我的开关没生效，还是插件没装好"。
import { describe, expect, it } from "vitest"
import {
  runtimeChipFor,
  runtimeSummary,
  runtimeToggleApplied,
  validatePluginSpec,
} from "@/lib/profiles"
import type { RuntimeEntry } from "@/types/ipc"

const e = (
  module_name: string,
  fiber_phase: RuntimeEntry["fiber_phase"],
  enabled = true,
): RuntimeEntry => ({ entry_id: `${module_name}-${Math.random()}`, module_name, enabled, fiber_phase })

describe("runtimeChipFor（运行侧徽标）", () => {
  it("returns null when module has no entries", () => {
    expect(runtimeChipFor("some-pkg", [e("other", "active")])).toBeNull()
  })

  it("failed dominates active (warn color)", () => {
    const chip = runtimeChipFor("p", [e("p", "active"), e("p", "failed")])
    expect(chip).toEqual({ kind: "failed", count: 1 })
  })

  it("loading beats active when no failure", () => {
    const chip = runtimeChipFor("p", [e("p", "active"), e("p", "loading"), e("p", "pending")])
    expect(chip).toEqual({ kind: "loading", count: 2 })
  })

  it("counts active entries only when enabled", () => {
    expect(runtimeChipFor("p", [e("p", "active"), e("p", "active")])).toEqual({
      kind: "active",
      count: 2,
    })
  })

  it("配置启用但会话里没有实例 → 未加载（不是「已停用」）", () => {
    // fiber 被 dispose（null）或条目尚未建 fiber，但 enabled=true
    expect(runtimeChipFor("p", [e("p", null)])).toEqual({
      kind: "unloaded",
      count: 1,
    })
    // 混合：一条 enabled=false + 一条 enabled=true 无 fiber ⇒ 仍是「没实例」
    expect(runtimeChipFor("p", [e("p", null), e("p", null, false)])).toEqual({
      kind: "unloaded",
      count: 2,
    })
  })

  it("会话里这一行仍是禁用 → 未生效（开关还没被运行中的 dsh 应用）", () => {
    expect(runtimeChipFor("p", [e("p", "active", false)])).toEqual({
      kind: "notApplied",
      count: 1,
    })
    expect(runtimeChipFor("p", [e("p", null, false), e("p", null, false)])).toEqual({
      kind: "notApplied",
      count: 2,
    })
  })
})

describe("runtimeToggleApplied（开关是否已在运行中的 dsh 里落地）", () => {
  it("目标态出现在任一条目即算落地", () => {
    expect(runtimeToggleApplied([e("p", "active", true)], "p", true)).toBe(true)
    expect(runtimeToggleApplied([e("p", null, false)], "p", false)).toBe(true)
  })

  it("仍是旧态 ⇒ 未落地（web profile 热载前的真实中间态）", () => {
    expect(runtimeToggleApplied([e("p", null, false)], "p", true)).toBe(false)
    expect(runtimeToggleApplied([e("p", "active", true)], "p", false)).toBe(false)
  })

  it("无该模块条目（会话未运行 / 无条目）⇒ 未落地，由调用方如实报「重启后生效」", () => {
    expect(runtimeToggleApplied([], "p", true)).toBe(false)
    expect(runtimeToggleApplied([e("other", "active")], "p", false)).toBe(false)
  })
})

describe("runtimeSummary", () => {
  it("buckets all phases", () => {
    const entries = [
      e("a", "active"),
      e("b", "active"),
      e("c", "failed"),
      e("d", "loading"),
      e("f", null),
      e("g", "active", false),
      e("h", "pending"),
    ]
    expect(runtimeSummary(entries)).toEqual({
      active: 2,
      failed: 1,
      loading: 2,
      disabled: 2,
    })
  })

  it("empty snapshot", () => {
    expect(runtimeSummary([])).toEqual({ active: 0, failed: 0, loading: 0, disabled: 0 })
  })
})

describe("validatePluginSpec（镜像后端 plugins::validate_plugin_spec）", () => {
  it("accepts names, scopes and version segments", () => {
    for (const ok of [
      "dsh-better-sidebar",
      "@scope/pkg",
      "pkg@0.16.1",
      "pkg@next",
      "pkg@^1.0.0",
    ]) {
      expect(validatePluginSpec(ok)).toBeNull()
    }
  })

  it("rejects flag injection, whitespace, metacharacters, oversize", () => {
    for (const bad of [
      "",
      "-flag",
      "--frozen-lockfile",
      "pkg; rm -rf ~",
      "a b",
      "pkg@>=2",
      "pkg`id`",
      "b".repeat(215),
    ]) {
      expect(validatePluginSpec(bad)).not.toBeNull()
    }
  })
})
