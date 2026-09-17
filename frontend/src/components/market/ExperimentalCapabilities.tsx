// ExperimentalCapabilities.tsx —— 「实验能力」开关面板（2026-09-16，ADR-0020 §7）。
//
// ## 为什么推倒重来
//
// v1（`OfficialLab.tsx`）以**包**为呈现单位，结果是：同一能力的三个互斥后端成了三张
// 等价卡片（用户看不出"现在用的是哪一个"）；唯一动作是"安装"，**关掉这件事在面板里
// 根本不存在**；状态只到"包装没装"（挂载行缺没缺、有没有被停用都看不见）；而
// "钉版本 / 需写入挂载行 / 由 dsh 自动激活"这些维护者信息占着第一阅读层。
//
// ## 现在的语义
//
// 1. **呈现单位 = 能力**：四项（多智能体协同 / 浏览器操作 / 桌面控制 / 自动安全审查），
//    互斥后端降级为**能力内的变体**——"换后端"成了同一张卡内的一个选择，而不是让用户
//    去理解"三选一"。
// 2. **开关是真开关**：关闭 = 写行级 `disabled`（**不卸载**、秒级可逆）；移除是独立的
//    次要动作（清停用桩 → 删壳写的行 → 逆序卸包）。含 profile 层的变体不能纯行级关闭
//    （层 patch 带副作用），此时开关的关闭动作就是移除，并在文案里说明原因。
// 3. **状态如实**：`未启用 / 已启用 / 已停用 / 需要修复 / 后端冲突` 由后端算全
//    （「包 × 行 × disabled」的函数），本组件不猜。
// 4. **实现细节折叠**：包名 / 钉版本 / 激活方式 / 行 id 收进「详情」，第一阅读层只留
//    价值、前置与状态。

import { useCallback, useEffect, useMemo, useState } from "react"
import { AnimatePresence, motion, useReducedMotion } from "framer-motion"
import {
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

interface Props {
  refreshKey: number
  onNotice?: (message: string, tone?: "ok" | "warn") => void
  /** 重启该 Profile（复用 ProfileManager 的既有确认链）；缺省则只给文字提示。 */
  onRestart?: (profile: string) => void
  /// 本轮是否以安全模式启动（ADR-0025）：为真时面板顶部说明"这里的'已启用'指配置层，
  /// 本轮实际未生效"——否则它会与已装插件列表的运行态徽标自相矛盾。
}

/** 正在执行的一次动作（进度导轨的数据源）。 */
interface RunState {
  capId: string
  variantId: string
  ops: readonly CapabilityOp[]
  index: number
}

export function ExperimentalCapabilities({
  refreshKey,
  onNotice,
  onRestart,
}: Props) {
  const { t } = useI18n()
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

  const load = useCallback(async () => {
    if (!profile) return
    setLoading(true)
    setError(null)
    try {
      setCaps(await api.listExperimentalCapabilities(profile))
    } catch (e) {
      setCaps(null)
      setError(`${t.market.capLoadFailed}：${String(e)}`)
    } finally {
      setLoading(false)
    }
  }, [profile, t])

  useEffect(() => {
    // 换档即清"选择"与"失败"：两者都绑定在具体档位上，留着会串味（选中态指向另一个
    // profile 的变体、失败详情挂在别的档上）。
    setPicked({})
    setFailures({})
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
      return cap.variants.find((v) => v.id === id) ?? cap.variants[0]
    },
    [picked],
  )

  /** 统一执行入口：跑计划 → 回读状态 → 记成败。 */
  const execute = useCallback(
    async (cap: Capability, variantId: string, ops: readonly CapabilityOp[]) => {
      if (ops.length === 0) {
        await load()
        return
      }
      setRun({ capId: cap.id, variantId, ops, index: 0 })
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
          onNotice?.(`${cap.labelZh}：${t.market.capDone}`, "ok")
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
          onNotice?.(`${cap.labelZh}：${t.market.capPartial(result.completedOps, result.totalOps)}`, "warn")
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

  // 确认后**先关框再执行**：进度与失败都在卡片里就地呈现（模态不遮住它们）。
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
      other: all.filter((c) => c.state !== "on" && c.state !== "off").length,
    }
  }, [caps])

  if (!profile) {
    return <p className="px-1 py-6 text-center text-label text-dim">{t.market.capPickProfile}</p>
  }

  return (
    <div className="flex flex-col gap-3">
      <header className="flex flex-wrap items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
            <h2 className="text-lead font-semibold text-ink">{t.market.capTitle}</h2>
            {/* DSH 官方标记（2026-09-17 维护者裁定）：这些是上游官方实验包，
                不是社区插件——来源必须在第一阅读层说清。 */}
            <Badge
              variant="outline"
              className="h-4.5 gap-1 rounded-full border-brand/25 bg-brand/5 px-2 text-meta font-normal text-brand-deep"
            >
              <BadgeCheck className="size-3" />
              {t.market.capOfficialBadge}
            </Badge>
          </div>
          <p className="mt-1 max-w-prose text-label leading-relaxed text-dim">
            {t.market.capDesc}
          </p>
          <p className="mt-0.5 max-w-prose text-label leading-relaxed text-faint">
            {t.market.capOfficialNote}
          </p>
        </div>
        <div className="flex shrink-0 flex-col items-end gap-1">
          <span className="whitespace-nowrap text-meta text-faint">
            {t.market.capTargetProfile}
          </span>
          <Select value={profile} onValueChange={setProfile}>
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

      {caps && (
        <div className="flex flex-wrap items-center gap-x-3 gap-y-1.5 rounded-lg border border-line bg-wash px-3 py-2">
          <div className="flex flex-wrap items-center gap-x-2 gap-y-1 text-label">
            <span className="text-ink">{t.market.capSummary(summary.on, caps.length)}</span>
            {summary.other > 0 && (
              <span className="text-warn">{t.market.capSummaryNeedsWork(summary.other)}</span>
            )}
            {summary.off > 0 && (
              <span className="text-faint">{t.market.capSummaryOff(summary.off)}</span>
            )}
          </div>
          {/* 状态轨：一卡一点，颜色只表状态（与卡片图标底同源）。纯装饰，文字已给全信息。 */}
          <div aria-hidden className="ml-auto flex items-center gap-1.5">
            {caps.map((c) => (
              <span
                key={c.id}
                title={c.labelZh}
                className={`block size-2 rounded-full ${
                  c.state === "on"
                    ? "bg-ok"
                    : c.state === "conflict" || c.state === "partial"
                      ? "bg-danger"
                      : c.state === "disabled"
                        ? "bg-warn"
                        : "bg-line"
                }`}
              />
            ))}
          </div>
        </div>
      )}

      {dirty && (
        <div className="flex flex-wrap items-center gap-2 rounded-lg border border-warn/30 bg-warn-soft px-3 py-2 text-label text-warn">
          <Info className="size-3.5 shrink-0" />
          <span>{t.market.capRestartHint}</span>
          {onRestart && (
            <Button
              size="sm"
              variant="outline"
              className="ml-auto h-6 gap-1 px-2 text-micro"
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
        <div className="flex flex-wrap items-center gap-2 rounded-lg border border-danger/30 bg-danger-soft px-3 py-2 text-label text-danger">
          <TriangleAlert className="size-3.5 shrink-0" />
          <span className="min-w-0 break-words">{error}</span>
          <Button
            size="sm"
            variant="outline"
            className="ml-auto h-6 px-2 text-micro"
            onClick={() => void load()}
          >
            {t.market.capReload}
          </Button>
        </div>
      )}

      {caps?.map((cap, i) => (
        <CapabilityCard
          key={cap.id}
          index={i}
          cap={cap}
          variant={selectedVariant(cap)}
          run={run?.capId === cap.id ? run : null}
          failure={failures[cap.id]?.error ?? null}
          failureKind={failures[cap.id]?.failureKind ?? null}
          failedVariantId={failures[cap.id]?.variantId ?? null}
          onPick={(id) => setPicked((prev) => ({ ...prev, [cap.id]: id }))}
          onToggle={handleSwitch}
          onRepair={(v) => runEnable(cap, v)}
          // 续跑目标 = **失败时那个变体**（不是此刻选中的那个）：`planReplace` 会按
          // 回读到的真实状态补缺口，重跑同一个变体才是"继续剩余步骤"。
          onResume={(v) => {
            const id = failures[cap.id]?.variantId
            const target = id ? cap.variants.find((x) => x.id === id) : undefined
            runEnable(cap, target ?? v)
          }}
          onRemove={(v) => setPending({ kind: "remove", cap, variantId: v.id })}
          t={t}
        />
      ))}

      <ConfirmDialog
        open={pending !== null}
        title={
          pending
            ? pending.kind === "enable"
              ? t.market.capConfirmTitle(pending.cap.labelZh)
              : t.market.capRemoveTitle(pending.cap.labelZh)
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
  return variant ? !variant.toggleOffSupported : false
}

/** 确认框的要点清单：启用给前置与替换警告，移除给不可逆影响面。 */
function confirmPoints(
  pending: PendingAction,
  t: ReturnType<typeof useI18n>["t"],
): string[] {
  const variant = pending.cap.variants.find((v) => v.id === pending.variantId)
  if (!variant) return []
  if (pending.kind === "enable") {
    const points = [...variant.prerequisitesZh]
    if (pending.cap.activeVariant && pending.cap.activeVariant !== variant.id) {
      const from = pending.cap.variants.find((v) => v.id === pending.cap.activeVariant)
      points.push(t.market.capReplaceFrom(from?.labelZh ?? ""))
    }
    return points.length > 0 ? points : [t.market.capNoPrereq]
  }
  return [
    t.market.capRemovePointPackages(variant.steps.length),
    t.market.capRemovePointRows,
    ...(variant.toggleOffSupported ? [] : [t.market.capRemovePointLayer]),
  ]
}

/** 单张能力卡：身份（这是谁）→ 状态 → 插件（哪几个包 / 选哪一档）→ 动作 → 详情。
 *
 * 2026-09-17 重做（维护者裁定：**命名直接用插件名** ＋「这一页 UI/UX 要规范」）：
 *   · **一档一行、行首就是插件名**：不再有"后端 chips ＋ 官方包清单"两套并行表达，
 *     也不再出现「Web 档 / 复用已装的 cua-driver」这类我们发明的名字（见
 *     `variantPackageRoles`：标识包 = 该档独有，基座包 = 兄弟档共用）；
 *   · 刻度按仓库既有 token：正文 note(13) / 次要 label(11) / 节标 meta(10) / 角标 micro(9)；
 *   · 圆角按角色：面 xl(14)、可选行与控制件 md(10)、状态徽标胶囊；
 *   · 品牌色只承担"选中 / 主操作"，状态色只承担状态，两者不混用（旧版把选中与生效都
 *     画成同一个点，用户分不清"我选了它"与"它在跑"）；
 *   · 动效克制：入场上浮 4px（可关）、详情高度展开；`prefers-reduced-motion` 全关。
 */
function CapabilityCard({
  cap,
  variant,
  run,
  failure,
  failureKind,
  failedVariantId,
  index,
  onPick,
  onToggle,
  onRepair,
  onResume,
  onRemove,
  t,
}: {
  cap: Capability
  variant: CapabilityVariant
  run: RunState | null
  failure: string | null
  /** 失败分类（后端给）；`null` = 未知，按通用话术处理。 */
  failureKind: FailureKind | null
  /** 发起失败的那个变体（`null` = 无失败）。 */
  failedVariantId: string | null
  /** 卡片序号：只用于入场错峰（≤6 张，延迟封顶，不做长尾）。 */
  index: number
  onPick: (id: string) => void
  onToggle: (cap: Capability, variant: CapabilityVariant, next: boolean) => void
  onRepair: (variant: CapabilityVariant) => void
  onResume: (variant: CapabilityVariant) => void
  onRemove: (variant: CapabilityVariant) => void
  t: ReturnType<typeof useI18n>["t"]
}) {
  const [openDetail, setOpenDetail] = useState(false)
  const reduceMotion = useReducedMotion()
  const Icon = ICONS[cap.id] ?? Bot
  const on = variant.state === "on"
  const busy = run !== null
  const multi = cap.variants.length > 1
  // 图标底 = 状态色（一眼看出这张卡是不是要处理；off 保持中性，不制造噪音）
  const tile =
    variant.state === "on"
      ? "border-ok/30 bg-ok-soft text-ok"
      : variant.state === "disabled"
        ? "border-warn/30 bg-warn-soft text-warn"
        : variant.state === "partial"
          ? "border-danger/30 bg-danger-soft text-danger"
          : "border-line bg-wash text-dim"
  // 层类能力：关闭 = 移除。这条**必须在按下开关之前就看得见**，否则用户以为能秒关。
  // 但只在"真的就位"（on/disabled）时才成立：`toggle_off_supported` 对**没装齐**的档
  // 也是 false（分类要读包自己的 manifest，没装就不知道），若照它判断，一次失败的安装
  // 会让卡片谎称"本档由 profile 层提供"——真机 2026-09-16 暴露。
  const offIsRemove =
    !variant.toggleOffSupported &&
    (variant.state === "on" || variant.state === "disabled")
  // 被超集档包含的档（如"自建档"之于"Web 档"）：不提供独立开关——它的包就是超集档的
  // 基础层，单独关掉会把超集档一起拆坏。真机暴露于 2026-09-16。名字统一用插件名。
  const subsumedBy = variant.subsumedBy ? variantDisplayName(cap, variant.subsumedBy) : null
  const otherActive =
    cap.activeVariant !== null && cap.activeVariant !== variant.id
      ? cap.variants.find((v) => v.id === cap.activeVariant)
      : undefined
  // 宿主前置缺失（2026-09-16 真机事故）：开关必须**禁用**，并把后端给的这句话原样展示。
  // 只做提示是不够的——这类包装上去的代价是"工作台起不来"，用户根本没有回退余地。
  const blocked = variant.prerequisiteMissing
  const variantName = variantDisplayName(cap, variant.id)
  const detailId = `cap-detail-${cap.id}`
  // 各档共有的前置只讲一次（去掉 Chrome 系前置在三行里重复三遍的噪音）
  const commonPrereqs = commonPrerequisites(cap)

  return (
    <motion.article
      initial={reduceMotion ? false : { opacity: 0, y: 4 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{
        duration: 0.24,
        ease: "easeOut",
        delay: reduceMotion ? 0 : Math.min(index, 6) * 0.035,
      }}
      className="rounded-xl border border-line bg-panel p-4 shadow-2xs transition-shadow hover:shadow-xs"
    >
      {/* 身份 + 控制：一行读全"这是什么 / 现在怎样 / 能做什么"。 */}
      <div className="flex items-start gap-3">
        <div
          className={`flex size-10 shrink-0 items-center justify-center rounded-md border ${tile}`}
        >
          <Icon className="size-4.5" />
        </div>

        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
            <h3 className="text-note font-semibold text-ink">{cap.labelZh}</h3>
            <Badge
              variant="outline"
              className="h-4.5 rounded-full border-line bg-wash px-1.5 text-micro font-normal text-faint"
            >
              {t.market.capOfficialBadge}
            </Badge>
            <StateBadge cap={cap} variant={variant} t={t} />
          </div>
          <p className="mt-1 max-w-prose text-label leading-relaxed text-dim">
            {cap.summaryZh}
          </p>
        </div>

        <div className="flex shrink-0 items-center gap-2">
          <Switch
            aria-label={t.market.capSwitchLabel(`${cap.labelZh} · ${variantName}`)}
            checked={on}
            disabled={busy || subsumedBy !== null || blocked !== null}
            onCheckedChange={(next) => onToggle(cap, variant, next)}
          />
        </div>
      </div>

      {/* 插件：这一档是哪几个包 + 兄弟档怎么切。行首即插件名（官方名），
          自己独有的包在前，共用的基座包随后如实标注。 */}
      <div className="mt-3 border-t border-line/70 pt-3">
        <div className="flex items-center justify-between gap-2">
          <span className="text-meta font-medium tracking-wider text-faint">
            {t.market.capPluginsLabel}
          </span>
          <button
            type="button"
            aria-expanded={openDetail}
            aria-controls={detailId}
            onClick={() => setOpenDetail((v) => !v)}
            className="flex items-center gap-0.5 rounded-md border border-line bg-panel px-1.5 py-0.5 text-meta text-dim transition-colors hover:border-brand/30 hover:text-ink focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand/40"
          >
            {t.market.capDetailToggle}
            <ChevronRight
              className={`size-3 transition-transform ${openDetail ? "rotate-90" : ""}`}
            />
          </button>
        </div>

        <div
          {...(multi
            ? { role: "radiogroup", "aria-label": t.market.capPluginsLabel }
            : {})}
          className="mt-1.5 flex flex-col gap-1.5"
        >
          {cap.variants.map((v) => {
            const roles = variantPackageRoles(cap, v.id)
            const selected = v.id === variant.id
            const isActive = cap.activeVariant === v.id
            const vBlocked = v.prerequisiteMissing
            // 被包含的档（它的包就是超集档的基础层）不是"可切换的后端"：不给交互，
            // 免得用户以为点它能单独开关（开关与移除统一由超集档控制，见 §⑤ 门禁）。
            const selectable = multi && !v.subsumedBy
            const extras = v.prerequisitesZh.filter((p) => !commonPrereqs.includes(p))
            const rowCls = [
              "flex w-full items-start gap-2.5 rounded-md border px-3 py-2.5 text-left transition-colors",
              selected
                ? "border-brand/40 bg-brand/5"
                : selectable
                  ? "border-line bg-bg hover:border-brand/25 hover:bg-wash/60"
                  : "border-line bg-bg",
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
                      <span key={pkg} className="break-all font-mono text-label font-medium text-ink">
                        {pkg}
                      </span>
                    ))}
                    {/* 行内徽标只在多档时才需要（它回答"哪一档在跑"）；单档能力的卡片头
                        已经说了同一件事，重复就是噪音。
                        **必须按变体自己的状态取词**：后端契约里 `activeVariant` 的含义是
                        "当前生效**或已就位但停用**的那一档"（`official_catalog.rs` 的
                        Disabled 分支），照抄「已启用」会在卡片说「已停用」时自相矛盾
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
                  {v.noteZh && (
                    <span className="mt-1 block max-w-prose text-label leading-relaxed text-dim">
                      {v.noteZh}
                    </span>
                  )}
                  {roles.shared.length > 0 && (
                    <span className="mt-1 block break-all font-mono text-meta text-faint">
                      {t.market.capAlsoInstalls(roles.shared.join(" · "))}
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
                      <span>{vBlocked}</span>
                    </span>
                  )}
                </span>
              </>
            )
            return selectable ? (
              <button
                key={v.id}
                type="button"
                role="radio"
                aria-checked={selected}
                aria-label={roles.primary.join(" + ")}
                disabled={busy}
                onClick={() => onPick(v.id)}
                className={rowCls}
              >
                {body}
              </button>
            ) : (
              <div key={v.id} className={rowCls}>
                {body}
              </div>
            )
          })}
        </div>

        {commonPrereqs.length > 0 && (
          <span className="mt-2 flex flex-col gap-0.5">
            <span className="text-meta text-faint">{t.market.capPrereqCommon}</span>
            {commonPrereqs.map((p) => (
              <span key={p} className="flex items-start gap-1.5 text-label leading-relaxed text-dim">
                <Info className="mt-0.5 size-3 shrink-0 text-faint" />
                <span>{p}</span>
              </span>
            ))}
          </span>
        )}

        {/* 同族替换与"包含关系"：说清开关会连带动到谁（名字一律用插件名）。 */}
        {subsumedBy ? (
          <p className="mt-2 max-w-prose text-label leading-relaxed text-faint">
            {t.market.capSubsumedBy(subsumedBy)}
          </p>
        ) : (
          otherActive && (
            <p className="mt-2 max-w-prose text-label leading-relaxed text-warn">
              {otherActive.state === "on"
                ? t.market.capOtherActive(variantDisplayName(cap, otherActive.id))
                : t.market.capOtherReady(variantDisplayName(cap, otherActive.id))}
            </p>
          )
        )}
        {offIsRemove && (
          <p className="mt-1 max-w-prose text-label leading-relaxed text-faint">
            {t.market.capOffIsRemove}
          </p>
        )}
      </div>

      {run && (
        <div className="mt-3 rounded-md border border-brand/25 bg-brand/5 px-3 py-2.5">
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

      {/* 动作行只在**有事要做**时出现（没有它就整行不渲染，卡片因此更矮更静） */}
      {!run && (variant.state === "partial" || cap.state === "conflict" || variant.state === "disabled") && (
        <div className="mt-3 flex flex-wrap items-center gap-2 border-t border-line/70 pt-2.5">
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

      <AnimatePresence initial={false}>
        {openDetail && (
          <motion.div
            id={detailId}
            initial={reduceMotion ? false : { height: 0, opacity: 0 }}
            animate={{ height: "auto", opacity: 1 }}
            exit={reduceMotion ? { height: 0, opacity: 0 } : { height: 0, opacity: 0 }}
            transition={{ duration: reduceMotion ? 0 : 0.2, ease: "easeOut" }}
            className="overflow-hidden"
          >
            <div className="mt-3 flex flex-col gap-3 rounded-xl border border-line bg-wash/60 px-3 py-3">
              <section>
                <h4 className="text-meta font-semibold tracking-wider text-ink">
                  {t.market.capUnlocks}
                </h4>
                <p className="mt-1 max-w-prose text-label leading-relaxed text-dim">
                  {cap.unlocksZh}
                </p>
              </section>
              <section>
                <h4 className="text-meta font-semibold tracking-wider text-ink">
                  {t.market.capImplTitle}
                </h4>
                <ul className="mt-1.5 flex flex-col gap-2.5">
                  {variant.steps.map((s) => (
                    <li key={s.package} className="text-label">
                      <div className="flex items-baseline gap-1.5 font-mono">
                        <span className="text-micro text-faint">{s.ordinal}.</span>
                        <span className="break-all text-ink">{s.package}</span>
                      </div>
                      <div className="mt-1 flex flex-wrap items-center gap-1.5 pl-4">
                        <Badge
                          variant="outline"
                          className="h-4.5 rounded-full border-line bg-panel px-1.5 text-micro font-normal text-dim"
                        >
                          {s.activation === "auto_bundle"
                            ? t.market.capActivationAuto
                            : t.market.capActivationInsert}
                        </Badge>
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
                      {s.description ? (
                        <p className="mt-1 max-w-prose pl-4 text-label leading-relaxed text-dim">
                          <span className="text-faint">{t.market.capOfficialDesc}：</span>
                          {s.description}
                        </p>
                      ) : (
                        <p className="mt-1 pl-4 text-label text-faint">
                          {t.market.capOfficialDescPending}
                        </p>
                      )}
                      <div className="mt-1 pl-4 font-mono text-meta text-faint">
                        {t.market.capPinned(s.spec)}
                        {s.activation === "insert_row" && ` · ${s.rowId}`}
                      </div>
                      {s.versionNotice && (
                        <p className="mt-1 max-w-prose pl-4 text-label leading-relaxed text-warn">
                          {s.versionNotice}
                        </p>
                      )}
                    </li>
                  ))}
                </ul>
              </section>
              {!subsumedBy &&
                (variant.state === "on" ||
                  variant.state === "disabled" ||
                  variant.state === "partial") && (
                <section className="flex items-center justify-between gap-2 border-t border-line/70 pt-2.5">
                  <span className="max-w-prose text-label leading-relaxed text-faint">
                    {variant.toggleOffSupported
                      ? t.market.capRemoveExplainedSoft
                      : t.market.capRemoveExplainedLayer}
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
          </motion.div>
        )}
      </AnimatePresence>
    </motion.article>
  )
}

/** 失败块：**一句人话** + 可折叠的原始输出 + 唯一的续跑入口。
 *
 * 真机 2026-09-16 暴露两件事：① 原始输出（registry URL + pnpm 调用链 + TLS 细节）上百字符，
 * 直接铺开会把"我该怎么办"淹没；② 失败块里的「继续剩余步骤」与卡片底部「修复」是同一个动作，
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
    <div className="mt-3 rounded-lg border border-danger/30 bg-danger-soft px-3 py-2.5">
      <p className="text-label font-semibold text-danger">
        {t.market.capFailed}
        {failedVariantLabel && `（${failedVariantLabel}）`}
      </p>
      <p className="mt-1 text-micro leading-relaxed text-dim">{hint}</p>
      <div className="mt-2 flex flex-wrap items-center gap-2">
        <Button
          size="sm"
          className="h-6 bg-brand px-2 text-micro text-white hover:bg-brand/90"
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
      <Badge variant="outline" className="h-5 border-line bg-wash px-1.5 text-micro text-dim">
        {t.market.capStateSubsumed}
      </Badge>
    )
  }
  if (cap.state === "conflict") {
    return (
      <Badge className="h-5 border-warn/40 bg-warn-soft px-1.5 text-micro text-warn">
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
  // 卡片头徽标说**能力**的实话，不说"当前选中的后端"的状态（2026-09-16 真机暴露）：
  // 桌面控制里「随包自带运行时」正在生效、用户点了被前置门挡住的「复用已装 cua-driver」，
  // 旧写法照**选中档**渲染 → 整张卡报「需要修复」，而能力其实是好的（原生档在跑）。
  // 选中档自身的状态由 chips 上的状态点 / ⚠ 表达。
  // （`conflict` 已在上面的早退分支拦掉，TS 据此也把类型收窄了，故可直接索引。）
  const shown: keyof typeof map = cap.state
  return (
    <Badge variant="outline" className={`h-5 px-1.5 text-micro ${map[shown].cls}`}>
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
