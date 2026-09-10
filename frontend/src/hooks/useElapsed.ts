// 连续计时叶子钩子（ADR-0014）：交接带上的「已用 2.4s」。
//
// 为什么单独抽成钩子：计时需要 ~5Hz 重渲染，但读数只出现在导轨/启动屏的
// 一行文字上。把时钟关在**叶子组件**里（谁显示谁订阅），父页面就不会因为
// 一个秒表每 200ms 重渲染整棵管理台树。
//
// 起点由调用方给（来自 Rust 交接意图的 startedAt）：主窗口在重启中途会整文档
// 重载，新文档读到的是同一个 startedAt —— 于是计时**不归零**，这正是"同一次
// 操作还在继续"最直接的体感来源。
import { useEffect, useState } from "react"

/** 自 `startedAt`（Unix ms）起的毫秒数；`startedAt` 为 null 时恒为 0 且不起表。 */
export function useElapsedMs(startedAt: number | null, tickMs = 200): number {
  const [nowMs, setNowMs] = useState(() => Date.now())
  useEffect(() => {
    if (startedAt === null) return
    setNowMs(Date.now())
    const id = setInterval(() => setNowMs(Date.now()), tickMs)
    return () => clearInterval(id)
  }, [startedAt, tickMs])
  if (startedAt === null) return 0
  return Math.max(0, nowMs - startedAt)
}
