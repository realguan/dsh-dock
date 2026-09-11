// Profile 管理器纯逻辑测试（4.3 前端刀）。Vitest 只测纯函数（AGENTS §4.4）：
// 名字校验镜像 / 模板表形状 / 创建结果归纳。
import { describe, expect, it } from "vitest"
import {
  PROFILE_LIST_FALLBACK_ERROR,
  profileListError,
  summarizeCreateOutcome,
  TEMPLATE_BUNDLES,
  validatePluginSpec,
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

describe("validatePluginSpec（镜像 plugins::validate_install_spec，ADR-0011 三形态）", () => {
  it("npm 形态：scope 包名与 tag/精确/^~ 区间放行，`><` 与元字符拒绝", () => {
    for (const ok of ["dsh-better-sidebar", "@scope/pkg", "pkg@next", "pkg@^1.0.0"]) {
      expect(validatePluginSpec(ok), `${ok} 应放行`).toBeNull()
    }
    for (const bad of ["pkg@>=2", "pkg`id`", "pkg$(id)", "a b", "pkg\tx"]) {
      expect(validatePluginSpec(bad), `${bad} 应被拒绝`).not.toBeNull()
    }
  })

  it("github 形态：registry 实测三形态放行，残缺/越界拒绝", () => {
    for (const ok of [
      "github:CAI-MH/dsh-quality-review",
      "github:zhu1090093659/dsh-web-ui#path:/packages/dsh-pet",
      "github:owner/repo#main",
      "github:owner/repo#dev&path:/packages/p",
    ]) {
      expect(validatePluginSpec(ok), `${ok} 应放行`).toBeNull()
    }
    for (const bad of [
      "github:",
      "github:o",
      "github:/r",
      "github:o/",
      "github:o/r/r2",
      "github:o/r#",
      "github:o/r#path:",
      "github:o/r#x;rm",
      "github:o/r#x y",
    ]) {
      expect(validatePluginSpec(bad), `${bad} 应被拒绝`).not.toBeNull()
    }
  })

  it("tarball 形态：https 直链放行，http/残缺/未知协议拒绝（fail-closed）", () => {
    expect(
      validatePluginSpec("https://github.com/o/r/releases/latest/download/p.tgz"),
    ).toBeNull()
    for (const bad of ["https://", "https:///x", "http://x/y.tgz", "npm:o/r", "git+https://github.com/o/r"]) {
      expect(validatePluginSpec(bad), `${bad} 应被拒绝`).not.toBeNull()
    }
  })

  it("注入面：前导 - 拒绝；长度上限 npm 214 / 总长 512", () => {
    expect(validatePluginSpec("-flag")).not.toBeNull()
    expect(validatePluginSpec("--frozen-lockfile")).not.toBeNull()
    expect(validatePluginSpec("a".repeat(215))).not.toBeNull()
    expect(validatePluginSpec("a".repeat(215))).toContain("214")
    expect(validatePluginSpec("a".repeat(513))).toContain("512")
    expect(validatePluginSpec(`github:o/${"r".repeat(300)}`)).toBeNull()
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

describe("profileListError（ADR-0016 §5-e 诚实错误面）", () => {
  it("原样透出后端详情——WSL 客体模式的「暂不支持 + 替代路径」不得被盖掉", () => {
    const detail =
      "「profile 列表」在 WSL 客体模式下暂不支持：本版本只下沉了插件装卸与插件清单（ADR-0016 P1），当前操作世界为 Ubuntu 客体。替代路径：在该发行版的终端里直接执行 dsh 命令，或切回本地模式后再从控制中心操作。"
    expect(profileListError(detail)).toBe(detail)
  })

  it("空/非字符串异常回退通用文案", () => {
    expect(profileListError("   ")).toBe(PROFILE_LIST_FALLBACK_ERROR)
    expect(profileListError(undefined)).toBe(PROFILE_LIST_FALLBACK_ERROR)
    expect(profileListError({ code: 1 })).toBe(PROFILE_LIST_FALLBACK_ERROR)
  })
})
