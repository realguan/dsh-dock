// 下载主角位（原 index/selector 的 .dl 区块迁移）：大百分比 + 进度条 +
// 字节/速度/剩余时间；total 未知时切不确定动画（··· + pulse 条）。
// 纯展示组件：进度数据经 props 注入（selector/index 各自消费 bootStore.progress）。
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

  return (
    <section className="dl mt-8">
      <div className="text-center">
        <span className="font-mono text-5xl font-semibold tracking-tight text-ink tabular-nums">
          {pct === null ? "···" : Math.floor(pct)}
        </span>
        <span className="text-faint ml-1 text-2xl">%</span>
      </div>

      <div className="mt-4">
        {pct === null ? (
          <div className="pulse-bar w-full" style={{ width: "min(320px,72%)", margin: "0 auto" }}>
            <div className="pulse-bar-fill" />
          </div>
        ) : (
          <Progress value={pct} className="mx-auto w-[min(320px,72%)]" />
        )}
      </div>

      <div className="text-faint mt-3 flex items-center justify-center gap-3 font-mono text-xs tabular-nums">
        <span>
          {progress.total !== null
            ? `${fmtBytes(progress.current)} / ${fmtBytes(progress.total)}`
            : `已下载 ${fmtBytes(progress.current)}`}
        </span>
        {fmtSpeed(progress.speed) && <span>{fmtSpeed(progress.speed)}</span>}
        {progress.eta !== null && fmtEta(progress.eta) && (
          <span>剩余 {fmtEta(progress.eta)}</span>
        )}
      </div>
    </section>
  )
}
