// 插件开关目标（2026-09-08，ADR-0009 第七次修订：补丁包开关）。
// 开关语义 = 写 profile cordis.patch.yml 的 `{<行id>, disabled}` 条目（写入例外
// #3 单函数 `set_plugin_disabled`）；开关目标：普通插件 = 自身行 id，补丁包
// （无自身行、以 `dsh.bundle.patch` insert 贡献行的 bundle）= 全部贡献行。
// dsh 侧依据见 docs/contracts/dsh-behavior-ledger.md 复现点 13。
import type { PluginRowState, RuntimeEntry } from "@/types/ipc"
import { runtimeToggleApplied } from "@/lib/profiles"

/** 开关目标行 id 列表：contributed_ids 非空（补丁包）取全部贡献行，否则取自身行。
 *  补丁包后端已按「贡献行全部禁用」聚合 shell_disabled；部分禁用（手改 patch
 *  中间态）显示为启用，切换一次全量禁用。 */
export function pluginToggleTargets(row: PluginRowState): string[] {
  return row.contributed_ids.length > 0 ? row.contributed_ids : [row.id]
}

/**
 * 一次开关的**两种口径**（2026-09-17，独立复核抓到的极性缺陷后补的护栏）。
 *
 * `set_plugin_disabled(profile, row_id, disabled)` 的第三参是 **disabled**（true = 停用）；
 * 而文案（「已启用/已禁用」）与落定判据（`runtimeToggleApplied(..., wantEnabled)`，
 * 比对清单里的 `enabled`）用的是 **enabled**。两者互为反相，混用即播与事实相反的话
 * ——复核前的实现正是把 `!shell_disabled`（= 新 disabled 值）当 enabled 传给文案，
 * 于是"点开关关掉插件"会报「已启用 X（已生效）」；源码文本门禁抓不到这类实参极性错误，
 * 故收敛成一个纯函数：口径在类型上分开，调用点不许再手算取反。
 */
export interface ToggleIntent {
  /** 本次要落成的**启用态**（true = 启用）——文案与落定判据只认这个。 */
  wantEnabled: boolean
  /** 写进 patch 的 `disabled` 值（与 `wantEnabled` 相反）——只喂 `setPluginDisabled`。 */
  writeDisabled: boolean
}

/** 由当前行态推出本次开关意图（现在禁用 ⇒ 这次要启用）。 */
export function toggleIntent(row: PluginRowState): ToggleIntent {
  const wantEnabled = row.shell_disabled
  return { wantEnabled, writeDisabled: !wantEnabled }
}

/**
 * 把「待落定的开关集合」按**这一次**运行态快照切成已落定 / 仍待定。
 *
 * 为什么是集合而不是单个（2026-09-17 独立复核 P2）：用户可能连点两行（0.5s 内），
 * 单个 pending ref 会在第二次点击时被清掉 ⇒ 前一行的「生效中」指示与结论一起丢失
 * （行内又退回可能已过期的运行态徽标）。集合化后一次轮询可同时结算多行，
 * 各行的结论互不吞并。
 */
export function splitSettledToggles(
  pending: Record<string, boolean>,
  entries: RuntimeEntry[],
): { settled: string[]; remaining: Record<string, boolean> } {
  const settled: string[] = []
  const remaining: Record<string, boolean> = {}
  for (const [pkg, wantEnabled] of Object.entries(pending)) {
    if (runtimeToggleApplied(entries, pkg, wantEnabled)) settled.push(pkg)
    else remaining[pkg] = wantEnabled
  }
  return { settled, remaining }
}
