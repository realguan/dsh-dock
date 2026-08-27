// 进行中动画条（步骤 running 时的生命感；下载条出现时让位）。
// 样式本体在 index.css（pulse-bar / pulse-bar-fill，自旧 app.css 迁移）。
export function PulseBar({ width = 320 }: { width?: number | string }) {
  return (
    <div className="pulse-bar mx-auto" style={{ width }}>
      <div className="pulse-bar-fill" />
    </div>
  )
}
