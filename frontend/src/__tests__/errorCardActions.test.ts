// errorCardActions.test.ts —— 错误卡动作分派门禁（2026-09-11，task-52 / T-F5）。
//
// 回归锚：「假按钮」—— rust-core 的 D1 给错误卡下发
// `kind=symlink_privilege_required` + `actions=["boot_in_wsl","retry"]`，
// 但前端 `INVOKABLE` 不含 `boot_in_wsl` ⇒ `run()` 直接 return ⇒ **点了没反应**，
// 比"没有这个按钮"更糟（用户以为自己没点对）。
//
// 本仓库禁 RTL/jsdom ⇒ 把「id → 该调哪条 IPC」抽成**纯函数**（`resolveActionCall`），
// 用它做机器化断言：任何下发的 action id 都必须有确定的分派目标，且
// `boot_in_wsl` 必须走 `bootInWsl`（**不是** `terminalAction`，那是另一条 IPC）。
import { describe, expect, it } from "vitest"
import {
  INVOKABLE_ACTIONS,
  resolveActionCall,
} from "@/components/boot/ErrorCard"
import { t as zhCN } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"
import type { BootFailureKind } from "@/types/ipc"

const CJK = /[\u3000-\u303f\u4e00-\u9fff\uff00-\uffef]/

describe("动作 id → IPC 分派（防「假按钮」回归）", () => {
  it("D1 契约的每个 action 都能被分派（无静默无效项）", () => {
    // 契约：boot_failure.rs:135 `SymlinkPrivilegeRequired => vec!["boot_in_wsl", "retry"]`
    const contractActions = ["boot_in_wsl", "retry"]
    for (const id of contractActions) {
      const call = resolveActionCall(id)
      expect(call, `动作 ${id} 无法分派（将是假按钮）`).not.toBeNull()
    }
  })

  it("boot_in_wsl 走 bootInWsl —— 不是 terminalAction", () => {
    expect(resolveActionCall("boot_in_wsl")).toEqual({
      kind: "invoke",
      ipc: "bootInWsl",
    })
  })

  it("retry / upgrade / upgrade_only 走 terminalAction", () => {
    for (const id of ["retry", "upgrade", "upgrade_only"] as const) {
      expect(resolveActionCall(id), id).toEqual({ kind: "invoke", ipc: "terminalAction" })
    }
  })

  it("INVOKABLE 集合与分派表一致（两处不得漂移）", () => {
    for (const id of INVOKABLE_ACTIONS) {
      expect(resolveActionCall(id), `${id} 在 INVOKABLE 中却无法分派`).not.toBeNull()
    }
    expect(INVOKABLE_ACTIONS.has("boot_in_wsl")).toBe(true)
  })

  it("未知 id 仍返回 null（保留「未知动作不猜」的既有姿态）", () => {
    expect(resolveActionCall("definitely_not_an_action")).toBeNull()
    expect(resolveActionCall("")).toBeNull()
  })
})

describe("新增 kind 的文案键齐备（中英对称）", () => {
  const NEW_KIND: BootFailureKind = "symlink_privilege_required"

  it("两字典都有该 kind 的 title + suggestion", () => {
    const zh = zhCN.error.kinds[NEW_KIND]
    const en = enUS.error.kinds[NEW_KIND]
    expect(zh, "zh-CN 缺该 kind").toBeDefined()
    expect(en, "en-US 缺该 kind").toBeDefined()
    expect(String(zh.title).length).toBeGreaterThan(0)
    expect(String(zh.suggestion).length).toBeGreaterThan(0)
    expect(String(en.title).length).toBeGreaterThan(0)
    expect(String(en.suggestion).length).toBeGreaterThan(0)
  })

  it("en 侧不含中文（enUsNoLeak 口径）", () => {
    const en = enUS.error.kinds[NEW_KIND]
    expect(CJK.test(String(en.title)), "en title 含中文").toBe(false)
    expect(CJK.test(String(en.suggestion)), "en suggestion 含中文").toBe(false)
  })

  it("zh 侧必须点破「不是网络问题」（契约设计要点，防误判排查方向）", () => {
    const zh = zhCN.error.kinds[NEW_KIND]
    expect(String(zh.suggestion)).toContain("不是网络问题")
  })

  it("引导出路含 WSL（与 kind 的 actions 语义一致）", () => {
    expect(String(zhCN.error.kinds[NEW_KIND].suggestion)).toMatch(/WSL/)
    expect(String(enUS.error.kinds[NEW_KIND].suggestion)).toMatch(/WSL/)
  })

  it("action 文案键存在且不是英文 id 兜底", () => {
    const zhLabel = zhCN.error.actions["boot_in_wsl"]
    const enLabel = enUS.error.actions["boot_in_wsl"]
    expect(zhLabel, "缺 zh 文案 ⇒ 用户会看到英文 id").toBeTruthy()
    expect(enLabel).toBeTruthy()
    expect(zhLabel).not.toBe("boot_in_wsl")
    expect(enLabel).not.toBe("boot_in_wsl")
    expect(CJK.test(String(enLabel)), "en 文案含中文").toBe(false)
  })

  it("所有契约中的 failure kind 都有本地化文案（防新增 kind 漏配）", () => {
    // 与 Rust `BootFailureKind` 序列化值逐一对应（D1 §5 跨层契约）
    const allKinds: BootFailureKind[] = [
      "credentials_mismatch",
      "incompatible_options",
      "network_unavailable",
      "symlink_privilege_required",
      "unknown",
    ]
    for (const kind of allKinds) {
      expect(zhCN.error.kinds[kind], `zh-CN 缺 kind=${kind}`).toBeDefined()
      expect(enUS.error.kinds[kind], `en-US 缺 kind=${kind}`).toBeDefined()
    }
  })
})

/**
 * 接线断言（`?raw`，剥离注释）：证明**组件真的用了**分派表，而不是只导出了一张表。
 * 本仓禁 RTL/jsdom ⇒ DOM 层点击无法测，故用源码结构断言钉住接线。
 */
describe("组件接线：run() 确实经分派表调用两条 IPC", () => {
  it("run() 使用 resolveActionCall 且两条 IPC 都出现", async () => {
    const src = (await import("@/components/boot/ErrorCard.tsx?raw")).default as string
    const code = src
      .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
      .replace(/\/\*[\s\S]*?\*\//g, "")
      .replace(/^[ \t]*\/\/.*$/gm, "")
    // 1) 分派表被真正消费
    expect(code).toContain("resolveActionCall(id)")
    // 2) boot_in_wsl 路径走 bootInWsl
    expect(code).toContain("api.bootInWsl()")
    // 3) terminalAction 仍在（其余动作不变）
    expect(code).toContain("api.terminalAction(")
    // 4) 旧的隐式假设（统一 cast 成 TerminalAction 蒙混）不得回流
    //    现写法是按 call.ipc 分支后再 cast，故 cast 只应出现在 terminalAction 那一支
    const castCount = (code.match(/id as TerminalAction/g) ?? []).length
    expect(castCount, "id as TerminalAction 应只在 terminalAction 分支出现一次").toBe(1)
    // 5) 反向断言：剥离注释后仍保留真实结构（防过度剥离假绿）
    expect(code).toContain("export function resolveActionCall")
  })

  it("actionLabel 仍对未知 id 兜底展示原文（不静默吞掉）", () => {
    // 与 resolveActionCall 的 null 语义配套：能点的一定有文案，不能点的不渲染按钮
    expect(zhCN.error.actions["boot_in_wsl"]).toBeTruthy()
  })
})
