// safeModeBanner.test.ts —— 安全模式可见性门禁（ADR-0026，2026-09-16 第二版）。
//
// 机制变更：安全模式从"临时 --patch overlay"改成**在 profile 配置里把三方插件 disable**
// （+ 覆写前备份 + 一键用备份恢复）。于是**不再有两个真相源**——配置说关、开关就是关，
// 面板徽标同源，故删掉旧的「这里的已启用指配置层」说明；本门禁改为钉住新的三件事：
//   ① 控制中心有横幅：交代"改的是配置、进入前已备份"+ 给一键恢复入口；
//   ② 备份不在了**就不给按钮**（不给点了必然报错的按钮），改为如实说明；
//   ③ 文案键 zh/en 齐备。
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

describe("安全模式可见性（横幅 + 一键恢复）", () => {
  const pm = stripComments(profileManagerSrc)

  it("控制中心取状态并渲染横幅（失败静默，不打断页面）", () => {
    expect(pm).toContain("getSafeModeState(")
    expect(pm).toMatch(/safeMode\?\.active && \(/)
    expect(pm, "横幅必须说明改的是配置").toContain("t.profiles.safeModeBody(")
  })

  it("一键恢复入口走已就绪的 safe_mode_exit（并 catch 错误）", () => {
    expect(pm).toContain('terminalAction("safe_mode_exit")')
    expect(pm).toMatch(/terminalAction\("safe_mode_exit"\)[\s\S]{0,260}?\.catch\(/)
  })

  it("恢复按钮受 `restorable` 门控：备份不在时不渲染按钮，只如实说明", () => {
    // 与后端 `safe_mode::state().restorable` 配套——宁可不给按钮，也不给一个点了会报错的。
    expect(pm, "横幅必须按 restorable 分支").toContain("safeMode.restorable")
    expect(pm, "不可恢复时的说明文案必须给出").toContain("t.profiles.safeModeNotRestorable")
  })

  it("面板不再有「配置层 vs 运行态」说明（机制已同源，说明即是噪音）", () => {
    const panel = stripComments(panelSrc)
    expect(panel).not.toContain("safeModeActive")
    expect(panel).not.toContain("capSafeModeNote")
  })

  it("文案键 zh/en 齐备（缺一侧会显示英文 id 或直接崩）", () => {
    for (const [zh, en] of [
      [zhCN.profiles.safeModeTitle, enUS.profiles.safeModeTitle],
      [zhCN.profiles.safeModeExit, enUS.profiles.safeModeExit],
      [zhCN.profiles.safeModeNotRestorable, enUS.profiles.safeModeNotRestorable],
      [zhCN.profiles.safeModeRestoreHint, enUS.profiles.safeModeRestoreHint],
    ]) {
      expect(String(zh).length).toBeGreaterThan(0)
      expect(String(en).length).toBeGreaterThan(0)
    }
    expect(zhCN.profiles.safeModeBody(3)).toContain("3")
    expect(enUS.profiles.safeModeBody(3)).toContain("3")
    // 说明里必须点出"配置里停用"+"可恢复"，否则用户不知道发生了什么
    expect(zhCN.profiles.safeModeBody(3)).toMatch(/配置/)
    expect(enUS.profiles.safeModeBody(3)).toMatch(/config/)
  })
})
