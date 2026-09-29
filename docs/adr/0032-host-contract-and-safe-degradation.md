# ADR-0032：宿主契约——把「我是桌面宿主」变成一份**完整的原子声明**，并让它可以自动降级

- **日期**：2026-09-29
- **状态**：**已接受，但范围经两次收束**
  1. 2026-09-29 首裁：采纳**路径 A（成为契约完整的桌面宿主）**+ 三层自动降级
     （原话「如果上游改动出现异常，能自动降级」）。降级交互见 §7 与
     [`docs/plans/host-contract-degradation-ux-2026-09-29.md`](../plans/host-contract-degradation-ux-2026-09-29.md)。
  2. ⚠️ **2026-09-29 终裁（现行）：dsh-dock 的定位是「web 档套壳」**（原话「我感觉跑偏了，
     我只想做个 web 档的套壳」）。**§3 方案 A 与 P1/P2/P3/P4 整体作废**；
     降级提示撤除；只保留 P0-a 的版本闸与原子性闸门。**见 §15（现行结论）**，
     §6/§7/§11/§13 收束为决策史，§14 记录了方案 D（时序打标记）实测不可行。
- **提出人**：维护者（真机启动事故触发）+ AI 实施
- **相关方**：`frontend/src/injected/immersive-chrome.js`、`src-tauri/src/ui.rs`、
  新增 `src-tauri/src/hostcontract/`、`frontend/src/injected/desktop-bridge.js`、
  `frontend/src/lib/immersiveChrome.ts`、`settings.rs`（持久化例外册）、
  `ipc.rs::COMMANDS` + `lib.rs` handler + `capabilities/default.json`（三处同步）
- **关联**：[ADR-0029](0029-immersive-titlebar-marker-injection.md)（本档是其**上游契约层**的
  立项与修订；§8 回写其 §7.3 依赖表）、[ADR-0030](0030-windows-keeps-native-decorations.md)
  （Windows 档撤回，本档不改该裁定）、[ADR-0012](0012-typed-boot-failure.md)（失败类型化，
  本档沿用其形状但**不**并入其六分类）、[ADR-0014](0014-restart-handoff-continuity.md)
  （交接幕布——降级金丝雀的遮盖面）、复现台账复现点 25

**上游锚点**：本机引擎 `@deepseek-ai/dsh@0.2.0-rc.1`（2026-09-29 13:46 升级）与官方客户端
`/Users/guan/Applications/dsh-official/…/app.asar → lib/preload-app.cjs`；逐条复现实录见
行为台账复现点 25。

---

## 1. 背景与问题

### 1.1 事故

dsh 升级到 0.1.7-rc.2 后，dsh-dock 主窗口工作台启动即报：

```
Failed to load plugins
@deepseek-ai/dsh-client-shortcuts
web boot: 25 entries did not activate
```

维护者追加的关键事实：**同一台机器、同一个 dsh，浏览器里打开完全正常，只有 dsh-dock 不正常。**
2026-09-29 在 dsh **0.2.0-rc.1** 上复现（23 条 pending，条目数差异来自版本），A/B 只差一个属性：

| 变体 | 注入 | 结果 |
|:--|:--|:--|
| A 对照（同一反代，逐字节转发） | 无 | ✅ `探索未至之境 \| 预览版 \| 选择工作区` |
| B 同代理 + `<html data-platform="darwin">` | 仅此一个属性 | ❌ `Failed to load plugins`，23 条 pending |

### 1.2 机制

`data-platform` 从「样式开关」升格成了「运行时身份判据」：

```js
// dsh-client-shortcuts/lib/client.js:721
function detectEnvironment(document, navigator) {
  const desktop = document.documentElement.dataset.platform;
  return { runtime: desktop === void 0 ? "web" : "desktop", … };
}
// :1854-1855
const keyboard = environment.runtime === "desktop" ? window.dshDesktop?.keyboard : void 0;
if (environment.runtime === "desktop" && keyboard === void 0)
  throw new Error("Desktop keyboard bridge unavailable");
```

`shortcuts` 服务因此从未注册 → 25（rc.2）/ 23（0.2.0-rc.1）个依赖它的客户端条目全部 pending。

### 1.3 关键事实：标记与能力对象是**同一份声明的两半**

官方 preload（`lib/preload-app.cjs`）**无条件**做两件事：

```js
markDocumentPlatform();                    // :826  ← 打 data-platform
exposeInMainWorld("dshDesktop",           // :823  ← 暴露能力对象
  location.protocol === 'dsh-app:' && location.hostname === 'app' && process.isMainFrame
    ? createProductApi()                  //   完整能力对象
    : { protocolVersion: 1 });            //   降级残桩
```

**在官方客户端里两者永远同时在场。** 而 dsh 上游对标记的**自述语义**是明确的身份断言——
`dsh-client-ui-primitives/lib/index.js:7947-7949`：

```js
/** … Read at render time — the mark may arrive as late as DOMContentLoaded.
 *  @returns true only inside the macOS Electron shell. */
function isDarwinDesktop() {
  return document.documentElement.dataset.platform === "darwin";
}
```

⇒ 结论：**dsh-dock 一直在宣称「我是官方 macOS Electron 宿主」，但只兑现了这份声明的表现层一半。**
rc.2 只是第一次对这项宣称收费。这不是「dsh 改了接口」，是**壳的声明不完整**。

### 1.4 同一轮升级里的第二处漂移（静默，且已经坏了）

ADR-0029 §7.3 登记的拖拽带钩子 `data-shell-leading-band` 在 0.2.0-rc.1 **全树 0 命中**，
只剩 `data-shell-leading`——**且语义变了**：它现在是 AppFrame overlay 层里
`leadingMounted &&` 的 **leadingSeat**（`dsh-client-ui-layout/lib/client.js:350`），
**不是**那条 52px / `pointer-events:none` 的拖拽带。

壳的 `BAND_SELECTOR = '[data-shell-leading-band]'`（[`immersive-chrome.js:55`](../../frontend/src/injected/immersive-chrome.js#L55)）
因此恒查不到，**静默退回顶部 52px 兜底几何带**——拖拽还活着，所以没人发现。
`ui.rs` 的内容闸门断言的是**这个已死的键名**，于是闸门**一直绿着**：假绿。

### 1.5 标记的消费者全集（机器枚举，2026-09-29）

全 dsh 读 `document.documentElement.dataset.platform` 的 **JS 只有 4 处**：

| # | 位置 | 用途 | 性质 |
|:--|:--|:--|:--|
| 1 | `dsh-client-ui-primitives/lib/index.js:7947` | `isDarwinDesktop()` 定义（自述「inside the macOS Electron shell」） | 身份 |
| 2 | `dsh-client-ui-layout/lib/client.js:240` | `collapsedWidth` 56→0（`:241`）、`leadingMounted`（`:314`） | **表现** |
| 3 | `dsh-client-ui-sidebar/lib/client.js:261` | 侧栏收起按钮的位置/形态 | **表现** |
| 4 | `dsh-client-shortcuts/lib/client.js:721` | `runtime: "web" \| "desktop"` | **身份** |

另有 `dsh-client-ui-layout/lib/client.js:350` 发布 `data-shell-leading` / `data-shell-overlay` 两个新钩子；
其余全部是被 `html[data-platform='darwin']` 选中的 **CSS**（页面透明、侧栏 tint、
`--dsh-frame-top-clearance: 48px`、plugin-manager 的 `calc(28px + var(--dsh-frame-top-clearance))`）。

⇒ **同一份标记在 dsh 内部被 3 处以「表现」消费、1 处以「身份」消费，而文档只描述了身份含义。**
这就是它对第三方壳脆弱的根因，也是本档对上游的唯一诉求（§5 行动项）。

### 1.6 做一个诚实宿主需要兑现的全部内容（机器枚举）

`"dshDesktop" in globalThis` 是**全有全无**的开关。2026-09-29 枚举到的消费者已从 5 个长到 7 个：

| 消费者 | 有 `dshDesktop` 后的行为 | 桥不完整的后果 |
|:--|:--|:--|
| `dsh-client-shortcuts` | 键盘走 native、配置走 `dshDesktop.shortcuts` | 缺 `keyboard` → **本次事故** |
| `dsh-client-ui-settings-account` | 整页账户设置进场（`:4197`） | 缺 `dshOnboarding` → `:4165` **`throw "desktop login bridge unavailable"`** |
| `dsh-client-ui-settings-models` | `credentialOnboarding` 关闭、不再注册 `settings.onboarding`（`:4010`、`:4076`） | **模型页凭证引导入口消失**，换成桌面 onboarding（另一套面） |
| `dsh-client-product-analytics` 🔺新 | `:20` 注册远端遥测策略流 | **静默开启产品分析上报**——产品/隐私决策 |
| `dsh-client-ui-chat` 🔺新 | `:12393` 转写默认切 `standard` | 轻微 |
| `dsh-client-ui-settings-general` | 读 `carrier.updates` 出更新徽标（`:978`） | 缺即无（可接受） |
| `dsh-client-ui-sidebar-browser` | `protocolVersion===1` 且有 `carrier.browser` 才切 Electron 托管页（`:1597`） | 只给 `protocolVersion` → 保持 iframe ✅ **可安全省略** |

新增可选字段 `deviceInfo`（`settings-account:4361`，`?.` + `void 0` 兜底 → 回退 `navigator.userAgent`）
⇒ **可安全省略**。

## 2. 约束与硬指标

1. **不改 dsh 源码**（AGENTS 红线 1）：不 fork、不 patch；读源码 + 文件系统层复现须登记台账。
2. **禁静默降级、禁假实现**（AGENTS 红线 3）：降级必须**显式可见并说明原因**——本档 §7 是其落地面。
3. **原子性（本档新增硬指标）**：`host_mode` 是**单一取值**，标记与能力对象**都从它派生**。
   两个方向都不得只翻一半：
   - 有标记无桥 → **本次事故**（插件构造抛错）
   - 有桥无标记 → `"dshDesktop" in globalThis` 为真 ⇒ **7 个消费者一起走桌面路径**
     （账户页进场、凭证引导消失、**静默开启遥测**）却没有对应表现层 ⇒ 换一种坏法
4. **降级目标必须永远可用**：web 宿主档 = 普通浏览器所见，dsh 必然支持（§3 实测变体 A 即证明）。
5. **不新增网络面**（唯一网络面仍是 `updates.rs`，ADR-0006）；新增 IPC 须按 §7 三处同步 + 登记册。
6. **平台显式**：本档只影响 macOS 宿主档；Windows/Linux 维持 ADR-0030 的裁定，不得被本档顺手改动。
7. **三平台可编译**：`hostcontract` 的平台分叉必须 `#[cfg]` 显式，CI 各目标 clippy 各自过。

## 3. 备选方案及评估

### 方案 A：成为契约完整的桌面宿主（标记 + `dshDesktop` 全套）—— ✅ 最终采纳

- 思路：把 §1.6 的消费者清单逐项兑现，标记与能力对象作为**一份原子声明**同时落下；
  配套 §6 的**三层自动降级**，使这份声明可回退。
- **可行性已实测**（2026-09-29，dsh 0.2.0-rc.1，三变体）：仅注入
  `data-platform` + `dshDesktop{protocolVersion, keyboard, shortcuts}` + `dshOnboarding`
  ⇒ **启动转绿**，且 dsh **确实消费**了我们的桥（`keyboard.subscribe` ×1、
  `shortcuts.subscribe` ×1、`shortcuts.get` ×31）。
- 优点：产品形态与官方客户端对齐；顺带解决 §1.6 各项；不再有「半份声明」。
- 代价/风险：**永久背负一份上游私有契约**（本轮升级即两处漂移）；`dshDesktop` 存在会
  静默打开遥测；`keyboard` 桥对 WKWebView 是**补偿一个并不存在的问题**（见下）。
- 对照约束：② 由 §6 三层降级满足；③ 由原子性闸门满足；④ 由「降级即回 web 档」满足。

#### §3.1 keyboard 子决策（本档最需要说清的一处）

`installKeyboard`（`dsh-client-shortcuts/lib/client.js:736-815`）的 native 分支只做一件事：

```js
fixed?.({ type: "keydown", gesture, context, consume });  // 固定动作仍由 DOM 喂
if (native) return;                                      // ← 可配置快捷键不再走 DOM
shortcuts.dispatch({...gesture, composing: …}, context, consume);
```

其中 `native = keyboard !== undefined && (platform === "macos" || platform === "windows")`（`:1884`）。

**这个桥的成因是 Electron 的原生菜单在 webview 之前截走按键**，必须在原生层把输入送回页面。
**WKWebView 没有这个截取**——DOM 交付本来就是完整正确的。因此对 Tauri 壳而言，装上这个桥
= 把 dsh 已经做对的 DOM 交付换成一次往返，且 `installNativeKeyboard` 把 `composing`
**硬编码为 `false`**（`:1902` 附近）⇒ 输入法组合期可配置快捷键会误触发，且症状静默。

**裁定**：桥**必须提供**（否则 `runtime==="desktop"` 不成立、事故复现），但实现必须
**在壳侧自守 composition**——注入层跟踪 `compositionstart/end` 与 dead key，组合期**不转发**，
以复刻 dsh DOM 路径的语义。该自守必须带测试，不得以「dsh 那边会处理」为由省略。

### 方案 B：撤标记 + 撤 macOS overlay/vibrancy（回原生装饰）—— ❌ 否决

- 思路：不再宣称桌面身份，视觉退回原生标题栏。
- 否决理由：**产品倒退**，且并不彻底——维护者已裁定沉浸式标题栏是主窗口的目标形态
  （ADR-0029）。它是**降级目标**（§6 的 web 宿主档），不是终态。

### 方案 C：构造期撤标记、mount 前恢复（时序规避）—— ❌ 否决

- 否决思路：让 `shortcuts` 构造时读到 `web`，布局挂载前再打上标记。
- 否决理由：首帧与布局初值都会错（`collapsedWidth` 与 `leadingMounted` 在挂载期读取），
  且把 bug 从「启动就红」变成「偶发错位」——**更精巧的假话**，违反约束 2。

### 方案 D：只推上游（把表现层与身份解耦），壳不改 —— ❌ 不作为依赖

- 思路：让 `detectEnvironment` 改看能力对象，标记降为纯表现层。
- 评估：**方向正确但不可依赖**——上游节奏不受我们控制，而线上事故是现在进行时。
  本档将其列为**并行轨**（§5 行动项），若采纳则 §6 的 P1 负资产可整条退役，
  但不能成为交付前提。

### 方案 E：引擎降回 0.1.7-rc.1 —— ❌ 否决（临时手段，不作为方案）

- 思路：挡住升级。
- 否决理由：不解决问题，只是推迟；且与 ADR-0010 的引擎自升级语义冲突。**仅作为事故当日的应急**。

## 4. 最终决策

**采纳方案 A**：dsh-dock 以「契约完整的桌面宿主」为目标形态，`data-platform` 与
`dshDesktop`/`dshOnboarding`/页面级全局作为**一份原子声明**同时落上；同时以
**三层自动降级**（版本闸 → 首启金丝雀 → 运行时看门狗）保证这份声明永远可回退到
已知良好的 **web 宿主档**。分期落地，**标记是最后一步而非第一步**。

## 5. 后果与后续行动项

### 正面后果

- 工作台恢复可用，且沉浸式标题栏得以保留（不再靠半份声明）。
- 顺带解决：快捷键配置持久化（见 §5.1）、账户页/`deviceInfo`、目录选择、宿主路径、更新徽标。
- 「依赖私有内部」这一失效模式**第一次有了机器兜底**：升级即探，判负即退。

### 负面后果 / 新增债务

- **永久契约债**：`dshDesktop` 是上游私有面，每加一个字段就是一笔债；本轮升级已两处漂移。
- **遥测**：`dshDesktop` 存在会激活 `dsh-client-product-analytics`。**必须显式裁定**，
  不得由「补桥」这个技术动作顺带决定（§5 行动项）。
- **keyboard 负资产**：§3.1 的自守 composition 是壳侧重复实现。
- **降级有成本**：每次契约漂移 = 一次幕布后的重载（L2/L3），或一次直落 web 档（L1）。

### §5.1 顺带修掉的独立缺陷（本次新发现，与本档解耦）

`webShortcutStorage` 用 `localStorage["dsh.keybindings.v1"]`（`dsh-client-shortcuts/lib/client.js:849`），
而 origin 由 `--port 0` 决定（[`shell.rs:122`](../../src-tauri/src/shell.rs#L122)，OS 随机端口）
⇒ **origin 每次启动都变 ⇒ 用户改过的快捷键从来没存下来过**。两条出路：
① P1 落地后由 `dshDesktop.shortcuts` 天然解决（宿主存储，与 origin 无关）；
② 独立小修：壳持久化上次成功端口并优先复用，使 origin 稳定。
**② 零新契约，且顺带稳住 dsh 一切 localStorage 状态**，建议无论走哪条路都做。

### 行动项

- [x] **P0-a** 契约表（dsh 版本区间 → 宿主契约代次）+ 版本闸 + `host_mode` 单一来源
      + **原子性闸门**（注入脚本里标记与桥必须同源，构建期可断言）
      —— 2026-09-29 落地：`src-tauri/src/hostcontract.rs`（裁决 + fragment 通道）
      + `resolve::LaunchSpec.host_mode` + `Executor::host_mode` + `boot.rs` 导航时下发
      + `immersive-chrome.js` 的 `HOST_MODE` 门；**三段实测通过**（见 §10）
- [ ] **P0-b** conformance 套件产品化（2026-09-29 的三变体探针 → `scripts/tests/`，
      接 CI 升级门）。**rig 的 6 条经验见 §13.4**——产品化时必须一次性解决，
      否则每次升级复核都要重踩（尤其：launch token 一次性、反代直通 SSE、转发 POST body）
- [x] **P0-c① 窗口层原子性 + 降级呈现**（2026-09-29）：`traffic_lights.rs` 增
      `set_desktop_host_active`（裁决翻窗口形态 + 红绿灯重放同源自停），`boot.rs` 导航时以同一
      `host_mode` 调用；`immersive-chrome.js` 增 `renderDegradedNotice`（Shadow DOM 浮层：
      展开 → 12s 自动收为状态点 → 同键不再弹；仅真动作按钮）。**三种行为实测通过**（下方 §11）。
      新闸门 5 条（键名三方同步 / 裁决形状 / 存储边界 / 窗口同源 / 顺序）均做过负例验证。
- [x] **P0-c② 常驻可查面**（2026-09-29）：让降级提示里「打开控制中心」那个按钮
      **名副实归**——它此前指向一个没有任何对应信息的面（按钮承诺了目的地没有的东西）。
      新增只读 IPC `get_host_contract`（三处同步 + 登记册）；`hostcontract::view_for()`
      纯函数生成 `HostContractView{mode,dshVersion,requiredGeneration,suppliedGeneration,reason}`；
      控制中心**偏好页顶部**（默认落地 tab，点完按钮不用再点一次）+ **关于页运行环境区**
      同源两处展示，均 macOS-only。见 §12。
- [ ] **P0-c③ 金丝雀 L2 + 一次性回滚 + `hostContract` 持久化登记（AGENTS §6）**：
      本轮**刻意不做**——见 §11「持久化为何延后」
- [ ] **P0-d** 修**假绿闸门**（三处断言的都是已死键名 `data-shell-leading-band`）：
      [`ui.rs:897`](../../src-tauri/src/ui.rs#L897)（沉浸式脚本内容闸门）、
      [`ui.rs:1310`](../../src-tauri/src/ui.rs#L1310) 与
      [`ui.rs:1348-1350`](../../src-tauri/src/ui.rs#L1348-L1350)（拖拽带钩子常量同步闸门）；
      改为能红的判据（新钩子名 + 几何兜底判据）。**注意 `ui.rs:895` 的
      `"dataset.platform"` 是对的**，不要一并改掉
- [ ] **P1** `dshDesktop{protocolVersion, keyboard, shortcuts}` + `dshOnboarding`；
      **遥测显式裁定**；`shortcuts.get` 会被调用 31 次 ⇒ 实现必须廉价且幂等
- [ ] **P2** 账户 / 引导面（官方在独立 welcome 窗口做 onboarding，`preload-welcome.cjs`；
      否则 `settings-models` 关掉凭证引导后用户无处输入 key）
- [ ] **P3** `__DSH_DIRECTORY_PICKER__` / `__DSH_HOST_PATHS__` / `__DSH_LOCALE__` /
      `dshDesktopBoot` / `dshPlatform`
- [ ] **P4** `browser`（Electron BrowserView 无 Tauri 等价物；缺失即保持 iframe，**安全省略**）
- [ ] **并行轨** 上游 issue：`data-platform` 在 dsh 内部被 3 处以「表现」消费、1 处以「身份」
      消费，而文档只描述身份含义（`ui-primitives/lib/index.js:7947-7949`）⇒ 请求发布
      **表现层专用标记**，或明确允许第三方壳设置该标记
- [ ] **文档同步**：AGENTS §6 持久化例外册登记 `hostContract`；复现台账复现点 25；
      ADR-0029 §8 回写其 §7.3 依赖表
- [ ] **验证**：`cargo test` / `cargo clippy --all-targets -- -D warnings`（宿主 + 各目标）、
      前端 `typecheck/lint/test`；真机清单 = 金丝雀判负 → 降级可见 → 重试 → 壳更新后自动恢复

## 6. 三层自动降级（本档的核心机制）

**结构性前提**：`data-platform` 与 `dshDesktop` 都在**插件构造期**被读取，因此降级**不可能**
是页内 try/catch——必须是**跨页面加载的状态机**。这一点想清楚后，其余均为可实现。

| 层 | 触发 | 机制 | 覆盖的失败模式 |
|:--|:--|:--|:--|
| **L1 版本闸** | 启动前（壳已知 dsh 版本） | 契约表只放行已验证版本区间；**未验证版本一律不进桌面档** | 上游发布新版本（**本次事故的全部**） |
| **L2 首启金丝雀** | 任一新版本首次启动 | 在既有交接幕布（ADR-0014）**后面**进桌面档，跑存活探针；判负则记 `rejected` + 翻 web 档 + 重载 | 同区间内的契约漂移 |
| **L3 运行时看门狗** | 运行期 | 廉价不变量检查（标记被清、桥方法久不被调用） | 仅在交互时才显形的漂移 |

### 6.1 存活探针：**观测我们自己有没有被消费**

`dsh-client-shortcuts` 的构造函数调 `syncDefinitions()`（`:1948-1950`）→
`this.adapter.get(...)`，而桌面档的 `adapter` 就是 `window.dshDesktop.shortcuts`
（`:879-881`）。因此：

> **桌面档健康 ⟺ 壳提供的 `shortcuts.get` / `keyboard.subscribe` 被调用。**

三个性质使它优于任何「读 dsh 错误文案」的方案：

1. **零 dsh 内部知识**——不读其文案、不猜其结构，只观测我们自己的契约有没有被消费；
2. **抗上游重构**——插件改名/重写/拆包都无所谓，判据是「该发生的事有没有发生」；
3. **fail-closed**——判据方向正确（缺调用即判负），不会因上游改一句文案而假绿。

2026-09-29 实测：本次事故在该探针下**必然判负**（marker 打了、`get` 一次未被调用）；
web 档下探针天然静默，无假阳性。

### 6.2 生命周期

```
版本 V 启动：
  V ∈ verified  → 直进桌面档（零额外延迟）
  V ∈ rejected  → 直进 web 档 + 说明（不再打扰）
  否则          → 金丝雀（幕布后）：
                    通过 → 记 verified，揭幕，沉浸式（用户无感）
                    判负 → 记 rejected，翻 web 档重载，揭幕，显式说明
```

**记忆三元组 =（引擎身份, dsh 版本, 壳契约代次）**，三者缺一不可：
- **引擎身份**：宿主引擎与 WSL 客体引擎可以是不同 dsh 版本（台账复现点 22 已记该探测缺口），
  不得共用一份裁决；
- **壳契约代次**：缺此维则 `rejected` 成为**永久判决**——壳升级后明明已适配，用户却永远卡在降级。

**恢复必须主动**：壳启动时发现「该 dsh 版本曾判负、但本次契约代次已支持」⇒ 自动重跑金丝雀，
通过则恢复并提示。不得等用户手动重试。

### 6.3 类型化（对齐 ADR-0012 的教训：措辞即契约）

```rust
enum HostContractMode { Desktop, Canary, Safe }
enum DegradeReason {
    BridgeNotConsumed { probe: String, budget_ms: u64 },
    BootDiagnosticsNonEmpty { entries: u32 },
    MarkerClobbered,
    ProbeTimeout,
    ForcedByUser { user_confirmed: bool },
}
```

新增变体触发 `match` 穷尽编译错误。**`HostContractState` 独立于 `BootFailure`**——
ADR-0012 §2 已把该模块边界限定在 boot 失败路径；「外观降级」不是 boot 失败，
塞进去会污染六分类语义。

## 7. 降级与呈现（摘要）

完整交互设计见 [`docs/plans/host-contract-degradation-ux-2026-09-29.md`](../plans/host-contract-degradation-ux-2026-09-29.md)。
三条不可协商的口径：

1. **这不是错误，是状态。** 工作台完全可用，只是外观回退。**不得**做成模态错误对话框，
   **不得**进 `BootFailure` 六分类。
2. **一次打扰 + 常驻可查。** 首次判负提示一次（工作台内注入层，与 `switcher.js` 同族），
   之后收为状态点；控制中心/关于常驻可查。对齐通知设计稿「N7 默认沉默」的判据
   （*"否则每次开机都弹一次，三次之后用户就会把权限关掉"*）。
3. **每个状态给出一个真的有用的下一步；给不出就不摆假按钮。**
   主按钮优先「检查 DSH Dock 更新」（绝大多数情况下能恢复它的是壳的新版本）。

文案须进 `frontend/src/content/zh-CN.ts` 集中管理（键名风格见该文件既有分组）。

## 8. 对 ADR-0029 的回写（本档生效即执行）

ADR-0029 §7.3 依赖表中**已失效**的两条按下表替换：

| 原依赖（失效） | 现状（2026-09-29，dsh 0.2.0-rc.1） | 处置 |
|:--|:--|:--|
| 拖拽带钩子 `data-shell-leading-band` | **全树 0 命中**；仅存 `data-shell-leading`，且语义变为 overlay 层的 **leadingSeat**（`ui-layout/lib/client.js:350`） | 壳改为按**几何语义**自驱（现兜底路径转正），钩子若要用须重新按新语义登记 |
| `isDarwinDesktop()` 读 `dataset.platform` | 仍存在（`ui-primitives/lib/index.js:7947`），且**新增** `ui-sidebar:261`、`ui-layout:314` 两处表现层消费；`shortcuts:721` 将其用于**身份** | 见 §1.5；本档 §6 的探针即该依赖的机器兜底 |

## 9. 复审条件

- **上游发布表现层专用标记，或明确允许第三方壳设置 `data-platform`**：§6 的 P1（keyboard 桥）
  及其负资产可整条退役 ⇒ 重开本档，范围大幅收窄。
- **dsh 大版本升级**：契约表逐项复核 + 复现台账复现点 25 逐条复核。
- **上游新增 `dshDesktop.*` 字段或新增 `"dshDesktop" in globalThis` 消费者**：§1.6 表重算。
- **`dshDesktop.protocolVersion` 语义变化**：协商机制重开。
- **降级率指标劣化**（判负次数占总启动次数比例上升）：说明契约表过宽或探针过严，重开。
- **Tauri 大版本升级**：`initialization_script` / 幕布遮盖面 / `on_window_event` 语义变化即重开。

## 10. P0-a 验证记录（2026-09-29 当日闭环）

**做法**：用**壳自己的注入脚本原文**（`frontend/src/injected/immersive-chrome.js`，
逐字 include）驱动一个反代，把 dsh 0.2.0-rc.1 工作台逐字节转发，唯一变量是 URL fragment。
`platform_script` 的 `os` 值取 `std::env::consts::OS` 的真实值 `macos`。

| 场景 | fragment | `data-platform` | dsh 启动诊断 |
|:--|:--|:--|:--|
| **P0-a 生效**（0.2.0-rc.1 的真实裁决） | `#dshHostMode=safe` | `null` | ✅ 干净（`探索未至之境 \| 预览版 \| 选择工作区`） |
| **反例**（未验约就宣称） | `#dshHostMode=desktop` | `darwin` | ❌ `Failed to load plugins`，23 条 pending |
| **兜底**（fragment 缺失） | 无 | `null` | ✅ 干净 |

四条判据全过：safe ⇒ 不打标记且工作台干净；desktop ⇒ 仍复现红屏（证明门不是摆设）；
fragment 缺失 ⇒ fail-closed；脚本确实注入（以上非 vacuous）。

> **探针踩过的两个坑（记下来以免重犯）**：
> ① `__DSH_PLATFORM__.os` 的真实值是 `std::env::consts::OS` = `macos`，**不是 `darwin`** ——
> 第一次探针填 `darwin` 撞上平台门 `platform.os !== 'macos'` 提前 return，三个场景全绿
> 但是 vacuous 的（"没打标记"是因为脚本根本没跑，不是因为门拦住了）。**判据全过的
> 同时可能一个都没验到**——所以探针必须断言"脚本确实执行过"（此处用
> `window.__dshDockImmersiveInjected`），否则等于没测；
> ② `Runtime.evaluate` 里手写 IIFE 时，外层模板字符串的 `${...}` 会被 JS 侧求值 ——
> 拼接探针源码时务必转义或改用字符串拼接。

**P0-a 期间新增的机器闸门**（`ui.rs::immersive_chrome_tests`）：
- `immersive_script_gates_on_the_host_contract_verdict` —— 三判据：裁决键名与 Rust 常量
  一致；`if (HOST_MODE !== 'desktop') return;` 这条 fail-closed 早退必须在场；
  **标记写入点必须出现在裁决之后**（顺序即语义，`mark_at > gate_at`）。拆掉门闸门即红。
- `effective_code` 抽成模块内 helper：剔注释后再断言。**这是 P0-d 那类假绿的解药**
  ——断言"机制在不在"，不是"字符串在不在"。

**P0-a 期间修掉的既有病灶**：`ui.rs` 的 `immersive_script_carries_its_contract` 原先内联
剔注释逻辑，本次抽为 `effective_code` 并被新闸门共用，避免两处各写一份（历史教训：
重复实现必然只改一处）。

**遗留边界（已知，不阻断）**：
- `dsh` 若在 SPA 内 `history.replaceState` 抹掉 fragment，刷新后判据丢失 ⇒ fail-closed
  退 web 档（**安全方向**，观感丢失而非红屏）；反向（老版本丢沉浸式）同属观感损失。
  P0-c 的 L3 看门狗负责把"页面现状 vs 裁决"对上账。
- **P0-a 不含降级呈现**：当前判 `safe` 时**只有 tracing 日志，用户界面无任何提示** ——
  按 AGENTS 红线 3 这是不允许的终态，故 P0-c（提示面 + 持久化）必须跟上。
  P0-a 单独上线只作为"止血"的中间态，**不得**作为发布态。

## 11. P0-c 验证记录与两处刻意的不做（2026-09-29）

### 11.1 修掉 P0-a 埋下的半坏形态（本轮最重要的一项）

P0-a 只翻了页面层，窗口层仍是 `TitleBarStyle::Overlay` + vibrancy。二者是 ADR-0029
立档时判定的**原子决策**：只翻一半 ⇒ 红绿灯悬浮在**不通明**的 web 页面上、页面也不为
红绿灯让位（`--dsh-frame-top-clearance` 不生效）。故 P0-c① 让**同一个 `host_mode`**
同时驱动两处：`traffic_lights::set_desktop_host_active(window, host_mode == Desktop)`。

`safe` 档下该函数做三件事：`set_effects(None)`（撤 vibrancy）、
`set_title_bar_style(Visible)`、`set_decorations(true)`；并让红绿灯重放
（`align`）**同源自停**——原生装饰后系统自管的按钮位才是正确位，再 `setFrameOrigin`
反而会把按钮拖进标题栏。

### 11.2 降级提示（交互设计稿落地面）

`renderDegradedNotice()`，Shadow DOM 浮层（与 `switcher.js` 同族同 token 镜像），
**五种行为实测通过**（`/tmp/dsh-notice-probe2.mjs`，真实工作台文档）：

| # | 行为 | 实测 |
|:--|:--|:--|
| 1 | 初次加载（未读过） | 展开态（title/body/三按钮齐全，top=42px 让开胶囊带，z=999998） |
| 2 | 12s 后自动收起 | 只剩状态点 ✅ |
| 3 | 点状态点重新展开 | ✅ |
| 4 | 「详情」开合 | ✅ |
| 5 | 「知道了」→ 记键 → 刷新 | 只留状态点，不重复打扰 ✅ |

**三个刻意选择**：
- **只给真动作按钮**：「打开控制中心」（`open_profiles_window`，常驻可查面就在那里）/
  「详情」（本地面板）/「知道了」；**不给「重试」**——判据是版本 × 契约表，立刻重试只会
  以同样方式再判一次，摆了就是假按钮。
- **判据是 `=== 'safe'`，不是 `!== 'desktop'`**：UNKNOWN（fragment 缺失/不认识）时
  不弹提示——那与「裁决说 safe」是两回事，弹了还会给用户错的解释（"版本不兼容"）。
- **存储边界**：这是注入层**第一次**往 dsh 文档的 Web Storage 写值。只写 `sessionStorage`
  （不落盘、随标签页消亡），键带 `dsh-dock-` 前缀，值只是"已读过哪版裁决"。
  **不碰 localStorage**——那是 dsh 自己的持久状态区（`dsh.keybindings.v1` 就在那里）。
  闸门把这三条钉死（含一条负例：改成 localStorage 即红）。

### 11.3 持久化为什么延后（P0-c③）

UX 设计稿里的 `hostContract`（引擎身份 × dsh 版本 × 壳契约代次 三元组）是**金丝雀 L2/L3**
的记忆体，用来记住"这个版本判负过、别再试"。但**当前没有金丝雀**——裁决完全由
`mode_for(版本)` 确定性算出，没有任何"试过一次才知道"的中间态。

**确定性重算严格优于持久化缓存**：没有缓存就没有"壳升级后代次变了但缓存没失效"
这类 bug。故本轮不引入 `settings.json` 字段；等 L2 金丝雀真有状态要记时再登记（AGENTS §6）。
当前跨启动记忆 = **无**，这是有意的。

### 11.4 本轮新增/加固的机器闸门（5 条，**均做过负例验证**）

| 闸门 | 钉什么 | 负例 |
|:--|:--|:--|
| `window_form_and_marker_share_one_verdict` | 窗口形态必须从同一 `host_mode` 派生 | 把实参改成常量 `true` → 红 ✅ |
| 裁决键三方同步（并入 `injected_script_constants_match_the_pure_model`） | Rust/TS/JS 三处同键 | 改注入侧键名 → 红 ✅ |
| 裁决形状（同上报） | 必须 `=== 'safe'`（非 `!== null` / `!== 'desktop'`） | 改成 `!== null` → 红 ✅ |
| 存储边界（同上报） | 禁 localStorage + 去重键必须命名空间化 | 改成 `localStorage` → 红 ✅ |
| `immersive_script_gates_on_the_host_contract_verdict`（P0-a 立） | fail-closed 早退在场 + 标记写点在裁决之后 | 把标记写点挪到门前 → 红 ✅ |

### 11.5 未做 / 待办

- **控制中心「外观」状态行 + 关于条目**（常驻可查面的另一半）：当前只有工作台内的状态点
  与「打开控制中心」入口，控制中心本身还没有对应展示。按红线 3 这仍算"可查到"，
  但完整形态需要它。
- **真机验证**：窗口形态切换（overlay ↔ native）在真机上的观感、红绿灯是否回到系统位、
  未聚焦态拖拽——本轮只能实测页面层（无显示环境）。按 ADR-0029 的先例，这类必须人工验。
- **L2 金丝雀 / L3 看门狗**（ADR-0032 §6）：本轮只交付 L1（版本闸）。

## 12. P0-c② 验证记录（2026-09-29）

**动机（一个我自己留下的洞）**：P0-c① 的降级提示里，「打开控制中心」是唯一的真动作按钮，
但它指向的控制中心**当时没有任何宿主契约信息**——按钮承诺了目的地没有的东西。
这比"少一个展示面"更严重，所以 P0-c② 的第一优先级是补它。

**落地**：
- `hostcontract::view_for(mode, dsh_version)` 纯函数 → `HostContractView`（camelCase）。
  文案按**失败方向分三种**：版本未知 ≠ 版本不兼容 ≠ 代次不足——用户能据此做不同动作
  （测试 `view_distinguishes_unknown_version_from_incompatible` 钉住，且断言
  未知**不得**被写成"不兼容"）。
- 只读 IPC `get_host_contract`：三处同步 + 登记册 + `tauri.ts` api 方法；dsh 版本走既有
  `engines::probe_engine`（不另起一套探测）。
- 双展示面、**同源同一份 IPC**（禁双源，AGENTS §7 同口径）：
  控制中心偏好页**顶部**（偏好页是控制中心默认落地 tab ⇒ 点完按钮不用再点一次）
  + 关于页运行环境区（`DimRow` 同族）；均 macOS-only（Win/Linux 恒原生，ADR-0030）。
- `reason` 渲染的是**机器判据**（版本 + 代次缺口 + 可行动作），可整句粘贴进 issue。

**凭据**：Rust `540 passed / 0 failed`（+10）、clippy 0、fmt 净；前端 `719 passed`（+13）、
tsc 0、oxlint 0；`scripts/tests/*.py` 全过。**闸门负例验证**：
撤掉 reason 渲染（= 静默降级）→ 红；撤字典键 → 红。

**两处返工（都是闸门抓到我的，记下来以免重犯）**：
1. `StateBlock` 没有 `warn` tone 且是大号居中块——不适合承载一句 inline 原因。
   改用既有 `DimNote`/裸段落（与关于页同族）。
2. 测试里写 `expect(body).not.toContain("invoke(")` 想断言"不绕过 api 对象"，结果
   **断言字符串本身**触发了 Rust 侧 `no_direct_invoke_outside_tauri_ts`（那条闸门只剔
   `//` 注释行，不剔字符串）。该检查本就由 Rust 侧对**全部**前端文件全局把关，
   重复断言是净负收益 —— 删掉。
   **教训**：想用"禁某模式"的断言时，先确认那条禁令有没有全局闸门；有就别在测试里复述它。**

**未做**：真机渲染未验（无显示环境）；`get_host_contract` 的 `dshVersion` 与控制中心
「关于/诊断」用的探测同源但**每次调用重新探测**（数百 ms 级子进程开销）——
当前调用点只有两个页面挂载，可接受；若将来做成实时刷新需加缓存。

## 13. P1 前置实验结果（2026-09-29，dsh 0.2.0-rc.1 实测）

**做法**：在 harness 里给工作台文档注入宿主契约**第 1 代最小集**
（`dshDesktop{protocolVersion:1, keyboard, shortcuts}` + `dshOnboarding`）+
`data-platform="darwin"`，与 P0-a/P0-c 的探针同一套反代 rig。

### 13.1 已证实

| # | 判据 | 结果 | 证据 |
|:--|:--|:--|:--|
| 1 | **启动干净** | ✅ **通过** | 0 个 pending / 无 `Failed to load plugins`；工作台正常渲染 |
| 2 | **桥被 dsh 消费** | ✅ **通过** | `keyboard.subscribe` ×1、`shortcuts.subscribe` ×1、`shortcuts.get` **×21** |
| 3 | **投递路径可用** | ✅ **通过** | 由 `keyboard.subscribe` 捕获的 listener 可调用，调用无异常（`delivered:true, threw:null`） |

⇒ **P1 的可行性前提成立**：最小集足够让工作台从红屏转为正常启动，且 dsh 确实消费我们提供的桥。
这与 P0-a 的结论一致（当时用同样的最小集转绿），本次把"消费面"也测出来了。

### 13.2 结论来自源码、无需实测的两项

- **IME 组合期不会被 dsh 自己过滤**：`dsh-client-shortcuts:912` 的 native 分支是
  `registry.dispatch({...input, composing: false, defaultPrevented: false}, context, () => {})`
  —— `composing` 被**硬编码为 false**，而 `dispatch` 开头 `if (gesture.defaultPrevented || gesture.composing) return`
  正是靠这两个字段过滤。故**壳必须自守 composition**（组合期不投递），否则 IME 期间快捷键会误触发。
- **desktop 档旁路 region 门控**：`dsh-client-shortcuts:675`
  `priority = runtime === "desktop" && (windows || macos)` ⇒ 区域检查被短路。
  即 P1 之后"可配置快捷键不再按焦点区域过滤"——这是相对今天 web 档的**真实行为变化**。

### 13.3 未决（阻塞 P1 决策）

- **判据"投递是否真的触发命令"未测到**：需要工作区加载后侧栏可切换才好观测，
  而 harness 里 dsh 的工作区引导（目录选择）难以自动化。**该 rig 在流式响应、
  一次性 launch token、SPA 导航三处反复受挫**，继续投入产出比低。
  ⇒ 建议把这条**降级为 P1 的验收项**（在壳的真实环境里跑，那时工作区本来就有），
  而不是继续在一次性 rig 上加码。
- **遥测**：`dsh-client-product-analytics:20` `if (!("dshDesktop" in globalThis)) return;`
  ⇒ 桥存在即激活远端策略流。**这是产品/隐私决策，须维护者裁定**；壳侧暂无已知开关
  （CLI 的 `DSH_TELEMETRY_DISABLED` 是另一条路径）。

### 13.4 给 P0-b（conformance 套件）的 rig 经验

本轮的探针 rig 踩过的坑，产品化时必须一次性解决，否则每次升级复核都要重踩：
1. **launch token 一次性**：复用 URL 必得 401 ⇒ rig 必须每次起全新 dsh 并重新取 URL。
2. **反代必须直通 SSE**（`text/event-stream`）：`arrayBuffer()` 缓冲会 `BodyTimeoutError`
   并带崩整个 rig（dsh 的 connection 面就是长连接）。
3. **反代必须转发 POST body**：否则页面内经 `/api` 打 dsh RPC 恒失败。
4. **`Runtime.evaluate` 用 `awaitPromise: true`**，且表达式**自己 `JSON.stringify`**——
   否则拿到 Promise/对象而不是值。
5. **探针必须断言"脚本确实执行过"**（如 `window.__dshDockImmersiveInjected`）：
   P0-a 那次三场景全绿却是 vacuous 的（脚本因平台门提前 return）。
6. **观测点要选"会变的东西"**：`[data-sidebar-collapsed]` 的**存在性**在未加载工作区时
   也为真，是错的观测点；应读会随状态变化的量（列宽 / 属性值）。

## 14. 时序实验（方案 D）的结论：**不可行** —— 2026-09-29 实测

### 14.1 为什么要试方案 D

维护者裁定 dsh-dock 的定位是 **web 档套壳**（见 §15），且**上游 issue 提不了** ⇒
原本的「推上游要回表现层专用标记」这条路断了。于是回头验一条此前只做过纸面推演的路：

> `data-platform` 的 4 个消费者**读取时机不同**——`dsh-client-shortcuts:721` 在**插件构造期**
> 读它决定 `runtime`（缺桥即抛错），而 `ui-layout` / `ui-sidebar` / `ui-primitives`
> 在**渲染期**读它做排版决策。若存在「构造已结束、渲染未开始」的窗口，就能只拿到表现层
> ——等于**用时序合成一个「表现层专用标记」**，不提供能力对象也不红屏。

### 14.2 做法

同一反代逐字节转发 dsh 0.2.0-rc.1 工作台，**唯一变量 = 打标记的时刻**。
双判据：① 启动是否干净；② **渲染期烘焙**的表现层信号是否拿到。

**观测点选型（这里踩过一次自己的坑，记下来）**：首版用
`--dsh-frame-top-clearance` 的计算值当信号——但那是 **CSS 变量、实时生效**，
任何时刻打标记它都会变成 48px，**根本区分不了「JS 布局是否读到 darwin」**。
改用 `[data-shell-leading]`：源码 `ui-layout:348` `leadingMounted && <div data-shell-leading>`，
而 `leadingMounted = darwin && sidebarCollapsed`（`:314`）——**只在渲染那一刻 darwin 为真**
才进 DOM，事后补标记长不出来。（工作区选择页侧栏为折叠态，故该信号无需工作区即可用。）

### 14.3 结果

| 变体 | 结果 |
|:--|:--|
| V1 `document-start` | ❌ 启动失败（打早了）——与 P0-a 的复现一致 |
| V2 `DOMContentLoaded` | ❌ 启动失败（打早了） |
| V3 `+1 rAF` | ❌ 启动失败（打早了） |
| V4 `+2 rAF` | ⚠️ **不稳定**：4 次里 2 次红屏、2 次干净 |
| V5 `+8 rAF` | ⚠️ 干净，但**表现层没拿到**（3/3） |
| V6 `window load` | ✅ 干净 + 表现层拿到（3/3） |

### 14.4 判定：**不可行**

两条独立理由，任一条即足以否决：

1. **V4 落在边界上**：4 次里 2 次红屏。一个"最优时刻"却有 ~50% 概率把用户工作台打成红屏，
   这不可接受——尤其它的失效方向是**最坏的那个**（启动失败），不是丢观感。
2. **没有可预测的模型**：V5（更早）拿不到表现层、V6（更晚）反而拿到了，两者**非单调**。
   也就是说我**无法解释**这些结果，因而无法为任何固定时刻给出可靠性论证。
   对一个失效方向是"工作台起不来"的机制，**"我不理解它为什么能工作"本身就是否决理由**。

对照组（完全不打标记）在既有所有批次里从未失败（P0-a/P0-c 多次运行均为干净基线），
故 V1–V3 的失败可归因于标记时机，不是 rig 噪声。
**残余不确定性（诚实边界）**：本轮最后一组「对照 × V4 交错」的补测因探针在 SSE 处挂起
而未跑完，故"V4 的不稳定是否 100% 由时序导致"没有拿到最后一块拼图。但**第 2 条理由
（非单调、无模型）不受此影响**，结论不变。

### 14.5 结论

**方案 A（放弃沉浸式观感）不是"退而求其次"，而是唯一答案**——不是不想留住，
是**物理上没有一个可靠的时刻**可以打那个标记。本档因此把 §6 的三层降级与 §13 的
P1 前置实验一并收束为决策史；**P1/P2/P3/P4 全部不做**。

将来若上游肯发布「表现层专用标记」或明确允许第三方壳设置 `data-platform`，
本档 §9 复审条件的第一条即触发重开——那时这条路才是诚实的，且不需要时序技巧。

## 15. 2026-09-29 维护者裁定：dsh-dock 的定位是 **web 档套壳**（本档范围的最终收束）

> **裁定原话**：「我感觉跑偏了，我只想做个 web 档的套壳。」
> **追加约束**：「dsh 提不了 issue，方案 A 不太好。」

### 15.1 这条裁定否掉了什么

本档 §3 方案 A（成为契约完整的桌面宿主）**整体作废**。理由不是技不如人，而是：

- **它不是 bug 修复，是产品立项**。「补全 `dshDesktop`」的真实含义是把壳做成官方 Electron
  客户端的等价物——连带 §1.6 的 7 个消费者、账户/引导面、`dshPlatform` 等一整套面。
- **键盘桥是在补偿一个本壳不存在的问题**（§3.1）：WKWebView 没有 Electron 的原生菜单截键，
  DOM 交付本来就完整正确。装这个桥只会把对的换成一次有损往返。
- **dsh-dock 的价值在管理面**（profile / 插件 / 凭据 / 会话 / 诊断），不在冒充官方客户端。

### 15.2 各项处置

| 项 | 处置 |
|:--|:--|
| **P1**（`dshDesktop`/`keyboard`/`shortcuts`/`dshOnboarding`） | ❌ 不做。四项待裁定（遥测 / region 旁路 / P4 / P2 同批）**随之全部失效** |
| **P2** 账户与引导面、**P3** 页面全局、**P4** browser | ❌ 不做 |
| **方案 D** 时序打标记（§14） | ❌ 不可行（实测：无可靠时刻） |
| **方案 C** 壳自注入 CSS 复刻 darwin | ❌ 不采纳（2 处 JS 布局复刻不了，且耦合比依赖标记更重） |
| **上游 issue** | ⛔ 维护者反馈提不了 ⇒ 这条路断开 |
| **降级提示**（§7 / §11.2） | ✅ **撤除**（见 §15.3） |
| **P0-a 版本闸 + 原子性闸门** | ✅ **保留**——它正是"诚实 web 套壳"的执行机制 |
| **复现台账复现点 25** | ✅ 保留（事实资产，与形态无关） |

### 15.3 为什么撤除降级提示（这是本轮唯一的行为回退）

P0-c① 的降级提示（工作台内一次性提示 + 状态点 + 打扰预算）是在「观感是被迫回退的」
前提下设计的——那时它是**告知**。定位定为 web 套壳之后，系统原生顶栏是**设计本身**，
什么都没有"回退"：再天天提示用户"外观回退了"就是**骚扰**，且是错的（没有回退）。

故：
- 注入层：`renderDegradedNotice` / `shouldShowHostNotice` / 打扰去重键 **整块删除**；
  顺带**收回**当时为它开的存储边界——沉浸式脚本现在**只读不写**（连 `sessionStorage`
  也不碰），由闸门正向钉住。
- 控制中心与关于页：从「safe ⇒ warn 色调 + 原因 + 可行动作」改为**中性事实陈述**
  （「当前形态：系统原生 / 沉浸式」+ dsh 版本 + 一句说明）。
- 两道**反向闸门**上线，防止将来有人"顺手加回来"（撤掉提示即红）。

### 15.4 现在的形态（一句话）

**dsh-dock = 以 Web 方式承载 dsh 工作台的原生窗口 + 一套 dsh 管理面**；
窗口用系统原生标题栏；`data-platform` 只在契约表判定安全时才打（当前 dsh 版本恒不打）。

### 15.5 与 ADR-0029 的关系（**须由维护者确认退役范围**）

ADR-0029（沉浸式标题栏）的**前提**——用官方标记唤醒 dsh 的桌面 CSS——在
「web 档套壳 + 无可靠打标记时机」之下不再成立。它的三层内容现状：

| ADR-0029 内容 | 现状 |
|:--|:--|
| §7 几何语义自驱拖拽（`immersive-chrome.js` 的拖拽部分） | 仍在代码里，但 `HOST_MODE !== 'desktop'` 时整脚本早退 ⇒ **当前不可达** |
| 窗口层 overlay + `hidden_title` + vibrancy + 红绿灯 AppKit 定位 | 仍在代码里，启动期默认生效，导航时由 `set_desktop_host_active(false)` 翻回原生 |
| §2 约束 2「失败必须静默降级」 | 已在 §8 被推翻（改为显式降级）→ 现又因 §15 整条作废 |

**待维护者裁定**：是否**物理删除**这三层（标记注入 + 窗口 overlay/红绿灯 + 拖拽自驱），
即 ADR-0029 正式退役。删掉即代码净减、与 dsh 私有内部彻底解耦；
保留则是一套当前永不执行的死代码（有闸门护着，但会误导后来人）。

**本档建议：删。** 但这是产品级取舍（万一将来上游放开标记，删了要重写），故不擅自决定。

## 16. 沉浸式栈物理删除（2026-09-29 维护者裁定「删」）

§15.5 提出的问题——ADR-0029 那三层是否物理删除——维护者裁定**删**。本节记录删除面。

### 16.1 删掉的东西

| 面 | 内容 |
|:--|:--|
| **注入层** | `frontend/src/injected/immersive-chrome.js`（整脚本）＋ 纯模型 `frontend/src/lib/immersiveChrome.ts` ＋ 其 vitest |
| **窗口层** | `ui.rs` 的 macOS `TitleBarStyle::Overlay` + `hidden_title(true)` + `Effect::Sidebar/Active`；`builder` 的 `mut`（原先只有那段要它）＋ 其 `cfg_attr(allow(unused_mut))` 兜底 |
| **红绿灯** | `src-tauri/src/traffic_lights.rs`（整模块，AppKit `setFrameOrigin` FFI）＋ `ui.rs` 的三处 `align` 接线 |
| **宿主契约机制** | `src-tauri/src/hostcontract.rs`（契约表 / `HostContractMode` / fragment 通道 / `HostContractView`） |
| **贯穿链** | `resolve::LaunchSpec.host_mode`＋计算、`Executor::host_mode()` trait 方法＋实现、`boot.rs` 的 `ShellState.host_mode`＋落账＋fragment 下发＋窗口形态切换 |
| **可查面** | IPC `get_host_contract`（`ipc.rs::COMMANDS` / `lib.rs` handler / `capabilities` 三处）＋ 登记册条目；`HostContractStatus.tsx` / `HostContractRow.tsx` ＋ 其测试 ＋ `hostContract` 字典组（zh/en）＋ `types/ipc.ts` 类型 ＋ `tauri.ts` api 方法 ＋ 两处页面接线 |
| **ACL** | `core:window:allow-start-dragging`、`core:window:allow-toggle-maximize`（只为自驱拖拽授的；已确认无其他使用者）、`allow-get-host-contract`。permissions 61 → 58 |

### 16.2 换来的东西：**一条反向闸门**

删掉机制之后，"保留版本闸"这个说法就失去对象了（没有标记可闸）。真正该留的是
**防复辟**——因为「打标记能让旧版 dsh 好看一点」这个诱惑会一直在。故
`ui.rs` 的 `windows_native_decorations_tests` 重写为 **`no_desktop_claim_tests`**：

```
ui_never_claims_desktop_host   —— 五条腿一并钉死，缺一条整套就不成立：
  title_bar_style(Overlay) / hidden_title(true) / Effect::Sidebar
  / traffic_lights:: / include_str!(immersive-chrome.js)
```

外加保留 ADR-0030 的 `windows_window_stays_decorated`（`decorations(false)` 不得生效）。
**负例已验证**：注入一段合法的 `w.title_bar_style(tauri::TitleBarStyle::Overlay)`
死代码后闸门即红，并给出「先改 §15/ADR-0029 并重跑 §14 时序实验」的指路。

另：`window_background_tests::macos_titlebar_hides_native_title_text` 的半句断言
（"注释承诺 ≠ 代码事实"那条）随对象消失，收窄为 `main_window_keeps_its_title`
——只保「窗口标题不得被顺手删掉」（它仍供窗口切换器/任务栏/关于弹窗使用）。

### 16.3 现在的形态（最终）

**dsh-dock = 原生窗口 + Web 方式承载 dsh 工作台 + 一套 dsh 管理面。**
不注入任何脚本到主窗口、不打 `data-platform`、不提供 `dshDesktop`、不做窗口 overlay。
shell 对 dsh 文档的唯一写入面**只剩三个既有注入**：`platform_script`（`__DSH_PLATFORM__`
供壳自己的前端用）、`link-hook`、`switcher`、`handoff-curtain`（后三者与本档无关）。

### 16.4 复审条件

若上游将来发布「表现层专用标记」或明确允许第三方壳设置 `data-platform`，
按 §9 第一条重开本档——那时**产出的是一套新机制**（不是恢复本次删掉的代码：
本次删掉的版本依赖 fragment 裁决与 fail-closed 门控，而那套门控的存在理由正是
"标记会被当运行时身份消费"这个前提）。ADR-0029 保留为决策史，不复活。
