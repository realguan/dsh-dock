# 前端框架迁移技术方案

> 本文档是 ADR-0008 的实施计划。ADR 记录「为什么」，本文档记录「怎么做」。
> 状态：待执行。2026-08-27 完成审核修订：设计 token 对照 app.css 逐变量校对（原稿深色系系杜撰，弃）；IPC 面对齐 v0.4.7 后现状（12 命令，open_about 已删）；错误卡改为数据驱动（对齐真实 payload）；新增平台/宿主抽象层与前瞻触发器条款（§11）
> 最后更新：2026-08-27

---

## 0. 定位：重构，不是复刻（2026-08-27 维护者裁定）

本次前端工作**不带历史包袱**：视觉稿、布局、交互流、文案措辞均可推翻重来。本文档各处「从原 xx.html 迁移的功能点」表格一律读作**能力清单，而非布局规范**——能力不可缺，外观全放开。

**设计基调（2026-08-27 二次裁定）**：整体沿用 **dsh web UI 风格的浅色主题**——明暗基调不在本次改动范围内；深色/主题切换只保留 data-theme 技术预留，不做实现。

**自由区 / 契约区边界**：

- ✅ **可动**：布局结构、排版、动画与交互细节、组件形态、页面组织（如重排启动引导、合并 selector 的呈现方式）、文案改写润色，以及**浅色基调内**的色彩微调。
- ❌ **不动（契约区）**：Rust 侧行为与 12 个 IPC 命令、5 类事件协议及 payload 形状、`withGlobalTauri`、initialization_script、外链拦截、平台能力语义（WSL 仅 Windows、远端会话拒绝 upgrade 类）、AGENTS §3 品牌规则（徽章一律官方 mark.svg 几何 + CSS mask，禁止第二份 path 或自造 logo），以及上条的**浅色基调本身**。
- ⚠️ **有条件可动**：① 窗口观感相关的任何改动必须核对/同步 lib.rs 里主窗/about 的原生 `background_color`，避免冷启动闪色；② 文案改写中涉技术事实的表述（如宿主解析链顺序、WSL 安装语义）须与源码/本文档核对后书写，不得凭印象写。

---

## 1. 技术栈

| 层面 | 选型 | 版本 | 用途 |
|:---|:---|:---|:---|
| 框架 | React | ^19 | UI 组件化 |
| 语言 | TypeScript | ^5.6（strict） | 类型安全 |
| 构建 | Vite | ^6 | 开发服务器 + 生产构建 |
| 样式 | Tailwind CSS | v4 | 原子化 CSS，`@theme` 定义设计 token |
| 组件库 | shadcn/ui | latest | 按需复制组件（Radix UI 基础） |
| 路由 | React Router | ^7 | 主窗口内页面切换（mode/selector） |
| 状态管理 | Zustand | ^5 | 启动状态 + 客户端更新状态 |
| 动画 | Framer Motion | ^11 | 组件进入/退出动画（AnimatePresence） |
| 图标 | Lucide React | ^0.460 | 与 shadcn/ui 配套 |
| Tauri API | @tauri-apps/api | ^2 | invoke / event / window 封装 |
| 测试 | Vitest | ^2 | 关键逻辑单测（不测 UI 渲染） |
| 静态检查 | tsc --noEmit + ESLint（flat config，react-hooks 规则必开） | — | CI 质量闸门；与 Now 阶段工程化基线（cargo fmt/clippy，roadmap §4.1）合批接入 |

### 执行前验证项

- [x] 在临时目录验证 Tailwind v4 + shadcn/ui 兼容性——**2026-08-27 已通过，保留 Tailwind v4，不降级**。证据：`npx shadcn@latest init -y -b radix -p nova` 对 Vite + `tailwindcss@4.3.3` 显式报 `Validating Tailwind CSS. Found v4.`；button/card/dialog/progress/badge/tooltip/separator 七组件全部生成；strict TS 下 `vite build` 成功（JS gzip 73KB，落在 ADR-0008 预估 60–80KB 内）。
- ~~如不兼容，降级 Tailwind v3.4（`tailwind.config.js` 配置，shadcn/ui 完全兼容），ADR-0008 §6 复审条件已覆盖~~（未触发）

**Spike 带回的三个执行注意点**（Stage A 落地时遵守）：

1. **别名不用 `baseUrl`**：新 TypeScript 已弃用该选项（TS5101 硬错误）；`paths` 以 tsconfig 所在目录为基准即可。
2. **新版 shadcn CLI 非交互参数**：组件库经 `-b radix` 选定（统一 `radix-ui` 包），预设必须 `-p <name>` 指定否则交互挂起；init 会改写 `src/index.css`。
3. **剥离预设字体**：preset 会把 Geist 字体 woff2 打进产物（约 30KB×4），与契约区的系统字体栈（PingFang SC 等）冲突——init 后须删除其 `@font-face` / `@theme` 中 `--font-*` 覆盖，恢复系统栈。

---

## 2. 目录结构

```
dsh-dock/
├── frontend/                              # React 前端源码
│   ├── package.json
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── tsconfig.node.json
│   ├── eslint.config.js                   # ESLint flat config 最小集（react-hooks + TS 必开项）
│   ├── tailwind.config.ts                 # Tailwind v4 配置（如需要）
│   ├── index.html                         # Vite SPA 入口
│   ├── public/
│   │   └── mark.svg                       # 官方徽章形状源（自 ui/assets/ 迁移；dsh-logo.svg 已迁至仓库根 assets/ 作品牌溯源，见 §7 §3 条目）
│   └── src/
│       ├── main.tsx                       # React 入口 + 事件总线初始化
│       ├── App.tsx                        # 窗口 label 判断 + 路由配置
│       ├── index.css                      # Tailwind 指令 + @theme token + 少量自定义类
│       │
│       ├── pages/
│       │   ├── BootIndex.tsx              # 启动序列（原 index.html）
│       │   ├── BootMode.tsx               # 运行环境选择（原 mode.html）
│       │   ├── BootSelector.tsx           # 工作台选择器（原 selector.html）
│       │   └── About.tsx                  # 关于/更新中心（原 about.html）
│       │
│       ├── components/
│       │   ├── ui/                        # shadcn/ui 组件（按需添加，不手动修改）
│       │   ├── boot/                      # 启动序列业务组件
│       │   │   ├── BootTimeline.tsx       # 5 步时间线
│       │   │   ├── BootStep.tsx           # 单步指示器
│       │   │   ├── DownloadProgress.tsx   # 下载进度（百分比/字节/速度/ETA）
│       │   │   ├── ErrorCard.tsx          # 错误卡（retry/upgrade/upgrade_only）
│       │   │   ├── VersionChip.tsx        # 三维度版本徽章
│       │   │   └── PulseBar.tsx           # 脉冲进度条
│       │   ├── about/                     # 关于页业务组件
│       │   │   ├── ClientUpdateCard.tsx   # 客户端自更新状态卡
│       │   │   ├── DshVersionCard.tsx     # dsh 版本卡
│       │   │   └── NodeVersionCard.tsx    # Node 版本卡
│       │   └── layout/
│       │       ├── Emblem.tsx             # 官方徽章（mark.svg + CSS mask）
│       │       └── PageShell.tsx          # 页面外壳（居中布局 + 最大宽度）
│       │
│       ├── stores/
│       │   ├── bootStore.ts               # 启动状态（step/progress/error/versions）
│       │   └── clientUpdateStore.ts       # 客户端自更新状态机
│       │
│       ├── lib/
│       │   ├── tauri.ts                   # invoke 封装 + 12 个命令的类型化 API（open_about 已于 v0.4.7 删除）
│       │   ├── events.ts                  # 事件总线初始化（5 类事件 → store，payload 边界规整）
│       │   ├── host.ts                    # 平台/宿主能力矩阵（读 __DSH_PLATFORM__，can() 过滤动作可见性）
│       │   ├── resource.ts                # 资源型取数统一入口约定（一期薄实现；Query 触发器见 §11）
│       │   ├── format.ts                  # 格式化工具（mb/fmtEta/版本显示）
│       │   └── utils.ts                   # cn() 等通用工具
│       │
│       ├── hooks/
│       │   └── usePlatform.ts             # 平台检测（WSL/Windows/macOS/Linux）
│       │
│       ├── types/
│       │   ├── ipc.ts                     # IPC 命令请求/响应类型
│       │   └── events.ts                  # 事件 payload 类型
│       │
│       ├── content/
│       │   └── zh-CN.ts                   # 中文文案常量（i18n 预留）
│       │
│       └── __tests__/                     # Vitest 测试
│           ├── format.test.ts             # 格式化函数测试
│           ├── bootProgress.test.ts       # 下载速度/ETA 计算测试
│           ├── bootStep.test.ts           # 步骤状态推演测试
│           └── updatePhase.test.ts        # 客户端更新状态机合法迁移测试
│
├── src-tauri/
│   ├── tauri.conf.json                    # frontendDist/devUrl/beforeCommand 变更
│   └── src/lib.rs                         # 窗口 URL + platform_script 注入对象变更
│
├── .github/workflows/build.yml            # 加 node 步骤
│
└── AGENTS.md                              # §1/§2/§4.2/§4.3 + 新增 §4.4
```

`ui/` 目录在迁移完成后删除。

---

## 3. 核心架构设计

### 3.1 多窗口路由（窗口 label 方案）

所有窗口都加载 `/`（`WebviewUrl::App("/".into())`），React 启动时根据 Tauri 窗口 label 决定渲染哪个页面：

```tsx
// App.tsx
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useEffect, useState } from 'react'
import { Routes, Route, Navigate } from 'react-router-dom'
import BootIndex from './pages/BootIndex'
import BootMode from './pages/BootMode'
import BootSelector from './pages/BootSelector'
import About from './pages/About'

export default function App() {
  const [label, setLabel] = useState<string | null>(null)

  useEffect(() => {
    getCurrentWindow().label().then(setLabel)
  }, [])

  if (label === null) return null // 短暂 loading，桌面应用可接受

  // about 窗口：只渲染 About 页面，不需要路由
  if (label === 'about') return <About />

  // 主窗口：启动序列 + 路由切换
  return (
    <Routes>
      <Route path="/" element={<BootIndex />} />
      <Route path="/mode" element={<BootMode />} />
      <Route path="/selector" element={<BootSelector />} />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  )
}
```

**Rust 侧窗口 URL 变更**：

| 位置 | 原 | 新 |
|:---|:---|:---|
| `create_main_window` | `WebviewUrl::App("index.html".into())` | `WebviewUrl::App("/".into())` |
| `open_about_window` | `WebviewUrl::App("about.html".into())` | `WebviewUrl::App("/".into())` |
| Rust 侧跳转 selector | `location.assign('selector.html?profiles={}')` | `location.assign('/selector?profiles={}')` |
| Rust 侧回退 index | `location.assign('index.html')` | `location.assign('/')` |

mode 页面跳转：原 `location.assign('index.html?mode=...&default=...')` 改为 `navigate('/?mode=...&default=...')`（React Router，在 BootMode 组件内处理）。

**生产可用性依据（2026-08-27 已验证）**：tauri 2.11.5 内嵌资产解析链为「精确路径 → `{path}.html` → `{path}/index.html` → 兜底 `index.html`」（tauri crate `src/manager/mod.rs` 的 `get_asset`），query/fragment 剥离后匹配——release 包内 pathname 路由可达；dev 下由 Vite historyApiFallback 兜底。此结论已于 **2026-08-27 在 release 产物实机钉板复核通过**（临时第二 profile 触发 `location.assign('/selector…')`，主窗口渲染 BootSelector 页；about 窗口 label 路由同批验证），复检记录见 broadcasts.md 同日条目。

**导航与状态生命周期（重要修正）**：Rust 侧跳转（`location.assign('/selector…')`）与 mode 页回跳都是**整页重载**，Zustand 内存态不会跨这些导航存活。「组件复用」指 DownloadProgress/ErrorCard/PulseBar 的代码级复用；每次进入页面 store 全新创建，靠 `get_update_status` / `get_client_update` 重新播种 + 事件流持续驱动。原稿「bootStore 在 BootIndex/BootSelector 间共享状态」的表述作废（§9 验收项同步改写）。

### 3.2 状态管理（Zustand）

#### bootStore.ts（启动状态，跨 BootIndex/BootSelector 共享）

```typescript
interface BootState {
  // boot:step —— 0=检查环境, 1=确定启动方式, 2=启动DSH, 3=等待就绪, 4=进入工作台
  step: number
  running: boolean
  stepDetails: string[]  // 每步 detail 文本

  // boot:progress —— 下载进度
  progress: {
    kind: string
    current: number
    total: number | null
    // 速度计算（从原 index.html speedSamples 迁移）
    speed: number | null       // bytes/s
    eta: number | null         // seconds
  } | null

  // boot:error —— 形状照实对齐 Rust 现有发射（ui/index.html 与 selector.html
  // showError 同款）：按钮集合 actions[] 由后端下发，前端只做 id→文案映射，
  // 不自行决定动作集合
  error: {
    title?: string       // 缺省「启动失败」
    detail?: string
    suggestion?: string
    actions?: string[]   // 如 ['retry'] / ['upgrade']；selector 场景本地固定追加 reselect
    log?: string         // 原始日志，折叠面板展示
  } | null

  // boot:update —— 三维度版本状态
  versions: {
    dsh: { version: string; origin: string; latest?: string } | null
    client: { version: string; latest?: string } | null
    node: { version: string; origin: string } | null
  } | null

  // actions
  setStep: (step: number, running: boolean, detail?: string) => void
  setProgress: (p: { kind: string; current: number; total: number | null }) => void
  setError: (e: BootErrorPayload) => void
  setVersions: (v: VersionsSnapshot) => void  // types/events 定义，边界处容忍未知字段
  reset: () => void
}
```

**步骤状态推演逻辑**（从原 index.html 迁移）：
- `setStep(N, running)` 被调用时，N 之前所有未定状态的步骤自动置为 `done`
- 步骤状态：`pending` → `running` → `done` / `error`
- 5 步的名称和提示文案从原 `STEPS` / `HEADLINES` 数组迁移到 `content/zh-CN.ts`

**速度计算逻辑**（从原 index.html 迁移，放在 store action 中统一计算）：
- `speedSamples` 数组保存最近 N 个采样点（时间戳 + 字节数）
- 每次 `setProgress` 更新时，计算滑动窗口内的平均速度
- ETA = (total - current) / speed
- 格式化在 `lib/format.ts` 中

**组件消费规范**（防止重渲染）：
```typescript
// ✅ 精细选择器，只订阅需要的字段
const step = useBootStore(s => s.step)
const progress = useBootStore(s => s.progress)

// ❌ 不要这样写，progress 高频更新会导致整个组件重渲染
const { step, error } = useBootStore()
```

#### clientUpdateStore.ts（客户端自更新状态机，About 页面用）

```typescript
type UpdatePhase = 'idle' | 'checking' | 'available' | 'downloading'
  | 'installing' | 'relaunching' | 'done' | 'error'

interface ClientUpdateState {
  phase: UpdatePhase
  version?: string           // 可更新到的版本
  currentVersion?: string    // 当前版本
  current?: number           // 下载进度（字节）
  total?: number
  error?: string

  hydrate: (snapshot: ClientUpdateSnapshot) => void  // 整页重载后播种初始状态
  dispatch: (event: AppUpdateEvent) => void          // 经纯函数迁移，非法迁移忽略
  reset: () => void
}

// 迁移表与迁移动作导出为纯函数，供 Vitest 直接测（updatePhase.test.ts）
export const TRANSITIONS: Record<UpdatePhase, UpdatePhase[]> = {
  idle: ['checking'],
  checking: ['idle', 'available', 'error'],
  available: ['downloading', 'idle'],
  downloading: ['installing', 'error'],
  installing: ['relaunching', 'done'],
  relaunching: ['done'],
  done: ['idle'],
  error: ['checking', 'idle'],
}
export function applyUpdateEvent(s: ClientUpdateState, e: AppUpdateEvent): ClientUpdateState {
  /* 纯函数实现：不在 TRANSITIONS[phase] 内的事件丢弃并 tracing 无关（前端 console.warn） */
}
```

### 3.3 Tauri 集成层

#### lib/tauri.ts（invoke 封装，12 个命令全类型化）

```typescript
import { invoke } from '@tauri-apps/api/core'
import type { UpdateStatus, ClientUpdate } from '../types/ipc'

export const api = {
  // 启动流程
  chooseProfile: (profile: string) =>
    invoke<void>('choose_profile', { profile }),
  chooseMode: (mode: string, setDefault: boolean) =>
    invoke<void>('choose_mode', { mode, setDefault }),
  bootInWsl: () => invoke<void>('boot_in_wsl'),

  // 版本状态
  getUpdateStatus: () => invoke<UpdateStatus>('get_update_status'),
  checkUpdates: () => invoke<void>('check_updates'),

  // 客户端自更新
  getClientUpdate: () => invoke<ClientUpdate>('get_client_update'),
  clientUpdateCheck: () => invoke<void>('client_update_check'),
  clientUpdateApply: () => invoke<void>('client_update_apply'),

  // 错误卡动作
  terminalAction: (action: 'retry' | 'upgrade' | 'upgrade_only') =>
    invoke<void>('terminal_action', { action }),

  // 窗口/导航
  openExternal: (url: string) => invoke<void>('open_external', { url }),
  openWorkbenchInBrowser: () => invoke<void>('open_workbench_in_browser'),
  getWorkbenchUrl: () => invoke<string | null>('get_workbench_url'),
}
```

所有调用统一 `.catch()` 处理。组件中不直接 `invoke()`，必须走 `api` 对象。参数拼写以本文件为准（已对照现网 ui/*.html 逐一核实：如 choose_mode 的 `{ mode, setDefault }` 由 Tauri 自动映射 snake_case Rust 参数）。`open_external` 的主要消费者是 initialization_script 注入 dsh 页面的脚本，壳页面仅白名单外链场景使用。**前端运行时不得新增任何网络请求**（无外链字体/CDN/统计）——壳网络面白名单仍唯一指向 Rust updates.rs。

#### lib/events.ts（事件总线，在 App 顶层 useEffect 中初始化）

```typescript
import { listen } from '@tauri-apps/api/event'
import { useBootStore } from '../stores/bootStore'
import { useClientUpdateStore } from '../stores/clientUpdateStore'

// listen<T>() 泛型标注 payload；入库前经 normalize*() 规整：
// 缺字段补默认、未知字段忽略、无法识别的整体丢弃——对「dsh/壳先行升级新增字段」前向兼容
export function initEventBus(): () => void {
  const unlisteners: Promise<() => void>[] = [
    listen<BootStepEvent>('boot:step', ({ payload }) =>
      useBootStore.getState().setStep(payload.step, payload.running, payload.detail)),
    listen<unknown>('boot:progress', ({ payload }) => {
      const p = normalizeProgress(payload)
      if (p) useBootStore.getState().setProgress(p)
    }),
    listen<unknown>('boot:error', ({ payload }) => {
      const err = normalizeError(payload)
      if (err) useBootStore.getState().setError(err)
    }),
    listen<unknown>('boot:update', ({ payload }) => {
      const v = normalizeVersions(payload)
      if (v) useBootStore.getState().setVersions(v)
    }),
    listen<unknown>('app:update', ({ payload }) => {
      const ev = normalizeAppUpdate(payload)
      if (ev) useClientUpdateStore.getState().dispatch(ev)
    }),
  ]
  return () => unlisteners.forEach(u => u.then(fn => fn()))
}
```

**关键（2026-08-27 阶段 D 修正）**：事件总线**不在 App effect 里初始化**——React 子组件 effect 先于父组件执行，页面播种的 invoke（如 BootIndex 的 `choose_mode` 握手）可能抢在监听挂载之前，启动线程首发射出的事件即被吞掉（旧 `ui/index.html` 用同步 `<script>` 注册监听正是为规避该竞态）。实现：事件总线在 **lib/events.ts 模块加载期**自动装配（`export const eventBusStarted = initEventBus()`），在任何渲染/播种发生前即注册；两个窗口各自 runtime 各一份；应用生命周期内不拆卸，也无 StrictMode 双监听问题。`boot:step` 的真实 payload 为 `{step, state, detail}`（state=pending|running|done|error，非草案的 running 布尔）——store 按真实形状建模。事件名常量集中定义在 `types/events.ts`，消灭魔法字符串。

### 3.4 组件分层

| 层 | 职责 | 数据来源 | 示例 |
|:---|:---|:---|:---|
| `pages/` | 页面级组合，路由入口，页面级副作用 | 组合 store + api | BootIndex, About |
| `components/boot/` | 启动序列业务组件 | 消费 bootStore | BootTimeline, DownloadProgress |
| `components/about/` | 关于页业务组件 | 消费 clientUpdateStore + bootStore | ClientUpdateCard |
| `components/layout/` | 纯布局组件，无业务逻辑 | props | Emblem, PageShell |
| `components/ui/` | shadcn/ui 基础组件 | props | Button, Card, Dialog |

**原则**：业务组件消费 store，基础组件只收 props，页面组件负责组合和 useEffect 副作用。

### 3.5 设计 token 与样式

#### index.css（Tailwind v4 @theme）

```css
@import "tailwindcss";

@theme {
  /* ↓↓↓ 设计基调 = dsh web UI 风格浅色主题（§0 二次裁定），以下即按此校对的
     目标值（2026-08-27 逐变量对照 ui/assets/app.css）。与原生窗口背景
     (#f9fafb) 一致，冷启动无闪色。基调内色彩微调随设计稿迭代；
     整组换底色/切换明暗属基调变更，不在本次范围，若将来立项须同步
     lib.rs 的 background_color。data-theme 为暗色远期技术预留。 */
  --color-bg: #f7f8fb;
  --color-panel: #ffffff;
  --color-line: #e5e9f1;
  --color-line-soft: #eef1f6;
  --color-ink: #191d27;
  --color-dim: #626a7a;      /* 原 muted；shadcn 语义层占用该名后改名（2026-08-27） */
  --color-faint: #a0a7b6;
  --color-brand: #4176e6;   /* 原 accent；同上改名 */
  --color-brand-deep: #3163cf;
  --color-wash: rgba(65, 118, 230, 0.07);
  --color-ok: #2f9e44;
  --color-ok-soft: rgba(47, 158, 68, 0.1);
  --color-warn: #d9480f;
  --color-warn-soft: rgba(217, 72, 15, 0.08);
  --color-badge-a: #1d2637;
  --color-badge-b: #0e1524;

  --font-sans: -apple-system, "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", sans-serif;
  --font-mono: ui-monospace, "SF Mono", "JetBrains Mono", Menlo, Consolas, monospace;
}

/* 无法用 Tailwind 表达的复杂样式，保留自定义类 */
.emblem::after {
  content: "";
  position: absolute;
  inset: 7px;
  -webkit-mask: url("/mark.svg") center / contain no-repeat;
  mask: url("/mark.svg") center / contain no-repeat;
  background: white;
}

@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}
.animate-blink { animation: blink 1.4s steps(2, start) infinite; }

@keyframes pulse-bar {
  /* 从原 app.css 迁移 */
}
```

**颜色使用规范**：
- ✅ `bg-bg`、`text-ink`、`text-accent`、`border-line`（走 token）
- ❌ `bg-[#0f1220]`、`text-[#e8ecf4]`（不硬编码 hex）
- 未来暗色模式经 `data-theme` 属性覆盖变量实现，Tailwind 类名不变

#### Emblem 组件

```tsx
// components/layout/Emblem.tsx
export function Emblem({ size = 44 }: { size?: number }) {
  return (
    <div
      className="emblem relative rounded-xl border border-white/10"
      style={{
        width: size,
        height: size,
        background: 'linear-gradient(160deg, var(--color-badge-a), var(--color-badge-b))',
        boxShadow: '0 10px 26px rgba(20,28,48,0.28), inset 0 1px 0 rgba(255,255,255,0.1)',
      }}
    />
  )
}
```

`.emblem::after` 的 CSS mask 在 index.css 中定义，不内联。

### 3.6 文案抽离（i18n 预留）

```typescript
// content/zh-CN.ts
// STEPS 三元组逐字取自 ui/index.html（第三列 hint 是步骤旁的灰色说明文字，
// 不可丢弃）；headlines 是另一套独立文案（与步骤名不同文），两者不得互相推断。
export const t = {
  boot: {
    steps: [
      { no: '01', name: '环境检测', hint: 'PATH · 版本闸 · 平台' },
      { no: '02', name: '宿主解析', hint: 'local：system → bundle → download' },
      { no: '03', name: '启动 DSH', hint: '--port 0' },
      { no: '04', name: '等待就绪', hint: '解析访问地址（慢速冷启动可稍候）' },
      { no: '05', name: '进入工作台', hint: 'WebView 导航' },
    ],
    headlines: ['检查运行环境', '确定启动方式', '启动 DSH', '等待工作台就绪', '即将进入工作台'],
  },
  // 错误动作文案表：boot:error payload 的 actions[] 只给 id，文案在此映射；
  // 未知 id 回退展示原文——新宿主/新失败模式无需改组件
  error: {
    fallbackTitle: '启动失败',
    actions: {
      retry: '重试',
      upgrade: '升级 DSH 并重试',
      upgrade_only: '后台升级',
      reselect: '返回重选',
    } as Record<string, string>,
  },
  mode: {
    title: '选择运行环境',
    local: '本机运行',
    wsl: 'WSL 中运行',
    setDefault: '设为默认方式',
    next: '下一步',
  },
  about: {
    title: '关于与更新',
    checkUpdate: '检查更新',
    // ...
  },
} as const
```

组件中引用 `{t.boot.headlines[step]}` 与 `{t.error.actions[actionId]}`（未知 actionId 回退展示 id 原文），不硬编码中文字符串。i18n 的未来形态 = 带 key 与插值参数的 `t(key, params)` 函数替换常量来源，页面结构无需返工。

---

## 4. 四个页面迁移详细设计

> 本节全部表格都是 §0 所指的能力清单：功能点对照实现不可缺失，但布局、交互与视觉**不作要求**，鼓励按新设计重排——包括页面级重组（如环境选择并入首屏引导流）。

### 4.1 BootIndex（启动序列，最复杂）

**从原 index.html 迁移的功能点**：

| 功能 | React 实现 |
|:---|:---|
| 平台检测 | `usePlatform()` hook 读 `window.__DSH_PLATFORM__`（Rust 注入 `{ os, wsl }`，扩展见 §5.2），前端聚合成 host 能力对象 |
| 5 步时间线 | `<BootTimeline>` 消费 `bootStore.step` + `stepDetails`，Framer Motion 做步骤切换动画；步表来自 `t.boot.steps`，UI 不写死步数 |
| 步骤状态推演 | `bootStore.setStep` 内部实现（N 之前未定步骤自动置 done） |
| 下载进度 | `<DownloadProgress>` 消费 `bootStore.progress`（含 speed/eta），进度条用 shadcn/ui Progress + Framer Motion |
| 字节/速度/ETA 格式化 | `lib/format.ts`（mb/fmtEta），Vitest 单测覆盖 |
| 错误卡 | `<ErrorCard>` 数据驱动消费 `bootStore.error`（title/detail/suggestion/log 全来自 payload），AnimatePresence 进出场动画；按钮遍历 `actions[]` 经 `t.error.actions` 映射后调 `api.terminalAction(actionId)`，未知 id 回退原文 |
| 版本徽章 | `<VersionChip>` 消费 `bootStore.versions`（dsh/client/node 三维度） |
| 客户端更新状态 | 消费 `clientUpdateStore`，在版本徽章旁显示 |
| pulseBar 动画 | `<PulseBar>` 组件，CSS animation |
| WSL 按钮 | 调用 `api.bootInWsl()`，经 `host.can('boot_wsl')` 控制显示（宿主能力，勿写裸平台 if） |
| headline/subline 动态文本 | headline 取 `t.boot.headlines[step]`，subline 取当前步骤 `hint` |

**页面级副作用**：
```typescript
useEffect(() => {
  // 事件总线已在 App 顶层初始化；这里只做「整页重载后的状态重播种」

  // 1. 初始版本状态（弥补进入本页前错过的 boot:update / app:update）
  api.getUpdateStatus()
    .then(v => useBootStore.getState().setVersions(v))
    .catch(() => {})
  api.getClientUpdate()
    .then(u => useClientUpdateStore.getState().hydrate(u))
    .catch(() => {})

  // 2. 检查 URL 参数（mode/default），触发 choose_mode
  const params = new URLSearchParams(location.search)
  const mode = params.get('mode')
  const setDefault = params.get('default') === '1'
  if (mode) {
    api.chooseMode(mode, setDefault).catch(() => {})
  }
}, [])
```

### 4.2 BootMode（运行环境选择）

| 功能 | React 实现 |
|:---|:---|
| local/WSL 卡片选择 | 两个 shadcn/ui Card，useState 管理选中，Framer Motion 做选中动画 |
| WSL 仅 Windows | `usePlatform()` 检测，非 Windows 不渲染 WSL 卡 |
| "设为默认"复选框 | useState 管理 |
| 下一步按钮 | 调用 `api.chooseMode(selectedMode, setDefault)`，成功后 `navigate('/')`（Router 内切换，内存态保留） |
| 官方徽章 | `<Emblem>` |

### 4.3 BootSelector（工作台选择器）

| 功能 | React 实现 |
|:---|:---|
| 从 URL 解析 profiles | `useSearchParams()` 获取 `?profiles=web,xxx`，split 成数组 |
| profile 卡片列表 | 遍历 profiles，shadcn/ui Card 显示名称/描述，点击调用 `api.chooseProfile(name)` |
| profile 元数据 | 从 `content/zh-CN.ts` 的 META 映射取名称/描述 |
| 下载进度 | 复用 `<DownloadProgress>` 代码（整页跳转后本页 store 重建，事件流驱动继续有效——组件级复用而非运行时共享） |
| 错误处理 | 复用 `<ErrorCard>` |
| pulseBar | 复用 `<PulseBar>` |
| headline/subline | 根据 bootStore.step 状态切换 |

### 4.4 About（关于/更新中心）

> 入口仅菜单（macOS）/ 托盘（非 macOS）：前端顶栏「关于」按钮与 `open_about` IPC 已于 v0.4.7 删除（见 broadcasts.md 2026-08-27 补记），本页无任何「打开关于」的对外入口职责。

| 功能 | React 实现 |
|:---|:---|
| 三维度版本状态 | 三个卡片组件，消费 `bootStore.versions` |
| 客户端自更新状态机 | `<ClientUpdateCard>` 消费 `clientUpdateStore`，根据 phase 条件渲染 |
| 检查更新按钮 | 调用 `api.clientUpdateCheck()`，按钮 loading 状态 |
| 下载并安装按钮 | 调用 `api.clientUpdateApply()`，显示进度条 |
| dsh 升级（upgrade_only） | 调用 `api.terminalAction('upgrade_only')`，不打断会话 |
| 手动检查 dsh 更新 | 调用 `api.checkUpdates()` |
| 在浏览器中打开工作台 | 调用 `api.openWorkbenchInBrowser()` |
| 初始状态获取 | useEffect 中 `api.getClientUpdate()` + `api.getUpdateStatus()` |

**客户端更新状态机渲染**（从原 about.html renderUpd 迁移）：

| phase | UI |
|:---|:---|
| idle | "检查更新"按钮 |
| checking | 按钮 disabled，"检查中..." |
| available | 版本号 + "下载并安装"按钮 |
| downloading | Progress 进度条 + 百分比 + 字节 |
| installing | "安装中..." |
| relaunching | "即将重启..." |
| done | "已更新，重启生效" |
| error | 错误信息 + 重试按钮 |

---

## 5. Tauri 侧变更

### 5.1 tauri.conf.json

```json
{
  "build": {
    "beforeDevCommand": "cd ../frontend && npm run dev",
    "beforeBuildCommand": "cd ../frontend && npm run build",
    "devUrl": "http://localhost:1420",
    "frontendDist": "../frontend/dist"
  },
  "app": {
    "withGlobalTauri": true,
    "security": { "csp": null }
  }
}
```

**注意**：
- `withGlobalTauri: true` 必须保持（dsh web UI 的 initialization_script 依赖 `window.__TAURI__`）
- `beforeDevCommand` / `beforeBuildCommand` 用 `cd ../frontend &&` 切换工作目录（Tauri 默认在 src-tauri/ 下执行）
- `devUrl` 端口 1420 与 Vite 配置一致

### 5.2 lib.rs 变更

只改窗口 URL 字符串、Rust 侧 `location.assign` 路径，以及 `platform_script` 注入对象的扩展——`window.__DSH_PLATFORM__ = { os: '<target_os>', wsl: <bool> }`（编译期 `cfg!` 判定，扩为对象但**不加任何新 IPC 命令**；SSH 未来落地时再增补 `host` 维度，见 §11 触发器）。不改任何 IPC 命令实现：

```rust
// create_main_window
tauri::WebviewUrl::App("/".into())  // 原 "index.html"

// open_about_window
tauri::WebviewUrl::App("/".into())  // 原 "about.html"

// Rust 侧跳转
format!("location.assign('/selector?profiles={}')", ...)  // 原 'selector.html?profiles={}'
"location.assign('/')"  // 原 'index.html'
```

### 5.3 不变的部分

- 12 个 IPC 命令实现（接口不变）
- 5 类事件发射（boot:step/progress/error/update, app:update）
- `initialization_script`（WebView 内存策略 + 外链兜底，注入到 dsh web UI）
- `on_navigation` / `on_new_window`（外链拦截）
- 所有 Rust 后端模块（shell/executor/resolve/updates/manifest/settings/updater）
- `build.rs` / `capabilities/default.json`（权限配置不变）
- about 窗口开启链路＝菜单（macOS）/ 托盘（非 macOS）→ `open_about_window`，本迁移不触碰（其窗口 URL 改 `/` 见 §3.1 表）

---

## 6. CI 变更

在 `.github/workflows/build.yml` 的 Rust 构建步骤前新增：

```yaml
- name: Setup Node
  uses: actions/setup-node@v4
  with:
    node-version: '20'
    cache: 'npm'
    cache-dependency-path: frontend/package-lock.json

- name: Install frontend dependencies
  working-directory: frontend
  run: npm ci

- name: Build frontend
  working-directory: frontend
  run: npm run build
```

三平台矩阵各自执行（Vite 构建 < 10s，影响可接受）。后续可优化为独立 job 构建 + artifact 传递。

质量闸门（typecheck / lint 与 Rust 侧 fmt、clippy 基线【roadmap §4.1】合批接入同一 workflow，避免两次改 build.yml）：
```yaml
- name: Typecheck frontend
  working-directory: frontend
  run: npm run typecheck

- name: Lint frontend
  working-directory: frontend
  run: npm run lint

- name: Test frontend
  working-directory: frontend
  run: npm run test -- --run
```

---

## 7. AGENTS.md 修改清单

| 章节 | 修改内容 |
|:---|:---|
| **§1 技术栈表** | 前端行改为：「Vite 6 + React 19 + TS strict + Tailwind v4 + shadcn/ui + React Router v7 + Zustand + Framer Motion（`frontend/`）」；新增「前端构建：Vite，CI/开发需 node 20+」；开发环境说明加「前端开发需 node 20+，Rust 开发不变」 |
| **§2 目录结构** | 新增 `frontend/` 目录说明（含子目录结构）；`ui/` 标记为「已迁移至 frontend/，删除」 |
| **§4.2 Rust ❌ 禁止** | 移除「引入任何前端构建链」禁令；改为「引入数据库 / IPC 总线 / 领域服务——壳要保持薄。前端框架仅限 React 生态（Vite + React + TS + Tailwind + shadcn/ui），不引入 Vue/Angular/Svelte 等其他框架」 |
| **§4.3 UI 规范** | 全面重写：React 组件规范（函数组件 + hooks，PascalCase 文件名）、Tailwind 规范（样式全用 Tailwind 类，颜色走 @theme token 不硬编码 hex）、shadcn/ui 规范（按需添加，不修改 components/ui 源码）、Zustand 规范（一个领域一个 store，组件用精细选择器）、invoke 规范（统一走 lib/tauri.ts，组件内不直接 invoke）、事件规范（统一在 App 顶层初始化，组件只消费 store）、Framer Motion 规范（进入/退出用 AnimatePresence，列表用 layout）、文案规范（文本走 content/ 常量，不硬编码） |
| **新增 §4.4 前端开发规范** | 前端开发约定：TypeScript strict 不允许 any（特殊情况用 `unknown` + 类型守卫）；组件目录组织（pages/components/ui/stores/lib/hooks/types/content）；测试策略（Vitest 测关键逻辑，不测 UI 渲染）；AI coding 约定（用 shadcn/ui 组件，样式用 Tailwind，状态用 Zustand）；品牌规则（Emblem 组件，mark.svg + CSS mask） |
| **§5 测试要求** | 新增前端测试说明：Vitest 覆盖格式化函数、速度计算、步骤推演、更新状态机迁移等纯逻辑；UI 走手动验证；不引入 React Testing Library/jsdom |
| **§7 IPC 例外册** | 第一期零增减（现存 **12 个命令**已登记；`open_about` 已于 v0.4.7 移除并有裁定记录）；后续管理功能新增命令时按三处同步登记 |
| **§3 品牌规则** | `dsh-logo.svg` 溯源路径由 `ui/assets/` 更新为仓库根 `assets/`（随迁移搬移）；emblem/mark 的组件化表述同步 |
| **§4.4（新增内容补强）** | 在原计划之上追加三条红线：① **依赖白名单**——允许 React 生态/Radix(shadcn)/未来数据层；禁止 AntD/MUI 类大全件库、CSS-in-JS 运行时；② **前端运行时禁止发起新网络请求**（网络面唯一指向 Rust updates.rs）；③ **跨窗口真相源原则**——主窗/about 各自独立 JS runtime，Zustand 不跨窗共享，事件广播是唯一跨窗通道 |

---

## 8. 测试策略

### 8.1 前端单测（Vitest）

覆盖以下纯逻辑：

| 测试文件 | 覆盖内容 | 用例数（预估） |
|:---|:---|:---|
| `format.test.ts` | mb() 字节格式化、fmtEta() 秒→分:秒、版本显示 | 6-8 |
| `bootProgress.test.ts` | 速度采样计算、ETA 估算、total=null 时的处理 | 5-6 |
| `bootStep.test.ts` | 步骤状态推演（N 之前自动置 done、error 状态传播） | 4-5 |
| `updatePhase.test.ts` | 客户端更新状态机：合法迁移放行、非法迁移拒绝、hydrate/reset 幂等 | 5-6 |

不测试：
- 组件渲染（jsdom + Testing Library 维护成本高，壳页面 UI 不复杂）
- 事件监听（Tauri API 在测试环境不可用，走手动验证）
- 路由跳转（React Router 标准行为，不需要测）

### 8.2 Rust 测试

`cargo test` 必须全绿。Rust 侧行为变更仅限 URL 字符串与 platform_script 注入对象（均不触及被测逻辑），现有测试全量保持通过（当前基线 90 个，以执行日实际数为准）。

### 8.3 手动验证清单

见 §9。

---

## 9. 功能验证清单

### 启动流程
- [ ] 冷启动：从 `/` 开始，完整走完 5 步启动序列，最终导航到 dsh web UI
- [ ] 下载进度：清理 Node 缓存后触发下载，进度条/百分比/字节/速度/ETA 正确显示
- [ ] 错误卡：断网模拟启动失败，按 payload 渲染 title/detail/suggestion/log（折叠面板），`actions[]` 生成的按钮全部可用且经 `t.error.actions` 映射文案
- [ ] 错误卡进入/退出动画正常（AnimatePresence）
- [ ] 步骤状态推演：boot:step 事件正确触发步骤状态变化
- [ ] 版本徽章：dsh/client/node 三维度版本正确显示
- [ ] WSL 按钮：Windows 下显示并可点击，非 Windows 不渲染

### 运行环境选择
- [ ] Windows：local/WSL 两张卡都显示，选中动画正常，设为默认可用
- [ ] macOS/Linux：只显示 local 卡
- [ ] 选择后正确跳转回 `/` 并触发 choose_mode

### 工作台选择器
- [ ] `/selector?profiles=web,xxx` 正确显示卡片列表
- [ ] 选择 profile 后触发 choose_profile 并继续启动
- [ ] 下载进度在 selector 页面正确显示（整页跳转后 store 重建 + getUpdateStatus 重播种 + 事件流驱动）

### 关于/更新中心
- [ ] 三维度版本状态正确显示
- [ ] 客户端更新全流程：检查 → 可用 → 下载 → 安装 → 重启
- [ ] dsh 升级（upgrade_only）不打断会话
- [ ] 在浏览器中打开工作台
- [ ] app:update 事件实时更新 UI

### 跨页面/窗口
- [ ] 整页跳转（index ↔ selector）后目标页正确重播种：进行中的下载进度 / 错误卡在新页面恢复显示
- [ ] 事件总线只在 App 顶层初始化一次（StrictMode 双调用无重复监听；页面副作用不再 init）
- [ ] about 窗口独立打开，不影响主窗口
- [ ] 页面路由切换无白屏（Framer Motion 过渡）

### 构建与 CI
- [ ] `npm run tauri dev` 正常启动，HMR 正常
- [ ] `npm run tauri build` 成功，产物可运行；**release 产物内人工触发 Rust 侧跳转 `/selector` 渲染正常**（SPA fallback 实机证据，解析链结论见 §3.1）
- [ ] `cd frontend && npm run test -- --run` 全绿
- [ ] `cd src-tauri && cargo test` 全绿
- [ ] CI 三平台矩阵全绿

### 回归
- [ ] initialization_script 在 dsh web UI 中仍然生效（内存策略 + 外链兜底）
- [ ] on_navigation/on_new_window 外链拦截仍然生效
- [ ] withGlobalTauri 保持 true，dsh web UI 的 open_external 调用正常
- [ ] 新设计全页面走查通过：无错位/截断/不可读文本；若调整了配色或明暗基调，冷启动到首帧无闪色（原生 background_color 已同步，§0 条件①）
- [ ] 品牌红线抽查：所有徽章实例均由 Emblem 组件渲染官方 mark.svg 几何（CSS mask 上白），无第二份 path、无自造图形
- [ ] 无外部网络资源（任何字体/图库/CDN 一律本地打包或系统字体栈，壳运行时不发起新网络请求）

---

## 10. 执行步骤

按顺序执行，每步完成后可独立提交，应用始终可运行。

> **执行前置条件（宪法级变更知会，AGENTS §10）**：开工前先在公共频道发布改动预告——本次涉及 AGENTS.md（§1/§2/§3/§4/§5/§7）、docs/roadmap.md（硬约束 2 与不做清单）、CI 工作流。收尾广播落档照旧（阶段 E 最后一步）。

### 阶段 A：脚手架与基础设施

3. 安装依赖：`npm install react-router-dom zustand framer-motion lucide-react @tauri-apps/api`
4. 安装 dev 依赖：`npm install -D tailwindcss @tailwindcss/vite vitest @types/node eslint eslint-plugin-react-hooks typescript-eslint @eslint/js`
5. 配置 Vite（server.port=1420 + **strictPort: true**，clearScreen=false, envPrefix）；写 package.json scripts：dev/build/typecheck/lint/test
6. 配置 Tailwind v4（vite 插件 + index.css @import + @theme token；token 即 §3.5 基调值，基调内微调随设计稿迭代）
7. **验证 shadcn/ui 兼容性**：`npx shadcn@latest init`，如失败降级 Tailwind v3.4 → **2026-08-27 已提前通过（见 §1 执行前验证项），保留 v4**；执行时按「别名裸 paths / `-b radix -p <name>` 非交互 / 剥离 Geist 字体」三注意点操作
8. 按需添加 shadcn/ui 组件：button card dialog progress badge tooltip separator。**toast/tabs/scroll-area 明确缓加**（错误反馈归 ErrorCard、480px 小窗纵排优于 tab、原生滚动够用——裁定时点与理由见 §11）
9. 配置 React Router（App.tsx，窗口 label 判断 + 路由）
10. 创建 Zustand stores（bootStore, clientUpdateStore——后者带 TRANSITIONS 纯函数迁移表）
11. 创建 lib/tauri.ts（12 个命令类型化封装）+ lib/events.ts（事件总线 + normalize* 边界规整）
12. 创建 lib/format.ts + lib/utils.ts + lib/host.ts（能力矩阵）+ lib/resource.ts（取数入口占位）
13. 编写 eslint.config.js 最小集（react-hooks 的 rules-of-hooks / exhaustive-deps 必开），跑通 `npm run lint` 与 `npm run typecheck`
14. 创建 types/ipc.ts + types/events.ts（事件名常量也在后者导出）
15. 创建 content/zh-CN.ts（steps 三元组 + headlines 独立文案 + error.actions 动作映射表）
16. 创建布局组件（Emblem, PageShell）
17. 复制 mark.svg 到 frontend/public/
18. 修改 tauri.conf.json（frontendDist, devUrl, beforeDevCommand, beforeBuildCommand）
19. 修改 lib.rs（窗口 URL 改 `/`，Rust 侧跳转改路径，platform_script 扩展为 `{ os, wsl }` 对象）
20. 验证：`npm run tauri dev` 启动，主窗口显示 App 骨架（label 未就绪空态），about 窗口加载同一入口不报错
21. 验证：`npm run tauri build` 成功；并在 **release 产物内触发一次 `/selector` 直达**，确认 SPA fallback 命中（§3.1 结论的实机钉板）

### 阶段 B：About 页面迁移

22. 创建 components/about/（ClientUpdateCard, DshVersionCard, NodeVersionCard）
23. 实现 pages/About.tsx（三维度版本 + 客户端更新状态机 + 按钮）
24. 接入 clientUpdateStore + bootStore
25. 验证：关于面板功能完整，客户端更新流程可用

### 阶段 C：Mode + Selector 页面迁移

26. 创建 hooks/usePlatform.ts（聚合 __DSH_PLATFORM__ → host 能力对象）
27. 实现 pages/BootMode.tsx（local/WSL 卡片 + 设为默认 + 下一步）
28. 实现 pages/BootSelector.tsx（profile 卡片列表 + 复用 DownloadProgress/ErrorCard/PulseBar）
29. 验证：运行环境选择 + 工作台选择器功能完整

### 阶段 D：Index 页面迁移（最复杂）

30. 完善 bootStore：步骤状态推演逻辑 + 速度计算 action
31. 实现 components/boot/BootTimeline + BootStep
32. 实现 components/boot/DownloadProgress
33. 实现 components/boot/ErrorCard（数据驱动渲染 actions[]/log 折叠面板，AnimatePresence 动画）
34. 实现 components/boot/VersionChip
35. 实现 components/boot/PulseBar
36. 实现 pages/BootIndex.tsx（组合所有组件 + 播种副作用）
37. 验证：启动序列全流程，下载进度，错误卡动作

### 阶段 E：测试与治理文件收口

38. 编写 Vitest 测试（format, bootProgress, bootStep, updatePhase）
39. 验证：`npm run test -- --run` 全绿
40. 删除 `ui/` 目录（前置检查：mark.svg 已在 frontend/public/；dsh-logo.svg 已迁至仓库根 assets/；app.css token 已全量进 @theme）
41. 修改 AGENTS.md（§1 / §2 / §3 品牌路径 / §4.2 / §4.3 + 新增 §4.4 三条红线 + §5 + §7 例外册口径）
42. 修改 CI（node 安装/依赖/构建 + typecheck + lint + vitest 闸门；与 roadmap §4.1 的 cargo fmt/clippy 步骤**同批接入 build.yml**）
43. 修订 docs/roadmap.md：硬约束 2 改写为「框架已裁定向 React 生态收敛（ADR-0008），仍禁数据库/IPC 总线/领域服务」；删除不做清单中「引入前端框架 / 构建器」一行
44. 全量验证：§9 功能验证清单全过
45. `cargo test` 全绿 + `npm run tauri build` 成功
46. 人肉读 git diff 确认无越界
47. 按 CONTRIBUTING 规范提交（宪法级文件单独 commit），公共频道广播完成通知并落档 broadcasts.md

---

## 11. 前瞻预留与触发器（防过度设计清单）

> 前瞻的正确姿势是留挂钩、不提前施工：以下每行都规定了一期姿态与「什么时候才允许立项升级」。

| 方向 | 一期姿态 | 触发器（满足才立项） |
|:---|:---|:---|
| 数据获取层 | 不引第三方；资源型异步一律经 `lib/resource` 统一入口约定 | 首个资源型管理页（Profile 列表）开工时立 micro-ADR 评审 TanStack Query（v5，与 React 19 兼容） |
| boot 步骤动态化 | UI 消费 `store.steps[]` 数组渲染，不写死步数；步表按 hostKind 从 zh-CN 选模板 | WSL v2 / SSH 需要变长启动序列时立 ADR 升级 `boot:start` 协议由 Rust 下发步表 |
| 平台/宿主能力 | `host.can()` 过滤动作可见性；SSH 相关能力显式 false | `ExecutorKind::Ssh` 接线时 `__DSH_PLATFORM__` 增补 `host` 维度；远端会话拒绝类动作（upgrade）接入同一过滤 |
| 进度 kind 字典 | DownloadProgress 按 kind 映射文案/图标，未知 kind 兜底渲染 | WSL 客体内安装、插件下载等新增 kind 出现时补字典项即可 |
| 虚拟列表 | 不做 | 会话管理器单列表规模达数百行时评估 @tanstack/react-virtual |
| IPC 类型自动生成 | 手写 types/ipc.ts | 沿用 ADR-0008 §6：前端超 8000 行评 tauri-specta |
| 主题切换 | token 全走变量 | 产品决定暗色方案时加 data-theme 覆盖层 |
| 包体预算 | build 产物 size 记入 PR 描述人工比对 | 超 500KB gzip 触发 ADR-0008 §6 复审 |

---

## 12. 风险与缓解

| 风险 | 缓解 |
|:---|:---|
| Tailwind v4 + shadcn/ui 不兼容 | ~~阶段 A 第 6 步先验证；不兼容降级 v3.4~~ **2026-08-27 临时目录 spike 验证通过，风险关闭**（ADR-0008 §6 复审条件保留） |
| 启动页面状态管理出 bug | bootStore 集中管理，先写 store 和 Vitest 测试再写组件；每写完一个组件手动验证 |
| React StrictMode 双调用导致重复监听 | 事件总线在 useEffect 中初始化并返回 cleanup |
| 事件丢失（listen 前 Rust 已发射） | 页面 useEffect 中主动调用 getUpdateStatus/getClientUpdate 获取初始状态 |
| Tauri dev 模式配置问题 | 阶段 A 先跑通空白页，确认 dev/build 都正常再迁移页面 |
| 包体积/冷启动 | 预估 gzip 60-80KB，桌面应用可忽略；如超 500KB 按 ADR-0008 §6 复审 |
| 协作者开发环境变化 | AGENTS.md 更新后公共频道广播；README 加 node 20+ 要求 |
