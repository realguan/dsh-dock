import type { RuntimeEntry } from "@/types/ipc"
// Profile 管理器纯逻辑（4.3 前端刀）。校验规则逐字镜像后端
// profiles::validate_profile_name（dsh resolveProfileDir @ 318）——前端只做
// 输入预检提效，后端校验仍是权威（不可信边界在 IPC 之外）。
// 模板 bundle 列表镜像后端 PROFILE_TEMPLATES（dsh-app-boot @ 323）。

/** 内置模板名 → 初始化 bundle 列表（未物化时的「首启将得到」预览）。 */
export const TEMPLATE_BUNDLES: Record<string, readonly string[]> = {
  web: ["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-web-app"],
  headless: ["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-headless"],
}

/** 名字非法时返回可行动错误文案；合法返回 null（与 dsh 拒绝集逐字一致）。 */
export function validateProfileName(name: string): string | null {
  if (name === "") return "名字不能为空"
  if (name.includes("/")) return "名字不能包含 /"
  if (name.includes("\\")) return "名字不能包含 \\"
  if (name === ".") return "名字不能是 ."
  if (name === "..") return "名字不能是 .."
  if (name === "node_modules") return "node_modules 是保留名（dsh 内部使用）"
  return null
}

/** 创建结果的前端展示态：ready=基础 + Web 工作台就绪；pending=已创建待装插件；failed=未物化。 */
export type CreateStatus = "ready" | "pending" | "failed"

export function summarizeCreateOutcome(o: {
  materialized: boolean
  installed: boolean
}): CreateStatus {
  if (o.installed && o.materialized) return "ready"
  if (o.materialized) return "pending"
  return "failed"
}

// ---------- 插件运行态合并（4.4①，Spike B / 复现点 11） ----------

/** fiber phase 的中文标签；null = 已停用（disposed）。 */
export function phaseLabel(phase: string | null): string {
  if (phase === null) return "已停用"
  const map: Record<string, string> = {
    active: "运行中",
    loading: "加载中",
    pending: "等待中",
    failed: "失败",
    unloading: "卸载中",
  }
  return map[phase] ?? phase
}

/**
 * 插件名/规格输入预检（4.4②）——逐字镜像后端 plugins::validate_plugin_spec
 * （前端只做提效预检，后端校验仍是权威）。防两类滥用：pnpm 旗标注入（前导
 * `-`）与控制字符/空白；scope 包名与版本段（tag/精确/^~ 区间）放行，`><`
 * 语义区间 v1 不开（走终端）。
 */
export function validatePluginSpec(spec: string): string | null {
  if (spec === "") return "包名不能为空"
  if (spec.length > 214) return "包名过长（npm 上限 214 字符）"
  if (spec.startsWith("-")) return "包名不能以 - 开头（会被当作命令参数）"
  if (!/^[a-zA-Z0-9@/._^~*-]+$/.test(spec))
    return "包名只允许字母数字与 @/._^~*-（版本段支持 tag、精确版本、^~ 区间）"
  return null
}

export interface RuntimeChip {
  label: string
  failed: boolean
}

/**
 * 某插件的运行态徽标：按 moduleName 匹配条目后取「最坏相位」——
 * failed 优先（警示色），其次 loading/pending/unloading，再 active（enabled=false
 * 的行不算 active——禁用行报「运行中」是误导），全停用兜底。
 * 无匹配条目返回 null（内置 bundle 多不以此名出现，无徽标即无运行态可显）。
 */
export function runtimeChipFor(
  moduleName: string,
  entries: RuntimeEntry[],
): RuntimeChip | null {
  const mine = entries.filter((e) => e.module_name === moduleName)
  if (mine.length === 0) return null
  const failed = mine.filter((e) => e.fiber_phase === "failed").length
  if (failed > 0) return { label: `${phaseLabel("failed")}×${failed}`, failed: true }
  const loading = mine.filter(
    (e) =>
      e.fiber_phase === "loading" ||
      e.fiber_phase === "pending" ||
      e.fiber_phase === "unloading",
  ).length
  if (loading > 0) return { label: `${phaseLabel("loading")}×${loading}`, failed: false }
  const active = mine.filter((e) => e.fiber_phase === "active" && e.enabled).length
  if (active > 0) return { label: `${phaseLabel("active")}×${active}`, failed: false }
  return { label: phaseLabel(null), failed: false }
}

export interface RuntimeSummary {
  active: number
  failed: number
  loading: number
  disabled: number
}

/** 快照全量汇总：徽标行上方的会话级一句话。禁用行（disposed 或 enabled=false）先归停用。 */
export function runtimeSummary(entries: RuntimeEntry[]): RuntimeSummary {
  const s: RuntimeSummary = { active: 0, failed: 0, loading: 0, disabled: 0 }
  for (const e of entries) {
    if (e.fiber_phase === null || !e.enabled) s.disabled += 1
    else if (e.fiber_phase === "active") s.active += 1
    else if (e.fiber_phase === "failed") s.failed += 1
    else s.loading += 1
  }
  return s
}
