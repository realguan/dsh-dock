# Spike B：插件清单与运行时状态获取路径验证（4.4 前置）

- **日期**：2026-08-29
- **执行人**：guan（AI 代理协作）
- **状态**：✅ 已完成——静态清单零新依赖；运行时状态 = 回环 HTTP POST 单调用，已实机打通
- **输入**：roadmap 4.4 关键行动 ①（运行时状态需 dsh 经 Typert RPC 的 `pluginInventory.list()`）与 ③（启用/禁用需 id 列表）
- **输出**：本文档 + 复现台账复现点 11 → 喂给 4.4 实施方案（冻结期结束后开工）

---

## 1. 问题定义

4.4 要在壳里列出 profile 的插件并显示运行状态（pending/loading/active/failed/disposed/unloading）。
三个未知：

1. `pluginInventory.list()` 是什么传输？壳（Rust，非浏览器、非 dsh 进程内）够得着吗？
2. `cordis.patch.yml` 的 `{id, disabled}` 写入，`id` 用哪个空间的值？
3. 插件版本/元数据从哪读？

## 2. 结论（先答问题）

### 2.1 运行时清单 = 回环 HTTP POST，一次调用，无需 WebSocket

浏览器客户端虽有 WS 复用流（`/api/events.mux`），但 unary 调用同时走**普通 HTTP POST**
（`dsh-client-connection` client.js `callUnary` @ 6203-6211：构造信封后
`postJson("/api/${method}", message)`）。实机验证（2026-08-29，本机运行中的 dsh 实例）：

```
POST http://127.0.0.1:<port>/api/pluginInventory/list
Content-Type: application/json

{"type":"client-request","rpcId":"<任意唯一串>","method":"pluginInventory/list","payload":{"args":{}}}

→ 200 {"type":"server-response","rpcId":"…","result":{"ok":true,"value":{"entries":[…]}}}
```

- `payload` 必须恰有一个 plain-object `args` 字段（`{}`/`{args:[]}` → 200 但
  `ok:false, code:"internal"`「Remote payload must contain exactly one plain-object args field」）。
- 响应条目形状（`dsh-host-plugin-inventory/lib/typert.host.js` zod schema，逐字）：
  `{ entryId, moduleName, enabled, fiberPhase }`，`fiberPhase ∈
  null("disposed")|failed|pending|active|loading|unloading`（FiberState 枚举映射 @ 33-46）。
- 实测全量 168 条：官方 bundle 行（`include:*` 前缀 entryId）与第三方插件
  （`dsh-better-sidebar` 等 3 个，恰为 web 档 dependencies）同场返回；
  被 patch `disabled: true` 的行 = `enabled:false` + `fiberPhase:null`（disposed）。
- **无鉴权门**：伪造 `Host: evil.example.com` 仍 200——本机任意进程都能驱动工作台
  API。这是 dsh 既有安全姿态（与壳同机即同信任域），壳侧照用即可，但不熟视无睹：
  壳新增的这条回环调用只读，不放大暴露面。
- 语义边界：这是**一次性快照**（roadmap 已裁定不订阅变化）；WS 流才是订阅通道。

### 2.2 CLI 动词面 = 原样转发 pnpm（安装/卸载/更新零新机制）

`dsh plugin --profile <名> --help` 输出即 pnpm help：`add` / `rm,remove` /
`up,update` / `ls,list` / `outdated` / `why` / `audit` 全部可用——4.4 安装/卸载/更新
就是 4.3 创建链（`shell.rs` `spawn_dsh` 同款转发）换个动词，pnpm 失败模式处理已就位。

### 2.3 启用/禁用的 id 空间：patch 行 id ≠ inventory entryId，需映射

两个空间并存：

- **patch/配置行 id**（`cordis.patch.yml` 的 `{id, disabled}` 与 `--dump-config`
  行内的 `id:`）：无组前缀，如 `tool-subagent-report`、`workflow-worker-thread`。
- **inventory entryId**：loader 树路径，带组前缀，如 `include:tool-subagent-report`、
  `include:agent-presets:tool-bash`（组嵌套用 `:` 连接）、动态条目甚至用哈希
  （`467ee25f`）。

映射规律：`include:` 段后缀 ≈ 配置行 id，但**组嵌套行**（agent-presets 等）的
entryId 不等于行 id。⇒ 4.4③ 禁用写入的 id 来源应以 **`--dump-config` 的行 id**
为准（dump-config 就是「组合后配置行清单」，行 id + disabled 现状一目了然），
inventory 只做运行态展示，不作为 patch 写入的 id 源。`{id, disabled}` 单键写入
不碰原行 config 的语义维持 roadmap §1 已核结论（2026-08-27）。

### 2.4 静态清单：零新机制，两条读取路径已验证

- **装了什么**：profile `package.json` 的 `dependencies`（第三方，含版本）+
  `dsh.profile.bundles`（官方内置）。web 档实测：patch 层为空 `[]`，
  第三方插件纯靠 dependencies 加载（loader 自动合成行，无需 patch 行）。
- **版本/描述**：`node_modules/<pkg>/package.json` 可读（符号链接农场不直写，
  只读穿透没问题；实测 `dsh-better-sidebar` → `0.16.1`）。
- 官方 bundle 的版本锚在 dsh 安装目录（`resolveBundleDir` 双锚点，Spike A §3.2
  已核），不在 profile node_modules——展示时版本可标「随 dsh」。

## 3. 对 4.4 实施的含义

1. **运行态获取 = 壳新回环调用**（复现点 11）：仅当 profile 会话在跑（有
   workbench URL）时调 `POST /api/pluginInventory/list`；未运行显示静态信息
   （roadmap 原裁定不变）。**治理项**：回环 HTTP 调用是 §7 网络面新用途（虽然
   127.0.0.1 且只读），实施前须在 AGENTS §7 登记例外。
2. **安装/卸载/更新**：转发链换动词，无新坑；`add` 裸包名 dist-tag 坑已核
   （复现点 7），4.4 安装 UI 应带版本参数或引导 tag 选择，避免重蹈 latest。
3. **启用/禁用**：写 patch 前置 = 新三件套写入面 → **需扩 ADR-0009**（例外 #3
   候选），id 从 dump-config 取。
4. **dsh 升级复核项**：typert 信封形状（`type/rpcId/method/payload.args`）与
   inventory schema（`typert.host.js`）都是生成物，随版本漂移风险中等——入台账，
   dsh 升级逐条复核。

## 4. 复核清单（dsh 升级时）

- [ ] `POST /api/<namespace>/<method>` unary 路由仍在（client-connection
      `callUnary` @ 6203）；
- [ ] `payload.args` 恰一 plain-object 字段约束未变；
- [ ] `pluginInventory/list` schema 未变（`typert.host.js`）；
- [ ] FiberState→phase 映射未变（index.js @ 33-46）；
- [ ] 回环调用无新增鉴权门（若有，壳侧需适配——预期 会以 Origin/fence 形式出现）。
