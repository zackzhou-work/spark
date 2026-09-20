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
