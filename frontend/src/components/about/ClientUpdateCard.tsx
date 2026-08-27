// 客户端自更新状态机卡（原 ui/about.html renderUpd 迁移）。
// 数据源：clientUpdateStore（app:update 事件驱动，页面进入时播种）；
// 前端零裁决——phase 由 Rust 写入，本组件只做「phase → 文案/控件」映射。
// 点击后的本地 busy 不需要单独 state：run_check/run_download_and_install
// 起手即回推 checking/downloading 事件，AnimatePresence 以 phase 为 key 过渡。
import { useEffect } from "react"
import { AnimatePresence, motion } from "framer-motion"
import { api } from "@/lib/tauri"
import { fmtBytes } from "@/lib/format"
import { t } from "@/content/zh-CN"
import { useClientUpdateStore } from "@/stores/clientUpdateStore"
import { Badge } from "@/components/ui/badge"
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

function StateBadge({ phase }: { phase: string }) {
  const tone = phaseTone(phase)
  const cls =
    tone === "ok"
      ? "bg-ok-soft text-ok border-transparent"
      : tone === "accent"
        ? "bg-wash text-brand-deep border-transparent"
        : tone === "warn"
          ? "bg-warn-soft text-warn border-transparent"
          : tone === "busy"
            ? "bg-line-soft text-dim border-transparent"
            : "bg-panel text-faint border-line"
  return (
    <Badge variant="outline" className={cls}>
      {t.about.phases[phase as keyof typeof t.about.phases]}
    </Badge>
  )
}

export function ClientUpdateCard() {
  const snapshot = useClientUpdateStore((s) => s.snapshot)
  const phase = snapshot?.phase ?? "idle"

  // done → 1.2s 后自动复查一次（沿用旧页语义：装完立即校准到最新态）
  useEffect(() => {
    if (phase !== "done") return
    const timer = window.setTimeout(() => {
      api.clientUpdateCheck().catch(() => {})
    }, 1200)
    return () => window.clearTimeout(timer)
  }, [phase])

  const busy = BUSY_PHASES.has(phase)
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
    <div className="border-line bg-panel rounded-xl border p-4 shadow-sm">
      <div className="mb-2 flex items-center justify-between">
        <span className="text-dim text-sm font-medium">{t.about.clientLabel}</span>
        <StateBadge phase={phase} />
      </div>

      <AnimatePresence mode="wait" initial={false}>
        <motion.div
          key={phase}
          initial={{ opacity: 0, y: 6 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -4 }}
          transition={{ duration: 0.18, ease: "easeOut" }}
        >
          <p
            className={
              phase === "failed"
                ? "text-warn text-lg font-semibold"
                : phase === "available" || phase === "done"
                  ? "text-ink text-lg font-semibold"
                  : "text-ink text-base font-medium"
            }
          >
            {mainLine}
          </p>

          {snapshot &&
          snapshot.phase === "available" &&
          typeof snapshot.notes === "string" &&
          snapshot.notes && (
            <p className="text-faint mt-1 line-clamp-2 text-xs">
              {t.about.releaseNotes}：{snapshot.notes.slice(0, 120)}
              {snapshot.notes.length > 120 ? "…" : ""}
            </p>
          )}
          {phase === "failed" && snapshot && snapshot.phase === "failed" && (
            <p className="text-dim mt-1 text-xs break-all">{snapshot.message}</p>
          )}

          {phase === "downloading" && snapshot && snapshot.phase === "downloading" && (
            <div className="mt-3">
              {snapshot.total != null && snapshot.total > 0 ? (
                <>
                  <Progress
                    value={Math.min(100, ((snapshot.current ?? 0) / snapshot.total) * 100)}
                  />
                  <div className="text-faint mt-1 flex justify-between text-xs">
                    <span>
                      {fmtBytes(snapshot.current ?? 0)} / {fmtBytes(snapshot.total)}
                    </span>
                    <span>
                      {Math.floor(((snapshot.current ?? 0) / snapshot.total) * 100)}%
                    </span>
                  </div>
                </>
              ) : (
                <>
                  <div className="pulse-bar">
                    <div className="pulse-bar-fill" />
                  </div>
                  <div className="text-faint mt-1 flex justify-between text-xs">
                    <span>{fmtBytes(snapshot.current ?? 0)}</span>
                    <span>获取中…</span>
                  </div>
                </>
              )}
            </div>
          )}
        </motion.div>
      </AnimatePresence>

      {/* 动作区：进行中隐藏全部按钮避免并发；其余状态常驻「检查更新」 */}
      {!busy && (
        <div className="mt-3 flex items-center gap-2">
          {phase === "available" ? (
            <Button size="sm" onClick={() => api.clientUpdateApply().catch(() => {})}>
              {t.about.downloadBtn}
              {latest ? ` v${latest}` : ""}
            </Button>
          ) : (
            <Button
              size="sm"
              variant="outline"
              onClick={() => api.clientUpdateCheck().catch(() => {})}
            >
              {t.about.checkBtn}
            </Button>
          )}
        </div>
      )}
    </div>
  )
}
