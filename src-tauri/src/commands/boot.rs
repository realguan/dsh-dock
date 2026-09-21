//! commands/boot.rs —— boot 域 IPC 命令处理层（2026-09-08 架构评审批次 2 从 lib.rs 拆出）。
//!
//! 本层只做「参数校验 + 数据目录定位 + 阻塞动作下沉 spawn_blocking」，
//! 业务实现全在对应域模块（profiles/plugins/sessions/settings/…）。
//! 命令清单的唯一事实源仍是 `src/ipc.rs::COMMANDS`，三处同步由 cargo test 闸门拦。

use crate::boot::{
    emit_boot_error, emit_step, emit_upgrade, executor_for_mode, launch_executor_after_probe,
    refresh_update_ui, run_executor_session, switch_mode, ShellState,
};
use std::sync::Arc;

use crate::settings;
use tauri::Manager;

/// 唯一 IPC 命令（②b profile 选择器）：选定 profile → 用 pending 的会话启动。
#[tauri::command]
pub fn choose_profile(app: tauri::AppHandle, profile: String) -> Result<(), String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    let mut executor = state
        .pending
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| "无待启动任务（请重新打开终端）".to_string())?;
    executor.select_profile(profile);
    let handle = app.clone();
    // 选择器落地也是一次新启动（ADR-0014 代际令牌）：领新令牌，作废在途启动线程。
    let token = state.begin_boot();
    // 后台线程启动（npm/下载动作不阻塞）
    std::thread::spawn(move || {
        if let Err(e) = run_executor_session(state, handle.clone(), executor, token) {
            tracing::error!("启动 DSH 失败: {e}");
        }
    });
    Ok(())
}
pub(crate) fn evaluate_needs_mode_selection(
    is_windows: bool,
    active_mode_is_none: bool,
    default_mode_is_none: bool,
) -> bool {
    is_windows && active_mode_is_none && default_mode_is_none
}

/// 读取启动阶段缓存的状态与错误（前端挂载时补水，解决 early emit 竞态丢失事件的问题）。
/// 2026-09-10（ADR-0014）：新增 `intent`（交接意图）——主窗口在切换/重启后是**整文档
/// 重载**，新文档只剩这一条通道能拿到「我是被谁重启的、从什么时候开始」，
/// 于是启动屏与幕布才能续上控制中心那条导轨与计时器。
#[tauri::command]
pub fn get_boot_status(app: tauri::AppHandle) -> Result<serde_json::Value, String> {
    if let Some(shell_state) = app.try_state::<Arc<ShellState>>() {
        // 语义（2026-09-11 收紧）：缓存由轮次起点 `begin_boot()` 清空，故这里返回的
        // 永远是**当前轮次**的可见状态——新一轮已开始但尚无新错误时必为 `null`，
        // 而不是上一轮的错误。曾因清缓存散落在 3 个调用点（漏 2 处）导致旧错误
        // 经本命令补水进新文档（v1.2.0 实测 2.1）。
        let error = shell_state.boot.error();
        let steps = shell_state.boot.steps();
        let data_dir = app.path().app_data_dir().ok();
        let needs_mode_selection = if cfg!(windows) {
            let active_is_none = shell_state.active_mode.lock().unwrap().is_none();
            let default_is_none = data_dir
                .as_deref()
                .map(crate::settings::load)
                .map(|s| s.default_mode.is_none())
                .unwrap_or(true);
            evaluate_needs_mode_selection(true, active_is_none, default_is_none)
        } else {
            false
        };
        Ok(serde_json::json!({
            "steps": steps,
            "error": error,
            "intent": shell_state.handoff_json(),
            "needs_mode_selection": needs_mode_selection,
            "needsModeSelection": needs_mode_selection,
        }))
    } else {
        Ok(serde_json::json!({
            "steps": [],
            "error": null,
            "intent": serde_json::Value::Null,
            "needs_mode_selection": false,
            "needsModeSelection": false,
        }))
    }
}
/// 「在 WSL 中打开」（顶栏入口；现已记默认 = 与菜单切换同语义）。
/// 非 Windows：WSL 不存在——防御性拒绝（前端该按钮本就不渲染，这里兜底防
/// 手工调用 / 旧页面缓存把会话 teardown 后写进脏默认）。
#[tauri::command]
pub fn boot_in_wsl(app: tauri::AppHandle) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("WSL 仅支持 Windows 平台。".to_string());
    }
    let state = app.state::<Arc<ShellState>>().inner().clone();
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let handle = app.clone();
    std::thread::spawn(move || {
        switch_mode(handle, state, settings::Mode::Wsl, data_dir);
    });
    Ok(())
}
/// 首次运行选择落地（壳运行环境页 → 写默认（可选）→ 按所选模式启动；
/// 2026-08-27 前端迁移后页面为 SPA /mode 路由，回跳主窗口经 React Router）。
#[tauri::command]
pub fn choose_mode(app: tauri::AppHandle, mode: String, set_default: bool) -> Result<(), String> {
    let m = settings::Mode::parse(&mode).ok_or_else(|| format!("未知运行环境：{mode}"))?;
    // 非 Windows：WSL 不存在（mode.html 本就不渲染 WSL 卡，这里兜底防旧页面缓存）。
    if m == settings::Mode::Wsl && !cfg!(windows) {
        return Err("WSL 仅支持 Windows 平台。".to_string());
    }
    let state = app.state::<Arc<ShellState>>().inner().clone();
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    // 选择落地同样是一次全新启动（ADR-0014）：领令牌作废在途启动线程。
    // 可见缓存（上一轮的错误/步骤）随 begin_boot 一并撤销——2026-09-11 前此处
    // 另有一句 clear_boot_cache()，与 begin_boot 合并后由轮次起点统一负责。
    // 位置刻意留在 data_dir 解析之后：解析失败时本轮尚未开始，不应作废在途启动。
    let token = state.begin_boot();
    *state.handoff.lock().unwrap() = None;
    if set_default {
        // load-modify-save：同 switch_mode，不得抹掉其他已存字段。
        let mut shell_settings = crate::settings::load(&data_dir);
        shell_settings.default_mode = Some(m);
        crate::settings::save(&data_dir, &shell_settings)
            .map_err(|e| format!("保存默认运行环境失败：{e}"))?;
    }
    *state.active_mode.lock().unwrap() = Some(m);
    let handle = app.clone();
    std::thread::spawn(move || match executor_for_mode(m, &handle, data_dir) {
        Ok(executor) => launch_executor_after_probe(state, handle, executor, token),
        Err(e) => emit_boot_error(&handle, &e, ""),
    });
    Ok(())
}
/// 安全模式状态（ADR-0026）：是否仍处于安全模式 + 此刻仍停用着的行（与配置实时联动）。
///
/// 只读、零副作用（读壳自有记账 + 查那份备份在不在）；**不读运行态**——那是回环快照的职责。
/// 前端用它渲染控制中心横幅（只说"停用了几个、去哪儿打开开关"；**没有恢复动作**）。
/// 安全模式在 WSL 客体档的诚实拒绝文案（**读侧与写侧同一句**，2026-09-21 平台审计 A9）。
fn wsl_safe_mode_unsupported() -> String {
    "安全模式暂不支持 WSL 客体档：需补客体侧 patch 写原语后方可启用（有意不回落宿主，以免改错 profile）。"
        .to_string()
}

/// 安全模式记账该用哪个 home（**纯函数**，三平台可测）。
///
/// 2026-09-21（平台审计 A9）修：读侧此前直接 `boot_target_home().unwrap_or_else(user_dsh_home)`，
/// 而 **WSL 执行器的 `dsh_home()` 返回 `None`**（`executor.rs:108-110`）⇒ 客体档下会拿
/// **宿主世界**的 home 去查安全模式记账，界面上显示出另一个世界的状态 —— 属红线 3 明禁的
/// 静默降级（"读错世界"比报错更坏）。此处与写侧（`terminal_action`）同口径：**显式拒绝**。
pub(crate) fn safe_mode_home(
    world: &crate::mgmt::World,
    boot_home: Option<std::path::PathBuf>,
    user_home: std::path::PathBuf,
) -> Result<std::path::PathBuf, String> {
    if let crate::mgmt::World::Wsl { .. } = world {
        return Err(wsl_safe_mode_unsupported());
    }
    Ok(boot_home.unwrap_or(user_home))
}

#[tauri::command]
pub async fn get_safe_mode_state(
    app: tauri::AppHandle,
    profile: String,
) -> Result<crate::safe_mode::SafeModeState, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    crate::profiles::validate_profile_name(&profile)?;
    // home 判定同 `terminal_action` 的口径：**本次启动实际用的那个**（缺记录时退用户 home），
    // 但客体档一律先拒绝（见 `safe_mode_home`）——绝不拿宿主 home 冒充客体档。
    let home = safe_mode_home(
        &crate::mgmt::current_world(&app)?,
        crate::boot::boot_target_home(&app),
        crate::resolve::user_dsh_home(),
    )?;
    Ok(crate::safe_mode::state(&data_dir, &profile, &home))
}

/// 关掉本轮的安全模式横幅（"不再提示"）：只写壳自有记账，**不碰 dsh 配置**。
///
/// 为什么要有它（2026-09-16 维护者裁定）：安全模式横幅只在"用户确实以安全模式进入过"时出现，
/// 且必须**可关闭**——用户看过一次就够了；同一轮再启动不打扰，**下一次进入安全模式会重新提示**
/// （新记账 = 新事件）。
#[tauri::command]
pub async fn dismiss_safe_mode_notice(
    app: tauri::AppHandle,
    profile: String,
) -> Result<bool, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    crate::profiles::validate_profile_name(&profile)?;
    let home = safe_mode_home(
        &crate::mgmt::current_world(&app)?,
        crate::boot::boot_target_home(&app),
        crate::resolve::user_dsh_home(),
    )?;
    crate::safe_mode::dismiss_notice(&data_dir, &profile, &home)
}

/// 错误卡动作（retry / upgrade）：重新解析并启动；upgrade 先升级全局 dsh。
/// upgrade_only：仅升级 + 刷新状态（不打断进行中的会话）。
/// `version`（2026-09-09 版本选择器）：upgrade / upgrade_only 的显式目标版本
/// （含 alpha 预览版，经版本列表选择进入）；None = 最新可接受版（稳定/rc）。
#[tauri::command]
pub fn terminal_action(
    app: tauri::AppHandle,
    action: String,
    version: Option<String>,
) -> Result<(), String> {
    let state = app.state::<Arc<ShellState>>().inner().clone();
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let handle = app.clone();
    std::thread::spawn(move || {
        // upgrade / upgrade_only：动用户全局 dsh（按钮确认即授权，动作在后台线程）。
        // 4.4⑤：全链路发 `dsh:upgrade` 事件（关于页升级按钮的真实反馈通道）——
        // 此前失败只发 boot:error，主窗口不在 boot 屏时用户完全看不到：
        // 2026-08-30 实测 0.1.2-alpha.2 依赖未完整发布，pnpm 两个 registry 均
        // 失败，错误无任何可见出口，用户视角 = 「点了没反应，等了也没升级」。
        let only = action == "upgrade_only";
        if only || action == "upgrade" {
            emit_upgrade(&handle, "running", "", false);
            let step_detail = match &version {
                Some(v) => format!("正在安装 DSH {v}…"),
                None => "正在升级官方 DSH 到最新稳定版…".to_string(),
            };
            emit_step(&handle, 2, "running", &step_detail);
            // 升级：世界感知（WSL 客体 vs 宿主引擎）。
            let world = crate::mgmt::current_world(&handle);
            let upgrade_result = match world {
                Ok(crate::mgmt::World::Wsl { distro }) => {
                    crate::executor::upgrade_guest_dsh(&distro, version.as_deref())
                        .map(crate::updates::UpgradePlan::Install)
                        .map_err(|e| anyhow::anyhow!("{e}"))
                }
                _ => {
                    let resources_dir = crate::ui::resolve_resources_dir(&handle);
                    let path_env = crate::resolve::effective_path();
                    crate::updates::upgrade_engine_dsh(
                        &data_dir,
                        &resources_dir,
                        &path_env,
                        version.as_deref(),
                    )
                }
            };
            match upgrade_result {
                Ok(plan) => {
                    let (installed_version, installed) = match &plan {
                        crate::updates::UpgradePlan::Install(v) => (v, true),
                        crate::updates::UpgradePlan::Skip(v) => (v, false),
                    };
                    let msg = if installed {
                        format!("DSH 已升级到 {installed_version}")
                    } else {
                        format!("DSH 已是最新（{installed_version}）")
                    };
                    emit_step(&handle, 2, "done", &msg);
                    // 刷新版本状态（托盘/前端 chip）
                    refresh_update_ui(&handle, &state);
                    emit_upgrade(&handle, "done", installed_version, installed);
                }
                Err(e) => {
                    tracing::error!(err = ?e, "dsh 升级失败");
                    emit_upgrade(&handle, "failed", &format!("{e:#}"), false);
                    emit_boot_error(&handle, &format!("升级失败：{e:#}"), "");
                    return;
                }
            }
            if only {
                let _ = handle;
                return;
            }
        }
        // ---- 安全模式（ADR-0026：写配置；恢复动作已按维护者第三次裁定移除）----
        // 两个动作都只动**用户自己的配置文件**（或读壳自有记账），随后一律走下面的
        // "重新解析链 + 启动"，因此失败不会留下半截状态。
        if matches!(action.as_str(), "safe_mode" | "safe_mode_reset") {
            // 用**启动目标**而不是会话槽：失败路径已 teardown，会话槽恒空（2026-09-16
            // 真机：拿不到 profile → 点了"像没反应"，且隔离按钮同时消失）。
            let profile = crate::boot::boot_target_profile(&handle);
            let Some(profile) = profile else {
                emit_boot_error(
                    &handle,
                    "安全模式需要一个已选定的 profile（当前查不到启动目标）——请先在启动页重选 profile。",
                    "",
                );
                return;
            };
            let world = match crate::mgmt::current_world(&handle) {
                Ok(w) => w,
                Err(e) => {
                    emit_boot_error(&handle, &format!("安全模式无法确定运行世界：{e}"), "");
                    return;
                }
            };
            if let crate::mgmt::World::Wsl { .. } = world {
                // 与实验能力目录/写行同口径：客体侧需补原语，宁可报错不回落宿主。
                emit_boot_error(&handle, &wsl_safe_mode_unsupported(), "");
                return;
            }
            // **改哪个 home 必须用本次启动的实际值**（2026-09-16 独立复核 P1）：快照档的
            // home 是 `<data_dir>/runtimes/fallback-home`，每次启动被重同步覆写；按用户 home
            // 写会改错文件（动用户的 `~/.dsh` 同名 profile）且不可能生效。故显式拒绝该档，
            // 口径同 WSL：宁可报错，不回落宿主。
            let user_home = crate::resolve::user_dsh_home();
            let home = crate::boot::boot_target_home(&handle).unwrap_or_else(|| user_home.clone());
            if home != user_home {
                emit_boot_error(
                    &handle,
                    &format!(
                        "安全模式暂不支持当前档位：本次启动用的工作区是 {}（快照档的 home 每次启动\
                         都会被重新同步覆盖，写进去不会生效）。为避免改错 profile，壳不回落用户 home。",
                        home.display()
                    ),
                    "",
                );
                return;
            }
            let outcome: Result<String, String> = match action.as_str() {
                // 进入：把**所有可停的三方挂载行**在配置文件里写成 `disabled: true`
                // （一次覆写、一次备份），然后正常启动——开关/徽标/下次启动同源。
                "safe_mode" => crate::plugins::row_attributions_blocking(
                    &profile, &data_dir, &world,
                )
                .and_then(|rows| {
                    // 层序：profile 层的停用桩停不到 **home 层**（更晚层）的行——如实区分，
                    // 否则会出现"报成功、再点一次改口早已停用"（2026-09-16 独立复核 P5）。
                    let (ids, unreachable) = crate::safe_mode::split_by_layer(
                        &rows,
                        &crate::safe_mode::profile_patch_path(&home, &profile),
                    );
                    let kept = rows.len() - ids.len() - unreachable.len();
                    // **无可停行**：如实报错、不空转启动。文案只陈述已知事实（0 条行时不能说
                    // "全部来自随包插件"）。
                    if ids.is_empty() {
                        let why = if rows.is_empty() {
                            "这个 profile 当前一条挂载行都没有".to_string()
                        } else if unreachable.is_empty() {
                            format!(
                                "这个 profile 的 {} 条挂载行全部来自随包插件，没有可停用的三方插件",
                                rows.len()
                            )
                        } else {
                            format!(
                                "可停用的行一条都没有，只有 {} 行够不到（不在本 profile 的 patch 层，\
                                 或在 home 层）：{}",
                                unreachable.len(),
                                unreachable.join("、")
                            )
                        };
                        return Err(format!(
                            "{why}——这类失败不是安全模式能解决的（本按钮未做任何改动，也没有启动）。"
                        ));
                    }
                    // 够不到的行必须说清（不能报"已全部停用"）。
                    let partial = if unreachable.is_empty() {
                        String::new()
                    } else {
                        format!(
                            "；另有 {} 行够不到（不在本 profile 的 patch 层，或在 home 层）：{}，未处理",
                            unreachable.len(),
                            unreachable.join("、")
                        )
                    };
                    crate::safe_mode::enter(&home, &data_dir, &profile, &ids).map(|outcome| {
                        if outcome.changed {
                            format!(
                                "已进入安全模式：在配置里停用 {} 行（保留随包 {} 行）{partial}；\
                                 配置已备份为 {}——想用哪个插件，到「实验能力」里打开哪个开关即可",
                                ids.len(),
                                kept,
                                outcome
                                    .backup
                                    .as_deref()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_default()
                            )
                        } else {
                            // 幂等：本次没改配置。**之前**进入过时记账仍在（横幅同屏可见），
                            // 故不能说"没有可恢复的备份"（2026-09-16 独立复核 P4）。
                            format!(
                                "这 {} 行早在停用态（本次未改动配置）{partial}；想用哪个插件，\
                                 到「实验能力」里打开哪个开关即可",
                                ids.len()
                            )
                        }
                    })
                }),
                // 兜底：配置写坏、行都枚举不出来时，备份 + 放空（前端须二次确认）。
                _ => crate::safe_mode::quarantine_patch(&home, &profile)
                    .map(|path| format!("已备份并放空 {}", path.display())),
            };
            match outcome {
                Ok(msg) => {
                    crate::boot::emit_step(&handle, 2, "running", &msg);
                    tracing::info!(action = %action, profile = %profile, "安全模式动作完成");
                }
                Err(e) => {
                    // 失败必须**换一张卡**（emit_boot_error 是替换语义），并在这张新卡上给出
                    // 唯一还能走的那一步：「备份并放空插件配置后启动」（2026-09-16 独立复核：
                    // 不给新卡指定动作，提示就是死指针）。
                    let payload = crate::boot_failure::BootErrorPayload::classify(
                        &format!(
                            "安全模式执行失败：{e}\n——若该 profile 的 cordis.patch.yml 语法已坏（连行都枚举不出来），\
                             下一步：备份并放空插件配置后再启动（会先留下 .bak-<时间戳> 备份）。"
                        ),
                        "",
                    )
                    .with_actions(&["safe_mode_reset"]);
                    crate::boot::emit_boot_error_payload(&handle, payload);
                    return;
                }
            }
        }
        // 重新走解析链 + 启动（重试同样领新令牌：作废在途的旧启动线程）
        let token = state.begin_boot();
        crate::boot::lib_boot_again(state, handle.clone(), data_dir, token);
        let _ = handle;
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_windows_never_needs_mode_selection() {
        assert!(!evaluate_needs_mode_selection(false, true, true));
        assert!(!evaluate_needs_mode_selection(false, false, true));
        assert!(!evaluate_needs_mode_selection(false, true, false));
        assert!(!evaluate_needs_mode_selection(false, false, false));
    }

    #[test]
    fn windows_needs_mode_selection_only_when_unselected_and_no_default() {
        // 首次启动且未设默认：需要模式选择
        assert!(evaluate_needs_mode_selection(true, true, true));
        // 已有默认模式：不需要模式选择
        assert!(!evaluate_needs_mode_selection(true, true, false));
        // 运行期已通过 choose_mode 选定模式（当前会话已激活）：不需要再次选择
        assert!(!evaluate_needs_mode_selection(true, false, true));
        // 既有默认又已激活：不需要
        assert!(!evaluate_needs_mode_selection(true, false, false));
    }
}

/// WSL 客体档安全模式「读错世界」的回归闸门（2026-09-21，平台审计 A9）。
#[cfg(test)]
mod safe_mode_world_tests {
    use super::safe_mode_home;
    use crate::mgmt::World;
    use std::path::PathBuf;

    fn user_home() -> PathBuf {
        PathBuf::from("/home/u/.dsh")
    }

    #[test]
    fn local_world_uses_the_boot_home_then_falls_back_to_user_home() {
        let boot = PathBuf::from("/data/runtimes/fallback-home");
        assert_eq!(
            safe_mode_home(&World::Local, Some(boot.clone()), user_home()).unwrap(),
            boot,
            "本地档必须用本次启动实际那个 home（快照档的 home 与用户 home 不是一个）"
        );
        assert_eq!(
            safe_mode_home(&World::Local, None, user_home()).unwrap(),
            user_home()
        );
    }

    /// **核心**：客体档必须拒绝，而不是回落宿主 home（旧行为 = 静默读错世界）。
    #[test]
    fn guest_world_refuses_instead_of_reading_the_host_home() {
        let err = safe_mode_home(
            &World::Wsl {
                distro: "Ubuntu-24.04".to_string(),
            },
            None, // WSL 执行器 dsh_home() 恒为 None —— 旧写法正是从这里回落宿主
            user_home(),
        )
        .expect_err("客体档不得返回宿主 home");
        assert!(err.contains("WSL 客体档"), "{err}");
        assert!(err.contains("不回落宿主"), "必须说明为什么不回落：{err}");
    }
}
