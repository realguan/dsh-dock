// clipboardGate.test.ts —— 剪贴板唯一入口的源码闸门（2026-09-08，批次 0b）。
//
// 复现的缺陷：11 处调用各自写 `navigator.clipboard.writeText(...)`，处理方式分四种
// （不接 promise / `.catch(() => {})` 后照样置位 / 只挂 `.then` 成功分支 / 唯一正确写法），
// 于是「写失败仍显示已复制」在多个面板反复出现。收口到 `lib/clipboard.ts` 后，
// 本闸门钉住它不再被绕过——新增调用点直接写 `clipboard.writeText` 即红。
import { describe, expect, it } from "vitest"

const RAW_SOURCES = import.meta.glob("../**/*.{ts,tsx}", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>

/** 允许出现 `clipboard.writeText` 的文件（唯一入口 + 本闸门自身）。 */
const ALLOWED = ["/lib/clipboard.ts", "/__tests__/clipboardGate.test.ts"]

describe("剪贴板唯一入口闸门", () => {
  it("只有 lib/clipboard.ts 可以直呼 navigator.clipboard.writeText", () => {
    const offenders = Object.entries(RAW_SOURCES)
      .filter(([path]) => !ALLOWED.some((ok) => path.endsWith(ok)))
      .filter(([, src]) => /clipboard\s*\.\s*writeText/.test(src))
      .map(([path]) => path)

    expect(
      offenders,
      "这些文件绕过了 lib/clipboard.ts（写失败会被静默成「已复制」）",
    ).toEqual([])
  })

  it("闸门自身有效：能识别直呼写法", () => {
    expect(/clipboard\s*\.\s*writeText/.test("navigator.clipboard.writeText(x)")).toBe(true)
    expect(/clipboard\s*\.\s*writeText/.test("navigator.clipboard . writeText(x)")).toBe(true)
    expect(/clipboard\s*\.\s*writeText/.test("navigator.clipboard.readText(x)")).toBe(false)
  })
})
