// OfficialLab.tsx —— 「官方实验室」策展目录面板（2026-09-15，ADR-0020）。
//
// 职责：把后端 `list_official_plugins` 给出的**可下发目录行**渲染成卡片，并按用户点击
// 驱动 `lib/officialCatalog.ts` 的**有序编排**（同族先替换 → 逐包串行安装 →
// `insert_row` 步紧跟写挂载行），把进度与失败续装如实呈现。
//
// 为什么进度与失败要显式呈现：条目的每一步都不可跳过——Agent Teams 的 Web 层装反了会在
// Team 服务就位前挂载而失败；而同族 provider 并列会直接**激活失败**。故 UI 不做"一键
// 乐观成功"，而是按步报告，失败时停在一致态并提供续装。

import { useCallback, useEffect, useState } from "react"
import { Check, LoaderCircle, PackagePlus, Sparkles } from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { api } from "@/lib/tauri"
import { useI18n } from "@/stores/i18nStore"
import {
  officialCatalogIO,
  planEntryRun,
  runPlan,
  type CatalogProgress,
} from "@/lib/officialCatalog"
import type {
  OfficialCatalogRow,
  OfficialCatalogStep,
  ProfileSummary,
} from "@/types/ipc"

interface OfficialLabProps {
  refreshKey: number
  onNotice?: (message: string, tone?: "ok" | "warn") => void
}

/** 单步呈现：谁负责写配置行、钉到哪个版本、是否已装、有无版本风险。 */
function StepLine({
  step,
  t,
}: {
  step: OfficialCatalogStep
  t: ReturnType<typeof useI18n>["t"]
}) {
  return (
    <li className="flex flex-col gap-0.5 text-label text-dim">
      <div className="flex items-center gap-1.5">
        <span className="font-mono text-faint">{step.ordinal}.</span>
        <span className="font-mono text-ink break-all">{step.package}</span>
        {step.installed && (
          <Badge variant="outline" className="h-4 px-1 text-micro">
            {t.market.labInstalledBadge}
          </Badge>
        )}
      </div>
      {/* 激活方式必须显式告知：bundle 类由 dsh 自行激活，plain 类才由壳写行——
          这正是 v1 方案两向皆错的地方，不能让用户猜。 */}
      <span className="pl-4 text-micro text-faint">
        {step.activation === "auto_bundle"
          ? t.market.labActivationAuto
          : t.market.labActivationInsert}
      </span>
      <span className="pl-4 text-micro text-faint">{t.market.labPinnedTo(step.spec)}</span>
      {step.versionNotice && (
        <span className="pl-4 text-micro text-warn">{step.versionNotice}</span>
      )}
    </li>
  )
}

export function OfficialLab({ refreshKey, onNotice }: OfficialLabProps) {
  const { t } = useI18n()
  const [profiles, setProfiles] = useState<ProfileSummary[]>([])
  const [activeProfile, setActiveProfile] = useState<string>("")
  const [rows, setRows] = useState<OfficialCatalogRow[] | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [busyLabel, setBusyLabel] = useState<string | null>(null)
  const [progress, setProgress] = useState<CatalogProgress | null>(null)
  /** 失败续装锚点：条目 → 已完成的步数。 */
  const [resumeFrom, setResumeFrom] = useState<Record<string, number>>({})

  useEffect(() => {
    void (async () => {
      const [profs, active] = await Promise.all([
        api.listProfiles().catch(() => [] as ProfileSummary[]),
        api.getActiveProfile().catch(() => null),
      ])
      setProfiles(profs)
      setActiveProfile(active ?? profs[0]?.name ?? "")
    })()
  }, [refreshKey])

  const loadRows = useCallback(async () => {
    if (!activeProfile) return
    setLoading(true)
    setError(null)
    try {
      setRows(await api.listOfficialPlugins(activeProfile))
    } catch (e) {
      setError(`${t.market.labLoadFailed}：${String(e)}`)
    } finally {
      setLoading(false)
    }
  }, [activeProfile, t])

  useEffect(() => {
    void loadRows()
  }, [loadRows])

  const handleRun = async (row: OfficialCatalogRow, from = 0) => {
    const plan = planEntryRun(row)
    const remaining = from > 0 ? { ...plan, ops: plan.ops.slice(from) } : plan
    setBusyLabel(row.labelZh)
    setProgress(null)
    try {
      const result = await runPlan(officialCatalogIO, activeProfile, remaining, setProgress)
      if (result.ok) {
        onNotice?.(`${row.labelZh}：${t.market.labDone}`, "ok")
        setResumeFrom((prev) => {
          const next = { ...prev }
          delete next[row.labelZh]
          return next
        })
      } else {
        // 失败**不回滚**：已完成的步处于一致态，回报进度让用户续装。
        const done = from + result.completedOps
        setResumeFrom((prev) => ({ ...prev, [row.labelZh]: done }))
        onNotice?.(
          t.market.labPartial(done, from + result.totalOps, result.failedAt?.error ?? ""),
          "warn",
        )
      }
      await loadRows()
    } finally {
      setBusyLabel(null)
      setProgress(null)
    }
  }

  if (!activeProfile) {
    return (
      <p className="px-1 py-6 text-center text-label text-dim">{t.market.labPickProfile}</p>
    )
  }

  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <p className="text-label text-dim">{t.market.labDesc}</p>
        <select
          aria-label={t.market.selectProfile}
          value={activeProfile}
          onChange={(e) => setActiveProfile(e.target.value)}
          className="ml-auto rounded-md border border-line bg-wash px-2 py-1 text-label text-ink"
        >
          {profiles.map((p) => (
            <option key={p.name} value={p.name}>
              {p.name}
            </option>
          ))}
        </select>
      </div>

      {loading && !rows && (
        <p className="px-1 py-6 text-center text-label text-dim">
          <LoaderCircle className="mx-auto size-4 animate-spin" />
        </p>
      )}
      {error && <p className="px-1 py-2 text-label text-warn">{error}</p>}

      {rows?.map((row) => {
        const planned = planEntryRun(row)
        const resumed = resumeFrom[row.labelZh] ?? 0
        const busy = busyLabel === row.labelZh
        const total = planned.ops.length
        return (
          <article
            key={row.labelZh}
            className="rounded-xl border border-line bg-panel p-4 shadow-2xs"
          >
            <div className="flex items-start gap-2.5">
              <div className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-brand/40 bg-brand/10 text-brand-deep">
                <Sparkles className="size-4.5" />
              </div>
              <div className="min-w-0 flex-1">
                <h3 className="text-note font-semibold text-ink">{row.labelZh}</h3>
                {row.requiresNoteZh && (
                  <p className="mt-0.5 text-micro text-faint">{row.requiresNoteZh}</p>
                )}
                {/* 互斥必须显式提示"将要移除谁"：并列会激活失败，静默替换更糟。 */}
                {row.conflictWith && (
                  <p className="mt-1 text-micro text-warn">
                    {t.market.labReplaces(row.conflictWith)}
                  </p>
                )}
                <ul className="mt-2 flex flex-col gap-1.5">
                  {row.steps.map((s) => (
                    <StepLine key={`${s.ordinal}-${s.package}`} step={s} t={t} />
                  ))}
                </ul>
              </div>
              <div className="flex shrink-0 flex-col items-end gap-1.5">
                <Button
                  size="sm"
                  disabled={busy}
                  onClick={() => void handleRun(row, resumed)}
                  className="h-7 gap-1 bg-brand text-white hover:bg-brand/90 px-2.5 text-xs"
                >
                  {busy ? (
                    <LoaderCircle className="size-3 animate-spin" />
                  ) : resumed > 0 ? (
                    <Check className="size-3" />
                  ) : (
                    <PackagePlus className="size-3" />
                  )}
                  <span>
                    {busy && progress
                      ? t.market.labProgress(progress.index + 1, progress.total)
                      : resumed > 0
                        ? t.market.labResume
                        : t.market.labInstallBtn}
                  </span>
                </Button>
                <span className="text-micro text-faint">
                  {planned.ops.length}/{total}
                </span>
              </div>
            </div>
          </article>
        )
      })}
    </div>
  )
}
