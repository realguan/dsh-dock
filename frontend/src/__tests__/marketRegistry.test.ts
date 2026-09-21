// marketRegistry.test.ts —— 市场注册表**共享缓存**的闸门（2026-09-21）。
//
// ## 事故形态（维护者实测："每次点好像都重新获取插件市场数据"）
// 两个消费方各自持有模块级缓存变量：
//   · `market/MarketplaceView.tsx` —— 插件中心 → 市场
//   · `profiles/PluginAddDialog.tsx` —— 添加插件弹窗 → 市场
// 变量互不可见 ⇒ 从任一入口切到另一个**必然重新拉取**：同一份 JSON、同一个 session、
// 两次网络往返。缓存的全部意义就是跨入口共享，各存一份等于没缓存。
//
// ## 本闸门测什么
// ① **行为**：把 `fetchMarketRegistry` 换成计数桩，直接问"调了几次"——命中缓存、
//    force 覆盖、并发去重、失败不污染缓存，四件事逐条钉死（不是读源码文本）；
// ② **结构**：两个消费方都必须走共享层，且**不得**再各自声明模块级缓存变量
//    （这正是事故的形态，不拦就会重演）。
//
// 每个用例用 `vi.resetModules()` + 动态 import 拿干净的模块状态（模块级变量必须重置）。
import { beforeEach, describe, expect, it, vi } from "vitest"

import marketViewSrc from "@/components/market/MarketplaceView.tsx?raw"
import addDialogSrc from "@/components/profiles/PluginAddDialog.tsx?raw"

const ipc = { fetchMarketRegistry: vi.fn() }
vi.mock("@/lib/tauri", () => ({ api: ipc }))

const REGISTRY = JSON.stringify({ plugins: [{ name: "a" }], generated_at: "" })
const EMPTY = JSON.stringify({ plugins: [], generated_at: "" })

// 每个用例一份全新模块状态（模块级 cache/inflight 必须从零开始）。
// 注意顺序：清理放 `beforeEach`，**不能**放 `fresh()`——用例是先设桩再调 fresh，
// 在 fresh 里 reset 会把刚设的实现一起清掉（本文件首版就踩了这个）。
beforeEach(() => {
  vi.resetModules()
  ipc.fetchMarketRegistry.mockReset()
})

async function fresh() {
  return await import("@/lib/marketRegistry")
}

describe("① 行为：缓存命中 / force / 并发去重 / 失败语义", () => {
  it("命中缓存：第二次调用不再发请求，且两次拿到**同一个对象**", async () => {
    ipc.fetchMarketRegistry.mockResolvedValue(REGISTRY)
    const m = await fresh()
    const a = await m.loadMarketRegistry()
    const b = await m.loadMarketRegistry()
    expect(ipc.fetchMarketRegistry, "第二次应命中缓存，不该再发请求").toHaveBeenCalledTimes(1)
    // 同一引用 = 真共享；若两处各解析一份，跨入口就还是两份数据
    expect(b).toBe(a)
    expect(m.peekMarketRegistry()).toBe(a)
  })

  it("force 绕过缓存并覆盖（「刷新」按钮要能让两个入口都拿到新数据）", async () => {
    ipc.fetchMarketRegistry.mockResolvedValueOnce(REGISTRY).mockResolvedValueOnce(EMPTY)
    const m = await fresh()
    expect((await m.loadMarketRegistry()).plugins).toHaveLength(1)
    const after = await m.loadMarketRegistry(true)
    expect(ipc.fetchMarketRegistry, "force 应真的重取").toHaveBeenCalledTimes(2)
    expect(after.plugins).toHaveLength(0)
    expect(m.peekMarketRegistry(), "缓存要被新数据覆盖").toBe(after)
  })

  it("并发去重：同时打两个入口只发一次请求", async () => {
    let release: (v: string) => void = () => {}
    ipc.fetchMarketRegistry.mockReturnValue(
      new Promise<string>((resolve) => {
        release = resolve
      }),
    )
    const m = await fresh()
    const p1 = m.loadMarketRegistry()
    const p2 = m.loadMarketRegistry()
    release(REGISTRY)
    const [r1, r2] = await Promise.all([p1, p2])
    expect(ipc.fetchMarketRegistry, "并发应共享同一趟请求").toHaveBeenCalledTimes(1)
    expect(r2).toBe(r1)
  })

  it("失败不写缓存：一次网络抖动不被固化成「这个 session 的市场是空的」", async () => {
    ipc.fetchMarketRegistry
      .mockRejectedValueOnce(new Error("offline"))
      .mockResolvedValueOnce(REGISTRY)
    const m = await fresh()
    await expect(m.loadMarketRegistry()).rejects.toThrow("offline")
    expect(m.peekMarketRegistry(), "失败不得留缓存").toBeNull()
    await expect(m.loadMarketRegistry(), "下一次应能重试").resolves.toBeTruthy()
    expect(ipc.fetchMarketRegistry).toHaveBeenCalledTimes(2)
  })

  it("invalidate 后可重取（显式失效入口）", async () => {
    ipc.fetchMarketRegistry.mockResolvedValue(REGISTRY)
    const m = await fresh()
    await m.loadMarketRegistry()
    m.invalidateMarketRegistry()
    expect(m.peekMarketRegistry()).toBeNull()
    await m.loadMarketRegistry()
    expect(ipc.fetchMarketRegistry).toHaveBeenCalledTimes(2)
  })
})

describe("② 结构：两个消费方共用同一份（禁各自再存一份）", () => {
  it("插件中心市场视图走共享层，且不再自己持有缓存变量", () => {
    expect(marketViewSrc, "应导入共享数据层").toContain("@/lib/marketRegistry")
    expect(marketViewSrc, "读缓存用 peek").toContain("peekMarketRegistry")
    expect(marketViewSrc, "取数用 load").toContain("loadMarketRegistry")
    // 事故形态的反向闸门：模块级 `let cachedRegistry` 不许回来
    expect(marketViewSrc, "不得再声明模块级缓存").not.toMatch(/^let cachedRegistry/m)
    expect(marketViewSrc, "不得再直呼 IPC 取市场").not.toContain("api.fetchMarketRegistry")
  })

  it("添加插件弹窗走共享层，且不再自己持有缓存变量", () => {
    expect(addDialogSrc, "应导入共享数据层").toContain("@/lib/marketRegistry")
    expect(addDialogSrc, "读缓存用 peek").toContain("peekMarketRegistry")
    expect(addDialogSrc, "取数用 load").toContain("loadMarketRegistry")
    expect(addDialogSrc, "不得再声明模块级缓存").not.toMatch(/^let cachedRegistry/m)
    expect(addDialogSrc, "不得再直呼 IPC 取市场").not.toContain("api.fetchMarketRegistry")
  })
})

describe("③ 弹窗宽度：必须用 sm: 前缀盖住 DialogContent 的默认值", () => {
  it("DialogContent 默认 sm:max-w-sm，无前缀的 max-w-* 会被它压掉", () => {
    // Tailwind 把带响应式前缀的规则**后生成**，同特异性下后者胜——`max-w-2xl`
    // （无前缀）斗不过 `sm:max-w-sm`，弹窗在 ≥640px 窗口里只剩 384px。
    // 2026-09-21 维护者截图："这个窗口还不协调"的主因就是这个。
    expect(addDialogSrc, "宽度须带 sm: 前缀").toMatch(/<DialogContent className="sm:max-w-\w+/)
    expect(addDialogSrc, "不得再用无前缀宽度覆写").not.toMatch(
      /<DialogContent className="max-w-/,
    )
  })
})
