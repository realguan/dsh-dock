// 详情对话框（4.3 前端刀）：开窗时经 api 播种单 profile 详情；
// patch 原文 mono 等宽展示（后端刻意不解析 YAML，原文即真相）。
import { useEffect, useState } from "react"
import { api } from "@/lib/tauri"
import { t } from "@/content/zh-CN"
import type { ProfileDetail } from "@/types/ipc"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

export function ProfileDetailDialog({
  name,
  onClose,
}: {
  name: string | null
  onClose: () => void
}) {
  const [detail, setDetail] = useState<ProfileDetail | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!name) return
    let alive = true
    setDetail(null)
    setError(null)
    api
      .getProfileDetail(name)
      .then((d) => {
        if (alive) setDetail(d)
      })
      .catch((e) => {
        if (alive) setError(String(e))
      })
    return () => {
      alive = false
    }
  }, [name])

  return (
    <Dialog open={!!name} onOpenChange={(o) => !o && onClose()}>
      <DialogContent className="sm:max-w-[480px]">
        <DialogHeader>
          <DialogTitle>{name ? t.profiles.detailTitle(name) : ""}</DialogTitle>
          <DialogDescription className="sr-only">
            {t.profiles.detailTitle(name ?? "")}
          </DialogDescription>
        </DialogHeader>

        {!detail && !error && (
          <div className="text-faint py-6 text-center text-sm">{t.profiles.busyShort}</div>
        )}
        {error && (
          <div className="bg-warn-soft text-warn rounded-lg px-3 py-2 text-xs whitespace-pre-wrap">
            {error}
          </div>
        )}
        {detail && (
          <div className="text-dim space-y-4 text-sm">
            {/* 插件组合 */}
            <section>
              <div className="text-faint mb-1.5 text-xs">{t.profiles.detailBundles}</div>
              <div className="flex flex-wrap gap-1.5">
                {detail.bundles.length === 0 && (
                  <span className="text-faint text-xs">{t.profiles.detailEmptyDeps}</span>
                )}
                {detail.bundles.map((b) => (
                  <span
                    key={b}
                    className="border-line bg-bg text-ink rounded-md border px-2 py-0.5 font-mono text-xs"
                  >
                    {b}
                  </span>
                ))}
              </div>
            </section>

            {/* 依赖 */}
            <section>
              <div className="text-faint mb-1.5 text-xs">{t.profiles.detailDeps}</div>
              {Object.keys(detail.dependencies).length === 0 ? (
                <div className="text-faint text-xs">{t.profiles.detailEmptyDeps}</div>
              ) : (
                <div className="border-line bg-bg divide-line-soft rounded-lg border">
                  {Object.entries(detail.dependencies).map(([pkg, spec]) => (
                    <div
                      key={pkg}
                      className="flex items-baseline justify-between gap-3 px-3 py-1.5 font-mono text-xs"
                    >
                      <span className="text-ink truncate">{pkg}</span>
                      <span className="text-faint shrink-0">{spec}</span>
                    </div>
                  ))}
                </div>
              )}
            </section>

            {/* patch 原文 */}
            <section>
              <div className="text-faint mb-1.5 text-xs">{t.profiles.detailPatch}</div>
              {detail.patch_yaml === null ? (
                <div className="text-faint text-xs">{t.profiles.detailPatchNone}</div>
              ) : (
                <pre className="border-line bg-bg text-dim max-h-56 overflow-auto rounded-lg border p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap">
                  {detail.patch_yaml}
                </pre>
              )}
            </section>
          </div>
        )}

        <DialogFooter>
          <Button variant="outline" onClick={onClose}>
            {t.profiles.detailClose}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
