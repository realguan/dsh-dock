// lib/market.ts —— 插件市场纯函数工具集 (可单元测试)
import type { MarketPlugin, MarketPluginDescription, MarketSortOption } from "@/types/market"

/**
 * 市场条目在本机已安装的 profile 列表（ADR-0011）。npm 来源按 npm 包名
 * 命中；git/tarball 来源的市场展示名可能与真实包名不同（如市场名 dsh-pet、
 * 实际 @linxin666/dsh-pet）——此时以安装 spec（install 串尾段）对齐聚合的
 * 依赖声明值。名字命中优先，spec 兜底。
 */
export function installedProfilesFor(
  plugin: Pick<MarketPlugin, "name" | "npm" | "install">,
  installedMap: Map<string, string[]>,
): string[] {
  const byName =
    installedMap.get(plugin.npm || "") ?? installedMap.get(plugin.name) ?? []
  if (byName.length > 0) return byName
  const spec = extractInstallSpec(plugin.install)
  return (spec ? installedMap.get(spec) : undefined) ?? []
}

/**
 * 从完整的 dsh plugin install 命令字符串中提取包名 / 安装 spec
 * 例如: "dsh plugin --profile web add @furongjun1999/dsh-memory" -> "@furongjun1999/dsh-memory"
 * 例如: "dsh plugin --profile web add github:foo/bar" -> "github:foo/bar"
 */
export function extractInstallSpec(installCmd: string): string {
  if (!installCmd) return ""
  const trimmed = installCmd.trim()
  const parts = trimmed.split(/\s+/)
  const last = parts[parts.length - 1] || ""
  // registry 数据形态：tarball 直链常被成对引号包裹（`dsh plugin add "https://…tgz"`）
  // ——argv 单传无 shell，引号会字面进入 spec，提取层剥掉（ADR-0011）
  if (
    last.length >= 2 &&
    ((last.startsWith('"') && last.endsWith('"')) ||
      (last.startsWith("'") && last.endsWith("'")))
  ) {
    return last.slice(1, -1)
  }
  return last
}

/**
 * 构造安装命令（动态绑定指定 Profile 与 Spec，自动清理上游命令中的硬编码 profile）
 * 例如: buildInstallCmd("my-profile", "@deepseek-ai/dsh-pet") -> "dsh plugin --profile my-profile add @deepseek-ai/dsh-pet"
 * 例如: buildInstallCmd("dev", "dsh plugin --profile web add @foo/bar") -> "dsh plugin --profile dev add @foo/bar"
 */
export function buildInstallCmd(profile: string, specOrInstallCmd: string): string {
  const cleanSpec = extractInstallSpec(specOrInstallCmd) || specOrInstallCmd.trim()
  const cleanProf = (profile || "web").trim()
  return `dsh plugin --profile ${cleanProf} add ${cleanSpec}`
}

/**
 * 从可能包含 monorepo 或路径前缀的名称中提取最终展示名
 * 例如: "dsh-web#packages/dsh-task-board" -> "dsh-task-board"
 * 例如: "dsh-web-ui#dsh-task-board" -> "dsh-task-board"
 * 例如: "dsh-trail#bundle" -> "bundle"
 * 例如: "@deepseek-ai/dsh-base" -> "@deepseek-ai/dsh-base"
 */
export function getPluginDisplayName(name: string): string {
  if (!name) return ""
  if (name.includes("#")) {
    const hashParts = name.split("#")
    const sub = hashParts[hashParts.length - 1] || ""
    if (sub.includes("/")) {
      const slashParts = sub.split("/")
      return slashParts[slashParts.length - 1] || sub
    }
    return sub
  }
  return name
}

export type InstallSourceType = "npm" | "github"

export interface InstallSourceInfo {
  type: InstallSourceType
  spec: string
  label: string
}

/**
 * 自动识别插件安装 Spec 与来源类型 (NPM 还是 GitHub)
 */
export function detectInstallSource(plugin: MarketPlugin): InstallSourceInfo {
  const extracted = extractInstallSpec(plugin.install)
  const spec = extracted || plugin.npm || (plugin.url ? `github:${plugin.owner}/${plugin.name}` : plugin.name)

  const isGitHub =
    spec.startsWith("github:") ||
    spec.startsWith("git+") ||
    spec.startsWith("https://github.com") ||
    (!plugin.npm && Boolean(plugin.url))

  return {
    type: isGitHub ? "github" : "npm",
    spec,
    label: isGitHub ? "GitHub 仓库" : "NPM 官方包",
  }
}

/**
 * 提取多语言描述
 */
export function getPluginDescription(
  desc: MarketPluginDescription | string | null | undefined,
  locale: string = "zh-CN",
): string {
  if (!desc) return ""
  if (typeof desc === "string") return desc
  if (locale.startsWith("zh")) {
    return desc.zh || desc.en || ""
  }
  return desc.en || desc.zh || ""
}

/**
 * 客户端搜索与分类过滤
 */
export function filterMarketPlugins({
  plugins,
  query,
  category,
  onlyInstalled,
  installedPluginNames,
}: {
  plugins: MarketPlugin[]
  query: string
  category: string
  onlyInstalled?: boolean
  installedPluginNames?: Set<string>
}): MarketPlugin[] {
  const q = query.toLowerCase().trim()
  const hasCategory = category && category !== "all"

  return plugins.filter((plugin) => {
    // 分类筛选
    if (hasCategory && plugin.category !== category) {
      return false
    }

    // 仅已装筛选
    if (onlyInstalled && installedPluginNames) {
      const isInstalled =
        (plugin.npm && installedPluginNames.has(plugin.npm)) ||
        installedPluginNames.has(plugin.name)
      if (!isInstalled) return false
    }

    // 关键词搜索 (匹配 名称 / npm / owner / 描述 / 分类)
    if (q) {
      const nameMatch = plugin.name.toLowerCase().includes(q)
      const npmMatch = plugin.npm ? plugin.npm.toLowerCase().includes(q) : false
      const ownerMatch = plugin.owner.toLowerCase().includes(q)
      const descZh = typeof plugin.description === "object" ? plugin.description?.zh ?? "" : plugin.description
      const descEn = typeof plugin.description === "object" ? plugin.description?.en ?? "" : ""
      const descMatch =
        descZh.toLowerCase().includes(q) || descEn.toLowerCase().includes(q)
      const catMatch = plugin.category.toLowerCase().includes(q)

      if (!nameMatch && !npmMatch && !ownerMatch && !descMatch && !catMatch) {
        return false
      }
    }

    return true
  })
}

/**
 * 排序插件列表
 */
export function sortMarketPlugins(
  plugins: MarketPlugin[],
  sortOption: MarketSortOption,
): MarketPlugin[] {
  const list = [...plugins]
  switch (sortOption) {
    case "stars":
      return list.sort((a, b) => (b.stars || 0) - (a.stars || 0))
    case "downloads":
      return list.sort((a, b) => (b.downloads || 0) - (a.downloads || 0))
    case "newest":
      return list.sort((a, b) => (b.added || "").localeCompare(a.added || ""))
    case "name":
      return list.sort((a, b) => a.name.localeCompare(b.name))
    default:
      return list
  }
}
