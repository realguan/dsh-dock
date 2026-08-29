// 详情对话框（4.3 前端刀）：开窗时经 api 播种单 profile 详情；
// patch 原文 mono 等宽展示（后端刻意不解析 YAML，原文即真相）。
// 4.4① 插件清单：dependencies 区升级为插件卡（官方/第三方、已装版本、
// 运行态徽标）——运行态快照仅在本 profile 是活跃会话时合并（复现点 11）。
import { useEffect, useState } from "react"
import { api } from "@/lib/tauri"
import { t } from "@/content/zh-CN"
import { runtimeChipFor, runtimeSummary } from "@/lib/profiles"
import type { PluginEntry, PluginRuntimeSnapshot, ProfileDetail } from "@/types/ipc"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

export function ProfileDetailDialog({
  name,
  onClose,
}: {
  name: string | null
  onClose: () => void
}) {
  const [detail, setDetail] = useState<ProfileDetail | null>(null)
  const [plugins, setPlugins] = useState<PluginEntry[] | null>(null)
  const [runtime, setRuntime] = useState<PluginRuntimeSnapshot | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!name) return
    let alive = true
    setDetail(null)
    setPlugins(null)
    setRuntime(null)
    setError(null)
    // 静态两源并行；运行态独立容错——回环查询失败不遮蔽静态清单
    api
      .getProfileDetail(name)
      .then((d) => {
        if (alive) setDetail(d)
      })
      .catch((e) => {
        if (alive) setError(String(e))
      })
    api
      .listProfilePlugins(name)
      .then((p) => {
        if (alive) setPlugins(p)
      })
      .catch(() => {
        if (alive) setPlugins([])
      })
    api
      .getPluginRuntime()
      .then((s) => {
        if (alive) setRuntime(s)
      })
      .catch(() => {
        if (alive) setRuntime({ profile: null, entries: [] })
      })
    return () => {
      alive = false
    }
  }, [name])

  // 运行态只属于活跃会话的 profile——非本 profile 的快照不合并（防张冠李戴）
  const liveEntries =
    runtime !== null && runtime.profile !== null && runtime.profile === name
      ? runtime.entries
      : []
  const deps = plugins?.filter((p) => p.kind === "dependency") ?? []
  const depCount = deps.length

  return (
    <Dialog open={!!name} onOpenChange={(o) => !o && onClose()}>
      {/* 2026-08-29 修复：基件 DialogContent 垂直居中且无高度上限——插件一多
          对话框上下两头裁出屏幕（同工具窗口裁顶族）。这里覆写为纵向 flex +
          视口限高，内容区自身滚动，header/footer 恒在。 */}
      <DialogContent className="flex max-h-[calc(100vh-4rem)] flex-col sm:max-w-[480px]">
        <DialogHeader>
          <DialogTitle>{name ? t.profiles.detailTitle(name) : ""}</DialogTitle>
          <DialogDescription className="sr-only">
            {t.profiles.detailTitle(name ?? "")}
          </DialogDescription>
        </DialogHeader>

        {!detail && !error && (
          <div className="text-faint py-6 text-center text-sm">{t.profiles.busyShort}</div>
        )}
        {error && (
          <div className="bg-warn-soft text-warn rounded-lg px-3 py-2 text-xs whitespace-pre-wrap">
            {error}
          </div>
        )}
        {detail && (
          <div className="text-dim min-h-0 min-w-0 flex-1 space-y-4 overflow-y-auto pr-1 text-sm">
            {/* 2026-08-28 修复：DialogContent 是 grid——本项（grid 项）无 min-w-0
                时最小宽度 = 内容 min-content，whitespace-pre 的最长行会把整条
                轨道撑破，依赖卡片与 footer 一起越界；min-w-0 后 pre 由自身
                overflow-auto 横向滚动。 */}
            {/* 插件组合 */}
            <section>
              <div className="text-faint mb-1.5 text-xs">{t.profiles.detailBundles}</div>
              <div className="flex flex-wrap gap-1.5">
                {detail.bundles.length === 0 && (
                  <span className="text-faint text-xs">{t.profiles.detailEmptyDeps}</span>
                )}
                {detail.bundles.map((b) => {
                  const chip = runtimeChipFor(b, liveEntries)
                  return (
                    <span
                      key={b}
                      className="border-line bg-bg text-ink inline-flex items-center gap-1.5 rounded-md border px-2 py-0.5 font-mono text-xs"
                    >
                      {b}
                      {chip && (
                        <span
                          className={`rounded px-1 text-[10px] leading-4 ${
                            chip.failed ? "bg-warn-soft text-warn" : "bg-ok-soft text-ok"
                          }`}
                        >
                          {chip.label}
                        </span>
                      )}
                    </span>
                  )
                })}
              </div>
            </section>

            {/* 外挂插件（4.4①）：官方/第三方、已装版本、运行态徽标；列表自限高
                滚动（max-h-64，同 patch 原文区模式）——多插件不撑破对话框 */}
            <section>
              <div className="text-faint mb-1.5 flex items-baseline justify-between gap-2 text-xs">
                <span>{t.profiles.detailDeps}</span>
                {plugins !== null && depCount > 0 && (
                  <span className="shrink-0 font-mono">{t.profiles.metaBundles(depCount)}</span>
                )}
              </div>
              {liveEntries.length > 0 ? (
                <div className="text-faint mb-1.5 text-[10px]">
                  {t.profiles.runtimeSummary(runtimeSummary(liveEntries))}
                </div>
              ) : (
                plugins !== null &&
                plugins.some((p) => p.kind === "dependency") && (
                  <div className="text-faint mb-1.5 text-[10px]">{t.profiles.runtimeUnavailable}</div>
                )
              )}
              {plugins === null ? (
                <div className="text-faint text-xs">{t.profiles.busyShort}</div>
              ) : depCount === 0 ? (
                <div className="text-faint text-xs">{t.profiles.detailEmptyDeps}</div>
              ) : (
                <div className="border-line bg-bg divide-line-soft max-h-64 overflow-y-auto rounded-lg border divide-y">
                  {deps
                    .map((p) => {
                      const spec = detail.dependencies[p.name]
                      const chip = runtimeChipFor(p.name, liveEntries)
                      return (
                        <div key={p.name} className="min-w-0 px-3 py-1.5">
                          <div className="flex items-baseline gap-2">
                            <span className="text-ink shrink-0 font-mono text-xs">{p.name}</span>
                            <span
                              className="text-faint min-w-0 truncate font-mono text-xs"
                              title={spec ? `声明：${spec}` : undefined}
                            >
                              {p.installed_version ?? (spec ? t.profiles.pluginNotInstalled : "")}
                            </span>
                            {chip && (
                              <span
                                className={`ml-auto shrink-0 rounded px-1 text-[10px] leading-4 ${
                                  chip.failed ? "bg-warn-soft text-warn" : "bg-ok-soft text-ok"
                                }`}
                              >
                                {chip.label}
                              </span>
                            )}
                          </div>
                          {(p.description || spec) && (
                            <div
                              className="text-faint mt-0.5 truncate text-[10px]"
                              title={p.description ?? spec}
                            >
                              {p.description ?? spec}
                            </div>
                          )}
                        </div>
                      )
                    })}
                </div>
              )}
            </section>

            {/* patch 原文 */}
            <section>
              <div className="text-faint mb-1.5 text-xs">{t.profiles.detailPatch}</div>
              {/* 2026-08-28 修复：原文视图保真优先——whitespace-pre 不折行 +
                  overflow-auto 横向滚动。原 pre-wrap 续行顶格无悬挂缩进，
                  与真实行混淆，破坏「原文」语义。 */}
              {detail.patch_yaml === null ? (
                <div className="text-faint text-xs">{t.profiles.detailPatchNone}</div>
              ) : (
                <pre className="border-line bg-bg text-dim max-h-56 overflow-auto rounded-lg border p-3 font-mono text-xs leading-relaxed whitespace-pre">
                  {detail.patch_yaml}
                </pre>
              )}
            </section>
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            {t.profiles.detailClose}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
