// profileManifestChip.test.ts —— Profile 详情页头「清单名」判据 + 页头合成层闸门
// （2026-10-14，真机截图：页头同时出现两份 `dsh-profile-web`）。
//
// 现场（v1.4.0 真机截图）：页头一行里并排两份灰字 `dsh-profile-web`，第二份落在
// 「过渡徽标出现时」清单名所在的位置（右侧 ~148px 处）。当时 DOM 里只有一枚清单名
//  span（源码只有一处渲染），故第二份是合成层复用旧栅格留下的残影：这一行里唯一
// 会横向位移的文字就是清单名（徽标出现把它推右、切档又随 profile 名宽度变化）。
//
// 两条修法各自钉一条闸门：
//  ① **不再渲染常规档位的清单名**——dsh 建档与壳的改名/复制都把 package.json 的
//     name 写成 `dsh-profile-<目录名>`（profiles.rs::rewrite_package_name），此时
//     它与左栏的 profile 名是同一句话（2026-09-21 维护者已就这处说过"同一句话说
//     两遍"）。只有真的不一致（手改过 package.json / 别处搬过目录）才展示。
//     于是这一行常规情况下**没有会位移的文字**，残影无从而来。
//  ② **页头不做 `backdrop-blur`**——背景恒为不透明面板底色，模糊本来看不见，却
//     白白换来一个独立合成层（残影的载体）。
// 纯逻辑 + 源码文本测试：不渲染 DOM（AGENTS §4.3 末段）。
import { describe, expect, it } from "vitest"

import paneSrc from "@/components/profiles/ProfileDetailPane.tsx?raw"
import libSrc from "@/lib/profiles.ts?raw"
import { manifestNameChip } from "@/lib/profiles"

/** 页头切片（`ProfileDetailPane` 的 `<header>`…`</header>`）：闸门只该钉这一块。 */
function headerRegion(): string {
  const start = paneSrc.indexOf('<header className="border-b border-line bg-panel px-5 py-3">')
  expect(start, "页头容器（不透明底色 + 无 backdrop）存在").toBeGreaterThanOrEqual(0)
  const end = paneSrc.indexOf("</header>", start)
  expect(end, "页头切片可闭合").toBeGreaterThan(start)
  return paneSrc.slice(start, end)
}

describe("manifestNameChip：清单名只在真的带信息时展示", () => {
  it("dsh 约定名（`dsh-profile-<profile 名>`）= 左栏已有信息 → 不展示", () => {
    expect(manifestNameChip("web", "dsh-profile-web")).toBeNull()
    expect(manifestNameChip("headless", "dsh-profile-headless")).toBeNull()
    expect(manifestNameChip("desktop", "dsh-profile-desktop")).toBeNull()
  })

  it("与 profile 名不一致 = 别处拿不到 → 原样展示", () => {
    expect(manifestNameChip("web", "dsh-profile-old")).toBe("dsh-profile-old")
    expect(manifestNameChip("web", "@deepseek-ai/dsh-desktop-runtime")).toBe(
      "@deepseek-ai/dsh-desktop-runtime",
    )
    // 只差一个后缀也不算约定名（不做前缀匹配——`dsh-profile-web2` 是真信息）
    expect(manifestNameChip("web", "dsh-profile-web2")).toBe("dsh-profile-web2")
    // 反例：约定名但大小写不同 ⇒ 后端一致化写的是全小写，差异说明被手改过
    expect(manifestNameChip("Web", "dsh-profile-web")).toBe("dsh-profile-web")
  })

  it("读不到清单名（null/空串/空白）→ 不展示空壳", () => {
    expect(manifestNameChip("web", null)).toBeNull()
    expect(manifestNameChip("web", "")).toBeNull()
    expect(manifestNameChip("web", "   ")).toBeNull()
    expect(manifestNameChip(null, null)).toBeNull()
  })

  it("profile 名缺失时不做约定名推断，如实展示清单名", () => {
    expect(manifestNameChip(null, "dsh-profile-web")).toBe("dsh-profile-web")
    expect(manifestNameChip("   ", "dsh-profile-web")).toBe("dsh-profile-web")
  })

  it("两侧空白先裁掉再比（IPC 取值可能带空白）", () => {
    expect(manifestNameChip(" web ", " dsh-profile-web ")).toBeNull()
  })

  it("判据单一来源：组件不自己写 `dsh-profile-` 前缀判断", () => {
    expect(paneSrc, "组件走纯函数判据").toContain("manifestNameChip(")
    expect(paneSrc, "组件内不再内联拼清单名约定").not.toContain("`dsh-profile-${")
    expect(libSrc, "约定名判据留在 lib/profiles.ts").toContain("dsh-profile-${profile}")
  })
})

describe("页头合成层闸门（残影载体）", () => {
  it("页头不再带 backdrop 合成层，底色不透明", () => {
    const header = headerRegion()
    expect(header, "页头无 backdrop-blur").not.toContain("backdrop-blur")
    expect(header, "页头底色不透明（不再 bg-panel/90）").not.toContain("bg-panel/90")
  })

  it("清单名渲染受判据门控（不是无条件渲染）", () => {
    const header = headerRegion()
    expect(header, "清单名走判据开关").toContain("manifestChip !== null &&")
    expect(header, "清单名内容取自判据结果").toContain("{manifestChip}")
    // 反转证：清单名数据源仍在（pluginListMergeGate 也钉这条），只是多了一层门控
    expect(paneSrc, "清单名数据源未丢").toContain("detail.package_name")
  })
})
