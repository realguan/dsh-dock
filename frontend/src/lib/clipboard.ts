// clipboard.ts —— 剪贴板写入的唯一入口（2026-09-08 裁定）。
//
// 历史缺陷（11 处，架构评审批次 0b）：要么完全不处理 promise（`ErrorCard` 写失败
// 仍显示「已复制」），要么 `.catch(() => {})` 吞掉后照样置位（`BootStep`），要么只挂
// `.then(...)` 成功分支 → 失败既无反馈、也留下 unhandled rejection。
//
// 统一收口：**成功才置「已复制」，失败必须由调用方上报**（面板 → onNotice，
// boot 页无 toast 渠道 → logger）。`write` 可注入，便于纯逻辑单测（AGENTS §5）。

export interface CopyOutcome {
  ok: boolean
  error?: unknown
}

/** 写入剪贴板；默认走 `navigator.clipboard`，失败不抛、以 `{ ok: false }` 返回。 */
export async function writeClipboard(
  text: string,
  write: (value: string) => Promise<void> = (value) =>
    navigator.clipboard.writeText(value),
): Promise<CopyOutcome> {
  try {
    await write(text)
    return { ok: true }
  } catch (error) {
    return { ok: false, error }
  }
}
