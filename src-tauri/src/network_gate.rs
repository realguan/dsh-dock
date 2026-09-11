//! network_gate.rs —— 「唯一网络面」（AGENTS §7 / ADR-0006）的**机器闸门**。
//!
//! ## 为什么需要它（2026-09-11）
//!
//! AGENTS §7 把「唯一网络面 = `updates.rs`，其余模块禁触网，新网络需求先登记」
//! 定为纪律，但纪律只活在人的记忆里：`boot.rs::authenticate_workbench_session`
//! 的 `ureq` 调用**落地近一周无人登记**（2026-09-04 落地，2026-09-11 文档巡检
//! 才被肉眼发现并补登记）。本模块把它升级为机器闸门——生产代码里出现网络原语
//! 而未在下方 [`EXEMPTIONS`] 登记 → `cargo test` 红。
//!
//! 范式照抄 `lifecycle.rs` 的 spawn 闸门（`scan_unguarded_spawns` /
//! `production_spawns_go_through_lifecycle_seam`）：**纯函数扫描 + 单测 + 显式豁免**。
//!
//! ## 口径（四条，逐条都有踩坑背景）
//!
//! 1. **判据是「是否发起请求」，不是「是否出现 URL」**。[`PRIMITIVES`] 只认**网络
//!    原语标识符**（网络客户端的类型 / 函数名）。理由：`engines.rs` / `executor.rs`
//!    / `build_policy.rs` / `shell.rs` / `ui.rs` 到处是 `https://…` 字面量（镜像地址、
//!    registry、外链白名单），它们只是常量，一个请求都不发。若按 URL 判，豁免表会被
//!    噪声淹没，闸门随即腐化成"人人加豁免"——比没有闸门更糟。
//! 2. **只扫生产代码**：`#[cfg(test)]` 条目整段剔除（含 `#[cfg(all(test, unix))]`），
//!    而 `#[cfg(not(test))]` / `#[cfg(all(not(test), unix))]` 是**生产代码**，必须照扫。
//!    不这么做，测试里的 mock / 假体网络实现会全量误报。
//! 3. **豁免集中在本文件的表里**（`(文件, 条目, 种类, 理由, 日期)`），**不在别人文件
//!    里插内联标注**：网络面横跨多个域（`plugins.rs` 属管理域），内联标注等于让
//!    "过一次闸门"的改动散落到非本域文件——与 spawn 闸门的 `// spawn-gate: exempt(...)`
//!    取舍相反，这里刻意选集中表（成因见 `docs/team/网络面闸门-2026-09-11.md`）。
//! 4. **扫描前把 `\r\n` 归一到 `\n`**：2026-09-11 v1.1.1 发版实测教训——Git for
//!    Windows 的 `autocrlf` 会把源码 checkout 成 CRLF，`include_str!` 拿到 `\r\n`，
//!    以源码文本为判据的闸门（spawn 闸门）模式失配、在 Windows 上恒红。凡"以源码
//!    文本为判据"的闸门都必须自己扛住行尾（`.gitattributes` 是补充不是替代）。
//!
//!    > **实测留痕（2026-09-11 变异测试）**：去掉本行归一后，CRLF 用例**仍然绿**
//!    > ——因为本闸门是**行式**设计（`.lines()` 切分、谓词先 `trim()`、`\r` 被
//!    > `is_ascii_whitespace()` 吃掉、花括号配对只看 ASCII 结构字符），天然抗 CRLF。
//!    > 故归一在此属**纵深防御**而非唯一防线；保留它是为了守口径 + 防未来有人把扫描
//!    > 改成"整段子串匹配 / 锚定 `$`"时重新踩坑。`crlf_sources_are_judged_identically`
//!    > 钉的是"两种行尾判定逐条一致"这条不变量（无论由哪一层提供），不是某一行的实现。
//!
//! ## 维护本文件时的一条纪律
//!
//! 本文件由 `lib.rs` 以 `#[cfg(test)] mod network_gate;` 挂载，**整个文件都是测试代码**；
//! 但闸门的扫描面是"逐个 `.rs` 文件"，**不跨文件追踪挂载点**——故本文件的**模块作用域**
//! 会被它自己当作生产代码扫描。因此：
//!
//! - 一切**含网络原语的示例/夹具必须留在 `#[cfg(test)] mod tests` 内**（会被正确剔除）；
//! - [`PRIMITIVES`] 表里的模式串是**字符串字面量**，代码视图会抹掉，故不触发自己；
//! - 若把夹具搬到模块作用域，本闸门会**误报本文件**（可见的红，不是漏扫）——
//!   首版给本文件加"自我豁免"时，`exemptions_are_live` 判定它是空条目并要求删除，
//!   说明**当前不需要任何自我豁免**。别再加回去。

use std::path::{Path, PathBuf};

// ---------- 网络原语清单 ----------

/// 匹配方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchMode {
    /// 标识符词边界（避免 `my_ureq_client` 这类名字误命中）；只在**代码**里判——
    /// 字符串里的 `ureq::get` 是数据不是调用。
    Word,
    /// 子串，同样只在**代码**里判。
    Substr,
    /// 子串，但**只在字符串字面量里**判（`"curl"` 必须带引号才有意义）。
    ///
    /// 这条模式是"两个文本视图"存在的原因：标识符模式判代码视图（字面量被抹掉），
    /// argv0 模式判字面量保留视图——否则一句用户文案里出现 `ureq::` 就会误报。
    Argv0,
}

/// 网络原语：`(模式串, 匹配方式, 说明)`。
///
/// **收录判据**：它一出现就**有可能在 in-process 发起网络请求**。收录刻意保守——
/// 每条都要能给出"为什么它算网络面"的一句话，不接受"顺手加的"。
///
/// **刻意不收录**（避免噪声 → 避免豁免泛滥）：`http://` / `https://` 字面量、
/// `SocketAddr` / `IpAddr`（纯类型，不发请求）、`.connect(`（Tauri / DB / 通道
/// 都会用）。子进程式触网只收 `"curl"` / `"wget"` 两个 argv0 字面量——其余
/// 子进程网络能力（如 `pnpm`）由 `lifecycle` 的 spawn 闸门与 AGENTS §7 登记管。
const PRIMITIVES: &[(&str, MatchMode, &str)] = &[
    (
        "ureq",
        MatchMode::Word,
        "ureq 阻塞客户端（唯一网络面的传输层）",
    ),
    (
        "reqwest",
        MatchMode::Word,
        "reqwest 客户端（tauri-plugin-updater 的传输层）",
    ),
    ("hyper::", MatchMode::Substr, "hyper（HTTP 栈）"),
    ("tonic::", MatchMode::Substr, "tonic（gRPC）"),
    (
        "UpdaterExt",
        MatchMode::Word,
        "tauri-plugin-updater 的发起接口（check / download / install）",
    ),
    ("TcpStream", MatchMode::Word, "std::net 原始 TCP"),
    ("TcpListener", MatchMode::Word, "std::net 监听"),
    ("UdpSocket", MatchMode::Word, "std::net UDP"),
    (
        "\"curl\"",
        MatchMode::Argv0,
        "子进程 curl（绕过 Rust 客户端触网）",
    ),
    ("\"wget\"", MatchMode::Argv0, "子进程 wget"),
];

// ---------- 豁免表（集中，唯一事实源 = AGENTS §7） ----------

/// 豁免种类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// **授予豁免**：该文件 / 条目允许出现网络原语。
    Exempt,
    /// **仅在册**（AGENTS §7 登记为**子进程编排**型网络）：网络在子进程
    /// （`pnpm` / `wsl.exe`）内发起，本文件当前**零 in-process 原语**。
    ///
    /// 刻意**不授予豁免**：给一个今天没有任何原语的文件开整文件豁口，等于预授权
    /// 它未来的任意触网；而闸门的存在意义正是"让每一次触网都被看见"。
    /// 有 `registered_entries_have_no_in_process_primitives` 测试钉住"今天确实零命中"。
    Registered,
}

/// 一条豁免 / 在册记录。`file` 形如 `src/updates.rs`（相对 `src-tauri/`，`/` 分隔）。
#[derive(Debug, Clone)]
struct NetworkEntry {
    file: &'static str,
    /// `None` = 整文件；`Some(ident)` = **仅该函数**（比整文件精确，能挡住"顺手在
    /// 同一文件里再加一处"）。目前只支持函数级（见 `item_ranges`）。
    item: Option<&'static str>,
    kind: Kind,
    /// 为什么它可以触网（必须与 AGENTS §7 的登记口径一致）。
    reason: &'static str,
    /// 落表日期（AGENTS §4.2：裁定性记录带日期）。
    date: &'static str,
}

/// **唯一事实源 = AGENTS §7「IPC 与网络面」登记册**（不含本闸门自己的成因，那是
/// 模块头部注释的职责）。
///
/// 新增网络面时的正确动作有两条，且都要过这张表：
/// 1. 先在 AGENTS §7 登记（人类可读的唯一事实源）；
/// 2. 再在这里加一行（机器可读的投影）——`exemptions_are_live` 测试会拒绝
///    加进来却从不命中的空条目，`registered_entries_have_no_in_process_primitives`
///    会拒绝"在册却把原语放进去"的偷跑。
const EXEMPTIONS: &[NetworkEntry] = &[
    NetworkEntry {
        file: "src/updates.rs",
        item: None,
        kind: Kind::Exempt,
        reason: "唯一网络面本体（ADR-0006）：ureq 客户端的唯一出口（UreqFetcher）",
        date: "2026-09-11",
    },
    NetworkEntry {
        file: "src/updater.rs",
        item: None,
        kind: Kind::Exempt,
        reason: "客户端自更新（AGENTS §7）：驱动 tauri-plugin-updater 的 check / download / install",
        date: "2026-09-11",
    },
    NetworkEntry {
        file: "src/boot.rs",
        item: Some("authenticate_workbench_session"),
        kind: Kind::Exempt,
        // 条目级而非整文件：boot.rs 是启动主路径，除这一处外不得触网。
        reason: "工作台 Token 环回兑换（AGENTS §7 2026-09-11 补登记）：127.0.0.1 GET、5s、redirects=0",
        date: "2026-09-11",
    },
    NetworkEntry {
        file: "src/plugins.rs",
        // 2026-09-11 收窄（task-19）：整文件 → 条目级。plugins.rs 生产段全模式
        // grep 仅一处原语命中（`ureq::post`，见 `fetch_runtime_snapshot`），
        // 整文件豁免会让**将来任何人往本文件加第二处触网都不被拦下**——
        // 正是 boot.rs 那次漏登的成因形态（"这文件本来就能触网"的模糊感）。
        item: Some("fetch_runtime_snapshot"),
        kind: Kind::Exempt,
        reason: "插件运行态回环只读查询（AGENTS §7 2026-08-29）：POST 127.0.0.1 /api/pluginInventory/list，2s",
        date: "2026-09-11",
    },
    NetworkEntry {
        file: "src/engines.rs",
        item: None,
        kind: Kind::Registered,
        reason: "引擎引导（AGENTS §7）：网络在 pnpm 子进程内（runtime set node / add -g），本文件无 in-process 原语",
        date: "2026-09-11",
    },
    NetworkEntry {
        file: "src/executor.rs",
        item: None,
        kind: Kind::Registered,
        reason: "WSL 客体投递（AGENTS §7）：网络在客体进程内，本文件无 in-process 原语",
        date: "2026-09-11",
    },
    // 刻意**没有** `src/network_gate.rs` 自己：本文件的原语示例全部写在
    // `#[cfg(test)] mod tests` 里（会被 `cfg_test_ranges` 正确剔除），而表里的模式串
    // 是**字符串字面量**（代码视图会抹掉）。首版曾给自己加一条豁免，
    // `exemptions_are_live` 当场判定它是空条目并删掉——这正是那条自检存在的意义。
];

// ---------- 词法：注释 / 字面量 / 代码三态 ----------

/// 逐**字节**的词法类别（长度 == `text.len()`，因为 Rust 的结构字符全是 ASCII）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Lex {
    /// 真正的代码：只有它参与花括号配对（`"}"` 里的花括号不是结构）。
    Code,
    /// 字符串 / 字符 / 原始字符串字面量内部：花括号不算结构，但**要参与原语扫描**
    /// ——`Command::new("curl")` 的 `"curl"` 就藏在字面量里。
    Literal,
    /// 注释（行注释 / **可嵌套**块注释）。扫描与配对都跳过。
    Comment,
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

/// 判断 `i` 处是不是原始字符串（`r"…"` / `r#"…"#` / `br"…"`）的起始，返回
/// `(井号个数, 引导字节数)`——引导字节数含开启引号本身。
fn raw_string_start(b: &[u8], i: usize) -> Option<(usize, usize)> {
    let n = b.len();
    // `r` 不能是标识符的一部分（否则 `var"…"` 之类会被误判；虽非合法 Rust，但
    // 这里追求"宁可判错成代码"的安全方向）。
    if i > 0 && is_ident_byte(b[i - 1]) {
        return None;
    }
    let mut p = i;
    if b[p] == b'b' {
        p += 1;
        if p >= n || b[p] != b'r' {
            return None;
        }
    } else if b[p] != b'r' {
        return None;
    }
    p += 1;
    let mut hashes = 0usize;
    while p < n && b[p] == b'#' {
        hashes += 1;
        p += 1;
    }
    if p < n && b[p] == b'"' {
        Some((hashes, p - i + 1))
    } else {
        None
    }
}

/// 字符字面量长度（含首尾引号）；`None` = 这是**生命周期标注**（`'a`），不是字面量。
///
/// 必须区分：`'{'` 的 `{` 在字符字面量里（非结构），而 `'a` 的 `'` 是代码。
fn char_literal_len(b: &[u8], i: usize) -> Option<usize> {
    let n = b.len();
    if i + 1 >= n {
        return None;
    }
    if b[i + 1] == b'\\' {
        // 转义：`'\n'`、`'\''`、`'\u{1F600}'`——扫到下一个 `'` 为止（上限 12 字节）。
        let end = (i + 13).min(n);
        if let Some(rel) = b[i + 2..end].iter().position(|&c| c == b'\'') {
            return Some(rel + 3); // (i + 2 + rel) - i + 1
        }
        return None;
    }
    let first = b[i + 1];
    if first < 0x80 {
        if i + 2 < n && b[i + 2] == b'\'' {
            return Some(3);
        }
        return None;
    }
    // 多字节 char：按 UTF-8 首字节推长度后要求紧跟闭引号。
    let extra = match first {
        0xF0..=0xF7 => 4,
        0xE0..=0xEF => 3,
        0xC0..=0xDF => 2,
        _ => return None,
    };
    if i + extra < n && b[i + extra] == b'\'' {
        Some(extra + 2)
    } else {
        None
    }
}

/// 逐字符分类。手写状态机（不引依赖，AGENTS §5）：处理行注释、可嵌套块注释、
/// 普通 / 原始 / 字节字符串、字符字面量与生命周期。
fn classify(text: &str) -> Vec<Lex> {
    let b = text.as_bytes();
    let n = b.len();
    let mut out = vec![Lex::Code; n];
    let mut i = 0usize;
    while i < n {
        let c = b[i];
        // 行注释
        if c == b'/' && i + 1 < n && b[i + 1] == b'/' {
            out[i] = Lex::Comment;
            out[i + 1] = Lex::Comment;
            i += 2;
            while i < n && b[i] != b'\n' {
                out[i] = Lex::Comment;
                i += 1;
            }
            continue;
        }
        // 块注释（可嵌套）
        if c == b'/' && i + 1 < n && b[i + 1] == b'*' {
            out[i] = Lex::Comment;
            out[i + 1] = Lex::Comment;
            i += 2;
            let mut depth = 1usize;
            while i < n && depth > 0 {
                if b[i] == b'/' && i + 1 < n && b[i + 1] == b'*' {
                    out[i] = Lex::Comment;
                    out[i + 1] = Lex::Comment;
                    depth += 1;
                    i += 2;
                    continue;
                }
                if b[i] == b'*' && i + 1 < n && b[i + 1] == b'/' {
                    out[i] = Lex::Comment;
                    out[i + 1] = Lex::Comment;
                    depth -= 1;
                    i += 2;
                    continue;
                }
                out[i] = Lex::Comment;
                i += 1;
            }
            continue;
        }
        // 原始字符串（含字节原始字符串）
        if let Some((hashes, lead)) = raw_string_start(b, i) {
            for slot in out.iter_mut().take((i + lead).min(n)).skip(i) {
                *slot = Lex::Literal;
            }
            i += lead;
            while i < n {
                out[i] = Lex::Literal;
                if b[i] == b'"' {
                    let mut ok = i + 1 + hashes <= n;
                    for k in 0..hashes {
                        if i + 1 + k >= n || b[i + 1 + k] != b'#' {
                            ok = false;
                            break;
                        }
                    }
                    if ok {
                        for k in 0..hashes {
                            out[i + 1 + k] = Lex::Literal;
                        }
                        i += 1 + hashes;
                        break;
                    }
                }
                i += 1;
            }
            continue;
        }
        // 普通字符串 / 字节字符串（`b"…"` 的 `b` 留在 Code，无害）
        if c == b'"' {
            out[i] = Lex::Literal;
            i += 1;
            while i < n {
                out[i] = Lex::Literal;
                if b[i] == b'\\' && i + 1 < n {
                    out[i + 1] = Lex::Literal;
                    i += 2;
                    continue;
                }
                if b[i] == b'"' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            continue;
        }
        // 字符字面量 vs 生命周期
        if c == b'\'' {
            if let Some(len) = char_literal_len(b, i) {
                for slot in out.iter_mut().take((i + len).min(n)).skip(i) {
                    *slot = Lex::Literal;
                }
                i += len;
            } else {
                i += 1; // 生命周期：留在 Code
            }
            continue;
        }
        i += 1;
    }
    out
}

fn structural_mask(lex: &[Lex]) -> Vec<bool> {
    lex.iter().map(|l| *l == Lex::Code).collect()
}

/// 按词法类别抹空白（保留换行 → 行号不变）。`blank_literals` 决定**是否连字符串
/// 字面量一起抹**：判标识符原语时要抹（字符串里的 `ureq::get` 是数据不是调用），
/// 判 `"curl"` 式 argv0 原语时不能抹（它就藏在字面量里）。
fn blanked(text: &str, lex: &[Lex], blank_literals: bool) -> String {
    let mut b = text.as_bytes().to_vec();
    for (i, l) in lex.iter().enumerate() {
        let wipe = match l {
            Lex::Comment => true,
            Lex::Literal => blank_literals,
            Lex::Code => false,
        };
        if wipe && b[i] != b'\n' && b[i] != b'\r' {
            b[i] = b' ';
        }
    }
    String::from_utf8(b).expect("抹注释/字面量只把整字节替换成空格，不应破坏 UTF-8")
}

/// 把若干字节区间抹成空白（保留换行 → **行号恒等于原始行号**）。
///
/// 区间边界全部落在 ASCII 定界符上（`#` / `]` / `;` / `}` / `fn`），故不会切开
/// 多字节字符。
fn blank_ranges(text: &str, ranges: &[(usize, usize)]) -> String {
    let mut b = text.as_bytes().to_vec();
    for &(s, e) in ranges {
        let e = e.min(b.len());
        for byte in b.iter_mut().take(e).skip(s) {
            if *byte != b'\n' && *byte != b'\r' {
                *byte = b' ';
            }
        }
    }
    String::from_utf8(b).expect("区间边界均在 ASCII 定界符上，不应破坏 UTF-8")
}

// ---------- cfg 谓词求值（`#[cfg(test)]` 的精确判定） ----------

/// `not(…)` / `any(…)` / `all(…)` 的外层剥离。
fn strip_call<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    let rest = s.strip_prefix(name)?.trim_start();
    rest.strip_prefix('(')?.strip_suffix(')')
}

/// 顶层逗号切分（尊重括号嵌套）。
fn split_top_args(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

/// 求值一个 cfg 谓词，`test` = 本次编译是否在测试档。
///
/// 与 `test` 无关的谓词（`unix` / `windows` / `feature = "x"` / `target_os = "…"`）
/// 一律按"该平台成立"处理——本闸门只关心"这段代码在**非测试**构建里会不会被编译"，
/// 不关心具体平台（平台分叉由 CI 三平台各自编译兜底）。
fn cfg_eval(pred: &str, test: bool) -> bool {
    let p = pred.trim();
    if p == "test" {
        return test;
    }
    if let Some(inner) = strip_call(p, "not") {
        return !cfg_eval(inner, test);
    }
    if let Some(inner) = strip_call(p, "any") {
        return split_top_args(inner).iter().any(|a| cfg_eval(a, test));
    }
    if let Some(inner) = strip_call(p, "all") {
        return split_top_args(inner).iter().all(|a| cfg_eval(a, test));
    }
    true
}

/// 求值一个 cfg 谓词，回答「它是不是**测试专用**」＝
/// 「测试档会编译 ∧ 非测试档不会编译」。
///
/// **必须求值而不是"含 test 就算"**：`#[cfg(not(test))]`（`resolve.rs:91` 实际用法）
/// 与 `#[cfg(all(not(test), unix))]` 都是**生产代码**，按"含 test"判会被整段漏扫
/// ——那正是本闸门要防的失败方向（漏扫比误报危险得多）。反过来 `#[cfg(any(test, unix))]`
/// 含 `test` 但在 unix 生产构建里同样编译，也**不是**测试专用。
fn cfg_pred_is_test(pred: &str) -> bool {
    !cfg_eval(pred, false) && cfg_eval(pred, true)
}

/// 属性文本（`[...]` 内部）是不是一个「测试专用」的 `cfg`。
fn attr_cfg_is_test(attr: &str) -> bool {
    let a = attr.trim();
    let Some(rest) = a.strip_prefix("cfg") else {
        return false; // `#[test]` / `#[allow(…)]` / `#[derive(…)]` 都不是 cfg
    };
    let rest = rest.trim_start();
    let Some(inner) = rest.strip_prefix('(').and_then(|x| x.strip_suffix(')')) else {
        return false;
    };
    cfg_pred_is_test(inner)
}

// ---------- 源码区间定位 ----------

/// 从 `open`（值为 `o`）找到配对的 `c` 的字节下标（只看结构字符）。
fn matching_bracket(bytes: &[u8], m: &[bool], open: usize, o: u8, c: u8) -> Option<usize> {
    let mut depth = 0usize;
    for i in open..bytes.len() {
        if !m[i] {
            continue;
        }
        if bytes[i] == o {
            depth += 1;
        } else if bytes[i] == c {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(i);
            }
        }
    }
    None
}

/// 从 `from` 起跳过空白 / 注释 / 后续属性，定位**条目**并返回其结束字节（不含）。
///
/// 条目结束 = 首个 `()`/`[]` 深度为 0 的 `;`，或首个花括号块的配对 `}`（若紧随
/// `;` 则把它也算进去，兼容 `use a::{b};`）。
fn item_end_after(bytes: &[u8], m: &[bool], from: usize) -> Option<usize> {
    let n = bytes.len();
    let mut i = from;
    while i < n {
        if !m[i] {
            i += 1; // 注释 / 字面量一律跳过
            continue;
        }
        let c = bytes[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if c == b'#' {
            let mut j = i + 1;
            while j < n && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            if j < n && bytes[j] == b'[' && m[j] {
                i = matching_bracket(bytes, m, j, b'[', b']')? + 1;
                continue;
            }
        }
        break;
    }

    let mut depth = 0usize;
    let mut k = i;
    while k < n {
        if !m[k] {
            k += 1;
            continue;
        }
        match bytes[k] {
            b'(' | b'[' => depth += 1,
            b')' | b']' => depth = depth.saturating_sub(1),
            b';' if depth == 0 => return Some(k + 1),
            b'{' if depth == 0 => {
                let close = matching_bracket(bytes, m, k, b'{', b'}')?;
                let mut t = close + 1;
                while t < n && !m[t] {
                    t += 1;
                }
                return Some(if t < n && bytes[t] == b';' {
                    t + 1
                } else {
                    close + 1
                });
            }
            _ => {}
        }
        k += 1;
    }
    None
}

/// 找出所有「测试专用 `#[cfg(…)]`」属性所辖条目的字节区间。
fn cfg_test_ranges(text: &str, m: &[bool]) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < n {
        if m[i] && bytes[i] == b'#' {
            let mut j = i + 1;
            while j < n && (bytes[j] == b' ' || bytes[j] == b'\t') {
                j += 1;
            }
            if j < n && bytes[j] == b'[' && m[j] {
                if let Some(close) = matching_bracket(bytes, m, j, b'[', b']') {
                    if attr_cfg_is_test(&text[j + 1..close]) {
                        if let Some(end) = item_end_after(bytes, m, close + 1) {
                            out.push((i, end));
                            i = end;
                            continue;
                        }
                    }
                    i = close + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

/// 找 `fn <ident>` 条目的字节区间（只认函数；同一 ident 的多处定义全部返回）。
///
/// 词边界检查是必需的：找 `fn a` 时不得命中 `fn abc`；而文档注释里写
/// `fn authenticate_workbench_session` 也不算（结构掩码已排除注释）。
fn item_ranges(text: &str, m: &[bool], ident: &str) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let needle = format!("fn {ident}");
    let mut out = Vec::new();
    for (i, _) in text.match_indices(&needle) {
        if i > 0 && is_ident_byte(bytes[i - 1]) {
            continue;
        }
        let after = i + needle.len();
        if after < bytes.len() && is_ident_byte(bytes[after]) {
            continue;
        }
        if !m.get(i).copied().unwrap_or(false) {
            continue; // 注释 / 字面量里的伪命中
        }
        if let Some(end) = item_end_after(bytes, m, i) {
            out.push((i, end));
        }
    }
    out
}

// ---------- 扫描 ----------

fn contains_word(hay: &str, needle: &str) -> bool {
    let b = hay.as_bytes();
    let mut from = 0usize;
    while let Some(rel) = hay[from..].find(needle) {
        let i = from + rel;
        let before_ok = i == 0 || !is_ident_byte(b[i - 1]);
        let after = i + needle.len();
        let after_ok = after >= b.len() || !is_ident_byte(b[after]);
        if before_ok && after_ok {
            return true;
        }
        from = i + 1;
    }
    false
}

/// 命中任一原语则返回其模式串。
///
/// `code_line` = 抹掉注释与字面量的行（判标识符原语）；`argv0_line` = 只抹注释的行
/// （判 `"curl"` 式 argv0 原语）。两个视图行数一致（抹空白保留换行）。
fn first_primitive(code_line: &str, argv0_line: &str) -> Option<&'static str> {
    PRIMITIVES.iter().find_map(|(pat, mode, _)| {
        let hit = match mode {
            MatchMode::Word => contains_word(code_line, pat),
            MatchMode::Substr => code_line.contains(pat),
            MatchMode::Argv0 => argv0_line.contains(pat),
        };
        hit.then_some(*pat)
    })
}

/// **纯函数闸门**：扫描一份源码文本，返回未登记的网络原语命中（`文件:行号: 内容`）。
///
/// 命中判定的完整口径：`(生产代码 ∧ 非注释)` 里出现 [`PRIMITIVES`] 任一模式，且该
/// 文件 / 条目不在 [`EXEMPTIONS`] 里。
fn scan_unguarded_network(file: &str, text: &str, table: &[NetworkEntry]) -> Vec<String> {
    // 口径 4：行尾归一（CRLF 检出不得影响判定）。
    let text = text.replace("\r\n", "\n");

    // 整文件豁免（条目级豁免不短路，它只抹掉那一段）。
    if table
        .iter()
        .any(|e| e.file == file && e.kind == Kind::Exempt && e.item.is_none())
    {
        return Vec::new();
    }

    let lex = classify(&text);
    let m = structural_mask(&lex);
    let mut ranges = cfg_test_ranges(&text, &m);
    for e in table
        .iter()
        .filter(|e| e.file == file && e.kind == Kind::Exempt)
    {
        if let Some(ident) = e.item {
            ranges.extend(item_ranges(&text, &m, ident));
        }
    }

    // 抹掉测试条目与条目级豁免后，再按两个视图扫描（行号全程不变）。
    let prod = blank_ranges(&text, &ranges);
    let prod_lex = classify(&prod);
    let code = blanked(&prod, &prod_lex, true);
    let argv0 = blanked(&prod, &prod_lex, false);

    let mut out = Vec::new();
    for (idx, (code_line, argv0_line)) in code.lines().zip(argv0.lines()).enumerate() {
        if let Some(pat) = first_primitive(code_line, argv0_line) {
            out.push(format!(
                "{}:{}: [{}] {}",
                file,
                idx + 1,
                pat,
                argv0_line.trim()
            ));
        }
    }
    out
}

// ---------- 源码集（运行时遍历，不用 include_str! 清单） ----------

struct SourceFile {
    /// 相对 `src-tauri/` 的路径，`/` 分隔（与豁免表同口径，Windows 上亦稳定）。
    name: String,
    path: PathBuf,
}

/// 遍历 `src/**/*.rs`。
///
/// **刻意不用 `include_str!` 白名单**（spawn 闸门用的是白名单）：本闸门要防的正是
/// "新加的触网代码没被登记"，若白名单也需要人记得加，就等于把同一个漏洞换个地方
/// 再犯一次（新增文件里的网络调用会被静默漏扫）。运行时遍历让**新文件自动在管辖内**。
fn source_files() -> Vec<SourceFile> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = Vec::new();
    collect(&root, &root, &mut out);
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<SourceFile>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let p = entry.path();
        if p.is_dir() {
            collect(root, &p, out);
        } else if p.extension().and_then(|x| x.to_str()) == Some("rs") {
            let rel = p
                .strip_prefix(root)
                .unwrap_or(&p)
                .to_string_lossy()
                .replace('\\', "/");
            out.push(SourceFile {
                name: format!("src/{rel}"),
                path: p,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(name: &str) -> String {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("读取 {} 失败：{e}", p.display()))
    }

    /// 只留除 `entry` 之外的豁免表（用来证明"这条豁免确实在挡东西"）。
    fn table_without(entry: &NetworkEntry) -> Vec<NetworkEntry> {
        EXEMPTIONS
            .iter()
            .filter(|e| !(e.file == entry.file && e.item == entry.item))
            .cloned()
            .collect()
    }

    // ---------- 表自身的形状 ----------

    /// 表条目必须**可解释**：理由非空、日期规范（AGENTS §4.2：裁定性记录带日期）。
    ///
    /// 这条同时是"字段确实被消费"的证据——一张只有 `file` 被读、理由与日期无人看的
    /// 表，会迅速退化成"随手加豁免"的工具。
    #[test]
    fn exemption_table_entries_are_well_formed() {
        for e in EXEMPTIONS {
            assert!(
                e.reason.len() >= 12,
                "{} 的理由太短（必须能解释「为什么它可以触网」）：{:?}",
                e.file,
                e.reason
            );
            assert!(
                e.reason.contains("AGENTS §7")
                    || e.reason.contains("ADR-")
                    || e.reason.contains("闸门自身"),
                "{} 的理由必须指向登记处（AGENTS §7 / ADR）或说明自己是闸门自身：{:?}",
                e.file,
                e.reason
            );
            assert_eq!(e.date.len(), 10, "{} 的日期应为 YYYY-MM-DD", e.file);
            assert_eq!(
                e.date.chars().filter(|c| *c == '-').count(),
                2,
                "{} 的日期格式应为 YYYY-MM-DD：{:?}",
                e.file,
                e.date
            );
            assert!(e.date.starts_with("202"), "{} 的日期应为本世纪", e.file);
        }
        // 原语清单同样要求逐条可解释（说明非空 = 不是顺手加的）。
        for (pat, _, why) in PRIMITIVES {
            assert!(!why.is_empty(), "网络原语 {pat} 缺说明");
        }
        // 同一个 (文件, 条目) 不得重复登记（重复会掩盖"哪条在起作用"）。
        for (i, a) in EXEMPTIONS.iter().enumerate() {
            for b in EXEMPTIONS.iter().skip(i + 1) {
                assert!(
                    !(a.file == b.file && a.item == b.item),
                    "豁免表有重复条目：{} {:?}",
                    a.file,
                    a.item
                );
            }
        }
    }

    // ---------- 主闸门 ----------

    /// **机器闸门**：生产代码里的网络原语必须在 [`EXEMPTIONS`] 里登记。
    ///
    /// 与 AGENTS §7 的关系：§7 是**人类可读的唯一事实源**（登记册），本测试是它的
    /// **机器投影**。两者不一致时，以"同步两者"为正确动作——不许只改一边。
    #[test]
    fn production_network_primitives_are_registered() {
        let files = source_files();
        assert!(
            files.len() >= 25,
            "源码遍历只找到 {} 个 .rs 文件——路径大概率错了（错路径会让闸门静默全绿）",
            files.len()
        );
        for must in [
            "src/lib.rs",
            "src/updates.rs",
            "src/boot.rs",
            "src/plugins.rs",
        ] {
            assert!(
                files.iter().any(|f| f.name == must),
                "源码遍历未覆盖 {must}"
            );
        }
        // 豁免表里的路径必须真实存在：写错路径 = 豁免静默失效（该文件照旧被扫，
        // 结果是红；但若写错的是"整文件豁免"，红会指向错的文件，排查成本高）。
        for e in EXEMPTIONS {
            assert!(
                files.iter().any(|f| f.name == e.file),
                "豁免表里的 {} 不在源码集里——路径写错了",
                e.file
            );
        }

        let violations: Vec<String> = files
            .iter()
            .flat_map(|f| {
                let text = std::fs::read_to_string(&f.path)
                    .unwrap_or_else(|e| panic!("读取 {} 失败：{e}", f.path.display()));
                scan_unguarded_network(&f.name, &text, EXEMPTIONS)
            })
            .collect();
        assert!(
            violations.is_empty(),
            "以下生产代码出现网络原语但未在 AGENTS §7 登记（唯一网络面 = updates.rs，ADR-0006）：\n{}\n\
             正确动作：① 能在 updates.rs 内完成 → 搬过去；② 确需就地触网 → 先在 \
             AGENTS §7 登记（人类可读），再在 src-tauri/src/network_gate.rs 的 \
             EXEMPTIONS 表加一行（理由 + 日期），两处必须同步。",
            violations.join("\n")
        );
    }

    /// **豁免表自检**：每条 `Exempt` 都必须**真的在挡东西**（否则是空条目 / 过宽条目）。
    ///
    /// 为什么需要：豁免表最大的腐化方式不是"漏加"，而是"加了却不再需要"——留下一条
    /// 宽豁免，等于给未来任意触网预先发了通行证。把条目摘掉后必须真的报违规，
    /// 才证明它今天仍在被使用。
    #[test]
    fn exemptions_are_live() {
        for e in EXEMPTIONS.iter().filter(|e| e.kind == Kind::Exempt) {
            let text = read(e.file);
            let without = table_without(e);
            let hits = scan_unguarded_network(e.file, &text, &without);
            assert!(
                !hits.is_empty(),
                "豁免条目已失效或过宽：{}（item={:?}）摘掉后仍无任何命中——\
                 说明它不再挡任何东西，请删除该条目",
                e.file,
                e.item
            );
            // 反过来：带上它必须干净（与主闸门一致，此处给出更聚焦的报错）。
            let with = scan_unguarded_network(e.file, &text, EXEMPTIONS);
            assert!(
                with.is_empty(),
                "豁免条目未生效：{}（item={:?}）仍报出 {:?}",
                e.file,
                e.item,
                with
            );
        }
    }

    /// **在册 ≠ 豁免**：AGENTS §7 把 `engines.rs` / `executor.rs` 登记为**子进程编排**
    /// 型网络（网络在 `pnpm` / `wsl.exe` 内发起），它们当前**不得**有 in-process 原语。
    ///
    /// 这条测试把"登记"与"授权"分开：一旦有人在引擎引导里直接 `ureq::get`，这里会红，
    /// 逼出一个显式决定（要么把网络收回 `updates.rs`，要么把该条改成 `Kind::Exempt`
    /// 并在 AGENTS §7 补登记理由）。
    #[test]
    fn registered_entries_have_no_in_process_primitives() {
        for e in EXEMPTIONS.iter().filter(|e| e.kind == Kind::Registered) {
            let hits = scan_unguarded_network(e.file, &read(e.file), EXEMPTIONS);
            assert!(
                hits.is_empty(),
                "{} 被 AGENTS §7 登记为子进程编排型网络，但出现了 in-process 原语：\n{}\n\
                 若确实要在这里 in-process 触网：先把该条改成 Kind::Exempt 并写理由，\
                 同时同步 AGENTS §7。",
                e.file,
                hits.join("\n")
            );
        }
    }

    // ---------- 单向性证明：闸门真的能拦住 ----------

    /// 合成片段：**未豁免文件**里的裸网络原语必须被拦下。
    ///
    /// 这是"单向性"证明的一半——绿色的现状 ❌ 不等于闸门有效（它可能只是没扫到）。
    /// 另一半是 `unguarded_network_primitive_inside_test_module_is_not_reported`。
    #[test]
    fn unguarded_network_primitive_in_synthetic_source_is_reported() {
        const SYNTHETIC: &str = r#"
use std::io::Write;

pub fn fetch_latest() -> String {
    let resp = ureq::get("https://example.com/latest.json")
        .timeout(std::time::Duration::from_secs(3))
        .call()
        .unwrap();
    let body = resp.into_string().unwrap();
    let client = reqwest::blocking::Client::new();
    let mut raw = std::net::TcpStream::connect("example.com:80").unwrap();
    let _ = raw.write_all(body.as_bytes());
    let _ = client;
    body
}
"#;
        let hits = scan_unguarded_network("src/synthetic_violation.rs", SYNTHETIC, EXEMPTIONS);
        assert!(
            !hits.is_empty(),
            "闸门没有拦住未登记的裸网络原语——单向性证明失败（闸门形同虚设）"
        );
        assert!(
            hits.iter().any(|h| h.contains("ureq")),
            "应报出 ureq 命中，实际：{hits:?}"
        );
        assert!(
            hits.iter().any(|h| h.contains("TcpStream")),
            "应报出 TcpStream 命中，实际：{hits:?}"
        );
    }

    /// 合成片段：**子进程式触网**（`curl` / `wget`）同样要拦——它绕过了 Rust 客户端，
    /// 但仍在发起网络请求。
    #[test]
    fn subprocess_network_binaries_are_reported() {
        const SYNTHETIC: &str = r#"
pub fn fetch_via_shell(url: &str) {
    let mut c = crate::child_cmd(Path::new("curl"));
    c.arg(url);
    let _ = c.status();
    let mut w = crate::child_cmd(Path::new("wget"));
    let _ = w.arg(url).status();
}
"#;
        let hits = scan_unguarded_network("src/synthetic_shell.rs", SYNTHETIC, EXEMPTIONS);
        assert!(hits.iter().any(|h| h.contains("curl")), "实际：{hits:?}");
        assert!(hits.iter().any(|h| h.contains("wget")), "实际：{hits:?}");
    }

    // ---------- 只扫生产代码 ----------

    /// `#[cfg(test)]` 模块里的网络用法**不是**违规（否则每个 mock / 假体都要进豁免表）。
    #[test]
    fn unguarded_network_primitive_inside_test_module_is_not_reported() {
        const SYNTHETIC: &str = r#"
pub fn production_thing() -> u8 { 0 }

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeFetcher {
        body: String,
    }

    #[test]
    fn mock_uses_network_api_shape() {
        let _ = ureq::get("https://example.com"); // 测试假体
        let _t = std::net::TcpListener::bind("127.0.0.1:0");
        assert_eq!(production_thing(), 0);
    }
}
"#;
        let hits = scan_unguarded_network("src/synthetic_ok.rs", SYNTHETIC, EXEMPTIONS);
        assert!(
            hits.is_empty(),
            "测试模块内的网络用法被误报（会让每个 mock 都要加豁免）：{hits:?}"
        );
    }

    /// **本闸门最关键的健壮性**：测试模块**夹在中间**时，其后的生产代码必须继续被扫。
    ///
    /// spawn 闸门（`lifecycle.rs::scan_unguarded_spawns`）用的是"遇到第一个
    /// `#[cfg(test)]\nmod tests` 就截断"的写法——对"测试模块在文件末尾"成立，但
    /// 本仓库已有多个文件把测试模块放在**中间**（`plugins.rs:215`、`lifecycle.rs:899`、
    /// `resolve.rs:712`）。若照抄截断写法，这些文件的**后半段生产代码会被静默漏扫**
    /// ——一个"看起来在守、实际只守半页"的闸门比没有闸门更危险。故本闸门按条目精确
    /// 剔除，并由此用例钉死。
    #[test]
    fn production_code_after_mid_file_test_module_is_still_scanned() {
        const SYNTHETIC: &str = r#"
pub fn before() -> u8 { 0 }

#[cfg(test)]
mod tests {
    #[test]
    fn t() { let _ = ureq::get("https://example.com"); }
}

pub fn after_mid_file_tests() {
    let _ = ureq::get("https://example.com/leak");
}
"#;
        let hits = scan_unguarded_network("src/synthetic_mid.rs", SYNTHETIC, EXEMPTIONS);
        assert_eq!(
            hits.len(),
            1,
            "应只报测试模块之后的**生产**代码那一条（截断式写法会漏掉它）：{hits:?}"
        );
        assert!(
            hits[0].contains("example.com/leak"),
            "命中的应是测试模块之后的生产代码：{hits:?}"
        );
    }

    /// `#[cfg(not(test))]` 是**生产代码**，必须照扫；`#[cfg(all(test, unix))]` 不是。
    #[test]
    fn only_test_only_cfg_items_are_excluded() {
        const SYNTHETIC: &str = r#"
#[cfg(all(test, unix))]
fn test_only() { let _ = ureq::get("https://a.example"); }

#[cfg(test)]
fn also_test_only() { let _ = ureq::get("https://b.example"); }

#[cfg(not(test))]
fn production_only() { let _ = ureq::get("https://c.example"); }

#[cfg(all(not(test), unix))]
fn production_unix_only() { let _ = ureq::get("https://d.example"); }
"#;
        let hits = scan_unguarded_network("src/synthetic_cfg.rs", SYNTHETIC, EXEMPTIONS);
        assert_eq!(hits.len(), 2, "只应报两条 `not(test)` 的生产代码：{hits:?}");
        assert!(hits.iter().any(|h| h.contains("production_only")));
        assert!(hits.iter().any(|h| h.contains("production_unix_only")));
    }

    /// cfg 谓词求值器（纯函数）：特别是 `all(not(test), unix)` **不是**测试专用。
    #[test]
    fn cfg_predicate_evaluation_is_not_substring_matching() {
        for p in ["test", "all(test, unix)", "any(test)", "all(unix, test)"] {
            assert!(cfg_pred_is_test(p), "`{p}` 应判为测试专用");
        }
        for p in [
            "not(test)",
            "all(not(test), unix)",
            "unix",
            "windows",
            "feature = \"test\"",
            "all(feature = \"test\", not(test))",
        ] {
            assert!(!cfg_pred_is_test(p), "`{p}` 是生产代码，不得判为测试专用");
        }
        // 属性层：非 cfg 属性一律不动
        assert!(attr_cfg_is_test("cfg(test)"));
        assert!(attr_cfg_is_test(" cfg( all( test , unix ) ) "));
        assert!(!attr_cfg_is_test("test"));
        assert!(!attr_cfg_is_test("allow(dead_code)"));
        assert!(!attr_cfg_is_test("cfg(not(test))"));
    }

    // ---------- 行尾 ----------

    /// **CRLF 教训（2026-09-11 v1.1.1）**：Git for Windows 的 `autocrlf` 会把源码
    /// checkout 成 CRLF。以源码文本为判据的闸门必须自己归一，否则在 Windows 上行为
    /// 与 macOS 不一致（spawn 闸门曾因此在 Windows 恒红）。此用例断言：同一份源码
    /// 无论 LF 还是 CRLF，判定**逐条相同**。
    #[test]
    fn crlf_sources_are_judged_identically() {
        const LF: &str = "\
pub fn a() -> u8 { 0 }

#[cfg(test)]
mod tests {
    #[test]
    fn t() { let _ = ureq::get(\"https://x.example\"); }
}

pub fn b() { let _ = ureq::get(\"https://y.example\"); }
";
        let crlf = LF.replace('\n', "\r\n");
        let a = scan_unguarded_network("src/synthetic_lf.rs", LF, EXEMPTIONS);
        let b = scan_unguarded_network("src/synthetic_crlf.rs", &crlf, EXEMPTIONS);
        assert_eq!(a.len(), 1, "LF 下应只报测试模块之后那一条：{a:?}");
        assert_eq!(
            a.iter()
                .map(|s| s.split(':').nth(1).map(str::to_string))
                .collect::<Vec<_>>(),
            b.iter()
                .map(|s| s.split(':').nth(1).map(str::to_string))
                .collect::<Vec<_>>(),
            "CRLF 与 LF 必须报出相同行号：LF={a:?} CRLF={b:?}"
        );
    }

    // ---------- 词法：注释 / 字面量 / 花括号 ----------

    /// 注释里的原语不算命中（否则每处文档说明都要加豁免）。
    #[test]
    fn comments_are_not_scanned() {
        const SYNTHETIC: &str = r#"
//! 模块文档：这里绝不能直接 ureq::get —— 网络统一走 updates.rs
/// 同上：TcpStream 也不行
/* 块注释里提到 curl 与 wget 也不算 */
pub fn ok() -> u8 {
    let _s = "字符串里的 ureq::get 也不算（它是数据，不是调用）";
    0
}
"#;
        let hits = scan_unguarded_network("src/synthetic_comments.rs", SYNTHETIC, EXEMPTIONS);
        assert!(
            hits.is_empty(),
            "注释被当成了代码（会让文档说明触发闸门）：{hits:?}"
        );
    }

    /// 花括号出现在字符串 / 字符字面量 / 注释里时，**不得**干扰条目边界判定。
    ///
    /// 若配对器被 `"}"` 骗到，测试模块会提前"结束"，其后的测试代码被当生产代码扫
    /// → 误报；或反过来把生产代码吞掉 → 漏扫。
    #[test]
    fn braces_in_literals_and_comments_do_not_break_item_extent() {
        const SYNTHETIC: &str = r##"
pub fn before() -> u8 { 0 }

#[cfg(test)]
mod tests {
    #[test]
    fn t() {
        let _json = "{\"a\":1}";
        let _close = '}';
        let _raw = r#"{"nested":"}"}"#;
        // 注释里的花括号 } 与 { 都不算结构
        let _ = ureq::get("https://x.example");
    }
}

pub fn after() { let _ = ureq::get("https://y.example"); }
"##;
        let hits = scan_unguarded_network("src/synthetic_braces.rs", SYNTHETIC, EXEMPTIONS);
        assert_eq!(
            hits.len(),
            1,
            "字面量/注释里的花括号不得打穿条目边界：{hits:?}"
        );
        assert!(hits[0].contains("after"), "实际：{hits:?}");
    }

    // ---------- 条目级豁免的边界 ----------

    /// 条目级豁免只覆盖**那个函数**：同一文件里别处的原语仍要报。
    #[test]
    fn item_scoped_exemption_does_not_leak_to_the_rest_of_the_file() {
        const SYNTHETIC: &str = r#"
fn allowed_entry() {
    let _ = ureq::get("https://registered.example");
}

pub fn somewhere_else() {
    let _ = ureq::get("https://not-registered.example");
}
"#;
        let table = vec![NetworkEntry {
            file: "src/synthetic_item.rs",
            item: Some("allowed_entry"),
            kind: Kind::Exempt,
            reason: "测试用",
            date: "2026-09-11",
        }];
        let hits = scan_unguarded_network("src/synthetic_item.rs", SYNTHETIC, &table);
        assert_eq!(hits.len(), 1, "条目级豁免泄漏到了整文件：{hits:?}");
        assert!(
            hits[0].contains("not-registered"),
            "命中的应是豁免条目之外的那处：{hits:?}"
        );
    }

    /// 词边界：`my_ureq_client` 这类标识符不得误命中。
    #[test]
    fn word_boundaries_prevent_identifier_false_positives() {
        const SYNTHETIC: &str = r#"
pub fn f() {
    let my_ureq_client = 1;
    let reqwestish = 2;
    let _ = (my_ureq_client, reqwestish);
}
"#;
        let hits = scan_unguarded_network("src/synthetic_words.rs", SYNTHETIC, EXEMPTIONS);
        assert!(
            hits.is_empty(),
            "标识符片段被误判为网络原语（会逼出无意义豁免）：{hits:?}"
        );
    }
}
