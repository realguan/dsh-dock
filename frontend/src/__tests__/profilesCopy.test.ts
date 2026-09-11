// profilesCopy.test.ts —— 官方桌面运行时说明文案门禁（2026-09-11 fix）。
// 背景：ProfileDetailPane 曾内联「内含 240+ 项本地预置底座服务」，数字既无壳侧
// 事实源（随 dsh 版本漂移），又违反 AGENTS §4.3「文案集中 content/zh-CN.ts」。
// 现文案拆为三段纯文本键，两个 `<code>` 标识符仍留在 JSX；本测试守住：
//   1) 三段均无硬编码数量；
//   2) 按组件拼接顺序可还原为完整句子（含两个标识符，括号闭合）；
//   3) 组件侧确实消费字典键、旧内联句不再回流（?raw 读源码，同 contrast.test.ts 先例）。
// 纯逻辑测试：只读字典常量 + 源码文本，不渲染 DOM、不引 RTL/jsdom（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"
import { t as zhCN } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"
import profileDetailPaneSrc from "@/components/profiles/ProfileDetailPane.tsx?raw"

const RUNTIME_PKG = "@deepseek-ai/dsh-desktop-runtime"
const PACKAGES_DIR = "desktop-packages/"

/** 与 ProfileDetailPane 的 JSX 顺序一致：Prefix + code + Mid + code + Suffix */
function renderNote(parts: readonly [string, string, string]): string {
  return `${parts[0]}${RUNTIME_PKG}${parts[1]}${PACKAGES_DIR}${parts[2]}`
}

const ZH_PARTS = [
  zhCN.profiles.desktopRuntimeDescPrefix,
  zhCN.profiles.desktopRuntimeDescMid,
  zhCN.profiles.desktopRuntimeDescSuffix,
] as const
const EN_PARTS = [
  enUS.profiles.desktopRuntimeDescPrefix,
  enUS.profiles.desktopRuntimeDescMid,
  enUS.profiles.desktopRuntimeDescSuffix,
] as const

describe("官方桌面运行时说明文案（ProfileDetailPane）", () => {
  it("三段键均为非空字符串且不含硬编码数量", () => {
    for (const [label, parts] of [
      ["zh-CN", ZH_PARTS],
      ["en-US", EN_PARTS],
    ] as const) {
      for (const [i, part] of parts.entries()) {
        expect(part.length, `${label} 第 ${i + 1} 段不应为空`).toBeGreaterThan(0)
        expect(part, `${label} 第 ${i + 1} 段内嵌了硬编码数字`).not.toMatch(/[0-9]/)
      }
      // 原失真文案的「240+」不得回归
      expect(renderNote(parts)).not.toContain("240")
    }
  })

  it("按组件顺序拼接可还原完整句子：标识符落在括号内且有间隔", () => {
    for (const [label, parts] of [
      ["zh-CN", ZH_PARTS],
      ["en-US", EN_PARTS],
    ] as const) {
      expect(parts[0], `${label} 前缀需以左括号收尾（code 落在括号内）`).toMatch(/(（|\()$/)
      expect(parts[1], `${label} 中段需以空格收尾（code 与后文有间隔）`).toMatch(/ $/)
      expect(parts[2], `${label} 后缀以右括号收尾`).toMatch(/）。$|\)\.$/)

      const sentence = renderNote(parts)
      expect(sentence).toContain(RUNTIME_PKG)
      expect(sentence).toContain(PACKAGES_DIR)
      // 两个标识符各出现一次，未被片段吞并或重复
      expect(sentence.split(RUNTIME_PKG)).toHaveLength(2)
      expect(sentence.split(PACKAGES_DIR)).toHaveLength(2)
    }
  })

  it("zh-CN 与 en-US 三段键对称且均为 string（非函数）", () => {
    const zhKeys = Object.keys(zhCN.profiles).filter((k) => k.startsWith("desktopRuntimeDesc"))
    const enKeys = Object.keys(enUS.profiles).filter((k) => k.startsWith("desktopRuntimeDesc"))
    expect(enKeys.sort()).toEqual(zhKeys.sort())
    expect(zhKeys).toHaveLength(3)
    for (const k of zhKeys) {
      expect(typeof (zhCN.profiles as Record<string, unknown>)[k]).toBe("string")
      expect(typeof (enUS.profiles as Record<string, unknown>)[k]).toBe("string")
    }
  })

  it("组件侧消费字典键且旧内联句不回流", () => {
    for (const key of [
      "t.profiles.desktopRuntimeDescPrefix",
      "t.profiles.desktopRuntimeDescMid",
      "t.profiles.desktopRuntimeDescSuffix",
    ]) {
      expect(profileDetailPaneSrc, `组件未消费 ${key}`).toContain(key)
    }
    // 旧内联句（含失真数字）不得回归——只查这一处的特征串，不做全文件中文扫荡
    expect(profileDetailPaneSrc).not.toContain("内含 240+")
    expect(profileDetailPaneSrc).not.toContain("项本地预置底座服务")
    expect(profileDetailPaneSrc).not.toContain("本地预置底座服务与核心组件（存放在")
  })
})
