import { describe, expect, it } from "vitest"
import { t as zhCN } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"

function checkKeysRecursively(zhObj: Record<string, unknown>, enObj: Record<string, unknown>, prefix = "") {
  const zhKeys = Object.keys(zhObj).sort()
  const enKeys = Object.keys(enObj).sort()

  expect(enKeys, `Keys mismatch at path "${prefix || 'root'}"`).toEqual(zhKeys)

  for (const k of zhKeys) {
    const zhVal = zhObj[k]
    const enVal = enObj[k]
    const currentPath = prefix ? `${prefix}.${k}` : k

    expect(typeof enVal, `Type mismatch at path "${currentPath}"`).toBe(typeof zhVal)

    if (typeof zhVal === "object" && zhVal !== null && !Array.isArray(zhVal)) {
      checkKeysRecursively(
        zhVal as Record<string, unknown>,
        enVal as Record<string, unknown>,
        currentPath,
      )
    }
  }
}

describe("i18n 多语言字典结构与一致性 (4.13)", () => {
  it("zh-CN 与 en-US 字典所有层级 key 深度完全对称", () => {
    checkKeysRecursively(
      zhCN as unknown as Record<string, unknown>,
      enUS as unknown as Record<string, unknown>,
    )
  })

  it("profiles 模块各二级 key 完整对齐", () => {
    const zhKeys = Object.keys(zhCN.profiles).sort()
    const enKeys = Object.keys(enUS.profiles).sort()
    expect(enKeys).toEqual(zhKeys)
  })

  it("sessions 模块各二级 key 完整对齐", () => {
    const zhKeys = Object.keys(zhCN.sessions).sort()
    const enKeys = Object.keys(enUS.sessions).sort()
    expect(enKeys).toEqual(zhKeys)
  })

  it("console 模块各二级 key 完整对齐", () => {
    const zhKeys = Object.keys(zhCN.console).sort()
    const enKeys = Object.keys(enUS.console).sort()
    expect(enKeys).toEqual(zhKeys)
  })

  it("动态模板函数能在两端正确求值", () => {
    expect(zhCN.console.profilesUsage(3, "12.5 MB")).toBe(
      "3 个 Profile · 占用 12.5 MB",
    )
    expect(enUS.console.profilesUsage(3, "12.5 MB")).toBe("3 Profiles · 12.5 MB")

    expect(zhCN.console.totalUsage("100 MB")).toBe("总存储占用：100 MB")
    expect(enUS.console.totalUsage("100 MB")).toBe("Total Storage: 100 MB")

    expect(zhCN.sessions.totalCount(5)).toBe("共发现 5 个会话")
    expect(enUS.sessions.totalCount(5)).toBe("Found 5 sessions")

    expect(zhCN.profiles.mcpSaveSuccess("github")).toContain("github")
    expect(enUS.profiles.mcpSaveSuccess("github")).toContain("github")
  })
})
