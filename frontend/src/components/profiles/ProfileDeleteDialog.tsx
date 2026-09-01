// 删除确认对话框（4.3 前端刀）。确认要素按 ADR-0009 §2/§4 逐条列明
// （不级联全局数据 / 其他 dsh 实例提醒 / 模板名删除后重新物化）——
// 这是破坏性操作的最后一道闸，文案不得精简。
import { useState } from "react"
import { api } from "@/lib/tauri"
import { useI18n } from "@/stores/i18nStore"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

export function ProfileDeleteDialog({
  name,
  onClose,
  onRefresh,
  onDone,
}: {
  name: string | null
  onClose: () => void
  onRefresh: () => void
  /** 删除成功后的页面级提示（defaultCleared 时提示已回退 web） */
  onDone: (defaultCleared: boolean) => void
}) {
  const { t } = useI18n()
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const close = () => {
    if (busy) return
    setBusy(false)
    setError(null)
    onClose()
  }

  const submit = () => {
    if (!name || busy) return
    setBusy(true)
    setError(null)
    api
      .deleteProfile(name)
      .then((out) => {
        setBusy(false)
        onRefresh()
        onDone(out.default_cleared)
        onClose()
      })
      .catch((e) => {
        setBusy(false)
        setError(String(e))
      })
  }

  return (
    <Dialog open={name !== null} onOpenChange={(o) => !o && close()}>
      <DialogContent className="sm:max-w-[440px]">
        <DialogHeader>
          <DialogTitle>{name ? t.profiles.deleteTitle(name) : ""}</DialogTitle>
          <DialogDescription className="text-warn text-xs">
            {t.profiles.deleteNote}
          </DialogDescription>
        </DialogHeader>

        <ul className="bg-warn-soft text-dim space-y-1.5 rounded-lg px-3 py-2.5 text-xs leading-relaxed">
          {t.profiles.deletePoint.map((p, i) => (
            <li key={i} className="flex gap-1.5">
              <span aria-hidden className="text-warn">
                ·
              </span>
              <span>{p}</span>
            </li>
          ))}
        </ul>

        {error && (
          <div className="bg-warn-soft text-warn rounded-lg px-3 py-2 text-xs whitespace-pre-wrap">
            {error}
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" disabled={busy} onClick={close}>
            {t.profiles.detailClose}
          </Button>
          <Button variant="destructive" disabled={busy} onClick={submit}>
            {busy ? t.profiles.deleteBusy : t.profiles.deleteConfirm(name ?? "")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
