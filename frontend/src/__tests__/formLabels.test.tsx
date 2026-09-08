// formLabels.test.ts —— 表单控件无障碍名称源码闸门（2026-09-08，UI/UX 批次 B2）。
//
// 复现的缺陷：11 处输入框只有 placeholder（`sk-...` / 搜索词）当标签，读屏拿不到名称；
// 另有 7 处 `<label>` 与控件无 `htmlFor` 关联（`McpManager` ×4、`MarketInstallDialog` ×2、
// `PluginOverview` ×1）。placeholder 不是标签，悬停/输入后即消失。
//
// 本闸门钉住：每个 `<input>` 必须至少满足其一——`aria-label` / `aria-labelledby` /
// `id=`（配 `htmlFor`）/ 被 `<label>` 包裹 / `type="hidden"`（无 UI 语义）。
import { describe, expect, it } from "vitest"

const RAW_TSX = import.meta.glob("../**/*.tsx", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>

/** 自闭合尖括号会被 `=>` 误判为标签结束，故把 `=>` 当整体跳过。 */
const INPUT_TAG = /<input\b(?:(?:=>)|[^>])*>/gs
const HAS_NAME = /aria-label|aria-labelledby|\sid=|type="hidden"/

function lineOf(src: string, index: number): number {
  return src.slice(0, index).split("\n").length
}

/** 该位置是否被 `<label>…</label>` 包裹（取最近的开/闭标签判断）。 */
function insideLabel(src: string, index: number): boolean {
  const before = src.slice(0, index)
  return before.lastIndexOf("<label") > before.lastIndexOf("</label>")
}

function offendingInputs(src: string): string[] {
  const out: string[] = []
  for (const m of src.matchAll(INPUT_TAG)) {
    const tag = m[0]
    if (HAS_NAME.test(tag) || insideLabel(src, m.index)) continue
    out.push(`${tag.replace(/\s+/g, " ").slice(0, 80)}（行 ${lineOf(src, m.index)}）`)
  }
  return out
}

describe("表单控件无障碍名称闸门", () => {
  it("每个 input 都有 aria-label / aria-labelledby / id / label 包裹", () => {
    const offenders = Object.entries(RAW_TSX)
      .filter(([path]) => !path.endsWith("/formLabels.test.tsx"))
      .flatMap(([path, src]) => offendingInputs(src).map((d) => `${path}: ${d}`))

    expect(
      offenders,
      "这些 input 只有 placeholder 当标签——placeholder 不是无障碍名称",
    ).toEqual([])
  })

  it("闸门自身有效：能识别缺名称与合法写法（正反例）", () => {
    // 反例：只有 placeholder
    expect(offendingInputs(`<input placeholder="sk-..." value={x} />`)).toHaveLength(1)
    // 正例：aria-label / id / label 包裹 / hidden
    expect(offendingInputs(`<input aria-label="API Key" placeholder="sk-..." />`)).toEqual([])
    expect(offendingInputs(`<input id="mcp-name" placeholder="github" />`)).toEqual([])
    expect(offendingInputs(`<label><input type="checkbox" /> 勾选</label>`)).toEqual([])
    expect(offendingInputs(`<input type="hidden" value={x} />`)).toEqual([])
    // 正例：onChange 里的 `=>` 不得把标签截断（否则会漏检后续属性）
    expect(
      offendingInputs(`<input onChange={(e) => setX(e.target.value)} aria-label="搜索" />`),
    ).toEqual([])
  })
})
