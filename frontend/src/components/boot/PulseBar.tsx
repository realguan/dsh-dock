// 进行中动态光条（步骤 running 时的生命感；下载条出现时让位）。
export function PulseBar({ width = 300 }: { width?: number | string }) {
  return (
    <div
      className="relative mx-auto h-1.5 overflow-hidden rounded-full border border-line bg-line-soft/80 shadow-xs"
      style={{ width }}
      role="progressbar"
      aria-label="启动进行中"
    >
      <div className="pulse-bar-fill rounded-full" />
    </div>
  )
}

