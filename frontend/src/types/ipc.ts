// IPC 命令的请求/响应类型。形状锚定 Rust 结构体的 serde 序列化：
// - UpdateStatus / ComponentUpdate / NodeRuntimeInfo ← src-tauri/src/updates.rs
// - ClientUpdate ← src-tauri/src/updater.rs（tag="phase"，camelCase）
// Rust 结构体变更时同步本文件（壳前后端同仓同发，无跨仓漂移风险）。

/// 单个可升级组件的版本维度（dsh 本体 / 桌面客户端）。
export interface ComponentUpdate {
  /** 当前版本；检测失败为 null */
  current: string | null
  latest: string | null
  /** latest > current */
  newer: boolean
  error: string | null
}

export interface NodeRuntimeInfo {
  version: string
  /** system = 复用用户已装的 node；managed = 应用私有缓存 */
  origin: "engine" | "system" | "managed"
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
export type TerminalAction = "retry" | "upgrade" | "upgrade_only"

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
/// ignored_builds 非空 = 撞 pnpm 12 构建审批门（ERR_PNPM_IGNORED_BUILDS），
/// 携带被点名包名 → 弹「构建脚本审批」逐包裁决后重试（2026-09-07）。
export interface PluginOpOutcome {
  ok: boolean
  detail: string
  ignored_builds?: string[]
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
  /** 该 profile 是默认启动 profile，引用已清除（读取侧兜底 web） */
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

/// MCP 服务器配置项
export interface McpServerConfig {
  name: string
  command: string
  args: string[]
  env: Record<string, string>
  disabled: boolean
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
export type BootFailureKind =
  | "credentials_mismatch"
  | "incompatible_options"
  | "network_unavailable"
  | "unknown"

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
  log: string
}
