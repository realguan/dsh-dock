# 📢 广播记录（公共频道知会档案）

> 协作知会的**仓库内正式载体**。依据 [`CONTRIBUTING.md`](./CONTRIBUTING.md) §0——
> 「所有必须共享的信息，唯一合法载体是仓库落盘内容」：聊天频道用于即时协调
> （占用抢先后到者得），但消息沉底即失忆、AI 冷启动读不到；知会类事件必须在此
> 落档。检索、审计、纠纷回溯一律以本档为准。
> 本文件是普通区 append-only 文档：只追加不改写历史条目（同 ADR 惯例），
> 不走宪法级修改流程。

## 一、登记范围（触发即记，当天落档）

| 类型 | 触发点 | 依据 |
|:---|:---|:---|
| 宪法级文件改动 | `AGENTS.md` / `docs/contract.md` / `CONTRIBUTING.md` / `node-map/` 的改动预告与合入归档 | AGENTS §10；CONTRIBUTING §3 |
| 快车道直推 master | 直合完成即记（单人小改通道同样适用） | CONTRIBUTING §2 流程图 I→M |
| PR 合并完成 | 合并人 squash 后记录 reviewer 与结论 | CONTRIBUTING §2 流程图 M |
| 发版事项 | 打 tag、冻结期起止、Release notes 征集与三平台验收 | CONTRIBUTING §8 |
| 占用声明/释放 | 频道声明后补一行即可（时效判定仍以频道时间戳为准） | CONTRIBUTING §3 |

## 二、条目格式

倒序追加（最新在上），一条一个三级标题：

```
### YYYY-MM-DD <类型> · <一句话主题> —— <发起人>
- 变更：<commit hash / 分支 / 文件清单>，两三行摘要
- 影响：<需要他人做什么动作；无需动作写「仅周知」>
- 凭据：测试结果 / diff 规模 / 频道消息时间点
```

漏记不补改旧条目——另发一条「补记」并注明原委。

## 三、记录

### 2026-08-31 发版通知 · v0.9.1 社区插件市场全量上线（awesome-dsh-plugin Registry 2700+ 插件发现与一键安装分发） —— guan（AI 协作）

- 变更：
  1. **社区插件市场（Registry 集成）**：`src-tauri/src/updates.rs` 接入 `awesome-dsh-plugin.com/plugins.json` 官方 Registry（单一网络面，带 3MB 上限与 GitHub raw 镜像兜底），新增 `fetch_market_registry` IPC 命令与外链域名白名单登记。
  2. **高质感插件市场工作台（`/frontend-design`）**：`ProfileManager.tsx` 新增第 5 个视图 Tab（`market`），引入 `MarketplaceView.tsx`、`MarketPluginCard.tsx` 与 `MarketInstallDialog.tsx`。
  3. **丰富发现与筛选能力**：支持全局搜索（名称/NPM/作者/描述）、22 个垂直分类快速筛选 Chips、四向排序（Stars ⭐ / Downloads ⬇️ / 最新 🆕 / 名称 🔤）与已安装插件过滤。
  4. **全域状态联动与一键分发**：插件卡片与全域已装 Profile 芯片实时联动（带绿色脉冲圆点），支持一键安装到指定 Profile 或分发至其他 Profile。
  5. **国际化与单测补齐**：中英多语言字典全量支持；新增 `market.test.ts` 14 个单测，全仓 166 Rust 单测 + 85 前端单测全绿。
- 影响：仅周知。用户可在 DSH Dock 内直接浏览、检索与安装 2700+ 社区插件。
- 凭据：`cargo test` 166 个单测全绿；前端 `npm run typecheck && npm run lint && npm run test && npm run build` 全量通过。

### 2026-08-31 完成通知 · 问题 9 & 10 深度优化（本地路径打开修复 + 系统控制台极简侧栏导航重构） —— guan（AI 协作）

- 变更：
  1. **问题 9（本地路径打开）**：`src-tauri/src/lib.rs` 升级 `open_external` 命令，判断参数为本地文件系统路径时调用系统文件管理器（Finder / File Explorer）直接打开该路径或其父目录，彻底解决之前被 Web URL 白名单拦截报错「不允许的外链」的问题。
  2. **问题 10（两层导航极简重构）**：严格遵循 `/frontend-design` 规范对 [`SystemConsole.tsx`](file:///Users/guan/git/realguan/dsh-plugin-hub/dsh-dock/frontend/src/components/system/SystemConsole.tsx) 进行极简与去噪重构：彻底移除左侧顶部冗余啰嗦的长句描述，将导航项升级为极简高级的轻量侧栏（单行微底色 Icon + 纯粹标题 + Active 边框态），消除多余边框包裹与视觉杂讯，与整体界面实现极致协调统一。
- 影响：仅周知。会话维护与系统控制台操作体验与视觉质感显著提升。
- 凭据：`cargo test` 166 个单测全绿；前端 `npm run typecheck && npm run test` 12 个测试套件 71 个单测全绿。

### 2026-08-31 完成通知 · 自动化发布日志流水线（GitHub Release & 客户端「关于」更新说明无缝打通） —— guan（AI 协作）

- 变更：全链路打通 GitHub Release 与桌面客户端自更新日志：
  1. **自动化发布日志提取引擎**：新增 [`scripts/extract-release-notes.py`](file:///Users/guan/git/realguan/dsh-plugin-hub/dsh-dock/scripts/extract-release-notes.py)，在打 tag 发布时自动从 `docs/broadcasts.md` 提取匹配当前版本的权威变更条目（包含变更要点、架构决议与测试凭据），并支持回退到 `git log`。
  2. **GitHub Actions 构建流无缝注入**：重构 [`.github/workflows/build.yml`](file:///Users/guan/git/realguan/dsh-plugin-hub/dsh-dock/.github/workflows/build.yml) 中的 `release` 任务，自动将提取出的 Markdown 升级日志写入 GitHub Release `body_path`，并注入到桌面自更新清单 `latest.json` 的 `notes` 字段中（彻底解决之前固定硬编码 `"DSH Dock v0.x"` 导致客户端无更新日志的缺陷）。
  3. **客户端「关于」更新卡片体验增强**：[`ClientUpdateCard.tsx`](file:///Users/guan/git/realguan/dsh-plugin-hub/dsh-dock/frontend/src/components/about/ClientUpdateCard.tsx) 升级 Release Notes 视窗，支持滚动阅读与一键「展开全部 / 收起」切换，让用户在应用内检查更新时能够清晰浏览完整的更新内容。
- 影响：仅周知。后续所有通过 git tag 触发的 CI 发布将全自动生成详尽的 GitHub Release 页面，且客户端关于窗口在检查到新版本时能完整呈现版本升级日志。
- 凭据：本地执行 `python3 scripts/extract-release-notes.py` 验证通过；`cargo test` 166 个单测全绿；前端 `npm run typecheck && npm run test` 12 个测试套件 71 个单测全绿。

### 2026-08-31 完成通知 · 8 项深度用户体验与逻辑缺陷全量优化（Bento 宫格插件市场、智能贪婪路径反解、凭据元数据过滤、诊断大盘缓存、命名升级控制中心） —— guan（AI 协作）

- 变更：全面落实 `问题记录.md` 中的 8 大反馈与优化建议：
  1. **新建 Profile 按钮布局**：侧边栏顶部独立设计高亮醒目的 `+ 新建工作台` 主操作按钮，与下方搜索框形成清晰主次操作流。
  2. **重置依赖感知增强**：明确弹窗文案（彻底清理 `node_modules` 并基于 `package.json` 通过 pnpm 纯净重装），执行态展示 Spinner 与防重点击，Toast 明确返回重置结果。
  3. **插件分发 Select 截断修复**：消除 Radix `SelectTrigger` 嵌套 span 截断问题，设置 `min-w-[240px]` 完整展示 Profile 名称。
  4. **插件总览 Bento 宫格卡片 + 分页 + Profile 筛选**：`PluginOverview.tsx` 重构为现代自适应 Bento Grid 宫格卡片，新增 Profile 专属下拉筛选器与分页控制器（支持 6/9/12/18 条切换与页码导航）。
  5. **会话项目分组、路径贪婪反解与 Icon 纠正**：`sessions.rs` 引入文件系统智能探测贪婪匹配算法，精准还原带连字符真实物理路径（`/Users/guan/git/realguan/dsh-plugin-hub/dsh-dock`）并提取简洁项目名（`dsh-dock`）；`SessionManager.tsx` 规整卡片上下双层结构，纠正复制会话路径 Icon 为 `Copy`。
  6. **凭据元数据过滤**：`credentials.rs` 引入 `RESERVED_METADATA_KEYS` 严格过滤 `version`、`refs`、`schema` 等非 Provider 元数据键，新增专属单测。
  7. **健康大盘缓存与 Icon 纠正**：`DiagnosticsPane.tsx` 引入 60 秒内存缓存机制实现秒开无感切换，保留手动「刷新体检」按钮；复制诊断报告按钮纠正为 `Copy` 图标。
  8. **产品命名统一升级**：程序菜单栏与独立窗口标题由「Profile 管理器」统一升级为更名副其实的「控制中心 (Control Center)」。
- 影响：仅周知。全栈用户体验与交互质感大幅提升。
- 凭据：`cargo test` 166 个单测全绿；前端 `npm run typecheck && npm run test` 12 个测试套件 71 个单测全绿，0 报错。

### 2026-08-31 完成通知 · v0.9.0 深度对齐审核报告与全量单元测试补齐（165 Rust 单测 + 71 前端单测全绿） —— guan（AI 协作）

- 变更：全量落实《DSH Dock v0.9.0 规划与可行性深度审核报告》的核心设计建议与落地红线，并补齐全部功能的边界单测：
  1. **4.5 凭据脱敏与 DSH 引擎全局设置**：Rust 新增 `dsh_settings.rs`（`settings.yaml` 安全原子读写）与 `credentials.rs`（脱敏掩码算法 `mask_api_key`、`get_credentials_summary`、`set_credential_key` 0600 权限）；前端 `CredentialsPane.tsx` 升级为结构化卡片与独立修改弹窗，新增 `DshSettingsPane.tsx` 管理 DSH 核心引擎配置。补齐掩码边界、清除/删除、多 Provider 并存单测。
  2. **4.6 会话与工作区真实路径联动**：Rust `sessions.rs` 实现 `decode_project_dir_to_path`，精准反解真实工作区绝对路径（含 Windows 盘符与 Unix 路径）；前端 `SessionManager.tsx` 支持按项目聚合分组折叠与一键在访达/资源管理器中打开项目。补齐各种异常目录名反解与序列倒退自愈重排备份单测。
  3. **4.7 MCP 服务器可视化结构化 CRUD 与运行态联动**：Rust `mcp.rs` 实现针对 `@deepseek-ai/dsh-mcp-client` 的结构化 CRUD，通过内存合并避免 Cordis Patch 覆盖非 MCP 插件条目；前端 `McpManager.tsx` 支持 GitHub/Filesystem/Postgres/Brave Search 预设一键应用、完整表单编辑与运行态 `mcp__<server>__*` 工具提取联动。补齐保留非 MCP 插件条目、更新已有 server、删除幂等单测。
  4. **4.11 / 4.12 诊断体检与多源日志流**：Rust `diagnostics.rs` 探测 Node/pnpm/dsh 运行状态并收集存储水位，提供多源日志安全尾部截断 `get_app_logs`；补齐各 source 路由与未截断/截断单测。
  5. **4.13 多语言深度对齐**：前端 `i18nStore.test.ts` 采用全量递归深度遍历断言，确保 `zh-CN.ts` 与 `en-US.ts` 所有层级 key 100% 深度对称无漏项。
- 影响：仅周知。全栈代码质量与测试覆盖率达到最高标准，保证所有新增功能与边缘场景均有机器单测严格守护。
- 凭据：`cargo test` 165 个测试全绿（165 passed, 0 failed）；前端 `npm run typecheck && npm run test` 12 个测试套件 71 个单测全绿（71 passed, 0 failed, 0 type errors）。

### 2026-08-31 完成通知 · v0.9.0 规划全量落地：稳定性守护、系统控制台与运维大盘、多语言基线、MCP 扩展与依赖重置 —— guan（AI 协作）

- 变更：全量三阶段（刀 1/2/3）落地收口：
  1. **稳定性、运维与多语言（4.11 + 4.12 + 4.13）**：Rust 接入崩溃守护熔断器 `guard_session`（60s 内 3 次崩溃触发熔断与诊断提示）；实现了 `diagnostics.rs`（系统环境健康体检、Node/pnpm/dsh 探测、存储分布递归统计、安全分页日志查看器 `get_app_logs`）；新增持久化字段 `locale` 与 `autoRestart`；前端落地完整中英双语国际化 `i18nStore` 与响应式翻译。
  2. **会话与设置大盘（4.6 + 4.5）**：实现 `credentials.rs`（`.credentials.yaml` 0600 安全权限原子写与脱敏查看）；实现 `remove_session` 物理删除会话与路径快捷复制；前端构建全新控制中心 `SystemConsole.tsx`（偏好设置、凭据安全编辑器、系统健康大盘、暗色终端日志视窗）。
  3. **MCP 生态与维护收口（4.7 + 4.11）**：实现 Profile 依赖一键重置 `reset_profile_dependencies`；前端实现 `McpManager.tsx` 可视化 MCP 服务器管理（支持 GitHub / Postgres / Brave Search / Filesystem 等预设与工具前缀一键复制）。
  - 新增 8 个 IPC 命令严格完成三处同步与 `AGENTS.md` 登记；全部通过 `gate_tests` 机器闸门。
- 影响：仅周知。v0.9.0 规划的全部目标均已高质量闭环交付，所有前端界面均严格遵循 `/frontend-design` 规范打造，兼具高质感设计与可靠健壮性。
- 凭据：Rust 侧 158 单元测试全部通过（158 passed, 0 failed）；前端 58 单元测试全部通过（58 passed, 0 failed, 0 type errors）。

### 2026-08-31 修复 · WebView 内存策略改用直接子代选择器（解决工具调用嵌套导致的大面积滚动空白与输入框悬空）—— guan（AI 协作）

- 变更：本 commit——`src-tauri/src/lib.rs`（`WEBVIEW_MEMORY_POLICY_SCRIPT` 中 CSS 规则由后代选择器 `FLOW + ' ' + ROW` 改为直接子代选择器 `FLOW + ' > ' + ROW`；`syncStreaming` 仅扫描顶层行；更新单测 `webview_memory_policy_script_contains_list_padding_defense` 断言直接子代连接符 `FLOW + ' > ' + ROW`）、`docs/adr/0002-webview-memory-policy.md`（补录直接子代修订说明）。
- 影响：仅周知。解决由于工具调用内部卡片（`ToolCallTree` 的 `callRow` / `subCalls`）带有相同的 `data-chat-anchor-key` 属性而触发多层嵌套 containment，导致 WebKit 严重虚高估算滚动高度、在内容到底后仍可下滑并露出大片空白、输入框脱离底部悬空的偶现 bug。
- 凭据：`cargo test` 152 绿 / `cargo fmt --check` / `cargo clippy -- -D warnings` 全绿；前端 test 55 绿。

### 2026-08-31 功能 · 会话自愈与健康维护面板（ProfileManager 会话工作台）—— guan（AI 协作）

- 变更：本 commit——新增 `src-tauri/src/sessions.rs`（扫描 `$DSH_HOME/sessions/`、单会话/全量自愈）、`scripts/repair-session.mjs`（会话日志 Turn 归流、seq 连续化重排与 Zstd 校验重打包）；`ipc.rs` / `lib.rs` / `capabilities/default.json`（登记 `list_sessions` / `repair_session` / `repair_all_sessions`）；`AGENTS.md` §7；`frontend/src/components/profiles/SessionManager.tsx`；`frontend/src/pages/ProfileManager.tsx`（三视图切换新增「会话维护」）；`frontend/src/content/zh-CN.ts` 与 `frontend/src/types/ipc.ts`。
- 影响：**触宪法级**——`AGENTS.md` §7 登记 3 项新 IPC 命令（IPC 三处同步严格保持一致）。不破坏 dsh 文件系统不变量，修复时自动备份原始 `.bak`，解决上游并发推流或断线重连导致的 "seq gap in committed region" / "history unavailable"。
- 凭据：`cargo test` 152 绿（新增 3 单测）/ `cargo fmt --check` / `cargo clippy -- -D warnings` 全绿；前端 typecheck 0 报错 / test 55 绿 / build 产物成功生成。

### 2026-08-31 修复 · WebView 内存策略 CSS 补齐列表内边距（解决 Paint Containment 裁切列表序号）—— guan（AI 协作）

- 变更：本 commit——`src-tauri/src/lib.rs`（`WEBVIEW_MEMORY_POLICY_SCRIPT` 常量化，注入 CSS 追加 `FLOW ol, FLOW ul { padding-left: 1.5em !important; }` 防止 `content-visibility: auto` 隐式 Paint Containment 把挂在行左侧外沿的有序/无序列表 markers 裁切截断；增加单测 `webview_memory_policy_script_contains_list_padding_defense`）、`docs/adr/0002-webview-memory-policy.md`（补录 2026-08-31 修订说明）。
- 影响：仅周知。保持长会话 WebKit 内存优化不变，彻底解决桌面端会话中有序列表序号（如 `1. 2. 3.`）只展示一半的问题。
- 凭据：`cargo test` 149 绿（+1 单测）/ `cargo fmt --check` / `cargo clippy -- -D warnings` 全绿；前端 test 55 绿。

### 2026-08-31 修复 + 发版事项 · 插件总览 UX 修正（包名展示全 / 描述降级）+ v0.8.0 tag（4.4 收口）—— guan（AI 协作）

- 变更：本 commit——PluginOverview 行布局重排（维护者实机截图反馈：包名被
  描述挤压截断、描述喧宾夺主）：包名独占整行 `break-all` 尽量展示齐全（超长
  折行不截断）、分布 chips 次行、描述降级为末行单行截断辅助信息；随后
  `chore: 版本 0.8.0` + 注解 tag `v0.8.0`，master 与 tag 推送。
- 影响：**冻结期重启**——v0.8.0 为新冻结点：Release notes 至三平台产物验收
  期间 master 只收 fix。Release notes 草稿见附录（覆盖 v0.7.0 → v0.8.0 全量：
  4.4 收口 ade90bd、升级链路事件化 83fa74b、可安装性预检 704f7ff、本 UX 修正）。
- 凭据：前端 typecheck / lint / test 55 绿（Rust 侧本批零改动，0.8.0 前全量
  147 绿见 ade90bd 凭据）。

**附录：Release notes 草稿（v0.7.0 → v0.8.0）**

```markdown
## 新增
- 插件总览：Profile 管理器页内新视图，聚合展示全部 profile 的第三方插件
  分布——包名独占整行尽量展示齐全，各 profile 实装版本一目了然（只读，
  纯本地文件扫描）
- 从其他 profile 安装插件：详情对话框「从其他导入」多选批量，默认安装来源
  同版本（装验证过能用的）；串行队列单项失败不中断，末尾汇总成败与明细
- 「连配置」可选搬移：勾选后把来源 profile 里该插件的配置行原样复制到目标
  （默认不勾；只追加不覆盖目标已有配置，ADR-0009 写入例外 #4）

## 修复
- DSH 升级「点了没反应」：升级链路全程事件反馈——真实进度与失败详情可见
- 升级前可安装性预检：目标版本依赖未完整发布时立即失败并指名缺失包，
  不再等数分钟后才失败
- 插件总览包名被描述挤压截断（包名独占整行、描述降级末行）

## 变更
- 插件管理器五个核心能力收口：清单 / 安装卸载更新 / 禁用启用 / 更新标识与
  选版本 / 跨 profile 安装
- AGENTS §7 IPC 名册 +2（list_all_plugins / copy_plugin_config）

## 已知问题
- 插件安装进度为单行 busy（pnpm 流式输出未回传）
- npm 搜索未实现（挂账）
```

### 2026-08-30 完成通知 · 4.4 收口：插件总览聚合 + 从其他 profile 安装（ADR-0009 第五次修订，patch 写入例外 #4）—— guan（AI 协作）

- 变更：本 commit——维护者经 grilling 逐条确认后把原「跨 profile 复制」重定义为
  ①**插件总览**（管理器页内切换视图，`list_all_plugins` 只读文件扫描聚合全部
  已物化 profile 的第三方插件，内置 bundle 不进聚合）与②**从其他 profile 安装**
  （详情对话框多选批量选择器，串行队列失败继续末尾汇总，版本默认来源同版本
  `pkg@ver`；「连配置」逐行可选勾选=来源 cordis.patch.yml 该插件行 id 全部条目
  原样复制，`copy_plugin_config` 只追加不覆盖——**写入例外 #4**，`PluginRowState`
  扩展 `patch_entries` 供置灰预检）。新 IPC 两枚三处同步 + AGENTS §7 登记；
  ADR-0009 第五次修订先行落档；roadmap 4.4 落地记录回写（五核心能力全齐）。
- 影响：仅周知。npm 搜索（4.4⑤）维持挂账；注意 `updates.rs` 尚有另一条工作线的
  H-1 预检未提交改动，本 commit 未包含、未触碰。
- 凭据：cargo test 147 绿（新增聚合归组 / 配置行原样复制与不覆盖 / patch 条目
  计数等 4 测试）+ fmt/clippy 干净；前端 55 测试绿（新增候选过滤与批量汇总
  纯逻辑）+ typecheck/lint 干净；IPC 一致性 gate_tests 兜底。

### 2026-08-30 排障 + 修复 · DSH 升级「点了没反应」——根因 = 不可安装的 alpha 版本 + 失败不可见；升级链路事件化 —— guan（AI 协作）

- 变更：本 commit——`lib.rs`（`terminal_action` 升级链路新增 `dsh:upgrade`
  事件 running/done/failed，failed 携带安装器完整错误链含 pnpm 输出尾部），
  `DshVersionCard`（升级 busy 改事件驱动真实时长，失败显示错误详情——原 2s
  固定假 busy，之后数分钟全程无反馈）、`ClientUpdateCard`（检查更新按钮
  busy 时禁用 + 内联转圈——原实现整组消失，无动效）。
- 影响：仅周知 + **一项裁定待议**：根因是 H-1 检查口径（排序最高，rc/预发布
  也追）会把 `0.1.2-alpha.2` 这种**依赖未发布的不可安装版本**提示为「有新版」
  （ledger 已记边界）。候选：a) 检查侧做可安装性预校验（每依赖一查，成本高）；
  b) 口径改 dist-tag 优先（推翻 H-1，需裁定）。现维持 H-1 + 失败可见。Windows
  侧注意：`install_global_dsh_with_prefix` 的回退链未变。
- 凭据：根因实机复现——shell.log 14:41-14:49 多次 `pnpm add -g
  @deepseek-ai/dsh@0.1.2-alpha.2` 双 registry 全败；手工复现同错
  （ERR_PNPM_META_FETCH_FAIL，依赖走死域名 r.cnpmjs.org）。cargo test 142 绿 /
  fmt / clippy；前端 typecheck / lint / test 49 绿。

### 2026-08-29 发版事项 · v0.7.0 tag（插件管理器全量 + Profile 重启），冻结期随验收重启 —— guan（AI 协作）

- 变更：本 commit——详情对话框行内操作防溢出修复（hover 操作组与运行徽标
  display 换位，长包名截断兜底）+ roadmap 4.4 落地记录回写；随后
  `chore: 版本 0.7.0` + 注解 tag `v0.7.0`，master 与 tag 推送。
- 影响：**冻结期重启**——v0.6.0 冻结期经维护者裁定提前开工 feat（见当日
  4.4① 批次条目），本 tag 即新的冻结点：Release notes 至三平台产物验收
  期间 master 只收 fix。Release notes 草稿见附录。
- 凭据：前端 typecheck / lint / test 49 绿；Rust 142 / fmt / clippy 全过。

**附录：Release notes 草稿（v0.6.0 → v0.7.0）**

```markdown
## 新增
- Profile 重启按钮（运行中行内，确认后同 profile 重启——插件变更借此生效）
- 插件安装 / 卸载 / 更新：详情对话框行内操作（dsh plugin 转发链，规格校验
  防参数注入，安装/卸载/更新均带「重启后生效」提示）
- 插件禁用 / 启用：cordis.patch.yml `{id, disabled}` 单键切换（ADR-0009
  第四次修订，写入例外 #3），行 id 经 dump-config 权威解析
- 插件更新标识 + 选版本更新：registry dist-tags 口径（镜像链 npmmirror →
  npmjs），版本选择弹窗标 最新/当前
- Profile 详情对话框：插件清单卡（内置/外挂、实装版本、运行态徽标、
  会话运行汇总）、多插件防溢出（对话框限高 + 分区滚动）

## 修复
- 行内操作组被卡片右缘裁切（hover 与徽标 display 换位 + 长包名截断）
- 多插件时详情对话框超出屏幕（基件无高度上限）

## 变更
- 外挂插件在徽章区与卡片区去重显示（dsh reconcile 数据模型本然双写）
- AGENTS §7 IPC 名册 +9、网络面 +2（回环运行态查询、registry 外网检查）

## 已知问题
- 安装进度为单行 busy（pnpm 流式输出未回传）
- 跨 profile 插件复制未实现（4.4 收尾项）
```

### 2026-08-29 完成通知 · 4.4④ 插件更新标识 + 选版本更新落地（registry 外网镜像链）—— guan（AI 协作）

- 变更：本 commit——`updates.rs`（`npm_packument_versions`：任意 npm 包
  packument 查询，与 dsh 版本检查同镜像链 npmmirror → npmjs / 同超时 / 同
  体积上限 + `parse_packument_versions` 纯函数 semver 升序排序 + 测试）、
  `plugins.rs`（`check_updates_blocking`：逐外挂插件查 dist-tags.latest——
  口径与 pnpm 默认安装一致，复现点 7 教训；current ≥ latest 不报；奇异名
  跳过不打 registry；`plugin_versions_blocking` 全版本降序）、`lib.rs`/
  `ipc.rs`/`capabilities`（`check_plugin_updates` / `list_plugin_versions`
  三处同步）、前端（外挂插件区「检查更新」按钮 + 行内 `0.16.1 ↑0.17.0`
  更新标识，点开版本选择弹窗——标 最新/当前，选定走既有安装链
  `pkg@version`）。
- 影响：**触宪法级**——`AGENTS.md` §7 两处：IPC 名册 +2；网络面登记新
  外网用途「插件更新检查」（registry packument，同镜像链）。本条即知会。
  语义：检查为按钮触发不自动跑（N 包串行查询，避免开窗即外网风暴）；
  dist-tag 口径意味着 dsh-base 那种「latest 停在坏版本」的包不会被误标
  升级目标之外——选版本弹窗可见全部版本自行决定。
- 凭据：cargo test 142 绿（+2 packument 解析含 semver 排序反例）/ fmt /
  clippy 全过；前端 typecheck / lint / test 49 绿。

### 2026-08-29 完成通知 · 4.4③ 禁用/启用插件落地（patch 单键切换，ADR 第四次修订实施）—— guan（AI 协作）

- 变更：本 commit——`Cargo.toml`（新增 `serde_yaml 0.9`，ADR 已裁定接受停维护
  风险，读写收敛在 `set_plugin_disabled` 单函数便于后继替换）、`plugins.rs`
  （`plugin_rows_blocking`：`dsh --profile <名> --dump-config` 行 id↔包名配对 +
  壳 toggle 态，行级扫描避开 `!!js` 标签；`set_plugin_disabled`：patch 顶层数组
  读改写——禁用置/追加 disabled 键、启用移键或整条移除，头部注释块保真，
  非顶层数组拒绝写入）、`lib.rs`/`ipc.rs`/`capabilities`（`get_plugin_rows` /
  `set_plugin_disabled` 三处同步）、前端（行内电源开关：禁用态常驻灰徽 +
  包名划线，运行徽标只对启用中插件显示；操作带「重启后生效」提示）。
- 影响：仅周知。行 id 权威来源 = dump-config 行表（一次 spawn 秒级，对话框
  打开时异步取）；重启按钮承接生效。
- 凭据：cargo test 140 绿（+3：toggle 幂等与注释保真、config 键保全、
  dump 行配对解析）/ fmt / clippy 全过；前端 typecheck / lint / test 49 绿。
  实机数据锚定见 ADR 第四次修订注。

### 2026-08-29 完成通知 · Profile 重启按钮 + 详情去重 + 禁用/启用 ADR（维护者四项指令批次 1/2）—— guan（AI 协作）

- 变更：本 commit——① 重启按钮（运行中行 RotateCw，同 profile 走切换链
  `switch_profile`，恒弹确认，弹窗文案按重启语义分叉）；④ 详情对话框去重
  （reconcile 把外挂同时写进 bundles 与 dependencies——徽章区只留层叠内置
  层，隐藏数 >0 给指引行，台账复现点 7）；③ 决策先行：**ADR-0009 第四次
  修订**——patch 写入例外 #3（`cordis.patch.yml` 的 `{id, disabled}` 单键
  切换）：行 id 不可从包名推导（实测 commandcode→`llm-commandcode`），来源
  定死 dump-config 行表；serde_yaml 0.9 读改写（注释头部保真策略）；运行中
  不热生效、重启承接；生效真相 = 壳自家 patch 条目。
- 影响：仅周知；③ 的**实现**（serde_yaml 依赖 + IPC + 行内开关 UI）随后续
  commit 落地，② 更新标识（npm registry 外网查询，§7 需新登记）排最后。
- 凭据：前端 typecheck / lint / test 49 绿；行 id 映射实测锚定（本机 web 档
  dump-config）。

### 2026-08-29 完成通知 · 4.4② 插件安装/卸载/更新落地：详情对话框行内操作 —— guan（AI 协作）

- 变更：本 commit——`plugins.rs`（`validate_plugin_spec` 纯校验：防 pnpm 旗标
  注入（前导 `-` 当参数）、控制字符/空白；scope 包名与版本段（tag/精确/^~
  区间）放行，`><` 语义区间 v1 不开 + `mutate_plugin_blocking`：`dsh plugin
  --profile <名> add/remove/update <spec>` 转发链复用创建刀基建，pnpm 防御
  补齐同源，未物化/非法名先拒不 spawn，超时同创建 600s，失败附 dsh 输出
  尾部）、`lib.rs`/`ipc.rs`/`capabilities`（`install_plugin` / `remove_plugin`
  / `update_plugin` 三条 IPC 三处同步）、前端（详情对话框区头「安装插件」
  输入行 + 行内更新/卸载 hover 操作 + busy 态 + 结果分箱展示；spec 预检
  `validatePluginSpec` 镜像后端校验 + Vitest）。
- 影响：**触宪法级**——`AGENTS.md` §7 IPC 名册 +3。本条即知会。语义边界：
  装到**运行中**的 profile 时 dsh 不热重载，壳侧成功文案带「重启后生效」；
  add 裸包名 dist-tag 坑由输入占位引导带版本段规避（复现点 7）；安装进度
  v1 为单行 busy（不订阅 pnpm 流式输出），后续独立插件管理视图再升级。
- 凭据：cargo test 137 绿（+2：spec 恶意反例集、未物化/非法名先拒不
  spawn）/ fmt / clippy 全过；前端 typecheck / lint / test 49 绿（+2 镜像
  校验）。

### 2026-08-29 完成通知 · 4.4① 插件清单落地：详情对话框插件卡 + 运行态回环快照 —— guan（AI 协作）

- 变更：本 commit——`plugins.rs`（新模块：静态清单 = bundles + dependencies +
  node_modules 已装版本/描述；运行态 = `POST /api/pluginInventory/list` 回环
  只读快照，2s 超时，Spike B 方案 + 复现点 11）、`lib.rs`/`ipc.rs`/
  `capabilities`（`list_profile_plugins` / `get_plugin_runtime` 两条 IPC 三处
  同步）、前端（详情对话框依赖区升级为插件卡：官方/第三方、已装版本、运行态
  徽标，快照按 profile 匹配合并防张冠李戴；纯函数 `runtimeChipFor` /
  `runtimeSummary` + Vitest）。
- 影响：**触宪法级**——`AGENTS.md` §7 两处：IPC 名册 +2；网络面登记新例外
  「插件运行态回环只读查询」（127.0.0.1、只读、仅活跃会话、一次性快照）。
  本条即对该宪法修订的知会。**冻结期说明**：v0.6.0 验收仍待三平台产物，
  本 feat 经维护者当日裁定提前开工（「直接开工吧」），验收并行不受影响。
- 凭据：cargo test 135 绿（+4：静态清单/非法名/信封形状/响应解析）/ fmt /
  clippy 全过；前端 typecheck / lint / test 47 绿（+7 运行态合并纯逻辑）；
  运行态获取路径已在本机运行中的 dsh 实例实机打通（Spike B 实测记录）。

### 2026-08-29 发版事项 · v0.6.0 tag 已推送，冻结期开始 —— guan（AI 协作）

- 变更：commit `5ef27ab`（`chore: 版本 0.6.0（Profile 管理器全量 + Profile 切换）`，
  bump Cargo.toml / tauri.conf.json / package.json + 双 lock）+ 注解 tag `v0.6.0`
  （指向 `5ef27ab`）；master 与 tag 已推 origin（`5cf0593..5ef27ab`）。tag `v*`
  触发 CI Release（CONTRIBUTING §8）。
- 影响：**冻结期开始**——Release notes 发出至三平台产物验收通过期间，master
  只收 fix 不收 feat。Release notes 草稿见本条附录，频道确认后随 Release 发布。
- 凭据：bump 后 cargo test 131 绿 / 前端 typecheck + test 40 绿；推送回执
  `master -> master` + `* [new tag] v0.6.0`。

**附录：Release notes 草稿（v0.5.1 → v0.6.0）**

```markdown
## 新增
- Profile 管理器（4.3 全量）：列表/详情（已物化 + 可首启模板两态合并）、
  创建（dsh plugin 转发链——零网络毫秒级，创建即 webUi 候选可设默认）、
  复制/重命名/删除（运行中防护、node_modules 删除 + dsh 自愈）、
  默认启动 profile 持久化与 boot 消费。
- Profile 切换（4.3⑥）：管理器行内「启动」= 停当前 dsh 以目标 profile
  重启；仅 webUi 候选；切换不写默认；运行中徽标实时（boot:step 广播订阅）；
  WSL guest 启动脚本参数化（profile 名 shell 引号安全）。
- pnpm 为 boot 硬依赖：缺失自动 `npm i -g pnpm` 补齐，失败阻断给可行动文案。
- IPC 三处同步机器闸门：ipc.rs 单一事实源，漏登记 cargo test 即红。

## 修复
- 创建路径 dist-tag 版本坑：`add @deepseek-ai/dsh-base` 裸名按 latest 解析到
  已弃用旧版 → 404 + 重试卡死；改 `install` 原始版语义。
- Profile 详情对话框：file: 依赖行溢出丢包名、cordis.patch.yml 折行失真、
  超长行 grid 撑破布局三连修。
- 工具窗口内容超高被裁顶：垂直居中改顶部锚定。

## 变更
- 对话框 footer 去脚手架灰底，收编为有意设计（静默化）。
- AGENTS.md 减法一轮 248→178 行；README/CONTRIBUTING/本地运行指引更新。

## 已知问题
- WSL 模式 Profile 切换待 Windows 真机人工验证（按 docs/executor.md 清单）。
- 多开（多 profile 并行多窗口）未排期：前置 = 双实例 storages 竞态 spike +
  1:1 生命周期 ADR 修订。
```

### 2026-08-29 完成通知 · 4.3⑥ Profile 切换落地（重启语义）+ WSL guest 脚本参数化；多开登记待办 —— guan（AI 协作）

- 变更：本 commit——`executor.rs`（Executor trait 新增 `set_forced_profile`；
  Local/WSL 双执行器 probe 内按档位消费，bundle 快照档忽略；`GUEST_BOOT` 常量
  改 `guest_boot_script(profile)` + `sh_quote` 单引号进参——profile 名可含
  空格/引号等元字符，反例测试 + bash 实跑回读；WSL `select_profile`/`active_profile`
  同步参数化，v1「写死 web」收口）、`lib.rs`（`switch_profile` / `get_active_profile`
  两条新 IPC；`forced_profile` 目标记录 + `launch_executor_after_probe` 注入，
  错误卡重试延续同目标；模式切换清空重走常规解析）、`profiles.rs`
  （`ProfileSummary.web_ui` 字段 = 启动入口可见性，本模块内判定）、
  `ipc.rs`/`capabilities`/前端（types/api/store/行内启动按钮/运行中徽标/切换
  确认弹窗/文案）。维护者实测反馈两处收口：① 运行中徽标**实时化**——管理器
  订阅 `bootStore.activeStep`（boot:step 经事件总线每窗口广播），切换开始
  徽标即灭、boot 完成即亮，不再等聚焦/手动刷新；② 新增 UI 按 frontend-design
  口径与页面既有语言对齐——启动升级为带字按钮（与页头「新建 Profile」同配方）、
  运行中徽标加脉动心跳点、「无界面」由徽标降级进 meta 行、切换弹窗描述中性化
  （切换非常规破坏性操作，不走删除那套警示红）。
- 影响：**语义裁定（ADR-0009 §4 第三次修订，已先行经维护者逐题确认）**——
  ① 切换 = 停当前 dsh 以目标 profile 重启（dsh 无运行时切换能力，重启是唯一
  语义）；② 仅 webUi 候选可切换，headless/无界面档不给入口；③ 切换**不写**
  defaultProfile（星标是唯一写入口）；④ 失败错误卡 + 重试同目标，不自动回滚；
  ⑤ **WSL 同轮覆盖**：guest 脚本已参数化，但 WSL 真机验证无法在 macOS 执行，
  **待 guan 在 Windows 侧按 `docs/executor.md` 清单人工过一遍**（含切换到
  非 web 档 + 引号/空格 profile 名）。**多开（多 profile 并行多窗口）登记
  roadmap 待办未排期**：机制面可行（`--port 0` 官方旗标），前置 = 双实例
  并发写 `~/.dsh/storages` 竞态 spike + 1:1 生命周期约束的 ADR 修订。
- 凭据：cargo test 131 绿（+5：sh_quote 反例/回读、guest 脚本参数化、
  强制目标档位守卫、切换目标校验）/ fmt / clippy 全过 + `cargo check
  --target x86_64-pc-windows-gnu` 过（WSL 块编译目标语义）；前端
  typecheck / lint / test 40 绿。行为变更纯增，既有命令语义未动。

### 2026-08-28 完成通知 · Profile 创建二次修订：install 后壳补写 Web 工作台声明（创建即 webUi 候选）—— guan（AI 协作）

- 变更：本 commit——`profiles.rs`（`declare_webui_bundle` + 纯函数
  `append_bundle_declaration`：非模板名 install 成功后向
  `dsh.profile.bundles` 幂等追加 `@deepseek-ai/dsh-web-app`；声明补写失败
  降级 pending 态可重试；classify 增 webui_error 分支；模板名跳过——dsh
  拥有模板元组）；`lib.rs` doc；前端创建文案（基础 + Web 工作台/可设为默认）；
  `ADR-0009` §4 第二次修订注；ledger 复现点 7 + 追记；roadmap 4.3② 与事实
  边界行；**AGENTS §6 不变量行扩展（宪法改动，本条即知会 + diff 摘要）**：
  三件套写入例外 #2 = 「非模板名创建成功后的 web-app 声明单键追加」。
- 动机：defaultProfile 消费只认 webUi 候选（bundles 含 web-app，`resolve.rs`
  `list_web_ui_profiles`）——纯 dsh-base 原始版创建出来即无法设为默认启动
  （无 URL 可导航，boot 静默回退 web），用户实机踩坑（设「11」为默认、重启
  仍进 web）。
- 红线边界：三件套写入例外 #2，同类先例 = name 一致化改写。依据 dsh 源码
  `normalizeShippedProfile`（app-boot index.js @ 472，2026-08-28 读）：
  「Any other list is user-owned」——模板精确元组之外的 bundles 列表本就归
  用户/工具所有；目标状态与出厂 web 模板同构（web-app 不进 dependencies、
  `resolveBundleDir` 双锚点零下载），即 web profile 日常运行态。否决替代：
  `dsh plugin add @…@版本`（真实依赖 + 网络安装 + 版本锚定难题，偏离「初始
  web 标准」）。旧版创建的纯 dsh-base profile **不追溯**（重试创建才按新
  标准补齐）。
- 凭据：cargo test 126 绿（install_outcome 产物语义测试改写为
  create_declares_webui_bundle_like_web_template，classify 增⑥声明失败分支）/
  fmt / clippy 全过；前端 typecheck / lint / test 40 绿；实机目检见当日验证记录。

### 2026-08-28 完成通知 · 对话框 footer 静默化：去库存灰底，收编为有意设计 —— guan（AI 协作）

- 变更：本 commit——`ui/dialog.tsx` `DialogFooter` 去掉脚手架模板自带的
  `bg-muted/50` 灰底 + `border-t` 出血分隔带（2026-08-27 `a9c1656` 库存样式，
  未经审视），改为无底无线的静默 footer：按钮右缘与正文对齐，留白交给
  `DialogContent` 自身节奏。四个 profile 对话框（详情/创建/重命名/删除）
  共用组件，一致生效，无逐个覆盖。
- 依据：对话框正文为白底 + 发丝线卡片语言，灰带是全对话框唯一的填充色块，
  且详情对话框 footer 分隔线与 patch 卡片下边框相距 16px 贴出双线噪音；
  按「意图优先于强度」（refined minimalism = restraint）收编为有意设计。
- 凭据：前端 typecheck / lint / test 40 绿；tauri dev 实机目检（详情对话框
  用户协同截图确认：灰带与分隔线移除、按钮对齐、内容区无回归）。

### 2026-08-28 补记 · 详情对话框上一刀引入 grid 撑破回归，已修 —— guan（AI 协作）

- 变更：本 commit——`ProfileDetailDialog.tsx` 内容包装 div 补 `min-w-0`。
- 原委：上一刀把 patch 原文改 `whitespace-pre` 后，`<pre>` 最小内容宽度 =
  最长一行；`ui/dialog.tsx` 的 `DialogContent` 是 **grid** 布局，内容包装 div
  作为 grid 项未设 `min-w-0`，自动最小尺寸被 pre 撑破 → 整条轨道比对话框宽，
  依赖卡片与 footer 一起越界（用户实机截图复现）。`min-w-0` 归零该项对轨道
  尺寸的贡献后，pre 收敛回对话框宽度、由自身 `overflow-auto` 横向滚动。
- 凭据：前端 typecheck / lint / test 40 绿；**tauri dev 实机目检通过**
  （vite HMR 后 AX 点开「web」详情截图核对：依赖三行完整、file: spec 省略号
  截断、原文不折行、footer 归位）。

### 2026-08-28 完成通知 · Profile 详情对话框修复：file: 依赖行溢出 + patch 原文折行失真 —— guan（AI 协作）

- 变更：本 commit——`frontend/src/components/profiles/ProfileDetailDialog.tsx`
  两处：① 依赖行 `file:` 超长 spec 溢出卡片边框、包名被挤压至零宽不可见
  （spec `shrink-0` + 包名 `truncate` 的 flex 收缩方向写反）——改为包名
  `shrink-0` 恒可见、spec `truncate` + `title` 悬停全文兜底；② patch 原文
  `whitespace-pre-wrap` 折行续行顶格、与真实行混淆破坏「原文」语义——改
  `whitespace-pre` + 既有 `overflow-auto` 横向滚动，逐字保真。
- 影响：仅详情对话框展示层，无 IPC / 契约 / 数据改动；超长 spec 悬停可见
  全文。遗留（评审发现、未在本刀范围）：空插件组合复用「无额外依赖」文案
  的措辞错位、依赖版本号 `text-faint` 对比度偏低、对话框无高度约束的矮窗口
  健壮性——待后续小刀。
- 凭据：前端 typecheck / lint / test 40 绿（纯样式改动，Vitest 纯逻辑测试
  不涉及）；diff 已人肉复核。

### 2026-08-28 完成通知 · Profile 创建路径修订：add @deepseek-ai/dsh-base → install（原始版语义）—— guan（AI 协作）

- 变更：本 commit——`src-tauri/src/profiles.rs`（`create_command_args` 改
  `["plugin","--profile",<名>,"install"]`，删除 `CREATE_ADD_BUNDLE`；结果分类
  文案与注释同步「原始版语义」；测试：install 参数 + 原产物语义 + 成功路径
  `Already up to date`）；`lib.rs`（create_profile 文档注释）；`frontend`
  （`content/zh-CN.ts` 创建文案：busy 秒级 / done 内置声明就绪 / hint 原始版；
  `ProfileCreateDialog.tsx` 注释；`types/ipc.ts` 注释）；`docs/`（ADR-0009 §4
  执行细则修订注 + §5 正面后果 + 验证项；ledger 复现点 7 与复核记录；
  roadmap 4.3 ② 与事实边界行）。
- 影响：**ADR-0009 方案 A 执行细则修订（非换方案）**——创建命令由
  `add @deepseek-ai/dsh-base` 改为 `install`：创建语义 = 原始版 profile
  （initProfile 写三件套，bundles 含内置插件随 dsh 安装目录解析；空依赖
  `pnpm install` → `Already up to date`，零网络毫秒级）。触发：2026-08-28
  本机创建 `test` profile 慢至 2 分钟失败/超时，查因 = `add` 裸包名按
  dist-tag `latest` 解析到 dsh-base 0.0.1-rc.1（已弃用旧版，依赖 37+ 个已从
  registry 删除的旧包名：dsh-bash-env / dsh-tasks-local / dsh-skill-local…，
  npmmirror/npmjs 均 404）→ pnpm 递增重试（10s/60s × 37 包）卡死；npmmirror
  对缺失 scoped 包回退到死域名 `r.cnpmjs.org`（本机解析到保留段 198.18.0.192）
  放大表象。`install` 不解析 dist-tag，版本语义免疫。仅周知：pnpm 缺失 →
  补齐 → 失败降级（ADR 口径 2）与「已创建未装插件」中间态重试语义不变；
  后续加外挂插件走同一条 `dsh plugin add` 链（4.4）。
- 凭据：`cargo test` 126 绿；`gate_tests` 三处同步一致
  性绿；`cargo fmt --check` / `clippy -D warnings` 全过；前端 typecheck /
  lint / test 40 绿；实机（macOS，DSH_HOME=临时目录零污染）
  `dsh plugin --profile <新名> install` → init 先行 / `Already up to date`
  186ms 零网络 / 产物 `dependencies:{}` + bundles 仅 dsh-base（含
  pnpm-lock.yaml）/ 后 `--dump-config` 组合启动正常（退出码 0）；diff 已
  逐行人肉复核。

### 2026-08-28 完成通知 · 工具窗口布局修正：内容顶部锚定替代垂直居中 —— guan（AI 协作）

- 变更：commit `4bed1c9`（本 commit 落档本条）——`PageShell` 新增 `align`
  两态：常驻工具窗口（关于 / Profile 管理器）`top` 锚定 + `py-10` 上下
  呼吸位；`BootMode` 等主窗口启动抉择屏维持默认 `center`。
- 影响：仅周知。根因：旧静态壳窗口尺寸贴内容，`items-center` 居中不可见；
  前端迁移后窗口可调大小（profiles 680×700），内容悬浮正中、且内容超高时
  顶部被裁切（flex 居中 + overflow 的经典缺陷），顶部锚定一并消除。
  纯布局改动，IPC / 契约 / Rust 零变化。
- 凭据：`npm run typecheck` / `lint` / `test`（40）全绿；diff −4/+13 已
  人肉复核；布局观感按 4.4 口径归人工目验（Vitest 纯逻辑无 DOM 断言）。

### 2026-08-28 排障记录 · debug 构建白屏——dev 工作流文档修正 —— guan（AI 协作）

- 变更：本 commit——`README.md` 开发段与 `docs/CONTRIBUTING.md` ④ 的运行
  指引改为 `cargo tauri dev`（或 vite + cargo run 两终端），并明示「直接
  cargo run 而 vite 未起 = 壳窗口白屏」。
- 影响：仅周知。**现象与根因**：用户以 `cd src-tauri && cargo run` 启动后
  Profile 管理器 / 关于窗口全白。根因是 debug 构建的前端从 `devUrl`
  （localhost:1420）加载（tauri.conf.json，前端迁移时引入），vite 未运行
  → 资源加载失败；主窗口"正常"是假象——boot 完就导航进 dsh 工作台，把
  白屏的壳 SPA 盖掉了（shell.log 佐证：dsh 就绪 + 导航均正常，1420 端口
  无监听）。此为前端迁移后的既定工作流（`beforeDevCommand` 已配好），
  非管理器代码缺陷；上一条文档修正 commit 写反了 npm run dev 的用途，
  本次一并纠正。
- 凭据：`lsof -i :1420` 无监听 + `~/Library/Application Support/
  io.github.realguan.dsh-dock/shell.log` 显示 boot 全链正常；复现路径
  与修复命令均实机核对（tauri-cli 2.11.4 在机）。

### 2026-08-28 文档修正 · README/CONTRIBUTING 面向前端迁移与 Profile 管理器现状更新 —— guan（AI 协作）

- 变更：本 commit——`README.md`（「它做什么」补 Profile 管理器与 pnpm 环境
  保障两条、WSL 条目 settings.json 字段表述更正、开发命令全面修正：前端
  React SPA 的 npm 闸门 + **`cargo run` 须先 `cd src-tauri`**、品牌资源路径
  ui/assets → frontend/public 与 Emblem、结构树重画含 profiles.rs/ipc.rs 等
  新模块）；`docs/CONTRIBUTING.md`（「壳前端免构建（ui/ 静态页）」过时表述
  更正为 React SPA + npm 三闸门）。
- 影响：仅周知。根因是 2026-08-27 前端迁移与 4.3 各刀落地后文档未跟上——
  本机实测 `cargo run` 在仓库根直接失败（Cargo.toml 在 src-tauri/ 下），
  README 旧命令会误导所有新上手者。
- 凭据：文档改动；所有引用路径已 `ls` 核实存在（frontend/public/mark.svg、
  assets/dsh-logo.svg、scripts/*、node-map/README.md 等）。

### 2026-08-28 完成通知 · pnpm 补齐落地（boot 硬依赖 + 创建时复用，ADR 红线 2 收口）—— guan（AI 协作）

- 变更：本 commit——`updates.rs` 新增 `ensure_pnpm`（PATH 可见即返回；
  缺失经 `npm install -g pnpm` 同步补齐，复用 ADR-0005 npm 全局链 +
  ADR-0006 镜像序；补齐后必须在同一 PATH 重新可见，否则按失败处理）+
  `install_pnpm_via_npm`（双镜像逐试、聚合报错；pnpm 纯 JS 不传
  allow-scripts）+ 3 条测试（命中不装 / 装后可见性强制 / 失败聚合文案，
  假 node/npm-cli fixture）；`executor.rs` LocalExecutor::probe 接线
  （boot 期检查 → 失败阻断 boot 出可行动错误卡；基准 = 注入 dsh 的
  PATH）；`profiles.rs` 创建时防御检测升级为「缺失 → 补齐 → 再失败才报
  错」（复用同一函数）；`find_pnpm` 回归私有（外部消费点已被
  ensure_pnpm 收编）。
- 影响：仅周知。ADR-0009 红线 2 口径 2 至此完整：boot 环境检查保证
  「dsh 全部子命令可用」，补齐失败 = 新增 boot 失败模式（可行动文案：
  检查网络 / npm 镜像 / 手动 `npm install -g pnpm`）。WSL 客体内
  node → pnpm → dsh 链仍归 4.9（ADR-0004 §7）。AGENTS §7 网络面登记
  无变化（boot 期 pnpm 补齐已于 2026-08-28 登记）。4.3 遗留仅剩
  Windows 转发链实机验证（Spike A 遗留）。
- 凭据：`cargo test` 125 绿（+4，含恢复一处被测试插入截断的既有测试
  node_download_urls_mirror_first）/ `fmt --check` / `clippy -D warnings`
  全过；diff 已逐行人肉复核。

### 2026-08-28 完成通知 · 4.3④ defaultProfile 消费接线 + WSL 放开评估收口 —— guan（AI 协作）

- 变更：本 commit——`resolve.rs` 新增纯函数 `consume_default_profile`
  （存储默认值 ∈ webUi 候选才消费，含正反例测试）；`executor.rs`
  LocalExecutor::probe 接线——命中即以该 profile 启动并跳过选择器，
  仅覆盖 dsh_home = 用户 home 的档位（system/download），bundle 快照
  世界不适用；未命中（headless 类无 webUi / 已被手工删除）回退常规
  流程并记日志。ADR-0009 §5 WSL GUEST_BOOT 放开评估收口：**本版不放开，
  维持 `--profile web`，归 4.9**（客体 home 与壳侧 home 不同世界 /
  非 webUi 无 URL 可导航 / 客体内 profile 管理属 4.9 范围）。
- 影响：仅周知。「设为默认启动」语义自此完整：设置 → 持久化 → 下次
  启动自动使用（多 webUi 不再出选择器）；选择器仍在（未设默认时），
  其「选择只影响本次会话」语义不变。验收路径：管理器设默认 → 重启
  应用 → 直接进入该工作台（日志可见 defaultProfile 命中行）。
- 凭据：`cargo test` 121 绿（+1）/ `fmt --check` / `clippy -D warnings`
  全过；diff 已逐行人肉复核。

### 2026-08-28 完成通知 · 4.3 Profile 管理器第五刀（前端管理页）+ AGENTS §4.4 重评 —— guan（AI 协作）

- 变更：本 commit——前端 `pages/ProfileManager.tsx` + `components/profiles/*`
  （列表行两态 / 详情 / 创建 / 复制 / 重命名 / 删除确认五组组件）+
  `stores/profilesStore.ts` + `lib/profiles.ts`（校验镜像等纯逻辑，6 条 Vitest）
  + `lib/tauri.ts`（8 个 profile api，共 20 命令全类型化）+ `types/ipc.ts`
  （五个响应类型锚定 Rust serde 形状）+ `content/zh-CN.ts`（profiles 文案段）
  + `App.tsx`（label=profiles 路由）；后端 `lib.rs`（`open_profiles_window`
  镜像 about 主线程约束 + macOS 菜单 / 非 macOS 托盘入口
  `profiles_manager`）+ `capabilities/default.json`（windows 数组加
  `profiles`——ACL 按窗授权，漏加即整页 IPC 静默拒绝）。
- 影响：**触宪法级**两处，仅周知——① AGENTS §4.4 Vitest 重评条件已触发并
  落盘结论：维持纯逻辑测试，RTL/jsdom 不引入（再评触发 = 需 DOM 断言的
  复杂交互）；② TanStack Query 未接入：管理页是「读一次 + 变更后手动刷新」
  形态，frontend-migration §11 的触发条件裁定延后（rationale 见
  profilesStore.ts 头注释，出现跨窗口订阅诉求再立 micro-ADR）。
  范围声明：4.3 全部六项能力的 UI 至此可用（菜单/托盘 → Profile 管理器）；
  defaultProfile 的 boot 消费接线、pnpm 补齐仍归后续刀。前端文案含
  删除确认三要素（不级联全局数据 / 其他 dsh 实例 / 模板名重新物化，ADR
  §2 要求）与创建 pending 中间态（ADR §3 方案 A 契约）。
- 凭据：`npm run typecheck` / `lint` / `test`（40，+6）/ `build` 全绿；
  `cargo test` 120 绿 / `fmt --check` / `clippy -D warnings` 全过；
  diff 已逐行人肉复核。

### 2026-08-28 完成通知 · 4.3 Profile 管理器第四刀（生命周期 + 默认持久化）—— guan（AI 协作）

- 变更：本 commit——`profiles.rs`（复制/重命名/删除文件层 + 前置校验 + 运行中
  防护文案 + patch `../` 引用扫描警告 + 8 条测试）；`settings.rs`
  （`defaultProfile` 字段，第二最小面例外）；`executor.rs`（`active_profile`
  trait 方法：运行中防护比对源，本地取 launch、WSL 固定 web）；`lib.rs`
  （5 个新 IPC 命令；**连带修复** `switch_mode`/`choose_mode` 改
  load-modify-save——原整体覆盖写法会抹掉 defaultProfile）；三处同步
  （ipc.rs/capabilities）。引用面全按 Spike B §3 执行：复制排除 node_modules
  + name 改写；重命名删 node_modules 让 dsh 自愈；删除不级联 sessions；
  defaultProfile 删除时清除、重命名时同步，失效读取侧兜底 web。
- 影响：**触宪法级**——AGENTS §6 例外册登记 `defaultProfile` 落地、§7 IPC
  登记表新增 5 命令（`copy_profile`/`rename_profile`/`delete_profile`/
  `set_default_profile`/`get_default_profile`），仅周知。范围声明：管理器
  后端能力至此齐备（列出/详情/创建/复制/重命名/删除/切换默认）；前端管理页、
  defaultProfile 的 boot 消费接线（含 WSL GUEST_BOOT 放开多 profile 评估）、
  pnpm 补齐均归后续刀。
- 凭据：`cargo test` 120 绿（112 + 新增 8：复制排除与改写 / 重命名自愈 /
  删除不级联 / 运行中防护文案要素 / 默认候选校验 / settings 旧格式兼容等）；
  `gate_tests` 三处同步一致性绿；`cargo fmt --check` / `clippy -D warnings`
  全过；diff 已逐行人肉复核（经维护者裁定 ①+② 范围：生命周期 + 默认持久化
  一刀完成，pnpm 补齐与前端拆分后续刀）。

### 2026-08-28 完成通知 · 4.3 Profile 管理器第三刀（创建能力）—— guan（AI 协作）

- 变更：本 commit——`src-tauri/src/profiles.rs`（创建段：转发链 spawn 封装 +
  前置校验 + 结果分类 + 4 条纯函数测试）；`updates.rs`（`find_pnpm` 转
  pub(crate) 共用）；`ipc.rs` / `lib.rs` / `capabilities/default.json`（IPC 三处
  同步，`create_profile` 为**首个异步命令**：探测 + 转发链全在 spawn_blocking，
  避免同步命令冻结主线程）；`AGENTS.md` §7；ADR-0009 §5；ledger 复现点 7。
  能力：spawn `dsh plugin --profile <名> add @deepseek-ai/dsh-base` 半官方路径
  创建 profile（三件套由 dsh initProfile 写出，壳零写入）；重名拒绝 + 半初始化
  放行重试（重跑 add 幂等，ADR §4）；pnpm 防御检测缺失即拒 spawn（可行动文案，
  补齐归后续刀）。
- 影响：**触宪法级**——AGENTS §7 IPC 命令登记表新增 `create_profile`，仅周知。
  范围声明：定位仅系统探测（离线档/未装系统 dsh 用户暂不可创建，报可行动错误）；
  复制/重命名/删除/默认持久化/WSL 客体内 profile/pnpm 补齐均归后续刀。
  实机验证（macOS，DSH_HOME=临时目录零污染）：init 先行 / pnpm 经注入 PATH 可
  定位 / 顺带实测 pnpm 网络失败模式（镜像 ECONNRESET -> 已创建未装中间态
  exit 1，分类文案与单测 fixture 逐字吻合）；成功路径 reconcile 沿用 Spike A
  §3.2 同机同版本结论；Windows 转发链（shell: win32 分支）仍为遗留。
- 凭据：`cargo test` 112 绿（108 + 新增 4）；`gate_tests` 三处同步一致性绿；
  `cargo fmt --check` / `clippy -D warnings` 全过；diff 已逐行人肉复核。

### 2026-08-28 完成通知 · 4.3 Profile 管理器第二刀（只读能力）—— guan（AI 协作）

- 变更：本 commit——新增 `src-tauri/src/profiles.rs`；`ipc.rs` / `lib.rs` /
  `capabilities/default.json`（IPC 三处同步）；`AGENTS.md` §7；`docs/adr/0009`
  §5 勾选；`docs/contracts/dsh-behavior-ledger.md`。能力：profile 非法名校验
  （与 dsh `resolveProfileDir` @ 318 逐字一致）、profiles 扫描器（已物化 +
  未物化内置模板名两态合并；排除 `profiles/node_modules` 符号链接农场）、单
  profile 详情（package.json 关键字段 + `cordis.patch.yml` 原文，YAML 不解析——
  serde_yaml 已弃维，依赖选型推迟到启停插件刀）；新 IPC 命令
  `list_profiles` / `get_profile_detail`。纯读：零写入、零 dsh 子进程、零网络。
- 影响：**触宪法级**——AGENTS §7 IPC 命令登记表新增两条（新命令流程规定动作），
  仅周知。范围声明：管理器仅覆盖壳侧本地 home（`user_dsh_home()`）；WSL 客体内
  profile、创建/复制/重命名/删除、`--dump-config` 详情、pnpm 补齐均归后续刀。
  ledger 复现点 6/8 已按行号勘误口径（318/323/353，弃 11826/13418）入册。
- 凭据：`cargo test` 108 绿（基线 98 + 新增 10：校验正反例 / 扫描两态与农场
  排除 / 详情路径遍历拒绝等）；`gate_tests` 验证三处同步一致性；
  `cargo fmt --check` / `clippy -D warnings` 全过；diff 已逐行人肉复核。

### 2026-08-28 宪法级改动 · AGENTS.md 减法：248 → 178 行，删微观管理留边界 —— guan

- 变更：`AGENTS.md` 全文重写（commit `876dbcb` 之后）——删除三类内容：① 通用工程
  常识（跨平台 #[cfg] 分叉、阻塞主线程禁令、组件文件命名、Zustand 选择器细则、
  mock 策略细节等——AI 工程常识默认做对，写清单反而暗示「除此以外随便」）；
  ② 可推导明细（模块职责逐个注释、依赖版本行、构建发布行、§7 专项裁定长文——
  §9 索引已有等价一行结论，消双源）；③ 过度规定（错误页样式、stdout 日志技法——
  归还代码注释与 ADR）。保留全部边界与真坑：两红线（含台账指针）、dsh 文件系统
  不变量、唯一网络面、IPC 三处同步、事件总线模块加载期竞态坑、child_cmd、
  tauri-cli 同代、zip pinned、必测四场景、AI 交互七条、持久化例外册、§11 元规则。
- 影响：**宪法级**——规范哲学转向「只写边界 + 真坑 + 为什么，不写怎么做」，给
  AI 留发挥空间；被删内容均有归宿（代码 / ADR / roadmap / 全局 AI 配置）或属
  可推导。此前批准的全部提交于 `876dbcb`，本笔减法独立可回溯。
- 凭据：纯文档改动，不触运行时；全文 178 行 ≤ 250 预算、单节最大 31 行（§4）；
  真坑与边界关键词 grep 抽查全数在册；修改未提交，待确认后提交。

### 2026-08-28 宪法级改动 · 规则方向性审核落地：复现台账 + 白名单闭环 + 规则触发器 —— guan

- 变更：① 新建 `docs/contracts/dsh-behavior-ledger.md`——dsh 行为复现台账（已落地
  5 项 / 计划 3 项，基线 v0.1.1-rc.2），堵「复现点随 dsh 升级静默漂移、CI 无法发现」
  缺口，dsh 升级 = 逐条复核触发器；`AGENTS.md` §0 红线 1 挂台账指针。② §4.4 白名单
  治理闭环——「未来数据层」改「数据获取层（取数·缓存·同步）」判据，新增依赖 =
  先回写清单（唯一权威）再广播。③ 两条规则补再评估触发器——RTL/jsdom 禁令挂
  「Profile 管理器 UI 落地时重评」、无 dev-dependencies 加「确需专用依赖先 ADR」口子。
  ④ roadmap Next 加工程前置——IPC 三处同步自动化自检 spike（4.3 开工前把人肉纪律
  升级为机器闸门）。
- 影响：**宪法级**（AGENTS.md）+ 新增台账文档。dsh 升级流程多一步「复核台账」；
  前端新增依赖流程变「回写清单 + 广播」；4.3 开工前多一个 spike 闸门。纯文档，
  不触运行时。
- 凭据：复现点逐条 grep 锚定代码实际位置（shell.rs / resolve.rs / updates.rs /
  executor.rs）；AGENTS 全文 248 行 ≤ 250 预算、§4 恰 40 行；修改未提交。

### 2026-08-28 宪法级改动 · 跨文档定位统一 + AGENTS 一致性修正 —— guan

- 变更：三份文档定位统一为「dsh 的桌面管理面板」（2026-08-27 重定义的收尾）——
  roadmap §1 开头「桌面终端 / 极小的壳」改写、README 开头定位段改写（补管理能力句）、
  contract.md 宿主解析节加历史定位注（ADR-0005 语义不变）；AGENTS.md 五处一致性
  修正——§7 例外册补登记 `app:update` 事件（updater 回推，仅 main/about）、§2 docs
  清单补 frontend-migration 与 spikes/、§5 测试命令收敛为 §1 指针（消双源）、§0
  工程准则补 §6 指针、§6 持久化例外改「例外册登记制」（与 roadmap 硬约束 4 对齐，
  第二例外落地前先登记字段名）。
- 影响：**宪法级**（AGENTS.md + contract.md）。定位口径此后以 AGENTS §0 为唯一
  权威；settings.json 第二字段落地前须先在 §6 登记。纯文档，不触运行时。
- 凭据：grep 全仓「桌面终端」仅剩 ADR 历史引用（刻意保留）；本笔与上一条（制度
  建设）同批未提交，待确认后按 CONTRIBUTING 提交。

### 2026-08-28 宪法级改动 · AGENTS.md 首轮回收：394 → 244 行，回归 §11 预算 —— guan

- 变更：`AGENTS.md` 全文按 §11 回收——§2 目录树（58→17 行）：`ls` 可推导的文件级
  明细删除，只留职责与陷阱（勿动 / 勿手改 / 永不入库）；§7 例外册（50→19）：IPC 命令
  收敛为登记行，专项裁定收敛为「一行一裁定 + ADR 编号」，推理细节以 §9 索引 + ADR
  为唯一源（消除双源）；§1 技术栈（35→18）：依赖版本明细还 Cargo.toml / package.json，
  只留坑与裁定锚点；§4 代码规范（69→40）：正反例去重合并；§5 测试明细（25→13）还
  roadmap；§0 / §8 / §9 / §10 措辞收紧。只删可推导明细与已有归宿的长文，裁定结论全保留。
- 影响：**宪法级**——纯回收、无新增裁定；所有删除项均有既有归宿（ADR-0001~0009 /
  roadmap 4.1/4.2 / 代码现状）。协作者若发现某被删细节在 ADR / roadmap 也找不到，
  频道提出即可，git 历史可回溯旧版全文。本笔与上一条（§11 元规则）同批未提交，
  建议合为一笔提交（hunk 交叉无法拆分）。
- 凭据：纯文档改动，不触运行时；全文 244 行 ≤ 250 预算、单节最大 40 行（§4），
  diff 394→244（−150）。另发现 README.md「结构」节仍是 React 迁移前旧版（ui/ 静态页），
  属已知失真，另行开工修正，本笔不动。

### 2026-08-28 宪法级改动 · AGENTS.md 新增 §11 写入边界（元规则）—— guan

- 变更：`AGENTS.md`——① 文件头挂一行指针（「最小必要集，不是知识库」）；② 新增
  §11 元规则：准入判据四条（高频 / 违约即事故 / 不可推导 / 无家可归，核心判据 =
  「没有这条 AI 会做错吗」）、排除清单七类（模块技法→注释、决策推理→ADR、流程
  细则→CONTRIBUTING、契约细节→contract.md、计划指标→roadmap、操作手册→docs
  专项、通知→broadcasts）、形态规则（结论一句 + 日期 + 指针；升格 ADR 后原条目
  必须回收，禁双源）、预算与回收（全文 ≤ 250 行 / 单节 ≤ 40 行）。
- 影响：**宪法级**——此后向 AGENTS.md 写入任何条目须先过 §11.1 判据，不合者
  review 可依据本节驳回；知会落档、broadcasts 登记范围不变。存量超支（~354 行，
  §2 目录树 / §7 例外册为主）按新规则另行开工一笔回收。
- 凭据：纯文档改动，不触运行时；本笔仅新增元规则、不回收存量（一次一意图）；
  修改未提交，待确认后按 CONTRIBUTING 提交。

### 2026-08-27 宪法级改动 · 项目边界重定义：dsh 桌面管理面板 + 两红线 —— guan

- 变更：`AGENTS.md`（§0 定位重写为「dsh 的桌面管理面板」/ §1 技术栈表数据库行 /
  §4.2 禁库改「需 ADR 评估」/ §6 存储与生命周期重写）、`docs/roadmap.md`（硬约束
  2/4 重写）、`docs/adr/0008`（壳保持薄加注「仅约束运行时」）、`docs/adr/0005`
  （转发链 vs 全局安装区分注）、`docs/adr/0006`（管理功能网络面不属唯一网络面注）、
  `docs/executor.md`（defaultMode 例外表述同步）、`docs/adr/0009`（§2 重写为
  3 红线 + 工程准则）。
- 影响：**宪法级**——① 项目定位从「通用产品壳」改为「dsh 的桌面管理面板」；
  ② 红线定为两条：不修改 dsh 源码 + 安装包不内置依赖（优先宿主检测、缺失时
  实时下载，含 pnpm 自动补齐扩展）；③ 「壳保持薄 / 无状态库」降为**仅约束运行时**，
  管理功能按优秀软件工程设计（可引入数据库 / 持久化）；④ 历史 ADR 保持原状仅加注。
  协作者注意：管理功能（4.3+）不再受「无状态库」限制，但 dsh 文件系统不
  变量（0600 凭据 / 三键 / 农场只读）继续必须维护。
- 凭据：纯文档改动，不触运行时；diff 已逐文件核对（AGENTS §0/§1/§4.2/§6 +
  roadmap 硬约束 + 4 个 ADR 加注 + executor.md 同步）。修改未提交，待确认后按
  CONTRIBUTING 提交。

### 2026-08-27 快车道直推 · 完成通知：Now 阶段收口（4.1 工程化基线 + 4.2 updater 测试）—— guan

- 变更：六连提交——`bd94596` fix(clippy) 全部 17 处警告清零（关键：
  shell.rs `floor_char_boundary` 击穿 rust-version=1.77.2 的 MSRV 承诺，
  CI 全用最新 stable 故未暴露）；`dd521d1` style 全仓 fmt 归一
  （9 文件 168+/147- 纯机械，与 clippy 修复分仓提交）+ 落地仅锁 edition 的
  `rustfmt.toml`；`9029ebe` chore(rust)；`10e0957` ci 三平台
  fmt --check / clippy -D warnings 闸门 + ubuntu coverage job
  （cargo-llvm-cov 出 lcov，先出数不定阈值）；`5417987` test(updater)
  六条纯函数测试；docs 提交（见下）。
- **宪法级改动（本次知会）**：① AGENTS §1 Rust 行——移除 `rust-version`
  基线，**Rust 工具链跟随最新 stable**（2026-08-27 维护者裁定：不设 MSRV；
  上限纪律不变：CI @stable 自动跟新）；② AGENTS §1 Lint/Format 段改写为
  已建基线状态；③ AGENTS §5 updater 待补条目改为已覆盖表述；
  ④ AGENTS **新增 §8 第 7 条「驳回不合理的规则」**——AI 判定规范与现实
  冲突/自相矛盾/失效时应停手提请驳回与修订（举证义务在提请方），
  不得以变形实现绕行；顺从≠忠诚，变形合规比违规更危险。
- 影响：CI 首跑新闸门有红的风险已用本地预演对冲（本机 1.98 三道全绿）；
  Next 阶段（4.3 Profile 管理器）进入条件满足，开工前先做两个前置 spike。
- 凭据：本地 rustc 1.98.0 下 fmt --check / clippy --all-targets -D warnings /
  cargo test 95 绿；fmt 与 clippy 两类 diff 分仓提交均经人肉复核。

### 2026-08-27 发版事项 · v0.5.1 三平台验收通过，冻结期解除 —— guan

- 确认：tag run `33049383090` 三平台 job 与 release job 全部 success；Release
  资产 14 个齐全（dmg/exe/msi/AppImage/deb/rpm + 签名 + latest.json）；
  下载 macOS `.app.tar.gz` 实拆——`Info.plist` 版本 0.5.1，主二进制内嵌前端
  bundle 五个事件名（app:update / boot:step / boot:progress / boot:update /
  boot:error）grep 全中（本次缺陷的产物级判据）。上游 dsh 会话工作正常。
- 影响：**v0.5.1 发布完成，冻结期自此解除**——master 恢复正常合流
  （改动仍按 CONTRIBUTING 占用声明纪律）；下一步按路线图 Now 阶段推进。
  已装 0.5.0 的环境因自更新面板同受缺陷影响，需手动换装 0.5.1。
- 凭据：`gh api .../runs/33049383090/jobs` 全 success；本条即对上两条
  「处置见下一条广播」预告的闭环。

### 2026-08-27 发版事项 · v0.5.0 缺陷确认 → 重切 v0.5.1 热修 —— guan

- 变更：fix `f3cef30`（事件总线 import 锚点，见下条补记）+ updater 观测日志
  （run_check 入口与 set_state 每次推进记 tracing）+ 全仓版本升 `0.5.1`
  （tauri.conf.json / Cargo.toml / Cargo.lock / frontend package.json+lock），
  tag `v0.5.1` 当日推送。
- 裁定（为何不沿用上次 force 迁移 tag 的做法）：v0.5.0 三平台产物**已发布且含
  缺陷**——事件监听缺失不止影响关于页：启动时间线 / 错误卡 / 下载进度同链路
  全部不刷新，冷启动表现为启动页冻结后硬跳工作台、失败时错误卡不渲染。
  上次 force 迁移的前提是「原 run 无任何产物」；本次 Release 已存在、可能已有
  下载，force 迁移会留下同名异物的资产，违反可追溯原则。按语义化版本重切
  v0.5.1。注意：**0.5.0 客户端的自更新面板恰好也受此缺陷影响**（自动检查在
  Rust 侧正常执行，但 UI 不回显），已装用户需手动换装 0.5.1。
- 影响：冻结期继续（master 只收 fix）；CI 三平台验收通过前不宣布发布完成。
- 凭据：cargo test 89 绿 + 前端四道门禁绿；实机 AX 观察到修复后徽章
  「检测中 → 最新」翻转（自动首查全链路），手动点击路径同链路 +
  新增日志可事后定位；版本 diff 六文件已人肉复核。

### 2026-08-27 补记 · v0.5.0 实机缺陷：事件总线未进 bundle（关于页检查更新无反应） —— guan

- 变更：fix 待提交——`frontend/src/main.tsx` 增加 `import "./lib/events"` 副作用
  锚点；`docs/frontend-migration.md` §9 新增「事件总线」产物级回归清单条目。
- 根因：`lib/events.ts` 靠模块加载期 `initEventBus()` 自装配（宪法 §4.3 裁定），
  但全仓没有任何运行时 import 它——Vite 树摇将其整体排除出 bundle（v0.5.0
  dist 中 `app:update` 出现 0 次，实锤）。所有窗口的 boot:*/app:update 监听均未
  注册；关于页显示的「已是最新」全部来自进入时的播种 invoke，恰好掩盖断链。
  纯逻辑单测（34 例全绿）测不出「装配丢失」这类集成缺失。
- 影响：**v0.5.0 三平台产物若已出包则携带此 bug**——关于页更新状态机不再实时
  推进、启动时间线/错误卡/下载进度不刷新。处置与是否重打 tag 见下一条广播。
- 凭据：前端四道门禁绿 + 新产物 grep 五事件名各 ≥1；修后复验结论随附。

### 2026-08-27 补记 · v0.5.0 发布中断修复（Ubuntu CI 失败 → 重发） —— guan

- 变更：`915657f`（冻结期 fix）—— beforeBuildCommand 钩子 cwd 显式化
  （tauri.conf.json ScriptWithOptions `cwd="../frontend"`），build.yml/AGENTS
  注释同步；`v0.5.0` tag 已 force 迁移指向该修复（原 tag run 全失败、无任何
  产物/Release，无污染可追溯亏损）。
- 根因（CI 实证两连修）：① tauri-cli「自动发现含 package.json 目录」深度遍历
  在 Linux ext4 目录序下可能先命中 `node-map/` → npm ci 找不到 lockfile（本地
  APFS 碰巧命中 frontend/，阶段 A 的验证结论被事实击穿）；② 显式 cwd 相对基准
  实为 **src-tauri**（build.rs `set_current_dir(dirs.tauri)`），首修用的
  `frontend/` 本地复现 No such file 后改为 `../frontend` 复测通过。
- 影响：仅发布链路，不触运行时行为——macOS/Windows 两 job 原 run 继续走完
  但其构建内容同构（hook 修复对三平台同效），三平台产物仍以重发 run 为准。
  经验已落档：**tauri 钩子 cwd 永远相对 src-tauri 且必须显式**，勿复信自动发现。

### 2026-08-27 发版事项 · v0.5.0 发布开始 —— guan

- 变更：`chore: 版本 0.5.0（…整批提交）`——tauri.conf.json / Cargo.toml /
  Cargo.lock / frontend package.json+(lock) 同步升版；roadmap 适用版本标 v0.5.0。
  tag `v0.5.0` 由本档案登记当日推送，CI 三平台矩阵 + Release 聚合
  （notes 由 GitHub 自动生成）。
- 影响：**冻结期开始**（CONTRIBUTING §8）——Release notes 发出至三平台产物
  验收通过期间，master 只收 fix 不收 feat。本版内容：前端自静态 HTML 全量迁移
  Vite+React+TS+Tailwind v4+shadcn/ui（ADR-0008 全流程，span commit
  aab68c1→本次），壳行为与 12 IPC 命令零变更；Move/关于/启动/选择器四页
  组件化；宪法同步修订（AGENTS §1/§2/§4.2/§4.3/§4.4/§5/§7）。
- 凭据：frontend gate 34/34；cargo test 89 passed；本机 release 构建三产物齐；
  已知遗留——Windows/Mode 页实机走查未做（广播 2026-08-27 阶段 C 条目）；
  fmt/clippy 基线待专项（阶段 E 条目）。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 E 落地（d003905 / 1868411） —— guan

- 变更：**宪法级（1868411）**——AGENTS §1/§2/§3/§4.2/§4.3/新增 §4.4/§5/§7、
  docs/roadmap.md（硬约束 2 与不做清单）、docs/CONTRIBUTING.md（路径行）、
  .github/workflows/build.yml（Frontend gates 步骤）；**非宪法（d003905）**——
  Vitest 34 用例（format/bootProgress/bootStep/updatePhase）、`ui/` 目录删除
  （dsh-logo.svg 迁至仓库根 `assets/`）、全仓悬空引用清理。
- 影响：① **宪法已生效**——「禁止引入任何前端构建链」修订为「前端框架仅限
  React 生态（§1/§4.2/§4.4 白名单）」；② 开发者须知——本地构建/调试请从
  **仓库根**调用 `cargo tauri dev/build`（钩子 cwd 发现逻辑），前端开发需
  node ≥20（`cd frontend && npm ci`）；③ **fmt/clippy 基线评估结论**：存量
  35 文件未归一 + clippy 9 警告，需专项 chore 落地（遵守「不引入全仓格式化
  diff」红线），本轮 CI 只接前端四道闸门，roadmap §4.1 [待补充] 保持；④ 迁移
  完成发布契——master 自此无 `ui/`，release 产物壳页面全 React。
- 凭据：frontend typecheck/lint/vitest 全绿（34/34）；`cargo test` 89 passed；
  `cargo tauri build --no-sign` 出齐三产物；diff 逐行人肉复核（lib.rs 仅两处
  注释；fmt 越界改动已回退——本次 session 自身纪律记录）。

### 2026-08-27 占用声明 · 前端迁移阶段 E 开工（宪法级变更预告） —— guan

- 变更：占用 `AGENTS.md`（§1/§2/§3/§4.2/§4.3+新增§4.4/§5/§7）、
  `docs/roadmap.md`（硬约束 2 与不做清单）、`.github/workflows/build.yml`
  （node 质量闸门 + fmt/clippy 评估）、`ui/`（删除）、`frontend/`（Vitest 测试）。
  依据：ADR-0008 行动项 + docs/frontend-migration.md §6/§7/§10 阶段 E 清单。
- 影响：**宪法级预告**——AGENTS §4.2「禁止引入任何前端构建链」将修订为
  「前端框架仅限 React 生态（Vite+React+TS+Tailwind+shadcn/ui）」，§4.3 全面
  重写为 React 组件规范，新增 §4.4 三条红线（依赖白名单 / 前端禁止网络请求 /
  跨窗口真相源）。`ui/` 目录删除、`dsh-logo.svg` 迁至仓库根 `assets/`。
  执行顺序：测试 → 删 ui → 宪法/CI（单独 commit）→ 全量验证 → 完成通知。
- 凭据：阶段 A-D 均已通过闸门与实机验证（见前四条约）；本预告为宪法修改
  前置知会，修改范围与方案 §7 清单一一对应。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 D 落地（1008cd6） —— guan

- 变更：`pages/BootIndex.tsx` 整页、`components/boot/{BootStep,BootTimeline}.tsx`
  新增、ErrorCard diag 形态、`lib/events.ts` 总线装配时机、`pages/BootMode.tsx`
  握手时序、方案文档 §3.3（总线裁定同步）。**四页至此全部迁入 React**。
- 影响：一处时序裁定周知——事件总线要求在**页面任何播种 invoke 之前**注册；
  实现为模块加载期装配（详情见方案 §3.3 与 lib/events.ts 注释）。stage B 中
  BootMode「先 invoke 再导航」的写法本轮已更正为旧握手（携参回启动页由
  BootIndex 落地）。
- 凭据：typecheck/lint/build 全绿；release 冷启动事件链全通（日志钉板）；
  BootIndex 静态帧经 dev 预览核对；下载条/错误卡的实机触发依赖特定失败路径，
  逐行对照旧码迁移（已复核）。阶段 E 前 master 中间态照旧：壳页面功能已全，
  待删 `ui/` 与宪法修订。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 C 落地（d780855） —— guan

- 变更：`frontend/src/pages/{BootMode,BootSelector}.tsx` 整页、
  `components/boot/`（DownloadProgress/ErrorCard/VersionChip/PulseBar 落地，
  阶段 D 复用）、`hooks/usePlatform.ts`、文案层 mode/selector 扩展。
- 影响：仅周知 + 一项待办迁移——**Mode 页（运行环境选择）是 Windows-only
  表面**，React 版实机目视验证待 Windows 环境（非 Windows 访问按裁定防御性
  回启动页，本机已验证该兜底路径编译正确）。其余同前：master 中间态渐次回填。
- 凭据：typecheck/lint/build 全绿；release 实机 fixture 双工作台触发选择器
  （双卡片 + DEFAULT/CUSTOM 徽标 + 版本芯片渲染正确，截图存档）；点击官方卡
  打通 `choose_profile` IPC 全链路（shell.log 钉板：dsh 启动 profile=web →
  1.3s 就绪 → 导航工作台）。测试进程与 fixture 均已清理。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 B 落地（5ce1296） —— guan

- 变更：`frontend/src/components/about/`（新建四组件）、`pages/About.tsx` 整页、
  `content/zh-CN.ts` 文案扩展、`index.css` token 改名、`App.tsx` 预览钩子、
  方案文档 §3.5 同步。关于窗口自旧 `ui/about.html` 完整迁入 React。
- 影响：仅周知 + 一处 token 命名裁定——`--color-muted/--color-accent` 与
  shadcn 语义层重名导致工具类被覆盖（实机截图发现文字近白），域 token 更名
  **dim / brand**；后续页面（阶段 C/D）直接用新名。master 中间态照旧：壳骨架页
  渐次回填，阶段 E 收口删 `ui/`。
- 凭据：typecheck/lint/build 全绿（gzip 137KB，<500KB 复审线）；release 实机
  截图验证整链路（自动首查→upToDate、三维度真实数据、工作台地址注入）；
  配色修复以构建产物 CSS 钉板（`.text-dim{color:var(--color-dim)}` 解析唯一，
  内联映射机制反向解释原 bug）；本地 release 产物已重建包含修复。

### 2026-08-27 补记 · 阶段 A 目视验证完成（步骤 20/21 清账） —— guan

- 变更：`docs/frontend-migration.md` §3.1 一处标注（钉板句从「待复核」改为
  「已复核通过」）。无代码改动。
- 影响：仅周知——release 产物内以临时第二 webUi profile 触发
  `/selector` 直达，**SPA fallback 实机命中**（主窗口渲染 BootSelector 页，
  步骤 21 钉板完成）；about 窗口经菜单打开，label 路由渲染 About 页（步骤 20）。
  临时 fixture（仅含 package.json 的 `~/.dsh/profiles/probe-dual-webui`）
  已删除，真实环境零残留。
- 凭据：实机截图两帧（主窗口 /selector 内容 + 关于窗口内容）；进程已退出。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 A 落地（a9c1656） —— guan

- 变更：`frontend/`（新建 41 文件）、`src-tauri/tauri.conf.json`、
  `src-tauri/src/lib.rs`、`.github/workflows/build.yml`。Vite+React+TS+Tailwind v4
  + shadcn/ui 七组件脚手架；窗口 label 路由；四页占位骨架；Rust 主/about 窗口
  改载 SPA 根、selector/index 跳转改 pathname；platform_script 扩 `{os,wsl}`。
- 影响：**master 中间态**——自本 commit 起 release 产物壳页面为 React 骨架
  （功能回填顺序 B About → C Mode/Selector → D Index，阶段 E 收口删 `ui/`
  并改宪法）。与占用声明的偏差仅一处：CI node 步骤自阶段 E 提前（frontendDist
  切换后 `cargo tauri build` 硬依赖 npm 构建，不提前则打 tag 即挂）；
  Build installers 工作目录随之移到仓库根。开发者注意：本地构建/调试请从
  **仓库根**调用 `cargo tauri dev/build`（钩子 cwd 发现逻辑要求，见 build.yml 注释）。
- 凭据：typecheck/lint/vite build 全绿（JS gzip 79.9KB）；`cargo test` 89 passed；
  本机 `cargo tauri build --no-sign` 出 dmg/.app/updater tar 三产物；release 实机
  启动 Rust 全链路通过入工作台。步骤 20/21 的目视项（骨架观感、release 内
  /selector 直达 SPA fallback 实测）待人工复核。

### 2026-08-27 占用声明 · 前端迁移阶段 A 开工（ADR-0008 实施开始） —— guan

- 变更：占用 `frontend/`（新建）、`tauri.conf.json`（frontendDist 切换）、
  `.github/workflows/build.yml`（node 步骤合批）、`.gitignore`；阶段 E 收口时改动
  `AGENTS.md`（§1/§2/§3/§4/§5/§7）与 `docs/roadmap.md`（硬约束 2 与不做清单）。
  实施依据：ADR-0008 + `docs/frontend-migration.md`（commit aab68c1、f21df95）。
- 影响：**宪法级变更预告**——AGENTS §4.2「禁止引入前端构建链」将在阶段 E 按既定
  方案修订为「Vite + React 定向许可」；执行顺序 A 脚手架 → B About → C Mode/Selector
  → D Index → E 测试与治理收口，各阶段完成即在频道知会。阶段 A–D 仅新增文件，
  不动现有 `ui/` 与 Rust 行为；master 保持随时可构建。他人如需动上述文件请先在
  频道协调。
- 凭据：Tailwind v4 + shadcn/ui 兼容性 spike 已通过（2026-08-27 临时目录验证：
  shadcn init 显式识别 v4、七组件生成、strict TS 下 vite build 成功 JS gzip 73KB；
  结论与三注意点已回写方案 §1 并勾销 ADR-0008 行动项）。本条与 spike 回写同 commit。

### 2026-08-27 快车道直推 · 完成通知：roadmap 对照 dsh 源码核查修订 —— guan

- 变更：`docs/roadmap.md`（+43/−27；本档案与该改动同 commit 落盘）。对照
  deepseek-harness v0.1.1-rc.2 源码逐条核查路线图事实主张后修订：事实表重做
  （子包实测 227、`dsh plugin` = pnpm 原样转发、patch 按 id 逐字段赋值且 `config`
  整体替换不深合并、出厂 profile 模板仅 web/headless 且无任何 profile 管理官方命令、
  dsh 无插件安装/卸载 UI），新增「事实边界与陷阱」清单（非法名校验、profiles/node_modules
  符号链接农场、`.credentials.yaml` 三条硬约束等）；行动项修正：4.3 创建路径弃
  agent-presets 误用改为半官方 plugin-add 引导、4.4 pnpm 失败模式入错误处理并补全
  运行时状态枚举、4.2 updater 测试表述更新、编号引用与版本头（v0.4.7 起）修正。
- 影响：两项裁定仅周知——① 4.3④ 默认启动 profile 持久化到 `settings.json`，经
  维护者批准作为第二最小面例外（落地实现时同步登记 AGENTS §6 后方可合入）；
  ② Next 阶段（Profile 管理器）开工前须先做两个 spike：GUI 环境（无 shell rc）
  pnpm 经 `dsh plugin` 转发链的可用性验证、复制/重命名 profile 的引用清点。
- 凭据：纯文档改动不触运行时；事实主张均锚定 dsh 源码位置（app-boot/src/profile.ts、
  vendor/include/src/index.ts、apps/cli/src/plugin.ts 等）；残留检查干净
  （旧引用 197 包数/（3.4）/零测试 表述已全部清零）。

### 2026-08-27 建档 —— guan

- 变更：新建本档案；`AGENTS.md`（§2 目录树 / §8.5 收尾三件事 / §10 知会落档条目）
  与 `CONTRIBUTING.md` §0 各挂一处指针。
- 影响：协作者此后按上表登记知会；本次指针挂接触宪法级文件，本条即为对其的知会。
- 凭据：纯文档改动，不触运行时。

### 2026-08-27 补记 · 完成通知：删除前端顶栏「关于」入口 —— guan

- 变更：commit `8075eea`——`ui/index.html` / `ui/mode.html` / `ui/selector.html` /
  `src-tauri/src/lib.rs` / `build.rs` / `capabilities/default.json`。删除三页壳顶栏
  「关于」按钮及 `open_about` IPC 整链，与原生常驻入口重复（Windows 启动页问题报告）。
- 影响：**触宪法级**——`AGENTS.md` §7 IPC 注册表移除 `open_about`、「更新常驻入口」
  条目改写为裁定后状态；关于面板此后只能经菜单（macOS）/ 托盘（非 macOS）打开。
  仅周知，无需动作。
- 凭据：`cargo test` 全绿（89 passed）；diff −36/+6 已逐行人肉复核；本条视为对
  该次宪法修订的知会。
