// Profile 列表行（4.3 前端刀）。两态视觉：已物化 = 实线卡 + 品牌左侧条；
// 可首启（未物化内置模板名）= 虚线卡 + ok 色标签，仅暴露「设为默认」
// （重命名/复制/删除对无目录的模板名在后端本就会被拒绝，前端不渲染）。
// 4.3⑥ 切换：webUi 候选（profile.web_ui）额外暴露「启动」——主动词给带字
// 按钮（与页头「新建 Profile」同一语言），图标动作保持静默次级；运行中行以
// 心跳徽标标识且不重复给启动；非 webUi 的「无界面」是恒定属性，归 meta 行。
import { Copy, Info, LoaderCircle, Pencil, Play, Star, Trash2 } from "lucide-react"
import { t } from "@/content/zh-CN"
import type { ProfileSummary } from "@/types/ipc"

interface ProfileRowProps {
  profile: ProfileSummary
  isDefault: boolean
  /** 该行是当前运行中的会话 profile（徽标；不给启动） */
  isRunning: boolean
  /** 该行有操作在途（设默认/切换）——动作按钮集体禁用 */
  busy: boolean
  index: number
  onDetail: () => void
  onSetDefault: () => void
  onLaunch: () => void
  onRename: () => void
  onCopy: () => void
  onDelete: () => void
}

/** 行内小图标按钮：透明底、faint 前景，hover 提亮；danger 变体 hover 警示色。 */
function IconAction({
  label,
  danger,
  onClick,
  disabled,
  children,
}: {
  label: string
  danger?: boolean
  onClick: () => void
  disabled?: boolean
  children: React.ReactNode
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      disabled={disabled}
      onClick={onClick}
      className={`text-faint hover:text-ink disabled:pointer-events-none disabled:opacity-40 inline-flex size-7 items-center justify-center rounded-md transition-colors ${
        danger ? "hover:bg-warn-soft hover:text-warn" : "hover:bg-wash"
      }`}
    >
      {children}
    </button>
  )
}

export function ProfileRow({
  profile,
  isDefault,
  isRunning,
  busy,
  index,
  onDetail,
  onSetDefault,
  onLaunch,
  onRename,
  onCopy,
  onDelete,
}: ProfileRowProps) {
  const { name, materialized, bundles, dependencies, web_ui } = profile
  // 两态元信息：已物化 = 插件数 · 依赖数（mono 展示；非 webUi 追加「无界面」
  // 说明为何没有启动入口）；可首启 = 模板说明
  const metaLine = materialized
    ? [
        t.profiles.metaBundles(bundles.length),
        t.profiles.metaDeps(dependencies.length),
        ...(web_ui ? [] : [t.profiles.tagNoUi]),
      ].join(` ${t.profiles.metaSep} `)
    : t.profiles.templateHint

  return (
    <div
      className={`page-rise border-line bg-panel group relative rounded-xl border shadow-sm transition-shadow hover:shadow-md ${
        materialized ? "" : "border-dashed"
      }`}
      style={{ animationDelay: `${Math.min(index, 8) * 45}ms` }}
    >
      {materialized && (
        <span className="bg-brand absolute inset-y-3 left-0 w-[3px] rounded-r-full" />
      )}
      <div className="flex items-center gap-3 py-3 pr-3 pl-4">
        {/* 主体：已物化点击看详情；模板行不接管点击（动作走右侧按钮） */}
        <button
          type="button"
          onClick={materialized ? onDetail : undefined}
          className="min-w-0 flex-1 text-left"
          title={materialized ? name : undefined}
        >
          <div className="flex flex-wrap items-center gap-1.5">
            <span className="text-ink truncate font-medium">{name}</span>
            {materialized ? (
              <span className="border-line text-dim rounded-full border px-1.5 text-[10px] leading-4">
                {t.profiles.tagMaterialized}
              </span>
            ) : (
              <span className="bg-ok-soft text-ok rounded-full px-1.5 text-[10px] leading-4">
                {t.profiles.tagTemplate}
              </span>
            )}
            {/* 非 webUi 的「无界面」是恒定属性 → meta 行（见 metaLine） */}
            {isRunning && (
              <span className="bg-ok-soft text-ok inline-flex items-center gap-1 rounded-full px-1.5 text-[10px] leading-4">
                {/* 心跳点：与「可首启」同绿但带脉动——活着的会话，一眼可辨 */}
                <span className="bg-ok size-1.5 animate-pulse rounded-full" aria-hidden />
                {t.profiles.runningBadge}
              </span>
            )}
            {isDefault && (
              <span className="bg-wash text-brand inline-flex items-center gap-0.5 rounded-full px-1.5 text-[10px] leading-4">
                <Star className="size-2.5 fill-current" />
                {t.profiles.defaultBadge}
              </span>
            )}
          </div>
          <div className="text-faint mt-0.5 truncate font-mono text-xs">{metaLine}</div>
        </button>

        {/* 动作区：启动是主动词 → 带字按钮（与页头「新建 Profile」同配方）；
            其余为静默图标动作 */}
        <div className="flex items-center gap-2">
          {web_ui && !isRunning && (
            <button
              type="button"
              title={busy ? t.profiles.launchWorking : t.profiles.launch}
              disabled={busy}
              onClick={onLaunch}
              className="border-line text-dim hover:border-brand hover:text-brand inline-flex items-center gap-1 rounded-lg border bg-white px-2 py-1 text-xs transition-colors disabled:pointer-events-none disabled:opacity-40"
            >
              {busy ? (
                <LoaderCircle className="size-3 animate-spin" aria-hidden />
              ) : (
                <Play className="size-3" aria-hidden />
              )}
              {busy ? t.profiles.launchWorking : t.profiles.launch}
            </button>
          )}
          <div className="flex items-center gap-0.5">
            <IconAction
              label={isDefault ? t.profiles.defaultIs : t.profiles.setDefault}
              disabled={busy}
              onClick={onSetDefault}
            >
              <Star className={`size-4 ${isDefault ? "text-brand fill-current" : ""}`} />
            </IconAction>
            {materialized && (
              <>
                <IconAction label={t.profiles.detailTitle(name)} disabled={busy} onClick={onDetail}>
                  <Info className="size-4" />
                </IconAction>
                <IconAction label={t.profiles.submitCopy} disabled={busy} onClick={onCopy}>
                  <Copy className="size-4" />
                </IconAction>
                <IconAction label={t.profiles.submitRename} disabled={busy} onClick={onRename}>
                  <Pencil className="size-4" />
                </IconAction>
                <IconAction label={t.profiles.deleteTitle(name)} danger disabled={busy} onClick={onDelete}>
                  <Trash2 className="size-4" />
                </IconAction>
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  )
}
