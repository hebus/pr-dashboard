//! The Settings modal (port of `Settings.tsx`).
//!
//! It edits a local copy of the fields and only touches the real config on
//! Save, so Cancel discards everything.

use std::time::Duration;

use gpui::{
    div, prelude::*, px, relative, rgba, Context, Entity, EventEmitter, FontWeight, IntoElement,
    SharedString, Subscription, Window,
};
use gpui_component::input::{Input, InputEvent, InputState};
use pr_dashboard_core::config::{Config, RepositoryConfig};
use pr_dashboard_core::logic::generate_id;
use pr_dashboard_core::providers::common::Provider;

use crate::state::Dashboard;
use crate::theme::Theme;
use crate::widgets::{icon, text_button};

/// Pre-filled when a repository is switched to GitLab with an empty namespace.
const DEFAULT_GITLAB_NAMESPACE: &str = "sinequa/rnd";
const MIN_REFRESH_SECS: u64 = 10;
const SENT_FEEDBACK: Duration = Duration::from_secs(3);

pub enum SettingsEvent {
    Close,
    /// Ask the app to raise a fake "new PR" event (toast + history entry).
    TestNotification,
}

impl EventEmitter<SettingsEvent> for SettingsView {}

struct SourceEdit {
    url: Entity<InputState>,
    token: Entity<InputState>,
    show_token: bool,
}

struct RepoEdit {
    id: String,
    provider: Provider,
    owner: Entity<InputState>,
    name: Entity<InputState>,
    label: Entity<InputState>,
}

pub struct SettingsView {
    dashboard: Entity<Dashboard>,
    github: SourceEdit,
    gitlab: SourceEdit,
    interval: Entity<InputState>,
    repos: Vec<RepoEdit>,
    notification_sent: bool,
    error: Option<String>,
    _subscriptions: Vec<Subscription>,
}

fn make_input(
    window: &mut Window,
    cx: &mut Context<SettingsView>,
    value: &str,
    placeholder: &str,
    masked: bool,
) -> Entity<InputState> {
    let (value, placeholder) = (value.to_string(), placeholder.to_string());
    cx.new(|cx| {
        InputState::new(window, cx)
            .placeholder(placeholder)
            .default_value(value)
            .masked(masked)
    })
}

fn owner_placeholder(provider: Provider) -> &'static str {
    match provider {
        Provider::Gitlab => "namespace",
        Provider::Github => "owner",
    }
}

fn repo_edit(
    repo: &RepositoryConfig,
    window: &mut Window,
    cx: &mut Context<SettingsView>,
) -> RepoEdit {
    RepoEdit {
        id: repo.id.clone(),
        provider: repo.provider,
        owner: make_input(window, cx, &repo.owner, owner_placeholder(repo.provider), false),
        name: make_input(window, cx, &repo.name, "repo", false),
        label: make_input(window, cx, repo.label.as_deref().unwrap_or(""), "Label (optional)", false),
    }
}

impl SettingsView {
    pub fn new(dashboard: Entity<Dashboard>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let config = dashboard.read(cx).config.clone();

        let github = SourceEdit {
            url: make_input(window, cx, &config.github_url, "https://github.sinequa.com", false),
            token: make_input(window, cx, &config.github_token, "ghp_...", true),
            show_token: false,
        };
        let gitlab = SourceEdit {
            url: make_input(window, cx, &config.gitlab_url, "https://gitlab.chapsvision.in", false),
            token: make_input(window, cx, &config.gitlab_token, "glpat-...", true),
            show_token: false,
        };
        let interval =
            make_input(window, cx, &config.refresh_interval.to_string(), "seconds", false);
        let repos = config.repositories.iter().map(|r| repo_edit(r, window, cx)).collect();

        // The "create one at <url>/settings/tokens" hint follows the URL field.
        let subscriptions = [&github.url, &gitlab.url]
            .into_iter()
            .map(|input| {
                cx.subscribe(input, |_, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        cx.notify();
                    }
                })
            })
            .collect();

        SettingsView {
            dashboard,
            github,
            gitlab,
            interval,
            repos,
            notification_sent: false,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    // -- Actions ---------------------------------------------------------------

    fn toggle_token(&mut self, provider: Provider, window: &mut Window, cx: &mut Context<Self>) {
        let edit = match provider {
            Provider::Github => &mut self.github,
            Provider::Gitlab => &mut self.gitlab,
        };
        edit.show_token = !edit.show_token;
        let masked = !edit.show_token;
        edit.token.update(cx, |input, cx| input.set_masked(masked, window, cx));
        cx.notify();
    }

    fn add_repo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let blank = RepositoryConfig {
            id: generate_id(),
            provider: Provider::Github,
            owner: String::new(),
            name: String::new(),
            label: None,
        };
        self.repos.push(repo_edit(&blank, window, cx));
        cx.notify();
    }

    fn remove_repo(&mut self, id: &str, cx: &mut Context<Self>) {
        self.repos.retain(|r| r.id != id);
        cx.notify();
    }

    fn toggle_repo_provider(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(repo) = self.repos.iter_mut().find(|r| r.id == id) {
            repo.provider = match repo.provider {
                Provider::Github => Provider::Gitlab,
                Provider::Gitlab => Provider::Github,
            };
            let provider = repo.provider;
            // Pre-fill the namespace when switching to GitLab, but never
            // overwrite something the user already typed.
            let owner_empty = repo.owner.read(cx).value().trim().is_empty();
            repo.owner.update(cx, |input, cx| {
                input.set_placeholder(owner_placeholder(provider), window, cx);
                if provider == Provider::Gitlab && owner_empty {
                    input.set_value(DEFAULT_GITLAB_NAMESPACE, window, cx);
                }
            });
        }
        cx.notify();
    }

    fn send_test_notification(&mut self, cx: &mut Context<Self>) {
        cx.emit(SettingsEvent::TestNotification);
        self.notification_sent = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SENT_FEEDBACK).await;
            let _ = this.update(cx, |this, cx| {
                this.notification_sent = false;
                cx.notify();
            });
        })
        .detach();
    }

    /// Builds the config from the fields, starting from the live one so that
    /// settings that are not editable here (the theme) are preserved.
    fn collect_config(&self, cx: &Context<Self>) -> Config {
        let mut config = self.dashboard.read(cx).config.clone();
        let text = |input: &Entity<InputState>| input.read(cx).value().trim().to_string();
        let url = |input: &Entity<InputState>| text(input).trim_end_matches('/').to_string();

        config.github_url = url(&self.github.url);
        config.github_token = text(&self.github.token);
        config.gitlab_url = url(&self.gitlab.url);
        config.gitlab_token = text(&self.gitlab.token);
        config.refresh_interval = text(&self.interval)
            .parse::<u64>()
            .unwrap_or(config.refresh_interval)
            .max(MIN_REFRESH_SECS);

        config.repositories = self
            .repos
            .iter()
            .map(|r| RepositoryConfig {
                id: r.id.clone(),
                provider: r.provider,
                owner: text(&r.owner),
                name: text(&r.name),
                label: Some(text(&r.label)).filter(|l| !l.is_empty()),
            })
            // A row left completely blank is just an unused "Add repo".
            .filter(|r| !(r.owner.is_empty() && r.name.is_empty()))
            .collect();
        config
    }

    fn save(&mut self, cx: &mut Context<Self>) {
        let config = self.collect_config(cx);
        match self.dashboard.update(cx, |d, cx| d.apply_config(config, cx)) {
            Ok(()) => cx.emit(SettingsEvent::Close),
            Err(e) => {
                self.error = Some(format!(
                    "Applied, but the configuration could not be saved to disk: {e}"
                ));
                cx.notify();
            }
        }
    }

    // -- View pieces ------------------------------------------------------------

    fn source_section(&self, provider: Provider, t: &Theme, cx: &Context<Self>) -> impl IntoElement {
        let t = *t;
        let is_gitlab = provider == Provider::Gitlab;
        let edit = if is_gitlab { &self.gitlab } else { &self.github };
        let (name, icon_path, hint, scope, token_path) = if is_gitlab {
            (
                "GitLab",
                "icons/gitlab.svg",
                "URL de votre instance GitLab.",
                "read_api",
                "/-/user_settings/personal_access_tokens",
            )
        } else {
            (
                "GitHub",
                "icons/github.svg",
                "URL de votre instance GitHub (Enterprise ou github.com).",
                "repo",
                "/settings/tokens",
            )
        };
        let url_now = edit.url.read(cx).value().trim_end_matches('/').to_string();

        let eye = div()
            .id(SharedString::from(format!("eye-{name}")))
            .cursor_pointer()
            .child(icon(
                if edit.show_token { "icons/eye-off.svg" } else { "icons/eye.svg" },
                14.,
                t.text_muted,
            ))
            .on_click(cx.listener(move |this, _, window, cx| this.toggle_token(provider, window, cx)));

        div()
            .flex()
            .flex_col()
            .gap_3()
            .p_3()
            .rounded_lg()
            .border_1()
            .border_color(t.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(icon(icon_path, 14., t.text))
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child(name)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1p5()
                    .child(div().text_xs().text_color(t.text_muted).child(hint))
                    .child(Input::new(&edit.url)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1p5()
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_1()
                            .text_xs()
                            .text_color(t.text_muted)
                            .child("Personal Access Token with")
                            .child(
                                div().px_1().rounded_sm().bg(t.bg_inset).child(scope),
                            )
                            .child("scope. Create one at")
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(t.accent)
                                    .child(format!("{url_now}{token_path}.")),
                            ),
                    )
                    .child(Input::new(&edit.token).suffix(eye)),
            )
    }

    fn repo_row(&self, repo: &RepoEdit, t: &Theme, cx: &Context<Self>) -> impl IntoElement {
        let t = *t;
        let is_gitlab = repo.provider == Provider::Gitlab;
        let toggle_id = repo.id.clone();
        let remove_id = repo.id.clone();

        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                text_button(
                    format!("provider-{}", repo.id),
                    if is_gitlab { "GitLab" } else { "GitHub" },
                    Some(if is_gitlab { "icons/gitlab.svg" } else { "icons/github.svg" }),
                    t.text,
                    Some(t.bg),
                    t.bg_hover,
                )
                .flex_none()
                .w(px(88.))
                .border_1()
                .border_color(t.border)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.toggle_repo_provider(&toggle_id, window, cx)
                })),
            )
            .child(div().w(px(112.)).flex_none().child(Input::new(&repo.owner)))
            .child(div().text_color(t.text_muted).child("/"))
            .child(div().flex_1().min_w_0().child(Input::new(&repo.name)))
            .child(div().w(px(112.)).flex_none().child(Input::new(&repo.label)))
            .child(
                div()
                    .id(SharedString::from(format!("remove-{}", repo.id)))
                    .flex_none()
                    .cursor_pointer()
                    .child(icon("icons/trash-2.svg", 14., t.text_muted))
                    .on_click(cx.listener(move |this, _, _, cx| this.remove_repo(&remove_id, cx))),
            )
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = *cx.global::<Theme>();
        let sent = self.notification_sent;
        let repo_rows: Vec<_> =
            self.repos.iter().map(|r| self.repo_row(r, &t, cx).into_any_element()).collect();
        let no_repos = repo_rows.is_empty();

        let header = div()
            .flex()
            .items_center()
            .justify_between()
            .px_5()
            .py_4()
            .border_b_1()
            .border_color(t.border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(icon("icons/git-pull-request.svg", 16., t.text))
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("Settings"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(t.text_muted)
                            .font_family(crate::widgets::MONO)
                            .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
                    ),
            )
            .child(
                div()
                    .id("settings-close")
                    .cursor_pointer()
                    .child(icon("icons/x.svg", 18., t.text_muted))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(SettingsEvent::Close))),
            );

        let body = div()
            .id("settings-body")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .p_5()
            .flex()
            .flex_col()
            .gap_5()
            .child(self.source_section(Provider::Github, &t, cx))
            .child(self.source_section(Provider::Gitlab, &t, cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Refresh Interval"))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().w(px(96.)).child(Input::new(&self.interval)))
                            .child(div().text_sm().text_color(t.text_muted).child("seconds")),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Notifications"))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .py_2p5()
                            .rounded_md()
                            .border_1()
                            .border_color(t.border)
                            .bg(t.bg)
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(t.text_muted)
                                    .child("Toasts and history for new / merged PRs"),
                            )
                            .child(
                                text_button(
                                    "settings-test",
                                    if sent { "Sent" } else { "Test" },
                                    Some(if sent { "icons/circle-check.svg" } else { "icons/bell.svg" }),
                                    t.text,
                                    Some(t.bg_inset),
                                    t.border,
                                )
                                .on_click(cx.listener(|this, _, _, cx| this.send_test_notification(cx))),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(div().text_sm().font_weight(FontWeight::MEDIUM).child("Repositories"))
                            .child(
                                text_button(
                                    "add-repo",
                                    "Add repo",
                                    Some("icons/plus.svg"),
                                    t.accent,
                                    None,
                                    t.bg_inset,
                                )
                                .px_2()
                                .on_click(cx.listener(|this, _, window, cx| this.add_repo(window, cx))),
                            ),
                    )
                    .when(no_repos, |e| {
                        e.child(
                            div()
                                .py_2()
                                .flex()
                                .justify_center()
                                .text_xs()
                                .text_color(t.text_subtle)
                                .child("No repositories configured. Click \"Add repo\" to start."),
                        )
                    })
                    .when(!no_repos, |e| {
                        e.child(div().flex().flex_col().gap_2().children(repo_rows))
                    }),
            );

        let footer = div()
            .flex()
            .items_center()
            .justify_end()
            .gap_2()
            .px_5()
            .py_4()
            .border_t_1()
            .border_color(t.border)
            .when_some(self.error.clone(), |e, msg| {
                e.child(div().flex_1().text_xs().text_color(t.red).child(msg))
            })
            .child(
                text_button("settings-cancel", "Cancel", None, t.text_muted, None, t.bg_hover)
                    .px_4()
                    .py_2()
                    .text_sm()
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(SettingsEvent::Close))),
            )
            .child(
                text_button(
                    "settings-save",
                    "Save",
                    None,
                    gpui::rgb(0xffffff),
                    Some(t.green_btn),
                    t.green_btn_hover,
                )
                .px_4()
                .py_2()
                .text_sm()
                .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
            );

        // Dimmed backdrop; `occlude` keeps clicks and scrolling away from the
        // dashboard underneath. It covers the whole window (positioned against
        // the root view).
        div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()
            .p_4()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x0000_0099))
            .occlude()
            .child(
                div()
                    .w_full()
                    .max_w(px(672.))
                    .max_h(relative(0.9))
                    .flex()
                    .flex_col()
                    .rounded_xl()
                    .border_1()
                    .border_color(t.border)
                    .bg(t.bg_subtle)
                    .shadow_2xl()
                    .text_color(t.text)
                    .child(header)
                    .child(body)
                    .child(footer),
            )
    }
}
