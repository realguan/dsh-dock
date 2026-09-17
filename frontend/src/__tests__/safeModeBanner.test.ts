// safeModeBanner.test.ts —— 安全模式可见性门禁（ADR-0026，2026-09-16 第三版）。
//
// 机制沿革：ADR-0025 的临时 `--patch` overlay（会导致"配置说启用/运行说停用"两个真相源）
// → ADR-0026 第二版"在配置里 disable + 一键用备份恢复" → **第三版：移除一键恢复**
// （维护者原话："恢复了还是起不来报错，多此一举"：恢复 = 把坏配置原样搬回来，启动照样失败）。
//
// 现在钉住三件事：
//   ① 控制中心有横幅：交代"改的是配置、进入前已备份"+ 停用了几个（**与配置实时联动**）；
//   ② **没有恢复按钮**（按钮、动作、字典键都不许再出现），出路是在「实验能力」里逐个打开开关；
//   ③ 文案键 zh/en 齐备且说清下一步。
// 纯逻辑测试：只读源码文本与字典常量，不引 RTL/jsdom（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"
import { t as zhCN } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"
import profileManagerSrc from "@/pages/ProfileManager.tsx?raw"
import panelSrc from "@/components/market/ExperimentalCapabilities.tsx?raw"

/** 结构断言剥离注释（注释里会写"为什么"，含中文，属正常）。 */
function stripComments(src: string): string {
  return src
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^[ \t]*\/\/.*$/gm, "")
}

describe("安全模式可见性（横幅；无恢复动作）", () => {
  const pm = stripComments(profileManagerSrc)

  it("控制中心取状态并渲染横幅（失败静默，不打断页面）", () => {
    expect(pm).toContain("getSafeModeState(")
    expect(pm).toMatch(/safeMode\?\.active && \(/)
    expect(pm, "横幅必须说明改的是配置").toContain("t.profiles.safeModeBody(")
  })

  it("**没有恢复动作**（维护者 2026-09-16 第三次裁定：恢复 = 把坏配置搬回来，照样失败）", () => {
    // 反向断言：横幅不得再出现恢复按钮/动作；出路是在「实验能力」里逐个打开开关。
    expect(pm).not.toContain("safe_mode_exit")
    expect(pm).not.toContain("terminalAction(")
    // 字典里也不该再留着恢复相关的键（留着就会被下一次误用）
    const zhKeys = zhCN.profiles as unknown as Record<string, unknown>
    const enKeys = enUS.profiles as unknown as Record<string, unknown>
    for (const key of [
      "safeModeExit",
      "safeModeExitFailed",
      "safeModeRestoreHint",
      "safeModeNotRestorable",
    ]) {
      expect(zhKeys[key], `zh 残留恢复键 ${key}`).toBeUndefined()
      expect(enKeys[key], `en 残留恢复键 ${key}`).toBeUndefined()
    }
  })

  it("面板不再有「配置层 vs 运行态」说明（机制已同源，说明即是噪音）", () => {
    const panel = stripComments(panelSrc)
    expect(panel).not.toContain("safeModeActive")
    expect(panel).not.toContain("capSafeModeNote")
  })

  it("文案键 zh/en 齐备且说清下一步（去哪儿打开开关）", () => {
    expect(String(zhCN.profiles.safeModeTitle).length).toBeGreaterThan(0)
    expect(String(enUS.profiles.safeModeTitle).length).toBeGreaterThan(0)
    expect(zhCN.profiles.safeModeBody(3)).toContain("3")
    expect(enUS.profiles.safeModeBody(3)).toContain("3")
    expect(zhCN.profiles.safeModeBody(3)).toMatch(/配置/)
    expect(zhCN.profiles.safeModeBody(3)).toMatch(/实验能力/)
    expect(enUS.profiles.safeModeBody(3)).toMatch(/config/)
    expect(enUS.profiles.safeModeBody(3)).toMatch(/Experimental Capabilities/)
  })
})
