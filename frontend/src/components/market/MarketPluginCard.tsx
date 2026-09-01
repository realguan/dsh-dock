// components/market/MarketPluginCard.tsx —— 插件市场单个插件卡片 (高质感工程控制台美学)
import { useState } from "react"
import {
  Check,
  Code2,
  Copy,
  Download,
  ExternalLink,
  Package,
  Plus,
  Send,
  Star,
} from "lucide-react"
import { useI18n } from "@/stores/i18nStore"
import type { MarketPlugin } from "@/types/market"
import { getProfileColorClass } from "@/lib/format"
import { getPluginDescription, getPluginDisplayName } from "@/lib/market"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"

interface MarketPluginCardProps {
  plugin: MarketPlugin
  categoryLabel?: string
  installedProfiles: string[]
  onInstall: (plugin: MarketPlugin) => void
  onOpenExternal: (url: string) => void
  onCopyNotice?: (text: string) => void
}

export function MarketPluginCard({
  plugin,
  categoryLabel,
  installedProfiles,
  onInstall,
  onOpenExternal,
  onCopyNotice,
}: MarketPluginCardProps) {
  const { t, activeLocale } = useI18n()
  const [copied, setCopied] = useState(false)

  const isInstalled = installedProfiles.length > 0
  const isOfficial = plugin.owner.toLowerCase().includes("deepseek") || plugin.name.startsWith("@deepseek-ai/")
  const displayName = getPluginDisplayName(plugin.name)
  const desc = getPluginDescription(plugin.description, activeLocale)

  const handleCopyCmd = (e: React.MouseEvent) => {
    e.stopPropagation()
    const cmd = plugin.install || `dsh plugin --profile web add ${plugin.npm || plugin.name}`
    void navigator.clipboard.writeText(cmd)
    setCopied(true)
    onCopyNotice?.(t.market.copied)
    setTimeout(() => setCopied(false), 2000)
  }

  return (
    <article className="group relative flex flex-col justify-between rounded-xl border border-line bg-panel p-4 shadow-2xs transition-all duration-200 hover:border-brand/40 hover:shadow-xs hover:-translate-y-0.5">
      {/* 卡片头部 */}
      <div>
        <div className="flex items-start justify-between gap-2.5">
          <div className="flex items-center gap-2.5 min-w-0">
            <div className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-line bg-wash text-brand shadow-2xs group-hover:border-brand/30 group-hover:bg-brand/5 transition-colors">
              <Package className="size-4.5" />
            </div>
            <div className="min-w-0">
              <div className="flex items-center gap-1.5 flex-wrap">
                <h3
                  className="truncate font-mono text-xs font-bold text-ink tracking-tight hover:text-brand cursor-pointer transition-colors"
                  title={plugin.name}
                  onClick={() => onOpenExternal(plugin.url || plugin.page)}
                >
                  {displayName}
                </h3>
                {isOfficial && (
                  <Badge variant="outline" className="h-4 px-1 text-[9px] bg-brand/10 text-brand border-brand/30 font-mono">
                    OFFICIAL
                  </Badge>
                )}
              </div>
              <p className="truncate text-[11px] text-dim font-mono mt-0.5">
                by <span className="text-ink/80">{plugin.owner}</span>
              </p>
            </div>
          </div>

          {/* 分类徽标 */}
          {categoryLabel && (
            <Badge
              variant="secondary"
              className="shrink-0 text-[10px] font-medium border border-line/60 bg-wash text-dim"
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
          {desc || "暂无描述"}
        </p>

        {/* 指标栏 (Stars, Downloads, Added) */}
        <div className="mt-3 flex items-center gap-3 text-[11px] text-faint font-mono">
          <div className="flex items-center gap-1 text-ink/70" title="GitHub Stars">
            <Star className="size-3 text-amber-500 fill-amber-500/20" />
            <span>{plugin.stars?.toLocaleString() ?? 0}</span>
          </div>

          {plugin.downloads !== null && plugin.downloads !== undefined && (
            <div className="flex items-center gap-1 text-ink/70" title="NPM Downloads">
              <Download className="size-3 text-brand" />
              <span>{plugin.downloads >= 1000 ? `${(plugin.downloads / 1000).toFixed(1)}k` : plugin.downloads}</span>
            </div>
          )}

          {plugin.added && (
            <div className="truncate ml-auto text-[10px] text-faint">
              {plugin.added}
            </div>
          )}
        </div>
      </div>

      {/* 卡片底部：分布 Profile 芯片 + 操作按钮 */}
      <div className="mt-4 pt-3 border-t border-line/70 flex flex-col gap-2.5">
        {/* 本地安装状态展示 */}
        <div className="flex items-center justify-between gap-2 min-h-[22px]">
          <div className="flex items-center gap-1 overflow-x-auto no-scrollbar py-0.5 max-w-[200px]">
            {isInstalled ? (
              installedProfiles.map((prof) => {
                const colorClass = getProfileColorClass(prof)
                return (
                  <span
                    key={prof}
                    className={`inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-[10px] font-mono font-medium shrink-0 shadow-2xs ${colorClass}`}
                    title={`已安装在 ${prof}`}
                  >
                    <span className="size-1 rounded-full bg-current opacity-80" />
                    {prof}
                  </span>
                )
              })
            ) : (
              <span className="text-[10px] text-faint font-mono">
                {t.market.notInstalled}
              </span>
            )}
          </div>

          {/* 外链快速图标 */}
          <div className="flex items-center gap-1 shrink-0">
            {plugin.url && (
              <button
                type="button"
                onClick={() => onOpenExternal(plugin.url)}
                className="rounded p-1 text-faint hover:text-ink hover:bg-wash transition-colors"
                title={t.market.viewReadme}
              >
                <Code2 className="size-3.5" />
              </button>
            )}
            {plugin.npm && (
              <button
                type="button"
                onClick={() => onOpenExternal(`https://www.npmjs.com/package/${plugin.npm}`)}
                className="rounded p-1 text-faint hover:text-ink hover:bg-wash transition-colors"
                title={t.market.viewNpm}
              >
                <ExternalLink className="size-3.5" />
              </button>
            )}
            <button
              type="button"
              onClick={handleCopyCmd}
              className="rounded p-1 text-faint hover:text-ink hover:bg-wash transition-colors"
              title={t.market.copyCmd}
            >
              {copied ? <Check className="size-3.5 text-emerald-500" /> : <Copy className="size-3.5" />}
            </button>
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
