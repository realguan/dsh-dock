// 页面外壳：垂直居中 + 最大宽度容器（frontend-migration §3.4 布局层）。
// 纯布局组件，无业务逻辑；入场动画一处定义全页复用（rise 与旧壳同曲线）。
import type { ReactNode } from "react"

export function PageShell({
  children,
  width = 560,
}: {
  children: ReactNode
  width?: number
}) {
  return (
    <main className="bg-bg flex min-h-dvh items-center justify-center px-8">
      <div
        className="page-rise w-full"
        style={{ maxWidth: width }}
      >
        {children}
      </div>
    </main>
  )
}
