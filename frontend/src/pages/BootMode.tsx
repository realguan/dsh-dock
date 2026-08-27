// 运行环境选择页（原 ui/mode.html 完整迁移，frontend-migration §4.2）。
// 平台语义保真：WSL 仅 Windows——非 Windows 到达本页时防御性跳回启动页
// （旧页 location.replace 同语义；避免与壳启动线程的 choose_mode 双启动竞争）。
// 纯 vite dev 浏览器预览同样落到该兜底（host 未知 → can.chooseMode=false）。
import { useState } from "react"
import { useNavigate, Navigate } from "react-router-dom"
import { motion } from "framer-motion"
import { Laptop, TerminalSquare } from "lucide-react"
import { api } from "@/lib/tauri"
import { usePlatform } from "@/hooks/usePlatform"
import { t } from "@/content/zh-CN"
import { Emblem } from "@/components/layout/Emblem"
import { PageShell } from "@/components/layout/PageShell"

type Mode = "local" | "wsl"

export function BootMode() {
  const { can } = usePlatform()
  const navigate = useNavigate()
  const [picked, setPicked] = useState<Mode | null>(null)
  const [setDefault, setSetDefault] = useState(true)
  const [submitting, setSubmitting] = useState(false)
  const [failed, setFailed] = useState<string | null>(null)

  // 非 Windows（含 dev 预览）零 WSL 感知：回启动页，壳按 local 自启
  if (!can.chooseMode) return <Navigate to="/" replace />

  const cards: { mode: Mode; name: string; desc: string; icon: typeof Laptop }[] = [
    { mode: "local", name: t.mode.local, desc: t.mode.localDesc, icon: Laptop },
    { mode: "wsl", name: t.mode.wsl, desc: t.mode.wslDesc, icon: TerminalSquare },
  ]

  return (
    <PageShell width={560}>
      {/* 头部 */}
      <div className="mb-8 flex flex-col items-center gap-4 text-center">
        <Emblem size={52} />
        <div>
          <h1 className="text-ink text-xl font-semibold tracking-tight">{t.mode.title}</h1>
          <p className="text-faint mt-2 max-w-md text-sm leading-relaxed">{t.mode.subline}</p>
        </div>
      </div>

      {/* 双卡片选择 */}
      <div className="grid gap-3.5 sm:grid-cols-2">
        {cards.map(({ mode, name, desc, icon: Icon }, i) => {
          const selected = picked === mode
          return (
            <motion.button
              key={mode}
              type="button"
              initial={{ opacity: 0, y: 10 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: 0.06 * i, duration: 0.24, ease: "easeOut" }}
              whileTap={{ scale: 0.985 }}
              onClick={() => {
                setPicked(mode)
                setFailed(null)
              }}
              aria-pressed={selected}
              className={`bg-panel rounded-xl border p-4 text-left transition-all ${
                selected
                  ? "border-brand shadow-[0_0_0_3px_rgba(65,118,230,0.16)]"
                  : "border-line hover:border-faint/60 hover:shadow-sm"
              }`}
            >
              <span
                className={`mb-2.5 inline-flex size-8 items-center justify-center rounded-lg ${
                  selected ? "bg-wash text-brand-deep" : "bg-line-soft text-dim"
                }`}
              >
                <Icon className="size-4" />
              </span>
              <span className="text-ink block text-[15px] font-semibold">{name}</span>
              <span className="text-dim mt-1.5 block text-xs leading-relaxed">{desc}</span>
              {selected && (
                <motion.span
                  layoutId="mode-sel"
                  className="text-brand-deep mt-2 block text-[11px] font-medium"
                >
                  已选择
                </motion.span>
              )}
            </motion.button>
          )
        })}
      </div>

      {/* 底部：设默认 + 开始 */}
      <div className="mt-7 flex items-center justify-center gap-5">
        <label className="text-dim flex cursor-pointer items-center gap-2 text-sm select-none">
          <input
            type="checkbox"
            checked={setDefault}
            onChange={(e) => setSetDefault(e.target.checked)}
            className="accent-brand size-3.5"
          />
          {t.mode.setDefault}
        </label>
        <button
          type="button"
          disabled={!picked || submitting}
          onClick={() => {
            if (!picked) return
            setSubmitting(true)
            api
              .chooseMode(picked, setDefault)
              .then(() => navigate("/", { replace: true }))
              .catch((e) => {
                setSubmitting(false)
                setFailed(String(e instanceof Error ? e.message : e))
              })
          }}
          className="bg-brand rounded-full px-8 py-2 text-sm font-medium text-white transition-opacity hover:opacity-90 disabled:cursor-default disabled:opacity-40"
        >
          {submitting ? t.mode.starting : t.mode.next}
        </button>
      </div>

      {failed && (
        <p role="alert" className="text-warn mt-4 text-center text-sm">
          {t.mode.failed}：{failed}
        </p>
      )}
    </PageShell>
  )
}
