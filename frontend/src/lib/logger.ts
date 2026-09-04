// 统一日志封装（2026-09-04 日志改造裁定）：前端代码禁止直接调用 console.*，
// 一律经本封装。格式：`[模块名] 行为描述 { 上下文参数 }`。
// 约束（与 Rust 侧同口径）：
// - 绝不打印密钥 / Token / 密码 / 用户敏感隐私数据（PII）——调用方负责脱敏；
// - 禁止在紧密循环或高频事件内打日志；
// - debug 级仅 dev 构建输出；warn/error 全环境保留（生产排障最低可观测面）。
type Level = "debug" | "info" | "warn" | "error"

const IS_DEV = import.meta.env.DEV

function emit(
  level: Level,
  module: string,
  message: string,
  context?: Record<string, unknown>,
): void {
  if (level === "debug" && !IS_DEV) return
  const line = `[${module}] ${message}`
  const args = context && Object.keys(context).length > 0 ? [line, context] : [line]
  if (level === "error") console.error(...args)
  else if (level === "warn") console.warn(...args)
  else console.log(...args)
}

export const logger = {
  debug: (module: string, message: string, context?: Record<string, unknown>) =>
    emit("debug", module, message, context),
  info: (module: string, message: string, context?: Record<string, unknown>) =>
    emit("info", module, message, context),
  warn: (module: string, message: string, context?: Record<string, unknown>) =>
    emit("warn", module, message, context),
  error: (module: string, message: string, context?: Record<string, unknown>) =>
    emit("error", module, message, context),
}
