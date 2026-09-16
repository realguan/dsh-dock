// queueStore.test.ts —— 安装队列**编排层**测试（2026-09-15，R2）。
//
// 为什么必须测编排层：R2 新增的 `enqueueAndWait` 是「官方实验室」目录安装的
// 唯一驱动方式（`lib/experimentalCapabilities.ts` 的 `install`/`remove` 都绑到它），
// 而目录的**顺序即语义**——「先移除同族 provider 再装新的」如果并发跑就会
// 激活失败。纯逻辑测试（queue.test.ts）只覆盖状态迁移，覆盖不到这条契约。
//
// 口径：mock 掉 IPC 边界（`@/lib/tauri`），跑真实的 store 与真实文案
// （不 mock i18nStore——新增的 remove 文案键要走真实路径才作数）。
import { beforeEach, describe, expect, it, vi } from "vitest"

interface Outcome {
  ok: boolean
  detail: string
}

const ipc = vi.hoisted(() => ({
  installPlugin: vi.fn<(p: string, s: string) => Promise<Outcome>>(),
  removePlugin: vi.fn<(p: string, pkg: string) => Promise<Outcome>>(),
  getShellSettings: vi.fn<() => Promise<Record<string, never>>>(),
}))
vi.mock("@/lib/tauri", () => ({ api: ipc }))

import { useQueueStore } from "@/stores/queueStore"

const ok = (detail = ""): Outcome => ({ ok: true, detail })
const softFail = (detail: string): Outcome => ({ ok: false, detail })

/** 手动放行的 Promise，用来把「串行」证伪成「并发」。 */
function deferred<T>() {
  let resolve!: (v: T) => void
  const promise = new Promise<T>((r) => {
    resolve = r
  })
  return { promise, resolve }
}

/** 让队列的 pump 链跑完一轮微任务（pump 自身是 async）。 */
const tick = () => new Promise((r) => setTimeout(r, 0))

beforeEach(() => {
  ipc.installPlugin.mockReset()
  ipc.removePlugin.mockReset()
  ipc.getShellSettings.mockReset()
  ipc.getShellSettings.mockResolvedValue({})
  useQueueStore.setState({ items: [], running: false, lastFinishedAt: 0 })
})

describe("enqueueAndWait（可 await 句柄）", () => {
  it("成功 → 解析 ok，且队列项落到 done", async () => {
    ipc.installPlugin.mockResolvedValue(ok())
    const outcome = await useQueueStore
      .getState()
      .enqueueAndWait({ pkg: "dsh-pet", spec: "dsh-pet@1.2.3", profile: "web", kind: "install" })

    expect(outcome).toEqual({ ok: true, detail: undefined })
    expect(useQueueStore.getState().items.map((i) => i.status)).toEqual(["done"])
    expect(ipc.installPlugin).toHaveBeenCalledWith("web", "dsh-pet@1.2.3")
  })

  it("软失败（ok:false）→ 解析 ok:false 带 detail（**不抛**）", async () => {
    ipc.installPlugin.mockResolvedValue(softFail("pnpm 被审批门拦住"))
    const outcome = await useQueueStore
      .getState()
      .enqueueAndWait({ pkg: "dsh-pet", spec: "dsh-pet@1.2.3", profile: "web", kind: "install" })

    expect(outcome).toEqual({ ok: false, detail: "pnpm 被审批门拦住" })
    expect(useQueueStore.getState().items[0].status).toBe("failed")
  })

  // 判据：调用方（`runPlan`）用 `requireOk` 判成败。若这里改成 reject，
  // 调用方要靠 try/catch 兜，两套错误通道会让「停在一致态」出现分叉。
  it("IPC 抛错 → 也解析成 ok:false（不 reject）", async () => {
    ipc.installPlugin.mockRejectedValue(new Error("boom"))
    const outcome = await useQueueStore
      .getState()
      .enqueueAndWait({ pkg: "dsh-pet", spec: "dsh-pet@1.2.3", profile: "web", kind: "install" })

    expect(outcome.ok).toBe(false)
    expect(outcome.detail).toContain("boom")
  })

  it("kind=remove → 走 removePlugin（不带 spec），不走 installPlugin", async () => {
    ipc.removePlugin.mockResolvedValue(ok())
    const outcome = await useQueueStore
      .getState()
      .enqueueAndWait({ pkg: "provider-a", spec: "", profile: "web", kind: "remove" })

    expect(outcome.ok).toBe(true)
    expect(ipc.removePlugin).toHaveBeenCalledWith("web", "provider-a")
    expect(ipc.installPlugin).not.toHaveBeenCalled()
  })
})

describe("串行语义（顺序即语义：先移除同族 provider，再装新的）", () => {
  it("前一项终结前，后一项不得发起 IPC", async () => {
    const first = deferred<Outcome>()
    const second = deferred<Outcome>()
    ipc.removePlugin.mockReturnValueOnce(first.promise)
    ipc.installPlugin.mockReturnValueOnce(second.promise)

    const store = useQueueStore.getState()
    const pRemove = store.enqueueAndWait({
      pkg: "old-provider",
      spec: "",
      profile: "web",
      kind: "remove",
    })
    const pInstall = store.enqueueAndWait({
      pkg: "new-provider",
      spec: "new-provider@1.0.0",
      profile: "web",
      kind: "install",
    })

    await tick()
    expect(ipc.removePlugin).toHaveBeenCalledTimes(1)
    // 关键断言：同一时刻只有一个在跑，安装不能抢在卸载之前发出去。
    expect(ipc.installPlugin).not.toHaveBeenCalled()

    first.resolve(ok())
    expect(await pRemove).toEqual({ ok: true, detail: undefined })
    await tick()
    expect(ipc.installPlugin).toHaveBeenCalledTimes(1)

    second.resolve(ok())
    expect(await pInstall).toEqual({ ok: true, detail: undefined })
    expect(useQueueStore.getState().items.map((i) => i.status)).toEqual(["done", "done"])
  })
})

describe("重试边界（有意不自动续跑编排）", () => {
  // 记档边界（`lib/experimentalCapabilities.ts` 生产绑定注释 ②）：面板「重试」只重跑该项，
  // 不会重新解析原 Promise。这里把该行为钉住——若将来改成自动续跑，应当是本测试
  // 先红、再带一个明确的 ADR 决策一起改，而不是悄悄变。
  it("失败后手动重试成功，原 Promise 仍是 ok:false；队列项转 done", async () => {
    ipc.installPlugin.mockResolvedValueOnce(softFail("首次失败"))
    const store = useQueueStore.getState()
    const outcome = await store.enqueueAndWait({
      pkg: "dsh-pet",
      spec: "dsh-pet@1.2.3",
      profile: "web",
      kind: "install",
    })
    expect(outcome.ok).toBe(false)

    const failed = useQueueStore.getState().items[0]
    ipc.installPlugin.mockResolvedValueOnce(ok())
    useQueueStore.getState().retry(failed.id)
    await tick()

    expect(useQueueStore.getState().items[0].status).toBe("done")
  })
})
