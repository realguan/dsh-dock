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
