// DSH 维度行（原 about.html render 的 dsh 段迁移）：状态展示 + 升级 + 版本选择器。
// 2026-09-09 口径对齐（假角标修复）：「有新版/升级」只按可升级口径（稳定/rc）；
// 预览版（alpha 等）经版本列表显式选择安装——选择权交给用户，稳定默认保护。
// alpha 与回退经 ConfirmDialog 知情确认（needsConfirm 纯函数判定）。
import { useEffect, useState } from "react"
import { ArrowUpCircle, LoaderCircle, RefreshCw } from "lucide-react"
import { listen } from "@tauri-apps/api/event"
import { api } from "@/lib/tauri"
import { useBootStore } from "@/stores/bootStore"
import { useI18n } from "@/stores/i18nStore"
import type { ComponentUpdate, DshVersionEntry } from "@/types/ipc"
import { needsConfirm } from "@/lib/dshVersions"
import { Button } from "@/components/ui/button"
import { ConfirmDialog } from "@/components/ui/confirm-dialog"
import { DimRow as Row, DimNote as Note } from "./DimRow"
import { DshVersionListDialog } from "./DshVersionListDialog"

const ACTION_BUSY_MS = 2000
/** 升级结果提示条（已升级到 / 已是最新）的停留时长。 */
const NOTE_DISMISS_MS = 4000

export function DshVersionCard() {
  const { t } = useI18n()
  const dsh = useBootStore((s) => s.versions?.dsh ?? null)
  const [busy, setBusy] = useState<"none" | "check">("none")
  const [upgrading, setUpgrading] = useState(false)
  /** 本次升级的目标版本；null = 未在升级 / 走缺省目标 */
  const [upgradeTarget, setUpgradeTarget] = useState<string | null>(null)
  const [upgradeFail, setUpgradeFail] = useState<string | null>(null)
  const [note, setNote] = useState<string | null>(null)
  const [listOpen, setListOpen] = useState(false)
  const [pendingConfirm, setPendingConfirm] = useState<DshVersionEntry | null>(null)

  useEffect(() => {
    const un = listen<{
      phase: string
      detail: string
      installed?: boolean
    }>("dsh:upgrade", ({ payload }) => {
      if (payload.phase === "running") {
        setUpgrading(true)
      } else {
        setUpgrading(false)
        setUpgradeTarget(null)
        if (payload.phase === "done") {
          const text =
            payload.installed === false
              ? `${t.about.noteAlreadyLatest}（${payload.detail}）`
              : `${t.about.noteUpgraded} ${payload.detail}`
          setNote(text)
          window.setTimeout(() => setNote((n) => (n === text ? null : n)), NOTE_DISMISS_MS)
        }
        if (payload.phase === "failed") setUpgradeFail(payload.detail || null)
      }
    })
    return () => {
      void un.then((f) => f())
    }
  }, [t])

  const lock = (kind: Exclude<typeof busy, "none">) => {
    setBusy(kind)
    window.setTimeout(() => setBusy((b) => (b === kind ? "none" : b)), ACTION_BUSY_MS)
  }

  const install = (version: string | null) => {
    setNote(null)
    setUpgradeFail(null)
    setUpgrading(true)
    setUpgradeTarget(version)
    api.terminalAction("upgrade_only", version).catch(() => {
      setUpgrading(false)
      setUpgradeTarget(null)
      setUpgradeFail(t.about.upgradeFailed)
    })
  }

  const onPick = (entry: DshVersionEntry) => {
    if (needsConfirm(entry)) setPendingConfirm(entry)
    else install(entry.version)
  }

  const confirmIsAlpha =
    pendingConfirm?.channel === "alpha" || pendingConfirm?.channel === "other"

  return (
    <Row label={t.about.dshLabel} badge="Core Engine">
      <div className="flex flex-col items-start gap-2 sm:flex-row sm:items-center sm:justify-end">
        <div className="min-w-0 text-left sm:text-right">
          <VersionView dim={dsh} />
          {upgrading && (
            <div className="text-brand-deep mt-1 flex items-center gap-1 text-label">
              <LoaderCircle className="size-3 animate-spin" />
              <span>
                {upgradeTarget
                  ? `${t.about.installingVersion} ${upgradeTarget}…`
                  : t.about.upgradeRunning}
              </span>
            </div>
          )}
          {!upgrading && dsh && !dsh.error && (
            <button
              type="button"
              onClick={() => setListOpen(true)}
              className="text-brand-deep hover:text-ink mt-1 inline-flex items-center gap-1 text-label transition-colors"
            >
              {dsh?.preview_latest ? (
                <span>
                  {t.about.previewReleased} {dsh.preview_latest}
                </span>
              ) : (
                <span>{t.about.versionListEntry}</span>
              )}
              <span aria-hidden>· {t.about.previewView} ›</span>
            </button>
          )}
          {note && (
            <div className="text-ok mt-1 text-label">{note}</div>
          )}
          {upgradeFail && (
            <div className="mt-1">
              <Note tone="warn">{t.about.upgradeFailed}</Note>
              <p
                className="text-faint mt-0.5 max-w-xs truncate font-mono text-meta"
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
              onClick={() => install(dsh.latest)}
              className="gap-1 text-xs"
            >
              {upgrading ? (
                <LoaderCircle className="size-3 animate-spin" />
              ) : (
                <ArrowUpCircle className="size-3.5" />
              )}
              <span>
                {upgrading
                  ? t.about.upgrading
                  : `${t.about.btnUpgradeTo} ${dsh.latest ?? ""}`}
              </span>
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

      <DshVersionListDialog
        open={listOpen}
        onOpenChange={setListOpen}
        busyVersion={upgrading ? upgradeTarget : null}
        onPick={onPick}
      />

      <ConfirmDialog
        open={pendingConfirm !== null}
        title={
          confirmIsAlpha ? t.about.confirmAlphaTitle : t.about.confirmRollbackTitle
        }
        note={
          pendingConfirm
            ? `${pendingConfirm.version} ${
                confirmIsAlpha
                  ? t.about.confirmAlphaNote
                  : t.about.confirmRollbackNote
              }`
            : undefined
        }
        points={
          confirmIsAlpha ? t.about.confirmAlphaPoints : t.about.confirmRollbackPoints
        }
        confirmLabel={
          confirmIsAlpha ? t.about.confirmInstallAnyway : t.about.confirmRollbackOk
        }
        cancelLabel={t.about.cancelBtn}
        busy={upgrading}
        onConfirm={() => {
          const version = pendingConfirm?.version ?? null
          setPendingConfirm(null)
          if (version) install(version)
        }}
        onClose={() => setPendingConfirm(null)}
      />
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
        <span className="bg-brand/10 text-brand-deep border border-brand/20 rounded-md px-1.5 py-0.5 font-mono text-meta font-medium">
          {t.about.hasNew} {dim.latest ?? ""}
        </span>
      </div>
    )
  // 运行中的版本高于稳定口径（如装了 alpha 后）——如实标注，不再误报「已是最新」
  if (dim.preview_latest && dim.current === dim.preview_latest)
    return (
      <span className="font-mono text-xs">
        <span className="font-semibold text-ink">{dim.current}</span>
        <span className="text-dim ml-1.5 text-label">
          （{t.about.onPreview} {dim.latest ?? ""}）
        </span>
      </span>
    )
  if (dim.current && dim.latest)
    return (
      <span className="font-mono text-xs">
        <span className="font-semibold text-ink">{dim.current}</span>
        <span className="text-faint ml-1.5 text-label">（{t.about.latestIsNewest}）</span>
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
