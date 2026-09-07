// 启动序列页（原 ui/index.html 升级重构，frontend-migration §4.1）。
// 2026-09-07 单主角重构：hero 区块撤编——原 hero 与时间线重复讲述同一状态
// （标题/副题逐字复现步骤行）。现在控制台卡是唯一主角：卡头讲述「现在怎样」
// （徽标 + 当前状态 + 分段进度），步骤列表讲述「到哪了」；下载进度经 banner
// 槽位入卡；出错时卡头转警示态、ErrorCard 就地展开。
import { useEffect, useState } from "react"
import { TerminalSquare, SlidersHorizontal } from "lucide-react"
import { useSearchParams } from "react-router-dom"
import { api } from "@/lib/tauri"
import { usePlatform } from "@/hooks/usePlatform"
import { resource } from "@/lib/resource"
import { useI18n } from "@/stores/i18nStore"
import type { BootErrorEvent } from "@/types/events"
import { normalizeError, normalizeStep } from "@/lib/events"
import { useBootStore } from "@/stores/bootStore"
import { VersionChip } from "@/components/boot/VersionChip"
import { DownloadProgress } from "@/components/boot/DownloadProgress"
import { BootTimeline } from "@/components/boot/BootTimeline"
import { ErrorCard } from "@/components/boot/ErrorCard"
import { UpdateBanner } from "@/components/update/UpdateBanner"

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

  // 「最后运行步」记忆：activeStep 在 done 后归 -1，卡头不能随之跌回初始
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

  // 播种早期启动状态与错误（规避 WebView 挂载前事件丢失的竞态）
  useEffect(() => {
    let alive = true
    api
      .getBootStatus()
      .then((status) => {
        if (!alive || !status) return
        if (Array.isArray(status.steps)) {
          for (const s of status.steps) {
            const step = normalizeStep(s)
            if (step) useBootStore.getState().setStep(step)
          }
        }
        if (status.error) {
          const err = normalizeError(status.error)
          if (err) useBootStore.getState().setError(err)
        }
      })
      .catch(() => {})
    return () => {
      alive = false
    }
  }, [])

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
  // 下载完成后延迟隐藏下载卡片，让卡头展示后续步骤详情
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

  // —— 卡头文案推演：错误 > 下载准备期 > 当前步骤名（detail 兜底回 hint） ——
  const idx = Math.min(lastRunning, 4)
  const inDownload = !hideDownload && progress !== null && maxStepSeen < 2
  const title = shownError
    ? t.selector.problemHeadline
    : inDownload
      ? t.selector.preparingTitle
      : t.boot.steps[idx].name
  const subtitle = shownError
    ? undefined
    : inDownload
      ? t.selector.preparingSub
      : steps[idx]?.detail || t.boot.steps[idx].hint

  return (
    <div className="relative flex min-h-dvh flex-col bg-bg selection:bg-wash selection:text-brand-deep">
      {/* 顶部环境渐变光晕 */}
      <div className="pointer-events-none absolute inset-x-0 top-0 h-96 bg-[radial-gradient(ellipse_80%_60%_at_50%_-20%,color-mix(in_srgb,var(--color-brand)_12%,transparent),transparent_70%)]" />

      {/* 顶栏：轻量工作台徽标 + 版本芯片 + WSL 切换 + 控制中心入口 */}
      <header className="absolute inset-x-0 top-0 z-20 flex items-center justify-between border-b border-line/60 bg-panel/75 px-6 py-3 backdrop-blur-md" data-tauri-drag-region>
        <div data-tauri-drag-region className="flex flex-1 items-center gap-2 select-none">
          <span className="flex size-2 rounded-full bg-brand ring-4 ring-brand/10" />
          <span className="font-mono text-xs font-semibold tracking-wider text-ink/90">DSH DOCK</span>
          <span className="rounded bg-line-soft px-1.5 py-0.5 font-mono text-[10px] text-faint">DESKTOP</span>
        </div>

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

      {/* 升级提示条：非阻断浮层（ADR-0010 升级呈现；忽略同版本不再弹） */}
      <div className="pointer-events-none absolute inset-x-0 top-14 z-10 flex justify-center px-6">
        <UpdateBanner />
      </div>

      {/* 主工作区：控制台卡即页面主角 */}
      <main className="relative z-10 flex flex-1 flex-col items-center justify-center px-6 pt-24 pb-12">
        <section className="w-full max-w-xl">
          <BootTimeline
            title={title}
            subtitle={subtitle}
            danger={!!shownError}
            banner={inDownload ? <DownloadProgress /> : undefined}
          />
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
