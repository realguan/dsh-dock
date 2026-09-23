// 沉浸式标题栏（immersive chrome）纯模型（2026-09-21，ADR-0029）。
//
// 背景：官方 dsh 桌面客户端（Electron）的沉浸式标题栏，真相源在 **dsh 自己的 web
// 前端**——`packages/client` 里一整批按 `html[data-platform='darwin']` 生效的桌面
// CSS（透明底、侧栏 tint、topStrip 与红绿灯共行、`-webkit-app-region: drag/no-drag`）。
// 壳（Tauri）里这些规则休眠，只差两件事：① 标记没人打；② `-webkit-app-region` 是
// Electron 行为，Tauri 只认 `data-tauri-drag-region` 属性。
//
// **范围（2026-09-23 维护者裁定，ADR-0030）**：仅 macOS。Windows 档 2026-09-22 曾按
// 官方方案进场（打标记 + 壳自绘三控件），2026-09-23 因 issue #16 真机事故整体撤回 ——
// `decorations(false)` 摘掉 `WS_CAPTION | WS_THICKFRAME`（缩放/贴靠退化），自绘控件缺 ACL
// 授权（点了没反应），拖拽条又是伪元素挂不上属性（窗口拖不动）⇒ Windows 与 Linux 同口径
// 维持原生装饰，标记计划与 40px 常量一并从本模块移除。
//
// 本模块是这两件事的**可测折算内核**（纯函数、无 IO）：
//   1. `immersivePlatformAttrFor(os)` —— 壳平台 → dsh 的 `data-platform` 标记值；
//   2. `dragRegionAttrFor(appRegion)` —— dsh 的 app-region 计算样式 → Tauri 拖拽属性值；
//   3. `syncWorkbenchProbe(origin, hostname, workbenchUrl)` —— 「同步宽松命中 +
//      异步精确确认」的 origin 判定（同步命中先画，精确不符再撤）。
// 供 `__tests__/immersiveChrome.test.ts` 覆盖；注入脚本
// `injected/immersive-chrome.js` 内联同一套逻辑（无打包器，改逻辑两处同步）。

/** dsh 官方 Electron preload 打的标记值（`preload-platform.ts:8`；2026-09-21 锚定）。 */
export const DSH_PLATFORM_MARKER = "darwin" as const

/**
 * 同步命中判据：dsh 就绪 URL 恒为 `http://127.0.0.1:<port>`（`shell.rs`
 * `parse_detected_url` 实测系列）。壳页面不可能是这个 hostname：
 * macOS 壳页 `tauri://localhost`（hostname 恰为 localhost，排除）、
 * Windows 壳页 `http://tauri.localhost`、dev 是 `http://localhost:1420`。
 */
export function isWorkbenchHostnameSync(hostname: string): boolean {
  return hostname === "127.0.0.1"
}

/**
 * 壳平台 → `data-platform` 标记值。
 *
 * 仅 macOS：Windows/Linux 保持原生装饰（ADR-0029 §4；Windows 档 2026-09-23 裁定撤回，
 * ADR-0030）。重开 Windows 档的前置条件写在 ADR-0030 —— 不是"补个分支"就能做：
 * dsh 的 Windows 拖拽条是伪元素 `::before`（挂不上 `data-tauri-drag-region`），
 * 且 `decorations(false)` 会带走缩放边框与贴靠。
 */
export function immersivePlatformAttrFor(os: string | undefined): string | null {
  return os === "macos" ? DSH_PLATFORM_MARKER : null
}

/** Tauri 2.11.5 `drag.js` 的属性取值（语义：deep=子树可拖，可点击子元素自动阻断）。 */
export type DragRegionAttr = "deep" | "false"

/**
 * dsh 的 `-webkit-app-region` 计算样式 → Tauri `data-tauri-drag-region` 属性值。
 *
 *  fidelity 要点：dsh 对可拖区域里的**非可点击**子元素（如 `.headerLeading`
 * 这类 div）显式 `no-drag`。Tauri 的 `deep` 会让整个子树可拖，若不把 no-drag
 * 映射成 `false`，这些 div 会把拖拽「漏」进按钮区——逐条映射才与官方同构。
 * 无 app-region 的元素返回 null（不碰）。
 */
export function dragRegionAttrFor(appRegion: string): DragRegionAttr | null {
  const value = appRegion.trim()
  if (value === "drag") return "deep"
  if (value === "no-drag") return "false"
  return null
}

/**
 * 「同步乐观命中 + 异步精确确认」的 origin 判定（switcher.js 同款 origin 口径，
 * ADR-0029 §3）：
 * - 工作台 URL 已知（IPC 已回）→ **精确** origin 比对，说了算（confirm/reject）；
 * - 未知 → 同步 hostname 判据兜底：127.0.0.1 乐观 confirm（抢先画，避免首帧闪
 *   不透明侧栏），其余 defer（等 IPC）。
 *
 * 返回 "reject" 时调用方必须**撤标记 + 断观察器**——同步乐观命中被精确判定翻盘
 * 的唯一路径（例如壳未来改用 localhost 形态的就绪 URL）。
 */
export function syncWorkbenchProbe(
  locationOrigin: string,
  locationHostname: string,
  workbenchUrl: string | null,
): "defer" | "confirm" | "reject" {
  if (workbenchUrl !== null) {
    try {
      return new URL(workbenchUrl).origin === locationOrigin ? "confirm" : "reject"
    } catch {
      return "reject"
    }
  }
  return isWorkbenchHostnameSync(locationHostname) ? "confirm" : "defer"
}
