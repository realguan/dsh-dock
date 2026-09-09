// stores/installFlightStore.ts —— 飞行动画的会话级请求槽（2026-09-09，
// 问题记录-2026-09-09 §1.1）。
//
// 为什么单独一个 store：入队发生在两个不同组件（市场安装弹窗 / 总览分发弹窗），
// 渲染发生在共同祖先（PluginHub 的飞行层），角标弹跳发生在 QueuePanel。三方都
// 不该互相引用，故经此槽位通信：`launch()` 写入请求 → 飞行层消费 → 落地时
// `land()` 清槽并打上 `landedAt` 供角标播放落点动效。
//
// 会话级运行态，不持久化。同时只保留一个请求：连点两次时后一次直接接管
// （重挂 key 会从新起点重飞，比排两条互撞的轨迹更好看）。
import { create } from "zustand"
import type { FlightPoint } from "@/lib/installFlight"

export interface FlightRequest {
  id: string
  /** 胶囊上的文字（插件展示名） */
  label: string
  /** 起点：点击处的视口坐标 */
  from: FlightPoint
}

interface InstallFlightState {
  request: FlightRequest | null
  /** 最近一次落地的时刻（角标据此播放弹跳；0 = 本次会话还没落地过） */
  landedAt: number
  launch: (from: FlightPoint, label: string) => void
  land: () => void
}

export const useInstallFlightStore = create<InstallFlightState>((set) => ({
  request: null,
  landedAt: 0,

  launch: (from, label) =>
    set(() => ({
      request: {
        id: `f-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
        label,
        from,
      },
    })),

  // 空槽时 no-op：飞行层挂载/卸载都会调用，不能让 landedAt 平白跳一次。
  land: () =>
    set((s) => (s.request === null ? s : { request: null, landedAt: Date.now() })),
}))
