// 维度行布局（About 运行环境区共用）：标签列 + 内容列。
import type { ReactNode } from "react"

export function DimRow({
  label,
  badge,
  children,
}: {
  label: string
  badge?: string
  children: ReactNode
}) {
  return (
    <div className="border-line flex flex-col gap-2 border-b p-4 last:border-b-0 sm:flex-row sm:items-center sm:justify-between">
      <div className="flex items-center gap-2">
        <span className="text-ink text-xs font-semibold tracking-tight">
          {label}
        </span>
        {badge && (
          <span className="rounded-md bg-line-soft px-1.5 py-0.5 font-mono text-meta text-dim">
            {badge}
          </span>
        )}
      </div>
      <div className="min-w-0 flex-1 sm:text-right">{children}</div>
    </div>
  )
}

export function DimNote({
  children,
  tone,
}: {
  children: ReactNode
  // 2026-09-18 收口：新增 danger 档——升级失败与 ClientUpdateCard failed
  // 曾 warn/danger 两色并存，统一走 danger。
  tone?: "warn" | "accent" | "danger"
}) {
  const cls =
    tone === "warn"
      ? "text-warn text-xs"
      : tone === "accent"
        ? "text-brand-deep text-xs font-medium"
        : tone === "danger"
          ? "text-danger text-xs font-medium"
          : "text-faint text-xs"
  return <span className={cls}>{children}</span>
}
