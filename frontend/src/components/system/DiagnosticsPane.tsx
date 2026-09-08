// DiagnosticsPane.tsx —— 运行环境健康诊断与存储大盘（4.11）。
import { useCallback, useEffect, useState } from "react"
import {
  AlertTriangle,
  Check,
  Copy,
  Cpu,
  Database,
  HardDrive,
  LoaderCircle,
  Package,
  RefreshCw,
  Server,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { useCopy } from "@/hooks/useCopy"
import { useI18n } from "@/stores/i18nStore"
import { Button } from "@/components/ui/button"
import type { SystemDiagnosticsReport } from "@/types/ipc"

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B"
  const k = 1024
  const sizes = ["B", "KB", "MB", "GB", "TB"]
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`
}

let cachedReport: SystemDiagnosticsReport | null = null
let lastFetchedAt = 0
const CACHE_TTL_MS = 60_000

export function DiagnosticsPane({
  onNotice,
}: {
  onNotice: (msg: string, kind?: "ok" | "warn") => void
}) {
  const { t } = useI18n()
  const [report, setReport] = useState<SystemDiagnosticsReport | null>(cachedReport)
  const [loading, setLoading] = useState(
    !cachedReport || Date.now() - lastFetchedAt >= CACHE_TTL_MS,
  )
  const [error, setError] = useState<string | null>(null)
  const { copied, copy } = useCopy()

  const fetchDiagnostics = useCallback(
    (force = false) => {
      if (
        !force &&
        cachedReport &&
        Date.now() - lastFetchedAt < CACHE_TTL_MS
      ) {
        setReport(cachedReport)
        setLoading(false)
        return
      }

      setLoading(true)
      setError(null)
      api
        .getSystemDiagnostics()
        .then((r) => {
          cachedReport = r
          lastFetchedAt = Date.now()
          setReport(r)
        })
        .catch((e) => {
          // 2026-09-08 裁定：采集失败**不得**降级成「全缺失」报告——那会让用户
          // 看到一份伪造的环境损坏结论（Node/pnpm/dsh 全部未检出）。
          setError(String(e))
          onNotice(String(e), "warn")
        })
        .finally(() => setLoading(false))
    },
    [onNotice],
  )

  useEffect(() => {
    fetchDiagnostics(false)
  }, [fetchDiagnostics])

  const copyFullReport = async () => {
    if (!report) return
    // 2026-09-08：写失败不再静默（原来只挂 .then 成功分支）
    const outcome = await copy(JSON.stringify(report, null, 2))
    if (outcome.ok) onNotice(t.console.reportCopied, "ok")
    else onNotice(t.error.copyFailed, "warn")
  }

  if (loading && !report) {
    return (
      <div className="flex h-64 flex-col items-center justify-center text-xs text-faint">
        <LoaderCircle className="mb-2 size-5 animate-spin text-brand" />
        <span>{t.console.diagnosticsSubtitle}…</span>
      </div>
    )
  }

  // 采集失败且无缓存 → 报错块，绝不渲染伪造的「全部未检出」报告
  if (!report) {
    return (
      <div className="flex h-64 flex-col items-center justify-center gap-3 px-6 text-center">
        <AlertTriangle className="size-5 text-warn" />
        <p className="max-w-md text-xs text-dim">
          {t.console.diagnosticsLoadFailed}
          <br />
          <span className="font-mono text-xs break-all text-faint">{error}</span>
        </p>
        <Button
          size="sm"
          variant="outline"
          onClick={() => fetchDiagnostics(true)}
          className="gap-1 text-xs"
        >
          <RefreshCw className="size-3" />
          <span>{t.console.retryLoad}</span>
        </Button>
      </div>
    )
  }

  const { node, pnpm, dsh, storage, platform } = report

  const profilesPercent =
    storage.totalBytes > 0
      ? Math.min(100, Math.round((storage.profilesBytes / storage.totalBytes) * 100))
      : 0
  const sessionsPercent =
    storage.totalBytes > 0
      ? Math.min(100, Math.round((storage.sessionsBytes / storage.totalBytes) * 100))
      : 0
  const otherPercent = Math.max(0, 100 - profilesPercent - sessionsPercent)

  return (
    <div className="space-y-5">
      {/* 顶部工具栏 */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h2 className="text-sm font-bold text-ink">
            {t.console.diagnosticsTitle}
          </h2>
          <p className="text-xs text-faint">{t.console.diagnosticsSubtitle}</p>
        </div>

        <div className="flex items-center gap-2">
          <Button
            size="sm"
            variant="outline"
            onClick={copyFullReport}
            className="gap-1 text-xs"
          >
            {copied ? (
              <Check className="size-3.5 text-ok" />
            ) : (
              <Copy className="size-3.5" />
            )}
            <span>{copied ? "已复制" : t.console.copyReport}</span>
          </Button>

          <Button
            size="sm"
            variant="outline"
            onClick={() => fetchDiagnostics(true)}
            disabled={loading}
            className="gap-1 text-xs"
          >
            <RefreshCw
              className={`size-3.5 ${loading ? "animate-spin text-brand" : ""}`}
            />
            <span>{t.console.refreshDiagnostics}</span>
          </Button>
        </div>
      </div>

      {/* 四大指标卡片 Grid */}
      <div className="grid grid-cols-1 gap-3.5 sm:grid-cols-2 lg:grid-cols-4">
        {/* 1. Node.js 运行时 */}
        <div className="flex flex-col justify-between rounded-2xl border border-line bg-panel p-4 shadow-2xs">
          <div>
            <div className="flex items-center justify-between">
              <div className="flex size-7 items-center justify-center rounded-lg bg-emerald-500/10 text-emerald-600">
                <Cpu className="size-4" />
              </div>
              <span
                className={`rounded-full px-2 py-0.5 text-meta font-medium leading-none ${
                  node.isReady
                    ? "bg-ok-soft text-ok font-semibold"
                    : "bg-warn-soft text-warn font-semibold"
                }`}
              >
                {node.isReady ? t.console.statusReady : t.console.statusMissing}
              </span>
            </div>
            <h3 className="mt-2.5 text-xs font-semibold text-ink">

            </h3>
            <p className="mt-0.5 font-mono text-xs font-bold text-ink">
              {node.version || "未检出"}
            </p>
          </div>
          <div className="mt-3 truncate border-t border-line/60 pt-2 font-mono text-meta text-faint">
            <span title={node.path}>{node.path || "无路径"}</span>
          </div>
        </div>

        {/* 2. pnpm 包管理 */}
        <div className="flex flex-col justify-between rounded-2xl border border-line bg-panel p-4 shadow-2xs">
          <div>
            <div className="flex items-center justify-between">
              <div className="flex size-7 items-center justify-center rounded-lg bg-amber-500/10 text-amber-600">
                <Package className="size-4" />
              </div>
              <span
                className={`rounded-full px-2 py-0.5 text-meta font-medium leading-none ${
                  pnpm.isReady
                    ? "bg-ok-soft text-ok font-semibold"
                    : "bg-warn-soft text-warn font-semibold"
                }`}
              >
                {pnpm.isReady ? t.console.statusReady : t.console.statusMissing}
              </span>
            </div>
            <h3 className="mt-2.5 text-xs font-semibold text-ink">

            </h3>
            <p className="mt-0.5 font-mono text-xs font-bold text-ink">
              {pnpm.isReady ? (pnpm.version ? `v${pnpm.version}` : "已全局就绪") : "缺失"}
            </p>
          </div>
          <div className="mt-3 truncate border-t border-line/60 pt-2 font-mono text-meta text-faint">
            <span title={pnpm.path}>{pnpm.path || "无路径"}</span>
          </div>
        </div>

        {/* 3. DSH 核心引擎 */}
        <div className="flex flex-col justify-between rounded-2xl border border-line bg-panel p-4 shadow-2xs">
          <div>
            <div className="flex items-center justify-between">
              <div className="flex size-7 items-center justify-center rounded-lg bg-brand/10 text-brand">
                <Server className="size-4" />
              </div>
              <span
                className={`rounded-full px-2 py-0.5 text-meta font-medium leading-none ${
                  dsh.isReady
                    ? "bg-ok-soft text-ok font-semibold"
                    : "bg-warn-soft text-warn font-semibold"
                }`}
              >
                {dsh.isReady ? t.console.statusReady : t.console.statusMissing}
              </span>
            </div>
            <h3 className="mt-2.5 text-xs font-semibold text-ink">

            </h3>
            <p className="mt-0.5 font-mono text-xs font-bold text-ink">
              {dsh.isReady ? (dsh.version ? `v${dsh.version}` : "官方源 (已就绪)") : "官方源 (未检出)"}
            </p>
          </div>
          <div className="mt-3 truncate border-t border-line/60 pt-2 font-mono text-meta text-faint">
            <span title={dsh.path}>{dsh.path || "无路径"}</span>
          </div>
        </div>

        {/* 4. 存储总览 */}
        <div className="flex flex-col justify-between rounded-2xl border border-line bg-panel p-4 shadow-2xs">
          <div>
            <div className="flex items-center justify-between">
              <div className="flex size-7 items-center justify-center rounded-lg bg-indigo-500/10 text-indigo-600">
                <HardDrive className="size-4" />
              </div>
              <span className="rounded-full bg-line px-2 py-0.5 font-mono text-meta text-faint">
                {platform.os} ({platform.arch})
              </span>
            </div>
            <h3 className="mt-2.5 text-xs font-semibold text-ink">

            </h3>
            <p className="mt-0.5 font-mono text-xs font-bold text-ink">
              {formatBytes(storage.totalBytes)}
            </p>
          </div>
          <div className="mt-3 truncate border-t border-line/60 pt-2 font-mono text-meta text-faint">
            <span title={storage.dshHome}>{storage.dshHome || "~/.dsh"}</span>
          </div>
        </div>
      </div>

      {/* 详细存储分布面板 */}
      <section className="rounded-2xl border border-line bg-panel p-5 shadow-2xs">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <Database className="size-4 text-brand" />
            <h3 className="text-xs font-semibold text-ink">
              DSH_HOME 存储空间分布
            </h3>
          </div>
          <span className="font-mono text-xs font-medium text-dim">
            {t.console.totalUsage(formatBytes(storage.totalBytes))}
          </span>
        </div>

        {/* 彩色占比条 */}
        <div className="mt-3.5 flex h-3 w-full overflow-hidden rounded-full bg-bg">
          <div
            style={{ width: `${profilesPercent}%` }}
            title={`Profiles: ${formatBytes(storage.profilesBytes)} (${profilesPercent}%)`}
            className="bg-brand transition-all"
          />
          <div
            style={{ width: `${sessionsPercent}%` }}
            title={`Sessions: ${formatBytes(storage.sessionsBytes)} (${sessionsPercent}%)`}
            className="bg-emerald-500 transition-all"
          />
          <div
            style={{ width: `${otherPercent}%` }}
            title={`Cache & Other: ${otherPercent}%`}
            className="bg-line-soft transition-all"
          />
        </div>

        {/* 分布图例 */}
        <div className="mt-4 grid grid-cols-1 gap-3 text-xs sm:grid-cols-3">
          <div className="flex items-center gap-2.5 rounded-xl border border-line bg-bg p-2.5">
            <span className="size-2.5 rounded-full bg-brand shrink-0" />
            <div className="min-w-0">
              <span className="text-ink font-medium">Profile 工作台</span>
              <p className="font-mono text-label text-faint truncate">
                {t.console.profilesUsage(
                  storage.profilesCount,
                  formatBytes(storage.profilesBytes),
                )}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2.5 rounded-xl border border-line bg-bg p-2.5">
            <span className="size-2.5 rounded-full bg-emerald-500 shrink-0" />
            <div className="min-w-0">
              <span className="text-ink font-medium">会话数据 (Sessions)</span>
              <p className="font-mono text-label text-faint truncate">
                {t.console.sessionsUsage(
                  storage.sessionsCount,
                  formatBytes(storage.sessionsBytes),
                )}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2.5 rounded-xl border border-line bg-bg p-2.5">
            <span className="size-2.5 rounded-full bg-line-soft shrink-0" />
            <div className="min-w-0">
              <span className="text-ink font-medium">系统缓存与其他</span>
              <p className="font-mono text-label text-faint truncate">
                {formatBytes(
                  Math.max(
                    0,
                    storage.totalBytes -
                      storage.profilesBytes -
                      storage.sessionsBytes,
                  ),
                )}
              </p>
            </div>
          </div>
        </div>
      </section>
    </div>
  )
}
