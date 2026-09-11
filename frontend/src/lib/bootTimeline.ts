// 启动屏「展开/收起时间线」的纯逻辑（2026-09-10；v1.2.0 修订 2026-09-11）。
//
// 为什么单独成文件：这段判据曾写死在 `BootIndex.tsx` 的 JSX 条件里，两处用了
// 不同的信号，导致「点开详情后回不去」——把判据抽成纯函数才能被测试钉住
// （仓库口径：Vitest 只测纯逻辑，不引 DOM 测试栈）。

/**
 * 用户对「启动详情」的显式意图。
 *
 * - `auto`：用户没表态，由自动信号（下载/错误）决定；
 * - `expanded`：用户点过「查看启动详情」；
 * - `collapsed`：用户点过「收起详情」。
 *
 * **为什么必须是三态而不是一个 `forceShowTimeline` 布尔**（v1.2.0 实测 1.2）：
 * 布尔表达不了「用户主动收起」——收起只能把标志置回 false，而自动信号仍然为真，
 * 于是收起点不动任何东西（死按钮）。三态让**显式意图压过自动信号**，
 * 收起才真正生效。两个布尔（forceShow + userCollapsed）也能表达，但会出现
 * 「同时展开又收起」的非法组合；三态从类型上排除该状态。
 */
export type TimelineIntent = "auto" | "expanded" | "collapsed"

/** 启动屏的可见状态（决定展示「极简秒启」还是「五步向导」）。 */
export interface TimelineVisibility {
  /** 是否已有错误。 */
  hasError: boolean
  /** 本次启动是否**收到过下载进度**（boot:progress）。 */
  hasEverDownloaded: boolean
  /** 下载正在进行中。 */
  inDownload: boolean
  /** 用户对详情的显式意图（见 `TimelineIntent`）。 */
  userIntent: TimelineIntent
}

/**
 * 是否展示五步向导列表。
 *
 * 日常秒启走极简启动屏；只有发生过环境下载、正在进行下载、出错、
 * 或用户主动要求时才展开。
 *
 * **显式意图优先**：用户点过「收起详情」后，即使错误仍在、下载仍进行，
 * 也不再自动展开——否则收起按钮永远无效（见 `TimelineIntent` 注释）。
 */
export function shouldShowTimeline(v: TimelineVisibility): boolean {
  if (v.userIntent === "collapsed") return false
  if (v.userIntent === "expanded") return true
  return v.hasEverDownloaded || v.inDownload || v.hasError
}

/**
 * 是否给出「收起详情」出口。
 *
 * **2026-09-10 修复（v1.1.0 Windows 实测 1.1b）**：原条件只看
 * `hasEverDownloaded`，而该标志**只在收到 `boot:progress` 时置真**——WSL 客体
 * 引擎引导全程只发 `boot:step`、从不发 progress（`executor.rs::ensure_guest_engine`
 * 里没有任何 progress 回调），于是标志恒假 → 用户点开「查看启动详情」后
 * **页面上没有任何收起入口**，只能重启应用。
 *
 * **2026-09-11 v1.2.0 实测 1.2 修复**：原实现还有 `if (hasError) return false`
 * 的一票否决，理由是「错误信息必须可见」。但该理由**不成立**：
 * `ErrorCard` 渲染在时间线区块**之外**（`pages/BootIndex.tsx:279-296`，
 * 与 `isSetupMode ? <BootTimeline/> : <splash/>` 是兄弟节点），收起向导
 * **不会藏掉任何错误信息**。于是否决只剩副作用：截图 ①/② 那种「错误出现于
 * 用户点开详情之后」的路径上，向导是用户自己开的却收不回。
 *
 * 现行判据：**只要这一屏还在展示，就必须让用户能收起**——
 * 错误由 `ErrorCard` 独立承担可见性，两者不再互相顶着。
 */
export function shouldOfferTimelineCollapse(v: TimelineVisibility): boolean {
  return shouldShowTimeline(v)
}
