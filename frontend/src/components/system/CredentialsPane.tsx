import { useCallback, useEffect, useState } from "react"
import {
  Code2,
  Edit2,
  KeyRound,
  LoaderCircle,
  RefreshCw,
  Save,
  ShieldCheck,
  Trash2,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { useI18n } from "@/stores/i18nStore"
import type { CredentialSummaryItem } from "@/types/ipc"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

export function CredentialsPane({
  onNotice,
}: {
  onNotice?: (text: string, kind?: "ok" | "warn") => void
}) {
  const { t } = useI18n()
  const [summary, setSummary] = useState<CredentialSummaryItem[] | null>(null)
  const [loading, setLoading] = useState(false)
  const [rawMode, setRawMode] = useState(false)
  const [rawContent, setRawContent] = useState("")
  const [savingRaw, setSavingRaw] = useState(false)

  // 弹窗编辑 Key
  const [editingProvider, setEditingProvider] = useState<CredentialSummaryItem | null>(null)
  const [inputKey, setInputKey] = useState("")
  const [savingKey, setSavingKey] = useState(false)

  const loadData = useCallback(async () => {
    setLoading(true)
    try {
      const items = await api.getCredentialsSummary()
      setSummary(items)
      const raw = await api.getCredentialsRaw()
      setRawContent(raw)
    } catch (e) {
      onNotice?.(String(e), "warn")
    } finally {
      setLoading(false)
    }
  }, [onNotice])

  useEffect(() => {
    void loadData()
  }, [loadData])

  const handleSaveKey = async () => {
    if (!editingProvider) return
    setSavingKey(true)
    try {
      await api.setCredentialKey(editingProvider.provider, inputKey)
      onNotice?.(t.console.keySaved, "ok")
      setEditingProvider(null)
      setInputKey("")
      await loadData()
    } catch (e) {
      onNotice?.(String(e), "warn")
    } finally {
      setSavingKey(false)
    }
  }

  const handleDeleteKey = async (item: CredentialSummaryItem) => {
    if (!window.confirm(`确定清除「${item.label}」的 API Key 吗？`)) return
    try {
      await api.setCredentialKey(item.provider, "")
      onNotice?.(t.console.keyRemoved, "ok")
      await loadData()
    } catch (e) {
      onNotice?.(String(e), "warn")
    }
  }

  const handleSaveRaw = async () => {
    setSavingRaw(true)
    try {
      await api.saveCredentialsRaw(rawContent)
      onNotice?.(t.console.credentialsSaved, "ok")
      await loadData()
    } catch (err) {
      onNotice?.(`${t.console.credentialsSaveFailed}: ${err}`, "warn")
    } finally {
      setSavingRaw(false)
    }
  }

  return (
    <div className="space-y-4">
      {/* 顶部标题栏与模式切换 */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <KeyRound className="size-4 text-brand" />
            <h3 className="text-sm font-bold text-ink">
              {t.console.credentialsTitle}
            </h3>
          </div>
          <p className="text-xs text-faint">{t.console.credentialsSubtitle}</p>
        </div>

        <div className="flex items-center gap-2">
          <Button
            size="sm"
            variant="outline"
            onClick={() => setRawMode(!rawMode)}
            className="gap-1.5 text-xs"
          >
            <Code2 className="size-3.5 text-dim" />
            <span>{rawMode ? "卡片视图" : t.console.rawYamlToggle}</span>
          </Button>

          <Button
            size="sm"
            variant="outline"
            onClick={loadData}
            disabled={loading}
            className="gap-1.5 text-xs"
          >
            <RefreshCw className={`size-3.5 ${loading ? "animate-spin text-brand" : "text-dim"}`} />
            <span>刷新</span>
          </Button>
        </div>
      </div>

      {/* 安全保障提示 */}
      <div className="flex items-start gap-2.5 rounded-xl border border-line bg-panel p-3 text-xs text-dim">
        <ShieldCheck className="size-4 text-emerald-500 shrink-0 mt-0.5" />
        <div className="space-y-0.5">
          <span className="font-semibold text-ink">0600 权限保障与前端脱敏</span>
          <p className="text-[11px] text-faint leading-relaxed">
            {t.console.permHint}。前端界面绝不持有全量明文 API Key，仅显示脱敏掩码。
          </p>
        </div>
      </div>

      {rawMode ? (
        /* YAML 原文专家模式 */
        <div className="space-y-3 rounded-2xl border border-line bg-panel p-4 shadow-xs">
          <div className="flex items-center justify-between">
            <span className="font-mono text-xs text-faint">$DSH_HOME/.credentials.yaml</span>
            <Button
              size="sm"
              onClick={handleSaveRaw}
              disabled={savingRaw}
              className="gap-1.5 bg-brand text-white hover:bg-brand/90"
            >
              {savingRaw ? (
                <LoaderCircle className="size-3.5 animate-spin" />
              ) : (
                <Save className="size-3.5" />
              )}
              <span>{t.console.saveCredentials}</span>
            </Button>
          </div>
          <textarea
            value={rawContent}
            onChange={(e) => setRawContent(e.target.value)}
            rows={14}
            className="w-full resize-y rounded-xl border border-line bg-slate-950 p-3.5 font-mono text-xs leading-relaxed text-slate-200 focus:border-brand focus:outline-none"
            placeholder={t.console.credentialsEmpty}
          />
        </div>
      ) : (
        /* 结构化 Provider 卡片列表 */
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          {summary?.map((item) => (
            <div
              key={item.provider}
              className="flex flex-col justify-between rounded-xl border border-line bg-panel p-3.5 shadow-2xs transition-colors hover:border-brand/30"
            >
              <div>
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-1.5 min-w-0 flex-1">
                    <span className="text-xs font-bold text-ink truncate" title={item.label}>
                      {item.label}
                    </span>
                    {item.label.toLowerCase() !== item.provider.toLowerCase() && (
                      <span className="font-mono text-[10px] text-faint truncate" title={item.provider}>
                        ({item.provider})
                      </span>
                    )}
                  </div>
                  {item.configured ? (
                    <span className="rounded-md bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-medium text-emerald-600 dark:text-emerald-400 shrink-0 whitespace-nowrap shadow-2xs">
                      {t.console.configuredTag}
                    </span>
                  ) : (
                    <span className="rounded-md bg-line-soft px-1.5 py-0.5 text-[10px] text-faint shrink-0 whitespace-nowrap">
                      {t.console.notConfiguredTag}
                    </span>
                  )}
                </div>

                <div className="mt-2.5 rounded-lg border border-line bg-bg px-2.5 py-1.5 font-mono text-xs text-dim">
                  {item.configured ? (
                    <span className="tracking-wider text-ink font-semibold">
                      {item.maskedKey}
                    </span>
                  ) : (
                    <span className="text-faint text-[11px] italic">尚未配置 API Key</span>
                  )}
                </div>
              </div>

              <div className="mt-3 flex items-center justify-end gap-2 border-t border-line/50 pt-2.5">
                {item.configured && (
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => handleDeleteKey(item)}
                    className="h-7 px-2 text-xs text-faint hover:text-rose-500"
                  >
                    <Trash2 className="size-3 mr-1" />
                    <span>{t.console.deleteKey}</span>
                  </Button>
                )}
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => {
                    setEditingProvider(item)
                    setInputKey("")
                  }}
                  className="h-7 gap-1 px-2.5 text-xs hover:border-brand hover:text-brand"
                >
                  <Edit2 className="size-3" />
                  <span>{item.configured ? "修改 Key" : t.console.editKey}</span>
                </Button>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* 设置 API Key 对话框 */}
      <Dialog
        open={editingProvider !== null}
        onOpenChange={(open) => {
          if (!open) {
            setEditingProvider(null)
            setInputKey("")
          }
        }}
      >
        <DialogContent className="sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-sm font-bold">
              <KeyRound className="size-4 text-brand" />
              <span>{editingProvider ? t.console.keyModalTitle(editingProvider.label) : ""}</span>
            </DialogTitle>
            <DialogDescription className="text-xs text-faint">
              {t.console.keyModalDesc}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-2 py-2">
            <input
              type="password"
              value={inputKey}
              onChange={(e) => setInputKey(e.target.value)}
              placeholder="sk-..."
              className="w-full rounded-xl border border-line bg-bg px-3 py-2 font-mono text-xs text-ink outline-none focus:border-brand"
              autoFocus
            />
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setEditingProvider(null)}
              disabled={savingKey}
            >
              取消
            </Button>
            <Button
              onClick={handleSaveKey}
              disabled={savingKey || !inputKey.trim()}
              className="bg-brand text-white hover:bg-brand/90"
            >
              {savingKey && <LoaderCircle className="size-3.5 animate-spin mr-1.5" />}
              <span>保存 API Key</span>
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
