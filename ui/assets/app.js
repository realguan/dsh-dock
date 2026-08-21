// 产品壳自带前端 = 启动加载页（无构建器、无框架）。
// 壳是通用机制：dsh 就绪后 Rust 侧会把本窗口 WebView 导航到 127.0.0.1 的
// dsh Web UI，这个页面只在「等 dsh 起来」的窗口期可见，出错时被错误页替换。
// 因此这里不做任何业务逻辑，只负责在无法定位 dsh 时把状态展示出来。
// （v0 错误路径由 Rust 直接导航到自包含错误页，无需与本页面通信。）
document.addEventListener("DOMContentLoaded", () => {
  const status = document.getElementById("status");
  if (status) {
    status.textContent = "正在启动装配好的工作台…";
  }
});
