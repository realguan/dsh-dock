// 顶栏版本芯片（原 selector/index 的 vchip 迁移）：DSH 版本 + 状态点 +
// 客户端更新短消息。数据全部来自 store（boot:update / app:update 已入），
// 本组件零监听、零 invoke。
import { useBootStore } from "@/stores/bootStore"
import { useClientUpdateStore } from "@/stores/clientUpdateStore"
import { t } from "@/content/zh-CN"

export function VersionChip() {
  const dsh = useBootStore((s) => s.versions?.dsh ?? null)
  const phase = useClientUpdateStore((s) => s.snapshot?.phase ?? "idle")
  const downloading = useClientUpdateStore((s) =>
    s.snapshot && s.snapshot.phase === "downloading" ? s.snapshot : null,
  )

  let dot: "" | "ok" | "new" = ""
  let text: string
  if (!dsh) {
    text = `DSH …`
  } else if (dsh.error) {
    dot = "new"
    text = `DSH · ${t.selector.chipCheckFailed}`
  } else if (dsh.newer) {
    dot = "new"
    text = `DSH ${dsh.current ?? "?"} → ${dsh.latest ?? ""}`
  } else if (dsh.current && dsh.latest) {
    dot = "ok"
    text = `DSH ${dsh.current}`
  } else {
    text = `DSH ${dsh.current ?? "…"}`
  }

  let clientMsg: string | null = null
  if (phase === "available") clientMsg = t.selector.chipClientNew
  else if (phase === "downloading")
    clientMsg = `${t.selector.chipClientUpdating} ${Math.floor(
      (100 * (downloading?.current ?? 0)) / (downloading?.total ?? 100),
    )}%`
  else if (phase === "installing" || phase === "relaunching")
    clientMsg = t.selector.chipClientUpdatingRun

  return (
    <div className="inline-flex max-w-[320px] items-center gap-2 rounded-full border border-line bg-panel/90 px-3 py-1 font-mono text-[11px] text-dim shadow-xs backdrop-blur-xs whitespace-nowrap">
      <span
        className={`size-2 shrink-0 rounded-full transition-colors ${
          dot === "ok"
            ? "bg-ok ring-2 ring-ok/20"
            : dot === "new"
              ? "animate-pulse bg-warn ring-2 ring-warn/25"
              : "bg-faint"
        }`}
      />
      <span className="truncate font-medium text-ink/85">{text}</span>
      {clientMsg && (
        <span className="truncate border-l border-line pl-2 font-medium text-warn">
          {clientMsg}
        </span>
      )}
    </div>
  )
}

