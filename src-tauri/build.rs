// 仅消费 COMMANDS;to_kebab 由 lib crate 测试消费,此处允许 dead。
#[allow(dead_code)]
#[path = "src/ipc.rs"]
mod ipc;

fn main() {
    // 前端资产嵌入追踪：tauri-build 默认不跟随 ../ui 变更，
    // 缺省会导致 cargo tauri build 复用旧前端（2026-08-23 实测踩坑）。
    for f in [
        "../ui/index.html",
        "../ui/mode.html",
        "../ui/selector.html",
        "../ui/assets/app.css",
        "../ui/assets/dsh-logo.svg",
    ] {
        println!("cargo:rerun-if-changed={f}");
    }
    // IPC 命令单一事实源变更须触发重跑，否则改 src/ipc.rs 不再生效
    // （2026-08-28 三处同步机器闸门；capabilities 一致性由 cargo test 拦）。
    println!("cargo:rerun-if-changed=src/ipc.rs");
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        // 应用自定义命令的 ACL：为每个命令自动生成 allow-<command> 权限。
        // 列表由 src/ipc.rs 的 COMMANDS 单一事实源生成（不再手写）——
        // dsh Web UI 是 remote origin（http://127.0.0.1），Tauri 2.11 规定
        // remote 上下文调用自定义命令必须经 capability 显式授权
        // （capabilities/default.json 里引用这里的 allow-* 权限），
        // 否则 IPC 被 ACL 拒绝（2026-08-25 外链打不开的根因）。
        tauri_build::AppManifest::new().commands(ipc::COMMANDS),
    ))
    .expect("tauri-build 失败");
}
