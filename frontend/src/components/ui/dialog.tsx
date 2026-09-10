"use client"

import * as React from "react"
import { Dialog as DialogPrimitive } from "radix-ui"

import { cn } from "@/lib/utils"
import { Button } from "@/components/ui/button"
import { XIcon } from "lucide-react"

function Dialog({
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Root>) {
  return <DialogPrimitive.Root data-slot="dialog" {...props} />
}

function DialogTrigger({
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Trigger>) {
  return <DialogPrimitive.Trigger data-slot="dialog-trigger" {...props} />
}

function DialogPortal({
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Portal>) {
  return <DialogPrimitive.Portal data-slot="dialog-portal" {...props} />
}

function DialogClose({
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Close>) {
  return <DialogPrimitive.Close data-slot="dialog-close" {...props} />
}

function DialogOverlay({
  className,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Overlay>) {
  return (
    <DialogPrimitive.Overlay
      data-slot="dialog-overlay"
      className={cn(
        "fixed inset-0 isolate z-50 bg-black/10 duration-100 supports-backdrop-filter:backdrop-blur-xs data-open:animate-in data-open:fade-in-0 data-closed:animate-out data-closed:fade-out-0",
        className
      )}
      {...props}
    />
  )
}

function DialogContent({
  className,
  children,
  showCloseButton = true,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Content> & {
  showCloseButton?: boolean
}) {
  return (
    <DialogPortal>
      <DialogOverlay />
      {/* 2026-09-10 定位层：grid place-items-center 布局层居中，弹窗自身回归文档流尺寸。
          前史：top/left-1/2 + -translate-1/2（合成层居中）在奇数视口/内容尺寸下必落半像素，
          弹窗被 WebKit 光栅化重采样后整卡文字发虚（外接 1x 屏尤甚）；改 inset-0 + m-auto +
          h-fit 后 WKWebView 对 absolute grid 容器的 fit-content 高度不收缩（整卡被拉满）。
          本层 pointer-events-none 放行点击到遮罩（outside-click 关闭不受影响），
          弹窗自身 pointer-events-auto 恢复交互；无常驻 transform，文字恒落物理像素。 */}
      <div className="pointer-events-none fixed inset-0 z-50 grid place-items-center">
        <DialogPrimitive.Content
          data-slot="dialog-content"
          className={cn(
            // max-h + 滚动：内容高于视口时容器内滚动，不溢出屏幕（2026-09-08，
            // ADR-0011 hotfix——fixed 居中定位下无约束会上下两端溢出且不可滚动）。
            // [&>*]:min-w-0：grid 隐式列 auto 轨道会被 nowrap 长内容（如安装源
            // spec 展示串）的固有宽度顶开，整行元素越出卡片右缘（2026-09-08
            // 弹窗横向截断修复；min-w-0 让轨道以容器宽度为准，深层由各自
            // truncate 收口）。
            "pointer-events-auto relative grid max-h-[calc(100dvh-2rem)] w-full max-w-[calc(100%-2rem)] overflow-y-auto gap-4 rounded-xl bg-popover p-4 text-sm text-popover-foreground ring-1 ring-foreground/10 duration-100 outline-none sm:max-w-sm [&>*]:min-w-0 data-open:animate-in data-open:fade-in-0 data-open:zoom-in-95 data-closed:animate-out data-closed:fade-out-0 data-closed:zoom-out-95",
            className
          )}
          {...props}
        >
          {children}
          {showCloseButton && (
            <DialogPrimitive.Close data-slot="dialog-close" asChild>
              <Button
                variant="ghost"
                className="absolute top-2 right-2"
                size="icon-sm"
              >
                <XIcon
                />
                <span className="sr-only">Close</span>
              </Button>
            </DialogPrimitive.Close>
          )}
        </DialogPrimitive.Content>
      </div>
    </DialogPortal>
  )
}

function DialogHeader({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="dialog-header"
      className={cn("flex flex-col gap-2", className)}
      {...props}
    />
  )
}

function DialogFooter({
  className,
  showCloseButton = false,
  children,
  ...props
}: React.ComponentProps<"div"> & {
  showCloseButton?: boolean
}) {
  return (
    <div
      data-slot="dialog-footer"
      className={cn(
        // 2026-08-28 裁定：静默 footer——去模板自带的 bg-muted/50 灰底 +
        // border-t 出血带。正文以白底 + 发丝线卡片为主语言，footer 再铺
        // 填充色块与卡片下边框贴出双线噪音；无底无线，按钮右缘与正文对齐，
        // 留白交给 DialogContent 自身节奏（前端迁移脚手架的库存样式，非
        // 有意设计，按 frontend-design skill「有意大于强度」原则收编）。
        "flex flex-col-reverse gap-2 sm:flex-row sm:justify-end",
        className
      )}
      {...props}
    >
      {children}
      {showCloseButton && (
        <DialogPrimitive.Close asChild>
          <Button variant="outline">Close</Button>
        </DialogPrimitive.Close>
      )}
    </div>
  )
}

function DialogTitle({
  className,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Title>) {
  return (
    <DialogPrimitive.Title
      data-slot="dialog-title"
      className={cn(
        "font-heading text-base leading-none font-medium",
        className
      )}
      {...props}
    />
  )
}

function DialogDescription({
  className,
  ...props
}: React.ComponentProps<typeof DialogPrimitive.Description>) {
  return (
    <DialogPrimitive.Description
      data-slot="dialog-description"
      className={cn(
        "text-sm text-muted-foreground *:[a]:underline *:[a]:underline-offset-3 *:[a]:hover:text-foreground",
        className
      )}
      {...props}
    />
  )
}

export {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogOverlay,
  DialogPortal,
  DialogTitle,
  DialogTrigger,
}
