// segmented.tsx —— 统一的分段选择器（2026-09-18 收口：全站曾并存 5 套配方）。
// 配方取健康大盘日志源一版：凹陷容器 + 抬起选中片。
import type { ReactNode } from "react"
import type { LucideIcon } from "lucide-react"

import { cn } from "@/lib/utils"

export interface SegmentedOption<T extends string> {
  value: T
  label: ReactNode
  icon?: LucideIcon
}

export function Segmented<T extends string>({
  options,
  value,
  onChange,
  ariaLabel,
  className,
}: {
  options: SegmentedOption<T>[]
  value: T
  onChange: (v: T) => void
  ariaLabel?: string
  className?: string
}) {
  return (
    <div
      role="tablist"
      aria-label={ariaLabel}
      className={cn(
        "flex w-fit items-center rounded-xl border border-line bg-line-soft p-0.5",
        className,
      )}
    >
      {options.map((o) => {
        const active = o.value === value
        const Icon = o.icon
        return (
          <button
            key={o.value}
            type="button"
            role="tab"
            aria-selected={active}
            onClick={() => onChange(o.value)}
            className={cn(
              "flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition-all",
              active
                ? "bg-panel text-ink shadow-xs"
                : "text-dim hover:text-ink",
            )}
          >
            {Icon ? <Icon className="size-3.5" /> : null}
            {o.label}
          </button>
        )
      })}
    </div>
  )
}
