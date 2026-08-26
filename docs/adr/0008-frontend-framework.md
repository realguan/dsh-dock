# ADR-0008：前端框架引入——Vite + React + TypeScript

- **日期**：2026-08-26
- **状态**：已接受
- **提出人**：guan
- **相关方**：所有前端代码（`ui/` → `frontend/`）、`tauri.conf.json`、CI（`.github/workflows/build.yml`）、AGENTS.md §1/§2/§4
- **关联**：ADR-0001~0007（壳架构基础）；`docs/roadmap.md`（Next 阶段管理功能扩展）

---

## 1. 背景与问题

壳自带前端当前是 4 个纯静态 HTML/CSS/JS 页面（`ui/index.html` 启动序列、`mode.html` 运行环境选择、`selector.html` 工作台选择器、`about.html` 关于/更新中心），共约 2360 行，零构建器、零框架、零依赖。内联 JS 共 626 行，通过 `window.__TAURI__.core.invoke()` 直接调用 13 个 IPC 命令，通过 `window.__TAURI__.event.listen()` 消费 5 类事件。

路线图（`docs/roadmap.md`）规划了大量壳管理功能：Profile 管理器、插件管理器、设置可视化编辑器、会话/工作区管理器、MCP 服务器管理器、诊断工具等。这些功能交互复杂度远超现有启动辅助页：列表渲染、表单、模态框、标签页、跨页面状态共享、组件复用。经 dsh 源码探索确认，这些管理功能可通过纯文件读写 + dsh CLI 调用实现，不需要修改 dsh 源码，但前端复杂度将从 ~2360 行增长到 ~4000+ 行。

矛盾点：
- 继续用原生 JS：组件复用靠复制粘贴，状态管理靠全局变量/DOM 操作，事件监听分散在各页面，~4000 行原生 JS 可维护性差，AI coding 效率低。
- 引入框架：违反 AGENTS.md §4.2「禁止引入前端构建链」的现有约束，但能解决组件化、状态管理、类型安全问题。

## 2. 约束与硬指标

- **不修改 dsh 源码**：所有管理功能通过文件读写 + dsh CLI 实现，框架只服务壳自带页面。
- **`withGlobalTauri: true` 不可关闭**：`initialization_script` 注入到 dsh web UI 的脚本依赖 `window.__TAURI__.core.invoke('open_external')`，关闭会导致 dsh 页面外链点击静默失败。
- **Rust 侧 IPC 命令接口不变**：13 个命令和 5 类事件的接口不变，只改前端调用方式。
- **`initialization_script`（WebView 内存策略 + 外链兜底）不受影响**：它注入到 dsh web UI，不是壳页面。
- **壳保持薄**：不引入数据库 / IPC 总线 / 领域服务；框架仅限 UI 层。
- **三平台 CI 必须全绿**：macOS / Windows / Linux 构建和测试通过。
- **品牌规则不变**：官方徽章用 `mark.svg` + CSS mask，禁止手绘 logo。
- **零数据库**：前端状态在内存中（Zustand），不引入持久化 store。

## 3. 备选方案及评估

### 方案 A：Vite + React 19 + TypeScript + Tailwind v4 + shadcn/ui —— ✅ 最终采纳

- 思路：新建 `frontend/` 目录，Vite 6 构建，React 19 + TS strict，Tailwind CSS v4（CSS-first `@theme` 配置），shadcn/ui 按需添加组件，React Router v7 路由，Zustand 状态管理，Framer Motion 动画，Lucide React 图标。所有 4 个现有页面一次性迁移，`ui/` 目录删除，`frontendDist` 指向 `frontend/dist`。
- 优点：
  - 组件化解决复用问题（DownloadProgress、ErrorCard、VersionChip 等在 index/selector 间共享）。
  - Zustand 集中管理启动状态（boot:step/progress/error/update），事件总线统一初始化，消除各页面分散监听。
  - TypeScript strict 提供类型安全，IPC 命令封装层有完整类型。
  - Tailwind 原子化 CSS + shadcn/ui 组件，AI coding 生成准确率高。
  - 一次性全量迁移，不留双轨制技术债。
  - Vite HMR 提升开发体验。
- 代价/风险：
  - CI 增加 node 20+ 步骤，构建时间增加（Vite 构建 < 10s，可接受）。
  - 包体积增加（React + Router + Zustand + Framer Motion gzip 约 60-80KB，桌面应用可忽略）。
  - Tailwind v4 与 shadcn/ui 的兼容性需执行前验证；如不兼容，降级到 Tailwind v3.4（v3→v4 升级成本低）。
  - AGENTS.md §1/§2/§4 需要修改（前端约束从「零构建」改为「Vite + React」）。
  - 冷启动首帧比纯静态 HTML 慢几十毫秒（桌面应用可接受）。
- 对照约束：
  - 不修改 dsh 源码 ✅（框架只服务壳页面）
  - withGlobalTauri 保持 true ✅（React 用 @tauri-apps/api import，全局注入保留给 dsh web UI）
  - IPC 接口不变 ✅（只改前端调用封装）
  - initialization_script 不受影响 ✅（注入到 dsh web UI）
  - 壳保持薄 ✅（无数据库/IPC 总线/领域服务，框架仅限 UI 层）
  - 三平台 CI ✅（加 node 步骤）
  - 品牌规则 ✅（Emblem 组件用 mark.svg + CSS mask）
  - 零数据库 ✅（Zustand 内存状态，不持久化）

### 方案 B：渐进引入——管理页面用框架，现有页面保留静态 —— ❌ 否决

- 思路：新建 `frontend/` 只做管理页面，Vite 构建产物输出到 `ui/manager/`，现有 4 个静态页面不动，`frontendDist` 仍指向 `ui/`。
- 否决理由：
  - 双轨制技术债：两套技术栈（静态 HTML vs React）长期共存，协作者需要同时维护两种范式，组件无法复用（如 ErrorCard 在静态页和 React 页各写一份）。
  - 路线图管理功能多，最终所有壳页面都会需要组件化，渐进只是推迟全量迁移，不是避免。
  - 用户明确要求「从现在引入框架，避免后面遗留技术债」。

### 方案 C：轻量方案——Preact + htm，零构建 —— ❌ 否决

- 思路：用 Preact（3KB）+ htm（JSX 替代，无需构建），保持零构建器定位，获得组件化能力。
- 否决理由：
  - 没有 shadcn/ui + Tailwind 生态，AI coding 效率远低于 React + Tailwind。
  - htm 的模板字符串语法不如 JSX 直观，AI 生成准确率低。
  - 不解决 TypeScript 类型安全问题。
  - 组件生态弱，复杂交互（模态框、标签页、虚拟列表）需要自己写。

### 方案 D：继续原生 JS + 轻量组件化模式 —— ❌ 否决

- 思路：不引入框架，用 ES Module + 自定义元素/工厂函数组织代码，共享组件放到 `ui/assets/`。
- 否决理由：
  - 无法解决状态管理问题（启动状态跨页面共享需要手写发布订阅）。
  - 无 TypeScript 类型安全，IPC 调用参数/返回值无校验。
  - 4000+ 行原生 JS 的 DOM 操作代码可维护性差，AI coding 效率低。
  - 路线图功能越多，原生 JS 的劣势越大。

## 4. 最终决策

采用方案 A：新建 `frontend/` 目录，Vite 6 + React 19 + TypeScript strict + Tailwind CSS v4 + shadcn/ui + React Router v7 + Zustand + Framer Motion + Lucide React，一次性迁移全部 4 个现有页面，删除 `ui/` 目录。多窗口路由采用窗口 label 方案（所有窗口加载 `/`，React 根据 `getCurrentWindow().label` 渲染对应页面）。前端测试用 Vitest 覆盖关键逻辑（速度计算、步骤推演、格式化函数），不测 UI 渲染。颜色走 Tailwind theme token、文本抽离到常量文件，为暗色模式和 i18n 预留架构。详细实施计划见 `docs/frontend-migration.md`。

## 5. 后果与后续行动项

### 正面后果
- 组件化和状态管理为路线图中的管理功能（Profile/插件/设置/会话管理）铺平道路。
- TypeScript 类型安全减少 IPC 调用错误。
- Tailwind + shadcn/ui 提升 AI coding 效率和 UI 一致性。
- Vite HMR 提升开发体验。
- 事件总线统一初始化，消除分散监听的重复注册风险。

### 负面后果 / 新增债务
- CI 增加 node 20+ 依赖和构建步骤（之前纯 Rust）。
- 协作者需要 node 20+ 开发环境（之前只需要 Rust toolchain）。
- Tailwind v4 与 shadcn/ui 兼容性存在不确定性，执行前需验证，不兼容则降级 v3.4。
- 包体积增加约 60-80KB gzip（桌面应用可忽略）。
- AGENTS.md §4.2「禁止前端构建链」约束需要修订，宪法变更需所有协作者知晓。

### 行动项
- [ ] 执行前验证 Tailwind v4 + shadcn/ui 兼容性；不兼容则降级 v3.4（负责人：guan）
- [ ] 按 `docs/frontend-migration.md` 执行迁移：脚手架 → About → Mode → Selector → Index → 清理
- [ ] 修改 AGENTS.md §1（技术栈表）/ §2（目录结构）/ §4.2（禁止项）/ §4.3（UI 规范），新增 §4.4（前端开发规范）
- [ ] CI 加 node 20+ 步骤（`.github/workflows/build.yml`）
- [ ] 删除 `ui/` 目录
- [ ] `cargo test` 全绿 + `npm run tauri build` 三平台成功 + 功能验证清单全过
- [ ] 公共频道广播宪法变更

## 6. 复审条件

- Tailwind v4 与 shadcn/ui 兼容性验证失败且降级 v3.4 后仍有问题 → 复评是否换用其他 CSS 方案（如 CSS Modules + Radix UI）。
- React 19 或 Vite 6 出现 Tauri 不兼容的重大问题 → 复评是否降级 React 18 / Vite 5。
- 迁移后前端包体积超过 500KB gzip（当前预估 60-80KB）→ 复评是否需要代码分割或移除 Framer Motion。
- 路线图管理功能全部完成后，如果前端代码量超过 8000 行且组件复用成为主要痛点 → 复评是否需要 tauri-specta 类型安全绑定。
- Tauri 未来版本原生支持 SPA fallback 或多窗口路由 → 复评窗口 label 路由方案。
