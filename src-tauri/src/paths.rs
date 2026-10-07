//! paths.rs —— 用户数据根（app 数据目录 + dsh home）的**唯一权威**与**唯一解析点**
//!（2026-10-07 第一性原理重构，替代原 `resolve::user_dsh_home()` 的全局读法）。
//!
//! ## 为什么要有这个模块
//!
//! Tauri 自己推应用数据目录的方式是 `PathResolver::app_data_dir()` =
//! `dirs::data_dir() / <Config.identifier>`——**身份决定数据位置**。壳的 dsh home
//! 属于同一类"用户数据根"，必须与它同源；此前它由 `DSH_HOME` 环境变量 +
//! `cfg!(debug_assertions)` 两个隐式输入决定，于是 `cargo run` 会出现
//! "app 数据目录是正式的、dsh home 是隔离的"这种混合态，并且调用环境里一个
//! 已导出的 `DSH_HOME` 就能静默把隔离档指到真实 `~/.dsh`。
//!
//! ## 规则（**唯一**一张表，别处不得再判断）
//!
//! | 档 | 来源 | dsh home | `DSH_HOME` |
//! |:---|:---|:---|:---|
//! | `Prod` | 生产 identifier（无 `.dev` 后缀） | `$DSH_HOME` ?? `~/.dsh` | 尊重（dsh 自己的官方开关 = 用户主权） |
//! | `Isolated` | `.dev` flavor（`tauri.dev.conf.json`） | `~/.dsh-dock-dev` | **忽略**——隔离是安全边界，环境变量不得把它指向正式数据 |
//! | `Test`（`cfg(test)`） | 手工给定的临时目录 | — | 忽略 |
//!
//! ## 解析一次、注入（不是"到处读环境"）
//!
//! [`Paths::resolve`] 只在 `lib.rs` 的 setup 里调用一次，结果存进 `ShellState`；
//! 命令层经 [`dsh_home_of`] 取，领域函数收 `&Path`。**全仓唯一读 `DSH_HOME` 与唯一
//! 判断数据档的地方就是本文件**（机器闸门见文件末尾 `paths_gate_tests`）。

use std::ffi::OsString;
use std::path::PathBuf;

/// 正式档 dsh home 目录名（AGENTS §6 运行时持久化例外册）。
pub const PROD_HOME_DIR_NAME: &str = ".dsh";

/// 隔离档 dsh home 目录名（`.dev` flavor 专用；AGENTS §6 在册）。
pub const ISOLATED_HOME_DIR_NAME: &str = ".dsh-dock-dev";

/// 测试专用 home 目录名：**永不**指向用户的真实 `.dsh`（`cargo test` 硬隔离）。
#[cfg(test)]
pub const TEST_HOME_DIR_NAME: &str = ".dsh-dock-test";

/// 数据世界：这一份进程写谁的 profiles / sessions / 凭据。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataWorld {
    /// 正式档：与已安装的 DSH Dock.app 同一份用户数据。
    Prod,
    /// 隔离档：`.dev` flavor，开发期的泄漏只污染自己的 home。
    Isolated,
}

impl DataWorld {
    /// **身份决定数据**：identifier 带 `.dev` 后缀即隔离档。
    ///
    /// 判据与 `tauri.dev.conf.json` 里的 identifier 同源——那份配置本来就是
    /// Tauri 官方的 "build flavor" 机制（`tauri dev --config …`），不引入第二套开关。
    pub fn from_identifier(identifier: &str) -> Self {
        if identifier.ends_with(".dev") {
            Self::Isolated
        } else {
            Self::Prod
        }
    }

    /// 该档的默认 home 目录名（不含用户主目录）。
    pub const fn home_dir_name(self) -> &'static str {
        match self {
            Self::Prod => PROD_HOME_DIR_NAME,
            Self::Isolated => ISOLATED_HOME_DIR_NAME,
        }
    }

    /// 供启动日志/诊断展示。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Prod => "prod",
            Self::Isolated => "isolated",
        }
    }
}

/// 一次解析出来的用户数据根（进程生命周期内不变）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub world: DataWorld,
    /// 应用数据目录（= Tauri `app_data_dir()`，identifier 派生）：settings / engines /
    /// 日志 / 进程登记表。留在结构里是为了日志与诊断**不必各处重算**。
    pub data_dir: PathBuf,
    /// dsh home：`profiles/` `sessions/` `.credentials.yaml` 所在。
    pub dsh_home: PathBuf,
}

impl Paths {
    /// 纯内核（可测）：**唯一**实现"档 + 输入 → 目录"的地方。
    ///
    /// 优先级（本文件即唯一事实源）：
    /// - `Prod`：`DSH_HOME`（dsh 自己的官方开关，用户显式指定最高优先）?? `~/.dsh`；
    /// - `Isolated`：恒 `~/.dsh-dock-dev`，**不看** `DSH_HOME`。
    pub fn resolve_pure(
        world: DataWorld,
        data_dir: PathBuf,
        user_home: Option<PathBuf>,
        dsh_home_env: Option<OsString>,
    ) -> Self {
        let dsh_home = match world {
            DataWorld::Prod => dsh_home_env
                .map(PathBuf::from)
                .or_else(|| user_home.clone().map(|h| h.join(world.home_dir_name())))
                .unwrap_or_else(|| PathBuf::from(world.home_dir_name())),
            DataWorld::Isolated => user_home
                .map(|h| h.join(world.home_dir_name()))
                .unwrap_or_else(|| PathBuf::from(world.home_dir_name())),
        };
        Self {
            world,
            data_dir,
            dsh_home,
        }
    }

    /// 运行时解析：**全仓唯一**读 config / 环境变量 / 平台目录的地方（setup 调一次）。
    pub fn resolve(app: &tauri::AppHandle) -> tauri::Result<Self> {
        use tauri::Manager;
        let world = DataWorld::from_identifier(&app.config().identifier);
        let data_dir = app.path().app_data_dir()?;
        Ok(Self::resolve_pure(
            world,
            data_dir,
            user_home_dir(),
            std::env::var_os("DSH_HOME"),
        ))
    }
}

/// 命令层取用户数据根：从 `ShellState` 取（setup 期已解析），**绝不重解析**。
pub fn of(app: &tauri::AppHandle) -> std::sync::Arc<Paths> {
    use tauri::Manager;
    app.state::<std::sync::Arc<crate::boot::ShellState>>()
        .inner()
        .paths
        .clone()
}

/// 命令层快捷取 dsh home（`Arc` 只做一次克隆，开销可忽略）。
pub fn dsh_home_of(app: &tauri::AppHandle) -> PathBuf {
    of(app).dsh_home.clone()
}

/// GUI 启动时统一取用户 home：Windows 常见的是 USERPROFILE，Unix 使用 HOME。
/// （2026-10-07 从 `resolve.rs` 迁入：数据根相关的环境读取集中在本文件。）
pub(crate) fn user_home_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
    } else {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(PathBuf::from)
    }
}

/// `#[cfg(test)]` 专用：测试的 dsh home（`~/.dsh-dock-test`）——**永不**指向用户的
/// 正式 / 隔离 home（原 `user_dsh_home()` 的 `#[cfg(test)]` 硬隔离分支，语义原样保留：
/// 测试隔离不是用户主权问题，是安全底线）。
#[cfg(test)]
pub fn test_dsh_home() -> PathBuf {
    user_home_dir()
        .map(|h| h.join(TEST_HOME_DIR_NAME))
        .unwrap_or_else(|| PathBuf::from(TEST_HOME_DIR_NAME))
}

/// `#[cfg(all(test, unix))]` 专用：需**真机引擎**的手工用例（引擎由已安装的正式/dev 包铺好）
/// 定位其数据目录。测试里没有 `AppHandle` 可问身份，故按构建档取——两个候选之一，
/// 且**只读**（这些用例不写用户数据）。
///
/// **为什么带 `unix`**：唯三调用点（`shell.rs::engine_session_is_guarded_and_reaped`、
/// `lifecycle.rs::{parent_role_probe, real_dsh_is_reaped_when_shell_is_sigkilled}`）
/// 都是 `#[cfg(unix)]` 的真机锚，Windows 上根本不存在。若这里只写 `cfg(test)`，
/// Windows 的 `clippy --all-targets -D warnings` 会以 dead_code 判红
/// ——2026-10-07 v1.3.5 tag 构建实测（宿主 clippy 看不见，正是 AGENTS §1 那条
/// "clippy 须逐目标各跑一次"的又一例）。
#[cfg(all(test, unix))]
pub fn engine_data_dir_for_test() -> PathBuf {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        user_home_dir().map(|h| h.join("Library/Application Support"))
    };
    let id = if cfg!(debug_assertions) {
        "io.github.realguan.dsh-dock.dev"
    } else {
        "io.github.realguan.dsh-dock"
    };
    base.map(|b| b.join(id))
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod paths_tests {
    use super::*;

    fn ohome() -> Option<PathBuf> {
        Some(PathBuf::from("/home/u"))
    }

    /// 身份 → 档：唯一权威，`.dev` 后缀即隔离（与 tauri.dev.conf.json 同源）。
    #[test]
    fn world_comes_from_identifier_suffix() {
        assert_eq!(
            DataWorld::from_identifier("io.github.realguan.dsh-dock"),
            DataWorld::Prod
        );
        assert_eq!(
            DataWorld::from_identifier("io.github.realguan.dsh-dock.dev"),
            DataWorld::Isolated
        );
        // 未知/未来 identifier 一律正式档（默认安全方向 = 不误判为隔离）
        assert_eq!(
            DataWorld::from_identifier("com.example.other"),
            DataWorld::Prod
        );
        assert_eq!(DataWorld::from_identifier(""), DataWorld::Prod);
        assert_eq!(DataWorld::from_identifier("x.developer"), DataWorld::Prod);
    }

    /// 正式档：`DSH_HOME` 优先（dsh 官方开关 = 用户主权），否则 `~/.dsh`。
    #[test]
    fn prod_honours_dsh_home_env() {
        let p = Paths::resolve_pure(
            DataWorld::Prod,
            "/data".into(),
            ohome(),
            Some(OsString::from("/custom/dsh")),
        );
        assert_eq!(p.dsh_home, PathBuf::from("/custom/dsh"));
        let p = Paths::resolve_pure(DataWorld::Prod, "/data".into(), ohome(), None);
        assert_eq!(p.dsh_home, PathBuf::from("/home/u/.dsh"));
        // 没有 home（极端环境）：退化为相对目录名，不 panic
        let p = Paths::resolve_pure(DataWorld::Prod, "/data".into(), None, None);
        assert_eq!(p.dsh_home, PathBuf::from(".dsh"));
    }

    /// 隔离档：**忽略 `DSH_HOME`** —— 环境变量不得把隔离档指向正式数据
    /// （2026-10-07 实测过的静默劫持路径：从 dsh 会话里起的终端会导出 DSH_HOME）。
    #[test]
    fn isolated_ignores_dsh_home_env() {
        let p = Paths::resolve_pure(
            DataWorld::Isolated,
            "/data".into(),
            ohome(),
            Some(OsString::from("/home/u/.dsh")),
        );
        assert_eq!(
            p.dsh_home,
            PathBuf::from("/home/u/.dsh-dock-dev"),
            "隔离档必须钉死在自己的 home 上"
        );
        // 两档目录名互不重叠（隔离失效 = ADR-0015 §1.2 那类事故的放大器）
        assert_ne!(PROD_HOME_DIR_NAME, ISOLATED_HOME_DIR_NAME);
    }

    /// 数据目录原样透传（= Tauri `app_data_dir()`，由 identifier 派生，不在本模块重算）。
    #[test]
    fn data_dir_is_passed_through() {
        let p = Paths::resolve_pure(DataWorld::Prod, "/data/app".into(), ohome(), None);
        assert_eq!(p.data_dir, PathBuf::from("/data/app"));
        assert_eq!(p.world.label(), "prod");
        assert_eq!(DataWorld::Isolated.label(), "isolated");
        assert_eq!(p.dsh_home, PathBuf::from("/home/u/.dsh"));
    }
}

/// 机器闸门：**数据根的输入只允许发生在本文件**（防回退到"到处读环境 / 自己判断构建档"）。
/// 同族先例：`network_gate.rs` 的 EXEMPTIONS + 源文本扫描。
///
/// 判据是**两个具体形态**，不是"提到 DSH_HOME 就算"：
/// - `user_dsh_home` / `dev_home_dir_name` —— 已删除的旧 API，出现即回退；
/// - `var_os("DSH_HOME")` / `var("DSH_HOME")` —— 直接读环境变量决定数据位置。
///
/// 合法用法不受影响：把**已注入的 home** 交给子进程（`.env("DSH_HOME", …)`）、
/// 探测时清掉它（`.env_remove(…)`）、客体脚本里的 `${DSH_HOME:-…}`、错误文案里的
/// 字面量——那些是在**消费**数据根，不是在**决定**它。
#[cfg(test)]
mod paths_gate_tests {
    use std::path::Path;

    const SRC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src");

    #[test]
    fn only_paths_module_reads_data_root_inputs() {
        let mut offenders: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(SRC).expect("读 src/") {
            let path = entry.expect("dir entry").path();
            collect_offenders(&path, &mut offenders);
        }
        assert!(
            offenders.is_empty(),
            "数据根输入只允许出现在 paths.rs，以下位置越界：{offenders:#?}"
        );
    }

    fn collect_offenders(path: &Path, out: &mut Vec<String>) {
        if path.is_dir() {
            for entry in std::fs::read_dir(path).expect("读目录") {
                collect_offenders(&entry.expect("dir entry").path(), out);
            }
            return;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("rs") || path.ends_with("paths.rs") {
            return;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            return;
        };
        for (idx, line) in text.lines().enumerate() {
            let code = line.trim_start();
            // 注释里提到这些名字是允许的（文档要能说明来龙去脉）
            if code.starts_with("//") {
                continue;
            }
            let reads_env =
                code.contains("var_os(\"DSH_HOME\")") || code.contains("var(\"DSH_HOME\")");
            if code.contains("user_dsh_home") || code.contains("dev_home_dir_name") || reads_env {
                out.push(format!("{}:{}: {}", path.display(), idx + 1, line.trim()));
            }
        }
    }
}
