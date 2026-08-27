// 关于/更新中心页（原 ui/about.html）——阶段 B 迁移，本文件先作骨架占位：
// 保住「about 窗口加载同一入口可达、编译全绿」，页面能力清单见 §4.4。
import { Emblem } from "@/components/layout/Emblem"
import { PageShell } from "@/components/layout/PageShell"
import { t } from "@/content/zh-CN"

export function About() {
  return (
    <PageShell width={480}>
      <div className="flex flex-col items-center gap-5 text-center">
        <Emblem size={56} />
        <h1 className="text-2xl font-semibold tracking-tight text-ink">
          {t.about.title}
        </h1>
        <p className="text-sm text-faint">阶段 B 迁移中——本页暂为骨架</p>
      </div>
    </PageShell>
  )
}
