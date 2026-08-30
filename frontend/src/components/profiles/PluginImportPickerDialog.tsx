// 从其他 profile 安装（4.4④ 收口，ADR-0009 第五次修订）：多选批量选择器。
// 候选 = 其他 profile 已安装（版本实读）且目标未装的第三方插件，一行 =
// 插件 × 来源 profile（多来源成多行，版本无歧义）；「连配置」逐行勾选（默认
// 不勾，仅来源 patch 有该插件条目时可选——patch_entries 预检，复制时后端
// 权威复核，写入例外 #4 只追加不覆盖）。执行 = 串行队列（安装走既有转发链
// pkg@version，规避裸名 dist-tag 坑），失败继续，末尾汇总成败与失败明细。
// 行表（getPluginRows，dump-config spawn 秒级）只用于置灰预检；个别失败
// 容忍（hasConfig 按无配置处理 + 提示行说明）。
import { useEffect, useState } from "react"
import { LoaderCircle } from "lucide-react"
import { api } from "@/lib/tauri"
import { t } from "@/content/zh-CN"
import { pickerCandidates, summarizeBatch, type BatchItemResult, type PickerCandidate } from "@/lib/profiles"
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

/** 候选唯一键：插件 × 来源（同名多来源各行独立勾选）。 */
const keyOf = (c: PickerCandidate) => `${c.pkg}\u0000${c.source}`

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
  const [phase, setPhase] = useState<Phase>({ kind: "loading" })
  const [selected, setSelected] = useState<Set<string>>(new Set())
  const [withConfig, setWithConfig] = useState<Set<string>>(new Set())
  // 播种失败（聚合/目标清单不可读）：错误卡 + 关闭按钮，不伪装成空候选
  const [loadError, setLoadError] = useState<string | null>(null)

  // 开窗播种：聚合 + 目标清单并行；来源行表并行（独立容错）
  useEffect(() => {
    if (!open) return
    let alive = true
    setPhase({ kind: "loading" })
    setSelected(new Set())
    setWithConfig(new Set())
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

  const toggle = (key: string) => {
    setSelected((s) => {
      const next = new Set(s)
      if (next.has(key)) next.delete(key)
      else next.add(key)
      return next
    })
    // 取消勾选即撤回其连配置意图（保持两集合一致）
    setWithConfig((s) => {
      const next = new Set(s)
      next.delete(key)
      return next
    })
  }

  const toggleConfig = (key: string) => {
    setWithConfig((s) => {
      const next = new Set(s)
      if (next.has(key)) next.delete(key)
      else next.add(key)
      return next
    })
  }

  // 串行队列：失败继续（安装成功而配置复制失败按该项失败计——勾了连配置
  // 就是一项完整工作），明细进汇总
  const runQueue = async (candidates: PickerCandidate[]) => {
    const queue = candidates.filter((c) => selected.has(keyOf(c)))
    if (queue.length === 0) return
    const results: BatchItemResult[] = []
    for (let i = 0; i < queue.length; i++) {
      const c = queue[i]
      setPhase({ kind: "running", done: i, total: queue.length, current: c.pkg })
      try {
        const out = await api.installPlugin(target, `${c.pkg}@${c.version}`)
        if (!out.ok) {
          results.push({ pkg: c.pkg, ok: false, detail: out.detail })
          continue
        }
        if (withConfig.has(keyOf(c))) {
          try {
            const cc = await api.copyPluginConfig(c.source, target, c.pkg)
            results.push({ pkg: c.pkg, ok: true, detail: cc.detail })
          } catch (e) {
            results.push({
              pkg: c.pkg,
              ok: false,
              detail: `插件已安装，但配置行复制失败：${String(e)}`,
            })
          }
        } else {
          results.push({ pkg: c.pkg, ok: true, detail: out.detail })
        }
      } catch (e) {
        results.push({ pkg: c.pkg, ok: false, detail: String(e) })
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
    if (phase.kind === "running") return // 队列进行中不可中断（无半途状态）
    setPhase({ kind: "loading" })
    onClose()
  }

  const candidates = phase.kind === "picking" ? phase.candidates : []

  return (
    <Dialog open={open} onOpenChange={(o) => !o && close()}>
      <DialogContent className="flex max-h-[calc(100vh-4rem)] flex-col sm:max-w-[520px]">
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
            {phase.rowsFailed && candidates.length > 0 && (
              <div className="text-faint text-[10px]">{t.profiles.importRowsFailedNote}</div>
            )}
            {candidates.length === 0 ? (
              <div className="text-faint border-line bg-bg rounded-lg border border-dashed px-3 py-6 text-center text-xs">
                {t.profiles.importEmpty}
              </div>
            ) : (
              <div className="border-line bg-bg divide-line-soft rounded-lg border divide-y">
                {candidates.map((c) => {
                  const key = keyOf(c)
                  const picked = selected.has(key)
                  return (
                    <div key={key} className="flex items-center gap-2 px-3 py-2">
                      {/* 主勾选：插件 + 来源（label 包裹保证点击区与可达性） */}
                      <label className="flex min-w-0 flex-1 cursor-pointer items-center gap-2">
                        <input
                          type="checkbox"
                          checked={picked}
                          onChange={() => toggle(key)}
                          className="accent-brand size-3.5 shrink-0"
                        />
                        <span className="min-w-0">
                          <span className="flex items-baseline gap-2">
                            <span className="text-ink min-w-0 truncate font-mono text-xs" title={c.pkg}>
                              {c.pkg}
                            </span>
                            <span className="text-faint shrink-0 font-mono text-xs">{c.version}</span>
                            <span className="border-line text-dim shrink-0 rounded bg-white px-1 text-[10px] leading-4">
                              {c.source}
                            </span>
                          </span>
                          {c.description && (
                            <span className="text-faint mt-0.5 block truncate text-[10px]" title={c.description}>
                              {c.description}
                            </span>
                          )}
                        </span>
                      </label>
                      {/* 连配置：逐行勾选，来源无条目置灰（patch_entries 预检） */}
                      <label
                        className={`flex shrink-0 cursor-pointer items-center gap-1 text-[10px] ${
                          c.hasConfig ? "text-dim" : "text-faint"
                        }`}
                        title={c.hasConfig ? undefined : t.profiles.importNoConfig}
                      >
                        <input
                          type="checkbox"
                          checked={picked && withConfig.has(key)}
                          disabled={!picked || !c.hasConfig}
                          onChange={() => toggleConfig(key)}
                          className="accent-brand size-3 shrink-0"
                        />
                        {t.profiles.importConfig}
                      </label>
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
          {phase.kind === "picking" && candidates.length > 0 && (
            <>
              <span className="text-faint mr-auto self-center text-xs">
                {t.profiles.importSelected(selected.size)}
              </span>
              <Button variant="outline" onClick={close}>
                {t.profiles.pluginInstallCancel}
              </Button>
              <Button
                disabled={selected.size === 0}
                onClick={() => void runQueue(candidates)}
              >
                {t.profiles.importStart(selected.size)}
              </Button>
            </>
          )}
          {(phase.kind === "picking" || phase.kind === "loading") && candidates.length === 0 && (
            <Button variant="outline" onClick={close}>
              {t.profiles.detailClose}
            </Button>
          )}
          {phase.kind === "done" && (
            <Button onClick={finish}>{t.profiles.importDoneBtn}</Button>
          )}
          {/* running：无按钮——队列不可中断，footer 留空 */}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
