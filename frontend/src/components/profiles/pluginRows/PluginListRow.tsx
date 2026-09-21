// PluginListRow.tsx —— 插件清单行（2026-09-21 结构收敛：自 ProfileDetailPane 抽出）。
//
// ## 一行一个包，三种行只有一种形态
//
// 演化史（判据跟着裁定走）：
//   v1 一体化能力卡片：能力 = 第一呈现单位，包降为卡片内的依赖明细行（`member` 形态）。
//   v2 维护者裁定：**「实验能力」是实验性插件的唯一操作入口**——插件列表里实验性行
//      一律**只读平铺**，与社区行同构。`member` 形态与能力卡片一起退役。
//   v3（本次）维护者四条真机反馈落地，名字/标记/图标三层一起校正：
//      ① **主标题 = 插件原本的名字**（"插件名应该是取插件原本的名字，中文为辅"）：
//         社区/内置行 = 包名去 scope（`dsh-memory-plugin`）；实验性行 = 去噪短名
//         （`browser-use` —— 那里 `dsh-experimental-` 是三个后端共有的纯噪音）。
//         完整包名一律留在 `title` 上，排查/读屏可取。
//      ② **中文名降为辅助灰字**：实验性行取所属能力的官方中文名（目录给的 `label`），
//         其余行取策展表里的中文友好名；两者都没有就整块不渲染（**不再回落到
//         Title Case 英文**——`Browser Use Playwright Mcp` 这种机器音译比没有更糟）。
//      ③ **已经按类筛过，就不再逐行复述类名**（维护者："已经按照 tab 分类了，
//         就不需要展示 tab 本身这类的标签"）：`kindFilter` 与自身类别相同的行不挂那枚
//         主标记（社区 tab 里不写「社区」、内置 tab 里不写「内置」……）；"全部"视图下
//         标记照旧全给——那时它是**区分依据**，不是复述。
//      ④ **图标要说它本来的意思**："实验这类，应该可以用烧杯这种 icon" → 一切表示
//         "实验性"的图标由 ✨(Sparkles, 意为"魔法/新奇") 换成 🧪(FlaskConical, 烧瓶)。
//
// ## 右侧控制基线
// `manageable`（= 社区包）才有 Switch 与 `···`（更新 / 选版本 / 卸载）；实验性包只读
// （操作在「实验能力」里）、内置层只读（归 dsh 自己的插件页）——只读行**整列不渲染**，
// 也不给 hover 高亮（没控件就不装作能点）。

import {
  ArrowUpCircle,
  FlaskConical,
  LoaderCircle,
  MoreHorizontal,
  Package,
  Puzzle,
  ShieldCheck,
  Trash2,
} from "lucide-react"

import { pluginChineseName, pluginMemberLabel, pluginShortId } from "@/lib/pluginDisplay"
import type { MergedPluginRow, PluginKindFilter } from "@/lib/pluginCatalog"
import type { RuntimeChip } from "@/lib/profiles"
import { Button } from "@/components/ui/button"
import { capabilityIcon } from "@/components/ui/capability-icon"
import { IconChip } from "@/components/ui/icon-chip"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Switch } from "@/components/ui/switch"
import type { useI18n } from "@/stores/i18nStore"

type Copy = ReturnType<typeof useI18n>["t"]

export interface PluginListRowProps {
  item: MergedPluginRow
  /** 当前生效的类别筛选：与该行类别相同时不挂主标记（复述 tab 名 = 噪音，见文件头 ③）。 */
  kindFilter: PluginKindFilter
  /** 该包在 dependencies 里的声明值（层没有）；仅作展示兜底。 */
  spec?: string
  chip: RuntimeChip | null
  /** 该行开关正在等落定（写完后短轮询期间）。 */
  isPending: boolean
  /** 本行有操作在途（行内小 spinner 取代右侧动作）。 */
  rowBusy: boolean
  /** 面板级 busy（任一操作在途）——关掉本行的动作入口。 */
  opBusy: boolean
  latest?: string
  onToggle: () => void
  onUpdate: () => void
  onRemove: () => void
  onVersionPick: () => void
  chipText: (chip: RuntimeChip) => string
  chipTone: (chip: RuntimeChip) => string
  t: Copy
  /** 界面语言（友好名映射按语言分表；见 lib/pluginDisplay.ts）。 */
  locale: string
}

export function PluginListRow(props: PluginListRowProps) {
  const { item, chip, isPending, rowBusy, chipText, chipTone, t, locale } = props
  const p = item.entry
  const shellDisabled = item.row?.shell_disabled ?? false
  /** 只有社区包可控：实验性包的操作在「实验能力」，内置层归 dsh 自己的插件页。 */
  const manageable = item.kindTag === "thirdParty"
  /** 版本徽标。**内置层不挂版本**（2026-09-21 维护者真机："『内置核心（随 DSH）』这些
   *  应该都不需要了"）：随 dsh 安装下发的层没有独立版本可读，那枚徽标既不是版本、
   *  又在重复区块已经在说的话——纯占位。社区行没装时仍如实写「未安装」。 */
  const versionText = p.installed_version
    ? `v${p.installed_version}`
    : props.spec
      ? t.profiles.pluginNotInstalled
      : ""

  /** 运行态徽标：运行中的 dsh 视角；配置侧「已禁用」是另一枚。 */
  const runtimeChipNode = !shellDisabled ? (
    isPending ? (
      <span
        title={t.profiles.chipHint.applying}
        className="bg-line-soft text-dim shrink-0 rounded-md px-1.5 py-0.5 text-meta leading-none"
      >
        {t.profiles.chip.applying}
      </span>
    ) : (
      chip && (
        <span
          title={t.profiles.chipHint[chip.kind]}
          className={`shrink-0 rounded-md px-1.5 py-0.5 text-meta leading-none ${chipTone(chip)}`}
        >
          {chipText(chip)}
        </span>
      )
    )
  ) : (
    <span
      title={t.profiles.pluginDisabledHint}
      className="border-line text-faint shrink-0 rounded-md border px-1.5 py-0.5 text-meta leading-none"
    >
      {t.profiles.pluginDisabled}
    </span>
  )

  const RowIcon =
    item.kindTag === "builtin"
      ? ShieldCheck
      : item.kindTag === "experimental" && item.capability
        ? // 一能力一形（与「实验能力」面板同一份映射，禁双源）：实验性行平铺后，
          // 图标是唯一还能一眼区分"这几行属于哪个能力"的线索。
          capabilityIcon(item.capability.id)
        : Puzzle
  /** 主标题 = **插件原本的名字**（维护者 2026-09-21："插件名应该是取插件原本的名字，
   *  中文为辅"）。实验性行走去噪短名（那一族包共有 `dsh-experimental-` 前缀，剥掉才认得出
   *  区分段）；其余行走包名去 scope——那就是 npm 上它的原名。完整包名在 `title` 上。 */
  const title = item.kindTag === "experimental" ? pluginMemberLabel(p.name) : pluginShortId(p.name)
  /** 辅助灰字 = 中文名：实验性行取所属能力的官方中文名；其余行取策展表命中。
   *  查表未命中时**不回落**到机器音译（`Browser Use Playwright Mcp` 比没有更糟）。 */
  const auxName =
    item.kindTag === "experimental" && item.capability
      ? item.capability.label
      : pluginChineseName(p.name, locale)
  /** 与当前筛选同类 ⇒ 类名已在 tab/芯片上写着，行内不再复述（"全部"视图下照给）。 */
  const hideKindTag = props.kindFilter === item.kindTag
  const isBase = p.name === "@deepseek-ai/dsh-base"
  const isWebApp = p.name === "@deepseek-ai/dsh-web-app"
  // 兜底说明只给**dsh 安装提供**的行（见 lib/pluginCatalog.ts 模型说明）：
  // 用户装的层与实验依赖不能落进"随 dsh 安装自带"（2026-09-17 张冠李戴的同族错误）。
  const fallbackDesc = isBase
    ? t.profiles.bundleDescBase
    : isWebApp
      ? t.profiles.bundleDescWebApp
      : t.profiles.bundleDescShipped
  const desc = p.description ?? props.spec ?? (item.dshProvided ? fallbackDesc : null)

  /** 右侧控制面是否存在：**只读行整列不渲染**——没有控件却留一段 68px 空白，
   *  看着像控件没加载出来。内置层（区标题已写"不可变更"）与实验性行（操作在
   *  「实验能力」）都走这条路，故不再有"逐行只读标"这回事。 */
  const hasRightSide = rowBusy || manageable

  return (
    <div
      // 只有可控的行给 hover 高亮：只读行（内置层 / 实验性行）亮一下就是暗示"能点"，
      // 而它们一个控件都没有——与能力卡片"整卡不做 hover"同一条判据。
      className={`flex items-center justify-between gap-3 p-3.5 transition-colors ${
        manageable ? "hover:bg-wash/30" : ""
      } ${shellDisabled ? "bg-bg/40 opacity-75" : ""}`}
    >
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <IconChip
            icon={RowIcon}
            tone={item.kindTag === "experimental" ? "brand" : "neutral"}
            className="size-6 rounded-md"
          />

          {/* 主标题 = 插件原本的名字；完整包名与官方描述进 title */}
          <span
            className={`break-all text-xs font-semibold ${shellDisabled ? "text-faint" : "text-ink"}`}
            title={`${p.name}${desc ? ` — ${desc}` : ""}`}
          >
            {title}
          </span>
          {/* 中文名**为辅**（灰字、小一号）；查不到中文名就整块不渲染 */}
          {auxName && (
            <span className="text-dim shrink-0 text-micro" title={auxName}>
              {auxName}
            </span>
          )}

          {versionText && (
            <span className="text-dim bg-line-soft/80 border-line/50 rounded-md border px-1.5 py-0.5 font-mono text-micro">
              {versionText}
            </span>
          )}

          {/* 主标记三选一（与筛选 chips 同名同源）。**已按类筛过就不复述类名**：
              社区 tab 里不写「社区」、内置 tab 里不写「内置」、实验性 tab 里不写「实验性」
              （维护者 2026-09-21："已经按照 tab 分类了，就不需要展示 tab 本身这类的标签"）。
              实验性标记里的能力名已经不在这里复述——它上移到主标题旁的辅助灰字。 */}
          {!hideKindTag &&
            (item.kindTag === "experimental" && item.capability ? (
              <span
                title={t.profiles.tagExperimentalHint(item.capability.label)}
                className="border-brand/30 bg-brand-soft text-brand-deep inline-flex items-center gap-1 rounded-md border px-2 py-0.5 text-meta font-medium leading-none"
              >
                {/* 烧杯 = 实验（维护者 2026-09-21："实验这类，应该可以用烧杯这种 icon"）。
                    原先用 ✨ —— 那个符号的语义是"魔法/新奇"，不是"实验"。 */}
                <FlaskConical className="size-3" />
                {t.profiles.tagExperimental}
              </span>
            ) : item.kindTag === "builtin" ? (
              <span
                title={t.profiles.tagBuiltinHint}
                className="border-line bg-line-soft text-dim rounded-md border px-1.5 py-0.5 text-meta leading-none"
              >
                {t.profiles.tagBuiltin}
              </span>
            ) : (
              <span
                title={t.profiles.tagThirdPartyHint}
                className="border-line bg-bg text-faint rounded-md border px-1.5 py-0.5 text-meta leading-none"
              >
                {t.profiles.tagThirdParty}
              </span>
            ))}

          {/* dsh 自带的实验层：主标实验性（上面那枚），这里**补一枚**内置 —— 与
              lib/pluginCatalog.ts 的 `alsoBuiltin` 注释同口径（"多一个内置标签就好了"）。
              它在「内置」tab 里同样是复述，故与主标记一样受筛选判据守护。 */}
          {item.alsoBuiltin && props.kindFilter !== "builtin" && (
            <span
              title={t.profiles.tagBuiltinHint}
              className="border-line bg-line-soft text-dim rounded-md border px-1.5 py-0.5 text-meta leading-none"
            >
              {t.profiles.tagBuiltin}
            </span>
          )}

          {/* 升级提示（只给可控的社区行：实验能力的版本归能力面板管） */}
          {manageable && props.latest && props.latest !== p.installed_version && (
            <button
              type="button"
              disabled={props.opBusy}
              onClick={props.onVersionPick}
              className="text-brand-deep hover:bg-wash border-brand/30 bg-wash inline-flex items-center gap-1 rounded-full border px-2 py-0.5 font-mono text-meta font-medium transition-colors"
            >
              <ArrowUpCircle className="size-3" />
              <span>{props.latest}</span>
            </button>
          )}

          {runtimeChipNode}
        </div>

        {desc && (
          <p className="text-faint mt-1 truncate text-xs" title={desc}>
            {desc}
          </p>
        )}
      </div>

      {/* 右侧控制基线：Switch 主入口 + `···` 溢出菜单；只读行给「只读」标 */}
      {hasRightSide && (
        <div className="flex min-w-[68px] shrink-0 items-center justify-end gap-2">
          {rowBusy ? (
            <LoaderCircle className="text-brand-deep size-4 animate-spin" />
          ) : (
            <>
              {manageable && item.row !== null && (
                <div className="flex items-center gap-1.5" title={t.profiles.pluginToggleHint}>
                  <Switch
                    aria-label={`${p.name}：${
                      shellDisabled ? t.profiles.pluginEnable : t.profiles.pluginDisable
                    }`}
                    checked={!shellDisabled}
                    disabled={props.opBusy}
                    onCheckedChange={props.onToggle}
                  />
                </div>
              )}

              {manageable && (
                <Popover>
                  <PopoverTrigger asChild>
                    <Button
                      size="icon-sm"
                      variant="ghost"
                      aria-label={t.profiles.rowMoreActions(p.name)}
                      title={t.profiles.rowMoreActions(p.name)}
                      disabled={props.opBusy}
                    >
                      <MoreHorizontal className="text-faint size-3.5" />
                    </Button>
                  </PopoverTrigger>
                  <PopoverContent align="end" className="w-52 p-1.5">
                    <button
                      type="button"
                      disabled={props.opBusy}
                      onClick={props.onUpdate}
                      className="text-ink hover:bg-wash flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-xs transition-colors disabled:opacity-50"
                    >
                      <ArrowUpCircle className="text-dim size-3.5" />
                      <span>{t.profiles.pluginUpdate}</span>
                      {props.latest && props.latest !== p.installed_version && (
                        <span className="text-brand-deep ml-auto font-mono text-meta">
                          {props.latest}
                        </span>
                      )}
                    </button>
                    <button
                      type="button"
                      disabled={props.opBusy}
                      onClick={props.onVersionPick}
                      className="text-ink hover:bg-wash flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-xs transition-colors disabled:opacity-50"
                    >
                      <Package className="text-dim size-3.5" />
                      <span>{t.profiles.updateHint}</span>
                    </button>
                    <div className="bg-line my-1 h-px" />
                    {/* 卸载不可撤销：destructive 语义 + 确认框（与其余行内删除统一） */}
                    <button
                      type="button"
                      disabled={props.opBusy}
                      onClick={props.onRemove}
                      className="text-danger hover:bg-danger-soft flex w-full items-center gap-2 rounded-lg px-2.5 py-1.5 text-left text-xs transition-colors disabled:opacity-50"
                    >
                      <Trash2 className="size-3.5" />
                      <span>{t.profiles.pluginUninstall}</span>
                    </button>
                  </PopoverContent>
                </Popover>
              )}
            </>
          )}
        </div>
      )}
    </div>
  )
}
