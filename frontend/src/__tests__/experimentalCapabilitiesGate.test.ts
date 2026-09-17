// experimentalCapabilitiesGate.test.ts —— 实验能力开关面板的**源码闸门**（2026-09-16）。
//
// 纯逻辑测试不渲染 DOM（AGENTS §4.3 末段），所以这里用 `?raw` + 窗口切片钉住四件
// 会**悄悄退化**的事——它们都是 v1「官方实验室」的真实缺陷形态：
//
//  ① **第一阅读层被实现细节污染**：钉版本 / 激活方式 / 行 id 必须只在「详情」
//     折叠区里出现（v1 把这些直接排在卡片第一屏）。反向也要钉：折叠区**必须**真有它们，
//     否则第 ① 条会靠"整个功能被删掉"而假绿。
//     **2026-09-17 修订**：维护者裁定「实验功能模块要突出是 dsh 官方实验功能，插件的**名字**
//     和描述也要以官方为主」——官方名就是 npm 包名，遂把**当前后端的官方包名**移到第一阅读层
//     （用官方标识 `capOfficialPackages` 标注）。这是裁定对 v1 判据的**有意放宽**，不是退化：
//     仍然禁止 spec（钉版本）、激活方式、行 id、版本提示出现在折叠态，并改用更强的窗口判据
//     （`variant.steps` 的官方包名行之外不得再出现任何逐包实现字段）。
//  ② **破坏性移除不确认**：`移除并卸载` 必须经 `ConfirmDialog`（U9 口径），
//     且按钮只**置起待确认态**，不得直接执行。
//  ③ **关闭 ≠ 卸载**：只有 `toggleOffSupported` 的变体才允许走行级停用；
//     含 profile 层的变体必须走移除（层的副作用回滚不了）。
//  ④ **开关必须有无障碍名称**（`Switch` 的类型闸门之外，再钉一次调用点确实传了）。

import { describe, expect, it } from "vitest"

import src from "@/components/market/ExperimentalCapabilities.tsx?raw"

/** 卡片函数体（第一阅读层的实现都在这里）。 */
function cardBody(): string {
  const start = src.indexOf("function CapabilityCard(")
  expect(start, "找不到 CapabilityCard：本闸门的窗口切片依赖它").toBeGreaterThan(-1)
  const end = src.indexOf("function StateBadge(")
  expect(end, "找不到 StateBadge：本闸门的窗口切片依赖它").toBeGreaterThan(start)
  return src.slice(start, end)
}

/** 卡片**折叠态**（`{openDetail &&` 之前）——用户不点「详情」时能看到的全部。 */
function collapsedRegion(): string {
  const body = cardBody()
  const marker = body.indexOf("{openDetail &&")
  expect(marker, "找不到 openDetail 折叠标记").toBeGreaterThan(-1)
  return body.slice(0, marker)
}

/** 去掉行注释与块注释（组件里的中文注释不该被"无内联中文"这条误伤）。 */
function stripComments(text: string): string {
  return text
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .split("\n")
    .filter((line) => !line.trim().startsWith("//"))
    .join("\n")
}

describe("① 第一阅读层不得出现实现细节（官方包名除外，2026-09-17 裁定）", () => {
  it("折叠态里没有钉版本、激活方式与行 id", () => {
    const collapsed = collapsedRegion()
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
      expect(collapsed, `第一阅读层出现了「${forbidden}」`).not.toContain(forbidden)
    }
  })

  it("插件名（官方名）在第一阅读层，且是行首标签（2026-09-17 裁定：名字用插件名）", () => {
    const collapsed = collapsedRegion()
    expect(collapsed, "第一阅读层缺少「插件」节标").toContain("t.market.capPluginsLabel")
    // 名字必须来自**派生规则**（标识包 = 该档独有；基座包另标共用），而不是我们发明的
    // 「Web 档 / 复用已装的 cua-driver」这类标签——`variantPackageRoles` 有单测钉住取值。
    expect(collapsed, "插件名必须由 variantPackageRoles 派生").toContain(
      "variantPackageRoles(cap, v.id)",
    )
    expect(collapsed, "行首必须渲染标识包（roles.primary）").toContain("roles.primary.map(")
    expect(collapsed, "共用基座包要如实标注，不得省略").toContain("capAlsoInstalls")
  })

  it("官方来源与官方简介文案齐备（面板与逐包两处）", () => {
    expect(src, "面板头缺官方标记").toContain("t.market.capOfficialBadge")
    expect(src, "面板头缺官方来源说明").toContain("t.market.capOfficialNote")
    expect(src, "详情缺逐包官方简介标签").toContain("t.market.capOfficialDesc")
    expect(src, "未装时必须如实说明（不得用我们的文案冒充官方简介）").toContain(
      "t.market.capOfficialDescPending",
    )
  })

  it("详情折叠区**确实**承载这些实现细节（否则上一条是假绿）", () => {
    const body = cardBody()
    const detail = body.slice(body.indexOf("{openDetail &&"))
    for (const required of ["s.package", "capPinned", "capImplTitle", "s.rowId", "s.description"]) {
      expect(detail, `折叠区缺少「${required}」`).toContain(required)
    }
  })

  it("用户视角的信息必须在第一阅读层（价值 / 前置 / 状态）", () => {
    const collapsed = collapsedRegion()
    for (const required of ["cap.summaryZh", "prerequisitesZh", "cap.labelZh", "Switch"]) {
      expect(collapsed, `第一阅读层缺少「${required}」`).toContain(required)
    }
  })
})

describe("①b 行内状态徽标按**变体自己**的状态取词（2026-09-17）", () => {
  it("activeVariant 也覆盖「已就位但停用」，因此行内徽标不得无条件写「已启用」", () => {
    // 后端契约：`official_catalog.rs` 的 Disabled 分支把 activeVariant 设成那条"包里齐了、
    // 行被停用"的变体。所以行内徽标必须按 v.state 分档——版面复盘时在 mock 数据上
    // 真抓到"卡片说已停用 / 行内说已启用"的自相矛盾。
    const collapsed = collapsedRegion()
    expect(collapsed).toContain('v.state === "on"')
    expect(collapsed).toContain('v.state === "disabled"')
    expect(collapsed).toContain("t.market.capStateDisabled")
    expect(collapsed, "两档之外要有兜底").toContain("t.market.capStatePartial")
    expect(collapsed, "单档能力不必在行内重复卡片头已说过的状态").toContain("multi && isActive")
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
    // 卡片的 onRemove 回调不得调用 planRemove。
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

  it("组件内没有内联中文文案（一律走 t.market.* 字典）", () => {
    const han = stripComments(src).match(/[\u4e00-\u9fff]/g) ?? []
    expect(han, `组件里有内联中文：${han.join("")}`).toEqual([])
  })
})

describe("⑤ 子集档（Agent Teams 的两档）不得被当成互斥后端", () => {
  it("`subsumedBy` 存在时开关被禁用、不给修复/移除（否则会拆掉超集档的基础层）", () => {
    // 真机暴露（2026-09-16）：自建档 = [agent-team-profile]，Web 档 = [同一包, web 层]。
    // 装 Web 档时子集档的包必然也齐 —— 那不是"两个后端并列"，报"冲突"是误报。
    const dock = src
    expect(dock).toContain("subsumedBy")
    expect(dock, "开关必须因被包含而禁用").toMatch(
      /disabled=\{busy \|\| subsumedBy !== null( \|\| blocked !== null)?\}/,
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

describe("⑦b 卡片徽标与 chip 标记（2026-09-16 用户真机验收第二轮）", () => {
  it("徽标说能力的实话，不照「当前选中的后端」渲染", () => {
    // 真机：桌面控制里「随包自带运行时」正在生效、用户点了被前置门挡住的
    // 「复用已装 cua-driver」，旧写法按选中档渲染 → 整张卡报「需要修复」。
    expect(src).toMatch(/const shown: keyof typeof map = cap\.state/)
    expect(src, "不得再照选中档渲染徽标").not.toMatch(/const shown = variant\.state/)
  })

  it("被前置门挡住的档在**它自己那一行**里有标记（否则切档看不出区别）", () => {
    // 2026-09-17 重做后，"档"以插件行呈现，被挡的原因落在该行内（仍是第一阅读层）。
    expect(src).toMatch(/const vBlocked = v\.prerequisiteMissing/)
    expect(src, "原因必须渲染出来").toMatch(/\{vBlocked && \(/)
  })
})

describe("⑦ 宿主前置缺失 = 硬门（2026-09-16 真机事故）", () => {
  it("前置缺失时开关禁用，且把后端给的原因**露在卡面上**（不是藏在折叠详情里）", () => {
    // 事故：缺 cua-driver 时装上 cua-driver-mcp → dsh 插件树加载失败 → 工作台起不来。
    // 所以这不是"提示"，是门：禁用开关 + 说明为什么灰。
    expect(src).toMatch(/const blocked = variant\.prerequisiteMissing/)
    expect(src, "前置缺失必须并入 Switch 的 disabled").toMatch(
      /disabled=\{busy \|\| subsumedBy !== null \|\| blocked !== null\}/,
    )
    // 原因必须在**第一阅读层**（不展开「详情」就能看到），且用警示色——原样展示后端文案
    // （禁前端自造）。2026-09-17 重做后它落在被挡那一行内：span + text-danger。
    const collapsed = collapsedRegion()
    expect(collapsed, "前置缺失的原因必须在第一阅读层").toContain("vBlocked")
    expect(collapsed, "原因必须用警示色").toMatch(/text-danger[\s\S]{0,240}?\{vBlocked\}/)
  })
})

describe("⑥ 失败态与状态衔接：不许说假话、不许两个按钮干一件事", () => {
  it("「关闭即移除」只在真就位（on/disabled）时才说——装一半就说会让用户以为自己是层类能力", () => {
    // 真机 2026-09-16：一次失败的安装把卡片变成 partial，而 toggleOffSupported 对
    // 没装齐的档也是 false（分类要读包自己的 manifest）→ 卡片谎称"本档由 profile 层提供"。
    expect(src).toMatch(
      /const offIsRemove =\s*\n?\s*!variant\.toggleOffSupported &&\s*\n?\s*\(variant\.state === "on" \|\| variant\.state === "disabled"\)/,
    )
  })

  it("有失败块时不再渲染「修复」——它与「继续剩余步骤」是同一个动作", () => {
    expect(src).toMatch(
      /!subsumedBy[\s\S]{0,60}!blocked[\s\S]{0,60}!failure[\s\S]{0,40}variant\.state === "partial"/,
    )
  })

  it("失败先给一句人话，原始输出折叠在后面（不把 200 字符的 pnpm 输出铺在卡面上）", () => {
    // 分类由**后端**给（`plugin_registry::classify_failure`）→ 前端只按 kind 选文案，
    // 不得再写第二份正则（否则两份分类会漂移：后端换源了、前端却说不是网络问题）。
    expect(src).toContain("failureKind")
    expect(src, "前端不得自带失败正则分类").not.toMatch(/tls handshake|failed to fetch metadata/)
    expect(src).toContain("capFailNetwork")
    expect(src).toContain("capFailNotFound")
    expect(src).toContain("capFailBuildApproval")
    expect(src, "原始输出必须可折叠").toMatch(/aria-expanded=\{showRaw\}/)
    // 原始输出不得再直接渲染在卡片第一层（现在只在展开后出现）。
    expect(src).toMatch(/showRaw && \(/)
  })
})
