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
import { validatePluginSpec } from "@/lib/profiles"
import {
  detectInstallSource,
  getPluginDescription,
  getPluginDisplayName,
} from "@/lib/market"
import { BuildApprovalDialog } from "@/components/profiles/BuildApprovalDialog"
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
  // pnpm 12 构建审批门：非空 = 弹逐包裁决框（保存后重试安装）
  const [gatePkgs, setGatePkgs] = useState<string[] | null>(null)

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
    // 提交前预检（ADR-0011 齐口径）：market spec 坏形时给出可读文案而非
    // 等后端校验失败再回显
    const invalid = validatePluginSpec(sourceInfo.spec.trim())
    if (invalid) {
      setError(invalid)
      return
    }
    setInstalling(true)
    setError(null)

    try {
      const outcome = await api.installPlugin(selectedProfile, sourceInfo.spec.trim())
      if (outcome.ok) {
        onSuccess(plugin.name, selectedProfile)
        onClose()
      } else if (outcome.ignored_builds?.length) {
        setGatePkgs(outcome.ignored_builds)
        setError(null)
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
    <>
    <Dialog open={open} onOpenChange={(val) => !installing && !val && onClose()}>
      <DialogContent className="max-w-md rounded-2xl border border-line bg-panel p-6 shadow-xl">
        <DialogHeader>
          <div className="flex items-center gap-3">
            <div className="flex size-10 shrink-0 items-center justify-center rounded-xl border border-brand/30 bg-brand/10 text-brand-deep shadow-2xs">
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
                  <div className="text-meta text-faint font-mono truncate" title={plugin.name}>
                    {plugin.name}
                  </div>
                )}
              </div>
              <div className="shrink-0 text-right">
                <span className="text-label text-dim font-mono">by {plugin.owner}</span>
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
            <span className="text-xs font-medium text-ink flex items-center justify-between">
              <span>{t.market.selectProfile}</span>
              {selectedProfile && isAlreadyInstalled && (
                <span className="text-meta text-amber-700 font-mono flex items-center gap-1">
                  <AlertCircle className="size-3" />
                  已在此 Profile 安装（将执行覆盖/重装）
                </span>
              )}
            </span>
            <Select value={selectedProfile} onValueChange={setSelectedProfile} disabled={installing}>
              <SelectTrigger
                aria-label={t.market.selectProfile}
                className="w-full h-9 rounded-xl border-line bg-panel text-ink text-xs"
              >
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
                          <span className="rounded bg-brand/10 px-1 py-0.2 text-micro text-brand-deep">Web</span>
                        )}
                        {hasIt && (
                          <span className="rounded bg-emerald-500/10 px-1 py-0.2 text-micro text-emerald-700">
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
              <span className="text-xs font-medium text-ink">
                {t.market.installSpecLabel}
              </span>
              {/* 自动识别徽标 */}
              {sourceInfo.type === "npm" ? (
                <span className="inline-flex items-center gap-1 rounded-md border border-emerald-500/30 bg-emerald-500/10 px-1.5 py-0.5 font-mono text-meta font-medium text-emerald-700 shadow-2xs">
                  <Package className="size-3" />
                  <span>{t.market.sourceNpm}</span>
                </span>
              ) : (
                <span className="inline-flex items-center gap-1 rounded-md border border-purple-500/30 bg-purple-500/10 px-1.5 py-0.5 font-mono text-meta font-medium text-purple-600 shadow-2xs">
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
            <div className="flex items-start gap-2 rounded-xl border border-rose-500/30 bg-rose-500/10 p-3 text-xs text-rose-600">
              <AlertCircle className="size-4 shrink-0 mt-0.5" />
              <div className="flex-1 break-all">{error}</div>
            </div>
          )}

          {/* 进行中提示 */}
          {installing && (
            <div className="flex items-center gap-2 rounded-xl border border-brand/30 bg-brand/10 p-3 text-xs text-brand-deep animate-pulse">
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
            className="rounded-xl bg-brand-deep text-white hover:bg-brand-deep/90 text-xs font-medium gap-1.5 shadow-xs"
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
    <BuildApprovalDialog
      profile={selectedProfile}
      packages={gatePkgs ?? []}
      open={gatePkgs !== null}
      onClose={() => setGatePkgs(null)}
      onApproved={() => {
        setGatePkgs(null)
        handleInstall()
      }}
    />
    </>
  )
}
