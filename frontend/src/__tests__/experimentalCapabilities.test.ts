// experimentalCapabilities.test.ts —— 实验能力开/关/换后端的**编排纯逻辑**测试
//（2026-09-16，ADR-0020 §7）。
//
// 覆盖的是**风险核心**（每条都对应一个真实故障面）：
//   · 顺序即语义：同族旧后端必须先让位；写行必须紧跟自己的那一步安装；
//   · 软失败必须当失败：`ok:false` 是常态（pnpm 审批门/包不存在），漏判 = 界面报成功；
//   · 失败可续跑：停在一致态，`completedOps` 是"已完成的步数"；
//   · 关闭 ≠ 卸载：disable 只写行，移除才删行卸包；**删行必须在卸包之前**（否则悬空挂载行）；
//   · 换后端要留住两变体共享的基座包。
// 按仓库惯例只测纯逻辑（注入式 IO），不引 DOM 测试栈（AGENTS §4.3 末段）。

import { describe, expect, it } from "vitest"

import {
  commonPrerequisites,
  commonSharedPackages,
  offMeansRemove,
  planDisable,
  planEnable,
  planRemove,
  planReplace,
  runCapabilityOps,
  variantDisplayName,
  variantPackageRoles,
  type CapabilityIO,
  type CapabilityOp,
} from "@/lib/experimentalCapabilities"
import type {
  Capability,
  CapabilityStep,
  CapabilityVariant,
  PluginOpOutcome,
  RowWriteOutcome,
} from "@/types/ipc"

const ok = (detail = "done"): PluginOpOutcome => ({ ok: true, detail })
const softFail = (detail: string): PluginOpOutcome => ({ ok: false, detail })
const rowOutcome = (autoActivated = false): RowWriteOutcome => ({
  changed: !autoActivated,
  autoActivated,
})

const BASE = "@deepseek-ai/dsh-browser-use"
const PLAYWRIGHT = "@deepseek-ai/dsh-experimental-browser-use-playwright-mcp"
const DEVTOOLS = "@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp"
const REVIEW = "@deepseek-ai/dsh-experimental-auto-review"

function step(over: Partial<CapabilityStep> = {}): CapabilityStep {
  const merged: CapabilityStep = {
    ordinal: 1,
    package: PLAYWRIGHT,
    activation: "insert_row",
    rowId: "dsh-dock-pw",
    installed: false,
    rowPresent: false,
    disabled: false,
    toggleTargets: [],
    versionNotice: null,
    description: null,
    ...over,
    // spec 跟着 package 走（除非显式覆盖）——否则"第二步"的期望值会带着第一步的包名，
    // 测出来的顺序就成了假的。
    spec: over.spec ?? `${over.package ?? PLAYWRIGHT}@0.1.6-alpha.1`,
  }
  return merged
}

function variant(over: Partial<CapabilityVariant> = {}): CapabilityVariant {
  return {
    id: "playwright",
    labelZh: "Playwright",
    noteZh: "通用浏览器自动化后端",
    prerequisitesZh: [],
    steps: [step()],
    state: "off",
    subsumedBy: null,
    toggleOffSupported: true,
    displaced: [],
    prerequisiteMissing: null,
    ...over,
  }
}

function capability(over: Partial<Capability> = {}): Capability {
  return {
    id: "browser-use",
    labelZh: "浏览器操作",
    summaryZh: "让模型自己开浏览器",
    unlocksZh: "多出一组浏览器工具",
    variants: [variant()],
    state: "off",
    activeVariant: null,
    // 默认"由 dock 策展"；dsh 自带那一档另有专门的闸门（`official_catalog::dsh_shipped_...`）。
    shippedByDsh: false,
    legacyCopy: false,
    ...over,
  }
}

/** 记录调用顺序的假 IO——串行性、次序语义都靠它证伪。 */
function fakeIO(overrides: Partial<CapabilityIO> = {}): {
  io: CapabilityIO
  calls: string[]
} {
  const calls: string[] = []
  const io: CapabilityIO = {
    install: async (_p, pkg, spec) => {
      calls.push(`install:${spec || pkg}`)
      return ok()
    },
    remove: async (_p, pkg) => {
      calls.push(`remove:${pkg}`)
      return ok()
    },
    ensureRow: async (_p, rowId) => {
      calls.push(`ensureRow:${rowId}`)
      return rowOutcome()
    },
    deleteRow: async (_p, rowId) => {
      calls.push(`deleteRow:${rowId}`)
      return true
    },
    setDisabled: async (_p, rowId, disabled) => {
      calls.push(`setDisabled:${rowId}=${disabled}`)
    },
    ...overrides,
  }
  return { io, calls }
}

/** 两步的后端变体：基座包 + provider（顺序即语义）。 */
function twoStepVariant(over: Partial<CapabilityVariant> = {}): CapabilityVariant {
  return variant({
    steps: [
      step({ ordinal: 1, package: BASE, rowId: "dsh-dock-base" }),
      step({ ordinal: 2, package: PLAYWRIGHT, rowId: "dsh-dock-pw" }),
    ],
    ...over,
  })
}

describe("开启（也用于修复）：只做缺的那部分", () => {
  it("逐包安装 → 写行紧跟自己的那一步 → 最后才解除停用", () => {
    const cap = capability({ variants: [twoStepVariant()] })

    expect(planEnable(cap, "playwright")).toEqual([
      { kind: "install", step: 1, package: BASE, spec: `${BASE}@0.1.6-alpha.1` },
      { kind: "ensureRow", step: 1, package: BASE, rowId: "dsh-dock-base" },
      { kind: "install", step: 2, package: PLAYWRIGHT, spec: `${PLAYWRIGHT}@0.1.6-alpha.1` },
      { kind: "ensureRow", step: 2, package: PLAYWRIGHT, rowId: "dsh-dock-pw" },
    ])
  })

  it("`planEnable` **不得**自己拆同族旧后端：它只会 remove 不删行，会留下悬空挂载行", () => {
    // 让位职责归 planReplace（删行 + 卸包成对）。这条是回归护栏：
    // 曾经这里发 `remove` 就完事，卸了包而 `- insert:` 行还在 → dsh 启动加载失败。
    const cap = capability({ variants: [twoStepVariant({ displaced: [DEVTOOLS] })] })
    const ops = planEnable(cap, "playwright")

    expect(ops.map((o) => o.kind)).toEqual(["install", "ensureRow", "install", "ensureRow"])
    expect(ops.some((o) => o.kind === "remove")).toBe(false)
  })

  it("已装的包不重装、行齐的不重写（修复路径只补缺口）", () => {
    const cap = capability({
      variants: [
        twoStepVariant({
          state: "partial",
          // 基座包已装且行在；provider 装了但行缺（典型的"装了却没生效"）。
          steps: [
            step({
              ordinal: 1,
              package: BASE,
              rowId: "dsh-dock-base",
              installed: true,
              rowPresent: true,
            }),
            step({ ordinal: 2, package: PLAYWRIGHT, installed: true }),
          ],
        }),
      ],
    })

    expect(planEnable(cap, "playwright")).toEqual([
      { kind: "ensureRow", step: 2, package: PLAYWRIGHT, rowId: "dsh-dock-pw" },
    ])
  })

  it("已就位但停用 → 只需解除停用（不重装、不重写行）", () => {
    const cap = capability({
      variants: [
        twoStepVariant({
          state: "disabled",
          steps: [
            step({
              ordinal: 1,
              package: BASE,
              rowId: "dsh-dock-base",
              installed: true,
              rowPresent: true,
              disabled: true,
              toggleTargets: ["dsh-dock-base"],
            }),
            step({
              ordinal: 2,
              package: PLAYWRIGHT,
              installed: true,
              rowPresent: true,
              disabled: true,
              toggleTargets: ["dsh-dock-pw"],
            }),
          ],
        }),
      ],
    })

    expect(planEnable(cap, "playwright")).toEqual([
      { kind: "setDisabled", rowId: "dsh-dock-base", disabled: false },
      { kind: "setDisabled", rowId: "dsh-dock-pw", disabled: false },
    ])
  })

  it("乱序输入按 ordinal 纠正（顺序是语义，不依赖数组恰好有序）", () => {
    const cap = capability({
      variants: [
        twoStepVariant({
          steps: [
            step({ ordinal: 2, package: PLAYWRIGHT, rowId: "dsh-dock-pw" }),
            step({ ordinal: 1, package: BASE, rowId: "dsh-dock-base" }),
          ],
        }),
      ],
    })
    const kinds = planEnable(cap, "playwright").map((op) =>
      op.kind === "setDisabled" ? op.kind : `${op.kind}:${op.step}`,
    )
    expect(kinds).toEqual(["install:1", "ensureRow:1", "install:2", "ensureRow:2"])
  })
})

describe("关闭 = 只写行级 disabled（不卸载）", () => {
  it("bundle 贡献多行时全部一起切（与「已安装」页的开关同口径）", () => {
    const v = variant({
      id: "standard",
      toggleOffSupported: true,
      steps: [
        step({
          package: REVIEW,
          activation: "auto_bundle",
          installed: true,
          rowPresent: true,
          toggleTargets: ["review-a", "review-b"],
        }),
      ],
    })
    expect(planDisable(v)).toEqual([
      { kind: "setDisabled", rowId: "review-a", disabled: true },
      { kind: "setDisabled", rowId: "review-b", disabled: true },
    ])
  })

  it("含 profile 层的变体**拒绝**行级关闭（层的副作用回滚不了，必须走移除）", () => {
    const v = variant({ id: "web", toggleOffSupported: false })
    expect(() => planDisable(v)).toThrow(/须走移除/)
  })
})

describe("移除：清桩 → 删行 → 逆序卸包", () => {
  it("删行必须早于卸包，且依赖者先卸（Web 层在宿主层之前）", () => {
    const v = twoStepVariant({
      state: "on",
      steps: [
        step({
          ordinal: 1,
          package: BASE,
          rowId: "dsh-dock-base",
          installed: true,
          rowPresent: true,
        }),
        step({
          ordinal: 2,
          package: PLAYWRIGHT,
          installed: true,
          rowPresent: true,
        }),
      ],
    })
    const cap = capability({ variants: [v] })

    expect(planRemove(cap, "playwright")).toEqual([
      { kind: "deleteRow", step: 1, package: BASE, rowId: "dsh-dock-base" },
      { kind: "deleteRow", step: 2, package: PLAYWRIGHT, rowId: "dsh-dock-pw" },
      { kind: "remove", step: 2, package: PLAYWRIGHT },
      { kind: "remove", step: 1, package: BASE },
    ])
  })

  it("停用态移除时先收回停用桩（否则留下指向空行 id 的垃圾）", () => {
    const v = variant({
      id: "standard",
      toggleOffSupported: false,
      state: "disabled",
      steps: [
        step({
          package: REVIEW,
          activation: "auto_bundle",
          installed: true,
          rowPresent: true,
          disabled: true,
          toggleTargets: ["review-a"],
        }),
      ],
    })
    const cap = capability({ variants: [v] })
    // bundle 的行随包消失，壳不代删；但仍要收回自己写的停用桩。
    expect(planRemove(cap, "standard")).toEqual([
      { kind: "setDisabled", rowId: "review-a", disabled: false },
      { kind: "remove", step: 1, package: REVIEW },
    ])
  })

  it("没装过、也没有行的变体 → 移除是空计划（幂等，不误报）", () => {
    const cap = capability({ variants: [variant()] })
    expect(planRemove(cap, "playwright")).toEqual([])
  })

  it("未知变体直接抛错（不得静默返回空计划）", () => {
    expect(() => planRemove(capability(), "nope")).toThrow(/没有变体/)
  })
})

describe("换后端：留住共享基座，只拆旧后端独占的部分", () => {
  it("基座包不作为共享依赖被卸掉，随后开启目标变体", () => {
    const cap = capability({
      state: "on",
      activeVariant: "chrome-devtools",
      variants: [
        twoStepVariant({
          id: "playwright",
          steps: [
            // 共享基座包已由"旧后端"那一轮装好（换后端不该重装它）。
            step({
              ordinal: 1,
              package: BASE,
              rowId: "dsh-dock-base",
              installed: true,
              rowPresent: true,
            }),
            step({ ordinal: 2, package: PLAYWRIGHT, rowId: "dsh-dock-pw" }),
          ],
        }),
        twoStepVariant({
          id: "chrome-devtools",
          state: "on",
          steps: [
            step({
              ordinal: 1,
              package: BASE,
              rowId: "dsh-dock-base",
              installed: true,
              rowPresent: true,
            }),
            step({
              ordinal: 2,
              package: DEVTOOLS,
              rowId: "dsh-dock-dt",
              installed: true,
              rowPresent: true,
            }),
          ],
        }),
      ],
    })

    const ops = planReplace(cap, "playwright")
    const calls = ops.map((op: CapabilityOp) =>
      op.kind === "install"
        ? `install:${op.package}`
        : op.kind === "remove"
          ? `remove:${op.package}`
          : `${op.kind}:${op.rowId}`,
    )
    // 旧后端独占的包与行被拆掉，基座包既不被卸也不重复装（它已装且行在）。
    expect(calls).toEqual([
      "deleteRow:dsh-dock-dt",
      "remove:" + DEVTOOLS,
      "install:" + PLAYWRIGHT,
      "ensureRow:dsh-dock-pw",
    ])
    expect(calls.some((c) => c.includes(`remove:${BASE}`))).toBe(false)
    expect(calls.some((c) => c.includes(`install:${BASE}`))).toBe(false)
  })
})

  it("两个后端同时生效（冲突）时，修复会把**另一个**也拆掉——不只拆选中之外的那一个", () => {
    const cap = capability({
      state: "conflict",
      activeVariant: null,
      variants: [
        variant({ id: "playwright", steps: [step({ package: PLAYWRIGHT, installed: false })] }),
        variant({
          id: "chrome-devtools",
          state: "on",
          steps: [
            step({ package: DEVTOOLS, rowId: "dsh-dock-dt", installed: true, rowPresent: true }),
          ],
        }),
        variant({
          id: "stagehand",
          state: "on",
          steps: [
            step({
              package: "@deepseek-ai/dsh-experimental-browser-use-stagehand-native",
              rowId: "dsh-dock-sh",
              installed: true,
              rowPresent: true,
            }),
          ],
        }),
      ],
    })

    const calls = planReplace(cap, "playwright").map((op) =>
      op.kind === "install" ? `install:${op.package}` : `${op.kind}:${"package" in op ? op.package : op.rowId}`,
    )
    // 两个在生效的后端都要先让位（否则同族并存照样激活失败），再装目标后端。
    expect(calls).toEqual([
      "deleteRow:" + DEVTOOLS,
      "remove:" + DEVTOOLS,
      "deleteRow:@deepseek-ai/dsh-experimental-browser-use-stagehand-native",
      "remove:@deepseek-ai/dsh-experimental-browser-use-stagehand-native",
      "install:" + PLAYWRIGHT,
      // 装完紧跟自己的那一步写行（行缺失才写）。
      "ensureRow:" + PLAYWRIGHT,
    ])
  })

describe("串行执行器：软失败当失败、失败可续跑", () => {
  const ops: CapabilityOp[] = [
    { kind: "install", step: 1, package: BASE, spec: `${BASE}@1.0.0` },
    { kind: "ensureRow", step: 1, package: BASE, rowId: "dsh-dock-base" },
    { kind: "install", step: 2, package: PLAYWRIGHT, spec: `${PLAYWRIGHT}@1.0.0` },
  ]

  it("按序执行并逐条回报进度", async () => {
    const { io, calls } = fakeIO()
    const seen: string[] = []
    const result = await runCapabilityOps(io, "p", ops, (p) =>
      seen.push(`${p.index + 1}/${p.total}:${p.op.kind}`),
    )

    expect(result).toEqual({ ok: true, completedOps: 3, totalOps: 3, failedAt: null })
    expect(calls).toEqual([
      `install:${BASE}@1.0.0`,
      "ensureRow:dsh-dock-base",
      `install:${PLAYWRIGHT}@1.0.0`,
    ])
    expect(seen).toEqual(["1/3:install", "2/3:ensureRow", "3/3:install"])
  })

  it("`ok:false` 是失败而不是成功（漏判 = 界面报成功、实际没装上）", async () => {
    const { io } = fakeIO({
      ensureRow: async () => {
        throw new Error("写入后复核未通过：行 id「dsh-dock-base」不在组合树中")
      },
    })
    const result = await runCapabilityOps(io, "p", ops)

    expect(result.ok).toBe(false)
    expect(result.completedOps).toBe(1)
    expect(result.failedAt?.index).toBe(1)
    expect(result.failedAt?.error).toContain("复核未通过")
  })

  it("安装软失败即刻停止：后续步骤不会被误当成做完", async () => {
    const { io, calls } = fakeIO({
      install: async (_p, pkg, spec) => {
        calls.push(`install:${spec}`)
        return pkg === PLAYWRIGHT ? softFail("需要人工审批构建脚本") : ok()
      },
    })
    const result = await runCapabilityOps(io, "p", ops)

    expect(result.ok).toBe(false)
    expect(result.failedAt?.error).toContain("需要人工审批构建脚本")
    // 失败那一步**不算完成**（completedOps 是"已一致的步数"，续跑从它开始）。
    expect(result.completedOps).toBe(2)
    expect(result.failedAt?.index).toBe(2)
    expect(calls).toEqual([
      `install:${BASE}@1.0.0`,
      "ensureRow:dsh-dock-base",
      `install:${PLAYWRIGHT}@1.0.0`,
    ])
  })

  it("空计划视为成功（移除一个从未启用的能力不该报错）", async () => {
    const { io } = fakeIO()
    await expect(runCapabilityOps(io, "p", [])).resolves.toEqual({
      ok: true,
      completedOps: 0,
      totalOps: 0,
      failedAt: null,
    })
  })
})

// 2026-09-17 维护者裁定「实际上就是插件，名字使用插件名就行了」：界面用**插件名**称呼
// 后端，并区分"这一档独有的包"与"兄弟档共用的基座"。规则纯函数化，正反例都钉住——
// 拿这条去渲染卡片头与确认框，就不会再冒出「Web 档 / 复用已装的 cua-driver」这类
// 我们发明的名字。
describe("变体的标识包与共用包（界面按插件名说话）", () => {
  it("基座 + provider 的两步档：标识 = provider，共用 = 基座", () => {
    const cap = capability({
      variants: [
        twoStepVariant({ id: "playwright" }),
        twoStepVariant({
          id: "chrome-devtools",
          steps: [
            step({ ordinal: 1, package: BASE, rowId: "dsh-dock-base" }),
            step({ ordinal: 2, package: DEVTOOLS, rowId: "dsh-dock-dt" }),
          ],
        }),
      ],
    })
    expect(variantPackageRoles(cap, "playwright")).toEqual({
      primary: [PLAYWRIGHT],
      shared: [BASE],
    })
    expect(variantDisplayName(cap, "playwright")).toBe(PLAYWRIGHT)
  })

  it("只有基座的档（Agent Teams 自建档那种）：标识 = 基座本身，不显示成「什么都没有」", () => {
    const cap = capability({
      variants: [
        variant({ id: "web", steps: [step({ package: BASE }), step({ ordinal: 2, package: PLAYWRIGHT })] }),
        variant({ id: "headless", steps: [step({ package: BASE })] }),
      ],
    })
    expect(variantPackageRoles(cap, "headless")).toEqual({ primary: [BASE], shared: [] })
    expect(variantPackageRoles(cap, "web")).toEqual({ primary: [PLAYWRIGHT], shared: [BASE] })
  })

  it("单档能力：整包即标识（无兄弟 ⇒ 没有共用概念）", () => {
    const cap = capability({ id: "auto-review", variants: [variant({ id: "standard", steps: [step({ package: REVIEW })] })] })
    expect(variantPackageRoles(cap, "standard")).toEqual({ primary: [REVIEW], shared: [] })
    expect(variantDisplayName(cap, "standard")).toBe(REVIEW)
  })

  it("未知变体：空结果，不抛（渲染路径上的防御）", () => {
    expect(variantPackageRoles(capability(), "nope")).toEqual({ primary: [], shared: [] })
    expect(variantDisplayName(capability(), "nope")).toBe("")
  })

  it("包序沿用步骤序（宿主层在前），不随兄弟变体数量变化", () => {
    const three = capability({
      variants: [
        twoStepVariant({ id: "a" }),
        twoStepVariant({ id: "b", steps: [step({ package: BASE }), step({ ordinal: 2, package: DEVTOOLS })] }),
        twoStepVariant({ id: "c", steps: [step({ package: BASE }), step({ ordinal: 2, package: REVIEW })] }),
      ],
    })
    expect(variantPackageRoles(three, "a").primary).toEqual([PLAYWRIGHT])
    expect(variantPackageRoles(three, "a").shared).toEqual([BASE])
  })
})

// 2026-09-17 版面复盘：browser-use 三档都是「基座 + 各自 provider」，"另装共用包"在三条
// 变体行里原样重复三遍——和共同前置是同一类噪音，同一类修法（提到面板级只讲一次）。
// 2026-09-17 独立复核：这条判据曾在 `offIsRemove` 一处带守卫、另三处裸读
// `toggleOffSupported` → 一次失败的安装就能让面板谎称"该能力由 profile 层提供"（假话）。
describe("层类判据：关闭是否等价于移除，必须连带确认「真就位」", () => {
  const layer = (state: CapabilityVariant["state"], toggleOffSupported: boolean) =>
    variant({ id: "layer", state, toggleOffSupported })

  it("真就位的层类档（on / disabled）→ 关闭即移除", () => {
    expect(offMeansRemove(layer("on", false))).toBe(true)
    expect(offMeansRemove(layer("disabled", false))).toBe(true)
  })

  it("装到一半（partial）**不算**层类：那时后端根本无从判定包的形态", () => {
    // `toggle_off_supported` 对未装齐的档恒为 false（要读包自己的 manifest），
    // 照它渲染就会把 browser-use 这种纯 insert_row 档说成"由 profile 层提供"。
    expect(offMeansRemove(layer("partial", false))).toBe(false)
    expect(offMeansRemove(layer("off", false))).toBe(false)
  })

  it("纯行级档（toggleOffSupported=true）永远不是层类", () => {
    expect(offMeansRemove(layer("on", true))).toBe(false)
    expect(offMeansRemove(layer("disabled", true))).toBe(false)
  })
})

describe("共用基座包：全体共有时只讲一次，否则保持行内标注", () => {
  const threeVariants = (thirdPackage: string) =>
    capability({
      variants: [
        twoStepVariant({ id: "a" }),
        twoStepVariant({
          id: "b",
          steps: [step({ package: BASE }), step({ ordinal: 2, package: DEVTOOLS })],
        }),
        twoStepVariant({
          id: "c",
          steps: [step({ package: BASE }), step({ ordinal: 2, package: thirdPackage })],
        }),
      ],
    })

  it("三档都带同一个基座 → 提到面板级，行内的重复被扣掉", () => {
    const cap = threeVariants(REVIEW)
    expect(commonSharedPackages(cap)).toEqual([BASE])
    const inline = (id: string) =>
      variantPackageRoles(cap, id).shared.filter((p) => !commonSharedPackages(cap).includes(p))
    expect(inline("a")).toEqual([])
    expect(inline("b")).toEqual([])
    // 标识包不受影响：扣掉的只是"共用"那一份。
    expect(variantDisplayName(cap, "b")).toBe(DEVTOOLS)
  })

  it("不是全体共有（Agent Teams 自建档那种）→ 交集为空，继续按行标注，不吞信息", () => {
    const cap = capability({
      variants: [
        variant({
          id: "web",
          steps: [step({ package: BASE }), step({ ordinal: 2, package: PLAYWRIGHT })],
        }),
        variant({ id: "headless", steps: [step({ package: BASE })] }),
      ],
    })
    expect(commonSharedPackages(cap)).toEqual([])
    // Web 档照样如实标出"另装共用包 = 基座"。
    expect(variantPackageRoles(cap, "web").shared).toEqual([BASE])
  })

  it("单档能力：没有「共用」这回事（整包即标识）", () => {
    const cap = capability({
      id: "auto-review",
      variants: [variant({ id: "standard", steps: [step({ package: REVIEW })] })],
    })
    expect(commonSharedPackages(cap)).toEqual([])
  })
})

describe("共同前置：卡片只讲一次，行内只讲各档额外的", () => {
  it("三档共有的 Chromium 前置只出现一次（交集），各档独有前置留在行内", () => {
    const cap = capability({
      variants: [
        variant({ id: "playwright", prerequisitesZh: ["浏览器只用 Chromium 系"] }),
        variant({
          id: "chrome-devtools",
          prerequisitesZh: ["浏览器只用 Chromium 系", "本机需安装 Chrome"],
        }),
        variant({ id: "stagehand", prerequisitesZh: ["浏览器只用 Chromium 系", "需填 model"] }),
      ],
    })
    expect(commonPrerequisites(cap)).toEqual(["浏览器只用 Chromium 系"])
    const extras = (id: string) =>
      cap.variants.find((v) => v.id === id)!.prerequisitesZh.filter((p) => !commonPrerequisites(cap).includes(p))
    expect(extras("playwright")).toEqual([])
    expect(extras("chrome-devtools")).toEqual(["本机需安装 Chrome"])
    expect(extras("stagehand")).toEqual(["需填 model"])
  })

  it("没有共同前置时交集为空（单档能力、各说各的）", () => {
    expect(commonPrerequisites(capability({ variants: [variant({ prerequisitesZh: ["a"] })] }))).toEqual(["a"])
    const two = capability({
      variants: [variant({ id: "a", prerequisitesZh: ["x"] }), variant({ id: "b", prerequisitesZh: ["y"] })],
    })
    expect(commonPrerequisites(two)).toEqual([])
  })
})

describe("失败分类归后端：前端只消费，不再自己写第二份正则（AGENTS §11.3 禁双源）", () => {
  it("`PluginOpOutcome.failureKind` 经队列一路带到 `failedAt`", async () => {
    const { io } = fakeIO({
      install: async () => ({ ok: false, detail: "两个源都失败", failureKind: "network" }),
    })
    const result = await runCapabilityOps(io, "p", [
      { kind: "install", step: 1, package: BASE, spec: `${BASE}@1.0.0` },
    ])
    expect(result.ok).toBe(false)
    expect(result.failedAt?.failureKind).toBe("network")
  })

  it("非插件操作导致的失败（如写行自证失败）分类为 null —— 不许硬塞一个类别", async () => {
    const { io } = fakeIO({
      ensureRow: async () => {
        throw new Error("写入后复核未通过：行不在组合树中")
      },
    })
    const result = await runCapabilityOps(io, "p", [
      { kind: "ensureRow", step: 1, package: BASE, rowId: "dsh-dock-base" },
    ])
    expect(result.ok).toBe(false)
    expect(result.failedAt?.failureKind).toBeNull()
  })
})
