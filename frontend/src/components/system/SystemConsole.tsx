// SystemConsole.tsx —— 系统控制台与运维大盘（极简克制 Master-Detail 布局）。
import { useState } from "react"
import {
  Activity,
  ChevronRight,
  Key,
  Sliders,
  Terminal,
  Wrench,
} from "lucide-react"
import { useI18n } from "@/stores/i18nStore"
import { PreferencesPane } from "@/components/system/PreferencesPane"
import { CredentialsPane } from "@/components/system/CredentialsPane"
import { DshSettingsPane } from "@/components/system/DshSettingsPane"
import { DiagnosticsPane } from "@/components/system/DiagnosticsPane"
import { LogViewerPane } from "@/components/system/LogViewerPane"

type ConsoleSubTab = "preferences" | "credentials" | "dshSettings" | "diagnostics" | "logs"

interface NavItemConfig {
  id: ConsoleSubTab
  label: string
  icon: typeof Sliders
}

export function SystemConsole({
  onNotice,
}: {
  onNotice: (msg: string, kind?: "ok" | "warn") => void
}) {
  const { t } = useI18n()
  const [subTab, setSubTab] = useState<ConsoleSubTab>("preferences")

  const navItems: NavItemConfig[] = [
    {
      id: "preferences",
      label: t.console.tabPreferences,
      icon: Sliders,
    },
    {
      id: "credentials",
      label: t.console.tabCredentials,
      icon: Key,
    },
    {
      id: "dshSettings",
      label: t.console.tabDshSettings,
      icon: Wrench,
    },
    {
      id: "diagnostics",
      label: t.console.tabDiagnostics,
      icon: Activity,
    },
    {
      id: "logs",
      label: t.console.tabLogs,
      icon: Terminal,
    },
  ]

  return (
    <div className="grid grid-cols-1 gap-6 lg:grid-cols-12 items-start">
      {/* 左侧紧凑极简子导航（Master-Nav） */}
      <aside
        aria-label="系统控制台导航"
        className="space-y-1 lg:col-span-4 xl:col-span-3"
      >
        <nav className="space-y-1.5">
          {navItems.map((item) => {
            const Icon = item.icon
            const active = subTab === item.id

            return (
              <button
                key={item.id}
                type="button"
                role="tab"
                aria-selected={active}
                onClick={() => setSubTab(item.id)}
                className={`group flex w-full items-center justify-between rounded-xl px-3.5 py-2.5 text-left transition-all cursor-pointer ${
                  active
                    ? "bg-panel border-line text-ink border shadow-2xs font-semibold"
                    : "bg-panel/40 border-transparent hover:bg-panel hover:border-line border text-dim hover:text-ink"
                }`}
              >
                <div className="flex items-center gap-2.5 min-w-0">
                  <div
                    className={`flex size-7 shrink-0 items-center justify-center rounded-lg transition-colors ${
                      active
                        ? "bg-brand/15 text-brand"
                        : "bg-line/60 text-faint group-hover:text-dim"
                    }`}
                  >
                    <Icon className="size-3.5" />
                  </div>
                  <span className="text-xs truncate">{item.label}</span>
                </div>

                <ChevronRight
                  className={`size-3.5 shrink-0 transition-transform ${
                    active ? "text-brand translate-x-0.5" : "text-transparent group-hover:text-faint"
                  }`}
                />
              </button>
            )
          })}
        </nav>
      </aside>

      {/* 右侧主工作区详情区（Detail-Panel） */}
      <main
        aria-label="系统控制台详情区"
        className="lg:col-span-8 xl:col-span-9 min-w-0"
      >
        {subTab === "preferences" && <PreferencesPane onNotice={onNotice} />}
        {subTab === "credentials" && <CredentialsPane onNotice={onNotice} />}
        {subTab === "dshSettings" && <DshSettingsPane onNotice={onNotice} />}
        {subTab === "diagnostics" && <DiagnosticsPane onNotice={onNotice} />}
        {subTab === "logs" && <LogViewerPane onNotice={onNotice} />}
      </main>
    </div>
  )
}
