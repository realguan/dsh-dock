// SystemBaseSection.tsx —— 「系统预置底座与运行时」下沉容器（2026-09-21 结构收敛：
// 自 ProfileDetailPane 抽出）。
//
// ## 为什么它必须与用户扩展**视觉上不同级**
// 这批包（dsh-base / dsh-web-app / 桌面运行时）由桌面系统统一部署，**既不能卸也不能
// 开关**。方案一要求把"用户能掌控的"与"系统自带的"物理分开——否则用户第一眼看到的是
// 一堆点不动的行，会以为界面坏了（这正是维护者最初截图里"底座凭什么排在最上面"的抱怨）。
//
// 视觉语言因此刻意与用户区分开：
//   · 用户扩展区：`bg-panel` + `shadow-xs`（浮起 = 这是你的东西）；
//   · 系统底座区：`bg-bg/50` + 更淡的边框、**无阴影**（沉下去 = 这是背景设施）。
//
// ## 为什么没有标题栏与折叠（2026-09-21 维护者真机裁定）
// 原版这里有一条头部：「系统预置底座与运行时 (N)」+「由桌面系统统一部署，不可变更」+
// 「收起 ⌄」，默认折叠。维护者在「内置」筛选下把整条头部圈了出来 —— 同一屏里
// **筛选芯片/当前 tab 已经写着"内置"**，头部再写一遍就是复述（与"社区 tab 里不写
// 「社区」"同一条判据）；而"不可变更"这件事，行内零控制面本身就在说。
// 故头部与折叠态整体退役：本组件只剩这层**下沉容器**，行照旧没有任何控制面。
//
// 顺序仍由父级保证（用户扩展在前、本区沉底）——这是最初那条抱怨的正解，不能一起丢。

import type { ReactNode } from "react"

export interface SystemBaseSectionProps {
  children: ReactNode
}

export function SystemBaseSection({ children }: SystemBaseSectionProps) {
  return (
    <section className="divide-line/60 border-line/60 bg-bg/50 divide-y overflow-hidden rounded-xl border">
      {children}
    </section>
  )
}
