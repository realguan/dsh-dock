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
    /// 兜底：无法类型化的外部文本（原文随载荷的 `detail` 传给前端，不参与分类决策）。
    Unknown { detail: String },
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
            Self::Unknown { .. } => "DSH 工作台启动失败",
        }
    }

    /// 错误卡建议（与旧 `classify_boot_error` 逐字一致；新增变体除外）。
    pub(crate) fn suggestion(&self) -> &'static str {
        match self {
            Self::CredentialsMismatch => {
                "通常是 DSH 版本过旧：升级到官方最新版可解决（升级只动 pnpm/npm 全局，不碰您的数据）。"
            }
            Self::IncompatibleOptions => "请升级您的 DSH 到支持当前终端行为的版本。",
            Self::NetworkUnavailable => "实时下载需要网络连接；检查网络后重试。",
            // **必须给出路**（v1.2.0 D1）：这条失败重试不会好，且与网络无关——
            // 说清"不是网络问题"能直接掐断用户的错误排查方向。
            Self::SymlinkPrivilegeRequired => {
                "这不是网络问题：引擎需要创建符号链接，而当前 Windows 账户没有该权限。\
                 最省事的办法是改用「WSL」运行环境（在启动页选择，或在控制中心设为默认）——\
                 已装好的 WSL 发行版不需要该权限。若必须用本地模式：以管理员身份运行本应用，\
                 或在「设置 → 系统 → 开发者选项」开启开发者模式后重试。"
            }
            Self::Unknown { .. } => "详情见日志；可重试，若持续请反馈。",
        }
    }

    /// 可用动作 id（前端做 id → 文案映射，未知 id 回退展示原文）。
    pub(crate) fn actions(&self) -> Vec<&'static str> {
        match self {
            Self::CredentialsMismatch | Self::IncompatibleOptions => vec!["upgrade", "retry"],
            // 特权失败：`retry` 留在列表里（用户按建议开了开发者模式后仍要重试），
            // 但 `boot_in_wsl` 才是真正的出路——它与 D3 的「下次默认打开方式」呼应。
            Self::SymlinkPrivilegeRequired => vec!["boot_in_wsl", "retry"],
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
    pub suggestion: &'static str,
    pub actions: Vec<&'static str>,
    pub log: String,
}

impl BootErrorPayload {
    /// 由「原始错误文本 + 日志尾部」构造（分类走兜底表）。
    pub(crate) fn classify(detail: &str, log_tail: &str) -> Self {
        Self::from_failure(BootFailure::from_legacy_detail(detail), detail, log_tail)
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
        }
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
}
