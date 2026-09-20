use std::time::Duration;
use gpui::{
    div, ease_in_out, prelude::*, px, rgb, Animation, AnimationExt as _, AnyElement, ClickEvent,
    ElementId, FontWeight, IntoElement, SharedString, Window,
};

use crate::state::TimeFilter;

const THUMB_RADIUS: f32 = 5.0;

pub fn render_filter_bar(
    current_filter: TimeFilter,
    prev_filter: Option<TimeFilter>,
    total_count: usize,
    on_select_today: impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    on_select_all: impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let item_w = 48.0;

    let thumb: AnyElement = match (prev_filter, current_filter) {
        (Some(TimeFilter::Today), TimeFilter::All) => {
            // Smoothly slide from Today (2.0) to All (50.0)
            div()
                .id("sliding_thumb_to_all")
                .absolute()
                .top(px(2.0))
                .w(px(item_w))
                .h(px(20.0))
                .rounded(px(THUMB_RADIUS))
                .bg(rgb(0xFFFFFF))
                .shadow_sm()
                .with_animation(
                    "slide_thumb_all",
                    Animation::new(Duration::from_millis(180)).with_easing(ease_in_out),
                    move |el, delta| {
                        let x = 2.0 + delta * item_w;
                        el.left(px(x))
                    },
                )
                .into_any_element()
        }
        (Some(TimeFilter::All), TimeFilter::Today) => {
            // Smoothly slide from All (50.0) to Today (2.0)
            div()
                .id("sliding_thumb_to_today")
                .absolute()
                .top(px(2.0))
                .w(px(item_w))
                .h(px(20.0))
                .rounded(px(THUMB_RADIUS))
                .bg(rgb(0xFFFFFF))
                .shadow_sm()
                .with_animation(
                    "slide_thumb_today",
                    Animation::new(Duration::from_millis(180)).with_easing(ease_in_out),
                    move |el, delta| {
                        let x = (2.0 + item_w) - delta * item_w;
                        el.left(px(x))
                    },
                )
                .into_any_element()
        }
        _ => {
            let left = if current_filter == TimeFilter::Today {
                2.0
            } else {
                2.0 + item_w
            };
            div()
                .id("static_thumb")
                .absolute()
                .top(px(2.0))
                .left(px(left))
                .w(px(item_w))
                .h(px(20.0))
                .rounded(px(THUMB_RADIUS))
                .bg(rgb(0xFFFFFF))
                .shadow_sm()
                .into_any_element()
        }
    };

    div()
        .id("filter_bar")
        .flex()
        .items_center()
        .justify_between()
        .px(px(14.0))
        .pt(px(1.0))
        .pb(px(6.0))
        .child(
            // Segmented pill control (matching user screenshot media_1789698927524.png)
            div()
                .id("segmented_control")
                .relative()
                .flex()
                .items_center()
                .p(px(2.0))
                .bg(rgb(0xEDEDED))
                .rounded(px(THUMB_RADIUS + 2.0))
                .child(thumb)
                .child(render_segment_label(
                    "Today",
                    current_filter == TimeFilter::Today,
                    item_w,
                    on_select_today,
                ))
                .child(render_segment_label(
                    "All",
                    current_filter == TimeFilter::All,
                    item_w,
                    on_select_all,
                )),
        )
        .child(render_count_badge(total_count))
}

fn render_count_badge(count: usize) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .min_w(px(20.0))
        .h(px(18.0))
        .px(px(6.0))
        .rounded_full()
        .bg(rgb(0xEDEDED))
        .text_size(px(10.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgb(0x6B7280))
        .child(count.to_string())
}

fn render_segment_label(
    label: &'static str,
    is_active: bool,
    width: f32,
    on_click_handler: impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let base = div()
        .id(ElementId::Name(SharedString::from(format!("seg_{}", label))))
        .flex()
        .items_center()
        .justify_center()
        .w(px(width))
        .h(px(20.0))
        .rounded(px(THUMB_RADIUS))
        .cursor_pointer()
        .text_size(px(11.0))
        .on_click(move |event, window, cx| {
            on_click_handler(event, window, cx);
        });

    if is_active {
        base
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(0x111827))
            .child(label)
    } else {
        base
            .font_weight(FontWeight::NORMAL)
            .text_color(rgb(0x6B7280))
            .hover(|s| s.text_color(rgb(0x111827)))
            .child(label)
    }
}
