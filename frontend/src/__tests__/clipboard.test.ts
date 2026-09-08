// clipboard.test.ts —— 剪贴板写入的失败语义单测（2026-09-08，批次 0b）。
// 复现先行：修复前 11 处调用里，写失败要么被 `.catch(() => {})` 吞掉后照样显示
// 「已复制」，要么完全没处理 promise。这里钉住：失败必须如实返回，绝不假装成功。
import { describe, expect, it, vi } from "vitest"
import { writeClipboard } from "@/lib/clipboard"

describe("writeClipboard", () => {
  it("写入成功 → ok，且原文透传（不裁剪、不转义）", async () => {
    const write = vi.fn<(v: string) => Promise<void>>(async () => {})
    const text = "mcp__github__*\n第二行  保留空格"

    const outcome = await writeClipboard(text, write)

    expect(outcome).toEqual({ ok: true })
    expect(write).toHaveBeenCalledWith(text)
  })

  it("写入失败 → ok:false 且带原始错误（不抛、不吞）", async () => {
    const boom = new Error("NotAllowedError: 剪贴板权限被拒")
    const outcome = await writeClipboard("x", async () => {
      throw boom
    })

    expect(outcome.ok).toBe(false)
    expect(outcome.error).toBe(boom)
  })

  it("非 Error 拒绝（字符串/undefined）同样不抛", async () => {
    const asString = await writeClipboard("x", async () => {
      throw "denied"
    })
    expect(asString).toEqual({ ok: false, error: "denied" })

    const asUndefined = await writeClipboard("x", async () => {
      throw undefined
    })
    expect(asUndefined.ok).toBe(false)
  })

  it("空串也照写（调用方负责过滤空内容，本层不做隐式拦截）", async () => {
    const write = vi.fn<(v: string) => Promise<void>>(async () => {})
    const outcome = await writeClipboard("", write)
    expect(outcome.ok).toBe(true)
    expect(write).toHaveBeenCalledWith("")
  })
})
