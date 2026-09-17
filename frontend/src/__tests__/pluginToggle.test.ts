// 插件开关目标纯逻辑（2026-09-08，ADR-0009 第七次修订：补丁包开关）。
// fixture 内联，不引 DOM。
import { describe, expect, it } from "vitest"
import { pluginToggleTargets, toggleIntent } from "@/lib/pluginToggle"
import { runtimeToggleApplied } from "@/lib/profiles"
import { t as zhCN } from "@/content/zh-CN"
import type { PluginRowState, RuntimeEntry } from "@/types/ipc"

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

// 2026-09-17 独立复核抓到的极性缺陷：`set_plugin_disabled(..., disabled)` 第三参是
// **disabled**，而文案（已启用/已禁用）与落定判据（比清单里的 `enabled`）是 **enabled**。
// 旧实现把 `!shell_disabled`（新 disabled 值）当 enabled 传给文案 ⇒ 点开关"关掉"会报
// 「已启用 X（已生效）」、点开关"打开"会报「已禁用 X（重启后生效）」。这条纯函数把
// 两种口径在类型上分开，下面钉住取值方向。
describe("toggleIntent（两种口径的方向护栏）", () => {
  it("当前启用的行 → 本次要禁用：wantEnabled=false，写 disabled=true", () => {
    expect(toggleIntent(row({ shell_disabled: false }))).toEqual({
      wantEnabled: false,
      writeDisabled: true,
    })
  })

  it("当前禁用的行 → 本次要启用：wantEnabled=true，写 disabled=false", () => {
    expect(toggleIntent(row({ shell_disabled: true }))).toEqual({
      wantEnabled: true,
      writeDisabled: false,
    })
  })

  it("两种口径恒为反相（谁被当成 enabled 用都不会悄悄一致）", () => {
    for (const shellDisabled of [true, false]) {
      const intent = toggleIntent(row({ shell_disabled: shellDisabled }))
      expect(intent.wantEnabled).toBe(!intent.writeDisabled)
      // wantEnabled 反向跟随当前态：现在禁用 ⇒ 要启用
      expect(intent.wantEnabled).toBe(shellDisabled)
    }
  })

  // 组合级护栏（复核建议）：把「意图 → 文案 → 落定判据」串起来按用户动作断言。
  // 只测单函数方向是不够的——P0 正是"函数各自正确、实参接反了"。
  const entry = (enabled: boolean): RuntimeEntry => ({
    entry_id: "include:row",
    module_name: "pkg-x",
    enabled,
    fiber_phase: null,
  })

  it("用户把「禁用」的行打开：写 disabled=false、文案说已启用、判据等 enabled=true", () => {
    const { wantEnabled, writeDisabled } = toggleIntent(row({ shell_disabled: true }))
    expect(writeDisabled).toBe(false)
    expect(zhCN.profiles.toggleApplied("pkg-x", wantEnabled)).toContain("已启用")
    expect(zhCN.profiles.toggleRestart("pkg-x", wantEnabled)).toContain("已启用")
    // 热载尚未落地 → 不报落定；落地后 → 报落定（都与文案同口径）
    expect(runtimeToggleApplied([entry(false)], "pkg-x", wantEnabled)).toBe(false)
    expect(runtimeToggleApplied([entry(true)], "pkg-x", wantEnabled)).toBe(true)
  })

  it("用户把「启用」的行关掉：写 disabled=true、文案说已禁用、判据等 enabled=false", () => {
    const { wantEnabled, writeDisabled } = toggleIntent(row({ shell_disabled: false }))
    expect(writeDisabled).toBe(true)
    expect(zhCN.profiles.toggleApplied("pkg-x", wantEnabled)).toContain("已禁用")
    expect(zhCN.profiles.toggleDone("pkg-x", wantEnabled)).toContain("已禁用")
    expect(runtimeToggleApplied([entry(true)], "pkg-x", wantEnabled)).toBe(false)
    expect(runtimeToggleApplied([entry(false)], "pkg-x", wantEnabled)).toBe(true)
  })
})
