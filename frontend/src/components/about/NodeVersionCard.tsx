// Node 维度行（原 about.html render 的 node 段迁移）：纯只读——版本 + 来源。
import { useBootStore } from "@/stores/bootStore"
import { t } from "@/content/zh-CN"
import { DimRow, DimNote } from "./DimRow"

export function NodeVersionCard() {
  const node = useBootStore((s) => s.versions?.node ?? null)
  return (
    <DimRow label={t.about.nodeLabel} badge="JavaScript VM">
      {node ? (
        <div className="flex items-center gap-2 sm:justify-end">
          <span className="font-mono text-xs font-semibold text-ink">
            {node.version}
          </span>
          <span className="rounded bg-line-soft px-1.5 py-0.5 font-mono text-[10px] text-dim">
            <DimNote>{node.origin === "system" ? t.about.nodeFromSystem : t.about.nodeManaged}</DimNote>
          </span>
        </div>
      ) : (
        <span className="text-faint font-mono text-xs">
          — <DimNote>{t.about.nodeUnknown}</DimNote>
        </span>
      )}
    </DimRow>
  )
}
