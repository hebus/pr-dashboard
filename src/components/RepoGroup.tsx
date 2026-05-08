import { useState } from "react";
import {
  ChevronDown,
  ChevronRight,
  RefreshCw,
  AlertCircle,
  ExternalLink,
} from "lucide-react";

function SkeletonCard({ titleW }: { titleW: string }) {
  return (
    <div className="animate-pulse bg-[var(--c-bg-subtle)] border border-[var(--c-border)] border-l-[3px] rounded-lg p-3">
      <div className="flex items-start justify-between gap-2">
        <div className="flex items-start gap-2 flex-1 min-w-0">
          <div className="w-4 h-4 rounded bg-[var(--c-bg-inset)] mt-0.5 shrink-0" />
          <div className="flex-1 flex flex-col gap-2 min-w-0">
            <div className="flex items-center gap-2">
              <div className="w-7 h-3 rounded bg-[var(--c-bg-inset)]" />
              <div className="h-3.5 rounded bg-[var(--c-bg-inset)]" style={{ width: titleW }} />
            </div>
            <div className="flex items-center gap-2 mt-0.5">
              <div className="w-4 h-4 rounded-full bg-[var(--c-bg-inset)]" />
              <div className="w-16 h-2.5 rounded bg-[var(--c-bg-inset)]" />
              <div className="w-10 h-2.5 rounded bg-[var(--c-bg-inset)]" />
              <div className="w-28 h-2.5 rounded bg-[var(--c-bg-inset)]" />
            </div>
          </div>
        </div>
        <div className="w-20 h-5 rounded-full bg-[var(--c-bg-inset)] shrink-0" />
      </div>
    </div>
  );
}
import { useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { RepositoryConfig, PullRequest, FilterStatus } from "../types";
import { matchesFilter, timeAgo } from "../types";
import { PRCard } from "./PRCard";

interface Props {
  repo: RepositoryConfig;
  prs: PullRequest[];
  error: string | null;
  lastUpdated: string | null;
  isLoading: boolean;
  filter: FilterStatus;
  search: string;
  token: string;
  githubUrl: string;
}

export function RepoGroup({
  repo,
  prs,
  error,
  lastUpdated,
  isLoading,
  filter,
  search,
  token,
  githubUrl,
}: Props) {
  const [collapsed, setCollapsed] = useState(false);
  const queryClient = useQueryClient();

  const filtered = prs.filter((pr) => {
    if (!matchesFilter(pr, filter)) return false;
    if (!search) return true;
    const q = search.toLowerCase();
    return (
      pr.title.toLowerCase().includes(q) ||
      pr.author.login.toLowerCase().includes(q) ||
      String(pr.number).includes(q)
    );
  });

  const pendingCount  = prs.filter((pr) => pr.reviewDecision !== "APPROVED").length;
  const approvedCount = prs.filter((pr) => pr.reviewDecision === "APPROVED").length;
  const repoLabel     = repo.label ?? `${repo.owner}/${repo.name}`;

  function handleRefresh() {
    queryClient.invalidateQueries({ queryKey: ["prs", repo.owner, repo.name] });
  }

  return (
    <div className="border border-[var(--c-border)] rounded-lg overflow-hidden">
      <div
        className="flex items-center justify-between px-4 py-3 bg-[var(--c-bg-subtle)] hover:bg-[var(--c-bg-hover)] cursor-pointer select-none"
        onClick={() => setCollapsed((c) => !c)}
      >
        <div className="flex items-center gap-2">
          {collapsed
            ? <ChevronRight size={16} className="text-[var(--c-text-muted)]" />
            : <ChevronDown  size={16} className="text-[var(--c-text-muted)]" />
          }
          <span className="font-semibold text-sm text-[var(--c-text)]">{repoLabel}</span>
          {error && <AlertCircle size={14} className="text-[var(--c-red)]" />}
          {isLoading && !error && <RefreshCw size={12} className="text-[var(--c-text-muted)] animate-spin" />}
        </div>

        <div className="flex items-center gap-3">
          {!error && (
            <>
              {pendingCount > 0 && (
                <span className="flex items-center gap-1 text-[11px] text-[var(--c-amber)]">
                  <span className="font-bold">{pendingCount}</span>
                  <span>pending</span>
                </span>
              )}
              {approvedCount > 0 && (
                <span className="flex items-center gap-1 text-[11px] text-[var(--c-green)]">
                  <span className="font-bold">{approvedCount}</span>
                  <span>approved</span>
                </span>
              )}
              {prs.length === 0 && !isLoading && (
                <span className="text-[11px] text-[var(--c-text-muted)]">No open PRs</span>
              )}
            </>
          )}
          {lastUpdated && (
            <span className="text-[10px] text-[var(--c-text-subtle)]">{timeAgo(lastUpdated)}</span>
          )}
          <button
            onClick={(e) => {
              e.stopPropagation();
              invoke("open_url", { url: `${githubUrl}/${repo.owner}/${repo.name}/pulls` });
            }}
            className="text-[var(--c-text-muted)] hover:text-[var(--c-accent)] transition-colors"
            title="Open on GitHub"
          >
            <ExternalLink size={13} />
          </button>
          <button
            onClick={(e) => { e.stopPropagation(); handleRefresh(); }}
            disabled={isLoading || token.length === 0}
            className="text-[var(--c-text-muted)] hover:text-[var(--c-text)] disabled:opacity-40 transition-colors"
            title="Refresh"
          >
            <RefreshCw size={13} className={isLoading ? "animate-spin" : ""} />
          </button>
        </div>
      </div>

      {!collapsed && (
        <div className="p-3 bg-[var(--c-bg)]">
          {error ? (
            <div className="flex items-center gap-2 p-3 bg-[var(--c-red-bg)] border border-[var(--c-red-border)] rounded text-sm text-[var(--c-red)]">
              <AlertCircle size={14} />
              <span>{error}</span>
            </div>
          ) : isLoading && prs.length === 0 ? (
            <div className="flex flex-col gap-2">
              <SkeletonCard titleW="58%" />
              <SkeletonCard titleW="42%" />
              <SkeletonCard titleW="72%" />
            </div>
          ) : filtered.length === 0 ? (
            <div className="text-center py-6 text-[var(--c-text-muted)] text-sm">
              No pull requests match the current filter.
            </div>
          ) : (
            <div className="flex flex-col gap-2">
              {filtered.map((pr) => (
                <PRCard key={pr.number} pr={pr} />
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
