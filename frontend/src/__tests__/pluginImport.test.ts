// 从其他 profile 安装纯逻辑（4.4④ 收口，ADR-0009 第五次修订）：候选过滤
// （排除目标已装 / 自身来源 / 声明未安装；连配置可用性预检）与批量结果汇总
// （失败继续口径）。fixture 内联，不引 DOM。
import { describe, expect, it } from "vitest"
import { groupPickerCandidates, pickerCandidates, summarizeBatch, type BatchItemResult } from "@/lib/profiles"
import type { AggregatePlugin, PluginRowState } from "@/types/ipc"

const agg = (name: string, sources: { profile: string; version: string | null }[]): AggregatePlugin => ({
  name,
  description: `${name} 的描述`,
  sources,
})

const rows = (pkg: string, patchEntries: number): PluginRowState => ({
  id: `${pkg}-row`,
  pkg_name: pkg,
  shell_disabled: false,
  patch_entries: patchEntries,
})

describe("pickerCandidates", () => {
  const aggregate = [
    // 契约：sources 按 profile 字典序（后端 scan_profiles 排序后聚合）
    agg("dsh-shared", [
      { profile: "dev", version: "0.15.3" },
      { profile: "web", version: "0.16.1" },
    ]),
    agg("dsh-ghost", [{ profile: "web", version: null }]),
    agg("dsh-only-dev", [{ profile: "dev", version: "1.2.0" }]),
  ]

  it("rows = plugin x source, excludes target-owned and uninstalled", () => {
    const out = pickerCandidates(aggregate, "prod", [], {})
    // dsh-shared 两个来源两行；dsh-ghost（声明未安装）不进候选；
    // dsh-only-dev 一行；目标 prod 自己不是来源
    expect(out.map((c) => `${c.pkg}@${c.version}:${c.source}`)).toEqual([
      "dsh-shared@0.15.3:dev",
      "dsh-shared@0.16.1:web",
      "dsh-only-dev@1.2.0:dev",
    ])
  })

  it("excludes plugins already installed in target", () => {
    const out = pickerCandidates(aggregate, "prod", ["dsh-shared"], {})
    expect(out.map((c) => c.pkg)).toEqual(["dsh-only-dev"])
  })

  it("hasConfig from source row table (patch_entries > 0); missing rows -> false", () => {
    const out = pickerCandidates(aggregate, "prod", [], {
      web: [rows("dsh-shared", 2)],
      // dev 行表查询失败 → 不在 map 里
    })
    expect(out.find((c) => c.source === "web")?.hasConfig).toBe(true)
    expect(out.find((c) => c.source === "dev")?.hasConfig).toBe(false)
  })

  it("empty aggregate -> no candidates", () => {
    expect(pickerCandidates([], "prod", [], {})).toEqual([])
  })
})

describe("groupPickerCandidates（去重折叠）", () => {
  it("groups flat candidates by pkg and preserves all sources", () => {
    const candidates = [
      {
        pkg: "dsh-better-sidebar",
        source: "web",
        version: "0.17.1",
        description: "sidebar desc",
        hasConfig: true,
      },
      {
        pkg: "dsh-better-sidebar",
        source: "test",
        version: "0.17.1",
        description: "sidebar desc",
        hasConfig: false,
      },
      {
        pkg: "dshmarket",
        source: "web",
        version: "1.38.1",
        description: "market desc",
        hasConfig: false,
      },
    ]

    const grouped = groupPickerCandidates(candidates)
    expect(grouped).toHaveLength(2)
    expect(grouped[0].pkg).toBe("dsh-better-sidebar")
    expect(grouped[0].sources).toHaveLength(2)
    expect(grouped[0].sources[0].profile).toBe("web")
    expect(grouped[0].sources[0].hasConfig).toBe(true)
    expect(grouped[0].sources[1].profile).toBe("test")
    expect(grouped[0].sources[1].hasConfig).toBe(false)

    expect(grouped[1].pkg).toBe("dshmarket")
    expect(grouped[1].sources).toHaveLength(1)
  })
})

describe("summarizeBatch（失败继续口径）", () => {
  it("counts ok vs failed and isolates failure details", () => {
    const results: BatchItemResult[] = [
      { pkg: "a", ok: true, detail: "已安装 a" },
      { pkg: "b", ok: false, detail: "安装失败（退出码 1）" },
      { pkg: "c", ok: true, detail: "已安装 c" },
    ]
    const s = summarizeBatch(results)
    expect(s.okCount).toBe(2)
    expect(s.failCount).toBe(1)
    expect(s.failures.map((f) => f.pkg)).toEqual(["b"])
  })

  it("empty queue -> all zeros", () => {
    expect(summarizeBatch([])).toEqual({ okCount: 0, failCount: 0, failures: [] })
  })
})
