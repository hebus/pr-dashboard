use serde::{Deserialize, Serialize};

// ── Provider ────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    Github,
    Gitlab,
}

impl Provider {
    pub fn label(&self) -> &'static str {
        match self {
            Provider::Github => "GitHub",
            Provider::Gitlab => "GitLab",
        }
    }
}

// ── Public output types (shared by every provider) ──────────────────────────

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Reviewer {
    pub login: String,
    pub avatar_url: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Label {
    pub name: String,
    pub color: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub is_draft: bool,
    pub created_at: String,
    pub updated_at: String,
    pub head_ref_name: String,
    pub base_ref_name: String,
    pub review_decision: Option<String>,
    pub author: Reviewer,
    pub labels: Vec<Label>,
    pub requested_reviewers: Vec<Reviewer>,
    pub approvers: Vec<Reviewer>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RepoPRs {
    pub repo: String,
    pub prs: Vec<PullRequest>,
    pub error: Option<String>,
    pub last_updated: Option<String>,
}

// ── Helpers ─────────────────────────────────────────────────────────────────

pub fn make_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("pr-dashboard/0.1.0")
        .build()
        .map_err(|e| e.to_string())
}

pub fn repo_error(repo: String, msg: String) -> RepoPRs {
    RepoPRs {
        repo,
        prs: vec![],
        error: Some(msg),
        last_updated: None,
    }
}

/// The frontend renders label colors as `#${hex}` (PRCard.tsx), so the hex must
/// never carry its own '#'. GitHub returns "1d76db", GitLab returns "#1d76db"
/// and sometimes the 3-digit form "#fff".
pub fn normalize_hex(color: &str) -> String {
    let c = color.trim().trim_start_matches('#');
    let valid = |s: &str| s.chars().all(|ch| ch.is_ascii_hexdigit());
    match c.len() {
        3 if valid(c) => c.chars().flat_map(|ch| [ch, ch]).collect::<String>().to_ascii_lowercase(),
        6 if valid(c) => c.to_ascii_lowercase(),
        _ => "8a8a8a".to_string(),
    }
}

/// GitLab avatars can be relative ("/uploads/-/system/user/avatar/42/avatar.png")
/// or protocol-relative ("//www.gravatar.com/avatar/…").
pub fn absolutize(base: &str, url: Option<&str>) -> String {
    match url.map(str::trim).filter(|u| !u.is_empty()) {
        None => String::new(),
        Some(u) if u.starts_with("http://") || u.starts_with("https://") => u.to_string(),
        Some(u) if u.starts_with("//") => format!("https:{}", u),
        Some(u) => format!("{}/{}", base.trim_end_matches('/'), u.trim_start_matches('/')),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_hex_strips_leading_hash() {
        assert_eq!(normalize_hex("#1D76DB"), "1d76db");
        assert_eq!(normalize_hex("1d76db"), "1d76db");
    }

    #[test]
    fn normalize_hex_expands_short_form() {
        assert_eq!(normalize_hex("#fff"), "ffffff");
    }

    #[test]
    fn normalize_hex_falls_back_on_garbage() {
        assert_eq!(normalize_hex("rebeccapurple"), "8a8a8a");
        assert_eq!(normalize_hex(""), "8a8a8a");
    }

    #[test]
    fn absolutize_handles_every_shape() {
        let base = "https://gitlab.example.com";
        assert_eq!(absolutize(base, None), "");
        assert_eq!(absolutize(base, Some("")), "");
        assert_eq!(
            absolutize(base, Some("https://cdn.example.com/a.png")),
            "https://cdn.example.com/a.png"
        );
        assert_eq!(
            absolutize(base, Some("//gravatar.com/avatar/x")),
            "https://gravatar.com/avatar/x"
        );
        assert_eq!(
            absolutize(base, Some("/uploads/avatar.png")),
            "https://gitlab.example.com/uploads/avatar.png"
        );
    }
}
