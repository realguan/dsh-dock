// bootSelectorCopy.test.ts —— BootSelector 文案归属门禁（2026-09-11，task-25）。
// 背景：`pages/BootSelector.tsx` 有 4 处非注释中文，其中 2 处是本次任务识别的缺陷：
//   · `:330`（改前）`"官方开箱即用"` —— **可达**，en 用户可见中文 = 真 i18n 缺陷；
//   · `:159`（改前）`` `${msg}（可返回重选）` `` —— **可达**（chooseProfile 失败路径），
//     同样是 en 用户可见中文；
//   · `:123`（改前）`title: name === "web" ? "默认工作台" : name` —— 当前不可达的
//     潜在陷阱（删掉 `items.web` 就会向 en 用户吐中文）；
//   · `:160` logger 消息 —— **开发者可见**，非 UI 文案（全仓 7 处同惯例），白名单化。
// 本门禁守住：① 该文件非注释行**不得再出现任何裸中文字面量**（logger 除外，
// 且例外显式列出并附理由）；② 新键两侧对称、en 侧无 CJK；③ 组件确实消费新键；
// ④ 只动文案：task-23 的判定解耦不得被本任务改动（回归断言）。
// 纯逻辑测试：只读字典常量 + 源码文本，不渲染 DOM、不引 RTL/jsdom（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"
import { t as zhCN } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"
import bootSelectorSrc from "@/pages/BootSelector.tsx?raw"

const CJK = /[\u3000-\u303f\u4e00-\u9fff\uff00-\uffef]/

/**
 * 断言「源码不含裸中文字面量」必须**剥离注释**：注释里会用中文说明（本文件大量如此），
 * 把注释当文案判红是假阳性。这是 task-20 / task-24 的同款教训，本任务直接做对。
 */
function stripComments(src: string): string {
  return src
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^[ \t]*\/\/.*$/gm, "")
}

/**
 * 例外白名单：**开发者可见的日志消息**（非 UI 文案、不参与本地化）。
 * 依据 AGENTS §4.3：`lib/logger.ts` 的格式是 `[模块] 描述 {ctx}`，描述用中文是全仓
 * 既有惯例（2026-09-11 实测 7 处同型），且日志面向维护者、不做 i18n。
 * 白名单按「行内出现 logger 调用」判定，而非钉死具体句子——这样将来改日志措辞
 * 不会误红，而**任何新增的 UI 中文都仍然会被抓住**。
 */
function isLoggerLine(line: string): boolean {
  return /\blogger\.(debug|info|warn|error)\(/.test(line)
}

const code = stripComments(bootSelectorSrc)
/** UI 面 = 剥离注释后的代码再剔除 logger 行（日志不是 UI 文案）。 */
const uiCode = code
  .split("\n")
  .filter((line) => !isLoggerLine(line))
  .join("\n")

describe("BootSelector 文案归属（task-25）", () => {
  it("非注释行不得出现裸中文（logger 消息为显式例外）", () => {
    const offenders = uiCode
      .split("\n")
      .map((line, i) => ({ line: line.trim(), no: i + 1 }))
      .filter(({ line }) => CJK.test(line))
      .map(({ line, no }) => `L${no}: ${line}`)
    expect(offenders, "组件内仍有裸中文 UI 文案（应入字典）").toEqual([])
  })

  it("例外白名单非空且确有命中（防止白名单腐化后静默放宽）", () => {
    // 若将来日志也改成英文，本断言会提示撤销白名单，而不是让门禁默默放水
    const loggerLines = code.split("\n").filter(isLoggerLine)
    expect(loggerLines.length, "logger 例外已无命中，应撤销该白名单").toBeGreaterThan(0)
    // 反向断言：剥离注释后仍保留真实结构（防 stripComments 过度剥离造成假绿）
    expect(code).toContain("resolveIsDefault(name, defaultProfile)")
    expect(code).toContain("t.selector.officialReadyToUse")
  })

  it("新键两侧对称存在、类型一致且非空", () => {
    for (const [path, zhVal, enVal] of [
      [
        "selector.officialReadyToUse",
        (zhCN.selector as unknown as Record<string, unknown>).officialReadyToUse,
        (enUS.selector as unknown as Record<string, unknown>).officialReadyToUse,
      ],
      [
        "error.reselectHint",
        (zhCN.error as unknown as Record<string, unknown>).reselectHint,
        (enUS.error as unknown as Record<string, unknown>).reselectHint,
      ],
    ] as const) {
      expect(typeof zhVal, `zh-CN 缺 ${path}`).toBe("string")
      expect(typeof enVal, `en-US 缺 ${path}`).toBe("string")
      expect(String(zhVal).length, `zh-CN ${path} 为空`).toBeGreaterThan(0)
      expect(String(enVal).length, `en-US ${path} 为空`).toBeGreaterThan(0)
    }
  })

  it("新键 en 侧不含中文（组件内缺陷的字典侧防线）", () => {
    const enSelector = enUS.selector as unknown as Record<string, unknown>
    const enError = enUS.error as unknown as Record<string, unknown>
    expect(CJK.test(String(enSelector.officialReadyToUse))).toBe(false)
    expect(CJK.test(String(enError.reselectHint))).toBe(false)
  })

  it("reselectHint 的括号体系正确：zh 全角无需空格，en ASCII 需前置空格", () => {
    const zhHint = String((zhCN.error as unknown as Record<string, unknown>).reselectHint)
    const enHint = String((enUS.error as unknown as Record<string, unknown>).reselectHint)
    // 拼接形式为 `${msg}${hint}`，故 en 侧缺前置空格会粘成一坨
    expect(zhHint.startsWith("（"), "zh 侧应以全角左括号开头").toBe(true)
    expect(zhHint.startsWith(" ("), "zh 侧不应带半角空格").toBe(false)
    expect(enHint.startsWith(" ("), "en 侧应以空格 + 半角左括号开头").toBe(true)
  })

  it("组件确实消费新键（不是只往字典里塞）", () => {
    expect(code).toContain("t.selector.officialReadyToUse")
    expect(code).toContain("t.error.reselectHint")
  })

  it("旧内联文案不得回流（仅本文件的特征串）", () => {
    for (const literal of ["官方开箱即用", "（可返回重选）", "默认工作台"]) {
      // 在 UI 面上（已剥离注释与 logger 行）不应再出现
      expect(uiCode, `回流了旧内联文案 ${literal}`).not.toContain(literal)
    }
  })

  it("回归：task-23 的判定解耦未被本任务改动", () => {
    // 只动文案，不动控制流——判定仍走纯函数，且不比较字典值
    expect(code).toContain("const isDefault = resolveIsDefault(name, defaultProfile)")
    expect(code).not.toContain("meta.tag ===")
    expect(code).toContain("FACTORY_DEFAULT_PROFILE")
    // 徽章渲染仍走字典键
    expect(code).toContain("{t.selector.defaultBadge}")
  })

  it("回归：判定的语义等价性（task-25 移除的不可达分支不影响可达路径）", () => {
    // 原 `title: name === "web" ? "默认工作台" : name`：对任何真正走到 `??` 回退的
    // name 必有 name !== "web"（items.web 恒存在）⇒ 旧值恒为 name ⇒ 新写法逐字等价。
    const items = zhCN.selector.items as unknown as Record<string, { title: string }>
    expect(Object.prototype.hasOwnProperty.call(items, "web"), "items.web 必须存在").toBe(true)
    expect(code).toMatch(/title:\s*name,/)
  })
})
