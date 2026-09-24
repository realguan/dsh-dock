// restartProfile.test.ts —— 「重启这个 Profile」统一出口（2026-09-24）。
//
// 复现的缺陷形态（预防性，非回归）：应用里凡有"变更需重启才生效"的地方都可能长出
// 自己的重启按钮，各写各的 invoke = 一种逻辑两种交互（确认口径/失败反馈漂移）。
// lib/restartProfile.ts 收口为一条链，本测试钉住两条路径：
//   ① 有处理器（ProfileManager 注册的既有确认链）→ 只走处理器，不直联 IPC；
//   ② 无处理器（本窗没挂 ProfileManager）→ 回退直联 switch_profile，且失败不得
//      把调用方（提示条 onClick）炸掉。
import { beforeEach, describe, expect, it, vi } from "vitest"

const ipc = vi.hoisted(() => ({
  switchProfile: vi.fn<(p: string) => Promise<unknown>>(),
}))
vi.mock("@/lib/tauri", () => ({ api: ipc }))

import { requestRestartProfile, setRestartProfileHandler } from "@/lib/restartProfile"

beforeEach(() => {
  // 模块级单例：每个用例前回到"无处理器"的干净态，否则用例间互相污染
  setRestartProfileHandler(null)
  ipc.switchProfile.mockReset()
  ipc.switchProfile.mockResolvedValue({})
})

describe("requestRestartProfile", () => {
  it("有处理器 → 走处理器（确认链），不直联 IPC", () => {
    const handler = vi.fn()
    setRestartProfileHandler(handler)

    requestRestartProfile("web")

    expect(handler).toHaveBeenCalledWith("web")
    expect(ipc.switchProfile).not.toHaveBeenCalled()
  })

  it("无处理器 → 回退直联 switch_profile", () => {
    requestRestartProfile("web")
    expect(ipc.switchProfile).toHaveBeenCalledWith("web")
  })

  it("注销后回到回退路径（热更新/关窗不得留下旧闭包）", () => {
    const handler = vi.fn()
    setRestartProfileHandler(handler)
    setRestartProfileHandler(null)

    requestRestartProfile("web")

    expect(handler).not.toHaveBeenCalled()
    expect(ipc.switchProfile).toHaveBeenCalledWith("web")
  })

  it("回退路径 IPC 失败不得抛出：提示条不该被一个 reject 炸掉", () => {
    ipc.switchProfile.mockRejectedValue(new Error("boom"))
    expect(() => requestRestartProfile("web")).not.toThrow()
  })
})
