// PluginOverview.tsx —— 全域插件总览与分发中心（4.4④ 升级版：宫格卡片 + 分页 + Profile 筛选）。
import { useEffect, useMemo, useState } from "react"
import {
  ChevronLeft,
  ChevronRight,
  Download,
  Filter,
  Layers,
  LoaderCircle,
  Package,
  Search,
  Send,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { useI18n } from "@/stores/i18nStore"
import type { AggregatePlugin, ProfileSummary } from "@/types/ipc"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Switch } from "@/components/ui/switch"

const PAGE_SIZE_OPTIONS = [6, 9, 12, 18]

export function PluginOverview({
  refreshKey,
  onNotice,
}: {
  refreshKey: number
  onNotice?: (text: string, kind?: "ok" | "warn") => void
}) {
  const { t } = useI18n()
  const [list, setList] = useState<AggregatePlugin[] | null>(null)
  const [profiles, setProfiles] = useState<ProfileSummary[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState("")

  // 筛选与分页状态
  const [selectedProfileFilter, setSelectedProfileFilter] = useState<string>("all")
  const [currentPage, setCurrentPage] = useState(1)
  const [pageSize, setPageSize] = useState(9)

  // 快速分发安装弹窗
  const [distributeTarget, setDistributeTarget] = useState<{
    pkg: string
    version: string
    sources: string[]
  } | null>(null)
  const [selectedDest, setSelectedDest] = useState<string | null>(null)
  const [withConfig, setWithConfig] = useState(false)
  const [distributing, setDistributing] = useState(false)
  const [distributeError, setDistributeError] = useState<string | null>(null)

  const loadData = () => {
    setLoading(true)
    setError(null)
    Promise.all([api.listAllPlugins(), api.listProfiles().catch(() => [])])
      .then(([plugins, profs]) => {
        setList(plugins)
        setProfiles(profs)
      })
      .catch((e) => {
        setError(String(e))
      })
      .finally(() => {
        setLoading(false)
      })
  }

  useEffect(() => {
    loadData()
  }, [refreshKey])

  // 过滤后的插件列表（按搜索关键词 + 按 Profile 筛选）
  const filteredList = useMemo(() => {
    if (!list) return []
    let result = list

    // Profile 过滤
    if (selectedProfileFilter !== "all") {
      result = result.filter((item) =>
        item.sources.some((s) => s.profile === selectedProfileFilter),
      )
    }

    // 关键词搜索
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase().trim()
      result = result.filter(
        (item) =>
          item.name.toLowerCase().includes(q) ||
          (item.description && item.description.toLowerCase().includes(q)) ||
          item.sources.some((s) => s.profile.toLowerCase().includes(q)),
      )
    }

    return result
  }, [list, selectedProfileFilter, searchQuery])

  // 重置分页（当搜索或筛选变更时）
  useEffect(() => {
    setCurrentPage(1)
  }, [searchQuery, selectedProfileFilter, pageSize])

  // 分页计算
  const totalItems = filteredList.length
  const totalPages = Math.max(1, Math.ceil(totalItems / pageSize))
  const paginatedList = useMemo(() => {
    const start = (currentPage - 1) * pageSize
    return filteredList.slice(start, start + pageSize)
  }, [filteredList, currentPage, pageSize])

  // 可分发的目标 Profiles（已物化且尚未安装该插件的）
  const distributeDestinations = useMemo(() => {
    if (!distributeTarget) return []
    return profiles.filter(
      (p) => p.materialized && !distributeTarget.sources.includes(p.name),
    )
  }, [profiles, distributeTarget])

  const handleDistribute = async () => {
    if (!distributeTarget || !selectedDest || distributing) return
    setDistributing(true)
    setDistributeError(null)

    try {
      // 1. 安装插件
      const outcome = await api.installPlugin(
        selectedDest,
        `${distributeTarget.pkg}@${distributeTarget.version}`,
      )
      if (!outcome.ok) {
        throw new Error(outcome.detail)
      }

      // 2. 如果勾选了带配置迁移，复制 patch 配置行
      if (withConfig && distributeTarget.sources[0]) {
        try {
          await api.copyPluginConfig(
            distributeTarget.sources[0],
            selectedDest,
            distributeTarget.pkg,
          )
        } catch {
          // 配置迁移非阻断
        }
      }

      onNotice?.(t.profiles.distributeDone(distributeTarget.pkg, selectedDest), "ok")
      setDistributeTarget(null)
      loadData()
    } catch (e) {
      setDistributeError(String(e))
    } finally {
      setDistributing(false)
    }
  }

  return (
    <div className="space-y-4">
      {/* 顶部搜索、Profile 筛选与分页大小控制栏 */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-wrap items-center gap-2.5 flex-1 min-w-[280px]">
          {/* 搜索框 */}
          <div className="relative flex-1 min-w-[200px]">
            <Search className="text-faint absolute top-1/2 left-3 size-3.5 -translate-y-1/2" />
            <input
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t.profiles.searchAllPluginsPlaceholder}
              className="border-line bg-panel text-ink placeholder:text-faint focus:border-brand w-full rounded-xl border py-1.5 pr-3 pl-8.5 font-mono text-xs outline-none transition-colors shadow-2xs"
            />
          </div>

          {/* Profile 下拉筛选器 */}
          <div className="w-48">
            <Select
              value={selectedProfileFilter}
              onValueChange={(v) => setSelectedProfileFilter(v)}
            >
              <SelectTrigger className="h-8.5 rounded-xl border-line bg-panel text-xs">
                <div className="flex items-center gap-1.5 truncate">
                  <Filter className="size-3 text-faint shrink-0" />
                  <span className="truncate">
                    {selectedProfileFilter === "all"
                      ? "全部 Profile"
                      : `Profile: ${selectedProfileFilter}`}
                  </span>
                </div>
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="all">全部 Profile</SelectItem>
                {profiles.map((p) => (
                  <SelectItem key={p.name} value={p.name}>
                    {p.name} {p.web_ui ? "(Web)" : ""}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>

        {/* 条数与统计 */}
        <div className="flex items-center gap-2">
          <span className="text-xs font-mono text-faint">
            共 {totalItems} 个插件
          </span>
          <Select
            value={String(pageSize)}
            onValueChange={(v) => setPageSize(Number(v))}
          >
            <SelectTrigger className="h-8 w-24 rounded-lg border-line bg-panel text-[11px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {PAGE_SIZE_OPTIONS.map((opt) => (
                <SelectItem key={opt} value={String(opt)}>
                  {opt} 条/页
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>

      {error && (
        <div className="rounded-xl border border-rose-500/20 bg-rose-500/10 p-3 text-xs text-rose-500">
          加载全域插件失败：{error}
        </div>
      )}

      {/* 宫格卡片呈现 (Bento Grid) */}
      {loading && !list ? (
        <div className="flex h-64 flex-col items-center justify-center rounded-2xl border border-line bg-panel text-xs text-faint">
          <LoaderCircle className="mb-2 size-6 animate-spin text-brand" />
          <span>正在扫描全域 Profile 插件矩阵...</span>
        </div>
      ) : paginatedList.length === 0 ? (
        <div className="flex h-64 flex-col items-center justify-center rounded-2xl border border-dashed border-line bg-panel/60 text-center">
          <Package className="text-faint mb-2 size-8" />
          <p className="text-xs font-medium text-ink">未找到匹配的插件</p>
          <p className="text-faint mt-1 text-[11px]">
            尝试调整搜索关键词或重置 Profile 筛选条件。
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-3.5 sm:grid-cols-2 lg:grid-cols-3">
          {paginatedList.map((item) => {
            const latestVersion =
              item.sources.find((s) => s.version)?.version || "latest"
            const installedProfiles = item.sources.map((s) => s.profile)

            return (
              <div
                key={item.name}
                className="flex flex-col justify-between rounded-2xl border border-line bg-panel p-4 shadow-2xs transition-all hover:border-brand/40 hover:shadow-xs"
              >
                <div className="space-y-2.5">
                  {/* 头部：包名与版本 */}
                  <div className="flex items-start justify-between gap-2">
                    <div className="flex items-center gap-2 min-w-0">
                      <div className="flex size-7 shrink-0 items-center justify-center rounded-lg bg-brand/10 text-brand">
                        <Package className="size-3.5" />
                      </div>
                      <span
                        className="font-mono text-xs font-bold text-ink truncate"
                        title={item.name}
                      >
                        {item.name}
                      </span>
                    </div>
                    <span className="shrink-0 rounded-md bg-line px-1.5 py-0.5 font-mono text-[10px] text-dim">
                      v{latestVersion}
                    </span>
                  </div>

                  {/* 描述信息 */}
                  <p className="text-xs text-faint line-clamp-2 leading-relaxed min-h-[32px]">
                    {item.description || "暂无插件描述说明"}
                  </p>

                  {/* 已安装到的 Profile 标签 */}
                  <div>
                    <span className="text-[10px] font-semibold text-dim block mb-1">
                      已安装到 ({item.sources.length}):
                    </span>
                    <div className="flex flex-wrap gap-1 max-h-16 overflow-y-auto">
                      {item.sources.map((src) => (
                        <span
                          key={src.profile}
                          className="inline-flex items-center gap-1 rounded-md bg-bg border border-line px-1.5 py-0.5 font-mono text-[10px] text-ink"
                        >
                          <Layers className="size-2.5 text-faint" />
                          <span>{src.profile}</span>
                        </span>
                      ))}
                    </div>
                  </div>
                </div>

                {/* 底部操作条 */}
                <div className="mt-3.5 flex items-center justify-between border-t border-line/60 pt-2.5">
                  <span className="text-[11px] text-faint font-mono">
                    {item.sources.length} 处引用
                  </span>

                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => {
                      setDistributeTarget({
                        pkg: item.name,
                        version: latestVersion,
                        sources: installedProfiles,
                      })
                      setSelectedDest(null)
                      setWithConfig(false)
                      setDistributeError(null)
                    }}
                    className="h-7 gap-1 px-2 text-xs hover:border-brand hover:text-brand"
                  >
                    <Send className="size-3" />
                    <span>分发到...</span>
                  </Button>
                </div>
              </div>
            )
          })}
        </div>
      )}

      {/* 分页控制器 */}
      {totalPages > 1 && (
        <div className="flex items-center justify-between border-t border-line pt-3 text-xs">
          <span className="text-faint text-[11px] font-mono">
            第 {currentPage} / {totalPages} 页
          </span>

          <div className="flex items-center gap-1.5">
            <Button
              size="sm"
              variant="outline"
              onClick={() => setCurrentPage((p) => Math.max(1, p - 1))}
              disabled={currentPage <= 1}
              className="h-7 gap-1 px-2 text-xs"
            >
              <ChevronLeft className="size-3.5" />
              <span>上一页</span>
            </Button>

            <div className="flex items-center gap-1 px-1">
              {Array.from({ length: totalPages }, (_, i) => i + 1).map((page) => (
                <button
                  key={page}
                  type="button"
                  onClick={() => setCurrentPage(page)}
                  className={`size-7 rounded-lg text-xs font-mono transition-colors ${
                    currentPage === page
                      ? "bg-brand text-white font-bold"
                      : "text-dim hover:bg-line"
                  }`}
                >
                  {page}
                </button>
              ))}
            </div>

            <Button
              size="sm"
              variant="outline"
              onClick={() => setCurrentPage((p) => Math.min(totalPages, p + 1))}
              disabled={currentPage >= totalPages}
              className="h-7 gap-1 px-2 text-xs"
            >
              <span>下一页</span>
              <ChevronRight className="size-3.5" />
            </Button>
          </div>
        </div>
      )}

      {/* 快速分发安装弹窗 (解决 Select 宽度截断问题) */}
      <Dialog
        open={distributeTarget !== null}
        onOpenChange={(open) => {
          if (!open) {
            setDistributeTarget(null)
            setDistributeError(null)
          }
        }}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="text-sm font-bold flex items-center gap-2">
              <Send className="size-4 text-brand" />
              <span>{distributeTarget ? t.profiles.distributeTitle(distributeTarget.pkg) : ""}</span>
            </DialogTitle>
            <DialogDescription className="text-xs text-faint">
              {t.profiles.distributeNote}
            </DialogDescription>
          </DialogHeader>

          {distributeError && (
            <div className="rounded-xl bg-rose-500/10 border border-rose-500/20 p-2.5 text-xs text-rose-500">
              {distributeError}
            </div>
          )}

          {distributeTarget && (
            <div className="space-y-4 py-2 text-xs">
              {/* 目标 Profile 选择框（修复宽度截断） */}
              <div className="space-y-1.5">
                <label className="text-dim font-semibold text-[11px]">
                  选择目标 Profile <span className="text-rose-500">*</span>
                </label>
                <Select
                  value={selectedDest ?? undefined}
                  onValueChange={(v) => setSelectedDest(v)}
                >
                  <SelectTrigger className="h-9 w-full min-w-[240px] rounded-xl border-line bg-bg font-mono text-xs">
                    <SelectValue placeholder="请选择目标 Profile..." />
                  </SelectTrigger>
                  <SelectContent className="min-w-[240px]">
                    {distributeDestinations.length === 0 ? (
                      <div className="p-3 text-center text-xs text-faint">
                        所有已物化 Profile 均已安装此插件
                      </div>
                    ) : (
                      distributeDestinations.map((p) => (
                        <SelectItem key={p.name} value={p.name}>
                          <span className="font-mono text-xs text-ink font-semibold">
                            {p.name} {p.web_ui ? "(Web)" : ""}
                          </span>
                        </SelectItem>
                      ))
                    )}
                  </SelectContent>
                </Select>
              </div>

              {/* 安装版本信息 */}
              <div className="rounded-xl border border-line bg-bg p-3 space-y-1.5">
                <div className="flex items-center justify-between">
                  <span className="text-faint text-[11px] flex items-center gap-1">
                    <Download className="size-3" />
                    目标版本
                  </span>
                  <span className="font-mono text-xs font-bold text-ink">
                    {distributeTarget.version}
                  </span>
                </div>
              </div>

              {/* 迁移配置选项 */}
              <div className="flex items-center justify-between pt-2 border-t border-line/60">
                <div className="space-y-0.5">
                  <span className="text-xs font-medium text-ink">连带复制配置行</span>
                  <p className="text-[10px] text-faint">
                    从首个来源 Profile 的 cordis.patch.yml 原样同步配置条目
                  </p>
                </div>
                <Switch checked={withConfig} onCheckedChange={setWithConfig} />
              </div>
            </div>
          )}

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setDistributeTarget(null)}
              disabled={distributing}
            >
              取消
            </Button>
            <Button
              onClick={handleDistribute}
              disabled={distributing || !selectedDest}
              className="bg-brand text-white hover:bg-brand/90"
            >
              {distributing && <LoaderCircle className="size-3.5 animate-spin mr-1.5" />}
              <span>开始分发安装</span>
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
