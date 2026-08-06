export type Provider = "github" | "gitlab";

export interface Config {
  githubToken: string;
  githubUrl: string;
  gitlabToken: string;
  gitlabUrl: string;
  refreshInterval: number;
  repositories: RepositoryConfig[];
}

export interface RepositoryConfig {
  id: string;
  provider: Provider;
  /** GitHub: the owner. GitLab: the full namespace path, e.g. "sinequa/rnd". */
  owner: string;
  name: string;
  label?: string;
}

/** Token and base URL of the source a given repository belongs to. */
export function providerCreds(config: Config, repo: RepositoryConfig) {
  return repo.provider === "gitlab"
    ? { token: config.gitlabToken, baseUrl: config.gitlabUrl }
    : { token: config.githubToken, baseUrl: config.githubUrl };
}

export function repoWebUrl(baseUrl: string, repo: RepositoryConfig): string {
  return repo.provider === "gitlab"
    ? `${baseUrl}/${repo.owner}/${repo.name}/-/merge_requests`
    : `${baseUrl}/${repo.owner}/${repo.name}/pulls`;
}

/** "PR" on GitHub, "MR" on GitLab — used in badges, toasts and empty states. */
export function prTerm(provider: Provider): "PR" | "MR" {
  return provider === "gitlab" ? "MR" : "PR";
}

/** GitLab writes merge request numbers as !123, GitHub as #123. */
export function prPrefix(provider: Provider): "!" | "#" {
  return provider === "gitlab" ? "!" : "#";
}

export interface Label {
  name: string;
  color: string;
}

export interface Reviewer {
  login: string;
  avatarUrl: string;
}

export type ReviewDecision =
  | "APPROVED"
  | "CHANGES_REQUESTED"
  | "REVIEW_REQUIRED"
  | null;

export interface PullRequest {
  number: number;
  title: string;
  url: string;
  isDraft: boolean;
  createdAt: string;
  updatedAt: string;
  headRefName: string;
  baseRefName: string;
  reviewDecision: ReviewDecision;
  author: Reviewer;
  labels: Label[];
  requestedReviewers: Reviewer[];
  approvers: Reviewer[];
}

export interface RepoPRs {
  repo: string;
  prs: PullRequest[];
  error: string | null;
  lastUpdated: string | null;
}

export type FilterStatus = "all" | "pending" | "approved";

export interface PREvent {
  type: "new_pr" | "merged";
  provider: Provider;
  repo: string;
  prNumber: number;
  prTitle: string;
  url: string;
}

export function generateId(): string {
  return Math.random().toString(36).slice(2, 9);
}

export interface StoredNotification {
  id: string;
  event: PREvent;
  timestamp: string;
  read: boolean;
}

export function timeAgo(dateStr: string): string {
  const now = new Date();
  const date = new Date(dateStr);
  const seconds = Math.floor((now.getTime() - date.getTime()) / 1000);
  if (seconds < 60) return "just now";
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  if (seconds < 604800) return `${Math.floor(seconds / 86400)}d ago`;
  return `${Math.floor(seconds / 604800)}w ago`;
}

export function matchesFilter(pr: PullRequest, filter: FilterStatus): boolean {
  if (filter === "all") return true;
  if (filter === "approved") return pr.reviewDecision === "APPROVED";
  if (filter === "pending") return pr.reviewDecision !== "APPROVED";
  return true;
}
