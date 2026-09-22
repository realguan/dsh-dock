// 沉浸式标题栏纯模型测试（ADR-0029）。
//
// 钉住四件容易在重构中悄悄坏掉的事：
//   1. **平台门**——只有 macOS 补 `data-platform` 标记；Win/Linux 一律 null
//      （一期不做半成品沉浸式，原生装饰不动）；
//   2. **拖拽语义 fidelity**——`drag`→`deep`、`no-drag`→`false` 必须逐条映射：
//      dsh 对可拖区域里的非可点击子元素（div）显式 no-drag，漏映射 = 按钮区
//      被拖拽"漏"进去；
//   3. **origin 判定**——精确（IPC 已回）说了算；未知时同步 127.0.0.1 乐观命中；
//      壳页 hostname（localhost / tauri.localhost）绝不误判；
//   4. **降级方向**——任何"不确定"都落在"不动作"或"撤销"上，绝不在壳页生效。
import { describe, expect, it } from "vitest"
import {
  DSH_PLATFORM_MARKER,
  dragRegionAttrFor,
  immersivePlatformAttrFor,
  isWorkbenchHostnameSync,
  syncWorkbenchProbe,
  WINDOWS_TITLEBAR_HEIGHT,
  windowsTitlebarPlanFor,
} from "@/lib/immersiveChrome"

describe("immersivePlatformAttrFor", () => {
  it("macOS → darwin 标记（dsh 官方 preload 同款值）", () => {
    expect(immersivePlatformAttrFor("macos")).toBe(DSH_PLATFORM_MARKER)
    expect(DSH_PLATFORM_MARKER).toBe("darwin")
  })

  it("Windows/Linux → null（一期保持原生装饰，ADR-0029 §4）", () => {
    expect(immersivePlatformAttrFor("windows")).toBeNull()
    expect(immersivePlatformAttrFor("linux")).toBeNull()
    expect(immersivePlatformAttrFor(undefined)).toBeNull()
  })
})

describe("dragRegionAttrFor", () => {
  it("drag → deep（Tauri 子树拖拽；可点击子元素由 drag.js 自动阻断）", () => {
    expect(dragRegionAttrFor("drag")).toBe("deep")
    expect(dragRegionAttrFor(" drag ")).toBe("deep")
  })

  it("no-drag → false（显式排除，防 deep 把按钮区拖进去）", () => {
    expect(dragRegionAttrFor("no-drag")).toBe("false")
  })

  it("无 app-region / 异常值 → null（不碰该元素）", () => {
    expect(dragRegionAttrFor("")).toBeNull()
    expect(dragRegionAttrFor("auto")).toBeNull()
    expect(dragRegionAttrFor("none")).toBeNull()
  })
})

describe("isWorkbenchHostnameSync", () => {
  it("127.0.0.1 命中（dsh 就绪 URL 恒该形态，shell.rs 实测）", () => {
    expect(isWorkbenchHostnameSync("127.0.0.1")).toBe(true)
  })

  it("壳页 hostname 绝不误判", () => {
    // macOS 壳页 tauri://localhost（hostname 恰为 localhost）
    expect(isWorkbenchHostnameSync("localhost")).toBe(false)
    // Windows 壳页 http://tauri.localhost
    expect(isWorkbenchHostnameSync("tauri.localhost")).toBe(false)
    expect(isWorkbenchHostnameSync("::1")).toBe(false)
  })
})

describe("syncWorkbenchProbe", () => {
  const WB = "http://127.0.0.1:34567/"

  it("IPC 已回且 origin 精确相符 → confirm", () => {
    expect(syncWorkbenchProbe("http://127.0.0.1:34567", "127.0.0.1", WB)).toBe("confirm")
  })

  it("IPC 已回但不相符 → reject（调用方须撤标记）", () => {
    expect(syncWorkbenchProbe("http://127.0.0.1:9999", "127.0.0.1", WB)).toBe("reject")
    expect(syncWorkbenchProbe("https://example.com", "example.com", WB)).toBe("reject")
  })

  it("IPC 已回但 URL 畸形 → reject（不猜）", () => {
    expect(syncWorkbenchProbe("http://127.0.0.1:1", "127.0.0.1", "not-a-url")).toBe("reject")
  })

  it("IPC 未回 + 同步命中 127.0.0.1 → confirm（抢先画，免首帧闪侧栏）", () => {
    expect(syncWorkbenchProbe("http://127.0.0.1:34567", "127.0.0.1", null)).toBe("confirm")
  })

  it("IPC 未回 + 非 127.0.0.1 → defer（等 IPC，壳页场景）", () => {
    expect(syncWorkbenchProbe("tauri://localhost", "localhost", null)).toBe("defer")
    expect(syncWorkbenchProbe("http://tauri.localhost", "tauri.localhost", null)).toBe("defer")
    expect(syncWorkbenchProbe("http://localhost:1420", "localhost", null)).toBe("defer")
  })

  it("IPC 未回但壳页 hostname 不得乐观命中（降级方向的安全性）", () => {
    // localhost 既是 macOS 壳页 hostname 也是 dev 服务器 hostname——判据只认
    // 127.0.0.1 就是为把这两者挡在门外；此处回归锁定。
    for (const origin of ["tauri://localhost", "http://tauri.localhost", "http://localhost:1420"]) {
      expect(syncWorkbenchProbe(origin, new URL(origin).hostname, null)).toBe("defer")
    }
  })
})

describe("Windows 标题栏标记（对标官方 preload-windows.ts）", () => {
  it("Windows ⇒ 打 data-windows-titlebar 并给 40px 高度变量（与官方常量一致）", () => {
    const plan = windowsTitlebarPlanFor("windows")
    expect(plan).not.toBeNull()
    expect(plan!.attr).toBe("data-windows-titlebar")
    expect(plan!.cssVar).toBe("--dsh-windows-titlebar-height")
    expect(plan!.cssValue).toBe("40px")
    expect(WINDOWS_TITLEBAR_HEIGHT).toBe(40)
  })

  it("反例守卫：仅 Windows 生效 —— macOS 走 darwin 标记、Linux 两者都不打", () => {
    expect(windowsTitlebarPlanFor("macos")).toBeNull()
    expect(windowsTitlebarPlanFor("linux")).toBeNull()
    expect(windowsTitlebarPlanFor(undefined)).toBeNull()
    expect(immersivePlatformAttrFor("windows")).toBeNull()
    expect(immersivePlatformAttrFor("macos")).toBe("darwin")
  })
})
