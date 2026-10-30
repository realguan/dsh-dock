# ADR-0035：工作台身份由 token 握手裁定，日志地址只作候选

- **日期**：2026-10-30
- **状态**：已接受
- **提出人**：维护者（真机事故驱动）
- **相关方**：`src-tauri/src/shell.rs`（就绪等待）、`src-tauri/src/boot.rs`（启动编排/身份兑换）、`src-tauri/src/ui.rs`（导航放行）、`src-tauri/src/lib.rs`（回环真相源）
- **关联**：ADR-0001（就绪判定=进程存活+日志进展）；复现点 1 / 10 / 11（`docs/contracts/dsh-behavior-ledger.md`）；`docs/contracts/ipc-and-network-register.md` §二 工作台 Token 环回兑换

---

## 1. 背景与问题

2026-10-30 一位 Windows 用户的 WSL 客体模式**永久卡在启动屏**：控制中心 5/5 全绿
（含「已进入工作台」），主窗口却始终停在壳的启动页；同一时刻托盘的
「用浏览器打开」能正常进工作台。诊断台原始日志（用户提供）：

```
[llm-deepseek-account] model listing from https://api.deepseek.com/models loaded 2 model(s), skipped 0 row(s)
dsh web: http://127.0.0.1:42135/?token=SL6rrH8emdKUA4mmfBp_IfNjO89tw_-RYyVf4pIVhw0
```

而壳日志打的是：

```
boot: step step=3 state=done detail=DSH 已就绪，地址：https://api.deepseek.com/models
boot: step step=4 state=done detail=已进入工作台
外链导航拦截：url=https://api.deepseek.com/models allowed=true
```

故障链（全部在代码里可逐条对上）：

1. `shell.rs::parse_detected_url` 取的是**日志里第一个以 `http(s)://` 开头的词**，
   不校验主机。dsh 的 stdout 是**所有插件共享**的自由文本，账号插件先把 deepseek 的
   models 接口地址打了出来，壳于是把它当成"工作台地址"。
2. `wait_for_ready` 立刻返回 `Ready("https://api.deepseek.com/models")`，
   `workbench_url` 被污染，step 3 播报了一个**从没被验证过**的地址。
3. 该 host 命中 `EXTERNAL_URL_HOSTS`（`deepseek.com`），`ui.rs::on_navigation`
   把导航判成外链 → 交给系统浏览器打开、**返回 false 拦下 WebView**。
4. `navigate()` 的返回值被丢弃，壳直接 `HandoffPhase::Ready` + `emit_step(4,"done")`
   —— **UI 断言成功，实际什么都没载入**。
5. `guard_session` 只盯进程是否退出；进程好好的，于是既不报错也不重试 → 永久卡住。

两层缺陷，第二层比第一层更致命：

- **A（判据）**：把"文本里出现过 URL"当作"工作台就绪/这就是工作台"。
- **B（宣告）**：把"导航被受理"当作"已经进去了"——成功是**猜**出来的，不是**观察**到的。

ADR-0001 §3 方案 C 早就写下"日志文案属上游实现细节，壳依赖它 = 跨边界耦合"，
但实现只把它降级成"拿 URL 导航"，判据仍绑在日志上。本 ADR 把那条原则落成机制。

## 2. 约束与硬指标

1. **唯一网络面 = `updates.rs`**（ADR-0006）；新的回环探测必须复用**已登记豁免**
   的条目（`network_gate::EXEMPTIONS`），否则 `cargo test` 直接红。
2. 不得修改 dsh 源码（红线 1）；只能读它的输出、调 CLI、在文件系统层复现。
3. 就绪判定不得依赖上游日志**文案**（ADR-0001 §3-C）。
4. 失败必须对用户可见，且**不能烧掉逃生口**——托盘「用浏览器打开」在事故里是唯一出路。
5. 平台覆盖不缩窄：宿主 + WSL 客体同口径（红线 3）。
6. launch token 是进程级凭据，**不得进日志、不得进 URL 栏、不得发给非回环主机**。

## 3. 备选方案及评估

### 方案 A：候选 ≠ 就绪——回环+token 收窄候选，token 握手裁定真伪，导航结果必须观察 ✅ 采纳

- **候选判据**：`parse_workbench_candidate` 只接受「回环主机 + 查询串里有非空
  `token=`」的地址（原 `parse_detected_url` 收敛而来）。这只是**提案**。
- **真伪判据**：候选交给 `boot.rs::authenticate_workbench_session` 做 token 握手。
  上游每次启动生成 `randomBytes(32)` 的 process launch token，`authenticatedUrl()`
  恒把它拼进 ready 行 URL；只有**我们 spawn 的那个 dsh** 认它 —— 有效则
  `303 + set-cookie`（随后 `browserAuth` 放行），无效则 401。这条握手同时证明
  「webserver 已监听 + browserAuth 已生效 + index 已可服务」，是**真就绪**的正向证据。
  握手失败只意味"这个候选不是我们的工作台"，`wait_for_ready` 换下一个候选继续等。
- **宣告判据**：导航后**观察**主窗口 URL 的 origin 是否落到工作台 origin
  （`confirm_workbench_entered`，100ms 轮询/15s 上限）。未落地即 step 4 报错，
  **不再**有"假绿"。
- 对照约束：① 复用既有豁免条目、零新增网络原语；② 不改 dsh；③ 判据只剩"带 token
  的回环地址"这一**结构性线索**，握手才是结论；④ 失败走 `emit_boot_error`，
  会话**保留**（逃生口不烧）；⑤ 主机/客体同一代码路径；⑥ token 不进日志
  （step 3 播报的是去掉 query 的干净地址）。

### 方案 B：壳自选端口（bind `:0` → 读回端口 → `--port <n>`），把未知变成已知 ❌ 本轮不做

- 思路：地址不再需要"解析"，壳 spawn 前就知道端口，`workbench_url` 可直接构造。
- 否决理由（不是错，是**代价/风险不划算**）：
  1. WSL 客体的端口在**另一个命名空间**——宿主 bind 的端口与客体 dsh 监听的端口
     不是同一张表，"先 bind 再放掉"在宿主侧毫无约束力；
  2. 触碰 `executor.rs::guest_boot_script` 及其**逐字节相等闸门**
     （`executor.rs:1369-1374` 断言脚本含 `--port 0`），跨宿主/客体两条链路都要改；
  3. 端口预占窗口（放开 → dsh bind 之间）反而制造一个**新的**竞态，而它带来的
     收益已被方案 A 的握手覆盖。
- 保留为将来选项：若上游提供"把端口写进机器可读的 ready 文件"或客体内分离口查询，
  该方案可重开（见 §6 复核项）。

### 方案 C：只给 `parse_detected_url` 加主机白名单 ❌ 否决

- 思路：`starts_with("http://127.0.0.1")` 之类的最小修。
- 否决理由：仍然把"文本"当结论。上游改端口写法、加 `localhost`、走 IPv6、
  或插件恰好打了个回环 URL（本地工具链、MCP 面板地址天天有回环 URL），
  任何一次都会复发——**这一类**故障没有被消除，只是把门槛抬高了一格。

### 方案 D：只加超时/重试/重开窗口 ❌ 否决

- 否决理由：事故里进程活着、导航被受理、UI 全绿，任何基于"超时/重试"的补丁都在
  给一个**没有被观测到**的失败加延时。它治不了"假成功"这一根因。

## 4. 决策

采纳方案 A。落地形态：

1. `lib.rs::is_loopback_host` 成为回环主机**唯一真相源**（`ui.rs::on_navigation`
   与 `shell.rs` 候选解析共用，此前两处各写一份字符串集）。
2. `shell.rs`：`parse_detected_url` → `parse_workbench_candidate`；`wait_for_ready`
   增加 `confirm: &mut dyn FnMut(&str) -> Option<String>`，同候选每 400ms 重试、
   最多 25 次（≈10s）后拉黑换下一个——**候选被拒 ≠ 启动失败**，错误候选不得吃掉
   整个 `BOOT_TIMEOUT`。
3. `boot.rs::authenticate_workbench_session` 语义收紧为"身份裁定者"：
   回环校验前置、无 token / 无 `set-cookie` / 非 303 一律 `Err`
   （旧的 `Ok(None)` 回退路径删除——那正是"把猜测当真身"的代码）。
4. `boot.rs` `Ready` 分支：`workbench_url` 只写**握手通过**的干净地址；step 3 播报
   该干净地址（launch token 不再进壳日志）；`navigate()` 的 `Err` 与
   `confirm_workbench_entered` 失败都走 step 4 报错 + `emit_boot_error`，会话保留。
5. **逃生口另存一份带 token 的地址**（`ShellState::workbench_browser_url`，只在内存、
   不进日志/前端）：托盘的「在浏览器中打开」经 `ShellState::browser_open_url()` 优先取它。
   系统浏览器没有壳内 WebView 的 cookie jar，拿干净地址进去恒 401——若不同时记住
   带 token 的原地址，这次修复就会**顺手烧掉事故里唯一的出路**。
6. **脱敏前置到源头**（`boot.rs::redact_url` + `shell.rs` 放弃候选的 warn）：身份裁定
   失败时 `bail!` 文案会同时进壳日志与用户可见的错误卡，而候选 URL 里就带着 token；
   故日志只保留 origin/port，query 一律变 `?redacted`。

## 5. 负面后果

- **一次额外回环请求**：每次启动多一次 GET（此前也已存在，只是从"就绪后"提前到
  "裁定候选时"）；候选被拉黑时最多再叠几次（有 25 次/候选的上限）。
- **launch token 在内存里多存一份**：`workbench_browser_url` 与 `workbench_cookie`
  同级机密（AGENTS §4.3：不落盘、不进日志、不下发前端）。上游 `authorizeIndex`
  **只比对不消费** `launchToken`（`tokenMatches(url.get("token"), this.launchToken)`，
  源码里没有 revoked/used 标记；`processLaunchToken(owner)` 还是按 owner 记忆的），
  所以带 token 的地址在 dsh 进程存活期内可反复兑换 cookie——这正是逃生口成立的前提。
  但**不要**把它当通用手段：WebView 侧仍走已注入的 Cookie，重放 token URL 只服务于
  "另一个没有 Cookie 的浏览器进程"这一种情形。
- **判别力来自上游**：`token=` 与 `303+Set-Cookie` 都是上游 `browser-auth` 的契约。
  上游若改成"URL 里不放 token"或"cookie 换名换位置"，握手会**集体失败**——
  但那种失败是**响亮**的（step 3 直接报"连续未通过"），不是这次这种静默假绿。
- **仍不覆盖**：dsh 起得来但页面自身 500/白屏（属 dsh 内部问题，origin 已落地，
  由 `guard_session` 与 dsh 自身错误呈现覆盖）。

## 6. 复核项（随 dsh 升级逐条复核，命中即更新复现点 1）

1. ready 行仍是 `dsh web: <回环URL?token=…>` 且 `authenticatedUrl` 恒附 token；
2. 有效 token 仍回 `303 + set-cookie`（名字/属性/`SameSite` 可漂移，判定只看
   "有无 Set-Cookie"）；
3. token 仍是**可重复兑换**（只比对不消费）——若哪天改成一次性，逃生口必须改走
   "复用 Cookie"路线（浏览器进程拿不到 WebView 的 jar，届时需另一套凭据下发）；
4. 上游是否提供机器可读的 ready 交接（文件/端口声明）——若有，方案 B 转为首选，
   候选解析可整体删除；
5. `--port 0` 语义是否仍在（复现点 10），以及客体内是否新增分离口查询。
