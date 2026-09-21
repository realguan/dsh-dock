//! macOS 红绿灯定位（ADR-0029 配套，2026-09-21）。
//!
//! 为什么需要它：官方 Electron 客户端显式设 `trafficLightPosition: { x: 16, y: 18 }`
//! （`apps/desktop/src/main.ts:127`，2026-09-21 源锚）；Tauri 的
//! `TitleBarStyle::Overlay` 只把红绿灯保留在**系统默认位**（原生探针实测 origin
//! (9,9)、按钮 14×14，垂直中心偏上），而 dsh 的桌面 CSS（`SidebarRoot.tsx` 的
//! topStrip「与红绿灯共行」、`ConversationRoot` 的 titleRow）是按官方 x16/y18
//! 设计的——默认位下红绿灯比顶栏条带视觉中心高约 10px（维护者截图比对官方效果后
//! 定位到此问题）。
//!
//! Tauri 无原生 API 调红绿灯位置，只能走 AppKit：取 `NSWindow` 的三个标准按钮
//! （关闭/最小化/全屏）逐个 `setFrameOrigin:`。这里用**裸 msg_send**（objc2）而非
//! objc2-app-kit 的类型化封装——只为三个 selector，引一整个 app-kit 生成面不划算；
//! `objc2` 本身已在依赖树里（tauri → objc2 0.6.4，Cargo.lock 在册），此处只是把
//! 既有传递依赖提升为直接依赖，不引入新 crate。
//!
//! **三个实测结论（2026-09-21 原生探针 /tmp/tltest，勿凭直觉"优化"）**：
//! 1. **坐标系是顶部原点**：标准按钮的 frame 在 theme frame（flipped 视图）里，
//!    y 从**窗口顶部**往下量——与 Electron `trafficLightPosition` 语义逐字一致，
//!    所以 `setFrameOrigin(16, 18)` 无需换算。默认位实测 (9, 9)。
//! 2. **三个按钮必须各设各的原点**：`setFrameOrigin:` 是绝对定位，给三个按钮设同一个
//!    (16,18) 会把它们叠成一坨（实测确认）。官方/Electron 的"一个定位点"内部按系统
//!    间距（实测 23px）展开——壳侧按各按钮**默认 x 相对 close 的偏移**平移，间距随
//!    系统版本自适应，不写死常数。
//! 3. **落位不是一次性的**——探针续测：`makeKeyAndOrderFront`（show）**不**重置按钮，
//!    但 `setFrame:`（resize）会把三个按钮**全部弹回默认位 (9,9)**；且 reset 可能发生在
//!    **不发 Tauri 窗口事件**的时机（tao 建窗后的内部 setFrame、vibrancy 视图插入）。
//!    故 `ui.rs` 三管齐下：① 任何窗口事件都重放；② setup 结束后经主线程事件循环补一
//!    枪；③ 本函数**幂等 + 变化检测**（已就位不写不打日志）——挂全量事件也不刷屏。
//!    首次落位打 info 并读回帧坐标，实机核对以该日志为凭据。
//!
//! 平台语义：仅 macOS 编译/执行；其他平台是 no-op（窗口装饰原生，无红绿灯浮层概念）。

#[cfg(target_os = "macos")]
mod imp {
    use objc2::runtime::AnyObject;
    use objc2::{msg_send, Encode, Encoding};
    use std::sync::atomic::{AtomicBool, Ordering};

    /// 首次落位记 info（带读回值），其后重放只记 debug（resize 高频，禁刷屏）。
    static FIRST_APPLY: AtomicBool = AtomicBool::new(true);

    /// 与官方 Electron `trafficLightPosition` 逐值一致（2026-09-21 锚定
    /// `apps/desktop/src/main.ts:127`）。改值 = 与官方效果分叉，须有依据。
    pub const TRAFFIC_LIGHT_X: f64 = 16.0;

    /// 官方 Electron 在 `apps/desktop/src/main.ts:127` 设置的 margin.y = 18.0。
    /// Electron 的 WindowButtonsProxy 据此计算 container 高度（14 + 2 * 18 ≈ 50~52px）。
    #[allow(dead_code)]
    pub const TRAFFIC_LIGHT_Y: f64 = 18.0;

    /// DSH 侧边栏顶部工具条（`.topStrip`）的官方高度（52px，Figma 与
    /// `SidebarRoot.module.css:185` 源锚）。
    /// Electron 的 `WindowButtonsProxy` 会将 `NSTitlebarContainerView` 高度
    /// 扩充为 52px，从而使居中的红绿灯与 `.topStrip`（`align-items: center`）
    /// 里的侧边栏收起按钮 `◫` 完美处于同一水平基准线（垂直中心距顶 26px）。
    pub const TITLEBAR_HEIGHT: f64 = 52.0;

    // ---- FFI 结构（Objective-C 编码：@encode(NSPoint) = {CGPoint=dd} 等）----

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NSPoint {
        x: f64,
        y: f64,
    }

    unsafe impl Encode for NSPoint {
        const ENCODING: Encoding = Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NSSize {
        w: f64,
        h: f64,
    }

    unsafe impl Encode for NSSize {
        const ENCODING: Encoding = Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]);
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct NSRect {
        origin: NSPoint,
        size: NSSize,
    }

    unsafe impl Encode for NSRect {
        const ENCODING: Encoding =
            Encoding::Struct("CGRect", &[NSPoint::ENCODING, NSSize::ENCODING]);
    }

    unsafe fn frame(obj: *mut AnyObject) -> NSRect {
        msg_send![obj, frame]
    }

    unsafe fn standard_button(window: *mut AnyObject, index: usize) -> *mut AnyObject {
        msg_send![window, standardWindowButton: index]
    }

    /// 把主窗口的三个红绿灯挪到官方同款位置，并与侧边栏收起按钮处于同一水平线上：
    /// 1. 扩充 `NSTitlebarContainerView` 与 `NSTitlebarView` 高度为 `TITLEBAR_HEIGHT`（52px）；
    /// 2. 在 52px 容器内将按钮垂直居中（Y = (52 - 14)/2 = 19px），使按钮垂直中心距顶
    ///    恰为 26px，与 DSH `.topStrip`（height: 52px; align-items: center）里的
    ///    收起按钮 `◫` 完美处于同一水平基准线！
    /// 3. X 坐标以 `TRAFFIC_LIGHT_X`（16px）为起点，保持系统原生按钮间距（~23px）。
    ///
    /// 调用时机：`create_main_window` 建窗后 + 任何窗口事件重放。函数幂等且带变化检测。
    pub fn align(window: &tauri::WebviewWindow) {
        let handle = match window.ns_window() {
            Ok(handle) if !handle.is_null() => handle,
            Ok(_) => {
                tracing::debug!("红绿灯定位跳过：ns_window 为空指针");
                return;
            }
            Err(e) => {
                tracing::debug!("红绿灯定位跳过：取不到 ns_window（{e}）");
                return;
            }
        };
        let ns_window: *mut AnyObject = handle.cast::<AnyObject>();
        unsafe {
            let close = standard_button(ns_window, 0);
            if close.is_null() {
                tracing::debug!("红绿灯定位跳过：取不到 close 标准按钮");
                return;
            }
            let mini = standard_button(ns_window, 1);
            let zoom = standard_button(ns_window, 2);

            let titlebar_view: *mut AnyObject = msg_send![close, superview];
            if titlebar_view.is_null() {
                return;
            }
            let container_view: *mut AnyObject = msg_send![titlebar_view, superview];
            if container_view.is_null() {
                return;
            }

            let win_frame: NSRect = msg_send![ns_window, frame];
            let win_height = win_frame.size.h;

            // 1. 将 NSTitlebarContainerView 扩充到 52px（与 DSH .topStrip 52px 顶栏高度一致）
            let mut c_frame: NSRect = msg_send![container_view, frame];
            let target_c_y = win_height - TITLEBAR_HEIGHT;
            let container_needs_update = (c_frame.size.h - TITLEBAR_HEIGHT).abs() > 0.5
                || (c_frame.origin.y - target_c_y).abs() > 0.5;

            if container_needs_update {
                c_frame.size.h = TITLEBAR_HEIGHT;
                c_frame.origin.y = target_c_y;
                let _: () = msg_send![container_view, setFrame: c_frame];

                let mut t_frame: NSRect = msg_send![titlebar_view, frame];
                t_frame.size.h = TITLEBAR_HEIGHT;
                t_frame.origin.y = 0.0;
                let _: () = msg_send![titlebar_view, setFrame: t_frame];
            }

            // 2. 垂直居中计算：在 52px 容器内，14px 按钮在 (52 - 14)/2 = 19px，
            // 距离窗口顶沿为 52 - 19 - 14 = 19px，中心距离窗口顶沿恰为 52/2 = 26px，
            // 与 .topStrip（height: 52px; align-items: center）里的侧边栏收起按钮处于同一水平线上！
            let raw_h = frame(close).size.h;
            let button_height = if raw_h > 5.0 { raw_h } else { 14.0 };
            let button_y = ((TITLEBAR_HEIGHT - button_height) / 2.0).round();

            // 3. 水平间距计算：提取 mini 与 close 的间距（默认约 23px）
            let spacing = if !mini.is_null() {
                let s = (frame(mini).origin.x - frame(close).origin.x).abs();
                if (15.0..35.0).contains(&s) {
                    s
                } else {
                    23.0
                }
            } else {
                23.0
            };

            let buttons = [close, mini, zoom];
            let mut landed = String::new();
            let mut moved = 0usize;

            for (i, &button) in buttons.iter().enumerate() {
                if button.is_null() {
                    continue;
                }
                let current = frame(button);
                let target = NSPoint {
                    x: TRAFFIC_LIGHT_X + (i as f64) * spacing,
                    y: button_y,
                };
                if (current.origin.x - target.x).abs() < 0.5
                    && (current.origin.y - target.y).abs() < 0.5
                    && !container_needs_update
                {
                    continue;
                }
                let _: () = msg_send![button, setFrameOrigin: target];
                let after = frame(button);
                landed += &format!("#{i}=({:.1},{:.1}) ", after.origin.x, after.origin.y);
                moved += 1;
            }

            if moved == 0 && !container_needs_update {
                return; // 全部就位：静默
            }

            // 首次落位打 info（带读回值，实机核对"到底生效没有"以该行为凭据）；
            // 其后重放打 debug。
            if FIRST_APPLY.swap(false, Ordering::Relaxed) {
                tracing::info!(
                    "红绿灯已定位 {landed}（高度={TITLEBAR_HEIGHT}，与侧边栏收起按钮水平对齐）"
                );
            } else {
                tracing::debug!("红绿灯重放 {landed}");
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    /// 非 macOS：窗口装饰原生、无红绿灯浮层概念，no-op（保留调用点三平台同构）。
    pub fn align(_window: &tauri::WebviewWindow) {}
}

pub(crate) use imp::align;

#[cfg(test)]
mod tests {
    /// 常量必须与官方 Electron `trafficLightPosition` 逐值一致——这里是唯一事实源，
    /// 改动即分叉（ADR-0029 §3：对标而非自造）。
    #[cfg(target_os = "macos")]
    #[test]
    fn traffic_light_position_matches_official_client() {
        assert_eq!(super::imp::TRAFFIC_LIGHT_X, 16.0);
        assert_eq!(super::imp::TRAFFIC_LIGHT_Y, 18.0);
        assert_eq!(super::imp::TITLEBAR_HEIGHT, 52.0);
    }
}
