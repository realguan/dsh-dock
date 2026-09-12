//! 引擎编排（ADR-0010）：壳自管 node / pnpm / dsh 的布局、就绪判定与引导子过程。
//!
//! 网络动作属 AGENTS §7 登记的「引擎引导」用途（updates.rs 编排的子进程网络），
//! 唯一入口 = [`crate::updates::ensure_engine_bootstrapped`]；本模块不对 IPC 直接暴露。
//! 布局与行为规格全部来自实机实证：docs/spikes/0003-pnpm12-engine-bootstrap.md。
//!
//! 设计要点（裁定台账见 ADR-0010 §7）：
//! - 单目录引擎：`PNPM_HOME = <数据目录>/engines/`，兼作 runtime 项目根（§2.3）；
//! - 就绪判定 = 三件齐验版本，不满足走幂等补缺，不作为错误；
//! - pnpm 随壳 pin，每次 boot 重铺（幂等覆盖）；node 版本由 node-map 定
//!   （`runtime set node` 幂等切换）；dsh 只验存在——升级显式走更新入口；
//! - 离线语义：首启必须联网；之后 registry 不可达时已装引擎直接启动（补缺失败
//!   仅在真缺件时才 Err）。

use anyhow::{anyhow, bail, Context, Result};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Stdio;

// ---------- 布局（单目录方案，spike 0003 §2.3） ----------

/// PNPM_HOME：`<数据目录>/engines/`，兼作 runtime 项目根（package.json /
/// node_modules / store / bin / global 共存，实测互不干扰）。
pub fn pnpm_home(data_dir: &Path) -> PathBuf {
    crate::resolve::engines_dir(data_dir)
}

/// 引擎 bin 目录：捆绑 pnpm、node shim、以及**壳自写的 dsh shim**都在这里
///（ADR-0017：dsh 不再是 pnpm 全局包，其启动器由壳生成）。
pub fn engine_bin_dir(data_dir: &Path) -> PathBuf {
    pnpm_home(data_dir).join("bin")
}

/// 引擎内 pnpm 可执行文件（捆绑物落地处；Windows 命名差异在此吸收）。
pub fn engine_pnpm_bin(data_dir: &Path) -> PathBuf {
    engine_bin_dir(data_dir).join(if cfg!(windows) { "pnpm.exe" } else { "pnpm" })
}

/// 引擎 node 可执行（`shim add node` 激活的硬链；Windows 命名差异在此吸收）。
pub fn engine_node_bin(data_dir: &Path) -> Option<PathBuf> {
    find_engine_tool(data_dir, "node")
}

/// 引擎 dsh 启动器：**壳自写 shim**（ADR-0017）：Unix shebang 脚本 / Windows `.cmd`
/// ——**不是链接**，也不再是 pnpm 的全局 shim（`pnpm add -g` 会在 Windows 普通账户
/// 因 global hash link 需要符号链接特权而失败，见 ADR-0017 §1）。
/// 形态差异由 `crate::child_cmd` 吸收（Windows 对 `.cmd` 包一层 `cmd.exe /C`）。
pub fn engine_dsh_bin(data_dir: &Path) -> Option<PathBuf> {
    find_engine_tool(data_dir, "dsh")
}

// ---------- dsh 的 project 内安装布局 + 壳自写 shim（ADR-0017） ----------

/// dsh 的 **project 内安装目录**：`<engines>/dsh-runtime/`。
///
/// 为什么单独一个目录（而不是直接把 dsh 装进 `<engines>/`）：`<engines>/` 同时是
/// **PNPM_HOME**（node 运行时 / store / 旧 `global/` 共存）；把 dsh 的 project 安装
/// 隔离出去，才能既不碰全局安装机制、也不打扰既有 node 布局（ADR-0017 §4.1）。
pub fn dsh_runtime_dir(data_dir: &Path) -> PathBuf {
    pnpm_home(data_dir).join("dsh-runtime")
}

/// dsh 包入口（其 `bin` 字段为 `{"dsh": "lib/bin.js"}`，已核实）。
fn dsh_entry_js(data_dir: &Path) -> PathBuf {
    dsh_runtime_dir(data_dir)
        .join("node_modules")
        .join("@deepseek-ai")
        .join("dsh")
        .join("lib")
        .join("bin.js")
}

/// `dsh-runtime/package.json` 的最小内容（pnpm project 前提）。
///
/// **只在缺失/为空时写入**：依赖条目归 pnpm 管（`pnpm add` 会往里写 `dependencies`），
/// 壳无权覆写——见 `ensure_dsh_runtime_layout`。
pub(crate) fn dsh_runtime_package_json() -> &'static str {
    "{\n  \"name\": \"dsh-runtime\",\n  \"private\": true,\n  \"version\": \"0.0.0\"\n}\n"
}

/// POSIX shim 内容（**纯函数**，供单测钉住形态）。
///
/// `exec` 直接替换进程（不留一层 shell）；node 用**绝对路径**，不经 PATH——
/// 引擎目录是壳资产，不依赖用户 PATH 是否前置了 `engines/bin`。
pub(crate) fn dsh_shim_posix(node_bin: &Path, entry_js: &Path) -> String {
    format!(
        "#!/bin/sh\n\
         # DSH Dock engine launcher (ADR-0017: shell-owned shim, not the pnpm global shim).\n\
         exec \"{}\" \"{}\" \"$@\"\n",
        node_bin.display(),
        entry_js.display()
    )
}

/// 壳自写 bootstrap 的文件名（落在 `<engines>/bin/` 下）。
///
/// **单一事实源**（task-61）：既是 `write_dsh_bootstrap` 写出的文件名，也是
/// Windows shim 指向的目标名（由 `dsh_shim_windows` 经 `format!` 取用）。
/// 这两个值过去各写一份（常量 + shim 里的字面量），改一个不会让任何测试红
/// ⇒ 改名会让 Windows shim 指向不存在的文件而**全套测试静默**（qa-verify 变异 M4）。
pub(crate) const DSH_BOOTSTRAP_NAME: &str = "dsh-boot.mjs";

/// Windows shim 内容（**纯函数**）。`%~dp0` = 本 `.cmd` 所在目录（自带尾反斜杠），
/// 故整条 shim 是**相对**的——引擎目录整体搬迁后仍可用。
///
/// **Windows 指向 bootstrap 而非 dsh 入口**（ADR-0018）：bootstrap 置 `process.pkg`
/// 使 dsh 走自带的「模块代理」分支，从而**不建符号链接**——Windows 普通账户没有
/// `SeCreateSymbolicLinkPrivilege`，dsh 默认的 `fs.symlinkSync` 会在启动期
/// `EPERM: symlink` 失败（这就是 ADR-0017 之后暴露的第二道坎）。
///
/// **目标名由 [`DSH_BOOTSTRAP_NAME`] 生成，不并列字面量**（task-61）：这样
/// 「shim 指向的文件」与「壳实际写出的文件」在**构造上不可能不一致**，
/// 而不是靠一条断言事后发现。
///
/// CRLF 行尾：`.cmd` 的约定行尾，避免某些解析器把 `@echo off` 与后续行粘连。
/// 纯 ASCII：Windows 控制台代码页不一，shim 内不放非 ASCII 字节。
pub(crate) fn dsh_shim_windows() -> String {
    format!(
        "@echo off\r\n\
         rem DSH Dock engine launcher (ADR-0018: bootstrap selects dsh's module-proxy mode).\r\n\
         \"%~dp0node.exe\" \"%~dp0{DSH_BOOTSTRAP_NAME}\" %*\r\n"
    )
}

/// 壳自写 bootstrap 内容（**纯函数**，供单测钉住 ADR-0018 §4.1 的三步与绊线）。
///
/// 三步（全部只是**选择 dsh 自带的模式**，不含任何业务逻辑）：
/// 1. `process.pkg ??= { dshDock: true }` ⇒ dsh 的 `isPackagedExecutable()` 为真
///    （其判据即 `process.pkg !== undefined`，`dsh-app-boot/lib/index.js:485`），
///    于是选中 `kind: "proxy"`（目录代理）而非 `kind: "symlink"`（`:612-619`）；
/// 2. `await import(入口)` —— 入口**相对本 bootstrap 自身**解析（`import.meta.url`），
///    不硬编码绝对路径（引擎目录可整体搬迁）；
/// 3. `await mod.runCli()` —— **必须显式调用**：被 import 时 `import.meta.main` 为假，
///    dsh 自己的 `if (import.meta.main) await runCli()`（`lib/bin.js:168`）不会执行，
///    **只 import 不调用 = 什么都不发生**。
///
/// 绊线（ADR-0018 §6「失败要响亮」）：我们依赖的是 dsh 的**未公开内部判据**；若上游
/// 改了它，此处必须给出可行动的错误，而不是一条看不懂的 EPERM。
pub(crate) fn dsh_bootstrap_mjs() -> &'static str {
    r#"// DSH Dock engine bootstrap (ADR-0018).
//
// On Windows a normal account lacks SeCreateSymbolicLinkPrivilege, so dsh's default
// module-fallback mode (fs.symlinkSync) fails with `EPERM: symlink` at startup.
// Setting `process.pkg` makes dsh's isPackagedExecutable() true, which selects its
// built-in module-proxy mode (real directories + entry-N.js re-exports, zero symlinks).
// This file only *selects* that mode; it contains no business logic and patches nothing.
process.pkg ??= { dshDock: true };

const entry = new URL(
  "../dsh-runtime/node_modules/@deepseek-ai/dsh/lib/bin.js",
  import.meta.url,
);

try {
  // Import, then call runCli() explicitly: when this file imports the entry,
  // `import.meta.main` is false, so the entry's own
  // `if (import.meta.main) await runCli()` never fires.
  const mod = await import(entry.href);
  await mod.runCli();
} catch (err) {
  if (err?.code === "EPERM" && err?.syscall === "symlink") {
    console.error(
      "[dsh-dock] dsh 启动时仍在创建符号链接（EPERM / symlink）。\n" +
        "本 bootstrap 已选中 dsh 的「模块代理」模式，说明 dsh 判断打包体的内部依据\n" +
        "（process.pkg）可能已变更——见 ADR-0018 §5「依赖 dsh 的内部判据」。\n" +
        "请反馈此错误：启动方式需要按新版 dsh 重新评估。",
    );
    process.exit(1);
  }
  throw err;
}
"#
}

/// 落位 bootstrap（幂等；无变化零写入）。
pub(crate) fn write_dsh_bootstrap(data_dir: &Path) -> Result<PathBuf> {
    let path = engine_bin_dir(data_dir).join(DSH_BOOTSTRAP_NAME);
    write_if_changed(&path, dsh_bootstrap_mjs().as_bytes(), false)?;
    Ok(path)
}

/// 幂等写文件：内容（含可执行位）一致则**零写入**，否则临时文件 + 原子替换。
///
/// 返回 true = 本次确实写了。沿用仓库既有纪律（无变化不触碰文件）。
fn write_if_changed(path: &Path, content: &[u8], exec: bool) -> Result<bool> {
    // 非 unix 无「可执行位」概念 ⇒ 该参数在 windows 目标上不被读取。显式消费之，
    // 否则 `cargo clippy --target x86_64-pc-windows-gnu -- -D warnings` 报 unused
    // （宿主 clippy 看不见这条：AGENTS §1「clippy 需逐目标各跑一次」）。
    #[cfg(not(unix))]
    let _ = exec;
    let same = std::fs::read(path)
        .map(|cur| cur == content)
        .unwrap_or(false);
    #[cfg(unix)]
    let mode_ok = !exec
        || std::fs::metadata(path)
            .map(|m| {
                use std::os::unix::fs::PermissionsExt;
                m.permissions().mode() & 0o111 != 0
            })
            .unwrap_or(false);
    #[cfg(not(unix))]
    let mode_ok = true;
    if same && mode_ok {
        return Ok(false);
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)
        .with_context(|| format!("创建目录 {} 失败", parent.display()))?;
    let tmp = path.with_extension("dsh-write.tmp");
    std::fs::write(&tmp, content).with_context(|| format!("写临时文件 {} 失败", tmp.display()))?;
    #[cfg(unix)]
    if exec {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))
            .with_context(|| format!("设置 {} 可执行位失败", tmp.display()))?;
    }
    // Windows 的 rename 不覆盖既有文件 ⇒ 先删目标（同 `land_binary` 的既有做法）。
    let _ = std::fs::remove_file(path);
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        anyhow!("替换 {} 失败：{e}", path.display())
    })?;
    Ok(true)
}

/// 写壳自写的 dsh shim（ADR-0017 §4.1）：POSIX 落 `bin/dsh`、Windows 落 `bin/dsh.cmd`
/// ——**两者都在 `find_engine_tool` 既有查找序（`dsh.exe` → `dsh.cmd` → `dsh`）覆盖
/// 的位置内，故查找序无需改动**。
pub(crate) fn write_dsh_shim(data_dir: &Path) -> Result<PathBuf> {
    let bin = engine_bin_dir(data_dir);
    if cfg!(windows) {
        let path = bin.join("dsh.cmd");
        write_if_changed(&path, dsh_shim_windows().as_bytes(), false)?;
        Ok(path)
    } else {
        let node = bin.join("node");
        let path = bin.join("dsh");
        write_if_changed(
            &path,
            dsh_shim_posix(&node, &dsh_entry_js(data_dir)).as_bytes(),
            true,
        )?;
        Ok(path)
    }
}

/// 落位 dsh 的 project 安装**前置布局**（ADR-0017），三件都幂等：
/// ① `dsh-runtime/package.json`（pnpm project 前提；存在即不动）；
/// ② `dsh-runtime/pnpm-workspace.yaml` 的 `nodeLinker: hoisted`
///    ——**复用 `build_policy` 的 upsert 单键唯一实现**，不另写键处理；
/// ③ `<engines>/bin/dsh[.cmd]` 壳自写 shim。
pub(crate) fn ensure_dsh_runtime_layout(data_dir: &Path) -> Result<()> {
    let rt = dsh_runtime_dir(data_dir);
    std::fs::create_dir_all(&rt).with_context(|| format!("创建 {} 失败", rt.display()))?;
    let pkg = rt.join("package.json");
    let missing_or_blank = std::fs::read_to_string(&pkg)
        .map(|s| s.trim().is_empty())
        .unwrap_or(true);
    if missing_or_blank {
        write_if_changed(&pkg, dsh_runtime_package_json().as_bytes(), false)?;
    }
    // 复用既有唯一实现（`<dir>/pnpm-workspace.yaml` 的 `nodeLinker` 单键 upsert）。
    crate::build_policy::ensure_engine_linker(&rt)
        .map_err(|e| anyhow!("dsh-runtime nodeLinker 写入失败：{e}"))?;
    write_dsh_shim(data_dir)?;
    // bootstrap（ADR-0018）：Windows 的 `dsh.cmd` 指向它。**两平台都落位**——
    // ① POSIX 侧无消费者，但它只是一份惰性文本（POSIX `bin/dsh` 仍直连入口，
    //    dsh 在 POSIX 走默认符号链接模式，分叉面未扩大）；
    // ② 两平台一致落位使落位逻辑无平台分支、可被本机单测覆盖（否则 macOS 上
    //    该函数无调用者 ⇒ dead_code 撞 `-D warnings`）。
    write_dsh_bootstrap(data_dir)?;
    Ok(())
}

/// dsh CLI 执行工具链（ADR-0010）：引擎档优先，引擎未就绪回退系统探测——
/// dsh CLI 执行工具链（ADR-0010）：引擎档唯一——探测层退役后系统安装不再
/// 是任何 dsh 操作的来源。消费方 = 插件操作与创建链
///（profiles::run_toolchain_forward 统一执行）。
#[derive(Debug)]
pub enum DshToolchain {
    /// dsh 启动器直接执行（node/pnpm 经 PATH 解析，engines/bin 前置）。
    Engine { node_bin: PathBuf, dsh_bin: PathBuf },
}

/// 解析本次 dsh 操作的工具链：node 与 dsh 双全才可用（半就绪 = boot 引导
/// 未完成），缺件给可行动错误（指向引擎引导）。
pub fn resolve_toolchain(data_dir: &Path) -> Result<DshToolchain, String> {
    match (engine_node_bin(data_dir), engine_dsh_bin(data_dir)) {
        (Some(node_bin), Some(dsh_bin)) => Ok(DshToolchain::Engine { node_bin, dsh_bin }),
        (node, dsh) => Err(format!(
            "引擎未就绪（node {}、dsh {}）——请先启动应用完成引擎引导后重试",
            node.map(|_| "已就绪").unwrap_or("缺失"),
            dsh.map(|_| "已就绪").unwrap_or("缺失"),
        )),
    }
}

/// 引擎 bin 内按名找工具：Windows cmd-shim 形态（.exe / .cmd）与 Unix（裸名）
/// 差异在此吸收。
///
/// 额外认**版本化回退名**（`<name>-<16位hex>[.exe]`）：`stage_pnpm_from_bundle`
/// 在"目标文件被残留进程占用、不可覆盖"时会退回到版本化落位（见 `land_binary`），
/// 此时标准名不可写，引擎必须仍能发现并使用这个回退产物——否则"回退"等于没落位。
/// 前缀匹配刻意收紧为「名字 + `-` + 纯 16 位 hex」，不吞掉别的工具。
fn find_engine_tool(data_dir: &Path, name: &str) -> Option<PathBuf> {
    let dir = engine_bin_dir(data_dir);
    let exts: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ""]
    } else {
        &[""]
    };
    if let Some(found) = exts
        .iter()
        .map(|ext| dir.join(format!("{name}{ext}")))
        .find(|p| p.is_file())
    {
        return Some(found);
    }
    find_versioned_fallback(&dir, name)
}

/// 在引擎 bin 里找 `<name>-<16位hex>[.exe]` 形态的版本化回退产物。
fn find_versioned_fallback(dir: &Path, name: &str) -> Option<PathBuf> {
    let prefix = format!("{name}-");
    let mut best: Option<PathBuf> = None;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        let Some(tag) = file_name.strip_prefix(&prefix) else {
            continue;
        };
        let tag = tag.strip_suffix(".exe").unwrap_or(tag);
        if tag.len() != 16 || !tag.chars().all(|c| c.is_ascii_hexdigit()) {
            continue;
        }
        let path = entry.path();
        // 多个回退产物时取字典序最大者（确定性；不依赖目录遍历顺序）
        if path.is_file() && best.as_ref().is_none_or(|b| path > *b) {
            best = Some(path);
        }
    }
    if let Some(p) = best.as_ref() {
        tracing::warn!(
            "引擎 {name} 使用版本化回退产物（标准名不可写）：{}",
            p.display()
        );
    }
    best
}

/// pnpm 子进程 env：PNPM_HOME 指向引擎目录 + 引擎 bin 前置 PATH
///（PNPM_HOME 已设而 bin 不在 PATH = ERR_PNPM_GLOBAL_BIN_DIR_NOT_IN_PATH）。
pub fn pnpm_process_env(data_dir: &Path, path_env: &str) -> Vec<(String, String)> {
    vec![
        (
            "PNPM_HOME".to_string(),
            pnpm_home(data_dir).display().to_string(),
        ),
        (
            "PATH".to_string(),
            crate::resolve::merge_paths(&[
                engine_bin_dir(data_dir).display().to_string(),
                path_env.to_string(),
            ]),
        ),
    ]
}

// ---------- node 下载镜像（spike 0003 §2.4：唯一有效通道 = env，键 = 发布通道） ----------

pub const NODE_MIRROR_PRIMARY: &str = "https://npmmirror.com/mirrors/node/";
pub const NODE_MIRROR_FALLBACK: &str = "https://nodejs.org/download/release/";

/// 镜像注入 env（键缺省时 pnpm 静默回退默认源——键必须精确为 `release`）。
pub fn node_mirrors_env(base: &str) -> (String, String) {
    (
        "PNPM_CONFIG_NODE_DOWNLOAD_MIRRORS".to_string(),
        format!("{{\"release\":\"{base}\"}}"),
    )
}

// ---------- 进度行解析（映射 boot:progress；实测格式见 spike 0003 §2.2） ----------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDownloadProgress {
    pub node_version: String,
    pub downloaded: u64,
    pub total: u64,
}

/// 解析 pnpm 非 TTY 字节进度行，如
/// `Downloading node@runtime:24.18.0: 1.33 MB/52.08 MB`；其余行返回 None。
pub fn parse_download_progress(line: &str) -> Option<NodeDownloadProgress> {
    let rest = line.trim().strip_prefix("Downloading node@runtime:")?;
    let (version, sizes) = rest.split_once(':')?;
    let (done, total) = sizes.trim().split_once('/')?;
    Some(NodeDownloadProgress {
        node_version: version.trim().to_string(),
        downloaded: parse_size(done.trim())?,
        total: parse_size(total.trim())?,
    })
}

fn parse_size(text: &str) -> Option<u64> {
    let (num, unit) = text.split_once(' ')?;
    let value: f64 = num.parse().ok()?;
    let multiplier = match unit.to_ascii_uppercase().as_str() {
        "B" => 1.0,
        "KB" => 1e3,
        "MB" => 1e6,
        "GB" => 1e9,
        _ => return None,
    };
    Some((value * multiplier) as u64)
}

/// 解析 pnpm 非 TTY 安装进度行（project 内 `add` 的 default reporter），如
/// `Progress: resolved 53, reused 48, downloaded 4, added 3`；实测（2026-09-07，
/// pnpm 12.3.1）行首可带安装目录标签前缀
/// `.../global/v11/<hash>   | Progress: resolved 2, …`——按 `Progress:`
/// 分隔符定位，兼容裸行与前缀行。返回 (已下载包数, resolved 总数)。
pub fn parse_package_progress(line: &str) -> Option<(u64, u64)> {
    let rest = line.trim().split_once("Progress:")?.1;
    let mut resolved = None;
    let mut downloaded = None;
    for part in rest.split(',') {
        let part = part.trim();
        if let Some(n) = part.strip_prefix("resolved ") {
            resolved = n.parse().ok();
        } else if let Some(n) = part.strip_prefix("downloaded ") {
            downloaded = n.parse().ok();
        }
    }
    match (downloaded, resolved) {
        (Some(done), Some(total)) if total > 0 => Some((done, total)),
        _ => None,
    }
}

// ---------- 就绪判定 ----------

/// 引擎三件状态（引擎 bin 内真实执行 `--version` 的结果）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EngineStatus {
    pub pnpm: Option<String>,
    pub node: Option<String>,
    pub dsh: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineGap {
    /// 捆绑 pnpm 未落地（staging 每次覆盖，正常引导不出现）。
    Pnpm,
    /// node 缺失或与 node-map 期望不符（`runtime set node` 幂等切换）。
    Node { found: Option<String> },
    /// dsh 缺失（升级不在此判定——boot 恒用已装版本，升级走更新入口）。
    Dsh,
}

/// 就绪判定：三件齐验版本；pnpm 版本不参与 gap（staging 每次重铺 = pin 随壳）。
pub fn readiness_gaps(status: &EngineStatus, node_expected: &str) -> Vec<EngineGap> {
    let mut gaps = Vec::new();
    if status.pnpm.is_none() {
        gaps.push(EngineGap::Pnpm);
    }
    let norm = |v: &str| v.trim().trim_start_matches('v').to_string();
    let node_found = status.node.as_deref().map(norm);
    if node_found.as_deref() != Some(norm(node_expected).as_str()) {
        gaps.push(EngineGap::Node {
            found: status.node.clone(),
        });
    }
    if status.dsh.is_none() {
        gaps.push(EngineGap::Dsh);
    }
    gaps
}

fn probe_version(bin: &Path, env: &[(String, String)]) -> Option<String> {
    let mut cmd = crate::child_cmd(bin);
    cmd.arg("--version");
    for (k, v) in env {
        cmd.env(k, v);
    }
    // 探测类（短命）：走守卫 seam 保持"无例外"，但 Role::Probe 不配生命线。
    let out = crate::lifecycle::run(
        &mut cmd,
        crate::lifecycle::Role::Probe,
        crate::lifecycle::GuardCtx::of("pnpm --version", None),
    )
    .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// 就绪探测：三件版本。
pub fn probe_engine(data_dir: &Path, path_env: &str) -> EngineStatus {
    let env = pnpm_process_env(data_dir, path_env);
    EngineStatus {
        pnpm: find_engine_tool(data_dir, "pnpm").and_then(|p| probe_version(&p, &env)),
        node: find_engine_tool(data_dir, "node").and_then(|p| probe_version(&p, &env)),
        dsh: find_engine_tool(data_dir, "dsh").and_then(|p| probe_version(&p, &env)),
    }
}

/// 探测引擎是否已完整就绪（三件都在且版本匹配）。
/// 若已就绪，返回 Some(status)；否则返回 None。
pub fn probe_engine_if_ready(data_dir: &Path, path_env: &str) -> Option<EngineStatus> {
    let status = probe_engine(data_dir, path_env);
    let expected = crate::updates::node_plan(data_dir).version;
    if readiness_gaps(&status, &expected).is_empty() {
        Some(status)
    } else {
        None
    }
}

/// 引擎是否已完整就绪（三件都在且版本匹配）。
/// 若已就绪，启动时可跳过步 0（环境检测）与步 1（准备引擎），直接进入步 2（启动工作台）。
#[allow(dead_code)]
pub fn is_engine_ready(data_dir: &Path, path_env: &str) -> bool {
    probe_engine_if_ready(data_dir, path_env).is_some()
}

// ---------- 引导子过程 ----------

/// 把打包期随壳内置的 pnpm 压缩包（@pnpm/exe.<platform> tgz，边界 A 裁定：
/// 安装包内压缩存储）解包落位 `engines/bin/pnpm`，幂等覆盖（pin 随壳）。
/// 解包用系统 tar（Windows 10+ 自带 bsdtar，零新增依赖）。
///
/// ## 2026-09-10 加固（v1.1.0 Windows 实测 2.0「落位 …pnpm.exe」首启即砖）
///
/// 旧实现有三处会**永久卡死启动**且**无法自愈**：
/// 1. **固定暂存目录 `stage-tmp`**：两次引导轮次重叠（首启 + 切换/重试/崩溃自动
///    拉起）时，B 轮的 `remove_dir_all` 会删掉 A 轮刚解出的文件 → A 的落位阶段
///    报"源文件不存在"；
/// 2. **Windows 上 `rename` 不能覆盖既有文件**，而 `copy` 到目标时若目标正被
///    **上一轮被强杀留下的 `pnpm.exe` 进程**占用，Windows 拒绝写（access denied）
///    ——此后**每次启动都失败**（用户实测正是"卡在 100% 很久后强制退出"的场景）；
/// 3. **错误只带上下文**：`with_context` 让 anyhow 的 `Display` 只打印最外层
///    （"落位 <路径>"），底层 `os error`（5/32/NotFound）既不进日志也不进错误卡
///    ——用户只看到一个路径，毫无线索。
///
/// 现改为：**唯一暂存目录**（pid + 序号）→ **先删后放**（消除"目标已存在"）→
/// **短重试**（给 AV 扫描/句柄释放留窗口）→ 仍失败则回退**版本化文件名**
/// （`bin/pnpm-<内容哈希>.exe`，落位语义从"覆盖同名"变成"换一个新名"，从根上
/// 绕开"文件被占用不可覆盖"）→ 失败时**展开完整错误链**。
pub fn stage_pnpm_from_bundle(bundle: &Path, data_dir: &Path) -> Result<PathBuf> {
    let dest = engine_pnpm_bin(data_dir);
    std::fs::create_dir_all(engine_bin_dir(data_dir))
        .with_context(|| format!("创建引擎 bin 目录 {}", engine_bin_dir(data_dir).display()))?;
    let member = if cfg!(windows) {
        "package/pnpm.exe"
    } else {
        "package/pnpm"
    };
    // 唯一暂存目录：并发轮次互不踩（旧实现用固定名，B 轮会删掉 A 轮的解包产物）。
    let tmp = pnpm_home(data_dir).join(format!(
        "stage-tmp-{}-{}",
        std::process::id(),
        STAGE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).with_context(|| format!("创建暂存目录 {}", tmp.display()))?;
    let mut tar = crate::child_cmd(Path::new("tar"));
    tar.arg("-xzf").arg(bundle).arg("-C").arg(&tmp).arg(member);
    let out = crate::lifecycle::run(
        &mut tar,
        crate::lifecycle::Role::Probe,
        crate::lifecycle::GuardCtx::of("tar", None),
    )
    .context("执行系统 tar 解包 pnpm 失败")?;
    if !out.status.success() {
        let _ = std::fs::remove_dir_all(&tmp);
        bail!(
            "tar 解包 pnpm 失败（{}）：{}",
            bundle.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let extracted = tmp.join(member);
    let landed = land_binary(&extracted, &dest).map_err(|e| {
        let _ = std::fs::remove_dir_all(&tmp);
        anyhow!("{e:#}") // 展开完整错误链（旧实现只打印最外层上下文）
    })?;
    let _ = std::fs::remove_dir_all(&tmp);
    Ok(landed)
}

/// 暂存目录序号（同进程内并发轮次唯一）。
static STAGE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// 把解出的二进制落到目标位置：先删后放 + 短重试 + 版本化回退。
///
/// 返回**实际落位**的路径（可能不是 `dest`——回退分支会换一个不冲突的文件名）。
fn land_binary(extracted: &Path, dest: &Path) -> Result<PathBuf> {
    // ① 先删目标：Windows 的 `rename` 不覆盖既有文件，留着它只会让 rename 必败。
    let _ = std::fs::remove_file(dest);
    // ② 短重试：给 AV 扫描 / 上一个持句柄的进程释放留窗口（Windows 实测常见 1–2s）。
    let mut last_err: Option<std::io::Error> = None;
    for attempt in 0..LAND_RETRIES {
        match std::fs::rename(extracted, dest) {
            Ok(()) => return Ok(dest.to_path_buf()),
            Err(rename_err) => {
                // rename 跨设备/被占用时退化为 copy
                match std::fs::copy(extracted, dest) {
                    Ok(_) => return Ok(dest.to_path_buf()),
                    Err(copy_err) => {
                        if attempt + 1 == LAND_RETRIES {
                            last_err = Some(copy_err);
                        } else {
                            tracing::warn!(
                                "落位 pnpm 第 {} 次失败（rename: {rename_err}；copy: {copy_err}），重试…",
                                attempt + 1
                            );
                            std::thread::sleep(LAND_RETRY_DELAY);
                        }
                    }
                }
            }
        }
    }
    // ③ 回退：目标文件被**他人长期占用**（典型场景 = 上一轮被强杀留下的
    //    pnpm.exe 仍在跑，Windows 拒绝写它）。换一个**不冲突的版本化文件名**，
    //    引擎 bin 里可执行文件按名发现（`find_engine_tool` 认该形态）。
    let stem = dest.file_stem().and_then(|s| s.to_str()).unwrap_or("pnpm");
    let tag = short_content_tag(extracted);
    // 无扩展名时**不得**留尾点：`pnpm-<hex>.` 的标签会变成 17 字符，而回退发现
    // 逻辑按 16 位 hex 校验 → 认不出自己落的文件（实测踩到，已加测试钉住）。
    let fallback = match dest.extension().and_then(|s| s.to_str()) {
        Some(ext) if !ext.is_empty() => dest.with_file_name(format!("{stem}-{tag}.{ext}")),
        _ => dest.with_file_name(format!("{stem}-{tag}")),
    };
    std::fs::copy(extracted, &fallback).with_context(|| {
        format!(
            "落位 pnpm 失败（目标 {} 重试 {} 次仍不可写：{}）",
            dest.display(),
            LAND_RETRIES,
            last_err
                .as_ref()
                .map(|e| e.to_string())
                .unwrap_or_else(|| "未知".into())
        )
    })?;
    tracing::warn!(
        "pnpm 目标被占用，已回退为版本化落位：{}（旧文件可能仍被残留进程持有）",
        fallback.display()
    );
    Ok(fallback)
}

/// 短内容标签（取文件字节的简单哈希片段）——用于版本化回退命名。
fn short_content_tag(path: &Path) -> String {
    use std::io::Read as _;
    let mut buf = Vec::new();
    if let Ok(f) = std::fs::File::open(path) {
        let _ = f.take(64 * 1024).read_to_end(&mut buf);
    }
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325; // FNV-1a 偏移基数
    for b in &buf {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// 落位重试次数与间隔（Windows 上 AV/句柄释放的实测窗口）。
const LAND_RETRIES: u32 = 4;
const LAND_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(600);

fn run_engine_pnpm(
    data_dir: &Path,
    path_env: &str,
    args: &[String],
    extra_env: &[(String, String)],
) -> Result<()> {
    let mut cmd = crate::child_cmd(&engine_pnpm_bin(data_dir));
    cmd.args(args).current_dir(pnpm_home(data_dir));
    for (k, v) in pnpm_process_env(data_dir, path_env) {
        cmd.env(k, v);
    }
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let out = crate::lifecycle::run(
        &mut cmd,
        crate::lifecycle::Role::Pnpm,
        crate::lifecycle::GuardCtx::of("pnpm", None),
    )
    .context("spawn 引擎 pnpm 失败")?;
    if !out.status.success() {
        bail!(
            "pnpm {} 失败：{}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

/// 引擎 pnpm 子进程流式执行：stdout 逐行经 `on_line` 上抛（调用方解析进度），
/// stderr 并发排水并收集尾部；失败时取 stderr 尾 8 行并入错误详情。
/// 镜像/registry 循环与进度语义归调用方。
fn run_engine_pnpm_streaming(
    data_dir: &Path,
    path_env: &str,
    cwd: &Path,
    args: &[String],
    extra_env: &[(String, String)],
    on_line: &mut dyn FnMut(&str),
) -> Result<()> {
    let mut cmd = crate::child_cmd(&engine_pnpm_bin(data_dir));
    // `cwd` 决定 pnpm 把哪个目录当 project：`runtime set node` 用 PNPM_HOME 本身，
    // dsh 的 project 内安装用 `dsh-runtime/`（ADR-0017）。
    cmd.args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in pnpm_process_env(data_dir, path_env) {
        cmd.env(k, v);
    }
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    // 守卫式 spawn（ADR-0015）：`runtime set node` / dsh 的 project 内安装是
    // **分钟级**动作，
    // 壳在中途被硬杀会留下占着 pnpm store 的孤儿，令下次引导失败。
    let mut child = crate::lifecycle::spawn(
        &mut cmd,
        crate::lifecycle::Role::Pnpm,
        crate::lifecycle::GuardCtx::of("pnpm", None),
    )
    .context("spawn 引擎 pnpm 失败")?;
    // stderr 并发排水：管道塞满（64KB）会让子进程写阻塞、stdout 永不 EOF，
    // 与「先排干 stdout 再 wait」互锁成死等（无超时）。行进 debug 日志，
    // 失败时取尾部并入错误详情。
    let stderr = child.stderr.take();
    let stderr_thread = std::thread::spawn(move || {
        let mut all = String::new();
        if let Some(s) = stderr {
            for line in BufReader::new(s).lines().map_while(Result::ok) {
                tracing::debug!("[pnpm-runtime] {line}");
                all.push_str(&line);
                all.push('\n');
            }
        }
        all
    });
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            tracing::debug!("[pnpm-runtime] {line}");
            on_line(&line);
        }
    }
    let status = child.wait().context("等待引擎 pnpm 子进程退出失败")?;
    if !status.success() {
        let detail: String = stderr_thread
            .join()
            .unwrap_or_default()
            .lines()
            .rev()
            .take(8)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        bail!("pnpm {} 失败：{}", args.join(" "), detail.trim());
    }
    Ok(())
}

/// node 下载镜像链（2026-09-07）：`DSH_DOCK_NODE_MIRRORS`（逗号分隔）覆盖
/// 顺序——默认链（npmmirror → 官方）面向最终用户网络；境外 CI 冒烟置官方源
/// 优先（npmmirror 自美国 Azure 访问可长时间零进度，smoke 首轮实证）。仅调整
/// §7「引擎引导」用途内的源顺序，不新增网络面。
pub fn node_mirror_chain() -> Vec<String> {
    crate::updates::parse_chain(
        &std::env::var("DSH_DOCK_NODE_MIRRORS").unwrap_or_default(),
        &[NODE_MIRROR_PRIMARY, NODE_MIRROR_FALLBACK],
    )
}

/// `pnpm runtime set node <version>`：镜像链（npmmirror → 官方）逐个尝试；
/// 非 TTY 字节进度行经回调上抛（映射 boot:progress，阶段 = Node）。
/// cwd = 引擎目录——runtime set 为项目作用域，单目录方案恰好把 node 装进
/// 引擎（spike 0003 §2.2）。
pub fn runtime_set_node(
    data_dir: &Path,
    version: &str,
    path_env: &str,
    progress: &mut dyn FnMut(crate::updates::ProgressStage, u64, Option<u64>),
) -> Result<()> {
    let mut errors = Vec::new();
    for base in node_mirror_chain() {
        tracing::info!("runtime set node {version}（镜像 {base}）…");
        let (mk, mv) = node_mirrors_env(&base);
        let mut progress_lines = 0usize;
        let mut last: Option<(u64, u64)> = None;
        let result = run_engine_pnpm_streaming(
            data_dir,
            path_env,
            &pnpm_home(data_dir),
            &[
                "runtime".to_string(),
                "set".to_string(),
                "node".to_string(),
                version.to_string(),
            ],
            &[(mk, mv)],
            &mut |line| {
                if let Some(p) = parse_download_progress(line) {
                    if progress_lines == 0 {
                        tracing::info!(
                            "node v{} 下载中（进度经 boot:progress 实时推进）",
                            p.node_version
                        );
                    }
                    progress_lines += 1;
                    last = Some((p.downloaded, p.total));
                    progress(
                        crate::updates::ProgressStage::Node,
                        p.downloaded,
                        Some(p.total),
                    );
                }
            },
        );
        match result {
            Ok(()) => {
                tracing::info!(
                    "node v{} 就位（{progress_lines} 行下载进度）",
                    crate::updates::display_version(version)
                );
                // 关单：pnpm 下完最后一段字节后转入 SHASUMS 校验/解包，不再输出
                // 进度行——末行常停在 99%，按满额补发一次完成事件（字节确已下完），
                // 前端据此收起下载卡（boot:progress 桥对完成事件不节流）。
                if let Some((_, total)) = last {
                    progress(crate::updates::ProgressStage::Node, total, Some(total));
                }
                return Ok(());
            }
            Err(e) => {
                tracing::warn!("runtime set node（{base}）失败：{e}");
                errors.push(format!("{base}: {e}"));
            }
        }
    }
    Err(anyhow!(
        "node 引导失败（镜像均不可达）：{}",
        errors.join("；")
    ))
}

/// Node 包内 `node` 可执行文件的候选路径（纯函数，供测试）。
///
/// **2026-09-10 修复（v1.1.0 Windows 实测 1.2 / 1.4 / 1.7 的连带项）**：旧实现两条
/// 候选路径都写死 `…/bin/node(.exe)`，**Windows 上必然找不到**——官方 Windows 份
/// 是 zip 变体，按 npm 包形态落地时 `node.exe` 在**包根**，没有 `bin/` 目录
/// （spike 0003 §2.7 已实测记录该布局分叉：「引擎 node 树根除 node.exe 外还有官方
/// 文档与 package.json 等」）。
///
/// 后果链：本函数返回 `None` → `link_real_node_binary` 只留一条 warn →
/// `engines/bin/node` 退化成 `pnpm shim add node` 的 shim dispatcher → 而
/// `link_real_node_binary` 的注释自己写明：该 shim 会让全局安装的 postinstall 报
/// `ERR_PNPM_SHIM_NO_TARGET`。
fn node_bin_candidates(pkg_dir: &Path) -> Vec<PathBuf> {
    let name = if cfg!(windows) { "node.exe" } else { "node" };
    vec![
        // unix 份（tar.gz）与 pnpm 虚拟 store 的标准布局
        pkg_dir.join("bin").join(name),
        // Windows 份（zip 变体）：可执行文件在包根
        pkg_dir.join(name),
        // 防御性：个别镜像再套一层 `node/`
        pkg_dir.join("node").join("bin").join(name),
    ]
}

/// 在给定 Node 包目录里按 [`node_bin_candidates`] 找出真实二进制。
pub fn find_node_bin_in(pkg_dir: &Path) -> Option<PathBuf> {
    node_bin_candidates(pkg_dir)
        .into_iter()
        .find(|p| p.is_file())
}

/// 查找 pnpm runtime set 下载并解包的实际 Node 二进制文件。
pub fn find_runtime_node_bin(data_dir: &Path) -> Option<PathBuf> {
    let base = pnpm_home(data_dir).join("node_modules");
    // 1. 标准软链路径：node_modules/node
    if let Some(found) = find_node_bin_in(&base.join("node")) {
        return Some(found);
    }
    // 2. 虚拟 store 扫描：node_modules/.pnpm/node@runtime+*/node_modules/node
    let pnpm_dir = base.join(".pnpm");
    if let Ok(entries) = std::fs::read_dir(&pnpm_dir) {
        for entry in entries.flatten() {
            let n = entry.file_name();
            if n.to_string_lossy().starts_with("node@runtime") {
                let dir = entry.path().join("node_modules").join("node");
                if let Some(found) = find_node_bin_in(&dir) {
                    return Some(found);
                }
            }
        }
    }
    None
}

/// 固化实际 Node 二进制到 `PNPM_HOME/bin/node`。
/// 原因：pnpm v12 的 `pnpm shim add node` 生成的是带有上下文检查的 shim dispatcher，
/// dsh 安装执行 postinstall 脚本（如 protobufjs/koffi）时，
/// 因子目录 package.json 无 devEngines 声明而报 ERR_PNPM_SHIM_NO_TARGET 退出 1。
/// 直接将真实的 Node 二进制链接/复制到 `bin/node`，确保生命周期脚本透明直接执行。
pub fn link_real_node_binary(data_dir: &Path) -> Result<()> {
    let real_bin = find_runtime_node_bin(data_dir)
        .ok_or_else(|| anyhow!("未在引擎 node_modules 中找到实际 Node 二进制文件"))?;
    let target = engine_bin_dir(data_dir).join(if cfg!(windows) { "node.exe" } else { "node" });
    let _ = std::fs::remove_file(&target);

    #[cfg(unix)]
    {
        if std::os::unix::fs::symlink(&real_bin, &target).is_ok() {
            return Ok(());
        }
    }
    if std::fs::hard_link(&real_bin, &target).is_ok() {
        return Ok(());
    }
    std::fs::copy(&real_bin, &target).with_context(|| {
        format!(
            "复制 Node 二进制从 {} 到 {}",
            real_bin.display(),
            target.display()
        )
    })?;
    Ok(())
}

/// `pnpm shim add node` 并固化实际 Node 二进制到 `PNPM_HOME/bin/node`。
pub fn shim_add_node(data_dir: &Path, path_env: &str) -> Result<()> {
    let _ = run_engine_pnpm(
        data_dir,
        path_env,
        &["shim".to_string(), "add".to_string(), "node".to_string()],
        &[],
    );
    if let Err(e) = link_real_node_binary(data_dir) {
        tracing::warn!("固化实际 Node 二进制未命中（回退 shim）：{e}");
    }
    Ok(())
}

/// 单条 registry 安装失败的成因类别（v1.2.0 D1，2026-09-11）。
///
/// **为什么需要它**：两条 registry 的失败被合并成一句「registry 均不可达」——
/// 但 Windows 普通账户的失败根本不是网络问题（是 global package hash link 需要
/// `SeCreateSymbolicLinkPrivilege`）。文案把真因盖住 ⇒ 错误卡给「检查网络后重试」
/// ⇒ 重试必然再失败。分类拆开后才可能给可行动出路。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstallFailureKind {
    /// 符号链接特权不足（Windows 普通账户；非网络问题）。
    SymlinkPrivilege,
    /// 网络 / registry 不可达。
    Network,
    /// 其它（构建脚本、磁盘、未知）。
    Other,
}

/// 归类单条安装错误文本（**纯函数**，供单测）。
///
/// 判据顺序：**先特权、后网络**——特权失败的文本里可能同时出现 `registry`
/// 与 `os error 5`（聚合文案即如此），顺序反了就会把特权类吸进网络类。
pub(crate) fn classify_install_failure(err: &str) -> InstallFailureKind {
    let e = err.to_lowercase();
    if crate::boot_failure::is_symlink_privilege_detail(&e) {
        InstallFailureKind::SymlinkPrivilege
    } else if e.contains("network")
        || e.contains("registry")
        || e.contains("econnrefused")
        || e.contains("etimedout")
        || e.contains("tls")
        || e.contains("handshake")
        || e.contains("eai_again")
    {
        InstallFailureKind::Network
    } else {
        InstallFailureKind::Other
    }
}

/// 由各 registry 的失败明细聚合出**面向用户**的失败原因（纯函数，供单测）。
///
/// 语义：
/// - 任一条为特权类 ⇒ 归为特权类（这是**可行动**的结论，且与"网络"完全不同）；
///   并带 [`crate::boot_failure::SYMLINK_PRIVILEGE_MARKER`] 稳定标记，供分类表命中。
/// - 否则若任一条为网络类 ⇒ 归为网络类（沿用原「registry 均不可达」口径）。
/// - 否则落兜底文案。
///
/// 保留逐条明细（`errors`）以便排查——聚合结论**不吞**证据。
pub(crate) fn install_failure_message(errors: &[String]) -> String {
    let kinds: Vec<InstallFailureKind> =
        errors.iter().map(|e| classify_install_failure(e)).collect();
    let any_privilege = kinds.contains(&InstallFailureKind::SymlinkPrivilege);
    let any_network = kinds.contains(&InstallFailureKind::Network);
    let detail = errors.join("；");
    if any_privilege {
        format!(
            "{}引擎安装需要创建符号链接，但当前账户没有该权限（Windows 的 \
             SeCreateSymbolicLinkPrivilege）。这不是网络问题，重试不会成功。\n\
             各 registry 明细：{detail}",
            crate::boot_failure::SYMLINK_PRIVILEGE_MARKER
        )
    } else if any_network {
        format!("dsh 引导安装失败（registry 均不可达）：{detail}")
    } else {
        format!("dsh 引导安装失败：{detail}")
    }
}

/// **project 内安装** dsh（ADR-0017）：`<engines>/dsh-runtime/` 里 `pnpm add
/// @deepseek-ai/dsh@<version>`（**非全局**），registry 镜像链逐个尝试
///（allow-build 放行沿 ADR-0009/0005 同一口径）。非 TTY 安装进度行
///（`Progress: resolved N, … downloaded M`）经回调上抛（阶段 = Dsh，包计数）。
///
/// **为什么不再 `add -g`**（ADR-0017 §1，Windows 实机证实）：pnpm 12 的全局安装会建
/// `global/v11/<hash>` **符号链接**（global package hash link），Windows 普通账户没有
/// `SeCreateSymbolicLinkPrivilege` ⇒ `os error 5 / 拒绝访问`，且**该机制无配置可绕**
/// （本机穷举 7 个候选均未消除，阴性结论已自证）。project 内安装 + `nodeLinker: hoisted`
/// **结构性地**不触碰该机制。
///
/// 版本切换/回滚语义不变：仍是「镜像链逐个尝试 → 全部失败才 Err」，错误分类沿用
/// `install_failure_message`（网络类 vs 特权类）。
pub fn install_dsh_project_local(
    data_dir: &Path,
    version: &str,
    path_env: &str,
    progress: &mut dyn FnMut(crate::updates::ProgressStage, u64, Option<u64>),
) -> Result<()> {
    let _ = link_real_node_binary(data_dir);
    // 前置布局（幂等）：package.json + hoisted + 壳自写 shim。放在安装**之前**，
    // 使安装中途失败时布局已就位（重试/手工排查都从同一形态出发）。
    ensure_dsh_runtime_layout(data_dir)?;
    let mut errors = Vec::new();
    for registry in crate::updates::registry_chain() {
        let args = crate::updates::pnpm_project_install_args(
            &registry,
            &format!("@deepseek-ai/dsh@{version}"),
        );
        tracing::info!(
            "pnpm add @deepseek-ai/dsh@{version}（project 内：{}；registry {registry}）…",
            dsh_runtime_dir(data_dir).display()
        );
        let mut progress_lines = 0usize;
        let mut last: Option<(u64, u64)> = None;
        let result = run_engine_pnpm_streaming(
            data_dir,
            path_env,
            &dsh_runtime_dir(data_dir),
            &args,
            &[],
            &mut |line| {
                if let Some((done, total)) = parse_package_progress(line) {
                    if progress_lines == 0 {
                        tracing::info!("dsh 包下载中（进度经 boot:progress 实时推进）");
                    }
                    progress_lines += 1;
                    last = Some((done, total));
                    progress(crate::updates::ProgressStage::Dsh, done, Some(total));
                }
            },
        );
        match result {
            Ok(()) => {
                tracing::info!(
                    "dsh v{} 就位（{progress_lines} 行安装进度）",
                    crate::updates::display_version(version)
                );
                // 关单：末条 Progress 行可能停在 downloaded < resolved，成功
                // 返回即全部就位，按满额补发一次完成事件。
                if let Some((_, total)) = last {
                    progress(crate::updates::ProgressStage::Dsh, total, Some(total));
                }
                // 安装成功后**复断言 shim**（幂等、无变化零写入）：防 pnpm 在
                // dsh-runtime 内建 `.bin/` 时把 `<engines>/bin/dsh` 一并改写/覆盖。
                write_dsh_shim(data_dir)?;
                return Ok(());
            }
            Err(e) => {
                tracing::warn!("dsh 引导安装（{registry}）失败：{e}");
                errors.push(format!("{registry}: {e}"));
            }
        }
    }
    // 失败归类后再聚合（v1.2.0 D1）：此前一律说「registry 均不可达」，
    // 把 Windows 的符号链接特权失败也说成网络问题 ⇒ 错误卡给「检查网络后重试」。
    Err(anyhow!(install_failure_message(&errors)))
}

// ---------- 编排入口 ----------

/// 引导结果：终态就绪状态 + 实际发生的补缺动作（供可观测/日志）。
#[derive(Debug, Clone)]
pub struct BootstrapOutcome {
    pub status: EngineStatus,
    pub node_switched: bool,
    pub dsh_installed: bool,
}

/// 引导 = 就绪判定驱动的幂等补缺。失败语义：已装件不回滚不阻塞（离线语义）；
/// 补缺后仍真缺件才 Err（首启必须联网，之后离线可启动）。
/// node / dsh 的期望版本经**惰性闭包**提供（解析本身可能触网）：
/// - node：缺失必须解析成功（无从引导即 Err）；已装但解析失败（离线且无缓存）
///   → 带警告用已装版本继续（boot 恒用已装引擎，ADR-0010）；
/// - dsh：缺失才调用（dist-tags 查询）；已装永不调用 → 就绪引擎离线零网络。
pub fn bootstrap(
    data_dir: &Path,
    path_env: &str,
    pnpm_bundle: &Path,
    node_resolve: &mut dyn FnMut() -> Result<String>,
    dsh_resolve: &mut dyn FnMut() -> Result<String>,
    progress: &mut dyn FnMut(crate::updates::ProgressStage, u64, Option<u64>),
) -> Result<BootstrapOutcome> {
    // ⓪ **免符号链接布局**（2026-09-10，v1.1.0 Windows 实测 1.2/1.4/1.7 根因）：
    // 必须在**任何 pnpm 装包动作之前**写——pnpm 默认的 isolated 布局要建目录
    // 符号链接，而 Windows 普通账户没有 SeCreateSymbolicLinkPrivilege，
    // `runtime set node` 会以 os error 5 失败，进而 dsh 永远装不上。
    // 非 Windows 平台也写（幂等、无副作用），保证三平台布局一致、行为可预期。
    crate::build_policy::ensure_engine_linker_best_effort(&pnpm_home(data_dir));

    // ① pnpm 随壳 pin：每次 boot 重铺（幂等覆盖，版本不再参与判定）
    tracing::info!("引擎引导：重铺捆绑 pnpm…");
    stage_pnpm_from_bundle(pnpm_bundle, data_dir)?;
    let mut status = probe_engine(data_dir, path_env);
    tracing::info!(
        "引擎引导：三件探测 pnpm={:?} node={:?} dsh={:?}",
        status.pnpm,
        status.node,
        status.dsh
    );
    let mut node_switched = false;
    let mut dsh_installed = false;

    // ② pnpm：staging 后仍缺失 = 捆绑包损坏（bin 未落地），硬错误。
    if status.pnpm.is_none() {
        bail!("捆绑 pnpm 落位后仍不可执行（引擎目录异常）——删除 engines 目录后重启应用可重建");
    }

    // ③ node：期望版本惰性解析（node-map，fail-closed 基线兜底）。已装但解析
    // 失败（离线且无缓存）→ 带警告用已装版本继续（boot 恒用已装引擎，ADR-0010）；
    // 缺失且解析失败 → 硬错误（无从引导）。版本不符 = 在线幂等切换（升级承接）。
    let node_expected = match node_resolve() {
        Ok(v) => {
            tracing::info!("node 期望版本（node-map）：{v}");
            Some(v)
        }
        Err(e) if status.node.is_some() => {
            tracing::warn!("node 期望版本解析失败，用已装引擎 node 继续启动：{e}");
            None
        }
        Err(e) => return Err(anyhow!("node 引导失败（缺失）：期望版本解析失败：{e}")),
    };
    if let Some(expected) = &node_expected {
        let node_gap = |s: &EngineStatus| {
            readiness_gaps(s, expected)
                .iter()
                .any(|g| matches!(g, EngineGap::Node { .. }))
        };
        if node_gap(&status) {
            tracing::info!("node 缺失或版本不符，开始 runtime set node…");
            runtime_set_node(data_dir, expected, path_env, progress).map_err(|e| {
                anyhow!(
                    "node 引导失败（{}，期望 {expected}）：{e}",
                    describe_found(&status.node)
                )
            })?;
            tracing::info!("激活 node shim（shim add node）…");
            shim_add_node(data_dir, path_env)?;
            node_switched = true;
            status = probe_engine(data_dir, path_env);
        } else {
            let _ = link_real_node_binary(data_dir);
        }
    } else {
        let _ = link_real_node_binary(data_dir);
    }

    // ④ dsh：缺失才补；目标版本（最新稳定版，dist-tags）惰性解析——已装永不
    // 调用，就绪引擎离线 boot 零网络（contract v3 在线语义）。
    if status.dsh.is_none() {
        tracing::info!("dsh 缺失，解析目标版本（registry dist-tags）…");
        let dsh_version =
            dsh_resolve().map_err(|e| anyhow!("dsh 引导失败（缺失）：目标版本解析失败：{e}"))?;
        tracing::info!("dsh 目标版本：{dsh_version}，开始 project 内安装…");
        install_dsh_project_local(data_dir, &dsh_version, path_env, progress)?;
        dsh_installed = true;
        status = probe_engine(data_dir, path_env);
    }

    // ⑤ 终验：三件缺一不可（期望版本未知 = 离线降级时，node 只验存在）。
    let expected = node_expected
        .as_deref()
        .or(status.node.as_deref())
        .unwrap_or("");
    let gaps = readiness_gaps(&status, expected);
    if !gaps.is_empty() {
        bail!("引擎就绪判定未通过（{gaps:?}）——首启需联网完成引导，之后可离线启动");
    }
    Ok(BootstrapOutcome {
        status,
        node_switched,
        dsh_installed,
    })
}

/// gap 展示用：引擎现装版本或「缺失」。
fn describe_found(found: &Option<String>) -> String {
    found
        .as_deref()
        .map(|v| format!("现 {v}"))
        .unwrap_or_else(|| "缺失".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **v1.2.0 D1 复现锚（纯函数层）**：Windows 普通账户的 `os error 5` 必须归为
    /// **特权类**而非网络类——否则聚合文案会继续说「registry 均不可达」。
    #[test]
    fn install_failure_classifies_privilege_apart_from_network() {
        // 截图① 的原文形态（含 `registry` 字样 + os error 5）——顺序判据的关键反例
        let privilege = "npmjs.org: link the global package install directory at \
                         C:\\x\\engines\\global\\v11\\db8a\\2082 -> 拒绝访问。(os error 5)";
        assert_eq!(
            classify_install_failure(privilege),
            InstallFailureKind::SymlinkPrivilege,
            "特权失败被归类错误（其中含 registry 字样，必须先判特权）"
        );
        // 纯网络类
        for net in [
            "npmmirror: tls handshake eof",
            "registry https://registry.npmjs.org/ unreachable",
            "connect ECONNREFUSED 104.16.0.35:443",
        ] {
            assert_eq!(
                classify_install_failure(net),
                InstallFailureKind::Network,
                "网络类误判：{net}"
            );
        }
        // 其它（不认识）
        assert_eq!(
            classify_install_failure("postinstall script failed: koffi build error"),
            InstallFailureKind::Other
        );
        // 英文形（`Access is denied` 是 pnpm 12 在 Windows 的原始文案）
        assert_eq!(
            classify_install_failure("Failed to create symlink: Access is denied. (os error 5)"),
            InstallFailureKind::SymlinkPrivilege
        );
    }

    /// 聚合文案：特权优先于网络，且带稳定标记（供 `boot_failure` 分类表命中）。
    #[test]
    fn install_failure_message_privileges_take_precedence_and_carry_marker() {
        // 混合：一条网络 + 一条特权 ⇒ 结论必须是特权（"可行动"的那条）
        let mixed = vec![
            "npmmirror: tls handshake eof".to_string(),
            "npmjs.org: -> 拒绝访问。(os error 5)".to_string(),
        ];
        let msg = install_failure_message(&mixed);
        assert!(
            msg.contains(crate::boot_failure::SYMLINK_PRIVILEGE_MARKER),
            "特权类结论必须带稳定标记（否则分类表命中不了）：{msg}"
        );
        assert!(
            !msg.contains("registry 均不可达"),
            "特权类不得继续说『registry 均不可达』：{msg}"
        );
        assert!(
            msg.contains("不是网络问题"),
            "文案必须掐断错误排查方向：{msg}"
        );
        // 明细不吞：两条原文都要在
        assert!(msg.contains("tls handshake eof") && msg.contains("os error 5"));

        // 纯网络 ⇒ 沿用原口径（回归保护：不要改掉既有的网络文案）
        let net = vec!["npmmirror: tls handshake eof".to_string()];
        assert!(install_failure_message(&net).contains("registry 均不可达"));

        // 其它 ⇒ 不冒充网络
        let other = vec!["build script failed".to_string()];
        let m = install_failure_message(&other);
        assert!(!m.contains("registry 均不可达") && !m.contains("不是网络问题"));
    }

    /// 端到端粘合：生产侧聚合出的特权文案，必须被 `boot_failure` 分类表判为
    /// **特权变体**（两条判据共用同一个标记常量 ⇒ 不会各自漂移）。
    #[test]
    fn privilege_message_round_trips_through_boot_failure_classifier() {
        let msg = install_failure_message(&["npmjs.org: -> 拒绝访问。(os error 5)".to_string()]);
        assert_eq!(
            crate::boot_failure::BootFailure::from_legacy_detail(&msg),
            crate::boot_failure::BootFailure::SymlinkPrivilegeRequired,
            "生产文案未被分类表识别——标记常量两侧漂移了"
        );
    }

    #[test]
    fn parse_download_progress_handles_spike_formats() {
        let p = parse_download_progress("Downloading node@runtime:24.18.0: 1.33 MB/52.08 MB")
            .expect("spike 实测行应可解析");
        assert_eq!(p.node_version, "24.18.0");
        assert_eq!(p.downloaded, 1_330_000);
        assert_eq!(p.total, 52_080_000);
        let zero =
            parse_download_progress("Downloading node@runtime:24.18.0: 0.00 B/52.08 MB").unwrap();
        assert_eq!(zero.downloaded, 0);
        assert_eq!(zero.total, 52_080_000);
        let big = parse_download_progress("Downloading node@runtime:22.20.0: 1.5 GB/2 GB").unwrap();
        assert_eq!(big.downloaded, 1_500_000_000);
        assert_eq!(big.total, 2_000_000_000);
    }

    #[test]
    fn parse_download_progress_ignores_other_lines() {
        assert!(parse_download_progress("Progress: resolved 1, reused 0, downloaded 0").is_none());
        assert!(parse_download_progress("Done in 7.6s using pnpm v12.3.1").is_none());
        assert!(parse_download_progress("").is_none());
    }

    #[test]
    fn parse_package_progress_handles_reporter_lines() {
        let p = parse_package_progress("Progress: resolved 53, reused 48, downloaded 4, added 3")
            .expect("pnpm 安装进度行应可解析");
        assert_eq!(p, (4, 53));
        let prefixed = parse_package_progress(
            ".../global/v11/881f-18d2f7834c8abb48-0   | Progress: resolved 2, reused 0, downloaded 2, added 2, done",
        )
        .expect("带目录标签前缀的实机样本应可解析（2026-09-07 pnpm 12.3.1 实测）");
        assert_eq!(prefixed, (2, 2));
        let first =
            parse_package_progress("Progress: resolved 52, reused 0, downloaded 0, added 0")
                .unwrap();
        assert_eq!(
            first,
            (0, 52),
            "首行 downloaded 0 也应给出总数（驱动包计数进度）"
        );
    }

    #[test]
    fn parse_package_progress_ignores_other_lines() {
        assert!(parse_package_progress("Done in 7.6s using pnpm v12.3.1").is_none());
        assert!(parse_package_progress("Packages: +52").is_none());
        assert!(parse_package_progress("").is_none());
        assert!(
            parse_package_progress("Progress: resolved 0, reused 0, downloaded 0, added 0")
                .is_none(),
            "resolved 0 时总数未知，不产出进度（防除零/误导性 0%）"
        );
    }

    #[test]
    fn node_mirrors_env_shape_matches_pnpm_schema() {
        let (key, value) = node_mirrors_env("https://npmmirror.com/mirrors/node/");
        assert_eq!(key, "PNPM_CONFIG_NODE_DOWNLOAD_MIRRORS");
        assert_eq!(
            value, "{\"release\":\"https://npmmirror.com/mirrors/node/\"}",
            "键必须为发布通道 release，否则 pnpm 静默回退默认源"
        );
    }

    #[test]
    fn pnpm_process_env_prepends_engine_bin_and_sets_home() {
        let root = std::env::temp_dir().join("dsh-engines-env-test");
        let env = pnpm_process_env(&root, "/usr/bin:/bin");
        let home = env.iter().find(|(k, _)| k == "PNPM_HOME").unwrap();
        assert_eq!(
            home.1,
            crate::resolve::engines_dir(&root).display().to_string()
        );
        let path = env.iter().find(|(k, _)| k == "PATH").unwrap();
        let sep = if cfg!(windows) { ';' } else { ':' };
        assert_eq!(
            path.1.split(sep).next(),
            Some(engine_bin_dir(&root).display().to_string().as_str()),
            "引擎 bin 必须在 PATH 首位"
        );
    }

    #[test]
    fn readiness_gaps_normalizes_v_prefix() {
        let ok = EngineStatus {
            pnpm: Some("12.3.1".into()),
            node: Some("v24.18.0".into()),
            dsh: Some("0.1.1".into()),
        };
        assert!(readiness_gaps(&ok, "24.18.0").is_empty());
        let old_node = EngineStatus {
            pnpm: Some("12.3.1".into()),
            node: Some("v24.17.0".into()),
            dsh: Some("0.1.1".into()),
        };
        assert_eq!(
            readiness_gaps(&old_node, "24.18.0"),
            vec![EngineGap::Node {
                found: Some("v24.17.0".into())
            }]
        );
        let missing = EngineStatus::default();
        assert_eq!(
            readiness_gaps(&missing, "24.18.0"),
            vec![
                EngineGap::Pnpm,
                EngineGap::Node { found: None },
                EngineGap::Dsh
            ]
        );
    }

    /// 造一个可执行假体（echo 固定版本），unix only（shebang + chmod）。
    #[cfg(unix)]
    fn fake_tool(dir: &Path, name: &str, version: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let bin = dir.join(name);
        std::fs::write(&bin, format!("#!/bin/sh\necho {version}\n")).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        bin
    }

    #[cfg(unix)]
    fn engine_root(label: &str) -> PathBuf {
        static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "dsh-engines-{label}-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[cfg(unix)]
    #[test]
    fn stage_pnpm_from_bundle_extracts_and_lands_binary() {
        use std::os::unix::fs::PermissionsExt;
        let root = engine_root("stage");
        let work = root.join("bundle-src");
        std::fs::create_dir_all(work.join("package")).unwrap();
        let script = work.join("package/pnpm");
        std::fs::write(&script, "#!/bin/sh\necho 12.3.1\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let bundle = root.join("pnpm-bundle.tgz");
        let tared = crate::child_cmd(Path::new("tar"))
            .arg("-czf")
            .arg(&bundle)
            .arg("-C")
            .arg(&work)
            .arg("package/pnpm")
            .output()
            .unwrap();
        assert!(tared.status.success(), "fixture tar 失败");

        let data_dir = root.join("data");
        let landed = stage_pnpm_from_bundle(&bundle, &data_dir).unwrap();
        assert_eq!(landed, engine_pnpm_bin(&data_dir));
        assert!(landed.is_file());
        let version = crate::child_cmd(&landed).output().unwrap();
        assert_eq!(String::from_utf8_lossy(&version.stdout).trim(), "12.3.1");
        std::fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn probe_engine_and_readiness_on_fake_tools() {
        let root = engine_root("probe");
        let data_dir = root.join("data");
        std::fs::create_dir_all(engine_bin_dir(&data_dir)).unwrap();
        fake_tool(&engine_bin_dir(&data_dir), "pnpm", "12.3.1");
        fake_tool(&engine_bin_dir(&data_dir), "node", "v24.18.0");
        fake_tool(&engine_bin_dir(&data_dir), "dsh", "0.1.1");

        let status = probe_engine(&data_dir, "");
        assert_eq!(status.pnpm.as_deref(), Some("12.3.1"));
        assert_eq!(status.node.as_deref(), Some("v24.18.0"));
        assert_eq!(status.dsh.as_deref(), Some("0.1.1"));
        assert!(readiness_gaps(&status, "24.18.0").is_empty());
        let ready = probe_engine_if_ready(&data_dir, "");
        assert!(ready.is_some());
        assert_eq!(ready.as_ref().unwrap().dsh.as_deref(), Some("0.1.1"));
        assert!(is_engine_ready(&data_dir, ""));
        std::fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn bootstrap_is_noop_network_free_when_engine_ready() {
        // 就绪判定全过 → 引导只重铺 pnpm + 探测，零网络动作（幂等语义）。
        let root = engine_root("noop");
        let data_dir = root.join("data");
        std::fs::create_dir_all(engine_bin_dir(&data_dir)).unwrap();
        fake_tool(&engine_bin_dir(&data_dir), "node", "v24.18.0");
        fake_tool(&engine_bin_dir(&data_dir), "dsh", "0.1.1");

        let work = root.join("bundle-src");
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(work.join("package")).unwrap();
        let script = work.join("package/pnpm");
        std::fs::write(&script, "#!/bin/sh\necho 12.3.1\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let bundle = root.join("pnpm-bundle.tgz");
        crate::child_cmd(Path::new("tar"))
            .arg("-czf")
            .arg(&bundle)
            .arg("-C")
            .arg(&work)
            .arg("package/pnpm")
            .output()
            .unwrap();

        let mut dsh_called = false;
        let outcome = bootstrap(
            &data_dir,
            "",
            &bundle,
            &mut || Ok("24.18.0".to_string()),
            &mut || {
                dsh_called = true;
                Ok("0.1.1".to_string())
            },
            &mut |_, _, _| {},
        )
        .unwrap();
        assert_eq!(outcome.status.pnpm.as_deref(), Some("12.3.1"));
        assert!(!outcome.node_switched);
        assert!(!outcome.dsh_installed);
        assert!(!dsh_called, "引擎就绪时不应解析 dsh 版本（离线零网络语义）");
        assert!(readiness_gaps(&outcome.status, "24.18.0").is_empty());
        std::fs::remove_dir_all(&root).ok();
    }

    #[cfg(unix)]
    #[test]
    fn bootstrap_continues_with_installed_node_when_resolve_fails() {
        // node 已装但期望版本解析失败（离线且无缓存）→ 降级用已装版本继续
        //（离线不阻塞，ADR-0010）；dsh 已装同样不触网。
        let root = engine_root("offline");
        let data_dir = root.join("data");
        std::fs::create_dir_all(engine_bin_dir(&data_dir)).unwrap();
        fake_tool(&engine_bin_dir(&data_dir), "pnpm", "12.3.1");
        fake_tool(&engine_bin_dir(&data_dir), "node", "v24.18.0");
        fake_tool(&engine_bin_dir(&data_dir), "dsh", "0.1.1");
        let work = root.join("bundle-src");
        use std::os::unix::fs::PermissionsExt;
        std::fs::create_dir_all(work.join("package")).unwrap();
        let script = work.join("package/pnpm");
        std::fs::write(&script, "#!/bin/sh\necho 12.3.1\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let bundle = root.join("pnpm-bundle.tgz");
        crate::child_cmd(Path::new("tar"))
            .arg("-czf")
            .arg(&bundle)
            .arg("-C")
            .arg(&work)
            .arg("package/pnpm")
            .output()
            .unwrap();

        let outcome = bootstrap(
            &data_dir,
            "",
            &bundle,
            &mut || Err(anyhow!("registry 不可达")),
            &mut || panic!("dsh 已装时不应解析目标版本"),
            &mut |_, _, _| {},
        )
        .unwrap();
        assert!(!outcome.node_switched);
        assert!(!outcome.dsh_installed);
        assert_eq!(outcome.status.node.as_deref(), Some("v24.18.0"));
        std::fs::remove_dir_all(&root).ok();
    }

    /// **Windows 布局分叉的回归闸门**（v1.1.0 实测 1.2/1.4/1.7 连带项）：
    /// 候选路径必须同时认 `bin/node(.exe)`（unix 份）与**包根** `node.exe`
    /// （Windows zip 变体）。旧实现只认前者，导致 Windows 上永远找不到真实
    /// node，`engines/bin/node` 退化成会让 postinstall 失败的 shim。
    #[test]
    fn node_bin_candidates_cover_both_platform_layouts() {
        let pkg = Path::new("/tmp/engines/node_modules/node");
        let cands = node_bin_candidates(pkg);
        let as_str: Vec<String> = cands.iter().map(|p| p.display().to_string()).collect();
        let want_bin = pkg
            .join("bin")
            .join(if cfg!(windows) { "node.exe" } else { "node" });
        let want_root = pkg.join(if cfg!(windows) { "node.exe" } else { "node" });
        assert!(
            as_str.contains(&want_bin.display().to_string()),
            "必须认标准 bin/ 布局：{as_str:?}"
        );
        assert!(
            as_str.contains(&want_root.display().to_string()),
            "必须认 Windows zip 变体的包根布局（旧实现漏了这条 → Windows 必失败）：{as_str:?}"
        );
        assert!(cands.len() >= 2, "两种布局都要覆盖");
    }

    /// 实盘验证：包根放 node.exe（Windows 形态）也能被找到。
    #[test]
    fn find_node_bin_in_accepts_root_layout() {
        let root = std::env::temp_dir().join(format!("dsh-node-layout-{}", std::process::id()));
        let pkg = root.join("node");
        std::fs::create_dir_all(&pkg).unwrap();
        let exe_name = if cfg!(windows) { "node.exe" } else { "node" };
        // 只放「包根」这一种（模拟 Windows 份），不放 bin/
        let root_exe = pkg.join(exe_name);
        std::fs::write(&root_exe, b"fake").unwrap();
        assert_eq!(
            find_node_bin_in(&pkg).as_deref(),
            Some(root_exe.as_path()),
            "包根布局必须命中"
        );

        // 反例：都没有 → None（不猜、不返回不存在的路径）
        let empty = root.join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        assert_eq!(find_node_bin_in(&empty), None);
        std::fs::remove_dir_all(&root).ok();
    }

    /// **真机端到端（联网、分钟级，故 `#[ignore]`）**：在**空数据目录**里跑完整
    /// 引擎引导，验证
    /// ① `nodeLinker: hoisted` 被写进引擎目录（Windows 免符号链接的根因修复）；
    /// ② node 与 dsh 真的装上且可执行；
    /// ③ 布局是**真实目录**而非符号链接（Windows 普通账户的关键判据）。
    ///
    /// 这条同时是 **Windows 实机验证锚**：在未开开发者模式的 Windows 上跑它，
    /// 旧实现必红（`os error 5`），新实现应绿。
    ///
    /// 跑法：`cargo test --lib engine_bootstrap_uses_symlink_free_layout -- --ignored --nocapture`
    #[test]
    #[ignore = "联网 + 分钟级；Windows 实机验证锚"]
    fn engine_bootstrap_uses_symlink_free_layout() {
        let data_dir = std::env::temp_dir().join(format!(
            "dsh-bootstrap-e2e-{}-{}",
            std::process::id(),
            crate::lifecycle::now_ms()
        ));
        let _ = std::fs::remove_dir_all(&data_dir);
        std::fs::create_dir_all(&data_dir).unwrap();
        let bundle = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/pnpm")
            .join(if cfg!(windows) {
                "win32-x64.tgz"
            } else if cfg!(target_os = "macos") {
                if cfg!(target_arch = "aarch64") {
                    "darwin-arm64.tgz"
                } else {
                    "darwin-x64.tgz"
                }
            } else {
                "linux-x64.tgz"
            });
        // **不得静默跳过**（2026-09-10）：本用例是 Windows 免符号链接修复的
        // 验收锚，而它的前置（捆绑 pnpm）是一个**明确的构建产物**——不是环境
        // 偶然。若这里 return，验证者会看到"绿"却什么都没测（假绿），正是这次
        // 要防的事。故硬失败并给出可行动指令。
        assert!(
            bundle.is_file(),
            "缺少捆绑 pnpm：{}——请先在仓库根执行\n  \
             scripts/fetch-pnpm-bundle.sh {} linux-x64\n\
             （Windows 上于 Git Bash 运行；需 node + curl）",
            bundle.display(),
            if cfg!(windows) {
                "win32-x64"
            } else {
                "darwin-arm64"
            }
        );
        crate::lifecycle::init(&data_dir);
        let outcome = bootstrap(
            &data_dir,
            &crate::resolve::effective_path(),
            &bundle,
            &mut || Ok(crate::updates::node_plan(&data_dir).version),
            &mut crate::updates::latest_stable_dsh_version,
            &mut |_, _, _| {},
        )
        .expect("引擎引导应成功");

        // ① 免符号链接布局已落位
        let cfg_path = pnpm_home(&data_dir).join("pnpm-workspace.yaml");
        let cfg = std::fs::read_to_string(&cfg_path).expect("引擎配置应存在");
        assert!(
            cfg.contains("nodeLinker: hoisted"),
            "必须写入 nodeLinker: hoisted（Windows 免符号链接）：{cfg}"
        );

        // ② 三件就绪
        assert!(outcome.status.pnpm.is_some(), "pnpm 应就绪");
        let node_ver = outcome.status.node.expect("node 应就绪");
        assert!(node_ver.contains("24."), "node 版本异常：{node_ver}");
        assert!(outcome.status.dsh.is_some(), "dsh 应就绪");

        // ③ 真实二进制可执行 + 布局免符号链接
        let node_bin = engine_node_bin(&data_dir).expect("引擎 node 路径");
        let out = crate::child_cmd(&node_bin).arg("-v").output().unwrap();
        assert!(
            String::from_utf8_lossy(&out.stdout).trim().starts_with('v'),
            "引擎 node 应可执行"
        );
        let pkg_dir = pnpm_home(&data_dir).join("node_modules").join("node");
        assert!(
            !pkg_dir
                .symlink_metadata()
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false),
            "node 包应是真实目录（hoisted），不得是符号链接：{}",
            pkg_dir.display()
        );
        std::fs::remove_dir_all(&data_dir).ok();
    }

    // ---------- 落位加固（v1.1.0 Windows 实测 2.0） ----------

    /// 版本化回退产物必须能被引擎发现——否则"回退"等于没落位。
    #[test]
    fn versioned_fallback_is_discoverable() {
        let root = std::env::temp_dir().join(format!("dsh-fallback-{}", std::process::id()));
        let bin = engine_bin_dir(&root);
        std::fs::create_dir_all(&bin).unwrap();
        // 先只有回退产物（模拟"标准名被占用，落了版本化文件"）
        let tag = "0123456789abcdef";
        let fb = bin.join(if cfg!(windows) {
            format!("pnpm-{tag}.exe")
        } else {
            format!("pnpm-{tag}")
        });
        std::fs::write(&fb, b"fake").unwrap();
        assert_eq!(
            find_versioned_fallback(&bin, "pnpm").as_deref(),
            Some(fb.as_path()),
            "版本化回退产物必须可被发现"
        );

        // 标准名出现后优先用标准名
        let std_name = bin.join(if cfg!(windows) { "pnpm.exe" } else { "pnpm" });
        std::fs::write(&std_name, b"fake").unwrap();
        assert_eq!(
            find_engine_tool(&root, "pnpm").as_deref(),
            Some(std_name.as_path()),
            "标准名存在时应优先"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// 回退匹配必须**收紧**：非 16 位 hex 标签、其他工具名不得被吞。
    #[test]
    fn versioned_fallback_matching_is_strict() {
        let root = std::env::temp_dir().join(format!("dsh-fb-strict-{}", std::process::id()));
        let bin = engine_bin_dir(&root);
        std::fs::create_dir_all(&bin).unwrap();
        for bad in [
            "pnpm-notahexlabel12",    // 非 hex
            "pnpm-0123456789abcde",   // 15 位
            "pnpm-0123456789abcdef0", // 17 位
            "pnpm-backup.exe",        // 人为命名（非 hex 标签）
        ] {
            std::fs::write(bin.join(bad), b"x").unwrap();
        }
        assert_eq!(
            find_versioned_fallback(&bin, "pnpm"),
            None,
            "可疑命名不得被当成回退产物（防误选任意文件）"
        );
        // 其他工具名不受影响
        std::fs::write(bin.join("node"), b"x").unwrap();
        assert_eq!(find_versioned_fallback(&bin, "dsh"), None);
        std::fs::remove_dir_all(&root).ok();
    }

    /// 并发轮次不得互相踩（旧实现用固定 `stage-tmp`，B 轮会删掉 A 轮的解包产物）。
    #[test]
    fn staging_dirs_are_unique_per_call() {
        let root = std::env::temp_dir().join(format!("dsh-stage-uniq-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let a = root.join(format!(
            "stage-tmp-{}-{}",
            std::process::id(),
            STAGE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let b = root.join(format!(
            "stage-tmp-{}-{}",
            std::process::id(),
            STAGE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        assert_ne!(a, b, "两次引导轮次的暂存目录必须不同名");
        std::fs::remove_dir_all(&root).ok();
    }

    /// `land_binary`：目标不存在 → 正常落位；目标**已被占用**且不可覆盖 →
    /// 回退版本化文件名（并返回实际路径）。
    #[cfg(unix)]
    #[test]
    fn land_binary_falls_back_when_target_is_unwritable() {
        let root = std::env::temp_dir().join(format!("dsh-land-{}", std::process::id()));
        let src_dir = root.join("src");
        let bin = root.join("bin");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&bin).unwrap();
        let src = src_dir.join("pnpm");
        std::fs::write(&src, b"content-1").unwrap();

        // ① 正常落位
        let dest = bin.join("pnpm");
        let landed = land_binary(&src, &dest).unwrap();
        assert_eq!(landed, dest, "目标可写时应落标准名");
        assert_eq!(std::fs::read(&dest).unwrap(), b"content-1");

        // ② 目标是一个**不可写的目录**（模拟"被占用/不可覆盖"）
        // 注意：① 的 rename 已把 src **移走**，这里必须重新造一份源。
        std::fs::remove_file(&dest).unwrap();
        std::fs::create_dir(&dest).unwrap();
        std::fs::write(&src, b"content-2").unwrap();
        let landed2 = land_binary(&src, &dest).unwrap();
        assert_ne!(landed2, dest, "目标不可写时应回退到版本化文件名");
        assert!(landed2.is_file(), "回退产物必须真的落地");
        assert_eq!(std::fs::read(&landed2).unwrap(), b"content-2");
        // 回退产物必须能被引擎发现
        assert_eq!(
            find_versioned_fallback(&bin, "pnpm").as_deref(),
            Some(landed2.as_path())
        );
        std::fs::remove_dir_all(&root).ok();
    }

    // ---------- ADR-0017：project 内安装 + 壳自写 shim（T-B1，2026-09-11） ----------

    /// `engines.rs` 的**生产段**源码（`#[cfg(test)] mod tests` 之前）——源码闸门只在
    /// 生产段上判，避免测试里的字样自己把自己"通过"（task-36/39 的教训）。
    fn production_section() -> String {
        let src = include_str!("engines.rs").replace("\r\n", "\n");
        let marker = "\n#[cfg(test)]\nmod tests";
        let cut = src
            .find(marker)
            .expect("engines.rs 应有 `#[cfg(test)] mod tests` 标记");
        src[..cut].to_string()
    }

    /// **源码闸门（复现先行，ADR-0017 §6）**：生产段不得再出现 dsh 的**全局安装实参**。
    ///
    /// 这是防"改回去"的机器判据——任何人重新引入 `pnpm add -g`（无论写 `--global`
    /// 还是 `-g`，或改回旧实参构造函数）都会让本断言红。
    /// 缺陷史：Windows 普通账户下 pnpm 全局安装要建 `global/v11/<hash>` 符号链接，
    /// 无 `SeCreateSymbolicLinkPrivilege` ⇒ `os error 5`（ADR-0017 §1）。
    #[test]
    fn dsh_install_never_uses_global_args() {
        let prod = production_section();
        for pat in [
            "\"--global\"",
            "\"-g\"",
            // 旧实参构造函数（含 `--global`）——新口径是 pnpm_project_install_args
            "pnpm_install_args",
        ] {
            assert!(
                !prod.contains(pat),
                "engines.rs 生产段出现 dsh 全局安装残迹 `{pat}`——ADR-0017 已改为 \
                 project 内安装（Windows 免符号链接特权），回归会让 Windows 首启再次必败"
            );
        }
    }

    /// 正向判据：安装必须落在 `<engines>/dsh-runtime/` 且经 project 实参构造。
    /// （只判"没有全局残迹"会被"删掉安装调用"蒙混过关。）
    #[test]
    fn dsh_install_uses_project_local_path() {
        let prod = production_section();
        assert!(
            prod.contains("pnpm_project_install_args"),
            "dsh 安装必须走 project 内实参构造（ADR-0017）"
        );
        assert!(
            prod.contains("dsh-runtime"),
            "dsh 安装必须落在 <engines>/dsh-runtime/（ADR-0017 §4.1）"
        );
    }

    /// **旧布局兼容守门（ADR-0017 §5.1，硬要求）**：已装好的老用户
    /// （`global/v11/…` + pnpm 生成的 `bin/dsh`）必须**继续被判就绪**。
    ///
    /// 判据形态：就绪判定**只吃「三件在位 + 版本匹配」**，与布局无关——故在同一个
    /// 引擎目录上，「有 legacy `global/` 树」与「没有」两态的就绪结论必须**完全一致**。
    /// 这条断言在改动前后都必须绿（它是**守门**而非复现锚）；其判别力由变异证伪证明
    /// （把就绪判定改成依赖布局 ⇒ 红）。
    #[cfg(unix)]
    #[test]
    fn legacy_global_layout_still_counts_as_ready() {
        let root = engine_root("legacy-layout");
        let data_dir = root.join("data");
        std::fs::create_dir_all(engine_bin_dir(&data_dir)).unwrap();
        fake_tool(&engine_bin_dir(&data_dir), "pnpm", "12.3.1");
        fake_tool(&engine_bin_dir(&data_dir), "node", "v24.18.0");
        fake_tool(&engine_bin_dir(&data_dir), "dsh", "0.1.5-rc.2");

        let before = probe_engine(&data_dir, "");
        assert!(
            readiness_gaps(&before, "24.18.0").is_empty(),
            "基线引擎目录应判就绪"
        );

        // 模拟存量用户：pnpm 全局安装留下的 `global/v11/<64hex> -> <real>` 链接树
        let legacy = pnpm_home(&data_dir).join("global").join("v11");
        std::fs::create_dir_all(legacy.join("cfb2-18d426bbc0aba9e8-0").join("node_modules"))
            .unwrap();
        std::os::unix::fs::symlink(
            "cfb2-18d426bbc0aba9e8-0",
            legacy.join("4daac8d21495ae25ce2a440f439031ec8bd56f6a56dd5c8d07881223417c7547"),
        )
        .unwrap();

        let after = probe_engine(&data_dir, "");
        assert_eq!(
            format!("{before:?}"),
            format!("{after:?}"),
            "就绪判定不得受 legacy global/ 是否存在影响（布局无关）"
        );
        assert!(
            readiness_gaps(&after, "24.18.0").is_empty(),
            "旧布局必须继续判就绪——否则存量用户被强制重装"
        );
        assert!(
            probe_engine_if_ready(&data_dir, "").is_some(),
            "旧布局的引擎必须仍可直接进入就绪快路径"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    // ---------- ADR-0017：shim 形态 / 布局 / 幂等 ----------

    /// **POSIX shim 形态**（ADR-0017 §4.1）：`exec` 直接替换进程、node 用绝对路径、
    /// 入口指向 `dsh-runtime/.../lib/bin.js`、透传 `"$@"`。
    #[test]
    fn posix_shim_shape_is_exact() {
        let node = Path::new("/tmp/eng/bin/node");
        let entry =
            Path::new("/tmp/eng/bin/../dsh-runtime/node_modules/@deepseek-ai/dsh/lib/bin.js");
        let shim = dsh_shim_posix(node, entry);
        assert!(shim.starts_with("#!/bin/sh\n"), "必须有 shebang：{shim}");
        assert!(shim.contains("exec "), "必须 exec（不留 shell 层）：{shim}");
        assert!(
            shim.contains("\"/tmp/eng/bin/node\""),
            "node 必须绝对路径且加引号（路径可含空格）：{shim}"
        );
        assert!(
            shim.contains(
                "\"/tmp/eng/bin/../dsh-runtime/node_modules/@deepseek-ai/dsh/lib/bin.js\""
            ),
            "入口必须指向 dsh-runtime 的 lib/bin.js：{shim}"
        );
        assert!(shim.contains("\"$@\""), "必须透传参数：{shim}");
        // 不得经 PATH 解析 node（引擎目录是壳资产，不依赖用户 PATH）
        assert!(
            !shim.contains("exec node "),
            "node 不得裸名经 PATH 解析：{shim}"
        );
    }

    /// **Windows shim 形态**：ASCII + CRLF、`%~dp0` 相对定位、`%*` 透传。
    ///
    /// **目标在 ADR-0018 后改动**：由「直连 dsh 入口」改为「指向 bootstrap」
    /// （bootstrap 再选 dsh 的模块代理模式）。本断言随契约更新，但**保留**原意图中
    /// 仍然成立的部分（CRLF / 纯 ASCII / `%~dp0` 相对 / `%*` 透传）。
    #[test]
    fn windows_shim_shape_is_exact() {
        let shim = dsh_shim_windows();
        assert!(
            shim.starts_with("@echo off\r\n"),
            "首行必须是 @echo off：{shim:?}"
        );
        assert!(shim.ends_with("%*\r\n"), "必须以 %* 透传参数结尾：{shim:?}");
        assert!(
            shim.contains("\"%~dp0node.exe\""),
            "node 必须相对本 .cmd 所在目录：{shim:?}"
        );
        assert!(
            shim.contains("\"%~dp0dsh-boot.mjs\""),
            "目标必须是 bootstrap（ADR-0018）：{shim:?}"
        );
        assert!(
            !shim.contains('\n') || !shim.replace("\r\n", "").contains('\n'),
            "必须全部 CRLF：{shim:?}"
        );
        assert!(
            shim.is_ascii(),
            "shim 必须纯 ASCII（控制台代码页）：{shim:?}"
        );
    }

    /// **两平台最终指向同一入口**（原意图在 ADR-0018 后的新形态）：POSIX shim 直连
    /// dsh 入口；Windows shim 经 bootstrap 到达**同一个**入口。防一侧改了包名/入口
    /// 而另一侧漏改。
    #[test]
    fn both_platform_paths_target_the_same_relative_entry() {
        let posix = dsh_shim_posix(
            Path::new("/x/bin/node"),
            Path::new("/x/dsh-runtime/node_modules/@deepseek-ai/dsh/lib/bin.js"),
        );
        let boot = dsh_bootstrap_mjs();
        // POSIX：直连入口；Windows：经 boot（第 3 条断言保证 boot 指向入口）
        for target in [&posix, &boot.to_string()] {
            assert!(
                target.contains("dsh-runtime") && target.contains("bin.js"),
                "两平台路径都必须到达 dsh-runtime 的 bin.js：{target}"
            );
        }
        assert!(
            dsh_shim_windows().contains("dsh-boot.mjs"),
            "Windows 侧应经 bootstrap 到达入口"
        );
        // **带分隔符的完整相对路径**（独立字面量）：只查裸段 `dsh` 会被
        // `dsh-wrong` 这类错误名蒙混过关（变异证伪实测：裸段检查确实没红）。
        for expect in [
            "dsh-runtime",
            "dsh@PLACEHOLDER",
            "@deepseek-ai/dsh/lib/bin.js",
        ] {
            let expect = expect.replace("@PLACEHOLDER", "");
            assert!(posix.contains(&expect), "POSIX shim 缺 `{expect}`：{posix}");
            // Windows 侧经 bootstrap；bootstrap 用 **URL 路径**（正斜杠），故同形断言。
            assert!(boot.contains(&expect), "bootstrap 缺 `{expect}`：{boot}");
        }
    }

    /// **布局落位 + 幂等**（ADR-0017 §4.1 / 仓库"无变化零写入"纪律）：
    /// 首次落位写三件（`dsh-runtime/package.json`、`pnpm-workspace.yaml`、`bin/dsh`），
    /// 再次调用必须**零写入**（mtime 不变）。
    #[cfg(unix)]
    #[test]
    fn runtime_layout_is_laid_down_and_idempotent() {
        use std::os::unix::fs::PermissionsExt;
        let root = engine_root("adr0017-layout");
        let data_dir = root.join("data");
        std::fs::create_dir_all(engine_bin_dir(&data_dir)).unwrap();

        ensure_dsh_runtime_layout(&data_dir).unwrap();

        let rt = dsh_runtime_dir(&data_dir);
        assert!(rt.is_dir(), "必须创建 dsh-runtime/");
        let pkg = rt.join("package.json");
        assert!(pkg.is_file(), "必须有 package.json（pnpm project 前提）");
        assert!(
            std::fs::read_to_string(&pkg)
                .unwrap()
                .contains("\"private\": true"),
            "package.json 应为最小私有包声明"
        );
        // ② nodeLinker 走 build_policy 的既有 upsert（内容应为单键）
        let ws = rt.join("pnpm-workspace.yaml");
        assert!(ws.is_file(), "必须有 pnpm-workspace.yaml");
        assert!(
            std::fs::read_to_string(&ws).unwrap().contains("hoisted"),
            "必须写入 nodeLinker: hoisted"
        );
        // ③ shim 可执行
        let shim = engine_bin_dir(&data_dir).join("dsh");
        assert!(shim.is_file(), "必须有壳自写 shim");
        let mode = std::fs::metadata(&shim).unwrap().permissions().mode();
        assert_eq!(
            mode & 0o111,
            0o111,
            "shim 必须可执行（0o755），实际 {mode:o}"
        );
        assert_eq!(
            find_engine_tool(&data_dir, "dsh").as_deref(),
            Some(shim.as_path()),
            "既有查找序必须能发现 shim（不得改查找序）"
        );

        // ④ bootstrap 也必须落位（ADR-0018）
        let boot = engine_bin_dir(&data_dir).join(DSH_BOOTSTRAP_NAME);
        assert!(boot.is_file(), "必须落位 bootstrap：{}", boot.display());

        // 幂等：marker 时间戳不变 = 零写入
        let stamps: Vec<_> = [&pkg, &ws, &shim, &boot]
            .iter()
            .map(|p| std::fs::metadata(p).unwrap().modified().unwrap())
            .collect();
        std::thread::sleep(std::time::Duration::from_millis(20));
        ensure_dsh_runtime_layout(&data_dir).unwrap();
        for (p, before) in [&pkg, &ws, &shim, &boot].iter().zip(stamps) {
            assert_eq!(
                std::fs::metadata(p).unwrap().modified().unwrap(),
                before,
                "二次调用不得重写 {}（内容无变化应零写入）",
                p.display()
            );
        }
        std::fs::remove_dir_all(&root).ok();
    }

    /// **package.json 不夺权**：pnpm 会把依赖条目写进 `dsh-runtime/package.json`，
    /// 壳**不得**覆写它（否则每次安装都抹掉依赖声明，逼 pnpm 重解析）。
    #[cfg(unix)]
    #[test]
    fn runtime_layout_does_not_clobber_pnpm_managed_package_json() {
        let root = engine_root("adr0017-pkgjson");
        let data_dir = root.join("data");
        std::fs::create_dir_all(engine_bin_dir(&data_dir)).unwrap();
        ensure_dsh_runtime_layout(&data_dir).unwrap();

        let pkg = dsh_runtime_dir(&data_dir).join("package.json");
        let pnpm_managed = "{\n  \"name\": \"dsh-runtime\",\n  \"private\": true,\n  \"version\": \"0.0.0\",\n  \"dependencies\": {\n    \"@deepseek-ai/dsh\": \"0.1.5-rc.2\"\n  }\n}\n";
        std::fs::write(&pkg, pnpm_managed).unwrap();

        ensure_dsh_runtime_layout(&data_dir).unwrap();
        assert_eq!(
            std::fs::read_to_string(&pkg).unwrap(),
            pnpm_managed,
            "壳不得覆写 pnpm 维护的 package.json（依赖条目归 pnpm）"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// **空/损坏 package.json 自愈**：内容为空（或缺失）时补最小声明——
    /// 否则 pnpm project 前提不成立、安装直接失败。
    #[cfg(unix)]
    #[test]
    fn runtime_layout_repairs_blank_package_json() {
        let root = engine_root("adr0017-blank");
        let data_dir = root.join("data");
        std::fs::create_dir_all(dsh_runtime_dir(&data_dir)).unwrap();
        let pkg = dsh_runtime_dir(&data_dir).join("package.json");
        std::fs::write(&pkg, "   \n").unwrap();

        ensure_dsh_runtime_layout(&data_dir).unwrap();
        assert_eq!(
            std::fs::read_to_string(&pkg).unwrap(),
            dsh_runtime_package_json(),
            "空白 package.json 应被补成最小声明"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// **端到端（离线、hermetic）**：用**生产函数** `write_dsh_shim` 生成的 shim
    /// 真的能被执行，并把 `bin/node` 与 `dsh-runtime` 入口以**正确顺序**接起来、
    /// 原样透传参数（含带空格的参数）。
    ///
    /// 判据形态：把 `bin/node` 换成会回显参数的假 node——于是断言就落在
    /// 「shim → node → 入口 JS → 参数」这条真实链路上，不依赖真实 Node 二进制。
    #[cfg(unix)]
    #[test]
    fn generated_shim_executes_and_forwards_args() {
        use std::os::unix::fs::PermissionsExt;
        let root = engine_root("adr0017-exec");
        let data_dir = root.join("data");
        std::fs::create_dir_all(engine_bin_dir(&data_dir)).unwrap();

        // 假 node：回显「被调用的入口 + 透传参数」
        let node = engine_bin_dir(&data_dir).join("node");
        std::fs::write(
            &node,
            "#!/bin/sh\nfor a in \"$@\"; do printf '<%s>' \"$a\"; done\n",
        )
        .unwrap();
        std::fs::set_permissions(&node, std::fs::Permissions::from_mode(0o755)).unwrap();

        let shim = write_dsh_shim(&data_dir).unwrap();
        let out = crate::child_cmd(&shim)
            .arg("--version")
            .arg("arg with space")
            .output()
            .expect("shim 应可执行");
        assert!(out.status.success(), "shim 执行失败：{out:?}");
        let text = String::from_utf8_lossy(&out.stdout);
        // 期望用**独立字面量**（不经 `dsh_entry_js`）——否则断言与生产实现同源自洽，
        // 改坏实现也照样绿（qa-verify「两态等价」教训：测试必须先证明自己会鉴别）。
        const EXPECT_TAIL: &str = "dsh-runtime/node_modules/@deepseek-ai/dsh/lib/bin.js";
        assert!(
            text.starts_with(&format!("<{}>", dsh_entry_js(&data_dir).display())),
            "shim 必须把 dsh-runtime 入口作为 node 的第一个参数，实际：{text}"
        );
        assert!(
            text.contains(&format!("<{}>", dsh_entry_js(&data_dir).display()))
                && dsh_entry_js(&data_dir)
                    .to_string_lossy()
                    .ends_with(EXPECT_TAIL),
            "入口实测路径必须以 `{EXPECT_TAIL}` 结尾（独立字面量判据），实际：{text}"
        );
        // 其余参数原样透传（含空格不被拆）
        assert!(text.contains("<--version>"), "参数未透传：{text}");
        assert!(text.contains("<arg with space>"), "含空格参数被拆：{text}");
        std::fs::remove_dir_all(&root).ok();
    }

    // ---------- ADR-0018：Windows 以 proxy 模式启动（T-B4，2026-09-12） ----------

    /// **复现锚（ADR-0018）**：Windows shim 必须指向 **bootstrap**，而不是 dsh 入口。
    ///
    /// 缺陷：ADR-0017 让 dsh **装上了**，但 dsh **启动时**在
    /// `$DSH_HOME/profiles/node_modules` 建符号链接（本机实测 481 个）⇒ Windows
    /// 普通账户 `EPERM: symlink`。bootstrap 置 `process.pkg` 使 dsh 走自带的
    /// 「模块代理」分支（本机实测 **0 软链 / 1184 代理**）。
    #[test]
    fn windows_shim_points_at_bootstrap_not_entry() {
        let shim = dsh_shim_windows();
        assert!(
            shim.contains("dsh-boot.mjs"),
            "Windows shim 必须指向 bootstrap（ADR-0018）：{shim:?}"
        );
        assert!(
            !shim.contains("lib\\bin.js"),
            "Windows shim 不得再直连 dsh 入口（那会走符号链接模式 ⇒ EPERM）：{shim:?}"
        );
        assert!(
            shim.contains("\"%~dp0node.exe\"") && shim.ends_with("%*\r\n"),
            "node 相对定位与 %* 透传必须保持：{shim:?}"
        );
    }

    /// 抹掉 JS 注释（行注释 + 块注释），保留换行。
    ///
    /// **为什么必须**：bootstrap 的注释里天然会写 `process.pkg` / `runCli()` /
    /// `EPERM` 等字样（解释"为什么这么做"）。直接 `contains` 会被注释满足——
    /// 变异证伪实测：删掉 `await mod.runCli()` 后断言**仍然绿**，闸门形同虚设。
    /// 与 `lifecycle.rs` spawn 闸门「只在生产代码上判」同一条纪律。
    fn js_code_only(src: &str) -> String {
        let mut out = String::with_capacity(src.len());
        let b = src.as_bytes();
        let mut i = 0usize;
        while i < b.len() {
            if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'/' {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            } else if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    if b[i] == b'\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
                i += 2;
            } else {
                out.push(b[i] as char);
                i += 1;
            }
        }
        out
    }

    /// **bootstrap 三步齐备 + 入口相对解析 + 绊线**（ADR-0018 §4.1 / §6）。
    /// 全部断言落在**去掉注释后的代码**上（见 `js_code_only` 的理由）。
    #[test]
    fn bootstrap_has_three_steps_relative_entry_and_tripwire() {
        let code = js_code_only(dsh_bootstrap_mjs());
        // ① 选择打包体分支（断言赋值语句，不是注释里的键名）
        assert!(
            code.contains("process.pkg ??="),
            "必须实际赋值 process.pkg（dsh 的 isPackagedExecutable 判据）：{code}"
        );
        // ② 相对解析（不硬编码绝对路径）
        assert!(
            code.contains("new URL(") && code.contains("import.meta.url"),
            "入口必须相对 bootstrap 自身解析：{code}"
        );
        assert!(
            code.contains("\"../dsh-runtime/node_modules/@deepseek-ai/dsh/lib/bin.js\""),
            "入口相对路径必须精确指向 dsh 入口：{code}"
        );
        for abs in ["/Users/", "C:\\", "file:///"] {
            assert!(
                !code.contains(abs),
                "bootstrap 不得硬编码绝对路径 `{abs}`（引擎目录可搬迁）：{code}"
            );
        }
        // ③ 显式调用 runCli（断言**调用语句**；注释里的同名字样不算）
        assert!(
            code.contains("await mod.runCli();"),
            "必须实际 `await mod.runCli();`——被 import 时 import.meta.main 为假，\
             dsh 自己的 `if (import.meta.main) await runCli()` 不会执行：{code}"
        );
        // 绊线：EPERM + symlink ⇒ 可行动错误
        assert!(
            code.contains("=== \"EPERM\"") && code.contains("=== \"symlink\""),
            "必须实际判定 EPERM/symlink 并改写为可行动错误（ADR-0018 §6）：{code}"
        );
        assert!(
            code.contains("process.exit(1)") && code.contains("throw err"),
            "绊线必须非零退出、其余错误上抛（不得静默吞掉）：{code}"
        );
    }

    /// **分叉面守门（ADR-0018 §4.2）**：POSIX shim **逐字冻结**——那里建软链无特权问题，
    /// 改它只会白 churn 481 个链接并偏离上游默认。
    ///
    /// 本断言在改动前后都绿（**守门**而非复现锚）；判别力由变异证伪证明。
    #[test]
    fn posix_shim_is_frozen_byte_for_byte() {
        const FROZEN: &str = concat!(
            "#!/bin/sh\n",
            "# DSH Dock engine launcher (ADR-0017: shell-owned shim, not the pnpm global shim).\n",
            "exec \"/x/bin/node\" \"/x/dsh-runtime/node_modules/@deepseek-ai/dsh/lib/bin.js\" \"$@\"\n",
        );
        assert_eq!(
            dsh_shim_posix(
                Path::new("/x/bin/node"),
                Path::new("/x/dsh-runtime/node_modules/@deepseek-ai/dsh/lib/bin.js")
            ),
            FROZEN,
            "POSIX shim 被改动了——ADR-0018 §4.2 要求它一律不动（无特权问题）"
        );
    }

    // ---------- ADR-0018 核心判据：真实 dsh 以 bootstrap 启动 ⇒ profiles/node_modules 零软链 ----------

    /// 本机可能的引擎目录（只**读**，用于真机判据定位真实 dsh 包与 node）。
    ///
    /// 不碰用户 `~/.dsh`（DSH_HOME）：这里找的是**引擎资产目录**，boot 本身仍锁在
    /// 临时 `DSH_HOME`（AGENTS §6 的约束针对 home/会话锁，不是引擎包）。
    #[cfg(unix)]
    fn real_engine_roots() -> Vec<PathBuf> {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let mut roots = Vec::new();
        if let Ok(explicit) = std::env::var("DSH_DOCK_ENGINES") {
            roots.push(PathBuf::from(explicit));
        }
        roots.push(PathBuf::from(&home).join(".dsh-dock-test").join("engines"));
        // macOS：app_data 的 bundle identifier
        roots.push(
            PathBuf::from(&home)
                .join("Library/Application Support/io.github.realguan.dsh-dock/engines"),
        );
        roots
    }

    /// 本机真实 dsh 包目录（含 `lib/bin.js`）：新布局或旧 `global/` 布局均可。
    #[cfg(unix)]
    fn real_dsh_package() -> Option<PathBuf> {
        for engines in real_engine_roots() {
            let direct = engines.join("dsh-runtime/node_modules/@deepseek-ai/dsh");
            if direct.join("lib/bin.js").is_file() {
                return Some(direct);
            }
            if let Ok(rd) = std::fs::read_dir(engines.join("global/v11")) {
                for e in rd.flatten() {
                    let p = e.path().join("node_modules/@deepseek-ai/dsh");
                    if p.join("lib/bin.js").is_file() {
                        return Some(std::fs::canonicalize(&p).unwrap_or(p));
                    }
                }
            }
        }
        None
    }

    /// 本机引擎 node（真机判据用）。
    #[cfg(unix)]
    fn real_engine_node() -> Option<PathBuf> {
        real_engine_roots()
            .into_iter()
            .map(|e| e.join("bin/node"))
            .find(|p| p.is_file())
    }

    /// **ADR-0018 §6 核心判据（实现无关的结果判据，比内容断言更强）**：
    /// 用**真实 dsh** 在**临时 DSH_HOME** 里以 bootstrap 方式 boot，断言
    /// `profiles/node_modules` **零符号链接**且 dsh 能起。
    ///
    /// `#[ignore]`：需要本机已装引擎（真实 dsh），与
    /// `lifecycle::tests::real_dsh_is_reaped_when_shell_is_sigkilled` 同一口径。
    /// 运行：`cargo test --lib -- --ignored real_dsh_boots_in_proxy_mode --nocapture`。
    ///
    /// **安全**：一律锁进临时 `DSH_HOME` + 临时引擎目录，**不碰用户真实 `~/.dsh`**（AGENTS §6）。
    #[cfg(unix)]
    #[ignore = "需要本机已装引擎（真实 dsh 包）"]
    #[test]
    fn real_dsh_boots_in_proxy_mode() {
        let Some(real_dsh) = real_dsh_package() else {
            eprintln!("跳过：本机未找到真实 dsh 包（新布局或旧 global 布局）");
            return;
        };
        let root = engine_root("adr0018-real");
        let data_dir = root.join("data");
        let bin = engine_bin_dir(&data_dir);
        std::fs::create_dir_all(&bin).unwrap();

        let Some(node_src) = real_engine_node() else {
            eprintln!("跳过：未找到引擎 node");
            return;
        };
        std::os::unix::fs::symlink(&node_src, bin.join("node")).unwrap();

        // 临时 dsh-runtime：把真实包挂进来（bootstrap 按相对路径解析到它）
        let rt_pkg = dsh_runtime_dir(&data_dir)
            .join("node_modules")
            .join("@deepseek-ai");
        std::fs::create_dir_all(&rt_pkg).unwrap();
        std::os::unix::fs::symlink(&real_dsh, rt_pkg.join("dsh")).unwrap();

        // 用**生产函数**落位 bootstrap（不是手抄副本）
        let boot = write_dsh_bootstrap(&data_dir).unwrap();
        assert!(boot.is_file());

        let home = root.join("dshhome");
        std::fs::create_dir_all(&home).unwrap();
        let mut cmd = crate::child_cmd(&bin.join("node"));
        cmd.arg(&boot)
            .arg("web")
            .arg("--no-open")
            .env("DSH_HOME", &home)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut child = crate::lifecycle::spawn(
            &mut cmd,
            crate::lifecycle::Role::Probe,
            crate::lifecycle::GuardCtx::of("dsh-boot", None),
        )
        .expect("应能启动 bootstrap");

        // 轮询等待 dsh 铺设完成（最多 ~20s）
        // 等**铺设稳定**（不再是「首个非空采样」——那可能只铺到一半，断言会偏弱）
        let nm = home.join("profiles").join("node_modules");
        let mut entries = 0usize;
        let mut stable = 0;
        for _ in 0..60 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let now = nm.read_dir().map(|d| d.count()).unwrap_or(0);
            if now > 0 && now == entries {
                stable += 1;
                if stable >= 2 {
                    break;
                }
            } else {
                stable = 0;
            }
            entries = now;
        }
        let alive = child.try_wait().map(|s| s.is_none()).unwrap_or(false);
        let _ = child.kill();
        let _ = child.wait();

        assert!(
            nm.is_dir(),
            "dsh 未铺设 profiles/node_modules（boot 失败？）"
        );
        assert!(entries > 0, "profiles/node_modules 为空（boot 未完成铺设）");
        let links: Vec<PathBuf> = std::fs::read_dir(&nm)
            .unwrap()
            .flatten()
            .filter(|e| e.file_type().map(|t| t.is_symlink()).unwrap_or(false))
            .map(|e| e.path())
            .collect();
        let proxies = count_files_named(&nm, "entry-");
        eprintln!(
            "真实 dsh boot：软链={} 顶层条目={} 代理 entry-*.js={} 进程存活={alive}",
            links.len(),
            entries,
            proxies
        );
        assert!(
            links.is_empty(),
            "bootstrap 模式下仍出现 {} 个符号链接（前 3：{:?}）——proxy 模式未被选中",
            links.len(),
            links.iter().take(3).collect::<Vec<_>>()
        );
        assert!(
            proxies > 0,
            "未发现目录代理 entry-*.js ⇒ 选的不是 proxy 分支（ADR-0018 未生效）"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// 递归数出以 `prefix` 开头的文件（用于数 `entry-*.js` 代理）。
    #[cfg(unix)]
    fn count_files_named(dir: &Path, prefix: &str) -> usize {
        let mut n = 0usize;
        let Ok(rd) = std::fs::read_dir(dir) else {
            return 0;
        };
        for e in rd.flatten() {
            let p = e.path();
            match e.file_type() {
                Ok(t) if t.is_dir() => n += count_files_named(&p, prefix),
                Ok(t)
                    if t.is_file()
                        && p.file_name()
                            .and_then(|s| s.to_str())
                            .is_some_and(|s| s.starts_with(prefix)) =>
                {
                    n += 1
                }
                _ => {}
            }
        }
        n
    }

    /// 从 Windows shim 文本里**解析**出 `%~dp0` 之后的目标文件名（不经常量）。
    ///
    /// 刻意用「解析 shim 文本」而不是「读常量」：这样断言的是**shim 真正指向的东西**，
    /// 而不是我们以为它指向的东西——若将来有人把字面量写回 shim，解析出的名字就会
    /// 与落盘文件名不符，测试即红。
    fn windows_shim_target_name(shim: &str) -> String {
        // shim 形如 `"%~dp0node.exe" "%~dp0<NAME>" %*`：取**第 2 个** `%~dp0` 之后
        // 到下一个 `"` 之间的内容。
        let mut rest = shim;
        for _ in 0..2 {
            let i = rest
                .find("%~dp0")
                .unwrap_or_else(|| panic!("shim 应含第 2 个 %~dp0：{shim:?}"));
            rest = &rest[i + "%~dp0".len()..];
        }
        rest.split('"')
            .next()
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| panic!("shim 的 %~dp0 之后应紧跟目标名：{shim:?}"))
            .to_string()
    }

    /// **task-61 / qa-verify 变异 M4**：Windows shim 指向的文件名，必须**就是**
    /// `write_dsh_bootstrap` 实际写出的那个文件。
    ///
    /// 缺陷史：`DSH_BOOTSTRAP_NAME`（写文件用）与 shim 里的硬编码字面量 `dsh-boot.mjs`
    /// **没有任何绑定** ⇒ 只改常量名，**408 条测试全绿**，而 Windows 上 `dsh.cmd` 会指向
    /// 不存在的文件、启动失败（静默、且只在 Windows 爆）。
    ///
    /// 本断言取**最强形态**：① 名字经常量在**构造上**生成（见 `dsh_shim_windows`）；
    /// ② 再从 shim **文本里解析**出目标名，断言该名对应的文件确实被落位。
    /// 两层都不是"改红了就删"——原有意图（CRLF / ASCII / `%~dp0node.exe` / `%*`）全保留。
    #[cfg(unix)]
    #[test]
    fn windows_shim_target_is_the_file_we_actually_write() {
        let shim = dsh_shim_windows();

        // ① 构造上单一来源：目标名 = 常量
        assert!(
            shim.contains(&format!("\"%~dp0{DSH_BOOTSTRAP_NAME}\"")),
            "shim 目标必须由 DSH_BOOTSTRAP_NAME 生成：{shim:?}"
        );

        // ② 端到端：从 shim 文本解析出目标名 → 该名的文件必须真的被落位
        let target = windows_shim_target_name(&shim);
        assert_eq!(
            target, DSH_BOOTSTRAP_NAME,
            "shim 解析出的目标名与落盘常量不一致——正是 M4 那个静默失效"
        );
        let root = engine_root("adr0018-binding");
        let data_dir = root.join("data");
        std::fs::create_dir_all(engine_bin_dir(&data_dir)).unwrap();
        ensure_dsh_runtime_layout(&data_dir).unwrap();
        let on_disk = engine_bin_dir(&data_dir).join(&target);
        assert!(
            on_disk.is_file(),
            "Windows shim 指向 `{target}`，但该文件并未被落位（实际落盘：{:?}）\
             ——改名常量就会触发这条，正是 M4",
            std::fs::read_dir(engine_bin_dir(&data_dir))
                .map(|rd| rd.flatten().map(|e| e.file_name()).collect::<Vec<_>>())
        );
        std::fs::remove_dir_all(&root).ok();
    }

    /// 解析器自检：换一个名字也必须解析得对（防"解析器只认写死的一个名字"）。
    #[test]
    fn windows_shim_target_parser_follows_the_name() {
        let fake = "@echo off\r\n\"%~dp0node.exe\" \"%~dp0totally-other.mjs\" %*\r\n";
        assert_eq!(windows_shim_target_name(fake), "totally-other.mjs");
    }
}
