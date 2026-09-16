# ADR-0002：WebView 长会话内存策略（content-visibility 注入）

- **日期**：2026-08-26
- **状态**：已废弃（2026-09-15 撤销；原因：隐式 Paint Containment 导致卡片左右边缘裁切，且 content-visibility 触发 WebKit 触控板滑动掉帧与排版抖动）
- **提出人**：guan
- **相关方**：lib.rs（initialization_script `webview_memory_policy`）
- **关联**：上游 ADR-0004（内嵌 WebView）

---

## 1. 背景与问题

dsh web 前端全量渲染会话（无虚拟化），WebKit 对视口外渲染资源的回收弱于 Chromium。长会话下 WebContent 实测膨胀 2.7~4.3 GB，最终拖垮壳。需要一个壳侧、零依赖 dsh 前端配合的缓解手段。

## 2. 约束与硬指标

- 壳不修改 dsh 前端代码（跨仓库边界）。
- 不引入前端构建链 / 依赖（§4.3 红线）。
- 不破坏既有用户本地状态（localStorage / IndexedDB 等）。
- 老 WebKitGTK（Ubuntu 22.04 ≈ 2.38）必须优雅降级，不能注入后白屏。
- 流式输出中的行不能被裁剪（tooltip / popover / 行内浮层不能断）。

## 3. 备选方案及评估

### 方案 A：CSS `content-visibility: auto` 注入 + 动态豁免 + 能力探测降级 —— ✅ 最终采纳

- 思路：经 initialization_script 注入 CSS，对会话行打 `content-visibility: auto` + `contain-intrinsic-size`；流式中的行经 MutationObserver 动态加豁免类；不支持该特性的引擎整段退出。
- 优点：壳侧独立可行，不动前端；流式行不裁剪；老引擎零副作用。
- 代价/风险：依赖 dsh 前端 DOM 属性（`data-chat-flow` / `data-chat-anchor-key` / `data-streaming`）——上游改名即失效（但失效是降级为「不优化」，非崩溃）。
- 对照约束：逐条满足（不改前端 / 无构建链 / 不动本地状态 / 优雅降级 / 不裁剪浮层）。

### 方案 B：`translateZ(0)` / `will-change` 合成层 —— ❌ 否决

- 思路：对行打合成层提升渲染效率。
- 否决理由：违反「不破坏状态」之外的更硬约束——全列表 `will-change` 会反致内存爆炸（合成层常驻显存），与目标背道而驰。

### 方案 C：`data_store_identifier` 隔离会话存储 —— ❌ 否决

- 思路：给 WebView 设独立数据存储标识，定期隔离/清理。
- 否决理由：会让现有用户的本地状态「丢失」（违反硬指标），且不解决渲染资源膨胀本身。

## 4. 最终决策

经 initialization_script（`webview_memory_policy`）注入 CSS：会话行 `content-visibility: auto` + `contain-intrinsic-size: auto 64px`；流式中行加 `dsh-cv-skip` 豁免类（MutationObserver 监听 `data-streaming` 增删动态维护）；`CSS.supports` 能力探测，不支持则整段 return（优雅降级）；document-start 时 `document.body` 为 null，观察 `documentElement` + `DOMContentLoaded` 兜底。不显式加 `contain: paint`。HOW 见 `lib.rs`。

- **2026-08-31 修订（列表裁切与选择器直接子代修复）**：
  1. `content-visibility: auto` 规范隐式赋予元素 Paint Containment，导致有序/无序列表（`list-style-position: outside`）的序号/圆点挂在行左侧外沿时被行边界裁切截断（实测 `1. 2. 3.` 仅露出一半）。同步在注入 CSS 中追加 `FLOW ol, FLOW ul { padding-left: 1.5em !important; }`，补足内边距防止 markers 溢出被裁剪。
  2. 原后代选择器 `FLOW + ' ' + ROW` 会渗透匹配到消息内部挂有 `data-chat-anchor-key="call:<id>"` 的每个工具调用（`ToolCallTree` 的 `callRow` / `subCalls`），在几十步工具调用的长轮次中产生多层嵌套 containment，导致 WebKit 估算排版高度严重虚高数千像素，在实际渲染收缩后遗留巨大底部滚动空白并导致 `composerSeat` 悬空。规则修改为直接子代选择器 `FLOW + ' > ' + ROW`，约束 containment 仅作用于顶层消息卡片。

- **2026-09-15 废弃与撤销（左右两侧显示不全与触控板滑动卡顿）**：
  实测表明 `content-visibility: auto` 存在难以调和的体验缺陷：
  1. **左右两侧显示不全**：CSS Containment Level 2 规范规定 `content-visibility: auto` 会强制施加 `contain: paint`（哪怕元素在视口内）。这导致消息卡片左右外露的 Action 按钮、阴影、列表序号等墨水溢出（ink overflow）被硬生生切断；
  2. **滑动不丝滑**：`contain-intrinsic-size: auto 64px` 的固定高度与成百上千像素的复杂内容（长文本/代码块/工具卡片）存在巨大落差。在 macOS 触控板快速惯性滑动时，大量元素连续在 skipped 与 visible 之间切换，导致 WebKit 主线程高频触发 Layout Thrashing（重排重构），并与合成器线程的惯性滚动严重脱节，产生显著的掉帧、卡顿与视口跳动；
  3. **裁决**：在现代硬件统一内存普遍充裕的情况下，牺牲核心视觉完整性与丝滑滚动体验得不偿失。彻底移除壳侧该注入脚本与兜底 CSS；长会话若有更深度的内存诉求，应由上游通过标准 DOM 虚拟列表（Virtual List）从根本上解决。

## 5. 后果与后续行动项

### 正面后果
- 长会话 WebContent 膨胀显著缓解，壳侧独立、不动前端。

### 负面后果 / 新增债务
- 依赖 dsh 前端 DOM 属性名——上游改名需同步改注入脚本。
- `dsh-cv-skip` 与 MutationObserver 是运行时维护，一次性扫描会漏。

### 行动项
- [x] `webview_memory_policy` 脚本实现（lib.rs）。

## 6. 复审条件

- dsh 前端会话 DOM 属性改名 → 同步更新注入脚本选择器。
- WebKit 视口外渲染资源回收改进（或前端引入虚拟化）→ 复评是否仍需注入。
- Tauri 提供 WebView 内存/虚拟化原生支持 → 复评替换。
