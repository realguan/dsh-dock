// capability-icon.ts —— 实验能力的图标映射（2026-09-21 单一源收敛）。
//
// 起因：能力卡片与能力面板各自内联了一份映射，卡片那份还用统一的 Sparkles 兜到底
// ——真机截图里四个能力左侧全是同一个 ✨，完全看不出是"哪个能力"，图标退化成装饰。
// 图标是**能力的身份信号**，必须一能力一形，且只有一份来源。
//
// 兜底给**烧瓶**（"这是一项实验能力"）而不是某个具体能力图标：用 Bot 兜底会把
// 未知 id 误导成 agent-team。2026-09-21 维护者真机反馈："icon 用的与本身的意思不符，
// 比如实验这类，应该可以用烧杯这种 icon" —— ✨(Sparkles) 的语义是"魔法/新奇"，
// 跟"实验"没关系，故兜底从 Sparkles 换成 FlaskConical。
import {
  Bot,
  FlaskConical,
  Globe,
  MonitorSmartphone,
  ShieldCheck,
  type LucideIcon,
} from "lucide-react"

/** 能力 id → 图标（能力 id 由后端稳定下发，见 official_catalog::CAPABILITIES）。 */
const ICONS: Record<string, LucideIcon> = {
  "agent-team": Bot,
  "browser-use": Globe,
  "computer-use": MonitorSmartphone,
  "auto-review": ShieldCheck,
}

/** 取能力图标；未知 id 退回通用实验图标（后端新增能力时前端不崩、也不张冠李戴）。 */
export function capabilityIcon(id: string): LucideIcon {
  return ICONS[id] ?? FlaskConical
}
