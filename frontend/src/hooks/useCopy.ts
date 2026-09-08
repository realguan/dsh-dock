// useCopy.ts —— 剪贴板「已复制」反馈（2026-09-08，架构评审批次 0b）。
//
// 约定：**成功才置位**，失败不置位、由调用方按自身渠道提示（面板 → onNotice，
// 无 toast 的 boot 页 → logger）。不接收回调参数——回调会导致引用不稳
// （同 `ProfileManager` 的 onNotice 裁定，见 `__tests__/onNoticeStability.test.ts`）。
import { useCallback, useState } from "react"
import { writeClipboard, type CopyOutcome } from "@/lib/clipboard"

export function useCopy(resetMs = 2000) {
  // 单一状态：copied = 是否有成功写入；copiedKey = 该次写入的标识（按行场景用）。
  const [state, setState] = useState<{ key: string | null } | null>(null)

  const copy = useCallback(
    async (text: string, key: string | null = null): Promise<CopyOutcome> => {
      const outcome = await writeClipboard(text)
      if (!outcome.ok) return outcome
      setState({ key })
      window.setTimeout(
        () => setState((curr) => (curr?.key === key ? null : curr)),
        resetMs,
      )
      return outcome
    },
    [resetMs],
  )

  return { copied: state !== null, copiedKey: state?.key ?? null, copy }
}
