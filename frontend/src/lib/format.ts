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
  // 不足 1 秒 = 即将完成：显示 00:00 读起来像卡死，直接隐藏（调用方不渲染芯片）。
  if (seconds < 1) return null
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

/**
 * Profile 身份芯片样式（2026-09-10 批次 E 重写）。
 *
 * 旧实现按 profile 名哈希派发 **7 色彩虹**（sky/violet/emerald/amber/rose/teal/indigo），
 * 外加 7 处 `dark:` 变体（`.dark` 从未启用 = 死代码）。三处问题：
 *   ① 颜色不承载信息——芯片里就有 profile **名字**，颜色是冗余装饰；
 *   ② 7 个色相在等明度约束下互相挤压（实测最小 ΔOKLab < 0.10，低于可辨阈值），
 *      且在色域夹紧后明度参差——正是「散」的来源；
 *   ③ 借用状态色域（emerald=成功、rose=危险）表达纯身份，语义串台。
 * 改为单一分类档 token：`alt` 是语义中性的「分类/身份」色（见 index.css），
 * 与状态四族互不冒充。名字是信息，颜色只负责「这是个标签」。
 *
 * 参数保留是为了不改三处调用点签名；身份色不再与名字相关。
 */
export function getProfileColorClass(_profileName: string): string {
  return "border-alt/30 bg-alt-soft text-alt font-medium"
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

/**
 * 计算分页带缩略的页码列表。
 * - 总页数 <= 7 时全部展示：1 2 3 4 5 6 7
 * - 靠近开头时（currentPage <= 4）：1 2 3 4 5 ... N
 * - 靠近结尾时（currentPage >= N - 3）：1 ... N-4 N-3 N-2 N-1 N
 * - 居中时：1 ... P-1 P P+1 ... N
 *
 * @param currentPage 当前页（1-indexed）
 * @param totalPages 总页数（>= 1）
 * @returns 页码数字与省略号占位符构成的数组
 */
export function getPaginationPages(
  currentPage: number,
  totalPages: number,
): (number | "...")[] {
  if (totalPages <= 7) {
    return Array.from({ length: totalPages }, (_, i) => i + 1)
  }

  // 靠近开头：1 2 3 4 5 ... totalPages
  if (currentPage <= 4) {
    return [1, 2, 3, 4, 5, "...", totalPages]
  }

  // 靠近结尾：1 ... N-4 N-3 N-2 N-1 N
  if (currentPage >= totalPages - 3) {
    return [
      1,
      "...",
      totalPages - 4,
      totalPages - 3,
      totalPages - 2,
      totalPages - 1,
      totalPages,
    ]
  }

  // 居中：1 ... P-1 P P+1 ... N
  return [
    1,
    "...",
    currentPage - 1,
    currentPage,
    currentPage + 1,
    "...",
    totalPages,
  ]
}
