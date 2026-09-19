// cnFontScale.test.ts —— 字号 token 与 tailwind-merge 的分组闸门（2026-09-19）。
//
// 复现的缺陷：MCP 弹窗 InfoTip 渲染成整块乌漆麻黑——`cn()` 里 tailwind-merge 把
// 本仓库的 `text-label`（字号）误判成**文字颜色**，于是把同一元素上真正的颜色类
// `text-background` 当冲突项删掉 ⇒ ink 底 + ink 字。源码看着没毛病、类名合法、
// typecheck/lint/contrast 全绿，只有渲染后才看得见。
//
// 两道闸门：
// ① 行为闸门——字号档与文字颜色必须共存（这条直接钉住"合并即消失"）；
// ② 同步闸门——`index.css` 每新增一个 `--text-*` 档位，`lib/utils.ts` 的
//    font-size 组必须登记，否则该档名又会落进颜色兜底组。
import { describe, expect, it } from "vitest"

import { cn } from "@/lib/utils"

const RAW_SOURCES = import.meta.glob("../**/*.{ts,tsx,css}", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>

const readRaw = (suffix: string) => {
  const hit = Object.entries(RAW_SOURCES).find(([path]) => path.endsWith(suffix))
  expect(hit, `未找到 ${suffix} 源码`).toBeDefined()
  return hit![1]
}

/**
 * Tailwind v4 的 `--text-<档>--<修饰>` 是**成对修饰键**（行高/字距/字重…），
 * 不是档位。正则的 `[\w-]+` 会把 `micro--line-height` 整段吞进来，故须剥后缀。
 * 2026-09-19 字号阶梯首次成对声明行高时暴露出这个解析缺陷。
 */
const PAIRED_MODIFIER =
  /--(line-height|letter-spacing|font-weight|font-feature-settings|variation-settings)$/

/** index.css 里真实声明的字号档位名（已排除成对修饰键）。 */
const textRungs = (indexCss: string) =>
  [...indexCss.matchAll(/^\s*--text-([\w-]+):/gm)]
    .map((m) => m[1])
    .filter((step) => !PAIRED_MODIFIER.test(step))

describe("cn() 字号档与颜色不互斥", () => {
  it("深色面上成对的 bg/text token 不因调用方传字号而消失", () => {
    const merged = cn("z-50 bg-term text-term-ink shadow-lg", "text-label")
    expect(merged).toContain("bg-term")
    expect(merged).toContain("text-term-ink")
    expect(merged).toContain("text-label")
  })

  it("回归锚点：text-background 不得被 text-label 挤掉（乌漆麻黑那一次）", () => {
    const merged = cn("rounded-md bg-foreground text-xs text-background", "text-label")
    expect(merged).toContain("text-background")
    expect(merged).toContain("text-label")
  })

  it("同族颜色仍按后来的覆盖（闸门不是把 merge 关掉了）", () => {
    expect(cn("text-term-ink", "text-dim")).not.toContain("text-term-ink")
    expect(cn("text-note", "text-label")).not.toContain("text-note")
  })

  it("index.css 的每个 --text-* 档位都登记进 utils 的 font-size 组", () => {
    const scale = textRungs(readRaw("/index.css"))
    expect(scale.length, "未从 index.css 读到字号刻度").toBeGreaterThan(0)

    const registered = readRaw("/lib/utils.ts")
    for (const step of scale) {
      expect(
        registered.includes(`"${step}"`),
        `字号档位 text-${step} 未登记进 tailwind-merge 的 font-size 组——` +
          `它会被当成文字颜色，与同元素的真颜色类互斥`,
      ).toBe(true)
    }
  })

  // 2026-09-19 审美批次 1 新增：字号与行高**必须成对**。
  // Tailwind v4 改 `--text-xs` 的值不会重算它原生的 `--text-xs--line-height`
  // （xs 的默认行高是 16px）。只抬字号不写行高，13.5px 的中文正文会继续吃
  // 16px 行高（1.19），多行描述挤成一坨——而这在源码和 typecheck 里都看不出来。
  it("每个字号档位都成对声明 line-height（防字号抬了行高没跟上）", () => {
    const indexCss = readRaw("/index.css")
    const paired = new Set(
      [...indexCss.matchAll(/^\s*--text-([\w-]+)--line-height:/gm)].map((m) => m[1]),
    )
    const missing = textRungs(indexCss).filter((step) => !paired.has(step))
    expect(missing, "这些字号档位缺成对行高").toEqual([])
  })

  it("闸门自身有效：成对修饰键不算档位", () => {
    const probe = "  --text-body: 15px;\n  --text-body--line-height: 22px;\n"
    expect(textRungs(probe)).toEqual(["body"])
  })
})
