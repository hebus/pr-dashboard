# Architecture

PR Dashboard watches pull requests (GitHub) and merge requests (GitLab) across
repositories. This is the pure-Rust version, built on
[GPUI](https://crates.io/crates/gpui); it replaces the Tauri + React build.

```
Cargo.toml                    workspace (version lives in [workspace.package])
crates/core                   no UI, no GPUI: providers, config, pure logic
crates/app                    the GPUI application (binary: pr-dashboard)
scripts/make_latest_json.py   builds the update manifest in CI
.github/workflows             release.yml (release-please + Windows installer), ci.yml
```

## `crates/core` (testable without a window)

| Module | Role |
|---|---|
| `providers/github.rs`, `gitlab.rs` | GraphQL queries and mapping to `PullRequest`. Author, reviewers and approvers carry `login` **and** display `name` (falls back to the login). |
| `providers/common.rs` | Shared output types (`PullRequest`, `Reviewer`, `RepoPRs`), HTTP client, label color normalization. |
| `config.rs` | `Config` and its JSON file. Same file and shape as the Tauri build (`%APPDATA%/com.sinequa.prdashboard/config.json`, `{ "config": {...} }`) so an existing setup carries over. Every field has a `default`; never add `deny_unknown_fields` (a parse failure silently resets the user's tokens). |
| `fetch.rs` | `fetch_repo_prs` / `check_pr_merged`. Failures are reported in `RepoPRs.error`, never as `Err`. |
| `logic.rs` | Filters, search, branch ordering (develop, release/x by version desc, rest), relative time, avatar initials, the new/vanished PR diff, header totals, `generate_id`. |

## `crates/app`

| Module | Role |
|---|---|
| `main.rs` | Builds the tokio runtime, theme, assets, the `Dashboard` and `Updater` models and the window. |
| `state.rs` | `Dashboard` model: config, per-repository data, notification history. One polling loop per repository (GPUI task + timer). Network calls run on a tokio runtime and are awaited from GPUI. Emits `PrEvent` for new / merged PRs. |
| `view.rs` | Root view: header, update banner, filter bar, list, overlays. |
| `repo_group.rs` | Flattens repositories into `Row`s for a **virtualized** `gpui::list` (only visible rows are built; rendering every card made scrolling crawl). The list is updated with `splice`, never `reset`, so the scroll position survives polling. `rows_dirty` marks when to rebuild. |
| `pr_card.rs`, `filter_bar.rs` | Card and filter bar. |
| `settings.rs` | Settings modal: edits a local copy, applied on Save. |
| `notification_panel.rs`, `toasts.rs` | Notification history dropdown and transient toasts. |
| `updater.rs`, `update_banner.rs` | Self-update (below). |
| `theme.rs`, `widgets.rs` | Color tokens (light/dark) and small stateless UI pieces. |
| `assets.rs` | Icons: own SVGs (Lucide, GitHub/GitLab marks) over `gpui-component-assets`. |

Notes that are easy to get wrong:

* **Fixed versions.** `gpui` 0.2.2 and `gpui-component`/`gpui-component-assets`
  `=0.5.1` go together. `gpui-component` 0.6+ depends on a fork of GPUI
  (`gpui-pre`); mixing the two gives two incompatible GPUI copies.
* **Dev profile.** Dependencies are compiled with `opt-level = 3` even in dev
  builds (`[profile.dev.package."*"]`): GPUI is unusably slow unoptimized.
* **Fetch errors do not diff.** A failed fetch has no PRs; diffing it would report
  every PR as vanished, then as new on the next success.
* **Label colors** are adjusted to stay readable on the current theme
  (`widgets::readable_label_color`).
* **No OS notifications.** The Tauri build registered the plugin but never
  called it; new / merged PRs show as toasts and in the history panel only.

## Updates and releases

* `release-please` (release-type `simple`, version in `[workspace.package]`)
  opens a release PR from conventional commits; merging it tags `vX.Y.Z`.
* `build-windows` builds with `cargo-packager` (NSIS installer, `currentUser`
  mode), signs it with the minisign key (`TAURI_SIGNING_PRIVATE_KEY` secret,
  mapped to `CARGO_PACKAGER_SIGN_PRIVATE_KEY`) and uploads the installer, its
  `.sig` and `latest.json` to the release.
* **The installer is signed with the Tauri CLI, not with `cargo-packager`.**
  The release step runs `npx @tauri-apps/cli@2.11.1 signer sign`, the same tool
  and version that signed the Tauri releases with this same secret and an empty
  password variable. `cargo packager` and `cargo packager signer sign` both
  failed on this key with "Wrong password for that key" (the first with an
  empty password variable, the second which always sends an empty password).
  The `.sig` is a standard minisign signature, so `cargo-packager-updater`
  verifies it. Run the `Sign check` workflow after touching the key or this
  step: it signs a throwaway file in about a minute.
* If the build job fails after the release exists, run the `Release` workflow
  by hand (Actions, "Run workflow") with the tag as input. It rebuilds that tag
  and uploads to the existing release. Re-running the failed run would use the
  workflow file as it was when it ran, so it cannot pick up a fix.
* `latest.json` is read by two updaters: this app (`cargo-packager-updater`,
  needs a `format` field) and the previous Tauri build (`tauri-plugin-updater`,
  ignores it). It is served from `releases/latest/download/latest.json`.
* The app checks 5 s after start and every 6 h. Nothing is downloaded until
  the user clicks "Install and restart"; the installer's signature is verified
  against the public key embedded in `updater.rs`.
* The binary and product name are `pr-dashboard`, the identifier
  `com.sinequa.prdashboard`: the NSIS installer upgrades an existing Tauri
  install in place only if these match.
* Debug builds do not check for updates unless `PR_DASHBOARD_UPDATE_URL` points
  at a manifest (the signature is still verified).

### Replacing an installed Tauri build (checklist before the first public release)

The goal is that an existing Tauri install updates itself into this app. Compared
statically (the generated Tauri `installer.nsi` against `cargo-packager`'s
template), the two installers agree on everything the upgrade relies on:
manufacturer `sinequa`, product and binary `pr-dashboard`, `currentUser` mode,
uninstall key, install-dir registry key, and the `/P` (passive) and `/R`
(relaunch) flags. The Tauri updater also passes `/UPDATE` and `/ARGS`, which the
new installer ignores. In passive mode the old uninstaller's "delete app data"
checkbox is never shown, so `%APPDATA%\com.sinequa.prdashboard\config.json` is
kept. **This is reading, not running**: do the following once on a machine that
has the Tauri build installed.

1. Copy `%APPDATA%\com.sinequa.prdashboard\config.json` somewhere safe.
2. `cd crates/app` then `cargo packager --release` (needs `cargo install
   cargo-packager --locked --version 0.11.8`). The installer lands in `dist/`.
3. Run it exactly as the Tauri updater would:
   `pr-dashboard_<version>_x64-setup.exe /P /UPDATE /R`
4. Check: the app relaunches and is the new one; tokens and repositories are
   still there; "Apps & features" lists a single PR Dashboard; the Start menu and
   desktop shortcuts still work; `%LOCALAPPDATA%\pr-dashboard` was reused.
5. Then publish a test release and let the Tauri build update itself through its
   own banner. Also try it on a machine or VM without a GPU or over Remote
   Desktop: GPUI draws with DirectX 11, while the Tauri build only needed WebView2.

Windows is the only supported target. GPUI also targets macOS and Linux, but
nothing here has been built or tested on them.
