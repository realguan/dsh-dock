// FirstPartyPlugins.tsx —— 「自研插件」货架子视图（2026-10-13；2026-10-14 视觉与布局重构）。
//
// 定位：DSH Dock 团队官方策展的自研插件货架（收在「插件列表」tab 的子 tab 内）。
// 布局与审美原则（frontend-design 规范）：
//   · 宽裕横向展示卡：绝不使用在侧边栏分栏下导致宽度仅 250px 的狭窄多列栅格；
//   · 完整舒展的信息层级：中文主标题、等宽版本胶囊、运行态徽标各得其所，不发生任何截断；
//   · 核心能力一览：原生子视窗、双模并排/全屏、独立持久会话等特性微标签自然排布；
//   · 桌面工匠质感：squircle 品牌图标盒、精炼的 GitHub 来源指示器、明确可点击的操作主按钮。
//
// 架构边界（承 lib/firstPartyPlugins.ts 文件头，勿混）：
//   · 目录静态内置：新增/升级 = 改 lib + 发版，不引远端目录链；
//   · 装/卸/激活零新机制：走现有 `install_plugin` / `remove_plugin` IPC；
//   · 官方保留档（desktop）遵循 ADR-0034：开放内容面，动作按钮完全可用。

import { useMemo, useState } from "react"
import {
  Check,
  Download,
  ExternalLink,
  LoaderCircle,
  MessageSquareText,
  Puzzle,
  Sparkles,
  Trash2,
} from "lucide-react"

import { api } from "@/lib/tauri"
import { pluginChineseName } from "@/lib/pluginDisplay"
import { FIRST_PARTY_PLUGINS, firstPartyRows } from "@/lib/firstPartyPlugins"
import type { FirstPartyPlugin } from "@/lib/firstPartyPlugins"
import type { PluginEntry } from "@/types/ipc"
import { useI18n } from "@/stores/i18nStore"
import { useRestartNeededStore } from "@/stores/restartNeededStore"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { ConfirmDialog } from "@/components/ui/confirm-dialog"
import { GithubIcon } from "@/components/market/MarketPluginCard"

type Copy = ReturnType<typeof useI18n>["t"]

interface ItemCopyResult {
  desc: string
  features?: readonly string[]
}

/** 按目录条目 id 取双语文案（描述与核心能力标签）。 */
function itemCopy(copy: Copy, id: string): ItemCopyResult | null {
  const table = copy.profiles.firstPartyItems as Record<string, ItemCopyResult | undefined>
  return table[id] ?? null
}

/** 仓库 owner（`https://github.com/<owner>/<repo>` → `<owner>`）。 */
function repoOwner(repo: string): string {
  const match = /^https?:\/\/github\.com\/([^/]+)\//.exec(repo)
  return match?.[1] ?? ""
}

/** 格式化仓库短名（如 `realguan/dsh-deepseek-chat-embed`）。 */
function repoShortName(repo: string, fallback: string): string {
  const match = /^https?:\/\/github\.com\/([^/]+\/[^/]+)/.exec(repo)
  return match?.[1] ?? fallback
}

/** 根据插件 id 选取专有视觉图标（赋予核心第一方插件鲜明的产品特征）。 */
function getPluginIcon(id: string) {
  if (id === "deepseek-chat-embed") {
    return MessageSquareText
  }
  return Puzzle
}

export function FirstPartyPlugins({
  profileName,
  plugins,
  onNotice,
  onChanged,
}: {
  /** 当前选中档（安装 / 卸载 / 重启标记都按它归属）。 */
  profileName: string
  /** 本档已装插件清单（`list_profile_plugins` 返回值，父级已加载——同页禁双源）。 */
  plugins: PluginEntry[] | null
  onNotice: (text: string, kind?: "ok" | "warn") => void
  /** 装 / 卸成功后通知父级整页回读（清单 / 行表 / 运行态一起刷新）。 */
  onChanged: () => void
}) {
  const { t, activeLocale } = useI18n()
  const [busyId, setBusyId] = useState<string | null>(null)
  const [confirmTarget, setConfirmTarget] = useState<FirstPartyPlugin | null>(null)

  // 货架行 = 静态目录 × 已装清单。plugins 为 null 按"全未装"呈现。
  const rows = useMemo(() => firstPartyRows(FIRST_PARTY_PLUGINS, plugins ?? []), [plugins])

  const install = async (plugin: FirstPartyPlugin) => {
    setBusyId(plugin.id)
    try {
      const out = await api.installPlugin(profileName, plugin.spec)
      if (!out.ok) {
        onNotice(out.detail, "warn")
        return
      }
      // 安装 / 更新都改变依赖树 ⇒ 标记待重启（与插件列表 / 实验能力同一 store）。
      useRestartNeededStore.getState().mark(profileName)
      onNotice(out.detail, "ok")
      onChanged()
    } catch (e) {
      onNotice(String(e), "warn")
    } finally {
      setBusyId(null)
    }
  }

  const remove = async (plugin: FirstPartyPlugin) => {
    setBusyId(plugin.id)
    try {
      // 卸载按真实包名（dependencies 里的键），不是安装 spec。
      const out = await api.removePlugin(profileName, plugin.package)
      if (!out.ok) {
        onNotice(out.detail, "warn")
        return
      }
      useRestartNeededStore.getState().mark(profileName)
      onNotice(out.detail, "ok")
      onChanged()
    } catch (e) {
      onNotice(String(e), "warn")
    } finally {
      setBusyId(null)
    }
  }

  if (rows.length === 0) {
    return (
      <div className="max-w-2xl rounded-2xl border border-dashed border-line bg-bg p-8 text-center text-xs text-faint">
        {t.profiles.firstPartyEmpty}
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-4 max-w-3xl">
      {rows.map((row) => {
        const { plugin, installed, installedVersion, updateAvailable } = row
        const busy = busyId === plugin.id
        const item = itemCopy(t, plugin.id)
        const chinese = pluginChineseName(plugin.package, activeLocale)
        const owner = repoOwner(plugin.repo)
        const repoShort = repoShortName(plugin.repo, plugin.package)
        const PluginIcon = getPluginIcon(plugin.id)
        const features = item?.features ?? []

        return (
          <article
            key={plugin.id}
            className="group relative flex flex-col justify-between overflow-hidden rounded-2xl border border-line bg-panel p-5.5 shadow-2xs transition-all duration-200 hover:-translate-y-0.5 hover:border-brand/40 hover:shadow-xs"
          >
            {/* 顶角柔和环境光晕（纯 token，增强层次深度） */}
            <div
              className="pointer-events-none absolute -top-12 -right-12 size-36 rounded-full bg-wash blur-2xl opacity-60 transition-opacity duration-300 group-hover:opacity-100"
              aria-hidden
            />

            <div>
              {/* 卡头：左侧图标与标题信息 + 右侧状态徽标（横向完全舒展，互不压迫） */}
              <div className="flex items-start justify-between gap-4">
                <div className="flex items-start gap-3.5 min-w-0">
                  {/* 精致品牌图标容器 */}
                  <div
                    className="flex size-11.5 shrink-0 items-center justify-center rounded-xl border border-brand/25 bg-wash text-brand-deep shadow-2xs transition-all duration-200 group-hover:scale-105 group-hover:border-brand/40 group-hover:shadow-glow-sm"
                    title={plugin.package}
                  >
                    <PluginIcon className="size-6" aria-hidden />
                  </div>

                  <div className="min-w-0">
                    {/* 主标题行：中文主名 + 专属版本号胶囊 */}
                    <div className="flex items-center gap-2.5 flex-wrap">
                      <h3 className="text-base font-semibold text-ink tracking-tight transition-colors group-hover:text-brand-deep">
                        {chinese || plugin.package}
                      </h3>

                      {/* 专属版本胶囊：即使未安装也能一眼看见要装哪个版本 */}
                      <span
                        className="inline-flex shrink-0 items-center rounded-md border border-brand/20 bg-wash px-2 py-0.5 font-mono text-meta font-medium text-brand-deep"
                        title={plugin.spec}
                      >
                        v{row.pinnedVersion}
                      </span>
                    </div>

                    {/* 次级元数据行：包名（可点开仓库） + 作者 */}
                    <div className="mt-1 flex items-center gap-2 text-meta text-faint">
                      <button
                        type="button"
                        className="font-mono text-dim hover:text-brand-deep cursor-pointer text-left transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand/40"
                        title={plugin.package}
                        onClick={() => {
                          api.openExternal(plugin.repo).catch((e) => onNotice(String(e), "warn"))
                        }}
                      >
                        {plugin.package}
                      </button>

                      {owner && (
                        <span>
                          · by {owner}
                        </span>
                      )}
                    </div>
                  </div>
                </div>

                {/* 状态徽标：独立占位，不与主标题抢占横向空间 */}
                <div className="shrink-0 pt-0.5">
                  {installed ? (
                    updateAvailable ? (
                      <Badge className="gap-1 border border-brand/30 bg-wash text-brand-deep text-meta font-medium px-2.5 py-1">
                        <Sparkles className="size-3.5" aria-hidden />
                        {t.profiles.firstPartyUpdateAvailable(row.pinnedVersion)}
                      </Badge>
                    ) : (
                      <Badge
                        variant="secondary"
                        className="gap-1 border border-ok/30 bg-ok-soft text-ok text-meta font-medium px-2.5 py-1"
                      >
                        <Check className="size-3.5" aria-hidden />
                        {t.profiles.firstPartyInstalledWithVersion(
                          installedVersion ?? row.pinnedVersion,
                        )}
                      </Badge>
                    )
                  ) : (
                    <Badge
                      variant="outline"
                      className="border-line text-faint text-meta font-medium px-2.5 py-1"
                    >
                      {t.profiles.firstPartyNotInstalled}
                    </Badge>
                  )}
                </div>
              </div>

              {/* 能力特性胶囊行：横向齐整排布 */}
              {features.length > 0 && (
                <div className="mt-3.5 flex flex-wrap items-center gap-2">
                  {features.map((feat) => (
                    <span
                      key={feat}
                      className="inline-flex items-center rounded-lg border border-line-soft bg-bg px-2.5 py-1 text-meta font-medium text-dim shadow-2xs"
                    >
                      {feat}
                    </span>
                  ))}
                </div>
              )}

              {/* 描述段落：自然舒展 */}
              <p className="mt-3 text-xs leading-relaxed text-dim max-w-2xl">
                {item?.desc || t.profiles.firstPartyNoDesc}
              </p>
            </div>

            {/* 卡尾：来源仓库胶囊（左） + 动作按钮区（右） */}
            <div className="mt-5 flex min-h-[38px] items-center justify-between gap-4 border-t border-line/70 pt-3.5">
              <button
                type="button"
                className="group/repo -ml-1.5 inline-flex items-center gap-2 rounded-lg px-2.5 py-1.5 font-mono text-meta text-dim transition-colors hover:bg-bg hover:text-ink cursor-pointer"
                title={plugin.repo}
                onClick={() => {
                  api.openExternal(plugin.repo).catch((e) => onNotice(String(e), "warn"))
                }}
              >
                <GithubIcon className="size-4 shrink-0 text-faint transition-colors group-hover/repo:text-ink" />
                <span>{repoShort}</span>
                <ExternalLink className="size-3.5 shrink-0 text-faint opacity-50 transition-opacity group-hover/repo:opacity-100" />
              </button>

              {!busy ? (
                <div className="flex shrink-0 items-center justify-end gap-2">
                  {installed ? (
                    <>
                      {updateAvailable && (
                        <Button
                          size="sm"
                          className="h-8 gap-1.5 px-3.5 text-xs font-semibold shadow-2xs transition-all hover:shadow-xs cursor-pointer"
                          onClick={() => void install(plugin)}
                        >
                          <Sparkles className="size-3.5" aria-hidden />
                          {t.profiles.firstPartyUpdate(row.pinnedVersion)}
                        </Button>
                      )}
                      <Button
                        size="sm"
                        variant="outline"
                        className="h-8 gap-1.5 px-3 text-xs text-dim transition-colors hover:border-danger/30 hover:bg-danger-soft hover:text-danger cursor-pointer"
                        onClick={() => setConfirmTarget(plugin)}
                      >
                        <Trash2 className="size-3.5" aria-hidden />
                        {t.profiles.firstPartyUninstall}
                      </Button>
                    </>
                  ) : (
                    <Button
                      size="sm"
                      className="h-8 gap-1.5 px-4 text-xs font-semibold shadow-2xs transition-all hover:shadow-xs cursor-pointer"
                      onClick={() => void install(plugin)}
                    >
                      <Download className="size-3.5" aria-hidden />
                      {t.profiles.firstPartyInstallWithVersion(row.pinnedVersion)}
                    </Button>
                  )}
                </div>
              ) : (
                <span className="flex shrink-0 items-center gap-2 px-3 py-1 text-meta text-faint">
                  <LoaderCircle className="size-3.5 animate-spin text-brand-deep" aria-hidden />
                  {t.profiles.firstPartyBusy}
                </span>
              )}
            </div>
          </article>
        )
      })}

      {/* 卸载确认（U9 同口径：不可撤销动作先过确认；只说后果，不吓唬） */}
      <ConfirmDialog
        open={confirmTarget !== null}
        title={t.profiles.firstPartyUninstallConfirmTitle}
        points={[
          confirmTarget
            ? t.profiles.firstPartyUninstallConfirmBody(confirmTarget.package)
            : "",
        ]}
        confirmLabel={t.profiles.firstPartyUninstall}
        cancelLabel={t.confirm.cancel}
        onConfirm={() => {
          const target = confirmTarget
          setConfirmTarget(null)
          if (target) void remove(target)
        }}
        onClose={() => setConfirmTarget(null)}
      />
    </div>
  )
}
