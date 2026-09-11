import {
  Download,
  Package,
  Plus,
  Send,
  Sparkles,
  Star,
} from "lucide-react"
import { useI18n } from "@/stores/i18nStore"
import type { MarketPlugin } from "@/types/market"
import { PROFILE_CHIP_CLASS } from "@/lib/format"
import { getPluginDescription, getPluginDisplayName } from "@/lib/market"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"

function GithubIcon({ className = "size-3.5" }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="currentColor" className={className} aria-hidden="true">
      <path fillRule="evenodd" clipRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.53 1.032 1.53 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" />
    </svg>
  )
}

function NpmIcon({ className = "size-3.5" }: { className?: string }) {
  return (
    <svg viewBox="0 0 780 250" fill="currentColor" className={className} aria-hidden="true">
      <path d="M240,250h100v-50h100V0H240V250z M340,50h50v100h-50V50z M480,0v200h100V50h50v150h50V50h50v150h50V0H480z M0,200h100V50h50v150h50V0H0V200z" />
    </svg>
  )
}

interface MarketPluginCardProps {
  plugin: MarketPlugin
  categoryLabel?: string
  installedProfiles: string[]
  onInstall: (plugin: MarketPlugin) => void
  onOpenExternal: (url: string) => void
}

export function MarketPluginCard({
  plugin,
  categoryLabel,
  installedProfiles,
  onInstall,
  onOpenExternal,
}: MarketPluginCardProps) {
  const { t, activeLocale } = useI18n()

  const isInstalled = installedProfiles.length > 0
  const isOfficial = plugin.owner.toLowerCase().includes("deepseek") || plugin.name.startsWith("@deepseek-ai/")
  const displayName = getPluginDisplayName(plugin.name)
  const desc = getPluginDescription(plugin.description, activeLocale)

  return (
    <article className="group relative flex flex-col justify-between rounded-xl border border-line bg-panel p-4 shadow-2xs transition-all duration-200 hover:border-brand/40 hover:shadow-xs hover:-translate-y-0.5">
      {/* 卡片头部 */}
      <div>
        <div className="flex items-start justify-between gap-2.5">
          <div className="flex items-center gap-2.5 min-w-0">
            {/* 左上角品牌/来源定制主图标 */}
            {isOfficial ? (
              <div className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-brand/40 bg-brand/10 text-brand-deep shadow-2xs group-hover:border-brand transition-colors" title={t.market.officialCoreTitle}>
                <Sparkles className="size-4.5 text-brand-deep" />
              </div>
            ) : plugin.npm ? (
              <div className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-line bg-wash text-ink shadow-2xs group-hover:border-brand/40 transition-colors" title={t.market.sourceNpm}>
                <NpmIcon className="h-3 w-5 text-ink/80" />
              </div>
            ) : plugin.url?.includes("github.com") ? (
              <div className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-line bg-wash text-ink shadow-2xs group-hover:border-brand/40 transition-colors" title={t.market.sourceGithub}>
                <GithubIcon className="size-4.5 text-ink/80" />
              </div>
            ) : (
              <div className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-line bg-wash text-brand-deep shadow-2xs group-hover:border-brand/30 group-hover:bg-brand/5 transition-colors">
                <Package className="size-4.5" />
              </div>
            )}
            <div className="min-w-0">
              <div className="flex items-center gap-1.5 flex-wrap">
                <h3
                  className="truncate font-mono text-xs font-bold text-ink tracking-tight hover:text-brand-deep cursor-pointer transition-colors"
                  title={displayName}
                  onClick={() => onOpenExternal(plugin.url || plugin.page)}
                >
                  {displayName}
                </h3>
                {isOfficial && (
                  <Badge variant="outline" className="h-4 px-1 text-micro bg-brand/10 text-brand-deep border-brand/30 font-mono">
                    OFFICIAL
                  </Badge>
                )}
              </div>
              <p className="truncate text-label text-dim font-mono mt-0.5" title={plugin.owner}>
                by <span className="text-ink/80">{plugin.owner}</span>
              </p>
            </div>
          </div>

          {/* 分类徽标 */}
          {categoryLabel && (
            <Badge
              variant="secondary"
              className="shrink-0 text-meta font-medium border border-line/60 bg-wash text-dim"
            >
              {categoryLabel}
            </Badge>
          )}
        </div>

        {/* 描述文本 */}
        <p
          className="mt-3 line-clamp-2 text-xs text-dim leading-relaxed min-h-[32px]"
          title={desc || undefined}
        >
          {desc || t.market.noDescription}
        </p>

        {/* 指标栏 (Stars, Downloads, Added) */}
        <div className="mt-3 flex items-center gap-3 text-label text-faint font-mono">
          <div className="flex items-center gap-1 text-ink/70" title="GitHub Stars">
            <Star className="size-3 fill-dim/30" />
            <span>{plugin.stars?.toLocaleString() ?? 0}</span>
          </div>

          {plugin.downloads !== null && plugin.downloads !== undefined && (
            <div className="flex items-center gap-1 text-ink/70" title="NPM Downloads">
              <Download className="size-3" />
              <span>{plugin.downloads >= 1000 ? `${(plugin.downloads / 1000).toFixed(1)}k` : plugin.downloads}</span>
            </div>
          )}

          {plugin.added && (
            <div className="truncate ml-auto text-meta text-faint" title={plugin.added}>
              {plugin.added}
            </div>
          )}
        </div>
      </div>

      {/* 卡片底部：分布 Profile 芯片 + 操作按钮 */}
      <div className="mt-3.5 pt-3 border-t border-line/70 flex flex-col gap-2.5">
        {/* 本地安装状态展示与官方外链 */}
        <div className="flex items-center justify-between gap-2 min-h-[22px]">
          <div className="flex items-center gap-1 overflow-x-auto no-scrollbar py-0.5 max-w-[200px]">
            {isInstalled ? (
              installedProfiles.map((prof) => {
                return (
                  <span
                    key={prof}
                    className={`inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-meta font-mono font-medium shrink-0 shadow-2xs ${PROFILE_CHIP_CLASS}`}
                    title={t.market.installedInProfile(prof)}
                  >
                    <span className="size-1 rounded-full bg-current opacity-80" />
                    {prof}
                  </span>
                )
              })
            ) : (
              <span className="text-meta text-faint font-mono">
                {t.market.notInstalled}
              </span>
            )}
          </div>

          {/* 外链快速官方图标 */}
          <div className="flex items-center gap-1 shrink-0">
            {plugin.url && (
              <button
                type="button"
                onClick={() => onOpenExternal(plugin.url)}
                className="rounded-lg p-1.5 text-faint hover:text-ink hover:bg-wash transition-colors flex items-center gap-1 cursor-pointer"
                title={t.market.viewReadme}
              >
                <GithubIcon className="size-3.5" />
              </button>
            )}
            {plugin.npm && (
              <button
                type="button"
                onClick={() => onOpenExternal(`https://www.npmjs.com/package/${plugin.npm}`)}
                className="rounded-lg p-1.5 text-faint hover:text-danger hover:bg-danger-soft transition-colors flex items-center gap-1 cursor-pointer"
                title={t.market.viewNpm}
              >
                <NpmIcon className="h-3 w-4.5 text-faint hover:text-danger" />
              </button>
            )}
          </div>
        </div>

        {/* 主动作按钮 */}
        <Button
          size="sm"
          variant={isInstalled ? "outline" : "default"}
          onClick={() => onInstall(plugin)}
          className={`w-full h-8 text-xs font-medium gap-1.5 rounded-lg transition-all ${
            isInstalled
              ? "border-line text-ink hover:bg-wash hover:border-brand/40"
              : "bg-brand text-white hover:bg-brand/90 shadow-2xs"
          }`}
        >
          {isInstalled ? (
            <>
              <Send className="size-3" />
              <span>{t.market.distributeBtn}</span>
            </>
          ) : (
            <>
              <Plus className="size-3.5" />
              <span>{t.market.installBtn}</span>
            </>
          )}
        </Button>
      </div>
    </article>
  )
}
