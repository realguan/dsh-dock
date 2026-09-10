// 错误卡（数据驱动渲染，原 index/selector showError 升级迁移）：
// actions[] 由后端下发 id 集合，前端只做 id→文案映射（未知 id 回退展示原文）；
// 调用方追加本地动作 reselect（返回重选）。支持一键复制诊断日志与微交互。
import { useState } from "react"
import { motion } from "framer-motion"
import {
  AlertTriangle,
  ChevronDown,
  Sparkles,
  Copy,
  Check,
  RefreshCw,
  Terminal,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { logger } from "@/lib/logger"
import { useCopy } from "@/hooks/useCopy"
import type { TerminalAction } from "@/types/ipc"
import type { BootErrorEvent } from "@/types/events"
import { useI18n } from "@/stores/i18nStore"
import { useBootStore } from "@/stores/bootStore"
import { Button } from "@/components/ui/button"

const INVOKABLE: ReadonlySet<string> = new Set(["retry", "upgrade", "upgrade_only"])

export function ErrorCard({
  payload,
  onReselect,
  diag = false,
  index,
}: {
  payload: BootErrorEvent
  /** 本页本地动作：返回重选（旧页 = location.reload()） */
  onReselect?: () => void
  /** diag 形态（启动页内嵌）：DIAG 头行 + 日志默认展开；边框收进容器 */
  diag?: boolean
  /** diag 头行右侧的 #NN 序号（多次错误自增，由父级计数） */
  index?: number
}) {
  const { t } = useI18n()
  const [pending, setPending] = useState<string | null>(null)
  const [actionError, setActionError] = useState<string | null>(null)
  const { copied, copy } = useCopy()
  const actions = payload.actions?.length ? payload.actions : ["retry"]
  // 2026-09-08（ADR-0012）：优先按结构化分类取本地化文案；后端文案作为兼容分支
  // （旧缓存载荷 / 未来新增的未识别 kind）。
  const kindCopy = payload.failure ? t.error.kinds[payload.failure.kind] : undefined
  const title = kindCopy?.title ?? payload.title ?? t.error.fallbackTitle
  const suggestion = kindCopy?.suggestion ?? payload.suggestion

  const actionLabel = (id: string): string => {
    return t.error.actions[id] ?? id
  }

  const run = (id: string) => {
    if (pending) return
    if (!INVOKABLE.has(id)) return
    setPending(id)
    setActionError(null)
    useBootStore.getState().clearError()
    api
      .terminalAction(id as TerminalAction)
      .catch((e) => {
        setPending(null)
        const msg = String(e instanceof Error ? e.message : e)
        setActionError(`${t.error.actionFailed}：${msg}（可返回重选）`)
      })
      .finally(() => setPending((p) => (p === id ? null : p)))
  }

  // 2026-09-08：原写法连 promise 都没接——写失败照样显示「已复制」。
  const handleCopyLog = async () => {
    if (!payload.log) return
    const outcome = await copy(payload.log)
    if (!outcome.ok) logger.warn("[boot]", "复制诊断日志失败", { error: outcome.error })
  }

  return (
    <motion.section
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.25, ease: "easeOut" }}
      className={`overflow-hidden rounded-2xl border bg-panel shadow-sm ${
        diag ? "w-full border-danger/30" : "mx-auto mt-6 w-full max-w-xl border-danger/35 shadow-md"
      }`}
      role="alert"
      data-failure-kind={payload.failure?.kind ?? "none"}
    >
      {/* 诊断状态头 */}
      <div className="flex items-center justify-between border-b border-danger/20 bg-danger-soft/40 px-4 py-2.5">
        <div className="flex items-center gap-2">
          <span className="flex size-5 items-center justify-center rounded-full bg-danger text-white">
            <AlertTriangle className="size-3" />
          </span>
          <span className="font-mono text-xs font-semibold tracking-wide text-danger">
            {diag ? t.error.diagHeader : t.error.cardHeader}
          </span>
        </div>
        <span className="font-mono text-label font-medium text-dim tabular-nums">
          #{typeof index === "number" ? String(index).padStart(2, "0") : "01"}
        </span>
      </div>

      <div className="p-5">
        <h2 className="text-base font-semibold tracking-tight text-ink">{title}</h2>

        {/* 错误详情 */}
        {payload.detail && (
          <div className="mt-2.5 rounded-xl border border-danger/20 bg-danger-soft/30 p-3 text-xs leading-relaxed text-dim break-words">
            {payload.detail}
          </div>
        )}

        {/* 建议解决方案 */}
        {suggestion && (
          <div className="mt-3 flex items-start gap-2.5 rounded-xl border border-brand/20 bg-wash/70 p-3 text-xs leading-relaxed text-dim">
            <Sparkles className="mt-0.5 size-3.5 shrink-0 text-brand-deep" />
            <div className="flex-1">
              <span className="font-semibold text-brand-deep">{t.error.suggestionLabel}</span>
              {suggestion}
            </div>
          </div>
        )}

        {actionError && (
          <div className="mt-3 rounded-lg bg-danger/10 p-2.5 text-xs text-danger break-words">
            {actionError}
          </div>
        )}

        {/* 行动按钮条 */}
        <div className="mt-4 flex flex-wrap items-center gap-2.5">
          {actions.map((a) => {
            const isPrimary = a === "retry" || a === "upgrade" || a === "upgrade_only"
            return (
              <Button
                key={a}
                size="sm"
                variant={isPrimary ? "default" : "outline"}
                disabled={pending !== null}
                onClick={() => run(a)}
                className="gap-1.5"
              >
                {pending === a ? (
                  <>
                    <RefreshCw className="size-3.5 animate-spin" />
                    <span>{actionLabel(a)}…</span>
                  </>
                ) : (
                  <>
                    <RefreshCw className="size-3.5" />
                    <span>{actionLabel(a)}</span>
                  </>
                )}
              </Button>
            )
          })}
          {onReselect && (
            <Button
              size="sm"
              variant="outline"
              disabled={pending !== null}
              onClick={onReselect}
              className="text-dim"
            >
              {t.error.actions.reselect}
            </Button>
          )}
        </div>

        {/* 原始终端日志折叠 */}
        {payload.log && (
          <details open={diag} className="group mt-4 overflow-hidden rounded-xl border border-line">
            <summary className="flex cursor-pointer select-none items-center justify-between bg-muted/40 px-3.5 py-2 text-xs text-dim transition-colors hover:bg-muted/70">
              <div className="flex items-center gap-1.5 font-mono text-label">
                <Terminal className="size-3 text-faint" />
                <span>
                  原始诊断日志 · 尾部 {payload.log.split("\n").filter(Boolean).length} 行
                </span>
              </div>
              <ChevronDown className="size-3.5 text-faint transition-transform group-open:rotate-180" />
            </summary>
            <div className="relative border-t border-term-line bg-term p-3">
              <button
                type="button"
                onClick={handleCopyLog}
                className="absolute top-2.5 right-2.5 inline-flex items-center gap-1 rounded-md border border-white/10 bg-white/10 px-2 py-1 font-mono text-meta text-white/80 transition-colors hover:bg-white/20"
                title="复制日志"
              >
                {copied ? (
                  <>
                    <Check className="size-3 text-term-ok" />
                    <span>已复制</span>
                  </>
                ) : (
                  <>
                    <Copy className="size-3" />
                    <span>复制日志</span>
                  </>
                )}
              </button>
              <pre className="max-h-60 overflow-x-auto font-mono text-label leading-relaxed text-term-ok whitespace-pre-wrap">
                {payload.log}
              </pre>
            </div>
          </details>
        )}
      </div>
    </motion.section>
  )
}

