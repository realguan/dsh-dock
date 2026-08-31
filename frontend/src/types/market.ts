// types/market.ts —— 社区插件市场 (dsh-market / awesome-dsh-plugin) 数据模型

export interface MarketCategory {
  en: string
  zh: string
}

export interface MarketPluginDescription {
  en?: string
  zh?: string
}

export interface MarketPlugin {
  name: string
  owner: string
  url: string
  page: string
  category: string
  description: MarketPluginDescription | string
  npm: string | null
  tarball?: string
  stars: number
  downloads: number | null
  install: string
  added: string
}

export interface MarketRegistry {
  name: string
  url: string
  updated: string
  count: number
  categories: Record<string, MarketCategory>
  plugins: MarketPlugin[]
}

export type MarketSortOption = "stars" | "downloads" | "newest" | "name"
