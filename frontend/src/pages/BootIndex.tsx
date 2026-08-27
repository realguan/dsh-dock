// 启动序列页（原 ui/index.html）——阶段 D 迁移，本文件先作骨架占位：
// 保住「窗口路由可达、编译全绿」，页面能力清单见 docs/frontend-migration.md §4.1。
import { Emblem } from "@/components/layout/Emblem"
import { PageShell } from "@/components/layout/PageShell"
import { t } from "@/content/zh-CN"

export function BootIndex() {
  return (
    <PageShell>
      <div className="flex flex-col items-center gap-5 text-center">
        <Emblem size={56} />
        <h1 className="text-2xl font-semibold tracking-tight text-ink">
          {t.boot.headlines[0]}
        </h1>
        <div className="pulse-bar w-56">
          <div className="pulse-bar-fill" />
        </div>
        <p className="text-sm text-faint">阶段 D 迁移中——本页暂为骨架</p>
      </div>
    </PageShell>
  )
}
