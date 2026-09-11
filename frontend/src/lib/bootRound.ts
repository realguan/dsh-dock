// bootRound.ts —— 「新一轮启动」判定（纯逻辑，2026-09-11 v1.2.0 实测 2.1）。
//
// 为什么需要它：`bootStore.error` 只在用户点错误卡动作时清（`ErrorCard` 的
// `clearError()`），而**新一轮启动开始**没有任何清理点。于是截图 ② 的现象成立：
// 本机模式失败留下错误 → 用户点右上角切到 WSL → 新会话真的起来了（01–03 全绿、
// 04 运行中），但 `bootStore.error` 仍是上一轮那条「网络不可用 / os error 5」，
// 诊断卡继续挂在迁移成功的画面上——**错误描述的是一个已经不存在的世界**。
//
// 判据为什么是「步 0 从非 running 变为 running」：
// 启动轮次由 Rust 侧从步 0 重新起跑（`emit_step` 序列），故步 0 的状态边沿
// 就是「新一轮开始」在**前端唯一可观测**的信号；稳态（一直 running）不算，
// 否则会把本轮刚产生的真实错误立刻抹掉。
//
// 与 T-D4r 的分工：Rust 侧负责 `get_boot_status` 不再回吐过期错误（跨文档路径）；
// 本模块负责**同一文档内**的新轮次清理（模式切换、重试等原地重启路径）。
import type { BootStepState } from "@/types/events"

/** 步 0 的状态（`undefined` = 尚未收到任何步骤事件）。 */
export type Step0Status = BootStepState | undefined

/**
 * 是否观测到「新一轮启动起跑」。
 *
 * 边沿判定：`cur === "running" && prev !== "running"`。
 * 稳态不触发——否则错误卡刚出现就会被下一帧抹掉（那才是「误藏错误」）。
 */
export function isNewBootRound(prev: Step0Status, cur: Step0Status): boolean {
  return cur === "running" && prev !== "running"
}

/**
 * 该轮次开始时是否应当撤销上一轮的错误。
 *
 * 之所以单独成函数而不是内联：`isNewBootRound` 只回答"是不是新轮次"，
 * 而"要不要清错误"还取决于当前**是否持有可清的错误**——两件事分开表达，
 * 测试才能分别钉住（避免"新轮次必然清"这种过宽断言掩盖真实条件）。
 */
export function shouldClearStaleError(
  prev: Step0Status,
  cur: Step0Status,
  hasError: boolean,
): boolean {
  return hasError && isNewBootRound(prev, cur)
}
