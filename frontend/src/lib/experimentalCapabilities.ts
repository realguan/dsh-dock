// experimentalCapabilities.ts —— 实验能力的**开/关/换后端**编排纯逻辑
//（2026-09-15 立，ADR-0020；2026-09-16 第二次修订：目录行 → 能力开关）。
//
// 为什么单独成模块：一个能力不是"装一个包"，而是**有序操作序列**，且**三种动作语义不同**：
//   ① 开启（enable）：同族旧后端先让位（同族并存会激活失败）→ 逐步串行安装 →
//      装完的包逐条确保挂载行（写不写行由**后端写前当场重判**，前端不猜）；
//   ② 关闭（disable）：**只写行级 disabled，不卸载**——秒级可逆、不必重下；
//      仅当该变体每步都是壳写的行时才可用（`toggleOffSupported`）；
//   ③ 移除（remove）：清停用桩 → 删壳写的行（否则留下**悬空挂载行**）→ 逆序卸包。
//
// 顺序即语义（Agent Teams 的 Web 层装反了会在 Team 服务就位前挂载而失败），
// 所以这里做成可单测的纯函数 + 注入式 IO，UI 只负责渲染进度与状态。
//
// IO 注入沿用 `lib/shellSettings.ts` 的既有模式（纯逻辑可测、生产绑定单列）。

import type {
  Capability,
  CapabilityVariant,
  FailureKind,
  PluginOpOutcome,
  RowWriteOutcome,
} from "@/types/ipc"

/** 编排所需的最小 IO 面（注入以便纯逻辑单测；生产绑定见 `capabilityIO`）。 */
export interface CapabilityIO {
  /** 安装/升级一个包（spec 已由后端钉版本）。`pkg` 单列是为了让队列面板显示包名。 */
  install: (profile: string, pkg: string, spec: string) => Promise<PluginOpOutcome>
  /** 移除一个包（换后端 / 彻底移除时使用）。 */
  remove: (profile: string, pkg: string) => Promise<PluginOpOutcome>
  /** 确保挂载行就位；后端**写前当场重判**该包是否声明 `dsh.bundle`。 */
  ensureRow: (profile: string, rowId: string, pkg: string) => Promise<RowWriteOutcome>
  /** 删除壳写过的挂载行（只接受 `dsh-dock-` 前缀）。 */
  deleteRow: (profile: string, rowId: string) => Promise<boolean>
  /** 行级停用/启用（`disabled` 单键写 patch；不卸载包）。 */
  setDisabled: (profile: string, rowId: string, disabled: boolean) => Promise<void>
}

/** 一步操作。`step` = 该步在变体里的序号（1 起，用于进度文案"第 2/2 步"）。 */
export type CapabilityOp =
  | { readonly kind: "install"; readonly step: number; readonly package: string; readonly spec: string }
  | { readonly kind: "remove"; readonly step: number; readonly package: string }
  | { readonly kind: "ensureRow"; readonly step: number; readonly package: string; readonly rowId: string }
  | { readonly kind: "deleteRow"; readonly step: number; readonly package: string; readonly rowId: string }
  | { readonly kind: "setDisabled"; readonly rowId: string; readonly disabled: boolean }

/** 执行进度（UI 据此渲染"正在安装 / 正在写配置"）。 */
export interface CapabilityProgress {
  readonly index: number
  readonly total: number
  readonly op: CapabilityOp
}

/** 执行结果。失败时 `completedOps` 即"已一致的步数"，据此可续跑。 */
export interface RunResult {
  readonly ok: boolean
  readonly completedOps: number
  readonly totalOps: number
  readonly failedAt: {
    readonly index: number
    readonly op: CapabilityOp
    readonly error: string
    /** 失败分类（**后端给**，见 `types/ipc.ts` 的 `FailureKind`）：界面据此选文案。 */
    readonly failureKind: FailureKind | null
  } | null
}

function variantOf(cap: Capability, variantId: string): CapabilityVariant {
  const found = cap.variants.find((v) => v.id === variantId)
  if (!found) throw new Error(`能力「${cap.id}」没有变体「${variantId}」`)
  return found
}

/** 按 `ordinal` 排序的步骤（顺序是语义，不依赖数组恰好有序）。 */
function ordered(v: CapabilityVariant) {
  return [...v.steps].sort((a, b) => a.ordinal - b.ordinal)
}

/**
 * **开启 / 修复**：只做缺的那部分——已装的包不重装、行齐的不重写。
 *
 * 步骤（**不含"让位"**）：
 * ① 逐包串行安装 → 紧跟该步确保挂载行（**写不写行由后端重判**，bundle 类自动激活）；
 * ② 把被停用的行重新启用（这是"关而不卸"能秒回的原因）。
 *
 * **为什么不在这里拆同族旧后端**：拆必须"先删行、再卸包"，而这里只发 `remove`——
 * 2026-09-16 独立评审核出：那样会留下**悬空挂载行**（指向已卸的包，dsh 启动即失败），
 * 正是 D5 那个病换了个入口。让位职责**只归 [`planReplace`]**（它用 [`teardownOps`]，
 * 删行与卸包成对）。对外请用 `planReplace`，本函数是它的一半。
 */
export function planEnable(cap: Capability, variantId: string): CapabilityOp[] {
  const variant = variantOf(cap, variantId)
  const ops: CapabilityOp[] = []

  const steps = ordered(variant)
  for (const step of steps) {
    if (!step.installed) {
      ops.push({ kind: "install", step: step.ordinal, package: step.package, spec: step.spec })
    }
    // 行缺失才补：`rowPresent` 已含 bundle 类"由包自身激活"的情形。
    if (!step.rowPresent) {
      ops.push({
        kind: "ensureRow",
        step: step.ordinal,
        package: step.package,
        rowId: step.rowId,
      })
    }
  }

  for (const step of steps) {
    if (!step.disabled) continue
    for (const target of step.toggleTargets) {
      ops.push({ kind: "setDisabled", rowId: target, disabled: false })
    }
  }
  return ops
}

/**
 * **关闭**：只写行级 disabled，**保留包**。
 *
 * 前置：调用方必须确认 `variant.toggleOffSupported`——含 profile 层的变体不能用行级开关
 * 关干净（层的 patch 带副作用，实测会停用旧行），那种情况走 `planRemove`。
 */
export function planDisable(variant: CapabilityVariant): CapabilityOp[] {
  if (!variant.toggleOffSupported) {
    throw new Error(
      `变体「${variant.id}」由 profile 层提供，行级停用无法回滚层的副作用，须走移除`,
    )
  }
  const ops: CapabilityOp[] = []
  for (const step of ordered(variant)) {
    for (const target of step.toggleTargets) {
      ops.push({ kind: "setDisabled", rowId: target, disabled: true })
    }
  }
  return ops
}

/**
 * 拆解一个变体：清停用桩 → 删壳写的行 → 逆序卸包。
 *
 * `keep` = 不要动的包（换后端时用来留住两变体共享的基座包）。
 *
 * **删行必须在卸包之前**：包先没了而 `- insert:` 行还在，就是**悬空挂载行**
 * （指向不存在的包，dsh 启动即加载失败）——v1 只有写行、没有反向原语，正是这个坑。
 */
function teardownOps(variant: CapabilityVariant, keep: readonly string[] = []): CapabilityOp[] {
  const steps = ordered(variant).filter((s) => !keep.includes(s.package))
  const ops: CapabilityOp[] = []

  // ① 停用桩先收回：`{id, disabled}` 是壳写下的补丁，行要没了它就成了指向空 id 的垃圾。
  for (const step of steps) {
    if (!step.disabled) continue
    for (const target of step.toggleTargets) {
      ops.push({ kind: "setDisabled", rowId: target, disabled: false })
    }
  }
  // ② 删壳写的行（bundle 自身的行随包一起消失，不用也不能代删）。
  for (const step of steps) {
    if (step.activation === "insert_row" && step.rowPresent) {
      ops.push({ kind: "deleteRow", step: step.ordinal, package: step.package, rowId: step.rowId })
    }
  }
  // ③ 逆序卸包：依赖者先走（Web 层在宿主层之前）。
  for (const step of [...steps].reverse()) {
    if (step.installed) {
      ops.push({ kind: "remove", step: step.ordinal, package: step.package })
    }
  }
  return ops
}

/** **彻底移除**：把该变体的包与壳写的行一起收干净（含停用态）。 */
export function planRemove(cap: Capability, variantId: string): CapabilityOp[] {
  return teardownOps(variantOf(cap, variantId))
}

/**
 * **换后端 / 冲突修复 / 直接开启** —— 三件事同一个动作：先把目标变体以外**已就位**的
 * 同能力变体拆掉（共享包留住），再开启目标变体。
 *
 * 为什么合成一个：同族并存会**激活失败**，所以"开启"这件事在语义上本就蕴含"让位"。
 * 拆开写会让"两个后端同时生效"这种异常态没有单一入口可修（多入口 = 有的入口漏了让位）。
 * 没有任何其它变体就位时，它退化为 [`planEnable`]（同一个函数，不另算）。
 *
 * **为什么是"先拆后装"而不是"先装后拆"**（2026-09-16 独立评审提出过反向顺序）：两种顺序
 * 都有失败窗口，但**窗口的后果不同**。先装后拆时，若拆除阶段失败，patch 里会同时留下两个
 * 同族后端的行——那会在**下次 dsh 启动时激活失败**（用户的整棵插件树都起不来）。先拆后装
 * 失败时，用户丢掉的是"旧后端还在"这一便利，但 patch 恒处于**没有同族并存**的一致态，
 * dsh 照常启动。本仓库的一贯取舍是：宁可少一个便利，也不留能让启动失败的状态。
 * （安装本身不触发激活——激活发生在 dsh 组合树加载时，即重启之后。）
 */
export function planReplace(cap: Capability, variantId: string): CapabilityOp[] {
  const to = variantOf(cap, variantId)
  const keep = to.steps.map((s) => s.package)
  const others = cap.variants.filter(
    (v) => v.id !== variantId && v.steps.some((s) => s.installed || s.rowPresent),
  )
  return [...others.flatMap((v) => teardownOps(v, keep)), ...planEnable(cap, variantId)]
}

/**
 * 变体在界面上的**标识包**与**共用包**（2026-09-17 维护者裁定「实际上就是插件，
 * 名字使用插件名就行了」）。
 *
 * 数据事实：一个"后端"本就是一串插件——`browser-use` 的三档都是
 * `@deepseek-ai/dsh-browser-use`（基座）+ 各自的 provider 包；Agent Teams 的
 * 「Web 档」是基座 + `-web-profile`，而「自建档」**就是基座本身**。所以界面不该再造
 * 「Web 档 / 复用已装的 cua-driver」这类我们发明的名字，直接说插件名，并如实区分：
 *
 * - `primary` = **该变体独有**的包（兄弟变体没有的）→ 它才是"这一档是什么"；
 * - `shared`  = 兄弟变体也带的基座包（换后端时"留住的共享基座"，与 [`planReplace`]
 *   的 `keep` 同一语义）；
 * - 没有任何独有包时（「自建档」那种）→ `primary` 退回它自己的全部包、`shared` 为空：
 *   它的标识就是基座包本身，不该显示成"什么也没有"。
 *
 * 包序沿用步骤序（宿主层在前、provider 在后），与安装顺序同源。
 */
export function variantPackageRoles(
  cap: Capability,
  variantId: string,
): { primary: readonly string[]; shared: readonly string[] } {
  const me = cap.variants.find((v) => v.id === variantId)
  if (!me) return { primary: [], shared: [] }
  const mine = ordered(me).map((s) => s.package)
  const others = new Set(
    cap.variants
      .filter((v) => v.id !== variantId)
      .flatMap((v) => v.steps.map((s) => s.package)),
  )
  const unique = mine.filter((p) => !others.has(p))
  if (unique.length === 0) return { primary: mine, shared: [] }
  return { primary: unique, shared: mine.filter((p) => !unique.includes(p)) }
}

/**
 * 该能力**所有变体共有**的前置条件（交集）——卡片级只讲一次，行内只讲各档**额外**的。
 *
 * 为什么要去重（2026-09-17 版面复盘）：Chromium 这条前置写在三个浏览器档里，三行完全
 * 相同；而"本机需安装 Chrome"（Chrome DevTools 档）才是切档时要对比的信息。重复行把
 * 真正的差异淹掉了——去重后卡片短两行、差异一眼可见。
 */
export function commonPrerequisites(cap: Capability): readonly string[] {
  if (cap.variants.length === 0) return []
  const [first, ...rest] = cap.variants
  return first.prerequisitesZh.filter((p) => rest.every((v) => v.prerequisitesZh.includes(p)))
}

/**
 * **"关掉它"是否等价于"移除它"**（层类能力）——判定必须连带确认它**真的就位**。
 *
 * `toggle_off_supported` 只在 `state !== "off"` 时才有意义：未装齐时后端读不到包的 manifest，
 * 一律给 `false`。"未装齐"包含**装到一半失败**（`partial`）——那种档并不是层类能力，
 * 只是我们还没资格判定。所以判据是 `!toggle_off_supported && (on | disabled)`。
 *
 * 为什么单列成函数（2026-09-17 独立复核）：这条判据曾被写进 `off_is_remove` 一处、
 * 另外三处（移除确认框的 layer 说明与要点、详情面的移除说明）却照着裸
 * `toggle_off_supported` 渲染 → 一次失败安装就能让面板谎称"该能力由 profile 层提供"，
 * 而这句话是**假的**（browser-use 是纯 insert_row，根本不是层）。
 * 四处共用本函数，才不会有第五处再漏。
 */
export function offMeansRemove(variant: CapabilityVariant): boolean {
  return (
    !variant.toggleOffSupported && (variant.state === "on" || variant.state === "disabled")
  )
}

/**
 * 该能力**所有变体都带**的共用包（交集）——详情面只讲一次，变体行里只留各档**额外**的。
 *
 * 为什么（2026-09-17 版面复盘）：`browser-use` 三档各自都是「基座 `dsh-browser-use` +
 * 各自的 provider 包」，于是"另装共用包 …"在三条变体行里原样重复三遍，把真正用来区分
 * 各档的那一行淹掉了——与 [`commonPrerequisites`] 是同一类噪音、同一类修法。
 *
 * 只对**全体共有**的包生效：Agent Teams 的自建档不带 Web 层，它的 `shared` 是空集，
 * 交集为空 ⇒ 继续按行标注（不为了"整齐"而吞掉信息）。
 */
export function commonSharedPackages(cap: Capability): readonly string[] {
  if (cap.variants.length === 0) return []
  const [first, ...rest] = cap.variants
  const candidates = variantPackageRoles(cap, first.id).shared
  return candidates.filter((p) => rest.every((v) => variantPackageRoles(cap, v.id).shared.includes(p)))
}

/** 变体的**界面名** = 其标识包（多个时以 ` + ` 相连）。确认框、替换提示、失败点名都用它
 *  ——各处对"哪个后端"的称呼保持一致，不再出现卡片上已不存在的「Web 档」这类名字。 */
export function variantDisplayName(cap: Capability, variantId: string): string {
  return variantPackageRoles(cap, variantId).primary.join(" + ")
}

/** 插件操作失败：把后端的分类一起带出来（组件据此选文案，不再自己正则判一次）。 */
class PluginOpFailed extends Error {
  readonly failureKind: FailureKind | null
  constructor(message: string, failureKind: FailureKind | null) {
    super(message)
    this.name = "PluginOpFailed"
    this.failureKind = failureKind
  }
}

/** 把 `PluginOpOutcome` 的**软失败**也当失败——IPC 成功解析但 `ok:false` 是常态
 *  （pnpm 审批门、包不存在等）。漏判它会变成"界面报成功、实际没装上"。 */
function requireOk(what: string, outcome: PluginOpOutcome): void {
  if (!outcome.ok) {
    throw new PluginOpFailed(`${what}未成功：${outcome.detail}`, outcome.failureKind ?? null)
  }
}

/**
 * **严格串行**执行操作序列（绝不并发）。
 *
 * 任一步失败即刻停止并返回 `completedOps`：调用方据此呈现"已完成 N/M，可继续"，
 * 而不是把半成品当成功。失败**不是**回滚——已完成的步本就处于一致状态
 * （装可重入、行幂等、disabled 可再切），回滚反而会把用户别的插件一起动到。
 */
export async function runCapabilityOps(
  io: CapabilityIO,
  profile: string,
  ops: readonly CapabilityOp[],
  onProgress?: (p: CapabilityProgress) => void,
): Promise<RunResult> {
  const total = ops.length
  for (let index = 0; index < total; index += 1) {
    const op = ops[index]
    onProgress?.({ index, total, op })
    try {
      switch (op.kind) {
        case "install":
          requireOk(`安装「${op.package}」`, await io.install(profile, op.package, op.spec))
          break
        case "remove":
          requireOk(`移除「${op.package}」`, await io.remove(profile, op.package))
          break
        case "ensureRow":
          await io.ensureRow(profile, op.rowId, op.package)
          break
        case "deleteRow":
          await io.deleteRow(profile, op.rowId)
          break
        case "setDisabled":
          await io.setDisabled(profile, op.rowId, op.disabled)
          break
      }
    } catch (e) {
      return {
        ok: false,
        completedOps: index,
        totalOps: total,
        failedAt: {
          index,
          op,
          error: String(e),
          failureKind: e instanceof PluginOpFailed ? e.failureKind : null,
        },
      }
    }
  }
  return { ok: true, completedOps: total, totalOps: total, failedAt: null }
}

/** 生产绑定：`install`/`remove` 走**下载队列**（可 await），其余直连 IPC。
 *
 * 下面三处 `await import(...)` 是**有意的动态引入**（沿用本模块前身 `officialCatalog.ts`
 * 的既有形态）：组件层已静态引入 queueStore / tauri 门面，这里静态引入会在模块求值期
 * 形成回环。代价是构建期有一条 `INEFFECTIVE_DYNAMIC_IMPORT` 警告（rollup 判定该模块
 * 反正会进主 chunk）——**警告非错误，且与收敛前的构建输出一致**。 */
export const capabilityIO: CapabilityIO = {
  install: (profile, pkg, spec) => enqueueOp(profile, pkg, spec, "install"),
  remove: (profile, pkg) => enqueueOp(profile, pkg, "", "remove"),
  ensureRow: (profile, rowId, pkg) => apiEnsureRow(profile, rowId, pkg),
  deleteRow: (profile, rowId) => apiDeleteRow(profile, rowId),
  setDisabled: (profile, rowId, disabled) => apiSetDisabled(profile, rowId, disabled),
}

// 安装与卸载**都经** `stores/queueStore.ts` 的队列，用可 await 的 `enqueueAndWait`
// 拿逐步成败：这样能力开关的包操作会出现在「下载管理」面板里，复用既有的失败重试
// 入口与 toast 链；串行语义不变（`runCapabilityOps` 逐步 await，队列本身也串行）。
//
// 两条**有意保留**的边界：
//  ① 行操作（写/删/停用）不入队——那是一次配置文件的原子写，不是一次包操作：队列项
//     承载的是 pnpm 下载/安装的进度与重试，把配置文件写进去会让「重试」的含义变模糊。
//  ② 队列的「重试」只重跑**该项**，不会续跑 `runCapabilityOps` 的后续步骤；重试成功后
//     面板显示该项已完成，用户需重新触发能力开关以续跑剩余步骤——这是有意的：
//     自动续跑需要队列持有调用方上下文，会让队列从「下载管理」变成「工作流引擎」。

/** 队列项 → `PluginOpOutcome`（`detail` 在 IPC 契约里是必填 string）。 */
async function enqueueOp(
  profile: string,
  pkg: string,
  spec: string,
  kind: "install" | "remove",
): Promise<PluginOpOutcome> {
  const { useQueueStore } = await import("@/stores/queueStore")
  const outcome = await useQueueStore.getState().enqueueAndWait({ pkg, spec, profile, kind })
  return {
    ok: outcome.ok,
    detail: outcome.detail ?? "",
    failureKind: outcome.failureKind ?? null,
  }
}

async function apiEnsureRow(
  profile: string,
  rowId: string,
  pkg: string,
): Promise<RowWriteOutcome> {
  const { api } = await import("./tauri")
  return api.applyOfficialPatchRow(profile, rowId, pkg)
}

async function apiDeleteRow(profile: string, rowId: string): Promise<boolean> {
  const { api } = await import("./tauri")
  return api.removeOfficialPatchRow(profile, rowId)
}

async function apiSetDisabled(
  profile: string,
  rowId: string,
  disabled: boolean,
): Promise<void> {
  const { api } = await import("./tauri")
  return api.setPluginDisabled(profile, rowId, disabled)
}
