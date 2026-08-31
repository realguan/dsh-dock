// 页面外壳：最大宽度容器，两种纵向对齐（frontend-migration §3.4 布局层）。
// 纯布局组件，无业务逻辑；入场动画一处定义全页复用（rise 与旧壳同曲线）。
// align="top"（2026-08-28 裁定）：常驻工具窗口（关于/Profile 管理器）内容
// 顶部锚定——窗口可调大小后垂直居中会让内容悬浮、且内容超高时顶部被裁切；
// align="center"（默认）：主窗口启动抉择等瞬时决策屏，居中即聚焦。
import type { ReactNode } from "react"

export function PageShell({
  children,
  width = 560,
  align = "center",
  className = "",
}: {
  children: ReactNode
  width?: number | string
  align?: "top" | "center"
  className?: string
}) {
  return (
    <main
      className={`bg-bg flex min-h-dvh justify-center px-4 sm:px-6 md:px-8 ${
        align === "top" ? "items-start py-6 sm:py-8" : "items-center"
      } ${className}`}
    >
      <div
        className="page-rise w-full"
        style={{ maxWidth: typeof width === "number" ? `${width}px` : width }}
      >
        {children}
      </div>
    </main>
  )
}
