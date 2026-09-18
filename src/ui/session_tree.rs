use gpui::{
    div, prelude::*, px, rgb, svg, FontWeight, IntoElement, SharedString,
};

use crate::platform::focus_session_window;
use crate::state::ProjectGroup;
use crate::ui::icons::FOLDER_PATH;
use crate::ui::status_dot::render_status_dot;

pub fn render_session_tree(groups: &[ProjectGroup], id_prefix: &str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .px(px(8.0))
        .pt(px(2.0))
        .pb(px(10.0))
        .gap(px(10.0))
        .children(groups.iter().map(|group| {
            let project_name = group.project_name.clone();
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    // Project Group Header
                    div()
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .px(px(6.0))
                        .py(px(2.0))
                        .child(
                            svg()
                                .path(FOLDER_PATH)
                                .size(px(12.0))
                                .text_color(rgb(0x9CA3AF)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .overflow_hidden()
                                .truncate()
                                .text_size(px(11.0))
                                .font_weight(FontWeight::NORMAL)
                                .text_color(rgb(0x9CA3AF))
                                .child(project_name),
                        ),
                )
                .children(group.sessions.iter().map(|session| {
                    let pid = session.pid;
                    let title = session.title.clone();
                    let state = session.state;
                    let session_id_str = session.id.clone();
                    let anim_id = SharedString::from(format!("{}_dot_{}", id_prefix, session_id_str));

                    div()
                        .id(SharedString::from(format!("{}_row_{}", id_prefix, session_id_str)))
                        .flex()
                        .items_center()
                        .gap(px(7.0))
                        .px(px(6.0))
                        .py(px(5.0))
                        .rounded(px(6.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(0xF3F4F6)))
                        .on_click(move |_, _, _| {
                            focus_session_window(pid);
                        })
                        .child(render_status_dot(state, anim_id))
                        .child(
                            div()
                                .flex_1()
                                .overflow_hidden()
                                .truncate()
                                .text_size(px(12.0))
                                .font_weight(FontWeight::NORMAL)
                                .text_color(rgb(0x1F2328))
                                .child(title),
                        )
                }))
        }))
}
