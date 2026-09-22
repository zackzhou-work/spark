use gpui::{div, ease_in_out, prelude::*, px, rgb, AnyElement};

use crate::state::TaskState;

/// 呼吸一个来回的时长
pub const BREATH_PERIOD_MS: u64 = 2000;

/// 灰度呼吸：#D1D5DB (209, 213, 219) ↔ #374151 (55, 65, 81)
fn breath_color(phase: f32) -> u32 {
    let factor = (ease_in_out(phase) * std::f32::consts::PI * 2.0).cos() * 0.5 + 0.5;
    let mix = |light: f32, dark: f32| (light * factor + dark * (1.0 - factor)) as u32;
    mix(209.0, 55.0) << 16 | mix(213.0, 65.0) << 8 | mix(219.0, 81.0)
}

/// phase 是呼吸周期内的位置（0.0 ~ 1.0），只有 Running 用得上
pub fn render_status_dot(state: TaskState, phase: f32) -> AnyElement {
    let base = div()
        .w(px(6.0))
        .h(px(6.0))
        .rounded_full()
        .flex_none();

    match state {
        TaskState::Running => base.bg(rgb(breath_color(phase))).into_any_element(),
        TaskState::Waiting => base.bg(rgb(0xEF4444)).into_any_element(),
        TaskState::Unread => base.bg(rgb(0xF59E0B)).into_any_element(),
        TaskState::Completed => base
            .border_1()
            .border_color(rgb(0xD1D5DB))
            .into_any_element(),
    }
}
