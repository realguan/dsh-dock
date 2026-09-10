// components/market/MarketCustomInstallDialog.tsx —— 插件市场手动安装/自定义地址分发对话框
import { useEffect, useState } from "react"
import { AlertCircle, Code2, Download, Package } from "lucide-react"
import { useQueueStore } from "@/stores/queueStore"
import { useInstallFlightStore } from "@/stores/installFlightStore"
import { useI18n } from "@/stores/i18nStore"
import type { ProfileSummary } from "@/types/ipc"
import { validatePluginSpec } from "@/lib/profiles"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"

interface MarketCustomInstallDialogProps {
  open: boolean
  profiles: ProfileSummary[]
  installedMap: Map<string, string[]>
  onClose: () => void
  onNotice?: (text: string, kind?: "ok" | "warn") => void
}

/** 从手动输入的 Spec 中推断出友好的包名展示 */
function inferPkgName(spec: string): string {
  const s = spec.trim()
  if (!s) return "plugin"
  if (s.startsWith("github:") || s.startsWith("git+")) {
    const parts = s.split("/")
    const last = parts[parts.length - 1]?.replace(/\.git$/, "") || "plugin"
    return last
  }
  // 处理 @scope/pkg@ver 或 pkg@ver
  if (s.startsWith("@")) {
    const atParts = s.split("@")
    return atParts[1] ? `@${atParts[1]}` : s
  }
  return s.split("@")[0] || s
}

export function MarketCustomInstallDialog({
  open,
  profiles,
  installedMap,
  onClose,
  onNotice,
}: MarketCustomInstallDialogProps) {
  const { t } = useI18n()
  const [specInput, setSpecInput] = useState("")
  const [selectedProfile, setSelectedProfile] = useState("")
  const [error, setError] = useState<string | null>(null)
  const enqueue = useQueueStore((s) => s.enqueue)

  useEffect(() => {
    if (open) {
      setSpecInput("")
      setError(null)
      const defaultProf = profiles.find((p) => p.materialized)?.name || profiles[0]?.name || "web"
      setSelectedProfile(defaultProf)
    }
  }, [open, profiles])

  const trimmedSpec = specInput.trim()
  const inferredName = inferPkgName(trimmedSpec)
  const isGithub = trimmedSpec.startsWith("github:") || trimmedSpec.startsWith("git+") || trimmedSpec.includes("github.com/")
  const installedProfiles = installedMap.get(inferredName) || []
  const isAlreadyInstalled = selectedProfile ? installedProfiles.includes(selectedProfile) : false

  const handleInstall = (e: React.MouseEvent) => {
    if (!trimmedSpec || !selectedProfile) return
    const invalid = validatePluginSpec(trimmedSpec)
    if (invalid) {
      setError(invalid)
      return
    }

    enqueue({
      pkg: inferredName,
      spec: trimmedSpec,
      profile: selectedProfile,
      kind: "install",
    })

    useInstallFlightStore.getState().launch({ x: e.clientX, y: e.clientY }, inferredName)
    onNotice?.(t.market.installQueued(inferredName, selectedProfile), "ok")
    onClose()
  }

  return (
    <Dialog open={open} onOpenChange={(val) => !val && onClose()}>
      <DialogContent className="max-w-md rounded-2xl border border-line bg-panel p-6 shadow-xl">
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div className="flex size-10 shrink-0 items-center justify-center rounded-xl border border-brand/30 bg-brand/10 text-brand-deep shadow-2xs">
              <Download className="size-5" />
            </div>
            <div className="min-w-0 flex-1">
              <DialogTitle className="text-base font-bold text-ink">
                {t.market.manualInstallTitle}
              </DialogTitle>
              <DialogDescription className="text-xs text-dim mt-0.5 leading-relaxed">
                {t.market.manualInstallDesc}
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <div className="space-y-4 py-2 text-xs">
          {/* 规范输入框 */}
          <div className="space-y-1.5">
            <div className="flex items-center justify-between">
              <label htmlFor="custom-plugin-spec-input" className="text-xs font-medium text-ink">
                {t.market.installSpecLabel}
              </label>
              {/* 来源徽标一律中性（由图标 + 文案表意）：旧实现 npm 走成功绿，
                  而同一对话框的「已安装」也用同一个绿——来源与状态撞色。
                  2026-09-10 批次 E 复核修正（与另两处来源徽标统一）。 */}
              {trimmedSpec && (
                <span className="border-line bg-line-soft text-dim inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 font-mono text-meta font-medium shadow-2xs">
                  {isGithub ? <Code2 className="size-3" /> : <Package className="size-3" />}
                  <span>{isGithub ? t.market.sourceGithub : t.market.sourceNpm}</span>
                </span>
              )}
            </div>

            <input
              id="custom-plugin-spec-input"
              type="text"
              autoFocus
              aria-label={t.market.manualInstallSpecPlaceholder}
              value={specInput}
              onChange={(e) => {
                setSpecInput(e.target.value)
                if (error) setError(null)
              }}
              onKeyDown={(e) => e.key === "Enter" && handleInstall(e as unknown as React.MouseEvent)}
              placeholder={t.market.manualInstallSpecPlaceholder}
              className="w-full rounded-xl border border-line bg-wash px-3 py-2 font-mono text-xs text-ink placeholder:text-faint outline-none focus:border-brand focus:bg-panel shadow-2xs transition-all"
            />
          </div>

          {/* 目标 Profile 选择 */}
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-ink flex items-center justify-between">
              <span>{t.market.selectProfile}</span>
              {selectedProfile && isAlreadyInstalled && (
                <span className="text-meta text-warn font-mono flex items-center gap-1">
                  <AlertCircle className="size-3" />
                  已在此 Profile 安装（将覆盖重装）
                </span>
              )}
            </label>
            <Select value={selectedProfile} onValueChange={setSelectedProfile}>
              <SelectTrigger className="w-full h-9 rounded-xl border-line bg-panel text-ink text-xs">
                <SelectValue placeholder={t.market.selectProfile} />
              </SelectTrigger>
              <SelectContent className="rounded-xl border-line bg-panel text-xs text-ink">
                {profiles.map((p) => {
                  const hasIt = installedProfiles.includes(p.name)
                  return (
                    <SelectItem key={p.name} value={p.name} className="py-2">
                      <div className="flex items-center gap-2">
                        <span className="font-mono font-medium">{p.name}</span>
                        {p.web_ui && (
                          <span className="rounded-md bg-brand/10 px-1 py-0.2 text-micro text-brand-deep">Web</span>
                        )}
                        {hasIt && (
                          <span className="rounded-md bg-ok-soft px-1 py-0.2 text-micro text-ok">
                            已安装
                          </span>
                        )}
                      </div>
                    </SelectItem>
                  )
                })}
              </SelectContent>
            </Select>
          </div>

          {/* 错误提示 */}
          {error && (
            <div className="flex items-start gap-2 rounded-xl border border-danger/30 bg-danger-soft p-3 text-xs text-danger">
              <AlertCircle className="size-4 shrink-0 mt-0.5" />
              <div className="flex-1 break-all">{error}</div>
            </div>
          )}
        </div>

        <DialogFooter className="gap-2 sm:gap-0 pt-2 border-t border-line/60">
          <Button variant="outline" size="sm" onClick={onClose} className="rounded-xl text-xs">
            {t.about.cancelBtn || "取消"}
          </Button>
          <Button
            size="sm"
            onClick={handleInstall}
            disabled={!trimmedSpec || !selectedProfile}
            className="rounded-xl bg-brand text-white hover:bg-brand/90 text-xs font-medium gap-1.5 shadow-xs"
          >
            <Download className="size-3.5" />
            <span>{selectedProfile ? `安装到 ${selectedProfile}` : t.market.manualInstallSubmit}</span>
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
