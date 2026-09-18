// Profile 导航卡片（Master-Detail 架构重构）。
// 视觉：选中态高亮 + 运行中翡翠绿心跳 + 默认金星 + 主次动词分流。
import {
  Copy,
  Info,
  LoaderCircle,
  MoreHorizontal,
  Pencil,
  Play,
  RotateCw,
  Star,
  Trash2,
} from "lucide-react"
import { useI18n } from "@/stores/i18nStore"
import type { ProfileSummary } from "@/types/ipc"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { DropdownMenu } from "radix-ui"

/** 2026-09-18 收口：下拉菜单五项此前各抄一份同款类名字符串，提取为常量。 */
const MENU_ITEM_CLASS =
  "flex cursor-pointer select-none items-center gap-2 rounded-lg px-2.5 py-1.5 outline-none hover:bg-wash hover:text-brand-deep"
const MENU_ITEM_DANGER_CLASS =
  "flex cursor-pointer select-none items-center gap-2 rounded-lg px-2.5 py-1.5 text-danger outline-none hover:bg-danger-soft"

interface ProfileRowProps {
  profile: ProfileSummary
  isDefault: boolean
  isSelected: boolean
  isRunning: boolean
  isSwitching?: boolean
  busy: boolean
  index: number
  onSelect: () => void
  onDetail: () => void
  onSetDefault: () => void
  onLaunch: () => void
  onRestart: () => void
  onRename: () => void
  onCopy: () => void
  onDelete: () => void
}

export function ProfileRow({
  profile,
  isDefault,
  isSelected,
  isRunning,
  isSwitching = false,
  busy,
  index,
  onSelect,
  onDetail,
  onSetDefault,
  onLaunch,
  onRestart,
  onRename,
  onCopy,
  onDelete,
}: ProfileRowProps) {
  const { t } = useI18n()
  const { name, materialized, bundles, dependencies, web_ui } = profile

  const metaLine = materialized
    ? [
        t.profiles.metaBundles(bundles.length),
        t.profiles.metaDeps(dependencies.length),
        ...(web_ui ? [] : [t.profiles.tagNoUi]),
      ].join(` ${t.profiles.metaSep} `)
    : t.profiles.templateHint

  return (
    // 2026-09-08 裁定：整卡不可改成 <button>（内部还有启动/更多菜单按钮，嵌套交互元素
    // 非法且读屏会乱）。改为「名字即主控件」——键盘 Tab 到名字按钮回车即选中，卡片用
    // focus-within 呈现整卡焦点环，指针点击行为不变。
    <div
      onClick={onSelect}
      style={{ animationDelay: `${Math.min(index, 8) * 35}ms` }}
      className={`page-rise group relative cursor-pointer rounded-xl border p-3 transition-all duration-200 focus-within:ring-2 focus-within:ring-brand/50 ${
        isSelected
          ? "border-brand/40 bg-panel shadow-md ring-1 ring-brand/30"
          : "border-line bg-panel hover:border-line hover:bg-panel hover:shadow-xs"
      } ${materialized ? "" : "border-dashed"} ${
        // 交接期间用 info 环标记；底色不动——旧实现同时写 bg-panel 与 bg-info/5，
        // 两者冲突且 bg-panel 恒定胜出（dist 中 .bg-panel 在 .bg-info\/5 之后），
        // 即那层 wash 从未渲染过。2026-09-10 批次 E 复核修正为「只加环」。
        isSwitching ? "ring-1 ring-info/40" : ""
      }`}
    >
      {/* 活跃/重载指示条（2026-09-18 收口：w-[3.5px]/w-[3px] 两档并存，统一为 w-1） */}
      {isSwitching ? (
        <span className="bg-info absolute inset-y-2.5 left-0 w-1 rounded-r-full shadow-xs shadow-info/50 animate-pulse" />
      ) : isRunning ? (
        <span className="bg-ok absolute inset-y-2.5 left-0 w-1 rounded-r-full shadow-xs shadow-ok/50" />
      ) : isSelected ? (
        <span className="bg-brand absolute inset-y-2.5 left-0 w-1 rounded-r-full" />
      ) : null}

      <div className="flex items-start justify-between gap-2 pl-1.5">
        <div className="min-w-0 flex-1">
          {/* 首行：Profile 名字（主控件，键盘可达） + 状态徽标 */}
          <div className="flex flex-wrap items-center gap-1.5">
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation()
                onSelect()
              }}
              aria-current={isSelected ? "true" : undefined}
              title={name}
              className={`max-w-full truncate rounded-md p-0 text-left text-sm font-semibold tracking-tight outline-none ${
                isSelected ? "text-brand-deep font-bold" : "text-ink"
              }`}
            >
              {name}
            </button>

            {/* 重载过渡中：琥珀色脉动 */}
            {isSwitching ? (
              <span className="bg-info-soft text-info border border-info/30 inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-meta font-medium leading-none animate-pulse">
                <LoaderCircle className="size-3 animate-spin text-info" aria-hidden />
                <span>{isRunning ? t.profiles.reloadingWorkbench : t.profiles.launchingProfile}</span>
              </span>
            ) : isRunning ? (
              /* 运行中：翡翠绿脉动点 */
              <span className="bg-ok-soft text-ok inline-flex items-center gap-1 rounded-full px-1.5 py-0.5 text-meta font-medium leading-none">
                <span className="bg-ok size-1.5 animate-pulse rounded-full" aria-hidden />
                {t.profiles.runningBadge}
              </span>
            ) : null}

            {/* 默认启动（品牌=wash，2026-09-18 收口） */}
            {isDefault && (
              <span className="bg-wash text-brand-deep border border-brand/20 inline-flex items-center gap-0.5 rounded-full px-1.5 py-0.5 text-meta font-medium leading-none">
                <Star className="size-2.5 fill-current" />
                {t.profiles.defaultBadge}
              </span>
            )}

            {/* 模板 / 已创建 */}
            {!materialized && (
              <span className="bg-line-soft text-dim rounded-full px-1.5 py-0.5 text-meta leading-none">
                {t.profiles.tagTemplate}
              </span>
            )}
          </div>

          {/* 元信息行 */}
          <div className="text-faint mt-1 truncate font-mono text-label" title={metaLine}>
            {metaLine}
          </div>
        </div>

        {/* 动作区：启动主动作 + 更多菜单 */}
        <div
          className="flex shrink-0 items-center gap-1 pt-0.5"
          onClick={(e) => e.stopPropagation()}
        >
          {/* 主动作：重载中、启动或重启（2026-09-18 收口：手搓 bg-white 按钮改基座 Button） */}
          {isSwitching ? (
            <span className="bg-info-soft text-info inline-flex items-center gap-1.5 rounded-lg border border-line/80 px-2 py-1 text-xs font-medium cursor-wait shadow-2xs">
              <LoaderCircle className="size-3 animate-spin text-info" aria-hidden />
              <span>{t.profiles.launchWorking}</span>
            </span>
          ) : isRunning ? (
            <Button
              size="icon-sm"
              variant="outline"
              title={busy ? t.profiles.launchWorking : t.profiles.restart}
              aria-label={busy ? t.profiles.launchWorking : t.profiles.restart}
              disabled={busy}
              onClick={onRestart}
            >
              <RotateCw className={`size-3.5 ${busy ? "animate-spin" : ""}`} />
            </Button>
          ) : web_ui ? (
            <Button size="sm" variant="outline" title={busy ? t.profiles.launchWorking : t.profiles.launch} disabled={busy} onClick={onLaunch}>
              {busy ? (
                <LoaderCircle className="size-3 animate-spin" aria-hidden />
              ) : (
                <Play className="size-3 fill-current opacity-70" aria-hidden />
              )}
              <span>{t.profiles.launch}</span>
            </Button>
          ) : null}

          {/* 更多管理操作下拉菜单 */}
          {materialized && (
            <DropdownMenu.Root>
              <DropdownMenu.Trigger asChild>
                <button
                  type="button"
                  title={t.profiles.moreActions}
                  aria-label={t.profiles.moreActions}
                  disabled={busy}
                  className="text-faint hover:text-ink hover:bg-line-soft inline-flex size-7 items-center justify-center rounded-lg transition-colors"
                >
                  <MoreHorizontal className="size-4" />
                </button>
              </DropdownMenu.Trigger>

              <DropdownMenu.Portal>
                <DropdownMenu.Content
                  align="end"
                  sideOffset={4}
                  className="z-50 min-w-[150px] overflow-hidden rounded-xl border border-line bg-panel p-1 text-xs text-ink shadow-lg ring-1 ring-black/5 animate-in fade-in-0 zoom-in-95"
                >
                  {(
                    [
                      {
                        key: "detail",
                        Icon: Info,
                        label: t.profiles.actionDetail,
                        run: onDetail,
                        iconClass: undefined,
                      },
                      {
                        key: "default",
                        Icon: Star,
                        label: isDefault ? t.profiles.defaultIs : t.profiles.setDefault,
                        run: onSetDefault,
                        iconClass: isDefault ? "fill-current text-brand-deep" : undefined,
                      },
                      { key: "copy", Icon: Copy, label: t.profiles.submitCopy, run: onCopy, iconClass: undefined },
                      { key: "rename", Icon: Pencil, label: t.profiles.actionRename, run: onRename, iconClass: undefined },
                    ] as const
                  ).map(({ key, Icon, label, run, iconClass }) => (
                    <DropdownMenu.Item
                      key={key}
                      onClick={run}
                      className={MENU_ITEM_CLASS}
                    >
                      <Icon className={cn("size-3.5 text-dim", iconClass)} />
                      <span>{label}</span>
                    </DropdownMenu.Item>
                  ))}

                  <DropdownMenu.Separator className="my-1 h-px bg-line" />

                  {/* 删除：破坏性动作用 danger 语义色（修复"危险动作视觉最弱"的权重倒挂） */}
                  <DropdownMenu.Item onClick={onDelete} className={MENU_ITEM_DANGER_CLASS}>
                    <Trash2 className="size-3.5" />
                    <span>{t.profiles.actionDelete}</span>
                  </DropdownMenu.Item>
                </DropdownMenu.Content>
              </DropdownMenu.Portal>
            </DropdownMenu.Root>
          )}
        </div>
      </div>
    </div>
  )
}
