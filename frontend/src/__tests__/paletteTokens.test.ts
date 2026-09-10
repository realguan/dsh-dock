// paletteTokens.test.ts —— 调色板 token 收口闸门（2026-09-10，UI/UX 批次 E）。
//
// 复现的缺陷：`index.css` 定义了一套语义 token（ok/warn/danger…），但全仓有
// **200 处**直接使用 Tailwind 原生调色板绕过它——
//   · ok(28) vs emerald(46)：同一件「成功/运行中」有两个绿；
//   · warn(41) vs amber(59)：同一个「警告」有两个橙；
//   · rose(45) 被当成「NPM 官方包」的分类色，而它在别处表示删除/失败；
//   · purple(9) + violet(6) + indigo(3) 三个紫混编同一类「分类」语义。
// 观感「散」的根因不是配色不好，是同一件事有 2–3 个色。
//
// 本闸门把「状态/分类/深色面一律走 token」变成可执行约束：新写原生调色板即红。
// 判据是**语义**而非美观——Tailwind 原生档位没有语义，无法统一收缩。
import { describe, expect, it } from "vitest"

const RAW_SOURCES = import.meta.glob("../**/*.{ts,tsx,css,js}", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>

/** index.html 是 SPA 入口，不在上面 glob 的范围内（在 frontend/ 根而非 src/）。 */
const INDEX_HTML = import.meta.glob("../../index.html", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>

/** 被 token 取代的原生色系（收口范围）。
 *  rose/amber/emerald/sky = 状态四族；purple/violet/indigo = 分类档；
 *  slate = 深色终端面。gray/zinc/neutral/stone 一并禁——它们是「没有语义的灰」，
 *  灰阶一律用 ink/dim/faint/line/line-soft。 */
const BANNED_HUES = [
  "rose",
  "red",
  "amber",
  "yellow",
  "orange",
  "emerald",
  "green",
  "lime",
  "teal",
  "cyan",
  "sky",
  "blue",
  "indigo",
  "violet",
  "purple",
  "fuchsia",
  "pink",
  "slate",
  "gray",
  "zinc",
  "neutral",
  "stone",
]

// 前缀表补全（2026-09-10 复核）：原表漏了**方向性边框**（border-s/e/t/r/b/l/x/y）、
// ring-offset、inset-shadow、text-shadow——探针 `border-s-rose-500` 与
// `ring-offset-amber-500` 原本可以穿过。
const BANNED = new RegExp(
  String.raw`\b(?:bg|text|border|border-[sexytrbl]|ring|ring-offset|from|to|via|shadow|inset-shadow|text-shadow|divide|fill|stroke|decoration|outline|accent|caret|placeholder)-(?:${BANNED_HUES.join("|")})-(?:[0-9]{2,3})\b`,
  "g",
)

/** 任意值色（Tailwind arbitrary value）：`bg-[#047857]` / `text-[rgb(...)]` /
 *  `bg-[oklch(...)]`——探针实测原闸门全部放行。注意排除 `color-mix(...var(--color-*))`
 *  这类**令牌引用**（那是合法的）。 */
const ARBITRARY_COLOR =
  /\b(?:bg|text|border|ring|from|to|via|shadow|divide|fill|stroke|decoration|outline|placeholder)-\[[^\]]*(?:#[0-9a-fA-F]{3,8}|rgba?\(|hsla?\(|oklch\(|oklab\(|lch\(|lab\()[^\]]*\]/g

/** 裸的十六进制 / rgb() 字面量（CSS 文件与 TS/TSX 里的字符串）。
 *  @theme 块内的 token 定义是唯一合法来源，由下方用例排除。 */
const RAW_COLOR_LITERAL = /(?:#[0-9a-fA-F]{3,8}\b|\brgba?\(\s*\d+\s*,\s*\d+\s*,\s*\d+)/g

/** 豁免清单（按文件路径后缀匹配）。2026-09-10 批次 E 收尾时已清空——
 *  `components/ui/toast.tsx` 曾在此豁免（理由：深色胶囊自带独立配色），
 *  但复核发现它的三种 kind 恰是**语义**的（ok/warn/info）且本就是深色面，
 *  正是 `term-*` 族的用途，故收编入 token 体系并删除豁免。
 *  保留此机制供将来确有需要时登记——但请优先问「它是不是也该用 token」。 */
const EXEMPT_PATHS: string[] = []

function isExempt(path: string): boolean {
  return EXEMPT_PATHS.some((p) => path.endsWith(p))
}

/** 取 index.css 的 `@theme { … }` 块正文。 */
function themeBlockOf(cssSrc: string): string {
  const start = cssSrc.indexOf("@theme {")
  const end = cssSrc.indexOf("\n}", start)
  if (start < 0 || end < 0) throw new Error("index.css 缺少完整的 @theme 块")
  return cssSrc.slice(start, end)
}

describe("调色板 token 收口闸门", () => {
  it("源码不得使用 Tailwind 原生调色板档位（一律走语义 token）", () => {
    const offenders = Object.entries(RAW_SOURCES)
      .filter(([path]) => !path.endsWith("/paletteTokens.test.ts"))
      .filter(([path]) => !isExempt(path))
      .flatMap(([path, src]) => {
        const hits = [...src.matchAll(BANNED)].map((m) => m[0])
        return hits.length ? [`${path}: ${[...new Set(hits)].join(" ")}`] : []
      })

    expect(
      offenders,
      [
        "这些位置绕过了语义 token。对照表：",
        "  状态四族 → text-ok / text-info / text-warn / text-danger（+ bg-<族>-soft）",
        "  分类档   → text-alt / bg-alt-soft（来源、排序、分组等语义中性的区分）",
        "  深色面   → bg-term / bg-term-panel / text-term-ink / text-term-dim / text-term-faint",
        "             日志级别 → text-term-ok / -info / -warn / -danger",
        "  灰阶     → text-ink / text-dim / text-faint / border-line / bg-line-soft",
        "需要新的语义色，请先在 index.css 的 @theme 加 token 并同步 contrast.test.ts。",
      ].join("\n"),
    ).toEqual([])
  })

  it("闸门自身有效：能识别原生档位、放行 token 写法", () => {
    expect("text-emerald-700".match(BANNED)).toHaveLength(1)
    expect("bg-amber-500/10".match(BANNED)).toHaveLength(1)
    expect("border-slate-800".match(BANNED)).toHaveLength(1)
    // 变体前缀与任意值也必须拦下（2026-09-10 实测探针验证）
    expect("hover:bg-purple-500".match(BANNED)).toHaveLength(1)
    expect("md:text-sky-700".match(BANNED)).toHaveLength(1)
    expect("data-[state=open]:text-teal-700".match(BANNED)).toHaveLength(1)
    expect("bg-emerald-500/[0.07]".match(BANNED)).toHaveLength(1)
    // 2026-09-10 复核补：方向性边框与 ring-offset 原可穿过
    expect("border-s-rose-500".match(BANNED)).toHaveLength(1)
    expect("ring-offset-amber-500".match(BANNED)).toHaveLength(1)
    expect("border-b-emerald-600".match(BANNED)).toHaveLength(1)
    expect("border-line".match(BANNED)).toBeNull()
    expect("text-ok".match(BANNED)).toBeNull()
    expect("bg-term-panel".match(BANNED)).toBeNull()
    expect("text-dim".match(BANNED)).toBeNull()
  })

  it("不得使用任意值颜色（bg-[#047857] / text-[rgb(...)] / bg-[oklch(...)]）", () => {
    // 探针实测（2026-09-10 复核）：原闸门只认「色系-档位」形态，
    // `bg-[#047857]`、`text-[rgb(4,120,87)]`、`bg-[oklch(0.5_0.1_150)]` 全部放行。
    const offenders = Object.entries(RAW_SOURCES)
      .filter(([path]) => !path.endsWith("/paletteTokens.test.ts"))
      .filter(([path]) => !isExempt(path))
      .flatMap(([path, src]) => {
        const hits = [...src.matchAll(ARBITRARY_COLOR)].map((m) => m[0])
        return hits.length ? [`${path}: ${[...new Set(hits)].join(" ")}`] : []
      })
    expect(offenders, "任意值颜色绕过 token——请改用 @theme 语义 token").toEqual([])
  })

  it("源码里的裸色值只能是 index.css 的 @theme 定义（或白/透明）", () => {
    // 探针实测：普通 .css 里写 `#ef4444` / `border-radius:7px` 无人拦；
    // `style={{ color: "tomato" }}` 一类具名色同样漏网（故同时禁任意值类名）。
    // 合法来源 = index.css 的 @theme 块（token 定义）与 index.html 的首帧底色
    // （由专门用例与 Rust 侧双向锁定）。其余位置出现裸色值即红。
    const offenders: string[] = []
    for (const [path, src] of Object.entries(RAW_SOURCES)) {
      // 测试文件以色值为 fixture 做断言（如 contrast.test.ts 内联反例），跳过。
      if (/\.test\.tsx?$/.test(path)) continue
      if (path.endsWith("/index.css")) continue // token 定义源本身
      // 注入脚本：跨文档拿不到壳的 CSS 变量，只能镜像硬编码——
      // 由「幕布/胶囊镜像色值必须与 token 同步」的专门用例逐值锁定。
      if (path.endsWith("/injected/handoff-curtain.js")) continue
      if (path.endsWith("/injected/switcher.js")) continue
      for (const m of src.matchAll(RAW_COLOR_LITERAL)) {
        const lit = m[0].toLowerCase()
        // 白与纯黑（阴影/遮罩/mask 用）不属调色板语义，放行。
        if (lit === "#ffffff" || lit === "#fff" || lit === "#000000" || lit === "#000") continue
        offenders.push(`${path}: ${m[0]} @${m.index}`)
      }
    }
    expect(
      offenders,
      "裸色值绕过 token 体系——请改用 @theme token（需要新色先加 token 并同步 contrast.test.ts）",
    ).toEqual([])
  })

  it("不得用内联 style 绕过 token（类名闸门的盲区）", () => {
    // 探针实测：`style={{ color: "#047857" }}`、`style={{ color: "tomato" }}`
    // 不会被类名正则命中。上一条已覆盖裸字面量；本条额外禁内联 style 里的
    // **具名色/函数色**（tomato / currentcolor 之外的 CSS 颜色关键字）。
    const INLINE_COLOR =
      /style=\{\{[^}]*(?:#[0-9a-fA-F]{3,8}|\b(?:rgba?|hsla?|oklch|oklab|lch|lab|color-mix|color)\()/
    const offenders = Object.entries(RAW_SOURCES)
      .filter(([path]) => !path.endsWith("/paletteTokens.test.ts"))
      .filter(([path]) => !isExempt(path))
      .filter(([, src]) => INLINE_COLOR.test(src))
      .map(([path]) => path)
    expect(offenders, "内联 style 里的色值绕过了 token 体系——请改用 token 类名").toEqual([])
  })

  it("冷启动底色：index.html 首帧与 --color-bg 逐值一致", () => {
    // 「冷启动无闪色」要求首帧 HTML 底色 = CSS --color-bg = 原生窗口 background_color。
    // 2026-09-10 批次 E 发现这三处**长期不一致**（HTML/CSS 是 #f7f8fb，
    // ui.rs 是 #f9fafb），注释宣称同调而事实不符。
    // 本闸门覆盖前两处；第三处（ui.rs）由 Rust 侧
    // `ui::window_background_tests` 从同一份 index.css 解析比对——前端测试无法
    // 读取 frontend/ 之外的源码，故拆两处而非强行合并。
    const html = Object.entries(INDEX_HTML).find(([path]) =>
      path.endsWith("/index.html"),
    )
    const css = Object.entries(RAW_SOURCES).find(([path]) => path.endsWith("/index.css"))
    expect(html, "未找到 index.html").toBeDefined()
    expect(css, "未找到 index.css").toBeDefined()
    const htmlHex = html![1].match(/html\s*\{[^}]*background:\s*(#[0-9a-fA-F]{6})/)?.[1]
    const cssHex = css![1]
      .slice(css![1].indexOf("@theme {"))
      .match(/--color-bg:\s*(#[0-9a-fA-F]{6})/)?.[1]
    expect(htmlHex, "index.html 首帧底色未取到").toBeDefined()
    expect(cssHex, "index.css --color-bg 未取到").toBeDefined()
    expect(
      htmlHex!.toLowerCase(),
      "index.html 首帧底色与 --color-bg 不一致——WebView 首帧会闪色",
    ).toBe(cssHex!.toLowerCase())
  })

  it("shadcn 语义层由 :root 映射到 token，不得再手抄 hex", () => {
    // 旧口径 :root 把 bg/ink/line/brand 又硬编码了一遍（第三份真相源）。
    const indexCss = Object.entries(RAW_SOURCES).find(([path]) =>
      path.endsWith("/index.css"),
    )
    expect(indexCss, "未找到 index.css 源码").toBeDefined()
    const [, src] = indexCss!
    const rootStart = src.indexOf(":root {")
    const rootEnd = src.indexOf("\n}", rootStart)
    const rootBlock = src.slice(rootStart, rootEnd)
    const hexes = [...rootBlock.matchAll(/#[0-9a-fA-F]{6}/g)].map((m) => m[0])
    // 仅 --primary-foreground / --destructive-foreground = #ffffff 允许直写
    const offenders = hexes.filter((h) => h.toLowerCase() !== "#ffffff")
    expect(offenders, ":root 请用 var(--color-*) 引用 @theme token").toEqual([])
  })

  it("注入脚本的色值必须与 index.css token 同步（跨文档无法用变量）", () => {
    // 注入脚本（幕布 + 悬浮胶囊）跑在 dsh 文档里，拿不到壳的 CSS 变量，只能镜像
    // 硬编码。本条闸门把它们钉在 token 上：改 token 忘改镜像 = 测试红。
    // 2026-09-10 复核补：原只锁 handoff-curtain.js，而 switcher.js（用户可见的
    // 悬浮胶囊）整个漏在收口之外——它当时仍是收口前的 blue/zinc 原始调色板。
    const indexCss = Object.entries(RAW_SOURCES).find(([path]) =>
      path.endsWith("/index.css"),
    )
    const curtain = Object.entries(RAW_SOURCES).find(([path]) =>
      path.endsWith("/injected/handoff-curtain.js"),
    )
    expect(indexCss, "未找到 index.css").toBeDefined()
    expect(curtain, "未找到 handoff-curtain.js").toBeDefined()
    const [, cssSrc] = indexCss!
    const [, curtainSrc] = curtain!

    const themeBlock = themeBlockOf(cssSrc)
    const token = (name: string) => {
      const m = themeBlock.match(new RegExp(`--color-${name}:\\s*(#[0-9a-fA-F]{6})`))
      if (!m) throw new Error(`index.css 缺少 --color-${name}`)
      return m[1]
    }
    // 幕布镜像的 token（浅色面 + 暗色面各一组）
    const mirrored = [
      "bg",
      "ink",
      "dim",
      "faint",
      "line",
      "line-soft",
      "brand",
      "term",
      "term-panel",
      "term-line",
      "term-ink",
      "term-dim",
    ]
    const missing = mirrored.filter((name) => !curtainSrc.includes(token(name)))
    expect(
      missing.map((n) => `${n}=${token(n)}`),
      "幕布脚本缺少这些 token 的镜像色值——请同步 handoff-curtain.js 的 css()",
    ).toEqual([])

    // 悬浮胶囊（switcher.js）镜像同样逐值锁定。
    const switcher = Object.entries(RAW_SOURCES).find(([path]) =>
      path.endsWith("/injected/switcher.js"),
    )
    expect(switcher, "未找到 switcher.js").toBeDefined()
    const [, switcherSrc] = switcher!
    // 胶囊是毛玻璃半透明件，部分色以 rgba(r,g,b,a) 形式表达——等价接受。
    const asRgbTriplet = (hex: string) =>
      [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)).join(",")
    const mirroredIn = (src: string, hex: string) =>
      src.includes(hex) || src.replace(/\s/g, "").includes(asRgbTriplet(hex))
    // 不含 `bg`：胶囊是浮在 dsh 页面上的半透明件，不承载页面底色。
    const capMirrored = ["ink", "line", "line-soft", "faint", "brand", "term-panel", "term-line", "term-ink", "term-dim"]
    const capMissing = capMirrored.filter((name) => !mirroredIn(switcherSrc, token(name)))
    expect(
      capMissing.map((n) => `${n}=${token(n)}`),
      "悬浮胶囊缺少这些 token 的镜像色值——请同步 switcher.js 的胶囊 CSS",
    ).toEqual([])
    // term-brand 是非 hex token 值之外唯一的例外（品牌蓝深底档），单独校验。
    const termBrand = themeBlockOf(cssSrc).match(/--color-term-brand:\s*(#[0-9a-fA-F]{6})/)?.[1]
    expect(termBrand, "index.css 缺少 --color-term-brand").toBeDefined()
    expect(switcherSrc, "悬浮胶囊缺少 term-brand 镜像").toContain(termBrand!)
  })
})
