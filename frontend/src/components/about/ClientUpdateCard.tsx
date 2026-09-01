// 客户端自更新状态机卡（更新中心重构）。
// 包含流光下载进度条、Release Notes 展开与状态化动作按钮。
import { useEffect, useState } from "react"
import {
  ArrowDownCircle,
  CheckCircle2,
  Download,
  FileText,
  LoaderCircle,
  RefreshCw,
  Sparkles,
} from "lucide-react"
import { AnimatePresence, motion } from "framer-motion"
import { api } from "@/lib/tauri"
import { fmtBytes } from "@/lib/format"
import { useI18n } from "@/stores/i18nStore"
import { useClientUpdateStore } from "@/stores/clientUpdateStore"
import { Button } from "@/components/ui/button"
import { Progress } from "@/components/ui/progress"

const BUSY_PHASES = new Set(["checking", "downloading", "installing", "relaunching"])

type PhaseTone = "idle" | "busy" | "ok" | "accent" | "warn"

function phaseTone(phase: string): PhaseTone {
  if (phase === "upToDate" || phase === "done") return "ok"
  if (phase === "available") return "accent"
  if (phase === "failed") return "warn"
  if (BUSY_PHASES.has(phase)) return "busy"
  return "idle"
}

export function ClientUpdateCard() {
  const { t } = useI18n()
  const snapshot = useClientUpdateStore((s) => s.snapshot)
  const phase = snapshot?.phase ?? "idle"
  const [expandNotes, setExpandNotes] = useState(false)

  // done → 1.2s 后自动复查一次
  useEffect(() => {
    if (phase !== "done") return
    const timer = window.setTimeout(() => {
      api.clientUpdateCheck().catch(() => {})
    }, 1200)
    return () => window.clearTimeout(timer)
  }, [phase])

  const busy = BUSY_PHASES.has(phase)
  const tone = phaseTone(phase)
  const latest =
    snapshot && "latest" in snapshot && typeof snapshot.latest === "string"
      ? snapshot.latest
      : null

  const mainLine = (() => {
    switch (phase) {
      case "available":
        return latest ? `${t.about.foundNew} v${latest}` : t.about.lines.checking
      case "done":
        return `${t.about.updatedDone} ${snapshot && "version" in snapshot ? `v${snapshot.version}` : ""}`
      case "failed":
        return t.about.lines.failedTitle
      case "checking":
      case "downloading":
      case "installing":
      case "relaunching":
      case "upToDate":
        return t.about.lines[phase]
      default:
        return t.about.lines.idle
    }
  })()

  return (
    <div className="border-line bg-panel rounded-2xl border p-4.5 shadow-xs transition-shadow hover:shadow-sm">
      {/* 顶栏：标题 + 状态胶囊 */}
      <div className="mb-3 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div className="flex size-7 items-center justify-center rounded-lg bg-brand/10 text-brand">
            <Sparkles className="size-3.5" />
          </div>
          <div>
            <h3 className="text-ink text-xs font-bold tracking-tight">
              {t.about.clientLabel}
            </h3>
            <p className="text-faint text-[10px]">{t.about.officialChannel}</p>
          </div>
        </div>

        <span
          className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium leading-none ${
            tone === "ok"
              ? "bg-ok-soft text-ok"
              : tone === "accent"
                ? "bg-brand/10 text-brand border border-brand/20"
                : tone === "warn"
                  ? "bg-warn-soft text-warn"
                  : tone === "busy"
                    ? "bg-line-soft text-dim"
                    : "bg-bg text-faint border border-line"
          }`}
        >
          {tone === "ok" && <CheckCircle2 className="size-2.5" />}
          {tone === "accent" && <ArrowDownCircle className="size-2.5" />}
          {tone === "busy" && <LoaderCircle className="size-2.5 animate-spin" />}
          <span>{t.about.phases[phase as keyof typeof t.about.phases]}</span>
        </span>
      </div>

      <AnimatePresence mode="wait" initial={false}>
        <motion.div
          key={phase}
          initial={{ opacity: 0, y: 4 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -4 }}
          transition={{ duration: 0.15, ease: "easeOut" }}
          className="space-y-2"
        >
          <p
            className={`text-sm font-semibold tracking-tight ${
              phase === "failed"
                ? "text-warn"
                : phase === "available" || phase === "done"
                  ? "text-brand-deep"
                  : "text-ink"
            }`}
          >
            {mainLine}
          </p>

          {/* Release Notes */}
          {snapshot &&
            snapshot.phase === "available" &&
            typeof snapshot.notes === "string" &&
            snapshot.notes && (
              <div className="rounded-xl border border-line bg-bg/80 p-3 text-xs text-dim space-y-1.5 shadow-2xs">
                <div className="text-faint flex items-center justify-between text-[10px] font-semibold border-b border-line/60 pb-1">
                  <div className="flex items-center gap-1.5 text-brand">
                    <FileText className="size-3" />
                    <span>{t.about.releaseNotes}</span>
                  </div>
                  {snapshot.notes.length > 150 && (
                    <button
                      type="button"
                      onClick={() => setExpandNotes(!expandNotes)}
                      className="text-brand hover:underline cursor-pointer select-none"
                    >
                      {expandNotes ? "收起日志" : "展开全部"}
                    </button>
                  )}
                </div>
                <div
                  className={`text-[11px] leading-relaxed whitespace-pre-wrap font-mono text-ink/90 overflow-y-auto transition-all ${
                    expandNotes ? "max-h-60" : "max-h-24"
                  }`}
                >
                  {snapshot.notes}
                </div>
              </div>
            )}

          {/* 错误详情 */}
          {phase === "failed" && snapshot && snapshot.phase === "failed" && (
            <div className="rounded-xl bg-warn-soft p-3 text-xs text-warn break-all">
              {snapshot.message}
            </div>
          )}

          {/* 下载进度条 */}
          {phase === "downloading" && snapshot && snapshot.phase === "downloading" && (
            <div className="mt-3 space-y-1.5">
              {snapshot.total != null && snapshot.total > 0 ? (
                <>
                  <Progress
                    value={Math.min(100, ((snapshot.current ?? 0) / snapshot.total) * 100)}
                    className="h-2 rounded-full"
                  />
                  <div className="text-faint flex justify-between font-mono text-[11px]">
                    <span>
                      {fmtBytes(snapshot.current ?? 0)} / {fmtBytes(snapshot.total)}
                    </span>
                    <span className="font-semibold text-ink">
                      {Math.floor(((snapshot.current ?? 0) / snapshot.total) * 100)}%
                    </span>
                  </div>
                </>
              ) : (
                <>
                  <div className="pulse-bar">
                    <div className="pulse-bar-fill" />
                  </div>
                  <div className="text-faint flex justify-between font-mono text-[11px]">
                    <span>{fmtBytes(snapshot.current ?? 0)}</span>
                    <span>正在获取资源…</span>
                  </div>
                </>
              )}
            </div>
          )}
        </motion.div>
      </AnimatePresence>

      {/* 动作区 */}
      <div className="mt-3.5 flex items-center gap-2 pt-1 border-t border-line/60">
        {phase === "available" && !busy ? (
          <Button
            size="sm"
            onClick={() => api.clientUpdateApply().catch(() => {})}
            className="gap-1.5 text-xs font-semibold"
          >
            <Download className="size-3.5" />
            <span>
              {t.about.downloadBtn}
              {latest ? ` (v${latest})` : ""}
            </span>
          </Button>
        ) : (
          <Button
            size="sm"
            variant="outline"
            disabled={busy}
            onClick={() => api.clientUpdateCheck().catch(() => {})}
            className="gap-1.5 text-xs"
          >
            <RefreshCw
              className={`size-3.5 ${phase === "checking" ? "animate-spin text-brand" : ""}`}
            />
            <span>{phase === "checking" ? t.about.phases.checking : t.about.checkBtn}</span>
          </Button>
        )}
      </div>
    </div>
  )
}
