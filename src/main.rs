mod config;
mod monitor;
mod platform;
mod state;
mod ui;

use config::WindowConfig;
use gpui::{
    actions, point, px, size, App, AppContext, Application, Bounds, KeyBinding, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions,
};

use platform::{hide_native_traffic_lights, set_window_always_on_top, setup_window_resizable};
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
                        titlebar: None,
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
                        setup_window_resizable(window, 240.0, 280.0);

                        // 启动时立即扫描加载会话
                        let mut scanner = crate::monitor::ProcessScanner::new();
                        let initial_sessions = scanner.scan_claude_sessions();
                        let mut updates = crate::monitor::watcher::spawn(scanner);
                        let app_view = cx.new(|cx| {
                            let subscription = cx.observe_window_bounds(window, |_this, window, _cx| {
                                hide_native_traffic_lights(window);
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

                            AppWindow::with_sessions(initial_sessions)
                        });

                        // 后台线程监听文件变化并扫描，这里只负责把结果刷到 UI
                        let view = app_view.clone();
                        window
                            .spawn(cx, async move |cx| {
                                while let Some(sessions) = updates.recv().await {
                                    let updated = view.update(cx, |view, cx| {
                                        view.state.project_groups = sessions;
                                        cx.notify();
                                    });
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
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        });
}

