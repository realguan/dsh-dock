// McpManager.tsx —— Profile 的 MCP 服务器可视化结构化管理工作台（4.7 完整版）。
import { useCallback, useEffect, useMemo, useState } from "react"
import {
  AlertTriangle,
  Boxes,
  Check,
  ChevronDown,
  Code2,
  Copy,
  Database,
  Edit2,
  Globe,
  HardDrive,
  LoaderCircle,
  Plus,
  PlusCircle,
  Server,
  Sparkles,
  Terminal,
  Trash2,
  Wrench,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { fmtClock } from "@/lib/format"
import { useCopy } from "@/hooks/useCopy"
import { useI18n } from "@/stores/i18nStore"
import type { McpNamed, McpProbe, McpServerConfig, PluginRuntimeSnapshot } from "@/types/ipc"
import { Button } from "@/components/ui/button"
import { ConfirmDialog } from "@/components/ui/confirm-dialog"
import { StateBlock } from "@/components/ui/state-block"
import { Switch } from "@/components/ui/switch"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

// 2026-09-18 收口：预设描述文案迁入 t.profiles.mcpPresetDescs（按 preset.name 索引），
// 这里只留结构化配置。
const MCP_PRESETS: Array<{
  name: string
  label: string
  icon: typeof Code2
  command: string
  args: string[]
  env: Record<string, string>
}> = [
  {
    name: "github",
    label: "GitHub Tools",
    icon: Code2,
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-github"],
    env: { GITHUB_PERSONAL_ACCESS_TOKEN: "" },
  },
  {
    name: "filesystem",
    label: "Filesystem Sandbox",
    icon: HardDrive,
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/workspace"],
    env: {},
  },
  {
    name: "postgres",
    label: "PostgreSQL Database",
    icon: Database,
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-postgres", "postgresql://user:pass@localhost/db"],
    env: {},
  },
  {
    name: "brave-search",
    label: "Brave Web Search",
    icon: Globe,
    command: "npx",
    args: ["-y", "@modelcontextprotocol/server-brave-search"],
    env: { BRAVE_API_KEY: "" },
  },
]

/** 一次探测的行内呈现形态（2026-09-15 R1b）：成功携能力清单，失败携错误原文。
 *  两态都**保留**——失败不是"没有结果"，它本身就是结果（且必须持续可见）。
 *  `at` 是快照时刻：一次探测**不反映后续变更**（ADR-0022 §5），UI 必须标出来。 */
type ProbeEntry =
  | { ok: true; probe: McpProbe; at: number }
  | { ok: false; error: string }

/** 能力清单的一节（Tools / Resources / Templates）。三节都用它，避免三份复制。 */
function ProbeList({ label, items }: { label: string; items: McpNamed[] }) {
  return (
    <div>
      <div className="flex items-center gap-1.5 text-label font-semibold text-dim">
        <span>{label}</span>
        <span className="rounded-md bg-line px-1 font-mono text-meta text-faint">{items.length}</span>
      </div>
      {items.length === 0 ? (
        // 空 ≠ 缺：真正的"没这个方法"由 notes 说明，这里只陈述"本节为空"。
        <p className="mt-0.5 pl-1 text-meta text-faint">—</p>
      ) : (
        <ul className="mt-0.5 space-y-0.5 pl-1">
          {items.map((it) => (
            <li key={`${it.name}:${it.detail}`} className="flex items-baseline gap-1.5">
              <span className="shrink-0 font-mono text-meta text-ink">{it.name}</span>
              {it.detail ? (
                <span className="truncate text-meta text-faint" title={it.detail}>
                  {it.detail}
                </span>
              ) : null}
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}

/** 探测结果行内卡（2026-09-15 R1b）。
 *  成功 → 可折叠：表头一行给三个计数（收起时也看得见），展开才是清单。
 *  失败 → **恒展开**：错误折叠起来等于把"没探测成"藏起来，用户会读成"没能力"。 */
function ProbeCard({
  serverName,
  entry,
  open,
  onToggle,
}: {
  serverName: string
  entry: ProbeEntry
  open: boolean
  onToggle: () => void
}) {
  const { t } = useI18n()

  if (!entry.ok) {
    return (
      <div className="rounded-lg border border-danger/30 bg-danger-soft p-2 text-xs">
        <div className="flex items-start gap-1.5 text-label font-semibold text-danger">
          <AlertTriangle className="mt-0.5 size-3 shrink-0" />
          <span>{t.profiles.mcpProbeFailed(serverName, entry.error)}</span>
        </div>
      </div>
    )
  }

  const { probe } = entry
  return (
    <div className="rounded-lg border border-line bg-bg text-xs">
      <button
        type="button"
        aria-expanded={open}
        aria-label={`${t.profiles.mcpProbeToggle}：${serverName}`}
        onClick={onToggle}
        className="flex w-full items-center justify-between gap-2 p-2 text-left"
      >
        <span className="flex min-w-0 items-center gap-1.5 text-dim">
          <Wrench className="size-3 shrink-0 text-brand-deep" />
          <span className="truncate">
            {t.profiles.mcpProbeResult(
              serverName,
              probe.tools.length,
              probe.resources.length,
              probe.templates.length,
            )}
          </span>
        </span>
        {/* 快照时间必显（ADR-0022 §5）：不标时间，旧快照会被当成"当前能力"。 */}
        <span className="shrink-0 font-mono text-meta text-faint">
          {t.profiles.mcpProbeAt(fmtClock(entry.at))}
        </span>
        <ChevronDown
          className={`size-3 shrink-0 text-faint transition-transform ${open ? "rotate-180" : ""}`}
        />
      </button>
      {probe.notes.length > 0 && (
        <p className="px-2 pb-2 text-meta text-warn">{probe.notes.join("；")}</p>
      )}
      {open && (
        <div className="space-y-2 border-t border-line p-2">
          <p className="font-mono text-meta text-faint">
            {t.profiles.mcpProbeProtocol} {probe.protocolVersion} · {probe.serverName}
          </p>
          <ProbeList label={t.profiles.mcpProbeTools} items={probe.tools} />
          <ProbeList label={t.profiles.mcpProbeResources} items={probe.resources} />
          <ProbeList label={t.profiles.mcpProbeTemplates} items={probe.templates} />
        </div>
      )}
    </div>
  )
}

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
  const { copiedKey: copiedName, copy } = useCopy()
  const [loadError, setLoadError] = useState<string | null>(null)

  // 新建/编辑 Dialog 状态
  const [dialogOpen, setDialogOpen] = useState(false)
  const [formName, setFormName] = useState("")
  const [formCommand, setFormCommand] = useState("npx")
  const [formArgs, setFormArgs] = useState("")
  const [formEnv, setFormEnv] = useState<Array<{ key: string; value: string }>>([])
  const [formDisabled, setFormDisabled] = useState(false)
  const [saving, setSaving] = useState(false)
  const [deletingName, setDeletingName] = useState<string | null>(null)
  // 删除确认（2026-09-08，U9）：统一走模态确认（原为 window.confirm）
  const [pendingDelete, setPendingDelete] = useState<string | null>(null)
  /** 正在探测的服务器名（按钮转圈用）。 */
  const [probingName, setProbingName] = useState<string | null>(null)
  /** 探测结果按服务器名缓存（2026-09-15 R1b）。 */
  const [probes, setProbes] = useState<Record<string, ProbeEntry>>({})
  /** 成功卡的折叠态；失败卡不可折叠（错误不该藏在一次点击后面）。 */
  const [probeOpen, setProbeOpen] = useState<Record<string, boolean>>({})

  /**
   * MCP 能力探测（2026-09-15，ADR-0022 **stdio 分支**）：后端主动握手后枚举
   * Tools / Resources / Resource Templates。
   *
   * **结果落成行内卡片**（2026-09-15 R1b，取代原先的"仅通知"呈现）：通知是瞬时
   * 的、一次只能看一条，而能力清单是要**对照着看**的东西（哪个 server 有哪些
   * tool）。失败同样落在行内且**恒展开**——三类错误（`streamable-http` 分支未实现、
   * WSL 客体档不支持、该 profile 未配置此服务器）各有明确文案，吞掉会让用户误以为
   * "这个服务器没能力"，而其实是"没探测成"，两者处置完全不同。
   */
  const handleProbe = async (serverName: string) => {
    setProbingName(serverName)
    try {
      const probe = await api.probeMcpServer(profileName, serverName)
      setProbes((prev) => ({ ...prev, [serverName]: { ok: true, probe, at: Date.now() } }))
      setProbeOpen((prev) => ({ ...prev, [serverName]: true }))
    } catch (e) {
      setProbes((prev) => ({ ...prev, [serverName]: { ok: false, error: String(e) } }))
    } finally {
      setProbingName(null)
    }
  }

  /** 丢弃某个 server 的探测结果（配置变了，旧结果就是错的——不能新配置配旧清单）。 */
  const forgetProbe = (serverName: string) => {
    setProbes((prev) => {
      if (!(serverName in prev)) return prev
      const next = { ...prev }
      delete next[serverName]
      return next
    })
    setProbeOpen((prev) => {
      if (!(serverName in prev)) return prev
      const next = { ...prev }
      delete next[serverName]
      return next
    })
  }

  // 换 profile = 换一整套 server：整批丢弃，避免把上一个 profile 的能力清单挂在
  // 同名 server 行上（同名不同配置是常态）。
  useEffect(() => {
    setProbes({})
    setProbeOpen({})
  }, [profileName])

  // 2026-09-08（问题记录095 #1）：主数据失败改面板内错误态（原 catch 走
  // onNotice toast——onNotice 是父组件内联箭头，引用不稳，effect 依赖
  // loadData 形成「失败→toast→重渲染→再加载」死循环）。运行态快照是辅助
  // 信息：失败静默降级为空（回环查询仅在 dsh 运行时可达，非配置错误）。
  const loadData = useCallback(async () => {
    setLoading(true)
    setLoadError(null)
    try {
      const list = await api.listMcpServers(profileName)
      setServers(list)
    } catch (e) {
      setServers(null)
      setLoadError(String(e))
      setLoading(false)
      return
    } finally {
      setLoading(false)
    }
    api
      .getPluginRuntime()
      .then(setRuntime)
      .catch(() => setRuntime(null))
  }, [profileName])

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
      onNotice?.(t.profiles.mcpNameRequired, "warn")
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
      // 配置变了，旧探测结果即失效——继续展示等于拿旧清单描述新配置。
      forgetProbe(srv.name)
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
    setDeletingName(srvName)
    try {
      await api.deleteMcpServer(profileName, srvName)
      forgetProbe(srvName)
      onNotice?.(t.profiles.mcpDeleteSuccess, "ok")
      await loadData()
    } catch (e) {
      onNotice?.(String(e), "warn")
    } finally {
      setDeletingName(null)
    }
  }

  const copyPrefix = async (serverName: string) => {
    const prefix = `mcp__${serverName}__*`
    // 2026-09-08：写失败不再静默（原来只挂 .then 成功分支）
    const outcome = await copy(prefix, serverName)
    if (outcome.ok) onNotice?.(t.profiles.mcpPrefixCopied(prefix), "ok")
    else onNotice?.(t.error.copyFailed, "warn")
  }

  return (
    <div className="space-y-4">
      {/* 标题与操作栏 */}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <div className="flex items-center gap-2">
            <Boxes className="size-4 text-brand-deep" />
            <h2 className="text-sm font-bold text-ink">
              {t.profiles.mcpTitle}
            </h2>
          </div>
          <p className="text-xs text-faint">{t.profiles.mcpSubtitle}</p>
        </div>

        <div className="flex items-center gap-2">
          <Button size="sm" onClick={openCreateDialog} className="gap-1.5">
            <Plus className="size-3.5" />
            <span>{t.profiles.mcpAddBtn}</span>
          </Button>
        </div>
      </div>

      {/* 已配置的 MCP 服务器列表（2026-09-18 收口：错误/加载/空态统一 StateBlock） */}
      {loadError ? (
        <StateBlock
          tone="error"
          title={t.profiles.mcpLoadFailed}
          hint={<span className="break-all">{loadError}</span>}
          action={
            <Button size="sm" variant="outline" onClick={() => void loadData()} className="gap-1">
              <LoaderCircle className={`size-3.5 ${loading ? "animate-spin text-brand-deep" : "text-dim"}`} />
              <span>{t.profiles.mcpRetry}</span>
            </Button>
          }
        />
      ) : loading && !servers ? (
        <StateBlock tone="loading" title={t.profiles.mcpLoading} />
      ) : !servers || servers.length === 0 ? (
        <StateBlock
          tone="empty"
          icon={Server}
          title={t.profiles.mcpEmpty}
          hint={t.profiles.mcpEmptyHint}
        />
      ) : (
        <div className="grid grid-cols-1 gap-3">
          {servers.map((s) => {
            const activeTools = activeToolsByServer.get(s.name) || []
            const isDeleting = deletingName === s.name
            const probeEntry = probes[s.name]

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
                        <span className="rounded-md bg-line-soft px-1.5 py-0.5 text-meta text-faint">
                          {t.profiles.pluginDisabled}
                        </span>
                      ) : (
                        <span className="rounded-md bg-ok-soft px-1.5 py-0.5 text-meta font-medium text-ok">
                          {t.profiles.mcpEnabledTag}
                        </span>
                      )}

                      <button
                        type="button"
                        onClick={() => copyPrefix(s.name)}
                        className="flex items-center gap-1 font-mono text-meta text-faint hover:text-ink rounded-md px-1.5 py-0.5 border border-line bg-bg transition-colors"
                        title={t.profiles.mcpCopyPrefix}
                      >
                        {copiedName === s.name ? (
                          <Check className="size-2.5 text-ok" />
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
                        className="gap-1"
                      >
                        <Edit2 className="size-3" />
                        <span>{t.profiles.mcpEditBtn}</span>
                      </Button>
                      {/* MCP 能力探测（2026-09-15，ADR-0022 stdio 分支）：
                          主动握手后枚举 Tools/Resources/Templates；失败原样透出。 */}
                      <Button
                        size="sm"
                        variant="outline"
                        title={t.profiles.mcpProbeBtn}
                        aria-label={`${t.profiles.mcpProbeBtn}：${s.name}`}
                        onClick={() => void handleProbe(s.name)}
                        disabled={probingName === s.name || isDeleting}
                        className="gap-1"
                      >
                        {probingName === s.name ? (
                          <LoaderCircle className="size-3 animate-spin" />
                        ) : (
                          <Wrench className="size-3 text-faint" />
                        )}
                        <span>{t.profiles.mcpProbeBtn}</span>
                      </Button>
                      <Button
                        size="icon-sm"
                        variant="destructive"
                        aria-label={`${t.profiles.mcpDeleteBtn}：${s.name}`}
                        onClick={() => setPendingDelete(s.name)}
                        disabled={isDeleting}
                      >
                        {isDeleting ? (
                          <LoaderCircle className="size-3 animate-spin text-danger" />
                        ) : (
                          <Trash2 className="size-3 text-faint" />
                        )}
                      </Button>
                    </div>
                  </div>

                  {/* 命令与参数 */}
                  <div className="flex items-center gap-2 font-mono text-xs text-dim bg-bg rounded-lg border border-line p-2">
                    <Terminal className="size-3.5 text-faint shrink-0" />
                    <span className="truncate" title={`${s.command} ${s.args.join(" ")}`}>
                      {s.command} {s.args.join(" ")}
                    </span>
                  </div>

                  {/* 环境变量标签 */}
                  {Object.keys(s.env).length > 0 && (
                    <div className="flex flex-wrap items-center gap-1.5 pt-1 text-label text-faint">
                      <span className="font-semibold text-ink">ENV:</span>
                      {Object.keys(s.env).map((k) => (
                        <span key={k} className="rounded-md bg-line px-1.5 py-0.5 font-mono text-meta">
                          {k}=••••
                        </span>
                      ))}
                    </div>
                  )}

                  {/* 运行态工具联动展示 */}
                  {activeTools.length > 0 ? (
                    <div className="rounded-lg border border-line bg-ok-soft p-2 text-xs">
                      <div className="flex items-center gap-1.5 text-ok font-semibold text-label">
                        <Wrench className="size-3" />
                        <span>{t.profiles.mcpActiveTools(activeTools.length)}</span>
                      </div>
                      <div className="mt-1 flex flex-wrap gap-1">
                        {activeTools.map((tool) => (
                          <span key={tool} className="rounded-md bg-ok-soft px-1.5 py-0.5 font-mono text-meta text-ok">
                            {tool}
                          </span>
                        ))}
                      </div>
                    </div>
                  ) : null}

                  {/* 探测结果行内卡（2026-09-15 R1b）：成功可折叠、失败恒展开。 */}
                  {probeEntry ? (
                    <ProbeCard
                      serverName={s.name}
                      entry={probeEntry}
                      open={probeOpen[s.name] ?? false}
                      onToggle={() =>
                        setProbeOpen((prev) => ({ ...prev, [s.name]: !(prev[s.name] ?? false) }))
                      }
                    />
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
          <Sparkles className="size-3.5 text-brand-deep" />
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
                      size="xs"
                      variant="outline"
                      onClick={() => applyPreset(preset)}
                      className="gap-1"
                    >
                      <PlusCircle className="size-3" />
                      <span>{t.profiles.mcpApplyPreset}</span>
                    </Button>
                  </div>
                  <p className="mt-1.5 text-label text-faint leading-relaxed">
                    {t.profiles.mcpPresetDescs[preset.name as keyof typeof t.profiles.mcpPresetDescs]}
                  </p>
                </div>
                <div
                  className="mt-2 truncate font-mono text-meta text-faint border-t border-line/50 pt-1.5"
                  title={`${preset.command} ${preset.args.join(" ")}`}
                >
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
              <Boxes className="size-4 text-brand-deep" />
              <span>{t.profiles.mcpModalTitle}</span>
            </DialogTitle>
            <DialogDescription className="text-xs text-faint">
              {t.profiles.mcpModalDesc}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-3.5 py-2 text-xs">
            <div>
              <label htmlFor="mcp-form-name" className="text-faint font-semibold text-label">
                {t.profiles.mcpServerName} <span className="text-danger">*</span>
              </label>
              <input
                id="mcp-form-name"
                value={formName}
                onChange={(e) => setFormName(e.target.value)}
                placeholder={t.profiles.mcpNamePlaceholder}
                className="mt-1 w-full rounded-xl border border-line bg-bg px-3 py-1.5 font-mono text-xs text-ink outline-none focus:border-brand"
              />
            </div>

            <div className="grid grid-cols-3 gap-2">
              <div className="col-span-1">
                <label htmlFor="mcp-form-command" className="text-faint font-semibold text-label">
                  {t.profiles.mcpCommand}
                </label>
                <input
                  id="mcp-form-command"
                  value={formCommand}
                  onChange={(e) => setFormCommand(e.target.value)}
                  placeholder="npx / uvx"
                  className="mt-1 w-full rounded-xl border border-line bg-bg px-3 py-1.5 font-mono text-xs text-ink outline-none focus:border-brand"
                />
              </div>
              <div className="col-span-2">
                <label htmlFor="mcp-form-args" className="text-faint font-semibold text-label">
                  {t.profiles.mcpArgs}
                </label>
                <input
                  id="mcp-form-args"
                  value={formArgs}
                  onChange={(e) => setFormArgs(e.target.value)}
                  placeholder="-y @modelcontextprotocol/server-..."
                  className="mt-1 w-full rounded-xl border border-line bg-bg px-3 py-1.5 font-mono text-xs text-ink outline-none focus:border-brand"
                />
              </div>
            </div>

            {/* 环境变量列表 */}
            <div role="group" aria-label={t.profiles.mcpEnv}>
              <div className="flex items-center justify-between mb-1">
                <span className="text-faint font-semibold text-label">
                  {t.profiles.mcpEnv}
                </span>
                <button
                  type="button"
                  onClick={() => setFormEnv([...formEnv, { key: "", value: "" }])}
                  className="text-label text-brand-deep hover:underline"
                >
                  {t.profiles.mcpAddEnv}
                </button>
              </div>
              {formEnv.length === 0 ? (
                <p className="text-label text-faint italic">{t.profiles.mcpNoEnv}</p>
              ) : (
                <div className="space-y-1.5 max-h-32 overflow-y-auto">
                  {formEnv.map((item, idx) => (
                    <div key={idx} className="flex items-center gap-1.5">
                      <input
                        value={item.key}
                        aria-label={`${t.profiles.mcpEnv} ${idx + 1} KEY`}
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
                        aria-label={`${t.profiles.mcpEnv} ${idx + 1} VALUE`}
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
                        aria-label={t.profiles.mcpEnvRemove}
                        onClick={() => {
                          const next = formEnv.filter((_, i) => i !== idx)
                          setFormEnv(next)
                        }}
                        className="text-faint hover:text-danger px-1"
                      >
                        ×
                      </button>
                    </div>
                  ))}
                </div>
              )}
            </div>

            <div className="flex items-center justify-between pt-2 border-t border-line/60">
              <span className="text-xs text-dim">{t.profiles.mcpDisable}</span>
              <Switch
                aria-label={t.profiles.mcpDisable}
                checked={formDisabled}
                onCheckedChange={setFormDisabled}
              />
            </div>
          </div>

          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setDialogOpen(false)}
              disabled={saving}
            >
              {t.confirm.cancel}
            </Button>
            <Button onClick={handleSaveServer} disabled={saving || !formName.trim()}>
              {saving && <LoaderCircle className="size-3.5 animate-spin mr-1.5" />}
              <span>{t.profiles.mcpSaveBtn}</span>
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* 删除确认（U9）：从 cordis.patch.yml 安全删除，先确认 */}
      <ConfirmDialog
        open={pendingDelete !== null}
        title={pendingDelete ? t.profiles.mcpDeleteConfirm(pendingDelete) : ""}
        note={t.profiles.mcpDeleteNote}
        confirmLabel={t.profiles.mcpDeleteBtn}
        cancelLabel={t.confirm.cancel}
        onConfirm={() => {
          const target = pendingDelete
          setPendingDelete(null)
          if (target) void handleDeleteServer(target)
        }}
        onClose={() => setPendingDelete(null)}
      />
    </div>
  )
}
