//! Application state and polling. Replaces TanStack Query (`usePRData`),
//! `useConfig` and `useNotifications` of the React app.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use gpui::{Context, EventEmitter, Global, Task};
use pr_dashboard_core::config::{save_config, Config, RepositoryConfig};
use pr_dashboard_core::fetch;
use pr_dashboard_core::logic::{
    diff_prs, has_token, missing_providers, provider_creds, snapshot, totals, EventKind, PrEvent,
    PrSnapshot, SnapshotMap, Totals,
};
use pr_dashboard_core::providers::common::PullRequest;
use pr_dashboard_core::providers::common::repo_error;
use pr_dashboard_core::providers::RepoPRs;

use crate::theme::{Theme, ThemeMode};

/// reqwest needs a tokio reactor, which GPUI's own executor does not provide:
/// network futures are spawned here and awaited from GPUI tasks (a
/// `JoinHandle` is a plain future, so it works under any executor).
pub struct TokioRuntime(pub Arc<tokio::runtime::Runtime>);

impl Global for TokioRuntime {}

/// Notifications kept in the history panel.
const MAX_HISTORY: usize = 100;

/// Never poll faster than this, whatever the config says.
const MIN_REFRESH_SECS: u64 = 5;

#[derive(Default)]
pub struct RepoState {
    pub data: Option<RepoPRs>,
    pub loading: bool,
}

pub struct StoredEvent {
    pub id: u64,
    pub event: PrEvent,
    /// RFC 3339.
    pub timestamp: String,
    pub read: bool,
}

pub struct Dashboard {
    pub config: Config,
    /// Keyed by `RepositoryConfig::id`.
    pub repos: HashMap<String, RepoState>,
    pub events: Vec<StoredEvent>,
    /// Previous PR set per repo, to detect new and vanished PRs. Keyed by
    /// "provider:owner/name": two sources may host the same owner/name.
    snapshots: HashMap<String, SnapshotMap>,
    next_event_id: u64,
    /// One polling loop per repository; dropping a task cancels it.
    tasks: HashMap<String, Task<()>>,
}

impl EventEmitter<PrEvent> for Dashboard {}

/// Raised when merging a PR/MR from its card failed (the view shows a toast).
#[derive(Clone)]
pub struct MergeFailed {
    pub repo_id: String,
    pub event: PrEvent,
    pub error: String,
}

impl EventEmitter<MergeFailed> for Dashboard {}

fn snapshot_key(repo: &RepositoryConfig) -> String {
    format!("{}:{}/{}", repo.provider.label(), repo.owner, repo.name)
}

impl Dashboard {
    pub fn new(config: Config) -> Self {
        Dashboard {
            config,
            repos: HashMap::new(),
            events: Vec::new(),
            snapshots: HashMap::new(),
            next_event_id: 1,
            tasks: HashMap::new(),
        }
    }

    // -- Derived data ------------------------------------------------------------

    pub fn totals(&self) -> Totals {
        totals(
            self.repos
                .values()
                .filter_map(|r| r.data.as_ref())
                .flat_map(|d| d.prs.iter()),
        )
    }

    pub fn has_repos(&self) -> bool {
        !self.config.repositories.is_empty()
    }

    pub fn has_token(&self) -> bool {
        has_token(&self.config)
    }

    pub fn missing_providers(&self) -> Vec<&'static str> {
        missing_providers(&self.config)
    }

    pub fn unread_count(&self) -> usize {
        self.events.iter().filter(|e| !e.read).count()
    }

    pub fn theme_mode(&self) -> ThemeMode {
        ThemeMode::parse(&self.config.theme)
    }

    // -- Polling -------------------------------------------------------------------

    /// (Re)starts one polling loop per repository that has a token. Also serves
    /// as "refresh all": cancelling the old loops makes the new ones fetch
    /// immediately. Snapshots survive, so a refresh never replays events.
    pub fn start_polling(&mut self, cx: &mut Context<Self>) {
        self.tasks.clear();
        let known: Vec<String> = self.config.repositories.iter().map(|r| r.id.clone()).collect();
        self.repos.retain(|id, _| known.contains(id));

        for repo in self.config.repositories.clone() {
            let (token, base_url) = provider_creds(&self.config, &repo);
            if token.is_empty() {
                continue;
            }
            let task = self.poll_repo(repo.clone(), token.to_string(), base_url.to_string(), cx);
            self.tasks.insert(repo.id, task);
        }
    }

    /// Restarts a single repository's loop, so it fetches immediately.
    pub fn refresh_repo(&mut self, repo_id: &str, cx: &mut Context<Self>) {
        let Some(repo) = self.config.repositories.iter().find(|r| r.id == repo_id).cloned() else {
            return;
        };
        let (token, base_url) = provider_creds(&self.config, &repo);
        if token.is_empty() {
            return;
        }
        let task = self.poll_repo(repo.clone(), token.to_string(), base_url.to_string(), cx);
        self.tasks.insert(repo.id, task);
    }

    fn poll_repo(
        &self,
        repo: RepositoryConfig,
        token: String,
        base_url: String,
        cx: &mut Context<Self>,
    ) -> Task<()> {
        let runtime = cx.global::<TokioRuntime>().0.clone();
        let interval = Duration::from_secs(self.config.refresh_interval.max(MIN_REFRESH_SECS));

        cx.spawn(async move |this, cx| loop {
            if this.update(cx, |d, cx| d.set_loading(&repo.id, cx)).is_err() {
                return; // the model is gone
            }

            let fetch_repo = repo.clone();
            let (fetch_token, fetch_base) = (token.clone(), base_url.clone());
            let fetched = runtime
                .spawn(async move {
                    fetch::fetch_repo_prs(
                        fetch_repo.provider,
                        &fetch_repo.owner,
                        &fetch_repo.name,
                        &fetch_token,
                        &fetch_base,
                    )
                    .await
                })
                .await;
            // A panicked fetch must surface as a repo error, not kill the loop.
            let data = fetched.unwrap_or_else(|e| {
                repo_error(format!("{}/{}", repo.owner, repo.name), e.to_string())
            });

            if this.update(cx, |d, cx| d.apply(&repo, data, cx)).is_err() {
                return;
            }
            cx.background_executor().timer(interval).await;
        })
    }

    fn set_loading(&mut self, repo_id: &str, cx: &mut Context<Self>) {
        self.repos.entry(repo_id.to_string()).or_default().loading = true;
        cx.notify();
    }

    /// Stores a fetch result and raises events for PRs that appeared or vanished.
    fn apply(&mut self, repo: &RepositoryConfig, data: RepoPRs, cx: &mut Context<Self>) {
        let repo_name = format!("{}/{}", repo.owner, repo.name);

        // A failed fetch carries no PRs. Diffing it would report every PR as
        // vanished, then as new on the next success. (The React app did exactly
        // that; skipping failures here is deliberate.)
        if data.error.is_none() {
            let key = snapshot_key(repo);
            let diff = diff_prs(self.snapshots.get(&key), &data.prs);
            let appeared: Vec<PrEvent> = diff
                .appeared
                .iter()
                .map(|pr| PrEvent {
                    kind: EventKind::NewPr,
                    provider: repo.provider,
                    repo: repo_name.clone(),
                    pr_number: pr.number,
                    pr_title: pr.title.clone(),
                    url: pr.url.clone(),
                })
                .collect();
            let vanished = diff.disappeared;
            self.snapshots.insert(key, snapshot(&data.prs));

            for event in appeared {
                self.push_event(event, cx);
            }
            for (number, snap) in vanished {
                self.confirm_merged(repo, number, snap, cx);
            }
        }

        self.repos.insert(repo.id.clone(), RepoState { data: Some(data), loading: false });
        cx.notify();
    }

    /// Merges a PR/MR from its card, then refetches the repo at once: on success
    /// the PR vanishes and `confirm_merged` raises the "merged" event. On failure
    /// a `MergeFailed` is emitted.
    pub fn merge_pr(&mut self, repo_id: &str, pr: &PullRequest, cx: &mut Context<Self>) {
        let Some(repo) = self.config.repositories.iter().find(|r| r.id == repo_id).cloned() else {
            return;
        };
        let (token, base_url) = provider_creds(&self.config, &repo);
        let (token, base_url) = (token.to_string(), base_url.to_string());
        let runtime = cx.global::<TokioRuntime>().0.clone();
        let (number, title, url) = (pr.number, pr.title.clone(), pr.url.clone());
        let repo_id = repo_id.to_string();

        cx.spawn(async move |this, cx| {
            let (provider, owner, name) = (repo.provider, repo.owner.clone(), repo.name.clone());
            let result = runtime
                .spawn(async move {
                    fetch::merge_pr(provider, &owner, &name, number, &token, &base_url).await
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()));
            let _ = this.update(cx, |d, cx| {
                if let Err(error) = result {
                    cx.emit(MergeFailed {
                        repo_id: repo_id.clone(),
                        event: PrEvent {
                            kind: EventKind::Merged,
                            provider: repo.provider,
                            repo: format!("{}/{}", repo.owner, repo.name),
                            pr_number: number,
                            pr_title: title,
                            url,
                        },
                        error,
                    });
                }
                d.refresh_repo(&repo_id, cx);
            });
        })
        .detach();
    }

    /// A vanished PR may have been merged or merely closed: only a merge raises
    /// an event. Check failures are ignored, as in the React app.
    fn confirm_merged(
        &mut self,
        repo: &RepositoryConfig,
        number: u64,
        snap: PrSnapshot,
        cx: &mut Context<Self>,
    ) {
        let (token, base_url) = provider_creds(&self.config, repo);
        let (token, base_url) = (token.to_string(), base_url.to_string());
        let runtime = cx.global::<TokioRuntime>().0.clone();
        let repo = repo.clone();

        cx.spawn(async move |this, cx| {
            let (provider, owner, name) = (repo.provider, repo.owner.clone(), repo.name.clone());
            let merged = runtime
                .spawn(async move {
                    fetch::check_pr_merged(provider, &owner, &name, number, &token, &base_url).await
                })
                .await;
            if let Ok(Ok(true)) = merged {
                let event = PrEvent {
                    kind: EventKind::Merged,
                    provider: repo.provider,
                    repo: format!("{}/{}", repo.owner, repo.name),
                    pr_number: number,
                    pr_title: snap.title,
                    url: snap.url,
                };
                let _ = this.update(cx, |d, cx| d.push_event(event, cx));
            }
        })
        .detach();
    }

    // -- Notifications ---------------------------------------------------------------

    /// Records an event in the history and emits it (the view shows a toast).
    pub fn push_event(&mut self, event: PrEvent, cx: &mut Context<Self>) {
        let id = self.next_event_id;
        self.next_event_id += 1;
        self.events.insert(
            0,
            StoredEvent {
                id,
                event: event.clone(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                read: false,
            },
        );
        // The React history grows without bound; a long-running dashboard
        // should not.
        self.events.truncate(MAX_HISTORY);
        cx.emit(event);
        cx.notify();
    }

    pub fn mark_read(&mut self, id: u64, cx: &mut Context<Self>) {
        if let Some(e) = self.events.iter_mut().find(|e| e.id == id) {
            e.read = true;
            cx.notify();
        }
    }

    pub fn mark_all_read(&mut self, cx: &mut Context<Self>) {
        self.events.iter_mut().for_each(|e| e.read = true);
        cx.notify();
    }

    pub fn remove_event(&mut self, id: u64, cx: &mut Context<Self>) {
        self.events.retain(|e| e.id != id);
        cx.notify();
    }

    // -- Config ------------------------------------------------------------------------

    /// Persists the config. Saving never restarts polling by itself.
    pub fn save(&self) -> Result<(), String> {
        save_config(&self.config)
    }

    /// Applies a config edited in Settings: the new repositories and tokens
    /// take effect immediately, even if writing the file fails (the error is
    /// returned so the dialog can tell the user it will not survive a restart).
    pub fn apply_config(&mut self, config: Config, cx: &mut Context<Self>) -> Result<(), String> {
        self.config = config;
        let saved = self.save();
        self.start_polling(cx);
        cx.notify();
        saved
    }

    pub fn toggle_theme(&mut self, cx: &mut Context<Self>) {
        let mode = self.theme_mode().toggled();
        self.config.theme = mode.as_str().to_string();
        cx.set_global(Theme::for_mode(mode));
        if let Err(e) = self.save() {
            eprintln!("[config] could not save theme: {e}");
        }
        cx.notify();
    }
}
