// 详情对话框（4.3 前端刀）：开窗时经 api 播种单 profile 详情；
// patch 原文 mono 等宽展示（后端刻意不解析 YAML，原文即真相）。
// 4.4① 插件清单：dependencies 区升级为插件卡（官方/第三方、已装版本、
// 运行态徽标）——运行态快照仅在本 profile 是活跃会话时合并（复现点 11）。
// 4.4② 插件操作：行内卸载/更新 + 区头安装输入行——转发链阻塞可达分钟级，
// 全程 busy 态；结果文案（成功含「重启后生效」、失败附 dsh 输出尾部）由
// 后端给，前端只分箱展示。spec 预检镜像后端校验（validatePluginSpec）。
import { useCallback, useEffect, useState } from "react"
import { LoaderCircle, Plus, RefreshCw, Trash2 } from "lucide-react"
import { api } from "@/lib/tauri"
import { t } from "@/content/zh-CN"
import { runtimeChipFor, runtimeSummary, validatePluginSpec } from "@/lib/profiles"
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
  // 4.4② 操作态：opBusy = "install" | `${op}:${pkg}`；结果分箱 opMessage（成功）/ opError（失败）
  const [opBusy, setOpBusy] = useState<string | null>(null)
  const [opMessage, setOpMessage] = useState<string | null>(null)
  const [opError, setOpError] = useState<string | null>(null)
  const [installOpen, setInstallOpen] = useState(false)
  const [installSpec, setInstallSpec] = useState("")
  const [installError, setInstallError] = useState<string | null>(null)

  const reload = useCallback(() => {
    if (!name) return
    // 静态两源并行；运行态独立容错——回环查询失败不遮蔽静态清单
    api
      .getProfileDetail(name)
      .then((d) => setDetail(d))
      .catch((e) => setError(String(e)))
    api
      .listProfilePlugins(name)
      .then((p) => setPlugins(p))
      .catch(() => setPlugins([]))
    api
      .getPluginRuntime()
      .then((s) => setRuntime(s))
      .catch(() => setRuntime({ profile: null, entries: [] }))
  }, [name])

  useEffect(() => {
    if (!name) return
    setDetail(null)
    setPlugins(null)
    setRuntime(null)
    setError(null)
    setOpMessage(null)
    setOpError(null)
    setInstallOpen(false)
    setInstallSpec("")
    setInstallError(null)
    reload()
  }, [name, reload])

  const runOp = (op: "remove" | "update", pkg: string) => {
    if (!name || opBusy) return
    setOpError(null)
    setOpMessage(null)
    setOpBusy(`${op}:${pkg}`)
    const call = op === "remove" ? api.removePlugin : api.updatePlugin
    call(name, pkg)
      .then((out) => {
        if (out.ok) {
          setOpMessage(out.detail)
          reload()
        } else {
          setOpError(out.detail)
        }
      })
      .catch((e) => setOpError(String(e)))
      .finally(() => setOpBusy(null))
  }

  const submitInstall = () => {
    if (!name || opBusy) return
    const spec = installSpec.trim()
    const invalid = validatePluginSpec(spec)
    if (invalid) {
      setInstallError(invalid)
      return
    }
    setInstallError(null)
    setOpError(null)
    setOpMessage(null)
    setOpBusy("install")
    api
      .installPlugin(name, spec)
      .then((out) => {
        if (out.ok) {
          setOpMessage(out.detail)
          setInstallOpen(false)
          setInstallSpec("")
          reload()
        } else {
          setInstallError(out.detail)
        }
      })
      .catch((e) => setInstallError(String(e)))
      .finally(() => setOpBusy(null))
  }

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

            {/* 外挂插件（4.4①/②）：清单 + 行内卸载/更新 + 区头安装 */}
            <section>
              <div className="text-faint mb-1.5 flex items-baseline justify-between gap-2 text-xs">
                <span>
                  {t.profiles.detailDeps}
                  {plugins !== null && depCount > 0 && (
                    <span className="ml-1.5 font-mono">{t.profiles.metaBundles(depCount)}</span>
                  )}
                </span>
                <button
                  type="button"
                  disabled={opBusy !== null}
                  onClick={() => {
                    setInstallOpen((o) => !o)
                    setInstallError(null)
                  }}
                  className="border-line text-dim hover:border-brand hover:text-brand inline-flex shrink-0 items-center gap-1 rounded-lg border bg-white px-2 py-1 text-xs transition-colors disabled:pointer-events-none disabled:opacity-40"
                >
                  {opBusy === "install" ? (
                    <LoaderCircle className="size-3 animate-spin" aria-hidden />
                  ) : (
                    <Plus className="size-3" aria-hidden />
                  )}
                  {opBusy === "install" ? t.profiles.pluginInstallBusy : t.profiles.pluginInstallBtn}
                </button>
              </div>

              {/* 安装输入行：预检镜像后端校验；后端权威（错误以其返回为准） */}
              {installOpen && (
                <div className="mb-2 space-y-1.5">
                  <div className="flex gap-1.5">
                    <input
                      autoFocus
                      disabled={opBusy !== null}
                      value={installSpec}
                      onChange={(e) => setInstallSpec(e.target.value)}
                      onKeyDown={(e) => e.key === "Enter" && submitInstall()}
                      placeholder={t.profiles.pluginInstallPlaceholder}
                      className="border-line bg-bg text-ink placeholder:text-faint focus:border-brand min-w-0 flex-1 rounded-lg border px-2.5 py-1.5 font-mono text-xs outline-none transition-colors disabled:opacity-50"
                    />
                    <button
                      type="button"
                      disabled={opBusy !== null}
                      onClick={submitInstall}
                      className="border-brand/50 text-brand hover:bg-wash inline-flex shrink-0 items-center gap-1 rounded-lg border bg-white px-2.5 py-1.5 text-xs transition-colors disabled:pointer-events-none disabled:opacity-40"
                    >
                      {opBusy === "install" ? (
                        <LoaderCircle className="size-3 animate-spin" aria-hidden />
                      ) : null}
                      {t.profiles.pluginInstallSubmit}
                    </button>
                    <button
                      type="button"
                      disabled={opBusy !== null}
                      onClick={() => {
                        setInstallOpen(false)
                        setInstallSpec("")
                        setInstallError(null)
                      }}
                      className="border-line text-dim hover:text-ink shrink-0 rounded-lg border bg-white px-2.5 py-1.5 text-xs transition-colors disabled:pointer-events-none disabled:opacity-40"
                    >
                      {t.profiles.pluginInstallCancel}
                    </button>
                  </div>
                  {installError && (
                    <div className="bg-warn-soft text-warn rounded-lg px-2.5 py-1.5 text-xs whitespace-pre-wrap">
                      {installError}
                    </div>
                  )}
                </div>
              )}

              {/* 操作结果分箱：成功（含「重启后生效」）/ 失败（dsh 输出尾部） */}
              {opMessage && (
                <div className="bg-wash text-dim mb-1.5 rounded-lg px-3 py-2 text-xs whitespace-pre-wrap">
                  {opMessage}
                </div>
              )}
              {opError && (
                <div className="bg-warn-soft text-warn mb-1.5 rounded-lg px-3 py-2 text-xs whitespace-pre-wrap">
                  {opError}
                </div>
              )}

              {liveEntries.length > 0 ? (
                <div className="text-faint mb-1.5 text-[10px]">
                  {t.profiles.runtimeSummary(runtimeSummary(liveEntries))}
                </div>
              ) : (
                plugins !== null &&
                depCount > 0 && (
                  <div className="text-faint mb-1.5 text-[10px]">{t.profiles.runtimeUnavailable}</div>
                )
              )}
              {plugins === null ? (
                <div className="text-faint text-xs">{t.profiles.busyShort}</div>
              ) : depCount === 0 ? (
                <div className="text-faint text-xs">{t.profiles.detailEmptyDeps}</div>
              ) : (
                <div className="border-line bg-bg divide-line-soft max-h-64 overflow-y-auto rounded-lg border divide-y">
                  {deps.map((p) => {
                    const spec = detail.dependencies[p.name]
                    const chip = runtimeChipFor(p.name, liveEntries)
                    const rowBusy =
                      opBusy === `remove:${p.name}` || opBusy === `update:${p.name}`
                    return (
                      <div key={p.name} className="group min-w-0 px-3 py-1.5">
                        <div className="flex items-baseline gap-2">
                          <span className="text-ink shrink-0 font-mono text-xs">{p.name}</span>
                          <span
                            className="text-faint min-w-0 truncate font-mono text-xs"
                            title={spec ? `声明：${spec}` : undefined}
                          >
                            {p.installed_version ?? (spec ? t.profiles.pluginNotInstalled : "")}
                          </span>
                          {rowBusy ? (
                            <span className="text-faint ml-auto inline-flex shrink-0 items-center gap-1 text-[10px]">
                              <LoaderCircle className="size-3 animate-spin" aria-hidden />
                              {opBusy?.startsWith("remove") ? t.profiles.pluginOpBusyRemove : t.profiles.pluginOpBusyUpdate}
                            </span>
                          ) : (
                            <>
                              {chip && (
                                <span
                                  className={`ml-auto shrink-0 rounded px-1 text-[10px] leading-4 transition-opacity group-hover:opacity-0 ${
                                    chip.failed ? "bg-warn-soft text-warn" : "bg-ok-soft text-ok"
                                  }`}
                                >
                                  {chip.label}
                                </span>
                              )}
                              {/* 行内操作：更新/卸载，hover 显现（与运行徽标互斥位） */}
                              <span className="text-faint shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100">
                                <button
                                  type="button"
                                  title={t.profiles.pluginUpdate}
                                  aria-label={`${t.profiles.pluginUpdate} ${p.name}`}
                                  disabled={opBusy !== null}
                                  onClick={() => runOp("update", p.name)}
                                  className="hover:bg-wash hover:text-ink inline-flex size-6 items-center justify-center rounded-md transition-colors disabled:opacity-40"
                                >
                                  <RefreshCw className="size-3.5" />
                                </button>
                                <button
                                  type="button"
                                  title={t.profiles.pluginUninstall}
                                  aria-label={`${t.profiles.pluginUninstall} ${p.name}`}
                                  disabled={opBusy !== null}
                                  onClick={() => runOp("remove", p.name)}
                                  className="hover:bg-warn-soft hover:text-warn inline-flex size-6 items-center justify-center rounded-md transition-colors disabled:opacity-40"
                                >
                                  <Trash2 className="size-3.5" />
                                </button>
                              </span>
                            </>
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
