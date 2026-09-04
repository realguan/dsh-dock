// QuickDshSwitcher: 控制中心顶栏返回 DSH 主工作台的精致胶囊挂件。
// 视觉上采用半透毛玻璃胶囊设计，平时精简展示「返回工作台」，Hover 时平滑展开快捷键徽章。
// 支持与主工作台共用一组快捷键（⌘, / Ctrl+,）实现双向来回一键 Toggle 切换。
import { useEffect, useState } from "react"
import { ArrowUpRight } from "lucide-react"
import { api } from "@/lib/tauri"
import { listen } from "@tauri-apps/api/event"
import { usePlatform } from "@/hooks/usePlatform"
import { useI18n } from "@/stores/i18nStore"
import { useProfilesStore } from "@/stores/profilesStore"
import { logger } from "@/lib/logger"
import type { ShellSettings } from "@/types/ipc"

export function QuickDshSwitcher() {
  const { t } = useI18n()
  const { platform } = usePlatform()
  const isMac = platform.os === "macos"
  const { activeProfile } = useProfilesStore()
  const [switching, setSwitching] = useState(false)
  const [shortcutKey, setShortcutKey] = useState<string>("default")

  useEffect(() => {
    api.getShellSettings()
      .then((s) => {
        if (s.switcherShortcut) {
          setShortcutKey(s.switcherShortcut)
        }
      })
      .catch(() => {})

    let unlisten: (() => void) | undefined
    listen<ShellSettings>("app:settings-changed", (e) => {
      if (e.payload && e.payload.switcherShortcut) {
        setShortcutKey(e.payload.switcherShortcut)
      } else {
        setShortcutKey("default")
      }
    }).then((u) => {
      unlisten = u
    }).catch(() => {})

    return () => {
      if (unlisten) unlisten()
    }
  }, [])

  const shortcutDisplay = shortcutKey === "shift_p"
    ? (isMac ? "⌘ + ⇧ + P" : "Ctrl + ⇧ + P")
    : (isMac ? "⌘ + ," : "Ctrl + ,")

  const handleSwitch = () => {
    setSwitching(true)
    api.focusMainWindow()
      .catch((e) => {
        logger.error("quick-switcher", "聚焦主窗口失败", { error: String(e) })
      })
      .finally(() => {
        setTimeout(() => setSwitching(false), 300)
      })
  }

  // 快捷键支持：共用快捷键 Cmd/Ctrl + , (Toggle) 以及 Cmd/Ctrl + 1 / Enter / Shift+P 快速聚焦主窗口
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // 兼顾 metaKey (macOS) 与 ctrlKey (Windows)，增强容错
      const isModPressed = isMac ? e.metaKey : (e.ctrlKey || e.metaKey)
      const matchComma = isModPressed && !e.shiftKey && (e.key === "," || e.code === "Comma")
      const matchOneOrEnter = isModPressed && (e.key === "1" || e.code === "Digit1" || e.key === "Enter" || e.code === "Enter")
      const matchShiftP = isModPressed && e.shiftKey && (e.key === "P" || e.key === "p" || e.code === "KeyP")

      if (matchComma || matchOneOrEnter || matchShiftP) {
        e.preventDefault()
        e.stopPropagation()
        handleSwitch()
      }
    }
    // 捕获阶段优先截获快捷键
    window.addEventListener("keydown", handleKeyDown, true)
    return () => window.removeEventListener("keydown", handleKeyDown, true)
  }, [isMac])

  return (
    <button
      type="button"
      onClick={handleSwitch}
      disabled={switching}
      title={t.profiles.switchToDshTip}
      aria-label={t.profiles.switchToDshTip}
      className="group relative inline-flex h-8 shrink-0 items-center gap-1.5 rounded-full border border-line bg-panel/80 px-2.5 text-xs font-semibold text-ink shadow-2xs backdrop-blur-md transition-all duration-200 hover:border-brand/40 hover:bg-panel hover:shadow-xs active:scale-95 cursor-pointer disabled:opacity-70"
    >
      {/* 活跃状态微型呼吸指示灯 */}
      <span className="relative flex size-2 shrink-0 items-center justify-center">
        <span className="absolute inline-flex size-full animate-ping rounded-full bg-emerald-400 opacity-60" />
        <span className="relative inline-flex size-1.5 rounded-full bg-emerald-500" />
      </span>

      {/* 文本主体 */}
      <span className="font-medium tracking-tight text-ink/90 group-hover:text-brand transition-colors whitespace-nowrap">
        {t.profiles.switchToDsh}
      </span>

      {/* 关联 Profile 简要标记（若存在） */}
      {activeProfile && (
        <span className="hidden font-mono text-[10px] text-faint max-w-14 truncate whitespace-nowrap sm:inline-block">
          ({activeProfile})
        </span>
      )}

      {/* 快捷键徽章：平时隐藏，Hover 时平滑展开（禁止折行，预留充裕宽度） */}
      <span className="max-w-0 opacity-0 overflow-hidden group-hover:max-w-32 group-hover:opacity-100 transition-all duration-200 ease-out inline-flex items-center shrink-0">
        <kbd className="ml-1 inline-flex items-center whitespace-nowrap rounded-md bg-line-soft px-1.5 py-0.5 font-mono text-[9px] font-semibold text-dim border border-line/60 shadow-2xs leading-none">
          {shortcutDisplay}
        </kbd>
      </span>

      {/* 跳转微图标 */}
      <ArrowUpRight className="size-3 shrink-0 text-faint group-hover:text-brand group-hover:translate-x-0.5 group-hover:-translate-y-0.5 transition-all" />
    </button>
  )
}
