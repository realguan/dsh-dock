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
    <motion.div
      layout="position"
      className={`group flex items-start gap-3.5 px-3 py-2.5 rounded-lg transition-colors ${
        isRunning ? "bg-wash/60" : "hover:bg-line-soft/30"
      }`}
    >
      {/* 序号 / 状态指示器 */}
      <div className="relative mt-0.5 flex size-6 shrink-0 items-center justify-center">
        {isDone ? (
          <div className="flex size-6 items-center justify-center rounded-full border border-ok/30 bg-ok-soft text-ok shadow-xs">
            <Check className="size-3.5" strokeWidth={2.5} />
          </div>
        ) : isRunning ? (
          <div className="relative flex size-6 items-center justify-center rounded-full bg-brand text-white shadow-[0_0_12px_rgba(65,118,230,0.4)] ring-2 ring-brand/25">
            <Loader2 className="size-3.5 animate-spin" />
          </div>
        ) : isError ? (
          <div className="flex size-6 items-center justify-center rounded-full border border-warn/30 bg-warn-soft text-warn">
            <AlertCircle className="size-3.5" />
          </div>
        ) : (
          <div className="flex size-6 items-center justify-center rounded-full border border-line bg-line-soft/60 font-mono text-[10px] font-medium text-faint tabular-nums">
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
                  ? "font-medium text-warn"
                  : isDone
                    ? "font-medium text-ink/80"
                    : "font-normal text-faint"
            }`}
          >
            {name}
          </span>
          {isRunning && (
            <span className="inline-flex items-center gap-1 rounded-full bg-brand/10 px-1.5 py-0.2 text-[10px] font-medium text-brand">
              <span className="size-1 animate-pulse rounded-full bg-brand" />
              执行中
            </span>
          )}
        </div>

        <div className="mt-1">
          {detail ? (
            <span className="inline-block rounded border border-brand/20 bg-panel px-1.5 py-0.5 font-mono text-[11px] text-dim shadow-2xs">
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

