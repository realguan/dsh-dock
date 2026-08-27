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
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            // 应用自定义命令的 ACL：为每个命令自动生成 allow-<command> 权限。
            // 必须显式列出——dsh Web UI 是 remote origin（http://127.0.0.1），
            // Tauri 2.11 规定 remote 上下文调用自定义命令必须经 capability 显式
            // 授权（capabilities/default.json 里引用这里的 allow-* 权限），
            // 否则 IPC 被 ACL 拒绝（2026-08-25 外链打不开的根因）。
            tauri_build::AppManifest::new().commands(&[
                "choose_profile",
                "terminal_action",
                "get_update_status",
                "check_updates",
                "get_client_update",
                "client_update_check",
                "client_update_apply",
                "open_external",
                "open_workbench_in_browser",
                "get_workbench_url",
                "boot_in_wsl",
                "choose_mode",
            ]),
        ),
    )
    .expect("tauri-build 失败");
}
