// firstPartyPlugins.test.ts —— 自研插件目录的纯逻辑闸门（2026-10-13）。
//
// 覆盖三件事：
//   ① 目录自身健全性：spec 全钉版本 / github spec 的 repo 段 = package 字段 / id 唯一；
//   ② 已装状态推导：dependency 命中算装、bundle 不算、版本实读；
//   ③ 更新判据：钉版归一（tag 前导 v 剥掉）后与已装版本比对。

import { describe, expect, it } from "vitest"

import { t as zhCN } from "@/content/zh-CN"
import { enUS } from "@/content/en-US"
import { pluginChineseName } from "@/lib/pluginDisplay"
import {
  FIRST_PARTY_PLUGINS,
  firstPartyRows,
  pinVersion,
  specPackageName,
} from "@/lib/firstPartyPlugins"

describe("pinVersion", () => {
  it("github spec 取 #tag 并剥掉前导 v", () => {
    expect(pinVersion("github:realguan/dsh-deepseek-chat-embed#v0.1.0")).toBe("0.1.0")
    expect(pinVersion("github:realguan/x#1.2.3")).toBe("1.2.3")
  })

  it("npm spec 取 @ 后末段（含 scope 包）", () => {
    expect(pinVersion("dsh-deepseek-chat-embed@0.1.0")).toBe("0.1.0")
    expect(pinVersion("@deepseek-ai/dsh-base@0.2.0-rc.2")).toBe("0.2.0-rc.2")
  })

  it("未钉版本返回 null（空 tag / 无 @）", () => {
    expect(pinVersion("github:realguan/dsh-deepseek-chat-embed")).toBeNull()
    expect(pinVersion("github:realguan/dsh-deepseek-chat-embed#")).toBeNull()
    expect(pinVersion("dsh-deepseek-chat-embed")).toBeNull()
  })
})

describe("specPackageName", () => {
  it("github spec 取 repo 段（忽略 #tag）", () => {
    expect(specPackageName("github:realguan/dsh-deepseek-chat-embed#v0.1.0")).toBe(
      "dsh-deepseek-chat-embed",
    )
  })

  it("非 github spec 返回 null", () => {
    expect(specPackageName("dsh-deepseek-chat-embed@0.1.0")).toBeNull()
  })
})

describe("目录闸门（FIRST_PARTY_PLUGINS 自身健全性）", () => {
  it("每条 spec 都钉了版本——不钉 = 用户装到漂移的分支 HEAD", () => {
    for (const p of FIRST_PARTY_PLUGINS) {
      expect(pinVersion(p.spec), `${p.id} 的 spec 未钉版本: ${p.spec}`).not.toBeNull()
    }
  })

  it("github spec 的 repo 段 = package 字段——否则已装匹配按 name 漏判", () => {
    for (const p of FIRST_PARTY_PLUGINS) {
      expect(specPackageName(p.spec), `${p.id} 非 github spec，一致性闸门未覆盖`).toBe(
        p.package,
      )
    }
  })

  it("id 唯一（文案键按 id 取，重名即互相覆盖）", () => {
    const ids = FIRST_PARTY_PLUGINS.map((p) => p.id)
    expect(new Set(ids).size).toBe(ids.length)
  })

  it("package 唯一（同一包装两行会出现两个装卸入口）", () => {
    const names = FIRST_PARTY_PLUGINS.map((p) => p.package)
    expect(new Set(names).size).toBe(names.length)
  })

  it("repo 是 https 外链（经 api.openExternal 打开，URL 解析只认 http(s)）", () => {
    for (const p of FIRST_PARTY_PLUGINS) {
      expect(p.repo.startsWith("https://"), `${p.id} 的 repo 非 https: ${p.repo}`).toBe(true)
    }
  })

  it("每条目录条目在双语 content 里都有描述——货架渲染按 id 取文案，漏key=空白行", () => {
    for (const p of FIRST_PARTY_PLUGINS) {
      for (const [lang, dict] of [
        ["zh-CN", zhCN],
        ["en-US", enUS],
      ] as const) {
        // 同 FirstPartyPlugins.tsx::itemCopy：AppCopy 递归映射后无索引签名，
        // 动态 id 取值需显式查表断言。
        const table = dict.profiles.firstPartyItems as Record<
          string,
          { desc: string; note: string } | undefined
        >
        const item = table[p.id]
        expect(item, `${p.id} 缺 ${lang} 文案`).toBeDefined()
        expect(item?.desc.length, `${p.id} 的 ${lang} 描述为空`).toBeGreaterThan(0)
      }
    }
  })

  it("每个包在中文名策展表里双语命中——未命中则货架与插件列表两处都缺辅名", () => {
    for (const p of FIRST_PARTY_PLUGINS) {
      expect(pluginChineseName(p.package, "zh-CN"), `${p.package} 缺中文名`).not.toBeNull()
      expect(pluginChineseName(p.package, "en-US"), `${p.package} 缺英文名`).not.toBeNull()
    }
  })
})

describe("firstPartyRows", () => {
  const dep = (name: string, version: string) => ({
    name,
    kind: "dependency",
    installed_version: version,
  })

  it("未安装：installed=false、updateAvailable=false", () => {
    const rows = firstPartyRows(FIRST_PARTY_PLUGINS, [])
    expect(rows).toHaveLength(FIRST_PARTY_PLUGINS.length)
    const row = rows[0]
    expect(row.installed).toBe(false)
    expect(row.installedVersion).toBeNull()
    expect(row.updateAvailable).toBe(false)
    expect(row.pinnedVersion).not.toBe("")
  })

  it("已装且版本 = 钉版：不提示更新", () => {
    const pinned = pinVersion(FIRST_PARTY_PLUGINS[0].spec)
    const rows = firstPartyRows(FIRST_PARTY_PLUGINS, [dep(FIRST_PARTY_PLUGINS[0].package, pinned!)])
    expect(rows[0].installed).toBe(true)
    expect(rows[0].installedVersion).toBe(pinned)
    expect(rows[0].updateAvailable).toBe(false)
  })

  it("已装但版本 ≠ 钉版：提示可对齐", () => {
    const rows = firstPartyRows(FIRST_PARTY_PLUGINS, [dep(FIRST_PARTY_PLUGINS[0].package, "0.0.9")])
    expect(rows[0].installed).toBe(true)
    expect(rows[0].updateAvailable).toBe(true)
  })

  it("bundle 条目不算已装（随 dsh 提供 ≠ 本 profile 装的）", () => {
    const rows = firstPartyRows(FIRST_PARTY_PLUGINS, [
      { name: FIRST_PARTY_PLUGINS[0].package, kind: "bundle", installed_version: null },
    ])
    expect(rows[0].installed).toBe(false)
    expect(rows[0].installedVersion).toBeNull()
  })

  it("别的包装了不影响本行判定", () => {
    const rows = firstPartyRows(FIRST_PARTY_PLUGINS, [dep("@deepseek-ai/dsh-base", "0.2.0-rc.2")])
    expect(rows[0].installed).toBe(false)
  })
})
