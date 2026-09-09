// DSH 版本选择器（2026-09-09）：全版本列表 + 通道徽章 + 发布日期 + 行内安装/回退。
// 数据经 list_dsh_versions 一次拉取（打开时；安装结束后重拉刷新相对关系），
// 版本比较在后端（relation 字段）——本组件不做第二套比较器。筛选/行动作/
// 确认必要性判定来自 lib/dshVersions 纯函数；确认弹窗与 IPC 归 DshVersionCard。
import { useEffect, useRef, useState } from "react"
import { LoaderCircle } from "lucide-react"
import { api } from "@/lib/tauri"
import { useI18n } from "@/stores/i18nStore"
import type { DshVersionEntry, DshVersionsResult } from "@/types/ipc"
import {
  filterVersions,
  formatPublishedAt,
  rowAction,
  type VersionFilter,
} from "@/lib/dshVersions"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { cn } from "@/lib/utils"

const FILTERS: VersionFilter[] = ["all", "upgradable", "preview"]

const CHANNEL_DOT: Record<DshVersionEntry["channel"], string> = {
  stable: "bg-ok",
  rc: "bg-brand",
  alpha: "bg-warn",
  other: "bg-warn",
}

export function DshVersionListDialog({
  open,
  onOpenChange,
  busyVersion,
  onPick,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** 安装进行中的目标版本（行按钮禁用依据）；null = 空闲 */
  busyVersion: string | null
  /** 选中条目（确认/安装决策归调用方） */
  onPick: (entry: DshVersionEntry) => void
}) {
  const { t } = useI18n()
  const [filter, setFilter] = useState<VersionFilter>("all")
  const [data, setData] = useState<DshVersionsResult | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  /** 手动重拉计数（重试按钮 / 安装结束刷新，避免 effect 依赖 busy 值本身） */
  const [reloadTick, setReloadTick] = useState(0)
  const sawBusy = useRef(false)

  useEffect(() => {
    if (!open) return
    let alive = true
    setLoading(true)
    setError(null)
    api
      .listDshVersions()
      .then((d) => alive && setData(d))
      .catch((e) => alive && setError(String(e)))
      .finally(() => alive && setLoading(false))
    return () => {
      alive = false
    }
  }, [open, reloadTick])

  useEffect(() => {
    // 安装结束（busy 回落 null）且弹窗仍开 → 重拉，行内「当前/安装/回退」
    // 随新关系刷新；安装中不重拉，保持列表滚动位置。
    if (busyVersion) {
      sawBusy.current = true
      return
    }
    if (sawBusy.current && open) {
      sawBusy.current = false
      setReloadTick((n) => n + 1)
    }
  }, [busyVersion, open])

  const channelLabel = (ch: DshVersionEntry["channel"]) =>
    ch === "stable"
      ? t.about.channelStable
      : ch === "rc"
        ? t.about.channelRc
        : ch === "alpha"
          ? t.about.channelAlpha
          : t.about.channelOther

  const filterLabel = (f: VersionFilter) =>
    f === "all"
      ? t.about.filterAll
      : f === "upgradable"
        ? t.about.filterUpgradable
        : t.about.filterPreview

  const visible = data ? filterVersions(data.versions, filter) : []
  const busy = busyVersion !== null

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>{t.about.versionListTitle}</DialogTitle>
          <DialogDescription className="text-faint text-xs">
            {t.about.versionListHint}
          </DialogDescription>
        </DialogHeader>

        <div className="flex items-center gap-1" role="group">
          {FILTERS.map((f) => (
            <button
              key={f}
              type="button"
              aria-pressed={filter === f}
              onClick={() => setFilter(f)}
              className={cn(
                "rounded-full px-2.5 py-1 text-meta font-medium transition-colors",
                filter === f
                  ? "bg-brand-deep text-white"
                  : "bg-line-soft text-dim hover:text-ink",
              )}
            >
              {filterLabel(f)}
            </button>
          ))}
        </div>

        {loading && !data && (
          <div className="text-faint flex items-center justify-center gap-2 py-8 text-xs">
            <LoaderCircle className="size-3.5 animate-spin" />
          </div>
        )}

        {error && (
          <div className="flex items-center justify-between gap-2">
            <span className="text-warn text-xs">{t.about.listLoadFailed}</span>
            <Button
              size="xs"
              variant="outline"
              onClick={() => setReloadTick((n) => n + 1)}
              className="gap-1 text-xs"
            >
              {t.about.listRetry}
            </Button>
          </div>
        )}

        {data && visible.length === 0 && (
          <p className="text-faint py-6 text-center text-xs">{t.about.listEmpty}</p>
        )}

        {visible.length > 0 && (
          <ul className="-mx-1 max-h-72 overflow-y-auto px-1">
            {visible.map((e) => {
              const action = rowAction(e)
              const rowBusy = busy && busyVersion === e.version
              return (
                <li
                  key={e.version}
                  className="border-line/60 flex items-center gap-2 border-b py-2 last:border-b-0"
                >
                  <span className="text-ink shrink-0 font-mono text-xs font-semibold">
                    {e.version}
                  </span>
                  <span className="flex shrink-0 items-center gap-1">
                    <span className={cn("size-1.5 rounded-full", CHANNEL_DOT[e.channel])} />
                    <span className="text-dim text-meta">{channelLabel(e.channel)}</span>
                  </span>
                  <span className="text-faint ml-auto shrink-0 font-mono text-meta">
                    {formatPublishedAt(e.published_at)}
                  </span>
                  {action === "current" ? (
                    <span className="text-ok w-12 shrink-0 text-center text-meta font-medium">
                      {t.about.rowCurrent}
                    </span>
                  ) : rowBusy ? (
                    <span className="text-brand-deep flex w-12 shrink-0 justify-center">
                      <LoaderCircle className="size-3.5 animate-spin" />
                    </span>
                  ) : (
                    <Button
                      size="xs"
                      variant="outline"
                      disabled={busy || loading}
                      onClick={() => onPick(e)}
                      className="shrink-0 font-medium"
                    >
                      {action === "install" ? t.about.rowInstall : t.about.rowRollback}
                    </Button>
                  )}
                </li>
              )
            })}
          </ul>
        )}
      </DialogContent>
    </Dialog>
  )
}
