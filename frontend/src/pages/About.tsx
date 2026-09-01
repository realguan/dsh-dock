// 关于与更新控制舱（About & Update Center 全量重构）。
// 包含桌面客户端更新、DSH 引擎与 Node 运行时状态、工作台实时连接态及一键诊断复制。
import { useEffect, useState } from "react"
import { Copy, ExternalLink, Globe, Server } from "lucide-react"
import { api } from "@/lib/tauri"
import { resource } from "@/lib/resource"
import { useI18n } from "@/stores/i18nStore"
import { useBootStore } from "@/stores/bootStore"
import { useClientUpdateStore } from "@/stores/clientUpdateStore"
import { Emblem } from "@/components/layout/Emblem"
import { PageShell } from "@/components/layout/PageShell"
import { ClientUpdateCard } from "@/components/about/ClientUpdateCard"
import { DshVersionCard } from "@/components/about/DshVersionCard"
import { NodeVersionCard } from "@/components/about/NodeVersionCard"
import { FloatingToast, type ToastMessage } from "@/components/ui/toast"
import { Button } from "@/components/ui/button"

let autoCheckedOnce = false

export function About() {
  const { t } = useI18n()
  const clientVersion = useBootStore((s) => s.versions?.client.current ?? null)
  const clientNewer = useBootStore((s) => s.versions?.client.newer ?? false)
  const dshVersion = useBootStore((s) => s.versions?.dsh.current ?? null)
  const nodeVersion = useBootStore((s) => s.versions?.node?.version ?? null)
  const hydrate = useClientUpdateStore((s) => s.hydrate)
  const setVersions = useBootStore((s) => s.setVersions)

  const [wbUrl, setWbUrl] = useState<string | null>(null)
  const [toast, setToast] = useState<ToastMessage | null>(null)

  const showToast = (message: string, kind: "ok" | "warn" | "info" = "ok") => {
    setToast({ id: `${Date.now()}`, message, kind })
    setTimeout(() => setToast(null), 3000)
  }

  useEffect(() => {
    let alive = true
    resource.updateStatus().then((v) => {
      if (alive && v) setVersions(v)
    })
    resource.clientUpdate().then((u) => {
      if (!alive || !u) return
      hydrate(u)
      if (u.phase === "idle" && !autoCheckedOnce) {
        autoCheckedOnce = true
        api.clientUpdateCheck().catch(() => {})
      }
    })
    api.getWorkbenchUrl().then((u) => alive && setWbUrl(u)).catch(() => {})
    return () => {
      alive = false
    }
  }, [hydrate, setVersions])

  const copyDiagnostics = () => {
    const report = [
      `=== DSH Dock Diagnostics ===`,
      `Client Version: ${clientVersion ?? "Unknown"}`,
      `DSH Core: ${dshVersion ?? "Not Detected"}`,
      `Node Runtime: ${nodeVersion ?? "Unknown"}`,
      `Workbench URL: ${wbUrl ?? "Not Ready"}`,
      `User Agent: ${navigator.userAgent}`,
      `Timestamp: ${new Date().toISOString()}`,
    ].join("\n")

    void navigator.clipboard.writeText(report).then(() => {
      showToast(t.about.diagnosticsCopied, "ok")
    })
  }

  return (
    <PageShell width={480} align="top" className="py-6">
      {/* 顶栏 Hero 区域 */}
      <header className="mb-5 flex items-center gap-3.5">
        <div className="relative">
          <Emblem size={52} />
          {clientNewer && (
            <span className="bg-warn animate-blink absolute -top-1 -right-1 size-2.5 rounded-full ring-2 ring-panel" />
          )}
        </div>

        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h1 className="text-ink text-lg font-bold tracking-tight">DSH Dock</h1>
            <span className="bg-line-soft text-dim rounded-md px-1.5 py-0.5 font-mono text-xs font-semibold">
              v{clientVersion ?? "…"}
            </span>
          </div>
          <p className="text-faint mt-0.5 text-xs">{t.about.tagline}</p>
        </div>
      </header>

      <div className="space-y-3.5">
        {/* 1. 桌面客户端更新中心 */}
        <section aria-label={t.about.clientLabel} className="page-rise">
          <ClientUpdateCard />
        </section>

        {/* 2. 运行环境矩阵（DSH + Node） */}
        <section
          aria-label={t.about.envTitle}
          className="page-rise border-line bg-panel overflow-hidden rounded-2xl border shadow-xs transition-shadow hover:shadow-sm"
        >
          <DshVersionCard />
          <NodeVersionCard />
        </section>

        {/* 3. 工作台实时实例连接卡 */}
        <section
          aria-label="工作台实例"
          className="page-rise border-line bg-panel rounded-2xl border p-4 shadow-xs"
        >
          <div className="flex items-center justify-between gap-3">
            <div className="flex items-center gap-2.5 min-w-0">
              <div
                className={`flex size-8 shrink-0 items-center justify-center rounded-xl ${
                  wbUrl ? "bg-ok-soft text-ok" : "bg-line-soft text-faint"
                }`}
              >
                {wbUrl ? <Globe className="size-4" /> : <Server className="size-4" />}
              </div>
              <div className="min-w-0">
                <div className="flex items-center gap-1.5">
                  <span className="text-ink text-xs font-semibold">
                    {wbUrl ? t.about.liveSessionActive : t.about.workbenchNotReady}
                  </span>
                  {wbUrl && (
                    <span className="bg-ok size-1.5 animate-pulse rounded-full" />
                  )}
                </div>
                <p className="text-faint mt-0.5 truncate font-mono text-[11px]" title={wbUrl ?? undefined}>
                  {wbUrl ?? "启动 DSH 后将自动建立本地 HTTP 桥接"}
                </p>
              </div>
            </div>

            {wbUrl && (
              <Button
                size="sm"
                variant="outline"
                onClick={() => api.openWorkbenchInBrowser().catch(() => {})}
                className="shrink-0 gap-1 text-xs"
              >
                <ExternalLink className="size-3" />
                <span>{t.about.openInBrowser}</span>
              </Button>
            )}
          </div>
        </section>

        {/* 4. 诊断与操作栏 */}
        <div className="flex items-center justify-between pt-1">
          <Button
            size="sm"
            variant="ghost"
            onClick={copyDiagnostics}
            className="text-faint hover:text-ink gap-1.5 text-xs"
          >
            <Copy className="size-3" />
            <span>{t.about.copyDiagnostics}</span>
          </Button>

          <span className="text-faint text-[11px] font-mono">
            Tauri v2 · React 19
          </span>
        </div>

        {/* 脚注说明 */}
        <footer className="text-faint border-t border-line/60 pt-3 text-center text-[11px] leading-relaxed">
          {t.about.restartNote}
          <br />
          {t.about.dshUpgradeNote}
        </footer>
      </div>

      <FloatingToast toast={toast} onDismiss={() => setToast(null)} />
    </PageShell>
  )
}
