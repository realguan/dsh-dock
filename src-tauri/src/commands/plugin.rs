//! commands/plugin.rs —— plugin 域 IPC 命令处理层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 本层只做「参数校验 + 数据目录定位 + 阻塞动作下沉 spawn_blocking」，
//! 业务实现全在对应域模块（profiles/plugins/sessions/settings/…）。
//! 命令清单的唯一事实源仍是 `src/ipc.rs::COMMANDS`，三处同步由 cargo test 闸门拦。

use crate::boot::{active_session_profile, ShellState};
use std::sync::Arc;

use tauri::Manager;

/// 策展条目的**挂载行写入**（2026-09-15，ADR-0020）：把一条 `- insert: [{id, name}]`
/// 并入 profile 的 `cordis.patch.yml`（经 `plugins.rs::PatchFile`：覆写前备份 ＋
/// 原子替换 ＋ 未改条目原文保真；**幂等**，同 id 已存在则零写入返回 `false`）。
///
/// **调用方必须只在 `Activation::InsertRow` 时调用**（判定见 `official_catalog`）：
/// 声明了 `dsh.bundle` 的包由 `dsh plugin add` 自行激活，再写一条 `insert`
/// 会**重复挂载**（行身份是 `id`，同名不同 id 即两份实例）。
///
/// **职责边界**：本命令只写行、**不负责安装**——安装仍走既有 `install_plugin`
/// （`dsh plugin add` 转发链）与前端串行安装队列。前端按目录行的 `steps` **按序**
/// 执行「装 → 若该步需行则写行」，任一步失败即停在一致态（可续装）。
/// 这样复用既有队列的 pnpm 审批门/错误分类，不必在壳内复制一条安装链。
///
/// 世界择源同 `list_official_plugins`：WSL 客体档暂不支持（**显式报错，不回落本地写**
/// ——写错 profile 比报错严重得多）。
#[tauri::command]
pub async fn apply_official_patch_row(
    app: tauri::AppHandle,
    profile: String,
    row_id: String,
    package: String,
) -> Result<bool, String> {
    let world = crate::mgmt::current_world(&app)?;
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let home = match &world {
        crate::mgmt::World::Local => crate::resolve::user_dsh_home(),
        crate::mgmt::World::Wsl { .. } => {
            return Err(
                "官方策展的挂载行写入暂不支持 WSL 客体档：需补客体侧 patch 写原语后方可启用\
                 （有意不回落本地写入，以免写错 profile）。请在本地档使用。"
                    .to_string(),
            )
        }
    };
    tauri::async_runtime::spawn_blocking(move || {
        let changed =
            crate::plugins::ensure_catalog_insert_row(&home, &profile, &row_id, &package)?;
        // 写后自证（2026-09-15 补，ADR-0020）：回读 dump-config 组合树确认该行真的生效。
        // 理由：patch 写法不对时 DSH 会**退出码 0 地静默丢弃**条目，只校验"文件写成功"
        // 抓不到它。dump-config 自身失败时**不得谎报成功**——明确告知"已写入但未能复核"。
        let rows =
            crate::plugins::plugin_rows_blocking(&profile, &data_dir, &world).map_err(|e| {
                format!(
                    "挂载行已写入，但写后复核未能执行（dump-config 读取失败）：{e}\
                     ——请人工确认该行是否生效。"
                )
            })?;
        crate::plugins::verify_catalog_row(&rows, &row_id, &package)?;
        Ok(changed)
    })
    .await
    .map_err(|e| format!("策展挂载行写入任务异常终止：{e}"))?
}

/// 官方插件策展目录（2026-09-15，ADR-0020 已接受）：把 `official_catalog::CATALOG`
/// 解析成**可下发**的目录行——每步带钉版本 spec、激活方式（bundle 自动激活 vs
/// 须写 `insert` 行）、稳定行 `id`、已装标记、互斥冲突与版本错配提示。
///
/// **已装态采集在命令层**（本层职责即"数据目录定位"），`official_catalog` 保持纯函数：
/// 读 `profiles/<名>/package.json` 的 `dependencies`，再逐个读
/// `profiles/<名>/node_modules/<包>/package.json`，据其是否声明 **`dsh.bundle.patch`**
/// 判定激活方式——这是激活契约的**唯一分类依据**（ADR-0020 §2.8 禁双源），
/// 不得用包名/来源等启发式代替。
///
/// `runtime_version` **由后端本地检出**（`updates::detect_current_version`，离线读
/// `engines/`），不从前端传——前端没有廉价且权威的来源，传参只会引入漂移。
/// 检出不到时**不拼坏 spec**：退回裸包名并在该步的 `versionNotice` 里**明示未钉版本**
/// （裸包名按 `latest` 解析，而 Agent Teams 三包的 `latest` 实测落后于运行时，
/// 同台账行 7 的 dsh-base 事故）。
#[tauri::command]
pub async fn list_official_plugins(
    app: tauri::AppHandle,
    profile: String,
) -> Result<Vec<crate::official_catalog::CatalogRow>, String> {
    // 世界择源（ADR-0016 §5-b/c，**绝不回落本地**）：本地 = 宿主 home 直读；
    // **WSL 客体档当前显式报错**——客体侧需补一组客体文件读原语才能拼出同一份
    // `InstalledState`，本轮未实现。此处宁可如实报"该档暂不支持"，也**不**去读宿主
    // home（那会把客体 profile 的插件全报成"未安装"＝静默错数据）。
    let world = crate::mgmt::current_world(&app)?;
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let home = match world {
        crate::mgmt::World::Local => crate::resolve::user_dsh_home(),
        crate::mgmt::World::Wsl { .. } => {
            return Err(
                "官方插件策展目录暂不支持 WSL 客体档：需补客体侧 profile 读原语后方可启用\
                 （有意不回落本地读取，以免把客体插件误报成未安装）。请在本地档使用。"
                    .to_string(),
            )
        }
    };
    tauri::async_runtime::spawn_blocking(move || {
        let dir = home.join("profiles").join(&profile);
        let manifest = std::fs::read_to_string(dir.join("package.json"))
            .map_err(|e| format!("读取 profile 清单失败（{profile}）：{e}"))?;
        let installed = crate::plugins::dependency_names(&manifest)?;

        // 逐个依赖读其自身 package.json：声明 `dsh.bundle.patch` 即在册。
        // 读不到（未安装/纯内置）按**未声明**处理——保守方向：宁可让壳多写一条
        // `insert` 行（幂等、可复核），也不误判成"已自动激活"而漏挂。
        let declared_bundles: Vec<String> = installed
            .iter()
            .filter(|name| {
                std::fs::read_to_string(dir.join("node_modules").join(name).join("package.json"))
                    .ok()
                    .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
                    .and_then(|pkg| pkg.pointer("/dsh/bundle/patch").map(|v| !v.is_null()))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        // 运行时版本**本地离线检出**（读 `engines/`，不触网）：钉版本的唯一依据。
        // 检出不到 → `None`，由 `resolve_rows` 走诚实降级（裸包名 + 显式告知）。
        let runtime_version = crate::updates::detect_current_version(&data_dir);

        Ok(crate::official_catalog::resolve_rows(
            &crate::official_catalog::InstalledState {
                installed,
                declared_bundles,
                latest_by_package: Default::default(),
            },
            runtime_version.as_deref(),
        ))
    })
    .await
    .map_err(|e| format!("官方目录解析任务异常终止：{e}"))?
}

/// 插件清单（4.4①，Spike B 方案）：静态清单 = bundles（官方内置）+
/// dependencies（第三方，含已装版本/描述）。阻塞文件操作走 spawn_blocking。
///
/// 世界择源（ADR-0016 §5-b/c）：本地 = 宿主 home 直读；WSL 客体 = 客体读原语
/// 批量取原文，解析复用同一份纯装配函数。**绝不回落本地**（`current_world` 失败即报错）。
#[tauri::command]
pub async fn list_profile_plugins(
    app: tauri::AppHandle,
    profile: String,
) -> Result<Vec<crate::plugins::PluginEntry>, String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::plugins::list_profile_plugins(&home, &profile)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::plugins::list_profile_plugins_in_guest(&distro, &profile)
        }
    })
    .await
    .map_err(|e| format!("清单任务异常终止：{e}"))?
}
/// 插件运行态快照（一次性，不订阅）：仅活跃会话的 profile 有数据——
/// 无会话返回 `{profile: None, entries: []}`。回环只读查询见 AGENTS §7
/// 登记与复现点 11；阻塞 HTTP 走 spawn_blocking（2s 超时兜底）。
#[tauri::command]
pub async fn get_plugin_runtime(
    app: tauri::AppHandle,
) -> Result<crate::plugins::PluginRuntimeSnapshot, String> {
    let target = {
        let state = app.state::<Arc<ShellState>>().inner().clone();
        let profile = active_session_profile(&app);
        let origin = state.workbench_url.lock().unwrap().as_ref().map(|u| {
            format!(
                "{}://{}{}",
                u.scheme(),
                u.host_str().unwrap_or("127.0.0.1"),
                u.port().map(|p| format!(":{p}")).unwrap_or_default()
            )
        });
        // 2026-09-15：必须带上启动期兑换的 `/api` 会话 Cookie——dsh 0.1.6-alpha.1
        // 加了 `browserAuth` 栅栏（失败 401），不带 Cookie 会恒 401，本命令此前即
        // 因此静默失效（复现点 11 原记"回环无鉴权门"已过时）。
        let cookie = state.workbench_cookie.lock().unwrap().clone();
        (profile, origin, cookie)
    };
    match target {
        (Some(profile), Some(origin), cookie) => {
            let entries = tauri::async_runtime::spawn_blocking(move || {
                crate::plugins::fetch_runtime_snapshot(&origin, cookie.as_deref())
            })
            .await
            .map_err(|e| format!("运行态任务异常终止：{e}"))??;
            Ok(crate::plugins::PluginRuntimeSnapshot {
                profile: Some(profile),
                entries,
            })
        }
        _ => Ok(crate::plugins::PluginRuntimeSnapshot {
            profile: None,
            entries: Vec::new(),
        }),
    }
}
/// 安装/卸载/更新插件（4.4②）：`dsh plugin --profile <名> add/remove/update`
/// 转发链（复用创建刀基建，pnpm 防御补齐同源）；阻塞转发走 spawn_blocking，
/// 超时同创建 600s。ok=false 时 detail 带输出尾部，前端按警示态展示。
///
/// 世界择源（ADR-0016 §5-b）：本地 = 宿主引擎 + 宿主 home；WSL 客体 = 客体
/// `dsh` CLI（同一条链路的客体孪生，网络发生在客体进程内，ADR-0004 §7）。
#[tauri::command]
pub async fn install_plugin(
    app: tauri::AppHandle,
    profile: String,
    package: String,
) -> Result<crate::plugins::PluginOpOutcome, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("定位数据目录失败：{e}"))?;
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        crate::plugins::mutate_plugin_blocking(
            crate::plugins::PluginOp::Install,
            &profile,
            &package,
            &data_dir,
            &world,
        )
    })
    .await
    .map_err(|e| format!("安装任务异常终止：{e}"))?
}
#[tauri::command]
pub async fn remove_plugin(
    app: tauri::AppHandle,
    profile: String,
    package: String,
) -> Result<crate::plugins::PluginOpOutcome, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("定位数据目录失败：{e}"))?;
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        crate::plugins::mutate_plugin_blocking(
            crate::plugins::PluginOp::Remove,
            &profile,
            &package,
            &data_dir,
            &world,
        )
    })
    .await
    .map_err(|e| format!("卸载任务异常终止：{e}"))?
}
#[tauri::command]
pub async fn update_plugin(
    app: tauri::AppHandle,
    profile: String,
    package: String,
) -> Result<crate::plugins::PluginOpOutcome, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("定位数据目录失败：{e}"))?;
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        crate::plugins::mutate_plugin_blocking(
            crate::plugins::PluginOp::Update,
            &profile,
            &package,
            &data_dir,
            &world,
        )
    })
    .await
    .map_err(|e| format!("更新任务异常终止：{e}"))?
}
/// 插件行表（4.4③）：`dsh --profile <名> --dump-config` 行 id↔包名配对 +
/// 壳 patch toggle 态——行 id 不可从包名推导（ADR-0009 第四次修订），一次
/// spawn 全量拿到。阻塞 spawn 走 spawn_blocking。世界择源同插件清单。
#[tauri::command]
pub async fn get_plugin_rows(
    app: tauri::AppHandle,
    profile: String,
) -> Result<Vec<crate::plugins::PluginRowState>, String> {
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("定位数据目录失败：{e}"))?;
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        crate::plugins::plugin_rows_blocking(&profile, &data_dir, &world)
    })
    .await
    .map_err(|e| format!("行表任务异常终止：{e}"))?
}
/// 禁用/启用切换（4.4③）：patch 写入例外 #3（`{id, disabled}` 单键，
/// ADR-0009 第四次修订）；运行中会话不热生效，重启承接。
///
/// **世界择源（2026-09-11 第三批）**：WSL 模式读客体 patch 原文、同一份纯变换、
/// 客体侧原子写回——装上了却关不掉是半截功能（行表已在 P1 下沉）。
#[tauri::command]
pub async fn set_plugin_disabled(
    app: tauri::AppHandle,
    profile: String,
    row_id: String,
    disabled: bool,
) -> Result<(), String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::plugins::set_plugin_disabled(&home, &profile, &row_id, disabled)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::plugins::set_plugin_disabled_in_guest(&distro, &profile, &row_id, disabled)
        }
    })
    .await
    .map_err(|e| format!("切换任务异常终止：{e}"))?
}
/// 更新检查（4.4④）：逐外挂插件查 registry dist-tags.latest（外网经
/// `updates.rs` 镜像链，§7 已登记）；串行阻塞走 spawn_blocking，按钮触发。
///
/// **世界择源（2026-09-11 第三批）**：已装版本来自当前世界（WSL = 客体清单）；
/// registry 查询仍是 `updates.rs` 唯一网络面（ADR-0016 §1：registry 拉取与模式无关）。
#[tauri::command]
pub async fn check_plugin_updates(
    app: tauri::AppHandle,
    profile: String,
) -> Result<crate::plugins::PluginUpdateReport, String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::plugins::check_updates_blocking(&home, &profile)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::plugins::check_updates_blocking_in_guest(&distro, &profile)
        }
    })
    .await
    .map_err(|e| format!("更新检查任务异常终止：{e}"))?
}
/// 版本列表（选版本更新，4.4④）：降序最新在前；外网同镜像链。
/// 纯 registry 查询（不读任何 home）→ 与运行世界无关，无需择源。
#[tauri::command]
pub async fn list_plugin_versions(package: String) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || crate::plugins::plugin_versions_blocking(&package))
        .await
        .map_err(|e| format!("版本查询任务异常终止：{e}"))?
}
/// 插件总览聚合（4.4④ 收口，ADR-0009 第五次修订）：全部已物化 profile 的第
/// 三方插件按包名归组。只读纯文件扫描（零 dsh 子进程、零网络），spawn_blocking。
///
/// **世界择源（2026-09-11 第三批）**：按当前世界的全部 profile 聚合
/// （WSL = 客体扫描 + 客体清单；纯读，零 dsh 子进程、零网络）。
#[tauri::command]
pub async fn list_all_plugins(
    app: tauri::AppHandle,
) -> Result<Vec<crate::plugins::AggregatePlugin>, String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => Ok(crate::plugins::aggregate_plugins_blocking(
            &crate::resolve::user_dsh_home(),
        )),
        crate::mgmt::World::Wsl { distro } => {
            crate::plugins::aggregate_plugins_blocking_in_guest(&distro)
        }
    })
    .await
    .map_err(|e| format!("聚合任务异常终止：{e}"))?
}
/// 配置行原样复制（4.4④ 收口，patch 写入例外 #4，ADR-0009 第五次修订）：
/// 来源 patch 中该插件行 id 的全部条目 → 追加到目标 patch（只追加不覆盖，
/// 目标已有同 id 条目则零写入 skipped）。dump-config spawn + 文件操作走
/// spawn_blocking。
///
/// **世界择源（ADR-0016 §4 P2）**：WSL 模式读写客体 patch 并使用客体行表。
#[tauri::command]
pub async fn copy_plugin_config(
    app: tauri::AppHandle,
    source: String,
    target: String,
    package: String,
) -> Result<crate::plugins::CopyConfigOutcome, String> {
    let world = crate::mgmt::current_world(&app)?;
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("定位数据目录失败：{e}"))?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => crate::plugins::copy_plugin_config_blocking(
            &crate::resolve::user_dsh_home(),
            &source,
            &target,
            &package,
            &data_dir,
        ),
        crate::mgmt::World::Wsl { distro } => crate::plugins::copy_plugin_config_in_guest(
            &distro, &source, &target, &package, &data_dir,
        ),
    })
    .await
    .map_err(|e| format!("配置复制任务异常终止：{e}"))?
}
