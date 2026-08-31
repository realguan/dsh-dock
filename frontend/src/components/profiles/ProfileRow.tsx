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
import { t } from "@/content/zh-CN"
import type { ProfileSummary } from "@/types/ipc"
import { DropdownMenu } from "radix-ui"

interface ProfileRowProps {
  profile: ProfileSummary
  isDefault: boolean
  isSelected: boolean
  isRunning: boolean
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
  const { name, materialized, bundles, dependencies, web_ui } = profile

  const metaLine = materialized
    ? [
        t.profiles.metaBundles(bundles.length),
        t.profiles.metaDeps(dependencies.length),
        ...(web_ui ? [] : [t.profiles.tagNoUi]),
      ].join(` ${t.profiles.metaSep} `)
    : t.profiles.templateHint

  return (
    <div
      onClick={onSelect}
      style={{ animationDelay: `${Math.min(index, 8) * 35}ms` }}
      className={`page-rise group relative cursor-pointer rounded-xl border p-3 transition-all duration-200 ${
        isSelected
          ? "border-brand/40 bg-panel shadow-md ring-1 ring-brand/30"
          : "border-line bg-panel/80 hover:border-line hover:bg-panel hover:shadow-xs"
      } ${materialized ? "" : "border-dashed"}`}
    >
      {/* 活跃指示条 */}
      {isRunning ? (
        <span className="bg-ok absolute inset-y-2.5 left-0 w-[3.5px] rounded-r-full shadow-xs shadow-emerald-500/50" />
      ) : isSelected ? (
        <span className="bg-brand absolute inset-y-2.5 left-0 w-[3px] rounded-r-full" />
      ) : null}

      <div className="flex items-start justify-between gap-2 pl-1.5">
        <div className="min-w-0 flex-1">
          {/* 首行：Profile 名字 + 状态徽标 */}
          <div className="flex flex-wrap items-center gap-1.5">
            <span
              className={`truncate text-sm font-semibold tracking-tight ${
                isSelected ? "text-brand-deep font-bold" : "text-ink"
              }`}
              title={name}
            >
              {name}
            </span>

            {/* 运行中：翡翠绿脉动点 */}
            {isRunning && (
              <span className="bg-ok-soft text-ok inline-flex items-center gap-1 rounded-full px-1.5 py-0.5 text-[10px] font-medium leading-none">
                <span className="bg-ok size-1.5 animate-pulse rounded-full" aria-hidden />
                {t.profiles.runningBadge}
              </span>
            )}

            {/* 默认启动 */}
            {isDefault && (
              <span className="bg-amber-500/10 text-amber-600 border border-amber-500/20 inline-flex items-center gap-0.5 rounded-full px-1.5 py-0.5 text-[10px] font-medium leading-none">
                <Star className="size-2.5 fill-current" />
                {t.profiles.defaultBadge}
              </span>
            )}

            {/* 模板 / 已创建 */}
            {!materialized && (
              <span className="bg-line-soft text-dim rounded-full px-1.5 py-0.5 text-[10px] leading-none">
                {t.profiles.tagTemplate}
              </span>
            )}
          </div>

          {/* 元信息行 */}
          <div className="text-faint mt-1 truncate font-mono text-[11px]">
            {metaLine}
          </div>
        </div>

        {/* 动作区：启动主动作 + 更多菜单 */}
        <div
          className="flex shrink-0 items-center gap-1 pt-0.5"
          onClick={(e) => e.stopPropagation()}
        >
          {/* 主动作：启动或重启 */}
          {isRunning ? (
            <button
              type="button"
              title={busy ? t.profiles.launchWorking : t.profiles.restart}
              disabled={busy}
              onClick={onRestart}
              className="text-dim hover:text-ink hover:bg-line-soft inline-flex size-7 items-center justify-center rounded-lg border border-line bg-white transition-colors disabled:opacity-40"
            >
              <RotateCw className={`size-3.5 ${busy ? "animate-spin" : ""}`} />
            </button>
          ) : web_ui ? (
            <button
              type="button"
              title={busy ? t.profiles.launchWorking : t.profiles.launch}
              disabled={busy}
              onClick={onLaunch}
              className="border-line/80 text-dim hover:border-brand hover:text-brand hover:bg-wash inline-flex items-center gap-1 rounded-lg border bg-white px-2 py-1 text-xs font-medium transition-colors disabled:opacity-40"
            >
              {busy ? (
                <LoaderCircle className="size-3 animate-spin" aria-hidden />
              ) : (
                <Play className="size-3 fill-current opacity-70" aria-hidden />
              )}
              <span>{t.profiles.launch}</span>
            </button>
          ) : null}

          {/* 更多管理操作下拉菜单 */}
          {materialized && (
            <DropdownMenu.Root>
              <DropdownMenu.Trigger asChild>
                <button
                  type="button"
                  title="更多操作"
                  aria-label="更多操作"
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
                  <DropdownMenu.Item
                    onClick={onDetail}
                    className="flex cursor-pointer select-none items-center gap-2 rounded-lg px-2.5 py-1.5 outline-none hover:bg-wash hover:text-brand"
                  >
                    <Info className="size-3.5 text-dim" />
                    <span>查看详情</span>
                  </DropdownMenu.Item>

                  <DropdownMenu.Item
                    onClick={onSetDefault}
                    className="flex cursor-pointer select-none items-center gap-2 rounded-lg px-2.5 py-1.5 outline-none hover:bg-wash hover:text-brand"
                  >
                    <Star
                      className={`size-3.5 ${
                        isDefault ? "text-amber-500 fill-current" : "text-dim"
                      }`}
                    />
                    <span>{isDefault ? t.profiles.defaultIs : t.profiles.setDefault}</span>
                  </DropdownMenu.Item>

                  <DropdownMenu.Item
                    onClick={onCopy}
                    className="flex cursor-pointer select-none items-center gap-2 rounded-lg px-2.5 py-1.5 outline-none hover:bg-wash hover:text-brand"
                  >
                    <Copy className="size-3.5 text-dim" />
                    <span>{t.profiles.submitCopy}</span>
                  </DropdownMenu.Item>

                  <DropdownMenu.Item
                    onClick={onRename}
                    className="flex cursor-pointer select-none items-center gap-2 rounded-lg px-2.5 py-1.5 outline-none hover:bg-wash hover:text-brand"
                  >
                    <Pencil className="size-3.5 text-dim" />
                    <span>{t.profiles.submitRename}</span>
                  </DropdownMenu.Item>

                  <DropdownMenu.Separator className="my-1 h-px bg-line" />

                  <DropdownMenu.Item
                    onClick={onDelete}
                    className="flex cursor-pointer select-none items-center gap-2 rounded-lg px-2.5 py-1.5 text-warn outline-none hover:bg-warn-soft"
                  >
                    <Trash2 className="size-3.5" />
                    <span>{t.profiles.deleteConfirm(name)}</span>
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
