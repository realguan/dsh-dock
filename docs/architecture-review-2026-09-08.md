# 架构评审记录（2026-09-08）

> 性质：**只读评审产出**，不是已批准的改造计划。每条结论都有 file:line 或实测证据，
> 改造前须按 AGENTS §9 判定是否先立 ADR，按 §8.1 拆成独立增量。
> 基线实测：`cargo test` 182 passed · `pnpm typecheck` 0 err · `pnpm test` 112 passed ·
> `oxlint` 0 warning · CI 三平台 clippy+test 全绿。
> 行数口径：`wc -l`；churn 口径：`git log --since="90 days ago" --name-only`。

## 0. 测量基线

| 区域 | LOC | 测试 |
|:---|---:|---:|
| `src-tauri/src` | 14,911 | 182 |
| `frontend/src` | 14,975 | 112 |
| `docs` | 6,642 | — |
| `scripts` | 762 | 1 套（python） |

热点（churn × LOC）：`lib.rs` 252,237（83 次变更 / 3,039 行）为 Rust 侧第二名的
6.3 倍（`executor.rs` 39,858），同时是全仓变更最频繁的源码文件与测试密度最低的文件
（4 tests/kLOC vs 全仓 12）。

## 1. 已做对的部分（优化须叠加其上，不得推翻）

1. `Executor` trait（`executor.rs:74-145`）是真正的深模块：接口小、语义完整、
   Local/Wsl 双实现 + SSH 预留。
2. 领域层纯函数 + 路径注入（`scan_profiles(home)` / `read_profile_detail(home, name)`），
   环境读取仅发生在边界 `resolve::user_dsh_home()`——182 个测试因此可离线跑。
3. 机器闸门：`ipc.rs:78-160` 三条 gate_tests（handler↔COMMANDS↔capabilities）、
   `build.rs` 由常量生成 ACL、`lib.rs:2665` 版本锁步、`updater.rs:461` 跨语言事件名。
   **实测复核：55 个命令在 attribute / handler / COMMANDS / capabilities 四面完全一致。**
4. CI 完整：`fmt --check`(Linux) + `clippy -D warnings`/`cargo test`(三平台)
   + 前端三闸门 + 发布脚本测试。
5. 前端零 IPC 绕过（`invoke` 仅存在于 `lib/tauri.ts`）；i18n 键对称由类型系统强制。

## 2. 问题清单（按杠杆排序）

### P1 — `lib.rs` 上帝模块，且是仓库第一热点

3,039 行含 7 类职责：55 个 IPC 适配器（866）· `ShellState`+会话生命周期（350）·
注入 JS 字符串（330）· `run()`+setup（277）· 窗口/菜单/更新 UI（688）·
mode 编排（254）· tracing（101）· 测试（129）。
`create_main_window` 单函数 352 行（`lib.rs:1684`），`run()` 273 行（`lib.rs:2142`）。

**改造方向**：`lib.rs` 保留组合根（模块声明 + `generate_handler!` + `run()`），
其余拆 `commands/`、`boot/`、`ui/`、`injected/`。
⚠️ `ipc.rs:90-104` 解析器按 `,` 切分 handler 块——改成 `commands::profiles::list_profiles`
后须同步改为取路径最后一段，否则闸门误红。

### P2 — `ShellState` 全局可变状态聚合体

`lib.rs:135-165` 单结构体 12 字段，生命周期与归属各异，21 处 `.lock().unwrap()`。
叠加 `panic = "abort"`：任何 panic = 进程直接消失，无错误卡、无 teardown、
可能留孤儿 dsh 子进程（绕过 §6 的 1:1 生命周期纪律）。

**改造方向**：拆 `SessionState` / `UpdateState` / `BootUiState`，用 `AppCtx` 组合。

### P3 — 330 行产品逻辑活在 Rust 原始字符串里（工具链看不见的代码）

`lib.rs` 三个注入脚本：链接钩子 24 行、内存策略 68 行（`lib.rs:1605`）、
悬浮胶囊 238 行（`lib.rs:1728`）。胶囊自实现快捷键匹配、`app:settings-changed`
订阅、Shadow DOM、拖拽、深浅色样式——无类型检查、无 lint、无测试
（仅 1 条断言子串的测试 `lib.rs:2692`），且与 Rust 侧 `ShellSettings.switcherShortcut`
语义重复。

**改造方向**（仓库内已有先例 `updater.rs:461` 的 `include_str!`）：
移至 `frontend/src/injected/*.js`（纳入 oxlint），Rust 侧 `include_str!`。

### P4 — Rust↔TS 契约：名字有闸门，形状没有

| 契约面 | 闸门 | 状态 |
|:---|:---|:---|
| 命令名 attribute/handler/COMMANDS/capabilities | ✅ 机器闸门 | — |
| 命令名 `frontend/src/lib/tauri.ts` | ❌ 无（今日实测 55/55 手工一致） | ✅ 已闸（批次 1） |
| 事件名 | ✅ `include_str!` | — |
| 事件/返回值**字段形状** | ❌ 无 | ✅ 已闸（批次 1） |
| 序列化命名风格 | ❌ 无约定：`profiles.rs`/`plugins.rs` 无 `rename_all`（snake_case），`sessions.rs:25`/`settings.rs:39`/`mcp.rs:18`/`diagnostics.rs:13`/`credentials.rs:16` 为 camelCase，TS 在 `types/ipc.ts` 内同时镜像两套（`ipc.ts:56 web_ui` vs `ipc.ts:180 projectName`） | ✅ 已定契约（批次 1，保持现状并闸住） |

Rust 改字段名 → TS 静默 `undefined`：编译绿、测试绿、运行时错。
`emit_step`（`lib.rs:2444`）另以 `serde_json::json!` 手拼 payload，连 Rust 侧结构体约束都没有。

**改造方向**：① `ipc.rs` gate 加第 4 条断言（`tauri.ts` invoke 名 ↔ COMMANDS）；
② IPC 结构体序列化 key 集断言 + checked-in fixture；③ `json!` payload 换结构体；
④ 统一 `rename_all = "camelCase"`。全部零新依赖。

**批次 1 落地（2026-09-08）**：①② 已做（见 §6）；③④ 未做——④ 属契约变更，
需先定「统一 camelCase 还是保持混用」的裁定（现状已被闸门钉住，不再是隐患）。

### P5 — 错误是字符串，UI 决策靠子串匹配

`Result<T, String>` 约 110 处。`classify_boot_error`（`lib.rs:2525-2552`）以
`detail.contains("credentials"|"network"|"timeout")` 决定错误卡标题/建议/按钮；
`read_error_detail`（`lib.rs:2555`）从日志刮 `Error:` 行。
同一代码库里 `updater.rs` 的 `ClientUpdate` 已是 `#[serde(tag="phase")]` 标签枚举
——更新状态机类型化、启动失败字符串化。

**改造方向**：boot 边界引入 `BootFailure { code, title, suggestion, actions, detail }`；
深域内保留 `String` 亦可，映射在 IPC 边缘完成。**属契约变更 → 先立 ADR。**

### P6 — 验证缺口正好落在风险最高的三处

| 缺口 | 证据 | 漏掉什么 |
|:---|:---|:---|
| 唯一网络面几乎无测试 | `updates.rs` 824 行 / 4 测试，全为纯解析器；`fetch_latest_version`(421)、镜像链回退、超时、`npm_packument_versions`(695)、`fetch_market_registry`(741) 全未测 | 干净机器引擎引导失败而 CI 全绿 |
| 唯一改用户数据的脚本靠可跳过测试 | `repair-session.mjs` 1,521 行；`sessions.rs:861/1022/1085/1181` 在 `node` 缺失时静默 `return` | 会话数据损坏无人发现 |
| WSL 路径仅手工验证 | 4 个 guest 测试 `#[cfg(all(unix, test))]`（`executor.rs:1366`），Windows CI 一个不跑；`boot-smoke.yml` 仅 `workflow_dispatch` | ADR-0004 主平台回归 |

**改造方向**：`updates.rs` 注入 HTTP 闭包 seam（照抄 `engines::bootstrap` 的注入模式，
`engines.rs:581-583`）；`repair-session.mjs` 改 fixture 驱动 + CI 环境 `node` 缺失硬失败。

### P7 — 前端：死代码 / 缺失效缝 / 规则只存在于文档

- **死代码**：`ProfileDetailDialog.tsx` 671 行，**全仓 0 引用**（实测 grep 仅自身定义），
  90 天内仍被改 8 次；与 `ProfileDetailPane` 有约 200 行近似重复。
- **缺缓存失效缝**：`ProfileDetailPane.reload()`（`:101-119`）只刷自己 4 个切片，
  从不通知 `useProfilesStore`；`ProfileRow` 用 store 的 `dependencies.length` 渲染
  → 装完插件左栏计数需窗口失焦再获焦才更新（`ProfileManager.tsx:75-79`）。
- **`.catch` 规则只写在 AGENTS.md**：oxlint 默认规则 0 warning；11 处
  `clipboard.writeText` 有 10 处无 `.catch`，其中 `ErrorCard.tsx:67` 完全游离且
  **失败仍显示「已复制」**。
- **4 个测试文件同义反复**：`credentials/diagnostics/mcp/sessions.test.ts` 仅
  `import type`，在测试体重实现逻辑后断言字面量，不触达生产代码。
- **i18n 泄漏**：25 个领域组件共 910 个中文字符（`PluginOverview` 136、
  `McpManager` 122、`ProfileDetailPane` 103…），en-US 用户看到中文。
- 未测纯逻辑：`lib/host.ts`（70 行，平台/WSL 判定 + UA 回退）。

### P8 — 文档/注释漂移与失效守卫

- `frontend/src/lib/tauri.ts:1` 写「20 个命令」（实为 55）；
  `docs/frontend-migration.md` §3.3 写「12 个命令」。
- `build.rs:9-15` 监视 5 个 `../ui/*` 路径，而 `ui/` 已不存在——**这条守卫是为一次
  真实踩坑（2026-08-23 复用旧前端）加的，现在完全惰性**。
- `scripts/regen-icons.sh:16` 引用已删除路径。
- **实测到一次 flaky**：`resolve.rs:975` 的 `probe_no_open_finds_flag_on_stderr`
  并行全量跑时挂在 `elapsed < 5s` 断言；单独跑 3 次 + 全量重跑 2 次均通过。
  同类挂钟断言还有 `resolve.rs:908-977`。
- 其它可跳过的测试：4 个 sessions 测试在无 `node` 时静默返回。

## 3. 建议批次（每批可独立合入）

| 批次 | 内容 | 风险 | ADR | 状态 |
|:---|:---|:---|:---|:---|
| 0a | 修 `build.rs` 惰性 `ui/*` 监视与 `regen-icons.sh` 死路径；删死代码 `ProfileDetailDialog`；删 4 个同义反复测试；修正易腐注释 | 无 | 否 | ✅ 已落地（见 §7） |
| 0b | 10 处 clipboard promise 假成功（`ErrorCard` 失败仍显示「已复制」） | 无 | 否 | ✅ 已落地（见 §8） |
| 1 | `ipc.rs` gate 加 `tauri.ts` 名集断言 + IPC 结构体 key 集 fixture 断言 | 无（纯新增测试） | 否 | ✅ 已落地（见 §6） |
| 2 | 按域拆 `lib.rs`：`commands/` → `ui/` → `boot/`（P1/P2） | 中 | 否（结构重构，不涉契约） | ✅ 已落地（见 §11/§12/§13） |
| 3 | 注入 JS 迁出 Rust（P3），先迁胶囊 238 行 | 低 | 否 | ✅ 已落地（见 §9，330 行全迁） |
| 4 | 错误类型化 `BootFailure`（P5） | 中 | **是** | ⚠️ ADR-0012 已立（草案）；实施待评审 |
| 5 | `updates.rs` HTTP seam + 离线测试；`repair-session.mjs` fixture 驱动；去 flaky | 低 | 否 | ⚠️ 部分（见 §10；HTTP seam 未做） |

## 4. 不建议做

- 不引入 `ts-rs`/`specta` 做类型生成（需 ADR；批次 1 的 fixture 闸门已覆盖九成风险，成本低一个数量级）。
- 不引入 TanStack Query（痛点实为缺失效通知，补事件比换数据层便宜）。
- 不为覆盖率而写组件测试（§5 明确 Vitest 只测纯逻辑；应先把组件内纯逻辑下沉 `lib/`）。
- 不全仓 `clippy --fix` / 批量格式化（§8.2 明令禁止）。

## 5. 落点建议

若只做一件：**批次 1**。零行为风险、不需 ADR、复用既有 `include_str!` 与 gate_tests
范式，直接封住 P4 的「编译绿、运行错」高危面，并为后续结构重构提供安全网。

## 6. 批次 1 落地记录（2026-09-08，`test(ipc): 契约形状跨语言闸门`）

新增三个闸门（`ipc.rs::gate_tests`，共 6 个测试）：

| 闸门 | 断言 | 实测抓漏 |
|:---|:---|:---|
| `tauri_ts_matches_ipc_commands` | `tauri.ts` 的 invoke 名集 ↔ `COMMANDS`（双向） | 改 `get_shell_settings` → `get_shell_settings_renamed` 即红 |
| `no_direct_invoke_outside_tauri_ts` | 全 `frontend/src` 递归扫描，`invoke(` / `invoke<` 只允许出现在 `lib/tauri.ts`（AGENTS §4.3） | 当前 0 命中 |
| `ipc_struct_shapes_match_fixture` | 14 个 IPC 结构体的**真实 serde 序列化 key 集** ↔ 共享 fixture | 改 fixture 的 `web_ui`→`webUi` 即红 |

**共享 fixture = 唯一事实源**：`frontend/src/types/ipc-shapes.json`。
Rust 侧 `include_str!` 读入并比对真实序列化结果；前端侧
`__tests__/ipcShapes.test.ts` 用 `AllKeys<T>`（`Required` 展开 + 多余属性检查）
断言 TS 接口 key 集与同一份 fixture 一致。**任一侧改名/换 casing 都会红**。

覆盖结构体（14）：`ShellSettings` · `ProfileSummary` · `SessionItem` · `PluginRowState` ·
`AggregatePlugin` · `AggregateSource` · `CopyConfigOutcome` · `RepairOutcome` ·
`SystemDiagnosticsReport` + 5 个诊断子结构。

**命名风格裁定（保持现状并闸住）**：`profiles`/`plugins` 域为 snake_case
（`web_ui` / `pkg_name` / `skipped_existing`），`sessions`/`settings`/`diagnostics`
域为 camelCase。批次 1 不统一（属契约变更，须先裁定），但两侧已钉死——
再改名必须同时改 fixture，无法再「静默 undefined」。

**未纳入**：③ `emit_step` 的 `json!` 手拼 payload 换结构体（`lib.rs:2444`，随批次 2
拆 `lib.rs` 时一并做）；④ 统一 `rename_all`。

## 7. 批次 0a 落地记录（2026-09-08，`chore: 清理惰性守卫、死代码与同义反复测试`）

| 项 | 处理 | 依据 |
|:---|:---|:---|
| `build.rs:9-15` 5 条 `../ui/*` rerun 监视 | **删** | `tauri-build` 2.6.3 自己就为 `frontendDist`（`src/codegen/context.rs:87-95`）与 `capabilities/`（`src/acl.rs:427`）发 rerun 指令；`ui/` 已不存在，那 5 行是 2026-08-23 事故的惰性遗留 |
| `scripts/regen-icons.sh:16,17,19` 死路径 | 改指 `assets/dsh-logo.svg` | `ui/assets/` 随迁移搬至仓库根 `assets/`（`frontend-migration.md:66` 早已记录） |
| `ProfileDetailDialog.tsx`（671 行，全仓 0 引用） | **删** | ADR-0011 §5 行动项（随小版本批） |
| 4 个同义反复测试（`credentials`/`diagnostics`/`mcp`/`sessions`） | **删** | 仅 `import type`，在测试体重写逻辑后断言字面量，从不触达生产代码 = 假覆盖率 |
| `tauri.ts:1`「20 个命令」/ `frontend-migration.md`「12 个命令」 | 前者改 rot-proof（指向 `COMMANDS` + 闸门）；后者加计数口径补注 | P8 文档漂移 |

**未纳入**：10 处 clipboard promise 假成功 → 批次 0b；「组件内纯逻辑下沉 `lib/` 再写真实测试」
（被删的 sessions/mcp 测试所覆盖的逻辑仍在组件内）→ 归 P7 后续批次。


## 8. 批次 0b 落地记录（2026-09-08，`fix(frontend): 剪贴板写入唯一入口`）

**缺陷**：11 处调用各自直呼 `navigator.clipboard.writeText(...)`，处理方式分四种：

| 写法 | 处 | 后果 |
|:---|:---|:---|
| 完全不接 promise | `ErrorCard.tsx` | 写失败仍显示「已复制」（最严重） |
| `.catch(() => {})` 后立即置位 | `BootStep.tsx` | 失败照样显示「已复制」 |
| 只挂 `.then` 成功分支 | 8 处 | 无失败反馈 + unhandled rejection |
| `.then` + 置位 | `BootStep.tsx:34`（评审原判「唯一正确」） | 同第 2 行——原判只看了「有没有 catch」 |

**收口**：新增 `lib/clipboard.ts::writeClipboard`（`write` 可注入，纯逻辑可测）+
`hooks/useCopy`（成功才置位、自动复位、不收回调参数以免引用不稳）。11 处全部改为
`await copy(...)` 并按结果分流：有 toast 渠道的走 `onNotice(t.error.copyFailed)`，
boot 页走 `logger.warn`。

**机器闸门**：`__tests__/clipboardGate.test.ts` 用 `import.meta.glob(?raw)` 扫全量源，
断言 `clipboard.writeText` 只允许出现在 `lib/clipboard.ts`——探针文件实测被抓出。

**测试**：`__tests__/clipboard.test.ts`（4 条）：成功透传原文 / 失败带原始错误不抛 /
非 Error 拒绝不抛 / 空串照写（调用方负责过滤）。

## 9. 批次 3 落地记录（2026-09-08，`refactor(tauri): 注入脚本迁出 Rust 原始字符串`）

**缺陷（P3）**：330 行 JS 活在 Rust `r#"…"#` 原始字符串里——tsc / oxlint / 语法检查
全部看不见，改错只能等运行时。

**落地**：三段脚本全部迁到 `frontend/src/injected/*.js`，Rust 侧改 `include_str!`
（同 `updater.rs:461` 的跨语言引用范式）：

| 脚本 | 原位置 | 现文件 | 行数 |
|:---|:---|:---|:---|
| 内存策略（`content-visibility` 注入） | `lib.rs:1605-1672` | `injected/memory-policy.js` | 68 |
| 外链兜底 hook | `lib.rs:1685-1708` | `injected/link-hook.js` | 24 |
| 悬浮胶囊 | `lib.rs:1728-1965` | `injected/switcher.js` | 238 |

`lib.rs` 3,039 → 2,713 行（−326）。

**迁出即见真章（工具链首次扫到这 330 行，立刻报 6 条）**：
- `memory-policy.js` 的 `scan(root)` **是死代码**（MutationObserver 已内联同逻辑，从未调用）→ 删。
- `switcher.js` 的 `var isMac = isMacPlatform()` 赋值未使用 → 删（`isMacPlatform` 本身仍被
  两处使用）。
- 4 处 `catch (e)` 未用参数 → 改可选捕获绑定 `catch {}`。

**为什么放 `frontend/src/injected/`**：`pnpm run lint` 覆盖 `frontend/src`，迁到这里即被
oxlint 纳管（当前 0 warning）；Vite 只打包被 import 的模块，这些文件不进产物；
`tsconfig.app.json` 未开 `allowJs`，tsc 不会误编译它们。

**验证**：`cargo test` 187 绿（含 `WEBVIEW_MEMORY_POLICY_SCRIPT` 的三条子串断言）·
`cargo fmt --check` 干净 · `node --check` 三个文件语法通过 · `pnpm lint` 0 warning。

## 10. 批次 5 落地记录（2026-09-08，`test(rust): 去 flaky + 强制 node 用例 + 网络面离线覆盖`）

**① 去 flaky（P8）**：`resolve.rs` 三处挂钟断言过紧，并行全量跑实测挂过一次：

| 用例 | 原断言 | 现断言 | 假体 sleep |
|:---|:---|:---|:---|
| `probe_no_open_returns_early_on_hit` | `< 10s` | `< 20s` | 30 → 60s |
| `probe_no_open_finds_flag_on_stderr` | `< 5s`（**实测挂过**） | `< 20s` | 30 → 60s |
| `probe_no_open_times_out_when_hung` | `< 3s` | `< 10s` | 60s |

口径：断言只需区分「命中即早退」与「等满自然退出」，故把假体 sleep 拉到 60s、断言留
3–6× 负载余量——回归形态必超（60s），负载抖动不再误红。

**② 静默跳过 → CI 硬失败（P6）**：`sessions.rs` 4 个修复链用例在缺 `node` 时
`return`（0 断言通过 = 假绿）。新增 `require_node_or_skip(test_name)`：本地缺 node 允许
跳过并打印提示，CI 设 `DSH_TEST_REQUIRE_NODE=1` 即 panic。`.github/workflows/build.yml`
的 Unit tests 步骤已挂该环境变量。**实测**：剥掉 PATH 里的 node + 打开开关 → 立即
panic（`PATH 上找不到 node，但 CI 要求真跑`）。

**③ 网络面离线覆盖（P6）**：`updates.rs::read_body_capped` 是网络层里唯一可离线全覆盖
的一环（`ureq` 的 `into_string()` 上限随版本漂移，本函数把它换成显式实现）——此前**零测试**。
补 5 条：未超限原样返回 / 恰好等于上限放行 / 超限报明确文案 / 空体合法 / 非 UTF-8 带上下文报错。
`updates.rs` 测试 4 → 9 条。

**未做（仍是批次 5 的欠账）**：`updates.rs` 的 **HTTP seam 注入**（把 `ureq::Agent` 调用
收成可注入闭包，离线覆盖镜像链回退/超时语义）；`repair-session.mjs` fixture 驱动。
现状：镜像链回退仍只能靠真网络或人工验证。

## 11. 批次 2 第一步落地记录（2026-09-08，`refactor(tauri): 拆出 commands 模块`）

**`lib.rs` 2,713 → 1,778 行（−935）**，55 个 `#[tauri::command]` 按域迁到
`src-tauri/src/commands/`：

| 模块 | 命令数 | 行数 | 域 |
|:---|:---|:---|:---|
| `commands/boot.rs` | 5 | 146 | 启动/运行环境/终端动作 |
| `commands/profile.rs` | 10 | 198 | profile 生命周期 |
| `commands/plugin.rs` | 12 | 234 | 插件安装/行表/聚合/构建审批 |
| `commands/console.rs` | 13 | 148 | 设置/诊断/凭据/引擎/MCP |
| `commands/session.rs` | 4 | 80 | 会话列表与自愈 |
| `commands/update.rs` | 5 | 46 | 壳与 dsh 升级 |
| `commands/link.rs` | 3 | 56 | 外链与工作台 URL |
| `commands/window.rs` | 2 | 62 | 窗口切换 |
| `commands/market.rs` | 1 | 13 | 市场 registry |

**先修闸门再搬**：`ipc.rs` 的 handler 解析器原本把条目当裸标识符——搬完会变成
`commands::profile::list_profiles` 而误判「未登记」。故先让解析器取**最后一段路径**，
并补 `handler_parser_strips_module_path`（合成源码正反例）——搬完 55 条 handler 全绿。

**可见性**：`ShellState` 与 17 个被命令调用的辅助函数改 `pub(crate)`；`EXTERNAL_URL_HOSTS`
同理。命令层只依赖 `crate::…`，不反向依赖 `commands::`（`lib.rs` 仅菜单回调一处改为
`commands::window::open_profiles_window`）。

**未纳入（批次 2 后续）**：`ui/`（`create_main_window` 352 行、菜单/托盘/窗口、
`resolve_resources_dir`）与 `boot/`（`run()` 400+ 行、会话守卫、`emit_*` 族）。
本次只搬**叶子层**（命令 → 域模块），不动启动管线，回归面可控。

## 12. 批次 2 第二步落地记录（2026-09-08，`refactor(tauri): 拆出 ui 模块`）

**`lib.rs` 1,778 → 1,372 行**（相对原始 3,039 已 −55%），窗口/菜单/托盘层迁到
`src-tauri/src/ui.rs`（423 行）：

| 迁出项 | 说明 |
|:---|:---|
| `create_main_window` | 主窗口创建 + 三个 `initialization_script` 装配 + 导航/新窗口拦截（93 行） |
| `resolve_resources_dir` | 打包资源目录解析（dev/release 双路径） |
| `build_app_menu` / `build_tray_menu` / `setup_update_tray` | 菜单与托盘（含 macOS/非 macOS 分叉） |
| `refresh_app_menu` ×2（`#[cfg]` 双实现）/ `current_active_mode` | 更新态在菜单上的刷新 |
| `open_about_window` | 关于窗口 |
| `WEBVIEW_MEMORY_POLICY_SCRIPT` | 注入脚本常量（只被 `create_main_window` 用）随之下沉；`lib.rs` 的测试引用改为 `crate::ui::…` |

依赖方向单向：`ui::` → `crate::{ShellState, is_allowed_external_url, settings, emit_*}`；
`lib.rs::run` 与 `commands::window` 调用 `ui::create_main_window`。

**未纳入（批次 2 最后一步）**：`boot/`——`run()`（400+ 行）、`ShellState` 与会话守卫
（`guard_session`/`teardown_session`/`run_executor_session`）、`emit_*` 族、
`classify_boot_error`/`read_error_detail`、`init_tracing`/`TeeWriter`。
这一步动的是启动管线本身，需单独一轮并逐段验证（boot 是最高风险路径）。

## 13. 批次 2 第三步落地记录（2026-09-08，`refactor(tauri): 拆出 boot 模块`）——批次 2 收口

**`lib.rs` 1,372 → 574 行**（相对原始 3,039 **−81%**），启动管线迁到
`src-tauri/src/boot.rs`（818 行）：

| 迁出项 | 说明 |
|:---|:---|
| `ShellState` + 字段 + `impl` | 壳进程内会话真相源（字段改 `pub(crate)`） |
| `run_executor_session` / `authenticate_workbench_session` / `session_is_current` / `engine_session_alive` / `teardown_session` / `guard_session` | executor 拉起与 1:1 生命周期守卫 |
| `launch_executor_after_probe` / `executor_for_mode` / `lib_boot_again` / `switch_mode` | 启动/切换/重启编排 |
| `emit_step` / `boot_sink` / `download_progress_bridge` / `emit_upgrade` / `emit_boot_error` / `emit_update` / `refresh_update_ui` | boot 遥测与更新态刷新 |
| `classify_boot_error` / `read_error_detail` / `read_log_tail` | 启动失败分类与日志刮取 |
| `TeeWriter` / `TeeWriterGuard` / `MakeWriter` impl / `init_tracing` / `install_signal_exit_handler` / `signal_exit_handler` / `SIGNAL_EXIT` | tracing 双写与信号监护 |
| `BOOT_TIMEOUT` / `BOOT_STALL` / `cached_update_status` / `cached_status_or_default` / `empty_update_status` / `active_session_profile` / `ensure_switchable_profile` | 常量与状态缓存 |

**`run()` 有意留在 `lib.rs`**：它是组合根（装配 Builder、注册 handler、挂菜单/托盘），
只做接线不承载领域逻辑——拆出去反而让「入口在哪」变模糊。

**最终模块地图**：`lib.rs`(574，入口+接线+子进程工具+契约测试) · `boot.rs`(818) ·
`ui.rs`(424) · `commands/`(9 文件 947) · `injected/`(3 文件 330 JS) + 18 个既有域模块。
`lib.rs` 从「全仓第一热点」变为薄入口层。

**验证**：`cargo test` 193 绿 · `cargo fmt --check` 干净 · `clippy -D warnings` 干净 ·
前端 `typecheck`/`lint` 0 warning/`test` 135 全绿（无行为变更，纯搬迁）。
