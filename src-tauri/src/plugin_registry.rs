//! plugin_registry.rs —— 插件安装的**源选择策略**（2026-09-16，ADR-0006 §6）。
//!
//! ## 为什么单独成模块
//!
//! 真机故障：`@deepseek-ai/dsh-experimental-*` 的**部分 provider 在镜像上没同步**（404），
//! 而 `~/.npmrc` 把源指向了镜像 → 安装必然失败；镜像可达但缺包、官方可达但可能慢/不通，
//! 两侧各有各的缺。维护者裁定：「对于有条件的环境，直接走官方源，没有条件的环境走 npmmirror。」
//!
//! 本模块只做**纯决策**：先试哪个源、失败了换不换、什么失败才值得换。三件事都不碰 IO：
//! - 把结论变成 pnpm 参数 → `plugins.rs::mutate_plugin_blocking(registry:)`；
//! - 把结论记住 → `settings.rs` 的 `pluginRegistry` / `pluginRegistryLastGood`；
//! - 把"用了哪个源"如实说给用户 → `PluginOpOutcome.detail`。
//!
//! ## 分类只有这一处（禁双源）
//!
//! `FailureClass` 是**唯一**的失败分类实现：后端用它决定换不换源，前端**消费**
//! `PluginOpOutcome.failureKind` 决定显示哪句话。前端不得再写一份正则分类——
//! 两份分类会在下一次改动里漂移，而漂移的表现是"后端换源了、前端说不是网络问题"。

use serde::{Deserialize, Serialize};

/// 官方源。镜像由**用户自己的 npm 配置**决定（本机 `~/.npmrc` → `registry.npmmirror.com`）：
/// 壳不写死"哪个是镜像"——私有源、企业源、地区镜像都该被尊重。
pub const OFFICIAL_REGISTRY: &str = "https://registry.npmjs.org";

/// 一次安装实际使用的源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistrySource {
    /// 官方源：显式传 `--registry https://registry.npmjs.org`。
    Official,
    /// 用户配置的源：**不传** `--registry`，沿用 pnpm 的配置（`~/.npmrc` / profile `.npmrc`）。
    Configured,
}

impl RegistrySource {
    /// 传给 pnpm 的 `--registry` 取值；`Configured` = `None`（不传，用用户配置）。
    pub fn registry_arg(self) -> Option<&'static str> {
        match self {
            RegistrySource::Official => Some(OFFICIAL_REGISTRY),
            RegistrySource::Configured => None,
        }
    }

    /// 另一个源（双向兜底用：官方不可达 → 换配置源；镜像缺包 → 换官方）。
    pub fn other(self) -> Self {
        match self {
            RegistrySource::Official => RegistrySource::Configured,
            RegistrySource::Configured => RegistrySource::Official,
        }
    }

    /// 持久化键值（`settings.json`）。
    pub fn as_key(self) -> &'static str {
        match self {
            RegistrySource::Official => "official",
            RegistrySource::Configured => "configured",
        }
    }

    /// 解析持久化键值；不认识的一律 `None`（**失效值不消费**，回落到自动策略）。
    pub fn from_key(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "official" => Some(RegistrySource::Official),
            // `mirror` 是同日早先草案里的别名，保留解析以免旧值变哑值。
            "configured" | "mirror" => Some(RegistrySource::Configured),
            _ => None,
        }
    }

    /// 展示用中文名（写进 `detail`，用户要能看出用的是哪个源）。
    pub fn label_zh(self) -> &'static str {
        match self {
            RegistrySource::Official => "官方源",
            RegistrySource::Configured => "本机配置的源",
        }
    }
}

/// 用户在偏好里选了什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryPref {
    /// 自动：先按记忆试，失败换另一个源重试一次。
    Auto,
    /// 只用指定源，**不换源**（用户显式覆盖，壳不得擅自改）。
    Only(RegistrySource),
}

/// 解析偏好（`None` / 空 / 不认识的取值 → `Auto`）。
pub fn parse_pref(raw: Option<&str>) -> RegistryPref {
    match raw.and_then(RegistrySource::from_key) {
        Some(source) => RegistryPref::Only(source),
        None => RegistryPref::Auto,
    }
}

/// 失败分类：决定"换源有没有意义"，也是给用户看的那句话的依据。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureClass {
    /// 网络层：TLS/连接/超时/metadata 取不回来 —— 换源有意义。
    Network,
    /// 包或版本不存在（含镜像未同步）—— 换源有意义。
    NotFound,
    /// pnpm 构建脚本审批门 —— 换源也白搭（同一份 approval 配置），且会掩盖真因。
    BuildApproval,
    /// 其它（spec 非法、profile 未初始化…）—— 不换源。
    Other,
}

impl FailureClass {
    /// 这次失败值不值得换另一个源重试（ADR-0006 §6.3）。
    pub fn worth_switching_registry(self) -> bool {
        matches!(self, FailureClass::Network | FailureClass::NotFound)
    }
}

/// 把 pnpm/dsh 的输出分类（**纯函数**，分类的唯一实现，见模块头）。
///
/// 顺序有讲究：先判"包不存在"再判网络——镜像未同步时，pnpm 常以
/// `Failed to fetch metadata` 的面目报出来，只有 404 字样能把它和"连不上"分开；
/// 而两者的处理虽然都是换源，对用户说的话不同。
pub fn classify_failure(detail: &str) -> FailureClass {
    let d = detail.to_ascii_lowercase();
    if d.contains("err_pnpm_ignored_builds") || d.contains("allowbuilds") {
        return FailureClass::BuildApproval;
    }
    if d.contains("404") || d.contains("not found") || d.contains("e404") {
        return FailureClass::NotFound;
    }
    if d.contains("tls handshake")
        || d.contains("client error (connect)")
        || d.contains("failed to fetch metadata")
        || d.contains("econnreset")
        || d.contains("etimedout")
        || d.contains("timed out")
        || d.contains("超时")
        || d.contains("network")
        || d.contains("enotfound")
    {
        return FailureClass::Network;
    }
    FailureClass::Other
}

/// 第一轮试哪个源。
///
/// 顺序：**显式偏好** → **记忆**（上次成功过的源）→ **官方**（维护者 2026-09-16 裁定：
/// 「有条件的环境直接走官方源」；无记忆时官方优先，失败即换配置源）。
pub fn first_source(pref: RegistryPref, last_good: Option<RegistrySource>) -> RegistrySource {
    match pref {
        RegistryPref::Only(source) => source,
        RegistryPref::Auto => last_good.unwrap_or(RegistrySource::Official),
    }
}

/// 失败后**下一个**要试的源；`None` = 不换（只试这一个源就报错）。
pub fn next_source(
    pref: RegistryPref,
    attempted: RegistrySource,
    detail: &str,
) -> Option<RegistrySource> {
    match pref {
        // 显式指定源：壳不得擅自换（用户选择优先，换源会是"偷偷改了他的配置"）。
        RegistryPref::Only(_) => None,
        RegistryPref::Auto => classify_failure(detail)
            .worth_switching_registry()
            .then(|| attempted.other()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pref_parses_known_values_and_falls_back_to_auto() {
        assert_eq!(
            parse_pref(Some("official")),
            RegistryPref::Only(RegistrySource::Official)
        );
        assert_eq!(
            parse_pref(Some("configured")),
            RegistryPref::Only(RegistrySource::Configured)
        );
        // 同日草案里的旧别名仍能解析（否则旧值变哑值：用户设了却不生效）。
        assert_eq!(
            parse_pref(Some("mirror")),
            RegistryPref::Only(RegistrySource::Configured)
        );
        for raw in [None, Some(""), Some("auto"), Some("nonsense")] {
            assert_eq!(parse_pref(raw), RegistryPref::Auto, "{raw:?} 应回落自动");
        }
        assert_eq!(
            RegistrySource::from_key("OFFICIAL"),
            Some(RegistrySource::Official)
        );
    }

    /// 官方优先、有记忆按记忆——维护者裁定的核心一句。
    #[test]
    fn auto_prefers_official_then_memory() {
        assert_eq!(
            first_source(RegistryPref::Auto, None),
            RegistrySource::Official,
            "无记忆时先官方（「有条件的环境直接走官方源」）"
        );
        assert_eq!(
            first_source(RegistryPref::Auto, Some(RegistrySource::Configured)),
            RegistrySource::Configured,
            "有记忆时直接命中上次可用的源，不再先失败一次"
        );
        assert_eq!(
            first_source(
                RegistryPref::Only(RegistrySource::Configured),
                Some(RegistrySource::Official)
            ),
            RegistrySource::Configured,
            "显式偏好优先于记忆"
        );
    }

    /// 双向兜底：两个源互为替补（既有镜像链只覆盖了"镜像→官方"一个方向）。
    #[test]
    fn fallback_is_bidirectional() {
        let net = "Failed to fetch metadata from https://registry.npmjs.org/x: client error (Connect): tls handshake eof";
        assert_eq!(
            next_source(RegistryPref::Auto, RegistrySource::Official, net),
            Some(RegistrySource::Configured),
            "官方不可达 → 换配置源"
        );
        let missing =
            "Failed to fetch metadata from https://registry.npmmirror.com/x: 404 Not Found";
        assert_eq!(
            next_source(RegistryPref::Auto, RegistrySource::Configured, missing),
            Some(RegistrySource::Official),
            "镜像缺包 → 换官方"
        );
    }

    /// 显式偏好下**绝不**自动换源（换源等于偷偷改用户的配置）。
    #[test]
    fn explicit_pref_never_switches() {
        for detail in [
            "404 not found",
            "tls handshake eof",
            "ERR_PNPM_IGNORED_BUILDS",
        ] {
            assert_eq!(
                next_source(
                    RegistryPref::Only(RegistrySource::Official),
                    RegistrySource::Official,
                    detail
                ),
                None,
                "{detail}"
            );
        }
    }

    /// 换源无意义的失败不得换（否则白等一轮，还会把真因埋进两条错误里）。
    #[test]
    fn only_network_and_missing_worth_switching() {
        assert!(FailureClass::Network.worth_switching_registry());
        assert!(FailureClass::NotFound.worth_switching_registry());
        assert!(!FailureClass::BuildApproval.worth_switching_registry());
        assert!(!FailureClass::Other.worth_switching_registry());

        let approval =
            "ERR_PNPM_IGNORED_BUILDS  Ignored build scripts: esbuild. Run pnpm approve-builds";
        assert_eq!(classify_failure(approval), FailureClass::BuildApproval);
        assert_eq!(
            next_source(RegistryPref::Auto, RegistrySource::Official, approval),
            None
        );
        assert_eq!(
            next_source(
                RegistryPref::Auto,
                RegistrySource::Official,
                "spec 非法：以 - 开头"
            ),
            None
        );
        // 超时也算网络（dsh 侧 10 分钟超时的文案里带"超时"二字）。
        assert_eq!(
            classify_failure("安装超时（10 分钟）已终止：网络或 registry 不可达时常见"),
            FailureClass::Network
        );
    }

    /// `Configured` 不传 `--registry`：尊重用户自己的源（含私有源），壳不改配置。
    #[test]
    fn configured_source_passes_no_registry_flag() {
        assert_eq!(
            RegistrySource::Official.registry_arg(),
            Some(OFFICIAL_REGISTRY)
        );
        assert_eq!(RegistrySource::Configured.registry_arg(), None);
        assert_eq!(RegistrySource::Configured.other(), RegistrySource::Official);
        assert_eq!(
            RegistrySource::from_key(RegistrySource::Configured.as_key()),
            Some(RegistrySource::Configured)
        );
    }
}
