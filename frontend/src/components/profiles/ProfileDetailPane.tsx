import { useCallback, useEffect, useMemo, useState } from "react"
import {
  ArrowUpCircle,
  Boxes,
  Check,
  Code2,
  Copy,
  Import,
  Layers,
  LoaderCircle,
  Package,
  Plus,
  RefreshCw,
  Search,
  Star,
  Trash2,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { useCopy } from "@/hooks/useCopy"
import { useI18n } from "@/stores/i18nStore"
import { runtimeChipFor, runtimeSummary, validatePluginSpec } from "@/lib/profiles"
import { pluginToggleTargets } from "@/lib/pluginToggle"
import type {
  PluginEntry,
  PluginRowState,
  PluginRuntimeSnapshot,
  ProfileDetail,
} from "@/types/ipc"
import { Button } from "@/components/ui/button"
import { ConfirmDialog } from "@/components/ui/confirm-dialog"
import { Switch } from "@/components/ui/switch"
import { PluginImportPickerDialog } from "@/components/profiles/PluginImportPickerDialog"
import { McpManager } from "@/components/profiles/McpManager"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

export function ProfileDetailPane({
  name,
  isDefault,
  isRunning,
  isSwitching = false,
  busy: _busy,
  onLaunch: _onLaunch,
  onRestart: _onRestart,
  onSetDefault,
  onNotice,
}: {
  name: string | null
  isDefault: boolean
  isRunning: boolean
  isSwitching?: boolean
  busy: boolean
  onLaunch: () => void
  onRestart: () => void
  onSetDefault: () => void
  onCopy: () => void
  onRename: () => void
  onDelete: () => void
  onNotice: (text: string, kind?: "ok" | "warn") => void
}) {
  const { t } = useI18n()
  const [tab, setTab] = useState<"plugins" | "bundles" | "patch" | "mcp">("plugins")
  const [detail, setDetail] = useState<ProfileDetail | null>(null)
  const [plugins, setPlugins] = useState<PluginEntry[] | null>(null)
  const [runtime, setRuntime] = useState<PluginRuntimeSnapshot | null>(null)
  const [rows, setRows] = useState<PluginRowState[] | null>(null)
  const [error, setError] = useState<string | null>(null)

  // 插件搜索过滤
  const [searchQuery, setSearchQuery] = useState("")

  // 操作状态
  const [opBusy, setOpBusy] = useState<string | null>(null)
  const [installOpen, setInstallOpen] = useState(false)
  const [installSpec, setInstallSpec] = useState("")
  const [installError, setInstallError] = useState<string | null>(null)
  const [importOpen, setImportOpen] = useState(false)

  // 更新检查状态
  const [updateMap, setUpdateMap] = useState<Record<string, string> | null>(null)
  const [checkState, setCheckState] = useState<"idle" | "busy" | "done">("idle")
  const [checkMeta, setCheckMeta] = useState<{ checked: number; failed: number } | null>(null)

  // 版本选择对话框
  const [versionPick, setVersionPick] = useState<{
    pkg: string
    current: string
    latest: string
    items: string[] | null
  } | null>(null)
  const [versionsError, setVersionsError] = useState<string | null>(null)

  // YAML 复制反馈
  const { copied: copiedYaml, copy: copyYamlText } = useCopy()

  // 卸载确认（2026-09-08，U9）：卸载不可撤销，先过确认对话框再执行。
  const [confirmRemove, setConfirmRemove] = useState<string | null>(null)

  const reload = useCallback(() => {
    if (!name) return
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
    setInstallOpen(false)
    setInstallSpec("")
    setInstallError(null)
    setSearchQuery("")
    setUpdateMap(null)
    setCheckState("idle")
    reload()
  }, [name, reload])

  // 卸载 / 更新插件
  const runOp = (op: "remove" | "update", pkg: string) => {
    if (!name || opBusy) return
    setOpBusy(`${op}:${pkg}`)
    const call = op === "remove" ? api.removePlugin : api.updatePlugin
    call(name, pkg)
      .then((out) => {
        if (out.ok) {
          onNotice(out.detail, "ok")
          reload()
        } else {
          onNotice(out.detail, "warn")
        }
      })
      .catch((e) => onNotice(String(e), "warn"))
      .finally(() => setOpBusy(null))
  }

  // 卸载入口：先确认（U9）。确认后立即关框、由行内 busy 态回报进度。
  const requestRemove = (pkg: string) => {
    if (!name || opBusy) return
    setConfirmRemove(pkg)
  }

  // 安装插件
  const submitInstall = () => {
    if (!name || opBusy) return
    const spec = installSpec.trim()
    const invalid = validatePluginSpec(spec)
    if (invalid) {
      setInstallError(invalid)
      return
    }
    setInstallError(null)
    setOpBusy("install")
    api
      .installPlugin(name, spec)
      .then((out) => {
        if (out.ok) {
          onNotice(out.detail, "ok")
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

  // 启停插件（通过现代 Switch 切换）；补丁包（ADR 第七次修订）= 全部贡献行，
  // 同一 patch 文件读改写必须串行（并发 invoke 相互覆盖，2026-09-08）。
  const toggleDisabled = (pkg: string) => {
    if (!name || opBusy) return
    const row = rows?.find((r) => r.pkg_name === pkg)
    if (!row) return
    const targets = pluginToggleTargets(row)
    if (targets.length === 0) return
    setOpBusy(`toggle:${pkg}`)
    const next = !row.shell_disabled
    targets
      .reduce<Promise<void>>(
        (acc, id) => acc.then(() => api.setPluginDisabled(name, id, next)),
        Promise.resolve(),
      )
      .then(() => {
        onNotice(
          row.shell_disabled
            ? `已启用 ${pkg}（重启该 Profile 后生效）`
            : `已禁用 ${pkg}（重启该 Profile 后生效）`,
          "ok",
        )
        api
          .getPluginRows(name)
          .then((r) => setRows(r))
          .catch(() => {})
      })
      .catch((e) => onNotice(String(e), "warn"))
      .finally(() => setOpBusy(null))
  }

  // 更新检查
  const runUpdateCheck = () => {
    if (!name || opBusy || checkState === "busy") return
    setCheckState("busy")
    api
      .checkPluginUpdates(name)
      .then((r) => {
        setUpdateMap(Object.fromEntries(r.updates.map((u) => [u.name, u.latest])))
        setCheckMeta({ checked: r.checked, failed: r.failed })
        setCheckState("done")
      })
      .catch((e) => {
        onNotice(String(e), "warn")
        setCheckState("idle")
      })
  }

  // 版本弹窗
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
    setOpBusy("install")
    api
      .installPlugin(name, spec)
      .then((out) => {
        if (out.ok) {
          onNotice(out.detail, "ok")
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

  const copyYaml = async () => {
    if (!detail?.patch_yaml) return
    // 2026-09-08：写失败不再静默（原来只挂 .then 成功分支）
    const outcome = await copyYamlText(detail.patch_yaml)
    if (outcome.ok) onNotice(t.profiles.copyPatchSuccess, "ok")
    else onNotice(t.error.copyFailed, "warn")
  }

  const liveEntries =
    runtime !== null && runtime.profile !== null && runtime.profile === name
      ? runtime.entries
      : []
  const deps = useMemo(
    () => plugins?.filter((p) => p.kind === "dependency") ?? [],
    [plugins],
  )
  const depCount = deps.length

  const filteredDeps = useMemo(() => {
    if (!searchQuery.trim()) return deps
    const q = searchQuery.toLowerCase().trim()
    return deps.filter(
      (d) =>
        d.name.toLowerCase().includes(q) ||
        (d.description && d.description.toLowerCase().includes(q)),
    )
  }, [deps, searchQuery])

  const depNames = new Set(deps.map((d) => d.name))
  const layerBundles = detail?.bundles.filter((b) => !depNames.has(b)) ?? []
  const hiddenLayers = (detail?.bundles.length ?? 0) - layerBundles.length

  if (!name) {
    return (
      <div className="flex h-full min-h-[460px] flex-col items-center justify-center rounded-2xl border border-dashed border-line bg-panel/50 p-8 text-center">
        <div className="mb-3 flex size-12 items-center justify-center rounded-2xl bg-line-soft text-faint">
          <Package className="size-6" />
        </div>
        <h2 className="text-ink text-sm font-semibold">{t.profiles.emptySelectTitle}</h2>
        <p className="text-dim mt-1 max-w-sm text-xs leading-relaxed">
          {t.profiles.emptySelectSubtitle}
        </p>
      </div>
    )
  }

  return (
    <div className="flex h-full min-h-[540px] flex-col overflow-hidden rounded-2xl border border-line bg-panel shadow-xs">
      {/* 顶部 Profile 标题栏 */}
      <header className="border-b border-line bg-panel/90 px-5 py-4 backdrop-blur-xs">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <h2
                className="text-ink truncate font-mono text-base font-bold tracking-tight"
                title={name ?? undefined}
              >
                {name}
              </h2>
              {isSwitching ? (
                <span className="bg-info-soft text-info border border-info/30 inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-meta font-medium animate-pulse">
                  <LoaderCircle className="size-3 animate-spin text-info" />
                  <span>{isRunning ? t.profiles.reloadingWorkbench : t.profiles.launchingProfile}</span>
                </span>
              ) : isRunning ? (
                <span className="bg-ok-soft text-ok inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-meta font-medium">
                  <span className="bg-ok size-1.5 animate-pulse rounded-full" />
                  {t.profiles.runningBadge}
                </span>
              ) : null}
            </div>
            <p className="text-faint mt-0.5 text-xs">
              {detail ? (
                <>
                  <span>清单名: </span>
                  <span className="font-mono font-medium text-ink">{detail.package_name}</span>
                </>
              ) : (
                "正在加载配置档案..."
              )}
            </p>
          </div>

          {/* 顶栏快速操作 */}
          <div className="flex items-center gap-2">
            <Button
              size="sm"
              variant={isDefault ? "secondary" : "outline"}
              onClick={onSetDefault}
              disabled={isDefault}
              className={`gap-1 text-xs ${
                isDefault
                  ? "bg-brand/10 text-brand-deep border border-brand/20 font-medium cursor-default opacity-100"
                  : ""
              }`}
            >
              <Star className={`size-3.5 ${isDefault ? "text-brand-deep fill-current" : ""}`} />
              {isDefault ? t.profiles.defaultIs : t.profiles.setDefault}
            </Button>
          </div>
        </div>

        {/* 顶部 Tab 切换 */}
        <div
          role="tablist"
          className="mt-4 flex rounded-xl border border-line bg-line-soft/80 p-1"
        >
          <button
            type="button"
            role="tab"
            aria-selected={tab === "plugins"}
            onClick={() => setTab("plugins")}
            className={`flex flex-1 items-center justify-center gap-1.5 rounded-lg py-1.5 text-xs font-medium transition-all ${
              tab === "plugins"
                ? "bg-panel text-ink shadow-xs"
                : "text-dim hover:text-ink"
            }`}
          >
            <Package className="size-3.5" />
            <span>{t.profiles.tabPlugins}</span>
            {depCount > 0 && (
              <span className="rounded-full bg-line px-1.5 text-meta font-mono">
                {depCount}
              </span>
            )}
          </button>

          <button
            type="button"
            role="tab"
            aria-selected={tab === "bundles"}
            onClick={() => setTab("bundles")}
            className={`flex flex-1 items-center justify-center gap-1.5 rounded-lg py-1.5 text-xs font-medium transition-all ${
              tab === "bundles"
                ? "bg-panel text-ink shadow-xs"
                : "text-dim hover:text-ink"
            }`}
          >
            <Layers className="size-3.5" />
            <span>{t.profiles.tabBundles}</span>
            {layerBundles.length > 0 && (
              <span className="rounded-full bg-line px-1.5 text-meta font-mono">
                {layerBundles.length}
              </span>
            )}
          </button>

          <button
            type="button"
            role="tab"
            aria-selected={tab === "mcp"}
            onClick={() => setTab("mcp")}
            className={`flex flex-1 items-center justify-center gap-1.5 rounded-lg py-1.5 text-xs font-medium transition-all ${
              tab === "mcp"
                ? "bg-panel text-ink shadow-xs"
                : "text-dim hover:text-ink"
            }`}
          >
            <Boxes className="size-3.5" />
            <span>{t.profiles.tabMcp}</span>
          </button>

          <button
            type="button"
            role="tab"
            aria-selected={tab === "patch"}
            onClick={() => setTab("patch")}
            className={`flex flex-1 items-center justify-center gap-1.5 rounded-lg py-1.5 text-xs font-medium transition-all ${
              tab === "patch"
                ? "bg-panel text-ink shadow-xs"
                : "text-dim hover:text-ink"
            }`}
          >
            <Code2 className="size-3.5" />
            <span>{t.profiles.tabPatch}</span>
          </button>
        </div>
      </header>

      {/* 主体工作区 */}
      <div className="flex-1 overflow-y-auto p-5">
        {error && (
          <div className="mb-4 rounded-xl bg-warn-soft p-3 text-xs text-warn">
            {error}
          </div>
        )}

        {/* ================= Tab 1: 外挂插件控制台 ================= */}
        {tab === "plugins" && (
          <div className="space-y-4">
            {/* 插件工具栏：搜索 + 安装 + 导入 + 检查更新 */}
            <div className="flex flex-wrap items-center justify-between gap-2.5">
              <div className="relative min-w-[180px] flex-1">
                <Search className="text-faint absolute inset-y-0 left-2.5 my-auto size-3.5" />
                <input
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder={t.profiles.searchPluginsPlaceholder}
                  aria-label={t.profiles.searchPluginsPlaceholder}
                  className="border-line bg-bg text-ink placeholder:text-faint focus:border-brand w-full rounded-lg border py-1.5 pr-3 pl-8 font-mono text-xs outline-none transition-colors"
                />
              </div>

              <div className="flex shrink-0 items-center gap-1.5">
                <Button
                  size="sm"
                  variant="outline"
                  disabled={opBusy !== null}
                  onClick={() => {
                    setInstallOpen((o) => !o)
                    setInstallError(null)
                  }}
                  className="gap-1 text-xs"
                >
                  <Plus className="size-3.5" />
                  {t.profiles.pluginInstallBtn}
                </Button>

                <Button
                  size="sm"
                  variant="outline"
                  disabled={opBusy !== null}
                  onClick={() => setImportOpen(true)}
                  className="gap-1 text-xs"
                >
                  <Import className="size-3.5" />
                  {t.profiles.importBtn}
                </Button>

                <Button
                  size="sm"
                  variant="outline"
                  title={t.profiles.checkUpdatesBtn}
                  disabled={opBusy !== null || depCount === 0 || checkState === "busy"}
                  onClick={runUpdateCheck}
                  className="gap-1.5 text-xs font-medium"
                >
                  {checkState === "busy" ? (
                    <LoaderCircle className="size-3.5 animate-spin text-brand-deep" />
                  ) : (
                    <RefreshCw className="size-3.5" />
                  )}
                  <span>{checkState === "busy" ? t.profiles.checkingBtn : t.profiles.checkUpdatesBtn}</span>
                </Button>
              </div>
            </div>

            {/* 内联安装行 */}
            {installOpen && (
              <div className="rounded-xl border border-brand/40 bg-wash/30 p-3">
                <div className="flex gap-2">
                  <input
                    autoFocus
                    disabled={opBusy !== null}
                    aria-label={t.profiles.pluginInstallPlaceholder}
                    value={installSpec}
                    onChange={(e) => setInstallSpec(e.target.value)}
                    onKeyDown={(e) => e.key === "Enter" && submitInstall()}
                    placeholder={t.profiles.pluginInstallPlaceholder}
                    className="border-line bg-panel text-ink placeholder:text-faint focus:border-brand min-w-0 flex-1 rounded-lg border px-3 py-1.5 font-mono text-xs outline-none transition-colors"
                  />
                  <Button
                    size="sm"
                    disabled={opBusy !== null}
                    onClick={submitInstall}
                    className="shrink-0"
                  >
                    {opBusy === "install" ? (
                      <LoaderCircle className="size-3.5 animate-spin" />
                    ) : (
                      t.profiles.pluginInstallSubmit
                    )}
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={opBusy !== null}
                    onClick={() => {
                      setInstallOpen(false)
                      setInstallSpec("")
                      setInstallError(null)
                    }}
                  >
                    {t.profiles.pluginInstallCancel}
                  </Button>
                </div>
                {installError && (
                  <p className="text-warn mt-2 text-xs">{installError}</p>
                )}
              </div>
            )}

            {/* 更新检查结论 */}
            {checkState === "done" && checkMeta && (
              <div className="text-faint flex items-center justify-between rounded-lg bg-bg px-3 py-1.5 text-label">
                <span>{t.profiles.updateChecked(checkMeta)}</span>
                {updateMap && Object.keys(updateMap).length === 0 && (
                  <span className="text-ok font-medium">{t.profiles.allUpToDate}</span>
                )}
              </div>
            )}

            {/* 运行态概要 */}
            {liveEntries.length > 0 ? (
              <div className="text-faint px-1 text-label">
                {t.profiles.runtimeSummary(runtimeSummary(liveEntries))}
              </div>
            ) : null}

            {/* 插件列表 */}
            {plugins === null ? (
              <div className="text-faint py-12 text-center text-xs">
                <LoaderCircle className="mx-auto mb-2 size-5 animate-spin text-brand-deep" />
                {t.profiles.busyShort}
              </div>
            ) : depCount === 0 ? (
              <div className="rounded-xl border border-dashed border-line bg-bg p-8 text-center text-xs text-faint">
                {t.profiles.detailEmptyDeps}
              </div>
            ) : filteredDeps.length === 0 ? (
              <div className="rounded-xl border border-dashed border-line bg-bg p-8 text-center text-xs text-faint">
                未匹配到搜索词对应的已装插件
              </div>
            ) : (
              <div className="divide-y divide-line rounded-xl border border-line bg-panel shadow-xs">
                {filteredDeps.map((p) => {
                  const spec = detail?.dependencies[p.name]
                  const chip = runtimeChipFor(p.name, liveEntries)
                  const rowBusy =
                    opBusy === `remove:${p.name}` ||
                    opBusy === `update:${p.name}` ||
                    opBusy === `toggle:${p.name}`
                  const row = rows?.find((r) => r.pkg_name === p.name)
                  const shellDisabled = row?.shell_disabled ?? false
                  const latest = updateMap?.[p.name]

                  return (
                    <div
                      key={p.name}
                      className={`group flex items-center justify-between gap-3 p-3.5 transition-colors hover:bg-wash/30 ${
                        shellDisabled ? "bg-bg/40" : ""
                      }`}
                    >
                      <div className="min-w-0 flex-1">
                        <div className="flex flex-wrap items-center gap-2">
                          <span
                            className={`font-mono text-xs font-semibold ${
                              shellDisabled ? "line-through text-faint" : "text-ink"
                            }`}
                            title={p.name}
                          >
                            {p.name}
                          </span>

                          <span className="font-mono text-xs text-faint">
                            {p.installed_version ?? (spec ? t.profiles.pluginNotInstalled : "")}
                          </span>

                          {/* 升级提示 */}
                          {latest && latest !== p.installed_version && (
                            <button
                              type="button"
                              disabled={opBusy !== null}
                              onClick={() =>
                                openVersionPick(p.name, p.installed_version ?? "", latest)
                              }
                              className="text-brand-deep hover:bg-wash inline-flex items-center gap-1 rounded-full border border-brand/30 bg-brand/5 px-2 py-0.5 font-mono text-meta font-medium transition-colors"
                            >
                              <ArrowUpCircle className="size-3" />
                              <span>{latest}</span>
                            </button>
                          )}

                          {/* 运行态徽标 */}
                          {!shellDisabled && chip && (
                            <span
                              className={`rounded-md px-1.5 py-0.5 text-meta leading-none ${
                                chip.failed
                                  ? "bg-warn-soft text-warn"
                                  : "bg-ok-soft text-ok font-medium"
                              }`}
                            >
                              {chip.label}
                            </span>
                          )}

                          {shellDisabled && (
                            <span className="border border-line text-faint rounded-md px-1.5 py-0.5 text-meta leading-none">
                              {t.profiles.pluginDisabled}
                            </span>
                          )}
                        </div>

                        {(p.description || spec) && (
                          <p
                            className="text-faint mt-1 truncate text-xs"
                            title={p.description ?? spec}
                          >
                            {p.description ?? spec}
                          </p>
                        )}
                      </div>

                      {/* 右侧控制：Toggle 开关 + 动作按钮 */}
                      <div className="flex shrink-0 items-center gap-2">
                        {rowBusy ? (
                          <LoaderCircle className="size-4 animate-spin text-brand-deep" />
                        ) : (
                          <>
                            {row && (
                              <div
                                className="flex items-center gap-1.5"
                                title={
                                  shellDisabled
                                    ? t.profiles.pluginEnable
                                    : t.profiles.pluginDisable
                                }
                              >
                                <Switch
                                  aria-label={`${p.name}：${
                                    shellDisabled
                                      ? t.profiles.pluginEnable
                                      : t.profiles.pluginDisable
                                  }`}
                                  checked={!shellDisabled}
                                  disabled={opBusy !== null}
                                  onCheckedChange={() => toggleDisabled(p.name)}
                                />
                              </div>
                            )}

                            <button
                              type="button"
                              title={t.profiles.pluginUpdate}
                              disabled={opBusy !== null}
                              onClick={() => runOp("update", p.name)}
                              className="text-faint hover:text-ink hover:bg-line-soft inline-flex size-7 items-center justify-center rounded-lg transition-colors"
                            >
                              <ArrowUpCircle className="size-3.5" />
                            </button>

                            <button
                              type="button"
                              title={t.profiles.pluginUninstall}
                              disabled={opBusy !== null}
                              onClick={() => requestRemove(p.name)}
                              className="text-faint hover:text-warn hover:bg-warn-soft inline-flex size-7 items-center justify-center rounded-lg transition-colors"
                            >
                              <Trash2 className="size-3.5" />
                            </button>
                          </>
                        )}
                      </div>
                    </div>
                  )
                })}
              </div>
            )}
          </div>
        )}

        {/* ================= Tab 2: 底座组合架构 ================= */}
        {tab === "bundles" && (
          <div className="space-y-3">
            {detail?.package_name === "@deepseek-ai/dsh-desktop-runtime" && (
              <div className="rounded-xl border border-line bg-wash/50 p-4 space-y-2">
                <div className="flex items-center gap-2">
                  <span className="font-mono text-xs font-semibold text-ink">
                    DeepSeek 官方桌面客户端底座运行时
                  </span>
                  <span className="bg-brand/10 text-brand-deep border border-brand/20 rounded-md px-1.5 py-0.5 text-meta font-medium">
                    官方桌面版
                  </span>
                </div>
                <p className="text-dim text-xs leading-relaxed">
                  该 Profile 归属于 DeepSeek 官方桌面客户端（<code>@deepseek-ai/dsh-desktop-runtime</code>），内含 240+ 项本地预置底座服务与核心组件（存放在 <code>desktop-packages/</code> 本地包目录）。
                </p>
                <p className="text-faint text-meta">
                  注：这些核心组件由客户端底座统一部署维护，属于内置底座体系，不计入第三方外挂插件。在此 Profile 安装的自定义扩展将独立展示在「外挂插件」列表中。
                </p>
              </div>
            )}

            <p className="text-dim text-xs leading-relaxed">
              底座组合由 Profile 初始化时写入（<code>dsh.profile.bundles</code>
              ），定义了工作台的基础界面宿主与系统能力：
            </p>

            <div className="grid gap-2.5">
              {layerBundles.map((b) => {
                const chip = runtimeChipFor(b, liveEntries)
                const isBase = b === "@deepseek-ai/dsh-base"
                return (
                  <div
                    key={b}
                    className="flex items-center justify-between rounded-xl border border-line bg-bg p-3.5"
                  >
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-ink font-mono text-xs font-semibold">
                          {b}
                        </span>
                        {isBase && (
                          <span className="bg-line-soft text-dim rounded-md px-1.5 py-0.5 text-meta">
                            系统核心
                          </span>
                        )}
                      </div>
                      <p className="text-faint mt-0.5 text-xs font-mono">
                        {isBase
                          ? "Cordis 底座与通用服务插件集合"
                          : "Web 界面与交互控制台渲染器"}
                      </p>
                    </div>

                    {chip && (
                      <span
                        className={`rounded-md px-2 py-0.5 text-meta font-medium leading-none ${
                          chip.failed ? "bg-warn-soft text-warn" : "bg-ok-soft text-ok"
                        }`}
                      >
                        {chip.label}
                      </span>
                    )}
                  </div>
                )
              })}
            </div>

            {hiddenLayers > 0 && (
              <p className="text-faint text-xs">
                {t.profiles.hiddenLayersHint(hiddenLayers)}
              </p>
            )}
          </div>
        )}

        {/* ================= Tab 3: MCP 扩展服务器 ================= */}
        {tab === "mcp" && (
          <McpManager
            profileName={name}
            patchYaml={detail?.patch_yaml ?? null}
            onNotice={onNotice}
          />
        )}

        {/* ================= Tab 4: Patch YAML 原文视窗 ================= */}
        {tab === "patch" && (
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-faint text-xs">{t.profiles.rawYamlHint}</span>
              {detail?.patch_yaml && (
                <Button
                  size="sm"
                  variant="outline"
                  onClick={copyYaml}
                  className="gap-1.5 text-xs"
                >
                  {copiedYaml ? (
                    <Check className="size-3.5 text-ok" />
                  ) : (
                    <Copy className="size-3.5" />
                  )}
                  {copiedYaml ? "已复制" : "复制 YAML"}
                </Button>
              )}
            </div>

            {detail?.patch_yaml ? (
              <div className="relative overflow-hidden rounded-xl border border-term-line bg-term p-4 font-mono text-xs text-term-ink">
                <pre className="max-h-[420px] overflow-auto leading-relaxed whitespace-pre font-mono selection:bg-brand/30">
                  {detail.patch_yaml}
                </pre>
              </div>
            ) : (
              <div className="rounded-xl border border-dashed border-line bg-bg p-8 text-center text-xs text-faint">
                {t.profiles.detailPatchNone}
              </div>
            )}
          </div>
        )}
      </div>

      {/* 从其他 profile 导入选择器 */}
      <PluginImportPickerDialog
        target={name ?? ""}
        open={importOpen && name !== null}
        onClose={() => setImportOpen(false)}
        onDone={(ok, fail) => {
          onNotice(t.profiles.importDone(ok, fail), ok > 0 ? "ok" : "warn")
          reload()
        }}
      />

      {/* 版本选择弹窗 */}
      <Dialog open={versionPick !== null} onOpenChange={(o) => !o && setVersionPick(null)}>
        <DialogContent className="flex max-h-[calc(100vh-6rem)] flex-col sm:max-w-[400px]">
          <DialogHeader>
            <DialogTitle className="text-sm font-semibold">
              {versionPick ? t.profiles.pickVersionTitle(versionPick.pkg) : ""}
            </DialogTitle>
            <DialogDescription className="sr-only">
              {versionPick ? t.profiles.pickVersionTitle(versionPick.pkg) : ""}
            </DialogDescription>
          </DialogHeader>

          {versionsError && (
            <div className="rounded-lg bg-warn-soft px-3 py-2 text-xs text-warn">
              {versionsError}
            </div>
          )}

          {versionPick?.items === null && !versionsError && (
            <div className="text-faint py-6 text-center text-xs">
              <LoaderCircle className="mx-auto mb-2 size-4 animate-spin text-brand-deep" />
              {t.profiles.busyShort}
            </div>
          )}

          {versionPick?.items && (
            <div className="min-h-0 flex-1 overflow-y-auto pr-1">
              <div className="divide-y divide-line rounded-lg border border-line bg-bg">
                {versionPick.items.map((v) => {
                  const isLatest = v === versionPick.latest
                  const isCurrent = v === versionPick.current
                  return (
                    <button
                      key={v}
                      type="button"
                      disabled={opBusy !== null}
                      onClick={() => installVersion(`${versionPick.pkg}@${v}`)}
                      className="hover:bg-wash flex w-full items-baseline gap-2 px-3 py-2 text-left transition-colors disabled:opacity-40"
                    >
                      <span className="text-ink font-mono text-xs">{v}</span>
                      {isLatest && (
                        <span className="bg-ok-soft text-ok rounded-md px-1.5 text-meta leading-none">
                          {t.profiles.versionLatest}
                        </span>
                      )}
                      {isCurrent && (
                        <span className="border border-line text-faint ml-auto rounded-md px-1.5 text-meta leading-none">
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

      {/* 卸载确认（U9）：不可撤销，逐条列出影响面后再执行 */}
      <ConfirmDialog
        open={confirmRemove !== null}
        title={confirmRemove ? t.profiles.pluginUninstallConfirm(confirmRemove) : ""}
        points={t.profiles.pluginUninstallPoints}
        confirmLabel={t.profiles.pluginUninstall}
        cancelLabel={t.confirm.cancel}
        onConfirm={() => {
          const pkg = confirmRemove
          setConfirmRemove(null)
          if (pkg) runOp("remove", pkg)
        }}
        onClose={() => setConfirmRemove(null)}
      />
    </div>
  )
}
