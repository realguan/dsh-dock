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

/** 创建结果的前端展示态：ready=完整可用；pending=已创建待装插件；failed=未物化。 */
export type CreateStatus = "ready" | "pending" | "failed"

export function summarizeCreateOutcome(o: {
  materialized: boolean
  installed: boolean
}): CreateStatus {
  if (o.installed && o.materialized) return "ready"
  if (o.materialized) return "pending"
  return "failed"
}
