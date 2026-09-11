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
    // 字典里**已不存在任何名为 tag 的键**（task-27 清理）——故常量不可能与它沾边。
    // 这比 task-23 的「比较两者不相等」更强：不是"碰巧不同"，是"根本不存在"。
    const items = zhCN.selector.items as unknown as Record<string, Record<string, unknown>>
    expect(Object.keys(items.web)).not.toContain("tag")
    expect(Object.keys(zhCN.selector)).not.toContain("customTag")
  })

  it("空串默认值按「未设置」处理需谨慎：仅 null/undefined 触发兜底", () => {
    // `??` 对空串不兜底——保持严格语义，避免把「显式空值」误当未设置
    expect(resolveIsDefault("", "")).toBe(true)
    expect(resolveIsDefault("web", "")).toBe(false)
  })
})

describe("防复发：判定不依赖可翻译文案（本任务核心）", () => {
  it("把字典里**仍在渲染**的文案改掉后，判定结果**不变**", () => {
    // 2026-09-11（task-27）：原用 `items.web.tag` 作变异靶子；该键已随死代码清理删除
    // （见下方结构门禁）。改为用**仍然活着且参与渲染**的 `items.web.title` 作靶子，
    // 保留「行为层演示」而不依赖已删键——否则这条断言会变成 vacuous（永远通过）。
    // （`defaultBadge` 的变异演示由下一条用例专门覆盖，两者靶子不重叠。）
    const items = zhCN.selector.items as unknown as Record<string, { title: string }>
    const originalTitle = items.web.title
    try {
      items.web.title = "官方工作台（中文名）"
      expect(resolveIsDefault("web", null), "改文案后 web 仍应为默认").toBe(true)
      expect(resolveIsDefault("custom-a", "custom-a")).toBe(true)
      expect(resolveIsDefault("web", "custom-a"), "改文案不应让 web 冒充默认").toBe(false)

      // 极端情形：把标题改成与工厂默认常量同值，判定也不应受影响
      items.web.title = FACTORY_DEFAULT_PROFILE
      expect(resolveIsDefault("custom-a", "custom-a")).toBe(true)
      expect(resolveIsDefault("web", "custom-a")).toBe(false)
    } finally {
      items.web.title = originalTitle
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

  it("死字段 `tag` 已彻底移除且**不得回流**（本断言防回流，不防「清理」）", () => {
    // 2026-09-11（task-27）修正：本用例原先断言 `toContain("tag: t.selector.customTag")`
    // ——那是在**钉死死代码**（要求已无消费者的字段必须存在），将来谁清理它就红，
    // 属「测试防腐烂失败」的隐蔽反模式。现按真实意图重写：
    //   ① 组件侧：任何 `tag:` 赋值都不得存在（含回退 meta 与返回对象）；
    //   ② 字典侧：`customTag` 与 `items[*].tag` 已删，不得回流。
    expect(bootSelectorCode).not.toContain("TEMPLATE")
    expect(bootSelectorCode).not.toMatch(/isDefault \? t\.selector\.defaultBadge/)
    expect(bootSelectorCode.match(/^\s*tag:/gm) ?? [], "组件内 tag: 赋值疑似回流").toEqual([])
    expect(bootSelectorCode, "回退了字典 customTag 引用").not.toContain("t.selector.customTag")

    // 字典侧同查（两语对称，防只删一侧）
    for (const [label, dict] of [
      ["zh-CN", zhCN],
      ["en-US", enUS],
    ] as const) {
      const sel = dict.selector as unknown as Record<string, unknown>
      expect(Object.keys(sel), `${label} 的 customTag 疑似回流`).not.toContain("customTag")
      const items = sel.items as Record<string, Record<string, unknown>>
      expect(Object.keys(items.web), `${label} 的 items.web.tag 疑似回流`).not.toContain("tag")
    }
  })

  it("默认徽章渲染仍走字典键（解耦未改渲染文案）", () => {
    expect(bootSelectorCode).toContain("{t.selector.defaultBadge}")
  })
})

describe("消费点清单核对（防止「还有第三处同类」被漏掉）", () => {
  it("组件内已无任何字典 tag 消费点（结构上不可能再耦合）", () => {
    // task-27 清理后：`tag` 既不参与判定也不参与渲染，且连字段都不存在。
    // 保留「取字典项」计数上界，防将来有人重新引入一条读 tag 的路径。
    const occurrences = (bootSelectorCode.match(/meta\.tag|items\[name\]/g) ?? []).length
    expect(occurrences, "疑似重新引入 meta.tag 消费").toBeLessThanOrEqual(2)
    // 判定函数体内不出现 meta（判据与字典彻底隔离）
    expect(exportedFnBody(bootSelectorSrc, "resolveIsDefault")).not.toContain("meta")
  })

  it("同文件内无第三处「字典值当控制流」（大写字符串比较已清零）", () => {
    const comparisons = bootSelectorCode.match(/=== *"[A-Z_]{2,}"/g) ?? []
    expect(comparisons, `仍有字面量控制流：${comparisons.join(", ")}`).toEqual([])
  })
})
