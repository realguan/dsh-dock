// marketRegistry.ts —— 插件市场注册表的**唯一**数据层（2026-09-21 收口）。
//
// ## 为什么必须收成一处（维护者实测报"每次点好像都重新拉"）
//
// 原先两个消费方各自持有模块级缓存：
//   · `market/MarketplaceView.tsx` —— 插件中心 → 市场
//   · `profiles/PluginAddDialog.tsx` —— 添加插件弹窗 → 市场
// 两个变量互不可见，于是**从任一入口切到另一个入口必然重新拉取**：同一份 JSON、
// 同一个 session、两次网络往返。缓存的全部意义就是跨入口共享，各存一份等于没缓存。
// 并发的两个入口还会双发（原先无 inflight 去重）。
//
// ## 口径（保留原先已工作的语义，只把"份数"从 2 收到 1）
//   · cache    —— session 内复用（原 `cachedRegistry` 的语义照搬）；
//   · inflight —— 并发去重：同时打两个入口只发一次请求，后到者共享同一 Promise；
//   · force    —— 「刷新」按钮用：绕缓存重取并**覆盖**，两个入口同时拿到新数据；
//   · 失败**不写缓存**：一次网络抖动不该被固化成"这次 session 的市场是空的"。
//
// 网络面归 Rust（AGENTS §4.4 红线 2），本模块只做取数编排，不发新网络请求。

import { api } from "@/lib/tauri"
import type { MarketRegistry } from "@/types/market"

let cache: MarketRegistry | null = null
let inflight: Promise<MarketRegistry> | null = null

/** 同步读缓存：让消费方首帧就能渲染已知道的数据，避免"打开弹窗先闪一下 loading"。 */
export function peekMarketRegistry(): MarketRegistry | null {
  return cache
}

/**
 * 取注册表。命中缓存即返回；并发调用共享同一次请求；`force=true` 绕过缓存重取。
 * 失败时抛出（调用方各自决定怎么显示错误），且不污染缓存。
 */
export function loadMarketRegistry(force = false): Promise<MarketRegistry> {
  if (!force && cache !== null) return Promise.resolve(cache)
  if (inflight !== null) return inflight
  inflight = api
    .fetchMarketRegistry()
    .then((raw) => {
      const parsed = JSON.parse(raw) as MarketRegistry
      cache = parsed
      return parsed
    })
    .finally(() => {
      // 无论成败都清空 inflight：失败后下一次调用应当能重试。
      inflight = null
    })
  return inflight
}

/** 丢弃缓存（`force` 刷新之外的显式失效入口；留给将来切档/登出等场景）。 */
export function invalidateMarketRegistry(): void {
  cache = null
}
