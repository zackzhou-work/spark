use gpui::{
    div, prelude::*, px, rgb, svg, AnyView, Context, Entity, FontWeight, IntoElement, Render,
    SharedString, StyleRefinement, Window,
};

use crate::platform::focus_session_window;
use crate::state::ProjectGroup;
use crate::ui::icons::FOLDER_PATH;
use crate::ui::status_dot::render_status_dot;

// 两列各自渲染，靠固定行高对齐
const HEADER_HEIGHT: f32 = 20.0;
const ROW_HEIGHT: f32 = 24.0;
const GROUP_GAP: f32 = 10.0;
const ROW_GAP: f32 = 2.0;
const ICON_COLUMN_WIDTH: f32 = 18.0;

/// 标题和项目名这一列作为缓存视图，呼吸动效重绘时不重新布局和排版
pub struct SessionTextColumn {
    groups: Vec<ProjectGroup>,
    id_prefix: &'static str,
}

impl SessionTextColumn {
    pub fn new(groups: Vec<ProjectGroup>, id_prefix: &'static str) -> Self {
        Self { groups, id_prefix }
    }

    pub fn set_groups(&mut self, groups: Vec<ProjectGroup>, cx: &mut Context<Self>) {
        if self.groups != groups {
            self.groups = groups;
            cx.notify();
        }
    }
}

impl Render for SessionTextColumn {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let id_prefix = self.id_prefix;
        div()
            .flex()
            .flex_col()
            .gap(px(GROUP_GAP))
            .children(self.groups.iter().map(|group| {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(ROW_GAP))
                    .child(
                        div()
                            .h(px(HEADER_HEIGHT))
                            .flex()
                            .items_center()
                            .pl(px(5.0))
                            .pr(px(6.0))
                            .overflow_hidden()
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::NORMAL)
                                    .text_color(rgb(0x9CA3AF))
                                    .child(group.project_name.clone()),
                            ),
                    )
                    .children(group.sessions.iter().map(|session| {
                        let pid = session.pid;
                        div()
                            .id(SharedString::from(format!("{}_row_{}", id_prefix, session.id)))
                            .h(px(ROW_HEIGHT))
                            .flex()
                            .items_center()
                            .pl(px(1.0))
                            .pr(px(6.0))
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .hover(|s| s.bg(rgb(0xF3F4F6)))
                            .on_click(move |_, _, _| focus_session_window(pid))
                            .overflow_hidden()
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(12.0))
                                    .font_weight(FontWeight::NORMAL)
                                    .text_color(rgb(0x1F2328))
                                    .child(session.title.clone()),
                            )
                    }))
            }))
    }
}

/// 文件夹图标和状态点这一列每帧重绘，内容很少
fn render_icon_column(groups: &[ProjectGroup], id_prefix: &str) -> impl IntoElement {
    div()
        .w(px(ICON_COLUMN_WIDTH))
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(GROUP_GAP))
        .children(groups.iter().map(|group| {
            div()
                .flex()
                .flex_col()
                .gap(px(ROW_GAP))
                .child(
                    div()
                        .h(px(HEADER_HEIGHT))
                        .flex()
                        .items_center()
                        .pl(px(6.0))
                        .child(svg().path(FOLDER_PATH).size(px(12.0)).text_color(rgb(0x9CA3AF))),
                )
                .children(group.sessions.iter().map(|session| {
                    let anim_id = SharedString::from(format!("{}_dot_{}", id_prefix, session.id));
                    div()
                        .h(px(ROW_HEIGHT))
                        .flex()
                        .items_center()
                        .pl(px(6.0))
                        .child(render_status_dot(session.state, anim_id))
                }))
        }))
}

pub fn render_session_tree(
    groups: &[ProjectGroup],
    id_prefix: &str,
    text_column: &Entity<SessionTextColumn>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_start()
        .px(px(8.0))
        .pt(px(2.0))
        .pb(px(10.0))
        .child(render_icon_column(groups, id_prefix))
        .child(AnyView::from(text_column.clone()).cached(StyleRefinement::default().flex_1().min_w_0()))
}
