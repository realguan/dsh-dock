// 连续计时徽章（ADR-0014）：启动屏卡头右侧的「已用 2.4s」。
// 时钟只在这个叶子里走（5Hz），页面主体不因秒表重渲染；起点来自 Rust 交接
// 意图的 startedAt —— 主窗口整文档重载后接着数，不归零。
import { useElapsedMs } from "@/hooks/useElapsed"
import { formatElapsed } from "@/lib/handoff"
import { useI18n } from "@/stores/i18nStore"

export function ElapsedChip({ startedAt }: { startedAt: number }) {
  const { t } = useI18n()
  const ms = useElapsedMs(startedAt)
  return (
    <span
      title={t.handoff.elapsedTitle}
      className="text-dim bg-line-soft/80 shrink-0 rounded-full px-1.5 py-0.5 font-mono text-micro tabular-nums"
    >
      {formatElapsed(ms)}
    </span>
  )
}
