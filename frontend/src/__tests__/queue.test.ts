// 安装队列纯逻辑测试（095 #4 / ADR-0011 队列形态）。Vitest 只测纯函数
// （AGENTS §4.4）：串行选择 / 结果两态迁移 / 角标计数；编排见 stores/queueStore。
// 2026-09-09（ADR-0013）：构建脚本改默认批准，审批门相位（blocked_gate）退役。
import { describe, expect, it } from "vitest"
import { activeCount, applyOutcome, nextQueued, outcomeOf, type QueueItem } from "@/lib/queue"

const item = (over: Partial<QueueItem> = {}): QueueItem => ({
  id: "q-1",
  kind: "install",
  pkg: "dsh-pet",
  spec: "github:o/r#path:/packages/dsh-pet",
  profile: "web",
  status: "queued",
  createdAt: 1,
  ...over,
})

describe("nextQueued（串行：取最早入队的待处理项）", () => {
  it("跳过非 queued 项取第一个排队项", () => {
    const items = [
      item({ id: "a", status: "done" }),
      item({ id: "b", status: "failed" }),
      item({ id: "c" }),
      item({ id: "d" }),
    ]
    expect(nextQueued(items)?.id).toBe("c")
  })

  it("空队列返回 undefined", () => {
    expect(nextQueued([])).toBeUndefined()
  })
})

describe("applyOutcome（安装结果两态迁移）", () => {
  it("ok → done 且清空 detail", () => {
    const got = applyOutcome(item({ status: "installing", detail: "旧错" }), { ok: true })
    expect(got.status).toBe("done")
    expect(got.detail).toBeUndefined()
  })

  it("失败 → failed 带 detail", () => {
    const got = applyOutcome(item({ status: "installing" }), { ok: false, detail: "x" })
    expect(got.status).toBe("failed")
    expect(got.detail).toBe("x")
  })

  it("失败无 detail → failed 且 detail 为空", () => {
    const got = applyOutcome(item(), { ok: false })
    expect(got.status).toBe("failed")
    expect(got.detail).toBeUndefined()
  })
})

describe("activeCount（面板角标：未终结项）", () => {
  it("queued/installing 计入，done/failed 不计", () => {
    const items = [
      item({ id: "a", status: "queued" }),
      item({ id: "b", status: "installing" }),
      item({ id: "c", status: "done" }),
      item({ id: "d", status: "failed" }),
    ]
    expect(activeCount(items)).toBe(2)
  })
})

// 2026-09-15（R2）：`enqueueAndWait` 的解析值由这里决定——可 await 契约的判据
// 落在这个纯函数上，编排（谁何时 resolve）在 stores/queueStore。
describe("outcomeOf（终态 → 可 await 结果）", () => {
  it("done → ok:true 且不带 detail", () => {
    expect(outcomeOf(item({ status: "done", detail: "陈旧错误" }))).toEqual({ ok: true })
  })

  it("failed → ok:false 带 detail", () => {
    expect(outcomeOf(item({ status: "failed", detail: "pnpm 失败" }))).toEqual({
      ok: false,
      detail: "pnpm 失败",
    })
  })

  // 防误用：未终结项不是「成功」。调用方若在终结前调用，必须拿到 ok:false
  // 而不是被静默当成成功——否则目录编排会跳过失败步骤继续下发。
  it("未终结（queued/installing）→ ok:false，不得当成成功", () => {
    expect(outcomeOf(item({ status: "queued" })).ok).toBe(false)
    expect(outcomeOf(item({ status: "installing" })).ok).toBe(false)
  })

  it("remove 终态同样两态成立（与 kind 无关）", () => {
    expect(outcomeOf(item({ kind: "remove", status: "done" })).ok).toBe(true)
    expect(outcomeOf(item({ kind: "remove", status: "failed", detail: "x" })).ok).toBe(false)
  })
})
