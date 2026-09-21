// unmaterializedProfileGate.test.ts —— 「未物化 profile」闸门（2026-09-21 真机 bug）。
//
// ## 复现
// 左栏选中 `headless`（内置模板名，目录还没创建），右栏：
//   · 「插件列表」tab 显示的是**人话**（详情读取失败 → Rust 给的"尚未物化"说明）；
//   · 「实验能力」tab 却把 `listExperimentalCapabilities` 的原始 OS 错误直接铺出来：
//     `读取 profile 清单失败（headless）：No such file or directory (os error 2)`。
// 同一件事两种口径，且第二种是给工程师看的——用户看到的是"界面坏了"。
//
// ## 真因
// 未物化 = 目录不存在 ⇒ 详情 / 插件清单 / 行表 / 能力目录**四个读取全部必失败**。
// 面板原先不区分"未物化"与"读取失败"，四条链照样发、各自把失败写进各自的状态位，
// 于是同一件事报了两次（琥珀横幅 + 红色错误盒），其中一次还是原始错误串。
//
// ## 判据
// ① 面板拿得到 `materialized`（与左栏虚线框同一个标记，禁双源）；
// ② 未物化时**一条请求都不发**（不是"发了再把错误藏起来"）；
// ③ 四个 tab 共用**一句人话 + 唯一出路**（启动一次即物化），不给每个 tab 各报一次；
// ④ 物化后（标记 false→true）自动回到正常取数。
// 纯逻辑测试：只读源码文本，不渲染 DOM（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"

import paneSrc from "@/components/profiles/ProfileDetailPane.tsx?raw"
import managerSrc from "@/pages/ProfileManager.tsx?raw"
import zhDictSrc from "@/content/zh-CN.ts?raw"
import enDictSrc from "@/content/en-US.ts?raw"

describe("未物化 profile：不发请求、只给一句人话", () => {
  it("物化标记由父级传入（与左栏虚线框同一个标记）", () => {
    // 面板自己推不出来（它只能看到"读取失败"），必须消费 `list_profiles` 的同一枚标记，
    // 否则又会出现"左栏说可首启、右栏说读取失败"的双源打架。
    expect(paneSrc, "面板收 materialized prop").toMatch(/materialized: boolean/)
    expect(managerSrc, "父级传入选中档的物化标记").toContain(
      "materialized={currentSelectedProfile?.materialized ?? false}",
    )
  })

  it("未物化时两条取数效应都早退（不是发了再藏错误）", () => {
    // 详情/清单/行表在同一个 reload 里，能力目录是另一条 effect —— 两条都要挡住。
    expect(paneSrc, "主取数效应按物化标记早退").toMatch(
      /if \(!materialized\) return\s*\n\s*reload\(\)/,
    )
    expect(paneSrc, "能力目录效应同样早退").toContain(
      "if (!name || !materialized) return",
    )
    // 反转证：早退必须在 reload/loadCaps **之前**（写在后面等于没挡）
    const effectAt = paneSrc.indexOf("if (!materialized) return")
    const reloadAt = paneSrc.indexOf("    reload()\n  }, [name, materialized, reload])")
    expect(effectAt, "找不到早退点").toBeGreaterThan(-1)
    expect(reloadAt, "找不到 reload 调用").toBeGreaterThan(effectAt)
  })

  it("标记翻转（启动一次即物化）后自动回到正常取数", () => {
    // 依赖数组少写 materialized，用户启动完 profile 还得手动刷新才看得到内容。
    expect(paneSrc, "主效应依赖物化标记").toContain("[name, materialized, reload]")
    expect(paneSrc, "目录效应依赖物化标记").toContain("[name, materialized, loadCaps]")
  })

  it("四个 tab 共用一句人话 + 唯一出路，而不是各报一次错", () => {
    expect(paneSrc, "主体按物化分支").toMatch(/\{!materialized \? \(/)
    expect(paneSrc, "给出人话标题").toContain("t.profiles.notMaterializedTitle(")
    expect(paneSrc, "给出解释与出路").toContain("t.profiles.notMaterializedBody")
    expect(paneSrc, "唯一动作 = 启动（启动即物化）").toMatch(
      /onClick=\{onLaunch\} disabled=\{busy\}/,
    )
    // 页头不再谎报"正在加载配置档案…"（那份档案永远不会到）
    expect(paneSrc, "页头给「未物化」标").toContain("t.profiles.notMaterializedTag")
    expect(paneSrc, "未物化时不显示加载态文案").toMatch(
      /\) : materialized \? \(\s*\n\s*<span className="text-faint text-micro">\{t\.profiles\.detailLoading\}/,
    )
  })

  it("文案齐备且 zh/en 对称", () => {
    for (const [name, src] of [
      ["zh-CN", zhDictSrc],
      ["en-US", enDictSrc],
    ] as const) {
      expect(src, `${name} 缺标题键`).toContain("notMaterializedTitle:")
      expect(src, `${name} 缺说明键`).toContain("notMaterializedBody:")
      expect(src, `${name} 缺页头标键`).toContain("notMaterializedTag:")
    }
    // 出路必须写清楚（"首次启动或首次添加插件后才会创建"），否则用户不知道该干嘛
    expect(zhDictSrc, "zh 写明物化时机").toContain("首次启动或首次添加插件后才会创建")
  })
})
