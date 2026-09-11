//! guest.rs —— WSL 客体**管理面**原语（ADR-0016 P1）。
//!
//! ## 为什么存在
//!
//! WSL 模式下 dsh 与它的引擎都在**客体**里（`~/.dsh-dock/engines`，见 ADR-0010 客体同构），
//! 而管理面（插件中心 / profile 生命周期 / 只读控制台）原实现全部按**宿主** home 与宿主
//! 引擎实现——于是控制中心在 WSL 模式下管的是宿主世界、且宿主引擎按设计永不就绪
//! （ADR-0004：Windows 侧壳不触网），实测表现即「引擎未就绪（node 已就绪、dsh 缺失）」。
//! 本模块提供"把一次操作打进客体"的最小原语，供管理面按活动模式择源。
//!
//! ## 设计约束（逐条对应 ADR-0016 §2）
//!
//! - **读原文**：`read_files` 返回「客体路径 → 文件原文」，于是 `plugins.rs` /
//!   `profiles.rs` 既有的解析与派生纯逻辑**原样复用**，不产生第二套解析实现。
//! - **单源脚本片段**：路径准备（`guest_prep!`）与 shell 引用（`sh_quote`）从
//!   `executor.rs` **迁入本模块**成为唯一源，`executor` 反向引用——避免两处漂移。
//! - **spawn 一律经 `lifecycle`**（ADR-0015 / AGENTS §6）：读走
//!   `executor::run_wsl_capture`（`Role::Probe`），CLI 转发走 `Role::DshCli`
//!   （`profiles::run_dsh_cli_in_guest`）。
//! - **纯函数可测**：脚本拼装与帧解析都是纯字符串函数，跨平台可测；脚本本身在
//!   macOS/Linux 上可直接以 bash 实跑验证（同 `guest_prep` 既有做法）。
//! - 本模块**不触网**：网络发生在客体 dsh/pnpm 进程内（ADR-0004 §7 口径）。

/// 客体内「准备好 PATH + 引擎目录」的公共前缀（固定脚本，不插值用户输入）。
///
/// ADR-0010 客体同构（2026-09-04 收缩）：工具链 = 壳引擎目录
/// `~/.dsh-dock/engines`（pnpm 投递落点 + node/dsh 引擎引导均在此），
/// 版本管理器兜底扫描随探测层退役——系统 node 不再是任何环节的来源。
/// 仍用**交互式登录壳 `bash -lic`**（rc 非交互守卫放行，2026-08-26 实机
/// bug 教训，见 `rc_guard_blocks_non_interactive_login_shell`）；source
/// 三个标准 rc 后**末尾**前置引擎 bin（盖掉 rc 里的任何 PATH 设置），
/// 并导出 PNPM_HOME（pnpm 全局 bin 目录不在 PATH = ERR_PNPM_GLOBAL_BIN_DIR_NOT_IN_PATH）。
///
/// 模板为纯字符串 → 跨平台可测（macOS/Linux 测试直接以 bash 实跑验证）。
#[cfg(any(windows, test))]
macro_rules! guest_prep {
    () => {
        concat!(
            "DSH_ENGINES=\"$HOME/.dsh-dock/engines\";",
            ". /etc/profile 2>/dev/null;",
            ". \"$HOME/.profile\" 2>/dev/null; . \"$HOME/.bashrc\" 2>/dev/null;",
            "if [ -d \"$DSH_ENGINES/bin\" ]; then",
            " PNPM_HOME=\"$DSH_ENGINES\"; export PNPM_HOME;",
            " PATH=\"$DSH_ENGINES/bin:$PATH\"; export PATH;",
            "fi;",
        )
    };
}
#[cfg(any(windows, test))]
pub(crate) use guest_prep;

/// POSIX shell 单引号字面量：`'` → `'\''`。profile 名虽经 `validate_profile_name`
/// 校验，拒绝集之外仍可含空格/引号/`;`/`$`/反引号等元字符——插入 guest 脚本
/// 必须过这里，防脚本断裂与注入面（同机自伤亦是伤）。
#[cfg(any(windows, test))]
pub(crate) fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

// ---------- 纯函数：脚本拼装与帧解析（跨平台可测） ----------

/// `read_files` 的帧头。单行一条记录，内容一律 base64——
/// 单行形态天然免疫文件内容里的换行与任意字节。
#[cfg(any(windows, test))]
pub(crate) const FILE_FRAME: &str = "@@DSH_DOCK_FILE@@";

/// 组装「客体 dsh CLI 转发」脚本：路径准备 → `exec dsh <args>`。
///
/// `exec` 让 dsh 取代 bash，退出码与信号语义直接透传给 `wsl.exe`，宿主侧读到的
/// `status.code()` 即 dsh 真实退出码（`profiles::run_dsh_cli_in_guest` 依赖此点）。
/// **不设 `DSH_HOME`**：与 boot 的客体启动同口径，让客体自己的环境（rc 里的
/// `DSH_HOME` 或 dsh 默认 `~/.dsh`）决定世界——管理面必须与运行中的会话同源
/// （ADR-0016 §2.6）。
#[cfg(any(windows, test))]
pub(crate) fn dsh_cli_script(args: &[String]) -> String {
    let quoted = args
        .iter()
        .map(|a| sh_quote(a))
        .collect::<Vec<_>>()
        .join(" ");
    format!("{}exec dsh {quoted}", guest_prep!())
}

/// 组装「读客体文件原文」脚本：每条路径一行 base64 帧，缺失以 `-` 标注。
///
/// base64 编码在**客体侧**完成（`base64 | tr -d '\n'`：GNU 与 busybox 皆有，
/// 不用 GNU 专有的 `-w0`），宿主侧只做解码——避免 wsl.exe 的 UTF-16/缓冲
/// 转换把内容弄坏（该坑见 `shell.rs` 的 UTF-16 解码注释）。
#[cfg(any(windows, test))]
pub(crate) fn read_files_script(paths: &[String]) -> String {
    let mut out = String::from(guest_prep!());
    for p in paths {
        let q = sh_quote(p);
        // 单引号字面量直接进 printf 的 %s，无需再转义
        out.push_str(&format!(
            "if [ -f {q} ]; then printf '{FILE_FRAME}%s:%s\\n' \
             \"$(printf '%s' {q} | base64 | tr -d '\\n')\" \
             \"$(base64 < {q} | tr -d '\\n')\"; \
             else printf '{FILE_FRAME}%s:-\\n' \
             \"$(printf '%s' {q} | base64 | tr -d '\\n')\"; fi;"
        ));
    }
    out
}

/// 解析 `read_files_script` 的输出帧 → `[(客体路径, Some(原文) | None)]`。
///
/// 容错：非帧行（rc 噪音、motd、bash 警告）一律忽略；帧内 base64 解不开的行
/// 跳过而非整体失败——一个文件读坏不该让整张插件清单塌掉。
#[cfg(any(windows, test))]
pub(crate) fn parse_read_files(raw: &str) -> Vec<(String, Option<String>)> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let Some(rest) = line.trim_end_matches('\r').strip_prefix(FILE_FRAME) else {
            continue;
        };
        let Some((path_b64, content_b64)) = rest.split_once(':') else {
            continue;
        };
        let Some(path) = base64_decode(path_b64).and_then(|b| String::from_utf8(b).ok()) else {
            continue;
        };
        let content = if content_b64 == "-" {
            None
        } else {
            base64_decode(content_b64).map(|b| String::from_utf8_lossy(&b).to_string())
        };
        out.push((path, content));
    }
    out
}

/// 标准 base64 解码（不引第三方依赖，AGENTS §4.2）。非法字符/长度返回 None。
#[cfg(any(windows, test))]
pub(crate) fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    let mut out = Vec::with_capacity(s.len() / 4 * 3);
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            b'=' | b'\n' | b'\r' => continue,
            _ => return None,
        } as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

// ---------- Windows 实体：真正打进客体 ----------

/// 一次读多份客体文件（一次 `wsl.exe` 往返）。返回顺序与入参一致；
/// 文件缺失 = `None`（不是错误——调用方按存在性分支，如清单缺失即报"未初始化"）。
///
/// `expect(dead_code)`：管理面择源尚未接线（ADR-0016 §5 行动项剩余清单 a–e）。
/// 接线后本 expect 会「不再触发」→ CI 立即报 unfulfilled，**强制删除**——
/// 自清理闸门，防惰性死代码长期挂着。
#[cfg(windows)]
#[expect(dead_code)]
pub(crate) fn read_files(
    distro: &str,
    paths: &[String],
) -> Result<Vec<(String, Option<String>)>, String> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let script = read_files_script(paths);
    let out = crate::executor::run_wsl_capture(
        Some(distro),
        &["-e", "bash", "-lic", &script],
        std::time::Duration::from_secs(30),
    )
    .ok_or_else(|| format!("读取 {distro} 内文件失败：wsl.exe 调用失败或无输出（客体不可达？）"))?;
    Ok(parse_read_files(&out))
}

/// 非 Windows 孪生：客体只存在于 Windows。保留同一签名是为了让**接线后的调用点**
/// 在所有平台都参与编译与 lint（否则 `#[cfg(windows)]` 之外的分支永不被检查）。
#[cfg(not(windows))]
#[expect(dead_code)]
pub(crate) fn read_files(
    _distro: &str,
    _paths: &[String],
) -> Result<Vec<(String, Option<String>)>, String> {
    Err("WSL 客体管理面仅在 Windows 宿主可用".to_string())
}

/// 在客体里跑一段脚本，stdout/stderr 汇入 `log`，按 `timeout` 收口。
///
/// 契约对齐 `profiles::run_dsh_forward`（本地孪生）：输出落**同一个**运维日志文件
/// 由调用方追加/轮转，故宿主侧的 `current_run_output` 切片逻辑对客体运行同样成立。
/// 退出码 = 客体命令真实退出码（脚本里用 `exec` 透传）。超时即杀（`wsl.exe` 被杀，
/// 客体侧进程随之收口——`Role::DshCli` 的生命线守卫同时兜底）。
#[cfg(windows)]
pub(crate) fn run_script_to_log(
    distro: &str,
    script: &str,
    log: &std::fs::File,
    timeout: std::time::Duration,
) -> Result<(Option<i32>, bool), String> {
    let mut cmd = crate::executor::wsl_command(Some(distro));
    cmd.args(["-e", "bash", "-lic", script])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::from(
            log.try_clone().map_err(|e| e.to_string())?,
        ))
        .stderr(std::process::Stdio::from(
            log.try_clone().map_err(|e| e.to_string())?,
        ));
    let mut child = crate::lifecycle::spawn(
        &mut cmd,
        crate::lifecycle::Role::DshCli,
        crate::lifecycle::GuardCtx::of("wsl-dsh-cli", None),
    )
    .map_err(|e| format!("spawn wsl.exe 失败（{distro}）：{e}"))?;
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok((status.code(), false)),
            Ok(None) => {}
            Err(e) => return Err(format!("等待客体命令退出失败：{e}")),
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Ok((None, true));
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sh_quote_escapes_single_quotes_and_metacharacters() {
        assert_eq!(sh_quote("web"), "'web'");
        assert_eq!(sh_quote("a'b"), r#"'a'\''b'"#);
        // 注入面：分号/反引号/$ 在单引号字面量里全部惰性化
        assert_eq!(sh_quote("x; rm -rf ~"), "'x; rm -rf ~'");
        assert_eq!(sh_quote("$(whoami)"), "'$(whoami)'");
    }

    #[test]
    fn dsh_cli_script_quotes_every_arg_and_execs() {
        let args = vec![
            "plugin".to_string(),
            "--profile".to_string(),
            "we b".to_string(),
            "add".to_string(),
            "github:o/r#path:pkg".to_string(),
        ];
        let s = dsh_cli_script(&args);
        assert!(s.contains("exec dsh 'plugin' '--profile' 'we b' 'add' 'github:o/r#path:pkg'"));
        // PATH 准备必须在内（否则客体 dsh 不可达）
        assert!(s.contains("DSH_ENGINES=\"$HOME/.dsh-dock/engines\""));
        // 不得注入宿主 DSH_HOME：世界由客体自己决定（ADR-0016 §2.6）
        assert!(!s.contains("DSH_HOME="));
    }

    #[test]
    fn base64_decode_roundtrips_known_vectors() {
        assert_eq!(base64_decode("aGVsbG8=").unwrap(), b"hello");
        assert_eq!(base64_decode("").unwrap(), b"");
        assert_eq!(base64_decode("5Lit5paH").unwrap(), "中文".as_bytes());
        // 换行（`base64` 默认 76 列折行）必须被容忍
        assert_eq!(base64_decode("aGVs\nbG8=").unwrap(), b"hello");
        assert!(base64_decode("not*base64").is_none());
    }

    #[test]
    fn parse_read_files_keeps_content_and_marks_missing() {
        let raw = "\
rc 噪音一行
@@DSH_DOCK_FILE@@d2Vi:ZGVwZW5kZW5jaWVzCg==
@@DSH_DOCK_FILE@@bWlzc2luZw==:-
";
        let got = parse_read_files(raw);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].0, "web");
        assert_eq!(got[0].1.as_deref(), Some("dependencies\n"));
        assert_eq!(got[1].0, "missing");
        assert_eq!(got[1].1, None);
    }

    /// 脚本**实跑**验证（同 `guest_prep` 既有做法）：在 macOS/Linux 上以 bash 跑
    /// 真实脚本，验证帧格式、缺失标注与 base64 通道端到端成立——只测解析器会漏掉
    /// 脚本拼装错误。
    #[cfg(unix)]
    #[test]
    fn read_files_script_runs_under_bash_and_roundtrips() {
        let dir = std::env::temp_dir().join(format!("dsh-dock-guest-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("pkg.json");
        std::fs::write(&file, "{\n  \"a\": 1\n}\n").unwrap();
        let missing = dir.join("nope.json");

        let paths = vec![file.display().to_string(), missing.display().to_string()];
        let script = read_files_script(&paths);
        let out = std::process::Command::new("bash")
            .arg("-c")
            .arg(&script)
            .output()
            .expect("bash 应可用");
        assert!(out.status.success(), "脚本应成功：{script}");
        let parsed = parse_read_files(&String::from_utf8_lossy(&out.stdout));
        assert_eq!(parsed.len(), 2, "两条路径都应回帧：{parsed:?}");
        assert_eq!(parsed[0].1.as_deref(), Some("{\n  \"a\": 1\n}\n"));
        assert_eq!(parsed[1].1, None);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
