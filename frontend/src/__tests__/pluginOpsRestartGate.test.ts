// pluginOpsRestartGate.test.ts —— 插件操作批次（2026-09-24）源码闸门。
//
// 覆盖四件事的「不许回头」判据（tsc/oxlint 看不到的那类）：
//   ① dsh 升级后自动重启当前 profile——判据纯函数 + DshVersionCard 的三处接线；
//   ② 市场排序两个面同源、默认同为下载量（弹窗原先没有排序）；
//   ③ 添加插件-插件市场的安装 loading 从队列派生（撤掉 300ms 假完成）；
//   ④ 「重启后生效」提示全仓唯一面——旧 capRestart* 键回收、两个渲染点共用
//      同一个组件、动作走统一出口、确认链注册在 ProfileManager。
//
// 纯源码文本断言，不渲染 DOM（AGENTS §4.3 末段：Vitest 只测纯逻辑；与
// marketCategorySourceGate / onNoticeStability 同口径）。
import { describe, expect, it } from "vitest"

import addDialogSrc from "@/components/profiles/PluginAddDialog.tsx?raw"
import marketViewSrc from "@/components/market/MarketplaceView.tsx?raw"
import dshCardSrc from "@/components/about/DshVersionCard.tsx?raw"
import paneSrc from "@/components/profiles/ProfileDetailPane.tsx?raw"
import hubSrc from "@/components/market/PluginHub.tsx?raw"
import managerSrc from "@/pages/ProfileManager.tsx?raw"
import restartHintSrc from "@/components/ui/restart-hint.tsx?raw"
import zhDictSrc from "@/content/zh-CN.ts?raw"
import enDictSrc from "@/content/en-US.ts?raw"

describe("① dsh 升级后自动重启当前 profile", () => {
  it("判据用纯函数（不内联在事件回调里）", () => {
    expect(dshCardSrc, "升级 done 分支调纯判据").toContain(
      "shouldRestartAfterUpgrade(payload)",
    )
  })

  it("两道闸门接线齐备：取活跃会话 + switch_profile（Rust 自判 Restart）", () => {
    // 链式调用跨行是 oxlint 的既有排版，判据用容忍空白的正则
    expect(dshCardSrc, "取当前活跃会话").toMatch(/api\s*\.getActiveProfile\(\)/)
    expect(dshCardSrc, "交给 switch_profile").toMatch(/api\s*\.switchProfile\(active\)/)
  })

  it("无活跃会话时不得空跑 switch_profile（如实说明下次启动生效）", () => {
    expect(dshCardSrc, "None 分支走「无活跃会话」文案").toContain(
      "t.about.upgradeRestartNoActive",
    )
  })

  it("事件名与载荷类型收敛到 types/events（不再内联魔法字符串）", () => {
    expect(dshCardSrc).toContain("EV.dshUpgrade")
    expect(dshCardSrc).toContain("listen<DshUpgradeEvent>")
  })
})

describe("② 市场排序：两个面同源、默认同为下载量", () => {
  it("排序只有一份实现（lib/market 纯函数），两个面都调它", () => {
    expect(marketViewSrc, "插件中心").toContain("sortMarketPlugins(filteredPlugins, sortOption)")
    expect(addDialogSrc, "添加插件弹窗").toContain("sortMarketPlugins(list, sortOption)")
  })

  it("默认值两处一致 = downloads（2026-09-24 维护者裁定）", () => {
    expect(marketViewSrc).toContain('useState<MarketSortOption>("downloads")')
    expect(addDialogSrc).toContain('useState<MarketSortOption>("downloads")')
  })

  it("弹窗用插件中心同一批排序文案键（不得自造第二套）", () => {
    for (const key of ["sortDownloads", "sortStars", "sortNewest", "sortName"]) {
      expect(addDialogSrc, `弹窗缺 ${key}`).toContain(`t.market.${key}`)
    }
  })
})

describe("③ 安装 loading：从队列派生，撤掉假完成", () => {
  it("busy 判据来自队列（会话级单一真相源，含排队中相位）", () => {
    expect(addDialogSrc, "订阅队列项").toContain("useQueueStore((s) => s.items)")
    expect(addDialogSrc, "按 包名 + 目标档 匹配").toMatch(
      /i\.pkg === p\.name &&\s*i\.profile === target/,
    )
    expect(addDialogSrc, "queued/installing 都算 loading").toMatch(
      /i\.status === "queued" \|\| i\.status === "installing"/,
    )
  })

  it("假 loading 三件套不得回来", () => {
    expect(addDialogSrc, "不得再有局部 busy 态").not.toContain("installingMarketPkg")
    expect(addDialogSrc, "不得再有定时器假装完成").not.toContain("setTimeout")
    // 入队/成功/失败三条 toast 由队列统一发；弹窗再发一条就是双弹
    expect(zhDictSrc, "入队通知键已随假 loading 退役").not.toContain("pluginAddMarketInstallDone")
    expect(enDictSrc, "入队通知键已随假 loading 退役").not.toContain("pluginAddMarketInstallDone")
  })

  it("父页按「终结在本档」刷新（弹窗关了也翻）", () => {
    expect(paneSrc, "订阅按档终结信号").toContain("useQueueStore((s) => s.lastFinishedProfile)")
    expect(paneSrc, "只认终结在选中档").toContain("queueFinishedProfile !== name")
    expect(paneSrc, "终结即刷新详情与能力目录").toContain("loadCaps(name)")
  })
})

describe("④ 重启提示唯一面（一种逻辑一种交互）", () => {
  it("旧 capRestart* 键两字典均已回收（迁至中立 restart section）", () => {
    expect(zhDictSrc).not.toContain("capRestartHint:")
    expect(zhDictSrc).not.toContain("capRestartNow:")
    expect(enDictSrc).not.toContain("capRestartHint:")
    expect(enDictSrc).not.toContain("capRestartNow:")
  })

  it("中立 restart section 两字典齐备且对称", () => {
    for (const [name, src] of [
      ["zh-CN", zhDictSrc],
      ["en-US", enDictSrc],
    ] as const) {
      expect(src, `${name} 缺 neededHint`).toContain("neededHint:")
      expect(src, `${name} 缺 nowBtn`).toContain("nowBtn:")
    }
  })

  it("两个渲染点共用同一个组件（详情页选中档 + 插件中心逐档）", () => {
    expect(paneSrc, "详情页按选中档渲染").toContain("<RestartNeededHint profile={name} />")
    expect(hubSrc, "插件中心逐档渲染").toContain("<RestartNeededHint key={p} profile={p} />")
    expect(hubSrc, "插件中心读 pending 集合").toContain("useRestartNeededStore((s) => s.pending)")
  })

  it("组件动作走统一出口；确认链注册在 ProfileManager", () => {
    expect(restartHintSrc, "组件只调统一出口").toContain("requestRestartProfile(profile)")
    expect(restartHintSrc, "不得自行 invoke switch_profile").not.toContain("switchProfile")
    expect(managerSrc, "宿主注册确认链（与左栏行重启同链）").toContain(
      "setRestartProfileHandler(handleRestart)",
    )
  })

  it("详情页不再经 props 转发重启（onRestart 链已撤）", () => {
    expect(paneSrc).not.toContain("onRestart")
    expect(hubSrc).not.toContain("onRestart")
  })

  it("档没了/改名了，标记跟着挪或撤（不留死横幅）", () => {
    expect(managerSrc, "删除即摘除").toMatch(/clear\(deleteTarget\)/)
    expect(managerSrc, "重命名把标记从旧名挪到新名（配置还在，只是换了名字）").toMatch(
      /clear\(nameOp\.source\)[\s\S]{0,80}mark\(newName\)/,
    )
  })
})
