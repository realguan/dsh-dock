import { mockIPC, mockWindows } from "@tauri-apps/api/mocks"
import sampleMarket from "./market-sample.json"

export function setupDevMock() {
  if (typeof window === "undefined") return
  const isTauri = typeof (window as any).__TAURI_INTERNALS__?.invoke === "function"
  if (isTauri) return

  const forcedLabel = new URLSearchParams(window.location.search).get("_label") || "profiles"
  mockWindows(forcedLabel)
  const handoffDemo = new URLSearchParams(window.location.search).get("_handoff") === "1"

  const mockProfiles = [
    {
      name: "default",
      materialized: true,
      bundles: ["@deepseek-ai/dsh-web-app", "@deepseek-ai/dsh-base"],
      dependencies: [
        "@deepseek-ai/dsh-plugin-git",
        "@deepseek-ai/dsh-plugin-terminal",
        "@deepseek-ai/dsh-plugin-browser",
        "@deepseek-ai/dsh-plugin-memory",
      ],
      web_ui: true,
    },
    {
      name: "frontend-dev",
      materialized: true,
      bundles: ["@deepseek-ai/dsh-web-app", "@deepseek-ai/dsh-base"],
      dependencies: [
        "@deepseek-ai/dsh-plugin-git",
        "@deepseek-ai/dsh-plugin-code-linter",
        "@deepseek-ai/dsh-plugin-vite-preview",
        "@deepseek-ai/dsh-plugin-tailwind-inspect",
      ],
      web_ui: true,
    },
    {
      name: "deepseek-coder",
      materialized: true,
      bundles: ["@deepseek-ai/dsh-web-app", "@deepseek-ai/dsh-base"],
      dependencies: [
        "@deepseek-ai/dsh-plugin-ast-analyzer",
        "@deepseek-ai/dsh-plugin-unit-test-gen",
        "@deepseek-ai/dsh-plugin-git-diff-reviewer",
      ],
      web_ui: true,
    },
    {
      name: "data-analysis",
      materialized: true,
      bundles: ["@deepseek-ai/dsh-web-app", "@deepseek-ai/dsh-base"],
      dependencies: [
        "@deepseek-ai/dsh-plugin-python-jupyter",
        "@deepseek-ai/dsh-plugin-echarts-render",
        "@deepseek-ai/dsh-plugin-duckdb-query",
      ],
      web_ui: true,
    },
  ]

  const mockSessions = [
    {
      id: "sess-20260910-01",
      title: "重构前端状态机架构与事件总线设计",
      projectName: "dsh-dock-frontend",
      projectDirRaw: "/Users/guan/git/dsh-dock",
      decodedProjectPath: "/Users/guan/git/dsh-dock",
      filePath: "/Users/guan/.dsh/sessions/sess-20260910-01.jsonl.zst",
      updatedAt: Date.now() - 1000 * 60 * 25,
      sizeBytes: 148200,
      isCompressed: true,
      hasBackup: true,
      status: "healthy" as const,
      eventCount: 42,
      endState: "stop",
      agentPreset: "standard",
    },
    {
      id: "sess-20260910-02",
      title: "DeepSeek-V3 核心代码单元测试套件生成",
      projectName: "deepseek-coder-engine",
      projectDirRaw: "/Users/guan/git/deepseek-coder",
      decodedProjectPath: "/Users/guan/git/deepseek-coder",
      filePath: "/Users/guan/.dsh/sessions/sess-20260910-02.jsonl.zst",
      updatedAt: Date.now() - 1000 * 60 * 120,
      sizeBytes: 324100,
      isCompressed: true,
      hasBackup: true,
      status: "healthy" as const,
      eventCount: 88,
      endState: "stop",
      agentPreset: "standard",
    },
    {
      id: "sess-20260909-03",
      title: "大数据清洗流水线性能剖析与修复",
      projectName: "data-pipeline",
      projectDirRaw: "/Users/guan/git/data-pipeline",
      decodedProjectPath: "/Users/guan/git/data-pipeline",
      filePath: "/Users/guan/.dsh/sessions/sess-20260909-03.jsonl.zst",
      updatedAt: Date.now() - 1000 * 60 * 60 * 18,
      sizeBytes: 512000,
      isCompressed: true,
      hasBackup: true,
      status: "needs_repair" as const,
      healthDetail: "检测到尾部 JSON 坏块 (行 128 截断)，支持一键原子修复",
      eventCount: 65,
      endState: "interrupted",
      agentPreset: "code",
    },
    {
      id: "sess-20260908-04",
      title: "多语言国际化 i18n 资源文件全量抽取",
      projectName: "dsh-dock",
      projectDirRaw: "/Users/guan/git/dsh-dock",
      decodedProjectPath: "/Users/guan/git/dsh-dock",
      filePath: "/Users/guan/.dsh/sessions/sess-20260908-04.jsonl.zst",
      updatedAt: Date.now() - 1000 * 60 * 60 * 48,
      sizeBytes: 98400,
      isCompressed: true,
      hasBackup: true,
      status: "healthy" as const,
      eventCount: 31,
      endState: "stop",
      agentPreset: "standard",
    },
  ]

  mockIPC((cmd, args: any) => {
    switch (cmd) {
      // 交接导轨/幕布设计走查（ADR-0014）：`?_handoff=1` 时给一份**在途**意图，
      // 于是纯浏览器预览（pnpm dev）也能看到控制中心导轨与启动屏交接态。
      // 默认（无参数）返回空意图，保持原预览行为不变。
      case "get_boot_status":
        return handoffDemo
          ? {
              steps: [
                { step: 0, state: "done", detail: "环境检测通过" },
                { step: 1, state: "done", detail: "环境已就绪（内置引擎）" },
                { step: 2, state: "done", detail: "「default」工作台已启动" },
                { step: 3, state: "running", detail: "等待 DSH 服务就绪…" },
              ],
              error: null,
              intent: {
                target: "default",
                kind: "restart",
                phase: "waiting",
                startedAt: Date.now() - 4200,
                generation: 7,
                active: true,
              },
            }
          : { steps: [], error: null, intent: null }
      case "switch_profile":
        return {
          target: args?.profile ?? "default",
          kind: "restart",
          phase: "stopping",
          startedAt: Date.now(),
          generation: 8,
          active: true,
        }
      case "focus_main_window":
        return undefined
      case "list_profiles":
        return mockProfiles
      case "get_default_profile":
        return "default"
      case "get_active_profile":
        return "default"
      case "get_profile_detail": {
        const p = mockProfiles.find((x) => x.name === args?.profile) || mockProfiles[0]
        const deps: Record<string, string> = {}
        p.dependencies.forEach((d) => {
          deps[d] = "^1.0.0"
        })
        return {
          package_name: `dsh-profile-${p.name}`,
          bundles: p.bundles,
          dependencies: deps,
          patch_yaml: "plugins:\n  - id: git\n    disabled: false\n  - id: terminal\n    disabled: false",
        }
      }

      // 工作台详情 · 外挂插件 Tab：插件清单（dependency = 第三方外挂）
      case "list_profile_plugins": {
        const p = mockProfiles.find((x) => x.name === args?.profile) || mockProfiles[0]
        const desc: Record<string, string> = {
          "@deepseek-ai/dsh-plugin-git": "Git 仓库读写、提交与分支操作",
          "@deepseek-ai/dsh-plugin-terminal": "在会话中执行本地终端命令",
          "@deepseek-ai/dsh-plugin-browser": "网页抓取与浏览器自动化能力",
          "@deepseek-ai/dsh-plugin-memory": "跨会话长期记忆存取",
          "@deepseek-ai/dsh-plugin-code-linter": "代码规范检查与自动修复建议",
          "@deepseek-ai/dsh-plugin-vite-preview": "前端产物预览与热更新联调",
          "@deepseek-ai/dsh-plugin-tailwind-inspect": "Tailwind 类名溯源与检查",
          "@deepseek-ai/dsh-plugin-ast-analyzer": "AST 结构分析与重构建议",
          "@deepseek-ai/dsh-plugin-unit-test-gen": "依据实现生成单元测试骨架",
          "@deepseek-ai/dsh-plugin-git-diff-reviewer": "提交前 diff 审阅与风险提示",
          "@deepseek-ai/dsh-plugin-python-jupyter": "Jupyter 内核与数据探索",
          "@deepseek-ai/dsh-plugin-echarts-render": "图表渲染与报表产出",
          "@deepseek-ai/dsh-plugin-duckdb-query": "本地 DuckDB 查询与分析",
        }
        return [
          ...p.bundles.map((b: string) => ({
            name: b,
            kind: "bundle" as const,
            installed_version: null,
            description: null,
          })),
          ...p.dependencies.map((d: string, i: number) => ({
            name: d,
            kind: "dependency" as const,
            installed_version: ["1.4.2", "2.0.1", "0.9.7", "1.1.0"][i % 4],
            description: desc[d] ?? null,
          })),
        ]
      }

      // 工作台详情 · 行表（toggle 开关 + 禁用意图）
      case "get_plugin_rows": {
        const p = mockProfiles.find((x) => x.name === args?.profile) || mockProfiles[0]
        return p.dependencies.map((d: string, i: number) => ({
          id: d.replace("@deepseek-ai/dsh-plugin-", ""),
          pkg_name: d,
          // 第 3 个插件演示「已禁用」态，其余启用
          shell_disabled: i === 2,
          patch_entries: i === 0 ? 1 : 0,
          contributed_ids: [],
        }))
      }

      // 工作台详情 · 运行态快照（仅活跃会话有值）
      case "get_plugin_runtime": {
        const active = mockProfiles[0]
        return {
          profile: active.name,
          entries: active.dependencies.map((d: string, i: number) => ({
            entry_id: `entry-${i}`,
            module_name: d,
            enabled: i !== 2,
            fiber_phase: i === 2 ? null : ("active" as const),
          })),
        }
      }

      // 插件更新检查：演示一只可升级插件
      case "check_plugin_updates":
        return {
          updates: [
            {
              name: mockProfiles[0].dependencies[0],
              current: "1.4.2",
              latest: "1.5.0",
            },
          ],
          checked: mockProfiles[0].dependencies.length,
          failed: 0,
        }

      case "get_shell_settings":
        return {
          defaultProfile: "default",
          defaultMode: "local",
          language: "zh-CN",
        }
      case "list_all_plugins":
        return [
          {
            name: "@deepseek-ai/dsh-plugin-git",
            sources: [{ profile: "default", spec: "@deepseek-ai/dsh-plugin-git" }],
          },
          {
            name: "@deepseek-ai/dsh-plugin-terminal",
            sources: [{ profile: "default", spec: "@deepseek-ai/dsh-plugin-terminal" }],
          },
        ]
      case "fetch_market_registry":
        return JSON.stringify(sampleMarket)
      case "list_sessions":
        return mockSessions
      case "get_client_update":
        return { phase: "idle" }
      case "get_update_status":
        return {
          dsh: { current: "0.1.1", latest: "0.1.1", newer: false, error: null, preview_latest: null },
          client: { current: "1.0.0", latest: "1.0.0", newer: false, error: null, preview_latest: null },
          node: { version: "20.18.0", origin: "managed" },
        }
      case "list_dsh_versions":
        return { current: "0.1.1", versions: [] }
      // 模型凭据（脱敏摘要 + raw YAML）：README 截图用样本。
      // 顺序与 label 必须与 Rust 侧 KNOWN_PROVIDERS 一致（credentials.rs），
      // 否则截图会展示真实产品里不存在的服务商行。
      case "get_credentials_summary":
        return [
          { provider: "deepseek", label: "DeepSeek", configured: true, maskedKey: "sk-••••••••••••••••c4f1" },
          { provider: "openai", label: "OpenAI", configured: true, maskedKey: "sk-proj-••••••••••7a3e" },
          { provider: "anthropic", label: "Anthropic (Claude)", configured: true, maskedKey: "sk-ant-••••••••••9b20" },
          { provider: "google", label: "Google Gemini", configured: false, maskedKey: "" },
          { provider: "moonshot", label: "Moonshot (Kimi)", configured: false, maskedKey: "" },
          { provider: "zhipu", label: "智谱 GLM", configured: false, maskedKey: "" },
          { provider: "groq", label: "Groq", configured: false, maskedKey: "" },
          { provider: "openrouter", label: "OpenRouter", configured: false, maskedKey: "" },
        ]
      case "get_credentials_raw":
        return [
          "# DSH 模型凭据（0600 · 原子写）",
          "deepseek:",
          '  api_key: "sk-XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXc4f1"',
          "openai:",
          '  api_key: "sk-proj-XXXXXXXXXXXXXXXXXXXXXXXX7a3e"',
          "anthropic:",
          '  api_key: "sk-ant-XXXXXXXXXXXXXXXXXXXXXX9b20"',
          "",
        ].join("\n")

      // DSH 全局引擎设置 raw YAML
      case "get_dsh_settings_raw":
        return [
          "# DSH 引擎全局设置",
          "model: deepseek-v4",
          "temperature: 0.7",
          "maxTokens: 8192",
          "telemetry: false",
          "",
        ].join("\n")

      // 运行环境健康诊断
      case "get_system_diagnostics":
        return {
          node: {
            path: "~/.dsh-dock/engines/bin/node",
            version: "v22.14.0",
            source: "managed",
            isReady: true,
          },
          pnpm: {
            path: "~/.dsh-dock/engines/bin/pnpm",
            version: "10.15.1",
            isReady: true,
          },
          dsh: {
            path: "~/.dsh-dock/engines/global/v11/bin/dsh",
            version: "0.1.5-alpha.1",
            source: "managed",
            isReady: true,
          },
          storage: {
            dshHome: "~/.dsh",
            totalBytes: 1288490188,
            profilesBytes: 734003200,
            sessionsBytes: 402653184,
            profilesCount: mockProfiles.length,
            sessionsCount: mockSessions.length,
          },
          platform: { os: "macOS 15.4 (Sequoia)", arch: "aarch64" },
        }

      // 运行日志（壳层）
      case "get_app_logs":
        return {
          source: String(args?.source ?? "shell"),
          path: "~/.dsh-dock/logs/shell.log",
          totalLines: 12,
          truncated: false,
          lines: [
            "2026-09-10T04:51:02.114Z INFO  [boot] 引擎就绪检查开始",
            "2026-09-10T04:51:02.268Z INFO  [engines] node 命中托管运行时 {version: v22.14.0}",
            "2026-09-10T04:51:02.301Z INFO  [engines] pnpm 命中托管运行时 {version: 10.15.1}",
            "2026-09-10T04:51:02.412Z INFO  [engines] dsh 版本校验通过 {version: 0.1.5-alpha.1}",
            "2026-09-10T04:51:02.560Z INFO  [profiles] 扫描工作台 {count: 4}",
            "2026-09-10T04:51:02.688Z INFO  [shell] 启动 dsh 子进程 {profile: default, mode: local}",
            "2026-09-10T04:51:03.921Z INFO  [shell] 探测 --no-open 就绪标记命中",
            "2026-09-10T04:51:04.035Z INFO  [shell] 工作台 URL 已捕获 {port: 51837}",
            "2026-09-10T04:51:04.102Z INFO  [ui] 主窗口导航完成 {label: main}",
            "2026-09-10T04:51:19.775Z INFO  [plugins] 插件清单快照刷新 {profile: default, count: 4}",
            "2026-09-10T04:52:41.208Z INFO  [sessions] 会话健康巡检 {scanned: 4, needsRepair: 1}",
            "2026-09-10T04:53:07.663Z INFO  [settings] settings.json 原子写完成",
          ],
        }

      // MCP 服务器（结构化配置）
      case "list_mcp_servers":
        return [
          {
            name: "github",
            command: "npx",
            args: ["-y", "@modelcontextprotocol/server-github"],
            env: { GITHUB_PERSONAL_ACCESS_TOKEN: "ghp_••••••••••••" },
            disabled: false,
          },
          {
            name: "filesystem",
            command: "npx",
            args: ["-y", "@modelcontextprotocol/server-filesystem", "~/git"],
            env: {},
            disabled: false,
          },
          {
            name: "postgres",
            command: "npx",
            args: ["-y", "@modelcontextprotocol/server-postgres"],
            env: { DATABASE_URL: "postgresql://localhost:5432/app" },
            disabled: true,
          },
        ]

      case "get_diagnostics":
        return { system: "macOS", status: "ok" }
      default:
        return null
    }
  })
}
