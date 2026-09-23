// 沉浸式标题栏纯模型测试（ADR-0029）。
//
// 钉住四件容易在重构中悄悄坏掉的事：
//   1. **平台门**——只有 macOS 补 `data-platform` 标记；Win/Linux 一律 null
//      （Windows 档 2026-09-22 进场、2026-09-23 裁定整体撤回，ADR-0030/issue #16：
//      `decorations(false)` 带走缩放与贴靠、自绘控件无 ACL 授权、拖拽条是伪元素；
//      Windows 自此与 Linux 同口径维持原生装饰）；
//   2. **拖拽几何语义（2026-09-23 改版）**——`dragDecisionFor` 必须只在
//      「带内 ∧ 在 #root 内 ∧ 不在交互元素上 ∧ 非全屏」时才接管拖拽：
//      多一个条件放松 = 按钮点不动或正文选不中，少一个条件 = 窗口拖不动；
//   3. **origin 判定**——精确（IPC 已回）说了算；未知时同步 127.0.0.1 乐观命中；
//      壳页 hostname（localhost / tauri.localhost）绝不误判；
//   4. **降级方向**——任何"不确定"都落在"不动作"或"撤销"上，绝不在壳页生效。
import { describe, expect, it } from "vitest"
import {
  DRAG_BAND_FALLBACK_HEIGHT,
  DRAG_EXCLUSION_SELECTOR,
  DRAG_SURFACE_ROOT_SELECTOR,
  DSH_PLATFORM_MARKER,
  dragDecisionFor,
  immersivePlatformAttrFor,
  isWorkbenchHostnameSync,
  LEADING_BAND_SELECTOR,
  syncWorkbenchProbe,
} from "@/lib/immersiveChrome"

describe("immersivePlatformAttrFor", () => {
  it("macOS → darwin 标记（dsh 官方 preload 同款值）", () => {
    expect(immersivePlatformAttrFor("macos")).toBe(DSH_PLATFORM_MARKER)
    expect(DSH_PLATFORM_MARKER).toBe("darwin")
  })

  it("Windows/Linux → null（维持原生装饰；Windows 档已按 2026-09-23 裁定撤回，ADR-0030）", () => {
    expect(immersivePlatformAttrFor("windows")).toBeNull()
    expect(immersivePlatformAttrFor("linux")).toBeNull()
    expect(immersivePlatformAttrFor(undefined)).toBeNull()
  })
})

describe("dragDecisionFor（几何语义：注入层自驱窗口拖拽）", () => {
  const plainBandPress = {
    inBand: true,
    insideRoot: true,
    onInteractive: false,
    fullscreen: false,
  }

  it("带内空白 + #root 内 + 非全屏 ⇒ drag", () => {
    expect(dragDecisionFor(plainBandPress)).toBe("drag")
  })

  it("落在 dsh 的交互元素排除表上 ⇒ skip（按钮/链接/tab 必须照常可点）", () => {
    expect(dragDecisionFor({ ...plainBandPress, onInteractive: true })).toBe("skip")
  })

  it("带外 ⇒ skip（正文区域不得被拖拽吃掉：文本选择/划选必须保留）", () => {
    expect(dragDecisionFor({ ...plainBandPress, inBand: false })).toBe("skip")
  })

  it("不在 #root 内 ⇒ skip（dsh 的浮层/门户与壳自绘胶囊都挂在 body 下）", () => {
    expect(dragDecisionFor({ ...plainBandPress, insideRoot: false })).toBe("skip")
  })

  it("全屏 ⇒ skip（全屏无标题栏语义）", () => {
    expect(dragDecisionFor({ ...plainBandPress, fullscreen: true })).toBe("skip")
  })

  it("四个条件的任意组合都不得越界：只有全真组合才 drag", () => {
    const flags = [false, true] as const
    let drags = 0
    for (const inBand of flags)
      for (const insideRoot of flags)
        for (const onInteractive of flags)
          for (const fullscreen of flags) {
            const verdict = dragDecisionFor({ inBand, insideRoot, onInteractive, fullscreen })
            const expected = inBand && insideRoot && !onInteractive && !fullscreen ? "drag" : "skip"
            expect(verdict).toBe(expected)
            if (verdict === "drag") drags++
          }
    expect(drags).toBe(1)
  })
})

describe("与 dsh 的契约常量（改 dsh 升级时须复核，见 ADR-0029 §5 表）", () => {
  it("拖拽带钩子用 dsh 自己发布的稳定标记（不碰带哈希的类名）", () => {
    expect(LEADING_BAND_SELECTOR).toBe("[data-shell-leading-band]")
  })

  it("兜底带高与 dsh .leadingBand 同值（52px）", () => {
    expect(DRAG_BAND_FALLBACK_HEIGHT).toBe(52)
  })

  it("交互元素排除表覆盖 dsh base.css:72-78 的 no-drag 关键项", () => {
    for (const needle of [
      "button",
      "a,",
      "input",
      "select",
      "textarea",
      "summary",
      "[contenteditable='true']",
      "[tabindex]",
      "[role='dialog']",
      "[role='menu']",
      "[role='button']",
      "[role='tab']",
      "[role='menuitem']",
      "[role='option']",
      "[role='checkbox']",
      "[role='switch']",
      "[role='slider']",
      "[role='combobox']",
      "[role='textbox']",
    ]) {
      expect(DRAG_EXCLUSION_SELECTOR).toContain(needle)
    }
    // 反例：不得混入 dsh 没声明的交互角色（否则会把可拖区挖出莫名其妙的洞）
    expect(DRAG_EXCLUSION_SELECTOR).not.toContain("[role='progressbar']")
  })

  it("拖拽面锚在 #root（dsh 的 body > :not(#root) 全是 no-drag 浮层）", () => {
    expect(DRAG_SURFACE_ROOT_SELECTOR).toBe("#root")
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
