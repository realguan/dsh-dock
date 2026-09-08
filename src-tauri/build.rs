// 仅消费 COMMANDS;to_kebab 由 lib crate 测试消费,此处允许 dead。
#[allow(dead_code)]
#[path = "src/ipc.rs"]
mod ipc;

fn main() {
    // 前端资产嵌入追踪：**已由 tauri-build 自管**——`frontendDist`（tauri.conf.json
    // 指向 ../frontend/dist）与 `capabilities/` 的 rerun 指令由 tauri-build 自己发出
    // （tauri-build 2.6.3 `src/codegen/context.rs:87-95`、`src/acl.rs:427`）。
    // 2026-09-08 删除 5 条手写 `../ui/*` 监视：`ui/` 目录早已不存在，这些行是
    // 2026-08-23 旧前端复用事故留下的惰性守卫，现由上游覆盖，留着只会误导。
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
