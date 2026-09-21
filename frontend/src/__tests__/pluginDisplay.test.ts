// pluginDisplay.test.ts —— 插件行「名字三层」纯函数门禁。
// 纯逻辑测试，不引 RTL/jsdom（AGENTS §4.3 末段）。
//
// 2026-09-21 三次修订（维护者真机反馈）：**插件名取原本的名字，中文为辅**。
// 判据随之改写——旧版是"包名不许当主标题，查表未命中就 Title Case 兜底"，
// 现在是"主标题 = 包名本身 / 中文名 = 查表命中才有（`null` = 没有）"。
import { describe, expect, it } from "vitest"

import {
  commonPackagePrefix,
  packageTail,
  pluginChineseName,
  pluginMemberLabel,
  pluginShortId,
} from "@/lib/pluginDisplay"

describe("pluginChineseName：查表命中给中文/英文辅助名", () => {
  it("官方高频包按语言分表", () => {
    expect(pluginChineseName("@deepseek-ai/dsh-base", "zh-CN")).toBe("Cordis 底座")
    expect(pluginChineseName("@deepseek-ai/dsh-base", "en-US")).toBe("Cordis Base")
    expect(pluginChineseName("@deepseek-ai/dsh-browser-use", "zh-CN")).toBe("浏览器操作")
    expect(pluginChineseName("@deepseek-ai/dsh-browser-use", "en-US")).toBe("Browser Use")
    expect(pluginChineseName("@openviking/dsh-memory-plugin", "zh-CN")).toBe("记忆与上下文")
    expect(pluginChineseName("@openviking/dsh-memory-plugin", "en-US")).toBe("Memory & Context")
  })

  it("en-US 输出不得夹带中文（漏译门禁同族）", () => {
    const CJK = /[\u3000-\u303f\u4e00-\u9fff\uff00-\uffef]/
    for (const pkg of [
      "@deepseek-ai/dsh-base",
      "@deepseek-ai/dsh-web-app",
      "@deepseek-ai/dsh-browser-use",
      "@deepseek-ai/dsh-computer-use",
      "@deepseek-ai/dsh-experimental-auto-review",
      "@openviking/dsh-memory-plugin",
    ]) {
      const name = pluginChineseName(pkg, "en-US")
      expect(name, `${pkg} 应有英文名`).not.toBeNull()
      expect(CJK.test(name as string), `${pkg} 的 en 名夹带中文`).toBe(false)
    }
  })
})

describe("pluginChineseName：未命中必须返回 null（不许机器音译冒充中文名）", () => {
  it("社区包是主路径：绝大多数包没有策展中文名", () => {
    // 旧版这里会 Title Case 兜底（`dsh-pet` → `Pet`）——那是机器音译：既不是插件
    // 自己的名字（主标题已经给了），也不是中文。裁定"取原本的名字，中文为辅"后，
    // 它没有位置，故判据翻转为**必须为 null**（UI 据此整块不渲染辅名）。
    expect(pluginChineseName("@linxin666/dsh-pet", "zh-CN")).toBeNull()
    expect(pluginChineseName("dsh-memory-plugin", "en-US")).toBeNull()
    expect(
      pluginChineseName("@deepseek-ai/dsh-experimental-browser-use-playwright-mcp", "en-US"),
    ).toBeNull()
    // 空入参也不再"永不为空"式兜底（占位符由调用方决定要不要给）
    expect(pluginChineseName("", "zh-CN")).toBeNull()
    expect(pluginChineseName("---", "en-US")).toBeNull()
  })
})

describe("pluginShortId：主标题（插件原本的名字，去 scope）", () => {
  it("去掉 scope，保留可读尾部", () => {
    expect(pluginShortId("@openviking/dsh-memory-plugin")).toBe("dsh-memory-plugin")
    expect(pluginShortId("plain-pkg")).toBe("plain-pkg")
    expect(pluginShortId("")).toBe("")
  })
})

describe("pluginMemberLabel：去噪短标签（明细行与实验能力面板共用）", () => {
  it("去 scope + 去 dsh/experimental 噪音段，**区分尾部完整保留**", () => {
    // 2026-09-21：实验能力的包名带两层噪音前缀（dsh- + experimental-），
    // 不去掉的话三个后端在界面上看不出区别（都在前 40 字符里相同）。
    expect(pluginMemberLabel("@deepseek-ai/dsh-browser-use")).toBe("browser-use")
    expect(pluginMemberLabel("@deepseek-ai/dsh-web-app")).toBe("web-app")
    expect(
      pluginMemberLabel("@deepseek-ai/dsh-experimental-browser-use-playwright-mcp"),
    ).toBe("browser-use-playwright-mcp")
    expect(
      pluginMemberLabel("@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp"),
    ).toBe("browser-use-chrome-devtools-mcp")
  })

  it("非 dsh 包原样保留标识（不许把用户包改得对不上 npm）", () => {
    expect(pluginMemberLabel("@openviking/dsh-memory-plugin")).toBe("memory-plugin")
    expect(pluginMemberLabel("@playwright/mcp")).toBe("mcp")
    expect(pluginMemberLabel("lodash")).toBe("lodash")
    // 全是噪音段时退回原标识，不返回空
    expect(pluginMemberLabel("dsh-experimental")).toBe("dsh-experimental")
  })
})

describe("commonPackagePrefix / packageTail：变体对照的公共前缀", () => {
  const PKGS = [
    "@deepseek-ai/dsh-experimental-browser-use-playwright-mcp",
    "@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp",
    "@deepseek-ai/dsh-experimental-browser-use-stagehand-native",
  ]

  it("按 `-` 分段对齐取公共前缀（不切在词中间）", () => {
    expect(commonPackagePrefix(PKGS)).toBe("@deepseek-ai/dsh-experimental-browser-use-")
  })

  it("尾部即区分段（60 字符 → 15 字符，仍可对照 npm）", () => {
    const prefix = commonPackagePrefix(PKGS)
    expect(PKGS.map((p) => packageTail(p, prefix))).toEqual([
      "playwright-mcp",
      "chrome-devtools-mcp",
      "stagehand-native",
    ])
  })

  it("边界：单成员/无公共前缀时不硬凑", () => {
    expect(commonPackagePrefix(["@a/only-one"])).toBe("")
    expect(commonPackagePrefix([])).toBe("")
    expect(commonPackagePrefix(["@a/x-y", "@b/x-y"])).toBe("")
    // 前缀不匹配时原样返回（不许把包名改得对不上 npm）
    expect(packageTail("@x/other", "@a/")).toBe("@x/other")
  })
})
