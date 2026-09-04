// 时间线单步指示器（原 index.html .tstep 升级迁移）。
// 状态视觉：pending=faint / running=品牌蓝呼吸光环 + 名字加重 / done=绿勾 /
// error=警示橙。detail 存在时替换 hint 行（后端遥测的可行动文案优先展示为代码胶囊）。
import { motion } from "framer-motion"
import { Check, Loader2, AlertCircle } from "lucide-react"
import type { BootStepState } from "@/types/events"

export function BootStep({
  no,
  name,
  hint,
  detail,
  status,
  isFirst = false,
  isLast = false,
}: {
  no: string
  name: string
  hint: string
  detail?: string
  status: BootStepState
  isFirst?: boolean
  isLast?: boolean
}) {
  const isDone = status === "done"
  const isRunning = status === "running"
  const isError = status === "error"

  return (
    <motion.div
      layout="position"
      className={`group relative flex items-start gap-3.5 rounded-xl px-3 py-2.5 transition-all ${
        isRunning ? "bg-wash/70 shadow-2xs" : "hover:bg-line-soft/40"
      }`}
    >
      {/* 竖向流水线导轨与状态指示器 */}
      <div className="relative mt-0.5 flex size-6 shrink-0 items-center justify-center">
        {/* 顶部连接线 */}
        {!isFirst && (
          <div
            className={`absolute -top-3 left-1/2 -translate-x-1/2 w-0.5 h-3 transition-colors duration-300 ${
              isDone || isRunning ? "bg-ok/40" : "bg-line"
            }`}
          />
        )}
        {/* 底部连接线 */}
        {!isLast && (
          <div
            className={`absolute -bottom-3 left-1/2 -translate-x-1/2 w-0.5 h-3 transition-colors duration-300 ${
              isDone ? "bg-ok/40" : isRunning ? "bg-brand/40" : "bg-line"
            }`}
          />
        )}

        {/* 状态节点 */}
        {isDone ? (
          <div className="relative z-1 flex size-6 items-center justify-center rounded-full border border-ok/30 bg-ok-soft text-ok shadow-2xs">
            <Check className="size-3.5" strokeWidth={2.5} />
          </div>
        ) : isRunning ? (
          <div className="relative z-1 flex size-6 items-center justify-center rounded-full bg-brand text-white shadow-[0_0_12px_rgba(65,118,230,0.45)] ring-3 ring-brand/20">
            <Loader2 className="size-3.5 animate-spin" />
          </div>
        ) : isError ? (
          <div className="relative z-1 flex size-6 items-center justify-center rounded-full border border-warn/30 bg-warn-soft text-warn shadow-2xs">
            <AlertCircle className="size-3.5" />
          </div>
        ) : (
          <div className="relative z-1 flex size-6 items-center justify-center rounded-full border border-line bg-panel font-mono text-[10px] font-semibold text-faint tabular-nums shadow-2xs">
            {no}
          </div>
        )}
      </div>

      {/* 步骤文本与遥测详情 */}
      <div className="min-w-0 flex-1 leading-snug">
        <div className="flex items-center gap-2">
          <span
            className={`text-[13px] tracking-tight transition-colors ${
              isRunning
                ? "font-semibold text-ink"
                : isError
                  ? "font-semibold text-warn"
                  : isDone
                    ? "font-medium text-ink/80"
                    : "font-normal text-dim/70"
            }`}
          >
            {name}
          </span>
          {isRunning && (
            <span className="inline-flex items-center gap-1 rounded-full border border-brand/20 bg-brand/10 px-1.5 py-0.2 font-mono text-[10px] font-medium text-brand">
              <span className="size-1 animate-pulse rounded-full bg-brand" />
              运行中
            </span>
          )}
        </div>

        <div className="mt-1">
          {detail ? (
            <span className="inline-block max-w-full truncate rounded-md border border-brand/20 bg-panel px-2 py-0.5 font-mono text-[11px] text-dim shadow-2xs">
              {detail}
            </span>
          ) : (
            <span className="block text-xs text-faint">{hint}</span>
          )}
        </div>
      </div>
    </motion.div>
  )
}

