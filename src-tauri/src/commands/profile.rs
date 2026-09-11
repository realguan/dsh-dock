//! commands/profile.rs —— profile 域 IPC 命令处理层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 本层只做「参数校验 + 数据目录定位 + 阻塞动作下沉 spawn_blocking」，
//! 业务实现全在对应域模块（profiles/plugins/sessions/settings/…）。
//! 命令清单的唯一事实源仍是 `src/ipc.rs::COMMANDS`，三处同步由 cargo test 闸门拦。

use crate::boot::{
    active_session_profile, ensure_switchable_profile, lib_boot_again, teardown_session, ShellState,
};
use std::sync::Arc;

use tauri::Manager;

/// Profile 管理器（4.3 只读刀）：列出全部 profile——已物化目录 + 未物化的
/// 内置模板名（web/headless）两态合并。纯读：零写入、零 dsh 子进程。
///
/// **世界择源（ADR-0016 §5 读侧下沉，2026-09-11 第三批）**：WSL 模式下扫的是
/// **客体** `~/.dsh/profiles`（客体列举 + 批量读清单），本地模式零变化。
/// 这条链是控制中心的着陆页、也是市场安装的目标 profile 选择器数据源——它被
/// P0 守卫挡住会让已下沉的插件链**不可达**，故从 P3 提前到本批。
/// 客体列目录/读清单是阻塞 IO（`wsl.exe` 往返）→ 走 spawn_blocking。
#[tauri::command]
pub async fn list_profiles(
    app: tauri::AppHandle,
) -> Result<Vec<crate::profiles::ProfileSummary>, String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => Ok(crate::profiles::scan_profiles(
            &crate::resolve::user_dsh_home(),
        )),
        crate::mgmt::World::Wsl { distro } => crate::profiles::scan_profiles_in_guest(&distro),
    })
    .await
    .map_err(|e| format!("profile 列表任务异常终止：{e}"))?
}
/// Profile 管理器（4.3 只读刀）：单个 profile 详情（package.json 关键字段 +
/// cordis.patch.yml 原文，不解析 YAML）。名字先过 dsh 同款校验（防路径遍历）。
/// **世界择源**同列表（ADR-0016 §5 读侧下沉）：WSL 模式读客体清单与 patch 原文。
#[tauri::command]
pub async fn get_profile_detail(
    app: tauri::AppHandle,
    profile: String,
) -> Result<crate::profiles::ProfileDetail, String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            crate::profiles::read_profile_detail(&crate::resolve::user_dsh_home(), &profile)
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::profiles::read_profile_detail_in_guest(&distro, &profile)
        }
    })
    .await
    .map_err(|e| format!("profile 详情任务异常终止：{e}"))?
}
/// Profile 管理器（4.3 创建刀）：spawn `dsh plugin --profile <名> install`
/// 半官方转发链创建 profile——dsh 首用 initProfile 写三件套（bundles 声明
/// 内置插件 dsh-base）→ `pnpm install` 空依赖零网络毫秒级；成功后壳对非模板
/// 名追加 web-app 单键声明（三件套写入例外 #2，ADR-0009 §4 第二次修订
/// 2026-08-28：创建即 webUi 候选，可设为默认启动；与出厂 web 模板同构）。
/// 阻塞动作（系统探测 + 转发链 + 声明补写）全部在 spawn_blocking——同步命令
/// 跑主线程会冻结 UI（setup 注释同源坑）。
///
/// P0 诚实兜底（ADR-0016 §5-e）：profile 创建属客体下沉的 P2（创建走客体 CLI，
/// Profile 管理器（4.3 创建刀）：spawn `dsh plugin --profile <名> install`
/// 半官方转发链创建 profile——dsh 首用 initProfile 写三件套（bundles 声明
/// 内置插件 dsh-base）→ `pnpm install` 空依赖零网络毫秒级；成功后壳对非模板
/// 名追加 web-app 单键声明（三件套写入例外 #2，ADR-0009 §4 第二次修订
/// 2026-08-28：创建即 webUi 候选，可设为默认启动；与出厂 web 模板同构）。
/// 阻塞动作（系统探测 + 转发链 + 声明补写）全部在 spawn_blocking——同步命令
/// 跑主线程会冻结 UI（setup 注释同源坑）。
///
/// **世界择源（ADR-0016 §4 P2，2026-09-11）**：WSL 模式在客体执行
/// `dsh plugin install` 转发链，客体写追加 web-app 声明，写单键 build policy。
#[tauri::command]
pub async fn create_profile(
    app: tauri::AppHandle,
    profile: String,
) -> Result<crate::profiles::CreateProfileOutcome, String> {
    let world = crate::mgmt::current_world(&app)?;
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("定位数据目录失败：{e}"))?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => crate::profiles::create_profile_blocking(&profile, &data_dir),
        crate::mgmt::World::Wsl { distro } => {
            crate::profiles::create_profile_in_guest(&distro, &profile, &data_dir)
        }
    })
    .await
    .map_err(|e| format!("创建任务异常终止：{e}"))?
}
/// Profile 管理器（4.3 生命周期刀）：复制 profile——整目录复制排除
/// node_modules + `name` 一致化改写（Spike B §3.2，红线 3 允许的两处
/// 三件套写入之一）。阻塞文件操作在 spawn_blocking。
/// **世界择源（ADR-0016 §4 P2）**：WSL 模式客体复制（排除 node_modules）并改写。
#[tauri::command]
pub async fn copy_profile(
    app: tauri::AppHandle,
    source: String,
    new_name: String,
) -> Result<crate::profiles::LifecycleOutcome, String> {
    let world = crate::mgmt::current_world(&app)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::profiles::copy_blocker(&home, &source, &new_name)?;
            let warnings = crate::profiles::copy_profile_tree(
                &home.join("profiles").join(&source),
                &home.join("profiles").join(&new_name),
                &new_name,
            )?;
            Ok(crate::profiles::LifecycleOutcome {
                profile: new_name,
                warnings,
            })
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::profiles::copy_profile_in_guest(&distro, &source, &new_name)
        }
    })
    .await
    .map_err(|e| format!("复制任务异常终止：{e}"))?
}
/// Profile 管理器（4.3 生命周期刀）：重命名——目录 rename + `name` 改写 +
/// 删 node_modules 让 dsh 自愈（Spike B §3.1）；运行中防护；defaultProfile
/// 引用同步旧名 → 新名（保持用户意图）。
/// **世界择源（ADR-0016 §4 P2）**：WSL 模式客体重命名目录 + 清理 node_modules + 改写清单。
#[tauri::command]
pub async fn rename_profile(
    app: tauri::AppHandle,
    old_name: String,
    new_name: String,
) -> Result<crate::profiles::LifecycleOutcome, String> {
    let world = crate::mgmt::current_world(&app)?;
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("定位数据目录失败：{e}"))?;
    let active = active_session_profile(&app);
    crate::profiles::running_conflict(active.as_deref(), &old_name)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            crate::profiles::rename_blocker(&home, &old_name, &new_name)?;
            let warnings = crate::profiles::rename_profile_dir(&home, &old_name, &new_name)?;
            // defaultProfile 引用同步（load-modify-save，防抹掉其他字段）
            let mut settings = crate::settings::load(&data_dir);
            if settings.default_profile.as_deref() == Some(old_name.as_str()) {
                settings.default_profile = Some(new_name.clone());
                crate::settings::save(&data_dir, &settings)
                    .map_err(|e| format!("同步默认 profile 失败：{e}"))?;
            }
            Ok(crate::profiles::LifecycleOutcome {
                profile: new_name,
                warnings,
            })
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::profiles::rename_profile_in_guest(&distro, &old_name, &new_name, &data_dir)
        }
    })
    .await
    .map_err(|e| format!("重命名任务异常终止：{e}"))?
}
/// Profile 管理器（4.3 生命周期刀）：删除——整目录删除，不级联 sessions
/// （dsh 明示）；运行中防护；defaultProfile 指向被删 profile → 清除（读取侧
/// 兜底 web，ADR-0009 §4）。node_modules 体量大，删除走 spawn_blocking。
/// **世界择源（ADR-0016 §4 P2）**：WSL 模式客体删除目录 + 壳端清除默认 profile 引用。
#[tauri::command]
pub async fn delete_profile(
    app: tauri::AppHandle,
    profile: String,
) -> Result<crate::profiles::DeleteOutcome, String> {
    let world = crate::mgmt::current_world(&app)?;
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("定位数据目录失败：{e}"))?;
    crate::profiles::validate_profile_name(&profile)?;
    let active = active_session_profile(&app);
    crate::profiles::running_conflict(active.as_deref(), &profile)?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            let home = crate::resolve::user_dsh_home();
            if !home.join("profiles").join(&profile).is_dir() {
                return Err(format!(
                    "profile「{profile}」不存在或尚未物化——无目录可删除"
                ));
            }
            crate::profiles::delete_profile_dir(&home, &profile)?;
            // 默认启动 profile 引用检查（Spike B §3.3/ADR-0009 §4）：指向被删
            // profile → 清除；None 读取侧即兜底 web
            let mut settings = crate::settings::load(&data_dir);
            let mut default_cleared = false;
            if settings.default_profile.as_deref() == Some(profile.as_str()) {
                settings.default_profile = None;
                crate::settings::save(&data_dir, &settings)
                    .map_err(|e| format!("回退默认 profile 失败：{e}"))?;
                default_cleared = true;
            }
            Ok(crate::profiles::DeleteOutcome {
                profile,
                default_cleared,
            })
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::profiles::delete_profile_in_guest(&distro, &profile, &data_dir)
        }
    })
    .await
    .map_err(|e| format!("删除任务异常终止：{e}"))?
}
/// Profile 管理器（4.3④）：设置默认启动 profile（持久化 settings.json
/// `defaultProfile`，第二最小面例外，AGENTS §6 已登记；None/失效值读取侧
/// 兜底 web）。
///
/// **世界择源**（ADR-0016 §5 读侧下沉）：候选校验按**当前世界**的 profile 列表
/// ——WSL 模式校验客体名单。默认档仍是壳侧单一设置，启动时按当前模式的候选过滤
/// （`consume_default_profile`），跨世界残留名字读取侧兜底，不会误启动。
#[tauri::command]
pub fn set_default_profile(app: tauri::AppHandle, profile: String) -> Result<(), String> {
    match crate::mgmt::current_world(&app)? {
        crate::mgmt::World::Local => {
            crate::profiles::ensure_default_candidate(&crate::resolve::user_dsh_home(), &profile)?
        }
        crate::mgmt::World::Wsl { distro } => {
            crate::profiles::ensure_default_candidate_in_guest(&distro, &profile)?
        }
    }
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let mut settings = crate::settings::load(&data_dir);
    settings.default_profile = Some(profile);
    crate::settings::save(&data_dir, &settings).map_err(|e| format!("保存默认 profile 失败：{e}"))
}
/// 读取默认启动 profile（None = 未设置，消费方兜底 web；前端展示当前值用）。
#[tauri::command]
pub fn get_default_profile(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    Ok(crate::settings::load(&data_dir).default_profile)
}
/// Profile 管理器「启动/切换」（4.3⑥，ADR-0009 §4 三次修订）：停当前会话 →
/// 以目标 profile 重启（重启语义，dsh 无运行时切换能力；主窗口回壳 boot 屏
/// 走既有进度，就绪自动进新工作台）。切换**不写** defaultProfile（唯一写入口
/// = 星标）；失败落错误卡，重试经 forced_profile 延续同一目标，不自动回滚。
/// 仅 webUi 候选（非 webUi 无工作台 URL 可导航）；bundle 快照档由 probe 内
/// 档位守卫忽略强制目标。WSL 模式同链路（guest 脚本已参数化）。
///
/// **世界择源（2026-09-11 第三批）**：webUi 候选名单按当前世界取（WSL = 客体
/// 名单）；切换本身走 forced_profile → 客体启动脚本（已参数化），两侧同源。
///
/// 2026-09-10（ADR-0014）交接化重构，返回交接意图（供控制中心立即起导轨）：
/// 1. 先领启动代际令牌——在途的旧启动线程就此作废（并发双击不再双 spawn）；
/// 2. 建交接意图（两窗贯穿状态，`get_boot_status` 暴露给刚重载的主窗口）；
/// 3. **先落幕布再杀进程**：主窗口若正停在工作台上，先给它盖上「正在重启…」，
///    用户不会看到一个已经死掉的工作台页面（旧顺序 = 先 teardown 再导航）；
/// 4. teardown 旧会话 → 带令牌回启动屏重启。
#[tauri::command]
pub fn switch_profile(app: tauri::AppHandle, profile: String) -> Result<serde_json::Value, String> {
    crate::profiles::validate_profile_name(&profile)?;
    // 世界择源（ADR-0016 §5 读侧下沉）：候选 = **当前世界**的 webUi profile 名单。
    // 此前这里恒读宿主 home——WSL 模式下会拿宿主名单去校验客体 profile（错世界），
    // 是 P1 时登记在案、随本批读侧下沉消掉的边界。
    let candidates = match crate::mgmt::current_world(&app)? {
        crate::mgmt::World::Local => {
            crate::resolve::list_web_ui_profiles(&crate::resolve::user_dsh_home())
        }
        crate::mgmt::World::Wsl { distro } => crate::profiles::web_ui_profiles_in_guest(&distro)?,
    };
    ensure_switchable_profile(&profile, &candidates)?;
    let state = app.state::<Arc<ShellState>>().inner().clone();
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    // 语义分叉（导轨文案/首启与重启不同款）：目标 == 当前会话占用 = 重启。
    let kind = match active_session_profile(&app) {
        Some(active) if active == profile => crate::boot::HandoffKind::Restart,
        Some(_) => crate::boot::HandoffKind::Switch,
        None => crate::boot::HandoffKind::Start,
    };
    // 1) 代际令牌：本次启动是唯一有效的一次（ADR-0014）。
    let token = state.begin_boot();
    // 2) 交接意图：两窗共用（含主窗口整文档重载后的补水）。
    let handoff = crate::boot::Handoff::begin(profile.clone(), kind, token);
    let handoff_json = handoff.snapshot_json(crate::boot::now_ms());
    *state.handoff.lock().unwrap() = Some(handoff.clone());
    // 3) 幕布：仅当主窗口此刻确实停在工作台上（壳页面不需要——React 自己画）。
    if crate::boot::workbench_page_visible(&state.window, &state) {
        crate::boot::show_handoff_curtain(&state.window, &handoff);
    }
    // 4) 停旧 → 带令牌重启。
    let _ = teardown_session(&state);
    *state.forced_profile.lock().unwrap() = Some(profile.clone());
    tracing::info!("切换 profile → {profile}（kind={kind:?}, gen={token}）");
    let handle = app.clone();
    std::thread::spawn(move || {
        // 先回壳 boot 屏再启动：事件总线模块加载期装配——晚挂监听吞首发
        // 遥测（AGENTS §4.3）；就绪后 run_executor_session 导航进新工作台。
        let shell_url = crate::ui::shell_app_url(&handle);
        let _ = state.window.navigate(shell_url);
        lib_boot_again(state, handle, data_dir, token);
    });
    Ok(handoff_json)
}
/// 当前会话占用的 profile（None = 无活跃会话）：管理器「运行中」徽标与切换
/// 确认文案的数据源（读侧，与删除/重命名防护同源 `active_session_profile`）。
#[tauri::command]
pub fn get_active_profile(app: tauri::AppHandle) -> Result<Option<String>, String> {
    Ok(active_session_profile(&app))
}
