# UI/UX 与布局评审记录（2026-09-08）

> 性质：**只读评审产出**，不是已批准的改造计划。所有结论带 file:line 或截图证据；
> 改造前按 AGENTS §8.1 拆增量、§9 判 ADR。
> 证据口径：① 代码（`frontend/src`，v0.9.6 HEAD `2f34e6b`）；
> ② 业主自报问题单 `docs/known-issues/问题记录.md`、`问题记录095.md`；
> ③ 截图 `docs/known-issues/image*.png`（v0.9.0–v0.9.5 期间，**仅用于布局与信息架构**，
> 单项缺陷一律以代码现状为准）。
> 实测基线：`pnpm typecheck` 0 err · `pnpm test` 112 passed · `oxlint` 0 warning
> （批次 A 落地后复测：**125 passed**，含 `shellSettings` 4 条 + `onNoticeStability` 2 条
> + `switchA11y` 2 条）。

## 0. 结论摘要（按严重度）

| # | 缺陷 | 严重度 | 状态 | 证据 |
|:--|:---|:---|:---|:---|
| U1 | **持久失败时无限重试 + 无限弹 toast**（5 个面板） | BLOCKER | ✅ 已修 | `ProfileManager.tsx:263` 内联 `onNotice` + `SessionManager.tsx:125-141` 等 |
| U2 | **Profile 卡片键盘不可达**（主操作） | BLOCKER | ✅ 已修 | `ProfileRow.tsx:63` |
| U3 | **诊断页失败时伪造「环境全缺失」报告** | HIGH | ✅ 已修 | `DiagnosticsPane.tsx:63,92-105` |
| U4 | **偏好设置读取失败后把默认值写回**，可能清空其他键 | HIGH | ✅ 已修 | `PreferencesPane.tsx:36,54,72,90` + `i18nStore.ts:54` |
| U5 | **配置迁移失败却提示「分发完成」** | HIGH | ✅ 已修 | `PluginOverview.tsx:165-167,170` |
| U6 | **对比度系统性不达标**（`faint` 2.1–2.4；白字/品牌蓝 4.23） | HIGH | ⬜ 待做 | `index.css:36` + 260 处 `text-faint` |
| U7 | **字号未走 token**：161 处任意值，主体 9–11px | HIGH | ✅ 已修 | `text-[10px]`×90、`text-[11px]`×72 |
| U8 | **窗口最小宽度 < 布局断点**，主从布局在允许的窗口尺寸下静默塌成单列 | HIGH | ✅ 已修 | `lib.rs:3012` (860) vs `lg`=1024 |
| U9 | **破坏性操作无确认**：卸载插件、覆写凭据、覆写引擎设置 | HIGH | ⬜ 待做 | `ProfileDetailPane.tsx:692-700` 等 |
| U10 | 页面头部不吸顶，长列表滚动后视图切换入口消失 | MEDIUM | ✅ 已修 | 全仓 `sticky` 0 处 |
| U11 | toast 无 `aria-live`；11 处「已复制」假成功 | MEDIUM | ✅ 已修（live region + clipboard 收口） | `ui/toast.tsx:11-61`、`lib/clipboard.ts` |
| U12 | `dark:` 变体不可达（40 处死代码），掩盖对比度问题 | MEDIUM | ⬜ 待做 | `index.css:8` 无 `.dark` 应用点 |
| U13 | 图标语义错配 3 处 + 文案动词漂移（7 种「刷新」） | MEDIUM | ⬜ 待做 | 见 §6 |
| U14 | 会话行状态重复展示（左徽标 + 右胶囊同显「运行中」） | MEDIUM | ✅ 已修 | `SessionManager.tsx:344-360` vs `:477-489` |
| U15 | 截断无 tooltip 系统性缺口（Select 尤其） | MEDIUM | ⬜ 待做 | `PluginOverview.tsx:197`、`select.tsx:30` |

## 1. 布局骨架与信息架构

### 1.1 导航范式不统一（顶层横向 / 二级左侧）
顶层四个视图（Profile 列表 / 插件中心 / 会话维护 / 系统控制台）是页头横向分段控件
（`ProfileManager.tsx:170-232`）；进入系统控制台后，二级导航已改为左侧 Master-Nav
（`SystemConsole.tsx:66-116`，`lg:col-span-4 xl:col-span-3`）——**这是业主 §问题记录#10
建议的落地**，方向正确。但由此产生三层嵌套的范式混用：
横向 tabs（一级）→ 左侧栏（二级）→ 面板内 tabs（三级，如
`ProfileDetailPane.tsx:363-434` 插件/内置/补丁/MCP）。同一个窗口里出现三种导航形态。

**建议**：把一级导航也收进左侧栏（单一 Master-Nav），面板内 tabs 保留为内容分区；
或维持横向一级，但让二级与三级统一为「面板内 tab」，避免「左栏 + 横向 tab」并存。
属信息架构变更，建议先出方案再动。

### 1.2 纵向 chrome 过厚
页面自上而下堆了：页头（Emblem + 标题 + 副标题 + 一级 tabs）→ 面板头（标题 + 副标题 +
二级 tabs）→ 内容。截图 `image-3.png` 中三段头部占据约 150 CSS px 后内容才开始。
`SystemConsole` 改左栏后已缓解；`PluginHub`（`PluginHub.tsx:22-53`）仍是「面板头 + 横向子 tab」。

### 1.3 主从布局断点与窗口最小宽度冲突 ★
- `ProfileManager.tsx:271` `lg:grid-cols-12`，左 `lg:col-span-4`、右 `lg:col-span-8`。
- Tailwind v4 `lg` = 1024px；`profiles` 窗口 `inner_size(1180,780)`、**`min_inner_size(860,600)`**
  （`lib.rs:3011-3012`）；`main` 窗口 `min_inner_size(960,640)`（`lib.rs:1984`）。
- 结论：窗口宽度落在 860–1023（或主窗口 960–1023）时，`lg:` 不生效，主从布局塌成
  单列——详情面板被推到列表下方，而窗口高度只有 600–640，用户几乎看不到详情。

**建议**（二选一，都很小）：① `min_inner_size` 提到 ≥1024（保证双栏恒成立）；
② 把断点从 `lg` 降到 `md`（768）并压缩左右比例。**不建议同时改**。

### 1.4 页面头部不吸顶
全仓 `sticky` 出现 0 次。长列表（插件总览、会话维护、运行日志）滚动后，一级视图
切换入口滚出视口，用户必须滚回顶部才能换视图。
**建议**：`ProfileManager` 的页头加 `sticky top-0 z-20 bg-bg/95 backdrop-blur`。

### 1.5 空态与高度
- 详情区未选中态是设计过的（`ProfileDetailPane.tsx:308-320`）✓。
- `ProfileManager.tsx:307-333`：`loading` 期间列表区**什么都不渲染**（`loading` 只抑制
  空态，无骨架）→ 切窗口回来会闪空白。
- `min-h-[560px]`/`min-h-[540px]`/`min-h-[460px]` 三处硬编码高度（`ProfileManager.tsx:340`、
  `ProfileDetailPane.tsx:322`、`ProfileDetailPane.tsx:309`）。

## 2. 排版与视觉层级

### 2.1 字号未走 token ★
| 写法 | 出现次数 |
|:---|---:|
| `text-[10px]` | 90 |
| `text-[11px]` | 72 |
| `text-[9px]` | 6 |
| `text-[13px]` | 3 |
| `text-[15px]` | 1 |
| `text-xs`（12px） | 282 |

即：主体信息大量使用 **9–11px**，且是绕过 token 的任意值。桌面端 10px 正文低于
可读性下限，也低于任何无障碍指南。`@theme` 里没有字号 token，因此「统一字号」目前
无单一事实源。

**建议**：在 `index.css` 的 `@theme` 增加 `--text-micro: 11px` / `--text-caption: 12px` /
`--text-body: 13px` 三档，按文件批量替换（一次一个模块，符合 §8.1），并把 9px 一档删除。

### 2.2 对比度系统性不达标 ★
实测（WCAG 2.x，sRGB）：

| 组合 | 比值 | 结论 |
|:---|---:|:---|
| `--color-faint #a0a7b6` on `bg #f7f8fb` | **2.27** | FAIL（260 处 `text-faint`） |
| `faint` on `panel #ffffff` | **2.41** | FAIL |
| `faint` on `line-soft #eef1f6` | **2.13** | FAIL |
| 白字 on `--color-brand #4176e6`（主按钮） | **4.23** | FAIL（AA 需 4.5） |
| `brand` on white（链接/强调文字） | **4.23** | FAIL |
| `--color-warn #d9480f` on white | **4.30** | FAIL |
| `--color-ok #2f9e44` on white | **3.45** | FAIL |
| `--color-dim #626a7a` on bg / panel | 5.12 / 5.44 | PASS |
| `--color-brand-deep #3163cf` on white | 5.50 | PASS |
| `--color-ink` on bg / panel | 15.87 / 16.85 | PASS |

原始调色板（未走 token）另有一批失败：`amber-500` on white 2.15、`amber-600/70` 1.88、
`slate-600` on `slate-950` 2.66、`emerald-600` on `emerald-500/10` 3.43、`rose-500` 3.67、
`sky-600` on `sky-500/4%` 3.92、`rose-600` on `rose-500/10` 4.13。涉及 24 个文件，最密集
的是 `SessionManager.tsx`、`LogViewerPane.tsx:212-264`、`MarketplaceView.tsx`、`McpManager.tsx`。

**建议**（最小改动，不动版式）：
1. `--color-faint` 仅用于**装饰性**图标；正文/次要文字改用 `--color-dim`（已达标）。
   若需保留更浅一档，新增 `--color-faint-text: #6b7280` 一类达标值。
2. 主按钮底色改用 `--color-brand-deep`（白字 5.50 达标），hover 再深一档；
   链接文字同理。品牌蓝 #4176e6 仅保留给填充色块与描边。
3. 原始调色板 → token：把上表失败项逐文件替换（一次一个模块）。

### 2.3 `dark:` 变体不可达 ★
`index.css:8` 定义了 `@custom-variant dark`，但**全仓没有任何地方给根元素加 `.dark`**
（grep 确认：仅该行出现 `.dark`）。而 8 个文件、40 处写了 `dark:` 变体
（如 `text-emerald-600 dark:text-emerald-400`）。
后果：① 暗色模式的「兜底」是死代码；② 更隐蔽——**浅色值恰恰是不达标的那个**
（`emerald-600` on 浅底 3.43），而达标的 `dark:` 值永远不会生效。
**建议**：要么按 `index.css:6-7` 的预留落地 `.dark` 变量覆盖，要么删除这 40 处
`dark:` 并让浅色值达标。**不建议两者都不做**——现状会让对比度审计得出错误结论。

### 2.4 间距与圆角
间距已形成事实上的 2px 栅格（`gap-1/1.5/2/2.5/3`、`px-1.5/2/2.5/3`、`py-0.5/1.5/2`），
一致性良好。圆角混用 `rounded-lg/xl/2xl/full` + 5 处任意值，层级语义尚可推断，不列为缺陷。

## 3. 交互状态与反馈

### 3.1 无限重试循环 ★★（最高优先）
机制：`ProfileManager.tsx:263` 把 `onNotice` 以**内联箭头函数**传给子面板 → 每次渲染
新引用 → 子面板 `useCallback([onNotice])` → `useEffect([...])` 重跑 → 失败又 `onNotice`
→ `setToast` → 父组件重渲染 → 回到第一步。

已验证的受影响面板（模式完全相同）：
| 面板 | 内联传参 | 依赖链 |
|:---|:---|:---|
| `SessionManager` | `ProfileManager.tsx:263` | `:125-137` → `:139-141` |
| `CredentialsPane` | `ProfileManager.tsx:266` | `:42-56` → `:58-60` |
| `DshSettingsPane` | 同上 | `:37,41` |
| `DiagnosticsPane` | 同上 | `:66,70-72` |
| `LogViewerPane` | 同上 | `:48,53` |

`McpManager.tsx:107-110` 已经记录过这个坑并单独修好——**同一根因仍存在于其余 5 个面板**。
**建议**：在 `ProfileManager` 用 `useCallback` 包住 `onNotice`（一处修复，五个面板同时解除），
并加一条注释说明为什么不能内联。

### 3.2 静默失败（用户看到的与事实相反）
| 位置 | 失败时用户看到 | 后果 |
|:---|:---|:---|
| `DiagnosticsPane.tsx:63,92-105` | 「Node/pnpm/dsh 未检出、缺失」 | **伪造环境损坏报告** |
| `PreferencesPane.tsx:36` | 默认值界面（胶囊默认开） | 下一次开关写回 `{}`，**可能清空其他设置键** |
| `PluginOverview.tsx:165-167,170` | 「分发完成」 | 配置其实没迁移 |
| `PluginOverview.tsx:73` + `:455-458` | 「所有已物化 Profile 均已安装」 | 因 profile 列表为空而误判 |
| `ProfileDetailPane.tsx:110,114,118` | 「无额外依赖」+ 无开关 | 看起来像「没装插件」 |
| `MarketplaceView.tsx:74-75,85-87` | 已安装徽标消失 | 用户可能重复安装 |
| `LogViewerPane.tsx:45,232-235` | 「暂无日志内容」 | 与「日志为空」不可区分 |
| `CredentialsPane.tsx:49-53,178-179` | 空网格 | 与「没有凭据」不可区分 |
| `About.tsx:50,142` | 「工作台未就绪」/点击无反应 | — |

**建议**：统一引入 `LoadState<T> = { status: "loading"|"ok"|"error"; data?: T; error?: string }`，
失败一律渲染带重试按钮的错误块——**绝不把失败降级成空数据**。这是本次评审里
用户可见影响最大的一类问题。

### 3.3 破坏性操作确认缺口
| 操作 | 位置 | 现状 |
|:---|:---|:---|
| 卸载插件 | `ProfileDetailPane.tsx:692-700` | **无确认、无撤销** |
| 覆写 `.credentials.yaml` | `CredentialsPane.tsx:87-98` | **无确认、无备份** |
| 覆写 `settings.yaml` | `DshSettingsPane.tsx:43-54` | **无确认、无 diff、无备份** |
| 删除会话 / MCP / 凭据 | `SessionManager.tsx:236-241` 等 | 原生 `window.confirm`（风格不统一） |
| 删除 Profile | `ProfileDeleteDialog.tsx:44-56` | ✓ 模态 + 风险清单（标杆） |

**建议**：以 `ProfileDeleteDialog` 为模板统一；两个文件覆写加「先备份到 `*.bak-<时间戳>`」，
与本次配置事故（`settings.yaml` 被写残）同源风险。

### 3.4 「已复制」假成功（11 处）—— ✅ 已修（架构批次 0b，2026-09-08）
`copied` 在 Promise settle 之前就置位，拒绝被忽略：`ErrorCard.tsx:65-70`（连 `.catch` 都没有）、
`MarketPluginCard.tsx:46-53`、`BootStep.tsx:32-37`、`ProfileDetailPane.tsx:273-280`、
`DiagnosticsPane.tsx:76-81`、`LogViewerPane.tsx:62-70`、`DshSettingsPane.tsx:56-61`、
`SessionManager.tsx:212-217`（另有 `:219-225` 复制路径，本表漏记）、`McpManager.tsx:239-244`、
`About.tsx:56-70`。
**更正**：原文写「`BootStep.tsx:34` 是唯一正确写法（`.catch`）」——不成立，它
`.catch(() => {})` 吞掉后**照样立即置位**，与其余各处的「假成功」同病。
**修法**：抽 `lib/clipboard.ts::writeClipboard` + `hooks/useCopy`，11 处一次收口；
`clipboardGate.test.ts` 源码闸门禁止再绕过。

### 3.5 toast
- 两个独立实例：`ProfileManager.tsx:58-63`（3500ms）与 `About.tsx:30-35`（3000ms）。
- 单槽：第二条消息直接替换第一条。
- 消息 `truncate` 到 420px 且**无 `title`**（`ui/toast.tsx:47`）——长错误看不全。
- **无 `role`/`aria-live`**（`ui/toast.tsx:11-61`）→ 读屏用户收不到任何确认/错误。

## 4. 可访问性

| 项 | 结论 | 状态 |
|:---|:---|:---|
| 键盘不可达 | `ProfileRow.tsx:63` 整卡 `div onClick`（**主操作**）；`MarketPluginCard.tsx:66` `<h3 onClick>`；`SessionManager.tsx:757` 分组折叠头 | ⚠️ ProfileRow 已修；余 2 处待做 |
| 图标按钮无名称 | `McpManager.tsx:348-360`（删除 MCP，**无任何名称**）、`MarketplaceView.tsx:220-227`（清空搜索）、`McpManager.tsx:544-553`（删 ENV 行） | ✅ 已修 |
| 仅靠 `title` 命名 | 15 处（`ProfileRow.tsx:129` 等）——tooltip 不保证被 AT 读出 | ⬜ 待做 |
| 未命名 Switch | 7 处：`PreferencesPane.tsx:238,311`、`McpManager.tsx:562`、`LogViewerPane.tsx:144`、`BuildApprovalDialog.tsx:106`、`PluginOverview.tsx:493`、`ProfileDetailPane.tsx:674`（仅 `BootMode.tsx:127` 正确） | ✅ 已修（8 处全带名称 + 类型闸门） |
| 表单标签 | 11 处 placeholder-only 输入框；`McpManager.tsx:468,481,492,507` 的 `<label>` 与控件无 `htmlFor` 关联；`MarketInstallDialog.tsx:159,198` 同 | ✅ 已修（批次 B2，含源码闸门） |
| 标题层级 | 每个面板从 `h3` 起（跳过 `h2`）；`ProfileDetailPane.tsx:312` h3 先于 `:327` h2 | ✅ 已修（批次 B2） |
| tab 语义 | 4 处 `role="tablist"` 无 roving tabindex / `aria-controls`；`SystemConsole.tsx:75-105` 的 `role="tab"` 落在裸 `<nav>` 里 | ⬜ 待做 |
| 减少动效 | `index.css` 无 `prefers-reduced-motion`；Framer Motion 9 个文件未用 `useReducedMotion`；仅 2 处 `motion-reduce:animate-none` | ⬜ 待做 |
| 命中区 | <24px：toast 关闭 18×18、`MarketPluginCard` 3 个 22×22、`UpdateBanner` 22×22、MCP 删行 ≈16×16、`Switch` 16×28（高度不足） | ⬜ 待做 |
| 对比度 | 见 §2.2 | ⬜ 待做 |
| toast 播报 | `ui/toast.tsx` 无 `role`/`aria-live` → 读屏收不到任何确认/错误 | ✅ 已修（常驻 live region） |

## 5. 图标与文案一致性

**图标错配**（业主两次反馈过同类问题）：
| 位置 | 图标 → 动作 | 问题 |
|:---|:---|:---|
| `ProfileManager.tsx:247` | `DownloadCloud` → 检查更新 | 下载语义用在「检查」上 |
| `MarketPluginCard.tsx:161` | `Code2` → 打开 GitHub | 代码语义用在「外链」上 |
| `LogViewerPane.tsx:172` | `Trash2` → 清屏（非破坏性） | 删除语义用在只清视图的操作上 |
| `SessionManager.tsx:407` vs `:519` | `Clipboard` 复制 ID vs `Copy` 复制路径 | 同一动作两套图标 |

**文案动词漂移**（同一动作 7 种说法）：
刷新扫描（`SessionManager.tsx:582`）· 刷新体检（`DiagnosticsPane.tsx:153`）· 刷新市场
（`MarketplaceView.tsx:277`）· 拉取最新日志（`LogViewerPane.tsx:178`）· 重新加载
（`DshSettingsPane.tsx:101`、`MarketplaceView.tsx:364`）· 重新读取凭据
（`CredentialsPane.tsx:133`）· 重试（`ProfileManager.tsx:302`、`McpManager.tsx:280`）。
复制动作 5 种标签：复制 YAML / 复制代码 / 复制全部日志 / 复制完整诊断报告 / 复制日志。

**i18n 泄漏**：25 个领域组件共 910 个中文字符（`PluginOverview` 136、`McpManager` 122、
`ProfileDetailPane` 103…），en-US 用户看到中文。另有 8 处**中文 `aria-label`**
（`ProfileRow.tsx:162`、`toast.tsx:51`、`PulseBar.tsx:8`、`ui/dialog.tsx:79` 等）——读屏
在英文环境下读中文。硬编码文案例：`ProfileManager.tsx:330`「未找到匹配的 Profile」、
`PluginOverview.tsx:204-205`「全部 Profile / Profile: …」。

## 6. 长文本与截断

- 大多数截断已配 `title=` ✓（`ProfileRow.tsx:84-90`、`ProfileDetailPane.tsx:599-608` 等）。
- **系统性缺口**：`ui/select.tsx:30` 强制 `[&>span]:truncate` 但 trigger 无 `title`，
  `SelectItem` 亦无 `title` → **下拉里的选项本身也读不全**；
  `PluginOverview.tsx:197` 把筛选 Select 钉在 `w-48`（192px）→ 业主「只显示 3 个字符」
  这一类问题**仍可复现**，只是当前 profile 名较短而已。
- 其它无 tooltip 的截断：`McpManager.tsx:367,444-446`、`SessionManager.tsx:431,475,767-769`、
  `MarketPluginCard.tsx:79-81,119-121`、`LogViewerPane.tsx:217-219`、`ui/toast.tsx:47`、
  `ProfileManager.tsx:160,163`。

## 7. 会话行：状态重复展示
`SessionManager.tsx:344-360` 在首行左侧渲染状态徽标（运行中/健康/需修复），
`:477-489` 又在右侧操作区渲染同义胶囊（含旋转图标）。同一行同一状态出现两次，
且右侧那枚占据操作位（挤压修复/删除按钮）。
**建议**：右侧只保留动作，状态合并进左侧徽标（或把右侧胶囊改成动作按钮）。

## 8. 业主问题单核对（现状）

| 问题单条目 | 现状 |
|:---|:---|
| #1 新建 Profile 按钮位置 | ✅ 已改为左栏顶部通栏主按钮（`ProfileManager.tsx:277-284`） |
| #2 重置依赖语义不清 | ✅ 功能已下线（v0.9.1） |
| #3 Select 只显示 3 字符 | ⚠️ **半修复**：`PluginOverview.tsx:197` `w-48` + 无 `title` 仍可复现 |
| #5 按项目分组：项目名带路径 | ✅ 现为目录名（`image-10.png` 佐证） |
| #5 分组行「展示不一致」（按钮换行） | ⚠️ 长路径下仍会换行（`image.png`）——缺 `min-w-0` + 固定操作列 |
| #5 图标乱用（目录 icon 做复制） | ⚠️ 部分修复，仍有 3 处错配（§5） |
| #7 监控大盘需缓存 | ⚠️ 仅 60s 模块级缓存（`DiagnosticsPane.tsx:27-29`），切面板仍闪 loading |
| #10 两层导航改左侧 | ✅ 系统控制台已改左栏；一级仍为横向（§1.1） |
| 095#1/#2 MCP 死循环 + 刷新按钮语义化 | ✅ 已修（`530fd91`） |
| 095#4 插件市场后台排队下载 | ❌ 未实现 |
| 095#5 会话状态误判 | ✅ 已修（`e67f9f4`、`7473f2a`） |

## 9. 建议批次

| 批次 | 内容 | 量级 | 状态 |
|:---|:---|:---|:---|
| **A（止血）** | U1 引用稳定性；U3 诊断失败不再伪造报告；U4 设置读取失败禁写；U5 配置迁移结果如实通知 | 极小，1 PR | ✅ 已落地（见 §12） |
| **B（可见性）** | U2 卡片键盘可达；toast 常驻 live region；8 个 Switch 补名称（+类型闸门）；4 个图标按钮补名称 | 小 | ✅ B1 已落地（见 §13） |
| **B2（表单与标题）** | 12 处输入框补名称/关联（+源码闸门）；面板标题层级 h3→h2 并修正子标题 | 小 | ✅ 已落地（见 §14） |
| **C（视觉达标）** | U6 对比度三改（`faint`→仅装饰、主按钮用 `brand-deep`、原始调色板逐文件替换）；U12 决定 `.dark` 去留 | 中，逐模块 | ⬜ 待做 |
| **D（版式）** | U7 字号 token 化；U8 断点/最小宽度对齐；U10 页头吸顶；U14 会话行去重 | 中 | ✅ 已落地（见 §15） |
| **E（一致性）** | U13 图标与动词统一；U15 Select tooltip 补全；i18n 泄漏逐文件收口 | 中，机械但量大 | ⬜ 待做 |

## 10. 不建议做
- 不引入组件库/设计系统重写：现有 shadcn + token 骨架可用，问题集中在**取值**与**状态处理**。
- 不引入 jsdom/RTL 去补组件测试（§5 明确禁止）；本表 U1/U3/U4 一类缺陷应通过
  「纯函数 + 状态机」下沉到 `lib/` 后测，而不是渲染测试。
- 不在同一 PR 里同时改断点与窗口尺寸（U8），否则回归面无法界定。

## 11. 补充发现与 ADR-0011 对账（2026-09-08 16:2x 复核）

### 11.1 本表遗漏的一条：弹窗溢出（已由 ADR-0011 立项）
`ui/dialog.tsx:63-66` 的 `DialogContent` 基类**无 `max-h`、无 `overflow`**；`fixed top-1/2
-translate-y-1/2` 居中定位下，内容或视口不足时弹窗上下两端溢出且不可滚动救援。
全站弹窗共用此基类 → 影响面全站。ADR-0011 §1.3 已记录并立项。

### 11.2 归 ADR-0011 收口、本表不再双源（AGENTS §11.3）
| 本表条目 | 收口位置 |
|:---|:---|
| 弹窗溢出 | ADR-0011 §4「弹窗基类补高度约束与滚动」 |
| 删除死代码 `ProfileDetailDialog.tsx` | ADR-0011 §5 行动项「术语与死代码」 |
| 动词统一「安装到…」、Tab 名改「插件」 | ADR-0011 §5 同上（与本表 U13 同源） |
| 插件市场后台排队下载 | ADR-0011 §5 行动项「跨 profile 更新检查 + 批量更新」 |

### 11.3 ADR-0011 行动项：复核时未落地 → **已于 `413570d` 落地**
ADR-0011 §5 三项标记 `[x]`（2026-09-08 hotfix），本评审 16:2x 复核时工作树中查无对应改动。
经维护者确认（2026-09-08）：当时**正由另一 agent 实现中**，故标记先于代码属进行中状态，
非改动丢失。**当日稍后已合入 `413570d`，三项均已落地**（下表「复核时现状」保留为历史记录）。

| ADR-0011 行动项 | 标记 | 复核时（16:2x）代码现状 | 现状（413570d 后） |
|:---|:---|:---|:---|
| Rust `validate_install_spec` 三形态 + 恶意反例测试 | `[x]` | 未落地：`plugins.rs:346` 仅 `validate_plugin_spec` | ✅ `plugins.rs:371` 落地，`install_plugin` `:464` 调用 |
| 前端 `validatePluginSpec` 镜像三形态 | `[x]` | 未落地：`lib/profiles.ts` 仍严格 npm 正则 | ✅ `lib/profiles.ts` + `__tests__/profiles.test.ts` 扩测 |
| `DialogContent` 基类 `max-h-[calc(100dvh-2rem)] overflow-y-auto` | `[x]` | 未落地：`ui/dialog.tsx:63-66` 无 `max-h`/`overflow` | ✅ `ui/dialog.tsx:66` 落地 |

复核口径（当时）：`git status`（工作树无前端/Rust 改动）· `git branch -a` + `git stash list` +
`git worktree list`（无其它分支/暂存/工作树）· `git log --all` 最新为 `2f34e6b`。

**对后续工作的约束**：ADR-0011 实施期间**不得触碰**其文件面（见 §11.4），
否则与在途改动冲突。本评审的批次 A（U1/U3/U4/U5）大部分落在
`ProfileManager` / `SessionManager` / `DiagnosticsPane` / `PreferencesPane`，
与 ADR-0011 文件面基本不相交；**但 `PluginOverview.tsx` 相交**（U5 所在），
须待 ADR-0011 合入后再动。

### 11.4 ADR-0011 在途文件面（占位，避免并行冲突）——**已释放**
`src-tauri/src/plugins.rs` · `frontend/src/lib/profiles.ts` · `frontend/src/lib/market.ts` ·
`frontend/src/components/ui/dialog.tsx` · `frontend/src/components/profiles/ProfileDetailDialog.tsx`（待删）·
`frontend/src/components/market/*` · `frontend/src/components/profiles/ProfileDetailPane.tsx`
（术语/Tab 名）。以 ADR-0011 §5 行动项为准。

**2026-09-08 收口**：ADR-0011 已合入 `413570d`（§11.3 三项标记均已落地，前置条件
`validate_install_spec` 存在 + `dialog.tsx` 带 `max-h-[calc(100dvh-2rem)] overflow-y-auto`
双条验证通过），上述文件面占用释放。

## 12. 落地记录：批次 A（2026-09-08，`fix(uiux): 静默失败与引用不稳止血`）

| 条目 | 改动 | 文件 |
|:---|:---|:---|
| U1 | 三处内联箭头改传稳定引用 `showToast`；加裁定注释说明为何禁内联 | `pages/ProfileManager.tsx:255-267` |
| U3 | 采集失败且无缓存 → 报错块 + 重试；删除 `report ?? {全缺失}` 伪造分支 | `components/system/DiagnosticsPane.tsx` |
| U4 | 抽 `lib/shellSettings.ts` 安全基线（读失败即中止，绝不回退 `{}`）；`PreferencesPane` 失败即停用保存并渲染重试块；`i18nStore.setLocale` 同根因收口 | `lib/shellSettings.ts`（新）· `components/system/PreferencesPane.tsx` · `stores/i18nStore.ts` |
| U5 | 迁移失败/未覆盖不再被「分发完成」掩盖，按实际结果通知 | `components/profiles/PluginOverview.tsx:157-193` |

**测试（复现先行）**
- `__tests__/shellSettings.test.ts`（4 条）：读失败不写盘、只改 patch 键、`null` 显式落盘、写失败抛出。
  把基线改回 `read().catch(() => ({}))` 后首条即红（已验证）。
- `__tests__/onNoticeStability.test.ts`（2 条）：源码文本闸门（`?raw` 读入，不引 DOM/fs）。
  对修复前的 HEAD 复算命中 `:258,263,268`，修复后 0 命中（已验证）。
  该缺陷对 tsc/oxlint 不可见（类型一致），故用与 `ipc.rs` gate_tests 同口径的源码闸门兜底。

**未纳入本次（同源但独立）**
- `setLocale` 存储失败仍返回成功 → `PreferencesPane.handleLanguageChange` 会弹「设置已保存」
  （假成功 toast，§3.4 一类）；本次只保证「不再写坏其他键」，通知语义留待批次 B。
- `set_shell_settings` 仍是整体覆盖写（Rust 侧无 merge 语义）：前端已加安全基线，
  但任何新调用点仍可能踩坑，建议后续在 `settings.rs` 侧补 merge 或收窄命令面。

## 13. 落地记录：批次 B（2026-09-08，`fix(a11y): 键盘与读屏可达性`）

| 条目 | 改动 | 文件 |
|:---|:---|:---|
| U2 | 卡片不可整体改 `<button>`（内含启动/更多菜单按钮，嵌套交互元素非法）；改为**名字即主控件**：名字变 `<button>` + `aria-current`，卡片用 `focus-within:ring-2` 呈现整卡焦点环，指针点击行为不变 | `components/profiles/ProfileRow.tsx` |
| toast 播报 | `role="status" aria-live="polite" aria-atomic` 挂在**常驻**容器上（挂在随 toast 挂载的节点上部分读屏会整条漏播）；消息 `truncate` 补 `title` | `components/ui/toast.tsx` |
| Switch 名称 | 8 处全部带 `aria-label`（含 `BootMode`，其 `<label>` 包裹仍保留）；文案取自相邻可见标签 | 6 个文件 |
| 图标按钮名称 | `McpManager` 删 MCP（`删除：<name>`）、删 ENV 行、`MarketplaceView` 清空搜索、`ProfileRow` 重启（原仅 `title`） | 3 个文件 |

**机器闸门（新增，替代「靠人记得加」）**
- `ui/switch.tsx` 的 `SwitchProps` 改为**类型层强制**：必须带 `aria-label` 或 `aria-labelledby`，
  否则 `pnpm typecheck` 红。8 处漏写一次性全被点出（已实测）。
- `__tests__/switchA11y.test.tsx`（2 条）：用 `@ts-expect-error` 钉住闸门存在性——把 `SwitchProps`
  退回全可选后立即报 `TS2578 Unused '@ts-expect-error' directive`（已验证）。
- 类型闸门优于源码文本闸门：不误判、不依赖文件路径、覆盖未来所有新调用点。

**未纳入本次（批次 B2）**
- 11 处 placeholder-only 输入框补 label；`McpManager.tsx:468,481,492,507`、
  `MarketInstallDialog.tsx:159,198` 的 `<label>` 无 `htmlFor` 关联。
- 标题层级（面板一律 `h3` 起、跳过 `h2`）、tab roving tabindex / `aria-controls`、
  `prefers-reduced-motion`、命中区 <24px、「仅靠 `title` 命名」15 处。
- 本次新增的 `aria-label` 中有 3 条沿用**相邻可见文案的硬编码中文**（`PluginOverview`
  「连带复制配置行」、`McpManager`「停用该 MCP 服务」、`MarketplaceView`「清空搜索」）——
  刻意与可见文案逐字一致（WCAG 2.5.3 Label in Name），随批次 E 的 i18n 收口一起改。

**实机验证清单（本次未跑：Tauri 壳需真实窗口，DOM 测试栈按 §5 禁用）**
1. `Tab` 进 Profile 列表 → 焦点落在名字按钮、整卡显示焦点环 → `Enter` 切换详情。
2. `Tab` 到启动/重启/更多菜单 → 各自可触发，焦点环不遮内容。
3. 触发一次失败 toast（如断网刷新诊断）→ 读屏应播报整条消息。
4. 键盘走查 MCP 删除、市场清空搜索、日志自动滚底开关。


## 14. 落地记录：批次 B2（2026-09-08，`fix(a11y): 表单标签与标题层级`）

**表单标签（12 处）**

| 类型 | 处 | 修法 |
|:---|:---|:---|
| 只有 placeholder 当标签 | 8 处搜索框（`ProfileManager`/`SessionManager`/`LogViewerPane`/`MarketplaceView`/`PluginOverview`/`ProfileDetailPane` 搜索与安装 spec）+ `CredentialsPane` 的 `sk-...` | `aria-label`（搜索框用同一 i18n 键；凭据框新增 `console.keyInputLabel`） |
| `<label>` 与控件无关联 | `McpManager` 服务器名/命令/参数 ×3 | `htmlFor` + `id`（`mcp-form-*`） |
| `<label>` 指向非控件 | `McpManager` ENV 组标题、`MarketInstallDialog` 安装源只读框 | 改 `<span>`；ENV 容器加 `role="group"` + `aria-label` |
| `<label>` 指向 Radix Select | `MarketInstallDialog` 目标 Profile、`PluginOverview` 分发目标 | `SelectTrigger` 加 `aria-label` |

ENV 行的 KEY/VALUE 输入框另加 `aria-label={`${t.profiles.mcpEnv} ${idx+1} KEY`}`（多行可区分）。

**标题层级**：页面 `h1` → 面板/节 `h2` → 卡片/子块 `h3`。
- 面板标题 `h3`→`h2`：`CredentialsPane`·`DshSettingsPane`·`DiagnosticsPane`·`PreferencesPane`×4·`SessionManager`·`McpManager`·`ProfileDetailPane`（空态）·`ClientUpdateCard`。
- 子块 `h4`→`h3`：`DiagnosticsPane` 5 个指标卡（否则 `h2` 后跳到 `h4`）。
- `MarketplaceView`/`PluginOverview` 的卡片标题是 `h3` 但无父级 `h2`（视觉标题由 Tab 承担）→ 补 `sr-only` `h2`。
- 评审原判「`ProfileDetailPane.tsx:312` h3 先于 `:327` h2」实为**空态与选中态两个互斥分支**，不存在渲染顺序问题；仅把空态标题对齐到 `h2`。

**机器闸门**：`__tests__/formLabels.test.tsx`（`import.meta.glob(?raw)` 扫全量 `.tsx`）
断言每个 `<input>` 至少有 `aria-label`/`aria-labelledby`/`id`/`<label>` 包裹/`type="hidden"` 之一；
探针文件实测被抓出。正则把 `=>` 当整体跳过，避免 `onChange` 的箭头把标签截断。

## 15. 落地记录：批次 D（2026-09-08，`refactor(uiux): 字号刻度、断点对齐、页头吸顶、会话行去重`）

| 条目 | 改动 | 文件 |
|:---|:---|:---|
| U7 | `@theme` 新增 5 档字号 token（`--text-micro/meta/label/note/lead` = 9/10/11/13/15px，**逐像素等同改造前**）；161 处 `text-[Npx]` 全量替换；`ui/button.tsx` 的 `text-[0.8rem]`→`text-xs`（12.8→12px，sm 按钮，视觉无感） | `index.css` + 38 个组件 |
| U8 | 两处主从布局 `lg:`(1024)→`md:`(768)：`ProfileManager` 与 `SystemConsole`。窗口最小宽度 860/960 均 ≥ 768 ⇒ **在允许的任何窗口尺寸下都不再塌成单列**；只改断点、不动窗口尺寸（评审 §10 明令） | `pages/ProfileManager.tsx`、`components/system/SystemConsole.tsx` |
| U10 | `ProfileManager` 页头 `sticky top-0 z-20` + `bg-bg/90 backdrop-blur-sm`，负外边距抵消 `PageShell` 的 `px-4/6/8`；长列表滚动后视图切换入口常驻 | `pages/ProfileManager.tsx` |
| U14 | 会话行右侧操作区**只留动作**：删掉与左徽标重复的「运行中」「健康/未知」胶囊（原来三选一渲染），仅在「需修复且非运行中」时给修复按钮 | `components/profiles/SessionManager.tsx` |

**机器闸门**：`__tests__/fontTokens.test.ts` 禁止再出现 `text-[Npx]`/`text-[Nrem]`，
需要新档位先在 `@theme` 加 token。

**未做/风险**：本次改动均为类名与结构，**未实机目检**（Tauri 需真实窗口）。
需人工确认两点：① 860px 宽时左栏 Profile 卡片不挤（4/12 栏 ≈ 280px）；
② 吸顶页头在滚动时不遮挡首行内容（`bg-bg/90` + 模糊为遮挡兜底）。
