use tauri::command;

use crate::providers::common::{make_client, repo_error};
use crate::providers::{github, gitlab, Provider, RepoPRs};

#[command]
pub async fn fetch_repo_prs(
    provider: Option<Provider>,
    owner: String,
    repo: String,
    token: String,
    base_url: String,
) -> Result<RepoPRs, String> {
    let provider = provider.unwrap_or_default();
    let repo_name = format!("{}/{}", owner, repo);

    if token.is_empty() {
        return Ok(repo_error(
            repo_name,
            format!("{} token not configured", provider.label()),
        ));
    }

    let client = make_client()?;
    let base = base_url.trim_end_matches('/');

    let result = match provider {
        Provider::Github => github::fetch_prs(&client, base, &token, &owner, &repo).await,
        Provider::Gitlab => gitlab::fetch_prs(&client, base, &token, &owner, &repo).await,
    };

    // The frontend reads failures from `RepoPRs.error`, never from a rejected
    // promise — keep that invariant in this single place.
    Ok(match result {
        Ok(prs) => RepoPRs {
            repo: repo_name,
            prs,
            error: None,
            last_updated: Some(chrono::Utc::now().to_rfc3339()),
        },
        Err(e) => repo_error(repo_name, e),
    })
}

#[command]
pub async fn check_pr_merged(
    provider: Option<Provider>,
    owner: String,
    repo: String,
    number: u64,
    token: String,
    base_url: String,
) -> Result<bool, String> {
    if token.is_empty() {
        return Ok(false);
    }

    let client = make_client()?;
    let base = base_url.trim_end_matches('/');

    match provider.unwrap_or_default() {
        Provider::Github => github::check_merged(&client, base, &token, &owner, &repo, number).await,
        Provider::Gitlab => gitlab::check_merged(&client, base, &token, &owner, &repo, number).await,
    }
}

#[command]
pub async fn open_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(url, None::<String>)
        .map_err(|e| e.to_string())
}
