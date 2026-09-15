// officialCatalog.ts —— 官方策展条目的**安装编排纯逻辑**（2026-09-15，ADR-0020）。
//
// 为什么单独成模块：目录条目不是"装一个包"，而是**有序操作序列**：
//   ① 同族冲突时先**移除**旧 provider（替换而非叠加）；
//   ② 逐包**串行**安装（`dsh plugin add` 一次只接受一个包）；
//   ③ 声明了 `dsh.bundle` 的包由 CLI 自行激活（**不写 patch**）；
//      未声明的包必须**紧跟该步**写一条挂载行。
// 顺序即语义（Agent Teams 的 Web 层装反了会在 Team 服务就位前挂载而失败），
// 所以这里做成可单测的纯函数 + 注入式 IO，UI 只负责渲染进度。
//
// IO 注入沿用 `lib/shellSettings.ts` 的既有模式（纯逻辑可测、生产绑定单列）。

import type {
  OfficialCatalogRow,
  OfficialCatalogStep,
  PluginOpOutcome,
} from "@/types/ipc"

/** 编排所需的最小 IO 面（注入以便纯逻辑单测；生产绑定见 `officialCatalogIO`）。 */
export interface OfficialCatalogIO {
  /** 安装/升级一个包（spec 已由后端钉版本）。pkg 单列是为了让队列面板显示包名。 */
  install: (profile: string, pkg: string, spec: string) => Promise<PluginOpOutcome>
  /** 移除一个包（同族替换时使用）。 */
  remove: (profile: string, pkg: string) => Promise<PluginOpOutcome>
  /** 写挂载行；返回是否改动了文件（幂等时 false）。 */
  writeRow: (profile: string, rowId: string, pkg: string) => Promise<boolean>
}

/** 一步操作。 */
export type CatalogOp =
  | { readonly kind: "remove"; readonly package: string }
  | {
      readonly kind: "install"
      readonly ordinal: number
      readonly package: string
      readonly spec: string
      readonly activation: OfficialCatalogStep["activation"]
    }
  | { readonly kind: "writeRow"; readonly package: string; readonly rowId: string }

/** 条目的一次完整下发计划。 */
export interface CatalogRunPlan {
  readonly label: string
  readonly ops: readonly CatalogOp[]
  /** 非 null = 计划会先移除这个同族 provider（替换语义）。 */
  readonly replaces: string | null
}

/**
 * 纯函数：目录行 → **有序**操作序列。
 *
 * 排序依据是后端给的 `ordinal`（而非数组下标）——顺序是语义，UI/编排都不应依赖
 * 数组恰好有序；乱序输入按 `ordinal` **纠正**（见单测）。
 *
 * 同族冲突（`conflictWith`）先插一步 `remove`：ADR-0020 §2.9 要求"替换"而非"叠加"
 * （同族第二个 provider 会**激活失败**）。
 */
export function planEntryRun(row: OfficialCatalogRow): CatalogRunPlan {
  const steps = [...row.steps].sort((a, b) => a.ordinal - b.ordinal)
  const ops: CatalogOp[] = []
  if (row.conflictWith !== null) {
    ops.push({ kind: "remove", package: row.conflictWith })
  }
  for (const step of steps) {
    ops.push({
      kind: "install",
      ordinal: step.ordinal,
      package: step.package,
      spec: step.spec,
      activation: step.activation,
    })
    // 只有未声明 dsh.bundle 的包才需要壳写挂载行；bundle 类由 CLI 自行激活，
    // 再写 insert 会**重复挂载**（行身份是 id）。
    if (step.activation === "insert_row") {
      ops.push({
        kind: "writeRow",
        package: step.package,
        rowId: step.rowId,
      })
    }
  }
  return { label: row.labelZh, ops, replaces: row.conflictWith }
}

/** 执行进度（UI 据 phase 渲染"正在安装 / 正在写配置"）。 */
export interface CatalogProgress {
  readonly index: number
  readonly total: number
  readonly op: CatalogOp
}

/** 执行结果。失败时 `completedOps` 即"已一致的步数"，据此可续装。 */
export interface CatalogRunResult {
  readonly ok: boolean
  readonly completedOps: number
  readonly totalOps: number
  readonly failedAt: {
    readonly index: number
    readonly op: CatalogOp
    readonly error: string
  } | null
}

/** 把 `PluginOpOutcome` 的**软失败**也当失败——IPC 成功解析但 `ok:false` 是常态
 * （pnpm 审批门、包不存在等）。漏判它会变成"界面报成功、实际没装上"。 */
function requireOk(what: string, outcome: PluginOpOutcome): void {
  if (!outcome.ok) {
    throw new Error(`${what}未成功：${outcome.detail}`)
  }
}

/**
 * **严格串行**执行计划（绝不并发）。
 *
 * 任一步失败即刻停止并返回 `completedOps`：调用方据此呈现"已完成 N/M，可续装"，
 * 而不是把半成品当成功。失败**不是**回滚——已完成的步本就处于一致状态
 * （`insert` 行幂等、安装可重入），回滚反而会把用户别的插件一起动到。
 */
export async function runPlan(
  io: OfficialCatalogIO,
  profile: string,
  plan: CatalogRunPlan,
  onProgress?: (p: CatalogProgress) => void,
): Promise<CatalogRunResult> {
  const total = plan.ops.length
  for (let index = 0; index < total; index += 1) {
    const op = plan.ops[index]
    onProgress?.({ index, total, op })
    try {
      switch (op.kind) {
        case "remove":
          requireOk(`移除同族 provider「${op.package}」`, await io.remove(profile, op.package))
          break
        case "install":
          requireOk(`安装「${op.package}」`, await io.install(profile, op.package, op.spec))
          break
        case "writeRow":
          await io.writeRow(profile, op.rowId, op.package)
          break
      }
    } catch (e) {
      return {
        ok: false,
        completedOps: index,
        totalOps: total,
        failedAt: { index, op, error: String(e) },
      }
    }
  }
  return { ok: true, completedOps: total, totalOps: total, failedAt: null }
}

/** 生产绑定：`install`/`remove` 走**下载队列**（可 await），`writeRow` 直连 IPC。 */
export const officialCatalogIO: OfficialCatalogIO = {
  install: (profile, pkg, spec) => enqueueOp(profile, pkg, spec, "install"),
  remove: (profile, pkg) => enqueueOp(profile, pkg, "", "remove"),
  writeRow: (profile, rowId, pkg) => apiWriteRow(profile, rowId, pkg),
}

// 2026-09-15（R2 收敛，取代本模块此前的「已知偏离」记录）：安装与卸载**都经**
// `stores/queueStore.ts` 的队列，用可 await 的 `enqueueAndWait` 拿逐步成败。
// 这样目录安装/卸载会出现在「下载管理」面板里，复用既有的失败重试入口与 toast 链；
// 串行语义不变（`runPlan` 逐步 await，队列本身也串行）。
//
// 两条**有意保留**的边界：
//  ① `writeRow`（写 patch 挂载行）不入队——它是一次配置文件的原子写，不是一次包
//     操作：队列项承载的是 pnpm 下载/安装的进度与重试，把配置文件写进去会让
//     「重试」按钮的含义变模糊（重试一次 install 与重写一次 patch 是两回事）。
//  ② 队列的「重试」只重跑**该项**，不会续跑 `runPlan` 的后续步骤（编排进度在
//     `runPlan` 的调用栈里，队列不持有）。重试成功后面板显示该项已完成，用户需
//     重新触发目录安装以续跑剩余步骤——这是有意的不自动续跑：自动续跑需要队列
//     持有调用方上下文，会让队列从「下载管理」变成「工作流引擎」。

/** 队列项 → `PluginOpOutcome`（`detail` 在 IPC 契约里是必填 string）。 */
async function enqueueOp(
  profile: string,
  pkg: string,
  spec: string,
  kind: "install" | "remove",
): Promise<PluginOpOutcome> {
  const { useQueueStore } = await import("@/stores/queueStore")
  const outcome = await useQueueStore.getState().enqueueAndWait({ pkg, spec, profile, kind })
  return { ok: outcome.ok, detail: outcome.detail ?? "" }
}
async function apiWriteRow(
  profile: string,
  rowId: string,
  pkg: string,
): Promise<boolean> {
  const { api } = await import("./tauri")
  return api.applyOfficialPatchRow(profile, rowId, pkg)
}
