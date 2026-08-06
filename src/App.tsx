import { useState, useMemo, useEffect, useRef, useCallback } from "react";
import {
  Settings as SettingsIcon,
  RefreshCw,
  GitPullRequest,
  GitMerge,
  Sun,
  Moon,
  Bell,
  Power,
} from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useConfig } from "./hooks/useConfig";
import { usePRData } from "./hooks/usePRData";
import { useToast } from "./hooks/useToast";
import { useNotifications } from "./hooks/useNotifications";
import { FilterBar } from "./components/FilterBar";
import { RepoGroup } from "./components/RepoGroup";
import { Settings } from "./components/Settings";
import { ToastContainer } from "./components/ToastContainer";
import { NotificationPanel } from "./components/NotificationPanel";
import type { FilterStatus, PREvent } from "./types";
import { providerCreds } from "./types";

export default function App() {
  const [showSettings, setShowSettings] = useState(false);
  const [search, setSearch] = useState("");
  const [filter, setFilter] = useState<FilterStatus>("all");
  const [theme, setTheme] = useState<"dark" | "light">(() => {
    return (localStorage.getItem("theme") as "dark" | "light") ?? "dark";
  });

  const { config, loading, saveConfig } = useConfig();
  const queryClient = useQueryClient();
  const { toasts, addToast, dismissToast } = useToast();
  const { notifications, addNotification, markRead, markAllRead, remove, unreadCount } =
    useNotifications();
  const [showNotifications, setShowNotifications] = useState(false);
  const pendingRef = useRef<PREvent[]>([]);

  const handlePREvent = useCallback(async (event: PREvent) => {
    addNotification(event);
    const visible = await getCurrentWindow().isVisible();
    if (visible) {
      addToast(event);
    } else {
      pendingRef.current.push(event);
      const count = pendingRef.current.length;
      invoke("set_tray_tooltip", {
        tooltip: `PR Dashboard — ${count} new event${count > 1 ? "s" : ""}`,
      }).catch(() => {});
    }
  }, [addToast, addNotification]);

  useEffect(() => {
    let cleanup: (() => void) | undefined;
    getCurrentWindow()
      .listen("tauri://focus", () => {
        const pending = pendingRef.current;
        if (pending.length > 0) {
          pending.forEach(addToast);
          pendingRef.current = [];
          invoke("set_tray_tooltip", { tooltip: "PR Dashboard" }).catch(() => {});
        }
      })
      .then((unlisten) => { cleanup = unlisten; });
    return () => { cleanup?.(); };
  }, [addToast]);

  const queries = usePRData(config, handlePREvent);

  useEffect(() => {
    document.documentElement.classList.toggle("dark", theme === "dark");
    localStorage.setItem("theme", theme);
  }, [theme]);

  const hasRepos = config.repositories.length > 0;
  // Enough to work with as soon as one configured repo has a token for its own
  // source — a missing token on the other source only affects its own group.
  const hasToken = config.repositories.some(
    (repo) => providerCreds(config, repo).token.length > 0,
  );
  const missingProviders = Array.from(
    new Set(
      config.repositories
        .filter((repo) => providerCreds(config, repo).token.length === 0)
        .map((repo) => (repo.provider === "gitlab" ? "GitLab" : "GitHub")),
    ),
  );

  const { totalPending, totalApproved } = useMemo(() => {
    let pending = 0;
    let approved = 0;
    queries.forEach((q) => {
      if (!q.data) return;
      q.data.prs.forEach((pr) => {
        if (pr.reviewDecision === "APPROVED") approved++;
        else pending++;
      });
    });
    return { totalPending: pending, totalApproved: approved };
  }, [queries]);

  function refreshAll() {
    queryClient.invalidateQueries({ queryKey: ["prs"] });
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen bg-[var(--c-bg)]">
        <RefreshCw size={20} className="text-[var(--c-text-muted)] animate-spin" />
      </div>
    );
  }

  return (
    <div className="flex flex-col h-screen bg-[var(--c-bg)] text-[var(--c-text)] overflow-hidden">
      <header className="flex items-center justify-between px-4 py-3 border-b border-[var(--c-border)] bg-[var(--c-bg-subtle)] shrink-0">
        <div className="flex items-center gap-2">
          <GitMerge size={20} className="text-[var(--c-text)]" />
          <h1 className="font-semibold text-[var(--c-text)]">PR Dashboard</h1>
          {(totalPending > 0 || totalApproved > 0) && (
            <div className="flex items-center gap-1 ml-1">
              {totalPending > 0 && (
                <span className="px-1.5 py-0.5 bg-[var(--c-amber)] text-white rounded-full text-[10px] font-bold">
                  {totalPending}
                </span>
              )}
              {totalApproved > 0 && (
                <span className="px-1.5 py-0.5 bg-[var(--c-green-btn)] text-white rounded-full text-[10px] font-bold">
                  {totalApproved}
                </span>
              )}
            </div>
          )}
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={refreshAll}
            disabled={!hasToken || !hasRepos}
            className="p-1.5 text-[var(--c-text-muted)] hover:text-[var(--c-text)] disabled:opacity-40 transition-colors"
            title="Refresh all"
          >
            <RefreshCw size={16} />
          </button>
          <button
            onClick={() => setTheme((t) => (t === "dark" ? "light" : "dark"))}
            className="p-1.5 text-[var(--c-text-muted)] hover:text-[var(--c-text)] transition-colors"
            title={theme === "dark" ? "Switch to light mode" : "Switch to dark mode"}
          >
            {theme === "dark" ? <Sun size={16} /> : <Moon size={16} />}
          </button>
          <div className="relative">
            <button
              onClick={() => setShowNotifications((v) => !v)}
              className="p-1.5 text-[var(--c-text-muted)] hover:text-[var(--c-text)] transition-colors"
              title="Notifications"
            >
              <Bell size={16} />
              {unreadCount > 0 && (
                <span className="absolute top-1 right-1 flex h-2 w-2">
                  <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-[var(--c-accent)] opacity-75" />
                  <span className="relative inline-flex rounded-full h-2 w-2 bg-[var(--c-accent)]" />
                </span>
              )}
            </button>
            {showNotifications && (
              <NotificationPanel
                notifications={notifications}
                onMarkRead={markRead}
                onMarkAllRead={markAllRead}
                onRemove={remove}
                onClose={() => setShowNotifications(false)}
              />
            )}
          </div>
          <button
            onClick={() => setShowSettings(true)}
            className="p-1.5 text-[var(--c-text-muted)] hover:text-[var(--c-text)] transition-colors"
            title="Settings"
          >
            <SettingsIcon size={16} />
          </button>
          <button
            onClick={() => invoke("quit_app")}
            className="p-1.5 text-[var(--c-text-muted)] hover:text-red-500 transition-colors"
            title="Quit application"
          >
            <Power size={16} />
          </button>
        </div>
      </header>

      {!hasToken || !hasRepos ? (
        <div className="flex flex-col items-center justify-center flex-1 gap-4 text-center px-8">
          <GitPullRequest size={40} className="text-[var(--c-border)]" />
          <div>
            <p className="text-[var(--c-text)] font-medium mb-1">
              {!hasRepos
                ? "Add repositories to monitor"
                : `Configure your ${missingProviders.join(" / ")} token`}
            </p>
            <p className="text-sm text-[var(--c-text-muted)]">
              {!hasRepos
                ? "Open Settings to add GitHub or GitLab repositories."
                : "Open Settings to add the Personal Access Token for that source."}
            </p>
          </div>
          <button
            onClick={() => setShowSettings(true)}
            className="flex items-center gap-2 px-4 py-2 bg-[var(--c-green-btn)] hover:bg-[var(--c-green-btn-hover)] text-white text-sm font-medium rounded transition-colors"
          >
            <SettingsIcon size={14} />
            Open Settings
          </button>
        </div>
      ) : (
        <>
          <FilterBar
            search={search}
            onSearchChange={setSearch}
            filter={filter}
            onFilterChange={setFilter}
            totalPending={totalPending}
            totalApproved={totalApproved}
          />
          <main className="flex-1 overflow-y-auto p-4">
            <div className="flex flex-col gap-3 max-w-4xl mx-auto">
              {config.repositories.map((repo, i) => {
                const query = queries[i];
                const { token, baseUrl } = providerCreds(config, repo);
                return (
                  <RepoGroup
                    key={repo.id}
                    repo={repo}
                    prs={query?.data?.prs ?? []}
                    error={query?.data?.error ?? (query?.error ? String(query.error) : null)}
                    lastUpdated={query?.data?.lastUpdated ?? null}
                    isLoading={query?.isFetching ?? false}
                    filter={filter}
                    search={search}
                    token={token}
                    baseUrl={baseUrl}
                  />
                );
              })}
            </div>
          </main>
        </>
      )}

      {showSettings && (
        <Settings
          config={config}
          onSave={saveConfig}
          onClose={() => setShowSettings(false)}
          onTestNotification={() => {
            const event = {
              type: "new_pr" as const,
              provider: "github" as const,
              repo: "sinequa/test",
              prNumber: 42,
              prTitle: "This is a test notification",
              url: "",
            };
            addToast(event);
            addNotification(event);
          }}
        />
      )}

      <ToastContainer toasts={toasts} onDismiss={dismissToast} />
    </div>
  );
}
