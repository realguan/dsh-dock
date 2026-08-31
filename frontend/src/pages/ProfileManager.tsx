import { useCallback, useEffect, useMemo, useState } from "react"
import {
  Layers,
  Plus,
  RefreshCw,
  Search,
  Settings,
  ShieldCheck,
  SlidersHorizontal,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { useI18n, useI18nStore } from "@/stores/i18nStore"
import { useBootStore } from "@/stores/bootStore"
import { useProfilesStore } from "@/stores/profilesStore"
import { Emblem } from "@/components/layout/Emblem"
import { PageShell } from "@/components/layout/PageShell"
import { ProfileRow } from "@/components/profiles/ProfileRow"
import { ProfileDetailPane } from "@/components/profiles/ProfileDetailPane"
import { PluginOverview } from "@/components/profiles/PluginOverview"
import { SessionManager } from "@/components/profiles/SessionManager"
import { SystemConsole } from "@/components/system/SystemConsole"
import { ProfileCreateDialog } from "@/components/profiles/ProfileCreateDialog"
import { ProfileNameDialog, type NameOpMode } from "@/components/profiles/ProfileNameDialog"
import { ProfileDeleteDialog } from "@/components/profiles/ProfileDeleteDialog"
import { ProfileSwitchDialog } from "@/components/profiles/ProfileSwitchDialog"
import { FloatingToast, type ToastMessage } from "@/components/ui/toast"
import { Button } from "@/components/ui/button"

export function ProfileManager() {
  const { t } = useI18n()
  const { list, defaultProfile, activeProfile, loading, loadError, load } = useProfilesStore()

  // 选中的 Profile（默认为当前运行中的 Profile 或第一个 Profile）
  const [selectedName, setSelectedName] = useState<string | null>(null)

  // 对话框状态
  const [createOpen, setCreateOpen] = useState(false)
  const [nameOp, setNameOp] = useState<{ mode: NameOpMode; source: string } | null>(null)
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null)
  const [switchTarget, setSwitchTarget] = useState<string | null>(null)
  const [rowBusy, setRowBusy] = useState<string | null>(null)

  // 视图切换（Profile 管理列表 vs 插件全景矩阵 vs 会话维护与自愈 vs 系统控制台）
  const [view, setView] = useState<"list" | "plugins" | "sessions" | "console">("list")
  const [overviewTick, setOverviewTick] = useState(0)

  // 初始化语言
  useEffect(() => {
    void useI18nStore.getState().initFromSettings()
  }, [])

  // Profile 搜索筛选
  const [profileFilter, setProfileFilter] = useState("")

  // 浮动通知 Toast
  const [toast, setToast] = useState<ToastMessage | null>(null)

  const showToast = useCallback((message: string, kind: "ok" | "warn" | "info" = "ok") => {
    setToast({ id: `${Date.now()}-${Math.random()}`, message, kind })
    setTimeout(() => setToast((curr) => (curr?.message === message ? null : curr)), 3500)
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  // 双视图统一刷新面
  const refreshAll = useCallback(() => {
    void load()
    setOverviewTick((n) => n + 1)
  }, [load])

  useEffect(() => {
    const onFocus = () => refreshAll()
    window.addEventListener("focus", onFocus)
    return () => window.removeEventListener("focus", onFocus)
  }, [refreshAll])

  // 会话状态感知自动刷新
  useEffect(
    () =>
      useBootStore.subscribe((s, prev) => {
        if (s.activeStep !== prev.activeStep) refreshAll()
      }),
    [refreshAll],
  )

  // 首次加载或列表变更时自动选定 Profile
  useEffect(() => {
    if (list.length === 0) return
    if (!selectedName || !list.some((p) => p.name === selectedName)) {
      const preferred =
        list.find((p) => p.name === activeProfile)?.name ??
        list.find((p) => p.name === defaultProfile)?.name ??
        list[0]?.name ??
        null
      setSelectedName(preferred)
    }
  }, [list, activeProfile, defaultProfile, selectedName])

  // 设为默认
  const handleSetDefault = (name: string) => {
    setRowBusy(name)
    api
      .setDefaultProfile(name)
      .then(() => {
        showToast(t.profiles.setDefaultDone(name), "ok")
        refreshAll()
      })
      .catch((e) => showToast(String(e), "warn"))
      .finally(() => setRowBusy(null))
  }

  // 启动 / 切换
  const handleLaunch = (name: string) => {
    if (activeProfile !== null) {
      setSwitchTarget(name)
      return
    }
    doSwitch(name)
  }

  // 重启
  const handleRestart = (name: string) => {
    setSwitchTarget(name)
  }

  const doSwitch = (name: string) => {
    setRowBusy(name)
    api
      .switchProfile(name)
      .then(() => {
        showToast(t.profiles.switchDone(name), "ok")
        refreshAll()
      })
      .catch((e) => showToast(String(e), "warn"))
      .finally(() => setRowBusy(null))
  }

  // 过滤后的 Profile 列表
  const filteredList = useMemo(() => {
    if (!profileFilter.trim()) return list
    const q = profileFilter.toLowerCase().trim()
    return list.filter((p) => p.name.toLowerCase().includes(q))
  }, [list, profileFilter])

  const currentSelectedProfile = useMemo(() => {
    return list.find((p) => p.name === selectedName) ?? null
  }, [list, selectedName])

  return (
    <PageShell width={1040} align="top" className="px-4 py-4 sm:px-6">
      {/* 顶部全局标题栏 */}
      <header className="mb-4 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <Emblem size={32} />
          <div>
            <h1 className="text-ink text-base font-bold tracking-tight">
              {t.profiles.title}
            </h1>
            <p className="text-faint text-xs">{t.profiles.subtitle}</p>
          </div>
        </div>

        {/* 顶部右侧：视图分段切换 + 刷新 */}
        <div className="flex items-center gap-2">
          <div
            role="tablist"
            className="flex rounded-xl border border-line bg-line-soft/80 p-0.5 shadow-2xs"
          >
            <button
              type="button"
              role="tab"
              aria-selected={view === "list"}
              onClick={() => setView("list")}
              className={`flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition-all ${
                view === "list"
                  ? "bg-panel text-ink shadow-xs"
                  : "text-dim hover:text-ink"
              }`}
            >
              <SlidersHorizontal className="size-3.5" />
              <span>{t.profiles.viewProfiles}</span>
            </button>

            <button
              type="button"
              role="tab"
              aria-selected={view === "plugins"}
              onClick={() => setView("plugins")}
              className={`flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition-all ${
                view === "plugins"
                  ? "bg-panel text-ink shadow-xs"
                  : "text-dim hover:text-ink"
              }`}
            >
              <Layers className="size-3.5" />
              <span>{t.profiles.viewPlugins}</span>
            </button>

            <button
              type="button"
              role="tab"
              aria-selected={view === "sessions"}
              onClick={() => setView("sessions")}
              className={`flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition-all ${
                view === "sessions"
                  ? "bg-panel text-ink shadow-xs"
                  : "text-dim hover:text-ink"
              }`}
            >
              <ShieldCheck className="size-3.5" />
              <span>{t.profiles.viewSessions}</span>
            </button>

            <button
              type="button"
              role="tab"
              aria-selected={view === "console"}
              onClick={() => setView("console")}
              className={`flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition-all ${
                view === "console"
                  ? "bg-panel text-ink shadow-xs"
                  : "text-dim hover:text-ink"
              }`}
            >
              <Settings className="size-3.5" />
              <span>{t.profiles.viewConsole}</span>
            </button>
          </div>

          <Button
            size="sm"
            variant="outline"
            title={loading ? t.profiles.refreshing : t.profiles.refresh}
            aria-label={t.profiles.refresh}
            onClick={refreshAll}
            className="size-8 p-0"
          >
            <RefreshCw className={`size-3.5 ${loading ? "animate-spin text-brand" : ""}`} />
          </Button>
        </div>
      </header>

      {/* 主视图区 */}
      {view === "console" ? (
        <SystemConsole
          onNotice={(msg, kind) => showToast(msg, kind)}
        />
      ) : view === "sessions" ? (
        <SessionManager
          refreshKey={overviewTick}
          onNotice={(msg, kind) => showToast(msg, kind)}
        />
      ) : view === "plugins" ? (
        <PluginOverview
          refreshKey={overviewTick}
          onNotice={(msg, kind) => showToast(msg, kind)}
        />
      ) : (
        <div className="grid grid-cols-1 gap-6 lg:grid-cols-12">
          {/* 左侧 List：Profile 列表导航 */}
          <section
            aria-label="Profile 列表"
            className="space-y-3 lg:col-span-4 xl:col-span-4"
          >
            {/* 新建 Profile 专属醒目操作条 */}
            <Button
              onClick={() => setCreateOpen(true)}
              className="w-full gap-1.5 bg-brand text-white hover:bg-brand/90 text-xs shadow-xs h-9 rounded-xl font-medium"
            >
              <Plus className="size-4" />
              <span>{t.profiles.createBtn}</span>
            </Button>

            {/* 搜索框 */}
            <div className="relative">
              <Search className="text-faint absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2" />
              <input
                value={profileFilter}
                onChange={(e) => setProfileFilter(e.target.value)}
                placeholder={t.profiles.searchPlaceholder}
                className="border-line bg-panel text-ink placeholder:text-faint focus:border-brand w-full rounded-xl border py-1.5 pr-3 pl-8 text-xs outline-none shadow-2xs transition-colors"
              />
            </div>

            {loadError && (
              <div className="rounded-xl border border-dashed border-line bg-panel p-6 text-center">
                <p className="text-dim mb-2 text-xs">{loadError}</p>
                <Button size="sm" variant="outline" onClick={refreshAll}>
                  <RefreshCw className="mr-1 size-3" />
                  {t.profiles.retryLoad}
                </Button>
              </div>
            )}

            {!loadError && (
              <div className="space-y-2">
                {filteredList.map((p, i) => (
                  <ProfileRow
                    key={p.name}
                    profile={p}
                    index={i}
                    isSelected={selectedName === p.name}
                    isDefault={defaultProfile === p.name}
                    isRunning={activeProfile === p.name}
                    busy={rowBusy === p.name}
                    onSelect={() => setSelectedName(p.name)}
                    onDetail={() => setSelectedName(p.name)}
                    onSetDefault={() => handleSetDefault(p.name)}
                    onLaunch={() => handleLaunch(p.name)}
                    onRestart={() => handleRestart(p.name)}
                    onRename={() => setNameOp({ mode: "rename", source: p.name })}
                    onCopy={() => setNameOp({ mode: "copy", source: p.name })}
                    onDelete={() => setDeleteTarget(p.name)}
                  />
                ))}
                {filteredList.length === 0 && !loading && (
                  <div className="rounded-xl border border-dashed border-line bg-panel/50 p-6 text-center text-xs text-faint">
                    未找到匹配的 Profile
                  </div>
                )}
              </div>
            )}
          </section>

          {/* 右侧 Detail：选中的 Profile 工作台面板 */}
          <section
            aria-label="Profile 详情工作区"
            className="min-h-[560px] lg:col-span-8 xl:col-span-8"
          >
            <ProfileDetailPane
              name={currentSelectedProfile?.name ?? null}
              isDefault={defaultProfile === currentSelectedProfile?.name}
              isRunning={activeProfile === currentSelectedProfile?.name}
              busy={rowBusy === currentSelectedProfile?.name}
              onLaunch={() => currentSelectedProfile && handleLaunch(currentSelectedProfile.name)}
              onRestart={() =>
                currentSelectedProfile && handleRestart(currentSelectedProfile.name)
              }
              onSetDefault={() =>
                currentSelectedProfile && handleSetDefault(currentSelectedProfile.name)
              }
              onCopy={() =>
                currentSelectedProfile &&
                setNameOp({ mode: "copy", source: currentSelectedProfile.name })
              }
              onRename={() =>
                currentSelectedProfile &&
                setNameOp({ mode: "rename", source: currentSelectedProfile.name })
              }
              onDelete={() =>
                currentSelectedProfile && setDeleteTarget(currentSelectedProfile.name)
              }
              onNotice={(msg, kind) => showToast(msg, kind)}
            />
          </section>
        </div>
      )}

      {/* 浮动 Toast 通知 */}
      <FloatingToast toast={toast} onDismiss={() => setToast(null)} />

      {/* 模态对话框群 */}
      <ProfileSwitchDialog
        target={switchTarget}
        active={activeProfile}
        restart={switchTarget !== null && switchTarget === activeProfile}
        onClose={() => setSwitchTarget(null)}
        onDone={() => {
          showToast(t.profiles.switchDone(switchTarget ?? ""), "ok")
          refreshAll()
        }}
      />

      <ProfileCreateDialog
        open={createOpen}
        existing={list}
        onClose={() => setCreateOpen(false)}
        onRefresh={refreshAll}
      />

      <ProfileNameDialog
        mode={nameOp?.mode ?? "copy"}
        source={nameOp?.source ?? null}
        existing={list}
        onClose={() => setNameOp(null)}
        onRefresh={refreshAll}
        onDone={(newName, warnings) => {
          if (warnings.length > 0) {
            showToast(warnings.join(" "), "warn")
          } else if (nameOp?.mode === "rename") {
            showToast(t.profiles.renameDone(newName), "ok")
          } else {
            showToast(t.profiles.copyDone(newName), "ok")
          }
        }}
      />

      <ProfileDeleteDialog
        name={deleteTarget}
        onClose={() => setDeleteTarget(null)}
        onRefresh={refreshAll}
        onDone={(defaultCleared) => {
          showToast(
            defaultCleared ? t.profiles.deleteDoneCleared : t.profiles.deleteDone,
            defaultCleared ? "warn" : "ok",
          )
          if (selectedName === deleteTarget) {
            setSelectedName(null)
          }
        }}
      />
    </PageShell>
  )
}
