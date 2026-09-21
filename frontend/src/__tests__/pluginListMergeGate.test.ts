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
//
// 判据变更史（落点跟着结构走，防护意图不松）：
//   · 2026-09-21（方案一/二 结构收敛）：清单行 / 能力卡片 / 系统底座区自
//     ProfileDetailPane 抽成 `components/profiles/pluginRows/*`，窗口切片改指新文件。
//   · 2026-09-21（**三次修订**，维护者三条裁定）：
//     ① 插件列表进去会"先闪一下全部插件，然后才收敛到社区"——分类判据同时依赖
//        行表 / 层栈 / 能力目录三条链，而首屏只等行表，另两条未到时实验包与内置层
//        **降级成社区**被渲染出来（新增 ④ 段闸门）；
//     ② 「实验性」筛选里只平铺展示实验性插件，与社区列表同构、**只读不可操作**；
//     ③ 「实验能力」模块是实验性插件的**唯一**操作入口 → 插件列表侧的能力卡片
//        （CapabilityCard）整体退役，`member` 明细行形态、角色标、就地开关/换后端
//        随之删除（新增 ⑤ 段闸门）。
import { describe, expect, it } from "vitest"

import paneSrc from "@/components/profiles/ProfileDetailPane.tsx?raw"
import rowFileSrc from "@/components/profiles/pluginRows/PluginListRow.tsx?raw"
import baseSrc from "@/components/profiles/pluginRows/SystemBaseSection.tsx?raw"
import segmentedSrc from "@/components/ui/segmented.tsx?raw"
import iconSrc from "@/components/ui/capability-icon.ts?raw"
import panelSrc from "@/components/market/ExperimentalCapabilities.tsx?raw"
import rowListSrc from "@/components/profiles/ProfileRow.tsx?raw"
import addDialogSrc from "@/components/profiles/PluginAddDialog.tsx?raw"
import catalogSrc from "@/lib/pluginCatalog.ts?raw"
import zhDictSrc from "@/content/zh-CN.ts?raw"
import enDictSrc from "@/content/en-US.ts?raw"

/** 清单行组件体（2026-09-21 三次修订后只剩一种形态：一个包一行）。 */
function rowRegion(): string {
  expect(
    rowFileSrc.includes("export function PluginListRow("),
    "找不到 PluginListRow：本闸门的窗口切片依赖它",
  ).toBe(true)
  return rowFileSrc
}

describe("① 单一入口：实验性插件的操作只在「实验能力」，插件列表侧零控制面", () => {
  const row = rowRegion()

  it("开关 / 更新 / 卸载三个入口都受 manageable（= 社区）守护", () => {
    // 拆掉任一个 `manageable &&`，实验性行就会多出一个控制面——就地开关与能力面板
    // 各说各话，卸载更会留下悬空挂载行（dsh 启动即失败）。
    // 2026-09-21 维护者裁定（方案二）：更新/卸载从常驻图标改收 `···` 溢出菜单，
    // 守护点形态随之变为 `manageable && item.row !== null`（开关）与
    // `manageable && (<Popover`（溢出菜单）。
    expect(row, "开关必须受 manageable 守护").toMatch(/manageable && item\.row !== null &&/)
    expect(row, "溢出菜单必须受 manageable 守护").toMatch(/manageable && \(\s*<Popover/)
    const guarded = (row.match(/manageable &&/g) ?? []).length
    expect(guarded, "至少三个可控点都要有 manageable 守护").toBeGreaterThanOrEqual(3)
    // 判据本体：只有社区包可控（实验性归「实验能力」、内置层归 dsh 自己的插件页）
    expect(row, "可控判据 = 社区").toMatch(/const manageable = item\.kindTag === "thirdParty"/)
  })

  it("能力卡片退役：插件列表侧不再有任何能力控制面", () => {
    // 2026-09-21 维护者裁定（三次修订）：实验性插件的操作**唯一入口**是「实验能力」。
    // 于是上一版刚做的能力卡片（就地 Switch / 换后端 / 高级设置）成了第二套控制面，
    // 与"唯一入口"直接冲突——卡片、就地开关、就地换后端一并删除。
    expect(paneSrc, "不得再引能力卡片组件").not.toContain("CapabilityCard")
    expect(paneSrc, "不得再有就地启停函数").not.toContain("toggleCapability")
    expect(paneSrc, "不得再有就地换后端函数").not.toContain("switchCapabilityVariant")
    expect(paneSrc, "不得在列表侧跑计划（计划只在能力面板里跑）").not.toContain("planReplace(")
    expect(paneSrc, "不得在列表侧跑计划").not.toContain("planDisable(")
    expect(row, "行内不得挂能力控制动作").not.toContain("toggleCapability")
    expect(row, "行内不得出现后端切换器").not.toContain("switchCapabilityVariant")
    // 反转证：这些动作仍在能力面板里（单一入口 ≠ 砍功能）
    expect(panelSrc, "能力面板持就地启停").toContain("planDisable(")
    expect(panelSrc, "能力面板持启用/换档").toContain("planReplace(")
    expect(panelSrc, "能力面板持开关").toContain("onCheckedChange={(next) => onToggle(")
    expect(panelSrc, "能力面板持移除（经确认框）").toContain("planRemove(")
  })

  it("唯一入口是**一条提示 + 一个按钮**，不逐行挂", () => {
    // 只读却不给出路 = 死胡同；但逐行挂「去管理」等于把刚收掉的第二套入口铺回 N 遍。
    expect(paneSrc, "入口按钮走字典").toContain("t.profiles.experimentalManageEntry")
    expect(paneSrc, "入口就是切到实验能力 tab").toContain('setTab("caps")')
    expect(paneSrc, "提示条只在有实验性行时出现").toMatch(
      /filteredRows\.some\(\(r\) => r\.kindTag === "experimental" && !r\.alsoBuiltin\) && \(/,
    )
    expect(row, "行内不得挂「前往实验能力」").not.toContain("experimentalManageEntry")
    expect(zhDictSrc, "提示口径写明唯一入口").toContain("统一在「实验能力」里进行")
    expect(enDictSrc, "en 同口径").toContain("experimentalReadOnlyNote:")
  })

  it("状态异常不再静默跳页：禁用开关 + 说明原因（控制权在用户手里）", () => {
    // 旧实现在「该变体不支持行级停用」时会**静默**把人踢去 caps tab（隐藏死胡同）。
    // 现在一律禁用开关并把原因挂同一行的 `aria-describedby`，用户看得见为什么点不动。
    // 三次修订后这段逻辑归能力面板（插件列表侧已无开关）。
    expect(panelSrc, "禁用条件只有三个正当理由").toContain(
      "disabled={busy || subsumedBy || blocked}",
    )
    expect(panelSrc, "禁用原因挂在开关的描述上（读屏可读）").toContain(
      "aria-describedby={why.length > 0 ? descId : undefined}",
    )
    // 反转证：`partial`（装了一半）**不许**被并进禁用条件——planReplace→planEnable
    // 正好补齐缺的步骤，那是用户唯一的一键修好入口。
    expect(panelSrc, "半装状态不在禁用条件里").not.toMatch(/disabled=\{[^}]*partial/)
    expect(panelSrc, "半装给「修复」按钮").toContain("t.market.capRepair")
    expect(panelSrc, "不得在面板里静默跳页").not.toContain('setTab("')
  })

  it("能力面板忙碌时不塌陷（开关留在原位，只禁用不卸载）", () => {
    // 旧版把整块控制区换成一颗 spinner：点一下整行控件向右跳。现在忙碌只表现为
    // 禁用 + 行内附加的小 spinner，行高与控件位置一动不动。
    expect(panelSrc, "忙碌时开关不卸载（不做三元替换）").not.toMatch(/busy \? <LoaderCircle/)
    expect(panelSrc, "忙碌时行内小 spinner 是**附加**的").toMatch(/running && <LoaderCircle/)
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
    // 2026-09-21 三次修订：实验性标**不再带能力名**（能力名上移到主标题旁的辅助灰字，
    // 标只负责"类别"）——判据随之从 `tagExperimental(label)` 改为无参调用 + 悬浮里带名。
    expect(row, "实验性标记（类别，无能力名）").toContain("t.profiles.tagExperimental}")
    expect(row, "能力名仍在（悬浮里说清属于哪项）").toContain(
      "t.profiles.tagExperimentalHint(item.capability.label)",
    )
    expect(row, "内置标记").toContain("t.profiles.tagBuiltin")
    expect(row, "第三方标记").toContain("t.profiles.tagThirdParty")
  })

  it("行只有一种形态：一个包一行，不再分卡片内明细行", () => {
    // 2026-09-21 三次修订：能力卡片退役 → 卡片内的 `member` 明细行形态、角色标
    // （功能基座 / 后端）、缩进从属关系全部没有存在意义，一并删除。
    expect(row, "不得再有 member 明细行分支").not.toContain('variant === "member"')
    expect(row, "不得再有卡片内角色标").not.toContain("tagCapabilityBase")
    expect(row, "不得再有卡片内「后端」标").not.toContain("tagBackend")
    expect(row, "不得再有组内 hover 联动").not.toContain("groupHovered")
    expect(zhDictSrc, "角色标字典键已回收").not.toContain("tagCapabilityBase:")
    expect(zhDictSrc, "「后端」标字典键已回收").not.toContain("tagBackend:")
    expect(enDictSrc, "en 同").not.toContain("tagCapabilityBase:")
    expect(enDictSrc, "en 同").not.toContain("tagBackend:")
  })

  it("名字三层：主标题 = 原本的名字，中文为辅，完整包名进 title", () => {
    // 2026-09-21 三次修订（维护者真机反馈）："插件名应该是取插件原本的名字，中文为辅"。
    // 旧版把中文友好名当主标题、包名降为灰字 —— 方向反了。
    expect(row, "实验性行主标题 = 去噪短名（那一族共有前缀是纯噪音）").toMatch(
      /item\.kindTag === "experimental" \? pluginMemberLabel\(p\.name\) : pluginShortId\(p\.name\)/,
    )
    expect(row, "辅助灰字走中文名（查表命中才有）").toContain("pluginChineseName(p.name, locale)")
    expect(row, "实验性行的中文辅名 = 所属能力名").toMatch(
      /item\.kindTag === "experimental" && item\.capability\s*\n\s*\? item\.capability\.label/,
    )
    expect(row, "查不到就不渲染辅名（不许机器音译冒充）").toMatch(/\{auxName && \(/)
    // 完整包名挂在主标题的 title 上（与官方描述拼在一起）
    expect(row, "完整包名进 title（不丢信息）").toMatch(/title=\{`\$\{p\.name\}/)
    // 反转证：旧版的「友好名当主标题 + 包名灰字」两件都已退役
    expect(row, "不得再用 pluginFriendlyName 当主标题").not.toContain("pluginFriendlyName")
    expect(row, "不得再单列一行灰字包名（与主标题重复）").not.toContain("{pluginShortId(p.name)}")
  })

  it("dsh 自带的实验层：主标实验性 + 补一枚内置（alsoBuiltin）", () => {
    // 反证：若渲染条件只有 kindTag==="builtin"，Agent Teams 两层会丢掉「内置」标，
    // 与维护者裁定（"多一个内置标签就好了；内置的一律不允许卸载"）相反。
    // 三次修订：实验性主标现在**任何**实验性行都挂（含 dsh 自带的），内置标作为补充
    // 单独一枚——与 pluginCatalog 的注释同口径（"主标实验性，补一枚内置"）。
    expect(row, "实验性主标不排除 alsoBuiltin").toMatch(
      /item\.kindTag === "experimental" && item\.capability \?/,
    )
    expect(row, "dsh 自带的实验层补一枚内置").toMatch(
      /item\.alsoBuiltin && props\.kindFilter !== "builtin" && \(/,
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
    expect(row, "不得再按层成员/bundle 类给层描述").not.toContain('p.kind === "bundle"')
  })

  it("内置层不挂版本徽标；社区行照旧如实写版本/未安装", () => {
    // 沿革：第一版把 bundle 行版本恒 null 显示成「未安装」（与同一行的层标自相矛盾），
    // 第二版改成兜底文案「内置核心 (随 DSH)」，2026-09-21 维护者真机裁定**整枚退役**：
    // "『内置核心（随 DSH）』这些应该都不需要了" —— 随 dsh 下发的层没有独立版本可读，
    // 那枚徽标既不是版本、又在重复别处已经说过的话，纯占位。
    expect(row, "不得再渲染内置兜底版本").not.toContain("pluginWithDsh")
    expect(row, "兜底键已回收").not.toContain("dshProvided\n      ?")
    expect(zhDictSrc, "字典键已回收").not.toContain("pluginWithDsh:")
    expect(enDictSrc, "en 同").not.toContain("pluginWithDsh:")
    // 反转证：实读版本仍带 `v` 前缀，社区行没装时仍如实写「未安装」
    expect(row, "实读版本带 v 前缀").toMatch(/`v\$\{p\.installed_version\}`/)
    expect(row, "未安装仍如实写").toContain("t.profiles.pluginNotInstalled")
  })

  it("三类筛选用与标记同源的纯函数判据", () => {
    expect(catalogSrc, "筛选判据在纯函数里").toContain("export function matchesKindFilter")
    expect(paneSrc, "组件只消费判据").toContain("matchesKindFilter(r, kindFilter)")
  })
})

describe("④ 首屏不闪「全部」：分类三条链落定前不渲染行（2026-09-21 真机）", () => {
  it("分类判据同时依赖行表 / 行态 / 层栈 / 能力目录，四者都落定才渲染", () => {
    // 复现（维护者）：进插件列表先闪一下**全部**插件，然后才收敛到社区。
    // 成因不是筛选逻辑（matchesKindFilter 一直是对的），而是**渲染时机**：
    //   · 目录未到 → 实验包判不出归属，降级成社区；
    //   · 层栈未到 → dsh 自带的层判不出「安装提供」，也降级成社区。
    // 首屏只等 `plugins`，于是先渲染出一份混着实验包与内置层的全量列表。
    // 行态同理：`shell_disabled` 未到前，停用的行会被先画成启用。
    expect(paneSrc, "存在分类就绪判据").toContain("const classifyReady =")
    expect(paneSrc, "判据含行表").toMatch(/classifyReady =\s*\n\s*plugins !== null/)
    expect(paneSrc, "判据含行态").toMatch(/plugins !== null &&\s*\n\s*rows !== null/)
    expect(paneSrc, "判据含层栈（失败也算落定）").toContain("(detail !== null || error !== null)")
    expect(paneSrc, "判据含能力目录（失败也算落定）").toContain(
      "(caps !== null || capsError !== null)",
    )
  })

  it("列表渲染吃这个判据，而不是只等行表", () => {
    expect(paneSrc, "列表以 classifyReady 为闸门").toMatch(/\{!classifyReady \?/)
    // 反转证：旧闸门（只等 plugins）若回来，上面那条就失效了
    expect(paneSrc, "不得再只等 plugins").not.toMatch(/\{?plugins === null \?/)
    expect(paneSrc, "降级成因写在代码里（换个人不会把三条链拆回去）").toContain("降级成社区")
  })
})

describe("③.5 主次分离：用户扩展置顶，系统底座折叠沉底（方案一）", () => {
  it("分区判据：按 kindTag 逐行分，非内置一律归用户区", () => {
    // 维护者最初截图抱怨"底座凭什么排在最上面"——分区必须由**纯判据**决定，
    // 且默认（全部）视图下系统块沉底。反转证：若把内置行也放进 user，主次就没了。
    expect(paneSrc, "存在 user/system 分区").toContain("系统预置底座")
    expect(paneSrc, "内置行归 system").toContain(
      'filteredRows.filter((r) => r.kindTag === "builtin")',
    )
    expect(paneSrc, "其余一律归 user").toContain(
      'filteredRows.filter((r) => r.kindTag !== "builtin")',
    )
  })

  it("行渲染只有一处 props 装配（三处抄同一段 = 漏改一处的同族 bug）", () => {
    // 三次修订把"卡片内明细行 / 独立行 / 底座行"三份重复的 props 收敛成一个 helper：
    // 旧版任一处漏改就是"这一栏的开关没接上"，而三份代码不会同时被读。
    expect(paneSrc, "存在统一的行渲染 helper").toContain("const renderRow = (item: MergedPluginRow")
    const rows = (paneSrc.match(/<PluginListRow/g) ?? []).length
    expect(rows, "只应有一处 <PluginListRow>（helper 内）").toBe(1)
  })

  it("系统底座区视觉上与用户区不同级（浮起 vs 沉下），但**不再有头部**", () => {
    // 用户区 = bg-panel + shadow-xs（浮起 = 你的东西）；系统区 = bg-bg/50 + 更淡边框、
    // 无阴影（沉下去 = 背景设施）。两者若同款，用户会以为底座也是自己能动的。
    // 断言前**剥注释**：注释里为了讲清对比会提到 "shadow-xs"（不是渲染事实）。
    const base = baseSrc
      .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
      .replace(/\/\*[\s\S]*?\*\//g, "")
      .replace(/^[ \t]*\/\/.*$/gm, "")
    expect(baseSrc, "系统区用下沉底色").toContain("bg-bg/50")
    expect(base, "系统区不加阴影（否则与用户区同款）").not.toContain("shadow-xs")
    // 2026-09-21 维护者真机（「内置」筛选下圈出整条头部）：标题 + "不可变更" + 折叠
    // 三件都在复述当前筛选/行内零控制面已经说过的事，整条退役。
    expect(baseSrc, "不得再有折叠按钮").not.toContain("onToggleOpen")
    expect(baseSrc, "不得再有 aria-expanded").not.toContain("aria-expanded")
    expect(baseSrc, "不得再挂头部文案").not.toContain("t.profiles.systemBase")
    expect(zhDictSrc, "头部字典键已回收").not.toContain("systemBaseTitle:")
    expect(enDictSrc, "en 同").not.toContain("systemBaseTitle:")
  })

  it("筛选芯片不许被压到逐字折行（真机截图：全部 → 全/部）", () => {
    // 2026-09-21 真机截图：筛选项被 flex 父容器压窄后中文逐字折行，芯片不再是芯片。
    // 判据 = 芯片文字单行 + 整组不可压缩。
    expect(segmentedSrc, "芯片文字单行").toContain("whitespace-nowrap")
    expect(segmentedSrc, "非 stretch 时整组不压缩").toMatch(/stretch \? "w-full" : "w-fit shrink-0"/)
    expect(segmentedSrc, "图标不参与压缩").toMatch(/cn\("shrink-0", size === "md"/)
  })

  it("工具栏三段各自成块：任何一块都不被压扁", () => {
    // 旧写法给左侧块 `flex-1 min-w-[280px]`，窄一点就先压筛选、再压按钮。
    // 现为「搜索块 + 筛选块 + 弹性空隙 + 操作块」，窄窗口整块换行而非局部压扁。
    expect(paneSrc, "搜索框固定宽度不参与压缩").toMatch(/relative w-\[190px\] shrink-0/)
    expect(paneSrc, "有弹性空隙把操作区推右").toContain('<div className="flex-1" />')
    expect(paneSrc, "操作区整块不压缩").toContain("flex shrink-0 items-center gap-1.5")
  })

  it("运行态汇总不与插件数矛盾（真机截图：列表 8 个插件却写「运行 145」）", () => {
    // runtimeSummary 的口径是 **dsh 运行时的全部 fiber 条目**，不是插件数。把它整句
    // 铺在插件列表里，数字与列表当场矛盾。现在只在有失败时冒头。
    expect(paneSrc, "只在失败时冒头").toContain("runtimeSum.failed > 0")
    expect(paneSrc, "不再直接渲染全量汇总句").not.toContain(
      "t.profiles.runtimeSummary(runtimeSummary(",
    )
    expect(paneSrc, "失败汇总走专用文案").toContain("t.profiles.runtimeFailedSummary(")
  })

  it("能力图标一能力一形（真机截图：四个能力左侧全是同一个 ✨）", () => {
    expect(iconSrc, "图标映射集中一处").toContain("export function capabilityIcon(")
    expect(iconSrc, "四个策展能力各有其形").toMatch(/"agent-team": Bot/)
    expect(iconSrc, "未知 id 退回通用实验图标（烧瓶）").toContain("?? FlaskConical")
    expect(panelSrc, "面板按 id 取图标").toContain("capabilityIcon(cap.id)")
    // 三次修订：卡片退役后，插件列表的实验性行接管这份映射（平铺后图标是唯一还能
    // 一眼看出"这几行同属一个能力"的线索）——两处必须同一份映射，禁双源。
    expect(rowFileSrc, "插件列表的实验性行复用同一映射").toContain(
      "capabilityIcon(item.capability.id)",
    )
    expect(rowFileSrc, "行内不得内联第二份能力图标映射").not.toContain("const ICONS")
  })

  it("不再有「逐行只读标」这回事（区块标题/提示条已声明）", () => {
    // 与"能力名重复"同类的毛病：同一句话说 N 遍就是噪音。底座区标题写着
    // "由桌面系统统一部署，不可变更"，每行再挂一枚「只读」标纯属重复。
    // 三次修订后内置层只会落进底座区 → 那段分支连可达性都没有了，整体删除
    // （父级也不再需要传 `hideReadOnlyBadge`）。
    expect(rowRegion(), "逐行只读标分支已删").not.toContain("pluginRuntimeReadOnly")
    expect(rowRegion(), "隐藏标开关已随之退役").not.toContain("hideReadOnlyBadge")
    expect(paneSrc, "父级不再传隐藏标开关").not.toContain("hideReadOnlyBadge")
    expect(zhDictSrc, "只读标字典键已回收").not.toContain("pluginRuntimeReadOnly:")
    expect(enDictSrc, "en 同").not.toContain("pluginRuntimeReadOnly:")
    // 反转证：区内确实没有控制面（否则删掉的就是必要的说明）
    expect(rowRegion(), "只读行没有开关").toMatch(/const manageable = item\.kindTag === "thirdParty"/)
    expect(rowRegion(), "无控制面时不渲染右列").toContain("const hasRightSide = rowBusy || manageable")
  })

  it("详情头不重复左栏已有信息（真机截图圈出的那一块）", () => {
    // 左栏选中行已经写着 profile 名 +「运行中」+「默认启动」，行尾 `···` 菜单里也有
    // 同一个「设为默认」动作。详情头再摆一遍，就是同一句话说两遍。
    // 详情头只该留两个**别处拿不到**的信息：启动/重载过渡态、清单名 package_name。
    expect(paneSrc, "详情头不再重复「运行中」徽标").not.toContain("t.profiles.runningBadge")
    expect(paneSrc, "详情头不再重复「默认启动」按钮").not.toContain("t.profiles.defaultIs")
    expect(paneSrc, "详情头不再有「设为默认」动作").not.toContain("onSetDefault")
    expect(paneSrc, "保留启动/重载过渡态（左栏只显示结果）").toContain(
      "t.profiles.reloadingWorkbench",
    )
    expect(paneSrc, "保留清单名（manifest 真实包名）").toContain("detail.package_name")
    // 左栏仍是这两件事的唯一入口（反转证：别把入口一起删没了）
    expect(rowListSrc, "左栏持「运行中」").toContain("t.profiles.runningBadge")
    expect(rowListSrc, "左栏持「默认启动」标").toContain("t.profiles.defaultBadge")
    expect(rowListSrc, "左栏持「设为默认」动作").toContain("t.profiles.setDefault")
  })

  it("添加插件只有一个入口（真机截图圈出的入口冲突）", () => {
    // 旧版工具栏并排「添加插件 / 从其他导入 / 安装插件 / 检查更新」，但前三个是同一
    // 件事的三个分身——「添加插件」弹窗里本来就有「市场 / 从其他导入 / 自定义」三条
    // 路径。四个按钮里三个通同一个面板 = 入口冲突。
    expect(paneSrc, "工具栏不再摆「从其他导入」").not.toContain("t.profiles.importBtn")
    expect(paneSrc, "工具栏不再摆「安装插件」").not.toContain("t.profiles.pluginInstallBtn")
    expect(paneSrc, "不再有内联安装输入区").not.toContain("pluginInstallPlaceholder")
    expect(paneSrc, "添加入口只此一处").toMatch(/setAddDialogOpen\(true\)/)
    expect(paneSrc, "「检查更新」是独立功能、保留").toContain("t.profiles.checkUpdatesBtn")
    // 三条路径必须都还在（合并入口 ≠ 砍功能）
    expect(addDialogSrc, "路径①：市场").toContain("pluginAddTabMarket")
    expect(addDialogSrc, "路径②：从其他导入").toContain("pluginAddTabImport")
    expect(addDialogSrc, "路径③：自定义 spec").toContain("pluginAddTabCustom")
  })

  it("默认档 = 社区（2026-09-21 维护者裁定），且空列表必须给出路", () => {
    // 进插件列表最常想看的是"我自己装的插件"，而不是把内置与实验依赖一起铺出来。
    // 撤掉「全部」芯片后，"看全部"由**再点一次已选芯片**承担（回到 all = 不筛选）。
    expect(paneSrc, "初始档 = 社区").toContain('useState<PluginKindFilter>("thirdParty")')
    expect(paneSrc, "换 profile 也回到社区（与首进一致）").toContain('setKindFilter("thirdParty")')
    // 撤掉「全部」芯片后 tablist 语义不再成立（tablist 要求恒有选中项），
    // 芯片组必须改用 filters 语义（role=group + aria-pressed，允许全不选）
    expect(paneSrc, "芯片组用 filters 语义").toContain('variant="filters"')
    expect(paneSrc, "再点一次取消 = 回到全部").toMatch(/v === kindFilter \? "all" : v/)
    // 默认档是社区 ⇒ 新工作台常常一个社区插件都没有，空列表必须给出路
    expect(paneSrc, "空状态给「显示全部」出口").toContain("t.profiles.listFilterShowAll(")
    // 反转证：真的一个插件都没装时不给这个按钮（"显示全部"也救不了，那是另一句文案）
    expect(paneSrc, "总数为 0 时不给出口").toMatch(
      /!searchQuery\.trim\(\) && merged\.rows\.length > 0 &&/,
    )
  })

  it("筛选芯片不得再出现「全部」（它是其余三者之和，冗余）", () => {
    // 撤掉的是**芯片**，不是状态：`all` 仍是"不筛选"的内部态（上面那条钉了取消回 all）。
    expect(paneSrc, "不得再渲染「全部」芯片").not.toContain("t.profiles.listFilterAll")
    expect(zhDictSrc, "「全部」字典键已随芯片退役").not.toContain("listFilterAll:")
    expect(enDictSrc, "en 同").not.toContain("listFilterAll:")
    // 顺序：社区 → 内置 → 实验性（按关心程度，维护者 2026-09-21 指定）。
    // 切片边界 = 芯片组开始 到 紧随其后的「弹性空隙」注释（那段只含这三个选项）。
    const from = paneSrc.indexOf('variant="filters"')
    const to = paneSrc.indexOf("弹性空隙", from)
    expect(from, "找不到芯片组").toBeGreaterThan(-1)
    expect(to, "找不到切片终点（芯片组结构变了？）").toBeGreaterThan(from)
    const opts = paneSrc.slice(from, to)
    const order = ["thirdParty", "builtin", "experimental"].map((v) => opts.indexOf(`value: "${v}"`))
    expect(order.every((i) => i > -1), `三个选项都要在（实际 ${JSON.stringify(order)}）`).toBe(true)
    expect(order, "顺序必须是 社区→内置→实验性").toEqual([...order].sort((a, b) => a - b))
  })

  it("底座区只负责「沉底 + 与用户区不同级」，不再自带头部与折叠", () => {
    // 原来的折叠是"默认收起、需要时展开"的附录；维护者裁掉头部后，本区就是一个
    // **常驻的下沉容器**——顺序仍由父级保证（用户扩展在前、本区最后）。
    expect(baseSrc, "本区是一个纯容器").toContain("children: ReactNode")
    expect(paneSrc, "父级仍把它排在其他行之后").toMatch(
      /userRows\.map\(\(item\) => renderRow\(item\)\)[\s\S]{0,400}systemRows\.map/,
    )
    expect(paneSrc, "折叠状态已随头部退役").not.toContain("systemBaseOpen")
  })
})

describe("⑤ 实验性行只读平铺（2026-09-21 维护者裁定：操作唯一入口 = 「实验能力」）", () => {
  it("平铺：不再按能力分组、不再有组内缩进与 hover 联动", () => {
    // 判据原文："插件列表-实验性 这里面只平铺展示实验性质的插件列表，与社区插件列表
    // 结构一致，只读不可操作"。分组卡片是上一版的形态，随之退役。
    expect(paneSrc, "不得再有能力分组切块").not.toContain("rowChunks")
    expect(paneSrc, "不得再有组内 hover 联动").not.toContain("hoveredCap")
    expect(paneSrc, "不得再有卡片级 hover 回调").not.toContain("onHoverChange")
    expect(zhDictSrc, "「高级设置」入口字典键已回收").not.toContain("goToggle:")
    expect(zhDictSrc, "就地开关相关字典键已回收").not.toContain("capToggleAria:")
    expect(enDictSrc, "en 同").not.toContain("goToggle:")
  })

  it("实验性行右侧不留空列、也不给 hover 暗示（没有控件就不装可点）", () => {
    // 只读行若照旧渲染 `min-w-[68px]` 的右列，就是每行右侧一段空白——看着像控件没加载出来；
    // 若照旧给 `hover:bg-wash`，又暗示"能点"。两条一起钉：**没控件就没有可点的样子**。
    expect(rowRegion(), "右侧列存在性有判据").toContain("const hasRightSide =")
    expect(rowRegion(), "整列按判据条件渲染").toMatch(/\{hasRightSide && \(/)
    expect(rowRegion(), "hover 高亮只给可控行").toMatch(/manageable \? "hover:bg-wash\/30" : ""/)
    expect(rowRegion(), "不得再有无条件 hover 高亮").not.toContain(
      'transition-colors hover:bg-wash/30',
    )
  })

  it("只读 ≠ 没有出口：提示条 + 唯一入口按钮在列表顶部", () => {
    expect(paneSrc, "提示条走字典（zh）").toContain("t.profiles.experimentalReadOnlyNote")
    expect(paneSrc, "按钮走字典（zh）").toContain("t.profiles.experimentalManageEntry")
    // 反转证：按钮不得是"实验性行自己的开关"——行内零控制面（上面 ① 已钉 manageable）
    expect(rowRegion(), "行内不得出现实验能力 tab 切换").not.toContain('setTab("caps")')
  })

  it("已按类筛过 ⇒ 不逐行复述类名（2026-09-21 维护者裁定）", () => {
    // 判据原文："已经按照 tab 分类了，就不需要展示 tab 本身这类的标签"。
    // 社区 tab 里每行写「社区」、内置 tab 里每行写「内置」——那是复述筛选条件，
    // 不是新信息；"全部"视图下标记照旧全给，那时它是**区分依据**。
    expect(rowRegion(), "同类判定 = 筛选值 === 自身类别").toContain(
      "const hideKindTag = props.kindFilter === item.kindTag",
    )
    expect(rowRegion(), "主标记受该判据守护").toMatch(/\{!hideKindTag &&/)
    expect(rowRegion(), "dsh 自带的实验层补标同样不复述").toContain(
      'props.kindFilter !== "builtin"',
    )
    expect(paneSrc, "父级把筛选值传给行").toContain("kindFilter={kindFilter}")
    // 反转证：三类筛选 chip 本身仍在（去掉的是行内标记，不是筛选能力）
    expect(paneSrc, "三类筛选 chip 仍在").toContain("t.profiles.listFilterExperimental")
  })

  it("表示「实验性」的图标是烧杯，不是 ✨（维护者 2026-09-21）", () => {
    // 判据原文："icon 用的与本身的意思不符，需要矫正，比如实验这类，应该可以用烧杯这种 icon"。
    // ✨(Sparkles) 的语义是"魔法/新奇"，跟"实验"没关系；烧瓶(FlaskConical)才是实验的标准符号。
    // 断言前剥注释：文件头注释里会写"由 ✨ 换成 🧪"的来龙去脉（不是渲染事实）。
    const strip = (text: string) =>
      text
        .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
        .replace(/\/\*[\s\S]*?\*\//g, "")
        .split("\n")
        .filter((line) => !line.trim().startsWith("//"))
        .join("\n")
    expect(rowRegion(), "行内实验标用烧杯").toContain("<FlaskConical")
    expect(strip(rowRegion()), "行内不得再用 ✨ 表示实验").not.toContain("Sparkles")
    expect(iconSrc, "未知能力的兜底图标也是烧杯").toContain("?? FlaskConical")
    expect(strip(iconSrc), "兜底不得再回到 ✨").not.toContain("Sparkles")
    expect(paneSrc, "「实验能力」tab 图标").toMatch(/value: "caps", icon: FlaskConical/)
    expect(paneSrc, "「实验性」筛选 chip 图标").toMatch(
      /value: "experimental",[\s\S]{0,200}icon: FlaskConical/,
    )
    expect(paneSrc, "提示条图标同为烧杯").toContain("<FlaskConical className=")
    // 反转证：✨ 在本仓库另有正当用法（"最新"排序等），故只钉**实验语义**这几处
    expect(strip(paneSrc), "实验语义处不得再用 ✨").not.toContain("Sparkles")
  })
})

describe("⑥ 「底座组合」tab 退役 + 目录数据受控（禁双源）", () => {
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
    // 三次修订：深链（focus + nonce）随能力卡片退役——入口只剩"切到这个 tab"，
    // 不再需要"选中某个能力"。面板侧也不该留着没人传的 prop。
    expect(paneSrc, "不再有 focus 深链").not.toContain("capFocus")
    expect(panelSrc, "面板不再有 focus prop").not.toContain("focus?:")
  })

  it("桌面运行时说明随 tab 退役迁到插件列表（旧消费点不得消失）", () => {
    // profilesCopy.test.ts 同款判据：三段键仍被本组件消费（否则文案成为死键）。
    expect(paneSrc).toContain("t.profiles.desktopRuntimeDescPrefix")
    expect(paneSrc).toContain("t.profiles.desktopRuntimeNote")
  })
})
