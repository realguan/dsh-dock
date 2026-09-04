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
    let out = cmd.output().ok()?;
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
    let out = tar.output().context("执行系统 tar 解包 pnpm 失败")?;
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
    let out = cmd.output().context("spawn 引擎 pnpm 失败")?;
    if !out.status.success() {
        bail!(
            "pnpm {} 失败：{}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

/// `pnpm runtime set node <version>`：镜像链（npmmirror → 官方）逐个尝试；
/// 非 TTY 字节进度行经回调上抛（映射 boot:progress）。cwd = 引擎目录——
/// runtime set 为项目作用域，单目录方案恰好把 node 装进引擎（spike 0003 §2.2）。
pub fn runtime_set_node(
    data_dir: &Path,
    version: &str,
    path_env: &str,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<()> {
    let mut errors = Vec::new();
    for base in [NODE_MIRROR_PRIMARY, NODE_MIRROR_FALLBACK] {
        let pnpm = engine_pnpm_bin(data_dir);
        let mut cmd = crate::child_cmd(&pnpm);
        cmd.args(["runtime", "set", "node", version])
            .current_dir(pnpm_home(data_dir))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (k, v) in pnpm_process_env(data_dir, path_env) {
            cmd.env(k, v);
        }
        let (mk, mv) = node_mirrors_env(base);
        cmd.env(mk, mv);
        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("{base}: spawn 失败 {e}"));
                continue;
            }
        };
        if let Some(stdout) = child.stdout.take() {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if let Some(p) = parse_download_progress(&line) {
                    progress(p.downloaded, Some(p.total));
                }
            }
        }
        let output = child.wait_with_output()?;
        if output.status.success() {
            return Ok(());
        }
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
        errors.push(format!("{base}: {detail}"));
        tracing::warn!("runtime set node（{base}）失败：{detail}");
    }
    Err(anyhow!(
        "node 引导失败（镜像均不可达）：{}",
        errors.join("；")
    ))
}

/// `pnpm shim add node`：激活 node 到 PNPM_HOME/bin（硬链，spike 0003 §2.2）。
pub fn shim_add_node(data_dir: &Path, path_env: &str) -> Result<()> {
    run_engine_pnpm(
        data_dir,
        path_env,
        &["shim".to_string(), "add".to_string(), "node".to_string()],
        &[],
    )
}

/// `pnpm add -g @deepseek-ai/dsh@<version>`：registry 镜像链逐个尝试
///（allow-build 放行沿 ADR-0009/0005 同一口径）。
pub fn install_dsh_global(data_dir: &Path, version: &str, path_env: &str) -> Result<()> {
    let mut errors = Vec::new();
    for registry in crate::updates::package_registry_bases() {
        let args =
            crate::updates::pnpm_install_args(registry, &format!("@deepseek-ai/dsh@{version}"));
        match run_engine_pnpm(data_dir, path_env, &args, &[]) {
            Ok(()) => return Ok(()),
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
pub fn bootstrap(
    data_dir: &Path,
    path_env: &str,
    pnpm_bundle: &Path,
    node_version: &str,
    dsh_version: &str,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<BootstrapOutcome> {
    // ① pnpm 随壳 pin：每次 boot 重铺（幂等覆盖，版本不再参与判定）
    stage_pnpm_from_bundle(pnpm_bundle, data_dir)?;

    // ② 就绪判定 → 逐项补缺 → 重探
    let mut status = probe_engine(data_dir, path_env);
    let mut node_switched = false;
    let mut dsh_installed = false;
    for gap in readiness_gaps(&status, node_version) {
        match gap {
            EngineGap::Pnpm => unreachable!("staging 已覆盖 pnpm 落位"),
            EngineGap::Node { found } => {
                runtime_set_node(data_dir, node_version, path_env, progress).map_err(|e| {
                    anyhow!(
                        "node 引导失败（{}）：{e}",
                        found
                            .map(|v| format!("现 {v}"))
                            .unwrap_or_else(|| "缺失".to_string())
                    )
                })?;
                shim_add_node(data_dir, path_env)?;
                node_switched = true;
            }
            EngineGap::Dsh => {
                install_dsh_global(data_dir, dsh_version, path_env)?;
                dsh_installed = true;
            }
        }
        status = probe_engine(data_dir, path_env);
    }

    // ③ 终验：三件缺一不可（首启必须联网；之后离线可启动）
    let gaps = readiness_gaps(&status, node_version);
    if !gaps.is_empty() {
        bail!("引擎就绪判定未通过（{gaps:?}）——首启需联网完成引导，之后可离线启动");
    }
    Ok(BootstrapOutcome {
        status,
        node_switched,
        dsh_installed,
    })
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

        let outcome =
            bootstrap(&data_dir, "", &bundle, "24.18.0", "0.1.1", &mut |_, _| {}).unwrap();
        assert_eq!(outcome.status.pnpm.as_deref(), Some("12.3.1"));
        assert!(!outcome.node_switched);
        assert!(!outcome.dsh_installed);
        assert!(readiness_gaps(&outcome.status, "24.18.0").is_empty());
        std::fs::remove_dir_all(&root).ok();
    }
}
