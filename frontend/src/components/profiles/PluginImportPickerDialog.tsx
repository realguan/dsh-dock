import { useEffect, useMemo, useState } from "react"
import { LoaderCircle } from "lucide-react"
import { api } from "@/lib/tauri"
import { useI18n } from "@/stores/i18nStore"
import {
  groupPickerCandidates,
  pickerCandidates,
  summarizeBatch,
  type BatchItemResult,
  type GroupedPickerCandidate,
  type PickerCandidate,
} from "@/lib/profiles"
import { getPluginDisplayName } from "@/lib/market"
import { getProfileColorClass } from "@/lib/format"
import type { AggregatePlugin, PluginRowState } from "@/types/ipc"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

type Phase =
  | { kind: "loading" }
  | { kind: "picking"; candidates: PickerCandidate[]; rowsFailed: boolean }
  | { kind: "running"; done: number; total: number; current: string }
  | { kind: "done"; results: BatchItemResult[] }

export function PluginImportPickerDialog({
  target,
  open,
  onClose,
  onDone,
}: {
  target: string
  open: boolean
  onClose: () => void
  /** 全部队列结束后回调（父层刷新清单 + 页面级提示） */
  onDone: (okCount: number, failCount: number) => void
}) {
  const { t } = useI18n()
  const [phase, setPhase] = useState<Phase>({ kind: "loading" })
  const [selectedPkgs, setSelectedPkgs] = useState<Set<string>>(new Set())
  const [chosenSource, setChosenSource] = useState<Record<string, string>>({})
  const [withConfigPkgs, setWithConfigPkgs] = useState<Set<string>>(new Set())
  // 播种失败（聚合/目标清单不可读）：错误卡 + 关闭按钮，不伪装成空候选
  const [loadError, setLoadError] = useState<string | null>(null)

  // 开窗播种：聚合 + 目标清单并行；来源行表并行（独立容错）
  useEffect(() => {
    if (!open) return
    let alive = true
    setPhase({ kind: "loading" })
    setSelectedPkgs(new Set())
    setChosenSource({})
    setWithConfigPkgs(new Set())
    setLoadError(null)
    void (async () => {
      try {
        const [aggregate, targetPlugins] = await Promise.all([
          api.listAllPlugins(),
          api.listProfilePlugins(target),
        ])
        const targetDeps = targetPlugins
          .filter((p) => p.kind === "dependency")
          .map((p) => p.name)
        const sourceProfiles = [
          ...new Set(
            aggregate.flatMap((a: AggregatePlugin) => a.sources.map((s) => s.profile)),
          ),
        ].filter((p) => p !== target)
        const rowsList = await Promise.allSettled(
          sourceProfiles.map((p) => api.getPluginRows(p)),
        )
        const rowsByProfile: Record<string, PluginRowState[]> = {}
        let rowsFailed = false
        rowsList.forEach((r, i) => {
          if (r.status === "fulfilled") rowsByProfile[sourceProfiles[i]] = r.value
          else rowsFailed = true
        })
        if (!alive) return
        const candidates = pickerCandidates(aggregate, target, targetDeps, rowsByProfile)
        setPhase({ kind: "picking", candidates, rowsFailed })
      } catch (e) {
        if (!alive) return
        setLoadError(String(e))
      }
    })()
    return () => {
      alive = false
    }
  }, [open, target])

  const groupedCandidates = useMemo(() => {
    const raw = phase.kind === "picking" ? phase.candidates : []
    return groupPickerCandidates(raw)
  }, [phase])

  const togglePkg = (pkg: string) => {
    setSelectedPkgs((prev) => {
      const next = new Set(prev)
      if (next.has(pkg)) next.delete(pkg)
      else next.add(pkg)
      return next
    })
    // 取消勾选时撤回连配置意图
    setWithConfigPkgs((prev) => {
      const next = new Set(prev)
      next.delete(pkg)
      return next
    })
  }

  const toggleConfig = (pkg: string) => {
    setWithConfigPkgs((prev) => {
      const next = new Set(prev)
      if (next.has(pkg)) next.delete(pkg)
      else next.add(pkg)
      return next
    })
  }

  const setPkgSource = (pkg: string, sourceProfile: string) => {
    setChosenSource((prev) => ({ ...prev, [pkg]: sourceProfile }))
  }

  // 串行队列：按已选插件依次安装
  const runQueue = async (groups: GroupedPickerCandidate[]) => {
    const queue = groups.filter((g) => selectedPkgs.has(g.pkg))
    if (queue.length === 0) return
    const results: BatchItemResult[] = []
    for (let i = 0; i < queue.length; i++) {
      const g = queue[i]
      const activeSrcName = chosenSource[g.pkg] || g.sources[0]?.profile
      const activeSrc = g.sources.find((s) => s.profile === activeSrcName) || g.sources[0]
      setPhase({ kind: "running", done: i, total: queue.length, current: g.pkg })
      try {
        const out = await api.installPlugin(target, `${g.pkg}@${activeSrc.version}`)
        if (!out.ok) {
          results.push({ pkg: g.pkg, ok: false, detail: out.detail })
          continue
        }
        if (withConfigPkgs.has(g.pkg) && activeSrc.hasConfig) {
          try {
            const cc = await api.copyPluginConfig(activeSrc.profile, target, g.pkg)
            results.push({ pkg: g.pkg, ok: true, detail: cc.detail })
          } catch (e) {
            results.push({
              pkg: g.pkg,
              ok: false,
              detail: `插件已安装，但配置行复制失败：${String(e)}`,
            })
          }
        } else {
          results.push({ pkg: g.pkg, ok: true, detail: out.detail })
        }
      } catch (e) {
        results.push({ pkg: g.pkg, ok: false, detail: String(e) })
      }
    }
    setPhase({ kind: "done", results })
  }

  const finish = () => {
    if (phase.kind !== "done") return
    const s = summarizeBatch(phase.results)
    setPhase({ kind: "loading" })
    onDone(s.okCount, s.failCount)
    onClose()
  }

  const close = () => {
    if (phase.kind === "running") return // 队列进行中不可中断
    setPhase({ kind: "loading" })
    onClose()
  }

  return (
    <Dialog open={open} onOpenChange={(o) => !o && close()}>
      <DialogContent className="flex max-h-[calc(100vh-4rem)] flex-col sm:max-w-[560px]">
        <DialogHeader>
          <DialogTitle>{t.profiles.importTitle(target)}</DialogTitle>
          <DialogDescription className="sr-only">
            {t.profiles.importTitle(target)}
          </DialogDescription>
        </DialogHeader>

        {phase.kind !== "done" && (
          <p className="text-dim rounded-lg bg-wash/60 px-3 py-2 text-xs leading-relaxed">
            {t.profiles.importNote}
          </p>
        )}

        {/* 主体四态：loading / picking / running / done；播种失败 = 错误卡 + 关闭 */}
        {loadError ? (
          <div className="bg-warn-soft text-warn rounded-lg px-3 py-2 text-xs whitespace-pre-wrap">
            {loadError}
          </div>
        ) : phase.kind === "loading" ? (
          <div className="text-faint py-6 text-center text-sm">{t.profiles.importLoading}</div>
        ) : null}

        {phase.kind === "picking" && (
          <div className="min-h-0 min-w-0 flex-1 space-y-2 overflow-y-auto pr-1">
            {phase.rowsFailed && groupedCandidates.length > 0 && (
              <div className="text-faint text-[10px]">{t.profiles.importRowsFailedNote}</div>
            )}
            {groupedCandidates.length === 0 ? (
              <div className="text-faint border-line bg-bg rounded-lg border border-dashed px-3 py-6 text-center text-xs">
                {t.profiles.importEmpty}
              </div>
            ) : (
              <div className="divide-y divide-line rounded-xl border border-line bg-bg shadow-2xs">
                {groupedCandidates.map((g) => {
                  const picked = selectedPkgs.has(g.pkg)
                  const currentSrcName = chosenSource[g.pkg] || g.sources[0]?.profile
                  const currentSrc =
                    g.sources.find((s) => s.profile === currentSrcName) || g.sources[0]
                  const hasConfig = currentSrc?.hasConfig ?? false
                  const displayName = getPluginDisplayName(g.pkg)

                  return (
                    <div
                      key={g.pkg}
                      className={`flex flex-col gap-2 p-3 transition-colors ${
                        picked ? "bg-wash/40" : "hover:bg-panel"
                      }`}
                    >
                      <div className="flex items-start justify-between gap-3">
                        {/* 主勾选：插件 + 名称 */}
                        <label className="flex min-w-0 flex-1 cursor-pointer items-start gap-3">
                          <input
                            type="checkbox"
                            checked={picked}
                            onChange={() => togglePkg(g.pkg)}
                            className="accent-brand mt-0.5 size-4 shrink-0 rounded"
                          />
                          <span className="min-w-0 flex-1">
                            <span className="flex flex-wrap items-center gap-2">
                              <span
                                className="text-ink font-mono text-xs font-semibold"
                                title={g.pkg}
                              >
                                {displayName}
                              </span>
                              {displayName !== g.pkg && (
                                <span className="text-faint font-mono text-[10px]" title={g.pkg}>
                                  ({g.pkg})
                                </span>
                              )}
                              <span className="text-brand font-mono text-[11px] font-medium">
                                v{currentSrc?.version}
                              </span>
                            </span>
                            {g.description && (
                              <span
                                className="text-faint mt-0.5 block truncate text-[11px]"
                                title={g.description}
                              >
                                {g.description}
                              </span>
                            )}
                          </span>
                        </label>

                        {/* 连配置：逐行勾选，来源无条目置灰 */}
                        <label
                          className={`flex shrink-0 cursor-pointer items-center gap-1.5 rounded-lg border border-line bg-panel px-2 py-1 text-xs transition-opacity ${
                            hasConfig
                              ? "text-dim hover:text-ink"
                              : "text-faint opacity-50 cursor-not-allowed"
                          }`}
                          title={hasConfig ? undefined : t.profiles.importNoConfig}
                        >
                          <input
                            type="checkbox"
                            checked={picked && withConfigPkgs.has(g.pkg)}
                            disabled={!picked || !hasConfig}
                            onChange={() => toggleConfig(g.pkg)}
                            className="accent-brand size-3.5"
                          />
                          <span className="text-[11px]">{t.profiles.importConfig}</span>
                        </label>
                      </div>

                      {/* 来源 Profile 切换器（折叠去重） */}
                      <div className="ml-7 flex flex-wrap items-center gap-1.5 text-[11px]">
                        <span className="text-faint">来源：</span>
                        {g.sources.map((src) => {
                          const isSelectedSrc = src.profile === currentSrcName
                          const colorCls = getProfileColorClass(src.profile)
                          return (
                            <button
                              key={src.profile}
                              type="button"
                              onClick={() => setPkgSource(g.pkg, src.profile)}
                              className={`inline-flex items-center gap-1 rounded-md px-2 py-0.5 font-mono text-[10px] transition-all ${
                                isSelectedSrc
                                  ? `${colorCls} ring-1 ring-brand/40 font-semibold shadow-2xs`
                                  : "bg-panel text-dim hover:bg-line-soft opacity-75"
                              }`}
                              title={`从 ${src.profile} (v${src.version}) 导入${src.hasConfig ? "，包含配置" : ""}`}
                            >
                              <span>{src.profile}</span>
                              <span className="text-faint opacity-80">v{src.version}</span>
                              {src.hasConfig && (
                                <span className="text-brand text-[9px] font-bold">⚙</span>
                              )}
                            </button>
                          )
                        })}
                      </div>
                    </div>
                  )
                })}
              </div>
            )}
          </div>
        )}

        {phase.kind === "running" && (
          <div className="text-dim flex flex-1 flex-col items-center justify-center gap-2 py-8 text-sm">
            <LoaderCircle className="text-brand size-5 animate-spin" aria-hidden />
            <span>
              {t.profiles.importRunning(phase.done + 1, phase.total)}
              <span className="text-faint ml-2 font-mono text-xs">{phase.current}</span>
            </span>
          </div>
        )}

        {phase.kind === "done" && (() => {
          const s = summarizeBatch(phase.results)
          return (
            <div className="min-h-0 min-w-0 flex-1 space-y-2 overflow-y-auto pr-1">
              <div
                className={`rounded-lg px-3 py-2 text-xs ${
                  s.failCount === 0 ? "bg-ok-soft text-ok" : "bg-warn-soft text-warn"
                }`}
              >
                {t.profiles.importDone(s.okCount, s.failCount)}
              </div>
              {s.failures.length > 0 && (
                <div className="border-line bg-bg divide-line-soft rounded-lg border divide-y">
                  <div className="text-warn px-3 py-1.5 text-[10px]">{t.profiles.importFailures}</div>
                  {s.failures.map((f) => (
                    <div key={f.pkg} className="px-3 py-2">
                      <div className="text-ink font-mono text-xs">{f.pkg}</div>
                      <div className="text-warn mt-0.5 text-xs whitespace-pre-wrap">{f.detail}</div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          )
        })()}

        <DialogFooter>
          {phase.kind === "picking" && groupedCandidates.length > 0 && (
            <>
              <span className="text-faint mr-auto self-center text-xs">
                {t.profiles.importSelected(selectedPkgs.size)}
              </span>
              <Button variant="outline" onClick={close}>
                {t.profiles.pluginInstallCancel}
              </Button>
              <Button
                disabled={selectedPkgs.size === 0}
                onClick={() => void runQueue(groupedCandidates)}
              >
                {t.profiles.importStart(selectedPkgs.size)}
              </Button>
            </>
          )}
          {(phase.kind === "picking" || phase.kind === "loading") &&
            groupedCandidates.length === 0 && (
              <Button variant="outline" onClick={close}>
                {t.profiles.detailClose}
              </Button>
            )}
          {phase.kind === "done" && (
            <Button onClick={finish}>{t.profiles.importDoneBtn}</Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
