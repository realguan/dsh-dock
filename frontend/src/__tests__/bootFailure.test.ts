// bootFailure.test.ts —— boot:error 载荷规整（2026-09-08，ADR-0012）。
//
// 后端 `emit_boot_error` 现在下发 `failure`（tagged enum，`kind` 判别式）。前端
// `normalizeError` 是唯一入口：未知 kind 必须降级为 undefined（不能给 UI 一个
// 「看起来有分类」的假值），旧载荷没有 failure 字段也必须照旧可用。
import { describe, expect, it } from "vitest"
import { normalizeError } from "@/lib/eventPayloads"
import { BOOT_FAILURE_KINDS, type BootFailureKind } from "@/types/ipc"
import shapesRaw from "@/types/ipc-shapes.json?raw"

describe("normalizeError：failure 分类", () => {
  it("已知 kind 原样透出（含 unknown 携带的 detail）", () => {
    const e = normalizeError({
      failure: { kind: "credentials_mismatch" },
      title: "t",
      detail: "d",
      suggestion: "s",
      actions: ["upgrade", "retry"],
      log: "l",
    })
    expect(e?.failure).toEqual({ kind: "credentials_mismatch", detail: undefined })

    const unknown = normalizeError({ failure: { kind: "unknown", detail: "boom" } })
    expect(unknown?.failure).toEqual({ kind: "unknown", detail: "boom" })
  })

  it("未识别的 kind 降级为 undefined（不伪造分类）", () => {
    expect(normalizeError({ failure: { kind: "engine_not_ready" } })?.failure).toBeUndefined()
    expect(normalizeError({ failure: { kind: 42 } })?.failure).toBeUndefined()
    expect(normalizeError({ failure: "network_unavailable" })?.failure).toBeUndefined()
    expect(normalizeError({ failure: null })?.failure).toBeUndefined()
  })

  it("旧载荷（无 failure）仍可用：其余字段照常规整", () => {
    const e = normalizeError({
      title: "网络不可用",
      detail: "registry 不可达",
      suggestion: "检查网络",
      actions: ["retry", 7],
      log: "tail",
    })
    expect(e?.failure).toBeUndefined()
    expect(e?.title).toBe("网络不可用")
    expect(e?.actions).toEqual(["retry"])
    expect(e?.log).toBe("tail")
  })

  it("非对象载荷整体丢弃", () => {
    expect(normalizeError(null)).toBeNull()
    expect(normalizeError("oops")).toBeNull()
  })

  it("一键隔离计划：形状合法才透出（半个计划一律丢弃）", () => {
    // 2026-09-16：后端在"壳自己写的挂载行把插件树搞挂"时下发
    // `actions=["quarantine_plugin_row"]` + `quarantine={profile,rowId}`。
    // 前端只认完整形状——拿半个计划去调删除 IPC 会删错 profile。
    const ok = normalizeError({
      actions: ["quarantine_plugin_row"],
      quarantine: { profile: "web", rowId: "dsh-dock--deepseek-ai-x" },
    })
    expect(ok?.quarantine).toEqual({ profile: "web", rowId: "dsh-dock--deepseek-ai-x" })
    expect(ok?.actions).toEqual(["quarantine_plugin_row"])

    for (const bad of [
      { profile: "web" },
      { rowId: "dsh-dock-x" },
      { profile: "", rowId: "dsh-dock-x" },
      { profile: "web", rowId: "" },
      { profile: 42, rowId: "dsh-dock-x" },
      null,
      "nope",
    ]) {
      expect(normalizeError({ quarantine: bad })?.quarantine, JSON.stringify(bad)).toBeUndefined()
    }
  })

  it("空动作集原样透出（别把「后端说没有出路」补成「重试」）", () => {
    const e = normalizeError({ actions: [] })
    expect(e?.actions).toEqual([])
  })
})

/**
 * **边界不放行 = UI 静默少一块**（2026-09-16，独立复核抓到的真缺陷）。
 *
 * `normalizeError` 是按字段手写的白名单入口，**没有机器闸门**时它的默认状态是"漏字段"：
 * 新加的 `advancedActions` 就漏在这里 ⇒ `ErrorCard` 的 `payload.advancedActions ?? []`
 * 恒为空 ⇒「展开详情 → 其它出路」整块永不渲染，两条次级出路在 UI 上彻底不可达，
 * 而 Rust 侧契约测试、ipc-shapes 形状闸门、ErrorCard 源码结构门禁**全绿**。
 *
 * 现在的判据是**派生 + fixture 对齐**，不再靠人记：
 * ① kind 白名单由 `types/ipc.ts::BOOT_FAILURE_KINDS` 派生（类型层唯一事实源）；
 * ② 本组用共享 fixture `ipc-shapes.json` 的字段表逐键验"放行"，fixture 加字段即红。
 */
describe("normalizeError：边界不得静默丢字段（ADR-0012 派生口径）", () => {
  /** 覆盖 fixture 全部字段的样例载荷（缺样例也会在下一条断言里红）。 */
  const sample: Record<string, unknown> = {
    failure: { kind: "unknown", detail: "boom" },
    title: "t",
    detail: "d",
    suggestion: "s",
    actions: ["safe_mode"],
    advancedActions: ["quarantine_plugin_row", "safe_mode_reset"],
    log: "tail",
    quarantine: { profile: "web", rowId: "dsh-dock--x" },
  }

  it("共享 fixture 的每个字段都必须被放行（漏一个 = 前端少一块 UI）", () => {
    const keys = (JSON.parse(shapesRaw) as Record<string, string[]>).BootErrorPayload
    expect(keys.length, "fixture 里没有 BootErrorPayload 字段表").toBeGreaterThan(0)
    const normalized = normalizeError(sample) as unknown as Record<string, unknown>
    for (const key of keys) {
      expect(sample[key], `fixture 字段 ${key} 在本测试里没有样例`).toBeDefined()
      expect(
        normalized[key],
        `normalizeError 丢了 ${key}——前端会静默少一块 UI（曾发生在 advancedActions 上）`,
      ).toBeDefined()
    }
  })

  it("次级出路原样透出（含类型过滤与缺省）", () => {
    const e = normalizeError({
      advancedActions: ["safe_mode_reset", 7, null, "quarantine_plugin_row"],
    })
    expect(e?.advancedActions).toEqual(["safe_mode_reset", "quarantine_plugin_row"])
    expect(normalizeError({})?.advancedActions).toBeUndefined()
    expect(normalizeError({ advancedActions: "safe_mode_reset" })?.advancedActions).toBeUndefined()
  })

  it("类型层声明的每个 kind 都必须被放行（D1 的 kind 曾在白名单里缺席）", () => {
    for (const kind of BOOT_FAILURE_KINDS) {
      const e = normalizeError({ failure: { kind } })
      expect(e?.failure?.kind, `白名单漏了 ${kind} ⇒ UI 回退后端中文文案`).toBe(kind)
    }
    // 白名单之外仍然降级（不能伪造分类）
    expect(normalizeError({ failure: { kind: "engine_not_ready" } })?.failure).toBeUndefined()
  })

  it("kind 类型的取值集与 fixture/序列化值同源（防两份手抄表）", () => {
    // 这条是"派生"本身的护栏：BOOT_FAILURE_KINDS 是类型 BootFailureKind 的唯一来源，
    // 加 kind 只能加在它上面（`BootFailureKind` 由它派生，见 types/ipc.ts）。
    const asKinds: readonly BootFailureKind[] = BOOT_FAILURE_KINDS
    expect(new Set(asKinds).size).toBe(asKinds.length)
    expect(asKinds).toContain("plugin_row_failed")
    expect(asKinds).toContain("symlink_privilege_required")
  })
})
