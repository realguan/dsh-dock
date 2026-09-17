# dsh 行为复现台账（Behavior Ledger）

> 红线 1 允许「文件系统层复现 dsh 行为」——复现 = 与 dsh 源码的影子同步。
> dsh 升级后复现点可能**静默漂移**（CI 测的是壳自己的逻辑，发现不了）。
> 本台账是全部复现点的唯一登记处；**宿主解析命中新版 dsh（或 dsh 大版本升级）时，
> 须逐条复核第一节并更新「最后复核」列**，复核结论随广播知会。
> 锚点注释书写规范见 AGENTS.md §0 红线 1（源码参考位置 + 日期注释）；
> 决策推理见对应 ADR，本册只登记「是什么、在哪、锚什么」。

**基线**：dsh v0.1.1-rc.2（2026-08-27 架构核查口径，见 roadmap §1）

## 一、已落地复现点（随每次 dsh 升级复核）

| # | 复现点 | 壳侧位置 | 依赖的 dsh / 工具链行为 | 最后复核 |
|:--|:--|:--|:--|:--|
| 1 | dsh 就绪判定 | `shell.rs`（`--port 0` → 日志轮询 URL；无进展判 `Stalled`） | dsh 启动日志格式（打印访问地址） | 基线 |
| 2 | 宿主 dsh 版本闸 | `resolve.rs`（`version_at_least` / `engines.node` / 平台三重闸） | dsh `engines.node` 声明语义 | 基线 |
| 3 | pnpm global-bin-dir 注入 + npm 回退 | `updates.rs`（`pnpm_global_bin_dirs`，ADR-0005） | pnpm 10 全局目录解析（GUI 无 rc） | 基线 |
| 4 | WSL PATH 兼容探测 | `executor.rs`（`bash -lic` + nvm/fnm/n/volta 兜底扫描） | nvm/fnm 非交互 rc 守卫行为 | 2026-08-26 |
| 5 | WSL 客体内 dsh 自动安装 | `executor.rs`（ADR-0004：壳不触网，装进发行版） | dsh npm 包名与 `npm i -g` 语义 | 基线 |
| 6 | profile 列举 / 详情（文件系统模拟） | `profiles.rs`（扫描 `profiles/*/package.json`；详情 = 清单关键字段 + `cordis.patch.yml` 原文不解析） | profile 目录布局与三件套格式（`initProfile` @ 353）、内置模板名与 bundle（`PROFILE_TEMPLATES` @ 323） | 2026-08-28 |
| 7 | 创建 profile 半官方引导 | `profiles.rs`（spawn `dsh plugin --profile <名> install` + 结果分类；**2026-08-28 两次修订**：① add @deepseek-ai/dsh-base → install（零网络毫秒级，实测 `Already up to date`）；② install 成功后壳对非模板名追加 `@deepseek-ai/dsh-web-app` 单键声明（`declare_webui_bundle`，幂等）——创建即 webUi 候选可设为默认启动，与出厂 web 模板同构零下载，后 `--dump-config` 可正常组合启动） | `runPlugin` init-if-needed + pnpm 转发 + reconcile（`lib/plugin-9h8shc4d.js` @ 101；initProfile 三件套 @ 353）——注意：**add 裸包名会按 dist-tag `latest` 解析**（dsh-base 的 latest 停在已弃用 0.0.1-rc.1，当前版走 `next` tag 0.1.1-rc.2；0.0.1-rc.1 依赖 37+ 个已从 registry 删除的旧包名 → 404 + pnpm 递增重试 → 数分钟失败/超时），`install` 不解析任何 dist-tag，版本语义免疫；**bundles 追加的 dsh 侧依据**：`normalizeShippedProfile`（app-boot index.js @ 472，2026-08-28 读）——模板精确元组之外的 bundles 列表 = user-owned，且 `reconcilePlugins` 对 in-box bundle 零动作（never touched），web-app 由 `resolveBundleDir` 双锚点从 dsh 安装目录解析；**reconcile 会把声明 `dsh.bundle` 的新装依赖追加进 bundles**（plugin-9h8shc4d.js @ 46-75「joins the layer stack」，卸载对称移除）——2026-08-29 实测 web 档 add 3 插件后 bundles = base + web-app + 3 插件，同一包名 bundles/dependencies 双现属 dsh 数据模型本然 | 2026-08-29 |
| 8 | profile 非法名校验 | `profiles.rs`（`validate_profile_name`，详情/后续创建重命名共用的路径遍历防线） | `resolveProfileDir` 校验规则（空名 / `/` `\` / `.` / `..` / 字面量 `node_modules`，@ 318；拒绝集之外一律合法） | 2026-08-28 |
| 9 | 复制/重命名的 `name` 一致化改写 | `profiles.rs`（`rewrite_manifest_name`；红线 3 允许的三件套写入） | `initProfile` 写 `name: dsh-profile-<basename>`（@ 353）；该前缀无外部消费处（Spike B §2.2），改写为一致性保持 | 2026-08-28 |
| 10 | profile 切换（webUi 重启语义）+ WSL guest 脚本参数化 | `lib.rs`（`switch_profile` / `forced_profile` 注入 probe）、`executor.rs`（`guest_boot_script(profile)` + `sh_quote` 单引号进参，ADR-0009 §4 第三次修订） | dsh CLI 旗标面：launcher 只认 `--profile`/`--patch`/config dumps，其余原样转发给 app 树（dsh-cmdline @ 4-9）；web 命令族自带 `--host`/`--port`/`--no-open`/`--trusted-host`（dsh-web-app startup.js @ 16-44），**`--port 0` = OS 选空闲端口（help 明文）**；bundles 进程启动时挂载，无运行时切换/热加载能力 | 2026-08-29 |
| 11 | 插件运行时清单（4.4 前置，Spike B） | 壳侧回环调用（实施时落位；仅会话在跑时 `POST http://127.0.0.1:<port>/api/pluginInventory/list`，信封 `{type:"client-request",rpcId,method,payload:{args:{}}}`，见 `docs/spikes/0002-plugin-inventory.md`） | unary 调用兼走普通 HTTP POST（`dsh-client-connection` callUnary @ 6203-6211：`postJson("/api/${method}")`）；`payload` 恰一 plain-object `args` 字段；响应 `{entries:[{entryId,moduleName,enabled,fiberPhase}]}`（`dsh-host-plugin-inventory/typert.host.js` schema；FiberState→phase 映射 index.js @ 33-46，disposed→null）；回环无鉴权门（伪造 Host 仍 200，2026-08-29 实测）**← 2026-09-15 实测更正：此条已失效，见下**；dsh 0.1.6-alpha.1 的 `/api` 实为**两道**栅栏——`isTrustedApiRequest`（Host 回环/可信 + 无跨站标记，失败 **403**）之后还有 `browserAuth.isAuthenticated`（失败 **401**，`rpc-host.ts:97-99`），要的是启动期由 launch token 兑换出的**签名 Cookie**（`browser-auth.ts:285-300`：authority 绑定、`Max-Age=2592000`、`HttpOnly`、`SameSite=Strict`），**裸回环请求恒 401**；壳侧 `fetch_runtime_snapshot` 自 2026-08-29 起即因此静默失效，2026-09-15 随 ADR-0021 一并修复（`ShellState.workbench_cookie` 内存留存 + 回环附 `Cookie` 头）；patch/配置行 id ≠ entryId（无组前缀 vs `include:*` 树路径），patch 写入 id 以 `--dump-config` 行 id 为准 | 2026-09-15（安全栅栏口径更正于 2026-09-15；其余 2026-08-29） |
| 12 | pnpm 12 构建脚本审批门（4.4② 插件操作失败的主新因） | ~~`build_approvals.rs`（`parse_ignored_builds` 解析被点名包 + `set_profile_build_approvals` 受控改写 allowBuilds，写入例外 #5 / ADR-0009 第六次修订）~~ **2026-09-09 退役（ADR-0013）**：改默认批准——`build_policy.rs` 幂等写 profile 顶层键 `dangerouslyAllowAllBuilds: true`（pnpm 12.3.1 实测两种门槛形态均被压过，含 `allowBuilds` 显式 false）；解析/裁决链删除 | pnpm ≥12（本机 12.3.1 实测 2026-09-07）：`pnpm add` 装完全部包后，依赖树存在未获批安装脚本的包 → 追加 `allowBuilds: {包名: "set this to true or false"}` 模板进项目 `pnpm-workspace.yaml`（占位串在 pnpm 二进制内，非 dsh 生成）+ `ERR_PNPM_IGNORED_BUILDS` **退出 1**；`allowBuilds.<pkg>: false` = 显式忽略→退出 0；`true` = 真跑脚本（本机无 node-gyp 时 cpu-features 类包 127 失败——true 并非总可行）。dsh `runPlugin`（`@deepseek-ai/dsh` 0.1.2-rc.1 `lib/plugin-F7ZVfRyo.js`）= 裸 `spawnSync("pnpm", args, cwd=profile目录)` 透传退出码，非 0 跳过 reconcile → 半安装态（包已落盘/manifest 已写/bundles 未更）；其错误提示明示人工出路 = 编辑 pnpm-workspace.yaml allowBuilds 后重跑。dsh 全局安装目录的 allowBuilds 由 dsh 侧自填 true（核心原生依赖自带 prebuilds），**插件依赖树的包 dsh 不预批**——每次装/更新插件都可能撞门。**升级复核项**：pnpm 升级后占位模板/错误码措辞变化、dsh 升级后 runPlugin 是否自带审批处理；**2026-09-09 新增**：`dangerouslyAllowAllBuilds` 键名/语义是否仍在（ADR-0013 复审条件） | 2026-09-07 |
| 13 | 补丁包开关：段落归属 + 浅覆盖禁用 + 组禁用继承（4.4③ 补丁包，ADR-0009 第七次修订） | `plugins.rs`（`parse_dump_rows_with_section` 段落归属 + `build_row_states` 合成条目；`set_plugin_disabled` 写面不变） | dsh 0.1.2-rc.1 实测 + 源码锚定：① `dsh --profile <名> --dump-config` 为每个 bundle 输出顶格段落注释 `# == <bundle 包名>`（机器生成，实测 web 档：`# == @openviking/dsh-memory-plugin` 段内 = 它 insert 的行 `openviking-memory` 组；`# == dsh-better-sidebar` 段内 = 自身行）——段内行 = 该 bundle 声明/插入的行，补丁包（`dsh.bundle.patch` 纯 insert、自身不成行）由此可映射「包 → 贡献行」；**段头变体**：段落有 profile patch 命中时追加 `, patched by <路径>` 后缀（`# == @tt-a1i/archify-dsh, patched by /…/cordis.patch.yml`，2026-09-08 注入实机验证）——解析归属名须取逗号前；② include 插件 patch = 浅覆盖：`applyEntryPatches`（`cordis-plugin-include@1.0.7` lib/index.js）`{id, insert, name, ...overrides}` 对匹配行逐键写 overrides、`disabled` 属 overrides——profile patch 写 `- id: <贡献行>\n  disabled: true` 只置禁用位，原行 name/config 不丢（`name` 键只在 patch 也带 name 时才校验，写入不带即无匹配门）；③ loader 禁用继承：`cordis-plugin-loader@1.0.3` lib/index.js `Group.disabled`（@ 359-368）= 自身或**任意父级**条目禁用即禁用，`disabledOf` 支持 `!!js`（@ 377-379）——对组贡献行（`openviking-memory`）写一条 disabled 即组内全部子行禁用。**升级复核项**：dump 段落注释格式可能变（行表配对解析同通道脆弱点）、applyEntryPatches 签名/浅覆盖语义、loader 父级继承实现 | 2026-09-08 |
| 14 | 会话写所有权 = 跨进程内核租约（无过期） | 壳侧暂无消费点（P0 孤儿回收 / P0′ 持锁者诊断的设计依据；当前只为排查口径） | dsh 0.1.5-rc.1 实测 + 源码锚定（2026-09-10）：每个会话目录 `session.lock` 一把**内核**写锁——POSIX 非阻塞 `flock(2)`（锁的是 inode；加锁后校验 inode 未漂移，unlink+重建即失守），Windows 用具名内核信号量（**无锁文件**）；**刻意无过期**——「活着的持锁者一直持有，直到其进程退出；绝不设过期去剥夺一个卡住的写者」。争用 → `SessionAlreadyOwnedError`（`… is already owned by an active write handle`）；写打开时取锁，进程死亡由内核释放。实测：21 个 `session.lock` 逐个 `flock(LOCK_EX\|LOCK_NB)`，报 LOCKED 者与 `lsof` 持有者完全一致；对持有者发 SIGTERM 后锁即刻可获取（无残留）。**读者不碰锁**（读/搜索/删目录不受影响）。错误呈现层与锁无关：gateway 把任意 resume 异常包成 `RemoteError('gateway/internal', 'resume failed for session …')`（`dsh-api-session-controller/lib/index.js:238`）——**`gateway/internal` 是包装层，不是持有者**。**升级复核项**：`LEASE_FILENAME`、无过期语义是否仍在；错误类名与 gateway 包装文案是否变化 | 2026-09-10 |
| 15 | 壳侧子进程守卫所依赖的 OS 行为（fd 继承 / flock 语义 / 硬杀连坐） | `lifecycle.rs`（登记锁 fd 继承 + 生命线 watcher + 启动期清扫；ADR-0015 §7/§7.1 实测依据） | 2026-09-10 实机实测（macOS，node = 引擎 v24.18.0）：① **node 全程保留**继承来的 fd（父 `flock` 后经 `pass_fds` 传入，`lsof` 见 `3u REG … .lock`）——登记锁判据成立；② 该 fd **不流入孙进程**（node 内 `spawnSync('sleep')`，孙进程 `lsof` 无该 fd）——登记锁与「壳的直接子进程」严格 1:1，不因孙进程存活而误判；③ 父被 `SIGKILL` 后子进程仍持锁（`flock(LOCK_EX\|LOCK_NB)` 报 LOCKED）——孤儿可被探测；④ 子进程死亡后锁由**内核**自动释放（复测 UNLOCKED）。**flock 关键语义**：锁属**打开文件描述**，`fork`/`dup` 共享同一描述 → 显式 `LOCK_UN` 会**连同子进程的持有一起撤销**（实测：父 `LOCK_UN` 后探测立即 UNLOCKED），而仅 `close()` 父 fd 则子进程仍持有（探测 LOCKED）。故守卫**只 close 绝不 unlock**（`Registration::drop` 注释 + `sweep_reaps_live_orphan_*` 锚定）。**升级复核项**：node 未来版本是否仍保留继承 fd（若不保留，清扫判据退化为 pid + 启动时间双校验，见契约 §2.1） | 2026-09-10 |
| 16 | 取消归档的 typert 远程契约（方法串 / wire 键规则 / 响应形状 / 幂等性） | `sessions.rs::request_unarchive`（`unarchive_request_body` + `parse_unarchive_response` 纯函数；条目级网络豁免）+ `commands/session.rs::unarchive_session`（ADR-0021 **路线 A**；前端 `SessionManager` 归档档位） | dsh 0.1.6-alpha.1 源码锚定（2026-09-15）：① `@Remote('unarchiveSession')` 注册在 **namespace `workspace`**（`@deepseek-ai/dsh-api-workspace-controller` `src/index.ts:35,43,118-121`），故**方法串 = `workspace/unarchiveSession`**、URL = `/api/<method>`（Connection 信封 `clientRequestSchema`：`{type:'client-request',rpcId,method,payload}`，`rpc-schema.ts:36-41`；`payload` 形状归端点自校验）；② `payload.args` 的键取**源码形参名**（`@deepseek-ai/dsh-typert-generator` `src/analyzer.ts:1137-1139` `wire: parameter.name.text`）→ 本端点必须**恰**为 `{request:{sessionId}}`；gateway 侧 `assertExactArguments`（`api-gateway` `src/index.ts:1107-1128`）对多键（如把请求对象展开成顶层 `{sessionId}`）与缺键**一律拒绝**（`gateway/arguments-invalid`）——即"少一层包装"会稳定失败，不会静默成功；③ 响应 `{result:{ok,value}}`，成功值 = `WorkspaceArchiveValue` = `{archivedSessionIds: string[]}`（`workspace-controller` `src/types.ts:113-115`）＝变更后**完整**归档集；④ **幂等**：未归档的 id 不报错（`src/commands.ts:163-174` "An id that is not archived is not an error"）；⑤ 回环**需带签名 Cookie**（2026-09-15 实测更正，见复现点 11 的更正说明）：无 Cookie 恒 401。**复核项**：namespace/动词名是否变、wire 键"由形参名派生"的规则是否变、`assertExactArguments` 严格性是否放宽、`WorkspaceArchiveValue` 字段名、该动词是否从 Remote 面移除（上游若移除，ADR-0021 的翻转条件触发）、**`/api` 鉴权栅栏的层数与 Cookie 契约** | 2026-09-15 |
| 17 | MCP 探测依赖的传输取值集合 / 线协议事实 / SDK 客户端行为 | `mcp_probe.rs`（两分支：`probe_stdio` = 子进程；`probe_http` = 条目级网络豁免）+ `mcp.rs`（`McpTransport` 配置模型；`commands/console.rs` 按 transport 分派） | dsh 0.1.6-alpha.1 源码锚定 + 本机安装的引擎（2026-09-15 复核）：① **传输取值集合恰两种**——`stdio` 与 `streamable-http`，**无 `sse` 字面量**（`packages/mcp/mcp-client/src/transport.ts:32-44` 的 `switch` 仅此两分支）；配置投影 `src/index.ts:133-136`：`transport: z.const('streamable-http')`、`url` 必填、`headers` 为 dict 且默认 `{}`；② 方法名：`initialize` / `tools/list` / `resources/list` / `resources/templates/list`（后两者 DSH 自身亦在用，`connection.ts:372,376`）；③ 协议版本：随引擎安装的 `@modelcontextprotocol/sdk@1.30.0`，`dist/esm/types.js:2` `LATEST_PROTOCOL_VERSION = '2025-11-25'`，`:4` supported = \[该版本, `2025-06-18`, `2025-03-26`, `2024-11-05`, `2024-10-07`\]；壳发最新版并**以服务端协商回来的版本为准**；④ **streamable-http 客户端行为**（SDK 锚定；壳侧自实现 `post_rpc` 与之一致）：请求头 `accept: application/json, text/event-stream`（`client/streamableHttp.js:299`）、按响应媒体类型 `text/event-stream` 分支解析（`:388`）、`initialize` 响应可下发 `mcp-session-id` 且后续请求须回带（`:309` 读取 / `:68` 发送）；⑤ 能力可选：`resources/list` / `resources/templates/list` 可回 `-32601`（method not found）→ 壳**降级为「空 + notes」而非整单失败**（否则用户把"没这能力"读成"连不上"）；⑥ 可行性前提：**DSH 无供 GUI 枚举 MCP 的 RPC**（`packages/api/` 全目录 `grep -ril mcp` 命中 **0** 文件）——壳要体检只能自己说协议（ADR-0022 §1.3）。**注**：④ 中 SSE 与 session-id 属 **MCP 协议/SDK 行为**，⑤ 的降级是**壳的**口径选择，两者不是同一类事实。**升级复核项**：transport 集合是否新增/移除（ADR-0022 §6 复审条件之一）、`LATEST_PROTOCOL_VERSION` 与 supported 列表、方法名与 `-32601` 语义、`mcp-session-id` 头名与 SSE 媒体类型分支、`/api` 是否新增 MCP RPC（若新增，ADR-0022 方案 A 的子进程与两处豁免全部失去必要性） | 2026-09-15 |
| 18 ❌已退役 | `~/.ssh/config` 解析所依赖的 **OpenSSH 行为**（壳侧解析器必须与真 ssh 同口径） | `ssh_config.rs::parse_ssh_config`（纯函数；`load_ssh_hosts` 是唯一 IO 点）+ `commands/ssh.rs::list_ssh_hosts`（ADR-0023 §2.6：解析固定在 Rust 后端） | **本机实测锚定**（2026-09-15，macOS 自带 OpenSSH；方法 = 写临时 config 后 `ssh -F <file> -G <alias>` 读回生效值）：① **`#` 只在行首是注释**——`Host a # prod comment` 下 `ssh -G '#'` 返回 `hostname #`，即 `#`/`prod`/`comment` 都被当成**主机模式**；壳因此**如实保留**这些模式，但另出 `notes` 解释（否则用户以为壳解析错了）。② 单值关键字**只取首个参数**：`HostName 1.2.3.4 # trailing` → `hostname 1.2.3.4`。③ **首个取值优先、且全局段优先**：文件开头的 `User globaluser` 胜过后面的 `Host *` 段。④ **通配段填充未设字段**：`Host a` 只设 `HostName`、`Host *` 设 `Port 2222` → `a` 拿到 `2222`。⑤ **取反排除**：`Host * !b` 的 `Port 2222` 对 `a` 生效、对 `b` 不生效（`b` 回落到默认 22）。⑥ `HostName` 未设时 ssh 以 **alias 本身**为默认值（壳侧返回 `null`，由 UI 兜底显示 alias）。⑦ `Include` 壳**不跟随**（这是壳的读取面纪律，不是 OpenSSH 限制）。**升级复核项**：OpenSSH 对行中 `#` 的处置是否变化（若有版本开始剥行尾注释，壳的 `notes` 会变成误报）、`Match` 段是否变得可从静态配置推导、新关键字是否影响默认值兜底 ｜ **已退役（2026-09-17）：SSH 远程工作区整体移除，未随任何版本发布** | 2026-09-15 |
| 19 ❌已退役 | SSH 远程工作区的**服务映射 / 五键必填与运行时校验 / 非交互与平台约束** | `ssh_profile.rs`（四行注册 + 钉版本安装 + 写后自证）+ `ssh_remote.rs`（五键校验 + `BatchMode` 预检）+ `profiles.rs`（app bundle 取 headless） | dsh 0.1.6-alpha.1 源码锚定（2026-09-15）：① **四包齐注册**，**无伞包/meta 包**（`packages/ssh/README.md:2` 的 `kind: package-group` 只是文档词汇）——`@deepseek-ai/dsh-ssh` / `@deepseek-ai/dsh-fs-ssh` / `@deepseek-ai/dsh-subprocess-ssh` / `@deepseek-ai/dsh-sandbox-ssh`（各自 `package.json` 的 `name`）；依赖边 `fs-ssh/src/index.ts:21` `inject = ['ssh','sandboxPolicy']`、`subprocess-ssh/src/index.ts:230` `inject = ['ssh']`、`sandbox-ssh/src/index.ts:11` `inject = ['ssh']` ⇒ **`dsh-ssh` 必须排最前**（顺序即语义）。② **只有 `dsh-ssh` 接受 `config`**（其余三个 README 明写无配置）；配置形状 `ssh/src/index.ts:17-40`（接口）、`:48-54`（schemastery，五个 `.required()`）、**运行时 zod 更严** `:76-83`：`host` 须匹配 `/^[a-zA-Z0-9][a-zA-Z0-9_.@-]*$/`、`node`/`helper`/`workspace` 须 `startsWith('/')`、`helperHash` 须 `/^[0-9a-f]{64}$/`（**只认小写**）；可选 `bootstrapPath`+`bootstrapHash` **必须成对**（`:83` 的 `refine`，原文 "must be paired"）；调参默认 `requestTimeoutMs=30000`（≤2147483647）、`maxFrameBytes=64MiB`（≤64MiB）、`maxPending=128`（≤128）、`leaseMs=30000`（3000–600000）。**不存在** `target` / `remoteNodePath` 选项。③ **平台限制是运行时硬错**：`:75` `if (process.platform !== 'linux' && process.platform !== 'darwin') throw new Error('SSH runtime requires a POSIX client')` ——**两端**须 Linux/macOS，故 Windows 宿主上本功能不可用（壳提前到向导第一步拒绝，非壳自造限制）。④ **非交互**（`packages/ssh/ssh/README.md:93` 与 `.agents/notes/implemented/architecture/2026-09-11-posix-ssh-runtime.md:45`）：服务启用 `BatchMode`、强制严格 host-key 校验、**禁用 agent forwarding**、不提供交互式认证；壳侧预检据此锁死 `-o BatchMode=yes -o ForwardAgent=no -o StrictHostKeyChecking=yes`。⑤ **范围限定**：`docs/subsystems/ssh.md`(Composition scope) 原文 "Web workspace views that assume host filesystem access need separate integration; replacing providers alone does **not** make those views remote-aware" + 同口径 "The initial composition scope is **POSIX headless and custom profiles**" ⇒ 壳生成 headless profile 而**不**声明 web-app（否则承诺一个上游不支持的形态）。⑥ **无上游范本**：六个 bundle patch、全部 agent preset、`apps/cli/config/examples/` 均无 ssh 行，唯一字面量在测试夹具与生成的 `docs/config-catalog.md` —— 壳是首个落地者。**升级复核项**：四包名/数量或服务映射变化（ADR-0023 §6）、五键增减或校验放宽、`helperHash` 是否仍只认小写、`bootstrap` 配对规则、POSIX 限制是否解除、上游是否出现 shipped ssh 预设（出现即应以之为对照基准）、Web 视图是否变为远端感知（若变，ADR-0023 的范围可放宽到 web profile） ｜ **已退役（2026-09-17）：SSH 远程工作区整体移除，未随任何版本发布** | 2026-09-15 |

| 20 | 实验能力的**开启 / 关闭机制**与「关而不卸」的边界（策展四能力的开关语义） | `official_catalog.rs`（能力→变体→状态）、`plugins.rs::{package_declares_bundle, remove_catalog_insert_row}`、`commands/plugin.rs::{list_experimental_capabilities, apply_official_patch_row, remove_official_patch_row}` | dsh 0.1.6-alpha.1 @ `0d1f5000`（2026-09-15）源码锚定＋实查：① **上游无运行时 feature flag**——`DSH_EXPERIMENTAL` / `--experimental` 只存在于已归档 note（`.agents/notes/archived/feature/2026-07-31-experimental-subcommand-gate.md:3-4`，`Archived: 2026-08-03`），`.agents/notes/AGENTS.md:7` 明令归档件不得当作现行权威，当前树全仓 grep 0 命中；「experimental」是**发布隔离分类**（`packages/experimental/AGENTS.md:6`；`scripts/experimental-package-policy.ts:2` denylist 为空数组 ⇒ 该组 16 包全部公开）——故「功能开关」只能由壳在 **profile 层 + Cordis 行**这一层实现。② **层叠顺序是「行级停用能盖住层引入的行」的前提**：各 bundle patch → profile 自身 `cordis.patch.yml` → home 级 → `--patch`（`docs/user/develop/basic/publish.md:114-119`）。③ **「关而不卸」= 行级 `disabled`**：`vendor/loader/src/config/entry.ts:18-19` "Prevents this entry and descendants from running."（父级继承 `:72-82`，生效路径 `:133-136`）；文档口径 `docs/cordis-tutorial/06-composition-and-hmr.zh.md:16,19`「**卸载插件但不删除其 Cordis 配置项**，改回原值后再次加载」；真实先例 = CLI 自用 `apps/cli/src/profile-boot.ts:171-173`（`DSH_TELEMETRY_DISABLED` → `{id, disabled:true}`）。④ **层类能力不能被行级开关关干净**：`packages/experimental/agent-team-profile/cordis.patch.yml:4-14` 除插入 2 行外**还停用 4 条旧 subagent 控件行**——只禁用壳插入的行会变成「新旧都没有」；干净出路是 `dsh plugin remove`（**同时把该包从有序层列表 `dsh.profile.bundles` 移除**，`agent-team-profile/README.zh.md:37`、`agent-team-web-profile/README.zh.md:37`）。壳因此把 `toggle_off_supported` 定义为「每一步都是壳写的行」。⑤ **分类判据只有一条**：读目标包自身 `package.json` 的 `/dsh/bundle/patch`。该组内声明者为 auto-review / agent-team-profile / agent-team-web-profile；五 provider 与 agent-team、tool-agent-team 无 `dsh` 字段（须写行）；两个基座包 `@deepseek-ai/dsh-browser-use`、`@deepseek-ai/dsh-computer-use` **也没有 `dsh` 字段**（`packages/browser-use/browser-use/package.json`、`packages/computer-use/computer-use/package.json`）⇒ 浏览器/桌面两族的全部包都是「壳写的行」，可整族秒级开关。**推论（已修）**：分类只能在包已装之后读出，「安装前判定」= 把任何包都判成需写行 ⇒ 给 profile 层多写一条 = **重复挂载**（ADR-0020 §7.1 D7）。⑥ **族内互斥是服务侧硬约束**：`docs/subsystems/browser-use.zh.md:17`、`docs/subsystems/computer-use.zh.md:16`「共享服务只注册名称，并**拒绝任何第二次提供方注册，包括同名实例**」。⑦ 上游插件清单 GUI **只读**（`packages/client/ui-settings-plugin-inventory/src/client/PluginInventorySettingsTab.tsx:20-27,184`的注入面只有 `list()`，`EnablementKind` 无写接口）⇒ 写路径只能由壳自建。**升级复核项**：`dsh.bundle` 分类是否变化；层 patch 是否新增副作用（又去停用别的旧行）；`disabled` 与父级继承语义；`dsh plugin remove` 是否仍同步清理 `dsh.profile.bundles`；两个基座包是否获得 `dsh` 字段（一旦获得，「两族可纯行级开关」的结论立即失效）；归档的 `--experimental` 门控是否有等价物复活 | 2026-09-16 |
| 21 | profile 补丁层的**生效时机**（`patchReload: live` 热载）与 `pluginInventory/list` 的字段语义 | `commands/plugin.rs::get_plugin_runtime`（回环取快照）、`frontend/src/lib/profiles.ts::{runtimeChipFor, runtimeToggleApplied}`、`components/profiles/ProfileDetailPane.tsx`（开关后短轮询落定） | dsh 0.1.6-alpha.1 @ `0d1f5000`（2026-09-17 源锚 + 克隆实机实测）：① `dsh.profile.patchReload ∈ {live, startup}`（`packages/util/package-manifest/src/types.ts:66`）；出厂模板 `web` = **live**、`headless`/`acp`/`sdk`/`sdk-minimal` = `startup`、无模板的自定义 profile 默认 **live**（`packages/boot/app-boot/src/profile.ts:139-171`；`normalizeShippedProfile` 会把缺省值回写进 package.json）；② `live` 时启动器装 chokidar 盯 `cordis.patch.yml`（`apps/cli/src/profile-boot.ts:375-402` → `packages/boot/app-boot/src/watch-config.ts:44-70`），改动经 `watchUserPatches` → Include `internal/update` 就地重放补丁，`disabled` 行 `dispose()`、恢复时 `init()`（`vendor/include/src/index.ts:190-201`、`vendor/loader/src/config/entry.ts:108-148`）——**不是只在启动时读**；③ 克隆实机实测（`~/.dsh-dock-dev` 副本 + 进程自持端口回环查询，未动现场）：给"已装且 active"的行追加 `- id: <row>` + `disabled: true` → **0.43s** 内 `enabled=false, fiberPhase=null`；删掉该条 → **0.44s** 内回到 `enabled=true, fiberPhase=active`；④ 清单字段（`packages/host/plugin-inventory/src/index.ts:65-89`）：`enabled = !entry.disabled`（**配置级生效值，不等于有存活 fiber**），`fiberPhase` 由 FiberState 投影（disposed 与"无 fiber"都是 `null`）⇒ 只有 `enabled` 能区分"被禁用"与"启用了但没装好"；⑤ `entryId` = loader 树路径（`include:<row-id>`，可带组前缀），**不等于** patch 行 id（壳侧匹配只用 `moduleName`）；⑥ 依赖只是解析面：既不在 `dsh.profile.bundles`、也没被 `insert` 行挂载的包不会加载（`apps/cli/src/plugin.ts:48-95`）；本机 `chrome-devtools-mcp` 即"有 insert 行但不在 dependencies"⇒ 导入失败（`enabled=true, fiberPhase=null`，是"没有实例"而不是"已停用"） | 2026-09-17 |
| 22 | **dsh 安装包自带的官方实验层**（optional bundle：谁自带、怎么开关、能不能卸） | `official_catalog.rs::installation_shipped`（对 `<engines>/dsh-runtime/node_modules/` 的**实测**探测）＋ `CapabilityView.{shippedByDsh, legacyCopy}`；前端 `ExperimentalCapabilities.tsx` 的 `ShippedPane` | dsh **0.1.6-alpha.2** @ 上游 `ddfc45fb`（2026-09-17 源锚 + 本机正式档引擎实测）：① **自带清单 = 上游 `OPTIONAL_BUNDLES`**（`packages/boot/app-boot/src/profile.ts`）：`@deepseek-ai/dsh-experimental-agent-team-profile`、`…-web-profile`；判据是"**是 `apps/cli` 的运行时依赖 + 声明 `dsh.bundle.patch` + 没有任何出厂模板选中它**"，由 `scripts/` 的静态门与打包验收门共同保证（设计笔记 `.agents/notes/implemented/process/2026-09-15-shipped-optional-bundles.md`）；② **语义**：随安装包下载、**默认关**、在 dsh 自己的插件页开关、`optional: true` 且 **`removable: false`**（`packages/boot/plugin-manager/src/index.ts:203` 的 `OPTIONAL_BUNDLES.includes(name)`；`manager-store.ts:73` 注释原文 "official, off until selected, never removable"）；③ **该笔记明确否决**"由某个面板按名字从 registry 安装官方 bundle"这条替代路径（理由：开关那一刻要联网 + 每次发布要钉一个版本）——**本模块原来的做法正是它**；④ 真机实测：正式档引擎 `dsh 0.1.6-alpha.2` 的 `node_modules/@deepseek-ai/` 下确有这两个包（`0.1.6-alpha.2`），且用户 `~/.dsh/profiles/web` 的 `dsh.profile.bundles` 含它们、而 **profile 依赖里没有**（＝ dsh 提供的层）；dev 档引擎 `0.1.6-alpha.1` 两者皆无；⑤ `auto-review` **不在**自带清单（同笔记原文："a published experimental package the page's install guide names as its example, not an optional bundle"）；`browser-use` / `computer-use` 亦未自带（仍是"要装才有"的 provider 包）。**升级复核项**：`@deepseek-ai/dsh` 的 `dependencies` 里是否出现新的 `@deepseek-ai/dsh-experimental-*`（等价于 `OPTIONAL_BUNDLES` 增项）→ 出现即出现在"自带"集合里（**无需改代码**：判据是安装实测），但要复核本模块的文案与边界仍成立；`optional`/`removable` 语义与 `not-removable` 拒绝码是否仍在；自带层是否开始支持"随包一起装 companion 层"（笔记明写当前**不**自动选 companion） | 2026-09-17 |

## 二、计划复现点（4.3 Profile 管理器落地时入册）

（空——复现点 6/7/8 均已落地，见第一节；后续新增复现点在此登记后随实现转一。）

## 三、复核记录（append-only）

- 2026-09-17 新增复现点 22（dsh 自带官方实验层）：维护者发现「dsh 新版本已经把智能体团队插件
  内置了」，据此确立「dsh 自带的、dock 不代管」（ADR-0020 §7.2 第三次修订）。自带清单**随 dsh
  版本而变**（同一台机器上：0.1.6-alpha.2 有、0.1.6-alpha.1 无），故本条与复核项一起入册。
- 2026-08-28 建册：基线 v0.1.1-rc.2，已落地 5 项、计划 3 项，全量登记。
- 2026-08-28 4.3 只读刀：复现点 6/8 自「计划」转入「已落地」（壳侧 `profiles.rs`；
  行号按当日勘误口径 318/323/353，早期文档的 11826/13418 系 bundle 行号混入作废）。
- 2026-08-28 4.3 创建刀：复现点 7 转入「已落地」（spawn 转发链 + 结果分类）；
  实机验证含 pnpm 网络失败模式（镜像 ECONNRESET -> 已创建未装中间态，exit 1），
  成功路径 reconcile 沿用 Spike A §3.2 同机同版本结论。
- 2026-08-28 4.3 生命周期刀：复现点 9 入册（复制排除 node_modules / 重命名删
  node_modules 让 dsh 自愈 / sessions 不级联，引用面全按 Spike B §3 执行）。
- 2026-08-28 创建路径修订：复现点 7 改 `add @deepseek-ai/dsh-base` → `install`。
  触发：2026-08-28 本机实测创建 `test` profile 慢至 2 分钟失败/超时，
  查因 = `add` 裸包名按 dist-tag `latest` 解析到 dsh-base 0.0.1-rc.1（已弃用），
  其依赖 37+ 个旧包名（dsh-bash-env / dsh-tasks-local / dsh-skill-local…）已从
  registry 删除（npmmirror/npmjs 均 404）→ pnpm 递增重试（10s/60s × 37 包）
  → 「2 分钟失败或拖满 600s 超时」；同时 npmmirror 对缺失 scoped 包回退到死
  域名 r.cnpmjs.org（本机解析到保留段 198.18.0.192）放大了表象。Spike A §3.2
  当时「零网络全命中 store」结论掩盖了裸名版本语义的漂移风险（复现了字符串、
  没复现版本语义）——已收编为本条「依赖的 dsh 行为」栏的显式复核项。
- 2026-08-28 创建路径二次修订：install 成功后壳对非模板名追加 web-app 声明。
  触发：用户设默认 profile「11」（纯 dsh-base 原始版）重启仍启动 web——
  defaultProfile 消费只认 webUi 候选（bundles 含 web-app），无 webUi 的 profile
  无 URL 可导航属设计内回退。追加的 dsh 侧依据 = `normalizeShippedProfile`
  「Any other list is user-owned」（index.js @ 472）+ reconcilePlugins 对
  in-box bundle 零动作；目标状态与出厂 web 模板同构（该形态即 web profile
  日常运行态）。红线走 ADR-0009 §4 第二次修订（写入例外 #2），AGENTS §6
  不变量行同步扩展，详见当日广播。
- 2026-08-29 4.3⑥ 切换刀：复现点 10 入册（CLI 旗标面 + `--port 0` 官方语义 +
  无运行时切换能力——切换 = 重启的依据）。WSL guest 启动脚本由写死
  `--profile web` 参数化为 `guest_boot_script(profile)`（`sh_quote` 单引号字面量，
  反例测试 + bash 实跑回读）；WSL 真机验证待 Windows 侧按 docs/executor.md
  清单人工执行，切换其余路径本机已实装验证。多开（多实例并行）依赖同一
  旗标面，storages 竞态未验证，登记 roadmap 待办。
- 2026-08-29 4.4 前置 Spike B：复现点 11 入册（插件运行时清单 = 回环 HTTP
  POST 单调用，`payload:{args:{}}` 信封实机打通；WS mux 并非唯一传输）。
  关键意外：回环无鉴权门（伪造 Host 仍 200）——记为 dsh 既有姿态，壳侧
  只读使用；id 空间分叉（patch 行 id vs entryId）以 dump-config 为 patch
  写入源。详见 docs/spikes/0002-plugin-inventory.md。
- 2026-08-30 H-1 口径边界实测：**排序最高版本可能不可安装**。`@deepseek-ai/dsh`
  0.1.2-alpha.2（当前 sort-max，挂 dist-tag `alpha`；`latest` = 0.1.1-rc.2）依赖
  `@deepseek-ai/dsh-util-time` / `dsh-util-workspace-path` 等未发布子包——npmmirror
  缺失回退死域名 r.cnpmjs.org（复现点 7 同款放大器）ECONNRESET，npmjs 亦不可达
  依赖 → `pnpm add -g` 两 registry 均失败（app 内复现：shell.log 14:41-14:49 多次
  尝试全败；手工复现同错）。H-1「rc 也追，不认 dist-tag」口径会向用户提示一个
  装不上的版本——已知边界：失败现经 `dsh:upgrade` 事件对用户可见（含 pnpm 输出
  尾部）；根治需检查侧可安装性预校验（每依赖一查，成本高）或口径改 dist-tag 优先
  （裁定事项，未动）。
- 2026-09-07 复现点 12 入册：v0.9.5 引擎档（pnpm 12.3.1）上线当日用户装
  dsh-web-all 报「dsh 退出码 1」——实机取证链条：plugin-op.log 全文（added 240
  done 后 ERR_PNPM_IGNORED_BUILDS）→ dsh 转发源码（runPlugin 裸 spawnSync pnpm）
  → pnpm 二进制 strings（占位模板/allowBuilds 语义）→ /tmp 双向复现（无裁决
  必败 exit 1 + 写模板；false 退出 0；true 无工具链 127）。修复 = 壳内裁决流
  （ADR-0009 第六次修订/写入例外 #5 + IPC set_profile_build_approvals），用户
  实机 true/false 取值留给对话框逐包裁决。
- 2026-09-08 复现点 13 入册：用户问「为何两个插件没有开关」（补丁包形态误判
  为未安装/解析失败）——实机取证：dump-config 段落注释 `# == <bundle>` 归属
  实测 web 档两补丁包 + include/loader 源码锚定（浅覆盖 disabled 单键 + 组
  禁用继承）。修复 = 补丁包开关（ADR-0009 第七次修订：get_plugin_rows 合成
  条目 + 前端串行多目标写入）。升级复核项已入第 13 行。
- 2026-09-09 复现点 12 退役（ADR-0013，默认批准）：当日实测失效链条——装
  `dsh-ssh` 却弹出 `dsh-pet` 的旧 exact key。根因 = 追加式日志（commit 9d341f8）
  让 `ForwardRun.output` 变成历史全量，解析器先撞上旧 `ERR_PNPM_GIT_DEP_PREPARE_NOT_ALLOWED`
  标记 → 返回历史键；批准对真门槛（`cpu-features`/`ssh2`）无效 → 重试死循环
  （plugin-op.log 04:31/04:32/04:33 三次同型）。/tmp 实测 `dangerouslyAllowAllBuilds: true`
  压过两种门槛形态（npm 依赖 + git `prepare`，且 `allowBuilds` 显式 false 也被压过、
  `prepare` 确实执行）。壳侧改为幂等写该顶层键，解析/裁决链删除；pnpm 两种门槛
  形态的事实描述仍保留在第 12 行（升级复核项追加键名存续性）。
- 2026-09-10 复现点 14 入册（会话写租约语义与争用）：触发 = 用户报「会话点不开，
  弹 `SessionAlreadyOwnedError … (gateway/internal)`，删会话 + 重启才恢复，且最近
  经常出现」。实机取证链条：报错文案定位到 gateway 包装层（
  `dsh-api-session-controller/lib/index.js:238`，`gateway/internal` 非持有者）→
  锁语义源码锚定（`dsh-session-persistence-jsonl/lib/types/lease.d.ts`：POSIX
  `flock(session.lock)` / Windows 具名信号量，**刻意无过期**）→ 21 个
  `session.lock` 逐个 `flock(LOCK_EX|LOCK_NB)` 探测，LOCKED 集合与 `lsof` 持有者
  完全互证 → 持有者锁定为 `PPID=1` 的 `.dev` 逃逸 dsh（`pid 85156`，持的正是报错
  会话）。根因 = 壳侧子进程逃逸的**历史存量**（会话槽覆盖，已由 ADR-0014 /
  `0cd40c2` 2026-09-10 17:05 修复；11 个孤儿全部创建于该修复构建之前）+ **硬杀
  绕过**（SIGKILL 不走 `RunEvent::Exit`，teardown 无从执行——残余风险待 ADR）+ 无
  任何逃逸进程回收机制 + dev/prod 共用 `DSH_HOME` 放大到正式包。处置 = 11 个孤儿
  SIGTERM 回收（全部正常退出），复测仅剩活着的正式包 dsh 持有的两把锁；未触碰
  正式包进程。诊断全文见 `docs/known-issues/问题记录-2026-09-10-会话写锁被孤儿dsh占死.md`
  （含 P0/P0′/P1/P2 修复建议，未实施）。**升级复核项**已入第 14 行。
- 2026-09-10 复现点 15 入册（守卫所依赖的 OS 行为）：触发 = ADR-0015 方案 A 实施。
  该 ADR 把「node 是否全程保留继承 fd」列为**实施前置**（不成立则清扫判据退化），
  刀 0 实测通过并额外发现**fd 不流入孙进程**（使登记锁与直接子进程 1:1）。实施中
  又踩到两条 flock/fd 语义坑并已固化进代码与闸门：① 显式 `LOCK_UN` 会撤销**整个
  打开文件描述**的锁（含子进程的持有）→ 守卫改为只 close；② `pre_exec` + 固定 fd 号
  的生命线通道在实机上让 `read` 立即失败，watcher **误杀**正常子进程（真 dsh 一启动
  即吃 SIGTERM）→ 改用 `Stdio` 通道且失败方向改为"宁可不收口也不误杀"（详见
  ADR-0015 §7.1）。
- 2026-09-15 复现点 16 入册（取消归档的 typert 远程契约）：触发 = ADR-0021 **路线 A**
  实施（维护者裁定「认可」，ADR 已接受）。入册理由与复现点 11 同源——**回环 RPC 的
  契约是壳对 dsh 内部形状的硬依赖，升级可静默漂移**：方法串、`payload.args` 的键名、
  严格参数校验、响应值字段、幂等语义，任一处变化都会让"取消归档"从能用变成稳定失败。
  故把契约连同上游 `file:line` 锚点一起钉在册，并写入 §一 的复核项；壳侧同时以
  `unarchive_request_body_carries_exact_wire_keys`（键集恰为 `["request"]`）与
  `parse_unarchive_response_*`（正反例）把该契约固化为单测，漂移时先红在壳内。
  **实现取舍留痕**：ADR-0021 行动项原写"world 分派"，实施时**有意简化为不分派**——
  路线 A 不碰任何文件路径，归档状态由**当前活跃 Host** 持有，其地址在 Local 与 WSL
  两种模式下都记在 `ShellState.workbench_url`（`boot.rs:384`），故没有宿主/客体之分；
  无活跃 Host 时如实报错而非降级改文件。该偏差已回报维护者。
- 2026-09-15 **更正复现点 11 的一处失效断言（本轮最重要的账目修正）**：原记
  "回环无鉴权门（伪造 Host 仍 200，2026-08-29 实测）"**已不成立**。触发 = I2 落地后
  按 §5/§8.3 做**实机对账**（而非只看单测绿）：先对真实运行中的工作台
  （`127.0.0.1:53805`）打既有回环端点，得 **`401 unauthorized`**；再读上游
  `rpc-host.ts:97-99` 确认 `/api` 有**两道**栅栏——信任栅栏（Host 回环/可信，失败
  **403**）之后是 `browserAuth.isAuthenticated`（失败 **401**），后者要**签名 Cookie**
  （`browser-auth.ts:285-300`）。在抛离式 `DSH_HOME` 里完整复现了正确姿态并附实据：
  `GET /?token=…` → `303` + `set-cookie: dsh-auth-…`（`Max-Age=2592000`、`HttpOnly`、
  `SameSite=Strict`）→ 带该 Cookie 再打 `/api/pluginInventory/list` → **`200`**；
  打 `/api/workspace/unarchiveSession`（不存在的 id，上游保证为**无写早退**）
  → **`200` + `{"result":{"ok":true,"value":{"archivedSessionIds":[]}}}`**。
  **两点后果**：① 本轮 I2 的**线协议本身据此实机确认无误**（方法串、`args.request`
  键、响应字段全对）；② 但**壳侧既有功能 `fetch_runtime_snapshot`（2026-08-29 落地）
  自落地起即对带栅栏版本恒 401 而静默失效**——这是"记在册的姿态随上游漂移"的教科书
  案例，正是本册存在的理由。处置：`ShellState` 新增内存态 `workbench_cookie`
  （boot 兑换时留存，不落盘不打日志），`fetch_runtime_snapshot` 与 `request_unarchive`
  一并附 `Cookie` 头。**教训**：跨版本断言必须带"实测版本 + 复测日期"，且**回环功能
  上线前须打真实 Host**——单测与闸门全绿也不足以证明它没在线上 401。

- 2026-09-15 复现点 17 入册（ADR-0022 行动项补登，随 §7 R3 实施）：**原缺登**——
  ADR-0022 §5 明确要求"新增复现点行：MCP transport 取值集合、`*/list` 方法名、
  以及'DSH 无 MCP RPC'这一可行性前提"，但该 ADR 的 stdio 分支落地时只登记了网络面
  （`network_gate.rs` + 登记册 §二），**未回写本台账**——红线 1 的"锚定源码位置 +
  日期"要求因此有缺口。本轮实现 `streamable-http` 分支时补齐：全部 6 项事实**在本机
  重新验证**（不引用上一轮的结论），并补上 ④ SDK 客户端行为（`accept` 头、SSE 媒体
  类型分支、`mcp-session-id` 头名）——这三条在自实现 HTTP 客户端时必须逐字对齐，
  否则会出现"合规服务端被判成连不上"的假失败。

- 2026-09-15 复现点 18 入册（ADR-0023 R4a，随 `~/.ssh/config` 解析器落地）：本行锚的是
  **OpenSSH** 而非 dsh 的行为（同复现点 15 锚 OS 行为的先例）——壳侧解析器是 ssh 配置的
  影子实现，**口径一旦与真 ssh 不同，给用户的就是错误的事实**（下拉里少一个主机、或
  少一个 User/Port）。七项全部以 `ssh -F <file> -G <alias>` 实测回读锚定，不靠文档推断：
  其中 ①（行中 `#` 不是注释）与 ⑥（`HostName` 缺省=alias）是**违反直觉**的两条，
  也是唯二靠"读文档会写错"的地方。**开发期实测已当场抓住一个真 bug**：初版把 `Match`
  段的关键字落进"全局段"进而泄漏给**所有**主机（`ssh` 不会这样）——该 bug 由单测
  `match_block_does_not_leak_into_previous_host` 拦下，说明"先写反例"的纪律有效。
  **注**：ADR-0023 行动项要求的另一行（**四包服务映射 / 五键必填与运行时校验规则 /
  非交互约束**）属**生成面**，随 R4c 落地时另登一行——不在此处预写未实现的事实。

- 2026-09-15 复现点 19 入册（ADR-0023 R4b/R4c，随 SSH 生成面落地）：补齐该 ADR 行动项要求
  但第 18 行**刻意未预写**的那部分（生成面事实）——四包映射、五键与运行时校验、非交互约束、
  平台硬错、范围限定、无上游范本。**本轮新增的实证是"四包必须齐 + `dsh-ssh` 必须最前"**：
  依赖边只有 `ssh` / `sandboxPolicy` 两条，故 `fs-ssh`/`subprocess-ssh`/`sandbox-ssh` 彼此无序，
  但**都在 `dsh-ssh` 之后**；壳侧据此把 `SSH_PACKAGES` 的顺序当语义并单测钉住
  （`four_rows_are_written_with_ssh_first`）。**注**：本行不含"装第二个同族 provider 会激活失败"
  一类**互斥族**结论——那是 ADR-0020/复现点 13 的范围，本功能不涉及。
- 2026-09-16 实验能力开关重构：复现点 20 入册（上游无运行时 feature flag；「关而不卸」= 行级 `disabled`；层类能力须走 `remove` 才干净；分类只能在包已装后判定）。
- 2026-09-16 复现点 21 入册（ADR-0020 §8，随「挂载行载荷 + 前置门 + 启动可见性」落地）：
  **MCP provider 的行载荷是必填契约，且失败是"零输出 + 迟报"**。四项事实全部以真机
  隔离复现（`cp -Rc` 克隆 dsh home，未动用户现场）取得，不靠文档推断：
  ① 浏览器族 MCP provider 的 `Config` 里 `mode` 为 **required**（`launch` / `attach`），
  缺 `config` 时 `apply` 拿到 `undefined` → `TypeError: Cannot read properties of
  undefined (reading 'mode')`（`browser-use-runtime/lib/types/mcp.js:26` `validateBrowserMcpConfig`，
  锚上游 `packages/experimental/browser-use-runtime/src/types/mcp.ts` 的 `BrowserMcpConfig` 联合）；
  ② `cua-driver-mcp` 的 `Config.command` 默认 `cua-driver`，缺该可执行文件时
  `spawn cua-driver ENOENT` → `mcp-client(cua-driver-mcp): initial connection or tool
  synchronization failed`（锚 `packages/experimental/computer-use-cua-driver-mcp/README.md`
  「Choose this provider when Cua Driver is already installed」）；
  ③ 二者都表现为**整棵 plugin tree 拒绝加载**：`dsh: plugin tree failed to load: failed to
  apply loader entry include (cordis:include): loader entries failed to apply`，且
  **全程零 stdout/stderr**——错误只在 boot promise 拒绝时打印，实测 `--profile web --port 0
  --no-open` **34 s 后退出码 1**（这是"壳 20 s 判卡死 → SIGKILL → 日志全空"的直接成因）；
  ④ 上游日志里同时出现两条 `failed to apply loader entry …`，第一条的 `include` 是 loader
  条目名**不是行 id**——解析必须逐处扫并按行 id 形态（含 `-`）过滤，壳侧据此实现
  `boot_failure::parse_failed_loader_entry`。
  **升级复核触发**：上游若给 `mode` 补默认值 / 让 `Config` 变可选 / 打包分发 `cua-driver`
  / 改为失败即快退，本节四条逐条重测（ADR-0020 §8.5）。
- 2026-09-16 复现点 22 入册（ADR-0025，随「安全模式」落地）：**`--patch` overlay 的层栈位置与
  停用语义**。五条以只读源码 + 克隆体实测取得：① 层栈序 `bundle → profile 层 → home 层 → --patch`，
  各层挂载前**拍平成单列表**（`apps/cli/src/profile-boot.ts:212-219,246-249`、
  `packages/boot/app-boot/src/index.ts:393-395`），故 overlay 可停用任何更早层的行；
  ② `- id: x` + `disabled: true` 覆盖该行字段，**id 匹配不到只 warning 不报错**
  （`vendor/include/src/index.ts:110-112,120-123`）——"能禁就禁"因此安全；
  ③ `--dump-*` 只打印**永不 boot**，`--dump-default-config` 与 `--patch` **互斥**
  （`apps/cli/src/args.ts:100-105,113-115`）；④ `$DSH_HOME/cordis.patch.yml` 是**第二个用户层**
  且优先级高于 profile 层（`profile-boot.ts:77-79,243`）→ 救援 profile 躲不开它；
  ⑤ **源码与构建产物分歧**：`vendor/loader/src`（@0d1f500）对坏行宽容（降级 warning），
  而 `vendor/loader/lib`（陈旧构建，与已安装运行时**逐字节相同**）严格抛错 + 整组回滚
  （`src/config/entry.ts:175-189`、`group.ts:56-64` vs `lib/index.js:91-125,307-309,516-529`）
  ⇒ 本机跑的是严格版（解释 34–44s 退出）；"重构建能否消除非 required 行的致命性"**未实测**。
  实测两条：`--dump-config − --dump-default-config` 的 id 差集恰为用户层行；语法坏时
  `--dump-config` exit 1（故需"备份并放空"兜底）。**升级复核触发**：上游提供官方安全模式开关 /
  loader 构建口径变化 / `--dump-*` 输出格式变更。

- 2026-09-16 **复现点 22 的更正**（ADR-0026 §2 实测）：旧结论「三方 **bundle 层**行停不掉、
  整层摘掉必 `exit 1`」**不成立**。克隆体（`~/.dsh-dock-dev` 副本，未动现场）逐条复测：
  ① 在 `cordis.patch.yml` 里给**9 条三方行**（5 用户 patch 行 + 4 三方 bundle 行
  `agent-team`/`tool-agent-team`/`ui-agent-team`/`auto-review`）写 `- id: x` + `disabled: true`
  → **8.2s 正常就绪**；② 同一 9 行改用 `--patch` overlay → 7.9s 就绪（两种写法等价）；
  ③ 再多停**一条随包行** `tools` → `exit 1`：`required startup failure: 1 entry did not activate`
  + `agent-loop (@deepseek-ai/dsh-agent-loop): pending (waiting for service: tools)`
  ——**这正是当年那次 `exit 1` 的签名**，真因是判据把"被用户 patch 过的随包行"也算成三方行
  （dump 段落标签形如 `@deepseek-ai/dsh-base, patched by …/cordis.patch.yml`）。
  ④ 判定随包与否必须取**段落主段**（`, patched by` 之前），见 `safe_mode::primary_section`。
  **升级复核触发**：`--dump-*` 的段落标签格式变更（判据依赖 `, patched by` 与主段形态）。

- 2026-09-17 **复现点 21 入册 + 更正一条旧结论（ADR-0009 §4 4.4③ 的「运行语义」）**：
  旧记「patch 变更对运行中会话不热生效（hmr 默认停用），重启后生效」——对基线
  v0.1.1-rc.2 成立，对 dsh 0.1.6-alpha.1 的 **`patchReload: live` profile（出厂 `web`
  模板即 live）不成立**：启动器装 chokidar 盯 `cordis.patch.yml`，热应用到运行中的树
  （见复现点 21；克隆实机实测 0.43s 注销 / 0.44s 重建）。触发：维护者实机报
  「刚打开插件却还是显示已停用，切到别的标签页再切回来又是运行中」——**真因在壳侧**：
  开关写成功后只重取行表（配置），运行态快照停在进页面那一刻，直到切页重挂才自愈；
  同时行内「已禁用」（配置）/「已停用」（运行）两个近义词并排且无解释。
  已修：开关写完后立刻重取运行态并短轮询到落定（期间该行显「生效中」）；运行侧徽标按
  真相源拆成 运行中/加载中/失败/未加载（配置启用但会话内无实例）/未生效（会话里这行
  仍是禁用），各带自解释 title；配色不再把"没到位"刷成 ok 绿；开关标签与 toast 不再写死
  「重启后生效」，改按观测结果回报（已生效 / 重启后生效）。
  **升级复核触发**：`patchReload` 的取值与默认、监视器安装条件（`ctx.get('hmr')` 回退
  挂载）、`pluginInventory/list` 的 `enabled`/`fiberPhase` 语义、`entryId` 前缀形态。

- 2026-09-17 **复现点 18 / 19 退役（SSH 远程工作区整体移除）**：维护者裁定「SSH 远程工作区向导
  这个功能相关的代码逻辑都干掉」。当日退役面：`ssh_config.rs` / `ssh_profile.rs` / `ssh_remote.rs` /
  `commands/ssh.rs`、前端向导与 `sshWizardGate.test.ts`、三条 IPC 命令（`list_ssh_hosts` /
  `probe_ssh_target` / `generate_ssh_profile`，登记册 65 → **62**）＋ `capabilities` 三条授权、
  `network_gate.rs` 的 ssh_remote 整文件登记、登记册 §二「SSH 非交互预检」行与 §三「用户 SSH 配置」
  读取域、AGENTS §6 的 app-bundle 写入例外、`profiles.rs::ssh_app_bundle`（"指定 bundle"参数化随之
  收回单一路径）。**该功能未随任何版本发布**，故两行降级为决策史而非"现行复现点"：
  OpenSSH 解析语义（`#` 只在行首为注释、单值关键字取首个参数、首个取值优先、通配段填充、取反排除、
  `HostName` 缺省 = alias、`Include` 不跟随——真 `ssh -G` 实测口径）与 `dsh-ssh` 五键/非交互约束
  仍可作为将来重启该功能的起点（ADR-0023 状态：已撤回，正文保留）。**升级复核触发**：不再需要
  （无代码依赖）；若重启本项，须重新入册并复核上述语义是否漂移。
