//! ssh_config.rs —— `~/.ssh/config` 解析（2026-09-15，ADR-0023 §2.6 / §2.11）。
//!
//! ## 为什么要壳解析
//!
//! SSH 远程工作区向导要**让用户从已有 alias 里选一个**（`dsh-ssh` 的 `host` 是
//! "已存在的 OpenSSH alias"，不是任意 hostname）。因此壳必须知道用户有哪些 alias。
//! ADR-0023 §3 方案 D 明确否决了"前端解析"（违反 AGENTS §4.4 红线 2 与 §4.3），
//! §2.6 要求**解析在 Rust 后端**。
//!
//! ## 读取面边界（本仓库首个 `$DSH_HOME` 之外的读取面）
//!
//! - **只读 `~/.ssh/config` 一个文件**；**不跟随 `Include`**。跟随会把读取面扩张成
//!   "任意路径"——那正是 ADR-0023 §2.7 要求显式登记的东西。跳过的 `Include` 会出现在
//!   `notes` 里，让用户知道列出的主机可能不完整。
//! - **只取必要字段**，全部是**非机密**的：alias、`HostName`/`User`/`Port`/
//!   `ProxyJump`、`IdentityFile` 的**路径**。**不**回传任何密钥内容（私钥不在这个文件里，
//!   而这里也没有任何字段能承载它）。
//! - 文件读取有 1 MiB 上限：超过说明这不是一份手写 config（或被误指向大文件），
//!   截断并如实记一笔。
//!
//! ## 语义忠实度（错了会给用户**错误的事实**，比报错更糟）
//!
//! - 关键字**大小写不敏感**；`Keyword Value` 与 `Keyword=Value` 两种写法都接受。
//! - `#` 只在**行首**（前导空白后）是注释——OpenSSH 的 `ssh_config(5)` 口径如此，
//!   行中 `#` 是数据。
//! - 值支持双引号与 `\` 转义；行尾 `\` 续行。
//! - `Host` 可一次给多个模式（空白或**逗号**分隔），可含 `*` / `?` 与 `!` 取反。
//! - **「先出现的取值优先」**（`first obtained value`）：故解析结果 = 全局段（出现在
//!   第一个 `Host` 之前的关键字）+ 各 `Host` 段，按**文件顺序**逐段取第一个匹配段的
//!   取值。全局段等价于一个写在最前、匹配一切的模式块。
//! - `Match` 段**不参与**（其取值依赖运行时条件，无法静态呈现）——但会记入 `notes`，
//!   而不是静默忽略。
//!
//! ## 可测性
//!
//! 解析与匹配是纯函数（AGENTS §5）：`parse_ssh_config` / `host_matches` / `glob_match`
//! 全部无 IO、无环境依赖；真实文件读取只在 [`load_ssh_hosts`] 里，**单测不碰它**
//! （否则会读开发机真实 ssh 配置，属测试隔离问题）。

use std::path::{Path, PathBuf};

use serde::Serialize;

/// 单个可选择的 SSH 主机（**只含非机密字段**）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshHost {
    /// `Host` 里的**具体**别名（不含通配符/取反）——即用户可以当 `host` 用的那个名字。
    pub alias: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<u16>,
    /// `ProxyJump`（或 `ProxyCommand` 的存在性由 notes 体现；此处只取 `ProxyJump`）。
    pub proxy_jump: Option<String>,
    /// `IdentityFile` 的**路径**（可多条）。路径非机密；私钥内容从不在此文件里。
    pub identity_files: Vec<String>,
}

/// 一次解析的结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshHosts {
    pub hosts: Vec<SshHost>,
    /// 解析期**如实记录**的降级说明（`Include` 未跟随 / `Match` 段忽略 / 畸形行 /
    /// 非法端口 / 截断）。空 = 无降级。宁可多说一句，也不让用户以为列表是完整的。
    pub notes: Vec<String>,
}

/// 读取上限。手写 `~/.ssh/config` 通常几 KB。
const MAX_CONFIG_BYTES: usize = 1024 * 1024;

/// 一个 `Host` 段（或全局段）。字段都按"首个取值优先"在解析期定下。
#[derive(Debug, Default, Clone)]
struct Block {
    /// 模式列表；全局段用 `["*"]` 表示"匹配一切"。
    patterns: Vec<String>,
    hostname: Option<String>,
    user: Option<String>,
    port_raw: Option<String>,
    proxy_jump: Option<String>,
    identity_files: Vec<String>,
}

/// `~/.ssh/config` 的路径（**纯路径推导，不读文件**——单测据此断言，不碰真实文件）。
pub fn ssh_config_path(home: &Path) -> PathBuf {
    home.join(".ssh").join("config")
}

/// 解析一份 `~/.ssh/config` 文本（**纯函数**）。
pub fn parse_ssh_config(text: &str) -> SshHosts {
    let mut blocks: Vec<Block> = Vec::new();
    // 全局段恒存在且排第一：出现在第一个 `Host` 之前的关键字对所有主机生效，
    // 且因"先出现者优先"而**优先于**后面任何段（含 `Host *`）。
    blocks.push(Block {
        patterns: vec!["*".to_string()],
        ..Block::default()
    });
    // `current` 的三态语义（`Option<usize>` 里塞第三个状态容易写错，故用注释钉死）：
    //   `Some(0)`   = **全局段**（第一个 `Host` 之前的关键字，对所有主机生效）
    //   `Some(i>0)` = 第 i 个 `Host` 段
    //   `None`      = **正在 `Match` 段内** → 其后的关键字一律**丢弃**
    //
    // 初版把 `Match` 也归到 `None` 再 `unwrap_or(0)` 落到全局段，`Match` 段里的
    // `HostName` 于是泄漏给**所有**主机——正是 `match_block_does_not_leak_into_previous_host`
    // 要拦的那种错（给用户错误的事实）。
    let mut current: Option<usize> = Some(0);
    let mut notes: Vec<String> = Vec::new();
    let mut seen_include = false;
    let mut seen_match = false;

    for (line_no, raw) in logical_lines(text) {
        let Some((keyword, value)) = split_keyword(&raw) else {
            continue;
        };
        if value.trim().is_empty() && keyword != "host" && keyword != "match" {
            // `HostName` 这类关键字空值：OpenSSH 也当无值处理，静默跳过（不是错误）。
            continue;
        }
        match keyword.as_str() {
            "host" => {
                let (args, unterminated) = split_args(&value);
                if unterminated {
                    notes.push(format!("第 {line_no} 行引号未闭合，按原样取值"));
                }
                // 模式还可逗号分隔（`Host a,b`）。
                let patterns: Vec<String> = args
                    .iter()
                    .flat_map(|a| a.split(','))
                    .map(str::trim)
                    .filter(|p| !p.is_empty())
                    .map(str::to_string)
                    .collect();
                if patterns.is_empty() {
                    notes.push(format!("第 {line_no} 行的 `Host` 没有参数，已跳过该段"));
                    current = None;
                    continue;
                }
                // 行中 `#` **不是**注释（OpenSSH 只在行首识别注释），于是
                // `Host a # prod` 里的 `#` 与 `prod` 会成为**额外主机名**。
                // 实测确认：`ssh -G '#'` 对 `Host a # prod comment` 返回 `hostname #`
                // ——OpenSSH 确实把它们当模式。这里如实保留，但要告诉用户为什么
                // 下拉里多了几个怪名字（否则他会以为壳解析错了）。
                if patterns.iter().any(|p| p.starts_with('#')) {
                    notes.push(format!(
                        "第 {line_no} 行的 `Host` 里出现 `#`：OpenSSH **只在行首**把 `#` 当注释，\
                         所以 `#` 与它后面的词被当成了额外主机名（若本意是注释，请把注释移到单独一行）"
                    ));
                }
                blocks.push(Block {
                    patterns,
                    ..Block::default()
                });
                current = Some(blocks.len() - 1);
            }
            "match" => {
                if !seen_match {
                    seen_match = true;
                    notes.push(
                        "配置含 `Match` 段：已**忽略**（其取值依赖运行时条件，无法静态呈现），\
                         被它设置的主机参数不会出现在下面的结果里"
                            .to_string(),
                    );
                }
                // `Match` 之后的关键字不属于任何 Host 段——必须把 current 置空，
                // 否则 `Match` 段里的 `HostName` 会被错记到上一个 Host 上。
                current = None;
            }
            "include" => {
                if !seen_include {
                    seen_include = true;
                    notes.push(
                        "配置含 `Include`：**未跟随**（本解析器只读 ~/.ssh/config 一个文件），\
                         由被包含文件定义的主机不会出现在下面的结果里"
                            .to_string(),
                    );
                }
            }
            "hostname" | "user" | "port" | "proxyjump" | "identityfile" => {
                let (args, unterminated) = split_args(&value);
                if unterminated {
                    notes.push(format!("第 {line_no} 行引号未闭合，按原样取值"));
                }
                let Some(first) = args.first().cloned() else {
                    continue;
                };
                // `Match` 段内（`current == None`）的关键字**一律丢弃**：其取值依赖
                // 运行时条件，静态呈现不了；落到全局段则会泄漏给所有主机。
                let Some(idx) = current else {
                    continue;
                };
                let block = &mut blocks[idx];
                match keyword.as_str() {
                    // 首个取值优先：已设过就不再覆盖。
                    "hostname" => block.hostname.get_or_insert(first),
                    "user" => block.user.get_or_insert(first),
                    "port" => block.port_raw.get_or_insert(first),
                    "proxyjump" => block.proxy_jump.get_or_insert(first),
                    // IdentityFile 是**累加**语义（OpenSSH 可配多条），不是首个优先。
                    "identityfile" => {
                        block.identity_files.extend(args);
                        continue;
                    }
                    _ => unreachable!(),
                };
            }
            _ => {}
        }
    }

    // 具体别名：去掉含通配/取反的模式（它们不能当 `host` 用），保持文件顺序、去重。
    let mut aliases: Vec<String> = Vec::new();
    for block in blocks.iter().skip(1) {
        for p in &block.patterns {
            if is_concrete_alias(p) && !aliases.contains(p) {
                aliases.push(p.clone());
            }
        }
    }

    let hosts = aliases
        .into_iter()
        .map(|alias| {
            let hostname = resolve(&blocks, &alias, |b| b.hostname.clone());
            let user = resolve(&blocks, &alias, |b| b.user.clone());
            let proxy_jump = resolve(&blocks, &alias, |b| b.proxy_jump.clone());
            let port_raw = resolve(&blocks, &alias, |b| b.port_raw.clone());
            let port = match port_raw {
                None => None,
                Some(v) => match v.parse::<u16>() {
                    Ok(p) if p > 0 => Some(p),
                    _ => {
                        notes.push(format!(
                            "主机「{alias}」的 `Port` 不是合法端口号：{v}（已忽略）"
                        ));
                        None
                    }
                },
            };
            let identity_files = resolve(&blocks, &alias, |b| {
                if b.identity_files.is_empty() {
                    None
                } else {
                    Some(b.identity_files.clone())
                }
            })
            .unwrap_or_default();
            SshHost {
                alias,
                hostname,
                user,
                port,
                proxy_jump,
                identity_files,
            }
        })
        .collect();

    SshHosts { hosts, notes }
}

/// 按**文件顺序**取第一个匹配该别名的段的取值（"首个取值优先"）。
fn resolve<T>(blocks: &[Block], alias: &str, pick: impl Fn(&Block) -> Option<T>) -> Option<T> {
    blocks
        .iter()
        .filter(|b| host_matches(&b.patterns, alias))
        .find_map(pick)
}

/// 模式列表是否匹配该主机（OpenSSH `match_pattern_list` 语义：任一取反模式命中即整段不匹配）。
pub fn host_matches(patterns: &[String], host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    let mut matched = false;
    for p in patterns {
        let (negated, pat) = match p.strip_prefix('!') {
            Some(rest) => (true, rest),
            None => (false, p.as_str()),
        };
        if glob_match(&pat.to_ascii_lowercase(), &host) {
            if negated {
                return false;
            }
            matched = true;
        }
    }
    matched
}

/// `*`（任意长度，含空）与 `?`（恰好一个字符）——OpenSSH 模式只有这两个通配符。
/// 回溯式匹配，`*` 不吞掉后续字面量。逐字符比较，**不做**大小写折叠（调用方先折叠）。
fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star, mut backtrack) = (usize::MAX, 0usize);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = pi;
            backtrack = ti;
            pi += 1;
        } else if star != usize::MAX {
            pi = star + 1;
            backtrack += 1;
            ti = backtrack;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

/// 能否当 `host` 用：不含 `*` / `?` / `!`，且非空。
fn is_concrete_alias(pattern: &str) -> bool {
    !pattern.is_empty()
        && !pattern.contains('*')
        && !pattern.contains('?')
        && !pattern.starts_with('!')
}

/// 把文本切成**逻辑行**（处理行尾 `\` 续行、去注释与空行），保留起始行号。
fn logical_lines(text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut pending: Option<(usize, String)> = None;
    for (idx, raw) in text.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw.trim_end_matches('\r');
        let (text_part, continues) = match split_continuation(line) {
            Some(head) => (head, true),
            None => (line.to_string(), false),
        };
        match pending.as_mut() {
            Some((_, acc)) => {
                acc.push_str(&text_part);
                if !continues {
                    let (start, joined) = pending.take().expect("刚判过存在");
                    push_logical(&mut out, start, &joined);
                }
            }
            None => {
                if continues {
                    pending = Some((line_no, text_part));
                } else {
                    push_logical(&mut out, line_no, &text_part);
                }
            }
        }
    }
    // 文件以续行符结尾：把已经攒下的部分当作逻辑行，而不是丢掉。
    if let Some((start, joined)) = pending {
        push_logical(&mut out, start, &joined);
    }
    out
}

fn push_logical(out: &mut Vec<(usize, String)>, line_no: usize, line: &str) {
    let trimmed = line.trim_start();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return;
    }
    out.push((line_no, trimmed.to_string()));
}

/// 行尾 `\` = 续行（**奇数个**反斜杠才是续行；`\\` 是转义后的字面反斜杠）。
fn split_continuation(line: &str) -> Option<String> {
    let trailing = line.chars().rev().take_while(|c| *c == '\\').count();
    if trailing % 2 == 1 {
        Some(line[..line.len() - 1].to_string())
    } else {
        None
    }
}

/// 拆出关键字与原始值。`Keyword value` / `Keyword=value` / `Keyword = value` 都认。
/// 关键字统一小写——`HostName` 与 `hostname` 是一回事。
fn split_keyword(line: &str) -> Option<(String, String)> {
    let line = line.trim_start();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let end = line
        .find(|c: char| c.is_whitespace() || c == '=')
        .unwrap_or(line.len());
    let (keyword, rest) = line.split_at(end);
    if keyword.is_empty() {
        return None;
    }
    let rest = rest.trim_start().trim_start_matches('=');
    Some((keyword.to_ascii_lowercase(), rest.trim_start().to_string()))
}

/// 按空白拆参数，支持双引号与 `\` 转义。
///
/// 返回 `(参数, 引号是否未闭合)`——未闭合**不当错误**（OpenSSH 容忍），但要**如实记一笔**，
/// 因为那通常意味着用户写漏了一个引号，而结果会与他预期不同。
fn split_args(s: &str) -> (Vec<String>, bool) {
    let mut args: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut in_quote = false;
    let mut has_token = false;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                has_token = true;
                if let Some(next) = chars.next() {
                    cur.push(next);
                }
            }
            '"' => {
                has_token = true;
                in_quote = !in_quote;
            }
            c if c.is_whitespace() && !in_quote => {
                if has_token {
                    args.push(std::mem::take(&mut cur));
                    has_token = false;
                }
            }
            c => {
                has_token = true;
                cur.push(c);
            }
        }
    }
    if has_token {
        args.push(cur);
    }
    (args, in_quote)
}

/// 读取并解析用户真实的 `~/.ssh/config`（**唯一有 IO 的函数**；单测不调用）。
///
/// 文件不存在**不是错误**（"还没配过 SSH"是完全正常的状态），返回空列表 + 一条说明。
pub fn load_ssh_hosts() -> Result<SshHosts, String> {
    let home = crate::resolve::user_home()
        .ok_or_else(|| "无法确定用户 home 目录，读不到 ~/.ssh/config".to_string())?;
    let path = ssh_config_path(&home);
    let raw = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(SshHosts {
                hosts: Vec::new(),
                notes: vec![format!(
                    "未找到 {}——你还没有 SSH 主机配置（`~/.ssh/config`）",
                    path.display()
                )],
            })
        }
        Err(e) => return Err(format!("读取 {} 失败：{e}", path.display())),
    };
    let truncated = raw.len() > MAX_CONFIG_BYTES;
    let bytes = if truncated {
        &raw[..MAX_CONFIG_BYTES]
    } else {
        &raw[..]
    };
    // 非法 UTF-8 用替换字符兜住，**不 panic**：配置文件不该让整个面板崩掉。
    let text = String::from_utf8_lossy(bytes);
    let mut parsed = parse_ssh_config(&text);
    if truncated {
        parsed.notes.insert(
            0,
            format!(
                "{} 超过 {} KiB，只读取了前一部分——列出的主机可能不完整",
                path.display(),
                MAX_CONFIG_BYTES / 1024
            ),
        );
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> SshHosts {
        parse_ssh_config(text)
    }

    fn host<'a>(r: &'a SshHosts, alias: &str) -> &'a SshHost {
        r.hosts
            .iter()
            .find(|h| h.alias == alias)
            .unwrap_or_else(|| panic!("未解析出主机 {alias}；实际：{:?}", r.hosts))
    }

    #[test]
    fn parses_basic_host_with_all_display_fields() {
        let r = parse(
            "Host myserver\n  HostName 10.0.0.5\n  User deploy\n  Port 2222\n  \
             ProxyJump bastion\n  IdentityFile ~/.ssh/id_ed25519\n",
        );
        let h = host(&r, "myserver");
        assert_eq!(h.hostname.as_deref(), Some("10.0.0.5"));
        assert_eq!(h.user.as_deref(), Some("deploy"));
        assert_eq!(h.port, Some(2222));
        assert_eq!(h.proxy_jump.as_deref(), Some("bastion"));
        assert_eq!(h.identity_files, vec!["~/.ssh/id_ed25519"]);
        assert!(r.notes.is_empty(), "干净配置不该有降级说明：{:?}", r.notes);
    }

    #[test]
    fn keyword_case_and_equals_form_are_accepted() {
        let r = parse("host a\nHOSTNAME=1.2.3.4\nuser=jane\n");
        let h = host(&r, "a");
        assert_eq!(h.hostname.as_deref(), Some("1.2.3.4"));
        assert_eq!(h.user.as_deref(), Some("jane"));
    }

    #[test]
    fn comment_only_at_line_start() {
        // **实测锚定**（2026-09-15，macOS 自带 OpenSSH）：
        //   `Host a # prod comment` + `HostName 1.2.3.4 # trailing` →
        //   `ssh -F ... -G a` = `hostname 1.2.3.4`；而 `ssh -G '#'` = `hostname #`
        //   ⇒ 行中 `#` 不是注释（`#` 本身成了可匹配的模式），且单值关键字只取**首个**参数。
        // 本解析器与之一致：值只取首个参数，`#` 等额外模式如实保留（另出 notes 解释）。
        let r = parse("Host a\n  HostName 1.2.3.4 # 不是注释\n# Host b\n");
        assert_eq!(host(&r, "a").hostname.as_deref(), Some("1.2.3.4"));
        assert_eq!(r.hosts.len(), 1, "行首的 `# Host b` 才是注释，不得产生主机");
    }

    #[test]
    fn inline_hash_in_host_line_is_kept_but_explained() {
        // 与 OpenSSH 一致地把 `#`/`prod` 当主机名，但必须**解释**，
        // 否则用户在下拉里看到 `#` 会以为壳解析错了。
        let r = parse("Host a # prod\n  User u\n");
        assert_eq!(r.hosts.len(), 3, "OpenSSH 口径：a / # / prod 都是模式");
        assert!(
            r.notes.iter().any(|n| n.contains("只在行首")),
            "必须解释行中 # 的语义：{:?}",
            r.notes
        );
    }

    #[test]
    fn multiple_aliases_share_one_block_comma_and_space() {
        let r = parse("Host a b,c\n  User u\n");
        for alias in ["a", "b", "c"] {
            assert_eq!(host(&r, alias).user.as_deref(), Some("u"), "{alias}");
        }
    }

    #[test]
    fn first_obtained_value_wins() {
        let r = parse("Host a\n  User first\nHost a\n  User second\n");
        assert_eq!(host(&r, "a").user.as_deref(), Some("first"));
    }

    #[test]
    fn wildcard_block_fills_unset_fields_but_is_not_itself_selectable() {
        // 真实语义：`Host *` 的 User 对 a 生效；但 `*` 本身不是可选 alias。
        let r = parse("Host a\n  HostName 1.2.3.4\nHost *\n  User fallback\n");
        assert_eq!(r.hosts.len(), 1, "通配块不得变成可选主机：{:?}", r.hosts);
        assert_eq!(host(&r, "a").user.as_deref(), Some("fallback"));
    }

    #[test]
    fn global_section_applies_and_outranks_later_blocks() {
        // 第一个 Host 之前的关键字对所有主机生效，且因"先出现者优先"胜过后面任何段。
        let r = parse("User global\nHost a\n  HostName 1.2.3.4\n  User local\n");
        assert_eq!(host(&r, "a").user.as_deref(), Some("global"));
        assert_eq!(host(&r, "a").hostname.as_deref(), Some("1.2.3.4"));
    }

    #[test]
    fn negation_excludes_host() {
        let r = parse("Host a\n  User ua\nHost b\n  User ub\nHost * !b\n  Port 2222\n");
        assert_eq!(host(&r, "a").port, Some(2222), "取反模式只排除 b");
        assert_eq!(host(&r, "b").port, None, "b 被取反排除，不该拿到 2222");
        assert_eq!(host(&r, "b").user.as_deref(), Some("ub"));
    }

    #[test]
    fn glob_matching_semantics() {
        assert!(host_matches(&["*".into()], "anything"));
        assert!(host_matches(&["web*".into()], "web1"));
        assert!(host_matches(&["web?".into()], "web1"));
        assert!(!host_matches(&["web?".into()], "web12"));
        assert!(!host_matches(&["web*".into()], "db1"));
        assert!(host_matches(&["*".into(), "!db1".into()], "web1"));
        assert!(!host_matches(&["*".into(), "!db1".into()], "db1"));
        // 大小写不敏感（OpenSSH 主机名匹配口径）。
        assert!(host_matches(&["Web1".into()], "web1"));
    }

    #[test]
    fn match_block_does_not_leak_into_previous_host() {
        let r = parse("Host a\n  User ua\nMatch host b\n  HostName leaked\n");
        assert_eq!(
            host(&r, "a").hostname,
            None,
            "`Match` 段的 HostName 不得被错记到上一个 Host"
        );
        assert!(
            r.notes.iter().any(|n| n.contains("Match")),
            "忽略 Match 必须如实记一笔，不能静默：{:?}",
            r.notes
        );
    }

    #[test]
    fn include_is_not_followed_but_reported() {
        let r = parse("Include ~/.ssh/config.d/*\nHost a\n  HostName 1.2.3.4\n");
        assert_eq!(host(&r, "a").hostname.as_deref(), Some("1.2.3.4"));
        assert!(
            r.notes.iter().any(|n| n.contains("Include")),
            "未跟随 Include 必须如实告知（否则用户以为列表完整）：{:?}",
            r.notes
        );
    }

    #[test]
    fn line_continuation_joins_logical_line() {
        let r = parse("Host a\n  HostName \\\n 1.2.3.4\n");
        // 续行拼接后成为 `HostName 1.2.3.4`（前导空白被 trim）。
        assert_eq!(host(&r, "a").hostname.as_deref(), Some("1.2.3.4"));
    }

    #[test]
    fn trailing_double_backslash_is_literal_not_continuation() {
        let r = parse("Host a\n  HostName x\\\\\n  User u\n");
        assert_eq!(host(&r, "a").hostname.as_deref(), Some("x\\"));
        assert_eq!(host(&r, "a").user.as_deref(), Some("u"));
    }

    // ---------- 畸形输入反例（AGENTS §5：契约字段正反例各一） ----------

    #[test]
    fn malformed_host_without_argument_is_skipped_and_reported() {
        let r = parse("Host\n  User orphan\nHost good\n  User u\n");
        assert_eq!(r.hosts.len(), 1);
        assert_eq!(host(&r, "good").alias, "good");
        assert!(
            r.notes.iter().any(|n| n.contains("没有参数")),
            "畸形 Host 必须记账：{:?}",
            r.notes
        );
    }

    #[test]
    fn malformed_port_is_reported_not_silently_dropped() {
        let r = parse("Host a\n  Port not-a-number\n");
        assert_eq!(host(&r, "a").port, None);
        assert!(
            r.notes.iter().any(|n| n.contains("Port")),
            "非法端口必须记账，否则用户看到的是「没配端口」：{:?}",
            r.notes
        );
    }

    #[test]
    fn port_out_of_range_is_rejected() {
        for bad in ["0", "70000", "-1", ""] {
            let r = parse(&format!("Host a\n  Port {bad}\n"));
            assert_eq!(host(&r, "a").port, None, "端口 {bad:?} 应被拒");
        }
    }

    #[test]
    fn unterminated_quote_takes_rest_and_is_reported() {
        let r = parse("Host a\n  User \"unterminated\n");
        assert_eq!(host(&r, "a").user.as_deref(), Some("unterminated"));
        assert!(
            r.notes.iter().any(|n| n.contains("引号未闭合")),
            "未闭合引号必须记账：{:?}",
            r.notes
        );
    }

    #[test]
    fn quoted_value_keeps_inner_whitespace() {
        let r = parse("Host a\n  HostName \"1.2.3.4 5\"\n");
        assert_eq!(host(&r, "a").hostname.as_deref(), Some("1.2.3.4 5"));
    }

    /// **反例：不得把通配/取反模式当可选 alias** —— 拿它当 `host` 传给 `dsh-ssh`
    /// 会得到一个永远连不上的配置（alias 必须真实存在于解析后的 ssh 视角）。
    #[test]
    fn wildcard_and_negated_patterns_are_never_offered_as_aliases() {
        let r = parse("Host *\n  User u\nHost !bad\n  User v\nHost web?\n  User w\n");
        assert!(
            r.hosts.is_empty(),
            "通配/取反不该产出可选主机：{:?}",
            r.hosts
        );
    }

    #[test]
    fn empty_and_garbage_input_yields_nothing_and_does_not_panic() {
        for text in [
            "",
            "\n\n\n",
            "# only a comment\n",
            "Host a\nHost b\nHost c\n",
            "= \n=Host\n",
            "Host a\n  HostName\n",
        ] {
            let r = parse(text);
            // 不 panic、不产出非法别名为全部要求。
            assert!(r.hosts.iter().all(|h| !h.alias.is_empty()));
        }
    }

    #[test]
    fn trailing_newline_and_crlf_are_tolerated() {
        let r = parse("Host a\r\n  HostName 1.2.3.4\r\n");
        assert_eq!(host(&r, "a").hostname.as_deref(), Some("1.2.3.4"));
    }

    #[test]
    fn ssh_config_path_is_under_home_dot_ssh() {
        // 纯路径推导：不读文件，故可安全断言（真实读取只在 load_ssh_hosts）。
        let p = ssh_config_path(Path::new("/home/u"));
        assert!(p.ends_with(".ssh/config"), "{p:?}");
    }

    /// **真机端到端**（`#[ignore]`）：对**本机真实** `~/.ssh/config` 跑一次完整解析。
    ///
    /// 为什么必须有：ADR-0023 §2.10 明记"无上游范本 ⇒ 正确性完全依赖实机验证"，
    /// 而 `docs/executor.md` G1/G2 是手工项。这条把 G1/G2 的**解析侧**机器化（UI 侧仍需手点）：
    /// 列出真实 alias，并核对"配置里出现 `Include`/`Match` 时有如实降级说明"。
    ///
    /// 用法：`DSH_SSH_E2E=1 cargo test --lib ssh_config::tests::real_user_config -- --ignored --nocapture`
    #[test]
    #[ignore = "真机项：需 DSH_SSH_E2E=1 且本机存在 ~/.ssh/config"]
    fn real_user_config_parses_with_honest_notes() {
        if std::env::var("DSH_SSH_E2E").as_deref() != Ok("1") {
            eprintln!("跳过：未设置 DSH_SSH_E2E=1");
            return;
        }
        // 真机项读**真实** home：`load_ssh_hosts` 用的是 `resolve::user_home()`，
        // 不是测试隔离的 `user_dsh_home()`（后者在 `cfg(test)` 下锁进 `~/.dsh-dock-test`）。
        let home = crate::resolve::user_home().expect("本机应有 home 目录");
        let text =
            std::fs::read_to_string(ssh_config_path(&home)).expect("本机应存在 ~/.ssh/config");
        let parsed = parse_ssh_config(&text);
        eprintln!(
            "真实配置：alias {} 个 → {:?}\n降级说明（{} 条）：{:?}",
            parsed.hosts.len(),
            parsed
                .hosts
                .iter()
                .map(|h| h.alias.as_str())
                .collect::<Vec<_>>(),
            parsed.notes.len(),
            parsed.notes
        );
        assert!(!parsed.hosts.is_empty(), "真实配置里应有可选主机");
        // 诚实降级：出现 `Include` / `Match` 时**必须**有对应说明，否则用户会以为列表是完整的。
        for needle in ["Include", "Match"] {
            let present = text.lines().any(|l| {
                l.trim_start()
                    .to_lowercase()
                    .starts_with(&needle.to_lowercase())
            });
            if present {
                assert!(
                    parsed.notes.iter().any(|n| n.contains(needle)),
                    "配置含 {needle} 时必须如实降级：{:?}",
                    parsed.notes
                );
            }
        }
    }
}
