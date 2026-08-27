// 平台/宿主能力 hook（frontend-migration §10 步骤 26）。
// __DSH_PLATFORM__ 由 Rust 在 document-start 注入且单窗口生命周期内不变，
// 这里 memo 一次即可；组件层不散写 os === 'windows' 判断，一律经 can.* 消费。
import { useMemo } from "react"
import { getCapabilities, getPlatform } from "@/lib/host"
import type { HostCapabilities, PlatformInfo } from "@/lib/host"

export function usePlatform(): { platform: PlatformInfo; can: HostCapabilities } {
  return useMemo(() => ({ platform: getPlatform(), can: getCapabilities() }), [])
}
