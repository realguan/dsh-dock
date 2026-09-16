# ADR-0006：唯一网络面 updates.rs 与四路镜像链 / 下载双超时

- **日期**：2026-08-26
- **状态**：已接受（汇聚 §7 长期裁定，本 ADR 追溯补录）
- **提出人**：guan
- **相关方**：updates.rs（四路镜像链 / 下载循环 / `APP_RELEASE_FEED`）
- **关联**：node-map/README.md（签名映射包）、AGENTS.md §7（网络边界）
- **注（2026-08-27 边界重定义）**：本 ADR 的「唯一网络面 = updates.rs」约束的是**壳运行时
  网络**（下载 Node/dsh/自更新）。管理功能（4.3+）若引入 pnpm 自动下载 / dsh CLI 调用
  产生的网络，不属本 ADR 管辖——届时须单独评估网络面归属（见 AGENTS §0 边界 + §7 登记）。

---

## 1. 背景与问题

壳运行时有四类网络需求：包元数据/dsh 安装、Node 二进制下载、Node 版本映射包（签名验证）、客户端自更新源。若各模块各自触网，网络面会扩散到全仓，失控且难审计。同时国内网络下官方源慢/不通，需镜像优先 + 官方兜底；大文件下载在慢网络下合法地超过一分钟，整体超时会掐断合法下载。

## 2. 约束与硬指标

- 壳运行时网络面只在 `updates.rs`——其余模块不得触网，新网络面须先在 §7 登记。
- 网络动作一律后台线程，不阻塞 UI。
- Node 版本映射包必须 ed25519 验签后才采纳，失败回退内置基线（fail-closed）。
- 大文件下载不设整体上限；用「连接超时 + 单次读超时」双超时，慢网络合法慢不被掐断。
- 元数据用整体超时（小体积，超时即视为失败）。

## 3. 备选方案及评估

### 方案 A：四路镜像链 + 镜像优先官方兜底 + 双超时 + fail-closed 验签 —— ✅ 最终采纳

- 思路：包元数据/dsh（registry.npmmirror → registry.npmjs）、Node 二进制（cdn.npmmirror.com/binaries/node → nodejs.org/dist）、映射包（拉 packument+tarball，ed25519 验签，失败回退基线）、客户端自更新（`APP_RELEASE_FEED` = GitHub Releases latest API）。大文件用连接+单次读双超时，不设整体上限；元数据整体超时。
- 优点：网络面收口在 updates.rs；国内镜像优先、官方兜底；慢网络合法慢不被掐断；映射包验签 fail-closed 防篡改。
- 代价/风险：镜像域名写死，变更需改代码；双超时参数是经验值。
- 对照约束：逐条满足。

### 方案 B：各模块按需触网 —— ❌ 否决

- 思路：哪个模块需要就在哪个模块联网。
- 否决理由：违反「网络面收口」硬指标，审计与安全边界失控。

### 方案 C：大文件下载设整体超时 —— ❌ 否决

- 思路：下载整体 N 秒超时。
- 否决理由：慢网络下 40MB 合法地超过一分钟会被整体超时掐断，违反「慢网络合法慢不被掐断」。

## 4. 最终决策

唯一网络面 = `updates.rs`，四路镜像链：包元数据/dsh 安装（registry.npmmirror → registry.npmjs）、Node 二进制（cdn.npmmirror.com/binaries/node → nodejs.org/dist）、Node 版本映射包 `@dsh-dock/node-map`（packument+tarball，ed25519 验签后才采纳，失败回退内置基线 fail-closed）、客户端自更新源 `APP_RELEASE_FEED`（GitHub Releases latest API）。网络动作一律后台线程；元数据整体超时，大文件用「连接 + 单次读」双超时、不设整体上限。非 updates.rs 的网络需求先在 §7 登记再写。HOW 见 `updates.rs`。

## 5. 后果与后续行动项

### 正面后果
- 网络面单一可审计；国内体验优先且官方兜底；慢网络不误杀；映射包防篡改。

### 负面后果 / 新增债务
- 镜像域名写死常量，变更需改代码。
- 双超时与整体超时参数为经验值，需随真实网络调。

### 行动项
- [x] 四路镜像链 + 双超时 + 验签回退实现（updates.rs，33 个测试）。

## 6. 复审条件

- 镜像域名变更 / 新增镜像 → 同步改代码（不需重开本 ADR）。
- 下载策略改并发 / 分片 / 断点续传架构 → 本决策重开。
- `@dsh-dock/node-map` 签名机制变更 → 复评验签回退路径。
- 客户端自更新源迁移（非 GitHub Releases）→ 复评 `APP_RELEASE_FEED`。

## 6. 第二次修订（2026-09-16）：插件安装的 registry 选择策略

> 触发：真机装实验性插件时 `Failed to fetch metadata from https://registry.npmmirror.com/…`
> ——`@deepseek-ai/dsh-experimental-*` 的**部分 provider 在镜像上 404**（未同步），官方源 200；
> 叠加一次 `tls handshake eof` 抖动。维护者裁定（原文）：
> **「对于有条件的环境，直接走官方源，没有条件的环境走 npmmirror。」**

### 6.1 机制（为什么不新增网络面）

`dsh plugin --profile <n> <args…>` 是 **pnpm 薄转发器**：`apps/cli/src/plugin.ts:98`
原文 "registry names, and every other pnpm argument pass through"——所以 `--registry <url>`
可以**按次**指定，取包仍发生在 **pnpm 子进程内**（登记册 §二「子进程内触网」已覆盖）。
**壳不新增任何 in-process 网络客户端，也不做"可达性探测"**：所谓"有条件"由**实测结果**定义，
不由壳猜。

### 6.2 决策

1. **偏好三态** `pluginRegistry ∈ {auto, official, configured}`（`None` = `auto`）：
   - `official`：只传 `--registry https://registry.npmjs.org`；
   - `configured`：**不传** `--registry`，沿用 pnpm 的配置（本机 `~/.npmrc` → npmmirror，
     也兼容私有源）。**壳绝不改写用户的 npm 配置**；
   - `auto`（默认）：先按 `pluginRegistryLastGood` 试；无记忆时**先官方**（维护者裁定），
     失败后**换另一个源重试一次**。
2. **双向兜底**：两侧都可能缺——官方不可达 → 换 configured；镜像未同步 → 换 official。
   故兜底是**互逆**的，不是单向的（既有镜像链 npmmirror→npmjs 只覆盖了其中一个方向）。
3. **只在"换源有意义"时换**（纯函数分类）：网络类失败（TLS/连接/超时/metadata 取不到）
   与"包或版本不存在"才换；**构建审批门**（`ERR_PNPM_IGNORED_BUILDS`）、spec 非法**不换**
   （换源也白搭，且会掩盖真因）。
4. **记忆**（`pluginRegistryLastGood ∈ {official, configured}`）：**成功**才写，失败不写
   （抖动不得带偏记忆）。有记忆后首试即命中，不再先失败一次。
5. **如实呈现**：`PluginOpOutcome.detail` 必须写明用了哪个源、是否换过源；两个源都失败时
   **两侧错误都报**（只报最后一个会让用户看不出真因）。
6. 首次安装**可能多一次失败往返**（镜像缺包 / 官方不可达各一次），有记忆后消失——
   这是"自动"的固有代价，Preferences 里可固定为单源以消除它。

### 6.3 登记

- 新增两个持久化键 → `AGENTS.md` §6 运行时持久化例外册登记；
- 网络面**无新增**：本修订只改变传给 pnpm 的**参数**，登记册 §二「子进程内触网 · 插件装卸」
  补注"可按次指定 registry"即可，machine projection（`network_gate.rs`）**不变**。
