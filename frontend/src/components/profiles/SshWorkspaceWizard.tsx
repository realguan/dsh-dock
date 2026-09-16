// SshWorkspaceWizard.tsx —— SSH 远程工作区向导（2026-09-15，ADR-0023 方案 A）。
//
// 三步，顺序不可换（每一步的后置条件都是下一步的前置）：
//   ① 选 alias（来自 `~/.ssh/config`，Rust 侧解析）＋ 填四个远端路径/摘要；
//   ② **非交互预检**（`ssh -o BatchMode=yes`）——不通过就**不往下走**；
//   ③ 逐包装四个 ssh 包（复用既有安装队列）→ 写四行注册行（`generateSshProfile`）。
//
// ## 两条必须显式告诉用户的事（不说就是骗人）
//
// 1. **范围**：本功能面向 **headless / 自建 profile**。把 ssh 家族装进 `web` profile
//    **不会**让 Web 工作台的文件树/编辑器/终端变成远端感知——上游明写
//    "replacing providers alone does **not** make those views remote-aware"。
//    因此生成的 profile 声明的是 `@deepseek-ai/dsh-headless` 而非 web-app。
// 2. **平台**：两端须 Linux/macOS（上游在非 POSIX 宿主上直接抛错）。Windows 宿主
//    上本向导**不可用**，且这不是壳的额外限制。
//
// ## 第 ③ 步为什么是**一个**后端命令（而不是复用安装队列）
//
// 两条硬约束：① 挂载行必须在四包装完之后才写（反过来会留下指向未安装包的幽灵
// 挂载行，dsh 启动即加载失败）；② 四个包必须**钉运行期版本**，而版本只有后端有
// （裸包名按 `latest` 装，`@deepseek-ai/*` 的 latest 已实测会落后）。
// 代价是这一步**可能数分钟且没有逐包进度**——故界面必须给出明确的进行中文案，
// 不能只转圈（见 `sshInstalling`）。
import { useCallback, useEffect, useMemo, useState } from "react"
import { AlertTriangle, Check, LoaderCircle, Terminal, X } from "lucide-react"
import { api } from "@/lib/tauri"
import { useI18n } from "@/stores/i18nStore"
import type { SshHost, SshProbe, SshProbeCheck, SshTarget } from "@/types/ipc"
import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"

type Phase =
  | { kind: "form" }
  | { kind: "probing" }
  | { kind: "probeFailed"; error: string }
  | { kind: "probeOk"; probe: SshProbe }
  | { kind: "installing" }
  | { kind: "rows"; created: boolean; changed: boolean }

const EMPTY_TARGET: SshTarget = {
  host: "",
  node: "",
  helper: "",
  helperHash: "",
  workspace: "",
}

/** 平台判定：与后端 `ssh_remote::host_supports_ssh` 同口径（非 Windows）。
 *  用 `navigator.userAgent` 而非 Tauri API：这是**能力提示**，不是安全边界
 *  ——真正的闸门在后端（`probe_ssh_target` 会拒绝）。 */
function hostIsPosix(): boolean {
  if (typeof navigator === "undefined") return true
  return !/Windows/i.test(navigator.userAgent)
}

export function SshWorkspaceWizard({
  open,
  profileName,
  onClose,
  onRefresh,
}: {
  open: boolean
  /** 已知 profile 名列表（向导要求选一个**已存在或将要创建**的名字）。 */
  profileName: string
  onClose: () => void
  onRefresh: () => void
}) {
  const { t } = useI18n()
  const [hosts, setHosts] = useState<SshHost[] | null>(null)
  const [hostNotes, setHostNotes] = useState<string[]>([])
  const [loadError, setLoadError] = useState<string | null>(null)
  const [name, setName] = useState(profileName)
  const [target, setTarget] = useState<SshTarget>(EMPTY_TARGET)
  const [phase, setPhase] = useState<Phase>({ kind: "form" })
  const posix = useMemo(hostIsPosix, [])

  const reset = useCallback(() => {
    setName(profileName)
    setTarget(EMPTY_TARGET)
    setPhase({ kind: "form" })
  }, [profileName])

  // 打开时读一次 `~/.ssh/config`：这是**唯一**能拿到可选 alias 的途径
  // （前端不得自行解析——ADR-0023 §3 方案 D 已否决）。
  useEffect(() => {
    if (!open) return
    let cancelled = false
    api
      .listSshHosts()
      .then((r) => {
        if (cancelled) return
        setHosts(r.hosts)
        setHostNotes(r.notes)
      })
      .catch((e: unknown) => {
        if (cancelled) return
        setHosts([])
        setLoadError(String(e))
      })
    return () => {
      cancelled = true
    }
  }, [open])

  const busy = phase.kind === "probing" || phase.kind === "installing" || phase.kind === "rows"

  const close = () => {
    if (busy) return
    reset()
    onClose()
  }

  const probe = async () => {
    setPhase({ kind: "probing" })
    try {
      // 预检**不通过**也走 `probeOk` 相位：逐项红/绿比一句"失败了"有用得多
      // （用户要的是"哪一项不对、观测值是什么"）。`probeFailed` 只留给
      // IPC 层错误——那种情况连逐项结果都拿不到。
      setPhase({ kind: "probeOk", probe: await api.probeSshTarget(target) })
    } catch (e) {
      setPhase({ kind: "probeFailed", error: String(e) })
    }
  }

  /** 生成：建 profile → 装四包（钉运行期版本，后端做）→ 写四行 → 写后自证。
   *  失败即整体报错，**不会**留下"有行无包"的中间态。 */
  const generate = async () => {
    setPhase({ kind: "installing" })
    try {
      const out = await api.generateSshProfile(name.trim(), target)
      setPhase({ kind: "rows", created: out.created, changed: out.changed })
      onRefresh()
    } catch (e) {
      setPhase({ kind: "probeFailed", error: String(e) })
    }
  }

  const canProbe = target.host !== "" && !busy

  return (
    <Dialog open={open} onOpenChange={(o) => !o && close()}>
      <DialogContent className="sm:max-w-[560px]">
        <DialogHeader>
          <DialogTitle>{t.profiles.sshWizardTitle}</DialogTitle>
          <DialogDescription className="text-dim text-xs">
            {t.profiles.sshWizardScope}
          </DialogDescription>
        </DialogHeader>

        {!posix && (
          <div className="bg-warn-soft text-warn rounded-lg px-3 py-2.5 text-xs">
            {t.profiles.sshWizardWindows}
          </div>
        )}

        <div className="space-y-3">
          <label className="text-dim block text-xs" htmlFor="ssh-profile-name">
            {t.profiles.sshNameLabel}
          </label>
          <input
            id="ssh-profile-name"
            value={name}
            disabled={busy}
            onChange={(e) => setName(e.target.value)}
            placeholder="ssh-remote"
            className="border-line bg-bg text-ink placeholder:text-faint focus:border-brand w-full rounded-lg border px-3 py-2 font-mono text-sm outline-none transition-colors disabled:opacity-50"
          />

          <label className="text-dim block text-xs" htmlFor="ssh-host-select">
            {t.profiles.sshHostLabel}
          </label>
          {loadError ? (
            <p className="text-danger text-xs">{t.profiles.sshHostLoadFailed(loadError)}</p>
          ) : (
            <select
              id="ssh-host-select"
              value={target.host}
              disabled={busy}
              onChange={(e) => setTarget((s) => ({ ...s, host: e.target.value }))}
              className="border-line bg-bg text-ink focus:border-brand w-full rounded-lg border px-3 py-2 font-mono text-xs outline-none transition-colors disabled:opacity-50"
            >
              <option value="">{t.profiles.sshHostPlaceholder}</option>
              {(hosts ?? []).map((h) => (
                <option key={h.alias} value={h.alias}>
                  {h.user ? `${h.alias} → ${h.user}@` : `${h.alias} → `}
                  {h.hostname ?? h.alias}
                  {h.port ? `:${h.port}` : ""}
                </option>
              ))}
            </select>
          )}
          {/* 「列表可能不完整」必须显示：未跟随 Include / 忽略 Match 都会少主机，
              不说明的话用户会以为自己的配置没生效。 */}
          {hostNotes.length > 0 && (
            <ul className="text-warn space-y-0.5 text-meta">
              {hostNotes.map((n) => (
                <li key={n}>{n}</li>
              ))}
            </ul>
          )}

          {(
            [
              ["node", t.profiles.sshNodeLabel, "/usr/local/bin/node"],
              ["helper", t.profiles.sshHelperLabel, "/opt/dsh/helper.js"],
              ["helperHash", t.profiles.sshHashLabel, "64 位小写十六进制"],
              ["workspace", t.profiles.sshWorkspaceLabel, "/home/deploy/work"],
            ] as const
          ).map(([key, label, placeholder]) => (
            <div key={key}>
              <label className="text-dim block text-xs" htmlFor={`ssh-${key}`}>
                {label}
              </label>
              <input
                id={`ssh-${key}`}
                value={target[key]}
                disabled={busy}
                onChange={(e) => setTarget((s) => ({ ...s, [key]: e.target.value }))}
                placeholder={placeholder}
                className="border-line bg-bg text-ink placeholder:text-faint focus:border-brand mt-1 w-full rounded-lg border px-3 py-2 font-mono text-xs outline-none transition-colors disabled:opacity-50"
              />
            </div>
          ))}
        </div>

        {/* 预检结果：逐项列出「通过/不通过 + 实际观测值」。只说"不通过"等于让用户猜。 */}
        {phase.kind === "probeOk" && (
          <div className="space-y-1.5">
            {phase.probe.checks.map((c) => (
              <CheckRow key={c.key} check={c} />
            ))}
          </div>
        )}
        {phase.kind === "probeFailed" && (
          <div className="bg-danger-soft text-danger flex items-start gap-1.5 rounded-lg px-3 py-2.5 text-xs">
            <AlertTriangle className="mt-0.5 size-3 shrink-0" />
            <span>
              {t.profiles.sshProbeFailed}
              {phase.error ? `：${phase.error}` : ""}
            </span>
          </div>
        )}
        {phase.kind === "installing" && (
          <div className="bg-wash text-dim rounded-lg px-3 py-2.5 text-xs">
            <LoaderCircle className="mr-1.5 inline size-3 animate-spin" />
            {t.profiles.sshInstalling}
          </div>
        )}
        {phase.kind === "rows" && (
          <div className="bg-ok-soft text-ok rounded-lg px-3 py-2.5 text-xs">
            {phase.created
              ? t.profiles.sshDoneCreated(name.trim())
              : t.profiles.sshDoneReused(name.trim())}
            {" "}
            {phase.changed ? t.profiles.sshDoneWrote : t.profiles.sshDoneUnchanged}
          </div>
        )}

        <DialogFooter>
          {/* 四包必须全部装成才能写行 → 只有预检全绿才放开安装。 */}
          {phase.kind === "probeOk" && phase.probe.ok && (
            <Button onClick={() => void generate()} disabled={busy || name.trim() === ""}>
              <Terminal className="size-3.5" />
              {t.profiles.sshInstallBtn}
            </Button>
          )}
          {phase.kind !== "probeOk" && (
            <Button onClick={() => void probe()} disabled={!canProbe || !posix}>
              {t.profiles.sshProbeBtn}
            </Button>
          )}
          <Button variant="outline" onClick={close} disabled={busy}>
            {t.profiles.detailClose}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

/** 一项预检结论。`check.key` 是稳定的机器可读键——图标只看它，不解析文案。 */
function CheckRow({ check }: { check: SshProbeCheck }) {
  return (
    <div className="flex items-start gap-1.5 text-xs">
      {check.ok ? (
        <Check className="text-ok mt-0.5 size-3 shrink-0" />
      ) : (
        <X className="text-danger mt-0.5 size-3 shrink-0" />
      )}
      <span className={check.ok ? "text-dim" : "text-danger"}>{check.detail}</span>
    </div>
  )
}
