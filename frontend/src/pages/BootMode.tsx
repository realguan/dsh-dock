// 运行环境选择页（原 ui/mode.html 升级重构，frontend-migration §4.2）。
// 平台语义保真：WSL 仅 Windows——非 Windows 到达本页时防御性跳回启动页。
import { useState } from "react"
import { useNavigate, Navigate } from "react-router-dom"
import { motion } from "framer-motion"
import { Laptop, TerminalSquare, ArrowRight, CheckCircle2 } from "lucide-react"
import { usePlatform } from "@/hooks/usePlatform"
import { t } from "@/content/zh-CN"
import { Emblem } from "@/components/layout/Emblem"
import { PageShell } from "@/components/layout/PageShell"
import { Switch } from "@/components/ui/switch"
import { Button } from "@/components/ui/button"

type Mode = "local" | "wsl"

export function BootMode() {
  const { can } = usePlatform()
  const navigate = useNavigate()
  const [picked, setPicked] = useState<Mode>("local")
  const [setDefault, setSetDefault] = useState(true)

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
      badge: "推荐 · 原生极速",
      icon: Laptop,
    },
    {
      mode: "wsl",
      name: t.mode.wsl,
      desc: t.mode.wslDesc,
      badge: "Linux 隔离环境",
      icon: TerminalSquare,
    },
  ]

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
                  ? "border-brand bg-panel shadow-[0_0_0_3px_rgba(65,118,230,0.15)] ring-1 ring-brand/50"
                  : "border-line bg-panel/90 hover:border-brand/40 hover:bg-wash/20 hover:shadow-sm"
              }`}
            >
              <div className="flex items-center justify-between">
                <span
                  className={`inline-flex size-10 items-center justify-center rounded-xl transition-colors ${
                    selected ? "bg-brand text-white shadow-xs" : "bg-line-soft text-dim group-hover:text-brand"
                  }`}
                >
                  <Icon className="size-5" />
                </span>

                <span
                  className={`rounded-full border px-2 py-0.5 text-[10px] font-semibold tracking-wide ${
                    selected
                      ? "border-brand/30 bg-wash text-brand-deep"
                      : "border-line bg-line-soft/60 text-faint"
                  }`}
                >
                  {badge}
                </span>
              </div>

              <div className="mt-4 flex-1">
                <span className="block text-[15px] font-semibold tracking-tight text-ink">
                  {name}
                </span>
                <span className="mt-1.5 block text-xs leading-relaxed text-dim">{desc}</span>
              </div>

              {selected && (
                <div className="mt-4 flex items-center gap-1 text-[11px] font-medium text-brand-deep">
                  <CheckCircle2 className="size-3.5 text-brand" />
                  <span>已选定此模式</span>
                </div>
              )}
            </motion.button>
          )
        })}
      </div>

      {/* 底部：设默认 Switch 行 + 开始 CTA */}
      <div className="mt-7 flex flex-col gap-4 rounded-2xl border border-line bg-panel/80 p-4 shadow-2xs backdrop-blur-xs sm:flex-row sm:items-center sm:justify-between">
        <label className="flex cursor-pointer items-center gap-3 select-none">
          <Switch checked={setDefault} onCheckedChange={setSetDefault} />
          <div>
            <span className="block text-xs font-medium text-ink">{t.mode.setDefault}</span>
            <span className="text-[11px] text-faint">随时可在设置或托盘菜单中更改</span>
          </div>
        </label>

        <Button
          type="button"
          disabled={!picked}
          onClick={() => {
            if (!picked) return
            navigate(
              `/?mode=${picked}&default=${setDefault ? "1" : "0"}`,
              { replace: true },
            )
          }}
          className="gap-2 rounded-full px-7 shadow-xs"
        >
          <span>{t.mode.next}</span>
          <ArrowRight className="size-4" />
        </Button>
      </div>
    </PageShell>
  )
}
