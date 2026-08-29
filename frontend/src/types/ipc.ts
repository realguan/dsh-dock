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
  origin: "system" | "managed"
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

/// 复制/重命名结果（warnings = 需人工关注项，如 patch 相对路径引用）。
export interface LifecycleOutcome {
  profile: string
  warnings: string[]
}

/// 删除结果。
export interface DeleteOutcome {
  profile: string
  /** 该 profile 是默认启动 profile，引用已清除（读取侧兜底 web） */
  default_cleared: boolean
}
