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
    text = `DSH ${dsh.current ?? "?"} · ${t.selector.chipDshNew} ${dsh.latest ?? ""}`
  } else if (dsh.current && dsh.latest) {
    dot = "ok"
    text = `DSH ${dsh.current} · ${t.selector.chipDshOk}`
  } else {
    text = `DSH ${dsh.current ?? "…"} · ${t.selector.chipDetecting}`
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
    <span className="border-line bg-panel/80 text-dim inline-flex max-w-[280px] items-center gap-1.5 rounded-full border px-2.5 py-1 font-mono text-[11px] whitespace-nowrap">
      <span
        className={`size-1.5 shrink-0 rounded-full ${
          dot === "ok" ? "bg-ok" : dot === "new" ? "bg-warn animate-blink" : "bg-faint"
        }`}
      />
      <span className="truncate">{text}</span>
      {clientMsg && (
        <span className="text-warn truncate border-l pl-1.5">{clientMsg}</span>
      )}
    </span>
  )
}
