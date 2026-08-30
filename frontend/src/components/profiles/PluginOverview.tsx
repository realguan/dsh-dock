// 插件总览（4.4④ 收口，ADR-0009 第五次修订）：跨 profile 第三方插件聚合视图。
// 只读——数据源 list_all_plugins 纯文件扫描（零 dsh 子进程、零网络）；写操作
// （安装/启停/卸载）仍在各 profile 详情。行 = 插件 × 安装分布（来源 chips 各
// 带实装版本；声明未安装以警示色标注）。refreshKey 由页面统一推进（聚焦 /
// boot 事件 / 手动刷新），保持与列表视图同一真相节奏。
import { useEffect, useState } from "react"
import { api } from "@/lib/tauri"
import { t } from "@/content/zh-CN"
import type { AggregatePlugin } from "@/types/ipc"

export function PluginOverview({ refreshKey }: { refreshKey: number }) {
  const [list, setList] = useState<AggregatePlugin[] | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let alive = true
    setError(null)
    api
      .listAllPlugins()
      .then((v) => {
        if (alive) setList(v)
      })
      .catch((e) => {
        if (alive) setError(String(e))
      })
    return () => {
      alive = false
    }
  }, [refreshKey])

  // 汇总口径：插件数 = 聚合行数；profile 数 = 出现过安装记录的去重 profile
  const profileCount = list
    ? new Set(list.flatMap((a) => a.sources.map((s) => s.profile))).size
    : 0

  return (
    <section aria-label={t.profiles.viewPlugins} className="space-y-2">
      {/* 汇总行：口径一句话（刷新走页头统一 refreshKey，双视图同一节奏） */}
      <div className="text-faint flex items-baseline justify-between px-1 text-xs">
        <span>{t.profiles.overviewSubtitle}</span>
        {list !== null && list.length > 0 && (
          <span className="font-mono">
            {t.profiles.metaBundles(list.length)} · {t.profiles.overviewSourceCount(profileCount)}
          </span>
        )}
      </div>

      {error && (
        <div className="border-line bg-warn-soft text-warn rounded-xl border border-dashed px-4 py-6 text-center text-xs whitespace-pre-wrap">
          {error}
        </div>
      )}
      {!error && list === null && (
        <div className="text-faint border-line bg-panel rounded-xl border border-dashed px-4 py-8 text-center text-sm">
          {t.profiles.busyShort}
        </div>
      )}
      {!error && list !== null && list.length === 0 && (
        <div className="border-line bg-panel rounded-xl border border-dashed px-4 py-8 text-center">
          <div className="text-dim text-sm">{t.profiles.overviewEmpty}</div>
          <div className="text-faint mt-1 text-xs">{t.profiles.overviewEmptyHint}</div>
        </div>
      )}
      {list?.map((a, i) => (
        <article
          key={a.name}
          className="page-rise border-line bg-panel rounded-xl border shadow-sm transition-shadow hover:shadow-md"
          style={{ animationDelay: `${Math.min(i, 8) * 45}ms` }}
        >
          <div className="px-4 py-3">
            {/* 首行：包名独占整行、尽量展示齐全（超长折行不截断——名字是本视图
                的主语，2026-08-31 维护者裁定）；分布数徽标右缘随行。 */}
            <div className="flex items-start justify-between gap-2">
              <span
                className="text-ink min-w-0 break-all font-mono text-sm font-medium leading-snug"
                title={a.name}
              >
                {a.name}
              </span>
              <span className="border-line text-dim mt-0.5 shrink-0 rounded-full border px-1.5 text-[10px] leading-4">
                {t.profiles.overviewSourceCount(a.sources.length)}
              </span>
            </div>
            {/* 次行：分布 chips（本视图的核心聚合信息，紧跟主语） */}
            <div className="mt-2 flex flex-wrap gap-1.5">
              {a.sources.map((s) =>
                s.version === null ? (
                  <span
                    key={s.profile}
                    className="bg-warn-soft text-warn rounded-md px-1.5 py-0.5 font-mono text-[11px]"
                  >
                    {s.profile} · {t.profiles.overviewNotInstalled}
                  </span>
                ) : (
                  <span
                    key={s.profile}
                    className="border-line bg-bg text-dim rounded-md px-1.5 py-0.5 font-mono text-[11px]"
                  >
                    {s.profile} <span className="text-ink">{s.version}</span>
                  </span>
                ),
              )}
            </div>
            {/* 末行：描述降级为辅助信息（单行截断 + title 兜底，不与主语争位） */}
            {a.description && (
              <div className="text-faint mt-1.5 truncate text-[11px]" title={a.description}>
                {a.description}
              </div>
            )}
          </div>
        </article>
      ))}
    </section>
  )
}
