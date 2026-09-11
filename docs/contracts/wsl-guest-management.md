# WSL 客体管理面（wsl-guest-management）契约

> 本文件是 **ADR-0016** 的配套契约文档：定义「控制中心在 WSL 客体环境下的管理面」
> 的原语抽象、跨环境数据流、文件系统不变量与安全防线。
> ADR 记决策背景与架构推导，本册记**不许悄悄变的那一面**。

## 状态

版本：**v1（随 ADR-0016 落地）**｜状态：**稳定**｜
消费方：
- IPC 命令入口：`commands/profile.rs` / `commands/plugin.rs` / `commands/session.rs` / `commands/console.rs`
- 管理面领域模块：`mgmt.rs` / `guest.rs` / `profiles.rs` / `plugins.rs` / `credentials.rs` / `dsh_settings.rs` / `mcp.rs` / `diagnostics.rs` / `sessions.rs`
- 壳生命周期与执行层：`lifecycle.rs`（`Role::DshCli`）/ `executor.rs`（`WslExecutor`）

---

## 0. 为什么需要这份契约

历史背景（2026-09-11，ADR-0016）：
当 dsh 运行在 WSL 客体内时，控制中心此前误用宿主文件系统与宿主 engines 目录，导致
市场插件安装、Profile 管理、会话维护与控制台在 WSL 模式下结构性不可用。

WSL 客体管理面的核心原则是：
1. **世界一致性**：管理面读写的对象，必须严格对齐当前活跃的运行时世界（`Local` 还是 `Wsl { distro }`）；
2. **纯函数单实现**：宿主与客体的文件内容解析与数据装配逻辑 100% 共享纯函数，严禁双份实现；
3. **文件系统不变量**：客体侧的文件写入、权限保障、原子替换、前置备份必须与宿主完全等价，绝不放宽。

---

## 1. 接口签名

### 1.1 世界判定 Seam（`src-tauri/src/mgmt.rs`）

```rust
/// 运行环境世界：决定管理面命令分发至宿主还是客体。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum World {
    Local,
    Wsl { distro: String },
}

/// 解析当前活跃的世界。模式为 WSL 时必须持有探测确定的 active_wsl_distro，
/// 绝不静默回退 Local。
pub fn current_world(app: &AppHandle) -> Result<World, String>;

/// 纯内核判定：供测试与无 AppHandle 上下文消费。
pub fn world_from(mode: Mode, active_distro: Option<String>, is_windows: bool) -> Result<World, String>;
```

### 1.2 客体操作原语（`src-tauri/src/guest.rs`）

全部客体原语仅在 `#[cfg(windows)]` 下编译生效，非 Windows 平台提供对等的未实现报错桩函数。
原语执行均受 `lifecycle::Role::DshCli` 子进程生命周期守卫管控。

```rust
/// 批量读取客体文件内容（单次 wsl.exe 往返，base64 帧流传输）。
pub fn read_files(distro: &str, rel_paths: &[String]) -> Result<Vec<(String, Option<String>)>, String>;

/// 批量原子写入客体 dsh 目录文件（base64 解码 → 同目录临时文件 → mv 原子替换）。
/// credentials 文件自动应用 0600 权限。单文件写上限 8 KiB。
pub fn write_home_files(distro: &str, entries: &[(String, String)]) -> Result<(), String>;

/// 列举客体 dsh 子目录条目（含点文件，返回名称与目录标志）。
pub fn list_dir(distro: &str, rel_dir: &str) -> Result<Vec<GuestDirEntry>, String>;

/// 客体文件覆盖前备份：生成形如 <path>.bak-<timestamp> 的副本。
pub fn backup_file(distro: &str, rel_path: &str) -> Result<Option<String>, String>;

/// Profile 目录复制：排除 node_modules，原子迁移配置与依赖声明。
pub fn copy_profile_dir(distro: &str, from_name: &str, to_name: &str) -> Result<(), String>;

/// Profile 目录重命名：排除 node_modules，目录迁移后清理源目录。
pub fn rename_profile_dir(distro: &str, from_name: &str, to_name: &str) -> Result<(), String>;

/// Profile 目录删除：仅删除 profiles/<name>，严禁级联删除会话目录。
pub fn delete_profile_dir(distro: &str, name: &str) -> Result<(), String>;

/// 会话文件原始扫描：提取路径、字节大小与更新时间戳。
pub fn scan_sessions_raw_in_guest(distro: &str) -> Result<Vec<RawSessionFile>, String>;

/// 会话删除：严格校验安全根路径，防路径穿越与意外全删。
pub fn delete_session_in_guest(distro: &str, session_id: &str) -> Result<(), String>;

/// 会话修复：通过 stdin 管道投递 94 KiB 的 repair-session.mjs 规避 Windows 命令行超长截断。
pub fn run_repair_in_guest(distro: &str, session_id: Option<&str>, all: bool) -> Result<RepairSummary, String>;

/// 系统诊断收集：跨环境收集发行版信息、架构、node/pnpm/dsh 版本与磁盘容量。
pub fn collect_diagnostics_in_guest(distro: &str) -> Result<GuestDiagnostics, String>;

/// 执行 dsh CLI 脚本并将标准输出重定向到日志文件（安装/卸载/更新插件等）。
pub fn run_script_to_log(distro: &str, script: &str, log_prefix: &str) -> Result<(), String>;
```

---

## 2. 数据模型

### 2.1 目录项与扫描模型

| 结构体 | 字段 | 类型 | 含义与约束 |
|:---|:---|:---|:---|
| `GuestDirEntry` | `name` | `String` | 相对目录下的文件名或子目录名（含隐藏文件） |
| | `is_dir` | `bool` | 是否为目录 |
| `RawSessionFile` | `rel_path` | `String` | 相对于 `sessions/` 的相对路径 |
| | `size_bytes` | `u64` | 文件大小（字节） |
| | `updated_at` | `u64` | 最后修改时间戳（毫秒） |

### 2.2 诊断与修复报告模型

| 结构体 | 字段 | 含义 |
|:---|:---|:---|
| `GuestDiagnostics` | `distro`, `kernel`, `os_name`, `arch` | 客体 Linux 系统特征 |
| | `node_version`, `pnpm_version`, `dsh_version` | 客体运行环境核心依赖版本 |
| | `total_space`, `free_space` | 客体根分区存储容量指标 |
| `RepairSummary` | `total`, `scanned`, `repaired`, `failed` | 修复脚本汇总统计结果 |

---

## 3. 行为承诺与文件系统不变量

### 3.1 路径真相源

- **客体绝对路径一律锚定 `${DSH_HOME:-$HOME/.dsh}`**（`guest::HOME_EXPR`）；
- 严禁假定客体用户名固定为某值（如 `/home/user`）；
- 严禁将 Windows 宿主路径通过字符串拼接后传给客体解析。

### 3.2 纯函数单实现（零双源）

跨环境共用的解析与装配逻辑必须抽离为宿主与客体 100% 共享的纯函数：
- `assemble_profile_summaries` / `assemble_profile_detail`（Profile 读侧）；
- `rewrite_manifest_name_text` / `scan_patch_relative_path_warnings`（Profile 写侧）；
- `assemble_plugin_entries` / `dependency_names` / `apply_disabled_toggle` / `apply_copy_config_entries`（插件管理）；
- `parse_credentials_summary` / `apply_set_provider_key`（凭据安全）；
- `parse_mcp_servers` / `apply_save_mcp_server` / `apply_delete_mcp_server`（MCP 配置）；
- `assemble_session_items` / `parse_archived_session_ids`（会话维护）。

### 3.3 凭据安全不变量

- 客体 `.credentials.yaml` 写入前必须执行 `chmod 600`，保持 POSIX 权限安全红线；
- 顶层严格限定 `provider`, `deepseek`, `token` 三键，密码值在 IPC 读侧及前端彻底脱敏；
- 写入前自动生成 `.bak-<timestamp>` 副本。

### 3.4 目录排除与符号链接保护

- `copy_profile` 与 `rename_profile` 在客体执行时，**必须显式排除 `node_modules`**（`--exclude "node_modules"`）；
- 严禁直接递归覆盖或深拷贝符号链接农场，防止引起文件系统死锁或存储爆炸。

### 3.5 会话保护与路径逃逸防线

- 会话删除原语必须严格校验目标路径绝对位于 `${DSH_HOME:-$HOME/.dsh}/sessions/` 内部；
- 包含 `..`、`/` 根目录或空会话 ID 的输入一律直接拒绝，防止误删整个用户主目录；
- 删除 Profile 时严格禁止级联删除会话目录（AGENTS §6 既定红线）。

### 3.6 命令行防溢出纪律

- Windows `CreateProcess` 存在 **32,767 字符**硬限制；
- 超过 4 KiB 的大型脚本（如 94 KiB 的 `repair-session.mjs`）**严禁内联至 `wsl.exe -e bash -c '...'`**；
- 必须通过子进程 **stdin 管道**流式投递并由客体接收执行。

---

## 4. 禁止外部访问的内部实现

1. **私有 bash 脚本模板**：`read_files_script`, `write_home_files_script`, `copy_profile_script`, `delete_session_script` 等为 `guest.rs` 内部生成函数，禁止外部模块直接拼装裸 bash 字符串；
2. **Base64 单行帧协议**：跨环境 IPC 的数据封包格式属于传输层细节，业务层只消费解码后的结构化类型；
3. **前端透明化**：前端 UI/API 完全不感知 `World` 枚举与平台分发细节，所有分发逻辑在 Rust IPC 命令层完成。

---

## 5. 演进规则

1. **新环境档扩展**：若未来引入 SSH、Docker 容器或远程环境，必须扩展 `World` 枚举并在本契约内新增原语规范，严禁在 `Wsl` 分支下夹带无关实现；
2. **上游 API 演进**：若 dsh 上游原生提供插件与 Profile 管理 RPC/API，本契约对应原语应有序降级为兼容备选或退役，改走上游官方 API。
