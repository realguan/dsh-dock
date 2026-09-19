// info-tip.tsx —— 解释性文案的收口形态（2026-09-19 裁定：长说明不平铺，图标 + 悬浮）。
//
// 起因：MCP 表单一次弹出 7 段说明、列表每行最多挂 4 个告警盒，"要填什么"被"为什么"
// 淹没。裁定口径 = **结论留在表面，原因挂悬浮**：状态与错误仍然看得见，只有解释性
// 长句收进 Tip。所以这里刻意不做"整块徽标当触发器"的变体——那会让结论只剩悬浮，
// 而悬浮是**扫不到**的。
//
// 为什么自带 `TooltipProvider`：本仓库每个窗口是独立 JS runtime（AGENTS §4.4 第三
// 红线），全局挂载点不止一处，漏一处就是"这个窗口的悬浮不出"。逐点包一层的影响面
// 比在各 root 各挂一次小。`delayDuration=0` 已在 ui/tooltip.tsx 的 Provider 里设。
//
// 键盘可达性：悬浮不能只对鼠标可用，触发器因此是**可聚焦的 button**（ESC 可关）。
// 但"可聚焦"≠"被聚焦就该打开"——Radix 1.2.16 的 `TooltipTrigger.onFocus` 无条件
// 调 `context.onOpen()`（没有 `:focus-visible` 闸门），于是**程序化焦点**（弹窗挂载
// 时的 autofocus，见 ui/dialog.tsx）也会弹说明。现只放键盘焦点进来：非
// `:focus-visible` 的 focus 事件用 `preventDefault()` 掐掉 Radix 的后续 handler
// （`composeEventHandlers` 见 defaultPrevented 即跳过；focus 本不可取消，无副作用）。
// 这条**不能当成 dialog.tsx 的重复保险而删掉**：本机实测键盘模态下的程序化 focus
// 同样命中 `:focus-visible`，两条闸门各拦一半——dialog 管"焦点该落在哪"，这里管
// "这种焦点该不该开悬浮"。
import { Info } from "lucide-react"

import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip"

export function Tip({
  text,
  label,
  side = "top",
  className,
}: {
  /** 悬浮里的那段说明（一律来自字典，不在此写文案）。 */
  text: string
  /** 图标按钮的无障碍名称（它没有可见文字）。 */
  label: string
  side?: "top" | "bottom" | "left" | "right"
  /** 悬浮内容层的附加类名（长段落里要嵌代码/换行时用）。 */
  className?: string
}) {
  return (
    <TooltipProvider>
      <Tooltip>
        <TooltipTrigger
          asChild
          onFocus={(event) => {
            if (!event.currentTarget.matches(":focus-visible"))
              event.preventDefault()
          }}
        >
          <button
            type="button"
            aria-label={label}
            className="inline-flex size-4 shrink-0 cursor-help items-center justify-center rounded-full text-faint transition-colors hover:text-ink focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand/40"
          >
            <Info className="size-3.5" />
          </button>
        </TooltipTrigger>
        <TooltipContent side={side} className={className}>
          {text}
        </TooltipContent>
      </Tooltip>
    </TooltipProvider>
  )
}
