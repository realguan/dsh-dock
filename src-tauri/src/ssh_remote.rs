//! ssh_remote.rs —— SSH 远程工作区的**目标校验与非交互预检**（2026-09-15，ADR-0023 §2.3–§2.5）。
//!
//! ## 职责
//!
//! 把「用户填的五个键是否合法」与「远端到底能不能用」分开：
//! - [`validate_target`] —— **纯函数**，与 `@deepseek-ai/dsh-ssh` 的**运行时**校验同口径
//!   （`packages/ssh/ssh/src/index.ts:82-90` 的 zod schema，比 cordis/schemastery 那层更严）；
//! - [`probe_ssh_target`] —— 以 `ssh -o BatchMode=yes` 做**一次**非交互往返，回读
//!   远端 `uname` / node / helper / workspace / helper 摘要。
//!
//! ## 为什么必须非交互（ADR-0023 §1.4）
//!
//! 上游口径是"服务启用 `BatchMode`、强制严格 host-key 校验、**禁用 agent forwarding**、
//! 不提供任何交互式认证流程"。本预检据此把参数锁死（见 [`probe_ssh_args`]）：
//! 任何一项落到用户 `~/.ssh/config` 的宽松设置上，都会把"向导"变成"替用户跑一次
//! 可能弹密码框的 ssh"——而壳没有终端，弹框即挂死。
//!
//! ## 预检失败 = **拒绝生成**（ADR-0023 §2.5）
//!
//! 不可达时**不生成 profile**：生成一个注定启动失败的 profile，比不生成更糟——
//! 用户会以为是 dsh 的问题。这是本模块与"仅存配置"形态（§3 方案 F 已否决）的分界。
//!
//! ## 远端脚本：一次往返、标签化输出、纯函数解析
//!
//! 三次 `ssh` 往返会让"有界"变得难说清，也更容易被中途的交互提示卡住。本模块用**一条**
//! 远端 `sh -c` 脚本输出 `key=value` 行，再由 [`parse_probe_output`]（纯函数）解析——
//! 于是"输出理解"这一最容易出错的部分可单测，而真实往返只留给实机清单。

use std::io::Read;
use std::path::Path;
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::guest::sh_quote;

/// `dsh-ssh` 的五个必填配置键（ADR-0023 §2.3；**不存在** `target` / `remoteNodePath` 选项）。
///
/// 实现 `Deserialize`（camelCase）以便整体作为一条 IPC 参数传递：向导的表单就是一个
/// 对象，拆成五个平铺参数会让"哪个字段漏传"变成运行期才发现的事。
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTarget {
    /// **已存在的 OpenSSH alias**（含其既有 user/key/known_hosts 配置），不是任意 hostname。
    pub host: String,
    /// 远端 Node 可执行文件的**绝对**路径。
    pub node: String,
    /// 已安装的 helper 入口的**绝对**路径。
    pub helper: String,
    /// helper 的 SHA-256（**小写** 64 位十六进制）；不符即拒绝连接。
    pub helper_hash: String,
    /// 远端默认工作区的**绝对**路径。
    pub workspace: String,
}

/// helper 摘要格式：`/^[0-9a-f]{64}$/`（与上游 zod 同口径，**只认小写**）。
pub fn is_sha256_hex(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// host alias 格式：`/^[a-zA-Z0-9][a-zA-Z0-9_.@-]*$/`（与上游 zod 同口径）。
pub fn is_valid_host_alias(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphanumeric() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '@' | '-'))
}

/// 校验五个键，返回**全部**问题（空 = 通过）。
///
/// 一次给全部问题而非首个：向导要一次把表单标完，逐个报错会让用户来回五轮。
pub fn validate_target(t: &SshTarget) -> Vec<String> {
    let mut problems = Vec::new();

    if t.host.is_empty() {
        problems.push("host 不能为空（它是 `~/.ssh/config` 里已存在的别名）".to_string());
    } else if !is_valid_host_alias(&t.host) {
        problems.push(format!(
            "host「{}」不合法：只允许字母/数字开头，后接字母、数字、`_`、`.`、`@`、`-`",
            t.host
        ));
    }

    for (label, value) in [
        ("node", &t.node),
        ("helper", &t.helper),
        ("workspace", &t.workspace),
    ] {
        if value.is_empty() {
            problems.push(format!("{label} 不能为空"));
        } else if !value.starts_with('/') {
            // 上游 zod 是 `startsWith('/')`：只认 **POSIX 绝对路径**，
            // 相对路径在远端会以 ssh 会话的 cwd 解析，结果不可预期。
            problems.push(format!("{label} 必须是绝对路径（以 `/` 开头）：{value}"));
        }
    }

    if t.helper_hash.is_empty() {
        problems.push("helperHash 不能为空".to_string());
    } else if !is_sha256_hex(&t.helper_hash) {
        problems.push(
            "helperHash 必须是小写 64 位十六进制 SHA-256（大写或非十六进制字符都会被拒绝）"
                .to_string(),
        );
    }

    problems
}

/// 宿主是否支持 SSH 远程工作区（ADR-0023 §1.4：**两端**须 Linux/macOS）。
///
/// 上游在构造 `SshConnection` 时就会 `throw new Error('SSH runtime requires a POSIX client')`
/// （`packages/ssh/ssh/src/index.ts:79-80`），故这不是壳的额外限制，而是**上游事实**——
/// 壳只是把它提前到向导第一步，免得用户填完一张表才发现。
///
/// 用 `cfg!()` 而非 `#[cfg]`：Windows 分支因此**在所有目标上都被编译与 lint**，
/// 不会躲过 CI（AGENTS §1 的教训）。
pub const fn host_supports_ssh() -> bool {
    !cfg!(windows)
}

/// 非交互预检的 ssh 参数（ADR-0023 §1.4 / §2.5）。
///
/// 四项锁定，全部有出处：
/// - `BatchMode=yes` —— 禁止任何交互式认证提示（壳没有终端，弹提示即挂死）；
/// - `ForwardAgent=no` —— 明文要求禁用 agent forwarding，**不依赖**用户配置里恰好没开；
/// - `StrictHostKeyChecking=yes` —— 强制严格 host-key 校验（不自动接受未知主机）；
/// - `ConnectTimeout` —— 有界的连接阶段上限（整轮超时另见 [`probe_ssh_target`]）。
///
/// **刻意不传 `-F`**：alias 就是从用户默认配置里读出来的，让 ssh 读同一份配置才对得上；
/// 传 `-F` 会让"从哪个文件读别名"与"用哪个文件连接"变成两个真相源。
///
/// 远端命令整体经 [`sh_quote`] 单引号进参：`ssh` 把远端 argv **按空格拼接**下发，
/// 不经本地 shell 转义——不自己quote 的话，带空格的路径会在远端被重新切分。
pub fn probe_ssh_args(target: &SshTarget) -> Vec<String> {
    let remote = format!(
        "sh -c {} probe {} {} {}",
        sh_quote(PROBE_SCRIPT),
        sh_quote(&target.node),
        sh_quote(&target.helper),
        sh_quote(&target.workspace),
    );
    vec![
        "-o".to_string(),
        "BatchMode=yes".to_string(),
        "-o".to_string(),
        "ForwardAgent=no".to_string(),
        "-o".to_string(),
        "StrictHostKeyChecking=yes".to_string(),
        "-o".to_string(),
        "ConnectTimeout=10".to_string(),
        "--".to_string(),
        target.host.clone(),
        remote,
    ]
}

/// 远端体检脚本（POSIX sh；`$1`=node、`$2`=helper、`$3`=workspace）。
///
/// 输出 `key=value` 若干行，未知行由解析器忽略。`sha256sum` 与 `shasum` 两个名字都要试
/// ——Linux 有前者，macOS 只有后者，而 ADR-0023 §1.4 明说**两端**都可能是 Linux 或 macOS。
pub const PROBE_SCRIPT: &str = r#"
p(){ printf '%s=%s\n' "$1" "$2"; }
p os "$(uname -s 2>/dev/null)"
if [ -x "$1" ]; then p node_ok yes; p node_ver "$("$1" --version 2>/dev/null | head -n 1)"; else p node_ok no; fi
if [ -f "$2" ]; then p helper_ok yes; else p helper_ok no; fi
if command -v sha256sum >/dev/null 2>&1; then h=$(sha256sum -- "$2" 2>/dev/null | cut -d" " -f1)
elif command -v shasum >/dev/null 2>&1; then h=$(shasum -a 256 -- "$2" 2>/dev/null | cut -d" " -f1)
else h=; fi
p helper_hash "$h"
if [ -d "$3" ]; then p workspace_ok yes; else p workspace_ok no; fi
"#;

/// 解析预检输出（**纯函数**）：只认 `key=value`，忽略不属于本协议的噪声行。
///
/// 忽略噪声是必要的：ssh 会把 banner / motd / 远端 shell 的告警混进 stdout。
/// 把它们当失败会让"能连上"被误判成"连不上"。
pub fn parse_probe_output(text: &str) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    for raw in text.lines() {
        let line = raw.trim_end_matches('\r').trim();
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        if k.is_empty() || k.contains(char::is_whitespace) {
            continue;
        }
        out.insert(k.to_string(), v.trim().to_string());
    }
    out
}

/// 一项体检结论。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshProbeCheck {
    /// 稳定的机器可读键（UI 用它做分组/图标，不解析文案）。
    pub key: String,
    pub ok: bool,
    /// 人能读的一句结论（含实际观测值）。
    pub detail: String,
}

/// 一次预检的结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshProbe {
    /// 全部检查是否通过。**false 即不得生成 profile**（ADR-0023 §2.5）。
    pub ok: bool,
    pub checks: Vec<SshProbeCheck>,
}

/// 把标签化输出评成体检结论（**纯函数**，可单测）。
///
/// 每一项都带上**实际观测值**：只说"不通过"等于让用户猜；
/// 而"helperHash 不符"与"helper 不存在"的处置完全不同。
pub fn assess_probe(
    report: &std::collections::BTreeMap<String, String>,
    configured_hash: &str,
) -> SshProbe {
    let mut checks = Vec::new();
    let mut push = |key: &str, ok: bool, detail: String| {
        checks.push(SshProbeCheck {
            key: key.to_string(),
            ok,
            detail,
        });
    };

    // ① 平台：两端须 Linux/macOS（ADR-0023 §1.4）。
    let os = report.get("os").cloned().unwrap_or_default();
    push(
        "platform",
        os == "Linux" || os == "Darwin",
        match os.as_str() {
            "" => "未取到远端 `uname -s`（远端 shell 可能不是 POSIX sh）".to_string(),
            other => format!("远端 uname -s = {other}（须为 Linux 或 Darwin）"),
        },
    );

    // ② node 可执行。
    let node_ok = report.get("node_ok").map(String::as_str) == Some("yes");
    let node_ver = report.get("node_ver").cloned().unwrap_or_default();
    push(
        "node",
        node_ok,
        if node_ok {
            format!("远端 node：{node_ver}")
        } else {
            "远端 node 路径不存在或不可执行".to_string()
        },
    );

    // ③ helper 存在。
    let helper_ok = report.get("helper_ok").map(String::as_str) == Some("yes");
    push(
        "helper",
        helper_ok,
        if helper_ok {
            "远端 helper 存在".to_string()
        } else {
            "远端 helper 路径不存在（须先把 helper 部署到 workspace 与可写临时根之外）".to_string()
        },
    );

    // ④ helper 摘要与配置一致——**不符即拒绝连接**（上游同口径）。
    let remote_hash = report.get("helper_hash").cloned().unwrap_or_default();
    let hash_ok = !remote_hash.is_empty() && remote_hash == configured_hash;
    push(
        "helperHash",
        hash_ok,
        if remote_hash.is_empty() {
            "未取到远端摘要（远端既无 `sha256sum` 也无 `shasum`）".to_string()
        } else if hash_ok {
            "helper 摘要与配置一致".to_string()
        } else {
            format!("helper 摘要不符：远端 {remote_hash}，配置 {configured_hash}")
        },
    );

    // ⑤ workspace。
    let ws_ok = report.get("workspace_ok").map(String::as_str) == Some("yes");
    push(
        "workspace",
        ws_ok,
        if ws_ok {
            "远端 workspace 目录存在".to_string()
        } else {
            "远端 workspace 不是已存在的目录".to_string()
        },
    );

    let ok = checks.iter().all(|c| c.ok);
    SshProbe { ok, checks }
}

/// 以一次非交互往返预检远端（阻塞；调用方负责放进 `spawn_blocking`）。
///
/// `timeout` 是**整轮**上限（含 ssh 建连与远端脚本），不是每步上限。
pub fn probe_ssh_target(target: &SshTarget, timeout: Duration) -> Result<SshProbe, String> {
    let problems = validate_target(target);
    if !problems.is_empty() {
        return Err(format!("配置校验未通过：{}", problems.join("；")));
    }
    if !host_supports_ssh() {
        return Err(
            "SSH 远程工作区需要 POSIX 客户端（Linux/macOS）：上游 `dsh-ssh` 在非 POSIX 宿主上\
             直接抛 `SSH runtime requires a POSIX client`（ADR-0023 §1.4）"
                .to_string(),
        );
    }

    let args = probe_ssh_args(target);
    // 成功路径上 stderr 通常为空（告警也已被 `StrictHostKeyChecking=yes` 挡掉）；
    // 失败路径的诊断在 `run_ssh` 的 `Err` 里，这里无需再看。
    let (stdout, _stderr) = run_ssh(&args, timeout)?;
    Ok(assess_probe(
        &parse_probe_output(&stdout),
        &target.helper_hash,
    ))
}

/// 跑一次 `ssh` 并回读 stdout/stderr（有界）。
///
/// stderr 单开线程排空：ssh 的失败诊断（`Permission denied (publickey)` 等）是
/// **归因的全部价值**所在，不能为了省事丢掉；而只 piped 不读会在管道写满时把子进程挂住。
fn run_ssh(args: &[String], timeout: Duration) -> Result<(String, String), String> {
    let mut cmd = crate::child_cmd(Path::new("ssh"));
    cmd.args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let mut child = crate::lifecycle::spawn(
        &mut cmd,
        crate::lifecycle::Role::Probe,
        crate::lifecycle::GuardCtx::of("ssh-probe", None),
    )
    .map_err(|e| format!("无法启动 ssh（请确认本机已安装 OpenSSH 客户端）：{e}"))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "无法取得 ssh stdout".to_string())?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| "无法取得 ssh stderr".to_string())?;
    let err_reader = std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = stderr.read_to_string(&mut buf);
        buf
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => return Err(format!("等待 ssh 结束失败：{e}")),
        }
    };

    let mut out = String::new();
    let _ = stdout.read_to_string(&mut out);
    let err = err_reader.join().unwrap_or_default();
    let err = err.trim();
    match status {
        Some(s) if s.success() => Ok((out, err.to_string())),
        Some(s) => Err(format!(
            "ssh 预检失败（退出码 {}）：{}",
            s.code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "被信号终止".to_string()),
            if err.is_empty() {
                "无 stderr 输出"
            } else {
                err
            }
        )),
        None => Err(format!(
            "ssh 预检超时（{}s 内未返回）：{}",
            timeout.as_secs(),
            if err.is_empty() {
                "无 stderr 输出"
            } else {
                err
            }
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn target() -> SshTarget {
        SshTarget {
            host: "myserver".to_string(),
            node: "/usr/local/bin/node".to_string(),
            helper: "/opt/dsh/helper.js".to_string(),
            helper_hash: HASH.to_string(),
            workspace: "/home/deploy/work".to_string(),
        }
    }

    #[test]
    fn valid_target_has_no_problems() {
        assert!(validate_target(&target()).is_empty());
    }

    // ---------- 正反例成对（AGENTS §5） ----------

    #[test]
    fn host_alias_regex_matches_upstream() {
        for ok in ["a", "A1", "my-host", "a.b_c", "user@host", "h-1"] {
            assert!(is_valid_host_alias(ok), "{ok} 应合法");
        }
        for bad in [
            "",
            "-lead",
            ".lead",
            "_lead",
            "@lead",
            "has space",
            "tab\tx",
            "斜杠/",
        ] {
            assert!(!is_valid_host_alias(bad), "{bad:?} 应非法");
        }
    }

    #[test]
    fn helper_hash_requires_lowercase_64_hex() {
        assert!(is_sha256_hex(HASH));
        assert!(
            !is_sha256_hex(&HASH.to_uppercase()),
            "大写须被拒（上游只认小写）"
        );
        assert!(!is_sha256_hex(&HASH[..63]), "63 位须被拒");
        assert!(!is_sha256_hex(&format!("{HASH}0")), "65 位须被拒");
        assert!(!is_sha256_hex(
            "g123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        ));
        assert!(!is_sha256_hex(""));
    }

    #[test]
    fn non_absolute_paths_are_rejected_for_all_three_path_keys() {
        // 相对路径在远端会以 ssh 会话的 cwd 解析，结果不可预期——上游 zod 就是 startsWith('/')。
        for (label, mutate) in [("node", 0usize), ("helper", 1usize), ("workspace", 2usize)] {
            let mut t = target();
            match mutate {
                0 => t.node = "node".to_string(),
                1 => t.helper = "./helper.js".to_string(),
                _ => t.workspace = "work".to_string(),
            }
            let problems = validate_target(&t);
            assert!(
                problems.iter().any(|p| p.contains(label)),
                "{label} 的相对路径应被拒：{problems:?}"
            );
        }
        // 反例：`~/x` 也不行——`~` 不是绝对路径，远端不会展开成 home。
        let mut t = target();
        t.workspace = "~/work".to_string();
        assert!(!validate_target(&t).is_empty());
    }

    #[test]
    fn validation_reports_all_problems_at_once() {
        // 向导要一次标完表单；逐个报错会让用户来回五轮。
        let mut t = target();
        t.host = "bad host".to_string();
        t.node = "n".to_string();
        t.helper_hash = "XYZ".to_string();
        let problems = validate_target(&t);
        assert_eq!(problems.len(), 3, "三个键各有问题应同时报出：{problems:?}");
    }

    #[test]
    fn empty_values_are_reported_as_empty_not_as_format_errors() {
        let t = SshTarget {
            host: String::new(),
            node: String::new(),
            helper: String::new(),
            helper_hash: String::new(),
            workspace: String::new(),
        };
        let problems = validate_target(&t);
        assert_eq!(problems.len(), 5, "{problems:?}");
        assert!(
            problems.iter().all(|p| p.contains("不能为空")),
            "{problems:?}"
        );
    }

    // ---------- ssh 参数锁定 ----------

    #[test]
    fn probe_args_lock_down_non_interactive_operation() {
        let args = probe_ssh_args(&target());
        let joined = args.join(" ");
        for required in [
            "BatchMode=yes",
            "ForwardAgent=no",
            "StrictHostKeyChecking=yes",
            "ConnectTimeout=10",
        ] {
            assert!(joined.contains(required), "缺少 {required}：{joined}");
        }
        // `-F` 必须**没有**：别名从用户默认配置读出，连接也必须读同一份。
        assert!(!joined.contains(" -F "), "不得传 -F：{joined}");
        assert!(args.contains(&"--".to_string()), "须用 `--` 终止选项解析");
        assert_eq!(args[args.len() - 2], "myserver");
    }

    /// **注入面**：`ssh` 按空格拼接远端 argv 下发，不带 quote 的路径会在远端被重新切分。
    #[test]
    fn probe_args_quote_paths_so_spaces_cannot_resplit_remotely() {
        let t = SshTarget {
            node: "/opt/my node/bin/node".to_string(),
            helper: "/opt/it's/helper.js".to_string(),
            workspace: "/home/a b/work".to_string(),
            ..target()
        };
        let args = probe_ssh_args(&t);
        let remote = args.last().expect("最后一项是远端命令");
        assert!(
            remote.contains("'/opt/my node/bin/node'"),
            "含空格的路径必须被单引号包住：{remote}"
        );
        assert!(
            remote.contains(r"'/opt/it'\''s/helper.js'"),
            "单引号必须被转义（sh_quote 口径）：{remote}"
        );
        assert!(remote.contains("'/home/a b/work'"), "{remote}");
    }

    #[test]
    fn probe_script_uses_the_posix_argument_slots() {
        // 脚本按 `$1`=node、`$2`=helper、`$3`=workspace 取参；
        // 调用侧写的是 `sh -c <script> probe <node> <helper> <workspace>`（$0=probe）。
        let remote = probe_ssh_args(&target()).pop().unwrap();
        assert!(remote.starts_with("sh -c "), "{remote}");
        assert!(remote.contains(" probe "), "$0 占位符应在：{remote}");
    }

    // ---------- 输出解析与评估 ----------

    #[test]
    fn parse_ignores_noise_lines_from_banner_and_motd() {
        let out = "Welcome to Ubuntu 24.04\nLast login: Mon\nos=Linux\nnode_ok=yes\n";
        let parsed = parse_probe_output(out);
        assert_eq!(parsed.get("os").map(String::as_str), Some("Linux"));
        assert_eq!(parsed.get("node_ok").map(String::as_str), Some("yes"));
        assert_eq!(parsed.len(), 2, "噪声行不得进入结果：{parsed:?}");
    }

    fn report(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn all_green_report_passes() {
        let r = report(&[
            ("os", "Darwin"),
            ("node_ok", "yes"),
            ("node_ver", "v22.11.0"),
            ("helper_ok", "yes"),
            ("helper_hash", HASH),
            ("workspace_ok", "yes"),
        ]);
        let probe = assess_probe(&r, HASH);
        assert!(probe.ok, "{:?}", probe.checks);
        assert_eq!(probe.checks.len(), 5);
        assert!(probe.checks.iter().all(|c| c.ok));
    }

    #[test]
    fn hash_mismatch_is_reported_with_both_values() {
        let other = "f".repeat(64);
        let r = report(&[
            ("os", "Linux"),
            ("node_ok", "yes"),
            ("helper_ok", "yes"),
            ("helper_hash", &other),
            ("workspace_ok", "yes"),
        ]);
        let probe = assess_probe(&r, HASH);
        assert!(!probe.ok);
        let check = probe.checks.iter().find(|c| c.key == "helperHash").unwrap();
        assert!(!check.ok);
        assert!(
            check.detail.contains(&other) && check.detail.contains(HASH),
            "{}",
            check.detail
        );
    }

    #[test]
    fn windows_remote_is_rejected_as_unsupported_platform() {
        let r = report(&[
            ("os", "MINGW64_NT-10.0"),
            ("node_ok", "yes"),
            ("helper_ok", "yes"),
            ("helper_hash", HASH),
            ("workspace_ok", "yes"),
        ]);
        let probe = assess_probe(&r, HASH);
        assert!(!probe.ok);
        assert!(!probe.checks[0].ok, "非 Linux/Darwin 必须判不通过");
    }

    #[test]
    fn missing_output_yields_all_checks_failed_not_a_panic() {
        // 远端 shell 不是 POSIX sh（或 banner 里恰好没有 key=value）时：
        // 必须逐项报"没取到"，而不是当成通过。
        let probe = assess_probe(&BTreeMap::new(), HASH);
        assert!(!probe.ok);
        assert_eq!(probe.checks.len(), 5);
        assert!(probe.checks.iter().all(|c| !c.ok));
        assert!(
            probe.checks[0].detail.contains("未取到"),
            "{:?}",
            probe.checks[0]
        );
    }

    #[test]
    fn empty_remote_hash_reports_missing_digest_tool_distinctly() {
        // 「远端没有 sha256sum/shasum」与「摘要不符」是两回事，处置也不同。
        let r = report(&[
            ("os", "Linux"),
            ("node_ok", "yes"),
            ("helper_ok", "yes"),
            ("helper_hash", ""),
            ("workspace_ok", "yes"),
        ]);
        let probe = assess_probe(&r, HASH);
        let check = probe.checks.iter().find(|c| c.key == "helperHash").unwrap();
        assert!(check.detail.contains("sha256sum"), "{}", check.detail);
    }

    #[test]
    fn validation_failure_short_circuits_before_touching_network() {
        // 五键不合法时**不得**起 ssh（否则用户填错一个字符就要等一次网络往返）。
        let mut t = target();
        t.helper_hash = "not-a-hash".to_string();
        let err = probe_ssh_target(&t, Duration::from_millis(50)).unwrap_err();
        assert!(err.contains("配置校验未通过"), "{err}");
    }

    #[test]
    fn host_support_follows_the_compile_target() {
        // ADR-0023 §1.4：两端须 Linux/macOS。断言与编译目标一致（非 Windows 为 true）。
        assert_eq!(host_supports_ssh(), !cfg!(windows));
    }
}
