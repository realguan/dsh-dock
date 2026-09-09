// components/market/InstallFlight.tsx —— 入队飞行层（2026-09-09，问题记录-2026-09-09 §1.1）。
//
// 职责：消费 stores/installFlightStore 的请求，从点击处飞一颗胶囊到「下载管理」
// 触发按钮，落地时清槽（角标据此弹跳）。整层 pointer-events-none，不挡任何点击。
//
// 降级：`prefers-reduced-motion` 或锚点未登记（触发按钮不在树上）时不飞，直接
// `land()`——反馈落到角标弹跳上，不丢交互确认。
import { useEffect, useState } from "react"
import { AnimatePresence, motion, useReducedMotion } from "framer-motion"
import { Package } from "lucide-react"
import { flightTrack, queueAnchorCenter, type FlightTrack } from "@/lib/installFlight"
import { useInstallFlightStore } from "@/stores/installFlightStore"

/** 动画完成回调的兜底余量（ms）：页面不可见时 framer-motion 可能不回调。 */
const LANDING_GRACE_MS = 150

export function InstallFlight() {
  const request = useInstallFlightStore((s) => s.request)
  const land = useInstallFlightStore((s) => s.land)
  const reduceMotion = useReducedMotion()
  const [track, setTrack] = useState<FlightTrack | null>(null)

  // 轨道要读锚点的真实矩形，只能在渲染后算。
  useEffect(() => {
    if (!request) {
      setTrack(null)
      return
    }
    const target = queueAnchorCenter()
    if (reduceMotion || !target) {
      land()
      return
    }
    setTrack(flightTrack(request.from, target))
  }, [request, reduceMotion, land])

  // 兜底清槽：动画回调丢失时也不能让胶囊卡在屏幕上（否则角标不再响应后续入队）。
  useEffect(() => {
    if (!track) return
    const timer = window.setTimeout(land, track.duration * 1000 + LANDING_GRACE_MS)
    return () => window.clearTimeout(timer)
  }, [track, land])

  return (
    <div aria-hidden className="pointer-events-none fixed inset-0 z-[60] overflow-hidden">
      <AnimatePresence>
        {request && track && (
          <motion.div
            key={request.id}
            className="absolute"
            style={{ left: request.from.x, top: request.from.y }}
            initial={{ x: 0, y: 0, opacity: 0, scale: 0.75 }}
            animate={{
              x: track.x,
              y: track.y,
              opacity: [0, 1, 0.85],
              scale: [0.75, 1.04, 0.45],
            }}
            exit={{ opacity: 0 }}
            transition={{ duration: track.duration, times: track.times, ease: "easeInOut" }}
            onAnimationComplete={land}
          >
            {/* 内层单独做居中位移：外层 transform 归 framer-motion 的 x/y 所有 */}
            <div className="border-brand/40 bg-panel/95 text-ink inline-flex max-w-52 -translate-x-1/2 -translate-y-1/2 items-center gap-1.5 rounded-full border px-2.5 py-1 font-mono text-meta font-medium shadow-lg backdrop-blur-sm">
              <Package className="text-brand-deep size-3 shrink-0" />
              <span className="truncate">{request.label}</span>
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  )
}
