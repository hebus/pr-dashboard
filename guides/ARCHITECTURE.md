# PR Dashboard — Architecture

Application desktop Tauri 2 + React 19 qui affiche en temps réel les Pull Requests ouvertes sur plusieurs repositories GitHub Enterprise, avec notifications desktop.

---

## Stack technique

| Couche | Technologie | Rôle |
|--------|-------------|------|
| Frontend | React 19 + TypeScript | UI, état local, polling |
| Build frontend | Vite 7 | Dev server (port 1420), bundling |
| Styles | Tailwind CSS 4 + CSS variables | Utility-first, theming light/dark |
| Async/Cache | TanStack React Query 5 | Fetching, polling, invalidation |
| Desktop shell | Tauri 2 (Rust) | Fenêtre native, system tray, commandes |
| HTTP Rust | reqwest 0.12 + tokio | Appels GitHub API (async) |
| Persistance config | tauri-plugin-store | Fichier JSON dans AppData |
| Notifications | tauri-plugin-notification | Notifications OS natives |
| Registre Windows | winreg 0.52 (Windows only) | Enregistrement AUMID au démarrage |
| Historique notifs | useNotifications (in-memory) | Stockage, lecture et suppression des notifications |
| Ouverture liens | tauri-plugin-opener | Ouvre GitHub dans le navigateur par défaut |

---

## Structure des fichiers

```
pr-dashboard/
├── src/                          # Frontend React/TypeScript
│   ├── types/
│   │   └── index.ts              # Tous les types + helpers (timeAgo, matchesFilter…)
│   ├── hooks/
│   │   ├── useConfig.ts          # Chargement/sauvegarde config via Tauri
│   │   ├── usePRData.ts          # Polling GitHub + émission d'événements PREvent
│   │   ├── useToast.ts           # Gestion de la queue de toasts (state + auto-dismiss 5s)
│   │   └── useNotifications.ts   # Historique persistant des notifications (markRead, remove…)
│   ├── components/
│   │   ├── ErrorBoundary.tsx     # Capture les erreurs React
│   │   ├── FilterBar.tsx         # Barre recherche + toggle All/Pending/Approved
│   │   ├── RepoGroup.tsx         # Groupe collapsible par repository (avec squelettes au premier chargement)
│   │   ├── PRCard.tsx            # Carte d'une Pull Request (nom de branche cliquable → copie clipboard)
│   │   ├── ToastContainer.tsx    # Toasts in-app (slide-in, barre de progression, cliquables)
│   │   ├── NotificationPanel.tsx # Panneau dropdown historique des notifications (icône cloche dans le header)
│   │   └── Settings.tsx          # Modal de configuration (token, repos, intervalle, test notification)
│   ├── App.tsx                   # Racine : layout, orchestration état global, toggle thème, gestion événements PR
│   ├── main.tsx                  # Bootstrap React + QueryClient
│   └── index.css                 # Tailwind import, variables CSS des deux thèmes, scrollbar, @keyframes toastProgress
│
└── src-tauri/                    # Backend Rust
    ├── src/
    │   ├── lib.rs                # Init Tauri : plugins, tray icon, handlers, register_aumid, set_tray_tooltip
    │   ├── main.rs               # Point d'entrée (délègue à lib.rs)
    │   └── commands/
    │       ├── mod.rs            # Déclaration des modules
    │       ├── config.rs         # Commandes get_config / save_config
    │       └── fetch_prs.rs      # Commandes fetch_repo_prs / check_pr_merged / open_url
    ├── capabilities/
    │   └── default.json          # Permissions Tauri (tray, store, notification, opener)
    ├── icons/                    # Icônes de l'application (utilisée aussi pour le tray)
    ├── Cargo.toml                # Dépendances Rust
    └── tauri.conf.json           # Config Tauri (nom, fenêtre, bundle)
```

---

## Configuration (Config)

La configuration est persistée dans un fichier JSON via `tauri-plugin-store` (emplacement : `%APPDATA%\com.sinequa.prdashboard\config.json`).

```typescript
interface Config {
  githubToken: string;        // Personal Access Token (scope: repo)
  githubUrl: string;          // URL de base de l'instance GitHub (défaut: https://github.sinequa.com)
  refreshInterval: number;    // Intervalle de polling en secondes (défaut: 60)
  repositories: RepositoryConfig[];
}

interface RepositoryConfig {
  id: string;       // UUID généré localement
  owner: string;    // Propriétaire GitHub (ex: "sinequa")
  name: string;     // Nom du repo (ex: "sba-internal")
  label?: string;   // Alias d'affichage optionnel
}
```

**Flux de config :**
1. `useConfig` appelle `invoke("get_config")` au montage
2. Le backend Rust lit le store → retourne la config ou la valeur par défaut
3. `saveConfig()` appelle `invoke("save_config", { config })` → persiste sur disque

> **Thème :** la préférence light/dark n'est pas dans la `Config` Tauri — elle est stockée dans `localStorage` et lue au démarrage de l'application (`"dark"` par défaut).

---

## Flux de données principal

```
App.tsx
  ├─ useState(theme)      → localStorage               → classe .dark sur <html>
  ├─ useConfig()          → invoke("get_config")        → config.json (AppData)
  └─ usePRData(config)
       └─ useQueries()    → invoke("fetch_repo_prs")    → GitHub GraphQL API
            │                  (une query par repo, polling toutes les N secondes)
            │
            ↓ résultats
       RepoPRs[]  ──────────────────────────────────────────────────────────────
            │                                                                   │
            ↓                                                                   ↓
       RepoGroup (par repo)                                            Notifications
            └─ PRCard (par PR)                                  (nouvelle PR / PR mergée)
```

---

## GitHub API

L'application supporte **GitHub Enterprise Server** (GHES). L'URL de base est configurable dans les Settings (`githubUrl`). Les endpoints sont dérivés automatiquement :

| Endpoint | URL construite |
|----------|---------------|
| GraphQL  | `{githubUrl}/api/graphql` |
| REST     | `{githubUrl}/api/v3/repos/{owner}/{repo}/pulls/{number}` |
| Web      | `{githubUrl}/{owner}/{repo}/pulls` |

Pour `https://github.sinequa.com` :
- GraphQL → `https://github.sinequa.com/api/graphql`
- REST → `https://github.sinequa.com/api/v3/repos/...`

### GraphQL (fetch_repo_prs)

Un seul appel GraphQL par repository pour récupérer les 50 PRs ouvertes les plus récentes avec toutes leurs informations (reviews incluses) :

```
POST {githubUrl}/api/graphql
Authorization: Bearer <token>

query GetOpenPRs($owner, $repo, $first: 50) {
  repository(owner, name) {
    pullRequests(states: [OPEN], orderBy: UPDATED_AT DESC) {
      nodes {
        number, title, url, isDraft
        createdAt, updatedAt
        headRefName, baseRefName
        reviewDecision          ← APPROVED | CHANGES_REQUESTED | REVIEW_REQUIRED | null
        author { login, avatarUrl }
        labels { name, color }
        reviews(last: 20) { state, author }   ← pour extraire les approbateurs
      }
    }
  }
}
```

`reviewDecision` est calculé par GitHub et reflète la décision agrégée de review.

> **Note GHES :** Le champ `requestedReviewers` n'est pas disponible sur toutes les versions de GitHub Enterprise Server. Il a été retiré de la query pour éviter une erreur GraphQL bloquante. Le champ `requestedReviewers` dans les types TypeScript et Rust est conservé mais retourne toujours un tableau vide.

### REST (check_pr_merged)

Appelé uniquement quand une PR disparaît de la liste ouverte, pour distinguer une fermeture d'un merge :

```
GET {githubUrl}/api/v3/repos/{owner}/{repo}/pulls/{number}
→ { state: "closed", merged_at: "2024-..." }   ← merged si merged_at non null
```

---

## Types de données

```typescript
interface PullRequest {
  number: number;
  title: string;
  url: string;
  isDraft: boolean;
  createdAt: string;          // ISO 8601
  updatedAt: string;
  headRefName: string;        // branche source
  baseRefName: string;        // branche cible
  reviewDecision: ReviewDecision;
  author: Reviewer;
  labels: Label[];
  requestedReviewers: Reviewer[];  // toujours vide (non supporté sur GHES, voir note)
  approvers: Reviewer[];           // extrait des reviews avec state === "APPROVED"
}

type ReviewDecision = "APPROVED" | "CHANGES_REQUESTED" | "REVIEW_REQUIRED" | null;

interface RepoPRs {
  repo: string;               // "owner/name"
  prs: PullRequest[];
  error: string | null;
  lastUpdated: string | null; // horodatage du dernier fetch (UTC, côté Rust via chrono)
}
```

---

## Chargement initial — squelettes

Au premier lancement (avant que les données GitHub ne soient disponibles), `RepoGroup` affiche 3 cartes squelettes animées (`animate-pulse`) qui reproduisent la structure de `PRCard` :
- Icône + numéro + barre de titre (largeurs variées : 58 %, 42 %, 72 %)
- Ligne auteur / date / branches avec avatar rond
- Badge de review

La condition d'affichage est `isLoading && prs.length === 0` : les squelettes n'apparaissent qu'au **premier fetch** (pas données en cache). Les refreshs en arrière-plan restent discrets — seul le spinner dans l'en-tête du groupe tourne.

---

## Interactions sur la carte PR (`PRCard`)

| Cible | Action | Comportement |
|-------|--------|-------------|
| Carte (zone générale) | Clic | Ouvre la PR dans le navigateur (`invoke("open_url")`) |
| Nom de la branche source (`headRefName`) | Clic | Copie le nom de branche dans le presse-papiers via `navigator.clipboard.writeText` |

Le nom de branche est rendu comme un `<button>` :
- `e.stopPropagation()` empêche le clic d'ouvrir aussi la PR.
- Une icône `Copy` (lucide) apparaît au survol de la carte (`opacity-0 group-hover:opacity-60`) ; après copie elle bascule en `Check` vert pendant 1,5 s via un état local `copied` (`setTimeout`).
- Le `title` reflète l'état : `Copy "<branche>"` ou `Copied!`.
- `navigator.clipboard` est disponible dans la WebView2 (Tauri) ; l'appel est protégé par un `try/catch` silencieux si l'API est indisponible.

---

## Système de notifications

### Enregistrement Windows (AUMID)

Sur Windows, les Toast Notifications requièrent que l'application soit enregistrée dans le registre avec un **AppUserModelId (AUMID)**. L'installeur NSIS généré par Tauri n'effectue pas cet enregistrement automatiquement.

La fonction `register_aumid()` dans `lib.rs` y remédie en écrivant la clé au démarrage de l'app (mode dev et production) :

```
HKCU\SOFTWARE\Classes\AppUserModelId\com.sinequa.prdashboard
  DisplayName = "PR Dashboard"
```

Cet enregistrement est idempotent (aucun effet si la clé existe déjà) et ne nécessite pas de droits admin (HKCU). Sans cette clé, `tauri-plugin-notification` échoue silencieusement et l'app n'apparaît pas dans Windows → Paramètres → Notifications.

### Détection des changements de PR

Les notifications sont gérées dans `usePRData` via un `useRef` qui conserve l'état précédent des PRs entre les renders :

```
Ref: Map<repoKey, Map<prNumber, prTitle>>
                  ↑ persist entre les renders

À chaque poll :
  Pour chaque PR actuelle :
    ├─ absente du ref précédent  →  "New PR in {repo}"  notification
  Pour chaque PR du ref précédent :
    ├─ absente de la liste actuelle
    │     └─ invoke("check_pr_merged")  →  REST API
    │           ├─ merged_at non null   →  "PR merged in {repo}"  notification
    │           └─ pas mergée           →  silencieux (fermée sans merge)
  Mise à jour du ref avec l'état actuel
```

> **Premier démarrage :** le `useRef` est vide, donc aucune notification ne tire lors du premier fetch. Les notifications démarrent à partir du second poll.

**Permissions :** vérifiées avant chaque notification via `isPermissionGranted()`. Si non accordées, `requestPermission()` affiche la boîte de dialogue OS au premier déclenchement.

### Toasts in-app (alternative aux notifications OS)

Quand les notifications OS sont bloquées (politique d'entreprise), un système de toasts in-app prend le relais.

**`useToast`** gère la queue : `addToast(event)` ajoute un toast et programme sa suppression automatique après 5 secondes via `setTimeout`.

**`ToastContainer`** rend les toasts en position fixe en bas à droite (z-50). Chaque `ToastItem` :
- Slide in depuis la droite via une transition CSS (`translateX` + `opacity`)
- Affiche l'icône selon le type (bleu `GitPullRequest` = new PR, vert `GitMerge` = merged)
- Barre de progression qui se vide en 5s (`@keyframes toastProgress` dans `index.css`)
- Cliquable pour ouvrir la PR (`invoke("open_url", ...)`)
- Bouton ✕ pour dismisser manuellement

**Événements PR (`PREvent`)** — type défini dans `types/index.ts` :
```typescript
interface PREvent {
  type: "new_pr" | "merged";
  repo: string;       // "owner/name"
  prNumber: number;
  prTitle: string;
  url: string;
}
```

**`usePRData`** n'appelle plus directement les APIs de notification — il appelle le callback `onEvent: (e: PREvent) => void` passé par `App.tsx`. Le ref interne stocke désormais `{ title, url }` (au lieu de juste `title`) pour pouvoir inclure l'URL dans les événements de merge.

### Comportement quand la fenêtre est cachée (tray)

```
PR event détecté
  ├─ fenêtre visible  →  addToast()  →  toast affiché immédiatement
  └─ fenêtre cachée   →  pendingRef.current.push(event)
                         invoke("set_tray_tooltip", "PR Dashboard — N new events")

tauri://focus (fenêtre rouverte depuis le tray)
  →  pending.forEach(addToast)   [tous les toasts manqués s'affichent]
  →  pendingRef.current = []
  →  invoke("set_tray_tooltip", "PR Dashboard")   [tooltip réinitialisé]
```

La commande Rust `set_tray_tooltip` retrouve l'icône tray via son ID `"main"` (défini avec `.with_id("main")` dans le builder) et appelle `tray.set_tooltip(...)`.

**Test :** le bouton **Test** dans Settings → Notifications appelle `onTestNotification` (prop fournie par `App.tsx`) qui déclenche un toast ET ajoute une entrée dans l'historique.

### Historique des notifications (`useNotifications` + `NotificationPanel`)

En complément des toasts éphémères, chaque événement PR est aussi ajouté à un historique persistent en mémoire via `useNotifications`.

**`StoredNotification`** (type dans `types/index.ts`) :
```typescript
interface StoredNotification {
  id: string;
  event: PREvent;
  timestamp: string;  // ISO 8601
  read: boolean;
}
```

**`useNotifications`** expose : `notifications`, `addNotification`, `markRead`, `markAllRead`, `remove`, `unreadCount`.

**`NotificationPanel`** s'affiche en dropdown depuis l'icône cloche dans le header. Un badge pulsant (bleu, `animate-ping`) indique le nombre de notifications non lues. Le panel :
- Liste les notifications avec indicateur de lecture (point bleu), icône de type, repo, numéro/titre et timestamp relatif
- Clic sur une ligne → marque comme lu + ouvre la PR dans le navigateur
- Bouton "Mark all read" visible si des notifications non lues existent
- Bouton ✕ par ligne (visible au hover) pour supprimer
- Fermeture automatique au clic en dehors (via `mousedown` listener sur `document`)

---

## Theming (light / dark)

Les couleurs ne sont **pas** hardcodées dans les classes Tailwind — elles passent toutes par des CSS custom properties définies dans `index.css`. Un seul changement de classe sur `<html>` bascule l'intégralité du thème.

```css
/* index.css */
:root      { /* light theme */ --c-bg: #ffffff; --c-accent: #0969da; … }
:root.dark { /* dark theme  */ --c-bg: #0d1117; --c-accent: #58a6ff; … }
```

Les composants utilisent ces variables via la syntaxe Tailwind arbitraire :
```tsx
<div className="bg-[var(--c-bg)] text-[var(--c-text)] border-[var(--c-border)]">
```

**Toggle :** bouton Sun/Moon dans le header (`App.tsx`). La valeur est lue/écrite dans `localStorage` sous la clé `"theme"`. Au montage, `useEffect` applique ou retire la classe `.dark` sur `document.documentElement`.

| Variable | Light | Dark | Usage |
|----------|-------|------|-------|
| `--c-bg` | `#ffffff` | `#0d1117` | Fond principal |
| `--c-bg-subtle` | `#f6f8fa` | `#161b22` | Header, cartes, settings |
| `--c-bg-hover` | `#f3f4f6` | `#1c2128` | Hover des éléments interactifs |
| `--c-bg-inset` | `#eaeef2` | `#21262d` | Badges neutres, filtre actif |
| `--c-border` | `#d0d7de` | `#30363d` | Toutes les bordures |
| `--c-text` | `#1f2328` | `#e6edf3` | Texte principal |
| `--c-text-muted` | `#57606a` | `#8b949e` | Labels secondaires |
| `--c-accent` | `#0969da` | `#58a6ff` | Liens, focus, icône PR |
| `--c-green` | `#1a7f37` | `#3fb950` | Statut Approved (texte) |
| `--c-green-bg` | `#dafbe1` | `#1a4428` | Badge Approved (fond) |
| `--c-green-btn` | `#1a7f37` | `#238636` | Boutons d'action |
| `--c-amber` | `#9a6700` | `#d29922` | Statut Pending Review |
| `--c-amber-bg` | `#fff8c5` | `#2d2005` | Badge Pending (fond) |
| `--c-red` | `#cf222e` | `#f85149` | Statut Changes Requested, erreurs |
| `--c-red-bg` | `#ffebe9` | `#3d1212` | Badge Changes Requested (fond) |

---

## System Tray

Configuré dans `lib.rs` via `TrayIconBuilder` :

| Action | Comportement |
|--------|-------------|
| Clic gauche sur l'icône | Toggle show/hide de la fenêtre principale |
| Menu → "Show Dashboard" | Affiche et focus la fenêtre |
| Menu → "Quit" | Quitte l'application |
| Clic sur la croix ✕ | Réduit au tray (ne quitte pas) grâce à `on_window_event` |

---

## Communication Frontend ↔ Backend

Tauri expose un système de RPC via `invoke()` côté frontend. Les handlers Rust sont enregistrés dans `lib.rs` :

| Commande | Paramètres clés | Description |
|----------|-----------------|-------------|
| `get_config` | — | Lit la config depuis le store |
| `save_config` | `config` | Persiste la config sur disque |
| `fetch_repo_prs` | `owner`, `repo`, `token`, `githubUrl` | Appel GraphQL, retourne `RepoPRs` |
| `check_pr_merged` | `owner`, `repo`, `number`, `token`, `githubUrl` | Appel REST, retourne `bool` |
| `open_url` | `url` | Ouvre une URL dans le navigateur système |
| `set_tray_tooltip` | `tooltip` | Met à jour le tooltip de l'icône tray |

Les `AppHandle` et injections Tauri sont gérés automatiquement — le frontend ne passe que les paramètres métier.

---

## Polling et performance

- **React Query** gère le cache, le polling (`refetchInterval`) et les invalidations manuelles
- Chaque repo a sa propre query indépendante : clé `["prs", owner, name]`
- L'invalidation manuelle (bouton refresh) appelle `queryClient.invalidateQueries()`
- Les queries sont désactivées (`enabled: false`) si le token GitHub est vide
- Le backend fait l'appel HTTP en Rust avec un timeout de 15s

---

## Lancer l'application

```bash
# Développement (hot-reload frontend, rebuild Rust si modifié)
npm run tauri dev

# Build de production
npm run tauri build
```

**Prérequis :** Node.js, Rust toolchain, WebView2 (Windows, déjà présent sur Windows 11).
