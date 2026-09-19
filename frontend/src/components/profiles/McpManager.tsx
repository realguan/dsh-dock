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
  Eye,
  EyeOff,
  Globe,
  HardDrive,
  Layers,
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
import { SERVER_NAME_RE, dupInfo, joinArgs, splitArgs } from "@/lib/mcpForm"
import { useCopy } from "@/hooks/useCopy"
import { useI18n } from "@/stores/i18nStore"
import type {
  McpNamed,
  McpProbe,
  McpScope,
  McpServerConfig,
  McpTransport,
  PluginRuntimeSnapshot,
} from "@/types/ipc"
import { Button } from "@/components/ui/button"
import { ConfirmDialog } from "@/components/ui/confirm-dialog"
import { Tip } from "@/components/ui/info-tip"
import { Segmented } from "@/components/ui/segmented"
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

/** 上游 MCP 客户端包名 —— 运行态插件行按它筛（`moduleName` 就是包名）。 */
const MCP_CLIENT_PKG = "@deepseek-ai/dsh-mcp-client"

// 2026-09-18 收口：预设描述文案迁入 t.profiles.mcpPresetDescs（按 preset.name 索引），
// 这里只留结构化配置。
//
// **命令一律 `pnpm dlx`，不是 `npx`**（2026-09-18 二修）：dsh 子进程的 PATH 来自
// 壳的 engines bin 目录（`resolve::dsh_child_path`），那里随壳内置的是 **pnpm**，
// **没有 npx**（npx 只在 node 官方发行版的 bin 里，而引擎引导只装 node+pnpm+dsh）。
// 预设为 `npx` 的条目在界面上"看着对、探测必 ENOENT"，是最高频的假配置来源。
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
    command: "pnpm",
    args: ["dlx", "@modelcontextprotocol/server-github"],
    env: { GITHUB_PERSONAL_ACCESS_TOKEN: "" },
  },
  {
    name: "filesystem",
    label: "Filesystem Sandbox",
    icon: HardDrive,
    command: "pnpm",
    args: ["dlx", "@modelcontextprotocol/server-filesystem", "/path/to/workspace"],
    env: {},
  },
  {
    name: "postgres",
    label: "PostgreSQL Database",
    icon: Database,
    command: "pnpm",
    args: ["dlx", "@modelcontextprotocol/server-postgres", "postgresql://user:pass@localhost/db"],
    env: {},
  },
  {
    name: "brave-search",
    label: "Brave Web Search",
    icon: Globe,
    command: "pnpm",
    args: ["dlx", "@modelcontextprotocol/server-brave-search"],
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

/** 行身份键 = `(scope, name, 层内序号)`（探测缓存 / 折叠态 / 待删·待存态都用它）。
 *
 *  为什么三段缺一不可：
 *  - `scope`（2026-09-18）：同一 serverName 可同时存在于 profile 层与全局层（两条独立
 *    的生效范围），只按名字缓存会把一层的探测结果挂到另一层那一行。
 *  - 层内序号（2026-09-18 三修）：**同层**也可能有重复 serverName（上游两条都插入，
 *    后加载的那条实例化失败）。少了它，点第二条的开关会让第一条一起转圈、探测结果
 *    两行共用——而这两行的 command 往往正是差异所在。 */
function rowKey(scope: McpScope, name: string, idx: number): string {
  return `${scope}:${name}:${idx}`
}

/** 生效范围徽标（颜色按"影响面"分级：全局 > 单 profile）。 */
// 2026-09-19：解释从原生 `title` 换成 Tip——同一屏里悬浮只有一种机制（原生 title
// 有约 1s 延迟、不可样式化、键盘到不了），两套并存就是"有的悬浮快有的慢"。
function ScopeBadge({ scope }: { scope: McpScope }) {
  const { t } = useI18n()
  const label = scope === "global" ? t.profiles.mcpScopeGlobal : t.profiles.mcpScopeProfile
  const hint = scope === "global" ? t.profiles.mcpScopeGlobalHint : t.profiles.mcpScopeProfileHint
  return (
    <span className="flex items-center gap-1">
      <span
        className={
          scope === "global"
            ? "rounded-md bg-wash px-1.5 py-0.5 text-meta font-medium text-brand-deep"
            : "rounded-md bg-line-soft px-1.5 py-0.5 text-meta text-dim"
        }
      >
        {label}
      </span>
      <Tip text={hint} label={t.tip.ariaFor(label)} />
    </span>
  )
}

/** 一个键值对（`env` 与 `headers` 的形状完全相同）。 */
type KvPair = { key: string; value: string }

/** 键 → 值 列表编辑器（2026-09-18）：MCP 的 env 与 streamable-http 的 headers
 *  共用一份。两处都是"字符串→字符串、经常装 token"，分成两份复制粘贴的输入框
 *  只会漂移（改了一处忘了另一处）。 */
function KvEditor({
  label,
  addLabel,
  removeLabel,
  emptyText,
  keyPlaceholder,
  valuePlaceholder,
  pairs,
  onChange,
}: {
  label: string
  addLabel: string
  removeLabel: string
  emptyText: string
  keyPlaceholder: string
  valuePlaceholder: string
  pairs: KvPair[]
  onChange: (next: KvPair[]) => void
}) {
  const setAt = (idx: number, patch: Partial<KvPair>) => {
    const next = pairs.map((p, i) => (i === idx ? { ...p, ...patch } : p))
    onChange(next)
  }
  return (
    <div role="group" aria-label={label}>
      <div className="mb-1 flex items-center justify-between">
        <span className="text-label font-semibold text-faint">{label}</span>
        <button
          type="button"
          onClick={() => onChange([...pairs, { key: "", value: "" }])}
          className="text-label text-brand-deep hover:underline"
        >
          {addLabel}
        </button>
      </div>
      {pairs.length === 0 ? (
        <p className="text-label text-faint italic">{emptyText}</p>
      ) : (
        <div className="max-h-40 space-y-1.5 overflow-y-auto">
          {pairs.map((item, idx) => (
            <div key={idx} className="flex items-center gap-1.5">
              <input
                value={item.key}
                aria-label={`${label} ${idx + 1} KEY`}
                onChange={(e) => setAt(idx, { key: e.target.value })}
                placeholder={keyPlaceholder}
                className="w-1/3 rounded-lg border border-line bg-bg px-2 py-1 font-mono text-xs text-ink outline-none focus:border-brand"
              />
              <input
                value={item.value}
                aria-label={`${label} ${idx + 1} VALUE`}
                onChange={(e) => setAt(idx, { value: e.target.value })}
                placeholder={valuePlaceholder}
                className="flex-1 rounded-lg border border-line bg-bg px-2 py-1 font-mono text-xs text-ink outline-none focus:border-brand"
              />
              <button
                type="button"
                aria-label={removeLabel}
                onClick={() => onChange(pairs.filter((_, i) => i !== idx))}
                className="px-1 text-faint hover:text-danger"
              >
                ×
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

/** 列表里的敏感值（2026-09-18）：默认打码、点一下显形。
 *
 *  为什么默认打码：这些值通常是 PAT / API key，而这个面板正是"截图求助 / 投屏讲
 *  配置"的场合。为什么仍然可以显形：这是用户自己的本地配置文件，遮到看不见等于
 *  逼他去开编辑器——打码的目的是防外泄，不是防他自己。 */
function SecretChip({ name, value }: { name: string; value: string }) {
  const { t } = useI18n()
  const [show, setShow] = useState(false)
  return (
    <button
      type="button"
      onClick={() => setShow((v) => !v)}
      aria-pressed={show}
      title={show ? t.profiles.mcpSecretHide : t.profiles.mcpSecretReveal}
      className="flex max-w-full items-center gap-1 rounded-md bg-line px-1.5 py-0.5 font-mono text-meta transition-colors hover:bg-line-soft"
    >
      <span className="text-ink">{name}</span>
      <span className="text-faint">=</span>
      <span className="truncate text-dim">
        {show ? value || "∅" : "••••••"}
      </span>
      {show ? (
        <EyeOff className="size-2.5 shrink-0 text-faint" />
      ) : (
        <Eye className="size-2.5 shrink-0 text-faint" />
      )}
    </button>
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
  const [formCommand, setFormCommand] = useState("pnpm")
  const [formArgs, setFormArgs] = useState("")
  const [formEnv, setFormEnv] = useState<KvPair[]>([])
  const [formDisabled, setFormDisabled] = useState(false)
  /** 生效范围（2026-09-18）：写 profile 层还是 home 级全局层。 */
  const [formScope, setFormScope] = useState<McpScope>("profile")
  /** stdio 子进程工作目录（可空）。 */
  const [formCwd, setFormCwd] = useState("")
  /** 传输方式（2026-09-18 补表单）：后端保存时**按它清掉另一分支的键**。
   *  此前表单没有这一项、恒发 stdio ⇒ 编辑一条 `streamable-http` 条目会把它的
   *  `url`/`headers` 删掉、改写成一条 `command: npx` 的 stdio 行（静默毁配置）。 */
  const [formTransport, setFormTransport] = useState<McpTransport>("stdio")
  /** `streamable-http` 端点 URL。 */
  const [formUrl, setFormUrl] = useState("")
  /** `streamable-http` 附加请求头（常装 Authorization）。 */
  const [formHeaders, setFormHeaders] = useState<KvPair[]>([])
  /** 编辑既有条目时它的**原身份**（名字 + 层）。改名或改层都要把原来那条清掉，
   *  否则界面上"改一下作用范围"会留下一条跨层同名的重复（那正是最坏的那种状态：
   *  两条都加载、后加载的实例化失败）。新建时 = null。 */
  const [formOriginal, setFormOriginal] = useState<{
    name: string
    scope: McpScope
    rowId: string
  } | null>(null)
  /** 编辑的那一行原文含 `!!js` 表达式 ⇒ 保存只允许改启停（后端会拒，界面先说清）。 */
  const [formExpr, setFormExpr] = useState(false)
  const [saving, setSaving] = useState(false)
  /** 待删除项：**完整行身份**（层 + 名字 + 行 id + 层内序号）。同名可能同时存在于
   *  两层、甚至同层两条，只记名字会让确认框说"删 fs"而实际删掉另一条。 */
  const [pendingDelete, setPendingDelete] = useState<{
    name: string
    scope: McpScope
    rowId?: string
    idx: number
    dupSameLayer: boolean
  } | null>(null)
  const [deletingKey, setDeletingKey] = useState<string | null>(null)
  /** 正在行内启停的服务器（行身份键）。 */
  const [togglingKey, setTogglingKey] = useState<string | null>(null)
  /** 正在探测的服务器（行身份键）。 */
  const [probingKey, setProbingKey] = useState<string | null>(null)
  /** 探测结果按**行身份键**缓存（2026-09-15 R1b；2026-09-18 加 scope，三修加层内序号）。 */
  const [probes, setProbes] = useState<Record<string, ProbeEntry>>({})
  /** 成功卡的折叠态；失败卡不可折叠（错误不该藏在一次点击后面）。 */
  const [probeOpen, setProbeOpen] = useState<Record<string, boolean>>({})

  /**
   * MCP 能力探测（2026-09-15，ADR-0022）：后端主动握手后枚举
   * Tools / Resources / Resource Templates；stdio 与 streamable-http 两条分支都探。
   *
   * **结果落成行内卡片**（2026-09-15 R1b，取代原先的"仅通知"呈现）：通知是瞬时
   * 的、一次只能看一条，而能力清单是要**对照着看**的东西（哪个 server 有哪些
   * tool）。失败同样落在行内且**恒展开**——WSL 客体档不支持、`!!js` 表达式行不探、
   * 命令不可达各有明确文案，吞掉会让用户误以为"这个服务器没能力"，而其实是
   * "没探测成"，两者处置完全不同。
   */
  const handleProbe = async (
    serverName: string,
    scope: McpScope,
    idx: number,
    rowId?: string,
  ) => {
    const key = rowKey(scope, serverName, idx)
    setProbingKey(key)
    try {
      const probe = await api.probeMcpServer(profileName, serverName, scope, rowId)
      setProbes((prev) => ({ ...prev, [key]: { ok: true, probe, at: Date.now() } }))
      setProbeOpen((prev) => ({ ...prev, [key]: true }))
    } catch (e) {
      setProbes((prev) => ({ ...prev, [key]: { ok: false, error: String(e) } }))
    } finally {
      setProbingKey(null)
    }
  }

  /** 丢弃某个 serverName 的探测结果与折叠态（配置变了，旧结果就是错的——不能新配置
   *  配旧清单）。按**整片名字**丢弃而不是单行：同层同名有两行时，宁可多丢一次缓存
   *  （代价 = 再点一次探测），也不能让第一条的旧清单继续挂在改过的第二条上。 */
  const forgetProbe = (scope: McpScope, serverName: string) => {
    const prefix = `${scope}:${serverName}:`
    const drop = <T,>(prev: Record<string, T>): Record<string, T> => {
      if (!Object.keys(prev).some((k) => k.startsWith(prefix))) return prev
      const next: Record<string, T> = {}
      for (const [k, v] of Object.entries(prev)) {
        if (!k.startsWith(prefix)) next[k] = v
      }
      return next
    }
    setProbes(drop)
    setProbeOpen(drop)
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

  /** 运行态里"这一条 MCP 到底装上没有"。
   *
   *  2026-09-18 修：原实现扫 `module_name`/`entry_id` 是否以 `mcp__` 开头，
   *  但插件清单给的是**插件行**（`moduleName = @deepseek-ai/dsh-mcp-client`、
   *  `entryId = include:mcp-<名>`），从不以 `mcp__` 开头 ⇒ 该展示**恒为空**。
   *  真正该看的是**行的装配状态**：`enabled`（配置级生效值）+ `fiberPhase`（有无存活实例）。
   *
   *  2026-09-18 二修：**按行 id 精确匹配**。只按"剥掉 `mcp-` 前缀的名字"匹配，
   *  用户手写行 id 的行（本轮刻意保留不改写）会**静默没有徽标**，而两层同名时
   *  剥出来的名字又是同一个 ⇒ 一层的装配状态会挂到另一层那一行。 */
  const runtimeByRow = useMemo(() => {
    const byId = new Map<string, { enabled: boolean; fiberPhase: string | null }>()
    const byName = new Map<string, { enabled: boolean; fiberPhase: string | null }>()
    if (!runtime || !runtime.profile || runtime.profile !== profileName) {
      return { byId, byName }
    }
    for (const entry of runtime.entries) {
      if (entry.module_name !== MCP_CLIENT_PKG) continue
      // entryId 是 loader 树路径（`include:<行id>`，可能带组前缀）⇒ 取最后一段。
      const tail = (entry.entry_id || "").split(":").pop() ?? ""
      if (!tail) continue
      const info = { enabled: entry.enabled, fiberPhase: entry.fiber_phase }
      byId.set(tail, info)
      // 行 id 的约定是 `mcp-<serverName>`：这一路兜底给"后端没带回 rowId"的形状。
      const name = tail.startsWith("mcp-") ? tail.slice(4) : tail
      if (name && !byName.has(name)) byName.set(name, info)
    }
    return { byId, byName }
  }, [runtime, profileName])

  /** 列表按**生效范围分区**（2026-09-18）：混排时"这条到底写在哪个文件"要靠逐行
   *  读徽标才知道，而两层的行为差别（影响几个 profile）是这一屏最该先说清的事。
   *  只有一层有条目时不画分区头（一层标题 = 噪音）。 */
  const groups = useMemo(() => {
    const tagged = (servers ?? []).map((s, idx) => ({
      s,
      idx,
      scope: (s.scope ?? "profile") as McpScope,
    }))
    const out: Array<{ scope: McpScope; rows: typeof tagged }> = []
    for (const sc of ["profile", "global"] as const) {
      const rows = tagged.filter((r) => r.scope === sc)
      if (rows.length > 0) out.push({ scope: sc, rows })
    }
    return out
  }, [servers])

  /** 服务标识的即时校验（约束与后端 `validate_server_name` 同源）。 */
  const nameOk = SERVER_NAME_RE.test(formName.trim())

  const resetForm = () => {
    setFormName("")
    setFormCommand("pnpm")
    setFormArgs("")
    setFormEnv([])
    setFormDisabled(false)
    setFormScope("profile")
    setFormTransport("stdio")
    setFormUrl("")
    setFormHeaders([])
    setFormCwd("")
    setFormExpr(false)
    setFormOriginal(null)
  }

  const openCreateDialog = () => {
    resetForm()
    setDialogOpen(true)
  }

  const openEditDialog = (srv: McpServerConfig) => {
    setFormName(srv.name)
    setFormCommand(srv.command)
    setFormArgs(joinArgs(srv.args))
    setFormEnv(Object.entries(srv.env).map(([key, value]) => ({ key, value })))
    setFormDisabled(srv.disabled)
    // 层的归属必须带进表单，否则"编辑全局条目"会被当成 profile 层，凭空多出一条。
    setFormScope(srv.scope ?? "profile")
    // 传输方式 / url / headers **必须原样带回**：后端按传输方式清掉另一分支的键。
    setFormTransport(srv.transport ?? "stdio")
    setFormUrl(srv.url ?? "")
    setFormHeaders(Object.entries(srv.headers ?? {}).map(([key, value]) => ({ key, value })))
    setFormCwd(srv.cwd ?? "")
    setFormExpr(srv.expr ?? false)
    setFormOriginal({
      name: srv.name,
      scope: srv.scope ?? "profile",
      rowId: srv.rowId ?? "",
    })
    setDialogOpen(true)
  }

  const applyPreset = (preset: typeof MCP_PRESETS[0]) => {
    resetForm()
    setFormName(preset.name)
    setFormCommand(preset.command)
    setFormArgs(joinArgs(preset.args))
    setFormEnv(Object.entries(preset.env).map(([key, value]) => ({ key, value })))
    setDialogOpen(true)
  }

  const handleSaveServer = async () => {
    const name = formName.trim()
    if (!name) {
      onNotice?.(t.profiles.mcpNameRequired, "warn")
      return
    }
    if (!SERVER_NAME_RE.test(name)) {
      // 不合规的行**照样能写进 patch**，但插件在加载期被 schema 拒掉 ⇒
      // 界面"已启用"、模型侧永远拿不到工具。所以在这里就拦住，不等到保存后。
      onNotice?.(t.profiles.mcpNameInvalid(name), "warn")
      return
    }
    if (formTransport === "streamable-http" && !formUrl.trim()) {
      onNotice?.(t.profiles.mcpUrlRequired, "warn")
      return
    }
    // **不许借"新增/改名"踩到同层的另一条同名行**（2026-09-18 三修）：后端按名字
    // upsert，那样的保存等于**改写**另一条——用户以为自己在新增。编辑自己那一行
    // （行身份相同）不算占用。
    const origRowId = formOriginal?.rowId
    const taken = (servers ?? []).some((s) => {
      if (s.name !== name || (s.scope ?? "profile") !== formScope) return false
      // 新建（origRowId 缺省）⇒ 本层任何同名行都算占用；编辑⇒ 只有"不是自己那一行"
      // 才算（手写无 id 的行 rowId 为空串，按身份比而不是按名字比才不会误判）。
      return origRowId === undefined || (s.rowId ?? "") !== origRowId
    })
    if (taken) {
      onNotice?.(t.profiles.mcpNameTaken(name), "warn")
      return
    }
    setSaving(true)
    try {
      const toRecord = (pairs: KvPair[]) => {
        const out: Record<string, string> = {}
        for (const item of pairs) {
          if (item.key.trim()) out[item.key.trim()] = item.value.trim()
        }
        return out
      }
      const isHttp = formTransport === "streamable-http"

      const srv: McpServerConfig = {
        name,
        command: isHttp ? "" : formCommand.trim() || "pnpm",
        args: isHttp ? [] : splitArgs(formArgs),
        env: isHttp ? {} : toRecord(formEnv),
        disabled: formDisabled,
        transport: formTransport,
        url: isHttp ? formUrl.trim() : "",
        headers: isHttp ? toRecord(formHeaders) : {},
        scope: formScope,
        cwd: isHttp ? "" : formCwd.trim(),
        // 行身份：编辑哪一行就写回哪一行（同层重复 serverName 时后端靠它选目标）；
        // 新增时为空 ⇒ 后端按名字回退（并追加新行）。
        rowId: formOriginal?.rowId ?? "",
      }

      await api.saveMcpServer(profileName, srv)
      // 配置变了，旧探测结果即失效——继续展示等于拿旧清单描述新配置。
      forgetProbe(formScope, srv.name)
      // 改名或改层 = 换了一条身份：**先落新的，再清旧的**（顺序不可反——
      // 先删后存时保存失败就把条目弄丢了）。失败只 warn：此时两条并存，
      // 列表会立刻把它标成跨层同名并让用户处置，不静默。
      const orig = formOriginal
      if (orig && (orig.scope !== formScope || orig.name !== srv.name)) {
        try {
          await api.deleteMcpServer(profileName, orig.name, orig.scope, orig.rowId)
          forgetProbe(orig.scope, orig.name)
        } catch (e) {
          onNotice?.(String(e), "warn")
        }
      }
      onNotice?.(t.profiles.mcpSaveSuccess(srv.name), "ok")
      setDialogOpen(false)
      await loadData()
    } catch (e) {
      onNotice?.(String(e), "warn")
    } finally {
      setSaving(false)
    }
  }

  const handleDeleteServer = async (
    srvName: string,
    scope: McpScope,
    idx: number,
    rowId?: string,
  ) => {
    setDeletingKey(rowKey(scope, srvName, idx))
    try {
      // 必须带层 + 带行身份：同名条目两层都有时只删用户确认的那一条，同层重复时
      // 也只用行 id 删他点的那一条（后端只按名字删会删掉**另一条**）。
      await api.deleteMcpServer(profileName, srvName, scope, rowId)
      forgetProbe(scope, srvName)
      onNotice?.(t.profiles.mcpDeleteSuccess, "ok")
      await loadData()
    } catch (e) {
      onNotice?.(String(e), "warn")
    } finally {
      setDeletingKey(null)
    }
  }

  /** 行内启停（2026-09-18）：把"停一下这个 MCP"从 4 次点击（编辑→翻开关→保存→关窗）
   *  压到 1 次。
   *
   *  **整行原样带回、只改 `disabled`**——这不是偷懒，而是后端那条规则的触发条件：
   *  含 `!!js` 表达式的行只有"建模字段全等、只动 disabled"才允许通过（走文本级
   *  改写保住表达式）。若这里只发 `{name, disabled}`，那条行会被判成"改了配置"
   *  并拒绝——用户看到的就是一次莫名其妙的失败。 */
  const handleToggleDisabled = async (srv: McpServerConfig, idx: number) => {
    const scope = srv.scope ?? "profile"
    setTogglingKey(rowKey(scope, srv.name, idx))
    try {
      await api.saveMcpServer(profileName, { ...srv, scope, disabled: !srv.disabled })
      forgetProbe(scope, srv.name)
      await loadData()
    } catch (e) {
      onNotice?.(String(e), "warn")
    } finally {
      setTogglingKey(null)
    }
  }

  /** 复制工具前缀。复制态按**行键**记（不是 `scope:name`）——同层重名时两行的
   *  前缀文案相同，按名字记会让两处勾同时亮。 */
  const copyPrefix = async (serverName: string, rowKey: string) => {
    const prefix = `mcp__${serverName}__*`
    // 2026-09-08：写失败不再静默（原来只挂 .then 成功分支）
    const outcome = await copy(prefix, rowKey)
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
            <h2 className="text-sm font-semibold text-ink">
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
        <div className="space-y-5">
          {groups.map((g) => (
            <section key={g.scope} className="space-y-3">
              {/* 分区头：只在两层都有条目时画（一层时它是噪音）。带上文件名——
                  "这条写在哪个文件里"是排查时第一个要回答的问题。 */}
              {groups.length > 1 ? (
                <div className="flex flex-wrap items-center gap-2">
                  <ScopeBadge scope={g.scope} />
                  <span className="font-mono text-meta text-faint">
                    {g.scope === "global"
                      ? "$DSH_HOME/cordis.patch.yml"
                      : `profiles/${profileName}/cordis.patch.yml`}
                  </span>
                  <span className="text-meta text-faint">· {g.rows.length}</span>
                </div>
              ) : null}

              <div className="grid grid-cols-1 gap-3">
                {g.rows.map(({ s, idx }) => {
                  // 行身份 = (scope, name, 层内序号)：启停/探测/删除/折叠态都要逐行独立。
                  const scope = g.scope
                  const key = rowKey(scope, s.name, idx)
                  const isHttp = (s.transport ?? "stdio") === "streamable-http"
                  const rt =
                    (s.rowId ? runtimeByRow.byId.get(s.rowId) : undefined) ??
                    runtimeByRow.byName.get(s.name)
                  const isDeleting = deletingKey === key
                  const isToggling = togglingKey === key
                  const probeEntry = probes[key]
                  const dup = dupInfo(servers ?? [], s.name, scope, idx)
                  const conflicted = dup.total > 1
                  const headers = s.headers ?? {}
                  /* 装配状态成对派生（2026-09-19）：`tag` 是表面的短结论，`why` 是
                     悬浮里的完整判定。配置存在 ≠ 已加载——`enabled` 是配置级生效值，
                     `fiberPhase` 才说明有没有存活实例。 */
                  const runtime = !rt
                    ? null
                    : !rt.enabled
                      ? { tag: t.profiles.mcpRuntimeDisabledTag, why: t.profiles.mcpRuntimeDisabled }
                      : rt.fiberPhase === "active"
                        ? { tag: t.profiles.mcpRuntimeActiveTag, why: t.profiles.mcpRuntimeActive }
                        : rt.fiberPhase === "failed"
                          ? { tag: t.profiles.mcpRuntimeFailedTag, why: t.profiles.mcpRuntimeFailed }
                          : {
                              tag: t.profiles.mcpRuntimeNotLoadedTag,
                              why: t.profiles.mcpRuntimeNotLoaded(rt.fiberPhase),
                            }
                  const runtimeOk = rt?.enabled === true && rt?.fiberPhase === "active"

                  return (
                    <div
                      key={key}
                      className={`flex flex-col justify-between gap-3 rounded-2xl border bg-panel p-4 shadow-xs transition-colors ${
                        conflicted ? "border-danger/40" : "border-line hover:border-brand/40"
                      }`}
                    >
                      <div className="space-y-2">
                        <div className="flex flex-wrap items-center justify-between gap-2">
                          <div className="flex min-w-0 flex-wrap items-center gap-2">
                            <span
                              className={`font-mono text-xs font-semibold ${
                                s.disabled ? "text-dim" : "text-ink"
                              }`}
                            >
                              {s.name}
                            </span>
                            {/* 行内启停（2026-09-18）：状态与控制合成一处，
                                不再"看徽标 + 进弹窗"两段式。 */}
                            <span className="flex items-center gap-1.5 rounded-lg border border-line bg-bg px-1.5 py-0.5">
                              <Switch
                                aria-label={
                                  s.disabled
                                    ? t.profiles.mcpEnableAria(s.name)
                                    : t.profiles.mcpDisableAria(s.name)
                                }
                                checked={!s.disabled}
                                disabled={isToggling || isDeleting}
                                onCheckedChange={() => void handleToggleDisabled(s, idx)}
                              />
                              <span
                                className={`text-meta ${
                                  s.disabled ? "text-faint" : "font-medium text-ok"
                                }`}
                              >
                                {s.disabled
                                  ? t.profiles.pluginDisabled
                                  : t.profiles.mcpEnabledTag}
                              </span>
                            </span>

                            {/* `!!js` 表达式行：表单看到的是展平字符串，必须标出来，
                                否则用户会以为改这里就能改密钥来源。2026-09-19：徽标
                                说"是什么"，悬浮说"因此能改什么"。 */}
                            {s.expr ? (
                              <span className="flex items-center gap-1">
                                <span className="rounded-md bg-warn-soft px-1.5 py-0.5 text-meta font-medium text-warn">
                                  {t.profiles.mcpExprTag}
                                </span>
                                <Tip
                                  text={t.profiles.mcpExprHint}
                                  label={t.tip.ariaFor(t.profiles.mcpExprTag)}
                                  side="bottom"
                                />
                              </span>
                            ) : null}

                            <button
                              type="button"
                              onClick={() => copyPrefix(s.name, key)}
                              className="flex items-center gap-1 rounded-md border border-line bg-bg px-1.5 py-0.5 font-mono text-meta text-faint transition-colors hover:text-ink"
                              title={t.profiles.mcpCopyPrefix}
                            >
                              {copiedName === key ? (
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
                            {/* MCP 能力探测（ADR-0022）：两条传输分支都探；失败原样透出。
                                结果按行身份取（同层重名也能探对那一行，2026-09-18 三修）。 */}
                            <Button
                              size="sm"
                              variant="outline"
                              title={t.profiles.mcpProbeBtn}
                              aria-label={`${t.profiles.mcpProbeBtn}：${s.name}`}
                              onClick={() => void handleProbe(s.name, scope, idx, s.rowId)}
                              disabled={probingKey === key || isDeleting || isToggling}
                              className="gap-1"
                            >
                              {probingKey === key ? (
                                <LoaderCircle className="size-3 animate-spin" />
                              ) : (
                                <Wrench className="size-3 text-faint" />
                              )}
                              <span>{t.profiles.mcpProbeBtn}</span>
                            </Button>
                            <Button
                              size="icon-sm"
                              variant="destructive-ghost"
                              aria-label={`${t.profiles.mcpDeleteBtn}：${s.name}`}
                              onClick={() =>
                                setPendingDelete({
                                  name: s.name,
                                  scope,
                                  rowId: s.rowId,
                                  idx,
                                  dupSameLayer: dup.sameScope,
                                })
                              }
                              disabled={isDeleting}
                            >
                              {isDeleting ? (
                                <LoaderCircle className="size-3 animate-spin" />
                              ) : (
                                <Trash2 className="size-3" />
                              )}
                            </Button>
                          </div>
                        </div>

                        {/* 连接方式（2026-09-18）：stdio 显示命令 + cwd，http 显示端点 URL。
                            此前恒显示 `command args` ⇒ 一条 streamable-http 行渲染成空命令，
                            用户读到的是"这个服务器没配命令"，而它其实配得好好的。 */}
                        <div className="flex items-center gap-2 rounded-lg border border-line bg-bg p-2 font-mono text-xs text-dim">
                          {isHttp ? (
                            <>
                              <Globe className="size-3.5 shrink-0 text-faint" />
                              <span className="truncate" title={s.url}>
                                {s.url?.trim() ? (
                                  s.url
                                ) : (
                                  <span className="text-danger">{t.profiles.mcpUrlMissing}</span>
                                )}
                              </span>
                            </>
                          ) : (
                            <>
                              <Terminal className="size-3.5 shrink-0 text-faint" />
                              <span
                                className="truncate"
                                title={`${s.command} ${joinArgs(s.args)}${
                                  s.cwd ? `\n(cwd) ${s.cwd}` : ""
                                }`}
                              >
                                {s.command} {joinArgs(s.args)}
                                {s.cwd ? <span className="text-faint"> · cwd {s.cwd}</span> : null}
                              </span>
                            </>
                          )}
                        </div>

                        {/* 环境变量 / 请求头标签（默认打码，点击显形） */}
                        {Object.keys(s.env).length > 0 && (
                          <div className="flex flex-wrap items-center gap-1.5 pt-1 text-label text-faint">
                            <span className="font-semibold text-ink">ENV:</span>
                            {Object.entries(s.env).map(([k, v]) => (
                              <SecretChip key={k} name={k} value={v} />
                            ))}
                          </div>
                        )}
                        {Object.keys(headers).length > 0 && (
                          <div className="flex flex-wrap items-center gap-1.5 pt-1 text-label text-faint">
                            <span className="font-semibold text-ink">
                              {t.profiles.mcpHeadersLabel}:
                            </span>
                            {Object.entries(headers).map(([k, v]) => (
                              <SecretChip key={k} name={k} value={v} />
                            ))}
                          </div>
                        )}

                        {/* 行级状态一行装完（2026-09-19 收拢）：重名两种与装配状态此前
                            各占一个盒子，一行最多堆 4 个、把配置本体挤出屏幕。结论留表面，
                            "为什么两条都插入 / 为什么没实例"这类长句一律进悬浮。
                            重名不是覆盖：跨层与同层都一样——两条都会插入，后加载的那条
                            因 serverName 已预留而实例化失败；只说"配置重复"用户会以为后者覆盖前者。 */}
                        {(dup.sameScope || dup.crossScope || runtime) && (
                          <div className="flex flex-wrap items-center gap-x-2.5 gap-y-1.5 pt-0.5">
                            {dup.sameScope ? (
                              <span className="flex items-center gap-1">
                                <span className="flex items-center gap-1 rounded-md border border-danger/30 bg-danger-soft px-1.5 py-0.5 text-meta font-medium text-danger">
                                  <AlertTriangle className="size-3 shrink-0" />
                                  {t.profiles.mcpDupSameScopeTag(dup.rank)}
                                </span>
                                <Tip
                                  text={t.profiles.mcpDupSameScope(s.name, dup.rank)}
                                  label={t.tip.ariaFor(t.profiles.mcpDupSameScopeTag(dup.rank))}
                                />
                              </span>
                            ) : null}
                            {dup.crossScope ? (
                              <span className="flex items-center gap-1">
                                <span className="flex items-center gap-1 rounded-md border border-danger/30 bg-danger-soft px-1.5 py-0.5 text-meta font-medium text-danger">
                                  <AlertTriangle className="size-3 shrink-0" />
                                  {t.profiles.mcpScopeConflictTag(dup.rank)}
                                </span>
                                <Tip
                                  text={t.profiles.mcpScopeConflict(s.name, dup.rank)}
                                  label={t.tip.ariaFor(t.profiles.mcpScopeConflictTag(dup.rank))}
                                />
                              </span>
                            ) : null}
                            {runtime ? (
                              <span className="flex items-center gap-1">
                                <span
                                  className={`flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-meta font-medium ${
                                    runtimeOk
                                      ? "border-line bg-ok-soft text-ok"
                                      : "border-warn/30 bg-warn-soft text-warn"
                                  }`}
                                >
                                  <Wrench className="size-3 shrink-0" />
                                  {runtime.tag}
                                </span>
                                <Tip
                                  text={runtime.why}
                                  label={t.tip.ariaFor(runtime.tag)}
                                />
                              </span>
                            ) : null}
                          </div>
                        )}

                        {/* 探测结果行内卡（2026-09-15 R1b）：成功可折叠、失败恒展开。 */}
                        {probeEntry ? (
                          <ProbeCard
                            serverName={s.name}
                            entry={probeEntry}
                            open={probeOpen[key] ?? false}
                            onToggle={() =>
                              setProbeOpen((prev) => ({ ...prev, [key]: !(prev[key] ?? false) }))
                            }
                          />
                        ) : null}
                      </div>
                    </div>
                  )
                })}
              </div>
            </section>
          ))}
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
            <DialogTitle className="flex items-center gap-2 text-sm font-semibold">
              <Boxes className="size-4 text-brand-deep" />
              <span>{t.profiles.mcpModalTitle}</span>
            </DialogTitle>
            <DialogDescription className="text-xs text-faint">
              {t.profiles.mcpModalDesc}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-3.5 py-2 text-xs">
            {/* 表达式行的事前告知（2026-09-18）：这类行只有「启用/停用」能直接保存。
                不先说，用户就是"改了个字段 → 被拒 → 更不知道哪些改动是允许的"。
                2026-09-19：表面一句短结论，完整口径进悬浮。 */}
            {formExpr ? (
              <div className="flex items-center gap-1.5 rounded-lg border border-warn/30 bg-warn-soft p-2 text-label text-warn">
                <AlertTriangle className="size-3 shrink-0" />
                <span>{t.profiles.mcpExprShort}</span>
                <Tip
                  text={t.profiles.mcpExprHint}
                  label={t.tip.ariaFor(t.profiles.mcpExprShort)}
                  side="bottom"
                />
              </div>
            ) : null}

            <div>
              <div className="flex items-center gap-1">
                <label htmlFor="mcp-form-name" className="text-faint font-semibold text-label">
                  {t.profiles.mcpServerName} <span className="text-danger">*</span>
                </label>
                <Tip
                  text={t.profiles.mcpNameHint}
                  label={t.tip.ariaFor(t.profiles.mcpServerName)}
                  side="bottom"
                />
              </div>
              <input
                id="mcp-form-name"
                value={formName}
                onChange={(e) => setFormName(e.target.value)}
                placeholder={t.profiles.mcpNamePlaceholder}
                aria-invalid={!nameOk}
                className={`mt-1 w-full rounded-xl border bg-bg px-3 py-1.5 font-mono text-xs text-ink outline-none focus:border-brand ${
                  nameOk || !formName.trim() ? "border-line" : "border-danger/50"
                }`}
              />
              {/* 约束在输入时就说出：不合规的行**能保存成功**，但插件加载期被上游
                  schema 拒掉 ⇒ 界面"已启用"、模型侧永无工具。 */}
              {!nameOk && formName.trim() ? (
                <p className="mt-0.5 text-meta text-danger">
                  {t.profiles.mcpNameInvalid(formName.trim())}
                </p>
              ) : null}
            </div>

            {/* 生效范围（2026-09-18）：决定写哪一层的 cordis.patch.yml。
                与下面的「连接方式」同一个原语（Segmented）——同一弹窗里两套
                选择器配方，视觉上就是两个不相干的控件。 */}
            <div>
              <div className="flex items-center gap-1">
                <span className="text-faint font-semibold text-label">
                  {t.profiles.mcpScopeLabel}
                </span>
                <Tip
                  text={
                    formScope === "global"
                      ? t.profiles.mcpScopeGlobalHint
                      : t.profiles.mcpScopeProfileHint
                  }
                  label={t.tip.ariaFor(t.profiles.mcpScopeLabel)}
                  side="bottom"
                />
              </div>
              <div className="mt-1">
                <Segmented
                  size="sm"
                  ariaLabel={t.profiles.mcpScopeLabel}
                  value={formScope}
                  onChange={setFormScope}
                  options={[
                    { value: "profile", label: t.profiles.mcpScopeProfile, icon: Boxes },
                    { value: "global", label: t.profiles.mcpScopeGlobal, icon: Layers },
                  ]}
                />
              </div>
            </div>

            {/* 连接方式（2026-09-18 补）：这条选择器**就是 P0 的修复**——此前表单
                恒按 stdio 保存，编辑一条 streamable-http 条目会删掉它的 url/headers
                并写进一条空命令，而界面上什么都看不出来。 */}
            <div>
              <div className="flex items-center gap-1">
                <span className="text-faint font-semibold text-label">
                  {t.profiles.mcpTransportLabel}
                </span>
                <Tip
                  text={
                    formTransport === "streamable-http"
                      ? t.profiles.mcpTransportHttpHint
                      : t.profiles.mcpTransportStdioHint
                  }
                  label={t.tip.ariaFor(t.profiles.mcpTransportLabel)}
                  side="bottom"
                />
              </div>
              <div className="mt-1">
                <Segmented
                  size="sm"
                  ariaLabel={t.profiles.mcpTransportLabel}
                  value={formTransport}
                  onChange={setFormTransport}
                  options={[
                    { value: "stdio", label: t.profiles.mcpTransportStdio, icon: Terminal },
                    {
                      value: "streamable-http",
                      label: t.profiles.mcpTransportHttp,
                      icon: Globe,
                    },
                  ]}
                />
              </div>
            </div>

            {formTransport === "streamable-http" ? (
              <>
                <div>
                  <div className="flex items-center gap-1">
                    <label htmlFor="mcp-form-url" className="text-faint font-semibold text-label">
                      {t.profiles.mcpUrl} <span className="text-danger">*</span>
                    </label>
                    <Tip
                      text={t.profiles.mcpUrlHint}
                      label={t.tip.ariaFor(t.profiles.mcpUrl)}
                      side="bottom"
                    />
                  </div>
                  <input
                    id="mcp-form-url"
                    value={formUrl}
                    onChange={(e) => setFormUrl(e.target.value)}
                    placeholder="https://mcp.example.com/… 或 http://127.0.0.1:8000/mcp"
                    className="mt-1 w-full rounded-xl border border-line bg-bg px-3 py-1.5 font-mono text-xs text-ink outline-none focus:border-brand"
                  />
                </div>

                <KvEditor
                  label={t.profiles.mcpHeaders}
                  addLabel={t.profiles.mcpAddHeader}
                  removeLabel={t.profiles.mcpHeaderRemove}
                  emptyText={t.profiles.mcpNoHeaders}
                  keyPlaceholder="Authorization"
                  valuePlaceholder="Bearer …"
                  pairs={formHeaders}
                  onChange={setFormHeaders}
                />
              </>
            ) : (
              <>
                {/* 两条提示都是"保存成功但工具永不出现"的头号真因（2026-09-18），
                    2026-09-19 收进各自标签后的悬浮：它们说的是**怎么填**，不是当前
                    出了什么问题，平铺在字段下面就是把表单当文档写。 */}
                <div className="grid grid-cols-3 gap-2">
                  <div className="col-span-1">
                    <div className="flex items-center gap-1">
                      <label
                        htmlFor="mcp-form-command"
                        className="text-faint font-semibold text-label"
                      >
                        {t.profiles.mcpCommand}
                      </label>
                      <Tip
                        text={t.profiles.mcpCommandHint}
                        label={t.tip.ariaFor(t.profiles.mcpCommand)}
                        side="bottom"
                      />
                    </div>
                    <input
                      id="mcp-form-command"
                      value={formCommand}
                      onChange={(e) => setFormCommand(e.target.value)}
                      placeholder="pnpm / uvx / 绝对路径 node"
                      className="mt-1 w-full rounded-xl border border-line bg-bg px-3 py-1.5 font-mono text-xs text-ink outline-none focus:border-brand"
                    />
                  </div>
                  <div className="col-span-2">
                    <div className="flex items-center gap-1">
                      <label htmlFor="mcp-form-args" className="text-faint font-semibold text-label">
                        {t.profiles.mcpArgs}
                      </label>
                      <Tip
                        text={t.profiles.mcpArgsHint}
                        label={t.tip.ariaFor(t.profiles.mcpArgs)}
                        side="bottom"
                      />
                    </div>
                    <input
                      id="mcp-form-args"
                      value={formArgs}
                      onChange={(e) => setFormArgs(e.target.value)}
                      placeholder="dlx @modelcontextprotocol/server-…"
                      className="mt-1 w-full rounded-xl border border-line bg-bg px-3 py-1.5 font-mono text-xs text-ink outline-none focus:border-brand"
                    />
                  </div>
                </div>

                {/* 工作目录（cwd）：不依赖 PATH 的写法必须填 */}
                <div>
                  <div className="flex items-center gap-1">
                    <label htmlFor="mcp-form-cwd" className="text-faint font-semibold text-label">
                      {t.profiles.mcpCwd}
                    </label>
                    <Tip
                      text={t.profiles.mcpCwdHint}
                      label={t.tip.ariaFor(t.profiles.mcpCwd)}
                      side="bottom"
                    />
                  </div>
                  <input
                    id="mcp-form-cwd"
                    value={formCwd}
                    onChange={(e) => setFormCwd(e.target.value)}
                    placeholder="/绝对/路径/到/mcp-server"
                    className="mt-1 w-full rounded-xl border border-line bg-bg px-3 py-1.5 font-mono text-xs text-ink outline-none focus:border-brand"
                  />
                </div>

                <KvEditor
                  label={t.profiles.mcpEnv}
                  addLabel={t.profiles.mcpAddEnv}
                  removeLabel={t.profiles.mcpEnvRemove}
                  emptyText={t.profiles.mcpNoEnv}
                  keyPlaceholder="KEY"
                  valuePlaceholder="VALUE"
                  pairs={formEnv}
                  onChange={setFormEnv}
                />
              </>
            )}

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
            <Button onClick={handleSaveServer} disabled={saving || !nameOk}>
              {saving && <LoaderCircle className="size-3.5 animate-spin mr-1.5" />}
              <span>{t.profiles.mcpSaveBtn}</span>
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* 删除确认（U9）：从**所在层**的 cordis.patch.yml 安全删除，先确认 */}
      <ConfirmDialog
        open={pendingDelete !== null}
        title={pendingDelete ? t.profiles.mcpDeleteConfirm(pendingDelete.name) : ""}
        note={`${t.profiles.mcpDeleteNote}${
          pendingDelete?.scope === "global" ? `（${t.profiles.mcpScopeGlobalHint}）` : ""
        }${
          // 同层同名时"删 fs"这句歧义（两行都叫 fs）——把行 id 说出来，它正是文件里
          // 那一行的 `id:`，用户能在原文里对上。
          pendingDelete?.dupSameLayer && pendingDelete.rowId
            ? t.profiles.mcpDeleteRowId(pendingDelete.rowId)
            : ""
        }`}
        confirmLabel={t.profiles.mcpDeleteBtn}
        cancelLabel={t.confirm.cancel}
        onConfirm={() => {
          const target = pendingDelete
          setPendingDelete(null)
          if (target) {
            void handleDeleteServer(target.name, target.scope, target.idx, target.rowId)
          }
        }}
        onClose={() => setPendingDelete(null)}
      />
    </div>
  )
}
