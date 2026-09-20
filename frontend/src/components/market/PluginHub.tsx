// components/market/PluginHub.tsx —— 插件中心（统一承载「插件市场」与「已安装总览」两大子视图）
//
// 2026-09-20（ADR-0028）：「实验能力」子页下线——能力开关迁入 Profile 详情页
// （与「插件列表」同作用域），本组件收敛为跨档视角的市场 / 已安装两子页。
import { useState } from "react"
import { Layers, Store } from "lucide-react"
import { useI18n } from "@/stores/i18nStore"
import { MarketplaceView } from "@/components/market/MarketplaceView"
import { PluginOverview } from "@/components/profiles/PluginOverview"
import { QueuePanel } from "@/components/market/QueuePanel"
import { InstallFlight } from "@/components/market/InstallFlight"
import { Segmented } from "@/components/ui/segmented"

interface PluginHubProps {
  refreshKey: number
  onNotice?: (text: string, kind?: "ok" | "warn") => void
}

export function PluginHub({
  refreshKey,
  onNotice,
}: PluginHubProps) {
  const { t } = useI18n()
  const [subTab, setSubTab] = useState<"market" | "installed">("market")

  return (
    <>
      <div className="space-y-4">
        {/* 插件中心内部子 Tab 切换器（吸顶保证在长列表滚动时下载管理入口始终可见）。
            2026-09-18 收口：三枚手搓 tab 按钮 → 统一 Segmented 基座。 */}
        <div className="sticky top-14 z-15 -mx-2 -mt-2 flex items-center justify-between gap-3 rounded-2xl border-b border-line/60 bg-bg/95 px-2 py-2.5 backdrop-blur-md transition-all">
          <Segmented<"market" | "installed">
            size="sm"
            ariaLabel={t.market.subtabMarket}
            options={[
              { value: "market", label: t.market.subtabMarket, icon: Store },
              { value: "installed", label: t.market.subtabInstalled, icon: Layers },
            ]}
            value={subTab}
            onChange={setSubTab}
          />

          {/* 下载管理（095 #4：队列项状态一览） */}
          <QueuePanel />
        </div>

        {/* 子视图渲染（2026-09-18 收口：key 重挂 + page-rise 补上切换动效，
            与控制台 tab 面板同款） */}
        <div key={subTab} className="page-rise">
          {subTab === "market" ? (
            <MarketplaceView onNotice={onNotice} />
          ) : (
            <PluginOverview refreshKey={refreshKey} onNotice={onNotice} />
          )}
        </div>
      </div>

      {/* 入队飞行层（问题记录-2026-09-09 §1.1）：fixed 覆盖层，放在 space-y
        容器之外——否则会被 `> * + *` 的 margin 推离视口边缘 */}
      <InstallFlight />
    </>
  )
}
