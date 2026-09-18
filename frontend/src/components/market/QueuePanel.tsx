// components/market/QueuePanel.tsx —— 下载管理面板（095 #4 / ADR-0011 队列形态）：
// 队列项状态一览。角标 = 未终结项数。2026-09-09（ADR-0013）：构建脚本改默认
// 批准，审批门内联审核（blocked_gate 相位）已退役——安装不再有「待审批」中间态。
// 2026-09-09（问题记录-2026-09-09 §1.2）：面板从手写 useState 下拉换成 Radix
// Popover——点面板外部 / 按 ESC 都能收起（原先只能再点一次触发按钮，不符合
// 系统内其它浮层的习惯）。
// 2026-09-09（同记录 §1.1）：触发按钮登记为飞行动画锚点（胶囊飞到这里），
// 角标在入队瞬间弹跳、在胶囊落地瞬间再闪一圈。
import { useEffect, useRef } from "react"
import { motion } from "framer-motion"
import { Download, Inbox, LoaderCircle, X } from "lucide-react"
import { useQueueStore } from "@/stores/queueStore"
import { useI18n } from "@/stores/i18nStore"
import { useInstallFlightStore } from "@/stores/installFlightStore"
import { setQueueAnchor } from "@/lib/installFlight"
import type { QueueItem } from "@/lib/queue"
import { Button } from "@/components/ui/button"
import { StateBlock } from "@/components/ui/state-block"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"

type StatusKey = "queueStatusQueued" | "queueStatusInstalling" | "queueStatusDone" | "queueStatusFailed"

const STATUS_CHIP: Record<QueueItem["status"], { key: StatusKey; cls: string }> = {
  queued: { key: "queueStatusQueued", cls: "border-line bg-line-soft/60 text-faint" },
  installing: { key: "queueStatusInstalling", cls: "border-brand/30 bg-wash text-brand-deep" },
  done: { key: "queueStatusDone", cls: "border-ok/30 bg-ok-soft text-ok" },
  failed: { key: "queueStatusFailed", cls: "border-danger/30 bg-danger-soft text-danger" },
}

export function QueuePanel() {
  const { t } = useI18n()
  const items = useQueueStore((s) => s.items)
  const retry = useQueueStore((s) => s.retry)
  const dismiss = useQueueStore((s) => s.dismiss)
  const clearFinished = useQueueStore((s) => s.clearFinished)
  const landedAt = useInstallFlightStore((s) => s.landedAt)
  const triggerRef = useRef<HTMLButtonElement>(null)

  // 飞行动画的落点锚点：挂载即登记、卸载清空（Tab 切换/热更新都会走）。
  useEffect(() => {
    setQueueAnchor(triggerRef.current)
    return () => setQueueAnchor(null)
  }, [])

  const active = items.filter(
    (i) => i.status === "queued" || i.status === "installing",
  )
  const finished = items.some((i) => i.status === "done" || i.status === "failed")

  return (
    <Popover>
      <PopoverTrigger asChild>
        <button
          ref={triggerRef}
          type="button"
          className="border-line bg-panel text-dim hover:text-ink data-open:border-brand/40 data-open:bg-wash data-open:text-ink inline-flex items-center gap-1.5 rounded-xl border px-3 py-1.5 text-xs font-medium transition-all"
        >
          <Download className="size-3.5 text-brand-deep" />
          <span>{t.market.queueTitle}</span>
          <span className="relative inline-flex">
            {active.length > 0 && (
              // key 随计数变化重挂：入队瞬间弹一下（胶囊落地时再闪一圈，见下）
              <motion.span
                key={`badge-${active.length}`}
                initial={{ scale: 0.55 }}
                animate={{ scale: 1 }}
                transition={{ type: "spring", stiffness: 520, damping: 16 }}
                className="inline-flex rounded-full bg-brand px-1.5 py-0.5 text-meta font-semibold leading-none text-primary-foreground"
              >
                {active.length}
              </motion.span>
            )}
            {landedAt > 0 && (
              <motion.span
                key={`ring-${landedAt}`}
                aria-hidden
                initial={{ scale: 1, opacity: 0.5 }}
                animate={{ scale: 2.2, opacity: 0 }}
                transition={{ duration: 0.45, ease: "easeOut" }}
                className="bg-brand pointer-events-none absolute inset-0 rounded-full"
              />
            )}
          </span>
        </button>
      </PopoverTrigger>

      {/* 2026-09-18 收口：定宽 26rem 在窄窗口会横向溢出 → 视口自适应上限；
          空态统一走 StateBlock（带图标，与其他空态同款）。 */}
      <PopoverContent
        align="end"
        sideOffset={8}
        className="max-h-96 w-[min(26rem,90vw)] overflow-y-auto rounded-2xl p-3 shadow-xl"
      >
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
          <StateBlock tone="empty" icon={Inbox} title={t.market.queueEmpty} />
        ) : (
          <div className="space-y-2">
            {items.map((item) => {
              const chip = STATUS_CHIP[item.status]
              // 2026-09-15（R2）：卸载项复用 done 相位（成功=绿），但文案必须是
              // 「已卸载」——对用户说「已安装」是错的陈述，不是措辞偏好。
              const chipLabel =
                item.status === "done" && item.kind === "remove"
                  ? t.market.queueStatusRemoved
                  : t.market[chip.key]
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
                        {chipLabel}
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
                    <p className="text-danger mt-1.5 line-clamp-2 text-label" title={item.detail}>
                      {item.detail}
                    </p>
                  )}
                  {item.status === "failed" && (
                    <Button
                      size="xs"
                      variant="outline"
                      className="mt-1.5 text-label"
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
      </PopoverContent>
    </Popover>
  )
}
