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
