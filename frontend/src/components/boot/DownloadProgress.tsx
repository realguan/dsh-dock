// 下载主角位（原 index/selector 的 .dl 区块升级迁移）：大百分比 + 进度条 +
// 字节/速度/剩余时间；total 未知时切不确定动画。
import { motion } from "framer-motion"
import { DownloadCloud, Zap, Clock, ShieldCheck } from "lucide-react"
import { fmtBytes, fmtEta, fmtSpeed } from "@/lib/format"
import { useBootStore } from "@/stores/bootStore"

export function DownloadProgress() {
  const progress = useBootStore((s) => s.progress)
  if (!progress) return null

  const pct =
    progress.total !== null && progress.total > 0
      ? Math.min(100, (progress.current / progress.total) * 100)
      : null

  const kindLabel =
    progress.kind === "node"
      ? "Node.js 运行时"
      : progress.kind === "dsh"
        ? "DSH 引擎"
        : progress.kind

  return (
    <motion.section
      initial={{ opacity: 0, scale: 0.98, y: 6 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.98, y: -6 }}
      transition={{ duration: 0.25, ease: "easeOut" }}
      className="mx-auto mt-6 w-full max-w-xl rounded-2xl border border-brand/25 bg-panel/95 p-5 shadow-lg shadow-brand/5 backdrop-blur-md"
    >
      {/* 顶栏：下载目标 + 引擎引导芯片 + 动态速率/ETA */}
      <div className="flex items-center justify-between border-b border-line/70 pb-3">
        <div className="flex items-center gap-2.5">
          <span className="flex size-8 items-center justify-center rounded-xl bg-wash text-brand-deep shadow-2xs">
            <DownloadCloud className="size-4 animate-bounce" />
          </span>
          <div className="flex items-center gap-2">
            <span className="text-sm font-semibold tracking-tight text-ink">{kindLabel}</span>
            <span className="inline-flex items-center gap-1 rounded-full border border-brand/20 bg-brand/5 px-2 py-0.5 font-mono text-[10px] font-medium text-brand">
              <span className="size-1 animate-pulse rounded-full bg-brand" />
              引擎在线引导
            </span>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {fmtSpeed(progress.speed) && (
            <span className="inline-flex items-center gap-1 rounded-lg border border-brand/20 bg-wash px-2 py-0.5 font-mono text-[11px] font-medium text-brand-deep shadow-2xs">
              <Zap className="size-3" />
              {fmtSpeed(progress.speed)}
            </span>
          )}
          {progress.eta !== null && fmtEta(progress.eta) && (
            <span className="inline-flex items-center gap-1 rounded-lg border border-line bg-line-soft/70 px-2 py-0.5 font-mono text-[11px] text-dim shadow-2xs">
              <Clock className="size-3" />
              {fmtEta(progress.eta)}
            </span>
          )}
        </div>
      </div>

      {/* 进度百分比与进度条 */}
      <div className="mt-4">
        <div className="mb-2 flex items-baseline justify-between">
          <div className="flex items-baseline gap-1">
            <span className="font-mono text-4xl font-bold tracking-tight text-ink tabular-nums">
              {pct === null ? "···" : Math.floor(pct)}
            </span>
            <span className="text-lg font-semibold text-dim">%</span>
          </div>
          <span className="font-mono text-xs font-medium text-dim tabular-nums">
            {progress.total !== null
              ? `${fmtBytes(progress.current)} / ${fmtBytes(progress.total)}`
              : `已传输 ${fmtBytes(progress.current)}`}
          </span>
        </div>

        <div className="relative h-2.5 w-full overflow-hidden rounded-full border border-line/80 bg-line-soft/80 shadow-inner">
          {pct === null ? (
            <div className="pulse-bar-fill rounded-full" />
          ) : (
            <motion.div
              className="h-full rounded-full bg-gradient-to-r from-brand-deep via-brand to-sky-400 shadow-[0_0_8px_rgba(65,118,230,0.35)] transition-all duration-300 ease-out"
              style={{ width: `${pct}%` }}
            />
          )}
        </div>
      </div>

      {/* 底部保障与隔离说明 */}
      <div className="mt-3.5 flex items-center justify-between border-t border-line/40 pt-2.5 text-[11px] text-faint">
        <span className="flex items-center gap-1">
          <ShieldCheck className="size-3 text-ok" />
          官方镜像链下载 · 完整性校验
        </span>
        <span>自包含引擎 · 首启就绪后离线直通</span>
      </div>
    </motion.section>
  )
}

