// DshSettingsPane.tsx —— DSH 引擎全局配置文件管理（`$DSH_HOME/settings.yaml`，4.5）。
import { useCallback, useEffect, useState } from "react"
import {
  Check,
  Copy,
  FileCode2,
  LoaderCircle,
  RefreshCw,
  Save,
  Sliders,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { useCopy } from "@/hooks/useCopy"
import { useI18n } from "@/stores/i18nStore"
import { Button } from "@/components/ui/button"
import { ConfirmDialog } from "@/components/ui/confirm-dialog"

export function DshSettingsPane({
  onNotice,
}: {
  onNotice?: (text: string, kind?: "ok" | "warn") => void
}) {
  const { t } = useI18n()
  const [content, setContent] = useState("")
  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)
  // 覆写确认（2026-09-08，U9）：整文件重写前先过确认对话框
  const [confirmSave, setConfirmSave] = useState(false)
  const { copied, copy } = useCopy()

  const loadData = useCallback(async () => {    setLoading(true)
    try {
      const text = await api.getDshSettingsRaw()
      setContent(text)
    } catch (e) {
      onNotice?.(String(e), "warn")
    } finally {
      setLoading(false)
    }
  }, [onNotice])

  useEffect(() => {
    void loadData()
  }, [loadData])

  // 确认后执行（对话框已关闭，进度由按钮的 saving 态回报）
  const saveNow = async () => {
    setSaving(true)
    try {
      await api.saveDshSettingsRaw(content)
      onNotice?.(t.console.dshSettingsSaved, "ok")
      await loadData()
    } catch (e) {
      onNotice?.(String(e), "warn")
    } finally {
      setSaving(false)
    }
  }

  const handleCopy = async () => {
    // 2026-09-08：写失败不再静默（原来只挂 .then 成功分支）
    const outcome = await copy(content)
    if (!outcome.ok) onNotice?.(t.error.copyFailed, "warn")
  }

  return (
    <div className="space-y-4">
      {/* 顶部标题与操作栏 */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <Sliders className="size-4 text-brand-deep" />
            <h2 className="text-sm font-bold text-ink">
              {t.console.dshSettingsTitle}
            </h2>
          </div>
          <p className="text-xs text-faint">{t.console.dshSettingsSubtitle}</p>
        </div>

        <div className="flex items-center gap-2">
          <Button
            size="sm"
            variant="outline"
            onClick={handleCopy}
            disabled={!content}
            className="gap-1.5 text-xs"
          >
            {copied ? (
              <Check className="size-3.5 text-emerald-700" />
            ) : (
              <Copy className="size-3.5 text-dim" />
            )}
            <span>{copied ? "已复制" : "复制 YAML"}</span>
          </Button>

          <Button
            size="sm"
            variant="outline"
            onClick={loadData}
            disabled={loading || saving}
            className="gap-1.5 text-xs"
          >
            <RefreshCw className={`size-3.5 ${loading ? "animate-spin text-brand-deep" : "text-dim"}`} />
            <span>重新加载</span>
          </Button>

          <Button
            size="sm"
            onClick={() => setConfirmSave(true)}
            disabled={saving || loading}
            className="gap-1.5 bg-brand-deep text-white hover:bg-brand-deep/90 text-xs"
          >
            {saving ? (
              <LoaderCircle className="size-3.5 animate-spin" />
            ) : (
              <Save className="size-3.5" />
            )}
            <span>{t.console.dshSettingsSave}</span>
          </Button>
        </div>
      </div>

      {/* YAML 编辑器区域 */}
      <div className="rounded-2xl border border-line bg-panel p-4 shadow-xs space-y-2">
        <div className="flex items-center justify-between text-xs text-faint">
          <div className="flex items-center gap-1.5 font-mono">
            <FileCode2 className="size-3.5" />
            <span>$DSH_HOME/settings.yaml</span>
          </div>
          <span>YAML 格式</span>
        </div>

        <textarea
          value={content}
          onChange={(e) => setContent(e.target.value)}
          rows={16}
          className="w-full resize-y rounded-xl border border-line bg-slate-950 p-4 font-mono text-xs leading-relaxed text-slate-200 focus:border-brand focus:outline-none"
          placeholder="# DSH settings.yaml\n# model: deepseek-chat\n# defaultProvider: deepseek"
        />
      </div>

      {/* 覆写确认（U9）：整文件重写，先列明影响面 */}
      <ConfirmDialog
        open={confirmSave}
        title={t.console.dshSettingsOverwriteConfirm}
        points={t.console.dshSettingsOverwritePoints}
        confirmLabel={t.console.dshSettingsSave}
        cancelLabel={t.confirm.cancel}
        onConfirm={() => {
          setConfirmSave(false)
          void saveNow()
        }}
        onClose={() => setConfirmSave(false)}
      />
    </div>
  )
}
