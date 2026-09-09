"use client"

// ui/popover.tsx —— Radix Popover 薄封装（2026-09-09，问题记录-2026-09-09 §1.2）。
//
// 为什么立这一层：下载管理面板原先手写「useState + absolute 定位」下拉，只能点
// 触发按钮收起——点外部、按 ESC 都不关，与仓库内其它浮层（Select / Tooltip /
// Dialog，均由 Radix 提供 dismiss 语义）行为不一致。手写外点检测会重复实现
// 焦点归还 / 嵌套浮层 / 碰撞翻转等一堆细节，收口到原语层是唯一可持续做法。
//
// 样式基调沿用 tooltip.tsx（`data-open` / `data-closed` 是 index.css 里为
// Radix 注册的 @custom-variant，匹配 `[data-state="open"|"closed"]`）。
import * as React from "react"
import { Popover as PopoverPrimitive } from "radix-ui"

import { cn } from "@/lib/utils"

function Popover({
  ...props
}: React.ComponentProps<typeof PopoverPrimitive.Root>) {
  return <PopoverPrimitive.Root data-slot="popover" {...props} />
}

function PopoverTrigger({
  ...props
}: React.ComponentProps<typeof PopoverPrimitive.Trigger>) {
  return <PopoverPrimitive.Trigger data-slot="popover-trigger" {...props} />
}

function PopoverContent({
  className,
  align = "center",
  sideOffset = 4,
  collisionPadding = 8,
  ...props
}: React.ComponentProps<typeof PopoverPrimitive.Content>) {
  return (
    <PopoverPrimitive.Portal>
      <PopoverPrimitive.Content
        data-slot="popover-content"
        align={align}
        sideOffset={sideOffset}
        collisionPadding={collisionPadding}
        className={cn(
          "z-50 w-72 origin-(--radix-popover-content-transform-origin) rounded-xl border border-line bg-panel text-ink shadow-md outline-none data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95 data-closed:animate-out data-closed:fade-out-0 data-closed:zoom-out-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2",
          className,
        )}
        {...props}
      />
    </PopoverPrimitive.Portal>
  )
}

export { Popover, PopoverContent, PopoverTrigger }
