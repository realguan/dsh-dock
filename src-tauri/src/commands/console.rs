//! commands/console.rs —— console 域 IPC 命令处理层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 本层只做「参数校验 + 数据目录定位 + 阻塞动作下沉 spawn_blocking」，
//! 业务实现全在对应域模块（profiles/plugins/sessions/settings/…）。
//! 命令清单的唯一事实源仍是 `src/ipc.rs::COMMANDS`，三处同步由 cargo test 闸门拦。
//!
//! **世界择源（ADR-0016 §4 P3）**：凭据 / DSH 设置 / MCP / 系统诊断
//! 均根据当前世界择源：本地模式操作宿主 home，WSL 模式下沉至客体读写与探测。
//! 例外：`get_shell_settings`/`set_shell_settings`（壳自身设置，存 app_data）与
//! `get_app_logs`（源主要是壳自己的日志）与运行世界无关，不设闸。

use tauri::Manager;

/// 偏好与设置：读取壳设置（locale、auto_restart、default_mode 等）
#[tauri::command]
pub fn get_shell_settings(app: tauri::AppHandle) -> Result<crate::settings::ShellSettings, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(crate::settings::load(&data_dir))
}
/// 偏好与设置：保存壳设置并跨窗口广播更新
#[tauri::command]
pub fn set_shell_settings(
    app: tauri::AppHandle,
    settings: crate::settings::ShellSettings,
) -> Result<(), String> {
    use tauri::Emitter;
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    crate::settings::save(&data_dir, &settings)?;
    let _ = app.emit("app:settings-changed", &settings);
    Ok(())
}
/// 环境健康体检：全量诊断大盘数据采集（4.11）
#[tauri::command]
pub async fn get_system_diagnostics(
    app: tauri::AppHandle,
) -> Result<crate::diagnostics::SystemDiagnosticsReport, String> {
    let world = crate::mgmt::current_world(&app)?;
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            Ok(crate::diagnostics::collect_diagnostics(&home, &data_dir))
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::diagnostics::collect_diagnostics_in_guest(&distro)
        }
    })
    .await
    .map_err(|e| format!("诊断报告收集异常终止：{e}"))?
}
/// 运行日志：安全读取指定源的尾部日志（4.11）
#[tauri::command]
pub async fn get_app_logs(
    app: tauri::AppHandle,
    source: String,
    tail_lines: Option<usize>,
) -> Result<crate::diagnostics::LogQueryResult, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    // 2026-09-21 平台审计 A5：本命令此前是 console.rs 里**唯一**没有世界分发的命令
    // —— WSL 客体档下会静默读宿主世界的日志（给错结果比报错更坏）。日志源随世界
    // 解析：`dsh` 在宿主档是 dsh-shell.log、客体档是 dsh-wsl.log；profile 级日志
    // 在客体档改读**客体** home（`diagnostics::resolve_log_target` 真值表）。
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let home = crate::resolve::user_dsh_home();
        let log_world = match &world {
            crate::mgmt::World::Local => crate::diagnostics::LogWorld::Local,
            crate::mgmt::World::Wsl { distro } => crate::diagnostics::LogWorld::Guest { distro },
        };
        crate::diagnostics::read_app_logs(
            &source,
            log_world,
            &data_dir,
            &home,
            tail_lines.unwrap_or(500),
        )
    })
    .await
    .map_err(|e| format!("读取日志任务异常终止：{e}"))?
}
/// 凭据管理：读取 .credentials.yaml 原文（4.5）
#[tauri::command]
pub async fn get_credentials_raw(app: tauri::AppHandle) -> Result<String, String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::credentials::read_credentials(&home)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::credentials::get_credentials_raw_in_guest(&distro)
        }
    })
    .await
    .map_err(|e| format!("读取凭据任务异常终止：{e}"))?
}
/// 凭据管理：保存 .credentials.yaml 原文（4.5，严格 0600 权限与原子写）
#[tauri::command]
pub async fn save_credentials_raw(app: tauri::AppHandle, content: String) -> Result<(), String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::credentials::overwrite_credentials(&home, &content)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::credentials::save_credentials_raw_in_guest(&distro, &content)
        }
    })
    .await
    .map_err(|e| format!("保存凭据任务异常终止：{e}"))?
}
/// 凭据管理：获取脱敏后的凭据摘要列表（4.5）
#[tauri::command]
pub async fn get_credentials_summary(
    app: tauri::AppHandle,
) -> Result<Vec<crate::credentials::CredentialSummaryItem>, String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::credentials::get_credentials_summary(&home)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::credentials::get_credentials_summary_in_guest(&distro)
        }
    })
    .await
    .map_err(|e| format!("读取凭据摘要任务异常终止：{e}"))?
}
/// 凭据管理：针对指定 Provider 设置 API Key（4.5）
#[tauri::command]
pub async fn set_credential_key(
    app: tauri::AppHandle,
    provider: String,
    key: String,
) -> Result<(), String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::credentials::set_provider_key(&home, &provider, &key)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::credentials::set_provider_key_in_guest(&distro, &provider, &key)
        }
    })
    .await
    .map_err(|e| format!("保存 Provider 凭据任务异常终止：{e}"))?
}
/// DSH 引擎设置：读取 settings.yaml 原文（4.5）
#[tauri::command]
pub async fn get_dsh_settings_raw(app: tauri::AppHandle) -> Result<String, String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::dsh_settings::read_dsh_settings(&home)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::dsh_settings::read_dsh_settings_in_guest(&distro)
        }
    })
    .await
    .map_err(|e| format!("读取 DSH 设置任务异常终止：{e}"))?
}
/// DSH 引擎设置：保存 settings.yaml 原文（4.5）
#[tauri::command]
pub async fn save_dsh_settings_raw(app: tauri::AppHandle, content: String) -> Result<(), String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::dsh_settings::overwrite_dsh_settings(&home, &content)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::dsh_settings::overwrite_dsh_settings_in_guest(&distro, &content)
        }
    })
    .await
    .map_err(|e| format!("保存 DSH 设置任务异常终止：{e}"))?
}
/// MCP 管理：获取指定 profile 的 MCP 服务列表（4.7）
#[tauri::command]
pub async fn list_mcp_servers(
    app: tauri::AppHandle,
    profile: String,
) -> Result<Vec<crate::mcp::McpServerConfig>, String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::mcp::list_mcp_servers(&home, &profile)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::mcp::list_mcp_servers_in_guest(&distro, &profile)
        }
    })
    .await
    .map_err(|e| format!("读取 MCP 服务列表任务异常终止：{e}"))?
}
/// MCP 管理：保存或更新单个 MCP 服务（4.7）
#[tauri::command]
pub async fn save_mcp_server(
    app: tauri::AppHandle,
    profile: String,
    server: crate::mcp::McpServerConfig,
) -> Result<(), String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::mcp::save_mcp_server(&home, &profile, server)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::mcp::save_mcp_server_in_guest(&distro, &profile, server)
        }
    })
    .await
    .map_err(|e| format!("保存 MCP 服务任务异常终止：{e}"))?
}
/// MCP 管理：删除指定 MCP 服务（4.7）。
///
/// `scope` 由前端按 `list_mcp_servers` 回传的条目所属层给（2026-09-18）：删除语义是
/// "从这条**生效范围**里拿掉"，只删 profile 层会让全局层条目**删不掉**（此前正是如此）。
/// `row_id` 同理带上（2026-09-18 三修）：同层重复 `serverName` 时只有行 id 能确定用户
/// 点的是哪一条，缺了它会删掉**列表里另一条**同名行。
#[tauri::command]
pub async fn delete_mcp_server(
    app: tauri::AppHandle,
    profile: String,
    server_name: String,
    scope: Option<crate::mcp::McpScope>,
    row_id: Option<String>,
) -> Result<(), String> {
    let world = crate::mgmt::current_world(&app)?;
    let scope = scope.unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::mcp::delete_mcp_server(&home, &profile, &server_name, scope, row_id.as_deref())
        }
        crate::mgmt::World::Wsl { distro } => crate::mcp::delete_mcp_server_in_guest(
            &distro,
            &profile,
            &server_name,
            scope,
            row_id.as_deref(),
        ),
    })
    .await
    .map_err(|e| format!("删除 MCP 服务任务异常终止：{e}"))?
}
/// MCP 管理：探测指定服务器的能力（2026-09-15，ADR-0022 **stdio 分支**）。
///
/// 主动连上服务器、握手后枚举 Tools / Resources / Resource Templates，
/// 供「MCP 工作台」做**配置体检与失败归因**（ADR-0022 §1.4 的价值主张）。
/// 合规：spawn 经 `lifecycle` seam（`Role::Probe`）；本模块**无进程内网络客户端**
/// （`network_gate` 的 `Registered` 行反向绑定这一点）。`streamable-http` 分支
/// 尚未实现——此类服务器会被 `probe_stdio` 明确拒绝并指出该走 http 分支。
///
/// 世界择源：**WSL 客体档显式报错**。探测要 spawn 服务器命令，客体档下该命令应在
/// 客体里跑；在宿主 spawn 等于跑成"宿主的另一个进程"，语义错。故不回落本地。
#[tauri::command]
pub async fn probe_mcp_server(
    app: tauri::AppHandle,
    profile: String,
    server_name: String,
    scope: Option<crate::mcp::McpScope>,
    row_id: Option<String>,
) -> Result<crate::mcp_probe::McpProbe, String> {
    let world = crate::mgmt::current_world(&app)?;
    let home = match world {
        crate::mgmt::World::Local => crate::resolve::user_dsh_home(),
        crate::mgmt::World::Wsl { .. } => {
            return Err(
                "MCP 能力探测暂不支持 WSL 客体档：探测需在客体内部 spawn 服务器命令，\
                 宿主侧执行语义不等价（有意不回落本地）。请在本地档使用。"
                    .to_string(),
            )
        }
    };
    // 探测要复现 **dsh 子进程的解析条件**（2026-09-18）：PATH 用同源的
    // `dsh_child_path`——否则会出现"探测通过、dsh 里却 ENOENT"的假通过。
    //
    // **不得退化成当前进程 PATH**（2026-09-18 二修）：引擎未就绪时退着探，探到的
    // 是"从 Dock 进程继承的 PATH 里恰好有什么"，而 dsh 子进程拿的是另一套 PATH ⇒
    // 这类"能跑通"的结论会直接误导用户去点那条不存在的 `npx`。宁可显式失败。
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let crate::engines::DshToolchain::Engine { node_bin, .. } =
        crate::engines::resolve_toolchain(&data_dir)?;
    let path_env = crate::resolve::dsh_child_path(&node_bin, &data_dir);
    let scope = scope.unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || {
        let servers = crate::mcp::list_mcp_servers(&home, &profile)?;
        // 同名条目可能同时存在于两层 ⇒ 按 (name, scope) 定位用户点的那一行；
        // **同层**还有重复时只有行 id 能区分（2026-09-18 三修）——探错一条会把
        // 另一条的结论挂到这一行上，比不探更坏。行 id 为空 ⇒ 回退按 (name, scope)。
        let want_row = row_id.as_deref().filter(|s| !s.is_empty());
        let server = servers
            .into_iter()
            .find(|s| {
                s.name == server_name
                    && s.scope == scope
                    && want_row.is_none_or(|id| s.row_id == id)
            })
            .ok_or_else(|| {
                format!("profile「{profile}」未配置名为「{server_name}」的 MCP 服务器")
            })?;
        // `!!js process.env.X` 行**不探**（2026-09-18 二修）：壳只会把等号右边的
        // **字面量**当 env 值传下去，而 dsh 侧是**求值后**的真实 token ⇒ 探测要么
        // 假失败（401）要么带着错凭据假通过。两者都比不探更坏。
        if server.expr {
            return Err(format!(
                "MCP 服务器「{}」的配置含 `!!js` 表达式（从环境变量取值），\
                 壳无法在探测里复现 dsh 的求值 ⇒ 不探测（避免假通过/假失败）。\
                 要验证能力，请在 dsh 会话里直接用它的工具。",
                server.name
            ));
        }
        // 整轮总期限（启动 + 握手 + 三次枚举）：卡住的服务器不得挂死详情页。
        // 30s（2026-09-18 二修，原 15s）：冷启动要走 `pnpm dlx` / 首跑装包，
        // 15s 会把"其实在装依赖"误判成失败。
        // 按 transport 分派（ADR-0022 §3.3）：两条分支**各自有独立的合规通道**
        // ——stdio 走子进程，streamable-http 走条目级豁免的进程内 HTTP。
        let budget = std::time::Duration::from_secs(30);
        match server.transport {
            crate::mcp::McpTransport::Stdio => {
                crate::mcp_probe::probe_stdio(&server, budget, Some(path_env.as_str()))
            }
            crate::mcp::McpTransport::StreamableHttp => {
                crate::mcp_probe::probe_http(&server, budget)
            }
        }
    })
    .await
    .map_err(|e| format!("MCP 能力探测任务异常终止：{e}"))?
}
