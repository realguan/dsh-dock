// contrast.test.ts —— 设计 token 对比度闸门（2026-09-08，UI/UX 批次 C / U6）。
//
// 复现的缺陷：`--color-faint` 在 bg/panel/line-soft 上只有 2.13–2.41，远低于
// WCAG AA 的 4.5:1（正文）；`--color-ok`/`--color-warn` 在白底 3.45/4.30 也不达标；
// 白字压在品牌蓝 `--color-brand` 上 4.23 同样不达标。
//
// 本闸门把「哪个 token 允许当文字色 / 填充底色」变成可执行约束：
// 改色即验算，低于阈值直接红——不再靠人肉目测。
import { describe, expect, it } from "vitest"
import indexCss from "@/index.css?raw"

const RAW_TSX = import.meta.glob("../**/*.tsx", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>

/** WCAG 相对亮度。 */
function luminance(hex: string): number {
  const c = hex.replace("#", "")
  const parts = [0, 2, 4].map((i) => parseInt(c.slice(i, i + 2), 16) / 255)
  const lin = parts.map((x) => (x <= 0.04045 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4))
  return 0.2126 * lin[0] + 0.7152 * lin[1] + 0.0722 * lin[2]
}

function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x)
  return (hi + 0.05) / (lo + 0.05)
}

/** 从 index.css 的第一个 `@theme { … }` 块取 `--color-<name>: <hex>`。 */
function themeTokens(): Record<string, string> {
  const start = indexCss.indexOf("@theme {")
  const end = indexCss.indexOf("\n}", start)
  const block = indexCss.slice(start, end)
  const out: Record<string, string> = {}
  for (const m of block.matchAll(/--color-([a-z-]+):\s*(#[0-9a-fA-F]{6})/g)) {
    out[m[1]] = m[2]
  }
  return out
}

const T = themeTokens()
const AA = 4.5

/** 允许当**文字色**的 token（bg / panel 两种主底色上都必须 ≥ 4.5）。 */
const TEXT_TOKENS = ["ink", "dim", "faint", "brand-deep", "warn", "ok"]
/** 允许**承载白字**的填充色。 */
const FILL_TOKENS = ["ink", "brand-deep", "warn", "ok"]

describe("设计 token 对比度闸门", () => {
  it("文字色 token 在 bg / panel 上均达到 WCAG AA 4.5:1", () => {
    const failures: string[] = []
    for (const name of TEXT_TOKENS) {
      for (const bg of ["bg", "panel"]) {
        const ratio = contrast(T[name], T[bg])
        if (ratio < AA) {
          failures.push(`${name}(${T[name]}) on ${bg}(${T[bg]}) = ${ratio.toFixed(2)}`)
        }
      }
    }
    expect(failures, "文字色对比度不足——换 token 或调色后同步本表").toEqual([])
  })

  it("白字在填充色 token 上达到 WCAG AA 4.5:1", () => {
    const failures: string[] = []
    for (const name of FILL_TOKENS) {
      const ratio = contrast("#ffffff", T[name])
      if (ratio < AA) failures.push(`white on ${name}(${T[name]}) = ${ratio.toFixed(2)}`)
    }
    expect(failures).toEqual([])
  })

  it("已知边际：faint 在 line-soft（浅色填充底）上 ≥ 4.2（记录在案，不静默放宽）", () => {
    // 三级灰阶在浅底上无法同时满足「≥4.5」与「与 dim 可区分」，故取两级 + 本边际值。
    const ratio = contrast(T.faint, T["line-soft"])
    expect(ratio).toBeGreaterThanOrEqual(4.2)
    expect(ratio).toBeLessThan(AA)
  })

  it("品牌蓝只作图形象：源码不得再把 text-brand 当文字色（一律 text-brand-deep）", () => {
    const offenders = Object.entries(RAW_TSX)
      .filter(([path]) => !path.endsWith("/contrast.test.ts"))
      .filter(([, src]) => /text-brand(?!-deep)/.test(src))
      .map(([path]) => path)
    expect(
      offenders,
      "brand(#4176e6) 在白底仅 4.23——文字一律用 brand-deep(#3163cf, 5.50)",
    ).toEqual([])
  })

  it("不得再出现 dark: 变体（.dark 从未被应用 = 死代码 + 暗色地雷）", () => {
    // 暗色方案已定（index.css：追加 `.dark` 覆盖语义变量，组件零改动）；
    // 散落的 `dark:` 变体既不可达，将来一旦启用又会用原始调色板破坏 token 体系。
    const offenders = Object.entries(RAW_TSX)
      .filter(([path]) => !path.endsWith("/contrast.test.ts"))
      .filter(([, src]) => /(?:^|[\s"'`])dark:/.test(src))
      .map(([path]) => path)
    expect(offenders, "要做暗色请覆盖 @theme 语义变量，不要散落 dark: 变体").toEqual([])
  })
})
