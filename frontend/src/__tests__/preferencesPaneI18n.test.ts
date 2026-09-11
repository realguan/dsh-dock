// preferencesPaneI18n.test.ts —— 偏好设置面板文案门禁（2026-09-11，task-20；task-22 修订）。
// 背景：`components/system/PreferencesPane.tsx` 有 11 处硬编码用户可见文案，
// 其中「简体中文 (默认)」是 en-US 用户可见中文（真 i18n 缺陷），
// "Auto Detect" / "English (US)" 是 zh 语境下的反向漏译。文案已全部入字典。
// task-22 追加：原副标题里的「默认」主张**本身是假的**——产品默认是「跟随系统」
// （i18nStore `preference: "system"`；Rust `settings.rs` locale 默认 None），
// 故按事实**删除**中文卡副标题，而不是为保测试绿而保留错误文案。
// 本门禁守住：
//   ① 新键两侧对称且非空，en 侧不含 CJK、zh 侧无英文整句；
//   ② 语言卡片不得复述自己的标题；仅系统卡有副标题；
//   ③ **事实门禁**：locale 组文案不得声称任何语言是「默认」；
//   ④ 组件确实消费新键、旧硬编码不得回流（?raw 源码断言，剥离注释）。
// 纯逻辑测试：只读字典常量 + 源码文本 + store 初始态，不渲染 DOM、不引 RTL/jsdom（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"
import { t as zhCN } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"
import { useI18nStore } from "@/stores/i18nStore"
import preferencesPaneSrc from "@/components/system/PreferencesPane.tsx?raw"

const CJK = /[\u3000-\u303f\u4e00-\u9fff\uff00-\uffef]/

/**
 * 断言「旧文案不回流」时必须**剥离注释**：源码注释里可以用旧文案做说明
 * （本组件就有这样的解释性注释），把注释当渲染文案判红会造成假阳性。
 */
function stripComments(src: string): string {
  return src
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "") // JSX 块注释 {/* … */}
    .replace(/\/\*[\s\S]*?\*\//g, "") // 普通块注释 /* … */
    .replace(/^[ \t]*\/\/.*$/gm, "") // 行注释 // …
}

const preferencesPaneCode = stripComments(preferencesPaneSrc)

/** 本轮为 PreferencesPane 文案收口新增的键（task-20；task-22 删去 `localeZhHint`）。 */
const NEW_KEYS = [
  "settingsLoading",
  "localeSystemHint",
  "breakerTitle",
  "breakerWindowLabel",
  "breakerWindowValue",
  "breakerThresholdLabel",
  "breakerThresholdValue",
  "breakerActionLabel",
  "breakerActionValue",
] as const

/** task-22 按事实**删除**的键：不得回归（回归即恢复「简体中文=默认」的谎报）。 */
const REMOVED_KEYS = ["localeZhHint", "localeEnHint"] as const

const zh = zhCN.console as unknown as Record<string, unknown>
const en = enUS.console as unknown as Record<string, unknown>

/**
 * 三张语言卡片的渲染行，**按组件 JSX 的组合顺序**（标题 → 副标题）：
 * 仅系统卡有副标题；中文卡与英文卡都只有标题（各自的副标题已按事实/重复删除）。
 */
function localeCards(dict: Record<string, unknown>): Array<{ id: string; lines: string[] }> {
  const systemLines = [String(dict.localeSystem), String(dict.localeSystemHint)]
  const zhLines = [String(dict.localeZh)]
  const enLines = [String(dict.localeEn)]
  return [
    { id: "system", lines: systemLines },
    { id: "zh-CN", lines: zhLines },
    { id: "en-US", lines: enLines },
  ]
}

describe("偏好设置面板文案收口（task-20）", () => {
  it("新增键两侧对称存在、类型一致且非空（杜绝半迁移）", () => {
    for (const key of NEW_KEYS) {
      expect(key in zh, `zh-CN 缺键 console.${key}`).toBe(true)
      expect(key in en, `en-US 缺键 console.${key}`).toBe(true)
      expect(typeof en[key], `console.${key} 两侧类型不一致`).toBe(typeof zh[key])
      expect(String(zh[key]).length, `zh-CN console.${key} 为空`).toBeGreaterThan(0)
      expect(String(en[key]).length, `en-US console.${key} 为空`).toBeGreaterThan(0)
    }
  })

  it("新增键的 en 侧不含 CJK（en 用户不得看到中文）", () => {
    const offenders = NEW_KEYS.filter((k) => CJK.test(String(en[k]))).map(
      (k) => `console.${k} = ${JSON.stringify(String(en[k]))}`,
    )
    expect(offenders, "en-US 侧出现中文 ⇒ 漏译").toEqual([])
  })

  it("新增键的 zh 侧不得残留英文整句（技术术语括注除外）", () => {
    // `breakerTitle` 刻意保留技术术语括注「（Circuit Breaker）」——术语对照，
    // 与 zh 侧 mcp* 字段名括注同类（2026-09-11 lead 已裁定不改）。
    const GLOSS_ALLOWED = new Set(["breakerTitle"])
    const offenders = NEW_KEYS.filter((k) => !GLOSS_ALLOWED.has(k)).filter((k) =>
      /\b[A-Za-z]{3,}\b/.test(String(zh[k])),
    )
    expect(offenders, "zh-CN 侧出现英文整词 ⇒ 反向漏译").toEqual([])
    expect(String(zh.breakerTitle)).toContain("Circuit Breaker")
    expect(String(en.breakerTitle)).not.toContain("Circuit Breaker Protocol（")
  })

  it("语言卡片不得复述自己的标题（原「简体中文 (默认)」重复渲染）", () => {
    for (const [label, dict] of [
      ["zh-CN", zh],
      ["en-US", en],
    ] as const) {
      for (const card of localeCards(dict)) {
        const unique = new Set(card.lines)
        expect(
          unique.size,
          `${label} 卡片 ${card.id} 出现重复行：${JSON.stringify(card.lines)}`,
        ).toBe(card.lines.length)
      }
    }
  })

  it("副标题不得等于任何卡片标题；只有系统卡有副标题键", () => {
    for (const [label, dict] of [
      ["zh-CN", zh],
      ["en-US", en],
    ] as const) {
      const titles = localeCards(dict).map((c) => c.lines[0])
      expect(titles, `${label} 的 localeSystemHint 复述了某个卡片标题`).not.toContain(
        String(dict.localeSystemHint),
      )
    }
    // 英文卡与中文卡一律无副标题键，且**不得再引入**（task-20 删英文卡、task-22 删中文卡）：
    // 若将来要恢复，必须先在两字典定义不重复标题、且事实为真的提示文案，过下面的事实门禁。
    for (const key of REMOVED_KEYS) {
      expect(key in zh, `zh-CN 不应存在 ${key}`).toBe(false)
      expect(key in en, `en-US 不应存在 ${key}`).toBe(false)
    }
    expect("localeSystemHint" in zh).toBe(true)
    expect("localeSystemHint" in en).toBe(true)
    // 组件侧不得残留被删键的引用（否则等于把副标题行挂在一个不存在的键上）
    for (const key of REMOVED_KEYS) {
      expect(preferencesPaneCode, `组件仍引用被删键 t.console.${key}`).not.toContain(key)
    }
  })

  it("事实门禁：卡片副标题不得声称某个语言是「默认」", () => {
    // 事实依据（实证，非推断）：
    //   · i18nStore.ts `preference: "system"`（下方直接读 store 初始态断言）；
    //   · initFromSettings() 在 settings.locale 缺失时保持 preference="system"；
    //   · Rust settings.rs `locale: Option<String>` 默认 None = 跟随操作系统语言。
    // 故把**某个具体语言**标为「默认」即谎报 ⇒ 红（原 `localeZhHint: "默认"` 正是此例）。
    expect(useI18nStore.getState().preference, "产品默认偏好应为跟随系统").toBe("system")

    // 只查「副标题/Hint」这类**主张性**文案：卡片标题本身不在此列——
    // `localeSystem = "System Default"` 是**系统项的选项名**（意为「用系统的默认」），
    // 与事实一致（产品默认确实是跟随系统），故不判红。
    const defaultClaim = /默认|[Dd]efault/
    const offenders: string[] = []
    for (const [label, dict] of [
      ["zh-CN", zh],
      ["en-US", en],
    ] as const) {
      for (const [key, value] of Object.entries(dict)) {
        if (!/^locale\w*Hint$/.test(key)) continue
        if (defaultClaim.test(String(value))) {
          offenders.push(`${label} console.${key} = ${JSON.stringify(String(value))}`)
        }
      }
    }
    expect(offenders, "卡片副标题出现「默认」主张（产品默认是跟随系统）").toEqual([])
  })

  it("中文卡只渲染标题一行（原「默认」副标题已按事实删除）", () => {
    const zhCard = localeCards(zh).find((c) => c.id === "zh-CN")!
    expect(zhCard.lines).toEqual(["简体中文"])
    expect(String(zh.localeZh)).toBe("简体中文")
  })

  it("系统卡副标题为真且有用：preference===\"system\" 确实走系统语言探测", () => {
    // 该条保留的依据：resolveSystemLocale() 读 navigator.language（i18nStore.ts），
    // 且选择「跟随系统」时 preference 落回 "system" ⇒ 「自动检测 / Auto Detect」属实。
    expect(String(zh.localeSystemHint)).toBe("自动检测")
    expect(String(en.localeSystemHint)).toBe("Auto Detect")
    expect(preferencesPaneSrc).toContain("t.console.localeSystemHint")
  })

  it("三张卡片标题互不相同（标题即语言自名/系统项，不得撞名）", () => {
    for (const [label, dict] of [
      ["zh-CN", zh],
      ["en-US", en],
    ] as const) {
      const titles = localeCards(dict).map((c) => c.lines[0])
      expect(new Set(titles).size, `${label} 卡片标题撞名：${titles.join(" / ")}`).toBe(3)
    }
  })

  it("组件确实消费新键（?raw 源码断言）", () => {
    for (const key of NEW_KEYS) {
      expect(preferencesPaneSrc, `组件未消费 t.console.${key}`).toContain(`t.console.${key}`)
    }
  })
  it("旧硬编码文案不得回流（仅本文件的 11 处特征串）", () => {
    const literals = [
      "正在加载偏好设置…",
      "Auto Detect",
      "简体中文 (默认)",
      "English (US)</p>",
      "智能熔断保护协议",
      "监控窗口：",
      "60 秒滑动窗口",
      "熔断阈值：",
      "连续 3 次崩溃",
      "熔断后动作：",
      "停机并弹诊断卡",
    ]
    for (const literal of literals) {
      expect(preferencesPaneCode, `回流了旧硬编码文案 ${literal}`).not.toContain(literal)
    }
    // 剥离注释后仍应保留真实结构，防止 stripComments 过度剥离导致假绿
    expect(preferencesPaneCode).toContain("t.console.localeEn")
    expect(preferencesPaneCode).toContain("t.console.breakerActionValue")
  })

  it("键盘快捷键符号不入字典（无翻译价值的技术符号保留在组件内）", () => {
    // ⌘ / ⇧ / Ctrl 是跨语言固定符号；字典里已有 `shortcutDefault` 等标签键，
    // 按键序列本身不迁移（task-20 分类：不应入字典）。
    expect(preferencesPaneSrc).toContain('"⌘ + ,"')
    expect(preferencesPaneSrc).toContain('"Ctrl + ,"')
    expect(preferencesPaneSrc).toContain('"⌘ + ⇧ + P"')
    expect(String(zh.shortcutDefault)).toContain("⌘ + ,")
    expect(String(en.shortcutDefault)).toContain("⌘ + ,")
  })
})

/**
 * F2：「下次启动的运行环境」入口（2026-09-11，v1.2.0 实测 1.3）。
 *
 * 缺口措辞（lead 订正）：不是「用户完全不可达」——Windows 托盘菜单「打开方式」
 * （boot.rs:777 的 switch_mode）也会写 default_mode；真实缺口是
 * 「**应用窗口内无入口**」。本组断言该窗口内入口的接线与契约约束。
 */
describe("F2：下次启动的运行环境入口", () => {
  const BOOT_MODE_KEYS = [
    "bootModeSection",
    "bootModeDesc",
    "bootModeAsk",
    "bootModeAskHint",
    "bootModeLocal",
    "bootModeLocalHint",
    "bootModeWsl",
    "bootModeWslHint",
    "bootModeFallbackHint",
  ] as const

  it("9 个新键两侧对称、非空，且 en 侧不含 CJK（enUsNoLeak 口径）", () => {
    for (const key of BOOT_MODE_KEYS) {
      expect(key in zh, `zh-CN 缺 console.${key}`).toBe(true)
      expect(key in en, `en-US 缺 console.${key}`).toBe(true)
      expect(String(zh[key]).length, `zh-CN console.${key} 为空`).toBeGreaterThan(0)
      expect(String(en[key]).length, `en-US console.${key} 为空`).toBeGreaterThan(0)
      expect(CJK.test(String(en[key])), `en-US console.${key} 含中文`).toBe(false)
    }
  })

  it("组件消费全部新键（入口真的渲染出来了）", () => {
    for (const key of BOOT_MODE_KEYS) {
      expect(preferencesPaneCode, `组件未消费 t.console.${key}`).toContain(
        `t.console.${key}`,
      )
    }
  })

  it("平台语义走 usePlatform().can.*（不散写 os === 'windows'）", () => {
    expect(preferencesPaneCode).toContain("can.chooseMode")
    expect(preferencesPaneCode).not.toMatch(/os === ["']windows["']/)
  })

  it("经 patchShellSettings 写 defaultMode —— 不新增 IPC（§7 登记制）", () => {
    expect(preferencesPaneCode).toContain("patchShellSettings({ defaultMode:")
    // 新增 IPC 的红线：本模块不得出现新的 invoke/api.* 调用面
    expect(preferencesPaneCode, "疑似新增 IPC 调用").not.toMatch(/api\.\w+\(/)
  })

  it("三态取值与 Rust 契约一致：null = 每次询问，local/wsl = 直接启动", () => {
    // 断言组件确实提供了三个选项，且 null 那一支存在（None 语义）
    expect(preferencesPaneCode).toContain("bootModeAsk")
    expect(preferencesPaneCode).toContain("bootModeLocal")
    expect(preferencesPaneCode).toContain("bootModeWsl")
    expect(preferencesPaneCode).toMatch(/value:\s*null/)
  })

  it("含「可切到 WSL」的可行动出路文案（与 T-D1 引导失败呼应）", () => {
    expect(String(zh.bootModeFallbackHint)).toContain("WSL")
    expect(String(en.bootModeFallbackHint)).toContain("WSL")
  })
})
