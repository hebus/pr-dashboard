//! Search box and status filter (port of `FilterBar.tsx`).

use gpui::{div, prelude::*, px, Context, FontWeight, IntoElement, SharedString};
use gpui_component::input::Input;
use pr_dashboard_core::logic::{FilterStatus, Totals};

use crate::theme::Theme;
use crate::view::AppView;
use crate::widgets::{count_pill, icon};

impl AppView {
    pub(crate) fn filter_bar(&self, totals: Totals, t: &Theme, cx: &Context<Self>) -> impl IntoElement {
        let t = *t;
        let filters = [
            (FilterStatus::All, "All", 0, t.amber),
            (FilterStatus::Pending, "Pending", totals.pending, t.amber),
            (FilterStatus::Approved, "Approved", totals.approved, t.green_btn),
            (FilterStatus::Mergeable, "Mergeable", totals.mergeable, t.green_btn),
            (FilterStatus::Conflicts, "Conflicts", totals.conflicts, t.red),
        ];

        let segments = filters.into_iter().map(|(status, label, count, color)| {
            let active = self.filter == status;
            div()
                .id(SharedString::from(format!("filter-{label}")))
                .flex()
                .items_center()
                .gap_1p5()
                .px_3()
                .py_1()
                .rounded_md()
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .cursor_pointer()
                .when(active, |e| e.bg(t.bg_inset).text_color(t.text))
                .when(!active, |e| e.text_color(t.text_muted).hover(move |s| s.text_color(t.text)))
                .child(label)
                .when(count > 0, |e| e.child(count_pill(color, count)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.filter = status;
                    this.rows_dirty = true;
                    cx.notify();
                }))
        });

        div()
            .flex()
            .items_center()
            .gap_3()
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(t.border)
            .bg(t.bg_subtle)
            .child(
                div().flex_1().max_w(px(384.)).child(
                    Input::new(&self.search)
                        .prefix(icon("icons/search.svg", 14., t.text_muted))
                        .cleanable(true),
                ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .p_0p5()
                    .rounded_md()
                    .border_1()
                    .border_color(t.border)
                    .bg(t.bg)
                    .children(segments),
            )
    }
}
