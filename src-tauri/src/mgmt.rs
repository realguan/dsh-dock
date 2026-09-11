//! mgmt.rs —— 管理面「世界择源」（ADR-0016 P1）。
//!
//! 控制中心（插件中心 / profile 管理器 / 会话维护 / 控制台）的每个入口都要先回答
//! 一个问题：**这次操作打在哪个世界？** 本地（宿主 fs + 宿主引擎）还是 WSL 客体
//! （客体 fs + 客体引擎）。ADR-0016 §2.6 是硬约束：呈现与改写的对象必须与**当前
//! 会话实际运行的世界**一致——"看起来在管、其实管错"比功能缺失更糟。
//!
//! ## 世界从哪来
//!
//! - 模式 = [`crate::ui::current_active_mode`]（会话内 `active_mode` → 已存
//!   `defaultMode` → Local）；
//! - 发行版 = [`ShellState::active_wsl_distro`]（本机 probe 成功后记录的**实际选中**
//!   值，ADR-0016 §4）——管理面不重新推导，否则"用哪个发行版"会有两个答案，可能打到
//!   与运行中会话不同的客体上。
//!
//! ## 绝不回落 Local（本模块立项根因）
//!
//! 模式是 WSL 而发行版未知（probe 未完成 / 尚未启动过 WSL 会话）时**返回错误**，
//! 而不是静默按本地世界执行——后者正是 ADR-0016 §1 记录的缺陷形态：控制中心在
//! 宿主世界里"假装"管住了客体（且宿主导擎按设计永不就绪，动作必然失败还给出与
//! ADR-0004 相矛盾的补救提示）。
//!
//! ## P0 诚实兜底（ADR-0016 §4/§5-e）
//!
//! 分期未下沉的动作（profile CRUD / 会话 / 控制台 / 插件开关与配置复制 …）在 WSL
//! 世界一律经 [`require_local`] 返回「暂不支持 + 替代路径」——**不依赖任何分期**，
//! 是 P0 交付物。禁止文案：任何形式的「请先启动应用完成引擎引导后重试」（与
//! ADR-0004「Windows 侧壳不触网」直接矛盾，重试是死路）；回归闸门见本文件测试。

use std::sync::Arc;

use tauri::Manager;

use crate::boot::ShellState;
use crate::settings::Mode;

/// 管理面操作的目标世界。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum World {
    /// 宿主世界：宿主 dsh home + 宿主引擎（现状路径，行为零变化）。
    Local,
    /// WSL 客体世界：客体 dsh home + 客体引擎；`distro` = 运行中会话实际选中的发行版。
    Wsl { distro: String },
}

/// 世界判定内核（纯函数，供单测；`windows` 由 `cfg!(windows)` 传入）。
///
/// 非 Windows 平台：WSL 不是可达状态（客体只存在于 Windows；`Mode::Wsl` 只可能来自
/// 手工改过的 `settings.json` 或被搬运的配置）——此时按本地世界执行，因为客体**不存在**，
/// 报"暂不支持"只会让唯一的本地世界不可用。
pub(crate) fn world_from(mode: Mode, distro: Option<&str>, windows: bool) -> Result<World, String> {
    if mode == Mode::Wsl && windows {
        return match distro {
            Some(d) if !d.is_empty() => Ok(World::Wsl {
                distro: d.to_string(),
            }),
            _ => Err(world_unresolved()),
        };
    }
    Ok(World::Local)
}

/// 世界无法确定（WSL 模式但无已选发行版）时的可行动错误。
fn world_unresolved() -> String {
    "无法确定当前管理世界：会话按 WSL 模式运行，但本次启动尚未选定发行版——\
     管理面必须与运行中的会话同源，不回落本地世界（ADR-0016 §2.6）。\
     请先在主窗口以 WSL 模式完成一次启动，再打开控制中心。"
        .to_string()
}

/// WSL 模式下「未下沉」动作的诚实兜底文案（原因 + 替代路径）。
///
/// 替代路径必须**真的可行**：WSL 终端里的 `dsh` 就是运行中会话用的那一个
/// （ADR-0016 §2.1：网络与 CLI 都在客体进程内），切换本地模式则回到现状路径。
pub(crate) fn unsupported_in_wsl(action: &str, distro: &str) -> String {
    format!(
        "「{action}」在 WSL 客体模式下暂不支持：本版本只下沉了插件装卸与插件清单\
         （ADR-0016 P1），当前操作世界为 {distro} 客体。\
         替代路径：在该发行版的终端里直接执行 dsh 命令（如 `dsh --profile <名> ...`），\
         或切回本地模式后再从控制中心操作。"
    )
}

/// 解析本次管理操作的目标世界（会话内模式 + 实际选中发行版）。
pub(crate) fn current_world(app: &tauri::AppHandle) -> Result<World, String> {
    let mode = crate::ui::current_active_mode(app);
    world_from(mode, active_wsl_distro(app).as_deref(), cfg!(windows))
}

/// 本次启动实际选中的 WSL 发行版（`ShellState` 记录值；未探测完成 = None）。
fn active_wsl_distro(app: &tauri::AppHandle) -> Option<String> {
    let state = app.try_state::<Arc<ShellState>>()?;
    let distro = state.active_wsl_distro.lock().ok()?.clone();
    distro
}

/// **P0 守卫**：未下沉到客体的管理动作在 WSL 世界一律拒绝执行（诚实可行动）。
///
/// 本地世界（含非 Windows 平台）零行为变化——调用点只多一次世界判定。
pub(crate) fn require_local(app: &tauri::AppHandle, action: &str) -> Result<(), String> {
    match current_world(app)? {
        World::Local => Ok(()),
        World::Wsl { distro } => Err(unsupported_in_wsl(action, &distro)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_mode_is_local_world_regardless_of_distro() {
        assert_eq!(
            world_from(Mode::Local, Some("Ubuntu"), true).unwrap(),
            World::Local
        );
        assert_eq!(world_from(Mode::Local, None, true).unwrap(), World::Local);
    }

    #[test]
    fn wsl_mode_on_windows_uses_the_recorded_distro() {
        assert_eq!(
            world_from(Mode::Wsl, Some("Ubuntu-24.04"), true).unwrap(),
            World::Wsl {
                distro: "Ubuntu-24.04".to_string()
            }
        );
    }

    /// **绝不回落 Local**：WSL 模式但发行版未知 → 错误，绝不静默按宿主世界执行。
    #[test]
    fn wsl_mode_without_distro_never_falls_back_to_local() {
        let err = world_from(Mode::Wsl, None, true).unwrap_err();
        assert!(err.contains("无法确定当前管理世界"), "{err}");
        // 空串（异常记录）与 None 同口径
        let err = world_from(Mode::Wsl, Some(""), true).unwrap_err();
        assert!(err.contains("无法确定当前管理世界"), "{err}");
    }

    /// 非 Windows：客体不存在，WSL 档不可达（只可能来自被搬运的手改配置）→ 本地世界可用。
    #[test]
    fn wsl_mode_is_local_on_non_windows() {
        assert_eq!(world_from(Mode::Wsl, None, false).unwrap(), World::Local);
        assert_eq!(
            world_from(Mode::Wsl, Some("Ubuntu"), false).unwrap(),
            World::Local
        );
    }

    #[test]
    fn unsupported_message_names_action_distro_and_alternative() {
        let msg = unsupported_in_wsl("创建 profile", "Ubuntu-24.04");
        assert!(msg.contains("创建 profile"), "{msg}");
        assert!(msg.contains("Ubuntu-24.04"), "{msg}");
        assert!(msg.contains("替代路径"), "{msg}");
    }

    /// **禁语回归闸门**（ADR-0016 §4）：诚实兜底文案绝不得包含与 ADR-0004 矛盾的
    /// 死路补救（"请先启动应用完成引擎引导后重试"）——WSL 模式下宿主引擎按设计
    /// 永不就绪，这句话指向的补救不可能成功。
    #[test]
    fn honest_fallbacks_never_advise_the_impossible_retry() {
        for msg in [unsupported_in_wsl("会话列表", "Ubuntu"), world_unresolved()] {
            assert!(
                !msg.contains("请先启动应用完成引擎引导后重试"),
                "诚实兜底文案不得含死路补救：{msg}"
            );
            assert!(
                !msg.contains("引擎未就绪"),
                "兜底文案不应再指向宿主引擎：{msg}"
            );
        }
    }
}
