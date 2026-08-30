// Profile 管理器页（4.3 前端刀）。独立窗口（label=profiles，镜像 about 的
// 「常驻入口在菜单/托盘」架构——主窗口 boot 后会导航进 dsh 工作台，壳页
// 不可达）。编排：列表/默认值经 profilesStore 播种；增删改走 api 后刷新；
// 详情与各对话框状态均为对话框局部态。4.4④ 收口：页内两视图切换——
// Profile 列表 | 插件总览（跨 profile 第三方插件聚合，只读）。
import { useCallback, useEffect, useState } from "react"
import { Plus, RefreshCw } from "lucide-react"
import { api } from "@/lib/tauri"
import { t } from "@/content/zh-CN"
import { useBootStore } from "@/stores/bootStore"
import { useProfilesStore } from "@/stores/profilesStore"
import { Emblem } from "@/components/layout/Emblem"
import { PageShell } from "@/components/layout/PageShell"
import { ProfileRow } from "@/components/profiles/ProfileRow"
import { PluginOverview } from "@/components/profiles/PluginOverview"
import { ProfileDetailDialog } from "@/components/profiles/ProfileDetailDialog"
import { ProfileCreateDialog } from "@/components/profiles/ProfileCreateDialog"
import { ProfileNameDialog, type NameOpMode } from "@/components/profiles/ProfileNameDialog"
import { ProfileDeleteDialog } from "@/components/profiles/ProfileDeleteDialog"
import { ProfileSwitchDialog } from "@/components/profiles/ProfileSwitchDialog"

interface Notice {
  kind: "ok" | "warn"
  text: string
}

export function ProfileManager() {
  const { list, defaultProfile, activeProfile, loading, loadError, load } = useProfilesStore()

  const [detailName, setDetailName] = useState<string | null>(null)
  const [createOpen, setCreateOpen] = useState(false)
  const [nameOp, setNameOp] = useState<{ mode: NameOpMode; source: string } | null>(null)
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null)
  const [switchTarget, setSwitchTarget] = useState<string | null>(null)
  const [rowBusy, setRowBusy] = useState<string | null>(null)
  const [notice, setNotice] = useState<Notice | null>(null)
  const [actionError, setActionError] = useState<string | null>(null)
  // 4.4④ 收口：页内视图 + 总览重取节奏（与列表同源刷新面：聚焦 / boot 事件 / 手动）
  const [view, setView] = useState<"list" | "plugins">("list")
  const [overviewTick, setOverviewTick] = useState(0)

  useEffect(() => {
    void load()
  }, [load])

  // 双视图统一刷新面：聚焦即对齐真相（切换是否完成只有壳知道）
  const refreshAll = useCallback(() => {
    void load()
    setOverviewTick((n) => n + 1)
  }, [load])

  useEffect(() => {
    const onFocus = () => refreshAll()
    window.addEventListener("focus", onFocus)
    return () => window.removeEventListener("focus", onFocus)
  }, [refreshAll])

  // 徽标实时化：boot:step 经事件总线（模块期装配，每窗口生效）流入 bootStore，
  // activeStep 每变一次重取运行中真相——切换开始（teardown）徽标即灭，boot
  // 完成即亮，不等聚焦。load 自带 loading 去重，事件密也只串行取。
  useEffect(
    () =>
      useBootStore.subscribe((s, prev) => {
        if (s.activeStep !== prev.activeStep) refreshAll()
      }),
    [refreshAll],
  )

  const refresh = refreshAll

  const handleSetDefault = (name: string) => {
    setRowBusy(name)
    setActionError(null)
    api
      .setDefaultProfile(name)
      .then(() => {
        setNotice({ kind: "ok", text: t.profiles.setDefaultDone(name) })
        refresh()
      })
      .catch((e) => setActionError(String(e)))
      .finally(() => setRowBusy(null))
  }

  // 切换入口（4.3⑥）：有活跃会话先确认（停 dsh 有中断代价），无会话直接切
  const handleLaunch = (name: string) => {
    setActionError(null)
    if (activeProfile !== null) {
      setSwitchTarget(name)
      return
    }
    doSwitch(name)
  }

  // 重启（4.4③）：同 profile 走切换链（teardown → 原样重起）；恒弹确认——
  // 重启必杀运行中会话。弹窗文案按重启语义分叉（isRestart）。
  const handleRestart = (name: string) => {
    setActionError(null)
    setSwitchTarget(name)
  }

  const doSwitch = (name: string) => {
    setRowBusy(name)
    api
      .switchProfile(name)
      .then(() => {
        setNotice({ kind: "ok", text: t.profiles.switchDone(name) })
        refresh()
      })
      .catch((e) => setActionError(String(e)))
      .finally(() => setRowBusy(null))
  }

  return (
    <PageShell width={620} align="top">
      {/* 头部：徽章 + 标题 + 动作（新建 / 刷新） */}
      <header className="mb-4 flex items-center gap-3">
        <Emblem size={44} />
        <div className="min-w-0 flex-1">
          <div className="text-ink text-lg font-semibold tracking-tight">{t.profiles.title}</div>
          <div className="text-faint truncate text-xs">{t.profiles.subtitle}</div>
        </div>
        <button
          type="button"
          onClick={() => setCreateOpen(true)}
          className="border-line text-dim hover:border-brand hover:text-brand inline-flex items-center gap-1 rounded-lg border bg-white px-2.5 py-1.5 text-xs transition-colors"
        >
          <Plus className="size-3.5" />
          {t.profiles.createBtn}
        </button>
        <button
          type="button"
          title={loading ? t.profiles.refreshing : t.profiles.refresh}
          aria-label={t.profiles.refresh}
          onClick={refresh}
          className="border-line text-dim hover:text-ink inline-flex size-8 items-center justify-center rounded-lg border bg-white transition-colors"
        >
          <RefreshCw className={`size-3.5 ${loading ? "animate-spin" : ""}`} />
        </button>
      </header>

      {/* 视图切换（4.4④ 收口）：Profile 列表 | 插件总览——分段控件整宽铺陈 */}
      <div
        role="tablist"
        aria-label={t.profiles.title}
        className="border-line bg-line-soft mb-3 flex rounded-lg border p-0.5"
      >
        {(["list", "plugins"] as const).map((v) => (
          <button
            key={v}
            type="button"
            role="tab"
            aria-selected={view === v}
            onClick={() => setView(v)}
            className={`flex-1 rounded-md px-3 py-1.5 text-xs transition-all ${
              view === v ? "bg-panel text-ink shadow-sm" : "text-dim hover:text-ink"
            }`}
          >
            {v === "list" ? t.profiles.viewProfiles : t.profiles.viewPlugins}
          </button>
        ))}
      </div>

      {/* 页面级提示（操作结果 / 操作错误），可关闭 */}
      {notice && (
        <div
          className={`page-rise mb-3 flex items-start justify-between gap-2 rounded-lg px-3 py-2 text-xs ${
            notice.kind === "ok" ? "bg-ok-soft text-ok" : "bg-warn-soft text-warn"
          }`}
        >
          <span>{notice.text}</span>
          <button
            type="button"
            aria-label={t.profiles.detailClose}
            onClick={() => setNotice(null)}
            className="shrink-0 opacity-60 hover:opacity-100"
          >
            ×
          </button>
        </div>
      )}
      {actionError && (
        <div className="bg-warn-soft text-warn page-rise mb-3 flex items-start justify-between gap-2 rounded-lg px-3 py-2 text-xs">
          <span className="whitespace-pre-wrap">{actionError}</span>
          <button
            type="button"
            aria-label={t.profiles.detailClose}
            onClick={() => setActionError(null)}
            className="shrink-0 opacity-60 hover:opacity-100"
          >
            ×
          </button>
        </div>
      )}

      {/* 两视图：插件总览（只读聚合）| 列表（已物化在前、模板名殿后，后端已排序） */}
      {view === "plugins" ? (
        <PluginOverview refreshKey={overviewTick} />
      ) : (
      <section aria-label={t.profiles.title} className="space-y-2">
        {loadError && (
          <div className="border-line bg-panel rounded-xl border border-dashed px-4 py-8 text-center">
            <div className="text-dim mb-2 text-sm">{loadError}</div>
            <button
              type="button"
              onClick={refresh}
              className="border-line text-dim hover:text-ink inline-flex items-center gap-1 rounded-lg border bg-white px-2.5 py-1.5 text-xs transition-colors"
            >
              <RefreshCw className="size-3" />
              {t.profiles.retryLoad}
            </button>
          </div>
        )}
        {!loadError &&
          list.map((p, i) => (
            <ProfileRow
              key={p.name}
              profile={p}
              index={i}
              isDefault={defaultProfile === p.name}
              isRunning={activeProfile === p.name}
              busy={rowBusy === p.name}
              onDetail={() => setDetailName(p.name)}
              onSetDefault={() => handleSetDefault(p.name)}
              onLaunch={() => handleLaunch(p.name)}
              onRestart={() => handleRestart(p.name)}
              onRename={() => setNameOp({ mode: "rename", source: p.name })}
              onCopy={() => setNameOp({ mode: "copy", source: p.name })}
              onDelete={() => setDeleteTarget(p.name)}
            />
          ))}
      </section>
      )}

      {/* 对话框群 */}
      <ProfileDetailDialog name={detailName} onClose={() => setDetailName(null)} />
      <ProfileSwitchDialog
        target={switchTarget}
        active={activeProfile}
        restart={switchTarget !== null && switchTarget === activeProfile}
        onClose={() => setSwitchTarget(null)}
        onDone={() => {
          setNotice({ kind: "ok", text: t.profiles.switchDone(switchTarget ?? "") })
          refresh()
        }}
      />
      <ProfileCreateDialog
        open={createOpen}
        existing={list}
        onClose={() => setCreateOpen(false)}
        onRefresh={refresh}
      />
      <ProfileNameDialog
        mode={nameOp?.mode ?? "copy"}
        source={nameOp?.source ?? null}
        existing={list}
        onClose={() => setNameOp(null)}
        onRefresh={refresh}
        onDone={(newName, warnings) => {
          if (warnings.length > 0) {
            setNotice({ kind: "warn", text: warnings.join(" ") })
          } else if (nameOp?.mode === "rename") {
            setNotice({ kind: "ok", text: t.profiles.renameDone(newName) })
          } else {
            setNotice({ kind: "ok", text: t.profiles.copyDone(newName) })
          }
        }}
      />
      <ProfileDeleteDialog
        name={deleteTarget}
        onClose={() => setDeleteTarget(null)}
        onRefresh={refresh}
        onDone={(defaultCleared) =>
          setNotice({
            kind: defaultCleared ? "warn" : "ok",
            text: defaultCleared ? t.profiles.deleteDoneCleared : t.profiles.deleteDone,
          })
        }
      />
    </PageShell>
  )
}
