# ADR-0030：Windows 撤回沉浸式标题栏，维持原生装饰

- **日期**：2026-09-23
- **状态**：已接受
- **提出人**：维护者（issue #16 触发裁定）+ AI 实施
- **相关方**：`src-tauri/src/ui.rs`（主窗口）、`frontend/src/injected/immersive-chrome.js`、
  `frontend/src/lib/immersiveChrome.ts`、`src-tauri/capabilities/default.json`
- **关联**：[ADR-0029](0029-immersive-titlebar-marker-injection.md)（本决策撤回其在 Windows 一侧的设想）；
  issue [#16](https://github.com/realguan/dsh-dock/issues/16)；`docs/team/平台对齐审计-2026-09-21.md` §3.1/§3.2

---

## 1. 背景与问题

v1.3.2（2026-09-22，提交 `73c6af1`）按维护者裁定「参照官方 dsh 客户端方案」落地了 **Windows
沉浸式标题栏**：`decorations(false)`（对标官方 Electron 的 `titleBarStyle:'hidden'`）+ 注入
官方同款 `data-windows-titlebar` / `--dsh-windows-titlebar-height:40px` 标记 + 壳**自绘**
最小化/最大化/关闭三控件。该提交自己写着「**待 Windows 真机验收（道 B 阻塞项）**：① 观感是否
与官方一致；② `decorations(false)` 后缩放/贴靠/投影是否退化 —— 若退化则回退本 cfg 块」，
随后即随 v1.3.2 发布。

2026-09-23，Windows 用户提 issue #16：**「web界面的 窗口没法移动 最大化 最小化 关闭按钮均无效果」**。
逐条复核发现四症状 = 三条独立断点（证据全部可在本机复核）：

| # | 症状 | 根因（证据） |
|:--|:--|:--|
| ① | 窗口无法移动 | dsh 的 40px 拖拽带是**伪元素**：`packages/client/ui-layout/src/client/AppFrame.module.css:40-46` 的 `.frame::before{ -webkit-app-region: drag }`；伪元素承载不了 `data-tauri-drag-region`，注入脚本的 Windows 分支又**完全没做** app-region 翻译（命中即 `return`）。ADR-0029 §6 早已写明此处「**另立评审**」，进场时未走 |
| ② | 三控件点了没反应 | 自绘控件调 `plugin:window\|minimize / toggle_maximize / close`，而 `core:window:default`（tauri 2.11.5，见本仓构建产物 `src-tauri/gen/schemas/acl-manifests.json`）只有 28 条只读 getter + `internal_toggle_maximize`——**三条命令均不在其中**，capabilities 又只授了 `core:default` ⇒ ACL 拒绝，调用点 `.catch(function () {})` 把它吞掉 = 用户侧「点了没反应」 |
| ③ | 鼠标缩放 / Aero Snap 失效（issue 未提，同源退化） | `decorations(false)` 在 tao 里一并摘掉 `WS_CAPTION \| WS_THICKFRAME`（`tao-0.35.3/src/platform_impl/windows/window_state.rs:307`）——官方 Electron 的 `titleBarStyle:'hidden'` 是**保留原生 frame** 的（可缩放 + 可贴靠），故「等价映射」在这一点上根本不成立，提交说明里「唯一偏差 = 自绘控件」不正确 |
| ④ | 窗口无法关闭（同上） | 三控件失效 + `WS_CAPTION` 消失 ⇒ 只剩 Alt+F4 / 托盘退出 / 任务管理器 |

**为什么闸门没拦住**：`ui.rs` 的 Windows 接线闸门只断言注入脚本里**出现**
`minimize()` / `toggleMaximize()` / `close()` 这些**字符串**，不断言 ACL 里有对应授权——
结构性通过、行为性失效（正是平台审计里点名的「验证空洞」一类）；而真机验收项自陈未完成即发版。

## 2. 约束与硬指标

1. **红线 3 口径**：撤回的是 v1.3.2 新增的「沉浸式观感」，不是既有能力——撤回后 Windows
   **恢复**移动 / 缩放 / 贴靠 / 原生三控件，能力面是**增加**不是收窄 ⇒ 不触发「删 leg / 删上传 /
   删发布下载」那类宪法级门槛。
2. **禁假实现 / 禁静默降级**（红线 3 附则）：一个「窗口拖不动、按钮点了没反应」的标题栏就是
   假实现，必须撤掉而不是留在包里等人踩。
3. **无真机不进正式包**：本仓库开发机为 macOS，Windows 窗口层行为（观感、缩放、贴靠、控件）
   只能在 Windows 上证明 ⇒ 任何 Windows 窗口层改动在恢复真机验收前不得进正式包。
4. **等价映射必须逐项对照**：`titleBarStyle:'hidden'` ≠ `decorations(false)`；跨框架对标要按
   「保留下来的系统能力」逐条核，不能只看名字对得上。

## 3. 备选方案及评估

### 方案 A：整体撤回，Windows 维持原生装饰 —— ✅ 最终采纳

- 思路：删 `decorations(false)` cfg 块；注入脚本收窄回 macOS 单平台（删除自绘控件与 Windows 标记）；
  纯模型去掉 Windows 计划；新增「防复辟」闸门。
- 优点：窗口能力**立即**恢复（移动 / 缩放 / 贴靠 / 三控件）；改动集中、回退面清楚；不再有
  静默失效的用户可见控件。
- 代价/风险：Windows 观感回到「原生标题栏 + 网页」两层皮，与官方客户端有可见差距。
- 对照约束：满足 1（能力增加）、2（撤掉假实现）、3（不引入需真机验证的新行为）、4（承认映射不成立）。

### 方案 B：保留沉浸式，就地补齐接线 —— ❌ 否决

- 思路：补 `core:window:allow-minimize / toggle-maximize / close / start-dragging`，把
  `data-tauri-drag-region` 挂到 `.frame`（承载 `::before` 的那个元素）上，自绘控件继续用。
- 否决理由：解决不了 ③——`WS_THICKFRAME` 被摘掉是 tao 的硬行为，要恢复鼠标缩放得自绘
  hit-test 缩放边框（`startResizeDragging`），贴靠之外的观感/行为还得逐项真机调；此外壳页面
  （启动屏 / 选择器）没有 40px 标题栏带，`decorations(false)` 下那段窗口**完全不可拖**，
  要改壳 UI 补一条带。整体远超一个补丁的体量，且期间用户手上的窗口是砖（违反约束 2、3）。

### 方案 C：保留 `decorations(false)`，只补 ACL 与拖拽属性 —— ❌ 否决

- 思路：最小改动止血（+4 条 window 权限 + 把拖拽属性挂到 `.frame`）。
- 否决理由：缩放边框与贴靠仍然丢失（③），窗口依旧不可缩放 ⇒ 只是把四个症状减到两个，
  仍属半成品；且没有任何 Windows 真机可以验收。

## 4. 最终决策

**Windows 撤回沉浸式标题栏，与 Linux 同口径维持原生装饰。** 主窗口不再出现
`decorations(false)`；注入脚本 `immersive-chrome.js` 收窄为 **macOS 单平台**（Windows 分支、
`data-windows-titlebar` 标记、自绘三控件、40px 常量全部移除）；纯模型 `immersiveChrome.ts`
同步去掉 Windows 计划。新增闸门 `windows_native_decorations_tests`（`ui.rs`）钉住：
`decorations(false)` 不得出现在生效代码里、注入脚本不得再带 Windows 分支、macOS 的
Overlay + 隐藏标题路径不得被误伤。

## 5. 后果与后续行动项

### 正面后果

- Windows 窗口恢复完整原生能力（移动 / 缩放 / 贴靠 / 最小化 / 最大化 / 关闭），issue #16 四症状消失。
- 少一处「自绘 UI 调用未授权 IPC 且静默失败」的隐患；注入脚本面积缩小。

### 负面后果 / 新增债务

- **Windows 观感退回两层皮**：与官方客户端（原生 overlay 标题栏 + 页面 40px 带）仍有差距；
  要重开必须先有 Windows 真机验收能力（见 §6）。
- **ADR-0029 在 Windows 一侧的设想作废**（`data-windows-titlebar` 标记、官方 40px 带），
  相关登记条目保留但标注失效。
- **macOS 侧缺口（已修，验证待补）**：ADR-0029 的 app-region → `data-tauri-drag-region`
  翻译依赖 `plugin:window|start_dragging`，该命令同样不在 `core:window:default` 且 capabilities
  未授 ⇒ 按 ACL 判据，**该翻译自 v1.3.0 起在真机上从未生效**（当前能拖的只是系统原生标题栏
  那一条带；dsh 自绘的 topStrip / titleRow 拖拽区是死的）。2026-09-23 已补
  `core:window:allow-start-dragging`（独立提交）并配 `immersive_drag_acl_tests` 配对闸门
  （负例实测：摘掉授权即红）；**但"拖得动"只能由 macOS 真机证明**，验证前不得对外宣称已修。
- 本次为代码 + 文档回退，**尚未发版**：v1.3.2 已在用户手上带着该缺陷，需 patch 版覆盖
  （发版须先按 `docs/prompts/release-notes.md` 落 `docs/RELEASE_NOTES.md`，禁裸打 tag）。

### 行动项

- [x] 撤回 `decorations(false)` 与 Windows 注入分支 + 纯模型（2026-09-23）
- [x] 「防复辟」闸门 `windows_native_decorations_tests`（正例 + 反例 + macOS 不误伤）（2026-09-23）
- [x] ADR-0030 立档 + 索引；ADR-0029 §5/§6 与本档互指（2026-09-23）
- [x] 平台审计 §3.1 / 平台对齐计划 B-4·C-4 状态回写（2026-09-23）
- [x] **补 `core:window:allow-start-dragging` 修 macOS 拖拽区** + 配对闸门
      `immersive_drag_acl_tests`（负例实测：摘掉授权即红）（2026-09-23）
- [ ] **macOS 实机验证拖拽**（顶栏 / 侧栏条带拖动 + 双击最大化）——验证前不得对外宣称"已修"
- [ ] 发 patch 版（v1.3.3）覆盖 v1.3.2 —— 发版日志 + 三平台验收清单按既有流程
- [ ] 恢复 Windows 真机验收能力后，再评估是否重开 Windows 沉浸式档

## 6. 复审条件

- **Tauri/tao 上游变化**：出现 Windows 侧 `titleBarOverlay` 等价 API，或 `decorations(false)`
  在 Windows 上不再摘掉缩放 / 贴靠 —— 此时方案 B 的成本结构改变，可重开。
- **dsh 上游变化**：Windows 标题栏拖拽带从伪元素 `::before` 改为真实元素（那时才可能属性映射）。
- **验收能力变化**：恢复 Windows 真机验证（`docs/roadmap.md` §4.16 / `docs/executor.md` 现为搁置）
  —— 这是重开任何 Windows 窗口层改动的前置。
- **平台审计重开**：若「三平台对齐」口径要求 Windows 也有沉浸式观感，须先解决真机验收，
  再按本档 §2 约束逐条重评。
