// Node 维度行（原 about.html render 的 node 段迁移）：纯只读——版本 + 来源。
// AGENTS：Node 无升级动作（版本由下载计划决定），此处不出任何按钮。
import { useBootStore } from "@/stores/bootStore"
import { t } from "@/content/zh-CN"
import { DimRow, DimNote } from "./DimRow"

export function NodeVersionCard() {
  const node = useBootStore((s) => s.versions?.node ?? null)
  return (
    <DimRow label={t.about.nodeLabel}>
      {node ? (
        <span className="text-sm">
          {node.version}
          <DimNote>{node.origin === "system" ? t.about.nodeFromSystem : t.about.nodeManaged}</DimNote>
        </span>
      ) : (
        <span className="text-faint text-sm">
          —
          <DimNote>{t.about.nodeUnknown}</DimNote>
        </span>
      )}
    </DimRow>
  )
}
