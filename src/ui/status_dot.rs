use std::time::Duration;
use gpui::{
    div, ease_in_out, prelude::*, px, rgb, Animation, AnimationExt as _, AnyElement,
};

use crate::state::TaskState;

pub fn render_status_dot(state: TaskState, anim_id: impl Into<gpui::ElementId>) -> AnyElement {
    let base = div()
        .w(px(6.0))
        .h(px(6.0))
        .rounded_full()
        .flex_none();

    match state {
        TaskState::Running => base
            .with_animation(
                anim_id,
                Animation::new(Duration::from_secs(2))
                    .repeat()
                    .with_easing(ease_in_out),
                |el, delta| {
                    // 2.0s 周期余弦插值：#D1D5DB (209, 213, 219) ↔ #374151 (55, 65, 81)
                    let factor = (delta * std::f32::consts::PI * 2.0).cos() * 0.5 + 0.5;
                    let r = (209.0 * factor + 55.0 * (1.0 - factor)) as u8;
                    let g = (213.0 * factor + 65.0 * (1.0 - factor)) as u8;
                    let b = (219.0 * factor + 81.0 * (1.0 - factor)) as u8;
                    el.bg(rgb((r as u32) << 16 | (g as u32) << 8 | (b as u32)))
                },
            )
            .into_any_element(),
        TaskState::Waiting => base.bg(rgb(0xEF4444)).into_any_element(),
        TaskState::Unread => base.bg(rgb(0xF59E0B)).into_any_element(),
        TaskState::Completed => base
            .border_1()
            .border_color(rgb(0xD1D5DB))
            .into_any_element(),
    }
}
