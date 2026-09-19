import { useCallback, useEffect, useMemo, useState } from "react"
import { motion } from "framer-motion"
import {
  AlertTriangle,
  Archive,
  ArchiveRestore,
  Bot,
  CalendarClock,
  Check,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  CircleAlert,
  Clock,
  Copy,
  CircleX,
  ExternalLink,
  FileArchive,
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
import { useCopy } from "@/hooks/useCopy"
import { useI18n } from "@/stores/i18nStore"
import type { SessionItem } from "@/types/ipc"
import { statusMeta } from "@/lib/sessionStatus"
import { Button } from "@/components/ui/button"
import { Segmented } from "@/components/ui/segmented"
import { StateBlock } from "@/components/ui/state-block"
import { ConfirmDialog } from "@/components/ui/confirm-dialog"

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

/** 2026-09-18 收口：相对时间文案迁入 content 字典（原为模块内硬编码中文）。 */
type RelativeTimeCopy = {
  timeUnknown: string
  timeJustNow: string
  timeMinutes: (n: number) => string
  timeHours: (n: number) => string
  timeDays: (n: number) => string
}

function formatRelativeTime(timestamp: number, c: RelativeTimeCopy): string {
  if (!timestamp) return c.timeUnknown
  const diff = Date.now() - timestamp
  const seconds = Math.floor(diff / 1000)
  if (seconds < 60) return c.timeJustNow
  const minutes = Math.floor(seconds / 60)
  if (minutes < 60) return c.timeMinutes(minutes)
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return c.timeHours(hours)
  const days = Math.floor(hours / 24)
  if (days < 30) return c.timeDays(days)
  return new Date(timestamp).toLocaleDateString()
}

/** 会话展示名：标题优先；无标题回退「未命名会话」（ID 由次行承载）。 */
function sessionDisplayName(s: SessionItem, noTitle: string): string {
  return s.title?.trim() ? s.title.trim() : noTitle
}

/** 结束状态展示文案：已知 kind 有专名（真实语料实证全集 completed/aborted/
 * error/open，2026-09-07；stop/interrupted 为文档词汇），其余按「未收尾」兜底。 */
function endStateLabel(
  endState: string,
  t: ReturnType<typeof useI18n>["t"],
): string {
  if (endState === "stop" || endState === "completed") return t.sessions.endStateStop
  if (endState === "interrupted") return t.sessions.endStateInterrupted
  if (endState === "aborted") return t.sessions.endStateAborted
  if (endState === "error") return t.sessions.endStateError
  return t.sessions.endStateOpen
}

/** 2026-09-18 边界收口：长会话列表不得无界直渲——行是重节点（徽标 + 两行元信息 +
 *  操作区），几百条同挂会拖死首屏。渐进披露：默认只出最近的，「显示更早」逐批放开。 */
const FLAT_BATCH = 50
const GROUP_BATCH = 10

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
  const [statusFilter, setStatusFilter] = useState<"all" | "needs_repair" | "archived">("all")
  const [repairingTarget, setRepairingTarget] = useState<string | null>(null)
  const [deletingTarget, setDeletingTarget] = useState<string | null>(null)
  const [unarchivingTarget, setUnarchivingTarget] = useState<string | null>(null)
  // 删除确认（2026-09-08，U9）：统一走模态确认（原为 window.confirm）
  const [pendingDelete, setPendingDelete] = useState<SessionItem | null>(null)
  const [batchRepairing, setBatchRepairing] = useState(false)
  // 复制反馈各用一套状态（id 与 path 可同时处于「已复制」），失败不置位（2026-09-08）
  const { copiedKey: copiedId, copy: copyId } = useCopy()
  const { copiedKey: copiedPath, copy: copyPath } = useCopy()

  // 视图模式：按项目分组 vs 平铺列表
  const [viewMode, setViewMode] = useState<"grouped" | "flat">("grouped")
  const [collapsedProjects, setCollapsedProjects] = useState<Set<string>>(new Set())
  // 渐进披露的放开额度（2026-09-18 边界收口）：平铺一份，分组按项目各一份。
  const [flatLimit, setFlatLimit] = useState(FLAT_BATCH)
  const [groupLimits, setGroupLimits] = useState<Record<string, number>>({})

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

  // 筛选条件一变，"更早"就换了语义（列出的已不是同一批），放开额度必须收回。
  useEffect(() => {
    setFlatLimit(FLAT_BATCH)
    setGroupLimits({})
  }, [searchQuery, statusFilter, viewMode, refreshKey])

  const stats = useMemo(() => {
    if (!sessions)
      return { total: 0, healthy: 0, needsRepair: 0, running: 0, projectsCount: 0, archived: 0 }
    let healthy = 0
    let needsRepair = 0
    let running = 0
    let archived = 0
    const projects = new Set<string>()
    for (const s of sessions) {
      if (s.archived) {
        // 归档会话不计入默认世界统计（默认隐藏，2026-09-07 口径）；
        // 数量单独展示在「已归档」筛选档上。
        archived++
        continue
      }
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
      total: sessions.length - archived,
      healthy,
      needsRepair,
      running,
      projectsCount: projects.size,
      archived,
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
        onNotice?.(res.message || t.sessions.repairFailed, "warn")
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
        onNotice?.(res.message || t.sessions.repairAllFailed, "warn")
      }
    } catch (e) {
      onNotice?.(String(e), "warn")
    } finally {
      setBatchRepairing(false)
    }
  }

  const handleCopyId = async (id: string) => {
    const outcome = await copyId(id, id)
    if (!outcome.ok) onNotice?.(t.error.copyFailed, "warn")
  }

  const handleCopyPath = async (path: string) => {
    const outcome = await copyPath(path, path)
    if (outcome.ok) onNotice?.(t.sessions.pathCopied, "ok")
    else onNotice?.(t.error.copyFailed, "warn")
  }

  const handleOpenWorkspace = async (path: string) => {
    try {
      await api.openExternal(path)
    } catch (e) {
      onNotice?.(t.sessions.openDirFailed(String(e)), "warn")
    }
  }

  const handleDeleteSingle = async (session: SessionItem) => {
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

  // 取消归档（2026-09-15，ADR-0021 路线 A）：经 Host RPC 完成，壳**不直接**改
  // `storages/workspace.json`（那是 dsh 内存状态的投影，运行中 Host 会整体覆盖）。
  // 无活跃 Host 时 Rust 侧返回可读错误，这里原样透出——用户需要知道"要先启动
  // DSH"，而不是遇到一次静默失败。
  const handleUnarchiveSingle = async (session: SessionItem) => {
    setUnarchivingTarget(session.id)
    try {
      await api.unarchiveSession(session.id)
      onNotice?.(t.sessions.unarchiveSuccess, "ok")
      await loadSessions()
    } catch (e) {
      onNotice?.(String(e), "warn")
    } finally {
      setUnarchivingTarget(null)
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
    const inArchiveView = statusFilter === "archived"
    return sessions.filter((s) => {
      // 归档可见性（2026-09-07，对齐 dsh 侧栏默认隐藏口径）：默认视图不含
      // 已归档会话；「已归档」档只看归档。搜索跟随可见性。
      if (inArchiveView !== (s.archived === true)) return false
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
    const isUnarchiving = unarchivingTarget === sess.id
    const isActive = sess.active === true
    // 活跃会话（运行中）不参与「需自愈」：修复会被 dsh 下次 flush 覆盖。
    const isNeedsRepair = sess.status === "needs_repair" && !isActive
    const isHealthy = sess.status === "healthy" && !isActive
    const meta = statusMeta(sess.status, t, Boolean(sess.healthDetail))
    const displayName = sessionDisplayName(sess, t.sessions.noTitle)

    return (
      <motion.div
        key={sess.id}
        initial={{ opacity: 0, y: 4 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.18 }}
        className={`group relative flex flex-col gap-2.5 p-3.5 transition-colors sm:flex-row sm:items-center ${
          isNeedsRepair ? "bg-warn-soft" : "hover:bg-line-soft/30"
        }`}
      >
        {/* 非健康左侧脉冲条 */}
        {isNeedsRepair && (
          <span className="absolute inset-y-1.5 left-0 w-0.5 rounded-full bg-warn animate-pulse" />
        )}

        <div className="min-w-0 flex-1 space-y-1">
          {/* 主行（2026-09-18 收口重排）：状态徽标 + 会话名 + 最后活跃时间 */}
          <div className="flex flex-wrap items-center gap-2">
            {isActive ? (
              <span
                className="inline-flex shrink-0 items-center gap-1.5 rounded-md bg-info-soft px-1.5 py-0.5 text-meta font-medium text-info"
                title={t.sessions.statusRunningDesc}
              >
                <span className="size-1.5 rounded-full bg-info animate-pulse" />
                {t.sessions.statusRunning}
              </span>
            ) : (
              <span
                className={`inline-flex shrink-0 items-center gap-1.5 rounded-md px-1.5 py-0.5 text-meta font-medium ${
                  isNeedsRepair
                    ? "bg-warn-soft text-warn"
                    : isHealthy
                      ? "bg-ok-soft text-ok"
                      : "bg-line text-dim"
                }`}
                title={`${meta.desc}${sess.validator ? t.sessions.validatorHint(sess.validator) : ""}`}
              >
                <span className={`size-1.5 rounded-full ${meta.dot}`} />
                {meta.badge}
              </span>
            )}

            {/* 已归档徽标：仅归档视图可见（默认视图已隐藏） */}
            {sess.archived && (
              <span
                className="flex shrink-0 items-center gap-1 rounded-md bg-alt-soft px-1.5 py-0.5 text-meta font-medium text-alt"
                title={t.sessions.archivedTagDesc}
              >
                <Archive className="size-2.5" />
                {t.sessions.archivedTag}
              </span>
            )}

            {/* 子代理会话：dsh 侧栏隐藏此类，维护工具标注展示 */}
            {sess.subagent && (
              <span
                className="flex shrink-0 items-center gap-1 rounded-md bg-line px-1.5 py-0.5 text-meta font-medium text-dim"
                title={t.sessions.subagentTagDesc}
              >
                <Bot className="size-2.5" />
                {t.sessions.subagentTag}
              </span>
            )}

            <span
              className="truncate text-note font-semibold text-ink"
              title={displayName}
            >
              {displayName}
            </span>

            {/* 时间进主行：它是"最近怎样"的第一问 */}
            <span
              className="inline-flex shrink-0 items-center gap-1 text-label text-faint"
              title={t.sessions.lastUpdated}
            >
              <Clock className="size-3" />
              {formatRelativeTime(sess.updatedAt, t.sessions)}
            </span>

            {sess.hasBackup && (
              <span className="flex shrink-0 items-center gap-1 rounded-md bg-ok-soft px-1.5 py-0.5 text-meta font-medium text-ok">
                <FileArchive className="size-2.5" />
                {t.sessions.backupTag}
              </span>
            )}
          </div>

          {/* 次行（重排分组①·身份）：ID（等宽可复制）/ 项目 / 预设 */}
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-label text-faint">
            <button
              type="button"
              onClick={() => handleCopyId(sess.id)}
              title={t.sessions.copyIdTitle}
              className="inline-flex items-center gap-1 font-mono hover:text-ink transition-colors"
            >
              {sess.id.slice(0, 18)}…
              {copiedId === sess.id ? (
                <Check className="size-3 text-ok" />
              ) : (
                <Copy className="size-2.5 opacity-0 group-hover:opacity-70 transition-opacity" />
              )}
            </button>

            <span className="inline-flex items-center gap-1">
              <Folder className="size-3" />
              <span className="max-w-[220px] truncate font-mono" title={sess.projectName}>
                {sess.projectName}
              </span>
            </span>
            {sess.agentPreset && (
              <span className="font-mono" title={t.sessions.agentPresetTitle}>
                @{sess.agentPreset}
              </span>
            )}

            {/* 次行分组②·体量与收尾：体积 / 创建时间 / 事件数 / 结束状态 */}
            <span aria-hidden className="h-3 w-px shrink-0 bg-line" />
            <span className="inline-flex items-center gap-1">
              <HardDrive className="size-3" />
              {formatBytes(sess.sizeBytes)}
            </span>
            {sess.createdAt ? (
              <span
                className="inline-flex items-center gap-1"
                title={`${t.sessions.createdAtTitle}：${new Date(sess.createdAt).toLocaleString()}`}
              >
                <CalendarClock className="size-3" />
                {formatRelativeTime(sess.createdAt, t.sessions)}
              </span>
            ) : null}
            {sess.eventCount ? (
              <span className="inline-flex items-center gap-1" title={t.sessions.eventCountTitle}>
                <Layers className="size-3" />
                {sess.eventCount}
              </span>
            ) : null}
            {sess.endState && (
              <span
                className="inline-flex items-center gap-1"
                title={endStateLabel(sess.endState, t)}
              >
                {sess.endState === "stop" || sess.endState === "completed" ? (
                  <CheckCircle2 className="size-3 text-ok" />
                ) : sess.endState === "error" ? (
                  <CircleX className="size-3 text-danger" />
                ) : sess.endState === "interrupted" || sess.endState === "aborted" ? (
                  <CircleAlert className="size-3 text-warn" />
                ) : (
                  <HelpCircle className="size-3" />
                )}
                {endStateLabel(sess.endState, t)}
              </span>
            )}
          </div>

          {/* 异常/未知详情（2026-09-08 §18：unknown 也带原因——JSON 不可解析 /
              空文件 / 版本高于本构建，旧实现只在 needs_repair 时展示，用户看不到） */}
          {sess.healthDetail && (
            <p
              className={`line-clamp-1 text-label ${
                isNeedsRepair ? "text-warn" : "text-faint"
              }`}
            >
              {sess.healthDetail}
            </p>
          )}
        </div>

        {/* 操作区：**只放动作**（2026-09-08 批次 D / U14 去重）——运行中/健康/未知
            三种状态原本在左侧徽标与右侧胶囊各显示一次，右侧还挤占修复/删除位。
            状态统一由左侧徽标表达；此处只在「需要修复且非运行中」时给修复按钮。 */}
        <div className="flex shrink-0 items-center gap-1.5 sm:self-center">
          {/* 取消归档（2026-09-15，ADR-0021 路线 A）：仅归档行可见。
              设计上刻意只给「动作」——状态仍由左侧归档徽标表达。 */}
          {sess.archived === true && (
            <Button
              size="sm"
              variant="outline"
              title={t.sessions.unarchiveBtn}
              onClick={() => handleUnarchiveSingle(sess)}
              disabled={isBusy || isDeleting || isUnarchiving || batchRepairing}
              className="gap-1"
            >
              {isUnarchiving ? (
                <LoaderCircle className="size-3 animate-spin" />
              ) : (
                <ArchiveRestore className="size-3" />
              )}
              <span>{t.sessions.unarchiveBtn}</span>
            </Button>
          )}

          {!isActive && isNeedsRepair && (
            <Button
              size="sm"
              variant="outline"
              onClick={() => handleRepairSingle(sess)}
              disabled={isBusy || batchRepairing || isDeleting}
              className="gap-1"
            >
              {isBusy ? (
                <LoaderCircle className="size-3 animate-spin" />
              ) : (
                <Wrench className="size-3" />
              )}
              <span>{t.sessions.repairBtn}</span>
            </Button>
          )}

          {/* 行内动作成对同权重（批次 2）：复制键不再描边——与右侧 ghost 删除键并排时，
              一框一无框会让复制看起来更像主行动。成功态仍靠 text-ok 徽标可见。 */}
          <Button
            size="icon-sm"
            variant="ghost"
            title={t.sessions.copyPath}
            onClick={() => handleCopyPath(sess.filePath)}
          >
            {copiedPath === sess.filePath ? (
              <Check className="size-3 text-ok" />
            ) : (
              <Copy className="size-3 text-faint" />
            )}
          </Button>

          <Button
            size="icon-sm"
            variant="destructive-ghost"
            title={t.sessions.deleteBtn}
            onClick={() => setPendingDelete(sess)}
            disabled={isBusy || batchRepairing || isDeleting}
          >
            {isDeleting ? (
              <LoaderCircle className="size-3 animate-spin" />
            ) : (
              <Trash2 className="size-3" />
            )}
          </Button>
        </div>
      </motion.div>
    )
  }

  return (
    <div className="space-y-4">
      {error && (
        <StateBlock
          tone="error"
          title={error}
          action={
            <Button size="sm" variant="outline" onClick={loadSessions} className="gap-1">
              <RefreshCw className="size-3" />
              {t.profiles.retryLoad}
            </Button>
          }
        />
      )}

      {/* 顶部状态与操作条 */}
      <div className="rounded-2xl border border-line bg-panel p-4 shadow-xs">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <div className="flex size-10 items-center justify-center rounded-xl bg-wash text-brand-deep">
              <ShieldCheck className="size-5" />
            </div>
            <div>
              <h2 className="text-sm font-semibold text-ink">{t.sessions.title}</h2>
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
              <RefreshCw className={`size-3.5 ${loading ? "animate-spin text-brand-deep" : "text-dim"}`} />
              <span>{t.sessions.scanBtn}</span>
            </Button>

            {/* 全局修复（2026-09-18 收口）：唯一的建设性主动作回到 default Button 的
                brand 权重；此前用 bg-warn 实底，危险动作视觉最弱、建设动作最重。 */}
            <Button
              size="sm"
              onClick={handleRepairAll}
              disabled={batchRepairing || loading || !sessions || stats.needsRepair === 0}
              title={stats.needsRepair === 0 ? t.sessions.repairAllDisabled : t.sessions.repairNeedHint}
              className="gap-1.5 text-xs"
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

        {/* 统计指标（2026-09-18 收口）：三张卡此前各自染色（info/warn 半透明底 +
            彩色描边），统一回中性 bg-bg，语义由色点与数字颜色承载。 */}
        <div className="mt-4 grid grid-cols-2 gap-2.5 sm:grid-cols-5">
          <div className="rounded-xl border border-line bg-bg p-3">
            <span className="text-label text-faint">{t.sessions.statProjects}</span>
            <div className="mt-1 font-mono text-base font-bold text-ink">{stats.projectsCount}</div>
          </div>
          <div className="rounded-xl border border-line bg-bg p-3">
            <span className="text-label text-faint">{t.sessions.statTotal}</span>
            <div className="mt-1 font-mono text-base font-bold text-ink">{stats.total}</div>
          </div>
          <div className="rounded-xl border border-line bg-bg p-3">
            <span className="flex items-center gap-1 text-label text-ok">
              <span className="size-1.5 rounded-full bg-ok" />
              {t.sessions.statHealthy}
            </span>
            <div className="mt-1 font-mono text-base font-bold text-ok">
              {stats.healthy}
            </div>
          </div>
          <div className="rounded-xl border border-line bg-bg p-3">
            <span className="flex items-center gap-1 text-label text-info">
              <span className="size-1.5 rounded-full bg-info animate-pulse" />
              {t.sessions.statRunning}
            </span>
            <div className="mt-1 font-mono text-base font-bold text-info">
              {stats.running}
            </div>
          </div>
          <div className="rounded-xl border border-line bg-bg p-3">
            <span className="flex items-center gap-1 text-label text-warn">
              <span className="size-1.5 rounded-full bg-warn animate-pulse" />
              {t.sessions.statNeedsRepair}
            </span>
            <div className="mt-1 font-mono text-base font-bold text-warn">
              {stats.needsRepair}
            </div>
          </div>
        </div>
      </div>

      {/* 搜索与视图切换 */}
      <div className="flex flex-wrap items-center justify-between gap-2.5">
        <div className="relative min-w-[220px] flex-1">
          <Search className="text-faint absolute inset-y-0 left-3 my-auto size-3.5" />
          <input
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={t.sessions.searchPlaceholder}
            aria-label={t.sessions.searchPlaceholder}
            className="border-line bg-panel text-ink placeholder:text-faint focus:border-brand w-full rounded-xl border py-1.5 pr-3 pl-8.5 font-mono text-xs outline-none transition-colors shadow-2xs"
          />
        </div>

        {/* 状态筛选：全部 / 仅看异常 / 已归档（2026-09-18 收口：手搓分段器 → Segmented，
            计数徽标随 label 走） */}
        <Segmented<"all" | "needs_repair" | "archived">
          size="sm"
          ariaLabel={t.sessions.title}
          value={statusFilter}
          onChange={setStatusFilter}
          options={[
            { value: "all", label: t.sessions.filterAll, icon: Layers },
            {
              value: "needs_repair",
              icon: AlertTriangle,
              label: (
                <>
                  {t.sessions.filterNeedsRepair}
                  {stats.needsRepair > 0 && (
                    <span className="ml-1 flex h-4 min-w-4 items-center justify-center rounded-full bg-warn-soft px-1 font-mono text-meta text-warn">
                      {stats.needsRepair}
                    </span>
                  )}
                </>
              ),
            },
            {
              value: "archived",
              icon: Archive,
              label: (
                <>
                  {t.sessions.filterArchived}
                  {stats.archived > 0 && (
                    <span className="ml-1 flex h-4 min-w-4 items-center justify-center rounded-full bg-alt-soft px-1 font-mono text-meta text-alt">
                      {stats.archived}
                    </span>
                  )}
                </>
              ),
            },
          ]}
        />

        <Segmented<"grouped" | "flat">
          size="sm"
          ariaLabel={t.sessions.title}
          value={viewMode}
          onChange={setViewMode}
          className="hidden sm:flex"
          options={[
            { value: "grouped", label: t.sessions.groupByProject, icon: Layers },
            { value: "flat", label: t.sessions.viewFlat, icon: List },
          ]}
        />
      </div>

      {/* 会话列表呈现 */}
      {loading && !sessions ? (
        <StateBlock tone="loading" title={t.sessions.loadingScanning} />
      ) : filteredSessions.length === 0 ? (
        <StateBlock
          tone="empty"
          icon={FileCode}
          title={
            searchQuery || statusFilter !== "all"
              ? t.sessions.emptyFilter
              : t.sessions.emptyList
          }
        />
      ) : viewMode === "grouped" ? (
        <div className="space-y-3.5">
          {projectGroups.map(([rawKey, group]) => {
            const isCollapsed = collapsedProjects.has(rawKey)
            const limit = groupLimits[rawKey] ?? GROUP_BATCH
            const visibleItems = group.items.slice(0, limit)
            const restCount = group.items.length - visibleItems.length
            return (
              <div
                key={rawKey}
                className="overflow-hidden rounded-2xl border border-line bg-panel shadow-xs transition-colors hover:border-brand/30"
              >
                <div className="border-b border-line bg-line-soft/40 px-4 py-3 space-y-1.5">
                  <div className="flex items-center justify-between gap-3">
                    {/* 可点折叠头：div[onClick] → 真 button（键盘可达，2026-09-18 收口） */}
                    <button
                      type="button"
                      onClick={() => toggleProjectCollapse(rawKey)}
                      aria-expanded={!isCollapsed}
                      className="flex min-w-0 flex-1 cursor-pointer items-center gap-2 select-none text-left"
                    >
                      {isCollapsed ? (
                        <ChevronRight className="size-4 text-faint shrink-0" />
                      ) : (
                        <ChevronDown className="size-4 text-faint shrink-0" />
                      )}
                      <Folder className="size-4 text-brand-deep shrink-0" />
                      <span
                        className="font-mono text-xs font-semibold text-ink truncate"
                        title={group.projectName}
                      >
                        {group.projectName}
                      </span>
                      <span className="shrink-0 rounded-md bg-line px-1.5 py-0.5 text-meta text-faint tabular-nums">
                        {t.sessions.groupMeta(group.items.length, formatBytes(group.totalBytes))}
                      </span>
                    </button>

                    <Button
                      size="sm"
                      variant="outline"
                      title={t.sessions.openInFinder}
                      onClick={() => handleOpenWorkspace(group.decodedPath)}
                      className="h-7 shrink-0 gap-1.5 px-2.5 text-xs hover:border-brand hover:text-brand-deep"
                    >
                      <ExternalLink className="size-3" />
                      <span>{t.sessions.openInFinder}</span>
                    </Button>
                  </div>

                  <div className="pl-6">
                    <span
                      className="font-mono text-label text-faint block truncate"
                      title={group.decodedPath}
                    >
                      {group.decodedPath}
                    </span>
                  </div>
                </div>

                {!isCollapsed && (
                  <div className="divide-y divide-line/60">
                    {visibleItems.map((sess) => renderSessionRow(sess))}
                    {restCount > 0 && (
                      <div className="p-1.5">
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() =>
                            setGroupLimits((prev) => ({
                              ...prev,
                              [rawKey]: (prev[rawKey] ?? GROUP_BATCH) + GROUP_BATCH,
                            }))
                          }
                          className="w-full text-xs text-dim"
                        >
                          {t.sessions.showMoreSessions(restCount)}
                        </Button>
                      </div>
                    )}
                  </div>
                )}
              </div>
            )
          })}
        </div>
      ) : (
        <div className="divide-y divide-line rounded-2xl border border-line bg-panel shadow-xs overflow-hidden">
          {filteredSessions.slice(0, flatLimit).map((sess) => renderSessionRow(sess))}
          {filteredSessions.length > flatLimit && (
            <div className="p-1.5">
              <Button
                size="sm"
                variant="ghost"
                onClick={() => setFlatLimit((n) => n + FLAT_BATCH)}
                className="w-full text-xs text-dim"
              >
                {t.sessions.showMoreSessions(filteredSessions.length - flatLimit)}
              </Button>
            </div>
          )}
        </div>
      )}

      {/* 删除确认（U9）：不可撤销，先列明后果 */}
      <ConfirmDialog
        open={pendingDelete !== null}
        title={pendingDelete ? t.sessions.deleteConfirmTitle(pendingDelete.id) : ""}
        note={t.sessions.deleteConfirmNote}
        confirmLabel={t.sessions.deleteBtn}
        cancelLabel={t.confirm.cancel}
        onConfirm={() => {
          const target = pendingDelete
          setPendingDelete(null)
          if (target) void handleDeleteSingle(target)
        }}
        onClose={() => setPendingDelete(null)}
      />
    </div>
  )
}
