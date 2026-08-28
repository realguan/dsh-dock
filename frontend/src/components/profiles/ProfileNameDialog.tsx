// 复制 / 重命名共用对话框（4.3 前端刀）：同一形态（新名字输入 + 提交），
// 差异只在 api 与说明文案；完成后 warnings（如 patch ../ 引用）就地展示，
// 不静默吞掉——那是 ADR-0009 明文要求的人工检查提示。
import { useState } from "react"
import { validateProfileName } from "@/lib/profiles"
import { api } from "@/lib/tauri"
import { t } from "@/content/zh-CN"
import type { ProfileSummary } from "@/types/ipc"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

export type NameOpMode = "copy" | "rename"

export function ProfileNameDialog({
  mode,
  source,
  existing,
  onClose,
  onRefresh,
  onDone,
}: {
  mode: NameOpMode
  /** 源 profile 名（重命名的旧名 / 复制的源名） */
  source: string | null
  existing: ProfileSummary[]
  onClose: () => void
  onRefresh: () => void
  /** 成功后的页面级提示（新名 + warnings 文本） */
  onDone: (newName: string, warnings: string[]) => void
}) {
  const [name, setName] = useState("")
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [warnings, setWarnings] = useState<string[] | null>(null)

  const open = source !== null
  const trimmed = name.trim()
  const invalid =
    mode === "rename" && trimmed === source
      ? t.profiles.renameSame
      : trimmed !== ""
        ? validateProfileName(trimmed)
        : null
  const occupied = existing.some((p) => p.name === trimmed)
  const nameError = trimmed === "" ? null : (invalid ?? (occupied ? t.profiles.nameOccupied : null))

  const reset = () => {
    setName("")
    setBusy(false)
    setError(null)
    setWarnings(null)
  }
  const close = () => {
    if (busy) return
    reset()
    onClose()
  }

  const submit = () => {
    if (!source || busy || nameError || trimmed === "") return
    setBusy(true)
    setError(null)
    const call =
      mode === "copy"
        ? api.copyProfile(source, trimmed)
        : api.renameProfile(source, trimmed)
    call
      .then((out) => {
        setBusy(false)
        onRefresh()
        if (out.warnings.length > 0) {
          // 有警告：留在对话框里展示（关窗即错过，违反 ADR 的「提示」意图）
          setWarnings(out.warnings)
        } else {
          onDone(out.profile, [])
          reset()
          onClose()
        }
      })
      .catch((e) => {
        setBusy(false)
        setError(String(e))
      })
  }

  const title =
    mode === "copy" ? t.profiles.copyTitle(source ?? "") : t.profiles.renameTitle(source ?? "")

  return (
    <Dialog open={open} onOpenChange={(o) => !o && close()}>
      <DialogContent className="sm:max-w-[440px]">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription className="sr-only">{title}</DialogDescription>
        </DialogHeader>

        {warnings === null && (
          <>
            <div className="space-y-3">
              <label className="text-dim block text-xs" htmlFor="profile-new-name-input">
                {t.profiles.newNameLabel}
              </label>
              <input
                id="profile-new-name-input"
                autoFocus
                disabled={busy}
                value={name}
                onChange={(e) => setName(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && submit()}
                placeholder={t.profiles.createNamePlaceholder}
                className="border-line bg-bg text-ink placeholder:text-faint focus:border-brand w-full rounded-lg border px-3 py-2 font-mono text-sm outline-none transition-colors disabled:opacity-50"
              />
              <div className="text-xs">
                {nameError ? (
                  <span className="text-warn">{nameError}</span>
                ) : (
                  <span className="text-faint">
                    {mode === "copy" ? t.profiles.copyNote : t.profiles.renameNote}
                  </span>
                )}
              </div>
            </div>
            {error && (
              <div className="bg-warn-soft text-warn rounded-lg px-3 py-2 text-xs whitespace-pre-wrap">
                {error}
              </div>
            )}
          </>
        )}

        {warnings !== null && (
          <div className="bg-warn-soft text-warn space-y-1.5 rounded-lg px-3 py-2.5 text-xs">
            <div className="font-medium">{t.profiles.warningsLabel}</div>
            {warnings.map((w, i) => (
              <div key={i}>{w}</div>
            ))}
          </div>
        )}

        <DialogFooter>
          {warnings !== null ? (
            <Button
              onClick={() => {
                onDone(trimmed, warnings)
                reset()
                onClose()
              }}
            >
              {t.profiles.detailClose}
            </Button>
          ) : (
            <>
              <Button variant="outline" disabled={busy} onClick={close}>
                {t.profiles.detailClose}
              </Button>
              <Button disabled={busy || !!nameError || trimmed === ""} onClick={submit}>
                {busy
                  ? t.profiles.busyShort
                  : mode === "copy"
                    ? t.profiles.submitCopy
                    : t.profiles.submitRename}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
