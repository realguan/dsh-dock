import { useCallback, useEffect, useMemo, useState } from "react"
import {
  AlertCircle,
  Check,
  Download,
  Import,
  LoaderCircle,
  Package,
  Plus,
  Search,
  Sparkles,
  Star,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { useI18n } from "@/stores/i18nStore"
import { useQueueStore } from "@/stores/queueStore"
import { useInstallFlightStore } from "@/stores/installFlightStore"
import type { AggregatePlugin, PluginRowState } from "@/types/ipc"
import type { MarketPlugin, MarketRegistry } from "@/types/market"
import {
  groupPickerCandidates,
  pickerCandidates,
  validatePluginSpec,
  type BatchItemResult,
  type PickerCandidate,
} from "@/lib/profiles"
import {
  detectInstallSource,
  getMarketCategoryLabel,
  getPluginDescription,
  getPluginDisplayName,
  marketCategoryOptions,
} from "@/lib/market"
import { PROFILE_CHIP_CLASS } from "@/lib/format"
import { loadMarketRegistry, peekMarketRegistry } from "@/lib/marketRegistry"
import { GithubIcon, NpmIcon } from "@/components/market/MarketPluginCard"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Segmented } from "@/components/ui/segmented"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

type AddTab = "market" | "import" | "custom"

/** 市场列表一次最多铺多少张卡（与市场页默认每页同口径）。
 *  registry 有几十条，弹窗里全铺会变成一条很长的滚动带；而弹窗的用途是
 *  "搜索后选一个"——引导去搜索/筛分类，比让人无限滚动有效。 */
const MARKET_VISIBLE_LIMIT = 24

interface PluginAddDialogProps {
  target: string
  open: boolean
  defaultTab?: AddTab
  installedPlugins: string[]
  onClose: () => void
  onDone: () => void
  onNotice?: (text: string, kind?: "ok" | "warn") => void
}

export function PluginAddDialog({
  target,
  open,
  defaultTab = "market",
  installedPlugins,
  onClose,
  onDone,
  onNotice,
}: PluginAddDialogProps) {
  const { t, activeLocale } = useI18n()
  const [tab, setTab] = useState<AddTab>(defaultTab)

  // 当外部重新打开弹窗时，对齐初始 tab
  useEffect(() => {
    if (open) {
      setTab(defaultTab)
    }
  }, [open, defaultTab])

  // ================= 1. 市场选购状态 =================
  const [registry, setRegistry] = useState<MarketRegistry | null>(peekMarketRegistry)
  const [marketLoading, setMarketLoading] = useState(false)
  const [marketError, setMarketError] = useState<string | null>(null)
  const [marketSearch, setMarketSearch] = useState("")
  const [selectedCategory, setSelectedCategory] = useState("all")
  const [installingMarketPkg, setInstallingMarketPkg] = useState<string | null>(null)

  const enqueue = useQueueStore((s) => s.enqueue)

  // 市场数据走 lib/marketRegistry.ts（**与插件中心的市场视图共用同一份缓存**）：
  // 2026-09-21 收口前两处各持一份模块级变量，从插件中心切到本弹窗必然重新拉取——
  // 同一份 JSON、同一个 session、两次网络往返（维护者实测报"每次点都重新获取"）。
  useEffect(() => {
    if (!open || tab !== "market") return
    const known = peekMarketRegistry()
    if (known !== null) {
      setRegistry(known)
      return
    }
    let alive = true
    setMarketLoading(true)
    setMarketError(null)
    loadMarketRegistry()
      .then((parsed) => {
        if (alive) setRegistry(parsed)
      })
      .catch((err) => {
        if (alive) setMarketError(String(err))
      })
      .finally(() => {
        if (alive) setMarketLoading(false)
      })
    return () => {
      alive = false
    }
  }, [open, tab])

  // 分类清单与「插件中心」的市场页**共用同一个纯函数**（2026-09-21 维护者真机：
  // "全部分类选项要与插件中心那边的分类保持一致"）。原先这里自己收集一遍原始键
  // （`agi` / `ui` / `usage`…），既没有中文标签、也没有计数、顺序还跟那边不同。
  const categories = useMemo(
    () => marketCategoryOptions(registry, activeLocale),
    [registry, activeLocale],
  )

  const filteredMarketPlugins = useMemo(() => {
    if (!registry) return []
    let list = registry.plugins
    if (selectedCategory !== "all") {
      list = list.filter((p) => p.category === selectedCategory)
    }
    if (marketSearch.trim()) {
      const q = marketSearch.toLowerCase().trim()
      list = list.filter((p) => {
        const desc = getPluginDescription(p.description, activeLocale).toLowerCase()
        return (
          p.name.toLowerCase().includes(q) ||
          p.owner.toLowerCase().includes(q) ||
          desc.includes(q)
        )
      })
    }
    return list
  }, [registry, selectedCategory, marketSearch, activeLocale])

  const visibleMarketPlugins = filteredMarketPlugins.slice(0, MARKET_VISIBLE_LIMIT)
  const hiddenMarketCount = filteredMarketPlugins.length - visibleMarketPlugins.length

  const handleInstallMarketPlugin = (e: React.MouseEvent, p: MarketPlugin) => {
    const sourceInfo = detectInstallSource(p)
    const spec = sourceInfo.spec.trim()
    const invalid = validatePluginSpec(spec)
    if (invalid) {
      onNotice?.(invalid, "warn")
      return
    }
    setInstallingMarketPkg(p.name)
    try {
      useInstallFlightStore
        .getState()
        .launch({ x: e.clientX, y: e.clientY }, p.name)
      enqueue({
        pkg: p.name,
        spec,
        profile: target,
        kind: "install",
      })
      onNotice?.(t.profiles.pluginAddMarketInstallDone(p.name), "ok")
      setTimeout(() => {
        setInstallingMarketPkg(null)
        onDone()
      }, 300)
    } catch (err) {
      setInstallingMarketPkg(null)
      onNotice?.(String(err), "warn")
    }
  }

  // ================= 2. 从其他 Profile 导入状态 =================
  const [importPhase, setImportPhase] = useState<
    | { kind: "loading" }
    | { kind: "picking"; candidates: PickerCandidate[]; rowsFailed: boolean }
    | { kind: "running"; done: number; total: number; current: string }
    | { kind: "done"; results: BatchItemResult[] }
  >({ kind: "loading" })
  const [selectedImportPkgs, setSelectedImportPkgs] = useState<Set<string>>(new Set())
  const [chosenSource, setChosenSource] = useState<Record<string, string>>({})
  const [withConfigPkgs, setWithConfigPkgs] = useState<Set<string>>(new Set())
  const [importLoadError, setImportLoadError] = useState<string | null>(null)

  const loadImportCandidates = useCallback(async () => {
    setImportPhase({ kind: "loading" })
    setSelectedImportPkgs(new Set())
    setChosenSource({})
    setWithConfigPkgs(new Set())
    setImportLoadError(null)
    try {
      const [aggregate, targetPlugins] = await Promise.all([
        api.listAllPlugins(),
        api.listProfilePlugins(target),
      ])
      const targetDeps = targetPlugins
        .filter((p) => p.kind === "dependency")
        .map((p) => p.name)
      const sourceProfiles = [
        ...new Set(
          aggregate.flatMap((a: AggregatePlugin) => a.sources.map((s) => s.profile)),
        ),
      ].filter((p) => p !== target)
      const rowsList = await Promise.allSettled(
        sourceProfiles.map((p) => api.getPluginRows(p)),
      )
      const rowsByProfile: Record<string, PluginRowState[]> = {}
      let rowsFailed = false
      rowsList.forEach((r, i) => {
        if (r.status === "fulfilled") rowsByProfile[sourceProfiles[i]] = r.value
        else rowsFailed = true
      })
      const candidates = pickerCandidates(aggregate, target, targetDeps, rowsByProfile)
      setImportPhase({ kind: "picking", candidates, rowsFailed })
    } catch (e) {
      setImportLoadError(String(e))
    }
  }, [target])

  useEffect(() => {
    if (open && tab === "import") {
      void loadImportCandidates()
    }
  }, [open, tab, loadImportCandidates])

  const groupedImportCandidates = useMemo(() => {
    const raw = importPhase.kind === "picking" ? importPhase.candidates : []
    return groupPickerCandidates(raw)
  }, [importPhase])

  const toggleImportPkg = (pkg: string) => {
    setSelectedImportPkgs((prev) => {
      const next = new Set(prev)
      if (next.has(pkg)) next.delete(pkg)
      else next.add(pkg)
      return next
    })
    setWithConfigPkgs((prev) => {
      const next = new Set(prev)
      next.delete(pkg)
      return next
    })
  }

  const toggleImportConfig = (pkg: string) => {
    setWithConfigPkgs((prev) => {
      const next = new Set(prev)
      if (next.has(pkg)) next.delete(pkg)
      else next.add(pkg)
      return next
    })
  }

  const runImportQueue = async () => {
    const queue = groupedImportCandidates.filter((g) => selectedImportPkgs.has(g.pkg))
    if (queue.length === 0) return
    const results: BatchItemResult[] = []
    for (let i = 0; i < queue.length; i++) {
      const g = queue[i]
      const activeSrcName = chosenSource[g.pkg] || g.sources[0]?.profile
      const activeSrc = g.sources.find((s) => s.profile === activeSrcName) || g.sources[0]
      setImportPhase({ kind: "running", done: i, total: queue.length, current: g.pkg })
      try {
        const out = await api.installPlugin(target, `${g.pkg}@${activeSrc.version}`)
        if (!out.ok) {
          results.push({ pkg: g.pkg, ok: false, detail: out.detail })
          continue
        }
        if (withConfigPkgs.has(g.pkg) && activeSrc.hasConfig) {
          try {
            const cc = await api.copyPluginConfig(activeSrc.profile, target, g.pkg)
            results.push({ pkg: g.pkg, ok: true, detail: cc.detail })
          } catch (e) {
            results.push({
              pkg: g.pkg,
              ok: true,
              detail: t.profiles.importConfigCopyFailed(String(e)),
            })
          }
        } else {
          results.push({ pkg: g.pkg, ok: true, detail: out.detail })
        }
      } catch (e) {
        results.push({ pkg: g.pkg, ok: false, detail: String(e) })
      }
    }
    setImportPhase({ kind: "done", results })
    const okCount = results.filter((r) => r.ok).length
    const failCount = results.length - okCount
    onDone()
    onNotice?.(t.profiles.importDone(okCount, failCount), failCount > 0 ? "warn" : "ok")
  }

  // ================= 3. 手动/自定义安装状态 =================
  const [customSpec, setCustomSpec] = useState("")
  const [customError, setCustomError] = useState<string | null>(null)
  const [customBusy, setCustomBusy] = useState(false)

  const handleCustomInstall = async () => {
    const spec = customSpec.trim()
    if (!spec) return
    const invalid = validatePluginSpec(spec)
    if (invalid) {
      setCustomError(invalid)
      return
    }
    setCustomBusy(true)
    setCustomError(null)
    try {
      const res = await api.installPlugin(target, spec)
      if (!res.ok) {
        setCustomError(res.detail)
        return
      }
      onNotice?.(res.detail, "ok")
      setCustomSpec("")
      onDone()
      onClose()
    } catch (e) {
      setCustomError(String(e))
    } finally {
      setCustomBusy(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      {/* 宽度必须写 `sm:max-w-2xl`：DialogContent 默认带 `sm:max-w-sm`(384px)，而带响应式
            前缀的规则在 CSS 里**后生成**——无前缀的 `max-w-2xl` 会被它压掉，弹窗只剩
            半个宽度（2026-09-21 维护者截图："这个窗口还不协调"的就是这个）。 */}
      <DialogContent className="sm:max-w-2xl max-h-[85vh] flex flex-col overflow-hidden">
        <DialogHeader className="space-y-1">
          <DialogTitle className="flex items-center gap-2 text-base font-semibold">
            <Plus className="size-4 text-brand-deep" />
            <span>{t.profiles.pluginAddTitle(target)}</span>
          </DialogTitle>
          <DialogDescription className="text-xs text-dim">
            {t.profiles.pluginAddDesc}
          </DialogDescription>
        </DialogHeader>

        {/* 顶部 Tab 切换 */}
        <div className="pt-2">
          <Segmented<AddTab>
            stretch
            size="sm"
            value={tab}
            onChange={setTab}
            options={[
              { value: "market", label: t.profiles.pluginAddTabMarket, icon: Sparkles },
              { value: "import", label: t.profiles.pluginAddTabImport, icon: Import },
              { value: "custom", label: t.profiles.pluginAddTabCustom, icon: Package },
            ]}
          />
        </div>

        {/* 主内容区域 */}
        <div className="flex-1 overflow-y-auto mt-3 pr-1 space-y-3 min-h-[300px]">
          {/* ================= TAB 1: 市场选购 ================= */}
          {tab === "market" && (
            <div className="space-y-3">
              {/* 搜索与分类过滤 */}
              <div className="flex flex-wrap items-center gap-2">
                <div className="relative flex-1 min-w-[200px]">
                  <Search className="text-faint absolute inset-y-0 left-2.5 my-auto size-3.5" />
                  <input
                    value={marketSearch}
                    onChange={(e) => setMarketSearch(e.target.value)}
                    placeholder={t.profiles.pluginAddMarketSearch}
                    aria-label={t.profiles.pluginAddMarketSearch}
                    className="border-line bg-panel text-ink placeholder:text-faint focus:border-brand w-full rounded-lg border py-1.5 pr-3 pl-8 text-xs outline-none transition-colors"
                  />
                </div>
                {/* 分类筛选走仓库统一的 Radix Select（原为原生 `<select>`：外观由
                    系统决定，与弹窗内其他控件/插件中心的下拉都不同款——"不协调"
                    的又一处来源）。 */}
                {categories.length > 0 && (
                  <Select value={selectedCategory} onValueChange={setSelectedCategory}>
                    <SelectTrigger
                      aria-label={t.profiles.pluginAddCategoryLabel}
                      className="border-line bg-panel w-[150px] shrink-0 rounded-lg text-xs"
                    >
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="all">{t.market.allCategories}</SelectItem>
                      {categories.map((c) => (
                        <SelectItem key={c.key} value={c.key}>
                          {t.market.categoryOption(c.label, c.count)}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                )}
              </div>

              {marketLoading ? (
                <div className="py-16 text-center text-xs text-faint">
                  <LoaderCircle className="size-5 animate-spin mx-auto mb-2 text-brand-deep" />
                  <span>{t.profiles.pluginAddMarketLoading}</span>
                </div>
              ) : marketError ? (
                <div className="rounded-xl border border-warn/40 bg-warn-soft p-4 text-xs text-warn flex items-start gap-2">
                  <AlertCircle className="size-4 shrink-0 mt-0.5" />
                  <div>
                    <p className="font-semibold">{t.profiles.pluginAddLoadMarketFailed}</p>
                    <p className="text-meta opacity-80 mt-1">{marketError}</p>
                  </div>
                </div>
              ) : filteredMarketPlugins.length === 0 ? (
                <div className="rounded-xl border border-dashed border-line bg-bg p-8 text-center text-xs text-faint">
                  {t.profiles.pluginAddEmptyMarket}
                </div>
              ) : (
                <div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2">
                  {visibleMarketPlugins.map((plugin) => {
                    const isInstalled =
                      installedPlugins.includes(plugin.name) ||
                      (plugin.npm ? installedPlugins.includes(plugin.npm) : false)
                    const displayName = getPluginDisplayName(plugin.name)
                    const desc = getPluginDescription(plugin.description, activeLocale)
                    const isBusy = installingMarketPkg === plugin.name

                    // 与插件中心的市场卡同款视觉语言（rounded-2xl / p-4 / 来源图标）：
                    // 同一份数据在两处呈现风格不一致，就是"这个窗口不协调"的来源。
                    return (
                      <div
                        key={plugin.name}
                        className="group border-line bg-panel shadow-2xs hover:border-brand/40 flex flex-col justify-between rounded-2xl border p-4 transition-colors"
                      >
                        <div className="space-y-1.5">
                          <div className="flex items-start justify-between gap-2">
                            <div className="flex min-w-0 items-center gap-2.5">
                              <span className="border-line bg-wash flex size-8 shrink-0 items-center justify-center rounded-lg border">
                                {plugin.npm ? (
                                  <NpmIcon className="text-dim h-2.5 w-4" />
                                ) : plugin.url?.includes("github.com") ? (
                                  <GithubIcon className="text-dim size-4" />
                                ) : (
                                  <Package className="text-dim size-4" />
                                )}
                              </span>
                              <div className="min-w-0">
                                <h4
                                  className="text-ink truncate font-mono text-xs font-semibold"
                                  title={displayName}
                                >
                                  {displayName}
                                </h4>
                                <p
                                  className="text-faint truncate text-micro"
                                  title={plugin.owner}
                                >
                                  by {plugin.owner}
                                </p>
                              </div>
                            </div>
                            {/* 分类徽标与市场卡片同款（本地化标签 + 同一枚 Badge
                                基座）：原先是原始键 + 手搓小胶囊，同一个东西两处两个样。 */}
                            {plugin.category && (
                              <Badge
                                variant="secondary"
                                className="border-line/60 bg-wash text-dim shrink-0 border text-meta font-medium"
                              >
                                {getMarketCategoryLabel(registry, plugin.category, activeLocale)}
                              </Badge>
                            )}
                          </div>

                          <p
                            className="text-xs text-dim line-clamp-2 leading-relaxed min-h-[32px]"
                            title={desc || undefined}
                          >
                            {desc || t.market.noDescription}
                          </p>

                          <div className="flex items-center gap-3 text-micro text-faint font-mono pt-1">
                            <span className="flex items-center gap-1">
                              <Star className="size-3 text-dim fill-dim/30" />
                              {plugin.stars?.toLocaleString() ?? 0}
                            </span>
                            {plugin.downloads !== null && plugin.downloads !== undefined && (
                              <span className="flex items-center gap-1">
                                <Download className="size-3" />
                                {plugin.downloads >= 1000
                                  ? `${(plugin.downloads / 1000).toFixed(1)}k`
                                  : plugin.downloads}
                              </span>
                            )}
                          </div>
                        </div>

                        <div className="mt-3 flex items-center justify-end border-t border-line/60 pt-2.5">
                          {isInstalled ? (
                            <span className="inline-flex items-center gap-1 text-meta font-mono text-ok bg-ok-soft border border-ok/20 rounded-md px-2 py-0.5">
                              <Check className="size-3" />
                              {t.profiles.pluginAddAlreadyInstalled}
                            </span>
                          ) : (
                            <Button
                              size="sm"
                              disabled={isBusy}
                              onClick={(e) => handleInstallMarketPlugin(e, plugin)}
                              className="gap-1 text-xs"
                            >
                              {isBusy ? (
                                <LoaderCircle className="size-3 animate-spin" />
                              ) : (
                                <Plus className="size-3" />
                              )}
                              <span>
                                {isBusy
                                  ? t.profiles.pluginAddInstalling
                                  : t.profiles.pluginAddInstallAction}
                              </span>
                            </Button>
                          )}
                        </div>
                      </div>
                    )
                  })}
                </div>
              )}

              {hiddenMarketCount > 0 && (
                <p className="text-faint pt-1 text-center text-meta">
                  {t.profiles.pluginAddMoreHidden(hiddenMarketCount)}
                </p>
              )}
            </div>
          )}

          {/* ================= TAB 2: 从其他工作台导入 ================= */}
          {tab === "import" && (
            <div className="space-y-3">
              {importLoadError && (
                <div className="rounded-xl border border-warn/40 bg-warn-soft p-3 text-xs text-warn flex items-center gap-2">
                  <AlertCircle className="size-4 shrink-0" />
                  <span>{importLoadError}</span>
                </div>
              )}

              {importPhase.kind === "loading" ? (
                <div className="py-16 text-center text-xs text-faint">
                  <LoaderCircle className="size-5 animate-spin mx-auto mb-2 text-brand-deep" />
                  <span>{t.profiles.importLoading}</span>
                </div>
              ) : importPhase.kind === "running" ? (
                <div className="py-12 text-center text-xs space-y-2">
                  <LoaderCircle className="size-6 animate-spin mx-auto text-brand-deep" />
                  <p className="font-semibold text-ink">
                    {t.profiles.importRunning(importPhase.done + 1, importPhase.total)}
                  </p>
                  <p className="font-mono text-faint">{importPhase.current}</p>
                </div>
              ) : importPhase.kind === "done" ? (
                <div className="space-y-3">
                  <div className="rounded-xl border border-ok/30 bg-ok-soft p-4 text-center">
                    <Check className="size-6 text-ok mx-auto mb-2" />
                    <h4 className="text-xs font-semibold text-ink">{t.profiles.pluginAddImportDone}</h4>
                    <div className="mt-3 divide-y divide-line rounded-lg border border-line bg-panel text-left">
                      {importPhase.results.map((r) => (
                        <div key={r.pkg} className="p-2.5 flex items-center justify-between text-xs">
                          <span className="font-mono font-semibold">{r.pkg}</span>
                          <span className={r.ok ? "text-ok" : "text-warn"}>
                            {r.ok ? t.profiles.pluginAddImportOk : r.detail}
                          </span>
                        </div>
                      ))}
                    </div>
                  </div>
                </div>
              ) : groupedImportCandidates.length === 0 ? (
                <div className="rounded-xl border border-dashed border-line bg-bg p-8 text-center text-xs text-faint">
                  {t.profiles.importEmpty}
                </div>
              ) : (
                <div className="space-y-2">
                  <div className="divide-y divide-line rounded-xl border border-line bg-panel">
                    {groupedImportCandidates.map((g) => {
                      const isSelected = selectedImportPkgs.has(g.pkg)
                      const isWithConfig = withConfigPkgs.has(g.pkg)
                      const activeSrcName = chosenSource[g.pkg] || g.sources[0]?.profile
                      const activeSrc =
                        g.sources.find((s) => s.profile === activeSrcName) || g.sources[0]

                      return (
                        <div
                          key={g.pkg}
                          className="flex items-center justify-between gap-3 p-3 hover:bg-wash/30 transition-colors"
                        >
                          <div className="flex items-center gap-3 min-w-0">
                            <input
                              type="checkbox"
                              checked={isSelected}
                              aria-label={g.pkg}
                              onChange={() => toggleImportPkg(g.pkg)}
                              className="size-4 rounded-md border-line accent-brand cursor-pointer"
                            />
                            <div className="min-w-0">
                              <span className="font-mono text-xs font-semibold text-ink truncate block">
                                {g.pkg}
                              </span>
                              <div className="flex items-center gap-2 mt-0.5 text-micro text-faint">
                                <span>v{activeSrc.version}</span>
                                <span>{t.profiles.pluginAddImportFrom}:</span>
                                <span className={PROFILE_CHIP_CLASS}>{activeSrc.profile}</span>
                              </div>
                            </div>
                          </div>

                          <div className="flex items-center gap-2">
                            {activeSrc.hasConfig ? (
                              <label className="flex items-center gap-1 text-meta text-dim cursor-pointer">
                                <input
                                  type="checkbox"
                                  disabled={!isSelected}
                                  checked={isWithConfig}
                                  aria-label={`${g.pkg} ${t.profiles.importConfig}`}
                                  onChange={() => toggleImportConfig(g.pkg)}
                                  className="size-3.5 rounded-md border-line accent-brand"
                                />
                                <span>{t.profiles.importConfig}</span>
                              </label>
                            ) : (
                              <span className="text-micro text-faint">
                                {t.profiles.importNoConfig}
                              </span>
                            )}
                          </div>
                        </div>
                      )
                    })}
                  </div>

                  <div className="flex items-center justify-between pt-2">
                    <span className="text-xs text-faint font-mono">
                      {t.profiles.importSelected(selectedImportPkgs.size)}
                    </span>
                    <Button
                      size="sm"
                      disabled={selectedImportPkgs.size === 0}
                      onClick={() => void runImportQueue()}
                      className="gap-1.5"
                    >
                      <Import className="size-3.5" />
                      <span>{t.profiles.importStart(selectedImportPkgs.size)}</span>
                    </Button>
                  </div>
                </div>
              )}
            </div>
          )}

          {/* ================= TAB 3: 自定义 / 手动安装 ================= */}
          {tab === "custom" && (
            <div className="space-y-4 py-2">
              <div className="rounded-xl border border-line bg-wash/30 p-4 space-y-3">
                <div className="space-y-1">
                  <label
                    htmlFor="plugin-add-spec"
                    className="text-ink block text-xs font-semibold"
                  >
                    {t.profiles.pluginAddSpecLabel}
                  </label>
                  <input
                    id="plugin-add-spec"
                    autoFocus
                    value={customSpec}
                    onChange={(e) => setCustomSpec(e.target.value)}
                    onKeyDown={(e) => e.key === "Enter" && !customBusy && handleCustomInstall()}
                    placeholder={t.profiles.pluginAddCustomPlaceholder}
                    aria-label={t.profiles.pluginAddCustomPlaceholder}
                    className="border-line bg-panel text-ink placeholder:text-faint focus:border-brand w-full rounded-xl border px-3 py-2 font-mono text-xs outline-none transition-colors"
                  />
                  <p className="text-meta text-faint mt-1">
                    {t.profiles.pluginAddCustomHint}
                  </p>
                </div>

                {customError && (
                  <div className="rounded-lg bg-warn-soft p-2.5 text-xs text-warn flex items-center gap-2">
                    <AlertCircle className="size-3.5 shrink-0" />
                    <span>{customError}</span>
                  </div>
                )}

                <div className="flex items-center justify-end gap-2 pt-2">
                  <Button
                    size="sm"
                    disabled={customBusy || !customSpec.trim()}
                    onClick={handleCustomInstall}
                    className="gap-1.5"
                  >
                    {customBusy ? (
                      <LoaderCircle className="size-3.5 animate-spin" />
                    ) : (
                      <Download className="size-3.5" />
                    )}
                    <span>
                      {customBusy
                        ? t.profiles.pluginInstallBusy
                        : t.profiles.pluginAddCustomSubmit}
                    </span>
                  </Button>
                </div>
              </div>
            </div>
          )}
        </div>
      </DialogContent>
    </Dialog>
  )
}
