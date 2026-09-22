// 壳能力（2026-09-21，§3.6）：判断要不要在窗口内补「关于 / 更新」入口。
//
// 背景：原 `open_about` IPC 于 2026-08-27 因「与原生常驻入口重复」删除；该前提在
// **无 StatusNotifier 宿主的 Linux 桌面**上不成立（托盘建不起来 ⇒ 既看不到更新、也打不开关于）。
// 故由壳上报"常驻入口是否可达"，前端**只在那一种场景**补入口 —— 既不重现当年的重复按钮，
// 也不让用户困在没有入口的状态里。

import { api } from "@/lib/tauri"
import { logger } from "@/lib/logger"

export interface ShellCapabilities {
  /** 常驻入口（macOS 菜单栏 / 其余平台托盘）是否可达 */
  residentEntryAvailable: boolean
}

/** 拉取失败时的判定（**显式选择"补入口"**）：未知不等于"有入口"。
 *
 * 为什么偏向补：漏补 = 用户在这类桌面上**没有**任何通往关于/更新的路（功能缺失）；
 * 多补 = 多一个按钮（观感瑕疵，且只在拉取失败这种罕见情形）。两害相权取其轻。 */
export const CAPABILITIES_FALLBACK: ShellCapabilities = { residentEntryAvailable: false }

/** 是否应显示窗口内的「关于 / 更新」入口（**纯函数**，可测）。 */
export function shouldShowAboutEntry(caps: ShellCapabilities | null): boolean {
  if (caps === null) return true // 未知 ⇒ 补（见 CAPABILITIES_FALLBACK 的理由）
  return !caps.residentEntryAvailable
}

/** 取壳能力（失败如实记日志并回退到"补入口"）。 */
export async function fetchShellCapabilities(): Promise<ShellCapabilities> {
  try {
    const caps = await api.getShellCapabilities()
    return { residentEntryAvailable: caps?.residentEntryAvailable === true }
  } catch (e) {
    logger.warn("shellCapabilities", "取壳能力失败，按「补入口」处理", { error: String(e) })
    return CAPABILITIES_FALLBACK
  }
}
