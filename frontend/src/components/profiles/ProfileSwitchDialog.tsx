// 切换确认对话框（4.3⑥，ADR-0009 §4 三次修订）。切换 = 停当前 dsh 以目标
// profile 重启：进行中任务会中断，这是唯一需要用户点头的点——要素文案不得精简。
// 无活跃会话时管理器不弹本窗（无损操作，直接切）。
import { useState } from "react"
import { api } from "@/lib/tauri"
import { t } from "@/content/zh-CN"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

export function ProfileSwitchDialog({
  target,
  active,
  restart = false,
  onClose,
  onDone,
}: {
  /** 切换目标 profile；null = 关闭 */
  target: string | null
  /** 当前会话占用中的 profile（null = 无活跃会话） */
  active: string | null
  /** target === active：重启语义（同 profile 停止重起），文案分叉 */
  restart?: boolean
  onClose: () => void
  /** 切换指令已被壳受理后的页面级提示 */
  onDone: () => void
}) {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const close = () => {
    if (busy) return
    setBusy(false)
    setError(null)
    onClose()
  }

  const submit = () => {
    if (!target || busy) return
    setBusy(true)
    setError(null)
    api
      .switchProfile(target)
      .then(() => {
        setBusy(false)
        onDone()
        onClose()
      })
      .catch((e) => {
        setBusy(false)
        setError(String(e))
      })
  }

  return (
    <Dialog open={target !== null} onOpenChange={(o) => !o && close()}>
      <DialogContent className="sm:max-w-[440px]">
        <DialogHeader>
          <DialogTitle>
            {target ? (restart ? t.profiles.restartTitle(target) : t.profiles.switchTitle(target)) : ""}
          </DialogTitle>
          {/* 切换/重启是常规操作（会话历史落盘不丢）：描述走中性灰，不走删除
              那套警示红——中断代价一句话说清即可 */}
          <DialogDescription className="text-xs">
            {restart ? t.profiles.restartNote : t.profiles.switchNote}
          </DialogDescription>
        </DialogHeader>

        {active && (
          <div className="bg-wash text-dim rounded-lg px-3 py-2 text-xs">{t.profiles.switchFrom(active)}</div>
        )}

        {error && (
          <div className="bg-warn-soft text-warn rounded-lg px-3 py-2 text-xs whitespace-pre-wrap">
            {error}
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" disabled={busy} onClick={close}>
            {t.profiles.detailClose}
          </Button>
          <Button disabled={busy} onClick={submit}>
            {busy
              ? t.profiles.switchBusy
              : target
                ? restart
                  ? t.profiles.restartConfirm(target)
                  : t.profiles.switchConfirm(target)
                : ""}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
