// mcpForm.ts —— MCP 表单的纯逻辑（2026-09-18）。
//
// 为什么抽出来：这三件事都是"错了会静默毁配置"的判定，而本仓库不引 DOM 测试栈
// （AGENTS §4.4 末段），组件里没有断言通道——抽成纯函数才有测试覆盖。
import type { McpScope, McpServerConfig } from "@/types/ipc"

/** serverName 的上游约束（与 `mcp.rs::validate_server_name` 同一条规则）。
 *  名称会进工具名 `mcp__<名称>__<工具>`，中文 / 点号 / 空格会让**整行插件加载被拒**。 */
export const SERVER_NAME_RE = /^[A-Za-z0-9_-]{1,32}$/

/** 空格分隔的参数输入 → 数组，**引号内的空格算一个参数**。
 *
 *  为什么不能裸 `split(/\s+/)`：Windows 上 `/Program Files/...` 这类路径、以及
 *  `--desc="a b"` 这类值都会被切成两个参数——切错了保存就写坏，而且界面上看不出来。
 *  引号本身被剥掉（写进 YAML 数组的是剥引号后的值）。
 *
 *  与 [`joinArgs`] 互为逆运算：`splitArgs(joinArgs(list))` 必须等于 `list`，
 *  否则"打开编辑再保存"会改坏参数（回归测试钉住）。 */
export function splitArgs(input: string): string[] {
  const out: string[] = []
  let cur = ""
  let quoted: '"' | "'" | null = null
  let started = false
  for (const ch of input) {
    if (quoted) {
      if (ch === quoted) quoted = null
      else cur += ch
      continue
    }
    if (ch === '"' || ch === "'") {
      quoted = ch
      started = true
      continue
    }
    if (/\s/.test(ch)) {
      if (started || cur) {
        out.push(cur)
        cur = ""
        started = false
      }
      continue
    }
    cur += ch
  }
  if (started || cur) out.push(cur)
  return out
}

/** 参数数组 → 输入框文本（含空白的参数加引号，让上面那条规则能原样读回）。 */
export function joinArgs(args: readonly string[]): string {
  return args
    .map((a) => (/\s/.test(a) ? `"${a}"` : a))
    .join(" ")
}

/** 某个 serverName 的重名事实（一次遍历算出，UI 按字段决定说哪句话）。 */
export interface DupInfo {
  /** 同名行的总数；> 1 才是重名。 */
  total: number
  /** 这一行在**加载次序**里的序号（1 起）。dsh 先应用 profile 层再应用全局层
   *  （`dsh-app-boot/lib/index.js:1005`），层内按文件顺序；而上游 `serverName` 是
   *  **加载期预留** ⇒ `rank > 1` 的那条必然实例化失败。UI 因此能说清"该删哪一条"，
   *  而不是只喊"有重复"。 */
  rank: number
  /** 同一个文件里有两条以上同名行 ⇒ **名字**不足以定位那一行：写操作必须带 `rowId`
   *  （2026-09-18 三修已在探测/删除/保存三处贯穿）。这里为真时 UI 要说的是"同层重名
   *  本身就配不对"（两条都插入、后一条抢不到 `serverName`），并点出行 id。 */
  sameScope: boolean
  /** profile 层与全局层各有一条同名。 */
  crossScope: boolean
}

const scopeOf = (s: McpServerConfig): McpScope => s.scope ?? "profile"

/** `idx` = 该行在**整个列表**里的下标（后端按层、层内按文件顺序返回，故它就是加载序）。 */
export function dupInfo(
  servers: readonly McpServerConfig[],
  name: string,
  scope: McpScope,
  idx: number,
): DupInfo {
  let sameScope = 0
  let otherScope = 0
  let sameLayerBefore = 0
  servers.forEach((s, i) => {
    if (s.name !== name) return
    if (scopeOf(s) === scope) {
      sameScope += 1
      if (i < idx) sameLayerBefore += 1
    } else {
      otherScope += 1
    }
  })
  // dsh 先应用 profile 层再应用全局层 ⇒ 全局行的前面还压着所有 profile 同名行。
  const aheadOfLayer = scope === "global" ? otherScope : 0
  return {
    total: sameScope + otherScope,
    rank: aheadOfLayer + sameLayerBefore + 1,
    sameScope: sameScope > 1,
    crossScope: otherScope > 0,
  }
}
