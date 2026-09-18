// icon-chip.tsx —— 统一的「图标底」原语。
// 2026-09-18 收口：此前三种写法并存（size-7 rounded-lg bg-brand/15、
// size-7 rounded-lg bg-brand/10、size-8 rounded-xl + border）。
// 档位判据同圆角规则：控制件 10px（rounded-lg）。
import type { LucideIcon } from "lucide-react"

import { cn } from "@/lib/utils"

type IconChipTone = "brand" | "ok" | "warn" | "danger" | "neutral"

const toneClass: Record<IconChipTone, string> = {
  brand: "bg-wash text-brand-deep",
  ok: "bg-ok-soft text-ok",
  warn: "bg-warn-soft text-warn",
  danger: "bg-danger-soft text-danger",
  neutral: "bg-line-soft text-dim",
}

export function IconChip({
  icon: Icon,
  tone = "neutral",
  className,
}: {
  icon: LucideIcon
  tone?: IconChipTone
  className?: string
}) {
  return (
    <span
      className={cn(
        "flex size-7 shrink-0 items-center justify-center rounded-lg",
        toneClass[tone],
        className,
      )}
    >
      <Icon className="size-4" />
    </span>
  )
}
