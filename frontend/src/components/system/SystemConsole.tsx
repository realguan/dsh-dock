// SystemConsole.tsx —— 系统控制台与运维大盘（极简克制 Master-Detail 布局）。
import { useState, type KeyboardEvent } from "react"
import {
  Activity,
  ChevronRight,
  Key,
  Sliders,
  Terminal,
  Wrench,
} from "lucide-react"
import { useI18n } from "@/stores/i18nStore"
import { IconChip } from "@/components/ui/icon-chip"
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
  /** 2026-09-18 收口：图标底统一 IconChip，tone 按导航语义分配。 */
  tone: "brand" | "ok" | "neutral"
}

/** roving tabindex 的可达性前提：非选中 tab 不可 Tab 聚焦，方向键负责遍历。 */
const NAV_PANEL_ID = "console-detail"

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
      tone: "brand",
    },
    {
      id: "credentials",
      label: t.console.tabCredentials,
      icon: Key,
      tone: "brand",
    },
    {
      id: "dshSettings",
      label: t.console.tabDshSettings,
      icon: Wrench,
      tone: "brand",
    },
    {
      id: "diagnostics",
      label: t.console.tabDiagnostics,
      icon: Activity,
      tone: "ok",
    },
    {
      id: "logs",
      label: t.console.tabLogs,
      icon: Terminal,
      tone: "neutral",
    },
  ]

  // tablist 键盘协议：↑← 上一个 / ↓→ 下一个 / Home / End（不引新依赖）
  const handleNavKeyDown = (e: KeyboardEvent<HTMLElement>) => {
    if (!["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key))
      return
    e.preventDefault()
    const idx = navItems.findIndex((n) => n.id === subTab)
    const last = navItems.length - 1
    let next = idx
    if (e.key === "Home") next = 0
    else if (e.key === "End") next = last
    else if (e.key === "ArrowDown" || e.key === "ArrowRight")
      next = idx >= last ? 0 : idx + 1
    else next = idx <= 0 ? last : idx - 1
    const id = navItems[next].id
    setSubTab(id)
    document.getElementById(`console-tab-${id}`)?.focus()
  }

  return (
    <div className="grid grid-cols-1 gap-6 md:grid-cols-12 items-start">
      {/* 左侧紧凑极简子导航（Master-Nav）。批次 3：跟随吸顶——右侧详情最长 900px，
          导航只有 274px，滚到底就只剩一片空白且换面板要滚回顶。 */}
      <aside className="space-y-1 md:sticky md:top-16 md:col-span-4 xl:col-span-3">
        <nav
          role="tablist"
          aria-orientation="vertical"
          aria-label={t.console.navLabel}
          onKeyDown={handleNavKeyDown}
          className="space-y-1.5"
        >
          {navItems.map((item) => {
            const active = subTab === item.id

            return (
              <button
                key={item.id}
                type="button"
                role="tab"
                id={`console-tab-${item.id}`}
                aria-selected={active}
                aria-controls={NAV_PANEL_ID}
                tabIndex={active ? 0 : -1}
                onClick={() => setSubTab(item.id)}
                className={`group flex w-full items-center justify-between rounded-xl px-3.5 py-2.5 text-left transition-all cursor-pointer ${
                  active
                    ? "bg-panel border-line text-ink border shadow-2xs font-semibold"
                    : "bg-panel/40 border-transparent hover:bg-panel hover:border-line border text-dim hover:text-ink"
                }`}
              >
                <div className="flex items-center gap-2.5 min-w-0">
                  <IconChip icon={item.icon} tone={item.tone} />
                  <span className="text-xs truncate" title={item.label}>
                    {item.label}
                  </span>
                </div>

                <ChevronRight
                  className={`size-3.5 shrink-0 transition-transform ${
                    active ? "text-brand-deep translate-x-0.5" : "text-transparent group-hover:text-faint"
                  }`}
                />
              </button>
            )
          })}
        </nav>
      </aside>

      {/* 右侧主工作区详情区（Detail-Panel） */}
      <main
        aria-label={t.console.detailLabel}
        className="md:col-span-8 xl:col-span-9 min-w-0"
      >
        {/* 2026-09-18 收口：key 重挂 + page-rise 补上 tab 切换动效 */}
        <div
          key={subTab}
          role="tabpanel"
          id={NAV_PANEL_ID}
          aria-labelledby={`console-tab-${subTab}`}
          className="page-rise"
        >
          {subTab === "preferences" && <PreferencesPane onNotice={onNotice} />}
          {subTab === "credentials" && <CredentialsPane onNotice={onNotice} />}
          {subTab === "dshSettings" && <DshSettingsPane onNotice={onNotice} />}
          {subTab === "diagnostics" && <DiagnosticsPane onNotice={onNotice} />}
          {subTab === "logs" && <LogViewerPane onNotice={onNotice} />}
        </div>
      </main>
    </div>
  )
}
