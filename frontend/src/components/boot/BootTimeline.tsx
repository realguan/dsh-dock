import { useBootStore } from "@/stores/bootStore"
import { useI18n } from "@/stores/i18nStore"
import { BootStep } from "./BootStep"
import { Terminal, ShieldCheck, AlertTriangle, RefreshCw } from "lucide-react"

export function BootTimeline() {
  const { t } = useI18n()
  const steps = useBootStore((s) => s.steps)

  const consoleState = (stepsList: { status: string }[]): {
    label: string
    cls: string
    icon: typeof Terminal
  } | null => {
    if (stepsList.some((s) => s.status === "error"))
      return {
        label: t.boot.stError,
        cls: "bg-warn-soft text-warn border-warn/20",
        icon: AlertTriangle,
      }
    if (stepsList.some((s) => s.status === "running"))
      return {
        label: t.boot.stRunning,
        cls: "bg-wash text-brand-deep border-brand/20",
        icon: RefreshCw,
      }
    if (stepsList.every((s) => s.status === "done"))
      return {
        label: t.boot.stReady,
        cls: "bg-ok-soft text-ok border-ok/20",
        icon: ShieldCheck,
      }
    return null
  }

  const state = consoleState(steps)
  const doneCount = steps.filter((s) => s.status === "done").length

  return (
    <section className="w-full max-w-xl rounded-2xl border border-line/80 bg-panel/95 p-5 shadow-xs backdrop-blur-md">
      <div className="mb-2.5 flex items-center justify-between border-b border-line/70 pb-3.5">
        <div className="flex items-center gap-2.5">
          <span className="flex size-7 items-center justify-center rounded-lg border border-brand/20 bg-wash text-brand-deep shadow-2xs">
            <Terminal className="size-3.5" />
          </span>
          <span className="text-xs font-semibold tracking-wide text-ink">
            {t.boot.consoleTitle}
          </span>
          <span className="rounded-full border border-line bg-line-soft/80 px-2 py-0.5 font-mono text-[10px] font-medium text-dim shadow-2xs">
            {doneCount} / {steps.length}
          </span>
        </div>

        {state && (
          <div
            className={`inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 font-mono text-[11px] font-medium shadow-2xs ${state.cls}`}
          >
            <state.icon
              className={`size-3 ${state.label === t.boot.stRunning ? "animate-spin" : ""}`}
            />
            <span>{state.label}</span>
          </div>
        )}
      </div>

      <div className="flex flex-col gap-1">
        {t.boot.steps.map((def, i) => (
          <BootStep
            key={def.no}
            no={def.no}
            name={def.name}
            hint={def.hint}
            detail={steps[i]?.detail}
            status={steps[i]?.status ?? "pending"}
            isFirst={i === 0}
            isLast={i === t.boot.steps.length - 1}
          />
        ))}
      </div>
    </section>
  )
}

