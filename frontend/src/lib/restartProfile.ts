// lib/restartProfile.ts —— 「重启这个 Profile」的统一出口（2026-09-24）。
//
// 为什么要一个出口：应用里凡有"变更需重启才生效"的地方（实验能力横幅、插件安装/
// 卸载后的提示）都给一个重启按钮。若各写各的 invoke，就是"一种逻辑两种交互"——
// 按钮样式、确认口径、失败反馈都会漂移。这里收口为一条链：
//
//   按钮 → requestRestartProfile(profile)
//        → 已注册处理器（ProfileManager 挂载时注册 handleRestart：走既有的
//          ProfileSwitchDialog 确认 → startHandoff，与左栏行的「重启」完全同链）；
//        → 无处理器（本窗口没挂 ProfileManager，如 about/main）→ 直联壳的
//          switch_profile（Rust 按 目标==活跃会话 自判 Restart）。
//
// 模块级单例与 queueStore 的 notifier 同惯例（stores/queueStore.ts）：跨组件通信
// 不经 props 链，注册/注销由宿主页面负责。
import { api } from "@/lib/tauri"

type RestartHandler = (profile: string) => void

let handler: RestartHandler | null = null

/** 宿主页面（ProfileManager）挂载时注册；卸载传 null（热更新/关窗都会走）。 */
export function setRestartProfileHandler(next: RestartHandler | null): void {
  handler = next
}

/** 触发重启。有处理器走既有确认链；没有则直联壳（失败静默——调用方都是提示条，
 *  真正可见的反馈在主窗口的交接导轨上）。 */
export function requestRestartProfile(profile: string): void {
  if (handler) {
    handler(profile)
    return
  }
  void api.switchProfile(profile).catch(() => {})
}
