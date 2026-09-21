// ExperimentalCapabilities.tsx —— 「实验能力」开关面板（2026-09-16，ADR-0020 §7）。
//
// 2026-09-20（ADR-0028）：迁入 Profile 详情页、改为**受控组件**——档位来自父级
// （`ProfileDetailPane` 的选中档），不再自持 profile 下拉；插件中心子页下线。
//
// 2026-09-20（ADR-0028 第二批，清单打标合并）：**目录数据也改受控**——`caps` 由父级
// 取数后传入（与同页「插件列表」共用一次回读，禁双源：同窗不挂两条能力目录链）。
// 理由：插件列表要给每个包打「实验性 · <能力>」标，归属只有能力目录知道；而目录是
// 按 profile 的——两处各取一次，切档/写操作后必然对不上。负载随之搬到父级
// （`ProfileDetailPane` 的 `loadCaps`，同样带档位护栏）；本组件只保留**动作侧**闸门
// （落账复核档位 / 进度按发起档归属 / 迟到的 onChanged 作废）。
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
//
// ## 为什么第三次重做（v3 → v4，2026-09-20 维护者要求"布局和交互着重重构"）
//
// 先认账的四个结构性问题（v3 几轮迭代攒出来的）：
//   ① 左右栏大量重复——状态徽标、包名、前置、替换说明在两处各说一遍；
//   ② 详情面十节平铺——最核心的"选后端"埋在第 5 节，全靠发丝线分隔，眼睛找不到重点；
//   ③ 开关作用于"选中的后端"而非"生效的后台"——于是有 misaligned 中间态，
//      行里得挂小图标补救，两处开关还各说各话；
//   ④ 列表行扛 4 种信号，"扫一眼"变"读一遍"。
//
// v4 = **控制面 / 后果面拆分**（经 ui-ux-pro-max 规则核验后修订，2026-09-20）：
//   · 左栏 = 唯一控制面：一能力一行（图标 · 名称 · 状态 · **生效后端的包名** · 开关），
//     多变体时行尾一个**溢出菜单**「切换后端」（点选即发起，走既有确认链）；
//   · 右栏 = 后果面：身份 → 进度/失败 → 后端对照（**只读**）→ 会发生什么 + 共同前置
//     → 排障细节（默认折叠）→ 移除（危险区，沉底）。不再重复状态与开关。
//   · **picked 中间态取消**：菜单点哪个后端就直接发起；行内开关只管生效档的快速开停
//     （关→开时启用行内显示的那个，aria-label 写死目标）。misaligned 态随之死亡。
//
// 规则核验记死了两个"不许回头"的边界（ui-ux-pro-max，`compact-label-overflow` /
// `truncation-strategy` / `web-target-size` / `overflow-menu`）：
//   · **左栏行内不放后端 pills**：50+ 字符包名在 21–25rem 栏宽里 nowrap 放不下、
//     截断会砍掉区分各档的尾部（仓库 2026-09-17 裁定：包名完整折行不许截断）、
//     折行就不是 pill；且可点目标需 ≥24×24 CSSpx（WCAG 2.2 AA）。出路只有溢出菜单。
//   · **后端对照只读**：选择即动作，看完即走；要对比全部后端看右栏，要换看左栏菜单。

import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { motion, useReducedMotion } from "framer-motion"
import {
  ArrowLeftRight,
  BadgeCheck,
  Check,
  ChevronRight,
  Info,
  LoaderCircle,
  RotateCw,
  TriangleAlert,
  Wrench,
} from "lucide-react"

import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { ConfirmDialog } from "@/components/ui/confirm-dialog"
import { Tip } from "@/components/ui/info-tip"
import { Progress } from "@/components/ui/progress"
import { Switch } from "@/components/ui/switch"
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
  variantShortName,
  type CapabilityOp,
} from "@/lib/experimentalCapabilities"
import { useI18n } from "@/stores/i18nStore"
import { capabilityIcon } from "@/components/ui/capability-icon"
import { commonPackagePrefix, packageTail } from "@/lib/pluginDisplay"
import type {
  Capability,
  CapabilityStep,
  CapabilityVariant,
  FailureKind,
} from "@/types/ipc"

/** 详情面的 DOM id：左栏的行用 `aria-controls` 指向它（屏幕阅读器知道"这行控制谁"）。 */

/** 清单行的 DOM id（窄窗口"返回清单"时把焦点还给刚才那一行）。 */
const rowId = (capId: string) => `cap-row-${capId}`


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
  /** 目标 Profile（受控，ADR-0028）：随 ProfileDetailPane 选中档走，组件不自持档位。 */
  profile: string
  /** 能力目录（受控数据，ADR-0028 第二批）：父级取数，与「插件列表」共用一次回读。 */
  caps: Capability[] | null
  capsLoading: boolean
  capsError: string | null
  /** 目录刷新（重试 / 动作后重取都用它；不含插件行表）。 */
  onRefreshCaps: () => void
  onNotice?: (message: string, tone?: "ok" | "warn") => void
  /** 重启该 Profile（复用 ProfileManager 的既有确认链）；缺省则只给文字提示。 */
  onRestart?: (profile: string) => void
  /** 写操作完成后回调：父级据此刷新同页「插件列表」的行表/清单。 */
  onChanged?: () => void
}

/** 正在执行的一次动作（进度导轨的数据源）。
 *
 *  单槽是**有意的**：行写（写/删/停用挂载行）不入队，`plugins.rs` 侧也没有互斥，
 *  两条编排行交错会丢配置写入（界面报"已完成"，文件里那行没写上），同族两次 teardown
 *  还会打乱"先删行再卸包"的顺序（悬空行 = dsh 起不来）。所以 `run !== null` 期间
 *  **所有**动作入口都必须关掉——见下方 `busy` 的传法（2026-09-17 独立复核）。
 *
 *  `profile`：动作发起时那个档（2026-09-20，ADR-0028）。换档入口迁到父级左列表后，
 *  "跑着 A 档、用户看 B 档"首次成为可达状态——进度/`dirty`/失败都必须按它归属，
 *  否则 B 档的行会挂着 A 档的转圈、B 档会弹出"配置已变更"的假提示。 */
interface RunState {
  capId: string
  profile: string
  ops: readonly CapabilityOp[]
  index: number
}

export function ExperimentalCapabilities({
  profile,
  caps,
  capsLoading,
  capsError,
  onRefreshCaps,
  onNotice,
  onRestart,
  onChanged,
}: Props) {
  const { t } = useI18n()
  const [run, setRun] = useState<RunState | null>(null)
  /** 失败详情按能力记，**并钉住发起失败的那个变体**：让用户在原处看到原因并可续跑。
   *  只按能力记会出错——用户换一下后端选择器再点「继续剩余步骤」，就会去操作另一个变体
   *  （2026-09-16 独立评审核出：那是"什么都没做还报成功"）。 */
  const [failures, setFailures] = useState<
    Record<string, { variantId: string; error: string; failureKind: FailureKind | null }>
  >({})
  /** 每次成功动作后置位：提示"重启后生效"。 */
  const [dirty, setDirty] = useState(false)
  /** 手风琴：当前展开的能力（`null` = 全部收起，一屏看全所有能力）。
   *  2026-09-21 单栏化后，原先的 `selectedId` / `drilled` 那套"选谁看详情 / 窄窗下钻"
   *  状态一并退役——展开态本身就是"我在看谁"。 */
  const [expandedId, setExpandedId] = useState<string | null>(null)

  // 待确认的动作（启用 / 移除都走确认；停用可逆，不拦）。
  const [pending, setPending] = useState<PendingAction | null>(null)

  /** 档位的**最新值**（渲染期同步）：动作落账与父级刷新都靠它判断自己是否已经过期。
   *
   *  为什么需要（2026-09-20，ADR-0028 迁移带出的副作用）：换档入口在父级左列表，
   *  面板的 busy 管不到那条路，"跑着 A 档、用户看 B 档"首次成为可达状态。目录的
   *  **回读**护栏随负载一起搬到了父级（`ProfileDetailPane::loadCaps` 同款判据）；
   *  这里守的是**动作侧**：落账前的复核、finally 里迟到的 `onChanged`（闭包钉着
   *  发起时的旧档名，会把旧档详情写进父级此刻显示的新档页面）。 */
  const profileRef = useRef(profile)
  profileRef.current = profile

  // 换档即清"失败""dirty"：两者都绑定在具体档位上，留着会串味（失败详情挂在别的档上、
  // 「立即重启」打在没改过的档上）。
  useEffect(() => {
    setFailures({})
    setExpandedId(null)
    setDirty(false)
  }, [profile])

  /** 行的**开关目标**：当前生效（或已就位但停用）的档 → 首个**可用**档 → 首个档。
   *
   *  v4 起 picked 中间态取消（2026-09-20 重构）：行里显示的自古以来就是事实，不再有
   *  "选中 ≠ 生效"的隐藏态。兜底**跳过被前置门挡住的档**（2026-09-17 独立复核）：
   *  选它没有任何可用动作（开关必然灰），"清单随手开关"的承诺会被一次误点作废；
   *  落到第一个可用档上，清单的开关才是真的可用；全被挡才退回首个。
   *  目录序即推荐序（Rust 侧 `CAPABILITIES` 同口径：UI 默认选中首个）。 */
  const rowTarget = useCallback(
    (cap: Capability): CapabilityVariant =>
      cap.variants.find((v) => v.id === cap.activeVariant) ??
      cap.variants.find((v) => !v.prerequisiteMissing && !v.subsumedBy) ??
      cap.variants[0],
    [],
  )

  /** 统一执行入口：跑计划 → 回读状态 → 记成败。 */
  const execute = useCallback(
    async (cap: Capability, variantId: string, ops: readonly CapabilityOp[]) => {
      // 动作一发起就把详情面切到该能力：进度与失败都在用户正看着的那一栏里出现
      // （v3 的分栏形态下，"就地呈现"= 选中它，而不是把卡片往下长）。
      // 动作一发起就展开该能力：进度与失败都在用户正看着的那张卡里出现
      setExpandedId(cap.id)
      if (ops.length === 0) {
        // 无事可做（状态已就绪）：让父级重取一次目录即回报真实状态。
        onRefreshCaps()
        return
      }
      setRun({ capId: cap.id, profile, ops, index: 0 })
      setFailures((prev) => {
        const next = { ...prev }
        delete next[cap.id]
        return next
      })
      try {
        const result = await runCapabilityOps(capabilityIO, profile, ops, (p) =>
          setRun((r) => (r ? { ...r, index: p.index } : r)),
        )
        // 结果落账前先看档位是否还在：换了档就整体丢弃（下面的 `finally` 会重挂新档的清单）。
        // 不丢的话，A 档的失败会挂在 B 档的能力上（两档能力 id 同名）、B 档弹出 A 档的
        // "配置已变更"，而"立即重启"按钮会重启一个没被改过的 Profile。
        if (profileRef.current !== profile) return
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
        // 换了档：本次动作的结果属于**旧档**——目录回读与父级刷新一并作废。
        // `onChanged` 也在这里挡：它是发起时那一版 `reload`（闭包里钉着旧的 profile 名），
        // 迟到调用会把旧档的详情/行表写进父级现在显示的新档页面上。
        if (profileRef.current === profile) {
          onRefreshCaps()
          onChanged?.()
        }
      }
    },
    [profile, onRefreshCaps, onNotice, onChanged, t],
  )

  // 开启 = 让位 + 装上：`planReplace` 自己判断有没有别的后端要拆（同族并存会激活失败），
  // 所以"换后端 / 冲突修复 / 首次开启"共用这一个入口，不会漏掉让位。
  const runEnable = (cap: Capability, variant: CapabilityVariant) => {
    void execute(cap, variant.id, planReplace(cap, variant.id))
  }

  const runDisable = (cap: Capability, variant: CapabilityVariant) => {
    void execute(cap, variant.id, planDisable(variant))
  }

  /** 启用某个档（v4 单一入口）：要装新包、或要拆别的已就位档 → 先确认（破坏面说清楚）；
   *  已就位只差补几步行 → 免确认直接做（"关而不卸"秒回的手感）。 */
  const activate = (cap: Capability, variant: CapabilityVariant) => {
    const willReplace = cap.activeVariant !== null && cap.activeVariant !== variant.id
    if (willReplace || variant.state === "off" || variant.state === "on") {
      setPending({ kind: "enable", cap, variantId: variant.id })
      return
    }
    runEnable(cap, variant)
  }

  const handleSwitch = (cap: Capability, variant: CapabilityVariant, next: boolean) => {
    // 进入本组件的只可能是 dock 策展的能力（dsh 安装自带的已被父级 `dockCuratedCaps`
    // 过滤，2026-09-20 维护者裁定：它们归 dsh 官方插件页托管，dock 不摆第二套入口）。
    if (next) {
      activate(cap, variant)
      return
    }
    // 关闭：能纯行级关就秒关（可逆、不打断）；不能则等价于移除，走破坏性确认。
    if (variant.toggleOffSupported) {
      runDisable(cap, variant)
      return
    }
    setPending({ kind: "remove", cap, variantId: variant.id })
  }

  /** 「切换后端」菜单的点选：点当前生效档 = 无操作（菜单项本就禁用）；点别的档 =
   *  与"开开关"同一条启用链（`activate` 自己决定要不要确认）。picked 中间态取消后，
   *  选择即动作——不再有"选中 ≠ 生效"的隐藏态。 */
  const switchBackend = (cap: Capability, variant: CapabilityVariant) => {
    if (cap.activeVariant === variant.id) return
    activate(cap, variant)
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

  return (
    <div className="flex flex-col gap-2.5">
      {/* 页头压成**一行**（2026-09-21 三次修订）：这是"操作台"，不是"说明书"。
          身份 + 官方来源 + 模块说明三件事，只留前两件的表面文字，模块说明（capDesc）
          收进标题旁的 ⓘ——维护者口径："减少描述性信息，若必须要则收敛到小 tip icon"。 */}
      <header className="flex flex-wrap items-center gap-x-3 gap-y-2">
        <span className="flex items-center gap-1">
          <h2 className="text-lead font-semibold text-ink">{t.market.capTitle}</h2>
          <Tip text={t.market.capDesc} label={t.tip.ariaFor(t.market.capTitle)} />
        </span>
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

      {capsLoading && !caps && (
        <p className="px-1 py-6 text-center text-label text-dim">
          <LoaderCircle className="mx-auto size-4 animate-spin" />
        </p>
      )}

      {capsError && (
        <div className="flex flex-wrap items-center gap-2 rounded-lg border border-danger/30 bg-danger-soft px-3 py-1.5 text-label text-danger">
          <TriangleAlert className="size-3.5 shrink-0" />
          <span className="min-w-0 break-words">{capsError}</span>
          <Button
            size="xs"
            variant="outline"
            className="ml-auto"
            onClick={onRefreshCaps}
          >
            {t.market.capReload}
          </Button>
        </div>
      )}

      {caps && (
        /* 单栏手风琴（2026-09-21 第三次布局范式变更，理由见 `CapabilityPanel` 文档注释）：
           收起态一屏看全所有能力；点头部展开对照与细节，同一时刻只展开一个。
           这里只可能是 dock 策展的能力——dsh 安装自带的（OPTIONAL_BUNDLES）已由父级
           过滤（`dockCuratedCaps`）：它们归 dsh 官方插件页托管，dock 不再摆第二套入口
           （2026-09-20 维护者裁定，ADR-0020 §2.8）。 */
        <section aria-label={t.market.capListLabel}>
          <div className="text-meta text-faint mb-1.5 flex flex-wrap items-center gap-x-2 px-0.5">
            <span className="font-medium tracking-wider">{t.market.capListLabel}</span>
            <span className="text-ink tabular-nums">
              {t.market.capSummary(summary.on, caps.length)}
            </span>
            {summary.other > 0 && (
              <span className="text-warn tabular-nums">
                {t.market.capSummaryNeedsWork(summary.other)}
              </span>
            )}
            {/* 「已停用」用中性色：它是**用户自己关的**（关而不卸），拿警示色等于
                给一个正常状态报警（2026-09-17 独立复核）。 */}
            {summary.disabled > 0 && (
              <span className="text-dim tabular-nums">
                {t.market.capSummaryDisabled(summary.disabled)}
              </span>
            )}
            {summary.off > 0 && (
              <span className="tabular-nums">{t.market.capSummaryOff(summary.off)}</span>
            )}
          </div>
          <ul className="flex flex-col gap-2">
            {caps.map((cap, i) => (
              <CapabilityPanel
                key={cap.id}
                index={i}
                cap={cap}
                target={rowTarget(cap)}
                expanded={expandedId === cap.id}
                onToggleExpanded={() =>
                  setExpandedId((cur) => (cur === cap.id ? null : cap.id))
                }
                // 进度只画在**发起时那个档**的卡上（`run.profile`）：换档后在途动作仍在跑，
                // 但转圈不该出现在新档的能力上（两档的能力 id 同名，单看 capId 必然误挂）。
                running={run?.capId === cap.id && run.profile === profile}
                busy={run !== null}
                failed={Boolean(failures[cap.id])}
                run={run?.capId === cap.id && run.profile === profile ? run : null}
                failure={failures[cap.id]?.error ?? null}
                failureKind={failures[cap.id]?.failureKind ?? null}
                failedVariantId={failures[cap.id]?.variantId ?? null}
                onToggle={handleSwitch}
                onSwitchBackend={(v) => switchBackend(cap, v)}
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
          </ul>
        </section>
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

/** **清单行（v4 控制面）**：一能力一行——图标 · 名称 · 状态 · **生效后端的包名** ·
 *  开关 · 切换后端菜单。行的职责只有三个：看状态、随手开关、发起换后端。
 *
 *  · 行内显示的自古以来就是**事实**：`target` = 当前生效（或已就位但停用）的档，
 *    没有则为首个**可用**档（目录序即推荐序）。v4 起 picked 中间态取消，"选中 ≠ 生效"
 *    的隐藏态随之死亡——行里的插件名与开关目标永远一致。
 *  · 包名**折行完整显示、不截断**（2026-09-17 裁定）：它是唯一要对得上
 *    `node_modules` / `cordis.patch.yml` 的字符串，截掉的正好是区分各档的尾部
 *    （`…cua-driver-mcp` 与 `…cua-driver-native` 会截成同一个前缀）。
 *  · 被超集档包含 / 前置门挡住 / 有未处理失败，都只用一个小图标 + `aria-describedby`
 *    提示，不占高度——完整说明在后果面里。
 *  · **换后端不放行内 pills**（ui-ux-pro-max `compact-label-overflow` /
 *    `truncation-strategy` / `web-target-size` 叠加致死，见文件头）：走行尾溢出菜单
 *    （`overflow-menu`），点选即发起。 */
/** **能力卡片**（2026-09-21 单栏手风琴：本模块的第三次布局范式变更）。
 *
 * ## 为什么推倒左右分栏（v3 定稿的形态）
 * v3 的分栏（左清单 / 右常驻详情）是为了解决 v2 的"内联展开把第二张卡推出屏幕"
 * （真机实测整页 1897px、一屏只装得下一张卡）。但它带来了更贵的问题：
 *   ① **左右必然重复**——分栏 UI 里右栏要讲清"我在说谁"，就只能把左栏选中行的身份
 *      再说一遍；去重去到最后，右栏顶着一行**孤儿 summary**（2026-09-21 真机截图）；
 *   ② **范式分叉**——同一个应用里，「插件列表」用*单栏卡片 + 头部控制 + 子行*表达能力，
 *      本模块却用分栏。同一个东西两种长相，用户每换一个 tab 就要重新学一遍；
 *   ③ 左右两栏**视觉重量失衡**：左栏三行很轻、右栏一大块很重，而右栏其实是次要
 *      信息（对照 / 排障）。
 *
 * ## 现在：与「插件列表」同范式
 * 单栏 + 手风琴。头部一行读完 **身份 · 状态 · 当前后端 · 开关 · 展开**（收起态一屏
 * 能看到全部能力，这正是单栏的价值）；点头部展开出对照与细节，**同一时刻只展开一个**
 * ——v3 担心的"越长越高"由手风琴挡住（最多一张卡展开，高度可控）。
 *
 * 展开体直接复用 `CapabilityPane`（功能与文案一字未改，只是不再自带外框），
 * 所以这次换范式是**纯布局改动**，零功能丢失。 */
function CapabilityPanel({
  cap,
  target,
  expanded,
  onToggleExpanded,
  running,
  busy,
  failed,
  run,
  failure,
  failureKind,
  failedVariantId,
  onToggle,
  onSwitchBackend,
  onRepair,
  onResume,
  onRemove,
  index,
  t,
}: {
  cap: Capability
  target: CapabilityVariant
  expanded: boolean
  onToggleExpanded: () => void
  running: boolean
  busy: boolean
  failed: boolean
  run: RunState | null
  failure: string | null
  failureKind: FailureKind | null
  failedVariantId: string | null
  onToggle: (cap: Capability, variant: CapabilityVariant, next: boolean) => void
  onSwitchBackend: (variant: CapabilityVariant) => void
  onRepair: (variant: CapabilityVariant) => void
  onResume: (variant: CapabilityVariant) => void
  onRemove: (variant: CapabilityVariant) => void
  index: number
  t: ReturnType<typeof useI18n>["t"]
}) {
  const reduceMotion = useReducedMotion()
  const Icon = capabilityIcon(cap.id)
  const on = target.state === "on"
  // 与清单行同一套判据（Boolean() 的由来见 CapabilityRow 处的长注释：
  // 字段缺省会静默变 undefined，`!== null` 会把 undefined 判成"有值"）。
  const subsumedBy = Boolean(target.subsumedBy)
  const blocked = Boolean(target.prerequisiteMissing)
  const name = variantDisplayName(cap, target.id)
  const shortName = variantShortName(cap, target.id)
  const panelId = `${rowId(cap.id)}-panel`
  const descId = `${rowId(cap.id)}-why`
  const why = [
    blocked ? target.prerequisiteMissing : null,
    subsumedBy ? t.market.capSubsumedBy(name) : null,
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
      className={`group bg-panel overflow-hidden rounded-xl border transition-colors ${
        expanded ? "border-brand/40" : "border-line hover:border-brand/30"
      }`}
    >
      {/* 头部：身份 · 状态 · 当前后端 · 开关 · 展开。一行读完，收起态也能看出在跑什么。 */}
      <div className="flex items-center gap-2.5 px-3 py-2.5">
        <button
          id={rowId(cap.id)}
          type="button"
          aria-expanded={expanded}
          aria-controls={panelId}
          onClick={onToggleExpanded}
          className="focus-visible:ring-brand/40 flex min-w-0 flex-1 items-center gap-2.5 rounded-lg text-left focus-visible:ring-2 focus-visible:outline-none"
        >
          <span
            className={`flex size-7 shrink-0 items-center justify-center rounded-lg border ${stateTone(
              target.state === "off" ? cap.state : target.state,
            )}`}
          >
            <Icon className="size-3.5" />
          </span>
          <span className="min-w-0 flex-1">
            <span className="flex items-center gap-1.5">
              <span className="text-label text-ink truncate font-medium">{cap.label}</span>
              <StateBadge cap={cap} variant={target} t={t} />
              {running && <LoaderCircle className="text-brand-deep size-3 shrink-0 animate-spin" />}
              {!running && failed && <TriangleAlert className="text-danger size-3 shrink-0" />}
            </span>
            {/* 第二行只讲一件事：**哪个插件**在提供它（完整包名进 title）。 */}
            <span className="mt-0.5 flex flex-wrap items-center gap-x-1.5">
              <span className="text-dim font-mono text-meta leading-snug" title={name}>
                {shortName}
              </span>
              {subsumedBy && <Check className="text-faint size-3 shrink-0" />}
              {blocked && <TriangleAlert className="text-danger size-3 shrink-0" />}
            </span>
          </span>
          <ChevronRight
            className={`text-faint group-hover:text-brand-deep size-3.5 shrink-0 transition-all ${
              expanded ? "rotate-90" : ""
            }`}
          />
        </button>
        {/* 能力说明（summary + unlocks）挂在这颗 ⓘ 上，不再铺进展开体：
            它们是"这是干什么的"的解释文本，而展开体是"怎么操作"的地方。
            悬浮触发器是 button，**不能**塞进上面那个展开按钮里（button 不许嵌套）。 */}
        <Tip
          text={t.market.capTipText(cap.summary, cap.unlocks)}
          label={t.tip.ariaFor(cap.label)}
          className="max-w-72 text-label leading-relaxed"
        />
        <Switch
          aria-label={t.market.capSwitchLabel(`${cap.label} · ${name}`)}
          aria-describedby={why.length > 0 ? descId : undefined}
          checked={on}
          disabled={busy || subsumedBy || blocked}
          onCheckedChange={(next) => onToggle(cap, target, next)}
          className="shrink-0"
        />
        {why.length > 0 && (
          <span id={descId} className="sr-only">
            {why.join(t.market.capWhyJoin)}
          </span>
        )}
      </div>

      {/* 展开体：复用 CapabilityPane（外框归卡片，故传 embeddedId）。 */}
      {expanded && (
        <CapabilityPane
          cap={cap}
          target={target}
          run={run}
          busy={busy}
          failure={failure}
          failureKind={failureKind}
          failedVariantId={failedVariantId}
          onRepair={onRepair}
          onResume={onResume}
          onRemove={onRemove}
          onSwitchBackend={onSwitchBackend}
          embeddedId={panelId}
          t={t}
        />
      )}
    </motion.li>
  )
}

function CapabilityPane({
  cap,
  target,
  run,
  busy,
  failure,
  failureKind,
  failedVariantId,
  onRepair,
  onResume,
  onRemove,
  onSwitchBackend,
  embeddedId,
  t,
}: {
  cap: Capability
  /** 行的开关目标（生效档 → 首个可用档）：详情/移除都作用于它。 */
  target: CapabilityVariant
  run: RunState | null
  /** **任意能力**在跑 → 关掉本面的恢复入口（`run` 是单槽，两个能力并发会丢配置写入）。 */
  busy: boolean
  failure: string | null
  /** 失败分类（后端给）；`null` = 未知，按通用话术处理。 */
  failureKind: FailureKind | null
  /** 发起失败的那个变体（`null` = 无失败）。 */
  failedVariantId: string | null
  onRepair: (variant: CapabilityVariant) => void
  onResume: (variant: CapabilityVariant) => void
  onRemove: (variant: CapabilityVariant) => void
  /** 对照面就地点选某个后端（与左栏溢出菜单同一条 activate 链）。 */
  onSwitchBackend: (variant: CapabilityVariant) => void
  /** 展开体的 DOM id：外框归卡片（本组件只画 `border-t` 分隔），
   *  这个 id 供头部 `aria-controls` 指向。单栏手风琴之后**没有"非嵌入"形态**，
   *  故它是必填——留成可选等于留一段永不执行的死分支。 */
  embeddedId: string
  t: ReturnType<typeof useI18n>["t"]
}) {
  const multi = cap.variants.length > 1
  // 排障信息默认收起：它是"出问题时才看"的东西，铺在详情面里会让每次选能力都要多滚一屏。
  // 层类能力：关闭 = 移除。这条**必须在按下开关之前就看得见**，否则用户以为能秒关。
  // 但只在"真的就位"（on/disabled）时才成立：`toggle_off_supported` 对**没装齐**的档
  // 也是 false（分类要读包自己的 manifest，没装就不知道），若照它判断，一次失败的安装
  // 会让面板谎称"本档由 profile 层提供"——真机 2026-09-16 暴露。
  const offIsRemove = offMeansRemove(target)
  // 被超集档包含的档不是"可切换的后端"：不给交互，免得用户以为点它能单独开关
  // （开关与移除统一由超集档控制，见 §⑤ 门禁）。
  const subsumedBy = target.subsumedBy ? variantDisplayName(cap, target.subsumedBy) : null
  const otherActive =
    cap.activeVariant !== null && cap.activeVariant !== target.id
      ? cap.variants.find((v) => v.id === cap.activeVariant)
      : undefined
  const blocked = Boolean(target.prerequisiteMissing)
  // 各档共有的前置 / 共用的基座包只讲一次（去掉 Chrome 前置与 dsh-browser-use 基座
  // 在三条变体行里各重复三遍的噪音——重复把"真正区分各档的那行"淹掉了）
  const commonPrereqs = commonPrerequisites(cap)
  const commonShared = commonSharedPackages(cap)
  // 变体行里只留"这一档额外带的"共用包：全体共有的那句已经在下面统一讲过一次。
  const extraShared = (shared: readonly string[]) =>
    shared.filter((p) => !commonShared.includes(p))
  // 所有变体的标识包共有的前缀（`@deepseek-ai/dsh-experimental-browser-use-`）：
  // 提到分节标题讲一次，卡内只留区分段——否则三张卡的首行都是同样的 40 个字符。
  const commonPrefix = commonPackagePrefix(
    cap.variants.flatMap((v) => variantPackageRoles(cap, v.id).primary),
  )

  return (
    <motion.section
      id={embeddedId}
      tabIndex={-1}
      aria-label={t.market.capPaneLabel(cap.label)}
      className="border-line/70 flex flex-col border-t"
    >
      {/* 展开体从"后端对照"直接开始（2026-09-21 三次修订）：原先这里还有一块身份头
          （图标 + 能力名 + 状态徽标 + summary 段落），但**卡片头部已经写着同样四样**——
          同一张卡里说两遍。身份块整体删掉，summary 移进头部的 ⓘ。
          展开体只留三件"要动手才需要"的事：状态/进度与失败、后端对照、排障细节。 */}
      <div className="flex flex-col gap-3 px-3 py-3">
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
              failedVariantId && failedVariantId !== target.id
                ? variantDisplayName(cap, failedVariantId)
                : null
            }
            onResume={() => onResume(target)}
            t={t}
          />
        )}

        {/* 恢复行只在**有事要做**时出现（没有它就整行不渲染，后果面因此更静） */}
        {!run &&
          (target.state === "partial" ||
            cap.state === "conflict" ||
            target.state === "disabled") && (
            <div className="flex flex-wrap items-center gap-2">
              {/* 与错误块里的「继续剩余步骤」是同一个动作 → 只在没有失败块时出现，避免两个按钮
                  指向同一件事（真机 2026-09-16 暴露）。
                  **前置门挡住时不给「修复」**（同日真机暴露）：修复=装包+写行，而这两个动作
                  后端都会拒绝——按钮点下去必失败，属于"假按钮"。真正的出路写在红字里
                  （先装 cua-driver / 换自包含档）。 */}
              {!subsumedBy &&
                !blocked &&
                !failure &&
                (target.state === "partial" || cap.state === "conflict") && (
                  <Button
                    size="sm"
                    variant="outline"
                    className="h-6 gap-1 border-warn/40 px-2 text-meta text-warn"
                    disabled={busy}
                    onClick={() => onRepair(target)}
                  >
                    <Wrench className="size-3" />
                    {t.market.capRepair}
                  </Button>
                )}
              {target.state === "disabled" && (
                <span className="text-label text-ok">{t.market.capReadyToEnable}</span>
              )}
            </div>
          )}

        {/* 后端对照（2026-09-21：v4 的"纯只读"改为**就地可换档**）。
            多变体时标题用「选择后端」而不是「插件」——三张卡是**互斥的选项**，
            叫"插件"会让用户以为它们是三个各自独立的插件（真机截图的困惑点之一）。
            单变体时仍叫「插件」（那时它确实只是"这个能力装了哪些包"）。 */}
        <div>
          <span className="text-meta font-medium tracking-wider text-faint flex items-center gap-1">
            {multi ? t.market.capMenuBackendLabel : t.market.capPluginsLabel}
            {/* 「互斥，切换即让位」是**选之前就该知道**的规则，原先挂在 `title` 上
                （鼠标可及、键盘不可及）。这里与其它说明统一收成 ⓘ。 */}
            {multi && (
              <Tip
                text={t.market.capBackendExclusiveNote}
                label={t.tip.ariaFor(t.market.capMenuBackendLabel)}
                className="max-w-64 text-label leading-relaxed"
              />
            )}
          </span>
          <ul className="mt-1.5 flex flex-col gap-1.5">
            {cap.variants.map((v) => {
              const roles = variantPackageRoles(cap, v.id)
              const isActive = cap.activeVariant === v.id
              const vBlocked = v.prerequisiteMissing
              const extras = v.prerequisites.filter((p) => !commonPrereqs.includes(p))
              return (
                <li
                  key={v.id}
                  className={`rounded-lg border px-3 py-2 ${
                    isActive ? "border-brand/40 bg-wash" : "border-line bg-panel"
                  }`}
                >
                  <span className="flex flex-wrap items-center gap-x-2 gap-y-1">
                    {/* 单选标记：一眼看出"这几张是**互斥的选项**，只能要一个"。
                        形状（实心/空心）而非仅颜色区分，色盲可辨。
                        生效档**不再挂「已启用」文字徽标**——左栏那一行已经写着同一件事，
                        同屏说两遍是最典型的冗余（2026-09-21 二次重构）。 */}
                    <span
                      title={
                        isActive
                          ? t.market.capVariantActive
                          : vBlocked || v.subsumedBy
                            ? undefined
                            : t.market.capSwitchToThis
                      }
                      className={`flex size-3.5 shrink-0 items-center justify-center rounded-full border ${
                        isActive ? "border-brand bg-brand" : "border-line bg-panel"
                      }`}
                    >
                      {isActive && <span className="bg-panel size-1.5 rounded-full" />}
                    </span>
                    {/* 包名去掉公共前缀（前缀已在分节标题讲过一次）：60 字符 → 15 字符，
                        三个后端一眼可辨。完整包名进 title，排查时可取。 */}
                    {roles.primary.map((pkg) => (
                      <span
                        key={pkg}
                        title={pkg}
                        className="font-mono text-label font-medium text-ink"
                      >
                        {packageTail(pkg, commonPrefix)}
                      </span>
                    ))}
                    {/* 后端说明（note）同样只留一颗 ⓘ：三张对照卡各铺一句描述，
                        正是"选项被描述淹没"的来源——选后端要看的是包名与前置，
                        不是三行广告词（2026-09-21 三次修订）。 */}
                    {v.note && (
                      <Tip
                        text={v.note}
                        label={t.tip.ariaFor(variantDisplayName(cap, v.id))}
                        className="max-w-72 text-label leading-relaxed"
                      />
                    )}
                    {v.subsumedBy && (
                      <Badge
                        variant="outline"
                        className="h-4.5 rounded-full border-line bg-wash px-1.5 text-micro font-normal text-dim"
                      >
                        {t.market.capStateSubsumed}
                      </Badge>
                    )}
                    {/* 2026-09-21 裁定变更：v4 曾定"右栏纯只读、换后端只走左栏溢出菜单"，
                        但真机上用户看到三张并列卡却点不动，只能回左栏找一个图标-only 的
                        入口——选项就在眼前却不能选，是反直觉的。现在对照面**就地可切**，
                        与左栏同一条 activate 链（该确认的确认），不是第二套路径。
                        **整卡不可点**（换后端要拆旧档、带确认，误触代价高）：可点的
                        只有这个按钮——所以卡片也不做 hover 高亮，免得暗示"点我有反应"。 */}
                    {!isActive && !vBlocked && !v.subsumedBy && (
                      <Button
                        size="xs"
                        variant="outline"
                        disabled={busy}
                        onClick={() => onSwitchBackend(v)}
                        className="ml-auto shrink-0 gap-1"
                      >
                        <ArrowLeftRight className="size-3" />
                        {t.market.capSwitchToThis}
                      </Button>
                    )}
                  </span>
                  {extraShared(roles.shared).length > 0 && (
                    <span className="mt-1 block break-all font-mono text-meta text-faint">
                      {t.market.capAlsoInstalls(extraShared(roles.shared).join(" · "))}
                    </span>
                  )}
                  {extras.length > 0 && (
                    /* 前置条件用 `·` 项目符号 + 小字 + 紧行距（2026-09-21 二次重构）：
                       原先每条挂一枚 ⓘ（占一列图标宽）且用正文号，三张卡里出现三次，
                       把右栏撑成一条长滚动带——而 stagehand 那两条本来就是**同一条
                       信息的两个后果**，紧凑列出比逐条立牌更好读。 */
                    <ul className="text-dim mt-1 flex flex-col gap-0.5">
                      {extras.map((p) => (
                        <li key={p} className="flex items-start gap-1.5 text-meta leading-snug">
                          <span aria-hidden="true" className="text-faint">
                            ·
                          </span>
                          <span>{p}</span>
                        </li>
                      ))}
                    </ul>
                  )}
                  {vBlocked && (
                    <span className="text-danger mt-1.5 flex items-start gap-1.5 text-label leading-relaxed">
                      <TriangleAlert className="mt-0.5 size-3 shrink-0" />
                      <span>{vBlocked}</span>
                    </span>
                  )}
                </li>
              )
            })}
          </ul>

          {/* 「所有档共有」的两种事实（共用基座包 / 共同前置）合并成一段紧凑列表：
              原先一个是光秃秃的 `ⓘ …`、另一个带独立小标题「各档共同前置」而其下只有
              一行——标题比内容重，还被夹在两张卡之间像第三张卡。 */}
          {(commonShared.length > 0 || commonPrereqs.length > 0) && (
            <ul className="text-faint mt-2 flex flex-col gap-0.5">
              {commonShared.length > 0 && (
                <li className="flex items-start gap-1.5 text-meta leading-snug">
                  <span aria-hidden="true">·</span>
                  <span className="break-all font-mono">
                    {t.market.capBasePackages(commonShared.join(" · "))}
                  </span>
                </li>
              )}
              {commonPrereqs.map((p) => (
                <li key={p} className="flex items-start gap-1.5 text-meta leading-snug">
                  <span aria-hidden="true">·</span>
                  <span>{p}</span>
                </li>
              ))}
            </ul>
          )}

          {/* 同族替换与"包含关系"：说清开关会连带动到谁（名字一律用插件名）。 */}
          {subsumedBy ? (
            <p className="text-faint mt-2 text-label leading-relaxed">
              {t.market.capSubsumedBy(subsumedBy)}
            </p>
          ) : (
            otherActive && (
              <p className="text-warn mt-2 text-label leading-relaxed">
                {otherActive.state === "on"
                  ? t.market.capOtherActive(variantDisplayName(cap, otherActive.id))
                  : t.market.capOtherReady(variantDisplayName(cap, otherActive.id))}
              </p>
            )
          )}
          {offIsRemove && (
            <p className="text-faint mt-1 text-label leading-relaxed">{t.market.capOffIsRemove}</p>
          )}
        </div>

        {/* "启用后会发生什么"（unlocks）已并入头部 ⓘ（与 summary 同一条悬浮）：
            它是一句价值描述，不是操作步骤——放在展开体里只会把后端对照挤下去。
            「实现细节（排障用）」整节已退役（2026-09-21 维护者真机裁定："实现细节这个
            板块我觉得应该也不需要了"）：它铺的是逐包安装状态 / 激活方式 / 包自带官方
            简介 / 未钉版本提示——**排障时才看**的东西，而排障现场已经有更准的两处：
            进度导轨（正在装哪个包）与失败块（一句人话 + 原始输出）。留一节的代价是
            每次展开都要多滚一屏，收益只是"偶尔查一次包版本"。 */}

        {/* 危险区沉底（`destructive-nav-separation`：破坏性动作与常规操作空间分离） */}
        {!subsumedBy &&
          (target.state === "on" ||
            target.state === "disabled" ||
            target.state === "partial") && (
            <section className="flex flex-wrap items-center justify-between gap-2 border-t border-line/70 pt-3">
              <span className="text-label leading-relaxed text-faint">
                {offMeansRemove(target)
                  ? t.market.capRemoveExplainedLayer
                  : t.market.capRemoveExplainedSoft}
              </span>
              <Button
                size="sm"
                variant="outline"
                className="h-6 shrink-0 border-danger/40 px-2 text-meta text-danger"
                disabled={busy}
                onClick={() => onRemove(target)}
              >
                {t.market.capRemoveBtn}
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

