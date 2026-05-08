export interface Config {
  githubToken: string;
  githubUrl: string;
  refreshInterval: number;
  repositories: RepositoryConfig[];
}

export interface RepositoryConfig {
  id: string;
  owner: string;
  name: string;
  label?: string;
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
