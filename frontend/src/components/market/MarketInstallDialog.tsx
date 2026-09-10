// components/market/MarketInstallDialog.tsx —— 插件市场安装 / 分发确认对话框
// 2026-09-09 队列化（095 #4 / ADR-0011 队列形态）：确认目标与安装源后**入队**
// 即关闭——后台串行执行（构建脚本审批门已随 ADR-0013 退役，安装无中间态）。
import { useEffect, useMemo, useState } from "react"
import {
  AlertCircle,
  Code2,
  Download,
  Package,
} from "lucide-react"
import { useQueueStore } from "@/stores/queueStore"
import { useInstallFlightStore } from "@/stores/installFlightStore"
import { useI18n } from "@/stores/i18nStore"
import type { ProfileSummary } from "@/types/ipc"
import type { MarketPlugin } from "@/types/market"
import { validatePluginSpec } from "@/lib/profiles"
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
  onNotice?: (text: string, kind?: "ok" | "warn") => void
}

export function MarketInstallDialog({
  plugin,
  profiles,
  installedProfiles,
  open,
  onClose,
  onNotice,
}: MarketInstallDialogProps) {
  const { t, activeLocale } = useI18n()
  const [selectedProfile, setSelectedProfile] = useState<string>("")
  const [error, setError] = useState<string | null>(null)
  const enqueue = useQueueStore((s) => s.enqueue)

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
      // 默认选择第一个未安装该插件的 Profile，如果全装了则选第一个。
      // 只随「换插件」重算——profiles/installedProfiles 的属性引用在父组件
      // 每次 focus 刷新/装后回填时都会重建，纳入依赖会把用户手动改选的
      // 目标静默重置回默认（2026-09-09「选 web 批准装进 test」根因之一）。
      const notInstalled = profiles.find((p) => !installedProfiles.includes(p.name))
      setSelectedProfile(notInstalled ? notInstalled.name : profiles[0]?.name || "web")
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps -- 仅随弹窗目标插件重算默认选中
  }, [plugin])

  if (!plugin || !sourceInfo) return null

  const isAlreadyInstalled = installedProfiles.includes(selectedProfile)

  // 入队（ADR-0011 队列形态）：预检 → 队列串行执行；目标 profile 入队瞬间
  // 快照绑定。顺手起飞一颗胶囊到「下载管理」，并弹 Toast 告知用户任务已被接管。
  const handleEnqueue = (e: React.MouseEvent) => {
    if (!plugin || !selectedProfile || !sourceInfo.spec.trim()) return
    const spec = sourceInfo.spec.trim()
    const invalid = validatePluginSpec(spec)
    if (invalid) {
      setError(invalid)
      return
    }
    enqueue({ pkg: plugin.name, spec, profile: selectedProfile, kind: "install" })
    useInstallFlightStore.getState().launch({ x: e.clientX, y: e.clientY }, displayName)
    onNotice?.(t.market.installQueued(displayName, selectedProfile), "ok")
    onClose()
  }

  return (
    <>
    <Dialog open={open} onOpenChange={(val) => !val && onClose()}>
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
                  <div className="text-micro text-faint font-mono truncate" title={plugin.name}>
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
            <label className="text-xs font-medium text-ink flex items-center justify-between">
              <span>{t.market.selectProfile}</span>
              {selectedProfile && isAlreadyInstalled && (
                <span className="text-meta text-warn font-mono flex items-center gap-1">
                  <AlertCircle className="size-3" />
                  已在此 Profile 安装（将执行覆盖/重装）
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

          {/* 安装规范 Spec（只读展示，自动识别 NPM / GitHub） */}
          <div className="space-y-1.5">
            <div className="flex items-center justify-between">
              <label className="text-xs font-medium text-ink">
                {t.market.installSpecLabel}
              </label>
              {/* 自动识别徽标 */}
              {sourceInfo.type === "npm" ? (
                <span className="inline-flex items-center gap-1 rounded-md border border-line bg-line-soft px-1.5 py-0.5 font-mono text-meta font-medium text-dim shadow-2xs">
                  <Package className="size-3" />
                  <span>{t.market.sourceNpm}</span>
                </span>
              ) : (
                <span className="inline-flex items-center gap-1 rounded-md border border-line bg-line-soft px-1.5 py-0.5 font-mono text-meta font-medium text-dim shadow-2xs">
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
            <div className="flex items-start gap-2 rounded-xl border border-danger/30 bg-danger-soft p-3 text-xs text-danger">
              <AlertCircle className="size-4 shrink-0 mt-0.5" />
              <div className="flex-1 break-all">{error}</div>
            </div>
          )}
        </div>

        <DialogFooter className="gap-2 sm:gap-0 pt-2 border-t border-line/60">
          <Button variant="outline" size="sm" onClick={onClose} className="rounded-xl text-xs">
            {t.profiles.pluginInstallCancel}
          </Button>
          <Button
            size="sm"
            onClick={handleEnqueue}
            disabled={!selectedProfile || !sourceInfo.spec.trim()}
            className="rounded-xl bg-brand text-white hover:bg-brand/90 text-xs font-medium gap-1.5 shadow-xs"
          >
            <Download className="size-3.5" />
            <span>{isAlreadyInstalled ? "重新安装" : t.market.installBtn}</span>
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
    </>
  )
}
