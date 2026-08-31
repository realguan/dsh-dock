// 启动详情卡头（原 index.html .console 升级迁移）：五步时间线 + 仪表盘控制头。
// 状态灯跟随全局：任一步 error=异常 > 任一 running=进行中 > 全 done=就绪。
import { useBootStore } from "@/stores/bootStore"
import { t } from "@/content/zh-CN"
import { BootStep } from "./BootStep"
import { Terminal, ShieldCheck, AlertTriangle, RefreshCw } from "lucide-react"

function consoleState(steps: { status: string }[]): {
  label: string
  cls: string
  icon: typeof Terminal
} | null {
  if (steps.some((s) => s.status === "error"))
    return {
      label: t.boot.stError,
      cls: "bg-warn-soft text-warn border-warn/20",
      icon: AlertTriangle,
    }
  if (steps.some((s) => s.status === "running"))
    return {
      label: t.boot.stRunning,
      cls: "bg-wash text-brand-deep border-brand/20",
      icon: RefreshCw,
    }
  if (steps.every((s) => s.status === "done"))
    return {
      label: t.boot.stReady,
      cls: "bg-ok-soft text-ok border-ok/20",
      icon: ShieldCheck,
    }
  return null
}

export function BootTimeline() {
  const steps = useBootStore((s) => s.steps)
  const state = consoleState(steps)
  const doneCount = steps.filter((s) => s.status === "done").length

  return (
    <section className="w-full max-w-xl rounded-2xl border border-line bg-panel/95 p-4 shadow-sm backdrop-blur-xs">
      <div className="mb-2 flex items-center justify-between border-b border-line/70 pb-3">
        <div className="flex items-center gap-2">
          <span className="flex size-6 items-center justify-center rounded-md bg-muted text-dim">
            <Terminal className="size-3.5" />
          </span>
          <span className="text-xs font-semibold tracking-wide text-ink">
            {t.boot.consoleTitle}
          </span>
          <span className="rounded-full bg-line-soft px-2 py-0.5 font-mono text-[10px] text-faint">
            {doneCount} / {steps.length}
          </span>
        </div>

        {state && (
          <div
            className={`inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 text-[11px] font-medium shadow-2xs ${state.cls}`}
          >
            <state.icon
              className={`size-3 ${state.label === t.boot.stRunning ? "animate-spin" : ""}`}
            />
            <span>{state.label}</span>
          </div>
        )}
      </div>

      <div className="flex flex-col gap-0.5">
        {t.boot.steps.map((def, i) => (
          <BootStep
            key={def.no}
            no={def.no}
            name={def.name}
            hint={def.hint}
            detail={steps[i]?.detail}
            status={steps[i]?.status ?? "pending"}
          />
        ))}
      </div>
    </section>
  )
}

