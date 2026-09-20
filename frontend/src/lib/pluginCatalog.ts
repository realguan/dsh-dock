// pluginCatalog.ts —— 「插件列表」三类合一的**纯分类与合并**（2026-09-20，ADR-0028 第二批：
// 清单打标合并——底座 / 第三方 / 实验性一行，「底座组合」tab 退役）。
//
// ## 为什么需要一个合并函数（而不是在组件里 filter 两遍）
//
// 三类插件今天住在两个数据源里：第三方与实验性装在 profile `dependencies`
// （`list_profile_plugins` 的 `kind: "dependency"`），dsh 内置层只在
// `dsh.profile.bundles` 栈里（`kind: "bundle"`，不占依赖）。合出来的是
// **依赖 ∪ 层栈**的并集——「底座组合」tab 退役的前提正是它的内容被这一类吸收。
// 归属证据也换了：某个包"是从实验能力面板装的、归哪个能力管"只有能力目录
// （`list_experimental_capabilities`）知道，而这目录是按 profile 的——ADR-0028
// 把面板迁进 Profile 详情页之后，两者才同页同档，这个标记做得可靠。
//
// ## dsh 的插件模型（源码锚定，2026-09-20 修正第一版的的错误判据）
//
// 上游事实（`deepseek-harness`：`packages/boot/app-boot/src/profile.ts`、
// `profile-plugins.ts`、`packages/boot/plugin-manager/src/index.ts::listBundles`）：
//
// - `dsh.profile.bundles` = **激活的层列表**（组合序：各 bundle 的 patch 依次套，
//   后层覆盖先层，最后才是 profile 自己的 cordis.patch.yml）。init 时写入模板层
//   （web = base + web-app…），之后由 `dsh plugin add/remove` 的 reconcile 维护：
//   用户安装的、**自己声明了 `dsh.bundle.patch`** 的依赖会被追加进列表。
//   **reconcile 保留栈里已有的重复**（源码注释原样："template entries retain their
//   order and duplicates"）——重复是上游事实，壳侧展示必须按包名折叠。
// - dsh 自己的插件页对每个包给三个事实：`installed`（在 dependencies 里）、
//   `enabled`（在 bundles 里 = 作为层激活）、`removable = installed &&
//   !installation.dependencies`。**「内置/不可卸载」的权威判据是"由 dsh 安装目录
//   提供"，不是"在 bundles 里"**——用户经 npm 装进 profile 的层（如 memory-plugin）
//   一样进 bundles，但它可更新、可卸载。
// - 安装提供的层分两种：profile 模板层 + `OPTIONAL_BUNDLES`（Agent Teams 两层，
//   随安装下发、默认关、在 dsh 插件页开关、永不可卸）。桌面包（desktop-packages/）
//   由客户端底座部署，同样不可卸。
//
// 壳侧能拿到的事实（`list_profile_plugins` + `getProfileDetail` + 能力目录）足以
// 复刻这套判据，**无需契约改动**：
//
// - `installed` ⟺ 存在 `kind: "dependency"` 条目（有版本/简介实读）；
// - `dshProvided`（安装提供、不可卸载）⟺ 存在 `kind: "bundle"`条目 **且**没有
//   dependency 条目。依据是模块解析序（bundle 名先从安装目录解析，再到 profile
//   node_modules）：在层栈里却不是本 profile 依赖的包，只能由安装提供——这正是
//   dsh `removable` 规则的反面。桌面运行时内嵌包（Rust 已按 spec 判为 bundle 类、
//   不进依赖）同此覆盖。
//   已知边界：若有人绕过 reconcile 手工编辑 package.json，栈里可能留下"既非安装
//   提供、也未装"的失效条目——它会被本判据显示为「内置」（保守方向：不给卸载入口，
//   比让用户卸一个组合树正在用的层安全）。精确判据要后端补 provided 旗标，另案。
// - 层序 = **去重后**栈的名次（重复条目折叠到首个位置；dsh 的 listBundles 同样
//   `[...new Set(...)]` 去重后呈现）。
//
// ## 三条分类判据（按优先级，互斥的主标签）
//
// 1. **实验性**：该包出现在能力目录任一变体的步骤里（`capabilityOfPackage`）。
//    标到**具体能力**（「实验性 · 多智能体协同」），不只"实验性"——用户能立刻知道
//    这个包归哪个开关管。它的开关与移除只在「实验能力」面板（单一入口，ADR-0020 §2.8），
//    所以这类行在插件列表里**不给开关/卸载**，只给「去开关」跳转。
// 2. **内置**：`dshProvided`（随 dsh 安装提供：模板层 / OPTIONAL_BUNDLES / 桌面包）。
//    **不可卸载**；开关在 dsh 自己的插件页。
// 3. **第三方**：其余（用户经 npm 装进本 profile 的包，含**装进 profile 的层**——
//    它们带「层 N」标但控制面与普通第三方一致）。
//
// 一个包可以同时是"层栈成员"与"实验能力"：用户经面板装的（auto-review）= 纯实验性；
// dsh 自带的（Agent Teams 两层激活时）= 实验性 **+ 追加「内置」标**
// （2026-09-20 维护者裁定：它既是 dsh 已内置的实验功能；内置一律不允许卸载）。
// 层序是**正交**事实，单独一枚「层 N」标表达。

import type { Capability, PluginEntry, PluginRowState } from "@/types/ipc"

/** 行上的**主标签**（三类互斥；「内置」对 dsh 自带的实验层是追加标，见 `alsoBuiltin`）。 */
export type PluginKindTag = "builtin" | "thirdParty" | "experimental"

/** 列表级筛选（与主标签同名；「内置」筛选含 dsh 自带的实验层——它们也带「内置」标）。 */
export type PluginKindFilter = "all" | PluginKindTag

/** 包在能力里的**角色**（同一能力的包不是并列关系，2026-09-20 维护者真机指出）：
 *  · `primary` = 某变体**独有**的标识包（换后端时被替换的那个，如 playwright-mcp）；
 *  · `shared`  = 多变体共有的**基座**（如 `@deepseek-ai/dsh-browser-use`——官方描述
 *    "Exclusive named registration"，README 原话 "adds no model-visible tools"：
 *    它只登记 provider 插槽，自身不提供工具）。
 *  判据 = 该包出现在几个变体的步骤里（1 → primary，≥2 → shared），与
 *  `lib/experimentalCapabilities.ts::variantPackageRoles` 的 primary/shared 同源同义
 *  （同一份 steps 数据的不同问法，不构成第二事实源）。
 *
 *  **从属关系**（基座 ⊃ provider）：基座是能力本体的登记插槽（"同一时刻只允许一个
 *  provider 注册"由它执行），provider 是挂进插槽的后端。故组内**基座在上**（anchor），
 *  provider 缩进其下——交互上整组 hover 联动，「去开关」只长在 anchor 上。 */
export type CapabilityRole = "primary" | "shared"

/** 合并后的一行：**一个包一行**（层栈 / 依赖 / 重复条目全部折叠）+ 正交标记。
 *  同一能力的包在 `rows` 里**相邻**（组），由 `capabilityAnchor` / `capabilityChild`
 *  标记组内位置——渲染层据此缩进与联动，不需要自己再分组。 */
export interface MergedPluginRow {
  /** 该包的展示条目：优先 dependency 条目（有实读版本/简介），否则 bundle 条目。 */
  entry: PluginEntry
  /** 主标签。 */
  kindTag: PluginKindTag
  /** 主标签之外**追加**的「内置」标：实验能力 ∧ `dsh-provided`（dsh 自带的实验层）。 */
  alsoBuiltin: boolean
  /** 该包是否装在本 profile 的 dependencies 里（有版本可读、可更新/可卸载）。 */
  installed: boolean
  /** 该包是否由 dsh 安装提供（模板层 / OPTIONAL_BUNDLES / 桌面包）——不可卸载。 */
  dshProvided: boolean
  /** 归属的实验能力（`kindTag === "experimental"` 时非空；用于「实验性 · <能力名>」）。 */
  capability: Capability | null
  /** 该包在能力里的角色；`capability` 为空时为 `null`。 */
  capabilityRole: CapabilityRole | null
  /** 是否为该能力组的**首行**（anchor）：完整能力标 + 「去开关」都在这一行。
   *  有基座时基座是 anchor（它是能力本体）；没有基座（只装了 provider / 装一半）时
   *  第一个标识包是 anchor。 */
  capabilityAnchor: boolean
  /** 是否为组内**从属行**（provider 挂在基座下）：渲染缩进 + 「后端」标，不给跳转。 */
  capabilityChild: boolean
  /** 壳 patch 行态（开关/停用徽标的数据源）；`null` = 没有行。 */
  row: PluginRowState | null
}

export interface MergedPluginList {
  rows: MergedPluginRow[]
  /** 三个筛选各自的命中数（「内置」含 dsh 自带的实验层，故三者之和可大于总数）。 */
  counts: { builtin: number; thirdParty: number; experimental: number }
}

/**
 * 该包属于哪项**实验能力**（目录内第一个命中的能力）。
 *
 * 为什么按"任一变体的任一步"判：一个能力的包分散在各变体里（Agent Teams 的自建档
 * 就是基座本身），按包名反查必须覆盖全部变体，否则自建档会被漏标成第三方。
 * 首个命中即返回：策展集各能力的包集互不相交（Rust 侧 `catalog_is_well_formed`
 * 同口径维护），不存在"一个包归两项能力"的事实。
 *
 * **调用方须传 {@link dockCuratedCaps} 过滤后的目录**：安装自带的能力不归 dock
 * 策展，它们的包在插件列表里按内置层呈现，不挂「实验性」标。
 */
export function capabilityOfPackage(
  caps: readonly Capability[],
  pkg: string,
): Capability | null {
  for (const cap of caps) {
    if (cap.variants.some((v) => v.steps.some((s) => s.package === pkg))) return cap
  }
  return null
}

/**
 * **dock 策展的能力** = 目录排除 dsh 安装自带的那些（2026-09-20 维护者裁定）。
 *
 * `shippedByDsh` 判据（Rust `installation_shipped`：该能力**每个**包都能在安装的
 * runtime node_modules 里找到）正是 `OPTIONAL_BUNDLES` 那一类——随安装下发、默认关、
 * 在 **dsh 自己的插件页**开关、永不可卸。它已被官方插件管理托管，dock 再摆一份面板
 * 就是双入口（ADR-0020 §2.8），故：**能力面板不展示、插件列表不打「实验性」标**；
 * 用户在 dsh 插件页启用后，它的层随 `dsh.profile.bundles` 进列表，按 dsh 提供的内置层
 * 正常展示（「内置」+「层 N」+ 版本随 dsh，无控制面）。
 *
 * 单一规则只此一处：父级过滤一次，能力面板（展示）与插件列表（打标）共用。
 */
export function dockCuratedCaps(caps: readonly Capability[]): Capability[] {
  return caps.filter((cap) => !cap.shippedByDsh)
}

/** 合并三类（纯函数：入参即三个 IPC 的返回值，不碰文件系统）。
 *
 *  排序：层栈成员在前、按**去重后**的栈序（维护者 2026-09-20 裁定"层序保留"），其余
 *  保持 `list_profile_plugins` 的既有次序在后。**同一能力的包拉成相邻组**（基座在上），
 *  组的位置 = 其首个成员在原次序里的位置——层栈与依赖块边界的同能力包不被拆散。 */
export function mergePluginRows(input: {
  plugins: readonly PluginEntry[]
  /** `dsh.profile.bundles` 原始栈序（组合序；可能含重复，见上方模型说明）。 */
  bundles: readonly string[]
  rows: readonly PluginRowState[]
  caps: readonly Capability[]
}): MergedPluginList {
  // 栈去重保序：重复条目折叠到首个位置（dsh listBundles 同款去重后呈现）。
  const stack: string[] = []
  for (const name of input.bundles) if (!stack.includes(name)) stack.push(name)

  // 每个包只留一个展示位：dependency 条目优先（版本/简介是 node_modules 实读），
  // bundle 条目兜底（安装提供的层没有依赖条目）。按包名折叠，故重复入栈只出现一次。
  const merged = new Map<string, { dep?: PluginEntry; bundle?: PluginEntry }>()
  for (const entry of input.plugins) {
    const slot = merged.get(entry.name) ?? {}
    if (entry.kind === "dependency") slot.dep = entry
    else slot.bundle = entry
    merged.set(entry.name, slot)
  }
  const rowOf = (pkg: string): PluginRowState | null =>
    input.rows.find((r) => r.pkg_name === pkg) ?? null

  const build = (name: string): MergedPluginRow | null => {
    const slot = merged.get(name)
    if (!slot) return null
    const entry = slot.dep ?? slot.bundle
    if (!entry) return null
    const installed = slot.dep !== undefined
    // 安装提供 = 在层栈（或桌面包）里 ∧ 不是本 profile 依赖——镜像 dsh 的
    // `removable = installed && !installation.dependencies`（见上方模型说明）。
    const dshProvided = slot.bundle !== undefined && !installed
    const capability = capabilityOfPackage(input.caps, name)
    const kindTag: PluginKindTag = capability
      ? "experimental"
      : dshProvided
        ? "builtin"
        : "thirdParty"
    // 角色：出现在几个变体的步骤里（1 = 该变体独有的标识包；≥2 = 多变体共有的基座）。
    const capabilityRole: CapabilityRole | null = capability
      ? capability.variants.filter((v) => v.steps.some((s) => s.package === name)).length > 1
        ? "shared"
        : "primary"
      : null
    return {
      entry,
      kindTag,
      // dsh 自带的实验层（Agent Teams 两层激活态）：主标实验性，补一枚「内置」。
      alsoBuiltin: capability !== null && dshProvided,
      installed,
      dshProvided,
      capability,
      capabilityRole,
      // 组内位置占位，下面按"基座在场与否"回填
      capabilityAnchor: false,
      capabilityChild: false,
      row: rowOf(name),
    }
  }

  // 第一遍：层栈成员在前（去重后栈序），其余保持既有次序。
  const ordered: MergedPluginRow[] = []
  for (const name of stack) {
    const r = build(name)
    if (r) ordered.push(r)
  }
  for (const entry of input.plugins) {
    if (stack.includes(entry.name)) continue
    const r = build(entry.name)
    if (r) ordered.push(r)
  }

  // 第二遍：同能力包拉成相邻组——遇某能力首个成员即整组落位（基座在上，其余按原次序），
  // 后续成员跳过。非能力行原位不动。
  const rows: MergedPluginRow[] = []
  const emitted = new Set<string>()
  for (const r of ordered) {
    if (!r.capability) {
      rows.push(r)
      continue
    }
    const capId = r.capability.id
    if (emitted.has(capId)) continue
    emitted.add(capId)
    const group = ordered.filter((m) => m.capability?.id === capId)
    // 基座在上（能力本体）；没有基座（只装 provider / 装一半）则第一个标识包居首。
    const anchor = group.find((m) => m.capabilityRole === "shared") ?? group[0]
    const rest = group.filter((m) => m !== anchor)
    anchor.capabilityAnchor = true
    for (const m of rest) m.capabilityChild = true
    rows.push(anchor, ...rest)
  }

  return {
    rows,
    counts: {
      builtin: rows.filter((r) => matchesKindFilter(r, "builtin")).length,
      thirdParty: rows.filter((r) => matchesKindFilter(r, "thirdParty")).length,
      experimental: rows.filter((r) => matchesKindFilter(r, "experimental")).length,
    },
  }
}

/** 行是否命中某个筛选（「内置」= 带「内置」标的行：主标内置 + dsh 自带的实验层）。 */
export function matchesKindFilter(row: MergedPluginRow, filter: PluginKindFilter): boolean {
  switch (filter) {
    case "all":
      return true
    case "builtin":
      return row.kindTag === "builtin" || row.alsoBuiltin
    case "thirdParty":
      return row.kindTag === "thirdParty"
    case "experimental":
      return row.kindTag === "experimental"
  }
}

/** 搜索命中（包名或简介；与旧「插件列表」同口径，扩到三类行）。 */
export function matchesSearch(row: MergedPluginRow, query: string): boolean {
  const q = query.toLowerCase().trim()
  if (!q) return true
  const name = row.entry.name.toLowerCase()
  const desc = (row.entry.description ?? "").toLowerCase()
  return name.includes(q) || desc.includes(q)
}
