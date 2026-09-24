// dshUpgradeRestart.test.ts —— 「升级落定后要不要自动重启当前 profile」纯判据。
//
// 2026-09-24 维护者裁定：关于页升级 DSH 后自动重启当前 profile（新引擎必须真正
// 接管）。判据是行为改动，必须有测试：放错一次 = 要么无缘无故掐断用户进行中的
// 任务（Skip 也重启），要么装完新引擎却不生效（Install 不重启）。
import { describe, expect, it } from "vitest"
import { shouldRestartAfterUpgrade } from "@/lib/dshUpgradeRestart"
import type { DshUpgradeEvent } from "@/types/events"

/** 载荷构造器：默认 running 相位（其余字段按用例覆写）。 */
const ev = (p: Partial<DshUpgradeEvent>): DshUpgradeEvent => ({
  phase: "running",
  detail: "",
  ...p,
})

describe("shouldRestartAfterUpgrade", () => {
  it("done + 确实装了新版本（Install）→ 重启", () => {
    expect(
      shouldRestartAfterUpgrade(ev({ phase: "done", detail: "0.1.8", installed: true })),
    ).toBe(true)
  })

  it("done + 已是最新（Skip）→ 不重启：没装新东西，重启纯属掐断任务", () => {
    expect(
      shouldRestartAfterUpgrade(ev({ phase: "done", detail: "0.1.7", installed: false })),
    ).toBe(false)
  })

  it("done 但载荷缺 installed（旧后端/新字段前向兼容）→ 不猜，不重启", () => {
    expect(shouldRestartAfterUpgrade(ev({ phase: "done", detail: "0.1.8" }))).toBe(false)
  })

  it("running / failed / 未知相位 → 不重启", () => {
    expect(shouldRestartAfterUpgrade(ev({ phase: "running" }))).toBe(false)
    expect(shouldRestartAfterUpgrade(ev({ phase: "failed", detail: "boom" }))).toBe(false)
    expect(shouldRestartAfterUpgrade(ev({ phase: "whatever" }))).toBe(false)
  })
})
