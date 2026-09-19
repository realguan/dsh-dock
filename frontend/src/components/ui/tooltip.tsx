"use client"

import * as React from "react"
import { Tooltip as TooltipPrimitive } from "radix-ui"

import { cn } from "@/lib/utils"

function TooltipProvider({
  delayDuration = 0,
  ...props
}: React.ComponentProps<typeof TooltipPrimitive.Provider>) {
  return (
    <TooltipPrimitive.Provider
      data-slot="tooltip-provider"
      delayDuration={delayDuration}
      {...props}
    />
  )
}

function Tooltip({
  ...props
}: React.ComponentProps<typeof TooltipPrimitive.Root>) {
  return <TooltipPrimitive.Root data-slot="tooltip" {...props} />
}

function TooltipTrigger({
  ...props
}: React.ComponentProps<typeof TooltipPrimitive.Trigger>) {
  return <TooltipPrimitive.Trigger data-slot="tooltip-trigger" {...props} />
}

function TooltipContent({
  className,
  sideOffset = 6,
  children,
  ...props
}: React.ComponentProps<typeof TooltipPrimitive.Content>) {
  return (
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Content
        data-slot="tooltip-content"
        sideOffset={sideOffset}
        // 2026-09-19：留 8px 视口安全边距——窄窗口下悬浮会紧贴甚至压住窗口边缘，
        // 读起来像被裁掉（Radix 默认 collisionPadding=0）。
        collisionPadding={8}
        className={cn(
          // 2026-09-19 重做悬浮面（截图证据：整块乌漆麻黑、字完全看不见）。三处根因：
          // ① 颜色类被 `cn()` 的 tailwind-merge 当同族挤掉（修在 lib/utils.ts）；
          // ② 旧口径 `bg-foreground text-background` 跨两个命名族，读的人无从判断这是
          //    一对——现统一走 **term 深色阅读面**（与运行日志同族 token，term-ink 压
          //    在 term 上 ≈ 15:1；非纯白字，长句不刺眼）；
          // ③ 零阴影 + `text-xs` 当正文：承载整段解释的浮层既没有层次也没有可读字号。
          //    现按「浮层 = shadow-lg 档 / 解释性长句 = 正文 text-note」给。
          // 入场动画：Radix 只写 `data-state`，旧 `data-open:` / `data-closed:` 变体永不
          // 命中（等于无动画）；本层只在打开时挂载，故 animate-in 直写，hover 与 focus
          // 两条路径同享 100ms 淡入。kbd 相关类随旧口径一并退役（本仓库悬浮不放 kbd）。
          "z-50 w-fit max-w-80 origin-(--radix-tooltip-content-transform-origin) rounded-lg bg-term px-3.5 py-2.5 text-left text-note leading-relaxed text-term-ink shadow-lg duration-100 animate-in fade-in-0 zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2",
          className
        )}
        {...props}
      >
        {children}
        <TooltipPrimitive.Arrow className="size-2.5 translate-y-[calc(-50%_-_2px)] rotate-45 rounded-[2px] fill-term" />
      </TooltipPrimitive.Content>
    </TooltipPrimitive.Portal>
  )
}

export { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger }
