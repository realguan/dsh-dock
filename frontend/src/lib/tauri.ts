// Tauri IPC 唯一入口：20 个命令全类型化（frontend-migration §3.3）。
// 组件中不直接 invoke()，必须走本文件 api 对象；所有调用统一 .catch()。
// open_about 已于 v0.4.7 删除（8075eea）——常驻入口在菜单/托盘。
// 参数拼写已对照现网 ui/*.html 逐一核实；choose_mode 的 { mode, setDefault }
// 由 Tauri 自动映射 snake_case Rust 参数 set_default。
import { invoke } from "@tauri-apps/api/core"
import type {
  ClientUpdate,
  CreateProfileOutcome,
  DeleteOutcome,
  LifecycleOutcome,
  ProfileDetail,
  ProfileSummary,
  TerminalAction,
  UpdateStatus,
} from "@/types/ipc"

export const api = {
  // 启动流程
  chooseProfile: (profile: string) => invoke<void>("choose_profile", { profile }),
  chooseMode: (mode: string, setDefault: boolean) =>
    invoke<void>("choose_mode", { mode, setDefault }),
  bootInWsl: () => invoke<void>("boot_in_wsl"),

  // 版本状态
  getUpdateStatus: () => invoke<UpdateStatus>("get_update_status"),
  checkUpdates: () => invoke<void>("check_updates"),

  // 客户端自更新
  getClientUpdate: () => invoke<ClientUpdate>("get_client_update"),
  clientUpdateCheck: () => invoke<void>("client_update_check"),
  clientUpdateApply: () => invoke<void>("client_update_apply"),

  // 错误卡动作
  terminalAction: (action: TerminalAction) =>
    invoke<void>("terminal_action", { action }),

  // 窗口/导航
  openExternal: (url: string) => invoke<void>("open_external", { url }),
  openWorkbenchInBrowser: () => invoke<void>("open_workbench_in_browser"),
  getWorkbenchUrl: () => invoke<string | null>("get_workbench_url"),

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
  // 切换 = 停当前会话以目标 profile 重启（ADR-0009 §4 三次修订；确认在前端）
  switchProfile: (profile: string) => invoke<void>("switch_profile", { profile }),
  getActiveProfile: () => invoke<string | null>("get_active_profile"),
}
