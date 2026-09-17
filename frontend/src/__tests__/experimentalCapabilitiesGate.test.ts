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

import { describe, expect, it } from "vitest"

import src from "@/components/market/ExperimentalCapabilities.tsx?raw"
import mock from "@/lib/devMock.ts?raw"

/** 左栏**清单行**函数体（"不点任何东西就能扫到的东西"都在这里）。 */
function listRegion(): string {
  return slice("function CapabilityRow(", "function CapabilityPane(")
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

  it("两栏布局：清单是左栏，详情面是右栏，窄窗口退化为下钻", () => {
    // 结构调整 = 判据跟着走：v2 让详情内联展开；v3 要求两者并排（详情面常驻）。
    // 两栏 = 「一条有上下限的轨 + 一条自适应轨」。只钉 `lg:grid-cols-[minmax(` 是不够的：
    // 改成 `lg:grid-cols-[minmax(0,1fr)]`（退回纵向堆叠）照样命中，四条断言全绿
    // ——2026-09-17 独立复核给出的正是这个"绿着退化"的反例。
    expect(src, "必须是两栏栅格（左轨有宽度区间 + 右轨自适应）").toMatch(
      /lg:grid-cols-\[minmax\([^\]_]*\)\s*_\s*minmax\(0,1fr\)\]/,
    )
    const grid = src.indexOf('className="grid items-start gap-4')
    expect(grid, "找不到两栏栅格容器").toBeGreaterThan(-1)
    expect(src.indexOf("t.market.capListLabel"), "清单必须在两栏容器之后（左栏）").toBeGreaterThan(
      grid,
    )
    expect(src.indexOf("id={PANE_ID}"), "详情面必须在两栏容器之后（右栏）").toBeGreaterThan(grid)
    expect(src, "窄窗口（<1024）要有下钻 + 返回出口").toContain("capBack")
    expect(src, "窄窗口下钻态：清单与详情面互斥显示").toMatch(/drilled \? "hidden lg:block" : ""/)
  })

  it("行是**单行紧凑**结构（不给整块段落留位置）", () => {
    const row = listRegion()
    // 行里出现块级段落 = 行会重新变高。行只允许 span / 按钮 / 开关。
    expect(row, "清单行里出现了块级段落").not.toMatch(/<p[\s>]/)
    // 一个能力一行（而不是"一档一行"）：变体清单归详情面。
    expect(row, "清单行不得渲染完整变体清单").not.toContain("cap.variants.map(")
    expect(row, "长描述归详情面").not.toContain("cap.summaryZh")
    // 插件名**完整显示（折行）**，不截断：官方包名动辄 50+ 字符，截断截掉的正好是
    // 区分各档的那一段（`…cua-driver-mcp` 与 `…cua-driver-native` 会截成同一个前缀）。
    // 能力名可以截断（短且同一份数据），**插件名不行**：它是唯一"要对得上 node_modules"
    // 的字符串，截掉的正好是区分各档的尾部（`…cua-driver-mcp` / `…cua-driver-native`
    // 会截成同一个前缀）。所以钉住插件名那一行的类名。
    expect(row, "插件名要完整折行显示").toContain("break-words font-mono")
  })

  it("清单行保住必须「扫一眼就有」的四件事：名称 / 状态 / 插件名 / 开关", () => {
    const row = listRegion()
    for (const required of ["cap.labelZh", "StateBadge", "variantDisplayName(cap,", "Switch"]) {
      expect(row, `清单行缺少「${required}」`).toContain(required)
    }
  })
})

describe("① 实现细节只能在详情面，不得进清单行", () => {
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

  it("官方来源与官方简介文案齐备（面板与逐包两处）", () => {
    expect(src, "面板头缺官方标记").toContain("t.market.capOfficialBadge")
    expect(src, "面板头缺官方来源说明").toContain("t.market.capOfficialNote")
    expect(src, "详情面缺逐包官方简介标签").toContain("t.market.capOfficialDesc")
    expect(src, "未装时必须如实说明（不得用我们的文案冒充官方简介）").toContain(
      "t.market.capOfficialDescPending",
    )
  })

  it("详情面**确实**承载这些实现细节（否则上一条是假绿）", () => {
    const pane = paneRegion()
    for (const required of [
      "s.package",
      "capPinned",
      "capImplTitle",
      "s.rowId",
      "s.description",
      "cap.summaryZh",
      "cap.unlocksZh",
    ]) {
      expect(pane, `详情面缺少「${required}」`).toContain(required)
    }
  })

  it("用户视角的信息必须在清单行或详情面上（价值 / 前置 / 状态）", () => {
    expect(paneRegion(), "详情面缺少价值描述").toContain("cap.unlocksZh")
    expect(paneRegion(), "详情面缺少前置").toContain("prerequisitesZh")
    for (const required of ["cap.labelZh", "Switch"]) {
      expect(listRegion(), `清单行缺少「${required}」`).toContain(required)
    }
  })
})

describe("①b 行内状态徽标按**变体自己**的状态取词（2026-09-17）", () => {
  it("activeVariant 也覆盖「已就位但停用」，因此变体行徽标不得无条件写「已启用」", () => {
    // 后端契约：`official_catalog.rs` 的 Disabled 分支把 activeVariant 设成那条"包里齐了、
    // 行被停用"的变体。所以变体行徽标必须按 v.state 分档——版面复盘时在 mock 数据上
    // 真抓到"面板说已停用 / 行内说已启用"的自相矛盾。
    const pane = paneRegion()
    expect(pane).toContain('v.state === "on"')
    expect(pane).toContain('v.state === "disabled"')
    expect(pane).toContain("t.market.capStateDisabled")
    expect(pane, "两档之外要有兜底").toContain("t.market.capStatePartial")
    expect(pane, "单档能力不必在变体行里重复面板头已说过的状态").toContain("multi && isActive")
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

  it("清单行有「当前选中」的可访问状态（选中态不能只靠颜色）", () => {
    const row = listRegion()
    expect(row, "选中行必须有 aria-current").toContain('aria-current={selected ? "true"')
    expect(row, "行要指向它控制的详情面").toContain("aria-controls={PANE_ID}")
    expect(src, "详情面要有对应的 id").toContain("id={PANE_ID}")
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
      /!subsumedBy[\s\S]{0,60}!blocked[\s\S]{0,60}!failure[\s\S]{0,40}variant\.state === "partial"/,
    )
    expect(dock, "被包含的档不得提供独立移除入口").toMatch(/!subsumedBy &&\s*\n?\s*\(variant\.state === "on"/)
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
    expect(src, "不得再照选中档渲染徽标").not.toMatch(/const shown = variant\.state/)
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
    expect(src).toMatch(/const blocked = Boolean\(variant\.prerequisiteMissing\)/)
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
      /!subsumedBy[\s\S]{0,60}!blocked[\s\S]{0,60}!failure[\s\S]{0,40}variant\.state === "partial"/,
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

  it("动作一发起就切到该能力的详情面（分栏形态下，「就地呈现」= 选中它）", () => {
    // v2 靠"卡片就地展开"，v3 没有展开这回事——所以必须把详情面切过去，否则进度与失败
    // 会出现在用户没在看的另一栏里（改版时最容易漏的一条衔接）。
    expect(src).toMatch(/setSelectedId\(cap\.id\)[\s\S]{0,120}setDrilled\(true\)/)
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
    expect(src, "行与详情面都要收到全局 busy").toMatch(/busy=\{run !== null\}/)
    expect((src.match(/busy=\{run !== null\}/g) ?? []).length, "两栏都要传（行 + 详情面）").toBeGreaterThanOrEqual(2)
    expect(src, "在途时不许换档（换档会让在途动作的语义搅乱）").toMatch(
      /<Select value=\{profile\} onValueChange=\{setProfile\} disabled=\{run !== null\}>/,
    )
  })

  it("在途动作的回读不许覆盖新档的清单（请求令牌）", () => {
    // 失败路径：A 在途 → 切到 B → B 的 load 先写 caps → A 的 finally 用旧 profile 回读覆盖面
    // → 下拉显示 B、清单是 A 的事实，之后按 A 的 installed 算计划装进 B。
    expect(src).toContain("loadSeq")
    expect(src, "只接受最后一次请求").toMatch(/if \(seq === loadSeq\.current\) setCaps\(next\)/)
  })

  it("换档清状态时连 dirty 一起清（否则「立即重启」打在没改过的档上）", () => {
    expect(src).toMatch(/setDirty\(false\)\s*\n\s*void load\(\)/)
  })

  it("dev mock 与契约同形（tsc 兜底，不再数出现次数）", () => {
    expect(mock, "mock 的载荷要按契约标注（少字段编译期就红）").toContain(
      "] satisfies Capability[]",
    )
  })
})
