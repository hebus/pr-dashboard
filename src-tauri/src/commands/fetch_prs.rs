use serde::{Deserialize, Serialize};
use tauri::command;

// ── GraphQL input/output ────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GraphQLResponse {
    data: Option<GqlData>,
    errors: Option<Vec<GqlError>>,
}

#[derive(Deserialize)]
struct GqlError {
    message: String,
}

#[derive(Deserialize)]
struct GqlData {
    repository: Option<GqlRepository>,
}

#[derive(Deserialize)]
struct GqlRepository {
    #[serde(rename = "pullRequests")]
    pull_requests: PrConnection,
}

#[derive(Deserialize)]
struct PrConnection {
    nodes: Vec<GqlPR>,
}

#[derive(Deserialize)]
struct GqlUser {
    login: String,
    #[serde(rename = "avatarUrl")]
    avatar_url: String,
}

#[derive(Deserialize)]
struct GqlLabel {
    name: String,
    color: String,
}

#[derive(Deserialize)]
struct GqlLabelConn {
    nodes: Vec<GqlLabel>,
}

#[derive(Deserialize)]
struct GqlReview {
    state: String,
    author: Option<GqlUser>,
}

#[derive(Deserialize)]
struct GqlReviewConn {
    nodes: Vec<GqlReview>,
}

#[derive(Deserialize)]
struct GqlPR {
    number: u64,
    title: String,
    url: String,
    #[serde(rename = "isDraft", default)]
    is_draft: bool,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    #[serde(rename = "headRefName")]
    head_ref_name: String,
    #[serde(rename = "baseRefName")]
    base_ref_name: String,
    #[serde(rename = "reviewDecision")]
    review_decision: Option<String>,
    author: Option<GqlUser>,
    labels: GqlLabelConn,
    reviews: GqlReviewConn,
}

// ── Public output types ─────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Reviewer {
    pub login: String,
    #[serde(rename = "avatarUrl")]
    pub avatar_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Label {
    pub name: String,
    pub color: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub url: String,
    #[serde(rename = "isDraft")]
    pub is_draft: bool,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(rename = "headRefName")]
    pub head_ref_name: String,
    #[serde(rename = "baseRefName")]
    pub base_ref_name: String,
    #[serde(rename = "reviewDecision")]
    pub review_decision: Option<String>,
    pub author: Reviewer,
    pub labels: Vec<Label>,
    #[serde(rename = "requestedReviewers")]
    pub requested_reviewers: Vec<Reviewer>,
    pub approvers: Vec<Reviewer>,
}

#[derive(Serialize, Debug)]
pub struct RepoPRs {
    pub repo: String,
    pub prs: Vec<PullRequest>,
    pub error: Option<String>,
    #[serde(rename = "lastUpdated")]
    pub last_updated: Option<String>,
}

// ── GraphQL query ───────────────────────────────────────────────────────────

const PR_QUERY: &str = r#"
query GetOpenPRs($owner: String!, $repo: String!, $first: Int!) {
  repository(owner: $owner, name: $repo) {
    pullRequests(states: [OPEN], first: $first, orderBy: {field: UPDATED_AT, direction: DESC}) {
      nodes {
        number
        title
        url
        isDraft
        createdAt
        updatedAt
        headRefName
        baseRefName
        reviewDecision
        author { login avatarUrl }
        labels(first: 10) { nodes { name color } }
        reviews(last: 20) {
          nodes {
            state
            author { login avatarUrl }
          }
        }
      }
    }
  }
}
"#;

// ── Tauri commands ──────────────────────────────────────────────────────────

fn make_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("pr-dashboard/0.1.0")
        .build()
        .map_err(|e| e.to_string())
}

#[command]
pub async fn fetch_repo_prs(
    owner: String,
    repo: String,
    token: String,
    github_url: String,
) -> Result<RepoPRs, String> {
    let repo_name = format!("{}/{}", owner, repo);

    if token.is_empty() {
        return Ok(RepoPRs {
            repo: repo_name,
            prs: vec![],
            error: Some("GitHub token not configured".to_string()),
            last_updated: None,
        });
    }

    let client = make_client()?;

    let body = serde_json::json!({
        "query": PR_QUERY,
        "variables": { "owner": owner, "repo": repo, "first": 50 },
    });

    let graphql_url = format!("{}/api/graphql", github_url.trim_end_matches('/'));

    let resp = client
        .post(&graphql_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await;

    match resp {
        Err(e) => Ok(RepoPRs {
            repo: repo_name,
            prs: vec![],
            error: Some(e.to_string()),
            last_updated: None,
        }),
        Ok(response) => {
            if !response.status().is_success() {
                return Ok(RepoPRs {
                    repo: repo_name,
                    prs: vec![],
                    error: Some(format!("HTTP {}", response.status())),
                    last_updated: None,
                });
            }

            match response.json::<GraphQLResponse>().await {
                Err(e) => Ok(RepoPRs {
                    repo: repo_name,
                    prs: vec![],
                    error: Some(format!("Parse error: {}", e)),
                    last_updated: None,
                }),
                Ok(gql) => {
                    if let Some(errors) = gql.errors {
                        let msg = errors
                            .iter()
                            .map(|e| e.message.clone())
                            .collect::<Vec<_>>()
                            .join(", ");
                        return Ok(RepoPRs {
                            repo: repo_name,
                            prs: vec![],
                            error: Some(msg),
                            last_updated: None,
                        });
                    }

                    let prs = gql
                        .data
                        .and_then(|d| d.repository)
                        .map(|r| {
                            r.pull_requests
                                .nodes
                                .into_iter()
                                .map(|pr| {
                                    let approvers: Vec<Reviewer> = pr
                                        .reviews
                                        .nodes
                                        .iter()
                                        .filter(|r| r.state == "APPROVED")
                                        .filter_map(|r| r.author.as_ref())
                                        .map(|a| Reviewer {
                                            login: a.login.clone(),
                                            avatar_url: a.avatar_url.clone(),
                                        })
                                        .collect();

                                    PullRequest {
                                        number: pr.number,
                                        title: pr.title,
                                        url: pr.url,
                                        is_draft: pr.is_draft,
                                        created_at: pr.created_at,
                                        updated_at: pr.updated_at,
                                        head_ref_name: pr.head_ref_name,
                                        base_ref_name: pr.base_ref_name,
                                        review_decision: pr.review_decision,
                                        author: pr
                                            .author
                                            .map(|a| Reviewer {
                                                login: a.login,
                                                avatar_url: a.avatar_url,
                                            })
                                            .unwrap_or_else(|| Reviewer {
                                                login: "ghost".to_string(),
                                                avatar_url: String::new(),
                                            }),
                                        labels: pr
                                            .labels
                                            .nodes
                                            .into_iter()
                                            .map(|l| Label {
                                                name: l.name,
                                                color: l.color,
                                            })
                                            .collect(),
                                        requested_reviewers: vec![],
                                        approvers,
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default();

                    let now = chrono::Utc::now().to_rfc3339();
                    Ok(RepoPRs {
                        repo: repo_name,
                        prs,
                        error: None,
                        last_updated: Some(now),
                    })
                }
            }
        }
    }
}

#[derive(Deserialize)]
struct RestPR {
    state: String,
    merged_at: Option<String>,
}

#[command]
pub async fn check_pr_merged(
    owner: String,
    repo: String,
    number: u64,
    token: String,
    github_url: String,
) -> Result<bool, String> {
    let client = make_client()?;
    let base = github_url.trim_end_matches('/');
    let url = format!("{}/api/v3/repos/{}/{}/pulls/{}", base, owner, repo, number);

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Ok(false);
    }

    let pr: RestPR = resp.json().await.map_err(|e| e.to_string())?;
    Ok(pr.state == "closed" && pr.merged_at.is_some())
}

#[command]
pub async fn open_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_url(url, None::<String>).map_err(|e| e.to_string())
}
