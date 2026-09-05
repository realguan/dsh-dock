// 格式化工具（展示层专用纯函数；Vitest 覆盖见 __tests__/format.test.ts）。

/** 字节 → 自适应单位文本（B/KB/MB/GB，一位小数，整值不带 .0）。 */
export function fmtBytes(n: number): string {
  if (!Number.isFinite(n) || n < 0) return "—"
  if (n < 1024) return `${Math.round(n)} B`
  const units = ["KB", "MB", "GB", "TB"]
  let v = n
  let u = -1
  do {
    v /= 1024
    u++
  } while (v >= 1024 && u < units.length - 1)
  return `${v.toFixed(v >= 100 ? 0 : 1)} ${units[u]}`
}

/** 字节/秒 → 速度文本。样本不足或非有限值返回 null（调用方决定占位符）。 */
export function fmtSpeed(bytesPerSec: number | null): string | null {
  if (bytesPerSec === null || !Number.isFinite(bytesPerSec) || bytesPerSec <= 0)
    return null
  return `${fmtBytes(bytesPerSec)}/s`
}

/** 秒 → 剩余时间文本（mm:ss；超一小时进位 h）；无效输入返回 null。 */
export function fmtEta(seconds: number | null): string | null {
  if (seconds === null || !Number.isFinite(seconds) || seconds < 0) return null
  const s = Math.round(seconds)
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  const sec = s % 60
  const mm = String(h > 0 ? m : m).padStart(2, "0")
  const ss = String(sec).padStart(2, "0")
  return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`
}

/** 百分比：total 缺失返回 null（调用方切不确定进度形态），否则 0-100 整数。 */
export function fmtPercent(current: number, total: number | null): number | null {
  if (total === null || total <= 0) return null
  return Math.min(100, Math.max(0, Math.round((current / total) * 100)))
}

const PROFILE_COLOR_PALETTES = [
  "border-sky-500/30 bg-sky-500/10 text-sky-700 dark:text-sky-300",
  "border-violet-500/30 bg-violet-500/10 text-violet-700 dark:text-violet-300",
  "border-emerald-500/30 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
  "border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300",
  "border-rose-500/30 bg-rose-500/10 text-rose-700 dark:text-rose-300",
  "border-teal-500/30 bg-teal-500/10 text-teal-700 dark:text-teal-300",
  "border-indigo-500/30 bg-indigo-500/10 text-indigo-700 dark:text-indigo-300",
]

/** 根据 Profile 名称确定性派发彩色标签样式类（web 默认品牌蓝，其余名字哈希映射柔和色调）。 */
export function getProfileColorClass(profileName: string): string {
  if (profileName === "web") {
    return "border-brand/30 bg-brand/10 text-brand font-medium"
  }
  let hash = 0
  for (let i = 0; i < profileName.length; i++) {
    hash = profileName.charCodeAt(i) + ((hash << 5) - hash)
  }
  const idx = Math.abs(hash) % PROFILE_COLOR_PALETTES.length
  return PROFILE_COLOR_PALETTES[idx]
}

/**
 * 将日志行中的 ISO8601 UTC 时间戳转换为本地时区显示（日志时区修复，
 * 2026-09-05）：定位行内 ISO 时间戳区间，整体换算为本地时间（含日期
 * 偏移），兼容 ANSI 转义码包裹（如 `\x1b[2m2026-09-05T05:15:33.883479Z\x1b[0m`）；
 * 失败（无时间戳/非法）时原样返回。
 */
export function localizeLogTimestamp(line: string): string {
  if (!line) return line
  const m = line.match(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z/)
  if (!m || m.index === undefined) return line
  const date = new Date(m[0])
  if (Number.isNaN(date.getTime())) return line
  const pad = (n: number, w = 2) => String(n).padStart(w, "0")
  const localTs = `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(
    date.getHours(),
  )}:${pad(date.getMinutes())}:${pad(date.getSeconds())}${m[1] ? m[1].slice(0, 4) : ""}`
  const start = line.slice(0, m.index)
  const end = line.slice(m.index + m[0].length)
  return `${start}${localTs}${end}`
}

