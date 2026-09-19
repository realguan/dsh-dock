import { clsx, type ClassValue } from "clsx"
import { extendTailwindMerge } from "tailwind-merge"

// 2026-09-19 裁定：把本仓库 @theme 的字号档位登记进 tailwind-merge 的 font-size 组。
//
// 复现的缺陷（MCP 弹窗 InfoTip 整块乌漆麻黑）：`cn("… bg-foreground text-background …",
// "text-label")` 里 **`text-background` 被 merge 掉了**。tailwind-merge 按 Tailwind
// 原生刻度认 `text-xs/sm/base` 为字号，本仓库的 `text-label` / `text-meta` 这类自定义
// 档名落进它的 **text-color 兜底组** ⇒ 判定"后来的颜色覆盖前面的颜色" ⇒ 真正的颜色类
// 被静默删除 ⇒ ink 底 + ink 字。typecheck / lint / contrast 闸门全绿：类名合法，只是
// 不在最终 class 串里——这类"合并即消失"的缺陷源码读不出来。
//
// 登记后 `text-<档位>` 归字号组，与文字颜色不再互斥。**新增字号档位必须同步这里**
// （闸门：__tests__/cnFontScale.test.ts 逐档断言两族共存）。
const twMerge = extendTailwindMerge({
  extend: {
    classGroups: {
      "font-size": [
        {
          text: [
            "micro",
            "meta",
            "label",
            "note",
            "body",
            "lead",
            // 2026-09-19 审美批次 1：后五个是 Tailwind 原生档名，本仓库已在
            // @theme 里改值覆盖。原生档 tailwind-merge 本就认识，登记属冗余，
            // 但 cnFontScale 的同步闸门按"index.css 每个 --text-* 都要在册"断言，
            // 冗余登记比给闸门开例外安全。
            "xs",
            "sm",
            "base",
            "lg",
            "xl",
          ],
        },
      ],
    },
  },
})

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}
