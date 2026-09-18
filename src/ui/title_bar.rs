use gpui::{
    div, prelude::*, px, rgb, rgba, svg, App, ClickEvent, IntoElement, Window,
};

use crate::platform::{minimize_window, set_window_always_on_top, zoom_window};
use crate::ui::icons::{
    PIN_FILLED_PATH, PIN_OUTLINE_PATH, TRAFFIC_CLOSE_PATH, TRAFFIC_MIN_PATH, TRAFFIC_ZOOM_PATH,
};

pub fn render_title_bar(
    is_pinned: bool,
    on_toggle_pin: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id("title_bar")
        .flex()
        .items_center()
        .justify_between()
        .pt(px(12.0))
        .pb(px(8.0))
        .px(px(14.0))
        .bg(rgb(0xFFFFFF))
        .on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
            window.start_window_move();
        })
        .child(
            // Traffic lights cluster with macOS group hover effect
            div()
                .id("traffic_lights")
                .group("traffic_group")
                .flex()
                .items_center()
                .gap(px(7.0))
                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation();
                })
                .child(
                    // Close button (Red)
                    div()
                        .id("close_btn")
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(12.0))
                        .h(px(12.0))
                        .rounded_full()
                        .bg(rgb(0xFF5F56))
                        .border_1()
                        .border_color(rgba(0x0000001A))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0xE0443E)))
                        .on_click(|_, window, cx| {
                            window.remove_window();
                            cx.quit();
                        })
                        .child(
                            svg()
                                .path(TRAFFIC_CLOSE_PATH)
                                .size(px(6.0))
                                .opacity(0.0)
                                .group_hover("traffic_group", |s| s.opacity(1.0)),
                        ),
                )
                .child(
                    // Minimize button (Yellow)
                    div()
                        .id("min_btn")
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(12.0))
                        .h(px(12.0))
                        .rounded_full()
                        .bg(rgb(0xFFBD2E))
                        .border_1()
                        .border_color(rgba(0x0000001A))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0xDEA123)))
                        .on_click(|_, window, _| {
                            minimize_window(window);
                        })
                        .child(
                            svg()
                                .path(TRAFFIC_MIN_PATH)
                                .size(px(6.0))
                                .opacity(0.0)
                                .group_hover("traffic_group", |s| s.opacity(1.0)),
                        ),
                )
                .child(
                    // Zoom button (Green)
                    div()
                        .id("zoom_btn")
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(12.0))
                        .h(px(12.0))
                        .rounded_full()
                        .bg(rgb(0x27C93F))
                        .border_1()
                        .border_color(rgba(0x0000001A))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0x1AAB29)))
                        .on_click(|_, window, _| {
                            zoom_window(window);
                        })
                        .child(
                            svg()
                                .path(TRAFFIC_ZOOM_PATH)
                                .size(px(6.0))
                                .opacity(0.0)
                                .group_hover("traffic_group", |s| s.opacity(1.0)),
                        ),
                ),
        )
        .child(
            // Empty space between traffic lights and pin button
            div().flex_1().h(px(22.0)),
        )
        .child(
            // Pin button
            div()
                .id("pin_btn")
                .flex()
                .items_center()
                .justify_center()
                .w(px(22.0))
                .h(px(22.0))
                .cursor_pointer()
                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                    cx.stop_propagation();
                })
                .hover(|s| s.bg(rgb(0xF3F4F6)).rounded(px(6.0)))
                .on_click(move |event, window, cx| {
                    let new_pinned = !is_pinned;
                    set_window_always_on_top(window, new_pinned);
                    on_toggle_pin(event, window, cx);
                })
                .child(
                    if is_pinned {
                        svg()
                            .path(PIN_FILLED_PATH)
                            .size(px(13.0))
                            .text_color(rgb(0x1F2328))
                    } else {
                        svg()
                            .path(PIN_OUTLINE_PATH)
                            .size(px(13.0))
                            .text_color(rgb(0x9CA3AF))
                    },
                ),
        )
}
