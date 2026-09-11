// i18nLocale.test.ts —— 语言解析纯逻辑门禁（2026-09-11，task-24）。
// 背景：语言偏好跨窗口不生效——`initFromSettings()` 原先全仓只有一处调用
// （profiles 窗口），主窗口/About 窗口恒用 store 初始字典 zhCN ⇒ 选过 en-US 的
// 用户在启动序列与 About 仍见中文。修复把解析抽成纯函数并让每个窗口各自初始化
// （App.tsx）+ 订阅 app:settings-changed（跨窗同步）。
// 本测试覆盖三类输入（locale 存在 / 缺失 / 读取失败 → null）后的生效语言，
// 以及 store 的同步入口 `applySettingsLocale`（广播到达路径）。
// 纯逻辑测试：不渲染 DOM、不引 RTL/jsdom（AGENTS §4.3 末段）。
import { beforeEach, describe, expect, it, vi } from "vitest"

/** 让 `initFromSettings()` 走真实代码路径：mock 掉 IPC 层（同 shellSettings.test.ts 口径）。 */
const ipc = vi.hoisted(() => ({
  getShellSettings: vi.fn<() => Promise<{ locale?: string | null }>>(),
}))
vi.mock("@/lib/tauri", () => ({ api: { getShellSettings: ipc.getShellSettings } }))

import {
  resolveLocaleFromSettings,
  resolveSystemLocale,
  resolveSystemLocaleFrom,
  useI18nStore,
} from "@/stores/i18nStore"
import { t as zhCN } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"
import appSrc from "@/App.tsx?raw"
import profileManagerSrc from "@/pages/ProfileManager.tsx?raw"

/**
 * 断言「某处不再调用 X」时必须**剥离注释**——源码注释里会引用被移除的调用
 * （本组注释正是如此），把注释当代码判红是假阳性（同 task-20 的教训）。
 * 剥离后另有反向断言，防过度剥离导致假绿。
 */
function stripComments(src: string): string {
  return src
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^[ \t]*\/\/.*$/gm, "")
}

const appCode = stripComments(appSrc)
const profileManagerCode = stripComments(profileManagerSrc)

describe("语言解析纯函数（task-24）", () => {
  describe("resolveSystemLocaleFrom：系统语言 → 生效语言", () => {
    it("zh 开头 → zh-CN（大小写不敏感）", () => {
      expect(resolveSystemLocaleFrom("zh")).toBe("zh-CN")
      expect(resolveSystemLocaleFrom("zh-CN")).toBe("zh-CN")
      expect(resolveSystemLocaleFrom("ZH-hant-TW")).toBe("zh-CN")
    })

    it("非 zh → en-US", () => {
      expect(resolveSystemLocaleFrom("en-US")).toBe("en-US")
      expect(resolveSystemLocaleFrom("ja-JP")).toBe("en-US")
      // 「zh」不是开头就不算中文——与既有逐字行为一致
      expect(resolveSystemLocaleFrom("en-zh")).toBe("en-US")
    })

    it("缺失/空值 → en-US", () => {
      expect(resolveSystemLocaleFrom(null)).toBe("en-US")
      expect(resolveSystemLocaleFrom(undefined)).toBe("en-US")
      expect(resolveSystemLocaleFrom("")).toBe("en-US")
    })

    it("生产入口读全局 navigator 且不抛（浏览器预览无 navigator 时兜底）", () => {
      expect(["zh-CN", "en-US"]).toContain(resolveSystemLocale())
    })
  })

  describe("resolveLocaleFromSettings：三类输入", () => {
    it("① locale 存在 → 直接采用，preference 即该语言", () => {
      expect(resolveLocaleFromSettings({ locale: "en-US" }, "zh-CN")).toEqual({
        preference: "en-US",
        activeLocale: "en-US",
      })
      expect(resolveLocaleFromSettings({ locale: "zh-CN" }, "en-US")).toEqual({
        preference: "zh-CN",
        activeLocale: "zh-CN",
      })
    })

    it("① 兼容既有口径：已存语言优先于系统语言", () => {
      // 存了 zh-CN 而系统是 en-US ⇒ 仍用 zh-CN（用户显式选择优先）
      expect(resolveLocaleFromSettings({ locale: "zh-CN" }, "en-US").activeLocale).toBe("zh-CN")
    })

    it("① 未知存量值回落 zh-CN（既有行为，勿静默改动）", () => {
      expect(resolveLocaleFromSettings({ locale: "fr-FR" }, "en-US")).toEqual({
        preference: "zh-CN",
        activeLocale: "zh-CN",
      })
    })

    it("② locale 缺失 → preference=system，按系统语言", () => {
      expect(resolveLocaleFromSettings({}, "zh-CN")).toEqual({
        preference: "system",
        activeLocale: "zh-CN",
      })
      expect(resolveLocaleFromSettings({ locale: null }, "en-US")).toEqual({
        preference: "system",
        activeLocale: "en-US",
      })
      expect(resolveLocaleFromSettings({ locale: "" }, "en-US")).toEqual({
        preference: "system",
        activeLocale: "en-US",
      })
      expect(resolveLocaleFromSettings({ locale: undefined }, "en-US").preference).toBe("system")
    })

    it("③ 读取失败（null）→ 与缺失同口径：system + 系统语言", () => {
      expect(resolveLocaleFromSettings(null, "zh-CN")).toEqual({
        preference: "system",
        activeLocale: "zh-CN",
      })
      expect(resolveLocaleFromSettings(null, "en-US")).toEqual({
        preference: "system",
        activeLocale: "en-US",
      })
    })

    it("未注入系统语言时回落到读全局 navigator", () => {
      const { activeLocale } = resolveLocaleFromSettings({})
      expect(["zh-CN", "en-US"]).toContain(activeLocale)
      expect(resolveLocaleFromSettings({}).activeLocale).toBe(resolveSystemLocale())
    })
  })
})

describe("store 同步入口 applySettingsLocale（广播到达路径）", () => {
  beforeEach(() => {
    // 每个用例前还原到「未设置」初始态，避免用例间耦合
    useI18nStore.setState({ preference: "system", activeLocale: "zh-CN", t: zhCN })
  })

  it("广播载荷带 locale ⇒ 切换字典（跨窗同步的核心）", () => {
    useI18nStore.getState().applySettingsLocale({ locale: "en-US" })
    expect(useI18nStore.getState().activeLocale).toBe("en-US")
    expect(useI18nStore.getState().t).toBe(enUS)
    expect(useI18nStore.getState().preference).toBe("en-US")
  })

  it("载荷 locale 为 null（改回跟随系统）⇒ preference 回落 system", () => {
    useI18nStore.getState().applySettingsLocale({ locale: "en-US" })
    useI18nStore.getState().applySettingsLocale({ locale: null })
    const s = useI18nStore.getState()
    expect(s.preference).toBe("system")
    expect([zhCN, enUS]).toContain(s.t)
  })

  it("载荷异常（null 整体）不抛且不破坏既有文案", () => {
    useI18nStore.getState().applySettingsLocale({ locale: "en-US" })
    useI18nStore.getState().applySettingsLocale(null)
    expect(useI18nStore.getState().preference).toBe("system")
    expect([zhCN, enUS]).toContain(useI18nStore.getState().t)
  })

  it("字典与 activeLocale 始终一致（杜绝文案/语言标识错配）", () => {
    for (const locale of ["zh-CN", "en-US", null] as const) {
      useI18nStore.getState().applySettingsLocale({ locale })
      const s = useI18nStore.getState()
      expect(s.t).toBe(s.activeLocale === "zh-CN" ? zhCN : enUS)
    }
  })
})

describe("initFromSettings 三类输入（走真实方法，IPC 已 mock）", () => {
  beforeEach(() => {
    useI18nStore.setState({ preference: "system", activeLocale: "zh-CN", t: zhCN })
    ipc.getShellSettings.mockReset()
  })

  it("① settings.locale 存在 → 采用该语言", async () => {
    ipc.getShellSettings.mockResolvedValue({ locale: "en-US" })
    await useI18nStore.getState().initFromSettings()
    expect(useI18nStore.getState().activeLocale).toBe("en-US")
    expect(useI18nStore.getState().t).toBe(enUS)
    expect(useI18nStore.getState().preference).toBe("en-US")
  })

  it("② settings.locale 缺失 → preference=system，按系统语言", async () => {
    ipc.getShellSettings.mockResolvedValue({ locale: null })
    await useI18nStore.getState().initFromSettings()
    const s = useI18nStore.getState()
    expect(s.preference).toBe("system")
    expect(s.activeLocale).toBe(resolveSystemLocale())
    expect(s.t).toBe(s.activeLocale === "zh-CN" ? zhCN : enUS)
  })

  it("③ 读取失败（IPC reject）→ 同「缺失」口径，且不抛", async () => {
    ipc.getShellSettings.mockRejectedValue(new Error("IPC 不可用"))
    await expect(useI18nStore.getState().initFromSettings()).resolves.toBeUndefined()
    const s = useI18nStore.getState()
    expect(s.preference).toBe("system")
    expect(s.activeLocale).toBe(resolveSystemLocale())
  })

  it("幂等：重复调用结果稳定（各窗口各自初始化一次不会互相干扰）", async () => {
    ipc.getShellSettings.mockResolvedValue({ locale: "zh-CN" })
    await useI18nStore.getState().initFromSettings()
    await useI18nStore.getState().initFromSettings()
    expect(useI18nStore.getState().activeLocale).toBe("zh-CN")
    expect(ipc.getShellSettings).toHaveBeenCalledTimes(2)
  })
})

describe("跨窗口初始化接线（结构门禁，?raw 源码断言）", () => {
  // 跨窗行为无法纯逻辑测（各窗独立 runtime，禁 RTL/jsdom）——本组把**接线**
  // 机器化：App 必须自行初始化 + 订阅广播，且不得再让单个页面独占初始化。
  it("App.tsx 自行初始化语言（每个窗口一次）", () => {
    expect(appCode).toContain("initFromSettings()")
  })

  it("App.tsx 订阅 settings-changed 并同步语言（红线 3：只经事件广播）", () => {
    // 2026-09-11（task-27）修正：原断言 `toContain('"app:settings-changed"')` 是在钉
    // 「魔法字符串就写在这里」——task-27 把它收敛到 `EV.settingsChanged` 后该断言失真。
    // 按真实意图重写：订阅必须存在，且**必须经 EV 常量**（不得再出现裸字面量）。
    expect(appCode).toContain("EV.settingsChanged")
    expect(appCode).toContain("applySettingsLocale(")
    expect(appCode, "魔法字符串回流（应经 EV 常量）").not.toContain('"app:settings-changed"')
    // 不得试图让 Zustand 跨窗共享（红线 3）
    expect(appCode).not.toContain("window.__DSH_I18N__")
  })

  it("首帧门闸：语言未就绪不渲染页面（消除「先中文后英文」闪烁）", () => {
    expect(appCode).toContain("localeReady")
    expect(appCode).toMatch(/if \(label === null \|\| !localeReady\)/)
  })

  it("初始化只有单一入口：ProfileManager 不再重复调用", () => {
    // 修复前：initFromSettings() 全仓唯一调用点就是 ProfileManager（缺陷根因）
    expect(profileManagerCode).not.toContain("initFromSettings()")
    // 而 App 必须持有它（否则语言就没人在初始化了）
    expect(appCode.split("initFromSettings()").length - 1).toBe(1)
    // 反向断言：剥离注释后仍保留真实结构，防 stripComments 过度剥离造成假绿
    expect(profileManagerCode).toContain("useI18n()")
    expect(appCode).toContain("localeReady")
  })

  it("门闸有上限：IPC 挂起时也放行（不得永久停在骨架屏）", () => {
    expect(appCode).toMatch(/LOCALE_INIT_TIMEOUT_MS/)
    expect(appCode).toMatch(/setTimeout/)
  })
})
