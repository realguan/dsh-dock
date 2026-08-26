# ADR-0003：外链与导航策略（系统浏览器兜底 + 白名单拦截）

- **日期**：2026-08-26
- **状态**：已接受（2026-08-25 裁定，本 ADR 追溯补录）
- **提出人**：guan
- **相关方**：lib.rs（`create_main_window` / `on_navigation` / `on_new_window` / `hook_script` / `EXTERNAL_URL_HOSTS`）
- **关联**：上游 ADR-0004（内嵌 WebView）

---

## 1. 背景与问题

主窗口加载回环 dsh Web UI。dsh 页面的超链接 / 新窗口在 WebView 里默认「点不动」或留在 WebView 内，体验割裂；同时壳不能成为任意跳板——用户点到的任何 URL 都不能借壳的 WebView 任意导航。需要统一外链出口 + 拦截非白名单跳转。

## 2. 约束与硬指标

- 主窗口必须代码创建（`create_main_window`），tauri.conf.json 不静态定义 windows——只有代码创建才能挂 `on_navigation` / `on_new_window` 处理器。
- 壳页与回环 dsh 放行；其余 http(s) 过白名单后转系统浏览器并拦截导航；非白名单直接拦。
- `on_new_window` 一律 Deny，白名单内转浏览器。
- 跨源 `<a>` 点击兜底走 `open_external`（同一白名单）。
- 新外链域必须在 `EXTERNAL_URL_HOSTS` 登记（可审计、可收口）。

## 3. 备选方案及评估

### 方案 A：导航/新窗口双拦截 + 白名单 + 兜底 hook —— ✅ 最终采纳

- 思路：`on_navigation` 分流壳页 / 回环 / 白名单 / 非白名单；`on_new_window` 一律 Deny + 白名单转浏览器；initialization_script 兜底捕获 `<a>` 点击走 IPC `open_external`。
- 优点：三层覆盖（导航、新窗口、点击事件）；壳不成为跳板；白名单可收口。
- 代价/风险：三层逻辑需保持一致；白名单需随业务登记维护。
- 对照约束：逐条满足。

### 方案 B：仅 `on_navigation` 拦截，不处理新窗口/点击 —— ❌ 否决

- 思路：只在导航回调拦截。
- 否决理由：`target=_blank` 新窗口与 JS 触发的跨源点击不走 `on_navigation`，会漏；体验割裂未解决。

### 方案 C：放行所有 http(s) 在 WebView 内导航 —— ❌ 否决

- 思路：不拦截，任意外链在 WebView 内打开。
- 否决理由：壳成为任意跳板，违反安全边界硬指标。

## 4. 最终决策

导航/新窗口双拦截 + `EXTERNAL_URL_HOSTS` 白名单 + 兜底 hook 三层：壳页（tauri/about/data/blob + `tauri.localhost`）与回环 dsh（127.0.0.1/localhost/[::1]）放行；其余 http(s) 过白名单后转系统浏览器（open crate）并拦截 WebView 导航，非白名单直接拦；`on_new_window` 一律 Deny，白名单内转浏览器；initialization_script 兜底捕获跨源 `<a>` 点击走 `open_external`。新外链域 = 在 `EXTERNAL_URL_HOSTS` 登记。HOW 见 `lib.rs`。

## 5. 后果与后续行动项

### 正面后果
- 外链体验统一（系统浏览器）；壳不成为跳板；白名单收口可审计。

### 负面后果 / 新增债务
- 白名单是硬编码常量，新增域需改代码 + 登记。
- 三层分流逻辑需保持语义一致，改动任一层要同步另外两层。

### 行动项
- [x] `on_navigation` / `on_new_window` / `hook_script` 实现（lib.rs）。
- [x] `is_allowed_external_url` 白名单校验含反例测试（lib.rs）。

## 6. 复审条件

- Tauri 导航 / 新窗口 API 变更 → 复评三层覆盖是否仍完整。
- 新增外链域 → 在 `EXTERNAL_URL_HOSTS` 登记（本 ADR 不需重开，登记即可）。
- dsh 前端改为自带外链处理 → 复评兜底 hook 是否仍需。
