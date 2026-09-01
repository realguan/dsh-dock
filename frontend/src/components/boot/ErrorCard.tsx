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
import type { TerminalAction } from "@/types/ipc"
import type { BootErrorEvent } from "@/types/events"
import { useI18n } from "@/stores/i18nStore"
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
  const [copied, setCopied] = useState(false)
  const actions = payload.actions?.length ? payload.actions : ["retry"]
  const title = payload.title || t.error.fallbackTitle

  const actionLabel = (id: string): string => {
    return t.error.actions[id] ?? id
  }

  const run = (id: string) => {
    if (pending) return
    if (!INVOKABLE.has(id)) return
    setPending(id)
    setActionError(null)
    api
      .terminalAction(id as TerminalAction)
      .catch((e) => {
        setPending(null)
        const msg = String(e instanceof Error ? e.message : e)
        setActionError(`${t.error.actionFailed}：${msg}（可返回重选）`)
      })
      .finally(() => setPending((p) => (p === id ? null : p)))
  }

  const handleCopyLog = () => {
    if (!payload.log) return
    navigator.clipboard.writeText(payload.log)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <motion.section
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.25, ease: "easeOut" }}
      className={`overflow-hidden rounded-2xl border bg-panel shadow-sm ${
        diag ? "w-full border-warn/30" : "mx-auto mt-6 w-full max-w-xl border-warn/35 shadow-md"
      }`}
      role="alert"
    >
      {/* 诊断状态头 */}
      <div className="flex items-center justify-between border-b border-warn/20 bg-warn-soft/40 px-4 py-2.5">
        <div className="flex items-center gap-2">
          <span className="flex size-5 items-center justify-center rounded-full bg-warn text-white">
            <AlertTriangle className="size-3" />
          </span>
          <span className="font-mono text-xs font-semibold tracking-wide text-warn">
            {diag ? "DIAG 诊断控制台" : "启动中断"}
          </span>
        </div>
        <span className="font-mono text-[11px] font-medium text-dim tabular-nums">
          #{typeof index === "number" ? String(index).padStart(2, "0") : "01"}
        </span>
      </div>

      <div className="p-5">
        <h2 className="text-base font-semibold tracking-tight text-ink">{title}</h2>

        {/* 错误详情 */}
        {payload.detail && (
          <div className="mt-2.5 rounded-xl border border-warn/20 bg-warn-soft/30 p-3 text-xs leading-relaxed text-dim break-words">
            {payload.detail}
          </div>
        )}

        {/* 建议解决方案 */}
        {payload.suggestion && (
          <div className="mt-3 flex items-start gap-2.5 rounded-xl border border-brand/20 bg-wash/70 p-3 text-xs leading-relaxed text-dim">
            <Sparkles className="mt-0.5 size-3.5 shrink-0 text-brand-deep" />
            <div className="flex-1">
              <span className="font-semibold text-brand-deep">修复建议：</span>
              {payload.suggestion}
            </div>
          </div>
        )}

        {actionError && (
          <div className="mt-3 rounded-lg bg-warn/10 p-2.5 text-xs text-warn break-words">
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
              <div className="flex items-center gap-1.5 font-mono text-[11px]">
                <Terminal className="size-3 text-faint" />
                <span>
                  原始诊断日志 · 尾部 {payload.log.split("\n").filter(Boolean).length} 行
                </span>
              </div>
              <ChevronDown className="size-3.5 text-faint transition-transform group-open:rotate-180" />
            </summary>
            <div className="relative border-t border-line bg-badge-b p-3">
              <button
                type="button"
                onClick={handleCopyLog}
                className="absolute top-2.5 right-2.5 inline-flex items-center gap-1 rounded-md border border-white/10 bg-white/10 px-2 py-1 font-mono text-[10px] text-white/80 transition-colors hover:bg-white/20"
                title="复制日志"
              >
                {copied ? (
                  <>
                    <Check className="size-3 text-ok" />
                    <span>已复制</span>
                  </>
                ) : (
                  <>
                    <Copy className="size-3" />
                    <span>复制日志</span>
                  </>
                )}
              </button>
              <pre className="max-h-60 overflow-x-auto font-mono text-[11px] leading-relaxed text-emerald-400/90 whitespace-pre-wrap">
                {payload.log}
              </pre>
            </div>
          </details>
        )}
      </div>
    </motion.section>
  )
}

