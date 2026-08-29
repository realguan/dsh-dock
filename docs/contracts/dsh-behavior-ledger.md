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
| 11 | 插件运行时清单（4.4 前置，Spike B） | 壳侧回环调用（实施时落位；仅会话在跑时 `POST http://127.0.0.1:<port>/api/pluginInventory/list`，信封 `{type:"client-request",rpcId,method,payload:{args:{}}}`，见 `docs/spikes/0002-plugin-inventory.md`） | unary 调用兼走普通 HTTP POST（`dsh-client-connection` callUnary @ 6203-6211：`postJson("/api/${method}")`）；`payload` 恰一 plain-object `args` 字段；响应 `{entries:[{entryId,moduleName,enabled,fiberPhase}]}`（`dsh-host-plugin-inventory/typert.host.js` schema；FiberState→phase 映射 index.js @ 33-46，disposed→null）；回环无鉴权门（伪造 Host 仍 200，2026-08-29 实测）；patch/配置行 id ≠ entryId（无组前缀 vs `include:*` 树路径），patch 写入 id 以 `--dump-config` 行 id 为准 | 2026-08-29 |

## 二、计划复现点（4.3 Profile 管理器落地时入册）

（空——复现点 6/7/8 均已落地，见第一节；后续新增复现点在此登记后随实现转一。）

## 三、复核记录（append-only）

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
