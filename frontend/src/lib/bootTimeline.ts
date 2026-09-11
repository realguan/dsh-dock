// 启动屏「展开/收起时间线」的纯逻辑（2026-09-10）。
//
// 为什么单独成文件：这段判据曾写死在 `BootIndex.tsx` 的 JSX 条件里，两处用了
// 不同的信号，导致「点开详情后回不去」——把判据抽成纯函数才能被测试钉住
// （仓库口径：Vitest 只测纯逻辑，不引 DOM 测试栈）。

/** 启动屏的可见状态（决定展示「极简秒启」还是「五步向导」）。 */
export interface TimelineVisibility {
  /** 是否已有错误（出错必须展开向导，且不给"收起"出口）。 */
  hasError: boolean
  /** 本次启动是否**收到过下载进度**（boot:progress）。 */
  hasEverDownloaded: boolean
  /** 下载正在进行中。 */
  inDownload: boolean
  /** 用户手动点过「查看启动详情」。 */
  forceShowTimeline: boolean
}

/**
 * 是否展示五步向导列表。
 *
 * 日常秒启走极简启动屏；只有发生过环境下载、出错、或用户主动要求时才展开。
 */
export function shouldShowTimeline(v: TimelineVisibility): boolean {
  return v.hasEverDownloaded || v.inDownload || v.hasError || v.forceShowTimeline
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
 * 正确判据：**只要这一屏是用户自己要开的，就必须让他能关**。
 * 出错时不给收起（错误信息是必须看见的）。
 */
export function shouldOfferTimelineCollapse(v: TimelineVisibility): boolean {
  if (v.hasError) return false
  return v.forceShowTimeline || v.hasEverDownloaded
}
