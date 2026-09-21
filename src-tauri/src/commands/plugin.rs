//! commands/plugin.rs —— plugin 域 IPC 命令处理层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 本层只做「参数校验 + 数据目录定位 + 阻塞动作下沉 spawn_blocking」，
//! 业务实现全在对应域模块（profiles/plugins/sessions/settings/…）。
//! 命令清单的唯一事实源仍是 `src/ipc.rs::COMMANDS`，三处同步由 cargo test 闸门拦。

use crate::boot::{active_session_profile, ShellState};
use std::path::Path;
use std::sync::Arc;

use tauri::Manager;

/// 宿主前置探测：策展集要求、但**dsh 子进程 PATH 里找不到**的可执行文件名。
///
/// 判据与 `spawn_dsh` 给子进程的 PATH **同源**（`dsh_child_path`）——否则会出现
/// "壳说在、dsh 找不到"的漂移，而这里判错的代价是用户装完 profile 起不来
/// （2026-09-16 真机事故，ADR-0020 §7.5）。探不到工具链（引擎半就绪）时按**缺失**
/// 处理：宁可不给装，也不给装出一个起不来的 profile。
fn missing_prerequisites(data_dir: &Path, packages: &[&str]) -> Vec<String> {
    let path_env = match crate::engines::resolve_toolchain(data_dir) {
        Ok(crate::engines::DshToolchain::Engine { node_bin, .. }) => {
            crate::resolve::dsh_child_path(&node_bin, data_dir)
        }
        Err(_) => String::new(),
    };
    let mut missing: Vec<String> = Vec::new();
    for package in packages {
        if let Some(command) = crate::official_catalog::required_command(package) {
            if !crate::resolve::command_on_path(command, &path_env)
                && !missing.iter().any(|m| m == command)
            {
                missing.push(command.to_string());
            }
        }
    }
    missing
}

/// 策展集里出现过的**全部包名**（前置探测的输入集：单源在目录数据，不另立清单）。
fn catalog_packages() -> Vec<&'static str> {
    crate::official_catalog::CAPABILITIES
        .iter()
        .flat_map(|cap| cap.variants.iter())
        .flat_map(|variant| variant.packages.iter().copied())
        .collect()
}

/// 策展条目的**挂载行写入**（2026-09-15 立；2026-09-16 §7.4 改为**写前当场重判**，
/// §7.5 加**必写 `config`** 与**宿主前置硬门**）：
/// 把一条 `- insert: [{id, name, config?}]` 并入 profile 的 `cordis.patch.yml`（经
/// `plugins.rs::PatchFile`：覆写前备份 ＋ 原子替换 ＋ 未改条目原文保真；**幂等**，
/// 同 id 且必需 `config` 已齐则零写入返回 `changed: false`）。
///
/// **为什么必须在这里重判**：激活方式由目标包的 `package.json` 决定，而目录是**安装前**
/// 拉的——那时包还没进 `node_modules`，任何包都只会被判成"未声明 `dsh.bundle`"。于是
/// profile 层包（如 `auto-review`）会被误判成需要写行，装完就多写一条 → **同一插件挂两份**。
/// 本命令在 `dsh plugin add` **完成之后**才被调用，故在这里重读分类才是准的：声明了
/// `dsh.bundle` 就**拒绝写行**并如实回报 `autoActivated: true`（ADR-0020 §2.8 唯一依据）。
///
/// **为什么还要前置硬门**：`cua-driver-mcp` 缺外部 `cua-driver` 时，写行 = 装出一个
/// 起不来的 profile（插件树加载阶段 `spawn cua-driver` ENOENT）。前端会禁用开关，
/// 但命令层必须**同样拒绝**——前端是呈现，不是闸门。
///
/// **职责边界**：本命令只写行、**不负责安装**——安装仍走既有 `install_plugin`
/// （`dsh plugin add` 转发链）与前端串行安装队列。前端按变体的 `steps` **按序**执行
/// 「装 → 确保行」，任一步失败即停在一致态（可续装）。
///
/// 世界择源同 `list_experimental_capabilities`：**两侧都已下沉**（2026-09-21，P0/P1）——
/// 本地走宿主内核，客体走同一内核的孪生（读客体原文 → 变更 → 渲回客体，备份与原子替换
/// 由 `guest::backup_file` / `guest::write_home_files` 保证）；前置硬门与 bundle 分类
/// 也按世界分派（**同一问题在同一世界问**）。绝不回落本地写 —— 写错 profile 比报错严重得多。
#[tauri::command]
pub async fn apply_official_patch_row(
    app: tauri::AppHandle,
    profile: String,
    row_id: String,
    package: String,
) -> Result<crate::official_catalog::RowWriteOutcome, String> {
    let world = crate::mgmt::current_world(&app)?;
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    // 客体档（2026-09-21 下沉，P0）：写侧交客体孪生（同一内核）；宿主 home 仅在本地档需要。
    let home = match &world {
        crate::mgmt::World::Local => Some(crate::resolve::user_dsh_home()),
        crate::mgmt::World::Wsl { .. } => None,
    };
    let verify_profile = profile.clone();
    tauri::async_runtime::spawn_blocking(move || {
        // 写前当场重判（见函数文档）：包已装，此时读它的 package.json 才是权威分类。
        let declares_bundle = match &world {
            crate::mgmt::World::Local => crate::plugins::package_declares_bundle(
                &home
                    .as_deref()
                    .expect("本地档必有 home")
                    .join("profiles")
                    .join(&profile),
                &package,
            ),
            crate::mgmt::World::Wsl { distro } => {
                crate::plugins::package_declares_bundle_in_guest(distro, &profile, &package)?
            }
        };
        if !crate::plugins::needs_insert_row(declares_bundle) {
            // 声明了 `dsh.bundle`：CLI 已把它追加进层栈完成激活，再写 insert 会**重复挂载**。
            return Ok(crate::official_catalog::RowWriteOutcome {
                changed: false,
                auto_activated: true,
            });
        }
        // 宿主前置硬门（2026-09-16 §7.5）：前置未满足时**拒绝写行**——写下去就是
        // 一个起不来的 profile，而用户此时的处境是"应用再也进不去"。
        if let Some(command) = crate::official_catalog::required_command(&package) {
            // 宿主档查宿主 PATH；**客体档必须查客体 PATH** —— 服务器是在客体里启动的，
            // 拿宿主 PATH 判会误拒（客体有、宿主无）或误放（反之，然后把起不来的 profile
            // 写坏，而那正是这道门要防的）。同一问题必须在同一世界里问（P0 接线，2026-09-21）。
            let (missing, where_) = match &world {
                crate::mgmt::World::Local => (
                    missing_prerequisites(&data_dir, &[package.as_str()]).is_empty(),
                    "本机 PATH",
                ),
                crate::mgmt::World::Wsl { distro } => (
                    crate::guest::missing_commands(distro, &[command])?.is_empty(),
                    "客体 PATH",
                ),
            };
            if !missing {
                return Err(format!(
                    "已拒绝写入挂载行：{where_} 中找不到「{command}」，装上会让 dsh 在加载\
                     插件时直接失败、工作台起不来。请先装好 {command}（或改用自包含的那一档能力），\
                     再重试。"
                ));
            }
        }
        let row_config = crate::official_catalog::required_row_config(&package);
        let changed = match &world {
            crate::mgmt::World::Local => crate::plugins::ensure_catalog_insert_row(
                home.as_deref().expect("本地档必有 home"),
                &profile,
                &row_id,
                &package,
                row_config,
            )?,
            crate::mgmt::World::Wsl { distro } => {
                crate::plugins::ensure_catalog_insert_row_in_guest(
                    distro, &profile, &row_id, &package, row_config,
                )?
            }
        };
        // 写后自证（2026-09-15 补，ADR-0020）：回读 dump-config 组合树确认该行真的生效。
        // 理由：patch 写法不对时 DSH 会**退出码 0 地静默丢弃**条目，只校验"文件写成功"
        // 抓不到它。dump-config 自身失败时**不得谎报成功**——明确告知"已写入但未能复核"。
        let rows = crate::plugins::plugin_rows_blocking(&verify_profile, &data_dir, &world)
            .map_err(|e| {
                format!(
                    "挂载行已写入，但写后复核未能执行（dump-config 读取失败）：{e}\
                         ——请人工确认该行是否生效。"
                )
            })?;
        crate::plugins::verify_catalog_row(&rows, &row_id, &package)?;
        Ok(crate::official_catalog::RowWriteOutcome {
            changed,
            auto_activated: false,
        })
    })
    .await
    .map_err(|e| format!("策展挂载行写入任务异常终止：{e}"))?
}

/// **反向原语**：删除壳写过的策展挂载行（2026-09-16，ADR-0020 §7.2-3）。
///
/// 与 `apply_official_patch_row` 对称：写行有自证，删行同样有自证——写后复核该行
/// **已不在**组合树中（"删了却还在"是同一类静默失败）。
///
/// 只接受 `dsh-dock-` 前缀的行 id（所有权判据在 `plugins::remove_catalog_insert_row`）：
/// bundle 自带行与用户手写行**不得**由壳代删。
///
/// 世界择源同 `list_experimental_capabilities`：WSL 客体档显式报错，不回落本地写。
#[tauri::command]
pub async fn remove_official_patch_row(
    app: tauri::AppHandle,
    profile: String,
    row_id: String,
) -> Result<bool, String> {
    let world = crate::mgmt::current_world(&app)?;
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    // 客体档（2026-09-21 下沉，P0）：写侧交客体孪生（同一内核），**不再需要宿主 home**；
    // 自证仍走 world-aware 的 `plugin_rows_blocking`（它本就按世界取组合树）。
    let home = match &world {
        crate::mgmt::World::Local => Some(crate::resolve::user_dsh_home()),
        crate::mgmt::World::Wsl { .. } => None,
    };
    let verify_profile = profile.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let changed = match &world {
            crate::mgmt::World::Local => crate::plugins::remove_catalog_insert_row(
                home.as_deref().expect("本地档必有 home"),
                &profile,
                &row_id,
            )?,
            crate::mgmt::World::Wsl { distro } => {
                crate::plugins::remove_catalog_insert_row_in_guest(distro, &profile, &row_id)?
            }
        };
        // 删除后自证：回读 dump-config，确认该行真的不在组合树里了。
        let rows = crate::plugins::plugin_rows_blocking(&verify_profile, &data_dir, &world)
            .map_err(|e| {
                format!(
                    "挂载行已删除，但删除后复核未能执行（dump-config 读取失败）：{e}\
                     ——请人工确认该行是否已消失。"
                )
            })?;
        if rows.iter().any(|r| r.id == row_id) {
            return Err(format!(
                "删除后复核未通过：行 id「{row_id}」仍在组合树中——\
                 文件已改写但该行仍被 DSH 采纳，请人工核对该 profile 的 cordis.patch.yml。"
            ));
        }
        Ok(changed)
    })
    .await
    .map_err(|e| format!("策展挂载行删除任务异常终止：{e}"))?
}

/// 实验能力目录（2026-09-15 立；2026-09-16 第二次修订改名 + 改形，ADR-0020 §7）。
///
/// 返回**能力 → 变体 → 步骤**三级事实视图，能力状态（`off`/`on`/`disabled`/
/// `partial`/`conflict`）由后端一次算全——状态是「包 × 行 × disabled」的函数，
/// 前端不得凭 `installed` 猜（禁双源）。
///
/// **已装态与行态采集在命令层**（本层职责即"数据目录定位 + 起一次 dump-config"）：
/// ① 读 `profiles/<名>/package.json` 的 `dependencies`；
/// ② 逐个读 `profiles/<名>/node_modules/<包>/package.json`，据其是否声明
/// **`dsh.bundle.patch`** 判定激活方式——这是激活契约的**唯一分类依据**
/// （ADR-0020 §2.8 禁双源），不得用包名/来源等启发式代替；
/// ③ `plugin_rows_blocking` 一次 `--dump-config` 拿到挂载行表（行是否存在、是否被
/// `disabled` 停用、bundle 贡献了哪些行）——这是"关得掉吗 / 真生效了吗"的唯一来源。
///
/// `runtime_version` **由后端本地检出**（`updates::detect_current_version`，离线读
/// `engines/`），不从前端传——前端没有廉价且权威的来源，传参只会引入漂移。
/// 检出不到时**不拼坏 spec**：退回裸包名并在该步的 `versionNotice` 里**明示未钉版本**
/// （裸包名按 `latest` 解析，而 Agent Teams 三包的 `latest` 实测落后于运行时）。
///
/// `lang`（2026-09-18 边界A）：前端传当前界面 locale，视图**按请求语言出品**
/// （单语 payload，见 `official_catalog::CopyLang`）；缺省/未知回退中文（目录原文）。
#[tauri::command]
pub async fn list_experimental_capabilities(
    app: tauri::AppHandle,
    profile: String,
    lang: Option<String>,
) -> Result<Vec<crate::official_catalog::CapabilityView>, String> {
    let lang = crate::official_catalog::CopyLang::from_tag(lang.as_deref());
    // 世界择源（ADR-0016 §5-b/c，**绝不回落本地**）：本地 = 宿主 home 直读；
    // 客体档（2026-09-21 下沉，P1）= 读**客体** profile 的同一组文件（一次批量往返），
    // 分类判据与后续分类逻辑**两侧共用**。绝不读宿主 home —— 那会把客体 profile 的插件
    // 全报成"未安装"（静默错数据，比报错更坏）。
    let world = crate::mgmt::current_world(&app)?;
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let home = match &world {
        crate::mgmt::World::Local => Some(crate::resolve::user_dsh_home()),
        crate::mgmt::World::Wsl { .. } => None,
    };
    tauri::async_runtime::spawn_blocking(move || {
        // 插件事实（已装依赖 + 哪些声明了 dsh.bundle）—— **按世界取，同一组判据**。
        // 第三个元素 = 客体档一次批量读到的各包 manifest（本地档为 None，走直读）。
        let (installed, declared_bundles, guest_manifests) = match &world {
            crate::mgmt::World::Local => {
                let dir = home
                    .as_deref()
                    .expect("本地档必有 home")
                    .join("profiles")
                    .join(&profile);
                let manifest = std::fs::read_to_string(dir.join("package.json"))
                    .map_err(|e| format!("读取 profile 清单失败（{profile}）：{e}"))?;
                let installed = crate::plugins::dependency_names(&manifest)?;
                // 逐个依赖读其自身 package.json：声明 `dsh.bundle.patch` 即在册（判据单源在
                // `plugins::manifest_declares_bundle`）。读不到按**未声明**处理——保守方向：
                // 宁可让壳多写一条 `insert` 行（幂等、可复核），也不误判成"已自动激活"而漏挂。
                let declared: Vec<String> = installed
                    .iter()
                    .filter(|name| crate::plugins::package_declares_bundle(&dir, name))
                    .cloned()
                    .collect();
                (installed, declared, None)
            }
            crate::mgmt::World::Wsl { distro } => {
                let rel_manifest = format!("profiles/{profile}/package.json");
                let files = crate::guest::read_files(distro, std::slice::from_ref(&rel_manifest))?;
                let manifest = files
                    .into_iter()
                    .next()
                    .and_then(|(_, content)| content)
                    .ok_or_else(|| {
                        format!("读取 profile 清单失败（{profile}）：客体内不存在 {rel_manifest}")
                    })?;
                let installed = crate::plugins::dependency_names(&manifest)?;
                // **一次批量往返**读全部依赖的 package.json（逐个调用会是 N 次 wsl.exe）。
                let rels: Vec<String> = installed
                    .iter()
                    .map(|name| format!("profiles/{profile}/node_modules/{name}/package.json"))
                    .collect();
                let contents: std::collections::HashMap<String, String> =
                    crate::guest::read_files(distro, &rels)?
                        .into_iter()
                        .filter_map(|(rel, content)| content.map(|c| (rel, c)))
                        .collect();
                let declared: Vec<String> = installed
                    .iter()
                    .filter(|name| {
                        let rel = format!("profiles/{profile}/node_modules/{name}/package.json");
                        contents
                            .get(&rel)
                            .map(|text| crate::plugins::manifest_declares_bundle(text))
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect();
                (installed, declared, Some(contents))
            }
        };

        // 行态：一次 dump-config 拿全量行表（含 bundle 段落的贡献行合成条目）。
        let rows = crate::plugins::plugin_rows_blocking(&profile, &data_dir, &world)?;

        // 运行时版本**本地离线检出**（读 `engines/`，不触网）：钉版本的唯一依据。
        // 检出不到 → `None`，由 `resolve_capabilities` 走诚实降级（裸包名 + 显式告知）。
        let runtime_version = crate::updates::detect_current_version(&data_dir);

        // 宿主前置探测（2026-09-16 §7.5）：缺 `cua-driver` 之类的包**不得**被开关放行。
        let missing_commands = missing_prerequisites(&data_dir, &catalog_packages());

        // 已装包的**官方简介**（2026-09-17 维护者裁定「描述以官方为主」）：逐包读其
        // `node_modules/<包>/package.json` 的 description。未装的包读不到 ⇒ 不进表
        // （前端如实显示"装好后显示官方简介"），**不拿策展文案冒充**。
        let descriptions = installed
            .iter()
            .filter_map(|name| {
                // 客体档用**已批量读到**的那份 manifest（零额外往返）；本地档直读宿主文件。
                let desc = match &guest_manifests {
                    None => crate::plugins::installed_description(
                        home.as_deref().expect("本地档必有 home"),
                        &profile,
                        name,
                    ),
                    Some(map) => map
                        .get(&format!(
                            "profiles/{profile}/node_modules/{name}/package.json"
                        ))
                        .and_then(|text| crate::plugins::manifest_description(text)),
                };
                desc.map(|d| (name.clone(), d))
            })
            .collect();

        // 「这个安装自带哪些策展包」（2026-09-17）：dsh 0.1.6-alpha.2 起把官方实验层作为
        // optional bundle 随包下发。判据是**安装目录实测**而非写死的旗标——同一台机器上
        // dev 档引擎（0.1.6-alpha.1）不带、正式档（0.1.6-alpha.2）带，旗标必然在其中一边说谎。
        let catalog_pkgs: Vec<String> =
            catalog_packages().into_iter().map(str::to_string).collect();
        let shipped = crate::official_catalog::installation_shipped(
            &crate::engines::dsh_runtime_dir(&data_dir),
            &catalog_pkgs,
        );

        Ok(crate::official_catalog::resolve_capabilities(
            &crate::official_catalog::PackageFacts {
                installed,
                declared_bundles,
                missing_commands,
                descriptions,
                shipped,
            },
            &rows,
            runtime_version.as_deref(),
            lang,
        ))
    })
    .await
    .map_err(|e| format!("实验能力目录解析任务异常终止：{e}"))?
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
/// 一次插件操作的**源选择与双向兜底**（ADR-0006 §6）。
///
/// 为什么在命令层：策略要用 `settings`（偏好与记忆）与 `plugins::mutate_plugin_blocking`
/// （真正跑 pnpm）两侧，而这两侧都不该互相依赖。决策本身是纯函数
/// （`plugin_registry::{first_source, next_source}`），这里只做编排与回报。
///
/// 三条纪律：
/// - **只在换源有意义时换**（失败分类说了算；构建审批门/spec 非法不换，否则白等一轮还埋真因）；
/// - **显式偏好下绝不换源**（换源等于偷偷改用户配置）；
/// - **成功才写记忆**（抖动不得带偏下次的首试）。
fn mutate_with_registry_fallback(
    op: crate::plugins::PluginOp,
    profile: &str,
    spec: &str,
    data_dir: &Path,
    world: &crate::mgmt::World,
) -> Result<crate::plugins::PluginOpOutcome, String> {
    use crate::plugin_registry::{self as reg, RegistrySource};

    let settings = crate::settings::load(data_dir);
    let pref = reg::parse_pref(settings.plugin_registry.as_deref());
    let last_good = settings
        .plugin_registry_last_good
        .as_deref()
        .and_then(RegistrySource::from_key);
    let first = reg::first_source(pref, last_good);

    let mut outcome = crate::plugins::mutate_plugin_blocking(
        op,
        profile,
        spec,
        data_dir,
        world,
        first.registry_arg(),
    )?;
    if outcome.ok {
        remember_source(data_dir, pref, first);
        outcome
            .detail
            .push_str(&format!("（源：{}）", first.label_zh()));
        return Ok(outcome);
    }

    let Some(second) = reg::next_source(pref, first, &outcome.detail) else {
        return Ok(outcome);
    };
    let first_detail = outcome.detail.clone();
    let mut second_outcome = crate::plugins::mutate_plugin_blocking(
        op,
        profile,
        spec,
        data_dir,
        world,
        second.registry_arg(),
    )?;
    if second_outcome.ok {
        remember_source(data_dir, pref, second);
        second_outcome.detail = format!(
            "{}（{}不可用，已改用{}）\n—— 首次失败原因：{}",
            second_outcome.detail,
            first.label_zh(),
            second.label_zh(),
            summarize_failure(&first_detail)
        );
        return Ok(second_outcome);
    }
    // 两个源都失败：**两侧原因都报**——只报最后一个会让用户看不出真正的病根。
    second_outcome.detail = format!(
        "两个源都失败。\n· {}：{}\n· {}：{}",
        first.label_zh(),
        summarize_failure(&first_detail),
        second.label_zh(),
        summarize_failure(&second_outcome.detail)
    );
    Ok(second_outcome)
}

/// 只记成功（`auto` 才写记忆：显式偏好本就是用户的决定，无需壳记）。
fn remember_source(
    data_dir: &Path,
    pref: crate::plugin_registry::RegistryPref,
    source: crate::plugin_registry::RegistrySource,
) {
    if !matches!(pref, crate::plugin_registry::RegistryPref::Auto) {
        return;
    }
    let mut settings = crate::settings::load(data_dir);
    if settings.plugin_registry_last_good.as_deref() == Some(source.as_key()) {
        return;
    }
    settings.plugin_registry_last_good = Some(source.as_key().to_string());
    // 记忆写失败不阻断安装（它是优化，不是正确性）；下次仍会走"官方优先"。
    let _ = crate::settings::save(data_dir, &settings);
}

/// 失败摘要：取第一行有效信息（原始输出的尾部动辄多行，合并两条时更要短）。
fn summarize_failure(detail: &str) -> String {
    let line = detail
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("（无输出）");
    if line.chars().count() > 240 {
        format!("{}…", line.chars().take(240).collect::<String>())
    } else {
        line.to_string()
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
        mutate_with_registry_fallback(
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
        // 卸载不取包（不联网）：**不**走源策略，也不传 `--registry`。
        crate::plugins::mutate_plugin_blocking(
            crate::plugins::PluginOp::Remove,
            &profile,
            &package,
            &data_dir,
            &world,
            None,
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
        mutate_with_registry_fallback(
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

/// WSL 客体档接线不许回退的闸门（2026-09-21，P0）。
///
/// 这四处曾长期是 `World::Wsl => Err("需补客体侧 patch 写原语")` —— 审计 A3 判为**接线欠债**。
/// 回归到"显式拒绝"不会让任何测试变红（错误路径也能"正常"工作），所以必须用结构闸门钉住。
#[cfg(test)]
mod wsl_wiring_tests {
    /// 生产代码段（**剥掉本测试模块**）：`include_str!` 会把测试自身也读进来，
    /// 直接断言会**命中 needle 自身**而变成永真/永假 —— 这是本仓库踩过的自匹配坑。
    fn production_code() -> &'static str {
        include_str!("plugin.rs")
            .split("mod wsl_wiring_tests")
            .next()
            .expect("split 至少返回一段")
    }

    /// 两个写入命令都不得再对客体档整体拒绝，且都必须调用客体孪生。
    #[test]
    fn both_row_commands_are_wired_to_the_guest_twins() {
        let src = production_code();
        assert!(
            !src.contains("官方策展的挂载行写入暂不支持 WSL 客体档"),
            "写入命令不得退回「整体拒绝客体档」"
        );
        assert!(
            !src.contains("官方策展的挂载行删除暂不支持 WSL 客体档"),
            "删除命令不得退回「整体拒绝客体档」"
        );
        for call in [
            "ensure_catalog_insert_row_in_guest(",
            "remove_catalog_insert_row_in_guest(",
        ] {
            assert!(src.contains(call), "缺客体孪生接线：{call}");
        }
    }

    /// 前置硬门必须**按世界**问：宿主查宿主 PATH，客体查客体 PATH。
    /// 照搬宿主 PATH 判客体 = 误拒（客体有宿主无）或误放（反之，写坏 profile）。
    #[test]
    fn prerequisite_gate_asks_the_same_world() {
        let src = production_code();
        assert!(
            src.contains("crate::guest::missing_commands(distro, &[command])"),
            "客体档的前置硬门必须在客体里问（guest::missing_commands）"
        );
        assert!(
            src.contains("missing_prerequisites(&data_dir"),
            "本地档仍须查宿主 PATH（不得为了下沉而丢掉这道门）"
        );
    }

    /// P1：实验能力目录必须按世界取插件事实，且**不得**再对客体档整体拒绝。
    #[test]
    fn capability_catalog_reads_the_running_world() {
        let src = production_code();
        assert!(
            !src.contains("实验能力目录暂不支持 WSL 客体档"),
            "能力目录不得退回「整体拒绝客体档」"
        );
        assert!(
            src.contains("crate::guest::read_files(distro, &rels)"),
            "客体档必须**一次批量**读各包 manifest（逐个调用 = N 次 wsl.exe 往返）"
        );
        assert!(
            src.contains("manifest_description(text)"),
            "官方简介必须与宿主共用同一判据（禁第二份 manifest 解析）"
        );
    }

    /// bundle 分类必须按世界取同一判据（读失败 fail-closed，不得静默判 false）。
    #[test]
    fn bundle_classification_is_world_dispatch() {
        let src = production_code();
        assert!(
            src.contains("package_declares_bundle_in_guest("),
            "客体档必须读客体 manifest"
        );
        let plugins = include_str!("../plugins.rs").replace("\r\n", "\n");
        assert!(
            plugins.contains("fn manifest_declares_bundle("),
            "两侧必须共用同一个判定式（禁第二份判据）"
        );
    }
}
