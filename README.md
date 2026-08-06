# PR Dashboard

> Desktop app to monitor GitHub Pull Requests and GitLab Merge Requests in real-time — built with Tauri 2 + React 19.

![Platform](https://img.shields.io/badge/platform-Windows-blue)
![GitHub](https://img.shields.io/badge/GitHub-supported-181717?logo=github&logoColor=white)
![GitLab](https://img.shields.io/badge/GitLab-supported-fc6d26?logo=gitlab&logoColor=white)
![Tauri](https://img.shields.io/badge/Tauri-2-purple)
![React](https://img.shields.io/badge/React-19-61dafb)
![TypeScript](https://img.shields.io/badge/TypeScript-5.8-3178c6)
![Rust](https://img.shields.io/badge/Rust-stable-orange)

---

## What it does

Keep an eye on all your open PRs across multiple repositories without ever leaving your desktop. PR Dashboard polls GitHub (or GitHub Enterprise) **and GitLab** on a configurable interval and gives you:

- Instant visibility on every open PR — draft, pending review, approved, changes requested
- Native desktop notifications when a new PR is opened or merged
- A toast history panel so you never miss an event, even when the window is hidden
- System tray integration — minimize to tray, badge tooltip updates when events happen in the background
- Light/dark theme

---

## Features

| Feature | Details |
|---------|---------|
| **Multi-source** | GitHub and GitLab side by side — one token per source, provider picked per repo |
| **Multi-repo monitoring** | Track as many repositories as you want, grouped by repo |
| **Real-time polling** | Configurable interval (default: 60s), powered by TanStack Query |
| **Review status badges** | APPROVED / CHANGES REQUESTED / PENDING REVIEW at a glance |
| **Native notifications** | OS-level toast notifications (Windows) via `tauri-plugin-notification` |
| **In-app toasts** | Slide-in toasts with progress bar — fallback when OS notifications are blocked |
| **Notification history** | Bell icon with unread badge, full history panel with mark-as-read |
| **System tray** | Click-to-toggle window, pending event count in tooltip |
| **GitHub Enterprise** | Fully supports GHES — just set your instance URL in settings |
| **GitLab** | Self-hosted or gitlab.com; MR numbers shown as `!123`, approvals and « changes requested » included |
| **Persistent config** | Tokens, repos, interval stored in `%APPDATA%` via `tauri-plugin-store` |
| **Skeleton loading** | Animated placeholders on first fetch, silent background refresh afterwards |
| **Light / dark theme** | One-click toggle, preference stored in localStorage |

---

## Tech stack

| Layer | Technology |
|-------|-----------|
| Frontend | React 19 + TypeScript |
| Build | Vite 7 |
| Styles | Tailwind CSS 4 + CSS custom properties |
| Data fetching | TanStack React Query 5 |
| Desktop shell | Tauri 2 (Rust) |
| HTTP (Rust) | reqwest 0.12 + tokio |
| Config persistence | tauri-plugin-store |
| Notifications | tauri-plugin-notification |
| Link opening | tauri-plugin-opener |
| Windows registry | winreg 0.52 (AUMID registration) |

---

## Getting started

### Prerequisites

- [Node.js](https://nodejs.org) 18+
- [Rust toolchain](https://rustup.rs) (stable)
- WebView2 runtime — already present on Windows 10 21H2+ / Windows 11

### Install dependencies

```bash
npm install
```

### Run in development

```bash
npm run tauri dev
```

Hot-reload is active for the frontend. Rust code rebuilds automatically when modified.

### Build for production

```bash
npm run tauri build
```

Outputs a Windows installer in `src-tauri/target/release/bundle/`.

---

## Configuration

On first launch, open **Settings** (gear icon, top right) and fill in:

| Field | Description |
|-------|-------------|
| **GitHub URL / Token** | Your GitHub instance (default: `https://github.sinequa.com`) + a PAT with `repo` scope |
| **GitLab URL / Token** | Your GitLab instance (default: `https://gitlab.chapsvision.in`) + a PAT with `read_api` scope |
| **Refresh interval** | Polling interval in seconds (default: 60) |
| **Repositories** | One row per repo: a GitHub/GitLab switch, then `owner/repo` |

Only the source you actually use needs a token — a missing one only affects its own repo groups.

For a GitLab repo, the first field holds the **full namespace**, so `https://gitlab.chapsvision.in/sinequa/rnd/my-repo` is entered as `sinequa/rnd` + `my-repo` (the namespace is pre-filled when you flip the switch).

Config is persisted in `%APPDATA%\com.sinequa.prdashboard\config.json`.

> **GitHub Enterprise:** the app auto-derives API endpoints from the base URL — no extra config needed.

---

## APIs

One **GraphQL** call per repository fetches the 50 most recently updated open PRs/MRs, including review decisions and approvers. A second lightweight call happens only when a PR/MR disappears from the open list, to distinguish a merge from a simple close.

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
├── src/                    # React / TypeScript frontend
│   ├── components/         # UI components (PRCard, FilterBar, Settings…)
│   ├── hooks/              # useConfig, usePRData, useToast, useNotifications
│   ├── types/              # Shared TypeScript types + helpers
│   └── App.tsx             # Root — layout, state orchestration, theme
└── src-tauri/              # Rust backend
    ├── src/
    │   ├── lib.rs          # Tauri init, tray, AUMID registration
    │   ├── commands/       # get_config, save_config, fetch_repo_prs, check_pr_merged
    │   └── providers/      # One module per source: common, github, gitlab
    ├── capabilities/       # Tauri permission manifest
    └── tauri.conf.json     # App config (name, window size, bundle)
```

Full architecture details in [ARCHITECTURE.md](./guides/ARCHITECTURE.md).

---

## Windows notification setup

Windows Toast Notifications require the app to be registered in the registry with an **AppUserModelId (AUMID)**. The app handles this automatically at startup — no admin rights needed (written to `HKCU`):

```
HKCU\SOFTWARE\Classes\AppUserModelId\com.sinequa.prdashboard
  DisplayName = "PR Dashboard"
```

This makes the app appear in **Windows → Settings → Notifications** so you can manage its notification preferences.

---

## License

MIT
