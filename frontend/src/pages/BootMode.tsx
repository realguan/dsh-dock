// 运行环境选择页（原 ui/mode.html）——阶段 C 迁移，本文件先作骨架占位：
// 保住「窗口路由可达、编译全绿」，页面能力清单见 docs/frontend-migration.md §4.2。
import { Emblem } from "@/components/layout/Emblem"
import { PageShell } from "@/components/layout/PageShell"
import { t } from "@/content/zh-CN"

export function BootMode() {
  return (
    <PageShell>
      <div className="flex flex-col items-center gap-5 text-center">
        <Emblem size={56} />
        <h1 className="text-2xl font-semibold tracking-tight text-ink">
          {t.mode.title}
        </h1>
        <p className="text-sm text-faint">阶段 C 迁移中——本页暂为骨架</p>
      </div>
    </PageShell>
  )
}
