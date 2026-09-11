// 启动序列页（原 ui/index.html 升级重构，frontend-migration §4.1）。
// 2026-09-07 单主角重构：hero 区块撤编——原 hero 与时间线重复讲述同一状态
// （标题/副题逐字复现步骤行）。现在控制台卡是唯一主角：卡头讲述「现在怎样」
// （徽标 + 当前状态 + 分段进度），步骤列表讲述「到哪了」；下载进度经 banner
// 槽位入卡；出错时卡头转警示态、ErrorCard 就地展开。
import { useEffect, useMemo, useRef, useState } from "react"
import { TerminalSquare } from "lucide-react"
import { useSearchParams } from "react-router-dom"
import { api } from "@/lib/tauri"
import { usePlatform } from "@/hooks/usePlatform"
import { useI18n } from "@/stores/i18nStore"
import type { BootErrorEvent } from "@/types/events"
import { normalizeError, normalizeStep } from "@/lib/events"
import { useBootStore } from "@/stores/bootStore"
import { deriveHandoff } from "@/lib/handoff"
import {
  shouldOfferTimelineCollapse,
  shouldShowTimeline,
  type TimelineIntent,
} from "@/lib/bootTimeline"
import { shouldClearStaleError, type Step0Status } from "@/lib/bootRound"
import { handoffHeadline } from "@/components/boot/HandoffRail"
import { DownloadProgress } from "@/components/boot/DownloadProgress"
import { BootTimeline } from "@/components/boot/BootTimeline"
import { ErrorCard } from "@/components/boot/ErrorCard"
import { UpdateBanner } from "@/components/update/UpdateBanner"
import { Emblem } from "@/components/layout/Emblem"
import { PulseBar } from "@/components/boot/PulseBar"
import { ElapsedChip } from "@/components/boot/ElapsedChip"

/// StrictMode 双挂载下去重同一份握手参数（choose_mode 会 teardown+重启会话）。
/// 名字不与 ADR-0014 的「交接意图（handoff）」混用：这里指的是 URL 参数握手。
let lastModeHandoff = ""

/// 注入幕布交接（ADR-0014）：主窗口 document-start 由 injected/handoff-curtain.js
/// 先画同款首帧（旧工作台→壳页面→新工作台 三段都不断），React 一挂载就接管。
/// 幕布缺席（浏览器预览/旧文档）时是空操作。
declare global {
  interface Window {
    __dshDockCurtain?: { show?: (payload: unknown) => void; hide?: () => void }
  }
}

export function BootIndex() {
  const { t } = useI18n()
  const [params] = useSearchParams()
  const { can } = usePlatform()
  const [wslBusy, setWslBusy] = useState(false)
  const [localError, setLocalError] = useState<BootErrorEvent | null>(null)
  const [errorCount, setErrorCount] = useState(0)
  const [hideDownload, setHideDownload] = useState(false)
  const [maxStepSeen, setMaxStepSeen] = useState(-1)
  const [hasEverDownloaded, setHasEverDownloaded] = useState(false)
  // 用户对启动详情的显式意图（v1.2.0 实测 1.2）：三态而非布尔——布尔表达不了
  // 「用户主动收起」，收起后自动信号仍为真 → 收起点不动任何东西（死按钮）。
  const [timelineIntent, setTimelineIntent] = useState<TimelineIntent>("auto")

  // —— store 订阅（细粒度选择器防高频重渲染） ——
  const steps = useBootStore((s) => s.steps)
  const activeStep = useBootStore((s) => s.activeStep)
  const error = useBootStore((s) => s.error)
  const progress = useBootStore((s) => s.progress)
  const intent = useBootStore((s) => s.intent)
  const clearError = useBootStore((s) => s.clearError)
  const beginNewRound = useBootStore((s) => s.beginNewRound)

  const shownError: BootErrorEvent | null = localError ?? error

  // 交接视图（ADR-0014）：本页与「控制中心」读同一份纯模型 —— 同一条导轨、
  // 同一个计时起点（startedAt 来自 Rust，跨本页的两次整文档替换都不归零）。
  const handoff = useMemo(() => deriveHandoff(intent, steps), [intent, steps])
  const handoffText = useMemo(
    () => (handoff ? handoffHeadline(handoff, t) : null),
    [handoff, t],
  )

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
    // 幕布交接：React 首帧已就位，撤掉 document-start 注入的幕布（同款构图，
    // 用户看到的是"幕布 → 启动屏"无缝接管，而不是先闪一下再加载）。
    window.__dshDockCurtain?.hide?.()
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
        // 交接意图补水（整文档重载后唯一来源）：控制中心发起 → 主窗口新文档
        // 首帧即从壳读到「谁在重启、从何时开始」，于是导轨与计时接着走。
        useBootStore.getState().setIntent(status.intent ?? null)
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
    if (key === lastModeHandoff) return
    lastModeHandoff = key
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
    if (progress) {
      setHideDownload(false)
      setHasEverDownloaded(true)
    }
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

  // 新一轮启动起跑（步 0 重新 running）⇒ 上一轮的错误已描述过一个不存在的世界，
  // 撤销它（v1.2.0 实测 2.1）。判据是**边沿**而非稳态，见 lib/bootRound.ts。
  // 与模式切换处的显式调用共用同一个 store 动作（唯一清零点，不散落）。
  const prevStep0 = useRef<Step0Status>(undefined)
  const step0Status = steps[0]?.status
  useEffect(() => {
    const prev = prevStep0.current
    prevStep0.current = step0Status
    if (shouldClearStaleError(prev, step0Status, shownError !== null)) {
      setLocalError(null)
      beginNewRound()
    }
  }, [step0Status, shownError, beginNewRound])

  // —— 卡头文案推演：错误 > 交接 > 下载准备期 > 当前步骤名（detail 兜底回 hint） ——
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
      : handoffText && !handoff?.failed
        ? handoffText.subtitle
        : steps[idx]?.detail || t.boot.steps[idx].hint

  // 区分「首次安装准备向导」与「日常秒启 Splash」：
  // 只有在发生环境下载、出现错误或手动请求时才展示 5 步向导列表；
  // 普通秒级直启展示高保真极简启动屏，彻底消除每次打开装机自检的割裂感。
  // 2026-09-10 修订（ADR-0014 §7）：交接**不再**强制展开向导态——重启必须与
  // 开机长得一模一样（复用，不是新页面）；连续性由文案 + 连续计时 + 幕布承担。
  // 想看步骤的用户仍有「查看启动详情」这个既有出口。
  // 判据抽到 lib/bootTimeline.ts（纯函数 + 测试）：这两处曾用不同信号，
  // 导致「点开详情后回不去」（2026-09-10 修复；2026-09-11 v1.2.0 实测 1.2 续修：
  // hasError 不再否决收起——ErrorCard 渲染在本区块之外，收起不会藏掉错误）。
  const timelineVisibility = {
    hasError: shownError !== null,
    hasEverDownloaded,
    inDownload,
    userIntent: timelineIntent,
  }
  const isSetupMode = shouldShowTimeline(timelineVisibility)
  const canCollapseTimeline = shouldOfferTimelineCollapse(timelineVisibility)

  return (
    <div className="relative flex min-h-dvh flex-col bg-bg selection:bg-wash selection:text-brand-deep">
      {/* 顶部环境渐变光晕 */}
      <div className="pointer-events-none absolute inset-x-0 top-0 h-96 bg-[radial-gradient(ellipse_80%_60%_at_50%_-20%,color-mix(in_srgb,var(--color-brand)_12%,transparent),transparent_70%)]" />

      {/* WSL 模式切换（仅 Windows 渲染）：悬浮胶囊 */}
      {can.bootWsl && (
        <div className="absolute right-6 top-4 z-20">
          <button
            type="button"
            title={isWsl ? t.boot.localOpenTip : t.boot.wslOpenTip}
            disabled={wslBusy}
            onClick={() => {
              setWslBusy(true)
              const target = isWsl ? "local" : "wsl"
              // 切换模式 = 一次全新启动（ADR-0014），且 `choose_mode` 是**原地**
              // 调用（不 navigate）⇒ store 存活，必须显式开新一轮清掉上一轮的
              // 错误/进度，否则旧诊断卡会挂在新会话上（v1.2.0 实测 2.1）。
              // 失败时下面会写入新的 localError，故不存在"错误被误藏"。
              setLocalError(null)
              beginNewRound()
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
            className="inline-flex items-center gap-1.5 rounded-full border border-line bg-panel px-3 py-1 font-mono text-label font-medium text-dim shadow-2xs backdrop-blur-md transition-all hover:border-brand/40 hover:text-ink hover:shadow-xs disabled:cursor-default disabled:opacity-50"
          >
            <TerminalSquare className="size-3.5 text-brand-deep" />
            {isWsl ? t.boot.localOpen : t.boot.wslOpen}
          </button>
        </div>
      )}

      {/* 升级提示条：非阻断浮层（ADR-0010 升级呈现；忽略同版本不再弹） */}
      <div className="pointer-events-none absolute inset-x-0 top-4 z-10 flex justify-center px-6">
        <UpdateBanner />
      </div>

      {/* 主工作区 */}
      <main className="relative z-10 flex flex-1 flex-col items-center justify-center px-6 pt-16 pb-12">
        <section className="w-full max-w-xl">
          {isSetupMode ? (
            <>
              <BootTimeline
                title={title}
                subtitle={subtitle}
                danger={!!shownError}
                banner={inDownload ? <DownloadProgress /> : undefined}
                // 连续计时（ADR-0014）：交接期间本页唯一的"时间锚"——起点来自
                // Rust 的 startedAt，跨文档不归零，用户读得出"这次已经等了多久"。
                meta={handoff ? <ElapsedChip startedAt={handoff.startedAt} /> : undefined}
              />
              {canCollapseTimeline && (
                <div className="mt-3 text-center">
                  <button
                    type="button"
                    onClick={() => setTimelineIntent("collapsed")}
                    className="text-meta text-dim transition-colors hover:text-ink"
                  >
                    {t.boot.hideTimeline}
                  </button>
                </div>
              )}
            </>
          ) : (
            <div className="flex flex-col items-center text-center">
              <div className="relative mb-6">
                <div className="absolute -inset-4 animate-pulse rounded-2xl bg-brand/20 blur-xl motion-reduce:animate-none" />
                <Emblem size={64} />
              </div>
              {/* 交接复用同一屏：只换文案（重启谁）与副题（到哪一步了），
                  构图/字号/徽标与日常秒启逐像素一致——重启不该长得像另一个页面。 */}
              <h1 className="text-xl font-bold tracking-tight text-ink sm:text-2xl">
                {handoffText ? handoffText.title : t.boot.launchingTitle}
              </h1>
              <p className="mt-2 max-w-md text-xs text-dim sm:text-sm">
                {handoffText && !handoff?.failed
                  ? handoffText.subtitle
                  : steps[idx]?.detail || t.boot.launchingSub}
              </p>
              <div className="mt-6">
                <PulseBar width={220} />
              </div>
              {handoff && (
                <div className="mt-3">
                  <ElapsedChip startedAt={handoff.startedAt} />
                </div>
              )}
              <button
                type="button"
                onClick={() => setTimelineIntent("expanded")}
                className="mt-8 text-meta text-faint transition-colors hover:text-dim"
              >
                {t.boot.viewTimeline}
              </button>
            </div>
          )}

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
