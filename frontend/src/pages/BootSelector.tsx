// 工作台选择器页（原 ui/selector.html 升级重构，frontend-migration §4.3）。
// 两阶段叙事：一问一答（选卡片）→ 选定后原地切启动形态（与启动页同构）；
// 复用 DownloadProgress / ErrorCard / PulseBar。
import { useEffect, useState } from "react"
import { motion, AnimatePresence } from "framer-motion"
import { useSearchParams } from "react-router-dom"
import { Layout, Sparkles, ChevronRight, Package } from "lucide-react"
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
    <div className="relative flex min-h-dvh flex-col bg-bg selection:bg-wash selection:text-brand-deep">
      {/* 顶部环境渐变光晕 */}
      <div className="pointer-events-none absolute inset-x-0 top-0 h-80 bg-[radial-gradient(ellipse_at_top,_rgba(65,118,230,0.08),_transparent_70%)]" />

      {/* 顶栏：wordmark + 版本芯片 */}
      <header className="absolute inset-x-0 top-0 z-20 flex items-center justify-between border-b border-line/60 bg-panel/70 px-6 py-3 backdrop-blur-md">
        <div className="flex items-center gap-2">
          <div className="flex size-5 items-center justify-center rounded bg-brand/10 text-brand">
            <Sparkles className="size-3" />
          </div>
          <span className="font-mono text-[11px] font-bold tracking-[0.16em] text-ink/90">
            DSH DOCK
          </span>
        </div>
        <VersionChip />
      </header>

      <main className="relative z-10 flex flex-1 flex-col items-center justify-center px-6 pt-20 pb-12">
        {/* Hero */}
        <section className="flex w-full max-w-xl flex-col items-center text-center">
          <div className="relative mb-2">
            <div className="absolute -inset-2 rounded-2xl bg-brand/10 blur-xl" />
            <Emblem size={56} />
          </div>
          <h1 className="mt-3 text-2xl font-bold tracking-tight text-ink">{headline}</h1>
          <p className="mx-auto mt-2 max-w-md text-sm leading-relaxed text-dim">{subline}</p>
          {!shownError && showPulse && selected !== null && (
            <div className="mt-6 w-full">
              <PulseBar width={260} />
            </div>
          )}
        </section>

        {/* 阶段一：Profile 卡片选择阵列 */}
        <AnimatePresence mode="wait">
          {selected === null ? (
            <motion.div
              key="selector-cards"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.25, ease: "easeOut" }}
              className="mt-8 w-full max-w-xl"
            >
              <div className="grid gap-3">
                {profiles.map((name, i) => {
                  const meta = profileMeta(name)
                  const isDefault = meta.tag === "DEFAULT"
                  return (
                    <motion.button
                      key={name}
                      type="button"
                      initial={{ opacity: 0, y: 8 }}
                      animate={{ opacity: 1, y: 0 }}
                      transition={{ delay: i * 0.05, duration: 0.22 }}
                      whileHover={{ scale: 1.01, y: -1 }}
                      whileTap={{ scale: 0.99 }}
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
                      className="group relative flex items-center gap-4 rounded-2xl border border-line bg-panel/95 p-4 text-left shadow-xs transition-all hover:border-brand/50 hover:bg-wash/30 hover:shadow-md"
                    >
                      <div
                        className={`flex size-10 shrink-0 items-center justify-center rounded-xl transition-colors ${
                          isDefault
                            ? "bg-wash text-brand-deep group-hover:bg-brand group-hover:text-white"
                            : "bg-line-soft text-dim group-hover:bg-brand/10 group-hover:text-brand"
                        }`}
                      >
                        {isDefault ? <Layout className="size-5" /> : <Package className="size-5" />}
                      </div>

                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2">
                          <span className="text-sm font-semibold tracking-tight text-ink group-hover:text-brand-deep">
                            {meta.title}
                          </span>
                          <span className="rounded bg-line-soft px-1.5 py-0.5 font-mono text-[10px] text-faint">
                            {name}
                          </span>
                        </div>
                        <span className="mt-1 block truncate text-xs text-dim">{meta.desc}</span>
                      </div>

                      <div className="flex shrink-0 items-center gap-2">
                        <span
                          className={`rounded-full border px-2.5 py-0.5 text-[10px] font-semibold tracking-wide ${
                            isDefault
                              ? "border-brand/20 bg-brand/10 text-brand-deep"
                              : "border-line bg-line-soft/60 text-faint"
                          }`}
                        >
                          {meta.tag}
                        </span>
                        <ChevronRight className="size-4 text-faint transition-transform group-hover:translate-x-0.5 group-hover:text-brand" />
                      </div>
                    </motion.button>
                  )
                })}
              </div>

              <p className="mt-4 text-center text-xs text-faint">{t.selector.pickHint}</p>
            </motion.div>
          ) : !hideDownload && progress !== null ? (
            /* 阶段二：下载条接管 */
            <div className="w-full max-w-xl">
              <DownloadProgress />
            </div>
          ) : null}
        </AnimatePresence>

        {/* 错误卡 */}
        {shownError && (
          <div className="mt-6 w-full max-w-xl">
            <ErrorCard payload={shownError} onReselect={() => window.location.reload()} />
          </div>
        )}
      </main>
    </div>
  )
}

