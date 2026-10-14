// reservedProfileScopeGate.test.ts —— 官方保留档的**分面闸门**（ADR-0034，2026-10-14）。
//
// ## 背景
// 2026-10-06 发现 dsh 对 `--profile desktop` 有纯名字守卫，于是壳把该档定为「保留名」并
// **一刀切关闭一切写入口**。维护者 2026-10-14 裁定：这只是收口过度——「启动、删除、
// 重命名、设置为默认 profile 这一类的操作都禁用。插件、实验能力、MCP 等都正常使用」。
//
// ## 判据（本测试钉住的形状）
// ① **内容面开放**：插件行内动作（启停/更新/卸载）、「添加插件」、自研插件安装、MCP 增删改、
//    实验能力、市场安装目标——都不再因 `reserved` 关闭；
// ② **生命周期面仍关**：启动 / 重启 / 设为默认 / 改名 / 删除；
// ③ **文案分叉**：横幅讲两面（`reservedHint`），被禁的生命周期入口用 `reservedLifecycleHint`；
// ④ 文案 zh/en 对称。
//
// 纯逻辑测试：只读源码文本，不渲染 DOM（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"

import paneSrc from "@/components/profiles/ProfileDetailPane.tsx?raw"
import rowSrc from "@/components/profiles/ProfileRow.tsx?raw"
import mcpSrc from "@/components/profiles/McpManager.tsx?raw"
import firstPartySrc from "@/components/profiles/FirstPartyPlugins.tsx?raw"
import managerSrc from "@/pages/ProfileManager.tsx?raw"
import marketSrc from "@/components/market/MarketplaceView.tsx?raw"
import zhDictSrc from "@/content/zh-CN.ts?raw"
import enDictSrc from "@/content/en-US.ts?raw"

describe("官方保留档：只禁生命周期面（ADR-0034）", () => {
  it("内容面：插件行内动作与「添加插件」不再因 reserved 关闭", () => {
    // 行内动作的 busy 门只剩 opBusy（reserved 不再参与）
    expect(paneSrc, "行内动作门").toMatch(/opBusy=\{opBusy !== null\}/)
    expect(paneSrc, "不得再把 reserved 掺进行内动作门").not.toContain(
      "opBusy !== null || reserved",
    )
    // 「添加插件」是内容面入口，恢复可用
    expect(paneSrc, "添加插件按钮").not.toMatch(
      /disabled=\{opBusy !== null \|\| reserved\}/,
    )
  })

  it("内容面：自研插件与 MCP 面板整体不再有只读开关", () => {
    // 两个组件的 `readOnly` 属性已删除（不再有 true 的调用方 ⇒ 留着就是误导）
    expect(paneSrc, "父级不得再传 readOnly").not.toContain("readOnly={reserved}")
    expect(firstPartySrc, "自研插件不再有 readOnly").not.toContain("readOnly")
    expect(mcpSrc, "MCP 面板不再有 readOnly").not.toContain("readOnly")
  })

  it("内容面：市场安装目标不再过滤保留档", () => {
    expect(marketSrc, "保留档照常进安装目标").not.toContain(
      ".filter((p) => !p.reserved)",
    )
  })

  it("生命周期面：启动 / 重启 / 设为默认 / 改名 / 删除仍然关闭", () => {
    // 详情页「未物化」分支的唯一动作 = 启动，保留档仍不给
    expect(paneSrc, "详情页启动按钮").toMatch(
      /onClick=\{onLaunch\}\s*\n\s*disabled=\{busy \|\| reserved\}/,
    )
    // 左栏：启动 / 重启按钮
    expect(rowSrc, "左栏启动/重启门").toContain("disabled={busy || reserved}")
    // 左栏菜单：设为默认 / 改名
    expect(rowSrc, "设为默认与改名禁用").toContain("disabled: reserved")
    // 左栏菜单：删除
    expect(rowSrc, "删除禁用").toMatch(/disabled=\{reserved\}/)
    // 页面兜底：生命周期动作在打到后端前就拦下
    expect(managerSrc, "父级兜底仍拦生命周期").toContain("reservedBlocked(name)")
  })

  it("文案分叉：横幅讲两面、生命周期入口讲一面，且 zh/en 对称", () => {
    for (const [name, src] of [
      ["zh-CN", zhDictSrc],
      ["en-US", enDictSrc],
    ] as const) {
      expect(src, `${name} 缺横幅键`).toContain("reservedHint:")
      expect(src, `${name} 缺生命周期键`).toContain("reservedLifecycleHint:")
    }
    // 被禁的入口用生命周期文案（拿横幅文案会顺带宣称"插件可用"，语义错位）
    expect(rowSrc, "左栏禁用提示用生命周期文案").toContain(
      "t.profiles.reservedLifecycleHint",
    )
    expect(paneSrc, "详情页禁用提示用生命周期文案").toContain(
      "t.profiles.reservedLifecycleHint",
    )
    expect(managerSrc, "父级 toast 用生命周期文案").toContain(
      "t.profiles.reservedLifecycleHint",
    )
    // 横幅仍讲清两面（否则用户会以为整页只读）
    expect(paneSrc, "横幅用两面文案").toContain("{t.profiles.reservedHint}")
  })
})
