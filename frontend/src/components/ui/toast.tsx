import { AlertCircle, CheckCircle2, Info, X } from "lucide-react"
import { AnimatePresence, motion } from "framer-motion"
import { useI18n } from "@/stores/i18nStore"

export interface ToastMessage {
  id: string
  kind: "ok" | "warn" | "info"
  title?: string
  message: string
}

export function FloatingToast({
  toast,
  onDismiss,
}: {
  toast: ToastMessage | null
  onDismiss: () => void
}) {
  const { t } = useI18n()
  return (
    // 常驻 live region（2026-09-08 裁定）：读屏只在「区域已存在、内容变化」时可靠
    // 播报；把 role=status 挂在随 toast 挂载/卸载的节点上，部分读屏会整条漏播。
    // 因此容器恒在 DOM，动画只作用于内层 motion.div。
    // 2026-09-10 裁定：容器居中改 inset-x-0 + mx-auto + w-fit（布局层整数居中），
    // 弃用 left-1/2 + -translate-x-1/2——奇数视口宽下半像素合成重采样会让
    // toast 胶囊文字发虚（同 DialogContent 居中修复，见 ui/dialog.tsx）。
    <div
      role="status"
      aria-live="polite"
      aria-atomic="true"
      className="pointer-events-none fixed inset-x-0 bottom-5 z-50 mx-auto w-fit px-4"
    >
      <AnimatePresence>
        {toast && (
          <motion.div
            key={toast.id}
            initial={{ opacity: 0, y: 16, scale: 0.96 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 8, scale: 0.96 }}
            transition={{ type: "spring", stiffness: 400, damping: 30 }}
          >
            <div
              // 2026-09-10 批次 E：本胶囊改为 term-* 深色面 token——它本来就是
              // 「浮在内容之上的深色面板」，与日志/凭据面板同族；旧口径散用
              // slate-900/amber-950/emerald-500/blue-400 五个原生档，其中 info 用
              // blue（全仓唯一一处），与 token 体系脱节。三种 kind 即
              // ok/warn/info 三语义，正好对应 term 族的三个级别色。
              className={`pointer-events-auto flex items-center gap-2.5 rounded-full px-4 py-2 text-xs font-medium shadow-lg backdrop-blur-md transition-all ${
                toast.kind === "ok"
                  ? "bg-term/90 text-white ring-1 ring-term-ok/40"
                  : toast.kind === "warn"
                    ? "bg-term/90 text-term-warn ring-1 ring-term-warn/40"
                    : "bg-term/90 text-term-ink ring-1 ring-white/20"
              }`}
            >
              {toast.kind === "ok" && (
                <CheckCircle2 className="size-4 text-term-ok shrink-0" />
              )}
              {toast.kind === "warn" && (
                <AlertCircle className="size-4 text-term-warn shrink-0" />
              )}
              {toast.kind === "info" && <Info className="size-4 text-term-info shrink-0" />}
              {/* title：消息被 truncate 到 420px，长错误需悬停可读全（2026-09-08） */}
              <span className="max-w-[420px] truncate" title={toast.message}>
                {toast.message}
              </span>
              <button
                type="button"
                onClick={onDismiss}
                aria-label={t.console.toastClose}
                className="ml-1 shrink-0 rounded-full p-0.5 opacity-60 hover:opacity-100 hover:bg-white/10 transition-colors"
              >
                <X className="size-3.5" />
              </button>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  )
}
