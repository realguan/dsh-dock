// 启动序列页（原 ui/index.html 完整迁移，frontend-migration §4.1，最复杂一页）。
//
// 组成：顶栏（wordmark + 版本芯片 + WSL 入口）→ Hero 一句话叙事（pulse 让位规则
// 同旧 syncBars）→ 下载主角位接管 → 「启动详情」时间线卡（含内嵌错误区）。
//
// 模式握手（保真竞态规避裁定）：?mode=&default= 由 BootMode 携参跳转而来，
// 本页挂载后 invoke choose_mode——事件总线已在模块加载期注册完毕，
// 启动线程随后的 boot:step 遥测必然被消费（旧 ui/index.html 同款时序）。
import { useEffect, useState } from "react"
import { TerminalSquare } from "lucide-react"
import { useSearchParams } from "react-router-dom"
import { api } from "@/lib/tauri"
import { usePlatform } from "@/hooks/usePlatform"
import { resource } from "@/lib/resource"
import { t } from "@/content/zh-CN"
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
    <div className="bg-bg relative min-h-dvh">
      {/* 顶栏 */}
      <header className="border-line/70 absolute inset-x-0 top-0 z-10 flex items-center justify-between px-5 py-3">
        <span className="text-faint text-[11px] font-semibold tracking-[0.18em]">
          DSH DOCK
        </span>
        <div className="flex items-center gap-2">
          <VersionChip />
          {/* WSL 仅 Windows 渲染（2026-08-26 平台裁定，能力经 can.bootWsl） */}
          <button
            type="button"
            title={t.boot.wslOpenTip}
            disabled={!can.bootWsl || wslBusy}
            onClick={() => {
              setWslBusy(true)
              api
                .bootInWsl()
                .catch((e) =>
                  setLocalError({
                    title: t.boot.wslFailed,
                    detail: String(e instanceof Error ? e.message : e),
                  }),
                )
                .finally(() => setWslBusy(false))
            }}
            className="border-line text-dim hover:border-line hover:text-ink inline-flex items-center gap-1.5 rounded-full border bg-white px-2.5 py-1 text-[11px] transition-colors disabled:cursor-default disabled:opacity-50"
          >
            <TerminalSquare className="size-3.5" />
            {t.boot.wslOpen}
          </button>
        </div>
      </header>

      <main className="flex min-h-dvh flex-col items-center px-6 pt-20 pb-10">
        {/* Hero：一句话状态。出错时整块让位给错误卡（旧 hero.style.display='none'） */}
        {!shownError && (
          <section className="page-rise flex w-full max-w-xl flex-col items-center text-center">
            <Emblem size={52} />
            <h1 className="text-ink mt-4 text-2xl font-semibold tracking-tight">
              {headline}
            </h1>
            {subline && (
              <p className="text-dim mx-auto mt-2.5 max-w-md text-sm leading-relaxed">
                {subline}
              </p>
            )}
            {showPulse && (
              <div className="mt-6">
                <PulseBar width={280} />
              </div>
            )}
            {!hideDownload && progress !== null && maxStepSeen < 2 && (
              <div className="w-full">
                <DownloadProgress />
              </div>
            )}
          </section>
        )}

        {/* 启动详情：时间线 + 内嵌错误区 */}
        <section className="page-rise mt-6 w-full max-w-xl [animation-delay:80ms]">
          <BootTimeline />
          {shownError && (
            <div className="mt-3">
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

