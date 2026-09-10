// 交接导轨（ADR-0014）：一次「重启 / 切换 profile」的四段式贯穿进度。
//
// 为什么是一条**导轨**而不是一个转圈：转圈只表达"在忙"，用户读不出"到哪了"。
// 四段（停止旧会话 → 启动新会话 → 等待就绪 → 进入工作台）正是这次操作的
// 真实阶段序，且两个窗口（控制中心 / 主窗口启动屏）渲染的是**同一条**视图模型
// （`lib/handoff.ts::deriveHandoff`），所以状态是接续的、计时是连续的。
//
// 视觉：导轨沿一条基线排列，走过的段实心品牌蓝，当前段呼吸，未到段灰。
// 失败时整条转警示态并把当前段停在原地——不假装还活着（诚实优先于好看）。
import { AlertTriangle, ArrowUpRight, Check, LoaderCircle, X } from "lucide-react"
import { useI18n } from "@/stores/i18nStore"
import { useElapsedMs } from "@/hooks/useElapsed"
import { HANDOFF_STAGES, formatElapsed, type HandoffView } from "@/lib/handoff"

/** 阶段文案键（与 HANDOFF_STAGES 顺序一一对应）。 */
function stageLabels(t: ReturnType<typeof useI18n>["t"]): string[] {
  return [t.handoff.stageStopping, t.handoff.stageBooting, t.handoff.stageWaiting, t.handoff.stageEntering]
}

/** 标题与副题（跨窗口同款：控制中心与启动屏读同一份 view）。 */
export function handoffHeadline(
  view: HandoffView,
  t: ReturnType<typeof useI18n>["t"],
): { title: string; subtitle: string } {
  const title =
    view.kind === "restart"
      ? t.handoff.titleRestart(view.target)
      : view.kind === "switch"
        ? t.handoff.titleSwitch(view.target)
        : t.handoff.titleStart(view.target)
  const subtitle = view.failed
    ? t.handoff.phaseFailed
    : view.phase === "ready"
      ? t.handoff.phaseReady
      : view.phase === "entering"
        ? t.handoff.phaseEntering
        : view.phase === "waiting"
          ? t.handoff.phaseWaiting
          : view.phase === "booting"
            ? t.handoff.phaseBooting
            : t.handoff.phaseStopping
  return { title, subtitle }
}

export function HandoffRail({
  view,
  onFocusWorkbench,
  onDismiss,
  className,
}: {
  view: HandoffView
  /** 右侧动作：把主窗口拉到前台（未就绪＝去看进度，就绪＝进工作台） */
  onFocusWorkbench?: () => void
  /** 仅失败态提供：收起这条失败导轨（在途/就绪态不可手动收起——那是真相） */
  onDismiss?: () => void
  className?: string
}) {
  const { t } = useI18n()
  const labels = stageLabels(t)
  const { title } = handoffHeadline(view, t)
  const elapsedMs = useElapsedMs(view.startedAt)
  const tone = view.failed
    ? "border-warn/30 bg-warn-soft"
    : view.done
      ? "border-ok/30 bg-ok-soft"
      : "border-brand/25 bg-wash"

  return (
    <section
      aria-label={title}
      aria-live="polite"
      className={`rounded-xl border px-3.5 py-2.5 backdrop-blur-sm transition-colors duration-300 ${tone} ${className ?? ""}`}
    >
      <div className="flex items-center gap-2.5">
        {/* 状态点：在途呼吸 / 就绪打勾 / 失败警示——一眼看出这条导轨的命运 */}
        <span className="relative flex size-4 shrink-0 items-center justify-center">
          {view.failed ? (
            <AlertTriangle className="text-warn size-3.5" aria-hidden />
          ) : view.done ? (
            <Check className="text-ok size-3.5" aria-hidden />
          ) : (
            <>
              <span className="bg-brand/25 absolute inline-flex size-3.5 animate-ping rounded-full motion-reduce:animate-none" />
              <span className="bg-brand relative inline-flex size-2 rounded-full" />
            </>
          )}
        </span>

        <span className="text-ink min-w-0 flex-1 truncate text-xs font-semibold tracking-tight">
          {title}
        </span>

        {/* 连续计时：起点来自 Rust 的 startedAt，跨文档替换不归零 */}
        <span
          className="text-dim shrink-0 font-mono text-label tabular-nums"
          title={t.handoff.elapsedTitle}
        >
          {formatElapsed(elapsedMs)}
        </span>

        {onFocusWorkbench && (
          <button
            type="button"
            onClick={onFocusWorkbench}
            className={`inline-flex shrink-0 items-center gap-1 rounded-lg border px-2 py-0.5 text-label font-medium transition-colors ${
              view.failed
                ? "border-warn/40 bg-panel text-warn hover:bg-warn-soft"
                : view.done
                  ? "border-ok/40 bg-panel text-ok hover:bg-ok-soft"
                  : "border-brand/30 bg-panel text-brand-deep hover:bg-wash"
            }`}
          >
            {view.failed
              ? t.handoff.viewFailure
              : view.done
                ? t.handoff.enterWorkbench
                : t.handoff.focusWorkbench}
            <ArrowUpRight className="size-3" aria-hidden />
          </button>
        )}

        {view.failed && onDismiss && (
          <button
            type="button"
            onClick={onDismiss}
            title={t.handoff.dismissFailure}
            aria-label={t.handoff.dismissFailure}
            className="text-faint hover:text-ink hover:bg-line-soft -mr-1 inline-flex size-6 shrink-0 items-center justify-center rounded-lg transition-colors"
          >
            <X className="size-3.5" aria-hidden />
          </button>
        )}
      </div>

      {/* 四段导轨：走过的段实心，当前段上方叠一个转圈表示"正在这一段里" */}
      <ol className="mt-2 flex items-start gap-0" role="list">
        {HANDOFF_STAGES.map((stage, i) => {
          const done = view.done || i < view.stageIndex
          const current = !view.done && !view.failed && i === view.stageIndex
          return (
            <li key={stage} className="relative flex min-w-0 flex-1 flex-col gap-1">
              <span className="flex items-center">
                {/* 左连接线（首段不画） */}
                <span
                  aria-hidden
                  className={`h-0.5 flex-1 rounded-full ${i === 0 ? "bg-transparent" : done || current ? "bg-brand/60" : "bg-line"}`}
                />
                <span className="relative flex size-3 shrink-0 items-center justify-center">
                  {current ? (
                    <LoaderCircle className="text-brand-deep size-3 animate-spin" aria-hidden />
                  ) : (
                    <span
                      className={`size-1.5 rounded-full ${done ? "bg-brand" : view.failed && i === view.stageIndex ? "bg-warn" : "bg-line"}`}
                    />
                  )}
                </span>
                {/* 右连接线（末段不画） */}
                <span
                  aria-hidden
                  className={`h-0.5 flex-1 rounded-full ${
                    i === HANDOFF_STAGES.length - 1 ? "bg-transparent" : i < view.stageIndex ? "bg-brand/60" : "bg-line"
                  }`}
                />
              </span>
              <span
                className={`truncate text-center text-micro leading-tight ${
                  current ? "text-brand-deep font-semibold" : done ? "text-dim" : "text-faint"
                }`}
              >
                {labels[i]}
              </span>
            </li>
          )
        })}
      </ol>
    </section>
  )
}
