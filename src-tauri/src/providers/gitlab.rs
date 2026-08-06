use serde::Deserialize;

use super::common::{absolutize, normalize_hex, Label, PullRequest, Reviewer};

// ── GraphQL response shapes ─────────────────────────────────────────────────

#[derive(Deserialize)]
struct GqlResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GqlError>>,
}

#[derive(Deserialize)]
struct GqlError {
    message: String,
}

#[derive(Deserialize)]
struct MrListData {
    project: Option<GqlProject>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlProject {
    merge_requests: Conn<GqlMr>,
}

#[derive(Deserialize)]
struct Conn<T> {
    nodes: Vec<T>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlUser {
    username: String,
    avatar_url: Option<String>,
    /// Only present on `reviewers` nodes, never on `author` / `approvedBy`.
    merge_request_interaction: Option<GqlInteraction>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlInteraction {
    review_state: Option<String>,
}

#[derive(Deserialize)]
struct GqlLabel {
    title: String,
    color: String,
}

// Every connection below is Option: GitLab returns null instead of an empty
// connection when the token lacks permission on that sub-resource, and a
// missing Option would drop the whole repository.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlMr {
    iid: String,
    title: String,
    web_url: String,
    #[serde(default)]
    draft: bool,
    created_at: String,
    updated_at: String,
    source_branch: String,
    target_branch: String,
    #[serde(default)]
    approved: bool,
    author: Option<GqlUser>,
    labels: Option<Conn<GqlLabel>>,
    approved_by: Option<Conn<GqlUser>>,
    reviewers: Option<Conn<GqlUser>>,
}

#[derive(Deserialize)]
struct MrStateData {
    project: Option<GqlProjectMr>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GqlProjectMr {
    merge_request: Option<GqlMrState>,
}

#[derive(Deserialize)]
struct GqlMrState {
    state: String,
}

// ── GraphQL queries ─────────────────────────────────────────────────────────

// `state: opened` is lowercase and `sort: UPDATED_DESC` is uppercase — that
// asymmetry is the actual GitLab schema. Every nested connection is bounded so
// the query stays well under the complexity ceiling (250 when authenticated).
const MR_QUERY: &str = r#"
query GetOpenMRs($fullPath: ID!, $first: Int!) {
  project(fullPath: $fullPath) {
    mergeRequests(state: opened, sort: UPDATED_DESC, first: $first) {
      nodes {
        iid
        title
        webUrl
        draft
        createdAt
        updatedAt
        sourceBranch
        targetBranch
        approved
        author { username avatarUrl }
        labels(first: 10) { nodes { title color } }
        approvedBy(first: 20) { nodes { username avatarUrl } }
        reviewers(first: 20) {
          nodes {
            username
            avatarUrl
            mergeRequestInteraction { reviewState }
          }
        }
      }
    }
  }
}
"#;

const MR_STATE_QUERY: &str = r#"
query MrState($fullPath: ID!, $iid: String!) {
  project(fullPath: $fullPath) {
    mergeRequest(iid: $iid) { state }
  }
}
"#;

// ── Public API ──────────────────────────────────────────────────────────────

/// `owner` carries the full namespace path (e.g. "sinequa/rnd"), so the
/// project full path is simply "{owner}/{repo}".
pub async fn fetch_prs(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    owner: &str,
    repo: &str,
) -> Result<Vec<PullRequest>, String> {
    let body = serde_json::json!({
        "query": MR_QUERY,
        "variables": { "fullPath": format!("{}/{}", owner, repo), "first": 50 },
    });

    let data: MrListData = post_graphql(client, base_url, token, &body).await?;

    let project = data
        .project
        .ok_or_else(|| format!("Project '{}/{}' not found or not accessible", owner, repo))?;

    Ok(project
        .merge_requests
        .nodes
        .into_iter()
        .filter_map(|mr| map_mr(base_url, mr))
        .collect())
}

pub async fn check_merged(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    owner: &str,
    repo: &str,
    number: u64,
) -> Result<bool, String> {
    let body = serde_json::json!({
        "query": MR_STATE_QUERY,
        "variables": {
            "fullPath": format!("{}/{}", owner, repo),
            "iid": number.to_string(),
        },
    });

    let data: MrStateData = post_graphql(client, base_url, token, &body).await?;

    Ok(data
        .project
        .and_then(|p| p.merge_request)
        .map(|mr| mr.state == "merged")
        .unwrap_or(false))
}

async fn post_graphql<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    body: &serde_json::Value,
) -> Result<T, String> {
    let response = client
        .post(format!("{}/api/graphql", base_url))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }

    let gql: GqlResponse<T> = response
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    if let Some(errors) = gql.errors {
        if !errors.is_empty() {
            return Err(errors
                .iter()
                .map(|e| e.message.clone())
                .collect::<Vec<_>>()
                .join(", "));
        }
    }

    gql.data.ok_or_else(|| "Empty GraphQL response".to_string())
}

// ── Mapping ─────────────────────────────────────────────────────────────────

fn user_nodes(conn: &Option<Conn<GqlUser>>) -> &[GqlUser] {
    conn.as_ref().map(|c| c.nodes.as_slice()).unwrap_or(&[])
}

fn map_mr(base_url: &str, mr: GqlMr) -> Option<PullRequest> {
    // `iid` is a String in the GitLab schema. Drop the MR rather than coercing
    // to 0: the frontend keys its new/merged diff map by `number`.
    let number: u64 = mr.iid.parse().ok()?;

    let to_reviewer = |u: &GqlUser| Reviewer {
        login: u.username.clone(),
        avatar_url: absolutize(base_url, u.avatar_url.as_deref()),
    };
    let changes_requested = user_nodes(&mr.reviewers).iter().any(|r| {
        r.merge_request_interaction
            .as_ref()
            .and_then(|i| i.review_state.as_deref())
            == Some("REQUESTED_CHANGES")
    });

    // `approved` means "all *required* approvals are in". A 1-of-2 MR stays
    // REVIEW_REQUIRED while still exposing its approvers, which is what drives
    // the partially-approved card background — same semantics as GitHub.
    let review_decision = Some(
        if changes_requested {
            "CHANGES_REQUESTED"
        } else if mr.approved {
            "APPROVED"
        } else {
            "REVIEW_REQUIRED"
        }
        .to_string(),
    );

    Some(PullRequest {
        number,
        title: mr.title,
        url: mr.web_url,
        is_draft: mr.draft,
        created_at: mr.created_at,
        updated_at: mr.updated_at,
        head_ref_name: mr.source_branch,
        base_ref_name: mr.target_branch,
        review_decision,
        author: mr
            .author
            .as_ref()
            .map(to_reviewer)
            .unwrap_or_else(|| Reviewer {
                login: "ghost".to_string(),
                avatar_url: String::new(),
            }),
        labels: mr
            .labels
            .map(|c| {
                c.nodes
                    .into_iter()
                    .map(|l| Label {
                        name: l.title,
                        color: normalize_hex(&l.color),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        requested_reviewers: user_nodes(&mr.reviewers).iter().map(to_reviewer).collect(),
        approvers: user_nodes(&mr.approved_by).iter().map(to_reviewer).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "https://gitlab.chapsvision.in";

    fn mr_from(json: serde_json::Value) -> Option<PullRequest> {
        map_mr(BASE, serde_json::from_value::<GqlMr>(json).expect("fixture must parse"))
    }

    fn fixture() -> serde_json::Value {
        serde_json::json!({
            "iid": "412",
            "title": "Fix login redirect",
            "webUrl": "https://gitlab.chapsvision.in/sinequa/rnd/app/-/merge_requests/412",
            "draft": false,
            "createdAt": "2026-08-01T09:12:33Z",
            "updatedAt": "2026-08-05T14:02:10Z",
            "sourceBranch": "fix/login-redirect",
            "targetBranch": "main",
            "approved": false,
            "author": { "username": "otome", "avatarUrl": "/uploads/-/system/user/avatar/42/a.png" },
            "labels": { "nodes": [{ "title": "bug", "color": "#d9534f" }] },
            "approvedBy": { "nodes": [] },
            "reviewers": { "nodes": [] }
        })
    }

    #[test]
    fn maps_core_fields_and_parses_string_iid() {
        let pr = mr_from(fixture()).expect("should map");
        assert_eq!(pr.number, 412);
        assert_eq!(pr.head_ref_name, "fix/login-redirect");
        assert_eq!(pr.base_ref_name, "main");
        assert_eq!(pr.author.login, "otome");
        assert!(!pr.is_draft);
    }

    #[test]
    fn strips_hash_from_label_color() {
        let pr = mr_from(fixture()).expect("should map");
        assert_eq!(pr.labels[0].color, "d9534f");
        assert_eq!(pr.labels[0].name, "bug");
    }

    #[test]
    fn absolutizes_relative_avatar() {
        let pr = mr_from(fixture()).expect("should map");
        assert_eq!(
            pr.author.avatar_url,
            "https://gitlab.chapsvision.in/uploads/-/system/user/avatar/42/a.png"
        );
    }

    #[test]
    fn review_decision_pending_when_not_approved() {
        let pr = mr_from(fixture()).expect("should map");
        assert_eq!(pr.review_decision.as_deref(), Some("REVIEW_REQUIRED"));
    }

    #[test]
    fn review_decision_approved_when_all_required_approvals_in() {
        let mut json = fixture();
        json["approved"] = serde_json::json!(true);
        let pr = mr_from(json).expect("should map");
        assert_eq!(pr.review_decision.as_deref(), Some("APPROVED"));
    }

    #[test]
    fn review_decision_changes_requested_wins_over_approved() {
        let mut json = fixture();
        json["approved"] = serde_json::json!(true);
        json["reviewers"]["nodes"] = serde_json::json!([{
            "username": "reviewer1",
            "avatarUrl": null,
            "mergeRequestInteraction": { "reviewState": "REQUESTED_CHANGES" }
        }]);
        let pr = mr_from(json).expect("should map");
        assert_eq!(pr.review_decision.as_deref(), Some("CHANGES_REQUESTED"));
        assert_eq!(pr.requested_reviewers.len(), 1);
        assert_eq!(pr.requested_reviewers[0].avatar_url, "");
    }

    #[test]
    fn partial_approval_keeps_approvers_but_stays_pending() {
        let mut json = fixture();
        json["approvedBy"]["nodes"] = serde_json::json!([
            { "username": "alice", "avatarUrl": "https://cdn.example.com/a.png" }
        ]);
        let pr = mr_from(json).expect("should map");
        assert_eq!(pr.review_decision.as_deref(), Some("REVIEW_REQUIRED"));
        assert_eq!(pr.approvers.len(), 1);
        assert_eq!(pr.approvers[0].avatar_url, "https://cdn.example.com/a.png");
    }

    #[test]
    fn survives_null_connections_and_missing_author() {
        let mut json = fixture();
        json["author"] = serde_json::Value::Null;
        json["labels"] = serde_json::Value::Null;
        json["approvedBy"] = serde_json::Value::Null;
        json["reviewers"] = serde_json::Value::Null;
        let pr = mr_from(json).expect("should still map");
        assert_eq!(pr.author.login, "ghost");
        assert!(pr.labels.is_empty());
        assert!(pr.approvers.is_empty());
        assert!(pr.requested_reviewers.is_empty());
    }

    #[test]
    fn drops_merge_request_with_unparseable_iid() {
        let mut json = fixture();
        json["iid"] = serde_json::json!("not-a-number");
        assert!(mr_from(json).is_none());
    }
}
