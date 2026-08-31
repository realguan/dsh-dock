import { AlertCircle, CheckCircle2, Info, X } from "lucide-react"
import { AnimatePresence, motion } from "framer-motion"

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
  return (
    <AnimatePresence>
      {toast && (
        <motion.div
          key={toast.id}
          initial={{ opacity: 0, y: 16, scale: 0.96 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: 8, scale: 0.96 }}
          transition={{ type: "spring", stiffness: 400, damping: 30 }}
          className="fixed bottom-5 left-1/2 z-50 -translate-x-1/2 px-4 pointer-events-none"
        >
          <div
            className={`pointer-events-auto flex items-center gap-2.5 rounded-full px-4 py-2 text-xs font-medium shadow-lg backdrop-blur-md transition-all ${
              toast.kind === "ok"
                ? "bg-slate-900/90 text-white shadow-emerald-500/10 ring-1 ring-emerald-500/30"
                : toast.kind === "warn"
                  ? "bg-amber-950/90 text-amber-100 shadow-amber-500/10 ring-1 ring-amber-500/40"
                  : "bg-slate-900/90 text-slate-100 shadow-slate-900/20 ring-1 ring-white/20"
            }`}
          >
            {toast.kind === "ok" && (
              <CheckCircle2 className="size-4 text-emerald-400 shrink-0" />
            )}
            {toast.kind === "warn" && (
              <AlertCircle className="size-4 text-amber-400 shrink-0" />
            )}
            {toast.kind === "info" && (
              <Info className="size-4 text-blue-400 shrink-0" />
            )}
            <span className="max-w-[420px] truncate">{toast.message}</span>
            <button
              type="button"
              onClick={onDismiss}
              aria-label="关闭通知"
              className="ml-1 shrink-0 rounded-full p-0.5 opacity-60 hover:opacity-100 hover:bg-white/10 transition-colors"
            >
              <X className="size-3.5" />
            </button>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}
