//! boot_failure.rs —— 启动失败类型化（ADR-0012，2026-09-08）。
//!
//! 背景（架构评审 P5）：错误卡标题/建议过去由 `classify_boot_error` 对错误文本做
//! 子串匹配得出——**措辞即契约**：上游（dsh/pnpm/node/OS）改一句文案，分类就静默
//! 落到兜底分支，且没有任何测试能发现。
//!
//! 本模块把分类变成 tagged enum（`{"kind":"…"}`）：
//! - 内部知道原因的地方可直接构造变体；
//! - 外部自由文本（唯一无法类型化的来源）经 [`BootFailure::from_legacy_detail`] 兜底；
//! - 新增失败模式会触发编译错误（`match` 穷尽）。
//!
//! 边界（ADR-0012 §2 约束 2）：只服务 boot 失败路径，其余模块的 `Result<_, String>`
//! 本次不动。

use serde::Serialize;

/// **特权类失败的稳定标记**（v1.2.0 D1，2026-09-11）。
///
/// 为什么要一个"机器标记"而不是靠自然语言子串：ADR-0012 的教训是**措辞即契约**——
/// 上游改一句文案，分类就静默落兜底。生产侧（`engines::install_dsh_global`）与本表
/// **共用这一个常量**，两侧无法漂移：改这里 = 两边同时改，改错 = 编译期/测试可见。
///
/// 但它仍是"文本桥"（boot 路径的错误是 `Result<_, String>`，本模块收不到类型化信息）；
/// 真正的类型化直传需要 `emit_boot_error` 改签名，属独立意图，**本轮不动**。
pub(crate) const SYMLINK_PRIVILEGE_MARKER: &str = "[symlink-privilege]";

/// 判定一段（**已小写化**的）错误文本是否为「符号链接特权不足」（Windows 无
/// `SeCreateSymbolicLinkPrivilege` 时 pnpm 的 global hash link 会以 `os error 5` 失败）。
///
/// 判据 = 稳定标记 **或** 明确的 OS 拒绝访问特征。**不**匹配通用权限词
/// （如裸 `permission`）——那会把无关的权限问题也吸进来。
///
/// `pub(crate)` 是给 `engines::classify_install_failure` 复用的：**判据只此一份**，
/// 生产侧与分类侧不会各自维护一套模式（task-40 的教训：重复实现必然只改一处）。
pub(crate) fn is_symlink_privilege_detail(lowered: &str) -> bool {
    lowered.contains(&SYMLINK_PRIVILEGE_MARKER.to_lowercase())
        || lowered.contains("os error 5")
        || lowered.contains("拒绝访问")
        || lowered.contains("access is denied")
        || lowered.contains("requires the following privilege")
        || lowered.contains("secreatesymboliclinkprivilege")
}

/// 启动失败分类。
///
/// **为什么没有 `EngineNotReady`**：ADR-0012 §3A 曾列出该变体，但实施时逐点核对
/// boot 路径的全部错误来源（`boot.rs` 13 处 `emit_boot_error`）后确认——boot 路径
/// 不经 `engines::resolve_toolchain`（其调用方只有 plugins/profiles 的管理动作），
/// 引擎未就绪**在 boot 路径上不存在生产者**。凭空造一个没有生产者的变体违反
/// 「分类必须能穷尽覆盖」的初衷，故删除；ADR-0012 §7 记录该裁定与复审触发。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum BootFailure {
    /// 宿主 dsh 与用户凭据格式不匹配（通常是 dsh 版本过旧）。
    CredentialsMismatch,
    /// 宿主 dsh 不认当前参数（版本不兼容）。
    IncompatibleOptions,
    /// 网络 / registry 不可达（下载类失败）。
    NetworkUnavailable,
    /// **符号链接特权不足**（v1.2.0 D1）：pnpm 12 的 global package hash link 需要
    /// `SeCreateSymbolicLinkPrivilege`，Windows 普通账户没有 ⇒ `os error 5`。
    ///
    /// 与 `NetworkUnavailable` **必须分开**：两者都会在「引导安装」这一步失败，
    /// 但出路完全不同（网络 → 重试/换镜像；特权 → 改运行环境）。此前被合并成
    /// 「registry 均不可达」，用户重试必然再失败。
    SymlinkPrivilegeRequired,
    /// **挂载行加载失败**（2026-09-16 真机事故）：dsh 的插件树在加载某条 `- insert:`
    /// 行时抛错，**整棵树拒绝加载** → 工作台永不就绪（真机：34s 后退出码 1）。
    ///
    /// 与 `Unknown` **必须分开**：这类失败的出路是"去掉那一行"，而不是"重试"——
    /// 重试必然再失败。点名行 id 是本分类存在的全部理由：日志里上游已经把
    /// `failed to apply loader entry <行id> (<包>) : <根因>` 写清楚了，缺的只是
    /// 把它端到用户面前。
    PluginRowFailed {
        /// 出错的行 id（壳写的行恒带 `dsh-dock-` 前缀；bundle 自带行是包名派生 id）。
        row_id: String,
        /// 该行的包名（上游没写时为空串）。
        package: String,
        /// 上游给出的根因（取第一行；可能为空）。
        cause: String,
    },
    /// 兜底：无法类型化的外部文本（原文随载荷的 `detail` 传给前端，不参与分类决策）。
    Unknown { detail: String },
}

/// 从 dsh 输出里解析「哪条挂载行把插件树搞挂了」。
///
/// 上游**两种措辞**都要认（2026-09-16 追加第二种，真机复现抓到的缺口）：
/// ```text
/// failed to apply loader entry <id> (<pkg>): <cause>    ← 条目 apply() 抛错
/// failed to import loader entry <id> (<pkg>): <cause>    ← 模块解析失败（包没装/路径不对）
/// ```
/// 第二种的真实原文（手工写一条 chrome-devtools 行但包已卸载）：
/// ```text
/// dsh: plugin tree failed to load: failed to apply loader entry include (cordis:include):
///   failed to import loader entry dsh-dock--…-chrome-devtools-mcp (@deepseek-ai/…-chrome-devtools-mcp):
///     Cannot find package '@deepseek-ai/…-chrome-devtools-mcp' imported from …/profiles/web/
/// ```
/// **少认一种的代价**：用户看到兜底的「详情见日志 + 重试」，而这恰好是最不该给重试的一类
/// （重试必然再失败）。判据只有这两句上游原文；解析不出 → `None`，由调用方退回文本分类，**不猜**。
const LOADER_ENTRY_MARKS: &[&str] = &[
    "failed to apply loader entry ",
    "failed to import loader entry ",
];

/// 在 `failed to … loader entry ` 之后解析 `<id> (<pkg>): <cause>`；不是行 id 形态则 `None`。
fn parse_loader_entry_after(rest: &str) -> Option<(String, String, String)> {
    let (candidate, tail) = rest.split_once(char::is_whitespace)?;
    // 行 id 形态：壳写的 `dsh-dock-…` 或 bundle 的包名派生 id——都含 `-`；
    // 尾随冒号是无包名括号时的分隔符，去掉。太长的不是行 id。
    let row_id = candidate.trim_end_matches(':');
    if !row_id.contains('-') || row_id.len() > 128 {
        return None;
    }
    let (package, cause) = match tail.trim_start().strip_prefix('(') {
        Some(after) => match after.split_once(')') {
            Some((pkg, tail)) => (
                pkg.to_string(),
                tail.trim_start_matches([':', ' ']).to_string(),
            ),
            None => (String::new(), String::new()),
        },
        None => (
            String::new(),
            tail.trim_start_matches([':', ' ']).to_string(),
        ),
    };
    let cause = cause.lines().next().unwrap_or_default().trim().to_string();
    Some((row_id.to_string(), package, cause))
}

fn parse_failed_loader_entry(text: &str) -> Option<(String, String, String)> {
    for mark in LOADER_ENTRY_MARKS {
        // **逐处扫**：同一段日志里这句会出现多次——先是
        // `failed to apply loader entry include (cordis:include): …`（`include` 是 loader
        // 条目名，不是行 id），其后才轮到真正出错的那一行。只认"长得像行 id"的那一处。
        let mut offset = 0usize;
        while let Some(pos) = text[offset..].find(mark) {
            let start = offset + pos + mark.len();
            offset = start;
            if let Some(hit) = parse_loader_entry_after(&text[start..]) {
                return Some(hit);
            }
        }
    }
    None
}

impl BootFailure {
    /// 外部文本 → 分类（兜底表；只服务无法类型化的来源）。
    ///
    /// 与 2026-09-08 之前的 `classify_boot_error` **有意不完全等价**：裸 `timeout`
    /// 不再单独判为「网络不可用」——本地 socket 超时、用户取消都含该词，判成网络
    /// 会给出错误建议（ADR-0012 §1「同词不同因」）。网络判定收窄为 `network` /
    /// `registry` 两个网络域词，其余落 `Unknown`（标题/建议与旧兜底一致）。
    pub(crate) fn from_legacy_detail(detail: &str) -> Self {
        let d = detail.to_lowercase();
        // **特权判定必须先于网络判定**（v1.2.0 D1）：引导失败的聚合文案里同时含
        // `registry` 与 `os error 5`，若先判网络就会把特权类失败吸走——这正是本次
        // 缺陷的成因。顺序在此是语义的一部分，勿调换。
        if is_symlink_privilege_detail(&d) {
            Self::SymlinkPrivilegeRequired
        } else if d.contains("credentials") || d.contains("must be a string") {
            Self::CredentialsMismatch
        } else if d.contains("unknown option") || d.contains("incompatible") {
            Self::IncompatibleOptions
        } else if d.contains("network") || d.contains("registry") {
            Self::NetworkUnavailable
        } else {
            Self::Unknown {
                detail: detail.to_string(),
            }
        }
    }

    /// 错误卡标题（与旧 `classify_boot_error` 逐字一致）。
    pub(crate) fn title(&self) -> &'static str {
        match self {
            Self::CredentialsMismatch => "宿主 DSH 与您的凭据格式不匹配",
            Self::IncompatibleOptions => "宿主 DSH 参数不兼容",
            Self::NetworkUnavailable => "网络不可用",
            Self::SymlinkPrivilegeRequired => "系统权限不足：无法创建符号链接",
            Self::PluginRowFailed { .. } => "实验插件行导致启动失败",
            Self::Unknown { .. } => "DSH 工作台启动失败",
        }
    }

    /// 错误卡建议（与旧 `classify_boot_error` 逐字一致；新增变体除外）。
    ///
    /// 返回 `String` 而非常量：`PluginRowFailed` 的建议必须**点名行 id**才对用户有用
    /// （"去掉那一行"是唯一出路），而 id 只有运行时才知道。
    pub(crate) fn suggestion(&self) -> String {
        match self {
            Self::CredentialsMismatch => {
                "通常是 DSH 版本过旧：升级到官方最新版可解决（升级只动 pnpm/npm 全局，不碰您的数据）。"
                    .to_string()
            }
            Self::IncompatibleOptions => "请升级您的 DSH 到支持当前终端行为的版本。".to_string(),
            Self::NetworkUnavailable => "实时下载需要网络连接；检查网络后重试。".to_string(),
            // **必须给出路**（v1.2.0 D1）：这条失败重试不会好，且与网络无关——
            // 说清"不是网络问题"能直接掐断用户的错误排查方向。
            Self::SymlinkPrivilegeRequired => {
                "这不是网络问题：引擎需要创建符号链接，而当前 Windows 账户没有该权限。\
                 最省事的办法是改用「WSL」运行环境（在启动页选择，或在控制中心设为默认）——\
                 已装好的 WSL 发行版不需要该权限。若必须用本地模式：以管理员身份运行本应用，\
                 或在「设置 → 系统 → 开发者选项」开启开发者模式后重试。"
                    .to_string()
            }
            // 出路是"去掉那一行"，不是"重试"：把行 id 端到用户面前，并给两条可走的路
            // （界面里关掉该能力；界面进不去时直接删那一行——文件有自动备份）。
            Self::PluginRowFailed {
                row_id,
                package,
                cause,
            } => {
                let pkg = if package.is_empty() {
                    String::new()
                } else {
                    format!("（{package}）")
                };
                let why = if cause.is_empty() {
                    String::new()
                } else {
                    format!(" 上游给的原因：{cause}。")
                };
                format!(
                    "挂载行「{row_id}」{pkg}在加载阶段就失败了，dsh 因此拒绝整棵插件树。{why}\
                     这不是网络问题，重试不会好：请在控制中心 → 实验能力 里关掉或移除该能力后重试；\
                     若界面进不去，可编辑该 profile 的 cordis.patch.yml，删掉 id 为「{row_id}」的那一行\
                     （壳每次覆写前都会自动备份）。"
                )
            }
            Self::Unknown { .. } => "详情见日志；可重试，若持续请反馈。".to_string(),
        }
    }

    /// 可用动作 id（前端做 id → 文案映射，未知 id 回退展示原文）。
    pub(crate) fn actions(&self) -> Vec<&'static str> {
        match self {
            Self::CredentialsMismatch | Self::IncompatibleOptions => vec!["upgrade", "retry"],
            // 特权失败：`retry` 留在列表里（用户按建议开了开发者模式后仍要重试），
            // 但 `boot_in_wsl` 才是真正的出路——它与 D3 的「下次默认打开方式」呼应。
            Self::SymlinkPrivilegeRequired => vec!["boot_in_wsl", "retry"],
            // 插件行失败：**不给 `retry`**——同一行会再次失败，摆一个必然失败的按钮
            // 等于教用户白点一次。前端对空动作集不再渲染按钮区。
            Self::PluginRowFailed { .. } => Vec::new(),
            Self::NetworkUnavailable | Self::Unknown { .. } => vec!["retry"],
        }
    }
}

/// `boot:error` 事件与 `get_boot_status` 补水共用的载荷。
///
/// `failure` 是结构化分类（新增，前端按 `kind` 取本地化文案）；`title`/`suggestion`
/// 保留为后端文案（旧载荷与未识别 kind 的兼容分支，ADR-0012 §5 负面后果已登记）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BootErrorPayload {
    pub failure: BootFailure,
    pub title: &'static str,
    pub detail: String,
    /// 后端文案（`String`：插件行失败的建议要点名具体行 id，运行时才知道）。
    pub suggestion: String,
    pub actions: Vec<&'static str>,
    pub log: String,
    /// 可一键隔离的挂载行（2026-09-16）：`Some` 时前端渲染「移除该行并重启」。
    ///
    /// **为什么必须给按钮而不是只给文案**：这类失败的出路只有"去掉那一行"，而用户
    /// 此时正卡在启动页——控制中心能不能开是另一回事。文案指路是"告诉他去别处修"，
    /// 按钮才是"就地修好"。只对**壳自己写的行**（`dsh-dock-` 前缀）下发：那是所有权
    /// 标记，bundle 自带行与用户手写行不归壳处置。
    pub quarantine: Option<QuarantineRow>,
}

/// 可一键隔离的挂载行（`profile` + 行 id）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QuarantineRow {
    pub profile: String,
    pub row_id: String,
}

impl BootErrorPayload {
    /// 由「原始错误文本 + 日志尾部」构造。
    ///
    /// 分类顺序**是语义的一部分**：先看 dsh 输出里有没有"哪条挂载行把插件树搞挂了"
    /// （2026-09-16 真机事故；上游原文里行 id 与根因都在，端到端可行动），解析不出
    /// 才退到 [`BootFailure::from_legacy_detail`] 的子串兜底表。
    pub(crate) fn classify(detail: &str, log_tail: &str) -> Self {
        if let Some(failure) = Self::plugin_row_failure(detail, log_tail) {
            return Self::from_failure(failure, detail, log_tail);
        }
        Self::from_failure(BootFailure::from_legacy_detail(detail), detail, log_tail)
    }

    /// 挂载行失败判定：在**日志尾部与错误摘要**两处找上游那句
    /// `failed to apply loader entry <行id> (<包>): <根因>`。
    ///
    /// 为什么两处都找：`detail` 只取日志首条 Error 行（那是顶层的 `plugin tree failed
    /// to load`），行 id 在其后的 AggregateError 里；而极端情况下日志被截断时反过来
    /// 只剩 detail。两处取先命中者，判据仍是同一份上游原文。
    fn plugin_row_failure(detail: &str, log_tail: &str) -> Option<BootFailure> {
        let (row_id, package, cause) =
            parse_failed_loader_entry(log_tail).or_else(|| parse_failed_loader_entry(detail))?;
        Some(BootFailure::PluginRowFailed {
            row_id,
            package,
            cause,
        })
    }

    /// 由已确定的分类构造（内部知道原因时直连，不经文本匹配）。
    pub(crate) fn from_failure(failure: BootFailure, detail: &str, log_tail: &str) -> Self {
        Self {
            title: failure.title(),
            suggestion: failure.suggestion(),
            actions: failure.actions(),
            failure,
            detail: detail.to_string(),
            log: log_tail.to_string(),
            quarantine: None,
        }
    }

    /// 补上"就地修好"的出口：出错行是**壳自己写的**（`dsh-dock-` 前缀）时，下发
    /// 一键隔离（移除该行 + 重启）。`profile` 空（拿不到会话目标）时保持只读诊断——
    /// 宁可不给按钮，也不给一个会删错 profile 的按钮。
    ///
    /// 只对 [`BootFailure::PluginRowFailed`] 生效：其它失败没有"某一行"可删。
    pub(crate) fn with_quarantine(mut self, profile: Option<&str>) -> Self {
        let BootFailure::PluginRowFailed { row_id, .. } = &self.failure else {
            return self;
        };
        let Some(profile) = profile.filter(|p| !p.trim().is_empty()) else {
            return self;
        };
        if !crate::plugins::is_shell_row_id(row_id) {
            return self;
        }
        self.actions = vec!["quarantine_plugin_row"];
        self.quarantine = Some(QuarantineRow {
            profile: profile.to_string(),
            row_id: row_id.clone(),
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 旧 `classify_boot_error` 的四条子串规则 → 变体映射（逐条等价，`timeout` 例外）。
    #[test]
    fn legacy_detail_maps_to_variants() {
        assert_eq!(
            BootFailure::from_legacy_detail(
                "credentials-local: the value for \"version\" must be a string"
            ),
            BootFailure::CredentialsMismatch
        );
        assert_eq!(
            BootFailure::from_legacy_detail("error: unknown option '--no-open'"),
            BootFailure::IncompatibleOptions
        );
        assert_eq!(
            BootFailure::from_legacy_detail("registry 不可达：network timeout"),
            BootFailure::NetworkUnavailable
        );
        assert!(matches!(
            BootFailure::from_legacy_detail("some weird crash"),
            BootFailure::Unknown { .. }
        ));
    }

    /// 「同词不同因」反例（ADR-0012 §5 行动项）：本地 socket 超时不得判成网络。
    #[test]
    fn local_socket_timeout_is_not_network() {
        for detail in [
            "connect ETIMEDOUT 127.0.0.1:57746",
            "Error: timeout while waiting for local socket",
            "用户取消（timeout）",
        ] {
            assert!(
                !matches!(
                    BootFailure::from_legacy_detail(detail),
                    BootFailure::NetworkUnavailable
                ),
                "本地超时不应判成网络不可用：{detail}"
            );
        }
        // 网络域词仍应命中（否则收窄过头）
        assert_eq!(
            BootFailure::from_legacy_detail("network unreachable"),
            BootFailure::NetworkUnavailable
        );
    }

    /// **复现锚（v1.2.0 D1，2026-09-11）**：Windows 普通账户下 pnpm 的
    /// 「global package hash link」建符号链接失败（`os error 5` / `拒绝访问`），
    /// 属**特权类**失败，**不得**被判成「网络不可用」。
    ///
    /// 现状缺陷：`engines::install_dsh_global` 把两条 registry 的失败合并成
    /// 「dsh 引导安装失败（**registry 均不可达**）：……」——文案里带 `registry`
    /// ⇒ 兜底表命中网络分支 ⇒ 错误卡给「检查网络后重试」。
    /// **用户拿到的建议与真实原因（权限）无关**，重试必然再失败。
    ///
    /// 本断言在修复前**红**（正是「复现先行」要求的那条红）。
    #[test]
    fn privilege_failure_is_not_classified_as_network() {
        for detail in [
            // ① 图原文（截图①：pnpm 12 + Windows 普通账户）
            "dsh 引导安装失败（registry 均不可达）：npmmirror: tls handshake eof；\
             npmjs.org: link the global package install directory at \
             C:\\Users\\x\\engines\\global\\v11\\db8a\\2082 -> 拒绝访问。(os error 5)",
            "failed to create symlink: Access is denied. (os error 5)",
            "os error 5 拒绝访问",
        ] {
            assert!(
                !matches!(
                    BootFailure::from_legacy_detail(detail),
                    BootFailure::NetworkUnavailable
                ),
                "特权类失败被误判成网络不可用（用户会拿到『检查网络后重试』这种\
                 与真因无关的建议）：{detail}"
            );
        }
    }

    /// **v1.2.0 D1**：特权变体的文案必须「掐断错误方向 + 给出路」。
    #[test]
    fn symlink_privilege_variant_gives_actionable_way_out() {
        let f = BootFailure::SymlinkPrivilegeRequired;
        // 标题点明真因（不是网络）
        assert!(f.title().contains("权限"), "标题应点明权限：{}", f.title());
        // 建议：① 明确否定网络方向；② 给出 WSL 出路；③ 给出本地模式的两条自助路径
        let s = f.suggestion();
        assert!(s.contains("不是网络问题"), "必须掐断错误排查方向：{s}");
        assert!(s.contains("WSL"), "必须给出可行动出路（WSL 模式）：{s}");
        assert!(
            s.contains("管理员") && s.contains("开发者模式"),
            "本地模式须给两条自助路径：{s}"
        );
        // 动作：出路优先，retry 保留（用户照做后仍要重试）
        assert_eq!(f.actions(), vec!["boot_in_wsl", "retry"]);
        // 序列化 kind（前端据此取本地化文案）
        let v = serde_json::to_value(&f).unwrap();
        assert_eq!(v["kind"], "symlink_privilege_required");
    }

    /// 稳定标记是**两侧共用**的常量：生产侧拼进去的标记，本表必须认得。
    #[test]
    fn marker_const_is_recognized_by_classifier() {
        let msg = format!("{SYMLINK_PRIVILEGE_MARKER}引擎安装需要创建符号链接…");
        assert_eq!(
            BootFailure::from_legacy_detail(&msg),
            BootFailure::SymlinkPrivilegeRequired
        );
    }

    /// 顺序判据的**回归保护**：含 `registry` 字样的特权文本必须先命中特权分支。
    #[test]
    fn privilege_check_precedes_network_check() {
        let mixed = "registry 均不可达：npmjs.org: -> 拒绝访问。(os error 5)";
        assert_eq!(
            BootFailure::from_legacy_detail(mixed),
            BootFailure::SymlinkPrivilegeRequired,
            "判定顺序被调换——含 registry 的特权文本会被网络分支吸走（本次缺陷成因）"
        );
    }

    /// 兜底变体携带原文，且原文不影响分类（只用于展示）。
    #[test]
    fn unknown_carries_detail_verbatim() {
        let raw = "DSH 进程已退出（代码 1）Error: boom";
        match BootFailure::from_legacy_detail(raw) {
            BootFailure::Unknown { detail } => assert_eq!(detail, raw),
            other => panic!("应落兜底，实际：{other:?}"),
        }
    }

    /// 标题/建议/动作与旧实现逐字一致（避免「类型化」之名悄悄改文案）。
    #[test]
    fn copy_matches_legacy_text() {
        assert_eq!(
            BootFailure::CredentialsMismatch.title(),
            "宿主 DSH 与您的凭据格式不匹配"
        );
        assert_eq!(BootFailure::NetworkUnavailable.title(), "网络不可用");
        assert_eq!(
            BootFailure::Unknown {
                detail: String::new()
            }
            .title(),
            "DSH 工作台启动失败"
        );
        assert_eq!(
            BootFailure::CredentialsMismatch.actions(),
            vec!["upgrade", "retry"]
        );
        assert_eq!(BootFailure::NetworkUnavailable.actions(), vec!["retry"]);
    }

    /// 载荷形状：`failure` 为 tagged enum（`kind` 判别式），其余字段与旧载荷一致。
    #[test]
    fn payload_serializes_tagged_kind() {
        let payload = BootErrorPayload::classify("network unreachable", "tail");
        let v = serde_json::to_value(&payload).unwrap();
        assert_eq!(v["failure"]["kind"], "network_unavailable");
        assert_eq!(v["title"], "网络不可用");
        assert_eq!(v["detail"], "network unreachable");
        assert_eq!(v["actions"], serde_json::json!(["retry"]));
        assert_eq!(v["log"], "tail");

        let unknown = serde_json::to_value(BootErrorPayload::classify("boom", "")).unwrap();
        assert_eq!(unknown["failure"]["kind"], "unknown");
        assert_eq!(unknown["failure"]["detail"], "boom");
    }

    /// 真机日志尾部（2026-09-16 事故原文节选）→ 必须点名**出错的那一行**。
    ///
    /// 这条是本次事故的回归：旧实现分类必然落 `unknown`（措辞表里没有插件树一词），
    /// 于是错误卡只能显示"详情见日志"；而当时日志还是空的。
    const REAL_FAILURE_TAIL: &str = "\
Error: dsh: plugin tree failed to load: failed to apply loader entry include (cordis:include): loader entries failed to apply\n\
Error: failed to apply loader entry dsh-dock--deepseek-ai-dsh-experimental-computer-use-cua-driver-mcp (@deepseek-ai/dsh-experimental-computer-use-cua-driver-mcp): mcp-client(cua-driver-mcp): initial connection or tool synchronization failed\n\
Error: spawn cua-driver ENOENT\n";

    #[test]
    fn plugin_tree_failure_names_the_offending_row() {
        let payload = BootErrorPayload::classify(
            "dsh: plugin tree failed to load: failed to apply loader entry include (cordis:include)",
            REAL_FAILURE_TAIL,
        );
        match &payload.failure {
            BootFailure::PluginRowFailed {
                row_id,
                package,
                cause,
            } => {
                assert_eq!(
                    row_id,
                    "dsh-dock--deepseek-ai-dsh-experimental-computer-use-cua-driver-mcp"
                );
                assert_eq!(
                    package,
                    "@deepseek-ai/dsh-experimental-computer-use-cua-driver-mcp"
                );
                assert!(cause.contains("initial connection"), "根因：{cause}");
            }
            other => panic!("必须点名挂载行，得到 {other:?}"),
        }
        assert_eq!(payload.title, "实验插件行导致启动失败");
        // 建议必须同时给出**行 id**与出路（点不到名就等于没说）。
        assert!(
            payload.suggestion.contains("dsh-dock--") && payload.suggestion.contains("实验能力"),
            "建议须点明行并给出去路：{}",
            payload.suggestion
        );
        // 这类失败重试必然再失败：不得给 `retry` 按钮（除非换成"隔离后重启"）。
        assert!(
            !payload.actions.contains(&"retry"),
            "不得给必然失败的 retry：{:?}",
            payload.actions
        );
    }

    /// 壳自己写的行 → 下发一键隔离（移除该行 + 重启）；别人的行 → 只读诊断。
    #[test]
    fn quarantine_action_only_for_shell_owned_rows() {
        let mine = BootErrorPayload::classify("plugin tree failed to load", REAL_FAILURE_TAIL)
            .with_quarantine(Some("web"));
        assert_eq!(mine.actions, vec!["quarantine_plugin_row"]);
        let plan = mine.quarantine.expect("壳自己的行必须能一键隔离");
        assert_eq!(plan.profile, "web");
        assert!(crate::plugins::is_shell_row_id(&plan.row_id));

        // vendor 自带的 bundle 行（无 `dsh-dock-` 前缀）：壳无权删，不得下发动作。
        let foreign = BootErrorPayload::classify(
            "plugin tree failed to load",
            "Error: failed to apply loader entry agent-team-profile (@deepseek-ai/dsh-experimental-agent-team-profile): invalid plugin\n",
        )
        .with_quarantine(Some("web"));
        assert!(foreign.quarantine.is_none(), "vendor 行不得给删除按钮");
        assert!(!foreign.actions.contains(&"quarantine_plugin_row"));

        // 拿不到会话目标 profile → 不给按钮（宁可不给，也不能删错 profile）。
        let no_profile =
            BootErrorPayload::classify("plugin tree failed to load", REAL_FAILURE_TAIL)
                .with_quarantine(None);
        assert!(no_profile.quarantine.is_none());

        // 非插件行失败：即使给了 profile 也不得长出隔离动作。
        let other =
            BootErrorPayload::classify("network unreachable", "tail").with_quarantine(Some("web"));
        assert!(other.quarantine.is_none());
        assert_eq!(other.actions, vec!["retry"]);
    }

    /// 解析器不得被杂讯骗到：没有上游那句原文时**不猜**（宁可退回 unknown）。
    #[test]
    fn loader_entry_parser_does_not_guess() {
        assert!(parse_failed_loader_entry("no such phrase here").is_none());
        assert!(parse_failed_loader_entry("failed to apply loader entry ").is_none());
        // 没有包名括号（措辞变了）→ 仍能拿到行 id，但包名为空，不编造。
        let (row, pkg, cause) = parse_failed_loader_entry(
            "Error: failed to apply loader entry dsh-dock-x: boom\n第二行不应被吃进来",
        )
        .expect("行 id 形态合法即应解析");
        assert_eq!(row, "dsh-dock-x");
        assert!(pkg.is_empty());
        assert_eq!(cause, "boom");
    }

    /// **真机回归**（2026-09-16 用户验收抓到）：模块解析失败用的是
    /// `failed to import loader entry`（不是 `apply`）。只认 `apply` 会让这类失败落
    /// `unknown` 兜底 → 标题变「DSH 工作台启动失败」、建议变「详情见日志」、还摆一个
    /// **必然再失败**的「重试」——正是最不该给重试的一类。原文取自 dev home 的
    /// `dsh-shell.log`（手工写行但包已卸载：`ERR_MODULE_NOT_FOUND`）。
    const REAL_IMPORT_FAILURE: &str = "\
Error: dsh: plugin tree failed to load: failed to apply loader entry include (cordis:include): failed to import loader entry dsh-dock--deepseek-ai-dsh-experimental-browser-use-chrome-devtools-mcp (@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp): Cannot find package '@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp' imported from /Users/x/.dsh-dock-dev/profiles/web/\n\
Error [ERR_MODULE_NOT_FOUND]: Cannot find package '@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp' imported from /Users/x/.dsh-dock-dev/profiles/web/\n";

    #[test]
    fn import_wording_is_classified_and_names_the_row() {
        let payload = BootErrorPayload::classify(
            "dsh: plugin tree failed to load: failed to apply loader entry include (cordis:include)",
            REAL_IMPORT_FAILURE,
        );
        match &payload.failure {
            BootFailure::PluginRowFailed {
                row_id,
                package,
                cause,
            } => {
                assert_eq!(
                    row_id,
                    "dsh-dock--deepseek-ai-dsh-experimental-browser-use-chrome-devtools-mcp"
                );
                assert_eq!(
                    package,
                    "@deepseek-ai/dsh-experimental-browser-use-chrome-devtools-mcp"
                );
                assert!(cause.contains("Cannot find package"), "根因：{cause}");
            }
            other => panic!("import 措辞也必须点名挂载行，得到 {other:?}"),
        }
        assert_eq!(payload.title, "实验插件行导致启动失败");
        assert!(!payload.actions.contains(&"retry"), "不得给必然失败的重试");
        // 壳自有行 → 必须给出一键隔离（用户点一下就能回到可用状态）。
        let with_quarantine = payload.with_quarantine(Some("web"));
        let plan = with_quarantine.quarantine.expect("应下发一键隔离计划");
        assert_eq!(plan.profile, "web");
        assert!(crate::plugins::is_shell_row_id(&plan.row_id));
    }
}
