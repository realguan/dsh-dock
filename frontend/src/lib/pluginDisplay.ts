// pluginDisplay.ts —— 插件行的「人类友好显示名」纯函数。
//
// ## 名字的三层（2026-09-21 维护者裁定，本文件是唯一实现处）
//   ① **主标题 = 插件原本的名字**（"插件名应该是取插件原本的名字，中文为辅"）：
//      社区/内置行 = `pluginShortId`（包名去 scope，即 npm 上它的原名）；
//      实验性行 = `pluginMemberLabel`（再剥掉 `dsh`/`experimental` 噪音段 —— 那一族
//      包共有该前缀，留着的话三个后端的前 40 字符完全相同，眼睛扫不出区别）；
//   ② **辅助灰字 = 中文名**：`pluginChineseName`（**查表命中才有**，未命中返回 `null`）；
//   ③ 完整包名一律留在 `title` 属性上，排查/读屏可取。
//
// ## 为什么不给未命中项编中文/音译名
// 旧版有 `pluginFriendlyName`："查表未命中就 Title Case 兜底"（`dsh-pet` → `Pet`，
// `dsh-experimental-browser-use-playwright-mcp` → `Browser Use Playwright Mcp`）。
// 那是**机器音译**：既不是插件自己的名字（①已经有了），也不是中文（②的职责），
// 却占着第一阅读层。裁定改为"取原本的名字"后它失去了位置，故整体退役。
//
// 已知映射只放**官方与高频包**——社区包数量随 registry 漂移，逐一登记必然过期
// （与 ADR-0020「官方性由策展目录独家拥有」同族思路）。

/** 官方/高频包的中文名（按语言分表）。 */
const CHINESE: Record<string, { zh: string; en: string }> = {
  "@deepseek-ai/dsh-base": { zh: "Cordis 底座", en: "Cordis Base" },
  "@deepseek-ai/dsh-web-app": { zh: "Web 界面渲染器", en: "Web UI Renderer" },
  "@deepseek-ai/dsh-desktop-runtime": {
    zh: "官方桌面运行时",
    en: "Official Desktop Runtime",
  },
  "@deepseek-ai/dsh-browser-use": { zh: "浏览器操作", en: "Browser Use" },
  "@deepseek-ai/dsh-computer-use": { zh: "桌面控制", en: "Computer Use" },
  "@deepseek-ai/dsh-experimental-auto-review": {
    zh: "自动安全审查",
    en: "Auto Review",
  },
  "@openviking/dsh-memory-plugin": {
    zh: "记忆与上下文",
    en: "Memory & Context",
  },
}

/**
 * 插件的**中文辅助名**（行内主标题旁的灰字）。
 *
 * @param pkg 完整 npm 包名（或任意来源标识）
 * @param locale 当前界面语言（`zh-CN` / `en-US`）
 * @returns 查表命中返回本地化名；**未命中返回 `null`**（调用方据此整块不渲染——
 *          宁可不给辅名，也不拿机器音译冒充）。
 */
export function pluginChineseName(pkg: string, locale: string): string | null {
  if (!pkg) return null
  const hit = CHINESE[pkg]
  if (!hit) return null
  return locale === "zh-CN" ? hit.zh : hit.en
}

/**
 * 主标题旁的**短标识**（`title` 属性与次级灰字用）：保留可读的包名尾部，
 * 去掉 scope 前缀——`@deepseek-ai/dsh-experimental-browser-use-playwright-mcp`
 * → `dsh-experimental-browser-use-playwright-mcp`（仍长，但只作 title 兜底，
 * 不占主标题位）。
 */
export function pluginShortId(pkg: string): string {
  if (!pkg) return ""
  return pkg.includes("/") ? pkg.slice(pkg.indexOf("/") + 1) : pkg
}

/**
 * **去噪短标签**：去 scope + 去掉 `dsh` / `experimental` 这类噪音段。
 * `@deepseek-ai/dsh-experimental-browser-use-playwright-mcp` → `browser-use-playwright-mcp`
 *
 * 用于"依赖明细 / 生效后端"这类**不需要完整包名、但必须保留区分度**的位置：
 * 去掉的只是各包共有的前缀噪音，**尾部（区分各档的那部分）完整保留**——
 * 与仓库 2026-09-17 裁定"包名不许截断"同一意图（截断会砍掉区分点，去前缀不会）。
 * 完整包名一律留在 `title` 上，排查时可取。
 *
 * 插件列表的明细行与实验能力面板的生效后端共用本函数（同一语义，一处实现）。
 */
export function pluginMemberLabel(pkg: string): string {
  if (!pkg) return ""
  const short = pluginShortId(pkg)
  const segments = short.split("-").filter((s) => s !== "" && s !== "dsh" && s !== "experimental")
  return segments.length > 0 ? segments.join("-") : short
}

/**
 * 一组成员包名的**公共前缀**（按 `-` 分段对齐 —— 不切在词中间）。
 *
 * 用于变体对照：三条 `@deepseek-ai/dsh-experimental-browser-use-{playwright-mcp,
 * chrome-devtools-mcp,stagehand-native}` 的公共前缀是
 * `@deepseek-ai/dsh-experimental-browser-use-`，把它提到分节标题讲一次，
 * 卡内只留 `playwright-mcp` 这类区分段 —— 每张卡从 60 字符降到 15 字符。
 */
export function commonPackagePrefix(pkgs: readonly string[]): string {
  if (pkgs.length < 2) return ""
  const parts = pkgs.map((p) => p.split("-"))
  const first = parts[0]
  let i = 0
  while (i < first.length - 1 && parts.every((segs) => segs[i] === first[i])) i += 1
  return i === 0 ? "" : `${first.slice(0, i).join("-")}-`
}

/** 去掉公共前缀后的尾部；前缀不匹配时原样返回（不许把包名改得对不上 npm）。 */
export function packageTail(pkg: string, prefix: string): string {
  if (prefix && pkg.startsWith(prefix) && pkg.length > prefix.length) {
    return pkg.slice(prefix.length)
  }
  return pkg
}
