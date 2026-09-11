// enUsNoLeak.test.ts —— en-US 字典漏译门禁（2026-09-11，task-18）。
// 背景：`en-US.ts` 的 `console.localeSystem` 值含中文（`"System Default (跟随系统)"`），
// en-US 用户直接看到中文——与 task-16 的 `loadingBtn` 同类，是**真 i18n 缺陷**而非归属违规。
// 本门禁做**系统性**排查：遍历 en-US 字典全部叶子值（含函数求值、数组元素、
// 嵌套对象），命中 CJK 即失败——把「还有多少处」一次问清，杜绝逐点补漏。
// 例外**必须显式白名单化并写明理由**：不在表内的新漏译一律红（防静默放过）。
// 纯逻辑测试：只读字典常量，不渲染 DOM、不引 RTL/jsdom（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"
import { t as zhCN } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"

/** 汉字 + CJK 标点 + 全角字符（en 侧出现任一即为漏译或未本地化）。 */
const CJK = /[\u3000-\u303f\u4e00-\u9fff\uff00-\uffef]/

/**
 * 有意保留的中文（例外表）。**每一项都必须有理由**——没有理由的保留会被
 * review 驳回（AGENTS §4.3「文案集中」；本表即该规则的机器化例外清单）。
 */
const INTENTIONAL_CJK: Record<string, string> = {
  "console.localeZh":
    "语言自名（endonym）：语言选择器按通行惯例以「该语言自身」书写选项，" +
    "en 语境下显示「简体中文」是正确的，翻译成英文反而让母语用户难以辨认。",
}

/** 与 `localeZh` 同族的语言/系统选项键：两侧都须对各自语言纯净（不得夹带对方语言）。 */
const LOCALE_KEYS = ["localeSystem", "localeZh", "localeEn"] as const

type Leaf = { path: string; value: string }

/** 深度遍历字典，取全部叶子值；函数以探针实参求值（长度 3 覆盖现有全部动态文案形参）。 */
function collectLeaves(node: unknown, path = "", out: Leaf[] = []): Leaf[] {
  if (typeof node === "string") {
    out.push({ path, value: node })
    return out
  }
  if (typeof node === "function") {
    out.push({ path, value: String((node as (...a: unknown[]) => unknown)("probe1", "probe2", "probe3")) })
    return out
  }
  if (Array.isArray(node)) {
    node.forEach((item, i) => collectLeaves(item, `${path}[${i}]`, out))
    return out
  }
  if (node && typeof node === "object") {
    for (const [key, value] of Object.entries(node)) {
      collectLeaves(value, path ? `${path}.${key}` : key, out)
    }
  }
  return out
}

const enLeaves = collectLeaves(enUS)

describe("en-US 字典漏译门禁（task-18）", () => {
  it("扫描器确实遍历到全部叶子值（防扫描器自身失效导致假绿）", () => {
    // 下界取自 2026-09-11 实测（en-US 叶子值 600+）；若文案被大幅删减，
    // 应同步下调并说明——下界存在是为了防止 walker 静默返回空数组而「全绿」。
    expect(enLeaves.length).toBeGreaterThan(400)
    // 抽样确认三类形态都被覆盖：普通字符串 / 函数 / 数组元素
    const paths = new Set(enLeaves.map((l) => l.path))
    expect([...paths].some((p) => p.includes("[0]")), "数组元素未被遍历").toBe(true)
    expect([...paths].some((p) => p.includes(".")), "嵌套对象未被遍历").toBe(true)
    expect(paths.has("console.localeSystem"), "已知键未被遍历").toBe(true)
  })

  it("全部叶子值不含 CJK（例外表内除外）", () => {
    const offenders = enLeaves
      .filter((leaf) => CJK.test(leaf.value) && !(leaf.path in INTENTIONAL_CJK))
      .map((leaf) => `${leaf.path} = ${JSON.stringify(leaf.value)}`)
    expect(offenders, "en-US 侧出现中文 ⇒ en 用户可见中文（漏译）").toEqual([])
  })

  it("例外表内的键确实存在且确实含 CJK（防止例外表腐化成僵尸条目）", () => {
    for (const path of Object.keys(INTENTIONAL_CJK)) {
      const leaf = enLeaves.find((l) => l.path === path)
      expect(leaf, `例外表条目 ${path} 在字典中不存在（已腐化，应删除）`).toBeDefined()
      expect(CJK.test(leaf!.value), `例外表条目 ${path} 已不含 CJK（例外应撤销）`).toBe(true)
    }
  })

  it("例外表条目必须带非空理由", () => {
    for (const [path, reason] of Object.entries(INTENTIONAL_CJK)) {
      expect(reason.trim().length, `${path} 缺少保留理由`).toBeGreaterThan(8)
    }
  })

  it("语言/系统选项两侧对各自语言纯净（不夹带对方语言）", () => {
    const zhConsole = zhCN.console as unknown as Record<string, unknown>
    const enConsole = enUS.console as unknown as Record<string, unknown>
    for (const key of LOCALE_KEYS) {
      expect(typeof zhConsole[key], `zh-CN 缺 console.${key}`).toBe("string")
      expect(typeof enConsole[key], `en-US 缺 console.${key}`).toBe("string")
    }
    // zh 侧不得夹带英文括注（语言自名 `localeEn` 例外：英文自名本就该是英文）
    for (const key of ["localeSystem", "localeZh"] as const) {
      expect(
        String(zhConsole[key]),
        `zh-CN console.${key} 夹带了冗余英文`,
      ).not.toMatch(/\(?\s*[A-Za-z]{2,}/)
    }
    // en 侧不得夹带中文括注（`localeZh` 已在 INTENTIONAL_CJK 显式例外）
    expect(String(enConsole.localeSystem)).not.toMatch(CJK)
    expect(String(enConsole.localeSystem)).toBe("System Default")
  })
})
