// 官方徽章组件（AGENTS §3 品牌规则 & /ui-ux-pro-max）：
// 全局统一渲染 3D 鲸鱼娘品牌形象。
// - framed = false（默认）：直接展示 100% 透明背景的吉祥物形象，用于启动欢迎、Hero 区域与全屏等待，视觉生动呼吸。
// - framed = true：使用 Apple HIG 风格纯色浅色圆角方框（/icon.png），用于紧凑导航条、配置窗口与 About 弹窗，规范结构化。
export interface EmblemProps {
  /** 尺寸（像素），默认 44 */
  size?: number
  className?: string
  alt?: string
  /**
   * 是否使用浅色方框：
   * - false（默认）：直接展示纯透明背景吉祥物立绘，消除边界感。
   * - true：使用 macOS HIG 标准浅色圆角方框，适合工具栏、设置头与 About 弹窗。
   */
  framed?: boolean
}

export function Emblem({
  size = 44,
  className,
  alt = "DSH Dock",
  framed = false,
}: EmblemProps) {
  if (framed) {
    return (
      <div
        className={`relative inline-flex shrink-0 items-center justify-center select-none ${className ?? ""}`}
        style={{ width: size, height: size }}
      >
        <img
          src="/icon.png"
          alt={alt}
          width={size}
          height={size}
          className="size-full object-contain pointer-events-none drop-shadow-xs"
          draggable={false}
        />
      </div>
    )
  }

  return (
    <div
      className={`relative inline-flex shrink-0 items-center justify-center select-none ${className ?? ""}`}
      style={{ width: size, height: size }}
    >
      <img
        src="/whale-chan-cutout.png"
        alt={alt}
        width={size}
        height={size}
        className="size-full object-contain pointer-events-none drop-shadow-glow"
        draggable={false}
      />
    </div>
  )
}
