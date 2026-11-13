use crate::providers::common::{make_client, repo_error};
use crate::providers::{github, gitlab, Provider, RepoPRs};

/// Failures are reported through `RepoPRs.error`, never as an `Err`:
/// keep that invariant in this single place.
pub async fn fetch_repo_prs(
    provider: Provider,
    owner: &str,
    repo: &str,
    token: &str,
    base_url: &str,
) -> RepoPRs {
    let repo_name = format!("{}/{}", owner, repo);

    if token.is_empty() {
        return repo_error(repo_name, format!("{} token not configured", provider.label()));
    }
    let client = match make_client() {
        Ok(c) => c,
        Err(e) => return repo_error(repo_name, e),
    };
    let base = base_url.trim_end_matches('/');

    let result = match provider {
        Provider::Github | Provider::GithubCom => github::fetch_prs(&client, base, token, owner, repo).await,
        Provider::Gitlab => gitlab::fetch_prs(&client, base, token, owner, repo).await,
    };

    match result {
        Ok(prs) => RepoPRs {
            repo: repo_name,
            prs,
            error: None,
            last_updated: Some(chrono::Utc::now().to_rfc3339()),
        },
        Err(e) => repo_error(repo_name, e),
    }
}

pub async fn check_pr_merged(
    provider: Provider,
    owner: &str,
    repo: &str,
    number: u64,
    token: &str,
    base_url: &str,
) -> Result<bool, String> {
    if token.is_empty() {
        return Ok(false);
    }
    let client = make_client()?;
    let base = base_url.trim_end_matches('/');
    match provider {
        Provider::Github | Provider::GithubCom => github::check_merged(&client, base, token, owner, repo, number).await,
        Provider::Gitlab => gitlab::check_merged(&client, base, token, owner, repo, number).await,
    }
}

/// Merges a PR/MR with the repository's default method. Needs a write-capable token.
pub async fn merge_pr(
    provider: Provider,
    owner: &str,
    repo: &str,
    number: u64,
    token: &str,
    base_url: &str,
) -> Result<(), String> {
    if token.is_empty() {
        return Err(format!("{} token not configured", provider.label()));
    }
    let client = make_client()?;
    let base = base_url.trim_end_matches('/');
    match provider {
        Provider::Github | Provider::GithubCom => github::merge_pr(&client, base, token, owner, repo, number).await,
        Provider::Gitlab => gitlab::merge_mr(&client, base, token, owner, repo, number).await,
    }
}
