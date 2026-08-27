// 错误卡（数据驱动渲染，原 index/selector showError 迁移）：
// actions[] 由后端下发 id 集合，前端只做 id→文案映射（未知 id 回退展示原文）；
// 调用方追加本地动作 reselect（返回重选）。三次 Rust 动作外发统一走本卡。
import { useState } from "react"
import { motion } from "framer-motion"
import { AlertTriangle, ChevronDown } from "lucide-react"
import { api } from "@/lib/tauri"
import type { TerminalAction } from "@/types/ipc"
import type { BootErrorEvent } from "@/types/events"
import { t } from "@/content/zh-CN"
import { Button } from "@/components/ui/button"

const INVOKABLE: ReadonlySet<string> = new Set(["retry", "upgrade", "upgrade_only"])

function actionLabel(id: string): string {
  return t.error.actions[id] ?? id
}

export function ErrorCard({
  payload,
  onReselect,
}: {
  payload: BootErrorEvent
  /** 本页本地动作：返回重选（旧页 = location.reload()） */
  onReselect?: () => void
}) {
  const [pending, setPending] = useState<string | null>(null)
  const [actionError, setActionError] = useState<string | null>(null)
  const actions = payload.actions?.length ? payload.actions : ["retry"]
  const title = payload.title || t.error.fallbackTitle

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

  return (
    <motion.section
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.22, ease: "easeOut" }}
      className="border-warn-soft bg-panel mx-auto mt-6 w-full max-w-xl rounded-xl border p-5 shadow-sm"
      role="alert"
    >
      <div className="flex items-start gap-3">
        <span className="bg-warn-soft text-warn mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-lg">
          <AlertTriangle className="size-4" />
        </span>
        <div className="min-w-0">
          <div className="text-ink text-base font-semibold">{title}</div>
          {payload.detail && (
            <div className="text-dim mt-1 text-sm break-words">{payload.detail}</div>
          )}
          {payload.suggestion && (
            <div className="text-dim mt-2 text-xs leading-relaxed">{payload.suggestion}</div>
          )}
          {actionError && (
            <div className="text-warn mt-2 text-xs break-words">{actionError}</div>
          )}

          <div className="mt-4 flex flex-wrap items-center gap-2">
            {actions.map((a) => (
              <Button
                key={a}
                size="sm"
                variant={a === "upgrade" ? "default" : "outline"}
                disabled={pending !== null}
                onClick={() => run(a)}
              >
                {pending === a ? `${actionLabel(a)}…` : actionLabel(a)}
              </Button>
            ))}
            {onReselect && (
              <Button size="sm" variant="ghost" disabled={pending !== null} onClick={onReselect}>
                {t.error.actions.reselect}
              </Button>
            )}
          </div>

          {payload.log && (
            <details className="group border-line mt-3 rounded-lg border">
              <summary className="text-faint flex cursor-pointer select-none items-center gap-1 px-3 py-2 text-xs">
                原始日志
                <ChevronDown className="size-3 transition-transform group-open:rotate-180" />
              </summary>
              <pre className="border-line overflow-x-auto border-t px-3 py-2 font-mono text-[11px] leading-relaxed whitespace-pre-wrap">
                {payload.log}
              </pre>
            </details>
          )}
        </div>
      </div>
    </motion.section>
  )
}
