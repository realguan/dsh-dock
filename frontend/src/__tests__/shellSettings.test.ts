// shellSettings.test.ts —— settings.json 整体覆盖写的安全基线单测（2026-09-08）。
// 复现先行：修复前调用点写成 `getShellSettings().catch(() => ({}))` 再回写，
// 读取失败即以空对象为基线 → 未参与本次修改的键被清空。
import { describe, expect, it, vi } from "vitest"
import { writeShellSettingsPatch, type ShellSettingsIO } from "@/lib/shellSettings"
import type { ShellSettings } from "@/types/ipc"

/** 一份「用户已经配置过」的基线——任何写入都必须把它完整保留下来。 */
const LOADED: ShellSettings = {
  defaultMode: "local",
  defaultProfile: "web",
  locale: "zh-CN",
  autoRestart: true,
  showFloatingSwitcher: false,
  switcherShortcut: "shift_p",
  dismissedUpdate: "dsh@1.6.0",
}

function ioOf(base: ShellSettings) {
  const write = vi.fn<(next: ShellSettings) => Promise<void>>(async () => {})
  const io: ShellSettingsIO = { read: async () => base, write }
  return { io, write }
}

describe("writeShellSettingsPatch", () => {
  it("读取失败时不写盘，并把错误原样抛出", async () => {
    const write = vi.fn(async () => {})
    const io: ShellSettingsIO = {
      read: async () => {
        throw new Error("settings.json 读取失败")
      },
      write,
    }

    await expect(writeShellSettingsPatch(io, { autoRestart: false })).rejects.toThrow(
      "settings.json 读取失败",
    )
    // 核心断言：读不到基线就绝不落盘（否则会清空 defaultProfile / locale / …）
    expect(write).not.toHaveBeenCalled()
  })

  it("只改 patch 指定的键，其余键原样保留", async () => {
    const { io, write } = ioOf(LOADED)

    const next = await writeShellSettingsPatch(io, { autoRestart: false })

    expect(next).toEqual({ ...LOADED, autoRestart: false })
    expect(write).toHaveBeenCalledWith({ ...LOADED, autoRestart: false })
    // 逐键钉住：任一键被清空都算回归
    expect(next.defaultProfile).toBe("web")
    expect(next.locale).toBe("zh-CN")
    expect(next.switcherShortcut).toBe("shift_p")
    expect(next.dismissedUpdate).toBe("dsh@1.6.0")
    expect(next.showFloatingSwitcher).toBe(false)
    expect(next.defaultMode).toBe("local")
  })

  it("patch 值为 null 时显式落盘 null（locale 选「跟随系统」）", async () => {
    const { io } = ioOf(LOADED)

    const next = await writeShellSettingsPatch(io, { locale: null })

    expect(next.locale).toBeNull()
    expect(next.autoRestart).toBe(true)
  })

  it("写入失败时抛出，且不返回成功对象", async () => {
    const io: ShellSettingsIO = {
      read: async () => LOADED,
      write: async () => {
        throw new Error("磁盘只读")
      },
    }

    await expect(writeShellSettingsPatch(io, { autoRestart: true })).rejects.toThrow(
      "磁盘只读",
    )
  })
})
