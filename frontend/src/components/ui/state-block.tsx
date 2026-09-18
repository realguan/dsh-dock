// state-block.tsx —— 统一的空 / 加载 / 错误态原语
// （2026-09-18 收口：此前有图标版、纯 spinner 版、纯居中文字版并存）。
import type { ReactNode } from "react"
import { AlertTriangle, LoaderCircle, type LucideIcon } from "lucide-react"

import { cn } from "@/lib/utils"

export function StateBlock({
  tone = "empty",
  icon: Icon,
  title,
  hint,
  action,
  className,
}: {
  tone?: "empty" | "loading" | "error"
  icon?: LucideIcon
  title: string
  hint?: ReactNode
  action?: ReactNode
  className?: string
}) {
  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-line px-6 py-10 text-center",
        tone === "error" && "border-danger/25 bg-danger-soft",
        className,
      )}
    >
      {tone === "loading" ? (
        <LoaderCircle className="size-5 animate-spin text-brand-deep" />
      ) : tone === "error" ? (
        <AlertTriangle className="size-5 text-danger" />
      ) : Icon ? (
        <Icon className="size-5 text-faint" />
      ) : null}
      <p
        className={cn(
          "text-xs",
          tone === "error" ? "font-medium text-danger" : "text-dim",
        )}
      >
        {title}
      </p>
      {hint ? <p className="max-w-sm text-label text-faint">{hint}</p> : null}
      {action ? <div className="mt-1">{action}</div> : null}
    </div>
  )
}
