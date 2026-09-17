// dictCopyHygiene.test.ts —— 文案里**不许出现 markdown 标记**（2026-09-17 版面复盘）。
//
// 背景：面板没有 markdown 渲染器——`content/*.ts` 的字面量会**逐字**显示给用户。
// 版面复查时真抓到两处：能力面板的来源说明写着 ``@deepseek-ai/*``（反引号原样显示），
// Rust 侧「启用后会发生什么」写着 `**权限最高**`（星号原样显示）。前者已修，后者在
// `official_catalog.rs::user_facing_copy_has_no_markdown_markup` 里加了同口径门禁。
//
// 为什么用**求值后的字典叶子**而不是源码正则：字典里大量反引号是 JS 模板字面量的语法，
// 源码正则必然误报（已踩过）；叶子扫描只看用户真正会看到的字符串。
// 纯逻辑测试：只读字典常量，不引 RTL/jsdom（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"
import { t as zhCN } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"

/** 深度遍历字典取全部叶子字符串（函数以探针实参求值，覆盖动态文案）。 */
function collectLeaves(node: unknown, out: string[] = []): string[] {
  if (typeof node === "string") {
    out.push(node)
    return out
  }
  if (typeof node === "function") {
    const fn = node as (...args: unknown[]) => unknown
    out.push(String(fn("probe1", "probe2", "probe3", 1, 2)))
    return out
  }
  if (Array.isArray(node)) {
    for (const item of node) collectLeaves(item, out)
    return out
  }
  if (node && typeof node === "object") {
    for (const value of Object.values(node as Record<string, unknown>)) collectLeaves(value, out)
  }
  return out
}

describe("文案卫生：markdown 标记不得出现在用户可见文案里", () => {
  for (const [name, dict] of [
    ["zh-CN", zhCN],
    ["en-US", enUS],
  ] as const) {
    it(`${name}：没有反引号 / markdown 强调 / 标题标记`, () => {
      const bad = collectLeaves(dict).filter((v) => /`|\*\*|^#{1,6}\s/.test(v))
      expect(bad, `这些文案会被原样显示，请去掉 markdown 标记：\n${bad.join("\n---\n")}`).toEqual(
        [],
      )
    })
  }
})
