// 详情对话框（4.3 前端刀）：开窗时经 api 播种单 profile 详情；
// patch 原文 mono 等宽展示（后端刻意不解析 YAML，原文即真相）。
// 4.4① 插件清单：dependencies 区升级为插件卡（官方/第三方、已装版本、
// 运行态徽标）——运行态快照仅在本 profile 是活跃会话时合并（复现点 11）。
// 4.4② 插件操作：行内卸载/更新 + 区头安装输入行——转发链阻塞可达分钟级，
// 全程 busy 态；结果文案（成功含「重启后生效」、失败附 dsh 输出尾部）由
// 后端给，前端只分箱展示。spec 预检镜像后端校验（validatePluginSpec）。
import { useCallback, useEffect, useState } from "react"
import { ArrowUpCircle, Import, LoaderCircle, Plus, Power, RefreshCw, Trash2 } from "lucide-react"
import { api } from "@/lib/tauri"
import { useI18n } from "@/stores/i18nStore"
import { runtimeChipFor, runtimeSummary, validatePluginSpec } from "@/lib/profiles"
import type {
  PluginEntry,
  PluginRowState,
  PluginRuntimeSnapshot,
  ProfileDetail,
} from "@/types/ipc"
import { Button } from "@/components/ui/button"
import { PluginImportPickerDialog } from "@/components/profiles/PluginImportPickerDialog"
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
  const { t } = useI18n()
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
  // 4.4④ 收口：从其他 profile 安装（多选批量选择器，完成后回填清单）
  const [importOpen, setImportOpen] = useState(false)
  // 4.4③ 行表（行 id 权威来源；dump-config spawn ~秒级，独立容错）
  const [rows, setRows] = useState<PluginRowState[] | null>(null)
  // 4.4④ 更新检查：name → dist-tags.latest（按钮触发，不自动跑）
  const [updateMap, setUpdateMap] = useState<Record<string, string> | null>(null)
  const [checkState, setCheckState] = useState<"idle" | "busy" | "done">("idle")
  const [checkMeta, setCheckMeta] = useState<{ checked: number; failed: number } | null>(null)
  // 版本选择弹窗：选中某插件的「更新」后拉全版本列表
  const [versionPick, setVersionPick] = useState<{
    pkg: string
    current: string
    latest: string
    items: string[] | null
  } | null>(null)
  const [versionsError, setVersionsError] = useState<string | null>(null)

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
    api
      .getPluginRows(name)
      .then((r) => setRows(r))
      .catch(() => setRows([]))
  }, [name])

  useEffect(() => {
    if (!name) return
    setDetail(null)
    setPlugins(null)
    setRuntime(null)
    setRows(null)
    setError(null)
    setOpMessage(null)
    setOpError(null)
    setInstallOpen(false)
    setInstallSpec("")
    setInstallError(null)
    setImportOpen(false)
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

  // 禁用/启用（4.4③）：patch 单键切换；不热生效，成功文案带重启提示
  const toggleDisabled = (pkg: string) => {
    if (!name || opBusy) return
    const row = rows?.find((r) => r.pkg_name === pkg)
    if (!row) return
    setOpError(null)
    setOpMessage(null)
    setOpBusy(`toggle:${pkg}`)
    api
      .setPluginDisabled(name, row.id, !row.shell_disabled)
      .then(() => {
        setOpMessage(
          row.shell_disabled
            ? `已启用 ${pkg}——重启该 profile 后生效。`
            : `已禁用 ${pkg}——重启该 profile 后生效。`,
        )
        api
          .getPluginRows(name)
          .then((r) => setRows(r))
          .catch(() => {})
      })
      .catch((e) => setOpError(String(e)))
      .finally(() => setOpBusy(null))
  }

  // 更新检查（4.4④）：外网镜像链，串行可达数十秒——按钮触发 + busy 态
  const runUpdateCheck = () => {
    if (!name || opBusy || checkState === "busy") return
    setOpError(null)
    setCheckState("busy")
    api
      .checkPluginUpdates(name)
      .then((r) => {
        setUpdateMap(Object.fromEntries(r.updates.map((u) => [u.name, u.latest])))
        setCheckMeta({ checked: r.checked, failed: r.failed })
        setCheckState("done")
      })
      .catch((e) => {
        setOpError(String(e))
        setCheckState("idle")
      })
  }

  // 版本选择：打开即拉全版本（降序）；选定 → pkg@version 走既有安装链
  const openVersionPick = (pkg: string, current: string, latest: string) => {
    setVersionPick({ pkg, current, latest, items: null })
    setVersionsError(null)
    api
      .listPluginVersions(pkg)
      .then((items) => {
        setVersionPick((v) => (v && v.pkg === pkg ? { ...v, items } : v))
      })
      .catch((e) => setVersionsError(String(e)))
  }

  const installVersion = (spec: string) => {
    if (!name || opBusy) return
    const pkg = versionPick?.pkg ?? ""
    setOpError(null)
    setOpMessage(null)
    setOpBusy("install")
    api
      .installPlugin(name, spec)
      .then((out) => {
        if (out.ok) {
          setOpMessage(out.detail)
          setUpdateMap((m) => {
            if (!m) return m
            const next = { ...m }
            delete next[pkg]
            return next
          })
          setVersionPick(null)
          reload()
        } else {
          setVersionsError(out.detail)
        }
      })
      .catch((e) => setVersionsError(String(e)))
      .finally(() => setOpBusy(null))
  }

  // 运行态只属于活跃会话的 profile——非本 profile 的快照不合并（防张冠李戴）
  const liveEntries =
    runtime !== null && runtime.profile !== null && runtime.profile === name
      ? runtime.entries
      : []
  const deps = plugins?.filter((p) => p.kind === "dependency") ?? []
  const depCount = deps.length
  // 去重（4.4③）：reconcile 会把装的外挂同时写进 bundles 与 dependencies
  // （台账复现点 7）——徽章区只留「层叠内置层」，外挂在下方卡片区出现，
  // 不重复渲染；隐藏数 >0 时留一行指引
  const depNames = new Set(deps.map((d) => d.name))
  const layerBundles = detail?.bundles.filter((b) => !depNames.has(b)) ?? []
  const hiddenLayers = (detail?.bundles.length ?? 0) - layerBundles.length

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
            {/* 插件组合（层叠内置层；外挂层在下方卡片区，见 hiddenLayers 指引） */}
            <section>
              <div className="text-faint mb-1.5 text-xs">{t.profiles.detailBundles}</div>
              <div className="flex flex-wrap gap-1.5">
                {layerBundles.length === 0 && (
                  <span className="text-faint text-xs">{t.profiles.detailEmptyDeps}</span>
                )}
                {layerBundles.map((b) => {
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
              {hiddenLayers > 0 && (
                <div className="text-faint mt-1.5 text-[10px]">
                  {t.profiles.hiddenLayersHint(hiddenLayers)}
                </div>
              )}
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
                {/* 4.4④ 收口：从其他 profile 已装插件里导入（多选批量） */}
                <button
                  type="button"
                  title={t.profiles.importBtnTitle}
                  aria-label={t.profiles.importBtnTitle}
                  disabled={opBusy !== null}
                  onClick={() => setImportOpen(true)}
                  className="border-line text-dim hover:border-brand hover:text-brand inline-flex shrink-0 items-center gap-1 rounded-lg border bg-white px-2 py-1 text-xs transition-colors disabled:pointer-events-none disabled:opacity-40"
                >
                  <Import className="size-3" aria-hidden />
                  {t.profiles.importBtn}
                </button>
                <button
                  type="button"
                  disabled={opBusy !== null || depCount === 0}
                  onClick={runUpdateCheck}
                  title={t.profiles.checkUpdatesBtn}
                  aria-label={t.profiles.checkUpdatesBtn}
                  className="border-line text-dim hover:border-brand hover:text-brand inline-flex size-7 shrink-0 items-center justify-center rounded-lg border bg-white transition-colors disabled:pointer-events-none disabled:opacity-40"
                >
                  <RefreshCw
                    className={`size-3.5 ${checkState === "busy" ? "animate-spin" : ""}`}
                  />
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

              {/* 更新检查结论（4.4④） */}
              {checkState === "done" && checkMeta && (
                <div className="text-faint mb-1.5 text-[10px]">
                  {t.profiles.updateChecked(checkMeta)}
                  {updateMap &&
                    Object.keys(updateMap).length === 0 &&
                    ` · ${t.profiles.allUpToDate}`}
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
                      opBusy === `remove:${p.name}` ||
                      opBusy === `update:${p.name}` ||
                      opBusy === `toggle:${p.name}`
                    const row = rows?.find((r) => r.pkg_name === p.name)
                    const shellDisabled = row?.shell_disabled ?? false
                    return (
                      <div key={p.name} className="group min-w-0 px-3 py-1.5">
                        <div className="flex items-baseline gap-2">
                          {/* 2026-08-29 修复：包名由 shrink-0 改截断——hover 操作组
                              （3×24px）+ 更新徽标 + 版本同排，长名会把操作组推出
                              卡片右缘裁掉；名让位 truncate + title 兜底 */}
                          <span
                            className={`min-w-0 truncate font-mono text-xs ${
                              shellDisabled ? "text-faint line-through" : "text-ink"
                            }`}
                            title={p.name}
                          >
                            {p.name}
                          </span>
                          <span
                            className="text-faint min-w-0 truncate font-mono text-xs"
                            title={spec ? `声明：${spec}` : undefined}
                          >
                            {p.installed_version ?? (spec ? t.profiles.pluginNotInstalled : "")}
                          </span>
                          {/* 更新标识（4.4④）：current → latest，点开选版本 */}
                          {(() => {
                            const latest = updateMap?.[p.name]
                            if (!latest || latest === p.installed_version) return null
                            return (
                              <button
                                type="button"
                                title={t.profiles.updateHint}
                                aria-label={`${t.profiles.updateHint} ${p.name}`}
                                disabled={opBusy !== null}
                                onClick={() =>
                                  openVersionPick(p.name, p.installed_version ?? "", latest)
                                }
                                className="text-brand hover:bg-wash inline-flex shrink-0 items-center gap-0.5 rounded px-1 font-mono text-xs transition-colors disabled:opacity-40"
                              >
                                <ArrowUpCircle className="size-3.5" aria-hidden />
                                {latest}
                              </button>
                            )
                          })()}
                          {rowBusy ? (
                            <span className="text-faint ml-auto inline-flex shrink-0 items-center gap-1 text-[10px]">
                              <LoaderCircle className="size-3 animate-spin" aria-hidden />
                              {opBusy?.startsWith("remove")
                                ? t.profiles.pluginOpBusyRemove
                                : opBusy?.startsWith("toggle")
                                  ? t.profiles.pluginOpBusyUpdate
                                  : t.profiles.pluginOpBusyUpdate}
                            </span>
                          ) : (
                            <>
                              {/* 禁用态常驻；运行徽标只对启用中的插件显示。
                                  hover 时与操作组换位（display 切换而非透明——
                                  透明仍占位，会把操作组推出卡片裁掉） */}
                              {shellDisabled ? (
                                <span className="border-line text-faint ml-auto shrink-0 rounded border px-1 text-[10px] leading-4 group-hover:hidden">
                                  {t.profiles.pluginDisabled}
                                </span>
                              ) : (
                                chip && (
                                  <span
                                    className={`ml-auto shrink-0 rounded px-1 text-[10px] leading-4 group-hover:hidden ${
                                      chip.failed ? "bg-warn-soft text-warn" : "bg-ok-soft text-ok"
                                    }`}
                                  >
                                    {chip.label}
                                  </span>
                                )
                              )}
                              {/* 行内操作：禁用/启用、更新、卸载，hover 显现并补位 */}
                              <span className="text-faint ml-auto hidden shrink-0 items-center gap-0.5 group-hover:flex">
                                {row && (
                                  <button
                                    type="button"
                                    title={shellDisabled ? t.profiles.pluginEnable : t.profiles.pluginDisable}
                                    aria-label={`${shellDisabled ? t.profiles.pluginEnable : t.profiles.pluginDisable} ${p.name}`}
                                    disabled={opBusy !== null}
                                    onClick={() => toggleDisabled(p.name)}
                                    className={`hover:bg-wash inline-flex size-6 items-center justify-center rounded-md transition-colors disabled:opacity-40 ${
                                      shellDisabled ? "text-ok hover:text-ok" : "hover:text-ink"
                                    }`}
                                  >
                                    <Power className="size-3.5" />
                                  </button>
                                )}
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

        {/* 从其他 profile 安装（4.4④ 收口）：多选批量，完成后刷新清单并回填提示 */}
        <PluginImportPickerDialog
          target={name ?? ""}
          open={importOpen && name !== null}
          onClose={() => setImportOpen(false)}
          onDone={(ok, fail) => {
            setOpMessage(t.profiles.importDone(ok, fail))
            reload()
          }}
        />

        {/* 版本选择（4.4④）：降序全版本，标记 最新（dist-tags）/ 当前 */}
        <Dialog open={versionPick !== null} onOpenChange={(o) => !o && setVersionPick(null)}>
          <DialogContent className="flex max-h-[calc(100vh-6rem)] flex-col sm:max-w-[380px]">
            <DialogHeader>
              <DialogTitle className="text-base">
                {versionPick ? t.profiles.pickVersionTitle(versionPick.pkg) : ""}
              </DialogTitle>
              <DialogDescription className="sr-only">
                {versionPick ? t.profiles.pickVersionTitle(versionPick.pkg) : ""}
              </DialogDescription>
            </DialogHeader>
            {versionsError && (
              <div className="bg-warn-soft text-warn rounded-lg px-3 py-2 text-xs whitespace-pre-wrap">
                {versionsError}
              </div>
            )}
            {versionPick?.items === null && !versionsError && (
              <div className="text-faint py-4 text-center text-xs">{t.profiles.busyShort}</div>
            )}
            {versionPick?.items && (
              <div className="min-h-0 flex-1 overflow-y-auto pr-1">
                <div className="border-line bg-bg divide-line-soft rounded-lg border divide-y">
                  {versionPick.items.map((v) => {
                    const isLatest = v === versionPick.latest
                    const isCurrent = v === versionPick.current
                    return (
                      <button
                        key={v}
                        type="button"
                        disabled={opBusy !== null}
                        onClick={() => installVersion(`${versionPick.pkg}@${v}`)}
                        className="hover:bg-wash flex w-full items-baseline gap-2 px-3 py-1.5 text-left transition-colors disabled:opacity-40"
                      >
                        <span className="text-ink font-mono text-xs">{v}</span>
                        {isLatest && (
                          <span className="bg-ok-soft text-ok rounded px-1 text-[10px] leading-4">
                            {t.profiles.versionLatest}
                          </span>
                        )}
                        {isCurrent && (
                          <span className="border-line text-faint ml-auto rounded border px-1 text-[10px] leading-4">
                            {t.profiles.versionCurrent}
                          </span>
                        )}
                      </button>
                    )
                  })}
                </div>
              </div>
            )}
            <DialogFooter>
              <Button variant="outline" onClick={() => setVersionPick(null)}>
                {t.profiles.pluginInstallCancel}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      </DialogContent>
    </Dialog>
  )
}
