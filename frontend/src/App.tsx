// 窗口 label 路由（frontend-migration §3.1）：
// - 所有窗口都加载同一入口（WebviewUrl::App("/")），React 按
//   getCurrentWindow().label() 决定渲染哪个页面；
// - about 窗口只渲染 About，不需要路由；
// - 主窗口 pathname 路由：release 产物经 tauri get_asset 兜底链可达 /selector
//   直达（2026-08-27 验证，docs/frontend-migration.md §3.1）。
//
// 事件总线的初始化在 lib/events.ts 模块加载期完成（早于任何渲染与
// 页面播种 invoke，见该处裁定注释）。label 未就绪时渲染轻量骨架。
//
// 语言初始化（2026-09-11，task-24）：**每个窗口都在此自行初始化一次**，
// 并订阅 `app:settings-changed` 做运行期跨窗同步——
//   · 各窗独立 JS runtime（AGENTS §4.4 红线 3，Zustand 不跨窗），故不能共享 store；
//   · 首帧门闸：语言未解析前不渲染任何页面，复用下面既有的轻量骨架。
//     否则初始字典恒为 zhCN（store 初始态），en-US 用户会先看到一帧中文再切换
//     （详见 docs/team/跨窗口语言初始化-2026-09-11.md §首帧评估）。
import { getCurrentWindow } from "@tauri-apps/api/window"
import { listen } from "@tauri-apps/api/event"
import { useEffect, useState } from "react"
import { Navigate, Route, Routes, useNavigate } from "react-router-dom"
import { Emblem } from "@/components/layout/Emblem"
import { BootIndex } from "@/pages/BootIndex"
import { BootMode } from "@/pages/BootMode"
import { BootSelector } from "@/pages/BootSelector"
import { About } from "@/pages/About"
import { ProfileManager } from "@/pages/ProfileManager"
import { useI18nStore } from "@/stores/i18nStore"
import { logger } from "@/lib/logger"
import { EV } from "@/types/events"
import type { ShellSettings } from "@/types/ipc"

import { PulseBar } from "@/components/boot/PulseBar"

/** 首帧门闸上限：设置读取是本地文件读，正常毫秒级；万一 IPC 挂起也不能把界面
 *  永久挡在骨架屏（超时即放行，随后到达的解析结果仍会驱动一次重渲染）。 */
const LOCALE_INIT_TIMEOUT_MS = 1500

export default function App() {
  const [label, setLabel] = useState<string | null>(null)
  const [localeReady, setLocaleReady] = useState(false)
  const navigate = useNavigate()

  useEffect(() => {
    // 挂载全局 SPA 导航桥接（Rust 后端路由指令平滑切换，避免 location.assign 整页重载闪烁）
    ;(window as unknown as { __DSH_NAVIGATE__?: (path: string) => void }).__DSH_NAVIGATE__ = (path: string) => {
      navigate(path)
    }
    return () => {
      delete (window as unknown as { __DSH_NAVIGATE__?: (path: string) => void }).__DSH_NAVIGATE__
    }
  }, [navigate])

  useEffect(() => {
    // label 是同步 getter（@tauri-apps/api v2）；缺失/异常（纯 vite dev 浏览器
    // 预览）按主窗口兜底，可用 ?_label=about 强制路由便于页面级设计走查。
    try {
      const l = getCurrentWindow().label
      setLabel(typeof l === "string" ? l : "main")
    } catch {
      const forced = new URLSearchParams(window.location.search).get("_label")
      setLabel(forced ?? "main")
    }
  }, [])

  useEffect(() => {
    // ① 本窗口首次初始化：每窗一次 IPC 读设置（profiles 窗口原先的重复调用
    //    已收敛到此处，见 pages/ProfileManager.tsx）
    let alive = true
    const initSettled = Promise.race([
      useI18nStore.getState().initFromSettings(),
      new Promise<void>((resolve) => {
        window.setTimeout(() => {
          logger.warn("i18n", "语言初始化超时，先按当前字典放行", {
            timeoutMs: LOCALE_INIT_TIMEOUT_MS,
          })
          resolve()
        }, LOCALE_INIT_TIMEOUT_MS)
      }),
    ])

    // ② 运行期跨窗同步：set_shell_settings 广播全量 ShellSettings（含 locale）
    let unlisten: (() => void) | undefined
    listen<ShellSettings>(EV.settingsChanged, (event) => {
      useI18nStore.getState().applySettingsLocale(event.payload ?? null)
    })
      .then((fn) => {
        if (alive) unlisten = fn
        else fn()
      })
      .catch((err) => {
        logger.warn("i18n", "订阅 app:settings-changed 失败，跨窗语言同步不可用", {
          error: String(err),
        })
      })

    initSettled
      .then(() => {
        if (alive) setLocaleReady(true)
      })
      .catch(() => {
        // initFromSettings 内部已吞读取失败；此处只兜 Promise 链异常，
        // 保证门闸一定打开（否则界面永久停在骨架屏）。
        if (alive) setLocaleReady(true)
      })

    return () => {
      alive = false
      unlisten?.()
    }
  }, [])

  if (label === null || !localeReady)
    return (
      <main className="bg-bg text-ink flex min-h-dvh items-center justify-center">
        <div className="flex flex-col items-center gap-5">
          <Emblem size={56} />
          <PulseBar width={160} />
        </div>
      </main>
    )

  // about 窗口：单页，不经路由
  if (label === "about") return <About />

  // profile 管理器窗口（4.3）：单页，不经路由（独立窗口理由见页面头注释）
  if (label === "profiles") return <ProfileManager />

  // 主窗口：启动序列 + pathname 路由
  return (
    <Routes>
      <Route path="/" element={<BootIndex />} />
      <Route path="/mode" element={<BootMode />} />
      <Route path="/selector" element={<BootSelector />} />
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  )
}
