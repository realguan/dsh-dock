// components/market/PluginHub.tsx —— 插件中心（统一承载「插件市场」与「已安装总览」两大子视图）
import { useState } from "react"
import { Layers, Store } from "lucide-react"
import { useI18n } from "@/stores/i18nStore"
import { MarketplaceView } from "@/components/market/MarketplaceView"
import { PluginOverview } from "@/components/profiles/PluginOverview"

interface PluginHubProps {
  refreshKey: number
  onNotice?: (text: string, kind?: "ok" | "warn") => void
}

export function PluginHub({ refreshKey, onNotice }: PluginHubProps) {
  const { t } = useI18n()
  const [subTab, setSubTab] = useState<"market" | "installed">("market")

  return (
    <div className="space-y-4">
      {/* 插件中心内部子 Tab 切换器 */}
      <div className="flex items-center justify-between gap-3 border-b border-line/60 pb-3">
        <div
          role="tablist"
          className="flex items-center gap-1 rounded-xl border border-line bg-wash p-1 shadow-2xs"
        >
          <button
            type="button"
            role="tab"
            aria-selected={subTab === "market"}
            onClick={() => setSubTab("market")}
            className={`flex items-center gap-2 rounded-lg px-3.5 py-1.5 text-xs font-medium transition-all ${
              subTab === "market"
                ? "bg-panel text-ink shadow-xs font-semibold"
                : "text-dim hover:text-ink hover:bg-panel/40"
            }`}
          >
            <Store className={`size-3.5 ${subTab === "market" ? "text-brand" : "text-faint"}`} />
            <span>{t.market.subtabMarket}</span>
          </button>

          <button
            type="button"
            role="tab"
            aria-selected={subTab === "installed"}
            onClick={() => setSubTab("installed")}
            className={`flex items-center gap-2 rounded-lg px-3.5 py-1.5 text-xs font-medium transition-all ${
              subTab === "installed"
                ? "bg-panel text-ink shadow-xs font-semibold"
                : "text-dim hover:text-ink hover:bg-panel/40"
            }`}
          >
            <Layers className={`size-3.5 ${subTab === "installed" ? "text-brand" : "text-faint"}`} />
            <span>{t.market.subtabInstalled}</span>
          </button>
        </div>
      </div>

      {/* 子视图渲染 */}
      {subTab === "market" ? (
        <MarketplaceView onNotice={onNotice} />
      ) : (
        <PluginOverview refreshKey={refreshKey} onNotice={onNotice} />
      )}
    </div>
  )
}
