// components/market/QueuePanel.tsx —— 下载管理面板（095 #4 / ADR-0011 队列形态）：
// 队列项状态一览。角标 = 未终结项数。2026-09-09（ADR-0013）：构建脚本改默认
// 批准，审批门内联审核（blocked_gate 相位）已退役——安装不再有「待审批」中间态。
import { useState } from "react"
import { Download, LoaderCircle, X } from "lucide-react"
import { useQueueStore } from "@/stores/queueStore"
import { useI18n } from "@/stores/i18nStore"
import type { QueueItem } from "@/lib/queue"
import { Button } from "@/components/ui/button"

type StatusKey = "queueStatusQueued" | "queueStatusInstalling" | "queueStatusDone" | "queueStatusFailed"

const STATUS_CHIP: Record<QueueItem["status"], { key: StatusKey; cls: string }> = {
  queued: { key: "queueStatusQueued", cls: "border-line bg-line-soft/60 text-faint" },
  installing: { key: "queueStatusInstalling", cls: "border-brand/30 bg-brand/10 text-brand-deep" },
  done: { key: "queueStatusDone", cls: "border-emerald-500/30 bg-emerald-500/10 text-emerald-600" },
  failed: { key: "queueStatusFailed", cls: "border-rose-500/30 bg-rose-500/10 text-rose-600" },
}

export function QueuePanel() {
  const { t } = useI18n()
  const items = useQueueStore((s) => s.items)
  const retry = useQueueStore((s) => s.retry)
  const dismiss = useQueueStore((s) => s.dismiss)
  const clearFinished = useQueueStore((s) => s.clearFinished)
  const [open, setOpen] = useState(false)

  const active = items.filter(
    (i) => i.status === "queued" || i.status === "installing",
  )
  const finished = items.some((i) => i.status === "done" || i.status === "failed")

  return (
    <div className="relative">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className={`inline-flex items-center gap-1.5 rounded-xl border px-3 py-1.5 text-xs font-medium transition-all ${
          open ? "border-brand/40 bg-wash text-ink" : "border-line bg-panel text-dim hover:text-ink"
        }`}
      >
        <Download className="size-3.5 text-brand-deep" />
        <span>{t.market.queueTitle}</span>
        {active.length > 0 && (
          <span className="rounded-full bg-brand px-1.5 py-0.5 text-meta font-semibold leading-none text-white">
            {active.length}
          </span>
        )}
      </button>

      {open && (
        <div className="absolute right-0 top-full z-40 mt-2 max-h-96 w-[26rem] overflow-y-auto rounded-2xl border border-line bg-panel p-3 shadow-xl">
          <div className="flex items-center justify-between px-1 pb-2">
            <span className="text-xs font-semibold text-ink">{t.market.queueTitle}</span>
            {finished && (
              <button
                type="button"
                onClick={clearFinished}
                className="text-faint hover:text-ink text-label transition-colors"
              >
                {t.market.queueClearDone}
              </button>
            )}
          </div>

          {items.length === 0 ? (
            <p className="text-faint px-1 py-6 text-center text-xs">{t.market.queueEmpty}</p>
          ) : (
            <div className="space-y-2">
              {items.map((item) => {
                const chip = STATUS_CHIP[item.status]
                return (
                  <div key={item.id} className="rounded-xl border border-line bg-wash/60 p-2.5">
                    <div className="flex items-center justify-between gap-2">
                      <div className="min-w-0 flex-1">
                        <div className="truncate font-mono text-xs font-medium text-ink" title={item.pkg}>
                          {item.pkg}
                        </div>
                        <div className="text-faint mt-0.5 truncate font-mono text-meta">
                          → {item.profile}
                        </div>
                      </div>
                      <div className="flex shrink-0 items-center gap-1.5">
                        <span
                          className={`inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-meta font-medium ${chip.cls}`}
                        >
                          {item.status === "installing" && (
                            <LoaderCircle className="size-3 animate-spin" />
                          )}
                          {t.market[chip.key]}
                        </span>
                        {(item.status === "done" || item.status === "failed") && (
                          <button
                            type="button"
                            onClick={() => dismiss(item.id)}
                            className="text-faint hover:text-ink transition-colors"
                            title={t.market.queueDismiss}
                          >
                            <X className="size-3.5" />
                          </button>
                        )}
                      </div>
                    </div>

                    {item.status === "failed" && item.detail && (
                      <p className="text-rose-600 mt-1.5 line-clamp-2 text-label" title={item.detail}>
                        {item.detail}
                      </p>
                    )}
                    {item.status === "failed" && (
                      <Button
                        size="sm"
                        variant="outline"
                        className="mt-1.5 h-6 rounded-lg px-2 text-label"
                        onClick={() => retry(item.id)}
                      >
                        {t.market.queueRetry}
                      </Button>
                    )}
                  </div>
                )
              })}
            </div>
          )}
        </div>
      )}
    </div>
  )
}
