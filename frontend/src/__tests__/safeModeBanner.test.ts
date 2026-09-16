// safeModeBanner.test.ts —— 安全模式可见性门禁（ADR-0025，2026-09-16）。
//
// 真机教训（维护者报）：进了安全模式后，已装插件列表显示「已停用」而开关仍是开——
// 两个真相源（**运行时**回环快照 vs **配置层** cordis.patch.yml）本身都对，但没人解释
// 差异，于是看起来自相矛盾。故本门禁钉住三件事：
//   ① 控制中心有横幅：说明"配置未改动"+ 给退出入口；
//   ② 实验能力面板说明"这里的已启用指配置层"；
//   ③ 文案键 zh/en 齐备。
// 纯逻辑测试：只读源码文本与字典常量，不引 RTL/jsdom（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"
import { t as zhCN } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"
import profileManagerSrc from "@/pages/ProfileManager.tsx?raw"
import pluginHubSrc from "@/components/market/PluginHub.tsx?raw"
import panelSrc from "@/components/market/ExperimentalCapabilities.tsx?raw"

/** 结构断言剥离注释（注释里会写"为什么"，含中文，属正常）。 */
function stripComments(src: string): string {
  return src
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^[ \t]*\/\/.*$/gm, "")
}

describe("安全模式可见性（横幅 + 退出 + 两源说明）", () => {
  const pm = stripComments(profileManagerSrc)

  it("控制中心取状态并渲染横幅（失败静默，不打断页面）", () => {
    expect(pm).toContain("getSafeModeState(")
    expect(pm).toMatch(/safeMode\?\.active && \(/)
    expect(pm, "横幅必须说明配置未改动").toContain("t.profiles.safeModeBody(")
  })

  it("退出入口走已就绪的 safe_mode_exit（并 catch 错误）", () => {
    expect(pm).toContain('terminalAction("safe_mode_exit")')
    expect(pm).toMatch(/terminalAction\("safe_mode_exit"\)[\s\S]{0,200}?\.catch\(/)
  })

  it("实验能力面板说明「已启用指配置层」，且 prop 由 PluginHub 透传", () => {
    const panel = stripComments(panelSrc)
    expect(panel).toContain("safeModeActive")
    expect(panel).toContain("t.market.capSafeModeNote")
    expect(stripComments(pluginHubSrc)).toContain("safeModeActive={safeModeActive}")
  })

  it("文案键 zh/en 齐备（缺一侧会显示英文 id 或直接崩）", () => {
    for (const [zh, en] of [
      [zhCN.profiles.safeModeTitle, enUS.profiles.safeModeTitle],
      [zhCN.profiles.safeModeExit, enUS.profiles.safeModeExit],
      [zhCN.market.capSafeModeNote, enUS.market.capSafeModeNote],
    ]) {
      expect(String(zh).length).toBeGreaterThan(0)
      expect(String(en).length).toBeGreaterThan(0)
    }
    expect(zhCN.profiles.safeModeBody(3)).toContain("3")
    expect(enUS.profiles.safeModeBody(3)).toContain("3")
  })
})
