// 会话维护与自愈工作台（Session Doctor & Health Manager）。
// 提供 DSH 会话日志的健康体检、乱序检测与一键自愈修复。
import { useEffect, useMemo, useState } from "react"
import {
  AlertTriangle,
  Archive,
  Check,
  Clock,
  Copy,
  FileCode,
  HardDrive,
  LoaderCircle,
  RefreshCw,
  Search,
  ShieldCheck,
  Sparkles,
  Wrench,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { t } from "@/content/zh-CN"
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

export function SessionManager({
  refreshKey,
  onNotice,
}: {
  refreshKey: number
  onNotice?: (text: string, kind?: "ok" | "warn") => void
}) {
  const [sessions, setSessions] = useState<SessionItem[] | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [searchQuery, setSearchQuery] = useState("")
  const [repairingTarget, setRepairingTarget] = useState<string | null>(null)
  const [batchRepairing, setBatchRepairing] = useState(false)
  const [copiedId, setCopiedId] = useState<string | null>(null)

  const loadSessions = async () => {
    setLoading(true)
    setError(null)
    try {
      const list = await api.listSessions()
      setSessions(list)
    } catch (e) {
      setError(String(e))
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    void loadSessions()
  }, [refreshKey])

  // 单会话修复
  const handleRepairSingle = async (session: SessionItem) => {
    setRepairingTarget(session.id)
    try {
      const res = await api.repairSession(session.filePath)
      if (res.success) {
        onNotice?.(t.sessions.repairSuccess(session.id), "ok")
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

  // 全量体检与自愈修复
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
    navigator.clipboard.writeText(id)
    setCopiedId(id)
    setTimeout(() => setCopiedId((curr) => (curr === id ? null : curr)), 2000)
  }

  const filteredSessions = useMemo(() => {
    if (!sessions) return []
    if (!searchQuery.trim()) return sessions
    const q = searchQuery.toLowerCase().trim()
    return sessions.filter(
      (s) =>
        s.id.toLowerCase().includes(q) ||
        s.projectName.toLowerCase().includes(q) ||
        s.projectDirRaw.toLowerCase().includes(q),
    )
  }, [sessions, searchQuery])

  return (
    <div className="space-y-4">
      {/* 顶部状态与操作条 */}
      <div className="rounded-2xl border border-line bg-panel p-4 shadow-xs">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <div className="flex size-10 items-center justify-center rounded-xl bg-brand/10 text-brand">
              <ShieldCheck className="size-5" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h2 className="text-ink text-sm font-semibold">
                  {t.sessions.title}
                </h2>
                {sessions && (
                  <span className="rounded-full bg-line-soft px-2 py-0.5 text-xs text-dim">
                    {t.sessions.totalCount(sessions.length)}
                  </span>
                )}
              </div>
              <p className="text-faint text-xs">{t.sessions.subtitle}</p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <Button
              size="sm"
              variant="outline"
              onClick={loadSessions}
              disabled={loading}
              className="gap-1.5 text-xs"
            >
              <RefreshCw className={`size-3.5 ${loading ? "animate-spin text-brand" : ""}`} />
              <span>{t.sessions.scanBtn}</span>
            </Button>

            <Button
              size="sm"
              onClick={handleRepairAll}
              disabled={batchRepairing || loading || !sessions || sessions.length === 0}
              className="gap-1.5 text-xs bg-brand text-white shadow-xs hover:bg-brand/90"
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

        {/* 搜索过滤框 */}
        <div className="mt-3 pt-3 border-t border-line/60">
          <div className="relative">
            <Search className="text-faint absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2" />
            <input
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t.sessions.searchPlaceholder}
              className="border-line bg-panel text-ink placeholder:text-faint focus:border-brand w-full rounded-xl border py-1.5 pr-3 pl-8 text-xs outline-none shadow-2xs transition-colors"
            />
          </div>
        </div>
      </div>

      {/* 异常错误提示 */}
      {error && (
        <div className="flex items-center gap-2 rounded-xl border border-danger/30 bg-danger/5 p-4 text-xs text-danger">
          <AlertTriangle className="size-4 shrink-0" />
          <span>{error}</span>
        </div>
      )}

      {/* 会话列表 */}
      {loading && !sessions ? (
        <div className="flex min-h-48 items-center justify-center rounded-2xl border border-dashed border-line bg-panel/50 p-8 text-dim">
          <div className="flex flex-col items-center gap-2">
            <LoaderCircle className="size-6 animate-spin text-brand" />
            <span className="text-xs">{t.profiles.refreshing}</span>
          </div>
        </div>
      ) : filteredSessions.length === 0 ? (
        <div className="flex min-h-48 flex-col items-center justify-center rounded-2xl border border-dashed border-line bg-panel/50 p-8 text-center">
          <FileCode className="text-faint mb-2 size-8" />
          <p className="text-ink text-xs font-medium">
            {searchQuery ? t.sessions.emptyFilter : t.sessions.emptyList}
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-2.5">
          {filteredSessions.map((sess) => {
            const isBusy = repairingTarget === sess.id
            return (
              <div
                key={sess.id}
                className="group flex flex-col justify-between gap-3 rounded-xl border border-line bg-panel p-3.5 shadow-2xs transition-all hover:border-brand/40 hover:shadow-xs sm:flex-row sm:items-center"
              >
                {/* 会话信息主体 */}
                <div className="flex min-w-0 items-start gap-3">
                  <div className="mt-0.5 flex size-8 shrink-0 items-center justify-center rounded-lg bg-line-soft text-dim group-hover:bg-brand/10 group-hover:text-brand transition-colors">
                    <FileCode className="size-4" />
                  </div>

                  <div className="min-w-0 space-y-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="text-ink font-mono text-xs font-semibold truncate max-w-sm">
                        {sess.id}
                      </span>
                      <button
                        type="button"
                        title="复制会话 ID"
                        onClick={() => handleCopyId(sess.id)}
                        className="text-faint hover:text-ink transition-colors p-0.5 rounded"
                      >
                        {copiedId === sess.id ? (
                          <Check className="size-3 text-brand" />
                        ) : (
                          <Copy className="size-3" />
                        )}
                      </button>

                      {/* 项目标签 */}
                      <span className="rounded-md bg-line-soft px-1.5 py-0.5 text-[11px] font-medium text-dim">
                        📁 {sess.projectName}
                      </span>

                      {/* 压缩状态 */}
                      <span className="rounded-md border border-line px-1.5 py-0.5 text-[10px] text-faint">
                        {sess.isCompressed ? t.sessions.compressedTag : t.sessions.plainTag}
                      </span>

                      {/* 备份标识 */}
                      {sess.hasBackup && (
                        <span className="flex items-center gap-1 rounded-md bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-medium text-emerald-600 dark:text-emerald-400">
                          <Archive className="size-2.5" />
                          {t.sessions.backupTag}
                        </span>
                      )}
                    </div>

                    {/* 辅助元数据 */}
                    <div className="flex flex-wrap items-center gap-3 text-[11px] text-faint">
                      <span className="flex items-center gap-1">
                        <Clock className="size-3" />
                        {t.sessions.lastUpdated}: {formatRelativeTime(sess.updatedAt)}
                      </span>
                      <span className="flex items-center gap-1">
                        <HardDrive className="size-3" />
                        {t.sessions.fileSize}: {formatBytes(sess.sizeBytes)}
                      </span>
                    </div>
                  </div>
                </div>

                {/* 右侧动作 */}
                <div className="flex shrink-0 items-center gap-2 sm:self-center">
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => handleRepairSingle(sess)}
                    disabled={isBusy || batchRepairing}
                    className="gap-1.5 text-xs hover:border-brand hover:text-brand"
                  >
                    {isBusy ? (
                      <LoaderCircle className="size-3 animate-spin text-brand" />
                    ) : (
                      <Wrench className="size-3 text-dim" />
                    )}
                    <span>{t.sessions.repairBtn}</span>
                  </Button>
                </div>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}
