// contrast.test.ts —— 设计 token 对比度 / 可辨识度闸门。
//
// 前史：2026-09-08 批次 C（U6）建立本闸门——`--color-faint` 在 bg/panel/line-soft
// 上只有 2.13–2.41，远低于 WCAG AA 4.5:1；`--color-ok`/`--color-warn` 在白底
// 3.45/4.30 不达标；白字压品牌蓝 4.23 不达标。
//
// 2026-09-10 批次 E（token 收口）扩为四项闸门，起因是「token 都不好看」的
// 实测根因 = token 体系被绕过：
//   ① 对比度：文字色在三**种**底色（bg/panel/line-soft）上均须 ≥4.5；
//   ② 派生一致性：*-soft 的 RGB 必须等于同族基色（旧 ok-soft 手抄 RGB，
//      改基色即静默失联）；
//   ③ 族间可辨识度：状态族两两 ΔOKLab ≥0.12——这是「同一个世界」的
//      可执行判据，也是 warn×danger 必须靠明度差而非色相拉开的依据；
//   ④ 灰阶阶梯：ink/dim/faint 相邻两档 ΔOKLab ≥0.12，否则「三级」是假的
//      （旧 dim↔faint 仅 0.028 = 一级）。
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

/** sRGB hex → OKLab（Björn Ottosson 原始矩阵）。用于「两色能否分辨」判定。 */
function oklab(hex: string): [number, number, number] {
  const c = hex.replace("#", "")
  const [r, g, b] = [0, 2, 4].map((i) => {
    const v = parseInt(c.slice(i, i + 2), 16) / 255
    return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4
  })
  const l = Math.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b)
  const m = Math.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b)
  const s = Math.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b)
  return [
    0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s,
    1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s,
    0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s,
  ]
}

/** 两色的感知距离（ΔOKLab）。经验阈值 0.12 ≈ 并排可辨。 */
function deltaOk(a: string, b: string): number {
  const [l1, a1, b1] = oklab(a)
  const [l2, a2, b2] = oklab(b)
  return Math.hypot(l1 - l2, a1 - a2, b1 - b2)
}

/** rgba(r, g, b, a) 叠在底色上的合成结果（用于算 *-soft 上的文字对比度）。 */
function composite(rgba: string, base: string): string {
  const m = rgba.match(/rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*(?:,\s*([\d.]+))?\)/)
  if (!m) throw new Error(`无法解析颜色：${rgba}`)
  const a = m[4] === undefined ? 1 : parseFloat(m[4])
  const bg = [0, 2, 4].map((i) => parseInt(base.replace("#", "").slice(i, i + 2), 16))
  return (
    "#" +
    [0, 1, 2]
      .map((i) => Math.round(parseInt(m[i + 1]) * a + bg[i] * (1 - a)))
      .map((v) => v.toString(16).padStart(2, "0"))
      .join("")
  )
}

/** 从 index.css 的 `@theme { … }` 块取 `--color-<name>: <值>`（hex 或 rgba）。 */
function themeTokens(): Record<string, string> {
  const start = indexCss.indexOf("@theme {")
  const end = indexCss.indexOf("\n}", start)
  const block = indexCss.slice(start, end)
  const out: Record<string, string> = {}
  for (const m of block.matchAll(/--color-([a-z-]+):\s*(#[0-9a-fA-F]{6}|rgba?\([^)]*\))/g)) {
    out[m[1]] = m[2]
  }
  return out
}

const T = themeTokens()
const AA = 4.5
/** 三种文字承载底色：页面底 / 卡片面 / 凹陷填充（旧闸门只测前两种，漏了第三种）。 */
const SURFACES = ["bg", "panel", "line-soft"]

/** 允许当**文字色**的 token（浅色三种底色上都必须 ≥ 4.5）。
 *  深色终端面自成一套底色，在下方专项里对 `term` 校验。 */
const TEXT_TOKENS = ["ink", "dim", "faint", "brand-deep", "ok", "info", "warn", "danger", "alt"]
const TERM_TEXT_TOKENS = ["term-ink", "term-dim", "term-faint"]
/** 承载白字的填充色。 */
const FILL_TOKENS = ["ink", "brand-deep", "warn", "ok", "info", "danger", "alt"]
/** 状态族 + 分类档（用于族间可辨识度与 soft 派生校验）。 */
const FAMILIES = ["ok", "info", "warn", "danger", "alt"]

describe("设计 token 对比度闸门", () => {
  it("文字色 token 在 bg / panel / line-soft 三种底色上均达到 WCAG AA 4.5:1", () => {
    const failures: string[] = []
    for (const name of TEXT_TOKENS) {
      for (const bg of SURFACES) {
        const ratio = contrast(T[name], T[bg])
        if (ratio < AA) {
          failures.push(`${name}(${T[name]}) on ${bg}(${T[bg]}) = ${ratio.toFixed(2)}`)
        }
      }
    }
    expect(failures, "文字色对比度不足——换 token 或调色后同步本表").toEqual([])
  })

  it("深色终端面的文字色在 term 底上达到 WCAG AA 4.5:1", () => {
    const failures: string[] = []
    for (const name of TERM_TEXT_TOKENS) {
      const ratio = contrast(T[name], T.term)
      if (ratio < AA) failures.push(`${name}(${T[name]}) on term = ${ratio.toFixed(2)}`)
    }
    expect(failures).toEqual([])
  })

  it("深色终端面的日志级别色在 term 底上达到 4.5:1", () => {
    const failures: string[] = []
    for (const name of ["term-ok", "term-info", "term-warn", "term-danger"]) {
      const ratio = contrast(T[name], T.term)
      if (ratio < AA) failures.push(`${name}(${T[name]}) on term = ${ratio.toFixed(2)}`)
    }
    expect(failures).toEqual([])
  })

  it("白字在填充色 token 上达到 WCAG AA 4.5:1", () => {
    const failures: string[] = []
    for (const name of FILL_TOKENS) {
      const ratio = contrast("#ffffff", T[name])
      if (ratio < AA) failures.push(`white on ${name}(${T[name]}) = ${ratio.toFixed(2)}`)
    }
    expect(failures).toEqual([])
  })

  it("状态族文字在本族 soft 填充底上也达到 4.5:1（徽标实景）", () => {
    const failures: string[] = []
    for (const name of FAMILIES) {
      const onSoft = composite(T[`${name}-soft`], T.panel)
      const ratio = contrast(T[name], onSoft)
      if (ratio < AA) {
        failures.push(`${name}(${T[name]}) on ${name}-soft(${onSoft}) = ${ratio.toFixed(2)}`)
      }
    }
    expect(failures, "族色压在自家 soft 底上不达标——徽标文字会看不清").toEqual([])
  })

  it("*-soft 的 RGB 三元组必须等于同族基色（防手抄失联）", () => {
    // 旧 `--color-ok-soft: rgba(39,121,58,.1)` 与 ok 的 RGB 各写一遍：
    // 改 ok 而忘改 soft 不会报错，只会静默错色。本闸门令其不可能。
    const failures: string[] = []
    const rgb = (hex: string) =>
      [0, 2, 4].map((i) => parseInt(hex.replace("#", "").slice(i, i + 2), 16))
    for (const name of FAMILIES) {
      // 解析 soft 的 RGB 逐通道比对——不靠字符串拼接，避免逗号空格写法差异。
      const m = T[`${name}-soft`].match(/rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)/)
      const softRgb = m ? [Number(m[1]), Number(m[2]), Number(m[3])] : null
      const baseRgb = rgb(T[name])
      if (!softRgb || softRgb.some((v, i) => v !== baseRgb[i])) {
        failures.push(`${name}-soft(${T[`${name}-soft`]}) 与 ${name}(${T[name]}) 不同源`)
      }
    }
    expect(failures, "soft 必须由基色派生——请让 RGB 与基色一致").toEqual([])
  })

  it("状态族两两可辨识：ΔOKLab ≥ 0.12", () => {
    const failures: string[] = []
    for (let i = 0; i < FAMILIES.length; i++) {
      for (let j = i + 1; j < FAMILIES.length; j++) {
        const a = FAMILIES[i]
        const b = FAMILIES[j]
        const d = deltaOk(T[a], T[b])
        if (d < 0.12) failures.push(`${a}×${b} = ${d.toFixed(3)}`)
      }
    }
    expect(failures, "族色太接近——并排时会被读成同一个色").toEqual([])
  })

  it("品牌蓝文字档与状态族保持可辨识（ΔOKLab ≥ 0.12）", () => {
    const failures: string[] = []
    for (const name of FAMILIES) {
      const d = deltaOk(T["brand-deep"], T[name])
      if (d < 0.12) failures.push(`brand-deep×${name} = ${d.toFixed(3)}`)
    }
    expect(failures, "brand-deep 与某族太近——品牌色与状态色会互相冒充").toEqual([])
  })

  it("灰阶真三级：ink>dim>faint 相邻两档 ΔOKLab ≥ 0.12", () => {
    // 旧 dim↔faint = 0.028，号称两级实为一级——「字都糊在一起」的根因。
    const ladder = ["ink", "dim", "faint"]
    const failures: string[] = []
    for (let i = 0; i < ladder.length - 1; i++) {
      const d = deltaOk(T[ladder[i]], T[ladder[i + 1]])
      if (d < 0.12) failures.push(`${ladder[i]}×${ladder[i + 1]} = ${d.toFixed(3)}`)
    }
    expect(failures, "灰阶档间距不足——三级灰阶退化为两级").toEqual([])
  })

  it("底色三档两两可辨识（bg / line-soft / line）", () => {
    // 旧 line-soft(#eef1f6) 与新 bg(#f1f4f9) 只差 1.027 —— 凹陷轨道几乎隐形。
    const failures: string[] = []
    const pairs: [string, string][] = [
      ["panel", "bg"],
      ["bg", "line-soft"],
      ["line-soft", "line"],
      ["panel", "line"],
    ]
    for (const [a, b] of pairs) {
      const d = deltaOk(T[a], T[b])
      if (d < 0.012) failures.push(`${a}×${b} = ${d.toFixed(4)}`)
    }
    expect(failures, "底色分不出层——卡片/凹陷轨道会失去边界").toEqual([])
  })

  it("已知边际：白字在 brand 填充档上 ≥ 4.2（2026-09-10 维护者裁定记录在案）", () => {
    // 批次 C 曾把填充色也一并加深到 brand-deep(#3163cf)，观感偏暗被维护者退回：
    // 填充/图形象回到 brand(#4176e6)，文字档保持 brand-deep。白字压 brand = 4.23，
    // 低于 AA 4.5（控件填充属 UI 组件，AA 要求 ≥3:1），故按记录在案口径处理：
    // 只允许用于「填充 + 白字」的控件，不得下沉为文字色（文字由 brand-deep 兜底）。
    const ratio = contrast("#ffffff", T.brand)
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
      "brand(#4176e6) 在白底仅 4.23——文字一律用 brand-deep(#3163cf, 5.31)",
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
