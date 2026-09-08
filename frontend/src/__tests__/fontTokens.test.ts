// fontTokens.test.ts —— 字号 token 化源码闸门（2026-09-08，UI/UX 批次 D / U7）。
//
// 复现的缺陷：全站 161 处 `text-[10px]` / `text-[11px]` / `text-[9px]` 一类任意值，
// 没有刻度、无法统一收缩，改一处忘一处。收进 `index.css` 的 `@theme` 后，
// 本闸门禁止再出现新的任意字号——需要新档位就先在 `@theme` 里加 token。
import { describe, expect, it } from "vitest"

const RAW_SOURCES = import.meta.glob("../**/*.{ts,tsx,css}", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>

/** 任意字号写法：`text-[10px]` / `text-[0.8rem]`（变体前缀如 `sm:` 也算）。 */
const ARBITRARY_FONT_SIZE = /text-\[[0-9.]+(?:px|rem)\]/g

describe("字号 token 闸门", () => {
  it("不得出现任意字号写法，一律用 @theme 刻度", () => {
    const offenders = Object.entries(RAW_SOURCES)
      .filter(([path]) => !path.endsWith("/fontTokens.test.ts"))
      .flatMap(([path, src]) => {
        const hits = [...src.matchAll(ARBITRARY_FONT_SIZE)].map((m) => m[0])
        return hits.length ? [`${path}: ${[...new Set(hits)].join(" ")}`] : []
      })

    expect(
      offenders,
      "这些位置用了任意字号——请在 index.css 的 @theme 加档位后用 text-<token>",
    ).toEqual([])
  })

  it("闸门自身有效：能识别任意字号与 token 写法", () => {
    expect("text-[10px]".match(ARBITRARY_FONT_SIZE)).toHaveLength(1)
    expect("sm:text-[0.8rem]".match(ARBITRARY_FONT_SIZE)).toHaveLength(1)
    expect("text-meta".match(ARBITRARY_FONT_SIZE)).toBeNull()
  })
})
