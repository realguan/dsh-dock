// Profile 管理器纯逻辑测试（4.3 前端刀）。Vitest 只测纯函数（AGENTS §4.4）：
// 名字校验镜像 / 模板表形状 / 创建结果归纳。
import { describe, expect, it } from "vitest"
import {
  summarizeCreateOutcome,
  TEMPLATE_BUNDLES,
  validateProfileName,
} from "@/lib/profiles"

describe("validateProfileName（逐字镜像 dsh resolveProfileDir 拒绝集）", () => {
  it("拒绝 dsh 拒绝的六种名字", () => {
    for (const bad of ["", "a/b", "a\\b", ".", "..", "node_modules"]) {
      expect(validateProfileName(bad), `{bad} 应被拒绝`).not.toBeNull()
    }
  })

  it("放行 dsh 允许的名字（勿加码）", () => {
    for (const good of ["web", "headless", "my-profile", "中文名", ".hidden", "a b", "..foo"]) {
      expect(validateProfileName(good), `${good} 应放行`).toBeNull()
    }
  })
})

describe("TEMPLATE_BUNDLES（镜像后端 PROFILE_TEMPLATES）", () => {
  it("只有 web/headless 两个模板名，bundle 列表与 dsh 一致", () => {
    expect(Object.keys(TEMPLATE_BUNDLES).sort()).toEqual(["headless", "web"])
    expect(TEMPLATE_BUNDLES.web).toEqual([
      "@deepseek-ai/dsh-base",
      "@deepseek-ai/dsh-web-app",
    ])
    expect(TEMPLATE_BUNDLES.headless).toEqual([
      "@deepseek-ai/dsh-base",
      "@deepseek-ai/dsh-headless",
    ])
  })
})

describe("summarizeCreateOutcome（已创建未装插件 = pending 而非 failed）", () => {
  it("installed + materialized = ready", () => {
    expect(summarizeCreateOutcome({ materialized: true, installed: true })).toBe("ready")
  })
  it("materialized 但未 installed = pending（合法中间态）", () => {
    expect(summarizeCreateOutcome({ materialized: true, installed: false })).toBe("pending")
  })
  it("未物化 = failed", () => {
    expect(summarizeCreateOutcome({ materialized: false, installed: false })).toBe("failed")
  })
})
