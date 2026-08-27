// 平台/宿主能力矩阵（frontend-migration §2 lib/host.ts）。
// 数据源：Rust 在 document-start 注入的 window.__DSH_PLATFORM__ =
// { os, wsl }（lib.rs platform_script，cfg!(windows) 编译期判定）。
// 壳规则：平台语义显式来自 Rust，前端不猜 UA；global 缺失（纯 vite dev
// 浏览器环境）时按「非 Windows、无 WSL」防御性兜底，能力全关。

export type PlatformOs = "macos" | "windows" | "linux" | "unknown"

export interface PlatformInfo {
  os: PlatformOs
  wsl: boolean
}

declare global {
  interface Window {
    __DSH_PLATFORM__?: { os?: string; wsl?: boolean }
  }
}

function readPlatform(): PlatformInfo {
  const injected = typeof window !== "undefined" ? window.__DSH_PLATFORM__ : undefined
  const os = injected?.os
  return {
    os:
      os === "macos" || os === "windows" || os === "linux"
        ? (os as Exclude<PlatformOs, "unknown">)
        : "unknown",
    wsl: injected?.wsl === true,
  }
}

export function getPlatform(): PlatformInfo {
  return readPlatform()
}

/// 能力矩阵：动作可见性过滤。新增能力在此登记，组件经 useHost().can.* 消费，
/// 不在组件里散写 `os === 'windows'` 判断。
export interface HostCapabilities {
  /** 运行环境选择 / 「在 WSL 中打开」仅 Windows 有意义 */
  chooseMode: boolean
  bootWsl: boolean
  /** 壳客户端自更新与宿主无关，恒可用 */
  clientUpdate: boolean
}

function capabilitiesFor(p: PlatformInfo): HostCapabilities {
  const isWindows = p.os === "windows"
  return {
    chooseMode: isWindows,
    bootWsl: isWindows,
    clientUpdate: true,
  }
}

/// 能力查询入口（hooks/usePlatform 于阶段 C 挂到组件层）。
export function getCapabilities(): HostCapabilities {
  return capabilitiesFor(readPlatform())
}
