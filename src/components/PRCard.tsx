import { useState } from "react";
import { GitPullRequest, GitMerge, Clock, ChevronRight, Copy, Check } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import type { PullRequest, ReviewDecision, Reviewer } from "../types";
import { timeAgo } from "../types";

interface Props {
  pr: PullRequest;
}

function reviewBadge(decision: ReviewDecision, isDraft: boolean) {
  if (isDraft) {
    return (
      <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-[var(--c-bg-inset)] text-[var(--c-text-muted)] border border-[var(--c-border)]">
        Draft
      </span>
    );
  }
  switch (decision) {
    case "APPROVED":
      return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-[var(--c-green-bg)] text-[var(--c-green)] border border-[var(--c-green-border)]">
          <GitMerge size={10} />
          Approved
        </span>
      );
    case "CHANGES_REQUESTED":
      return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-[var(--c-red-bg)] text-[var(--c-red)] border border-[var(--c-red-border)]">
          Changes Requested
        </span>
      );
    case "REVIEW_REQUIRED":
      return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-[var(--c-amber-bg)] text-[var(--c-amber)] border border-[var(--c-amber-border)]">
          <Clock size={10} />
          Pending Review
        </span>
      );
    default:
      return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-[var(--c-bg-inset)] text-[var(--c-text-muted)] border border-[var(--c-border)]">
          No Review
        </span>
      );
  }
}

function cardBgColor(decision: ReviewDecision, approvers: Reviewer[], isDraft: boolean) {
  if (!isDraft && decision === "APPROVED")
    return "bg-[var(--c-card-approved-bg)] hover:bg-[var(--c-card-approved-bg-hover)]";
  if (!isDraft && approvers.length > 0)
    return "bg-[var(--c-card-partial-bg)] hover:bg-[var(--c-card-partial-bg-hover)]";
  return "bg-[var(--c-bg-subtle)] hover:bg-[var(--c-bg-hover)]";
}

function cardBorderColor(decision: ReviewDecision, isDraft: boolean) {
  if (isDraft) return "border-[var(--c-border)]";
  switch (decision) {
    case "APPROVED":         return "border-l-[var(--c-green-border)] border-l-[3px]";
    case "CHANGES_REQUESTED":return "border-l-[var(--c-red-border)] border-l-[3px]";
    case "REVIEW_REQUIRED":  return "border-l-[var(--c-amber-border)] border-l-[3px]";
    default:                 return "border-l-[var(--c-border)] border-l-[3px]";
  }
}

const AVATAR_COLORS = [
  { bg: "#dbeafe", text: "#1d4ed8" },
  { bg: "#dcfce7", text: "#15803d" },
  { bg: "#fef9c3", text: "#a16207" },
  { bg: "#fce7f3", text: "#be185d" },
  { bg: "#ede9fe", text: "#6d28d9" },
  { bg: "#ffedd5", text: "#c2410c" },
  { bg: "#ccfbf1", text: "#0f766e" },
  { bg: "#e0f2fe", text: "#0369a1" },
];

function avatarColor(login: string) {
  let hash = 0;
  for (let i = 0; i < login.length; i++) hash = (hash * 31 + login.charCodeAt(i)) >>> 0;
  return AVATAR_COLORS[hash % AVATAR_COLORS.length];
}

function Avatar({ login, size = 20 }: { login: string; avatarUrl?: string; size?: number }) {
  const initials = login.slice(0, 2).toUpperCase();
  const color = avatarColor(login);
  const fontSize = Math.max(8, Math.floor(size * 0.45));
  return (
    <div
      className="rounded-full flex items-center justify-center font-semibold select-none"
      style={{ width: size, height: size, background: color.bg, color: color.text, fontSize }}
      title={login}
    >
      {initials}
    </div>
  );
}

function labelColor(hex: string) {
  return { bg: `#${hex}26`, text: `#${hex}`, border: `#${hex}66` };
}

export function PRCard({ pr }: Props) {
  const [copied, setCopied] = useState(false);

  async function openPR() {
    await invoke("open_url", { url: pr.url });
  }

  async function copyBranch(e: React.MouseEvent) {
    e.stopPropagation();
    try {
      await navigator.clipboard.writeText(pr.headRefName);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* clipboard unavailable */
    }
  }

  const borderClass = cardBorderColor(pr.reviewDecision, pr.isDraft);
  const bgClass = cardBgColor(pr.reviewDecision, pr.approvers, pr.isDraft);

  return (
    <div
      className={`${bgClass} border border-[var(--c-border)] ${borderClass} rounded-lg p-3 transition-colors cursor-pointer group`}
      onClick={openPR}
    >
      <div className="flex items-start justify-between gap-2">
        <div className="flex items-start gap-2 min-w-0">
          <GitPullRequest size={16} className="mt-0.5 shrink-0 text-[var(--c-accent)]" />
          <div className="min-w-0">
            <div className="flex items-center gap-2 flex-wrap">
              <span className="text-[var(--c-text-muted)] text-xs font-mono">
                #{pr.number}
              </span>
              <span className="text-sm font-medium text-[var(--c-text)] group-hover:text-[var(--c-accent)] transition-colors truncate">
                {pr.title}
              </span>
            </div>
            <div className="flex items-center gap-2 mt-1 flex-wrap">
              <div className="flex items-center gap-1">
                <Avatar login={pr.author.login} avatarUrl={pr.author.avatarUrl} size={16} />
                <span className="text-[11px] text-[var(--c-text-muted)]">
                  {pr.author.login}
                </span>
              </div>
              <span className="text-[var(--c-border)]">·</span>
              <span className="text-[11px] text-[var(--c-text-muted)]">
                {timeAgo(pr.createdAt)}
              </span>
              <span className="text-[var(--c-border)]">·</span>
              <div className="flex items-center gap-1 text-[11px] text-[var(--c-text-muted)] font-mono">
                <span>{pr.baseRefName}</span>
                <ChevronRight size={10} />
                <button
                  type="button"
                  onClick={copyBranch}
                  title={copied ? "Copied!" : `Copy "${pr.headRefName}"`}
                  className="inline-flex items-center gap-1 rounded px-1 -mx-1 py-0.5 hover:bg-[var(--c-bg-inset)] hover:text-[var(--c-text)] transition-colors"
                >
                  <span>{pr.headRefName}</span>
                  {copied ? (
                    <Check size={10} className="text-[var(--c-green)]" />
                  ) : (
                    <Copy size={10} className="opacity-0 group-hover:opacity-60" />
                  )}
                </button>
              </div>
            </div>
          </div>
        </div>
        <div className="shrink-0">
          {reviewBadge(pr.reviewDecision, pr.isDraft)}
        </div>
      </div>

      {pr.labels.length > 0 && (
        <div className="flex flex-wrap gap-1 mt-2 ml-6">
          {pr.labels.map((label) => {
            const colors = labelColor(label.color);
            return (
              <span
                key={label.name}
                className="px-1.5 py-0.5 rounded-full text-[10px] font-medium border"
                style={{
                  backgroundColor: colors.bg,
                  color: colors.text,
                  borderColor: colors.border,
                }}
              >
                {label.name}
              </span>
            );
          })}
        </div>
      )}

      {pr.approvers.length > 0 && (
        <div className="flex items-center gap-1 mt-2 ml-6">
          <span className="text-[10px] text-[var(--c-green)]">Approved by:</span>
          <div className="flex -space-x-1">
            {pr.approvers.map((r) => (
              <Avatar key={r.login} login={r.login} avatarUrl={r.avatarUrl} size={18} />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
