// ExperimentalCapabilities.tsx —— 「实验能力」开关面板（2026-09-16，ADR-0020 §7）。
//
// ## 为什么推倒重来（v1 → v2，2026-09-16）
//
// v1（`OfficialLab.tsx`）以**包**为呈现单位：同一能力的三个互斥后端成了三张等价卡片
// （用户看不出"现在用的是哪一个"）；唯一动作是"安装"，**关掉这件事在面板里根本不存在**；
// 状态只到"包装没装"。v2 确立的语义（仍然成立）：
//
// 1. **呈现单位 = 能力**；互斥后端 = 能力内的变体（换后端是选择，不是让用户理解三选一）。
// 2. **开关是真开关**：关闭 = 写行级 `disabled`（**不卸载**、秒级可逆）；移除是独立动作。
//    含 profile 层的变体不能纯行级关闭（层 patch 带副作用），此时关闭动作即移除并说明原因。
// 3. **状态如实**：`未启用 / 已启用 / 已停用 / 需要修复 / 后端冲突` 由后端算全，本组件不猜。
//
// ## 为什么第二次重做（v2 → v3，2026-09-17）
//
// 维护者验收 v2 的判词："进去占用的空间也太多了，一屏只能看到一个工具；点击详情还又加长
// 卡片内容。"（真机实测：1280×820 默认窗口下整页 **1897px**，四张卡 341/530/360/278px，
// 一屏只装得下 **1** 张。）
//
// 病根不是"卡片里字太多"，而是**把清单与详情塞进同一个纵向流**——于是详情只有两条路：
// 内联展开（卡片越长越长，点一次详情把第二张卡推出屏幕）或藏起来（用户看不到）。两条都错。
//
// v3 = **清单 / 详情分栏**（macOS 系统设置、VS Code 设置同款形态）：
//   · 左栏：一能力一行，**恒为紧凑态**（图标 · 名称 · 状态徽标 · 当前插件名 · 开关），
//     四项一屏全见；行的作用只有两个——看状态、切走/开。
//   · 右栏：**选中能力的常驻详情面**（插件与后端切换 · 前置 · 排障细节 · 移除）。
//     **没有"展开"这件事**：点左栏的行只是换右栏内容。量出来的边界要说清——
//     **左栏恒不动**（1280×820 下四行 66/66/66/52px），右栏内容多的能力（三后端那一项）
//     仍会让整页长出约 70px（820 → 889）并出现轻微滚动：右栏是流式列，不裁剪内容。
//   · 窄窗口（<1024，应用最小 960）降级为**下钻**：列表 → 详情 + 返回。
//
// 一并保留的既有裁定（都是真机/评审换来的，别删）：
//   · 插件名（官方 npm 包名）进第一阅读层，我们发明的「Web 档」这类名字一律不出现
//     （2026-09-17 裁定，见 `variantPackageRoles` / `variantDisplayName`）；
//   · 宿主前置缺失是**硬门**：开关禁用 + 原因用警示色原样展示（后端文案，前端不自造）；
//   · 品牌色只承担"选中 / 主操作"，状态色只承担状态（v2 曾把两者画成同一个点）；
//   · 用户可见文案里不得出现 markdown 标记（面板没有渲染器）；
//   · 动效克制：入场上浮 4px、换详情面淡入；`prefers-reduced-motion` 全关。

import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { motion, useReducedMotion } from "framer-motion"
import {
  ArrowLeft,
  BadgeCheck,
  Bot,
  Check,
  ChevronRight,
  Globe,
  Info,
  LoaderCircle,
  MonitorSmartphone,
  RotateCw,
  ShieldCheck,
  TriangleAlert,
  Wrench,
} from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { ConfirmDialog } from "@/components/ui/confirm-dialog"
import { Tip } from "@/components/ui/info-tip"
import { Progress } from "@/components/ui/progress"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Switch } from "@/components/ui/switch"
import { api } from "@/lib/tauri"
import {
  capabilityIO,
  commonPrerequisites,
  commonSharedPackages,
  offMeansRemove,
  planDisable,
  planRemove,
  planReplace,
  runCapabilityOps,
  variantDisplayName,
  variantPackageRoles,
  type CapabilityOp,
} from "@/lib/experimentalCapabilities"
import { useI18n } from "@/stores/i18nStore"
import type {
  Capability,
  CapabilityStep,
  CapabilityVariant,
  FailureKind,
  ProfileSummary,
} from "@/types/ipc"

/** 能力 id → 图标（呈现归前端；后端不给 UI 资源）。未知 id 有兜底，不崩。 */
const ICONS: Record<string, typeof Bot> = {
  "agent-team": Bot,
  "browser-use": Globe,
  "computer-use": MonitorSmartphone,
  "auto-review": ShieldCheck,
}

/** 详情面的 DOM id：左栏的行用 `aria-controls` 指向它（屏幕阅读器知道"这行控制谁"）。 */
const PANE_ID = "cap-pane"

/** 清单行的 DOM id（窄窗口"返回清单"时把焦点还给刚才那一行）。 */
const rowId = (capId: string) => `cap-row-${capId}`

/** 是否走**下钻**形态（<1024：应用最小窗口 960，此时清单与详情面互斥显示）。
 *  只用于焦点管理——版面切换本身由 Tailwind 的 `lg:` 类完成，不靠 JS 量宽度。 */
function isDrillLayout(): boolean {
  // 与 Tailwind v4 的 `lg` 同口径（默认 64rem）——写死 1024px 会在根字号非 16px 时
  // 与版面的断点分叉：清单已经 `display:none`、焦点却按"宽窗口"不搬。
  return typeof window !== "undefined" && !window.matchMedia("(min-width: 64rem)").matches
}

/** 状态色（图标底 / 徽标同源）：off 保持中性，不制造噪音。 */
function stateTone(state: Capability["state"] | CapabilityVariant["state"]): string {
  switch (state) {
    case "on":
      return "border-ok/30 bg-ok-soft text-ok"
    case "disabled":
      return "border-warn/30 bg-warn-soft text-warn"
    case "partial":
    case "conflict":
      return "border-danger/30 bg-danger-soft text-danger"
    default:
      return "border-line bg-wash text-dim"
  }
}

interface Props {
  refreshKey: number
  onNotice?: (message: string, tone?: "ok" | "warn") => void
  /** 重启该 Profile（复用 ProfileManager 的既有确认链）；缺省则只给文字提示。 */
  onRestart?: (profile: string) => void
  /// 本轮是否以安全模式启动（ADR-0025）：为真时面板顶部说明"这里的'已启用'指配置层，
  /// 本轮实际未生效"——否则它会与已装插件列表的运行态徽标自相矛盾。
}

/** 正在执行的一次动作（进度导轨的数据源）。
 *
 *  单槽是**有意的**：行写（写/删/停用挂载行）不入队，`plugins.rs` 侧也没有互斥，
 *  两条编排行交错会丢配置写入（界面报"已完成"，文件里那行没写上），同族两次 teardown
 *  还会打乱"先删行再卸包"的顺序（悬空行 = dsh 起不来）。所以 `run !== null` 期间
 *  **所有**动作入口都必须关掉——见下方 `busy` 的传法（2026-09-17 独立复核）。 */
interface RunState {
  capId: string
  ops: readonly CapabilityOp[]
  index: number
}

export function ExperimentalCapabilities({
  refreshKey,
  onNotice,
  onRestart,
}: Props) {
  const { t, activeLocale } = useI18n()
  const [profiles, setProfiles] = useState<ProfileSummary[]>([])
  const [profile, setProfile] = useState("")
  const [caps, setCaps] = useState<Capability[] | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const [run, setRun] = useState<RunState | null>(null)
  /** 失败详情按能力记，**并钉住发起失败的那个变体**：让用户在原处看到原因并可续跑。
   *  只按能力记会出错——用户换一下后端选择器再点「继续剩余步骤」，就会去操作另一个变体
   *  （2026-09-16 独立评审核出：那是"什么都没做还报成功"）。 */
  const [failures, setFailures] = useState<
    Record<string, { variantId: string; error: string; failureKind: FailureKind | null }>
  >({})
  /** 用户选中的变体（**只影响选择**，不触发动作）。 */
  const [picked, setPicked] = useState<Record<string, string>>({})
  /** 每次成功动作后置位：提示"重启后生效"。 */
  const [dirty, setDirty] = useState(false)
  /** 详情面正在看哪个能力（`null` = 用第一个）。点左栏的行 = 换详情面内容。 */
  const [selectedId, setSelectedId] = useState<string | null>(null)
  /** 窄窗口下的下钻态：为真时只显示详情面（列表被"返回"带走）。宽窗口无影响。 */
  const [drilled, setDrilled] = useState(false)

  // 待确认的动作（启用 / 移除都走确认；停用可逆，不拦）。
  const [pending, setPending] = useState<PendingAction | null>(null)

  useEffect(() => {
    void (async () => {
      const [list, active] = await Promise.all([
        api.listProfiles().catch(() => [] as ProfileSummary[]),
        api.getActiveProfile().catch(() => null),
      ])
      const usable = list.filter((p) => p.materialized)
      setProfiles(usable)
      // 只在**下拉里真有**这一项时才采用活动档：活动档可能是尚未物化的模板名
      // （无目录、后端读不了），选了它只会立刻报错。
      setProfile(
        (prev) =>
          prev ||
          (active && usable.some((p) => p.name === active) ? active : "") ||
          usable[0]?.name ||
          "",
      )
    })()
  }, [refreshKey])

  /** 最新一次回读的令牌：只接受**最后一次**请求的结果。
   *
   *  为什么需要（2026-09-17 独立复核）：在途动作的 `finally` 会用它**发起时**那个闭包里的
   *  profile 回读一遍。若期间用户换了档，那次回读会把**旧档**的清单写进 `caps`——下拉显示 B、
   *  清单却是 A 的事实，之后按 A 的 `installed/rowPresent` 算出的计划会装进 B。 */
  const loadSeq = useRef(0)
  const load = useCallback(async () => {
    if (!profile) return
    const seq = (loadSeq.current += 1)
    setLoading(true)
    setError(null)
    try {
      // 文案是**单语 payload**（按请求语言出品），所以 activeLocale 进依赖：
      // 切语言即重新拉取（`load` 身份变化 → 下方 effect 重跑）。
      const next = await api.listExperimentalCapabilities(profile, activeLocale)
      if (seq === loadSeq.current) setCaps(next)
    } catch (e) {
      if (seq === loadSeq.current) {
        setCaps(null)
        setError(`${t.market.capLoadFailed}${t.market.capValueSep}${String(e)}`)
      }
    } finally {
      if (seq === loadSeq.current) setLoading(false)
    }
  }, [profile, t, activeLocale])

  useEffect(() => {
    // 换档即清"选择"与"失败"：两者都绑定在具体档位上，留着会串味（选中态指向另一个
    // profile 的变体、失败详情挂在别的档上）。
    setPicked({})
    setFailures({})
    setSelectedId(null)
    setDrilled(false)
    // 换档必须连 `dirty` 一起清：它是**按档**的事实（哪个档的配置被改过），
    // 留着会让"立即重启"打在没改过的那个档上，真正被改的档永远不会被重启。
    setDirty(false)
    void load()
  }, [load])

  /** 当前选中变体：用户的显式选择 → **当前生效（或已就位）的变体** → 首个变体。
   *
   *  中间那一档是必需的（2026-09-16 独立评审核出）：不兜底到 `activeVariant` 的话，
   *  正在用 Chrome DevTools 的用户一刷新，卡片就退回选中 Playwright —— 开关、前置、
   *  详情全指向另一个后端，"当前是哪一个"又只能靠猜，正是 v1 的 D1 缺陷。 */
  const selectedVariant = useCallback(
    (cap: Capability): CapabilityVariant => {
      const id = picked[cap.id] ?? cap.activeVariant ?? undefined
      const found = id ? cap.variants.find((v) => v.id === id) : undefined
      if (found) return found
      // 兜底**跳过被前置门挡住的档**（2026-09-17 独立复核）：选它没有任何可用动作
      // （开关必然灰），于是"清单里那个开关"对桌面控制这类能力形同虚设——用户得先去
      // 右栏换一档才能开。落到第一个可用档上，清单的开关才是真的可用；全被挡才退回首个。
      return cap.variants.find((v) => !v.prerequisiteMissing && !v.subsumedBy) ?? cap.variants[0]
    },
    [picked],
  )

  /** 统一执行入口：跑计划 → 回读状态 → 记成败。 */
  const execute = useCallback(
    async (cap: Capability, variantId: string, ops: readonly CapabilityOp[]) => {
      // 动作一发起就把详情面切到该能力：进度与失败都在用户正看着的那一栏里出现
      // （v3 的分栏形态下，"就地呈现"= 选中它，而不是把卡片往下长）。
      setSelectedId(cap.id)
      setDrilled(true)
      if (isDrillLayout()) {
        requestAnimationFrame(() => document.getElementById(PANE_ID)?.focus())
      }
      if (ops.length === 0) {
        await load()
        return
      }
      setRun({ capId: cap.id, ops, index: 0 })
      setFailures((prev) => {
        const next = { ...prev }
        delete next[cap.id]
        return next
      })
      try {
        const result = await runCapabilityOps(capabilityIO, profile, ops, (p) =>
          setRun((r) => (r ? { ...r, index: p.index } : r)),
        )
        if (result.ok) {
          setDirty(true)
          onNotice?.(t.market.capDoneFor(cap.label), "ok")
        } else {
          // 失败**不回滚**：已完成的步处于一致态，就地保留原因供续跑。
          const detail = result.failedAt?.error ?? ""
          setFailures((prev) => ({
            ...prev,
            [cap.id]: {
              variantId,
              error: detail,
              failureKind: result.failedAt?.failureKind ?? null,
            },
          }))
          // 只有**真的改过什么**才提示重启：0/N 步就失败时文件一个字节都没动，
          // 此时报"配置已变更"是谎报，会把用户骗去重启一个没变的 Profile。
          if (result.completedOps > 0) setDirty(true)
          onNotice?.(t.market.capPartialFor(cap.label, result.completedOps, result.totalOps), "warn")
        }
      } finally {
        setRun(null)
        await load()
      }
    },
    [profile, load, onNotice, t],
  )

  // 开启 = 让位 + 装上：`planReplace` 自己判断有没有别的后端要拆（同族并存会激活失败），
  // 所以"换后端 / 冲突修复 / 首次开启"共用这一个入口，不会漏掉让位。
  const runEnable = (cap: Capability, variant: CapabilityVariant) => {
    void execute(cap, variant.id, planReplace(cap, variant.id))
  }

  const runDisable = (cap: Capability, variant: CapabilityVariant) => {
    void execute(cap, variant.id, planDisable(variant))
  }

  const handleSwitch = (cap: Capability, variant: CapabilityVariant, next: boolean) => {
    // dsh 自带的能力一律不代管（2026-09-17）：连计划都不该算——界面藏了按钮、动作还在跑
    // 就是"半吊子"。计划层的护栏见 `resolve_capabilities` 的 shipped_by_dsh。
    if (cap.shippedByDsh) return
    if (next) {
      // 已就位（被停用）或只差补几步行 → 免确认直接做：这正是"关而不卸"要换来的体验，
      // 也是"修复"该有的手感（要装新包才需要确认）。
      if (variant.state === "disabled" || variant.state === "partial") {
        runEnable(cap, variant)
        return
      }
      setPending({ kind: "enable", cap, variantId: variant.id })
      return
    }
    // 关闭：能纯行级关就秒关（可逆、不打断）；不能则等价于移除，走破坏性确认。
    if (variant.toggleOffSupported) {
      runDisable(cap, variant)
      return
    }
    setPending({ kind: "remove", cap, variantId: variant.id })
  }

  // 确认后**先关框再执行**：进度与失败都在详情面里就地呈现（模态不遮住它们）。
  // 失败不留在框里，故框没有 busy/error 状态需要维护。
  const confirmPending = async () => {
    if (!pending) return
    const variant = pending.cap.variants.find((v) => v.id === pending.variantId)
    if (!variant) {
      setPending(null)
      return
    }
    const { cap, kind } = pending
    const ops = kind === "enable" ? planReplace(cap, variant.id) : planRemove(cap, variant.id)
    setPending(null)
    await execute(cap, variant.id, ops)
  }

  const summary = useMemo(() => {
    const all = caps ?? []
    return {
      on: all.filter((c) => c.state === "on").length,
      off: all.filter((c) => c.state === "off").length,
      // 「需要处理」只数真的坏了/冲突的；「已停用」是**用户自己关的**（关而不卸），
      // 算进"需要处理"等于面板对自己造成的状态报警（2026-09-17 独立复核实测）。
      other: all.filter((c) => c.state === "partial" || c.state === "conflict").length,
      disabled: all.filter((c) => c.state === "disabled").length,
    }
  }, [caps])

  if (!profile) {
    return <p className="px-1 py-6 text-center text-label text-dim">{t.market.capPickProfile}</p>
  }

  // 详情面归属：显式选中 → 第一项。列表随 profile 变化时 selectedId 已被清空。
  const selected = caps?.find((c) => c.id === selectedId) ?? caps?.[0] ?? null
  /** 点清单行 = 换详情面内容。窄窗口下这一步会隐藏整个清单，所以焦点必须跟着搬到
   *  详情面（否则它留在一个 `display:none` 的按钮上，键盘用户从此失踪）。 */
  const select = (cap: Capability) => {
    setSelectedId(cap.id)
    setDrilled(true)
    if (isDrillLayout()) {
      requestAnimationFrame(() => document.getElementById(PANE_ID)?.focus())
    }
  }

  /** 窄窗口的「返回清单」：把焦点还给刚才那一行（下钻的对称动作）。 */
  const backToList = (capId: string | null) => {
    setDrilled(false)
    if (isDrillLayout() && capId) {
      requestAnimationFrame(() => document.getElementById(rowId(capId))?.focus())
    }
  }

  return (
    <div className="flex flex-col gap-2.5">
      {/* 页头压成两行：身份 + 目标 Profile 同一行（v2 这里是 4 行 + 一条汇总条）。
          官方来源说明（capOfficialNote）2026-09-19 起挂在官方徽标上：它仍然是页头
          的一部分（"这不是社区插件"的口径不能挪进详情面），只是不再平铺一段散文。 */}
      <header className="flex flex-wrap items-center gap-x-3 gap-y-2">
        <h2 className="text-lead font-semibold text-ink">{t.market.capTitle}</h2>
        <span className="flex items-center gap-1">
          <Badge
            variant="outline"
            className="h-4.5 shrink-0 gap-1 rounded-full border-brand/25 bg-wash px-2 text-meta font-normal text-brand-deep"
          >
            <BadgeCheck className="size-3" />
            {t.market.capOfficialBadge}
          </Badge>
          <Tip text={t.market.capOfficialNote} label={t.tip.ariaFor(t.market.capOfficialBadge)} />
        </span>
        <span className="min-w-0 flex-1 basis-56 text-label leading-relaxed text-dim">
          {t.market.capDesc}
        </span>
        {/* 用 div 而不是 label：可聚焦元素是 Radix 的 trigger（一个 button），
            它已经用 aria-label 报了同一个名字；再套一层 label 只会多一条重复的关联路径。 */}
        <div className="flex shrink-0 items-center gap-2 whitespace-nowrap text-meta text-faint">
          {t.market.capTargetProfile}
          <Select value={profile} onValueChange={setProfile} disabled={run !== null}>
            <SelectTrigger
              aria-label={t.market.capTargetProfile}
              className="h-7 min-w-[132px] rounded-md border-line bg-panel font-mono text-label"
            >
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {profiles.map((p) => (
                <SelectItem key={p.name} value={p.name}>
                  <span className="font-mono text-label text-ink">{p.name}</span>
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </header>

      {dirty && (
        <div className="flex flex-wrap items-center gap-2 rounded-lg border border-warn/30 bg-warn-soft px-3 py-1.5 text-label text-warn">
          <Info className="size-3.5 shrink-0" />
          <span>{t.market.capRestartHint}</span>
          {onRestart && (
            <Button
              size="xs"
              variant="outline"
              className="ml-auto gap-1"
              onClick={() => onRestart(profile)}
            >
              <RotateCw className="size-3" />
              {t.market.capRestartNow}
            </Button>
          )}
        </div>
      )}

      {loading && !caps && (
        <p className="px-1 py-6 text-center text-label text-dim">
          <LoaderCircle className="mx-auto size-4 animate-spin" />
        </p>
      )}

      {error && (
        <div className="flex flex-wrap items-center gap-2 rounded-lg border border-danger/30 bg-danger-soft px-3 py-1.5 text-label text-danger">
          <TriangleAlert className="size-3.5 shrink-0" />
          <span className="min-w-0 break-words">{error}</span>
          <Button
            size="xs"
            variant="outline"
            className="ml-auto"
            onClick={() => void load()}
          >
            {t.market.capReload}
          </Button>
        </div>
      )}

      {caps && (
        // 两栏：[清单 | 详情面]。窄窗口（<1024）退化为下钻——列表与详情面互斥显示。
        <div className="grid items-start gap-4 lg:grid-cols-[minmax(21rem,25rem)_minmax(0,1fr)]">
          <section
            aria-label={t.market.capListLabel}
            className={drilled ? "hidden lg:block" : ""}
          >
            <div className="mb-1.5 flex flex-wrap items-center gap-x-2 px-0.5 text-meta text-faint">
              <span className="font-medium tracking-wider">{t.market.capListLabel}</span>
              <span className="text-ink">{t.market.capSummary(summary.on, caps.length)}</span>
              {summary.other > 0 && (
                <span className="text-warn">{t.market.capSummaryNeedsWork(summary.other)}</span>
              )}
              {/* 「已停用」用中性色：它是**用户自己关的**（关而不卸），拿警示色等于
                  给一个正常状态报警（2026-09-17 独立复核）。 */}
              {summary.disabled > 0 && (
                <span className="text-dim">{t.market.capSummaryDisabled(summary.disabled)}</span>
              )}
              {summary.off > 0 && <span>{t.market.capSummaryOff(summary.off)}</span>}
            </div>
            <ul className="flex flex-col gap-1.5">
              {caps.map((cap, i) => (
                <CapabilityRow
                  key={cap.id}
                  index={i}
                  cap={cap}
                  variant={selectedVariant(cap)}
                  selected={selected?.id === cap.id}
                  running={run?.capId === cap.id}
                  busy={run !== null}
                  failed={Boolean(failures[cap.id])}
                  onSelect={select}
                  onToggle={handleSwitch}
                  t={t}
                />
              ))}
            </ul>
          </section>

          {/* 详情面：常驻，没有展开/收起。`key` = 换能力时重新入场（轻淡入，不动高度）。 */}
          <div className={drilled ? "" : "hidden lg:block"}>
            {selected && selected.shippedByDsh && (
              <ShippedPane
                cap={selected}
                onCleanup={(v) => setPending({ kind: "remove", cap: selected, variantId: v.id })}
                t={t}
              />
            )}
            {selected && !selected.shippedByDsh && (
              <CapabilityPane
                key={selected.id}
                cap={selected}
                variant={selectedVariant(selected)}
                run={run?.capId === selected.id ? run : null}
                busy={run !== null}
                failure={failures[selected.id]?.error ?? null}
                failureKind={failures[selected.id]?.failureKind ?? null}
                failedVariantId={failures[selected.id]?.variantId ?? null}
                onPick={(id) => setPicked((prev) => ({ ...prev, [selected.id]: id }))}
                onRepair={(v) => runEnable(selected, v)}
                onBack={() => backToList(selected.id)}
                // 续跑目标 = **失败时那个变体**（不是此刻选中的那个）：`planReplace` 会按
                // 回读到的真实状态补缺口，重跑同一个变体才是"继续剩余步骤"。
                onResume={(v) => {
                  const id = failures[selected.id]?.variantId
                  const target = id ? selected.variants.find((x) => x.id === id) : undefined
                  runEnable(selected, target ?? v)
                }}
                onRemove={(v) => setPending({ kind: "remove", cap: selected, variantId: v.id })}
                t={t}
              />
            )}
          </div>
        </div>
      )}

      <ConfirmDialog
        open={pending !== null}
        title={
          pending
            ? pending.kind === "enable"
              ? t.market.capConfirmTitle(pending.cap.label)
              : t.market.capRemoveTitle(pending.cap.label)
            : ""
        }
        note={
          pending?.kind === "enable"
            ? t.market.capConfirmNote
            : pendingRemoveIsLayer(pending)
              ? t.market.capRemoveNoteLayer
              : t.market.capRemoveNote
        }
        points={pending ? confirmPoints(pending, t) : undefined}
        confirmLabel={
          pending?.kind === "enable" ? t.market.capConfirmStart : t.market.capRemoveConfirm
        }
        cancelLabel={t.market.capCancel}
        tone={pending?.kind === "enable" ? "primary" : "danger"}
        onConfirm={() => void confirmPending()}
        onClose={() => setPending(null)}
      >
        {pending && <PendingPackages pending={pending} t={t} />}
      </ConfirmDialog>
    </div>
  )
}

/** 确认框里的"将要动到哪些包"清单 + 确切版本（ADR-0020 §2.4 的原始要求）。
 *
 * 版本单列一行而不是跟在每个包名后面：包名动辄 50+ 字符，两列并排会把名字从中间劈开
 * （`break-all` 的副作用），而"钉到哪个版本"其实是**一个**值——运行期版本对所有步相同。
 * 版本未检出时**不编**：直接显示该步的 `versionNotice`（"未钉版本，裸包名会按 latest 解析"）。 */
function PendingPackages({
  pending,
  t,
}: {
  pending: PendingAction
  t: ReturnType<typeof useI18n>["t"]
}) {
  const steps: CapabilityStep[] =
    pending.cap.variants.find((v) => v.id === pending.variantId)?.steps ?? []
  const versions = [...new Set(steps.map((s) => s.spec.slice(s.package.length + 1)))]
  const allPinned = steps.every((s) => s.spec.length > s.package.length + 1)
  const notice = steps.find((s) => s.versionNotice)?.versionNotice ?? null
  return (
    <div className="flex flex-col gap-1.5 rounded-lg border border-line bg-wash px-3 py-2">
      {steps.map((s) => (
        <div key={s.package} className="flex items-baseline gap-1.5 font-mono text-micro">
          <span className="shrink-0 text-faint">{s.ordinal}.</span>
          <span className="min-w-0 flex-1 break-all text-ink">{s.package}</span>
        </div>
      ))}
      {notice ? (
        // 降级告知（如"运行时版本未检出 → 未钉版本"）**优先**：它比版本号本身更重要，
        // 不能因为"版本恰好都钉上了"就被吞掉（2026-09-16 独立评审核出的吞提示路径）。
        <p className="border-t border-line/60 pt-1.5 text-micro text-warn">{notice}</p>
      ) : allPinned && versions.length === 1 ? (
        <p className="border-t border-line/60 pt-1.5 text-micro text-faint">
          {t.market.capPinned(`v${versions[0]}`)}
        </p>
      ) : null}
    </div>
  )
}

/** 待确认的动作（启用 / 移除都要过确认框；停用可逆，不拦）。 */
type PendingAction =
  | { kind: "enable"; cap: Capability; variantId: string }
  | { kind: "remove"; cap: Capability; variantId: string }

/** 该待移除动作是不是"层类能力关闭"（层类不能纯行级停用，关闭就是移除）。 */
function pendingRemoveIsLayer(pending: PendingAction | null): boolean {
  if (!pending || pending.kind !== "remove") return false
  const variant = pending.cap.variants.find((v) => v.id === pending.variantId)
  return variant ? offMeansRemove(variant) : false
}

/** 确认框的要点清单：启用给前置与替换警告，移除给不可逆影响面。 */
function confirmPoints(
  pending: PendingAction,
  t: ReturnType<typeof useI18n>["t"],
): string[] {
  const variant = pending.cap.variants.find((v) => v.id === pending.variantId)
  if (!variant) return []
  if (pending.kind === "enable") {
    const points = [...variant.prerequisites]
    if (pending.cap.activeVariant && pending.cap.activeVariant !== variant.id) {
      const from = pending.cap.variants.find((v) => v.id === pending.cap.activeVariant)
      points.push(t.market.capReplaceFrom(from?.label ?? ""))
    } else if (variant.displaced.length > 0) {
      // 没有"当前生效"的档、但别的档已经有包就位（**冲突态**或装了一半）：`planReplace`
      // 仍会先把它们拆掉。后端早把要拆的包算在 `displaced` 里了，这里如实说出来——
      // 否则用户在冲突态点"启用"只看到"将安装下列包"，不知道还会卸掉别的东西。
      points.push(t.market.capReplaceDisplaced(variant.displaced.join(" · ")))
    }
    return points.length > 0 ? points : [t.market.capNoPrereq]
  }
  // 数字要按**真会卸的包**数：`teardownOps` 只对 `installed` 的步发 remove，
  // 装到一半（partial）时报总步数是谎报（2026-09-17 独立复核）。
  const installed = variant.steps.filter((s) => s.installed).length
  return [
    ...(installed > 0 ? [t.market.capRemovePointPackages(installed)] : []),
    t.market.capRemovePointRows,
    ...(offMeansRemove(variant) ? [t.market.capRemovePointLayer] : []),
  ]
}

/** **清单行**：一能力一行，恒为紧凑态（图标 · 名称 · 状态 · 当前插件名 · 开关）。
 *
 * 这一行是 v3 的核心取舍：**只放"扫一眼就要知道"的东西**——这是什么、现在什么状态、
 * 由哪个插件提供、开还是关。价值描述、插件清单、前置、版本、行 id 全部归详情面；
 * 行里再塞任何一条，"一屏四项"就崩了（v2 的教训）。
 *
 * 开关在行里（而不是详情面）是**有意**的：这块面板的本职是"看一眼四项、随手开关"，
 * 把开关埋进右栏会让每次开关都多一次点击。它操作的是**该能力当前选中的变体**
 * （`selectedVariant`：用户显式选择 → 当前生效 → 首个**可用**档），行内的插件名就是它。 */
function CapabilityRow({
  cap,
  variant,
  selected,
  running,
  busy,
  failed,
  index,
  onSelect,
  onToggle,
  t,
}: {
  cap: Capability
  variant: CapabilityVariant
  /** 是否是详情面正在展示的那一项（选中 ≠ 状态，用品牌色表达）。 */
  selected: boolean
  /** **本能力**正在跑一次动作（给行内转圈用）。 */
  running: boolean
  /** **任意能力**正在跑一次动作 → 关掉所有动作入口（`run` 是单槽，见 `RunState`）。 */
  busy: boolean
  /** 该能力有未处理的失败（行内给一个警示标记，免得用户只看得到"需要修复"却找不到原因）。 */
  failed: boolean
  /** 序号：只用于入场错峰（≤6 行，延迟封顶，不做长尾）。 */
  index: number
  onSelect: (cap: Capability) => void
  onToggle: (cap: Capability, variant: CapabilityVariant, next: boolean) => void
  t: ReturnType<typeof useI18n>["t"]
}) {
  const reduceMotion = useReducedMotion()
  const Icon = ICONS[cap.id] ?? Bot
  const on = variant.state === "on"
  // 被超集档包含的档（如"自建档"之于"Web 档"）：不提供独立开关——它的包就是超集档的
  // 基础层，单独关掉会把超集档一起拆坏。真机暴露于 2026-09-16。
  // **用 Boolean() 而不是 `!== null`**：这两个字段在契约里是 `T | null`，但"字段缺省"
  // 会静默变成 `undefined`，而 `undefined !== null` 为真 —— 2026-09-17 真机渲染复盘就是
  // 被 dev mock 少写一个 `prerequisiteMissing` 撞出来的：四行全亮红灯、开关全灰。
  const subsumedBy = Boolean(variant.subsumedBy)
  // 宿主前置缺失（2026-09-16 真机事故）：开关必须**禁用**，原因原样展示。
  // 只做提示是不够的——这类包装上去的代价是"工作台起不来"，用户根本没有回退余地。
  const blocked = Boolean(variant.prerequisiteMissing)
  // 选中的变体 ≠ 当前生效的变体时（用户在详情面点了另一个后端但还没开）：行内给
  // 一个警示小图标 —— 此时"行里的插件名"与"状态徽标"说的是两件事，必须看得出区别。
  const activeVariant =
    cap.activeVariant !== null ? cap.variants.find((v) => v.id === cap.activeVariant) : undefined
  const misaligned = activeVariant !== undefined && activeVariant.id !== variant.id
  const name = variantDisplayName(cap, variant.id)
  /** 开关为什么不可用 / 行内的额外事实：只挂在 aria-describedby 上。
   *  图标（lucide）自带 `aria-hidden`，悬停 `title` 键盘也触发不了——不写这段，
   *  读屏用户只会听到"开关，已禁用"，听不到"缺 cua-driver"这类真正的原因 */
  const descId = `${rowId(cap.id)}-why`
  const why = [
    blocked ? variant.prerequisiteMissing : null,
    subsumedBy ? t.market.capSubsumedBy(name) : null,
    misaligned && activeVariant
      ? activeVariant.state === "on"
        ? t.market.capOtherActive(variantDisplayName(cap, activeVariant.id))
        : t.market.capOtherReady(variantDisplayName(cap, activeVariant.id))
      : null,
    failed ? t.market.capRowHasFailure : null,
  ].filter((x): x is string => Boolean(x))

  return (
    <motion.li
      initial={reduceMotion ? false : { opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{
        duration: 0.2,
        ease: "easeOut",
        delay: reduceMotion ? 0 : Math.min(index, 6) * 0.03,
      }}
      className={`flex items-center gap-2.5 rounded-lg border px-2.5 py-2 transition-colors ${
        selected ? "border-brand/40 bg-wash" : "border-line bg-panel hover:border-brand/25"
      }`}
    >
      <button
        id={rowId(cap.id)}
        type="button"
        aria-current={selected ? "true" : undefined}
        aria-controls={PANE_ID}
        onClick={() => onSelect(cap)}
        className="flex min-w-0 flex-1 items-center gap-2.5 rounded-lg text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand/40"
      >
        <span
          className={`flex size-7 shrink-0 items-center justify-center rounded-lg border ${stateTone(
            variant.state === "off" ? cap.state : variant.state,
          )}`}
        >
          <Icon className="size-3.5" />
        </span>
        <span className="min-w-0 flex-1">
          <span className="flex items-center gap-1.5">
            <span className="truncate text-label font-medium text-ink">{cap.label}</span>
            {cap.shippedByDsh ? (
              // 自带的徽标只说一次：原来"随 dsh 自带"与右侧"dsh 已内置"同时出现，
              // 一块行里两个徽标讲同一件事＝噪音（维护者一贯口径：多此一举）。
              <Badge
                variant="outline"
                className="h-4.5 shrink-0 gap-1 rounded-full border-brand/25 bg-wash px-1.5 text-micro font-normal text-brand-deep"
              >
                <BadgeCheck className="size-3" />
                {t.market.capStateShipped}
              </Badge>
            ) : (
              <StateBadge cap={cap} variant={variant} t={t} />
            )}
            {running && <LoaderCircle className="size-3 shrink-0 animate-spin text-brand-deep" />}
            {!running && failed && <TriangleAlert className="size-3 shrink-0 text-danger" />}
          </span>
          {/* 行内第二行只讲一件事：**哪个插件**在提供它。包名很长 → **折行完整显示**，
              不截断：它是唯一要对得上 `node_modules` / `cordis.patch.yml` 的字符串，
              截掉的正好是区分各档的尾部（`…cua-driver-mcp` 与 `…cua-driver-native`
              会截成同一个前缀）。折行只让个别行多 14px，左栏总高仍远低于右栏。
              被超集档包含 / 前置门挡住 / 与当前生效不一致，都只用一个小图标提示，
              不占高度——它们的完整说明在详情面里（且经 `aria-describedby` 可被读屏听到）。 */}
          <span className="mt-0.5 flex flex-wrap items-center gap-x-1.5">
            <span className="break-words font-mono text-meta leading-snug text-faint">
              {cap.shippedByDsh ? t.market.capShippedMeta : name}
            </span>
            {subsumedBy && <Check className="size-3 shrink-0 text-faint" />}
            {blocked && <TriangleAlert className="size-3 shrink-0 text-danger" />}
            {misaligned && activeVariant && (
              // 悬停提示 + `aria-describedby`（见 why）：行里的插件名是"选中的那一档"，
              // 徽标说的却是当前生效的那一档——这个区别不能只靠悬停。
              <span
                className="flex shrink-0"
                title={
                  activeVariant.state === "on"
                    ? t.market.capOtherActive(variantDisplayName(cap, activeVariant.id))
                    : t.market.capOtherReady(variantDisplayName(cap, activeVariant.id))
                }
              >
                <Info className="size-3 text-warn" />
              </span>
            )}
          </span>
        </span>
      </button>
      {/* dsh 自带的能力**没有开关**：dsh 自己的插件页才是它的开关处，
          我们在这里点一下只会（a）被 dsh 拒（not-removable）或（b）装出第二份同名包。 */}
      {cap.shippedByDsh ? (
        // 没有可操作的控件：这一行的右边只留"点进去看详情"的指示（它没有开关，
        // 因为开关在 dsh 自己的插件页里）。窄窗口下由下面那个箭头负责，不重复画。
        <ChevronRight className="hidden size-3.5 shrink-0 text-faint lg:block" />
      ) : (
      <Switch
        aria-label={t.market.capSwitchLabel(`${cap.label} · ${name}`)}
        aria-describedby={why.length > 0 ? descId : undefined}
        checked={on}
        disabled={busy || subsumedBy || blocked}
        onCheckedChange={(next) => onToggle(cap, variant, next)}
        className="shrink-0"
      />
      )}
      {why.length > 0 && (
        <span id={descId} className="sr-only">
          {why.join(t.market.capWhyJoin)}
        </span>
      )}
      {!cap.shippedByDsh && (
        <ChevronRight className="size-3.5 shrink-0 text-faint lg:hidden" />
      )}
    </motion.li>
  )
}

/** **详情面**：选中能力的常驻详情——插件与后端切换 → 前置 → 排障细节 → 移除。
 *
 * 常驻是关键：这里的东西**不再需要"点详情"才能看到**，也永远不会把清单挤下去
 * （v2 的病灶）。信息分组顺序 = 用户的问题顺序："能不能用（前置）→ 用哪个（插件）→
 * 怎么做到的 / 出问题怎么查（实现细节）→ 怎么撤（移除）"。
 *
 * 它是一列**不裁剪内容**的面板：选到内容多的能力（如三后端那一项）时，整页会比视口
 * 高出约 70px。这是有意的取舍——不引入 `max-h-[calc(100vh-…)]` 这类靠魔数对齐全站吸顶
 * 头高度、并且在短窗口下会和多出一层的内部滚动条打架的做法。
 *
 * 2026-09-17 版面规范（沿用仓库既有 token，不另造风格）：
 *   · 刻度：正文 note(13) / 次要 label(11) / 节标与元信息 meta(10) / 角标 micro(9)；
 *   · 圆角按角色：面 xl(14)、行与控制件 md(10)、状态徽标胶囊；
 *   · 品牌色只承担"选中 / 主操作"，状态色只承担状态；
 *   · 官方包名（`roles.primary`）就是后端名，我们发明的「Web 档」这类名字一律不出现。 */
function CapabilityPane({
  cap,
  variant,
  run,
  busy,
  failure,
  failureKind,
  failedVariantId,
  onPick,
  onRepair,
  onResume,
  onRemove,
  onBack,
  t,
}: {
  cap: Capability
  variant: CapabilityVariant
  run: RunState | null
  /** **任意能力**在跑 → 关掉详情面的动作入口（`run` 是单槽，两个能力并发会丢配置写入）。 */
  busy: boolean
  failure: string | null
  /** 失败分类（后端给）；`null` = 未知，按通用话术处理。 */
  failureKind: FailureKind | null
  /** 发起失败的那个变体（`null` = 无失败）。 */
  failedVariantId: string | null
  onPick: (id: string) => void
  onRepair: (variant: CapabilityVariant) => void
  onResume: (variant: CapabilityVariant) => void
  onRemove: (variant: CapabilityVariant) => void
  /** 窄窗口下钻态的返回（宽窗口不渲染这个按钮）。 */
  onBack: () => void
  t: ReturnType<typeof useI18n>["t"]
}) {
  const reduceMotion = useReducedMotion()
  const Icon = ICONS[cap.id] ?? Bot
  const multi = cap.variants.length > 1
  // 排障信息默认收起：它是"出问题时才看"的东西，铺在详情面里会让每次选能力都要多滚一屏。
  // **注意与 v2 的「详情」区别**：这里的展开只增长右栏，左栏清单纹丝不动——v2 的病灶
  // 是"详情内联展开把后面的卡片顶下去"，那才是被点名的问题。
  const [showImpl, setShowImpl] = useState(false)
  // 层类能力：关闭 = 移除。这条**必须在按下开关之前就看得见**，否则用户以为能秒关。
  // 但只在"真的就位"（on/disabled）时才成立：`toggle_off_supported` 对**没装齐**的档
  // 也是 false（分类要读包自己的 manifest，没装就不知道），若照它判断，一次失败的安装
  // 会让面板谎称"本档由 profile 层提供"——真机 2026-09-16 暴露。
  const offIsRemove = offMeansRemove(variant)
  // 被超集档包含的档不是"可切换的后端"：不给交互，免得用户以为点它能单独开关
  // （开关与移除统一由超集档控制，见 §⑤ 门禁）。
  const subsumedBy = variant.subsumedBy ? variantDisplayName(cap, variant.subsumedBy) : null
  const otherActive =
    cap.activeVariant !== null && cap.activeVariant !== variant.id
      ? cap.variants.find((v) => v.id === cap.activeVariant)
      : undefined
  const blocked = Boolean(variant.prerequisiteMissing)
  // 单选组的 tab 停靠点（见下 focusTabIndex）
  const selectableIds = cap.variants
    .filter((v) => !v.subsumedBy && !v.prerequisiteMissing)
    .map((v) => v.id)
  const tabbableId = selectableIds.includes(variant.id) ? variant.id : (selectableIds[0] ?? null)
  // 各档共有的前置 / 共用的基座包只讲一次（去掉 Chrome 前置与 dsh-browser-use 基座
  // 在三条变体行里各重复三遍的噪音——重复把"真正区分各档的那行"淹掉了）
  const commonPrereqs = commonPrerequisites(cap)
  const commonShared = commonSharedPackages(cap)
  // 变体行里只留"这一档额外带的"共用包：全体共有的那句已经在下面统一讲过一次。
  const extraShared = (shared: readonly string[]) =>
    shared.filter((p) => !commonShared.includes(p))

  return (
    <motion.section
      id={PANE_ID}
      tabIndex={-1}
      aria-label={t.market.capPaneLabel(cap.label)}
      initial={reduceMotion ? false : { opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.2, ease: "easeOut" }}
      className="flex flex-col rounded-xl border border-line bg-panel shadow-2xs"
    >
      {/* 身份 + 状态：一行读全"这是什么 / 现在怎样"。 */}
      <div className="flex items-start gap-3 border-b border-line/70 px-4 py-3">
        <span
          className={`flex size-8 shrink-0 items-center justify-center rounded-lg border ${stateTone(
            variant.state === "off" ? cap.state : variant.state,
          )}`}
        >
          <Icon className="size-4" />
        </span>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
            <h3 className="text-note font-semibold text-ink">{cap.label}</h3>
            <Badge
              variant="outline"
              className="h-4.5 shrink-0 rounded-full border-line bg-wash px-1.5 text-micro font-normal text-faint"
            >
              {t.market.capOfficialBadge}
            </Badge>
            <StateBadge cap={cap} variant={variant} t={t} />
          </div>
          <p className="mt-1 text-label leading-relaxed text-dim">{cap.summary}</p>
        </div>
        {/* 窄窗口：下钻态的唯一出口（宽窗口隐藏）。 */}
        <button
          type="button"
          onClick={onBack}
          className="flex shrink-0 items-center gap-1 rounded-lg border border-line bg-panel px-2 py-1 text-meta text-dim transition-colors hover:text-ink focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand/40 lg:hidden"
        >
          <ArrowLeft className="size-3" />
          {t.market.capBack}
        </button>
      </div>

      <div className="flex flex-col gap-3 px-4 py-3">
        {run && (
          <div className="rounded-md border border-brand/25 bg-wash px-3 py-2.5">
            <div className="flex items-center gap-2 text-label text-ink">
              <LoaderCircle className="size-3.5 animate-spin text-brand-deep" />
              <span>{t.market.capRunning(run.index + 1, run.ops.length)}</span>
            </div>
            <Progress
              value={run.ops.length === 0 ? 0 : (run.index / run.ops.length) * 100}
              className="mt-2 h-1"
            />
            <ol className="mt-2 flex flex-col gap-1">
              {run.ops.map((op, i) => (
                <li
                  key={`${op.kind}-${i}`}
                  className={`flex items-center gap-1.5 font-mono text-meta ${
                    i < run.index ? "text-ok" : i === run.index ? "text-ink" : "text-faint"
                  }`}
                >
                  {i < run.index ? (
                    <Check className="size-3 shrink-0" />
                  ) : i === run.index ? (
                    <LoaderCircle className="size-3 shrink-0 animate-spin" />
                  ) : (
                    <span className="inline-block size-3 shrink-0 text-center leading-none">·</span>
                  )}
                  <span className="min-w-0 break-all">{opLabel(op, t)}</span>
                </li>
              ))}
            </ol>
          </div>
        )}

        {failure && !run && (
          <FailureBlock
            failure={failure}
            failureKind={failureKind}
            // 失败的是**哪个档**要说清：用户可能已经切到另一个档看详情了。
            failedVariantLabel={
              failedVariantId && failedVariantId !== variant.id
                ? variantDisplayName(cap, failedVariantId)
                : null
            }
            onResume={() => onResume(variant)}
            t={t}
          />
        )}

        {/* 动作行只在**有事要做**时出现（没有它就整行不渲染，详情面因此更静） */}
        {!run &&
          (variant.state === "partial" ||
            cap.state === "conflict" ||
            variant.state === "disabled") && (
            <div className="flex flex-wrap items-center gap-2">
              {/* 与错误块里的「继续剩余步骤」是同一个动作 → 只在没有失败块时出现，避免两个按钮
                  指向同一件事（真机 2026-09-16 暴露）。
                  **前置门挡住时不给「修复」**（同日真机暴露）：修复=装包+写行，而这两个动作
                  后端都会拒绝——按钮点下去必失败，属于"假按钮"。真正的出路写在红字里
                  （先装 cua-driver / 换自包含档）。 */}
              {!subsumedBy &&
                !blocked &&
                !failure &&
                (variant.state === "partial" || cap.state === "conflict") && (
                  <Button
                    size="sm"
                    variant="outline"
                    className="h-6 gap-1 border-warn/40 px-2 text-meta text-warn"
                    onClick={() => onRepair(variant)}
                  >
                    <Wrench className="size-3" />
                    {t.market.capRepair}
                  </Button>
                )}
              {variant.state === "disabled" && (
                <span className="text-label text-ok">{t.market.capReadyToEnable}</span>
              )}
            </div>
          )}

        {/* 插件：这一档是哪几个包 + 兄弟档怎么切。行首即插件名（官方名），
            自己独有的包在前，共用的基座包随后如实标注。 */}
        <div>
          <span className="text-meta font-medium tracking-wider text-faint">
            {t.market.capPluginsLabel}
          </span>
          <div
            {...(multi
              ? {
                  role: "radiogroup",
                  "aria-label": t.market.capBackendLabel,
                  onKeyDown: (e: React.KeyboardEvent) => {
                    const ids = cap.variants
                      .filter((x) => !x.subsumedBy && !x.prerequisiteMissing)
                      .map((x) => x.id)
                    if (ids.length === 0) return
                    const cur = ids.indexOf(variant.id)
                    const delta =
                      e.key === "ArrowDown" || e.key === "ArrowRight"
                        ? 1
                        : e.key === "ArrowUp" || e.key === "ArrowLeft"
                          ? -1
                          : 0
                    const next =
                      delta !== 0
                        ? ids[(cur + delta + ids.length) % ids.length]
                        : e.key === "Home"
                          ? ids[0]
                          : e.key === "End"
                            ? ids[ids.length - 1]
                            : null
                    if (!next) return
                    e.preventDefault()
                    onPick(next)
                    // roving tabindex：焦点跟着选中项走，否则方向键之后键盘就掉队了
                    requestAnimationFrame(() =>
                      document.getElementById(`cap-radio-${cap.id}-${next}`)?.focus(),
                    )
                  },
                }
              : {})}
            className="mt-1.5 flex flex-col gap-1.5"
          >
            {cap.variants.map((v) => {
              const roles = variantPackageRoles(cap, v.id)
              const selected = v.id === variant.id
              const isActive = cap.activeVariant === v.id
              const vBlocked = v.prerequisiteMissing
              // 被包含的档（它的包就是超集档的基础层）不是"可切换的后端"：不给交互。
              // **被前置门挡住的档同样不可选**（2026-09-17 独立复核）：选它没有任何可用动作，
              // 却会把行开关的目标切成一个必然灰掉的档——"清单随手开关"的承诺被一次误点作废。
              const selectable = multi && !v.subsumedBy && !v.prerequisiteMissing
              // 单选组的键盘语义（roving tabindex + 方向键）：组的可访问名说"单选按钮组"，
              // 就必须真的能用方向键切换，且组内**只能**有 radio（见下方非可选档的渲染）。
              const radioId = `cap-radio-${cap.id}-${v.id}`
              // roving tabindex：组里**只有一个** tab 停靠点——选中的那一档；
              // 若选中的档不可选（全体被挡），停靠点落在第一个可选项上，
              // 保证这一组仍然能被键盘进入。
              const focusTabIndex = selectable && v.id === tabbableId ? 0 : -1
              const extras = v.prerequisites.filter((p) => !commonPrereqs.includes(p))
              const rowCls = [
                "flex w-full items-start gap-2.5 rounded-lg border px-3 py-2 text-left transition-colors",
                selected
                  ? "border-brand/40 bg-wash"
                  : selectable
                    ? "border-line bg-panel hover:border-brand/25 hover:bg-wash/60"
                    : "border-line bg-panel",
                selectable
                  ? "cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand/40"
                  : "",
              ].join(" ")
              const body = (
                <>
                  {/* 选中点（品牌色）与"正在生效"（状态色徽标）是两件事，分开表达 */}
                  <span aria-hidden className="mt-1 shrink-0">
                    <span
                      className={`block size-2.5 rounded-full border transition-colors ${
                        selectable
                          ? selected
                            ? "border-brand bg-brand"
                            : "border-line bg-panel"
                          : "border-line/70 bg-wash"
                      }`}
                    />
                  </span>
                  <span className="min-w-0 flex-1">
                    <span className="flex flex-wrap items-center gap-x-2 gap-y-1">
                      {roles.primary.map((pkg) => (
                        <span
                          key={pkg}
                          className="break-all font-mono text-label font-medium text-ink"
                        >
                          {pkg}
                        </span>
                      ))}
                      {/* 行内徽标只在多档时才需要（它回答"哪一档在跑"）；单档能力的面板头
                          已经说了同一件事，重复就是噪音。
                      **必须按变体自己的状态取词**：后端契约里 `activeVariant` 的含义是
                      "当前生效**或已就位但停用**的那一档"（`official_catalog.rs` 的
                      Disabled 分支），照抄「已启用」会在面板说「已停用」时自相矛盾
                      ——2026-09-17 版面复盘在 mock 数据上真抓到了这一处。 */}
                      {multi && isActive && (
                        <Badge
                          className={`h-4.5 rounded-full px-1.5 text-micro font-normal ${
                            v.state === "on"
                              ? "border-ok/30 bg-ok-soft text-ok"
                              : v.state === "disabled"
                                ? "border-warn/30 bg-warn-soft text-warn"
                                : "border-danger/30 bg-danger-soft text-danger"
                          }`}
                        >
                          {v.state === "on"
                            ? t.market.capStateOn
                            : v.state === "disabled"
                              ? t.market.capStateDisabled
                              : t.market.capStatePartial}
                        </Badge>
                      )}
                      {v.subsumedBy && (
                        <Badge
                          variant="outline"
                          className="h-4.5 rounded-full border-line bg-wash px-1.5 text-micro font-normal text-dim"
                        >
                          {t.market.capStateSubsumed}
                        </Badge>
                      )}
                    </span>
                    {v.note && (
                      <span className="mt-1 block text-label leading-relaxed text-dim">
                        {v.note}
                      </span>
                    )}
                    {extraShared(roles.shared).length > 0 && (
                      <span className="mt-1 block break-all font-mono text-meta text-faint">
                        {t.market.capAlsoInstalls(extraShared(roles.shared).join(" · "))}
                      </span>
                    )}
                    {extras.length > 0 && (
                      <span className="mt-1.5 flex flex-col gap-0.5">
                        {extras.map((p) => (
                          <span
                            key={p}
                            className="flex items-start gap-1.5 text-label leading-relaxed text-dim"
                          >
                            <Info className="mt-0.5 size-3 shrink-0 text-faint" />
                            <span>{p}</span>
                          </span>
                        ))}
                      </span>
                    )}
                    {vBlocked && (
                      <span className="mt-1.5 flex items-start gap-1.5 text-label leading-relaxed text-danger">
                        <TriangleAlert className="mt-0.5 size-3 shrink-0" />
                        <span id={`cap-radio-${cap.id}-${v.id}-why`}>{vBlocked}</span>
                      </span>
                    )}
                  </span>
                </>
              )
              return (
                <button
                  key={v.id}
                  id={radioId}
                  type="button"
                  role="radio"
                  aria-checked={selectable ? selected : false}
                  aria-disabled={!selectable || busy || undefined}
                  aria-label={roles.primary.join(" + ")}
                  aria-describedby={vBlocked ? `${radioId}-why` : undefined}
                  tabIndex={focusTabIndex}
                  disabled={selectable ? busy : false}
                  onClick={selectable ? () => onPick(v.id) : undefined}
                  className={rowCls}
                >
                  {body}
                </button>
              )
            })}
          </div>

          {commonShared.length > 0 && (
            <span className="mt-2 flex items-start gap-1.5 text-label text-dim">
              <Info className="mt-0.5 size-3 shrink-0 text-faint" />
              <span className="break-all font-mono text-meta text-faint">
                {t.market.capBasePackages(commonShared.join(" · "))}
              </span>
            </span>
          )}

          {commonPrereqs.length > 0 && (
            <span className="mt-2 flex flex-col gap-0.5">
              <span className="text-meta text-faint">{t.market.capPrereqCommon}</span>
              {commonPrereqs.map((p) => (
                <span
                  key={p}
                  className="flex items-start gap-1.5 text-label leading-relaxed text-dim"
                >
                  <Info className="mt-0.5 size-3 shrink-0 text-faint" />
                  <span>{p}</span>
                </span>
              ))}
            </span>
          )}

          {/* 同族替换与"包含关系"：说清开关会连带动到谁（名字一律用插件名）。 */}
          {subsumedBy ? (
            <p className="mt-2 text-label leading-relaxed text-faint">
              {t.market.capSubsumedBy(subsumedBy)}
            </p>
          ) : (
            otherActive && (
              <p className="mt-2 text-label leading-relaxed text-warn">
                {otherActive.state === "on"
                  ? t.market.capOtherActive(variantDisplayName(cap, otherActive.id))
                  : t.market.capOtherReady(variantDisplayName(cap, otherActive.id))}
              </p>
            )
          )}
          {offIsRemove && (
            <p className="mt-1 text-label leading-relaxed text-faint">{t.market.capOffIsRemove}</p>
          )}
        </div>

        {/* 实现细节：常驻（不再折叠）——它在详情面里，展开/收起都不会挤动左栏的清单。
            "启用后会发生什么"是用户视角，单独一节；逐包细节是排障视角，标为排障用。 */}
        <section className="border-t border-line/70 pt-3">
          <h4 className="text-meta font-semibold tracking-wider text-faint">
            {t.market.capUnlocks}
          </h4>
          <p className="mt-1 text-label leading-relaxed text-dim">{cap.unlocks}</p>
        </section>

        <section className="border-t border-line/70 pt-3">
          <button
            type="button"
            aria-expanded={showImpl}
            aria-controls={`${PANE_ID}-impl`}
            onClick={() => setShowImpl((v) => !v)}
            className="flex w-full items-center gap-1.5 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand/40"
          >
            {/* 用 span 而不是 h4：button 只允许 phrasing content，标题元素会被浏览器从
                按钮名里拆出来（读屏念不到完整按钮名）。视觉样式照旧。 */}
            <span className="text-meta font-semibold tracking-wider text-faint">
              {t.market.capImplTitle}
            </span>
            <span className="text-meta text-faint">{t.market.capImplPackages(variant.steps.length)}</span>
            <ChevronRight
              className={`ml-auto size-3 shrink-0 text-faint transition-transform ${
                showImpl ? "rotate-90" : ""
              }`}
            />
          </button>
          {/* 列表**常驻**、用 `hidden` 收起：`aria-controls` 指向的 id 必须一直存在
              （收起时引向不存在的 id 是无效引用）。 */}
          <ul
            id={`${PANE_ID}-impl`}
            hidden={!showImpl}
            className="mt-1.5 flex flex-col gap-2.5"
          >
            {variant.steps.map((s) => (
              <li key={s.package} className="text-label">
                <div className="flex flex-wrap items-baseline gap-x-1.5 font-mono text-meta">
                  <span className="text-faint">{s.ordinal}.</span>
                  <span className="break-all text-ink">{s.package}</span>
                  <span
                    className={
                      s.installed && s.rowPresent && !s.disabled
                        ? "text-ok"
                        : s.installed || s.rowPresent
                          ? "text-warn"
                          : "text-faint"
                    }
                  >
                    {stepStatus(s, t)}
                  </span>
                </div>
                <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1">
                  <Badge
                    variant="outline"
                    className="h-4.5 rounded-full border-line bg-panel px-1.5 text-micro font-normal text-dim"
                  >
                    {s.activation === "auto_bundle"
                      ? t.market.capActivationAuto
                      : t.market.capActivationInsert}
                  </Badge>
                  <span className="break-all font-mono text-meta text-faint">
                    {t.market.capPinned(s.spec)}
                    {s.activation === "insert_row" && ` · ${s.rowId}`}
                  </span>
                </div>
                {s.description ? (
                  <p className="mt-1 text-label leading-relaxed text-dim">
                    <span className="text-faint">{t.market.capOfficialDesc}</span>
                    {s.description}
                  </p>
                ) : (
                  <p className="mt-1 text-label text-faint">{t.market.capOfficialDescPending}</p>
                )}
                {s.versionNotice && (
                  <p className="mt-1 text-label leading-relaxed text-warn">{s.versionNotice}</p>
                )}
              </li>
            ))}
          </ul>
        </section>

        {!subsumedBy &&
          (variant.state === "on" ||
            variant.state === "disabled" ||
            variant.state === "partial") && (
            <section className="flex flex-wrap items-center justify-between gap-2 border-t border-line/70 pt-3">
              <span className="text-label leading-relaxed text-faint">
                {offMeansRemove(variant)
                  ? t.market.capRemoveExplainedLayer
                  : t.market.capRemoveExplainedSoft}
              </span>
              <Button
                size="sm"
                variant="outline"
                className="h-6 shrink-0 border-danger/40 px-2 text-meta text-danger"
                onClick={() => onRemove(variant)}
              >
                {t.market.capRemoveBtn}
              </Button>
            </section>
          )}
      </div>
    </motion.section>
  )
}

/** **dsh 自带能力的详情面**（2026-09-17 立）：只说明、不代管。
 *
 * 为什么单独一块而不是复用 `CapabilityPane`：一个"能力"由 dsh 自带之后，**语义变了**——
 * 它不再是"我们策展的一组互斥后端"，而是 **dsh 官方的若干独立插件**（Agent Teams 的宿主层与
 * Web 层在 dsh 的插件页里是两个各自可开关的 Beta 插件，Web 层依赖宿主层但不互斥）。
 * 硬塞进变体单选组会把"三选一"的模型套到一个并非如此的东西上。
 *
 * 三个事实必须都在这一屏里：① 它现在归 dsh 管、开关在哪；② 随 dsh 自带的是哪些包；
 * ③ 本 Profile 里是否还有我们早期装的副本（它会遮蔽自带的那一份），以及怎么清掉。 */
function ShippedPane({
  cap,
  onCleanup,
  t,
}: {
  cap: Capability
  /** 清理遗留副本（走既有的破坏性确认链，不新增路径）。 */
  onCleanup: (variant: CapabilityVariant) => void
  t: ReturnType<typeof useI18n>["t"]
}) {
  const reduceMotion = useReducedMotion()
  const Icon = ICONS[cap.id] ?? Bot
  // 一个包可能被多个变体共用：去重后按变体顺序展示（顺序仍是"宿主层在前"的语义）。
  const packages = [...new Set(cap.variants.flatMap((v) => v.steps.map((s) => s.package)))]
  // 清理目标 = **真的还装着东西**的那个变体（遗留副本可能只是一档）
  const legacyVariant =
    cap.variants.find((v) => v.steps.some((s) => s.installed || s.rowPresent || s.disabled)) ??
    cap.variants[0]

  return (
    <motion.section
      id={PANE_ID}
      tabIndex={-1}
      aria-label={t.market.capPaneLabel(cap.label)}
      initial={reduceMotion ? false : { opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.2, ease: "easeOut" }}
      className="flex flex-col rounded-xl border border-line bg-panel shadow-2xs"
    >
      <div className="flex items-start gap-3 border-b border-line/70 px-4 py-3">
        <span className="flex size-8 shrink-0 items-center justify-center rounded-lg border border-brand/25 bg-wash text-brand-deep">
          <Icon className="size-4" />
        </span>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
            <h3 className="text-note font-semibold text-ink">{cap.label}</h3>
            {/* 「指路」那段（开关在 dsh 自己的插件页）2026-09-19 起挂在状态徽标上：
                它回答的是"那我在哪儿开"，正是鼠标停在这个徽标上时想问的。 */}
            <span className="flex items-center gap-1">
              <Badge
                variant="outline"
                className="h-4.5 shrink-0 gap-1 rounded-full border-brand/25 bg-wash px-1.5 text-micro font-normal text-brand-deep"
              >
                <BadgeCheck className="size-3" />
                {t.market.capStateShipped}
              </Badge>
              <Tip text={t.market.capShippedNote} label={t.tip.ariaFor(t.market.capStateShipped)} />
            </span>
          </div>
          <p className="mt-1 text-label leading-relaxed text-dim">{cap.summary}</p>
        </div>
      </div>

      <div className="flex flex-col gap-3 px-4 py-3">
        <div>
          <span className="text-meta font-medium tracking-wider text-faint">
            {t.market.capShippedPackages}
          </span>
          <ul className="mt-1.5 flex flex-col gap-1">
            {packages.map((pkg) => (
              <li key={pkg} className="flex items-start gap-1.5">
                <Check className="mt-0.5 size-3 shrink-0 text-ok" />
                <span className="break-all font-mono text-label text-ink">{pkg}</span>
              </li>
            ))}
          </ul>
        </div>

        <section className="border-t border-line/70 pt-3">
          <span className="text-meta font-medium tracking-wider text-faint">
            {t.market.capUnlocks}
          </span>
          <p className="mt-1 text-label leading-relaxed text-dim">{cap.unlocks}</p>
        </section>

        {/* 遗留副本：只有真的还在时才出现——平时这一屏就只有上面三块。 */}
        {cap.legacyCopy && legacyVariant && (
          <section className="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-warn/30 bg-warn-soft px-3 py-2.5">
            <span className="min-w-0 flex-1 text-label leading-relaxed text-warn">
              {t.market.capLegacyCopy}
            </span>
            <Button
              size="sm"
              variant="outline"
              className="h-6 shrink-0 border-danger/40 px-2 text-meta text-danger"
              onClick={() => onCleanup(legacyVariant)}
            >
              {t.market.capLegacyCleanupBtn}
            </Button>
          </section>
        )}
      </div>
    </motion.section>
  )
}

/** 失败块：**一句人话** + 可折叠的原始输出 + 唯一的续跑入口。
 *
 * 真机 2026-09-16 暴露两件事：① 原始输出（registry URL + pnpm 调用链 + TLS 细节）上百字符，
 * 直接铺开会把"我该怎么办"淹没；② 失败块里的「继续剩余步骤」与底部「修复」是同一个动作，
 * 并列出现等于给用户两个按钮干同一件事（「修复」在有失败块时已不再渲染）。 */
function FailureBlock({
  failure,
  failureKind,
  failedVariantLabel,
  onResume,
  t,
}: {
  failure: string
  failureKind: FailureKind | null
  /** 发起失败的档名（用户已切到别的档时用来点名）；`null` = 就是当前档。 */
  failedVariantLabel: string | null
  onResume: () => void
  t: ReturnType<typeof useI18n>["t"]
}) {
  const [showRaw, setShowRaw] = useState(false)
  // 分类由**后端**给（`plugin_registry::classify_failure`）——前端不再写第二份正则。
  // 到这里时自动策略已试过两侧源（若换源有意义），故文案说的是"最终结果"。
  const hint =
    failureKind === "network"
      ? t.market.capFailNetwork
      : failureKind === "not_found"
        ? t.market.capFailNotFound
        : failureKind === "build_approval"
          ? t.market.capFailBuildApproval
          : t.market.capFailUnknown
  return (
    <div className="rounded-lg border border-danger/30 bg-danger-soft px-3 py-2.5">
      <p className="text-label font-semibold text-danger">
        {t.market.capFailed}
        {failedVariantLabel && t.market.capFailedOn(failedVariantLabel)}
      </p>
      <p className="mt-1 text-label leading-relaxed text-dim">{hint}</p>
      <div className="mt-2 flex flex-wrap items-center gap-2">
        <Button
          size="sm"
          className="h-6 px-2 text-micro"
          onClick={onResume}
        >
          {t.market.capResume}
        </Button>
        <span className="text-micro text-faint">{t.market.capQueueHint}</span>
        <button
          type="button"
          aria-expanded={showRaw}
          onClick={() => setShowRaw((v) => !v)}
          className="ml-auto flex items-center gap-0.5 text-micro text-danger hover:underline"
        >
          {t.market.capFailRawToggle}
          <ChevronRight className={`size-3 transition-transform ${showRaw ? "rotate-90" : ""}`} />
        </button>
      </div>
      {showRaw && (
        <p className="mt-1.5 max-h-40 overflow-auto break-words rounded-md bg-panel/60 px-2 py-1.5 font-mono text-micro text-danger">
          {failure}
        </p>
      )}
    </div>
  )
}

/** 状态徽标：五态各自一色（颜色只在状态族里取，不新造色）。 */
function StateBadge({
  cap,
  variant,
  t,
}: {
  cap: Capability
  variant: CapabilityVariant
  t: ReturnType<typeof useI18n>["t"]
}) {
  // 被超集档包含的档先判：它确实是 On，但"已包含"才是用户需要知道的那件事。
  if (variant.subsumedBy) {
    return (
      <Badge
        variant="outline"
        className="h-4.5 shrink-0 border-line bg-wash px-1.5 text-micro font-normal text-dim"
      >
        {t.market.capStateSubsumed}
      </Badge>
    )
  }
  if (cap.state === "conflict") {
    return (
      <Badge className="h-4.5 shrink-0 border-warn/40 bg-warn-soft px-1.5 text-micro font-normal text-warn">
        {t.market.capStateConflict}
      </Badge>
    )
  }
  const map = {
    on: { text: t.market.capStateOn, cls: "border-ok/30 bg-ok-soft text-ok" },
    disabled: { text: t.market.capStateDisabled, cls: "border-warn/30 bg-warn-soft text-warn" },
    partial: { text: t.market.capStatePartial, cls: "border-danger/30 bg-danger-soft text-danger" },
    off: { text: t.market.capStateOff, cls: "border-line bg-wash text-faint" },
  } as const
  // 徽标说**能力**的实话，不说"当前选中的后端"的状态（2026-09-16 真机暴露）：
  // 桌面控制里「随包自带运行时」正在生效、用户点了被前置门挡住的「复用已装 cua-driver」，
  // 旧写法照**选中档**渲染 → 整个面板报「需要修复」，而能力其实是好的（原生档在跑）。
  // 选中档自身的状态由插件行上的徽标 / ⚠ 表达。
  // （`conflict` 已在上面的早退分支拦掉，TS 据此也把类型收窄了，故可直接索引。）
  const shown: keyof typeof map = cap.state
  return (
    <Badge
      variant="outline"
      className={`h-4.5 shrink-0 px-1.5 text-micro font-normal ${map[shown].cls}`}
    >
      {map[shown].text}
    </Badge>
  )
}

/** 一步的执行文案（进度导轨里逐行显示）。 */
function opLabel(op: CapabilityOp, t: ReturnType<typeof useI18n>["t"]): string {
  switch (op.kind) {
    case "install":
      return t.market.capOpInstall(op.package)
    case "remove":
      return t.market.capOpRemove(op.package)
    case "ensureRow":
      return t.market.capOpEnsureRow(op.package)
    case "deleteRow":
      return t.market.capOpDeleteRow(op.package)
    case "setDisabled":
      return op.disabled ? t.market.capOpDisableRow : t.market.capOpEnableRow
  }
}

/** 一步的落位状态（详情里逐包显示）。 */
function stepStatus(step: CapabilityStep, t: ReturnType<typeof useI18n>["t"]): string {
  if (step.disabled) return t.market.capStepDisabled
  if (step.installed && step.rowPresent) return t.market.capStepLive
  if (step.installed && !step.rowPresent) return t.market.capStepRowMissing
  if (!step.installed && step.rowPresent) return t.market.capStepPackageMissing
  return t.market.capStepNotInstalled
}
