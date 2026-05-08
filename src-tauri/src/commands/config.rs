use serde::{Deserialize, Serialize};
use tauri::command;
use tauri_plugin_store::StoreExt;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RepositoryConfig {
    pub id: String,
    pub owner: String,
    pub name: String,
    pub label: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(rename = "githubToken")]
    pub github_token: String,
    #[serde(rename = "githubUrl")]
    pub github_url: String,
    #[serde(rename = "refreshInterval")]
    pub refresh_interval: u64,
    pub repositories: Vec<RepositoryConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            github_token: String::new(),
            github_url: "https://github.sinequa.com".to_string(),
            refresh_interval: 60,
            repositories: vec![],
        }
    }
}

#[command]
pub fn get_config(app: tauri::AppHandle) -> Result<Config, String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    let config = store
        .get("config")
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    Ok(config)
}

#[command]
pub fn save_config(app: tauri::AppHandle, config: Config) -> Result<(), String> {
    let store = app.store("config.json").map_err(|e| e.to_string())?;
    store.set(
        "config",
        serde_json::to_value(&config).map_err(|e| e.to_string())?,
    );
    store.save().map_err(|e| e.to_string())
}
