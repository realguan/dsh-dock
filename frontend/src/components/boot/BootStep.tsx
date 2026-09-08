// 时间线单步行（原 index.html .tstep 升级迁移；2026-09-07 单主角重构）。
// done/pending 收成单行（名字 + 右对齐说明，悬停看全文）；running 行保留
// 强调与完整遥测详情（等宽可选中 + 一键复制）；error 行警示态、详情展开。
// 状态由图标 + 文字双重表达；竖向导轨由 BootTimeline 容器层统一绘制，
// 节点自带底色遮罩，行高变化不再撕裂连接线。
import { motion } from "framer-motion"
import { Check, Loader2, AlertCircle, Copy } from "lucide-react"
import type { BootStepState } from "@/types/events"
import { useI18n } from "@/stores/i18nStore"
import { useCopy } from "@/hooks/useCopy"
import { logger } from "@/lib/logger"

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
  const { t } = useI18n()
  const { copied, copy } = useCopy()
  const isDone = status === "done"
  const isRunning = status === "running"
  const isError = status === "error"
  const sideNote = detail ?? hint

  // 2026-09-08：写失败不再假装「已复制」（原写法 `.catch(() => {})` + 立即置位）
  const copyDetail = async () => {
    if (!detail) return
    const outcome = await copy(detail)
    if (!outcome.ok) logger.warn("[boot]", "复制步骤详情失败", { error: outcome.error })
  }

  return (
    <motion.div
      layout="position"
      className={`relative flex items-start gap-3 rounded-xl px-3 transition-colors duration-200 ${
        isRunning ? "bg-wash/70 py-2.5 shadow-2xs" : "py-2 hover:bg-line-soft/40"
      }`}
    >
      {/* 状态节点 */}
      <div className="relative z-1 mt-0.5 flex size-6 shrink-0 items-center justify-center">
        {isDone ? (
          <div className="flex size-6 items-center justify-center rounded-full border border-ok/30 bg-ok-soft text-ok shadow-2xs">
            <Check className="size-3.5" strokeWidth={2.5} />
          </div>
        ) : isRunning ? (
          <div className="flex size-6 items-center justify-center rounded-full bg-brand text-white shadow-[0_0_12px_color-mix(in_srgb,var(--color-brand)_45%,transparent)] ring-3 ring-brand/20">
            <Loader2 className="size-3.5 animate-spin motion-reduce:animate-none" />
          </div>
        ) : isError ? (
          <div className="flex size-6 items-center justify-center rounded-full border border-warn/30 bg-warn-soft text-warn shadow-2xs">
            <AlertCircle className="size-3.5" />
          </div>
        ) : (
          <div className="flex size-6 items-center justify-center rounded-full border border-line bg-panel font-mono text-[10px] font-semibold text-faint tabular-nums shadow-2xs">
            {no}
          </div>
        )}
      </div>

      {/* 步骤文本与遥测详情 */}
      <div className="min-w-0 flex-1 leading-snug">
        <div className="flex min-w-0 items-baseline gap-2">
          <span
            className={`shrink-0 text-[13px] tracking-tight transition-colors ${
              isRunning
                ? "font-semibold text-ink"
                : isError
                  ? "font-semibold text-warn"
                  : isDone
                    ? "font-medium text-ink/75"
                    : "font-normal text-dim"
            }`}
          >
            {name}
          </span>
          {isRunning && (
            <span className="inline-flex shrink-0 items-center gap-1 rounded-full border border-brand/20 bg-brand/10 px-1.5 py-0.5 font-mono text-[10px] font-medium text-brand">
              <span className="size-1 animate-pulse rounded-full bg-brand" aria-hidden />
              {t.boot.stRunning}
            </span>
          )}
          {!isRunning && !isError && (
            <span
              title={sideNote}
              className="ml-auto min-w-0 truncate pl-2 text-right text-xs text-faint"
            >
              {sideNote}
            </span>
          )}
        </div>

        {isRunning && detail && (
          <div className="mt-1.5 flex items-start gap-1.5">
            <code
              title={detail}
              className="min-w-0 flex-1 select-all break-all rounded-md bg-panel/70 px-2 py-1 font-mono text-[11px] leading-relaxed text-dim"
            >
              {detail}
            </code>
            <button
              type="button"
              title={copied ? t.boot.copied : t.boot.copyDetail}
              aria-label={copied ? t.boot.copied : t.boot.copyDetail}
              onClick={copyDetail}
              className="mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-md text-faint transition-colors hover:bg-line-soft hover:text-ink"
            >
              {copied ? <Check className="size-3.5 text-ok" /> : <Copy className="size-3.5" />}
            </button>
          </div>
        )}

        {isError && detail && (
          <p className="mt-1 break-all text-xs leading-relaxed text-warn/90">{detail}</p>
        )}
      </div>
    </motion.div>
  )
}
