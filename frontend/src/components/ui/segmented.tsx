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
  size = "md",
  stretch = false,
}: {
  options: SegmentedOption<T>[]
  value: T
  onChange: (v: T) => void
  ariaLabel?: string
  className?: string
  /**
   * 2026-09-19 审美批次 3：`md` = 换页面（顶栏全局导航），`sm` = 换面内内容
   * （子视图 / 筛选 / 分组）。此前两级同款同尺寸，插件中心顶栏与页面内的
   * 子 Tab 读起来像兄弟，分不清"哪个切换会离开当前页"。
   */
  size?: "md" | "sm"
  /** 等分铺满容器（详情面板的四段 Tab 用；不铺满时按内容收拢）。 */
  stretch?: boolean
}) {
  return (
    <div
      role="tablist"
      aria-label={ariaLabel}
      className={cn(
        "flex border border-line bg-line-soft",
        size === "md" ? "rounded-xl p-0.5" : "rounded-lg p-0.5",
        stretch ? "w-full" : "w-fit",
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
              "flex items-center gap-1.5 font-medium transition-all",
              size === "md"
                ? "rounded-lg px-3 py-1.5 text-xs"
                : "rounded-md px-2.5 py-1 text-label",
              stretch && "min-w-0 flex-1 justify-center",
              active ? "bg-panel text-ink shadow-xs" : "text-dim hover:text-ink",
            )}
          >
            {Icon ? (
              <Icon className={size === "md" ? "size-3.5" : "size-3"} />
            ) : null}
            {o.label}
          </button>
        )
      })}
    </div>
  )
}
