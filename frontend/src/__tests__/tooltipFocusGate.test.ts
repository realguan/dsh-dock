// tooltipFocusGate.test.ts —— 「程序化焦点不得弹出悬浮」的源码闸门（2026-09-19）。
//
// 复现的缺陷：打开「配置 MCP 服务」弹窗，解释图标（`Tip`）的悬浮每次都自动弹出来。
// 两条成因叠在一起：① Radix Dialog 的挂载焦点 = 容器里**第一个可聚焦元素**，而表单
// 弹窗的第一个常常是那个图标按钮（关闭键在 children 之后）；② Radix
// `@radix-ui/react-tooltip@1.2.16` 的 `TooltipTrigger.onFocus` **无条件** `onOpen()`
// （没有 `:focus-visible` 闸门）——于是"被聚焦"就等于"弹说明"。
//
// 本闸门钉住收口后的两条不变量：弹窗自己决定挂载焦点落在哪个字段；`Tip` 只对键盘
// 焦点开悬浮。回归形态 = 有人把 autofocus 交回默认，或把图标改成"焦点即开"。
import { describe, expect, it } from "vitest"

const RAW_SOURCES = import.meta.glob("../**/*.{ts,tsx}", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>

const DIALOG = "/components/ui/dialog.tsx"
const INFO_TIP = "/components/ui/info-tip.tsx"

/** 挂载焦点必须覆盖的三类原生字段。 */
const FIELD_KINDS = ["input", "textarea", "select"]

function sourceOf(suffix: string): string | undefined {
  return Object.entries(RAW_SOURCES).find(([path]) => path.endsWith(suffix))?.[1]
}

describe("悬浮焦点闸门", () => {
  it("DialogContent 接管挂载焦点，且只交给真字段", () => {
    const src = sourceOf(DIALOG)
    expect(src, `找不到 ${DIALOG}（闸门清单过期？）`).toBeDefined()
    expect(src ?? "", "弹窗没有自定义 onOpenAutoFocus：挂载焦点会落到第一个可聚焦元素").toMatch(
      /onOpenAutoFocus/,
    )
    for (const kind of FIELD_KINDS) {
      expect(
        src ?? "",
        `挂载焦点的选择器缺了 ${kind}`,
      ).toMatch(new RegExp(`\\b${kind}:not\\(`))
    }
    // 不接管时须让位给 Radix 默认（确认框没有输入框）。
    expect(src ?? "", "没有字段时也强行 preventDefault：确认框会失去焦点").toMatch(
      /if \(!field\) return/,
    )
  })

  it("Tip 只对 :focus-visible 的焦点开悬浮", () => {
    const src = sourceOf(INFO_TIP)
    expect(src, `找不到 ${INFO_TIP}（闸门清单过期？）`).toBeDefined()
    expect(
      src ?? "",
      "Tip 的 focus 处理没有 :focus-visible 闸门：程序化焦点会再次弹出说明",
    ).toMatch(/matches\(\s*":focus-visible"\s*\)/)
    // 图标必须仍可聚焦（键盘可达性），闸门只掐"焦点即开"，不摘掉 tabindex。
    expect(src ?? "", "Tip 触发器被移出 Tab 顺序：键盘用户读不到说明").not.toMatch(
      /tabIndex=\{-1\}/,
    )
  })

  it("闸门自身有效：能识别两种回归写法", () => {
    expect(/onOpenAutoFocus/.test("<DialogPrimitive.Content {...props} />")).toBe(
      false,
    )
    expect(/onOpenAutoFocus/.test("onOpenAutoFocus={handler}")).toBe(true)
    expect(/if \(!field\) return/.test("field.focus()")).toBe(false)
    expect(
      /matches\(\s*":focus-visible"\s*\)/.test(
        'onFocus={() => setOpen(true)}',
      ),
    ).toBe(false)
    expect(
      /matches\(\s*":focus-visible"\s*\)/.test(
        'if (!event.currentTarget.matches(":focus-visible")) return',
      ),
    ).toBe(true)
  })
})
