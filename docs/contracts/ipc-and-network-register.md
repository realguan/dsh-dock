# IPC 命令与网络面登记册

> **本册是 `AGENTS.md` §7「例外册（登记制）」的**登记台账**。**
> **规则留在 §7，登记册在本册**——迁出理由同 §9 的 ADR 索引：这两张册子随功能
> **线性增长**，而 §11.4 给 `AGENTS.md` 设的是**固定**行数预算。2026-09-15 迁出时
> `AGENTS.md` 已到 **249/250**，**再加一条登记就会越界**，与 ADR-0019 当日"存在却没进
> 索引"同源。按 §11.4 的「回收」机制迁出。
>
> **维护约定**：新增 IPC 命令或新增网络用途时，**改本册**（§一 / §二），
> `AGENTS.md` §7 只保留规则与指针。两处**不得各写一份**（AGENTS §11.3 禁双源）。
> 命令清单的**机器事实源**仍是 `src-tauri/src/ipc.rs::COMMANDS`——本册是**人类可读**的
> 登记册（含落地日期与边界说明），二者由 `ipc.rs::gate_tests` 与代码审查对齐。

---

## 一、IPC 命令登记（58 条）

> 新命令**先登记再实现**。清单一致性另有 cargo test 闸门：
> `handler_matches_ipc_commands` / `capabilities_match_ipc_commands` /
> `tauri_ts_matches_ipc_commands` / `ipc_struct_shapes_match_fixture`。

**核心与工作台**：`choose_profile` `terminal_action` `get_update_status` `check_updates`
`list_dsh_versions`（packument 全版本 + 通道归类，2026-09-09）`get_client_update`
`client_update_check` `client_update_apply` `open_external` `open_workbench_in_browser`
`get_workbench_url` `boot_in_wsl` `choose_mode` `get_boot_status`（启动态缓存，防竞态，2026-09-04）。

**Profile**：`list_profiles` `get_profile_detail` `create_profile` `copy_profile`
`rename_profile` `delete_profile` `set_default_profile` `get_default_profile`
`switch_profile` `get_active_profile` `open_profiles_window` `focus_main_window` `open_about` `get_shell_capabilities`（2026-09-21 立，§3.6：无托盘宿主的 Linux 桌面上，「关于 / 更新」原本**没有任何入口** —— 原 `open_about` 于 2026-08-27 因「与常驻入口重复」删除，而该前提在此场景失效。前端**只在 `residentEntryAvailable === false` 时**渲染入口；两命令均只读进程内状态 / 开壳自有窗口，不碰文件与网络）。

**插件**：`list_profile_plugins` `get_plugin_runtime` `install_plugin` `remove_plugin`
`update_plugin` `get_plugin_rows` `get_safe_mode_state`（2026-09-16 立，ADR-0026 第三版口径：安全模式的
只读状态——是否仍处于安全模式 + **此刻**仍停用着的行 id（记账里的集合 ∩ 当前配置的停用桩，
故用户逐个打开开关后数字随之下降，全开即 `active=false`）。**只读壳自有记账**
（`<app_data>/safe-mode/<profile>.json`）＋profile 自家 patch，**不读运行态**：运行态另由回环
快照给，两源禁混。安全模式本身改的是 profile 的 `cordis.patch.yml`（走 `PatchFile` 既有写入
纪律），故配置层即真相源；**命令不再提供任何恢复动作**——恢复已按维护者裁定移除）
`dismiss_safe_mode_notice`（2026-09-16 立，维护者裁定：安全模式横幅只在"确实以安全模式进入过"时
出现，且**必须可关闭**——用户看过一次就够；它只改壳自有记账里的 `dismissed_at`（**不碰 dsh 配置**），
同一轮再启动不打扰，**下一次进入安全模式会重新提示**；不是本 home 的记账一律不动）
`set_plugin_disabled` `check_plugin_updates`
`list_plugin_versions` `list_all_plugins` `list_experimental_capabilities`（2026-09-15 立、
2026-09-16 由 `list_official_plugins` 改名并改形，ADR-0020 §7：返回**能力 → 变体 → 步骤**
三级事实视图，状态含 `off/on/disabled/partial/conflict`，由后端按「包 × 挂载行 × disabled」
一次算全；另附 `toggleOffSupported` / `displaced`；**已下沉客体档**（2026-09-21，P0/P2：同一内核 + 客体原语），不回落本地读）
`apply_official_patch_row`（2026-09-15 立、2026-09-16 §7.4 改为**写前当场重判**：包已装后
重读其 `package.json` 判定是否声明 `dsh.bundle`，声明者**拒绝写行**并回报
`autoActivated`（安装前判定会给 profile 层多写一条 = 重复挂载）；幂等、经 `PatchFile`、
写后回读组合树自证；**已下沉客体档**（2026-09-21，P0/P2：同一内核 + 客体原语））`remove_official_patch_row`（2026-09-16，
ADR-0020 §7.2-3：**反向原语**——按 `id` 删除壳写过的挂载行并清同 id 的停用桩，
只接受 `dsh-dock-` 前缀（bundle 自带行与用户手写行不得代删），删除后回读自证该行已不在
组合树；幂等；**已下沉客体档**（2026-09-21，P0/P2：同一内核 + 客体原语））`copy_plugin_config`（patch 行原样复制，
写入例外 #4，ADR-0009 五修 2026-08-30）。

**控制台 / 凭据 / 设置 / MCP**（2026-08-31 批）：`get_shell_settings` `set_shell_settings`
`get_system_diagnostics` `get_app_logs` `get_credentials_raw` `save_credentials_raw`
`get_credentials_summary` `set_credential_key` `get_dsh_settings_raw`
`save_dsh_settings_raw` `list_mcp_servers` `save_mcp_server` `delete_mcp_server`
`probe_mcp_server`（2026-09-15，ADR-0022：探测 MCP 能力；**stdio 分支**经 `lifecycle`
seam 起子进程，http 分支为条目级网络豁免；**stdio 分支已下沉客体档**（2026-09-21，P2：宿主生成有序对话、客体搬运器执行、宿主同一套纯函数解析），**streamable-http 在客体档显式拒绝**并说明理由（url 指客体内部地址，从宿主发是错结论；非平台收窄，是语义边界））。

> **2026-09-18 生效范围建模（命令条数不变；参数与语义变更）**：
> - `list_mcp_servers` 改为读**两层**（`profiles/<名>/cordis.patch.yml` +
>   `$DSH_HOME/cordis.patch.yml`），每条带 `scope: "profile" | "global"`；
> - `save_mcp_server` 按 `server.scope` 选层写入（缺省 profile 层）；
> - `delete_mcp_server` / `probe_mcp_server` 各增 `scope` 入参（可选，缺省 profile）
>   —— **同名条目可同时存在于两层**，不带层会删错、探错那一条；
> - `McpServerConfig` 增 `cwd`（stdio 子进程工作目录）与 `scope` 两个字段。
>
> 依据：`dsh-app-boot/lib/index.js:1005 readProfilePatches` 实查（bundle 层 → profile 层 →
> home 级全局层 → `--patch` overlays）。**未新增命令，故 62 条不变**。
>
> **2026-09-18 三修（仍是那四条命令，参数与口径收紧；详见 ADR-0027 §7）**：
> - `delete_mcp_server` / `probe_mcp_server` 各增 `row_id` 入参（可选）。**空 ⇒ 按
>   `(name, scope)` 首条命中**（兼容无 `id` 的手写行）；**给了却没命中 ⇒ 直接报错**
>   （"请刷新列表后重试"），绝不回退按名字操作另一行——同层重名时那是可达路径；
> - `save_mcp_server` 同样按 `row_id` 定位被编辑的那一行；`row_id` 失效即报错，
>   不再"名字对不上就 append 一行"（会凭空多出一条）；
> - `McpServerConfig` 增 `rowId`（patch 行 id，装配状态按它匹配）与 `expr`
>   （原文含 `!!js` 标签值）。**`expr` 行的结构化保存被后端拒绝**：serde_yaml 会把
>   标签展平成字面量，写回去就毁掉凭据来源；只有"整行原样带回、仅 `disabled` 变化"
>   的启停走逐字节文本改写通道；
> - `probe_mcp_server` 在 `expr` 行上**不探测**（壳复现不了 dsh 的 `!!js` 求值 ⇒
>   要么假失败要么带错凭据假通过），引擎未就绪时不回落当前进程 PATH（同理）。

**市场**：`fetch_market_registry`（2026-08-31）。

**当前条数 = 62**（`ipc.rs::COMMANDS` 为唯一事实源，`ipc::gate_tests` 四处比对；
2026-09-17 净减 3：SSH 远程工作区整体移除，`list_ssh_hosts` / `probe_ssh_target` /
`generate_ssh_profile` 三条命令与对应权限一并退役——**未随任何版本发布**（见 ADR-0023 状态）；
2026-09-16 净增 3：`list_official_plugins` → `list_experimental_capabilities` 属改名，
新增 `remove_official_patch_row`、`get_safe_mode_state` 与 `dismiss_safe_mode_notice`）。

---

## 二、网络面用途登记

> **规则（在 `AGENTS.md` §7）**：唯一网络面 = `updates.rs`（ADR-0006）；其余模块禁触网，
> **新网络需求先在此登记**；外链域名在 `EXTERNAL_URL_HOSTS` 登记。机器投影在
> `src-tauri/src/network_gate.rs` 的 `EXEMPTIONS` 表（`Kind::Exempt` = 授权触网，
> `Kind::Registered` = 仅在册）；两处**必须同步**。

### 回环（127.0.0.1，无外部网络）

**回环调用必须附 `/api` 会话 Cookie**：dsh 0.1.6+ 在 Host 栅栏之外还有
`browserAuth`（缺失恒 401）；Cookie 由 boot 期 launch token 兑换后留在
`ShellState.workbench_cookie`（**仅内存、不打日志**；2026-09-15 实测更正，台账复现点 11）。

| 用途 | 落点 | 边界 |
|:---|:---|:---|
| 插件运行态回环**只读**查询 | `plugins.rs`，`POST http://127.0.0.1:<port>/api/pluginInventory/list` | 2s 超时、仅活跃会话、一次性快照不订阅——2026-08-29 |
| 工作台 Token 环回兑换 | `boot.rs::authenticate_workbench_session`，本地 `127.0.0.1` GET | 5s、redirects=0；2026-09-04 落地、2026-09-11 补登记——原漏登 |

### 子进程内触网（`Kind::Registered`，本文件无 in-process 原语）

| 用途 | 落点 | 边界 |
|:---|:---|:---|
| MCP 能力探测（**stdio 分支**） | `mcp_probe.rs`，2026-09-15，ADR-0022 | 网络在**被 spawn 的 MCP 服务器子进程内**；`network_gate` 的 `Registered` 行**反向绑定**此点。2026-09-15（R3）实现 `streamable-http` 分支后**该行保留**——含义收窄为"**除下节 `post_rpc` 条目级豁免覆盖的范围外**，本文件不得再有进程内原语"；两条并存才封住"第二处触网" |
| 引擎引导（ADR-0010） | `engines.rs` | 网络在 pnpm 子进程内：`runtime set node` 下载 node、经 `pnpm add`（**project 内安装，非 `-g`**：Windows 免符号链接特权，ADR-0017）下载 dsh（镜像 env 注入）；WSL 客体仍同源 `add -g` |
| WSL 客体投递与管理面（ADR-0016） | `executor.rs`；客体插件装卸/更新在客体 `dsh plugin`（客体 pnpm）子进程内 | 更新检查仍走 `updates.rs`，**壳不新增网络客户端** |
| 插件装卸/更新的**源选择**（ADR-0006 §6，2026-09-16） | `commands/plugin.rs::install_plugin` → `dsh plugin add … --registry <url>`（宿主；客体暂用其自身配置） | 网络仍在 **pnpm 子进程内**；壳只按次传 `--registry`，**不改写用户 npm 配置**、无 in-process 客户端；machine projection 不变 |

### 进程内触网（条目级豁免；`Kind::Exempt` + `item`）

| 用途 | 落点 | 边界 |
|:---|:---|:---|
| MCP 能力探测（**streamable-http 分支**） | `mcp_probe.rs::post_rpc`，2026-09-15，ADR-0022 §3.3 方案 A | POST **用户在 `cordis.patch.yml` 自填的 `url`**；整轮 30s（2026-09-18 三修：原 15s 会把"`pnpm dlx` 首跑正在装依赖"误判成失败；5 次往返共享一个 deadline）、`redirects=0`、只读一次性快照。**条目级而非整文件**：将来新增触网点（如 `resources/read` 预览）仍须单独登记（ADR-0022 §3.3 方案 C 否决）。**本探测扩大壳的信任边界**——用户配置的 URL 会被壳真实访问；ADR-0022 **不解决 SSRF 类风险**，仅以"用户自配端点 + 只读 + 有界超时"限制影响面（如需策略另开工） |

### 直接触网（`updates.rs` / `updater.rs`，唯一网络面本体）

| 用途 | 落点 | 边界 |
|:---|:---|:---|
| 唯一网络面本体（ADR-0006） | `updates.rs` | `ureq` 客户端的唯一出口（`UreqFetcher`） |
| 插件更新检查 / 市场 Registry 拉取 | `updates.rs` `npm_packument_versions` / `fetch_market_registry` | 镜像链与超时同 dsh 版本检查 |
| 客户端自更新 | `updates.rs::APP_RELEASE_FEED` + `updater.rs` | 清单端点在 `tauri.conf` 的 `plugins.updater.endpoints` |

---

## 三、壳的**文件系统读取域**登记（新域先登记，与 §二 网络面同口径）

> 规则：壳只在 `$DSH_HOME` / `app_data_dir()` 内读写是默认口径；**走出这两个根**即属
> 新读取域，须在此登记**读什么、读多少、不外传什么**。读取面比网络面更难察觉——它没有
> "触网"这种显眼动作，扩展起来往往是顺手加一句 `read_to_string`，且**没有机器闸门**
> （`network_gate.rs` 只拦网络原语），所以本节的登记纪律只能靠人守。

| 域 | 落点 | 边界（读什么 / 不外传什么） |
|:---|:---|:---|
（当前为空：2026-09-15 登记过的「用户 SSH 配置 `~/.ssh/config`」随 SSH 远程工作区整体
移除于 2026-09-17 退役——该功能未随任何版本发布，这段登记史保留在 ADR-0023 的「状态」里。）
