use std::time::{Duration, Instant};

use gpui::{
    div, linear_color_stop, linear_gradient, prelude::*, px, rgb, rgba, svg, AnyView, Context,
    Entity, FontWeight, IntoElement, Render, SharedString, StyleRefinement, Task, Window,
};

use crate::platform::jump_to_session;
use crate::state::{ProjectGroup, TaskState};
use crate::ui::icons::{FOLDER_PATH, JUMP_PATH};
use crate::ui::status_dot::{render_status_dot, BREATH_PERIOD_MS};

// 各层各自渲染，靠固定行高对齐
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

/// 缓存视图没有内在高度，撑开滚动区的高度只能按上面这套行节奏自己算
pub fn content_height(groups: &[ProjectGroup]) -> f32 {
    if groups.is_empty() {
        return 0.0;
    }
    let rows: f32 = groups
        .iter()
        .map(|g| HEADER_HEIGHT + g.sessions.len() as f32 * (ROW_HEIGHT + ROW_GAP))
        .sum();
    rows + (groups.len() - 1) as f32 * GROUP_GAP
}

/// 标题和项目名这一列作为缓存视图，hover 重绘时不重新布局和排版
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

/// 一页会话列表里不随动效变化的部分：hover 层、文件夹图标列和标题列。
/// 整页挂成缓存视图，状态点每帧重绘时它的布局和排版都能原样复用
pub struct SessionPage {
    groups: Vec<ProjectGroup>,
    id_prefix: &'static str,
    text_column: Entity<SessionTextColumn>,
}

impl SessionPage {
    pub fn new(
        groups: Vec<ProjectGroup>,
        id_prefix: &'static str,
        cx: &mut Context<Self>,
    ) -> Self {
        let text_column = cx.new(|_| SessionTextColumn::new(groups.clone()));
        Self {
            groups,
            id_prefix,
            text_column,
        }
    }

    pub fn set_groups(&mut self, groups: Vec<ProjectGroup>, cx: &mut Context<Self>) {
        if self.groups == groups {
            return;
        }
        self.groups = groups.clone();
        self.text_column
            .update(cx, |column, cx| column.set_groups(groups, cx));
        cx.notify();
    }
}

impl Render for SessionPage {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            .w_full()
            .child(render_hover_layer(&self.groups, self.id_prefix))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_start()
                    .child(render_folder_column(&self.groups))
                    .child(
                        AnyView::from(self.text_column.clone())
                            .cached(StyleRefinement::default().flex_1().min_w_0()),
                    ),
            )
            .child(render_jump_layer(&self.groups))
    }
}

/// 呼吸动效的重绘节奏。gpui 的 with_animation 按屏幕刷新率逐帧请求重绘，
/// 而一次重绘要走完整的 present，一颗 6px 的慢速渐变不值这个开销，
/// 这里自己按固定间隔推进相位
const BREATH_TICK: Duration = Duration::from_millis(80);

/// 状态点单独一层：只有它跟着呼吸重绘，其余各层都在缓存视图里原样复用
pub struct StatusDotLayer {
    groups: Vec<ProjectGroup>,
    started: Instant,
    visible: bool,
    breathing: Option<Task<()>>,
}

impl StatusDotLayer {
    pub fn new(groups: Vec<ProjectGroup>, visible: bool, cx: &mut Context<Self>) -> Self {
        let mut layer = Self {
            groups,
            started: Instant::now(),
            visible,
            breathing: None,
        };
        layer.sync_breathing(cx);
        layer
    }

    pub fn set_groups(&mut self, groups: Vec<ProjectGroup>, cx: &mut Context<Self>) {
        if self.groups == groups {
            return;
        }
        self.groups = groups;
        self.sync_breathing(cx);
        cx.notify();
    }

    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            self.sync_breathing(cx);
        }
    }

    /// 这一页没显示、或者没有运行中的会话，就把定时器停掉，窗口一帧都不用重绘
    fn sync_breathing(&mut self, cx: &mut Context<Self>) {
        let running = self.visible
            && self
                .groups
                .iter()
                .flat_map(|g| &g.sessions)
                .any(|s| s.state == TaskState::Running);

        if !running {
            self.breathing = None;
        } else if self.breathing.is_none() {
            self.breathing = Some(cx.spawn(async move |this, cx| loop {
                cx.background_executor().timer(BREATH_TICK).await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    return;
                }
            }));
        }
    }
}

impl Render for StatusDotLayer {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // 先取模再转 f32：跑上几小时之后毫秒数会超出 f32 的精度，相位会卡住
        let elapsed_ms = self.started.elapsed().as_millis() as u64;
        let phase = (elapsed_ms % BREATH_PERIOD_MS) as f32 / BREATH_PERIOD_MS as f32;
        div()
            .w(px(ICON_COLUMN_WIDTH))
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(GROUP_GAP))
            .children(self.groups.iter().map(|group| {
                div()
                    .flex()
                    .flex_col()
                    .gap(px(ROW_GAP))
                    .child(div().h(px(HEADER_HEIGHT)))
                    .children(group.sessions.iter().map(|session| {
                        div()
                            .h(px(ROW_HEIGHT))
                            .flex()
                            .items_center()
                            .pl(px(8.0))
                            .child(icon_cell().child(render_status_dot(session.state, phase)))
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

/// 文件夹图标这一列；会话行留空占位，状态点由上面那一层盖上来
fn render_folder_column(groups: &[ProjectGroup]) -> impl IntoElement {
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
                .children(group.sessions.iter().map(|_| div().h(px(ROW_HEIGHT))))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{SessionItem, TaskState};

    fn group(name: &str, sessions: usize) -> ProjectGroup {
        ProjectGroup {
            project_name: name.to_string(),
            sessions: (0..sessions)
                .map(|i| SessionItem {
                    id: format!("{name}-{i}"),
                    title: format!("session {i}"),
                    state: TaskState::Completed,
                    pid: None,
                    working_dir: String::new(),
                    is_today: true,
                })
                .collect(),
        }
    }

    #[test]
    fn content_height_follows_the_row_rhythm() {
        assert_eq!(content_height(&[]), 0.0);
        // 一组两行：表头 + 两行 + 组内三个子项之间的两个间距
        assert_eq!(
            content_height(&[group("a", 2)]),
            HEADER_HEIGHT + 2.0 * ROW_HEIGHT + 2.0 * ROW_GAP
        );
        // 两组之间再加一个组间距
        assert_eq!(
            content_height(&[group("a", 1), group("b", 1)]),
            2.0 * (HEADER_HEIGHT + ROW_HEIGHT + ROW_GAP) + GROUP_GAP
        );
    }
}
