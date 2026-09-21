// pluginToggleState.test.ts —— 插件开关「配置侧 / 运行侧」两个真相源的门禁
// （2026-09-17，维护者实机报「刚打开插件却还显示停用，切标签页回来才变运行中」）。
//
// 事故链（实机复现并测量）：
//   ① 行内有两个近乎同义的词——配置侧徽标「已禁用」与运行侧徽标「已停用」——
//      且「已停用」还被刷成 ok 绿，用户无法判断"是开关没生效，还是插件没装好"；
//   ② 开关写成功只重取**行表**（配置），运行态快照停在进页面那一刻 ⇒ 同一行出现
//      "开关是开的 / 徽标说停用"的自相矛盾，切标签页重挂才自愈；
//   ③ 开关标签/提示把「重启后生效」写死，而 web 形态 profile 的
//      `dsh.profile.patchReload = live`，克隆实机实测改文件后 0.43s 内 fiber 注销、
//      0.44s 内重建 ⇒ 承诺与事实相反。
// 本门禁钉住：运行侧徽标按真相源分种类、开关写完后要短轮询到落定、文案不得再写死重启。
// 纯逻辑测试：只读源码文本与字典常量，不引 RTL/jsdom（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"
import { t as zhCN } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"
import paneSrc from "@/components/profiles/ProfileDetailPane.tsx?raw"
import rowSrc from "@/components/profiles/pluginRows/PluginListRow.tsx?raw"

/** 结构断言剥离注释（注释里会写"为什么"，含中文，属正常）。 */
function stripComments(src: string): string {
  return src
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^[ \t]*\/\/.*$/gm, "")
}

const pane = stripComments(paneSrc)
/** 行内渲染区域：2026-09-21 结构收敛后行组件独立成文件（判据不变，落点跟着搬）。 */
const row = stripComments(rowSrc)

describe("插件开关：运行态必须在写完后自己追平（不再靠切页自愈）", () => {
  it("写完后立刻重取运行态，并短轮询到落定", () => {
    expect(pane, "开关写完后必须重取运行态快照").toContain("refreshRuntime(profile)")
    expect(pane, "落定判据走纯函数（可单测）").toContain("splitSettledToggles(")
    expect(pane, "轮询要有次数上限").toMatch(/const TOGGLE_SETTLE_ATTEMPTS = \d+/)
    expect(pane, "轮询要有间隔上限").toMatch(/const TOGGLE_SETTLE_INTERVAL_MS = \d+/)
  })

  it("落定前该行显示「生效中」而不是留着旧徽标", () => {
    expect(pane).toContain("setPending({ ...pendingRef.current, [pkg]: wantEnabled })")
    expect(row).toContain("t.profiles.chip.applying")
    expect(row).toContain("t.profiles.chipHint.applying")
  })

  it("多行连点不互相挤掉结论（待定集合而非单个 ref）", () => {
    expect(pane).toContain("pendingToggles")
    expect(pane).toContain("pendingRef.current")
    expect(pane, "已落定行逐条报结论").toContain("t.profiles.toggleApplied(p, pending[p])")
  })

  it("定时器随组件卸载收掉（切页/切 profile 不留残轮询）", () => {
    expect(pane).toContain("window.clearTimeout(settleTimer.current)")
    expect(pane, "卸载清理 effect").toMatch(/useEffect\(\s*\(\) => \(\) => \{[\s\S]*?clearTimeout/)
  })

  it("行表重取失败不再静默（旧实现 catch(()=>{}) 会留下假状态）", () => {
    expect(pane).toMatch(/getPluginRows\(profile\)[\s\S]{0,220}?catch\(\(e\) => onNotice\(String\(e\), "warn"\)\)/)
  })

  it("两个真相源的徽标分别带上自解释 title", () => {
    expect(row).toContain("t.profiles.chipHint[chip.kind]")
    expect(row).toContain("t.profiles.pluginDisabledHint")
    expect(row, "配色由 chipTone 按种类给").toContain("chipTone(chip)")
  })

  // 2026-09-17 独立复核抓到的 P0：写面用 `disabled` 口径、文案/判据用 `enabled` 口径，
  // 旧实现把两者当同一个 `next` 用 ⇒ toast 动词与"是否已生效"全反。
  // 文本门禁抓不到实参极性，只能靠"口径各自具名 + 极性纯函数单测"钉死。
  it("极性护栏：写 patch 用 writeDisabled，文案与落定判据用 wantEnabled", () => {
    expect(pane).toContain("toggleIntent(row)")
    expect(pane).toMatch(/setPluginDisabled\(profile, id, writeDisabled\)/)
    expect(pane, "落定结算走纯函数（内部按 wantEnabled 比 enabled）").toContain(
      "splitSettledToggles(pending, entries)",
    )
    expect(pane).toContain("t.profiles.toggleRestart(pkg, wantEnabled)")
    expect(pane).toContain("t.profiles.toggleDone(pkg, wantEnabled)")
    expect(pane, "已生效的回报在落定结算里").toContain("toggleApplied(p, pending[p])")
    expect(pane, "禁止再用裸 next 当两种口径").not.toMatch(
      /toggle(Applied|Restart|Done)\(pkg, next\)|setPluginDisabled\(profile, id, next\)/,
    )
    expect(pane).not.toMatch(/setPluginDisabled\(profile, id, wantEnabled\)/)
  })

  it("落定判定用本次取回的快照，不读渲染闭包里的旧 runtime", () => {
    expect(pane).toContain("appliesToProfile")
    expect(pane, "查询失败与会话没在跑必须分开（失败不许报「需要重启」）").toMatch(
      /!observed[\s\S]{0,160}?toggleDone/,
    )
    expect(pane, "启动后补取一次运行态（面板不重挂）").toMatch(/prevRunning\.current = isRunning/)
  })
})

describe("插件开关：文案不得再对生效时机许下相反的承诺", () => {
  it("开关标签只讲动作，不带「重启后生效」", () => {
    expect(zhCN.profiles.pluginEnable).toBe("启用")
    expect(zhCN.profiles.pluginDisable).toBe("禁用")
    expect(enUS.profiles.pluginEnable).toBe("Enable")
    expect(enUS.profiles.pluginDisable).toBe("Disable")
    expect(pane, "组件里不得硬编码重启文案").not.toContain("重启该 Profile 后生效")
  })

  it("生效与否由观测结果回报：已生效 / 重启后生效两句话都在字典", () => {
    expect(zhCN.profiles.toggleApplied("pkg-x", true)).toContain("已生效")
    expect(zhCN.profiles.toggleRestart("pkg-x", true)).toContain("重启")
    expect(enUS.profiles.toggleApplied("pkg-x", true)).toContain("applied")
    expect(enUS.profiles.toggleRestart("pkg-x", true)).toContain("restart")
    expect(pane).toContain("toggleApplied(p, pending[p])")
    expect(pane).toContain("t.profiles.toggleRestart(pkg, wantEnabled)")
  })

  it("观测不到运行态的行（补丁包）不承诺时机，只报「配置已写入」", () => {
    // 补丁包的贡献行用各自的 name 成条目，包名本身不成条目 ⇒ 轮询必然超时，
    // 若照旧报「重启后生效」就是对 live profile 说了假话。
    expect(zhCN.profiles.toggleDone("pkg-x", true)).toContain("配置已写入")
    expect(zhCN.profiles.toggleDone("pkg-x", true)).not.toContain("重启")
    expect(enUS.profiles.toggleDone("pkg-x", true)).toContain("config written")
    expect(pane).toMatch(/runtimeChipFor\(pkg, entries\) === null[\s\S]{0,220}?toggleDone/)
  })

  it("运行侧徽标的两个「没到位」状态用不同词，且都不叫「已停用」", () => {
    const { chip } = zhCN.profiles
    expect(chip.unloaded).not.toBe(chip.notApplied)
    expect(chip.unloaded).not.toBe(chip.active)
    // 与配置侧徽标「已禁用」也不再撞词/近义
    for (const label of [chip.unloaded, chip.notApplied, chip.applying]) {
      expect(label).not.toBe(zhCN.profiles.pluginDisabled)
    }
    expect(new Set(Object.values(chip)).size).toBe(6)
    expect(Object.keys(zhCN.profiles.chipHint).sort()).toEqual(Object.keys(chip).sort())
  })

  it("en 侧同键齐备（漏一处即红）", () => {
    expect(Object.keys(enUS.profiles.chip).sort()).toEqual(Object.keys(zhCN.profiles.chip).sort())
    expect(Object.keys(enUS.profiles.chipHint).sort()).toEqual(
      Object.keys(zhCN.profiles.chipHint).sort(),
    )
    expect(enUS.profiles.pluginToggleHint.length).toBeGreaterThan(20)
    expect(enUS.profiles.pluginDisabledHint.length).toBeGreaterThan(20)
  })
})
