import { useCallback, useEffect, useMemo, useState } from "react"
import {
  Plus,
  RefreshCw,
  Search,
  Settings,
  ShieldCheck,
  SlidersHorizontal,
  Store,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { useI18n, useI18nStore } from "@/stores/i18nStore"
import { useBootStore } from "@/stores/bootStore"
import { useProfilesStore } from "@/stores/profilesStore"
import { useQueueStore } from "@/stores/queueStore"
import { Emblem } from "@/components/layout/Emblem"
import { PageShell } from "@/components/layout/PageShell"
import { ProfileRow } from "@/components/profiles/ProfileRow"
import { ProfileDetailPane } from "@/components/profiles/ProfileDetailPane"
import { PluginHub } from "@/components/market/PluginHub"
import { SessionManager } from "@/components/profiles/SessionManager"
import { SystemConsole } from "@/components/system/SystemConsole"
import { ProfileCreateDialog } from "@/components/profiles/ProfileCreateDialog"
import { ProfileNameDialog, type NameOpMode } from "@/components/profiles/ProfileNameDialog"
import { ProfileDeleteDialog } from "@/components/profiles/ProfileDeleteDialog"
import { ProfileSwitchDialog } from "@/components/profiles/ProfileSwitchDialog"
import { FloatingToast, type ToastMessage } from "@/components/ui/toast"
import { Button } from "@/components/ui/button"
import { QuickDshSwitcher } from "@/components/layout/QuickDshSwitcher"

export function ProfileManager() {
  const { t } = useI18n()
  const { list, defaultProfile, activeProfile, loading, loadError, load } = useProfilesStore()

  // 选中的 Profile（默认为当前运行中的 Profile 或第一个 Profile）
  const [selectedName, setSelectedName] = useState<string | null>(() => {
    return new URLSearchParams(window.location.search).get("selected") || null
  })

  // 对话框状态
  const [createOpen, setCreateOpen] = useState(() => {
    return new URLSearchParams(window.location.search).get("dialog") === "create"
  })
  const [nameOp, setNameOp] = useState<{ mode: NameOpMode; source: string } | null>(() => {
    const d = new URLSearchParams(window.location.search).get("dialog")
    const src = new URLSearchParams(window.location.search).get("target") || "default"
    if (d === "copy" || d === "rename") return { mode: d, source: src }
    return null
  })
  const [deleteTarget, setDeleteTarget] = useState<string | null>(() => {
    // 深链 `?dialog=delete&target=<name>`：缺 target 时不弹窗（不臆造默认目标，
    // 否则会对一个本机不存在的工作台弹出删除确认）。
    return new URLSearchParams(window.location.search).get("dialog") === "delete"
      ? new URLSearchParams(window.location.search).get("target")
      : null
  })
  const [switchTarget, setSwitchTarget] = useState<string | null>(null)
  const [rowBusy, setRowBusy] = useState<string | null>(null)
  const [switchingTarget, setSwitchingTarget] = useState<string | null>(null)
  const [isRefreshingData, setIsRefreshingData] = useState(false)

  // 视图切换（Profile 管理列表 vs 插件中心 vs 会话维护与自愈 vs 系统控制台）
  const [view, setView] = useState<"list" | "plugins" | "sessions" | "console">(() => {
    const p = new URLSearchParams(window.location.search).get("view")
    if (p === "plugins" || p === "sessions" || p === "console" || p === "list") return p
    return "list"
  })
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

  // 安装队列通知接线（095 #4：入队/完成/失败经 toast 冒泡；下载管理面板见 PluginHub）
  useEffect(() => {
    useQueueStore.getState().setNotifier((text, kind) => showToast(text, kind ?? "ok"))
  }, [showToast])

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

  // 会话重载状态监听：当检测到目标 Profile 已就绪或已成功成为 activeProfile 时平滑解除过渡态
  useEffect(() => {
    if (!switchingTarget) return
    if (activeProfile === switchingTarget) {
      const timer = setTimeout(() => {
        setSwitchingTarget(null)
      }, 700)
      return () => clearTimeout(timer)
    }
  }, [switchingTarget, activeProfile])

  // 重载探活轮询（安全守卫：切换期间每秒探活，最长 15 秒兜底，防死锁）
  useEffect(() => {
    if (!switchingTarget) return
    const interval = setInterval(() => {
      void load()
    }, 1200)
    const timeout = setTimeout(() => {
      setSwitchingTarget(null)
    }, 15000)
    return () => {
      clearInterval(interval)
      clearTimeout(timeout)
    }
  }, [switchingTarget, load])

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
    setSwitchingTarget(name)
    setRowBusy(name)
    useProfilesStore.setState({ activeProfile: null })
    api
      .switchProfile(name)
      .then(() => {
        showToast(t.profiles.switchDone(name), "ok")
        refreshAll()
      })
      .catch((e) => {
        showToast(String(e), "warn")
        setSwitchingTarget(null)
      })
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
      {/* 顶部全局标题栏（单行弹性布局，右侧控制区 shrink-0 防止 Hover 展开时发生折行抖动）。
          2026-09-08 批次 D / U10：吸顶——长列表滚动后「视图切换」入口不再消失；
          负外边距抵消 PageShell 的 px-4/6/8，半透明底 + 模糊保证滚动内容不穿透。 */}
      <header className="sticky top-0 z-20 -mx-4 mb-4 flex items-center justify-between gap-3 bg-bg/90 px-4 py-2 backdrop-blur-sm sm:-mx-6 sm:px-6 md:-mx-8 md:px-8">
        <div className="flex items-center gap-3.5 min-w-0">
          <Emblem size={48} framed={true} />
          <div className="min-w-0">
            <h1 className="text-ink text-base sm:text-lg font-bold tracking-tight truncate" title={t.profiles.title}>
              {t.profiles.title}
            </h1>
            <p className="text-faint text-xs truncate" title={t.profiles.subtitle}>
              {t.profiles.subtitle}
            </p>
          </div>
        </div>

        {/* 顶部右侧：视图分段切换 + 刷新 + 胶囊 */}
        <div className="flex items-center gap-2 shrink-0">
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
              <Store className="size-3.5" />
              <span>{t.profiles.viewPluginHub}</span>
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

          {/* 控制台数据与状态手动刷新（带旋转反馈与清晰语义） */}
          <Button
            size="sm"
            variant="outline"
            title={t.profiles.refreshData}
            aria-label={t.profiles.refreshData}
            onClick={() => {
              setIsRefreshingData(true)
              refreshAll()
              showToast(t.profiles.dataRefreshed, "ok")
              setTimeout(() => setIsRefreshingData(false), 600)
            }}
            disabled={isRefreshingData}
            className="size-8 p-0 rounded-xl"
          >
            <RefreshCw className={`size-3.5 ${isRefreshingData ? "animate-spin text-brand-deep" : ""}`} />
          </Button>

          {/* 返回 DSH 主工作台操作入口 */}
          <QuickDshSwitcher />
        </div>
      </header>

      {/* 主视图区。
          2026-09-08 裁定：onNotice 必须传 useCallback 稳定的引用（此处即 showToast
          本身），禁止写成 `(msg, kind) => showToast(msg, kind)` 内联箭头——内联每次
          渲染都是新引用，子面板 `useCallback([onNotice])` 随之失效 → `useEffect`
          重跑 → 加载失败又 onNotice → setToast → 父重渲染 → 死循环（McpManager
          踩过同坑，见其 `:108` 注释）。传引用，不传包装。 */}
      {view === "console" ? (
        <SystemConsole onNotice={showToast} />
      ) : view === "sessions" ? (
        <SessionManager refreshKey={overviewTick} onNotice={showToast} />
      ) : view === "plugins" ? (
        <PluginHub refreshKey={overviewTick} onNotice={showToast} />
      ) : (
        <div className="grid grid-cols-1 gap-6 md:grid-cols-12">
          {/* 左侧 List：Profile 列表导航 */}
          <section
            aria-label={t.profiles.listLabel}
            className="space-y-3 md:col-span-4 xl:col-span-4"
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
              <Search className="text-faint absolute inset-y-0 left-2.5 my-auto size-3.5" />
              <input
                value={profileFilter}
                onChange={(e) => setProfileFilter(e.target.value)}
                placeholder={t.profiles.searchPlaceholder}
                aria-label={t.profiles.searchPlaceholder}
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
                    isSwitching={switchingTarget === p.name}
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
            aria-label={t.profiles.detailWorkspaceLabel}
            className="min-h-[560px] md:col-span-8 xl:col-span-8"
          >
            <ProfileDetailPane
              name={currentSelectedProfile?.name ?? null}
              isDefault={defaultProfile === currentSelectedProfile?.name}
              isRunning={activeProfile === currentSelectedProfile?.name}
              isSwitching={switchingTarget === currentSelectedProfile?.name}
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
              onNotice={showToast}
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
          if (switchTarget) {
            setSwitchingTarget(switchTarget)
            useProfilesStore.setState({ activeProfile: null })
          }
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
