//! Repository groups (port of `RepoGroup.tsx`), as rows of a virtualized list.
//!
//! Rendering every card of every repository each frame is what made scrolling
//! crawl with many PRs. The groups are flattened into `Row`s and drawn by
//! `gpui::list`, which only builds the rows currently on screen. Each group is
//! visually reassembled from its rows: a rounded header, body rows carrying the
//! side borders, and a last body row carrying the bottom border.

use std::collections::HashMap;

use gpui::{div, prelude::*, px, AnyElement, Context, FontWeight, IntoElement, SharedString};
use pr_dashboard_core::logic::{
    branch_kind, compare_branches, matches_filter, matches_search, pr_term, provider_creds,
    repo_web_url, time_ago, BranchKind,
};
use pr_dashboard_core::providers::common::{Provider, PullRequest};

use crate::theme::Theme;
use crate::view::AppView;
use crate::widgets::{badge, icon};

/// Widths of the three placeholder cards shown during the first fetch.
const SKELETON_WIDTHS: [f32; 3] = [160., 120., 210.];

/// Vertical gap between two groups.
const GROUP_GAP: f32 = 12.;

#[derive(Clone, Debug)]
pub(crate) enum BodyContent {
    Error(String),
    Skeleton(usize),
    NoMatch,
    BranchHeader { name: String, count: usize },
    /// Index into the repository's (unfiltered) PR list.
    Card { pr_idx: usize },
}

#[derive(Clone, Debug)]
pub(crate) enum Row {
    Header { repo_idx: usize, collapsed: bool },
    Body { repo_idx: usize, first: bool, last: bool, content: BodyContent },
    Spacer,
}

/// Static placeholder card shown while the first fetch is running. (The React
/// version pulses; GPUI animations come later.)
fn skeleton_card(t: &Theme, title_width: f32) -> impl IntoElement {
    let bar = |w: f32, h: f32| div().w(px(w)).h(px(h)).rounded_sm().bg(t.bg_inset);
    div()
        .w_full()
        .flex()
        .rounded_lg()
        .overflow_hidden()
        .border_1()
        .border_color(t.border)
        .bg(t.bg_subtle)
        .child(div().w(px(3.)).flex_none().bg(t.border))
        .child(
            div()
                .flex_1()
                .p_3()
                .flex()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(div().flex().gap_2().child(bar(28., 12.)).child(bar(title_width, 14.)))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().size(px(16.)).rounded_full().bg(t.bg_inset))
                                .child(bar(64., 10.))
                                .child(bar(40., 10.))
                                .child(bar(112., 10.)),
                        ),
                )
                .child(div().w(px(80.)).h(px(20.)).rounded_full().bg(t.bg_inset)),
        )
}

impl AppView {
    /// Flattens the repositories into list rows and tells the list. Uses
    /// `splice` rather than `reset`, which would jump back to the top on every
    /// polling refresh.
    pub(crate) fn rebuild_rows(&mut self, cx: &Context<Self>) {
        let query = self.search.read(cx).value().to_string();
        let d = self.dashboard.read(cx);
        let mut rows: Vec<Row> = Vec::new();

        for (repo_idx, repo) in d.config.repositories.iter().enumerate() {
            let collapsed = self.collapsed.contains(&repo.id);
            rows.push(Row::Header { repo_idx, collapsed });

            if !collapsed {
                let state = d.repos.get(&repo.id);
                let data = state.and_then(|s| s.data.as_ref());
                let prs: &[PullRequest] = data.map_or(&[], |d| d.prs.as_slice());
                let is_loading = state.is_some_and(|s| s.loading);

                let mut body: Vec<BodyContent> = Vec::new();
                if let Some(msg) = data.and_then(|d| d.error.clone()) {
                    body.push(BodyContent::Error(msg));
                } else if is_loading && prs.is_empty() {
                    body.extend((0..SKELETON_WIDTHS.len()).map(BodyContent::Skeleton));
                } else {
                    let filtered: Vec<(usize, &PullRequest)> = prs
                        .iter()
                        .enumerate()
                        .filter(|(_, pr)| {
                            matches_filter(pr, self.filter) && matches_search(pr, &query)
                        })
                        .collect();

                    if filtered.is_empty() {
                        body.push(BodyContent::NoMatch);
                    } else {
                        let mut by_branch: HashMap<&str, Vec<usize>> = HashMap::new();
                        for (idx, pr) in &filtered {
                            by_branch.entry(pr.base_ref_name.as_str()).or_default().push(*idx);
                        }
                        let mut branches: Vec<(&str, Vec<usize>)> = by_branch.into_iter().collect();
                        branches.sort_by(|a, b| compare_branches(a.0, b.0));

                        let grouped = branches.len() > 1;
                        for (name, idxs) in branches {
                            if grouped {
                                body.push(BodyContent::BranchHeader {
                                    name: name.to_string(),
                                    count: idxs.len(),
                                });
                            }
                            body.extend(idxs.into_iter().map(|pr_idx| BodyContent::Card { pr_idx }));
                        }
                    }
                }

                let last_ix = body.len().saturating_sub(1);
                for (i, content) in body.into_iter().enumerate() {
                    rows.push(Row::Body { repo_idx, first: i == 0, last: i == last_ix, content });
                }
            }
            rows.push(Row::Spacer);
        }

        let old_len = self.rows.len();
        self.rows = rows;
        self.list_state.splice(0..old_len, self.rows.len());
    }

    /// Builds one list row. Every row has the same definite width, centered and
    /// capped, so cards can never end up with different widths.
    pub(crate) fn render_row(&mut self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let t = *cx.global::<Theme>();
        let Some(row) = self.rows.get(ix).cloned() else {
            return div().into_any_element();
        };

        let inner: AnyElement = match row {
            Row::Spacer => div().h(px(GROUP_GAP)).into_any_element(),
            Row::Header { repo_idx, collapsed } => self.group_header(repo_idx, collapsed, &t, cx),
            Row::Body { repo_idx, first, last, content } => {
                self.body_row(repo_idx, first, last, content, &t, cx)
            }
        };

        div()
            .w_full()
            .flex()
            .justify_center()
            .px_4()
            .when(ix == 0, |e| e.pt_4())
            .child(div().w_full().max_w(px(896.)).child(inner))
            .into_any_element()
    }

    fn group_header(
        &self,
        repo_idx: usize,
        collapsed: bool,
        t: &Theme,
        cx: &Context<Self>,
    ) -> AnyElement {
        let t = *t;
        let d = self.dashboard.read(cx);
        let Some(repo) = d.config.repositories.get(repo_idx) else {
            return div().into_any_element();
        };
        let state = d.repos.get(&repo.id);
        let data = state.and_then(|s| s.data.as_ref());
        let prs: &[PullRequest] = data.map_or(&[], |d| d.prs.as_slice());
        let error = data.and_then(|d| d.error.clone());
        let last_updated = data.and_then(|d| d.last_updated.clone());
        let is_loading = state.is_some_and(|s| s.loading);
        let (token, base_url) = provider_creds(&d.config, repo);
        let has_token = !token.is_empty();
        let web_url = repo_web_url(base_url, repo);

        let approved_count =
            prs.iter().filter(|p| p.review_decision.as_deref() == Some("APPROVED")).count();
        let pending_count = prs.len() - approved_count;
        let label = repo
            .label
            .clone()
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| format!("{}/{}", repo.owner, repo.name));
        let term = pr_term(repo.provider);
        let provider_icon = match repo.provider {
            Provider::Gitlab => "icons/gitlab.svg",
            Provider::Github => "icons/github.svg",
        };

        let toggle_id = repo.id.clone();
        let refresh_id = repo.id.clone();
        let can_refresh = has_token && !is_loading;

        div()
            .id(SharedString::from(format!("group-{}", repo.id)))
            .w_full()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .py_3()
            .border_1()
            .border_color(t.border)
            .bg(t.bg_subtle)
            .when(collapsed, |e| e.rounded_lg())
            .when(!collapsed, |e| e.rounded_t_lg())
            .hover(move |s| s.bg(t.bg_hover))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                if !this.collapsed.remove(&toggle_id) {
                    this.collapsed.insert(toggle_id.clone());
                }
                this.rows_dirty = true;
                cx.notify();
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(icon(
                        if collapsed { "icons/chevron-right.svg" } else { "icons/chevron-down.svg" },
                        16.,
                        t.text_muted,
                    ))
                    .child(icon(provider_icon, 13., t.text_muted))
                    .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(label))
                    .when(error.is_some(), |e| e.child(icon("icons/circle-alert.svg", 14., t.red)))
                    .when(is_loading && error.is_none(), |e| {
                        e.child(icon("icons/refresh-cw.svg", 12., t.text_muted))
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .text_size(px(11.))
                    .when(error.is_none() && pending_count > 0, |e| {
                        e.child(
                            div()
                                .flex()
                                .gap_1()
                                .text_color(t.amber)
                                .child(
                                    div()
                                        .font_weight(FontWeight::BOLD)
                                        .child(pending_count.to_string()),
                                )
                                .child("pending"),
                        )
                    })
                    .when(error.is_none() && approved_count > 0, |e| {
                        e.child(
                            div()
                                .flex()
                                .gap_1()
                                .text_color(t.green)
                                .child(
                                    div()
                                        .font_weight(FontWeight::BOLD)
                                        .child(approved_count.to_string()),
                                )
                                .child("approved"),
                        )
                    })
                    .when(error.is_none() && prs.is_empty() && !is_loading, |e| {
                        e.child(div().text_color(t.text_muted).child(format!("No open {term}s")))
                    })
                    .when_some(last_updated, |e, at| {
                        e.child(
                            div().text_size(px(10.)).text_color(t.text_subtle).child(time_ago(&at)),
                        )
                    })
                    .child(
                        div()
                            .id(SharedString::from(format!("open-{}", repo.id)))
                            .cursor_pointer()
                            .p_0p5()
                            .child(icon("icons/external-link.svg", 13., t.text_muted))
                            .on_click(move |_, _, cx| {
                                cx.stop_propagation();
                                cx.open_url(&web_url);
                            }),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("refresh-{}", repo.id)))
                            .p_0p5()
                            .when(can_refresh, |e| {
                                e.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| {
                                    cx.stop_propagation();
                                    this.dashboard
                                        .update(cx, |d, cx| d.refresh_repo(&refresh_id, cx));
                                }))
                            })
                            .when(!can_refresh, |e| e.opacity(0.4))
                            .child(icon("icons/refresh-cw.svg", 13., t.text_muted)),
                    ),
            )
            .into_any_element()
    }

    fn body_row(
        &self,
        repo_idx: usize,
        first: bool,
        last: bool,
        content: BodyContent,
        t: &Theme,
        cx: &Context<Self>,
    ) -> AnyElement {
        let t = *t;
        let d = self.dashboard.read(cx);
        let Some(repo) = d.config.repositories.get(repo_idx) else {
            return div().into_any_element();
        };

        let inner: AnyElement = match content {
            BodyContent::Error(msg) => div()
                .flex()
                .items_center()
                .gap_2()
                .p_3()
                .rounded_md()
                .border_1()
                .border_color(t.red_border)
                .bg(t.red_bg)
                .text_sm()
                .text_color(t.red)
                .child(icon("icons/circle-alert.svg", 14., t.red))
                .child(msg)
                .into_any_element(),
            BodyContent::Skeleton(i) => div()
                .when(!last, |e| e.pb_2())
                .child(skeleton_card(&t, SKELETON_WIDTHS[i % SKELETON_WIDTHS.len()]))
                .into_any_element(),
            BodyContent::NoMatch => div()
                .py_3()
                .flex()
                .justify_center()
                .text_sm()
                .text_color(t.text_muted)
                .child(format!(
                    "No {} requests match the current filter.",
                    if repo.provider == Provider::Gitlab { "merge" } else { "pull" }
                ))
                .into_any_element(),
            BodyContent::BranchHeader { name, count } => {
                let (bg, fg, border) = match branch_kind(&name) {
                    BranchKind::Develop => (t.indigo_bg, t.indigo, t.indigo_border),
                    BranchKind::Release => (t.violet_bg, t.violet, t.violet_border),
                    BranchKind::Other => (t.bg_inset, t.text_muted, t.border),
                };
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .px_1()
                    .pb_1p5()
                    // 12px between two branch sections; the card above already adds 8px.
                    .when(!first, |e| e.pt_1())
                    .child(badge(bg, fg, border, Some("icons/git-branch.svg"), name))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(t.text_subtle)
                            .child(format!("· {count}")),
                    )
                    .into_any_element()
            }
            BodyContent::Card { pr_idx } => {
                let pr = d
                    .repos
                    .get(&repo.id)
                    .and_then(|s| s.data.as_ref())
                    .and_then(|data| data.prs.get(pr_idx));
                match pr {
                    Some(pr) => div()
                        .when(!last, |e| e.pb_2())
                        .child(self.pr_card(&repo.id, repo.provider, pr, &t, cx))
                        .into_any_element(),
                    None => div().into_any_element(),
                }
            }
        };

        div()
            .w_full()
            .px_3()
            .border_l_1()
            .border_r_1()
            .border_color(t.border)
            .bg(t.bg)
            .when(first, |e| e.pt_3())
            .when(last, |e| e.pb_3().border_b_1().rounded_b_lg())
            .child(inner)
            .into_any_element()
    }
}
