// bootSelectorDefault.test.ts —— 「默认工作台」判定与文案解耦门禁（2026-09-11，task-23）。
// 背景：`pages/BootSelector.tsx` 原先写
//     const isDefault = name === defaultProfile || meta.tag === "DEFAULT"
// 其中 `meta.tag` 是**字典值**（`t.selector.items.web.tag`）⇒ 字典值被当控制流令牌。
// 两类后果：
//   ① 潜在真 bug（严重）：任何人把该值「翻译」成 `"默认"`，判定**静默失效**
//      —— 改文案改掉控制流；
//   ② en/zh 语序无关的渲染耦合（本文件实测 `tag` 字段无消费者，见 §消费点清单）。
// 修复：判定收敛到纯函数 `resolveIsDefault(name, defaultProfile)`，判据只来自
// 真实数据（defaultProfile）与模块常量（FACTORY_DEFAULT_PROFILE）。
// 纯逻辑测试：不渲染 DOM、不引 RTL/jsdom（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"
import {
  FACTORY_DEFAULT_PROFILE,
  resolveIsDefault,
} from "@/pages/BootSelector"
import { t as zhCN } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"
import { useI18nStore } from "@/stores/i18nStore"
import bootSelectorSrc from "@/pages/BootSelector.tsx?raw"

/** 取导出函数的函数体原文（用于断言「判据不看文案」）。 */
function exportedFnBody(src: string, name: string): string {
  const start = src.indexOf(`export function ${name}`)
  expect(start, `${name} 未被导出`).toBeGreaterThanOrEqual(0)
  const end = src.indexOf("\n}", start)
  expect(end, `${name} 函数体未闭合`).toBeGreaterThan(start)
  return src.slice(start, end)
}

/**
 * 结构断言必须**剥离注释**：注释里会引用被移除的旧写法
 * （本文件头注释就引用了 `meta.tag === "DEFAULT"`），把注释当代码判红是假阳性
 * （task-20 / task-24 的同款教训）。剥离后另有反向断言，防过度剥离造成假绿。
 */
function stripComments(src: string): string {
  return src
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^[ \t]*\/\/.*$/gm, "")
}

const bootSelectorCode = stripComments(bootSelectorSrc)

describe("resolveIsDefault：判定判据（真实数据 + 常量）", () => {
  it("未设置默认（null）⇒ 工厂默认工作台 web 标为默认", () => {
    // 产品口径（AGENTS §6，2026-09-11 措辞校正）：`None 由消费方兜底 web`
    expect(resolveIsDefault("web", null)).toBe(true)
  })

  it("失效默认值 ⇒ 不消费，无任何卡片标为默认（AGENTS §6「失效值不消费」）", () => {
    // 与 Rust `resolve.rs::consume_default_profile` 同口径：非 webUi 候选内的存储值
    // 一律不消费（走常规流程＝出选择器，不预选）⇒ 没有"将被启动的默认"，不标徽章。
    expect(resolveIsDefault("web", "ghost-deleted")).toBe(false)
    expect(resolveIsDefault("ghost-deleted", "ghost-deleted")).toBe(true)
  })

  it("已设置默认 ⇒ 只有被设置的那一个标为默认", () => {
    expect(resolveIsDefault("my-workbench", "my-workbench")).toBe(true)
    expect(resolveIsDefault("web", "my-workbench")).toBe(false)
  })

  it("工厂默认常量是稳定标识而非文案（值为 profile 名）", () => {
    expect(FACTORY_DEFAULT_PROFILE).toBe("web")
    // 与字典里的任何 tag 值都不应有关联（这正是解耦要点）
    expect(FACTORY_DEFAULT_PROFILE).not.toBe(String(zhCN.selector.items.web?.tag))
  })

  it("空串默认值按「未设置」处理需谨慎：仅 null/undefined 触发兜底", () => {
    // `??` 对空串不兜底——保持严格语义，避免把「显式空值」误当未设置
    expect(resolveIsDefault("", "")).toBe(true)
    expect(resolveIsDefault("web", "")).toBe(false)
  })
})

describe("防复发：判定不依赖可翻译文案（本任务核心）", () => {
  it("把字典 tag 值改成中文后，判定结果**不变**", () => {
    // 模拟「有人做翻译把 tag: "DEFAULT" 改成 "默认"」——修复前这会让 isDefault 静默失效
    const items = zhCN.selector.items as unknown as Record<string, { tag: string }>
    const original = items.web.tag
    try {
      items.web.tag = "默认"
      expect(resolveIsDefault("web", null), "改文案后 web 仍应为默认").toBe(true)
      expect(resolveIsDefault("custom-a", "custom-a")).toBe(true)
      expect(resolveIsDefault("web", "custom-a"), "改文案不应让 web 冒充默认").toBe(false)

      // 极端情形：把 tag 改成与工厂默认常量同值，判定也不应受影响
      items.web.tag = FACTORY_DEFAULT_PROFILE
      expect(resolveIsDefault("custom-a", "custom-a")).toBe(true)
      expect(resolveIsDefault("web", "custom-a")).toBe(false)
    } finally {
      items.web.tag = original
    }
  })

  it("把两语字典的 defaultBadge 文案改掉，判定结果同样不变", () => {
    const zhSel = zhCN.selector as unknown as { defaultBadge: string }
    const enSel = enUS.selector as unknown as { defaultBadge: string }
    const zhOriginal = zhSel.defaultBadge
    const enOriginal = enSel.defaultBadge
    try {
      zhSel.defaultBadge = "默认（已本地化）"
      enSel.defaultBadge = "IS-DEFAULT"
      expect(resolveIsDefault("web", null)).toBe(true)
      expect(resolveIsDefault("web", "other")).toBe(false)
    } finally {
      zhSel.defaultBadge = zhOriginal
      enSel.defaultBadge = enOriginal
    }
  })

  it("切换 activeLocale（task-24 的语言初始化）不改变判定结果", () => {
    // 与 task-24 的交互独立性：语言初始化只改字典，判定只看 name/defaultProfile
    const before = resolveIsDefault("web", null)
    useI18nStore.setState({ preference: "en-US", activeLocale: "en-US", t: enUS })
    expect(resolveIsDefault("web", null)).toBe(before)
    useI18nStore.setState({ preference: "zh-CN", activeLocale: "zh-CN", t: zhCN })
    expect(resolveIsDefault("web", null)).toBe(before)
  })

  it("判定函数不接收语言参数（签名即证据：无 locale 入参）", () => {
    expect(resolveIsDefault.length, "判定函数不应有第三个（语言）参数").toBe(2)
    // 函数体内不得出现任何字典引用
    const body = exportedFnBody(bootSelectorSrc, "resolveIsDefault")
    expect(body).not.toMatch(/\bt\./)
    expect(body).not.toContain(".tag")
  })
})

describe("结构门禁：旧耦合与死字段不得回流（?raw 源码断言，剥离注释）", () => {
  it("判据不再比较字典值", () => {
    expect(bootSelectorCode).not.toContain('meta.tag === "DEFAULT"')
    expect(bootSelectorCode).not.toContain("meta.tag ===")
    // 全文件不应再有「字典值参与 === 判定」的写法
    expect(bootSelectorCode).not.toMatch(/t\.[A-Za-z.]+ *===/)
    // 反向断言：剥离注释后仍保留真实结构（防过度剥离假绿）
    expect(bootSelectorCode).toContain("resolveIsDefault(name, defaultProfile)")
    expect(bootSelectorCode).toContain("meta.title || name")
  })

  it("组件确实调用纯函数（不是留个导出摆设）", () => {
    expect(bootSelectorCode).toContain("resolveIsDefault(name, defaultProfile)")
  })

  it("死字段 `tag` 已移除（它把翻译值/硬编码 `TEMPLATE`/字典值混在一处却无人消费）", () => {
    // 实测：修复前 `tag:` 在 displayProfiles 的**返回对象**里赋值，全文件无
    // `p.tag` / `{p.tag}` 消费点 ⇒ 计算了但从不渲染（删掉后 typecheck 仍通过，
    // 反证其无消费者）。
    expect(bootSelectorCode).not.toContain("TEMPLATE")
    expect(bootSelectorCode).not.toMatch(/isDefault \? t\.selector\.defaultBadge/)
    // 剥离注释后 `tag:` 只剩「字典回退 meta」这一处数据用途（非判定、非渲染）
    const tagAssignments = bootSelectorCode.match(/^\s*tag:/gm) ?? []
    expect(tagAssignments, "死字段 tag 疑似回流").toHaveLength(1)
    expect(bootSelectorCode).toContain("tag: t.selector.customTag")
  })

  it("默认徽章渲染仍走字典键（解耦未改渲染文案）", () => {
    expect(bootSelectorCode).toContain("{t.selector.defaultBadge}")
  })
})

describe("消费点清单核对（防止「还有第三处同类」被漏掉）", () => {
  it("字典 tag 值的全部消费点均可枚举，且无判定用途", () => {
    const tagLineRe = /meta\.tag|items\[name\]/g
    const occurrences = (bootSelectorCode.match(tagLineRe) ?? []).length
    // 修复后只剩「取字典项」与「构造回退 meta」两处数据用途
    expect(occurrences).toBeLessThanOrEqual(3)
    // 判定函数体内不出现
    expect(exportedFnBody(bootSelectorSrc, "resolveIsDefault")).not.toContain("meta")
  })

  it("同文件内无第三处「字典值当控制流」（大写字符串比较已清零）", () => {
    const comparisons = bootSelectorCode.match(/=== *"[A-Z_]{2,}"/g) ?? []
    expect(comparisons, `仍有字面量控制流：${comparisons.join(", ")}`).toEqual([])
  })
})
