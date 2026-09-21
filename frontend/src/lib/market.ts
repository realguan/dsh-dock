// lib/market.ts —— 插件市场纯函数工具集 (可单元测试)
import type {
  MarketPlugin,
  MarketPluginDescription,
  MarketRegistry,
  MarketSortOption,
} from "@/types/market"

/**
 * 分类键 → **本地化标签**。
 *
 * 为什么要抽出来（2026-09-21 维护者真机）：同一份 registry 的分类在两个面各写一套，
 * 「插件中心」的市场页渲染的是 `categories[key].zh`（中文标签），而「添加插件」弹窗
 * 的筛选下拉直接渲染**原始键**（`agi` / `ui` / `usage`…）。同一个东西两种写法，
 * 用户在弹窗里看到的是一串英文枚举。
 *
 * 回退链：元数据缺失 → 原键原样（宁可露出原始键，也不瞎翻译）。
 */
export function getMarketCategoryLabel(
  registry: MarketRegistry | null | undefined,
  key: string,
  locale: string,
): string {
  if (!key) return ""
  const obj = registry?.categories?.[key]
  if (!obj) return key
  return locale.startsWith("zh") ? obj.zh || obj.en : obj.en || obj.zh
}

export interface MarketCategoryOption {
  key: string
  label: string
  count: number
}

/**
 * 分类清单（带计数、**按数量降序**）——市场页的分类矩阵与「添加插件」弹窗的筛选下拉
 * 共用同一份（禁双源：两处的集合/标签/顺序必须逐项一致，否则"这个下拉里的分类
 * 跟那边不一样"就是必然）。
 *
 * 集合取**元数据键 ∪ 插件实际用到的键**：元数据可能缺某个键（卡片那时也只能显示原始
 * 键），而插件用到的分类必须可选，否则那个分类下的插件无处可筛。
 * 排序只在数量上定序，同数量的保持既有相对次序（`Array#sort` 稳定）。
 */
export function marketCategoryOptions(
  registry: MarketRegistry | null | undefined,
  locale: string,
): MarketCategoryOption[] {
  if (!registry) return []
  const counts: Record<string, number> = {}
  for (const p of registry.plugins) {
    if (!p.category) continue
    counts[p.category] = (counts[p.category] || 0) + 1
  }
  const keys = new Set<string>(Object.keys(registry.categories ?? {}))
  for (const key of Object.keys(counts)) keys.add(key)
  return Array.from(keys)
    .map((key) => ({
      key,
      label: getMarketCategoryLabel(registry, key, locale),
      count: counts[key] || 0,
    }))
    .sort((a, b) => b.count - a.count)
}

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
