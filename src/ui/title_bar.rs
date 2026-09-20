use gpui::{div, prelude::*, px, rgb, svg, App, ClickEvent, IntoElement, Window};

use crate::platform::set_window_always_on_top;
use crate::ui::icons::{PIN_FILLED_PATH, PIN_OUTLINE_PATH};

/// Space the native macOS traffic lights are drawn over
const TRAFFIC_LIGHTS_WIDTH: f32 = 60.0;

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
        .child(div().w(px(TRAFFIC_LIGHTS_WIDTH)).h(px(22.0)))
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
