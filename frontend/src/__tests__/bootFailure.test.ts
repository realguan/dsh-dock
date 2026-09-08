// bootFailure.test.ts —— boot:error 载荷规整（2026-09-08，ADR-0012）。
//
// 后端 `emit_boot_error` 现在下发 `failure`（tagged enum，`kind` 判别式）。前端
// `normalizeError` 是唯一入口：未知 kind 必须降级为 undefined（不能给 UI 一个
// 「看起来有分类」的假值），旧载荷没有 failure 字段也必须照旧可用。
import { describe, expect, it } from "vitest"
import { normalizeError } from "@/lib/eventPayloads"

describe("normalizeError：failure 分类", () => {
  it("已知 kind 原样透出（含 unknown 携带的 detail）", () => {
    const e = normalizeError({
      failure: { kind: "credentials_mismatch" },
      title: "t",
      detail: "d",
      suggestion: "s",
      actions: ["upgrade", "retry"],
      log: "l",
    })
    expect(e?.failure).toEqual({ kind: "credentials_mismatch", detail: undefined })

    const unknown = normalizeError({ failure: { kind: "unknown", detail: "boom" } })
    expect(unknown?.failure).toEqual({ kind: "unknown", detail: "boom" })
  })

  it("未识别的 kind 降级为 undefined（不伪造分类）", () => {
    expect(normalizeError({ failure: { kind: "engine_not_ready" } })?.failure).toBeUndefined()
    expect(normalizeError({ failure: { kind: 42 } })?.failure).toBeUndefined()
    expect(normalizeError({ failure: "network_unavailable" })?.failure).toBeUndefined()
    expect(normalizeError({ failure: null })?.failure).toBeUndefined()
  })

  it("旧载荷（无 failure）仍可用：其余字段照常规整", () => {
    const e = normalizeError({
      title: "网络不可用",
      detail: "registry 不可达",
      suggestion: "检查网络",
      actions: ["retry", 7],
      log: "tail",
    })
    expect(e?.failure).toBeUndefined()
    expect(e?.title).toBe("网络不可用")
    expect(e?.actions).toEqual(["retry"])
    expect(e?.log).toBe("tail")
  })

  it("非对象载荷整体丢弃", () => {
    expect(normalizeError(null)).toBeNull()
    expect(normalizeError("oops")).toBeNull()
  })
})
