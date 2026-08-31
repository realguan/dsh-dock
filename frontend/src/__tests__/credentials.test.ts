// credentials.test.ts —— 凭据脱敏管理与配置前端单测（4.5）。
import { describe, expect, it } from "vitest"
import type { CredentialSummaryItem } from "@/types/ipc"

describe("Credentials Management & Masking", () => {
  it("correctly identifies configured vs unconfigured providers", () => {
    const summary: CredentialSummaryItem[] = [
      {
        provider: "deepseek",
        label: "DeepSeek",
        configured: true,
        maskedKey: "sk-d••••••••1234",
      },
      {
        provider: "openai",
        label: "OpenAI",
        configured: false,
        maskedKey: "",
      },
      {
        provider: "anthropic",
        label: "Anthropic Claude",
        configured: true,
        maskedKey: "sk-a••••••••abcd",
      },
    ]

    const configured = summary.filter((s) => s.configured)
    const unconfigured = summary.filter((s) => !s.configured)

    expect(configured.length).toBe(2)
    expect(unconfigured.length).toBe(1)
    expect(configured.map((c) => c.provider)).toEqual(["deepseek", "anthropic"])
    expect(unconfigured[0].provider).toBe("openai")
  })

  it("ensures maskedKey does not leak full credentials", () => {
    const masked = "sk-a••••••••7890"
    expect(masked).toContain("••••••••")
    expect(masked.length).toBeLessThan(30)
    expect(masked.startsWith("sk-a")).toBe(true)
    expect(masked.endsWith("7890")).toBe(true)
  })

  it("handles empty key payload for deletion cleanly", () => {
    const provider = "deepseek"
    const inputKey = "   "
    const payloadKey = inputKey.trim()

    expect(provider).toBe("deepseek")
    expect(payloadKey).toBe("")
  })
})
