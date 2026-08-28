// 关于/更新中心页（原 ui/about.html 完整迁移，frontend-migration §4.4）。
// 事件消费走 App 顶层的全局事件总线（boot:update / app:update 已入 store），
// 本页只做三件事：进入时播种（版本快照 + 更新状态机 + 工作台地址）、
// idle 态自动首查、纯展示组合。无「打开关于」入口职责（常驻入口在菜单/托盘）。
import { useEffect, useState } from "react"
import { Globe } from "lucide-react"
import { api } from "@/lib/tauri"
import { resource } from "@/lib/resource"
import { t } from "@/content/zh-CN"
import { useBootStore } from "@/stores/bootStore"
import { useClientUpdateStore } from "@/stores/clientUpdateStore"
import { Emblem } from "@/components/layout/Emblem"
import { PageShell } from "@/components/layout/PageShell"
import { ClientUpdateCard } from "@/components/about/ClientUpdateCard"
import { DshVersionCard } from "@/components/about/DshVersionCard"
import { NodeVersionCard } from "@/components/about/NodeVersionCard"

/// StrictMode 双挂载下只自动首查一次（run_check 每次调用都起线程，防重复触网）
let autoCheckedOnce = false

export function About() {
  const clientVersion = useBootStore((s) => s.versions?.client.current ?? null)
  const clientNewer = useBootStore((s) => s.versions?.client.newer ?? false)
  const hydrate = useClientUpdateStore((s) => s.hydrate)
  const setVersions = useBootStore((s) => s.setVersions)

  const [wbUrl, setWbUrl] = useState<string | null>(null)

  useEffect(() => {
    let alive = true
    // 播种一：三维度版本快照
    resource.updateStatus().then((v) => {
      if (alive && v) setVersions(v)
    })
    // 播种二：客户端更新状态机 + idle 自动首查（旧页语义：从未查过才自动查）
    resource.clientUpdate().then((u) => {
      if (!alive || !u) return
      hydrate(u)
      if (u.phase === "idle" && !autoCheckedOnce) {
        autoCheckedOnce = true
        api.clientUpdateCheck().catch(() => {})
      }
    })
    // 播种三：工作台地址（决定浏览器入口可用性）
    api.getWorkbenchUrl().then((u) => alive && setWbUrl(u)).catch(() => {})
    return () => {
      alive = false
    }
  }, [hydrate, setVersions])

  return (
    <PageShell width={432} align="top">
      {/* 头部：徽章 + 品牌名（含当前客户端版本）+ 角色行 */}
      <header className="mb-5 flex items-center gap-3">
        <Emblem size={48} />
        <div className="min-w-0">
          <div className="text-ink flex items-center gap-2 text-lg font-semibold tracking-tight">
            DSH Dock
            <span className="text-faint font-mono text-sm font-normal">
              {clientVersion ?? "…"}
            </span>
            {clientNewer && <span className="bg-warn animate-blink size-2 rounded-full" />}
          </div>
          <div className="text-faint truncate text-xs">{t.about.tagline}</div>
        </div>
      </header>

      {/* 动作主体：客户端自更新状态机 */}
      <section aria-label={t.about.clientLabel} className="page-rise mb-3">
        <ClientUpdateCard />
      </section>

      {/* 只读维度：dsh 可检查可升级；node 纯只读 */}
      <section aria-label={t.about.envTitle} className="page-rise border-line bg-panel rounded-xl border shadow-sm [animation-delay:60ms]">
        <DshVersionCard />
        <NodeVersionCard />
      </section>

      {/* 工作台浏览器入口 */}
      <div className="page-rise mt-3 flex items-center justify-between gap-3 [animation-delay:100ms]">
        <button
          type="button"
          disabled={!wbUrl}
          onClick={() => {
            api.openWorkbenchInBrowser().catch(() => {})
          }}
          className="border-line text-dim hover:border-line hover:text-ink inline-flex items-center gap-1.5 rounded-lg border bg-white px-2.5 py-1.5 text-xs transition-colors disabled:cursor-not-allowed disabled:opacity-50"
        >
          <Globe className="size-3.5" />
          {t.about.openInBrowser}
        </button>
        <span className="text-faint truncate font-mono text-xs" title={wbUrl ?? undefined}>
          {wbUrl ?? t.about.workbenchNotReady}
        </span>
      </div>

      <footer className="text-faint mt-4 text-center text-xs leading-relaxed">
        {t.about.restartNote}
        <br />
        {t.about.dshUpgradeNote}
      </footer>
    </PageShell>
  )
}
