use raw_window_handle::{HasWindowHandle, RawWindowHandle};

/// 控制 macOS 窗口的置顶状态 (NSWindow.Level)
/// pinned: true -> NSFloatingWindowLevel (3)
/// pinned: false -> NSNormalWindowLevel (0)
#[allow(unexpected_cfgs)]
pub fn set_window_always_on_top(window: &gpui::Window, pinned: bool) {
    #[cfg(target_os = "macos")]
    unsafe {
        use cocoa::base::id;
        use objc::{msg_send, sel, sel_impl};

        if let Ok(handle) = HasWindowHandle::window_handle(window) {
            if let RawWindowHandle::AppKit(appkit) = handle.as_raw() {
                let ns_view = appkit.ns_view.as_ptr() as id;
                let ns_window: id = msg_send![ns_view, window];
                if !ns_window.is_null() {
                    let level: i64 = if pinned { 3 } else { 0 };
                    let _: () = msg_send![ns_window, setLevel: level];
                }
            }
        }
    }
}

#[allow(unexpected_cfgs)]
pub fn setup_window_resizable(window: &gpui::Window, min_w: f64, min_h: f64) {
    #[cfg(target_os = "macos")]
    unsafe {
        use cocoa::base::{id, YES};
        use objc::{msg_send, sel, sel_impl};

        if let Ok(handle) = HasWindowHandle::window_handle(window) {
            if let RawWindowHandle::AppKit(appkit) = handle.as_raw() {
                let ns_view = appkit.ns_view.as_ptr() as id;
                let ns_window: id = msg_send![ns_view, window];
                if !ns_window.is_null() {
                    let current_mask: usize = msg_send![ns_window, styleMask];
                    // NSWindowStyleMaskClosable (2) | NSWindowStyleMaskMiniaturizable (4) | NSWindowStyleMaskResizable (8)
                    let _: () = msg_send![ns_window, setStyleMask: current_mask | 2 | 4 | 8];

                    let min_size = cocoa::foundation::NSSize::new(min_w, min_h);
                    let _: () = msg_send![ns_window, setMinSize: min_size];

                    // 彻底隐藏并淡化 macOS 原生红绿灯按钮，避免与自定义红绿灯重叠出现“重影”
                    for button_type in 0..=3 {
                        let btn: id = msg_send![ns_window, standardWindowButton: button_type];
                        if !btn.is_null() {
                            let _: () = msg_send![btn, setHidden: YES];
                            let _: () = msg_send![btn, setAlphaValue: 0.0f64];
                        }
                    }
                }
            }
        }
    }
}

#[allow(unexpected_cfgs)]
pub fn minimize_window(window: &gpui::Window) {
    #[cfg(target_os = "macos")]
    unsafe {
        use cocoa::base::id;
        use objc::{msg_send, sel, sel_impl};

        if let Ok(handle) = HasWindowHandle::window_handle(window) {
            if let RawWindowHandle::AppKit(appkit) = handle.as_raw() {
                let ns_view = appkit.ns_view.as_ptr() as id;
                let ns_window: id = msg_send![ns_view, window];
                if !ns_window.is_null() {
                    let _: () = msg_send![ns_window, miniaturize: 0 as id];
                }
            }
        }
    }
}

#[allow(unexpected_cfgs)]
pub fn zoom_window(window: &gpui::Window) {
    #[cfg(target_os = "macos")]
    unsafe {
        use cocoa::base::id;
        use objc::{msg_send, sel, sel_impl};

        if let Ok(handle) = HasWindowHandle::window_handle(window) {
            if let RawWindowHandle::AppKit(appkit) = handle.as_raw() {
                let ns_view = appkit.ns_view.as_ptr() as id;
                let ns_window: id = msg_send![ns_view, window];
                if !ns_window.is_null() {
                    let _: () = msg_send![ns_window, zoom: 0 as id];
                }
            }
        }
    }
}

#[allow(unexpected_cfgs)]
pub fn hide_native_traffic_lights(window: &gpui::Window) {
    #[cfg(target_os = "macos")]
    unsafe {
        use cocoa::base::{id, YES};
        use objc::{msg_send, sel, sel_impl};

        if let Ok(handle) = HasWindowHandle::window_handle(window) {
            if let RawWindowHandle::AppKit(appkit) = handle.as_raw() {
                let ns_view = appkit.ns_view.as_ptr() as id;
                let ns_window: id = msg_send![ns_view, window];
                if !ns_window.is_null() {
                    for button_type in 0..=3 {
                        let btn: id = msg_send![ns_window, standardWindowButton: button_type];
                        if !btn.is_null() {
                            let _: () = msg_send![btn, setHidden: YES];
                            let _: () = msg_send![btn, setAlphaValue: 0.0f64];
                        }
                    }
                }
            }
        }
    }
}
