// Tauri IPC 唯一入口：全部自定义命令全类型化。命令清单的**唯一事实源**是
// `src-tauri/src/ipc.rs::COMMANDS`（现 55 条），本文件与它由 cargo test 的
// `ipc::gate_tests::tauri_ts_matches_ipc_commands` 双向比对——此处不写死数量，
// 免得像 2026-09-08 之前那样写「20 个命令」而实际 55 个（frontend-migration §3.3）。
// 组件中不直接 invoke()，必须走本文件 api 对象；所有调用统一 .catch()。
// open_about 已于 v0.4.7 删除（8075eea）——常驻入口在菜单/托盘。
// 参数拼写已对照现网 ui/*.html 逐一核实；choose_mode 的 { mode, setDefault }
// 由 Tauri 自动映射 snake_case Rust 参数 set_default。
import { invoke } from "@tauri-apps/api/core"
import type {
  AggregatePlugin,
  Capability,
  ClientUpdate,
  CopyConfigOutcome,
  CreateProfileOutcome,
  CredentialSummaryItem,
  DeleteOutcome,
  DshVersionsResult,
  LifecycleOutcome,
  LogQueryResult,
  McpProbe,
  McpServerConfig,
  RowWriteOutcome,
  SafeModeState,
  PluginEntry,
  PluginOpOutcome,
  PluginRuntimeSnapshot,
  PluginRowState,
  PluginUpdateReport,
  ProfileDetail,
  ProfileSummary,
  HandoffSnapshot,
  RepairOutcome,
  SessionItem,
  ShellSettings,
  SystemDiagnosticsReport,
  TerminalAction,
  UpdateStatus,
} from "@/types/ipc"

export interface BootStatusResult {
  steps: unknown[]
  error: unknown | null
  /** 交接意图（ADR-0014）：主窗口整文档重载后靠它续上控制中心那条导轨与计时 */
  intent?: HandoffSnapshot | null
  needs_mode_selection?: boolean
  needsModeSelection?: boolean
}

export const api = {
  // 启动流程
  chooseProfile: (profile: string) => invoke<void>("choose_profile", { profile }),
  chooseMode: (mode: string, setDefault: boolean) =>
    invoke<void>("choose_mode", { mode, setDefault }),
  bootInWsl: () => invoke<void>("boot_in_wsl"),
  getBootStatus: () => invoke<BootStatusResult>("get_boot_status"),

  // 版本状态
  getUpdateStatus: () => invoke<UpdateStatus>("get_update_status"),
  checkUpdates: () => invoke<void>("check_updates"),
  // DSH 版本列表（版本选择器数据源：全版本 + 通道 + 发布时间 + 相对关系）
  listDshVersions: () => invoke<DshVersionsResult>("list_dsh_versions"),

  // 客户端自更新
  getClientUpdate: () => invoke<ClientUpdate>("get_client_update"),
  clientUpdateCheck: () => invoke<void>("client_update_check"),
  clientUpdateApply: () => invoke<void>("client_update_apply"),

  // 错误卡动作（version = upgrade/upgrade_only 的显式目标版本；None = 最新可接受版）
  terminalAction: (action: TerminalAction, version?: string | null) =>
    invoke<void>("terminal_action", { action, version: version ?? null }),

  // 窗口/导航
  openExternal: (url: string) => invoke<void>("open_external", { url }),
  openWorkbenchInBrowser: () => invoke<void>("open_workbench_in_browser"),
  getWorkbenchUrl: () => invoke<string | null>("get_workbench_url"),
  openProfilesWindow: () => invoke<void>("open_profiles_window"),
  focusMainWindow: () => invoke<void>("focus_main_window"),

  // Profile 管理器（4.3；Rust 侧 profiles.rs；「已创建未装插件」为合法中间态）
  listProfiles: () => invoke<ProfileSummary[]>("list_profiles"),
  getProfileDetail: (profile: string) =>
    invoke<ProfileDetail>("get_profile_detail", { profile }),
  createProfile: (profile: string) =>
    invoke<CreateProfileOutcome>("create_profile", { profile }),
  copyProfile: (source: string, newName: string) =>
    invoke<LifecycleOutcome>("copy_profile", { source, newName }),
  renameProfile: (oldName: string, newName: string) =>
    invoke<LifecycleOutcome>("rename_profile", { oldName, newName }),
  deleteProfile: (profile: string) => invoke<DeleteOutcome>("delete_profile", { profile }),
  setDefaultProfile: (profile: string) => invoke<void>("set_default_profile", { profile }),
  getDefaultProfile: () => invoke<string | null>("get_default_profile"),
  // 切换 = 停当前会话以目标 profile 重启（ADR-0009 §4 三次修订；确认在前端）。
  // 返回交接意图（ADR-0014）：控制中心据此立刻起贯穿导轨，与主窗口共享同一
  // startedAt/generation（两窗计时器同源，跨文档不归零）。
  switchProfile: (profile: string) => invoke<HandoffSnapshot>("switch_profile", { profile }),
  getActiveProfile: () => invoke<string | null>("get_active_profile"),
  // 4.4① 插件清单：静态读文件层；运行态 = 回环只读快照（仅活跃会话有数据）
  listProfilePlugins: (profile: string) =>
    invoke<PluginEntry[]>("list_profile_plugins", { profile }),
  getPluginRuntime: () => invoke<PluginRuntimeSnapshot>("get_plugin_runtime"),
  // 4.4② 插件安装/卸载/更新：dsh plugin 转发链（阻塞可达分钟级，前端按 busy 态处理）
  installPlugin: (profile: string, pkg: string) =>
    invoke<PluginOpOutcome>("install_plugin", { profile, package: pkg }),
  removePlugin: (profile: string, pkg: string) =>
    invoke<PluginOpOutcome>("remove_plugin", { profile, package: pkg }),
  updatePlugin: (profile: string, pkg: string) =>
    invoke<PluginOpOutcome>("update_plugin", { profile, package: pkg }),
  // 4.4③ 禁用/启用：行表（行 id 权威来源）+ patch 单键切换（重启后生效）
  getPluginRows: (profile: string) =>
    invoke<PluginRowState[]>("get_plugin_rows", { profile }),
  setPluginDisabled: (profile: string, rowId: string, disabled: boolean) =>
    invoke<void>("set_plugin_disabled", { profile, rowId, disabled }),
  // 4.4④ 更新检查：外网 registry 镜像链（updates.rs），按钮触发不自动跑
  checkPluginUpdates: (profile: string) =>
    invoke<PluginUpdateReport>("check_plugin_updates", { profile }),
  listPluginVersions: (pkg: string) =>
    invoke<string[]>("list_plugin_versions", { package: pkg }),
  // 4.4④ 收口：插件总览聚合（只读文件扫描）+ 配置行原样复制（写入例外 #4）
  listAllPlugins: () => invoke<AggregatePlugin[]>("list_all_plugins"),
  /** 实验能力目录（ADR-0020 §7）：能力 → 变体 → 步骤三级事实视图，能力状态由后端
   *  一次算全（「包 × 行 × disabled」的函数）。运行时版本由**后端本地检出**，前端不传。 */
  listExperimentalCapabilities: (profile: string) =>
    invoke<Capability[]>("list_experimental_capabilities", { profile }),
  /** 确保一条策展挂载行就位（ADR-0020 §7.4）：**写行前当场重判**该包是否声明
   *  `dsh.bundle`——声明了则由 CLI 激活，壳**不写行**（返回 `autoActivated: true`），
   *  因为"安装前"的行形态判定是过期的（v1 因此会给 profile 层多写一条行 = 重复挂载）。
   *  幂等：同 rowId 已存在则 `changed: false` 且零写入。
   *  **写后自证**：命令内部回读 `--dump-config` 组合树，返回成功即"行已在树中"；
   *  若该行未被 DSH 采纳（静默丢弃）或 dump-config 失败，则 reject 并说明"已写入但未生效"。 */
  applyOfficialPatchRow: (profile: string, rowId: string, pkg: string) =>
    invoke<RowWriteOutcome>("apply_official_patch_row", { profile, rowId, package: pkg }),
  /** **反向原语**：删除壳写过的策展挂载行（ADR-0020 §7.2-3）。
   *  只接受 `dsh-dock-` 前缀的行 id（bundle 自带行与用户手写行不得代删）。
   *  **删除后自证**：回读 dump-config 确认该行已不在组合树中；幂等（本就不存在 → false）。 */
  removeOfficialPatchRow: (profile: string, rowId: string) =>
    invoke<boolean>("remove_official_patch_row", { profile, rowId }),
  /** 安全模式状态（ADR-0026）：是否仍处于安全模式 + 此刻仍停用着的行 + 横幅是否已被关闭。
   *  只读；运行态另由回环快照给（两源禁混）。 */
  getSafeModeState: (profile: string) =>
    invoke<SafeModeState>("get_safe_mode_state", { profile }),
  /** 关掉本轮的安全模式横幅（"不再提示"）。只写壳自有记账，**不碰 dsh 配置**。 */
  dismissSafeModeNotice: (profile: string) =>
    invoke<boolean>("dismiss_safe_mode_notice", { profile }),
  copyPluginConfig: (source: string, target: string, pkg: string) =>
    invoke<CopyConfigOutcome>("copy_plugin_config", { source, target, package: pkg }),
  // 会话管理与自愈（4.6）
  listSessions: () => invoke<SessionItem[]>("list_sessions"),
  repairSession: (sessionPath: string) =>
    invoke<RepairOutcome>("repair_session", { sessionPath }),
  repairAllSessions: () => invoke<RepairOutcome>("repair_all_sessions"),
  deleteSession: (sessionPath: string) =>
    invoke<void>("delete_session", { sessionPath }),
  /** 取消归档（ADR-0021 路线 A）：经 Host RPC，返回变更后的完整归档集合。
   *  无活跃 Host 时 reject（需 DSH 在线，壳不直接改磁盘状态）。 */
  unarchiveSession: (sessionId: string) =>
    invoke<string[]>("unarchive_session", { sessionId }),

  // 系统设置与诊断（4.11 / 4.12 / 4.13）
  getShellSettings: () => invoke<ShellSettings>("get_shell_settings"),
  setShellSettings: (settings: ShellSettings) =>
    invoke<void>("set_shell_settings", { settings }),
  getSystemDiagnostics: () =>
    invoke<SystemDiagnosticsReport>("get_system_diagnostics"),
  getAppLogs: (source: string, tailLines?: number) =>
    invoke<LogQueryResult>("get_app_logs", { source, tailLines: tailLines ?? null }),

  // 凭据安全管理与脱敏（4.5）
  getCredentialsRaw: () => invoke<string>("get_credentials_raw"),
  saveCredentialsRaw: (content: string) =>
    invoke<void>("save_credentials_raw", { content }),
  getCredentialsSummary: () =>
    invoke<CredentialSummaryItem[]>("get_credentials_summary"),
  setCredentialKey: (provider: string, key: string) =>
    invoke<void>("set_credential_key", { provider, key }),

  // DSH 全局引擎设置（4.5）
  getDshSettingsRaw: () => invoke<string>("get_dsh_settings_raw"),
  saveDshSettingsRaw: (content: string) =>
    invoke<void>("save_dsh_settings_raw", { content }),

  // MCP 服务器结构化管理（4.7）
  listMcpServers: (profile: string) =>
    invoke<McpServerConfig[]>("list_mcp_servers", { profile }),
  saveMcpServer: (profile: string, server: McpServerConfig) =>
    invoke<void>("save_mcp_server", { profile, server }),
  deleteMcpServer: (profile: string, serverName: string) =>
    invoke<void>("delete_mcp_server", { profile, serverName }),
  /** 探测 MCP 服务器能力（ADR-0022 stdio 分支）：握手后枚举 Tools/Resources/Templates。
   *  `streamable-http` 与 WSL 客体档会 reject（各自说明原因）。 */
  probeMcpServer: (profile: string, serverName: string) =>
    invoke<McpProbe>("probe_mcp_server", { profile, serverName }),

  // 社区插件市场 Registry 拉取
  fetchMarketRegistry: () => invoke<string>("fetch_market_registry"),
}
