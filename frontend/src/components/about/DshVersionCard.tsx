// DSH 维度行（原 about.html render 的 dsh 段迁移）：只读展示 + 可选升级/检查。
// 版本状态消费 bootStore.versions.dsh（boot:update 事件 / 页面播种）；
// 动作语义：升级 = upgrade_only（后台装，不打断会话，下次启动生效）。
// 4.4⑤：升级反馈事件化——`dsh:upgrade`（running/done/failed）驱动真实 busy
// 与失败详情；原 2s 固定假 busy 只防手抖，全程无进度（用户视角「点了没反应」）。
import { useEffect, useState } from "react"
import { LoaderCircle } from "lucide-react"
import { listen } from "@tauri-apps/api/event"
import { api } from "@/lib/tauri"
import { useBootStore } from "@/stores/bootStore"
import type { ComponentUpdate } from "@/types/ipc"
import { t } from "@/content/zh-CN"
import { Button } from "@/components/ui/button"
import { DimRow as Row, DimNote as Note } from "./DimRow"

/// 检查按钮点击后的本地 busy 采用固定时窗（2s）清除：check_updates 是后台线程
/// 命令立即返回，真正的完成信号是 boot:update 快照刷新；此处 busy 只防
/// 手抖连点，不追求与网络生命周期严格对齐。
const ACTION_BUSY_MS = 2000

export function DshVersionCard() {
  const dsh = useBootStore((s) => s.versions?.dsh ?? null)
  const [busy, setBusy] = useState<"none" | "check">("none")
  // 升级态由 dsh:upgrade 事件驱动：running（转圈，可能数分钟）/ done（清）/
  // failed（清 + 展示安装器错误链，含 pnpm 输出尾部）
  const [upgrading, setUpgrading] = useState(false)
  const [upgradeFail, setUpgradeFail] = useState<string | null>(null)

  useEffect(() => {
    const un = listen<{ phase: string; detail: string }>("dsh:upgrade", ({ payload }) => {
      if (payload.phase === "running") {
        setUpgrading(true)
      } else {
        setUpgrading(false)
        if (payload.phase === "failed") setUpgradeFail(payload.detail || null)
      }
    })
    return () => {
      void un.then((f) => f())
    }
  }, [])

  const lock = (kind: Exclude<typeof busy, "none">) => {
    setBusy(kind)
    window.setTimeout(() => setBusy((b) => (b === kind ? "none" : b)), ACTION_BUSY_MS)
  }

  return (
    <Row label={t.about.dshLabel}>
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0">
          <VersionView dim={dsh} />
          {upgrading && <Note>{t.about.upgradeRunning}</Note>}
          {upgradeFail && (
            <>
              <Note tone="warn">{t.about.upgradeFailed}</Note>
              <p
                className="text-faint mt-0.5 line-clamp-3 whitespace-pre-wrap text-[10px]"
                title={upgradeFail}
              >
                {upgradeFail}
              </p>
            </>
          )}
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          {dsh?.newer && (
            <Button
              size="sm"
              disabled={busy !== "none" || upgrading}
              onClick={() => {
                setUpgradeFail(null)
                setUpgrading(true)
                api.terminalAction("upgrade_only").catch(() => {
                  setUpgrading(false)
                  setUpgradeFail(t.about.upgradeFailed)
                })
              }}
            >
              {upgrading && <LoaderCircle className="size-3 animate-spin" aria-hidden />}
              {upgrading ? t.about.upgrading : t.about.btnUpgrade}
            </Button>
          )}
          <Button
            size="sm"
            variant="outline"
            disabled={busy !== "none" || upgrading}
            onClick={() => {
              lock("check")
              api.checkUpdates().catch(() => {})
            }}
          >
            {busy === "check" ? t.about.detecting : t.about.btnCheck}
          </Button>
        </div>
      </div>
    </Row>
  )
}

function VersionView({ dim }: { dim: ComponentUpdate | null }) {
  // 展示映射沿用旧页优先级：error > newer > current&&latest > !current&&latest > else
  if (!dim)
    return (
      <span className="text-faint text-sm">
        {t.about.notDetected}
        <Note>{t.about.detecting}</Note>
      </span>
    )
  if (dim.error)
    return (
      <span className="text-sm">
        {dim.current ?? t.about.notDetected}
        <Note tone="warn">{t.about.checkFailedNet}</Note>
      </span>
    )
  if (dim.newer)
    return (
      <span className="text-sm">
        {dim.current ?? t.about.notDetected}
        <Note tone="accent">
          {t.about.hasNew} {dim.latest ?? ""}
        </Note>
      </span>
    )
  if (dim.current && dim.latest)
    return (
      <span className="text-sm">
        {dim.current}
        <Note>
          {t.about.latestIsNewest}（{dim.latest}）
        </Note>
      </span>
    )
  if (!dim.current && dim.latest)
    return (
      <span className="text-sm">
        {t.about.notDetected}
        <Note>
          {t.about.latestOfficial} {dim.latest} · {t.about.notYetLocal}
        </Note>
      </span>
    )
  return (
    <span className="text-sm">
      {dim.current ?? t.about.notDetected}
      <Note>{t.about.detecting}</Note>
    </span>
  )
}
