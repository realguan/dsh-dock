// restartHintGate.test.ts —— 待重启提示的**保留档分支**（2026-10-14 复核，ADR-0034 收尾）。
//
// 背景：ADR-0034 放开保留档（desktop）的内容面之后，该档也会有"配置已变更"的待重启
// 标记（插件装卸/启停、实验能力、MCP 都能改了）。但它的**生命周期面是关闭的**——壳里
// 没有重启入口，`requestRestartProfile` 只会被拒。若照旧渲染「立即重启」按钮，用户点
// 下去永远得到一句拒绝：那是"点不动的假入口"（红线三：不给假实现）。
//
// 口头事实：该档的"重启"是**官方桌面客户端下次启动**。故本组件在保留档上只留一句话，
// 后端 `plugins::classify_op_outcome` 的成功文案同口径（同一个事实不能有两种说法）。
// 纯源码文本测试：不渲染 DOM（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"

import hintSrc from "@/components/ui/restart-hint.tsx?raw"
import paneSrc from "@/components/profiles/ProfileDetailPane.tsx?raw"
import hubSrc from "@/components/market/PluginHub.tsx?raw"
import zhDictSrc from "@/content/zh-CN.ts?raw"
import enDictSrc from "@/content/en-US.ts?raw"

describe("待重启提示：保留档只给说法，不给点不动的重启按钮", () => {
  it("保留档走专用文案，判据用前端唯一镜像（不新增第二份名单）", () => {
    expect(hintSrc, "保留档专用文案").toContain("t.restart.reservedNeededHint")
    expect(hintSrc, "判据走 lib/profiles 的镜像").toContain("isReservedProfileName(profile)")
    // 镜像函数本身仍在（别只改用法把名单删了）
    expect(hintSrc, "镜像 import 在场").toContain('from "@/lib/profiles"')
  })

  it("两个调用点都走同一处自门控（别在别处再写一份分支）", () => {
    expect(paneSrc, "详情页挂提示（自门控，不判空）").toContain(
      "RestartNeededHint profile={name}",
    )
    expect(hubSrc, "插件中心逐档挂提示").toContain("RestartNeededHint key={p} profile={p}")
    // 反转证：两个调用点都不自己判 reserved（判据只该在组件里）
    expect(hubSrc, "插件中心不自己判保留档").not.toContain("isReservedProfileName")
    expect(paneSrc, "详情页不自己判保留档").not.toContain("isReservedProfileName")
  })

  it("常规档的重启入口与原文案都还在（别把功能一起删了）", () => {
    expect(hintSrc, "常规档保留重启按钮").toContain("t.restart.nowBtn")
    expect(hintSrc, "常规档保留原文案").toContain("t.restart.neededHint")
    expect(hintSrc, "常规档保留动作链路").toContain("requestRestartProfile(profile)")
  })

  it("文案 zh/en 对称", () => {
    for (const [name, src] of [
      ["zh-CN", zhDictSrc],
      ["en-US", enDictSrc],
    ] as const) {
      expect(src, `${name} 缺保留档待重启文案`).toContain("reservedNeededHint:")
      expect(src, `${name} 保留档文案不得承诺壳内重启`).not.toMatch(
        /reservedNeededHint: *"[^"]*重启该 Profile/,
      )
    }
  })
})
