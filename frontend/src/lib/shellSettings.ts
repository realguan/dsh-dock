// shellSettings.ts —— settings.json 读改写的安全基线（2026-09-08 裁定）。
//
// 背景：`settings.rs::save` 序列化 `ShellSettings` **全字段**（无
// `skip_serializing_if`），`set_shell_settings` 因此是**整体覆盖写**。
// 于是「读取失败 → 以 `{}` 为基线回写」会把本次没碰过的键
// （defaultProfile / locale / autoRestart / switcherShortcut /
// dismissedUpdate）一并清成 null——与本次配置事故（settings.yaml 被写残）
// 同源风险，只是换了个文件。
//
// 结论：任何 set_shell_settings 调用前，基线必须来自一次**成功**的读取；
// 读取失败即中止（抛出），绝不回退空对象。

import type { ShellSettings } from "@/types/ipc"
import { api } from "./tauri"

/** 读改写的最小依赖面（注入以便纯逻辑单测，生产绑定见 `patchShellSettings`）。 */
export interface ShellSettingsIO {
  read: () => Promise<ShellSettings>
  write: (next: ShellSettings) => Promise<void>
}

/**
 * 以一次成功读取为基线合并 patch 后整体写回，返回写回对象。
 * 读取失败不写、原样抛出；调用方可以降级（如界面继续用内存值），
 * 但**降级路径里不得包含写入**。
 */
export async function writeShellSettingsPatch(
  io: ShellSettingsIO,
  patch: Partial<ShellSettings>,
): Promise<ShellSettings> {
  const base = await io.read()
  const next: ShellSettings = { ...base, ...patch }
  await io.write(next)
  return next
}

/** 生产入口：绑定 IPC。patch 的键会覆盖基线同名字段，其余字段原样保留。 */
export function patchShellSettings(
  patch: Partial<ShellSettings>,
): Promise<ShellSettings> {
  return writeShellSettingsPatch(
    { read: api.getShellSettings, write: api.setShellSettings },
    patch,
  )
}
