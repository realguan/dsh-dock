// FirstPartyPlugins.tsx —— 「自研插件」货架面板（2026-10-13，profile 详情第 5 tab）。
//
// 形态 = **一插件一行卡片**：身份（原名 + 中文辅名 + 完整包名）→ 状态徽标 →
// 描述 → 宿主前置备注 → 动作（安装 / 更新 / 卸载）。没有"选中某个"这回事——
// 每个条目独立装、独立卸，行内即全部交互（对照「实验能力」的左右栏：那边是
// 多变体互斥、需要详情面；这边一包一行，全部信息一段描述装得下，不值得分栏）。
//
// 三条边界（承 lib/firstPartyPlugins.ts 文件头，勿混）：
//   · 目录是**静态**的：增长 = 改 lib + 发壳新版，不引远端目录链；
//   · 安装 / 卸载 / 激活**零新机制**：`install_plugin` / `remove_plugin` 现成 IPC，
//     插件声明 `dsh.bundle` ⇒ `dsh plugin add` 自动进 bundle 层栈（ADR-0020），
//     壳不写 patch；
//   · 更新 = 再装一次（`pnpm add` 同 spec 覆盖即换版本）——github 形态吃不到
//     npm 更新检查（ADR-0011 严格 npm 判别），不引第二条更新链。

import { useMemo, useState } from "react"
import { Check, ExternalLink, Info, LoaderCircle, Puzzle, Trash2 } from "lucide-react"

import { api } from "@/lib/tauri"
import { pluginChineseName, pluginShortId } from "@/lib/pluginDisplay"
import { FIRST_PARTY_PLUGINS, firstPartyRows } from "@/lib/firstPartyPlugins"
import type { FirstPartyPlugin } from "@/lib/firstPartyPlugins"
import type { PluginEntry } from "@/types/ipc"
import { useI18n } from "@/stores/i18nStore"
import { useRestartNeededStore } from "@/stores/restartNeededStore"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { ConfirmDialog } from "@/components/ui/confirm-dialog"

type Copy = ReturnType<typeof useI18n>["t"]

/** 按目录条目 id 取双语文案。`firstPartyItems` 经 `AppCopy` 递归映射后是**具体字面量
 *  类型**（无索引签名），动态 id 取不到——这里显式断言成查表（漏 key 返回 null，
 *  调用方降级为不渲染描述；目录闸门测试守着"每条目录都有文案"，正常不触发）。 */
function itemCopy(copy: Copy, id: string): { desc: string; note: string } | null {
  const table = copy.profiles.firstPartyItems as Record<
    string,
    { desc: string; note: string } | undefined
  >
  return table[id] ?? null
}

/** 状态徽标（已装 / 可更新 / 未装）。已装 = 绿色对勾，可更新 = 品牌色引导动作，
 *  未装 = 中性灰——状态色只承担状态，品牌色只承担"这里有动作"（承实验能力面板裁定）。 */
function StatusBadge({ copy, installed, installedVersion, updateAvailable }: {
  copy: Copy
  installed: boolean
  installedVersion: string | null
  updateAvailable: boolean
}) {
  if (!installed) {
    return (
      <Badge variant="outline" className="text-faint">
        {copy.profiles.firstPartyNotInstalled}
      </Badge>
    )
  }
  if (updateAvailable) {
    return (
      <Badge variant="default">
        {copy.profiles.firstPartyUpdateAvailable(
          installedVersion ?? copy.profiles.firstPartyNotInstalled,
        )}
      </Badge>
    )
  }
  return (
    <Badge variant="secondary" className="gap-1">
      <Check className="size-3" aria-hidden />
      {copy.profiles.firstPartyInstalled(installedVersion ?? "")}
    </Badge>
  )
}

export function FirstPartyPlugins({
  profileName,
  plugins,
  readOnly = false,
  onNotice,
  onChanged,
}: {
  /** 当前选中档（安装 / 卸载 / 重启标记都按它归属）。 */
  profileName: string
  /** 本档已装插件清单（`list_profile_plugins` 返回值，父级已加载——同页禁双源）。 */
  plugins: PluginEntry[] | null
  /** 官方保留档（desktop）：只读清点，不给装 / 卸入口。 */
  readOnly?: boolean
  onNotice: (text: string, kind?: "ok" | "warn") => void
  /** 装 / 卸成功后通知父级整页回读（清单 / 行表 / 运行态一起刷新）。 */
  onChanged: () => void
}) {
  const { t, activeLocale } = useI18n()
  const [busyId, setBusyId] = useState<string | null>(null)
  const [confirmTarget, setConfirmTarget] = useState<FirstPartyPlugin | null>(null)

  // 货架行 = 静态目录 × 已装清单。plugins 为 null = 父级读取未回（本 tab 与插件
  // 列表同一次加载，正常不会出现），按"全未装"呈现，动作照常可用。
  const rows = useMemo(
    () => firstPartyRows(FIRST_PARTY_PLUGINS, plugins ?? []),
    [plugins],
  )

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
      // 卸载按**真实包名**（dependencies 里的键），不是安装 spec。
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
      <div className="rounded-xl border border-dashed border-line bg-bg p-8 text-center text-xs text-faint">
        {t.profiles.firstPartyEmpty}
      </div>
    )
  }

  return (
    <div className="space-y-3.5">
      <p className="text-dim text-xs leading-relaxed">{t.profiles.firstPartyIntro}</p>

      {rows.map((row) => {
        const { plugin, installed, installedVersion, updateAvailable } = row
        const busy = busyId === plugin.id
        const item = itemCopy(t, plugin.id)
        const chinese = pluginChineseName(plugin.package, activeLocale)
        return (
          <div
            key={plugin.id}
            className="rounded-xl border border-line bg-panel p-4 shadow-2xs transition-colors hover:border-brand/30"
          >
            <div className="flex items-start justify-between gap-3">
              {/* 身份层：原名（主） + 中文辅名 + 完整包名（title 可取） */}
              <div className="min-w-0">
                <div className="flex flex-wrap items-center gap-2">
                  <Puzzle className="text-brand-deep size-4 shrink-0" aria-hidden />
                  <span className="text-ink text-sm font-semibold" title={plugin.package}>
                    {pluginShortId(plugin.package)}
                  </span>
                  {chinese && <span className="text-dim text-xs">{chinese}</span>}
                  <StatusBadge
                    copy={t}
                    installed={installed}
                    installedVersion={installedVersion}
                    updateAvailable={updateAvailable}
                  />
                </div>
                <p className="text-dim mt-1.5 text-xs leading-relaxed">{item?.desc ?? ""}</p>
                {/* 宿主前置备注：有则渲染（空串 = 无前置，整块不渲染） */}
                {item?.note ? (
                  <p className="text-faint mt-1.5 flex items-start gap-1.5 text-xs leading-relaxed">
                    <Info className="mt-0.5 size-3.5 shrink-0" aria-hidden />
                    <span>{item.note}</span>
                  </p>
                ) : null}
                <div className="text-faint mt-2 flex items-center gap-3 text-meta">
                  <button
                    type="button"
                    className="hover:text-dim flex items-center gap-1 transition-colors"
                    onClick={() => {
                      api.openExternal(plugin.repo).catch((e) => onNotice(String(e), "warn"))
                    }}
                  >
                    <ExternalLink className="size-3" aria-hidden />
                    {t.profiles.firstPartyRepo}
                  </button>
                  <span className="font-mono">{plugin.package}</span>
                </div>
              </div>

              {/* 动作层：只读档整列不渲染（没控件就不装作能点） */}
              {!readOnly && (
                <div className="flex shrink-0 items-center gap-2">
                  {busy ? (
                    <span className="text-faint flex items-center gap-1.5 text-xs">
                      <LoaderCircle className="size-3.5 animate-spin" aria-hidden />
                      {t.profiles.firstPartyBusy}
                    </span>
                  ) : installed ? (
                    <>
                      {updateAvailable && (
                        <Button
                          size="sm"
                          className="gap-1.5 text-xs"
                          onClick={() => void install(plugin)}
                        >
                          {t.profiles.firstPartyUpdate(row.pinnedVersion)}
                        </Button>
                      )}
                      <Button
                        size="sm"
                        variant="outline"
                        className="gap-1.5 text-xs"
                        onClick={() => setConfirmTarget(plugin)}
                      >
                        <Trash2 className="size-3.5" />
                        {t.profiles.firstPartyUninstall}
                      </Button>
                    </>
                  ) : (
                    <Button
                      size="sm"
                      className="gap-1.5 text-xs"
                      onClick={() => void install(plugin)}
                    >
                      {t.profiles.firstPartyInstall}
                    </Button>
                  )}
                </div>
              )}
            </div>
          </div>
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
