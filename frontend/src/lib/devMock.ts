import { mockIPC, mockWindows } from "@tauri-apps/api/mocks"
import sampleMarket from "./market-sample.json"
import type { Capability } from "@/types/ipc"

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

  mockIPC(async (cmd, args: any) => {
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
          entries: [
            ...active.dependencies.map((d: string, i: number) => ({
              entry_id: `entry-${i}`,
              module_name: d,
              enabled: i !== 2,
              fiber_phase: i === 2 ? null : ("active" as const),
            })),
            // MCP 插件行（2026-09-18 三修）：装配状态徽标的真相源就是这些行——
            // 模块名恒为 dsh-mcp-client，`entry_id` 是 loader 树路径 `include:<行id>`。
            // 与上面 `list_mcp_servers` 的 rowId 对齐，四种装配结论各占一条：
            // active / failed / loading / (enabled 但 fiber_phase=null = 没有实例)。
            // 完全没有条目的行**不给徽标**（观测不到就不陈述）。
            ...[
              (["mcp-github", true, "active"]) as const,
              (["mcp-fetch", true, "active"]) as const,
              (["mcp-fetch-2", true, "failed"]) as const,
              (["mcp-postgres", true, "loading"]) as const,
              (["mcp-tempad-dev", true, null]) as const,
              (["mcp-legacy-crash", false, null]) as const,
              (["mcp-filesystem", true, "active"]) as const,
              (["mcp-remote-search", true, "active"]) as const,
            ].map(([rowId, enabled, phase]) => ({
              entry_id: `include:${rowId}`,
              module_name: "@deepseek-ai/dsh-mcp-client",
              enabled,
              fiber_phase: phase,
            })),
          ],
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
          node: { version: "20.18.0", origin: "engine", plannedVersion: null },
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

      // MCP 服务器（结构化配置）。`scope` 演示两种生效范围（2026-09-18）：
      // global = $DSH_HOME/cordis.patch.yml（所有 profile），profile = 仅本 profile。
      // 2026-09-18 三修：这一屏要能直接看到全部**异常形状**（同层重名 / 跨层重名 /
      // `!!js` 表达式行 / streamable-http / 已停用），否则改 UI 时只能看到最顺的那条。
      // `rowId` 是写操作定位那一行的身份；命令一律 `pnpm dlx`——引擎目录里没有 npx。
      case "list_mcp_servers":
        return [
          {
            name: "github",
            command: "pnpm",
            args: ["dlx", "-y", "@modelcontextprotocol/server-github"],
            env: { GITHUB_PERSONAL_ACCESS_TOKEN: "ghp_••••••••••••" },
            disabled: false,
            scope: "profile",
            rowId: "mcp-github",
          },
          // 同层重名：两条同名都写进了**同一个文件** ⇒ 后加载那条必然实例化失败。
          {
            name: "fetch",
            command: "pnpm",
            args: ["dlx", "-y", "mcp-server-fetch"],
            env: {},
            disabled: false,
            scope: "profile",
            rowId: "mcp-fetch",
          },
          {
            name: "fetch",
            command: "pnpm",
            args: ["dlx", "-y", "@modelcontextprotocol/fetch"],
            env: {},
            disabled: false,
            scope: "profile",
            rowId: "mcp-fetch-2",
          },
          // 表达式行：patch 原文是 `!!js process.env.APP_DB_URL`，展平后就是右边那串
          // 字面量 ⇒ 界面必须标出来（改这类行会被后端拒绝）。
          {
            name: "postgres",
            command: "pnpm",
            args: ["dlx", "-y", "@modelcontextprotocol/server-postgres"],
            env: { DATABASE_URL: "process.env.APP_DB_URL" },
            disabled: false,
            scope: "profile",
            rowId: "mcp-postgres",
            expr: true,
          },
          {
            name: "tempad-dev",
            command: "/usr/local/bin/node",
            args: ["~/.dsh/mcp-servers/tempad-dev/node_modules/@tempad-dev/mcp/dist/cli.mjs"],
            env: {},
            cwd: "~/.dsh/mcp-servers/tempad-dev",
            disabled: false,
            scope: "global",
            rowId: "mcp-tempad-dev",
          },
          // 跨层重名：profile 层与全局层各一条同名（两条都插入，全局那条失败）。
          {
            name: "filesystem",
            command: "pnpm",
            args: ["dlx", "-y", "@modelcontextprotocol/server-filesystem", "~/git"],
            env: {},
            disabled: false,
            scope: "profile",
            rowId: "mcp-filesystem",
          },
          {
            name: "filesystem",
            command: "pnpm",
            args: ["dlx", "-y", "@modelcontextprotocol/server-filesystem", "~/docs"],
            env: {},
            disabled: false,
            scope: "global",
            rowId: "mcp-filesystem",
          },
          // streamable-http：没有 command/args/cwd，只有端点与请求头。
          {
            name: "remote-search",
            command: "",
            args: [],
            env: {},
            disabled: false,
            transport: "streamable-http",
            url: "https://mcp.example.com/search/mcp",
            headers: { Authorization: "Bearer sk-••••••••••", "X-Tenant": "acme" },
            scope: "global",
            rowId: "mcp-remote-search",
          },
          {
            name: "legacy-crash",
            command: "pnpm",
            args: ["dlx", "-y", "@acme/mcp-legacy"],
            env: {},
            disabled: true,
            scope: "global",
            rowId: "mcp-legacy-crash",
          },
        ]

      // MCP 能力探测（ADR-0022）：成功给一份三清单快照；表达式行**必须**走到拒绝分支
      // （后端不会探 `!!js` 行——壳侧复现不了 dsh 的求值，探了就是假通过/假失败）。
      case "probe_mcp_server": {
        const name = args?.serverName ?? "unknown"
        if (name === "postgres") {
          throw `MCP 服务器「${name}」的配置含 \`!!js\` 表达式（从环境变量取值），壳无法复现 dsh 的求值 ⇒ 不探测。`
        }
        return {
          protocolVersion: "2025-06-18",
          serverName: name,
          tools: [
            { name: "search", detail: "按关键词检索，返回带引用的摘要" },
            { name: "fetch_url", detail: "抓取一个 URL 并转成 Markdown" },
            { name: "read_file", detail: "读取工作目录内的文本文件" },
          ],
          resources: [{ name: "help://getting-started", detail: "入门说明文档" }],
          templates: [{ name: "record", detail: "records://{id}" }],
          notes: [],
        }
      }

      // 实验能力开关（2026-09-16，ADR-0020 §7）：四态各一，供浏览器直开时看全状态
      // —— 已启用 / 未启用 / 已停用 / 需要修复都在一屏里，改文案与布局时能立刻看出差别。
      // 命令名与载荷形状必须与 `list_experimental_capabilities` 的 serde 输出逐字一致
      // （改后端形状忘改这里，dev 联调会以"看不出问题"的方式失真）。
      // 实验能力的包操作与行操作（2026-09-16）：浏览器直开时要能**走完**一次启用/关闭，
      // 否则只会撞到未实现的命令，看到的是失败分支而不是真实交互。
      case "install_plugin":
        return { ok: true, detail: `已安装 ${args?.package ?? "插件"}` }
      case "remove_plugin":
        return { ok: true, detail: `已移除 ${args?.package ?? "插件"}` }
      case "update_plugin":
        return { ok: true, detail: `已更新 ${args?.package ?? "插件"}` }
      case "apply_official_patch_row":
        return { changed: true, autoActivated: false }
      case "remove_official_patch_row":
        return true
      case "set_plugin_disabled":
        return null

      // 视图是**单语 payload**（按 lang 出品，2026-09-18 边界A）；mock 固定模拟 zh 那份。
      case "list_experimental_capabilities": {
        const V = "@0.1.6-alpha.1"
        const step = (
          ordinal: number,
          pkg: string,
          activation: "auto_bundle" | "insert_row",
          state: "off" | "live" | "disabled" | "rowless",
          targets: string[] = [],
        ) => {
          const installed = state !== "off"
          const rowPresent = state === "live" || state === "disabled"
          return {
            ordinal,
            package: pkg,
            spec: pkg + V,
            activation,
            // 与 `official_catalog::row_id_for` 同式：`dsh-dock-` 前缀 + 非 [A-Za-z0-9_-] 折成 `-`。
            // 少了前缀，删行会在 `is_shell_row_id` 处被拒（dev 直开时表现为"移除总是失败"）。
            rowId: "dsh-dock-" + pkg.replace(/[^A-Za-z0-9_-]/g, "-"),
            installed,
            rowPresent,
            disabled: state === "disabled",
            toggleTargets: state === "off" ? [] : targets,
            versionNotice: null,
            description: null,
          }
        }
        // `satisfies` 让 tsc 按契约逐字段校验这份字面量（2026-09-17 独立复核）：
        // 少写一个字段、写错一个枚举，编译期就红——比"数出现次数"的闸门强得多
        // （当年就是漏了一个 `prerequisiteMissing`，让面板把四项全判成"前置缺失"）。
        const caps = [
          {
            id: "agent-team",
            // dsh 0.1.6-alpha.2 起 Agent Teams 随安装包自带（optional bundle）→ 由 dsh 官方
            // 插件页托管。2026-09-20 维护者裁定：dock 不再展示它（面板与插件列表都按
            // 「不含自带能力」的目录走）——浏览器直开时这项**不出现在面板里**即是预期，
            // 剩下三项覆盖 已停用 / 未启用 / 需要修复 三态。`legacyCopy` 字段按契约保留
            // （后端仍在下发，本壳不再消费）。
            shippedByDsh: true,
            legacyCopy: false,
            label: "多智能体协同",
            summary: "让模型自己拉人：创建具名 teammate、互相发消息、共享任务板",
            unlocks:
              "模型多出九个 team 工具。注意：它会取代旧的委派控件 —— subagent、subagent_fork 等四个旧行会被停用，两者不能并存；移除本能力后旧控件恢复。",
            state: "on",
            activeVariant: "web",
            variants: [
              {
                id: "web",
                label: "Web 档",
                note: "含宿主层与 Web 层，浏览器侧能看到 Team 面板。",
                prerequisites: ["需持久会话存储，团队状态才落得下来"],
                state: "on",
                toggleOffSupported: false,
                subsumedBy: null,
                displaced: [],
                prerequisiteMissing: null,
                steps: [
                  step(1, "@deepseek-ai/dsh-experimental-agent-team-profile", "auto_bundle", "live"),
                  step(
                    2,
                    "@deepseek-ai/dsh-experimental-agent-team-web-profile",
                    "auto_bundle",
                    "live",
                    ["team-web-row"],
                  ),
                ],
              },
              {
                id: "headless",
                label: "自建档（无 Web 界面）",
                note: "只装宿主层：工具与任务板可用，界面不新增面板。",
                prerequisites: ["需持久会话存储，团队状态才落得下来"],
                // 真机形态：Web 档生效时本档的包必然也齐 → 报"已包含"而不是"冲突"。
                state: "on",
                toggleOffSupported: true,
                subsumedBy: "web",
                displaced: [],
                prerequisiteMissing: null,
                steps: [
                  // 宿主层已随 Web 档就位（同一份包被两档共用）。
                  step(1, "@deepseek-ai/dsh-experimental-agent-team-profile", "auto_bundle", "live"),
                ],
              },
            ],
          },
          {
            id: "browser-use",
            // dsh 0.1.6-alpha.2 起 Agent Teams 随安装包自带（optional bundle）→ 由 dsh 管；
            // 这里同时置 legacyCopy，浏览器直开时能看到"清理旧副本"那条出路。
            shippedByDsh: false,
            legacyCopy: false,
            label: "浏览器操作",
            summary: "让模型自己开浏览器：点页面、读页面结构、跑导航任务",
            unlocks: "模型多出一组浏览器工具；同一时刻只允许一个后端生效。",
            state: "disabled",
            activeVariant: "playwright",
            variants: [
              {
                id: "playwright",
                label: "Playwright",
                note: "通用浏览器自动化后端，适合脚本化的多步导航。",
                prerequisites: ["浏览器只用 Chromium 系"],
                state: "disabled",
                subsumedBy: null,
                toggleOffSupported: true,
                displaced: [],
                prerequisiteMissing: null,
                steps: [
                  step(
                    1,
                    "@deepseek-ai/dsh-browser-use",
                    "insert_row",
                    "disabled",
                    ["dsh-dock-deepseek-ai-dsh-browser-use"],
                  ),
                  step(
                    2,
                    "@deepseek-ai/dsh-experimental-browser-use-playwright-mcp",
                    "insert_row",
                    "disabled",
                    ["dsh-dock-deepseek-ai-dsh-experimental-browser-use-playwright-mcp"],
                  ),
                ],
              },
              {
                id: "chrome-devtools",
                label: "Chrome DevTools",
                note: "直连本机 Chrome，多带一层 DevTools 检查能力。",
                prerequisites: ["浏览器只用 Chromium 系", "本机需安装 Chrome"],
                state: "off",
                subsumedBy: null,
                toggleOffSupported: true,
                displaced: [],
                prerequisiteMissing: null,
                steps: [
                  step(1, "@deepseek-ai/dsh-browser-use", "insert_row", "live", [
                    "dsh-dock-deepseek-ai-dsh-browser-use",
                  ]),
                  step(
                    2,
                    "@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp",
                    "insert_row",
                    "off",
                  ),
                ],
              },
              {
                id: "stagehand",
                label: "Stagehand",
                note: "用自然语言描述操作，由指定模型翻译成动作。",
                prerequisites: [
                  "浏览器只用 Chromium 系",
                  "需在 profile 配置里显式填 model，且不支持 DeepSeek 端点或 baseURL 覆盖",
                  "会额外消耗该模型的调用额度，且这部分用量不计入 dsh 会话统计",
                ],
                state: "off",
                subsumedBy: null,
                toggleOffSupported: true,
                displaced: [],
                prerequisiteMissing: null,
                steps: [
                  step(1, "@deepseek-ai/dsh-browser-use", "insert_row", "live", [
                    "dsh-dock-deepseek-ai-dsh-browser-use",
                  ]),
                  step(
                    2,
                    "@deepseek-ai/dsh-experimental-browser-use-stagehand-native",
                    "insert_row",
                    "off",
                  ),
                ],
              },
            ],
          },
          {
            id: "computer-use",
            // dsh 0.1.6-alpha.2 起 Agent Teams 随安装包自带（optional bundle）→ 由 dsh 管；
            // 这里同时置 legacyCopy，浏览器直开时能看到"清理旧副本"那条出路。
            shippedByDsh: false,
            legacyCopy: false,
            label: "桌面控制",
            summary: "让模型操作你的桌面：鼠标、键盘、窗口",
            unlocks:
              "模型多出一组桌面控制工具。这是权限最高的实验能力：多个会话共享同一个桌面，而取消调用无法撤销已经送到桌面的输入。",
            state: "off",
            activeVariant: null,
            variants: [
              {
                id: "cua-driver-mcp",
                label: "复用已装的 cua-driver",
                note: "通过 MCP 连你本机已装好的 cua-driver，本体不随包带入。",
                prerequisites: ["需先自行安装并保持 cua-driver 可用"],
                state: "off",
                subsumedBy: null,
                toggleOffSupported: true,
                displaced: [],
                // 真机形态（2026-09-16 事故）：本机没装 cua-driver 时后端下发这句话，
                // 面板必须禁掉开关并原样展示——dev 直开要能走到这条分支。
                prerequisiteMissing:
                  "本机没有可用的 cua-driver：请先安装它，或改用「随包自带运行时」那一档",
                steps: [
                  step(1, "@deepseek-ai/dsh-computer-use", "insert_row", "off"),
                  step(
                    2,
                    "@deepseek-ai/dsh-experimental-computer-use-cua-driver-mcp",
                    "insert_row",
                    "off",
                  ),
                ],
              },
              {
                id: "cua-driver-native",
                label: "随包自带运行时",
                note: "把 cua-driver 原生运行时作为依赖一起装上，自包含。",
                prerequisites: [
                  "需授予宿主桌面权限（装包本身不会授权，也不会创建桌面会话）",
                  "原生崩溃可能终止该进程；若原生关闭失败，换用另一个后端前需重启 dsh",
                ],
                state: "off",
                subsumedBy: null,
                toggleOffSupported: true,
                displaced: [],
                prerequisiteMissing: null,
                steps: [
                  step(1, "@deepseek-ai/dsh-computer-use", "insert_row", "off"),
                  step(
                    2,
                    "@deepseek-ai/dsh-experimental-computer-use-cua-driver-native",
                    "insert_row",
                    "off",
                  ),
                ],
              },
            ],
          },
          {
            id: "auto-review",
            // dsh 0.1.6-alpha.2 起 Agent Teams 随安装包自带（optional bundle）→ 由 dsh 管；
            // 这里同时置 legacyCopy，浏览器直开时能看到"清理旧副本"那条出路。
            shippedByDsh: false,
            legacyCopy: false,
            label: "自动安全审查",
            summary: "每次工具调用前用同一模型复核一遍，拦下危险操作",
            unlocks:
              "权限选择器里多出带 EXP 上标的 Auto review 模式。代价是每个动作多一轮模型调用（更慢更贵），且模型分类可能出错。",
            state: "partial",
            activeVariant: null,
            variants: [
              {
                id: "standard",
                label: "标准",
                note: "只对 Web 档有意义，装上即生效。",
                prerequisites: ["会额外消耗 token；不提供文件沙箱与确定性豁免"],
                state: "partial",
                subsumedBy: null,
                toggleOffSupported: false,
                displaced: [],
                prerequisiteMissing: null,
                steps: [
                  step(
                    1,
                    "@deepseek-ai/dsh-experimental-auto-review",
                    "auto_bundle",
                    "rowless",
                  ),
                ],
              },
            ],
          },
        ] satisfies Capability[]
        return caps

      }

      case "get_diagnostics":
        return { system: "macOS", status: "ok" }
      default:
        return null
    }
  })
}
