// sessionStatus.ts —— 会话状态 → 视觉映射（纯函数，2026-09-08 UI/UX 评审 §18）。
//
// 从 `SessionManager.tsx` 下沉：`unknown` 这一类既可能是「脚本已给出确切原因」
// （JSON 不可解析 / 空文件 / 版本高于本构建），也可能是「扫描失败，无原因」
// （引擎未就绪）。两者的描述必须不同——旧实现只有一句「无法判定健康状态
//（可能为活跃会话或引擎未就绪）」，对明确损坏的文件是误导。
import type { AppCopy } from "@/content/zh-CN"
import type { SessionItem } from "@/types/ipc"

/** 状态视觉映射：色点 + 徽标 + 描述（描述进徽标 title，读屏/悬停可读）。 */
export function statusMeta(
  status: SessionItem["status"],
  t: AppCopy,
  /** 是否携带脚本给出的具体原因（`healthDetail` 非空） */
  hasReason: boolean,
): { dot: string; badge: string; desc: string } {
  switch (status) {
    case "healthy":
      return {
        dot: "bg-ok",
        badge: t.sessions.statusHealthy,
        desc: t.sessions.statusHealthyDesc,
      }
    case "needs_repair":
      return {
        dot: "bg-warn animate-pulse",
        badge: t.sessions.statusNeedsRepair,
        desc: t.sessions.statusNeedsRepairDesc,
      }
    default:
      return {
        dot: "bg-faint/50",
        badge: t.sessions.statusUnknown,
        desc: hasReason ? t.sessions.statusUnknownDescWithReason : t.sessions.statusUnknownDesc,
      }
  }
}
