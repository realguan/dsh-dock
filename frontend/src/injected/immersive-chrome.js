// 沉浸式标题栏（ADR-0029，2026-09-21）——主窗口「唤醒 dsh 自带桌面 CSS + 翻译拖拽语义」层。
//
// 做什么：官方 dsh 桌面客户端的沉浸式标题栏，真相源在 dsh 自己的 web 前端——
// `packages/client` 里整批按 `html[data-platform='darwin']` 生效的桌面 CSS（页面透明、
// 侧栏 tint、topStrip 与红绿灯共行、`-webkit-app-region: drag/no-drag`，源锚见
// ADR-0029 §1 表）。壳里它们休眠，只差两件事，本脚本就是这两件事：
//   ① 补打 dsh 官方 Electron preload 同款标记 `data-platform="darwin"`
//      （`apps/desktop/src/preload-platform.ts:8`）→ dsh 的桌面布局全量生效；
//   ② 把 dsh CSS 的 `-webkit-app-region` 计算样式**翻译**成 Tauri 的
//      `data-tauri-drag-region` 属性（`-webkit-app-region` 是 Electron 行为，
//      Tauri 只认属性；语义对照见 tauri 2.11.5 `window/scripts/drag.js`）。
//
// 为什么不硬编码选择器：dsh 前端 CSS Modules 类名带构建哈希，每次升级都可能变；
// 按**计算样式**扫描与哈希无关（ADR-0029 §3 方案 A）。
//
// 边界（与既有注入脚本同口径）：
// - 只对**工作台 origin** 生效：同步 hostname=127.0.0.1 乐观命中（dsh 就绪 URL 恒该
//   形态，`shell.rs` 实测），异步 `get_workbench_url` 精确确认，不符即撤；
// - v1 仅 macOS（`__DSH_PLATFORM__.os === 'macos'`）：Win/Linux 无红绿灯且 Tauri
//   稳定版无 titleBarOverlay 等价物，保持原生装饰；
// - 全程零色值/零样式表注入——只打标记与属性，paletteTokens 闸门天然放行；
// - 失败即静默降级：扫描不到 app-region = 回到原生标题栏，不坏工作台任何功能。
//
// 纯逻辑与 `frontend/src/lib/immersiveChrome.ts` 同源（该文件有 vitest 覆盖）；
// 本脚本无打包器，改动时两处同步。
(function () {
  if (window.__dshDockImmersiveInjected) return;
  window.__dshDockImmersiveInjected = true;

  // ---- 平台分支（2026-09-21 扩展）----
  // macOS：打 darwin 标记 + 翻译 app-region 拖拽（ADR-0029 原范围）。
  // Windows：对标官方 preload-windows.ts —— 打 data-windows-titlebar + 40px 高度变量
  //   （页面据此预留标题栏带并自绘拖拽条，与官方逐字一致），外加**自绘**最小化/最大化/关闭
  //   （官方用 Electron 的原生 titleBarOverlay，Tauri 无等价 API ⇒ 唯一偏差，已登记）。
  // Linux 与其它：不打任何桌面标记（官方客户端无 Linux 版，无可对标形态）。
  var platform = window.__DSH_PLATFORM__;
  if (!platform) return;
  if (platform.os === 'windows') {
    installWindowsTitlebar();
    return;
  }
  if (platform.os !== 'macos') return;

  var MARKER = 'darwin'; // = immersiveChrome.ts 的 DSH_PLATFORM_MARKER
  var DRAG_ATTR = 'data-tauri-drag-region';
  var activated = false;
  var observer = null;

  function invokeTauri(cmd) {
    var tauri = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
    if (!tauri) return Promise.resolve(null);
    return tauri(cmd).catch(function () { return null; });
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

  function dragRegionAttrFor(appRegion) {
    var value = String(appRegion).trim();
    if (value === 'drag') return 'deep';
    if (value === 'no-drag') return 'false';
    return null;
  }

  // ---- app-region → data-tauri-drag-region 翻译 ----
  //
  // fidelity 要点：dsh 对可拖区域里的**非可点击**子元素（如 headerLeading 这类
  // div）显式 no-drag；Tauri 的 deep 会让整个子树可拖，不把 no-drag 映射成
  // false，这些 div 就会把拖拽"漏"进按钮区。
  function applyAttr(el) {
    if (!(el instanceof HTMLElement)) return;
    var region;
    try {
      region = getComputedStyle(el).getPropertyValue('-webkit-app-region');
    } catch {
      return; // 极端环境取不到计算样式：不碰
    }
    var attr = dragRegionAttrFor(region);
    if (attr === null) {
      // 类名翻转导致 app-region 消失：撤掉我们此前设的属性（自愈，不等整页重扫）。
      if (el.hasAttribute(DRAG_ATTR)) el.removeAttribute(DRAG_ATTR);
      return;
    }
    // 只在值变化时写——避免触发我们自己的 observer 回环（且 attributeFilter
    // 不含 DRAG_ATTR，双保险）。
    if (el.getAttribute(DRAG_ATTR) !== attr) el.setAttribute(DRAG_ATTR, attr);
  }

  function scanTree(node) {
    if (node instanceof HTMLElement) applyAttr(node);
    if (!node.querySelectorAll) return; // Text 等无子树
    var descendants = node.querySelectorAll('*');
    for (var i = 0; i < descendants.length; i++) applyAttr(descendants[i]);
  }

  function scanAll() {
    var all = document.querySelectorAll('*');
    for (var i = 0; i < all.length; i++) applyAttr(all[i]);
  }

  function activate() {
    if (activated) return;
    activated = true;
    document.documentElement.dataset.platform = MARKER;
    scanAll();
    // dsh 工作台是 SPA：titleRow/topStrip 等会被路由切换整体替换，流式输出也会
    // 插新节点——observer 跟随。attributes 只监 class/style（拖拽相关样式的来源）。
    observer = new MutationObserver(function (records) {
      for (var i = 0; i < records.length; i++) {
        var r = records[i];
        if (r.type === 'attributes') {
          if (r.target instanceof HTMLElement) applyAttr(r.target);
          continue;
        }
        for (var j = 0; j < r.addedNodes.length; j++) scanTree(r.addedNodes[j]);
      }
    });
    observer.observe(document.documentElement, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['class', 'style'],
    });
  }

  function deactivate() {
    if (!activated) return;
    activated = false;
    if (observer) { observer.disconnect(); observer = null; }
    delete document.documentElement.dataset.platform;
    var marked = document.querySelectorAll('[' + DRAG_ATTR + ']');
    for (var i = 0; i < marked.length; i++) marked[i].removeAttribute(DRAG_ATTR);
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

  // ---- Windows 标题栏（对标 apps/desktop/src/preload-windows.ts:12 + windows-layout.ts:4）----
  var WINDOWS_TITLEBAR_HEIGHT = 40;

  function installWindowsTitlebar() {
    var mark = function () {
      var root = document.documentElement;
      if (!root) return false;
      root.setAttribute('data-windows-titlebar', '');
      root.style.setProperty('--dsh-windows-titlebar-height', WINDOWS_TITLEBAR_HEIGHT + 'px');
      return true;
    };
    mark();
    // 自绘控件（官方由 Electron 原生 overlay 绘制；Tauri 无等价 API）——
    // 只在**主工作台页**挂载，且用 Shadow DOM 隔离样式（同胶囊的既有做法）。
    var mount = function () {
      if (document.getElementById('dsh-dock-window-controls')) return;
      if (!document.body) return;
      var host = document.createElement('div');
      host.id = 'dsh-dock-window-controls';
      host.style.cssText =
        'position:fixed;top:0;right:0;height:' + WINDOWS_TITLEBAR_HEIGHT +
        'px;display:flex;align-items:stretch;z-index:2147483000;';
      var shadow = host.attachShadow({ mode: 'open' });
      // **零裸色值**（paletteTokens 闸门 + AGENTS §4.3）：颜色一律从页面继承/派生 ——
      // `:host` 继承宿主元素的 color，按钮 `color:inherit`，hover 用 `color-mix` 从
      // 当前文字色派生。官方客户端让 Windows **原生**绘制这三个按钮（有系统红关闭态），
      // 自绘版改用主题派生色，正是为了不绕过本仓的 token 纪律。
      shadow.innerHTML =
        '<style>' +
        ':host{all:initial;color:inherit}' +
        'div{display:flex;height:100%;font-family:system-ui,sans-serif}' +
        'button{width:46px;height:100%;border:0;background:transparent;cursor:default;' +
        'color:inherit;display:grid;place-items:center;font-size:10px}' +
        'button:hover{background:color-mix(in srgb,currentColor 10%,transparent)}' +
        'button.dsh-close:hover{background:color-mix(in srgb,currentColor 22%,transparent)}' +
        '</style>' +
        '<div>' +
        '<button data-dsh-control="minimize" title="最小化">&#x2500;</button>' +
        '<button data-dsh-control="maximize" title="最大化">&#x25A1;</button>' +
        '<button data-dsh-control="close" class="dsh-close" title="关闭">&#x2715;</button>' +
        '</div>';
      document.body.appendChild(host);
      shadow.addEventListener('click', function (event) {
        var target = event.target;
        var action = target && target.getAttribute && target.getAttribute('data-dsh-control');
        if (!action) return;
        var win = window.__TAURI__ && window.__TAURI__.window;
        var current = win && win.getCurrentWindow ? win.getCurrentWindow() : null;
        if (!current) return;
        // 全部 .catch：关窗/最大化失败不得静默半途（AGENTS §4.3 的 Promise 纪律）。
        if (action === 'minimize') current.minimize().catch(function () {});
        else if (action === 'maximize') current.toggleMaximize().catch(function () {});
        else if (action === 'close') current.close().catch(function () {});
      });
    };
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', mark);
      document.addEventListener('DOMContentLoaded', mount);
    } else {
      mount();
    }
  }

})();
