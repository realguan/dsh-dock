// 插件开关目标（2026-09-08，ADR-0009 第七次修订：补丁包开关）。
// 开关语义 = 写 profile cordis.patch.yml 的 `{<行id>, disabled}` 条目（写入例外
// #3 单函数 `set_plugin_disabled`）；开关目标：普通插件 = 自身行 id，补丁包
// （无自身行、以 `dsh.bundle.patch` insert 贡献行的 bundle）= 全部贡献行。
// dsh 侧依据见 docs/contracts/dsh-behavior-ledger.md 复现点 13。
import type { PluginRowState } from "@/types/ipc"

/** 开关目标行 id 列表：contributed_ids 非空（补丁包）取全部贡献行，否则取自身行。
 *  补丁包后端已按「贡献行全部禁用」聚合 shell_disabled；部分禁用（手改 patch
 *  中间态）显示为启用，切换一次全量禁用。 */
export function pluginToggleTargets(row: PluginRowState): string[] {
  return row.contributed_ids.length > 0 ? row.contributed_ids : [row.id]
}
