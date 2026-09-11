// bootWiring.test.ts —— 启动屏组件接线断言（2026-09-11，task-46）。
//
// 为什么需要这一层：纯函数测试绿 ≠ 组件真的传对了参数/调用了它。
// qa-verify 明确要求本层断言，否则须登记「组件接线未被测试覆盖」。
// 做法沿用仓库既有先例（`marketI18n.test.ts` / `bootSelectorCopy.test.ts` 的 `?raw`
// 源码断言），并**剥离注释**后再断言——注释里会引用旧写法/旧理由，不剥离即假阳性
// （task-20/24/25/27 反复踩过的教训）。
import { describe, expect, it } from "vitest"
import bootIndexSrc from "@/pages/BootIndex.tsx?raw"

/**
 * 剥离注释：JSX 块注释、普通块注释、行注释三类。
 * （本文件描述这三类时一律用文字，不写字面量，避免注释自身被提前终止。）
 */
function stripComments(src: string): string {
  return src
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^[ \t]*\/\/.*$/gm, "")
}

const code = stripComments(bootIndexSrc)
/** 压掉空白便于跨行结构断言（JSX 属性常换行书写）。 */
const flat = code.replace(/\s+/g, " ")

describe("F1：时间线收起接线", () => {
  it("采用三态用户意图（布尔表达不了「主动收起」）", () => {
    expect(code).toContain("useState<TimelineIntent>")
    expect(code).toContain("userIntent: timelineIntent")
    // 旧布尔写法不得回流（它使收起成为死按钮）
    expect(code, "旧的 forceShowTimeline 布尔疑似回流").not.toContain("forceShowTimeline")
  })

  it("收起按钮把意图置为 collapsed（而不是把布尔置回 false）", () => {
    expect(flat).toContain('setTimelineIntent("collapsed")')
  })

  it("「查看启动详情」把意图置为 expanded", () => {
    expect(flat).toContain('setTimelineIntent("expanded")')
  })

  it("两类意图都被传入纯函数判据（接线完整，判据不是摆设）", () => {
    expect(code).toContain("shouldShowTimeline(timelineVisibility)")
    expect(code).toContain("shouldOfferTimelineCollapse(timelineVisibility)")
  })
})

describe("F3：过期错误清理接线", () => {
  it("模式切换路径在调用 chooseMode 之前开新一轮（原地调用，store 存活）", () => {
    // 断言顺序：beginNewRound 必须出现在胶囊那处 chooseMode 之前。
    // 注意：源码里 `api` 与 `.chooseMode` 分行书写，压平后中间有一个空格。
    const capsuleIdx = flat.indexOf(".chooseMode(target, false)")
    expect(capsuleIdx, "未找到胶囊的 chooseMode 调用").toBeGreaterThan(-1)
    const before = flat.slice(Math.max(0, capsuleIdx - 260), capsuleIdx)
    expect(before, "切换模式前未清上一轮状态（v1.2.0 实测 2.1 根因）").toContain(
      "beginNewRound()",
    )
    expect(before).toContain("setLocalError(null)")
  })

  it("新一轮起跑时也清（覆盖非模式切换的原地重启路径）", () => {
    expect(code).toContain("shouldClearStaleError(")
    expect(code).toContain("beginNewRound()")
    // 边沿检测需要保存上一步态
    expect(code).toContain("prevStep0")
  })

  it("清理只经 store 的权威动作，不散落多份实现", () => {
    // beginNewRound 是唯一清零点；BootIndex 内不应直接写 error: null 之类
    expect(code).not.toMatch(/set\(\s*\{\s*error:\s*null/)
  })
})

describe("F3：诊断卡折叠出口接线", () => {
  it("ErrorCard 被传入（既有接线未破坏）", () => {
    expect(code).toContain("<ErrorCard")
  })
})
