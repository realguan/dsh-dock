// 官方徽章（AGENTS §3 品牌规则）：形状源 public/mark.svg 经 CSS mask 上白，
// 页面不允许内联第二份鲸鱼 path 或第二种颜色。深色圆角底 + 白标 = 唯一形态。
export function Emblem({ size = 44 }: { size?: number }) {
  return (
    <div
      className="emblem relative rounded-xl border border-white/10"
      style={{
        width: size,
        height: size,
        background:
          "linear-gradient(160deg, var(--color-badge-a), var(--color-badge-b))",
        boxShadow:
          "0 10px 26px rgba(20,28,48,0.28), inset 0 1px 0 rgba(255,255,255,0.1)",
      }}
    />
  )
}
