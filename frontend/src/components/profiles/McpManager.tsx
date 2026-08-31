// McpManager.tsx —— Profile 的 MCP 服务器可视化结构化管理工作台（4.7 完整版）。
import { useCallback, useEffect, useMemo, useState } from "react"
import {
  Boxes,
  Check,
  Code2,
  Copy,
  Database,
  Edit2,
  Globe,
  HardDrive,
  LoaderCircle,
  Plus,
  PlusCircle,
  RefreshCw,
  Server,
  Sparkles,
  Terminal,
  Trash2,
  Wrench,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { useI18n } from "@/stores/i18nStore"
import type { McpServerConfig, PluginRuntimeSnapshot } from "@/types/ipc"
import { Button } from "@/components/ui/button"
import { Switch } from "@/components/ui/switch"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

const MCP_PRESETS: Array<{
  name: string
  label: string
  desc: string
  icon: typeof Code2
  command: string
  args: string[]
  env: Record<string, string>
}> = [
  {
    name: "github",
    label: "GitHub Tools",
    desc: "搜索代码、管理 Issue、PR 与提交历史",
    icon: Code2,
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-github"],
    env: { GITHUB_PERSONAL_ACCESS_TOKEN: "" },
  },
  {
    name: "filesystem",
    label: "Filesystem Sandbox",
    desc: "安全的本地指定目录文件读写沙箱能力",
    icon: HardDrive,
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/workspace"],
    env: {},
  },
  {
    name: "postgres",
    label: "PostgreSQL Database",
    desc: "连接 PostgreSQL 数据库并执行安全只读/读写 SQL",
    icon: Database,
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-postgres", "postgresql://user:pass@localhost/db"],
    env: {},
  },
  {
    name: "brave-search",
    label: "Brave Web Search",
    desc: "通过 Brave Search API 进行实时全网检索",
    icon: Globe,
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-brave-search"],
    env: { BRAVE_API_KEY: "" },
  },
]

export function McpManager({
  profileName,
  onNotice,
}: {
  profileName: string
  patchYaml?: string | null
  onNotice?: (msg: string, kind?: "ok" | "warn") => void
}) {
  const { t } = useI18n()
  const [servers, setServers] = useState<McpServerConfig[] | null>(null)
  const [runtime, setRuntime] = useState<PluginRuntimeSnapshot | null>(null)
  const [loading, setLoading] = useState(false)
  const [copiedName, setCopiedName] = useState<string | null>(null)

  // 新建/编辑 Dialog 状态
  const [dialogOpen, setDialogOpen] = useState(false)
  const [formName, setFormName] = useState("")
  const [formCommand, setFormCommand] = useState("npx")
  const [formArgs, setFormArgs] = useState("")
  const [formEnv, setFormEnv] = useState<Array<{ key: string; value: string }>>([])
  const [formDisabled, setFormDisabled] = useState(false)
  const [saving, setSaving] = useState(false)
  const [deletingName, setDeletingName] = useState<string | null>(null)

  const loadData = useCallback(async () => {
    setLoading(true)
    try {
      const list = await api.listMcpServers(profileName)
      setServers(list)
      const rt = await api.getPluginRuntime()
      setRuntime(rt)
    } catch (e) {
      onNotice?.(String(e), "warn")
    } finally {
      setLoading(false)
    }
  }, [profileName, onNotice])

  useEffect(() => {
    void loadData()
  }, [loadData])

  // 提取运行态导出的 MCP 工具列表（按 serverName 归类）
  const activeToolsByServer = useMemo(() => {
    const map = new Map<string, string[]>()
    if (!runtime || !runtime.profile || runtime.profile !== profileName) {
      return map
    }

    // 从 runtime.entries 中扫描导出项或者名字形如 mcp__<server>__<tool>
    for (const entry of runtime.entries) {
      const identifier = entry.module_name || entry.entry_id || ""
      if (identifier.startsWith("mcp__")) {
        const parts = identifier.split("__")
        if (parts.length >= 3) {
          const srv = parts[1]
          const tool = parts.slice(2).join("__")
          if (!map.has(srv)) map.set(srv, [])
          map.get(srv)!.push(tool)
        }
      }
    }
    return map
  }, [runtime, profileName])

  const openCreateDialog = () => {
    setFormName("")
    setFormCommand("npx")
    setFormArgs("")
    setFormEnv([])
    setFormDisabled(false)
    setDialogOpen(true)
  }

  const openEditDialog = (srv: McpServerConfig) => {
    setFormName(srv.name)
    setFormCommand(srv.command)
    setFormArgs(srv.args.join(" "))
    const envArr = Object.entries(srv.env).map(([key, value]) => ({ key, value }))
    setFormEnv(envArr)
    setFormDisabled(srv.disabled)
    setDialogOpen(true)
  }

  const applyPreset = (preset: typeof MCP_PRESETS[0]) => {
    setFormName(preset.name)
    setFormCommand(preset.command)
    setFormArgs(preset.args.join(" "))
    const envArr = Object.entries(preset.env).map(([key, value]) => ({ key, value }))
    setFormEnv(envArr)
    setDialogOpen(true)
  }

  const handleSaveServer = async () => {
    if (!formName.trim()) {
      onNotice?.("请输入服务名称", "warn")
      return
    }
    setSaving(true)
    try {
      const args = formArgs
        .trim()
        .split(/\s+/)
        .filter(Boolean)
      const env: Record<string, string> = {}
      for (const item of formEnv) {
        if (item.key.trim()) {
          env[item.key.trim()] = item.value.trim()
        }
      }

      const srv: McpServerConfig = {
        name: formName.trim(),
        command: formCommand.trim() || "npx",
        args,
        env,
        disabled: formDisabled,
      }

      await api.saveMcpServer(profileName, srv)
      onNotice?.(t.profiles.mcpSaveSuccess(srv.name), "ok")
      setDialogOpen(false)
      await loadData()
    } catch (e) {
      onNotice?.(String(e), "warn")
    } finally {
      setSaving(false)
    }
  }

  const handleDeleteServer = async (srvName: string) => {
    if (!window.confirm(t.profiles.mcpDeleteConfirm(srvName))) return
    setDeletingName(srvName)
    try {
      await api.deleteMcpServer(profileName, srvName)
      onNotice?.(t.profiles.mcpDeleteSuccess, "ok")
      await loadData()
    } catch (e) {
      onNotice?.(String(e), "warn")
    } finally {
      setDeletingName(null)
    }
  }

  const copyPrefix = (serverName: string) => {
    const prefix = `mcp__${serverName}__*`
    void navigator.clipboard.writeText(prefix).then(() => {
      setCopiedName(serverName)
      onNotice?.(`已复制工具匹配前缀：${prefix}`, "ok")
      setTimeout(() => setCopiedName(null), 2000)
    })
  }

  return (
    <div className="space-y-4">
      {/* 标题与操作栏 */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <Boxes className="size-4 text-brand" />
            <h3 className="text-sm font-bold text-ink">
              {t.profiles.mcpTitle}
            </h3>
          </div>
          <p className="text-xs text-faint">{t.profiles.mcpSubtitle}</p>
        </div>

        <div className="flex items-center gap-2">
          <Button
            size="sm"
            variant="outline"
            onClick={loadData}
            disabled={loading}
            className="gap-1 text-xs"
          >
            <RefreshCw className={`size-3.5 ${loading ? "animate-spin text-brand" : "text-dim"}`} />
            <span>刷新</span>
          </Button>

          <Button
            size="sm"
            onClick={openCreateDialog}
            className="gap-1.5 bg-brand text-white hover:bg-brand/90 text-xs shadow-xs"
          >
            <Plus className="size-3.5" />
            <span>{t.profiles.mcpAddBtn}</span>
          </Button>
        </div>
      </div>

      {/* 已配置的 MCP 服务器列表 */}
      {loading && !servers ? (
        <div className="flex flex-col items-center justify-center rounded-2xl border border-line bg-panel py-12 text-center">
          <LoaderCircle className="size-6 animate-spin text-brand" />
          <span className="text-faint mt-2 text-xs">正在读取 MCP 服务配置...</span>
        </div>
      ) : !servers || servers.length === 0 ? (
        <div className="flex flex-col items-center justify-center rounded-2xl border border-dashed border-line bg-panel/50 p-8 text-center">
          <Server className="size-8 text-faint mb-2" />
          <p className="text-xs font-medium text-ink">{t.profiles.mcpEmpty}</p>
          <p className="text-[11px] text-faint mt-1 max-w-sm">
            支持一键添加 GitHub、Postgres、Brave Search 等 MCP 官方工具库。
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 gap-3">
          {servers.map((s) => {
            const activeTools = activeToolsByServer.get(s.name) || []
            const isDeleting = deletingName === s.name

            return (
              <div
                key={s.name}
                className="flex flex-col justify-between gap-3 rounded-2xl border border-line bg-panel p-4 shadow-xs transition-colors hover:border-brand/40"
              >
                <div className="space-y-2">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <div className="flex items-center gap-2">
                      <span className="font-mono text-xs font-bold text-ink">
                        {s.name}
                      </span>
                      {s.disabled ? (
                        <span className="rounded-md bg-line-soft px-1.5 py-0.5 text-[10px] text-faint">
                          已禁用
                        </span>
                      ) : (
                        <span className="rounded-md bg-emerald-500/10 px-1.5 py-0.5 text-[10px] font-medium text-emerald-600 dark:text-emerald-400">
                          Active
                        </span>
                      )}

                      <button
                        type="button"
                        onClick={() => copyPrefix(s.name)}
                        className="flex items-center gap-1 font-mono text-[10px] text-faint hover:text-ink rounded px-1.5 py-0.5 border border-line bg-bg transition-colors"
                        title="复制工具前缀"
                      >
                        {copiedName === s.name ? (
                          <Check className="size-2.5 text-emerald-500" />
                        ) : (
                          <Copy className="size-2.5" />
                        )}
                        <span>mcp__{s.name}__*</span>
                      </button>
                    </div>

                    <div className="flex items-center gap-1.5">
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => openEditDialog(s)}
                        className="h-7 gap-1 px-2 text-xs"
                      >
                        <Edit2 className="size-3" />
                        <span>{t.profiles.mcpEditBtn}</span>
                      </Button>
                      <Button
                        size="sm"
                        variant="outline"
                        onClick={() => handleDeleteServer(s.name)}
                        disabled={isDeleting}
                        className="size-7 p-0 hover:border-rose-500/50 hover:bg-rose-500/10 hover:text-rose-500"
                      >
                        {isDeleting ? (
                          <LoaderCircle className="size-3 animate-spin text-rose-500" />
                        ) : (
                          <Trash2 className="size-3 text-faint" />
                        )}
                      </Button>
                    </div>
                  </div>

                  {/* 命令与参数 */}
                  <div className="flex items-center gap-2 font-mono text-xs text-dim bg-bg rounded-lg border border-line p-2">
                    <Terminal className="size-3.5 text-faint shrink-0" />
                    <span className="truncate">{s.command} {s.args.join(" ")}</span>
                  </div>

                  {/* 环境变量标签 */}
                  {Object.keys(s.env).length > 0 && (
                    <div className="flex flex-wrap items-center gap-1.5 pt-1 text-[11px] text-faint">
                      <span className="font-semibold text-ink">ENV:</span>
                      {Object.keys(s.env).map((k) => (
                        <span key={k} className="rounded bg-line px-1.5 py-0.5 font-mono text-[10px]">
                          {k}=••••
                        </span>
                      ))}
                    </div>
                  )}

                  {/* 运行态工具联动展示 */}
                  {activeTools.length > 0 ? (
                    <div className="rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-2 text-xs">
                      <div className="flex items-center gap-1.5 text-emerald-600 dark:text-emerald-400 font-semibold text-[11px]">
                        <Wrench className="size-3" />
                        <span>{t.profiles.mcpActiveTools(activeTools.length)}</span>
                      </div>
                      <div className="mt-1 flex flex-wrap gap-1">
                        {activeTools.map((tool) => (
                          <span key={tool} className="rounded bg-emerald-500/10 px-1.5 py-0.5 font-mono text-[10px] text-emerald-700 dark:text-emerald-300">
                            {tool}
                          </span>
                        ))}
                      </div>
                    </div>
                  ) : null}
                </div>
              </div>
            )
          })}
        </div>
      )}

      {/* 常用预设快捷卡片 */}
      <div className="rounded-2xl border border-line bg-panel p-4 shadow-xs space-y-3">
        <div className="flex items-center gap-2 font-semibold text-xs text-ink">
          <Sparkles className="size-3.5 text-brand" />
          <span>{t.profiles.mcpPresetTitle}</span>
        </div>

        <div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2">
          {MCP_PRESETS.map((preset) => {
            const Icon = preset.icon
            return (
              <div
                key={preset.name}
                className="group flex flex-col justify-between rounded-xl border border-line bg-bg p-3 transition-colors hover:border-brand/40"
              >
                <div>
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2">
                      <div className="flex size-6 items-center justify-center rounded-lg bg-line text-ink">
                        <Icon className="size-3.5" />
                      </div>
                      <span className="text-xs font-semibold text-ink">
                        {preset.label}
                      </span>
                    </div>
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => applyPreset(preset)}
                      className="h-6 gap-1 px-2 text-[11px] hover:border-brand hover:text-brand"
                    >
                      <PlusCircle className="size-3" />
                      <span>应用预设</span>
                    </Button>
                  </div>
                  <p className="mt-1.5 text-[11px] text-faint leading-relaxed">
                    {preset.desc}
                  </p>
                </div>
                <div className="mt-2 truncate font-mono text-[10px] text-faint border-t border-line/50 pt-1.5">
                  <code>{preset.command} {preset.args.slice(0, 2).join(" ")}…</code>
                </div>
              </div>
            )
          })}
        </div>
      </div>

      {/* 新建/编辑 MCP 服务弹窗 */}
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="sm:max-w-lg">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-sm font-bold">
              <Boxes className="size-4 text-brand" />
              <span>{t.profiles.mcpModalTitle}</span>
            </DialogTitle>
            <DialogDescription className="text-xs text-faint">
              {t.profiles.mcpModalDesc}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-3.5 py-2 text-xs">
            <div>
              <label className="text-faint font-semibold text-[11px]">
                {t.profiles.mcpServerName} <span className="text-rose-500">*</span>
              </label>
              <input
                value={formName}
                onChange={(e) => setFormName(e.target.value)}
                placeholder="例如 github, filesystem, postgres"
                className="mt-1 w-full rounded-xl border border-line bg-bg px-3 py-1.5 font-mono text-xs text-ink outline-none focus:border-brand"
              />
            </div>

            <div className="grid grid-cols-3 gap-2">
              <div className="col-span-1">
                <label className="text-faint font-semibold text-[11px]">
                  {t.profiles.mcpCommand}
                </label>
                <input
                  value={formCommand}
                  onChange={(e) => setFormCommand(e.target.value)}
                  placeholder="npx / uvx"
                  className="mt-1 w-full rounded-xl border border-line bg-bg px-3 py-1.5 font-mono text-xs text-ink outline-none focus:border-brand"
                />
              </div>
              <div className="col-span-2">
                <label className="text-faint font-semibold text-[11px]">
                  {t.profiles.mcpArgs}
                </label>
                <input
                  value={formArgs}
                  onChange={(e) => setFormArgs(e.target.value)}
                  placeholder="-y @modelcontextprotocol/server-..."
                  className="mt-1 w-full rounded-xl border border-line bg-bg px-3 py-1.5 font-mono text-xs text-ink outline-none focus:border-brand"
                />
              </div>
            </div>

            {/* 环境变量列表 */}
            <div>
              <div className="flex items-center justify-between mb-1">
                <label className="text-faint font-semibold text-[11px]">
                  {t.profiles.mcpEnv}
                </label>
                <button
                  type="button"
                  onClick={() => setFormEnv([...formEnv, { key: "", value: "" }])}
                  className="text-[11px] text-brand hover:underline"
                >
                  + 添加变量
                </button>
              </div>
              {formEnv.length === 0 ? (
                <p className="text-[11px] text-faint italic">无需特殊环境变量</p>
              ) : (
                <div className="space-y-1.5 max-h-32 overflow-y-auto">
                  {formEnv.map((item, idx) => (
                    <div key={idx} className="flex items-center gap-1.5">
                      <input
                        value={item.key}
                        onChange={(e) => {
                          const next = [...formEnv]
                          next[idx].key = e.target.value
                          setFormEnv(next)
                        }}
                        placeholder="KEY"
                        className="w-1/3 rounded-lg border border-line bg-bg px-2 py-1 font-mono text-xs text-ink outline-none focus:border-brand"
                      />
                      <input
                        value={item.value}
                        onChange={(e) => {
                          const next = [...formEnv]
                          next[idx].value = e.target.value
                          setFormEnv(next)
                        }}
                        placeholder="VALUE"
                        className="flex-1 rounded-lg border border-line bg-bg px-2 py-1 font-mono text-xs text-ink outline-none focus:border-brand"
                      />
                      <button
                        type="button"
                        onClick={() => {
                          const next = formEnv.filter((_, i) => i !== idx)
                          setFormEnv(next)
                        }}
                        className="text-faint hover:text-rose-500 px-1"
                      >
                        ×
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>

            <div className="flex items-center justify-between pt-2 border-t border-line/60">
              <span className="text-xs text-dim">停用该 MCP 服务</span>
              <Switch checked={formDisabled} onCheckedChange={setFormDisabled} />
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setDialogOpen(false)}
              disabled={saving}
            >
              取消
            </Button>
            <Button
              onClick={handleSaveServer}
              disabled={saving || !formName.trim()}
              className="bg-brand text-white hover:bg-brand/90"
            >
              {saving && <LoaderCircle className="size-3.5 animate-spin mr-1.5" />}
              <span>{t.profiles.mcpSaveBtn}</span>
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
