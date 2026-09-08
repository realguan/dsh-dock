// 进行中动态光条（步骤 running 时的生命感；下载条出现时让位）。
import { useI18n } from "@/stores/i18nStore"

export function PulseBar({ width = 300 }: { width?: number | string }) {
  const { t } = useI18n()
  return (
    <div
      className="relative mx-auto h-1.5 overflow-hidden rounded-full border border-line bg-line-soft/80 shadow-xs"
      style={{ width }}
      role="progressbar"
      aria-label={t.boot.progressLabel}
    >
      <div className="pulse-bar-fill rounded-full" />
    </div>
  )
}

