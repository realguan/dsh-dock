// IPC 命令的请求/响应类型。形状锚定 Rust 结构体的 serde 序列化：
// - UpdateStatus / ComponentUpdate / NodeRuntimeInfo ← src-tauri/src/updates.rs
// - ClientUpdate ← src-tauri/src/updater.rs（tag="phase"，camelCase）
// Rust 结构体变更时同步本文件（壳前后端同仓同发，无跨仓漂移风险）。

/// 单个可升级组件的版本维度（dsh 本体 / 桌面客户端）。
export interface ComponentUpdate {
  /** 当前版本；检测失败为 null */
  current: string | null
  /** 可升级口径最高版（dsh = 稳定/rc；client = 官方最新） */
  latest: string | null
  /** latest > current */
  newer: boolean
  error: string | null
  /** 排序最高但不在升级口径内的预览版（alpha 等）；无或与 latest 同版为 null。
      仅 dsh 维度使用——「有新版」只按可升级口径判定，预览版经版本列表显式选择 */
  preview_latest: string | null
}

/// dsh 版本列表条目（形状锚定 src-tauri/src/updates.rs 的 DshVersionEntry）。
export interface DshVersionEntry {
  version: string
  /** stable / rc / alpha / other（other = beta 等其余预发布标签） */
  channel: "stable" | "rc" | "alpha" | "other"
  /** 与当前已装版本的相对关系（后端按 semver 比较，前端不做第二套比较器）；
      当前版本未检出时一律 newer */
  relation: "newer" | "current" | "older"
  /** 发布时间（RFC3339 原文）；registry 未记录为 null */
  published_at: string | null
}

/// DSH 版本列表响应（list_dsh_versions）。
export interface DshVersionsResult {
  /** 探测到的当前已装版本；未检出为 null */
  current: string | null
  /** 降序全版本 */
  versions: DshVersionEntry[]
}

export interface NodeRuntimeInfo {
  /**
   * **实测**版本（引擎 `engines/bin/node --version`）；未安装 = null。
   *
   * 2026-09-10 修复：此前这里是**下载计划**版本（恒非空），于是引擎 node 其实
   * 没装时关于页照样显示「v24.18.0 · 应用托管」，与健康大盘的「未检出」自相矛盾
   * ——用户看到的版本号从来没被安装过。现在"装没装"与"打算装什么"分开。
   */
  version: string | null
  /** engine = 壳引擎资产（已装）；managed = 应用托管（未装，走引导补齐） */
  origin: "engine" | "system" | "managed"
  /** 未安装时**计划**安装的版本——只用于提示，不得渲染成已装。 */
  plannedVersion: string | null
}

/// boot:update 载荷 / get_update_status 返回值。
export interface UpdateStatus {
  dsh: ComponentUpdate
  client: ComponentUpdate
  node: NodeRuntimeInfo | null
}

/// 客户端自更新状态机快照（app:update 载荷 / get_client_update 返回值）。
export type ClientUpdate =
  | { phase: "idle" }
  | { phase: "checking" }
  | { phase: "available"; latest?: string | null; notes?: string | null }
  | { phase: "upToDate"; latest?: string | null }
  | { phase: "downloading"; current?: number | null; total?: number | null }
  | { phase: "installing" }
  | { phase: "relaunching" }
  | { phase: "done"; version: string }
  | { phase: "failed"; message: string }

/// 错误卡动作 id（terminal_action 的合法入参子集）。
export type TerminalAction =
  | "retry"
  | "upgrade"
  | "upgrade_only"
  // 安全模式（ADR-0026）：在 profile 配置里把三方插件写成 disabled / 配置已写坏时备份并放空。
  // （`safe_mode_exit` 已随"一键恢复"移除——恢复会把坏配置搬回来，启动照样失败。）
  | "safe_mode"
  | "safe_mode_reset"

// ---------- Profile 管理器（4.3；形状锚定 src-tauri/src/profiles.rs 的 serde 序列化） ----------

/// 列表条目：已物化 profile 或未物化的内置模板名（两态合并，ADR-0009 方案 E）。
export interface ProfileSummary {
  name: string
  /** true = 已物化（目录存在）；false = 内置模板名可首启 */
  materialized: boolean
  /** dsh.profile.bundles；未物化模板名 = dsh 内置模板 bundle 列表 */
  bundles: string[]
  /** package.json dependencies 的包名（字典序） */
  dependencies: string[]
  /** 是否 webUi 工作台（bundles 含 dsh-web-app）：启动/切换入口的可见性依据 */
  web_ui: boolean
}

/// 单个 profile 详情（package.json 关键字段 + cordis.patch.yml 原文）。
export interface ProfileDetail {
  /** package.json 的 name 字段（dsh 约定 dsh-profile-<目录名>）；缺失为 null */
  package_name: string | null
  bundles: string[]
  /** dependencies 的 name → specifier */
  dependencies: Record<string, string>
  /** patch 原文（后端不解析 YAML）；文件不存在为 null */
  patch_yaml: string | null
}

/// 创建结果：「基础 + Web 工作台已就绪」是成功态（声明零下载，创建即 webUi
/// 候选可设为默认启动）；「已创建未装插件」是合法中间态而非失败
/// （ADR-0009 方案 A 执行细则两次修订）。
export interface CreateProfileOutcome {
  profile: string
  materialized: boolean
  installed: boolean
  /** 人读状态 + 可行动建议（附 dsh 输出尾部） */
  detail: string
}

/// 插件清单条目（4.4①，形状锚定 src-tauri/src/plugins.rs）。
export interface PluginEntry {
  name: string
  /** bundle = dsh 内置（随 dsh 安装目录）；dependency = 第三方外挂 */
  kind: "bundle" | "dependency"
  /** 已安装版本（node_modules 实读）；null = 未安装 / 内置随 dsh */
  installed_version: string | null
  description: string | null
}

/// 运行态快照条目（复现点 11：pluginInventory/list；一次性，不订阅）。
export interface RuntimeEntry {
  entry_id: string
  module_name: string
  enabled: boolean
  /** null = 已停用（disposed） */
  fiber_phase: "active" | "loading" | "pending" | "failed" | "unloading" | null
}

export interface PluginRuntimeSnapshot {
  /** 快照归属 profile（活跃会话的）；null = 无活跃会话，前端不合并 */
  profile: string | null
  entries: RuntimeEntry[]
}

/// 插件安装/卸载/更新结果（4.4②）：ok = dsh 退出 0 且未超时；
/// detail 为人读文案（失败附 dsh 输出尾部，成功含「重启后生效」提示）。
/// 2026-09-09（ADR-0013）：构建脚本改默认批准，审批门载荷（ignored_builds）
/// 随审批链一并退役。
/** 插件操作失败分类：**唯一实现在后端**（`plugin_registry::classify_failure`），
 *  前端只消费它决定显示哪句话——两份正则分类会在下次改动里漂移。 */
export type FailureKind = "network" | "not_found" | "build_approval" | "other"

export interface PluginOpOutcome {
  ok: boolean
  detail: string
  /** 失败分类；成功为 `null`。 */
  failureKind?: FailureKind | null
}

/// 插件行表条目（4.4③，复现点 7/ADR 第四次修订）：行 id 不可从包名推导，
/// 来自 dump-config 行表；shell_disabled = 壳 patch toggle 的禁用意图。
/// 2026-09-08 补丁包开关（ADR 第七次修订）：无自身行的补丁包合成条目
/// contributed_ids 非空 = 该包贡献的行 id 列表，开关目标 = 全部贡献行。
export interface PluginRowState {
  id: string
  pkg_name: string
  shell_disabled: boolean
  /** 该 profile 自身 cordis.patch.yml 中此 id 的条目数（连配置勾选的置灰预检，
      4.4④ 收口 / ADR-0009 第五次修订） */
  patch_entries: number
  /** 补丁包（无自身行的 bundle）贡献的行 id 列表；普通插件为空。
      id 字段在补丁包上取第一贡献行（兼容单目标引用）。 */
  contributed_ids: string[]
}

/// 更新检查报告（4.4④，registry dist-tags.latest 口径）：failed 不计入 checked。
export interface PluginUpdateReport {
  updates: { name: string; current: string; latest: string }[]
  checked: number
  failed: number
}

/// 复制/重命名结果（warnings = 需人工关注项，如 patch 相对路径引用）。
export interface LifecycleOutcome {
  profile: string
  warnings: string[]
}

/// 插件总览聚合条目（4.4④ 收口，ADR-0009 第五次修订）：第三方插件在各
/// profile 的安装分布；纯文件扫描，只读。
export interface AggregateSource {
  profile: string
  /** 已装版本（node_modules 实读）；null = 声明未安装 */
  version: string | null
  /** package.json 依赖声明值原样（git/tarball 来源 = 安装 spec，市场对齐键） */
  spec?: string | null
}

export interface AggregatePlugin {
  name: string
  /** 首个非空 description（任一来源 profile 实读）；null = 均无 */
  description: string | null
  sources: AggregateSource[]
}

/// 配置行原样复制结果（patch 写入例外 #4）：copied = 追加条目数；
/// skipped_existing = 目标已有同 id 条目零写入（不覆盖）。
export interface CopyConfigOutcome {
  copied: number
  skipped_existing: boolean
  detail: string
}

/// 删除结果。
export interface DeleteOutcome {
  profile: string
  /**
   * 该 profile 是默认启动 profile，删除时已把引用**置为 None**
   * （`commands/profile.rs`：`settings.default_profile = None`）。
   * 之后按 AGENTS §6 口径：**None 由消费方兜底 `web`**；
   * **失效值不消费**（走常规流程＝出选择器）。
   * 2026-09-11（task-27）措辞校正：原文「读取侧兜底 web」易被读成
   * 「保留失效值、读时兜底」，与实现（删除即置 None）不符。
   */
  default_cleared: boolean
}

/// 会话简要信息
export interface SessionItem {
  id: string
  /** 会话标题（取自 dsh `session/title` 事件；无标题时为空串） */
  title: string
  projectName: string
  projectDirRaw: string
  decodedProjectPath: string
  filePath: string
  updatedAt: number
  sizeBytes: number
  isCompressed: boolean
  hasBackup: boolean
  status: "healthy" | "needs_repair" | "unknown"
  /** 健康检查附加信息（异常原因/未修复原因），无异常时不返回 */
  healthDetail?: string
  /** 可能仍在被 dsh 写入（复合判据：引擎存活且 mtime<5min）——UI 显示「运行中」，不显示修复按钮 */
  active?: boolean
  /** 已归档（dsh workspace.json archivedSessionIds）——默认隐藏，仅「已归档」筛选档可见 */
  archived?: boolean
  /** 会话创建时间（header.createdAt，毫秒 epoch；未知为 0） */
  createdAt?: number
  /** 展开后的事件总数（信息性统计；不可解析为 0） */
  eventCount?: number
  /** 结束状态：stop=正常结束 / interrupted=回合中断 / open=未收尾；不可解析为 null */
  endState?: string | null
  /** 子代理会话（header.origin=subagent） */
  subagent?: boolean
  /** 会话代理预设（如 standard） */
  agentPreset?: string | null
  /** 健康判定所用校验器（dsh-session@版本 / fallback） */
  validator?: string | null
}

/// 会话修复结果
export interface RepairOutcome {
  sessionId: string
  success: boolean
  message: string
}

/// 凭据脱敏摘要项
export interface CredentialSummaryItem {
  provider: string
  label: string
  configured: boolean
  maskedKey: string
}

/// MCP 条目所在的 patch 层 —— 决定它的**生效范围**（2026-09-18）。
///
/// dsh 的 patch 应用顺序：各 bundle 层 → `profile` → `global` → `--patch` overlays。
/// 两层**同名不是覆盖**：两条都会插入，上游 `serverName` 是加载期预留，
/// 先加载的占住名字、后加载的那条实例化失败。
export type McpScope = "profile" | "global"

/** 传输方式：上游 `@deepseek-ai/dsh-mcp-client` 只有这两种（无 `sse` 字面量）。 */
export type McpTransport = "stdio" | "streamable-http"

/// MCP 服务器配置项
export interface McpServerConfig {
  name: string
  command: string
  args: string[]
  env: Record<string, string>
  disabled: boolean
  /** 传输方式（2026-09-15，ADR-0022 前置）：上游 `@deepseek-ai/dsh-mcp-client`
   *  仅支持 `stdio` 与 `streamable-http`（无 `sse` 字面量）；后端缺省发 `stdio`。
   *  可选是为兼容既有构造点（表单只填 stdio 字段时无需补全）。
   *  **编辑时必须带回原值**：保存会按传输方式清掉另一分支的键（切传输不留残键），
   *  表单漏带 = 把一条 http 条目改写成 stdio 并丢掉 url/headers。 */
  transport?: McpTransport
  /** `streamable-http` 的 MCP 端点 URL（stdio 恒空）。 */
  url?: string
  /** `streamable-http` 的附加请求头（stdio 恒空）。 */
  headers?: Record<string, string>
  /** stdio 子进程工作目录（2026-09-18）。上游缺省是空串；「绝对路径 node +
   *  绝对路径入口脚本」这类不依赖 PATH 的写法必须靠它。不填则不下发该键。 */
  cwd?: string
  /** 条目所在层（生效范围）。读取时由后端按所在层填充；保存时据此选层。 */
  scope?: McpScope
  /** 条目在 patch 里的行 id（insert 项 `id`；旧字典形态为所在行 id）。
   *  2026-09-18：装配状态徽标按它精确匹配运行态 `entryId`，同层重复行也靠它区分。 */
  rowId?: string
  /** 该行原文含 `!!js` 之类的标签值（上游用它传 secret 引用）。
   *  表单里看到的是**展平后的字符串**；这类行只有「启用/停用」可以直接保存，
   *  改配置会被后端拒绝（结构化写入会把表达式静默展平成字面量）。 */
  expr?: boolean
}

/// 一条具名 MCP 能力（tool / resource / template）——2026-09-15，ADR-0022。
export interface McpNamed {
  name: string
  /** tool 取 description；resource 取 uri；template 取 uriTemplate。 */
  detail: string
}

/** 一次 MCP 能力探测的结果（stdio 分支）。 */
export interface McpProbe {
  /** **服务端协商后**的协议版本（非我方发送值）。 */
  protocolVersion: string
  /** 服务端自报名称（缺省回退配置里的 serverName）。 */
  serverName: string
  tools: McpNamed[]
  resources: McpNamed[]
  templates: McpNamed[]
  /** 降级说明（如"该服务器不支持 resources"）；空 = 全部枚举成功。 */
  notes: string[]
}

// ---------- 系统设置与诊断（4.11 / 4.12 / 4.13） ----------

export interface ShellSettings {
  defaultMode?: "local" | "wsl" | null
  defaultProfile?: string | null
  locale?: string | null
  autoRestart?: boolean | null
  showFloatingSwitcher?: boolean | null
  switcherShortcut?: string | null
  /** 升级提示条已忽略版本键（"dsh@x.y.z" / "client@x.y.z"；同键不再弹） */
  dismissedUpdate?: string | null
  /** 插件安装源偏好（ADR-0006 §6）：`auto` 先官方、失败换源一次；`official` /
   *  `configured` 只用一个源且不自动换。`null`/未知值 = auto。 */
  pluginRegistry?: "auto" | "official" | "configured" | null
  /** 上次**成功**用过的源（`auto` 时用来排序首试）；只记成功，抖动不带偏。 */
  pluginRegistryLastGood?: "official" | "configured" | null
}

export interface NodeDiagnosticInfo {
  path: string
  version: string
  source: string
  isReady: boolean
}

export interface PnpmDiagnosticInfo {
  path: string
  version: string | null
  isReady: boolean
}

export interface DshDiagnosticInfo {
  path: string
  version: string | null
  source: string
  isReady: boolean
}

export interface StorageDiagnosticInfo {
  dshHome: string
  totalBytes: number
  profilesBytes: number
  sessionsBytes: number
  profilesCount: number
  sessionsCount: number
}

export interface PlatformDiagnosticInfo {
  os: string
  arch: string
}

export interface SystemDiagnosticsReport {
  node: NodeDiagnosticInfo
  pnpm: PnpmDiagnosticInfo
  dsh: DshDiagnosticInfo
  storage: StorageDiagnosticInfo
  platform: PlatformDiagnosticInfo
}

export interface LogQueryResult {
  source: string
  path: string
  lines: string[]
  totalLines: number
  truncated: boolean
}

/// 启动失败分类（ADR-0012）：tagged enum，`kind` 为判别式。
///
/// **唯一事实源**（2026-09-16 收口）：`BOOT_FAILURE_KINDS` 是运行期白名单，
/// `BootFailureKind` 由它派生；`lib/eventPayloads.ts::normalizeFailure` 只认这张表。
/// 改前两处**各自手抄一份**，于是：
///   ① `symlink_privilege_required` 只加在类型上、白名单没跟上 ⇒ 该 kind 被静默丢弃，
///      UI 回退到后端中文文案（en 用户看到中文）；
///   ② 同一天 `normalizeError` 又漏掉新字段 `advancedActions` ⇒ 「其它出路」整块永不渲染
///      （两条次级出路在 UI 上完全不可达）——2026-09-16 独立复核抓到。
/// 结论：**能派生就派生**，不能靠两张手抄表互相追。
/// 与 `boot_failure.rs` 的 serde 序列化值逐字对应（只加不重排）。
export const BOOT_FAILURE_KINDS = [
  "credentials_mismatch",
  "incompatible_options",
  "network_unavailable",
  "symlink_privilege_required",
  "plugin_row_failed",
  "unknown",
] as const

export type BootFailureKind = (typeof BOOT_FAILURE_KINDS)[number]

export interface BootFailure {
  kind: BootFailureKind
  /// 仅 `unknown` 变体携带（无法类型化的外部原文；展示用，不参与分类）。
  detail?: string
}

/// `boot:error` 事件载荷（`BootErrorPayload`，camelCase；形状由 ipc-shapes.json 闸住）。
/// `title`/`suggestion` 是后端文案（兼容分支）；`failure.kind` 是结构化分类，
/// 前端按 kind 取本地化文案，取不到再回退后端文案。
export interface BootErrorPayload {
  failure: BootFailure
  title: string
  detail: string
  suggestion: string
  actions: string[]
  /// 次级出路（2026-09-16 维护者反馈后的分层）：**首屏只渲染 `actions`**（插件行失败时
  /// 恰好一项 = 「停用全部插件并启动」），`advancedActions` 收进"展开详情 → 其它出路"。
  /// 空表 = 无次级出路（非插件行失败恒空）。
  advancedActions: string[]
  log: string
  /// 可一键隔离的挂载行（2026-09-16）：`Some` 时"其它出路"里渲染
  /// 「只移除出错的那一行并重启」（**首屏不展示**）。**只读诊断**：行不归壳
  /// 所有、或拿不到会话目标 profile 时为 `null`（`null` 时前端不渲染该按钮）。
  quarantine: QuarantineRow | null
}

/// 可一键隔离的挂载行（`profile` + 行 id）：`boot_failure.rs::QuarantineRow`。
export interface QuarantineRow {
  profile: string
  rowId: string
}

/// 交接意图（ADR-0014）：一次「停旧 → 起新 → 进工作台」的贯穿状态。
/// 形状锚定 `boot.rs::Handoff` 的 serde 序列化（camelCase 字段 + snake_case 枚举），
/// 由 `ipc-shapes.json` 的 `Handoff` 条目跨语言闸住。
export type HandoffKind = "start" | "restart" | "switch"

export type HandoffPhase =
  | "stopping"
  | "booting"
  | "waiting"
  | "entering"
  | "ready"
  | "failed"

export interface HandoffIntent {
  target: string
  kind: HandoffKind
  phase: HandoffPhase
  /// 发起时刻（Unix ms）：两窗计时器同源，跨文档替换**不归零**
  startedAt: number
  /// 启动代际令牌（同批次启动线程持有；前端只用于去重展示）
  generation: number
}

/// `get_boot_status.intent` 的实际载荷：意图 + Rust 裁决的 `active`
/// （仍在途且未超 TTL）——TTL 只由 Rust 持有，前端与注入脚本都不再各抄一份。
export interface HandoffSnapshot extends HandoffIntent {
  active: boolean
}

// ---------- 实验能力开关（ADR-0020，2026-09-15 立 / 2026-09-16 第二次修订）----------
//
// 呈现单位是**能力**（capability），不是包：同一能力的互斥后端降级为**变体**
// （variant）。v1 把策展集摊平成 8 条并列条目，于是"三选一"要靠用户自己读卡片，
// 而"关掉"这件事在目录里根本不存在（只有安装）。

/// 激活方式：由目标包**是否声明 `dsh.bundle`** 决定，是激活契约的唯一依据。
/// - `auto_bundle`：声明了 → `dsh plugin add` 自行追加进 `dsh.profile.bundles`，
///   壳**不得**再写 `insert`（会重复挂载）；
/// - `insert_row`：未声明 → CLI 只装依赖并告警，壳**必须**写挂载行。
export type CapabilityActivation = "auto_bundle" | "insert_row"

/// 变体状态：由「包 × 挂载行 × disabled」三者共同决定（后端唯一判定，前端不猜）。
/// - `off`：一个包都没装；`on`：包齐 + 行齐 + 无停用；
/// - `disabled`：包齐 + 行齐，但行被停用（已就位、当前关着，秒级可开）；
/// - `partial`：装了一半或包在而行缺（需要修复）。
export type VariantState = "off" | "on" | "disabled" | "partial"

/// 能力状态 = 其变体状态的聚合；`conflict` = 同能力多个变体同时生效（上游会激活失败）。
export type CapabilityState = "off" | "on" | "disabled" | "partial" | "conflict"

/// 能力下的**一步**（一个包的装/挂/开关事实）。
export interface CapabilityStep {
  /// 第几步（1 起）：有序组合靠它表达"先宿主层再 Web 层"，也是失败续装的锚点。
  ordinal: number
  package: string
  /// 钉版本后的 spec（形如 `@scope/pkg@0.1.6-alpha.1`）；运行时版本未检出时**退回裸包名**
  /// 并在 `versionNotice` 里明示未钉版本。
  spec: string
  activation: CapabilityActivation
  /// patch 行 id（`insert_row` 时才真正落盘；缺 id 的行永不可再 patch）。
  rowId: string
  /// 该包是否已在 profile 依赖里。
  installed: boolean
  /// 该包的挂载行是否已**在组合树中**。
  rowPresent: boolean
  disabled: boolean
  /// 停用/启用该步要写的行 id（bundle 贡献多行时全部一起切）；空 = 无行可切。
  toggleTargets: string[]
  /// 版本错配提示（如 registry `latest` 落后于运行时）；`null` = 无需打扰用户。
  versionNotice: string | null
  /// **该包自己的** `description`（2026-09-17 裁定「描述以官方为主」）：装好后从
  /// `<profile>/node_modules/<包>/package.json` 读本地文件，**不转述不翻译**；
  /// 未装时 `null`（界面如实说"装好后显示它自带的官方简介"）。
  description: string | null
}

/// 能力下的一个可选后端。同能力的变体**互斥**（同一时刻只应有一个生效）。
/// 展示字段（`label/note/prerequisites`）**按请求语言出品**（2026-09-18 边界A：
/// IPC 传 locale，后端只下发该语言的一份文案），故字段名不带语言后缀。
export interface CapabilityVariant {
  id: string
  label: string
  /// 一句话：这个后端适合谁 / 代价是什么。
  note: string
  /// 需要用户自备或额外配置的东西（空 = 无）。
  prerequisites: string[]
  steps: CapabilityStep[]
  state: VariantState
  /// 本变体的包是**另一个已就位变体**的真子集 → 本档已被那一档包含（Agent Teams 的
  /// 自建档 ⊂ Web 档）。此时该档不单独开关：关掉它会把超集档的基础层一起拆掉。
  /// `null` = 独立档。
  subsumedBy: string | null
  /// **纯行级停用是否等价于"关掉"**（`false` → 关闭必须走移除）。
  ///
  /// `false` 的成因是变体含 profile 层（`dsh.bundle.patch`）：层 patch 带副作用
  /// （实测 Agent Teams 的层还停用了 4 条旧 subagent 行），行级 disabled 关不干净。
  /// **仅在 `state !== "off"` 时有意义**：未安装时无从判定包的形态，后端一律给 `false`。
  toggleOffSupported: boolean
  /// 同能力其它变体已装、而本变体不含的后端包；非空 = 启用本变体需先替换掉它们。
  displaced: string[]
  /// 本变体要求的宿主可执行文件**缺失**（`null` = 前置齐备）。
  ///
  /// 非 `null` 时**必须禁用开关**并原样展示这句话：缺 `cua-driver` 之类的前置时装上
  /// 该 provider，dsh 会在插件树加载阶段直接失败、工作台起不来（2026-09-16 真机事故）。
  /// 文案由后端下发——只有它知道缺的是哪个命令（禁前端自造文案，避免两处漂移）。
  prerequisiteMissing: string | null
}

/// 一项可开关的实验能力。
export interface Capability {
  /// 稳定 id（`agent-team` 等）——**不得用展示名当身份**（改文案即丢状态）。
  id: string
  label: string
  /// 一句话价值。
  summary: string
  /// 启用后**用户能观察到什么**（含"什么会消失"，如旧 subagent 控件被取代）。
  unlocks: string
  variants: CapabilityVariant[]
  state: CapabilityState
  /// 当前生效（或已就位但停用）的变体 id；`null` = 未启用 / 冲突。
  activeVariant: string | null
  /// **dsh 安装包自带这项能力**（2026-09-17 起：官方实验层作为 optional bundle 随产品下发）。
  /// 为真时界面**不渲染开关/安装/移除**——dsh 自己管，且视其为"永不卸载"。
  shippedByDsh: boolean
  /// 该能力的包**仍被本 Profile 自己持有**（dsh-dock 早期按 profile 装过的一份）。
  /// 与 `shippedByDsh` 同时为真 = 历史遗留副本（会遮蔽 dsh 自带的那一份）：只给"清理"。
  legacyCopy: boolean
}

/// 安全模式状态（ADR-0026，`get_safe_mode_state`）。安全模式改的是** profile 配置本身**，
/// 故配置层即真相源；这里只报壳的**记账**（是否在安全模式、停了哪些行、能否一键恢复），
/// 不报运行态——运行态由回环快照给，两源禁混。
export interface SafeModeState {
  /// 本轮是否以安全模式启动（= 壳的记账文件在，配置里已写入停用桩）。
  active: boolean
  /// **此刻**仍处于停用态、且是安全模式写入的那些行 id（用户逐个打开后会变少；
  /// 全打开 → `active=false`，横幅自动消失）。
  disabledRows: string[]
  /// 本轮安全模式的横幅是否已被用户关掉（"不再提示"）——前端据此不渲染横幅。
  /// **每次以安全模式进入都会重置为 false**（新事件值得再说一次）。
  noticeDismissed: boolean
}

/// `apply_official_patch_row` 的结果：**写行前当场重判**该包是否声明 `dsh.bundle`。
/// `autoActivated` = 该包是 profile 层，已由 CLI 激活，壳**未写行**（也不应写）。
export interface RowWriteOutcome {
  /// 是否真的改动了 patch 文件（幂等重写 = false）。
  changed: boolean
  autoActivated: boolean
}

