// onNoticeStability.test.ts —— onNotice 引用稳定性机器闸门（2026-09-08）。
//
// 复现的缺陷（U1，死循环）：`ProfileManager` 曾以**内联箭头**传 onNotice
// （`onNotice={(msg, kind) => showToast(msg, kind)}`）→ 每次渲染都是新引用 →
// 子面板 `useCallback([onNotice])` 失效 → `useEffect` 重跑 → 加载失败又 onNotice
// → `setToast` → 父重渲染 → 回到第一步。受影响：SessionManager / CredentialsPane /
// DshSettingsPane / DiagnosticsPane / LogViewerPane（McpManager 已单独修过同坑）。
//
// 该缺陷对 tsc / oxlint **完全不可见**（内联箭头与稳定引用类型一致），只能靠源码
// 文本闸门兜底——与 `src-tauri/src/ipc.rs` 的 gate_tests 同口径（读源码、钉契约），
// 不引 DOM 测试栈（AGENTS §5）。
import { describe, expect, it } from "vitest"
import profileManagerSrc from "@/pages/ProfileManager.tsx?raw"

/** 内联函数表达式（含跨行）传 onNotice = 引用不稳定。 */
const INLINE_HANDLER = /onNotice=\{\s*(?:\(|function\b)/g

function offendingLines(src: string): number[] {
  const lines: number[] = []
  for (const m of src.matchAll(INLINE_HANDLER)) {
    lines.push(src.slice(0, m.index).split("\n").length)
  }
  return lines
}

describe("onNotice 引用稳定性", () => {
  it("ProfileManager 只传稳定引用（不得内联箭头/函数表达式）", () => {
    expect(
      offendingLines(profileManagerSrc),
      "这些行以内联函数传 onNotice，会让子面板 effect 每次渲染重跑（死循环）",
    ).toEqual([])
  })

  it("闸门自身有效：能识别内联写法（正反例各一）", () => {
    expect(
      offendingLines(`<SessionManager onNotice={(msg, kind) => showToast(msg, kind)} />`),
    ).toEqual([1])
    expect(
      offendingLines("<SessionManager onNotice={showToast} />"),
    ).toEqual([])
    // 跨行写法同样命中
    expect(
      offendingLines("<SessionManager\n  onNotice={(\n    msg,\n  ) => showToast(msg)}\n/>"),
    ).toEqual([2])
  })
})
