// ipcShapes.test.ts —— IPC 契约「形状」跨语言闸门的前端半边（2026-09-08，P4）。
//
// 历史缺口：闸门只覆盖**名字**（handler ↔ COMMANDS ↔ capabilities ↔ tauri.ts），
// **形状**无人管——字段改名/换 casing（profiles/plugins 走 snake_case，
// sessions/settings/diagnostics 走 camelCase）两侧都不会红，只会「编译绿、运行错」。
//
// 本文件断言「TS 接口的 key 集」== 共享 fixture `types/ipc-shapes.json`；
// Rust 侧 `ipc.rs::ipc_struct_shapes_match_fixture` 断言「真实 serde 序列化的
// key 集」== 同一份 fixture。任一侧漂移都红，且 fixture 是唯一事实源。
import { describe, expect, it } from "vitest"
import shapesRaw from "@/types/ipc-shapes.json?raw"
import type {
  AggregatePlugin,
  BootErrorPayload,
  AggregateSource,
  ComponentUpdate,
  CopyConfigOutcome,
  DshDiagnosticInfo,
  DshVersionEntry,
  DshVersionsResult,
  NodeDiagnosticInfo,
  PlatformDiagnosticInfo,
  PluginRowState,
  PnpmDiagnosticInfo,
  ProfileSummary,
  RepairOutcome,
  SessionItem,
  ShellSettings,
  StorageDiagnosticInfo,
  SystemDiagnosticsReport,
} from "@/types/ipc"

/** 把接口的每个键展开成必填——漏键在编译期报错，多键触发多余属性检查。 */
type AllKeys<T> = { [K in keyof Required<T>]: true }

const shapes = JSON.parse(shapesRaw) as Record<string, string[]>

function expectShape<T>(name: string, keys: AllKeys<T>) {
  expect(
    Object.keys(keys).sort(),
    `${name} 的 TS 键集与 types/ipc-shapes.json 不一致`,
  ).toEqual([...shapes[name]].sort())
}

describe("IPC 形状契约（TS 接口 ↔ 共享 fixture）", () => {
  it("BootErrorPayload（camelCase，含 tagged failure）", () => {
    expectShape<BootErrorPayload>("BootErrorPayload", {
      failure: true,
      title: true,
      detail: true,
      suggestion: true,
      actions: true,
      log: true,
    })
  })

  it("ShellSettings（camelCase）", () => {
    expectShape<ShellSettings>("ShellSettings", {
      defaultMode: true,
      defaultProfile: true,
      locale: true,
      autoRestart: true,
      showFloatingSwitcher: true,
      switcherShortcut: true,
      dismissedUpdate: true,
    })
  })

  it("ProfileSummary（snake_case：web_ui）", () => {
    expectShape<ProfileSummary>("ProfileSummary", {
      name: true,
      materialized: true,
      bundles: true,
      dependencies: true,
      web_ui: true,
    })
  })

  it("SessionItem（camelCase，20 键）", () => {
    expectShape<SessionItem>("SessionItem", {
      id: true,
      title: true,
      projectName: true,
      projectDirRaw: true,
      decodedProjectPath: true,
      filePath: true,
      updatedAt: true,
      sizeBytes: true,
      isCompressed: true,
      hasBackup: true,
      status: true,
      healthDetail: true,
      active: true,
      archived: true,
      createdAt: true,
      eventCount: true,
      endState: true,
      subagent: true,
      agentPreset: true,
      validator: true,
    })
  })

  it("插件域（snake_case：pkg_name / shell_disabled / skipped_existing）", () => {
    expectShape<PluginRowState>("PluginRowState", {
      id: true,
      pkg_name: true,
      shell_disabled: true,
      patch_entries: true,
      contributed_ids: true,
    })
    expectShape<AggregatePlugin>("AggregatePlugin", {
      name: true,
      description: true,
      sources: true,
    })
    expectShape<AggregateSource>("AggregateSource", {
      profile: true,
      version: true,
      spec: true,
    })
    expectShape<CopyConfigOutcome>("CopyConfigOutcome", {
      copied: true,
      skipped_existing: true,
      detail: true,
    })
  })

  it("RepairOutcome（camelCase：sessionId）", () => {
    expectShape<RepairOutcome>("RepairOutcome", {
      sessionId: true,
      success: true,
      message: true,
    })
  })

  it("更新域（2026-09-09 版本选择器：ComponentUpdate / DshVersionEntry / DshVersionsResult）", () => {
    expectShape<ComponentUpdate>("ComponentUpdate", {
      current: true,
      latest: true,
      newer: true,
      error: true,
      preview_latest: true,
    })
    expectShape<DshVersionEntry>("DshVersionEntry", {
      version: true,
      channel: true,
      relation: true,
      published_at: true,
    })
    expectShape<DshVersionsResult>("DshVersionsResult", {
      current: true,
      versions: true,
    })
  })

  it("诊断域（camelCase：isReady / dshHome / totalBytes）", () => {
    expectShape<NodeDiagnosticInfo>("NodeDiagnosticInfo", {
      path: true,
      version: true,
      source: true,
      isReady: true,
    })
    expectShape<PnpmDiagnosticInfo>("PnpmDiagnosticInfo", {
      path: true,
      version: true,
      isReady: true,
    })
    expectShape<DshDiagnosticInfo>("DshDiagnosticInfo", {
      path: true,
      version: true,
      source: true,
      isReady: true,
    })
    expectShape<StorageDiagnosticInfo>("StorageDiagnosticInfo", {
      dshHome: true,
      totalBytes: true,
      profilesBytes: true,
      sessionsBytes: true,
      profilesCount: true,
      sessionsCount: true,
    })
    expectShape<PlatformDiagnosticInfo>("PlatformDiagnosticInfo", {
      os: true,
      arch: true,
    })
    expectShape<SystemDiagnosticsReport>("SystemDiagnosticsReport", {
      node: true,
      pnpm: true,
      dsh: true,
      storage: true,
      platform: true,
    })
  })
})
