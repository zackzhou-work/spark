mod config;
mod monitor;
mod platform;
mod state;
mod ui;

use config::WindowConfig;
use gpui::{
    actions, point, px, size, App, AppContext, Application, Bounds, KeyBinding, TitlebarOptions,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions,
};

use platform::set_window_always_on_top;
use ui::{AppWindow, EmbeddedAssets};

actions!(window, [Quit]);

fn main() {
    Application::new()
        .with_assets(EmbeddedAssets)
        .run(|cx: &mut App| {
            let window_config = WindowConfig::load();
            window_config.save();
            let bounds = Bounds::new(
                point(px(window_config.x), px(window_config.y)),
                size(px(window_config.width), px(window_config.height)),
            );

            let _window = cx
                .open_window(
                    WindowOptions {
                        titlebar: Some(TitlebarOptions {
                            title: None,
                            appears_transparent: true,
                            // 红绿灯离窗口左上角的偏移：x 对齐标题栏的左内边距，
                            // y 让这三颗按钮和右侧的图钉落在同一条中线上
                            traffic_light_position: Some(point(px(14.0), px(16.0))),
                        }),
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        kind: WindowKind::Normal,
                        is_movable: true,
                        is_resizable: true,
                        window_min_size: Some(size(px(240.0), px(280.0))),
                        window_background: WindowBackgroundAppearance::Transparent,
                        ..Default::default()
                    },
                    |window, cx| {
                        set_window_always_on_top(window, true);

                        // 启动时立即扫描加载会话
                        let mut scanner = crate::monitor::ProcessScanner::new();
                        let initial_sessions = scanner.scan_claude_sessions();
                        let mut updates = crate::monitor::watcher::spawn(scanner);
                        let app_view = cx.new(|cx| {
                            let subscription = cx.observe_window_bounds(window, |_this, window, _cx| {
                                let b = window.bounds();
                                let cfg = WindowConfig {
                                    x: f32::from(b.origin.x),
                                    y: f32::from(b.origin.y),
                                    width: f32::from(b.size.width),
                                    height: f32::from(b.size.height),
                                };
                                cfg.save();
                            });
                            subscription.detach();

                            AppWindow::with_sessions(initial_sessions, cx)
                        });

                        // 后台线程监听文件变化并扫描，这里只负责把结果刷到 UI
                        let view = app_view.clone();
                        window
                            .spawn(cx, async move |cx| {
                                while let Some(sessions) = updates.recv().await {
                                    let updated = view.update(cx, |view, cx| view.set_sessions(sessions, cx));
                                    if updated.is_err() {
                                        break;
                                    }
                                }
                            })
                            .detach();

                        app_view
                    },
                )
                .expect("Failed to open GPUI window");

            cx.activate(true);
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        });
}

