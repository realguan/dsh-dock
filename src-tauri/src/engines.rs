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

/// 引擎 bin 目录：捆绑 pnpm、node shim、`add -g` 的全局命令 shim 都在这里。
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

/// 引擎 dsh 启动器（`pnpm add -g` 全局 shim：Unix shebang 脚本 / Windows .cmd，
/// 可直接执行，node 经 PATH 解析——不必再深挖 pnpm 全局树取 lib/bin.js）。
pub fn engine_dsh_bin(data_dir: &Path) -> Option<PathBuf> {
    find_engine_tool(data_dir, "dsh")
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
fn find_engine_tool(data_dir: &Path, name: &str) -> Option<PathBuf> {
    let dir = engine_bin_dir(data_dir);
    let exts: &[&str] = if cfg!(windows) {
        &[".exe", ".cmd", ""]
    } else {
        &[""]
    };
    exts.iter()
        .map(|ext| dir.join(format!("{name}{ext}")))
        .find(|p| p.is_file())
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

/// 解析 pnpm 非 TTY 安装进度行（`add -g` 的 default reporter），如
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
pub fn stage_pnpm_from_bundle(bundle: &Path, data_dir: &Path) -> Result<PathBuf> {
    let dest = engine_pnpm_bin(data_dir);
    std::fs::create_dir_all(engine_bin_dir(data_dir))
        .with_context(|| format!("创建引擎 bin 目录 {}", engine_bin_dir(data_dir).display()))?;
    let member = if cfg!(windows) {
        "package/pnpm.exe"
    } else {
        "package/pnpm"
    };
    let tmp = pnpm_home(data_dir).join("stage-tmp");
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
        bail!(
            "tar 解包 pnpm 失败：{}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let extracted = tmp.join(member);
    std::fs::rename(&extracted, &dest)
        .or_else(|_| std::fs::copy(&extracted, &dest).map(|_| ()))
        .with_context(|| format!("落位 {}", dest.display()))?;
    let _ = std::fs::remove_dir_all(&tmp);
    Ok(dest)
}

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
    args: &[String],
    extra_env: &[(String, String)],
    on_line: &mut dyn FnMut(&str),
) -> Result<()> {
    let mut cmd = crate::child_cmd(&engine_pnpm_bin(data_dir));
    cmd.args(args)
        .current_dir(pnpm_home(data_dir))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in pnpm_process_env(data_dir, path_env) {
        cmd.env(k, v);
    }
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    // 守卫式 spawn（ADR-0015）：`runtime set node` / `add -g` 是**分钟级**动作，
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
                tracing::info!("node v{version} 就位（{progress_lines} 行下载进度）");
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

/// 查找 pnpm runtime set 下载并解包的实际 Node 二进制文件。
pub fn find_runtime_node_bin(data_dir: &Path) -> Option<PathBuf> {
    let base = pnpm_home(data_dir).join("node_modules");
    let name = if cfg!(windows) { "node.exe" } else { "node" };
    // 1. 标准软链路径：node_modules/node/bin/node
    let direct = base.join("node").join("bin").join(name);
    if direct.is_file() {
        return Some(direct);
    }
    // 2. 虚拟 store 扫描：node_modules/.pnpm/node@runtime+*/node_modules/node/bin/node
    let pnpm_dir = base.join(".pnpm");
    if let Ok(entries) = std::fs::read_dir(&pnpm_dir) {
        for entry in entries.flatten() {
            let n = entry.file_name();
            if n.to_string_lossy().starts_with("node@runtime") {
                let cand = entry
                    .path()
                    .join("node_modules")
                    .join("node")
                    .join("bin")
                    .join(name);
                if cand.is_file() {
                    return Some(cand);
                }
            }
        }
    }
    None
}

/// 固化实际 Node 二进制到 `PNPM_HOME/bin/node`。
/// 原因：pnpm v12 的 `pnpm shim add node` 生成的是带有上下文检查的 shim dispatcher，
/// 全局安装（pnpm add -g）执行 postinstall 脚本（如 protobufjs/koffi）时，
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

/// `pnpm add -g @deepseek-ai/dsh@<version>`：registry 镜像链逐个尝试
///（allow-build 放行沿 ADR-0009/0005 同一口径）。非 TTY 安装进度行
///（`Progress: resolved N, … downloaded M`）经回调上抛（阶段 = Dsh，
/// 包计数）。
pub fn install_dsh_global(
    data_dir: &Path,
    version: &str,
    path_env: &str,
    progress: &mut dyn FnMut(crate::updates::ProgressStage, u64, Option<u64>),
) -> Result<()> {
    let _ = link_real_node_binary(data_dir);
    let mut errors = Vec::new();
    for registry in crate::updates::registry_chain() {
        let args =
            crate::updates::pnpm_install_args(&registry, &format!("@deepseek-ai/dsh@{version}"));
        tracing::info!("pnpm add -g @deepseek-ai/dsh@{version}（registry {registry}）…");
        let mut progress_lines = 0usize;
        let mut last: Option<(u64, u64)> = None;
        let result = run_engine_pnpm_streaming(data_dir, path_env, &args, &[], &mut |line| {
            if let Some((done, total)) = parse_package_progress(line) {
                if progress_lines == 0 {
                    tracing::info!("dsh 包下载中（进度经 boot:progress 实时推进）");
                }
                progress_lines += 1;
                last = Some((done, total));
                progress(crate::updates::ProgressStage::Dsh, done, Some(total));
            }
        });
        match result {
            Ok(()) => {
                tracing::info!("dsh v{version} 就位（{progress_lines} 行安装进度）");
                // 关单：末条 Progress 行可能停在 downloaded < resolved，成功
                // 返回即全部就位，按满额补发一次完成事件。
                if let Some((_, total)) = last {
                    progress(crate::updates::ProgressStage::Dsh, total, Some(total));
                }
                return Ok(());
            }
            Err(e) => {
                tracing::warn!("dsh 引导安装（{registry}）失败：{e}");
                errors.push(format!("{registry}: {e}"));
            }
        }
    }
    Err(anyhow!(
        "dsh 引导安装失败（registry 均不可达）：{}",
        errors.join("；")
    ))
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
        tracing::info!("dsh 目标版本：{dsh_version}，开始全局安装…");
        install_dsh_global(data_dir, &dsh_version, path_env, progress)?;
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
}
