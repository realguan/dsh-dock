// 升级提示条纯逻辑测试（ADR-0010 升级呈现）：出条判定、键契约、忽略记忆。
import { describe, expect, it } from "vitest"
import { bannerKey, updateBanners } from "@/lib/updateBanner"
import type { UpdateStatus } from "@/types/ipc"

function status(overrides: Partial<UpdateStatus> = {}): UpdateStatus {
  return {
    dsh: { current: "1.5.0", latest: null, newer: false, error: null },
    client: { current: "0.9.4", latest: null, newer: false, error: null },
    node: null,
    ...overrides,
  }
}

describe("lib/updateBanner.ts", () => {
  it("no snapshot → no banners", () => {
    expect(updateBanners(null, null)).toEqual([])
  })

  it("emits dsh and client banners in order when newer", () => {
    const s = status({
      dsh: { current: "1.5.0", latest: "1.6.0", newer: true, error: null },
      client: { current: "0.9.4", latest: "0.9.5", newer: true, error: null },
    })
    expect(updateBanners(s, null).map((b) => b.key)).toEqual([
      "dsh@1.6.0",
      "client@0.9.5",
    ])
  })

  it("not-newer / missing-latest / error dimensions emit nothing", () => {
    const s = status({
      dsh: { current: "1.6.0", latest: "1.6.0", newer: false, error: null },
      client: { current: "0.9.4", latest: null, newer: false, error: "更新源不可达" },
    })
    expect(updateBanners(s, null)).toEqual([])
  })

  it("dismissed key suppresses the same version only", () => {
    const s = status({
      dsh: { current: "1.5.0", latest: "1.6.0", newer: true, error: null },
      client: { current: "0.9.4", latest: "0.9.5", newer: true, error: null },
    })
    expect(updateBanners(s, "dsh@1.6.0").map((b) => b.key)).toEqual(["client@0.9.5"])
    // 新版本键不受旧忽略影响
    expect(updateBanners(s, "dsh@1.5.9").map((b) => b.key)).toEqual([
      "dsh@1.6.0",
      "client@0.9.5",
    ])
  })

  it("bannerKey shape matches settings.rs dismissed_update contract", () => {
    expect(bannerKey("dsh", "1.6.0")).toBe("dsh@1.6.0")
    expect(bannerKey("client", "0.9.5")).toBe("client@0.9.5")
  })
})
