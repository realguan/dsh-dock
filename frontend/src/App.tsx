// 窗口 label 路由（frontend-migration §3.1）：
// - 所有窗口都加载同一入口（WebviewUrl::App("/")），React 按
//   getCurrentWindow().label() 决定渲染哪个页面；
// - about 窗口只渲染 About，不需要路由；
// - 主窗口 pathname 路由：release 产物经 tauri get_asset 兜底链可达 /selector
//   直达（2026-08-27 验证，docs/frontend-migration.md §3.1）。
//
// 事件总线仅在此初始化一次（每窗口 runtime 各一份），cleanup 保证
// StrictMode 双调用不重复监听。label 未就绪时渲染轻量骨架。
import { getCurrentWindow } from "@tauri-apps/api/window"
import { useEffect, useState } from "react"
import { Navigate, Route, Routes } from "react-router-dom"
import { initEventBus } from "@/lib/events"
import { Emblem } from "@/components/layout/Emblem"
import { BootIndex } from "@/pages/BootIndex"
import { BootMode } from "@/pages/BootMode"
import { BootSelector } from "@/pages/BootSelector"
import { About } from "@/pages/About"

export default function App() {
  const [label, setLabel] = useState<string | null>(null)

  useEffect(() => {
    const dispose = initEventBus()
    return () => dispose()
  }, [])

  useEffect(() => {
    // label 是同步 getter（@tauri-apps/api v2）；缺失/异常按主窗口兜底，
    // 让纯 vite dev 浏览器预览也能渲染。
    try {
      const l = getCurrentWindow().label
      setLabel(typeof l === "string" ? l : "main")
    } catch {
      setLabel("main")
    }
  }, [])

  if (label === null)
    return (
      <main className="bg-bg text-ink flex min-h-dvh items-center justify-center">
        <div className="flex flex-col items-center gap-5">
          <Emblem size={56} />
          <div className="pulse-bar w-40">
            <div className="pulse-bar-fill" />
          </div>
        </div>
      </main>
    )

  // about 窗口：单页，不经路由
  if (label === "about") return <About />

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
