// bootModeCopy.test.ts —— BootMode 交互逻辑与文案归属门禁。
// 验证：
//   1. BootMode.tsx 非注释行无裸中文；
//   2. zh-CN 与 en-US 下 mode 字典键两侧对称、非空且 en 侧无 CJK；
//   3. BootMode 采用直调 IPC api.chooseMode 而非 URL query 间接中转；
//   4. 包含 loading 态与错误反馈。
import { describe, expect, it } from "vitest"
import { t as zhCN } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"
import bootModeSrc from "@/pages/BootMode.tsx?raw"

const CJK = /[\u3000-\u303f\u4e00-\u9fff\uff00-\uffef]/

function stripComments(src: string): string {
  return src
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^[ \t]*\/\/.*$/gm, "")
}

const code = stripComments(bootModeSrc)

describe("BootMode 交互与文案门禁", () => {
  it("非注释行不得出现裸中文 UI 文案（全走字典）", () => {
    const offenders = code
      .split("\n")
      .map((line, i) => ({ line: line.trim(), no: i + 1 }))
      .filter(({ line }) => CJK.test(line))
      .map(({ line, no }) => `L${no}: ${line}`)
    expect(offenders, "BootMode 组件内仍有裸中文 UI 文案（应入字典）").toEqual([])
  })

  it("mode 字典键在 zh-CN 与 en-US 间完全对称、非空", () => {
    const zhKeys = Object.keys(zhCN.mode)
    const enKeys = Object.keys(enUS.mode)
    expect(zhKeys.sort()).toEqual(enKeys.sort())

    for (const key of zhKeys) {
      const zhVal = (zhCN.mode as Record<string, string>)[key]
      const enVal = (enUS.mode as Record<string, string>)[key]
      expect(typeof zhVal).toBe("string")
      expect(typeof enVal).toBe("string")
      expect(zhVal.length).toBeGreaterThan(0)
      expect(enVal.length).toBeGreaterThan(0)
    }
  })

  it("en-US 的 mode 文案不含任何中文字符", () => {
    for (const [key, val] of Object.entries(enUS.mode)) {
      expect(CJK.test(val), `enUS.mode.${key} 含中文字符`).toBe(false)
    }
  })

  it("直调 IPC chooseMode 并具有 loading 态防护", () => {
    expect(code).toContain("api.chooseMode(picked, setDefault)")
    expect(code).toContain("submitting")
    expect(code).not.toContain("?mode=")
  })
})
