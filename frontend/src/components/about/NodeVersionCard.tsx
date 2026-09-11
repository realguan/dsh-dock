import { useBootStore } from "@/stores/bootStore"
import { useI18n } from "@/stores/i18nStore"
import { DimRow, DimNote } from "./DimRow"

/**
 * Node 运行时维度行。
 *
 * 2026-09-10 修复（v1.1.0 Windows 实测 1.2）：`version` 现在是**实测**值，
 * 未安装为 `null`——此前它拿的是**下载计划**版本，于是引擎 node 其实没装时
 * 这里照样显示「v24.18.0 · 应用托管 · 随启动自动准备」，与健康大盘的真探测
 * （未检出）自相矛盾。现在未装就如实说「未安装」，并把计划版本放进提示里。
 */
export function NodeVersionCard() {
  const { t } = useI18n()
  const node = useBootStore((s) => s.versions?.node ?? null)
  return (
    <DimRow label={t.about.nodeLabel} badge="JavaScript VM">
      {node?.version ? (
        <div className="flex items-center gap-2 sm:justify-end">
          <span className="font-mono text-xs font-semibold text-ink">
            {node.version}
          </span>
          <span className="rounded-md bg-line-soft px-1.5 py-0.5 font-mono text-meta text-dim">
            <DimNote>
              {node.origin === "engine"
                ? t.about.nodeFromEngine
                : node.origin === "system"
                  ? t.about.nodeFromSystem
                  : t.about.nodeManaged}
            </DimNote>
          </span>
        </div>
      ) : (
        <span className="text-faint font-mono text-xs">
          {/*
            未安装：如实报"没装"，并给出**将装**哪个（不是"已装哪个"）。
            注意这里**不再**追加 `nodeUnknown`（"尚未确定"）：既然已经明确知道
            "未安装 + 计划版本"，说"尚未确定"是自相矛盾（2026-09-10 审核修正）。
          */}
          {node?.plannedVersion
            ? t.about.nodeNotInstalledPlanned(node.plannedVersion)
            : t.about.nodeNotInstalled}
        </span>
      )}
    </DimRow>
  )
}
