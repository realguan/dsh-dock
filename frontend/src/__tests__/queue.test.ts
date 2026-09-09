// 安装队列纯逻辑测试（095 #4 / ADR-0011 队列形态）。Vitest 只测纯函数
// （AGENTS §4.4）：串行选择 / 结果三态迁移 / 角标计数；编排见 stores/queueStore。
import { describe, expect, it } from "vitest"
import { activeCount, applyOutcome, nextQueued, type QueueItem } from "@/lib/queue"

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
      item({ id: "b", status: "blocked_gate" }),
      item({ id: "c" }),
      item({ id: "d" }),
    ]
    expect(nextQueued(items)?.id).toBe("c")
  })

  it("空队列返回 undefined", () => {
    expect(nextQueued([])).toBeUndefined()
  })
})

describe("applyOutcome（安装结果三态迁移）", () => {
  it("ok → done，清空审批残留", () => {
    const got = applyOutcome(item({ status: "installing", gatePkgs: ["ssh2"] }), {
      ok: true,
    })
    expect(got.status).toBe("done")
    expect(got.gatePkgs).toBeUndefined()
  })

  it("ignored_builds 非空 → blocked_gate 并记录键", () => {
    const got = applyOutcome(item({ status: "installing" }), {
      ok: false,
      ignored_builds: ["@scope/x@https://codeload.example/x#path:/p"],
    })
    expect(got.status).toBe("blocked_gate")
    expect(got.gatePkgs).toEqual(["@scope/x@https://codeload.example/x#path:/p"])
  })

  it("普通失败 → failed 带 detail；空 ignored_builds 不算门槛", () => {
    expect(applyOutcome(item(), { ok: false, detail: "x" }).status).toBe("failed")
    expect(applyOutcome(item(), { ok: false, ignored_builds: [] }).status).toBe("failed")
  })
})

describe("activeCount（面板角标：未终结项）", () => {
  it("queued/installing/blocked_gate 计入，done/failed 不计", () => {
    const items = [
      item({ id: "a", status: "queued" }),
      item({ id: "b", status: "installing" }),
      item({ id: "c", status: "blocked_gate" }),
      item({ id: "d", status: "done" }),
      item({ id: "e", status: "failed" }),
    ]
    expect(activeCount(items)).toBe(3)
  })
})
