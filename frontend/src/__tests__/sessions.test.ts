// sessions.test.ts —— 会话项目聚合、路径反解与自愈筛选前端单测（4.6）。
import { describe, expect, it } from "vitest"
import type { SessionItem } from "@/types/ipc"

describe("Session & Workspace Management", () => {
  const mockSessions: SessionItem[] = [
    {
      id: "session-001",
      title: "启动页重构",
      projectName: "dsh-dock",
      projectDirRaw: "--Users-guan-git-dsh-dock--",
      decodedProjectPath: "/Users/guan/git/dsh/dock",
      filePath: "/Users/guan/.dsh/sessions/--Users-guan-git-dsh-dock--/session-001/session.jsonl",
      updatedAt: 1700000000000,
      sizeBytes: 1024 * 50, // 50 KB
      isCompressed: false,
      hasBackup: false,
      status: "healthy",
    },
    {
      id: "session-002",
      title: "会话自愈失败需修复",
      projectName: "dsh-dock",
      projectDirRaw: "--Users-guan-git-dsh-dock--",
      decodedProjectPath: "/Users/guan/git/dsh/dock",
      filePath: "/Users/guan/.dsh/sessions/--Users-guan-git-dsh-dock--/session-002/session.jsonl",
      updatedAt: 1700001000000,
      sizeBytes: 1024 * 150, // 150 KB
      isCompressed: true,
      hasBackup: true,
      status: "needs_repair",
      healthDetail: "重放重叠已检测",
    },
    {
      id: "session-003",
      title: "",
      projectName: "my-web-app",
      projectDirRaw: "--Users-guan-projects-my-web-app--",
      decodedProjectPath: "/Users/guan/projects/my-web-app",
      filePath: "/Users/guan/.dsh/sessions/--Users-guan-projects-my-web-app--/session-003/session.jsonl",
      updatedAt: 1700002000000,
      sizeBytes: 1024 * 200,
      isCompressed: false,
      hasBackup: false,
      status: "healthy",
    },
  ]

  it("groups sessions correctly by workspace project", () => {
    const map = new Map<
      string,
      { projectName: string; decodedPath: string; items: SessionItem[]; totalBytes: number }
    >()

    for (const sess of mockSessions) {
      const key = sess.projectDirRaw
      if (!map.has(key)) {
        map.set(key, {
          projectName: sess.projectName,
          decodedPath: sess.decodedProjectPath,
          items: [],
          totalBytes: 0,
        })
      }
      const g = map.get(key)!
      g.items.push(sess)
      g.totalBytes += sess.sizeBytes
    }

    const groups = Array.from(map.entries())
    expect(groups.length).toBe(2)

    const dockGroup = map.get("--Users-guan-git-dsh-dock--")!
    expect(dockGroup.items.length).toBe(2)
    expect(dockGroup.decodedPath).toBe("/Users/guan/git/dsh/dock")
    expect(dockGroup.totalBytes).toBe(1024 * 200)

    const webAppGroup = map.get("--Users-guan-projects-my-web-app--")!
    expect(webAppGroup.items.length).toBe(1)
    expect(webAppGroup.decodedPath).toBe("/Users/guan/projects/my-web-app")
  })

  it("filters sessions by session ID, project name, or decoded path query", () => {
    const query1 = "002"
    const res1 = mockSessions.filter(
      (s) =>
        s.id.toLowerCase().includes(query1) ||
        s.projectName.toLowerCase().includes(query1) ||
        s.decodedProjectPath.toLowerCase().includes(query1),
    )
    expect(res1.length).toBe(1)
    expect(res1[0].id).toBe("session-002")

    const query2 = "projects"
    const res2 = mockSessions.filter(
      (s) =>
        s.id.toLowerCase().includes(query2) ||
        s.projectName.toLowerCase().includes(query2) ||
        s.decodedProjectPath.toLowerCase().includes(query2),
    )
    expect(res2.length).toBe(1)
    expect(res2[0].projectName).toBe("my-web-app")
  })

  it("filters sessions needing self-healing repair", () => {
    const needsRepair = mockSessions.filter((s) => s.status === "needs_repair")
    expect(needsRepair.length).toBe(1)
    expect(needsRepair[0].id).toBe("session-002")
  })
})
