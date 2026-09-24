// stores/restartNeededStore.ts —— 「哪些 Profile 有变更待重启才生效」（2026-09-24）。
//
// 单一来历：实验能力面板原先各自持一个本地 `dirty`（ExperimentalCapabilities.tsx），
// 语义 = "这个档的配置被改过，重启才生效"。插件安装/卸载/更新/分发/启停是同一语义，
// 于是提升为**按 profile 名键控的窗口级集合**——全仓只有一个"待重启"真相源，
// 所有提示面（详情页横幅、插件中心横幅）渲染同一个组件（ui/restart-hint.tsx）。
//
// 为什么按名键控而不是单个布尔（原 caps 本地态切档即清）：切走再切回，那个档**仍然**
// 待重启——旧实现把这个事实弄丢了。串档问题由"只给当前选中档渲染"解决，不需要清数据。
//
// 会话级运行态，不持久化；按窗口独立（Zustand 不跨窗，AGENTS §4.4-3——所有产生
// 待重启变更的操作都发生在控制中心窗口内，主窗口无此 UI，无需跨窗同步）。
import { create } from "zustand"

interface RestartNeededState {
  /** 有待重启才生效的变更的 profile 名（有序，重复 mark 去重） */
  pending: string[]
  /** 记上一个档"待重启"。幂等：已在集合里不重复入。 */
  mark: (profile: string) => void
  /** 重启/删除等事实发生后摘除。 */
  clear: (profile: string) => void
}

export const useRestartNeededStore = create<RestartNeededState>((set) => ({
  pending: [],

  mark: (profile) =>
    set((s) =>
      s.pending.includes(profile) ? s : { pending: [...s.pending, profile] },
    ),

  clear: (profile) =>
    set((s) =>
      s.pending.includes(profile) ? { pending: s.pending.filter((p) => p !== profile) } : s,
    ),
}))
