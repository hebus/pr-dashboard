//! Transient "new / merged PR" toasts (port of `ToastContainer.tsx` and `useToast`).

use std::time::Duration;

use gpui::{
    div, prelude::*, px, relative, Animation, AnimationExt, Context, FontWeight, IntoElement,
    SharedString,
};
use pr_dashboard_core::logic::{pr_prefix, pr_term, EventKind, PrEvent};

use crate::theme::Theme;
use crate::view::AppView;
use crate::widgets::icon;

pub(crate) const TOAST_DURATION: Duration = Duration::from_millis(5000);

pub(crate) struct Toast {
    pub id: u64,
    pub event: PrEvent,
}

/// "New PR" / "MR merged".
pub(crate) fn event_title(e: &PrEvent) -> String {
    let term = pr_term(e.provider);
    match e.kind {
        EventKind::NewPr => format!("New {term}"),
        EventKind::Merged => format!("{term} merged"),
    }
}

/// "#42 — Fix the thing".
pub(crate) fn event_line(e: &PrEvent) -> String {
    format!("{}{} \u{2014} {}", pr_prefix(e.provider), e.pr_number, e.pr_title)
}

impl AppView {
    pub(crate) fn add_toast(&mut self, event: PrEvent, cx: &mut Context<Self>) {
        let id = self.next_toast_id;
        self.next_toast_id += 1;
        self.toasts.push(Toast { id, event });
        cx.notify();

        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(TOAST_DURATION).await;
            let _ = this.update(cx, |this, cx| this.dismiss_toast(id, cx));
        })
        .detach();
    }

    pub(crate) fn dismiss_toast(&mut self, id: u64, cx: &mut Context<Self>) {
        self.toasts.retain(|t| t.id != id);
        cx.notify();
    }

    pub(crate) fn toast_stack(&self, t: &Theme, cx: &Context<Self>) -> impl IntoElement {
        let t = *t;
        let items = self.toasts.iter().map(|toast| {
            let id = toast.id;
            let is_new = toast.event.kind == EventKind::NewPr;
            let accent = if is_new { t.accent } else { t.green };
            let url = toast.event.url.clone();

            div()
                .id(SharedString::from(format!("toast-{id}")))
                .w(px(288.))
                .rounded_lg()
                .overflow_hidden()
                .border_1()
                .border_color(t.border)
                .bg(t.bg_subtle)
                .shadow_xl()
                .cursor_pointer()
                .hover(move |s| s.bg(t.bg_hover))
                .on_click(move |_, _, cx| {
                    if !url.is_empty() {
                        cx.open_url(&url);
                    }
                })
                .child(
                    div()
                        .p_3()
                        .flex()
                        .items_start()
                        .gap_2p5()
                        .child(div().mt_0p5().child(icon(
                            if is_new { "icons/git-pull-request.svg" } else { "icons/git-merge.svg" },
                            14.,
                            accent,
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
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(event_title(&toast.event))
                                        .child(
                                            div()
                                                .font_weight(FontWeight::NORMAL)
                                                .text_color(t.text_muted)
                                                .child(format!(" \u{b7} {}", toast.event.repo)),
                                        ),
                                )
                                .child(
                                    div()
                                        .mt_0p5()
                                        .text_xs()
                                        .text_color(t.text_muted)
                                        .truncate()
                                        .child(event_line(&toast.event)),
                                ),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!("toast-close-{id}")))
                                .mt_0p5()
                                .cursor_pointer()
                                .child(icon("icons/x.svg", 12., t.text_muted))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    // Do not also open the PR.
                                    cx.stop_propagation();
                                    this.dismiss_toast(id, cx);
                                })),
                        ),
                )
                .child(
                    // Countdown bar: shrinks from full width to nothing.
                    div().h(px(2.)).w_full().bg(t.bg_inset).child(
                        div().h_full().bg(accent).with_animation(
                            SharedString::from(format!("toast-bar-{id}")),
                            Animation::new(TOAST_DURATION),
                            |bar, delta| bar.w(relative(1.0 - delta)),
                        ),
                    ),
                )
        });

        div()
            .absolute()
            .bottom_4()
            .right_4()
            .flex()
            .flex_col()
            .items_end()
            .gap_2()
            .children(items)
    }
}
