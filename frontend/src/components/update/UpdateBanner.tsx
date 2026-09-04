// 窗口内非阻断升级提示条（ADR-0010 升级呈现台账行，2026-09-04）：新版首次
// 出现时显示，「忽略此版本」持久化到 settings.dismissedUpdate（同键不再弹、
// 新版本键不受影响），拒绝无硬惩罚。更新动作本身归关于窗口更新中心
//（ADR-0007：入口在菜单/托盘），本条只做告知与降噪。
import { useEffect, useState } from "react"
import { AnimatePresence, motion } from "framer-motion"
import { X } from "lucide-react"
import { api } from "@/lib/tauri"
import { useI18n } from "@/stores/i18nStore"
import { useBootStore } from "@/stores/bootStore"
import {
  updateBanners,
  type BannerSpec,
} from "@/lib/updateBanner"

export function UpdateBanner() {
  const { t } = useI18n()
  const versions = useBootStore((s) => s.versions)
  const [dismissed, setDismissed] = useState<string | null | undefined>(undefined)

  // 播种已忽略键（undefined = 未加载，加载前不出条避免闪现）
  useEffect(() => {
    let alive = true
    api
      .getShellSettings()
      .then((s) => alive && setDismissed(s.dismissedUpdate ?? null))
      .catch(() => alive && setDismissed(null))
    return () => {
      alive = false
    }
  }, [])

  const banners = dismissed === undefined ? [] : updateBanners(versions, dismissed)

  const dismiss = (spec: BannerSpec) => {
    setDismissed(spec.key)
    // 落盘前重读最新设置再合并，避免盖掉其他窗口刚写的偏好字段
    api
      .getShellSettings()
      .then((s) => api.setShellSettings({ ...s, dismissedUpdate: spec.key }))
      .catch(() => {})
  }

  return (
    <AnimatePresence>
      {banners.length > 0 && (
        <motion.div
          key={banners.map((b) => b.key).join("+")}
          initial={{ opacity: 0, y: -8 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -8 }}
          transition={{ duration: 0.22, ease: "easeOut" }}
          className="pointer-events-auto w-full max-w-md rounded-xl border border-warn/30 bg-panel/95 p-3.5 shadow-lg backdrop-blur-md"
          role="status"
        >
          <div className="flex flex-col gap-2">
            {banners.map((b) => (
              <div key={b.key} className="flex items-start gap-2.5">
                <span className="mt-1.5 size-2 shrink-0 animate-pulse rounded-full bg-warn ring-2 ring-warn/25" />
                <div className="min-w-0 flex-1">
                  <p className="text-[13px] font-medium leading-snug text-ink">
                    {b.kind === "dsh"
                      ? t.updateBanner.dshTitle.replace("{latest}", b.latest).replace("{current}", b.current ?? "?")
                      : t.updateBanner.clientTitle.replace("{latest}", b.latest)}
                  </p>
                  <p className="mt-0.5 text-xs leading-relaxed text-dim">
                    {b.kind === "dsh"
                      ? t.updateBanner.dshConsequence
                      : t.updateBanner.clientConsequence}
                  </p>
                </div>
                <button
                  type="button"
                  title={t.updateBanner.dismissTip}
                  onClick={() => dismiss(b)}
                  className="rounded-md p-1 text-faint transition-colors hover:bg-line/60 hover:text-dim"
                >
                  <X className="size-3.5" />
                </button>
              </div>
            ))}
            <p className="text-[11px] leading-relaxed text-faint">{t.updateBanner.entryHint}</p>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}
