// mcpScope.test.ts —— MCP **生效范围 + 行身份**的结构闸门（2026-09-18，同日三修扩到行身份）。
//
// 为什么是源码结构断言而不是 DOM 测试：本仓库口径明确不引 RTL/jsdom（AGENTS §4.4
// 末段），组件渲染没有断言通道。而"哪一条"这件事有五条**会被无意破坏**的落档决策：
//
//  ① **两层都要看得见**。dsh 的用户 patch 有两层——profile 层
//     （`profiles/<名>/cordis.patch.yml`，仅本 profile）与 home 级全局层
//     （`$DSH_HOME/cordis.patch.yml`，所有 profile）。界面若只画一层，用户会看到
//     "MCP 明明在跑、面板却说没配"，并且**删不掉**全局那条。
//  ② **重名不是覆盖**。跨层与同层都一样：两条都会插入，上游 `serverName` 是加载期
//     预留 ⇒ 后加载的那条实例化失败。UI 必须把这件事说出来，而不是静默并列两行。
//  ③ **行身份 = (scope, name, 层内序号)**，而**写操作 = (scope, name, row_id)**。
//     探测结果缓存、待删/待停态、折叠态都要逐行独立；探测/删除/保存必须带 rowId。
//     只按名字处理会串行——同层重复时"点第二条动第一条"，跨层时"删一层的探测结果
//     挂到另一层"。
//  ④ **改名/改层不得留跨层同名**：先存新的、再清旧的（反序会在保存失败时丢条目）。
//  ⑤ **装配状态按行 id 匹配**，且运行态快照不得跨 profile 合并。
//
// 结构断言脆弱，所以用**窗口切片**而非全文 grep：只要求"在这段代码范围里"，
// 不绑定行号与格式。若将来重写呈现层，本测试红是**预期**——请连同上面五条决策
// 一起重新落档，而不是改断言让它变绿。
import { describe, expect, it } from "vitest"
import src from "@/components/profiles/McpManager.tsx?raw"

/** 去掉注释——注释里会引用被移除的调用，把注释当代码判红是假阳性。 */
function stripComments(code: string): string {
  return code
    .replace(/\{\/\*[\s\S]*?\*\/\}/g, "")
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^[ \t]*\/\/.*$/gm, "")
}

const code = stripComments(src)

/** 取 [from, to) 之间的一段（找不到任一端即抛——避免"窗口为空⇒断言空过"的假绿）。 */
function slice(from: string, to: string): string {
  const a = code.indexOf(from)
  const b = code.indexOf(to)
  if (a < 0) throw new Error(`窗口起点未找到：${from}`)
  if (b < 0) throw new Error(`窗口终点未找到：${to}`)
  if (b <= a) throw new Error(`窗口顺序不对：${from} → ${to}`)
  return code.slice(a, b)
}

/** 单行渲染体（行身份、探测取数、重名判定都在这几行里）。 */
const ROW = "g.rows.map(({ s, idx })"
const ROW_END = "const headers ="

describe("① 两层都可见：scope 必须逐行呈现", () => {
  it("行身份键带层内序号，且渲染时按它取探测结果", () => {
    expect(code).toContain("function rowKey(scope: McpScope, name: string, idx: number)")
    const row = slice(ROW, ROW_END)
    expect(row).toContain("rowKey(scope, s.name, idx)")
    expect(row).toContain("probes[key]")
  })

  it("分组按每行自己的 scope 归层（漏一层就等于那一层不可见）", () => {
    const groups = slice("const groups = useMemo", "const nameOk")
    expect(groups).toContain("s.scope ?? \"profile\"")
    expect(groups).toContain("\"profile\", \"global\"")
  })

  it("两条范围的文案都在（不是只画一层）", () => {
    const badge = slice("function ScopeBadge", "type KvPair")
    expect(badge).toContain("mcpScopeGlobal")
    expect(badge).toContain("mcpScopeGlobalHint")
    expect(badge).toContain("mcpScopeProfile")
    expect(badge).toContain("mcpScopeProfileHint")
  })
})

describe("② 重名必须告警，且不得去重掩盖", () => {
  it("重名事实逐行计算（含加载序 rank）", () => {
    const row = slice(ROW, ROW_END)
    expect(row).toContain("dupInfo(servers ?? [], s.name, scope, idx)")
  })

  it("同层与跨层两种告警都渲染（不是静默并列两行）", () => {
    // 窗口用代码 token 而非注释文字：stripComments 会把注释删掉。
    const notes = slice("{dup.sameScope ?", "{probeEntry ? (")
    expect(notes).toContain("mcpDupSameScope")
    expect(notes).toContain("mcpScopeConflict")
  })

  it("2026-09-19 文案收拢：结论留在表面，长句挂在 Tip 上（两者都不得少）", () => {
    // 为什么钉这一条：把长句平铺会让一行堆 4 个告警盒、配置本体被挤出屏幕；
    // 但只留悬浮更糟——悬浮是**扫不到**的，冲突必须一眼看得见。所以两侧都钉：
    // 短结论（Tag）与完整判定（长句）必须同时出现在这一行里。
    const notes = slice("{dup.sameScope ?", "{probeEntry ? (")
    expect(notes).toContain("mcpDupSameScopeTag")
    expect(notes).toContain("mcpScopeConflictTag")
    expect(notes).toContain("runtime.tag")
    // 三种状态（同层重名 / 跨层重名 / 装配）各自配一个悬浮，缺一个就是"结论有了、
    // 但没人解释为什么"。
    expect(notes.match(/<Tip\b/g), "每种告警都要有自己的悬浮说明").toHaveLength(3)
    // 表达式行：徽标 + 悬浮成对出现（徽标说"是什么"，悬浮说"因此能改什么"）。
    const badge = slice("{s.expr ? (", "copyPrefix(s.name, key)")
    expect(badge).toContain("mcpExprTag")
    expect(badge).toContain("mcpExprHint")
    expect(badge).toContain("<Tip")
  })

  it("前端**不得**按名字去重（去重会把「两条都加载」的事实藏起来）", () => {
    // 列表数据直接来自后端（后端有意返回两条）；前端任何去重都会掩盖冲突。
    expect(code).not.toContain("new Set(servers")
    expect(code).not.toContain("dedupe")
  })
})

describe("③ 探测 / 删除 / 保存都带行身份", () => {
  it("删除按完整行身份记待删项，IPC 也带 scope + rowId", () => {
    const del = slice("const handleDeleteServer = async", "const copyPrefix")
    expect(del).toContain("scope: McpScope")
    expect(del).toContain("idx: number")
    expect(del).toContain("api.deleteMcpServer(profileName, srvName, scope, rowId)")
    // 窗口终点用待删项自己的字段：`})` 在文件更早处就出现过（函数签名），会切错段。
    const pending = slice("setPendingDelete({", "dupSameLayer: dup.sameScope,")
    expect(pending).toContain("rowId: s.rowId")
    expect(pending).toContain("idx")
  })

  it("同层重名的删除确认必须点出行 id（两行同名，只说名字就是歧义）", () => {
    const confirm = slice("<ConfirmDialog", "onConfirm")
    expect(confirm).toContain("mcpDeleteRowId")
    expect(confirm).toContain("dupSameLayer")
  })

  it("探测按 (name, scope, rowId) 定位那一行", () => {
    const probe = slice("const handleProbe = async", "const forgetProbe")
    expect(probe).toContain("api.probeMcpServer(profileName, serverName, scope, rowId)")
  })

  it("编辑既有条目时把所在层带进表单（否则编辑全局条目会凭空多出一条 profile 条）", () => {
    const edit = slice("const openEditDialog = (srv: McpServerConfig)", "const applyPreset")
    expect(edit).toContain("setFormScope(srv.scope ?? \"profile\")")
  })

  it("表单默认 profile 层（保守方向：不替用户改成影响所有 profile）", () => {
    const reset = slice("const resetForm = () =>", "const openCreateDialog")
    expect(reset).toContain('setFormScope("profile")')
  })

  it("启停整行带回 + 按行记状态（只发 name+disabled 会让表达式行被判成改了配置）", () => {
    const toggle = slice("const handleToggleDisabled = async", "const copyPrefix")
    expect(toggle).toContain("{ ...srv, scope, disabled: !srv.disabled }")
    expect(toggle).toContain("rowKey(scope, srv.name, idx)")
  })
})

describe("④ 改名 / 改层不得留下跨层同名（先落新的再清旧的）", () => {
  it("编辑时记住原身份（含 rowId），新建/预设走 resetForm ⇒ null", () => {
    const edit = slice("const openEditDialog = (srv: McpServerConfig)", "const applyPreset")
    expect(edit).toContain("setFormOriginal({")
    expect(edit).toContain("rowId: srv.rowId ?? \"\"")
    const reset = slice("const resetForm = () =>", "const openCreateDialog")
    expect(reset).toContain("setFormOriginal(null)")
    for (const win of [
      slice("const openCreateDialog = () =>", "const openEditDialog"),
      slice("const applyPreset = (preset", "const handleSaveServer"),
    ]) {
      expect(win).toContain("resetForm()")
    }
  })

  it("保存后按原身份清旧条目，且**顺序是先存后删**（先删后存失败即丢条目）", () => {
    const save = slice("await api.saveMcpServer", "setDialogOpen(false)")
    const saveAt = save.indexOf("await api.saveMcpServer")
    const delAt = save.indexOf(
      "await api.deleteMcpServer(profileName, orig.name, orig.scope, orig.rowId)",
    )
    expect(saveAt).toBeGreaterThanOrEqual(0)
    expect(delAt).toBeGreaterThan(saveAt)
    // 仅在身份真的变了时才清（同层同名的普通编辑不该触发删除）
    expect(save).toContain("orig.scope !== formScope || orig.name !== srv.name")
  })

  it("新增/改名不得踩到同层另一条同名行（后端按名字 upsert = 改写另一条）", () => {
    const guard = slice("const handleSaveServer = async", "setSaving(true)")
    expect(guard).toContain("mcpNameTaken")
    expect(guard).toContain("origRowId === undefined")
  })

  it("保存载荷带完整身份（transport / cwd / scope / rowId）", () => {
    const payload = slice("const srv: McpServerConfig", "await api.saveMcpServer")
    expect(payload).toContain("cwd: isHttp ? \"\" : formCwd.trim()")
    expect(payload).toContain("scope: formScope")
    expect(payload).toContain("transport: formTransport")
    expect(payload).toContain("rowId: formOriginal?.rowId ?? \"\"")
  })
})

describe("⑤ 装配状态：配置存在 ≠ 已加载", () => {
  it("按插件行的 moduleName 匹配，不再扫 mcp__ 前缀（那条路恒空）", () => {
    const memo = slice("const runtimeByRow = useMemo", "const groups = useMemo")
    expect(memo).toContain("MCP_CLIENT_PKG")
    expect(memo).toContain("fiber_phase")
    expect(memo).not.toContain('startsWith("mcp__")')
  })

  it("四种装配结论各有文案（active / disabled / failed / 无实例）", () => {
    for (const key of [
      "mcpRuntimeActive",
      "mcpRuntimeDisabled",
      "mcpRuntimeFailed",
      "mcpRuntimeNotLoaded",
    ]) {
      expect(code).toContain(key)
    }
  })

  it("运行态快照只覆盖活跃会话的 profile（不同 profile 不得合并）", () => {
    const memo = slice("const runtimeByRow = useMemo", "const groups = useMemo")
    expect(memo).toContain("runtime.profile !== profileName")
  })

  it("徽标按 rowId 精确取，名字只做兜底（手写行 id 的行最需要看装配状态）", () => {
    const row = slice(ROW, ROW_END)
    expect(row).toContain("runtimeByRow.byId.get(s.rowId)")
    expect(row).toContain("runtimeByRow.byName.get(s.name)")
  })
})

describe("⑥ cwd 与表达式行必须能表达", () => {
  it("表单有 cwd 输入", () => {
    expect(code).toContain("mcp-form-cwd")
  })

  it("表达式行在列表与表单里都有明确文案（不能等保存才报错）", () => {
    // 表达式徽标在连接方式那一块之后 ⇒ 单独开窗，不复用 ROW 头部窗口。
    const badge = slice("{s.expr ? (", "copyPrefix(s.name, key)")
    expect(badge).toContain("mcpExprTag")
    const form = slice("const openEditDialog", "const applyPreset")
    expect(form).toContain("setFormExpr(srv.expr ?? false)")
    // 2026-09-19：表单打开时先说短结论（`mcpExprShort`），完整口径在同一条悬浮里。
    const exprStrip = slice("{formExpr ? (", 'htmlFor="mcp-form-name"')
    expect(exprStrip).toContain("mcpExprShort")
    expect(exprStrip).toContain("mcpExprHint")
  })
})
