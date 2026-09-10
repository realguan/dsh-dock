// components/market/PluginHub.tsx —— 插件中心（统一承载「插件市场」与「已安装总览」两大子视图）
import { useState } from "react"
import { Layers, Store } from "lucide-react"
import { useI18n } from "@/stores/i18nStore"
import { MarketplaceView } from "@/components/market/MarketplaceView"
import { PluginOverview } from "@/components/profiles/PluginOverview"
import { QueuePanel } from "@/components/market/QueuePanel"
import { InstallFlight } from "@/components/market/InstallFlight"

interface PluginHubProps {
  refreshKey: number
  onNotice?: (text: string, kind?: "ok" | "warn") => void
}

export function PluginHub({ refreshKey, onNotice }: PluginHubProps) {
  const { t } = useI18n()
  const [subTab, setSubTab] = useState<"market" | "installed">("market")

  return (
    <>
      <div className="space-y-4">
        {/* 插件中心内部子 Tab 切换器（吸顶保证在长列表滚动时下载管理入口始终可见） */}
        <div className="sticky top-14 z-15 -mx-2 -mt-2 flex items-center justify-between gap-3 rounded-2xl border-b border-line/60 bg-bg/95 px-2 py-2.5 backdrop-blur-md transition-all">
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
              <Store className={`size-3.5 ${subTab === "market" ? "text-brand-deep" : "text-faint"}`} />
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
              <Layers className={`size-3.5 ${subTab === "installed" ? "text-brand-deep" : "text-faint"}`} />
              <span>{t.market.subtabInstalled}</span>
            </button>
          </div>

          {/* 下载管理（095 #4：队列项状态一览） */}
          <QueuePanel />
        </div>

        {/* 子视图渲染 */}
        {subTab === "market" ? (
          <MarketplaceView onNotice={onNotice} />
        ) : (
          <PluginOverview refreshKey={refreshKey} onNotice={onNotice} />
        )}
      </div>

      {/* 入队飞行层（问题记录-2026-09-09 §1.1）：fixed 覆盖层，放在 space-y
          容器之外——否则会被 `> * + *` 的 margin 推离视口边缘 */}
      <InstallFlight />
    </>
  )
}
