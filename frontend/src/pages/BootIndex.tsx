// 启动序列页（原 ui/index.html 升级重构，frontend-migration §4.1）。
// 2026-09-07 单主角重构：hero 区块撤编——原 hero 与时间线重复讲述同一状态
// （标题/副题逐字复现步骤行）。现在控制台卡是唯一主角：卡头讲述「现在怎样」
// （徽标 + 当前状态 + 分段进度），步骤列表讲述「到哪了」；下载进度经 banner
// 槽位入卡；出错时卡头转警示态、ErrorCard 就地展开。
import { useEffect, useState } from "react"
import { TerminalSquare } from "lucide-react"
import { useSearchParams } from "react-router-dom"
import { api } from "@/lib/tauri"
import { usePlatform } from "@/hooks/usePlatform"
import { useI18n } from "@/stores/i18nStore"
import type { BootErrorEvent } from "@/types/events"
import { normalizeError, normalizeStep } from "@/lib/events"
import { useBootStore } from "@/stores/bootStore"
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

      {/* WSL 模式切换（仅 Windows 渲染）：悬浮胶囊。boot 页不再渲染通栏顶栏
          ——原生标题栏之下再来一条导航条视觉上叠加成「双下巴」，且徽标/版本
          芯片/控制中心入口在 boot 期均无消费场景（错误卡兜底诊断入口）。 */}
      {can.bootWsl && (
        <div className="absolute right-6 top-4 z-20">
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
            className="inline-flex items-center gap-1.5 rounded-full border border-line bg-panel px-3 py-1 font-mono text-[11px] font-medium text-dim shadow-2xs backdrop-blur-md transition-all hover:border-brand/40 hover:text-ink hover:shadow-xs disabled:cursor-default disabled:opacity-50"
          >
            <TerminalSquare className="size-3.5 text-brand" />
            {isWsl ? t.boot.localOpen : t.boot.wslOpen}
          </button>
        </div>
      )}

      {/* 升级提示条：非阻断浮层（ADR-0010 升级呈现；忽略同版本不再弹） */}
      <div className="pointer-events-none absolute inset-x-0 top-4 z-10 flex justify-center px-6">
        <UpdateBanner />
      </div>

      {/* 主工作区：控制台卡即页面主角 */}
      <main className="relative z-10 flex flex-1 flex-col items-center justify-center px-6 pt-16 pb-12">
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
