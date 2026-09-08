// 事件总线：5 类事件 → store（payload 规整见 `lib/eventPayloads.ts` 纯函数）。
// 注意：本模块**加载期即装配监听**（文件末尾 `eventBusStarted`），
// 测试请 import `@/lib/eventPayloads` 而不是本文件。
import { listen } from "@tauri-apps/api/event"
import { EV } from "@/types/events"
import {
  normalizeAppUpdate,
  normalizeError,
  normalizeProgress,
  normalizeStep,
  normalizeVersions,
} from "@/lib/eventPayloads"
import { useBootStore } from "@/stores/bootStore"
import { useClientUpdateStore } from "@/stores/clientUpdateStore"

// 保持既有公开面：`normalize*` 仍可从 `@/lib/events` 取（内部已迁到纯函数模块）。
export {
  normalizeAppUpdate,
  normalizeError,
  normalizeProgress,
  normalizeStep,
  normalizeVersions,
}

// ---------- 总线初始化（仅 App 顶层 useEffect 调用一次；返回 cleanup） ----------

export function initEventBus(): () => void {
  const unlisteners: Promise<() => void>[] = [
    listen<unknown>(EV.bootStep, ({ payload }) => {
      const e = normalizeStep(payload)
      if (e) useBootStore.getState().setStep(e)
    }),
    listen<unknown>(EV.bootProgress, ({ payload }) => {
      const p = normalizeProgress(payload)
      if (p) useBootStore.getState().setProgress(p)
    }),
    listen<unknown>(EV.bootError, ({ payload }) => {
      const err = normalizeError(payload)
      if (err) useBootStore.getState().setError(err)
    }),
    listen<unknown>(EV.bootUpdate, ({ payload }) => {
      const v = normalizeVersions(payload)
      if (v) useBootStore.getState().setVersions(v)
    }),
    listen<unknown>(EV.appUpdate, ({ payload }) => {
      const ev = normalizeAppUpdate(payload)
      if (ev) useClientUpdateStore.getState().dispatch(ev)
    }),
  ]
  return () => unlisteners.forEach((u) => u.then((fn) => fn()))
}

// ---------- 模块级总线装配 ----------
// 为什么不放在 React effect 里：React 子组件 effect 先于父组件执行，
// 「页面播种 invoke」可能抢在「父级挂监听」之前发出首个事件（旧 ui/index.html
// 用同步 <script> 注册监听正是为了规避该竞态）。模块 import 于任何渲染前求值，
// 此处注册即最早期；应用生命周期内不需要拆卸，Tauri 窗口关闭即整体回收。
export const eventBusStarted = initEventBus()
