// confirm-dialog.tsx —— 破坏性操作确认对话框（2026-09-08，UI/UX 评审 U9）。
//
// 以 `ProfileDeleteDialog` 为模板抽出的**通用外壳**：标题 + 风险说明 + 要点清单 +
// 破坏性主按钮。本组件不含任何业务文案（全部由调用方从 `content/zh-CN.ts` 取），
// 也不发 IPC——调用方在 `onConfirm` 里做动作、用 `busy`/`error` 回报状态。
//
// 为什么统一到模态而非 `window.confirm`（评审 §3.3）：原生确认框无法列出风险要点、
// 无法在窗口内保持一致样式，且阻断渲染进程。新增破坏性操作一律走本组件。
import type { ReactNode } from "react"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

export function ConfirmDialog({
  open,
  title,
  note,
  points,
  confirmLabel,
  cancelLabel,
  busyLabel,
  busy = false,
  error,
  onConfirm,
  onClose,
  children,
}: {
  open: boolean
  title: string
  /** 一句话风险说明（醒目色）。缺省则不渲染。 */
  note?: string
  /** 风险要点清单（逐条列出不可撤销/影响面）。缺省则不渲染。 */
  points?: readonly string[]
  confirmLabel: string
  cancelLabel: string
  /** 执行中按钮文案；缺省沿用 `confirmLabel`。 */
  busyLabel?: string
  busy?: boolean
  /** 失败详情（调用方捕获后传入，展示在按钮上方）。 */
  error?: string | null
  onConfirm: () => void
  onClose: () => void
  /** 需要展示 diff / 附加信息时插入（可选）。 */
  children?: ReactNode
}) {
  // 执行中不允许关闭：动作已发出，关掉对话框只会让用户以为没发生。
  const close = () => {
    if (!busy) onClose()
  }

  return (
    <Dialog open={open} onOpenChange={(o) => !o && close()}>
      <DialogContent className="sm:max-w-[440px]">
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          {note && (
            <DialogDescription className="text-warn text-xs">{note}</DialogDescription>
          )}
        </DialogHeader>

        {points && points.length > 0 && (
          <ul className="bg-warn-soft text-dim space-y-1.5 rounded-lg px-3 py-2.5 text-xs leading-relaxed">
            {points.map((p, i) => (
              <li key={i} className="flex gap-1.5">
                <span aria-hidden className="text-warn">
                  ·
                </span>
                <span>{p}</span>
              </li>
            ))}
          </ul>
        )}

        {children}

        {error && (
          <div className="bg-danger-soft text-danger rounded-lg px-3 py-2 text-xs whitespace-pre-wrap">
            {error}
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" disabled={busy} onClick={close}>
            {cancelLabel}
          </Button>
          <Button variant="destructive" disabled={busy} onClick={onConfirm}>
            {busy ? (busyLabel ?? confirmLabel) : confirmLabel}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
