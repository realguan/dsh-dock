// 启动控制台卡（原 index.html 时间线卡升级迁移；2026-09-07 单主角重构）。
// 卡头 = 徽标 + 当前状态标题/副题 + 分段进度（一眼读出进度与异常，替代
// 原 2/5 计数 chip 与状态 chip）；步骤行沿一条连续导轨排列，行高变化不再
// 撕裂连接线；banner 槽位承载下载进度等瞬态区块（下载条入卡，不再另立区块）。
import type { ReactNode } from "react"
import { useBootStore } from "@/stores/bootStore"
import { useI18n } from "@/stores/i18nStore"
import { BootStep } from "./BootStep"
import { Emblem } from "@/components/layout/Emblem"
import type { BootStepState } from "@/types/events"

const SEG_CLS: Record<BootStepState, string> = {
  done: "bg-ok/60",
  running: "bg-brand animate-pulse motion-reduce:animate-none",
  error: "bg-warn",
  pending: "bg-line",
}

export function BootTimeline({
  title,
  subtitle,
  danger = false,
  banner,
}: {
  /** 卡头标题：当前步骤名 / 下载准备期文案 / 错误标题（调用方推演） */
  title: string
  subtitle?: string
  /** 错误态：标题转警示色、卡边框转警示调 */
  danger?: boolean
  /** 卡头与步骤列表之间的瞬态区块（下载进度条） */
  banner?: ReactNode
}) {
  const { t } = useI18n()
  const steps = useBootStore((s) => s.steps)
  const defs = t.boot.steps
  const doneCount = steps.filter((s) => s.status === "done").length

  return (
    <section
      className={`w-full rounded-2xl border bg-panel/95 shadow-xs backdrop-blur-md ${
        danger ? "border-warn/30" : "border-line/80"
      }`}
    >
      {/* 卡头：徽标 + 当前状态 + 分段进度 */}
      <div className="flex items-center gap-3 border-b border-line/70 px-5 py-4">
        <div className="relative shrink-0">
          <div className="absolute -inset-1.5 rounded-xl bg-brand/15 blur-md" aria-hidden />
          <Emblem size={28} framed={true} />
        </div>
        <div className="min-w-0 flex-1">
          <h1
            className={`truncate text-sm font-semibold tracking-tight ${
              danger ? "text-warn" : "text-ink"
            }`}
            title={title}
          >
            {title}
          </h1>
          {subtitle && (
            <p className="mt-0.5 truncate text-xs text-dim" title={subtitle}>
              {subtitle}
            </p>
          )}
        </div>
        <div
          className="flex shrink-0 items-center gap-1"
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={defs.length}
          aria-valuenow={doneCount}
          aria-label={t.boot.progressAria.replace("{done}", String(doneCount)).replace("{total}", String(defs.length))}
        >
          {defs.map((_, i) => (
            <span
              key={i}
              className={`h-1.5 w-4 rounded-full transition-colors duration-300 ${
                SEG_CLS[steps[i]?.status ?? "pending"]
              }`}
            />
          ))}
        </div>
      </div>

      {/* 瞬态区块槽位（下载进度等） */}
      {banner && <div className="border-b border-line/70 px-5 py-4">{banner}</div>}

      {/* 步骤列表：一条连续导轨贯穿节点中线（节点 z-1 自带底色遮罩；
          首末节点中线 = 容器 p-2.5 + 行 py-2 + 节点 mt-0.5 + 半径 12 = 32px） */}
      <div className="relative flex flex-col gap-0.5 p-2.5">
        <div aria-hidden className="absolute bottom-8 left-[33px] top-8 w-0.5 bg-line" />
        {defs.map((def, i) => (
          <BootStep
            key={def.no}
            no={def.no}
            name={def.name}
            hint={def.hint}
            detail={steps[i]?.detail}
            status={steps[i]?.status ?? "pending"}
          />
        ))}
      </div>
    </section>
  )
}
