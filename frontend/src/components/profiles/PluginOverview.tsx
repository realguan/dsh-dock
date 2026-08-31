// 插件总览矩阵（Plugin Matrix 2.0，4.4④ 收口重构）。
// 跨 Profile 第三方插件全景聚合视图 + 实时搜索 + 一键跨 Profile 分发安装。
import { useEffect, useMemo, useState } from "react"
import {
  ArrowRight,
  CircleDot,
  Copy,
  Download,
  Layers,
  LoaderCircle,
  Package,
  Search,
  Send,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { t } from "@/content/zh-CN"
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

export function PluginOverview({
  refreshKey,
  onNotice,
}: {
  refreshKey: number
  onNotice?: (text: string, kind?: "ok" | "warn") => void
}) {
  const [list, setList] = useState<AggregatePlugin[] | null>(null)
  const [profiles, setProfiles] = useState<ProfileSummary[]>([])
  const [error, setError] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState("")

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

  useEffect(() => {
    let alive = true
    setError(null)
    Promise.all([api.listAllPlugins(), api.listProfiles().catch(() => [])])
      .then(([plugins, profs]) => {
        if (alive) {
          setList(plugins)
          setProfiles(profs)
        }
      })
      .catch((e) => {
        if (alive) setError(String(e))
      })
    return () => {
      alive = false
    }
  }, [refreshKey])

  // 过滤后的插件列表
  const filteredList = useMemo(() => {
    if (!list) return null
    if (!searchQuery.trim()) return list
    const q = searchQuery.toLowerCase().trim()
    return list.filter(
      (a) =>
        a.name.toLowerCase().includes(q) ||
        (a.description && a.description.toLowerCase().includes(q)) ||
        a.sources.some((s) => s.profile.toLowerCase().includes(q)),
    )
  }, [list, searchQuery])

  // 汇总口径
  const profileCount = list
    ? new Set(list.flatMap((a) => a.sources.map((s) => s.profile))).size
    : 0

  // 分发弹窗的可选目标 Profile 列表
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
      const out = await api.installPlugin(
        selectedDest,
        `${distributeTarget.pkg}@${distributeTarget.version}`,
      )
      if (!out.ok) {
        setDistributeError(out.detail)
        setDistributing(false)
        return
      }

      if (withConfig && distributeTarget.sources[0]) {
        try {
          await api.copyPluginConfig(
            distributeTarget.sources[0],
            selectedDest,
            distributeTarget.pkg,
          )
        } catch {
          // 容忍配置复制失败
        }
      }

      onNotice?.(t.profiles.distributeDone(distributeTarget.pkg, selectedDest), "ok")
      setDistributeTarget(null)
      setSelectedDest(null)
    } catch (e) {
      setDistributeError(String(e))
    } finally {
      setDistributing(false)
    }
  }

  return (
    <section aria-label={t.profiles.viewPlugins} className="space-y-3">
      {/* ── 矩阵顶栏：搜索框 + 统计指标 ── */}
      <div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-line bg-panel p-3.5 shadow-xs">
        <div className="relative min-w-[240px] flex-1">
          <Search className="text-faint absolute top-1/2 left-3 size-3.5 -translate-y-1/2" />
          <input
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t.profiles.searchAllPluginsPlaceholder}
            className="border-line bg-bg text-ink placeholder:text-faint focus:border-brand w-full rounded-lg border py-1.5 pr-3 pl-8.5 font-mono text-xs outline-none transition-colors"
          />
        </div>

        {list !== null && (
          <div className="flex items-center gap-3 text-xs text-dim">
            <span className="inline-flex items-center gap-1 font-mono">
              <Package className="size-3.5 text-brand" />
              {t.profiles.metaBundles(list.length)}
            </span>
            <span className="text-line">|</span>
            <span className="inline-flex items-center gap-1 font-mono">
              <Layers className="size-3.5 text-brand" />
              {t.profiles.overviewSourceCount(profileCount)}
            </span>
          </div>
        )}
      </div>

      {/* ── 状态：加载 / 错误 / 空态 ── */}
      {error && (
        <div className="border-line bg-warn-soft text-warn rounded-xl border border-dashed px-4 py-6 text-center text-xs whitespace-pre-wrap">
          {error}
        </div>
      )}

      {!error && list === null && (
        <div className="border-line bg-panel text-faint rounded-xl border border-dashed py-12 text-center text-xs">
          <LoaderCircle className="mx-auto mb-2 size-5 animate-spin text-brand" />
          {t.profiles.busyShort}
        </div>
      )}

      {!error && list !== null && list.length === 0 && (
        <div className="border-line bg-panel rounded-xl border border-dashed px-4 py-12 text-center">
          <div className="text-dim text-sm font-medium">{t.profiles.overviewEmpty}</div>
          <div className="text-faint mt-1 text-xs">{t.profiles.overviewEmptyHint}</div>
        </div>
      )}

      {/* ── 插件卡片矩阵 ── */}
      {!error && filteredList !== null && filteredList.length > 0 && (
        <div className="space-y-2">
          {filteredList.map((a, i) => {
            const installedProfiles = new Set(a.sources.map((s) => s.profile))
            const uninstalledProfiles = profiles.filter(
              (p) => p.materialized && !installedProfiles.has(p.name),
            )
            const representativeVersion =
              a.sources.find((s) => s.version !== null)?.version ?? "latest"

            return (
              <article
                key={a.name}
                style={{ animationDelay: `${Math.min(i, 8) * 30}ms` }}
                className="page-rise group/plugin rounded-xl border border-line bg-panel shadow-xs transition-all hover:shadow-md hover:border-brand/20"
              >
                {/* ─── 卡片头部：插件名称 + 描述 + 分发按钮 ─── */}
                <div className="flex items-start gap-3 p-4 pb-0">
                  {/* 插件图标占位 */}
                  <div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-wash text-brand">
                    <Package className="size-4" />
                  </div>

                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span
                        className="text-ink font-mono text-sm font-bold tracking-tight"
                        title={a.name}
                      >
                        {a.name}
                      </span>
                    </div>

                    {a.description && (
                      <p className="text-dim mt-0.5 line-clamp-2 text-xs leading-relaxed" title={a.description}>
                        {a.description}
                      </p>
                    )}
                  </div>

                  {/* 快捷分发按钮 */}
                  {uninstalledProfiles.length > 0 && (
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => {
                        setDistributeTarget({
                          pkg: a.name,
                          version: representativeVersion,
                          sources: a.sources.map((s) => s.profile),
                        })
                        setSelectedDest(uninstalledProfiles[0]?.name ?? null)
                        setWithConfig(false)
                        setDistributeError(null)
                      }}
                      className="shrink-0 gap-1.5 text-xs opacity-60 transition-opacity group-hover/plugin:opacity-100"
                    >
                      <Send className="size-3" />
                      <span>{t.profiles.quickDistribute}</span>
                    </Button>
                  )}
                </div>

                {/* ─── 卡片底部：Profile 分布矩阵芯片 ─── */}
                <div className="flex flex-wrap items-center gap-1.5 px-4 pt-2.5 pb-3.5">
                  <span className="mr-1 text-[10px] font-medium tracking-wide text-faint uppercase">
                    分布
                  </span>
                  {a.sources.map((s) =>
                    s.version === null ? (
                      <span
                        key={s.profile}
                        className="inline-flex items-center gap-1 rounded-md bg-warn-soft px-2 py-1 font-mono text-[11px] text-warn"
                      >
                        <CircleDot className="size-2.5" />
                        {s.profile}
                        <span className="text-[10px] opacity-70">· {t.profiles.overviewNotInstalled}</span>
                      </span>
                    ) : (
                      <span
                        key={s.profile}
                        className="inline-flex items-center gap-1.5 rounded-md border border-line bg-bg px-2 py-1 font-mono text-[11px] text-dim transition-colors hover:border-brand/30 hover:bg-wash"
                      >
                        <span className="size-1.5 rounded-full bg-ok shrink-0" />
                        <span className="text-ink font-semibold">{s.profile}</span>
                        <span className="text-brand font-medium">{s.version}</span>
                      </span>
                    ),
                  )}
                </div>
              </article>
            )
          })}
        </div>
      )}

      {/* 搜索结果为空提示 */}
      {!error && filteredList !== null && filteredList.length === 0 && list !== null && list.length > 0 && (
        <div className="border-line bg-panel rounded-xl border border-dashed px-4 py-10 text-center">
          <Search className="mx-auto mb-2 size-5 text-faint" />
          <div className="text-dim text-sm font-medium">未找到匹配的插件</div>
          <div className="text-faint mt-1 text-xs">尝试缩短或修改搜索关键词</div>
        </div>
      )}

      {/* ── 快捷分发安装弹窗 ── */}
      <Dialog
        open={distributeTarget !== null}
        onOpenChange={(o) => !o && setDistributeTarget(null)}
      >
        <DialogContent className="sm:max-w-[440px]">
          <DialogHeader>
            <DialogTitle className="text-sm font-semibold">
              {distributeTarget ? t.profiles.distributeTitle(distributeTarget.pkg) : ""}
            </DialogTitle>
            <DialogDescription className="text-xs">
              {t.profiles.distributeNote}
            </DialogDescription>
          </DialogHeader>

          {distributeError && (
            <div className="rounded-lg bg-warn-soft p-3 text-xs text-warn">
              {distributeError}
            </div>
          )}

          {distributeTarget && (
            <div className="space-y-4 py-1">
              {/* 目标 Profile 选择器（Radix Select） */}
              <div>
                <label className="text-dim mb-2 block text-xs font-medium">
                  目标 Profile
                </label>
                <Select
                  value={selectedDest ?? undefined}
                  onValueChange={(v) => setSelectedDest(v)}
                >
                  <SelectTrigger className="h-9">
                    <SelectValue placeholder="选择目标 Profile..." />
                  </SelectTrigger>
                  <SelectContent>
                    {distributeDestinations.map((p) => (
                      <SelectItem key={p.name} value={p.name}>
                        <span className="inline-flex items-center gap-2">
                          <span>{p.name}</span>
                          {p.web_ui && (
                            <span className="rounded bg-wash px-1 py-px text-[9px] text-brand font-medium">
                              Web
                            </span>
                          )}
                        </span>
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>

              {/* 安装版本信息卡 */}
              <div className="rounded-lg border border-line bg-bg p-3">
                <div className="flex items-center justify-between">
                  <span className="text-xs text-dim flex items-center gap-1.5">
                    <Download className="size-3 text-faint" />
                    安装版本
                  </span>
                  <span className="text-xs text-ink font-mono font-semibold">
                    {distributeTarget.version}
                  </span>
                </div>
                {distributeTarget.sources.length > 0 && (
                  <div className="mt-2 flex items-center gap-1.5 border-t border-line-soft pt-2">
                    <span className="text-[10px] text-faint">来源</span>
                    <ArrowRight className="size-2.5 text-faint" />
                    <div className="flex flex-wrap gap-1">
                      {distributeTarget.sources.map((src) => (
                        <span
                          key={src}
                          className="inline-flex items-center gap-1 rounded bg-wash px-1.5 py-0.5 font-mono text-[10px] text-brand"
                        >
                          <Copy className="size-2" />
                          {src}
                        </span>
                      ))}
                    </div>
                  </div>
                )}
              </div>

              {/* 配置迁移开关 */}
              <label className="flex cursor-pointer items-center justify-between gap-3 rounded-lg border border-line-soft bg-bg/50 px-3 py-2.5 transition-colors hover:bg-wash/50">
                <div>
                  <div className="text-xs font-medium text-ink">连带复制 Patch 配置</div>
                  <div className="text-[10px] text-faint mt-0.5">
                    将来源 Profile 的 cordis.patch.yml 配置项一并迁移
                  </div>
                </div>
                <Switch
                  checked={withConfig}
                  onCheckedChange={setWithConfig}
                />
              </label>
            </div>
          )}

          <DialogFooter>
            <Button
              variant="outline"
              disabled={distributing}
              onClick={() => setDistributeTarget(null)}
            >
              {t.profiles.pluginInstallCancel}
            </Button>
            <Button
              disabled={!selectedDest || distributing}
              onClick={handleDistribute}
              className="gap-1.5"
            >
              {distributing ? (
                <LoaderCircle className="size-3.5 animate-spin" />
              ) : (
                <Send className="size-3.5" />
              )}
              <span>确认分发安装</span>
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  )
}
