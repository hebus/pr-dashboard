//! Root view (port of `App.tsx`): header, filter bar and repository groups.

use std::collections::HashSet;
use std::time::Instant;

use gpui::{
    div, list, prelude::*, px, rgb, Context, Entity, FontWeight, IntoElement, ListAlignment,
    ListState, ScrollHandle, SharedString, Subscription, Window,
};
use gpui_component::input::{InputEvent, InputState};
use gpui_component::scroll::ScrollableElement;
use pr_dashboard_core::logic::{EventKind, FilterStatus, PrEvent};
use pr_dashboard_core::providers::common::Provider;

use crate::repo_group::Row;
use crate::settings::{SettingsEvent, SettingsView};
use crate::state::Dashboard;
use crate::theme::{Theme, ThemeMode};
use crate::toasts::Toast;
use crate::updater::Updater;
use crate::widgets::{count_pill, icon, icon_button, text_button};

pub struct AppView {
    pub(crate) dashboard: Entity<Dashboard>,
    pub(crate) updater: Entity<Updater>,
    pub(crate) search: Entity<InputState>,
    pub(crate) filter: FilterStatus,
    /// Ids of the collapsed repository groups.
    pub(crate) collapsed: HashSet<String>,
    /// "<repo id>-<pr number>" of the card whose branch was just copied.
    pub(crate) copied: Option<String>,
    /// Flattened rows of the virtualized list (see `repo_group.rs`).
    pub(crate) rows: Vec<Row>,
    pub(crate) list_state: ListState,
    /// Set whenever the data, filter, search or collapsed groups change.
    pub(crate) rows_dirty: bool,
    /// The Settings modal, while open.
    settings: Option<Entity<SettingsView>>,
    _settings_subscription: Option<Subscription>,
    pub(crate) show_notifications: bool,
    /// When the panel last closed because of a click outside it.
    pub(crate) notifications_closed_at: Option<Instant>,
    /// Scroll position of the notification list (it has its own scrollbar).
    pub(crate) notifications_scroll: ScrollHandle,
    pub(crate) toasts: Vec<Toast>,
    pub(crate) next_toast_id: u64,
    _subscriptions: Vec<Subscription>,
}

/// What "Test" in Settings raises, as in the React app.
fn test_event() -> PrEvent {
    PrEvent {
        kind: EventKind::NewPr,
        provider: Provider::Github,
        repo: "sinequa/test".to_string(),
        pr_number: 42,
        pr_title: "This is a test notification".to_string(),
        url: String::new(),
    }
}

/// Extra pixels rendered above and below the viewport, for smoother scrolling.
const LIST_OVERDRAW: f32 = 400.;

impl AppView {
    pub fn new(
        dashboard: Entity<Dashboard>,
        updater: Entity<Updater>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search PRs / MRs..."));

        let subscriptions = vec![
            cx.observe(&dashboard, |this, _, cx| {
                this.rows_dirty = true;
                cx.notify();
            }),
            // Re-filter on every keystroke so the groups update live.
            cx.subscribe(&search, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.rows_dirty = true;
                    cx.notify();
                }
            }),
            cx.observe(&updater, |_, _, cx| cx.notify()),
            // New / merged PRs detected by the polling loops.
            cx.subscribe(&dashboard, |this, _, event: &PrEvent, cx| {
                this.add_toast(event.clone(), cx);
            }),
        ];

        AppView {
            dashboard,
            updater,
            search,
            filter: FilterStatus::All,
            collapsed: HashSet::new(),
            copied: None,
            rows: Vec::new(),
            // `measure_all`: without it the list only knows the height of the rows
            // it has drawn, and the scrollbar thumb jumps while scrolling with
            // the wheel. See `rebuild_rows`.
            list_state: ListState::new(0, ListAlignment::Top, px(LIST_OVERDRAW)).measure_all(),
            rows_dirty: true,
            settings: None,
            _settings_subscription: None,
            show_notifications: false,
            notifications_closed_at: None,
            notifications_scroll: ScrollHandle::new(),
            toasts: Vec::new(),
            next_toast_id: 1,
            _subscriptions: subscriptions,
        }
    }

    fn open_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings.is_some() {
            return;
        }
        let dashboard = self.dashboard.clone();
        let view = cx.new(|cx| SettingsView::new(dashboard, window, cx));
        self._settings_subscription =
            Some(cx.subscribe(&view, |this, _, event: &SettingsEvent, cx| match event {
                SettingsEvent::Close => {
                    this.settings = None;
                    cx.notify();
                }
                SettingsEvent::TestNotification => {
                    this.dashboard.update(cx, |d, cx| d.push_event(test_event(), cx));
                }
            }));
        self.settings = Some(view);
        cx.notify();
    }

    fn header(
        &self,
        t: &Theme,
        configured: bool,
        mode: ThemeMode,
        unread: usize,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let totals = self.dashboard.read(cx).totals();

        div()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(t.border)
            .bg(t.bg_subtle)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(icon("icons/git-merge.svg", 20., t.text))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("PR Dashboard"))
                    .when(totals.pending > 0 || totals.approved > 0, |e| {
                        e.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .ml_1()
                                .when(totals.pending > 0, |e| {
                                    e.child(count_pill(t.amber, totals.pending))
                                })
                                .when(totals.approved > 0, |e| {
                                    e.child(count_pill(t.green_btn, totals.approved))
                                }),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        icon_button("refresh", "icons/refresh-cw.svg", t.text_muted, t)
                            .when(!configured, |e| e.opacity(0.4))
                            .when(configured, |e| {
                                e.on_click(cx.listener(|this, _, _, cx| {
                                    this.dashboard.update(cx, |d, cx| d.start_polling(cx));
                                }))
                            }),
                    )
                    .child(
                        icon_button(
                            "theme",
                            if mode == ThemeMode::Dark { "icons/sun.svg" } else { "icons/moon.svg" },
                            t.text_muted,
                            t,
                        )
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.dashboard.update(cx, |d, cx| d.toggle_theme(cx));
                            let next = this.dashboard.read(cx).theme_mode();
                            gpui_component::Theme::change(
                                match next {
                                    ThemeMode::Dark => gpui_component::ThemeMode::Dark,
                                    ThemeMode::Light => gpui_component::ThemeMode::Light,
                                },
                                Some(window),
                                cx,
                            );
                        })),
                    )
                    .child(
                        div()
                            .relative()
                            .child(
                                icon_button("notifications", "icons/bell.svg", t.text_muted, t)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.toggle_notifications(cx)
                                    })),
                            )
                            .when(unread > 0, |e| {
                                e.child(
                                    div()
                                        .absolute()
                                        .top_1()
                                        .right_1()
                                        .size(px(8.))
                                        .rounded_full()
                                        .bg(t.accent),
                                )
                            }),
                    )
                    .child(
                        icon_button("settings", "icons/settings.svg", t.text_muted, t).on_click(
                            cx.listener(|this, _, window, cx| this.open_settings(window, cx)),
                        ),
                    )
                    .child(
                        icon_button("quit", "icons/power.svg", t.text_muted, t)
                            .on_click(|_, _, cx| cx.quit()),
                    ),
            )
    }
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = *cx.global::<Theme>();
        let (totals, has_repos, has_token, missing, unread, mode) = {
            let d = self.dashboard.read(cx);
            (
                d.totals(),
                d.has_repos(),
                d.has_token(),
                d.missing_providers(),
                d.unread_count(),
                d.theme_mode(),
            )
        };
        let configured = has_repos && has_token;

        if self.rows_dirty {
            self.rebuild_rows(cx);
            self.rows_dirty = false;
        }

        let header = self.header(&t, configured, mode, unread, cx);

        let content = if !configured {
            let (title, hint): (SharedString, &str) = if !has_repos {
                (
                    "Add repositories to monitor".into(),
                    "Open Settings to add GitHub or GitLab repositories.",
                )
            } else {
                (
                    format!("Configure your {} token", missing.join(" / ")).into(),
                    "Open Settings to add the Personal Access Token for that source.",
                )
            };
            div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .child(icon("icons/git-pull-request.svg", 40., t.border))
                .child(div().font_weight(FontWeight::MEDIUM).child(title))
                .child(div().text_sm().text_color(t.text_muted).child(hint))
                .child(
                    div().mt_2().child(
                        text_button(
                            "open-settings",
                            "Open Settings",
                            Some("icons/settings.svg"),
                            rgb(0xffffff),
                            Some(t.green_btn),
                            t.green_btn_hover,
                        )
                        .px_4()
                        .py_2()
                        .text_sm()
                        .on_click(cx.listener(|this, _, window, cx| this.open_settings(window, cx))),
                    ),
                )
                .into_any_element()
        } else {
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .child(self.filter_bar(totals, &t, cx))
                .child(
                    // Only the rows on screen are built each frame. The scrollbar is
                    // drawn over the right edge, in the gutter left by the rows' padding.
                    div()
                        .relative()
                        .flex_1()
                        .min_h_0()
                        .child(
                            list(
                                self.list_state.clone(),
                                cx.processor(|this, ix, _window, cx| this.render_row(ix, cx)),
                            )
                            .size_full(),
                        )
                        .vertical_scrollbar(&self.list_state),
                )
                .into_any_element()
        };

        // `relative` makes the overlays (panel, modal, toasts) position against
        // the whole window. They come last so they paint on top.
        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(t.bg)
            .text_color(t.text)
            .child(header)
            .when_some(self.update_banner(&t, cx), |e, banner| e.child(banner))
            .child(content)
            .when(self.show_notifications, |e| e.child(self.notification_panel(&t, cx)))
            .when_some(self.settings.clone(), |e, settings| e.child(settings))
            .when(!self.toasts.is_empty(), |e| e.child(self.toast_stack(&t, cx)))
    }
}
