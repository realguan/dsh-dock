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
  variant = "tabs",
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
  /**
   * 交互语义（2026-09-21 新增，为「可全部不选」的筛选芯片）：
   * · `tabs`（默认）—— 换视图：`role=tablist` + `aria-selected`，必有且仅有一个选中；
   * · `filters` —— 筛选芯片：`role=group` + `aria-pressed`。筛选允许"全不选"（= 不筛），
   *   而 tablist 在 a11y 上要求恒有选中项——用错语义会让读屏报出"没有选中的标签页"。
   *   点击已选中项是否取消，由调用方在 `onChange` 里决定。
   */
  variant?: "tabs" | "filters"
}) {
  const isFilter = variant === "filters"
  return (
    <div
      role={isFilter ? "group" : "tablist"}
      aria-label={ariaLabel}
      className={cn(
        "flex border border-line bg-line-soft",
        size === "md" ? "rounded-xl p-0.5" : "rounded-lg p-0.5",
        // 非 stretch 时**整体不可压缩**：2026-09-21 真机截图里筛选项被 flex 父容器
        // 压窄后中文逐字折行（「全部」渲染成「全 / 部」），芯片宽度必须由内容决定。
        stretch ? "w-full" : "w-fit shrink-0",
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
            {...(isFilter
              ? { "aria-pressed": active }
              : { role: "tab", "aria-selected": active })}
            onClick={() => onChange(o.value)}
            className={cn(
              // whitespace-nowrap：芯片文字一律单行（折行的芯片不再是芯片）。
              "flex items-center gap-1.5 font-medium whitespace-nowrap transition-all",
              size === "md"
                ? "rounded-lg px-3 py-1.5 text-xs"
                : "rounded-md px-2.5 py-1 text-label",
              stretch && "min-w-0 flex-1 justify-center",
              active ? "bg-panel text-ink shadow-xs" : "text-dim hover:text-ink",
            )}
          >
            {Icon ? (
              <Icon className={cn("shrink-0", size === "md" ? "size-3.5" : "size-3")} />
            ) : null}
            {o.label}
          </button>
        )
      })}
    </div>
  )
}
