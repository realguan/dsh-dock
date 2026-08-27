// 启动详情卡头（原 index.html .console 迁移）：五步时间线 + 卡头状态灯。
// 状态灯跟随全局：任一步 error=异常 > 任一 running=进行中 > 全 done=就绪。
import { useBootStore } from "@/stores/bootStore"
import { t } from "@/content/zh-CN"
import { BootStep } from "./BootStep"

function consoleState(steps: { status: string }[]): {
  label: string
  cls: string
} | null {
  if (steps.some((s) => s.status === "error"))
    return { label: t.boot.stError, cls: "bg-warn-soft text-warn" }
  if (steps.some((s) => s.status === "running"))
    return { label: t.boot.stRunning, cls: "bg-wash text-brand-deep" }
  if (steps.every((s) => s.status === "done"))
    return { label: t.boot.stReady, cls: "bg-ok-soft text-ok" }
  return null
}

export function BootTimeline() {
  const steps = useBootStore((s) => s.steps)
  const state = consoleState(steps)

  return (
    <section className="border-line bg-panel w-full max-w-xl rounded-xl border p-4 shadow-sm">
      <div className="border-line mb-1 flex items-center justify-between border-b pb-2.5">
        <span className="text-dim text-xs font-semibold tracking-wide">
          {t.boot.consoleTitle}
        </span>
        {state && (
          <span
            className={`rounded-full px-2 py-0.5 text-[10px] font-medium ${state.cls}`}
          >
            {state.label}
          </span>
        )}
      </div>
      <div className="divide-line/60 divide-y">
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
