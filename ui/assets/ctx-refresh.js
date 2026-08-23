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
// 沉浸式标题栏衬垫：**常驻固定条**，永不随滚动消失。
// 早期实现是 html 内边距——滚动时衬垫跟着内容滚走（观感回退，2026-08-23 修正）。
// 实测交通灯悬浮区约 y0..22、品牌胶囊 y24 起，故遮罩取 24px 与胶囊上缘平齐：
// 滚动后内容最多露到 y24，交通灯下方永远是浅色呼吸带（2026-08-23 视觉验证）。
// 内容本体仍由 PAD 保持静止基线间距；本条在下层提供持久遮罩。
(() => {
  const PAD = 10;
  const MASK_H = 24;
  document.documentElement.style.paddingTop = PAD + "px";
  const band = document.createElement("div");
  band.style.cssText =
    "position:fixed;top:0;left:0;right:0;height:" + MASK_H + "px;" +
    "background:rgba(249,250,251,0.98);" + // 与浅色主题 --bg 一致
    "z-index:2147483645;pointer-events:none;";
  document.documentElement.appendChild(band);
})();

// 沉浸式标题栏拖拽热区：全宽 y0..20 透明条（实测 y0..22 全视图无交互元素），
// 承担窗口拖动；data-tauri-drag-region 为 Tauri 原生拖拽协议。
(() => {
  const strip = document.createElement("div");
  strip.setAttribute("data-tauri-drag-region", "");
  strip.style.cssText =
    "position:fixed;top:0;left:0;right:0;height:20px;z-index:2147483646;" +
    "-webkit-user-select:none;user-select:none;cursor:default;";
  strip.addEventListener("contextmenu", (e) => e.preventDefault());
  document.documentElement.appendChild(strip);
})();
