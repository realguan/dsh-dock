// 交接幕布（ADR-0014）——主窗口的"跨文档连续加载"承接层（2026-09-10）。
//
// 为什么需要它：一次「重启 / 切换 profile」里，主窗口要换三次文档——
//     旧工作台(dsh) --navigate--> 壳启动屏(shell) --navigate--> 新工作台(dsh)
// 两处 navigate 前后各有一段"旧内容已消失、新内容还没画出来"的空档。壳启动屏
// 那一屏由 React 渲染；两端的工作台页面属于 dsh，壳源码不可改（AGENTS 红线 1），
// 于是由本脚本在 **document-start 先画一层与启动屏同构图的幕布**兜住这三段。
//
// 契约（壳侧协作方）：`boot.rs::show_handoff_curtain` 在 teardown **之前**注入
// `window.__dshDockCurtain.show(intent)`；React（BootIndex 挂载 / 工作台首帧）
// 调 `hide()` 撤销。自发现路径服务 navigate **之后**的新文档：本页若确为工作台
// origin（与 get_workbench_url 精确比对，同 switcher.js 判据）且壳侧交接仍在途
// （intent.active 且阶段 entering/ready），首帧即亮幕——不会先闪白再加载；
// 普通刷新（无交接）时 `active` 为假，脚本零动作。
//
// 文案/配色与壳启动屏同款：同一句「正在重启「X」」、同一条脉冲条、同一个自
// startedAt 起的计时（跨文档读数接着数，不归零）。中英两份极短表是必要重复：
// 注入脚本无打包器，够不着 content/*.ts（仅 6 条短句，改文案时两处同步）。
(function () {
  if (window.__dshDockCurtain) return;

  // 官方鲸标（assets/dsh-logo.svg 单一路径原样内联；外链图片在 dsh origin
  // 取不到壳资产，而这里只有 3.4KB，不需要任何网络请求）。
  var WHALE = 'M48.8354 10.0479C48.3232 9.79199 48.1025 10.2798 47.8032 10.5278C47.7007 10.6079 47.6143 10.7119 47.5273 10.8076C46.7793 11.624 45.9048 12.1597 44.7622 12.0957C43.0923 12 41.666 12.5356 40.4058 13.8398C40.1377 12.2319 39.2476 11.272 37.8926 10.6558C37.1836 10.3359 36.4668 10.0156 35.9702 9.31982C35.6235 8.82373 35.5293 8.27197 35.356 7.72754C35.2456 7.3999 35.1353 7.06396 34.7651 7.00781C34.3633 6.94385 34.2056 7.2876 34.0479 7.57568C33.418 8.75195 33.1733 10.0479 33.1973 11.3599C33.2524 14.312 34.4736 16.6641 36.8999 18.3359C37.1758 18.5278 37.2466 18.7197 37.1597 19C36.9946 19.5757 36.7974 20.1357 36.624 20.7119C36.5137 21.0801 36.3486 21.1597 35.9624 21C34.6309 20.4321 33.481 19.5918 32.4644 18.5757C30.7393 16.8721 29.1792 14.9917 27.2334 13.52C26.7764 13.1758 26.3193 12.856 25.8467 12.5518C23.8618 10.584 26.1069 8.96777 26.627 8.77588C27.1704 8.57568 26.8159 7.8877 25.0591 7.896C23.3022 7.90381 21.6953 8.50391 19.647 9.30371C19.3477 9.42383 19.0322 9.51172 18.7095 9.58398C16.8501 9.22363 14.9199 9.14355 12.9033 9.37598C9.10596 9.80762 6.07275 11.6396 3.84326 14.7681C1.16455 18.5278 0.53418 22.7998 1.30664 27.2559C2.11768 31.9521 4.46582 35.8398 8.07373 38.8799C11.8159 42.0322 16.1255 43.5762 21.041 43.2803C24.0269 43.104 27.3516 42.6963 31.1016 39.4561C32.0469 39.936 33.0396 40.1279 34.686 40.272C35.9546 40.3921 37.1758 40.208 38.1211 40.0078C39.6021 39.688 39.4995 38.2881 38.9639 38.0322C34.623 35.9678 35.5762 36.8081 34.71 36.1279C36.9155 33.4639 40.2402 30.6958 41.54 21.728C41.6426 21.0161 41.5557 20.5679 41.54 19.9917C41.5322 19.6396 41.6108 19.5039 42.0049 19.4639C43.0923 19.3359 44.1479 19.0317 45.1167 18.4878C47.9292 16.9199 49.064 14.3438 49.3315 11.2559C49.3711 10.7837 49.3237 10.2959 48.8354 10.0479ZM24.3262 37.8398C20.1196 34.4639 18.0791 33.3521 17.2358 33.3999C16.4482 33.4482 16.5898 34.3682 16.7632 34.9678C16.9443 35.5601 17.1812 35.9683 17.5117 36.4878C17.7402 36.832 17.8979 37.3442 17.2832 37.728C15.9282 38.584 13.5728 37.4399 13.4624 37.3838C10.7207 35.7358 8.42822 33.5601 6.81348 30.584C5.25342 27.7197 4.34766 24.6479 4.19775 21.3677C4.1582 20.5757 4.38672 20.2959 5.15869 20.1519C6.17529 19.96 7.22314 19.9199 8.23926 20.0718C12.5327 20.7119 16.1885 22.6719 19.2529 25.7759C21.002 27.5439 22.3252 29.6558 23.6885 31.7202C25.1377 33.9121 26.6978 36 28.6831 37.7119C29.3843 38.312 29.9434 38.7681 30.479 39.104C28.8643 39.2881 26.1699 39.3281 24.3262 37.8398ZM26.3433 24.6001C26.3433 24.248 26.6191 23.9678 26.9658 23.9678C27.0444 23.9678 27.1152 23.9839 27.1782 24.0078C27.2651 24.04 27.3438 24.0879 27.4067 24.1602C27.5171 24.272 27.5801 24.4321 27.5801 24.6001C27.5801 24.9521 27.3042 25.2319 26.9575 25.2319C26.6108 25.2319 26.3433 24.9521 26.3433 24.6001ZM32.6064 27.8799C32.2046 28.0479 31.8027 28.1919 31.4165 28.208C30.8179 28.2397 30.1641 27.9922 29.8096 27.688C29.2583 27.2158 28.8643 26.9521 28.6987 26.1279C28.6279 25.7759 28.6675 25.2319 28.7305 24.9199C28.8721 24.248 28.7144 23.8159 28.2495 23.4238C27.8716 23.104 27.3911 23.0161 26.8633 23.0161C26.666 23.0161 26.4849 22.9277 26.3511 22.856C26.1304 22.7441 25.9492 22.4639 26.1226 22.1201C26.1777 22.0078 26.4458 21.7358 26.5088 21.688C27.2256 21.272 28.0527 21.4077 28.8169 21.7197C29.5259 22.0161 30.0615 22.5601 30.834 23.3281C31.6216 24.2559 31.7632 24.5117 32.2124 25.208C32.5669 25.752 32.8901 26.312 33.1104 26.9521C33.2446 27.3521 33.0713 27.6802 32.6064 27.8799Z';

  var ZH = {
    start: function (n) { return '正在启动「' + n + '」'; },
    restart: function (n) { return '正在重启「' + n + '」'; },
    switch: function (n) { return '正在切换到「' + n + '」'; },
    stopping: '正在停止当前会话…',
    booting: '正在准备并启动新会话…',
    waiting: '服务已启动，正在等待就绪…',
    entering: '已就绪，正在打开工作台界面…',
    ready: '工作台已就绪',
    failed: '启动中断——详情见主窗口错误卡',
    elapsed: '本次操作已用时',
  };
  var EN = {
    start: function (n) { return 'Starting "' + n + '"'; },
    restart: function (n) { return 'Restarting "' + n + '"'; },
    switch: function (n) { return 'Switching to "' + n + '"'; },
    stopping: 'Stopping the current session…',
    booting: 'Preparing and starting the new session…',
    waiting: 'Service started, waiting until it is ready…',
    entering: 'Ready \u2014 opening the workbench…',
    ready: 'Workbench is ready',
    failed: 'Launch interrupted \u2014 see the error card in the main window',
    elapsed: 'Elapsed',
  };

  var ROOT_ID = 'dsh-dock-curtain';
  var SELF_HIDE_MS = 8000;    // 新文档路径：首屏画出来即撤；这是硬上限
  var STICKY_HIDE_MS = 12000; // 旧文档路径（Rust 注入）：撑到 navigate 把本页换掉
  var PAINT_WAIT_MAX_MS = 3000; // 等内容画出来的最长等待（超过即视为已画/画不出）
  var timerId = null;
  var capId = null;
  var cancelled = false;

  function dict() {
    return (navigator.language || 'zh').toLowerCase().indexOf('zh') === 0 ? ZH : EN;
  }

  function css() {
    return [
      '#' + ROOT_ID + '{position:fixed;inset:0;z-index:2147483000;display:flex;align-items:center;justify-content:center;',
      'background:#f7f8fb;color:#191d27;opacity:1;transition:opacity .22s ease;',
      'font-family:-apple-system,BlinkMacSystemFont,"PingFang SC","Hiragino Sans GB","Microsoft YaHei",sans-serif}',
      '#' + ROOT_ID + '.dsh-curtain-out{opacity:0}',
      '#' + ROOT_ID + ' .dsh-curtain-glow{position:absolute;left:0;right:0;top:0;height:24rem;pointer-events:none;',
      'background:radial-gradient(ellipse 80% 60% at 50% -20%,rgba(65,118,230,.12),transparent 70%)}',
      '#' + ROOT_ID + ' .dsh-curtain-box{position:relative;display:flex;flex-direction:column;align-items:center;gap:14px;padding:0 24px;text-align:center}',
      '#' + ROOT_ID + ' .dsh-curtain-mark{width:56px;height:56px;display:flex;align-items:center;justify-content:center;border-radius:16px;',
      'background:#fff;box-shadow:0 1px 2px rgba(16,24,40,.06),0 8px 24px -12px rgba(65,118,230,.35)}',
      '#' + ROOT_ID + ' .dsh-curtain-mark svg{width:34px;height:34px;display:block}',
      '#' + ROOT_ID + ' .dsh-curtain-mark path{fill:#4176e6}',
      '#' + ROOT_ID + ' .dsh-curtain-title{font-size:15px;font-weight:600;letter-spacing:-.01em}',
      '#' + ROOT_ID + ' .dsh-curtain-sub{font-size:12px;color:#626a7a;min-height:1.2em}',
      '#' + ROOT_ID + ' .dsh-curtain-bar{position:relative;width:220px;height:6px;border-radius:999px;border:1px solid #e5e9f1;background:#eef1f6;overflow:hidden}',
      '#' + ROOT_ID + ' .dsh-curtain-fill{position:absolute;top:0;bottom:0;width:38%;border-radius:3px;',
      'background:linear-gradient(90deg,transparent,#4176e6,#7d9cf2,transparent);animation:dsh-curtain-slide 1.3s cubic-bezier(.45,0,.55,1) infinite}',
      '#' + ROOT_ID + ' .dsh-curtain-timer{font-family:ui-monospace,"SF Mono",Menlo,Consolas,monospace;font-size:11px;color:#6b7280;font-variant-numeric:tabular-nums}',
      '@keyframes dsh-curtain-slide{0%{left:-40%}100%{left:100%}}',
      '@media (prefers-reduced-motion: reduce){#' + ROOT_ID + ' .dsh-curtain-fill{animation:none;left:30%}}',
      '@media (prefers-color-scheme: dark){',
      '#' + ROOT_ID + '{background:#14161c;color:#e8eaf0}',
      '#' + ROOT_ID + ' .dsh-curtain-mark{background:#1d2029;box-shadow:0 8px 24px -12px rgba(0,0,0,.6)}',
      '#' + ROOT_ID + ' .dsh-curtain-mark path{fill:#e8eaf0}',
      '#' + ROOT_ID + ' .dsh-curtain-sub{color:#9aa3b2}',
      '#' + ROOT_ID + ' .dsh-curtain-bar{border-color:#2a2e3a;background:#1d2029}',
      '#' + ROOT_ID + ' .dsh-curtain-timer{color:#9aa3b2}}',
    ].join('');
  }

  function fmtElapsed(ms) {
    var safe = Math.max(0, ms);
    if (safe < 10000) return (safe / 1000).toFixed(1) + 's';
    var total = Math.floor(safe / 1000);
    return Math.floor(total / 60) + ':' + String(total % 60).padStart(2, '0');
  }

  // body 尚不存在（document-start）：与 memory-policy.js 同法——观察 documentElement，
  // body 一出现就挂。绝不往 <html> 直接塞节点（解析中会干扰 HTML 解析器的插入模式）。
  function mount(el) {
    if (document.body) {
      document.body.appendChild(el);
      return;
    }
    var mo = new MutationObserver(function () {
      if (!document.body) return;
      mo.disconnect();
      if (cancelled) return; // 挂载前就被 hide 取消：不再补挂（观察器已断开）
      document.body.appendChild(el);
    });
    mo.observe(document.documentElement, { childList: true });
  }

  function remove() {
    var root = document.getElementById(ROOT_ID);
    if (!root) return;
    root.className = 'dsh-curtain-out';
    setTimeout(function () {
      if (root.parentNode) root.parentNode.removeChild(root);
    }, 240);
  }

  function clearTimers() {
    if (timerId) { clearInterval(timerId); timerId = null; }
    if (capId) { clearTimeout(capId); capId = null; }
  }

  function hide() {
    cancelled = true;
    clearTimers();
    remove();
  }

  // 「页面画出来了」判据：body 有实质节点且已有可见文本/画布类内容。
  // 为什么不用 load 事件：SPA 的 bundle 加载完 ≠ 首屏渲染完（异步取数/水合
  // 还在后面），load 就撤会把空白页露出来——那正是幕布要盖住的那一段。
  function contentReady() {
    var body = document.body;
    if (!body || body.children.length === 0) return false;
    if ((body.innerText || '').trim().length > 0) return true;
    return body.querySelector('canvas, svg, img, iframe, [role]') !== null;
  }

  // 收幕调度：`onPaint=true`（新文档路径）= 等内容画出来再撤（画出来那一帧
  // 之后再等 120ms，避开"内容刚出现就被幕布闪一下"）；`onPaint=false`
  // （sticky，盖在**即将被 navigate 换掉的旧页面**上）= 只认硬上限，否则本页
  // load 早已完成，幕布会在 teardown 的那几秒里提前撤掉、把死掉的页面露出来。
  function scheduleHide(capMs, onPaint) {
    if (onPaint) {
      var deadline = Date.now() + PAINT_WAIT_MAX_MS;
      var poll = function () {
        if (!document.getElementById(ROOT_ID)) return; // 已被 hide 收走
        if (contentReady() || Date.now() >= deadline) {
          requestAnimationFrame(function () { setTimeout(hide, 120); });
          return;
        }
        setTimeout(poll, 120);
      };
      if (document.readyState === 'complete') poll();
      else window.addEventListener('load', poll, { once: true });
    }
    capId = setTimeout(hide, capMs);
  }

  // 把意图写进已有幕布（含计时重启）：同一文档里连续两次交接（用户在导轨还在
  // 走时又点了另一个 profile）复用同一层幕，但文案/计时必须跟着新意图走，
  // 不能留着上一个目标的名字对用户说谎。
  function applyIntent(root, intent) {
    var d = dict();
    root.querySelector('.dsh-curtain-title').textContent = (d[intent.kind] || d.start)(intent.target);
    root.querySelector('.dsh-curtain-sub').textContent = d[intent.phase] || d.waiting;
    var timer = root.querySelector('.dsh-curtain-timer');
    timer.title = d.elapsed;
    var startedAt = typeof intent.startedAt === 'number' ? intent.startedAt : Date.now();
    if (timerId) { clearInterval(timerId); timerId = null; }
    var tick = function () { timer.textContent = fmtElapsed(Date.now() - startedAt); };
    tick();
    timerId = setInterval(tick, 200);
  }

  function show(intent, opts) {
    if (!intent || !intent.target) return;
    // 新一次交接：撤销上一次 hide 留下的取消标记（否则本页再也亮不起幕布）。
    cancelled = false;
    var existing = document.getElementById(ROOT_ID);
    if (existing) {
      clearTimers();
      applyIntent(existing, intent);
      var stickyRepeat = !!(opts && opts.sticky);
      scheduleHide(stickyRepeat ? STICKY_HIDE_MS : SELF_HIDE_MS, !stickyRepeat);
      return;
    }
    var root = document.createElement('div');
    root.id = ROOT_ID;
    root.setAttribute('role', 'status');
    root.setAttribute('aria-live', 'polite');

    var style = document.createElement('style');
    style.textContent = css();
    root.appendChild(style);

    var glow = document.createElement('div');
    glow.className = 'dsh-curtain-glow';
    var box = document.createElement('div');
    box.className = 'dsh-curtain-box';
    var mark = document.createElement('div');
    mark.className = 'dsh-curtain-mark';
    // 内联 SVG 用 DOM 构造（WHALE 是常量路径，仍不拼 HTML 字符串）
    var svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.setAttribute('viewBox', '0 0 50 50');
    var p = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    p.setAttribute('d', WHALE);
    svg.appendChild(p);
    mark.appendChild(svg);
    var title = document.createElement('div');
    title.className = 'dsh-curtain-title';
    var sub = document.createElement('div');
    sub.className = 'dsh-curtain-sub';
    var bar = document.createElement('div');
    bar.className = 'dsh-curtain-bar';
    var fill = document.createElement('div');
    fill.className = 'dsh-curtain-fill';
    bar.appendChild(fill);
    var timer = document.createElement('div');
    timer.className = 'dsh-curtain-timer';

    box.appendChild(mark);
    box.appendChild(title);
    box.appendChild(sub);
    box.appendChild(bar);
    box.appendChild(timer);
    root.appendChild(glow);
    root.appendChild(box);
    // 文案一律走 textContent（profile 名是用户输入，绝不进 innerHTML）
    applyIntent(root, intent);
    mount(root);

    // sticky（Rust 在旧工作台页注入）：本页马上会被 navigate 换掉，不按 load 撤，
    // 只用硬上限兜底（teardown grace 3s + 导航，12s 足够且不会永久盖住页面）。
    var sticky = !!(opts && opts.sticky);
    scheduleHide(sticky ? STICKY_HIDE_MS : SELF_HIDE_MS, !sticky);
  }

  function invoke(cmd) {
    var core = window.__TAURI__ && window.__TAURI__.core;
    if (!core || typeof core.invoke !== 'function') return Promise.reject(new Error('no ipc'));
    return core.invoke(cmd);
  }

  // 自发现：只对**工作台 origin** 生效——壳页面由 React 自己画，幕布不去抢
  // （否则系统深色下会在浅色壳页上闪一帧深色幕）。
  function selfDiscover() {
    Promise.all([invoke('get_workbench_url'), invoke('get_boot_status')])
      .then(function (res) {
        var wb = res[0];
        var intent = res[1] && res[1].intent;
        if (!wb || !intent || !intent.active) return;
        var origin = null;
        try { origin = new URL(wb).origin; } catch { return; }
        if (location.origin !== origin) return;
        if (intent.phase !== 'entering' && intent.phase !== 'ready') return;
        show(intent);
      })
      .catch(function () { /* IPC 不可用（浏览器预览）：静默，不阻断页面 */ });
  }

  window.__dshDockCurtain = { show: show, hide: hide };
  selfDiscover();
})();
