// 运行环境选择页（原 ui/mode.html 升级重构，frontend-migration §4.2）。
// 平台语义保真：WSL 仅 Windows——非 Windows 到达本页时防御性跳回启动页。
import { useState } from "react"
import { useNavigate, Navigate } from "react-router-dom"
import { motion } from "framer-motion"
import { Laptop, TerminalSquare, ArrowRight, CheckCircle2, Loader2, AlertCircle } from "lucide-react"
import { api } from "@/lib/tauri"
import { usePlatform } from "@/hooks/usePlatform"
import { useI18n } from "@/stores/i18nStore"
import { Emblem } from "@/components/layout/Emblem"
import { PageShell } from "@/components/layout/PageShell"
import { Switch } from "@/components/ui/switch"
import { Button } from "@/components/ui/button"

type Mode = "local" | "wsl"

export function BootMode() {
  const { t } = useI18n()
  const { can } = usePlatform()
  const navigate = useNavigate()
  const [picked, setPicked] = useState<Mode>("local")
  const [setDefault, setSetDefault] = useState(true)
  const [submitting, setSubmitting] = useState(false)
  const [errorMsg, setErrorMsg] = useState<string | null>(null)

  // 非 Windows（含 dev 预览）零 WSL 感知：回启动页，壳按 local 自启
  if (!can.chooseMode) return <Navigate to="/" replace />

  const cards: {
    mode: Mode
    name: string
    desc: string
    badge: string
    icon: typeof Laptop
  }[] = [
    {
      mode: "local",
      name: t.mode.local,
      desc: t.mode.localDesc,
      badge: t.mode.localBadge,
      icon: Laptop,
    },
    {
      mode: "wsl",
      name: t.mode.wsl,
      desc: t.mode.wslDesc,
      badge: t.mode.wslBadge,
      icon: TerminalSquare,
    },
  ]

  const handleStart = async () => {
    if (!picked || submitting) return
    setSubmitting(true)
    setErrorMsg(null)
    try {
      await api.chooseMode(picked, setDefault)
      navigate("/", { replace: true })
    } catch (err) {
      setSubmitting(false)
      setErrorMsg(String(err instanceof Error ? err.message : err))
    }
  }

  return (
    <PageShell width={620}>
      {/* 头部 */}
      <div className="mb-8 flex flex-col items-center gap-3 text-center">
        <div className="relative">
          <div className="absolute -inset-2 rounded-2xl bg-brand/10 blur-xl" />
          <Emblem size={56} />
        </div>
        <div>
          <h1 className="text-xl font-bold tracking-tight text-ink">{t.mode.title}</h1>
          <p className="mt-1.5 max-w-md text-xs leading-relaxed text-dim">{t.mode.subline}</p>
        </div>
      </div>

      {/* 双卡片选择阵列 */}
      <div className="grid gap-3.5 sm:grid-cols-2">
        {cards.map(({ mode, name, desc, badge, icon: Icon }, i) => {
          const selected = picked === mode
          return (
            <motion.button
              key={mode}
              type="button"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.06 * i, duration: 0.24, ease: "easeOut" }}
              whileHover={{ scale: 1.015, y: -2 }}
              whileTap={{ scale: 0.985 }}
              onClick={() => {
                setPicked(mode)
              }}
              aria-pressed={selected}
              className={`group relative flex flex-col rounded-2xl border p-5 text-left transition-all ${
                selected
                  ? "border-brand bg-panel ring-1 ring-brand/50 outline-3 outline-brand/15"
                  : "border-line bg-panel/90 hover:border-brand/40 hover:bg-wash/20 hover:shadow-sm"
              }`}
            >
              <div className="flex items-center justify-between">
                <span
                  className={`inline-flex size-10 items-center justify-center rounded-xl transition-colors ${
                    selected ? "bg-brand text-white shadow-xs" : "bg-line-soft text-dim group-hover:text-brand-deep"
                  }`}
                >
                  <Icon className="size-5" />
                </span>

                <span
                  className={`rounded-full border px-2 py-0.5 text-meta font-semibold tracking-wide ${
                    selected
                      ? "border-brand/30 bg-wash text-brand-deep"
                      : "border-line bg-line-soft/60 text-faint"
                  }`}
                >
                  {badge}
                </span>
              </div>

              <div className="mt-4 flex-1">
                <span className="block text-lead font-semibold tracking-tight text-ink">
                  {name}
                </span>
                <span className="mt-1.5 block text-xs leading-relaxed text-dim">{desc}</span>
              </div>

              {selected && (
                <div className="mt-4 flex items-center gap-1 text-label font-medium text-brand-deep">
                  <CheckCircle2 className="size-3.5 text-brand-deep" />
                  <span>{t.mode.selectedNotice}</span>
                </div>
              )}
            </motion.button>
          )
        })}
      </div>

      {/* 错误提示 */}
      {errorMsg && (
        <div className="mt-4 flex items-center gap-2 rounded-xl border border-destructive/30 bg-destructive/10 p-3 text-xs text-destructive">
          <AlertCircle className="size-4 shrink-0" />
          <span>{errorMsg}</span>
        </div>
      )}

      {/* 底部：设默认 Switch 行 + 开始 CTA */}
      <div className="mt-7 flex flex-col gap-4 rounded-2xl border border-line bg-panel/80 p-4 shadow-2xs backdrop-blur-xs sm:flex-row sm:items-center sm:justify-between">
        <label className="flex cursor-pointer items-center gap-3 select-none">
          <Switch
            aria-label={t.mode.setDefault}
            checked={setDefault}
            onCheckedChange={setSetDefault}
            disabled={submitting}
          />
          <div>
            <span className="block text-xs font-medium text-ink">{t.mode.setDefault}</span>
            <span className="text-label text-faint">{t.mode.changeAnytime}</span>
          </div>
        </label>

        <Button
          type="button"
          disabled={!picked || submitting}
          onClick={handleStart}
          className="gap-2 rounded-full px-7 shadow-xs"
        >
          {submitting ? (
            <>
              <Loader2 className="size-4 animate-spin" />
              <span>{t.mode.starting}</span>
            </>
          ) : (
            <>
              <span>{t.mode.next}</span>
              <ArrowRight className="size-4" />
            </>
          )}
        </Button>
      </div>
    </PageShell>
  )
}
