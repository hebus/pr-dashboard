import { useEffect, useRef } from "react";
import { X, Bell, CheckCheck, GitPullRequest, GitMerge } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import type { StoredNotification } from "../types";
import { prPrefix, prTerm, timeAgo } from "../types";

interface Props {
  notifications: StoredNotification[];
  onMarkRead: (id: string) => void;
  onMarkAllRead: () => void;
  onRemove: (id: string) => void;
  onClose: () => void;
}

export function NotificationPanel({
  notifications,
  onMarkRead,
  onMarkAllRead,
  onRemove,
  onClose,
}: Props) {
  const panelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        onClose();
      }
    }
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, [onClose]);

  const unreadCount = notifications.filter((n) => !n.read).length;

  function handleItemClick(n: StoredNotification) {
    if (!n.read) onMarkRead(n.id);
    if (n.event.url) invoke("open_url", { url: n.event.url }).catch(() => {});
  }

  return (
    <div
      ref={panelRef}
      className="absolute top-full right-0 mt-2 w-80 bg-[var(--c-bg-subtle)] border border-[var(--c-border)] rounded-lg shadow-xl z-50 flex flex-col overflow-hidden"
      style={{ maxHeight: "420px" }}
    >
      <div className="flex items-center justify-between px-3 py-2.5 border-b border-[var(--c-border)]">
        <span className="text-sm font-semibold text-[var(--c-text)]">
          Notifications
          {unreadCount > 0 && (
            <span className="ml-1.5 px-1.5 py-0.5 bg-[var(--c-bg-inset)] text-[var(--c-accent)] rounded-full text-[10px] font-bold">
              {unreadCount}
            </span>
          )}
        </span>
        {unreadCount > 0 && (
          <button
            onClick={onMarkAllRead}
            className="flex items-center gap-1 text-[11px] text-[var(--c-text-muted)] hover:text-[var(--c-accent)] transition-colors"
            title="Mark all as read"
          >
            <CheckCheck size={12} />
            Mark all read
          </button>
        )}
      </div>

      {notifications.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-10 gap-2 text-[var(--c-text-muted)]">
          <Bell size={20} className="opacity-40" />
          <span className="text-xs">No notifications</span>
        </div>
      ) : (
        <ul className="overflow-y-auto flex-1">
          {notifications.map((n) => {
            const isNew = n.event.type === "new_pr";
            const term = prTerm(n.event.provider);
            return (
              <li
                key={n.id}
                onClick={() => handleItemClick(n)}
                className={`flex items-start gap-2.5 px-3 py-2.5 border-b border-[var(--c-border)] last:border-0 cursor-pointer hover:bg-[var(--c-bg-hover)] transition-colors group ${
                  n.read ? "" : "bg-[var(--c-bg-inset)]"
                }`}
              >
                <span
                  className={`mt-1 shrink-0 w-1.5 h-1.5 rounded-full ${
                    n.read ? "opacity-0" : "bg-[var(--c-accent)]"
                  }`}
                />
                <div
                  className={`mt-0.5 shrink-0 ${
                    isNew ? "text-[var(--c-accent)]" : "text-[var(--c-green)]"
                  }`}
                >
                  {isNew ? <GitPullRequest size={13} /> : <GitMerge size={13} />}
                </div>
                <div className="flex-1 min-w-0">
                  <p className="text-xs font-medium text-[var(--c-text)]">
                    {isNew ? `New ${term}` : `${term} merged`}
                    <span className="font-normal text-[var(--c-text-muted)]">
                      {" "}· {n.event.repo}
                    </span>
                  </p>
                  <p className="text-[11px] text-[var(--c-text-muted)] truncate">
                    {prPrefix(n.event.provider)}{n.event.prNumber} — {n.event.prTitle}
                  </p>
                  <p className="text-[10px] text-[var(--c-text-muted)] opacity-60 mt-0.5">
                    {timeAgo(n.timestamp)}
                  </p>
                </div>
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onRemove(n.id);
                  }}
                  className="shrink-0 opacity-0 group-hover:opacity-100 text-[var(--c-text-muted)] hover:text-[var(--c-red)] transition-all mt-0.5"
                  title="Dismiss"
                >
                  <X size={12} />
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
