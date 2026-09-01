// 启动序列页（原 ui/index.html 升级重构，frontend-migration §4.1）。
// 组成：磨砂顶栏（wordmark + 版本芯片 + WSL 入口）→ Hero 空间质感叙事 →
// 下载主角位接管 → 「启动详情」控制台时间线卡（含内嵌错误区）。
import { useEffect, useState } from "react"
import { motion, AnimatePresence } from "framer-motion"
import { TerminalSquare, SlidersHorizontal } from "lucide-react"
import { useSearchParams } from "react-router-dom"
import { api } from "@/lib/tauri"
import { usePlatform } from "@/hooks/usePlatform"
import { resource } from "@/lib/resource"
import { useI18n } from "@/stores/i18nStore"
import type { BootErrorEvent } from "@/types/events"
import { useBootStore } from "@/stores/bootStore"
import { Emblem } from "@/components/layout/Emblem"
import { VersionChip } from "@/components/boot/VersionChip"
import { PulseBar } from "@/components/boot/PulseBar"
import { DownloadProgress } from "@/components/boot/DownloadProgress"
import { BootTimeline } from "@/components/boot/BootTimeline"
import { ErrorCard } from "@/components/boot/ErrorCard"

/// StrictMode 双挂载下去重同一份握手参数（choose_mode 会 teardown+重启会话）
let lastHandoff = ""

export function BootIndex() {
  const { t } = useI18n()
  const [params] = useSearchParams()
  const { can } = usePlatform()
  const [wslBusy, setWslBusy] = useState(false)
  const [localError, setLocalError] = useState<BootErrorEvent | null>(null)
  const [errorCount, setErrorCount] = useState(0)
  const [hideDownload, setHideDownload] = useState(false)
  const [maxStepSeen, setMaxStepSeen] = useState(-1)

  // —— store 订阅（细粒度选择器防高频重渲染） ——
  const steps = useBootStore((s) => s.steps)
  const activeStep = useBootStore((s) => s.activeStep)
  const error = useBootStore((s) => s.error)
  const progress = useBootStore((s) => s.progress)
  const setVersions = useBootStore((s) => s.setVersions)
  const clearError = useBootStore((s) => s.clearError)

  const shownError: BootErrorEvent | null = localError ?? error

  // 当前模式感知（URL params 经 choose_mode 落地时携带 mode=local|wsl）
  const currentMode = params.get("mode") || "local"
  const isWsl = currentMode === "wsl"

  // 「最后运行步」记忆：activeStep 在 done 后归 -1，headline 不能随之跌回初始
  const [lastRunning, setLastRunning] = useState(0)
  useEffect(() => {
    if (activeStep >= 0) setLastRunning(activeStep)
  }, [activeStep])

  // 播种版本快照（顶栏芯片立即可用；后续靠 boot:update 推送刷新）
  useEffect(() => {
    let alive = true
    resource.updateStatus().then((v) => {
      if (alive && v) setVersions(v)
    })
    return () => {
      alive = false
    }
  }, [setVersions])

  // 模式握手：带参到达 → 落地 choose_mode（每份参数只执行一次）
  useEffect(() => {
    const mode = params.get("mode")
    if (mode !== "local" && mode !== "wsl") return
    const key = `${mode}:${params.get("default") === "1"}`
    if (key === lastHandoff) return
    lastHandoff = key
    api.chooseMode(mode, params.get("default") === "1").catch(() => {})
  }, [params])

  // step>=2（spawn 起）下载条让位
  useEffect(() => {
    setMaxStepSeen((m) => Math.max(m, activeStep))
  }, [activeStep])
  useEffect(() => {
    if (maxStepSeen >= 2) setHideDownload(true)
  }, [maxStepSeen])
  useEffect(() => {
    if (progress) setHideDownload(false)
  }, [progress])
  // 下载完成后延迟隐藏下载卡片，让 headline/subline 展示后续步骤详情
  // 避免卡在 100% 进度条（后续解压/安装阶段无新 progress 事件）
  useEffect(() => {
    if (progress && progress.total != null && progress.total > 0 && progress.current >= progress.total) {
      const timer = setTimeout(() => setHideDownload(true), 1500)
      return () => clearTimeout(timer)
    }
  }, [progress])
  // 出错 → 全部进度让位 + WSL 按钮解锁（旧 renderError 语义）
  useEffect(() => {
    if (!error) return
    setHideDownload(true)
    setErrorCount((c) => c + 1)
    setWslBusy(false)
  }, [error])

  // —— 头部文案推演（旧 setStep 的 headline/subline 规则） ——
  const headline = (() => {
    if (shownError) return t.selector.problemHeadline
    if (!hideDownload && progress !== null && maxStepSeen < 2)
      return t.selector.preparingTitle
    return t.boot.headlines[Math.min(lastRunning, 4)]
  })()
  const subline = (() => {
    if (shownError?.detail) return undefined
    if (!hideDownload && progress !== null && maxStepSeen < 2)
      return t.selector.preparingSub
    return steps[Math.min(lastRunning, 4)]?.detail || undefined
  })()

  const showPulse = !shownError && !(!hideDownload && progress !== null)

  return (
    <div className="relative flex min-h-dvh flex-col bg-bg selection:bg-wash selection:text-brand-deep">
      {/* 顶部环境渐变光晕 */}
      <div className="pointer-events-none absolute inset-x-0 top-0 h-80 bg-[radial-gradient(ellipse_at_top,_rgba(65,118,230,0.08),_transparent_70%)]" />

      {/* 顶栏：准备阶段极简（方案 b），就绪后显示品牌与控制中心 */}
      <header className="absolute inset-x-0 top-0 z-20 flex items-center justify-between border-b border-line/60 bg-panel/70 px-6 py-3 backdrop-blur-md" data-tauri-drag-region>
        <div data-tauri-drag-region className="flex-1" />

        <div className="flex items-center gap-2.5">
          {maxStepSeen >= 4 && (
            <button
              type="button"
              title={t.boot.controlCenterTip}
              onClick={() => api.openProfilesWindow().catch(() => {})}
              className="inline-flex items-center gap-1.5 rounded-full border border-line bg-panel px-3 py-1 font-mono text-[11px] font-medium text-dim shadow-2xs transition-all hover:border-brand/40 hover:text-ink hover:shadow-xs"
            >
              <SlidersHorizontal className="size-3.5 text-brand" />
              {t.boot.controlCenter}
            </button>
          )}
          <VersionChip />
          {/* 模式切换：感知当前模式，双向切换（2026-09-01 修复硬编码） */}
          {can.bootWsl && (
            <button
              type="button"
              title={isWsl ? t.boot.localOpenTip : t.boot.wslOpenTip}
              disabled={wslBusy}
              onClick={() => {
                setWslBusy(true)
                const target = isWsl ? "local" : "wsl"
                api
                  .chooseMode(target, false)
                  .catch((e) =>
                    setLocalError({
                      title: isWsl ? t.boot.localFailed : t.boot.wslFailed,
                      detail: String(e instanceof Error ? e.message : e),
                    }),
                  )
                  .finally(() => setWslBusy(false))
              }}
              className="inline-flex items-center gap-1.5 rounded-full border border-line bg-panel px-3 py-1 font-mono text-[11px] font-medium text-dim shadow-2xs transition-all hover:border-brand/40 hover:text-ink hover:shadow-xs disabled:cursor-default disabled:opacity-50"
            >
              <TerminalSquare className="size-3.5 text-brand" />
              {isWsl ? t.boot.localOpen : t.boot.wslOpen}
            </button>
          )}
        </div>
      </header>

      {/* 主工作区 */}
      <main className="relative z-10 flex flex-1 flex-col items-center justify-center px-6 pt-24 pb-12">
        {/* Hero：一句话状态与生命感 */}
        <AnimatePresence mode="wait">
          {!shownError && (
            <motion.section
              key="boot-hero"
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -8 }}
              transition={{ duration: 0.25, ease: "easeOut" }}
              className="flex w-full max-w-xl flex-col items-center text-center"
            >
              <div className="relative mb-2">
                <div className="absolute -inset-2 rounded-2xl bg-brand/10 blur-xl" />
                <Emblem size={56} />
              </div>

              <h1 className="mt-3 text-2xl font-bold tracking-tight text-ink">
                {headline}
              </h1>

              {subline && (
                <p className="mx-auto mt-2 max-w-md text-sm leading-relaxed text-dim">
                  {subline}
                </p>
              )}

              {showPulse && (
                <div className="mt-6 w-full">
                  <PulseBar width={260} />
                </div>
              )}

              {!hideDownload && progress !== null && maxStepSeen < 2 && (
                <div className="w-full">
                  <DownloadProgress />
                </div>
              )}
            </motion.section>
          )}
        </AnimatePresence>

        {/* 启动详情：时间线 + 内嵌错误区 */}
        <section className="mt-7 w-full max-w-xl">
          <BootTimeline />
          {shownError && (
            <div className="mt-4">
              <ErrorCard
                payload={shownError}
                diag
                index={Math.max(errorCount, 1)}
                onReselect={
                  localError
                    ? () => {
                        setLocalError(null)
                        clearError()
                      }
                    : undefined
                }
              />
            </div>
          )}
        </section>
      </main>
    </div>
  )
}


