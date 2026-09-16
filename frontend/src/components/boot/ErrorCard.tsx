// 错误卡（数据驱动渲染，原 index/selector showError 升级迁移）：
// actions[] 由后端下发 id 集合，前端只做 id→文案映射（未知 id 回退展示原文）；
// 调用方追加本地动作 reselect（返回重选）。支持一键复制诊断日志与微交互。
import { useState } from "react"
import { motion } from "framer-motion"
import {
  AlertTriangle,
  ChevronDown,
  Sparkles,
  Copy,
  Check,
  RefreshCw,
  ShieldOff,
  Terminal,
} from "lucide-react"
import { api } from "@/lib/tauri"
import { logger } from "@/lib/logger"
import { useCopy } from "@/hooks/useCopy"
import type { TerminalAction } from "@/types/ipc"
import type { BootErrorEvent } from "@/types/events"
import { useI18n } from "@/stores/i18nStore"
import { useBootStore } from "@/stores/bootStore"
import { Button } from "@/components/ui/button"
import { ConfirmDialog } from "@/components/ui/confirm-dialog"

/**
 * 动作 id → IPC 分派表（2026-09-11，task-52 / T-F5）。
 *
 * **为什么抽成表**：改前 `run()` 是 `if (!INVOKABLE.has(id)) return` +
 * `api.terminalAction(id as TerminalAction)` —— 一条**只认 terminalAction** 的隐式假设。
 * rust-core 的 D1 给错误卡下发了 `actions=["boot_in_wsl","retry"]`（Windows 本地模式
 * 因符号链接特权失败 → 引导改用 WSL），而 `boot_in_wsl` 是**另一条 IPC**
 * （`api.bootInWsl()`），既不在 INVOKABLE 里、也不是 `TerminalAction` 的成员 ⇒
 * `run()` 直接 return ⇒ **按钮点了没反应（假按钮，比没有更糟）**。
 *
 * 现在分派是**显式**的：每个可点动作都必须在此登记目标 IPC，未知 id 才 return。
 * 契约来源：`docs/team/V120-D1-引擎引导全局安装.md` §5、
 * `src-tauri/src/boot_failure.rs:135`（`vec!["boot_in_wsl", "retry"]`）。
 */
export type ActionIpc = "terminalAction" | "bootInWsl" | "quarantineRow"

/** 可点动作 → 目标 IPC 的**唯一事实源**（`INVOKABLE_ACTIONS` 由它派生，两处不漂移）。 */
const ACTION_IPC: Readonly<Record<string, ActionIpc>> = {
  retry: "terminalAction",
  upgrade: "terminalAction",
  upgrade_only: "terminalAction",
  boot_in_wsl: "bootInWsl",
  // 2026-09-16：插件行把插件树搞挂时的**就地**出路——移除该行 + 重启（而不是
  // 让用户自己去别处找）。契约：`boot_failure.rs::with_quarantine`。
  quarantine_plugin_row: "quarantineRow",
  // 安全模式（ADR-0025，2026-09-16）：`safe_mode` 停用非随包层行后重启（零文件改动）；
  // `safe_mode_reset` 是"行枚举不出来"时（patch 语法坏）的兜底——**必须先过确认框**
  // （它会备份并放空用户的 cordis.patch.yml），故走本地 `needsConfirm` 分支。
  safe_mode: "terminalAction",
  safe_mode_reset: "terminalAction",
}

/** 可由本组件分派的动作 id 集合（从分派表派生，避免"集合/分派"两处漂移）。 */
export const INVOKABLE_ACTIONS: ReadonlySet<string> = new Set(Object.keys(ACTION_IPC))

/**
 * 纯函数：动作 id → 该调哪条 IPC。未知 id 返回 `null`（调用方不猜、保持原姿态）。
 * 抽成纯函数是为了可测——本仓禁 RTL/jsdom（AGENTS §4.3），否则"假按钮"这类
 * 「接线存在但目标错」的缺陷无法被机器化拦住。
 */
export function resolveActionCall(id: string): { kind: "invoke"; ipc: ActionIpc } | null {
  const ipc = ACTION_IPC[id]
  if (!ipc) return null
  return { kind: "invoke", ipc }
}

export function ErrorCard({
  payload,
  onReselect,
  diag = false,
  index,
}: {
  payload: BootErrorEvent
  /** 本页本地动作：返回重选（旧页 = location.reload()） */
  onReselect?: () => void
  /** diag 形态（启动页内嵌）：DIAG 头行 + 日志默认展开；边框收进容器 */
  diag?: boolean
  /** diag 头行右侧的 #NN 序号（多次错误自增，由父级计数） */
  index?: number
}) {
  const { t } = useI18n()
  const [pending, setPending] = useState<string | null>(null)
  const [actionError, setActionError] = useState<string | null>(null)
  const { copied, copy } = useCopy()
  // 诊断卡折叠态（2026-09-16 维护者裁定，取代 v1.2.0 的"默认展开"）：
  // **异常态首屏给的是"能点的动作 + 这个动作会带来什么"**，诊断细节是"需要时才看"，
  // 故默认收起。错误事实本身不会被藏：标题、一行摘要与全部动作永远在收起态里可见。
  const [collapsed, setCollapsed] = useState(true)
  // 「安全模式（备份并放空 patch）」会动用户的 cordis.patch.yml → **必须先确认**
  // （ADR-0025 §4 方案 B；本仓对破坏性动作一律走 ConfirmDialog）。
  const [resetSafeModeOpen, setResetSafeModeOpen] = useState(false)
  // `?? ["retry"]`（不是 `?.length ?`）——**空数组是后端明确说"没有可行动作"**：
  // 插件的挂载行把插件树搞挂时重试必然再失败，摆一个必然失败的按钮等于教用户白点一次。
  // 旧缓存载荷（无该字段）才回退「重试」。
  const actions = payload.actions ?? ["retry"]
  // 2026-09-08（ADR-0012）：优先按结构化分类取本地化文案；后端文案作为兼容分支
  // （旧缓存载荷 / 未来新增的未识别 kind）。
  const kindCopy = payload.failure ? t.error.kinds[payload.failure.kind] : undefined
  const title = kindCopy?.title ?? payload.title ?? t.error.fallbackTitle
  const suggestion = kindCopy?.suggestion ?? payload.suggestion

  const actionLabel = (id: string): string => {
    return t.error.actions[id] ?? id
  }

  /// 动作 → **它会造成什么**（首屏必须看得见，用户不该靠点一下才知道代价）。
  const actionImpact = (id: string): string | undefined => t.error.impacts[id]
  /// 一行"发生了什么"：收起态也看得见事实，细节留给展开。
  const reason = payload.detail?.split("\n").map((l) => l.trim()).find(Boolean)

  const run = (id: string) => {
    if (pending) return
    if (id === "safe_mode_reset") {
      setResetSafeModeOpen(true)
      return
    }
    doRun(id)
  }

  const doRun = (id: string) => {
    if (pending) return
    // 显式分派（task-52）：未知 id 才放弃；已知 id 各自走正确的那条 IPC。
    const call = resolveActionCall(id)
    if (!call) return
    // 隔离计划由后端下发（只有它知道行归谁、当前 profile 是哪个）；缺失即**不调**
    // ——宁可不给按钮，也不能拿半个计划去删行（会删错 profile）。
    const plan = payload.quarantine
    if (call.ipc === "quarantineRow" && !plan) return
    setPending(id)
    setActionError(null)
    useBootStore.getState().clearError()
    // `boot_in_wsl` 不是 terminalAction 的成员（TerminalAction 联合类型不含它），
    // 故必须按分派目标分别调用，不能统一 `id as TerminalAction` 蒙混过去。
    // 隔离是**两步一次点击**：先移除该行，再重启。移除失败就不重启——重启也还是
    // 那个坏 profile，只会把用户再带回同一张错误卡。
    const invoke =
      call.ipc === "bootInWsl"
        ? api.bootInWsl()
        : call.ipc === "quarantineRow" && plan
          ? api.removeOfficialPatchRow(plan.profile, plan.rowId).then(() => api.terminalAction("retry"))
          : api.terminalAction(id as TerminalAction)
    invoke
      .catch((e) => {
        setPending(null)
        const msg = String(e instanceof Error ? e.message : e)
        setActionError(`${t.error.actionFailedDetail(msg)}${t.error.reselectHint}`)
      })
      .finally(() => setPending((p) => (p === id ? null : p)))
  }

  // 2026-09-08：原写法连 promise 都没接——写失败照样显示「已复制」。
  const handleCopyLog = async () => {
    if (!payload.log) return
    const outcome = await copy(payload.log)
    if (!outcome.ok) logger.warn("[boot]", "复制诊断日志失败", { error: outcome.error })
  }

  return (
    <motion.section
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.25, ease: "easeOut" }}
      className={`overflow-hidden rounded-2xl border bg-panel shadow-sm ${
        diag ? "w-full border-danger/30" : "mx-auto mt-6 w-full max-w-xl border-danger/35 shadow-md"
      }`}
      role="alert"
      data-failure-kind={payload.failure?.kind ?? "none"}
    >
      {/* 诊断状态头 */}
      <div className="flex items-center justify-between border-b border-danger/20 bg-danger-soft/40 px-4 py-2.5">
        <div className="flex items-center gap-2">
          <span className="flex size-5 items-center justify-center rounded-full bg-danger text-white">
            <AlertTriangle className="size-3" />
          </span>
          <span className="font-mono text-xs font-semibold tracking-wide text-danger">
            {diag ? t.error.diagHeader : t.error.cardHeader}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <span className="font-mono text-label font-medium text-dim tabular-nums">
            #{typeof index === "number" ? String(index).padStart(2, "0") : "01"}
          </span>
          {/* 收起/展开出口（v1.2.0 实测 2.1）：诊断卡不再是"只能看不能关"的一块。
              **不误藏**：收起后卡头（警示图标 + 标题 + 序号 + 展开按钮）仍在原位，
              用户随时能展开回来——折的是内容，不是"这里出过错"这一事实。 */}
          <button
            type="button"
            aria-expanded={!collapsed}
            title={collapsed ? t.error.expandDetail : t.error.collapseDetail}
            onClick={() => setCollapsed((c) => !c)}
            className="inline-flex items-center gap-1 rounded-md px-1.5 py-0.5 font-mono text-label font-medium text-dim transition-colors hover:text-ink"
          >
            <ChevronDown
              className={`size-3.5 transition-transform ${collapsed ? "" : "rotate-180"}`}
            />
            {collapsed ? t.error.expandDetail : t.error.collapseDetail}
          </button>
        </div>
      </div>

      <div className="p-5">
        <h2 className="text-base font-semibold tracking-tight text-ink">{title}</h2>

        {/* 一行"发生了什么"：收起态也看得见错误事实 */}
        {reason && <p className="mt-1 truncate text-xs text-dim">{reason}</p>}

        {/* 首屏主角 = 可点的动作 + **它会造成什么**（维护者 2026-09-16 裁定）。
            动作集为空 = 后端明确说"没有可点的出路"，此时不留空行。 */}
        {(actions.length > 0 || onReselect) && (
          <div className="mt-3.5 flex flex-col gap-2">
            {actions.map((a) => {
              const isPrimary = a === "retry" || a === "upgrade" || a === "upgrade_only" || a === "safe_mode"
              // 隔离 = 移除出问题的那一行：图标要能一眼区分于"重试"（它做的事不同，
              // 而且会改动 profile 文件）。
              const ActionIcon =
                a === "quarantine_plugin_row" || a === "safe_mode" ? ShieldOff : RefreshCw
              const impact = actionImpact(a)
              return (
                <div key={a} className="flex flex-wrap items-baseline gap-x-2.5 gap-y-1">
                  <Button
                    size="sm"
                    variant={isPrimary ? "default" : "outline"}
                    disabled={pending !== null}
                    onClick={() => run(a)}
                    className={`gap-1.5 ${a === "safe_mode_reset" ? "border-danger/40 text-danger" : ""}`}
                  >
                    {pending === a ? (
                      <>
                        <RefreshCw className="size-3.5 animate-spin" />
                        <span>{actionLabel(a)}…</span>
                      </>
                    ) : (
                      <>
                        <ActionIcon className="size-3.5" />
                        <span>{actionLabel(a)}</span>
                      </>
                    )}
                  </Button>
                  {impact && <span className="text-micro text-faint">{impact}</span>}
                </div>
              )
            })}
            {onReselect && (
              <div className="flex items-baseline gap-2.5">
                <Button
                  size="sm"
                  variant="outline"
                  disabled={pending !== null}
                  onClick={onReselect}
                  className="text-dim"
                >
                  {t.error.actions.reselect}
                </Button>
                {actionImpact("reselect") && (
                  <span className="text-micro text-faint">{actionImpact("reselect")}</span>
                )}
              </div>
            )}
          </div>
        )}

        {actionError && (
          <div className="mt-3 rounded-lg bg-danger/10 p-2.5 text-xs text-danger break-words">
            {actionError}
          </div>
        )}

        <ConfirmDialog
          open={resetSafeModeOpen}
          title={t.error.safeModeResetTitle}
          note={t.error.safeModeResetNote}
          points={[t.error.safeModeResetPointBackup, t.error.safeModeResetPointScope]}
          confirmLabel={t.error.safeModeResetConfirm}
          cancelLabel={t.confirm.cancel}
          busy={pending === "safe_mode_reset"}
          onConfirm={() => {
            setResetSafeModeOpen(false)
            doRun("safe_mode_reset")
          }}
          onClose={() => setResetSafeModeOpen(false)}
        />

        {/* 细节默认收起（标题/摘要/动作之外的都在这）*/}
        {!collapsed && (
        <div className="mt-3 flex flex-col gap-3">
        {/* 错误详情 */}
        {payload.detail && (
          <div className="mt-2.5 rounded-xl border border-danger/20 bg-danger-soft/30 p-3 text-xs leading-relaxed text-dim break-words">
            {payload.detail}
          </div>
        )}

        {/* 建议解决方案 */}
        {suggestion && (
          <div className="mt-3 flex items-start gap-2.5 rounded-xl border border-brand/20 bg-wash/70 p-3 text-xs leading-relaxed text-dim">
            <Sparkles className="mt-0.5 size-3.5 shrink-0 text-brand-deep" />
            <div className="flex-1">
              <span className="font-semibold text-brand-deep">{t.error.suggestionLabel}</span>
              {suggestion}
            </div>
          </div>
        )}

        {/* 原始终端日志折叠 */}
        {payload.log && (
          <details open={diag} className="group mt-4 overflow-hidden rounded-xl border border-line">
            <summary className="flex cursor-pointer select-none items-center justify-between bg-muted/40 px-3.5 py-2 text-xs text-dim transition-colors hover:bg-muted/70">
              <div className="flex items-center gap-1.5 font-mono text-label">
                <Terminal className="size-3 text-faint" />
                <span>
                  {t.error.rawLogSummary(payload.log.split("\n").filter(Boolean).length)}
                </span>
              </div>
              <ChevronDown className="size-3.5 text-faint transition-transform group-open:rotate-180" />
            </summary>
            <div className="relative border-t border-term-line bg-term p-3">
              <button
                type="button"
                onClick={handleCopyLog}
                className="absolute top-2.5 right-2.5 inline-flex items-center gap-1 rounded-md border border-white/10 bg-white/10 px-2 py-1 font-mono text-meta text-white/80 transition-colors hover:bg-white/20"
                title={t.error.copyLog}
              >
                {copied ? (
                  <>
                    <Check className="size-3 text-term-ok" />
                    <span>{t.boot.copied}</span>
                  </>
                ) : (
                  <>
                    <Copy className="size-3" />
                    <span>{t.error.copyLog}</span>
                  </>
                )}
              </button>
              <pre className="max-h-60 overflow-x-auto font-mono text-label leading-relaxed text-term-ok whitespace-pre-wrap">
                {payload.log}
              </pre>
            </div>
          </details>
        )}
        </div>
        )}
      </div>
    </motion.section>
  )
}

