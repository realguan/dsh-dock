// Windows：壳永远不附加控制台窗口——release 与 debug 构建都要，否则
// `cargo run` 的调试版也会给用户/开发者弹一个黑色终端（2026-08-24 实测）。
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    dsh_dock_lib::run();
}
