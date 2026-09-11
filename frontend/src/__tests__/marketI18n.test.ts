// marketI18n.test.ts —— 市场文案 i18n 收口门禁（2026-09-11，task-16）。
// 背景：qa-verify 实测发现 `components/market/` 存在硬编码中文；其中
// `MarketplaceView.tsx` 的 `"加载中…"` 无 locale 分支 ⇒ en-US 用户直接看到
// 中文（真 i18n 缺陷，非风格问题）。本门禁守住四件事：
//   ① en-US 的 market 字典不得含中文（含动态文案的求值结果）；
//   ② 本轮新增键在 zh/en 两侧对称存在且类型一致；
//   ③ 组件确实消费字典键（?raw 读源码，同 profilesCopy.test.ts 先例）；
//   ④ 被收口的旧内联中文字面量不得回流（只查这 15 处特征串，不做全仓扫荡）。
// 纯逻辑测试：只读字典常量 + 源码文本，不渲染 DOM、不引 RTL/jsdom（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"
import { t as zhCN } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"
import marketplaceViewSrc from "@/components/market/MarketplaceView.tsx?raw"
import marketInstallDialogSrc from "@/components/market/MarketInstallDialog.tsx?raw"
import marketCustomInstallDialogSrc from "@/components/market/MarketCustomInstallDialog.tsx?raw"
import marketPluginCardSrc from "@/components/market/MarketPluginCard.tsx?raw"

/** 汉字 + CJK 标点/全角符号（en 侧出现任一即为未翻译）。 */
const HAN = /[\u3000-\u303f\u4e00-\u9fff\uff00-\uffef]/

/** 本轮为市场文案收口新增的键（task-16）。 */
const NEW_KEYS = [
  "reinstallBtn",
  "installToBtn",
  "installedBadge",
  "installedInProfile",
  "installedWillOverwrite",
  "installedWillReinstall",
  "noDescription",
  "officialCoreTitle",
  "loadingBtn",
  "openLinkFailed",
  "clearFilters",
] as const

const marketZh = zhCN.market as unknown as Record<string, unknown>
const marketEn = enUS.market as unknown as Record<string, unknown>

/** 用探针实参求值动态文案（市场文案的函数参数均为 string | number）。 */
function evaluate(value: unknown): string {
  if (typeof value === "function") {
    return String((value as (...args: unknown[]) => unknown)("probe", "probe2", "probe3"))
  }
  return String(value)
}

describe("市场文案 i18n 收口（task-16）", () => {
  it("en-US market 字典全部键不含中文（含动态文案求值结果）", () => {
    const offenders: string[] = []
    for (const [key, value] of Object.entries(marketEn)) {
      const rendered = evaluate(value)
      if (HAN.test(rendered)) offenders.push(`market.${key} = ${JSON.stringify(rendered)}`)
    }
    expect(offenders, "en-US 侧出现中文（en 用户可见）").toEqual([])
  })

  it("新增键两侧对称存在且类型一致（杜绝半迁移）", () => {
    for (const key of NEW_KEYS) {
      expect(key in marketZh, `zh-CN 缺键 market.${key}`).toBe(true)
      expect(key in marketEn, `en-US 缺键 market.${key}`).toBe(true)
      expect(typeof marketEn[key], `market.${key} 两侧类型不一致`).toBe(typeof marketZh[key])
      // 两侧均不得为空串，避免「补了键但没文案」
      expect(evaluate(marketZh[key]).length, `zh-CN market.${key} 为空`).toBeGreaterThan(0)
      expect(evaluate(marketEn[key]).length, `en-US market.${key} 为空`).toBeGreaterThan(0)
    }
  })

  it("loadingBtn 与 loadingRegistry 语义分离：按钮态不复用目录加载文案", () => {
    expect(marketZh.loadingBtn).not.toBe(marketZh.loadingRegistry)
    expect(String(marketZh.loadingBtn)).toBe("加载中…")
    // 按钮 loading 态不涉及 Registry，术语不得串台
    expect(String(marketZh.loadingBtn)).not.toMatch(/registry/i)
    expect(String(marketEn.loadingBtn)).not.toMatch(/registry/i)
    expect(marketplaceViewSrc).toContain("t.market.loadingBtn")
    expect(marketplaceViewSrc).not.toContain("loadingRegistry")
  })

  it("组件确实消费新键（?raw 源码断言）", () => {
    const consumption: Array<[string, string, readonly string[]]> = [
      ["MarketplaceView", marketplaceViewSrc, ["t.market.loadingBtn", "t.market.openLinkFailed(", "t.market.clearFilters"]],
      ["MarketInstallDialog", marketInstallDialogSrc, ["t.market.reinstallBtn", "t.market.installedWillOverwrite", "t.market.installedBadge"]],
      ["MarketCustomInstallDialog", marketCustomInstallDialogSrc, ["t.market.installedWillReinstall", "t.market.installedBadge", "t.market.installToBtn(", "t.about.cancelBtn"]],
      ["MarketPluginCard", marketPluginCardSrc, ["t.market.noDescription", "t.market.officialCoreTitle", "t.market.sourceNpm", "t.market.sourceGithub", "t.market.installedInProfile("]],
    ]
    for (const [label, src, keys] of consumption) {
      for (const key of keys) {
        expect(src, `${label} 未消费 ${key}`).toContain(key)
      }
    }
  })

  it("旧内联中文字面量不得回流（仅这 15 处特征串）", () => {
    const sources: Array<[string, string]> = [
      ["MarketplaceView", marketplaceViewSrc],
      ["MarketInstallDialog", marketInstallDialogSrc],
      ["MarketCustomInstallDialog", marketCustomInstallDialogSrc],
      ["MarketPluginCard", marketPluginCardSrc],
    ]
    const literals = [
      '"加载中…"',
      '"重新安装"',
      '"暂无描述"',
      '"DSH 官方核心插件"',
      "已在此 Profile 安装",
      "清除所有筛选条件",
      "打开链接失败",
      "已安装在 ",
      "安装到 ",
      '"取消"',
    ]
    for (const [label, src] of sources) {
      for (const literal of literals) {
        expect(src, `${label} 回流了旧内联文案 ${literal}`).not.toContain(literal)
      }
    }
  })

  it("死分支已清除：cancelBtn 不再挂无意义的中文兜底", () => {
    // 原实现 `t.about.cancelBtn || "取消"`——该键 zh/en 两侧恒在，兜底永不渲染，
    // 只会污染后续文案扫描（2026-09-11 裁定：删兜底而非为它补键）。
    expect(marketCustomInstallDialogSrc).toContain("t.about.cancelBtn")
    expect(marketCustomInstallDialogSrc).not.toContain('|| "取消"')
  })
})
