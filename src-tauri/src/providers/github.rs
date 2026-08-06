use serde::Deserialize;

use super::common::{normalize_hex, Label, PullRequest, Reviewer};

// ── GraphQL response shapes ─────────────────────────────────────────────────

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
#[serde(rename_all = "camelCase")]
struct GqlRepository {
    pull_requests: PrConnection,
}

#[derive(Deserialize)]
struct PrConnection {
    nodes: Vec<GqlPR>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlUser {
    login: String,
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
#[serde(rename_all = "camelCase")]
struct GqlPR {
    number: u64,
    title: String,
    url: String,
    #[serde(default)]
    is_draft: bool,
    created_at: String,
    updated_at: String,
    head_ref_name: String,
    base_ref_name: String,
    review_decision: Option<String>,
    author: Option<GqlUser>,
    labels: GqlLabelConn,
    reviews: GqlReviewConn,
}

// ── GraphQL query ───────────────────────────────────────────────────────────

// `requestedReviewers` is deliberately absent: it is unavailable on some GHES
// versions and a single unknown field fails the whole query.
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

// ── Public API ──────────────────────────────────────────────────────────────

pub async fn fetch_prs(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    owner: &str,
    repo: &str,
) -> Result<Vec<PullRequest>, String> {
    let body = serde_json::json!({
        "query": PR_QUERY,
        "variables": { "owner": owner, "repo": repo, "first": 50 },
    });

    let response = client
        .post(format!("{}/api/graphql", base_url))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let gql: GraphQLResponse = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    if let Some(errors) = gql.errors {
        return Err(errors
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
            .join(", "));
    }

    Ok(gql
        .data
        .and_then(|d| d.repository)
        .map(|r| r.pull_requests.nodes.into_iter().map(map_pr).collect())
        .unwrap_or_default())
}

fn map_pr(pr: GqlPR) -> PullRequest {
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
                color: normalize_hex(&l.color),
            })
            .collect(),
        requested_reviewers: vec![],
        approvers,
    }
}

#[derive(Deserialize)]
struct RestPR {
    state: String,
    merged_at: Option<String>,
}

pub async fn check_merged(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    owner: &str,
    repo: &str,
    number: u64,
) -> Result<bool, String> {
    let url = format!(
        "{}/api/v3/repos/{}/{}/pulls/{}",
        base_url, owner, repo, number
    );

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
