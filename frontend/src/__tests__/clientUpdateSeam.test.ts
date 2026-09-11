// clientUpdateSeam.test.ts —— **跨层 seam 契约测试**（2026-09-11，task-53 / T-F6）。
//
// ============================ 为什么有这条测试 ============================
// task-46 修好了「进度事件被丢」（补 `downloading → downloading`），但**两侧随后各自漂移**：
// rust-core 为「点了下载立刻有 loading」在 `blocked_check` 之前新增了 `Checking`，
// 而前端表 `checking: [available, upToDate, failed]` 不含 `downloading`
// ⇒ 整条进度序列从第一条起被判非法丢弃，用户界面**卡在「检查中」**（问题 3 换形式复发）。
//
// 两侧单测当时**都是绿的**：Rust 测的是 Rust 自己的相位模型，前端测的是前端表的自洽性——
// **没有任何一条测试跨过 IPC 边界**，比较「实际发射序」与「实际消费表」。
// 本文件就是那条缺失的测试：用**真实** `applyUpdateEvent` 回放**真实发射序**。
//
// ============ 同源声明（两侧改动 MUST 同步） ============
// 发射序锚定：`src-tauri/src/updater.rs::emitted_sequence_advances_monotonically`
// 与 `run_download_and_install` 的真实 `set_state` 调用位点（2026-09-11 核对）。
// **任何一侧改序/改表，都必须同步本文件**；本文件红了说明两侧又漂移了。
// ==========================================================================
import { describe, expect, it } from "vitest"
import { applyUpdateEvent, type UpdatePhase } from "@/stores/clientUpdateStore"
import type { AppUpdateEvent } from "@/types/events"

/**
 * 回放真实发射序，返回末态、进度采样、被丢弃的迁移。
 * 与 `useClientUpdateStore.dispatch` 同语义：非法迁移只记丢弃、状态不变。
 */
function replay(sequence: AppUpdateEvent[], from: UpdatePhase) {
  let phase: UpdatePhase = from
  const dropped: string[] = []
  const progress: number[] = []
  for (const e of sequence) {
    const next = applyUpdateEvent({ phase }, e)
    if (next === null) {
      dropped.push(`${phase}->${e.phase}`)
      continue
    }
    phase = next.phase
    if (next.phase === "downloading") progress.push(next.current ?? -1)
  }
  return { phase, dropped, progress }
}

/**
 * **Windows 真实发射序**（含 `Installing`：Windows 下载/安装分离，下载完成后才发）。
 * 同源：`updater.rs::emitted_sequence_advances_monotonically` 的 `seq`。
 */
const WINDOWS_SEQUENCE: AppUpdateEvent[] = [
  { phase: "checking" }, // 进入动作即发的可见态（blocked_check 之前）
  { phase: "downloading", current: 0, total: null },
  { phase: "downloading", current: 10, total: 100 },
  { phase: "downloading", current: 50, total: 100 },
  { phase: "downloading", current: 90, total: 100 },
  { phase: "installing" },
  { phase: "done", version: "1.3.0" },
  { phase: "relaunching" },
]

/**
 * **非 Windows 真实发射序**（**无 `Installing`**）。
 * 同源：`updater.rs::run_download_and_install` 的 `#[cfg(not(target_os = "windows"))]`
 * 分支注释——「非 Windows 走 `download_and_install` 一体化 API，**没有**『下载完成』
 * 这个可拦截的时刻 ⇒ 不能在中间插 `Installing`」。
 */
const NON_WINDOWS_SEQUENCE: AppUpdateEvent[] = [
  { phase: "checking" },
  { phase: "downloading", current: 0, total: null },
  { phase: "downloading", current: 10, total: 100 },
  { phase: "downloading", current: 50, total: 100 },
  { phase: "downloading", current: 90, total: 100 },
  { phase: "done", version: "1.3.0" },
  { phase: "relaunching" },
]

describe("跨层 seam：真实发射序不得被前端丢弃", () => {
  it("Windows 路径：零丢弃 + 末态 relaunching + 进度采样 ≥2 点", () => {
    const { phase, dropped, progress } = replay(WINDOWS_SEQUENCE, "available")
    // 这三条断言就是 seam 的机器判据：
    expect(dropped, "新发射序不得被前端丢弃").toEqual([])
    expect(phase, "末态应为 relaunching").toBe("relaunching")
    expect(progress.length, "进度采样应 ≥2 点（否则进度条不推进）").toBeGreaterThanOrEqual(2)
    expect(progress).toEqual([0, 10, 50, 90])
  })

  it("非 Windows 路径（无 Installing）：零丢弃 + 末态 relaunching + 进度采样 ≥2 点", () => {
    const { phase, dropped, progress } = replay(NON_WINDOWS_SEQUENCE, "available")
    expect(dropped, "非 Windows 序不得被前端丢弃（下一处潜在 seam 破口）").toEqual([])
    expect(phase).toBe("relaunching")
    expect(progress.length).toBeGreaterThanOrEqual(2)
    expect(progress).toEqual([0, 10, 50, 90])
  })

  it("逐对相位在真实序内合法（与 Rust transition_allowed 同判据）", () => {
    // 把「序」本身当断言对象：windows 序里每一对都必须被前端接受
    for (const [label, seq] of [
      ["windows", WINDOWS_SEQUENCE],
      ["non-windows", NON_WINDOWS_SEQUENCE],
    ] as const) {
      let phase: UpdatePhase = "available"
      for (const e of seq) {
        const next = applyUpdateEvent({ phase }, e)
        expect(next, `${label}: ${phase} → ${e.phase} 被前端拒绝`).not.toBeNull()
        phase = e.phase as UpdatePhase
      }
    }
  })

  it("首条进度事件即被接受（用户点完「下载」立刻要有进度条）", () => {
    // 这条单独钉住 task-53 的破口：checking → downloading
    const first = applyUpdateEvent({ phase: "checking" }, { phase: "downloading", current: 0, total: null })
    expect(first, "checking → downloading 必须放行（否则整条进度序列从第一条起全丢）").not.toBeNull()
  })

  it("守卫未被放宽成万能表（保留防万能化断言）", () => {
    // 关键：放宽的同时必须留边界，否则任意跳转都能过、表就失去意义
    expect(applyUpdateEvent({ phase: "idle" }, { phase: "done", version: "1" })).toBeNull()
    expect(applyUpdateEvent({ phase: "idle" }, { phase: "downloading", current: 1 })).toBeNull()
    expect(applyUpdateEvent({ phase: "available" }, { phase: "relaunching" })).toBeNull()
    // Rust 侧不变式：installing → downloading 是**倒退**，必须拒
    // （`updater.rs::reverse_transition_is_rejected` 明文断言）
    expect(
      applyUpdateEvent({ phase: "installing" }, { phase: "downloading", current: 1 }),
      "installing → downloading 是倒退，Rust 侧明文拒绝（reverse_transition_is_rejected）",
    ).toBeNull()
  })

  it("幂等重复相位不会冻结进度（同源规则 1：同相位合法）", () => {
    // Rust `transition_allowed` 规则 1：from == to 幂等允许（进度天然重复）
    const { dropped, progress } = replay(
      [
        { phase: "downloading", current: 1, total: 100 },
        { phase: "downloading", current: 2, total: 100 },
        { phase: "downloading", current: 3, total: 100 },
      ],
      "downloading",
    )
    expect(dropped).toEqual([])
    expect(progress).toEqual([1, 2, 3])
  })
})
