// mcp.test.ts —— MCP 服务器结构化管理与运行态工具联动单测（4.7）。
import { describe, expect, it } from "vitest"
import type { PluginRuntimeSnapshot } from "@/types/ipc"

describe("MCP Server Manager & Runtime Linkage", () => {
  it("converts between env object and form array representation", () => {
    const envObj: Record<string, string> = {
      GITHUB_TOKEN: "ghp_123456",
      API_KEY: "secret-key",
    }

    // Object to Form array
    const formArray = Object.entries(envObj).map(([key, value]) => ({ key, value }))
    expect(formArray.length).toBe(2)
    expect(formArray[0]).toEqual({ key: "GITHUB_TOKEN", value: "ghp_123456" })

    // Form array back to Object
    const backToObj: Record<string, string> = {}
    for (const item of formArray) {
      if (item.key.trim()) {
        backToObj[item.key.trim()] = item.value.trim()
      }
    }
    expect(backToObj).toEqual(envObj)
  })

  it("extracts active MCP runtime tools grouped by server name", () => {
    const mockRuntime: PluginRuntimeSnapshot = {
      profile: "dev-profile",
      entries: [
        {
          entry_id: "entry-1",
          module_name: "mcp__github__create_issue",
          enabled: true,
          fiber_phase: "active",
        },
        {
          entry_id: "entry-2",
          module_name: "mcp__github__search_repos",
          enabled: true,
          fiber_phase: "active",
        },
        {
          entry_id: "entry-3",
          module_name: "mcp__filesystem__read_file",
          enabled: true,
          fiber_phase: "active",
        },
        {
          entry_id: "entry-4",
          module_name: "standard_plugin_tool",
          enabled: true,
          fiber_phase: "active",
        },
      ],
    }

    const map = new Map<string, string[]>()
    for (const entry of mockRuntime.entries) {
      const id = entry.module_name || entry.entry_id || ""
      if (id.startsWith("mcp__")) {
        const parts = id.split("__")
        if (parts.length >= 3) {
          const srv = parts[1]
          const tool = parts.slice(2).join("__")
          if (!map.has(srv)) map.set(srv, [])
          map.get(srv)!.push(tool)
        }
      }
    }

    expect(map.size).toBe(2)
    expect(map.get("github")).toEqual(["create_issue", "search_repos"])
    expect(map.get("filesystem")).toEqual(["read_file"])
    expect(map.has("standard_plugin_tool")).toBe(false)
  })

  it("splits arguments correctly from user input", () => {
    const rawArgs = "-y @modelcontextprotocol/server-postgres postgresql://localhost/db"
    const parsed = rawArgs.trim().split(/\s+/).filter(Boolean)
    expect(parsed).toEqual([
      "-y",
      "@modelcontextprotocol/server-postgres",
      "postgresql://localhost/db",
    ])
  })
})
