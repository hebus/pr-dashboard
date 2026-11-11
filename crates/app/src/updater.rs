//! Self-update from GitHub Releases (port of `useUpdater.ts`).
//!
//! Checks 5 seconds after start and every 6 hours. Nothing is downloaded until
//! the user clicks "Install and restart". On Windows the downloaded NSIS
//! installer is verified against the minisign public key below, then launched
//! in passive mode and relaunches the app; this process exits.
//!
//! `cargo-packager-updater` is blocking (reqwest's blocking client), so every
//! call runs on GPUI's background executor, never on the UI thread.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use cargo_packager_updater::semver::Version;
use cargo_packager_updater::url::Url;
use cargo_packager_updater::{
    Config, Update, UpdaterBuilder, WindowsConfig, WindowsUpdateInstallMode,
};
use gpui::{Context, Task};

const FIRST_CHECK_DELAY: Duration = Duration::from_secs(5);
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const PROGRESS_POLL: Duration = Duration::from_millis(200);

/// Same endpoint and key as the Tauri build, so installations of either build
/// find this app's releases.
const DEFAULT_ENDPOINT: &str =
    "https://github.com/hebus/pr-dashboard/releases/latest/download/latest.json";
const PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEY5MjRBRTQwRDUzNTdBQkUKUldTK2VqWFZRSzRrK2J6aGN2b0FpV0pjKzdWdVovbWpzZC9TN0FzcWJqa1RrbVcwZXh2b2FsazkK";

/// Points the updater at another manifest, to try an update end to end against
/// a local server. The signature is still verified, so this cannot be used to
/// install anything that was not signed with the key above.
const ENDPOINT_ENV: &str = "PR_DASHBOARD_UPDATE_URL";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdaterStatus {
    Idle,
    Available,
    Installing,
    Error,
}

pub struct Updater {
    pub status: UpdaterStatus,
    pub version: Option<String>,
    /// 0..=100 while downloading, `None` until the size is known.
    pub progress: Option<u8>,
    pub error: Option<String>,
    /// The banner was closed with "Later". A newer release re-opens it.
    pub dismissed: bool,
    update: Option<Update>,
    schedule: Option<Task<()>>,
}

/// `received / total` as a percentage; `None` while the total is unknown.
pub fn progress_percent(received: u64, total: u64) -> Option<u8> {
    if total == 0 {
        return None;
    }
    let pct = (received as f64 / total as f64 * 100.0).round();
    Some(pct.clamp(0.0, 100.0) as u8)
}

fn endpoint(override_url: Option<&str>) -> Result<Url, String> {
    override_url
        .filter(|u| !u.trim().is_empty())
        .unwrap_or(DEFAULT_ENDPOINT)
        .trim()
        .parse()
        .map_err(|e| format!("invalid update endpoint: {e}"))
}

fn updater_config() -> Result<Config, String> {
    let override_url = std::env::var(ENDPOINT_ENV).ok();
    Ok(Config {
        endpoints: vec![endpoint(override_url.as_deref())?],
        pubkey: PUBKEY.to_string(),
        windows: Some(WindowsConfig {
            install_mode: Some(WindowsUpdateInstallMode::Passive),
            installer_args: None,
        }),
    })
}

fn current_version() -> Version {
    env!("CARGO_PKG_VERSION").parse().expect("CARGO_PKG_VERSION is valid semver")
}

/// Blocking: asks the endpoint whether a newer release exists.
fn check_blocking() -> Result<Option<Update>, String> {
    UpdaterBuilder::new(current_version(), updater_config()?)
        .build()
        .map_err(|e| e.to_string())?
        .check()
        .map_err(|e| e.to_string())
}

impl Updater {
    pub fn new() -> Self {
        Updater {
            status: UpdaterStatus::Idle,
            version: None,
            progress: None,
            error: None,
            dismissed: false,
            update: None,
            schedule: None,
        }
    }

    /// Starts the periodic check. A debug build only checks when the endpoint
    /// is overridden: a developer's `cargo run` must not offer to replace
    /// itself with the installed release.
    pub fn start(&mut self, cx: &mut Context<Self>) {
        if cfg!(debug_assertions) && std::env::var_os(ENDPOINT_ENV).is_none() {
            return;
        }
        self.schedule = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(FIRST_CHECK_DELAY).await;
            loop {
                if this.update(cx, |u, cx| u.check_now(cx)).is_err() {
                    return; // the model is gone
                }
                cx.background_executor().timer(CHECK_INTERVAL).await;
            }
        }));
    }

    fn busy(&self) -> bool {
        self.status == UpdaterStatus::Installing
    }

    fn check_now(&mut self, cx: &mut Context<Self>) {
        if self.busy() {
            return;
        }
        cx.spawn(async move |this, cx| {
            let result = cx.background_executor().spawn(async { check_blocking() }).await;
            let _ = this.update(cx, |u, cx| u.finish_check(result, cx));
        })
        .detach();
    }

    fn finish_check(&mut self, result: Result<Option<Update>, String>, cx: &mut Context<Self>) {
        match result {
            Ok(Some(update)) if !self.busy() => {
                // A newer release than the one already announced re-opens the banner.
                if self.version.as_deref() != Some(update.version.as_str()) {
                    self.dismissed = false;
                }
                self.version = Some(update.version.clone());
                self.update = Some(update);
                self.status = UpdaterStatus::Available;
                cx.notify();
            }
            Ok(_) => {}
            // Offline, no release yet, endpoint down... never worth bothering
            // the user: the next scheduled check tries again.
            Err(e) => eprintln!("[updater] check failed: {e}"),
        }
    }

    pub fn dismiss(&mut self, cx: &mut Context<Self>) {
        self.dismissed = true;
        cx.notify();
    }

    /// Downloads, verifies and launches the installer. On success the process
    /// exits (the installer relaunches the app), so only failures come back.
    pub fn install(&mut self, cx: &mut Context<Self>) {
        let Some(update) = self.update.clone() else { return };
        if self.busy() {
            return;
        }
        self.status = UpdaterStatus::Installing;
        self.progress = None;
        self.error = None;
        cx.notify();

        let received = Arc::new(AtomicU64::new(0));
        let total = Arc::new(AtomicU64::new(0));
        let done = Arc::new(AtomicBool::new(false));

        let job = {
            let (received, total, done) = (received.clone(), total.clone(), done.clone());
            cx.background_executor().spawn(async move {
                let result = update.download_and_install_extended(
                    |chunk, content_length| {
                        received.fetch_add(chunk as u64, Ordering::Relaxed);
                        if let Some(len) = content_length {
                            total.store(len, Ordering::Relaxed);
                        }
                    },
                    || {},
                );
                done.store(true, Ordering::Release);
                result.map_err(|e| e.to_string())
            })
        };

        cx.spawn(async move |this, cx| {
            while !done.load(Ordering::Acquire) {
                cx.background_executor().timer(PROGRESS_POLL).await;
                let pct = progress_percent(received.load(Ordering::Relaxed), total.load(Ordering::Relaxed));
                if this
                    .update(cx, |u, cx| {
                        u.progress = pct;
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
            }
            if let Err(e) = job.await {
                let _ = this.update(cx, |u, cx| {
                    u.status = UpdaterStatus::Error;
                    u.error = Some(e);
                    cx.notify();
                });
            }
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cargo_packager_updater::RemoteRelease;

    #[test]
    fn progress_is_a_clamped_percentage() {
        assert_eq!(progress_percent(0, 0), None);
        assert_eq!(progress_percent(0, 1000), Some(0));
        assert_eq!(progress_percent(500, 1000), Some(50));
        assert_eq!(progress_percent(1000, 1000), Some(100));
        // More bytes than announced (compressed transfer, bad Content-Length).
        assert_eq!(progress_percent(2000, 1000), Some(100));
    }

    #[test]
    fn endpoint_defaults_to_the_github_release_manifest() {
        assert_eq!(endpoint(None).unwrap().as_str(), DEFAULT_ENDPOINT);
        assert_eq!(endpoint(Some("  ")).unwrap().as_str(), DEFAULT_ENDPOINT);
        assert_eq!(
            endpoint(Some("http://127.0.0.1:8000/latest.json")).unwrap().as_str(),
            "http://127.0.0.1:8000/latest.json"
        );
        assert!(endpoint(Some("not a url")).is_err());
    }

    #[test]
    fn the_embedded_public_key_decodes_as_minisign() {
        // Same key as tauri.conf.json: base64 of a two-line minisign public key file.
        let decoded = String::from_utf8(
            base64_decode(PUBKEY).expect("valid base64"),
        )
        .expect("utf-8");
        assert!(decoded.starts_with("untrusted comment: minisign public key"), "{decoded}");
        assert!(decoded.lines().count() >= 2);
    }

    /// Minimal standard-alphabet base64 decoder, enough for the test above
    /// without adding a dependency.
    fn base64_decode(s: &str) -> Option<Vec<u8>> {
        const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = Vec::new();
        let (mut acc, mut bits) = (0u32, 0u32);
        for c in s.bytes().filter(|c| *c != b'=') {
            let v = T.iter().position(|t| *t == c)? as u32;
            acc = (acc << 6) | v;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((acc >> bits) as u8);
                acc &= (1 << bits) - 1;
            }
        }
        Some(out)
    }

    /// The manifest produced by `scripts/make_latest_json.py` must be readable by
    /// this app's updater.
    #[test]
    fn the_generated_manifest_is_understood_by_the_updater() {
        let json = include_str!("../tests/fixtures/latest.json");
        let release: RemoteRelease = serde_json::from_str(json).expect("manifest parses");

        assert_eq!(release.version.to_string(), "0.1.7");
        let target = "windows-x86_64";
        assert_eq!(
            release.download_url(target).unwrap().as_str(),
            "https://github.com/hebus/pr-dashboard/releases/download/v0.1.7/pr-dashboard_0.1.7_x64-setup.exe"
        );
        assert_eq!(release.format(target).unwrap().to_string(), "nsis");
        assert!(!release.signature(target).unwrap().is_empty());
    }

    /// The old Tauri updater reads the same file: it needs `url` and `signature`
    /// per platform, and must not choke on the extra `format` field.
    #[test]
    fn the_manifest_keeps_the_fields_the_tauri_updater_needs() {
        let v: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/latest.json")).unwrap();
        for key in ["windows-x86_64", "windows-x86_64-nsis"] {
            let p = &v["platforms"][key];
            assert!(p["url"].as_str().is_some_and(|u| u.ends_with("-setup.exe")), "{key}");
            assert!(p["signature"].as_str().is_some_and(|s| !s.is_empty()), "{key}");
        }
        assert!(v["pub_date"].as_str().is_some());
    }
}
