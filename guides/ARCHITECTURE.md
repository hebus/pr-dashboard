# PR Dashboard — Architecture

Application desktop Tauri 2 + React 19 qui affiche en temps réel les Pull Requests / Merge Requests ouvertes sur plusieurs repositories **GitHub Enterprise et GitLab**, avec notifications desktop.

---

## Stack technique

| Couche | Technologie | Rôle |
|--------|-------------|------|
| Frontend | React 19 + TypeScript | UI, état local, polling |
| Build frontend | Vite 7 | Dev server (port 1420), bundling |
| Styles | Tailwind CSS 4 + CSS variables | Utility-first, theming light/dark |
| Async/Cache | TanStack React Query 5 | Fetching, polling, invalidation |
| Desktop shell | Tauri 2 (Rust) | Fenêtre native, commandes |
| HTTP Rust | reqwest 0.12 + tokio | Appels GitHub / GitLab API (async) |
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
│   │   ├── useNotifications.ts   # Historique persistant des notifications (markRead, remove…)
│   │   └── useUpdater.ts         # Vérification des mises à jour (GitHub Releases) + installation
│   ├── components/
│   │   ├── ErrorBoundary.tsx     # Capture les erreurs React
│   │   ├── UpdateBanner.tsx      # Bannière « nouvelle version disponible » (installer / plus tard)
│   │   ├── FilterBar.tsx         # Barre recherche + toggle All/Pending/Approved/Mergeable
│   │   ├── ProviderIcon.tsx      # Logos GitHub / GitLab en SVG inline (lucide n'a plus d'icônes de marque)
│   │   ├── RepoGroup.tsx         # Groupe collapsible par repository (avec squelettes au premier chargement)
│   │   ├── PRCard.tsx            # Carte d'une PR/MR (nom de branche cliquable → copie clipboard)
│   │   ├── ToastContainer.tsx    # Toasts in-app (slide-in, barre de progression, cliquables)
│   │   ├── NotificationPanel.tsx # Panneau dropdown historique des notifications (icône cloche dans le header)
│   │   └── Settings.tsx          # Modal de configuration (sources GitHub/GitLab, repos, intervalle, test notification)
│   ├── App.tsx                   # Racine : layout, orchestration état global, toggle thème, gestion événements PR
│   ├── main.tsx                  # Bootstrap React + QueryClient
│   └── index.css                 # Tailwind import, variables CSS des deux thèmes, scrollbar, @keyframes toastProgress
│
└── src-tauri/                    # Backend Rust
    ├── src/
    │   ├── lib.rs                # Init Tauri : plugins, handlers, register_aumid, quit_app
    │   ├── main.rs               # Point d'entrée (délègue à lib.rs)
    │   ├── commands/
    │   │   ├── mod.rs            # Déclaration des modules
    │   │   ├── config.rs         # Commandes get_config / save_config
    │   │   └── fetch_prs.rs      # Commandes fetch_repo_prs / check_pr_merged / open_url (dispatch par provider)
    │   └── providers/            # Domaine : un module par source, sans dépendance à Tauri
    │       ├── mod.rs            # Re-exports (Provider, RepoPRs)
    │       ├── common.rs         # Types partagés + make_client, normalize_hex, absolutize
    │       ├── github.rs         # Query GraphQL GitHub + mapping + check_merged (REST v3)
    │       └── gitlab.rs         # Query GraphQL GitLab + mapping + check_merged (GraphQL) + tests
    ├── capabilities/
    │   └── default.json          # Permissions Tauri (store, notification, opener)
    ├── icons/                    # Icônes générées (ne pas éditer à la main)
    ├── icons-source/
    │   └── icon.svg              # Source de l'icône (régénérer : npx tauri icon src-tauri/icons-source/icon.svg)
    ├── Cargo.toml                # Dépendances Rust
    └── tauri.conf.json           # Config Tauri (nom, fenêtre, bundle)
```

---

## Configuration (Config)

La configuration est persistée dans un fichier JSON via `tauri-plugin-store` (emplacement : `%APPDATA%\com.sinequa.prdashboard\config.json`).

```typescript
type Provider = "github" | "gitlab";

interface Config {
  githubToken: string;        // PAT GitHub (scope: repo)
  githubUrl: string;          // URL de l'instance GitHub (défaut: https://github.sinequa.com)
  gitlabToken: string;        // PAT GitLab (scope: read_api)
  gitlabUrl: string;          // URL de l'instance GitLab (défaut: https://gitlab.chapsvision.in)
  refreshInterval: number;    // Intervalle de polling en secondes (défaut: 60)
  repositories: RepositoryConfig[];
}

interface RepositoryConfig {
  id: string;         // UUID généré localement
  provider: Provider; // Source du repo (défaut: "github")
  owner: string;      // GitHub : le owner ("sinequa") · GitLab : le namespace complet ("sinequa/rnd")
  name: string;       // Nom du repo (ex: "sba-internal")
  label?: string;     // Alias d'affichage optionnel
}
```

**Un token par source.** Chaque repo déclare son `provider` ; le couple token/URL est résolu par repo via le helper `providerCreds(config, repo)` (`types/index.ts`), réutilisé par `usePRData`, `App` et `RepoGroup`. Un token manquant n'affecte que les groupes de la source concernée.

**Flux de config :**
1. `useConfig` appelle `invoke("get_config")` au montage
2. Le backend Rust lit le store → retourne la config ou la valeur par défaut
3. `saveConfig()` appelle `invoke("save_config", { config })` → persiste sur disque

> **Rétrocompatibilité :** `get_config` retombe sur `Config::default()` si la désérialisation échoue — sans précaution, l'ajout d'un champ ferait perdre token et repos. Tous les champs portent donc `#[serde(default)]` (et non les seuls nouveaux), la struct utilise `rename_all = "camelCase"`, et `deny_unknown_fields` est proscrit. En dernier recours, une config illisible est recopiée sous la clé `config.broken` avant d'être remplacée, pour ne jamais la perdre définitivement.

> **Thème :** la préférence light/dark n'est pas dans la `Config` Tauri — elle est stockée dans `localStorage` et lue au démarrage de l'application (`"dark"` par défaut).

---

## Flux de données principal

```
App.tsx
  ├─ useState(theme)      → localStorage               → classe .dark sur <html>
  ├─ useConfig()          → invoke("get_config")        → config.json (AppData)
  └─ usePRData(config)
       └─ useQueries()    → invoke("fetch_repo_prs")    → GraphQL GitHub ou GitLab
            │                  (une query par repo, token/URL résolus par provider,
            │                   polling toutes les N secondes)
            │
            ↓ résultats
       RepoPRs[]  ──────────────────────────────────────────────────────────────
            │                                                                   │
            ↓                                                                   ↓
       RepoGroup (par repo)                                            Notifications
            └─ PRCard (par PR)                                  (nouvelle PR / PR mergée)
```

---

## APIs GitHub / GitLab

L'application supporte **GitHub Enterprise Server** (GHES) et **GitLab** (self-hosted ou gitlab.com). L'URL de base de chaque source est configurable dans les Settings ; les endpoints en sont dérivés automatiquement.

| Usage | GitHub | GitLab |
|-------|--------|--------|
| Liste des PR/MR | `{githubUrl}/api/graphql` (GraphQL) | `{gitlabUrl}/api/graphql` (GraphQL) |
| Vérification de merge | `{githubUrl}/api/v3/repos/{owner}/{repo}/pulls/{number}` (REST v3) | `{gitlabUrl}/api/graphql` (GraphQL) |
| Web | `{githubUrl}/{owner}/{repo}/pulls` | `{gitlabUrl}/{namespace}/{repo}/-/merge_requests` |

Authentification : `Authorization: Bearer <token>` dans les deux cas — un seul code path, et côté GitLab cela couvre PAT, project/group tokens et OAuth (contrairement à `PRIVATE-TOKEN`). Scopes requis : `repo` (GitHub), `read_api` (GitLab).

**Dispatch.** `commands/fetch_prs.rs` ne fait que router vers `providers::github` ou `providers::gitlab` selon le `provider` reçu. Chaque provider retourne une vraie `Err` ; c'est la commande qui la convertit en `RepoPRs { error: Some(..) }`, ce qui concentre en un seul endroit l'invariant « jamais de `Err` renvoyée au frontend ».

### GraphQL GitHub (fetch_repo_prs)

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
        mergeStateStatus        ← CLEAN | HAS_HOOKS | UNKNOWN | BEHIND | BLOCKED | DIRTY | DRAFT | UNSTABLE
        author { login, avatarUrl }
        labels { name, color }
        reviews(last: 20) { state, author }   ← pour extraire les approbateurs
      }
    }
  }
}
```

`reviewDecision` est calculé par GitHub et reflète la décision agrégée de review.

`mergeStateStatus` alimente aussi `hasConflicts` (`DIRTY` → `true`, voir « Badge et filtre « Conflicts » »). Il alimente `mergeable` (`github.rs::mergeable_from_state`) : `CLEAN` / `HAS_HOOKS` → `true` ; `UNKNOWN` (calcul en cours) → `null` ; tout le reste → `false`. `UNSTABLE` (checks non requis en échec) est volontairement traité comme bloquant : « fusionnable » signifie « au vert ». Ce champ n'existe que sur les GHES récents : sur une version plus ancienne, une erreur de champ GraphQL fera échouer la query du repo (voir la note GHES ci-dessous).

> **Note GHES :** Le champ `requestedReviewers` n'est pas disponible sur toutes les versions de GitHub Enterprise Server. Il a été retiré de la query pour éviter une erreur GraphQL bloquante. Le champ `requestedReviewers` dans les types TypeScript et Rust est conservé mais retourne toujours un tableau vide.

### REST GitHub (check_pr_merged)

Appelé uniquement quand une PR disparaît de la liste ouverte, pour distinguer une fermeture d'un merge :

```
GET {githubUrl}/api/v3/repos/{owner}/{repo}/pulls/{number}
→ { state: "closed", merged_at: "2024-..." }   ← merged si merged_at non null
```

### GraphQL GitLab (fetch_repo_prs)

Le chemin projet est `{owner}/{name}`, le `owner` d'un repo GitLab contenant le namespace complet (`sinequa/rnd`).

```
POST {gitlabUrl}/api/graphql
Authorization: Bearer <token>

query GetOpenMRs($fullPath: ID!, $first: Int!) {
  project(fullPath: $fullPath) {
    mergeRequests(state: opened, sort: UPDATED_DESC, first: $first) {
      nodes {
        iid title webUrl draft createdAt updatedAt
        sourceBranch targetBranch approved
        detailedMergeStatus
        conflicts
        author { username avatarUrl }
        labels(first: 10) { nodes { title color } }
        approvedBy(first: 20) { nodes { username avatarUrl } }
        reviewers(first: 20) {
          nodes { username avatarUrl mergeRequestInteraction { reviewState } }
        }
      }
    }
  }
}
```

Particularités du schéma GitLab :
- `state: opened` est en minuscules alors que `sort: UPDATED_DESC` est en majuscules — ce sont des littéraux enum inline.
- Toutes les sous-connexions sont bornées (`first:`) : sans borne GitLab applique `default_max_page_size = 100` et la complexité grimpe inutilement (plafond 250 en authentifié).
- Une erreur de champ ou de complexité revient en **HTTP 200 avec `errors[]`** — elle est concaténée dans `RepoPRs.error` et s'affiche dans le groupe, sans casser les autres repos.

**Mapping MergeRequest → `PullRequest`** (`providers/gitlab.rs::map_mr`, fonction pure couverte par tests) :

| Champ | Source | Traitement |
|-------|--------|-----------|
| `number` | `iid` | `iid` est une **`String`** dans le schéma GitLab → parsé en `u64` ; la MR est ignorée si le parse échoue (le frontend indexe ses diffs par `number`) |
| `headRefName` / `baseRefName` | `sourceBranch` / `targetBranch` | — |
| `author.login` | `author.username` | fallback `ghost` si nul |
| `labels[].color` | `labels.nodes[].color` | GitLab renvoie `#RRGGBB`, GitHub `RRGGBB` → `normalize_hex()` retire le `#` (le frontend écrit `#${hex}`) |
| `approvers` | `approvedBy.nodes` | — |
| `requestedReviewers` | `reviewers.nodes` | rempli côté GitLab (contrairement à GHES) |
| `avatarUrl` | `avatarUrl` | `absolutize()` : GitLab peut renvoyer une URL relative ou protocol-relative |

`reviewDecision` est dérivé dans cet ordre : un reviewer en `REQUESTED_CHANGES` → `CHANGES_REQUESTED` ; sinon `approved` → `APPROVED` ; sinon `REVIEW_REQUIRED`.

`mergeable` est dérivé de `detailedMergeStatus` (`gitlab.rs::mergeable_from_status`) : `MERGEABLE` → `true` ; `CHECKING` / `UNCHECKED` / `PREPARING` (GitLab calcule le statut de façon asynchrone, notamment juste après un push) → `null` ; tout le reste (`CONFLICT`, `CI_MUST_PASS`, `NOT_APPROVED`, `DRAFT_STATUS`, `DISCUSSIONS_NOT_RESOLVED`, `NEED_REBASE`…) → `false`.

`hasConflicts` vaut `conflicts || detailedMergeStatus == "CONFLICT"` (`gitlab.rs::map_mr`). Le booléen `conflicts` est prioritaire : `detailedMergeStatus` n'expose qu'**un seul** bloqueur à la fois, donc un pipeline en échec peut masquer un conflit.

> `approved` signifie « **toutes** les approbations requises sont obtenues ». Une MR approuvée 1 fois sur 2 reste `REVIEW_REQUIRED` tout en exposant ses `approvers` → elle prend le fond « partiellement approuvée », exactement comme sur GitHub. Ne pas dériver `APPROVED` d'un `approvedBy` non vide.

### GraphQL GitLab (check_pr_merged)

GraphQL plutôt que REST v4 : même endpoint, même auth, même gestion d'erreurs, et surtout aucun percent-encoding du chemin projet à gérer.

```
query MrState($fullPath: ID!, $iid: String!) {
  project(fullPath: $fullPath) { mergeRequest(iid: $iid) { state } }
}
→ merged si state == "merged"
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
  mergeable: boolean | null;       // true: fusionnable · false: bloquée · null: statut pas encore calculé
  hasConflicts: boolean;           // conflits avec la branche cible (GitLab `conflicts`, GitHub `DIRTY`)
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

Le modèle est **commun aux deux sources** : une Merge Request GitLab est mappée sur `PullRequest` côté Rust, si bien qu'aucun composant d'affichage n'a besoin de connaître la source. Seuls la terminologie et les liens en dépendent, via les helpers `prTerm()` (« PR » / « MR »), `prPrefix()` (`#` / `!`) et `repoWebUrl()` de `types/index.ts`.

---

## Chargement initial — squelettes

Au premier lancement (avant que les données GitHub ne soient disponibles), `RepoGroup` affiche 3 cartes squelettes animées (`animate-pulse`) qui reproduisent la structure de `PRCard` :
- Icône + numéro + barre de titre (largeurs variées : 58 %, 42 %, 72 %)
- Ligne auteur / date / branches avec avatar rond
- Badge de review

La condition d'affichage est `isLoading && prs.length === 0` : les squelettes n'apparaissent qu'au **premier fetch** (pas données en cache). Les refreshs en arrière-plan restent discrets — seul le spinner dans l'en-tête du groupe tourne.

---

## Groupement par branche cible (`RepoGroup`)

Quand un repo a des PR ouvertes sur plusieurs branches cibles (`baseRefName`) différentes, `RepoGroup` les regroupe en sous-sections au lieu d'une liste plate (comportement inchangé quand une seule branche cible est présente).

- **Classification et tri** (`branchKind()` / `compareBranches()` dans `types/index.ts`) : `develop` toujours en tête, puis les branches `release/x` triées par version décroissante (comparaison numérique segment par segment sur `.`/`-`/`_`), puis les autres branches par ordre alphabétique.
- **Pastille colorée** par sous-groupe (icône `GitBranch` + nom de branche, `rounded-full`) : indigo (`--c-indigo` / `--c-indigo-bg`) pour `develop`, violet (`--c-violet` / `--c-violet-bg`) pour `release/x`, neutre (`--c-bg-inset` / `--c-text-muted`) pour les autres branches — calculée par `branchBadgeClass()` dans `RepoGroup.tsx`.
- Ce système est indépendant du fond de carte « approuvée » (`PRCard`, voir plus bas) : chaque `PRCard` garde son propre fond quelle que soit la branche sous laquelle elle est rangée.

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

### Badge et filtre « Mergeable »

`mergeable` est tri-état et commun aux deux sources. Seul `true` produit un effet : badge vert `Mergeable` (icône `CheckCircle2`) sous le badge de review dans `PRCard`, et filtre `mergeable` dans `FilterBar` (`matchesFilter` : `pr.mergeable === true`) avec son compteur (`totalMergeable`, calculé dans `App.tsx`). `false` et `null` ne s'affichent pas : une MR dont le statut est encore en cours de calcul n'est jamais présentée comme bloquée.

### Badge et filtre « Conflicts »

`hasConflicts` est un booléen commun aux deux sources, indépendant de `mergeable` (une PR en conflit a toujours `mergeable === false`, mais l'inverse est faux). `true` affiche un badge rouge `Conflicts` (icône `AlertTriangle`) sous le badge de review dans `PRCard`, au-dessus du badge `Mergeable` qui ne peut pas coexister avec lui, et active le filtre `conflicts` de `FilterBar` (`matchesFilter` : `pr.hasConflicts`) avec son compteur rouge (`totalConflicts`, calculé dans `App.tsx`). GitLab calcule les conflits de façon asynchrone : juste après un push, `conflicts` peut être temporairement périmé.

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
     ↑ repoKey = "{provider}:{owner}/{name}" (clé interne uniquement)
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
  provider: Provider; // pilote la terminologie affichée : "New PR" ou "New MR"
  repo: string;       // "owner/name" — affiché tel quel, donc jamais préfixé du provider
  prNumber: number;
  prTitle: string;
  url: string;
}
```

**`usePRData`** n'appelle plus directement les APIs de notification — il appelle le callback `onEvent: (e: PREvent) => void` passé par `App.tsx`. Le ref interne stocke désormais `{ title, url }` (au lieu de juste `title`) pour pouvoir inclure l'URL dans les événements de merge.

### Comportement quand la fenêtre n'est pas visible

```
PR event détecté
  ├─ fenêtre visible      →  addToast()  →  toast affiché immédiatement
  └─ fenêtre non visible  →  pendingRef.current.push(event)

tauri://focus (fenêtre reprend le focus)
  →  pending.forEach(addToast)   [tous les toasts manqués s'affichent]
  →  pendingRef.current = []
```

Fermer la fenêtre (croix ✕) quitte l'application — il n'y a plus de mise en arrière-plan. Ce chemin ne sert donc que pour le cas où la fenêtre perd le focus sans être fermée (minimisée, autre fenêtre au premier plan).

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
| `--c-indigo` | `#4f46e5` | `#a5b4fc` | Pastille de branche `develop` |
| `--c-indigo-bg` | `#eef2ff` | `#1e1b4b` | Pastille de branche `develop` (fond) |
| `--c-violet` | `#8250df` | `#d8b4fe` | Pastille de branche `release/x` |
| `--c-violet-bg` | `#fbefff` | `#2e1065` | Pastille de branche `release/x` (fond) |

---

## Communication Frontend ↔ Backend

Tauri expose un système de RPC via `invoke()` côté frontend. Les handlers Rust sont enregistrés dans `lib.rs` :

| Commande | Paramètres clés | Description |
|----------|-----------------|-------------|
| `get_config` | — | Lit la config depuis le store |
| `save_config` | `config` | Persiste la config sur disque |
| `fetch_repo_prs` | `provider`, `owner`, `repo`, `token`, `baseUrl` | Appel GraphQL (GitHub ou GitLab), retourne `RepoPRs` |
| `check_pr_merged` | `provider`, `owner`, `repo`, `number`, `token`, `baseUrl` | REST v3 (GitHub) ou GraphQL (GitLab), retourne `bool` |
| `open_url` | `url` | Ouvre une URL dans le navigateur système |
| `quit_app` | — | Quitte l'application (`app.exit(0)`) |

Les `AppHandle` et injections Tauri sont gérés automatiquement — le frontend ne passe que les paramètres métier.

---

## Polling et performance

- **React Query** gère le cache, le polling (`refetchInterval`) et les invalidations manuelles
- Chaque repo a sa propre query indépendante : clé `["prs", provider, owner, name]` — le provider en fait partie pour que deux repos homonymes hébergés sur des sources différentes ne partagent pas de cache
- L'invalidation manuelle (bouton refresh) appelle `queryClient.invalidateQueries()`
- Les queries sont désactivées (`enabled: false`) si le token **de la source du repo** est vide
- Le backend fait l'appel HTTP en Rust avec un timeout de 15s

---

## Mise à jour automatique

L'app vérifie GitHub Releases au démarrage (après 5 s) puis toutes les 6 h. Rien n'est téléchargé tant que l'utilisateur n'a pas cliqué sur **Install and restart**.

| Élément | Rôle |
|---------|------|
| `tauri-plugin-updater` + `tauri-plugin-process` | Vérification/installation de la mise à jour, puis `relaunch()`. Permissions `updater:default` et `process:allow-restart` dans `capabilities/default.json` |
| `hooks/useUpdater.ts` | `check()` planifié, état `idle → available → installing → error`, progression du téléchargement. Plugins importés dynamiquement : inerte dans `npm run dev` hors webview |
| `components/UpdateBanner.tsx` | Bannière sous le header : « Version X disponible », bouton d'installation, croix « Later » (rouverte si une version plus récente sort) |
| `plugins.updater` (`tauri.conf.json`) | `pubkey` + endpoint `https://github.com/hebus/pr-dashboard/releases/latest/download/latest.json` ; `installMode: passive` sous Windows |
| `bundle.createUpdaterArtifacts` | Produit les `.sig` à côté des installeurs |

Une erreur de vérification (hors-ligne, pas encore de release…) est ignorée silencieusement (`console.warn`) ; seule une erreur d'installation s'affiche.

### Signature

Tauri refuse tout binaire non signé avec la clé correspondant à `pubkey`. La paire a été générée avec `npm run tauri signer generate`.
- **Clé publique** : dans `tauri.conf.json`.
- **Clé privée** : secrets GitHub `TAURI_SIGNING_PRIVATE_KEY` (+ `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, vide si la clé n'a pas de mot de passe). **À sauvegarder** : sans elle, les installations existantes ne pourront plus jamais se mettre à jour.
- Cette signature n'est pas une signature de code : sans certificat, Windows SmartScreen et macOS Gatekeeper avertissent à la première installation.

### Publier une release (release-please)

Le versionnement et la publication sont automatisés par [release-please](https://github.com/googleapis/release-please) (`release-please-config.json`, `.release-please-manifest.json`), à partir des **Conventional Commits** :

1. Chaque push sur `develop` exécute le job `release-please` de `.github/workflows/release.yml`, qui ouvre ou met à jour une **PR de release** (`chore(release): X.Y.Z`) : version dans `package.json`, `package-lock.json`, `tauri.conf.json` (`extra-files`, jsonpath `$.version`), `Cargo.toml` (toml, jsonpath), plus `CHANGELOG.md`. `Cargo.lock` n'est **pas** géré par `release-please` : son updater TOML ne sait pas cibler un paquet par son nom (les filtres `jsonpath` ne retournent rien sur son lockfile parsé, seul l'accès par index marche, trop fragile). Un job `sync-lockfile`, séparé pour ne jamais bloquer une release, exécute `cargo update --workspace` sur la branche de la PR de release et y pousse un commit `chore: sync Cargo.lock` si besoin. Le titre de la PR est fixé par `pull-request-title-pattern`. Fusionner les PR de fonctionnalité en **squash** : sinon le commit de branche et celui de `develop` apparaissent tous deux dans le changelog. Tant que la version est en 0.x : `fix` et `feat` → patch, `feat!` / `BREAKING CHANGE` → minor (`bump-patch-for-minor-pre-major` + `bump-minor-pre-major`).
2. **Fusionner cette PR** déclenche à nouveau le workflow : release-please crée le tag `vX.Y.Z` et la release GitHub (corps = changelog), `release_created` passe à `true` et les jobs `build` s'exécutent.
3. `build` (Windows, macOS arm64 + x64, Linux) tourne dans **le même workflow** : un tag créé avec `GITHUB_TOKEN` ne déclenche pas d'autre workflow. Il vérifie que le tag correspond à `tauri.conf.json`, puis `tauri-action` signe et attache installeurs, `.sig` et `latest.json` à la release existante (sans `releaseName`/`releaseBody`, pour conserver le changelog).

Prérequis : secrets `TAURI_SIGNING_PRIVATE_KEY` (+ `..._PASSWORD`) et réglage du dépôt *Settings → Actions → General → Allow GitHub Actions to create and approve pull requests*, sans quoi release-please ne peut pas ouvrir sa PR.

Conséquence du choix « release non brouillon » : la release existe dès la fusion de la PR, quelques minutes avant que les installeurs soient attachés. Pendant ce temps, `releases/latest/download/latest.json` répond 404 et l'updater (qui ignore ses erreurs) ne propose simplement rien. Si un build échoue, la release reste sans binaires : relancer les jobs en échec depuis l'onglet Actions.

L'updater compare la version installée à `latest.json` : chaque release doit avoir une version strictement supérieure.

---

## Lancer l'application

```bash
# Développement (hot-reload frontend, rebuild Rust si modifié)
npm run tauri dev

# Build de production
npm run tauri build
```

**Prérequis :** Node.js, Rust toolchain, WebView2 (Windows, déjà présent sur Windows 11).
