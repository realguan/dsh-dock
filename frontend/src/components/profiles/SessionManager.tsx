import { useCallback, useEffect, useMemo, useState } from "react"
import { motion } from "framer-motion"
import {
  AlertTriangle,
  Archive,
  Check,
  ChevronDown,
  ChevronRight,
  Clipboard,
  Clock,
  Copy,
  ExternalLink,
  FileCode,
  Folder,
  HardDrive,
  HelpCircle,
  Layers,
  List,
  LoaderCircle,
  RefreshCw,
  Search,
  ShieldCheck,
  Sparkles,
  Trash2,
  Wrench,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { useI18n } from "@/stores/i18nStore"
import type { SessionItem } from "@/types/ipc"
import { Button } from "@/components/ui/button"

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

function formatRelativeTime(timestamp: number): string {
  if (!timestamp) return "未知"
  const diff = Date.now() - timestamp
  const seconds = Math.floor(diff / 1000)
  if (seconds < 60) return "刚刚"
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return `${minutes} 分钟前`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours} 小时前`
  const days = Math.floor(hours / 24)
  if (days < 30) return `${days} 天前`
  return new Date(timestamp).toLocaleDateString()
}

/** 会话展示名：标题优先；无标题回退「未命名会话」（ID 由次行承载）。 */
function sessionDisplayName(s: SessionItem, noTitle: string): string {
  return s.title?.trim() ? s.title.trim() : noTitle
}

/** 状态视觉映射：色点 + 徽标 + 描述。 */
function statusMeta(
  status: SessionItem["status"],
  t: ReturnType<typeof useI18n>["t"],
): { dot: string; badge: string; desc: string } {
  switch (status) {
    case "healthy":
      return {
        dot: "bg-emerald-500",
        badge: `${t.sessions.statusHealthy}`,
        desc: t.sessions.statusHealthyDesc,
      }
    case "needs_repair":
      return {
        dot: "bg-amber-500 animate-pulse",
        badge: t.sessions.statusNeedsRepair,
        desc: t.sessions.statusNeedsRepairDesc,
      }
    default:
      return {
        dot: "bg-faint/50",
        badge: t.sessions.statusUnknown,
        desc: t.sessions.statusUnknownDesc,
      }
  }
}

export function SessionManager({
  refreshKey,
  onNotice,
}: {
  refreshKey: number
  onNotice?: (text: string, kind?: "ok" | "warn") => void
}) {
  const { t } = useI18n()
  const [sessions, setSessions] = useState<SessionItem[] | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState("")
  const [statusFilter, setStatusFilter] = useState<"all" | "needs_repair">("all")
  const [repairingTarget, setRepairingTarget] = useState<string | null>(null)
  const [deletingTarget, setDeletingTarget] = useState<string | null>(null)
  const [batchRepairing, setBatchRepairing] = useState(false)
  const [copiedId, setCopiedId] = useState<string | null>(null)
  const [copiedPath, setCopiedPath] = useState<string | null>(null)

  // 视图模式：按项目分组 vs 平铺列表
  const [viewMode, setViewMode] = useState<"grouped" | "flat">("grouped")
  const [collapsedProjects, setCollapsedProjects] = useState<Set<string>>(new Set())

  const loadSessions = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const data = await api.listSessions()
      setSessions(data)
    } catch (e) {
      setError(String(e))
      onNotice?.(String(e), "warn")
    } finally {
      setLoading(false)
    }
  }, [onNotice])

  useEffect(() => {
    void loadSessions()
  }, [refreshKey, loadSessions])

  const stats = useMemo(() => {
    if (!sessions) return { total: 0, healthy: 0, needsRepair: 0, running: 0, projectsCount: 0 }
    let healthy = 0
    let needsRepair = 0
    let running = 0
    const projects = new Set<string>()
    for (const s of sessions) {
      projects.add(s.projectDirRaw)
      if (s.active) {
        running++
      } else if (s.status === "needs_repair") {
        needsRepair++
      } else if (s.status === "healthy") {
        healthy++
      }
    }
    return {
      total: sessions.length,
      healthy,
      needsRepair,
      running,
      projectsCount: projects.size,
    }
  }, [sessions])

  const handleRepairSingle = async (session: SessionItem) => {
    setRepairingTarget(session.id)
    try {
      const res = await api.repairSession(session.filePath)
      if (res.success) {
        onNotice?.(t.sessions.repairSuccess(sessionDisplayName(session, t.sessions.noTitle)), "ok")
        await loadSessions()
      } else {
        onNotice?.(res.message || "修复失败", "warn")
      }
    } catch (e) {
      onNotice?.(String(e), "warn")
    } finally {
      setRepairingTarget(null)
    }
  }

  const handleRepairAll = async () => {
    setBatchRepairing(true)
    try {
      const res = await api.repairAllSessions()
      if (res.success) {
        onNotice?.(t.sessions.repairAllSuccess, "ok")
        await loadSessions()
      } else {
        onNotice?.(res.message || "全量修复失败", "warn")
      }
    } catch (e) {
      onNotice?.(String(e), "warn")
    } finally {
      setBatchRepairing(false)
    }
  }

  const handleCopyId = (id: string) => {
    void navigator.clipboard.writeText(id).then(() => {
      setCopiedId(id)
      setTimeout(() => setCopiedId((curr) => (curr === id ? null : curr)), 2000)
    })
  }

  const handleCopyPath = (path: string) => {
    void navigator.clipboard.writeText(path).then(() => {
      setCopiedPath(path)
      onNotice?.(t.sessions.pathCopied, "ok")
      setTimeout(() => setCopiedPath((curr) => (curr === path ? null : curr)), 2000)
    })
  }

  const handleOpenWorkspace = async (path: string) => {
    try {
      await api.openExternal(path)
    } catch (e) {
      onNotice?.(`打开目录失败：${e}`, "warn")
    }
  }

  const handleDeleteSingle = async (session: SessionItem) => {
    if (!window.confirm(`${t.sessions.deleteConfirmTitle(session.id)}\n\n${t.sessions.deleteConfirmNote}`)) {
      return
    }
    setDeletingTarget(session.id)
    try {
      await api.deleteSession(session.filePath)
      onNotice?.(t.sessions.deleteSuccess, "ok")
      await loadSessions()
    } catch (e) {
      onNotice?.(String(e), "warn")
    } finally {
      setDeletingTarget(null)
    }
  }

  const toggleProjectCollapse = (proj: string) => {
    setCollapsedProjects((prev) => {
      const next = new Set(prev)
      if (next.has(proj)) {
        next.delete(proj)
      } else {
        next.add(proj)
      }
      return next
    })
  }

  const filteredSessions = useMemo(() => {
    if (!sessions) return []
    const q = searchQuery.toLowerCase().trim()
    return sessions.filter((s) => {
      if (statusFilter === "needs_repair") {
        // 仅看异常 = 可修复的异常（活跃会话除外——它不能被修，另行展示）
        if (!(s.status === "needs_repair" && s.active !== true)) return false
      }
      if (!q) return true
      return (
        (s.title ?? "").toLowerCase().includes(q) ||
        s.id.toLowerCase().includes(q) ||
        s.projectName.toLowerCase().includes(q) ||
        s.decodedProjectPath.toLowerCase().includes(q) ||
        s.projectDirRaw.toLowerCase().includes(q)
      )
    })
  }, [sessions, searchQuery, statusFilter])

  // 按工作区项目分组
  const projectGroups = useMemo(() => {
    const map = new Map<
      string,
      { projectName: string; decodedPath: string; items: SessionItem[]; totalBytes: number }
    >()

    for (const sess of filteredSessions) {
      const key = sess.projectDirRaw
      if (!map.has(key)) {
        map.set(key, {
          projectName: sess.projectName,
          decodedPath: sess.decodedProjectPath,
          items: [],
          totalBytes: 0,
        })
      }
      const g = map.get(key)!
      g.items.push(sess)
      g.totalBytes += sess.sizeBytes
    }

    return Array.from(map.entries())
  }, [filteredSessions])

  /** 单条会话行（分组/平铺共用）。 */
  const renderSessionRow = (sess: SessionItem) => {
    const isBusy = repairingTarget === sess.id
    const isDeleting = deletingTarget === sess.id
    const isActive = sess.active === true
    // 活跃会话（运行中）不参与「需自愈」：修复会被 dsh 下次 flush 覆盖。
    const isNeedsRepair = sess.status === "needs_repair" && !isActive
    const isHealthy = sess.status === "healthy" && !isActive
    const meta = statusMeta(sess.status, t)
    const displayName = sessionDisplayName(sess, t.sessions.noTitle)

    return (
      <motion.div
        key={sess.id}
        initial={{ opacity: 0, y: 4 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.18 }}
        className={`group relative flex flex-col gap-2.5 p-3.5 transition-colors sm:flex-row sm:items-center ${
          isNeedsRepair
            ? "bg-amber-500/[0.06] hover:bg-amber-500/[0.10]"
            : isHealthy
              ? "hover:bg-line-soft/30"
              : "hover:bg-line-soft/30"
        }`}
      >
        {/* 非健康左侧脉冲条 */}
        {isNeedsRepair && (
          <span className="absolute inset-y-1.5 left-0 w-0.5 rounded-full bg-amber-500/80 animate-pulse" />
        )}

        <div className="min-w-0 flex-1 space-y-1">
          {/* 首行：状态徽标 + 会话名称（标题为主） */}
          <div className="flex flex-wrap items-center gap-2">
            {isActive ? (
              <span
                className="inline-flex shrink-0 items-center gap-1.5 rounded-md bg-sky-500/10 px-1.5 py-0.5 text-[10px] font-medium text-sky-600 dark:text-sky-400"
                title={t.sessions.statusRunningDesc}
              >
                <span className="size-1.5 rounded-full bg-sky-500 animate-pulse" />
                {t.sessions.statusRunning}
              </span>
            ) : (
              <span
                className={`inline-flex shrink-0 items-center gap-1.5 rounded-md px-1.5 py-0.5 text-[10px] font-medium ${
                  isNeedsRepair
                    ? "bg-amber-500/15 text-amber-600 dark:text-amber-400"
                    : isHealthy
                      ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
                      : "bg-line text-dim"
                }`}
                title={meta.desc}
              >
                <span className={`size-1.5 rounded-full ${meta.dot}`} />
                {meta.badge}
              </span>
            )}

            <span
              className="truncate text-[13px] font-semibold text-ink"
              title={displayName}
            >
              {displayName}
            </span>

            {sess.hasBackup && (
              <span className="flex shrink-0 items-center gap-1 rounded-md bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-medium text-emerald-600 dark:text-emerald-400">
                <Archive className="size-2.5" />
                {t.sessions.backupTag}
              </span>
            )}
          </div>

          {/* 次行：ID（等宽可复制）+ 元信息 */}
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-faint">
            <button
              type="button"
              onClick={() => handleCopyId(sess.id)}
              title="复制会话 ID"
              className="inline-flex items-center gap-1 font-mono hover:text-ink transition-colors"
            >
              {sess.id.slice(0, 18)}…
              {copiedId === sess.id ? (
                <Check className="size-3 text-emerald-500" />
              ) : (
                <Clipboard className="size-2.5 opacity-0 group-hover:opacity-70 transition-opacity" />
              )}
            </button>

            <span className="inline-flex items-center gap-1">
              <Clock className="size-3" />
              {formatRelativeTime(sess.updatedAt)}
            </span>
            <span className="inline-flex items-center gap-1">
              <HardDrive className="size-3" />
              {formatBytes(sess.sizeBytes)}
            </span>
            <span className="hidden items-center gap-1 sm:inline-flex">
              <Folder className="size-3" />
              <span className="max-w-[220px] truncate font-mono">{sess.projectName}</span>
            </span>
          </div>

          {/* 异常详情（非健康时展示原因） */}
          {isNeedsRepair && sess.healthDetail && (
            <p className="line-clamp-1 text-[11px] text-amber-600/70 dark:text-amber-400/70">
              {sess.healthDetail}
            </p>
          )}
        </div>

        {/* 操作区：修复按钮仅非健康显示；健康显示状态描述 */}
        <div className="flex shrink-0 items-center gap-1.5 sm:self-center">
          {isActive ? (
            <span
              className="inline-flex h-7 items-center gap-1 rounded-lg border border-sky-500/20 bg-sky-500/[0.06] px-2.5 text-[11px] text-sky-600 dark:text-sky-400"
              title={t.sessions.statusRunningDesc}
            >
              <LoaderCircle className="size-3 animate-spin" />
              {t.sessions.statusRunning}
            </span>
          ) : isNeedsRepair ? (
            <Button
              size="sm"
              onClick={() => handleRepairSingle(sess)}
              disabled={isBusy || batchRepairing || isDeleting}
              className="h-7 gap-1 bg-amber-500 text-white hover:bg-amber-500/90 px-2.5 text-xs"
            >
              {isBusy ? (
                <LoaderCircle className="size-3 animate-spin" />
              ) : (
                <Wrench className="size-3" />
              )}
              <span>{t.sessions.repairBtn}</span>
            </Button>
          ) : (
            <span
              className="inline-flex h-7 items-center gap-1 rounded-lg border border-line bg-panel px-2.5 text-[11px] text-dim"
              title={meta.desc}
            >
              {isHealthy ? (
                <ShieldCheck className="size-3 text-emerald-500/80" />
              ) : (
                <HelpCircle className="size-3 text-faint" />
              )}
              {isHealthy ? t.sessions.statusHealthy : t.sessions.statusUnknown}
            </span>
          )}

          <Button
            size="sm"
            variant="outline"
            title="复制会话文件完整路径"
            onClick={() => handleCopyPath(sess.filePath)}
            className="size-7 p-0"
          >
            {copiedPath === sess.filePath ? (
              <Check className="size-3 text-emerald-500" />
            ) : (
              <Copy className="size-3 text-faint" />
            )}
          </Button>

          <Button
            size="sm"
            variant="outline"
            title={t.sessions.deleteBtn}
            onClick={() => handleDeleteSingle(sess)}
            disabled={isBusy || batchRepairing || isDeleting}
            className="size-7 p-0 hover:border-rose-500/50 hover:bg-rose-500/10 hover:text-rose-500"
          >
            {isDeleting ? (
              <LoaderCircle className="size-3 animate-spin text-rose-500" />
            ) : (
              <Trash2 className="size-3 text-faint" />
            )}
          </Button>
        </div>
      </motion.div>
    )
  }

  return (
    <div className="space-y-4">
      {error && (
        <div className="rounded-xl border border-rose-500/20 bg-rose-500/10 p-3 text-xs text-rose-500">
          {error}
        </div>
      )}

      {/* 顶部状态与操作条 */}
      <div className="rounded-2xl border border-line bg-panel p-4 shadow-xs">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <div className="flex size-10 items-center justify-center rounded-xl bg-brand/10 text-brand">
              <ShieldCheck className="size-5" />
            </div>
            <div>
              <h3 className="text-sm font-bold text-ink">{t.sessions.title}</h3>
              <p className="text-xs text-faint">{t.sessions.subtitle}</p>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Button
              size="sm"
              variant="outline"
              onClick={loadSessions}
              disabled={loading || batchRepairing}
              className="gap-1.5 text-xs"
            >
              <RefreshCw className={`size-3.5 ${loading ? "animate-spin text-brand" : "text-dim"}`} />
              <span>{t.sessions.scanBtn}</span>
            </Button>

            {/* 全局修复：仅存在异常会话时才可点击 */}
            <Button
              size="sm"
              onClick={handleRepairAll}
              disabled={batchRepairing || loading || !sessions || stats.needsRepair === 0}
              title={stats.needsRepair === 0 ? t.sessions.repairAllDisabled : t.sessions.repairNeedHint}
              className={`gap-1.5 text-xs shadow-xs ${
                stats.needsRepair > 0
                  ? "bg-amber-500 text-white hover:bg-amber-500/90"
                  : "bg-brand/80 text-white opacity-55 hover:bg-brand/80 cursor-not-allowed"
              }`}
            >
              {batchRepairing ? (
                <LoaderCircle className="size-3.5 animate-spin" />
              ) : (
                <Sparkles className="size-3.5" />
              )}
              <span>{t.sessions.repairAllBtn}</span>
            </Button>
          </div>
        </div>

        {/* 统计指标：项目 / 总数 / 健康 / 运行中 / 待修复 */}
        <div className="mt-4 grid grid-cols-2 gap-2.5 sm:grid-cols-5">
          <div className="rounded-xl border border-line bg-bg p-3">
            <span className="text-[11px] text-faint">工作区项目数</span>
            <div className="mt-1 font-mono text-base font-bold text-ink">{stats.projectsCount}</div>
          </div>
          <div className="rounded-xl border border-line bg-bg p-3">
            <span className="text-[11px] text-faint">会话总数</span>
            <div className="mt-1 font-mono text-base font-bold text-ink">{stats.total}</div>
          </div>
          <div className="rounded-xl border border-line bg-bg p-3">
            <span className="flex items-center gap-1 text-[11px] text-emerald-600 dark:text-emerald-400">
              <span className="size-1.5 rounded-full bg-emerald-500" />
              健康就绪
            </span>
            <div className="mt-1 font-mono text-base font-bold text-emerald-600 dark:text-emerald-400">
              {stats.healthy}
            </div>
          </div>
          <div className="rounded-xl border border-sky-500/20 bg-sky-500/[0.04] p-3">
            <span className="flex items-center gap-1 text-[11px] text-sky-600 dark:text-sky-400">
              <span className="size-1.5 rounded-full bg-sky-500 animate-pulse" />
              运行中
            </span>
            <div className="mt-1 font-mono text-base font-bold text-sky-600 dark:text-sky-400">
              {stats.running}
            </div>
          </div>
          <div className="rounded-xl border border-amber-500/20 bg-amber-500/[0.04] p-3">
            <span className="flex items-center gap-1 text-[11px] text-amber-600 dark:text-amber-400">
              <span className="size-1.5 rounded-full bg-amber-500 animate-pulse" />
              待修复异常
            </span>
            <div className="mt-1 font-mono text-base font-bold text-amber-600 dark:text-amber-400">
              {stats.needsRepair}
            </div>
          </div>
        </div>
      </div>

      {/* 搜索与视图切换 */}
      <div className="flex flex-wrap items-center justify-between gap-2.5">
        <div className="relative min-w-[220px] flex-1">
          <Search className="text-faint absolute top-1/2 left-3 size-3.5 -translate-y-1/2" />
          <input
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t.sessions.searchPlaceholder}
            className="border-line bg-panel text-ink placeholder:text-faint focus:border-brand w-full rounded-xl border py-1.5 pr-3 pl-8.5 font-mono text-xs outline-none transition-colors shadow-2xs"
          />
        </div>

        {/* 状态筛选：全部 / 仅看异常 */}
        <div className="flex items-center rounded-xl border border-line bg-line-soft/80 p-0.5 shadow-2xs">
          <button
            type="button"
            onClick={() => setStatusFilter("all")}
            className={`flex items-center gap-1 rounded-lg px-2.5 py-1 text-xs font-medium transition-all ${
              statusFilter === "all" ? "bg-panel text-ink shadow-xs" : "text-dim hover:text-ink"
            }`}
          >
            <Layers className="size-3.5" />
            <span>{t.sessions.filterAll}</span>
          </button>
          <button
            type="button"
            onClick={() => setStatusFilter("needs_repair")}
            className={`flex items-center gap-1 rounded-lg px-2.5 py-1 text-xs font-medium transition-all ${
              statusFilter === "needs_repair"
                ? "bg-panel text-amber-600 dark:text-amber-400 shadow-xs"
                : "text-dim hover:text-ink"
            }`}
          >
            <AlertTriangle className="size-3.5" />
            <span>{t.sessions.filterNeedsRepair}</span>
            {stats.needsRepair > 0 && (
              <span className="flex h-4 min-w-4 items-center justify-center rounded-full bg-amber-500/15 px-1 font-mono text-[10px] text-amber-600 dark:text-amber-400">
                {stats.needsRepair}
              </span>
            )}
          </button>
        </div>

        <div className="hidden items-center rounded-xl border border-line bg-line-soft/80 p-0.5 shadow-2xs sm:flex">
          <button
            type="button"
            onClick={() => setViewMode("grouped")}
            className={`flex items-center gap-1 rounded-lg px-2.5 py-1 text-xs font-medium transition-all ${
              viewMode === "grouped" ? "bg-panel text-ink shadow-xs" : "text-dim hover:text-ink"
            }`}
          >
            <Layers className="size-3.5" />
            <span>{t.sessions.groupByProject}</span>
          </button>
          <button
            type="button"
            onClick={() => setViewMode("flat")}
            className={`flex items-center gap-1 rounded-lg px-2.5 py-1 text-xs font-medium transition-all ${
              viewMode === "flat" ? "bg-panel text-ink shadow-xs" : "text-dim hover:text-ink"
            }`}
          >
            <List className="size-3.5" />
            <span>平铺列表</span>
          </button>
        </div>
      </div>

      {/* 会话列表呈现 */}
      {loading && !sessions ? (
        <div className="flex flex-col items-center justify-center rounded-2xl border border-line bg-panel py-12 text-center">
          <LoaderCircle className="size-6 animate-spin text-brand" />
          <span className="text-faint mt-2 text-xs">正在扫描 DSH 会话记录...</span>
        </div>
      ) : filteredSessions.length === 0 ? (
        <div className="flex flex-col items-center justify-center rounded-2xl border border-dashed border-line bg-panel py-12 text-center">
          <FileCode className="text-faint size-8" />
          <p className="text-ink mt-2 text-xs font-medium">
            {searchQuery || statusFilter === "needs_repair"
              ? t.sessions.emptyFilter
              : t.sessions.emptyList}
          </p>
        </div>
      ) : viewMode === "grouped" ? (
        <div className="space-y-3.5">
          {projectGroups.map(([rawKey, group]) => {
            const isCollapsed = collapsedProjects.has(rawKey)
            return (
              <div
                key={rawKey}
                className="overflow-hidden rounded-2xl border border-line bg-panel shadow-xs transition-colors hover:border-brand/30"
              >
                <div className="border-b border-line bg-line-soft/40 px-4 py-3 space-y-1.5">
                  <div className="flex items-center justify-between gap-3">
                    <div
                      onClick={() => toggleProjectCollapse(rawKey)}
                      className="flex cursor-pointer items-center gap-2 min-w-0 flex-1 select-none"
                    >
                      {isCollapsed ? (
                        <ChevronRight className="size-4 text-faint shrink-0" />
                      ) : (
                        <ChevronDown className="size-4 text-faint shrink-0" />
                      )}
                      <Folder className="size-4 text-brand shrink-0" />
                      <span className="font-mono text-xs font-bold text-ink truncate">
                        {group.projectName}
                      </span>
                      <span className="shrink-0 rounded-md bg-line px-1.5 py-0.5 text-[10px] font-mono text-faint">
                        {group.items.length} 个会话 · {formatBytes(group.totalBytes)}
                      </span>
                    </div>

                    <Button
                      size="sm"
                      variant="outline"
                      title={t.sessions.openInFinder}
                      onClick={() => handleOpenWorkspace(group.decodedPath)}
                      className="h-7 shrink-0 gap-1.5 px-2.5 text-xs hover:border-brand hover:text-brand"
                    >
                      <ExternalLink className="size-3" />
                      <span>{t.sessions.openInFinder}</span>
                    </Button>
                  </div>

                  <div className="pl-6">
                    <span
                      className="font-mono text-[11px] text-faint block truncate"
                      title={group.decodedPath}
                    >
                      {group.decodedPath}
                    </span>
                  </div>
                </div>

                {!isCollapsed && (
                  <div className="divide-y divide-line/60">
                    {group.items.map((sess) => renderSessionRow(sess))}
                  </div>
                )}
              </div>
            )
          })}
        </div>
      ) : (
        <div className="divide-y divide-line rounded-2xl border border-line bg-panel shadow-xs overflow-hidden">
          {filteredSessions.map((sess) => renderSessionRow(sess))}
        </div>
      )}
    </div>
  )
}
