import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import {
  Boxes,
  Check,
  Code2,
  Copy,
  FlaskConical,
  LoaderCircle,
  Package,
  Play,
  Plus,
  Puzzle,
  RefreshCw,
  Search,
  ShieldCheck,
  TriangleAlert,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { useCopy } from "@/hooks/useCopy"
import { useI18n } from "@/stores/i18nStore"
import { runtimeChipFor, runtimeSummary } from "@/lib/profiles"
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
import { Tip } from "@/components/ui/info-tip"
import { Segmented } from "@/components/ui/segmented"
import { ConfirmDialog } from "@/components/ui/confirm-dialog"
import { YamlEditor } from "@/components/ui/yaml-editor"
import { PluginAddDialog } from "@/components/profiles/PluginAddDialog"
import { PluginListRow } from "@/components/profiles/pluginRows/PluginListRow"
import { SystemBaseSection } from "@/components/profiles/pluginRows/SystemBaseSection"
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
  materialized,
  isRunning,
  isSwitching = false,
  busy,
  onLaunch,
  onRestart,
  onNotice,
}: {
  name: string | null
  /** 该 profile 在盘上**是否已物化**（`list_profiles` 的同一枚标记，左栏也用它画虚线框）。
   *  未物化 = 目录还不存在（内置模板名首次启动 / 首次 plugin add 才会建）：此时详情、
   *  插件清单、行表、能力目录**四个读取全部必失败**，故整页不去取数、只给一句人话 +
   *  一个「启动」（2026-09-21 真机：原先四个失败里有一个漏成原始 OS 错误）。 */
  materialized: boolean
  isRunning: boolean
  isSwitching?: boolean
  busy: boolean
  onLaunch: () => void
  onRestart: () => void
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
  // 默认落在「社区」（2026-09-21 维护者裁定）：进插件列表最常想看的是"我自己装的
  // 插件"，而不是把系统内置与实验依赖一起铺出来。撤掉「全部」芯片后，"看全部"
  // 由**再点一次已选芯片**承担（回到 `all` = 不筛选）。
  const [kindFilter, setKindFilter] = useState<PluginKindFilter>("thirdParty")

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
  const [addDialogOpen, setAddDialogOpen] = useState(false)
  const [addDialogTab, setAddDialogTab] = useState<"market" | "import" | "custom">("market")

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
    setAddDialogOpen(false)
    setSearchQuery("")
    // 换 profile 也回到默认档（社区），与首次进入一致
    setKindFilter("thirdParty")
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
    // 未物化：目录还不存在，四个读取全都会失败——一条都不发（见 `materialized` 注释）。
    // 该标记由 false 转 true（用户启动了一次）时本效应会再跑一遍，届时正常取数。
    if (!materialized) return
    reload()
  }, [name, materialized, reload])

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
    if (!name || !materialized) return
    loadCaps(name)
  }, [name, materialized, loadCaps])

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
  /** 运行时条目汇总：**只在失败数 > 0 时**用于警示（口径见 lib/profiles.ts）。
   *  正常时的"运行 N"与插件列表的插件数不是一个量纲，不铺给用户看。 */
  const runtimeSum = useMemo(() => runtimeSummary(liveEntries), [liveEntries])
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

  /** **分类就绪闸门**（2026-09-21 真机报「进列表先闪一下全部插件，然后才收敛到社区」）。
   *
   *  行表（`listProfilePlugins`）、层栈（`getProfileDetail.bundles`）、能力目录
   *  （`listExperimentalCapabilities`）是三条独立的取数链，`mergePluginRows` 的
   *  `kindTag` 同时依赖三者：
   *    · 目录没到 → 实验包判不出归属，**降级成社区**；
   *    · 层栈没到 → dsh 自带的层判不出「安装提供」，**也降级成社区**。
   *  而首屏只等 `plugins`，于是"社区"档先渲染出一份**混着实验包与内置层**的全量列表，
   *  另两条链到达后行数骤减——用户看到的就是"闪一下全部"。
   *  （判据错在"渲染时机"，不是筛选逻辑：`matchesKindFilter` 一直是对的。）
   *
   *  行态（`getPluginRows`）同样算在内：`shell_disabled` 没到之前，**停用的行会被先画成
   *  启用**（开关默认 false），那是同一类"先给一个错的事实、再改回来"。
   *
   *  故：四条链都**落定**（成功或失败）之前不渲染列表。失败也必须放开——目录读不到时
   *  状态条已经如实说「实验性标记暂不可用」，此时按降级结果渲染是**已知**的近似，
   *  比让列表永远转圈好（三条链的 catch 都会把值落成 `[]`/`null`+error，不会卡住闸门）。 */
  const classifyReady =
    plugins !== null &&
    rows !== null &&
    (detail !== null || error !== null) &&
    (caps !== null || capsError !== null)

  // 用户区 / 系统底座区（方案一：主次分离）。**按行**分，不再按能力分组：
  // 实验性行在插件列表里只读平铺（操作唯一入口 = 「实验能力」tab，2026-09-21 维护者裁定）。
  const { userRows, systemRows } = useMemo(
    () => ({
      userRows: filteredRows.filter((r) => r.kindTag !== "builtin"),
      systemRows: filteredRows.filter((r) => r.kindTag === "builtin"),
    }),
    [filteredRows],
  )

  /** 行渲染（用户区与底座区共用一份 props 装配）。
   *  2026-09-21 三次修订：行只剩一种形态，故三个调用点收敛成一个——旧版把同一段
   *  props 抄了三遍（卡片内明细 / 独立行 / 底座行），任一处漏改就是"这一栏的开关
   *  没接上"的同族 bug。 */
  const renderRow = (item: MergedPluginRow) => (
    <PluginListRow
      key={item.entry.name}
      item={item}
      kindFilter={kindFilter}
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
      chipText={chipText}
      chipTone={chipTone}
      t={t}
      locale={activeLocale}
    />
  )

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
      {/* Profile 头部（2026-09-21 真机截图修订：**只留左栏没有的信息**）
          左栏选中行已经写着 profile 名 +「运行中」+「默认启动」三样，行尾 `···` 菜单里
          还有同一个「设为默认」动作——详情头再摆一遍，就是同一句话说两遍（维护者截图
          圈出的那块）。这里只留两个别处**拿不到**的信息：
            · 会话正在启动/重载（过渡态，左栏只显示结果不显示过程）；
            · 清单名 `package_name`（manifest 里的真实包名，与 profile 名可以不同）。
          行高也从两行压到一行，把纵向空间还给下面真正要看的内容。 */}
      <header className="border-b border-line bg-panel/90 px-5 py-3 backdrop-blur-xs">
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <h2
            className="text-ink truncate font-mono text-sm font-bold tracking-tight"
            title={name ?? undefined}
          >
            {name}
          </h2>
          {isSwitching && (
            <span className="bg-info-soft text-info border-info/30 inline-flex animate-pulse items-center gap-1 rounded-full border px-2 py-0.5 font-medium text-meta">
              <LoaderCircle className="text-info size-3 animate-spin" />
              <span>
                {isRunning ? t.profiles.reloadingWorkbench : t.profiles.launchingProfile}
              </span>
            </span>
          )}
          {detail ? (
            <span
              className="text-faint truncate font-mono text-micro"
              title={`${t.profiles.manifestNameLabel}${detail.package_name}`}
            >
              {detail.package_name}
            </span>
          ) : materialized ? (
            <span className="text-faint text-micro">{t.profiles.detailLoading}</span>
          ) : (
            /* 未物化：没有清单可读，此时写「正在加载配置档案…」是谎报（永远等不到） */
            <span className="border-line text-faint rounded-md border px-1.5 py-0.5 text-meta leading-none">
              {t.profiles.notMaterializedTag}
            </span>
          )}
        </div>

        {/* 顶部 Tab 切换（批次 3：第 6 份手搓分段器 → Segmented stretch；
            与顶栏导航同配方，靠"整宽等分"而非"更大更亮"区分层级）。
            2026-09-20（ADR-0028 第二批）：「底座组合」tab 退役——层栈并入「插件列表」
            （行内「层 N」序标承载组合序），本面板收敛为 插件列表 / 实验能力 / MCP /
            Patch 四段。 */}
        <Segmented<"plugins" | "caps" | "patch" | "mcp">
          className="mt-3"
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
            { value: "caps", icon: FlaskConical, label: t.profiles.tabCaps },
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

        {!materialized ? (
          /* 未物化 = 目录还不存在：四个 tab 一个都没有内容可读（插件清单 / 能力目录 /
             MCP 配置 / patch 原文全在目录里）。故不给四个各自报一次错，只给这一句人话
             + 唯一的出路（启动一次即物化）。
             2026-09-21 真机：原先插件列表 tab 显示详情读取失败的人话，实验能力 tab 却把
             `listExperimentalCapabilities` 的原始 OS 错误（`No such file or directory
             (os error 2)`）直接铺出来——同一件事两种口径，且第二种是给工程师看的。 */
          <div className="flex min-h-[320px] flex-col items-center justify-center rounded-2xl border border-dashed border-line bg-bg/40 p-8 text-center">
            <div className="bg-line-soft text-faint mb-3 flex size-12 items-center justify-center rounded-2xl">
              <Package className="size-6" />
            </div>
            <h2 className="text-ink text-sm font-semibold">
              {t.profiles.notMaterializedTitle(name ?? "")}
            </h2>
            <p className="text-dim mt-1 max-w-md text-xs leading-relaxed">
              {t.profiles.notMaterializedBody}
            </p>
            <Button size="sm" onClick={onLaunch} disabled={busy} className="mt-4 gap-1.5">
              <Play className="size-3.5" />
              <span>{t.profiles.launch}</span>
            </Button>
          </div>
        ) : (
          <>

          {/* ================= Tab 1: 插件列表（外挂插件控制台） ================= */}
          {tab === "plugins" && (
            <div className="space-y-3.5">
              {/* 插件紧凑控制栏：搜索 + 四类过滤 + 统一添加 + 检查更新。
                  2026-09-21 真机截图修订：旧写法给左侧块 `flex-1 min-w-[280px]`，
                  窄一点就先把筛选芯片压到逐字折行（「全部」→「全/部」）。现改为
                  「三段各自成块 + 弹性空隙」：宽窗口一行铺开，窄窗口整块换行，
                  **任何一块都不会被压扁**。 */}
              <div className="flex flex-wrap items-center gap-2">
                {/* 搜索框：固定舒适宽度，不再参与压缩 */}
                <div className="relative w-[190px] shrink-0">
                  <Search className="text-faint absolute inset-y-0 left-2.5 my-auto size-3.5" />
                  <input
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    placeholder={t.profiles.searchPluginsPlaceholder}
                    aria-label={t.profiles.searchPluginsPlaceholder}
                    className="border-line bg-bg text-ink placeholder:text-faint focus:border-brand w-full rounded-lg border py-1.5 pr-3 pl-8 font-mono text-xs shadow-2xs outline-none transition-colors"
                  />
                </div>

                  {/* 四类筛选（与三个行内标记同名；计数可重叠——dsh 自带的实验层同时计入「内置」与「实验性」） */}
                  {/* 三类筛选 chip（2026-09-21 维护者裁定）：**撤掉「全部」**——它是
                      其余三者之和，单独占一枚芯片纯属冗余；顺序按关心程度排
                      「社区 → 内置 → 实验性」。
                      撤掉「全部」后必须回答"那默认看什么"：答案是**全不选 = 不筛选**
                      （列表照常显示全部），点选某类即筛选，**再点一次取消**回到全部。
                      故这里用 `variant="filters"`（role=group + aria-pressed）而不是
                      tablist——tablist 在 a11y 上要求恒有选中项，"全不选"是非法状态。 */}
                  <Segmented<PluginKindFilter>
                    size="sm"
                    variant="filters"
                    ariaLabel={t.profiles.tabPlugins}
                    value={kindFilter}
                    onChange={(v) => setKindFilter(v === kindFilter ? "all" : v)}
                    options={[
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
                        value: "experimental",
                        icon: FlaskConical,
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

                {/* 弹性空隙：宽窗口把操作区推到最右，窄窗口先于任何一块被压缩 */}
                <div className="flex-1" />

                {/* 右侧主操作区（整块不压缩，窄窗口整体换行）。
                    2026-09-21 真机截图修订：**一个功能一个入口**。旧版并排摆着
                    「添加插件 / 从其他导入 / 安装插件 / 检查更新」四个按钮，但前三个
                    是同一件事的三个分身——「添加插件」弹窗里本来就有「市场 / 从其他
                    导入 / 自定义」三条路径。四个按钮里三个通同一个面板，是入口冲突
                    （维护者截图圈出的那排）。现在只留「添加插件」一个入口承载三条
                    路径，「检查更新」是独立功能、保留。 */}
                <div className="flex shrink-0 items-center gap-1.5">
                  <Button
                    size="sm"
                    disabled={opBusy !== null}
                    onClick={() => {
                      setAddDialogTab("market")
                      setAddDialogOpen(true)
                    }}
                    className="gap-1.5 text-xs font-semibold"
                  >
                    <Plus className="size-3.5" />
                    <span>{t.profiles.pluginAddBtn}</span>
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

              {/* 状态条：**只报需要行动的**信息。
                  2026-09-21 真机截图修订：旧写法在这里铺一句全量运行时统计
                  （截图实测「运行 145 · 停用 27」），而列表里一共只有 8 个插件——
                  两个数字当场互相矛盾，用户只会以为界面坏了。该统计的口径是
                  **dsh 运行时的全部 fiber 条目**（见 lib/profiles.ts::runtimeSummary），
                  不是插件数，放在插件列表里属于语义错位。
                  现在：正常时不铺汇总（会话态由页头「运行中」徽标讲、行态由每行的
                  运行态徽标讲），**只有加载失败才冒头**——那才是需要用户行动的事。 */}
              {(runtimeSum.failed > 0 || (checkState === "done" && checkMeta) || capsError) && (
                <div className="text-label text-faint flex flex-wrap items-center justify-between gap-2 px-1">
                  <div className="flex items-center gap-3">
                    {runtimeSum.failed > 0 && (
                      <span className="bg-warn-soft text-warn tabular-nums flex items-center gap-1.5 rounded-md px-2 py-0.5">
                        <TriangleAlert className="size-3.5 shrink-0" />
                        {t.profiles.runtimeFailedSummary(runtimeSum.failed)}
                      </span>
                    )}
                    {checkState === "done" && checkMeta && (
                      <span className="flex items-center gap-1.5">
                        <span>{t.profiles.updateChecked(checkMeta)}</span>
                        {updateMap && Object.keys(updateMap).length === 0 && (
                          <span className="text-ok ml-1 font-medium">· {t.profiles.allUpToDate}</span>
                        )}
                      </span>
                    )}
                  </div>

                  {capsError && (
                    <div className="bg-warn-soft text-warn flex items-center gap-1.5 rounded-md px-2 py-0.5">
                      <TriangleAlert className="size-3.5 shrink-0" />
                      <span>{t.profiles.capsTagUnavailable}</span>
                    </div>
                  )}
                </div>
              )}

              {/* 官方桌面运行时说明（方案三：大横幅 → Info Tip，长句只占一颗图标） */}
              {detail?.package_name === "@deepseek-ai/dsh-desktop-runtime" && (
                <div className="flex items-center gap-2 px-1 text-label">
                  <span className="font-mono text-xs font-semibold text-ink">
                    {t.profiles.desktopRuntimeName}
                  </span>
                  <span className="bg-wash text-brand-deep border border-brand/20 rounded-md px-1.5 py-0.5 text-meta font-medium">
                    {t.profiles.desktopRuntimeTag}
                  </span>
                  <Tip
                    label={t.profiles.pluginDesktopRuntimeToggle}
                    text={`${t.profiles.desktopRuntimeDescPrefix}@deepseek-ai/dsh-desktop-runtime${t.profiles.desktopRuntimeDescMid}desktop-packages/${t.profiles.desktopRuntimeDescSuffix} ${t.profiles.desktopRuntimeNote}`}
                    className="max-w-sm text-xs leading-relaxed"
                  />
                </div>
              )}

              {/* 三类合一的插件列表（ADR-0028 第二批：层栈在前、按组合序；其余按依赖序）。
                  2026-09-21 三次修订：**行平铺**——不再按能力包卡片（实验性行的操作唯一入口
                  是「实验能力」，卡片上的开关/换后端是第二套控制面，已退役）。 */}
              {!classifyReady ? (
                /* 分类未就绪：行表 / 行态 / 层栈 / 能力目录四条链任一未落定，kindTag 会降级成
                   「社区」、停用行会先画成启用——先渲染就是把「全部」闪给用户看
                   （见 `classifyReady` 注释）。 */
                <div className="text-faint py-12 text-center text-xs">
                  <LoaderCircle className="mx-auto mb-2 size-5 animate-spin text-brand-deep" />
                  {t.profiles.busyShort}
                </div>
              ) : merged.rows.length === 0 ? (
                <div className="rounded-xl border border-dashed border-line bg-bg p-8 text-center text-xs text-faint">
                  {t.profiles.detailEmptyDeps}
                </div>
              ) : filteredRows.length === 0 ? (
                <div className="border-line bg-bg text-faint flex flex-col items-center gap-2.5 rounded-xl border border-dashed p-8 text-center text-xs">
                  <span>
                    {searchQuery.trim() ? t.profiles.searchNoPlugin : t.profiles.listFilterEmpty}
                  </span>
                  {/* 默认档是「社区」，而新工作台常常一个社区插件都没有——空列表必须
                      给出路，否则用户以为插件列表坏了（2026-09-21）。
                      判据含 `merged.rows.length > 0`：真的一个插件都没装时，"显示全部"
                      也救不了，那是另一句文案（`detailEmptyDeps`）。 */}
                  {!searchQuery.trim() && merged.rows.length > 0 && (
                    <Button
                      size="sm"
                      variant="outline"
                      onClick={() => setKindFilter("all")}
                      className="gap-1 text-xs"
                    >
                      {t.profiles.listFilterShowAll(merged.rows.length)}
                    </Button>
                  )}
                </div>
              ) : (
                <div className="space-y-3">
                  {/* 实验性行只读：它们**不是**没做完的行，而是操作入口统一在「实验能力」。
                      不说清这一点，用户只会看到一行行没有开关的插件。
                      唯一入口 = 一个提示条 + 一个按钮（逐行挂「去管理」等于把刚收掉的
                      第二套入口又铺回 N 遍）。
                      判据排除 `alsoBuiltin`（dsh 安装自带的实验层）：它们**不在**「实验能力」
                      面板里（`dockCuratedCaps` 已过滤），指向那里就是把用户送进死胡同——
                      那类行的「内置」标自带正确出路（dsh 自己的插件页）。 */}
                  {filteredRows.some((r) => r.kindTag === "experimental" && !r.alsoBuiltin) && (
                    <div className="border-line bg-bg/60 text-dim flex flex-wrap items-center gap-x-2 gap-y-1.5 rounded-xl border border-dashed px-3 py-2 text-label">
                      <FlaskConical className="text-brand-deep size-3.5 shrink-0" />
                      <span className="min-w-0 flex-1">{t.profiles.experimentalReadOnlyNote}</span>
                      <Button
                        size="xs"
                        variant="outline"
                        onClick={() => setTab("caps")}
                        className="shrink-0 gap-1"
                      >
                        {t.profiles.experimentalManageEntry}
                      </Button>
                    </div>
                  )}

                  {/* 1. 用户扩展（方案一：用户能掌控的置顶） */}
                  {userRows.length > 0 && (
                    <div className="divide-line border-line bg-panel shadow-xs divide-y overflow-hidden rounded-xl border">
                      {userRows.map((item) => renderRow(item))}
                    </div>
                  )}

                  {/* 2. 系统预置底座（与用户扩展物理分开、**沉底**）。
                      2026-09-21 维护者裁定：区间那条头部（标题 + "不可变更" + 折叠）整体
                      退役——"内置"这层意思已由筛选芯片/当前 tab 承担，头部只是复述。 */}
                  {systemRows.length > 0 && (
                    <SystemBaseSection>
                      {systemRows.map((item) => renderRow(item))}
                    </SystemBaseSection>
                  )}
                </div>
              )}
            </div>
          )}

          {/* ================= Tab 1.5: 实验能力（ADR-0028，自插件中心迁入） =================
              2026-09-21 维护者裁定：本 tab 是**实验性插件的唯一操作入口**——插件列表里
              实验性行只读平铺，开关/换后端/移除/修复全部收在这里（`focus` 深链随能力
              卡片退役：入口只剩"切到这个 tab"，不再需要"选中某个能力"）。 */}
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
          </>
        )}
      </div>

      {/* 统一添加插件对话框：**添加插件的唯一入口**，内含三条路径
          （市场选购 / 从其他导入 / 自定义 spec）。
          2026-09-21 起它同时取代了旧的独立导入选择器（PluginImportPickerDialog）——
          那个组件与工具栏的「从其他导入」按钮一起退役：三条路径同住一个面板，
          不再有"同一个功能两个入口"。 */}
      <PluginAddDialog
        target={name ?? ""}
        open={addDialogOpen && name !== null}
        defaultTab={addDialogTab}
        installedPlugins={merged.rows.map((r) => r.entry.name)}
        onClose={() => setAddDialogOpen(false)}
        onDone={() => {
          reload()
          if (name) loadCaps(name)
        }}
        onNotice={onNotice}
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
              {t.profiles.detailClose}
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
