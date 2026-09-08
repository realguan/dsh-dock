// destructiveConfirmGate.test.ts —— 破坏性操作确认的源码闸门（2026-09-08，UI/UX 评审 U9）。
//
// 复现的缺陷：卸载插件 / 覆写 .credentials.yaml / 覆写 settings.yaml 三个**不可撤销**
// 动作没有任何确认，误点即执行；另有 3 处用原生 `window.confirm`，样式与风险说明
// 都与 `ProfileDeleteDialog` 的模态不一致。收口到 `ui/confirm-dialog.tsx` 后，本闸门
// 钉住两条：① 全仓不得再出现原生 `window.confirm`；② 6 个破坏性入口必须引用
// ConfirmDialog（漏掉任一处即红——回归形态是「有人新写了一个破坏性按钮」）。
import { describe, expect, it } from "vitest"

const RAW_SOURCES = import.meta.glob("../**/*.{ts,tsx}", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>

/** 必须走 ConfirmDialog 的破坏性入口（相对 src/ 的路径后缀）。 */
const DESTRUCTIVE_ENTRIES = [
  "/components/profiles/ProfileDeleteDialog.tsx", // 删除 profile
  "/components/profiles/ProfileDetailPane.tsx", // 卸载插件
  "/components/profiles/SessionManager.tsx", // 删除会话
  "/components/profiles/McpManager.tsx", // 移除 MCP 服务
  "/components/system/CredentialsPane.tsx", // 清除 Key / 覆写凭据
  "/components/system/DshSettingsPane.tsx", // 覆写引擎设置
]

describe("破坏性操作确认闸门", () => {
  it("全仓不再使用原生 window.confirm（统一走 ConfirmDialog）", () => {
    const offenders = Object.entries(RAW_SOURCES)
      .filter(([, src]) => /\bwindow\s*\.\s*confirm\s*\(/.test(src))
      .map(([path]) => path)

    expect(
      offenders,
      "这些文件仍用原生 window.confirm：无法列风险要点、样式与模态不一致",
    ).toEqual([])
  })

  it("6 个破坏性入口都引用了 ConfirmDialog", () => {
    const missing = DESTRUCTIVE_ENTRIES.filter((suffix) => {
      const hit = Object.entries(RAW_SOURCES).find(([path]) => path.endsWith(suffix))
      return !hit || !/ConfirmDialog/.test(hit[1])
    })

    expect(missing, "这些破坏性入口没有接 ConfirmDialog").toEqual([])
  })

  it("闸门自身有效：能识别两种写法", () => {
    expect(/\bwindow\s*\.\s*confirm\s*\(/.test("if (!window.confirm(x)) return")).toBe(true)
    expect(/\bwindow\s*\.\s*confirm\s*\(/.test("if (!window . confirm(x)) return")).toBe(true)
    expect(/\bwindow\s*\.\s*confirm\s*\(/.test("const confirmed = true")).toBe(false)
  })

  it("闸门自身有效：入口清单路径确实存在", () => {
    const paths = Object.keys(RAW_SOURCES)
    for (const suffix of DESTRUCTIVE_ENTRIES) {
      expect(
        paths.some((p) => p.endsWith(suffix)),
        `清单里的 ${suffix} 在 glob 结果里找不到（清单过期？）`,
      ).toBe(true)
    }
  })
})
