// mcpForm.test.ts —— MCP 表单纯逻辑的回归（2026-09-18 三修）。
//
// 为什么单独测这三件事：它们的失败方式都是**静默**的——参数切错会写坏 YAML 数组、
// 重名判定算错序号会让 UI 指错"该删哪一条"、serverName 放过非法字符会让**整行插件
// 加载被上游拒收**。界面没有断言通道（AGENTS §4.4 不引 DOM 测试栈），故逻辑抽在
// `lib/mcpForm.ts` 里纯测。
import { describe, expect, it } from "vitest"

import { SERVER_NAME_RE, dupInfo, joinArgs, splitArgs } from "@/lib/mcpForm"
import type { McpServerConfig } from "@/types/ipc"

/** 造一行：只需要 name/scope/rowId 参与判定，其余给默认。 */
function row(
  name: string,
  scope: McpServerConfig["scope"],
  rowId = "",
): McpServerConfig {
  return { name, command: "pnpm", args: [], env: {}, disabled: false, scope, rowId }
}

describe("serverName 上游约束（与 mcp.rs::validate_server_name 同源）", () => {
  it("合法：字母数字下划线连字符，1~32 位", () => {
    for (const ok of ["a", "dsh-web", "web_search", "Server1", "A-B_c9".repeat(4)]) {
      expect(SERVER_NAME_RE.test(ok), ok).toBe(true)
    }
  })

  it("非法：中文 / 点号 / 空格 / 空串 / 超 32 位一律拒", () => {
    // 名称会进工具名 `mcp__<名称>__<工具>`，放过去的是**整行加载失败**。
    for (const bad of ["", "中文", "a.b", "a b", "a/b", "a:b", "x".repeat(33)]) {
      expect(SERVER_NAME_RE.test(bad), bad).toBe(false)
    }
  })
})

describe("args 的空格语义（切错 = 保存即写坏配置）", () => {
  it("引号内的空格算一个参数，引号剥掉", () => {
    expect(splitArgs('/usr/local/bin/node /app/server.js --desc="a b"')).toEqual([
      "/usr/local/bin/node",
      "/app/server.js",
      "--desc=a b",
    ])
    expect(splitArgs("node server.js --msg='hello world'")).toEqual([
      "node",
      "server.js",
      "--msg=hello world",
    ])
  })

  it("Windows 含空格路径是一个参数，不是两个", () => {
    expect(splitArgs('"/Program Files/node/node.exe" "/srv/app index.js"')).toEqual([
      "/Program Files/node/node.exe",
      "/srv/app index.js",
    ])
  })

  it("多余空白 / 首尾空白 / 空串都不产生空参数", () => {
    expect(splitArgs("   a    b  ")).toEqual(["a", "b"])
    expect(splitArgs("")).toEqual([])
    expect(splitArgs("   ")).toEqual([])
  })

  it("未闭合引号按已输入内容处理（不得吞掉整行）", () => {
    expect(splitArgs('node "a b')).toEqual(["node", "a b"])
  })

  it("joinArgs ∘ splitArgs 往返恒等（打开编辑再保存不得改坏参数）", () => {
    for (const list of [
      ["node", "server.js"],
      ["/Program Files/node/node.exe", "/srv/app index.js"],
      ["--desc=a b", "-y"],
      ["带 空格 的中文参数"],
      [],
    ]) {
      expect(splitArgs(joinArgs(list))).toEqual(list)
    }
  })

  it("joinArgs 只给含空白的参数加引号", () => {
    expect(joinArgs(["a", "b c"])).toBe('a "b c"')
    // 往返的另一半：不加引号的裸空格必须被切开，否则说明 join 漏了转义。
    expect(splitArgs(joinArgs(["a", "b c"]))).toEqual(["a", "b c"])
  })
})

describe("重名事实（决定 UI 说哪句话、指哪一条）", () => {
  it("唯一行：total 1 / rank 1，两种告警都不触发", () => {
    const servers = [row("alpha", "profile"), row("beta", "global")]
    const d = dupInfo(servers, "alpha", "profile", 0)
    expect(d).toEqual({ total: 1, rank: 1, sameScope: false, crossScope: false })
  })

  it("同层两条：各自拿到加载序号，sameScope 为真", () => {
    const servers = [row("dupe", "profile", "mcp-dupe"), row("dupe", "profile", "mcp-dupe-2")]
    expect(dupInfo(servers, "dupe", "profile", 0).rank).toBe(1)
    const second = dupInfo(servers, "dupe", "profile", 1)
    expect(second).toEqual({ total: 2, rank: 2, sameScope: true, crossScope: false })
  })

  it("跨层两条：profile 先加载 ⇒ 全局那条 rank 2 且 crossScope 为真", () => {
    // 全局列表把 profile 行排在前面，故全局行的加载序还要压上所有同名 profile 行。
    const servers = [row("x", "profile", "mcp-x"), row("x", "global", "mcp-x")]
    expect(dupInfo(servers, "x", "profile", 0)).toEqual({
      total: 2,
      rank: 1,
      sameScope: false,
      crossScope: true,
    })
    expect(dupInfo(servers, "x", "global", 1).rank).toBe(2)
  })

  it("三种都有：rank 只数同一层之前的行，跨层的 profile 行整体压在全局层前面", () => {
    const servers = [
      row("y", "profile", "mcp-y-1"),
      row("y", "profile", "mcp-y-2"),
      row("y", "global", "mcp-y-3"),
      row("y", "global", "mcp-y-4"),
    ]
    expect(dupInfo(servers, "y", "profile", 1)).toEqual({
      total: 4,
      rank: 2,
      sameScope: true,
      crossScope: true,
    })
    // 全局第一条：前面有 2 条 profile 同名 ⇒ rank 3（它必然实例化失败）。
    expect(dupInfo(servers, "y", "global", 2).rank).toBe(3)
    expect(dupInfo(servers, "y", "global", 3).rank).toBe(4)
  })

  it("scope 缺省按 profile 判定（后端旧数据可能不带 scope）", () => {
    const servers = [row("z", undefined), row("z", undefined)]
    expect(dupInfo(servers, "z", "profile", 1)).toEqual({
      total: 2,
      rank: 2,
      sameScope: true,
      crossScope: false,
    })
  })

  it("不同名字互不影响", () => {
    const servers = [row("a", "profile"), row("b", "profile")]
    expect(dupInfo(servers, "b", "profile", 1).total).toBe(1)
  })
})
