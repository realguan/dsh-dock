import { describe, expect, it } from "vitest"
import {
  extractInstallSpec,
  filterMarketPlugins,
  getPluginDescription,
  sortMarketPlugins,
} from "@/lib/market"
import type { MarketPlugin } from "@/types/market"

describe("lib/market.ts", () => {
  describe("extractInstallSpec", () => {
    it("extracts package spec from standard dsh plugin install command", () => {
      expect(
        extractInstallSpec(
          "dsh plugin --profile web add @furongjun1999/dsh-memory",
        ),
      ).toBe("@furongjun1999/dsh-memory")
    })

    it("extracts github spec from github repo install command", () => {
      expect(
        extractInstallSpec(
          "dsh plugin --profile web add github:realguan/dsh-awesome-tool",
        ),
      ).toBe("github:realguan/dsh-awesome-tool")
    })

    it("handles single token or bare name", () => {
      expect(extractInstallSpec("@org/plugin")).toBe("@org/plugin")
      expect(extractInstallSpec("")).toBe("")
    })
  })

  describe("getPluginDescription", () => {
    it("extracts zh description when locale is zh-CN", () => {
      const desc = { en: "English text", zh: "中文描述" }
      expect(getPluginDescription(desc, "zh-CN")).toBe("中文描述")
    })

    it("falls back to en when zh is missing in zh-CN locale", () => {
      const desc = { en: "English text" }
      expect(getPluginDescription(desc, "zh-CN")).toBe("English text")
    })

    it("extracts en description when locale is en-US", () => {
      const desc = { en: "English text", zh: "中文描述" }
      expect(getPluginDescription(desc, "en-US")).toBe("English text")
    })

    it("handles plain string description", () => {
      expect(getPluginDescription("Plain string", "zh-CN")).toBe("Plain string")
      expect(getPluginDescription(null, "zh-CN")).toBe("")
    })
  })

  describe("filterMarketPlugins", () => {
    const mockPlugins: MarketPlugin[] = [
      {
        name: "dsh-memory",
        owner: "FuRongJun",
        url: "https://github.com/FuRongJun/dsh-memory",
        page: "https://awesome-dsh-plugin.com/p/dsh-memory",
        category: "agi",
        description: { zh: "白箱 AGI 记忆系统", en: "White-box AGI memory" },
        npm: "@furongjun/dsh-memory",
        stars: 120,
        downloads: 5000,
        install: "dsh plugin --profile web add @furongjun/dsh-memory",
        added: "2026-08-01",
      },
      {
        name: "dsh-plugin-theme-dracula",
        owner: "dracula",
        url: "https://github.com/dracula/dsh",
        page: "https://awesome-dsh-plugin.com/p/dracula",
        category: "theme",
        description: { zh: "经典 Dracula 暗黑主题", en: "Dracula dark theme" },
        npm: "dsh-theme-dracula",
        stars: 350,
        downloads: 12000,
        install: "dsh plugin --profile web add dsh-theme-dracula",
        added: "2026-08-15",
      },
      {
        name: "dsh-tools-calc",
        owner: "mathguy",
        url: "https://github.com/mathguy/calc",
        page: "https://awesome-dsh-plugin.com/p/calc",
        category: "tools",
        description: { zh: "计算器扩展", en: "Calculator tool" },
        npm: null,
        stars: 10,
        downloads: 200,
        install: "dsh plugin --profile web add github:mathguy/calc",
        added: "2026-08-20",
      },
    ]

    it("filters by keyword across name, npm, owner, and description", () => {
      const res1 = filterMarketPlugins({
        plugins: mockPlugins,
        query: "dracula",
        category: "all",
      })
      expect(res1).toHaveLength(1)
      expect(res1[0].name).toBe("dsh-plugin-theme-dracula")

      const res2 = filterMarketPlugins({
        plugins: mockPlugins,
        query: "记忆系统",
        category: "all",
      })
      expect(res2).toHaveLength(1)
      expect(res2[0].name).toBe("dsh-memory")
    })

    it("filters by category", () => {
      const res = filterMarketPlugins({
        plugins: mockPlugins,
        query: "",
        category: "theme",
      })
      expect(res).toHaveLength(1)
      expect(res[0].name).toBe("dsh-plugin-theme-dracula")
    })

    it("filters by onlyInstalled when set is provided", () => {
      const installedSet = new Set(["@furongjun/dsh-memory"])
      const res = filterMarketPlugins({
        plugins: mockPlugins,
        query: "",
        category: "all",
        onlyInstalled: true,
        installedPluginNames: installedSet,
      })
      expect(res).toHaveLength(1)
      expect(res[0].name).toBe("dsh-memory")
    })
  })

  describe("sortMarketPlugins", () => {
    const mockPlugins: MarketPlugin[] = [
      {
        name: "plugin-b",
        owner: "owner2",
        url: "",
        page: "",
        category: "tools",
        description: "b",
        npm: "b",
        stars: 50,
        downloads: 2000,
        install: "",
        added: "2026-08-10",
      },
      {
        name: "plugin-a",
        owner: "owner1",
        url: "",
        page: "",
        category: "tools",
        description: "a",
        npm: "a",
        stars: 500,
        downloads: 100,
        install: "",
        added: "2026-08-25",
      },
    ]

    it("sorts by stars descending", () => {
      const sorted = sortMarketPlugins(mockPlugins, "stars")
      expect(sorted[0].name).toBe("plugin-a")
      expect(sorted[1].name).toBe("plugin-b")
    })

    it("sorts by downloads descending", () => {
      const sorted = sortMarketPlugins(mockPlugins, "downloads")
      expect(sorted[0].name).toBe("plugin-b")
      expect(sorted[1].name).toBe("plugin-a")
    })

    it("sorts by newest descending", () => {
      const sorted = sortMarketPlugins(mockPlugins, "newest")
      expect(sorted[0].name).toBe("plugin-a")
      expect(sorted[1].name).toBe("plugin-b")
    })

    it("sorts by name ascending", () => {
      const sorted = sortMarketPlugins(mockPlugins, "name")
      expect(sorted[0].name).toBe("plugin-a")
      expect(sorted[1].name).toBe("plugin-b")
    })
  })
})
