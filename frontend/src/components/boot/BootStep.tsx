// 时间线单步指示器（原 index.html .tstep 迁移）。
// 状态视觉：pending=faint / running=品牌蓝呼吸点 + 名字加重 / done=绿勾 /
// error=警示橙。detail 存在时替换 hint 行（后端遥测的可行动文案优先）。
import { Check } from "lucide-react"
import type { BootStepState } from "@/types/events"

export function BootStep({
  no,
  name,
  hint,
  detail,
  status,
}: {
  no: string
  name: string
  hint: string
  detail?: string
  status: BootStepState
}) {
  const isDone = status === "done"
  const isRunning = status === "running"
  const isError = status === "error"

  return (
    <div className="flex items-start gap-3 py-2">
      {/* 序号/状态位 */}
      <span
        className={`mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-md font-mono text-[10px] tabular-nums ${
          isError
            ? "bg-warn-soft text-warn"
            : isRunning
              ? "bg-brand text-white"
              : isDone
                ? "bg-ok-soft text-ok"
                : "text-faint bg-line-soft/70"
        }`}
      >
        {isDone ? <Check className="size-3.5" strokeWidth={2.5} /> : no}
      </span>

      <span className="min-w-0 flex-1 leading-snug">
        <span
          className={`block text-[13px] ${
            isRunning ? "text-ink font-semibold" : isError ? "text-warn font-medium" : isDone ? "text-dim" : "text-faint"
          }`}
        >
          {name}
          {isRunning && (
            <span className="animate-blink text-brand ml-2 inline-block align-middle text-[9px]">●</span>
          )}
        </span>
        <span className={`mt-0.5 block text-xs ${detail ? "text-dim" : "text-faint"}`}>
          {detail || hint}
        </span>
      </span>
    </div>
  )
}
