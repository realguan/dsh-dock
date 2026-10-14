// FirstPartyPlugins.tsx —— 「自研插件」货架子视图（2026-10-13；2026-10-14 整容：
// 从顶级 tab 收进「插件列表」tab 内作为子 tab，卡片样式对齐社区市场 MarketPluginCard）。
//
// 形态 = **社区市场同款卡片网格**（`rounded-2xl` + 品牌角标盒 + 等宽名按钮 + 描述 +
// 底栏来源/动作），与 MarketplaceView 的 grid 断点一致——两个「可安装清单」长得一样，
// 用户不用重新学。差异只在数据源与动作面：
//   · 社区市场：远端 registry、跨档安装、队列/并发；
//   · 本货架：壳内置静态目录、只装当前档、串行单发。
//
// 2026-10-14 v1.4.0 验收整容（维护者原话"好丑"+"备注对用户没有意义"）：
//   ① 删掉宿主前置备注段——那是给贡献者看的内部上下文（ADR/版本前提），货架是
//      给用户的，用户不需要知道宿主原语是什么；
//   ② 包名**禁止截断**（承 2026-09-17 裁定"包名完整折行不许截断"，上一版
//      `truncate` 在窄卡片里把 `dsh-deepseek-chat-embed` 截成 "ds..."）；
//   ③ 描述不折行截断（我们的描述就一两句，截在半句中读着是坏的）；
//   ④ 底栏去掉与徽标重复的版本号，改为展示**安装来源**（spec + GitHub 图标，
//      与社区卡的来源语言一致）——版本归徽标，来源归底栏，各说一件事。
//
// 三条边界（承 lib/firstPartyPlugins.ts 文件头，勿混）：
//   · 目录是**静态**的：增长 = 改 lib + 发壳新版，不引远端目录链；
//   · 安装 / 卸载 / 激活**零新机制**：`install_plugin` / `remove_plugin` 现成 IPC，
//     插件声明 `dsh.bundle` ⇒ `dsh plugin add` 自动进 bundle 层栈（ADR-0020），
//     壳不写 patch；
//   · 更新 = 再装一次（`pnpm add` 同 spec 覆盖即换版本）——github 形态吃不到
//     npm 更新检查（ADR-0011 严格 npm 判别），不引第二条更新链。

import { useMemo, useState } from "react"
import { Check, LoaderCircle, Puzzle, Trash2 } from "lucide-react"

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

/** 按目录条目 id 取双语文案。`firstPartyItems` 经 `AppCopy` 递归映射后是**具体字面量
 *  类型**（无索引签名），动态 id 取不到——这里显式断言成查表（漏 key 返回 null，
 *  调用方降级为兜底文案；目录闸门测试守着"每条目录都有文案"，正常不触发）。 */
function itemCopy(copy: Copy, id: string): { desc: string } | null {
  const table = copy.profiles.firstPartyItems as Record<string, { desc: string } | undefined>
  return table[id] ?? null
}

/** 仓库 owner（`https://github.com/<owner>/<repo>` → `<owner>`）——卡片头
//  `by <owner>` 行与社区卡同构；取不到返回空串（整块不渲染）。 */
function repoOwner(repo: string): string {
  const match = /^https?:\/\/github\.com\/([^/]+)\//.exec(repo)
  return match?.[1] ?? ""
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

  // 货架行 = 静态目录 × 已装清单。plugins 为 null = 父级读取未回（本子视图与插件
  // 列表同一次加载，正常不会出现），按"全未装"呈现，动作照常可用。
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
      <div className="rounded-2xl border border-dashed border-line bg-bg p-8 text-center text-xs text-faint">
        {t.profiles.firstPartyEmpty}
      </div>
    )
  }

  return (
    <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
      {rows.map((row) => {
        const { plugin, installed, installedVersion, updateAvailable } = row
        const busy = busyId === plugin.id
        const item = itemCopy(t, plugin.id)
        const chinese = pluginChineseName(plugin.package, activeLocale)
        const owner = repoOwner(plugin.repo)
        return (
          <article
            key={plugin.id}
            className="group relative flex flex-col justify-between rounded-2xl border border-line bg-panel p-4 shadow-2xs transition-all duration-200 hover:border-brand/40 hover:shadow-xs hover:-translate-y-0.5"
          >
            <div>
              {/* 卡头：品牌角标盒（自研 = 壳团队策展，走品牌色而非来源灰盒）
                  + 等宽包名按钮（点开仓库，与社区卡同一交互；**完整折行不截断**） */}
              <div className="flex items-start gap-2.5">
                <div
                  className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-brand/40 bg-wash text-brand-deep shadow-2xs transition-colors group-hover:border-brand"
                  title={plugin.package}
                >
                  <Puzzle className="size-4.5" aria-hidden />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
                    <h3 className="min-w-0">
                      <button
                        type="button"
                        className="font-mono text-xs font-semibold text-ink tracking-tight hover:text-brand-deep cursor-pointer text-left transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand/40"
                        title={plugin.package}
                        onClick={() => {
                          api.openExternal(plugin.repo).catch((e) => onNotice(String(e), "warn"))
                        }}
                      >
                        {plugin.package}
                      </button>
                    </h3>
                    {/* 状态徽标占社区卡的分类徽位。按 2026-09-21 裁定**不挂「自研」标**——
                        本子 tab 已按类分，再挂同类名是复述噪音（同社区卡在市场里不挂「社区」）。 */}
                    {installed ? (
                      updateAvailable ? (
                        <Badge className="shrink-0 text-meta font-medium">
                          {t.profiles.firstPartyUpdateAvailable(row.pinnedVersion)}
                        </Badge>
                      ) : (
                        <Badge
                          variant="secondary"
                          className="shrink-0 gap-1 text-meta font-medium border border-line/60 bg-wash text-dim"
                        >
                          <Check className="size-3" aria-hidden />
                          {t.profiles.firstPartyInstalled(installedVersion ?? "")}
                        </Badge>
                      )
                    ) : (
                      <Badge variant="outline" className="shrink-0 text-meta font-medium text-faint">
                        {t.profiles.firstPartyNotInstalled}
                      </Badge>
                    )}
                  </div>
                  <p className="text-meta text-dim mt-0.5">
                    {chinese}
                    {chinese && owner ? " · " : ""}
                    {owner ? `by ${owner}` : ""}
                  </p>
                </div>
              </div>

              {/* 描述：全文展示（我们的描述就一两句；截断只会在半句中读着是坏的） */}
              <p className="mt-3 text-xs text-dim leading-relaxed">
                {item?.desc || t.profiles.firstPartyNoDesc}
              </p>
            </div>

            {/* 卡尾：安装来源（spec + GitHub 图标，与社区卡的来源语言一致） + 动作区。
                保留档整块不渲染动作——没控件就不装作能点。 */}
            <div className="mt-3.5 pt-3 border-t border-line/70 flex flex-col gap-2.5">
              <div className="flex items-center justify-between gap-2 min-h-[22px]">
                <span
                  className="flex min-w-0 items-center gap-1.5 text-meta text-faint"
                  title={plugin.spec}
                >
                  <GithubIcon className="size-3 shrink-0" />
                  <span className="truncate font-mono">{plugin.spec}</span>
                </span>
                {busy && (
                  <span className="flex shrink-0 items-center gap-1.5 text-meta text-faint">
                    <LoaderCircle className="size-3 animate-spin" aria-hidden />
                    {t.profiles.firstPartyBusy}
                  </span>
                )}
              </div>

              {!busy && (
                <div className="flex items-center justify-end gap-1.5">
                  {installed ? (
                    <>
                      {updateAvailable && (
                        <Button
                          size="sm"
                          className="h-7 gap-1 px-2.5 text-xs"
                          onClick={() => void install(plugin)}
                        >
                          {t.profiles.firstPartyUpdate(row.pinnedVersion)}
                        </Button>
                      )}
                      <Button
                        size="sm"
                        variant="outline"
                        className="h-7 gap-1 px-2.5 text-xs"
                        onClick={() => setConfirmTarget(plugin)}
                      >
                        <Trash2 className="size-3" />
                        {t.profiles.firstPartyUninstall}
                      </Button>
                    </>
                  ) : (
                    <Button
                      size="sm"
                      className="h-7 gap-1 px-2.5 text-xs font-semibold"
                      onClick={() => void install(plugin)}
                    >
                      {t.profiles.firstPartyInstall}
                    </Button>
                  )}
                </div>
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
