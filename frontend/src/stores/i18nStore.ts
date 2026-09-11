// i18nStore.ts —— 轻量响应式多语言状态机（4.13 i18n 引擎）。
//
// 2026-09-11（task-24）：语言偏好的**跨窗口**生效路径。各窗口是独立 JS runtime
// （AGENTS §4.4 红线 3，Zustand 不跨窗），故：
//   ① 首次载入 = 各窗口各自调一次 `initFromSettings()`（由 `App.tsx` 统一在
//      首帧门闸内调用，见该文件；本文件不再由单个页面自行初始化）；
//   ② 运行期变更 = 经 `app:settings-changed` 事件广播（Rust `set_shell_settings`
//      的载荷就是 `ShellSettings` 全量，含 `locale`）→ `applySettingsLocale()`。
// 解析逻辑抽成**纯函数**（`resolveLocaleFromSettings` / `resolveSystemLocaleFrom`），
// 三类输入（locale 存在 / 缺失 / 读取失败）可纯逻辑单测，见 `__tests__/i18nLocale.test.ts`。
import { create } from "zustand"
import { api } from "@/lib/tauri"
import { patchShellSettings } from "@/lib/shellSettings"
import { t as zhCN, type AppCopy } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"

export type LocaleKey = "zh-CN" | "en-US" | "system"

/** settings.json 里 `locale` 缺省（null/undefined）或非上述两值时 ⇒ 跟随系统。 */
export interface LocaleSettingsLike {
  locale?: string | null
}

export interface ResolvedLocale {
  /** 用户设定值：settings 存了具体语言则为该语言，否则 "system" */
  preference: LocaleKey
  /** 实际生效字典 */
  activeLocale: "zh-CN" | "en-US"
}

interface I18nState {
  /** 用户设定的语言（"zh-CN" | "en-US" | "system"） */
  preference: LocaleKey
  /** 实际生效的语言字典（"zh-CN" | "en-US"） */
  activeLocale: "zh-CN" | "en-US"
  /** 当前生效的语言文本字典 */
  t: AppCopy
  /** 切换语言并持久化到 settings.json */
  setLocale: (pref: LocaleKey) => Promise<void>
  /** 启动时从设置载入语言（各窗口各自调用一次） */
  initFromSettings: () => Promise<void>
  /** 跨窗口广播（app:settings-changed）到达时同步语言；不写盘、不产生副作用 */
  applySettingsLocale: (settings: LocaleSettingsLike | null) => void
}

/**
 * 纯函数：navigator.language → 生效语言。以 `zh` 开头取 zh-CN，其余（含缺失）= en-US。
 * 与既有行为逐字一致，仅抽出以消除对全局 `navigator` 的隐式依赖。
 */
export function resolveSystemLocaleFrom(
  navigatorLanguage: string | null | undefined,
): "zh-CN" | "en-US" {
  if (navigatorLanguage && navigatorLanguage.toLowerCase().startsWith("zh")) {
    return "zh-CN"
  }
  return "en-US"
}

/** 生产入口：读全局 navigator（浏览器预览下 navigator 可能缺失，兜底 en-US）。 */
export function resolveSystemLocale(): "zh-CN" | "en-US" {
  return resolveSystemLocaleFrom(
    typeof navigator === "undefined" ? null : navigator.language,
  )
}

/**
 * 纯函数：settings → 生效语言。三类输入语义（task-24 判据）：
 *   · `locale` 有值（"zh-CN"/"en-US"）→ 直接采用该语言，preference = 该语言；
 *   · `locale` 缺失（null/undefined/空串，= 首次运行）→ preference="system"，按系统语言；
 *   · 读取失败（调用方传 null）→ 同「缺失」：preference="system"，按系统语言。
 * 兼容既有口径：存了非 "en-US" 的未知值（如 "fr-FR"）回落 zh-CN（勿改，属既有行为）。
 */
export function resolveLocaleFromSettings(
  settings: LocaleSettingsLike | null,
  systemLanguage?: string | null,
): ResolvedLocale {
  const stored = settings?.locale
  if (stored) {
    const activeLocale = stored === "en-US" ? "en-US" : "zh-CN"
    return { preference: activeLocale, activeLocale }
  }
  return {
    preference: "system",
    activeLocale:
      systemLanguage === undefined
        ? resolveSystemLocale()
        : resolveSystemLocaleFrom(systemLanguage),
  }
}

function getDictionary(locale: "zh-CN" | "en-US"): AppCopy {
  return locale === "zh-CN" ? zhCN : enUS
}

export const useI18nStore = create<I18nState>((set) => ({
  preference: "system",
  activeLocale: "zh-CN", // 默认中文友好
  t: zhCN,

  setLocale: async (pref: LocaleKey) => {
    const effective: "zh-CN" | "en-US" =
      pref === "system" ? resolveSystemLocale() : pref

    const dict = getDictionary(effective)
    set({
      preference: pref,
      activeLocale: effective,
      t: dict,
    })

    // 持久化到 settings.json：经安全基线读改写（读失败不写，避免清空其他键，
    // 2026-09-08 裁定，见 lib/shellSettings.ts）
    try {
      await patchShellSettings({ locale: pref === "system" ? null : pref })
    } catch {
      // 忽略存储失败，保持内存中生效
    }
  },

  initFromSettings: async () => {
    // 三类输入统一走纯函数（存在 / 缺失 / 读取失败 → null），避免分支漂移：
    // 读取失败与「未设置」在当前产品口径下同为「跟随系统」（与 2026-09-08 前一致）。
    let settings: LocaleSettingsLike | null = null
    try {
      settings = await api.getShellSettings()
    } catch {
      settings = null
    }
    useI18nStore.getState().applySettingsLocale(settings)
  },

  applySettingsLocale: (settings) => {
    const { preference, activeLocale } = resolveLocaleFromSettings(settings)
    set({
      preference,
      activeLocale,
      t: getDictionary(activeLocale),
    })
  },
}))

/// 便捷 hook 统一暴露 t
export function useI18n() {
  const { t, preference, activeLocale, setLocale } = useI18nStore()
  return { t, preference, activeLocale, setLocale }
}
