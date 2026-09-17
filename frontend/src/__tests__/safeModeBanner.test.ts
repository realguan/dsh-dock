// safeModeBanner.test.ts —— 安全模式可见性门禁（ADR-0026，2026-09-16 第四版交互）。
//
// 机制沿革：ADR-0025 的临时 `--patch` overlay（会导致"配置说启用/运行说停用"两个真相源）
// → ADR-0026 第二版"在配置里 disable + 一键用备份恢复" → **第三版：移除一键恢复**
// （维护者原话："恢复了还是起不来报错，多此一举"：恢复 = 把坏配置原样搬回来，启动照样失败）。
//
// 现在钉住四件事（第四版交互，2026-09-16 维护者按 PM 口径重写）：
//   ① 横幅**只在确实以安全模式进入过**时出现（`active` + `noticeDismissed` 两个条件）；
//   ② 有**关闭 icon**，点了写记账（`dismissSafeModeNotice` → `dismiss_safe_mode_notice`），
//      关后同轮不再出现；下次进入安全模式重新提示（后端单测钉住重置）；
//   ③ **没有恢复动作**，文案也不提"备份"、不把人往「实验能力」引（那里只有策展能力）；
//   ④ 文案键 zh/en 齐备且说清"去哪儿把它们开回来"。
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
    expect(pm).toMatch(/safeMode\?\.active && !safeMode\.noticeDismissed && \(/)
    expect(pm, "横幅必须说明改的是配置").toContain("t.profiles.safeModeBody(")
  })

  it("只在安全模式进入后出现，且带可关闭的 icon（关后写入记账）", () => {
    // 展示条件：仍在安全模式 + 本轮尚未被关闭
    expect(pm).toMatch(/safeMode\?\.active && !safeMode\.noticeDismissed && \(/)
    expect(pm, "关闭必须写记账（持久，不是纯本地状态）").toContain(
      ".dismissSafeModeNotice(selectedName)",
    )
    expect(pm).toMatch(/dismissSafeModeNotice\([\s\S]{0,120}?\.catch\(/)
    // 关闭按钮必须有可访问名（图标按钮没有文字）
    expect(pm).toContain("aria-label={t.profiles.safeModeDismiss}")
    expect(pm, "关闭按钮用 X 图标").toMatch(/<X className=/)
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

  it("文案只说「发生了什么 + 去哪儿开回来」：不提备份、不指向「实验能力」", () => {
    expect(String(zhCN.profiles.safeModeTitle).length).toBeGreaterThan(0)
    expect(String(enUS.profiles.safeModeTitle).length).toBeGreaterThan(0)
    expect(zhCN.profiles.safeModeBody(3)).toContain("3")
    expect(enUS.profiles.safeModeBody(3)).toContain("3")
    // 说清"全部停用"与去处（「插件」页）
    expect(zhCN.profiles.safeModeBody(3)).toMatch(/全部三方插件/)
    expect(zhCN.profiles.safeModeBody(3)).toMatch(/插件/)
    expect(enUS.profiles.safeModeBody(3)).toMatch(/All third-party plugins/)
    expect(enUS.profiles.safeModeBody(3)).toMatch(/Plugins page/)
    // 噪音清零：不提备份、不指向策展面板
    for (const text of [zhCN.profiles.safeModeBody(3), enUS.profiles.safeModeBody(3)]) {
      expect(text).not.toMatch(/备份|backed up|backup/)
      expect(text).not.toMatch(/实验能力|Experimental Capabilities/)
    }
    // 关闭文案两侧齐备
    expect(String(zhCN.profiles.safeModeDismiss).length).toBeGreaterThan(0)
    expect(String(enUS.profiles.safeModeDismiss).length).toBeGreaterThan(0)
  })
})
