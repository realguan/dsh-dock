// components/market/MarketplaceView.tsx —— 插件市场全景工作台 (awesome-dsh-plugin Registry 2700+ 插件)
import { useCallback, useEffect, useMemo, useState } from "react"
import {
  AlertCircle,
  ArrowDownAZ,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  ChevronUp,
  Download,
  Package,
  RefreshCw,
  Search,
  Sparkles,
  Star,
  X,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { useI18n } from "@/stores/i18nStore"
import type { AggregatePlugin, ProfileSummary } from "@/types/ipc"
import type {
  MarketPlugin,
  MarketRegistry,
  MarketSortOption,
} from "@/types/market"
import { filterMarketPlugins, sortMarketPlugins } from "@/lib/market"
import { MarketPluginCard } from "@/components/market/MarketPluginCard"
import { MarketInstallDialog } from "@/components/market/MarketInstallDialog"
import { Button } from "@/components/ui/button"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"

const PAGE_SIZE_OPTIONS = [12, 24, 36, 48]
const DEFAULT_CATEGORY_LIMIT = 10

// 内存单例缓存，避免在同一个 session 内频繁切换 Tab 时重复拉取大 JSON
let cachedRegistry: MarketRegistry | null = null

export function MarketplaceView({
  onNotice,
}: {
  onNotice?: (text: string, kind?: "ok" | "warn") => void
}) {
  const { t, activeLocale } = useI18n()

  const [registry, setRegistry] = useState<MarketRegistry | null>(cachedRegistry)
  const [profiles, setProfiles] = useState<ProfileSummary[]>([])
  const [installedMap, setInstalledMap] = useState<Map<string, string[]>>(new Map())
  const [loading, setLoading] = useState(!cachedRegistry)
  const [error, setError] = useState<string | null>(null)

  // 搜索、分类与排序
  const [searchQuery, setSearchQuery] = useState("")
  const [selectedCategory, setSelectedCategory] = useState<string>("all")
  const [sortOption, setSortOption] = useState<MarketSortOption>("stars")
  const [expandAllCategories, setExpandAllCategories] = useState(false)

  // 分页状态
  const [currentPage, setCurrentPage] = useState(1)
  const [pageSize, setPageSize] = useState(12)

  // 安装弹窗状态
  const [installTarget, setInstallTarget] = useState<MarketPlugin | null>(null)

  // 加载本地安装状态与 Profile
  const loadLocalData = useCallback(async () => {
    try {
      const [profs, allPlugins] = await Promise.all([
        api.listProfiles().catch(() => [] as ProfileSummary[]),
        api.listAllPlugins().catch(() => [] as AggregatePlugin[]),
      ])
      setProfiles(profs)

      const map = new Map<string, string[]>()
      for (const p of allPlugins) {
        const profNames = p.sources.map((s) => s.profile)
        map.set(p.name, profNames)
      }
      setInstalledMap(map)
    } catch {
      // 忽略
    }
  }, [])

  // 加载 Registry 数据
  const loadRegistry = useCallback(
    async (forceRefresh = false) => {
      if (!forceRefresh && cachedRegistry) {
        setRegistry(cachedRegistry)
        setLoading(false)
        return
      }

      setLoading(true)
      setError(null)

      try {
        const rawJson = await api.fetchMarketRegistry()
        const parsed = JSON.parse(rawJson) as MarketRegistry
        cachedRegistry = parsed
        setRegistry(parsed)
      } catch (err) {
        setError(String(err))
      } finally {
        setLoading(false)
      }
    },
    [],
  )

  useEffect(() => {
    void loadLocalData()
    void loadRegistry()
  }, [loadLocalData, loadRegistry])

  // 本地已安装的包名集合（包含 npm 包名和 repo 名字）
  const installedPluginNames = useMemo(() => {
    const set = new Set<string>()
    for (const name of installedMap.keys()) {
      set.add(name)
    }
    return set
  }, [installedMap])

  // 分类列表计算与对应数量统计（按数量降序）
  const categoriesWithCounts = useMemo(() => {
    if (!registry) return []
    const counts: Record<string, number> = {}
    for (const p of registry.plugins) {
      counts[p.category] = (counts[p.category] || 0) + 1
    }

    const items = Object.entries(registry.categories || {}).map(([key, labelObj]) => {
      const label = activeLocale.startsWith("zh") ? labelObj.zh || labelObj.en : labelObj.en || labelObj.zh
      return {
        key,
        label,
        count: counts[key] || 0,
      }
    })

    return items.sort((a, b) => b.count - a.count)
  }, [registry, activeLocale])

  // 过滤后的插件列表
  const filteredPlugins = useMemo(() => {
    if (!registry?.plugins) return []
    return filterMarketPlugins({
      plugins: registry.plugins,
      query: searchQuery,
      category: selectedCategory,
      onlyInstalled: false,
      installedPluginNames,
    })
  }, [registry, searchQuery, selectedCategory, installedPluginNames])

  // 排序后的插件列表
  const sortedPlugins = useMemo(() => {
    return sortMarketPlugins(filteredPlugins, sortOption)
  }, [filteredPlugins, sortOption])

  // 分页切片计算
  const totalItems = sortedPlugins.length
  const totalPages = Math.max(1, Math.ceil(totalItems / pageSize))

  // 搜索或筛选变化时重置页码到第 1 页
  useEffect(() => {
    setCurrentPage(1)
  }, [searchQuery, selectedCategory, sortOption, pageSize])

  const paginatedPlugins = useMemo(() => {
    const start = (currentPage - 1) * pageSize
    return sortedPlugins.slice(start, start + pageSize)
  }, [sortedPlugins, currentPage, pageSize])

  // 打开外部 URL
  const handleOpenExternal = (url: string) => {
    if (!url) return
    api.openExternal(url).catch((err) => {
      onNotice?.(`打开链接失败: ${err}`, "warn")
    })
  }

  // 安装成功回调
  const handleInstallSuccess = (pluginName: string, targetProfile: string) => {
    onNotice?.(t.market.installSuccess(pluginName, targetProfile), "ok")
    void loadLocalData()
  }

  // 显示的分类集合（支持展开/折叠）
  const displayedCategories = useMemo(() => {
    if (expandAllCategories || categoriesWithCounts.length <= DEFAULT_CATEGORY_LIMIT) {
      return categoriesWithCounts
    }
    return categoriesWithCounts.slice(0, DEFAULT_CATEGORY_LIMIT)
  }, [categoriesWithCounts, expandAllCategories])

  return (
    <div className="space-y-4">
      {/* 顶部控制舱 */}
      <section className="rounded-2xl border border-line bg-panel p-4 shadow-2xs space-y-3">
        {/* 顶部搜索、排序与刷新栏 */}
        <div className="flex flex-wrap items-center gap-2.5">
          {/* 搜索框 */}
          <div className="relative min-w-[260px] flex-1">
            <Search className="text-faint absolute top-1/2 left-3 size-3.5 -translate-y-1/2" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t.market.searchPlaceholder}
              className="border-line bg-wash text-ink placeholder:text-faint focus:border-brand focus:bg-panel w-full rounded-xl border py-1.5 pr-8 pl-9 text-xs outline-none shadow-2xs transition-all"
            />
            {searchQuery && (
              <button
                type="button"
                onClick={() => setSearchQuery("")}
                className="absolute top-1/2 right-2.5 -translate-y-1/2 text-faint hover:text-ink"
              >
                <X className="size-3.5" />
              </button>
            )}
          </div>

          {/* 排序方式 */}
          <div className="w-[150px]">
            <Select
              value={sortOption}
              onValueChange={(val) => setSortOption(val as MarketSortOption)}
            >
              <SelectTrigger className="h-8.5 rounded-xl border-line bg-wash text-xs text-ink font-medium">
                <SelectValue />
              </SelectTrigger>
              <SelectContent className="rounded-xl border-line bg-panel text-xs text-ink">
                <SelectItem value="stars">
                  <div className="flex items-center gap-2">
                    <Star className="size-3.5 text-amber-500" />
                    <span>{t.market.sortStars}</span>
                  </div>
                </SelectItem>
                <SelectItem value="downloads">
                  <div className="flex items-center gap-2">
                    <Download className="size-3.5 text-sky-500" />
                    <span>{t.market.sortDownloads}</span>
                  </div>
                </SelectItem>
                <SelectItem value="newest">
                  <div className="flex items-center gap-2">
                    <Sparkles className="size-3.5 text-indigo-500" />
                    <span>{t.market.sortNewest}</span>
                  </div>
                </SelectItem>
                <SelectItem value="name">
                  <div className="flex items-center gap-2">
                    <ArrowDownAZ className="size-3.5 text-violet-500" />
                    <span>{t.market.sortName}</span>
                  </div>
                </SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* 刷新 Registry */}
          <Button
            size="sm"
            variant="outline"
            onClick={() => void loadRegistry(true)}
            disabled={loading}
            className="h-8.5 gap-1.5 rounded-xl text-xs"
          >
            <RefreshCw className={`size-3.5 ${loading ? "animate-spin text-brand" : ""}`} />
            <span>{loading ? "加载中…" : t.market.refreshRegistry}</span>
          </Button>
        </div>

        {/* 平铺分类标签矩阵（无横向滚动条，自动换行，支持展开收起） */}
        {categoriesWithCounts.length > 0 && (
          <div className="space-y-1.5 pt-2 border-t border-line/60">
            <div className="flex flex-wrap items-center gap-1.5">
              {/* 全部标签 */}
              <button
                type="button"
                onClick={() => setSelectedCategory("all")}
                className={`px-2.5 py-1 rounded-lg text-xs transition-all flex items-center gap-1.5 border ${
                  selectedCategory === "all"
                    ? "bg-brand text-white border-brand shadow-2xs font-semibold"
                    : "bg-wash text-dim border-line/60 hover:border-brand/40 hover:text-ink"
                }`}
              >
                <span>{t.market.allCategories}</span>
                <span
                  className={`text-[10px] font-mono rounded px-1 py-0.2 ${
                    selectedCategory === "all"
                      ? "bg-white/20 text-white"
                      : "bg-panel text-faint"
                  }`}
                >
                  {registry?.plugins?.length || 0}
                </span>
              </button>

              {/* 各个分类 Pill */}
              {displayedCategories.map((cat) => (
                <button
                  key={cat.key}
                  type="button"
                  onClick={() => setSelectedCategory(cat.key)}
                  className={`px-2.5 py-1 rounded-lg text-xs transition-all flex items-center gap-1.5 border ${
                    selectedCategory === cat.key
                      ? "bg-brand text-white border-brand shadow-2xs font-semibold"
                      : "bg-wash text-dim border-line/60 hover:border-brand/40 hover:text-ink"
                  }`}
                >
                  <span>{cat.label}</span>
                  <span
                    className={`text-[10px] font-mono rounded px-1 py-0.2 ${
                      selectedCategory === cat.key
                        ? "bg-white/20 text-white"
                        : "bg-panel text-faint"
                    }`}
                  >
                    {cat.count}
                  </span>
                </button>
              ))}

              {/* 展开全部 / 收起分类按钮 */}
              {categoriesWithCounts.length > DEFAULT_CATEGORY_LIMIT && (
                <button
                  type="button"
                  onClick={() => setExpandAllCategories(!expandAllCategories)}
                  className="px-2.5 py-1 rounded-lg text-xs text-brand hover:bg-brand/10 transition-colors flex items-center gap-1 font-medium border border-transparent"
                >
                  <span>
                    {expandAllCategories
                      ? t.market.collapseCategories
                      : t.market.expandCategories(categoriesWithCounts.length)}
                  </span>
                  {expandAllCategories ? (
                    <ChevronUp className="size-3" />
                  ) : (
                    <ChevronDown className="size-3" />
                  )}
                </button>
              )}
            </div>
          </div>
        )}
      </section>

      {/* 错误提示 */}
      {error && (
        <div className="rounded-2xl border border-rose-500/30 bg-rose-500/10 p-6 text-center text-xs text-rose-600 dark:text-rose-400 space-y-2">
          <AlertCircle className="size-6 mx-auto opacity-80" />
          <p className="font-medium">{t.market.loadFailed}</p>
          <p className="font-mono text-[11px] opacity-80 break-all">{error}</p>
          <Button size="sm" variant="outline" onClick={() => void loadRegistry(true)} className="rounded-xl mt-2">
            <RefreshCw className="mr-1 size-3.5" />
            {t.market.retry}
          </Button>
        </div>
      )}

      {/* 加载骨架屏 */}
      {loading && !registry && (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
          {Array.from({ length: 6 }).map((_, i) => (
            <div
              key={i}
              className="h-44 rounded-xl border border-line bg-panel p-4 animate-pulse space-y-3"
            >
              <div className="flex items-center gap-3">
                <div className="size-9 rounded-lg bg-wash" />
                <div className="space-y-1.5 flex-1">
                  <div className="h-3.5 w-24 rounded bg-wash" />
                  <div className="h-2.5 w-16 rounded bg-wash" />
                </div>
              </div>
              <div className="h-3 w-full rounded bg-wash" />
              <div className="h-3 w-3/4 rounded bg-wash" />
              <div className="h-8 w-full rounded-lg bg-wash mt-4" />
            </div>
          ))}
        </div>
      )}

      {/* 插件卡片矩阵 */}
      {!loading && !error && (
        <>
          {paginatedPlugins.length > 0 ? (
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
              {paginatedPlugins.map((plugin) => {
                const catObj = registry?.categories?.[plugin.category]
                const catLabel = catObj
                  ? activeLocale.startsWith("zh")
                    ? catObj.zh || catObj.en
                    : catObj.en || catObj.zh
                  : plugin.category

                // 本地安装在哪些 Profile
                const installedProfs =
                  installedMap.get(plugin.npm || "") ||
                  installedMap.get(plugin.name) ||
                  []

                return (
                  <MarketPluginCard
                    key={plugin.name}
                    plugin={plugin}
                    categoryLabel={catLabel}
                    installedProfiles={installedProfs}
                    onInstall={(p) => setInstallTarget(p)}
                    onOpenExternal={handleOpenExternal}
                    onCopyNotice={(msg) => onNotice?.(msg, "ok")}
                  />
                )
              })}
            </div>
          ) : (
            <div className="rounded-2xl border border-dashed border-line bg-panel/50 p-12 text-center text-xs space-y-2">
              <Package className="size-8 mx-auto text-faint opacity-60" />
              <p className="font-medium text-ink">{t.market.noResults}</p>
              <p className="text-dim text-[11px]">{t.market.noResultsHint}</p>
              {(searchQuery || selectedCategory !== "all") && (
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => {
                    setSearchQuery("")
                    setSelectedCategory("all")
                  }}
                  className="rounded-xl mt-3 text-xs"
                >
                  清除所有筛选条件
                </Button>
              )}
            </div>
          )}

          {/* 底部现代分页控制条 */}
          {totalPages > 1 && (
            <footer className="flex flex-wrap items-center justify-between gap-3 pt-3 border-t border-line/60">
              <div className="text-xs text-dim font-mono">
                {t.market.pageInfo(currentPage, totalPages, totalItems)}
              </div>

              <div className="flex items-center gap-2">
                {/* 每页大小选择 */}
                <div className="flex items-center gap-1.5 text-xs text-dim">
                  <span>{t.market.pageSize}</span>
                  <Select
                    value={String(pageSize)}
                    onValueChange={(val) => setPageSize(Number(val))}
                  >
                    <SelectTrigger className="h-7.5 w-18 rounded-lg border-line bg-wash text-xs text-ink font-mono">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent className="rounded-xl border-line bg-panel text-xs text-ink">
                      {PAGE_SIZE_OPTIONS.map((size) => (
                        <SelectItem key={size} value={String(size)}>
                          {size}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>

                {/* 翻页按钮 */}
                <div className="flex items-center gap-1">
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={currentPage <= 1}
                    onClick={() => setCurrentPage((p) => Math.max(1, p - 1))}
                    className="size-8 p-0 rounded-lg"
                    title={t.market.paginationPrev}
                  >
                    <ChevronLeft className="size-4" />
                  </Button>

                  {/* 简易页码指示 */}
                  <span className="px-2 font-mono text-xs text-ink">
                    {currentPage} / {totalPages}
                  </span>

                  <Button
                    size="sm"
                    variant="outline"
                    disabled={currentPage >= totalPages}
                    onClick={() => setCurrentPage((p) => Math.min(totalPages, p + 1))}
                    className="size-8 p-0 rounded-lg"
                    title={t.market.paginationNext}
                  >
                    <ChevronRight className="size-4" />
                  </Button>
                </div>
              </div>
            </footer>
          )}
        </>
      )}

      {/* 安装对话框 */}
      <MarketInstallDialog
        open={installTarget !== null}
        plugin={installTarget}
        profiles={profiles}
        installedProfiles={
          installTarget
            ? installedMap.get(installTarget.npm || "") ||
              installedMap.get(installTarget.name) ||
              []
            : []
        }
        onClose={() => setInstallTarget(null)}
        onSuccess={handleInstallSuccess}
      />
    </div>
  )
}
