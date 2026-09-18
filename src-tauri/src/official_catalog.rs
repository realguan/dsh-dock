//! official_catalog.rs —— 实验能力的**策展与状态判定**（2026-09-15 立，ADR-0020；
//! 2026-09-16 第二次修订：目录行 → **能力开关**，见 ADR-0020 §7）。
//!
//! ## 职责边界
//!
//! 本模块只做纯判定：**有哪些能力、每个能力有哪些变体、现在是何状态、开/关/移除
//! 各需要哪些行级目标**。不触网、不起子进程、不写文件。安装仍归
//! `commands/plugin.rs` 的 `dsh plugin add` 转发链，patch 写入仍归 `plugins.rs::PatchFile`。
//!
//! ## 为什么是「能力」而不是「包」（ADR-0020 §7.2）
//!
//! v1 把策展集摊平为 8 条并列条目，于是：同一能力的三个互斥后端成了三张等价卡片
//! （用户看不出"现在是哪一个"），而"关掉"这件事在目录里**根本不存在**（只有安装）。
//! 第二次修订把呈现单位提为**能力**，互斥后端降级为**能力内的变体**——互斥因此在
//! 同一张卡内被表达为"当前后端"，而不是让用户去理解"三选一"。
//!
//! ## 状态是「包 × 行 × disabled」的函数
//!
//! `installed`（profile `dependencies` 里有这个包）**不等于**能力已生效：还要看
//! 挂载行是否存在（`insert_row` 类由壳写、`auto_bundle` 类由包自带段落贡献）、
//! 以及该行是否被 `disabled` 停用（ADR-0009 例外 #3）。三者缺一即"没生效"。
//! 状态判定放在后端、只此一处（§2.8 禁双源）——前端不得凭 `installed` 猜。
//!
//! ## 上游事实锚点（2026-09-15 实查，dsh 0.1.6-alpha.1）
//!
//! - `dsh plugin add` 会**自行协调** `dsh.profile.bundles`：解析到声明 `dsh.bundle` 的包
//!   就追加进层栈，未声明的只装依赖并 stderr 告警
//!   （`apps/cli/src/plugin.ts:59-91`，检测条件 `:36-45`，告警 `:70-75`）；
//! - 行身份键是 **`id`**，缺 `id` 会被自动生成、该行此后**永不可被 patch 命中**
//!   （`vendor/loader/src/config/tree.ts:51-59`）；
//! - 真实 schema：`PatchOptions` / `EntryOptions`
//!   （`vendor/include/src/index.ts:129-141`；`vendor/loader/src/config/entry.ts:9-22`），
//!   **无 `before`/`after` 排序键**——顺序即数组位置；
//! - 裸包名按 `latest` dist-tag 解析，而 Agent Teams 三包的 `latest` **落后**于
//!   `alpha`（实测：裸装得 `0.1.5-alpha.2`，运行时为 `0.1.6-alpha.1`）。

use serde::{Deserialize, Serialize};

use crate::plugins::PluginRowState;

/// 激活方式：由目标包**是否声明 `dsh.bundle`** 决定（ADR-0020 §4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Activation {
    /// 声明了 `dsh.bundle`：`dsh plugin add` 已把它追加进 `dsh.profile.bundles` 完成激活。
    /// 壳**禁止**再写 `- insert:` —— 会**重复挂载**（行身份是 `id`，同名不同 id 即两份实例）。
    AutoBundle,
    /// 未声明 `dsh.bundle`：CLI 只装依赖并告警，**不会**激活。
    /// 壳**必须**写一条带稳定 `id` 的 `- insert:` 挂载行。
    InsertRow,
}

/// 按「是否声明 `dsh.bundle`」判定激活方式。这是本模块的**唯一分类依据**，
/// 不得由包名、来源或任何启发式替代（ADR-0020 §2.8 禁双源）。
pub fn activation_for(declares_bundle: bool) -> Activation {
    if declares_bundle {
        Activation::AutoBundle
    } else {
        Activation::InsertRow
    }
}

/// 由包名派生**稳定、可复算**的 patch 行 `id`。
///
/// 必要性（ADR-0020 §2.6）：缺 `id` 时 loader 自动生成，之后任何 `- id:` 都命不中该行
/// ——那将产生"装了却永远改不了配置"的不可维护状态。故 `id` 必须可复算，
/// 且对同一包名**恒等**（重复安装幂等）。
///
/// 规则：去掉 npm scope 的 `@` 与 `/`，非 `[A-Za-z0-9_-]` 一律折成 `-`，
/// 前置 `dsh-dock-` 命名空间前缀避免与 bundle 自身行 id 撞车。
///
/// 该前缀同时是**所有权标记**：只有以此前缀开头的行才允许删除
/// （`plugins::is_shell_row_id`）——否则一次误删会动到 bundle 自带行或用户手写行。
pub fn row_id_for(package: &str) -> String {
    let mut out = String::from("dsh-dock-");
    for ch in package.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        } else {
            out.push('-');
        }
    }
    out
}

/// 把包名与期望版本钉成一个显式 spec（ADR-0020 §2.4）。
///
/// **为什么必须钉**：`dsh plugin add <裸包名>` 走 `pnpm add`，按 `latest` dist-tag 解析；
/// 而 `@deepseek-ai/dsh-experimental-agent-team*` 三包的 `latest` 停在 `0.1.5-alpha.2`，
/// 与 `0.1.6-alpha.1` 运行时错配（本机实测复现，同台账行 7 的 dsh-base 事故）。
pub fn pinned_spec(package: &str, version: &str) -> String {
    format!("{package}@{version}")
}

// ---------- 挂载行的必需载荷与宿主前置（2026-09-16 真机事故后立，ADR-0020 §7.5） ----------

/// 挂载行 `config:` 的字段值（只覆盖策展集实际需要的三种形态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigValue {
    Str(&'static str),
    Bool(bool),
    StrList(&'static [&'static str]),
}

/// 浏览器类 MCP provider 的**启动型**配置：上游 `BrowserMcpConfig` 把 `mode` 定为
/// **必填**（`launch` / `attach`），缺 `config` 时插件的 `apply` 直接抛 TypeError。
const BROWSER_LAUNCH_CONFIG: &[(&str, ConfigValue)] = &[
    ("mode", ConfigValue::Str("launch")),
    ("headless", ConfigValue::Bool(true)),
];

/// cua-driver MCP provider 的显式命令配置（上游 README 的最小配置原文）。
const CUA_DRIVER_MCP_CONFIG: &[(&str, ConfigValue)] = &[
    ("command", ConfigValue::Str("cua-driver")),
    ("args", ConfigValue::StrList(&["mcp"])),
];

/// 包 → 作为挂载行时**必须**写进 `config:` 的字段。
///
/// **为什么这是一张必须存在的表**（2026-09-16 真机事故）：上游对 MCP provider 的
/// `Config` 是**必填**（浏览器族 `mode: launch|attach`）或**语义必填**（cua-driver 的
/// `command`）。壳原先一律只写 `{id, name}`，于是
/// `browser-use-chrome-devtools-mcp` 在 `apply` 里 `config` 为 `undefined` →
/// `TypeError: Cannot read properties of undefined (reading 'mode')` → **整棵 plugin tree
/// 加载失败** → dsh 永不就绪（真机复现：`profiles/web/cordis.patch.yml` 两行坏行，
/// 启动 34s 后退出码 1）。上游 README 的最小配置即本表来源，随 dsh 升级须复核。
const ROW_CONFIGS: &[(&str, &[(&str, ConfigValue)])] = &[
    (
        "@deepseek-ai/dsh-experimental-browser-use-playwright-mcp",
        BROWSER_LAUNCH_CONFIG,
    ),
    (
        "@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp",
        BROWSER_LAUNCH_CONFIG,
    ),
    (
        "@deepseek-ai/dsh-experimental-computer-use-cua-driver-mcp",
        CUA_DRIVER_MCP_CONFIG,
    ),
];

/// 包 → 它要求宿主**先具备**的可执行文件。
///
/// `cua-driver-mcp` 只负责连一个**已安装**的 Cua Driver（上游原文：安装与桌面权限归
/// Cua Driver 自己）。缺了它，插件在 `apply` 里 `spawn cua-driver` → `ENOENT` →
/// 同样是整棵 plugin tree 失败。故它必须在前置未满足时**拒绝安装**，而不是装完
/// 把 profile 弄成起不来。自包含的那档（`cua-driver-native`）无此要求。
const PACKAGE_REQUIRES_COMMAND: &[(&str, &str)] = &[(
    "@deepseek-ai/dsh-experimental-computer-use-cua-driver-mcp",
    "cua-driver",
)];

/// 该包作为挂载行时的必需 `config:` 载荷；空切片 = 只需 `{id, name}`。
pub fn required_row_config(package: &str) -> &'static [(&'static str, ConfigValue)] {
    ROW_CONFIGS
        .iter()
        .find(|(name, _)| *name == package)
        .map(|(_, fields)| *fields)
        .unwrap_or(&[])
}

/// 该包要求的宿主可执行文件（`None` = 无前置）。
pub fn required_command(package: &str) -> Option<&'static str> {
    PACKAGE_REQUIRES_COMMAND
        .iter()
        .find(|(name, _)| *name == package)
        .map(|(_, cmd)| *cmd)
}

// ---------- 策展集：能力 → 变体 ----------

/// 用户可见文案的语言（2026-09-18 task-边界A：英文界面下实验能力页不再漏中文）。
///
/// 视图**按请求语言出品**（单语 payload），而不是把两套文案都塞进 IPC——
/// 前端拿到的 `label`/`note` 等就是该展示的那一份，不存在"选错边"的可能。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyLang {
    Zh,
    En,
}

impl CopyLang {
    /// 宽松解析 IPC 传来的 locale 标签：`en`/`en-US` → En，其余（含 `None`）→ Zh。
    /// 未知语言退回中文是**有意的**：目录的原文是中文，英文是译文。
    pub fn from_tag(tag: Option<&str>) -> Self {
        match tag {
            Some(t) if t.to_ascii_lowercase().starts_with("en") => Self::En,
            _ => Self::Zh,
        }
    }
}

/// 能力下的一个**可选后端**（同能力变体互斥：同一时刻只应有一个生效）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    /// 稳定 id（前端用它做单选值，不得用展示名——改文案即丢状态）。
    pub id: &'static str,
    /// 展示名（中文原文）。
    pub label_zh: &'static str,
    pub label_en: &'static str,
    /// 一句话：这个后端适合谁 / 代价是什么。**人类语言，不得含 Markdown 反引号**
    /// （v1 直接把反引号渲染成了字面量，ADR-0020 §7.1 D6）。
    pub note_zh: &'static str,
    pub note_en: &'static str,
    /// 需要用户自备或额外配置的东西（空 = 无）。展示与确认框共用。
    /// 中英列表**必须等长**（有闸门测试钉住）。
    pub prerequisites_zh: &'static [&'static str],
    pub prerequisites_en: &'static [&'static str],
    /// 有序包清单：**顺序即语义**（Agent Teams 先宿主层再 Web 层，装反即激活失败）。
    pub packages: &'static [&'static str],
}

/// 一项实验能力：用户可开关的**呈现单位**。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    pub id: &'static str,
    pub label_zh: &'static str,
    pub label_en: &'static str,
    /// 一句话价值（卡片副标题）。
    pub summary_zh: &'static str,
    pub summary_en: &'static str,
    /// 启用后**用户能观察到什么**（详情与确认框）。
    pub unlocks_zh: &'static str,
    pub unlocks_en: &'static str,
    pub variants: &'static [Variant],
}

// ─────────────────────────────────────────────────────────────────────────────
// 「dsh 自带的能力，dock 不代管」（2026-09-17 立，ADR-0020 §7.2 第三次修订）
//
// dsh 0.1.6-alpha.2 起把官方实验层作为 **optional bundle** 随安装包下发（上游依据：
// `packages/boot/app-boot/src/profile.ts` 的 `OPTIONAL_BUNDLES` ＋ 设计笔记
// `.agents/notes/implemented/process/2026-09-15-shipped-optional-bundles.md`）：随包下载、
// 默认关、在 **dsh 自己的插件页**里开关、**永不卸载**。该笔记同时**明确否决**了"由某个
// 面板按名字从 registry 安装官方 bundle"这条替代路径——那正是本模块原来在做的事。
//
// 判据是**这次安装实测出来的事实**，不是写在目录里的旗标：某个能力的**每个包**都能在
// `<engines>/dsh-runtime/node_modules/<包>` 找到 ⇒ 这个安装自带它（见 `PackageFacts::shipped`）。
// 为什么不写旗标：同一台机器上 dev 档引擎 0.1.6-alpha.1 **不带**、正式档 0.1.6-alpha.2 **带**
// ——写死的旗标必然在其中一边说谎。实测判据还会**自动跟上** dsh 后续把更多实验能力内置的节奏：
// 升级复核点 = 安装包 `@deepseek-ai/dsh` 的 `dependencies` 里出现新的
// `@deepseek-ai/dsh-experimental-*`（＝ `OPTIONAL_BUNDLES` 增项），届时无需改代码。
// ─────────────────────────────────────────────────────────────────────────────

/// 策展集（**dsh-dock 策展，非"官方首批"**——上游无该概念，为全量发布 16 包，
/// ADR-0020 §1.1）。变体顺序 = 推荐顺序（UI 默认选中首个）。
///
/// ⚠ 包顺序勿随意重排；每项 `summary_zh` / `unlocks_zh` / `prerequisites_zh` 是
/// **用户可见承诺**，随 dsh 升级须与上游 README 逐条复核（ADR-0020 §7.3）。
pub const CAPABILITIES: &[Capability] = &[
    Capability {
        id: "agent-team",
        label_zh: "多智能体协同",
        label_en: "Multi-agent collaboration",
        summary_zh: "让模型自己拉人：创建具名 teammate、互相发消息、共享任务板",
        summary_en: "Let the model recruit on its own: named teammates, direct messages, a shared task board",
        unlocks_zh: "模型多出九个 team 工具（创建 / 收发消息 / 协调 teammate、读写共享任务板），\
                     消息与任务挺得过崩溃与重载。Web 档还会在主界面出现 Team 面板与任务板。\
                     注意：它会接管旧的委派控件 —— subagent、subagent_fork 等四个旧行会被停用，\
                     两者不能并存；移除本能力后旧控件恢复。",
        unlocks_en: "The model gains nine team tools (creating teammates, sending and receiving \
                     messages, coordinating them, reading and writing the shared task board); \
                     messages and tasks survive crashes and reloads. The Web tier also adds a Team \
                     panel and the task board to the main UI. Note: it takes over the older \
                     delegation controls — the four legacy rows (subagent, subagent_fork, etc.) \
                     are disabled and the two cannot coexist; removing this capability restores \
                     the old controls.",
        variants: &[
            Variant {
                id: "web",
                label_zh: "Web 档",
                label_en: "Web tier",
                note_zh: "含宿主层与 Web 层，浏览器侧能看到 Team 面板。",
                note_en: "Includes the host layer and the Web layer; the Team panel shows up in the browser.",
                prerequisites_zh: &["需持久会话存储，团队状态才落得下来"],
                prerequisites_en: &["Persistent session storage is required for team state to persist"],
                packages: &[
                    "@deepseek-ai/dsh-experimental-agent-team-profile",
                    "@deepseek-ai/dsh-experimental-agent-team-web-profile",
                ],
            },
            Variant {
                id: "headless",
                label_zh: "自建档（无 Web 界面）",
                label_en: "Profile-only (no Web UI)",
                note_zh: "只装宿主层：工具与任务板可用，界面不新增面板。",
                note_en: "Host layer only: tools and the task board work, no new panels in the UI.",
                prerequisites_zh: &["需持久会话存储，团队状态才落得下来"],
                prerequisites_en: &["Persistent session storage is required for team state to persist"],
                packages: &["@deepseek-ai/dsh-experimental-agent-team-profile"],
            },
        ],
    },
    Capability {
        id: "browser-use",
        label_zh: "浏览器操作",
        label_en: "Browser control",
        summary_zh: "让模型自己开浏览器：点页面、读页面结构、跑导航任务",
        summary_en: "Let the model drive a browser: click pages, read their structure, run navigation tasks",
        unlocks_zh: "模型多出一组浏览器工具（打开页面、点击、填表、截图、读取页面结构）。\
                     同一时刻只允许一个后端生效，换后端要走替换。\
                     浏览器状态按 Session 重建 —— 登录态与浏览器 profile 不会从会话历史恢复；\
                     取消调用也无法撤销已经送达页面的操作。",
        unlocks_en: "The model gains a set of browser tools (open pages, click, fill forms, \
                     screenshot, read page structure). Only one backend may be active at a \
                     time — switching backends is a replacement. Browser state is rebuilt per \
                     session: logins and browser profiles are not restored from session history, \
                     and cancelling a call cannot undo actions already delivered to the page.",
        variants: &[
            Variant {
                id: "playwright",
                label_zh: "Playwright",
                label_en: "Playwright",
                note_zh: "通用浏览器自动化后端，适合脚本化的多步导航。",
                note_en: "General-purpose browser automation backend, a good fit for scripted multi-step navigation.",
                prerequisites_zh: &["浏览器只用 Chromium 系"],
                prerequisites_en: &["Browser limited to Chromium-based ones"],
                packages: &[
                    "@deepseek-ai/dsh-browser-use",
                    "@deepseek-ai/dsh-experimental-browser-use-playwright-mcp",
                ],
            },
            Variant {
                id: "chrome-devtools",
                label_zh: "Chrome DevTools",
                label_en: "Chrome DevTools",
                note_zh: "直连本机 Chrome，多带一层 DevTools 检查能力。",
                note_en: "Connects directly to a local Chrome and adds a DevTools inspection layer.",
                prerequisites_zh: &["浏览器只用 Chromium 系", "本机需安装 Chrome"],
                prerequisites_en: &[
                    "Browser limited to Chromium-based ones",
                    "Chrome must be installed on this machine",
                ],
                packages: &[
                    "@deepseek-ai/dsh-browser-use",
                    "@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp",
                ],
            },
            Variant {
                id: "stagehand",
                label_zh: "Stagehand",
                label_en: "Stagehand",
                note_zh: "用自然语言描述操作，由指定模型翻译成动作。",
                note_en: "Actions are described in natural language and translated by a designated model.",
                prerequisites_zh: &[
                    "浏览器只用 Chromium 系",
                    "需在 profile 配置里显式填 model，且不支持 DeepSeek 端点或 baseURL 覆盖",
                    "会额外消耗该模型的调用额度，且这部分用量不计入 dsh 会话统计",
                ],
                prerequisites_en: &[
                    "Browser limited to Chromium-based ones",
                    "model must be filled in explicitly in the profile config; DeepSeek endpoints and baseURL overrides are not supported",
                    "Consumes that model's call quota additionally, and this usage is not counted in dsh session statistics",
                ],
                packages: &[
                    "@deepseek-ai/dsh-browser-use",
                    "@deepseek-ai/dsh-experimental-browser-use-stagehand-native",
                ],
            },
        ],
    },
    Capability {
        id: "computer-use",
        label_zh: "桌面控制",
        label_en: "Desktop control",
        summary_zh: "让模型操作你的桌面：鼠标、键盘、窗口",
        summary_en: "Let the model drive your desktop: mouse, keyboard, windows",
        unlocks_zh: "模型多出一组桌面控制工具，可以直接操作真实的鼠标键盘与窗口。\
                     这是权限最高的实验能力：多个会话共享同一个桌面，操作之间不会被串行化，\
                     而取消调用无法撤销已经送到桌面的输入。请只在受控环境启用。",
        unlocks_en: "The model gains a set of desktop-control tools that drive the real mouse, \
                     keyboard and windows. This is the highest-privilege experimental capability: \
                     multiple sessions share the same desktop, operations are not serialized \
                     between them, and cancelling a call cannot undo input already sent to the \
                     desktop. Enable it only in controlled environments.",
        variants: &[
            Variant {
                id: "cua-driver-mcp",
                label_zh: "复用已装的 cua-driver",
                label_en: "Reuse an installed cua-driver",
                note_zh: "通过 MCP 连你本机已装好的 cua-driver，本体不随包带入。",
                note_en: "Connects over MCP to the cua-driver already installed on this machine; brings no runtime of its own.",
                prerequisites_zh: &["需先自行安装并保持 cua-driver 可用"],
                prerequisites_en: &["You must install cua-driver yourself and keep it available"],
                packages: &[
                    "@deepseek-ai/dsh-computer-use",
                    "@deepseek-ai/dsh-experimental-computer-use-cua-driver-mcp",
                ],
            },
            Variant {
                id: "cua-driver-native",
                label_zh: "随包自带运行时",
                label_en: "Bundled runtime",
                note_zh: "把 cua-driver 原生运行时作为依赖一起装上，自包含。",
                note_en: "Installs the cua-driver native runtime as a dependency; self-contained.",
                prerequisites_zh: &[
                    "需授予宿主桌面权限（装包本身不会授权，也不会创建桌面会话）",
                    "原生崩溃可能终止该进程；若原生关闭失败，换用另一个后端前需重启 dsh",
                ],
                prerequisites_en: &[
                    "Requires granting desktop permissions to the host (installing the package neither grants them nor creates a desktop session)",
                    "A native crash may kill the process; if native shutdown fails, restart dsh before switching to the other backend",
                ],
                packages: &[
                    "@deepseek-ai/dsh-computer-use",
                    "@deepseek-ai/dsh-experimental-computer-use-cua-driver-native",
                ],
            },
        ],
    },
    Capability {
        id: "auto-review",
        label_zh: "自动安全审查",
        label_en: "Automatic safety review",
        summary_zh: "每次工具调用前用同一模型复核一遍，拦下危险操作",
        summary_en: "Every tool call is re-checked beforehand by the same model, blocking dangerous actions",
        unlocks_zh: "权限选择器里多出带 EXP 上标的 Auto review 模式：每个原生或 PTC 工具调用\
                     在执行前先由当前模型评估一次，可拦下危险动作。代价是每个动作多一轮模型\
                     调用（不缓存、不重试、更慢更贵），且模型分类可能出错 —— 既可能误放行，\
                     也可能误拒。卸载时正在使用它的会话会被迁移回 Full access。",
        unlocks_en: "The permission picker gains an Auto review mode marked with an EXP superscript: \
                     each native or PTC tool call is first evaluated by the current model, which \
                     can block dangerous actions. The cost is one extra model call per action \
                     (uncached, no retries — slower and pricier), and the model's classification \
                     can be wrong in both directions — false approvals as well as false blocks. \
                     Sessions using it at uninstall time are migrated back to Full access.",
        variants: &[Variant {
            id: "standard",
            label_zh: "标准",
            label_en: "Standard",
            note_zh: "只对 Web 档有意义，装上即生效。",
            note_en: "Only meaningful for the Web tier; effective once installed.",
            prerequisites_zh: &["会额外消耗 token；不提供文件沙箱与确定性豁免"],
            prerequisites_en: &["Consumes extra tokens; provides no file sandbox and no deterministic exemptions"],
            packages: &["@deepseek-ai/dsh-experimental-auto-review"],
        }],
    },
];

/// 一步安装动作。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallStep {
    /// 第几步（从 1 起，供 UI 展示进度与失败续装）。
    pub ordinal: usize,
    /// 包名。
    pub package: String,
    /// 该步的 patch 行 `id`（`activation_for` 为 `InsertRow` 时才会被写入）。
    pub row_id: String,
}

/// 把变体展开为**有序、串行**的安装步骤（ADR-0020 §2.10）。
///
/// 顺序不是装饰：`agent-team-web-profile` 的类型声明要求
/// `dsh-base` → `dsh-web-app` → `dsh-agent-team-profile` → 本包 **must remain in that order**，
/// 且其 `cordis.patch.yml` 首行注明 "Apply **after** … the host-side
/// `dsh-agent-team-profile` so the browser mounts only when the Team service is present"。
/// 装反了浏览器侧会在 Team 服务就位前挂载而失败。
///
/// 同时：`dsh plugin add` **一次只接受一个包**，故必须逐步下发；调用方须**串行**执行。
pub fn install_plan(variant: &Variant) -> Vec<InstallStep> {
    variant
        .packages
        .iter()
        .enumerate()
        .map(|(i, package)| InstallStep {
            ordinal: i + 1,
            package: (*package).to_string(),
            row_id: row_id_for(package),
        })
        .collect()
}

// ---------- 状态判定 ----------

/// 变体状态：由「包 × 行 × disabled」三者共同决定（ADR-0020 §7.2-4）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VariantState {
    /// 一个包都没装、也没有挂载行。
    Off,
    /// 包齐 + 行齐 + 无一被停用 = **正在生效**。
    On,
    /// 包齐 + 行齐，但行被停用 = 已就位但**当前关着**（秒级可开）。
    Disabled,
    /// 介于两者之间：装了一半、或包在而行缺（需要修复）。
    Partial,
}

/// 能力状态 = 其变体状态的聚合（同一能力多个变体同时生效 = 冲突）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityState {
    Off,
    On,
    Disabled,
    Partial,
    /// 同能力出现 ≥2 个变体同时生效——上游会激活失败，必须让用户看见。
    Conflict,
}

/// 一步的事实视图（可序列化给前端）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepView {
    pub ordinal: usize,
    pub package: String,
    /// 钉版本后的 spec（交给 `dsh plugin add`）。
    pub spec: String,
    pub activation: Activation,
    /// 挂载行 id（`InsertRow` 时才真正落盘）。
    pub row_id: String,
    /// 该包是否已在 profile 依赖里。
    pub installed: bool,
    /// 该包的挂载行是否已**在组合树中**（`AutoBundle` 类由包自带段落贡献）。
    pub row_present: bool,
    /// 该步的行是否被停用。
    pub disabled: bool,
    /// 停用/启用该步要写的行 id（可能多行：bundle 贡献多行时全部一起切）。
    /// 空 = 该步没有可切换的行，停用只能靠移除。
    pub toggle_targets: Vec<String>,
    /// 版本错配提示（`latest` 落后于运行时等）；`None` = 无需打扰用户。
    pub version_notice: Option<String>,
    /// **该包自己的** `description`（2026-09-17 维护者裁定「描述以官方为主」）。
    /// 装在 `<profile>/node_modules/<包>/package.json` 里才读得到，故未装时为 `None`
    /// ——**绝不用策展文案冒充官方描述**，前端对 `None` 如实显示"装好后显示官方简介"。
    pub description: Option<String>,
}

/// 变体的事实视图。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariantView {
    pub id: String,
    /// 以下展示字段**已按请求语言出品**（`resolve_capabilities` 的 `lang`），
    /// 故不带语言后缀——前端拿到的就是要展示的那一份。
    pub label: String,
    pub note: String,
    pub prerequisites: Vec<String>,
    pub steps: Vec<StepView>,
    pub state: VariantState,
    /// 本变体的包是**另一个已就位变体**的真子集 → 本档已被那一档包含。
    ///
    /// 为什么需要它（2026-09-16 真机暴露）：Agent Teams 的两档是**子集关系**而非互斥——
    /// 自建档 = `[agent-team-profile]`，Web 档 = `[同一个包, agent-team-web-profile]`。
    /// 装 Web 档时"自建档"的包必然也齐，若只看"是否装齐"就会把这份**完全正常的配置**
    /// 报成「后端冲突」。浏览器/桌面两族才是真互斥（各有独占 provider 包）。
    /// 有了它：冲突判定只数"没被包含"的档；被包含的档在 UI 上显式为「已包含」且不提供
    /// 独立开关（关掉它会把超集档的基础层一起拆掉）。
    pub subsumed_by: Option<String>,
    /// **纯行级停用是否等价于"关掉"**（`false` → 关闭必须走移除）。
    ///
    /// 判据：该变体的**每一步都是 `InsertRow`**（行完全由壳拥有，且包本身不带层 patch）。
    ///
    /// 为什么 bundle 步骤不能靠行级 disabled 关掉：`dsh.bundle.patch` 层是**带副作用的
    /// 补丁文档**。实测 `agent-team-profile/cordis.patch.yml` 除插入 2 条新行外，**还停用
    /// 4 条旧的 subagent 控件行**；此时把壳插入的行 `disabled: true` 只会关掉新工具，
    /// 那 4 条旧行仍被层停用着 → 用户手里**新旧都没有**。层副作用无法用行级开关回滚，
    /// 只能经 `dsh plugin remove`（同时清 `dsh.profile.bundles` 有序层列表）才干净。
    pub toggle_off_supported: bool,
    /// 同能力**其它**变体已装、而本变体不含的后端包；非空 = 启用本变体需先替换掉它们
    /// （同族并存会激活失败）。
    pub displaced: Vec<String>,
    /// 本变体要求的宿主可执行文件**缺失**（`None` = 前置齐备）——非空时前端必须
    /// **禁用开关**并原样展示这句话。
    ///
    /// 为什么是一个硬门而不是提示（2026-09-16 真机事故）：缺 `cua-driver` 时装上
    /// 该 provider，dsh 会在插件树加载阶段 `spawn cua-driver` → `ENOENT` →
    /// **整棵 plugin tree 失败、dsh 永不就绪**。也就是说"能装上"的代价是"profile 起不来"，
    /// 那就不该让用户装上——提示语不够，得挡住。
    pub prerequisite_missing: Option<String>,
}

/// 能力的事实视图。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityView {
    pub id: String,
    pub label: String,
    pub summary: String,
    pub unlocks: String,
    pub variants: Vec<VariantView>,
    pub state: CapabilityState,
    /// 当前生效（或已就位但停用）的变体 id；`None` = 未启用。
    pub active_variant: Option<String>,
    /// **dsh 安装包自带这项能力** → 前端不渲染开关/安装/移除，只做说明与指路。
    pub shipped_by_dsh: bool,
    /// 该能力**还有 dock 能清掉的东西**留在本 Profile 里：profile 在册的包、我们写过的
    /// `InsertRow` 挂载行、或我们写下的停用桩。
    ///
    /// 与 `shipped_by_dsh` 同时为真 = 历史遗留：dock 早期按 profile 装过一份，它会遮蔽
    /// dsh 自带的那一份。此时**唯一**允许的动作是"清理旧副本"。
    ///
    /// 判据刻意与 `planRemove` 的能力面对齐：bundle 类包贡献的行（`AutoBundle`）我们既没写
    /// 也不能删，算进来只会得到一个点了没反应的假按钮。
    pub legacy_copy: bool,
}

/// `apply_official_patch_row` 的处理结果（**写行前当场重判**的分类结论）。
///
/// 为什么要有 `auto_activated` 这一档：目录是安装前拉的，那时包还没进 `node_modules`，
/// 任何包都会被判成"未声明 `dsh.bundle`"。真正的分类只能在写之前、装之后当场读出来
/// （ADR-0020 §7.4）——声明了层的包由 CLI 激活，壳**不写行**，并把这件事如实回报，
/// 而不是静默地"什么都没做"。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowWriteOutcome {
    /// 是否真的改动了 patch 文件（幂等重写 = `false`）。
    pub changed: bool,
    /// 该包声明了 `dsh.bundle`：已由 `dsh plugin add` 激活，壳**未写行**（也**不应**写）。
    pub auto_activated: bool,
}

/// 状态判定的**入参快照**（由调用方采集，本模块保持纯函数）。
#[derive(Debug, Clone, Default)]
pub struct PackageFacts {
    /// profile `dependencies` 的包名。
    pub installed: Vec<String>,
    /// 已装并**声明 `dsh.bundle`** 的包名（激活分类的唯一依据）。
    pub declared_bundles: Vec<String>,
    /// 策展集要求、但**宿主 PATH 中找不到**的可执行文件名（由调用方探测后填入）。
    ///
    /// 判据单源在 [`required_command`]（包 → 前置命令）；本字段只是"探测结果"，
    /// 让 `resolve_capabilities` 保持纯函数（不碰文件系统）。
    pub missing_commands: Vec<String>,
    /// 已装包的官方 `description`（包名 → 简介）。同样由调用方读好后填入，
    /// 让本函数保持纯（不碰文件系统）。缺包/包没写 description = 不在表里。
    pub descriptions: std::collections::BTreeMap<String, String>,
    /// **这个 dsh 安装自带**的包名（调用方探测 `<engines>/dsh-runtime/node_modules/` 后填入）。
    ///
    /// 非空且覆盖某能力的**全部**包 ⇒ 该能力归 dsh 管，dock 不代管（见上方模块注释）。
    /// 探测失败/目录不存在 = 空表 = 一切照旧由 dock 策展（保守方向：宁可多管，也不谎称自带）。
    pub shipped: Vec<String>,
}

/// **这个安装自带**策展清单里的哪些包（纯函数，只做"目录里有没有"的判断）。
///
/// 判据 = `<dsh-runtime>/node_modules/<包>/package.json` 存在。dsh 自带的 optional bundle
/// 是安装包的运行时依赖（`apps/cli` 的 `dependencies`），因而必然落在这一层 node_modules 里；
/// 而**用户自己装进 profile** 的包在 `<profile>/node_modules`，不会被算成"自带"。
///
/// 保守方向：目录不存在 / 读不到 → 返回空表 ⇒ 一切照旧由 dock 策展（**宁可多管，
/// 也不谎称"dsh 已内置"**——那会让用户彻底没有打开它的入口）。
pub fn installation_shipped(runtime_dir: &std::path::Path, packages: &[String]) -> Vec<String> {
    let root = runtime_dir.join("node_modules");
    packages
        .iter()
        .filter(|pkg| root.join(pkg.as_str()).join("package.json").is_file())
        .cloned()
        .collect()
}

/// 判断某能力的某变体现在处于什么状态，以及开/关/移除各需要哪些行级目标。
///
/// `rows` 来自 `plugins::plugin_rows_blocking`（一次 `--dump-config`），是"行是否存在 /
/// 是否被停用"的**唯一**来源；`runtime_version` 用于钉版本（`None` → 诚实降级为裸包名）。
/// `lang` 决定**用户可见文案**（含运行时提示）出品哪一种语言。
pub fn resolve_capabilities(
    facts: &PackageFacts,
    rows: &[PluginRowState],
    runtime_version: Option<&str>,
    lang: CopyLang,
) -> Vec<CapabilityView> {
    // 闭包无法对入参生命周期做泛型（两个 `&str` 借用要统一到一个返回寿命），
    // 故用嵌套 fn；`lang` 显式传参。
    fn pick<'a>(lang: CopyLang, zh: &'a str, en: &'a str) -> &'a str {
        if lang == CopyLang::En {
            en
        } else {
            zh
        }
    }
    let installed: std::collections::HashSet<&str> =
        facts.installed.iter().map(String::as_str).collect();
    let declared: std::collections::HashSet<&str> =
        facts.declared_bundles.iter().map(String::as_str).collect();
    let shipped: std::collections::HashSet<&str> =
        facts.shipped.iter().map(String::as_str).collect();

    CAPABILITIES
        .iter()
        .map(|capability| {
            let all_packages: Vec<&str> = capability
                .variants
                .iter()
                .flat_map(|v| v.packages.iter().copied())
                .collect();
            let mut variants: Vec<VariantView> = capability
                .variants
                .iter()
                .map(|variant| {
                    let steps: Vec<StepView> = install_plan(variant)
                        .into_iter()
                        .map(|step| {
                            let declares_bundle = declared.contains(step.package.as_str());
                            let activation = activation_for(declares_bundle);
                            // 版本未知时**不拼 `pkg@`**（那是个坏 spec）：退回裸包名并**明示风险**
                            // ——裸包名按 `latest` 解析，正是 Agent Teams 三包错配的成因。
                            //
                            // 关于"registry `latest` 与期望版本不一致"的提示（ADR-0020 §2.4 曾设想）：
                            // **不做**。`latest` 只能联网取，而本命令禁网（唯一网络面 = `updates.rs`），
                            // 一旦留一个恒为空的入参，那条提示就是"看着有、其实永远不会触发"的死路径
                            // （2026-09-16 独立评审核出，见 ADR-0020 §7.2-8）。§2.4 的实质要求——
                            // "确认框显示将要安装的**确切版本**"——由确认框直接列出 spec 满足。
                            let (spec, notice) = match runtime_version {
                                Some(v) => (pinned_spec(&step.package, v), None),
                                None => (
                                    step.package.clone(),
                                    Some(
                                        pick(lang,
                                            "运行时版本未检出，本步未钉版本——裸包名会按 latest 解析，\
                                             可能与运行时错配（如需严格匹配请先让引擎就绪）",
                                            "Runtime version not detected; this step is not pinned — the \
                                             bare package name resolves to latest and may mismatch the \
                                             runtime (get the engine ready first for strict matching)",
                                        )
                                        .to_string(),
                                    ),
                                ),
                            };
                            let row = row_state_for(rows, &step.package, &step.row_id, activation);
                            // 官方简介先取（`step.package` 下一行被 move 进 StepView）
                            let description = facts.descriptions.get(&step.package).cloned();
                            StepView {
                                ordinal: step.ordinal,
                                package: step.package,
                                spec,
                                activation,
                                row_id: step.row_id,
                                installed: installed.contains(row.package.as_str()),
                                row_present: row.present,
                                disabled: row.disabled,
                                toggle_targets: row.targets,
                                version_notice: notice,
                                description,
                            }
                        })
                        .collect();

                    let all_installed = steps.iter().all(|s| s.installed);
                    let all_ready = steps.iter().all(|s| s.row_present);
                    let any_disabled = steps.iter().any(|s| s.disabled);
                    let state = if all_installed && all_ready {
                        if any_disabled {
                            VariantState::Disabled
                        } else {
                            VariantState::On
                        }
                    } else if steps.iter().any(|s| s.installed || s.row_present) {
                        VariantState::Partial
                    } else {
                        VariantState::Off
                    };
                    // 纯行级停用等价于"关掉" ⟺ 每一步都是 InsertRow（见字段文档）。
                    // 附加 `all installed`：**没装就不知道**该包是否声明 `dsh.bundle`
                    // （分类由包自身的 package.json 判定），无知不得被报成"可纯开关"。
                    // 在 On / Disabled 态下每步必然已装，故该附加条件不改变真值，只挡住误报。
                    let toggle_off_supported = steps
                        .iter()
                        .all(|s| s.activation == Activation::InsertRow && s.installed);
                    // 同能力**其它**变体已装、且本变体不含的包 = 需要替换掉的旧后端。
                    let displaced: Vec<String> = all_packages
                        .iter()
                        .filter(|p| !variant.packages.contains(*p))
                        .filter(|p| installed.contains(**p))
                        .map(|p| (*p).to_string())
                        .collect();

                    // 宿主前置：本变体任一包要求某个可执行文件而它不在 PATH 里 → 硬门。
                    // 全变体一起判（如 cua-driver-mcp 与 native 档都要 `@deepseek-ai/dsh-computer-use`，
                    // 但只有前者要求外部可执行文件——单源是 required_command 的包级表）。
                    let prerequisite_missing = variant.packages.iter().find_map(|package| {
                        let command = required_command(package)?;
                        facts
                            .missing_commands
                            .iter()
                            .any(|missing| missing == command)
                            .then(|| {
                                pick(lang,
                                    &format!(
                                        "本机 PATH 中找不到「{command}」——装上会让 dsh 在加载插件时\
                                         直接失败、工作台起不来。请先装好它（或改用自包含的那一档）"
                                    ),
                                    &format!(
                                        "\"{command}\" was not found in PATH on this machine — \
                                         installing this backend would make dsh fail while loading \
                                         plugins, and the workbench would not start. Install it \
                                         first (or use the self-contained tier)"
                                    ),
                                )
                                .to_string()
                            })
                    });

                    VariantView {
                        id: variant.id.to_string(),
                        label: pick(lang, variant.label_zh, variant.label_en).to_string(),
                        note: pick(lang, variant.note_zh, variant.note_en).to_string(),
                        prerequisites: (if lang == CopyLang::En {
                            variant.prerequisites_en
                        } else {
                            variant.prerequisites_zh
                        })
                        .iter()
                        .map(|s| (*s).to_string())
                        .collect(),
                        steps,
                        state,
                        toggle_off_supported,
                        displaced,
                        subsumed_by: None,
                        prerequisite_missing,
                    }
                })
                .collect();

            // 子集关系：**已就位**（On/Disabled）的档里，谁的包是另一个的**真子集**。
            // 真子集 = 全含 + 少至少一个包（"两档完全同包"不算包含，那是重复定义）。
            for i in 0..variants.len() {
                if !matches!(variants[i].state, VariantState::On | VariantState::Disabled) {
                    continue;
                }
                let mine: Vec<&str> = variants[i]
                    .steps
                    .iter()
                    .map(|s| s.package.as_str())
                    .collect();
                let subsumed = variants.iter().enumerate().find(|(j, other)| {
                    *j != i
                        && matches!(other.state, VariantState::On | VariantState::Disabled)
                        && mine
                            .iter()
                            .all(|p| other.steps.iter().any(|s| s.package == *p))
                        && other.steps.len() > mine.len()
                });
                if let Some((_, other)) = subsumed {
                    variants[i].subsumed_by = Some(other.id.clone());
                }
            }

            // 冲突/生效只数**没被包含**的档：被包含的档本来就该跟着超集档一起就位。
            let on: Vec<&VariantView> = variants
                .iter()
                .filter(|v| v.state == VariantState::On && v.subsumed_by.is_none())
                .collect();
            let (state, active_variant) = if on.len() >= 2 {
                (CapabilityState::Conflict, None)
            } else if let Some(only) = on.first() {
                (CapabilityState::On, Some(only.id.clone()))
            } else if let Some(off_but_ready) = variants
                .iter()
                .find(|v| v.state == VariantState::Disabled && v.subsumed_by.is_none())
            {
                (CapabilityState::Disabled, Some(off_but_ready.id.clone()))
            } else if variants.iter().any(|v| v.state == VariantState::Partial) {
                (CapabilityState::Partial, None)
            } else {
                (CapabilityState::Off, None)
            };

            // **这个安装自带它** ⇔ 该能力的每个包都能在安装的 `node_modules` 里找到。
            // 全部包都在才算（少一个就是要装，不能让"自带"把缺的那步吞掉）。
            let shipped_by_dsh =
                !all_packages.is_empty() && all_packages.iter().all(|p| shipped.contains(*p));

            // 「遗留副本」= 本 Profile 里**真能清掉**的东西：profile 在册的包、我们写过的挂载行
            //（`InsertRow` 且行在）、或我们写下的停用桩。与 `shipped_by_dsh` 同时为真 = 它遮蔽了
            // dsh 自带的那一份，唯一允许的动作是清理。
            //
            // **判据必须与 `planRemove` 的能力面对齐**（2026-09-17 自查）：bundle 类包的行由
            // 包自身的 patch 贡献（`AutoBundle`），我们既没写也不能删它们——若把它们算进来，
            // 「清理旧副本」就会变成一个**点了什么都不做**的假按钮。
            let legacy_copy = variants.iter().any(|v| {
                v.steps.iter().any(|s| {
                    s.installed
                        || s.disabled
                        || (s.activation == Activation::InsertRow && s.row_present)
                })
            });

            CapabilityView {
                id: capability.id.to_string(),
                label: pick(lang, capability.label_zh, capability.label_en).to_string(),
                summary: pick(lang, capability.summary_zh, capability.summary_en).to_string(),
                unlocks: pick(lang, capability.unlocks_zh, capability.unlocks_en).to_string(),
                variants,
                state,
                active_variant,
                shipped_by_dsh,
                legacy_copy,
            }
        })
        .collect()
}

/// 一步的行事实：`present` = 行已在组合树中；`disabled` = 已停用；`targets` = 切换目标。
struct RowFacts {
    package: String,
    present: bool,
    disabled: bool,
    targets: Vec<String>,
}

/// 从 dump-config 行表里读出该包的行事实（纯函数）。
///
/// 两种激活方式的判据**不同**，不可混用：
/// - `InsertRow`：行由**壳**写入，必须按**精确 `id` + 包名**同时命中才算在
///   （防"行在、但落在别的包上"的半对状态）；
/// - `AutoBundle`：行由**包自身的 patch**贡献，在行表里表现为以该包名为段落的合成条目
///   （`plugins::build_row_states`），其 `contributed_ids` 即全部可切换目标。
///   无贡献行 = 该包不产生配置行 → 无行可切（停用只能靠移除），但仍视为"已激活"。
fn row_state_for(
    rows: &[PluginRowState],
    package: &str,
    row_id: &str,
    activation: Activation,
) -> RowFacts {
    match activation {
        Activation::InsertRow => {
            let hit = rows
                .iter()
                .find(|r| r.id == row_id && r.pkg_name == package);
            RowFacts {
                package: package.to_string(),
                present: hit.is_some(),
                disabled: hit.map(|r| r.shell_disabled).unwrap_or(false),
                targets: if hit.is_some() {
                    vec![row_id.to_string()]
                } else {
                    Vec::new()
                },
            }
        }
        Activation::AutoBundle => {
            let hit = rows
                .iter()
                .find(|r| r.pkg_name == package && !r.contributed_ids.is_empty());
            match hit {
                Some(r) => RowFacts {
                    package: package.to_string(),
                    present: true,
                    disabled: r.shell_disabled,
                    targets: r.contributed_ids.clone(),
                },
                None => RowFacts {
                    package: package.to_string(),
                    present: true,
                    disabled: false,
                    targets: Vec::new(),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(name: &str) -> String {
        name.to_string()
    }

    /// 只有**声明** `dsh.bundle` 的包已装时才允许出现，故测试里要同时给两份清单。
    /// 宿主前置默认齐备（`missing_commands` 空）；要测门否决时用 [`facts_missing`]。
    fn facts(installed: &[&str], bundles: &[&str]) -> PackageFacts {
        PackageFacts {
            installed: installed.iter().map(|s| pkg(s)).collect(),
            declared_bundles: bundles.iter().map(|s| pkg(s)).collect(),
            missing_commands: Vec::new(),
            descriptions: Default::default(),
            shipped: Vec::new(),
        }
    }

    /// 2026-09-17（维护者裁定「描述以官方为主」）：`description` **只**来自调用方
    /// 读到的已装包事实——没装/包没写 description 时必须是 `None`（前端据此如实显示
    /// "装好后显示官方简介"），**不得**用策展文案兜底冒充官方。
    #[test]
    fn step_description_comes_only_from_installed_package_facts() {
        // 取策展目录里真实存在的第一个包作样本（不硬编码包名，目录变了测试仍成立）
        let sample = CAPABILITIES
            .iter()
            .flat_map(|c| c.variants.iter())
            .flat_map(|v| v.packages.iter())
            .next()
            .expect("策展目录非空")
            .to_string();
        let facts = PackageFacts {
            descriptions: [(sample.clone(), "该包自己的简介".to_string())]
                .into_iter()
                .collect(),
            ..facts(&[sample.as_str()], &[])
        };
        let view = resolve_capabilities(&facts, &[], Some("0.1.6-alpha.1"), CopyLang::Zh);
        let steps: Vec<_> = view
            .iter()
            .flat_map(|c| c.variants.iter())
            .flat_map(|v| v.steps.iter())
            .collect();
        let hit = steps
            .iter()
            .find(|s| s.package == sample)
            .expect("样本包必须在策展目录里");
        assert_eq!(hit.description.as_deref(), Some("该包自己的简介"));
        // 反例：没给描述的包（目录里其余包）不得凭空有描述
        assert!(
            steps
                .iter()
                .filter(|s| s.package != sample)
                .all(|s| s.description.is_none()),
            "未提供官方描述时必须是 None，不得编造"
        );
    }

    /// 同 [`facts`]，但把给定可执行文件标成"宿主 PATH 里没有"。
    fn facts_missing(installed: &[&str], bundles: &[&str], missing: &[&str]) -> PackageFacts {
        PackageFacts {
            missing_commands: missing.iter().map(|s| pkg(s)).collect(),
            ..facts(installed, bundles)
        }
    }

    fn row(id: &str, name: &str, disabled: bool) -> PluginRowState {
        PluginRowState {
            id: id.to_string(),
            pkg_name: name.to_string(),
            shell_disabled: disabled,
            patch_entries: 0,
            contributed_ids: Vec::new(),
        }
    }

    /// bundle 贡献行的合成条目形态（`plugins::build_row_states` 的产物）。
    fn bundle_row(name: &str, ids: &[&str], disabled: bool) -> PluginRowState {
        PluginRowState {
            id: ids[0].to_string(),
            pkg_name: name.to_string(),
            shell_disabled: disabled,
            patch_entries: 0,
            contributed_ids: ids.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn find<'a>(caps: &'a [CapabilityView], id: &str) -> &'a CapabilityView {
        caps.iter()
            .find(|c| c.id == id)
            .expect("能力必须在策展集里")
    }

    fn variant<'a>(cap: &'a CapabilityView, id: &str) -> &'a VariantView {
        cap.variants
            .iter()
            .find(|v| v.id == id)
            .expect("变体必须在能力里")
    }

    /// 静态策展项（非视图）：供 `install_plan` 这类吃 `Variant` 的纯函数使用。
    fn static_variant<'a>(cap_id: &str, variant_id: &str) -> &'a Variant {
        CAPABILITIES
            .iter()
            .find(|c| c.id == cap_id)
            .expect("能力必须在策展集里")
            .variants
            .iter()
            .find(|v| v.id == variant_id)
            .expect("变体必须在能力里")
    }

    /// B1/B2 的核心反例护栏：分类**只能**由 `dsh.bundle` 声明决定，
    /// 且两种取值必须给出**相反**的激活方式（防将来有人"统一"成一种）。
    #[test]
    fn activation_branches_on_bundle_declaration_only() {
        assert_eq!(activation_for(true), Activation::AutoBundle);
        assert_eq!(activation_for(false), Activation::InsertRow);
        assert_ne!(activation_for(true), activation_for(false));
    }

    /// `id` 必须稳定可复算：同包名恒等（重复安装幂等），不同包名不同，
    /// scope 的 `@` 与 `/` 不得留在 id 里（loader 的 id 是行身份键）。
    #[test]
    fn row_id_is_stable_and_namespaced() {
        let a = row_id_for("@deepseek-ai/dsh-experimental-auto-review");
        assert_eq!(a, row_id_for("@deepseek-ai/dsh-experimental-auto-review"));
        assert!(a.starts_with("dsh-dock-"), "须带壳命名空间前缀：{a}");
        assert!(
            !a.contains('@') && !a.contains('/'),
            "id 不得含 scoped 字符：{a}"
        );
        assert_ne!(a, row_id_for("@deepseek-ai/dsh-experimental-agent-team"));
    }

    /// spec 必须带版本，且**只有检出运行时版本时**才拼版本（检出不到时走裸包名 + 显式告知，
    /// 见 `resolve_without_runtime_version_degrades_honestly`）。
    #[test]
    fn pinned_spec_carries_version() {
        let spec = pinned_spec(
            "@deepseek-ai/dsh-experimental-agent-team-profile",
            "0.1.6-alpha.1",
        );
        assert!(spec.ends_with("@0.1.6-alpha.1"), "{spec}");
        // 检出到运行时版本时，本步**不得**再挂"未钉版本"一类的 notice（那是降级路径专用）。
        let f = facts(&[], &[]);
        let caps = resolve_capabilities(&f, &[], Some("0.1.6-alpha.1"), CopyLang::Zh);
        assert!(find(&caps, "auto-review").variants[0].steps[0]
            .version_notice
            .is_none());
    }

    /// 有序组合：步骤序号从 1 连续递增，顺序与 `packages` 声明一致
    /// （Agent Teams 的 Web 档装反即失败，顺序是语义不是装饰）。
    #[test]
    fn install_plan_preserves_declared_order_and_ordinals() {
        let plan = install_plan(static_variant("agent-team", "web"));
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].ordinal, 1);
        assert_eq!(plan[1].ordinal, 2);
        assert!(plan[0].package.contains("agent-team-profile"));
        assert!(plan[1].package.contains("agent-team-web-profile"));
        // 每一步都带自己的稳定 id（缺 id 的行永不可再 patch）。
        assert_ne!(plan[0].row_id, plan[1].row_id);
    }

    /// 策展集自身的一致性：能力 id 唯一、变体 id 唯一、包名非空且变体内不重复；
    /// **同能力变体必须真互斥**（各自有对方不含的包，否则"切换"无意义）。
    #[test]
    fn catalog_is_well_formed() {
        let mut cap_ids = std::collections::HashSet::new();
        for cap in CAPABILITIES {
            assert!(cap_ids.insert(cap.id), "能力 id 重复：{}", cap.id);
            assert!(!cap.variants.is_empty(), "{} 无变体", cap.id);
            let mut var_ids = std::collections::HashSet::new();
            for v in cap.variants {
                assert!(var_ids.insert(v.id), "{} 变体 id 重复：{}", cap.id, v.id);
                assert!(!v.packages.is_empty(), "{}::{} 无包", cap.id, v.id);
                let mut seen = std::collections::HashSet::new();
                for p in v.packages {
                    assert!(seen.insert(*p), "{}::{} 包名重复：{p}", cap.id, v.id);
                }
            }
        }
        // 多后端的两个能力：两两变体之间必须各有独占包（否则"同一时刻只一个生效"
        // 这条互斥语义无法表达成替换动作）。**只适用于真互斥族**——Agent Teams 的两档
        // 是子集关系（自建档 ⊂ Web 档），刻意不互斥，见 `subset_variant_is_subsumed_not_conflicting`。
        for cap_id in ["browser-use", "computer-use"] {
            let cap = CAPABILITIES.iter().find(|c| c.id == cap_id).unwrap();
            assert!(cap.variants.len() >= 2, "{cap_id} 应有多后端");
            for a in cap.variants {
                for b in cap.variants {
                    if a.id == b.id {
                        continue;
                    }
                    assert!(
                        a.packages.iter().any(|p| !b.packages.contains(p)),
                        "{cap_id} 的 {} 与 {} 无独占包，互斥无法表达",
                        a.id,
                        b.id
                    );
                }
            }
        }
    }

    /// **dsh 自带的能力，dock 不代管**（2026-09-17 立）。
    ///
    /// 真机事实：dsh 0.1.6-alpha.2 起把 Agent Teams 两个 bundle 作为 optional bundle 随安装包
    /// 下发（`<dsh-runtime>/node_modules/@deepseek-ai/` 实测在册），由 dsh 自己的插件页开关，
    /// 且 dsh 视其为 `not-removable`。于是：全部包都由安装自带 ⇒ dock 不代管（无开关、无安装、
    /// 无移除）；**残留副本**（profile 自己还持有）则只给"清理"这一条出路。
    ///
    /// 判据刻意取**安装实测**而不是写死旗标——同一台机器上 dev 档引擎（0.1.6-alpha.1）不带、
    /// 正式档（0.1.6-alpha.2）带，旗标必然在其中一边说谎；实测判据还会自动跟上 dsh 后续
    /// 把更多实验能力内置的节奏。
    #[test]
    fn dsh_shipped_capability_is_not_managed_by_dock() {
        let agent_team = CAPABILITIES.iter().find(|c| c.id == "agent-team").unwrap();
        let packages: Vec<String> = agent_team
            .variants
            .iter()
            .flat_map(|v| v.packages.iter().map(|p| (*p).to_string()))
            .collect();

        // ① 安装自带（正式档 0.1.6-alpha.2 的形态）→ shipped，且**没有**任何"安装"该做的事。
        let mut facts = PackageFacts {
            shipped: packages.clone(),
            ..PackageFacts::default()
        };
        let caps = resolve_capabilities(&facts, &[], Some("0.1.6-alpha.2"), CopyLang::Zh);
        let view = caps.iter().find(|c| c.id == "agent-team").unwrap();
        assert!(view.shipped_by_dsh, "安装自带该能力的全部包 → 归 dsh 管");
        assert!(!view.legacy_copy, "profile 没有这些包 → 没有遗留副本可清");

        // ② 老引擎（0.1.6-alpha.1 的形态：不带）→ 仍由 dock 策展，否则用户没有任何入口打开它。
        facts.shipped.clear();
        let caps = resolve_capabilities(&facts, &[], Some("0.1.6-alpha.1"), CopyLang::Zh);
        let view = caps.iter().find(|c| c.id == "agent-team").unwrap();
        assert!(
            !view.shipped_by_dsh,
            "安装不带它时**不得**谎称自带（那会把唯一入口也关掉）"
        );

        // ③ 只自带到一半（少一个包）不算自带：缺的那步仍要装。
        let partial: Vec<String> = packages.iter().take(1).cloned().collect();
        let facts = PackageFacts {
            shipped: partial,
            ..PackageFacts::default()
        };
        let caps = resolve_capabilities(&facts, &[], Some("0.1.6-alpha.2"), CopyLang::Zh);
        assert!(
            !caps
                .iter()
                .find(|c| c.id == "agent-team")
                .unwrap()
                .shipped_by_dsh
        );

        // ④ 遗留副本：profile 里还留着包 → 给一条"清理"出路（且必须**真能清掉**东西）。
        let facts = PackageFacts {
            installed: packages.clone(),
            shipped: packages.clone(),
            ..PackageFacts::default()
        };
        let caps = resolve_capabilities(&facts, &[], Some("0.1.6-alpha.2"), CopyLang::Zh);
        assert!(
            caps.iter()
                .find(|c| c.id == "agent-team")
                .unwrap()
                .legacy_copy
        );
    }

    /// 安装探测：只看**安装目录**里有没有这些包（profile 里装的不算"自带"）。
    #[test]
    fn installation_probe_reads_only_the_installation() {
        let root = std::env::temp_dir().join(format!("dsh-dock-shipped-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let nm = root.join("node_modules");
        for pkg in [
            "@deepseek-ai/dsh-experimental-agent-team-profile",
            "@deepseek-ai/dsh-base",
        ] {
            let dir = nm.join(pkg);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("package.json"), "{}").unwrap();
        }
        // 只建目录、不写 package.json 的包**不算**在册（半个包不叫自带）。
        std::fs::create_dir_all(nm.join("@deepseek-ai/dsh-experimental-auto-review")).unwrap();

        let ask = |names: &[&str]| {
            let owned: Vec<String> = names.iter().map(|s| (*s).to_string()).collect();
            installation_shipped(&root, &owned)
        };
        assert_eq!(
            ask(&[
                "@deepseek-ai/dsh-experimental-agent-team-profile",
                "@deepseek-ai/dsh-base",
            ]),
            vec![
                "@deepseek-ai/dsh-experimental-agent-team-profile".to_string(),
                "@deepseek-ai/dsh-base".to_string()
            ]
        );
        assert!(ask(&["@deepseek-ai/dsh-experimental-auto-review"]).is_empty());
        // 安装目录不存在（引擎还没装）→ 空表，一切照旧由 dock 策展。
        assert!(
            installation_shipped(&root.join("nope"), &["@deepseek-ai/dsh-base".to_string()])
                .is_empty()
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// 「清理旧副本」不能是假按钮：`legacy_copy` 的判据必须与 `planRemove` 的能力面对齐——
    /// bundle 类包的行由包自身的 patch 贡献，我们既没写也不能删，**不算**可清理的痕迹。
    #[test]
    fn legacy_copy_counts_only_what_we_can_remove() {
        // 该 bundle 贡献的行（`AutoBundle` 通道：行由包自身的 patch 插入，壳删不掉）
        let rows = vec![bundle_row(
            "@deepseek-ai/dsh-experimental-agent-team-profile",
            &["agent-team", "tool-agent-team"],
            false,
        )];
        // 行在、但由包的 patch 贡献（`AutoBundle`）且包不在 profile 依赖里 → 无可清理之物。
        let facts = PackageFacts {
            shipped: vec!["@deepseek-ai/dsh-experimental-agent-team-profile".to_string()],
            ..PackageFacts::default()
        };
        let caps = resolve_capabilities(&facts, &rows, None, CopyLang::Zh);
        let view = caps.iter().find(|c| c.id == "agent-team").unwrap();
        assert!(
            !view.legacy_copy,
            "bundle 贡献的行不属于壳，删不掉——算进来就是假按钮"
        );
    }

    /// 面向用户的文案不得含 Markdown 反引号（v1 的字面量渲染缺陷 D6 的回归护栏）。
    /// 中英**两侧**都扫（2026-09-18 边界A：英文文案自这轮起也是用户可见承诺）。
    #[test]
    fn user_facing_copy_has_no_markdown_markup() {
        for cap in CAPABILITIES {
            let mut texts: Vec<String> = vec![
                cap.label_zh.to_string(),
                cap.summary_zh.to_string(),
                cap.unlocks_zh.to_string(),
                cap.label_en.to_string(),
                cap.summary_en.to_string(),
                cap.unlocks_en.to_string(),
            ];
            for v in cap.variants {
                texts.push(v.label_zh.to_string());
                texts.push(v.note_zh.to_string());
                texts.extend(v.prerequisites_zh.iter().map(|s| (*s).to_string()));
                texts.push(v.label_en.to_string());
                texts.push(v.note_en.to_string());
                texts.extend(v.prerequisites_en.iter().map(|s| (*s).to_string()));
            }
            for text in texts {
                // 面板没有 markdown 渲染器：反引号与 `**` 都会**原样显示**给用户
                //（2026-09-17 版面复盘：详情区的「启用后会发生什么」里真的出现了星号）。
                assert!(!text.contains('`'), "{} 的文案含反引号：{text}", cap.id);
                assert!(
                    !text.contains("**"),
                    "{} 的文案含 markdown 强调：{text}",
                    cap.id
                );
            }
        }
    }

    /// 双语**完整性**闸门（2026-09-18 边界A）：每条中文都必须有对应英文，
    /// prerequisites 中英必须**等长**（前端按索引并排展示，缺一项就是整列错位）；
    /// 任何 `_en` 不得留空串——留空 = 英文界面渲染出空白。
    #[test]
    fn catalog_has_complete_english_copy() {
        for cap in CAPABILITIES {
            for (name, zh, en) in [
                ("label", cap.label_zh, cap.label_en),
                ("summary", cap.summary_zh, cap.summary_en),
                ("unlocks", cap.unlocks_zh, cap.unlocks_en),
            ] {
                assert!(!en.trim().is_empty(), "{}.{name} 缺英文文案", cap.id);
                assert_ne!(zh, en, "{}.{name} 英文与中文同值（未翻译？）", cap.id);
            }
            for v in cap.variants {
                assert!(
                    !v.label_en.trim().is_empty(),
                    "{}/{} 缺英文 label",
                    cap.id,
                    v.id
                );
                assert!(
                    !v.note_en.trim().is_empty(),
                    "{}/{} 缺英文 note",
                    cap.id,
                    v.id
                );
                assert_eq!(
                    v.prerequisites_zh.len(),
                    v.prerequisites_en.len(),
                    "{}/{} 的中英前置列表不等长（前端按序展示会错位）",
                    cap.id,
                    v.id
                );
                assert!(
                    v.prerequisites_en.iter().all(|s| !s.trim().is_empty()),
                    "{}/{} 的英文前置含空串",
                    cap.id,
                    v.id
                );
            }
        }
    }

    /// locale 标签解析：未知/缺失一律回退中文（原文语言），`en*` 才出英文。
    #[test]
    fn copy_lang_from_tag_falls_back_to_zh() {
        assert_eq!(CopyLang::from_tag(None), CopyLang::Zh);
        assert_eq!(CopyLang::from_tag(Some("zh-CN")), CopyLang::Zh);
        assert_eq!(CopyLang::from_tag(Some("fr")), CopyLang::Zh);
        assert_eq!(CopyLang::from_tag(Some("en")), CopyLang::En);
        assert_eq!(CopyLang::from_tag(Some("en-US")), CopyLang::En);
        assert_eq!(CopyLang::from_tag(Some("EN-us")), CopyLang::En);
    }

    /// 英文侧不得混入汉字（em dash /  curly apostrophe 等英文排版符号**不算**泄漏——
    /// 判据是"有没有中文"，不是"是不是 ASCII"）。
    fn contains_cjk(s: &str) -> bool {
        s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
    }

    /// 按请求语言出品（2026-09-18 边界A 的收口断言）：同一事实、两种语言，
    /// 视图里的**每一条**用户可见文案都应随 `lang` 切换，而非只切一半。
    /// （变体 label 允许中英同值——Playwright 等是专名，不做相等性断言。）
    #[test]
    fn views_follow_requested_language() {
        let f = facts(&[], &[]);
        let zh = resolve_capabilities(&f, &[], Some("0.1.6-alpha.1"), CopyLang::Zh);
        let en = resolve_capabilities(&f, &[], Some("0.1.6-alpha.1"), CopyLang::En);
        // 未钉版本的提示也双语（runtime_version = None 分支）
        let en_nowarn = resolve_capabilities(&f, &[], None, CopyLang::En);
        let step = en_nowarn
            .iter()
            .flat_map(|c| c.variants.iter())
            .flat_map(|v| v.steps.iter())
            .find(|s| s.version_notice.is_some())
            .expect("runtime_version=None 时每步都该有提示");
        assert!(
            !contains_cjk(step.version_notice.as_deref().unwrap()),
            "英文视图里的版本提示漏了中文：{}",
            step.version_notice.as_deref().unwrap()
        );
        for (a, b) in zh.iter().zip(en.iter()) {
            assert_ne!(a.label, b.label, "{} label 未随语言切换", a.id);
            assert_ne!(a.summary, b.summary, "{} summary 未随语言切换", a.id);
            assert_ne!(a.unlocks, b.unlocks, "{} unlocks 未随语言切换", a.id);
            assert!(!contains_cjk(&b.label));
            assert!(!contains_cjk(&b.summary));
            assert!(!contains_cjk(&b.unlocks));
            for (va, vb) in a.variants.iter().zip(b.variants.iter()) {
                assert_eq!(va.id, vb.id);
                assert_ne!(va.note, vb.note, "{}/{} note 未随语言切换", a.id, va.id);
                assert_eq!(va.prerequisites.len(), vb.prerequisites.len());
                assert!(vb.prerequisites.iter().all(|s| !contains_cjk(s)));
                assert!(
                    vb.prerequisite_missing.is_none(),
                    "本用例未探测缺失命令，不该出现硬门文案"
                );
            }
        }
        // 硬门文案双语（缺 cua-driver + 请求英文）
        let gate = resolve_capabilities(
            &facts_missing(&[], &[], &["cua-driver"]),
            &[],
            Some("0.1.6-alpha.1"),
            CopyLang::En,
        );
        let blocked = find(&gate, "computer-use")
            .variants
            .iter()
            .find(|v| v.id == "cua-driver-mcp")
            .expect("目录里有该变体");
        let reason = blocked
            .prerequisite_missing
            .as_deref()
            .expect("缺前置必须报硬门");
        assert!(
            !contains_cjk(reason),
            "英文视图的硬门文案漏了中文：{reason}"
        );
        assert!(reason.contains("cua-driver"));
    }

    /// 「装了」不等于「生效」：包已装但挂载行缺失 → Partial（需要修复），
    /// 而不是 On。这是 v1 只报 `installed` 时最危险的静默错误。
    #[test]
    fn installed_without_row_is_partial_not_on() {
        const PLAYWRIGHT: &str = "@deepseek-ai/dsh-experimental-browser-use-playwright-mcp";
        let f = facts(&["@deepseek-ai/dsh-browser-use", PLAYWRIGHT], &[]);
        let caps = resolve_capabilities(&f, &[], Some("0.1.6-alpha.1"), CopyLang::Zh);
        let cap = find(&caps, "browser-use");
        let v = variant(cap, "playwright");
        assert_eq!(v.state, VariantState::Partial, "行缺失必须报需要修复");
        // 基座包（bundle 类）无贡献行 → 视为已激活；provider 行缺 → not ready。
        assert_eq!(cap.state, CapabilityState::Partial);
        assert!(cap.active_variant.is_none());
    }

    /// 行写到位 = On，且 `toggle_targets` 给出**精确**的壳行 id（停用靠它）。
    #[test]
    fn row_present_means_on_and_exposes_toggle_target() {
        const PLAYWRIGHT: &str = "@deepseek-ai/dsh-experimental-browser-use-playwright-mcp";
        let f = facts(&["@deepseek-ai/dsh-browser-use", PLAYWRIGHT], &[]);
        let rows = vec![
            row(
                &row_id_for("@deepseek-ai/dsh-browser-use"),
                "@deepseek-ai/dsh-browser-use",
                false,
            ),
            row(&row_id_for(PLAYWRIGHT), PLAYWRIGHT, false),
        ];
        let caps = resolve_capabilities(&f, &rows, Some("0.1.6-alpha.1"), CopyLang::Zh);
        let cap = find(&caps, "browser-use");
        let v = variant(cap, "playwright");
        assert_eq!(v.state, VariantState::On);
        assert!(v.toggle_off_supported, "两步都是壳写的行 → 关而不卸可行");
        assert_eq!(cap.state, CapabilityState::On);
        assert_eq!(cap.active_variant.as_deref(), Some("playwright"));
        let provider = v.steps.iter().find(|s| s.package == PLAYWRIGHT).unwrap();
        assert_eq!(provider.toggle_targets, vec![row_id_for(PLAYWRIGHT)]);
        assert!(provider.row_present && !provider.disabled);
    }

    /// 停用（行在但 `disabled: true`）→ Disabled：与 On **必须**区分开——
    /// 「已就位但关着」和「正在生效」是用户最需要分辨的两种状态。
    #[test]
    fn disabled_row_is_its_own_state_and_still_quickly_reversible() {
        const PLAYWRIGHT: &str = "@deepseek-ai/dsh-experimental-browser-use-playwright-mcp";
        let f = facts(&["@deepseek-ai/dsh-browser-use", PLAYWRIGHT], &[]);
        let rows = vec![
            row(
                &row_id_for("@deepseek-ai/dsh-browser-use"),
                "@deepseek-ai/dsh-browser-use",
                true,
            ),
            row(&row_id_for(PLAYWRIGHT), PLAYWRIGHT, true),
        ];
        let caps = resolve_capabilities(&f, &rows, Some("0.1.6-alpha.1"), CopyLang::Zh);
        let cap = find(&caps, "browser-use");
        assert_eq!(variant(cap, "playwright").state, VariantState::Disabled);
        assert_eq!(cap.state, CapabilityState::Disabled);
        assert_eq!(cap.active_variant.as_deref(), Some("playwright"));
        // 停用态仍然"可开关"——这正是"关掉不必卸载"的落点。
        assert!(variant(cap, "playwright").toggle_off_supported);
    }

    /// bundle 类包：行由**包自身**贡献（`contributed_ids` 多行），启用目标取全部贡献行；
    /// 但**关闭不走行级开关**——层的 patch 带副作用（实测 agent-team-profile 还停用了
    /// 4 条旧 subagent 行），行级 disabled 关不干净，故 `toggle_off_supported` 必须为 false。
    #[test]
    fn auto_bundle_rows_are_togglable_but_toggle_off_is_not_equivalent_to_off() {
        const REVIEW: &str = "@deepseek-ai/dsh-experimental-auto-review";
        let f = facts(&[REVIEW], &[REVIEW]);

        let with_rows = vec![bundle_row(REVIEW, &["review-a", "review-b"], false)];
        let caps = resolve_capabilities(&f, &with_rows, Some("0.1.6-alpha.1"), CopyLang::Zh);
        let cap = find(&caps, "auto-review");
        let v = variant(cap, "standard");
        assert_eq!(v.state, VariantState::On);
        assert_eq!(
            v.steps[0].toggle_targets,
            vec!["review-a".to_string(), "review-b".to_string()]
        );
        assert!(
            !v.toggle_off_supported,
            "层类能力必须走移除：行级停用会留下层的副作用"
        );

        // 无贡献行：仍然"已激活"（CLI 管），但没有可切的行。
        let caps = resolve_capabilities(&f, &[], Some("0.1.6-alpha.1"), CopyLang::Zh);
        let cap = find(&caps, "auto-review");
        let v = variant(cap, "standard");
        assert_eq!(v.state, VariantState::On);
        assert!(v.steps[0].toggle_targets.is_empty());
    }

    /// 纯行级开关的**准入条件**：每一步都得是壳写的行。provider 两个能力满足
    /// （基座包 `dsh-browser-use` / `dsh-computer-use` 实测**无 `dsh` 字段** → 也是行），
    /// 含层包的能力永不满足。
    #[test]
    fn toggle_off_supported_only_for_insert_row_variants() {
        let rows = vec![
            row(
                &row_id_for("@deepseek-ai/dsh-browser-use"),
                "@deepseek-ai/dsh-browser-use",
                false,
            ),
            row(
                &row_id_for("@deepseek-ai/dsh-experimental-browser-use-playwright-mcp"),
                "@deepseek-ai/dsh-experimental-browser-use-playwright-mcp",
                false,
            ),
        ];
        let f = facts(
            &[
                "@deepseek-ai/dsh-browser-use",
                "@deepseek-ai/dsh-experimental-browser-use-playwright-mcp",
            ],
            &[],
        );
        let caps = resolve_capabilities(&f, &rows, Some("0.1.6-alpha.1"), CopyLang::Zh);
        assert_eq!(find(&caps, "browser-use").state, CapabilityState::On);
        assert!(variant(find(&caps, "browser-use"), "playwright").toggle_off_supported);

        for cap_id in ["agent-team", "auto-review"] {
            for v in &find(&caps, cap_id).variants {
                assert!(
                    !v.toggle_off_supported,
                    "{cap_id}::{} 未装 → 无从判定，不得声称可纯行级关闭",
                    v.id
                );
            }
        }

        // 含 profile 层的能力：即使**已装**也不得声称可纯行级关闭（层 patch 带副作用）。
        let layer_pkgs = [
            "@deepseek-ai/dsh-experimental-agent-team-profile",
            "@deepseek-ai/dsh-experimental-agent-team-web-profile",
        ];
        let layer = facts(&layer_pkgs, &layer_pkgs);
        let caps = resolve_capabilities(&layer, &[], Some("0.1.6-alpha.1"), CopyLang::Zh);
        let web = variant(find(&caps, "agent-team"), "web");
        assert_eq!(web.state, VariantState::On, "层装上即由 CLI 激活");
        assert!(!web.toggle_off_supported, "层类能力不得声称可纯行级关闭");
    }

    /// Agent Teams 的**子集档**不得报冲突（2026-09-16 真机暴露）：
    /// 自建档 = `[agent-team-profile]` 是 Web 档 `[同一包, agent-team-web-profile]` 的
    /// 真子集；装 Web 档时子集档的包必然也齐——那是**完全正常的配置**，不是"两个后端并列"。
    #[test]
    fn subset_variant_is_subsumed_not_conflicting() {
        const HOST: &str = "@deepseek-ai/dsh-experimental-agent-team-profile";
        const WEB: &str = "@deepseek-ai/dsh-experimental-agent-team-web-profile";
        // 两个包都是 profile 层（声明 dsh.bundle）→ 装上即由 CLI 激活，无壳行。
        let f = facts(&[HOST, WEB], &[HOST, WEB]);
        let caps = resolve_capabilities(&f, &[], Some("0.1.6-alpha.1"), CopyLang::Zh);
        let cap = find(&caps, "agent-team");

        assert_eq!(cap.state, CapabilityState::On, "子集关系不是冲突");
        assert_eq!(cap.active_variant.as_deref(), Some("web"), "活动档取超集");
        let headless = variant(cap, "headless");
        assert_eq!(headless.state, VariantState::On, "它的包确实齐了");
        assert_eq!(
            headless.subsumed_by.as_deref(),
            Some("web"),
            "必须显式说明「已被 Web 档包含」，UI 据此禁用它的独立开关"
        );
        assert!(
            variant(cap, "web").subsumed_by.is_none(),
            "超集档自己不被包含"
        );
    }

    /// 反向护栏：**真互斥**的两族仍必须报冲突（子集豁免不得把互斥一起放过）。
    #[test]
    fn genuinely_exclusive_variants_still_conflict() {
        const PW: &str = "@deepseek-ai/dsh-experimental-browser-use-playwright-mcp";
        const CDP: &str = "@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp";
        let f = facts(&["@deepseek-ai/dsh-browser-use", PW, CDP], &[]);
        let rows = vec![
            row(
                &row_id_for("@deepseek-ai/dsh-browser-use"),
                "@deepseek-ai/dsh-browser-use",
                false,
            ),
            row(&row_id_for(PW), PW, false),
            row(&row_id_for(CDP), CDP, false),
        ];
        let caps = resolve_capabilities(&f, &rows, Some("0.1.6-alpha.1"), CopyLang::Zh);
        let cap = find(&caps, "browser-use");
        assert_eq!(cap.state, CapabilityState::Conflict);
        assert!(variant(cap, "playwright").subsumed_by.is_none());
        assert!(variant(cap, "chrome-devtools").subsumed_by.is_none());
    }

    /// 同能力两个变体同时生效 = 冲突（上游会激活失败），必须单独报出来，
    /// 而不是随便挑一个报 On。
    #[test]
    fn two_live_variants_report_conflict() {
        const PW: &str = "@deepseek-ai/dsh-experimental-browser-use-playwright-mcp";
        const CDP: &str = "@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp";
        let f = facts(&["@deepseek-ai/dsh-browser-use", PW, CDP], &[]);
        let rows = vec![
            row(
                &row_id_for("@deepseek-ai/dsh-browser-use"),
                "@deepseek-ai/dsh-browser-use",
                false,
            ),
            row(&row_id_for(PW), PW, false),
            row(&row_id_for(CDP), CDP, false),
        ];
        let caps = resolve_capabilities(&f, &rows, Some("0.1.6-alpha.1"), CopyLang::Zh);
        let cap = find(&caps, "browser-use");
        assert_eq!(cap.state, CapabilityState::Conflict);
        assert!(cap.active_variant.is_none(), "冲突时不得谎报单一活动变体");
    }

    /// 变体切换的预判：另一个变体的后端已装 → `displaced` 报出包名（UI 据此说
    /// "将先替换掉谁"），而**共享的基座包不得**被算成需要替换。
    #[test]
    fn switching_variant_displaces_only_the_other_backend() {
        const CDP: &str = "@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp";
        let f = facts(&["@deepseek-ai/dsh-browser-use", CDP], &[]);
        let caps = resolve_capabilities(&f, &[], Some("0.1.6-alpha.1"), CopyLang::Zh);
        let cap = find(&caps, "browser-use");
        assert_eq!(
            variant(cap, "playwright").displaced,
            vec![CDP.to_string()],
            "换后端须先替换掉旧后端"
        );
        assert!(
            variant(cap, "chrome-devtools").displaced.is_empty(),
            "同一个后端自己不算被替换，基座包也不算"
        );
    }

    /// 跨能力不互斥：装了 browser-use 不影响 computer-use 的状态判定。
    #[test]
    fn capabilities_do_not_leak_across() {
        const CDP: &str = "@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp";
        let f = facts(&["@deepseek-ai/dsh-browser-use", CDP], &[]);
        let caps = resolve_capabilities(&f, &[], Some("0.1.6-alpha.1"), CopyLang::Zh);
        assert_eq!(find(&caps, "computer-use").state, CapabilityState::Off);
        assert_eq!(find(&caps, "auto-review").state, CapabilityState::Off);
        assert_eq!(find(&caps, "agent-team").state, CapabilityState::Off);
    }

    /// 运行时版本**未检出**时必须**降级而非拼坏 spec**：退回裸包名，且**必须**给出
    /// 未钉版本的显式告知（沉默会让用户以为已钉好）。
    #[test]
    fn resolve_without_runtime_version_degrades_honestly() {
        let caps = resolve_capabilities(&PackageFacts::default(), &[], None, CopyLang::Zh);
        let step = &find(&caps, "auto-review").variants[0].steps[0];
        assert_eq!(
            step.spec, step.package,
            "版本未知时不得拼出 `pkg@` 这种坏 spec：{}",
            step.spec
        );
        assert!(
            !step.spec.ends_with('@'),
            "spec 不得以 @ 结尾：{}",
            step.spec
        );
        let notice = step
            .version_notice
            .as_deref()
            .expect("未钉版本必须显式告知");
        assert!(
            notice.contains("未钉版本") && notice.contains("latest"),
            "告知须点明风险：{notice}"
        );
    }

    /// 钉版本落到**每一步**上（含 Agent Teams 的 web 档两步），且路径为**有序**：
    /// 宿主层在前、Web 层在后（装反即激活失败）。
    #[test]
    fn every_step_carries_the_pin_in_order() {
        let caps = resolve_capabilities(
            &PackageFacts::default(),
            &[],
            Some("0.1.6-alpha.1"),
            CopyLang::Zh,
        );
        let v = variant(find(&caps, "agent-team"), "web");
        assert_eq!(v.steps.len(), 2);
        for s in &v.steps {
            assert!(s.spec.ends_with("@0.1.6-alpha.1"), "{}", s.spec);
            assert!(s.version_notice.is_none());
        }
        assert!(v.steps[0].package.contains("agent-team-profile"));
        assert!(v.steps[1].package.contains("agent-team-web-profile"));
    }

    /// **配置载荷与前置命令的表不得悬空**（2026-09-16 真机事故的对偶约束）：
    /// 表里的每个包名都必须在策展集里真的出现——否则那是一段"看着有、其实永不生效"
    /// 的死数据（本仓库明令禁止的死路径）。
    #[test]
    fn row_config_and_prerequisite_tables_only_name_catalog_packages() {
        let catalog: Vec<&str> = CAPABILITIES
            .iter()
            .flat_map(|c| c.variants.iter())
            .flat_map(|v| v.packages.iter().copied())
            .collect();
        for (package, fields) in ROW_CONFIGS {
            assert!(
                catalog.contains(package),
                "ROW_CONFIGS 里的「{package}」不在策展集中（死数据）"
            );
            assert!(!fields.is_empty(), "「{package}」的 config 表为空");
        }
        for (package, command) in PACKAGE_REQUIRES_COMMAND {
            assert!(
                catalog.contains(package),
                "PACKAGE_REQUIRES_COMMAND 里的「{package}」不在策展集中（死数据）"
            );
            assert!(!command.trim().is_empty(), "「{package}」的前置命令为空");
        }
    }

    /// 浏览器族 MCP provider **必须**带上游要求的 `mode`（缺它 dsh 起不来）。
    #[test]
    fn browser_mcp_providers_require_mode_in_row_config() {
        for package in [
            "@deepseek-ai/dsh-experimental-browser-use-playwright-mcp",
            "@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp",
        ] {
            let fields = required_row_config(package);
            let mode = fields
                .iter()
                .find(|(key, _)| *key == "mode")
                .unwrap_or_else(|| panic!("「{package}」必须写 mode"));
            assert_eq!(mode.1, ConfigValue::Str("launch"));
        }
        assert!(
            required_row_config("@deepseek-ai/dsh-experimental-computer-use-cua-driver-native")
                .is_empty(),
            "native 档无配置字段，不得凭空写 config"
        );
    }

    /// 宿主前置缺失 = **硬门**：缺 `cua-driver` 时只有"复用已装 cua-driver"那档被挡，
    /// 自包含档不受影响；前置齐备时两档都不报缺。
    #[test]
    fn missing_driver_blocks_only_the_variant_that_needs_it() {
        let caps = resolve_capabilities(
            &facts_missing(&[], &[], &["cua-driver"]),
            &[],
            Some("0.1.6-alpha.1"),
            CopyLang::Zh,
        );
        let cap = find(&caps, "computer-use");
        let blocked = variant(cap, "cua-driver-mcp");
        let reason = blocked
            .prerequisite_missing
            .as_deref()
            .expect("缺 cua-driver 必须报前置未满足");
        assert!(
            reason.contains("cua-driver"),
            "原因必须点名缺什么：{reason}"
        );
        assert!(
            variant(cap, "cua-driver-native")
                .prerequisite_missing
                .is_none(),
            "自包含档无外部前置，不得被一起挡掉"
        );

        let ok = resolve_capabilities(&facts(&[], &[]), &[], Some("0.1.6-alpha.1"), CopyLang::Zh);
        assert!(
            variant(find(&ok, "computer-use"), "cua-driver-mcp")
                .prerequisite_missing
                .is_none(),
            "前置齐备时不得报缺"
        );
    }
}
