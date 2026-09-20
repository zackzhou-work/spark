use std::time::Duration;
use gpui::{
    div, ease_in_out, prelude::*, px, rgb, svg, Animation, AnimationExt as _, AnyElement, ClickEvent,
    Context, CursorStyle, Entity, IntoElement, Render, SharedString, Window,
};

use crate::state::{AppState, ProjectGroup, TimeFilter};
use crate::ui::filter_bar::render_filter_bar;
use crate::ui::icons::RESIZE_GRIP_PATH;
use crate::ui::session_tree::{render_session_tree, SessionTextColumn};
use crate::ui::title_bar::render_title_bar;

pub struct AppWindow {
    pub state: AppState,
    today_list: Entity<SessionTextColumn>,
    all_list: Entity<SessionTextColumn>,
}

impl AppWindow {
    pub fn with_sessions(project_groups: Vec<ProjectGroup>, cx: &mut Context<Self>) -> Self {
        let state = AppState {
            is_pinned: true,
            filter: TimeFilter::Today,
            prev_filter: None,
            project_groups,
        };
        let today_list = cx.new(|_| SessionTextColumn::new(state.today_groups(), "today"));
        let all_list = cx.new(|_| SessionTextColumn::new(state.all_groups(), "all"));
        Self {
            state,
            today_list,
            all_list,
        }
    }

    pub fn set_sessions(&mut self, project_groups: Vec<ProjectGroup>, cx: &mut Context<Self>) {
        if self.state.project_groups == project_groups {
            return;
        }
        self.state.project_groups = project_groups;
        let today = self.state.today_groups();
        let all = self.state.all_groups();
        self.today_list.update(cx, |list, cx| list.set_groups(today, cx));
        self.all_list.update(cx, |list, cx| list.set_groups(all, cx));
        cx.notify();
    }
}

impl Render for AppWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_pinned = self.state.is_pinned;
        let current_filter = self.state.filter;
        let prev_filter = self.state.prev_filter;
        let today_groups = self.state.today_groups();
        let all_groups = self.state.all_groups();
        let today_list = self.today_list.clone();
        let all_list = self.all_list.clone();

        let visible_count: usize = match current_filter {
            TimeFilter::Today => today_groups.iter().map(|g| g.sessions.len()).sum(),
            TimeFilter::All => all_groups.iter().map(|g| g.sessions.len()).sum(),
        };

        let viewport_size = window.viewport_size();
        let page_width = (f32::from(viewport_size.width) - 2.0).max(200.0);

        let page_today = div()
            .id("page_today_container")
            .flex()
            .flex_col()
            .w(px(page_width))
            .h_full()
            .flex_shrink_0()
            .overflow_y_scroll()
            .child(render_session_page(&today_groups, TimeFilter::Today, "today", &today_list));

        let page_all = div()
            .id("page_all_container")
            .flex()
            .flex_col()
            .w(px(page_width))
            .h_full()
            .flex_shrink_0()
            .overflow_y_scroll()
            .child(render_session_page(&all_groups, TimeFilter::All, "all", &all_list));

        let sliding_track: AnyElement = match (prev_filter, current_filter) {
            (Some(TimeFilter::Today), TimeFilter::All) => {
                // Today ➔ All: Slide track from 0.0 to -page_width (leftwards)
                div()
                    .id("track_sliding_to_all")
                    .absolute()
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .h_full()
                    .w(px(page_width * 2.0))
                    .flex()
                    .flex_row()
                    .child(page_today)
                    .child(page_all)
                    .with_animation(
                        "slide_track_to_all",
                        Animation::new(Duration::from_millis(180)).with_easing(ease_in_out),
                        move |el, delta| {
                            let x = 0.0 - delta * page_width;
                            el.left(px(x))
                        },
                    )
                    .into_any_element()
            }
            (Some(TimeFilter::All), TimeFilter::Today) => {
                // All ➔ Today: Slide track from -page_width to 0.0 (rightwards)
                div()
                    .id("track_sliding_to_today")
                    .absolute()
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .h_full()
                    .w(px(page_width * 2.0))
                    .flex()
                    .flex_row()
                    .child(page_today)
                    .child(page_all)
                    .with_animation(
                        "slide_track_to_today",
                        Animation::new(Duration::from_millis(180)).with_easing(ease_in_out),
                        move |el, delta| {
                            let x = -page_width + delta * page_width;
                            el.left(px(x))
                        },
                    )
                    .into_any_element()
            }
            _ => {
                let left_offset = if current_filter == TimeFilter::Today {
                    0.0
                } else {
                    -page_width
                };
                div()
                    .id("track_static")
                    .absolute()
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .h_full()
                    .left(px(left_offset))
                    .w(px(page_width * 2.0))
                    .flex()
                    .flex_row()
                    .child(page_today)
                    .child(page_all)
                    .into_any_element()
            }
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0xFFFFFF))
            .rounded(px(12.0))
            .border_1()
            .border_color(rgb(0xE8E8E8))
            .shadow_lg()
            .overflow_hidden()
            .relative()
            .child(render_title_bar(
                is_pinned,
                cx.listener(|this, _event: &ClickEvent, _window, cx| {
                    this.state.is_pinned = !this.state.is_pinned;
                    cx.notify();
                }),
            ))
            .child(render_filter_bar(
                current_filter,
                prev_filter,
                visible_count,
                cx.listener(|this, _event: &ClickEvent, _window, cx| {
                    if this.state.filter != TimeFilter::Today {
                        this.state.prev_filter = Some(this.state.filter);
                        this.state.filter = TimeFilter::Today;
                        cx.notify();
                    }
                }),
                cx.listener(|this, _event: &ClickEvent, _window, cx| {
                    if this.state.filter != TimeFilter::All {
                        this.state.prev_filter = Some(this.state.filter);
                        this.state.filter = TimeFilter::All;
                        cx.notify();
                    }
                }),
            ))
            .child(
                div()
                    .id("session_carousel_viewport")
                    .relative()
                    .flex_1()
                    .w_full()
                    .overflow_hidden()
                    .child(sliding_track),
            )
            .child(
                div()
                    .id("resize_grip")
                    .absolute()
                    .bottom(px(2.0))
                    .right(px(2.0))
                    .w(px(14.0))
                    .h(px(14.0))
                    .cursor(CursorStyle::ResizeUpLeftDownRight)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        svg()
                            .path(RESIZE_GRIP_PATH)
                            .size(px(10.0))
                            .text_color(rgb(0xD1D5DB)),
                    ),
            )
    }
}

fn render_session_page(
    groups: &[ProjectGroup],
    filter: TimeFilter,
    id_prefix: &'static str,
    text_column: &Entity<SessionTextColumn>,
) -> impl IntoElement {
    if groups.is_empty() {
        let (main_text, sub_text) = match filter {
            TimeFilter::Today => (
                "No active Claude sessions today",
                "Switch to \"All\" to view session history",
            ),
            TimeFilter::All => (
                "No Claude sessions found",
                "Run claude in terminal to start monitoring",
            ),
        };

        div()
            .id(SharedString::from(format!("{}_empty", id_prefix)))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .py(px(40.0))
            .gap(px(6.0))
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(rgb(0x6B7280))
                    .child(main_text),
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(rgb(0x9CA3AF))
                    .child(sub_text),
            )
            .into_any_element()
    } else {
        div()
            .id(SharedString::from(format!("{}_tree_wrap", id_prefix)))
            .size_full()
            .child(render_session_tree(groups, id_prefix, text_column))
            .into_any_element()
    }
}
