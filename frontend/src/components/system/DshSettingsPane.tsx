// DshSettingsPane.tsx —— DSH 引擎全局配置文件管理（`$DSH_HOME/settings.yaml`，4.5）。
import { useEffect, useState } from "react"
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
import { useI18n } from "@/stores/i18nStore"
import { Button } from "@/components/ui/button"

export function DshSettingsPane({
  onNotice,
}: {
  onNotice?: (text: string, kind?: "ok" | "warn") => void
}) {
  const { t } = useI18n()
  const [content, setContent] = useState("")
  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)
  const [copied, setCopied] = useState(false)

  const loadData = async () => {
    setLoading(true)
    try {
      const text = await api.getDshSettingsRaw()
      setContent(text)
    } catch (e) {
      onNotice?.(String(e), "warn")
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    void loadData()
  }, [])

  const handleSave = async () => {
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

  const handleCopy = () => {
    void navigator.clipboard.writeText(content).then(() => {
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    })
  }

  return (
    <div className="space-y-4">
      {/* 顶部标题与操作栏 */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <Sliders className="size-4 text-brand" />
            <h3 className="text-sm font-bold text-ink">
              {t.console.dshSettingsTitle}
            </h3>
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
              <Check className="size-3.5 text-emerald-500" />
            ) : (
              <Copy className="size-3.5 text-dim" />
            )}
            <span>{copied ? "已复制" : "复制代码"}</span>
          </Button>

          <Button
            size="sm"
            variant="outline"
            onClick={loadData}
            disabled={loading || saving}
            className="gap-1.5 text-xs"
          >
            <RefreshCw className={`size-3.5 ${loading ? "animate-spin text-brand" : "text-dim"}`} />
            <span>重新加载</span>
          </Button>

          <Button
            size="sm"
            onClick={handleSave}
            disabled={saving || loading}
            className="gap-1.5 bg-brand text-white hover:bg-brand/90 text-xs"
          >
            {saving ? (
              <LoaderCircle className="size-3.5 animate-spin" />
            ) : (
              <Save className="size-3.5" />
            )}
            <span>保存配置</span>
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
    </div>
  )
}
