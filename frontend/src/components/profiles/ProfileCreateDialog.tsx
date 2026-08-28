// 新建对话框（4.3 前端刀）。前端只做名字预检（validateProfileName 镜像）+
// 已占用预判；后端校验仍是权威。创建走 dsh plugin install（本地初始化，
// 秒级）；「已创建未装插件」按后端契约展示为 pending 而非失败。
import { useState } from "react"
import { summarizeCreateOutcome, TEMPLATE_BUNDLES, validateProfileName } from "@/lib/profiles"
import { api } from "@/lib/tauri"
import { t } from "@/content/zh-CN"
import type { CreateProfileOutcome, ProfileSummary } from "@/types/ipc"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

type Phase =
  | { kind: "form" }
  | { kind: "busy"; name: string }
  | { kind: "result"; outcome: CreateProfileOutcome }

export function ProfileCreateDialog({
  open,
  existing,
  onClose,
  onRefresh,
}: {
  open: boolean
  existing: ProfileSummary[]
  onClose: () => void
  /** 创建落地（含 pending 中间态）后刷新列表 */
  onRefresh: () => void
}) {
  const [name, setName] = useState("")
  const [phase, setPhase] = useState<Phase>({ kind: "form" })

  const trimmed = name.trim()
  const invalid = validateProfileName(trimmed)
  const occupied = existing.some((p) => p.name === trimmed && p.materialized && p.dependencies.length > 0)
  const templateHint = TEMPLATE_BUNDLES[trimmed] ? t.profiles.createTemplateHint(trimmed) : null

  const reset = () => {
    setName("")
    setPhase({ kind: "form" })
  }
  const close = () => {
    if (phase.kind === "busy") return
    reset()
    onClose()
  }
  const closeAndReset = () => {
    reset()
    onClose()
  }

  const submit = () => {
    if (phase.kind === "busy" || invalid || occupied || trimmed === "") return
    setPhase({ kind: "busy", name: trimmed })
    api
      .createProfile(trimmed)
      .then((outcome) => {
        setPhase({ kind: "result", outcome })
        onRefresh()
      })
      .catch((e) => {
        // IPC 层错误（非法名/重名/环境缺失）：以失败结果态展示原始文案
        setPhase({
          kind: "result",
          outcome: { profile: trimmed, materialized: false, installed: false, detail: String(e) },
        })
      })
  }

  const status = phase.kind === "result" ? summarizeCreateOutcome(phase.outcome) : null

  return (
    <Dialog open={open} onOpenChange={(o) => !o && close()}>
      <DialogContent className="sm:max-w-[440px]">
        <DialogHeader>
          <DialogTitle>{t.profiles.createTitle}</DialogTitle>
          <DialogDescription className="sr-only">{t.profiles.createTitle}</DialogDescription>
        </DialogHeader>

        {phase.kind !== "result" && (
          <div className="space-y-3">
            <label className="text-dim block text-xs" htmlFor="profile-name-input">
              {t.profiles.createNameLabel}
            </label>
            <input
              id="profile-name-input"
              autoFocus
              disabled={phase.kind === "busy"}
              value={name}
              onChange={(e) => setName(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && submit()}
              placeholder={t.profiles.createNamePlaceholder}
              className="border-line bg-bg text-ink placeholder:text-faint focus:border-brand w-full rounded-lg border px-3 py-2 font-mono text-sm outline-none transition-colors disabled:opacity-50"
            />
            <div className="text-xs">
              {invalid ? (
                <span className="text-warn">{invalid}</span>
              ) : occupied ? (
                <span className="text-warn">{t.profiles.createNameHelp}</span>
              ) : templateHint ? (
                <span className="text-ok">{templateHint}</span>
              ) : trimmed !== "" ? (
                <span className="text-faint">{t.profiles.createDefaultHint}</span>
              ) : (
                <span className="text-faint">{t.profiles.createNameHelp}</span>
              )}
            </div>
          </div>
        )}

        {phase.kind === "busy" && (
          <div className="bg-wash text-dim rounded-lg px-3 py-2.5 text-xs">
            <span className="text-brand mr-1.5 inline-block size-2 animate-pulse rounded-full bg-current align-middle" />
            {t.profiles.createBusy}
          </div>
        )}

        {phase.kind === "result" && (
          <div className="space-y-2">
            <div
              className={`rounded-lg px-3 py-2.5 text-xs font-medium ${
                status === "ready"
                  ? "bg-ok-soft text-ok"
                  : status === "pending"
                    ? "bg-warn-soft text-warn"
                    : "bg-warn-soft text-warn"
              }`}
            >
              {status === "ready"
                ? t.profiles.createDoneReady
                : status === "pending"
                  ? t.profiles.createDonePending
                  : t.profiles.createDoneFailed}
            </div>
            <pre className="border-line bg-bg text-dim max-h-40 overflow-auto rounded-lg border p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap">
              {phase.outcome.detail}
            </pre>
          </div>
        )}

        <DialogFooter>
          {phase.kind === "result" && status !== "ready" && (
            <Button variant="outline" onClick={submit}>
              {t.profiles.createAgain}
            </Button>
          )}
          {phase.kind === "result" ? (
            <Button onClick={closeAndReset}>{t.profiles.detailClose}</Button>
          ) : (
            <>
              <Button variant="outline" disabled={phase.kind === "busy"} onClick={close}>
                {t.profiles.detailClose}
              </Button>
              <Button
                disabled={phase.kind === "busy" || !!invalid || occupied || trimmed === ""}
                onClick={submit}
              >
                {t.profiles.createSubmit}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
