import { Info, RotateCw } from "lucide-react"
import { useI18n } from "@/stores/i18nStore"
import { useRestartNeededStore } from "@/stores/restartNeededStore"
import { requestRestartProfile } from "@/lib/restartProfile"
import { isReservedProfileName } from "@/lib/profiles"
import { Button } from "@/components/ui/button"

// components/ui/restart-hint.tsx —— 「配置已变更，重启该 Profile 后生效」提示条
// （2026-09-24 从 ExperimentalCapabilities 的内联横幅提取，全仓唯一重启提示面）。
//
// 为什么提取：实验能力面板早就有一套"置位 dirty → 横幅 + 立即重启按钮"的完整模式
// （含部分失败只报真的改过的判据、换档防串档）。插件安装/卸载/更新/启停是同一语义，
// 再各写一套就是一种逻辑两种交互。现在：
//   · 状态唯一来源 = stores/restartNeededStore（按 profile 名键控）；
//   · 动作唯一链路 = lib/restartProfile（走既有确认链，与左栏行「重启」同链）；
//   · 本组件只负责"该档在待重启集合里就画出来"——自门控，调用方不用判空。
//
// 视觉与文案 1:1 沿用原 caps 横幅（warn 边框/底 + Info + 立即重启），老用户看到的是
// 同一个东西出现在更多该出现的地方，而不是一个新控件。
//
// 2026-10-14 复核（ADR-0034）：**保留档不再给「立即重启」**。该档的内容面现在照常可
// 改（插件装卸/启停、实验能力、MCP），于是待重启标记也会落在它身上；但它的生命周期面
// 是关闭的（壳里没有重启入口，点下去只会被拒）——留下按钮就是"点不动的假入口"。
// 它的"重启"是**官方桌面客户端下次启动**，所以只留一句话说清改动何时生效。
export function RestartNeededHint({ profile }: { profile: string }) {
  const { t } = useI18n()
  const pending = useRestartNeededStore((s) => s.pending.includes(profile))
  if (!pending) return null
  // 判据与后端同源镜像（lib/profiles.ts 是前端唯一一份保留名名单）
  if (isReservedProfileName(profile)) {
    return (
      <div className="flex flex-wrap items-center gap-2 rounded-lg border border-warn/30 bg-warn-soft px-3 py-1.5 text-label text-warn">
        <Info className="size-3.5 shrink-0" />
        <span>{t.restart.reservedNeededHint}</span>
      </div>
    )
  }
  return (
    <div className="flex flex-wrap items-center gap-2 rounded-lg border border-warn/30 bg-warn-soft px-3 py-1.5 text-label text-warn">
      <Info className="size-3.5 shrink-0" />
      <span>{t.restart.neededHint}</span>
      <Button
        size="xs"
        variant="outline"
        className="ml-auto gap-1"
        onClick={() => requestRestartProfile(profile)}
      >
        <RotateCw className="size-3" />
        {t.restart.nowBtn}
      </Button>
    </div>
  )
}
