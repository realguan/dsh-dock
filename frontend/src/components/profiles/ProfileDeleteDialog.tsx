// 删除确认对话框（4.3 前端刀）。确认要素按 ADR-0009 §2/§4 逐条列明
// （不级联全局数据 / 其他 dsh 实例提醒 / 模板名删除后重新物化）——
// 这是破坏性操作的最后一道闸，文案不得精简。
//
// 2026-09-08（U9）：外壳改用通用 `ui/confirm-dialog`（本组件只留 profile 删除的
// 业务文案与 IPC），与其余破坏性操作共用同一套确认样式。
import { useState } from "react"
import { api } from "@/lib/tauri"
import { useI18n } from "@/stores/i18nStore"
import { ConfirmDialog } from "@/components/ui/confirm-dialog"

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
    <ConfirmDialog
      open={name !== null}
      title={name ? t.profiles.deleteTitle(name) : ""}
      note={t.profiles.deleteNote}
      points={t.profiles.deletePoint}
      confirmLabel={t.profiles.deleteConfirm(name ?? "")}
      busyLabel={t.profiles.deleteBusy}
      cancelLabel={t.confirm.cancel}
      busy={busy}
      error={error}
      onConfirm={submit}
      onClose={close}
    />
  )
}
