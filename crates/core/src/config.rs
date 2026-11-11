use serde::{Deserialize, Serialize};
use crate::providers::Provider;

fn default_github_url() -> String {
    "https://github.sinequa.com".to_string()
}

fn default_gitlab_url() -> String {
    "https://gitlab.chapsvision.in".to_string()
}

fn default_refresh_interval() -> u64 {
    60
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryConfig {
    pub id: String,
    /// GitHub: the owner. GitLab: the full namespace path, e.g. "sinequa/rnd".
    pub owner: String,
    pub name: String,
    pub label: Option<String>,
    #[serde(default)]
    pub provider: Provider,
}

// Every field carries a `default` on purpose: `get_config` falls back to
// `Config::default()` when deserialization fails, so a single missing field
// would silently wipe the user's token and repositories. Keeping the struct
// additive makes future fields safe too. Never add `deny_unknown_fields`.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default)]
    pub github_token: String,
    #[serde(default = "default_github_url")]
    pub github_url: String,
    #[serde(default)]
    pub gitlab_token: String,
    #[serde(default = "default_gitlab_url")]
    pub gitlab_url: String,
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u64,
    #[serde(default)]
    pub repositories: Vec<RepositoryConfig>,
    /// "dark" or "light". A plain string on purpose: an unknown value must never
    /// make the whole config fail to parse (see the note above).
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_theme() -> String {
    "dark".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            github_token: String::new(),
            github_url: default_github_url(),
            gitlab_token: String::new(),
            gitlab_url: default_gitlab_url(),
            refresh_interval: default_refresh_interval(),
            repositories: vec![],
            theme: default_theme(),
        }
    }
}

/// Same file as the Tauri build (`config.json` in the `com.sinequa.prdashboard`
/// app-data dir, shape `{ "config": {...} }`) so existing setups carry over.
pub fn config_path() -> Option<std::path::PathBuf> {
    dirs::data_dir().map(|d| d.join("com.sinequa.prdashboard").join("config.json"))
}

pub fn load_config() -> Config {
    let Some(path) = config_path() else { return Config::default() };
    let Ok(text) = std::fs::read_to_string(&path) else { return Config::default() };
    let Ok(mut root) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Config::default();
    };
    let Some(raw) = root.get("config").cloned() else { return Config::default() };
    match serde_json::from_value::<Config>(raw.clone()) {
        Ok(c) => c,
        Err(e) => {
            // Keep the unreadable value so the next save never loses it for good.
            eprintln!("[config] deserialize failed: {e} - raw value kept as 'config.broken'");
            root["config.broken"] = raw;
            let _ = std::fs::write(&path, serde_json::to_string_pretty(&root).unwrap_or_default());
            Config::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exactly the shape stored by the GitHub-only versions of the app: no
    /// `provider` on repositories, no GitLab fields at all. It must keep
    /// deserializing, or `get_config` silently wipes the user's setup.
    #[test]
    fn reads_pre_gitlab_config_without_losing_anything() {
        let legacy = serde_json::json!({
            "githubToken": "ghp_secret",
            "githubUrl": "https://github.sinequa.com",
            "refreshInterval": 60,
            "repositories": [
                { "id": "sazqcuw", "owner": "Product", "name": "sba-atomic-js", "label": null }
            ]
        });

        let config: Config = serde_json::from_value(legacy).expect("legacy config must parse");

        assert_eq!(config.github_token, "ghp_secret");
        assert_eq!(config.repositories.len(), 1);
        assert_eq!(config.repositories[0].name, "sba-atomic-js");
        assert_eq!(config.repositories[0].provider, Provider::Github);
        assert_eq!(config.gitlab_token, "");
        assert_eq!(config.gitlab_url, "https://gitlab.chapsvision.in");
    }

    #[test]
    fn reads_config_with_gitlab_repositories() {
        let stored = serde_json::json!({
            "githubToken": "ghp_secret",
            "githubUrl": "https://github.sinequa.com",
            "gitlabToken": "glpat_secret",
            "gitlabUrl": "https://gitlab.chapsvision.in",
            "refreshInterval": 30,
            "repositories": [
                { "id": "a", "provider": "gitlab", "owner": "sinequa/rnd", "name": "app", "label": null }
            ]
        });

        let config: Config = serde_json::from_value(stored).expect("config must parse");

        assert_eq!(config.repositories[0].provider, Provider::Gitlab);
        assert_eq!(config.repositories[0].owner, "sinequa/rnd");
        assert_eq!(config.gitlab_token, "glpat_secret");
    }

    #[test]
    fn unknown_fields_are_ignored_rather_than_fatal() {
        let from_a_newer_version = serde_json::json!({
            "githubToken": "ghp_secret",
            "repositories": [],
            "someFutureField": true
        });

        let config: Config = serde_json::from_value(from_a_newer_version).expect("must parse");
        assert_eq!(config.github_token, "ghp_secret");
        assert_eq!(config.refresh_interval, 60);
    }

    #[test]
    fn round_trips_through_json_in_camel_case() {
        let config = Config {
            gitlab_token: "glpat_secret".into(),
            repositories: vec![RepositoryConfig {
                id: "a".into(),
                owner: "sinequa/rnd".into(),
                name: "app".into(),
                label: None,
                provider: Provider::Gitlab,
            }],
            ..Config::default()
        };

        let json = serde_json::to_value(&config).expect("serializes");
        assert_eq!(json["gitlabToken"], "glpat_secret");
        assert_eq!(json["repositories"][0]["provider"], "gitlab");

        let back: Config = serde_json::from_value(json).expect("deserializes");
        assert_eq!(back.repositories[0].provider, Provider::Gitlab);
    }
}

pub fn save_config(config: &Config) -> Result<(), String> {
    let path = config_path().ok_or("no data dir")?;
    let mut root = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .filter(|v| v.is_object())
        .unwrap_or_else(|| serde_json::json!({}));
    root["config"] = serde_json::to_value(config).map_err(|e| e.to_string())?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

