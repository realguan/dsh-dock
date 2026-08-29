// 插件运行态合并纯逻辑（4.4①）：相位标签、按 moduleName 的徽标归并、
// 会话级汇总。fixture 内联，不引 DOM。
import { describe, expect, it } from "vitest"
import {
  phaseLabel,
  runtimeChipFor,
  runtimeSummary,
  validatePluginSpec,
} from "@/lib/profiles"
import type { RuntimeEntry } from "@/types/ipc"

const e = (
  module_name: string,
  fiber_phase: RuntimeEntry["fiber_phase"],
  enabled = true,
): RuntimeEntry => ({ entry_id: `${module_name}-${Math.random()}`, module_name, enabled, fiber_phase })

describe("phaseLabel", () => {
  it("maps known phases and disposed null", () => {
    expect(phaseLabel("active")).toBe("运行中")
    expect(phaseLabel("failed")).toBe("失败")
    expect(phaseLabel("loading")).toBe("加载中")
    expect(phaseLabel(null)).toBe("已停用")
    // 前向兼容：未知 phase 原样透传
    expect(phaseLabel("hibernating")).toBe("hibernating")
  })
})

describe("runtimeChipFor", () => {
  it("returns null when module has no entries", () => {
    expect(runtimeChipFor("some-pkg", [e("other", "active")])).toBeNull()
  })

  it("failed dominates active (warn color)", () => {
    const chip = runtimeChipFor("p", [e("p", "active"), e("p", "failed")])
    expect(chip).toEqual({ label: "失败×1", failed: true })
  })

  it("loading beats active when no failure", () => {
    const chip = runtimeChipFor("p", [
      e("p", "active"),
      e("p", "loading"),
      e("p", "pending"),
    ])
    expect(chip).toEqual({ label: "加载中×2", failed: false })
  })

  it("active counted, disabled falls back", () => {
    expect(runtimeChipFor("p", [e("p", "active"), e("p", "active")])).toEqual({
      label: "运行中×2",
      failed: false,
    })
    // 全部停用（disposed/禁用）→ 已停用兜底
    expect(runtimeChipFor("p", [e("p", null), e("p", "active", false)])).toEqual({
      label: "已停用",
      failed: false,
    })
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
