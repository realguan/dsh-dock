// shapeTokens.test.ts —— 形状 / 高度 token 闸门（2026-09-10，UI/UX 批次 E）。
//
// 复现的缺陷：圆角 4 档并用且**同类元素分属两档**（卡片既有 24 处 rounded-2xl
// 也有大量 rounded-xl；8px 与 10px 肉眼无从分辨），阴影 6 档无递进逻辑
// （2xs 与 xs 几乎同值），且阴影是 Tailwind 默认的**纯黑**投影——压在冷灰底上发脏。
//
// 收口口径（index.css @theme）：圆角按**元素角色**分两档 + 胶囊；阴影覆写
// Tailwind 原生档位名（避免制造第三套命名），每档双层 + 冷色偏。
// 本闸门锁住「不得引入新的圆角/阴影档位或任意值」——需要新档先改 @theme。
//
// 注：`components/ui/*` 是 shadcn 预设组件（vendor 性质），其内部的小圆角
// （如 kbd 的 2px、badge 的 rounded-4xl 胶囊）不参与页面梯度，故豁免。
import { describe, expect, it } from "vitest"
import indexCss from "@/index.css?raw"

// 2026-09-10 复核修正：原只扫 .tsx（探针 .ts 可穿过）——见 contrast.test.ts 同注。
const RAW_TSX = import.meta.glob("../**/*.{ts,tsx}", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>

/** shadcn 预设组件（vendor 性质）：其内部小圆角不参与页面梯度。
 *  2026-09-10 复核收窄：原按**整个目录**豁免，但 `components/ui/confirm-dialog.tsx`
 *  是本仓库自有的业务组件（中文文案、删除确认语义），不该躲在 vendor 豁免里；
 *  `toast.tsx` 亦然。按**文件名**白名单，新增自有组件不会被自动豁免。 */
const VENDOR_FILES = new Set([
  "badge.tsx",
  "button.tsx",
  "dialog.tsx",
  "input.tsx",
  "popover.tsx",
  "progress.tsx",
  "select.tsx",
  "switch.tsx",
  "tabs.tsx",
  "tooltip.tsx",
])

/** 页面代码（排除 vendor 预设组件与闸门自身）。 */
function featureSources(): [string, string][] {
  return Object.entries(RAW_TSX).filter(([path]) => {
    if (path.endsWith("/shapeTokens.test.ts")) return false
    const m = path.match(/\/components\/ui\/([^/]+)$/)
    if (m) return !VENDOR_FILES.has(m[1])
    return true
  })
}

/** 允许的圆角档位：sm(8) / md=lg(10 控制件) / xl=2xl(14 面) / full(胶囊)。
 *  md 与 lg、xl 与 2xl 是**同值别名**，保留两个名字是为了让既有调用点零改动迁移，
 *  新代码按角色选用即可（控制件用 md 或 lg 均可，面用 xl 或 2xl 均可）。 */
const ALLOWED_RADIUS = new Set(["sm", "md", "lg", "xl", "2xl", "full", "none"])

// 裸 `rounded`（Tailwind 默认 4px）曾完全不被识别（正则要求 `-<档位>` 后缀）——
// 它是事实上的**第四档**，与小徽标/代码块的实际圆角不一致。2026-09-10 复核修正：
// 显式捕获裸 `rounded`（不含 `rounded-full` 等），要求改用 @theme 档位或 full。
const BARE_ROUNDED = /\brounded(?![-a-z0-9])/g
const RADIUS_CLASS = /\brounded(?:-(?:t|r|b|l|tl|tr|br|bl|s|e))?-([a-z0-9]+)\b/g
const ARBITRARY_RADIUS = /\brounded(?:-[trbl]{1,2})?-\[[^\]]+\]/g
/** 任意值阴影——含 `drop-shadow-[…]`（滤镜光晕同理须走 token）。 */
const ARBITRARY_SHADOW = /\b(?:drop-)?shadow-\[[^\]]+\]/g

describe("形状 / 高度 token 闸门", () => {
  it("页面代码不得使用 @theme 之外的圆角档位", () => {
    const offenders: string[] = []
    for (const [path, src] of featureSources()) {
      for (const m of src.matchAll(RADIUS_CLASS)) {
        if (!ALLOWED_RADIUS.has(m[1])) offenders.push(`${path}: ${m[0]}`)
      }
    }
    expect(
      offenders,
      "新圆角档位请先在 index.css 的 @theme 定义 --radius-*（当前梯度：控制件 10 / 面 14 / 胶囊 full）",
    ).toEqual([])
  })

  it("页面代码不得使用裸 `rounded`（隐式 4px 第四档）", () => {
    const offenders: string[] = []
    for (const [path, src] of featureSources()) {
      if (BARE_ROUNDED.test(src)) offenders.push(path)
      BARE_ROUNDED.lastIndex = 0
    }
    expect(
      offenders,
      "裸 `rounded` 是 Tailwind 默认 4px，属 @theme 之外的第四档——请用 rounded-md(控制件) / rounded-xl(面) / rounded-full(胶囊)",
    ).toEqual([])
  })

  it("页面代码不得使用任意值圆角", () => {
    const offenders: string[] = []
    for (const [path, src] of featureSources()) {
      for (const m of src.matchAll(ARBITRARY_RADIUS)) offenders.push(`${path}: ${m[0]}`)
    }
    expect(offenders, "任意值圆角请改为 @theme 档位").toEqual([])
  })

  it("页面代码不得使用任意值阴影（改用 2xs/xs/sm/md/lg/xl/2xl/inner 语义档）", () => {
    const offenders: string[] = []
    for (const [path, src] of featureSources()) {
      for (const m of src.matchAll(ARBITRARY_SHADOW)) offenders.push(`${path}: ${m[0]}`)
    }
    expect(
      offenders,
      "任意值阴影绕过了 elevation 语义——请用 @theme 的 shadow-* 档位",
    ).toEqual([])
  })

  it("elevation 必须带冷色偏（不用纯黑投影）", () => {
    // 纯黑投影压在冷灰底上观感发脏；统一用 ink 冷化后的 rgb(23,37,84)。
    const start = indexCss.indexOf("@theme {")
    const end = indexCss.indexOf("\n}", start)
    const block = indexCss.slice(start, end)
    const shadows = [...block.matchAll(/--shadow-[a-z0-9]+:\s*([^;]+);/g)].map((m) => m[1])
    expect(shadows.length, "未找到 shadow token").toBeGreaterThan(0)
    const bad = shadows.filter((v) => /rgba\(\s*0\s*,\s*0\s*,\s*0/.test(v))
    expect(bad, "阴影请用 rgba(23,37,84,…) 冷色偏，不用纯黑").toEqual([])
  })

  it("圆角两档必须真的收敛（md=lg 且 xl=2xl，且两档可辨）", () => {
    const start = indexCss.indexOf("@theme {")
    const end = indexCss.indexOf("\n}", start)
    const block = indexCss.slice(start, end)
    const px = (name: string) => {
      const m = block.match(new RegExp(`--radius-${name}:\\s*(\\d+)px`))
      if (!m) throw new Error(`index.css 缺少 --radius-${name}`)
      return Number(m[1])
    }
    expect(px("md"), "控制件档：md 与 lg 应同值").toBe(px("lg"))
    expect(px("xl"), "面档：xl 与 2xl 应同值").toBe(px("2xl"))
    expect(px("xl"), "控制件档与面档必须可辨").toBeGreaterThan(px("md"))
  })

  it("闸门自身有效：能识别越界档位与任意值、放行合法写法", () => {
    // 注意：带 /g 的 String.match 不返回捕获组，故自检用 matchAll 取第一组。
    const rad = (s: string) => [...s.matchAll(RADIUS_CLASS)][0]?.[1]
    expect(rad("rounded-3xl")).toBe("3xl")
    expect(ALLOWED_RADIUS.has("3xl")).toBe(false)
    expect("rounded-[7px]".match(ARBITRARY_RADIUS)).toHaveLength(1)
    expect(rad("rounded-tl-md")).toBe("md")
    expect("rounded".match(BARE_ROUNDED)).toHaveLength(1)
    expect("rounded-full".match(BARE_ROUNDED)).toBeNull()
    expect("rounded-md".match(BARE_ROUNDED)).toBeNull()
    expect("shadow-[0_0_8px_red]".match(ARBITRARY_SHADOW)).toHaveLength(1)
    expect("drop-shadow-[0_1px_2px_red]".match(ARBITRARY_SHADOW)).toHaveLength(1)
    expect("shadow-2xs".match(ARBITRARY_SHADOW)).toBeNull()
    expect("shadow-glow".match(ARBITRARY_SHADOW)).toBeNull()
    expect("drop-shadow-glow".match(ARBITRARY_SHADOW)).toBeNull()
  })
})
