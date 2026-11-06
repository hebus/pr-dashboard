mod commands;
mod providers;

#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

#[cfg(target_os = "windows")]
fn register_aumid() {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = r"SOFTWARE\Classes\AppUserModelId\com.sinequa.prdashboard";
    if let Ok((key, _)) = hkcu.create_subkey(path) {
        let _ = key.set_value("DisplayName", &"PR Dashboard");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "windows")]
    register_aumid();

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::config::get_config,
            commands::config::save_config,
            commands::fetch_prs::fetch_repo_prs,
            commands::fetch_prs::check_pr_merged,
            commands::fetch_prs::open_url,
            quit_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
