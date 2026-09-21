// marketCategorySourceGate.test.ts —— 「分类选项与插件中心一致」闸门（2026-09-21 维护者真机）。
//
// ## 复现
// 「添加插件」弹窗（市场 tab）的分类筛选下拉里是一串**原始键**：`agi` / `ui` / `usage` /
// `theme` / `model` / `identity`；而同一份 registry 在「插件中心 → 插件市场」里渲染的是
// 中文标签 + 计数。维护者裁定："全部分类选项要与插件中心那边的分类保持一致。"
//
// ## 真因
// 同一个东西两个面各写一份取数逻辑：
//   · 市场页：`Object.entries(registry.categories)` → `zh || en` + 计数 + 按数量降序；
//   · 弹窗：自己 `Set` 一遍 `plugin.category` → 原始键、无计数、顺序随数据。
// 集合/标签/顺序三样全对不上，是"同一个 registry 两种呈现"的必然。
//
// ## 判据
// ① 标签映射只有一处（`getMarketCategoryLabel`），两个面都必须调它，不得再内联
//    `categories?.[key]` 的取标签代码；
// ② 选项清单只有一处（`marketCategoryOptions`：集合 ∪ 元数据、带计数、按数量降序），
//    两个面都必须调它；
// ③ 弹窗不得再把原始键当可见文案渲染（下拉项与卡片徽标都是）。
// 纯逻辑测试（另有 market.test.ts 钉纯函数取值），不渲染 DOM（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"

import marketLibSrc from "@/lib/market.ts?raw"
import marketViewSrc from "@/components/market/MarketplaceView.tsx?raw"
import addDialogSrc from "@/components/profiles/PluginAddDialog.tsx?raw"
import marketCardSrc from "@/components/market/MarketPluginCard.tsx?raw"
import zhDictSrc from "@/content/zh-CN.ts?raw"
import enDictSrc from "@/content/en-US.ts?raw"

describe("① 分类标签映射单一来源", () => {
  it("纯函数里只有一份实现，且带「元数据缺失回退原键」的兜底", () => {
    expect(marketLibSrc, "标签函数在纯函数层").toContain("export function getMarketCategoryLabel")
    expect(marketLibSrc, "回退链：元数据缺失给原键（不瞎翻译）").toMatch(
      /if \(!obj\) return key/,
    )
    expect(marketLibSrc, "zh 优先、en 兜底").toContain('obj.zh || obj.en')
    expect(marketLibSrc, "en 反向同理").toContain('obj.en || obj.zh')
  })

  it("两个面都调它，不得再内联取标签", () => {
    expect(marketViewSrc, "市场页的分类徽标").toContain(
      "getMarketCategoryLabel(registry, plugin.category, activeLocale)",
    )
    expect(addDialogSrc, "弹窗的卡片徽标").toContain(
      "getMarketCategoryLabel(registry, plugin.category, activeLocale)",
    )
    // 反转证：内联写法不许回来（那是"两套写法"的复发面）
    for (const [name, src] of [
      ["市场页", marketViewSrc],
      ["添加插件弹窗", addDialogSrc],
    ] as const) {
      expect(src, `${name} 又出现了内联取标签`).not.toContain("categories?.[plugin.category]")
      expect(src, `${name} 又出现了内联 zh||en`).not.toMatch(/catObj\?\.zh \|\| catObj\.en/)
    }
  })
})

describe("② 选项清单单一来源（集合 / 计数 / 顺序三样一致）", () => {
  it("两个面共用 marketCategoryOptions", () => {
    expect(marketViewSrc, "市场页的分类矩阵").toContain(
      "marketCategoryOptions(registry, activeLocale)",
    )
    expect(addDialogSrc, "弹窗的分类下拉").toContain(
      "marketCategoryOptions(registry, activeLocale)",
    )
    // 反转证：弹窗不得再自行收集原始键（那是集合对不上的来源）
    expect(addDialogSrc, "弹窗不得再自行 Set 一遍分类").not.toContain("new Set<string>()")
  })

  it("纯函数本身：集合取并集、带计数、按数量降序", () => {
    expect(marketLibSrc, "元数据键 ∪ 插件用到的键").toMatch(
      /new Set<string>\(Object\.keys\(registry\.categories/,
    )
    expect(marketLibSrc, "计数来自插件实际分布").toMatch(/counts\[p\.category\] = \(counts\[p\.category\]/)
    expect(marketLibSrc, "按数量降序（与市场页 pill 同序）").toContain(
      ".sort((a, b) => b.count - a.count)",
    )
  })
})

describe("③ 弹窗不再把原始键当可见文案", () => {
  it("下拉项走「标签（计数）」，卡片徽标走同一枚 Badge", () => {
    expect(addDialogSrc, "下拉项用本地化标签 + 计数").toContain(
      "t.market.categoryOption(c.label, c.count)",
    )
    expect(addDialogSrc, "「全部」用市场页同一句文案").toContain(
      "<SelectItem value=\"all\">{t.market.allCategories}",
    )
    // 卡片徽标改用市场卡片同款 Badge（不再手搓小胶囊 + 原始键）
    expect(addDialogSrc, "卡片分类徽标走 Badge").toMatch(
      /<Badge[\s\S]{0,400}getMarketCategoryLabel\(registry, plugin\.category, activeLocale\)/,
    )
    expect(addDialogSrc, "不得再直接渲染原始键").not.toContain("{plugin.category}\n")
    // 反转证：市场卡片那边也是同一枚 Badge 基座（两处"同款"要有出处）
    expect(marketCardSrc, "市场卡片的分类徽标同样是 Badge").toMatch(
      /categoryLabel && \(\s*\n\s*<Badge/,
    )
  })

  it("文案齐备且 zh/en 对称", () => {
    for (const [name, src] of [
      ["zh-CN", zhDictSrc],
      ["en-US", enDictSrc],
    ] as const) {
      expect(src, `${name} 缺 categoryOption`).toContain("categoryOption:")
      expect(src, `${name} 缺 allCategories`).toContain("allCategories:")
    }
    expect(zhDictSrc, "zh 用全角括号（与站内其它「标签（n）」一致）").toContain(
      "categoryOption: (label: string, n: number) => `${label}（${n}）`",
    )
  })
})
