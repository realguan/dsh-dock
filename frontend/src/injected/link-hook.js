(function () {
      if (window.__dshDockLinkHooked) return;
      window.__dshDockLinkHooked = true;
      document.addEventListener('click', function (ev) {
        var a = ev.target && ev.target.closest ? ev.target.closest('a[href]') : null;
        if (!a) return;
        var href = a.getAttribute('href') || '';
        if (!/^https?:\/\//i.test(href)) return; // 相对/锚点交给页面自己
        try {
          var u = new URL(href, location.href);
          if (u.origin === location.origin) return; // 同源（回环 dsh 自身）放行
        } catch { return; }
        // 只有 __TAURI__ IPC 可用时才拦截（否则放行，让 WKWebView 原生
        // on_navigation/on_new_window 兜底——preventDefault 后 invoke 失败
        // 会变成"点了没反应"）。
        var tauri = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
        if (!tauri) return;
        ev.preventDefault();
        try {
          window.__TAURI__.core.invoke('open_external', { url: href })
            .catch(function (e) { console.error('[dsh-dock] 外链打开失败:', e); });
        } catch {}
      }, true);
    })();
