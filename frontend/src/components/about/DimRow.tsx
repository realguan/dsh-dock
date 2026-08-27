// 维度行布局（About 运行环境区共用）：标签列 + 内容列，末行去分隔线。
import type { ReactNode } from "react"

export function DimRow({
  label,
  children,
}: {
  label: string
  children: ReactNode
}) {
  return (
    <div className="border-line flex min-h-[52px] items-center gap-3 border-b px-4 py-3 last:border-b-0">
      <span className="text-dim w-20 shrink-0 text-xs font-medium tracking-wide">
        {label}
      </span>
      <div className="min-w-0 flex-1">{children}</div>
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
      ? "text-warn ml-2 text-xs"
      : tone === "accent"
        ? "text-brand-deep ml-2 text-xs font-medium"
        : "text-faint ml-2 text-xs"
  return <span className={cls}>{children}</span>
}
