//! The notification history dropdown (port of `NotificationPanel.tsx`).

use std::time::{Duration, Instant};

use gpui::{div, prelude::*, px, Context, FontWeight, IntoElement, SharedString};
use pr_dashboard_core::logic::{time_ago, EventKind};

use crate::theme::Theme;
use crate::toasts::{event_line, event_title};
use crate::view::AppView;
use crate::widgets::{count_pill, icon, text_button};

/// A press outside the panel closes it on mouse-down; the bell's own click then
/// arrives on mouse-up and must not reopen it. Clicks this soon after an
/// outside-close are treated as that same press.
const REOPEN_GUARD: Duration = Duration::from_millis(300);

impl AppView {
    pub(crate) fn toggle_notifications(&mut self, cx: &mut Context<Self>) {
        if self.show_notifications {
            self.show_notifications = false;
        } else if !self.notifications_closed_at.is_some_and(|at| at.elapsed() < REOPEN_GUARD) {
            self.show_notifications = true;
        }
        cx.notify();
    }

    fn close_notifications(&mut self, cx: &mut Context<Self>) {
        self.show_notifications = false;
        self.notifications_closed_at = Some(Instant::now());
        cx.notify();
    }

    pub(crate) fn notification_panel(&self, t: &Theme, cx: &Context<Self>) -> impl IntoElement {
        let t = *t;
        let d = self.dashboard.read(cx);
        let unread = d.unread_count();

        let rows: Vec<_> = d
            .events
            .iter()
            .map(|n| {
                let id = n.id;
                let is_new = n.event.kind == EventKind::NewPr;
                let url = n.event.url.clone();
                let read = n.read;

                div()
                    .id(SharedString::from(format!("notif-{id}")))
                    .group("notif-row")
                    .flex()
                    .items_start()
                    .gap_2p5()
                    .px_3()
                    .py_2p5()
                    .border_b_1()
                    .border_color(t.border)
                    .cursor_pointer()
                    .when(!read, |e| e.bg(t.bg_inset))
                    .hover(move |s| s.bg(t.bg_hover))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.dashboard.update(cx, |d, cx| d.mark_read(id, cx));
                        if !url.is_empty() {
                            cx.open_url(&url);
                        }
                    }))
                    .child(
                        div()
                            .mt_1()
                            .flex_none()
                            .size(px(6.))
                            .rounded_full()
                            .when(!read, |e| e.bg(t.accent)),
                    )
                    .child(div().mt_0p5().flex_none().child(icon(
                        if is_new { "icons/git-pull-request.svg" } else { "icons/git-merge.svg" },
                        13.,
                        if is_new { t.accent } else { t.green },
                    )))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(event_title(&n.event))
                                    .child(
                                        div()
                                            .font_weight(FontWeight::NORMAL)
                                            .text_color(t.text_muted)
                                            .child(format!(" \u{b7} {}", n.event.repo)),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(t.text_muted)
                                    .truncate()
                                    .child(event_line(&n.event)),
                            )
                            .child(
                                div()
                                    .mt_0p5()
                                    .text_size(px(10.))
                                    .text_color(t.text_muted)
                                    .opacity(0.6)
                                    .child(time_ago(&n.timestamp)),
                            ),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("notif-dismiss-{id}")))
                            .mt_0p5()
                            .flex_none()
                            .cursor_pointer()
                            .opacity(0.)
                            .group_hover("notif-row", |s| s.opacity(1.))
                            .child(icon("icons/x.svg", 12., t.text_muted))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                // Do not also open the PR / mark it read.
                                cx.stop_propagation();
                                this.dashboard.update(cx, |d, cx| d.remove_event(id, cx));
                            })),
                    )
            })
            .collect();

        let empty = rows.is_empty();

        div()
            .absolute()
            .top(px(56.))
            .right(px(96.))
            .w(px(320.))
            .max_h(px(420.))
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded_lg()
            .border_1()
            .border_color(t.border)
            .bg(t.bg_subtle)
            .shadow_xl()
            // Swallow clicks so they never reach the list underneath.
            .occlude()
            .on_mouse_down_out(cx.listener(|this, _, _, cx| this.close_notifications(cx)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .py_2p5()
                    .border_b_1()
                    .border_color(t.border)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Notifications")
                            .when(unread > 0, |e| e.child(count_pill(t.accent, unread))),
                    )
                    .when(unread > 0, |e| {
                        e.child(
                            text_button(
                                "notif-mark-all",
                                "Mark all read",
                                Some("icons/check-check.svg"),
                                t.text_muted,
                                None,
                                t.bg_hover,
                            )
                            .px_1()
                            .py_0p5()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.dashboard.update(cx, |d, cx| d.mark_all_read(cx));
                            })),
                        )
                    }),
            )
            .when(empty, |e| {
                e.child(
                    div()
                        .py_10()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap_2()
                        .text_xs()
                        .text_color(t.text_muted)
                        .child(div().opacity(0.4).child(icon("icons/bell.svg", 20., t.text_muted)))
                        .child("No notifications"),
                )
            })
            .when(!empty, |e| {
                e.child(
                    div()
                        .id("notif-list")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .children(rows),
                )
            })
    }
}
