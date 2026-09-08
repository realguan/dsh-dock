# 📢 广播记录（公共频道知会档案）

> 协作知会的**仓库内正式载体**。依据 [`CONTRIBUTING.md`](./CONTRIBUTING.md) §0——
> 「所有必须共享的信息，唯一合法载体是仓库落盘内容」：聊天频道用于即时协调
> （占用抢先后到者得），但消息沉底即失忆、AI 冷启动读不到；知会类事件必须在此
> 落档。检索、审计、纠纷回溯一律以本档为准。
> 本文件是普通区 append-only 文档：只追加不改写历史条目（同 ADR 惯例），
> 不走宪法级修改流程。

## 一、登记范围（触发即记，当天落档）

| 类型 | 触发点 | 依据 |
|:---|:---|:---|
| 宪法级文件改动 | `AGENTS.md` / `docs/contract.md` / `CONTRIBUTING.md` / `node-map/` 的改动预告与合入归档 | AGENTS §10；CONTRIBUTING §3 |
| 快车道直推 master | 直合完成即记（单人小改通道同样适用） | CONTRIBUTING §2 流程图 I→M |
| PR 合并完成 | 合并人 squash 后记录 reviewer 与结论 | CONTRIBUTING §2 流程图 M |
| 发版事项 | 打 tag、冻结期起止、Release notes 征集与三平台验收 | CONTRIBUTING §8 |
| 占用声明/释放 | 频道声明后补一行即可（时效判定仍以频道时间戳为准） | CONTRIBUTING §3 |

## 二、条目格式

倒序追加（最新在上），一条一个三级标题：

```
### YYYY-MM-DD <类型> · <一句话主题> —— <发起人>
- 变更：<commit hash / 分支 / 文件清单>，两三行摘要
- 影响：<需要他人做什么动作；无需动作写「仅周知」>
- 凭据：测试结果 / diff 规模 / 频道消息时间点
```

漏记不补改旧条目——另发一条「补记」并注明原委。

## 三、记录

### 2026-09-07 建档 · Spike 0004：DeepSeek Harness SDK 能力源码调研 —— guan（AI 会话协作）

- 变更：新建 `docs/spikes/0004-dsh-sdk-capabilities-research.md`（+297 行，
  只读调研不触运行时）。核心结论：dsh 外部 SDK 是「本机进程集成边界」
  （stdio JSON-RPC 2.0 驱动完整 `dsh --profile` 子进程），不是托管 HTTP
  API；agent/工具/凭据/持久化/安全策略由 profile 决定，SDK 本身无会话
  存储层访问能力——会话修复等管理功能不能经 SDK 实现，维持文件层校验器
  路线（与 2026-09-07 会话维护两笔裁定互证）。
- 影响：仅周知；后续「经 SDK 集成」类诉求先读此档再立项。
- 凭据：纯文档；事实主张均锚 dsh 源码位置（d347e70，0.1.3-alpha.1）。

### 2026-09-07 快车道直推 · boot 启动页单主角重构 + 遥测文案去术语化 + get_boot_status 竞态补水 —— guan（AI 协作）

- 变更：
  1. **启动页单主角重构**：hero 区块撤编（与时间线重复讲述同一状态）——
     BootTimeline 卡成为唯一主角（卡头 = 徽标 + 当前状态标题/副题 + 分段
     进度条，danger 态转警示）；步骤行重做（done/pending 收单行 + running
     行保留完整遥测详情、等宽可选中 + 一键复制；竖向导轨容器层统一绘制，
     行高变化不再撕裂连接线）；下载进度经 banner 槽位入卡。
  2. **遥测文案去术语化**：boot.steps 五步 hint、executor/lib 的 sink/
     emit_step 遥测、崩溃守护与升级提示全部改为面向用户的可行动文案
     （PATH/spawn/tier/WebView/code= 等内部词汇不再外露；tier 经
     `tier_label` 映射「内置引擎/内置离线副本」）。
  3. **get_boot_status IPC 三处同步**（COMMANDS 登记 + handler +
     capability，AGENTS §7 登记）：BootIndex 挂载时播种缓存中的启动状态
     与错误（normalizeStep/normalizeError 入 bootStore）——修 WebView
     挂载前事件丢失的竞态；ErrorCard 动作触发前 clearError 防旧错残留。
- 影响：仅周知。`t.boot` 删除 consoleTitle/stError/stReady 三键（消费方
  已随重构移除）；新增 copyDetail/copied/progressAria。
- 凭据：cargo test 159 全绿 + fmt + clippy；前端 typecheck + oxlint +
  104 测试（全树在途状态下验证）；IPC 三处同步经 ipc.rs gate_tests 闸门。

### 2026-09-07 快车道直推 · 引擎引导加固：固化真实 node 二进制 + 引导版本口径放宽（rc 可用、alpha 仍拒） —— guan（AI 协作）

- 变更（ADR-0010 引导链修补，附在途复现工具一并入库）：
  1. **固化真实 node 到 `PNPM_HOME/bin/node`**：pnpm v12 `shim add node` 生成
     带上下文检查的 shim dispatcher，`pnpm add -g` 执行 postinstall（protobufjs/
     koffi 等）因子目录无 devEngines 声明报 `ERR_PNPM_SHIM_NO_TARGET` 退出 1。
     新增 `find_runtime_node_bin` / `link_real_node_binary`（symlink→硬链→复制
     三级兜底），`shim_add_node` / `install_dsh_global` / `bootstrap` 各环节接线；
     WSL 客体侧 guest 脚本同构修补（`ln -sf` 真实二进制）。
  2. **引导版本口径放宽**：`latest_stable_dsh_version` 由「仅稳定版」改为
     「稳定版或 rc 候选（`is_acceptable_dsh_version`），明确拒绝 alpha 等未
     稳定前置版」+ 测试——修 rc-only 发布期无引擎可引导的死路；alpha 事故
     预防口径不变。
  3. **打包资源补 `resources/pnpm/**/*`**：捆绑 pnpm 压缩包随壳安装包落地
     （ADR-0010 边界 A），此前缺失会致打包产物首启引导失败。
  4. 新增 `scripts/repro-boot-scenarios.sh`：引擎目录多场景裁剪复现工具
     （fresh/no-pnpm 等，只动壳自有资产，红线见脚本头注）。
- 影响：仅周知。引导行为变化点 = rc 版本可被自动引导；`bin/node` 由 shim
  变为真实二进制（生命周期脚本执行路径变化）。
- 凭据：cargo test 159 全绿（含新增 acceptable_dsh_version 用例）+ fmt +
  clippy（全树在途状态下验证）；真实引擎目录实证引导产物齐全。

### 2026-09-07 会话维护 · 归档感知 + 运行中复合判据 + 会话元数据透出 —— guan（AI 协作）

- 变更（grilling 共识后 Batch 1，意图 = 会话维护列表准确性）：
  1. **运行中误判修复（bug）**：active 判据由裸 `mtime<5min` 升级为复合式
     ——dsh 引擎进程存活（壳持有的会话执行器 `try_wait`，WSL 客体以
     wsl.exe 存活代理）**且** mtime<5min。Rust 经 `DSH_ENGINE_ALIVE=1/0`
     注入脚本（scan/repair 同源）。dsh 未运行时不再误判——修复了用户实测
     「归档/重命名会话被误标进行中 5 分钟、修不了」。残余竞态已评估并锚定：
     dsh 追加为逐批 `open("a")→write→fsync→close`（persistence-jsonl@0.1.2-rc.1
     appendLines），不持长驻 fd，失败批次回滚+游标重试，修复与写入竞争
     最坏情形 = dsh 稍后重放批次，无永久丢数据。
  2. **归档感知（bug/扩展）**：dsh 归档 = 仅原子重写 `~/.dsh/storages/
     workspace.json` 的 `global.archivedSessionIds`（2026-09-07 实证结构，
     会话日志零改动、仍可加载可修复；dsh 侧栏客户端过滤、无取消归档 API）。
     壳同口径：`SessionItem.archived` + 容错解析（缺失/损坏按无归档不阻断），
     默认隐藏 + 新增「已归档」筛选档；归档会话照常体检可修；统计卡/搜索
     跟随可见性（2026-09-07 口径 6 条）。
  3. **元数据透出（扩展项1）**：`--scan` 新增 createdAt（毫秒，实证真实日志）、
     eventCount（填实原恒 0 死字段）、endState（最后一条 turn/end 的
     reason.kind，真实语料全集 completed/aborted/error/open；无 turn/end 记
     open（未收尾））、subagent（dsh 侧栏亦隐藏子代理，壳标注展示不隐藏）、
     agentPreset；validator（校验器版本）透出到状态徽标 tooltip（0.1.3
     过渡期有用）。前端会话行新增元数据副行与三类徽标。
- 影响：仅周知。`list_sessions` 载荷扩展（无新 IPC 命令、不触 §7）；
  dsh 升级日「已归档集合经 workspace.json 读取」不受世代迁移影响。
- 凭据：cargo test 159 全绿 + fmt + clippy；前端 typecheck/oxlint/104 测试；
  真实语料 9 会话双态扫描（alive=1/0 复合判据行为正确）；fake-engines
  catalog 分支回归（原始损坏快照 needs_repair 检出）；同批工作区含另一在途
  boot 文案流改动，本次提交经 hunk 级摘取只含本意图 7 文件。

### 2026-09-07 会话维护补强 · dsh 0.1.3 格式世代前瞻适配（catalog 迁移管线 + 版本路由） —— guan（AI 协作）

- 变更（`scripts/repair-session.mjs` + `src-tauri/src/sessions.rs`；本条与该改动同
  commit 落盘）：
  1. **背景**：dsh 0.1.3（当前 alpha.1）将 `SESSION_FORMAT_VERSION` 0→2，存储改
     「不可变世代」模型——v0 源与 `session.vN.jsonl[.zstd]` 迁移世代并存，读取经
     `session-format-catalog` 迁移管线（decode → migrate → encode → 发布 v2 世代）。
     逐字核查其 jsonl 后端 / catalog / chain / filename 源码后提前适配。
  2. **校验器版本路由**：恢复层发现引擎档 `dsh-session-format-catalog`（与
     dsh-session 同代安装，`currentVersion` 须一致）时复刻真实读路径：readHeader
     分类 → decodeRecoverableArtifact → migrate → encodeCurrent → **迁移后表示**
     走同一条扫描+prepareCore 链；无 catalog（0.1.2 代）时维持「存储版本==已装
     版本」直通链。修复谓词跑在迁移后表示上，修复动作仍落在源世代表示。
  3. **语义修正**：存储版本比已装 dsh 新 / 迁移器拒绝 / fallback 遇 vN → 一律
     `unknown` + 升级提示（unsupportedVersion 标记），**不再误归 needs_repair**
     （与 dsh `refuseForeignFormatVersion` 同语义；修复入口对该类直接如实拒绝）。
  4. **世代文件族发现**：脚本 `findSessionFiles` 与 Rust `scan_sessions` 均识别
     `session.vN.jsonl[.zstd]`；同目录多世代只保留 dsh 实际读取的最高世代
    （列表单条目，修复入口不打在 dsh 不读的文件上）。
- 影响：dsh 升级 0.1.3 后存量 v0 会话不再被误判（迁移管线校验，闸门语义不变
  「通过 = dsh 一定能加载」）；本脚本无需随 dsh 升级改代码。**遗留一项升级日
  实测**：修复 v0 源后已发布 v2 世代的失效/重发布机制（generation.ts 源身份
  守卫）需装上 0.1.3 后实证；当前口径 = 各世代文件独立校验/修复。仅周知。
- 凭据：cargo test 156 全绿（+3：世代文件名解析、同目录最高世代选择、v2→
  unknown 分类回归）+ fmt + clippy；catalog 分支经高保真 stub 管道端到端验证
  （API 契约逐字锚 0.1.3-alpha.1 源码）——真实语料 9 会话双引擎路径全绿、
  原始 8650d6f2 损坏快照经 catalog 分支检出并修复、手造 v2 文件三模式
  （真引擎/有 catalog/fallback）一致归 unknown。

### 2026-09-07 会话维护 · 健康检测对齐 dsh 本尊恢复校验——surface 悬空 replace 可检可修 —— guan（AI 协作）

- 变更（`scripts/repair-session.mjs` + `src-tauri/src/sessions.rs`；本条与该改动同 commit 落盘）：
  1. **根因**：`session-8650d6f2` 为历代修复脚本的「重编号嵌合体」——压缩摘要
    （`user/message` + `surfaceOp.replace`）的 start/end 仍指向重编号前的旧 seq，
    dsh 打开即 `invalid seed event at index 37: surface replace: end seq 1686 not
    found in surface`；旧健康检查只验存储层 seq 连续性，对此类完全失明（第 4 类损坏）。
  2. **健康判定升级为两层**：存储层（原有 seq/出处链形检查）+ 恢复层（新增）。
    恢复层优先动态 import 引擎档 `@deepseek-ai/dsh-session` 本尊（与实际加载该
    会话的 dsh 同版本），按 `prepareCore` 全链复刻：词汇表闸门 → `adoptSessionEvent`
    → `interruptedTurnClosers` 补尾 → `Session.fromRestore`（含 surface fold 全量
    重放）——本层通过 = dsh 一定能加载。引擎缺包降级内置 fold 移植（锚已安装
    v0.1.2-rc.1 的 `surface.ts`/`index.ts`；注意 0.1.2 允许 assistant/message 携带
    出处链，与仓库 HEAD 0.1.3 规则不同，勿以后者为锚）。
  3. **修复策略（第 4 类）**：二分定位首个坏事件 → 最小变异（悬空 replace 转
    append、剥离失效 `sourceEventSeqs`；绝不重编号/不删事件/不改内容）→ 写盘前
    对产物字节再过「存储层 + 恢复层」双闸门，任一失败放弃写入。Rust 侧向脚本
    传 `DSH_DOCK_ENGINES`（引擎档根，ADR-0010 资产定位）。
- 影响：真实会话 8650d6f2 已修复（6171 事件零丢失，dsh 本尊 `fromRestore` 由
  失败转通过，重新打开即可用；损坏现场快照另存 /tmp，既有 `.bak` 未覆盖）；
  「一键检测/修复」此后对 surface 类损坏有检出能力。dsh 升级无需改脚本
  （校验器运行时解析引擎档最高版本包）。仅周知，无需动作。
- 凭据：cargo test 153 全绿（含新增回归 `repair_session_heals_dangling_surface_replace`，
  CI 走 fallback 路径确定性覆盖）+ fmt + clippy；真实语料 9 会话 `--scan` 零误报
  （dsh/fallback 双模式结论一致）；8650d6f2 修复前后 `Session.fromRestore` 失败→通过。

### 2026-09-05 会话维护 UI/UX 重构 · 健康检查 + 会话名称 + 日志时区修复 —— guan（AI 协作）

- 变更（commit 9935cdd）：
  1. **健康检查**：`repair-session.mjs` 新增 `--scan` 只读模式（JSON 输出 healthy/needs_repair/unknown + 标题），Rust `scan_sessions` 经引擎 node 调用填充 status/title/healthDetail（node 缺失降级 Unknown）；`SessionItem` 新增 `title`/`healthDetail` 字段。
  2. **会话名称**：标题取自 dsh `session/title` 事件，列表以会话名称为主视觉，ID 为等宽辅助（可复制）。
  3. **SessionManager UI/UX 重构（会话探针室风格）**：健康状态色点徽标；修复按钮仅非健康会话显示；全局「一键全量体检与自愈」仅异常时可用（脚本对健康 no-op）；状态筛选（全部/仅看异常带计数）；非健康行琥珀色脉冲边条 + 异常原因。
  4. **日志时区修复**：`localizeLogTimestamp`（lib/format.ts）ISO8601 UTC → 本地时区（兼容 ANSI 转义/跨日），LogViewerPane 接入；+4 单测。
- 验证：cargo test 151 全绿 + fmt + clippy；前端 typecheck + oxlint + 104 单测；真实 `--scan` 9 会话（8 健康 1 需修复）。
- 注：同批 `lib.rs` 含先前在途 boot 改动一并提交（编译依赖）。

### 2026-09-04 会话自愈重写 · 与 dsh 加载器语义对齐的重放重叠去重修复 —— guan（AI 协作）

- 触发：会话 `session-1214c12f`（用户会话）损坏，一键全量体检与自愈、单会话一键修复均无效（假成功）。
- 根因：dsh 0.1.2-rc.1 中断恢复后以相同 seq 重放被中断轮次真实事件，磁盘残留旧占位（`turn/end` / `session/end-seed` 等），形成「连续前缀 + 重放块」重叠；加载器 `dsh-session-persistence-jsonl` 在重叠处报 `seq gap in committed region` 并丢弃重叠点之后全部恢复事件。旧版自愈按 turn 重排 + 全量重编号 → 破坏 append-only 模型与 `sourceEventSeqs` 出处链，且失败一律 exit 0（假成功）。
- 变更（commit 2676ec4）：
  1. `scripts/repair-session.mjs` 重写：重放重叠检测与去重（丢弃被遮蔽旧事件，保留重放块，顺序与 seq 原样）；序列缺失按加载器语义截断；不可安全修复明确报错；写前备份 + 临时文件加载器语义校验 + 原子替换 + 写后竞态复查；健康文件幂等 no-op；失败退出码非 0。
  2. `src-tauri/src/sessions.rs`：临时脚本路径含 PID + 时间戳（并发不踩踏）；以脚本退出码为准；单测重写复现真实重放重叠（复现先行）+ 健康幂等测试。
- 实测：修复 session-1214c12f（11965 事件）与 session-8650d6f2（6171 事件），dsh v0.1.2-rc.1 真实 loader 语义校验通过，全量 8 会话扫描全部 OK。
- 凭据：`cargo test` 151 单测全绿 + `cargo fmt --check` + `clippy -D warnings` 零告警。

### 2026-09-04 宪法级改动 · 引擎引导链路与工作台 Token 跨源认证闭环 —— guan（AI 协作）

- 变更：
  1. **工作台 Token 认证与 Cookie 跨源直通**：针对 dsh 0.1.2-rc.1 引入的 URL `?token=...` 与 303 重定向设置 `SameSite=Strict` Cookie，在 WebKit/WKWebView 从 `tauri://localhost` 跨源导航时丢弃 Cookie 导致 401（`dsh web authentication required`）的问题，在 `src-tauri/src/lib.rs` 落地 `authenticate_workbench_session`：壳侧先经本地 HTTP 兑换 token 并将 `SameSite=Lax` Cookie 直接注入 WebView 原生 CookieStore，直达无参工作台根路径。
  2. **pnpm shim 调度器脱落修复**：pnpm v12 `shim add node` 生成的调度器硬链接在无 `devEngines` 依赖子目录（如 protobufjs/koffi postinstall）报错 `ERR_PNPM_SHIM_NO_TARGET`；在 `src-tauri/src/engines.rs` 与 `src-tauri/src/executor.rs` 增加真实 Node 二进制链接，确保生命周期脚本与客体引导平滑执行。
  3. **DSH 目标版本过滤放宽**：在 `src-tauri/src/updates.rs` 实现 `is_acceptable_dsh_version`，严格放行稳定版与 `-rc` 候选版本（排除 `alpha`），解决官方 registry 仅含 rc 版本时报无可用稳定版的阻塞。
  4. **启动初期事件竞态兜底（触 AGENTS §7）**：新增 `get_boot_status` IPC 命令并在 `ShellState` 缓存 step/error；前端 `BootIndex.tsx` 挂载时水合播种，根治早期错误卡无法呈现的竞态。
  5. **资源打包配置补齐**：`src-tauri/tauri.conf.json` 补齐 `resources/pnpm/**/*`，`lib.rs` 的 dev 模式解析优先回退源码树。
- 影响：**触宪法级**——`AGENTS.md` §7 增加 `get_boot_status` IPC 登记；全平台本地与 WSL 工作台启动认证与安装全链路畅通。
- 凭据：Rust 侧 `cargo test` 150 单测全绿（+2：`acceptable_dsh_version_accepts_stable_and_rc_rejects_alpha` / `cookie_parsing_adjusts_samesite_and_domain`）+ `cargo fmt --check` + `clippy -D warnings` 零告警；前端 `tsc` + `oxlint` + 15 文件 100 题全绿。

### 2026-09-04 完成通知 · 任务 G 启动页重构（对齐引擎倒置叙事 + 仪表盘级控制台视觉） —— guan（AI 协作）

- 占用声明：前端启动页组件与文案改动经维护者会话内指示（沿 P3-b 先例）。
- 变更：
  1. **文案全面对齐 ADR-0010 引擎倒置契约**：`zh-CN.ts` 与 `en-US.ts` 双语同步。启动步骤锚定后端实际序号（步骤 0 环境检测 / 步骤 1 准备引擎 / 步骤 2 启动工作台 / 步骤 3 等待就绪 / 步骤 4 进入工作台），剔除旧系统探测与 TooOld 叙事，模式说明全面更新为「基于应用内置引擎，首启自动引导 Node 与 DSH；就绪后完全离线运行」。
  2. **启动控制台流水线视觉（BootTimeline & BootStep）**：引入竖向一体化流水线导轨，5 步骤状态高保真渲染（Check/Spinner/Alert/Mono 数字），后端遥测 detail 升格为等宽代码徽标，进度芯片与状态点实时动态感知。
  3. **引擎引导专属卡片（DownloadProgress）**：下载主角位升级为自包含引擎引导面板，呈现实时传输速率与动态 ETA，结合两段式渐变进度条、SHA-256 完整性校验与离线直通提示。
  4. **顶栏与环境光晕质感精修（BootIndex & BootSelector & VersionChip）**：微光环境渐变光晕配合浮动 Emblem，顶栏统一轻量级品牌标识与精细化版本芯片，保留拖拽区与非阻断更新条能力。
- 影响：普通区前端组件与文案，后端契约与 IPC 零改动。
- 凭据：前端全量闸门通过：`pnpm run typecheck`（TS 7.0.2 绿）+ `pnpm run lint`（Oxlint 20ms 绿）+ `pnpm run test`（15 文件 100 单测全绿）+ Rust 侧 `cargo test` 148 单测全绿。

### 2026-09-04 宪法级改动 · 统一日志封装 lib/logger 与 console.* 全量收口（触 AGENTS §4.3） —— guan（AI 协作）

- 占用声明：经维护者会话内指示「统一日志封装标准」视同声明。
- 变更：
  1. **前端统一日志封装**：落地 `frontend/src/lib/logger.ts`，规范格式为 `[模块名] 行为描述 { 上下文参数 }`；debug 仅在 dev 构建输出，warn/error 全环境保留；明确保密合规（绝不打印密钥/Token/密码/PII）与防循环轰炸纪律。
  2. **console.* 全量收口**：存量 `console.*` 调用（clientUpdateStore 状态迁移告警、QuickDshSwitcher 聚焦失败）全部收拢至 logger。
  3. **单测覆盖**：`logger.test.ts` 落地（级别路由、空上下文省略、debug 门控三例）。
  4. **宪法 AGENTS.md §4.3 同步**：登记前端统一日志规范与口径。
  5. **Rust 侧审计**：`src-tauri/src/` 零残留 `println!`/`eprintln!`/`dbg!`，已全面落地 tracing 分级与结构化字段。
- 影响：**触宪法级 AGENTS.md §4.3**（日志统一 lib/logger，禁止直接调用 console.*）。
- 凭据：前端 typecheck + lint + 100 测试通过，Rust cargo clippy + cargo test 148 通过。

### 2026-09-04 宪法级改动 · 升级呈现与忽略版本记忆（触 AGENTS §6 持久化例外册） —— guan（AI 协作）

- 占用声明：经维护者会话内指示完成 ADR-0010 台账「升级呈现」增量。
- 变更：
  1. **窗口内非阻断升级提示条**：新增 `UpdateBanner` 组件（挂载于启动页 header 下方浮层），当 dsh 或桌面客户端有新版本时提示版本信息与升级后果文案，引导进入更新中心。
  2. **忽略版本偏好持久化**：新增 `settings.dismissedUpdate`（形如 `dsh@1.6.0`），点击「忽略此版本」后原子写回 `settings.json`，同一版本不再弹窗，新版本发布时自动恢复提示。
  3. **判定逻辑纯函数测试**：`updateBanner.ts` 抽离纯函数，Vitest 5 个测试覆盖全部状态分支。
- 影响：**触宪法级 AGENTS.md §6**（持久化例外册新增 `dismissedUpdate` 字段）。
- 凭据：Rust 148 测试 + 前端 97 测试通过，双侧闸门全绿。

### 2026-09-04 完成通知 · WSL 客体 glibc pnpm 投递与客体引擎链（ADR-0010 台账） —— guan（AI 协作）

- 占用声明：经维护者会话内指示完成客体投递增量。
- 变更：
  1. **客体引擎链五态探测**：GUEST_PROBE 升级为 `GUEST_MUSL / PNPM_MISSING / NODE_MISSING / DSH_MISSING / READY`，musl 系（Alpine）明确报出可行动错误。
  2. **glibc pnpm 投递**：Windows 宿主内置打包 `pnpm-linux-x64.tgz`，通过 `\\wsl$` 拷贝为主通道（体积复核），base64 stdin 为兜底通道，客体内 tar 解包落位至 `~/.dsh-dock/engines/bin`。
  3. **客体网络与执行**：在客体进程内通过 pnpm 执行 `runtime set node` 与 `pnpm add -g dsh`，网络发生在客体环境内，镜像链保持注入。旧 curl-tarball node 链路退役。
  4. **构建流水线支持**：Windows runner 构建前同时拉取 linux-x64 离线捆绑包。
- 影响：Windows 平台 WSL2 模式引擎自动化闭环，客体与宿主同享自包含引擎策略。
- 凭据：模板全量 bash 实跑测试（五态链序/prep前置/stage落位/RFC向量/UNC路径），Rust 148 测试全绿。

### 2026-09-04 宪法级改动 · 探测层退役与引擎档唯一来源（触 AGENTS §2、§7） —— guan（AI 协作）

- 占用声明：经维护者会话内指示完成 ADR-0010 核心重构（探测层退役）。
- 变更：
  1. **系统探测全量删除（-2799 行）**：删除 `login_shell_path`、固定目录扫描、fnm/nvm 查找、pnpm global 扫描等历史探测代码；`effective_path` 收缩为纯环境变量 PATH；`resolve_launch` 仅认 Engine/Bundle 档。
  2. **工具链收敛**：`engines.rs` 与 `plugins.rs` 仅保留引擎档，双缺直接出可行动错误，不再混搭系统环境。
  3. **诊断与更新收拢**：系统诊断返回引擎四件套状态，更新升级链路完全限定在引擎目录内。
  4. **宪法条款修订**：AGENTS.md §2 明确 `resources/pnpm/` 永不入库；§7 boot 期 `npm i -g pnpm` 条目标注退役。
- 影响：**触宪法级 AGENTS.md §2 与 §7**。用户机器安装的 Node/dsh 版本彻底与壳运行时解耦。
- 凭据：`cargo test` 147 绿（已清理退役机制测试），fmt 与 clippy -D warnings 零告警。

### 2026-09-04 宪法级改动 · 前端工具链全面 Native 化（pnpm 12 + TypeScript 7 + Oxlint + Lucide 1.40） —— guan（AI 协作）

- 变更：
  1. **包管理器全面倒置对齐 pnpm 12**：前端包管理由 `npm` 切换至 `pnpm 12`（锁定 `pnpm@12.3.1`，与壳内置引擎引导器版本单一真相源严格对齐）；生成 `frontend/pnpm-lock.yaml`；配置 `shared-workspace-lockfile=false` 隔离父仓库 monorepo 漂移；移除旧 `package-lock.json`。
  2. **TypeScript 7 + Oxlint 极速静态检查**：升级 TypeScript 至 `~7.0.2`（Go 原生编译器引擎）；废弃传统 ESLint 全家桶（移除 `eslint.config.js`），切换至 Rust 内核的 `oxlint`（`oxlint src` 耗时由 ~1.5s 骤降至 21ms，0 告警 0 错误）；`lucide-react` 平滑升级至 `^1.40.0`。
  3. **Tauri 与 CI 流水线同步升级**：`src-tauri/tauri.conf.json` 的 `beforeDevCommand` 与 `beforeBuildCommand` 统一切换为 `pnpm`；`.github/workflows/build.yml` 引入 `pnpm/action-setup@v4`（三平台统一预装 pnpm 12.3.1，自动接入 pnpm store 依赖缓存，Linux/macOS/Windows 构建闸门全闭环）。
  4. **宪法 AGENTS.md §1 对齐**：前端开发与质量闸门命令同步修订为 `cd frontend && pnpm install --frozen-lockfile && pnpm run typecheck/lint/test`。
- 影响：**宪法级**——前端构建链、锁文件与 CI 闸门切换至 pnpm 12。协作者需使用 pnpm 12 执行前端依赖安装。
- 凭据：全量质量闸门实测验证通过：`pnpm install --frozen-lockfile`（92ms）+ `pnpm run typecheck`（TS 7.0.2 绿）+ `pnpm run lint`（Oxlint 22ms 绿）+ `pnpm run test`（15 文件 100 单测全绿）+ `pnpm run build`（356ms 成功打包）+ Rust 侧 `cargo test` 148 单测全绿。

### 2026-09-04 完成通知 · P3-b boot 接线 + contract v3（MANIFEST_FORMAT=3，引擎档缺省） —— guan（AI 协作）

- 占用声明：lib.rs（共享区）改动经维护者会话内指示「继续」视同声明（2026-09-04，本条目即落档）。
- 变更：
  1. **contract v3 落地**（docs/contract.md「运行时策略 v3」同日实现）：壳
     `MANIFEST_FORMAT=3`，`TierKind::Engine` 新档位；manifest 加载统一规范化为
     `tiers ∈ {[Engine], [Bundle]}` + fallback——v3 快照档（snapshot 三件套）与
     v1/v2 兼容迁移（fallback→快照档、极简在线档→引擎档；resolution 档序语义废止）。
  2. **boot 接线**：resolve_launch 引擎档臂 = ensure_engine_bootstrapped → 引擎
     LaunchSpec；executor 引擎档跳过 pnpm 补齐（捆绑 pnpm 随 boot 重铺恒在）；
     lib.rs 去 engines 模块 allow(dead_code)。
  3. **离线语义收口**：engines::bootstrap 版本解析改**惰性闭包**——node 已装但
     解析失败（离线且无缓存）→ 警告后用已装版本继续；dsh 已装永不查 dist-tags
     → 就绪引擎离线 boot 零网络（契约「之后 registry 不可达 → 已装引擎直接启动」）。
  4. **执行形态**：LaunchSpec.dsh_entry = DshEntry（NodeScript | Launcher）——
     引擎档 dsh 启动器直接执行（spawn_dsh / no-open 探测共用，探测缓存机制不变）。
  5. **打包内置 pnpm**（边界 A）：新增 scripts/fetch-pnpm-bundle.sh（版本从
     updates.rs PINNED_PNPM_VERSION 推导防漂移；npmmirror→npmjs 镜像链 +
     packument dist.shasum 完整性校验；实测 darwin-arm64 16.8MB 落位）+
     build.yml 三平台构建前取件步骤 + resources/pnpm/ 入 .gitignore（永不入库）+
     render-product.sh 升 v3（snapshot 三件套，打包侧同步完成）+ 本仓 manifest
     升 v3 引擎档缺省。
- 影响：**引擎档自此为产品缺省形态**——下一发版起 boot 走壳引擎引导（首启需
  联网），用户全局 dsh/node 与启动解耦（TooOld 死局消失）。本仓 dev/CI 即时生效
  （resources manifest 已 v3）；本机冒烟 = `tauri dev` 走引擎档全链路。
  探测层退役、创建链切引擎档、WSL 客体投递、升级呈现为后续刀。
- 凭据：`cargo test` 197 全绿（+4：manifest v3 迁移×2/未知 mode 拒绝/引擎档
  LaunchSpec 离线构造/bootstrap 离线降级）+ fmt + clippy -D warnings 零告警；
  fetch 脚本本机实测通过（npmmirror 命中 + sha1 比对）。

### 2026-09-04 完成通知 · P3 插件操作改引擎档（工具链解析引擎优先 + 转发链统一内核） —— guan（AI 协作）

- 变更：
  1. `plugins.rs` 工具链解析 `resolve_toolchain`：引擎档优先——engines/bin 内
     node shim 与 dsh 全局启动器**双全**才选引擎（半就绪不混搭，整体回退）；
     引擎未就绪回退系统探测。移除「未检出系统 Node/系统 dsh」硬失败——引擎
     就绪后系统安装不再是插件操作的前置条件（P3-b 接线前引擎恒空，行为等价旧系统档）。
  2. 引擎档执行：dsh 启动器（pnpm 全局 shim）直接执行——Unix shebang 脚本 /
     Windows .cmd（child_cmd 吸收），node/pnpm 经 PATH 解析，不再深挖 pnpm
     全局树取 lib/bin.js；`engines.rs` 增 `engine_node_bin` / `engine_dsh_bin` 定位入口。
  3. `profiles.rs` 转发链统一内核 `run_dsh_forward`（program + prepend + child_path）：
     `run_dsh_plugin` 保持原行为成为系统档薄封装（创建链零改动，随 P3-b 一并切）。
  4. 顺带闭合系统档隐患：插件操作的 spawn PATH 改 `dsh_child_path`（引擎 bin →
     node bin → 用户 PATH），与 ensure_pnpm 可见性基准严格同源——原实现补齐到
     引擎目录的 pnpm 在 spawn 时不可见，dsh 内部 spawnSync("pnpm") 会 ENOENT。
- 影响：普通区三文件（engines / profiles / plugins），lib.rs / IPC / 网络面零改动；
  引擎档暂未激活（boot 接线前 engines/bin 为空），现有用户行为不变。
- 凭据：`cargo test` 193 全绿（+3：引擎优先选择 / 系统回退 fixture / 双缺可行动
  错误）+ `cargo fmt --check` + `clippy -D warnings` 零告警。

### 2026-09-04 完成通知 · Apple 凭据全量轮换与分发链路验收（CI 四处修复 + rc.4 试发回退） —— guan（AI 协作）

- 变更：
  1. **凭据轮换**：Developer ID Application 证书重签（openssl CSR 路线——旧证书私钥卡在
     数据保护钥匙串无法导出 p12，新路线私钥文件化、p12 由 key+cer 直接合成）；
     App Store Connect API Key 换新（`925T697654`）；GitHub 六个 `APPLE_*` secrets 同步
     更新（gh 客户端加密直传，密钥不落会话）。
  2. **CI 四处修复**（build.yml）：证书 CN 推导截断逗号尾巴（原实现连带 OU/O/C，与
     Tauri 的 p12 裸 CN 严格比对必挂）；tag 版本校验放行 `-rc.N` 预发布后缀；release
     守卫 `rg`→`grep`（runner 镜像无 ripgrep）；草稿识别改走 releases 列表过滤
     （`/releases/tags/{tag}` 对 draft 恒 404）。
  3. **rc.4 试发与回退**：v0.9.4-rc.4 全绿发布并完成验收（公证 Accepted id 994e99ee，
     CI runner Gatekeeper 实测 `source=Notarized Developer ID`），验收后按裁定删除
     release 与 tag，`/releases/latest` 回落 v0.9.3，更新链路恢复原状。
- 影响：分发链路（签名+公证）恢复健康；Spike ③ 挂账的「公证链路权威验证」正式闭环。
  本机 Gatekeeper 处于关闭态（`spctl` accepted 不算数），验收以 CI runner 为准。
  遗留优化：dmg/app staple 票据缺失（v0.9.3 起既有行为，在线校验不受影响，仅离线
  首启需要）。v0.9.4 正式版待发。
- 凭据：run 33835515772 全绿（三平台 build + release）；notarytool history 两条
  Accepted 可查；下载产物 codesign 有效、公证记录在案。

### 2026-09-04 完成通知 · P3-a 引擎编排模块落地（engines.rs + updates 引导入口） —— guan（AI 协作）

- 变更：
  1. 新增 `src-tauri/src/engines.rs`（引擎编排，ADR-0010 主体第一件）：单目录布局
     （PNPM_HOME = engines/）· pnpm 子进程 env（PNPM_HOME + 引擎 bin 前置 PATH）·
     node 镜像 env 注入（键=release）· 非 TTY 进度行解析（映射 boot:progress）·
     就绪判定（三件齐验版本，v 前缀归一）· 幂等引导四步（捆绑 pnpm 重铺 →
     `runtime set node` 镜像链重试 → `shim add node` → `add -g dsh` registry 链
     重试）；失败语义 = 离线可启动（缺件补不齐才 Err，首启必须联网）。
  2. `updates.rs` 引擎引导唯一入口（AGENTS §7「引擎引导」）：`ensure_engine_bootstrapped`
     （node 版本取 node-map、dsh 取最新**稳定版**并排除预发布）+ 内置 pnpm tgz
     命名契约（resources/pnpm/<平台>.tgz，边界 A 压缩存储，系统 tar 解包零新增
     依赖）；boot 接线随 P3-b，新入口暂标注 allow(dead_code)。
- 影响：纯新增，不改变现有 boot 行为（P3-b 接线前 engine 路径不激活）。
- 凭据：`cargo test` 190 全绿（+8：进度行 spike 实测格式 / 镜像 env JSON 形状 /
  env 次序 / 就绪判定 v 归一 / tar 解包落位 / 假体探测 / 幂等零网络路径）+
  `cargo fmt --check` + `clippy -D warnings` 零告警。

### 2026-09-03 完成通知 · Spike 0003 实机闭环 + P2 引擎私有 pnpm 与子进程 PATH 自构 —— guan（AI 协作）

- 变更：
  1. **Spike 0003**（`docs/spikes/0003-pnpm12-engine-bootstrap.md`）：ADR-0010
     Spike①② 的 macOS 侧实机闭环——镜像注入通道实锤（`PNPM_CONFIG_NODE_DOWNLOAD_MIRRORS`，
     JSON 键 = 发布通道 `release` 等，缺键静默回退默认源；SHASUMS256 强制且与镜像同源）、
     `runtime set node` 非 TTY 全绿（字节进度行可解析、npm/npx/corepack 缺位、
     单目录引擎布局成立）、引擎链 e2e（镜像装 node → 引擎 pnpm add -g dsh →
     引擎 node 执行 dsh）。
  2. **P2**：pnpm 补齐落 `<数据目录>/engines/npm`（替代用户 npm 全局；bundle 档
     只读 resources 亦因此可补齐）+ 显式 pin `pnpm@12.3.1`（`latest` dist-tag 实测
     仍指 v11）；子进程 PATH 自构第一步 `dsh_child_path`（引擎 bin → node bin →
     用户 PATH），ensure_pnpm 可见性检查与 dsh spawn 同源。
- 影响：boot 行为变化——pnpm 不再写用户 npm 全局（用户已装 pnpm 仍被复用，引擎 bin
  恒优先）；dsh 子进程 PATH 前置引擎 bin。**两处待维护者裁定**（spike 0003 §4）：
  pnpm 二进制解包 32MB 触发 ADR-0010 §6 体积复审线；musl node 下载源硬编码
  unofficial-builds.nodejs.org 不可镜像注入（实测可达，暂接受）。
- 凭据：`cargo test` 182 全绿（+4：引擎 bin 平台布局 / 前置去重 / dsh_child_path
  次序 / pin 全 triplet）+ `cargo fmt --check` + `clippy -D warnings` 零告警；
  Spike 决定性证据（本地 404 服务器路由实锤）见 spike 0003 §2.4。

### 2026-09-03 完成通知 · P1 TooOld 死局过渡修复（system dsh 过低不再拒绝启动） —— guan（AI 协作）

- 变更：
  1. `resolve_launch`（ADR-0010 P1，独立先行）：system dsh 版本低于 `minVersion`
     时不再直接 bail（旧实现整个应用拒绝启动 = 死局）——记 warn 后跳过 system 档、
     按档序落 bundle/download 继续 boot，用户全局 dsh 仍不被触碰；仅当档序耗尽仍无
     宿主时，报错保留可行动文案（含实测版本与升级命令）。
- 影响：过渡期行为修复，不依赖引擎倒置落地；manifest v2 语义不变。LocalExecutor
  与 WSL 客体探测的 `resolve_launch` 调用面随此自动受益，无接口变化。
- 凭据：复现先行测试 `resolve_too_old_system_falls_to_next_tier`（过低 → 落 bundle
  档）/ `resolve_too_old_exhausted_reports_actionable_error`（档序耗尽报错不丢版本
  信息）；`cargo test` 178 全绿 + `cargo fmt --check` + `clippy -D warnings` 零告警。

### 2026-09-03 宪法级改动 · ADR-0010 引擎倒置：环境准备阶段重造（pnpm12 引导 / 探测层退役 / 升级全显式） —— guan（AI 协作）

- 变更：
  1. `docs/adr/0010-engine-inversion.md`（新建，已接受，含全节点裁定台账 §7）：壳内置
     pnpm12 为唯一引导器（node/dsh/pnpm 下载、布局、激活全委托 pnpm），探测层与自有
     下载器退役；dsh_home 保持 `~/.dsh`；node/dsh 升级统一「提示 → 用户决定 → 下次
     启动生效」；首启联网、之后离线可启动；WSL 客体投递 musl pnpm（用户零安装）。
  2. `AGENTS.md`：红线 2 修订（内置依赖唯一例外 = pnpm 引导器）· §6 例外册登记
     `engines/`、pnpm 硬依赖补齐方式改写 · §7 网络面登记「引擎引导」用途 ·
     §9 索引补 ADR-0010。
  3. `docs/contract.md`：追加「运行时策略 v3」章节（resolution/fallback 废止，
     `runtime.mode: engine` 缺省 / 声明 snapshot 三件套即快照档；`format: 3` 随 P3
     实现升版）。
  4. 新建根 `CONTEXT.md` 术语表（引擎 / 引擎档 / 快照档 / 用户世界 / 引导 / 就绪判定 /
     引擎目录）。
- 影响：**宪法级**——红线 2 生效文本变化；装配方与壳按 contract v3 对接（P3 落地前
  v2 语义仍有效）。实现排期：Spike 实机验证 → P1（TooOld 死局过渡修复，可独立先行）
  → P2（子进程环境自构）→ P3（倒置落地）。
- 凭据：纯文档改动不触运行时；pnpm12 事实核查锚定 v12.3.1 源码（镜像注入通道 /
  CANNOT_MANAGE_NODE 判定 / 两段式安装 / 进度输出）；环境准备阶段全节点经 grilling
  三轮逐项裁定（台账见 ADR §7）。

### 2026-09-01 发版通知 · v0.9.4 Windows/WSL 启动体验优化与 WSL Node 自动补齐体系 —— guan（AI 协作）

- 变更：
  1. **WSL 客体内 Node.js 全自动补齐**：落地 ADR-0004 §7 裁定，通过 npmmirror 镜像链在客体内自动拉取并解压官方 tarball 至 `~/.dsh-dock/node`，`guest_prep!` 宏优先将其注入 PATH，全自动串联 `node -> dsh` 安装链。
  2. **启动准备阶段顶栏精简（方案 b）**：环境准备与宿主解析阶段精简 Header，隐藏未就绪的控制中心入口与品牌文字，保留透明可拖拽区域，就绪后恢复控制中心入口。
  3. **首次环境准备等待体验升级**：下载达到 100% 后平滑淡出，细化宿主解析、依赖准备与包管理器补齐阶段的状态提示，消除静默等待卡顿感。
  4. **模式切换双向感知**：启动页模式切换按钮动态感知当前环境，WSL 模式显示「在本机中打开」并调用 `chooseMode("local")`，Local 模式显示「在 WSL 中打开」，中英文字典同步更新。
  5. **控制中心窗口 WebView2 背景色防护**：注入统一背景色配置，消除 Win32 HWND 创建时 WebView2 异步加载白屏闪烁。
- 影响：仅周知。v0.9.4 发版完成并打 tag。
- 凭据：`cargo test` 176 个单测全绿（含客体 Node 探测新单测）+ `cargo fmt --check` + `cargo clippy -- -D warnings` 零告警通过。

### 2026-09-01 发版通知 · v0.9.3 主工作台与控制中心双向快捷切换胶囊体系 —— guan（AI 协作）

- 变更：
  1. **双向快速切换入口（零遮挡架构）**：
     - DSH 主工作台注入胶囊：`top: 8px; left: 50%` 居于顶部开阔空白区，支持鼠标自由拖拽；
     - 控制中心顶栏胶囊：`QuickDshSwitcher` 内嵌于 Header 工具区，彻底移除底部悬浮层；
     - 两端统一交互：平时仅展示极简文本，鼠标 Hover 时平滑单行展开（强制 nowrap）快捷键徽章。
  2. **一键双向 Toggle 快捷键与跨窗口事件广播**：
     - 共用一组快捷键 `⌘,`（macOS）/ `Ctrl+,`（Windows/Linux）在主工作台与控制中心之间来回 Toggle 切换；
     - 新增 `app:settings-changed` 事件广播，工作台胶囊开关（即刻挂载/卸载）与快捷键风格切换实时响应；
     - `AGENTS.md` §6、§7 规范登记 `open_profiles_window`、`focus_main_window`、`showFloatingSwitcher`、`switcherShortcut` 与 `app:settings-changed`。
  3. **控制中心窗口与偏好设置升级**：
     - 控制中心默认打开尺寸提升至 `1180x780`（开箱即为宽屏双栏工作台）；
     - 「系统控制台 -> 偏好设置」增加工作台悬浮胶囊开关与快捷键风格偏好配置。
  4. **跨平台原生适配加固**：
     - `profiles` 独立窗口注入 `platform_script`，`host.ts` 增强平台 fallback 识别，确保快捷键准确匹配操作系统按键。
- 影响：仅周知。v0.9.2 发版完成并打 tag。
- 凭据：`cargo test` 175 个单测全绿 + `cargo fmt --check`；前端 `npm run typecheck` + Vitest 92 个单测全绿。

- 变更：
  1. **插件中心统一视图**：Profile 管理器市场/总览两个 Tab 合并为「插件中心」单入口（`PluginHub.tsx` 子 Tab 承载），分类下拉改平铺标签矩阵（可展开收起），排序项矢量图标化。
  2. **跨 Profile 安装去重折叠**：导入弹窗同名插件按包名聚合 + 来源 Profile 标签切换器（`groupPickerCandidates`），独立绑定配置复制。
  3. **模型凭据别名识别**：`credentials.rs` 支持 `refs.DEEPSEEK_API_KEY` 等大写环境变量与别名映射，修复「已配置仍提示未配置」。
  4. **Release Notes 规范化体系**：新增模板与生成规范，构建期脚本优先读 `docs/RELEASE_NOTES.md`。
  5. **下线 `reset_profile_dependencies`**：IPC 登记/capabilities/前端入口三处同步清理，依赖自愈回归 pnpm 自动补齐。
  6. **i18n 与设计规范对齐**：组件全面切换 `useI18n()`；清理非标准 emoji 与冗余文案；悬浮提示补齐；Profile 彩色确定性标签。
- 影响：仅周知。v0.9.2 已打 tag，进入冻结期（master 只收 fix）。
- 凭据：`cargo test` 175 个单测全绿 + `clippy -D warnings` 零告警 + `cargo fmt --check`；前端 `typecheck`/`lint`/92 单测全绿。

### 2026-09-01 完成通知 · 启动环境检查提速（--no-open 探测缓存 + 流式早退 + 超时兜底） —— guan（AI 协作）

- 变更：`src-tauri/src/resolve.rs` 环境检查链路优化 + `AGENTS.md` §6 例外册登记：
  1. **跨启动持久化缓存**：`--no-open` 能力探测结果（dsh 版本 → bool）落 `<data_dir>/probe-cache.json`
     （原子写、损坏/缺失回退探测、不阻断 boot），稳态启动零 spawn（实测原 8~11s → 缓存命中 ≈0）。
  2. **流式读数早退**：`probe_no_open` 从 `output()` 等自然退出改为双线程读 stdout/stderr
     （usage 可能打到任一流），命中 `--no-open` 即 kill 早退——缓存 miss 时 1.4s 返回（原 8~11s）。
  3. **硬超时兜底**：20s 探测总超时（原无超时，探测进程卡死会永久阻塞 boot）；超时按
     「不支持」处理（不传 `--no-open`，保持旧版 dsh 秒退防护默认值）。
  4. 新增 8 个单测（缓存命中零 spawn / miss 写回 / 流式早退 / stderr 命中 / 卡死超时 /
     版本变化失效 / 缓存损坏回退 / 往返），全仓 `cargo test` 175 passed，`clippy -D warnings` 通过。
  5. `AGENTS.md` §6 运行时持久化例外册登记 `probe-cache.json`（可丢失可重建缓存）。
- 影响：仅周知。日常启动环境检查从 ~8~11s 降至 ~0.3s（仅剩 PATH/node 探测子进程开销）；
  探测进程卡死不再阻塞启动。
- 凭据：本机实测（macOS + fnm node v24.18.0 / dsh 0.1.1-rc.2）：旧实现 `--help` 探测 7~11s、
  新流式早退 1.3s；`cargo test` 175 全绿 + `cargo clippy --all-targets -- -D warnings` 零告警。

### 2026-08-31 发版通知 · v0.9.1 社区插件市场全量上线（awesome-dsh-plugin Registry 2700+ 插件发现与一键安装分发） —— guan（AI 协作）

- 变更：
  1. **社区插件市场（Registry 集成）**：`src-tauri/src/updates.rs` 接入 `awesome-dsh-plugin.com/plugins.json` 官方 Registry（单一网络面，带 3MB 上限与 GitHub raw 镜像兜底），新增 `fetch_market_registry` IPC 命令与外链域名白名单登记。
  2. **高质感插件市场工作台（`/frontend-design`）**：`ProfileManager.tsx` 新增第 5 个视图 Tab（`market`），引入 `MarketplaceView.tsx`、`MarketPluginCard.tsx` 与 `MarketInstallDialog.tsx`。
  3. **丰富发现与筛选能力**：支持全局搜索（名称/NPM/作者/描述）、22 个垂直分类快速筛选 Chips、四向排序（Stars ⭐ / Downloads ⬇️ / 最新 🆕 / 名称 🔤）与已安装插件过滤。
  4. **全域状态联动与一键分发**：插件卡片与全域已装 Profile 芯片实时联动（带绿色脉冲圆点），支持一键安装到指定 Profile 或分发至其他 Profile。
  5. **国际化与单测补齐**：中英多语言字典全量支持；新增 `market.test.ts` 14 个单测，全仓 166 Rust 单测 + 85 前端单测全绿。
- 影响：仅周知。用户可在 DSH Dock 内直接浏览、检索与安装 2700+ 社区插件。
- 凭据：`cargo test` 166 个单测全绿；前端 `npm run typecheck && npm run lint && npm run test && npm run build` 全量通过。

### 2026-08-31 完成通知 · 问题 9 & 10 深度优化（本地路径打开修复 + 系统控制台极简侧栏导航重构） —— guan（AI 协作）

- 变更：
  1. **问题 9（本地路径打开）**：`src-tauri/src/lib.rs` 升级 `open_external` 命令，判断参数为本地文件系统路径时调用系统文件管理器（Finder / File Explorer）直接打开该路径或其父目录，彻底解决之前被 Web URL 白名单拦截报错「不允许的外链」的问题。
  2. **问题 10（两层导航极简重构）**：严格遵循 `/frontend-design` 规范对 [`SystemConsole.tsx`](file:///Users/guan/git/realguan/dsh-plugin-hub/dsh-dock/frontend/src/components/system/SystemConsole.tsx) 进行极简与去噪重构：彻底移除左侧顶部冗余啰嗦的长句描述，将导航项升级为极简高级的轻量侧栏（单行微底色 Icon + 纯粹标题 + Active 边框态），消除多余边框包裹与视觉杂讯，与整体界面实现极致协调统一。
- 影响：仅周知。会话维护与系统控制台操作体验与视觉质感显著提升。
- 凭据：`cargo test` 166 个单测全绿；前端 `npm run typecheck && npm run test` 12 个测试套件 71 个单测全绿。

### 2026-08-31 完成通知 · 自动化发布日志流水线（GitHub Release & 客户端「关于」更新说明无缝打通） —— guan（AI 协作）

- 变更：全链路打通 GitHub Release 与桌面客户端自更新日志：
  1. **自动化发布日志提取引擎**：新增 [`scripts/extract-release-notes.py`](file:///Users/guan/git/realguan/dsh-plugin-hub/dsh-dock/scripts/extract-release-notes.py)，在打 tag 发布时自动从 `docs/broadcasts.md` 提取匹配当前版本的权威变更条目（包含变更要点、架构决议与测试凭据），并支持回退到 `git log`。
  2. **GitHub Actions 构建流无缝注入**：重构 [`.github/workflows/build.yml`](file:///Users/guan/git/realguan/dsh-plugin-hub/dsh-dock/.github/workflows/build.yml) 中的 `release` 任务，自动将提取出的 Markdown 升级日志写入 GitHub Release `body_path`，并注入到桌面自更新清单 `latest.json` 的 `notes` 字段中（彻底解决之前固定硬编码 `"DSH Dock v0.x"` 导致客户端无更新日志的缺陷）。
  3. **客户端「关于」更新卡片体验增强**：[`ClientUpdateCard.tsx`](file:///Users/guan/git/realguan/dsh-plugin-hub/dsh-dock/frontend/src/components/about/ClientUpdateCard.tsx) 升级 Release Notes 视窗，支持滚动阅读与一键「展开全部 / 收起」切换，让用户在应用内检查更新时能够清晰浏览完整的更新内容。
- 影响：仅周知。后续所有通过 git tag 触发的 CI 发布将全自动生成详尽的 GitHub Release 页面，且客户端关于窗口在检查到新版本时能完整呈现版本升级日志。
- 凭据：本地执行 `python3 scripts/extract-release-notes.py` 验证通过；`cargo test` 166 个单测全绿；前端 `npm run typecheck && npm run test` 12 个测试套件 71 个单测全绿。

### 2026-08-31 完成通知 · 8 项深度用户体验与逻辑缺陷全量优化（Bento 宫格插件市场、智能贪婪路径反解、凭据元数据过滤、诊断大盘缓存、命名升级控制中心） —— guan（AI 协作）

- 变更：全面落实 `问题记录.md` 中的 8 大反馈与优化建议：
  1. **新建 Profile 按钮布局**：侧边栏顶部独立设计高亮醒目的 `+ 新建工作台` 主操作按钮，与下方搜索框形成清晰主次操作流。
  2. **重置依赖感知增强**：明确弹窗文案（彻底清理 `node_modules` 并基于 `package.json` 通过 pnpm 纯净重装），执行态展示 Spinner 与防重点击，Toast 明确返回重置结果。
  3. **插件分发 Select 截断修复**：消除 Radix `SelectTrigger` 嵌套 span 截断问题，设置 `min-w-[240px]` 完整展示 Profile 名称。
  4. **插件总览 Bento 宫格卡片 + 分页 + Profile 筛选**：`PluginOverview.tsx` 重构为现代自适应 Bento Grid 宫格卡片，新增 Profile 专属下拉筛选器与分页控制器（支持 6/9/12/18 条切换与页码导航）。
  5. **会话项目分组、路径贪婪反解与 Icon 纠正**：`sessions.rs` 引入文件系统智能探测贪婪匹配算法，精准还原带连字符真实物理路径（`/Users/guan/git/realguan/dsh-plugin-hub/dsh-dock`）并提取简洁项目名（`dsh-dock`）；`SessionManager.tsx` 规整卡片上下双层结构，纠正复制会话路径 Icon 为 `Copy`。
  6. **凭据元数据过滤**：`credentials.rs` 引入 `RESERVED_METADATA_KEYS` 严格过滤 `version`、`refs`、`schema` 等非 Provider 元数据键，新增专属单测。
  7. **健康大盘缓存与 Icon 纠正**：`DiagnosticsPane.tsx` 引入 60 秒内存缓存机制实现秒开无感切换，保留手动「刷新体检」按钮；复制诊断报告按钮纠正为 `Copy` 图标。
  8. **产品命名统一升级**：程序菜单栏与独立窗口标题由「Profile 管理器」统一升级为更名副其实的「控制中心 (Control Center)」。
- 影响：仅周知。全栈用户体验与交互质感大幅提升。
- 凭据：`cargo test` 166 个单测全绿；前端 `npm run typecheck && npm run test` 12 个测试套件 71 个单测全绿，0 报错。

### 2026-08-31 完成通知 · v0.9.0 深度对齐审核报告与全量单元测试补齐（165 Rust 单测 + 71 前端单测全绿） —— guan（AI 协作）

- 变更：全量落实《DSH Dock v0.9.0 规划与可行性深度审核报告》的核心设计建议与落地红线，并补齐全部功能的边界单测：
  1. **4.5 凭据脱敏与 DSH 引擎全局设置**：Rust 新增 `dsh_settings.rs`（`settings.yaml` 安全原子读写）与 `credentials.rs`（脱敏掩码算法 `mask_api_key`、`get_credentials_summary`、`set_credential_key` 0600 权限）；前端 `CredentialsPane.tsx` 升级为结构化卡片与独立修改弹窗，新增 `DshSettingsPane.tsx` 管理 DSH 核心引擎配置。补齐掩码边界、清除/删除、多 Provider 并存单测。
  2. **4.6 会话与工作区真实路径联动**：Rust `sessions.rs` 实现 `decode_project_dir_to_path`，精准反解真实工作区绝对路径（含 Windows 盘符与 Unix 路径）；前端 `SessionManager.tsx` 支持按项目聚合分组折叠与一键在访达/资源管理器中打开项目。补齐各种异常目录名反解与序列倒退自愈重排备份单测。
  3. **4.7 MCP 服务器可视化结构化 CRUD 与运行态联动**：Rust `mcp.rs` 实现针对 `@deepseek-ai/dsh-mcp-client` 的结构化 CRUD，通过内存合并避免 Cordis Patch 覆盖非 MCP 插件条目；前端 `McpManager.tsx` 支持 GitHub/Filesystem/Postgres/Brave Search 预设一键应用、完整表单编辑与运行态 `mcp__<server>__*` 工具提取联动。补齐保留非 MCP 插件条目、更新已有 server、删除幂等单测。
  4. **4.11 / 4.12 诊断体检与多源日志流**：Rust `diagnostics.rs` 探测 Node/pnpm/dsh 运行状态并收集存储水位，提供多源日志安全尾部截断 `get_app_logs`；补齐各 source 路由与未截断/截断单测。
  5. **4.13 多语言深度对齐**：前端 `i18nStore.test.ts` 采用全量递归深度遍历断言，确保 `zh-CN.ts` 与 `en-US.ts` 所有层级 key 100% 深度对称无漏项。
- 影响：仅周知。全栈代码质量与测试覆盖率达到最高标准，保证所有新增功能与边缘场景均有机器单测严格守护。
- 凭据：`cargo test` 165 个测试全绿（165 passed, 0 failed）；前端 `npm run typecheck && npm run test` 12 个测试套件 71 个单测全绿（71 passed, 0 failed, 0 type errors）。

### 2026-08-31 完成通知 · v0.9.0 规划全量落地：稳定性守护、系统控制台与运维大盘、多语言基线、MCP 扩展与依赖重置 —— guan（AI 协作）

- 变更：全量三阶段（刀 1/2/3）落地收口：
  1. **稳定性、运维与多语言（4.11 + 4.12 + 4.13）**：Rust 接入崩溃守护熔断器 `guard_session`（60s 内 3 次崩溃触发熔断与诊断提示）；实现了 `diagnostics.rs`（系统环境健康体检、Node/pnpm/dsh 探测、存储分布递归统计、安全分页日志查看器 `get_app_logs`）；新增持久化字段 `locale` 与 `autoRestart`；前端落地完整中英双语国际化 `i18nStore` 与响应式翻译。
  2. **会话与设置大盘（4.6 + 4.5）**：实现 `credentials.rs`（`.credentials.yaml` 0600 安全权限原子写与脱敏查看）；实现 `remove_session` 物理删除会话与路径快捷复制；前端构建全新控制中心 `SystemConsole.tsx`（偏好设置、凭据安全编辑器、系统健康大盘、暗色终端日志视窗）。
  3. **MCP 生态与维护收口（4.7 + 4.11）**：实现 Profile 依赖一键重置 `reset_profile_dependencies`；前端实现 `McpManager.tsx` 可视化 MCP 服务器管理（支持 GitHub / Postgres / Brave Search / Filesystem 等预设与工具前缀一键复制）。
  - 新增 8 个 IPC 命令严格完成三处同步与 `AGENTS.md` 登记；全部通过 `gate_tests` 机器闸门。
- 影响：仅周知。v0.9.0 规划的全部目标均已高质量闭环交付，所有前端界面均严格遵循 `/frontend-design` 规范打造，兼具高质感设计与可靠健壮性。
- 凭据：Rust 侧 158 单元测试全部通过（158 passed, 0 failed）；前端 58 单元测试全部通过（58 passed, 0 failed, 0 type errors）。

### 2026-08-31 修复 · WebView 内存策略改用直接子代选择器（解决工具调用嵌套导致的大面积滚动空白与输入框悬空）—— guan（AI 协作）

- 变更：本 commit——`src-tauri/src/lib.rs`（`WEBVIEW_MEMORY_POLICY_SCRIPT` 中 CSS 规则由后代选择器 `FLOW + ' ' + ROW` 改为直接子代选择器 `FLOW + ' > ' + ROW`；`syncStreaming` 仅扫描顶层行；更新单测 `webview_memory_policy_script_contains_list_padding_defense` 断言直接子代连接符 `FLOW + ' > ' + ROW`）、`docs/adr/0002-webview-memory-policy.md`（补录直接子代修订说明）。
- 影响：仅周知。解决由于工具调用内部卡片（`ToolCallTree` 的 `callRow` / `subCalls`）带有相同的 `data-chat-anchor-key` 属性而触发多层嵌套 containment，导致 WebKit 严重虚高估算滚动高度、在内容到底后仍可下滑并露出大片空白、输入框脱离底部悬空的偶现 bug。
- 凭据：`cargo test` 152 绿 / `cargo fmt --check` / `cargo clippy -- -D warnings` 全绿；前端 test 55 绿。

### 2026-08-31 功能 · 会话自愈与健康维护面板（ProfileManager 会话工作台）—— guan（AI 协作）

- 变更：本 commit——新增 `src-tauri/src/sessions.rs`（扫描 `$DSH_HOME/sessions/`、单会话/全量自愈）、`scripts/repair-session.mjs`（会话日志 Turn 归流、seq 连续化重排与 Zstd 校验重打包）；`ipc.rs` / `lib.rs` / `capabilities/default.json`（登记 `list_sessions` / `repair_session` / `repair_all_sessions`）；`AGENTS.md` §7；`frontend/src/components/profiles/SessionManager.tsx`；`frontend/src/pages/ProfileManager.tsx`（三视图切换新增「会话维护」）；`frontend/src/content/zh-CN.ts` 与 `frontend/src/types/ipc.ts`。
- 影响：**触宪法级**——`AGENTS.md` §7 登记 3 项新 IPC 命令（IPC 三处同步严格保持一致）。不破坏 dsh 文件系统不变量，修复时自动备份原始 `.bak`，解决上游并发推流或断线重连导致的 "seq gap in committed region" / "history unavailable"。
- 凭据：`cargo test` 152 绿（新增 3 单测）/ `cargo fmt --check` / `cargo clippy -- -D warnings` 全绿；前端 typecheck 0 报错 / test 55 绿 / build 产物成功生成。

### 2026-08-31 修复 · WebView 内存策略 CSS 补齐列表内边距（解决 Paint Containment 裁切列表序号）—— guan（AI 协作）

- 变更：本 commit——`src-tauri/src/lib.rs`（`WEBVIEW_MEMORY_POLICY_SCRIPT` 常量化，注入 CSS 追加 `FLOW ol, FLOW ul { padding-left: 1.5em !important; }` 防止 `content-visibility: auto` 隐式 Paint Containment 把挂在行左侧外沿的有序/无序列表 markers 裁切截断；增加单测 `webview_memory_policy_script_contains_list_padding_defense`）、`docs/adr/0002-webview-memory-policy.md`（补录 2026-08-31 修订说明）。
- 影响：仅周知。保持长会话 WebKit 内存优化不变，彻底解决桌面端会话中有序列表序号（如 `1. 2. 3.`）只展示一半的问题。
- 凭据：`cargo test` 149 绿（+1 单测）/ `cargo fmt --check` / `cargo clippy -- -D warnings` 全绿；前端 test 55 绿。

### 2026-08-31 修复 + 发版事项 · 插件总览 UX 修正（包名展示全 / 描述降级）+ v0.8.0 tag（4.4 收口）—— guan（AI 协作）

- 变更：本 commit——PluginOverview 行布局重排（维护者实机截图反馈：包名被
  描述挤压截断、描述喧宾夺主）：包名独占整行 `break-all` 尽量展示齐全（超长
  折行不截断）、分布 chips 次行、描述降级为末行单行截断辅助信息；随后
  `chore: 版本 0.8.0` + 注解 tag `v0.8.0`，master 与 tag 推送。
- 影响：**冻结期重启**——v0.8.0 为新冻结点：Release notes 至三平台产物验收
  期间 master 只收 fix。Release notes 草稿见附录（覆盖 v0.7.0 → v0.8.0 全量：
  4.4 收口 ade90bd、升级链路事件化 83fa74b、可安装性预检 704f7ff、本 UX 修正）。
- 凭据：前端 typecheck / lint / test 55 绿（Rust 侧本批零改动，0.8.0 前全量
  147 绿见 ade90bd 凭据）。

**附录：Release notes 草稿（v0.7.0 → v0.8.0）**

```markdown
## 新增
- 插件总览：Profile 管理器页内新视图，聚合展示全部 profile 的第三方插件
  分布——包名独占整行尽量展示齐全，各 profile 实装版本一目了然（只读，
  纯本地文件扫描）
- 从其他 profile 安装插件：详情对话框「从其他导入」多选批量，默认安装来源
  同版本（装验证过能用的）；串行队列单项失败不中断，末尾汇总成败与明细
- 「连配置」可选搬移：勾选后把来源 profile 里该插件的配置行原样复制到目标
  （默认不勾；只追加不覆盖目标已有配置，ADR-0009 写入例外 #4）

## 修复
- DSH 升级「点了没反应」：升级链路全程事件反馈——真实进度与失败详情可见
- 升级前可安装性预检：目标版本依赖未完整发布时立即失败并指名缺失包，
  不再等数分钟后才失败
- 插件总览包名被描述挤压截断（包名独占整行、描述降级末行）

## 变更
- 插件管理器五个核心能力收口：清单 / 安装卸载更新 / 禁用启用 / 更新标识与
  选版本 / 跨 profile 安装
- AGENTS §7 IPC 名册 +2（list_all_plugins / copy_plugin_config）

## 已知问题
- 插件安装进度为单行 busy（pnpm 流式输出未回传）
- npm 搜索未实现（挂账）
```

### 2026-08-30 完成通知 · 4.4 收口：插件总览聚合 + 从其他 profile 安装（ADR-0009 第五次修订，patch 写入例外 #4）—— guan（AI 协作）

- 变更：本 commit——维护者经 grilling 逐条确认后把原「跨 profile 复制」重定义为
  ①**插件总览**（管理器页内切换视图，`list_all_plugins` 只读文件扫描聚合全部
  已物化 profile 的第三方插件，内置 bundle 不进聚合）与②**从其他 profile 安装**
  （详情对话框多选批量选择器，串行队列失败继续末尾汇总，版本默认来源同版本
  `pkg@ver`；「连配置」逐行可选勾选=来源 cordis.patch.yml 该插件行 id 全部条目
  原样复制，`copy_plugin_config` 只追加不覆盖——**写入例外 #4**，`PluginRowState`
  扩展 `patch_entries` 供置灰预检）。新 IPC 两枚三处同步 + AGENTS §7 登记；
  ADR-0009 第五次修订先行落档；roadmap 4.4 落地记录回写（五核心能力全齐）。
- 影响：仅周知。npm 搜索（4.4⑤）维持挂账；注意 `updates.rs` 尚有另一条工作线的
  H-1 预检未提交改动，本 commit 未包含、未触碰。
- 凭据：cargo test 147 绿（新增聚合归组 / 配置行原样复制与不覆盖 / patch 条目
  计数等 4 测试）+ fmt/clippy 干净；前端 55 测试绿（新增候选过滤与批量汇总
  纯逻辑）+ typecheck/lint 干净；IPC 一致性 gate_tests 兜底。

### 2026-08-30 排障 + 修复 · DSH 升级「点了没反应」——根因 = 不可安装的 alpha 版本 + 失败不可见；升级链路事件化 —— guan（AI 协作）

- 变更：本 commit——`lib.rs`（`terminal_action` 升级链路新增 `dsh:upgrade`
  事件 running/done/failed，failed 携带安装器完整错误链含 pnpm 输出尾部），
  `DshVersionCard`（升级 busy 改事件驱动真实时长，失败显示错误详情——原 2s
  固定假 busy，之后数分钟全程无反馈）、`ClientUpdateCard`（检查更新按钮
  busy 时禁用 + 内联转圈——原实现整组消失，无动效）。
- 影响：仅周知 + **一项裁定待议**：根因是 H-1 检查口径（排序最高，rc/预发布
  也追）会把 `0.1.2-alpha.2` 这种**依赖未发布的不可安装版本**提示为「有新版」
  （ledger 已记边界）。候选：a) 检查侧做可安装性预校验（每依赖一查，成本高）；
  b) 口径改 dist-tag 优先（推翻 H-1，需裁定）。现维持 H-1 + 失败可见。Windows
  侧注意：`install_global_dsh_with_prefix` 的回退链未变。
- 凭据：根因实机复现——shell.log 14:41-14:49 多次 `pnpm add -g
  @deepseek-ai/dsh@0.1.2-alpha.2` 双 registry 全败；手工复现同错
  （ERR_PNPM_META_FETCH_FAIL，依赖走死域名 r.cnpmjs.org）。cargo test 142 绿 /
  fmt / clippy；前端 typecheck / lint / test 49 绿。

### 2026-08-29 发版事项 · v0.7.0 tag（插件管理器全量 + Profile 重启），冻结期随验收重启 —— guan（AI 协作）

- 变更：本 commit——详情对话框行内操作防溢出修复（hover 操作组与运行徽标
  display 换位，长包名截断兜底）+ roadmap 4.4 落地记录回写；随后
  `chore: 版本 0.7.0` + 注解 tag `v0.7.0`，master 与 tag 推送。
- 影响：**冻结期重启**——v0.6.0 冻结期经维护者裁定提前开工 feat（见当日
  4.4① 批次条目），本 tag 即新的冻结点：Release notes 至三平台产物验收
  期间 master 只收 fix。Release notes 草稿见附录。
- 凭据：前端 typecheck / lint / test 49 绿；Rust 142 / fmt / clippy 全过。

**附录：Release notes 草稿（v0.6.0 → v0.7.0）**

```markdown
## 新增
- Profile 重启按钮（运行中行内，确认后同 profile 重启——插件变更借此生效）
- 插件安装 / 卸载 / 更新：详情对话框行内操作（dsh plugin 转发链，规格校验
  防参数注入，安装/卸载/更新均带「重启后生效」提示）
- 插件禁用 / 启用：cordis.patch.yml `{id, disabled}` 单键切换（ADR-0009
  第四次修订，写入例外 #3），行 id 经 dump-config 权威解析
- 插件更新标识 + 选版本更新：registry dist-tags 口径（镜像链 npmmirror →
  npmjs），版本选择弹窗标 最新/当前
- Profile 详情对话框：插件清单卡（内置/外挂、实装版本、运行态徽标、
  会话运行汇总）、多插件防溢出（对话框限高 + 分区滚动）

## 修复
- 行内操作组被卡片右缘裁切（hover 与徽标 display 换位 + 长包名截断）
- 多插件时详情对话框超出屏幕（基件无高度上限）

## 变更
- 外挂插件在徽章区与卡片区去重显示（dsh reconcile 数据模型本然双写）
- AGENTS §7 IPC 名册 +9、网络面 +2（回环运行态查询、registry 外网检查）

## 已知问题
- 安装进度为单行 busy（pnpm 流式输出未回传）
- 跨 profile 插件复制未实现（4.4 收尾项）
```

### 2026-08-29 完成通知 · 4.4④ 插件更新标识 + 选版本更新落地（registry 外网镜像链）—— guan（AI 协作）

- 变更：本 commit——`updates.rs`（`npm_packument_versions`：任意 npm 包
  packument 查询，与 dsh 版本检查同镜像链 npmmirror → npmjs / 同超时 / 同
  体积上限 + `parse_packument_versions` 纯函数 semver 升序排序 + 测试）、
  `plugins.rs`（`check_updates_blocking`：逐外挂插件查 dist-tags.latest——
  口径与 pnpm 默认安装一致，复现点 7 教训；current ≥ latest 不报；奇异名
  跳过不打 registry；`plugin_versions_blocking` 全版本降序）、`lib.rs`/
  `ipc.rs`/`capabilities`（`check_plugin_updates` / `list_plugin_versions`
  三处同步）、前端（外挂插件区「检查更新」按钮 + 行内 `0.16.1 ↑0.17.0`
  更新标识，点开版本选择弹窗——标 最新/当前，选定走既有安装链
  `pkg@version`）。
- 影响：**触宪法级**——`AGENTS.md` §7 两处：IPC 名册 +2；网络面登记新
  外网用途「插件更新检查」（registry packument，同镜像链）。本条即知会。
  语义：检查为按钮触发不自动跑（N 包串行查询，避免开窗即外网风暴）；
  dist-tag 口径意味着 dsh-base 那种「latest 停在坏版本」的包不会被误标
  升级目标之外——选版本弹窗可见全部版本自行决定。
- 凭据：cargo test 142 绿（+2 packument 解析含 semver 排序反例）/ fmt /
  clippy 全过；前端 typecheck / lint / test 49 绿。

### 2026-08-29 完成通知 · 4.4③ 禁用/启用插件落地（patch 单键切换，ADR 第四次修订实施）—— guan（AI 协作）

- 变更：本 commit——`Cargo.toml`（新增 `serde_yaml 0.9`，ADR 已裁定接受停维护
  风险，读写收敛在 `set_plugin_disabled` 单函数便于后继替换）、`plugins.rs`
  （`plugin_rows_blocking`：`dsh --profile <名> --dump-config` 行 id↔包名配对 +
  壳 toggle 态，行级扫描避开 `!!js` 标签；`set_plugin_disabled`：patch 顶层数组
  读改写——禁用置/追加 disabled 键、启用移键或整条移除，头部注释块保真，
  非顶层数组拒绝写入）、`lib.rs`/`ipc.rs`/`capabilities`（`get_plugin_rows` /
  `set_plugin_disabled` 三处同步）、前端（行内电源开关：禁用态常驻灰徽 +
  包名划线，运行徽标只对启用中插件显示；操作带「重启后生效」提示）。
- 影响：仅周知。行 id 权威来源 = dump-config 行表（一次 spawn 秒级，对话框
  打开时异步取）；重启按钮承接生效。
- 凭据：cargo test 140 绿（+3：toggle 幂等与注释保真、config 键保全、
  dump 行配对解析）/ fmt / clippy 全过；前端 typecheck / lint / test 49 绿。
  实机数据锚定见 ADR 第四次修订注。

### 2026-08-29 完成通知 · Profile 重启按钮 + 详情去重 + 禁用/启用 ADR（维护者四项指令批次 1/2）—— guan（AI 协作）

- 变更：本 commit——① 重启按钮（运行中行 RotateCw，同 profile 走切换链
  `switch_profile`，恒弹确认，弹窗文案按重启语义分叉）；④ 详情对话框去重
  （reconcile 把外挂同时写进 bundles 与 dependencies——徽章区只留层叠内置
  层，隐藏数 >0 给指引行，台账复现点 7）；③ 决策先行：**ADR-0009 第四次
  修订**——patch 写入例外 #3（`cordis.patch.yml` 的 `{id, disabled}` 单键
  切换）：行 id 不可从包名推导（实测 commandcode→`llm-commandcode`），来源
  定死 dump-config 行表；serde_yaml 0.9 读改写（注释头部保真策略）；运行中
  不热生效、重启承接；生效真相 = 壳自家 patch 条目。
- 影响：仅周知；③ 的**实现**（serde_yaml 依赖 + IPC + 行内开关 UI）随后续
  commit 落地，② 更新标识（npm registry 外网查询，§7 需新登记）排最后。
- 凭据：前端 typecheck / lint / test 49 绿；行 id 映射实测锚定（本机 web 档
  dump-config）。

### 2026-08-29 完成通知 · 4.4② 插件安装/卸载/更新落地：详情对话框行内操作 —— guan（AI 协作）

- 变更：本 commit——`plugins.rs`（`validate_plugin_spec` 纯校验：防 pnpm 旗标
  注入（前导 `-` 当参数）、控制字符/空白；scope 包名与版本段（tag/精确/^~
  区间）放行，`><` 语义区间 v1 不开 + `mutate_plugin_blocking`：`dsh plugin
  --profile <名> add/remove/update <spec>` 转发链复用创建刀基建，pnpm 防御
  补齐同源，未物化/非法名先拒不 spawn，超时同创建 600s，失败附 dsh 输出
  尾部）、`lib.rs`/`ipc.rs`/`capabilities`（`install_plugin` / `remove_plugin`
  / `update_plugin` 三条 IPC 三处同步）、前端（详情对话框区头「安装插件」
  输入行 + 行内更新/卸载 hover 操作 + busy 态 + 结果分箱展示；spec 预检
  `validatePluginSpec` 镜像后端校验 + Vitest）。
- 影响：**触宪法级**——`AGENTS.md` §7 IPC 名册 +3。本条即知会。语义边界：
  装到**运行中**的 profile 时 dsh 不热重载，壳侧成功文案带「重启后生效」；
  add 裸包名 dist-tag 坑由输入占位引导带版本段规避（复现点 7）；安装进度
  v1 为单行 busy（不订阅 pnpm 流式输出），后续独立插件管理视图再升级。
- 凭据：cargo test 137 绿（+2：spec 恶意反例集、未物化/非法名先拒不
  spawn）/ fmt / clippy 全过；前端 typecheck / lint / test 49 绿（+2 镜像
  校验）。

### 2026-08-29 完成通知 · 4.4① 插件清单落地：详情对话框插件卡 + 运行态回环快照 —— guan（AI 协作）

- 变更：本 commit——`plugins.rs`（新模块：静态清单 = bundles + dependencies +
  node_modules 已装版本/描述；运行态 = `POST /api/pluginInventory/list` 回环
  只读快照，2s 超时，Spike B 方案 + 复现点 11）、`lib.rs`/`ipc.rs`/
  `capabilities`（`list_profile_plugins` / `get_plugin_runtime` 两条 IPC 三处
  同步）、前端（详情对话框依赖区升级为插件卡：官方/第三方、已装版本、运行态
  徽标，快照按 profile 匹配合并防张冠李戴；纯函数 `runtimeChipFor` /
  `runtimeSummary` + Vitest）。
- 影响：**触宪法级**——`AGENTS.md` §7 两处：IPC 名册 +2；网络面登记新例外
  「插件运行态回环只读查询」（127.0.0.1、只读、仅活跃会话、一次性快照）。
  本条即对该宪法修订的知会。**冻结期说明**：v0.6.0 验收仍待三平台产物，
  本 feat 经维护者当日裁定提前开工（「直接开工吧」），验收并行不受影响。
- 凭据：cargo test 135 绿（+4：静态清单/非法名/信封形状/响应解析）/ fmt /
  clippy 全过；前端 typecheck / lint / test 47 绿（+7 运行态合并纯逻辑）；
  运行态获取路径已在本机运行中的 dsh 实例实机打通（Spike B 实测记录）。

### 2026-08-29 发版事项 · v0.6.0 tag 已推送，冻结期开始 —— guan（AI 协作）

- 变更：commit `5ef27ab`（`chore: 版本 0.6.0（Profile 管理器全量 + Profile 切换）`，
  bump Cargo.toml / tauri.conf.json / package.json + 双 lock）+ 注解 tag `v0.6.0`
  （指向 `5ef27ab`）；master 与 tag 已推 origin（`5cf0593..5ef27ab`）。tag `v*`
  触发 CI Release（CONTRIBUTING §8）。
- 影响：**冻结期开始**——Release notes 发出至三平台产物验收通过期间，master
  只收 fix 不收 feat。Release notes 草稿见本条附录，频道确认后随 Release 发布。
- 凭据：bump 后 cargo test 131 绿 / 前端 typecheck + test 40 绿；推送回执
  `master -> master` + `* [new tag] v0.6.0`。

**附录：Release notes 草稿（v0.5.1 → v0.6.0）**

```markdown
## 新增
- Profile 管理器（4.3 全量）：列表/详情（已物化 + 可首启模板两态合并）、
  创建（dsh plugin 转发链——零网络毫秒级，创建即 webUi 候选可设默认）、
  复制/重命名/删除（运行中防护、node_modules 删除 + dsh 自愈）、
  默认启动 profile 持久化与 boot 消费。
- Profile 切换（4.3⑥）：管理器行内「启动」= 停当前 dsh 以目标 profile
  重启；仅 webUi 候选；切换不写默认；运行中徽标实时（boot:step 广播订阅）；
  WSL guest 启动脚本参数化（profile 名 shell 引号安全）。
- pnpm 为 boot 硬依赖：缺失自动 `npm i -g pnpm` 补齐，失败阻断给可行动文案。
- IPC 三处同步机器闸门：ipc.rs 单一事实源，漏登记 cargo test 即红。

## 修复
- 创建路径 dist-tag 版本坑：`add @deepseek-ai/dsh-base` 裸名按 latest 解析到
  已弃用旧版 → 404 + 重试卡死；改 `install` 原始版语义。
- Profile 详情对话框：file: 依赖行溢出丢包名、cordis.patch.yml 折行失真、
  超长行 grid 撑破布局三连修。
- 工具窗口内容超高被裁顶：垂直居中改顶部锚定。

## 变更
- 对话框 footer 去脚手架灰底，收编为有意设计（静默化）。
- AGENTS.md 减法一轮 248→178 行；README/CONTRIBUTING/本地运行指引更新。

## 已知问题
- WSL 模式 Profile 切换待 Windows 真机人工验证（按 docs/executor.md 清单）。
- 多开（多 profile 并行多窗口）未排期：前置 = 双实例 storages 竞态 spike +
  1:1 生命周期 ADR 修订。
```

### 2026-09-08 完成通知 · 补丁包启用/禁用开关（4.4③ 补丁包形态补全）—— guan（AI 协作）

- 变更：本 commit——`plugins.rs`（`PluginRowState` 扩展 `contributed_ids`；
  行表解析 `parse_dump_rows_with_section` 带 bundle 段落归属 `# == <bundle>`
  （含 `, patched by <路径>` 后缀剥离——2026-09-08 实机实测）；`build_row_states`
  合成补丁包条目：id = 第一贡献行、shell_disabled = 贡献行**全禁用**（all 语义，
  部分禁用显示启用、切换一次收敛）、patch_entries 合计；`plugin_rows_blocking`
  读 manifest dependencies 走合成）、前端（`types/ipc.ts` 新字段；
  `lib/pluginToggle.ts` 开关目标纯函数；`ProfileDetailPane.toggleDisabled`
  补丁包多目标**串行**写——同一 patch 文件读改写，并发 invoke 相互覆盖）、
  文档（ADR-0009 第七次执行细则修订；ledger 复现点 13 入册——dsh 0.1.2-rc.1
  段落归属 + include 浅覆盖 + loader 组禁用继承；roadmap 4.4 落地记录）。
- 影响：无自身行的补丁包（实测 `@openviking/dsh-memory-plugin`、
  `@tt-a1i/archify-dsh`，`dsh.bundle.patch` 纯 insert 形态）此前无开关、用户
  误判「未安装/解析失败」——现在开关真实可切换：写 profile
  `cordis.patch.yml` 贡献行 `{id, disabled}`，重启生效（不热生效，同 4.4③
  口径）；bundle 级禁用第一类语义**不做**（dsh 无该命名空间）；id 来源仍定死
  dump-config 行表（第四次修订「勿自解析包内 patch 结构」口径不变）。
  **升级复核项**：dump 段落注释格式（含 patched by 后缀）随 dsh 升级须按
  ledger 复现点 13 逐条复核。
- 凭据：cargo test 182 绿（+3）/ clippy -D warnings 清 / fmt 过；前端
  typecheck / lint / vitest 112 绿（+3）；实机验证 = 备份 → 注入两条禁用补丁
  → dump 确认（archify-skill-filesystem 与 openviking-memory 行均得
  `disabled: true`、name/config 不丢）→ 还原 patch（diff 一致）。

### 2026-08-29 完成通知 · 4.3⑥ Profile 切换落地（重启语义）+ WSL guest 脚本参数化；多开登记待办 —— guan（AI 协作）

- 变更：本 commit——`executor.rs`（Executor trait 新增 `set_forced_profile`；
  Local/WSL 双执行器 probe 内按档位消费，bundle 快照档忽略；`GUEST_BOOT` 常量
  改 `guest_boot_script(profile)` + `sh_quote` 单引号进参——profile 名可含
  空格/引号等元字符，反例测试 + bash 实跑回读；WSL `select_profile`/`active_profile`
  同步参数化，v1「写死 web」收口）、`lib.rs`（`switch_profile` / `get_active_profile`
  两条新 IPC；`forced_profile` 目标记录 + `launch_executor_after_probe` 注入，
  错误卡重试延续同目标；模式切换清空重走常规解析）、`profiles.rs`
  （`ProfileSummary.web_ui` 字段 = 启动入口可见性，本模块内判定）、
  `ipc.rs`/`capabilities`/前端（types/api/store/行内启动按钮/运行中徽标/切换
  确认弹窗/文案）。维护者实测反馈两处收口：① 运行中徽标**实时化**——管理器
  订阅 `bootStore.activeStep`（boot:step 经事件总线每窗口广播），切换开始
  徽标即灭、boot 完成即亮，不再等聚焦/手动刷新；② 新增 UI 按 frontend-design
  口径与页面既有语言对齐——启动升级为带字按钮（与页头「新建 Profile」同配方）、
  运行中徽标加脉动心跳点、「无界面」由徽标降级进 meta 行、切换弹窗描述中性化
  （切换非常规破坏性操作，不走删除那套警示红）。
- 影响：**语义裁定（ADR-0009 §4 第三次修订，已先行经维护者逐题确认）**——
  ① 切换 = 停当前 dsh 以目标 profile 重启（dsh 无运行时切换能力，重启是唯一
  语义）；② 仅 webUi 候选可切换，headless/无界面档不给入口；③ 切换**不写**
  defaultProfile（星标是唯一写入口）；④ 失败错误卡 + 重试同目标，不自动回滚；
  ⑤ **WSL 同轮覆盖**：guest 脚本已参数化，但 WSL 真机验证无法在 macOS 执行，
  **待 guan 在 Windows 侧按 `docs/executor.md` 清单人工过一遍**（含切换到
  非 web 档 + 引号/空格 profile 名）。**多开（多 profile 并行多窗口）登记
  roadmap 待办未排期**：机制面可行（`--port 0` 官方旗标），前置 = 双实例
  并发写 `~/.dsh/storages` 竞态 spike + 1:1 生命周期约束的 ADR 修订。
- 凭据：cargo test 131 绿（+5：sh_quote 反例/回读、guest 脚本参数化、
  强制目标档位守卫、切换目标校验）/ fmt / clippy 全过 + `cargo check
  --target x86_64-pc-windows-gnu` 过（WSL 块编译目标语义）；前端
  typecheck / lint / test 40 绿。行为变更纯增，既有命令语义未动。

### 2026-08-28 完成通知 · Profile 创建二次修订：install 后壳补写 Web 工作台声明（创建即 webUi 候选）—— guan（AI 协作）

- 变更：本 commit——`profiles.rs`（`declare_webui_bundle` + 纯函数
  `append_bundle_declaration`：非模板名 install 成功后向
  `dsh.profile.bundles` 幂等追加 `@deepseek-ai/dsh-web-app`；声明补写失败
  降级 pending 态可重试；classify 增 webui_error 分支；模板名跳过——dsh
  拥有模板元组）；`lib.rs` doc；前端创建文案（基础 + Web 工作台/可设为默认）；
  `ADR-0009` §4 第二次修订注；ledger 复现点 7 + 追记；roadmap 4.3② 与事实
  边界行；**AGENTS §6 不变量行扩展（宪法改动，本条即知会 + diff 摘要）**：
  三件套写入例外 #2 = 「非模板名创建成功后的 web-app 声明单键追加」。
- 动机：defaultProfile 消费只认 webUi 候选（bundles 含 web-app，`resolve.rs`
  `list_web_ui_profiles`）——纯 dsh-base 原始版创建出来即无法设为默认启动
  （无 URL 可导航，boot 静默回退 web），用户实机踩坑（设「11」为默认、重启
  仍进 web）。
- 红线边界：三件套写入例外 #2，同类先例 = name 一致化改写。依据 dsh 源码
  `normalizeShippedProfile`（app-boot index.js @ 472，2026-08-28 读）：
  「Any other list is user-owned」——模板精确元组之外的 bundles 列表本就归
  用户/工具所有；目标状态与出厂 web 模板同构（web-app 不进 dependencies、
  `resolveBundleDir` 双锚点零下载），即 web profile 日常运行态。否决替代：
  `dsh plugin add @…@版本`（真实依赖 + 网络安装 + 版本锚定难题，偏离「初始
  web 标准」）。旧版创建的纯 dsh-base profile **不追溯**（重试创建才按新
  标准补齐）。
- 凭据：cargo test 126 绿（install_outcome 产物语义测试改写为
  create_declares_webui_bundle_like_web_template，classify 增⑥声明失败分支）/
  fmt / clippy 全过；前端 typecheck / lint / test 40 绿；实机目检见当日验证记录。

### 2026-08-28 完成通知 · 对话框 footer 静默化：去库存灰底，收编为有意设计 —— guan（AI 协作）

- 变更：本 commit——`ui/dialog.tsx` `DialogFooter` 去掉脚手架模板自带的
  `bg-muted/50` 灰底 + `border-t` 出血分隔带（2026-08-27 `a9c1656` 库存样式，
  未经审视），改为无底无线的静默 footer：按钮右缘与正文对齐，留白交给
  `DialogContent` 自身节奏。四个 profile 对话框（详情/创建/重命名/删除）
  共用组件，一致生效，无逐个覆盖。
- 依据：对话框正文为白底 + 发丝线卡片语言，灰带是全对话框唯一的填充色块，
  且详情对话框 footer 分隔线与 patch 卡片下边框相距 16px 贴出双线噪音；
  按「意图优先于强度」（refined minimalism = restraint）收编为有意设计。
- 凭据：前端 typecheck / lint / test 40 绿；tauri dev 实机目检（详情对话框
  用户协同截图确认：灰带与分隔线移除、按钮对齐、内容区无回归）。

### 2026-08-28 补记 · 详情对话框上一刀引入 grid 撑破回归，已修 —— guan（AI 协作）

- 变更：本 commit——`ProfileDetailDialog.tsx` 内容包装 div 补 `min-w-0`。
- 原委：上一刀把 patch 原文改 `whitespace-pre` 后，`<pre>` 最小内容宽度 =
  最长一行；`ui/dialog.tsx` 的 `DialogContent` 是 **grid** 布局，内容包装 div
  作为 grid 项未设 `min-w-0`，自动最小尺寸被 pre 撑破 → 整条轨道比对话框宽，
  依赖卡片与 footer 一起越界（用户实机截图复现）。`min-w-0` 归零该项对轨道
  尺寸的贡献后，pre 收敛回对话框宽度、由自身 `overflow-auto` 横向滚动。
- 凭据：前端 typecheck / lint / test 40 绿；**tauri dev 实机目检通过**
  （vite HMR 后 AX 点开「web」详情截图核对：依赖三行完整、file: spec 省略号
  截断、原文不折行、footer 归位）。

### 2026-08-28 完成通知 · Profile 详情对话框修复：file: 依赖行溢出 + patch 原文折行失真 —— guan（AI 协作）

- 变更：本 commit——`frontend/src/components/profiles/ProfileDetailDialog.tsx`
  两处：① 依赖行 `file:` 超长 spec 溢出卡片边框、包名被挤压至零宽不可见
  （spec `shrink-0` + 包名 `truncate` 的 flex 收缩方向写反）——改为包名
  `shrink-0` 恒可见、spec `truncate` + `title` 悬停全文兜底；② patch 原文
  `whitespace-pre-wrap` 折行续行顶格、与真实行混淆破坏「原文」语义——改
  `whitespace-pre` + 既有 `overflow-auto` 横向滚动，逐字保真。
- 影响：仅详情对话框展示层，无 IPC / 契约 / 数据改动；超长 spec 悬停可见
  全文。遗留（评审发现、未在本刀范围）：空插件组合复用「无额外依赖」文案
  的措辞错位、依赖版本号 `text-faint` 对比度偏低、对话框无高度约束的矮窗口
  健壮性——待后续小刀。
- 凭据：前端 typecheck / lint / test 40 绿（纯样式改动，Vitest 纯逻辑测试
  不涉及）；diff 已人肉复核。

### 2026-08-28 完成通知 · Profile 创建路径修订：add @deepseek-ai/dsh-base → install（原始版语义）—— guan（AI 协作）

- 变更：本 commit——`src-tauri/src/profiles.rs`（`create_command_args` 改
  `["plugin","--profile",<名>,"install"]`，删除 `CREATE_ADD_BUNDLE`；结果分类
  文案与注释同步「原始版语义」；测试：install 参数 + 原产物语义 + 成功路径
  `Already up to date`）；`lib.rs`（create_profile 文档注释）；`frontend`
  （`content/zh-CN.ts` 创建文案：busy 秒级 / done 内置声明就绪 / hint 原始版；
  `ProfileCreateDialog.tsx` 注释；`types/ipc.ts` 注释）；`docs/`（ADR-0009 §4
  执行细则修订注 + §5 正面后果 + 验证项；ledger 复现点 7 与复核记录；
  roadmap 4.3 ② 与事实边界行）。
- 影响：**ADR-0009 方案 A 执行细则修订（非换方案）**——创建命令由
  `add @deepseek-ai/dsh-base` 改为 `install`：创建语义 = 原始版 profile
  （initProfile 写三件套，bundles 含内置插件随 dsh 安装目录解析；空依赖
  `pnpm install` → `Already up to date`，零网络毫秒级）。触发：2026-08-28
  本机创建 `test` profile 慢至 2 分钟失败/超时，查因 = `add` 裸包名按
  dist-tag `latest` 解析到 dsh-base 0.0.1-rc.1（已弃用旧版，依赖 37+ 个已从
  registry 删除的旧包名：dsh-bash-env / dsh-tasks-local / dsh-skill-local…，
  npmmirror/npmjs 均 404）→ pnpm 递增重试（10s/60s × 37 包）卡死；npmmirror
  对缺失 scoped 包回退到死域名 `r.cnpmjs.org`（本机解析到保留段 198.18.0.192）
  放大表象。`install` 不解析 dist-tag，版本语义免疫。仅周知：pnpm 缺失 →
  补齐 → 失败降级（ADR 口径 2）与「已创建未装插件」中间态重试语义不变；
  后续加外挂插件走同一条 `dsh plugin add` 链（4.4）。
- 凭据：`cargo test` 126 绿；`gate_tests` 三处同步一致
  性绿；`cargo fmt --check` / `clippy -D warnings` 全过；前端 typecheck /
  lint / test 40 绿；实机（macOS，DSH_HOME=临时目录零污染）
  `dsh plugin --profile <新名> install` → init 先行 / `Already up to date`
  186ms 零网络 / 产物 `dependencies:{}` + bundles 仅 dsh-base（含
  pnpm-lock.yaml）/ 后 `--dump-config` 组合启动正常（退出码 0）；diff 已
  逐行人肉复核。

### 2026-08-28 完成通知 · 工具窗口布局修正：内容顶部锚定替代垂直居中 —— guan（AI 协作）

- 变更：commit `4bed1c9`（本 commit 落档本条）——`PageShell` 新增 `align`
  两态：常驻工具窗口（关于 / Profile 管理器）`top` 锚定 + `py-10` 上下
  呼吸位；`BootMode` 等主窗口启动抉择屏维持默认 `center`。
- 影响：仅周知。根因：旧静态壳窗口尺寸贴内容，`items-center` 居中不可见；
  前端迁移后窗口可调大小（profiles 680×700），内容悬浮正中、且内容超高时
  顶部被裁切（flex 居中 + overflow 的经典缺陷），顶部锚定一并消除。
  纯布局改动，IPC / 契约 / Rust 零变化。
- 凭据：`npm run typecheck` / `lint` / `test`（40）全绿；diff −4/+13 已
  人肉复核；布局观感按 4.4 口径归人工目验（Vitest 纯逻辑无 DOM 断言）。

### 2026-08-28 排障记录 · debug 构建白屏——dev 工作流文档修正 —— guan（AI 协作）

- 变更：本 commit——`README.md` 开发段与 `docs/CONTRIBUTING.md` ④ 的运行
  指引改为 `cargo tauri dev`（或 vite + cargo run 两终端），并明示「直接
  cargo run 而 vite 未起 = 壳窗口白屏」。
- 影响：仅周知。**现象与根因**：用户以 `cd src-tauri && cargo run` 启动后
  Profile 管理器 / 关于窗口全白。根因是 debug 构建的前端从 `devUrl`
  （localhost:1420）加载（tauri.conf.json，前端迁移时引入），vite 未运行
  → 资源加载失败；主窗口"正常"是假象——boot 完就导航进 dsh 工作台，把
  白屏的壳 SPA 盖掉了（shell.log 佐证：dsh 就绪 + 导航均正常，1420 端口
  无监听）。此为前端迁移后的既定工作流（`beforeDevCommand` 已配好），
  非管理器代码缺陷；上一条文档修正 commit 写反了 npm run dev 的用途，
  本次一并纠正。
- 凭据：`lsof -i :1420` 无监听 + `~/Library/Application Support/
  io.github.realguan.dsh-dock/shell.log` 显示 boot 全链正常；复现路径
  与修复命令均实机核对（tauri-cli 2.11.4 在机）。

### 2026-08-28 文档修正 · README/CONTRIBUTING 面向前端迁移与 Profile 管理器现状更新 —— guan（AI 协作）

- 变更：本 commit——`README.md`（「它做什么」补 Profile 管理器与 pnpm 环境
  保障两条、WSL 条目 settings.json 字段表述更正、开发命令全面修正：前端
  React SPA 的 npm 闸门 + **`cargo run` 须先 `cd src-tauri`**、品牌资源路径
  ui/assets → frontend/public 与 Emblem、结构树重画含 profiles.rs/ipc.rs 等
  新模块）；`docs/CONTRIBUTING.md`（「壳前端免构建（ui/ 静态页）」过时表述
  更正为 React SPA + npm 三闸门）。
- 影响：仅周知。根因是 2026-08-27 前端迁移与 4.3 各刀落地后文档未跟上——
  本机实测 `cargo run` 在仓库根直接失败（Cargo.toml 在 src-tauri/ 下），
  README 旧命令会误导所有新上手者。
- 凭据：文档改动；所有引用路径已 `ls` 核实存在（frontend/public/mark.svg、
  assets/dsh-logo.svg、scripts/*、node-map/README.md 等）。

### 2026-08-28 完成通知 · pnpm 补齐落地（boot 硬依赖 + 创建时复用，ADR 红线 2 收口）—— guan（AI 协作）

- 变更：本 commit——`updates.rs` 新增 `ensure_pnpm`（PATH 可见即返回；
  缺失经 `npm install -g pnpm` 同步补齐，复用 ADR-0005 npm 全局链 +
  ADR-0006 镜像序；补齐后必须在同一 PATH 重新可见，否则按失败处理）+
  `install_pnpm_via_npm`（双镜像逐试、聚合报错；pnpm 纯 JS 不传
  allow-scripts）+ 3 条测试（命中不装 / 装后可见性强制 / 失败聚合文案，
  假 node/npm-cli fixture）；`executor.rs` LocalExecutor::probe 接线
  （boot 期检查 → 失败阻断 boot 出可行动错误卡；基准 = 注入 dsh 的
  PATH）；`profiles.rs` 创建时防御检测升级为「缺失 → 补齐 → 再失败才报
  错」（复用同一函数）；`find_pnpm` 回归私有（外部消费点已被
  ensure_pnpm 收编）。
- 影响：仅周知。ADR-0009 红线 2 口径 2 至此完整：boot 环境检查保证
  「dsh 全部子命令可用」，补齐失败 = 新增 boot 失败模式（可行动文案：
  检查网络 / npm 镜像 / 手动 `npm install -g pnpm`）。WSL 客体内
  node → pnpm → dsh 链仍归 4.9（ADR-0004 §7）。AGENTS §7 网络面登记
  无变化（boot 期 pnpm 补齐已于 2026-08-28 登记）。4.3 遗留仅剩
  Windows 转发链实机验证（Spike A 遗留）。
- 凭据：`cargo test` 125 绿（+4，含恢复一处被测试插入截断的既有测试
  node_download_urls_mirror_first）/ `fmt --check` / `clippy -D warnings`
  全过；diff 已逐行人肉复核。

### 2026-08-28 完成通知 · 4.3④ defaultProfile 消费接线 + WSL 放开评估收口 —— guan（AI 协作）

- 变更：本 commit——`resolve.rs` 新增纯函数 `consume_default_profile`
  （存储默认值 ∈ webUi 候选才消费，含正反例测试）；`executor.rs`
  LocalExecutor::probe 接线——命中即以该 profile 启动并跳过选择器，
  仅覆盖 dsh_home = 用户 home 的档位（system/download），bundle 快照
  世界不适用；未命中（headless 类无 webUi / 已被手工删除）回退常规
  流程并记日志。ADR-0009 §5 WSL GUEST_BOOT 放开评估收口：**本版不放开，
  维持 `--profile web`，归 4.9**（客体 home 与壳侧 home 不同世界 /
  非 webUi 无 URL 可导航 / 客体内 profile 管理属 4.9 范围）。
- 影响：仅周知。「设为默认启动」语义自此完整：设置 → 持久化 → 下次
  启动自动使用（多 webUi 不再出选择器）；选择器仍在（未设默认时），
  其「选择只影响本次会话」语义不变。验收路径：管理器设默认 → 重启
  应用 → 直接进入该工作台（日志可见 defaultProfile 命中行）。
- 凭据：`cargo test` 121 绿（+1）/ `fmt --check` / `clippy -D warnings`
  全过；diff 已逐行人肉复核。

### 2026-08-28 完成通知 · 4.3 Profile 管理器第五刀（前端管理页）+ AGENTS §4.4 重评 —— guan（AI 协作）

- 变更：本 commit——前端 `pages/ProfileManager.tsx` + `components/profiles/*`
  （列表行两态 / 详情 / 创建 / 复制 / 重命名 / 删除确认五组组件）+
  `stores/profilesStore.ts` + `lib/profiles.ts`（校验镜像等纯逻辑，6 条 Vitest）
  + `lib/tauri.ts`（8 个 profile api，共 20 命令全类型化）+ `types/ipc.ts`
  （五个响应类型锚定 Rust serde 形状）+ `content/zh-CN.ts`（profiles 文案段）
  + `App.tsx`（label=profiles 路由）；后端 `lib.rs`（`open_profiles_window`
  镜像 about 主线程约束 + macOS 菜单 / 非 macOS 托盘入口
  `profiles_manager`）+ `capabilities/default.json`（windows 数组加
  `profiles`——ACL 按窗授权，漏加即整页 IPC 静默拒绝）。
- 影响：**触宪法级**两处，仅周知——① AGENTS §4.4 Vitest 重评条件已触发并
  落盘结论：维持纯逻辑测试，RTL/jsdom 不引入（再评触发 = 需 DOM 断言的
  复杂交互）；② TanStack Query 未接入：管理页是「读一次 + 变更后手动刷新」
  形态，frontend-migration §11 的触发条件裁定延后（rationale 见
  profilesStore.ts 头注释，出现跨窗口订阅诉求再立 micro-ADR）。
  范围声明：4.3 全部六项能力的 UI 至此可用（菜单/托盘 → Profile 管理器）；
  defaultProfile 的 boot 消费接线、pnpm 补齐仍归后续刀。前端文案含
  删除确认三要素（不级联全局数据 / 其他 dsh 实例 / 模板名重新物化，ADR
  §2 要求）与创建 pending 中间态（ADR §3 方案 A 契约）。
- 凭据：`npm run typecheck` / `lint` / `test`（40，+6）/ `build` 全绿；
  `cargo test` 120 绿 / `fmt --check` / `clippy -D warnings` 全过；
  diff 已逐行人肉复核。

### 2026-08-28 完成通知 · 4.3 Profile 管理器第四刀（生命周期 + 默认持久化）—— guan（AI 协作）

- 变更：本 commit——`profiles.rs`（复制/重命名/删除文件层 + 前置校验 + 运行中
  防护文案 + patch `../` 引用扫描警告 + 8 条测试）；`settings.rs`
  （`defaultProfile` 字段，第二最小面例外）；`executor.rs`（`active_profile`
  trait 方法：运行中防护比对源，本地取 launch、WSL 固定 web）；`lib.rs`
  （5 个新 IPC 命令；**连带修复** `switch_mode`/`choose_mode` 改
  load-modify-save——原整体覆盖写法会抹掉 defaultProfile）；三处同步
  （ipc.rs/capabilities）。引用面全按 Spike B §3 执行：复制排除 node_modules
  + name 改写；重命名删 node_modules 让 dsh 自愈；删除不级联 sessions；
  defaultProfile 删除时清除、重命名时同步，失效读取侧兜底 web。
- 影响：**触宪法级**——AGENTS §6 例外册登记 `defaultProfile` 落地、§7 IPC
  登记表新增 5 命令（`copy_profile`/`rename_profile`/`delete_profile`/
  `set_default_profile`/`get_default_profile`），仅周知。范围声明：管理器
  后端能力至此齐备（列出/详情/创建/复制/重命名/删除/切换默认）；前端管理页、
  defaultProfile 的 boot 消费接线（含 WSL GUEST_BOOT 放开多 profile 评估）、
  pnpm 补齐均归后续刀。
- 凭据：`cargo test` 120 绿（112 + 新增 8：复制排除与改写 / 重命名自愈 /
  删除不级联 / 运行中防护文案要素 / 默认候选校验 / settings 旧格式兼容等）；
  `gate_tests` 三处同步一致性绿；`cargo fmt --check` / `clippy -D warnings`
  全过；diff 已逐行人肉复核（经维护者裁定 ①+② 范围：生命周期 + 默认持久化
  一刀完成，pnpm 补齐与前端拆分后续刀）。

### 2026-08-28 完成通知 · 4.3 Profile 管理器第三刀（创建能力）—— guan（AI 协作）

- 变更：本 commit——`src-tauri/src/profiles.rs`（创建段：转发链 spawn 封装 +
  前置校验 + 结果分类 + 4 条纯函数测试）；`updates.rs`（`find_pnpm` 转
  pub(crate) 共用）；`ipc.rs` / `lib.rs` / `capabilities/default.json`（IPC 三处
  同步，`create_profile` 为**首个异步命令**：探测 + 转发链全在 spawn_blocking，
  避免同步命令冻结主线程）；`AGENTS.md` §7；ADR-0009 §5；ledger 复现点 7。
  能力：spawn `dsh plugin --profile <名> add @deepseek-ai/dsh-base` 半官方路径
  创建 profile（三件套由 dsh initProfile 写出，壳零写入）；重名拒绝 + 半初始化
  放行重试（重跑 add 幂等，ADR §4）；pnpm 防御检测缺失即拒 spawn（可行动文案，
  补齐归后续刀）。
- 影响：**触宪法级**——AGENTS §7 IPC 命令登记表新增 `create_profile`，仅周知。
  范围声明：定位仅系统探测（离线档/未装系统 dsh 用户暂不可创建，报可行动错误）；
  复制/重命名/删除/默认持久化/WSL 客体内 profile/pnpm 补齐均归后续刀。
  实机验证（macOS，DSH_HOME=临时目录零污染）：init 先行 / pnpm 经注入 PATH 可
  定位 / 顺带实测 pnpm 网络失败模式（镜像 ECONNRESET -> 已创建未装中间态
  exit 1，分类文案与单测 fixture 逐字吻合）；成功路径 reconcile 沿用 Spike A
  §3.2 同机同版本结论；Windows 转发链（shell: win32 分支）仍为遗留。
- 凭据：`cargo test` 112 绿（108 + 新增 4）；`gate_tests` 三处同步一致性绿；
  `cargo fmt --check` / `clippy -D warnings` 全过；diff 已逐行人肉复核。

### 2026-08-28 完成通知 · 4.3 Profile 管理器第二刀（只读能力）—— guan（AI 协作）

- 变更：本 commit——新增 `src-tauri/src/profiles.rs`；`ipc.rs` / `lib.rs` /
  `capabilities/default.json`（IPC 三处同步）；`AGENTS.md` §7；`docs/adr/0009`
  §5 勾选；`docs/contracts/dsh-behavior-ledger.md`。能力：profile 非法名校验
  （与 dsh `resolveProfileDir` @ 318 逐字一致）、profiles 扫描器（已物化 +
  未物化内置模板名两态合并；排除 `profiles/node_modules` 符号链接农场）、单
  profile 详情（package.json 关键字段 + `cordis.patch.yml` 原文，YAML 不解析——
  serde_yaml 已弃维，依赖选型推迟到启停插件刀）；新 IPC 命令
  `list_profiles` / `get_profile_detail`。纯读：零写入、零 dsh 子进程、零网络。
- 影响：**触宪法级**——AGENTS §7 IPC 命令登记表新增两条（新命令流程规定动作），
  仅周知。范围声明：管理器仅覆盖壳侧本地 home（`user_dsh_home()`）；WSL 客体内
  profile、创建/复制/重命名/删除、`--dump-config` 详情、pnpm 补齐均归后续刀。
  ledger 复现点 6/8 已按行号勘误口径（318/323/353，弃 11826/13418）入册。
- 凭据：`cargo test` 108 绿（基线 98 + 新增 10：校验正反例 / 扫描两态与农场
  排除 / 详情路径遍历拒绝等）；`gate_tests` 验证三处同步一致性；
  `cargo fmt --check` / `clippy -D warnings` 全过；diff 已逐行人肉复核。

### 2026-08-28 宪法级改动 · AGENTS.md 减法：248 → 178 行，删微观管理留边界 —— guan

- 变更：`AGENTS.md` 全文重写（commit `876dbcb` 之后）——删除三类内容：① 通用工程
  常识（跨平台 #[cfg] 分叉、阻塞主线程禁令、组件文件命名、Zustand 选择器细则、
  mock 策略细节等——AI 工程常识默认做对，写清单反而暗示「除此以外随便」）；
  ② 可推导明细（模块职责逐个注释、依赖版本行、构建发布行、§7 专项裁定长文——
  §9 索引已有等价一行结论，消双源）；③ 过度规定（错误页样式、stdout 日志技法——
  归还代码注释与 ADR）。保留全部边界与真坑：两红线（含台账指针）、dsh 文件系统
  不变量、唯一网络面、IPC 三处同步、事件总线模块加载期竞态坑、child_cmd、
  tauri-cli 同代、zip pinned、必测四场景、AI 交互七条、持久化例外册、§11 元规则。
- 影响：**宪法级**——规范哲学转向「只写边界 + 真坑 + 为什么，不写怎么做」，给
  AI 留发挥空间；被删内容均有归宿（代码 / ADR / roadmap / 全局 AI 配置）或属
  可推导。此前批准的全部提交于 `876dbcb`，本笔减法独立可回溯。
- 凭据：纯文档改动，不触运行时；全文 178 行 ≤ 250 预算、单节最大 31 行（§4）；
  真坑与边界关键词 grep 抽查全数在册；修改未提交，待确认后提交。

### 2026-08-28 宪法级改动 · 规则方向性审核落地：复现台账 + 白名单闭环 + 规则触发器 —— guan

- 变更：① 新建 `docs/contracts/dsh-behavior-ledger.md`——dsh 行为复现台账（已落地
  5 项 / 计划 3 项，基线 v0.1.1-rc.2），堵「复现点随 dsh 升级静默漂移、CI 无法发现」
  缺口，dsh 升级 = 逐条复核触发器；`AGENTS.md` §0 红线 1 挂台账指针。② §4.4 白名单
  治理闭环——「未来数据层」改「数据获取层（取数·缓存·同步）」判据，新增依赖 =
  先回写清单（唯一权威）再广播。③ 两条规则补再评估触发器——RTL/jsdom 禁令挂
  「Profile 管理器 UI 落地时重评」、无 dev-dependencies 加「确需专用依赖先 ADR」口子。
  ④ roadmap Next 加工程前置——IPC 三处同步自动化自检 spike（4.3 开工前把人肉纪律
  升级为机器闸门）。
- 影响：**宪法级**（AGENTS.md）+ 新增台账文档。dsh 升级流程多一步「复核台账」；
  前端新增依赖流程变「回写清单 + 广播」；4.3 开工前多一个 spike 闸门。纯文档，
  不触运行时。
- 凭据：复现点逐条 grep 锚定代码实际位置（shell.rs / resolve.rs / updates.rs /
  executor.rs）；AGENTS 全文 248 行 ≤ 250 预算、§4 恰 40 行；修改未提交。

### 2026-08-28 宪法级改动 · 跨文档定位统一 + AGENTS 一致性修正 —— guan

- 变更：三份文档定位统一为「dsh 的桌面管理面板」（2026-08-27 重定义的收尾）——
  roadmap §1 开头「桌面终端 / 极小的壳」改写、README 开头定位段改写（补管理能力句）、
  contract.md 宿主解析节加历史定位注（ADR-0005 语义不变）；AGENTS.md 五处一致性
  修正——§7 例外册补登记 `app:update` 事件（updater 回推，仅 main/about）、§2 docs
  清单补 frontend-migration 与 spikes/、§5 测试命令收敛为 §1 指针（消双源）、§0
  工程准则补 §6 指针、§6 持久化例外改「例外册登记制」（与 roadmap 硬约束 4 对齐，
  第二例外落地前先登记字段名）。
- 影响：**宪法级**（AGENTS.md + contract.md）。定位口径此后以 AGENTS §0 为唯一
  权威；settings.json 第二字段落地前须先在 §6 登记。纯文档，不触运行时。
- 凭据：grep 全仓「桌面终端」仅剩 ADR 历史引用（刻意保留）；本笔与上一条（制度
  建设）同批未提交，待确认后按 CONTRIBUTING 提交。

### 2026-08-28 宪法级改动 · AGENTS.md 首轮回收：394 → 244 行，回归 §11 预算 —— guan

- 变更：`AGENTS.md` 全文按 §11 回收——§2 目录树（58→17 行）：`ls` 可推导的文件级
  明细删除，只留职责与陷阱（勿动 / 勿手改 / 永不入库）；§7 例外册（50→19）：IPC 命令
  收敛为登记行，专项裁定收敛为「一行一裁定 + ADR 编号」，推理细节以 §9 索引 + ADR
  为唯一源（消除双源）；§1 技术栈（35→18）：依赖版本明细还 Cargo.toml / package.json，
  只留坑与裁定锚点；§4 代码规范（69→40）：正反例去重合并；§5 测试明细（25→13）还
  roadmap；§0 / §8 / §9 / §10 措辞收紧。只删可推导明细与已有归宿的长文，裁定结论全保留。
- 影响：**宪法级**——纯回收、无新增裁定；所有删除项均有既有归宿（ADR-0001~0009 /
  roadmap 4.1/4.2 / 代码现状）。协作者若发现某被删细节在 ADR / roadmap 也找不到，
  频道提出即可，git 历史可回溯旧版全文。本笔与上一条（§11 元规则）同批未提交，
  建议合为一笔提交（hunk 交叉无法拆分）。
- 凭据：纯文档改动，不触运行时；全文 244 行 ≤ 250 预算、单节最大 40 行（§4），
  diff 394→244（−150）。另发现 README.md「结构」节仍是 React 迁移前旧版（ui/ 静态页），
  属已知失真，另行开工修正，本笔不动。

### 2026-08-28 宪法级改动 · AGENTS.md 新增 §11 写入边界（元规则）—— guan

- 变更：`AGENTS.md`——① 文件头挂一行指针（「最小必要集，不是知识库」）；② 新增
  §11 元规则：准入判据四条（高频 / 违约即事故 / 不可推导 / 无家可归，核心判据 =
  「没有这条 AI 会做错吗」）、排除清单七类（模块技法→注释、决策推理→ADR、流程
  细则→CONTRIBUTING、契约细节→contract.md、计划指标→roadmap、操作手册→docs
  专项、通知→broadcasts）、形态规则（结论一句 + 日期 + 指针；升格 ADR 后原条目
  必须回收，禁双源）、预算与回收（全文 ≤ 250 行 / 单节 ≤ 40 行）。
- 影响：**宪法级**——此后向 AGENTS.md 写入任何条目须先过 §11.1 判据，不合者
  review 可依据本节驳回；知会落档、broadcasts 登记范围不变。存量超支（~354 行，
  §2 目录树 / §7 例外册为主）按新规则另行开工一笔回收。
- 凭据：纯文档改动，不触运行时；本笔仅新增元规则、不回收存量（一次一意图）；
  修改未提交，待确认后按 CONTRIBUTING 提交。

### 2026-08-27 宪法级改动 · 项目边界重定义：dsh 桌面管理面板 + 两红线 —— guan

- 变更：`AGENTS.md`（§0 定位重写为「dsh 的桌面管理面板」/ §1 技术栈表数据库行 /
  §4.2 禁库改「需 ADR 评估」/ §6 存储与生命周期重写）、`docs/roadmap.md`（硬约束
  2/4 重写）、`docs/adr/0008`（壳保持薄加注「仅约束运行时」）、`docs/adr/0005`
  （转发链 vs 全局安装区分注）、`docs/adr/0006`（管理功能网络面不属唯一网络面注）、
  `docs/executor.md`（defaultMode 例外表述同步）、`docs/adr/0009`（§2 重写为
  3 红线 + 工程准则）。
- 影响：**宪法级**——① 项目定位从「通用产品壳」改为「dsh 的桌面管理面板」；
  ② 红线定为两条：不修改 dsh 源码 + 安装包不内置依赖（优先宿主检测、缺失时
  实时下载，含 pnpm 自动补齐扩展）；③ 「壳保持薄 / 无状态库」降为**仅约束运行时**，
  管理功能按优秀软件工程设计（可引入数据库 / 持久化）；④ 历史 ADR 保持原状仅加注。
  协作者注意：管理功能（4.3+）不再受「无状态库」限制，但 dsh 文件系统不
  变量（0600 凭据 / 三键 / 农场只读）继续必须维护。
- 凭据：纯文档改动，不触运行时；diff 已逐文件核对（AGENTS §0/§1/§4.2/§6 +
  roadmap 硬约束 + 4 个 ADR 加注 + executor.md 同步）。修改未提交，待确认后按
  CONTRIBUTING 提交。

### 2026-08-27 快车道直推 · 完成通知：Now 阶段收口（4.1 工程化基线 + 4.2 updater 测试）—— guan

- 变更：六连提交——`bd94596` fix(clippy) 全部 17 处警告清零（关键：
  shell.rs `floor_char_boundary` 击穿 rust-version=1.77.2 的 MSRV 承诺，
  CI 全用最新 stable 故未暴露）；`dd521d1` style 全仓 fmt 归一
  （9 文件 168+/147- 纯机械，与 clippy 修复分仓提交）+ 落地仅锁 edition 的
  `rustfmt.toml`；`9029ebe` chore(rust)；`10e0957` ci 三平台
  fmt --check / clippy -D warnings 闸门 + ubuntu coverage job
  （cargo-llvm-cov 出 lcov，先出数不定阈值）；`5417987` test(updater)
  六条纯函数测试；docs 提交（见下）。
- **宪法级改动（本次知会）**：① AGENTS §1 Rust 行——移除 `rust-version`
  基线，**Rust 工具链跟随最新 stable**（2026-08-27 维护者裁定：不设 MSRV；
  上限纪律不变：CI @stable 自动跟新）；② AGENTS §1 Lint/Format 段改写为
  已建基线状态；③ AGENTS §5 updater 待补条目改为已覆盖表述；
  ④ AGENTS **新增 §8 第 7 条「驳回不合理的规则」**——AI 判定规范与现实
  冲突/自相矛盾/失效时应停手提请驳回与修订（举证义务在提请方），
  不得以变形实现绕行；顺从≠忠诚，变形合规比违规更危险。
- 影响：CI 首跑新闸门有红的风险已用本地预演对冲（本机 1.98 三道全绿）；
  Next 阶段（4.3 Profile 管理器）进入条件满足，开工前先做两个前置 spike。
- 凭据：本地 rustc 1.98.0 下 fmt --check / clippy --all-targets -D warnings /
  cargo test 95 绿；fmt 与 clippy 两类 diff 分仓提交均经人肉复核。

### 2026-08-27 发版事项 · v0.5.1 三平台验收通过，冻结期解除 —— guan

- 确认：tag run `33049383090` 三平台 job 与 release job 全部 success；Release
  资产 14 个齐全（dmg/exe/msi/AppImage/deb/rpm + 签名 + latest.json）；
  下载 macOS `.app.tar.gz` 实拆——`Info.plist` 版本 0.5.1，主二进制内嵌前端
  bundle 五个事件名（app:update / boot:step / boot:progress / boot:update /
  boot:error）grep 全中（本次缺陷的产物级判据）。上游 dsh 会话工作正常。
- 影响：**v0.5.1 发布完成，冻结期自此解除**——master 恢复正常合流
  （改动仍按 CONTRIBUTING 占用声明纪律）；下一步按路线图 Now 阶段推进。
  已装 0.5.0 的环境因自更新面板同受缺陷影响，需手动换装 0.5.1。
- 凭据：`gh api .../runs/33049383090/jobs` 全 success；本条即对上两条
  「处置见下一条广播」预告的闭环。

### 2026-08-27 发版事项 · v0.5.0 缺陷确认 → 重切 v0.5.1 热修 —— guan

- 变更：fix `f3cef30`（事件总线 import 锚点，见下条补记）+ updater 观测日志
  （run_check 入口与 set_state 每次推进记 tracing）+ 全仓版本升 `0.5.1`
  （tauri.conf.json / Cargo.toml / Cargo.lock / frontend package.json+lock），
  tag `v0.5.1` 当日推送。
- 裁定（为何不沿用上次 force 迁移 tag 的做法）：v0.5.0 三平台产物**已发布且含
  缺陷**——事件监听缺失不止影响关于页：启动时间线 / 错误卡 / 下载进度同链路
  全部不刷新，冷启动表现为启动页冻结后硬跳工作台、失败时错误卡不渲染。
  上次 force 迁移的前提是「原 run 无任何产物」；本次 Release 已存在、可能已有
  下载，force 迁移会留下同名异物的资产，违反可追溯原则。按语义化版本重切
  v0.5.1。注意：**0.5.0 客户端的自更新面板恰好也受此缺陷影响**（自动检查在
  Rust 侧正常执行，但 UI 不回显），已装用户需手动换装 0.5.1。
- 影响：冻结期继续（master 只收 fix）；CI 三平台验收通过前不宣布发布完成。
- 凭据：cargo test 89 绿 + 前端四道门禁绿；实机 AX 观察到修复后徽章
  「检测中 → 最新」翻转（自动首查全链路），手动点击路径同链路 +
  新增日志可事后定位；版本 diff 六文件已人肉复核。

### 2026-08-27 补记 · v0.5.0 实机缺陷：事件总线未进 bundle（关于页检查更新无反应） —— guan

- 变更：fix 待提交——`frontend/src/main.tsx` 增加 `import "./lib/events"` 副作用
  锚点；`docs/frontend-migration.md` §9 新增「事件总线」产物级回归清单条目。
- 根因：`lib/events.ts` 靠模块加载期 `initEventBus()` 自装配（宪法 §4.3 裁定），
  但全仓没有任何运行时 import 它——Vite 树摇将其整体排除出 bundle（v0.5.0
  dist 中 `app:update` 出现 0 次，实锤）。所有窗口的 boot:*/app:update 监听均未
  注册；关于页显示的「已是最新」全部来自进入时的播种 invoke，恰好掩盖断链。
  纯逻辑单测（34 例全绿）测不出「装配丢失」这类集成缺失。
- 影响：**v0.5.0 三平台产物若已出包则携带此 bug**——关于页更新状态机不再实时
  推进、启动时间线/错误卡/下载进度不刷新。处置与是否重打 tag 见下一条广播。
- 凭据：前端四道门禁绿 + 新产物 grep 五事件名各 ≥1；修后复验结论随附。

### 2026-08-27 补记 · v0.5.0 发布中断修复（Ubuntu CI 失败 → 重发） —— guan

- 变更：`915657f`（冻结期 fix）—— beforeBuildCommand 钩子 cwd 显式化
  （tauri.conf.json ScriptWithOptions `cwd="../frontend"`），build.yml/AGENTS
  注释同步；`v0.5.0` tag 已 force 迁移指向该修复（原 tag run 全失败、无任何
  产物/Release，无污染可追溯亏损）。
- 根因（CI 实证两连修）：① tauri-cli「自动发现含 package.json 目录」深度遍历
  在 Linux ext4 目录序下可能先命中 `node-map/` → npm ci 找不到 lockfile（本地
  APFS 碰巧命中 frontend/，阶段 A 的验证结论被事实击穿）；② 显式 cwd 相对基准
  实为 **src-tauri**（build.rs `set_current_dir(dirs.tauri)`），首修用的
  `frontend/` 本地复现 No such file 后改为 `../frontend` 复测通过。
- 影响：仅发布链路，不触运行时行为——macOS/Windows 两 job 原 run 继续走完
  但其构建内容同构（hook 修复对三平台同效），三平台产物仍以重发 run 为准。
  经验已落档：**tauri 钩子 cwd 永远相对 src-tauri 且必须显式**，勿复信自动发现。

### 2026-08-27 发版事项 · v0.5.0 发布开始 —— guan

- 变更：`chore: 版本 0.5.0（…整批提交）`——tauri.conf.json / Cargo.toml /
  Cargo.lock / frontend package.json+(lock) 同步升版；roadmap 适用版本标 v0.5.0。
  tag `v0.5.0` 由本档案登记当日推送，CI 三平台矩阵 + Release 聚合
  （notes 由 GitHub 自动生成）。
- 影响：**冻结期开始**（CONTRIBUTING §8）——Release notes 发出至三平台产物
  验收通过期间，master 只收 fix 不收 feat。本版内容：前端自静态 HTML 全量迁移
  Vite+React+TS+Tailwind v4+shadcn/ui（ADR-0008 全流程，span commit
  aab68c1→本次），壳行为与 12 IPC 命令零变更；Move/关于/启动/选择器四页
  组件化；宪法同步修订（AGENTS §1/§2/§4.2/§4.3/§4.4/§5/§7）。
- 凭据：frontend gate 34/34；cargo test 89 passed；本机 release 构建三产物齐；
  已知遗留——Windows/Mode 页实机走查未做（广播 2026-08-27 阶段 C 条目）；
  fmt/clippy 基线待专项（阶段 E 条目）。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 E 落地（d003905 / 1868411） —— guan

- 变更：**宪法级（1868411）**——AGENTS §1/§2/§3/§4.2/§4.3/新增 §4.4/§5/§7、
  docs/roadmap.md（硬约束 2 与不做清单）、docs/CONTRIBUTING.md（路径行）、
  .github/workflows/build.yml（Frontend gates 步骤）；**非宪法（d003905）**——
  Vitest 34 用例（format/bootProgress/bootStep/updatePhase）、`ui/` 目录删除
  （dsh-logo.svg 迁至仓库根 `assets/`）、全仓悬空引用清理。
- 影响：① **宪法已生效**——「禁止引入任何前端构建链」修订为「前端框架仅限
  React 生态（§1/§4.2/§4.4 白名单）」；② 开发者须知——本地构建/调试请从
  **仓库根**调用 `cargo tauri dev/build`（钩子 cwd 发现逻辑），前端开发需
  node ≥20（`cd frontend && npm ci`）；③ **fmt/clippy 基线评估结论**：存量
  35 文件未归一 + clippy 9 警告，需专项 chore 落地（遵守「不引入全仓格式化
  diff」红线），本轮 CI 只接前端四道闸门，roadmap §4.1 [待补充] 保持；④ 迁移
  完成发布契——master 自此无 `ui/`，release 产物壳页面全 React。
- 凭据：frontend typecheck/lint/vitest 全绿（34/34）；`cargo test` 89 passed；
  `cargo tauri build --no-sign` 出齐三产物；diff 逐行人肉复核（lib.rs 仅两处
  注释；fmt 越界改动已回退——本次 session 自身纪律记录）。

### 2026-08-27 占用声明 · 前端迁移阶段 E 开工（宪法级变更预告） —— guan

- 变更：占用 `AGENTS.md`（§1/§2/§3/§4.2/§4.3+新增§4.4/§5/§7）、
  `docs/roadmap.md`（硬约束 2 与不做清单）、`.github/workflows/build.yml`
  （node 质量闸门 + fmt/clippy 评估）、`ui/`（删除）、`frontend/`（Vitest 测试）。
  依据：ADR-0008 行动项 + docs/frontend-migration.md §6/§7/§10 阶段 E 清单。
- 影响：**宪法级预告**——AGENTS §4.2「禁止引入任何前端构建链」将修订为
  「前端框架仅限 React 生态（Vite+React+TS+Tailwind+shadcn/ui）」，§4.3 全面
  重写为 React 组件规范，新增 §4.4 三条红线（依赖白名单 / 前端禁止网络请求 /
  跨窗口真相源）。`ui/` 目录删除、`dsh-logo.svg` 迁至仓库根 `assets/`。
  执行顺序：测试 → 删 ui → 宪法/CI（单独 commit）→ 全量验证 → 完成通知。
- 凭据：阶段 A-D 均已通过闸门与实机验证（见前四条约）；本预告为宪法修改
  前置知会，修改范围与方案 §7 清单一一对应。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 D 落地（1008cd6） —— guan

- 变更：`pages/BootIndex.tsx` 整页、`components/boot/{BootStep,BootTimeline}.tsx`
  新增、ErrorCard diag 形态、`lib/events.ts` 总线装配时机、`pages/BootMode.tsx`
  握手时序、方案文档 §3.3（总线裁定同步）。**四页至此全部迁入 React**。
- 影响：一处时序裁定周知——事件总线要求在**页面任何播种 invoke 之前**注册；
  实现为模块加载期装配（详情见方案 §3.3 与 lib/events.ts 注释）。stage B 中
  BootMode「先 invoke 再导航」的写法本轮已更正为旧握手（携参回启动页由
  BootIndex 落地）。
- 凭据：typecheck/lint/build 全绿；release 冷启动事件链全通（日志钉板）；
  BootIndex 静态帧经 dev 预览核对；下载条/错误卡的实机触发依赖特定失败路径，
  逐行对照旧码迁移（已复核）。阶段 E 前 master 中间态照旧：壳页面功能已全，
  待删 `ui/` 与宪法修订。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 C 落地（d780855） —— guan

- 变更：`frontend/src/pages/{BootMode,BootSelector}.tsx` 整页、
  `components/boot/`（DownloadProgress/ErrorCard/VersionChip/PulseBar 落地，
  阶段 D 复用）、`hooks/usePlatform.ts`、文案层 mode/selector 扩展。
- 影响：仅周知 + 一项待办迁移——**Mode 页（运行环境选择）是 Windows-only
  表面**，React 版实机目视验证待 Windows 环境（非 Windows 访问按裁定防御性
  回启动页，本机已验证该兜底路径编译正确）。其余同前：master 中间态渐次回填。
- 凭据：typecheck/lint/build 全绿；release 实机 fixture 双工作台触发选择器
  （双卡片 + DEFAULT/CUSTOM 徽标 + 版本芯片渲染正确，截图存档）；点击官方卡
  打通 `choose_profile` IPC 全链路（shell.log 钉板：dsh 启动 profile=web →
  1.3s 就绪 → 导航工作台）。测试进程与 fixture 均已清理。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 B 落地（5ce1296） —— guan

- 变更：`frontend/src/components/about/`（新建四组件）、`pages/About.tsx` 整页、
  `content/zh-CN.ts` 文案扩展、`index.css` token 改名、`App.tsx` 预览钩子、
  方案文档 §3.5 同步。关于窗口自旧 `ui/about.html` 完整迁入 React。
- 影响：仅周知 + 一处 token 命名裁定——`--color-muted/--color-accent` 与
  shadcn 语义层重名导致工具类被覆盖（实机截图发现文字近白），域 token 更名
  **dim / brand**；后续页面（阶段 C/D）直接用新名。master 中间态照旧：壳骨架页
  渐次回填，阶段 E 收口删 `ui/`。
- 凭据：typecheck/lint/build 全绿（gzip 137KB，<500KB 复审线）；release 实机
  截图验证整链路（自动首查→upToDate、三维度真实数据、工作台地址注入）；
  配色修复以构建产物 CSS 钉板（`.text-dim{color:var(--color-dim)}` 解析唯一，
  内联映射机制反向解释原 bug）；本地 release 产物已重建包含修复。

### 2026-08-27 补记 · 阶段 A 目视验证完成（步骤 20/21 清账） —— guan

- 变更：`docs/frontend-migration.md` §3.1 一处标注（钉板句从「待复核」改为
  「已复核通过」）。无代码改动。
- 影响：仅周知——release 产物内以临时第二 webUi profile 触发
  `/selector` 直达，**SPA fallback 实机命中**（主窗口渲染 BootSelector 页，
  步骤 21 钉板完成）；about 窗口经菜单打开，label 路由渲染 About 页（步骤 20）。
  临时 fixture（仅含 package.json 的 `~/.dsh/profiles/probe-dual-webui`）
  已删除，真实环境零残留。
- 凭据：实机截图两帧（主窗口 /selector 内容 + 关于窗口内容）；进程已退出。

### 2026-08-27 快车道直推 · 完成通知：前端迁移阶段 A 落地（a9c1656） —— guan

- 变更：`frontend/`（新建 41 文件）、`src-tauri/tauri.conf.json`、
  `src-tauri/src/lib.rs`、`.github/workflows/build.yml`。Vite+React+TS+Tailwind v4
  + shadcn/ui 七组件脚手架；窗口 label 路由；四页占位骨架；Rust 主/about 窗口
  改载 SPA 根、selector/index 跳转改 pathname；platform_script 扩 `{os,wsl}`。
- 影响：**master 中间态**——自本 commit 起 release 产物壳页面为 React 骨架
  （功能回填顺序 B About → C Mode/Selector → D Index，阶段 E 收口删 `ui/`
  并改宪法）。与占用声明的偏差仅一处：CI node 步骤自阶段 E 提前（frontendDist
  切换后 `cargo tauri build` 硬依赖 npm 构建，不提前则打 tag 即挂）；
  Build installers 工作目录随之移到仓库根。开发者注意：本地构建/调试请从
  **仓库根**调用 `cargo tauri dev/build`（钩子 cwd 发现逻辑要求，见 build.yml 注释）。
- 凭据：typecheck/lint/vite build 全绿（JS gzip 79.9KB）；`cargo test` 89 passed；
  本机 `cargo tauri build --no-sign` 出 dmg/.app/updater tar 三产物；release 实机
  启动 Rust 全链路通过入工作台。步骤 20/21 的目视项（骨架观感、release 内
  /selector 直达 SPA fallback 实测）待人工复核。

### 2026-08-27 占用声明 · 前端迁移阶段 A 开工（ADR-0008 实施开始） —— guan

- 变更：占用 `frontend/`（新建）、`tauri.conf.json`（frontendDist 切换）、
  `.github/workflows/build.yml`（node 步骤合批）、`.gitignore`；阶段 E 收口时改动
  `AGENTS.md`（§1/§2/§3/§4/§5/§7）与 `docs/roadmap.md`（硬约束 2 与不做清单）。
  实施依据：ADR-0008 + `docs/frontend-migration.md`（commit aab68c1、f21df95）。
- 影响：**宪法级变更预告**——AGENTS §4.2「禁止引入前端构建链」将在阶段 E 按既定
  方案修订为「Vite + React 定向许可」；执行顺序 A 脚手架 → B About → C Mode/Selector
  → D Index → E 测试与治理收口，各阶段完成即在频道知会。阶段 A–D 仅新增文件，
  不动现有 `ui/` 与 Rust 行为；master 保持随时可构建。他人如需动上述文件请先在
  频道协调。
- 凭据：Tailwind v4 + shadcn/ui 兼容性 spike 已通过（2026-08-27 临时目录验证：
  shadcn init 显式识别 v4、七组件生成、strict TS 下 vite build 成功 JS gzip 73KB；
  结论与三注意点已回写方案 §1 并勾销 ADR-0008 行动项）。本条与 spike 回写同 commit。

### 2026-08-27 快车道直推 · 完成通知：roadmap 对照 dsh 源码核查修订 —— guan

- 变更：`docs/roadmap.md`（+43/−27；本档案与该改动同 commit 落盘）。对照
  deepseek-harness v0.1.1-rc.2 源码逐条核查路线图事实主张后修订：事实表重做
  （子包实测 227、`dsh plugin` = pnpm 原样转发、patch 按 id 逐字段赋值且 `config`
  整体替换不深合并、出厂 profile 模板仅 web/headless 且无任何 profile 管理官方命令、
  dsh 无插件安装/卸载 UI），新增「事实边界与陷阱」清单（非法名校验、profiles/node_modules
  符号链接农场、`.credentials.yaml` 三条硬约束等）；行动项修正：4.3 创建路径弃
  agent-presets 误用改为半官方 plugin-add 引导、4.4 pnpm 失败模式入错误处理并补全
  运行时状态枚举、4.2 updater 测试表述更新、编号引用与版本头（v0.4.7 起）修正。
- 影响：两项裁定仅周知——① 4.3④ 默认启动 profile 持久化到 `settings.json`，经
  维护者批准作为第二最小面例外（落地实现时同步登记 AGENTS §6 后方可合入）；
  ② Next 阶段（Profile 管理器）开工前须先做两个 spike：GUI 环境（无 shell rc）
  pnpm 经 `dsh plugin` 转发链的可用性验证、复制/重命名 profile 的引用清点。
- 凭据：纯文档改动不触运行时；事实主张均锚定 dsh 源码位置（app-boot/src/profile.ts、
  vendor/include/src/index.ts、apps/cli/src/plugin.ts 等）；残留检查干净
  （旧引用 197 包数/（3.4）/零测试 表述已全部清零）。

### 2026-08-27 建档 —— guan

- 变更：新建本档案；`AGENTS.md`（§2 目录树 / §8.5 收尾三件事 / §10 知会落档条目）
  与 `CONTRIBUTING.md` §0 各挂一处指针。
- 影响：协作者此后按上表登记知会；本次指针挂接触宪法级文件，本条即为对其的知会。
- 凭据：纯文档改动，不触运行时。

### 2026-08-27 补记 · 完成通知：删除前端顶栏「关于」入口 —— guan

- 变更：commit `8075eea`——`ui/index.html` / `ui/mode.html` / `ui/selector.html` /
  `src-tauri/src/lib.rs` / `build.rs` / `capabilities/default.json`。删除三页壳顶栏
  「关于」按钮及 `open_about` IPC 整链，与原生常驻入口重复（Windows 启动页问题报告）。
- 影响：**触宪法级**——`AGENTS.md` §7 IPC 注册表移除 `open_about`、「更新常驻入口」
  条目改写为裁定后状态；关于面板此后只能经菜单（macOS）/ 托盘（非 macOS）打开。
  仅周知，无需动作。
- 凭据：`cargo test` 全绿（89 passed）；diff −36/+6 已逐行人肉复核；本条视为对
  该次宪法修订的知会。

### 2026-09-07 完成通知：boot 链路修复批 + 跨平台测试面建立 —— guan（AI 起草）

- 变更：8 个 commit（`1daf6ad`..`6894bf7`）——① 引导进度回调阶段化（node 字节/
  dsh 包计数）+ add -g 流式进度解析 + 完成关单补发（修 99% 卡死）；② boot 步骤
  链生命周期（step0 即刻收口 / step2 done / step3 running，修双「运行中」与
  前后步倒挂）；③ Windows 潜伏编译错误修复（ensure_guest_engine 归位 + Path
  导入——master Windows CI 自 0dae87c 起红，本地 windows-gnu target check 抓出）；
  ④ SIGTERM/SIGINT 优雅退出收会话子进程（修孤儿进程）；⑤ 下载进度卡 dsh 形态 +
  ETA 亚秒隐藏 + 启动页撤通栏顶栏（双下巴）；⑥ 控制中心冗余检查更新入口；
  ⑦ repro-boot-scenarios.sh 补 no-runtime/corrupt-dsh/readonly 场景 + auto 全
  矩阵自动化（macOS 10 场景 PASS）；⑧ 新增 boot-smoke.yml 三平台 boot 端到端
  冒烟（仅 workflow_dispatch，重大变更后手动跑；日常 push/tag/release 不跑）。
- 影响：①②⑤ 为用户可感 boot 体验修复；③ 修复后 Windows CI 恢复可绿；
  ④ 补齐 §6「壳与 dsh 严格 1:1」在信号终止路径的缺口；⑦⑧ 为测试面扩充，
  非宪法级（AGENTS 未动）；Linux 无托盘宿主时 setup_update_tray 的 `?` 仍会
  中断启动（CI 侧 dbus-run-session 已绕过）——产品级降级策略待裁定另行开工。
- 凭据：cargo test 161 绿 / clippy -D warnings 干净 / fmt 过；windows-gnu
  target check 绿；前端 typecheck/lint/vitest（105）绿；macOS 场景矩阵 10/10
  PASS；pkill 孤儿检查实证；boot-smoke 三作业首跑待远端验证。

### 2026-09-07 补记 · 三项裁定落档：boot-smoke 实验性 + 托盘降级 + 修复链 Windows 韧性 —— guan（AI 起草）

- 变更：boot-smoke 六轮实证 GitHub hosted Windows runner 环境层启动不稳定
  （stderr/WER 干净、同代码同 env 结果随机），Windows 双作业标
  continue-on-error 实验性不挡闸门（`d4c3adf`）；Linux 作业四连 PASS 维持
  有效闸门。连带裁定三项：① `repair-session.mjs` 主修复路径 rename 加
  EPERM 退避重试（杀软/索引器瞬态锁，重试+退避，穷尽才报错）；② Linux
  托盘初始化失败降级 warn 不阻断启动（i3/sway/精简桌面可正常用壳，
  ADR-0007 边界澄清：托盘是首选常驻更新入口，但入口缺失 ≠ 应用不可用）；
  ③ Windows/WSL 确定性 boot 验证暂缓——走 docs/executor.md 手动清单，
  逻辑层由三平台 build 覆盖，待实机或 self-hosted runner 再收口。
- 影响：② 为 ADR-0007 的边界澄清（非推翻：托盘仍是首选入口），仅周知；
  ③ boot-smoke 的 Windows 作业结果仅作诊断参考，不作为任何合入闸门。
- 凭据：cargo test 162 绿 / clippy 干净 / fmt 过；node --check 过；macOS
  场景矩阵 10/10；Linux smoke 五连 PASS；build 三平台全绿。

### 2026-09-07 发版：v0.9.5 —— guan

- 变更：自 v0.9.4 起的累积发布——boot 体验批（引擎进度阶段化 + dsh 流式
  进度 + 99% 卡死修复、步骤链生命周期、启动页撤通栏顶栏）、Windows 潜伏
  编译错误修复（ensure_guest_engine 归位）、SIGTERM/SIGINT 优雅退出收孤儿、
  sessions 四项 Windows 单测兼容、托盘缺失降级 warn、修复链 rename EPERM
  退避重试、CI 闸门修复（失效 pin / fetch pnpm 顺序）与 boot-smoke 三平台
  冒烟建立（Windows 侧实验性）、repro-boot-scenarios 场景矩阵。详见同日
  两则落档与 commit 列表（1daf6ad..HEAD）。
- 影响：发版；tag 构建走签名安装器 + 更新 feed。
- 凭据：build 三平台全绿（34106505184 起持续）；smoke Linux 五连 PASS；
  Windows 冒烟实验性（环境层不稳定，见同日补记）。

### 2026-09-07 宪法外登记：pnpm 12 构建审批门产品化（ADR-0009 第六次修订）—— guan

- 背景：v0.9.5 引擎档 pnpm 12.3.1 上线当日，插件安装撞
  `ERR_PNPM_IGNORED_BUILDS`——pnpm 12 装完包后对未获批安装脚本的依赖硬失败
  退出 1，并把 allowBuilds 裁决模板写进 profile 的 pnpm-workspace.yaml；
  dsh 转发链透传退出码并跳过 bundle 调和（复现点 12 已入册）。
- 裁定（维护者选项 A）：壳把 dsh/pnpm 文档化的人工出路产品化——失败解析被
  点名包 → 前端逐包裁决框（默认跳过）→ 新 IPC `set_profile_build_approvals`
  受控改写 allowBuilds 单键（pnpm-workspace.yaml **写入例外 #5**，非三件套
  成员；非裁决条目逐字节保留，流式/嵌套/引号键拒绝写入）→ 自动重试原操作。
  AGENTS §6 不变量行 + §7 IPC 清单已同步登记。
- 影响：插件安装/更新/卸载失败面收敛为可自愈流程；allowBuilds 成为壳的第
  五个 profile 受控写点；真实 true/false 取值是用户供应链决定，壳不预填。
- 凭据：cargo test 176 绿（含真实 web profile yaml 全文 fixture）+ clippy
  干净 + fmt 过 + Windows 交叉 check 过；前端 typecheck/lint/test 全绿
  （buildApprovals 纯逻辑 4 例）。

### 2026-09-08 问题记录095 批 1：会话误判回归修复 + 控制中心反馈面治理—— guan

- 修复 ①：engine_session_alive 被 19ec36e（SIGTERM 批）机械改写为
  is_none_or 致语义反转（无会话=存活），叠加 mtime 窗口把刚收尾会话误标
  「进行中」；恢复 is_some_and + 脚本侧 active 增订 endState==='open'
  降噪子句（漏标两轮间隙的代价经裁定接受）。
- 修复 ②：MCP 面板「无限刷新 + 报错轰炸」——onNotice 内联引用不稳 +
  effect 依赖 loadData + catch 弹 toast 构成失败循环；改面板内错误态 +
  重试，运行态快照降级为辅助信息，删除无价值的刷新按钮。
- 治理 ③：全站刷新按钮清单裁定——删 2（Profile 顶栏/MCP）、补语义标签
  2（日志拉取/凭据重读，去硬编码走 i18n）、换图标 2（检查更新/更新插件
  停用 RefreshCw 防混淆），其余 9 处保留（错误重试/强刷绕缓存/编辑器
  重载均有真实语义）。
- 凭据：cargo test 177 绿 + fmt/clippy/Windows 交叉 check 过；前端
  typecheck/lint/109 测试全绿。问题记录095 剩余两项（插件边界梳理、
  下载队列）设计中，另批开工。

### 2026-09-08 发版：v0.9.6 —— guan

- 变更：自 v0.9.5 起累积——pnpm 12 构建审批门产品化（安装撞
  ERR_PNPM_IGNORED_BUILDS → 逐包裁决对话框 → allowBuilds 受控写入
  （写入例外 #5）→ 自动重试，ADR-0009 第六次修订 + 复现点 12）；
  会话「进行中」误判回归修复（engine_session_alive 反转 +
  endState 降噪子句）；MCP 面板失败死循环根治（面板内错误态）与
  全站刷新按钮语义化治理（删 2/补 2/换 2/留 9）。
- 影响：发版；存量用户经更新 feed 收提示。挂账：插件中心职责边界
  （问题记录095 #3/#4）留下个小版本。
- 凭据：cargo test 177 绿 + fmt/clippy/Windows 交叉 check 过；前端
  typecheck/lint/109 测试全绿；tag 构建验证随 CI。

### 2026-09-08 fix(boot)：selector 页双下巴清尾 + 悬浮胶囊误挂壳页面根治 —— guan

- 双下巴残留：af0e3ee 只撤了启动页通栏顶栏，selector 页漏网——原生标题栏
  之下仍叠一条导航（wordmark/SELECTOR 徽章/控制中心/版本芯片）。同口径
  撤除；控制中心入口经菜单/托盘/全局快捷键可达，boot 期页面不再重复承载。
- 根因补刀：截图里 selector 顶上多出的第二颗「控制中心」是 Rust 注入的
  悬浮胶囊——回环 hostname 判断在 macOS 被壳页面 origin（tauri://localhost，
  hostname 恰为 localhost）误命中，selector/启动页/控制中心/关于页全部
  误挂。挂载条件收紧为「location.origin 与 get_workbench_url 精确比对」，
  origin 未就绪一律不挂；工作台页（http://127.0.0.1:*）行为不变。
- 凭据：前端 typecheck/lint/112 测试绿；cargo test 182 绿 + fmt/clippy
  干净（probe_no_open 首跑偶发失败，单跑与全量复跑均过——存量偶发，与
  本次改动无关，留观）；vite 预览目检 selector 页顶栏已消失。

### 2026-09-08 fix(plugins)：市场非 npm 来源安装必败修复 + 弹窗溢出治理 + 插件职责边界立宪（ADR-0011）—— guan

- 缺陷（用户实测复现 dsh-pet）：线上 registry 3363 条中 51.6%（1736 条）
  install spec 为 `github:owner/repo`（含 `#path:/子目录` 片段）或带成对引号的
  tarball 直链，`validate_plugin_spec` 白名单不含 `:` `#` 必拒——市场半数插件
  点安装即报「包名只允许字母数字与 @/._^~*-」。pnpm 原生支持三形态，
  唯一挡路点是壳的校验白名单。
- 修复：校验谓词拆二——安装/卸载/更新入口换 `validate_install_spec`
  （npm / github[:#frag] / https tarball 三形态；总长 512、npm 形态仍守 214；
  `#path:` 空值 fail-closed；仍拒前导 `-`、空白、`><` 区间）；更新检查与
  选版本保持严格 npm 判别（防拿 github 形态打 packument）。前端预检镜像
  同步；registry 提取层剥离成对引号；市场弹窗提交前预检。恶意反例入测。
- UI：`DialogContent` 基类无 max-h 无滚动，fixed 居中定位下视口不足即上下
  两端溢出且不可滚动救援——补 `max-h-[calc(100dvh-2rem)] overflow-y-auto`，
  全站弹窗受益。
- 职责边界立宪（ADR-0011）：跨 profile 归插件中心、单 profile 归详情；
  安装来源三形态两面对齐。CONTEXT.md 新增插件领域词汇四条；AGENTS §9
  登记索引；roadmap 4.4 落地记录补 09-08 行。
- 小版本挂账：分发 github 包改从来源 package.json dependencies 取 spec；
  跨 profile 更新检查 + 批量更新 + 下载队列（问题记录095 #4）；动词统一
  「安装到…」/Tab 名「插件」/删 ProfileDetailDialog.tsx 死代码。
- 挂账实测：git dep 的安装/更新/卸载需 dev 环境各实测一轮（update 对
  git dep 是否重解析默认分支存疑）。
- 凭据：cargo test 184 绿 + fmt/clippy 干净；前端 typecheck/lint/117 测试绿。

### 2026-09-08 fix(uiux)：静默失败与引用不稳止血（UI/UX 评审批次 A：U1/U3/U4/U5）—— guan（AI 起草）

- 依据：`docs/uiux-review-2026-09-08.md` 批次 A（「用户看到的与事实相反」类）。
  ADR-0011 合入 `413570d` 后其文件面占用释放，本批为释放后的首个前端改动。
- U1 BLOCKER（5 个面板无限重试 + 无限弹 toast）：`ProfileManager` 曾以**内联箭头**
  传 `onNotice` → 每次渲染新引用 → 子面板 `useCallback([onNotice])` 失效 →
  `useEffect` 重跑 → 失败又通知 → `setToast` → 父重渲染 → 死循环。三处改传稳定引用
  `showToast`，并加裁定注释说明为何禁内联（McpManager 已踩过同坑）。
- U3 HIGH（诊断页伪造结论）：采集失败时 `report ?? {全缺失}` 渲染出「Node/pnpm/dsh
  全部未检出」的**伪造环境损坏报告**。改为失败且无缓存 → 报错块 + 重试，伪造分支删除。
- U4 HIGH（settings.json 键丢失）：`set_shell_settings` 是**整体覆盖写**
  （`settings.rs::save` 序列化全字段、无 merge），而 `PreferencesPane` 与
  `i18nStore.setLocale` 都在读取失败时以 `{}` 为基线回写 → 清空 `defaultProfile` /
  `locale` / `dismissedUpdate` 等未参与本次修改的键。抽 `lib/shellSettings.ts`
  安全基线：基线必须来自一次**成功**读取，读失败即中止（不写）；两处调用点收口；
  设置页读取失败改为渲染错误块 + 重试并停用保存。
- U5 HIGH（迁移假成功）：分发时勾了「迁移配置」但 `copyPluginConfig` 失败/未覆盖，
  仍弹「分发完成」。改为按实际结果通知（失败/未覆盖 → warn 文案）。
- 影响：仅周知。**遗留两条**已登记问题记录：① `setLocale` 存储失败仍 resolve →
  设置页会弹「设置已保存」（假成功 toast，批次 B）；② Rust 侧 `set_shell_settings`
  无 merge 语义，前端安全基线只是止血，任何新调用点仍可能踩坑——建议后续在
  `settings.rs` 补 merge 或收窄命令面（收窄属契约改动 → 先 ADR）。
- 凭据：`pnpm typecheck` 0 err · `oxlint` 0 warning · `pnpm test` **123 passed**
  （117 → +4 `shellSettings` 纯单测 +2 `onNoticeStability` 源码闸门）。
  复现先行：安全基线改回 `read().catch(() => ({}))` → 首条即红（已验证）；
  `onNoticeStability` 对修复前 HEAD 复算命中 `ProfileManager.tsx:258,263,268`，
  修复后 0 命中（已验证）。U1 缺陷对 tsc/oxlint 不可见（类型一致），故用与
  `ipc.rs` gate_tests 同口径的源码文本闸门兜底（`?raw` 读入，不引 DOM 测试栈）。

### 2026-09-08 fix(a11y)：键盘与读屏可达性（UI/UX 评审批次 B1）—— guan（AI 起草）

- 依据：`docs/uiux-review-2026-09-08.md` 批次 B。本次只做「能操作、能听见」两类，
  表单 label 关联 / 标题层级 / tab 语义 / 动效降级留批次 B2。
- U2 BLOCKER（主操作键盘不可达）：`ProfileRow` 整卡 `div onClick`。**卡片不能整体
  改 `<button>`**——内部还有启动/重启/更多菜单按钮，嵌套交互元素既非法又让读屏
  语义错乱。改为「名字即主控件」：名字变 `<button>` + `aria-current`，卡片用
  `focus-within:ring-2` 呈现整卡焦点环，指针点击行为不变。
- toast 播报：`role="status" aria-live="polite" aria-atomic` 必须挂在**常驻**容器上
  ——挂在随 toast 挂载/卸载的节点上，部分读屏会整条漏播；容器恒在 DOM，动画只作用
  于内层 motion.div。顺带补 `title`（消息 truncate 到 420px，长错误原本看不全）。
- 8 个 `Switch` 全部补 `aria-label`（评审记 7 处，实际含 `BootMode` 共 8 处；后者靠
  `<label>` 包裹关联，仍补显式名称）。
- 4 个图标按钮补名称：MCP 删除（原**无任何名称**）、MCP 删 ENV 行（原只有「×」）、
  市场清空搜索、ProfileRow 重启（原仅 `title`）。
- **机器闸门（新增）**：`ui/switch.tsx` 的 `SwitchProps` 改为类型层强制——开关必须带
  `aria-label` 或 `aria-labelledby`，漏写即 `pnpm typecheck` 红；`__tests__/switchA11y.test.tsx`
  用 `@ts-expect-error` 钉住闸门存在性（把类型退回全可选立即报 `TS2578`，已验证）。
  类型闸门优于源码文本闸门：不误判、不依赖路径、覆盖未来所有新调用点。
- 影响：仅周知。**实机验证清单待跑**（Tauri 壳需真实窗口，DOM 测试栈按 AGENTS §5 禁用）：
  Tab 走 Profile 列表 → Enter 切换详情；失败 toast 读屏播报；键盘走查 MCP 删除 /
  市场清空搜索 / 日志自动滚底开关。清单见评审档 §13。
- 凭据：`pnpm typecheck` 0 err · `oxlint` 0 warning · `pnpm test` **125 passed**
  （123 → +2 `switchA11y` 类型闸门测试）。

### 2026-09-08 test(ipc)：契约「形状」跨语言闸门（架构评审批次 1）—— guan（AI 起草）

- 依据：`docs/architecture-review-2026-09-08.md` 批次 1（评审结论：若只做一件就做它）。
  P4「名字有闸门、形状没有」——Rust 改字段名 → TS 静默 `undefined`，编译绿、测试绿、
  运行时错，是「编译绿、运行错」的高危面。
- 新增三个闸门（`ipc.rs::gate_tests`）：
  1. `tauri_ts_matches_ipc_commands`——`lib/tauri.ts` 的 invoke 名集 ↔ `COMMANDS` 双向。
     前端是第 5 个消费面，此前完全无闸（今日实测 55/55 手工一致）。
  2. `no_direct_invoke_outside_tauri_ts`——递归扫 `frontend/src`，`invoke(` / `invoke<`
     只允许出现在 `lib/tauri.ts`（AGENTS §4.3「组件内不直接 invoke」）。当前 0 命中。
  3. `ipc_struct_shapes_match_fixture`——14 个 IPC 结构体的**真实 serde 序列化 key 集**
     ↔ 共享 fixture。
- **共享 fixture = 唯一事实源**：新增 `frontend/src/types/ipc-shapes.json`。Rust 侧
  `include_str!` 读入比对真实序列化结果；前端侧 `__tests__/ipcShapes.test.ts` 用
  `AllKeys<T>`（`Required` 展开 + 多余属性检查）断言 TS 接口 key 集与同一份 fixture
  一致。**任一侧改名/换 casing 都会红**。
- 覆盖（14）：`ShellSettings`·`ProfileSummary`·`SessionItem`·`PluginRowState`·
  `AggregatePlugin`·`AggregateSource`·`CopyConfigOutcome`·`RepairOutcome`·
  `SystemDiagnosticsReport` + 5 个诊断子结构。
- 命名风格**保持现状并闸住**：profiles/plugins 域 snake_case（`web_ui`/`pkg_name`/
  `skipped_existing`），sessions/settings/diagnostics 域 camelCase。统一 `rename_all`
  属契约变更未做（须先裁定）；但两侧已钉死，再改名必须同时改 fixture。
- 未纳入：`emit_step` 的 `json!` 手拼 payload 换结构体（随批次 2 拆 `lib.rs` 一并做）。
- 实测抓漏（复现先行）：改 `tauri.ts` 的 `get_shell_settings` 名 → 闸门 1 红；
  改 fixture 的 `web_ui`→`webUi` → Rust 闸门 3 红 **且** 前端 `ipcShapes` 红。
- 凭据：`cargo test` **187 passed**（184 → +3）· `cargo fmt --check` 干净 ·
  `clippy --all-targets -D warnings` 干净 · `pnpm typecheck` 0 err · `oxlint` 0 warning ·
  `pnpm test` **131 passed**（125 → +6）。

### 2026-09-08 chore：清理惰性守卫、死代码与同义反复测试（架构评审批次 0a）—— guan（AI 起草）

- `build.rs` 删 5 条 `../ui/*` rerun 监视：`ui/` 早已不存在，是 2026-08-23「复用旧前端」
  事故留下的惰性守卫；**上游已覆盖**——`tauri-build` 2.6.3 自己就为 `frontendDist`
  （`src/codegen/context.rs:87-95`）与 `capabilities/`（`src/acl.rs:427`）发 rerun 指令。
  留着只会让人以为守卫还在。
- `scripts/regen-icons.sh` 三条 `ui/assets/dsh-logo.svg` 死路径 → `assets/dsh-logo.svg`
  （`frontend-migration.md:66` 早已记录搬移，脚本是漏网）。
- 删 `ProfileDetailDialog.tsx`（671 行、全仓 0 引用；ADR-0011 §5 行动项「随小版本批」）。
- 删 4 个同义反复测试（`credentials`/`diagnostics`/`mcp`/`sessions`）：仅 `import type`，
  在测试体重写逻辑后断言字面量，**从不触达生产代码**——假覆盖率比没有覆盖率更危险。
  被覆盖的纯逻辑仍在组件内，替代路径是「下沉 `lib/` 再写真测试」（归 P7 后续批次）。
- 文档漂移：`tauri.ts:1`「20 个命令」（实为 55）改为 rot-proof 表述（指向
  `ipc.rs::COMMANDS` + 双向闸门）；`frontend-migration.md` 加计数口径补注
  （该文是迁移期记录，「12 个命令」是当时数量）。
- 影响：仅周知。**未纳入**：10 处 clipboard promise 假成功（批次 0b）。
- 凭据：`cargo build` 通过 · `pnpm typecheck` 0 err · `oxlint` 0 warning ·
  `pnpm test` **120 passed**（131 − 11 条假测试）。





### 2026-09-08 fix(frontend)：剪贴板写入唯一入口（架构评审批次 0b）—— guan（AI 起草）

- 缺陷：11 处各自直呼 `navigator.clipboard.writeText(...)`，处理方式分四种——
  完全不接 promise（`ErrorCard` 写失败仍显示「已复制」）、`.catch(() => {})` 后照样
  置位（`BootStep`）、只挂 `.then` 成功分支（8 处，无失败反馈 + unhandled rejection）。
  **更正评审原判**：原文写「`BootStep.tsx:34` 是唯一正确写法」不成立——它吞掉拒绝后
  仍立即置位，与其余各处同病；另外 `SessionManager` 还有一处复制路径（`:219-225`）
  原表漏记，实为 11 处。
- 修法：新增 `lib/clipboard.ts::writeClipboard`（`write` 可注入 → 纯逻辑可测）+
  `hooks/useCopy`（成功才置位、自动复位、不收回调参数以免引用不稳，同 onNotice 裁定）。
  11 处改为 `await copy(...)` 后按结果分流：有 toast 渠道 → `onNotice(t.error.copyFailed)`，
  boot 页无 toast → `logger.warn`。
- 机器闸门：`__tests__/clipboardGate.test.ts`（`import.meta.glob(?raw)` 扫全量源）
  断言 `clipboard.writeText` 只允许出现在 `lib/clipboard.ts`；探针文件实测被抓出。
- 影响：仅周知。
- 凭据：`pnpm typecheck` 0 err · `oxlint` 0 warning · `pnpm test` **126 passed**
  （120 → +4 `clipboard` 单测 +2 闸门）。

### 2026-09-08 fix(a11y)：表单标签与标题层级（UI/UX 评审批次 B2）—— guan（AI 起草）

- 表单标签 12 处：8 处搜索框/凭据框只有 placeholder 当标签（placeholder 不是无障碍
  名称，输入后即消失）→ 补 `aria-label`（搜索框复用同一 i18n 键，凭据框新增
  `console.keyInputLabel`）；`McpManager` 服务器名/命令/参数 3 处 `<label>` 无
  `htmlFor` → 补 `id` + `htmlFor`；ENV 组标题与市场安装源只读框的 `<label>` 指向
  非控件 → 改 `<span>`（ENV 容器加 `role="group"` + `aria-label`）；两处 Radix Select
  的 `<label>` 无法关联 → `SelectTrigger` 加 `aria-label`。
- 标题层级：页面 h1 → 面板/节 h2 → 卡片/子块 h3。面板标题 h3→h2（凭据/引擎设置/
  诊断/偏好×4/会话/MCP/详情空态/客户端更新卡）；`DiagnosticsPane` 5 个指标卡 h4→h3
  （否则 h2 后跳 h4）；市场与已装总览的卡片 h3 缺父级 h2 → 补 `sr-only` h2。
  **更正评审原判**：`ProfileDetailPane.tsx:312 h3 先于 :327 h2` 实为空态与选中态两个
  互斥分支，不存在渲染顺序问题，仅把空态标题对齐到 h2。
- 机器闸门：`__tests__/formLabels.test.tsx`（`import.meta.glob(?raw)` 扫全量 .tsx）
  断言每个 `<input>` 至少有 aria-label/aria-labelledby/id/`<label>` 包裹/type=hidden；
  探针实测被抓出。正则把 `=>` 当整体跳过，避免 onChange 箭头把标签截断。
- 影响：仅周知。**B2 残留**：tab roving tabindex/aria-controls、prefers-reduced-motion、
  命中区 <24px、「仅靠 title 命名」15 处。
- 凭据：`pnpm typecheck` 0 err · `oxlint` 0 warning · `pnpm test` **128 passed**（126 → +2）。

### 2026-09-08 refactor(uiux)：字号刻度、断点对齐、页头吸顶、会话行去重（评审批次 D）—— guan（AI 起草）

- U7 字号 token 化：`@theme` 新增 5 档（`--text-micro/meta/label/note/lead` =
  9/10/11/13/15px，**逐像素等同改造前**，避免视觉回归）；161 处 `text-[Npx]` 全量
  替换（38 个文件）；`ui/button.tsx` 的 `text-[0.8rem]`→`text-xs`（12.8→12px，sm 按钮，
  视觉无感）。闸门 `__tests__/fontTokens.test.ts` 禁止再出现任意字号。
- U8 主从布局断点 `lg`(1024)→`md`(768)：`ProfileManager` 与 `SystemConsole`。窗口
  最小宽度 860/960 均 ≥ 768 ⇒ **允许的任何窗口尺寸下都不再塌成单列**。只改断点、
  不动窗口尺寸（评审 §10 明令「不在同一 PR 里同时改断点与窗口尺寸」）。
- U10 页头吸顶：`ProfileManager` 页头 `sticky top-0 z-20` + `bg-bg/90 backdrop-blur-sm`，
  负外边距抵消 `PageShell` 的 px-4/6/8；长列表滚动后视图切换入口常驻。
- U14 会话行去重：右侧操作区**只留动作**——删掉与左徽标重复的「运行中」「健康/未知」
  胶囊（原来三选一渲染），仅在「需修复且非运行中」时给修复按钮，修复/复制/删除不再被挤。
- 影响：**需人工目检两点**（Tauri 需真实窗口，本次未实机验证）：① 860px 宽时左栏
  Profile 卡片不挤（4/12 栏 ≈ 280px）；② 吸顶页头滚动时不遮首行内容。
- 凭据：`pnpm typecheck` 0 err · `oxlint` 0 warning · `pnpm test` **130 passed**（128 → +2）·
  `pnpm build` 通过（确认 5 档 token 生成 `.text-meta` 等工具类）。

### 2026-09-08 fix(uiux)：对比度达标与 dark 变体清理（评审批次 C）—— guan（AI 起草）

- U6 对比度：实测「既 ≥4.5:1 又能与 `dim` 明显区分」的浅灰不存在（浅底上需 ≈#656d7c，
  与 dim(#626a7a) 已无差别），故**三级灰阶压缩为两级**：`--color-faint` #a0a7b6→**#6b7280**
  （bg 4.55 / panel 4.83 / line-soft 4.27，最后一项为记录在案的边际值）；
  `--color-ok` #2f9e44→**#27793a**（3.45→5.41）；`--color-warn` #d9480f→**#c2410c**（4.30→5.18）。
- 品牌蓝**降级为图形象**（边框/描边/色块/点，3:1 适用）：`text-brand`→`text-brand-deep`
  （98 处）、`bg-brand text-white`→`bg-brand-deep text-white`（15 处）、shadcn
  `--primary`/`--destructive` 同步。
- 原始调色板同步修正：浅底文字色统一 -700 档（amber-500 2.15→5.02、emerald-500
  2.54→5.48、sky-500 3.0→5.93、rose-500 3.4→6.29、violet/indigo-500→-700；含 /70 变体
  改实色）；`bg-amber-500 text-white`（修复按钮，白字 2.15）→`bg-amber-700 text-white`。
  深底文字（toast/日志终端）保持 -300/-400 档不动。
- U12 `dark:` 去留 → **删 40 处**：全仓无 `.dark` 应用点（不可达），且与 index.css
  已记录的暗色方案（「追加 `.dark` 覆盖语义变量，组件零改动」）冲突——留着是地雷，
  一旦启用会以原始调色板破坏 token 体系。
- 机器闸门：`__tests__/contrast.test.ts`（5 条）——文字色 token ≥4.5、白字在填充 token
  上 ≥4.5、faint 的 line-soft 边际 ≥4.2、源码禁 `text-brand`、源码禁 `dark:`。
  为此 `vite.config.ts` 增 `test.css: true`（Vitest 默认把 CSS 桩成空串，`?raw` 读不到）；
  配置改用 `vitest/config` 的 `defineConfig` 以获得 `test` 键类型。
- 影响：**需人工目检**配色观感（尤其 faint 变深后的层级、修复按钮由黄转深橙）。
- 凭据：`pnpm typecheck` 0 err · `oxlint` 0 warning · `pnpm test` **135 passed**（130 → +5）·
  `pnpm build` 通过。闸门实测抓漏：把 faint 改回 #a0a7b6 → 两条断言即红。

### 2026-09-08 fix(uiux)：图标与动词统一、Select 截断、aria-label 收口（评审批次 E）—— guan（AI 起草）

- U13 图标错配 4 处：`DownloadCloud`→`RefreshCw`（检查更新不是下载）、`Code2`→
  `ExternalLink`（打开 GitHub 是外链）、`Trash2`→`Eraser`（清屏非破坏性）、
  `Clipboard`→`Copy`（同行复制路径用 Copy，复制 ID 却用 Clipboard）。
- U13 动词统一：所有刷新类动作以「刷新」起头——刷新扫描→刷新会话、拉取最新日志→刷新日志、
  重新读取凭据→刷新凭据、市场失败态重新加载→重试；复制标签去修饰词（复制完整诊断报告→
  复制诊断报告、复制全部日志→复制日志）；引擎设置面板「复制代码」→「复制 YAML」（内容是 YAML）。
- U15 Select 截断：`ui/select.tsx` 的 `SelectItem` 对纯文本子节点自动挂 `title`；
  `PluginOverview` 筛选 Select `w-48`→`w-56` + trigger 补 `title`（原长 profile 名被截成
  「只显示 3 个字符」且无法悬停读全）。
- 中文 `aria-label` 13 处 → i18n（读屏在英文环境不再读中文）。
- 影响：仅周知。**i18n 正文抽取拆为批次 E2**（实测 2,876 字 / 45 文件，其中 6 个大文件
  60 条可枚举文案 + 模板串 + 17 个较小文件；纯机械但横跨 25+ 文件，按 §8.1 单独立项）。
- 凭据：`pnpm typecheck` 0 err · `oxlint` 0 warning · `pnpm test` **135 passed**。

### 2026-09-08 refactor(tauri)：注入脚本迁出 Rust 原始字符串（架构评审批次 3）—— guan（AI 起草）

- 缺陷（P3）：330 行 JS 活在 Rust `r#"…"#` 里——tsc/oxlint/语法检查全部看不见，改错只能
  等运行时。
- 落地：三段全部迁到 `frontend/src/injected/*.js`，Rust 侧改 `include_str!`（同
  `updater.rs:461` 跨语言引用范式）：`memory-policy.js`（68 行）、`link-hook.js`（24 行）、
  `switcher.js`（238 行）。`lib.rs` 3,039 → 2,713 行（−326）。
- **迁出即见真章**：工具链首次扫到这 330 行立刻报 6 条——`memory-policy.js` 的
  `scan(root)` 是死代码（MutationObserver 已内联同逻辑、从未调用）、`switcher.js` 的
  `var isMac` 赋值未用、4 处 `catch (e)` 未用参数。已一并清理（改 `catch {}`）。
- 放 `frontend/src/injected/` 的理由：`pnpm run lint` 覆盖该目录 → 即被 oxlint 纳管；
  Vite 只打包被 import 的模块 → 不进产物；tsconfig 未开 `allowJs` → tsc 不误编译。
- 影响：仅周知。运行期行为不变（脚本内容逐行迁移，仅删死代码与未用参数）。
- 凭据：`cargo test` 187 绿（含内存策略三条子串断言）· `cargo fmt --check` 干净 ·
  `node --check` ×3 通过 · `pnpm lint` **0 warning**（迁出前这 330 行是 lint 盲区）。

### 2026-09-08 test(rust)：去 flaky + 强制 node 用例 + 网络面离线覆盖（架构评审批次 5）—— guan（AI 起草）

- 去 flaky（P8）：`resolve.rs` 三处挂钟断言过紧（`finds_flag_on_stderr` 的 `< 5s` 并行全量
  跑实测挂过一次）。口径改为「区分命中早退与等满自然退出」——假体 sleep 拉到 60s，
  断言留 3–6× 负载余量（10s/20s），回归形态必超。
- 静默跳过 → CI 硬失败（P6）：`sessions.rs` 4 个修复链用例缺 `node` 时 `return`（0 断言
  通过 = 假绿）。新增 `require_node_or_skip()`：本地允许跳过并打印提示，CI 由
  `DSH_TEST_REQUIRE_NODE=1` 强制 panic；`.github/workflows/build.yml` Unit tests 步骤已挂。
  **实测**：剥掉 PATH 里的 node + 打开开关 → 立即 panic。
- 网络面离线覆盖（P6）：`updates.rs::read_body_capped`（ureq 上限漂移的显式替代实现）
  此前零测试 → 补 5 条（未超限/恰好等于上限/超限文案/空体/非 UTF-8 上下文）。4 → 9 条。
- **欠账**：HTTP seam 注入（离线覆盖镜像链回退/超时）、`repair-session.mjs` fixture 驱动。
- 凭据：`cargo test` **192 passed**（187 → +5）· `cargo fmt --check` 干净 ·
  `clippy --all-targets -D warnings` 干净 · `DSH_TEST_REQUIRE_NODE=1` 下全量绿。

### 2026-09-08 ADR-0012 立项：启动失败错误类型化（架构评审批次 4 前置）—— guan（AI 起草）

- 依据架构评审 P5：`classify_boot_error`（lib.rs）把错误文本小写后做子串匹配来决定
  错误卡标题/建议/按钮——**措辞即契约**（上游改文案即静默落兜底）、**同词不同因**
  （任何含 timeout 的错误都判「网络不可用」）、**不可穷尽**（新增失败模式不触发编译错误）。
- ADR-0012 决策：只为 **boot 失败路径**引入 `BootFailure` 枚举（含 `Unknown { detail }`
  兜底），`classify_boot_error` 改造为 `from_legacy_detail` 映射表（与今天子串规则逐条等价
  并纳入离线单测）；其余 110 处 `Result<_, String>` 本次不动（§8.2 增量）。
  备选：B 强化分类器（措辞仍是契约，治标）／C 全仓类型化（一次性大改）／
  D 引 thiserror/specta（新依赖 + 与既有形状闸门重复）——均已记录否决理由。
- 状态：**草案，待维护者评审**；行动项 4 条在 ADR §5（枚举 + 等价映射表单测 → boot 路径
  改造 → 前端结构化分支 + `ipc-shapes.json` 登记 → AGENTS/台账回收）。
- 影响：AGENTS §9 索引已加 ADR-0012 一行（宪法级文件，改动即本广播）。**代码未动**——
  §9 要求先立 ADR 再动代码，实施待评审通过后另起一批。
- 凭据：纯文档（ADR 91 行）。

### 2026-09-08 refactor(tauri)：拆出 commands 模块（架构评审批次 2 第一步）—— guan（AI 起草）

- **`lib.rs` 2,713 → 1,778 行（−935）**：55 个 `#[tauri::command]` 按域迁到
  `src-tauri/src/commands/`——boot(5) / profile(10) / plugin(12) / console(13) /
  session(4) / update(5) / link(3) / window(2) / market(1)，共 9 文件 947 行。
- **先修闸门再搬**：`ipc.rs` 的 handler 解析器原把条目当裸标识符，搬完会变成
  `commands::profile::list_profiles` 而误判「未登记」→ 先改为取最后一段路径，并补
  `handler_parser_strips_module_path`（合成源码正反例）。搬完 55 条 handler 全绿。
- 可见性：`ShellState` 与 17 个被命令调用的辅助函数、`EXTERNAL_URL_HOSTS` 改 `pub(crate)`；
  命令层只依赖 `crate::…`，不反向依赖 `commands::`（lib.rs 仅菜单回调一处改为全路径）。
- **未纳入**：`ui/`（create_main_window 352 行 + 菜单/托盘/窗口）与 `boot/`
  （run() 400+ 行 + 会话守卫 + emit_* 族）。本次只搬叶子层（命令 → 域模块），
  不动启动管线，回归面可控。
- 影响：仅周知，无行为变更（纯搬迁；IPC 名集/形状/契约均未动）。
- 凭据：`cargo test` **193 passed**（192 → +1 解析器用例）· `cargo fmt --check` 干净 ·
  `clippy --all-targets -D warnings` 干净 · 前端 `typecheck`/`lint` 0 warning/`test` 135 全绿。

### 2026-09-08 refactor(tauri)：拆出 ui 模块（架构评审批次 2 第二步）—— guan（AI 起草）

- **`lib.rs` 1,778 → 1,372 行**（相对原始 3,039 已 −55%）：窗口/菜单/托盘层迁到
  `src-tauri/src/ui.rs`（423 行）——`create_main_window`（含三个 initialization_script
  装配与导航拦截）、`resolve_resources_dir`、`build_app_menu`、`build_tray_menu`、
  `setup_update_tray`、`refresh_app_menu`（`#[cfg]` 双实现）、`current_active_mode`、
  `open_about_window`；注入脚本常量 `WEBVIEW_MEMORY_POLICY_SCRIPT` 随之下沉（只被
  `create_main_window` 用），`lib.rs` 测试引用改 `crate::ui::…`。
- 依赖方向单向：`ui::` → `crate::{ShellState, is_allowed_external_url, settings, emit_*}`；
  `lib.rs::run` 与 `commands::window` 调用 `ui::create_main_window`。
- **未纳入（最后一步）**：`boot/`——`run()`（400+ 行）、`ShellState` 与会话守卫、
  `emit_*` 族、错误分类、`init_tracing`/`TeeWriter`。这一步动启动管线本身，需单独一轮
  并逐段验证。
- 影响：仅周知，无行为变更。
- 凭据：`cargo test` **193 passed** · `cargo fmt --check` 干净 ·
  `clippy --all-targets -D warnings` 干净 · 前端 `typecheck`/`lint` 0 warning/`test` 135 全绿。

### 2026-09-08 refactor(tauri)：拆出 boot 模块（架构评审批次 2 第三步 · 批次 2 收口）—— guan（AI 起草）

- **`lib.rs` 1,372 → 574 行**（相对原始 3,039 **−81%**）：启动管线迁到
  `src-tauri/src/boot.rs`（818 行）——`ShellState`（字段 pub(crate)）+ executor 会话
  拉起与 1:1 守卫、启动/切换/重启编排、boot 遥测（`emit_step`/`emit_upgrade`/
  `emit_boot_error`/`emit_update`/`refresh_update_ui`）、失败分类与日志刮取、
  `TeeWriter`+`MakeWriter`+`init_tracing`、`install_signal_exit_handler`+`SIGNAL_EXIT`、
  `BOOT_TIMEOUT`/`BOOT_STALL` 与状态缓存。
- **`run()` 有意留在 lib.rs**：它是组合根（装配 Builder / 注册 handler / 挂菜单托盘），
  只接线不承载领域逻辑。
- 最终模块地图：`lib.rs`(574) · `boot.rs`(818) · `ui.rs`(424) · `commands/`(9 文件 947) ·
  `injected/`(3 文件 330 JS) + 18 个既有域模块。`lib.rs` 从「全仓第一热点」变为薄入口层。
- 影响：仅周知，无行为变更（纯搬迁 + 可见性调整）。
- 凭据：`cargo test` **193 passed** · `cargo fmt --check` 干净 ·
  `clippy --all-targets -D warnings` 干净 · 前端 `typecheck`/`lint` 0 warning/`test` 135 全绿。

### 2026-09-08 test(rust)：网络面 HTTP seam 注入 + 镜像链离线覆盖（架构评审批次 5 欠账收口）—— guan（AI 起草）

- **P6 第一条缺口（唯一网络面几乎无测试）关闭**：`updates.rs` 的 `ureq::Agent` 调用收成
  `trait HttpGet`（`get_text`/`get_bytes`），生产实现 `UreqGet`，测试注入 `FakeHttp`
  （URL 子串 → 预置响应 + 调用顺序记录）——不触网、不起 mock server。
- seam 点 5 处：`fetch_packument_with` / `fetch_client_latest_with` /
  `npm_packument_versions_with` / `fetch_market_registry_with` / `fetch_node_map_with`。
  镜像链列表一并注入（生产传 `npm_registry_urls()`/`registry_chain()`，测试传固定链），
  避免用例跟着 `DSH_DOCK_NPM_REGISTRIES` 漂移。
- 新增 11 条离线用例：镜像链回退（坏 JSON / 传输错误 / 形状不符）、全失败报末错、
  非法包名零请求、市场 CDN → GitHub raw 回落、node 映射 packument→tarball 两步
  与 `%2F` 编码、`flate2`+`tar` 现造 tarball 覆盖双文件解包。
- **行为差异（有意，更严）**：`fetch_node_map` 旧 `.take(cap)` 静默截断 → 现显式超限报错，
  读失败回落次镜像（与 `read_body_capped` 同口径）。`dist-tags.latest` 缺失仍是既有
  `?` 提前返回语义，本次不动（已记账）。
- 影响：仅周知，无行为变更（对外 IPC / 契约未动）。
- 凭据：`cargo test` **204 passed**（193 → +11）· `cargo fmt --check` 干净 ·
  `clippy --all-targets -D warnings` 干净 · 前端 `typecheck`/`oxlint` 0 warning/
  `test` 135 全绿/`build` 通过（未动前端，仅回归确认）。
