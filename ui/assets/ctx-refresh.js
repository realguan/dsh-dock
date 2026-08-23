// 主窗口初始化脚本（每个文档加载时运行，含 dsh Web UI）。
// 空白处右击 → 原生菜单「刷新」（单一功能，用户裁定）；
// 选中文本 / 输入框 / 可编辑区 → 放行系统默认菜单（Look Up/Translate/搜索等 macOS 能力）。
// 依赖 withGlobalTauri 注入的 window.__TAURI__.menu；缺失时静默跳过。
(() => {
  if (!window.__TAURI__ || !window.__TAURI__.menu) return;

  document.addEventListener("contextmenu", (e) => {
    // 选中文本 → 保留系统富菜单
    const sel = (window.getSelection && String(window.getSelection())) || "";
    if (sel.trim().length > 0) return;
    // 输入类区域 → 保留系统菜单（编辑上下文）
    const t = e.target;
    if (
      t &&
      (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName || ""))
    ) {
      return;
    }
    e.preventDefault();
    window.__TAURI__.menu.MenuItem.new({ id: "refresh", text: "刷新" })
      .then((item) => {
        item.onClick(() => {
          location.reload();
        });
        return window.__TAURI__.menu.Menu.new({ items: [item] });
      })
      .then((menu) => menu.popup())
      .catch(() => {});
  });
})();