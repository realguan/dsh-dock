// 升级提示条纯逻辑（ADR-0010 升级呈现，2026-09-04）：从更新快照与已忽略
// 版本键推导该展示哪些非阻断提示条。组件只消费本文件导出的纯函数——
// 拒绝后不再弹窗（同键）、无硬惩罚的判定都在这里，Vitest 纯逻辑可测。
import type { UpdateStatus } from "@/types/ipc"

export interface BannerSpec {
  kind: "dsh" | "client"
  /** settings.dismissedUpdate 的记忆键（如 "dsh@1.6.0"） */
  key: string
  latest: string
  current: string | null
}

/** 版本键（settings.rs ShellSettings.dismissed_update 契约同形） */
export function bannerKey(kind: "dsh" | "client", latest: string): string {
  return `${kind}@${latest}`
}

/// 当前该展示的提示条（顺序 = dsh 优先）。不出示的条件：无快照 / 该维度
/// 无 latest（检测失败或未配置）/ latest 不比当前新 / 用户已忽略同键。
export function updateBanners(
  status: UpdateStatus | null,
  dismissed: string | null | undefined,
): BannerSpec[] {
  if (!status) return []
  const out: BannerSpec[] = []
  for (const [kind, dim] of [
    ["dsh", status.dsh],
    ["client", status.client],
  ] as const) {
    if (!dim.newer || !dim.latest) continue
    const key = bannerKey(kind, dim.latest)
    if (dismissed === key) continue
    out.push({ kind, key, latest: dim.latest, current: dim.current })
  }
  return out
}
