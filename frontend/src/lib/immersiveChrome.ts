// 沉浸式标题栏（immersive chrome）纯模型（2026-09-21，ADR-0029；拖拽机制 2026-09-23 改版）。
//
// 背景：官方 dsh 桌面客户端（Electron）的沉浸式标题栏，真相源在 **dsh 自己的 web
// 前端**——`packages/client` 里一整批按 `html[data-platform='darwin']` 生效的桌面
// CSS（透明底、侧栏 tint、topStrip 与红绿灯共行），以及 AppFrame 在 darwin 下挂载的
// 窗口拖拽带（`data-shell-leading-band`，52px / 带 tabs 时 76px、`pointer-events:none`）。
// 壳（Tauri）里这些规则休眠，只差两件事：① 标记没人打；② **拖拽没人驱动**。
//
// **2026-09-23 改版（本机 WKWebView 探针实测）**：原方案「扫 `-webkit-app-region` 计算样式
// → 翻译成 `data-tauri-drag-region`」在 macOS 上不成立 —— WKWebView 不认该属性
// （`CSS.supports` 为 false、CSSOM 读回为空），且拖拽带是 `pointer-events:none`
// （Electron 的 app-region 是几何语义，Tauri 的是命中测试语义）。现改为在注入层复刻
// 几何语义：见 `dragDecisionFor`。
//
// **范围（2026-09-23 维护者裁定，ADR-0030）**：仅 macOS。Windows 档 2026-09-22 曾按
// 官方方案进场（打标记 + 壳自绘三控件），2026-09-23 因 issue #16 真机事故整体撤回 ——
// `decorations(false)` 摘掉 `WS_CAPTION | WS_THICKFRAME`（缩放/贴靠退化），自绘控件缺 ACL
// 授权（点了没反应），拖拽条又是伪元素挂不上属性（窗口拖不动）⇒ Windows 与 Linux 同口径
// 维持原生装饰，标记计划与 40px 常量一并从本模块移除。
//
// 本模块是可测折算内核（纯函数、无 IO）：
//   1. `immersivePlatformAttrFor(os)` —— 壳平台 → dsh 的 `data-platform` 标记值；
//   2. `dragDecisionFor({inBand, insideRoot, onInteractive, fullscreen})` —— 按下时是否
//      由壳接管为窗口拖拽（几何语义）；
//   3. `syncWorkbenchProbe(origin, hostname, workbenchUrl)` —— 「同步宽松命中 +
//      异步精确确认」的 origin 判定（同步命中先画，精确不符再撤）。
// 供 `__tests__/immersiveChrome.test.ts` 覆盖；注入脚本
// `injected/immersive-chrome.js` 内联同一套逻辑与常量（无打包器，改逻辑两处同步；
// 常量一致性由 `ui.rs` 的 `immersive_drag_acl_tests` 机器闸门钉住）。

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

/**
 * dsh 自己发布的窗口拖拽带钩子（`AppFrame.tsx` 的 `{darwin && <div data-shell-leading-band />}`）。
 * 名字即用途：给壳用的稳定标记 —— 我们据此取拖拽带几何，不必碰带哈希的类名。
 */
export const LEADING_BAND_SELECTOR = "[data-shell-leading-band]"

/** 钩子缺失（更老的 dsh）时的兜底带高，与 dsh `.leadingBand` 同值（`AppFrame.module.css` 52px）。 */
export const DRAG_BAND_FALLBACK_HEIGHT = 52

/**
 * dsh 的交互元素排除表 —— `packages/client/web/src/base.css:72-78` 里
 * `html[data-platform='darwin'] :is(...) { -webkit-app-region: no-drag }` 的原样镜像。
 * 注入脚本里同名常量必须逐字一致（`ui.rs` 的 `immersive_drag_acl_tests` 有机器闸门钉住）。
 */
export const DRAG_EXCLUSION_SELECTOR =
  "button,a,input,select,textarea,summary,[contenteditable='true'],[tabindex]," +
  "[role='dialog'],[role='alertdialog'],[role='menu'],[role='listbox'],[role='tooltip']," +
  "[role='button'],[role='link'],[role='tab'],[role='menuitem'],[role='menuitemcheckbox']," +
  "[role='menuitemradio'],[role='option'],[role='checkbox'],[role='radio'],[role='switch']," +
  "[role='slider'],[role='combobox'],[role='textbox']"

/** dsh 把浮层/门户挂在 `body > :not(#root)`（base.css:60）：拖拽面只在 `#root` 内。 */
export const DRAG_SURFACE_ROOT_SELECTOR = "#root"

/** 一次按下的拖拽裁定：drag = 交给 `startDragging()`；skip = 让页面照常处理。 */
export type DragDecision = "drag" | "skip"

/**
 * 几何语义的拖拽裁定（2026-09-23 改版内核）。
 *
 * 为什么不是「读 `-webkit-app-region` 计算样式」：WKWebView 不认该属性（`CSS.supports`
 * 为 false、CSSOM 读回为空），且 dsh 的拖拽带是 `pointer-events:none`，Tauri 的
 * `data-tauri-drag-region` 走命中测试 —— 两头都到不了。故改为在注入层复刻 Electron 的
 * 几何合成：**带内 ∧ 在 #root 内 ∧ 不落在交互元素上 ∧ 非全屏** 才算拖拽。
 *
 * @param inBand 命中点是否落在拖拽带矩形内
 * @param insideRoot 命中元素是否在 `#root` 内（浮层/门户/壳自绘胶囊都在 body 下，属 no-drag）
 * @param onInteractive 命中元素是否落在 dsh 的交互元素排除表上
 * @param fullscreen 是否处于全屏（全屏下无标题栏语义，拖拽无意义）
 */
export function dragDecisionFor(input: {
  inBand: boolean
  insideRoot: boolean
  onInteractive: boolean
  fullscreen: boolean
}): DragDecision {
  return input.inBand && input.insideRoot && !input.onInteractive && !input.fullscreen
    ? "drag"
    : "skip"
}

/**
 * 「同步乐观命中 + 异步精确确认」的 origin 判定（switcher.js 同款 origin 口径，
 * ADR-0029 §3）：
 * - 工作台 URL 已知（IPC 已回）→ **精确** origin 比对，说了算（confirm/reject）；
 * - 未知 → 同步 hostname 判据兜底：127.0.0.1 乐观 confirm（抢先画，避免首帧闪
 *   不透明侧栏），其余 defer（等 IPC）。
 *
 * 返回 "reject" 时调用方必须**撤标记 + 断拖拽监听**——同步乐观命中被精确判定翻盘
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
