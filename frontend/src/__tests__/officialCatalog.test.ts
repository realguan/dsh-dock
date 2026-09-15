// officialCatalog.test.ts —— 官方策展条目编排的纯逻辑测试（2026-09-15，ADR-0020）。
//
// 覆盖的是**风险核心**：顺序即语义、软失败必须当失败、失败可续装。
// 按仓库惯例只测纯逻辑（注入式 IO），不引 DOM 测试栈。

import { describe, expect, it } from "vitest"

import {
  planEntryRun,
  runPlan,
  type CatalogOp,
  type OfficialCatalogIO,
} from "@/lib/officialCatalog"
import type {
  OfficialCatalogRow,
  OfficialCatalogStep,
  PluginOpOutcome,
} from "@/types/ipc"

const ok = (detail = "done"): PluginOpOutcome => ({ ok: true, detail })
const softFail = (detail: string): PluginOpOutcome => ({ ok: false, detail })

function step(over: Partial<OfficialCatalogStep> = {}): OfficialCatalogStep {
  return {
    ordinal: 1,
    package: "@deepseek-ai/pkg",
    spec: "@deepseek-ai/pkg@0.1.6-alpha.1",
    activation: "insert_row",
    rowId: "dsh-dock-pkg",
    installed: false,
    versionNotice: null,
    ...over,
  }
}

function row(over: Partial<OfficialCatalogRow> = {}): OfficialCatalogRow {
  return {
    labelZh: "测试条目",
    family: null,
    requiresNoteZh: null,
    steps: [step()],
    conflictWith: null,
    ...over,
  }
}

/** 记录调用顺序的假 IO——串行性靠它证伪。 */
function fakeIO(
  overrides: Partial<OfficialCatalogIO> = {},
): { io: OfficialCatalogIO; calls: string[] } {
  const calls: string[] = []
  const io: OfficialCatalogIO = {
    install: async (_p, _pkg, spec) => {
      calls.push(`install:${spec}`)
      return ok()
    },
    remove: async (_p, pkg) => {
      calls.push(`remove:${pkg}`)
      return ok()
    },
    writeRow: async (_p, rowId) => {
      calls.push(`writeRow:${rowId}`)
      return true
    },
    ...overrides,
  }
  return { io, calls }
}

describe("planEntryRun", () => {
  it("insert_row 步紧跟一步 writeRow；auto_bundle 步不写行", () => {
    const plan = planEntryRun(
      row({
        steps: [
          step({ ordinal: 1, package: "a", activation: "auto_bundle", rowId: "dsh-dock-a" }),
          step({ ordinal: 2, package: "b", activation: "insert_row", rowId: "dsh-dock-b" }),
        ],
      }),
    )
    expect(plan.ops.map((o) => o.kind)).toEqual(["install", "install", "writeRow"])
    // 关键的"紧跟"：writeRow 必须紧随它那一步 install，不能排到末尾。
    expect(plan.ops[2]).toEqual({ kind: "writeRow", package: "b", rowId: "dsh-dock-b" })
  })

  it("乱序输入按 ordinal 纠正（顺序是语义，不依赖数组下标）", () => {
    const plan = planEntryRun(
      row({
        steps: [
          step({ ordinal: 2, package: "second", rowId: "dsh-dock-second" }),
          step({ ordinal: 1, package: "first", rowId: "dsh-dock-first" }),
        ],
      }),
    )
    const installs = plan.ops.filter((o): o is Extract<CatalogOp, { kind: "install" }> => o.kind === "install")
    expect(installs.map((o) => o.package)).toEqual(["first", "second"])
    expect(installs.map((o) => o.ordinal)).toEqual([1, 2])
  })

  it("同族冲突时先插一步 remove（替换而非叠加，否则第二个 provider 会激活失败）", () => {
    const plan = planEntryRun(
      row({ conflictWith: "@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp" }),
    )
    expect(plan.ops[0]).toEqual({
      kind: "remove",
      package: "@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp",
    })
    expect(plan.replaces).toBe("@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp")
    // 移除必须在安装之前。
    expect(plan.ops.map((o) => o.kind)).toEqual(["remove", "install", "writeRow"])
  })
})

describe("runPlan", () => {
  const plan = planEntryRun(
    row({
      conflictWith: "old-provider",
      steps: [
        step({ ordinal: 1, package: "host-layer", spec: "host-layer@1.0.0", activation: "auto_bundle", rowId: "dsh-dock-host" }),
        step({ ordinal: 2, package: "web-layer", spec: "web-layer@1.0.0", activation: "insert_row", rowId: "dsh-dock-web" }),
      ],
    }),
  )

  it("严格串行且按计划顺序下发（有序组合装反即失败）", async () => {
    const { io, calls } = fakeIO()
    const result = await runPlan(io, "web", plan)
    expect(result.ok).toBe(true)
    expect(result.completedOps).toBe(4)
    expect(calls).toEqual([
      "remove:old-provider",
      "install:host-layer@1.0.0",
      "install:web-layer@1.0.0",
      "writeRow:dsh-dock-web",
    ])
  })

  it("软失败（ok:false）必须当失败——否则会'界面报成功、实际没装上'", async () => {
    const { io, calls } = fakeIO({
      install: async (_p, _pkg, spec) =>
        spec.startsWith("web-layer") ? softFail("pnpm 被审批门拦住") : ok(),
    })
    const result = await runPlan(io, "web", plan)
    expect(result.ok).toBe(false)
    expect(result.failedAt?.error).toContain("pnpm 被审批门拦住")
    // 失败即停：后面的 writeRow 不得执行（否则会写一条指向未安装包的挂载行）。
    expect(calls).not.toContain("writeRow:dsh-dock-web")
    expect(result.completedOps).toBe(2)
    expect(result.totalOps).toBe(4)
  })

  it("抛错同样停在一致态，并给出可续装的进度", async () => {
    const { io } = fakeIO({
      writeRow: async () => {
        throw new Error("写入后复核未通过：未生效")
      },
    })
    const result = await runPlan(io, "web", plan)
    expect(result.ok).toBe(false)
    expect(result.failedAt?.error).toContain("未生效")
    expect(result.completedOps).toBe(3)
    // 续装锚点：从 completedOps 切片即可接着下发剩余操作。
    expect(plan.ops.slice(result.completedOps)).toEqual([
      { kind: "writeRow", package: "web-layer", rowId: "dsh-dock-web" },
    ])
  })

  it("进度回调按序报出每一步（UI 据此渲染 正在安装/正在写配置）", async () => {
    const { io } = fakeIO()
    const seen: string[] = []
    await runPlan(io, "web", plan, (p) => seen.push(`${p.index + 1}/${p.total}:${p.op.kind}`))
    expect(seen).toEqual(["1/4:remove", "2/4:install", "3/4:install", "4/4:writeRow"])
  })

  // 2026-09-15（R2）：安装步必须把**包名**一并交给 IO，而不是只给 spec。
  // 生产绑定拿它当队列项的展示名（`QueuePanel` 渲染 `item.pkg`）；只用 spec 的话
  // 面板会显示 "host-layer@1.0.0" 这类来源串而非用户可以认的包名。
  it("install 步把包名单独交给 IO（队列面板的展示名）", async () => {
    const seen: Array<[string, string]> = []
    const { io } = fakeIO({
      install: async (_p, pkg, spec) => {
        seen.push([pkg, spec])
        return ok()
      },
    })
    await runPlan(io, "web", plan)
    expect(seen).toEqual([
      ["host-layer", "host-layer@1.0.0"],
      ["web-layer", "web-layer@1.0.0"],
    ])
  })
})
