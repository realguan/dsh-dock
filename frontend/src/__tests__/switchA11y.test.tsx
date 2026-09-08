// switchA11y.test.tsx —— Switch 无障碍名称闸门的**类型级**回归测试（2026-09-08）。
//
// 闸门本体在 `ui/switch.tsx` 的 `SwitchProps`（类型层强制每个开关必须带
// `aria-label` 或 `aria-labelledby`）。本文件把它钉成可回归的形式：
// 若有人把名称改回可选，下面的 `@ts-expect-error` 会变成「未使用指令」，
// `pnpm typecheck` 立刻报错——不必引入 DOM 测试栈（AGENTS §5）。
import { describe, expect, it } from "vitest"
import { Switch } from "@/components/ui/switch"

const noop = () => {}

/** 正例：带无障碍名称，类型通过。 */
export const namedSwitch = (
  <Switch aria-label="自动滚底" checked onCheckedChange={noop} />
)

/** 反例：缺名称，类型系统必须拒绝（闸门存在性证明）。 */
// @ts-expect-error 缺 aria-label / aria-labelledby —— 见 ui/switch.tsx SwitchProps
export const unnamedSwitch = <Switch checked onCheckedChange={noop} />

describe("Switch 无障碍名称闸门", () => {
  it("正例可构造，名称落在 aria-label 上", () => {
    expect(namedSwitch.props["aria-label"]).toBe("自动滚底")
  })

  it("反例在类型层面被拒（否则上面的 @ts-expect-error 会失效并让 tsc 变红）", () => {
    expect(unnamedSwitch).toBeTruthy()
  })
})
