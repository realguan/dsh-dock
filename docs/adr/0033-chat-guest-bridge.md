# ADR-0033：通过受限标题信道桥接内嵌聊天子视窗

- **日期**：2026-10-10
- **状态**：已接受
- **提出人**：维护者授权的实现任务
- **相关方**：`commands/chat_embed.rs`、`ipc.rs`、`capabilities/default.json`、`dsh-deepseek-chat-embed`
- **关联**：ADR-0003（导航边界）、ADR-0032（web 宿主定位）

---

## 1. 背景与问题

DSH Dock 以 Tauri 原生子 WebView 加载 `https://chat.deepseek.com/`，绕过该站点拒绝 iframe 的 `frame-ancestors 'none'`。子 WebView 与 DSH 主页面分属不同 DOM/runtime，插件无法像 Electron `<webview>` 那样直接执行脚本并读取返回值，也无法同步捕获客页侧栏点击；因此两栏互斥联动无法对齐。

## 2. 约束与硬指标

- 不移除、代理或重写 DeepSeek 的 CSP 响应头；不改 dsh 源码。
- 客页不获得任意宿主 IPC：不得给 `chat.deepseek.com` 增加可调用命令的远端 capability。
- 任意 guest eval 只允许主 DSH 工作台的本地 loopback WebView 发起；Rust 校验调用方 label 与来源 URL。
- Guest→host 信道只承载短小、带固定前缀的状态结果或 `expand` 事件；不记录页面内容、令牌或凭据。
- 行为保持跨 macOS、Windows、Linux 一致；Tauri 各平台均使用通用 document-title callback。

## 3. 备选方案及评估

### 方案 A：本地反向代理改写 CSP —— ❌ 否决

- 思路：代理 chat.deepseek.com 并删除 frame 限制。
- 否决理由：改变认证、Cookie、Cloudflare/Turnstile 与站点来源语义；触及壳禁止触网边界，且存在风控及安全风险。

### 方案 B：允许远端 guest 调用自定义 Tauri IPC —— ❌ 否决

- 思路：给 `https://chat.deepseek.com` capability，并由客页调用 signal 命令。
- 否决理由：即便命令很窄，也把宿主自定义 IPC 授权给远端页面及其脚本；没有必要扩大权限面。

### 方案 C：主页面受限 eval + 子页标题回传 —— ✅ 采纳

- 思路：主页面唯一新增的 `embed_chat_eval` 命令校验来源并向子 WebView 执行短脚本；子页及 eval 包装器只短暂设置固定前缀的 `document.title`，由 Rust `on_document_title_changed` 识别、校验后向 `main` 发本地事件。标题立即恢复。
- 优点：不授予远端页面 Tauri capability；复用 Tauri 已支持的 WebView eval/title-change API；回传脚本结果及客页按钮点击都可走同一信道。
- 代价/风险：标题属性短暂变化；需限制长度、校验 request id/事件种类并忽略普通页面标题。
- 对照约束：eval 命令仅主窗口 loopback 可用；远端子页不获任何 ACL；消息前缀、大小和事件种类均采用白名单；不发网络请求。

## 4. 最终决策

采用方案 C。远端聊天页面不获得 IPC capability；`embed_chat_eval` 仅供本地 `main` WebView 调用，调用者 label 必须为 `main` 且 URL 必须是 loopback。Rust 仅转发规定前缀、大小受限的标题信号到 `main`，并立即恢复标题。

## 5. 后果与后续行动项

### 正面后果

- Dock 插件可以通过同一个适配器读取/切换客页侧栏，并接收客页展开侧栏的即时信号。
- 远端客页不会获得 Tauri command 权限。

### 负面后果 / 新增债务

- 依赖 WebView title-change 通知；每次消息都会短暂改写子页 document.title。标题恢复若被页面并发更新覆盖，下一条普通标题不作为桥消息处理。
- eval 只承诺用于插件内固定脚本与短结果，不是通用开发者工具接口。

### 行动项

- [x] 为 `embed_chat_eval` 完成 IPC、调用方校验与标题消息桥（2026-10-12：`commands/chat_embed.rs`
  + `ipc.rs` / `lib.rs` / `capabilities` / `frontend/src/lib/tauri.ts` 四处同步 + 登记册补登五条）。
- [x] 插件侧用 request-id 配对短结果，并接入客页侧栏点击事件（2026-10-12：插件
  `v1.7.0-dock-bridge` 的 `mountTauriGuest`，单事件 `dsce:guest-message` 分派
  `result` / `expand`；调用串行化 + 深度上限，避免 80ms 轮询在桥卡住时堆积）。
- [ ] Rust/frontend/plugin 测试通过；macOS DSH Dock 实机验证工具栏和侧栏互斥。
  （Rust 542 项 / 桥专项 11 项、前端 `tsc`、插件契约 64 项均绿；实机待验。）

## 6. 复审条件

Tauri/Wry 移除或改变 document-title callback、跨平台测试显示标题桥不可用、DeepSeek 客页开始主动冲突设置标题，或需求扩大到承载任意消息/脚本时，必须复审；不得扩充远端 capability 作为静默替代。
