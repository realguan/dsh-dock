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

| 契约面 | 闸门 |
|:---|:---|
| 命令名 attribute/handler/COMMANDS/capabilities | ✅ 机器闸门 |
| 命令名 `frontend/src/lib/tauri.ts` | ❌ 无（今日实测 55/55 手工一致） |
| 事件名 | ✅ `include_str!` |
| 事件/返回值**字段形状** | ❌ 无 |
| 序列化命名风格 | ❌ 无约定：`profiles.rs`/`plugins.rs` 无 `rename_all`（snake_case），`sessions.rs:25`/`settings.rs:39`/`mcp.rs:18`/`diagnostics.rs:13`/`credentials.rs:16` 为 camelCase，TS 在 `types/ipc.ts` 内同时镜像两套（`ipc.ts:56 web_ui` vs `ipc.ts:180 projectName`） |

Rust 改字段名 → TS 静默 `undefined`：编译绿、测试绿、运行时错。
`emit_step`（`lib.rs:2444`）另以 `serde_json::json!` 手拼 payload，连 Rust 侧结构体约束都没有。

**改造方向**：① `ipc.rs` gate 加第 4 条断言（`tauri.ts` invoke 名 ↔ COMMANDS）；
② IPC 结构体序列化 key 集断言 + checked-in fixture；③ `json!` payload 换结构体；
④ 统一 `rename_all = "camelCase"`。全部零新依赖。

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

| 批次 | 内容 | 风险 | ADR |
|:---|:---|:---|:---|
| 0 | 删 `ProfileDetailDialog`；修 10 处 clipboard promise；修 `build.rs` 惰性 `ui/*` 监视与 `regen-icons.sh` 死路径；删 4 个同义反复测试；修正易腐注释 | 无 | 否 |
| 1 | `ipc.rs` gate 加 `tauri.ts` 名集断言 + IPC 结构体 key 集 fixture 断言 | 无（纯新增测试） | 否 |
| 2 | 按域拆 `lib.rs`：`commands/` → `ui/` → `boot/`（P1/P2） | 中 | 否（结构重构，不涉契约） |
| 3 | 注入 JS 迁出 Rust（P3），先迁胶囊 238 行 | 低 | 否 |
| 4 | 错误类型化 `BootFailure`（P5） | 中 | **是** |
| 5 | `updates.rs` HTTP seam + 6 个离线测试；`repair-session.mjs` fixture 驱动；去 flaky | 低 | 否 |

## 4. 不建议做

- 不引入 `ts-rs`/`specta` 做类型生成（需 ADR；批次 1 的 fixture 闸门已覆盖九成风险，成本低一个数量级）。
- 不引入 TanStack Query（痛点实为缺失效通知，补事件比换数据层便宜）。
- 不为覆盖率而写组件测试（§5 明确 Vitest 只测纯逻辑；应先把组件内纯逻辑下沉 `lib/`）。
- 不全仓 `clippy --fix` / 批量格式化（§8.2 明令禁止）。

## 5. 落点建议

若只做一件：**批次 1**。零行为风险、不需 ADR、复用既有 `include_str!` 与 gate_tests
范式，直接封住 P4 的「编译绿、运行错」高危面，并为后续结构重构提供安全网。
