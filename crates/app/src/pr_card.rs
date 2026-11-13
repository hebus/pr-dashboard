//! One pull/merge request card (port of `PRCard.tsx`).

use std::time::Duration;

use gpui::{div, prelude::*, px, ClipboardItem, Context, FontWeight, IntoElement, SharedString};
use pr_dashboard_core::logic::{can_merge, pr_prefix, time_ago};
use pr_dashboard_core::providers::common::{Provider, PullRequest};

use crate::theme::Theme;
use crate::view::{AppView, MergeUi};
use crate::widgets::{
    avatar_with_tooltip, badge, hex_with_alpha, icon, readable_label_color, text_button, MONO,
};

const COPIED_FEEDBACK: Duration = Duration::from_millis(1500);
/// How long "Confirm?" stays armed before falling back to "Merge".
const CONFIRM_WINDOW: Duration = Duration::from_millis(4000);

fn review_badge(t: &Theme, decision: Option<&str>, is_draft: bool) -> impl IntoElement {
    if is_draft {
        return badge(t.bg_inset, t.text_muted, t.border, None, "Draft");
    }
    match decision {
        Some("APPROVED") => {
            badge(t.green_bg, t.green, t.green_border, Some("icons/git-merge.svg"), "Approved")
        }
        Some("CHANGES_REQUESTED") => {
            badge(t.red_bg, t.red, t.red_border, None, "Changes Requested")
        }
        Some("REVIEW_REQUIRED") => {
            badge(t.amber_bg, t.amber, t.amber_border, Some("icons/clock.svg"), "Pending Review")
        }
        _ => badge(t.bg_inset, t.text_muted, t.border, None, "No Review"),
    }
}

impl AppView {
    pub(crate) fn pr_card(
        &self,
        repo_id: &str,
        provider: Provider,
        pr: &PullRequest,
        t: &Theme,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let t = *t;
        let key = format!("{repo_id}-{}", pr.number);
        let decision = pr.review_decision.as_deref();
        let approved = !pr.is_draft && decision == Some("APPROVED");
        let partial = !pr.is_draft && !pr.approvers.is_empty();

        let (bg, bg_hover) = if approved {
            (t.card_approved_bg, t.card_approved_bg_hover)
        } else if partial {
            (t.card_partial_bg, t.card_partial_bg_hover)
        } else {
            (t.bg_subtle, t.bg_hover)
        };

        // The React card has a 3px left border in the review color; GPUI has a
        // single border color per element, so it is a separate strip.
        let strip = if pr.is_draft {
            t.border
        } else {
            match decision {
                Some("APPROVED") => t.green_border,
                Some("CHANGES_REQUESTED") => t.red_border,
                Some("REVIEW_REQUIRED") => t.amber_border,
                _ => t.border,
            }
        };

        let copied = self.copied.as_deref() == Some(key.as_str());
        let url = pr.url.clone();
        let branch = pr.head_ref_name.clone();
        let copy_key = key.clone();

        let copy_button = div()
            .id(SharedString::from(format!("copy-{key}")))
            .flex()
            .items_center()
            .gap_1()
            .rounded_sm()
            .px_1()
            .py_0p5()
            .cursor_pointer()
            .hover(move |s| s.bg(t.bg_inset).text_color(t.text))
            .child(pr.head_ref_name.clone())
            .child(if copied {
                div().child(icon("icons/check.svg", 10., t.green)).into_any_element()
            } else {
                div()
                    .opacity(0.)
                    .group_hover("pr-card", |s| s.opacity(0.6))
                    .child(icon("icons/copy.svg", 10., t.text_muted))
                    .into_any_element()
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                // Do not also open the PR: the whole card is clickable.
                cx.stop_propagation();
                cx.write_to_clipboard(ClipboardItem::new_string(branch.clone()));
                this.copied = Some(copy_key.clone());
                cx.notify();

                let expected = copy_key.clone();
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(COPIED_FEEDBACK).await;
                    let _ = this.update(cx, |this, cx| {
                        if this.copied.as_deref() == Some(expected.as_str()) {
                            this.copied = None;
                            cx.notify();
                        }
                    });
                })
                .detach();
            }));

        let title_row = div()
            .flex()
            .items_center()
            .gap_2()
            .flex_wrap()
            .child(
                div()
                    .text_xs()
                    .font_family(MONO)
                    .text_color(t.text_muted)
                    .child(format!("{}{}", pr_prefix(provider), pr.number)),
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(t.text)
                    .group_hover("pr-card", move |s| s.text_color(t.accent))
                    .truncate()
                    .child(pr.title.clone()),
            );

        let meta_row = div()
            .flex()
            .items_center()
            .gap_2()
            .flex_wrap()
            .mt_1()
            .text_size(px(11.))
            .text_color(t.text_muted)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .flex_none()
                    .child(avatar_with_tooltip(format!("author-{key}"), &pr.author, 16.))
                    // Never let flex squeeze the name away.
                    .child(
                        div()
                            .flex_none()
                            .whitespace_nowrap()
                            .child(pr.author.name.clone()),
                    ),
            )
            .child(div().text_color(t.border).child("·"))
            .child(time_ago(&pr.created_at))
            .child(div().text_color(t.border).child("·"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .font_family(MONO)
                    .child(pr.base_ref_name.clone())
                    .child(icon("icons/chevron-right.svg", 10., t.text_muted))
                    .child(copy_button),
            );

        let merge_button = can_merge(pr).then(|| {
            let ui = self.merge_ui.get(&key).copied();
            let label = match ui {
                None => "Merge",
                Some(MergeUi::Confirming) => "Confirm?",
                Some(MergeUi::Merging) => "Merging...",
            };
            let (bg, hover) = if ui == Some(MergeUi::Confirming) {
                (t.red, t.red)
            } else {
                (t.green_btn, t.green_btn_hover)
            };
            let (click_key, repo_id, merge_pr) = (key.clone(), repo_id.to_string(), pr.clone());
            text_button(
                SharedString::from(format!("merge-{key}")),
                label,
                Some("icons/git-merge.svg"),
                gpui::rgb(0xffffff),
                Some(bg),
                hover,
            )
            .when(ui == Some(MergeUi::Merging), |e| e.opacity(0.4))
            .on_click(cx.listener(move |this, _, _, cx| {
                // Do not also open the PR: the whole card is clickable.
                cx.stop_propagation();
                match this.merge_ui.get(&click_key).copied() {
                    Some(MergeUi::Merging) => {}
                    Some(MergeUi::Confirming) => {
                        this.merge_ui.insert(click_key.clone(), MergeUi::Merging);
                        cx.notify();
                        let (repo_id, pr) = (repo_id.clone(), merge_pr.clone());
                        this.dashboard.update(cx, |d, cx| d.merge_pr(&repo_id, &pr, cx));
                    }
                    None => {
                        this.merge_ui.insert(click_key.clone(), MergeUi::Confirming);
                        cx.notify();
                        let expected = click_key.clone();
                        cx.spawn(async move |this, cx| {
                            cx.background_executor().timer(CONFIRM_WINDOW).await;
                            let _ = this.update(cx, |this, cx| {
                                if this.merge_ui.get(&expected) == Some(&MergeUi::Confirming) {
                                    this.merge_ui.remove(&expected);
                                    cx.notify();
                                }
                            });
                        })
                        .detach();
                    }
                }
            }))
        });

        let badges = div()
            .flex_none()
            .flex()
            .flex_col()
            .items_end()
            .gap_1()
            .child(review_badge(&t, decision, pr.is_draft))
            .when(pr.has_conflicts, |e| {
                e.child(badge(
                    t.red_bg,
                    t.red,
                    t.red_border,
                    Some("icons/triangle-alert.svg"),
                    "Conflicts",
                ))
            })
            .when(pr.pipeline_failed, |e| {
                e.child(badge(
                    t.red_bg,
                    t.red,
                    t.red_border,
                    Some("icons/circle-x.svg"),
                    "Pipeline failed",
                ))
            })
            .when(pr.mergeable == Some(true), |e| {
                e.child(badge(
                    t.green_bg,
                    t.green,
                    t.green_border,
                    Some("icons/circle-check.svg"),
                    "Mergeable",
                ))
            })
            .children(merge_button);

        div()
            .id(SharedString::from(format!("pr-{key}")))
            .group("pr-card")
            .w_full()
            .flex()
            .rounded_lg()
            .overflow_hidden()
            .border_1()
            .border_color(t.border)
            .bg(bg)
            .hover(move |s| s.bg(bg_hover))
            .cursor_pointer()
            .on_click(move |_, _, cx| cx.open_url(&url))
            .child(div().w(px(3.)).flex_none().bg(strip))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .flex_1()
                                    .min_w_0()
                                    .items_start()
                                    .gap_2()
                                    .child(
                                        div()
                                            .mt_0p5()
                                            .child(icon("icons/git-pull-request.svg", 16., t.accent)),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .flex()
                                            .flex_col()
                                            .child(title_row)
                                            .child(meta_row),
                                    ),
                            )
                            .child(badges),
                    )
                    .when(!pr.labels.is_empty(), |e| {
                        e.child(
                            div().flex().flex_wrap().gap_1().ml_6().children(
                                pr.labels.iter().map(|l| {
                                    // The label's raw color is unreadable when it is
                                    // close to the background; use an adjusted one
                                    // for the text and the border.
                                    let text = readable_label_color(&l.color, t.bg_subtle);
                                    div()
                                        .px_1p5()
                                        .py_0p5()
                                        .rounded_full()
                                        .border_1()
                                        .text_size(px(10.))
                                        .font_weight(FontWeight::MEDIUM)
                                        .bg(hex_with_alpha(&l.color, 0x26))
                                        .text_color(text)
                                        .border_color(gpui::Rgba { a: 0.4, ..text })
                                        .child(l.name.clone())
                                }),
                            ),
                        )
                    })
                    .when(!pr.approvers.is_empty(), |e| {
                        e.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .ml_6()
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(t.green)
                                        .child("Approved by:"),
                                )
                                .child(div().flex().children(
                                    pr.approvers.iter().enumerate().map(|(i, r)| {
                                        div().when(i > 0, |e| e.ml(px(-4.))).child(
                                            avatar_with_tooltip(
                                                format!("approver-{key}-{}", r.login),
                                                r,
                                                18.,
                                            ),
                                        )
                                    }),
                                )),
                        )
                    }),
            )
    }
}
