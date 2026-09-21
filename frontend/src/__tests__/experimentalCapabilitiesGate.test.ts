// experimentalCapabilitiesGate.test.ts —— 实验能力开关面板的**源码闸门**（2026-09-16）。
//
// 纯逻辑测试不渲染 DOM（AGENTS §4.3 末段），所以这里用 `?raw` + 窗口切片钉住几件
// 会**悄悄退化**的事——它们都是真实缺陷形态：
//
//  ① **清单行被实现细节污染**：钉版本 / 激活方式 / 行 id 不得出现在左栏的清单行里，
//     只能在右栏常驻的详情面出现。反向也要钉：详情面**必须**真有它们，否则第 ① 条会靠
//     "整个功能被删掉"而假绿。
//  ② **破坏性移除不确认**：`移除并卸载` 必须经 `ConfirmDialog`（U9 口径），
//     且按钮只**置起待确认态**，不得直接执行。
//  ③ **关闭 ≠ 卸载**：只有 `toggleOffSupported` 的变体才允许走行级停用；
//     含 profile 层的变体必须走移除（层的副作用回滚不了）。
//  ④ **开关必须有无障碍名称**（`Switch` 的类型闸门之外，再钉一次调用点确实传了）。
//  ⑧ **版面预算（2026-09-17 新增，本轮维护者验收的直接产物）**：面板是"看一眼四项、
//     随手开关"的地方，**清单与详情必须分栏**——清单恒紧凑、详情常驻且**不许内联展开**。
//     真机实测（1280×820）：v2 整页 1897px、四张卡 341/530/360/278px、一屏只装得下 1 项；
//     病根是把清单与详情塞进同一个纵向流，于是详情只有"内联展开（越点越长）"或"藏起来"
//     两条路，两条都错。⑧ 把这条路堵死：没有 `openDetail`、没有「详情」开关、左侧必须
//     是两栏布局的第一栏。
//
// 判据变更史（判据跟着裁定走，不跟着实现走）：
//   · 2026-09-17 v2：维护者裁定「实验功能要突出是 dsh 官方实验功能，插件的**名字**和描述
//     也要以官方为主」→ 官方名（npm 包名）进第一阅读层，判据从"包名进折叠区"放宽为
//     "钉版本 / 激活方式 / 行 id / 版本提示不进第一阅读层"。
//   · 2026-09-17 v3：维护者验收 v2 = "占用空间太多、一屏一个工具、点详情还加长卡片"
//     → 第一阅读层缩到"名称 / 状态 / 当前插件名 / 开关"，其余全部归常驻详情面。
//   · 2026-09-20（ADR-0028）：面板迁入 Profile 详情页、档位改受控 prop → 「在途不许
//     换档」判据从"下拉 disabled"改为"面板内不存在换档机制"；新增 onChanged 联动断言。
//     **迁移自查补记**：换档入口搬到父级左列表后，"在途换档"重新可达，且原请求令牌
//     拦不住（旧档回读自取更大 seq）——故补「三道闸门」断言（回读作废 / 落账复核 /
//     进度按发起档归属）。判据注释里"缺陷形态随之消失"的说法已按事实更正。
//   · 2026-09-20（ADR-0028 第二批，清单打标合并）：**目录数据也改受控**（与「插件列表」
//     共用一次回读、禁双源）——回读作废 / 请求令牌 / 换档清 dirty 三条判据随负载一起
//     搬到父级 `ProfileDetailPane.tsx`（本文件下方改指 `paneSrc`）；组件内留下的
//     是**动作侧**闸门（落账复核 / 进度归属 / 迟到的 onChanged 作废），缺它们三症状
//     （清单串档 / 失败串档 / 假重启提示）照样成立。

import { describe, expect, it } from "vitest"

import src from "@/components/market/ExperimentalCapabilities.tsx?raw"
import paneSrc from "@/components/profiles/ProfileDetailPane.tsx?raw"
import catalogSrc from "@/lib/pluginCatalog.ts?raw"
import mock from "@/lib/devMock.ts?raw"

/** **卡片头部**（能力身份 · 状态 · 当前后端 · 开关 · 展开）——"不点任何东西就能
 *  扫到的东西"全在这里。2026-09-21 三次重构：左栏清单行（CapabilityRow）并入
 *  单栏卡片的头部（CapabilityPanel），切片锚点随之改指它。 */
function listRegion(): string {
  return slice("function CapabilityPanel(", "function CapabilityPane(")
}

/** 右栏**详情面**函数体（常驻详情：插件 / 前置 / 排障 / 移除）。 */
function paneRegion(): string {
  return slice("function CapabilityPane(", "function FailureBlock(")
}

/** 按两个标记切一段源码（标记缺失即闸门自身坏了，直接失败而不是静默返回空串）。 */
function slice(startMarker: string, endMarker: string): string {
  const start = src.indexOf(startMarker)
  expect(start, `找不到 ${startMarker}：本闸门的窗口切片依赖它`).toBeGreaterThan(-1)
  const end = src.indexOf(endMarker, start)
  expect(end, `找不到 ${endMarker}：本闸门的窗口切片依赖它`).toBeGreaterThan(start)
  return src.slice(start, end)
}

/** 去掉行注释与块注释（组件里的中文注释不该被"无内联中文"这条误伤）。 */
function stripComments(text: string): string {
  return text
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split("\n")
    .filter((line) => !line.trim().startsWith("//"))
    .join("\n")
}

describe("⑧ 版面预算：清单 / 详情分栏，详情**不得**内联展开（2026-09-17）", () => {
  it("没有「详情」展开开关，也没有 openDetail 折叠态", () => {
    // 这正是维护者验收 v2 时点名的那个交互："点击详情还又加长卡片内容"。
    expect(src, "详情不能再有内联展开态").not.toContain("openDetail")
    expect(src, "「详情」展开控件已移除（复制文案 key 也不该留着）").not.toContain("capDetailToggle")
    expect(src, "详情面不该再包在高度动画里").not.toMatch(/AnimatePresence/)
  })

  it("单栏手风琴：一屏看全能力，同一时刻只展开一个（2026-09-21 三次重构）", () => {
    // 结构调整 = 判据跟着走：v2 详情内联展开（会把第二张卡推出屏幕，真机 1897px）；
    // v3 改左右两栏（解决了高度，却让左右身份必然重复、且与「插件列表」范式分叉）；
    // 本次改**单栏手风琴**——与「插件列表」同范式（一能力一卡片、头部控制、展开看细节），
    // v3 担心的"越长越高"由"同时只展开一个"挡住。
    expect(src, "不得再有左右分栏栅格").not.toContain("lg:grid-cols-[minmax(")
    expect(stripComments(src), "不得再有窄窗下钻态（注释提及历史不算）").not.toContain("drilled")
    // 手风琴的身份：一个 state + 互斥切换
    expect(src, "展开态是受控 state").toContain("const [expandedId, setExpandedId]")
    expect(src, "再点一次收起（互斥）").toMatch(/cur === cap\.id \? null : cap\.id/)
    expect(src, "展开体复用 CapabilityPane（零功能丢失）").toMatch(
      /\{expanded && \(\s*<CapabilityPane/,
    )
  })

  it("行是**单行紧凑**结构（不给整块段落留位置）", () => {
    const row = listRegion()
    // 行里出现块级段落 = 行会重新变高。行只允许 span / 按钮 / 开关 / ⓘ。
    expect(row, "清单行里出现了块级段落").not.toMatch(/<p[\s>]/)
    // 一个能力一行（而不是"一档一行"）：变体清单归详情面。
    expect(row, "清单行不得渲染完整变体清单").not.toContain("cap.variants.map(")
    // 描述性信息**只在 ⓘ 里**（2026-09-21 三次修订，维护者口径："希望减少描述性信息，
    // 若必须要则收敛到小 tip icon 里面去"）。判据随之改写：旧版是"长描述归详情面"
    // （cap.summary 不许出现在行里），现在是"行里可以有它、但只能作为悬浮文本"——
    // 平铺成段落即违约，故下面同时钉住"进了 Tip"与"没有 `<p>`"两条。
    expect(row, "能力说明必须收进 ⓘ").toContain("capTipText(cap.summary, cap.unlocks)")
    // 插件名（2026-09-21 维护者："实验能力这个模块的 UIUX 设计太差了"）：
    // 原口径是"完整包名折行显示、不许截断"——不许截断是对的（截掉的正好是区分各档
    // 的尾部），但**折行**的代价是每行两行高、且三个后端的前 40 字符完全相同
    // （`@deepseek-ai/dsh-experimental-browser-use-`），眼睛扫不出区别。
    // 现改为**去噪短名**：去掉 scope 与 `dsh`/`experimental` 噪音段，尾部（区分点）
    // 完整保留——既不截断，也不再折行；完整包名进 `title`，排查/读屏仍可取。
    expect(row, "插件名走去噪短名").toContain("variantShortName(cap, target.id)")
    expect(row, "完整包名进 title（不丢信息）").toMatch(/title=\{name\}/)
  })

  it("清单行保住必须「扫一眼就有」的四件事：名称 / 状态 / 插件名 / 开关", () => {
    const row = listRegion()
    for (const required of ["cap.label", "StateBadge", "variantDisplayName(cap,", "Switch"]) {
      expect(row, `清单行缺少「${required}」`).toContain(required)
    }
  })
})

describe("① 实现细节整节退役（2026-09-21 维护者真机裁定）", () => {
  // 沿革：v2 把这批东西**收进详情面**（"清单行不许被实现细节污染"），2026-09-21 维护者
  // 真机圈出「实现细节（排障用） 2 个包」整行："实现细节这个板块我觉得应该也不需要了"。
  // 判据随之从"只能在详情面"变成"**哪里都不再有这一节**"——但反向那半条（清单行不许
  // 出现）仍然成立，故保留在下面，并补一条"排障出口还在"的反转证（免得靠删功能假绿）。
  it("清单行里没有钉版本、激活方式、行 id 与版本提示", () => {
    const row = listRegion()
    for (const forbidden of [
      "capPinned",
      "capImplTitle",
      "capActivationAuto",
      "capActivationInsert",
      "s.rowId",
      "versionNotice",
      "s.spec",
      "s.ordinal",
    ]) {
      expect(row, `清单行出现了「${forbidden}」`).not.toContain(forbidden)
    }
  })

  it("展开体里也没有这一节（板块整体删除，不是搬家）", () => {
    for (const gone of [
      "capImplTitle",
      "capImplPackages",
      "capActivationAuto",
      "capActivationInsert",
      "capOfficialDesc",
      "capOfficialDescPending",
      "capStepLive",
      "capStepRowMissing",
      "capStepPackageMissing",
      "stepStatus",
      "showImpl",
    ]) {
      expect(src, `「${gone}」应随本节退役`).not.toContain(gone)
    }
    expect(paneRegion(), "详情面不得再逐包渲染 steps").not.toContain("target.steps.map(")
    // 反转证：排障出口没有跟着消失——进度导轨（正在装哪个包）与失败块仍在
    expect(src, "进度导轨仍在").toContain("t.market.capRunning(")
    expect(src, "失败块仍在").toContain("function FailureBlock(")
  })

  it("插件名（官方名）在清单行与详情面都有，且由派生规则给出", () => {
    expect(src, "详情面缺「插件」节标").toContain("t.market.capPluginsLabel")
    // 名字必须来自**派生规则**（标识包 = 该档独有；基座包另标共用），而不是我们发明的
    // 「Web 档 / 复用已装的 cua-driver」这类标签——`variantPackageRoles` 有单测钉住取值。
    expect(paneRegion(), "详情面的插件名必须由 variantPackageRoles 派生").toContain(
      "variantPackageRoles(cap, v.id)",
    )
    expect(src, "变体行首必须渲染标识包（roles.primary）").toContain("roles.primary.map(")
    expect(src, "共用基座包要如实标注，不得省略").toContain("capAlsoInstalls")
  })

  it("官方来源仍在面板头（「这不是社区插件」的口径不能随该节一起丢）", () => {
    expect(src, "面板头缺官方标记").toContain("t.market.capOfficialBadge")
    expect(src, "面板头缺官方来源说明").toContain("t.market.capOfficialNote")
  })

  it("包名仍由派生规则给出（承包的官方简介退役 ≠ 包名可以不来自事实）", () => {
    expect(src, "变体行首必须渲染标识包（roles.primary）").toContain("roles.primary.map(")
    expect(src, "共用基座包要如实标注，不得省略").toContain("capAlsoInstalls")
    // 2026-09-21 三次修订的正面证据：`cap.summary` / `cap.unlocks` 搬进了头部 ⓘ
    // （描述文本不是操作步骤）——它们必须还在（真删掉就是功能丢失），只是换了呈现面。
    expect(listRegion(), "价值描述与简介搬进 ⓘ 后仍须在").toContain(
      "capTipText(cap.summary, cap.unlocks)",
    )
  })

  it("用户视角的信息必须在清单行或详情面上（价值 / 前置 / 状态）", () => {
    // 价值描述（unlocks）现在住头部 ⓘ，前置与状态仍在详情面/行里。
    expect(listRegion(), "缺少价值描述（应挂在头部 ⓘ）").toContain("cap.unlocks")
    expect(paneRegion(), "详情面缺少前置").toContain("prerequisites")
    for (const required of ["cap.label", "Switch"]) {
      expect(listRegion(), `清单行缺少「${required}」`).toContain(required)
    }
  })
})

describe("①b 行内状态徽标按**变体自己**的状态取词（2026-09-17）", () => {
  it("「当前档」标记不承诺「运行中」（activeVariant 含已就位但停用）", () => {
    // 2026-09-17 真机抓到的形态：变体行**无条件**写「已启用」，而 `activeVariant` 的
    // 契约是"当前生效**或已就位但停用**"的那一档 —— 于是面板说「已停用」时，
    // 卡片上还挂着「已启用」，自相矛盾。
    //
    // 2026-09-21 二次重构后，对照卡上只剩**形状**标记（实心 = 当前档），不再有任何
    // 文字状态断言：措辞的唯一来源是 `StateBadge`，而它按 `cap.state` / `variant.state`
    // 逐档取词（含 `off` / `disabled` / `partial` / `subsumedBy` / `conflict`）。
    // 故本条翻转为**反向闸门**：卡内不得再出现状态文字，状态必须走 StateBadge。
    // 三次修订（同日）把身份块（含 StateBadge）整体移出展开体 → 正向落点改指**卡片头部**
    // （`listRegion()`）：状态徽标仍在，只是在"扫一眼"那一层，展开体里一个都不许有。
    const pane = paneRegion()
    expect(pane, "卡内不得再写「已启用」").not.toContain("t.market.capStateOn")
    expect(pane, "卡内不得再写其它状态词").not.toContain("t.market.capStatePartial")
    expect(pane, "展开体不得再挂状态徽标（身份归头部）").not.toContain("<StateBadge")
    expect(listRegion(), "状态措辞唯一来源 = StateBadge").toContain("<StateBadge")
    // 反转证：StateBadge 本身仍逐档取词（否则上面那条就成了空防护）
    const badge = slice("function StateBadge(", "function opLabel(")
    expect(badge, "StateBadge 按状态取词").toMatch(/disabled: \{ text: t\.market\.capStateDisabled/)
    expect(badge, "含 off 档").toMatch(/off: \{ text: t\.market\.capStateOff/)
    expect(badge, "含被包含档").toContain("variant.subsumedBy")
  })
})

describe("② 破坏性移除必须走确认框", () => {
  it("面板引用了 ConfirmDialog（U9 口径：不用原生 window.confirm）", () => {
    expect(src).toContain("ConfirmDialog")
    expect(/\bwindow\s*\.\s*confirm\s*\(/.test(src)).toBe(false)
  })

  it("「移除并卸载」只置起待确认态，不直接执行", () => {
    expect(src).toContain('setPending({ kind: "remove"')
    // 直接执行的动作只允许出现在 execute/runEnable/runDisable 这条链上；
    // 详情面的 onRemove 回调不得调用 planRemove。
    expect(src).not.toMatch(/onRemove=\{[^}]*planRemove/)
  })

  it("确认框列出不可逆影响面（包 / 挂载行 / 层列表）", () => {
    expect(src).toContain("capRemovePointPackages")
    expect(src).toContain("capRemovePointRows")
    expect(src).toContain("capRemovePointLayer")
  })
})

describe("③ 关闭 = 停用行，只有纯行级变体才允许", () => {
  it("planDisable 只在 toggleOffSupported 为真时才被调用（层类变体走移除）", () => {
    const calls = src.split("\n").filter((l) => l.includes("planDisable("))
    expect(calls.length, "planDisable 调用点数量变了，需重新核对分支").toBeGreaterThan(0)
    // 调用点必须被 `variant.toggleOffSupported` 分支守护。
    expect(src).toMatch(/if \(variant\.toggleOffSupported\) \{\s*\n\s*runDisable\(/)
  })

  it("停用与移除是两条路（关闭不必卸载）", () => {
    expect(src).toContain("planRemove(")
    expect(src).toContain("capRemoveExplainedSoft")
  })
})

describe("④ 无障碍与 i18n", () => {
  it("每个开关都带 aria-label", () => {
    const switches = src.split("<Switch").slice(1)
    expect(switches.length).toBeGreaterThan(0)
    for (const s of switches) {
      const tag = s.slice(0, s.indexOf("/>"))
      expect(tag, `Switch 缺 aria-label：${tag.slice(0, 60)}`).toContain("aria-label")
    }
  })

  it("头部有「可展开」的可访问状态（展开态不能只靠图标方向）", () => {
    const head = listRegion()
    expect(head, "头部要报展开/收起").toContain("aria-expanded={expanded}")
    expect(head, "头部要指向它控制的展开体").toContain("aria-controls={panelId}")
    expect(head, "展开体要有对应的 id").toContain("embeddedId={panelId}")
  })

  it("组件内没有内联中文文案，也没有内联 CJK 标点（一律走 t.market.* 字典）", () => {
    // 口径对齐 `enUsNoLeak.test.ts` 的 CJK 定义（汉字 + CJK 标点 + 全角字符）：
    // 原来只查汉字，于是 `：`「（」这类全角标点在 en 界面里原样泄漏而闸门全绿
    // ——2026-09-17 独立复核实测（`Could not read experimental capabilities：…`）。
    const cjk = stripComments(src).match(/[\u3000-\u303f\u4e00-\u9fff\uff00-\uffef]/g) ?? []
    expect(cjk, `组件里有内联 CJK 文案/标点：${cjk.join("")}`).toEqual([])
  })
})

describe("⑤ 子集档（Agent Teams 的两档）不得被当成互斥后端", () => {
  it("`subsumedBy` 存在时开关被禁用、不给修复/移除（否则会拆掉超集档的基础层）", () => {
    // 真机暴露（2026-09-16）：自建档 = [agent-team-profile]，Web 档 = [同一包, web 层]。
    // 装 Web 档时子集档的包必然也齐 —— 那不是"两个后端并列"，报"冲突"是误报。
    const dock = src
    expect(dock).toContain("subsumedBy")
    expect(dock, "开关必须因被包含而禁用").toMatch(
      /disabled=\{busy \|\| subsumedBy( \|\| blocked)?\}/,
    )
    expect(dock, "被包含的档不得提供冲突修复入口").toMatch(
      // 2026-09-16 追加：前置门挡住的档**也不得给「修复」**——修复=装包+写行，
      // 后端两道都会拒，按钮点下去必失败（假按钮）。三个排除项必须同处一个条件里。
      /!subsumedBy[\s\S]{0,60}!blocked[\s\S]{0,60}!failure[\s\S]{0,40}target\.state === "partial"/,
    )
    expect(dock, "被包含的档不得提供独立移除入口").toMatch(/!subsumedBy &&\s*\n?\s*\(target\.state === "on"/)
    // 状态徽标要显式说"已包含"，而不是照抄底层 On/Disabled。
    expect(dock).toMatch(/if \(variant\.subsumedBy\) \{/)
    expect(dock).toContain("capStateSubsumed")
  })
})

describe("⑦b 面板徽标与变体行标记（2026-09-16 用户真机验收第二轮）", () => {
  it("徽标说能力的实话，不照「当前选中的后端」渲染", () => {
    // 真机：桌面控制里「随包自带运行时」正在生效、用户点了被前置门挡住的
    // 「复用已装 cua-driver」，旧写法按选中档渲染 → 整个面板报「需要修复」。
    expect(src).toMatch(/const shown: keyof typeof map = cap\.state/)
    expect(src, "不得再照选中档渲染徽标").not.toMatch(/const shown = target\.state/)
  })

  it("被前置门挡住的档在**它自己那一行**里有标记（否则切档看不出区别）", () => {
    expect(src).toMatch(/const vBlocked = v\.prerequisiteMissing/)
    expect(src, "原因必须渲染出来").toMatch(/\{vBlocked && \(/)
  })
})

describe("⑦ 宿主前置缺失 = 硬门（2026-09-16 真机事故）", () => {
  it("前置缺失时开关禁用，且把后端给的原因**露在详情面上**（不是藏起来）", () => {
    // 事故：缺 cua-driver 时装上 cua-driver-mcp → dsh 插件树加载失败 → 工作台起不来。
    // 所以这不是"提示"，是门：禁用开关 + 说明为什么灰。
    expect(src).toMatch(/const blocked = Boolean\(target\.prerequisiteMissing\)/)
    expect(src, "前置缺失必须并入 Switch 的 disabled").toMatch(
      /disabled=\{busy \|\| subsumedBy \|\| blocked\}/,
    )
    // 原因必须在**详情面**（常驻、不点任何东西就能看到），且用警示色——原样展示后端文案
    // （禁前端自造）。2026-09-17 v3 后它落在被挡的那个变体行内：span + text-danger。
    const pane = paneRegion()
    expect(pane, "前置缺失的原因必须在详情面上").toContain("vBlocked")
    expect(pane, "原因必须用警示色").toMatch(/text-danger[\s\S]{0,240}?\{vBlocked\}/)
  })

  it("被前置门挡住的清单行只给一个小标记（原因正文在详情面，行不因此变高）", () => {
    const row = listRegion()
    expect(row, "行内的门标记来自 blocked").toContain("blocked && <TriangleAlert")
  })
})

describe("⑥ 失败态与状态衔接：不许说假话、不许两个按钮干一件事", () => {
  it("四处「层类能力」判据共用同一个带「真就位」守卫的纯函数", () => {
    // 真机 2026-09-16：一次失败的安装把面板变成 partial，而 toggleOffSupported 对
    // 没装齐的档也是 false（分类要读包自己的 manifest）→ 面板谎称"本档由 profile 层提供"。
    // 当时只修了 `offIsRemove` 一处，另三处（移除确认框的 note 与要点、详情面的移除说明）
    // 照旧裸读 `toggleOffSupported`——2026-09-17 独立复核把这三处一起抓到。
    // 现在四处必须**共用** `offMeansRemove`（判据在 `lib/experimentalCapabilities.ts`，
    // 有单测），任何一处再裸读就红。
    const uses = (src.match(/offMeansRemove\(/g) ?? []).length
    expect(uses, "层类判据的调用点少于四处（有地方又裸读了 toggleOffSupported）").toBeGreaterThanOrEqual(4)
    // 详情面里不得再出现裸判：`toggleOffSupported` 只允许出现在"关闭=停用"的动作分支上。
    const pane = paneRegion()
    expect(pane, "详情面不得裸读 toggleOffSupported 决定文案").not.toContain("toggleOffSupported")
    const bare = src
      .split("\n")
      .filter((l) => l.includes("toggleOffSupported") && !l.includes('"on"') && !l.includes("runDisable"))
    // 允许的裸读只有两种：动作分支（`if (variant.toggleOffSupported)`，紧邻 runDisable/remove）
    // 与 `offMeansRemove` 自身所在文件。这里把"出现在本组件里的裸读"全部列出来人工可审。
    expect(bare.filter((l) => !/if \(variant\.toggleOffSupported\)/.test(l)), "有裸读 toggleOffSupported 的渲染分支").toEqual([])
  })

  it("有失败块时不再渲染「修复」——它与「继续剩余步骤」是同一个动作", () => {
    expect(src).toMatch(
      /!subsumedBy[\s\S]{0,60}!blocked[\s\S]{0,60}!failure[\s\S]{0,40}target\.state === "partial"/,
    )
  })

  it("失败先给一句人话，原始输出折叠在后面（不把 200 字符的 pnpm 输出铺在详情面上）", () => {
    // 分类由**后端**给（`plugin_registry::classify_failure`）→ 前端只按 kind 选文案，
    // 不得再写第二份正则（否则两份分类会漂移：后端换源了、前端却说不是网络问题）。
    expect(src).toContain("failureKind")
    expect(src, "前端不得自带失败正则分类").not.toMatch(/tls handshake|failed to fetch metadata/)
    expect(src).toContain("capFailNetwork")
    expect(src).toContain("capFailNotFound")
    expect(src).toContain("capFailBuildApproval")
    expect(src, "原始输出必须可折叠").toMatch(/aria-expanded=\{showRaw\}/)
    // 原始输出不得直接铺在详情面第一层（现在只在展开后出现）。
    expect(src).toMatch(/showRaw && \(/)
  })

  it("动作一发起就展开该能力（「就地呈现」= 那张卡片自己打开）", () => {
    // v2 靠"卡片就地展开"，v3 靠"把右栏切过去"，本次单栏手风琴又回到"卡片自己展开"
    // ——三版形态不同，衔接要求一样：进度与失败必须出现在用户正看着的地方
    // （改版时最容易漏的一条）。
    expect(src).toMatch(/setExpandedId\(cap\.id\)/)
    // 失败/进度在清单行上也要有标记，否则用户看不出是哪一项出的问题。
    const row = listRegion()
    expect(row, "行上要有在跑标记").toContain("animate-spin")
    expect(row, "行上要有失败标记").toContain("failed && <TriangleAlert")
  })
})

describe("⑨ dev mock 必须与 serde 输出同形（2026-09-17 真机复盘）", () => {
  it("每个变体都带 prerequisiteMissing（serde 不 skip_serializing_if，缺省会退化成 undefined）", () => {
    // 真事：mock 漏写这个字段 → `undefined !== null` 为真 → 四行全被判成"前置缺失"，
    // 红灯 + 开关全灰。契约字段在 mock 里少一个，前端就会以"看起来很合理"的方式失真。
    const variants = (mock.match(/toggleOffSupported:/g) ?? []).length
    const prereqs = (mock.match(/prerequisiteMissing:/g) ?? []).length
    expect(variants, "mock 里一个变体都没有？").toBeGreaterThan(0)
    expect(prereqs, "变体数与 prerequisiteMissing 数不一致（漏字段）").toBe(variants)
    // 且至少留一条真值，否则"硬门"这条分支在浏览器直开时永远走不到。
    expect(mock, "mock 里没有 prerequisiteMissing 的真值：硬门走不到").toMatch(
      /prerequisiteMissing:\s*\n?\s*"/,
    )
  })
})

describe("⑩ 动作互斥：一次只能跑一个动作（2026-09-17 独立复核）", () => {
  it("busy 按**全局** run 算，不按能力算（否则两个能力的动作会并发）", () => {
    // `run` 是单槽，而行写（写/删/停用挂载行）不入队、`plugins.rs` 也没有互斥：
    // A 在跑时用户点 B，`setRun({capId:B})` 会把 A 的进度与 busy 一起顶掉，A 于是能再被启动
    // → 两条编排行交错，同一份 `cordis.patch.yml` 上的 read-modify-write 可能丢更新。
    // 单栏手风琴后的完整传参链：父级 → 卡片 → （头部开关 / 展开体）。
    // 链路任何一环断了，都会出现"别人跑着时还能启动"的窗口——比数字面量更该钉的是链。
    expect(src, "父级把全局 busy 传给卡片").toMatch(/busy=\{run !== null\}/)
    expect(src, "卡片把 busy 透给展开体").toMatch(/busy=\{busy\}/)
    expect(listRegion(), "卡片头部的开关也吃 busy").toMatch(/disabled=\{busy \|\|/)
  })

  it("档位受控于父级（ADR-0028）：面板内不再有自持档位切换机制", () => {
    // 原判据「在途时不许换档」（`Select disabled={run !== null}`）守护的是**面板内下拉**——
    // 该下拉已随迁入 Profile 详情页删除：档位是必传 prop（= ProfileDetailPane 选中档）。
    // **但"在途换档"这个缺陷形态没有消失**：换档入口搬到了父级左列表，面板的 busy 管不到
    // 那条路 → 见下方「在途换档」一节的三道闸门（2026-09-20 迁移自查时补）。
    expect(src, "不得再有自持档位 state").not.toContain("setProfile")
    expect(src, "不得再有档位下拉（Select 已随迁入移除）").not.toContain("<Select")
    expect(src, "档位必须是受控 prop").toContain("profile: string")
    // 同作用域联动的落点：写操作完成必须回调父级刷新同页「插件列表」。
    expect(src, "写后必须回调父级刷新（onChanged）").toContain("onChanged?.()")
    // 目录数据同受控（ADR-0028 第二批）：面板不自己取数，由父级 props 喂。
    expect(src, "目录数据必须是受控 props").toContain("caps: Capability[] | null")
    expect(src, "面板内不得再自行 invoke 能力目录").not.toContain("listExperimentalCapabilities")
  })

  it("在途动作的回读不许覆盖新档的清单（请求令牌）", () => {
    // 失败路径：A 在途 → 切到 B → B 的 load 先写 caps → A 的 finally 用旧 profile 回读覆盖面
    // → 下拉显示 B、清单是 A 的事实，之后按 A 的 installed 算计划装进 B。
    // 2026-09-20 第二批：回读随负载搬到父级（`ProfileDetailPane::loadCaps`），令牌判据
    // 随之改指 `paneSrc`——负载在哪，判据跟到哪，不靠"组件里看不到就当时不存在"。
    expect(paneSrc, "父级的目录回读要有令牌").toContain("capsSeq")
    expect(paneSrc, "只接受最后一次请求").toMatch(
      /if \(seq !== capsSeq\.current\) return\s*\n\s*setCaps\(next\)/,
    )
  })

  it("换档清状态时连 dirty 一起清（否则「立即重启」打在没改过的档上）", () => {
    // dirty 是**按档**的事实（哪个档的配置被改过）。第二批起负载搬到父级，但"清 dirty"
    // 仍在本组件（它是组件内 state），判据改为：换档 effect 里必须连 dirty 一起清。
    expect(src).toMatch(
      /useEffect\(\(\) => \{[\s\S]{0,240}setDirty\(false\)\s*\n\s*\}, \[profile\]\)/,
    )
  })

  it("在途换档（ADR-0028 迁移带出的新可达态）：旧档的回读与落账整体作废", () => {
    // 旧版在途换档被面板内下拉的 `disabled={run !== null}` 挡着；换档入口搬到父级左列表
    // （侧栏 ProfileRow 的 onSelect 不受面板 busy 约束）后这条路重新可达，且**令牌拦不住**：
    // 旧档的回读自取更大的 seq，成了"最后一次请求"。三道闸门缺一，症状分别是——
    // ① 清单串档：下拉/左列表显示 B、清单是 A 的事实；② 失败串档：A 的失败挂在 B 的同名
    // 能力上（两档能力 id 同名）；③ 假重启提示：B 档弹"配置已变更"，重启一个没改过的档。
    // 闸门一（回读作废）随负载在父级：旧档回读整体作废，不占令牌、不发请求。
    expect(paneSrc, "旧档回读必须整体作废（不占令牌）").toMatch(
      /if \(nameRef\.current !== profile\) return[\s\S]{0,120}capsSeq\.current \+= 1/,
    )
    // 闸门二（落账复核）在组件：动作结果落账前先看档位是否还在。
    expect(src, "动作结果落账前必须复核档位").toMatch(
      /if \(profileRef\.current !== profile\) return/,
    )
    // 进度按**发起档**归属：两档能力 id 同名，单看 capId 必然把转圈画到新档头上。
    expect(src, "RunState 必须钉住发起档").toMatch(/interface RunState \{[\s\S]{0,80}profile: string/)
    expect((src.match(/run\.profile === profile/g) ?? []).length, "行与详情面都要按档归属").toBeGreaterThanOrEqual(2)
    // 父级刷新同样要挡：`onChanged` 是发起时的闭包（钉着旧 profile），迟到调用会把
    // 旧档的详情/行表写进父级此刻显示的新档页面。
    expect(src, "换档后的父级刷新必须一并作废").toMatch(
      /if \(profileRef\.current === profile\) \{[\s\S]{0,140}onChanged\?\.\(\)/,
    )
  })

  it("dev mock 与契约同形（tsc 兜底，不再数出现次数）", () => {
    expect(mock, "mock 的载荷要按契约标注（少字段编译期就红）").toContain(
      "] satisfies Capability[]",
    )
  })
})

describe("⑪ dsh 自带的能力不进面板：官方插件页托管，dock 不摆第二套入口（2026-09-20 修订）", () => {
  // 上游事实：dsh 0.1.6-alpha.2 起把 Agent Teams 两个 bundle 作为 **optional bundle**
  // 随安装包下发（`packages/boot/app-boot/src/profile.ts` 的 `OPTIONAL_BUNDLES` ＋
  // `.agents/notes/implemented/process/2026-09-15-shipped-optional-bundles.md`）：
  // 随包下载、默认关、在 **dsh 自己的插件页**开关、**永不卸载**。
  //
  // 判据变更史：2026-09-17 立的原判据是"面板展示但只说明+清理遗留副本"（ShippedPane）；
  // 2026-09-20 维护者裁定 **直接在面板里不展示**——它已被官方插件管理托管，dock 摆一份
  // 说明页仍是双入口（ADR-0020 §2.8）。判据随裁定翻面：从"必须有 ShippedPane"变为
  // "必须过滤掉"。过滤规则单一：`lib/pluginCatalog.ts::dockCuratedCaps`（父级过滤一次，
  // 面板展示与插件列表打标共用）。

  it("面板不再渲染任何 shipped 分支（ShippedPane / 徽标 / 开关豁免全部移除）", () => {
    expect(src, "ShippedPane 已随裁定移除").not.toContain("ShippedPane")
    expect(src, "面板内不得再有任何 shipped 判定分支").not.toContain("shippedByDsh")
    expect(src, "自带的说明/指路/清理文案键已随面板分支移除").not.toContain("capShipped")
    expect(src, "遗留副本清理入口已移除").not.toContain("capLegacyCleanup")
  })

  it("过滤发生在父级数据边界（单一规则，面板与插件列表共用）", () => {
    expect(catalogSrc, "过滤规则是纯函数").toContain("export function dockCuratedCaps")
    expect(catalogSrc, "判据锚定安装实测（shipped_by_dsh）").toMatch(/!cap\.shippedByDsh/)
    expect(paneSrc, "父级过滤一次").toContain("dockCuratedCaps(caps ?? [])")
    expect(paneSrc, "能力面板拿过滤后的目录").toContain("caps={curatedCaps}")
    expect(paneSrc, "插件列表同样用过滤后的目录打标").toContain("caps: curatedCaps,")
  })

  it("dev mock 保留一个自带能力（契约形状 + 过滤路径在浏览器直开时可见）", () => {
    // mock 仍带 shippedByDsh: true：它是契约字段（后端在下发），且浏览器直开时
    // "少一项"正是过滤生效的可观察证据。
    expect(mock, "mock 至少要留一个自带能力").toMatch(/shippedByDsh: true/)
    expect(mock, "mock 的载荷要按契约标注（少字段编译期就红）").toContain(
      "] satisfies Capability[]",
    )
  })
})

describe("⑫ v4 控制面/后果面：行内即事实、换后端走溢出菜单、无 picked 中间态", () => {
  // 2026-09-20 维护者要求"布局和交互着重重构"，经 ui-ux-pro-max 规则核验后定稿：
  //  · 左栏 = 唯一控制面（开关 + 切换后端菜单）；右栏 = 后果面（只读 + 恢复 + 危险区）；
  //  · picked 中间态取消 → misaligned 隐藏态死亡（行里显示的自古以来就是事实）；
  //  · 换后端**不放行内 pills**（compact-label-overflow / truncation-strategy /
  //    web-target-size 叠加致死），走 overflow-menu。

  it("picked / misaligned 中间态已移除（行里显示的就是事实）", () => {
    expect(src, "不得再有 picked 选中态").not.toContain("setPicked")
    // 注释里会写"misaligned 已死亡"的来龙去脉，故剥注释后判码
    expect(stripComments(src), "不得再有不一致的选中态").not.toContain("misaligned")
    expect(src, "行的开关目标来自 rowTarget（生效档 → 首个可用档）").toMatch(
      /const rowTarget = useCallback/,
    )
    expect(src, "每张卡片同样用 rowTarget（详情与移除作用于事实档）").toContain("target={rowTarget(cap)}")
  })

  it("换后端只有一个入口：右栏对照列表（2026-09-21 二次重构）", () => {
    // 旧形态有两处入口做同一件事：左栏一个**无文字的 ⇄ 图标**（BackendMenu 触发器）
    // + 右栏的「切换到此档」按钮。前者既看不懂（只能 hover 猜），又是纯冗余。
    // 现在只保留右栏那处（带文字），并**保留**"清单行不得内联变体清单"这条老防护
    // ——它拦的是"把三档铺回左栏行里"（会撑爆 21–25rem 栏宽）。
    expect(src, "BackendMenu 已退役").not.toContain("function BackendMenu(")
    expect(src, "左栏不再有第二处换后端入口").not.toContain("t.market.capMenuBackend(")
    expect(listRegion(), "清单行不得内联渲染变体清单").not.toContain("cap.variants.map(")
    expect(paneRegion(), "唯一入口在右栏对照列表").toContain("t.market.capSwitchToThis")
    expect(src, "右栏换档复用 switchBackend（无第二套启用路径）").toContain(
      "onSwitchBackend={(v) => switchBackend(cap, v)}",
    )
  })

  it("对照列表用单选标记：一眼看出「互斥、选一个」", () => {
    // 三张并排卡原先看不出是"三选一"（像三个独立插件）。加形状可辨的单选标记
    // （实心/空心，不只靠颜色），生效档**不再挂文字徽标**——左栏那一行已经写着
    // 同一件事，同屏说两遍是最典型的冗余。
    const pane = paneRegion()
    expect(pane, "有单选标记（实心=生效 / 空心=可选）").toMatch(
      /isActive \? "border-brand bg-brand" : "border-line bg-panel"/,
    )
    expect(pane, "生效档不重复能力级徽标").not.toContain("t.market.capStateOn")
  })

  it("菜单点选与开关同链：都经 activate（该确认的确认）", () => {
    expect(src, "启用链统一收口到 activate").toMatch(/const activate = \(cap: Capability, variant: CapabilityVariant\)/)
    expect(src, "换后端复用 activate（无第二套启用路径）").toMatch(
      /const switchBackend = \(cap: Capability, variant: CapabilityVariant\) => \{[\s\S]{0,160}activate\(cap, variant\)/,
    )
    expect(src, "要装新包/要拆旧档 → 先确认").toMatch(/willReplace \|\| variant\.state === "off"/)
  })

  it("后果面不持**能力级开关**，但可就地换档（2026-09-21 裁定变更）", () => {
    // v4 曾定"右栏纯只读、换后端只走左栏溢出菜单"。真机上用户看到三张并列卡却点不动，
    // 只能回左栏找一个图标-only 的入口——选项就在眼前却不能选，是反直觉的。
    // 现改为：右栏对照面**就地可换档**，但仍**不持能力级开关**（避免出现两个开关）。
    // 防护意图不变：一个能力只能有一个"开/关"，换档不是开关。
    const pane = paneRegion()
    expect(pane, "右栏不得再有开关（能力级开/关仍只在左栏）").not.toContain("<Switch")
    expect(pane, "仍不引入单选组（换了另一套等价控件也是双入口）").not.toContain("radiogroup")
    expect(pane, "后端对照仍由 variantPackageRoles 派生").toContain("variantPackageRoles(cap, v.id)")
    // 换档入口：当前生效档不给按钮（点了也是无操作）
    expect(pane, "非生效档给「切换到此档」").toContain("t.market.capSwitchToThis")
    expect(pane, "生效档/被挡住的档不给按钮").toMatch(/\{!isActive && !vBlocked && !v\.subsumedBy &&/)
    expect(pane, "恢复/危险区入口仍在（busy 互斥）").toContain("disabled={busy}")
    // 与左栏菜单**同一条链**（不是第二套启用路径）
    expect(src, "卡片换档复用 switchBackend").toContain("onSwitchBackend={(v) => switchBackend(cap, v)}")
  })

  it("换档只有一条链：activate（右栏与左栏菜单共用）", () => {
    expect(src, "switchBackend 收口到 activate").toMatch(
      /const switchBackend = \(cap: Capability, variant: CapabilityVariant\) => \{[\s\S]{0,160}activate\(cap, variant\)/,
    )
  })

  it("汇总计数用等宽数字（number-tabular：状态变化时行不跳）", () => {
    expect(src, "汇总计数要 tabular-nums").toContain("tabular-nums")
  })
})

describe("⑨ 描述性信息只在 ⓘ 里（2026-09-21 三次修订）", () => {
  // 维护者口径原文："实验功能这个模块提供唯一的对实验性插件的操作入口，希望减少
  // 描述性信息，若必须要则收敛到小 tip icon 里面去。" —— 本段钉的就是"操作面不许
  // 再被说明文字占位"，同时反转钉住"收敛 ≠ 删掉"（信息必须仍然可达）。
  it("模块与能力两级说明都收进 ⓘ，不再平铺成正文", () => {
    expect(src, "模块说明挂标题旁的 ⓘ").toMatch(/<Tip text=\{t\.market\.capDesc\}/)
    expect(src, "能力说明（简介 + 开启后）合成的单条悬浮").toContain(
      "capTipText(cap.summary, cap.unlocks)",
    )
    expect(src, "后端说明挂同名 ⓘ").toContain("text={v.note}")
    // 反转证：旧形态（正文段落）不许回来
    expect(paneRegion(), "能力简介不得再平铺成段落").not.toMatch(/<p[^>]*>\{cap\.summary\}/)
    expect(paneRegion(), "后端说明不得再平铺成块").not.toMatch(
      /<span className="mt-1 block[^>]*>\{v\.note\}/,
    )
    // 收敛 ≠ 删掉：两块文本都必须仍然可达（否则本条会靠"功能被删"假绿）
    expect(src, "简介仍可达").toContain("cap.summary")
    expect(src, "开启后说明仍可达").toContain("cap.unlocks")
  })

  it("展开体不再重复卡片头部的身份（同一张卡不说两遍）", () => {
    // 身份块（图标 + 能力名 + 状态徽标 + 简介段落）原先在展开体里又摆了一遍，
    // 而卡片头部已经写着同样四样。整块删除后，展开体只留"要动手才需要"的内容。
    const pane = paneRegion()
    expect(pane, "不得再挂能力名标题").not.toContain("<h3")
    expect(pane, "不得再挂状态徽标（头部独家）").not.toContain("<StateBadge")
    expect(pane, "不得再挂能力图标（头部独家）").not.toContain("capabilityIcon(")
    expect(src, "头部仍是身份的唯一出处").toContain("{cap.label}")
  })

  it("「互斥，切换即让位」这条规则可键盘可达（原先是 title 属性）", () => {
    // `title` 只有鼠标够得着，读屏与键盘用户拿不到——而它是"选后端之前就该知道"的规则。
    expect(src, "规则走 ⓘ").toMatch(/text=\{t\.market\.capBackendExclusiveNote\}/)
    expect(paneRegion(), "不得再退回 title 属性").not.toMatch(
      /title=\{multi \? t\.market\.capBackendExclusiveNote/,
    )
  })
})
