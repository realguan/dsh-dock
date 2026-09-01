// PreferencesPane.tsx —— 界面语言偏好与崩溃高可用守护配置（4.12 & 4.13）。
import { useEffect, useState } from "react"
import {
  Check,
  Globe,
  Keyboard,
  LoaderCircle,
  Shield,
  ShieldAlert,
  ShieldCheck,
  SlidersHorizontal,
  Sparkles,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { usePlatform } from "@/hooks/usePlatform"
import { useI18n, type LocaleKey } from "@/stores/i18nStore"
import { Switch } from "@/components/ui/switch"
import type { ShellSettings } from "@/types/ipc"

export function PreferencesPane({
  onNotice,
}: {
  onNotice: (msg: string, kind?: "ok" | "warn") => void
}) {
  const { t, preference, setLocale } = useI18n()
  const { platform } = usePlatform()
  const isMac = platform.os === "macos"
  const [settings, setSettings] = useState<ShellSettings | null>(null)
  const [saving, setSaving] = useState(false)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    api
      .getShellSettings()
      .then((s) => setSettings(s))
      .catch(() => setSettings({}))
      .finally(() => setLoading(false))
  }, [])

  const handleLanguageChange = (key: LocaleKey) => {
    void setLocale(key).then(() => {
      onNotice(t.console.saveSuccess, "ok")
    })
  }

  const handleToggleAutoRestart = async (checked: boolean) => {
    if (!settings || saving) return
    setSaving(true)
    const next: ShellSettings = {
      ...settings,
      autoRestart: checked,
    }
    try {
      await api.setShellSettings(next)
      setSettings(next)
      onNotice(t.console.saveSuccess, "ok")
    } catch (e) {
      onNotice(`${t.console.saveFailed}: ${e}`, "warn")
    } finally {
      setSaving(false)
    }
  }

  const handleToggleFloatingSwitcher = async (checked: boolean) => {
    if (!settings || saving) return
    setSaving(true)
    const next: ShellSettings = {
      ...settings,
      showFloatingSwitcher: checked,
    }
    try {
      await api.setShellSettings(next)
      setSettings(next)
      onNotice(t.console.saveSuccess, "ok")
    } catch (e) {
      onNotice(`${t.console.saveFailed}: ${e}`, "warn")
    } finally {
      setSaving(false)
    }
  }

  const handleChangeShortcut = async (key: string) => {
    if (!settings || saving) return
    setSaving(true)
    const next: ShellSettings = {
      ...settings,
      switcherShortcut: key,
    }
    try {
      await api.setShellSettings(next)
      setSettings(next)
      onNotice(t.console.saveSuccess, "ok")
    } catch (e) {
      onNotice(`${t.console.saveFailed}: ${e}`, "warn")
    } finally {
      setSaving(false)
    }
  }

  if (loading) {
    return (
      <div className="flex h-64 items-center justify-center text-xs text-faint">
        <LoaderCircle className="mr-2 size-4 animate-spin text-brand" />
        <span>正在加载偏好设置…</span>
      </div>
    )
  }

  const autoRestartActive = settings?.autoRestart ?? false
  const floatingSwitcherActive = settings?.showFloatingSwitcher ?? true
  const shortcutChoice = settings?.switcherShortcut ?? "default"

  return (
    <div className="space-y-6">
      {/* 模块 1：界面语言选择 */}
      <section className="rounded-2xl border border-line bg-panel p-5 shadow-2xs">
        <div className="flex items-center gap-2.5 mb-1.5">
          <div className="flex size-7 items-center justify-center rounded-lg bg-brand/10 text-brand">
            <Globe className="size-4" />
          </div>
          <div>
            <h3 className="text-sm font-semibold text-ink">
              {t.console.localeLabel}
            </h3>
            <p className="text-xs text-faint">{t.console.localeDesc}</p>
          </div>
        </div>

        <div className="mt-4 grid grid-cols-1 gap-2.5 sm:grid-cols-3">
          {/* 跟随系统 */}
          <button
            type="button"
            onClick={() => handleLanguageChange("system")}
            className={`group flex items-center justify-between rounded-xl border p-3.5 text-left transition-all ${
              preference === "system"
                ? "border-brand bg-brand/5 shadow-xs"
                : "border-line bg-bg hover:border-line-hover"
            }`}
          >
            <div>
              <div className="flex items-center gap-1.5">
                <Sparkles className="size-3.5 text-faint" />
                <span className="text-xs font-semibold text-ink">
                  {t.console.localeSystem}
                </span>
              </div>
              <p className="mt-1 text-[11px] text-faint">Auto Detect</p>
            </div>
            {preference === "system" && <Check className="size-4 text-brand" />}
          </button>

          {/* 简体中文 */}
          <button
            type="button"
            onClick={() => handleLanguageChange("zh-CN")}
            className={`group flex items-center justify-between rounded-xl border p-3.5 text-left transition-all ${
              preference === "zh-CN"
                ? "border-brand bg-brand/5 shadow-xs"
                : "border-line bg-bg hover:border-line-hover"
            }`}
          >
            <div>
              <span className="text-xs font-semibold text-ink">
                {t.console.localeZh}
              </span>
              <p className="mt-1 text-[11px] text-faint">简体中文 (默认)</p>
            </div>
            {preference === "zh-CN" && <Check className="size-4 text-brand" />}
          </button>

          {/* English */}
          <button
            type="button"
            onClick={() => handleLanguageChange("en-US")}
            className={`group flex items-center justify-between rounded-xl border p-3.5 text-left transition-all ${
              preference === "en-US"
                ? "border-brand bg-brand/5 shadow-xs"
                : "border-line bg-bg hover:border-line-hover"
            }`}
          >
            <div>
              <span className="text-xs font-semibold text-ink">
                {t.console.localeEn}
              </span>
              <p className="mt-1 text-[11px] text-faint">English (US)</p>
            </div>
            {preference === "en-US" && <Check className="size-4 text-brand" />}
          </button>
        </div>
      </section>

      {/* 模块 2：崩溃自动恢复与熔断守护 */}
      <section className="rounded-2xl border border-line bg-panel p-5 shadow-2xs">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="flex items-start gap-3">
            <div
              className={`flex size-8 shrink-0 items-center justify-center rounded-xl transition-colors ${
                autoRestartActive
                  ? "bg-ok-soft text-ok border border-ok/20"
                  : "bg-line-soft text-faint"
              }`}
            >
              {autoRestartActive ? (
                <ShieldCheck className="size-4" />
              ) : (
                <Shield className="size-4" />
              )}
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-sm font-semibold text-ink">
                  {t.console.autoRestartLabel}
                </h3>
                <span
                  className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium leading-none ${
                    autoRestartActive
                      ? "bg-ok-soft text-ok"
                      : "bg-line-soft text-faint"
                  }`}
                >
                  <span
                    className={`size-1.5 rounded-full ${
                      autoRestartActive ? "bg-ok animate-pulse" : "bg-faint"
                    }`}
                  />
                  {autoRestartActive
                    ? t.console.autoRestartEnabled
                    : t.console.autoRestartDisabled}
                </span>
              </div>
              <p className="mt-1 max-w-xl text-xs leading-relaxed text-dim">
                {t.console.autoRestartDesc}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <Switch
              checked={autoRestartActive}
              disabled={saving}
              onCheckedChange={handleToggleAutoRestart}
            />
          </div>
        </div>

        {/* 熔断机制图解卡片 */}
        <div className="mt-4 rounded-xl border border-line/80 bg-bg p-3.5 text-xs text-dim">
          <div className="flex items-center gap-2 font-medium text-ink">
            <ShieldAlert className="size-3.5 text-amber-500" />
            <span>智能熔断保护协议（Circuit Breaker）</span>
          </div>
          <div className="mt-2 grid grid-cols-1 gap-2 sm:grid-cols-3 font-mono text-[11px]">
            <div className="rounded-lg bg-panel p-2 border border-line">
              <span className="text-faint">监控窗口：</span>
              <span className="text-ink font-semibold ml-1">60 秒滑动窗口</span>
            </div>
            <div className="rounded-lg bg-panel p-2 border border-line">
              <span className="text-faint">熔断阈值：</span>
              <span className="text-amber-500 font-semibold ml-1">连续 3 次崩溃</span>
            </div>
            <div className="rounded-lg bg-panel p-2 border border-line">
              <span className="text-faint">熔断后动作：</span>
              <span className="text-ink font-semibold ml-1">停机并弹诊断卡</span>
            </div>
          </div>
        </div>
      </section>

      {/* 模块 3：工作台快速切换与悬浮胶囊 */}
      <section className="rounded-2xl border border-line bg-panel p-5 shadow-2xs">
        <div className="flex flex-wrap items-start justify-between gap-4">
          <div className="flex items-start gap-3">
            <div
              className={`flex size-8 shrink-0 items-center justify-center rounded-xl transition-colors ${
                floatingSwitcherActive
                  ? "bg-brand/15 text-brand border border-brand/20"
                  : "bg-line-soft text-faint"
              }`}
            >
              <SlidersHorizontal className="size-4" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <h3 className="text-sm font-semibold text-ink">
                  {t.console.floatingSwitcherLabel}
                </h3>
                <span
                  className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] font-medium leading-none ${
                    floatingSwitcherActive
                      ? "bg-brand/10 text-brand"
                      : "bg-line-soft text-faint"
                  }`}
                >
                  <span
                    className={`size-1.5 rounded-full ${
                      floatingSwitcherActive ? "bg-brand animate-pulse" : "bg-faint"
                    }`}
                  />
                  {floatingSwitcherActive
                    ? t.console.floatingSwitcherEnabled
                    : t.console.floatingSwitcherDisabled}
                </span>
              </div>
              <p className="mt-1 max-w-xl text-xs leading-relaxed text-dim">
                {t.console.floatingSwitcherDesc}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <Switch
              checked={floatingSwitcherActive}
              disabled={saving}
              onCheckedChange={handleToggleFloatingSwitcher}
            />
          </div>
        </div>

        {/* 快捷键配置 */}
        <div className="mt-5 border-t border-line/60 pt-4">
          <div className="flex items-center gap-2 mb-2.5">
            <Keyboard className="size-3.5 text-faint" />
            <h4 className="text-xs font-semibold text-ink">
              {t.console.shortcutLabel}
            </h4>
            <span className="text-[11px] text-faint">
              ({t.console.shortcutDesc})
            </span>
          </div>

          <div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2">
            {/* 默认快捷键 */}
            <button
              type="button"
              onClick={() => handleChangeShortcut("default")}
              className={`group flex items-center justify-between rounded-xl border p-3.5 text-left transition-all ${
                shortcutChoice === "default"
                  ? "border-brand bg-brand/5 shadow-xs"
                  : "border-line bg-bg hover:border-line-hover"
              }`}
            >
              <div>
                <span className="text-xs font-semibold text-ink">
                  {t.console.shortcutDefault}
                </span>
                <p className="mt-1 font-mono text-[10px] text-faint">
                  {isMac ? "⌘ + ," : "Ctrl + ,"}
                </p>
              </div>
              {shortcutChoice === "default" && <Check className="size-4 text-brand" />}
            </button>

            {/* 命令面板风格 */}
            <button
              type="button"
              onClick={() => handleChangeShortcut("shift_p")}
              className={`group flex items-center justify-between rounded-xl border p-3.5 text-left transition-all ${
                shortcutChoice === "shift_p"
                  ? "border-brand bg-brand/5 shadow-xs"
                  : "border-line bg-bg hover:border-line-hover"
              }`}
            >
              <div>
                <span className="text-xs font-semibold text-ink">
                  {t.console.shortcutShiftP}
                </span>
                <p className="mt-1 font-mono text-[10px] text-faint">
                  {isMac ? "⌘ + ⇧ + P" : "Ctrl + ⇧ + P"}
                </p>
              </div>
              {shortcutChoice === "shift_p" && <Check className="size-4 text-brand" />}
            </button>
          </div>
        </div>
      </section>
    </div>
  )
}
