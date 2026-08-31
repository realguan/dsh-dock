// 下载主角位（原 index/selector 的 .dl 区块升级迁移）：大百分比 + 进度条 +
// 字节/速度/剩余时间；total 未知时切不确定动画。
import { motion } from "framer-motion"
import { DownloadCloud, Zap, Clock } from "lucide-react"
import { fmtBytes, fmtEta, fmtSpeed } from "@/lib/format"
import { useBootStore } from "@/stores/bootStore"
import { Progress } from "@/components/ui/progress"

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
      initial={{ opacity: 0, scale: 0.98 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.25, ease: "easeOut" }}
      className="mx-auto mt-6 w-full max-w-xl rounded-2xl border border-brand/20 bg-panel/95 p-5 shadow-md backdrop-blur-xs"
    >
      {/* 顶栏：下载目标 + 动态速率/ETA 芯片 */}
      <div className="flex items-center justify-between border-b border-line/70 pb-3">
        <div className="flex items-center gap-2">
          <span className="flex size-7 items-center justify-center rounded-lg bg-wash text-brand-deep">
            <DownloadCloud className="size-4 animate-bounce" />
          </span>
          <div>
            <span className="text-xs font-semibold text-ink">{kindLabel}</span>
            <span className="ml-2 font-mono text-[11px] text-faint">自动拉取中</span>
          </div>
        </div>

        <div className="flex items-center gap-2">
          {fmtSpeed(progress.speed) && (
            <span className="inline-flex items-center gap-1 rounded-md bg-wash px-2 py-0.5 font-mono text-[11px] text-brand-deep">
              <Zap className="size-3" />
              {fmtSpeed(progress.speed)}
            </span>
          )}
          {progress.eta !== null && fmtEta(progress.eta) && (
            <span className="inline-flex items-center gap-1 rounded-md bg-line-soft px-2 py-0.5 font-mono text-[11px] text-dim">
              <Clock className="size-3" />
              {fmtEta(progress.eta)}
            </span>
          )}
        </div>
      </div>

      {/* 进度百分比与进度条 */}
      <div className="mt-4">
        <div className="flex items-baseline justify-between mb-2">
          <div className="flex items-baseline gap-1">
            <span className="font-mono text-4xl font-bold tracking-tight text-ink tabular-nums">
              {pct === null ? "···" : Math.floor(pct)}
            </span>
            <span className="text-lg font-medium text-faint">%</span>
          </div>
          <span className="font-mono text-xs text-dim tabular-nums">
            {progress.total !== null
              ? `${fmtBytes(progress.current)} / ${fmtBytes(progress.total)}`
              : `已传输 ${fmtBytes(progress.current)}`}
          </span>
        </div>

        {pct === null ? (
          <div className="relative h-2 w-full overflow-hidden rounded-full border border-line bg-line-soft">
            <div className="pulse-bar-fill rounded-full" />
          </div>
        ) : (
          <Progress value={pct} className="h-2 w-full" />
        )}
      </div>

      <div className="mt-3 flex items-center justify-between text-[11px] text-faint">
        <span>官方发布源下载 · 校验签名完整性</span>
        <span>首次使用自动准备 · 仅此一次</span>
      </div>
    </motion.section>
  )
}

