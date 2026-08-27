// 工作台选择器页（原 ui/selector.html 完整迁移，frontend-migration §4.3）。
// 两阶段叙事：一问一答（选卡片）→ 选定后原地切启动形态（与启动页同构）；
// 复用 DownloadProgress / ErrorCard / PulseBar——整页导航后 store 重建，
// 事件流持续驱动同一套组件（组件级复用而非运行时共享）。
//
// 时序保真（旧 syncBars 规则）：选定后 PulseBar 常驻生命感；仅「下载条接管」
// 与「出错」两个时点让位。step>=2（spawn 起）隐藏下载条。
import { useEffect, useState } from "react"
import { useSearchParams } from "react-router-dom"
import { api } from "@/lib/tauri"
import { t } from "@/content/zh-CN"
import type { BootErrorEvent } from "@/types/events"
import { useBootStore } from "@/stores/bootStore"
import { Emblem } from "@/components/layout/Emblem"
import { VersionChip } from "@/components/boot/VersionChip"
import { PulseBar } from "@/components/boot/PulseBar"
import { DownloadProgress } from "@/components/boot/DownloadProgress"
import { ErrorCard } from "@/components/boot/ErrorCard"

function profileMeta(name: string) {
  return (
    t.selector.items[name] ?? {
      title: name,
      desc: t.selector.customDesc,
      tag: t.selector.customTag,
    }
  )
}

export function BootSelector() {
  const [params] = useSearchParams()
  const profiles = (params.get("profiles") || "web")
    .split(",")
    .map((s) => s.trim())
    .filter(Boolean)

  // —— 本地叙事状态（store 管运行时事件，这里管页面阶段） ——
  const [selected, setSelected] = useState<{ name: string; title: string } | null>(null)
  const [localError, setLocalError] = useState<BootErrorEvent | null>(null)
  const [hideDownload, setHideDownload] = useState(false)
  const [maxStepSeen, setMaxStepSeen] = useState(-1)

  // —— store 选择订阅 ——
  const activeStep = useBootStore((s) => s.activeStep)
  const steps = useBootStore((s) => s.steps)
  const error = useBootStore((s) => s.error)
  const progress = useBootStore((s) => s.progress)

  const shownError: BootErrorEvent | null = localError ?? error

  // step>=2（spawn DSH 及之后）→ 隐藏下载条
  useEffect(() => {
    setMaxStepSeen((m) => Math.max(m, activeStep))
  }, [activeStep])
  useEffect(() => {
    if (maxStepSeen >= 2) setHideDownload(true)
  }, [maxStepSeen])
  // 新的进度帧到达 → 下载条重新接管（新一轮下载）
  useEffect(() => {
    if (progress) setHideDownload(false)
  }, [progress])
  // 出错 → 下载条让位
  useEffect(() => {
    if (shownError) setHideDownload(true)
  }, [shownError])

  const showPulse = selected !== null && !shownError && !(!hideDownload && progress !== null)

  const headline =
    shownError !== null
      ? t.selector.problemHeadline
      : selected === null
        ? t.selector.headline
        : maxStepSeen >= 1 && activeStep >= 2
          ? t.boot.headlines[Math.min(Math.max(activeStep, 2), 4)]
          : `${t.selector.launchingPrefix}${selected.title}${t.selector.launchingSuffix}`

  const subline = (() => {
    if (selected === null) return t.selector.subline
    if (!hideDownload && progress !== null && maxStepSeen < 2)
      return t.selector.preparingSub
    const detail = steps[Math.max(activeStep, 0)]?.detail
    return detail ?? selected.name
  })()

  return (
    <div className="bg-bg relative min-h-dvh">
      {/* 顶栏：wordmark + 版本芯片 */}
      <header className="border-line/70 absolute inset-x-0 top-0 z-10 flex items-center justify-between px-5 py-3">
        <span className="text-faint text-[11px] font-semibold tracking-[0.18em]">
          DSH DOCK
        </span>
        <VersionChip />
      </header>

      <main className="flex min-h-dvh flex-col items-center px-6 pt-16 pb-10">
        {/* Hero */}
        <section className="page-rise w-full max-w-xl text-center" id="hero">
          <div className="mb-4 flex justify-center">
            <Emblem size={52} />
          </div>
          <h1 className="text-ink text-2xl font-semibold tracking-tight">{headline}</h1>
          <p className="text-dim mx-auto mt-2.5 max-w-md text-sm leading-relaxed">{subline}</p>
          {!shownError && showPulse && selected !== null && (
            <div className="mt-6">
              <PulseBar width={280} />
            </div>
          )}
        </section>

        {/* 阶段一：问题清单 */}
        {selected === null ? (
          <>
            <div className="mt-8 grid w-full max-w-xl gap-2.5">
              {profiles.map((name, i) => {
                const meta = profileMeta(name)
                return (
                  <button
                    key={name}
                    type="button"
                    style={{ animationDelay: `${i * 70}ms` }}
                    onClick={() => {
                      setSelected({ name, title: meta.title })
                      api
                        .chooseProfile(name)
                        .catch((e) =>
                          setLocalError({
                            title: t.error.fallbackTitle,
                            detail: `${String(e instanceof Error ? e.message : e)}（可返回重选）`,
                            actions: ["retry"],
                          }),
                        )
                    }}
                    className="page-rise border-line bg-panel group hover:border-brand/50 hover:shadow-md flex items-center gap-4 rounded-xl border p-4 text-left transition-all"
                  >
                    <span className="font-mono text-faint text-xs tabular-nums">
                      {String(i + 1).padStart(2, "0")}
                    </span>
                    <span className="min-w-0 flex-1">
                      <span className="text-ink block text-[15px] font-semibold">{meta.title}</span>
                      <span className="text-dim mt-0.5 block truncate text-xs">{meta.desc}</span>
                    </span>
                    <span
                      className={`shrink-0 rounded-full border px-2 py-0.5 text-[10px] font-medium tracking-wide ${
                        meta.tag === "DEFAULT"
                          ? "bg-wash text-brand-deep border-transparent"
                          : "text-faint border-line"
                      }`}
                    >
                      {meta.tag}
                    </span>
                  </button>
                )
              })}
            </div>
            <p className="text-faint mt-4 text-center text-xs">{t.selector.pickHint}</p>
          </>
        ) : !hideDownload && progress !== null ? (
          /* 阶段二：下载条接管 */
          <DownloadProgress />
        ) : null}

        {/* 错误卡：后端错误 + 本地动作失败统一渲染；reselect = 整页重载 */}
        {shownError && (
          <ErrorCard payload={shownError} onReselect={() => window.location.reload()} />
        )}
      </main>
    </div>
  )
}
