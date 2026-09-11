// errorCardCopy.test.ts —— ErrorCard 文案归属门禁（2026-09-11，task-27）。
// 背景：`components/boot/ErrorCard.tsx` 有 4 处**可达**的裸中文（en 用户可见）：
//   · `:66`  `` `${t.error.actionFailed}：${msg}（可返回重选）` `` —— 与 task-25 已修的
//     BootSelector 同串，是「同串第三处」；复用 `t.error.reselectHint`（不新建重复键）。
//   · `:178` `原始诊断日志 · 尾部 {N} 行` —— 无既有键可复用 → 新增 `error.rawLogSummary(n)`。
//   · `:188` `title="复制日志"` 与 `:198` `<span>复制日志</span>` —— 新增 `error.copyLog`；
//     注意它与 `boot.copyDetail`（BootStep 的「复制详情」）语义不同，故未复用。
//   · `:193` `<span>已复制</span>` —— **复用既有** `boot.copied`（同一 boot 流程族通用串，
//     避免新建重复键）。
// 另 `:75` 的 logger 消息为开发者可见，白名单化（同 task-25 口径）。
// 纯逻辑测试：只读字典常量 + 源码文本，不渲染 DOM、不引 RTL/jsdom（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"
import { t as zhCN } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"
import errorCardSrc from "@/components/boot/ErrorCard.tsx?raw"

const CJK = /[\u3000-\u303f\u4e00-\u9fff\uff00-\uffef]/

/** 结构断言必须剥离注释（注释里引用了被移除的旧写法，否则假阳性）。 */
function stripComments(src: string): string {
  return src
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^[ \t]*\/\/.*$/gm, "")
}

/** 开发者可见的日志消息不入字典（同 task-25 口径：按「行内有 logger 调用」判定）。 */
function isLoggerLine(line: string): boolean {
  return /\blogger\.(debug|info|warn|error)\(/.test(line)
}

const code = stripComments(errorCardSrc)
const uiCode = code
  .split("\n")
  .filter((line) => !isLoggerLine(line))
  .join("\n")

const zhError = zhCN.error as unknown as Record<string, unknown>
const enError = enUS.error as unknown as Record<string, unknown>

describe("ErrorCard 文案归属（task-27）", () => {
  it("UI 面不得出现裸中文（logger 消息为显式例外）", () => {
    const offenders = uiCode
      .split("\n")
      .map((line, i) => ({ line: line.trim(), no: i + 1 }))
      .filter(({ line }) => CJK.test(line))
      .map(({ line, no }) => `L${no}: ${line}`)
    expect(offenders, "组件内仍有裸中文 UI 文案（应入字典）").toEqual([])
  })

  it("例外白名单非空且有命中（防腐化：日志改英文后应撤销白名单）", () => {
    expect(code.split("\n").filter(isLoggerLine).length).toBeGreaterThan(0)
    // 反向断言：剥离注释后仍保留真实结构（防过度剥离假绿）
    expect(code).toContain("t.error.reselectHint")
    expect(code).toContain("t.error.rawLogSummary(")
    expect(code).toContain("t.error.copyLog")
  })

  it("新增键两侧对称、类型一致且非空", () => {
    for (const key of ["copyLog", "rawLogSummary"] as const) {
      expect(key in zhError, `zh-CN 缺 error.${key}`).toBe(true)
      expect(key in enError, `en-US 缺 error.${key}`).toBe(true)
      expect(typeof enError[key], `error.${key} 两侧类型不一致`).toBe(typeof zhError[key])
      expect(String(zhError[key]).length, `zh-CN error.${key} 为空`).toBeGreaterThan(0)
    }
    expect(zhError.rawLogSummary).toBeTypeOf("function")
    expect((zhError.rawLogSummary as (n: number) => string)(12)).toBe("原始诊断日志 · 尾部 12 行")
    expect((enError.rawLogSummary as (n: number) => string)(12)).toBe("Raw diagnostic log · last 12 lines")
  })

  it("新增键 en 侧不含中文（组件内漏译的字典侧防线）", () => {
    for (const key of ["copyLog", "rawLogSummary"] as const) {
      const rendered = typeof enError[key] === "function"
        ? String((enError[key] as (n: number) => string)(7))
        : String(enError[key])
      expect(CJK.test(rendered), `en-US error.${key} 含中文`).toBe(false)
    }
  })

  it("复用既有键而非新建重复键（任务明确要求）", () => {
    // 「已复制」复用 boot.copied —— 不得在 error 下另起同义键
    expect(code).toContain("t.boot.copied")
    expect("copied" in zhError, "error 下不应新增重复的 copied 键").toBe(false)
    // 「（可返回重选）」复用 task-25 的 error.reselectHint —— 不得再写一份
    expect(code).toContain("t.error.reselectHint")
    expect(code).not.toContain("（可返回重选）")
  })

  it("copyLog 与 boot.copyDetail 语义分离（不复用、不混用）", () => {
    // 「复制日志」复制的是原始诊断日志；「复制详情」复制的是步骤遥测——两者不同键
    expect(String(zhError.copyLog)).toBe("复制日志")
    expect(String((zhCN.boot as unknown as Record<string, unknown>).copyDetail)).toBe("复制详情")
    expect(String(zhError.copyLog)).not.toBe(String((zhCN.boot as unknown as Record<string, unknown>).copyDetail))
  })

  it("旧内联文案不得回流（本文件特征串）", () => {
    for (const literal of ["原始诊断日志 · 尾部", "复制日志", "可返回重选"]) {
      // 「复制日志」作为字典值存在，但不得以字面量形式留在组件里
      expect(uiCode, `回流了旧内联文案 ${literal}`).not.toContain(`"${literal}`)
      expect(uiCode, `回流了旧内联文案 ${literal}`).not.toContain(`>${literal}<`)
    }
  })
})
