// pluginCatalog.test.ts —— 「插件列表」三类合一的纯逻辑单测（2026-09-20，ADR-0028 第二批；
// 同日第二次修订：判据从「在 bundles 里 = 内置」修正为 dsh 源码的 removable 规则；
// 同日第三次修订：dsh 安装自带的能力（OPTIONAL_BUNDLES）不归 dock 策展——面板不展示、
// 列表不挂「实验性」标，启用后按内置层呈现，见 dockCuratedCaps）。
//
// 钉住的都是真实事实，不是实现细节：
//   · 三类判据的优先级（实验性 > 内置 > 第三方）；
//   · 安装自带的能力被 dockCuratedCaps 过滤：其层按内置展示，不挂实验性标；
//   · **按包名折叠**：层栈 + 依赖 + 栈内重复 → 一个包一行（dsh 的 listBundles 同款
//     去重；reconcile 明确保留重复，壳侧不折叠就会一行变 N 行）；
//   · 「内置」= dsh 安装提供（bundle 条目 ∧ 无依赖条目），**不是**"在层栈里"——
//     用户经 npm 装进 profile 的层（memory-plugin 这类）可更新可卸载；
//   · 层序 = 去重后栈序（重复条目不各占一个层号）；
//   · 反查能力必须覆盖**全部变体**的步骤（自建档 = 基座本身，只查 primary 会漏标）。
// 纯逻辑测试：fixture 内联，不引 RTL/jsdom（AGENTS §5 / §4.3 末段）。
import { describe, expect, it } from "vitest"

import {
  capabilityOfPackage,
  dockCuratedCaps,
  matchesKindFilter,
  matchesSearch,
  mergePluginRows,
} from "@/lib/pluginCatalog"
import type { Capability, PluginEntry, PluginRowState } from "@/types/ipc"

/** dependency 条目（用户经 npm 装进 profile；版本/简介来自 node_modules 实读）。 */
function dep(name: string, version = "1.0.0", description: string | null = null): PluginEntry {
  return { name, kind: "dependency", installed_version: version, description }
}

/** bundle 条目（层栈成员；版本锚在安装目录，Rust 不实读 → 恒 null）。 */
function bundle(name: string): PluginEntry {
  return { name, kind: "bundle", installed_version: null, description: null }
}

const row = (pkg: string, shellDisabled = false): PluginRowState => ({
  id: `dsh-dock-${pkg}`,
  pkg_name: pkg,
  shell_disabled: shellDisabled,
  patch_entries: 1,
  contributed_ids: [],
})

/** 一项两变体的能力（镜像 Agent Teams 的形状：自建档 = 基座，Web 档 = 基座 + Web 层）。
 *  `shipped` = dsh 安装是否自带（OPTIONAL_BUNDLES 那一类的判据）。 */
function capabilityFixture(shipped = false): Capability {
  return {
    id: "agent-team",
    label: "多智能体协同",
    summary: "s",
    unlocks: "u",
    state: "off",
    activeVariant: null,
    shippedByDsh: shipped,
    legacyCopy: false,
    variants: [
      {
        id: "self",
        label: "self",
        note: "",
        prerequisites: [],
        state: "off",
        subsumedBy: null,
        toggleOffSupported: false,
        displaced: [],
        prerequisiteMissing: null,
        steps: [
          {
            ordinal: 1,
            package: "@deepseek-ai/dsh-agent-team-profile",
            spec: "@deepseek-ai/dsh-agent-team-profile@1",
            activation: "auto_bundle",
            rowId: "dsh-dock-x",
            installed: false,
            rowPresent: false,
            disabled: false,
            toggleTargets: [],
            versionNotice: null,
            description: null,
          },
        ],
      },
      {
        id: "web",
        label: "web",
        note: "",
        prerequisites: [],
        state: "off",
        subsumedBy: null,
        toggleOffSupported: false,
        displaced: [],
        prerequisiteMissing: null,
        steps: [
          {
            ordinal: 1,
            package: "@deepseek-ai/dsh-agent-team-profile",
            spec: "@deepseek-ai/dsh-agent-team-profile@1",
            activation: "auto_bundle",
            rowId: "dsh-dock-x",
            installed: false,
            rowPresent: false,
            disabled: false,
            toggleTargets: [],
            versionNotice: null,
            description: null,
          },
          {
            ordinal: 2,
            package: "@deepseek-ai/dsh-agent-team-web-profile",
            spec: "@deepseek-ai/dsh-agent-team-web-profile@1",
            activation: "auto_bundle",
            rowId: "dsh-dock-y",
            installed: false,
            rowPresent: false,
            disabled: false,
            toggleTargets: [],
            versionNotice: null,
            description: null,
          },
        ],
      },
    ],
  }
}

/** 单项能力（镜像 auto-review 的形状：发布在 npm 上的实验包，用户经面板装进 profile）。 */
function autoReviewCapabilityFixture(): Capability {
  return {
    id: "auto-review",
    label: "自动安全审查",
    summary: "s",
    unlocks: "u",
    state: "off",
    activeVariant: null,
    shippedByDsh: false,
    legacyCopy: false,
    variants: [
      {
        id: "standard",
        label: "standard",
        note: "",
        prerequisites: [],
        state: "off",
        subsumedBy: null,
        toggleOffSupported: false,
        displaced: [],
        prerequisiteMissing: null,
        steps: [
          {
            ordinal: 1,
            package: "@deepseek-ai/dsh-experimental-auto-review",
            spec: "@deepseek-ai/dsh-experimental-auto-review@0.1.6-alpha.1",
            activation: "insert_row",
            rowId: "auto-review",
            installed: false,
            rowPresent: false,
            disabled: false,
            toggleTargets: [],
            versionNotice: null,
            description: null,
          },
        ],
      },
    ],
  }
}

/** 三项变体的能力（镜像 browser-use 的真实形状：基座在每个变体里，provider 各自独有）。
 *  源码事实：`@deepseek-ai/dsh-browser-use` 的官方描述是 "Exclusive named browser-use
 *  provider registration"、README 原话 "adds no model-visible tools"——它只登记插槽；
 *  provider（playwright / chrome-devtools / stagehand）互斥，同一时刻只生效一个。 */
function browserUseCapabilityFixture(): Capability {
  const step = (pkg: string, ordinal: number) => ({
    ordinal,
    package: pkg,
    spec: pkg + "@0.1.6-alpha.1",
    activation: "insert_row" as const,
    rowId: "dsh-dock-" + pkg.replace(/[^A-Za-z0-9_-]/g, "-"),
    installed: false,
    rowPresent: false,
    disabled: false,
    toggleTargets: [] as string[],
    versionNotice: null,
    description: null,
  })
  const variant = (id: string, provider: string) => ({
    id,
    label: id,
    note: "",
    prerequisites: [] as string[],
    state: "off" as const,
    subsumedBy: null,
    toggleOffSupported: false,
    displaced: [] as string[],
    prerequisiteMissing: null,
    steps: [step("@deepseek-ai/dsh-browser-use", 1), step(provider, 2)],
  })
  return {
    id: "browser-use",
    label: "浏览器操作",
    summary: "s",
    unlocks: "u",
    state: "off",
    activeVariant: null,
    shippedByDsh: false,
    legacyCopy: false,
    variants: [
      variant("playwright", "@deepseek-ai/dsh-experimental-browser-use-playwright-mcp"),
      variant("chrome-devtools", "@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp"),
      variant("stagehand", "@deepseek-ai/dsh-experimental-browser-use-stagehand-native"),
    ],
  }
}

describe("capabilityRole：一个能力 = 基座 + 一个生效后端，不是两个并列插件（2026-09-20 真机）", () => {
  const caps = [browserUseCapabilityFixture()]
  // 真机形状：基座 + playwright provider 都在 dependencies 里，能力 = playwright 档生效
  const list = mergePluginRows({
    plugins: [
      dep("@deepseek-ai/dsh-browser-use", "0.1.6-alpha.1"),
      dep("@deepseek-ai/dsh-experimental-browser-use-playwright-mcp", "0.1.6-alpha.1"),
    ],
    bundles: [],
    rows: [],
    caps,
  })
  const rowOf = (name: string) => list.rows.find((r) => r.entry.name === name)!

  it("多变体共有的包 = shared（基座，自身不提供工具）", () => {
    const base = rowOf("@deepseek-ai/dsh-browser-use")
    expect(base.kindTag).toBe("experimental")
    expect(base.capabilityRole).toBe("shared")
    expect(base.capability?.label).toBe("浏览器操作")
  })

  it("变体独有的包 = primary（标识包，这一档真正干活的那个）", () => {
    const provider = rowOf("@deepseek-ai/dsh-experimental-browser-use-playwright-mcp")
    expect(provider.capabilityRole).toBe("primary")
    expect(provider.capabilityAnchor).toBe(false)
    expect(provider.capabilityChild).toBe(true)
  })

  it("从属关系：基座是 anchor（能力本体），provider 缩进其下", () => {
    // 组件据此：anchor 挂完整「实验性 · <能力>」+ 去开关；child 挂「后端」+ 缩进
    const base = rowOf("@deepseek-ai/dsh-browser-use")
    expect(base.capabilityAnchor).toBe(true)
    expect(base.capabilityChild).toBe(false)
  })

  it("同一能力在列表里相邻成组（基座在上）", () => {
    const names = list.rows.map((r) => r.entry.name)
    expect(names.indexOf("@deepseek-ai/dsh-browser-use")).toBeGreaterThan(-1)
    // 基座与它的 provider 相邻，且基座在前
    const baseAt = names.indexOf("@deepseek-ai/dsh-browser-use")
    const providerAt = names.indexOf("@deepseek-ai/dsh-experimental-browser-use-playwright-mcp")
    expect(providerAt).toBe(baseAt + 1)
  })

  it("输入里 provider 在基座之前时，组内仍基座在上（从属关系不随输入序翻转）", () => {
    const reordered = mergePluginRows({
      // 依赖字典序下 playwright-mcp 可能排在 dsh-browser-use 之前
      plugins: [
        dep("@deepseek-ai/dsh-experimental-browser-use-playwright-mcp", "0.1.6-alpha.1"),
        dep("@deepseek-ai/dsh-browser-use", "0.1.6-alpha.1"),
      ],
      bundles: [],
      rows: [],
      caps,
    })
    expect(reordered.rows.map((r) => r.entry.name)).toEqual([
      "@deepseek-ai/dsh-browser-use",
      "@deepseek-ai/dsh-experimental-browser-use-playwright-mcp",
    ])
    expect(reordered.rows[0].capabilityAnchor).toBe(true)
    expect(reordered.rows[1].capabilityChild).toBe(true)
  })

  it("只装了基座（装一半）时基座就是 anchor", () => {
    const half = mergePluginRows({
      plugins: [dep("@deepseek-ai/dsh-browser-use", "0.1.6-alpha.1")],
      bundles: [],
      rows: [],
      caps,
    }).rows[0]
    expect(half.capabilityRole).toBe("shared")
    expect(half.capabilityAnchor).toBe(true)
    expect(half.capabilityChild).toBe(false)
  })

  it("单变体能力（auto-review）自成一组：anchor = primary", () => {
    const single = mergePluginRows({
      plugins: [dep("@deepseek-ai/dsh-experimental-auto-review", "0.1.6-alpha.1")],
      bundles: [],
      rows: [],
      caps: [autoReviewCapabilityFixture()],
    }).rows[0]
    expect(single.capabilityRole).toBe("primary")
    expect(single.capabilityAnchor).toBe(true)
    expect(single.capabilityChild).toBe(false)
  })
})

describe("dockCuratedCaps：安装自带的能力不归 dock 策展（2026-09-20 裁定）", () => {
  it("滤掉 shippedByDsh 的能力，保留其余", () => {
    const curated = dockCuratedCaps([
      capabilityFixture(true),
      autoReviewCapabilityFixture(),
    ])
    expect(curated.map((c) => c.id)).toEqual(["auto-review"])
  })

  it("不过滤未自带的能力（探测不到安装 = 仍由 dock 策展）", () => {
    const curated = dockCuratedCaps([capabilityFixture(false)])
    expect(curated).toHaveLength(1)
    expect(dockCuratedCaps([])).toEqual([])
  })

  it("过滤后其包在列表里按内置层展示，不挂「实验性」标", () => {
    const list = mergePluginRows({
      plugins: [bundle("@deepseek-ai/dsh-agent-team-profile")],
      bundles: ["@deepseek-ai/dsh-agent-team-profile"],
      rows: [],
      caps: dockCuratedCaps([capabilityFixture(true)]),
    })
    const r = list.rows[0]
    expect(r.kindTag).toBe("builtin")
    expect(r.capability).toBeNull()
  })
})

describe("capabilityOfPackage：反查覆盖全部变体的步骤", () => {
  it("自建档独有的基座包也归该能力（只查 primary 会漏标成第三方）", () => {
    const caps = [capabilityFixture(), autoReviewCapabilityFixture()]
    expect(capabilityOfPackage(caps, "@deepseek-ai/dsh-agent-team-profile")?.id).toBe("agent-team")
    expect(capabilityOfPackage(caps, "@deepseek-ai/dsh-agent-team-web-profile")?.id).toBe(
      "agent-team",
    )
    expect(capabilityOfPackage(caps, "some-other-plugin")).toBeNull()
    expect(capabilityOfPackage([], "@deepseek-ai/dsh-agent-team-profile")).toBeNull()
  })
})

describe("mergePluginRows：一个包一行（层栈 + 依赖 + 栈内重复全部折叠）", () => {
  // 真机形状（2026-09-20 dev/web）：memory-plugin 同时在层栈与依赖里，且在栈里重复多次
  // （dsh reconcile 保留重复）；auto-review 同理（实验能力面板经 dsh plugin add 装入）。
  const plugins: PluginEntry[] = [
    bundle("@deepseek-ai/dsh-base"),
    bundle("@deepseek-ai/dsh-web-app"),
    bundle("@deepseek-ai/dsh-experimental-auto-review"),
    bundle("@openviking/dsh-memory-plugin"),
    bundle("@openviking/dsh-memory-plugin"),
    bundle("@openviking/dsh-memory-plugin"),
    dep("@deepseek-ai/dsh-experimental-auto-review", "0.1.6-alpha.1"),
    dep("@deepseek-ai/dsh-browser-use", "0.1.6-alpha.1"),
    dep("@openviking/dsh-memory-plugin", "0.3.2", "OpenViking memory and context bundle"),
  ]
  const bundles = [
    "@deepseek-ai/dsh-base",
    "@deepseek-ai/dsh-web-app",
    "@deepseek-ai/dsh-experimental-auto-review",
    "@openviking/dsh-memory-plugin",
    "@openviking/dsh-memory-plugin",
    "@openviking/dsh-memory-plugin",
  ]
  // 带 auto-review 目录（该块同时验"实验 ∧ 用户装"不带内置标）
  const list = mergePluginRows({
    plugins,
    bundles,
    rows: [],
    caps: [autoReviewCapabilityFixture()],
  })
  const rowOf = (name: string) => list.rows.find((r) => r.entry.name === name)!

  it("每个包只出现一次（栈内重复不放大成多行）", () => {
    expect(list.rows.map((r) => r.entry.name)).toEqual([
      "@deepseek-ai/dsh-base",
      "@deepseek-ai/dsh-web-app",
      "@deepseek-ai/dsh-experimental-auto-review",
      "@openviking/dsh-memory-plugin",
      "@deepseek-ai/dsh-browser-use",
    ])
  })

  it("层栈成员在前且按去重后栈序（「层 N」标已撤，顺序留在排列里）", () => {
    // 2026-09-20 维护者裁定：不展示「层 N」这类号标；组合序仍保留在行的排列次序里。
    // 上一条（每个包只出现一次）已钉住完整次序，这里补两条边界：
    const names = list.rows.map((r) => r.entry.name)
    // 层栈四成员（去重后）占据前四位，次序 = bundles 栈序
    expect(names.slice(0, 4)).toEqual([
      "@deepseek-ai/dsh-base",
      "@deepseek-ai/dsh-web-app",
      "@deepseek-ai/dsh-experimental-auto-review",
      "@openviking/dsh-memory-plugin",
    ])
    // 不在层栈里的依赖排在其后
    expect(names.indexOf("@deepseek-ai/dsh-browser-use")).toBeGreaterThan(3)
  })

  it("展示条目优先 dependency（版本/简介实读），不顶「未安装」", () => {
    const mem = rowOf("@openviking/dsh-memory-plugin")
    expect(mem.entry.kind).toBe("dependency")
    expect(mem.entry.installed_version).toBe("0.3.2")
    expect(mem.entry.description).toBe("OpenViking memory and context bundle")
    const review = rowOf("@deepseek-ai/dsh-experimental-auto-review")
    expect(review.entry.installed_version).toBe("0.1.6-alpha.1")
  })

  it("真机回归（2026-09-20 截图）：层栈 ∧ 依赖同时在场 → 第三方，不是「内置」", () => {
    // 截图病灶：memory-plugin 同时是层栈成员（bundle 条目）和 profile 依赖
    // （dependency 条目），第一版按「在层栈里 = 内置」把它标成内置、还因栈内重复
    // 渲染了 5 行。判据修正后：可更新可卸载 → 第三方。
    const mem = rowOf("@openviking/dsh-memory-plugin")
    expect(mem.installed).toBe(true)
    expect(mem.dshProvided).toBe(false)
    expect(mem.kindTag).toBe("thirdParty")
    expect(mem.alsoBuiltin).toBe(false)
    // auto-review 同理：层栈成员 ∧ 依赖 ∧ 实验能力 → 纯实验性，不带内置
    const review = rowOf("@deepseek-ai/dsh-experimental-auto-review")
    expect(review.kindTag).toBe("experimental")
    expect(review.alsoBuiltin).toBe(false)
  })
})

describe("三类判据（优先级：实验性 > 内置 > 第三方；内置 = dsh 安装提供）", () => {
  // agent-team 置 shipped：真实目录里它就在 OPTIONAL_BUNDLES（安装实测自带）
  const caps = dockCuratedCaps([capabilityFixture(true), autoReviewCapabilityFixture()])
  const plugins: PluginEntry[] = [
    bundle("@deepseek-ai/dsh-base"), // 模板层：安装提供、不在依赖里
    bundle("@deepseek-ai/dsh-agent-team-profile"), // dsh 自带的实验层（激活态）
    bundle("@deepseek-ai/dsh-agent-team-web-profile"),
    dep("@deepseek-ai/dsh-experimental-auto-review", "0.1.6-alpha.1"), // 用户装的实验层
    dep("@openviking/dsh-memory-plugin", "0.3.2", "mem"), // 用户装的第三方层
    dep("community-plugin", "2.0.0", "社区插件"), // 普通第三方
  ]
  const bundles = [
    "@deepseek-ai/dsh-base",
    "@deepseek-ai/dsh-agent-team-profile",
    "@deepseek-ai/dsh-agent-team-web-profile",
    "@deepseek-ai/dsh-experimental-auto-review",
    "@openviking/dsh-memory-plugin",
  ]
  const rows = [row("community-plugin"), row("@openviking/dsh-memory-plugin")]
  const list = mergePluginRows({ plugins, bundles, rows, caps })
  const rowOf = (name: string) => list.rows.find((r) => r.entry.name === name)!

  it("主标签互斥", () => {
    expect(rowOf("community-plugin").kindTag).toBe("thirdParty")
    expect(rowOf("@deepseek-ai/dsh-base").kindTag).toBe("builtin")
    expect(rowOf("@deepseek-ai/dsh-experimental-auto-review").kindTag).toBe("experimental")
  })

  it("用户装进 profile 的层不是「内置」（可更新可卸载——dsh removable 规则）", () => {
    const mem = rowOf("@openviking/dsh-memory-plugin")
    expect(mem.installed).toBe(true)
    expect(mem.dshProvided).toBe(false)
    expect(mem.kindTag).toBe("thirdParty")
    expect(mem.alsoBuiltin).toBe(false)
    // 同理：用户经能力面板装的 auto-review 也不带内置标
    const review = rowOf("@deepseek-ai/dsh-experimental-auto-review")
    expect(review.installed).toBe(true)
    expect(review.alsoBuiltin).toBe(false)
  })

  it("安装自带的实验层不进策展：按内置层展示（2026-09-20 裁定）", () => {
    // dockCuratedCaps 把 shipped 能力滤掉后，Agent Teams 两层与模板层同列：内置 +
    // 层 N + 版本随 dsh，**不挂「实验性」标、也没有「去开关」**（开关在 dsh 插件页）。
    const shipped = rowOf("@deepseek-ai/dsh-agent-team-profile")!
    expect(shipped.kindTag).toBe("builtin")
    expect(shipped.capability).toBeNull()
    expect(shipped.alsoBuiltin).toBe(false)
    expect(shipped.dshProvided).toBe(true)
    // 层序只留在排列里（「层 N」标已撤）：base 之后紧跟它
    const names = list.rows.map((r) => r.entry.name)
    expect(names.indexOf("@deepseek-ai/dsh-agent-team-profile")).toBe(
      names.indexOf("@deepseek-ai/dsh-base") + 1,
    )
    expect(rowOf("@deepseek-ai/dsh-agent-team-web-profile").kindTag).toBe("builtin")
  })

  it("混合态才留双标：非自带的能力 ∧ 安装提供的包 → 实验性 + 内置", () => {
    // 能力未整体自带（另一档缺），但本档的包由安装提供且已进层栈 ⇒ 原裁定
    // （Agent Teams「已内置的实验功能」）的判据仍然成立，只是入口换成了过滤后的目录。
    const mixed = mergePluginRows({
      plugins: [bundle("@deepseek-ai/dsh-agent-team-profile")],
      bundles: ["@deepseek-ai/dsh-agent-team-profile"],
      rows: [],
      caps: [capabilityFixture(false)],
    }).rows[0]
    expect(mixed.kindTag).toBe("experimental")
    expect(mixed.alsoBuiltin).toBe(true)
    expect(mixed.dshProvided).toBe(true)
  })

  it("行态按包名挂行；没有行 = null", () => {
    expect(rowOf("community-plugin").row?.id).toBe("dsh-dock-community-plugin")
    expect(rowOf("@openviking/dsh-memory-plugin").row?.pkg_name).toBe(
      "@openviking/dsh-memory-plugin",
    )
    expect(rowOf("@deepseek-ai/dsh-base").row).toBeNull()
  })
})

describe("筛选口径与计数", () => {
  const caps = dockCuratedCaps([capabilityFixture(true), autoReviewCapabilityFixture()])
  const plugins: PluginEntry[] = [
    bundle("@deepseek-ai/dsh-base"),
    bundle("@deepseek-ai/dsh-agent-team-profile"),
    dep("@deepseek-ai/dsh-experimental-auto-review", "0.1.6-alpha.1"),
    dep("community-plugin", "2.0.0"),
  ]
  const bundles = [
    "@deepseek-ai/dsh-base",
    "@deepseek-ai/dsh-agent-team-profile",
    "@deepseek-ai/dsh-experimental-auto-review",
  ]
  const list = mergePluginRows({ plugins, bundles, rows: [], caps })

  it("三类计数可重叠（「内置」含 dsh 自带的层）", () => {
    expect(list.counts.builtin).toBe(2) // base + 自带实验层（滤掉实验性标后按内置计）
    expect(list.counts.thirdParty).toBe(1)
    expect(list.counts.experimental).toBe(1) // 只剩用户装的 auto-review
  })

  it("「内置」筛选含 dsh 自带层；「实验性」只剩 dock 策展的", () => {
    const names = (f: Parameters<typeof matchesKindFilter>[1]) =>
      list.rows.filter((r) => matchesKindFilter(r, f)).map((r) => r.entry.name)
    expect(names("all")).toHaveLength(4)
    expect(names("builtin")).toEqual([
      "@deepseek-ai/dsh-base",
      "@deepseek-ai/dsh-agent-team-profile",
    ])
    expect(names("experimental")).toEqual(["@deepseek-ai/dsh-experimental-auto-review"])
    expect(names("thirdParty")).toEqual(["community-plugin"])
  })
})

describe("matchesSearch", () => {
  const list = mergePluginRows({
    plugins: [dep("community-plugin", "2.0.0", "一个社区插件")],
    bundles: [],
    rows: [],
    caps: [],
  })
  const r = list.rows[0]
  it("命中包名或简介；空查询全放行", () => {
    expect(matchesSearch(r, "")).toBe(true)
    expect(matchesSearch(r, "COMMUNITY")).toBe(true)
    expect(matchesSearch(r, "社区")).toBe(true)
    expect(matchesSearch(r, "不存在")).toBe(false)
  })
})
