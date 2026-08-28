//! profiles.rs —— Profile 管理器（4.3 第二刀只读 + 第三刀创建，2026-08-28）。
//!
//! 职责：扫描 `<dsh_home>/profiles/` 列出全部 profile（已物化 + 内置模板名两态
//! 合并展示）、读取单个 profile 详情（package.json 关键字段 + cordis.patch.yml
//! 原文）。只读零写入、零 dsh 子进程。
//! 创建：spawn `dsh plugin --profile <名> add @deepseek-ai/dsh-base` 半官方
//! 转发链（ADR-0009 方案 A）——三件套由 dsh `initProfile` 写出，壳对 profiles/
//! 零写入；spawn 前做 pnpm 防御检测（缺失给可行动错误，补齐归后续刀）。
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

// ---------- 创建（4.3 第三刀，2026-08-28；ADR-0009 方案 A） ----------

/// 创建 profile 的 add 参数：非模板名 = dsh `DEFAULT_PROFILE_BUNDLES` @ 334
/// （`@deepseek-ai/dsh-base`）；模板名 web/headless 的 init 由 dsh 侧
/// `PROFILE_TEMPLATES` @ 323 命中，与本参数无关——reconcile 对模板内置
/// bundle 零动作（plugin-9h8shc4d.js reconcilePlugins 注释：In-box bundles
/// from the profile template are not dependencies and are never touched），
/// 故两类名字统一传 dsh-base，安全且幂等。
const CREATE_ADD_BUNDLE: &str = "@deepseek-ai/dsh-base";

/// 转发链超时上限：pnpm 冷网络安装的余量。超时杀 dsh 进程；其 pnpm 孙进程
/// 会自行退出（不做进程组追杀），已写盘的依赖对重试幂等无碍。
const CREATE_FORWARD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

/// spawn 参数序列：`plugin --profile <名> add @deepseek-ai/dsh-base`。
/// profile 名作为单个 argv 元素传递（不经 shell 拼接，空格/Unicode 名安全；
/// 合法性由 creation_blocker 先行把关）。
pub fn create_command_args(profile: &str) -> Vec<String> {
    vec![
        "plugin".to_string(),
        "--profile".to_string(),
        profile.to_string(),
        "add".to_string(),
        CREATE_ADD_BUNDLE.to_string(),
    ]
}

/// 创建前置校验：Ok = 可发起（新名，或半初始化重试）；Err = 拒绝（原因可行动）。
/// - 非法名 / 路径被普通文件占用（initProfile 的 mkdirSync 会失败）：拒绝；
/// - 已存在且 dependencies 非空：完整 profile，重名拒绝；
/// - 已存在但依赖为空（半初始化 / 清单缺失或损坏）：**放行 = 重跑同名 add**
///   （init-if-needed 幂等；ADR-0009 §4：「已创建未装插件」中间态的
///   重试 = 重跑同名 add）。
pub fn creation_blocker(home: &Path, profile: &str) -> Result<(), String> {
    validate_profile_name(profile)?;
    let dir = home.join("profiles").join(profile);
    if dir.exists() && !dir.is_dir() {
        return Err(format!(
            "路径 {} 被普通文件占用，无法创建 profile",
            dir.display()
        ));
    }
    if dir.is_dir() {
        let (_, dependencies) = read_manifest_fields(&dir.join("package.json"));
        if !dependencies.is_empty() {
            return Err(format!(
                "profile「{profile}」已存在——创建请换名（删除属后续版本能力）"
            ));
        }
    }
    Ok(())
}

/// dsh plugin 转发链单次执行结果（run_dsh_plugin 产出，供纯函数分类）。
#[derive(Debug, Clone)]
pub struct ForwardRun {
    /// dsh 进程退出码（超时/被信号杀死 = None）。
    pub code: Option<i32>,
    pub timed_out: bool,
    /// dsh 输出全文（stdout+stderr 合流，见 run_dsh_plugin 的文件中转）。
    pub output: String,
}

/// 执行一次 dsh plugin 转发链（阻塞，调用方负责放后台线程）。
/// stdout/stderr 合流落 log_path（双管道直读有死锁风险，文件中转与
/// shell.rs spawn_dsh 同风格），200ms 轮询 try_wait，超时 kill。
/// env 注入与 shell.rs spawn_dsh 同链：显式 DSH_HOME（与扫描器同一
/// user_dsh_home()，创建的目录必落在列表可见位置）；PATH = node 首位 +
/// effective_path（dsh 内部 spawnSync("pnpm") 靠它找到 pnpm，Spike A §3.4）。
pub fn run_dsh_plugin(
    node_bin: &Path,
    dsh_bin_js: &Path,
    args: &[String],
    dsh_home: &Path,
    log_path: &Path,
) -> Result<ForwardRun, String> {
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let log = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(log_path)
        .map_err(|e| format!("打开日志 {} 失败：{e}", log_path.display()))?;
    let mut cmd = crate::child_cmd(node_bin);
    cmd.arg(dsh_bin_js)
        .args(args)
        .env("DSH_HOME", dsh_home)
        .env(
            "PATH",
            crate::resolve::path_with_bin(node_bin, &crate::resolve::effective_path()),
        )
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::from(
            log.try_clone().map_err(|e| e.to_string())?,
        ))
        .stderr(std::process::Stdio::from(log));
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("spawn dsh 失败（{}）：{e}", node_bin.display()))?;
    let deadline = std::time::Instant::now() + CREATE_FORWARD_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Ok(ForwardRun {
                    code: status.code(),
                    timed_out: false,
                    output: crate::resolve::read_log_auto(log_path),
                });
            }
            Ok(None) => {}
            Err(e) => return Err(format!("等待 dsh 退出失败：{e}")),
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(ForwardRun {
                code: None,
                timed_out: true,
                output: crate::resolve::read_log_auto(log_path),
            });
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

/// 创建结果（前端展示态）。「已创建未装插件」是合法中间态而非失败：
/// dsh 先 init 后 pnpm，pnpm 失败不回滚（Spike A §3.3，ADR-0009 §3 方案 A）。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CreateProfileOutcome {
    pub profile: String,
    /// 三件套已由 dsh initProfile 写出（调用方以 package.json 存在性判定）。
    pub materialized: bool,
    /// dsh 退出码 0：基础插件已装（exit 0 = init + pnpm + reconcile 全过）。
    pub installed: bool,
    /// 人读状态 + 可行动建议（附 dsh 输出尾部，便于排障）。
    pub detail: String,
}

/// 把转发链执行结果分类为前端可消费的创建结果（纯函数）。
/// dsh 输出锚定（plugin-9h8shc4d.js runPlugin @ 101）：
/// init 行 = `dsh: initialized profile <名> at <目录>`；
/// pnpm 缺失 = `pnpm not found on PATH`（exit 127，dsh 自带文案）；
/// pnpm 失败 = `pnpm failed in profile directory`（exit = pnpm 退出码）。
pub fn classify_create_outcome(
    profile: &str,
    run: &ForwardRun,
    materialized: bool,
) -> CreateProfileOutcome {
    let installed = !run.timed_out && run.code == Some(0);
    let pnpm_missing = run.output.contains("pnpm not found on PATH");
    let code_text = run
        .code
        .map(|c| c.to_string())
        .unwrap_or_else(|| "未知".to_string());
    let status_line = if installed && materialized {
        format!("profile「{profile}」已创建，基础插件（{CREATE_ADD_BUNDLE}）安装完成。")
    } else if installed {
        "dsh 报告成功，但未检出 profile 目录（异常状态，请反馈）。".to_string()
    } else if run.timed_out {
        format!(
            "创建超时（{} 分钟）已终止：profile 可能已初始化，可重试创建（重跑幂等）。",
            CREATE_FORWARD_TIMEOUT.as_secs() / 60
        )
    } else if materialized && pnpm_missing {
        format!(
            "profile「{profile}」已创建，但插件未安装：pnpm 不在 PATH 上。\
             可在终端运行 npm install -g pnpm 后重试创建（重跑幂等）。"
        )
    } else if materialized {
        format!(
            "profile「{profile}」已创建，但插件安装失败（退出码 {code_text}）。\
             可重试创建（重跑幂等）；若持续失败请检查网络与 npm 镜像配置。"
        )
    } else {
        format!("创建失败：dsh 初始化未完成（退出码 {code_text}）。")
    };
    let tail = output_tail(&run.output);
    let detail = if tail.is_empty() {
        status_line
    } else {
        format!("{status_line}\n—— dsh 输出尾部 ——\n{tail}")
    };
    CreateProfileOutcome {
        profile: profile.to_string(),
        materialized,
        installed,
        detail,
    }
}

/// dsh 输出尾部（最后 15 行、至多 4000 字符，超限从头截断保住末尾）。
fn output_tail(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(15);
    let joined = lines[start..].join("\n");
    let chars: Vec<char> = joined.chars().collect();
    if chars.len() > 4000 {
        chars[chars.len() - 4000..].iter().collect()
    } else {
        joined
    }
}

/// 创建 profile 的完整阻塞流程（调用方负责放后台线程，不冻结主线程）：
/// 前置校验 -> 定位系统 node/dsh -> pnpm 防御检测 -> spawn 转发链 -> 分类。
/// 定位仅走系统探测（detect_system_*，与扫描器面向的 system 档用户一致；
/// 离线档/未装用户得到可行动错误，复用运行会话解析结果留待后续刀评估）。
/// pnpm 防御检测（ADR-0009 §4：基准 = 注入后的 PATH）：缺失直接给可行动
/// 错误而不 spawn——dsh 会先 init 再失败，留下半初始化目录（虽可重试，
/// 失败前置更干净）；补齐（npm i -g pnpm，复用 boot 同一函数）属后续刀。
pub fn create_profile_blocking(
    profile: &str,
    data_dir: &Path,
) -> Result<CreateProfileOutcome, String> {
    let home = crate::resolve::user_dsh_home();
    creation_blocker(&home, profile)?;
    let path_env = crate::resolve::effective_path();
    let node = crate::resolve::detect_system_node(&path_env)
        .ok_or("未检出系统 Node（PATH 上无 node）——创建 profile 需要系统 Node 与 dsh")?;
    let dsh = crate::resolve::detect_system_dsh(&path_env).ok_or(
        "未检出系统 dsh（PATH 上无官方安装）——profile 创建经 dsh CLI 完成，需要系统安装的 dsh",
    )?;
    let runtime_path = crate::resolve::path_with_bin(&node.bin, &path_env);
    if crate::updates::find_pnpm(&runtime_path).is_none() {
        return Err(
            "pnpm 未在 PATH 上找到--dsh 的 plugin 子命令依赖 pnpm 管理插件。\
             可在终端运行 npm install -g pnpm 后重试创建"
                .to_string(),
        );
    }
    let args = create_command_args(profile);
    let run = run_dsh_plugin(
        &node.bin,
        &dsh.bin_js,
        &args,
        &home,
        &data_dir.join("profile-create.log"),
    )?;
    let materialized = home
        .join("profiles")
        .join(profile)
        .join("package.json")
        .is_file();
    Ok(classify_create_outcome(profile, &run, materialized))
}

// ---------- 生命周期：复制 / 重命名 / 删除（4.3 第四刀，2026-08-28） ----------
//
// 引用面全部按 Spike B（docs/spikes/0002-profile-reference-surface.md）执行：
// 目录名是唯一硬身份；`name` 一致化改写是壳仅有的两处三件套写入之一（红线 3
// 允许，ledger 复现点 9）；`profiles/node_modules` 农场不碰；sessions 不级联。
// 运行中防护（ADR-0009 §2）在 IPC 层经 executor::active_profile 比对后调
// running_conflict——复制不防护（源只读不动，ADR 仅要求删除/重命名）。

/// 复制/重命名结果（warnings = 需人工关注项，如 patch 相对路径引用）。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct LifecycleOutcome {
    pub profile: String,
    pub warnings: Vec<String>,
}

/// 删除结果。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct DeleteOutcome {
    pub profile: String,
    /// 该 profile 是默认启动 profile，引用已清除（读取侧兜底 web，ADR-0009 §4）。
    pub default_cleared: bool,
}

/// 运行中防护（ADR-0009 §2 工程准则）：目标 profile 正被壳当前会话使用 →
/// 拒绝并给可行动文案（POSIX 删运行中目录致 dsh 半瘫、Windows 目录占用
/// 删除失败；文案含「其他 dsh 实例」要素，ADR 明文要求）。
pub fn running_conflict(active: Option<&str>, target: &str) -> Result<(), String> {
    if active == Some(target) {
        return Err(format!(
            "profile「{target}」正由当前壳会话使用——请先停止运行中的 dsh（退出应用或切换会话）\
             再删除/重命名；并确保没有其他 dsh 实例（含终端自启）正在使用该 profile"
        ));
    }
    Ok(())
}

/// 默认启动 profile 候选校验（set_default_profile 用）：名字合法且在扫描结果中
/// （已物化或内置模板名均可——模板名恒可首启，web 本身就是 ADR 定死的回退值）。
pub fn ensure_default_candidate(home: &Path, name: &str) -> Result<(), String> {
    validate_profile_name(name)?;
    if scan_profiles(home).iter().any(|p| p.name == name) {
        Ok(())
    } else {
        Err(format!(
            "「{name}」不是可用的默认 profile：既无目录也不是内置模板名（web/headless）"
        ))
    }
}

/// 复制前置校验：源必须已物化（is_dir，未物化模板名无内容可复制）；目标名
/// 合法且完全不存在（目录/文件都算占用——复制不是创建，不走半初始化重试语义）。
pub fn copy_blocker(home: &Path, source: &str, new_name: &str) -> Result<(), String> {
    validate_profile_name(source)?;
    validate_profile_name(new_name)?;
    if !home.join("profiles").join(source).is_dir() {
        return Err(format!(
            "源 profile「{source}」不存在或尚未物化——复制需要已初始化的 profile 目录"
        ));
    }
    if home.join("profiles").join(new_name).exists() {
        return Err(format!("目标名「{new_name}」已被占用——复制请换名"));
    }
    Ok(())
}

/// 重命名前置校验：旧名存在（is_dir）；新名合法且完全不存在。
pub fn rename_blocker(home: &Path, old_name: &str, new_name: &str) -> Result<(), String> {
    validate_profile_name(old_name)?;
    validate_profile_name(new_name)?;
    if !home.join("profiles").join(old_name).is_dir() {
        return Err(format!("profile「{old_name}」不存在或尚未物化"));
    }
    if home.join("profiles").join(new_name).exists() {
        return Err(format!("目标名「{new_name}」已被占用——重命名请换名"));
    }
    Ok(())
}

/// 复制（Spike B §3.2）：整目录复制**排除 node_modules/**（让 pnpm 在新目录
/// 重装，避免旧相对链接）；package.json `name` 一致化改写；其余文件（patch /
/// workspace / 用户自加文件）逐字照搬。返回 warnings（patch 相对路径引用）。
pub fn copy_profile_tree(
    src_dir: &Path,
    dst_dir: &Path,
    new_name: &str,
) -> Result<Vec<String>, String> {
    copy_tree_excluding_node_modules(src_dir, dst_dir)?;
    rewrite_manifest_name(dst_dir, new_name)?;
    Ok(patch_relative_path_warnings(dst_dir))
}

/// 重命名（Spike B §3.1）：目录 rename（同文件系统原子）→ `name` 一致化改写
/// → 删 node_modules（「删 + dsh 下次启动自愈」为第一方案：pnpm 的虚拟 store
/// 相对链接搬移会断）→ patch 相对路径警告。
pub fn rename_profile_dir(
    home: &Path,
    old_name: &str,
    new_name: &str,
) -> Result<Vec<String>, String> {
    let old_dir = home.join("profiles").join(old_name);
    let new_dir = home.join("profiles").join(new_name);
    fs::rename(&old_dir, &new_dir).map_err(|e| {
        format!(
            "目录改名失败（{} → {}）：{e}",
            old_dir.display(),
            new_dir.display()
        )
    })?;
    rewrite_manifest_name(&new_dir, new_name)?;
    let modules = new_dir.join("node_modules");
    if modules.exists() {
        fs::remove_dir_all(&modules)
            .map_err(|e| format!("清理 node_modules 失败（pnpm 将在下次启动重装）：{e}"))?;
    }
    Ok(patch_relative_path_warnings(&new_dir))
}

/// 删除（Spike B §3.3）：整目录删除。不级联 sessions（dsh 明示不级联，会话
/// 容忍悬空 profile 引用）；符号农场残留链接不做（stale 链接对模块解析不可见，
/// 且 dsh heal 幂等）。调用方先做存在性检查与运行中防护。
pub fn delete_profile_dir(home: &Path, name: &str) -> Result<(), String> {
    let dir = home.join("profiles").join(name);
    fs::remove_dir_all(&dir).map_err(|e| format!("删除目录失败（{}）：{e}", dir.display()))
}

/// 递归复制目录，跳过名为 `node_modules` 的子树（复制与重命名的共用件；
/// 符号农场同名的顶层目录天然被排除）。
fn copy_tree_excluding_node_modules(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("创建目录失败（{}）：{e}", dst.display()))?;
    for entry in fs::read_dir(src).map_err(|e| format!("读取目录失败（{src:?}）：{e}"))? {
        let entry = entry.map_err(|e| format!("遍历目录失败（{src:?}）：{e}"))?;
        if entry.file_name() == "node_modules" {
            continue;
        }
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_tree_excluding_node_modules(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|e| format!("复制文件失败（{}）：{e}", from.display()))?;
        }
    }
    Ok(())
}

/// package.json `name` 一致化改写为 `dsh-profile-<新名>`（dsh initProfile @ 353
/// 写入约定；Spike B §2.2：该前缀字段无外部消费处，改写为一致性保持）。
/// 清单缺失（半初始化）跳过；格式对齐 dsh writeProfileManifest（2 空格缩进 +
/// 末尾换行；键序不敏感，dsh 以 JSON.parse 读取）。
fn rewrite_manifest_name(dir: &Path, new_name: &str) -> Result<(), String> {
    let path = dir.join("package.json");
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(());
    };
    let mut pkg: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("package.json 非法 JSON（{}）：{e}", path.display()))?;
    if let Some(obj) = pkg.as_object_mut() {
        obj.insert(
            "name".to_string(),
            serde_json::Value::String(format!("dsh-profile-{new_name}")),
        );
    }
    let out = serde_json::to_string_pretty(&pkg).map_err(|e| e.to_string())?;
    fs::write(&path, out + "\n").map_err(|e| format!("写 package.json 失败：{e}"))
}

/// 扫描 cordis.patch.yml 的 `../` 相对路径引用（Spike B §2.2：patch 语义不含
/// profile 名，但相对路径在目录改名后可能断链——替用户做人工检查的机器版，
/// ADR-0009 行动项）。纯文本逐行扫描（本刀不引 YAML 依赖）：跳过空行与
/// `#` 注释行。
pub fn patch_relative_path_warnings(dir: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(dir.join(PROFILE_PATCH_FILENAME)) else {
        return Vec::new();
    };
    let hits = text
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .filter(|l| l.contains("../"))
        .count();
    if hits == 0 {
        Vec::new()
    } else {
        vec![format!(
            "cordis.patch.yml 检测到 {hits} 处 ../ 相对路径引用——profile 目录变更后这些引用可能断链，请人工检查"
        )]
    }
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

    // ---------- 创建（第三刀） ----------

    #[test]
    fn create_args_forward_profile_name_verbatim() {
        // 名字作为单个 argv 元素（空格/Unicode 不经 shell 拼接）
        assert_eq!(
            create_command_args("my profile"),
            vec![
                "plugin",
                "--profile",
                "my profile",
                "add",
                "@deepseek-ai/dsh-base"
            ]
        );
        // 模板名走同一命令：init 由 dsh PROFILE_TEMPLATES 命中，add 参数不变
        // （ADR-0009 方案 D：dsh plugin add 对模板名也适用，无需单独路径）
        assert_eq!(
            create_command_args("web"),
            vec!["plugin", "--profile", "web", "add", "@deepseek-ai/dsh-base"]
        );
    }

    #[test]
    fn creation_blocker_rejects_invalid_file_and_complete() {
        let home = tmp();
        assert!(creation_blocker(&home, "a/b").is_err(), "非法名先拒");
        assert!(creation_blocker(&home, "fresh").is_ok(), "全新名字放行");

        // 半初始化（目录在、无清单）：放行 = 重试语义（重跑 add 幂等）
        std::fs::create_dir_all(home.join("profiles").join("half")).unwrap();
        assert!(creation_blocker(&home, "half").is_ok());

        // 清单在但依赖空（init 后 pnpm 失败的中间态）：放行 = ADR 重试路径
        materialize(&home, "empty-deps", r#"{ "dependencies": {} }"#);
        assert!(creation_blocker(&home, "empty-deps").is_ok());

        // 完整 profile（依赖非空）：重名拒绝
        materialize(&home, "full", PKG_ALPHA);
        assert!(creation_blocker(&home, "full").is_err());

        // 路径被普通文件占用：拒绝（initProfile 的 mkdirSync 会失败）
        std::fs::write(home.join("profiles").join("afile"), "x").unwrap();
        assert!(creation_blocker(&home, "afile").is_err());
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn classify_covers_dsh_forward_chain_outcomes() {
        let run = |code: Option<i32>, timed_out: bool, output: &str| ForwardRun {
            code,
            timed_out,
            output: output.to_string(),
        };
        // ① 成功（Spike A §3.2 实测输出形态）
        let ok = classify_create_outcome(
            "alpha",
            &run(
                Some(0),
                false,
                "dsh: initialized profile alpha at /tmp/x/profiles/alpha\nProgress: resolved 1\n",
            ),
            true,
        );
        assert!(ok.installed && ok.materialized);
        assert!(ok.detail.contains("安装完成"));

        // ② pnpm 缺失（Spike A §3.3 实测输出，exit 127）：已创建未装 + 可行动建议
        let no_pnpm = classify_create_outcome(
            "alpha",
            &run(
                Some(127),
                false,
                "dsh: initialized profile alpha at /tmp/x/profiles/alpha\n\
                 dsh: pnpm not found on PATH - install pnpm to manage profile plugins\n",
            ),
            true,
        );
        assert!(no_pnpm.materialized && !no_pnpm.installed);
        assert!(
            no_pnpm.detail.contains("npm install -g pnpm"),
            "须含可行动建议"
        );

        // ③ pnpm 失败：已创建未装 + 重试提示（中间态重试 = 重跑同名 add）
        let failed = classify_create_outcome(
            "alpha",
            &run(
                Some(1),
                false,
                "dsh: initialized profile alpha at /tmp/x/profiles/alpha\n\
                 dsh: pnpm failed in profile directory /tmp/x/profiles/alpha\n",
            ),
            true,
        );
        assert!(failed.materialized && !failed.installed);
        assert!(failed.detail.contains("重试"));

        // ④ dsh 自身失败（未物化）：创建失败，不是「已创建未装」
        let dsh_fail =
            classify_create_outcome("alpha", &run(Some(1), false, "node: bad option\n"), false);
        assert!(!dsh_fail.materialized && !dsh_fail.installed);
        assert!(dsh_fail.detail.contains("创建失败"));

        // ⑤ 超时：可重试提示
        let timeout = classify_create_outcome("alpha", &run(None, true, ""), false);
        assert!(!timeout.installed);
        assert!(timeout.detail.contains("超时"));
    }

    #[test]
    fn output_tail_keeps_last_lines_capped() {
        let text: String = (1..=30).map(|i| format!("line-{i}\n")).collect();
        let tail = output_tail(&text);
        assert!(!tail.contains("line-1\n"), "只留最后 15 行");
        assert!(tail.contains("line-30"));
        // 超长单行：从头截断保住末尾标记
        let long = format!("{}END", "x".repeat(5000));
        assert!(output_tail(&long).ends_with("END"));
        assert!(output_tail("").is_empty());
    }

    // ---------- 生命周期（第四刀） ----------

    /// 造一个「完整」profile：三件套 + node_modules 假体（验证排除/清理）。
    fn materialize_full(home: &Path, name: &str) {
        materialize(home, name, PKG_ALPHA);
        let dir = home.join("profiles").join(name);
        std::fs::write(dir.join("cordis.patch.yml"), "# patch\n[]\n").unwrap();
        std::fs::write(dir.join("pnpm-workspace.yaml"), "packages:\n  - .\n").unwrap();
        std::fs::create_dir_all(dir.join("node_modules").join(".pnpm")).unwrap();
        std::fs::write(dir.join("node_modules").join("junk"), "x").unwrap();
    }

    #[test]
    fn copy_excludes_node_modules_and_rewrites_name() {
        let home = tmp();
        materialize_full(&home, "src");
        let warnings = copy_profile_tree(
            &home.join("profiles").join("src"),
            &home.join("profiles").join("dst"),
            "dst",
        )
        .unwrap();
        assert!(warnings.is_empty(), "patch 无 ../ 时无警告：{warnings:?}");

        let dst = home.join("profiles").join("dst");
        assert!(dst.join("package.json").is_file());
        assert!(dst.join("cordis.patch.yml").is_file());
        assert!(dst.join("pnpm-workspace.yaml").is_file());
        assert!(
            !dst.join("node_modules").exists(),
            "node_modules 必须排除（Spike B §3.2：让 pnpm 重装避免旧链接）"
        );
        // name 一致化改写；bundles/dependencies 照搬
        let pkg: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dst.join("package.json")).unwrap())
                .unwrap();
        assert_eq!(pkg["name"], "dsh-profile-dst");
        assert_eq!(pkg["dsh"]["profile"]["bundles"][0], "@deepseek-ai/dsh-base");
        assert!(pkg["dependencies"]["my-plugin"].is_string());
        // 源完全不动（含 name）
        let src_pkg: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(home.join("profiles").join("src").join("package.json"))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(src_pkg["name"], "dsh-profile-alpha");
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn copy_warns_on_patch_relative_paths() {
        let home = tmp();
        let src = home.join("profiles").join("src");
        materialize(&home, "src", PKG_ALPHA);
        std::fs::write(
            src.join("cordis.patch.yml"),
            "# 注释行里的 ../ 不算引用\n[]\n- id: x\n  config:\n    path: ../shared/thing\n",
        )
        .unwrap();
        let warnings = copy_profile_tree(&src, &home.join("profiles").join("dst"), "dst").unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("../"));
        assert!(
            warnings[0].contains("1 处"),
            "注释行不得计入：{}",
            warnings[0]
        );
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn copy_and_rename_blockers_cover_edges() {
        let home = tmp();
        materialize_full(&home, "full");
        std::fs::create_dir_all(home.join("profiles").join("half")).unwrap();
        // 复制：源缺失 / 目标占用（含半初始化目录）/ 非法名
        assert!(copy_blocker(&home, "ghost", "x").is_err());
        assert!(
            copy_blocker(&home, "full", "half").is_err(),
            "半初始化目录也算占用"
        );
        assert!(copy_blocker(&home, "full", "a/b").is_err());
        assert!(copy_blocker(&home, "full", "fresh").is_ok());
        // 半初始化源：目录在即可复制（内容照搬）
        assert!(copy_blocker(&home, "half", "c2").is_ok());
        // 重命名：旧缺失 / 新占用（含普通文件）/ 非法名
        assert!(rename_blocker(&home, "ghost", "x").is_err());
        assert!(rename_blocker(&home, "full", "half").is_err());
        std::fs::write(home.join("profiles").join("afile"), "x").unwrap();
        assert!(
            rename_blocker(&home, "full", "afile").is_err(),
            "文件占用同样拒绝"
        );
        assert!(rename_blocker(&home, "full", "renamed").is_ok());
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn rename_moves_rewrites_and_drops_node_modules() {
        let home = tmp();
        materialize_full(&home, "old");
        let warnings = rename_profile_dir(&home, "old", "new").unwrap();
        assert!(warnings.is_empty());
        assert!(
            !home.join("profiles").join("old").exists(),
            "旧目录必须消失"
        );
        let new_dir = home.join("profiles").join("new");
        assert!(new_dir.join("package.json").is_file());
        assert!(
            !new_dir.join("node_modules").exists(),
            "删 node_modules 让 dsh 下次启动自愈（Spike B §3.1 第一方案）"
        );
        let pkg: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(new_dir.join("package.json")).unwrap())
                .unwrap();
        assert_eq!(pkg["name"], "dsh-profile-new");
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn delete_removes_directory_only() {
        let home = tmp();
        materialize_full(&home, "gone");
        std::fs::create_dir_all(home.join("sessions")).unwrap();
        delete_profile_dir(&home, "gone").unwrap();
        assert!(!home.join("profiles").join("gone").exists());
        assert!(home.join("sessions").is_dir(), "不级联全局数据（dsh 明示）");
        // 调用方存在性检查的兜底：删不存在目录是错误而非静默成功
        assert!(delete_profile_dir(&home, "ghost").is_err());
        std::fs::remove_dir_all(&home).ok();
    }

    #[test]
    fn running_conflict_message_covers_adr_required_elements() {
        assert!(running_conflict(None, "web").is_ok());
        assert!(running_conflict(Some("other"), "web").is_ok());
        let err = running_conflict(Some("web"), "web").unwrap_err();
        assert!(err.contains("先停止"), "须提示先停止：{err}");
        assert!(err.contains("其他 dsh 实例"), "ADR 要求的确认要素：{err}");
    }

    #[test]
    fn default_candidate_accepts_templates_and_materialized_only() {
        let home = tmp();
        materialize(&home, "alpha", PKG_ALPHA);
        assert!(ensure_default_candidate(&home, "alpha").is_ok());
        assert!(
            ensure_default_candidate(&home, "web").is_ok(),
            "未物化模板名可作默认（恒可首启，ADR-0009 §4）"
        );
        assert!(ensure_default_candidate(&home, "ghost").is_err());
        assert!(ensure_default_candidate(&home, "../x").is_err());
        std::fs::remove_dir_all(&home).ok();
    }
}
