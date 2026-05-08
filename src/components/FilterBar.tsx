import { Search } from "lucide-react";
import type { FilterStatus } from "../types";

interface Props {
  search: string;
  onSearchChange: (v: string) => void;
  filter: FilterStatus;
  onFilterChange: (f: FilterStatus) => void;
  totalPending: number;
  totalApproved: number;
}

const FILTERS: { key: FilterStatus; label: string }[] = [
  { key: "all", label: "All" },
  { key: "pending", label: "Pending" },
  { key: "approved", label: "Approved" },
];

export function FilterBar({
  search,
  onSearchChange,
  filter,
  onFilterChange,
  totalPending,
  totalApproved,
}: Props) {
  return (
    <div className="flex items-center gap-3 px-4 py-3 border-b border-[var(--c-border)] bg-[var(--c-bg-subtle)]">
      <div className="relative flex-1 max-w-sm">
        <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-[var(--c-text-muted)]" />
        <input
          type="text"
          placeholder="Search PRs..."
          value={search}
          onChange={(e) => onSearchChange(e.target.value)}
          className="w-full pl-8 pr-3 py-1.5 bg-[var(--c-bg)] border border-[var(--c-border)] rounded text-sm text-[var(--c-text)] placeholder-[var(--c-text-subtle)] focus:outline-none focus:border-[var(--c-accent)] transition-colors"
        />
      </div>

      <div className="flex items-center gap-1 bg-[var(--c-bg)] border border-[var(--c-border)] rounded p-0.5">
        {FILTERS.map(({ key, label }) => (
          <button
            key={key}
            onClick={() => onFilterChange(key)}
            className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
              filter === key
                ? "bg-[var(--c-bg-inset)] text-[var(--c-text)]"
                : "text-[var(--c-text-muted)] hover:text-[var(--c-text)]"
            }`}
          >
            {label}
            {key === "pending" && totalPending > 0 && (
              <span className="ml-1.5 px-1.5 py-0.5 bg-[var(--c-amber)] text-white rounded-full text-[10px] font-bold">
                {totalPending}
              </span>
            )}
            {key === "approved" && totalApproved > 0 && (
              <span className="ml-1.5 px-1.5 py-0.5 bg-[var(--c-green-btn)] text-white rounded-full text-[10px] font-bold">
                {totalApproved}
              </span>
            )}
          </button>
        ))}
      </div>
    </div>
  );
}
