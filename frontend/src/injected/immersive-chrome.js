// 沉浸式标题栏（ADR-0029，2026-09-21）——主窗口「唤醒 dsh 自带桌面 CSS + 驱动窗口拖拽」层。
//
// 做什么：官方 dsh 桌面客户端（Electron）的沉浸式标题栏，真相源在 dsh 自己的 web 前端——
// `packages/client` 里整批按 `html[data-platform='darwin']` 生效的桌面 CSS（页面透明、侧栏
// tint、topStrip 与红绿灯共行），以及 AppFrame 在 darwin 下挂载的**窗口拖拽带**
// （`AppFrame.tsx` 的 `{darwin && <div data-shell-leading-band />}`；CSS 为
// `position:absolute; top:0; left/right:0; height:52px; pointer-events:none`，带会话 tabs
// 时加高到 76px）。壳里它们休眠，本脚本补两件事：
//   ① 补打 dsh 官方 Electron preload 同款标记 `data-platform="darwin"`
//      （`apps/desktop/src/preload-platform.ts:8`）→ dsh 的桌面布局与拖拽带全量生效；
//   ② **自己驱动窗口拖拽**（机制于 2026-09-23 改版，见下）。
//
// 为什么不再翻译 `-webkit-app-region`（2026-09-23 本机 WKWebView 探针实测推翻原方案）：
//   · WKWebView（Tauri 在 macOS 的引擎）**根本不认这个属性**：
//     `CSS.supports('-webkit-app-region','drag') === false`，computed style 与 CSSOM
//     `style.setProperty` 读回**皆为 `""`** ⇒ 原「扫计算样式」方案在 macOS 上恒扫不到元素，
//     一个拖拽属性都打不上（v1.3.0~v1.3.3 的实况，与 ACL 无关）；
//   · 且拖拽带本身是 `pointer-events:none`：Electron 的 app-region 是**几何**语义，而 Tauri 的
//     `data-tauri-drag-region` 是**命中测试**语义（`drag.js` 走 `composedPath`）——属性挂在
//     一个永不成为事件目标的元素上也不会触发。
// 现方案 = 在注入层**复刻 Electron 的几何语义**：命中点落在拖拽带矩形内、且不在 dsh 自己
// 声明的交互元素排除表里（`web/src/base.css:72-78` 的 no-drag 选择器）⇒ 调 Tauri 的
// `startDragging()`；双击 ⇒ `toggleMaximize()`（对齐 Tauri `drag.js` 的 macOS 行为与系统
// 标题栏习惯：按下不算，抬起且未移动才算）。
//
// 为什么不硬编码 dsh 类名：拖拽带由 dsh 自己发布稳定钩子 `data-shell-leading-band`（名字即
// 「给壳用的」），类名哈希与我们无关；钩子缺失（更老的 dsh）时退回「顶部 52px」几何带。
//
// 边界（与既有注入脚本同口径）：
// - 只对**工作台 origin** 生效：同步 hostname=127.0.0.1 乐观命中（dsh 就绪 URL 恒该形态，
//   `shell.rs` 实测），异步 `get_workbench_url` 精确确认，不符即撤；
// - 仅 macOS（`__DSH_PLATFORM__.os === 'macos'`）：Windows 档 2026-09-22 曾进场、
//   **2026-09-23 维护者裁定撤回**（ADR-0030，issue #16 真机事故），与 Linux 同口径
//   保持原生装饰；
// - 全程零色值/零样式表注入——只打标记与读几何，paletteTokens 闸门天然放行；
// - 失败即静默降级：拿不到拖拽带用兜底几何带；Tauri API 不可用则完全不动作。
//
// 纯逻辑与 `frontend/src/lib/immersiveChrome.ts` 同源（该文件有 vitest 覆盖）；
// 本脚本无打包器，改动时两处同步。
(function () {
  if (window.__dshDockImmersiveInjected) return;
  window.__dshDockImmersiveInjected = true;

  // ---- 平台门（2026-09-23 收窄回 macOS）----
  // macOS：打 darwin 标记 + 自驱拖拽（ADR-0029 / 2026-09-23 改版）。
  // Windows：2026-09-23 维护者裁定撤回（ADR-0030，issue #16）——原分支的三条 window 命令不在
  //   `core:window:default` 里（ACL 拒绝且被 `.catch` 吞掉）、dsh 的 Windows 拖拽条是伪元素、
  //   且 `decorations(false)` 会摘掉缩放边框与贴靠 ⇒ 维持原生装饰。
  // Linux 与其它：不打任何桌面标记（官方客户端无 Linux 版，无可对标形态）。
  var platform = window.__DSH_PLATFORM__;
  if (!platform || platform.os !== 'macos') return;

  var MARKER = 'darwin'; // = immersiveChrome.ts 的 DSH_PLATFORM_MARKER
  /** dsh 自己发布的拖拽带钩子（AppFrame.tsx 的 data-shell-leading-band）。 */
  var BAND_SELECTOR = '[data-shell-leading-band]'; // = immersiveChrome.ts 的 LEADING_BAND_SELECTOR
  /** 钩子缺失时的兜底带高（与 dsh 的 .leadingBand 同值：`AppFrame.module.css` 52px）。 */
  var FALLBACK_BAND_HEIGHT = 52; // = immersiveChrome.ts 的 DRAG_BAND_FALLBACK_HEIGHT
  /** dsh 的交互元素排除表（`web/src/base.css:72-78` 的 `-webkit-app-region: no-drag` 列表）。 */
  var EXCLUSION_SELECTOR =
    "button,a,input,select,textarea,summary,[contenteditable='true'],[tabindex]," +
    "[role='dialog'],[role='alertdialog'],[role='menu'],[role='listbox'],[role='tooltip']," +
    "[role='button'],[role='link'],[role='tab'],[role='menuitem'],[role='menuitemcheckbox']," +
    "[role='menuitemradio'],[role='option'],[role='checkbox'],[role='radio'],[role='switch']," +
    "[role='slider'],[role='combobox'],[role='textbox']"; // = immersiveChrome.ts 的 DRAG_EXCLUSION_SELECTOR
  /** dsh 把浮层/门户挂在 `body > :not(#root)`（base.css:60），拖拽面只在 #root 里。 */
  var ROOT_SELECTOR = '#root'; // = immersiveChrome.ts 的 DRAG_SURFACE_ROOT_SELECTOR

  var activated = false;
  var lastPress = null; // 双击判定：{ x, y, moved }

  function invokeTauri(cmd) {
    var tauri = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
    if (!tauri) return Promise.resolve(null);
    return tauri(cmd).catch(function () { return null; });
  }

  function currentWindow() {
    var win = window.__TAURI__ && window.__TAURI__.window;
    if (!win || !win.getCurrentWindow) return null;
    try {
      return win.getCurrentWindow();
    } catch {
      return null;
    }
  }

  // ---- 纯逻辑镜像（lib/immersiveChrome.ts，改逻辑两处同步）----

  function isWorkbenchHostnameSync(hostname) {
    return hostname === '127.0.0.1';
  }

  function syncWorkbenchProbe(locationOrigin, locationHostname, workbenchUrl) {
    if (workbenchUrl !== null) {
      try {
        return new URL(workbenchUrl).origin === locationOrigin ? 'confirm' : 'reject';
      } catch {
        return 'reject';
      }
    }
    return isWorkbenchHostnameSync(locationHostname) ? 'confirm' : 'defer';
  }

  /** 是否该由我们把这次按下变成窗口拖拽（纯逻辑镜像）。 */
  function shouldStartWindowDrag(inBand, insideRoot, onInteractive, fullscreen) {
    return inBand && insideRoot && !onInteractive && !fullscreen;
  }

  // ---- 拖拽带几何 ----

  /** 拖拽带矩形：优先读 dsh 自己的钩子元素，取不到则用顶部兜底带。 */
  function bandRect() {
    var band = document.querySelector(BAND_SELECTOR);
    if (band) {
      var rect = band.getBoundingClientRect();
      if (rect.width > 0 && rect.height > 0) return rect;
    }
    if (typeof window.innerWidth !== 'number') return null;
    return {
      left: 0,
      right: window.innerWidth,
      top: 0,
      bottom: FALLBACK_BAND_HEIGHT,
      width: window.innerWidth,
      height: FALLBACK_BAND_HEIGHT,
    };
  }

  function withinBand(rect, x, y) {
    return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;
  }

  // ---- 自驱拖拽（几何语义，等价于 Electron 的 app-region 合成）----

  function onMouseDown(event) {
    if (!activated || event.button !== 0) return;
    if (event.detail > 2) return; // 三连击及以上交给页面
    var rect = bandRect();
    if (!rect || !withinBand(rect, event.clientX, event.clientY)) return;

    var hit = document.elementFromPoint(event.clientX, event.clientY);
    var insideRoot = !!(hit && hit.closest && hit.closest(ROOT_SELECTOR));
    var onInteractive = !!(hit && hit.closest && hit.closest(EXCLUSION_SELECTOR));
    var fullscreen = document.documentElement.hasAttribute('data-fullscreen');
    if (!shouldStartWindowDrag(true, insideRoot, onInteractive, fullscreen)) return;

    var win = currentWindow();
    if (!win) return;

    // 双击 = 最大化（与 Tauri drag.js 的 macOS 分支同款：**按下不算**，抬起且未移动才算）。
    if (event.detail === 2) {
      lastPress = { x: event.clientX, y: event.clientY, moved: false };
      return;
    }
    lastPress = null;
    // 阻止拖拽期间的文本选择/焦点争夺（Tauri drag.js 同样的 preventDefault）。
    event.preventDefault();
    win.startDragging().catch(function () {});
  }

  function onMouseMove(event) {
    if (!lastPress) return;
    if (Math.abs(event.clientX - lastPress.x) > 4 || Math.abs(event.clientY - lastPress.y) > 4) {
      lastPress.moved = true;
    }
  }

  function onMouseUp(event) {
    var press = lastPress;
    lastPress = null;
    if (!press || press.moved || event.button !== 0) return;
    var win = currentWindow();
    if (!win) return;
    win.toggleMaximize().catch(function () {});
  }

  function installDrag() {
    document.addEventListener('mousedown', onMouseDown, true);
    document.addEventListener('mousemove', onMouseMove, true);
    document.addEventListener('mouseup', onMouseUp, true);
  }

  function uninstallDrag() {
    document.removeEventListener('mousedown', onMouseDown, true);
    document.removeEventListener('mousemove', onMouseMove, true);
    document.removeEventListener('mouseup', onMouseUp, true);
    lastPress = null;
  }

  function activate() {
    if (activated) return;
    activated = true;
    document.documentElement.dataset.platform = MARKER;
    installDrag();
  }

  function deactivate() {
    if (!activated) return;
    activated = false;
    uninstallDrag();
    delete document.documentElement.dataset.platform;
  }

  // ---- 主流程：同步乐观命中 → 异步精确确认（翻盘即撤）----
  function start() {
    var sync = syncWorkbenchProbe(location.origin, location.hostname, null);
    if (sync === 'confirm') activate();
    invokeTauri('get_workbench_url').then(function (url) {
      var verdict = syncWorkbenchProbe(location.origin, location.hostname, url || null);
      if (verdict === 'confirm') activate();
      else if (verdict === 'reject') deactivate();
      // defer：IPC 未就绪且同步未命中——保持现状（壳页面场景，不动作）
    });
  }

  // document-start 时 documentElement 可能尚未创建（WebKit 边界）——兜底等
  // DOMContentLoaded（dsh 官方 preload-platform.ts 同款防御）。
  if (document.documentElement) start();
  else window.addEventListener('DOMContentLoaded', function () { start(); }, { once: true });

})();
