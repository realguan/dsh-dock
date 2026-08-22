//! dsh-desktop-shell —— ADR-0004 的「产品壳」：一个极小的 Tauri 桌面壳。
//!
//! 职责只有四件（见 docs/contract.md）：
//!   1. 读取 resources 里的 product.manifest.json（运行时契约）；
//!   2. spawn 快照内的 node + dsh（`--port 0`），从日志解析实际访问地址；
//!   3. 把主窗口 WebView 导航到 `http://127.0.0.1:<port>/`（同源回环，无鉴权——
//!      探针定论，见 dsh-launcher ADR-0004 开放问题 1）；
//!   4. 应用退出时优雅停止 dsh（SIGTERM → SIGKILL 兜底）。
//!
//! 刻意保持薄：没有 IPC、没有状态库、没有领域服务——壳是通用机制，产品是数据。
//! 任何变多变的逻辑都属于打包侧（启动器 packaging 服务）或快照本身，不进本仓库。

mod manifest;
mod shell;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tauri::{Manager, RunEvent};

/// 壳运行时状态：dsh 子进程 + 主窗口句柄（供后台线程导航与 Exit 时停止）。
struct ShellState {
    dsh: Mutex<Option<shell::DshProcess>>,
    window: tauri::WebviewWindow,
}

/// dsh 启动等待上限：超过即认为装配有问题，进错误页。
const BOOT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

/// 定位含 product.manifest.json 的资源根（dev/prod 布局差异见 setup 注释）。
fn resolve_resources_dir(app: &tauri::App) -> PathBuf {
    let runtime = app.path().resource_dir().ok().unwrap_or_default();
    // 生产（bundle）：Tauri v2 打包器保留相对 src-tauri 的路径前缀——
    // 配置 `"resources": ["resources/**"]` 时，文件实际落在 `<资源根>/resources/`
    // 下（打包 e2e 实测，2026-08-21）。优先探测嵌套布局。
    let bundled = runtime.join("resources");
    if bundled.join("product.manifest.json").is_file() {
        return bundled;
    }
    // 兼容平铺布局：不同 bundler 版本/配置可能把资源直接放在资源根。
    if runtime.join("product.manifest.json").is_file() {
        return runtime;
    }
    // dev 回退链（Windows 语义的 tauri-build 副本 → 源码树，本仓库开发常态）。
    let exe_res = app.path().executable_dir().ok().unwrap_or_default().join("resources");
    if exe_res.join("product.manifest.json").is_file() {
        return exe_res;
    }
    let src_res = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    if src_res.join("product.manifest.json").is_file() {
        return src_res;
    }
    // 全落空：返回运行时路径，让契约读取给出可行动错误（A6）。
    runtime
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 幂等初始化：测试/外部可能已设全局日志。
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    tauri::Builder::default()
        .setup(|app| {
            // 资源根解析（dev/prod 差异）：
            //   - 生产（bundle）：Tauri v2 保留相对 src-tauri 的路径前缀，
            //     `resources/**` 落在 `.app/Contents/Resources/resources/`；
            //   - dev（cargo run，macOS）：resource_dir() 指向不存在的 target/Resources，
            //     回退链：exe_dir/resources（tauri-build 的副本，Windows 语义）→
            //     CARGO_MANIFEST_DIR/resources（源码树，本仓库开发常态）。
            let resources_dir = resolve_resources_dir(app);
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;

            // 契约缺失/不兼容 → 直接在窗口里给出可行动错误（A6：就地呈现）。
            let window = app
                .get_webview_window("main")
                .expect("main 窗口应由 tauri.conf.json 创建");
            let manifest = match manifest::ProductManifest::load(&resources_dir.join("product.manifest.json"))
            {
                Ok(m) => m,
                Err(e) => {
                    tracing::error!("product.manifest 读取失败: {e}");
                    let _ = window.navigate(error_page(&format!(
                        "产品清单读取失败：{e}<br/>该安装包缺少装配快照，请从启动器重新打包后安装。"
                    )));
                    return Ok(());
                }
            };

            // 兜底副本解析（内置档）；极简档（无 fallback）在此过渡期给出明确文案，
            // 宿主解析链（system→download）在 ② 落地时替换本块。
            let fallback = match &manifest.fallback {
                Some(fb) => fb.clone(),
                None => {
                    let _ = window.navigate(error_page(
                        "本安装为极简档，未内置 dsh 兜底副本。<br/>终端将优先复用您机器上的官方 dsh；                         如未安装将实时下载（该解析链即将随版本提供），请稍后重试或先安装 dsh。",
                    ));
                    return Ok(());
                }
            };
            // spawn dsh（快速失败路径：零部件缺失 → 错误页）。
            let dsh = match shell::spawn_dsh(&fallback, &resources_dir, &data_dir) {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!("启动 dsh 失败: {e}");
                    let _ = window.navigate(error_page(&format!(
                        "启动 dsh 失败：{e}<br/>请从启动器重新打包包含完整快照的桌面版。"
                    )));
                    return Ok(());
                }
            };
            let log_path = dsh.log_path.clone();

            let state = Arc::new(ShellState {
                dsh: Mutex::new(Some(dsh)),
                window: window.clone(),
            });
            app.manage(state.clone());

            // 后台线程：等 dsh 报告 URL → 导航进 dsh UI；此后持续监护子进程。
            // 超时 / 子进程死亡 → 导航回错误页。
            let _ = std::thread::spawn(move || {
                match shell::detect_url(&log_path, BOOT_TIMEOUT) {
                    None => {
                        let detail = read_error_detail(&log_path);
                        let _ = state.window.navigate(error_page(&format!(
                            "dsh 未在预期时间内就绪{detail}"
                        )));
                    }
                    Some(raw) => {
                        match tauri::Url::parse(&raw) {
                            Ok(url) => {
                                tracing::info!("dsh 已就绪，进入 {url}");
                                let _ = state.window.navigate(url);
                            }
                            Err(e) => {
                                let _ = state.window.navigate(error_page(&format!(
                                    "dsh 报告了无效地址（{raw}）：{e}"
                                )));
                                return;
                            }
                        }
                        // 监护：产品壳与 dsh 同生命周期——dsh 崩溃即错误页。
                        loop {
                            std::thread::sleep(std::time::Duration::from_millis(500));
                            let exited = {
                                let mut guard = state.dsh.lock().unwrap();
                                guard
                                    .as_mut()
                                    .and_then(|p| p.child.try_wait().ok())
                                    .flatten()
                            };
                            if let Some(status) = exited {
                                let code = status.code().unwrap_or(-1);
                                let detail = read_error_detail(&log_path);
                                tracing::error!("dsh 异常退出 code={code}{detail}");
                                let _ = state.window.navigate(error_page(&format!(
                                    "dsh 进程已退出（code={code}）{detail}"
                                )));
                                return;
                            }
                        }
                    }
                }
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("构建 Tauri app 失败")
        .run(|app_handle, event| {
            // 应用退出 → 优雅停止 dsh（壳退 = dsh 停，同生命周期）。
            if let RunEvent::Exit = event {
                if let Some(state) = app_handle.try_state::<Arc<ShellState>>() {
                    if let Some(mut dsh) = state.dsh.lock().unwrap().take() {
                        let code = shell::stop_dsh(&mut dsh.child, std::time::Duration::from_secs(3));
                        tracing::info!("dsh 已停止（exit {code}）");
                    }
                }
            }
        });
}

/// 从日志提取崩溃原因摘要（首条顶层 Error 行，截断 200 字符），带 `<br/>` 前缀。
fn read_error_detail(log_path: &std::path::Path) -> String {
    let text = std::fs::read_to_string(log_path).unwrap_or_default();
    let line = text
        .lines()
        .find(|l| l.starts_with("Error:"))
        .map(|l| l.trim_start_matches("Error:").trim().to_string())
        .unwrap_or_default();
    if line.is_empty() {
        String::new()
    } else {
        let cut: String = line.chars().take(200).collect();
        let suffix = if line.chars().count() > 200 { "…" } else { "" };
        format!("<br/>错误摘要：{}{}", html_escape(&cut), suffix)
    }
}

/// 一个自包含的错误页（data: URL），错误就地呈现（ADR-0004 A6）。
fn error_page(msg: &str) -> tauri::Url {
    let html = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>无法启动 {}</title></head>\
         <body style=\"margin:0;height:100vh;display:flex;align-items:center;justify-content:center;\
         font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;background:#0b0e14;color:#e6edf3\">\
         <div style=\"max-width:560px;padding:32px\">\
         <div style=\"font-size:15px;font-weight:600;letter-spacing:.08em;color:#4d94ff\">DEEPSEEK HARNESS</div>\
         <h1 style=\"margin:14px 0 10px;font-size:22px\">请重新打包后安装</h1>\
         <p style=\"font-size:14px;line-height:1.7;color:#9aa7b4\">该桌面版以冻结快照方式分发。遇到启动问题时，\
         请回到 dsh 启动器，在装配好的工作台上重新执行「打包为桌面版」，再安装新版本。</p>\
         <pre style=\"margin-top:18px;padding:14px;border-radius:10px;background:#111722;font-size:13px;color:#ffb86b;\
         white-space:pre-wrap;word-break:break-word\">{}</pre>\
         </div></body></html>",
        html_escape("DeepSeek Harness"),
        html_escape(msg)
    );
    tauri::Url::parse(&format!("data:text/html;charset=utf-8,{}", url_encode(&html)))
        .expect("错误页 data URL 必可解析")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// 极简百分号编码（data: URL 用），保留 RFC3986 unreserved。
fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}
