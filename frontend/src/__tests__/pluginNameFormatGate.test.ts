// pluginNameFormatGate.test.ts —— 「插件名格式跨子页一致」闸门（2026-09-21 维护者真机）。
//
// ## 复现
// 同一个插件在两个子页里长得完全不一样：
//   · 插件市场卡片：`dsh-browser-use`；
//   · 已安装卡片：`@deepseek-ai/dsh-br…`（带 scope 的 npm 全名，14 个字符的 scope
//     把卡片宽度吃光，剩下的名字被截成一个认不出的前缀）。
// 维护者裁定："已安装里的插件名格式应该要与插件市场的插件名格式保持一致。"
//
// ## 判据（三个面同一条规则）
//   ① 展示名一律**去 npm scope**（`pluginShortId`）；市场那边的 registry 名本来就不带
//      scope，故两边对同一个插件给出**同一个字符串**；
//   ② 完整包名留在 `title`（去 scope ≠ 丢信息，排查仍可取）；
//   ③ 排版同源：mono / xs / semibold / tracking-tight —— 同一个东西不该有两套字号；
//   ④ 名字格吃满剩余宽度（`flex-1 min-w-0`），不因为 flex 收缩而提前截断。
// 纯逻辑测试：只读源码文本，不渲染 DOM（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"

import overviewSrc from "@/components/profiles/PluginOverview.tsx?raw"
import marketCardSrc from "@/components/market/MarketPluginCard.tsx?raw"
import rowSrc from "@/components/profiles/pluginRows/PluginListRow.tsx?raw"

/** 去掉 `{item.name}` 这种"直接渲染完整包名"的可见槽位判据用。 */
describe("① 展示名去 scope（与插件市场同格式）", () => {
  it("已安装卡片走 pluginShortId，完整包名进 title", () => {
    expect(overviewSrc, "缺 pluginShortId 导入").toContain(
      'import { pluginShortId } from "@/lib/pluginDisplay"',
    )
    expect(overviewSrc, "展示名必须去 scope").toContain("{pluginShortId(item.name)}")
    expect(overviewSrc, "完整包名进 title").toMatch(/title=\{item\.name\}/)
    // 反转证：可见槽位不得再直接渲染带 scope 的全名
    expect(overviewSrc, "不得直接渲染 item.name 当标题").not.toMatch(
      /text-ink[^>]*>\s*\{item\.name\}/,
    )
  })

  it("三个面共用同一套名字排版（mono / xs / semibold / tracking-tight）", () => {
    // 同一个东西两套字号 = 用户以为它们不是同一类对象。
    for (const [name, src] of [
      ["插件市场卡片", marketCardSrc],
      ["已安装卡片", overviewSrc],
    ] as const) {
      expect(src, `${name} 的名字排版不齐`).toContain(
        "font-mono text-xs font-semibold",
      )
      expect(src, `${name} 缺 tracking-tight`).toContain("tracking-tight")
    }
    // 插件列表行另有形态（表格行更密），但同样走 lib/pluginDisplay 的同一族函数
    expect(rowSrc, "插件列表行也走同一族").toMatch(
      /pluginMemberLabel\(p\.name\) : pluginShortId\(p\.name\)/,
    )
  })
})

describe("② 名字格吃满剩余宽度（不再提前截断）", () => {
  it("已安装卡片的名字格是 flex-1 min-w-0", () => {
    // 旧写法：外层 `flex items-center gap-2 min-w-0` 不带 flex-1 → 名字与版本徽标争宽，
    // 名字先被压到只剩十几个字符。现在名字格吃满剩余宽度，装得下就完整显示。
    expect(overviewSrc, "名字格吃满剩余宽度").toMatch(
      /min-w-0 flex-1 truncate font-mono text-xs font-semibold tracking-tight/,
    )
    expect(overviewSrc, "外层容器同样 flex-1 min-w-0").toMatch(
      /flex min-w-0 flex-1 items-center gap-2/,
    )
    expect(overviewSrc, "版本徽标不参与压缩").toMatch(
      /shrink-0 rounded-md bg-line px-1\.5 py-0\.5 font-mono/,
    )
  })
})
