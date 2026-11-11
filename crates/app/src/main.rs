// Release builds on Windows are GUI-subsystem executables: without this a
// console window opens next to the app.
#![cfg_attr(all(not(debug_assertions), windows), windows_subsystem = "windows")]

mod assets;
mod filter_bar;
mod notification_panel;
mod pr_card;
mod repo_group;
mod settings;
mod state;
mod theme;
mod toasts;
mod update_banner;
mod updater;
mod view;
mod widgets;

use std::sync::Arc;

use gpui::{
    px, size, App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions,
};
use gpui_component::Root;
use pr_dashboard_core::config::load_config;

use crate::assets::AppAssets;
use crate::state::{Dashboard, TokioRuntime};
use crate::theme::{Theme, ThemeMode};
use crate::updater::Updater;
use crate::view::AppView;

fn main() {
    let config = load_config();
    let mode = ThemeMode::parse(&config.theme);

    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("tokio runtime"),
    );

    Application::new().with_assets(AppAssets).run(move |cx: &mut App| {
        // Must run before any gpui-component widget is used.
        gpui_component::init(cx);
        gpui_component::Theme::change(
            match mode {
                ThemeMode::Dark => gpui_component::ThemeMode::Dark,
                ThemeMode::Light => gpui_component::ThemeMode::Light,
            },
            None,
            cx,
        );

        cx.set_global(TokioRuntime(runtime));
        cx.set_global(Theme::for_mode(mode));

        let dashboard = cx.new(|cx| {
            let mut d = Dashboard::new(config);
            d.start_polling(cx);
            d
        });

        let updater = cx.new(|cx| {
            let mut u = Updater::new();
            u.start(cx);
            u
        });

        let bounds = Bounds::centered(None, size(px(1100.), px(750.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(800.), px(500.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("PR Dashboard".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| AppView::new(dashboard, updater, window, cx));
                // The first level of a window must be a Root.
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("open window");
        cx.activate(true);
    });
}
