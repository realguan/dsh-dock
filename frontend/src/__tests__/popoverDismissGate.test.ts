// popoverDismissGate.test.ts —— 浮层「点外/ESC 收起」闸门（2026-09-09，
// 问题记录-2026-09-09 §1.2）。
//
// 复现的缺陷：下载管理面板是手写 `useState + absolute` 下拉——只能再点一次触发
// 按钮收起，点面板外部、按 ESC 都不关；同仓 Select / Tooltip / Dialog 都走 Radix
// 原语，行为不一致。手写外点检测还会连带漏掉焦点归还、嵌套浮层、碰撞翻转。
//
// 本闸门钉住两条：① 全仓不得手写「外点关闭」的指针监听；② 下载管理面板必须
// 引用 Popover（回归形态 = 有人又写了一个手写下拉）。
import { describe, expect, it } from "vitest"

const RAW_SOURCES = import.meta.glob("../**/*.{ts,tsx}", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>

/** 手写外点检测的写法（document/window 上的指针类监听）。 */
const HANDROLLED_OUTSIDE = /(?:document|window)\s*\.\s*addEventListener\s*\(\s*["'](?:mousedown|pointerdown|click|touchstart)["']/

const QUEUE_PANEL = "/components/market/QueuePanel.tsx"

function sourceOf(suffix: string): string | undefined {
  return Object.entries(RAW_SOURCES).find(([path]) => path.endsWith(suffix))?.[1]
}

describe("浮层收起闸门", () => {
  it("全仓不再手写外点关闭（统一走 Radix 原语的 dismiss 语义）", () => {
    const offenders = Object.entries(RAW_SOURCES)
      .filter(([, src]) => HANDROLLED_OUTSIDE.test(src))
      .map(([path]) => path)

    expect(
      offenders,
      "这些文件手写了 document/window 指针监听做外点关闭：漏掉焦点归还与嵌套浮层，改用 ui/popover.tsx 或既有 Radix 原语",
    ).toEqual([])
  })

  it("下载管理面板引用了 Popover（点外/ESC 可收起）", () => {
    const src = sourceOf(QUEUE_PANEL)
    expect(src, `找不到 ${QUEUE_PANEL}（闸门清单过期？）`).toBeDefined()
    expect(src ?? "", "下载管理面板没有用 Popover——手写下拉的收起习惯问题会回归").toMatch(
      /ui\/popover/,
    )
  })

  it("闸门自身有效：能识别手写外点监听", () => {
    expect(HANDROLLED_OUTSIDE.test('document.addEventListener("mousedown", onDown)')).toBe(true)
    expect(HANDROLLED_OUTSIDE.test('window . addEventListener("pointerdown", onDown)')).toBe(true)
    expect(HANDROLLED_OUTSIDE.test('document.addEventListener("keydown", onKey)')).toBe(false)
    expect(HANDROLLED_OUTSIDE.test('window.addEventListener("focus", onFocus)')).toBe(false)
  })
})
