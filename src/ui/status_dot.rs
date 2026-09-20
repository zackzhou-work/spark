use gpui::{div, prelude::*, px, rgb, AnyElement};

use crate::state::TaskState;

/// 呼吸动效相位由外部定时器推进，避免 GPUI 动画每帧重绘
pub fn render_status_dot(state: TaskState, breath_phase: f32) -> AnyElement {
    let base = div().w(px(6.0)).h(px(6.0)).rounded_full().flex_none();

    match state {
        TaskState::Running => base.bg(rgb(breath_color(breath_phase))).into_any_element(),
        TaskState::Waiting => base.bg(rgb(0xF59E0B)).into_any_element(),
        TaskState::Completed => base.bg(rgb(0x3B82F6)).into_any_element(),
    }
}

/// 余弦插值：#D1D5DB (209, 213, 219) ↔ #374151 (55, 65, 81)
fn breath_color(phase: f32) -> u32 {
    let factor = (phase * std::f32::consts::PI * 2.0).cos() * 0.5 + 0.5;
    let mix = |light: f32, dark: f32| (light * factor + dark * (1.0 - factor)) as u32;
    mix(209.0, 55.0) << 16 | mix(213.0, 65.0) << 8 | mix(219.0, 81.0)
}
