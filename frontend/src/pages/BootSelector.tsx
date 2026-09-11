// 工作台启动中心（Workbench Launchpad，2026-09-09 产品体验重塑）。
// 职责：当应用已就绪但未指定默认工作台时，以沉浸式 Launchpad 呈现所有工作空间；
// 支持卡片点选、键盘快捷直达（1~9）、一键设为默认工作台、直达控制中心。
import { useEffect, useState } from "react"
import { motion, AnimatePresence } from "framer-motion"
import { useSearchParams } from "react-router-dom"
import {
  Layout,
  Package,
  Sparkles,
  Plus,
  SlidersHorizontal,
  CheckCircle2,
  ArrowUpRight,
  ShieldCheck,
  Loader2,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { useI18n } from "@/stores/i18nStore"
import type { BootErrorEvent } from "@/types/events"
import type { ProfileSummary } from "@/types/ipc"
import { useBootStore } from "@/stores/bootStore"
import { Emblem } from "@/components/layout/Emblem"
import { PulseBar } from "@/components/boot/PulseBar"
import { DownloadProgress } from "@/components/boot/DownloadProgress"
import { ErrorCard } from "@/components/boot/ErrorCard"
import { logger } from "@/lib/logger"

/**
 * 工厂默认工作台名——**判定用稳定标识，不是文案**。
 * AGENTS §6（2026-09-11 措辞校正后）：`defaultProfile` 为 **None 时由消费方兜底
 * `web`**；**失效值不消费**（走常规流程＝出选择器，不预选任何 profile）。
 * 本常量只承担前半句的兜底语义。
 *
 * 两条口径与 `resolveIsDefault` 的对应关系：
 *   · `defaultProfile === null`  → 命中 `web`（兜底：web 会被启动）；
 *   · `defaultProfile` 是失效值  → 不命中任何候选（不消费：没有"将被启动的默认"）。
 */
export const FACTORY_DEFAULT_PROFILE = "web"

/**
 * 「该卡片是否标为默认工作台」**纯函数**（2026-09-11，task-23）。
 *
 * 判据只允许来自两个**与文案无关**的稳定来源：settings 里的 `defaultProfile`
 * （真实数据）与 `FACTORY_DEFAULT_PROFILE`（模块常量）。
 *
 * 修复前这里是 `name === defaultProfile || meta.tag === "DEFAULT"`——`meta.tag`
 * 是**字典值**（`t.selector.items.web.tag`）。后果：任何人把该值"翻译"成
 * `"默认"`，`isDefault` 判定会**静默失效**——改文案改掉控制流。本函数把判定与
 * 文案彻底隔开，防复发断言见 `__tests__/bootSelectorDefault.test.ts`。
 */
export function resolveIsDefault(name: string, defaultProfile: string | null): boolean {
  return name === (defaultProfile ?? FACTORY_DEFAULT_PROFILE)
}

export function BootSelector() {
  const { t } = useI18n()
  const [params] = useSearchParams()

  const rawParams = params.get("profiles")
  const urlCandidateNames = rawParams
    ? rawParams
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean)
    : []

  const [profileSummaries, setProfileSummaries] = useState<ProfileSummary[]>([])
  const [defaultProfile, setDefaultProfile] = useState<string | null>(null)
  const [rememberChoice, setRememberChoice] = useState(false)
  const [launchingName, setLaunchingName] = useState<string | null>(null)
  const [localError, setLocalError] = useState<BootErrorEvent | null>(null)
  const [hideDownload, setHideDownload] = useState(false)
  const [maxStepSeen, setMaxStepSeen] = useState(-1)

  // —— store 选择订阅 ——
  const activeStep = useBootStore((s) => s.activeStep)
  const steps = useBootStore((s) => s.steps)
  const error = useBootStore((s) => s.error)
  const progress = useBootStore((s) => s.progress)

  const shownError: BootErrorEvent | null = localError ?? error

  // 挂载时并发拉取 Profile 详情与当前默认工作台
  useEffect(() => {
    let alive = true
    Promise.all([
      api.listProfiles().catch(() => [] as ProfileSummary[]),
      api.getDefaultProfile().catch(() => null),
    ]).then(([list, def]) => {
      if (!alive) return
      setProfileSummaries(list)
      setDefaultProfile(def)
    })
    return () => {
      alive = false
    }
  }, [])

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

  // 计算展示的工作台列表（富元数据加持）
  const displayProfiles = (() => {
    const summaryMap = new Map(profileSummaries.map((p) => [p.name, p]))
    const candidateNames =
      urlCandidateNames.length > 0
        ? urlCandidateNames
        : profileSummaries.length > 0
          ? profileSummaries.map((p) => p.name)
          : ["web"]

    return candidateNames.map((name) => {
      const summary = summaryMap.get(name)
      // 未知名（无字典条目的自定义工作台）回退：标题即 profile 名——语言中立。
      // 2026-09-11（task-25）：原为 `title: name === "web" ? "默认工作台" : name`。
      //   ① 该 web 分支**不可达**：`items.web` 恒存在，`??` 回退对 "web" 不触发；
      //   ② 更糟的是**潜在陷阱**：若将来删掉 `items.web`，回退分支就会向 en 用户
      //      吐中文（en 侧漏译闸门只扫 en 字典，抓不到组件内字面量）。
      // 等价性：对任何**真正走到 `??` 回退**的 name，必有 `name !== "web"`，
      //   故旧代码取值恒为 `name`，与新写法逐字一致；差别仅在「items.web 被删」
      //   这一假想未来——那时新写法展示 profile 名（诚实、可本地化），而非中文。
      // 内置条目（含 web）的标题一律由字典 `t.selector.items[*].title` 提供。
      const meta = t.selector.items[name] ?? {
        title: name,
        desc: t.selector.customDesc,
        tag: t.selector.customTag,
      }
      const isDefault = resolveIsDefault(name, defaultProfile)
      const pluginCount = summary?.dependencies.length ?? 0
      const isTemplate = summary ? !summary.materialized : false

      return {
        name,
        title: meta.title || name,
        desc: meta.desc || (pluginCount > 0 ? t.selector.pluginsCount.replace("{count}", String(pluginCount)) : t.selector.customDesc),
        pluginCount,
        isTemplate,
        isDefault,
      }
    })
  })()

  // 启动所选工作台
  const handleLaunch = async (name: string) => {
    if (launchingName) return
    setLaunchingName(name)
    setLocalError(null)

    try {
      if (rememberChoice) {
        await api.setDefaultProfile(name).catch((e) => {
          logger.warn("[selector]", "保存默认工作台偏好失败", { error: e })
        })
      }
      await api.chooseProfile(name)
    } catch (e) {
      setLaunchingName(null)
      setLocalError({
        title: t.error.fallbackTitle,
        detail: `${String(e instanceof Error ? e.message : e)}${t.error.reselectHint}`,
        actions: ["retry"],
      })
    }
  }

  // 键盘快捷直达：按数字键 1~9 快速启动
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (launchingName || e.metaKey || e.ctrlKey || e.altKey) return
      const num = parseInt(e.key, 10)
      if (!isNaN(num) && num >= 1 && num <= displayProfiles.length) {
        const target = displayProfiles[num - 1]
        if (target) {
          e.preventDefault()
          handleLaunch(target.name)
        }
      }
    }
    window.addEventListener("keydown", onKeyDown)
    return () => window.removeEventListener("keydown", onKeyDown)
  }, [displayProfiles, launchingName, rememberChoice])

  const showPulse = launchingName !== null && !shownError && !(!hideDownload && progress !== null)

  const headline =
    shownError !== null
      ? t.selector.problemHeadline
      : launchingName === null
        ? t.selector.headline
        : maxStepSeen >= 1 && activeStep >= 2
          ? t.boot.headlines[Math.min(Math.max(activeStep, 2), 4)]
          : `${t.selector.launchingPrefix}${launchingName}${t.selector.launchingSuffix}`

  const subline = (() => {
    if (launchingName === null) return t.selector.subline
    if (!hideDownload && progress !== null && maxStepSeen < 2) return t.selector.preparingSub
    const detail = steps[Math.max(activeStep, 0)]?.detail
    return detail ?? launchingName
  })()

  return (
    <div className="relative flex min-h-dvh flex-col bg-bg text-ink selection:bg-wash selection:text-brand-deep">
      {/* 顶部环境渐变光晕 */}
      <div className="pointer-events-none absolute inset-x-0 top-0 h-96 bg-[radial-gradient(ellipse_60%_40%_at_50%_-10%,color-mix(in_srgb,var(--color-brand)_14%,transparent),transparent_80%)]" />

      {/* 顶栏微导航 */}
      <header className="relative z-10 flex items-center justify-between border-b border-line/60 px-6 py-3.5 backdrop-blur-md sm:px-8">
        <div className="flex items-center gap-3">
          <Emblem size={24} framed={true} />
          <span className="font-mono text-sm font-semibold tracking-tight text-ink">DSH Dock</span>
          <span className="inline-flex items-center gap-1 rounded-full border border-ok/25 bg-ok/10 px-2 py-0.5 text-meta font-medium text-ok">
            <ShieldCheck className="size-3" />
            {t.selector.engineReady}
          </span>
        </div>

        <div className="flex items-center gap-3">
          <button
            type="button"
            onClick={() => api.openProfilesWindow().catch(() => {})}
            className="inline-flex items-center gap-1.5 rounded-xl border border-line bg-panel px-3 py-1.5 text-xs font-medium text-dim shadow-2xs transition-all hover:border-brand/40 hover:text-ink hover:shadow-xs"
          >
            <SlidersHorizontal className="size-3.5 text-brand-deep" />
            <span>{t.selector.manageWorkbenches}</span>
          </button>
        </div>
      </header>

      {/* 主工作区 */}
      <main className="relative z-10 mx-auto flex w-full max-w-5xl flex-1 flex-col justify-center px-6 py-10 sm:px-8">
        {/* 欢迎语与引导 */}
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.25 }}
          className="mb-8 text-center"
        >
          <h1 className="text-2xl font-bold tracking-tight text-ink sm:text-3xl">{headline}</h1>
          <p className="mx-auto mt-2 max-w-lg text-sm text-dim">{subline}</p>
          {!shownError && showPulse && (
            <div className="mt-5 flex justify-center">
              <PulseBar width={260} />
            </div>
          )}
        </motion.div>

        {/* 状态区：下载进度条接管 */}
        {!hideDownload && progress !== null && (
          <div className="mx-auto mb-8 w-full max-w-xl">
            <DownloadProgress />
          </div>
        )}

        {/* 卡片矩阵 */}
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
          <AnimatePresence>
            {displayProfiles.map((p, idx) => {
              const isLaunching = launchingName === p.name

              return (
                <motion.div
                  key={p.name}
                  initial={{ opacity: 0, y: 14 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: idx * 0.04, duration: 0.22 }}
                  whileHover={{ y: -2, transition: { duration: 0.15 } }}
                  whileTap={{ scale: 0.99 }}
                  onClick={() => handleLaunch(p.name)}
                  className={`group relative flex cursor-pointer flex-col justify-between rounded-2xl border p-5 shadow-xs transition-all ${
                    isLaunching
                      ? "border-brand bg-wash/60 ring-2 ring-brand/30"
                      : "border-line bg-panel/95 hover:border-brand/50 hover:bg-wash/30 hover:shadow-md"
                  }`}
                >
                  {/* 卡片顶栏：图标 + 数字快捷键 + 默认标签 */}
                  <div>
                    <div className="flex items-center justify-between">
                      <div
                        className={`flex size-10 items-center justify-center rounded-xl border transition-colors ${
                          p.isTemplate
                            ? "border-line/70 bg-alt-soft text-alt group-hover:bg-alt group-hover:text-white"
                            : p.isDefault
                              ? "border-brand/30 bg-wash text-brand-deep group-hover:bg-brand group-hover:text-white"
                              : "border-line/70 bg-line-soft text-dim group-hover:bg-brand/10 group-hover:text-brand-deep"
                        }`}
                      >
                        {p.isTemplate ? (
                          <Sparkles className="size-5" />
                        ) : p.isDefault ? (
                          <Layout className="size-5" />
                        ) : (
                          <Package className="size-5" />
                        )}
                      </div>

                      <div className="flex items-center gap-1.5">
                        {p.isDefault && (
                          <span className="inline-flex items-center gap-1 rounded-full border border-brand/25 bg-brand/10 px-2 py-0.5 text-meta font-medium text-brand-deep">
                            <CheckCircle2 className="size-3" />
                            {t.selector.defaultBadge}
                          </span>
                        )}
                        <span className="rounded-md border border-line bg-panel px-1.5 py-0.5 font-mono text-meta text-faint group-hover:border-brand/30 group-hover:text-ink">
                          {idx + 1}
                        </span>
                      </div>
                    </div>

                    {/* 工作台信息 */}
                    <div className="mt-4">
                      <div className="flex items-center gap-2">
                        <h2 className="text-base font-semibold tracking-tight text-ink group-hover:text-brand-deep">
                          {p.title}
                        </h2>
                        <span className="rounded-md bg-line-soft px-1.5 py-0.5 font-mono text-meta text-faint">
                          {p.name}
                        </span>
                      </div>
                      <p className="mt-1 line-clamp-2 text-xs leading-relaxed text-dim" title={p.desc}>
                        {p.desc}
                      </p>
                    </div>
                  </div>

                  {/* 卡片底栏：插件数与进入按钮 */}
                  <div className="mt-6 flex items-center justify-between border-t border-line/60 pt-3">
                    <span className="font-mono text-meta text-faint">
                      {p.pluginCount > 0
                        ? t.selector.pluginsCount.replace("{count}", String(p.pluginCount))
                        : p.name === "web"
                          ? t.selector.officialReadyToUse
                          : t.selector.customDesc}
                    </span>
                    <div className="flex items-center gap-1 text-xs font-medium text-brand-deep transition-transform group-hover:translate-x-0.5">
                      {isLaunching ? (
                        <span className="inline-flex items-center gap-1">
                          <Loader2 className="size-3.5 animate-spin" />
                          {t.selector.launching}
                        </span>
                      ) : (
                        <span className="inline-flex items-center gap-1">
                          {t.selector.enterWorkbench}
                          <ArrowUpRight className="size-3.5" />
                        </span>
                      )}
                    </div>
                  </div>
                </motion.div>
              )
            })}

            {/* 新建工作台卡片 */}
            <motion.div
              key="create-new"
              initial={{ opacity: 0, y: 14 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: displayProfiles.length * 0.04, duration: 0.22 }}
              whileHover={{ y: -2 }}
              onClick={() => api.openProfilesWindow().catch(() => {})}
              className="group flex cursor-pointer flex-col items-center justify-center rounded-2xl border border-dashed border-line bg-panel/40 p-6 text-center shadow-2xs transition-all hover:border-brand/60 hover:bg-wash/20"
            >
              <div className="flex size-10 items-center justify-center rounded-full border border-line bg-panel text-faint transition-colors group-hover:border-brand/40 group-hover:text-brand-deep">
                <Plus className="size-5" />
              </div>
              <h3 className="mt-3 text-sm font-semibold text-ink group-hover:text-brand-deep">
                {t.selector.createWorkbench}
              </h3>
              <p className="mt-1 text-xs text-dim">{t.selector.createWorkbenchDesc}</p>
            </motion.div>
          </AnimatePresence>
        </div>

        {/* 错误卡 */}
        {shownError && (
          <div className="mx-auto mt-6 w-full max-w-xl">
            <ErrorCard payload={shownError} onReselect={() => window.location.reload()} />
          </div>
        )}

        {/* 底部偏好设置栏：记住默认选择 + 快捷提示 */}
        <div className="mt-10 flex flex-col items-center justify-between gap-4 border-t border-line/60 pt-6 text-xs text-dim sm:flex-row">
          <label className="flex cursor-pointer items-center gap-2 select-none hover:text-ink">
            <input
              type="checkbox"
              checked={rememberChoice}
              onChange={(e) => setRememberChoice(e.target.checked)}
              className="accent-brand size-4 rounded-md"
            />
            <span>{t.selector.rememberChoice}</span>
          </label>

          <div className="flex items-center gap-4 text-faint">
            <span>{t.selector.quickKeysHint}</span>
          </div>
        </div>
      </main>
    </div>
  )
}

