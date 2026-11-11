# PR Dashboard

> Desktop app to monitor GitHub Pull Requests and GitLab Merge Requests in real time. Written in Rust with [GPUI](https://crates.io/crates/gpui).

![Platform](https://img.shields.io/badge/platform-Windows-blue)
![GitHub](https://img.shields.io/badge/GitHub-supported-181717?logo=github&logoColor=white)
![GitLab](https://img.shields.io/badge/GitLab-supported-fc6d26?logo=gitlab&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-stable-orange)

---

## What it does

Keep an eye on all your open PRs across multiple repositories without leaving your desktop. PR Dashboard polls GitHub (or GitHub Enterprise) **and GitLab** on a configurable interval and gives you:

- Instant visibility on every open PR: draft, pending review, approved, changes requested, conflicts, failed pipeline
- A toast and a history entry when a new PR is opened or merged
- A notification history panel, so you never miss an event
- Light/dark theme

It is a single native executable: no browser engine, no Node.js.

---

## Features

| Feature | Details |
|---------|---------|
| **Multi-source** | GitHub and GitLab side by side: one token per source, provider picked per repo |
| **Multi-repo monitoring** | Track as many repositories as you want, grouped by repo and by target branch |
| **Real-time polling** | One loop per repository, configurable interval (default: 60 s) |
| **Large lists stay fast** | The list is virtualized: only the cards on screen are drawn |
| **Review status badges** | Approved / changes requested / pending review, conflicts, failed pipeline, mergeable |
| **Search and filters** | By title, author (login or full name) or number; All / Pending / Approved / Mergeable / Conflicts |
| **In-app toasts** | New and merged PRs, with a countdown bar |
| **Notification history** | Bell icon with unread dot, mark as read, dismiss |
| **GitHub Enterprise** | Just set your instance URL in Settings |
| **GitLab** | Self-hosted or gitlab.com; MR numbers shown as `!123`, approvals and "changes requested" included |
| **Persistent config** | Tokens, repos, interval and theme stored in `%APPDATA%` |
| **Auto-update** | Checks GitHub Releases, installs on click (see below) |

---

## Getting started

### Prerequisites

- [Rust toolchain](https://rustup.rs) (stable)
- Windows 10/11 (the only supported platform: GPUI draws with DirectX 11)

### Run in development

```bash
cargo run -p pr-dashboard-app
```

The first build is long: dependencies are compiled optimized even in dev builds, because GPUI is unusably slow otherwise. Later builds only recompile the app.

### Test

```bash
cargo test --workspace
```

### Build the Windows installer

```bash
cargo install cargo-packager --locked --version 0.11.8   # once
cd crates/app
cargo packager --release
```

The NSIS installer is written to `dist/`.

---

## Configuration

On first launch, open **Settings** (gear icon, top right) and fill in:

| Field | Description |
|-------|-------------|
| **GitHub URL / Token** | Your GitHub instance (default: `https://github.sinequa.com`) + a PAT with `repo` scope |
| **GitLab URL / Token** | Your GitLab instance (default: `https://gitlab.chapsvision.in`) + a PAT with `read_api` scope |
| **Refresh interval** | Polling interval in seconds (default: 60, minimum 10) |
| **Repositories** | One row per repo: a GitHub/GitLab switch, then `owner` / `repo` and an optional label |

Only the source you actually use needs a token: a missing one only affects its own repo groups.

For a GitLab repo, the first field holds the **full namespace**, so `https://gitlab.chapsvision.in/sinequa/rnd/my-repo` is entered as `sinequa/rnd` + `my-repo` (the namespace is pre-filled when you flip the switch).

Config is persisted in `%APPDATA%\com.sinequa.prdashboard\config.json`, the same file the previous Tauri version used, so an existing setup carries over.

---

## APIs

One **GraphQL** call per repository fetches the 50 most recently updated open PRs/MRs, including review decisions, approvers and display names. A second lightweight call happens only when a PR/MR disappears from the open list, to distinguish a merge from a simple close.

```
GitHub  list   →  {githubUrl}/api/graphql
        merged →  {githubUrl}/api/v3/repos/{owner}/{repo}/pulls/{number}

GitLab  list   →  {gitlabUrl}/api/graphql
        merged →  {gitlabUrl}/api/graphql
```

Both use `Authorization: Bearer <token>`. GitLab merge requests are mapped onto the same internal model as GitHub pull requests, so the whole UI is source-agnostic.

---

## Project structure

```
pr-dashboard/
├── Cargo.toml              # workspace (the version lives here)
├── crates/
│   ├── core/               # providers, config, pure logic: no UI, fully unit-tested
│   └── app/                # the GPUI application (binary: pr-dashboard)
├── scripts/                # make_latest_json.py: builds the update manifest
└── .github/workflows/      # release.yml, ci.yml
```

Full architecture details in [ARCHITECTURE.md](./guides/ARCHITECTURE.md).

---

## Auto-update & releases

Installed apps check GitHub Releases 5 seconds after start and every 6 hours, and show a banner with an **Install and restart** button when a newer version exists. Nothing is installed without a click, and the installer's signature is verified.

Releases are automated with [release-please](https://github.com/googleapis/release-please). Use [Conventional Commits](https://www.conventionalcommits.org/) (`feat:`, `fix:`...) and merge into `develop` as usual:

1. A **release PR** ("chore(release): X.Y.Z") is opened and kept up to date. It bumps the version in `Cargo.toml` (and `Cargo.lock`, synced by a separate `sync-lockfile` job) and writes `CHANGELOG.md`. Merge feature PRs with **squash** so each change appears once in the changelog. `fix:` → patch, `feat:` → patch while the version is 0.x, `feat!:` / `BREAKING CHANGE` → minor.
2. **Merge that PR** to release: release-please tags the commit and creates the GitHub release, then the same `Release` workflow builds the Windows installer with `cargo-packager`, signs it and uploads it with `latest.json`.

It needs the repository secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (a minisign key; the names are kept from the Tauri version), and the repo setting *Settings → Actions → General → Allow GitHub Actions to create and approve pull requests*. The installer is not code-signed, so Windows SmartScreen warns on first install.

---

## License

MIT
