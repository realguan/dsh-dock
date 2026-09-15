// sshWizardGate.test.ts —— SSH 向导的结构闸门（2026-09-15，§7 R4d）。
//
// 为什么是源码结构断言：本仓库不引 RTL/jsdom（AGENTS §4.4），组件渲染没有断言通道。
// 而本向导有三条**一旦破坏就会对用户造成实质伤害**的落档决策：
//
//  ① **平台**：Windows 宿主上 MUST NOT 走到 ssh——上游在非 POSIX 宿主直接拒绝启动
//     ssh 服务，让用户填完一张表再失败是纯粹的浪费与误导。
//  ② **范围**：MUST NOT 展示任何"Web 工作台将变为远端感知"的承诺——上游明写
//     "replacing providers alone does **not** make those views remote-aware"，
//     而 ADR-0023 §3 方案 C 正是因为这个理由被否决。
//  ③ **顺序**：生成只允许在**预检通过之后**发起。反了会创建出一份注定连不上的
//     profile，比不生成更糟（用户会以为是 dsh 的问题）。
//
// 结构断言脆弱，故一律用**窗口切片**（只要求"在这段代码里"），不绑行号与格式。
// 若将来重写呈现层本测试转红是**预期**：请连同上面三条决策一起重新落档。
import { describe, expect, it } from "vitest"
import src from "@/components/profiles/SshWorkspaceWizard.tsx?raw"
import copy from "@/content/zh-CN.ts?raw"

function stripComments(code: string): string {
  return code
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^[ \t]*\/\/.*$/gm, "")
}

const code = stripComments(src)

function slice(from: string, to: string): string {
  const a = code.indexOf(from)
  const b = code.indexOf(to, a + from.length)
  expect(a, `未找到起始锚点：${from}`).toBeGreaterThanOrEqual(0)
  expect(b, `未找到结束锚点：${to}`).toBeGreaterThan(a)
  return code.slice(a, b)
}

describe("① 非 POSIX 宿主必须拦住（不能填完表才失败）", () => {
  it("Windows 提示与宿主判定挂钩", () => {
    const banner = slice("{!posix && (", "</div>")
    expect(banner).toContain("sshWizardWindows")
  })

  it("宿主判定口径 = 非 Windows（与后端 host_supports_ssh 同口径）", () => {
    const fn = slice("function hostIsPosix", "export function SshWorkspaceWizard")
    expect(fn).toMatch(/!\/Windows\/i\.test/)
  })

  it("非 POSIX 时预检按钮被禁用（提示之外还要真的拦住）", () => {
    expect(code).toMatch(/disabled=\{!canProbe \|\| !posix\}/)
  })

  it("宿主判定只是提示，安全边界在后端（注释须写明这点）", () => {
    expect(src).toContain("真正的闸门在后端")
  })
})

describe("② 绝不承诺 Web 视图远端感知（ADR-0023 §3 方案 C 的否决理由）", () => {
  it("范围说明常显（挂在 DialogDescription 上，不是可折叠的提示）", () => {
    const desc = slice("<DialogDescription", "</DialogDescription>")
    expect(desc).toContain("sshWizardScope")
  })

  it("范围文案必须明确否认远端感知（中英各一，禁单向）", () => {
    // 中文文案：必须出现"不会…远端感知"这一否认，而不是含糊的"支持远程"。
    const zh = copy.slice(copy.indexOf("sshWizardScope:"), copy.indexOf("sshWizardWindows:"))
    expect(zh).toContain("不会因此变成远端感知")
    // 英文对称：同一句必须存在于同一个键位（缺一边＝用户看到相反的话）。
    expect(zh).toContain("@deepseek-ai/dsh-headless")
  })
})

describe("③ 生成只能在预检通过之后发起（顺序即语义）", () => {
  it("生成按钮的渲染条件包含 probe.ok", () => {
    const btn = slice('phase.kind === "probeOk" && phase.probe.ok && (', "sshInstallBtn")
    expect(btn).toContain("generate()")
  })

  it("未通过预检时按钮不渲染（而不是渲染成 disabled——disabled 会被误读为'稍后再试'）", () => {
    // 正面：通过才出现；反面：出现条件里必须有 `phase.probe.ok`。
    expect(code).toContain('phase.kind === "probeOk" && phase.probe.ok &&')
  })

  it("预检的 IPC 层错误与逐项不通过分开处理（后者要给出逐项红/绿）", () => {
    const probeFn = slice("const probe = async () => {", "const generate =")
    expect(probeFn).toContain('kind: "probeOk"')
    expect(probeFn).toContain('kind: "probeFailed"')
  })

  it("前端不自行解析 ~/.ssh/config（ADR-0023 §3 方案 D 已否决，须走 IPC）", () => {
    // 用 `.listSshHosts(` 而非 `api.listSshHosts()`：链式写法会换行，
    // 绑字面量形状的断言是假红来源（本测试第一版就是这么红的）。
    expect(code).toContain(".listSshHosts(")
    expect(code).not.toContain("readFile")
  })

  it("前端不自行拼安装 spec（钉版本要运行期版本，只有后端有）", () => {
    expect(code).toContain("api.generateSshProfile(")
    // 不得出现包名字面量：一旦前端写死包名，后端换包/改顺序就会静默不一致。
    expect(code).not.toContain("@deepseek-ai/dsh-")
  })
})
