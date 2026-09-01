// components/market/MarketInstallDialog.tsx —— 插件市场安装 / 分发模态对话框
import { useEffect, useMemo, useState } from "react"
import {
  AlertCircle,
  Code2,
  Download,
  Loader2,
  Package,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { useI18n } from "@/stores/i18nStore"
import type { ProfileSummary } from "@/types/ipc"
import type { MarketPlugin } from "@/types/market"
import {
  detectInstallSource,
  getPluginDescription,
  getPluginDisplayName,
} from "@/lib/market"
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

interface MarketInstallDialogProps {
  plugin: MarketPlugin | null
  profiles: ProfileSummary[]
  installedProfiles: string[]
  open: boolean
  onClose: () => void
  onSuccess: (pluginName: string, targetProfile: string) => void
}

export function MarketInstallDialog({
  plugin,
  profiles,
  installedProfiles,
  open,
  onClose,
  onSuccess,
}: MarketInstallDialogProps) {
  const { t, activeLocale } = useI18n()
  const [selectedProfile, setSelectedProfile] = useState<string>("")
  const [installing, setInstalling] = useState(false)
  const [error, setError] = useState<string | null>(null)

  // 提取简短展示名与安装源元数据
  const displayName = useMemo(
    () => (plugin ? getPluginDisplayName(plugin.name) : ""),
    [plugin],
  )
  const sourceInfo = useMemo(
    () => (plugin ? detectInstallSource(plugin) : null),
    [plugin],
  )
  const desc = useMemo(
    () => (plugin ? getPluginDescription(plugin.description, activeLocale) : ""),
    [plugin, activeLocale],
  )

  useEffect(() => {
    if (plugin) {
      setError(null)
      // 默认选择第一个未安装该插件的 Profile，如果全装了则选第一个
      const notInstalled = profiles.find((p) => !installedProfiles.includes(p.name))
      setSelectedProfile(notInstalled ? notInstalled.name : profiles[0]?.name || "web")
    }
  }, [plugin, profiles, installedProfiles])

  if (!plugin || !sourceInfo) return null

  const isAlreadyInstalled = installedProfiles.includes(selectedProfile)

  const handleInstall = async () => {
    if (!selectedProfile || !sourceInfo.spec.trim()) return
    setInstalling(true)
    setError(null)

    try {
      const outcome = await api.installPlugin(selectedProfile, sourceInfo.spec.trim())
      if (outcome.ok) {
        onSuccess(plugin.name, selectedProfile)
        onClose()
      } else {
        setError(outcome.detail || "安装失败")
      }
    } catch (err) {
      setError(String(err))
    } finally {
      setInstalling(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={(val) => !installing && !val && onClose()}>
      <DialogContent className="max-w-md rounded-2xl border border-line bg-panel p-6 shadow-xl">
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div className="flex size-10 shrink-0 items-center justify-center rounded-xl border border-brand/30 bg-brand/10 text-brand shadow-2xs">
              <Download className="size-5" />
            </div>
            <div className="min-w-0 flex-1">
              <DialogTitle className="text-base font-bold text-ink truncate" title={plugin.name}>
                {t.market.installModalTitle(displayName)}
              </DialogTitle>
              <DialogDescription className="text-xs text-dim mt-0.5 line-clamp-2">
                {t.market.installModalDesc}
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <div className="space-y-4 py-2 text-xs">
          {/* 插件基本信息卡片（防换行溢出优化） */}
          <div className="rounded-xl border border-line bg-wash p-3.5 space-y-2">
            <div className="flex items-start justify-between gap-2.5">
              <div className="min-w-0 flex-1">
                <div className="font-mono font-bold text-ink text-sm truncate" title={plugin.name}>
                  {displayName}
                </div>
                {displayName !== plugin.name && (
                  <div className="text-[10px] text-faint font-mono truncate" title={plugin.name}>
                    {plugin.name}
                  </div>
                )}
              </div>
              <div className="shrink-0 text-right">
                <span className="text-[11px] text-dim font-mono">by {plugin.owner}</span>
              </div>
            </div>

            {desc && (
              <p className="text-xs text-faint line-clamp-2 leading-relaxed pt-1 border-t border-line/60">
                {desc}
              </p>
            )}
          </div>

          {/* 目标 Profile 选择 */}
          <div className="space-y-1.5">
            <label className="text-xs font-medium text-ink flex items-center justify-between">
              <span>{t.market.selectProfile}</span>
              {selectedProfile && isAlreadyInstalled && (
                <span className="text-[10px] text-amber-500 font-mono flex items-center gap-1">
                  <AlertCircle className="size-3" />
                  已在此 Profile 安装（将执行覆盖/重装）
                </span>
              )}
            </label>
            <Select value={selectedProfile} onValueChange={setSelectedProfile} disabled={installing}>
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
                          <span className="rounded bg-brand/10 px-1 py-0.2 text-[9px] text-brand">Web</span>
                        )}
                        {hasIt && (
                          <span className="rounded bg-emerald-500/10 px-1 py-0.2 text-[9px] text-emerald-600 dark:text-emerald-400">
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

          {/* 安装规范 Spec（只读展示，自动识别 NPM / GitHub） */}
          <div className="space-y-1.5">
            <div className="flex items-center justify-between">
              <label className="text-xs font-medium text-ink">
                {t.market.installSpecLabel}
              </label>
              {/* 自动识别徽标 */}
              {sourceInfo.type === "npm" ? (
                <span className="inline-flex items-center gap-1 rounded-md border border-emerald-500/30 bg-emerald-500/10 px-1.5 py-0.5 font-mono text-[10px] font-medium text-emerald-600 dark:text-emerald-400 shadow-2xs">
                  <Package className="size-3" />
                  <span>{t.market.sourceNpm}</span>
                </span>
              ) : (
                <span className="inline-flex items-center gap-1 rounded-md border border-purple-500/30 bg-purple-500/10 px-1.5 py-0.5 font-mono text-[10px] font-medium text-purple-600 dark:text-purple-400 shadow-2xs">
                  <Code2 className="size-3" />
                  <span>{t.market.sourceGithub}</span>
                </span>
              )}
            </div>

            {/* 安装源显示框（不可编辑） */}
            <div className="flex items-center justify-between rounded-xl border border-line bg-wash/80 px-3 py-2 font-mono text-xs text-ink shadow-2xs">
              <span className="truncate select-all font-medium text-ink flex-1" title={sourceInfo.spec}>
                {sourceInfo.spec}
              </span>
            </div>
          </div>

          {/* 错误提示 */}
          {error && (
            <div className="flex items-start gap-2 rounded-xl border border-rose-500/30 bg-rose-500/10 p-3 text-xs text-rose-600 dark:text-rose-400">
              <AlertCircle className="size-4 shrink-0 mt-0.5" />
              <div className="flex-1 break-all">{error}</div>
            </div>
          )}

          {/* 进行中提示 */}
          {installing && (
            <div className="flex items-center gap-2 rounded-xl border border-brand/30 bg-brand/10 p-3 text-xs text-brand animate-pulse">
              <Loader2 className="size-4 shrink-0 animate-spin" />
              <span>{t.market.installingBusy}</span>
            </div>
          )}
        </div>

        <DialogFooter className="gap-2 sm:gap-0 pt-2 border-t border-line/60">
          <Button variant="outline" size="sm" onClick={onClose} disabled={installing} className="rounded-xl text-xs">
            {t.profiles.pluginInstallCancel}
          </Button>
          <Button
            size="sm"
            onClick={handleInstall}
            disabled={installing || !selectedProfile || !sourceInfo.spec.trim()}
            className="rounded-xl bg-brand text-white hover:bg-brand/90 text-xs font-medium gap-1.5 shadow-xs"
          >
            {installing ? (
              <>
                <Loader2 className="size-3.5 animate-spin" />
                <span>安装中…</span>
              </>
            ) : (
              <>
                <Download className="size-3.5" />
                <span>{isAlreadyInstalled ? "重新安装" : t.market.installBtn}</span>
              </>
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
