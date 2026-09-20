// pluginListMergeGate.test.ts —— 「插件列表」三类合一的源码闸门（2026-09-20，ADR-0028
// 第二批：底座 / 第三方 / 实验性三类一行，「底座组合」tab 退役）。
//
// 钉住的都是会**悄悄退化**的结构事实：
//  ① **双入口回归**：实验性行若又拿回自己的开关/卸载，同一能力就有了两个控制面
//     （ADR-0020 §2.8 / ADR-0028：卸载还必须连带清挂载行，否则悬空行让 dsh 起不来）；
//  ② **层序丢失 / 重复放大**：层栈成员若不去重（dsh reconcile 保留栈内重复，源码注释
//     原样可查），同一个包会一行变 N 行、层号还全取首个下标（2026-09-20 真机抓到）；
//  ③ **「内置」判据错**：把"在 dsh.profile.bundles 里"当内置，会把用户经 npm 装进
//     profile 的层（memory-plugin 这类可卸载的）误标成内置——dsh 的权威判据是
//     `removable = installed && !installation.dependencies`（plugin-manager listBundles）；
//  ④ **合并逻辑散进组件**：打标/合并若非走纯函数（lib/pluginCatalog.ts），单测就
//     鞭长莫及，下次改动只能靠真机验收；
//  ⑤ **双源回归**：目录由父级取（与能力面板共用一次回读），组件不得再自取。
// 纯逻辑测试：只读源码文本，不渲染 DOM（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"

import paneSrc from "@/components/profiles/ProfileDetailPane.tsx?raw"
import catalogSrc from "@/lib/pluginCatalog.ts?raw"
import zhDictSrc from "@/content/zh-CN.ts?raw"
import enDictSrc from "@/content/en-US.ts?raw"

/** 「插件列表」行组件体（`function PluginListRow(` 到文件尾）。 */
function rowRegion(): string {
  const start = paneSrc.indexOf("function PluginListRow(")
  expect(start, "找不到 PluginListRow：本闸门的窗口切片依赖它").toBeGreaterThan(-1)
  return paneSrc.slice(start)
}

describe("① 单一入口：实验性行不得有自己的开关 / 更新 / 卸载", () => {
  const row = rowRegion()

  it("开关 / 更新 / 卸载三个入口都受 manageable（= 第三方）守护", () => {
    // 拆掉任一个 `manageable &&`，实验性行就会多出一个控制面——开关与能力面板各说
    // 各话，卸载更会留下悬空挂载行（dsh 启动即失败）。四个可控点：开关、升级提示、
    // 更新、卸载（升级芯片也算——实验包的版本归能力面板管，不给直更新入口）。
    expect(row, "开关必须受 manageable 守护").toMatch(/manageable && hasRow &&/)
    expect(row, "更新按钮必须受 manageable 守护").toMatch(/manageable && \(\s*<Button/)
    const guarded = (row.match(/manageable &&/g) ?? []).length
    expect(guarded, "四个可控点都要有 manageable 守护").toBeGreaterThanOrEqual(4)
  })

  it("实验性行的动作是「去开关」跳转，不是开关本身", () => {
    expect(row, "实验性行给「去开关」跳转").toContain("t.profiles.goToggle")
    expect(row, "跳转要有可访问名称").toContain("t.profiles.goToggleAria(")
    expect(paneSrc, "跳转目标 = 切到实验能力 tab 并选中该能力").toContain(
      'setTab("caps")',
    )
    expect(paneSrc, "选中请求带 nonce（同一能力连跳两次也要重新触发）").toContain(
      "nonce: Date.now()",
    )
  })
})

describe("② 层序保留 + 按包名折叠：栈成员在前，重复条目不放大", () => {
  it("合并入参带 `dsh.profile.bundles` 原始栈序", () => {
    expect(paneSrc, "层序数据源必须是 detail.bundles").toContain("bundles: detail?.bundles")
  })

  it("「层 N」号标已撤（2026-09-20 维护者裁定）：顺序留在排列里，不再亮号", () => {
    // 裁定原文："我不希望展示层一 层二 这种类型的标签"。组合序仍保留（层栈成员在前），
    // 但行内不再渲染层号标记——故这两条是**反向**闸门：字典键与渲染调用都不许回来。
    const row = rowRegion()
    expect(row, "行内不得再渲染层号标记").not.toContain("tagLayer")
    expect(row, "行内不得再读层序字段").not.toContain("layerIndex")
    expect(zhDictSrc, "层号字典键已删").not.toContain("tagLayer:")
    expect(enDictSrc, "层号字典键已删（en 同）").not.toContain("tagLayer:")
  })

  it("栈先去重（dsh reconcile 保留栈内重复，壳侧不折叠就会一行变 N 行）", () => {
    expect(catalogSrc, "栈必须去重保序").toMatch(/for \(const name of input\.bundles\)/)
    // 反转证：若按原始 bundles 长度建行，重复条目会让同一个包出现多行
    expect(catalogSrc, "一个包一个展示位").toContain("const merged = new Map<string")
    expect(catalogSrc, "不得按原始 bundles 逐条建行").not.toContain("input.bundles.length")
  })

  it("一个包只留一个展示位（dependency 优先，重复条目折叠）", () => {
    expect(catalogSrc, "按包名合并槽位").toMatch(/const merged = new Map<string/)
    expect(catalogSrc, "dependency 条目优先").toContain("const entry = slot.dep ?? slot.bundle")
    expect(paneSrc, "组件只调用纯函数").toContain("mergePluginRows({")
  })
})

describe("③ 三类打标：每行一个主标记，dsh 自带实验层补「内置」", () => {
  const row = rowRegion()

  it("三个标记的文案都走字典", () => {
    expect(row, "实验性标记带具体能力名").toContain("t.profiles.tagExperimental(item.capability.label)")
    expect(row, "内置标记").toContain("t.profiles.tagBuiltin")
    expect(row, "第三方标记").toContain("t.profiles.tagThirdParty")
  })

  it("从属分组：基座是 anchor 挂全标，provider 缩进挂「后端」标（不是两个并列插件）", () => {
    // 2026-09-20 真机两轮修订。源码事实：基座官方描述 "Exclusive named browser-use
    // provider registration"、README "adds no model-visible tools"——它是能力本体的
    // 登记插槽；provider 互斥生效、挂进插槽。故从属关系 = 基座在上（anchor）、
    // provider 缩进其下，而不是两行平铺各挂一个能力标。
    expect(row, "完整实验性标只在 anchor 行").toMatch(
      /item\.capability && item\.capabilityAnchor[\s\S]{0,400}tagExperimental\(item\.capability\.label\)/,
    )
    expect(row, "anchor 且为基座时补「基座」标").toContain(
      "t.profiles.tagCapabilityBase(item.capability.label)",
    )
    expect(row, "provider 行挂「后端」标").toContain("t.profiles.tagBackend")
    expect(row, "provider 行缩进（从属关系一眼可读）").toContain("item.capabilityChild")
    expect(catalogSrc, "角色判据 = 出现在几个变体里").toMatch(/\.length > 1\s*\n\s*\? "shared"/)
    expect(catalogSrc, "anchor = 基座优先").toContain(
      'const anchor = group.find((m) => m.capabilityRole === "shared") ?? group[0]',
    )
    expect(catalogSrc, "同能力包必须相邻成组").toContain("ordered.filter((m) => m.capability?.id === capId)")
    expect(paneSrc, "去开关只在 anchor 行（同一能力一个入口）").toMatch(
      /item\.capabilityAnchor\s*\n?\s*\? \(\) => goToggleCapability/,
    )
  })

  it("组内 hover 联动（从属关系从交互上感知）", () => {
    expect(paneSrc, "组容器要接 hover 事件").toContain("onMouseEnter={() => setHoveredCap(")
    expect(paneSrc, "行要高亮整组").toContain('groupHovered ? "bg-wash/40"')
    expect(paneSrc, "同能力连续行合成一组").toContain("last.capability?.id === capId")
  })

  it("dsh 自带的实验层：主标实验性 + 补一枚内置（alsoBuiltin）", () => {
    // 反证：若渲染条件只有 kindTag==="builtin"，Agent Teams 两层会丢掉「内置」标，
    // 与维护者裁定（"多一个内置标签就好了；内置的一律不允许卸载"）相反。
    expect(row, "内置标的渲染条件必须含 alsoBuiltin").toMatch(
      /item\.kindTag === "builtin" \|\| item\.alsoBuiltin/,
    )
    expect(catalogSrc, "alsoBuiltin 的判据 = 实验能力 ∧ dsh 安装提供").toContain(
      "capability !== null && dshProvided",
    )
  })

  it("「内置」= dsh 安装提供（bundle 条目 ∧ 无依赖条目），不是「在层栈里」", () => {
    // 2026-09-20 第一版的错误判据（kind=bundle/在 bundles 里即内置）会把用户经 npm
    // 装进 profile 的层误标成内置。dsh 权威口径：removable = installed &&
    // !installation.dependencies——镜像规则见 pluginCatalog.ts 模型说明。
    expect(catalogSrc, "dshProvided 判据必须是 bundle∧非依赖").toMatch(
      /const dshProvided = slot\.bundle !== undefined && !installed/,
    )
    expect(catalogSrc, "用户装的层保留第三方主标与控制面").toContain(
      'dshProvided\n        ? "builtin"',
    )
    expect(catalogSrc, "判据注释须锚定 dsh 源码").toContain("removable")
  })

  it("兜底说明只给 dsh 安装提供的行：用户装的层/实验依赖不得落进「随 dsh 安装自带」", () => {
    // 2026-09-17 同族 bug 的复发面：合并后"没有 npm 描述"的行变多（安装层、桌面包），
    // 若兜底判据写成"非第三方"，用户经能力面板装的实验依赖会被说成 dsh 自带。
    expect(row, "兜底判据 = dshProvided").toContain("item.dshProvided ? fallbackDesc : null")
    expect(row, "不得再按层成员/bundle 类给层描述").not.toContain("p.kind === \"bundle\"")
  })

  it("安装提供的层显示「版本随 dsh」（不实读 node_modules，不谎报未安装）", () => {
    // 第一版把 bundle 行版本恒 null 显示成「未安装」，与同一行的层标自相矛盾。
    expect(row, "版本兜底走字典（版本随 dsh）").toContain("t.profiles.pluginWithDsh")
    expect(row, "版本优先级：实读 > 版本随 dsh > 未安装").toMatch(/installed_version \?\?/)
  })

  it("三类筛选用与标记同源的纯函数判据", () => {
    expect(catalogSrc, "筛选判据在纯函数里").toContain("export function matchesKindFilter")
    expect(paneSrc, "组件只消费判据").toContain("matchesKindFilter(r, kindFilter)")
  })
})

describe("④ 「底座组合」tab 退役 + 目录数据受控（禁双源）", () => {
  it("面板不再有 bundles tab，也不再有旧的层说明区", () => {
    expect(paneSrc, "底座组合 tab 已退役").not.toContain("tabBundles")
    expect(paneSrc, "tab 联合类型里没有 bundles").not.toMatch(/"plugins" \| "caps" \| "bundles"/)
    expect(paneSrc, "旧的隐藏层提示已随 tab 退役").not.toContain("hiddenLayersHint")
    expect(paneSrc, "旧的底座intro已退役").not.toContain("bundleIntroPre")
  })

  it("目录由父级取一次，能力面板只消费 props", () => {
    expect(paneSrc, "父级持有目录 state").toContain("listExperimentalCapabilities")
    expect(paneSrc, "面板拿受控数据（且是过滤后的目录）").toContain("caps={curatedCaps}")
    expect(paneSrc, "面板刷新目录走回调").toContain("onRefreshCaps={() => loadCaps(name)}")
    expect(paneSrc, "切档要清旧档目录（归属是按档的）").toContain("setCaps(null)")
  })

  it("桌面运行时说明随 tab 退役迁到插件列表（旧消费点不得消失）", () => {
    // profilesCopy.test.ts 同款判据：三段键仍被本组件消费（否则文案成为死键）。
    expect(paneSrc).toContain("t.profiles.desktopRuntimeDescPrefix")
    expect(paneSrc).toContain("t.profiles.desktopRuntimeNote")
  })
})
