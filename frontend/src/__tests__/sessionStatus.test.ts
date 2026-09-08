// sessionStatus.test.ts —— 会话状态视觉映射（2026-09-08 UI/UX 评审 §18）。
//
// 缺陷复现：`unknown` 类在 UI 上被当成「无法判定（可能为活跃会话或引擎未就绪）」，
// 而 `--scan` 对 JSON 不可解析 / 空文件 / 版本高于本构建其实给了确切原因。本测试
// 钉住「有原因」与「无原因」两种 unknown 的描述必须不同（前者指向下方原因）。
import { describe, expect, it } from "vitest"
import { t } from "@/content/zh-CN"
import { statusMeta } from "@/lib/sessionStatus"

describe("statusMeta", () => {
  it("healthy / needs_repair 用各自描述", () => {
    expect(statusMeta("healthy", t, false)).toEqual({
      dot: "bg-emerald-500",
      badge: t.sessions.statusHealthy,
      desc: t.sessions.statusHealthyDesc,
    })
    expect(statusMeta("needs_repair", t, false).badge).toBe(t.sessions.statusNeedsRepair)
  })

  it("unknown：无原因时提示可能是活跃会话/引擎未就绪", () => {
    const meta = statusMeta("unknown", t, false)
    expect(meta.badge).toBe(t.sessions.statusUnknown)
    expect(meta.desc).toBe(t.sessions.statusUnknownDesc)
    expect(meta.desc).toContain("引擎未就绪")
  })

  it("unknown：有原因时不得再说「无法判定」——指向下方具体原因", () => {
    const meta = statusMeta("unknown", t, true)
    expect(meta.desc).toBe(t.sessions.statusUnknownDescWithReason)
    expect(meta.desc).not.toBe(t.sessions.statusUnknownDesc)
    expect(meta.desc).toContain("下方")
  })
})
