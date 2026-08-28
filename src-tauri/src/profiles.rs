//! profiles.rs —— Profile 管理器 · 只读能力（4.3 第二刀，2026-08-28）。
//!
//! 职责：扫描 `<dsh_home>/profiles/` 列出全部 profile（已物化 + 内置模板名两态
//! 合并展示）、读取单个 profile 详情（package.json 关键字段 + cordis.patch.yml
//! 原文）。纯读：零写入、零 dsh 子进程、零网络（AGENTS §7）。
//!
//! 行为复现锚定（dsh v0.1.1-rc.2，`dsh-app-boot/lib/index.js`，2026-08-28 核对；
//! 已入 `docs/contracts/dsh-behavior-ledger.md` §一 复现点 6/8）：
//! - 非法名校验逐字复刻 `resolveProfileDir` @ 318：拒绝 空名 / 含 `/` / 含 `\` /
//!   `.` / `..` / 字面量 `node_modules`——其余一律合法（dsh 不拒绝点开头、空格、
//!   Unicode 名，勿自行加码）；
//! - 内置模板名与 bundle 列表复刻 `PROFILE_TEMPLATES` @ 323（web/headless 首次
//!   使用才物化，此前目录不存在——列表须把未物化模板名一并给出）；
//! - profile 目录布局锚定 `initProfile` @ 353：package.json（`name` 约定
//!   `dsh-profile-<目录名>`、`dsh.profile.bundles`、`dependencies`）+
//!   cordis.patch.yml + pnpm-workspace.yaml；node_modules 由 pnpm 生成。
//!
//! ⚠️ 不可复用 `resolve::list_web_ui_profiles`（resolve.rs）：那是 webUi 选择器
//! 原型，无条件注入 `"web"` 并跳过同名目录，与管理器「全量列出 + 两态区分」
//! 语义不同（ADR-0009 §3 方案 E 评审裁定）。
//!
//! home 解析复用壳既有链 `resolve::user_dsh_home()`（$DSH_HOME 环境变量 →
//! `~/.dsh`），与壳 spawn dsh 时注入的 DSH_HOME 同源；不能自行读环境变量——
//! dsh 侧解析优先级是 显式配置 > 环境变量 > `~/.dsh`（dsh-home-paths @ 73），
//! 壳以 env → 默认 为其可见范围。管理器仅覆盖壳侧本地 home，WSL 客体内
//! profile 明确范围外（handoff-4.3-readonly §4.4）。

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// 内置模板名及其 bundle 列表（逐字复刻 dsh `PROFILE_TEMPLATES` @ 323，键序一致）。
/// 未物化时列表页以此展示「首次启动将得到什么」。
const PROFILE_TEMPLATES: &[(&str, &[&str])] = &[
    (
        "web",
        &["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-web-app"],
    ),
    (
        "headless",
        &["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-headless"],
    ),
];

/// 三件套中的用户 patch 层文件名（dsh `PROFILE_PATCH_FILENAME` @ 311）。
const PROFILE_PATCH_FILENAME: &str = "cordis.patch.yml";

/// profile 名校验：与 dsh `resolveProfileDir` @ 318 逐字一致（复现点 8）。
/// 拒绝：空名 / 含 `/` / 含 `\` / `.` / `..` / 字面量 `node_modules`。
/// 名字会直接拼进文件路径（`profiles/<名>`），前端传值不可信——详情/创建/
/// 重命名等一切按名定位的动作都必须先过这里（防路径遍历）。
pub fn validate_profile_name(name: &str) -> Result<(), String> {
    let invalid = name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name == "."
        || name == ".."
        || name == "node_modules";
    if invalid {
        return Err(format!(
            "非法 profile 名「{name}」：dsh 只拒绝空名、含 / 或 \\、.、..、node_modules，其余名字均可用"
        ));
    }
    Ok(())
}

/// 列表条目：已物化 profile 或未物化的内置模板名（两态合并，ADR-0009 方案 E）。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ProfileSummary {
    pub name: String,
    /// true = 已物化（`profiles/<名>/` 目录存在）；false = 内置模板名可首启
    /// （首次启动/首次 plugin add 才物化）。
    pub materialized: bool,
    /// `dsh.profile.bundles`（物化但清单缺失/损坏时为空；未物化模板名 = dsh
    /// 内置模板 bundle 列表）。
    pub bundles: Vec<String>,
    /// package.json `dependencies` 的包名（字典序；仅物化且清单可读时非空）。
    pub dependencies: Vec<String>,
}

/// 扫描 `<home>/profiles/`：每个子目录 = 一个已物化 profile（目录名是唯一硬
/// 身份，Spike B §2.1——半初始化目录也占名，照列、字段置空），再加未物化的
/// 内置模板名（web/headless）。排序：已物化在前，各组内按名字典序。
/// 纯函数：home 由调用方传入（IPC 层用 `resolve::user_dsh_home()`）。
pub fn scan_profiles(home: &Path) -> Vec<ProfileSummary> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(home.join("profiles")) {
        for entry in entries.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }
            let Some(name) = dir.file_name().map(|s| s.to_string_lossy().into_owned()) else {
                continue;
            };
            // node_modules = 跨 profile 符号链接农场（dsh healProfilesModuleFallback
            // @ 409 维护），不是 profile；dsh resolveProfileDir 同样拒绝该名。
            if name == "node_modules" {
                continue;
            }
            let (bundles, dependencies) = read_manifest_fields(&dir.join("package.json"));
            out.push(ProfileSummary {
                name,
                materialized: true,
                bundles,
                dependencies,
            });
        }
    }
    for (name, bundles) in PROFILE_TEMPLATES {
        if out.iter().any(|p| &p.name == name) {
            continue;
        }
        out.push(ProfileSummary {
            name: (*name).to_string(),
            materialized: false,
            bundles: bundles.iter().map(|s| (*s).to_string()).collect(),
            dependencies: Vec::new(),
        });
    }
    out.sort_by(|a, b| {
        b.materialized
            .cmp(&a.materialized)
            .then_with(|| a.name.cmp(&b.name))
    });
    out
}

/// 从 package.json 文本提取 `(dsh.profile.bundles, dependencies 包名)`。
/// 缺失 / 非法 JSON / 字段形状不符 → 空列表（列表页容忍损坏；详情页另行报错）。
fn read_manifest_fields(path: &Path) -> (Vec<String>, Vec<String>) {
    let Ok(text) = fs::read_to_string(path) else {
        return (Vec::new(), Vec::new());
    };
    let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&text) else {
        return (Vec::new(), Vec::new());
    };
    let bundles = manifest_bundles(&pkg);
    let dependencies = pkg
        .pointer("/dependencies")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.keys()
                .cloned()
                .collect::<BTreeSet<String>>()
                .into_iter()
                .collect()
        })
        .unwrap_or_default();
    (bundles, dependencies)
}

/// 提取 `dsh.profile.bundles` 的字符串项（形状不符/混入非字符串项时取子集）。
fn manifest_bundles(pkg: &serde_json::Value) -> Vec<String> {
    pkg.pointer("/dsh/profile/bundles")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str())
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

/// 详情：package.json 关键字段 + cordis.patch.yml 原文。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ProfileDetail {
    /// package.json 的 `name` 字段（dsh initProfile @ 353 约定
    /// `dsh-profile-<目录名>`）；缺失/非字符串 = None（原样展示，不做推断）。
    pub package_name: Option<String>,
    /// `dsh.profile.bundles`（插件组合，reconcile 回写后的清单为准）。
    pub bundles: Vec<String>,
    /// dependencies 的 name → specifier（pnpm 回写均为字符串；非字符串值跳过）。
    pub dependencies: BTreeMap<String, String>,
    /// cordis.patch.yml 原文（不做 YAML 解析——serde_yaml 已归档弃维，依赖选型
    /// 推迟到启停插件的刀，handoff-4.3-readonly §4.3）；文件不存在 = None。
    pub patch_yaml: Option<String>,
}

/// 读单个 profile 详情。名字先过 dsh 同款校验（防路径遍历）；目录不存在
/// （含未物化模板名）报错——首启前无详情可读。
pub fn read_profile_detail(home: &Path, name: &str) -> Result<ProfileDetail, String> {
    validate_profile_name(name)?;
    let dir = home.join("profiles").join(name);
    if !dir.is_dir() {
        return Err(format!(
            "profile「{name}」尚未物化（目录不存在）：内置模板名首次启动或首次 plugin add 后才有详情"
        ));
    }
    let manifest_path = dir.join("package.json");
    let text = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("读取 package.json 失败（{}）：{e}", manifest_path.display()))?;
    let pkg: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("package.json 非法 JSON（{}）：{e}", manifest_path.display()))?;
    let dependencies = pkg
        .pointer("/dependencies")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();
    let patch_yaml = fs::read_to_string(dir.join(PROFILE_PATCH_FILENAME)).ok();
    Ok(ProfileDetail {
        package_name: pkg.get("name").and_then(|v| v.as_str()).map(String::from),
        bundles: manifest_bundles(&pkg),
        dependencies,
        patch_yaml,
    })
}

#[cfg(test)]
mod profiles_tests {
    use super::*;

    /// 内联 fixture 临时目录（settings.rs 既有风格：进程级递增编号防并发冲突）。
    fn tmp() -> std::path::PathBuf {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let d = std::env::temp_dir().join(format!(
            "dsh-dock-profiles-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// 写一个 profile 目录的最小三件套（package.json 必写，patch 可选）。
    fn materialize(home: &Path, name: &str, package_json: &str) {
        let dir = home.join("profiles").join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("package.json"), package_json).unwrap();
    }

    const PKG_WEB: &str = r#"{
  "name": "dsh-profile-web",
  "private": true,
  "dependencies": {},
  "dsh": { "profile": { "bundles": ["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-web-app"] } }
}"#;
    const PKG_ALPHA: &str = r#"{
  "name": "dsh-profile-alpha",
  "dependencies": { "@deepseek-ai/dsh-base": "^0.1.0", "my-plugin": "1.2.3" },
  "dsh": { "profile": { "bundles": ["@deepseek-ai/dsh-base"] } }
}"#;

    // ---------- 命名校验（逐字一致 = 拒绝集恰好这六种，其余全放行） ----------

    #[test]
    fn name_validation_rejects_exactly_dsh_rules() {
        // dsh resolveProfileDir @ 318 的拒绝集：空名 / `/` / `\` / `.` / `..` / node_modules
        for bad in ["", "a/b", "a\\b", ".", "..", "node_modules"] {
            assert!(
                validate_profile_name(bad).is_err(),
                "名字 {bad:?} 应被拒绝（dsh 同款规则）"
            );
        }
    }

    #[test]
    fn name_validation_allows_what_dsh_allows() {
        // 勿加码：点开头 / 空格 / Unicode / ..前缀 / node_modules 扩展名都合法；
        // 语义差异名（".."开头、"node_modules" 子串）不是字面量匹配。
        for good in [
            "web",
            "headless",
            "my-profile",
            "profile_1",
            "中文名",
            ".hidden",
            "a b",
            "..foo",
            "node_modulesx",
            "my.node_modules",
        ] {
            assert_eq!(
                validate_profile_name(good),
                Ok(()),
                "名字 {good:?} dsh 允许，壳不得拒绝"
            );
        }
    }

    // ---------- 扫描器：两态合并 + node_modules 农场排除 ----------

    #[test]
    fn scan_lists_materialized_and_unmaterialized_templates() {
        let home = tmp();
        materialize(&home, "alpha", PKG_ALPHA);
        materialize(&home, "web", PKG_WEB);
        std::fs::write(home.join("profiles").join("loose.txt"), "x").unwrap();

        let list = scan_profiles(&home);
        let names: Vec<&str> = list.iter().map(|p| p.name.as_str()).collect();

        // 已物化在前、字典序；未物化模板名追加；node_modules 农场与非目录不出现
        assert_eq!(names, vec!["alpha", "web", "headless"]);

        let alpha = &list[0];
        assert!(alpha.materialized);
        assert_eq!(alpha.bundles, vec!["@deepseek-ai/dsh-base"]);
        assert_eq!(
            alpha.dependencies,
            vec!["@deepseek-ai/dsh-base", "my-plugin"]
        );

        let web = &list[1];
        assert!(web.materialized);
        assert_eq!(
            web.bundles,
            vec!["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-web-app"]
        );

        let headless = &list[2];
        assert!(!headless.materialized, "无目录的模板名 = 可首启态");
        assert_eq!(
            headless.bundles,
            vec!["@deepseek-ai/dsh-base", "@deepseek-ai/dsh-headless"]
        );
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn scan_skips_node_modules_farm_but_lists_broken_profiles() {
        let home = tmp();
        // 符号链接农场目录：不是 profile，绝不入列
        std::fs::create_dir_all(home.join("profiles").join("node_modules")).unwrap();
        // 半初始化目录（init 中断：无 package.json）：目录名占名，照列、字段置空
        std::fs::create_dir_all(home.join("profiles").join("half")).unwrap();
        // 清单损坏的目录：照列、字段置空（列表页容忍，详情页报错）
        materialize(&home, "corrupt", "{ not json");

        let list = scan_profiles(&home);
        let names: Vec<&str> = list.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["corrupt", "half", "headless", "web"]);
        assert!(
            !names.contains(&"node_modules"),
            "profiles/node_modules 是符号链接农场（dsh @ 409 维护），不是 profile"
        );
        for broken in list.iter().take(2) {
            assert!(broken.bundles.is_empty() && broken.dependencies.is_empty());
        }
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn scan_without_profiles_dir_yields_only_templates() {
        let home = tmp();
        let list = scan_profiles(&home);
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|p| !p.materialized));
        assert_eq!(list[0].name, "headless");
        assert_eq!(list[1].name, "web");
        std::fs::remove_dir_all(&home).ok();
    }

    // ---------- 详情：package.json 关键字段 + patch 原文 ----------

    #[test]
    fn detail_reads_manifest_and_raw_patch() {
        let home = tmp();
        materialize(&home, "alpha", PKG_ALPHA);
        let patch =
            "# Your patch layer for this dsh profile, applied after every bundle layer:\n[]\n";
        std::fs::write(
            home.join("profiles").join("alpha").join("cordis.patch.yml"),
            patch,
        )
        .unwrap();

        let d = read_profile_detail(&home, "alpha").unwrap();
        assert_eq!(d.package_name.as_deref(), Some("dsh-profile-alpha"));
        assert_eq!(d.bundles, vec!["@deepseek-ai/dsh-base"]);
        assert_eq!(
            d.dependencies
                .get("@deepseek-ai/dsh-base")
                .map(String::as_str),
            Some("^0.1.0")
        );
        assert_eq!(
            d.dependencies.get("my-plugin").map(String::as_str),
            Some("1.2.3")
        );
        // 原文逐字返回（本刀不解析 YAML）
        assert_eq!(d.patch_yaml.as_deref(), Some(patch));
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn detail_missing_patch_is_none_and_non_string_deps_skipped() {
        let home = tmp();
        materialize(
            &home,
            "odd",
            r#"{ "dependencies": { "str": "1.0.0", "obj": { "workspace": true } } }"#,
        );
        let d = read_profile_detail(&home, "odd").unwrap();
        assert_eq!(d.package_name, None);
        assert!(d.bundles.is_empty());
        assert_eq!(d.dependencies.len(), 1, "非字符串依赖值跳过（不误报）");
        assert_eq!(d.patch_yaml, None, "patch 层未初始化 = None");
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn detail_rejects_invalid_names_before_touching_fs() {
        let home = tmp();
        materialize(&home, "ok", PKG_ALPHA);
        // 路径遍历与非法名：一律在校验层拒绝，不产生任何文件读取
        for bad in ["", "../ok", "a/b", "a\\b", ".", "..", "node_modules"] {
            let err = read_profile_detail(&home, bad).unwrap_err();
            assert!(
                err.contains("非法 profile 名"),
                "名字 {bad:?} 应被校验层拒绝：{err}"
            );
        }
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn detail_unmaterialized_name_is_actionable_error() {
        let home = tmp();
        let err = read_profile_detail(&home, "headless").unwrap_err();
        assert!(err.contains("尚未物化"), "应提示先启动/初始化：{err}");
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn detail_corrupt_manifest_is_error_not_empty() {
        let home = tmp();
        materialize(&home, "corrupt", "{ not json");
        let err = read_profile_detail(&home, "corrupt").unwrap_err();
        assert!(err.contains("非法 JSON"), "损坏清单在详情页须可见：{err}");
        std::fs::remove_dir_all(&home).ok();
    }
}
