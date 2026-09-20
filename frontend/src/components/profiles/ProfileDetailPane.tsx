import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import {
  ArrowUpCircle,
  Boxes,
  Check,
  Code2,
  Copy,
  Import,
  Layers,
  LoaderCircle,
  Package,
  Plus,
  Puzzle,
  RefreshCw,
  Search,
  ShieldCheck,
  Sparkles,
  Star,
  Trash2,
  TriangleAlert,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { useCopy } from "@/hooks/useCopy"
import { useI18n } from "@/stores/i18nStore"
import { runtimeChipFor, runtimeSummary, validatePluginSpec } from "@/lib/profiles"
import type { RuntimeChip } from "@/lib/profiles"
import { pluginToggleTargets, splitSettledToggles, toggleIntent } from "@/lib/pluginToggle"
import {
  dockCuratedCaps,
  matchesKindFilter,
  matchesSearch,
  mergePluginRows,
} from "@/lib/pluginCatalog"
import type { MergedPluginRow, PluginKindFilter } from "@/lib/pluginCatalog"
import type {
  Capability,
  PluginEntry,
  PluginRowState,
  PluginRuntimeSnapshot,
  ProfileDetail,
  RuntimeEntry,
} from "@/types/ipc"
import { Button } from "@/components/ui/button"
import { Segmented } from "@/components/ui/segmented"
import { ConfirmDialog } from "@/components/ui/confirm-dialog"
import { Switch } from "@/components/ui/switch"
import { YamlEditor } from "@/components/ui/yaml-editor"
import { PluginImportPickerDialog } from "@/components/profiles/PluginImportPickerDialog"
import { McpManager } from "@/components/profiles/McpManager"
import { ExperimentalCapabilities } from "@/components/market/ExperimentalCapabilities"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

// 开关落定轮询（2026-09-17）：dsh 对 web 形态 profile 是 `patchReload: live`
// （chokidar 盯 cordis.patch.yml），克隆实机实测改文件后 0.43s 内 fiber 注销 /
// 0.44s 内重建；8×400ms ≈ 3.2s 留足余量，到点未落定即如实报「重启后生效」。
const TOGGLE_SETTLE_ATTEMPTS = 8
const TOGGLE_SETTLE_INTERVAL_MS = 400

export function ProfileDetailPane({
  name,
  isDefault,
  isRunning,
  isSwitching = false,
  busy: _busy,
  onLaunch: _onLaunch,
  onRestart,
  onSetDefault,
  onNotice,
}: {
  name: string | null
  isDefault: boolean
  isRunning: boolean
  isSwitching?: boolean
  busy: boolean
  onLaunch: () => void
  onRestart: () => void
  onSetDefault: () => void
  onCopy: () => void
  onRename: () => void
  onDelete: () => void
  onNotice: (text: string, kind?: "ok" | "warn") => void
}) {
  const { t, activeLocale } = useI18n()
  const [tab, setTab] = useState<"plugins" | "caps" | "patch" | "mcp">("plugins")
  const [detail, setDetail] = useState<ProfileDetail | null>(null)
  const [plugins, setPlugins] = useState<PluginEntry[] | null>(null)
  const [runtime, setRuntime] = useState<PluginRuntimeSnapshot | null>(null)
  const [rows, setRows] = useState<PluginRowState[] | null>(null)
  const [error, setError] = useState<string | null>(null)

  // 插件搜索过滤
  const [searchQuery, setSearchQuery] = useState("")
  // 三类筛选（ADR-0028 第二批：内置 / 第三方 / 实验性合一后的 facet chips）
  const [kindFilter, setKindFilter] = useState<PluginKindFilter>("all")
  // 能力组 hover 联动：鼠标进组即高亮整组（基座 + provider 从属关系的交互呈现）
  const [hoveredCap, setHoveredCap] = useState<string | null>(null)

  // 操作状态
  const [opBusy, setOpBusy] = useState<string | null>(null)
  // 开关已写入、运行态尚未落定的行：pkg -> 目标启用态（行内显示「生效中」，见 settleToggle）。
  // 用**集合**而非单个（2026-09-17 独立复核 P2）：连点两行时前一行不该被后一行挤掉结论。
  const [pendingToggles, setPendingToggles] = useState<Record<string, boolean>>({})
  const pendingRef = useRef<Record<string, boolean>>({})
  // ref 与 state 始终同写：轮询回调读 ref（不吃闭包旧值），渲染读 state
  const setPending = (next: Record<string, boolean>) => {
    pendingRef.current = next
    setPendingToggles(next)
  }
  const settleTimer = useRef<number | null>(null)
  // 落定回调里比对"当前还在看的 profile"（切换后旧轮询静默收尾，不播结论）
  const nameRef = useRef(name)
  nameRef.current = name
  const [installOpen, setInstallOpen] = useState(false)
  const [installSpec, setInstallSpec] = useState("")
  const [installError, setInstallError] = useState<string | null>(null)
  const [importOpen, setImportOpen] = useState(false)

  // 更新检查状态
  const [updateMap, setUpdateMap] = useState<Record<string, string> | null>(null)
  const [checkState, setCheckState] = useState<"idle" | "busy" | "done">("idle")
  const [checkMeta, setCheckMeta] = useState<{ checked: number; failed: number } | null>(null)

  // 版本选择对话框
  const [versionPick, setVersionPick] = useState<{
    pkg: string
    current: string
    latest: string
    items: string[] | null
  } | null>(null)
  const [versionsError, setVersionsError] = useState<string | null>(null)

  // YAML 复制反馈
  const { copied: copiedYaml, copy: copyYamlText } = useCopy()

  // 卸载确认（2026-09-08，U9）：卸载不可撤销，先过确认对话框再执行。
  const [confirmRemove, setConfirmRemove] = useState<string | null>(null)

  // 运行态快照（回环 pluginInventory/list）：开关落定轮询与整页 reload 共用同一取数
  // 口径，避免两处各自拼一份（2026-09-17）。返回**这一次**取回的结论 + 条目：
  //  - `observed=false` = 这次查询本身失败（回环不可达/401…）：**不可**据此断言任何时机；
  //  - `appliesToProfile=false` = 查到了，但会话没在跑（快照 profile 不是它）。
  // 两者必须分开，否则"查询失败"会被当成"需要重启"播出去（复核 P1 的同族问题）。
  // 判定也不许读渲染闭包里的旧 `runtime`。
  const refreshRuntime = useCallback((profile: string) => {
    return api
      .getPluginRuntime()
      .then((s) => {
        // 换档后旧档的快照不许写进新页面（与 loadCaps 同族护栏，ADR-0028 §4.1：
        // 左列表切档不受面板 busy 约束，慢响应会串档）。
        if (nameRef.current === profile) setRuntime(s)
        const appliesToProfile = s.profile !== null && s.profile === profile
        return { observed: true, appliesToProfile, entries: appliesToProfile ? s.entries : [] }
      })
      .catch(() => {
        if (nameRef.current === profile) setRuntime({ profile: null, entries: [] })
        return { observed: false, appliesToProfile: false, entries: [] as RuntimeEntry[] }
      })
  }, [])

  const reload = useCallback(() => {
    if (!name) return
    const target = name
    // 旧档的回读整体作废：请求发出后用户可能已经切到别的档（左列表 onSelect 不受
    // busy 约束），迟到的响应会把旧档的详情/清单写进新档页面（ADR-0028 §4.1 同族）。
    api
      .getProfileDetail(target)
      .then((d) => {
        if (nameRef.current === target) setDetail(d)
      })
      .catch((e) => {
        if (nameRef.current === target) setError(String(e))
      })
    api
      .listProfilePlugins(target)
      .then((p) => {
        if (nameRef.current === target) setPlugins(p)
      })
      .catch(() => {
        if (nameRef.current === target) setPlugins([])
      })
    refreshRuntime(target).catch(() => {})
    api
      .getPluginRows(target)
      .then((r) => {
        if (nameRef.current === target) setRows(r)
      })
      .catch(() => {
        if (nameRef.current === target) setRows([])
      })
  }, [name, refreshRuntime])

  useEffect(() => {
    if (!name) return
    setDetail(null)
    setPlugins(null)
    setRuntime(null)
    setRows(null)
    setError(null)
    setInstallOpen(false)
    setInstallSpec("")
    setInstallError(null)
    setSearchQuery("")
    setKindFilter("all")
    // 旧档的能力目录必须清：留着会让新档的包顶着旧档的「实验性」标（归属是按档的）
    setCaps(null)
    setCapsError(null)
    setUpdateMap(null)
    setCheckState("idle")
    setPending({})
    // 换 profile 也要收掉上一轮的落定轮询（否则旧实例的快照/toast 会串到新页面）
    if (settleTimer.current !== null) {
      window.clearTimeout(settleTimer.current)
      settleTimer.current = null
    }
    reload()
  }, [name, reload])

  // 卸载即收掉待定轮询（切页不残留定时器）
  useEffect(
    () => () => {
      if (settleTimer.current !== null) window.clearTimeout(settleTimer.current)
    },
    [],
  )

  // ── 实验能力目录（受控数据，2026-09-20 ADR-0028 第二批）──────────────────
  //
  // 为什么由**父级**取数：插件列表要给每个包打「实验性 · <能力>」标，归属只有能力目录
  // 知道；而目录是按 profile 的。两处各取一次 = 同窗两条目录链（ADR-0028 §2 禁），
  // 且写操作后必然对不上。故目录与行表/详情一样随本页取数，面板只消费。
  //
  // 档位护栏与 ExperimentalCapabilities 动作侧闸门同族（ADR-0028 §4.1）：换档入口
  // 在左侧列表，旧档的回读自己会取到更大的 seq——"最后一次请求"反而成了旧档的数据，
  // 令牌拦不住。判据因此是**按最新档位早退**：旧档回读整体作废，不占令牌、不发请求。
  const [caps, setCaps] = useState<Capability[] | null>(null)
  const [capsLoading, setCapsLoading] = useState(false)
  const [capsError, setCapsError] = useState<string | null>(null)
  const capsSeq = useRef(0)
  const loadCaps = useCallback(
    (profile: string) => {
      if (nameRef.current !== profile) return
      const seq = (capsSeq.current += 1)
      setCapsLoading(true)
      api
        .listExperimentalCapabilities(profile, activeLocale)
        .then((next) => {
          if (seq !== capsSeq.current) return
          setCaps(next)
          setCapsError(null)
        })
        .catch((e) => {
          if (seq !== capsSeq.current) return
          setCaps(null)
          setCapsError(`${t.market.capLoadFailed}${t.market.capValueSep}${String(e)}`)
        })
        .finally(() => {
          if (seq === capsSeq.current) setCapsLoading(false)
        })
    },
    [activeLocale, t],
  )

  useEffect(() => {
    if (!name) return
    loadCaps(name)
  }, [name, loadCaps])

  /** 「插件列表」→「实验能力」的跳转请求（去开关）。nonce 保证同一能力连跳两次
   *  也能再次触发选中（id 相同也要重新展开详情面）。 */
  const [capFocus, setCapFocus] = useState<{ id: string; nonce: number } | null>(null)
  const goToggleCapability = (cap: Capability) => {
    setTab("caps")
    setCapFocus({ id: cap.id, nonce: Date.now() })
  }

  // 会话由「未运行」转为「运行中」时补取一次运行态（复核 P1 的另一半）：面板不重挂，
  // 旧实现只按 name 变化取数 ⇒ 启动后所有行都没有运行态徽标，页头却写着"会话运行中"。
  const prevRunning = useRef(isRunning)
  useEffect(() => {
    const wasRunning = prevRunning.current
    prevRunning.current = isRunning
    if (!name || !isRunning || wasRunning) return
    refreshRuntime(name).catch(() => {})
  }, [isRunning, name, refreshRuntime])

  // 卸载 / 更新插件
  const runOp = (op: "remove" | "update", pkg: string) => {
    if (!name || opBusy) return
    setOpBusy(`${op}:${pkg}`)
    const call = op === "remove" ? api.removePlugin : api.updatePlugin
    call(name, pkg)
      .then((out) => {
        if (out.ok) {
          onNotice(out.detail, "ok")
          reload()
        } else {
          onNotice(out.detail, "warn")
        }
      })
      .catch((e) => onNotice(String(e), "warn"))
      .finally(() => setOpBusy(null))
  }

  // 卸载入口：先确认（U9）。确认后立即关框、由行内 busy 态回报进度。
  const requestRemove = (pkg: string) => {
    if (!name || opBusy) return
    setConfirmRemove(pkg)
  }

  // 安装插件
  const submitInstall = () => {
    if (!name || opBusy) return
    const spec = installSpec.trim()
    const invalid = validatePluginSpec(spec)
    if (invalid) {
      setInstallError(invalid)
      return
    }
    setInstallError(null)
    setOpBusy("install")
    api
      .installPlugin(name, spec)
      .then((out) => {
        if (out.ok) {
          onNotice(out.detail, "ok")
          setInstallOpen(false)
          setInstallSpec("")
          reload()
        } else {
          setInstallError(out.detail)
        }
      })
      .catch((e) => setInstallError(String(e)))
      .finally(() => setOpBusy(null))
  }

  // 启停插件（通过现代 Switch 切换）；补丁包（ADR 第七次修订）= 全部贡献行，
  // 同一 patch 文件读改写必须串行（并发 invoke 相互覆盖，2026-09-08）。
  //
  // 2026-09-17（维护者实机报「刚打开插件却还显示停用，切标签页回来才变运行中」）：
  // 旧实现写完成功后**只重取行表**（配置），运行态快照一直停在进入页面那一刻 ⇒
  // 同一行出现"开关是开的 / 徽标说停用"的自相矛盾，直到切页重挂才自愈。
  // 现在：写完后立刻拉一次运行态，并短轮询到落定（实测热载 0.43s 内完成，故 400ms×8
  // 足够；到点仍未落定 = 该 Profile 的 patchReload 不是 live 或未运行 → 如实报"重启后生效"）。
  const settleToggle = (profile: string, attempt: number) => {
    settleTimer.current = window.setTimeout(() => {
      settleTimer.current = null
      refreshRuntime(profile)
        .then(({ observed, appliesToProfile, entries }) => {
          // 已切走/已换 profile：静默收尾，别把上一页的结论播到新页面
          if (nameRef.current !== profile) {
            setPending({})
            return
          }
          const pending = pendingRef.current
          // 查询失败时不结算（没有观测面），但保留待定继续重试
          const { settled, remaining } = observed
            ? splitSettledToggles(pending, entries)
            : { settled: [] as string[], remaining: pending }
          if (settled.length > 0) {
            setPending(remaining)
            for (const p of settled) onNotice(t.profiles.toggleApplied(p, pending[p]), "ok")
          }
          if (Object.keys(remaining).length === 0) return
          // 会话确实没在跑：等不到观测面，直接如实报"重启后生效"
          if (observed && !appliesToProfile) {
            setPending({})
            for (const [p, wantEnabled] of Object.entries(remaining)) {
              onNotice(t.profiles.toggleRestart(p, wantEnabled), "ok")
            }
            return
          }
          if (attempt + 1 >= TOGGLE_SETTLE_ATTEMPTS) {
            // 到点仍未落定：查到"没生效"→ 重启后生效；查不到 → 只敢说"配置已写入"
            setPending({})
            for (const [p, wantEnabled] of Object.entries(remaining)) {
              onNotice(
                observed
                  ? t.profiles.toggleRestart(p, wantEnabled)
                  : t.profiles.toggleDone(p, wantEnabled),
                "ok",
              )
            }
            return
          }
          settleToggle(profile, attempt + 1)
        })
        .catch(() => setPending({}))
    }, TOGGLE_SETTLE_INTERVAL_MS)
  }

  const toggleDisabled = (pkg: string) => {
    if (!name || opBusy) return
    const row = rows?.find((r) => r.pkg_name === pkg)
    if (!row) return
    const targets = pluginToggleTargets(row)
    if (targets.length === 0) return
    const profile = name
    // 两种口径分开（2026-09-17 独立复核抓到极性缺陷后的护栏，见 lib/pluginToggle.ts）：
    // 写 patch 用 `writeDisabled`，文案与落定判据用 `wantEnabled`。
    const { wantEnabled, writeDisabled } = toggleIntent(row)
    setOpBusy(`toggle:${pkg}`)
    targets
      .reduce<Promise<void>>(
        (acc, id) => acc.then(() => api.setPluginDisabled(profile, id, writeDisabled)),
        Promise.resolve(),
      )
      .then(() => {
        // 配置已落盘：行表（配置侧徽标/开关）与运行态（运行侧徽标）都要立刻刷新。
        api
          .getPluginRows(profile)
          .then((r) => setRows(r))
          .catch((e) => onNotice(String(e), "warn"))
        refreshRuntime(profile)
          .then(({ observed, appliesToProfile, entries }) => {
            // 判定用**这一次**取回的新快照，不看渲染闭包里的旧 runtime（复核 P1）
            if (!observed) {
              // 查询本身失败：没有观测面，不猜时机，只报"配置已写入"
              onNotice(t.profiles.toggleDone(pkg, wantEnabled), "ok")
              return
            }
            if (!appliesToProfile) {
              // 查到会话没在跑：配置写完即完成，生效只能等下次启动——不空转轮询
              onNotice(t.profiles.toggleRestart(pkg, wantEnabled), "ok")
              return
            }
            if (runtimeChipFor(pkg, entries) === null) {
              // 该行没有可观测的运行态条目（补丁包：贡献行用的是各自的 name，包名不成条目）
              // ——观测不到就不许承诺时机，只报"配置已写入"。
              onNotice(t.profiles.toggleDone(pkg, wantEnabled), "ok")
              return
            }
            // 并入待定集合（不挤掉上一行的落定）；已有轮询在跑就让它一并结算
            setPending({ ...pendingRef.current, [pkg]: wantEnabled })
            if (settleTimer.current === null) settleToggle(profile, 0)
          })
          .catch(() => {})
      })
      .catch((e) => onNotice(String(e), "warn"))
      .finally(() => setOpBusy(null))
  }

  // 更新检查
  const runUpdateCheck = () => {
    if (!name || opBusy || checkState === "busy") return
    setCheckState("busy")
    api
      .checkPluginUpdates(name)
      .then((r) => {
        setUpdateMap(Object.fromEntries(r.updates.map((u) => [u.name, u.latest])))
        setCheckMeta({ checked: r.checked, failed: r.failed })
        setCheckState("done")
      })
      .catch((e) => {
        onNotice(String(e), "warn")
        setCheckState("idle")
      })
  }

  // 版本弹窗
  const openVersionPick = (pkg: string, current: string, latest: string) => {
    setVersionPick({ pkg, current, latest, items: null })
    setVersionsError(null)
    api
      .listPluginVersions(pkg)
      .then((items) => {
        setVersionPick((v) => (v && v.pkg === pkg ? { ...v, items } : v))
      })
      .catch((e) => setVersionsError(String(e)))
  }

  const installVersion = (spec: string) => {
    if (!name || opBusy) return
    const pkg = versionPick?.pkg ?? ""
    setOpBusy("install")
    api
      .installPlugin(name, spec)
      .then((out) => {
        if (out.ok) {
          onNotice(out.detail, "ok")
          setUpdateMap((m) => {
            if (!m) return m
            const next = { ...m }
            delete next[pkg]
            return next
          })
          setVersionPick(null)
          reload()
        } else {
          setVersionsError(out.detail)
        }
      })
      .catch((e) => setVersionsError(String(e)))
      .finally(() => setOpBusy(null))
  }

  const copyYaml = async () => {
    if (!detail?.patch_yaml) return
    // 2026-09-08：写失败不再静默（原来只挂 .then 成功分支）
    const outcome = await copyYamlText(detail.patch_yaml)
    if (outcome.ok) onNotice(t.profiles.copyPatchSuccess, "ok")
    else onNotice(t.error.copyFailed, "warn")
  }

  const liveEntries =
    runtime !== null && runtime.profile !== null && runtime.profile === name
      ? runtime.entries
      : []

  // 运行态徽标文案与配色（2026-09-17）：文案走字典（en 不再漏中文）；
  // 配色只有 failed 用警示色、「运行中/加载中」用 ok 色，两个"还没到位"的状态
  // 用中性灰——旧版把「已停用」也刷成 ok 绿，是这次实机困惑的一部分。
  const chipText = (chip: RuntimeChip) =>
    chip.count > 1
      ? `${t.profiles.chip[chip.kind]}×${chip.count}`
      : t.profiles.chip[chip.kind]
  const chipTone = (chip: RuntimeChip) =>
    chip.kind === "failed"
      ? "bg-warn-soft text-warn"
      : chip.kind === "active" || chip.kind === "loading"
        ? "bg-ok-soft text-ok font-medium"
        : "bg-line-soft text-dim"

  const deps = useMemo(
    () => plugins?.filter((p) => p.kind === "dependency") ?? [],
    [plugins],
  )
  const depCount = deps.length

  // ── 三类合一的插件清单（ADR-0028 第二批：底座 / 第三方 / 实验性一行）────────
  // 合并、打标、层序全部走纯函数（lib/pluginCatalog.ts，可单测）；这里只做筛选。
  // 层栈成员在前（组合序），其余在后——层序是「底座组合」tab 留下的唯一信息。
  //
  // dock 策展的能力 = 目录排 dsh 安装自带的（单一规则，父级过滤一次）：面板与列表
  // 共用——自带能力（OPTIONAL_BUNDLES）归 dsh 官方插件页托管，dock 不再摆第二套入口，
  // 启用后其层按内置层在列表展示（2026-09-20 维护者裁定，ADR-0020 §2.8）。
  const curatedCaps = useMemo(() => dockCuratedCaps(caps ?? []), [caps])
  const merged = useMemo(
    () =>
      mergePluginRows({
        plugins: plugins ?? [],
        bundles: detail?.bundles ?? [],
        rows: rows ?? [],
        caps: curatedCaps,
      }),
    [plugins, detail, rows, curatedCaps],
  )
  const filteredRows = useMemo(
    () =>
      merged.rows.filter(
        (r) => matchesKindFilter(r, kindFilter) && matchesSearch(r, searchQuery),
      ),
    [merged, kindFilter, searchQuery],
  )

  // 能力组切块：同一能力的行在 merged 里已相邻（lib 保证），这里按 capability id 把
  // 连续行合成一组——组内基座在上、provider 缩进其下，整组 hover 联动（从属关系的交互呈现）。
  const rowChunks = useMemo(() => {
    const chunks: { capability: Capability | null; rows: MergedPluginRow[] }[] = []
    for (const r of filteredRows) {
      const capId = r.capability?.id ?? null
      const last = chunks[chunks.length - 1]
      if (capId !== null && last && last.capability?.id === capId) last.rows.push(r)
      else chunks.push({ capability: r.capability, rows: [r] })
    }
    return chunks
  }, [filteredRows])

  if (!name) {
    return (
      <div className="flex h-full min-h-[480px] flex-col items-center justify-center rounded-2xl border border-dashed border-line bg-panel/50 p-8 text-center">
        <div className="mb-3 flex size-12 items-center justify-center rounded-2xl bg-line-soft text-faint">
          <Package className="size-6" />
        </div>
        <h2 className="text-ink text-sm font-semibold">{t.profiles.emptySelectTitle}</h2>
        <p className="text-dim mt-1 max-w-sm text-xs leading-relaxed">
          {t.profiles.emptySelectSubtitle}
        </p>
      </div>
    )
  }

  return (
    <div className="flex h-full min-h-[480px] flex-col overflow-hidden rounded-2xl border border-line bg-panel shadow-xs">
      {/* 顶部 Profile 标题栏 */}
      <header className="border-b border-line bg-panel/90 px-5 py-4 backdrop-blur-xs">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <h2
                className="text-ink truncate font-mono text-base font-bold tracking-tight"
                title={name ?? undefined}
              >
                {name}
              </h2>
              {isSwitching ? (
                <span className="bg-info-soft text-info border border-info/30 inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-meta font-medium animate-pulse">
                  <LoaderCircle className="size-3 animate-spin text-info" />
                  <span>{isRunning ? t.profiles.reloadingWorkbench : t.profiles.launchingProfile}</span>
                </span>
              ) : isRunning ? (
                <span className="bg-ok-soft text-ok inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-meta font-medium">
                  <span className="bg-ok size-1.5 animate-pulse rounded-full" />
                  {t.profiles.runningBadge}
                </span>
              ) : null}
            </div>
            <p className="text-faint mt-0.5 text-xs">
              {detail ? (
                <>
                  <span>{t.profiles.manifestNameLabel}</span>
                  <span className="font-mono font-medium text-ink">{detail.package_name}</span>
                </>
              ) : (
                t.profiles.detailLoading
              )}
            </p>
          </div>

          {/* 顶栏快速操作 */}
          <div className="flex items-center gap-2">
            <Button
              size="sm"
              variant={isDefault ? "secondary" : "outline"}
              onClick={onSetDefault}
              disabled={isDefault}
              className={`gap-1 text-xs ${
                isDefault
                  ? "bg-wash text-brand-deep border border-brand/20 font-medium cursor-default opacity-100"
                  : ""
              }`}
            >
              <Star className={`size-3.5 ${isDefault ? "text-brand-deep fill-current" : ""}`} />
              {isDefault ? t.profiles.defaultIs : t.profiles.setDefault}
            </Button>
          </div>
        </div>

        {/* 顶部 Tab 切换（批次 3：第 6 份手搓分段器 → Segmented stretch；
            与顶栏导航同配方，靠"整宽等分"而非"更大更亮"区分层级）。
            2026-09-20（ADR-0028 第二批）：「底座组合」tab 退役——层栈并入「插件列表」
            （行内「层 N」序标承载组合序），本面板收敛为 插件列表 / 实验能力 / MCP /
            Patch 四段。 */}
        <Segmented<"plugins" | "caps" | "patch" | "mcp">
          className="mt-4"
          stretch
          ariaLabel={name}
          value={tab}
          onChange={setTab}
          options={[
            {
              value: "plugins",
              icon: Package,
              label: (
                <>
                  {t.profiles.tabPlugins}
                  {merged.rows.length > 0 && (
                    <span className="rounded-full bg-line px-1.5 font-mono text-meta">
                      {merged.rows.length}
                    </span>
                  )}
                </>
              ),
            },
            // 实验能力（ADR-0028：自插件中心迁入，档位 = 本页选中档，与插件列表同作用域）。
            { value: "caps", icon: Sparkles, label: t.profiles.tabCaps },
            { value: "mcp", icon: Boxes, label: t.profiles.tabMcp },
            { value: "patch", icon: Code2, label: t.profiles.tabPatch },
          ]}
        />
      </header>

      {/* 主体工作区 */}
      <div className="flex-1 overflow-y-auto p-5">
        {error && (
          <div className="mb-4 rounded-xl bg-warn-soft p-3 text-xs text-warn">
            {error}
          </div>
        )}

        {/* ================= Tab 1: 插件列表（外挂插件控制台） ================= */}
        {tab === "plugins" && (
          <div className="space-y-4">
            {/* 插件工具栏：搜索 + 安装 + 导入 + 检查更新 */}
            <div className="flex flex-wrap items-center justify-between gap-2.5">
              <div className="relative min-w-[180px] flex-1">
                <Search className="text-faint absolute inset-y-0 left-2.5 my-auto size-3.5" />
                <input
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  placeholder={t.profiles.searchPluginsPlaceholder}
                  aria-label={t.profiles.searchPluginsPlaceholder}
                  className="border-line bg-bg text-ink placeholder:text-faint focus:border-brand w-full rounded-lg border py-1.5 pr-3 pl-8 font-mono text-xs outline-none transition-colors"
                />
              </div>

              <div className="flex shrink-0 items-center gap-1.5">
                <Button
                  size="sm"
                  variant="outline"
                  disabled={opBusy !== null}
                  onClick={() => {
                    setInstallOpen((o) => !o)
                    setInstallError(null)
                  }}
                  className="gap-1 text-xs"
                >
                  <Plus className="size-3.5" />
                  {t.profiles.pluginInstallBtn}
                </Button>

                <Button
                  size="sm"
                  variant="outline"
                  disabled={opBusy !== null}
                  onClick={() => setImportOpen(true)}
                  className="gap-1 text-xs"
                >
                  <Import className="size-3.5" />
                  {t.profiles.importBtn}
                </Button>

                <Button
                  size="sm"
                  variant="outline"
                  title={t.profiles.checkUpdatesBtn}
                  disabled={opBusy !== null || depCount === 0 || checkState === "busy"}
                  onClick={runUpdateCheck}
                  className="gap-1.5 text-xs font-medium"
                >
                  {checkState === "busy" ? (
                    <LoaderCircle className="size-3.5 animate-spin text-brand-deep" />
                  ) : (
                    <RefreshCw className="size-3.5" />
                  )}
                  <span>{checkState === "busy" ? t.profiles.checkingBtn : t.profiles.checkUpdatesBtn}</span>
                </Button>
              </div>
            </div>

            {/* 内联安装行 */}
            {installOpen && (
              <div className="rounded-xl border border-brand/40 bg-wash/30 p-3">
                <div className="flex gap-2">
                  <input
                    autoFocus
                    disabled={opBusy !== null}
                    aria-label={t.profiles.pluginInstallPlaceholder}
                    value={installSpec}
                    onChange={(e) => setInstallSpec(e.target.value)}
                    onKeyDown={(e) => e.key === "Enter" && submitInstall()}
                    placeholder={t.profiles.pluginInstallPlaceholder}
                    className="border-line bg-panel text-ink placeholder:text-faint focus:border-brand min-w-0 flex-1 rounded-lg border px-3 py-1.5 font-mono text-xs outline-none transition-colors"
                  />
                  <Button
                    size="sm"
                    disabled={opBusy !== null}
                    onClick={submitInstall}
                    className="shrink-0"
                  >
                    {opBusy === "install" ? (
                      <LoaderCircle className="size-3.5 animate-spin" />
                    ) : (
                      t.profiles.pluginInstallSubmit
                    )}
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={opBusy !== null}
                    onClick={() => {
                      setInstallOpen(false)
                      setInstallSpec("")
                      setInstallError(null)
                    }}
                  >
                    {t.profiles.pluginInstallCancel}
                  </Button>
                </div>
                {installError && (
                  <p className="text-warn mt-2 text-xs">{installError}</p>
                )}
              </div>
            )}

            {/* 更新检查结论 */}
            {checkState === "done" && checkMeta && (
              <div className="text-faint flex items-center justify-between rounded-lg bg-bg px-3 py-1.5 text-label">
                <span>{t.profiles.updateChecked(checkMeta)}</span>
                {updateMap && Object.keys(updateMap).length === 0 && (
                  <span className="text-ok font-medium">{t.profiles.allUpToDate}</span>
                )}
              </div>
            )}

            {/* 运行态概要 */}
            {liveEntries.length > 0 ? (
              <div className="text-faint px-1 text-label tabular-nums">
                {t.profiles.runtimeSummary(runtimeSummary(liveEntries))}
              </div>
            ) : null}

            {/* 三类筛选（与三个行内标记同名；计数可重叠——dsh 自带的实验层
                同时计入「内置」与「实验性」，合计可大于总数，不是分区） */}
            <Segmented<PluginKindFilter>
              size="sm"
              ariaLabel={t.profiles.tabPlugins}
              value={kindFilter}
              onChange={setKindFilter}
              options={[
                {
                  value: "all",
                  icon: Package,
                  label: (
                    <>
                      {t.profiles.listFilterAll}
                      <span className="font-mono text-meta opacity-70">{merged.rows.length}</span>
                    </>
                  ),
                },
                {
                  value: "builtin",
                  icon: ShieldCheck,
                  label: (
                    <>
                      {t.profiles.listFilterBuiltin}
                      <span className="font-mono text-meta opacity-70">
                        {merged.counts.builtin}
                      </span>
                    </>
                  ),
                },
                {
                  value: "thirdParty",
                  icon: Puzzle,
                  label: (
                    <>
                      {t.profiles.listFilterThirdParty}
                      <span className="font-mono text-meta opacity-70">
                        {merged.counts.thirdParty}
                      </span>
                    </>
                  ),
                },
                {
                  value: "experimental",
                  icon: Sparkles,
                  label: (
                    <>
                      {t.profiles.listFilterExperimental}
                      <span className="font-mono text-meta opacity-70">
                        {merged.counts.experimental}
                      </span>
                    </>
                  ),
                },
              ]}
            />

            {/* 能力目录读取失败时如实说明：没标的实验包不许被默默显示成第三方 */}
            {capsError && (
              <div className="flex items-center gap-2 rounded-lg bg-warn-soft px-3 py-1.5 text-label text-warn">
                <TriangleAlert className="size-3.5 shrink-0" />
                <span className="min-w-0">{t.profiles.capsTagUnavailable}</span>
              </div>
            )}

            {/* 官方桌面运行时说明（随「底座组合」tab 退役迁到此处：这批内置组件
                现在就在下方列表里，带「内置」标记） */}
            {detail?.package_name === "@deepseek-ai/dsh-desktop-runtime" && (
              <div className="rounded-xl border border-line bg-wash/50 p-4 space-y-2">
                <div className="flex items-center gap-2">
                  <span className="font-mono text-xs font-semibold text-ink">
                    {t.profiles.desktopRuntimeName}
                  </span>
                  <span className="bg-wash text-brand-deep border border-brand/20 rounded-md px-1.5 py-0.5 text-meta font-medium">
                    {t.profiles.desktopRuntimeTag}
                  </span>
                </div>
                <p className="text-dim text-xs leading-relaxed">
                  {t.profiles.desktopRuntimeDescPrefix}
                  <code>@deepseek-ai/dsh-desktop-runtime</code>
                  {t.profiles.desktopRuntimeDescMid}
                  <code>desktop-packages/</code>
                  {t.profiles.desktopRuntimeDescSuffix}
                </p>
                <p className="text-faint text-meta">
                  {t.profiles.desktopRuntimeNote}
                </p>
              </div>
            )}

            {/* 三类合一的插件列表（ADR-0028 第二批：层栈在前、按组合序；其余按依赖序） */}
            {plugins === null ? (
              <div className="text-faint py-12 text-center text-xs">
                <LoaderCircle className="mx-auto mb-2 size-5 animate-spin text-brand-deep" />
                {t.profiles.busyShort}
              </div>
            ) : merged.rows.length === 0 ? (
              <div className="rounded-xl border border-dashed border-line bg-bg p-8 text-center text-xs text-faint">
                {t.profiles.detailEmptyDeps}
              </div>
            ) : filteredRows.length === 0 ? (
              <div className="rounded-xl border border-dashed border-line bg-bg p-8 text-center text-xs text-faint">
                {searchQuery.trim() ? t.profiles.searchNoPlugin : t.profiles.listFilterEmpty}
              </div>
            ) : (
              <div className="divide-y divide-line rounded-xl border border-line bg-panel shadow-xs">
                {rowChunks.map((chunk) =>
                  chunk.capability ? (
                    // 能力组：基座 + provider 从属一组。整组 hover 联动（从属关系要从
                    // 交互上感知，不是两行平铺各说各话）；provider 行缩进挂在其下。
                    <div
                      key={chunk.capability.id}
                      onMouseEnter={() => setHoveredCap(chunk.capability!.id)}
                      onMouseLeave={() => setHoveredCap(null)}
                    >
                      {chunk.rows.map((item) => (
                        <PluginListRow
                          key={item.entry.name}
                          item={item}
                          spec={detail?.dependencies[item.entry.name]}
                          chip={runtimeChipFor(item.entry.name, liveEntries)}
                          isPending={item.entry.name in pendingToggles}
                          rowBusy={
                            opBusy === `remove:${item.entry.name}` ||
                            opBusy === `update:${item.entry.name}` ||
                            opBusy === `toggle:${item.entry.name}`
                          }
                          opBusy={opBusy !== null}
                          latest={updateMap?.[item.entry.name]}
                          onToggle={() => toggleDisabled(item.entry.name)}
                          onUpdate={() => runOp("update", item.entry.name)}
                          onRemove={() => requestRemove(item.entry.name)}
                          onVersionPick={() =>
                            openVersionPick(
                              item.entry.name,
                              item.entry.installed_version ?? "",
                              updateMap?.[item.entry.name] ?? "",
                            )
                          }
                          onGoToggle={
                            item.capabilityAnchor
                              ? () => goToggleCapability(item.capability!)
                              : undefined
                          }
                          groupHovered={hoveredCap === chunk.capability!.id}
                          chipText={chipText}
                          chipTone={chipTone}
                          t={t}
                        />
                      ))}
                    </div>
                  ) : (
                    <PluginListRow
                      key={chunk.rows[0].entry.name}
                      item={chunk.rows[0]}
                      spec={detail?.dependencies[chunk.rows[0].entry.name]}
                      chip={runtimeChipFor(chunk.rows[0].entry.name, liveEntries)}
                      isPending={chunk.rows[0].entry.name in pendingToggles}
                      rowBusy={
                        opBusy === `remove:${chunk.rows[0].entry.name}` ||
                        opBusy === `update:${chunk.rows[0].entry.name}` ||
                        opBusy === `toggle:${chunk.rows[0].entry.name}`
                      }
                      opBusy={opBusy !== null}
                      latest={updateMap?.[chunk.rows[0].entry.name]}
                      onToggle={() => toggleDisabled(chunk.rows[0].entry.name)}
                      onUpdate={() => runOp("update", chunk.rows[0].entry.name)}
                      onRemove={() => requestRemove(chunk.rows[0].entry.name)}
                      onVersionPick={() =>
                        openVersionPick(
                          chunk.rows[0].entry.name,
                          chunk.rows[0].entry.installed_version ?? "",
                          updateMap?.[chunk.rows[0].entry.name] ?? "",
                        )
                      }
                      chipText={chipText}
                      chipTone={chipTone}
                      t={t}
                    />
                  ),
                )}
              </div>
            )}
          </div>
        )}

        {/* ================= Tab 1.5: 实验能力（ADR-0028，自插件中心迁入） ================= */}
        {tab === "caps" && (
          <ExperimentalCapabilities
            profile={name}
            caps={curatedCaps}
            capsLoading={capsLoading}
            capsError={capsError}
            onRefreshCaps={() => loadCaps(name)}
            onNotice={onNotice}
            onRestart={onRestart}
            onChanged={reload}
            focus={capFocus}
          />
        )}

        {/* ================= Tab 3: MCP 扩展服务器 ================= */}
        {tab === "mcp" && (
          <McpManager
            profileName={name}
            patchYaml={detail?.patch_yaml ?? null}
            onNotice={onNotice}
          />
        )}

        {/* ================= Tab 4: Patch YAML 原文视窗 ================= */}
        {tab === "patch" && (
          <div className="space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-faint text-xs">{t.profiles.rawYamlHint}</span>
              {detail?.patch_yaml && (
                <Button
                  size="sm"
                  variant="outline"
                  onClick={copyYaml}
                  className="gap-1.5 text-xs"
                >
                  {copiedYaml ? (
                    <Check className="size-3.5 text-ok" />
                  ) : (
                    <Copy className="size-3.5" />
                  )}
                  {copiedYaml ? t.console.copied : t.profiles.copyYaml}
                </Button>
              )}
            </div>

            {detail?.patch_yaml ? (
              /* 只读视窗与编辑面同一套（2026-09-18）：行号 / 折叠 / 高亮 / ⌘F 查找，
                 封顶口径沿用旧 pre 的 max-h-[420px]（≈21 行）。 */
              <YamlEditor value={detail.patch_yaml} rows={6} maxRows={21} readOnly />
            ) : (
              <div className="rounded-xl border border-dashed border-line bg-bg p-8 text-center text-xs text-faint">
                {t.profiles.detailPatchNone}
              </div>
            )}
          </div>
        )}
      </div>

      {/* 从其他 profile 导入选择器 */}
      <PluginImportPickerDialog
        target={name ?? ""}
        open={importOpen && name !== null}
        onClose={() => setImportOpen(false)}
        onDone={(ok, fail) => {
          onNotice(t.profiles.importDone(ok, fail), ok > 0 ? "ok" : "warn")
          reload()
        }}
      />

      {/* 版本选择弹窗 */}
      <Dialog open={versionPick !== null} onOpenChange={(o) => !o && setVersionPick(null)}>
        <DialogContent className="flex max-h-[calc(100dvh-4rem)] flex-col sm:max-w-md">
          <DialogHeader>
            <DialogTitle className="text-sm font-semibold">
              {versionPick ? t.profiles.pickVersionTitle(versionPick.pkg) : ""}
            </DialogTitle>
            <DialogDescription className="sr-only">
              {versionPick ? t.profiles.pickVersionTitle(versionPick.pkg) : ""}
            </DialogDescription>
          </DialogHeader>

          {versionsError && (
            <div className="rounded-lg bg-warn-soft px-3 py-2 text-xs text-warn">
              {versionsError}
            </div>
          )}

          {versionPick?.items === null && !versionsError && (
            <div className="text-faint py-6 text-center text-xs">
              <LoaderCircle className="mx-auto mb-2 size-4 animate-spin text-brand-deep" />
              {t.profiles.busyShort}
            </div>
          )}

          {versionPick?.items && (
            <div className="min-h-0 flex-1 overflow-y-auto pr-1">
              <div className="divide-y divide-line rounded-lg border border-line bg-bg">
                {versionPick.items.map((v) => {
                  const isLatest = v === versionPick.latest
                  const isCurrent = v === versionPick.current
                  return (
                    <button
                      key={v}
                      type="button"
                      disabled={opBusy !== null}
                      onClick={() => installVersion(`${versionPick.pkg}@${v}`)}
                      className="hover:bg-wash flex w-full items-baseline gap-2 px-3 py-2 text-left transition-colors disabled:opacity-40"
                    >
                      <span className="text-ink font-mono text-xs">{v}</span>
                      {isLatest && (
                        <span className="bg-ok-soft text-ok rounded-md px-1.5 text-meta leading-none">
                          {t.profiles.versionLatest}
                        </span>
                      )}
                      {isCurrent && (
                        <span className="border border-line text-faint ml-auto rounded-md px-1.5 text-meta leading-none">
                          {t.profiles.versionCurrent}
                        </span>
                      )}
                    </button>
                  )
                })}
              </div>
            </div>
          )}

          <DialogFooter>
            <Button variant="outline" onClick={() => setVersionPick(null)}>
              {t.profiles.pluginInstallCancel}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* 卸载确认（U9）：不可撤销，逐条列出影响面后再执行 */}
      <ConfirmDialog
        open={confirmRemove !== null}
        title={confirmRemove ? t.profiles.pluginUninstallConfirm(confirmRemove) : ""}
        points={t.profiles.pluginUninstallPoints}
        confirmLabel={t.profiles.pluginUninstall}
        cancelLabel={t.confirm.cancel}
        onConfirm={() => {
          const pkg = confirmRemove
          setConfirmRemove(null)
          if (pkg) runOp("remove", pkg)
        }}
        onClose={() => setConfirmRemove(null)}
      />
    </div>
  )
}

// ─────────────────────────────────────────────────────────────────────────────
// 三类合一的清单行（2026-09-20，ADR-0028 第二批）
//
// 一行 = 一个包（层 / 依赖只出现一次），行内三枚正交标记：
//   · 「层 N」——组合序（已退役的「底座组合」tab 的唯一独有信息，随合表迁到这里）；
//   · 主标记三选一——「实验性 · <能力>」/「内置」/「第三方」；
//   · dsh 自带的实验层（Agent Teams 两层）在主标记之外**再补一枚「内置」**。
//
// 控制面按主标记分流，这是**单一入口**的落点（ADR-0020 §2.8 / ADR-0028）：
//   · 第三方：开关 + 更新 + 卸载（本面板的传统控制面，原样保留）；
//   · 实验性：**不给开关/卸载**——那个能力的开关与移除只在「实验能力」面板
//     （卸载还必须连带清挂载行，否则悬空行让 dsh 起不来），这里只给「去开关」跳转；
//   · 内置：不给任何控制（随 dsh 自带、不可卸载；开关在 dsh 自己的插件页）。
// ─────────────────────────────────────────────────────────────────────────────
function PluginListRow({
  item,
  spec,
  chip,
  isPending,
  rowBusy,
  opBusy,
  latest,
  onToggle,
  onUpdate,
  onRemove,
  onVersionPick,
  onGoToggle,
  groupHovered,
  chipText,
  chipTone,
  t,
}: {
  item: MergedPluginRow
  /** 该包在 dependencies 里的声明值（层没有）；仅作展示兜底。 */
  spec?: string
  chip: RuntimeChip | null
  isPending: boolean
  rowBusy: boolean
  /** 面板级 busy（任一操作在途）——关掉本行的动作入口。 */
  opBusy: boolean
  latest?: string
  onToggle: () => void
  onUpdate: () => void
  onRemove: () => void
  onVersionPick: () => void
  /** 实验性 anchor 行的「去开关」跳转（非 anchor 行为 undefined——同一能力一个入口）。 */
  onGoToggle?: () => void
  /** 所在能力组正被 hover（整组联动高亮——基座/provider 从属关系的交互呈现）。 */
  groupHovered?: boolean
  chipText: (chip: RuntimeChip) => string
  chipTone: (chip: RuntimeChip) => string
  t: ReturnType<typeof useI18n>["t"]
}) {
  const p = item.entry
  const shellDisabled = item.row?.shell_disabled ?? false
  const hasRow = item.row !== null
  // 只有第三方行保留开关/更新/卸载（实验性与内置的控制面见上方注释）。
  const manageable = item.kindTag === "thirdParty"
  const isBase = p.name === "@deepseek-ai/dsh-base"
  const isWebApp = p.name === "@deepseek-ai/dsh-web-app"
  // 兜底说明只给**dsh 安装提供**的行（模板层 / optional / 桌面包——判定见
  // lib/pluginCatalog.ts 的模型说明）。只有确实认识的两层才具体说，其余给中性说明——
  // **用户装的层与实验依赖不能落进这句**：说它们"随 dsh 安装自带"就是 2026-09-17
  // 那种张冠李戴（当时 agent-team 两层被写成「Web 界面渲染器」的同族错误）。
  const fallbackDesc = isBase
    ? t.profiles.bundleDescBase
    : isWebApp
      ? t.profiles.bundleDescWebApp
      : t.profiles.bundleDescShipped
  const desc = p.description ?? spec ?? (item.dshProvided ? fallbackDesc : null)
  // 版本：实读 > 安装提供（无 node_modules 可读，版本锚在安装目录）> 未安装
  const versionText =
    p.installed_version ??
    (item.dshProvided
      ? t.profiles.pluginWithDsh
      : spec
        ? t.profiles.pluginNotInstalled
        : "")

  return (
    <div
      className={`group flex items-center justify-between gap-3 p-3.5 transition-colors ${
        // 组内联动：hover 组内任一行，整组（基座 + provider）一起亮
        groupHovered ? "bg-wash/40" : "hover:bg-wash/30"
      } ${shellDisabled ? "bg-bg/40" : ""} ${
        // provider 行缩进挂在基座下（从属关系一眼可读）
        item.capabilityChild ? "ml-5 border-l-2 border-line/60" : ""
      }`}
    >
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <span
            className={`font-mono text-xs font-semibold ${
              // 停用行不划删除线：包名是标识符，划掉既读不动也误判为"已删除"；
              // 停用已由徽标 + 开关 + 行底色三重表达（批次 3 去掉第四重）。
              shellDisabled ? "text-faint" : "text-ink"
            }`}
            title={p.name}
          >
            {p.name}
          </span>

          <span className="font-mono text-xs text-faint">{versionText}</span>

          {/* 能力标记按组内位置分流（2026-09-20 真机两轮修订）：一个能力 = 基座 + 一个
              生效 provider，**从属一组而非两个并列插件**——基座是 anchor（能力本体，
              "Exclusive named registration"），provider 缩进其下。anchor 挂完整的
              「实验性 · <能力>」（+「基座」标说明它自身不提供工具）；provider 行只挂
              弱化「后端」标。 */}
          {item.capability && item.capabilityAnchor && (
            <span
              title={t.profiles.tagExperimentalHint(item.capability.label)}
              className="border-brand/25 bg-wash text-brand-deep inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-meta leading-none"
            >
              <Sparkles className="size-3" />
              {t.profiles.tagExperimental(item.capability.label)}
            </span>
          )}
          {item.capability && item.capabilityAnchor && item.capabilityRole === "shared" && (
            <span
              title={t.profiles.tagCapabilityBaseHint(item.capability.label)}
              className="border-line bg-bg text-faint inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-meta leading-none"
            >
              <Layers className="size-3" />
              {t.profiles.tagCapabilityBase(item.capability.label)}
            </span>
          )}
          {item.capabilityChild && (
            <span
              title={t.profiles.tagBackendHint}
              className="border-line bg-bg text-faint inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-meta leading-none"
            >
              <Puzzle className="size-3" />
              {t.profiles.tagBackend}
            </span>
          )}
          {item.kindTag === "builtin" || item.alsoBuiltin ? (
            <span
              title={t.profiles.tagBuiltinHint}
              className="border-line bg-line-soft text-dim rounded-md border px-1.5 py-0.5 text-meta leading-none"
            >
              {t.profiles.tagBuiltin}
            </span>
          ) : item.kindTag === "thirdParty" ? (
            <span
              title={t.profiles.tagThirdPartyHint}
              className="border-line bg-bg text-faint rounded-md border px-1.5 py-0.5 text-meta leading-none"
            >
              {t.profiles.tagThirdParty}
            </span>
          ) : null}

          {/* 升级提示（只给可控的第三方行：实验能力的版本归能力面板管） */}
          {manageable && latest && latest !== p.installed_version && (
            <button
              type="button"
              disabled={opBusy}
              onClick={onVersionPick}
              className="text-brand-deep hover:bg-wash inline-flex items-center gap-1 rounded-full border border-brand/30 bg-wash px-2 py-0.5 font-mono text-meta font-medium transition-colors"
            >
              <ArrowUpCircle className="size-3" />
              <span>{latest}</span>
            </button>
          )}

          {/* 运行态徽标（运行中的 dsh 视角；配置侧「已禁用」是另一个徽标，
              两者语义与配色都不同——2026-09-17 维护者实机提问后拆开） */}
          {!shellDisabled &&
            (isPending ? (
              <span
                title={t.profiles.chipHint.applying}
                className="bg-line-soft text-dim rounded-md px-1.5 py-0.5 text-meta leading-none"
              >
                {t.profiles.chip.applying}
              </span>
            ) : (
              chip && (
                <span
                  title={t.profiles.chipHint[chip.kind]}
                  className={`rounded-md px-1.5 py-0.5 text-meta leading-none ${chipTone(chip)}`}
                >
                  {chipText(chip)}
                </span>
              )
            ))}

          {shellDisabled && (
            <span
              title={t.profiles.pluginDisabledHint}
              className="border-line text-faint rounded-md border px-1.5 py-0.5 text-meta leading-none"
            >
              {t.profiles.pluginDisabled}
            </span>
          )}
        </div>

        {desc && (
          <p className="text-faint mt-1 truncate text-xs" title={desc}>
            {desc}
          </p>
        )}
      </div>

      {/* 右侧控制：按主标记分流（见上方注释：单一入口） */}
      <div className="flex shrink-0 items-center gap-2">
        {rowBusy ? (
          <LoaderCircle className="size-4 animate-spin text-brand-deep" />
        ) : (
          <>
            {manageable && hasRow && (
              <div className="flex items-center gap-1.5" title={t.profiles.pluginToggleHint}>
                <Switch
                  aria-label={`${p.name}：${
                    shellDisabled ? t.profiles.pluginEnable : t.profiles.pluginDisable
                  }`}
                  checked={!shellDisabled}
                  disabled={opBusy}
                  onCheckedChange={onToggle}
                />
              </div>
            )}

            {manageable && (
              <Button
                size="icon-sm"
                variant="ghost"
                title={t.profiles.pluginUpdate}
                disabled={opBusy}
                onClick={onUpdate}
              >
                <ArrowUpCircle className="size-3.5 text-faint" />
              </Button>
            )}

            {/* 批次 2：手搓 warn 底改 destructive-ghost——卸载不可撤销，
                语义与其余三处行内删除键统一（danger 只在 hover 出现）。 */}
            {manageable && (
              <Button
                size="icon-sm"
                variant="destructive-ghost"
                title={t.profiles.pluginUninstall}
                disabled={opBusy}
                onClick={onRemove}
              >
                <Trash2 className="size-3.5" />
              </Button>
            )}

            {/* 实验性行不持开关/卸载：跳去「实验能力」面板（唯一入口）。
                入口只在标识包行；基座行仅在该能力没有标识包在场（只装了基座 / 装一半）
                时才承担——同一能力不给两个跳转按钮。 */}
            {onGoToggle && item.capability && (
              <Button
                size="sm"
                variant="outline"
                aria-label={t.profiles.goToggleAria(item.capability.label)}
                title={t.profiles.goToggleHint(item.capability.label)}
                onClick={onGoToggle}
                className="gap-1 text-xs"
              >
                <Sparkles className="size-3.5" />
                {t.profiles.goToggle}
              </Button>
            )}
          </>
        )}
      </div>
    </div>
  )
}
