use std::time::Duration;
use gpui::{
    div, ease_in_out, prelude::*, px, rgb, svg, Animation, AnimationExt as _, AnyElement, AnyView,
    ClickEvent, Context, CursorStyle, Entity, IntoElement, Render, SharedString, StyleRefinement,
    Task, Window,
};

use crate::state::{AppState, ProjectGroup, TimeFilter};
use crate::ui::filter_bar::render_filter_bar;
use crate::ui::icons::RESIZE_GRIP_PATH;
use crate::ui::session_tree::{content_height, SessionPage, StatusDotLayer};
use crate::ui::title_bar::render_title_bar;

/// 切换 Today / All 的滑动时长，filter_bar 的滑块用的也是这个值
const SLIDE_DURATION: Duration = Duration::from_millis(180);

pub struct AppWindow {
    pub state: AppState,
    today_page: Entity<SessionPage>,
    all_page: Entity<SessionPage>,
    today_dots: Entity<StatusDotLayer>,
    all_dots: Entity<StatusDotLayer>,
    _slide_reset: Option<Task<()>>,
}

impl AppWindow {
    pub fn with_sessions(project_groups: Vec<ProjectGroup>, cx: &mut Context<Self>) -> Self {
        let state = AppState {
            is_pinned: true,
            filter: TimeFilter::Today,
            prev_filter: None,
            project_groups,
        };
        let today = state.today_groups();
        let all = state.all_groups();
        Self {
            today_page: cx.new(|cx| SessionPage::new(today.clone(), "today", cx)),
            all_page: cx.new(|cx| SessionPage::new(all.clone(), "all", cx)),
            today_dots: cx.new(|cx| StatusDotLayer::new(today, true, cx)),
            all_dots: cx.new(|cx| StatusDotLayer::new(all, false, cx)),
            state,
            _slide_reset: None,
        }
    }

    pub fn set_sessions(&mut self, project_groups: Vec<ProjectGroup>, cx: &mut Context<Self>) {
        if self.state.project_groups == project_groups {
            return;
        }
        self.state.project_groups = project_groups;
        let today = self.state.today_groups();
        let all = self.state.all_groups();
        self.today_page
            .update(cx, |page, cx| page.set_groups(today.clone(), cx));
        self.all_page
            .update(cx, |page, cx| page.set_groups(all.clone(), cx));
        self.today_dots
            .update(cx, |dots, cx| dots.set_groups(today, cx));
        self.all_dots.update(cx, |dots, cx| dots.set_groups(all, cx));
        cx.notify();
    }

    /// 滑动期间两页都在树上，动效跑完就把 prev_filter 清掉，隐藏的那一页不再参与布局
    fn select_filter(&mut self, filter: TimeFilter, cx: &mut Context<Self>) {
        if self.state.filter == filter {
            return;
        }
        self.state.prev_filter = Some(self.state.filter);
        self.state.filter = filter;
        self.sync_dot_visibility(true, cx);
        self._slide_reset = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SLIDE_DURATION).await;
            let _ = this.update(cx, |this, cx| {
                this.state.prev_filter = None;
                this.sync_dot_visibility(false, cx);
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// 只有挂在树上的那一页需要跑呼吸定时器，滑动期间两页都在
    fn sync_dot_visibility(&mut self, sliding: bool, cx: &mut Context<Self>) {
        let today = sliding || self.state.filter == TimeFilter::Today;
        let all = sliding || self.state.filter == TimeFilter::All;
        self.today_dots
            .update(cx, |dots, cx| dots.set_visible(today, cx));
        self.all_dots.update(cx, |dots, cx| dots.set_visible(all, cx));
    }
}

impl Render for AppWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_pinned = self.state.is_pinned;
        let current_filter = self.state.filter;
        let prev_filter = self.state.prev_filter;
        let today_groups = self.state.today_groups();
        let all_groups = self.state.all_groups();

        let visible_count: usize = match current_filter {
            TimeFilter::Today => today_groups.iter().map(|g| g.sessions.len()).sum(),
            TimeFilter::All => all_groups.iter().map(|g| g.sessions.len()).sum(),
        };

        let viewport_size = window.viewport_size();
        let page_width = (f32::from(viewport_size.width) - 2.0).max(200.0);

        let page_today = render_page_container(
            page_width,
            "today",
            &today_groups,
            TimeFilter::Today,
            &self.today_page,
            &self.today_dots,
        );
        let page_all = render_page_container(
            page_width,
            "all",
            &all_groups,
            TimeFilter::All,
            &self.all_page,
            &self.all_dots,
        );

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
                        Animation::new(SLIDE_DURATION).with_easing(ease_in_out),
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
                        Animation::new(SLIDE_DURATION).with_easing(ease_in_out),
                        move |el, delta| {
                            let x = -page_width + delta * page_width;
                            el.left(px(x))
                        },
                    )
                    .into_any_element()
            }
            _ => {
                // 静止时只挂当前这一页，隐藏的那一页连布局都不做
                let visible = match current_filter {
                    TimeFilter::Today => page_today,
                    TimeFilter::All => page_all,
                };
                div()
                    .id("track_static")
                    .absolute()
                    .top(px(0.0))
                    .bottom(px(0.0))
                    .h_full()
                    .left(px(0.0))
                    .w(px(page_width))
                    .flex()
                    .flex_row()
                    .child(visible)
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
                    this.select_filter(TimeFilter::Today, cx);
                }),
                cx.listener(|this, _event: &ClickEvent, _window, cx| {
                    this.select_filter(TimeFilter::All, cx);
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

/// 一页的滚动容器。两个缓存视图叠在同一个盒子里：内容层不随动效重绘，状态点层单独重绘
fn render_page_container(
    page_width: f32,
    id_prefix: &'static str,
    groups: &[ProjectGroup],
    filter: TimeFilter,
    page: &Entity<SessionPage>,
    dots: &Entity<StatusDotLayer>,
) -> AnyElement {
    let content: AnyElement = if groups.is_empty() {
        render_empty_state(filter, id_prefix)
    } else {
        // 外层是 overflow_y_scroll 的容器，可滚动范围取自这个直接子元素的高度，
        // 高度必须由内容撑开，钉成 100% 就永远滚不动
        div()
            .id(SharedString::from(format!("{}_tree_wrap", id_prefix)))
            .w_full()
            .flex_none()
            .child(
                // 两个缓存视图都没有内在高度，这个盒子按行节奏算出高度把它们撑开
                div()
                    .relative()
                    .h(px(content_height(groups)))
                    .mx(px(10.0))
                    .mt(px(6.0))
                    .mb(px(10.0))
                    .child(
                        AnyView::from(page.clone())
                            .cached(StyleRefinement::default().w_full().h_full()),
                    )
                    .child(
                        AnyView::from(dots.clone())
                            .cached(StyleRefinement::default().absolute().inset_0()),
                    ),
            )
            .into_any_element()
    };

    div()
        .id(SharedString::from(format!("page_{}_container", id_prefix)))
        .flex()
        .flex_col()
        .w(px(page_width))
        .h_full()
        .flex_shrink_0()
        .overflow_y_scroll()
        .child(content)
        .into_any_element()
}

fn render_empty_state(filter: TimeFilter, id_prefix: &str) -> AnyElement {
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
}
