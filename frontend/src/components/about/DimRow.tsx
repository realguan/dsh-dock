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
          <span className="rounded bg-line-soft px-1.5 py-0.5 font-mono text-[10px] text-dim">
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
  tone?: "warn" | "accent"
}) {
  const cls =
    tone === "warn"
      ? "text-warn text-xs"
      : tone === "accent"
        ? "text-brand-deep text-xs font-medium"
        : "text-faint text-xs"
  return <span className={cls}>{children}</span>
}
