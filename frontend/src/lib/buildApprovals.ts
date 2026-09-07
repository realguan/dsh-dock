// lib/buildApprovals.ts —— pnpm 12 构建审批门的纯逻辑（2026-09-07）。
// 被点名包名 → 裁决行（默认跳过 = false：安装必成，个别可选功能可能缺
// 二进制；允许 = true 由用户逐包显式开启）。

export interface BuildApprovalChoice {
  name: string
  allowed: boolean
}

/** 去重保序生成默认裁决行（全部跳过）。 */
export function defaultApprovals(pkgs: string[]): BuildApprovalChoice[] {
  const seen = new Set<string>()
  const out: BuildApprovalChoice[] = []
  for (const raw of pkgs) {
    const name = raw.trim()
    if (!name || seen.has(name)) continue
    seen.add(name)
    out.push({ name, allowed: false })
  }
  return out
}

/** 合并既有裁决（重试再撞门时保留用户上次的选择）。 */
export function mergeApprovals(
  base: string[],
  prior: BuildApprovalChoice[],
): BuildApprovalChoice[] {
  const priorMap = new Map(prior.map((p) => [p.name, p.allowed]))
  return defaultApprovals(base).map((row) => ({
    ...row,
    allowed: priorMap.get(row.name) ?? row.allowed,
  }))
}
