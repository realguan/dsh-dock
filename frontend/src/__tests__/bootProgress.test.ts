// 下载速度采样纯函数测试（frontend-migration §8.1 bootProgress）。
import { describe, expect, it } from "vitest"
import { computeSpeed, pushSample } from "@/stores/bootStore"

describe("pushSample（6s 滑动窗裁剪）", () => {
  it("窗口内样本全部保留", () => {
    const s: { t: number; bytes: number }[] = []
    pushSample(s, 1000, 0)
    pushSample(s, 2000, 10)
    pushSample(s, 3000, 20)
    expect(s).toHaveLength(3)
  })
  it("超过 6s 的旧样本被裁剪（含恰好 6000ms 边界保留）", () => {
    const s: { t: number; bytes: number }[] = []
    pushSample(s, 1000, 0)
    pushSample(s, 7000, 100) // 恰好 6s：保留
    expect(s).toHaveLength(2)
    pushSample(s, 7001, 100) // 采样点 1000 距 now 7000→7001 差 6001ms：裁掉
    expect(s).toHaveLength(2)
    expect(s[0].t).toBe(7000)
  })
  it("空数组追加首样本", () => {
    const s: { t: number; bytes: number }[] = []
    pushSample(s, 500, 42)
    expect(s[0]).toEqual({ t: 500, bytes: 42 })
  })
})

describe("computeSpeed", () => {
  it("不足 2 个样本不出速度", () => {
    expect(computeSpeed([])).toBeNull()
    expect(computeSpeed([{ t: 0, bytes: 0 }])).toBeNull()
  })
  it("窗口平均速度计算（bytes/s）", () => {
    expect(
      computeSpeed([
        { t: 1000, bytes: 0 },
        { t: 3000, bytes: 2048 },
      ]),
    ).toBe(1024)
  })
  it("时间零增量返回 null", () => {
    expect(
      computeSpeed([
        { t: 1000, bytes: 0 },
        { t: 1000, bytes: 100 },
      ]),
    ).toBeNull()
  })
  it("减速（字节回退）不崩，按代数值计算", () => {
    expect(
      computeSpeed([
        { t: 1000, bytes: 2048 },
        { t: 2000, bytes: 1024 },
      ]),
    ).toBe(-1024)
  })
})
