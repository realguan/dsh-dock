// firstPartyPlugins.ts —— **自研插件目录**（DSH Dock 自研插件的静态策展架，2026-10-13）。
//
// 定位：profile 详情页「自研插件」tab 的货架——只回答四个问题：有哪些自研插件、
// 装没装、装的什么版本、目录钉的什么版本。不回答"插件怎么工作"（dsh 侧的事，
// 壳只提供安装/卸载与宿主原语）。
//
// 与另外两份"插件清单"的边界（三份清单三种可信度，勿混）：
//   · 本架 = 壳内置**静态**清单，只收 DSH Dock 自研插件——"一直显示在那里"的
//     可信货架；增长方式 = 改本文件 + 发壳新版（每条自研插件都是刻意发布事件，
//     不值得为它引一条远端目录链）。
//   · 官方目录（`official_catalog.rs`，ADR-0020）= 后端策展，管 @deepseek-ai
//     官方包的变体 / 前置 / 修复态；
//   · 社区市场（`dsh-plugin-catalog`，updates.rs 拉取）= 远端动态、人人可上架。
//
// 安装/卸载/激活零新机制：`install_plugin` / `remove_plugin` 现成 IPC；自研插件
// 一律声明 `dsh.bundle`（ADR-0020 `Activation::AutoBundle`）⇒ `dsh plugin add`
// 自动进 bundle 层栈，壳不写 patch。

/** 目录条目：**稳定数据**。用户可见文案（名 / 描述 / 备注）不在本文件——
 *  显示名走 `pluginDisplay.ts` 的 CHINESE 策展表（与插件列表行同源），
 *  长文案走 `content/*.ts` 的 `profiles.firstParty.items.<id>`。 */
export interface FirstPartyPlugin {
  /** 稳定 id：文案键与测试锚点。 */
  id: string
  /** npm 真实包名 = 安装后 `list_profile_plugins` 里的 `name`
   *  （github 形态安装也按 package.json 的 name 落 dependencies）。 */
  package: string
  /** 安装 spec：**必须钉版本**（github `#tag` / npm `@ver`）——不钉 = 用户装到
   *  随时漂移的分支 HEAD，重蹈 ADR-0020 `latest` dist-tag 错配的那类事故。
   *  钉法由 {@link pinVersion} 解析，目录级闸门测试守着（见 `__tests__/`）。 */
  spec: string
  /** 仓库地址（外链，经 `api.openExternal` 打开）。 */
  repo: string
}

/** 当前自研插件目录。新增条目 = 加一行 + 补 `pluginDisplay.ts` 中文名 +
 *  content 双语文案 + 目录闸门测试自动覆盖。 */
export const FIRST_PARTY_PLUGINS: readonly FirstPartyPlugin[] = [
  {
    id: "deepseek-chat-embed",
    package: "dsh-deepseek-chat-embed",
    // ⚠️ 钉的 tag 必须已在 GitHub 上存在（安装期 `git` 解析，缺 tag 即装失败）。
    // 发版流程：插件仓库打 `v<package.json version>` 并推送后，本行才生效。
    spec: "github:realguan/dsh-deepseek-chat-embed#v0.1.0",
    repo: "https://github.com/realguan/dsh-deepseek-chat-embed",
  },
]

/** 从安装 spec 提取**钉住的版本**；未钉返回 `null`（目录闸门要求全目录非 null）。
 *
 *  两种合法形态：
 *   · github：`github:<owner>/<repo>#<tag>` → tag 去掉前导 `v`（tag 惯例 `v0.1.0`，
 *     package.json version 是 `0.1.0`，比对前必须归一，否则恒判"可更新"）；
 *   · npm：`<pkg>@<ver>` / `@scope/<pkg>@<ver>` → `@` 后末段。
 *     （本目录当前只用 github 形态；npm 分支为后续发布到 registry 预留。）
 */
export function pinVersion(spec: string): string | null {
  const hash = spec.indexOf("#")
  if (spec.startsWith("github:") && hash !== -1) {
    const tag = spec.slice(hash + 1).trim()
    if (!tag) return null
    return tag.replace(/^v/, "")
  }
  const at = spec.lastIndexOf("@")
  if (at > 0) {
    const ver = spec.slice(at + 1).trim()
    return ver || null
  }
  return null
}

/** github spec 的**包名段**（`github:<owner>/<repo>` 的 `<repo>`）——与条目
 *  `package` 字段的一致性判据（装出来的 node_modules 目录名 = repo 名，若与
 *  package.json name 不一致，`list_profile_plugins` 按 name 匹配会漏判"已装"）。
 *  非 github spec 返回 `null`（一致性闸门只覆盖 github 形态）。 */
export function specPackageName(spec: string): string | null {
  if (!spec.startsWith("github:")) return null
  const body = spec.slice("github:".length)
  const hash = body.indexOf("#")
  const path = hash === -1 ? body : body.slice(0, hash)
  const segments = path.split("/").filter(Boolean)
  return segments.length >= 2 ? segments[segments.length - 1] : null
}

/** 货架上一行：目录条目 + 从 profile 插件清单推导出的状态。 */
export interface FirstPartyRow {
  plugin: FirstPartyPlugin
  /** 已装判据 = dependencies 里有**同名 dependency 条目**。
   *  bundle 条目不算（那表示"随 dsh 安装提供"，不是本 profile 装的；
   *  与 `pluginCatalog.ts` 的 installed 判据同源：有版本可读的 dependency 才算）。 */
  installed: boolean
  /** 已装版本（dependency 条目实读的 `installed_version`）；未装 = null。 */
  installedVersion: string | null
  /** 目录钉的版本（恒非空——闸门测试保证）。 */
  pinnedVersion: string
  /** 已装 ∧ 版本 ≠ 钉版 ⇒ 可对齐。动作仍是安装（`pnpm add` 同 spec 覆盖即更新），
   *  不引第二条更新链（github 形态吃不到 npm 更新检查，ADR-0011）。 */
  updateAvailable: boolean
}

/** 目录 × 已装清单 → 货架行。纯函数：`plugins` 传 `list_profile_plugins` 的返回值。 */
export function firstPartyRows(
  catalog: readonly FirstPartyPlugin[],
  plugins: readonly { name: string; kind: string; installed_version: string | null }[],
): FirstPartyRow[] {
  return catalog.map((plugin) => {
    const dep = plugins.find((p) => p.kind === "dependency" && p.name === plugin.package)
    const installed = dep !== undefined
    const installedVersion = dep?.installed_version ?? null
    const pinnedVersion = pinVersion(plugin.spec) ?? ""
    return {
      plugin,
      installed,
      installedVersion,
      pinnedVersion,
      updateAvailable:
        installed && installedVersion !== null && installedVersion !== pinnedVersion,
    }
  })
}
