//! commands/ssh.rs —— SSH 远程工作区向导的后端命令（2026-09-15，ADR-0023）。
//!
//! 本文件只做「参数校验 + 阻塞动作下沉 `spawn_blocking`」；解析与校验的实现在
//! `crate::ssh_config`（纯函数，可单测）。
//!
//! **范围限定**（ADR-0023 §1.3/§2.8）：向导面向 **headless / 自建 profile**。
//! 把 ssh 家族装进 `web` profile **不会**让 Web 工作台的文件树/编辑器/终端变成远端感知
//! ——上游明写 "replacing providers alone does **not** make those views remote-aware"。
//! 因此本命令与生成面都不得对 web profile 承诺远端开发模式。

use tauri::Manager;

/// 列出 `~/.ssh/config` 里**可选**的 SSH 主机（ADR-0023 §2.6：解析必须在 Rust 后端）。
///
/// 只读 `~/.ssh/config` 一个文件、**不跟随 `Include`**、只回传非机密字段；
/// 降级情况（未跟随的 Include / 忽略的 Match / 畸形行）经 `notes` 如实透出。
#[tauri::command]
pub async fn list_ssh_hosts() -> Result<crate::ssh_config::SshHosts, String> {
    tauri::async_runtime::spawn_blocking(crate::ssh_config::load_ssh_hosts)
        .await
        .map_err(|e| format!("读取 SSH 配置任务异常终止：{e}"))?
}

/// 非交互预检一个 SSH 目标（ADR-0023 §2.5）。
///
/// 一次 `ssh -o BatchMode=yes` 往返，回读远端 uname / node / helper / 摘要 / workspace。
/// **预检不通过即不得生成 profile**——生成一个注定启动失败的 profile 比不生成更糟。
///
/// 整轮 30s 上限：`BatchMode` 下不会有交互提示，卡住只可能是网络或远端 shell。
#[tauri::command]
pub async fn probe_ssh_target(
    target: crate::ssh_remote::SshTarget,
) -> Result<crate::ssh_remote::SshProbe, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::ssh_remote::probe_ssh_target(&target, std::time::Duration::from_secs(30))
    })
    .await
    .map_err(|e| format!("SSH 预检任务异常终止：{e}"))?
}

/// 生成 SSH 远程工作区 profile（ADR-0023 §2.2/§2.4/§2.8）。
///
/// 一次调用做四件事（顺序是硬约束，见 `ssh_profile::generate` 的注释）：
/// 建 profile（如缺）→ 装四包（**钉运行期版本**）→ 写四行 → 写后自证。
///
/// 四包安装**刻意放在后端**而非复用前端队列：① 挂载行必须在四包装完之后才写
/// （否则失败会留下指向未安装包的幽灵挂载行）；② 钉版本要运行期版本，只有后端有。
#[tauri::command]
pub async fn generate_ssh_profile(
    app: tauri::AppHandle,
    profile: String,
    target: crate::ssh_remote::SshTarget,
) -> Result<crate::ssh_profile::SshProfileOutcome, String> {
    let world = crate::mgmt::current_world(&app)?;
    let data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("定位数据目录失败：{e}"))?;
    tauri::async_runtime::spawn_blocking(move || match world {
        crate::mgmt::World::Local => {
            crate::ssh_profile::generate(&profile, &data_dir, &target, &world)
        }
        crate::mgmt::World::Wsl { .. } => Err(
            "SSH 远程工作区暂不支持 WSL 客体档：向导读的是**宿主**的 ~/.ssh/config，\
             客体档语义不等价（有意不回落宿主）。请在本地档使用。"
                .to_string(),
        ),
    })
    .await
    .map_err(|e| format!("生成 SSH profile 任务异常终止：{e}"))?
}
