# dsh 移动端（手机操控 PC）可行性研究

> 状态：**研究稿，未经裁定**（2026-09-19）。落档于此供频道讨论；任何动手前须先立 §8 的 ADR。
> 证据口径：dsh 侧结论读自本机 `@deepseek-ai/dsh` **0.1.6-alpha.2 编译产物**
> （`engines/dsh-runtime/node_modules/@deepseek-ai/`，无 `.ts` 源码，行号为编译产物行号，
> 复核日期 2026-09-19）；壳侧结论读自本仓库 `master` 工作区。

## 0. 结论先行

1. **「手机直连 dsh」这条路是上游主动封死的，不是我们没找到开关。** dsh 的 `--host 0.0.0.0`
   在启动时被硬性拒绝，报错原文：`--host 0.0.0.0 is intentionally not supported yet for
   safety: it would expose remote code execution to the network; use 127.0.0.1 instead`
   （`dsh-web-app/lib/startup.js:40`），且 bind host 的 schema 只允许两个字面量
   `127.0.0.1 | 0.0.0.0`（`dsh-host-webserver/lib/index.js:141`）——连"只绑某个内网网卡 IP"
   都表达不了。**在不修改 dsh 源码的红线（AGENTS §0 红线 1）下，任何移动端方案都必须由我们自己
   出一个入站监听面**，这个角色只有 dock 能承担。
   → 本研究的结构性结论：移动端**不是"再加一个前端"**，而是**壳第一次成为入站服务方**，
   属安全边界变更，必须先 ADR。
2. **但壳做「认证反向代理」技术上完全成立**，而且不需要碰任何上游禁令：
   dsh 的 Host 栅栏是**头部判定，不是套接字判定**——`isTrustedApiRequest()` 只看三个请求头
   （`dsh-client-connection/lib/index.js:201-215`）：`Host` 是回环即放行、
   `sec-fetch-site: cross-site` 才拒、**无 `Origin` 直接 return true**。
   原生 App 不发 `Origin`/`sec-fetch-site`，只要把 `Host` 归一成 `127.0.0.1:<port>`，
   再带上壳**本来就已经在内存里合法持有的** Cookie（`src-tauri/src/boot.rs:496`
   → `ShellState.workbench_cookie`；用法见 `commands/session.rs:137`），
   dsh 就把这个客户端当成正常浏览器。**不需要 `--trusted-host`，不需要伪造签名，不需要读
   `.credentials.yaml` 里的签名密钥**。
3. **对标 Qoder 移动端的能力清单，dsh 侧的接口基本都在**：`/api/remote.mux` WebSocket
   推 `session.event` / `session.status`（`dsh-api-gateway/lib/index.js:11`），
   `session/{list,search,follow,prompt,cancel,selectModel,modelCatalog}`、
   `workspace/changes`、以及审批链路 `approval/request`（`dsh-user-approval/lib/index.js:179`）。
   **真正的缺口只有两个：通知推送、设备配对。**
4. 推荐路线：**分级放权**（只读 → 写 → 管理面）＋ **传输层不自研信任**（默认只在
   用户自带的 overlay 网/内网可达，配对用一次性 token）＋ **推送走不承载控制指令的
   通知旁路**。**明确不做**：自建云中继让第三方服务器经手用户代码（见 §3 D 行）。

## 1. 对标：Qoder 移动端到底做了什么（公开资料，2026-09-19）

形态：原生 iOS / Android / HarmonyOS（**CN 与国际版是两套不互认的账号体系**），
另有 `qoder.com/agents` Web 控制台。

| 维度 | Qoder 的做法 | dsh-dock 能不能照抄 |
|:---|:---|:---|
| 能做什么 | 实时跟看 PC 上跑的任务、回答 agent 提问、追加指令、**审批关键操作**、看执行计划/diff/产物、驱动云端 agent | dsh 侧 RPC 基本对得上（§2.1） |
| 不能做什么 | 手机本地跑代码、屏幕镜像 IDE（控制是**任务/会话粒度**，不是桌面镜像）、PC 关机后继续本地任务 | 同理，且"PC 关机不中断"我们更做不到（§3 F） |
| 配对 | IDE：设置里两个开关 + **同账号登录，无 QR**，手机自动看到机器；CLI：`/remote-control` 或 `qoder remote-control` 守护，**QR / URL 配对，普通浏览器可扫** | **我们没有账号体系**——这是第三方壳与官方产品最大的差距，配对必须自研（§6） |
| 传输 | 文档未写；从"跨网可用 / 蜂窝可用 / 有 Web 控制台 / 同账号联动"判断**几乎确定是厂商云中继**，机器侧发起出站连接 | 照抄= 我们要运维中继并经手用户代码，不可取（§3 D） |
| 唤醒 | 有显式「保持电脑处于唤醒状态」开关；社区反馈"合盖休眠手机这儿就断了" | 可直接做，且**成本极低、价值极高**（§4 阶段 1） |
| 通知 | APNs/FCM + iOS 实时活动/灵动岛做审批提醒 | 需开发者账号与服务端 key；用通知旁路可绕开大部分成本 |
| 撤销 | **未证实**：只找到账号鉴权 + PC 侧显式开关两道，未见按设备撤销/会话失效文档 | 我们必须自己设计（§6） |

参照系（同类产品的取舍）：Claude Code 云会话跑在**厂商托管 VM**（OAuth 拉 GitHub，结果以
可评审分支回推）；Cursor 有云中继 agent + 2026-09 的"自托管机器"；开源的 Happy 用
**只看见密文的中继 + 端到端加密 + 权限请求推送**；Vibetunnel 类走 **Tailscale WireGuard
直连，无中继、也无推送**。→ 行业上只有两条诚实的路：**要么信任一个中继（那就必须 E2EE），
要么信任一个 overlay 网络（那就放弃推送）**。

## 2. 地基事实

### 2.1 dsh 侧（决定方案形态的六条）

| # | 事实 | 指针（0.1.6-alpha.2 编译产物） |
|:---|:---|:---|
| 1 | 只绑回环；`0.0.0.0` 启动即报错；host schema 只有两个字面量 | `dsh-web-app/lib/startup.js:40`；`dsh-host-webserver/lib/index.js:141,296` |
| 2 | 端口由 `--port 0` 交给 OS，壳从 stdout 解析 URL（壳不选端口） | 壳侧 `src-tauri/src/shell.rs:110-125,233` |
| 3 | `/api` 双栅栏：**403** Host/Origin/sec-fetch-site（头部判定），**401** 签名 Cookie | `dsh-client-connection/lib/index.js:201-215,554-556` |
| 4 | 鉴权=进程级 launch token（32B base64url）→ `GET /?token=` 换签名 Cookie，**默认 30 天**、HttpOnly、authority 绑定 | 同上 `:219-246,385-408,740` |
| 5 | 有 WebSocket 多路复用 `/api/remote.mux`（`noServer` + 同一 `requestRejection` 前置校验，2s 心跳）→ 事件流不是从零造 | `dsh-api-gateway/lib/index.js:11,223,463-468` |
| 6 | 会话所有权 = `flock(2)` 非阻塞、**故意不设过期**；读侧不取锁，写侧单 owner；第二个客户端并发 `prompt` 撞锁，网关把它翻成 `RemoteError("session/writer-held")` | `~/.dsh/sessions/<slug>/session-<uuid>/session.lock`；`dsh-api-session-controller/lib/index.js:239` |

无隧道 / 无配对 / 无 remote-device 概念（grep `tunnel|pairing|pair code|remote access` → 0 命中）；
dsh 语义里的 "remote" 是进程内类型化 RPC，不是远程设备。**上游没有任何"官方移动端"的地基可依赖。**

### 2.2 壳侧：有什么 / 没有什么

有：`workbench_cookie` 与 `workbench_url`（内存态）、62 条 IPC 命令
（`src-tauri/src/ipc.rs:14-77`，登记册 `docs/contracts/ipc-and-network-register.md`）、
Tauri 事件总线（`boot:step/error/update/progress`、`app:*`，`frontend/src/lib/events.ts`）、
生命周期守卫（`lifecycle::spawn/run` + `procs/` 锁登记 + 孤儿清扫，ADR-0015）、
headless 对照通道（`dsh --profile headless "<task>" --json` → NDJSON；另有 SDK stdio JSON-RPC 与 ACP）。

没有（**这些是"从零"的部分**）：任何入站监听（全仓零 `TcpListener`，唯一一处在闸门测试里
`network_gate.rs:1057`）、任何 WebSocket/SSE 服务端或客户端、任何推送通道、任何配对/token 机制、
审批/权限应答命令、会话 watch 命令。跨窗真相源只有事件广播（§4.4 三红线 3），
移动端等于把这条约束放大成"跨设备真相源"。

## 3. 候选架构对比

| 方案 | 一句话 | 致命点 | 与本项目红线的冲突度 | 裁定 |
|:---|:---|:---|:---|:---|
| **A 手机直连 dsh workbench** | 手机浏览器访问 `内网IP:端口` | dsh 根本不绑非回环（§2.1#1） | 需改 dsh 源码 → **踩红线 1** | **否** |
| **B 壳做认证反代 + 方法白名单**（推荐主路线） | 手机 → 壳监听 → `Host` 归一 + Cookie 重放 → `127.0.0.1` | 壳成为 RCE 能力的暴露点；协议是上游内部实现，无契约保证 | 入站面=新边界（须 ADR）；`TcpListener` 须在 `network_gate.rs::EXEMPTIONS` 登记；§7「唯一网络面」需重述为「唯一**出站**面 + 受登记入站面」 | **采纳** |
| **C 任务通道（headless/SDK stdio/ACP）** | 手机下发**新任务**，壳 spawn headless，NDJSON 回传 | 接管不了桌面正在跑的会话（headless 拒接活着的 in-process agent）；审批语义要在壳层重建 | 几乎无冲突（不出站、不入站 dsh、不碰 Cookie） | **采纳为阶段 1 的对照/降级通道** |
| **D 自建云中继（Qoder 模型）** | 机器侧出站长连到中继，手机走同一中继 | 我们没有账号体系；要运维服务器；**用户代码/会话内容经我们的服务器** | 与"第三方壳"的身份根本不匹配（信任与合规双杀） | **否**（除非 Happy 式：只过密文，或退化为 D′ 通知旁路） |
| **D′ 通知旁路** | 只推"该看一眼了"，**不承载指令、不承载内容**（或 E2EE）；通道用现成的（ntfy/Bark/自建 APNs key） | iOS 后台仍受限；审批提醒有延迟 | 新增出站用途 → 登记册 §二 + `EXEMPTIONS` | **采纳（分期）** |
| **E overlay 网络作 B 的传输层** | 壳只绑 tailnet 接口，WireGuard 负责鉴权，我们**不自研密码学信任** | 用户要多装一个东西；无推送 | 反而**降低** B 的风险面 | **采纳为默认安全姿态** |
| **F 云沙箱（Cloud Agents 模型）** | 任务搬到远端跑，"关电脑也不中断" | 这不是"操控 PC"，是替代 PC；要自建 runner | 对应 `docs/adr/README.md` 里 **ADR-0023 SSH remote 已于 2026-09-17 撤回**，`docs/roadmap.md:210-221` 的 SSH executor 仍在 Later | **列为长期，不混入一期** |

**关键取舍：B + E + D′ 组合**。B 解决"能连"，E 解决"凭什么信"，D′ 解决"手机怎么知道该看"。
C 作为阶段 1 的对照通道先跑通端到端体验，B 作为阶段 2 的真接管。

## 4. 推荐路线（分期，每期独立可交付）

**阶段 0 —— spike，不动产品代码**（落 `docs/spikes/0005-mobile-control-proxy.md` +
`docs/contracts/dsh-behavior-ledger.md` 新增复现点）。要证伪/证实的四条命题：

1. 壳侧最小 HTTP/WS 反代：`Host: 127.0.0.1:<port>` + `workbench_cookie` 重放，
   第二客户端能否跑通 `POST /api/session/list` 与 `/api/remote.mux` 的 follow？
   （curl 模拟即可；顺带证实"无 Origin 即放行"这条）
2. **写锁竞争真机表现**：桌面浏览器正持有会话时，第二客户端 `session/prompt` 到底返回什么、
   UI 如何不误导（§2.1#6）。
3. **审批能否由第二个客户端应答**：`approval/request` 是 cordis waterfall（
   `dsh-user-approval/lib/index.js:179`），浏览器侧 `dsh-client-ui-approval` 是应答者。
   第三方 mux 客户端能不能抢到这个应答权——**这是"手机端审批"整条价值主张的生死题**。
4. headless `--json` NDJSON 作为对照通道：事件是否足以驱动一个移动端进度视图，
   以及 `--session-id` 续跑的体验边界。

**阶段 1 —— 只读 + 唤醒 + 通知**（风险最低、日常价值最高）。
Qoder 移动端的核心价值其实就是"人离开电脑也知道进度"。范围：保持唤醒开关
（对标 Qoder 那个 toggle，`caffeinate`/IOKit 断言）、会话列表/状态/进度只读、
D′ 通知旁路。**只读面不引入写能力 → 反代即使被攻破也只是信息泄漏，不是 RCE。**

**阶段 2 —— 写通道（真正"操控"）**：`session/prompt` / `cancel` / `follow` / `selectModel`
+ 审批应答。**方法白名单是硬要求**（默认拒绝，逐条放行；`terminal/write`、
`settings/mutate|replace`、`credentials/set` 一期一律不放）。每次远程写操作在 PC 侧
显性可见（横幅/日志），并留审计。

**阶段 3 —— 管理面**（dock 独有、不重复 dsh 官方 Web UI 的部分）：多实例总览、
profile 切换、插件/MCP 开关、诊断。这一步才回答 positioning 文档的「看见」动词——
**dsh 官方 Web UI 已经覆盖单实例会话控制，移动端只做单实例是自我重复**
（`docs/plans/positioning-pivot-roadmap-2026-09-18.md`）。

## 5. 客户端形态

推荐 **Tauri v2 mobile**：与壳同一栈，`frontend/` 的 React 19 + Tailwind v4 + shadcn/ui +
Zustand 直接复用，不触发 §4.2「禁引 React 生态外前端框架」；网络与配对密钥留在移动端
Rust 侧，**§4.4 三红线 2「前端运行时禁发起网络请求」在移动端语义下依然成立**，
心智模型与桌面壳同构。代价：新增依赖（二维码、Keychain/Keystore、推送）要回写依赖白名单。

本机工具链现状（2026-09-19 实测）：Xcode 26.6 ✓、`tauri-cli 2.11.4` ✓（与 crate 同代，
§1 锚点满足）；**CocoaPods ✗、iOS/Android rust target ✗、Android SDK ✗** → 移动端构建链
要从零搭，签名/公证可复用 `docs/macos-signing.md` 的经验但不能直接套用。

备选：移动 Web/PWA（本质就是访问 workbench UI，但 dsh UI 未做移动适配，且无后台与推送）、
微信小程序（国内分发现实，但长连接与后台受限，不适合实时流）。

## 6. 风险（按致死度排序）

1. **把 RCE 能力开放到网络**——上游明确拒绝的事我们代理做了一遍。必答题是
   **「凭什么我们比上游更保守」**，答不出就别开写通道。建议硬约束写进 ADR：
   默认关闭 + 默认只绑 overlay/内网接口 + 默认只读 + 写方法白名单 + PC 侧显性指示 +
   token 可按设备撤销 + **永不端口映射/UPnP**。
2. **上游漂移**：Host 栅栏、Cookie 派生、mux 路径、审批 waterfall 全是 dsh 内部实现，
   无契约保证。承接机制=仓库已有的复现台账（`docs/contracts/dsh-behavior-ledger.md`），
   dsh 升级逐条复核；这条决定"能不能可维护"，比技术难度更重要。
3. **没有账号体系**：Qoder 的"同账号自动看到机器"我们抄不了。自研配对=自研信任，
   最容易出安全事的正是这一环。倾向：**一次性高熵 token + QR（含短时效）+ 每台机器
   显式开关 + 可撤销列表**，身份问题交给 overlay 网络（E）。
4. **推送是独立成本**：iOS 要开发者账号 + APNs key + 一个服务端角色；Android FCM 类似。
   D′ 用现成通道能把成本压到很低，但要把"通知不承载指令"写死成规则。
5. **会话单 owner**（§2.1#6）：手机与 PC 同时操作会撞锁。必须 UX 化成"PC 正在输入"，
   不能让人以为"点了没反应"。
6. **合盖即失联**：唤醒保持只能挡 idle sleep， clamshell/电池策略各有差异，要真机验证。
7. **品牌与合规**：dsh 是 `@deepseek-ai` 的产品，第三方壳做"远程控制其 agent"是否有
   品牌/授权问题——**不定性，交维护者确认**（AGENTS §8.6 不确定就问）。

## 7. 附带发现（先于移动端，建议立刻单独修）

研究过程中实测确认：**dsh 的 launch token 明文落盘在壳日志里**。
证据（本机 dev 实例，已脱敏）：`<app_data>/shell.log` 自 2026-09-08 起累积 **402 行含 `token=`**，
形如 `INFO DSH 已就绪，进入 http://127.0.0.1:51461/?token=<b64url>`（同一次还进了 `boot:step`）。
另一路是 `dsh-shell.log`（`src-tauri/src/shell.rs:72-92` 把 dsh stdout 原样 tee 进文件，
每启动 truncate），dsh 自己打的地址行也带 token。

为什么算事故：token 是**进程级**的（进程死了就作废），但用它换来的签名 Cookie
**默认 30 天有效**（§2.1#4）。任何能读到 `app_data` 的东西（备份、Time Machine、
日志上传、诊断页复制粘贴）都可能在有效期内拿到对"可执行代码的本地服务"的会话凭据。
这直接违反 AGENTS §4.3「绝不打密钥/PII」。

修法建议（一句话，另开工单）：日志出口统一 redact `token=` 与其后的 base64url，
`boot.rs` 的就绪消息只记 authority 不记完整认证 URL；同时把 §2.1 的 token/Cookie 事实
补进复现台账。**注意：一旦上了移动端（B 方案），这个泄漏的爆炸半径会显著放大——
先修再谈入站面。**

## 8. 待裁定的 ADR 议题（动手前必须立）

| 议题 | 为什么必须 ADR |
|:---|:---|
| ADR：壳首次引入**入站监听面** | 安全边界变更；同时要把 §7「唯一网络面 = `updates.rs`」重述为「唯一出站面 + 受登记入站面」，并新增 `network_gate.rs::EXEMPTIONS` 条目 |
| ADR：**移动端 = 第二个壳工程**？ | 目录形态（同仓 `frontend-mobile/`？独立仓？）、依赖白名单扩展、跨设备真相源规则（§4.4 三红线 3 的放大版）、CI 是否覆盖 |
| ADR：**方法白名单分级**（只读/写/管理面）与默认姿态 | 决定阶段 1/2 的边界，违约即事故 |
| 通知旁路是否允许第三方推送通道 | 与出站面登记规则冲突与否 |
| 契约影响评估 | 若移动端要读 `product.manifest.json` 才需要升 `MANIFEST_FORMAT`；初步判断**不需要**（走运行时接口即可） |

## 9. 未证实 / 存疑（不猜，留给阶段 0）

- `/api` 是否接受把 launch token 直接放 header/query（代码里只看到 Cookie 路径）。
- 已授权的第二客户端能否在桌面浏览器持有会话期间成功 `session/prompt`（由锁语义推断，未真机跑）。
- 审批 `approval/request` 能否被第三方 mux 客户端应答（§4 阶段 0 命题 3）。
- dsh 完整的 `/api/*` 路由表（只 grep 了前缀注册，`dsh-web-frontend` 侧可能还有）。
- 本机 pnpm 全局装的是 `0.1.5-alpha.1`，壳引擎里是 `0.1.6-alpha.2`——**结论只适用于 0.1.6-alpha.2**。
- Qoder 中继的协议细节、以及"手机丢失后如何撤销"——公开文档未覆盖，**不可当设计依据**。
