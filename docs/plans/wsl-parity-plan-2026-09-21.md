# WSL 客体档能力对齐 · 施工方案（2026-09-21）

> **为什么必须有这份方案**：`AGENTS.md` §0 红线 3 把 **`Windows（x64，含 WSL2 客体模式）`** 写成适配目标。
> 安全模式 / MCP 能力探测 / 官方插件挂载行增删 / 实验能力目录**四项核心能力**在客体档缺失，
> 现状是**欠债**（原报错文案自陈「需补客体侧 X 原语**后方可启用**」），**不是**平台限制 ——
> 此前把它写进发布日志「已知限制」属擅自收窄（维护者 2026-09-21 指出，已更正+落档）。
> **目标**：客体档与本地档**能力等价**；在补齐之前，各入口保持**显式报错 + 替代路径**（绝不静默回落宿主）。

## 0. 现状与既有原语（不是从零开始）

| 面 | 本地档 | 客体档现状 | 结论 |
|:--|:--|:--|:--|
| profile 全生命周期（列/详/建/拷/改/删/默认） | ✓ | ✓ **已下沉**（`commands/profile.rs` 逐命令 match world） | 参照样板 |
| 插件装卸/更新（`dsh plugin` 转发） | ✓ | ✓ 已下沉（客体 pnpm/子进程内） | — |
| 凭据 / dsh 设置 / MCP 配置读写 | ✓ | ✓ 已下沉（`credentials.rs` / `dsh_settings.rs` / `mcp.rs` 用 `guest::read_files`+`write_home_files`+`backup_file`） | **写路径已有先例** |
| **官方插件挂载行增删**（`apply_/remove_official_patch_row`） | ✓ | ❌ 显式拒绝 | 本方案 P0 |
| **安全模式**（进入/退出/横幅读写） | ✓ | ❌ 写侧显式拒绝；读侧**已修**为显式拒绝（`safe_mode_home`，2026-09-21） | 本方案 P0 |
| **实验能力目录**（`list_experimental_capabilities`） | ✓ | ❌ 显式拒绝 | 本方案 P1（只读） |
| **MCP 能力探测**（`probe_mcp_server`） | ✓ | ❌ 显式拒绝（跨子系统 stdio） | 本方案 P2 |

**可直接复用的客体原语**（`src-tauri/src/guest.rs`）：`read_files` · `write_home_files` · `backup_file` · `list_dir` · `dsh_cli_script`（经 `executor::run_wsl_capture` 跑 `wsl.exe -e bash -lic`，stdin 走管道规避 32K 命令行上限）。
**既有"往客体投脚本"的成熟套路**：`guest.rs:883` 用 `include_str!("../../scripts/repair-session.mjs")` 把仓库内的 `.mjs` 投进客体执行 —— 客体**必有 node**（引擎引导链保证）。本方案的新写入内核与探测脚本**都走这条路**，不新增任何网络面（红线：唯一网络面 = `updates.rs`）。

## 1. P0 · 客体侧 patch 写入（keystone：一次接线，解锁三项）

> **方案修正（2026-09-21 开工第一天，落码前）**：本节原方案是"新写 `scripts/patch-row.mjs` 在内核侧复刻
> `PatchFile` 的三不变量"。**这是错的** —— 那会造出**第二个内核**，直接违 AGENTS §6 的
> 「宿主/客体**同一内核**：未改条目原文保真含行间注释 + 覆写前备份 + 原子替换」，且两侧必然漂移。
> 开工核查发现两条事实，使正确方案更小：
> 1. **内核本就是文本进文本出**：`safe_mode.rs:246` 已在用 `crate::plugins::PatchFile::from_text(&text)`
>    （`plugins.rs:1787` 的 `read()` 也只是读文件后转调 `from_text`）⇒ **同一个 `PatchFile` 就能服务客体**：
>    `guest::read_files` 拿到原文 → `PatchFile::from_text` → 变更 → `render_checked()` → 写回客体。
> 2. **客体写已经是原子的**：`guest::write_home_files_script`（`guest.rs:252-257`）是
>    `base64 -d > <path>.dsh-dock.tmp && … && mv -f … <path>`（tmp + rename，并对 `credentials.y*` 补 chmod 600）
>    ⇒ 「原子替换」不变量**既有原语已满足**，配合 `guest::backup_file` 即凑齐三不变量。
>
> ⇒ **P0 的正确形态**：**零新内核**，只加一层 ~20 行的客体包装 + 三处接线。

**接线形状**（每个操作一份，形状相同）：

```rust
// 宿主档（既有，不动）：ensure_catalog_insert_row(&home, &profile, id, pkg, cfg)
// 客体档（新增）：同内核 + 既有原语
fn ensure_catalog_insert_row_in_guest(distro, profile, id, pkg, cfg) -> Result<bool, String> {
    let rel = format!("profiles/{profile}/cordis.patch.yml");
    let text = guest::read_files(distro, &[rel.clone()])?          // 读客体原文（缺失 → "[]\n"）
        .into_iter().next().and_then(|(_, c)| c).unwrap_or_else(|| "[]\n".into());
    let mut patch = PatchFile::from_text(&text)?;                   // ★ 同一个内核
    let changed = patch.<变更方法>(id, pkg, cfg)?;                  // 幂等：未改即返回 false，零写入
    if !changed { return Ok(false); }
    let next = patch.render_checked()?;                             // 渲染自检（顶层数组契约）
    guest::backup_file(distro, &rel)?;                              // 不变量②：覆写前备份
    guest::write_home_files(distro, &[(rel, next)])?;               // 不变量③：原子替换（既有 tmp+mv）
    Ok(true)
}
```

**待确认一步（下一条命令）**：`PatchFile` 的**变更方法名与签名**（`ensure_insert_row` / `remove_row` / 停用桩写入分别叫什么、
是否已带幂等返回值）—— 本轮仅确认了 `read` / `from_text` / `write` / `write_with_backup` / `render_checked`
（`plugins.rs:1784-1939`），未读完 `impl` 全表即停手，**不凭猜写码**。
**写入点三处**：`commands/plugin.rs` 的 `apply_official_patch_row` / `remove_official_patch_row` 与
`safe_mode` 的进入/退出（后者 `safe_mode.rs:246` 已在用同内核，客体档只需换成"读客体→渲回客体"）。
**测试**：`PatchFile` 的既有单测**直接复用**（同一内核，无需重写）；新增客体包装的路径/参数纯函数测试；
客体实跑仍归 Windows 真机（道 B 阻塞项）。

## 2. P1 · 实验能力目录（只读，风险最低）

`list_experimental_capabilities` = 读客体 profile 的插件清单 + 各包 `package.json` + patch 行，再经**既有纯逻辑**分类（`plugins.rs` 的三级事实视图）。
客体侧只需一次 `guest::read_files(distro, [profiles/<名>/package.json, profiles/<名>/cordis.patch.yml, …每包 package.json])`（一次往返，路径一次算全——同 `mcp.rs:1294` 的批量读法），分类代码原样复用。
**顺序建议放 P1 而非 P0**：只读、无写入风险，且能立刻验证"客体读 → 既有分类"这条路；但**用户价值低于 P0**（P0 解锁三入口）。

## 3. P2 · MCP 能力探测（跨子系统 stdio）

**难点**：探测要在**客体内部**起服务器进程并走 stdio JSON-RPC（`initialize` / `tools/list` / `resources/list`），
本地档由 `mcp_probe.rs` 直接持有子进程管道；跨 `wsl.exe` 携带长连接 stdio 语义不等价（缓冲实测见 `executor.rs:607`：90s 不 flush）。
**做法**：新增 `scripts/mcp-probe.mjs`（客体侧跑），把协议对话**整体放进客体**，只把**结果 JSON**（工具/资源/模板计数、协议版本、耗时、失败原因）经 stdout 带回 —— 与 P0 同套路。
**保持**：有界超时（既有 `mcp_probe` 的时限口径）、失败原样透出（不回落本地、不假装成功）、`--no-open` 探测缓存语义不变。
**残余差异须显式记录**：客体侧 `stdio` 服务器的 `cwd` / 环境（PATH 指向客体引擎）与本地档不同，UI 要如实说明探测发生在哪个世界。

## 4. 顺序、验收与诚实边界

| 阶段 | 内容 | 验收 |
|:--|:--|:--|
| **P0** | 客体 patch 内核 `.mjs` + Rust 接线 | 三入口在客体档**可用**（不再是 Err）；`.mjs` 本机实跑闸门（保真/幂等/备份/原子）；负例：故意重排 YAML 会被闸门抓到 |
| **P1** | 实验能力目录客体读 | 客体档返回与本地档**同形状**的三级视图；一次 `wsl.exe` 往返 |
| **P2** | MCP 探测客体内执行 | 真实服务器在客体档探出工具/资源；超时有界；失败原因原样透出 |
| **收尾** | 删除 `commands/*.rs` 中四处 `World::Wsl => Err(...)` 分支与 `wsl_safe_mode_unsupported()`；`docs/contracts/ipc-and-network-register.md` 五处「WSL 客体档显式报错」改为「已下沉」；发布日志「补齐项」相应收敛 | grep 归零 + 文档一致 |

**诚实边界（必须与进度同档）**：① **真机验证需要 Windows + WSL2**（`docs/executor.md` 现为搁置）—— 本机只能做到"`.mjs` 实跑 + Rust 纯逻辑 + CI 三平台编译测试"，**不足以宣称"客体档已适配"**；② 因此 P0–P2 每步落地后，状态写「已实现（待 Windows 真机验收）」，不写「已适配」。
