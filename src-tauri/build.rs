fn main() {
    // 前端资产嵌入追踪：tauri-build 默认不跟随 ../ui 变更，
    // 缺省会导致 cargo tauri build 复用旧前端（2026-08-23 实测踩坑）。
    for f in [
        "../ui/index.html",
        "../ui/selector.html",
        "../ui/assets/app.css",
        "../ui/assets/dsh-logo.svg",
    ] {
        println!("cargo:rerun-if-changed={f}");
    }
    tauri_build::build()
}
