//! Pure UI-independent logic ported from `src/types/index.ts` and
//! `src/hooks/usePRData.ts` of the Tauri/React app.

use std::cmp::Ordering;
use std::collections::HashMap;

use crate::config::{Config, RepositoryConfig};
use crate::providers::common::{Provider, PullRequest};

// -- Provider-dependent helpers ------------------------------------------------

/// Token and base URL of the source a given repository belongs to.
pub fn provider_creds<'a>(config: &'a Config, repo: &RepositoryConfig) -> (&'a str, &'a str) {
    match repo.provider {
        Provider::Gitlab => (&config.gitlab_token, &config.gitlab_url),
        Provider::Github => (&config.github_token, &config.github_url),
        Provider::GithubCom => (&config.github_com_token, GITHUB_COM_URL),
    }
}

/// Web URL of github.com, the fixed "instance" of the `GithubCom` provider.
pub const GITHUB_COM_URL: &str = "https://github.com";

pub fn repo_web_url(base_url: &str, repo: &RepositoryConfig) -> String {
    match repo.provider {
        Provider::Gitlab => format!("{base_url}/{}/{}/-/merge_requests", repo.owner, repo.name),
        Provider::Github | Provider::GithubCom => {
            format!("{base_url}/{}/{}/pulls", repo.owner, repo.name)
        }
    }
}

/// "PR" on GitHub, "MR" on GitLab.
pub fn pr_term(provider: Provider) -> &'static str {
    match provider {
        Provider::Gitlab => "MR",
        Provider::Github | Provider::GithubCom => "PR",
    }
}

/// GitLab writes merge request numbers as !123, GitHub as #123.
pub fn pr_prefix(provider: Provider) -> &'static str {
    match provider {
        Provider::Gitlab => "!",
        Provider::Github | Provider::GithubCom => "#",
    }
}

/// Short random id (7 base-36 characters) for new repository entries, like the
/// React app's `generateId`. Only needs to be unique within one config.
pub fn generate_id() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    // A fresh RandomState carries fresh random keys: hashing nothing is enough.
    let mut n = RandomState::new().build_hasher().finish();
    (0..7)
        .map(|_| {
            let c = ALPHABET[(n % 36) as usize] as char;
            n /= 36;
            c
        })
        .collect()
}

// -- Filtering -----------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FilterStatus {
    #[default]
    All,
    Pending,
    Approved,
    Mergeable,
    Conflicts,
}

pub fn matches_filter(pr: &PullRequest, filter: FilterStatus) -> bool {
    let approved = pr.review_decision.as_deref() == Some("APPROVED");
    match filter {
        FilterStatus::All => true,
        FilterStatus::Approved => approved,
        FilterStatus::Pending => !approved,
        FilterStatus::Mergeable => pr.mergeable == Some(true),
        FilterStatus::Conflicts => pr.has_conflicts,
    }
}

/// Text search of the filter bar: title, author (login or display name) or PR
/// number, case-insensitive. An empty query matches everything.
/// The merge button is offered only when nothing blocks the merge.
pub fn can_merge(pr: &PullRequest) -> bool {
    pr.mergeable == Some(true) && !pr.is_draft && !pr.has_conflicts
}

pub fn matches_search(pr: &PullRequest, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let q = query.to_lowercase();
    pr.title.to_lowercase().contains(&q)
        || pr.author.login.to_lowercase().contains(&q)
        || pr.author.name.to_lowercase().contains(&q)
        || pr.number.to_string().contains(&q)
}

// -- Avatars (initials only: the React app never rendered the remote image) ----

/// "Olivier Tome" -> "OT" (first letter of the first and last words);
/// a single word such as "alice" -> "AL" (its first two characters).
pub fn avatar_initials(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let initials: String = match words.as_slice() {
        [] => String::new(),
        [one] => one.chars().take(2).collect(),
        [first, .., last] => {
            first.chars().take(1).chain(last.chars().take(1)).collect()
        }
    };
    initials.to_uppercase()
}

/// Stable palette index for a login: `hash = hash * 31 + code_unit` over the
/// UTF-16 code units, wrapping on 32 bits, like the JS implementation.
pub fn avatar_color_index(login: &str, palette_len: usize) -> usize {
    let hash = login
        .encode_utf16()
        .fold(0u32, |h, unit| h.wrapping_mul(31).wrapping_add(unit as u32));
    hash as usize % palette_len.max(1)
}

// -- Branch ordering -----------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchKind {
    Develop,
    Release,
    Other,
}

pub fn branch_kind(branch: &str) -> BranchKind {
    if branch == "develop" {
        BranchKind::Develop
    } else if branch.starts_with("release/") {
        BranchKind::Release
    } else {
        BranchKind::Other
    }
}

fn release_version_key(branch: &str) -> Vec<i64> {
    branch["release/".len()..]
        .split(['.', '-', '_'])
        .map(|part| {
            // Mirrors JS parseInt: leading digits, otherwise -1.
            let digits: String = part.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse().unwrap_or(-1)
        })
        .collect()
}

fn kind_order(k: BranchKind) -> u8 {
    match k {
        BranchKind::Develop => 0,
        BranchKind::Release => 1,
        BranchKind::Other => 2,
    }
}

/// develop first, then release/x by version descending, then the rest alphabetically.
pub fn compare_branches(a: &str, b: &str) -> Ordering {
    let (ka, kb) = (branch_kind(a), branch_kind(b));
    if ka != kb {
        return kind_order(ka).cmp(&kind_order(kb));
    }
    if ka == BranchKind::Release {
        let (va, vb) = (release_version_key(a), release_version_key(b));
        for i in 0..va.len().max(vb.len()) {
            let (x, y) = (va.get(i).copied().unwrap_or(0), vb.get(i).copied().unwrap_or(0));
            if x != y {
                return y.cmp(&x);
            }
        }
        return Ordering::Equal;
    }
    a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b))
}

// -- Relative time -------------------------------------------------------------

pub fn time_ago_secs(seconds: i64) -> String {
    match seconds {
        s if s < 60 => "just now".into(),
        s if s < 3_600 => format!("{}m ago", s / 60),
        s if s < 86_400 => format!("{}h ago", s / 3_600),
        s if s < 604_800 => format!("{}d ago", s / 86_400),
        s => format!("{}w ago", s / 604_800),
    }
}

/// `date` is an RFC 3339 timestamp; unparseable input yields an empty string.
pub fn time_ago(date: &str) -> String {
    match chrono::DateTime::parse_from_rfc3339(date) {
        Ok(d) => time_ago_secs((chrono::Utc::now() - d.with_timezone(&chrono::Utc)).num_seconds()),
        Err(_) => String::new(),
    }
}

// -- PR change detection (port of usePRData's diff) ---------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    NewPr,
    Merged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrEvent {
    pub kind: EventKind,
    pub provider: Provider,
    /// "owner/name", shown as-is in toasts and history.
    pub repo: String,
    pub pr_number: u64,
    pub pr_title: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrSnapshot {
    pub title: String,
    pub url: String,
}

pub type SnapshotMap = HashMap<u64, PrSnapshot>;

pub fn snapshot(prs: &[PullRequest]) -> SnapshotMap {
    prs.iter()
        .map(|pr| (pr.number, PrSnapshot { title: pr.title.clone(), url: pr.url.clone() }))
        .collect()
}

pub struct Diff<'a> {
    /// PRs present now and not before.
    pub appeared: Vec<&'a PullRequest>,
    /// PRs that vanished: the caller must confirm with `check_pr_merged`
    /// (closed without merge must not raise a "merged" event).
    pub disappeared: Vec<(u64, PrSnapshot)>,
}

/// `prev == None` is the first load: nothing is reported, so startup does not
/// raise a notification per existing PR.
pub fn diff_prs<'a>(prev: Option<&SnapshotMap>, current: &'a [PullRequest]) -> Diff<'a> {
    let Some(prev) = prev else {
        return Diff { appeared: vec![], disappeared: vec![] };
    };
    let now = snapshot(current);
    let appeared = current.iter().filter(|pr| !prev.contains_key(&pr.number)).collect();
    let mut disappeared: Vec<_> = prev
        .iter()
        .filter(|(n, _)| !now.contains_key(n))
        .map(|(n, s)| (*n, s.clone()))
        .collect();
    disappeared.sort_by_key(|(n, _)| *n);
    Diff { appeared, disappeared }
}

// -- Header totals and empty states (port of App.tsx) ------------------------

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Totals {
    pub pending: usize,
    pub approved: usize,
    pub mergeable: usize,
    pub conflicts: usize,
}

pub fn totals<'a>(prs: impl IntoIterator<Item = &'a PullRequest>) -> Totals {
    let mut t = Totals::default();
    for pr in prs {
        if pr.review_decision.as_deref() == Some("APPROVED") {
            t.approved += 1;
        } else {
            t.pending += 1;
        }
        if pr.mergeable == Some(true) {
            t.mergeable += 1;
        }
        if pr.has_conflicts {
            t.conflicts += 1;
        }
    }
    t
}

/// Enough to work with as soon as one configured repo has a token for its own
/// source: a missing token on the other source only affects its own group.
pub fn has_token(config: &Config) -> bool {
    config.repositories.iter().any(|r| !provider_creds(config, r).0.is_empty())
}

/// Sources that have repositories but no token, in first-seen order.
pub fn missing_providers(config: &Config) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = vec![];
    for r in &config.repositories {
        if provider_creds(config, r).0.is_empty() && !out.contains(&r.provider.label()) {
            out.push(r.provider.label());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::common::Reviewer;

    #[test]
    fn search_matches_title_author_or_number() {
        let mut p = pr(42, None);
        p.title = "Fix Login bug".into();
        p.author = Reviewer::new("alice42x".into(), Some("Alice Martin".into()), String::new());
        assert!(matches_search(&p, ""));
        assert!(matches_search(&p, "login"));
        assert!(matches_search(&p, "ALICE4")); // login
        assert!(matches_search(&p, "martin")); // display name
        assert!(matches_search(&p, "42"));
        assert!(!matches_search(&p, "bob"));
    }

    #[test]
    fn generated_ids_are_short_alphanumeric_and_distinct() {
        let a = generate_id();
        let b = generate_id();
        assert_eq!(a.len(), 7);
        assert!(a.chars().all(|c| c.is_ascii_digit() || c.is_ascii_lowercase()));
        assert_ne!(a, b);
    }

    #[test]
    fn reviewer_name_falls_back_to_login() {
        assert_eq!(Reviewer::new("otome".into(), None, String::new()).name, "otome");
        assert_eq!(Reviewer::new("otome".into(), Some("  ".into()), String::new()).name, "otome");
        let r = Reviewer::new("otome".into(), Some(" Olivier Tome ".into()), String::new());
        assert_eq!(r.name, "Olivier Tome");
        assert_eq!(r.full_label(), "Olivier Tome (otome)");
        assert_eq!(Reviewer::ghost().full_label(), "ghost");
    }

    #[test]
    fn avatar_helpers_match_the_js_implementation() {
        assert_eq!(avatar_initials("alice"), "AL");
        assert_eq!(avatar_initials("x"), "X");
        assert_eq!(avatar_initials("Olivier Tome"), "OT");
        assert_eq!(avatar_initials("jean-pierre de la Fontaine"), "JF");
        assert_eq!(avatar_initials("  "), "");
        // "ab": ('a'*31 + 'b') = 97*31 + 98 = 3105; 3105 % 8 = 1
        assert_eq!(avatar_color_index("ab", 8), 1);
        assert_eq!(avatar_color_index("", 8), 0);
        // Long logins must wrap on 32 bits instead of overflowing.
        let _ = avatar_color_index(&"z".repeat(200), 8);
    }

    #[test]
    fn totals_and_token_helpers() {
        let mut mergeable = pr(1, Some("APPROVED"));
        mergeable.mergeable = Some(true);
        let mut conflicted = pr(2, Some("REVIEW_REQUIRED"));
        conflicted.has_conflicts = true;
        let prs = [mergeable, conflicted, pr(3, None)];
        assert_eq!(
            totals(&prs),
            Totals { pending: 2, approved: 1, mergeable: 1, conflicts: 1 }
        );

        let repo = |id: &str, provider| RepositoryConfig {
            id: id.into(),
            owner: "o".into(),
            name: "n".into(),
            label: None,
            provider,
        };
        let mut cfg = Config {
            github_token: "gh".into(),
            repositories: vec![repo("a", Provider::Gitlab), repo("b", Provider::Github)],
            ..Config::default()
        };
        assert!(has_token(&cfg));
        assert_eq!(missing_providers(&cfg), ["GitLab"]);
        cfg.github_token.clear();
        assert!(!has_token(&cfg));
        assert_eq!(missing_providers(&cfg), ["GitLab", "GitHub"]);
    }

    fn pr(number: u64, decision: Option<&str>) -> PullRequest {
        PullRequest {
            number,
            title: format!("PR {number}"),
            url: format!("https://x/{number}"),
            is_draft: false,
            created_at: String::new(),
            updated_at: String::new(),
            head_ref_name: "f".into(),
            base_ref_name: "develop".into(),
            review_decision: decision.map(Into::into),
            mergeable: None,
            has_conflicts: false,
            pipeline_failed: false,
            author: Reviewer::new("a".into(), None, String::new()),
            labels: vec![],
            requested_reviewers: vec![],
            approvers: vec![],
        }
    }

    #[test]
    fn can_merge_requires_clean_non_draft() {
        let mut p = pr(1, None);
        assert!(!can_merge(&p)); // unknown
        p.mergeable = Some(true);
        assert!(can_merge(&p));
        p.is_draft = true;
        assert!(!can_merge(&p));
        p.is_draft = false;
        p.has_conflicts = true;
        assert!(!can_merge(&p));
        p.has_conflicts = false;
        p.mergeable = Some(false);
        assert!(!can_merge(&p));
    }

    #[test]
    fn filters() {
        let approved = pr(1, Some("APPROVED"));
        let pending = pr(2, Some("REVIEW_REQUIRED"));
        let mut conflicted = pr(3, None);
        conflicted.has_conflicts = true;
        let mut mergeable = pr(4, None);
        mergeable.mergeable = Some(true);

        assert!(matches_filter(&approved, FilterStatus::Approved));
        assert!(!matches_filter(&approved, FilterStatus::Pending));
        assert!(matches_filter(&pending, FilterStatus::Pending));
        assert!(matches_filter(&pr(5, None), FilterStatus::Pending));
        assert!(matches_filter(&conflicted, FilterStatus::Conflicts));
        assert!(!matches_filter(&pending, FilterStatus::Conflicts));
        assert!(matches_filter(&mergeable, FilterStatus::Mergeable));
        assert!(!matches_filter(&pr(6, None), FilterStatus::Mergeable));
        assert!(matches_filter(&pending, FilterStatus::All));
    }

    #[test]
    fn branch_ordering() {
        let mut v = vec![
            "feature/b",
            "release/1.9",
            "develop",
            "release/1.10",
            "Alpha",
            "release/2.0-rc1",
        ];
        v.sort_by(|a, b| compare_branches(a, b));
        assert_eq!(
            v,
            ["develop", "release/2.0-rc1", "release/1.10", "release/1.9", "Alpha", "feature/b"]
        );
    }

    #[test]
    fn release_versions_compare_numerically_not_lexically() {
        assert_eq!(compare_branches("release/1.10", "release/1.9"), Ordering::Less);
        assert_eq!(compare_branches("release/1.2", "release/1.2.0"), Ordering::Equal);
    }

    #[test]
    fn relative_time_buckets() {
        assert_eq!(time_ago_secs(59), "just now");
        assert_eq!(time_ago_secs(60), "1m ago");
        assert_eq!(time_ago_secs(3_599), "59m ago");
        assert_eq!(time_ago_secs(3_600), "1h ago");
        assert_eq!(time_ago_secs(86_400), "1d ago");
        assert_eq!(time_ago_secs(604_800), "1w ago");
        assert_eq!(time_ago("not a date"), "");
    }

    #[test]
    fn first_load_reports_nothing() {
        let current = [pr(1, None)];
        let d = diff_prs(None, &current);
        assert!(d.appeared.is_empty() && d.disappeared.is_empty());
    }

    #[test]
    fn diff_detects_new_and_vanished() {
        let prev = snapshot(&[pr(1, None), pr(2, None)]);
        let current = [pr(2, None), pr(3, None)];
        let d = diff_prs(Some(&prev), &current);
        assert_eq!(d.appeared.iter().map(|p| p.number).collect::<Vec<_>>(), [3]);
        assert_eq!(d.disappeared.len(), 1);
        assert_eq!(d.disappeared[0].0, 1);
        assert_eq!(d.disappeared[0].1.title, "PR 1");
    }

    #[test]
    fn creds_follow_the_repo_provider() {
        let cfg = Config {
            github_token: "gh".into(),
            gitlab_token: "gl".into(),
            ..Config::default()
        };
        let mk = |provider| RepositoryConfig {
            id: "i".into(),
            owner: "o".into(),
            name: "n".into(),
            label: None,
            provider,
        };
        assert_eq!(provider_creds(&cfg, &mk(Provider::Gitlab)).0, "gl");
        assert_eq!(provider_creds(&cfg, &mk(Provider::Github)).0, "gh");
        assert_eq!(
            repo_web_url("https://h", &mk(Provider::Gitlab)),
            "https://h/o/n/-/merge_requests"
        );
        assert_eq!(pr_prefix(Provider::Gitlab), "!");
        assert_eq!(pr_term(Provider::Github), "PR");
    }
}
