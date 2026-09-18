// logLevel.ts —— 日志行级别判定（2026-09-18 修复：原 `line.includes("error")`
// 使路径含 "error" 的正常行整行误红）。只认行首区域内的级别标记，
// 大小写敏感，避免正文与路径误伤。

export type LogLevel = "error" | "warn" | "info" | null

// 级别标记出现在时间戳/前缀之后的行首区域；超出即视为正文内容，不着色。
const SCAN_LIMIT = 100

const PATTERNS: [LogLevel, RegExp][] = [
  ["error", /(?:^|[\s[\]>|,-])(?:ERROR|FATAL|PANIC|error[:!])(?![\w-])/],
  ["warn", /(?:^|[\s[\]>|,-])(?:WARN|WARNING|warn[:!])(?![\w-])/],
  ["info", /(?:^|[\s[\]>|,-])(?:INFO|info[:!])(?![\w-])/],
]

export function classifyLogLine(line: string): LogLevel {
  const head = line.slice(0, SCAN_LIMIT)
  for (const [level, re] of PATTERNS) {
    if (re.test(head)) return level
  }
  return null
}
