// dsh 版本列表的纯交互逻辑（2026-09-09 版本选择器）。组件只消费本文件导出的
// 纯函数——筛选、行动作形态、确认必要性都在这里，Vitest 直测不引 DOM。
// 版本比较不做第二套：`relation` 由后端按 semver 算好，这里只做语义映射。
import type { DshVersionEntry } from "@/types/ipc"

export type VersionFilter = "all" | "upgradable" | "preview"

/** 行尾动作形态：install（高于当前）/ current（已装）/ rollback（低于当前）。 */
export type RowAction = "install" | "current" | "rollback"

export function matchFilter(entry: DshVersionEntry, filter: VersionFilter): boolean {
  switch (filter) {
    case "all":
      return true
    case "upgradable":
      return entry.channel === "stable" || entry.channel === "rc"
    case "preview":
      return entry.channel === "alpha" || entry.channel === "other"
  }
}

export function filterVersions(
  entries: DshVersionEntry[],
  filter: VersionFilter,
): DshVersionEntry[] {
  return entries.filter((e) => matchFilter(e, filter))
}

export function rowAction(entry: DshVersionEntry): RowAction {
  if (entry.relation === "current") return "current"
  if (entry.relation === "older") return "rollback"
  return "install"
}

/**
 * 是否需要二次确认（知情同意）：预览通道（alpha/other，可能不稳定或依赖未
 * 完整发布）一律确认；低于当前版本（回退）一律确认。高于当前的稳定/rc 直接装。
 */
export function needsConfirm(entry: DshVersionEntry): boolean {
  return (
    rowAction(entry) === "rollback" ||
    entry.channel === "alpha" ||
    entry.channel === "other"
  )
}

/** 发布时间展示口径：RFC3339 截取日期段（YYYY-MM-DD）；无时间返回 null。 */
export function formatPublishedAt(rfc3339: string | null): string | null {
  if (!rfc3339) return null
  const date = rfc3339.slice(0, 10)
  return /^\d{4}-\d{2}-\d{2}$/.test(date) ? date : null
}
