// LogViewerPane.tsx —— 极客暗色终端风格的实时日志查看器（4.11）。
import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import {
  ArrowDown,
  Check,
  Copy,
  LoaderCircle,
  RefreshCw,
  Search,
  Terminal,
  Trash2,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { useI18n } from "@/stores/i18nStore"
import { localizeLogTimestamp } from "@/lib/format"
import { Button } from "@/components/ui/button"
import { Switch } from "@/components/ui/switch"
import type { LogQueryResult } from "@/types/ipc"

type LogSourceKey = "shell" | "dsh" | "session_repair"

export function LogViewerPane({
  onNotice,
}: {
  onNotice: (msg: string, kind?: "ok" | "warn") => void
}) {
  const { t } = useI18n()
  const [source, setSource] = useState<LogSourceKey>("shell")
  const [logData, setLogData] = useState<LogQueryResult | null>(null)
  const [loading, setLoading] = useState(false)
  const [searchFilter, setSearchFilter] = useState("")
  const [autoScroll, setAutoScroll] = useState(true)
  const [copied, setCopied] = useState(false)

  const terminalRef = useRef<HTMLDivElement>(null)

  const fetchLogs = useCallback(
    (src: LogSourceKey) => {
      setLoading(true)
      api
        .getAppLogs(src, 600)
        .then((res) => {
          setLogData(res)
        })
        .catch((e) => onNotice(String(e), "warn"))
        .finally(() => setLoading(false))
    },
    [onNotice],
  )

  useEffect(() => {
    fetchLogs(source)
  }, [source, fetchLogs])

  // 自动滚到底部
  useEffect(() => {
    if (autoScroll && terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight
    }
  }, [logData, autoScroll])

  const copyLogs = () => {
    if (!logData?.lines.length) return
    const text = logData.lines.join("\n")
    void navigator.clipboard.writeText(text).then(() => {
      setCopied(true)
      onNotice(t.console.logsCopied, "ok")
      setTimeout(() => setCopied(false), 2000)
    })
  }

  const clearScreen = () => {
    setLogData((prev) =>
      prev ? { ...prev, lines: ["// 屏幕已清空（日志原文件不受影响）"] } : null,
    )
  }

  // 过滤后的日志行
  const filteredLines = useMemo(() => {
    if (!logData?.lines) return []
    if (!searchFilter.trim()) return logData.lines
    const q = searchFilter.toLowerCase().trim()
    return logData.lines.filter((l) => l.toLowerCase().includes(q))
  }, [logData, searchFilter])

  return (
    <div className="flex flex-col space-y-3.5">
      {/* 顶部控制栏 */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        {/* 日志源 Segmented Tabs */}
        <div
          role="tablist"
          className="flex rounded-xl border border-line bg-line-soft/80 p-0.5"
        >
          <button
            type="button"
            role="tab"
            aria-selected={source === "shell"}
            onClick={() => setSource("shell")}
            className={`rounded-lg px-3 py-1.5 text-xs font-medium transition-all ${
              source === "shell"
                ? "bg-panel text-ink shadow-xs"
                : "text-dim hover:text-ink"
            }`}
          >
            {t.console.sourceShell}
          </button>

          <button
            type="button"
            role="tab"
            aria-selected={source === "dsh"}
            onClick={() => setSource("dsh")}
            className={`rounded-lg px-3 py-1.5 text-xs font-medium transition-all ${
              source === "dsh"
                ? "bg-panel text-ink shadow-xs"
                : "text-dim hover:text-ink"
            }`}
          >
            {t.console.sourceDsh}
          </button>

          <button
            type="button"
            role="tab"
            aria-selected={source === "session_repair"}
            onClick={() => setSource("session_repair")}
            className={`rounded-lg px-3 py-1.5 text-xs font-medium transition-all ${
              source === "session_repair"
                ? "bg-panel text-ink shadow-xs"
                : "text-dim hover:text-ink"
            }`}
          >
            {t.console.sourceRepair}
          </button>
        </div>

        {/* 快速动作栏 */}
        <div className="flex items-center gap-2">
          {/* 自动滚底开关 */}
          <div className="flex items-center gap-1.5 rounded-lg border border-line bg-panel px-2.5 py-1 text-xs text-dim">
            <ArrowDown className="size-3 text-faint" />
            <span className="text-[11px]">{t.console.autoScroll}</span>
            <Switch
              checked={autoScroll}
              onCheckedChange={setAutoScroll}
              className="scale-75"
            />
          </div>

          <Button
            size="sm"
            variant="outline"
            onClick={copyLogs}
            className="gap-1 text-xs"
          >
            {copied ? (
              <Check className="size-3.5 text-ok" />
            ) : (
              <Copy className="size-3.5" />
            )}
            <span>{copied ? "已复制" : t.console.copyLogs}</span>
          </Button>

          <Button
            size="sm"
            variant="outline"
            title={t.console.clearLogs}
            onClick={clearScreen}
            className="size-8 p-0"
          >
            <Trash2 className="size-3.5 text-faint" />
          </Button>

          <Button
            size="sm"
            variant="outline"
            disabled={loading}
            onClick={() => fetchLogs(source)}
            className="size-8 p-0"
          >
            <RefreshCw
              className={`size-3.5 ${loading ? "animate-spin text-brand" : ""}`}
            />
          </Button>
        </div>
      </div>

      {/* 搜索与元信息栏 */}
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="relative min-w-[200px] flex-1 max-w-sm">
          <Search className="text-faint absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2" />
          <input
            value={searchFilter}
            onChange={(e) => setSearchFilter(e.target.value)}
            placeholder={t.console.searchPlaceholder}
            className="border-line bg-panel text-ink placeholder:text-faint focus:border-brand w-full rounded-xl border py-1.5 pr-3 pl-8 font-mono text-xs outline-none shadow-2xs transition-colors"
          />
        </div>

        <div className="flex items-center gap-2 font-mono text-[11px] text-faint">
          <span>{logData ? t.console.totalLines(logData.totalLines) : ""}</span>
          {logData?.truncated && <span>· {t.console.truncatedHint}</span>}
        </div>
      </div>

      {/* 极客暗色终端日志面板 */}
      <div className="relative flex flex-col overflow-hidden rounded-2xl border border-line bg-slate-950 shadow-md">
        {/* 终端顶栏 */}
        <div className="flex items-center justify-between border-b border-slate-800 bg-slate-900/90 px-4 py-2 text-xs">
          <div className="flex items-center gap-2 font-mono text-slate-300">
            <Terminal className="size-3.5 text-brand" />
            <span className="font-semibold">{logData?.source || source}</span>
          </div>
          <span className="font-mono text-[10px] text-slate-500 truncate max-w-xs">
            {logData?.path}
          </span>
        </div>

        {/* 日志内容滚动区 */}
        <div
          ref={terminalRef}
          className="h-[460px] overflow-y-auto p-4 font-mono text-xs leading-relaxed selection:bg-brand/30"
        >
          {loading && !logData ? (
            <div className="flex h-full items-center justify-center text-slate-500">
              <LoaderCircle className="mr-2 size-4 animate-spin text-brand" />
              <span>正在读取日志流…</span>
            </div>
          ) : filteredLines.length === 0 ? (
            <div className="flex h-full items-center justify-center text-slate-500">
              {t.console.emptyLogs}
            </div>
          ) : (
            <div className="space-y-0.5">
              {filteredLines.map((line, idx) => {
                const displayLine = localizeLogTimestamp(line)
                const isError =
                  line.includes("ERROR") ||
                  line.includes("error") ||
                  line.includes("Err") ||
                  line.includes("failed")
                const isWarn = line.includes("WARN") || line.includes("warn")
                const isInfo = line.includes("INFO") || line.includes("info")

                return (
                  <div
                    key={`${idx}-${line.slice(0, 15)}`}
                    className="flex items-start gap-3 hover:bg-slate-900/60 rounded px-1 -mx-1"
                  >
                    <span className="select-none text-[10px] text-slate-600 w-8 text-right shrink-0 pt-0.5">
                      {idx + 1}
                    </span>
                    <span
                      className={`break-all whitespace-pre-wrap ${
                        isError
                          ? "text-rose-400 font-semibold"
                          : isWarn
                            ? "text-amber-300"
                            : isInfo
                              ? "text-sky-300"
                              : "text-slate-300"
                      }`}
                    >
                      {displayLine}
                    </span>
                  </div>
                )
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
