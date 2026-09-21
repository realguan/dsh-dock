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

## 1. P0 · 客体侧 patch 写入内核（keystone：一次实现，解锁三项）

**为什么要内核**：`apply_official_patch_row` / `remove_official_patch_row` / 安全模式三者都要**同一组不变量**，
本地档由 `plugins.rs::PatchFile` 保证（AGENTS §6）：**未改条目原文保真（含行间注释）+ 覆写前 `.bak-<unix秒>` 备份 + 原子替换（同目录临时文件 + rename）**。
客体档缺的正是这个内核。

**做法**：新增 `scripts/patch-row.mjs`（客体侧执行，node 已就位），CLI 契约：

```
node patch-row.mjs apply  --home <abs> --rel <profile 相对路径> --row <base64(json)>   # 追加壳写过的挂载行
node patch-row.mjs remove --home <abs> --rel <...> --id  <dsh-dock-...>                # 按 id 删行 + 清同 id 停用桩
node patch-row.mjs disable-all --home <abs> --rel <...> --ids <base64(json[])>         # 安全模式：把指定行写成 disabled: true
node patch-row.mjs restore  --home <abs> --rel <...> --backup <abs 路径>               # 安全模式退出：用备份覆盖回去
```
stdout 恒为**单行 JSON**（`{"ok":true,...}` / `{"ok":false,"error":"..."}`），与 `WRITE_OK`/`WRITE_FAILED` 哨兵同口径（非零退出会被 `run_with_timeout_raw` 折叠成"无输出"，会丢诊断）。
**文本级保真**，不做 YAML 重新序列化（重新序列化会吃掉注释与顺序 —— 本地内核刻意避免的正是这个）。

**Rust 侧**：`guest.rs` 增 `patch_row_in_guest(distro, op, rel, payload) -> Result<String, String>`（投脚本 + 解析单行 JSON）。
**接线**：`commands/plugin.rs` 的 `apply_official_patch_row` / `remove_official_patch_row` 与 `commands/boot.rs` 的安全模式进入/退出，
把 `World::Wsl { distro } => Err(...)` 改成 `World::Wsl { distro } => patch_row_in_guest(&distro, ...)`。
**测试**：`.mjs` 本机可真跑（bundled node + 临时 fixture）：原文保真（恶意夹具：行间注释、CRLF、无尾换行）、幂等零写入、备份存在性、原子性（rename 后旧 inode 不变）；
Rust 侧纯函数测 CLI 参数拼装与 JSON 解析（正反例）；`#[cfg(unix)]` 门控实跑（同 `guest.rs` 既有纪律）。

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
