// i18nStore.ts —— 轻量响应式多语言状态机（4.13 i18n 引擎）。
import { create } from "zustand"
import { api } from "@/lib/tauri"
import { t as zhCN, type AppCopy } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"

export type LocaleKey = "zh-CN" | "en-US" | "system"

interface I18nState {
  /** 用户设定的语言（"zh-CN" | "en-US" | "system"） */
  preference: LocaleKey
  /** 实际生效的语言字典（"zh-CN" | "en-US"） */
  activeLocale: "zh-CN" | "en-US"
  /** 当前生效的语言文本字典 */
  t: AppCopy
  /** 切换语言并持久化到 settings.json */
  setLocale: (pref: LocaleKey) => Promise<void>
  /** 启动时从设置载入语言 */
  initFromSettings: () => Promise<void>
}

function resolveSystemLocale(): "zh-CN" | "en-US" {
  if (typeof navigator !== "undefined" && navigator.language) {
    const lang = navigator.language.toLowerCase()
    if (lang.startsWith("zh")) {
      return "zh-CN"
    }
  }
  return "en-US"
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

    // 持久化到 settings.json
    try {
      const curr = await api.getShellSettings().catch(() => ({}))
      await api.setShellSettings({
        ...curr,
        locale: pref === "system" ? null : pref,
      })
    } catch {
      // 忽略存储失败，保持内存中生效
    }
  },

  initFromSettings: async () => {
    try {
      const settings = await api.getShellSettings()
      if (settings.locale) {
        const pref: LocaleKey =
          settings.locale === "en-US" ? "en-US" : "zh-CN"
        const effective = pref
        set({
          preference: pref,
          activeLocale: effective,
          t: getDictionary(effective),
        })
      } else {
        const effective = resolveSystemLocale()
        set({
          preference: "system",
          activeLocale: effective,
          t: getDictionary(effective),
        })
      }
    } catch {
      const effective = resolveSystemLocale()
      set({
        preference: "system",
        activeLocale: effective,
        t: getDictionary(effective),
      })
    }
  },
}))

/// 便捷 hook 统一暴露 t
export function useI18n() {
  const { t, preference, activeLocale, setLocale } = useI18nStore()
  return { t, preference, activeLocale, setLocale }
}
