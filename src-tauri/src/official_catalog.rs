//! official_catalog.rs —— 官方插件策展目录的**纯决策层**（2026-09-15，ADR-0020 已接受）。
//!
//! ## 职责边界
//!
//! 本模块只做「装什么、谁来激活、按什么顺序、彼此是否互斥」的**纯判定**：
//! 不触网、不起子进程、不写文件。安装仍归 `commands/plugin.rs` 的
//! `dsh plugin add` 转发链，patch 写入仍归 `plugins.rs::PatchFile`。
//!
//! 为什么单独成模块并做成纯函数（ADR-0020 §2.10 / §4）：激活契约是**按包分类的分支**，
//! 其正确性完全取决于「目标包是否声明 `dsh.bundle`」。把它纯函数化 + 单测，才能让
//! 「**退出码 0 但插件没生效**」这类静默失败**先在壳内红**——这正是 v1 方案两向皆错的
//! 地方（统一 `- name:` → 6 个包静默漏挂；统一 `- insert:` → bundle 类重复挂载）。
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

/// 期望版本与 registry `latest` 不一致时给出提示文案（`None` = 一致，无需提示）。
///
/// 用于安装确认弹窗「显示将要安装的确切版本」这一要求：当 `latest` 落后时，
/// 用户必须看到壳在钉版本，而不是以为自己拿到的是最新版。
pub fn version_skew_notice(latest: &str, wanted: &str) -> Option<String> {
    if latest == wanted {
        return None;
    }
    Some(format!(
        "registry latest 为 {latest}，与运行时期望的 {wanted} 不一致；\
         将按 {wanted} 安装（裸包名会装到 latest，属已知错配风险）"
    ))
}

/// 互斥族：同族 provider **装第二个会激活失败**（ADR-0020 §2.9）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExclusiveFamily {
    /// 浏览器后端三选一（`docs/subsystems/browser-use.md:17`）。
    BrowserUse,
    /// 桌面控制二选一（`computer-use-cua-driver-mcp/README.md:53`：
    /// "A second computer-use provider **fails activation**"）。
    ComputerUse,
}

/// 策展条目：一条「用户可点」的目录项，展开为**有序**的安装步骤。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    /// 展示名（中文）。
    pub label_zh: &'static str,
    /// 需要安装的包，**按顺序**（顺序即语义，见 `install_plan`）。
    pub packages: &'static [&'static str],
    /// 互斥族；`None` = 不与任何条目互斥。
    pub family: Option<ExclusiveFamily>,
    /// 是否需要额外前置（展示用；不参与判定）。
    pub requires_note_zh: Option<&'static str>,
}

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

/// 把条目展开为**有序、串行**的安装步骤（ADR-0020 §2.10）。
///
/// 顺序不是装饰：`agent-team-web-profile` 的类型声明要求
/// `dsh-base` → `dsh-web-app` → `dsh-agent-team-profile` → 本包 **must remain in that order**，
/// 且其 `cordis.patch.yml` 首行注明 "Apply **after** … the host-side
/// `dsh-agent-team-profile` so the browser mounts only when the Team service is present"。
/// 装反了浏览器侧会在 Team 服务就位前挂载而失败。
///
/// 同时：`dsh plugin add` **一次只接受一个包**，故必须逐步下发；调用方须**串行**执行
/// （既有安装队列本身就是串行编排）。`ordinal` 让"第二步失败"能停在一致态并可续装。
pub fn install_plan(entry: &CatalogEntry) -> Vec<InstallStep> {
    entry
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

/// 已装 provider 与将要安装的条目是否**同族冲突**。
///
/// 返回冲突时已装的那个包名，供 UI 走"**替换**"流程（先移除再安装）而非"叠加"。
/// 同族且**同一个包**不算冲突（重复安装同一 provider 属幂等重装）。
pub fn exclusive_conflict<'a>(
    family: ExclusiveFamily,
    already_installed: &'a [String],
    incoming: &CatalogEntry,
) -> Option<&'a str> {
    if incoming.family != Some(family) {
        return None;
    }
    already_installed
        .iter()
        .find(|installed| {
            // 冲突判据严格为两条同时成立：① 已装包**属于同一互斥族**
            // （未知包不得算冲突，否则会拦下无关安装）；② 它**不在**本次要装的包里
            // （同一 provider 重装属幂等，不拦）。
            family_of_package(installed) == Some(family)
                && !incoming.packages.contains(&installed.as_str())
        })
        .map(String::as_str)
}

/// 已知包名 → 互斥族（仅覆盖策展集内的 provider；未知包返回 `None`）。
pub fn family_of_package(package: &str) -> Option<ExclusiveFamily> {
    match package {
        p if p.starts_with("@deepseek-ai/dsh-experimental-browser-use-")
            || p == "@deepseek-ai/dsh-browser-use" =>
        {
            Some(ExclusiveFamily::BrowserUse)
        }
        p if p.starts_with("@deepseek-ai/dsh-experimental-computer-use-")
            || p == "@deepseek-ai/dsh-computer-use" =>
        {
            Some(ExclusiveFamily::ComputerUse)
        }
        _ => None,
    }
}

/// 策展集（**dsh-dock 策展，非"官方首批"**——上游无该概念，为全量发布 16 包，
/// ADR-0020 §1.1）。`packages` 顺序即安装顺序，勿随意重排。
pub const CATALOG: &[CatalogEntry] = &[
    CatalogEntry {
        label_zh: "自动安全审查（Auto review）",
        packages: &["@deepseek-ai/dsh-experimental-auto-review"],
        family: None,
        requires_note_zh: None,
    },
    CatalogEntry {
        label_zh: "浏览器操作 · Playwright（三选一）",
        packages: &[
            "@deepseek-ai/dsh-browser-use",
            "@deepseek-ai/dsh-experimental-browser-use-playwright-mcp",
        ],
        family: Some(ExclusiveFamily::BrowserUse),
        requires_note_zh: Some("需自备 Chromium；`browser-use-runtime` 是库、不可挂载，随依赖带入"),
    },
    CatalogEntry {
        label_zh: "浏览器操作 · Chrome DevTools（三选一）",
        packages: &[
            "@deepseek-ai/dsh-browser-use",
            "@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp",
        ],
        family: Some(ExclusiveFamily::BrowserUse),
        requires_note_zh: Some("需本机 Chrome；与另两个 browser-use 后端互斥"),
    },
    CatalogEntry {
        label_zh: "浏览器操作 · Stagehand（三选一）",
        packages: &[
            "@deepseek-ai/dsh-browser-use",
            "@deepseek-ai/dsh-experimental-browser-use-stagehand-native",
        ],
        family: Some(ExclusiveFamily::BrowserUse),
        requires_note_zh: Some("`model` 必填（连纯导航也要求）；与另两个后端互斥"),
    },
    CatalogEntry {
        label_zh: "桌面控制 · Cua Driver（MCP，二选一）",
        packages: &[
            "@deepseek-ai/dsh-computer-use",
            "@deepseek-ai/dsh-experimental-computer-use-cua-driver-mcp",
        ],
        family: Some(ExclusiveFamily::ComputerUse),
        requires_note_zh: Some("需外置安装 cua-driver；与 native 变体互斥"),
    },
    CatalogEntry {
        label_zh: "桌面控制 · Cua Driver（原生，二选一）",
        packages: &[
            "@deepseek-ai/dsh-computer-use",
            "@deepseek-ai/dsh-experimental-computer-use-cua-driver-native",
        ],
        family: Some(ExclusiveFamily::ComputerUse),
        requires_note_zh: Some("随依赖带入原生运行时；与 mcp 变体互斥"),
    },
    CatalogEntry {
        label_zh: "多智能体协同 · Agent Teams（headless / 自建档）",
        packages: &["@deepseek-ai/dsh-experimental-agent-team-profile"],
        family: None,
        requires_note_zh: Some("该 convenience bundle 会一并插入 team 域与工具行"),
    },
    CatalogEntry {
        label_zh: "多智能体协同 · Agent Teams（Web 档，**须在上一层之后**）",
        packages: &[
            "@deepseek-ai/dsh-experimental-agent-team-profile",
            "@deepseek-ai/dsh-experimental-agent-team-web-profile",
        ],
        family: None,
        requires_note_zh: Some(
            "两步**有序**：先建 host 层再建 Web 层；顺序颠倒浏览器侧会因 Team 服务未就位而挂载失败",
        ),
    },
];

/// 一条目录行的**一步**（可序列化给前端）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogStep {
    /// 第几步（1 起；UI 据此展示进度与"第二步未完成"的一致态）。
    pub ordinal: usize,
    /// 包名。
    pub package: String,
    /// 钉版本后的 spec（交给 `dsh plugin add`）。
    pub spec: String,
    /// 激活方式：决定壳写不写 patch 行（见 [`Activation`]）。
    pub activation: Activation,
    /// patch 行 id（`InsertRow` 时才真正落盘）。
    pub row_id: String,
    /// 该包当前是否已声明在 profile 依赖里。
    pub installed: bool,
    /// 版本错配提示（`latest` 落后于运行时等）；`None` = 无需打扰用户。
    pub version_notice: Option<String>,
}

/// 一条目录行（可序列化给前端）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogRow {
    /// 展示名（中文）。
    pub label_zh: &'static str,
    /// 互斥族；UI 据此把同族行渲染成"替换"而非"叠加"。
    pub family: Option<ExclusiveFamily>,
    /// 前置条件/注意事项（展示用）。
    pub requires_note_zh: Option<&'static str>,
    /// 有序步骤。
    pub steps: Vec<CatalogStep>,
    /// 与**已装**同族 provider 的冲突（`Some` = UI 须走"替换"流程）。
    pub conflict_with: Option<String>,
}

/// 已装状态与 registry 事实的**入参快照**（由调用方采集，本模块保持纯函数）。
#[derive(Debug, Clone, Default)]
pub struct InstalledState {
    /// profile `dependencies` 的包名。
    pub installed: Vec<String>,
    /// 已装并**声明 `dsh.bundle`** 的包名（激活分类的唯一依据）。
    pub declared_bundles: Vec<String>,
    /// 包名 → registry `latest`（可空；用于版本错配提示）。
    pub latest_by_package: std::collections::HashMap<String, String>,
}

/// 把策展集解析为可下发的目录行（纯函数）。
///
/// 每一步的 `activation` 由 `declared_bundles` 决定：
/// - 在册 → [`Activation::AutoBundle`]（`dsh plugin add` 自行激活，壳**不写** patch）；
/// - 不在册 → [`Activation::InsertRow`]（壳**必须**写挂载行）。
///
/// `runtime_version` 用于钉版本（ADR-0020 §2.4）——**裸包名会装到 `latest`**，
/// 而 Agent Teams 三包的 `latest` 实测落后于运行时。
pub fn resolve_rows(state: &InstalledState, runtime_version: Option<&str>) -> Vec<CatalogRow> {
    let declared: std::collections::HashSet<&str> =
        state.declared_bundles.iter().map(String::as_str).collect();

    CATALOG
        .iter()
        .map(|entry| {
            let steps = install_plan(entry)
                .into_iter()
                .map(|step| {
                    let declares_bundle = declared.contains(step.package.as_str());
                    // 版本未知时**不拼 `pkg@`**（那是个坏 spec）：退回裸包名并**明示风险**
                    // ——裸包名按 `latest` 解析，正是 Agent Teams 三包错配的成因。
                    let (spec, notice) = match runtime_version {
                        Some(v) => (
                            pinned_spec(&step.package, v),
                            state
                                .latest_by_package
                                .get(&step.package)
                                .and_then(|latest| version_skew_notice(latest, v)),
                        ),
                        None => (
                            step.package.clone(),
                            Some(
                                "运行时版本未检出，本步**未钉版本**——裸包名会按 latest 解析，\
                                 可能与运行时错配（如需严格匹配请先让引擎就绪）"
                                    .to_string(),
                            ),
                        ),
                    };
                    CatalogStep {
                        ordinal: step.ordinal,
                        activation: activation_for(declares_bundle),
                        spec,
                        installed: state.installed.contains(&step.package),
                        version_notice: notice,
                        row_id: step.row_id,
                        package: step.package,
                    }
                })
                .collect();
            CatalogRow {
                label_zh: entry.label_zh,
                family: entry.family,
                requires_note_zh: entry.requires_note_zh,
                steps,
                conflict_with: entry
                    .family
                    .and_then(|f| exclusive_conflict(f, &state.installed, entry))
                    .map(str::to_string),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// B3 护栏：spec 必须带版本；`latest` 与期望不一致时**必须**提示（不得静默）。
    #[test]
    fn pinned_spec_carries_version_and_skew_is_surfaced() {
        let spec = pinned_spec(
            "@deepseek-ai/dsh-experimental-agent-team-profile",
            "0.1.6-alpha.1",
        );
        assert!(spec.ends_with("@0.1.6-alpha.1"), "{spec}");

        // 本机实测的错配形态：latest 落后于运行时。
        let notice = version_skew_notice("0.1.5-alpha.2", "0.1.6-alpha.1")
            .expect("latest 落后时必须给出提示");
        assert!(notice.contains("0.1.5-alpha.2") && notice.contains("0.1.6-alpha.1"));
        // 一致时不得打扰用户。
        assert!(version_skew_notice("0.1.6-alpha.1", "0.1.6-alpha.1").is_none());
    }

    /// 有序组合：步骤序号从 1 连续递增，顺序与 `packages` 声明一致
    /// （Agent Teams 的 Web 档装反即失败，顺序是语义不是装饰）。
    #[test]
    fn install_plan_preserves_declared_order_and_ordinals() {
        let web_team = CATALOG
            .iter()
            .find(|e| e.label_zh.contains("Web 档"))
            .expect("策展集须含 Agent Teams Web 档条目");
        let plan = install_plan(web_team);
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0].ordinal, 1);
        assert_eq!(plan[1].ordinal, 2);
        assert!(plan[0].package.contains("agent-team-profile"));
        assert!(plan[1].package.contains("agent-team-web-profile"));
        // 每一步都带自己的稳定 id（缺 id 的行永不可再 patch）。
        assert_ne!(plan[0].row_id, plan[1].row_id);
    }

    /// 互斥组：同族换一个 provider = 冲突（须走"替换"）；同包重装 = 不拦（幂等）。
    #[test]
    fn exclusive_conflict_distinguishes_replace_from_reinstall() {
        let playwright = CATALOG
            .iter()
            .find(|e| e.label_zh.contains("Playwright"))
            .unwrap();
        let installed_other =
            vec!["@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp".to_string()];
        assert_eq!(
            exclusive_conflict(ExclusiveFamily::BrowserUse, &installed_other, playwright),
            Some("@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp"),
            "同族不同后端须判冲突（UI 走替换）"
        );

        let installed_same: Vec<String> =
            playwright.packages.iter().map(|p| p.to_string()).collect();
        assert_eq!(
            exclusive_conflict(ExclusiveFamily::BrowserUse, &installed_same, playwright),
            None,
            "同包重装是幂等，不该拦"
        );

        // 跨族不互斥：装了 browser-use 不影响装 computer-use。
        let cua = CATALOG
            .iter()
            .find(|e| e.label_zh.contains("Cua Driver（MCP"))
            .expect("策展集须含 Cua Driver MCP 条目");
        assert_eq!(
            exclusive_conflict(ExclusiveFamily::ComputerUse, &installed_other, cua),
            None
        );
    }

    /// provider 互斥族的判别：两个 release 服务与 6 个 provider 都要归族，
    /// 非 provider 包不得误判（否则会拦下正常安装）。
    #[test]
    fn family_classification_covers_services_and_providers_only() {
        for p in [
            "@deepseek-ai/dsh-browser-use",
            "@deepseek-ai/dsh-experimental-browser-use-playwright-mcp",
            "@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp",
            "@deepseek-ai/dsh-experimental-browser-use-stagehand-native",
        ] {
            assert_eq!(
                family_of_package(p),
                Some(ExclusiveFamily::BrowserUse),
                "{p}"
            );
        }
        for p in [
            "@deepseek-ai/dsh-computer-use",
            "@deepseek-ai/dsh-experimental-computer-use-cua-driver-mcp",
            "@deepseek-ai/dsh-experimental-computer-use-cua-driver-native",
        ] {
            assert_eq!(
                family_of_package(p),
                Some(ExclusiveFamily::ComputerUse),
                "{p}"
            );
        }
        // 非 provider：不得误判。
        assert_eq!(
            family_of_package("@deepseek-ai/dsh-experimental-auto-review"),
            None
        );
        assert_eq!(family_of_package("@deepseek-ai/dsh-base"), None);
    }

    /// 策展集自身的一致性：每个条目 non-empty、包名唯一（同一条目内不重复）、
    /// 互斥族条目必须声明 `family`（否则互斥校验形同虚设）。
    #[test]
    fn catalog_entries_are_well_formed() {
        for entry in CATALOG {
            assert!(!entry.packages.is_empty(), "{} 无包", entry.label_zh);
            let mut seen = std::collections::HashSet::new();
            for p in entry.packages {
                assert!(seen.insert(*p), "{} 条目内包名重复：{p}", entry.label_zh);
            }
        }
        // 浏览器/桌面控制条目必须归族。
        for entry in CATALOG {
            let touches_browser = entry
                .packages
                .iter()
                .any(|p| family_of_package(p) == Some(ExclusiveFamily::BrowserUse));
            if touches_browser {
                assert_eq!(
                    entry.family,
                    Some(ExclusiveFamily::BrowserUse),
                    "{} 触及 browser-use 却未归族",
                    entry.label_zh
                );
            }
        }
    }

    /// 端到端（纯函数）解析：两种激活分支、钉版本、已装标记、冲突提示
    /// 必须在同一次解析里**各就各位**——这正是 v1 方案两向皆错的那层判定。
    #[test]
    fn resolve_rows_marks_activation_pin_and_conflict() {
        let state = InstalledState {
            installed: vec![
                // 已装一个 bundle 类包 + 一个同族的**别的** provider（制造冲突）。
                "@deepseek-ai/dsh-experimental-auto-review".to_string(),
                "@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp".to_string(),
            ],
            declared_bundles: vec!["@deepseek-ai/dsh-experimental-auto-review".to_string()],
            latest_by_package: std::collections::HashMap::from([(
                "@deepseek-ai/dsh-experimental-agent-team-profile".to_string(),
                "0.1.5-alpha.2".to_string(),
            )]),
        };
        let rows = resolve_rows(&state, Some("0.1.6-alpha.1"));

        // ① auto-review 声明了 dsh.bundle → 自动激活（壳不写 patch），且已装。
        let review = rows
            .iter()
            .find(|r| r.label_zh.contains("Auto review"))
            .unwrap();
        assert_eq!(review.steps[0].activation, Activation::AutoBundle);
        assert!(review.steps[0].installed);
        assert!(
            review.steps[0].version_notice.is_none(),
            "latest 未提供则不提"
        );

        // ② Playwright 条目未装 → 须写 insert 行；且与已装的别的后端冲突。
        let playwright = rows
            .iter()
            .find(|r| r.label_zh.contains("Playwright"))
            .unwrap();
        assert_eq!(playwright.steps[1].activation, Activation::InsertRow);
        assert!(!playwright.steps[1].installed);
        assert_eq!(
            playwright.conflict_with.as_deref(),
            Some("@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp"),
            "同族已装别的后端须报冲突"
        );

        // ③ 钉版本 + 错配提示（模拟 latest 落后）。
        let web_team = rows.iter().find(|r| r.label_zh.contains("Web 档")).unwrap();
        assert!(web_team.steps[0].spec.ends_with("@0.1.6-alpha.1"));
        assert!(
            web_team.steps[0]
                .version_notice
                .as_deref()
                .is_some_and(|n| n.contains("0.1.5-alpha.2")),
            "latest 落后时必须提示"
        );

        // ④ 互斥族与条目数一致，且无条目被漏掉。
        assert_eq!(rows.len(), CATALOG.len());
        // ⑤ 非互斥条目不得凭空报冲突。
        assert!(rows
            .iter()
            .find(|r| r.label_zh.contains("Auto review"))
            .unwrap()
            .conflict_with
            .is_none());
    }

    /// 运行时版本**未检出**时必须**降级而非拼坏 spec**：退回裸包名，且**必须**给出
    /// 未钉版本的显式告知（沉默会让用户以为已钉好）。
    #[test]
    fn resolve_rows_without_runtime_version_degrades_honestly() {
        let rows = resolve_rows(&InstalledState::default(), None);
        let step = &rows[0].steps[0];
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
}
