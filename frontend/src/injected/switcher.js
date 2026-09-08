(function () {
      if (window.__dshDockSwitcherInjected) return;
      window.__dshDockSwitcherInjected = true;

      var currentSettings = null;
      var workbenchOrigin = null;

      function isMacPlatform() {
        return /Mac|iPod|iPhone|iPad/.test(navigator.platform || navigator.userAgent);
      }

      window.addEventListener('keydown', function (e) {
        var isMac = isMacPlatform();
        var modKey = isMac ? e.metaKey : (e.ctrlKey || e.metaKey);
        var shortcutKey = currentSettings && currentSettings.switcherShortcut ? currentSettings.switcherShortcut : 'default';
        var matchDefault = modKey && !e.shiftKey && (e.key === ',' || e.code === 'Comma');
        var matchShiftP = modKey && e.shiftKey && (e.key === 'P' || e.key === 'p' || e.code === 'KeyP');

        var isMatch = false;
        if (shortcutKey === 'shift_p') {
          isMatch = matchShiftP || matchDefault;
        } else {
          isMatch = matchDefault || matchShiftP;
        }

        if (isMatch) {
          e.preventDefault();
          e.stopPropagation();
          var tauri = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
          if (tauri) {
            window.__TAURI__.core.invoke('open_profiles_window').catch(function () {});
          }
        }
      }, true);

      function getShortcutText(settings) {
        var isMac = isMacPlatform();
        var shortcutKey = settings && settings.switcherShortcut ? settings.switcherShortcut : 'default';
        return shortcutKey === 'shift_p' ? (isMac ? '⌘ + ⇧ + P' : 'Ctrl + ⇧ + P') : (isMac ? '⌘ + ,' : 'Ctrl + ,');
      }

      // 2026-09-08 裁定：壳页面在 macOS 是 tauri://localhost、Windows 是
      // http://tauri.localhost、dev 是 vite 端口——都不是工作台；只有当前
      // location.origin 与 get_workbench_url 一致（主窗口 navigate 落地后
      // 本脚本随文档重跑）才允许挂载。origin 未知/未就绪一律不挂。
      function onWorkbenchPage() {
        return workbenchOrigin !== null && location.origin === workbenchOrigin;
      }

      function fetchWorkbenchOrigin() {
        var tauri = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
        if (!tauri) {
          workbenchOrigin = null;
          return Promise.resolve();
        }
        return tauri('get_workbench_url').then(function (u) {
          try {
            workbenchOrigin = u ? new URL(u).origin : null;
          } catch {
            workbenchOrigin = null;
          }
        }).catch(function () {
          workbenchOrigin = null;
        });
      }

      function removeCapsule() {
        var root = document.getElementById('dsh-dock-switcher-root');
        if (root) root.remove();
      }

      function renderCapsule(settings) {
        if (document.getElementById('dsh-dock-switcher-root')) return;
        if (!onWorkbenchPage()) return;

        var host = document.createElement('div');
        host.id = 'dsh-dock-switcher-root';
        host.style.cssText = 'position:fixed;top:8px;left:50%;transform:translateX(-50%);z-index:999999;font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,sans-serif;pointer-events:auto;user-select:none;-webkit-user-select:none;touch-action:none;';

        var shadow = host.attachShadow({ mode: 'open' });
        var shortcutText = getShortcutText(settings);

        var wrap = document.createElement('div');
        wrap.innerHTML = '<style>' +
          '.capsule{' +
            'display:inline-flex;align-items:center;gap:5px;height:26px;padding:0 8px 0 7px;' +
            'border-radius:9999px;background:rgba(255,255,255,0.85);' +
            'backdrop-filter:blur(16px);-webkit-backdrop-filter:blur(16px);' +
            'border:1px solid rgba(0,0,0,0.08);' +
            'box-shadow:0 2px 8px rgba(0,0,0,0.04),0 1px 2px rgba(0,0,0,0.02);' +
            'color:#27272a;font-size:11px;font-weight:600;cursor:grab;' +
            'transition:all 0.2s cubic-bezier(0.16,1,0.3,1);outline:none;' +
          '}' +
          '.capsule.dragging{cursor:grabbing;opacity:0.9;}' +
          '@media(prefers-color-scheme:dark){' +
            '.capsule{background:rgba(24,24,27,0.85);border-color:rgba(255,255,255,0.12);box-shadow:0 2px 10px rgba(0,0,0,0.35);color:#f4f4f5;}' +
          '}' +
          '.capsule:hover{' +
            'background:rgba(255,255,255,0.98);border-color:rgba(59,130,246,0.45);' +
            'box-shadow:0 4px 14px rgba(59,130,246,0.18),0 2px 4px rgba(0,0,0,0.04);' +
          '}' +
          '@media(prefers-color-scheme:dark){' +
            '.capsule:hover{background:rgba(39,39,42,0.98);border-color:rgba(96,165,250,0.5);box-shadow:0 4px 16px rgba(96,165,250,0.25);}' +
          '}' +
          '.icon{display:flex;align-items:center;justify-content:center;width:13px;height:13px;color:#2563eb;}' +
          '@media(prefers-color-scheme:dark){.icon{color:#60a5fa;}}' +
          '.label{letter-spacing:-0.01em;line-height:1;}' +
          '.badge{' +
            'display:none;align-items:center;justify-content:center;' +
            'padding:1px 4px;margin-left:2px;border-radius:4px;' +
            'background:rgba(0,0,0,0.06);color:#71717a;' +
            'font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,monospace;' +
            'font-size:9px;font-weight:500;line-height:1;white-space:nowrap;flex-shrink:0;' +
          '}' +
          '.capsule:hover .badge{display:inline-flex;}' +
          '@media(prefers-color-scheme:dark){' +
            '.badge{background:rgba(255,255,255,0.08);color:#a1a1aa;}' +
          '}' +
        '</style>' +
        '<div class="capsule" id="btn" title="控制中心 (' + shortcutText + ') · 可拖拽移动">' +
          '<span class="icon">' +
            '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">' +
              '<line x1="4" y1="21" x2="4" y2="14"></line><line x1="4" y1="10" x2="4" y2="3"></line>' +
              '<line x1="12" y1="21" x2="12" y2="12"></line><line x1="12" y1="8" x2="12" y2="3"></line>' +
              '<line x1="20" y1="21" x2="20" y2="16"></line><line x1="20" y1="12" x2="20" y2="3"></line>' +
              '<line x1="1" y1="14" x2="7" y2="14"></line><line x1="9" y1="8" x2="15" y2="8"></line><line x1="17" y1="16" x2="23" y2="16"></line>' +
            '</svg>' +
          '</span>' +
          '<span class="label">控制中心</span>' +
          '<span class="badge" id="badge">' + shortcutText + '</span>' +
        '</div>';

        shadow.appendChild(wrap);
        var btn = shadow.getElementById('btn');
        if (btn) {
          var isDragging = false;
          var startX = 0, startY = 0;
          var initialLeft = 0, initialTop = 0;
          var hasMoved = false;

          btn.addEventListener('pointerdown', function (e) {
            isDragging = true;
            hasMoved = false;
            startX = e.clientX;
            startY = e.clientY;
            var rect = host.getBoundingClientRect();
            initialLeft = rect.left;
            initialTop = rect.top;
            btn.classList.add('dragging');
            btn.setPointerCapture(e.pointerId);
          });

          btn.addEventListener('pointermove', function (e) {
            if (!isDragging) return;
            var dx = e.clientX - startX;
            var dy = e.clientY - startY;
            if (Math.abs(dx) > 3 || Math.abs(dy) > 3) {
              hasMoved = true;
            }
            host.style.transform = 'none';
            host.style.left = Math.max(8, Math.min(window.innerWidth - 110, initialLeft + dx)) + 'px';
            host.style.top = Math.max(8, Math.min(window.innerHeight - 36, initialTop + dy)) + 'px';
          });

          btn.addEventListener('pointerup', function (e) {
            if (!isDragging) return;
            isDragging = false;
            btn.classList.remove('dragging');
            btn.releasePointerCapture(e.pointerId);
            if (!hasMoved) {
              var tauri = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
              if (tauri) {
                window.__TAURI__.core.invoke('open_profiles_window').catch(function (err) {
                  console.error('[dsh-dock] 打开控制中心失败:', err);
                });
              }
            }
          });
        }
        document.body.appendChild(host);
      }

      function updateCapsuleBadge(settings) {
        var host = document.getElementById('dsh-dock-switcher-root');
        if (!host || !host.shadowRoot) return;
        var badge = host.shadowRoot.getElementById('badge');
        var btn = host.shadowRoot.getElementById('btn');
        var shortcutText = getShortcutText(settings);
        if (badge) badge.textContent = shortcutText;
        if (btn) btn.setAttribute('title', '控制中心 (' + shortcutText + ') · 可拖拽移动');
      }

      function applySettings(settings) {
        currentSettings = settings;
        if (settings && settings.showFloatingSwitcher === false) {
          removeCapsule();
        } else {
          if (!document.getElementById('dsh-dock-switcher-root')) {
            renderCapsule(settings);
          } else {
            updateCapsuleBadge(settings);
          }
        }
      }

      function initSwitcher() {
        var tauri = window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.core.invoke;
        if (tauri) {
          window.__TAURI__.core.invoke('get_shell_settings').then(function (settings) {
            applySettings(settings);
          }).catch(function () {
            applySettings(null);
          });
          // origin 就绪后补一次挂载判定（与 settings 并发到达，晚到者触发挂载）
          fetchWorkbenchOrigin().then(function () {
            applySettings(currentSettings);
          });
        } else {
          applySettings(null);
        }

        // 监听来自控制中心保存的实时广播事件
        if (window.__TAURI__ && window.__TAURI__.event && window.__TAURI__.event.listen) {
          window.__TAURI__.event.listen('app:settings-changed', function (ev) {
            if (ev && ev.payload) {
              applySettings(ev.payload);
            }
          });
        }
      }

      if (document.body) {
        initSwitcher();
      } else {
        document.addEventListener('DOMContentLoaded', initSwitcher);
      }
    })();
