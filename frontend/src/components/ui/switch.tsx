import * as React from "react"
import { Switch as SwitchPrimitive } from "radix-ui"
import { cn } from "@/lib/utils"

type SwitchRootProps = React.ComponentProps<typeof SwitchPrimitive.Root>

/**
 * 无障碍名称强制（2026-09-08 裁定）：开关必须带 `aria-label` 或 `aria-labelledby`。
 *
 * 此前 8 处调用中 7 处缺失（`LogViewerPane` / `PreferencesPane`×2 / `PluginOverview` /
 * `McpManager` / `BuildApprovalDialog` / `ProfileDetailPane`），读屏用户只能听到
 * 「开关」而不知其义；第 8 处（`BootMode`）靠 `<label>` 包裹关联。
 * 用**类型闸门**钉住：新调用点漏写即 `pnpm typecheck` 红，不必引入 DOM 测试栈
 * （AGENTS §5）。文案取自相邻可见标签，勿写与界面不一致的自造名称。
 */
export type SwitchProps = SwitchRootProps &
  (
    | { "aria-label"?: string; "aria-labelledby": string }
    | { "aria-label": string; "aria-labelledby"?: string }
  )

function Switch({ className, ...props }: SwitchProps) {
  return (
    <SwitchPrimitive.Root
      data-slot="switch"
      className={cn(
        "peer inline-flex h-4 w-7 shrink-0 cursor-pointer items-center rounded-full border border-transparent transition-colors duration-200 outline-none focus-visible:ring-2 focus-visible:ring-brand focus-visible:ring-offset-1 disabled:cursor-not-allowed disabled:opacity-40 data-[state=checked]:bg-ok data-[state=unchecked]:bg-line data-[state=unchecked]:hover:bg-dim/30",
        className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb
        data-slot="switch-thumb"
        className={cn(
          "pointer-events-none block size-3 rounded-full bg-white shadow-xs ring-0 transition-transform duration-200 data-[state=checked]:translate-x-3.5 data-[state=unchecked]:translate-x-0.5",
        )}
      />
    </SwitchPrimitive.Root>
  )
}

export { Switch }
