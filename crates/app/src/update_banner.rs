//! "A new version is available" banner (port of `UpdateBanner.tsx`).

use gpui::{div, prelude::*, rgb, AnyElement, Context, IntoElement, SharedString};

use crate::theme::Theme;
use crate::updater::UpdaterStatus;
use crate::view::AppView;
use crate::widgets::{icon, text_button};

impl AppView {
    /// `None` when there is nothing to show (no update, or dismissed).
    pub(crate) fn update_banner(&self, t: &Theme, cx: &Context<Self>) -> Option<AnyElement> {
        let t = *t;
        let u = self.updater.read(cx);
        if u.status == UpdaterStatus::Idle || u.dismissed {
            return None;
        }

        let installing = u.status == UpdaterStatus::Installing;
        let version = u.version.clone().unwrap_or_default();
        let message: SharedString = if installing {
            match u.progress {
                Some(p) => format!("Downloading version {version} \u{2014} {p}%"),
                None => format!("Downloading version {version}\u{2026}"),
            }
        } else if u.status == UpdaterStatus::Error {
            format!(
                "Update to {version} failed: {}",
                u.error.as_deref().unwrap_or("unknown error")
            )
        } else {
            format!("Version {version} is available.")
        }
        .into();
        let is_error = u.status == UpdaterStatus::Error;

        Some(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .px_4()
                .py_2()
                .border_b_1()
                .border_color(t.indigo_border)
                .bg(t.indigo_bg)
                .text_color(t.indigo)
                .text_xs()
                .child(
                    div()
                        .flex()
                        .flex_1()
                        .min_w_0()
                        .items_center()
                        .gap_2()
                        .child(icon(
                            if installing { "icons/refresh-cw.svg" } else { "icons/download.svg" },
                            13.,
                            t.indigo,
                        ))
                        .child(div().truncate().child(message)),
                )
                .when(!installing, |e| {
                    e.child(
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .gap_2()
                            .child(
                                text_button(
                                    "update-install",
                                    if is_error { "Retry" } else { "Install and restart" },
                                    None,
                                    rgb(0xffffff),
                                    Some(t.indigo),
                                    t.indigo_border,
                                )
                                .px_2p5()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.updater.update(cx, |u, cx| u.install(cx));
                                })),
                            )
                            .child(
                                div()
                                    .id("update-later")
                                    .p_1()
                                    .rounded_sm()
                                    .cursor_pointer()
                                    .hover(move |s| s.bg(t.indigo_border))
                                    .child(icon("icons/x.svg", 13., t.indigo))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.updater.update(cx, |u, cx| u.dismiss(cx));
                                    })),
                            ),
                    )
                })
                .into_any_element(),
        )
    }
}
