// mcpProbeCard.test.ts —— MCP 探测结果卡的**结构闸门**（2026-09-15，§7 R1b）。
//
// 为什么是源码结构断言而不是 DOM 测试：本仓库口径明确不引 RTL/jsdom（AGENTS §4.4
// 末段），组件渲染没有断言通道。而这张卡承载两条**会被无意破坏**的落档决策：
//
//  ① **错误不得被折叠**。「探测失败」和「这个 server 没能力」是两件事，处置也完全
//     不同；把错误放进折叠体里等于默认把它藏起来，用户读到的结论正好相反。故断言
//     `mcpProbeFailed` 出现在 `if (!entry.ok)` 分支内、且该分支不含展开门 `open`。
//  ② **配置变了旧结果即失效**。保存/删除/换 profile 之后继续展示旧清单 = 拿旧数据
//     描述新配置（会给出**错误的事实**，不是"过时但无害"）。
//
// 结构断言脆弱，所以两处都用**窗口切片**而非全文 grep：只要求"在这段代码范围里"，
// 不绑定行号与格式。若将来重写呈现层，本测试红是**预期**——请连同上面两条决策一起
// 重新落档，而不是改断言让它变绿。
import { describe, expect, it } from "vitest"
import src from "@/components/profiles/McpManager.tsx?raw"

/** 去掉注释——注释里会引用被移除的调用，把注释当代码判红是假阳性。 */
function stripComments(code: string): string {
  return code
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^[ \t]*\/\/.*$/gm, "")
}

const code = stripComments(src)

/** 取 [from, to) 之间的一段（找不到任一端即抛——避免"窗口为空⇒断言空过"的假绿）。 */
function slice(from: string, to: string): string {
  const a = code.indexOf(from)
  const b = code.indexOf(to, a + from.length)
  expect(a, `未找到起始锚点：${from}`).toBeGreaterThanOrEqual(0)
  expect(b, `未找到结束锚点：${to}`).toBeGreaterThan(a)
  return code.slice(a, b)
}

describe("① 探测失败恒展开（不得被折叠藏起来）", () => {
  it("失败分支里直接渲染 mcpProbeFailed，且不经过展开门 open", () => {
    const failureBranch = slice("if (!entry.ok) {", "const { probe } = entry")
    expect(failureBranch).toContain("mcpProbeFailed")
    // 关键：错误文案不得被 `open` 门控。出现 `open` 即说明有人把它挪进了折叠体。
    expect(failureBranch).not.toMatch(/\bopen\b/)
  })

  it("mcpProbeFailed 在整份源码里只用于失败分支（没有第二处折叠渲染）", () => {
    const hits = code.split("mcpProbeFailed").length - 1
    expect(hits).toBe(1)
  })

  it("成功分支的清单体确实受 open 门控（对照面：不是把两者混成一谈）", () => {
    const successBody = slice("const { probe } = entry", "export function McpManager")
    expect(successBody).toContain("{open &&")
    expect(successBody).toContain("mcpProbeTools")
    expect(successBody).toContain("mcpProbeResources")
    expect(successBody).toContain("mcpProbeTemplates")
  })

  // ADR-0022 §5 明确要求「UI 必须标注快照时间」：一次探测不反映后续变更。
  // 若把快照时间挪进折叠体（或删掉），用户会把旧快照读成"当前能力"——
  // 这与 §① 的错误被折叠是同一类失败：都是让 UI 陈述一个它并不知道的事实。
  it("快照时间恒显（挂在表头，不随折叠消失）", () => {
    const successBody = slice("const { probe } = entry", "export function McpManager")
    const header = successBody.slice(0, successBody.indexOf("{open &&"))
    expect(header).toContain("mcpProbeAt")
    expect(header).toContain("fmtClock")
  })
})

describe("② 配置变更即丢弃旧探测结果", () => {
  it("保存与删除各调用一次 forgetProbe（紧跟 IPC 之后）", () => {
    const saveWindow = slice("await api.saveMcpServer", "onNotice?.(")
    expect(saveWindow).toContain("forgetProbe(srv.name)")

    const deleteWindow = slice("await api.deleteMcpServer", "onNotice?.(")
    expect(deleteWindow).toContain("forgetProbe(srvName)")

    // 恰好两处调用点：多出来的应当是**有意的**新决策，需要人重新读一遍。
    // （定义形如 `const forgetProbe = (`，不含 `forgetProbe(`，故不计数。）
    expect(code).toContain("const forgetProbe = (")
    expect(code.split("forgetProbe(").length - 1).toBe(2)
  })

  it("换 profile 时整批清空（同名 server 在不同 profile 配置可以不同）", () => {
    const reset = slice("useEffect(() => {", "}, [profileName])")
    expect(reset).toContain("setProbes({})")
    expect(reset).toContain("setProbeOpen({})")
  })
})

describe("③ 探测结果不再只经通知（R1a → R1b 的收口判据）", () => {
  it("handleProbe 不再调 onNotice（结果落卡片，通知是瞬时的、承载不了清单）", () => {
    const handler = slice("const handleProbe = async", "const forgetProbe")
    expect(handler).not.toContain("onNotice")
    expect(handler).toContain("setProbes(")
  })
})
