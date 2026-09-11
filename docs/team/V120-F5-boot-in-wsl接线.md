# V120-F5：`boot_in_wsl` 动作接线（假按钮修复）

> 日期：2026-09-11 ｜ 执行：`frontend` ｜ 任务：**task-52（T-F5）**，授权自 task-46 §7.1 上报
> 证据强度：**实测**（行为复现 + 两轮变异测试 + 闸门逐条 exit code + sha256 校验复原）
> 基线：43 files / 346 passed → 交付后 **44 files / 359 passed**
> 范围：`frontend/src/**`（4 个改动文件 + 1 个新测试）；**未动 Rust / 依赖**；**禁 git 写**（未执行）

## 0. 闸门（逐条独立执行）

| 命令 | exit | 输出 |
| :--- | :--- | :--- |
| `cd frontend && pnpm run typecheck` | **0** | `tsc -b --pretty false`（无输出） |
| `cd frontend && pnpm run lint` | **0** | `Found 0 warnings and 0 errors.`（143 files） |
| `cd frontend && pnpm run test` | **0** | `Test Files 44 passed (44)` / `Tests 359 passed (359)` |

残留扫描 `grep -rE 'zz_|MUTATION|QA 注入' frontend/src/` ⇒ **0**。

## 1. 契约核对（**先核对，不臆测**）

任务要求「以 `docs/team/V120-D1-引擎引导全局安装.md` §5 为准，不要臆测字段名」。我**同时**核了
D1 文档与 **Rust 源码**（文档可能滞后于代码，源码才是真相）：

| 契约项 | D1 §5 声明 | Rust 源码实测 | 一致？ |
| :--- | :--- | :--- | :--- |
| kind 值 | `symlink_privilege_required` | `boot_failure.rs:274` 断言 `v["kind"] == "symlink_privilege_required"` | ✅ |
| actions | `["boot_in_wsl","retry"]` | `boot_failure.rs:135` `vec!["boot_in_wsl", "retry"]`；`:271` 有断言 | ✅ |
| IPC 方法 | `api.bootInWsl()`（**非** `terminalAction`） | `lib/tauri.ts:49` `bootInWsl: () => invoke<void>("boot_in_wsl")`；`ipc.rs:26` 已登记 | ✅ |
| 文案 | 标题「系统权限不足：无法创建符号链接」+ 建议（含"这不是网络问题"） | `boot_failure.rs:104 / :119-127` 逐字一致 | ✅ |
| 类型字面量 | `types/ipc.ts` 如需同步「只加不重排」 | `BootFailureKind` 联合**确实**缺该字面量 | ✅ 需同步（已做） |

**结论：契约与源码一致，无冲突需上报。** 未发现 D1 描述与实现不符之处。

## 2. 缺陷与行为复现（复现先行）

### 2.1 根因（读源码确认，非推断）

改前 `run()`（`ErrorCard.tsx:93-107`）是：

```ts
if (!INVOKABLE.has(id)) return                  // INVOKABLE = {retry, upgrade, upgrade_only}
...
api.terminalAction(id as TerminalAction)        // 隐式假设：所有动作都走这一条 IPC
```

`boot_in_wsl` **既不在 `INVOKABLE`、也不是 `TerminalAction` 联合类型成员** ⇒
两道门都把它挡下 ⇒ **按钮渲染出来但点了没反应（假按钮）**。

### 2.2 行为复现（**实测**，非"测试文件缺导出"式的弱证据）

用探针直接按改前源码的 `INVOKABLE` 定义求值：

```text
INVOKABLE = retry, upgrade, upgrade_only
boot_in_wsl 命中 INVOKABLE = false
=> run() 会直接 return（假按钮） = true
AssertionError: 契约下发的 boot_in_wsl 不在 INVOKABLE ⇒ 点了没反应
                expected false to be true      ← 行为红（探针已删除）
```

> **证据分级说明（如实标注）**：新门禁文件对 HEAD 整体跑出 13 条红，但其中多数是
> `TypeError: resolveActionCall is not a function`——属**「新导出尚不存在」的弱红**。
> 真正的**行为红**是上面这条探针（改前源码结构 ⇒ `boot_in_wsl` 必然被 `run()` 丢弃）。
> 故本任务的复现证据以**探针 + 两轮变异测试**（§5）为准，不以"文件级红"充数。

## 3. 改动（4 文件）

| 文件 | 改动 |
| :--- | :--- |
| `components/boot/ErrorCard.tsx` | ① 新增 `ActionIpc` 类型与 **`ACTION_IPC` 分派表**（唯一事实源）；② 导出 `INVOKABLE_ACTIONS`（**由分派表派生**，杜绝"集合/分派"两处漂移）；③ 导出纯函数 `resolveActionCall(id)`；④ **重写 `run()`**：经 `resolveActionCall` 分派，`bootInWsl` 走 `api.bootInWsl()`、其余走 `api.terminalAction()`；未知 id 才 return |
| `content/zh-CN.ts` | `error.actions.boot_in_wsl` = 「改用 WSL 模式打开」（D1 §5 建议文案）；`error.kinds.symlink_privilege_required` 的 title/suggestion（与 `boot_failure.rs` 对齐，含"这不是网络问题"） |
| `content/en-US.ts` | 同上英文对称：`"Switch to WSL mode"` + 对应 title/suggestion（`enUsNoLeak` 口径：无 CJK） |
| `types/ipc.ts` | `BootFailureKind` 补 `symlink_privilege_required`（**只加不重排**，位置在 `network_unavailable` 与 `unknown` 之间，与 Rust 枚举序一致） |

**未动**：`F1–F4` 的任何代码（本任务只做 F5）；其它邻近问题（按 task 要求只报告，本任务未发现新增项）。

## 4. 测试（`__tests__/errorCardActions.test.ts`，13 例）

本仓禁 RTL/jsdom ⇒ **无法做 DOM 点击测试**，故把「id → 该调哪条 IPC」抽成纯函数后做机器化断言：

| 组 | 覆盖 |
| :--- | :--- |
| 分派（防假按钮） | D1 契约的每个 action 都可分派（`not.toBeNull`）；`boot_in_wsl` **必须** `{ipc:"bootInWsl"}`；`retry/upgrade/upgrade_only` 走 `terminalAction`；`INVOKABLE_ACTIONS` 与分派表**一致**；未知 id 返回 `null`（保留"未知动作不猜"） |
| kind 文案齐备 | 两字典均有 title/suggestion；en 侧无 CJK；**zh 必须含「不是网络问题」**（D1 §4 设计要点：掐断错误排查方向）；引导含 WSL；action 文案键存在且**不是英文 id 兜底** |
| 防漏配 | 逐一断言契约中的 5 个 kind 在两字典都有文案（新增 kind 漏配即红） |
| **组件接线**（`?raw`，剥离注释） | `run()` **确实**使用 `resolveActionCall(id)`；`api.bootInWsl()` 与 `api.terminalAction(` 同时存在；`id as TerminalAction` **只出现一次**（防"统一 cast 蒙混"回流）；反向断言防过度剥离假绿 |

## 5. 变异测试（**证明门禁真有牙齿**，非"今天恰好绿"）

前几轮教训：断言可能变装饰（vacuous）或只匹配形态。故对本门禁做**两轮注入**：

| # | 注入 | 结果 | 结论 |
| :-- | :--- | :--- | :--- |
| M1 | 从 `ACTION_IPC` **移除** `boot_in_wsl`（= 回退成假按钮） | **3 failed**：分派表断言 / `boot_in_wsl` 目标断言 / 集合一致性断言 | ✅ 精确抓住"假按钮"回归 |
| M2 | `run()` **绕过分派表**、统一 `id as TerminalAction`（= 恢复隐式假设） | **1 failed**：`run() 使用 resolveActionCall 且两条 IPC 都出现` | ✅ 抓住"表存在但没接线" |

两轮均 `trap` 复原，sha256 前后一致（`ErrorCard.tsx` 各自 RESTORE_OK）。
**M2 是这条修复最容易漏的失败模式**——只建表不接线，表面上"有分派表"却仍是假按钮。

## 6. 契约与形状闸门影响

- `types/ipc-shapes.json` **不含 `BootFailure`**（实测 grep 无命中）⇒ 本次 kind 字面量同步
  **不影响**跨语言形状闸门（`ipcShapes.test.ts` 全绿）。
- 前端新增的「所有 kind 都有文案」断言与 Rust `boot_failure.rs` 的枚举**人工对齐**；
  若将来 Rust 新增 kind，需同步两处——已在 `types/ipc.ts` 注释中写明对齐关系。

## 7. 未决 / 风险

1. **Windows 无实机**：本任务结论 = **「机制级修复 + 单元/契约级验证，真机待复验」**，
   **不得**表述为"已修好"。可本机验证的部分（分派逻辑、文案、类型、接线）已全部覆盖；
   真实的「点击 → WSL 拉起」行为需 Windows 实机。
2. **动作可见性未加过滤**：`actions` 数组完全由后端下发，前端不按平台过滤。
   若某天后端在非 Windows 平台下发 `boot_in_wsl`（`boot_in_wsl` 在非 Windows 会
   `Err("WSL 仅支持 Windows 平台")`，见 `commands/boot.rs:71-74`），用户会看到按钮但点击得到
   错误提示。**当前不会发生**（该 kind 仅在 Windows 符号链接失败时产生），
   故**未动**——按 task 要求"不碰邻近问题，只报告"。
3. **`reselect` 不在分派表内**（既有设计）：它由 `onReselect` prop 单独承接、不经 `actions` 数组，
   故未纳入 `ACTION_IPC`。已在测试中用"未知 id 返回 null"覆盖其边界。
