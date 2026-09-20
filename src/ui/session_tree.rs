use gpui::{
    div, linear_color_stop, linear_gradient, prelude::*, px, rgb, rgba, svg, AnyView, Context,
    Entity, FontWeight, IntoElement, Render, SharedString, StyleRefinement, Window,
};

use crate::platform::jump_to_session;
use crate::state::ProjectGroup;
use crate::ui::icons::{FOLDER_PATH, JUMP_PATH};
use crate::ui::status_dot::render_status_dot;

// 两列各自渲染，靠固定行高对齐
const HEADER_HEIGHT: f32 = 20.0;
const ROW_HEIGHT: f32 = 24.0;
const GROUP_GAP: f32 = 10.0;
const ROW_GAP: f32 = 2.0;
const ICON_COLUMN_WIDTH: f32 = 24.0;
const ICON_CELL_SIZE: f32 = 12.0;
const ROW_RADIUS: f32 = 6.0;
/// hover 底色，行高亮和跳转图标的渐变底共用，避免两处颜色走散
const HOVER_BG: u32 = 0xF3F4F6;
/// 跳转图标连同它底下的渐变一起占这么宽，让长标题在图标左侧淡出
const JUMP_FADE_WIDTH: f32 = 48.0;

/// 标题和项目名这一列作为缓存视图，呼吸动效重绘时不重新布局和排版
pub struct SessionTextColumn {
    groups: Vec<ProjectGroup>,
}

impl SessionTextColumn {
    pub fn new(groups: Vec<ProjectGroup>) -> Self {
        Self { groups }
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
        // 缓存视图按根节点单独布局，不写宽度就会被最长标题撑开。
        // 每一行用 flex_col 而不是 flex_row：taffy 量 flex 行子项时主轴按 MaxContent 测，
        // 文本拿不到确定宽度就不会截断；列的交叉轴宽度是确定的，truncate 才能生效
        div()
            .w_full()
            .overflow_hidden()
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
                            .flex_col()
                            .justify_center()
                            .pl(px(1.0))
                            .pr(px(6.0))
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
                        div()
                            .h(px(ROW_HEIGHT))
                            .flex()
                            .flex_col()
                            .justify_center()
                            .pl(px(1.0))
                            .pr(px(6.0))
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

/// 文件夹图标和状态点共用同一个格子，保证两者中心在同一根竖线上
fn icon_cell() -> gpui::Div {
    div()
        .size(px(ICON_CELL_SIZE))
        .flex()
        .items_center()
        .justify_center()
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
                        .pl(px(8.0))
                        .child(
                            icon_cell().child(
                                svg().path(FOLDER_PATH).size(px(ICON_CELL_SIZE)).text_color(rgb(0x9CA3AF)),
                            ),
                        ),
                )
                .children(group.sessions.iter().map(|session| {
                    let anim_id = SharedString::from(format!("{}_dot_{}", id_prefix, session.id));
                    div()
                        .h(px(ROW_HEIGHT))
                        .flex()
                        .items_center()
                        .pl(px(8.0))
                        .child(icon_cell().child(render_status_dot(session.state, anim_id)))
                }))
        }))
}

/// 跳转图标层：画在两列之上，hover 时淡入，底下压一条渐变把标题尾部盖掉。
/// 和下面的 hover 层各管各的命中框——gpui 的命中框默认互不遮挡，两层会同时进入 hover
fn render_jump_layer(groups: &[ProjectGroup]) -> impl IntoElement {
    div()
        .absolute()
        .inset_0()
        .flex()
        .flex_col()
        .gap(px(GROUP_GAP))
        .children(groups.iter().map(|group| {
            div()
                .flex()
                .flex_col()
                .gap(px(ROW_GAP))
                .child(div().h(px(HEADER_HEIGHT)))
                .children(group.sessions.iter().map(|_| {
                    div()
                        .h(px(ROW_HEIGHT))
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_end()
                        .opacity(0.0)
                        .hover(|s| s.opacity(1.0))
                        .child(
                            div()
                                .w(px(JUMP_FADE_WIDTH))
                                .h_full()
                                .rounded_r(px(ROW_RADIUS))
                                .flex()
                                .items_center()
                                .justify_end()
                                .pr(px(6.0))
                                .bg(linear_gradient(
                                    90.0,
                                    linear_color_stop(rgba(HOVER_BG << 8), 0.0),
                                    linear_color_stop(rgb(HOVER_BG), 0.55),
                                ))
                                .child(
                                    svg()
                                        .path(JUMP_PATH)
                                        .size(px(ICON_CELL_SIZE))
                                        .text_color(rgb(0x9CA3AF)),
                                ),
                        )
                }))
        }))
}

/// 铺满整行宽度的透明层，承载 hover 高亮和点击；画在两列下面，普通元素不会挡住它的命中
fn render_hover_layer(groups: &[ProjectGroup], id_prefix: &str) -> impl IntoElement {
    div()
        .absolute()
        .inset_0()
        .flex()
        .flex_col()
        .gap(px(GROUP_GAP))
        .children(groups.iter().map(|group| {
            div()
                .flex()
                .flex_col()
                .gap(px(ROW_GAP))
                .child(div().h(px(HEADER_HEIGHT)))
                .children(group.sessions.iter().map(|session| {
                    let session_id = session.id.clone();
                    div()
                        .id(SharedString::from(format!("{}_row_{}", id_prefix, session.id)))
                        .h(px(ROW_HEIGHT))
                        .w_full()
                        .rounded(px(ROW_RADIUS))
                        .cursor_pointer()
                        .hover(|s| s.bg(rgb(HOVER_BG)))
                        .on_click(move |_, _, _| jump_to_session(&session_id))
                }))
        }))
}

pub fn render_session_tree(
    groups: &[ProjectGroup],
    id_prefix: &str,
    text_column: &Entity<SessionTextColumn>,
) -> impl IntoElement {
    div()
        .relative()
        .mx(px(10.0))
        .mt(px(6.0))
        .mb(px(10.0))
        .child(render_hover_layer(groups, id_prefix))
        .child(
            div()
                .flex()
                .flex_row()
                .items_start()
                .child(render_icon_column(groups, id_prefix))
                .child(AnyView::from(text_column.clone()).cached(StyleRefinement::default().flex_1().min_w_0())),
        )
        .child(render_jump_layer(groups))
}
