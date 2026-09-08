// components/profiles/BuildApprovalDialog.tsx —— pnpm 12 构建脚本审批门
// 裁决对话框（2026-09-07，ADR-0009 写入例外 #5）：安装/更新撞
// ERR_PNPM_IGNORED_BUILDS 时，被点名依赖逐包允许/跳过 → 写 profile 的
// pnpm-workspace.yaml allowBuilds → 父组件重试原操作。默认全部跳过
// （安装必成），允许由用户显式开启。
import { useEffect, useState } from "react"
import { ShieldCheck } from "lucide-react"
import { api } from "@/lib/tauri"
import { useI18n } from "@/stores/i18nStore"
import { defaultApprovals } from "@/lib/buildApprovals"
import type { BuildApprovalChoice } from "@/lib/buildApprovals"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Switch } from "@/components/ui/switch"

interface BuildApprovalDialogProps {
  profile: string
  packages: string[]
  open: boolean
  onClose: () => void
  /** 保存成功后回调（父组件在此重试原插件操作）。 */
  onApproved: () => void
}

export function BuildApprovalDialog({
  profile,
  packages,
  open,
  onClose,
  onApproved,
}: BuildApprovalDialogProps) {
  const { t } = useI18n()
  const [rows, setRows] = useState<BuildApprovalChoice[]>([])
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // packages 由父组件持有于 state（引用稳定），open 翻转时重置裁决行
  useEffect(() => {
    if (open) {
      setRows(defaultApprovals(packages))
      setError(null)
    }
  }, [open, packages])

  if (!open || packages.length === 0) return null

  const handleConfirm = () => {
    if (saving) return
    setSaving(true)
    setError(null)
    api
      .setProfileBuildApprovals(profile, rows)
      .then(() => {
        onApproved()
      })
      .catch((e) => setError(t.buildGate.saveFailed(String(e))))
      .finally(() => setSaving(false))
  }

  return (
    <Dialog open={open} onOpenChange={(v) => !saving && !v && onClose()}>
      <DialogContent className="max-w-md rounded-2xl border border-line bg-panel p-6 shadow-xl">
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div className="flex size-10 shrink-0 items-center justify-center rounded-xl border border-brand/30 bg-brand/10 text-brand shadow-2xs">
              <ShieldCheck className="size-5" />
            </div>
            <div className="min-w-0 flex-1">
              <DialogTitle className="text-base font-bold text-ink">
                {t.buildGate.title}
              </DialogTitle>
              <DialogDescription className="text-xs text-dim mt-0.5">
                {t.buildGate.subtitle}
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <div className="space-y-2 py-2">
          <div className="grid grid-cols-[1fr_auto] gap-x-3 px-1 text-meta font-medium text-faint">
            <span>package</span>
            <span>
              {t.buildGate.allow} / {t.buildGate.skip}
            </span>
          </div>
          {rows.map((row, i) => (
            <div
              key={row.name}
              className="flex items-center justify-between gap-3 rounded-xl border border-line bg-wash/80 px-3 py-2"
            >
              <div className="min-w-0">
                <div className="font-mono text-xs font-medium text-ink truncate" title={row.name}>
                  {row.name}
                </div>
                <div className="text-meta text-faint leading-snug mt-0.5">
                  {row.allowed ? t.buildGate.allowHint : t.buildGate.skipHint}
                </div>
              </div>
              <Switch
                aria-label={row.name}
                checked={row.allowed}
                disabled={saving}
                onCheckedChange={(v) =>
                  setRows((rs) =>
                    rs.map((r, j) => (j === i ? { ...r, allowed: v } : r)),
                  )
                }
              />
            </div>
          ))}

          {error && (
            <div className="rounded-xl border border-rose-500/30 bg-rose-500/10 p-3 text-xs text-rose-600 dark:text-rose-400 break-all">
              {error}
            </div>
          )}
        </div>

        <DialogFooter className="gap-2 sm:gap-0 pt-2 border-t border-line/60">
          <Button variant="outline" size="sm" onClick={onClose} disabled={saving} className="rounded-xl text-xs">
            {t.profiles.pluginInstallCancel}
          </Button>
          <Button
            size="sm"
            onClick={handleConfirm}
            disabled={saving}
            className="rounded-xl bg-brand text-white hover:bg-brand/90 text-xs font-medium shadow-xs"
          >
            <span>{saving ? t.buildGate.saving : t.buildGate.confirm}</span>
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
