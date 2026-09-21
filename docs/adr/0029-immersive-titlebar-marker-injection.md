# ADR-0029：沉浸式标题栏——标记补打 + app-region 计算样式映射

- **日期**：2026-09-21
- **状态**：已接受
- **提出人**：维护者（截图对比触发）+ AI 实施
- **相关方**：`src-tauri/src/ui.rs`（主窗口）、`frontend/src/injected/`、`frontend/src/lib/immersiveChrome.ts`
- **关联**：ADR-0014（交接幕布，注入脚本先例）；dsh 上游 `apps/desktop`（Electron 官方客户端实证）

---

## 1. 背景与问题

官方 dsh 桌面客户端（Electron）是**单窗口沉浸式**形态：无独立原生标题栏，macOS 红绿灯浮在
内容左上角，应用自己的顶栏（会话标题 + 模式按钮 + 「对话/轨迹」tabs）占据标题栏带并兼任
拖拽区。dsh-dock 主窗口当前是原生装饰 + title "DSH Dock"，导航到 dsh 工作台后**整窗就是
dsh 页面**（`boot.rs` `state.window.navigate(workbench_url)`），与官方客户端的差距恰好只剩
标题栏这一圈原生 chrome。

dsh 源码实证（`/Users/guan/git/deepseek-harness`，2026-09-21 读）：

- **窗口层** `apps/desktop/src/main.ts:111-133`：
  - macOS：`titleBarStyle: 'hiddenInset'` + `trafficLightPosition: {x:16, y:18}` +
    `vibrancy: 'sidebar'` + `visualEffectState: 'active'` + 透明底色；
  - Windows：`titleBarStyle: 'hidden'` + `titleBarOverlay`（高 40px，`windows-layout.ts:4`
    的 `WINDOWS_TITLEBAR_HEIGHT`），标题栏颜色由页面 computed color 经 canvas 像素探针 +
    IPC 反推同步（`preload-windows.ts`）。
- **标记层**（两个 Electron preload）：`preload-platform.ts:8` 打
  `document.documentElement.dataset.platform = 'darwin'`；`preload-windows.ts:12` 打
  `data-windows-titlebar` + `--dsh-windows-titlebar-height: 40px`。
- **页面层**（同一套 web 前端，`packages/client`，全部按标记生效）：

  | 位置 | 规则 |
  |---|---|
  | `web/src/base.css:37` | `html[data-platform='darwin']` → 页面透明，透出 vibrancy |
  | `ui-layout/.../AppFrame.module.css:76-92` | darwin → 侧栏半透明 tint、中栏自绘不透明底 |
  | 同上 `:20-40` | `[data-windows-titlebar]` → `.frame` padding-top 40px + `::before` 拖拽条 |
  | `ui-conversation/.../ConversationRoot.module.css:35-46` | darwin → `.titleRow` drag，子按钮/a/headerLeading 等 no-drag |
  | `ui-sidebar/.../SidebarRoot.module.css:189-194` | `.topStrip` drag + `.iconButton` no-drag |
  | `ui-sidebar-right/.../SidebarRight.module.css:90-94` | 右栏同类 |

**关键发现**：这套沉浸式 CSS 本来就随 dsh 工作台一起被壳加载，在壳里**休眠**——因为
① 两个标记是 Electron preload 打的，Tauri 壳没打（壳只注入 `window.__DSH_PLATFORM__`）；
② `-webkit-app-region` 是 Electron 行为，Tauri 只认 `data-tauri-drag-region` 属性。

壳侧的既有机制恰好覆盖唤醒它所需的一切：主窗口 `initialization_script` 注入链
（`ui.rs:126-129`）随每次文档加载生效，`switcher.js` 已有「工作台 origin 精确判定 +
Shadow DOM UI」先例，`handoff-curtain.js`（ADR-0014）已有「document-start 改 dsh 页面
外观」先例。

## 2. 约束与硬指标

1. **不改 dsh 源码**（AGENTS.md 红线 1）——壳只做渲染层适配，不 fork / 不上游 patch。
2. **失败必须静默降级**：标记打不上、拖拽映射落空，最坏回退 = 原生标题栏，**不得**
   破坏工作台任何功能（现有注入脚本同口径）。
3. **三平台行为显式**：Windows/Linux 不做半成品沉浸式（无红绿灯 + Tauri 稳定版无
   `titleBarOverlay` 等价物），保持原生装饰。
4. **不新增 IPC 命令、不新增网络面**：复用既有 `get_workbench_url`（switcher 已在用）。
5. **不依赖 CSS Modules 类名**：dsh 构建产物类名带哈希，硬编码选择器必漂移。
6. 注入脚本只对**工作台 origin** 生效，壳页面（启动屏/选择器/关于）与控制中心窗口
   （label=profiles，独立创建不含本脚本）零影响。

## 3. 备选方案及评估

### 方案 A：标记补打 + 计算样式映射拖拽区 —— ✅ 最终采纳

- 思路：新增注入脚本 `immersive-chrome.js`（document-start、仅主窗口）：
  1. 同步判定 `location.hostname === '127.0.0.1'`（dsh 就绪 URL 恒为该形态，
     `shell.rs` 实测锚定）即补打 `data-platform="darwin"`，唤醒 dsh 自带桌面 CSS；
  2. 异步 `get_workbench_url` 精确确认 origin，不符则**撤标记 + 断观察器**；
  3. 扫描 computed style `-webkit-app-region` ∈ {drag, no-drag} 的元素，改设
     `data-tauri-drag-region="deep"/"false"`，MutationObserver 跟随 SPA 重渲染。
  窗口层（`ui.rs`，`#[cfg(target_os = "macos")]`）：主窗口
  `title_bar_style(TitleBarStyle::Overlay)` + `hidden_title(true)` +
  `effects(Effect::Sidebar, EffectState::Active)`（对标 Electron
  `vibrancy:'sidebar'` + `visualEffectState:'active'`）。**`hidden_title(true)` 不可省**：
  Overlay 只让红绿灯浮起，**不会**隐去 `.title()` 的文字——不设它，红绿灯旁边会长期挂着
  一行「DSH Dock」（维护者截图圈出的正是这块）。窗口标题本身不动（仍供窗口切换器/
  任务栏/关于弹窗），只关掉标题栏里的那份渲染（`ui.rs` 有内容闸门测试钉住）。
- 优点：零选择器知识、免疫类名哈希；dsh 的桌面布局代码全量复用（透明底/半透明侧栏/
  topStrip 与红绿灯共行都是 dsh 自己画的）；拖拽语义与官方逐条同构（deep 子树可拖 +
  no-drag 显式排除）。
- 代价/风险：① computed-style 全量扫描一次性成本（毫秒级，之后增量）；② 依赖 dsh 继续
  携带这套桌面 CSS（登记 §5 复核项）；③ Tauri `Overlay` 的已知 caveat（tauri 2.11.5
  源码注释）：各 macOS 版本标题栏高度不一、窗口未聚焦时拖不动（tauri issue #4316）、
  标题颜色随系统主题；④ 红绿灯位置无 Tauri 原生 API（见下 §3.1 补立）。
- 对照约束：①不改 dsh 源码；②扫描/标记落空即零动作；③Win/Linux 不进场；④零新 IPC；
  ⑤按计算样式而非类名；⑥origin 门 + 脚本只进主窗口链。

#### §3.1 补立（2026-09-21 真机截图后）：红绿灯定位走 AppKit FFI

维护者截图比对：壳侧红绿灯紧贴窗口顶沿，与 DSH 侧边栏收起按钮 `◫` 垂直中心错位（灯光在顶沿 y=7px，按钮在 y=26px）。
查证 Electron 源码（`shell/browser/ui/cocoa/window_buttons_proxy.mm`）与 AppKit 真实层级机制（2026-09-21 实测）：

1. **真实高度与坐标系**：AppKit 的 `NSTitlebarView` 默认高度仅为 32px 且非 flipped（原点在底部）。
   若直接设 `y = 18`，14px 按钮顶沿落在 `18 + 14 = 32px`（即直接贴死在窗口最顶端，距离顶沿 0px，中心距离顶沿仅 7px）！
2. **Electron 官方实现的关键**：`WindowButtonsProxy::redraw` 会将 `NSTitlebarContainerView`
   （即 `button.superview.superview`）与 `NSTitlebarView` 扩充到与 DSH 侧边栏 `.topStrip`
   一致的 52px 高度（`SidebarRoot.module.css:185`）；
3. **完美水平对齐计算**：在 52px 高度的 titlebar 容器内，将 14px 按钮垂直居中放置于
   `y = (52 - 14) / 2 = 19px`。在窗口全局坐标系下，按钮中心距离顶沿恰为 `52 / 2 = 26px`，
   与 `.topStrip`（`height: 52px; align-items: center`）内部垂直居中的收起按钮 `◫`
   完美处于**同一水平线**！
4. **间距自适应与全量事件防重置**：水平方向以 `TRAFFIC_LIGHT_X = 16.0` 为起始，间距根据系统按钮间隙（约 23px）自适应等距排列；
   并通过 Tauri `on_window_event` 在任何布局/尺寸变化时自动防抖重放。

代价：`objc2` 由传递依赖提升为直接依赖（lock 在册，无新 crate）；AppKit 调用限主
线程建窗后；失败只记日志、停默认位属可接受降级。已立内容闸门
（`traffic_lights.rs` 常量测试 + `ui.rs` 接线断言）。

### 方案 B：硬编码选择器注入 CSS/属性 —— ❌ 否决

- 否决理由：dsh 前端 CSS Modules 类名带构建哈希，每次 dsh 升级都可能变；壳侧无法
  稳定引用 `.titleRow`。违反约束 5。

### 方案 C：只改窗口层（Overlay），不动页面 —— ❌ 否决

- 否决理由：拿不到拖拽区（窗口不可拖），且 dsh 页面不知自己在桌面壳里，侧栏不透明、
  无 topStrip 适配——「沉浸式」只剩半个标题栏，与目标形态（截图）不符。

### 方案 D：fork/patch dsh 上游 —— ❌ 否决

- 否决理由：违反红线 1。

## 4. 最终决策

主窗口在 macOS 以 `TitleBarStyle::Overlay` + `hidden_title(true)` +
`Effect::Sidebar/Active` 呈现；新增注入脚本
`immersive-chrome.js` 仅在工作台 origin 补打 dsh 官方 preload 同款标记
（`data-platform="darwin"`），并把 dsh 自己 CSS 里的 `-webkit-app-region` 计算样式映射为
Tauri 的 `data-tauri-drag-region`。Windows/Linux 一期保持原生装饰（`data-windows-titlebar`
标记与自绘按钮不进一期）。**dsh 的桌面布局代码是事实源，壳只负责「唤醒 + 语义翻译」。**

## 5. 后果与后续行动项

### 正面后果

- dsh-dock 主窗口视觉与官方客户端对齐：红绿灯浮层、顶栏可拖、侧栏毛玻璃。
- 机制通用：dsh 前端后续任何按 `data-platform`/`data-windows-titlebar` 生效的桌面适配
  都会被壳自动继承（如 Windows 档将来进场只需补标记 + 自绘拖拽条）。

### 负面后果 / 新增债务

- **对 dsh 前端 CSS 的渲染层依赖**（本 ADR 即其登记处，dsh 升级须复核）：

  | 依赖 | dsh 源码位置（2026-09-21 读） | 失效症状 |
  |---|---|---|
  | `data-platform="darwin"` 标记名与整套桌面 CSS | `apps/desktop/src/preload-platform.ts:8`；`packages/client/**`（§1 表） | 侧栏不透明 / 无 topStrip / 无拖拽（降级为原生标题栏，不坏功能） |
  | `-webkit-app-region: drag/no-drag` 规则集 | `ConversationRoot.module.css:35-46`、`SidebarRoot.module.css:189-194`、`SidebarRight.module.css:90-94`、`AppFrame.module.css:34-40` | 顶栏/侧栏条带不可拖 |
  | 工作台 URL 恒 `http://127.0.0.1:<port>` | `shell.rs` 实测（`parse_detected_url` 系列） | 同步判定落空 → 标记晚一个 IPC 往返到达（仍可用，首帧可能闪一下不透明侧栏） |
  | Windows 桌面档（`data-windows-titlebar` + 40px 条） | `preload-windows.ts`、`AppFrame.module.css:20-40`、`SidebarRoot.module.css:45-101` | 一期不使用，仅登记备用 |

- Tauri `Overlay` caveat（未聚焦不可拖、标题栏高度随系统版本）需实机验证后决定是否
  补充说明到用户文档。
- `macos-private-api` Cargo feature 引入（target-gated，仅 macOS 目标启用）。

### 行动项

- [x] ADR-0029 立档 + README 索引（2026-09-21）
- [x] `frontend/src/lib/immersiveChrome.ts` 纯逻辑 + vitest
- [x] `frontend/src/injected/immersive-chrome.js` + `ui.rs` 接线（含 macOS cfg 门）
- [x] §3.1 补立：`traffic_lights.rs` 红绿灯 AppKit 定位（原生探针实证坐标系/间距）
- [ ] 实机验证：macOS 拖拽 / 红绿灯位置（官方 x16/y18 逐值比对）/ 侧栏 vibrancy /
      未聚焦态 / 深浅色
- [ ] dsh 升级时按 §5 表逐条复核（随广播知会）

## 6. 复审条件

- dsh 大版本升级或 `packages/client` 重构：§1 表的规则集/标记名变化即重开；
- Tauri 大版本升级：`TitleBarStyle`/`Effect`/拖拽区语义（`drag.js`）变化即重开；
- Windows 档要进场时：补 `data-windows-titlebar` 标记 + 自绘拖拽条（伪元素
  `::before` 无法属性映射）与原生 caption 颜色同步方案，另立评审；
- 若 tauri issue #4316（未聚焦不可拖）出现上游修复，评估去掉降级文案。
