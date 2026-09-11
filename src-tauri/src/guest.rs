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
//! - **读原文**：`read_files` 返回「相对客体 dsh home 的路径 → 文件原文」，于是
//!   `plugins.rs` / `profiles.rs` 既有的解析与派生纯逻辑**原样复用**，不产生第二套
//!   解析实现。路径基址在**客体侧**展开（[`HOME_EXPR`]：`${DSH_HOME:-$HOME/.dsh}`，
//!   与客体 dsh 自身的 home 解析同源）——宿主不知道客体用户名，也不该拿宿主 home
//!   顶替（那就是 ADR-0016 §2.6 要禁的"管错世界"）。
//! - **写单键**：`write_home_files` 以 base64 载荷 + 同目录临时文件 + `mv` 原子替换
//!   落位；父目录不存在即失败（壳不得代 dsh 生成 profile 目录）。唯一调用方是
//!   ADR-0013 的单键受控写入口（`build_policy`），不生成/复刻三件套内容。
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

/// 客体 dsh home 的 shell 表达式（管理面读写路径的基址）。
///
/// `guest_prep!` 已 source 三个 rc，故这里读到的 `DSH_HOME`（若有）与客体 dsh
/// 自己读到的是同一份环境——ADR-0016 §1.5：客体 dsh 用自己的 WSL home。宿主**不能**
/// 硬编码 `/home/<用户>/.dsh`（用户名未知），更不该用宿主 home 顶替。
///
/// 注意：与 boot 路径同口径（`executor` 不导出 `DSH_HOME`），dev 构建在客体里同样
/// 落到 `~/.dsh`——管理面必须与运行中的客体会话同源。
#[cfg(any(windows, test))]
pub(crate) const HOME_EXPR: &str = "${DSH_HOME:-$HOME/.dsh}";

/// 标准 base64 编码（客体写载荷与 pnpm 投递兜底通道共用；不引第三方依赖，
/// AGENTS §4.2）。2026-09-11 自 `executor.rs` 迁入：与 [`base64_decode`] 同处一源。
#[cfg(any(windows, test))]
pub(crate) fn base64_encode(data: &[u8]) -> String {
    const TBL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(TBL[(n >> 18) as usize & 63] as char);
        out.push(TBL[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            TBL[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TBL[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

// ---------- 纯函数：脚本拼装与帧解析（跨平台可测） ----------

/// `read_files` 的帧头。单行一条记录，内容一律 base64——
/// 单行形态天然免疫文件内容里的换行与任意字节。
#[cfg(any(windows, test))]
pub(crate) const FILE_FRAME: &str = "@@DSH_DOCK_FILE@@";

/// `list_dir` 的条目帧：`@@DSH_DOCK_ENTRY@@<b64 名字>:<d|f>`（d=目录，f=文件）。
#[cfg(any(windows, test))]
pub(crate) const ENTRY_FRAME: &str = "@@DSH_DOCK_ENTRY@@";

/// 目录**不存在**（与「存在但为空」必须区分：前者 = 该世界尚未初始化，
/// 后者 = 真的没有子项）。
#[cfg(any(windows, test))]
pub(crate) const LIST_MISSING: &str = "@@DSH_DOCK_LIST_MISSING@@";

/// 目录**存在**的确认哨兵：即使目录为空或无匹配项，也确保脚本至少产生一行输出，
/// 避免被 `run_wsl_capture` 的空串折叠误判为客体不可达。
#[cfg(any(windows, test))]
pub(crate) const LIST_PRESENT: &str = "@@DSH_DOCK_LIST_PRESENT@@";

/// 组装「列客体 dsh home 下某目录」脚本：一条 entry 帧一行。
/// 显式覆盖点文件（`.[!.]*` / `..?*`）——shell 通配默认不匹配点文件，而宿主侧
/// `read_dir` 会列出它们（两侧口径必须一致，否则同名 profile 在两个世界可不见）。
#[cfg(any(windows, test))]
pub(crate) fn list_dir_script(rel_dir: &str) -> String {
    let path = format!("\"{HOME_EXPR}\"/{}", sh_quote(rel_dir));
    format!(
        "{}if [ -d {path} ]; then \
         printf '{LIST_PRESENT}\\n'; \
         for e in {path}/* {path}/.[!.]* {path}/..?*; do \
         [ -e \"$e\" ] || continue; \
         printf '{ENTRY_FRAME}%s:%s\\n' \
         \"$(printf '%s' \"$(basename \"$e\")\" | base64 | tr -d '\\n')\" \
         \"$([ -d \"$e\" ] && printf d || printf f)\"; \
         done; else printf '{LIST_MISSING}\\n'; fi",
        guest_prep!()
    )
}

/// 解析 `list_dir_script` 输出：`None` = 目录不存在；`Some` = `(名字, 是否目录)`。
/// 非帧行（rc 噪音等）忽略；解码失败的行跳过（一个怪名字不该让整张列表塌掉）。
#[cfg(any(windows, test))]
pub(crate) fn parse_list_dir(raw: &str) -> Option<Vec<(String, bool)>> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let line = line.trim_end_matches('\r');
        if line.contains(LIST_MISSING) {
            return None;
        }
        let Some(rest) = line.strip_prefix(ENTRY_FRAME) else {
            continue;
        };
        let Some((name_b64, kind)) = rest.split_once(':') else {
            continue;
        };
        let Some(name) = base64_decode(name_b64).and_then(|b| String::from_utf8(b).ok()) else {
            continue;
        };
        out.push((name, kind == "d"));
    }
    Some(out)
}

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

/// 组装「读客体 dsh home 下文件原文」脚本：每条**相对路径**一行 base64 帧，
/// 缺失以 `-` 标注。
///
/// 路径基址在客体侧展开（[`HOME_EXPR`]），帧标签 = 入参相对路径原样——宿主侧
/// 只做解码与按标签取用，不需要（也无法）知道客体绝对路径。
/// base64 编码在**客体侧**完成（`base64 | tr -d '\n'`：GNU 与 busybox 皆有，
/// 不用 GNU 专有的 `-w0`），宿主侧只做解码——避免 wsl.exe 的 UTF-16/缓冲
/// 转换把内容弄坏（该坑见 `shell.rs` 的 UTF-16 解码注释）。
#[cfg(any(windows, test))]
pub(crate) fn read_files_script(rel_paths: &[String]) -> String {
    let mut out = String::from(guest_prep!());
    for rel in rel_paths {
        // 标签 = 原样相对路径（帧内 base64，免疫换行/元字符）
        let label = sh_quote(rel);
        // 客体侧绝对路径表达式：前缀不引号整体（要展开），相对路径经 sh_quote
        let path = format!("\"{HOME_EXPR}\"/{label}");
        // 单引号字面量直接进 printf 的 %s，无需再转义
        out.push_str(&format!(
            "if [ -f {path} ]; then printf '{FILE_FRAME}%s:%s\\n' \
             \"$(printf '%s' {label} | base64 | tr -d '\\n')\" \
             \"$(base64 < {path} | tr -d '\\n')\"; \
             else printf '{FILE_FRAME}%s:-\\n' \
             \"$(printf '%s' {label} | base64 | tr -d '\\n')\"; fi;"
        ));
    }
    out
}

/// 写成功哨兵：写脚本**恒 `exit 0`**，成败看哨兵行——同
/// `GUEST_STAGE_PNPM` 的规避口径（`run_with_timeout_raw` 把非零退出折叠成
/// "无输出"，会丢掉诊断）。
#[cfg(any(windows, test))]
pub(crate) const WRITE_OK: &str = "DSH_DOCK_WRITE_OK";
/// 写失败哨兵前缀（后接相对路径，指出是哪一个文件失败）。
#[cfg(any(windows, test))]
pub(crate) const WRITE_FAILED: &str = "DSH_DOCK_WRITE_FAILED";

/// 单次写载荷上限（base64 后约 4/3，进 wsl.exe 命令行；Windows 上限 32K 字符）。
/// 超限拒绝而非硬塞：pnpm-workspace.yaml 这类单键补写的文件本就该是小的——
/// 宁可不自动化，也不冒"命令行被截断后写坏用户文件"的风险（同 ADR-0013 纪律）。
#[cfg(windows)]
pub(crate) const WRITE_MAX_BYTES: usize = 8 * 1024;

/// 组装「原子写客体 dsh home 下文件」脚本。
///
/// 形态：base64 载荷内嵌（单引号字面量，无展开面）→ `base64 -d` 落**同目录**
/// 临时文件 → `mv -f` 原子替换（跨设备 mv 会退化为拷贝，但临时文件与目标同目录，
/// 不跨设备）。**不 `mkdir -p`**：父目录不存在即失败——壳不得代 dsh 生成 profile
/// 目录（AGENTS §6：三件套内容归 dsh 初始化）。
#[cfg(any(windows, test))]
pub(crate) fn write_home_files_script(files: &[(String, String)]) -> String {
    let mut out = String::from(guest_prep!());
    out.push_str("DSH_DOCK_FAIL=;");
    for (rel, content) in files {
        let label = sh_quote(rel);
        let path = format!("\"{HOME_EXPR}\"/{label}");
        let payload = sh_quote(&base64_encode(content.as_bytes()));
        out.push_str(&format!(
            "if [ -d \"$(dirname {path})\" ] && \
             printf '%s' {payload} | base64 -d > {path}.dsh-dock.tmp && \
             mv -f {path}.dsh-dock.tmp {path}; then :; \
             else rm -f {path}.dsh-dock.tmp; \
             echo '{WRITE_FAILED}:{label}'; DSH_DOCK_FAIL=1; fi;"
        ));
    }
    out.push_str(&format!(
        "if [ -z \"$DSH_DOCK_FAIL\" ]; then echo '{WRITE_OK}'; fi; exit 0"
    ));
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

/// 一次读多份客体 dsh home 下的文件（一次 `wsl.exe` 往返）。返回顺序与入参一致；
/// 文件缺失 = `None`（不是错误——调用方按存在性分支，如清单缺失即报"未初始化"）。
///
/// 入参为**相对客体 dsh home** 的路径（`profiles/web/package.json`），基址由
/// [`HOME_EXPR`] 在客体侧展开。
#[cfg(windows)]
pub(crate) fn read_files(
    distro: &str,
    rel_paths: &[String],
) -> Result<Vec<(String, Option<String>)>, String> {
    if rel_paths.is_empty() {
        return Ok(Vec::new());
    }
    let script = read_files_script(rel_paths);
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
pub(crate) fn read_files(
    _distro: &str,
    _rel_paths: &[String],
) -> Result<Vec<(String, Option<String>)>, String> {
    Err("WSL 客体管理面仅在 Windows 宿主可用".to_string())
}

/// 列客体 dsh home 下某目录（一次 `wsl.exe` 往返）。
///
/// `Ok(None)` = 目录不存在（该世界尚未初始化，调用方按"只有内置模板"处理）；
/// `Ok(Some(entries))` = 条目清单（含点文件；顺序由 shell 通配给出，调用方自行排序）；
/// `Err` = 客体不可达/无输出（真错误，不静默当空目录——那会让 UI 显示"没有 profile"）。
#[cfg(windows)]
pub(crate) fn list_dir(distro: &str, rel_dir: &str) -> Result<Option<Vec<(String, bool)>>, String> {
    let script = list_dir_script(rel_dir);
    let out = crate::executor::run_wsl_capture(
        Some(distro),
        &["-e", "bash", "-lic", &script],
        std::time::Duration::from_secs(30),
    )
    .ok_or_else(|| {
        format!("列 {distro} 内目录 {rel_dir} 失败：wsl.exe 调用失败或无输出（客体不可达？）")
    })?;
    Ok(parse_list_dir(&out))
}

/// 非 Windows 孪生（同 [`read_files`] 口径）。
#[cfg(not(windows))]
pub(crate) fn list_dir(
    _distro: &str,
    _rel_dir: &str,
) -> Result<Option<Vec<(String, bool)>>, String> {
    Err("WSL 客体管理面仅在 Windows 宿主可用".to_string())
}

/// 原子写客体 dsh home 下的文件（一次 `wsl.exe` 往返，载荷内嵌脚本）。
/// 入参 = `(相对路径, 全文)`；任一文件失败即整体报错（列出失败路径）。
#[cfg(windows)]
pub(crate) fn write_home_files(distro: &str, files: &[(String, String)]) -> Result<(), String> {
    if files.is_empty() {
        return Ok(());
    }
    // 逐文件上限：只报路径，内容不参与诊断——`_` 绑定是必须的（Windows 目标的
    // `-D warnings` 会把未使用变量判红，2026-09-11 CI 实测）
    if let Some((rel, _)) = files.iter().find(|(_, c)| c.len() > WRITE_MAX_BYTES) {
        return Err(format!(
            "客体文件 {rel} 超出单次写入上限（{} KiB）——请在该发行版终端里手工编辑",
            WRITE_MAX_BYTES / 1024
        ));
    }
    let script = write_home_files_script(files);
    let out = crate::executor::run_wsl_capture(
        Some(distro),
        &["-e", "bash", "-lic", &script],
        std::time::Duration::from_secs(30),
    )
    .ok_or_else(|| format!("写 {distro} 内文件失败：wsl.exe 调用失败或无输出（客体不可达？）"))?;
    if out.contains(WRITE_OK) {
        return Ok(());
    }
    let failed: Vec<&str> = out
        .lines()
        .filter_map(|l| l.trim().strip_prefix(&format!("{WRITE_FAILED}:")))
        .collect();
    if failed.is_empty() {
        Err(format!("写 {distro} 内文件失败：{}", out.trim()))
    } else {
        Err(format!(
            "写 {distro} 内文件失败（{}）——目标 profile 可能尚未初始化，或客体磁盘不可写",
            failed.join("、")
        ))
    }
}

/// 非 Windows 孪生（同 [`read_files`] 口径）。
#[cfg(not(windows))]
pub(crate) fn write_home_files(_distro: &str, _files: &[(String, String)]) -> Result<(), String> {
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
    /// 真实脚本，验证「客体 home 相对路径 → 帧」端到端成立——只测解析器会漏掉
    /// 脚本拼装与路径展开错误。`HOME` 指向临时目录、显式清除 `DSH_HOME`
    /// （测试一律无视环境里的 `DSH_HOME`，AGENTS §6）。
    #[cfg(unix)]
    #[test]
    fn read_files_script_runs_under_bash_and_roundtrips() {
        let home = std::env::temp_dir().join(format!("dsh-dock-guest-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        // `HOME` = 客体用户 home；路径基址是**客体 dsh home**（`$HOME/.dsh`）
        let profile_dir = home.join(".dsh/profiles/web");
        std::fs::create_dir_all(&profile_dir).unwrap();
        std::fs::write(profile_dir.join("package.json"), "{\n  \"a\": 1\n}\n").unwrap();

        let rels = vec![
            "profiles/web/package.json".to_string(),
            "profiles/web/nope.json".to_string(),
        ];
        let script = read_files_script(&rels);
        let out = std::process::Command::new("bash")
            .arg("-c")
            .arg(&script)
            .env("HOME", &home)
            .env_remove("DSH_HOME")
            .output()
            .expect("bash 应可用");
        assert!(out.status.success(), "脚本应成功：{script}");
        let parsed = parse_read_files(&String::from_utf8_lossy(&out.stdout));
        assert_eq!(parsed.len(), 2, "两条路径都应回帧：{parsed:?}");
        // 帧标签 = 入参相对路径原样（宿主不需要知道客体绝对路径）
        assert_eq!(parsed[0].0, "profiles/web/package.json");
        assert_eq!(parsed[0].1.as_deref(), Some("{\n  \"a\": 1\n}\n"));
        assert_eq!(parsed[1].0, "profiles/web/nope.json");
        assert_eq!(parsed[1].1, None);

        let _ = std::fs::remove_dir_all(&home);
    }

    /// 列目录脚本**实跑**验证：目录条目（含点文件与文件/目录区分）、
    /// 目录不存在 → `None`（脚本总会输出，空目录不会被误判为「无输出」）。
    #[cfg(unix)]
    #[test]
    fn list_dir_script_runs_under_bash_and_reports_entries() {
        let home = std::env::temp_dir().join(format!("dsh-dock-guest-l-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let profiles = home.join(".dsh/profiles");
        std::fs::create_dir_all(profiles.join("web")).unwrap();
        std::fs::create_dir_all(profiles.join("node_modules")).unwrap();
        std::fs::create_dir_all(profiles.join(".hidden")).unwrap();
        std::fs::write(profiles.join("stray.txt"), "x").unwrap();

        let run = |rel: &str| {
            let script = list_dir_script(rel);
            let out = std::process::Command::new("bash")
                .arg("-c")
                .arg(&script)
                .env("HOME", &home)
                .env_remove("DSH_HOME")
                .output()
                .expect("bash 应可用");
            assert!(out.status.success(), "脚本应成功：{script}");
            parse_list_dir(&String::from_utf8_lossy(&out.stdout))
        };

        let mut entries = run("profiles").expect("目录存在");
        entries.sort();
        assert_eq!(
            entries,
            vec![
                (".hidden".to_string(), true),
                ("node_modules".to_string(), true),
                ("stray.txt".to_string(), false),
                ("web".to_string(), true),
            ],
            "点文件必须列出（与宿主 read_dir 同口径），文件/目录要可区分"
        );

        // 空目录 → Some(空表)（不是 None：世界已初始化但确实没有 profile）
        std::fs::create_dir_all(home.join(".dsh/empty")).unwrap();
        assert_eq!(run("empty"), Some(Vec::new()));

        // 目录不存在 → None
        assert_eq!(run("profiles/nope"), None);

        let _ = std::fs::remove_dir_all(&home);
    }

    /// 纯解析容错：杂讯行忽略、坏帧跳过、缺失哨兵优先。
    #[test]
    fn parse_list_dir_ignores_noise_and_marks_missing() {
        let raw = "motd 噪音\n@@DSH_DOCK_ENTRY@@d2Vi:d\n@@DSH_DOCK_ENTRY@@bWlzc2luZ19raW5k\n";
        assert_eq!(
            parse_list_dir(raw),
            Some(vec![("web".to_string(), true)]),
            "缺 kind 段的坏帧应跳过"
        );
        assert_eq!(parse_list_dir("@@DSH_DOCK_LIST_MISSING@@\n"), None);
        assert_eq!(
            parse_list_dir("@@DSH_DOCK_LIST_PRESENT@@\n"),
            Some(Vec::new())
        );
        assert_eq!(parse_list_dir(""), Some(Vec::new()));
    }

    /// 写脚本**实跑**验证：base64 载荷 → 同目录临时文件 → `mv` 原子替换；
    /// 父目录缺失时给失败哨兵且**不创建目录**（壳不得代 dsh 生成 profile）。
    /// 脚本恒 `exit 0`（成败看哨兵行）。
    #[cfg(unix)]
    #[test]
    fn write_home_files_script_writes_atomically_and_reports_failure() {
        let home = std::env::temp_dir().join(format!("dsh-dock-guest-w-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        let dsh_home = home.join(".dsh");
        std::fs::create_dir_all(dsh_home.join("profiles/web")).unwrap();
        let rel = "profiles/web/pnpm-workspace.yaml";
        let content = "packages:\n  - .\n\ndangerouslyAllowAllBuilds: true\n";

        let script = write_home_files_script(&[(rel.to_string(), content.to_string())]);
        let out = std::process::Command::new("bash")
            .arg("-c")
            .arg(&script)
            .env("HOME", &home)
            .env_remove("DSH_HOME")
            .output()
            .expect("bash 应可用");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success());
        assert!(stdout.contains(WRITE_OK), "{stdout}");
        assert_eq!(
            std::fs::read_to_string(dsh_home.join(rel)).unwrap(),
            content,
            "内容应逐字节落位"
        );
        assert!(
            !dsh_home.join(format!("{rel}.dsh-dock.tmp")).exists(),
            "临时文件必须已被 mv 消耗"
        );

        // 父目录不存在 → 失败哨兵 + 不创建目录
        let ghost = "profiles/ghost/pnpm-workspace.yaml";
        let script = write_home_files_script(&[(ghost.to_string(), content.to_string())]);
        let out = std::process::Command::new("bash")
            .arg("-c")
            .arg(&script)
            .env("HOME", &home)
            .env_remove("DSH_HOME")
            .output()
            .expect("bash 应可用");
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(out.status.success(), "脚本恒 exit 0");
        assert!(!stdout.contains(WRITE_OK), "{stdout}");
        assert!(
            stdout.contains(&format!("{WRITE_FAILED}:{ghost}")),
            "{stdout}"
        );
        assert!(
            !dsh_home.join("profiles/ghost").exists(),
            "不得代 dsh 建目录"
        );

        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn base64_codecs_match_rfc4648_vectors_and_roundtrip() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        // 往返（含非 ASCII 与换行）
        let raw = "键: 值\n".as_bytes();
        assert_eq!(base64_decode(&base64_encode(raw)).unwrap(), raw);
    }
}
