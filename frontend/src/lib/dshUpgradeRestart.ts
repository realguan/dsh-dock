// lib/dshUpgradeRestart.ts —— DSH 升级落定后「要不要自动重启当前 profile」的纯判据。
//
// 2026-09-24 维护者裁定：关于页升级 DSH 后自动重启当前 profile，让新引擎真正接管
// （此前 `upgrade_only` 刻意不打断会话，装完还要用户自己去控制中心重启——新引擎
// 一直没生效，升级像个没完成的动作）。
//
// 判据收紧为一条：**确实装了新版本**才值得打断会话。`installed === false` 是
// 「已是最新」的 Skip 路径（`updates::UpgradePlan::Skip`），此时重启等于无缘无故
// 掐断进行中的任务。running / failed 相位更不在考虑范围。
import type { DshUpgradeEvent } from "@/types/events"

/** 升级落定且确实安装了新版本 → 应自动重启当前 profile。 */
export function shouldRestartAfterUpgrade(payload: DshUpgradeEvent): boolean {
  return payload.phase === "done" && payload.installed === true
}
