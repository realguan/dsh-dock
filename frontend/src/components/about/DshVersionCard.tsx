// DSH 维度行（原 about.html render 的 dsh 段迁移）：只读展示 + 可选升级/检查。
import { useEffect, useState } from "react"
import { ArrowUpCircle, LoaderCircle, RefreshCw } from "lucide-react"
import { listen } from "@tauri-apps/api/event"
import { api } from "@/lib/tauri"
import { useBootStore } from "@/stores/bootStore"
import { useI18n } from "@/stores/i18nStore"
import type { ComponentUpdate } from "@/types/ipc"
import { Button } from "@/components/ui/button"
import { DimRow as Row, DimNote as Note } from "./DimRow"

const ACTION_BUSY_MS = 2000

export function DshVersionCard() {
  const { t } = useI18n()
  const dsh = useBootStore((s) => s.versions?.dsh ?? null)
  const [busy, setBusy] = useState<"none" | "check">("none")
  const [upgrading, setUpgrading] = useState(false)
  const [upgradeFail, setUpgradeFail] = useState<string | null>(null)

  useEffect(() => {
    const un = listen<{ phase: string; detail: string }>("dsh:upgrade", ({ payload }) => {
      if (payload.phase === "running") {
        setUpgrading(true)
      } else {
        setUpgrading(false)
        if (payload.phase === "failed") setUpgradeFail(payload.detail || null)
      }
    })
    return () => {
      void un.then((f) => f())
    }
  }, [])

  const lock = (kind: Exclude<typeof busy, "none">) => {
    setBusy(kind)
    window.setTimeout(() => setBusy((b) => (b === kind ? "none" : b)), ACTION_BUSY_MS)
  }

  return (
    <Row label={t.about.dshLabel} badge="Core Engine">
      <div className="flex flex-col items-start gap-2 sm:flex-row sm:items-center sm:justify-end">
        <div className="min-w-0 text-left sm:text-right">
          <VersionView dim={dsh} />
          {upgrading && (
            <div className="mt-1 flex items-center gap-1 text-[11px] text-brand">
              <LoaderCircle className="size-3 animate-spin" />
              <span>{t.about.upgradeRunning}</span>
            </div>
          )}
          {upgradeFail && (
            <div className="mt-1">
              <Note tone="warn">{t.about.upgradeFailed}</Note>
              <p
                className="text-faint mt-0.5 max-w-xs truncate font-mono text-[10px]"
                title={upgradeFail}
              >
                {upgradeFail}
              </p>
            </div>
          )}
        </div>

        <div className="flex shrink-0 items-center gap-1.5 pt-1 sm:pt-0">
          {dsh?.newer && (
            <Button
              size="sm"
              disabled={busy !== "none" || upgrading}
              onClick={() => {
                setUpgradeFail(null)
                setUpgrading(true)
                api.terminalAction("upgrade_only").catch(() => {
                  setUpgrading(false)
                  setUpgradeFail(t.about.upgradeFailed)
                })
              }}
              className="gap-1 text-xs"
            >
              {upgrading ? (
                <LoaderCircle className="size-3 animate-spin" />
              ) : (
                <ArrowUpCircle className="size-3.5" />
              )}
              <span>{upgrading ? t.about.upgrading : t.about.btnUpgrade}</span>
            </Button>
          )}

          <Button
            size="sm"
            variant="outline"
            disabled={busy !== "none" || upgrading}
            onClick={() => {
              lock("check")
              api.checkUpdates().catch(() => {})
            }}
            className="gap-1 text-xs"
          >
            <RefreshCw className={`size-3 ${busy === "check" ? "animate-spin" : ""}`} />
            <span>{busy === "check" ? t.about.detecting : t.about.btnCheck}</span>
          </Button>
        </div>
      </div>
    </Row>
  )
}

function VersionView({ dim }: { dim: ComponentUpdate | null }) {
  const { t } = useI18n()
  if (!dim)
    return (
      <span className="text-faint font-mono text-xs">
        {t.about.notDetected} · <Note>{t.about.detecting}</Note>
      </span>
    )
  if (dim.error)
    return (
      <span className="font-mono text-xs">
        <span className="text-ink font-semibold">{dim.current ?? t.about.notDetected}</span>
        <span className="ml-1.5"><Note tone="warn">{t.about.checkFailedNet}</Note></span>
      </span>
    )
  if (dim.newer)
    return (
      <div className="flex items-center gap-1.5">
        <span className="font-mono text-xs font-semibold text-ink">
          {dim.current ?? t.about.notDetected}
        </span>
        <span className="bg-brand/10 text-brand border border-brand/20 rounded px-1.5 py-0.5 font-mono text-[10px] font-medium">
          {t.about.hasNew} {dim.latest ?? ""}
        </span>
      </div>
    )
  if (dim.current && dim.latest)
    return (
      <span className="font-mono text-xs">
        <span className="font-semibold text-ink">{dim.current}</span>
        <span className="text-faint ml-1.5 text-[11px]">（{t.about.latestIsNewest}）</span>
      </span>
    )
  if (!dim.current && dim.latest)
    return (
      <span className="font-mono text-xs text-dim">
        {t.about.notDetected} · <Note>{t.about.latestOfficial} {dim.latest}</Note>
      </span>
    )
  return (
    <span className="font-mono text-xs">
      {dim.current ?? t.about.notDetected}
    </span>
  )
}
